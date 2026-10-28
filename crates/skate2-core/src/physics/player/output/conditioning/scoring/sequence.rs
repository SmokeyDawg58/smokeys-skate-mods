//! Sequence82df31c8/2e20/32c8/8c78 and non-substituting sequence pattern matching.
use super::{
    super::scoring::{Scorable, Scorables, Scoring2, TrickSettings},
    scoring_attributes::ScoreError,
    scoring_catalog::info,
    scoring_database::TrickDatabase,
    sequence::{PatternMatcher, Sequence},
};

impl PatternMatcher {
    // Sequence::AddScorable passes allowTrickSubstitution=false (82df8cac).
    fn add_sequence_trick(
        &mut self,
        db: &TrickDatabase,
        kind: i32,
        index: i32,
    ) -> Result<bool, ScoreError> {
        if !self.is_active {
            return Ok(false);
        }
        let count =
            usize::try_from(self.length).map_err(|_| ScoreError::Invalid("pattern length"))?;
        if count > 4 {
            return Err(ScoreError::Invalid("pattern exceeds four"));
        }
        if kind == 4 {
            let mut contains_grab = false;
            for item in &self.items[..count] {
                let item_kind = match item.match_type {
                    2 => item.value,
                    1 => {
                        let mut settings = TrickSettings::new();
                        db.lookup_index(item.value, &mut settings)?;
                        if settings.is_valid {
                            settings.trick_type
                        } else {
                            0
                        }
                    }
                    _ => 0,
                };
                contains_grab |= item_kind == 4;
            }
            if !contains_grab {
                return Ok(false);
            }
        }
        for i in (0..count).rev() {
            if i != 0 && !self.matches[i - 1] {
                continue;
            }
            let item = &self.items[i];
            let matched = match item.match_type {
                1 => item.value == index,
                2 => item.value == kind,
                _ => return Err(ScoreError::Invalid("pattern match type")),
            };
            if matched && i == count - 1 {
                return Ok(true);
            }
            if i < count - 1 {
                self.matches[i] = matched;
            }
        }
        Ok(false)
    }
}

impl Sequence {
    pub(super) fn most_recent(&self) -> Option<&Scorable> {
        if self.active_scorables.num_entries > 0 {
            self.active_scorables
                .elements
                .get(self.active_scorables.num_entries as usize - 1)
        } else if self.trick_count > 0 {
            Some(&self.scorables[((self.trick_count - 1) & 3) as usize])
        } else {
            None
        }
    }
    pub(super) fn add_scorable(
        &mut self,
        db: &TrickDatabase,
        scorable: &Scorable,
    ) -> Result<(), ScoreError> {
        let id = scorable.valid_id().unwrap_or(-1);
        let mut settings = TrickSettings::new();
        db.lookup_id(id, &mut settings)?;
        let index = if settings.is_valid {
            settings.trick_index as i32
        } else {
            -1
        };
        let kind = info(id).map_or(0, |i| i.trick_type);
        for (matcher, result) in self
            .pattern_matchers
            .iter_mut()
            .zip(&mut self.pattern_match_results)
        {
            *result |= matcher.add_sequence_trick(db, kind, index)?;
        }
        let previous = self.trick_count;
        self.trick_count = previous
            .checked_add(1)
            .ok_or(ScoreError::Invalid("sequence trick count overflow"))?;
        if scorable.was_a_tap_trick {
            self.tapped_trick_count = self.tapped_trick_count.wrapping_add(1);
        }
        self.scorables[(previous & 3) as usize] = scorable.clone();
        Ok(())
    }
    pub(super) fn update(
        &mut self,
        worthy: bool,
        restart: bool,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let was_in = self.flags_word & 0x8000_0000 != 0;
        self.flags_word &= 0x9fff_ffff;
        if was_in {
            if !worthy || restart {
                self.end(out)?;
            }
        } else if !worthy {
            self.reset();
        }
        if worthy && (!was_in || restart) {
            let metrics = self
                .global_metrics
                .as_ref()
                .ok_or(ScoreError::Missing("sequence metrics"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            self.flags_word |= 0x2000_0000;
            self.start_visual_game_tick = metrics.visual_tick;
            self.start_sim_game_tick = metrics.tick;
            self.replay_start_frame_number = metrics.last_frame_number_written;
        }
        self.flags_word = (self.flags_word & 0x7fff_ffff) | ((worthy as u32) << 31);
        out.live_capped_sequence_trick_count = self.trick_count;
        Ok(())
    }
    fn end(&mut self, out: &mut Scoring2) -> Result<(), ScoreError> {
        let count = self.trick_count.min(4);
        out.finished_sequence_pattern_matches.did_match = self.pattern_match_results;
        out.finished_sequence_trick_count = self.trick_count;
        out.finished_sequence_tapped_trick_count = self.tapped_trick_count;
        out.finished_sequence_stored_trick_count = count;
        out.finished_sequence_start_visual_game_tick = self.start_visual_game_tick;
        out.finished_sequence_start_sim_game_tick = self.start_sim_game_tick;
        out.finished_sequence_replay_start_frame_number = self.replay_start_frame_number;
        let tick = self
            .global_metrics
            .as_ref()
            .ok_or(ScoreError::Missing("sequence metrics"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .tick;
        let ticks = tick
            .checked_sub(self.start_sim_game_tick)
            .ok_or(ScoreError::Invalid("sequence end before start"))?;
        out.finished_sequence_duration_in_seconds = ticks as f32 * (1.0_f32 / 60.0);
        let mut list = Scorables::new();
        for scorable in &self.scorables[..count as usize] {
            list.add(scorable)?;
        }
        out.finished_sequence_scorables = list;
        out.finished_sequence_dynamic_object_set.is_set = self.dynamic_objects_used;
        self.reset();
        self.flags_word |= 0x4000_0000;
        Ok(())
    }
    pub(super) fn fill_output(&self, out: &mut Scoring2) -> Result<(), ScoreError> {
        out.current_sequence_pattern_matches.did_match = self.pattern_match_results;
        out.sequence_completed = self.flags_word & 0x4000_0000 != 0;
        out.sequence_begun = self.flags_word & 0x2000_0000 != 0;
        out.in_sequence = self.flags_word & 0x8000_0000 != 0;
        out.active_tricks = self.active_scorables.clone();
        let count = self.trick_count.min(4);
        out.current_sequence_trick_count = self.trick_count;
        out.current_sequence_tapped_trick_count = self.tapped_trick_count;
        out.current_sequence_stored_trick_count = count;
        let mut list = Scorables::new();
        // Native82df34d0 starts at count&3, including when fewer than four exist.
        for i in 0..count {
            list.add(&self.scorables[((i + (self.trick_count & 3)) & 3) as usize])?;
        }
        out.current_sequence_scorables = list;
        out.current_sequence_dynamic_object_set.is_set = self.dynamic_objects_used;
        Ok(())
    }
}
