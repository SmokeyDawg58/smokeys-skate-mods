//! Fallible numeric leaves shared by the existing trajectory walker.
use super::{BallisticTrajectory, PHYSICS_STEP_SECONDS};
use crate::{
    collision_query_batches::trajectory::{SkeletonTrajectoryInput, linear_collision_time},
    reckoning_numeric::{ReckoningNumeric, host_dot3, length_from_squared_and_estimate},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::convert::Infallible;

/// Compatibility only for the pre-existing public XYZ API. Actual world jobs
/// receive their caller's fallible `ReckoningNumeric`, including conversions.
pub(super) struct LegacyTrajectoryNumeric;
impl ReckoningNumeric for LegacyTrajectoryNumeric {
    type Error = Infallible;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(host_dot3(a, b))
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(1.0 / value)
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "old public API compatibility; not used by live trajectory jobs"
    )]
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        Ok(value as i32)
    }
}

pub(super) fn position(trajectory: &BallisticTrajectory, time: f32) -> [f32; 3] {
    let time_squared = time * time;
    std::array::from_fn(|axis| {
        (trajectory.gravity[axis] * 0.5).mul_add(
            time_squared,
            trajectory.initial_velocity[axis].mul_add(time, trajectory.initial_position[axis]),
        )
    })
}

pub(super) fn dot<N: ReckoningNumeric>(
    a: [f32; 3],
    b: [f32; 3],
    numeric: &mut N,
) -> Result<f32, N::Error> {
    numeric.dot3([a[0], a[1], a[2], 0.0], [b[0], b[1], b[2], 0.0])
}

pub(super) fn reciprocal<N: ReckoningNumeric>(
    value: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let mut inverse = numeric.reciprocal_estimate(value)?;
    for _ in 0..2 {
        let residual = -inverse.mul_add(value, -1.0);
        inverse = inverse.mul_add(residual, inverse);
    }
    Ok(inverse)
}

fn square_root<N: ReckoningNumeric>(value: f32, numeric: &mut N) -> Result<f32, N::Error> {
    let estimate = numeric.reciprocal_sqrt_estimate(value)?;
    Ok(length_from_squared_and_estimate(value, estimate))
}

/// `Get_dTWithErrorR`827bf868; caller selects this only for negative gravity Y.
#[expect(clippy::float_cmp, reason = "native exact gravity-axis assertions")]
pub(super) fn time_step<N: ReckoningNumeric>(
    trajectory: &BallisticTrajectory,
    error_radius: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    debug_assert_eq!(trajectory.gravity[0], 0.0);
    debug_assert_eq!(trajectory.gravity[2], 0.0);
    debug_assert!(trajectory.gravity[1] <= 0.0);
    let inverse = reciprocal(trajectory.gravity[1], numeric)?;
    let numerator = -8.0 * error_radius;
    square_root(inverse * numerator, numeric)
}

/// `CalculateTimeOfCollision`827bce08. Shared linear branch is already recovered
/// in `collision_query_trajectory`; its typed scratch W is never published.
pub(super) fn collision_time<N: ReckoningNumeric>(
    trajectory: &BallisticTrajectory,
    hit: [f32; 3],
    previous_time: f32,
    numeric: &mut N,
) -> Result<(f32, i32), N::Error> {
    let half_gravity = reciprocal(2.0, numeric)? * trajectory.gravity[1];
    if half_gravity.abs() <= f32::from_bits(0x38d1_b717) {
        return linear_collision_time(
            SkeletonTrajectoryInput {
                position: SkeletonRootVector {
                    xyz: trajectory.initial_position,
                    w: None,
                },
                velocity: SkeletonRootVector {
                    xyz: trajectory.initial_velocity,
                    w: None,
                },
                radius: 0.0, // This leaf consumes position and velocity only.
            },
            hit,
            numeric,
        );
    }
    let mut time = previous_time;
    let mut previous_distance_squared = f32::MAX;
    let mut passed_nearest_point = false;
    if trajectory.duration >= time {
        loop {
            let sampled = position(trajectory, time);
            let delta = std::array::from_fn(|axis| hit[axis] - sampled[axis]);
            let squared = dot(delta, delta, numeric)?;
            if previous_distance_squared >= squared {
                previous_distance_squared = squared;
            } else if squared > previous_distance_squared {
                time -= PHYSICS_STEP_SECONDS;
                passed_nearest_point = true;
                break;
            }
            time += PHYSICS_STEP_SECONDS;
            if !matches!(
                trajectory.duration.partial_cmp(&time),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ) {
                break;
            }
        }
    }
    let inverse_dt = reciprocal(PHYSICS_STEP_SECONDS, numeric)?;
    let collision_time = if passed_nearest_point {
        time
    } else {
        trajectory.duration
    };
    let frame = numeric.truncate_i32((inverse_dt * collision_time).ceil())?;
    Ok((collision_time, frame))
}
