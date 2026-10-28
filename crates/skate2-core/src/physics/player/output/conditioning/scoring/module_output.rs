//! ScoreModule FillPhysOut82df64f8, after current collector FillPhysOut.
use super::{
    super::scoring::Scoring2, score_module::ScoreModule, scoring_attributes::ScoreError,
    scoring_meters::MeterKind,
};
use crate::physics::FilteredPhysicsState as Filtered;
impl ScoreModule {
    pub(super) fn fill_module_output(
        &mut self,
        filtered: Filtered,
        multiplier_override: Option<i32>,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let holder = self
            .holder
            .published_value()
            .ok_or(ScoreError::Missing("score holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        holder.fill_output(out)?;
        let line = self
            .line_timer
            .published_value()
            .ok_or(ScoreError::Missing("line timer"))?;
        let momentum = self
            .momentum
            .published_value()
            .ok_or(ScoreError::Missing("momentum"))?;
        let state = self
            .reset_suffix
            .as_mut()
            .ok_or(ScoreError::Missing("score reset"))?;
        out.line_timer_seconds = line.seconds(MeterKind::Line)?;
        out.line_timer_points = line.points as i32;
        let score = ((holder.trick_score + holder.air_sequence_score_pending)
            + holder.air_sequence_score)
            + holder.sequence_score;
        let multiplier = if state.is_multiplier_active {
            momentum.get_multiplier(multiplier_override)?
        } else {
            1.0
        };
        out.pre_multiplied_hud_sequence_score = (multiplier * score) as i32;
        out.momentum_meter_seconds = momentum.base.seconds(MeterKind::Momentum)?;
        out.momentum_meter_points = momentum.base.points as i32;
        out.momentum_meter_finished = momentum.base.just_reached_zero;
        out.momentum_meter_multiplier = momentum.get_multiplier(multiplier_override)?;
        out.current_gesture = state.current_gesture;
        out.just_performed_gesture = state.just_performed_gesture;
        out.just_wiped_out = state.just_entered_wipeout;
        let (count, tapped) = if out.sequence_completed {
            (
                out.finished_sequence_trick_count,
                out.finished_sequence_tapped_trick_count,
            )
        } else {
            (
                out.current_sequence_trick_count,
                out.current_sequence_tapped_trick_count,
            )
        };
        super::scoring_thresholds::update(
            &self.general,
            &mut out.big_trick_achieved_data,
            10,
            count.wrapping_sub(tapped) as f32 + 0.1,
        )?;
        let offboard =
            matches!(filtered, Filtered::OffBoard | Filtered::OffBoardAir) && !out.is_hippy_jumping;
        // Original excludes Grind here; do not replace with !offboard.
        let onboard = matches!(filtered, Filtered::Ground | Filtered::Air) || out.is_hippy_jumping;
        out.is_offboard_and_not_hippy_jumping = offboard;
        out.was_offboard_and_not_hippy_jumping = state.was_offboard_and_not_hippy_jumping;
        out.is_onboard_or_hippy_jumping = onboard;
        out.was_onboard_or_hippy_jumping = state.was_onboard_or_hippy_jumping;
        state.was_onboard_or_hippy_jumping = onboard;
        state.was_hippy_jumping = out.is_hippy_jumping;
        state.was_offboard_and_not_hippy_jumping = offboard;
        if out.sequence_completed {
            out.sequence_completed_successfully = !(filtered == Filtered::Wipeout
                || out.wipeout_interrupted_trick
                || out.just_wiped_out
                || out.is_running_out);
        }
        Ok(())
    }
}
