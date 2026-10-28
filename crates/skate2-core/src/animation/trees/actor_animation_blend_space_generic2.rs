//! Generic two-dimensional `BlendSpace::UpdateParametrisation`, 0x82d76a08.
//! This is the pure numeric portion; the caller owns live cvar reads, clip time,
//! debug output, and the node's destination vectors.

use super::parameterization::float_at;
use skate2_formats::andale::AndaleBlendSpaceVolume;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Generic2Result {
    pub(crate) selected: Option<usize>,
    pub(crate) projected: [f32; 3],
    pub(crate) coefficients: [f32; 3],
    pub(crate) needs_projection: bool,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Generic2Error<E> {
    VolumeBounds,
    SlicingCvar(E),
    /// Native assertion at 0x82d76ef8. A host caller must not publish this result.
    CoefficientSum(f32),
}

struct Triangle {
    vertices: [[f32; 3]; 3],
    normals: [[f32; 2]; 3],
    heights: [f32; 3],
}

fn geometry<E>(volume: &AndaleBlendSpaceVolume) -> Result<Triangle, Generic2Error<E>> {
    let normals =
        usize::try_from(volume.normals_offset).map_err(|_| Generic2Error::VolumeBounds)?;
    let heights =
        usize::try_from(volume.heights_offset).map_err(|_| Generic2Error::VolumeBounds)?;
    let mut triangle = Triangle {
        vertices: [[0.; 3]; 3],
        normals: [[0.; 2]; 3],
        heights: [0.; 3],
    };
    for i in 0..3 {
        for lane in 0..3 {
            triangle.vertices[i][lane] = float_at::<E>(volume, 20 + i * 12 + lane * 4)
                .map_err(|_| Generic2Error::VolumeBounds)?;
        }
        for lane in 0..2 {
            let offset = normals
                .checked_add(4 + i * 12 + lane * 4)
                .ok_or(Generic2Error::VolumeBounds)?;
            triangle.normals[i][lane] =
                float_at::<E>(volume, offset).map_err(|_| Generic2Error::VolumeBounds)?;
        }
        triangle.heights[i] = float_at::<E>(
            volume,
            heights
                .checked_add(i * 4)
                .ok_or(Generic2Error::VolumeBounds)?,
        )
        .map_err(|_| Generic2Error::VolumeBounds)?;
    }
    Ok(triangle)
}

/// `SolveVolume`, 0x82d6caf8. The normal is an `NVector` of two floats.
fn solve(triangle: &Triangle, point: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| {
        let anchor = triangle.vertices[(i + 1) % 3];
        let delta_x = point[0] - anchor[0];
        let delta_y = point[1] - anchor[1];
        let dot = delta_x.mul_add(triangle.normals[i][0], 0.0);
        delta_y.mul_add(triangle.normals[i][1], dot) * triangle.heights[i]
    })
}

/// `Slice`, 0x82d6c230: fixed `n - 1` capacity, then replicate the final
/// intersection if fewer rows were written. The source excludes the final
/// homogeneous coordinate before dropping the sliced lane.
fn slice<const N: usize, const M: usize>(
    rows: &[[f32; N]; M],
    target: f32,
    active_coords: usize,
    output: &mut [[f32; N]; M],
) -> bool {
    let capacity = M - 1;
    let mut written = 0;
    for first in 0..M {
        for second in first + 1..M {
            if written == capacity {
                break;
            }
            let a = rows[first];
            let b = rows[second];
            // PPC `bge` after `fcmpu` means !LT, including unordered NaN.
            if (a[0] < target) == (b[0] < target) {
                continue;
            }
            let denominator = b[0] - a[0];
            if !(denominator.abs() > f32::from_bits(0x3727_c5ac)) {
                for endpoint in [a, b] {
                    if written == capacity {
                        break;
                    }
                    output[written] = shift(endpoint, active_coords);
                    written += 1;
                }
            } else {
                let ratio = (target - a[0]) / denominator;
                let mut row = [0.; N];
                for lane in 1..active_coords {
                    row[lane - 1] = (b[lane] - a[lane]).mul_add(ratio, a[lane]);
                }
                output[written] = row;
                written += 1;
            }
        }
    }
    if written == 0 {
        return false;
    }
    while written < capacity {
        output[written] = output[written - 1];
        written += 1;
    }
    true
}

