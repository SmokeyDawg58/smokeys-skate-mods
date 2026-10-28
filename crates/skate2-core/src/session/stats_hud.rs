//! `PhysOutModule_Hud::Update` (`0x827c79b8`)'s statistics, personal bests and achievements, with
//! the line bests `TricksBackEnd` keeps (`SetLineTimePoints` `0x82a735b8`, `SetLineScore`
//! `0x82a73588`, `UpdateTrickLandedStats` `0x825770e8`, `Update` `0x82564a88`), one simulation frame
//! at a time. `AreStatsAndAchievementsAvailable` (`0x827c4ac0`: recording allowed, for the local
//! human skater) holds back the module's own updates; `UpdateStats`, `UpdatePersonalBests` and
//! `RecordPersonalBests` keep their own gates, and `UpdateDmoStats`, `UpdateMovableObjectStat` and
//! `GrantAchievement` have none.
use super::{StatThresholds, StatsBackend};
use crate::{
    input_cvar_runtime::InputCvarReferences, physics_state_ids::PHYSICS_STATE_SKITCHING,
    reckoning_numeric::host_dot3, ui::skate_profile::fctiwz,
};

/// `kUpdateTime`: one 60 Hz frame.
const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);
/// The squared distance a grabbed object has to move from where the grab began (`0x827c7100`).
const OBJECT_MOVED: f32 = f32::from_bits(0x38d1_b717);
/// `UpdateDistanceTravelled`'s `kBankableMultiple`.
const BANKABLE_MULTIPLE: f32 = 1.0;
/// `CheckForObjectSequences` (`0x827c4508`)'s object lists, each `eEntityType` (barrier 14 to table
/// 30) a finished sequence has to have touched, -1 unused: `UpdateDmoStats(3 + list, score)`.
const OBJECT_SEQUENCES: [[i32; 4]; 10] = [
    [28, 30, 22, -1],
    [28, 29, 18, -1],
    [15, 30, 18, -1],
    [21, 25, 23, -1],
    [30, 18, 19, -1],
    [15, 28, 19, -1],
    [29, 22, 18, -1],
    [29, 14, 20, 24],
    [28, 15, 22, -1],
    [28, 29, 27, -1],
];

/// A trick of `mTricksEnded` that was not a tap trick, by `GetTrickType`: a flip trick, a late flip
/// (`IsLateFlipTrick`), or another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EndedTrick {
    Flip,
    LateFlip,
    Other,
}

/// `mAirFinishedData` of a frame with `mAirFinished`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HudAir {
    /// `mLandedSuccesfully`.
    pub(crate) landed: bool,
    pub(crate) total_height_gain: f32,
    pub(crate) in_air_frames: i32,
    pub(crate) height_to_peak: f32,
    pub(crate) horizontal_distance: f32,
    /// `mAirGapData[0]` (grass) and `[1]` (stairs): `mAccumulatedGapDistance`.
    pub(crate) grass_gap: f32,
    pub(crate) stair_gap: f32,
    pub(crate) grab_time: f32,
    /// `mPlayerNumFlips`.
    pub(crate) flips: i32,
    /// The accumulated context less the take-off's and the landing's: group 1's first bank, group
    /// 0's, and the security guards in group 0's.
    pub(crate) vehicles_over: u32,
    pub(crate) pedestrians_over: u32,
    pub(crate) security_over: u32,
}

/// The finished sequence of a frame with `mSequenceCompleted`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HudSequence {
    /// `mSequenceCompletedSuccessfully`.
    pub(crate) successful: bool,
    pub(crate) score: u32,
    pub(crate) tricks: u32,
    pub(crate) tapped: u32,
    pub(crate) duration: f32,
    /// `mFinishedSequenceDynamicObjectSet`, from `eEntityType_FirstDmo`.
    pub(crate) objects: [bool; 17],
}

