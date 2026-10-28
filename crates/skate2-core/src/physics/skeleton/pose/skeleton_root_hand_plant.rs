//! Retained hand-plant roots, `UpdateRootTransformsHandPlant` 0x82b43880.
use super::{
    SkeletonRootMatrix, SkeletonRootTransforms, SkeletonRootVector, fixed_part::FixedPartAssertion,
};

fn has_nan_xyz(vector: SkeletonRootVector) -> bool {
    vector.xyz.iter().any(|value| value.is_nan())
}

impl SkeletonRootTransforms {
    /// No heading refresh and no ortho-normalization. Debug asserts 0xCAB/0xCAC
    /// test AnimAngle and the Reckoning transform, 0xCAE the composed AnimToWorld,
    /// 0xCB4 the translation and 0xCB6 AnimToWorld again (XYZ self-compare, so a
    /// NaN fails and an infinity passes). The current AnimRecord COM is rotated
    /// (X multiply, Y FMA, Z FMA) and subtracted from `position`. The trajectory
    /// and root-pose identity stores 0x82b44028..0b4 touch no root lane and
    /// belong to the caller; the rigid inverse follows them.
    pub(crate) fn try_update_hand_plant(
        &mut self,
        reckoning: &SkeletonRootMatrix,
        position: SkeletonRootVector,
        centre: [f32; 4],
    ) -> Result<(), FixedPartAssertion> {
        use FixedPartAssertion as A;
        if self.animation_angle.has_nan_xyz() {
            return Err(A::AnimationAngle);
        }
        if reckoning.has_nan_xyz() {
            return Err(A::Reckoning);
        }
        self.animation_to_world = reckoning.compose(&self.animation_angle);
        if self.animation_to_world.has_nan_xyz() {
            return Err(A::AnimationToWorld);
        }
        // 0x82b43e18..e24: X multiply, Y FMA, Z FMA; no translation term.
        let axes = self.animation_to_world.columns();
        let first = axes[0].map(|v| v * centre[0]);
        let second = axes[1].zip(first, |v, old| v.mul_add(centre[1], old));
        let rotated = axes[2].zip(second, |v, old| v.mul_add(centre[2], old));
        let translation = position.zip(rotated, |a, b| a - b);
        if has_nan_xyz(translation) {
            return Err(A::Translation);
        }
        self.animation_to_world.set_column(3, translation);
        if self.animation_to_world.has_nan_xyz() {
            return Err(A::AnimationToWorld);
        }
        self.world_to_animation = self.animation_to_world.inverse();
        Ok(())
    }
}
