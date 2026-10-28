//! Selected `FillPhysOut` 0x82b2f714..0x82b2f7f8 only. No preceding-stage completion.
use super::{PhysOutAccessError, PhysOutMotionScalar, PhysOutMotionVector, PhysicalPlayerPhysOut};
use crate::{
    physical_player_phys_out::construct::board_fill::board_speed_from_squared_and_estimate,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
#[derive(Debug, PartialEq)]
pub(crate) enum PhysOutMotionError<S, N> {
    Access(PhysOutAccessError),
    Source(S),
    Numeric(N),
}
impl PhysicalPlayerPhysOut {
    /// The supplied linear value was sampled at 0x82b2f550. Angular sampling is
    /// late, between its latch sample and assertion; deck dot follows its check.
    pub(crate) fn try_publish_selected_board_motion<S, N: SkeletonCollisionNumeric>(
        &mut self,
        linear: SkeletonRootVector,
        angular: impl FnOnce() -> Result<SkeletonRootVector, S>,
        numeric: &mut N,
    ) -> Result<(), PhysOutMotionError<S, N::Error>> {
        use PhysOutMotionError as E;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let angular = angular().map_err(E::Source)?;
        self.publish_motion_vector(&guard, PhysOutMotionVector::AngularVelocity, angular)
            .map_err(E::Access)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        let squared = numeric.dot3(linear.xyz, linear.xyz).map_err(E::Numeric)?;
        self.publish_motion_vector(&guard, PhysOutMotionVector::DeckVelocity, linear)
            .map_err(E::Access)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let estimate = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let speed = board_speed_from_squared_and_estimate(squared, estimate);
        self.publish_motion_scalar(&guard, PhysOutMotionScalar::DeckSpeed, speed)
            .map_err(E::Access)
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_motion_tests.rs"]
mod tests;