/// The physics output `PhysOutModule_Hud::Update` reads for its statistics.
#[derive(Clone, Debug, Default, PartialEq)]
#[expect(clippy::struct_excessive_bools, reason = "the output's own flags")]
pub(crate) struct HudStatsFrame {
    /// `PhysOut_State`: `mIsWipingOut` and `mPhysicsState`.
    pub(crate) wiping_out: bool,
    pub(crate) physics_state: i32,
    /// `PhysOut_Filtered::mFilteredPhysicsState` is ground or off board.
    pub(crate) filtered_ground: bool,
    pub(crate) filtered_offboard: bool,
    /// `PhysOut_Skeleton::mIsWipeoutOver`.
    pub(crate) wipeout_over: bool,
    /// `PhysOut_Animation`: `mPushMongo` and `mPushRegular`.
    pub(crate) push_mongo: bool,
    pub(crate) push_regular: bool,
    /// `PhysOut_SystemReckoning`: `mCentreOfMass`'s height and `mDampedCentreOfMassVelocity`.
    pub(crate) centre_of_mass_height: f32,
    pub(crate) damped_velocity: [f32; 4],
    /// `PhysOut_Scoring2`: `mDistanceTravelledThisFrame`, `mIsOffboardAndNotHippyJumping`,
    /// `mIsRunningOut`, `mCurrentOffboardDurationInSeconds` and `mIsAttemptingABodyFlip`.
    pub(crate) distance: f32,
    pub(crate) offboard_not_hippy_jumping: bool,
    pub(crate) running_out: bool,
    pub(crate) offboard_duration: f32,
    pub(crate) attempting_body_flip: bool,
    /// `mTricksCancelled` that were not tap tricks.
    pub(crate) cancelled: u32,
    /// `mTricksEnded` is not empty, and its tricks that were not tap tricks.
    pub(crate) tricks_ended: bool,
    pub(crate) ended: Vec<EndedTrick>,
    /// `mTricksBegun`'s and `mTricksConvertedTo`'s first trick when there is one: whether its
    /// settings are valid, which makes its index `SetCurrentTrick`'s.
    pub(crate) begun: Option<bool>,
    pub(crate) converted: Option<bool>,
    /// `mIsInState_Air` or `mIsHippyJumping`, and the 180s: `mHippyJump180Count` while hippy
    /// jumping, else `mCurrentAirData.mPlayerSpin180Count`.
    pub(crate) aerial: bool,
    pub(crate) spins: i32,
    pub(crate) sequence_begun: bool,
    pub(crate) wipeout_interrupted_trick: bool,
    pub(crate) just_wiped_out: bool,
    /// The finished handplant's duration, the finished manual's, grind's and powerslide's length.
    pub(crate) handplant: Option<f32>,
    pub(crate) manual: Option<f32>,
    pub(crate) grind: Option<f32>,
    pub(crate) powerslide: Option<f32>,
    pub(crate) air: Option<HudAir>,
    pub(crate) sequence: Option<HudSequence>,
    /// `mLineScore` and `mLineTimerPoints`.
    pub(crate) line_score: i32,
    pub(crate) line_timer_points: i32,
    /// `UpdateHUDsAndObjectGrabbing`'s grab: `PhysOut_State::mGrabSplineDesc.mObjectID` while the
    /// skater is not skitching, is off board and `PhysOut_OffBoard::mIsGrabbingObject` holds.
    pub(crate) grab_spline: Option<u32>,
    pub(crate) thresholds: StatThresholds,
}

/// The dynamic object `UpdateHUDsAndObjectGrabbing` (`0x827c6ee0`) finds the skater grabbing (not
/// skitching, off board, `GetGrabSplineOwnerObjectID`): its entity, `GetPosition`, and its
/// characteristics' `MovableObjectType` while they have a collection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GrabbedObject {
    pub(crate) entity: u32,
    pub(crate) position: [f32; 4],
    pub(crate) movable_type: Option<i32>,
}

impl GrabbedObject {
    /// `GetGrabSplineOwnerObjectID`'s dynamic object, `GetPosition` (`0x82d1d220`: its transform's
    /// translation) and the characteristics layout's `MovableObjectType` (+0x68).
    pub(crate) fn find(
        objects: &crate::living_world_grind_services::dmo_objects::DmoObjectMan,
        spline: u32,
    ) -> Result<Option<Self>, String> {
        let entity = objects.grab_spline_owner(spline);
        if entity == 0 {
            return Ok(None);
        }
        let Some(object) = objects.object(entity) else {
            return Ok(None);
        };
        let movable_type = object
            .model
            .characteristics
            .try_cached_layout_field(0x68, 4)
            .map_err(|error| format!("MovableObjectType: {error:?}"))?
            .map(|data| data.bytes().try_into().map(i32::from_be_bytes))
            .transpose()
            .map_err(|_| "MovableObjectType size")?;
        let transform = &object.transform;
        Ok(Some(Self {
            entity,
            position: [transform[12], transform[13], transform[14], transform[15]],
            movable_type,
        }))
    }
}

