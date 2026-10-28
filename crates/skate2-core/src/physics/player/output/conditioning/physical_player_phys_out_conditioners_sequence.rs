//! Scoring global metrics82dee818 and Sim-allocated Sequence82df2d68.
//! Owner occurrences are copied from the actual output; metrics are shared by
//! allocation, preserving the holder/collector/sequence pointer relationship.
use super::super::{
    SkeletonRootVector, UP,
    scoring::{ContextUids, Scorable, Scorables},
};
use std::sync::{Arc, Mutex};

pub(super) struct GlobalMetrics {
    pub(super) output: Arc<()>,
    // The native constructor explicitly stores null. No external-input owner
    // is admitted by this constructor path.
    pub(super) external_input: Option<Arc<super::scoring_external::ScoringExternalInput>>,
    pub(super) takeoff_normal: SkeletonRootVector,
    pub(super) over_dmo_context: [ContextUids; 8],
    pub(super) tick: u32,
    pub(super) visual_tick: u32,
    pub(super) last_frame_number_written: u32,
    pub(super) over_dmo_context_next_index: u32,
    pub(super) over_dmo_context_buffer_full: bool,
}
impl GlobalMetrics {
    /// Constructor82dee818; every named non-padding field is explicitly written.
    pub(super) fn new(output: Arc<()>) -> Self {
        Self {
            output,
            external_input: None,
            takeoff_normal: UP,
            over_dmo_context: std::array::from_fn(|_| ContextUids::new()),
            tick: 0,
            visual_tick: 0,
            last_frame_number_written: 0,
            over_dmo_context_next_index: 0,
            over_dmo_context_buffer_full: false,
        }
    }
    /// Inline metrics reset in `ScoreModule::Reset82df26f0` preserves UID payloads,
    /// ticks and external-input binding; it clears only the context counts.
    pub(super) fn reset_contexts(&mut self) {
        self.takeoff_normal = UP;
        for context in &mut self.over_dmo_context {
            context.reset();
        }
        self.over_dmo_context_next_index = 0;
        self.over_dmo_context_buffer_full = false;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PatternItem {
    pub(super) match_type: i32,
    pub(super) value: i32,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PatternMatcher {
    pub(super) length: i32,
    pub(super) items: [PatternItem; 4],
    pub(super) matches: [bool; 3],
    pub(super) is_active: bool,
}
impl PatternMatcher {
    /// Group constructor82ded890 calls item constructor82de9660. Invalid is0
    /// (raw li/stw), not the -1 sentinel used by unrelated scoring enums.
    fn new() -> Self {
        Self {
            length: 0,
            items: std::array::from_fn(|_| PatternItem {
                match_type: 0,
                value: 0,
            }),
            matches: [false; 3],
            is_active: false,
        }
    }
}

pub(super) struct Sequence {
    pub(super) pattern_matchers: [PatternMatcher; 3],
    pub(super) pattern_match_results: [bool; 3],
    pub(super) output: Arc<()>,
    pub(super) trick_count: u32,
    pub(super) tapped_trick_count: u32,
    pub(super) start_visual_game_tick: u32,
    pub(super) start_sim_game_tick: u32,
    pub(super) replay_start_frame_number: u32,
    pub(super) scorables: [Scorable; 4],
    pub(super) active_scorables: Scorables,
    pub(super) global_metrics: Option<Arc<Mutex<GlobalMetrics>>>,
    pub(super) dynamic_objects_used: [bool; 17],
    pub(super) flags_word: u32,
}
impl Sequence {
    /// Caller must use the actual Sim allocation: zero initialization of the
    /// constructor-unwritten embedded settings/bitfield storage depends on it.
    pub(super) fn new_sim(
        output: Arc<()>,
        global_metrics: Option<Arc<Mutex<GlobalMetrics>>>,
    ) -> Self {
        let mut result = Self {
            pattern_matchers: std::array::from_fn(|_| PatternMatcher::new()),
            pattern_match_results: [false; 3],
            output,
            trick_count: 0,
            tapped_trick_count: 0,
            start_visual_game_tick: 0,
            start_sim_game_tick: 0,
            replay_start_frame_number: 0,
            scorables: std::array::from_fn(|_| Scorable::new()),
            active_scorables: Scorables::new(),
            global_metrics,
            dynamic_objects_used: [false; 17],
            flags_word: 0,
        };
        result.reset();
        result
    }
    /// Reset82dedbe0 preserves patterns, active flags and all stored Scorables.
    pub(super) fn reset(&mut self) {
        for matcher in &mut self.pattern_matchers {
            matcher.matches = [false; 3];
        }
        self.pattern_match_results = [false; 3];
        self.trick_count = 0;
        self.tapped_trick_count = 0;
        self.active_scorables.reset();
        self.start_visual_game_tick = 0;
        self.replay_start_frame_number = 0;
        self.start_sim_game_tick = 0;
        self.flags_word &= 0x1fff_ffff;
        self.dynamic_objects_used = [false; 17];
    }
}