fn shift<const N: usize>(row: [f32; N], active_coords: usize) -> [f32; N] {
    let mut shifted = [0.; N];
    shifted[..active_coords - 1].copy_from_slice(&row[1..active_coords]);
    shifted
}

/// `Projection`, 0x82d6c700. The source's `bge` skip keeps the first tie
/// and skips an unordered (NaN) distance.
fn nearest<const N: usize, const M: usize>(rows: &[[f32; N]; M], target: f32) -> [f32; N] {
    let mut best = f32::MAX;
    let mut selected = [0.; N];
    for row in rows {
        let delta = row[0] - target;
        let distance = delta * delta;
        if distance < best {
            best = distance;
            selected = *row;
        }
    }
    selected
}

fn sliced_candidate(triangle: &Triangle, query: [f32; 3]) -> [f32; 3] {
    let mut line = [[0.; 3]; 3];
    if !slice(&triangle.vertices, query[0], 2, &mut line) {
        let closest = nearest(&triangle.vertices, query[0]);
        return [closest[0], closest[1], 1.];
    }
    let mut point = [[0.; 3]; 2];
    if !slice(&[line[0], line[1]], query[1], 1, &mut point) {
        let closest = nearest(&[line[0], line[1]], query[1]);
        return [query[0], closest[0], 1.];
    }
    [query[0], query[1], 1.]
}

/// `NVector::Projection`, 0x82d6aff8. Division remains scalar even for zero.
fn project_coefficients(weights: &mut [f32; 3]) {
    let mut sum = 0.;
    for weight in weights.iter_mut() {
        if *weight < 0. {
            *weight = 0.;
        } else {
            sum += *weight;
        }
    }
    let inverse = 1. / sum;
    for weight in weights.iter_mut() {
        *weight *= inverse;
    }
}