/// `PhysOutModule_Hud`'s statistics members and the `TricksBackEnd` members its line bests use.
#[derive(Clone, Debug, Default, PartialEq)]
#[expect(clippy::struct_excessive_bools, reason = "the native members")]
pub(crate) struct HudStats {
    /// `mPendingStatOverallDistanceOnBoard`, `...OffBoard`, `...Skitched` and `...Wipedout`.
    pending_onboard: f32,
    pending_offboard: f32,
    pending_skitched: f32,
    pending_wipedout: f32,
    /// `mPendingStatCurrentDistanceOffBoard` and `...Skitched`, and what of them is banked
    /// (`mCurrentDistanceOffBoard`, `mCurrentDistanceSkitched`).
    pending_current_offboard: f32,
    pending_current_skitched: f32,
    current_offboard: f32,
    current_skitched: f32,
    /// `mTimeAtMaxSpeed`.
    time_at_max_speed: f32,
    /// `mObjectUsedLastFrame`, and its position when the grab began.
    object_used_last_frame: Option<u32>,
    object_start: [f32; 4],
    /// `mbActiveSpin`.
    active_spin: bool,
    /// `TricksBackEndCore`: `mIsWipeoutOver`, and whether `mTrickInfo.mTrickIndex` names valid
    /// settings.
    wipeout_over: bool,
    current_trick_valid: bool,
    /// `TricksBackEnd`: `mbLineScoreOn`, `mLineTimerPoints`, `mCurrentLineTime` and
    /// `mNumCurrentLineTricks`.
    line_on: bool,
    line_timer_points: i32,
    line_time: f32,
    line_tricks: u32,
}

/// One frame's back end with its recording gates (`IsRecordingAllowed`,
/// `IsPersonalBestsRecordingAllowed`) and thresholds.
struct Recorder<'a> {
    stats: &'a mut StatsBackend,
    recording: bool,
    bests: bool,
    thresholds: &'a StatThresholds,
}

impl Recorder<'_> {
    /// `UpdateStats(item, value, false)`.
    fn stat(&mut self, item: usize, value: f32) -> Result<(), String> {
        if self.recording { self.stats.apply_stat(item, value, self.thresholds) } else { Ok(()) }
    }

    /// `UpdatePersonalBests(item, value)`.
    fn best(&mut self, item: usize, value: f32) -> Result<(), String> {
        if self.bests { self.stats.raise_best(item, value) } else { Ok(()) }
    }

    /// `RecordPersonalBests(item)`.
    fn record(&mut self, item: usize) -> Result<(), String> {
        if self.bests { self.stats.record_best(item) } else { Ok(()) }
    }

    /// `UpdateAndRecordPersonalBest` (`0x827c43e8`).
    fn best_and_record(&mut self, item: usize, value: f32) -> Result<(), String> {
        self.best(item, value)?;
        self.record(item)
    }

    /// `BankStatistic` (`0x827c4900`): the pending value's whole multiples go to the statistic.
    fn bank(&mut self, item: usize, pending: &mut f32) -> Result<(), String> {
        let banked = whole_multiples(*pending);
        if banked > 0.0 {
            self.stat(item, banked)?;
            *pending -= banked;
        }
        Ok(())
    }

    /// `BankPersonalBest` (`0x827c4998`): the pending value's whole multiples go to the run, whose
    /// banked part with what is still pending raises the best.
    fn bank_best(&mut self, item: usize, run: &mut f32, pending: &mut f32) -> Result<(), String> {
        let banked = whole_multiples(*pending);
        if banked > 0.0 {
            *run += banked;
            *pending -= banked;
            self.best(item, *run + *pending)?;
            self.record(item)?;
        }
        Ok(())
    }
}

