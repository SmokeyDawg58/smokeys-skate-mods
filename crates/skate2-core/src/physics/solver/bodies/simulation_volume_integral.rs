//! Authored mapped-array homogeneous mass integrals and source Jacobi solve.
//!
//! Source: `ComputeMappedArrayIntegral` (0x82ae8028), `ConstructIntegral`
//! (0x82ae90f0), `TransformIntegral` (0x82ae91c0), and
//! `DiagonaliseVolumeIntegral` (0x82ae9560). Contributions are ordered sums,
//! not geometric unions. This is not general `ComputeMassProperties` or an
//! arbitrary arena aggregate resolver. Host exceptional arithmetic and estimate
//! results remain explicit boundaries, not captured Xenon numeric observations.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "authored volume-integral producer staged for the seven-part skateboard owner"
    )
)]

use crate::{
    serialised_assembly_bbox::{
        HostSerialisedAssemblyBBoxNumeric, SerialisedAssemblyBBoxMinMaxNumeric,
        SerialisedAssemblyPartBBoxNumeric, try_get_primitive_bbox,
    },
    skateboard_wheels::{capsule_unit_mass, sphere_unit_mass},
};
use skate2_formats::collision::{Volume, VolumeType};

#[cfg(test)]
#[path = "simulation_volume_integral_tests.rs"]
mod tests;

#[path = "simulation_volume_integral_arena.rs"]
pub(crate) mod arena;

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Four stored homogeneous second-moment columns, not an inertia tensor.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SimulationVolumeIntegral {
    pub(crate) columns: [[f32; 4]; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SimulationMappedArrayIntegral {
    pub(crate) integral: SimulationVolumeIntegral,
    /// Inner mapped-array return, not the outer recognized-aggregate return.
    /// An empty/all-Null array still returns Ok with a zero outer integral.
    pub(crate) any_child_succeeded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SimulationVolumePrincipalMass {
    pub(crate) diagonal: [f32; 3],
    pub(crate) volume: f32,
    /// Three transposed axes (each W=Y), then COM (W=X). No invented affine W.
    pub(crate) body_to_part: [f32; 16],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationVolumeReciprocalSite {
    CenterVolume,
    FinalVolume,
    Schur { p: usize, q: usize },
}

/// Only platform numeric operations not fixed by the source f32/FMA graph.
/// Capsule unit mass reuses the previously exposed host scalar division seam;
/// Schur's separate `fsqrts`/`fdivs` are explicit here, never reciprocal seeds.
pub(crate) trait SimulationVolumeIntegralNumeric: SerialisedAssemblyPartBBoxNumeric {
    fn reciprocal_estimate(
        &mut self,
        site: SimulationVolumeReciprocalSite,
        value: f32,
    ) -> Result<f32, Self::Error>;
    fn scalar_sqrt(&mut self, value: f32) -> Result<f32, Self::Error>;
    fn scalar_divide(&mut self, numerator: f32, denominator: f32) -> Result<f32, Self::Error>;
}

#[derive(Default)]
pub(crate) struct HostSimulationVolumeIntegralNumeric;

impl SerialisedAssemblyBBoxMinMaxNumeric for HostSimulationVolumeIntegralNumeric {
    type Error = core::convert::Infallible;
    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        HostSerialisedAssemblyBBoxNumeric.minimum(left, right)
    }
    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        HostSerialisedAssemblyBBoxNumeric.maximum(left, right)
    }
}

impl SerialisedAssemblyPartBBoxNumeric for HostSimulationVolumeIntegralNumeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        HostSerialisedAssemblyBBoxNumeric.sum_three_products(left, right)
    }
    fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        HostSerialisedAssemblyBBoxNumeric.reciprocal_square_root_estimate(value)
    }
}

