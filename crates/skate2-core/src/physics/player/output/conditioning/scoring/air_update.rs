//! AirCollector::Update2_TrickAnalysis82dfbcb0, preserving native metric order.
use super::{
    super::scoring::Scoring2,
    air_collector::AirCollector,
    scoring_air_tricks::AirTrickFrame,
    scoring_attributes::{ScoreError, scalar},
    scoring_database::TrickDatabase,
};
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};
impl AirCollector {
    pub(super) fn update_analysis(
        &mut self,
        db: &TrickDatabase,
        trick: &AirTrickFrame<'_>,
        deck: SkeletonRootVector,
        player_matrix: SkeletonRootMatrix,
        board_matrix: SkeletonRootMatrix,
        up: SkeletonRootVector,
        flipping: bool,
        front_flipping: bool,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        self.total_trick_points = 0.0;
        self.accumulated_context.add(&out.current_context_data)?;
        let trajectory = self.update_trajectory_metrics(deck)?;
        let spin = self.update_spin_metric(player_matrix, board_matrix, up)? + trajectory;
        let flip = self.update_flip_metric(flipping, front_flipping, out)? + spin;
        let gap = self.update_gap_detection(trick.ground)?;
        {
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("air holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            holder.trick_score = gap + holder.trick_score;
        }
        self.total_trick_points = (gap + flip) + self.total_trick_points;
        self.total_trick_points = self.update_tricks(db, trick, out)? + self.total_trick_points;
        if !self.tweak_occurred_in_air {
            let threshold = scalar(&self.base.general, 0x0772_6116_a20d_5bf0)?;
            if self.base.animation_trick.tweak_x.abs() > threshold
                || self.base.animation_trick.tweak_y.abs() > threshold
            {
                self.tweak_occurred_in_air = true;
            }
        }
        self.air_frames = self.air_frames.wrapping_add(1);
        Ok(())
    }
}
