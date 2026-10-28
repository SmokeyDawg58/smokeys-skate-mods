//! What a `cChallengeObjective` records from the skater's airs, gestures, tricks, grinds and
//! round results for the objective natives: `mAirData` (`AirObjectiveData`), `mGestureData`,
//! `mBigTrickAchievements`, `mContextualDataSet`, `mRoundScore`, `mRoundRank` and
//! `mOverallRank`.

use super::mission::{ContextUidLists, FinishedAir, MissionFrame};

/// `eContextualAchievementType`.
pub mod achievement {
    pub const AIR_ONTO: i32 = 0;
    pub const AIR_OFF: i32 = 1;
    pub const AIR_OVER: i32 = 2;
    pub const GRIND: i32 = 3;
}

/// `EContextSensitiveDataGroups::eContextSensitiveDataGroup_DMO`.
const GROUP_DMO: i32 = 3;
/// `mGestureData.mUniqueGestures` (`fixed_vector<eAnimGestureNames, 8>`, no overflow).
const UNIQUE_GESTURES_CAPACITY: usize = 8;
/// `mBigTrickAchievements` and `mContextualDataSet` (`xfixed_vector<_, 16>`, filled only while not
/// full).
const ACHIEVEMENTS_CAPACITY: usize = 16;

/// `ContextualData`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextualData {
    /// `eContextualAchievementType`.
    pub achievement_type: i32,
    /// `EContextSensitiveDataGroups`.
    pub group: i32,
    pub unique_id: u32,
    /// `EEntityType`.
    pub entity_type: i32,
}

/// An objective's records; the constructor's `ResetObjectiveData(true, true)` leaves them empty.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectiveData {
    /// `mAirData.mLastAirTime` and `mTotalAirTime`.
    pub last_air_time: f32,
    pub total_air_time: f32,
    /// `mAirData.mLastAirHeight` and `mTotalAirHeight`.
    pub last_air_height: f32,
    pub total_air_height: f32,
    /// `mAirData.mDegreesOfSpin`, `mNumBodyFlips` and `mNumSkatersAiredOver`.
    pub degrees_of_spin: f32,
    pub num_body_flips: i32,
    pub num_skaters_aired_over: i32,
    /// `mGestureData.mUniqueGestures`.
    pub unique_gestures: Vec<i32>,
    /// `mBigTrickAchievements`: (`mType`, `mThreshold`).
    pub big_trick_achievements: Vec<(i32, i32)>,
    /// `mContextualDataSet`.
    pub contextual: Vec<ContextualData>,
    /// `mRoundScore` and `mRoundRank` of the five rounds, and `mOverallRank`.
    pub round_score: [i32; 5],
    pub round_rank: [i32; 5],
    pub overall_rank: i32,
}

impl ContextUidLists {
    /// `ContextSensitiveDataGroups::Remove` (`0x82de8cb8`) on the first lists:
    /// `ContextSensitiveUIDData::Remove` (`0x82de8a00`) takes each of the other's UIDs out, the
    /// last entry moving into its place.
    fn remove(&mut self, other: &Self) {
        for group in 0..5 {
            for &(unique_id, _) in &other.uids[group][..(other.counts[group] as usize).min(8)] {
                let count = (self.counts[group] as usize).min(8);
                if let Some(index) = self.uids[group][..count].iter().position(|&(id, _)| id == unique_id) {
                    self.counts[group] -= 1;
                    let last = self.counts[group] as usize;
                    if index < last {
                        self.uids[group][index] = self.uids[group][last];
                    }
                }
            }
        }
    }
}

impl ObjectiveData {
    /// `ResetObjectiveData`'s (`0x82d4cf18`) share: the last air, gestures, big-trick achievements and
    /// contextual data, and with the cumulative data the round results and the air totals.
    pub fn reset(&mut self, reset_cumulative: bool) {
        self.last_air_time = 0.0;
        self.last_air_height = 0.0;
        self.num_body_flips = 0;
        self.degrees_of_spin = 0.0;
        self.num_skaters_aired_over = 0;
        self.unique_gestures.clear();
        self.big_trick_achievements.clear();
        self.contextual.clear();
        if reset_cumulative {
            self.round_score = [0; 5];
            self.round_rank = [0; 5];
            self.total_air_time = 0.0;
            self.overall_rank = 0;
            self.total_air_height = 0.0;
        }
    }

    /// `UpdateBigTrickAchievement` (`0x82d4d6b8`): a reached threshold (`mWasAchieved[1]`, else
    /// `[0]`) records its type with threshold 1, whichever level it reached, while there is room.
    fn update_big_trick_achievement(&mut self, achieved: [bool; 2], threshold_type: i32) {
        if (achieved[1] || achieved[0]) && self.big_trick_achievements.len() < ACHIEVEMENTS_CAPACITY {
            self.big_trick_achievements.push((threshold_type, 1));
        }
    }

