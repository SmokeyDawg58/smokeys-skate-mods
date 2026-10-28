//! ScoreModule DigestExternalInput82df6400 and per-update clearing82dfcd60.
use super::{
    super::scoring::Scoring2, score_module::ScoreModule, scoring_attributes::ScoreError,
    scoring_context_collect::ContextFrame, scoring_external::ScoringExternalInput,
};
use crate::attribute_runtime::AttributeRuntimeVault;
use std::sync::Arc;
impl ScoreModule {
    pub(super) fn digest_external(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        input: Arc<ScoringExternalInput>,
        frame: &ContextFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let mut holder = self
            .holder
            .published_value()
            .ok_or(ScoreError::Missing("score holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        let sequence = holder.sequence_mut()?;
        self.metrics
            .published_value()
            .ok_or(ScoreError::Missing("global metrics"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .digest_external(vault, input.clone(), frame, sequence, out)?;
        self.ground
            .published_value_mut()
            .ok_or(ScoreError::Missing("ground collector"))?
            .manuals_are_connectors = input.manuals_are_connectors;
        let state = self
            .reset_suffix
            .as_mut()
            .ok_or(ScoreError::Missing("score reset"))?;
        state.reset_sequence = input.reset_sequence && sequence.flags_word & 0x8000_0000 != 0;
        state.is_multiplier_active = input.multiplier_active;
        for i in 0..3 {
            let matcher = &mut sequence.pattern_matchers[i];
            if input.activate_patterns[i] {
                // ActivatePattern82ded8f0 copies all nine words, including the
                // unused pattern items, and clears the three partial matches.
                matcher.length = input.patterns[i].length;
                matcher.items = input.patterns[i].items.clone();
                matcher.is_active = true;
                matcher.matches = [false; 3];
            } else if input.deactivate_patterns[i] {
                matcher.is_active = false;
            }
        }
        Ok(())
    }
    pub(super) fn begin_scoring_frame(&mut self) -> Result<(), ScoreError> {
        let mut holder = self
            .holder
            .published_value()
            .ok_or(ScoreError::Missing("score holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        holder.tricks_begun.reset();
        holder.tricks_ended.reset();
        holder.tricks_converted_from.reset();
        holder.tricks_converted_to.reset();
        holder.tricks_cancelled.reset();
        holder.sequence_mut()?.active_scorables.reset();
        holder.trick_score = 0.0;
        holder.banked_sequence_score = 0.0;
        holder.initial_sequence_score = holder.sequence_score as i32;
        Ok(())
    }
    pub(super) fn finish_raw_animation(&mut self, out: &mut Scoring2) -> Result<(), ScoreError> {
        let state = self
            .reset_suffix
            .as_mut()
            .ok_or(ScoreError::Missing("score reset"))?;
        let previous = state.raw_scorable_id;
        state.raw_scorable_id = out.raw_scorable_id;
        if previous != state.raw_scorable_id {
            if previous != -1 {
                out.raw_scorable_ended = previous;
            }
            if state.raw_scorable_id != -1 {
                out.raw_scorable_started = state.raw_scorable_id;
            }
        }
        Ok(())
    }
}
