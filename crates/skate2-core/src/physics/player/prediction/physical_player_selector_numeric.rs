//! Numeric leaves for the retained selector, using existing fallible VMX services.
use super::selector_update::SelectorError;
use crate::{
    physical_player_footplant::RetainedTrajectory,
    physical_player_trajectory_owners::RetainedPredictionResult,
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector as V,
};
pub(super) const DT: f32 = f32::from_bits(0x3c88_8889);

/// `PredictionResults.GetCollisionPosition82d8a648`. Actual full-lane FMAs.
pub(super) fn collision_position<E, N>(
    r: &RetainedPredictionResult,
) -> Result<V, SelectorError<E, N>> {
    let t = r.time[0];
    if t.abs().partial_cmp(&100.0) != Some(std::cmp::Ordering::Less) {
        return Err(SelectorError::CollisionTime(t.to_bits()));
    }
    let dt2 = t * t;
    let linear = r
        .trajectory
        .velocity
        .zip(r.trajectory.position, |v, p| v.mul_add(t, p));
    Ok(r.trajectory
        .gravity
        .map(|g| g * 0.5)
        .zip(linear, |g, p| g.mul_add(dt2, p)))
}
/// `CalculateLandingVel82db9b68` uses collision FRAME, not fractional time.
#[expect(clippy::cast_precision_loss, reason = "native signed fcfid then frsp")]
pub(super) fn landing_velocity(r: &RetainedPredictionResult) -> V {
    let t = r.frame as f32 * DT;
    r.trajectory
        .gravity
        .zip(r.trajectory.velocity, |g, v| g.mul_add(t, v))
}
/// tTrajectory.AdjustTrajectory82db6e78. This is not the generic clamp helper:
/// squared gate, conditional length estimate, scalar division then final FMA.
#[expect(clippy::cast_precision_loss, reason = "native signed fcfid then frsp")]
pub(crate) fn adjust<E, N: ReckoningNumeric>(
    t: &mut RetainedTrajectory,
    frame: i32,
    delta: V,
    maximum: f32,
    numeric: &mut N,
) -> Result<(), SelectorError<E, N::Error>> {
    if maximum < 0.0 {
        return Err(SelectorError::NegativeMaximum(maximum.to_bits()));
    }
    if frame <= 0 {
        return Ok(());
    }
    let seconds = frame as f32 * DT;
    let seed = numeric
        .reciprocal_estimate(seconds)
        .map_err(SelectorError::Numeric)?;
    let inverse = refine_drive_reciprocal(seconds, seed);
    let velocity = delta.map(|v| inverse * v);
    let squared = numeric
        .dot_vector(velocity, velocity)
        .map_err(SelectorError::Numeric)?;
    let scale = if squared > maximum * maximum {
        let seed = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(SelectorError::Numeric)?;
        maximum / length_from_squared_and_estimate(squared, seed)
    } else {
        1.0
    };
    t.velocity = velocity.zip(t.velocity, |d, v| d.mul_add(scale, v));
    Ok(())
}

#[cfg(test)]
#[path = "physical_player_selector_numeric_tests.rs"]
mod tests;