/// `fctiwz(pending / kBankableMultiple)` back to a float, times the multiple.
#[expect(clippy::cast_precision_loss, reason = "the native fcfid")]
fn whole_multiples(pending: f32) -> f32 {
    fctiwz(pending / BANKABLE_MULTIPLE) as f32 * BANKABLE_MULTIPLE
}

/// `DoesObjectSetContainObjects` (`0x827c4438`): every used entry is a dynamic object type the set
/// holds.
fn contains_objects(set: &[bool; 17], objects: &[i32; 4]) -> bool {
    objects.iter().all(|&kind| {
        kind == -1 || ((14..=30).contains(&kind) && usize::try_from(kind - 14).is_ok_and(|index| set[index]))
    })
}

impl super::LocalBackendOwner {
    /// One simulation frame of `PhysOutModule_Hud::Update`'s statistics, then `TricksBackEnd::Update`'s
    /// line bests, which `FrontEndManager::Update` runs after the frame's simulation; the line time
    /// adds `Simulation::mCurrentSimUpdateRate`.
    pub(crate) fn update_hud_stats(
        &self,
        hud: &mut HudStats,
        frame: &HudStatsFrame,
        grabbed: Option<GrabbedObject>,
        sim_period: f32,
        cvars: &InputCvarReferences,
    ) -> Result<(), String> {
        let mut stats = self.statistics.lock().map_err(|_| "StatsBackEnd lock")?;
        let recording = self.statistics_recording_allowed(&stats, cvars)?;
        let bests = self.personal_bests_recording_allowed(&stats, cvars)?;
        let mut recorder = Recorder { stats: &mut stats, recording, bests, thresholds: &frame.thresholds };
        hud.update(&mut recorder, frame, grabbed, sim_period)
    }
}

impl HudStats {
    #[expect(clippy::cast_precision_loss, reason = "the native fcfid of the line score")]
    fn update(&mut self, r: &mut Recorder<'_>, f: &HudStatsFrame, grabbed: Option<GrabbedObject>, sim_period: f32) -> Result<(), String> {
        // AreStatsAndAchievementsAvailable: the local skater is the human one.
        let track = r.recording;
        r.stats.clear_best_flags();
        self.object_grabbing(r, grabbed, track)?;
        air_context(r, f, track)?;
        if track && f.attempting_body_flip {
            r.stat(35, 1.0)?;
        }
        if track {
            // UpdatePushingStats (0x827c4338) and UpdateTricksCancelled (0x827c5a30).
            if f.push_mongo {
                r.stat(38, 1.0)?;
            }
            if f.push_regular {
                r.stat(37, 1.0)?;
            }
            for _ in 0..f.cancelled {
                r.stat(41, 1.0)?;
            }
        }
        if f.begun.is_some() {
            self.active_spin = true;
        }
        if f.sequence_begun {
            self.active_spin = false;
        }
        self.set_line_timer_points(f.line_timer_points);
        // SetLineScore: UpdateLineScoreStats (0x8255c2d8) raises the line score best each frame.
        r.best(19, f.line_score.cast_unsigned() as f32)?;
        self.distance_travelled(r, f, track)?;
        air_finished(r, f, track)?;
        if !f.wipeout_interrupted_trick {
            if let Some(valid) = f.begun.or(f.converted) {
                self.current_trick_valid = valid;
            }
            if f.aerial && !self.active_spin && f.spins != 0 {
                self.active_spin = true;
                self.current_trick_valid = false;
            }
            self.trick_completion(r, f, track)?;
            self.sequence_completion(r, f, track)?;
        }
        self.wipeouts(r, f, track)?;
        self.line_update(r, sim_period)
    }

    /// `UpdateHUDsAndObjectGrabbing`'s statistics: a grabbed object moved from where the grab began
    /// adds the frame to its type's time and to `UpdateDmoStats(0)`, and grants I Like To Move It.
    fn object_grabbing(&mut self, r: &mut Recorder<'_>, grabbed: Option<GrabbedObject>, track: bool) -> Result<(), String> {
        let Some(object) = grabbed.filter(|_| track) else {
            self.object_used_last_frame = None;
            return Ok(());
        };
        if self.object_used_last_frame != Some(object.entity) {
            self.object_start = object.position;
            self.object_used_last_frame = Some(object.entity);
            return Ok(());
        }
        let (position, start) = (object.position, self.object_start);
        let delta = std::array::from_fn(|i| position[i] - start[i]);
        if host_dot3(delta, delta) > OBJECT_MOVED {
            if let Some(kind) = object.movable_type.filter(|&kind| kind != -1) {
                let kind = usize::try_from(kind).map_err(|_| "UpdateMovableObjectStat type assertion")?;
                r.stats.add_moving_time(kind, UPDATE_TIME)?;
            }
            r.stats.update_dmo(0, UPDATE_TIME, r.thresholds)?;
            r.stats.grant(2, 0)?;
        }
        Ok(())
    }

