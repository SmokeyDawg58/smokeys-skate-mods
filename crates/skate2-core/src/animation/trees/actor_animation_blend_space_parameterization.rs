//! Optimized three-dimensional `BlendSpace` parameterization, 82d723f8..73518.
//! Host estimates, reductions and exceptional min/max remain explicit services.
use crate::actor_face_playback::presentation::read::decode::PresentationDecodeNumeric;
use crate::simulation_drive_jacobian::refine_drive_reciprocal;
use skate2_formats::andale::AndaleBlendSpaceVolume;

pub(crate) trait BlendSpaceNumeric: PresentationDecodeNumeric {
    fn maximum(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<[f32; 4], Self::Error>;
    fn minimum(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<[f32; 4], Self::Error>;
}
impl BlendSpaceNumeric for crate::reckoning_numeric::HostReckoningNumeric {
    fn maximum(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(std::array::from_fn(
            |i| if a[i] > b[i] { a[i] } else { b[i] },
        ))
    }
    fn minimum(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(std::array::from_fn(
            |i| if a[i] < b[i] { a[i] } else { b[i] },
        ))
    }
}
#[derive(Debug)]
pub(crate) enum ParameterizationError<E> {
    VolumeBounds,
    Numeric(E),
}
/// Borrowed native destinations: the worker writes the node's own vectors.
pub(crate) struct ParameterizationOutput<'a> {
    pub(crate) projected: &'a mut [f32; 4],
    pub(crate) coefficients: &'a mut [f32; 4],
    pub(crate) found: &'a mut Option<usize>,
    pub(crate) needs_projection: &'a mut bool,
    pub(crate) ready: &'a mut bool,
}
struct VolumeGeometry {
    vertices: [[f32; 4]; 4],
    normals: [[f32; 4]; 4],
    heights: [f32; 4],
}
pub(super) fn float_at<E>(
    volume: &AndaleBlendSpaceVolume,
    offset: usize,
) -> Result<f32, ParameterizationError<E>> {
    let end = offset
        .checked_add(4)
        .ok_or(ParameterizationError::VolumeBounds)?;
    let word = volume
        .serialized_data
        .get(offset..end)
        .or_else(|| {
            if offset == volume.serialized_data.len() {
                volume
                    .serialized_following_word
                    .as_ref()
                    .map(<[u8; 4]>::as_slice)
            } else {
                None
            }
        })
        .ok_or(ParameterizationError::VolumeBounds)?;
    Ok(f32::from_bits(u32::from_be_bytes(
        word.try_into().expect("four bytes"),
    )))
}
// Checked host admission of the retained serialized volume. Bounds failures
// precede numeric calls; malformed-buffer trap precedence is not native parity.
fn geometry<E>(
    volume: &AndaleBlendSpaceVolume,
) -> Result<VolumeGeometry, ParameterizationError<E>> {
    let normals =
        usize::try_from(volume.normals_offset).map_err(|_| ParameterizationError::VolumeBounds)?;
    let heights =
        usize::try_from(volume.heights_offset).map_err(|_| ParameterizationError::VolumeBounds)?;
    let mut result = VolumeGeometry {
        vertices: [[0.; 4]; 4],
        normals: [[0.; 4]; 4],
        heights: [0.; 4],
    };
    for i in 0..4 {
        for lane in 0..4 {
            result.vertices[i][lane] = float_at(volume, 20 + i * 16 + lane * 4)?;
            let offset = normals
                .checked_add(4 + i * 16 + lane * 4)
                .ok_or(ParameterizationError::VolumeBounds)?;
            result.normals[i][lane] = float_at(volume, offset)?;
        }
        result.heights[i] = float_at(
            volume,
            heights
                .checked_add(i * 4)
                .ok_or(ParameterizationError::VolumeBounds)?,
        )?;
    }
    Ok(result)
}
fn coefficients<N: BlendSpaceNumeric>(
    geometry: &VolumeGeometry,
    query: [f32; 4],
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    let mut result = [0.; 4];
    for (i, coefficient) in result.iter_mut().enumerate() {
        let mut delta =
            std::array::from_fn(|lane| query[lane] - geometry.vertices[(i + 1) % 4][lane]);
        delta[3] = 0.;
        *coefficient = numeric.dot4(geometry.normals[i], delta)? * geometry.heights[i];
    }
    Ok(result)
}
/// Fixed capacity source edge walk: no deduplication or geometric repair.
fn slice<N: BlendSpaceNumeric>(
    vertices: &[[f32; 4]],
    target: f32,
    output: &mut [[f32; 4]],
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let mut written = 0;
    for i in 0..vertices.len() {
        for j in i + 1..vertices.len() {
            let p = vertices[i];
            let q = vertices[j];
            if (p[0] >= target) == (q[0] >= target) {
                continue;
            }
            let denominator = q[0] - p[0];
            let numerator = target - p[0];
            let delta: [f32; 4] = std::array::from_fn(|lane| q[lane] - p[lane]);
            let seed = numeric.reciprocal_estimate(denominator)?;
            let inverse = refine_drive_reciprocal(denominator, seed);
            let usable =
                f32::from_bits(denominator.to_bits() & 0x7fff_ffff) > f32::from_bits(0x3727_c5ac);
            let scaled = numerator * inverse;
            let ratio = if usable { scaled } else { 0. };
            output[written] = std::array::from_fn(|lane| delta[lane].mul_add(ratio, p[lane]));
            written += 1;
            if written == output.len() {
                return Ok(true);
            }
            if !usable {
                output[written] = q;
                written += 1;
                if written == output.len() {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}
fn nearest(vertices: &[[f32; 4]], target: f32) -> [f32; 4] {
    let mut best = f32::MAX;
    let mut selected = [0.; 4];
    for vertex in vertices {
        let delta = vertex[0] - target;
        let distance = delta * delta;
        if best > distance {
            best = distance;
            selected = *vertex;
        }
    }
    selected
}
fn rotate(value: [f32; 4]) -> [f32; 4] {
    [value[1], value[2], value[3], value[0]]
}
fn sliced_candidate<N: BlendSpaceNumeric>(
    vertices: &[[f32; 4]; 4],
    query: [f32; 4],
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    let mut triangle = [[0.; 4]; 3];
    if !slice(vertices, query[0], &mut triangle, numeric)? {
        let best = nearest(vertices, query[0]);
        return Ok([best[0], best[1], best[2], 1.]);
    }
    let triangle = triangle.map(rotate);
    let mut line = [[0.; 4]; 2];
    if !slice(&triangle, query[1], &mut line, numeric)? {
        let best = nearest(&triangle, query[1]);
        return Ok([query[0], best[0], best[1], 1.]);
    }
    let line = line.map(rotate);
    // 730e0..73154 probes SAME endpoint twice: XOR is always zero, even NaN.
    // Preserve the native fallback; do not replace with line/plane intersection.
    let best = nearest(&line, query[2]);
    Ok([query[0], query[1], best[0], 1.])
}
/// Authored volume order, strict best-distance ties, and native output-store order.
/// `use_slicing` samples the live cvar separately for each outside volume.
#[expect(
    clippy::needless_pass_by_value,
    reason = "consumes exclusive output lane borrows for one worker invocation"
)]
pub(crate) fn compute_parameterization3<N: BlendSpaceNumeric>(
    query: [f32; 4],
    volumes: &[AndaleBlendSpaceVolume],
    output: ParameterizationOutput<'_>,
    numeric: &mut N,
    mut use_slicing: impl FnMut() -> Result<bool, N::Error>,
) -> Result<(), ParameterizationError<N::Error>> {
    use ParameterizationError::Numeric;
    *output.needs_projection = false;
    let mut best_point = [0.; 4];
    let mut best_coefficients = [0.; 4];
    let mut best_distance = 10000.;
    for (index, volume) in volumes.iter().enumerate() {
        let geometry = geometry(volume)?;
        let mut weights = coefficients(&geometry, query, numeric).map_err(Numeric)?;
        if weights.iter().all(|&value| value >= 0.) {
            *output.found = Some(index);
            *output.needs_projection = false;
            best_point = query;
            best_coefficients = weights;
            break;
        }
        if use_slicing().map_err(Numeric)? {
            let candidate =
                sliced_candidate(&geometry.vertices, query, numeric).map_err(Numeric)?;
            weights = coefficients(&geometry, candidate, numeric).map_err(Numeric)?;
        }
        let positive = weights.map(|value| if value > 0. { value } else { 0. });
        let sum = numeric.dot4([1.; 4], positive).map_err(Numeric)?;
        let seed = numeric.reciprocal_estimate(sum).map_err(Numeric)?;
        let inverse = refine_drive_reciprocal(sum, seed);
        let weights = positive.map(|value| value * inverse);
        let mut candidate = [0.; 4];
        for lane in [0, 3, 2, 1] {
            let row = std::array::from_fn(|i| geometry.vertices[i][lane]);
            candidate[lane] = numeric.dot4(row, weights).map_err(Numeric)?;
        }
        let delta = std::array::from_fn(|lane| candidate[lane] - query[lane]);
        let distance = numeric.dot4(delta, delta).map_err(Numeric)?;
        if best_distance > distance {
            *output.found = Some(index);
            *output.needs_projection = true;
            best_point = candidate;
            best_distance = distance;
            best_coefficients = weights;
        }
    }
    let nonnegative = numeric
        .maximum(best_coefficients, [0.; 4])
        .map_err(Numeric)?;
    output.projected[0] = best_point[0];
    let clamped = numeric.minimum(nonnegative, [1.; 4]).map_err(Numeric)?;
    output.projected[1] = best_point[1];
    let sum = numeric.dot4([1.; 4], clamped).map_err(Numeric)?;
    output.projected[2] = best_point[2];
    let seed = numeric.reciprocal_estimate(sum).map_err(Numeric)?;
    output.projected[3] = best_point[3];
    let inverse = refine_drive_reciprocal(sum, seed);
    for (destination, weight) in output.coefficients.iter_mut().zip(clamped) {
        *destination = weight * inverse;
    }
    *output.ready = true;
    Ok(())
}

#[cfg(test)]
#[path = "actor_animation_blend_space_parameterization_tests.rs"]
mod tests;
