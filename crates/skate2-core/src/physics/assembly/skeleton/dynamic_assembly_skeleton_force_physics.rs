//! Proposed `skeleton_normal` child: the SAME mode consumed by animation query.
use super::SkeletonNormalAssembly;
use crate::generated_motion_load::construct::control::PhysicsAnimationMode;
impl SkeletonNormalAssembly {
    /// ForcePhysics.Begin82ca307c stores Skeleton+3df0 mPhysAnimState.
    pub(crate) fn set_motion_physics_animation_mode(&mut self, mode: PhysicsAnimationMode) {
        self.physics_animation_state = Some(mode as i32);
    }
}
