//! `Skeleton::ReorientSkaterUpright` 0x82b3b668. This isolated leaf does not
//! publish a root transform until its caller reaches the native write site.

use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_full4},
    reckoning_sin_cos::ReckoningSinCosNumeric,
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
    skeleton_affine_interpolation::{
        SkeletonAffineInterpolationError, try_interpolate_affine_typed,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

const VECFLOAT_SMALL: f32 = f32::from_bits(0x3780_0000);
const UPRIGHT_BLEND: f32 = f32::from_bits(0x3dcc_cccd);
const WORLD_UP: SkeletonRootVector = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ReorientError<N, R> {
    Numeric(N),
    Interpolation(SkeletonAffineInterpolationError<N, R>),
}

/// Returns `None` at either native early return, retaining the exact caller
/// matrix. `Some` is the complete four-column value written at 0x82b3b804.
pub(crate) fn try_reorient_skater_upright<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    animation_to_world: &SkeletonRootMatrix,
    numeric: &mut N,
    trigonometry: &mut R,
) -> Result<Option<SkeletonRootMatrix>, ReorientError<N::Error, R::Error>> {
    let columns = animation_to_world.columns();
    let first = cross(columns[1], WORLD_UP);
    let first_squared = numeric
        .dot3(dot_lanes(first), dot_lanes(first))
        .map_err(ReorientError::Numeric)?;
    // 0x82b3b6e4: ordered, strict VMX greater-than; equality and NaN return.
    if !(first_squared > VECFLOAT_SMALL) {
        return Ok(None);
    }

    let second = cross(columns[1], columns[2]);
    let second_squared = numeric
        .dot3(dot_lanes(second), dot_lanes(second))
        .map_err(ReorientError::Numeric)?;
    if !(second_squared > VECFLOAT_SMALL) {
        return Ok(None);
    }

    let estimate = numeric
        .reciprocal_sqrt_estimate(second_squared)
        .map_err(ReorientError::Numeric)?;
    let inverse = refine_drive_reciprocal_sqrt(second_squared, estimate);
    let right = second.map(|lane| lane * inverse);
    let forward = cross(right, WORLD_UP);
    let upright = SkeletonRootMatrix::from_columns([right, WORLD_UP, forward, columns[3]]);
    try_interpolate_affine_typed(
        animation_to_world,
        &upright,
        UPRIGHT_BLEND,
        numeric,
        trigonometry,
    )
    .map(Some)
    .map_err(ReorientError::Interpolation)
}

fn dot_lanes(value: SkeletonRootVector) -> [f32; 4] {
    [
        value.xyz[0],
        value.xyz[1],
        value.xyz[2],
        value.w.unwrap_or(0.0),
    ]
}

fn cross(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    let xyz = cross_full4(dot_lanes(left), dot_lanes(right));
    let w = left
        .w
        .zip(right.w)
        .map(|(left_w, right_w)| cross_full4([0.0, 0.0, 0.0, left_w], [0.0, 0.0, 0.0, right_w])[3]);
    SkeletonRootVector {
        xyz: [xyz[0], xyz[1], xyz[2]],
        w,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        reckoning_numeric::HostReckoningNumeric, reckoning_sin_cos::HostReckoningSinCosNumeric,
    };

    fn matrix(y: [f32; 4], z: [f32; 4]) -> SkeletonRootMatrix {
        SkeletonRootMatrix::from_columns([
            SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
            SkeletonRootVector::known(y),
            SkeletonRootVector::known(z),
            SkeletonRootVector::known([4.0, 5.0, 6.0, 1.0]),
        ])
    }

    #[test]
    fn upright_and_parallel_axes_return_without_a_matrix_write() {
        let mut numeric = HostReckoningNumeric;
        let mut trig = HostReckoningSinCosNumeric;
        assert_eq!(
            try_reorient_skater_upright(
                &matrix([0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]),
                &mut numeric,
                &mut trig,
            )
            .unwrap(),
            None,
        );
        assert_eq!(
            try_reorient_skater_upright(
                &matrix([1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]),
                &mut numeric,
                &mut trig,
            )
            .unwrap(),
            None,
        );
    }

    #[test]
    fn tilted_axes_blend_toward_upright_and_retain_translation() {
        let mut input = matrix([0.6, 0.8, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]);
        input.set_column(0, SkeletonRootVector::known([0.8, -0.6, 0.0, 0.0]));
        let result = try_reorient_skater_upright(
            &input,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.columns()[3], input.columns()[3]);
        assert!(result.columns()[1].xyz[0] < 0.6);
        assert!(result.columns()[1].xyz[1] > 0.8);
    }
}