    /// `UpdateDistanceTravelled` (`0x827c5e48`): the frame's distance banks into the wipeout, the
    /// off-board (and its run's best) or the on-board distance, and the skitching distance, time and
    /// run's best; Skater Evolved for a run off board, Need For Speed for time on the ground at the
    /// top speed.
    fn distance_travelled(&mut self, r: &mut Recorder<'_>, f: &HudStatsFrame, track: bool) -> Result<(), String> {
        let distance = if f.distance > f32::from_bits(0x3ba3_d70a) { f.distance } else { 0.0 };
        let offboard = !f.wiping_out && f.offboard_not_hippy_jumping;
        let skitching = !f.wiping_out && !offboard && f.physics_state == PHYSICS_STATE_SKITCHING;
        let mut speeding = false;
        if track {
            if f.wiping_out {
                self.pending_wipedout += distance;
                r.bank(49, &mut self.pending_wipedout)?;
            } else {
                if offboard {
                    self.pending_offboard += distance;
                    self.pending_current_offboard += distance;
                    r.bank(20, &mut self.pending_offboard)?;
                    r.bank_best(9, &mut self.current_offboard, &mut self.pending_current_offboard)?;
                    if f.filtered_offboard && !f.running_out && f.offboard_duration > 0.5 {
                        r.stats.grant(1, 0)?;
                    }
                } else {
                    self.pending_onboard += distance;
                    r.bank(18, &mut self.pending_onboard)?;
                    if skitching {
                        self.pending_skitched += distance;
                        self.pending_current_skitched += distance;
                        r.bank(29, &mut self.pending_skitched)?;
                        r.stat(30, UPDATE_TIME)?;
                        r.bank_best(8, &mut self.current_skitched, &mut self.pending_current_skitched)?;
                    }
                }
                let top = f.thresholds.max_speed;
                speeding = host_dot3(f.damped_velocity, f.damped_velocity) > top * top;
                if speeding && f.filtered_ground {
                    self.time_at_max_speed += UPDATE_TIME;
                    if self.time_at_max_speed > f.thresholds.time_at_max_speed {
                        r.stats.grant(38, 0)?;
                    }
                }
            }
        }
        if !speeding {
            self.time_at_max_speed = 0.0;
        }
        if !offboard {
            self.pending_current_offboard = 0.0;
            self.current_offboard = 0.0;
        }
        if !skitching {
            self.current_skitched = 0.0;
            self.pending_current_skitched = 0.0;
        }
        Ok(())
    }

    /// `UpdateTrickCompletion` (`0x827c5af0`): `SetTrickLanded(Success)` for ended tricks; each one
    /// not tapped counts a flip or late flip and a successful trick; the finished handplant's time
    /// (and On Top Of The World above its height), manual, grind and powerslide count and raise
    /// their bests.
    fn trick_completion(&mut self, r: &mut Recorder<'_>, f: &HudStatsFrame, track: bool) -> Result<(), String> {
        if f.tricks_ended {
            self.trick_landed(r)?;
        }
        if !track {
            return Ok(());
        }
        for &trick in &f.ended {
            match trick {
                EndedTrick::Flip => r.stat(33, 1.0)?,
                EndedTrick::LateFlip => r.stat(36, 1.0)?,
                EndedTrick::Other => {}
            }
            r.stat(42, 1.0)?;
        }
        if let Some(duration) = f.handplant {
            r.stat(21, duration)?;
            if f.centre_of_mass_height > f.thresholds.top_of_the_world {
                r.stats.grant(0, 0)?;
            }
        }
        for (length, best, statistic) in [(f.manual, 4, 39), (f.grind, 5, 31), (f.powerslide, 3, 40)] {
            if let Some(length) = length {
                r.best(best, length)?;
                r.record(best)?;
                r.stat(statistic, length)?;
            }
        }
        Ok(())
    }

