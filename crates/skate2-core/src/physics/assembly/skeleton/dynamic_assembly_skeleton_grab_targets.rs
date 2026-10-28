//! DMO hand target setters use the existing mapped bones and retained IK owner.
use super::SkeletonNormalAssembly;
use crate::{
    reckoning_numeric::ReckoningNumeric,
    skeleton_ik_line_frame::{SkeletonIkClampError, try_clamp_vector_within_range},
    skeleton_ik_state::external::transform_point,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[derive(Debug)]
pub(crate) enum GrabHandTargetError<N> {
    MissingAnimationBone { part: usize },
    Clamp(SkeletonIkClampError<N>),
    MissingIk,
}

impl SkeletonNormalAssembly {
    /// SetLeft82b41100 / Right82b411a8. Shoulder positions are raw animation
    /// bone5/9 at Skeleton+37b0/38b0, NOT mapped collision volumes or AnimRecord.
    pub(crate) fn try_set_world_hand_ik_target<N: ReckoningNumeric>(
        &mut self,
        left: bool,
        position: SkeletonRootVector,
        max_arm_length: f32,
        blend: f32,
        numeric: &mut N,
    ) -> Result<(), GrabHandTargetError<N::Error>> {
        let part = if left { 5 } else { 9 };
        let bone = self
            .animation_targets
            .animation_row(part)
            .ok_or(GrabHandTargetError::MissingAnimationBone { part })?;
        let shoulder = transform_point(
            self.root_transforms.animation_to_world(),
            SkeletonRootMatrix::known(*bone).columns()[3],
        );
        let mut clamped = position;
        try_clamp_vector_within_range(position, shoulder, max_arm_length, &mut clamped, numeric)
            .map_err(GrabHandTargetError::Clamp)?;
        self.current_animation
            .ik_runtime_mut()
            .ok_or(GrabHandTargetError::MissingIk)?
            .set_world_hand_target(left, clamped, blend);
        Ok(())
    }

    /// SkeletonController82dd4768 -> Skeleton82b4ef00.
    pub(crate) fn disable_hand_collisions(&mut self, frames: i32) {
        self.state.disable_hand_collisions(frames);
    }

    /// SkeletonController82dd4770 -> Skeleton82b4f3b8.
    pub(crate) fn disable_lower_leg_collisions(&mut self, frames: i32) {
        self.state.disable_lower_leg_collisions(frames);
    }
}