    /// `AddContextualData` (`0x82d4d798`): a new entry, while there is room.
    fn add_contextual_data(&mut self, data: ContextualData) {
        if self.contextual.len() < ACHIEVEMENTS_CAPACITY && !self.contextual.contains(&data) {
            self.contextual.push(data);
        }
    }

    /// `UpdateContextualData` (`0x82d4e1a0`): every UID of each group's first list.
    fn update_contextual_data(&mut self, lists: &ContextUidLists, achievement_type: i32) {
        for (group, (uids, &count)) in (0..).zip(lists.uids.iter().zip(&lists.counts)) {
            for &(unique_id, entity_type) in &uids[..(count as usize).min(8)] {
                self.add_contextual_data(ContextualData { achievement_type, group, unique_id, entity_type });
            }
        }
    }

    /// `AirCompletedMsg` (`0x82d51368`) of an active objective after a clean landing: the air's time
    /// (`GetCompletedAirTime`: frames over 60), height to peak, spin degrees and flips (both
    /// unsigned), the air thresholds 5 to 14, and the contexts taken off from (initial but not end),
    /// landed onto (end but not initial) and aired over (accumulated but neither), the last giving
    /// the number of skaters aired over.
    #[expect(clippy::cast_precision_loss, reason = "the native int to float conversions")]
    pub fn air_completed(&mut self, air: &FinishedAir) {
        let time = air.in_air_frames as f32 * 0.016_666_668;
        let total = self.total_air_time + time;
        self.last_air_time = time;
        self.total_air_time = total;
        let height = air.height_to_peak;
        let total = height + self.total_air_height;
        self.last_air_height = height;
        self.total_air_height = total;
        self.degrees_of_spin = air.player_spin180_count.wrapping_mul(180).wrapping_abs() as f32;
        self.num_body_flips = air.player_num_flips.wrapping_abs();
        for (threshold_type, &achieved) in (5..).zip(&air.big_trick_thresholds) {
            self.update_big_trick_achievement(achieved, threshold_type);
        }
        let mut off = air.initial_context;
        off.remove(&air.end_context);
        let mut onto = air.end_context;
        onto.remove(&air.initial_context);
        let mut over = air.accumulated_context;
        over.remove(&air.initial_context);
        over.remove(&air.end_context);
        self.update_contextual_data(&off, achievement::AIR_OFF);
        self.update_contextual_data(&onto, achievement::AIR_ONTO);
        self.update_contextual_data(&over, achievement::AIR_OVER);
        self.num_skaters_aired_over = over.counts[2].cast_signed();
    }

    /// `GesturePerformedMsg` (`0x82d515c8`) of an active objective: a gesture not done yet, while
    /// there is room.
    pub fn gesture_performed(&mut self, gesture: i32) {
        if !self.unique_gestures.contains(&gesture) && self.unique_gestures.len() < UNIQUE_GESTURES_CAPACITY {
            self.unique_gestures.push(gesture);
        }
    }

    /// `TrickCompleteMsg`'s (`0x82d4dfd0`) big-trick achievements, for a player the skater manager
    /// counts: a finished powerslide (threshold 2) or manual (0), and a trick of type 4 (grab
    /// time, threshold 4) or 6 (handplant height, 3).
    pub fn trick_completed(&mut self, frame: &MissionFrame, trick_type: i32) {
        if frame.finished_powerslide.is_some() {
            self.update_big_trick_achievement(frame.big_trick_achieved[2], 2);
        }
        if frame.finished_manual.is_some() {
            self.update_big_trick_achievement(frame.big_trick_achieved[0], 0);
        }
        if trick_type == 4 {
            self.update_big_trick_achievement(frame.big_trick_achieved[4], 4);
        } else if trick_type == 6 {
            self.update_big_trick_achievement(frame.big_trick_achieved[3], 3);
        }
    }

    /// `GrindCompletedMsg`'s (`0x82d51140`) grind-length achievement (threshold 1) and, for a valid
    /// grind object with a type, its DMO grind context.
    pub fn grind_completed(&mut self, frame: &MissionFrame) {
        self.update_big_trick_achievement(frame.big_trick_achieved[1], 1);
        let (valid, entity_type, unique_id) = frame.finished_grind_object;
        if valid && entity_type != -1 {
            self.add_contextual_data(ContextualData { achievement_type: achievement::GRIND, group: GROUP_DMO, unique_id, entity_type });
        }
    }

    /// `OnPlayerResult`'s (`0x82d43bc0`) round score and rank; a round outside 1 to 5 is the native's
    /// assertion, written past the arrays.
    pub fn player_result(&mut self, round: i32, rank: i32, score: i32) {
        if let Some(index) = usize::try_from(round - 1).ok().filter(|&i| i < 5) {
            self.round_score[index] = score;
            self.round_rank[index] = rank;
        }
    }
}
