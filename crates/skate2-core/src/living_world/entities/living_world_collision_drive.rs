//! Numeric prefix of cLivingWorldCollision.UpdateCollisionBodyPartsFromTransform
//! 82e506d8. Actual body publication, gravity cancellation and activation belong
//! to its physics session. This does not teleport bodies to their target pose.
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_rain::geometry::Transform,
    reckoning_numeric::{
        ReckoningClampError, ReckoningNumeric,
        matrix33::{RotationVectorError, try_orthonormalize, try_rotation_vector},
        try_clamp,
    },
    reckoning_sin_cos::ReckoningSinCosNumeric,
};
pub(crate) struct CollisionPartDrive {
    pub(crate) velocity: [f32; 4],
    pub(crate) omega: [f32; 4],
}
#[derive(Debug)]
pub(crate) enum CollisionDriveError<N, T> {
    Numeric(N),
    Rotation(RotationVectorError<N, T>),
    Cvar(InputCvarError),
    Clamp(ReckoningClampError<N>),
}

/// FillCollisionPartTargetTransforms82e5f8d8 and shift82e50ae0, raw VMX.
/// Pipe geometry keeps its authored origin; constructed shapes move half the
/// model height along target up. Predict position by the fixed kUpdateTime,
/// not the caller's frame dT. Preserve all four lanes and fused multiply-add.
pub(crate) fn collision_part_target(
    mut target: Transform,
    velocity: [f32; 4],
    from_pipe: bool,
    model_height: f32,
) -> Transform {
    if !from_pipe {
        let half_height = model_height * 0.5;
        for lane in 0..4 {
            target[3][lane] += target[1][lane] * half_height;
        }
    }
    for lane in 0..4 {
        target[3][lane] = velocity[lane].mul_add(f32::from_bits(0x3c88_8889), target[3][lane]);
    }
    target
}

#[cfg(test)]
mod target_tests {
    use super::*;

    #[test]
    fn constructed_target_offsets_along_up_and_predicts_all_four_lanes() {
        let target = [
            [1., 0., 0., 0.],
            [0., 1., 0., 2.],
            [0., 0., 1., 0.],
            [5., 6., 7., 8.],
        ];
        let velocity = [60., 120., 180., 240.];
        let pipe = collision_part_target(target, velocity, true, 4.);
        let constructed = collision_part_target(target, velocity, false, 4.);
        assert_eq!(pipe[3], [6., 8., 10., 12.]);
        assert_eq!(constructed[3], [6., 10., 10., 16.]);
        assert_eq!(&constructed[..3], &target[..3]);
    }
}
pub(crate) fn try_part_drive<N: ReckoningNumeric, T: ReckoningSinCosNumeric>(
    old: Transform,
    target: Transform,
    dt: f32,
    cvars: &InputCvarReferences,
    numeric: &mut N,
    trig: &mut T,
) -> Result<CollisionPartDrive, CollisionDriveError<N::Error, T::Error>> {
    let inverse_dt = 1. / dt; // scalar fdivs82e50740, not VMX refined reciprocal
    let unchanged = (0..3).all(|axis| {
        (0..3).all(|lane| {
            !((target[axis][lane] - old[axis][lane]).abs() > f32::from_bits(0x3780_0000))
        })
    });
    let relative = if unchanged {
        [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]]
    } else {
        // Raw merge sequence82e508b4: transpose old XYZ, padding lane=Y.
        let transpose: [[f32; 4]; 3] =
            std::array::from_fn(|i| [old[0][i], old[1][i], old[2][i], old[1][i]]);
        let matrix = std::array::from_fn(|axis| {
            std::array::from_fn(|lane| {
                let first = transpose[0][lane] * target[axis][0];
                let second = transpose[1][lane].mul_add(target[axis][1], first);
                transpose[2][lane].mul_add(target[axis][2], second)
            })
        });
        try_orthonormalize(matrix, numeric).map_err(CollisionDriveError::Numeric)?
    };
    let rotation =
        try_rotation_vector(relative, numeric, trig).map_err(CollisionDriveError::Rotation)?;
    let mut omega: [f32; 4] = std::array::from_fn(|lane| {
        let first = old[0][lane] * rotation[0];
        let second = old[1][lane].mul_add(rotation[1], first);
        old[2][lane].mul_add(rotation[2], second)
    });
    let mut velocity = std::array::from_fn(|lane| (target[3][lane] - old[3][lane]) * inverse_dt);
    if cvars
        .try_living_world_clamp_velocity()
        .map_err(CollisionDriveError::Cvar)?
    {
        let maximum = cvars
            .try_living_world_clamp_max_velocity()
            .map_err(CollisionDriveError::Cvar)?;
        velocity = try_clamp(velocity, maximum, numeric).map_err(CollisionDriveError::Clamp)?;
    }
    omega = omega.map(|v| v * inverse_dt); // after the optional linear clamp
    Ok(CollisionPartDrive { velocity, omega })
}
