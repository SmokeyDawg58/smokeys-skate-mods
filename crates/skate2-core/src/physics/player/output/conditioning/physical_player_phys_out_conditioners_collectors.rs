//! Non-cleared concrete collectors and their original reset bodies.
use super::{
    super::scoring::GrindInfo,
    collector_base::{CollectorBase, RotationTracker, TimedArcLength},
    scorable::Scorable,
};

pub(super) struct GrindCollector {
    pub(super) base: CollectorBase,
    pub(super) scorable: Scorable,
    pub(super) tracker: TimedArcLength,
    pub(super) info: GrindInfo,
}
impl GrindCollector {
    ///82df9490. The three GUID stores use sign-extended li r10,-1, proven
    /// at82df94d0 and std82df9544/9554/9558: all64 bits are ones.
    pub(super) fn new(base: CollectorBase) -> Self {
        let scorable = Scorable::new_noncleared();
        let tracker = TimedArcLength::new(&base, 9, 8);
        let mut result = Self {
            base,
            scorable,
            tracker,
            info: GrindInfo::new(),
        };
        result.reset();
        result
    }
    pub(super) fn reset(&mut self) {
        self.base.reset_analysis();
        self.tracker.reset();
        self.scorable.reset();
        self.info.reset();
    }
}

pub(super) struct GroundCollector {
    pub(super) base: CollectorBase,
    pub(super) total_trick_score: Option<f32>,
    pub(super) powerslide_state: i32,
    pub(super) powerslide_scorable: Scorable,
    pub(super) manual_scorable: Scorable,
    pub(super) grab_scorable: Scorable,
    pub(super) revert_scorable: Scorable,
    pub(super) num_frames_on_ground: u32,
    pub(super) manual_revert_count: u32,
    pub(super) most_recent_animation_revert: i32,
    pub(super) previous_animation_revert: i32,
    pub(super) reverting: bool,
    pub(super) manuals_are_connectors: bool,
    pub(super) powerslide_tracker: TimedArcLength,
    pub(super) manual_tracker: TimedArcLength,
    pub(super) grab_tracker: TimedArcLength,
}
impl GroundCollector {
    /// Constructor82df9a78 and Reset82defef8 leave `total_trick_score` unwritten.
    pub(super) fn new(base: CollectorBase) -> Self {
        let powerslide_scorable = Scorable::new_noncleared();
        let manual_scorable = Scorable::new_noncleared();
        let grab_scorable = Scorable::new_noncleared();
        let revert_scorable = Scorable::new_noncleared();
        let powerslide_tracker = TimedArcLength::new(&base, 0, 1);
        let manual_tracker = TimedArcLength::new(&base, 3, 2);
        let grab_tracker = TimedArcLength::new(&base, 7, 6);
        let mut result = Self {
            base,
            total_trick_score: None,
            powerslide_state: 0,
            powerslide_scorable,
            manual_scorable,
            grab_scorable,
            revert_scorable,
            num_frames_on_ground: 0,
            manual_revert_count: 0,
            most_recent_animation_revert: 0,
            previous_animation_revert: 0,
            reverting: false,
            manuals_are_connectors: false,
            powerslide_tracker,
            manual_tracker,
            grab_tracker,
        };
        result.reset();
        result
    }
    pub(super) fn reset(&mut self) {
        self.base.reset_analysis();
        self.powerslide_tracker.reset();
        self.manual_tracker.reset();
        self.grab_tracker.reset();
        self.powerslide_scorable.reset();
        self.manual_scorable.reset();
        self.grab_scorable.reset();
        self.revert_scorable.reset();
        self.powerslide_state = 0;
        self.reverting = false;
        self.num_frames_on_ground = 0;
        self.manual_revert_count = 0;
        self.most_recent_animation_revert = 0;
        self.previous_animation_revert = 0;
    }
}

pub(super) struct HandplantCollector {
    pub(super) base: CollectorBase,
    pub(super) scorable: Scorable,
    pub(super) max_height_above_ground: f32,
    pub(super) max_height_above_ground_points: f32,
    pub(super) time_duration_points: f32,
    pub(super) num_frames_in_collector: u32,
    pub(super) num_frames_in_handplant: u32,
}
impl HandplantCollector {
    /// Constructor82dfabf8 contains the complete reset inline.
    pub(super) fn new(base: CollectorBase) -> Self {
        let mut result = Self {
            base,
            scorable: Scorable::new_noncleared(),
            max_height_above_ground: 0.0,
            max_height_above_ground_points: 0.0,
            time_duration_points: 0.0,
            num_frames_in_collector: 0,
            num_frames_in_handplant: 0,
        };
        result.reset();
        result
    }
    pub(super) fn reset(&mut self) {
        self.base.reset_analysis();
        self.scorable.reset();
        self.max_height_above_ground = 0.0;
        self.max_height_above_ground_points = 0.0;
        self.num_frames_in_collector = 0;
        self.time_duration_points = 0.0;
        self.num_frames_in_handplant = 0;
    }
}

pub(super) struct OffboardCollector {
    pub(super) base: CollectorBase,
    pub(super) player_spin_tracker: RotationTracker,
    pub(super) board_spin_tracker: RotationTracker,
    pub(super) total_trick_score: Option<f32>,
    pub(super) num_frames_in_collector: u32,
    pub(super) hippy_jump_scorable: Scorable,
    pub(super) hippy_jump180_count: i32,
    pub(super) hippy_jump180_points: f32,
    pub(super) is_hippy_jumping: bool,
}
impl OffboardCollector {
    /// Constructor82dfafd0 explicitly initializes both rotation types to Yaw0.
    /// Matrices and total trick score remain unwritten until later lifecycle.
    pub(super) fn new(base: CollectorBase) -> Self {
        let mut result = Self {
            base,
            player_spin_tracker: RotationTracker::new_noncleared(0),
            board_spin_tracker: RotationTracker::new_noncleared(0),
            total_trick_score: None,
            num_frames_in_collector: 0,
            hippy_jump_scorable: Scorable::new_noncleared(),
            hippy_jump180_count: 0,
            hippy_jump180_points: 0.0,
            is_hippy_jumping: false,
        };
        result.reset();
        result
    }
    pub(super) fn reset(&mut self) {
        self.base.reset_analysis();
        self.num_frames_in_collector = 0;
        self.hippy_jump_scorable.reset();
        self.hippy_jump180_points = 0.0;
        self.is_hippy_jumping = false;
        self.hippy_jump180_count = 0;
    }
}
