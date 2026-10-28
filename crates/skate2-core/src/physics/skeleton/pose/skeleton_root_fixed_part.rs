//! Retained fixed-Part roots, `UpdateRootTransformsFixedPart` 0x82b42110.
use super::{SkeletonRootMatrix, SkeletonRootTransforms, SkeletonRootVector};

/// Debug `FailAssert` lines 2847..2869 and the caller's 2819/2822 (0xB03/B06):
/// each XYZ lane is compared with itself, so a NaN fails and an infinity
/// passes. W lanes are never tested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FixedPartAssertion {
    Position,
    AnimationAngle,
    Reckoning,
    AnimationToWorld,
    RotatedPart,
    Translation,
    Orientation,
}

fn has_nan_xyz(vector: SkeletonRootVector) -> bool {
    vector.xyz.iter().any(|value| value.is_nan())
}

impl SkeletonRootTransforms {
    /// No heading refresh and no ortho-normalization. AnimToWorld is the
    /// retained angle under the Reckoning transform; its translation then puts
    /// the Part's animation position at `position`. The trajectory and root
    /// pose identity stores 0x82b4283c..8c8 before the inverse touch no root
    /// lane and belong to the caller.
    pub(crate) fn try_update_fixed_part(
        &mut self,
        reckoning: &SkeletonRootMatrix,
        position: SkeletonRootVector,
        part: SkeletonRootVector,
    ) -> Result<(), FixedPartAssertion> {
        use FixedPartAssertion as A;
        if has_nan_xyz(position) {
            return Err(A::Position);
        }
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
        // 0x82b42748..750: X multiply, Y FMA, Z FMA; no translation term.
        let axes = self.animation_to_world.columns();
        let first = axes[0].map(|v| v * part.xyz[0]);
        let second = axes[1].zip(first, |v, old| v.mul_add(part.xyz[1], old));
        let rotated = axes[2].zip(second, |v, old| v.mul_add(part.xyz[2], old));
        if has_nan_xyz(rotated) {
            return Err(A::RotatedPart);
        }
        let translation = position.zip(rotated, |a, b| a - b);
        if has_nan_xyz(translation) {
            return Err(A::Translation);
        }
        self.animation_to_world.set_column(3, translation);
        self.world_to_animation = self.animation_to_world.inverse();
        Ok(())
    }
}