impl SimulationVolumeIntegralNumeric for HostSimulationVolumeIntegralNumeric {
    fn reciprocal_estimate(
        &mut self,
        _site: SimulationVolumeReciprocalSite,
        value: f32,
    ) -> Result<f32, Self::Error> {
        Ok(value.recip())
    }
    fn scalar_sqrt(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(value.sqrt())
    }
    fn scalar_divide(&mut self, numerator: f32, denominator: f32) -> Result<f32, Self::Error> {
        Ok(numerator / denominator)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum SimulationVolumeIntegralError<E> {
    /// Host subset boundary, not a native unsupported-volume return value.
    UnsupportedVolume {
        ordinal: usize,
        volume_type: Option<VolumeType>,
    },
    Numeric(E),
}

/// `ComputeMappedArrayIntegral`, 0x82ae8028, composed with recognized outer
/// aggregate handling in `ComputeVolumeIntegral`, 0x82ae9878.
///
/// Zero output precedes child traversal. Empty input touches no child; typed
/// borrowed storage does not invent a native child-pointer read. Child flags
/// never filter mass. The aggregate's own transform runs only on inner success.
pub(crate) fn try_compute_mapped_array_integral<N: SimulationVolumeIntegralNumeric>(
    children: &[Volume],
    aggregate_transform: Option<&[f32; 16]>,
    numeric: &mut N,
) -> Result<SimulationMappedArrayIntegral, SimulationVolumeIntegralError<N::Error>> {
    try_sum_integrals(
        children
            .iter()
            .enumerate()
            .map(|(ordinal, child)| try_child_integral(child, ordinal, numeric)),
        aggregate_transform,
    )
}

/// Shared ordered accumulation for constructed and arena-backed mapped arrays.
/// The outer recognized aggregate succeeds even when no child succeeds.
fn try_sum_integrals<E>(
    children: impl IntoIterator<Item = Result<Option<SimulationVolumeIntegral>, E>>,
    aggregate_transform: Option<&[f32; 16]>,
) -> Result<SimulationMappedArrayIntegral, E> {
    let mut result = SimulationMappedArrayIntegral {
        integral: SimulationVolumeIntegral {
            columns: [[0.0; 4]; 4],
        },
        any_child_succeeded: false,
    };
    for child in children {
        let Some(integral) = child? else {
            continue;
        };
        for (total, contribution) in result.integral.columns.iter_mut().zip(integral.columns) {
            for (lane, value) in total.iter_mut().zip(contribution) {
                *lane += value;
            }
        }
        result.any_child_succeeded = true;
    }
    if result.any_child_succeeded {
        transform_integral(&mut result.integral, aggregate_transform);
    }
    Ok(result)
}

/// ComputeSimpleInertia82ae93c0. Primitive mass helpers are shared by the
/// skateboard, skeleton and LivingWorld; the Volume transform is the source
/// principal frame for these four primitive types.
pub(crate) fn simple_volume_principal(volume: &Volume) -> Option<SimulationVolumePrincipalMass> {
    let (diagonal, amount) = match volume.volume_type()? {
        VolumeType::Sphere => sphere_unit_mass(volume.radius),
        VolumeType::Capsule => {
            capsule_unit_mass(volume.radius, f32::from_bits(volume.shape_words[0]))
        }
        VolumeType::Box => fat_box_unit_mass(volume.shape_words.map(f32::from_bits), volume.radius),
        VolumeType::Cylinder => cylinder_unit_mass(
            f32::from_bits(volume.shape_words[1]),
            volume.radius,
            f32::from_bits(volume.shape_words[0]),
        ),
        _ => return None,
    };
    Some(SimulationVolumePrincipalMass {
        diagonal,
        volume: amount,
        body_to_part: volume.transform,
    })
}

fn try_child_integral<N: SimulationVolumeIntegralNumeric>(
    volume: &Volume,
    ordinal: usize,
    numeric: &mut N,
) -> Result<Option<SimulationVolumeIntegral>, SimulationVolumeIntegralError<N::Error>> {
    if let Some(principal) = simple_volume_principal(volume) {
        let mut integral = construct_integral(principal.diagonal, principal.volume);
        transform_integral(&mut integral, Some(&volume.transform));
        return Ok(Some(integral));
    }
    match volume.volume_type() {
        Some(VolumeType::Null) => return Ok(None),
        Some(VolumeType::Triangle) => {
            // Original Triangle path uses its own point vectors with NULL
            // parent and tight0; radius is already expanded by GetBBox.
            let bounds = try_get_primitive_bbox(volume, None, 0, numeric)
                .map_err(SimulationVolumeIntegralError::Numeric)?
                .expect("Triangle GetBBox always succeeds");
            let low = bounds.min_f32();
            let high = bounds.max_f32();
            return Ok(Some(integral_from_bbox(
                std::array::from_fn(|axis| low[axis]),
                std::array::from_fn(|axis| high[axis]),
            )));
        }
        volume_type => {
            return Err(SimulationVolumeIntegralError::UnsupportedVolume {
                ordinal,
                volume_type,
            });
        }
    }
}

/// ComputeVolumeIntegral82ae9984 fallback. This is the game's own bbox mass
/// approximation for non-mapped aggregates (including ClusteredMesh).
fn integral_from_bbox(low: [f32; 3], high: [f32; 3]) -> SimulationVolumeIntegral {
    let half_extents = std::array::from_fn(|axis| (high[axis] - low[axis]) * 0.5);
    let (diagonal, amount) = fat_box_unit_mass(half_extents, 0.0);
    let mut integral = construct_integral(diagonal, amount);
    let mut frame = IDENTITY;
    for axis in 0..3 {
        frame[3][axis] = low[axis] + half_extents[axis];
    }
    transform_integral(&mut integral, Some(&flatten(frame)));
    integral
}

/// `ComputeCylinderInertia`, 0x82ae81d0..8270. Source Cylinder has separate
/// inner radius (+0x48), outer rounding radius (+0x50), and half-height (+0x44).
/// No clamp or bbox approximation. Unit inertia is XYZ only; callers retain
/// their preexisting packed W independently where the native output leaves it.
pub(crate) fn cylinder_unit_mass(
    inner_radius: f32,
    outer_radius: f32,
    half_height: f32,
) -> ([f32; 3], f32) {
    let radius = inner_radius + outer_radius;
    let height = outer_radius + half_height;
    let radius_squared = radius * radius;
    let height_squared = height * height;
    let product = height * radius;
    let radial_three = radius_squared * 3.0;
    let axial = radius_squared * 0.5;
    let length_product = product * radius;
    let transverse_numerator = height_squared.mul_add(4.0, radial_three);
    let volume = length_product * f32::from_bits(0x40c9_0fdb);
    let transverse = transverse_numerator * f32::from_bits(0x3daa_aaab);
    ([transverse, transverse, axial], volume)
}

fn select_nonnegative(difference: f32, nonnegative: f32, negative_or_nan: f32) -> f32 {
    if difference >= 0.0 {
        nonnegative
    } else {
        negative_or_nan
    }
}

/// `ComputeFatBoxInertia`, 0x82ae7ee0. Radius is not clamped.
pub(crate) fn fat_box_unit_mass([x, y, z]: [f32; 3], radius: f32) -> ([f32; 3], f32) {
    let largest_yz = select_nonnegative(y - z, y, z);
    let largest = select_nonnegative(x - largest_yz, x, largest_yz);
    let minimum = (largest + radius) * f32::from_bits(0x3d4c_cccd);
    let epsilon = f32::from_bits(0x33d6_bf95);
    let minimum = select_nonnegative(minimum - epsilon, minimum, epsilon);
    let floor = minimum - radius;
    let clamped_x = select_nonnegative(x - floor, x, floor);
    let clamped_y = select_nonnegative(y - floor, y, floor);
    let clamped_z = select_nonnegative(z - floor, z, floor);
    let radial = (radius * radius) * 0.5;
    let third = f32::from_bits(0x3eaa_aaab);
    let diagonal_x = (((clamped_y + clamped_z) * radius).mul_add(2.0, clamped_z * clamped_z)
        + clamped_y * clamped_y)
        .mul_add(third, radial);
    let diagonal_y = (((clamped_x + clamped_z) * radius).mul_add(2.0, clamped_x * clamped_x)
        + clamped_z * clamped_z)
        .mul_add(third, radial);
    let diagonal_z = (((clamped_x + clamped_y) * radius).mul_add(2.0, clamped_x * clamped_x)
        + clamped_y * clamped_y)
        .mul_add(third, radial);
    let rounded = radius.mul_add(f32::from_bits(0x3f2a_aaab), clamped_x);
    let rounded = rounded + clamped_y;
    let rounded = rounded + clamped_z;
    let rounded = rounded * radius;
    let rounded = rounded * f32::from_bits(0x40c9_0fdb);
    let faces = (clamped_y + clamped_z).mul_add(clamped_x, clamped_y * clamped_z);
    let base = ((clamped_x * clamped_y) * clamped_z) * 8.0;
    let amount = faces.mul_add(8.0, rounded).mul_add(radius, base);
    ([diagonal_x, diagonal_y, diagonal_z], amount)
}

/// `ConstructIntegral`, 0x82ae90f0. Multiply by volume BEFORE half.
fn construct_integral([x, y, z]: [f32; 3], volume: f32) -> SimulationVolumeIntegral {
    let mut columns = [[0.0; 4]; 4];
    columns[0][0] = (((z + y) - x) * volume) * 0.5;
    columns[1][1] = (((z + x) - y) * volume) * 0.5;
    columns[2][2] = (((y + x) - z) * volume) * 0.5;
    columns[3][3] = volume;
    SimulationVolumeIntegral { columns }
}

fn flatten(columns: [[f32; 4]; 4]) -> [f32; 16] {
    std::array::from_fn(|index| columns[index / 4][index % 4])
}

/// One ordered X product, then fused Y/Z/W additions, per output lane.
fn combine_columns(columns: &[[f32; 4]; 4], weights: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| {
        let x = columns[0][lane] * weights[0];
        let xy = columns[1][lane].mul_add(weights[1], x);
        let xyz = columns[2][lane].mul_add(weights[2], xy);
        columns[3][lane].mul_add(weights[3], xyz)
    })
}

/// `TransformIntegral`, 0x82ae91c0. H Q H-transpose, with fresh homogeneous
/// W lanes. Input frame W lanes are ignored; both intermediates are complete.
fn transform_integral(integral: &mut SimulationVolumeIntegral, frame: Option<&[f32; 16]>) {
    let Some(frame) = frame else {
        return;
    };
    let homogeneous: [[f32; 4]; 4] = std::array::from_fn(|column| {
        std::array::from_fn(|lane| {
            if lane == 3 {
                if column == 3 { 1.0 } else { 0.0 }
            } else {
                frame[column * 4 + lane]
            }
        })
    });
    let intermediate = integral
        .columns
        .map(|column| combine_columns(&homogeneous, column));
    let result = std::array::from_fn(|column| {
        let transposed_column = std::array::from_fn(|lane| homogeneous[lane][column]);
        combine_columns(&intermediate, transposed_column)
    });
    integral.columns = result;
}

fn refine_twice(denominator: f32, mut inverse: f32) -> f32 {
    for _ in 0..2 {
        let residual = (-denominator).mul_add(inverse, 1.0);
        inverse = inverse.mul_add(residual, inverse);
    }
    inverse
}

/// `sym_schur2`, 0x82ae8270. Equal diagonals/nonzero cross term select the
/// NEGATIVE tau-zero root. Zero cross term alone returns identity rotation.
fn try_schur<N: SimulationVolumeIntegralNumeric>(
    matrix: &[[f32; 3]; 3],
    p: usize,
    q: usize,
    numeric: &mut N,
) -> Result<(f32, f32), N::Error> {
    let cross = matrix[p][q];
    if cross == 0.0 {
        return Ok((1.0, 0.0));
    }
    let inverse = refine_twice(
        cross,
        numeric.reciprocal_estimate(SimulationVolumeReciprocalSite::Schur { p, q }, cross)?,
    );
    let tau = inverse * ((matrix[q][q] - matrix[p][p]) * 0.5);
    let root = numeric.scalar_sqrt(tau.mul_add(tau, 1.0))?;
    let tangent = if tau > 0.0 {
        numeric.scalar_divide(1.0, root + tau)?
    } else {
        numeric.scalar_divide(-1.0, root - tau)?
    };
    let length = numeric.scalar_sqrt(tangent.mul_add(tangent, 1.0))?;
    let cosine = numeric.scalar_divide(1.0, length)?;
    Ok((cosine, cosine * tangent))
}

/// Capture old pairs; write every p entry before any q entry. The p arm's
/// subtraction is NOT fused; the q arm's rounded v*c precedes its fused sum.
fn rotate_columns(matrix: &mut [[f32; 3]; 3], p: usize, q: usize, cosine: f32, sine: f32) {
    let old_p: [f32; 3] = std::array::from_fn(|row| matrix[row][p]);
    let old_q: [f32; 3] = std::array::from_fn(|row| matrix[row][q]);
    for row in 0..3 {
        matrix[row][p] = (old_p[row] * cosine) - (old_q[row] * sine);
    }
    for row in 0..3 {
        matrix[row][q] = old_p[row].mul_add(sine, old_q[row] * cosine);
    }
}

fn rotate_rows(matrix: &mut [[f32; 3]; 3], p: usize, q: usize, cosine: f32, sine: f32) {
    let old_p = matrix[p];
    let old_q = matrix[q];
    for column in 0..3 {
        matrix[p][column] = (old_p[column] * cosine) - (old_q[column] * sine);
    }
    for column in 0..3 {
        matrix[q][column] = old_p[column].mul_add(sine, old_q[column] * cosine);
    }
}

/// `JacobiRowCyclic`, 0x82ae8b20: native pair order and remaining-count return.
/// Only xyz storage exists here; no claim that native temporary W was initialized.
fn try_jacobi<N: SimulationVolumeIntegralNumeric>(
    matrix: &mut [[f32; 3]; 3],
    remaining: usize,
    tolerance: f32,
    numeric: &mut N,
) -> Result<([[f32; 3]; 3], usize), N::Error> {
    let mut remaining = remaining;
    let mut rotation = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    loop {
        let xx = matrix[0][0] * matrix[0][0];
        let yy = matrix[1][1] * matrix[1][1];
        let zz = matrix[2][2] * matrix[2][2];
        let scale = if yy > xx {
            if zz > xx { xx } else { zz }
        } else if zz > yy {
            yy
        } else {
            zz
        };
        let off = matrix[2][1].mul_add(
            matrix[2][1],
            matrix[1][0].mul_add(matrix[1][0], matrix[2][0] * matrix[2][0]),
        );
        let threshold = scale * (tolerance * tolerance);
        if off > threshold && remaining != 0 {
            for (p, q) in [(1, 0), (2, 0), (2, 1)] {
                let (cosine, sine) = try_schur(matrix, p, q, numeric)?;
                rotate_columns(&mut rotation, p, q, cosine, sine);
                rotate_columns(matrix, p, q, cosine, sine);
                rotate_rows(matrix, p, q, cosine, sine);
            }
            remaining -= 1;
        } else {
            break;
        }
    }
    Ok((rotation, remaining))
}

/// `DiagonaliseVolumeIntegral`, 0x82ae9560. Centers the input integral in-place.
/// No zero-volume, nonfinite, positive-eigenvalue, or sorting guard is added.
pub(crate) fn try_diagonalise_volume_integral<N: SimulationVolumeIntegralNumeric>(
    integral: &mut SimulationVolumeIntegral,
    numeric: &mut N,
) -> Result<SimulationVolumePrincipalMass, N::Error> {
    let volume = integral.columns[3][3];
    let center_inverse = refine_twice(
        volume,
        numeric.reciprocal_estimate(SimulationVolumeReciprocalSite::CenterVolume, volume)?,
    );
    let center: [f32; 3] = std::array::from_fn(|axis| integral.columns[3][axis] * center_inverse);
    let mut centering_frame = IDENTITY;
    centering_frame[3] = [-center[0], -center[1], -center[2], -center[0]];
    transform_integral(integral, Some(&flatten(centering_frame)));
    let q = &integral.columns;
    let xx = q[1][1] + q[2][2];
    let yy = q[0][0] + q[2][2];
    let zz = q[0][0] + q[1][1];
    let xy = (q[1][0] + q[0][1]) * -0.5;
    let yz = (q[2][1] + q[1][2]) * -0.5;
    let xz = (q[2][0] + q[0][2]) * -0.5;
    let mut tensor = [[xx, xy, xz], [xy, yy, yz], [xz, yz, zz]];
    let (rotation, _) = try_jacobi(&mut tensor, 10, f32::from_bits(0x2b8c_bccc), numeric)?;
    let mut body_to_part = [[0.0; 4]; 4];
    for axis in 0..3 {
        body_to_part[axis] = [
            rotation[0][axis],
            rotation[1][axis],
            rotation[2][axis],
            rotation[1][axis],
        ];
    }
    body_to_part[3] = [center[0], center[1], center[2], center[0]];
    let [xx, yy, zz] = [tensor[0][0], tensor[1][1], tensor[2][2]];
    let yz = select_nonnegative(yy - zz, yy, zz);
    let largest = select_nonnegative(xx - yz, xx, yz);
    let floor = largest * f32::from_bits(0x3ba3_d70b);
    let clamped = [xx, yy, zz].map(|axis| select_nonnegative(axis - floor, axis, floor));
    let final_inverse = refine_twice(
        volume,
        numeric.reciprocal_estimate(SimulationVolumeReciprocalSite::FinalVolume, volume)?,
    );
    Ok(SimulationVolumePrincipalMass {
        diagonal: clamped.map(|axis| axis * final_inverse),
        volume,
        body_to_part: flatten(body_to_part),
    })
}