    /// `UpdateTrickLandedStats(eTrickCompletionStatus_Success)`'s line count, while the current
    /// trick's settings are valid.
    #[expect(clippy::cast_precision_loss, reason = "the native fcfid")]
    fn trick_landed(&mut self, r: &mut Recorder<'_>) -> Result<(), String> {
        if self.current_trick_valid && self.line_on {
            self.line_tricks = self.line_tricks.wrapping_add(1);
            r.best(21, self.line_tricks as f32)?;
        }
        Ok(())
    }

    /// `UpdateSequenceCompletion` (`0x827c4668`): a sequence finished without a wipeout ends the
    /// active spin; one finished successfully raises the score, trick and time bests and
    /// `CheckForObjectSequences`' dynamic object bests.
    #[expect(clippy::cast_precision_loss, reason = "the native fcfid")]
    fn sequence_completion(&mut self, r: &mut Recorder<'_>, f: &HudStatsFrame, track: bool) -> Result<(), String> {
        let Some(sequence) = &f.sequence else { return Ok(()) };
        if f.wipeout_interrupted_trick || f.just_wiped_out {
            return Ok(());
        }
        self.active_spin = false;
        if !(track && sequence.successful) {
            return Ok(());
        }
        let tricks = sequence.tricks.wrapping_sub(sequence.tapped);
        let score = sequence.score as f32;
        r.best_and_record(16, score)?;
        r.best_and_record(18, tricks as f32)?;
        r.best_and_record(17, sequence.duration)?;
        for (index, objects) in OBJECT_SEQUENCES.iter().enumerate() {
            if contains_objects(&sequence.objects, objects) {
                r.stats.update_dmo(index + 3, score, r.thresholds)?;
            }
        }
        Ok(())
    }

    /// `UpdateWipeouts` (`0x827c47b8`): a wipeout counts once it is over, until the skater is back on
    /// the ground.
    fn wipeouts(&mut self, r: &mut Recorder<'_>, f: &HudStatsFrame, track: bool) -> Result<(), String> {
        if f.wipeout_over && !self.wipeout_over {
            if track {
                r.stat(17, 1.0)?;
            }
            self.wipeout_over = true;
        }
        if self.wipeout_over && f.filtered_ground {
            self.wipeout_over = false;
        }
        Ok(())
    }

    /// `SetLineTimePoints(points)`: a line starting resets its time and tricks.
    const fn set_line_timer_points(&mut self, points: i32) {
        self.line_timer_points = points;
        if points > 0 {
            if !self.line_on {
                self.line_tricks = 0;
                self.line_time = 0.0;
            }
            self.line_on = true;
        }
    }

    /// `TricksBackEnd::Update`'s line: its time raises the line time best while its timer has points,
    /// and its end records the line's three bests.
    fn line_update(&mut self, r: &mut Recorder<'_>, sim_period: f32) -> Result<(), String> {
        if !self.line_on {
            return Ok(());
        }
        if self.line_timer_points > 0 {
            self.line_time += sim_period;
            r.best(20, self.line_time)
        } else {
            r.record(19)?;
            r.record(20)?;
            r.record(21)?;
            self.line_on = false;
            Ok(())
        }
    }
}

/// `UpdateAirContextData` (`0x827c6c40`)'s statistics: a successful landing counts the vehicles,
/// pedestrians and security guards jumped over.
#[expect(clippy::cast_precision_loss, reason = "the native fcfid")]
fn air_context(r: &mut Recorder<'_>, f: &HudStatsFrame, track: bool) -> Result<(), String> {
    let Some(air) = f.air.as_ref().filter(|air| air.landed && track) else { return Ok(()) };
    r.stat(47, air.vehicles_over as f32)?;
    if air.pedestrians_over != 0 {
        r.stat(43, air.pedestrians_over as f32)?;
    }
    if air.security_over != 0 {
        r.stat(46, air.security_over as f32)?;
    }
    Ok(())
}

