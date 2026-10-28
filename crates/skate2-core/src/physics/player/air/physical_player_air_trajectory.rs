//! Air COM trajectory stores at 0x82d9f784..0x82d9f7a0.
use crate::{
    physical_player_footplant::RetainedTrajectory, skeleton_root_transforms::SkeletonRootVector,
};

/// Fixed `kUpdateTime_65` (0x82341f44), independent of Processed.dt.
pub(crate) fn advance(trajectory: &mut RetainedTrajectory) -> SkeletonRootVector {
    let dt = f32::from_bits(0x3c88_8889);
    let dt_squared = dt * dt;
    let linear = trajectory
        .velocity
        .zip(trajectory.position, |v, p| v.mul_add(dt, p));
    let velocity = trajectory
        .gravity
        .zip(trajectory.velocity, |g, v| g.mul_add(dt, v));
    let half_gravity = trajectory.gravity.map(|g| g * (1.0 * 0.5));
    let position = half_gravity.zip(linear, |g, p| g.mul_add(dt_squared, p));
    trajectory.position = position;
    trajectory.velocity = velocity;
    position
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_full_vector_and_unmodified_time() {
        let mut t = RetainedTrajectory::constructed();
        t.velocity = SkeletonRootVector::known([60.0, -60.0, 0.0, 120.0]);
        t.gravity = SkeletonRootVector::known([0.0, 3600.0, 0.0, -7200.0]);
        let old_time = t.time;
        let p = advance(&mut t);
        assert_eq!(p.xyz[0].to_bits(), 1.0_f32.to_bits());
        assert!((p.xyz[1] + 0.5).abs() < 0.000_001);
        assert!((p.w.unwrap() - 1.0).abs() < 0.000_001);
        assert_eq!(t.time.map(f32::to_bits), old_time.map(f32::to_bits));
        assert_eq!(t.gravity.xyz[1].to_bits(), 3600.0_f32.to_bits());
    }
    #[test]
    fn unknown_lane_propagates_and_no_nonfinite_repair() {
        let mut t = RetainedTrajectory::constructed();
        t.position.w = None;
        t.gravity.xyz[0] = f32::INFINITY;
        assert_eq!(advance(&mut t).w, None);
        assert_eq!(t.position.xyz[0].to_bits(), f32::INFINITY.to_bits());
    }
    #[test]
    fn linear_step_is_fused_before_gravity_contribution() {
        let mut t = RetainedTrajectory::constructed();
        t.position.xyz[0] = -1.0;
        t.velocity.xyz[0] = 60.0;
        assert_eq!(advance(&mut t).xyz[0].to_bits(), 0x3360_0000);
    }
}
