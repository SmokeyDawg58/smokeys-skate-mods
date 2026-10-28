//! GlobalMetrics DigestExternalInput82df4370 / Update82deaab8.
use super::{
    super::scoring::Scoring2,
    scoring_attributes::ScoreError,
    scoring_context_collect::{self, ContextFrame},
    scoring_external::ScoringExternalInput,
    sequence::{GlobalMetrics, Sequence},
};
use crate::{
    attribute_runtime::AttributeRuntimeVault, physics::PhysicsCategory,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;
impl GlobalMetrics {
    pub(super) fn digest_external(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        input: Arc<ScoringExternalInput>,
        frame: &ContextFrame<'_>,
        sequence: &mut Sequence,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        input.validate()?;
        self.external_input = Some(input.clone());
        self.tick = input.tick;
        self.visual_tick = input.visual_tick;
        self.last_frame_number_written = input.last_frame_number_written;
        out.ho_m_is_effect_enabled = input.hall_of_meat_effect;
        out.current_context_data.reset();
        scoring_context_collect::collect(vault, &input, frame, sequence, out)?;
        let index = self.over_dmo_context_next_index as usize;
        let dst = self
            .over_dmo_context
            .get_mut(index)
            .ok_or(ScoreError::Invalid("over DMO ring index"))?;
        *dst = out.current_context_data.context_sensitive_data[3].uid_data[0].clone();
        self.over_dmo_context_next_index += 1;
        if self.over_dmo_context_next_index >= 8 {
            self.over_dmo_context_next_index = 0;
            self.over_dmo_context_buffer_full = true;
        }
        Ok(())
    }
    pub(super) fn update_takeoff_normal(
        &mut self,
        category: PhysicsCategory,
        ground_normal: SkeletonRootVector,
        grind_normal: SkeletonRootVector,
    ) {
        self.takeoff_normal = match category {
            PhysicsCategory::Ground => ground_normal,
            PhysicsCategory::Grind => grind_normal,
            _ => SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
        };
    }
}