/// Generic `UpdateParametrisation`, 0x82d76a08, for a two-dimensional set.
/// The callback is invoked once per outside volume, at the original cvar site.
pub(crate) fn compute_parameterization2<E>(
    query: [f32; 3],
    volumes: &[AndaleBlendSpaceVolume],
    mut use_slicing: impl FnMut() -> Result<bool, E>,
) -> Result<Generic2Result, Generic2Error<E>> {
    let mut result = Generic2Result {
        selected: None,
        projected: [0.; 3],
        coefficients: [0.; 3],
        needs_projection: false,
    };
    let mut scratch_point = [0.; 3];
    let mut best_distance = 10000.;
    for (index, volume) in volumes.iter().enumerate() {
        let triangle = geometry(volume)?;
        let mut weights = solve(&triangle, query);
        if weights.iter().all(|weight| *weight >= 0.) {
            result.selected = Some(index);
            result.projected = query;
            result.coefficients = weights;
            result.needs_projection = false;
            break;
        }
        if use_slicing().map_err(Generic2Error::SlicingCvar)? {
            scratch_point = sliced_candidate(&triangle, query);
            weights = solve(&triangle, scratch_point);
        }
        project_coefficients(&mut weights);
        for lane in 0..2 {
            let mut coordinate = 0.;
            for (i, weight) in weights.iter().enumerate() {
                coordinate = triangle.vertices[i][lane].mul_add(*weight, coordinate);
            }
            scratch_point[lane] = coordinate;
        }
        let mut distance = 0.;
        for lane in 0..2 {
            let delta = scratch_point[lane] - query[lane];
            distance = delta.mul_add(delta, distance);
        }
        if distance < best_distance {
            result.selected = Some(index);
            result.projected = scratch_point;
            result.coefficients = weights;
            result.needs_projection = true;
            best_distance = distance;
        }
    }
    let mut sum = 0.;
    for weight in &mut result.coefficients {
        // Two `fsel` instructions: the second selects 1 for an unordered NaN.
        let nonnegative = if -*weight >= 0. { 0. } else { *weight };
        *weight = if 1. - nonnegative >= 0. {
            nonnegative
        } else {
            1.
        };
        sum += *weight;
    }
    if !(sum > 0.7 && sum < 1.3) {
        return Err(Generic2Error::CoefficientSum(sum));
    }
    let inverse = 1. / sum;
    for weight in &mut result.coefficients {
        *weight *= inverse;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> AndaleBlendSpaceVolume {
        // Vertices (0,0), (1,0), (0,1) with barycentric plane normals.
        let vertices = [[0., 0., 1.], [1., 0., 1.], [0., 1., 1.]];
        let normals = [[-1., -1.], [1., 0.], [0., 1.]];
        let mut bytes = vec![0; 108];
        for (i, value) in vertices.into_iter().flatten().enumerate() {
            bytes[20 + i * 4..24 + i * 4].copy_from_slice(&f32::to_be_bytes(value));
        }
        for (i, value) in normals.into_iter().flatten().enumerate() {
            let row = i / 2;
            let lane = i % 2;
            let at = 60 + row * 12 + lane * 4;
            bytes[at..at + 4].copy_from_slice(&f32::to_be_bytes(value));
        }
        for i in 0..3 {
            bytes[96 + i * 4..100 + i * 4].copy_from_slice(&1f32.to_be_bytes());
        }
        AndaleBlendSpaceVolume {
            animation_indexes_offset: 0,
            heights_offset: 96,
            normals_offset: 56,
            matrix_coordinates: vec![],
            serialized_data: bytes,
            serialized_following_word: None,
        }
    }

    #[test]
    fn first_inside_volume_keeps_query_and_does_not_sample_cvar() {
        let result = compute_parameterization2(
            [0.25, 0.25, 1.],
            &[triangle(), triangle()],
            || -> Result<bool, ()> { panic!("inside must not sample slicing") },
        )
        .unwrap();
        assert_eq!(result.selected, Some(0));
        assert!(!result.needs_projection);
        assert_eq!(result.projected, [0.25, 0.25, 1.]);
        assert_eq!(result.coefficients, [0.5, 0.25, 0.25]);
    }

    #[test]
    fn outside_volume_samples_live_cvar_then_projects() {
        let mut calls = 0;
        let result = compute_parameterization2([2., 0., 1.], &[triangle(), triangle()], || {
            calls += 1;
            Ok::<_, ()>(calls == 1)
        })
        .unwrap();
        assert_eq!(calls, 2);
        assert_eq!(result.selected, Some(0));
        assert!(result.needs_projection);
        assert_eq!(result.projected, [1., 0., 1.]);
        assert_eq!(result.coefficients, [0., 1., 0.]);
    }

    #[test]
    fn slice_pair_order_tiny_crossing_and_nearest_ties() {
        let vertices = [[0., 1., 1.], [2., 3., 1.], [3., 7., 1.]];
        let mut sliced = [[0.; 3]; 3];
        assert!(slice(&vertices, 1., 2, &mut sliced));
        assert_eq!(sliced[0], [2., 0., 0.]);
        assert_eq!(sliced[1], [3., 0., 0.]);
        let tiny = [[0., 4., 1.], [0.000_001, 9., 1.], [1., 11., 1.]];
        assert!(slice(&tiny, 0.000_000_5, 2, &mut sliced));
        assert_eq!(sliced[0], [4., 0., 0.]);
        assert_eq!(sliced[1], [9., 0., 0.]);
        let unordered = [[f32::NAN, 4., 1.], [1., 9., 1.], [2., 11., 1.]];
        assert!(!slice(&unordered, 0., 2, &mut sliced));
        let unordered_crossing = [[-1., 4., 1.], [f32::NAN, 9., 1.], [2., 11., 1.]];
        assert!(slice(&unordered_crossing, 0., 2, &mut sliced));
        assert_eq!(sliced[0], [4., 0., 0.]);
        assert_eq!(sliced[1], [9., 0., 0.]);
        assert_eq!(nearest(&[[0., 4.], [2., 9.]], 1.), [0., 4.]);
        assert_eq!(nearest(&[[0., 4.], [2., 9.]], f32::NAN), [0., 0.]);
        let mut weights = [f32::NAN, 1., 0.];
        project_coefficients(&mut weights);
        assert!(weights.iter().all(|value| value.is_nan()));
    }
}