/// `UpdateAirFinished` (`0x827c5818`)'s statistics: a successful landing raises the drop, air time,
/// launch, distance and gap bests (Grasshopper and Stairmaster past their gaps), counts the grab
/// time and the flips.
#[expect(clippy::cast_precision_loss, reason = "the native fcfid")]
fn air_finished(r: &mut Recorder<'_>, f: &HudStatsFrame, track: bool) -> Result<(), String> {
    let Some(air) = f.air.as_ref().filter(|air| air.landed && track) else { return Ok(()) };
    let drop = -air.total_height_gain;
    if drop > 0.0 {
        r.best_and_record(7, drop)?;
    }
    r.best_and_record(2, air.in_air_frames as f32 * UPDATE_TIME)?;
    r.best_and_record(6, air.height_to_peak)?;
    r.best_and_record(10, air.horizontal_distance)?;
    r.best_and_record(11, air.grass_gap)?;
    r.best_and_record(12, air.stair_gap)?;
    if air.grass_gap > f.thresholds.grass_gap {
        r.stats.grant(41, 0)?;
    }
    if air.stair_gap > f.thresholds.stair_gap {
        r.stats.grant(42, 0)?;
    }
    if air.grab_time > 0.0 {
        r.stat(32, air.grab_time)?;
    }
    if air.flips != 0 {
        r.stat(34, air.flips.wrapping_abs() as f32)?;
    }
    Ok(())
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "exact native accumulations")]
mod tests {
    use super::{EndedTrick, GrabbedObject, HudAir, HudSequence, HudStats, HudStatsFrame, Recorder, contains_objects};
    use crate::skater_manager_local_identity::stats::{StatThresholds, StatsBackend};

    fn run(hud: &mut HudStats, stats: &mut StatsBackend, frame: &HudStatsFrame, grabbed: Option<GrabbedObject>) {
        let mut recorder = Recorder { stats, recording: true, bests: true, thresholds: &frame.thresholds };
        hud.update(&mut recorder, frame, grabbed, 1.0 / 60.0).unwrap();
    }

    #[test]
    fn distance_banks_whole_metres() {
        let (mut hud, mut stats) = (HudStats::default(), StatsBackend::initialize());
        let frame = HudStatsFrame { distance: 0.4, ..Default::default() };
        for _ in 0..5 {
            run(&mut hud, &mut stats, &frame, None);
        }
        // 2.0 m travelled: whole metres reach the statistic, the rest stays pending.
        assert_eq!(stats.statistics[18], 2.0);
        let frame = HudStatsFrame { distance: 0.004, ..Default::default() };
        run(&mut hud, &mut stats, &frame, None);
        assert_eq!(stats.statistics[18], 2.0);
    }

    #[test]
    fn a_landing_raises_air_bests_and_counts_flips() {
        let (mut hud, mut stats) = (HudStats::default(), StatsBackend::initialize());
        let air = HudAir { landed: true, total_height_gain: -3.0, in_air_frames: 120, height_to_peak: 2.5, flips: -2, grab_time: 0.5, ..Default::default() };
        let frame = HudStatsFrame { air: Some(air), ended: vec![EndedTrick::Flip, EndedTrick::LateFlip], ..Default::default() };
        run(&mut hud, &mut stats, &frame, None);
        assert_eq!(stats.personal_bests[7].value, 3.0);
        assert_eq!(stats.personal_bests[2].value, 120.0 * f32::from_bits(0x3c88_8889));
        assert_eq!(stats.personal_bests[6].value, 2.5);
        assert_eq!((stats.statistics[32], stats.statistics[34]), (0.5, 2.0));
        assert_eq!((stats.statistics[33], stats.statistics[36], stats.statistics[42]), (1.0, 1.0, 2.0));
    }

    #[test]
    fn line_bests_follow_the_line_timer() {
        let (mut hud, mut stats) = (HudStats::default(), StatsBackend::initialize());
        let begun = HudStatsFrame { line_timer_points: 10, line_score: 3000, begun: Some(true), ..Default::default() };
        run(&mut hud, &mut stats, &begun, None);
        let landed = HudStatsFrame { line_timer_points: 10, line_score: 4500, tricks_ended: true, ..Default::default() };
        for _ in 0..1300 {
            run(&mut hud, &mut stats, &landed, None);
        }
        // Each best only breaks past its threshold: 2000 points, 20 seconds, 4 tricks.
        assert_eq!(stats.personal_bests[19].value, 4500.0);
        assert_eq!(stats.personal_bests[21].value, 1300.0);
        assert!(stats.personal_bests[20].value > 21.0);
        assert!(!stats.personal_bests[21].recorded);
        run(&mut hud, &mut stats, &HudStatsFrame::default(), None);
        assert!((19..=21).all(|item| stats.personal_bests[item].recorded));
    }

