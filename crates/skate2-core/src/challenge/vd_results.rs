//! `Sk8::VisualDirector::VDWriter`'s challenge success and failure info. The writer (constructor
//! `0x827feae8`) takes `cMsgObjectiveComplete` (`ReceiveObjectiveCompleteMsg` `0x827fc728`),
//! `cMsgChallengeSuccess` (`0x827fc738`, from `cSignPostManager::WinChallenge`) and
//! `cMsgChallengeFailed` (`0x827fc748`, from `LoseChallenge` and `AbandonChallenge`) on the
//! challenge broadcaster; each starts its time at 0 (-1 while idle). Each simulation submit
//! (`VDManager::DoSimSubmit` `0x82800120`, `WriteData` `0x827ff8c8`) evaluates the running times
//! on the challenge-objectives visual treatment and steps them, and the presentation restores
//! the info with the frame (`VDReader::ReadData` `0x827fdd78`) for `PostFX::VisualFX_Animation`
//! (`0x82847610`).

use super::{
    attrib_db::AttribDb,
    keys,
    system::{Event, World},
};
use crate::physics::PointGraph;

/// The challenge-objectives visual treatment (`VDWriter::mChallengeObjectivesAttribs`): class
/// `0xF332E7155560FC55`, collection `default`.
const TREATMENT_CLASS: u64 = 0xf332_e715_5560_fc55;
const TREATMENT: u64 = 0xd7ed_bd36_2d7d_2152;

/// The treatment's graphs (`0x90` bytes: the duration at +8, sixteen x at +0x10 and sixteen y
/// at +0x50). Every y of the shipped data is 0.
mod field {
    /// After an objective completes (1 s).
    pub const OBJECTIVE_COMPLETE: u64 = 0x4f36_d468_78b3_a902;
    /// After `WinChallenge` (3 s).
    pub const CHALLENGE_SUCCESS: u64 = 0x3470_740a_cc17_6507;
    /// After `LoseChallenge` or `AbandonChallenge` (3 s).
    pub const CHALLENGE_FAILED: u64 = 0x978a_5a9d_ce1c_e26e;
}

/// `kUpdateTime`.
const UPDATE_TIME: f32 = 0.016_666_668;

/// `mTimeSinceObjectiveComplete`, `mTimeSinceChallengeSuccess` and `mTimeSinceChallengeFail`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResultTimes {
    objective_complete: f32,
    challenge_success: f32,
    challenge_fail: f32,
}

impl Default for ResultTimes {
    fn default() -> Self {
        Self { objective_complete: -1.0, challenge_success: -1.0, challenge_fail: -1.0 }
    }
}

/// One submit's `VD_ChallengeSuccessInfo` and `VD_ChallengeFailedInfo`
/// (`mNormalizedChallengeSuccessValue`, `mNormalizedChallengeFailValue`), each while written.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResultInfo {
    pub success: Option<f32>,
    pub failed: Option<f32>,
}

/// The writer's receivers.
pub(crate) fn receive(world: &mut World, event: &Event) {
    let times = &mut world.vd_results;
    match event {
        Event::ChallengeFailed { .. } => times.challenge_fail = 0.0,
        Event::Challenge(m) if m.kind == keys::hash32("cMsgObjectiveComplete") => times.objective_complete = 0.0,
        Event::Challenge(m) if m.kind == keys::hash32("cMsgChallengeSuccess") => times.challenge_success = 0.0,
        _ => {}
    }
}

/// A running time's graph value, then its step: `PointGraphEval::Evaluate` (`0x824e2c40`) of the
/// graph at the time, the time plus `kUpdateTime`, idle again past the graph's duration. A
/// missing graph reads `Attrib::DefaultDataArea`'s zeros.
fn step(db: &AttribDb, key: u64, time: &mut f32) -> f32 {
    let bytes = db.attribute(TREATMENT_CLASS, TREATMENT, key).and_then(|a| a.value_bytes(0)).unwrap_or_default();
    let float = |offset: usize| bytes.get(offset..offset + 4).map_or(0.0, |b| f32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let graph =
        PointGraph::<16> { x: std::array::from_fn(|i| float(0x10 + 4 * i)), y: std::array::from_fn(|i| float(0x50 + 4 * i)) };
    let value = graph.evaluate(*time);
    *time += UPDATE_TIME;
    if *time > float(8) {
        *time = -1.0;
    }
    value
}

/// `_PrepareChallengeSuccessData` (`0x827fc820`) and `_PrepareChallengeFailedData`
/// (`0x827fc9c0`) of a submit: the success value is the larger (`fsel`) of the objective's and
/// the challenge's, each -1 while idle; an info is written unless its value is negative.
pub(crate) fn write(world: &mut World) -> ResultInfo {
    let (db, times) = (&world.db, &mut world.vd_results);
    let mut objective = -1.0;
    if times.objective_complete >= 0.0 {
        objective = step(db, field::OBJECTIVE_COMPLETE, &mut times.objective_complete);
    }
    let mut challenge = -1.0;
    if times.challenge_success >= 0.0 {
        challenge = step(db, field::CHALLENGE_SUCCESS, &mut times.challenge_success);
    }
    let success = if objective - challenge >= 0.0 { objective } else { challenge };
    let mut failed = None;
    if times.challenge_fail >= 0.0 {
        let value = step(db, field::CHALLENGE_FAILED, &mut times.challenge_fail);
        failed = if value < 0.0 { None } else { Some(value) };
    }
    ResultInfo { success: if success < 0.0 { None } else { Some(success) }, failed }
}
