//! Same Processed fields written by `Update1_ProcessInput82e03580..36c4`.
use super::SkeletonProcessedContact;
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};
pub(crate) struct ProcessedGroundPosition {
    pub(crate) time_offboard: Option<f32>,
    pub(crate) effective_animation: Option<SkeletonRootMatrix>,
    pub(crate) ground_position: Option<SkeletonRootVector>,
    pub(crate) damped_ground_velocity: Option<SkeletonRootVector>,
}
impl ProcessedGroundPosition {
    pub(super) const fn new() -> Self {
        Self {
            time_offboard: None,
            effective_animation: None,
            ground_position: None,
            damped_ground_velocity: None,
        }
    }
    pub(super) fn reset(&mut self) {
        self.time_offboard = Some(0.0); //39584
        self.effective_animation = Some(SkeletonRootMatrix::known(
            crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM,
        )); //391a8..b8
        self.ground_position = Some(SkeletonRootVector::known([0.0; 4])); //39244
        self.damped_ground_velocity = Some(SkeletonRootVector::known([0.0; 4])); //39254
    }
}
impl SkeletonProcessedContact {
    /// Actual Processed+1e0, distinct from the all-states position at+1f0.
    pub(crate) fn ground_position_fields(&self) -> &ProcessedGroundPosition {
        &self.ground_position
    }
    pub(crate) fn ground_position_fields_mut(&mut self) -> &mut ProcessedGroundPosition {
        &mut self.ground_position
    }
    /// Actual Processed+80 `mEffectiveTransform` (physical board), distinct from
    /// the effective animation transform at +c0.
    pub(crate) fn effective_transform_field(&self) -> Option<&SkeletonRootMatrix> {
        self.effective.as_ref()
    }
    pub(crate) fn publish_ground_position_all_states(&mut self, value: SkeletonRootVector) {
        self.prior_output.publish_ground_position_all_states(value);
    }
}