    #[test]
    fn a_grabbed_object_counts_once_it_moved() {
        let (mut hud, mut stats) = (HudStats::default(), StatsBackend::initialize());
        let at = |x| Some(GrabbedObject { entity: 7, position: [x, 0.0, 0.0, 1.0], movable_type: Some(3) });
        run(&mut hud, &mut stats, &HudStatsFrame::default(), at(0.0));
        run(&mut hud, &mut stats, &HudStatsFrame::default(), at(0.005));
        assert_eq!(stats.movable_object_times[3], 0.0);
        run(&mut hud, &mut stats, &HudStatsFrame::default(), at(0.5));
        assert_eq!(stats.movable_object_times[3], f32::from_bits(0x3c88_8889));
        assert_eq!(stats.dmo_statistics[0], f32::from_bits(0x3c88_8889));
        assert!(stats.achievements[2]);
    }

    #[test]
    fn statistics_survive_a_save_and_load() {
        use crate::{
            challenge::attrib_db::AttribDb,
            simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo,
            skater_manager_local_identity::LocalBackendOwner,
        };
        let game = PhysicalGameInfo::construct_network_field();
        let saved = LocalBackendOwner::construct_static_fields(std::sync::Arc::clone(&game));
        {
            let mut stats = saved.statistics.lock().unwrap();
            let frame = HudStatsFrame { distance: 2.5, attempting_body_flip: true, ..Default::default() };
            run(&mut HudStats::default(), &mut stats, &frame, None);
            stats.update_dmo(5, 900.0, &StatThresholds::default()).unwrap();
            stats.add_moving_time(7, 0.5).unwrap();
            stats.raise_best(10, 30.0).unwrap();
            stats.grant(41, 0).unwrap();
        }
        let (_, version, tuples) = saved.save_statistics([2026, 10, 18, 12, 0, 0]).unwrap().remove(0);
        let loaded = LocalBackendOwner::construct_static_fields(game);
        let leaf = skate2_formats::skater_profile::MemcardLeaf { version, tuples };
        loaded.load_statistics(Some(&leaf), &AttribDb::empty()).unwrap();
        let (before, after) = (saved.statistics.lock().unwrap(), loaded.statistics.lock().unwrap());
        assert_eq!((after.statistics[18], after.statistics[35], after.dmo_statistics[5]), (2.0, 1.0, 900.0));
        assert_eq!((after.movable_object_times[7], after.personal_bests[10].value), (0.5, 30.0));
        assert_eq!(before.statistics, after.statistics);
        assert_eq!(before.dmo_statistics, after.dmo_statistics);
        assert_eq!(before.movable_object_times, after.movable_object_times);
        assert_eq!(before.achievements, after.achievements);
        assert!(before.personal_bests.iter().zip(&after.personal_bests).all(|(a, b)| a.value == b.value));
    }

    #[test]
    fn object_sequences_need_every_listed_type() {
        let mut set = [false; 17];
        for kind in [28, 30, 22] {
            set[kind - 14] = true;
        }
        assert!(contains_objects(&set, &[28, 30, 22, -1]));
        assert!(!contains_objects(&set, &[28, 29, 18, -1]));
        let sequence = HudSequence { successful: true, score: 900, tricks: 9, tapped: 1, duration: 4.0, objects: set };
        let (mut hud, mut stats) = (HudStats::default(), StatsBackend::initialize());
        run(&mut hud, &mut stats, &HudStatsFrame { sequence: Some(sequence), thresholds: StatThresholds::default(), ..Default::default() }, None);
        assert_eq!((stats.personal_bests[16].value, stats.personal_bests[18].value), (900.0, 8.0));
        assert_eq!(stats.dmo_statistics[3], 900.0);
    }
}
