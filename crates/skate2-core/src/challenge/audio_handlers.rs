//! `Sk8::Audio::AudioSystem`'s handlers on the challenge broadcaster (`AddDelayedMessageHandlers`
//! `0x82508858`: `cMsgTrickComplete`, `cMsgCalloutTriggered`, `cMsgCalloutComplete`,
//! `cMsgChallengeFailed`, `cMsgAirCompleted`, `cMsgAbandonChallenge` and
//! `cMsgAudioSequenceComplete`): the port has the failed and the abandoned challenge's. Also what
//! `TriggerMissionMusicImp` reads of a challenge.

use super::{
    keys::{class, hash32},
    runtime::field,
    signup,
    system::{Event, Output, World},
};

/// `challenge_global_data.ChallengeType`.
const CHALLENGE_TYPE: u64 = 0x02b8_28cc_db47_85e4;
/// `challenges.AudioRaceTuning`, an `aud_race` `RefSpec`.
const AUDIO_RACE_TUNING: u64 = 0xe6ee_8734_584b_1523;

/// The audio system's handlers of a challenge-broadcaster message.
pub(crate) fn receive(world: &mut World, event: &Event) {
    match event {
        Event::ChallengeFailed { activity, result } => challenge_failed(world, *activity, *result),
        // QuitChallenge (0x824ff478).
        Event::Challenge(message) if message.kind == hash32("cMsgAbandonChallenge") => world.output(Output::QuitChallengeMusic),
        _ => {}
    }
}

/// What `AudioSystem::TriggerMissionMusicImp` (`0x824fee80`) reads of the challenge
/// (`Attrib::Gen::challenges` of the key): its global data's `ChallengeType`
/// (`cActivity::GetGlobalDataFromChallenge` `0x82a8f580`; 19 without a key) and the collection of
/// its `AudioRaceTuning`.
#[must_use]
pub fn mission_music(world: &World, challenge: u64) -> (i32, u64) {
    if challenge == 0 {
        return (19, 0);
    }
    let challenge_type = world
        .db
        .attribute(class::CHALLENGES, challenge, field::GLOBAL_DATA)
        .and_then(|attribute| attribute.ref_spec(0))
        .and_then(|(c, k)| world.db.attribute(c, k, CHALLENGE_TYPE))
        .and_then(|attribute| attribute.i32(0))
        .unwrap_or(0);
    let race_tuning = world.db.attribute(class::CHALLENGES, challenge, AUDIO_RACE_TUNING).and_then(|attribute| attribute.ref_spec(0)).map_or(0, |(_, k)| k);
    (challenge_type, race_tuning)
}

/// `ChallengeFailedHandler` (`0x82505740`): the failed challenge's `ChallengeType` (its
/// `challenge_global_data`, through `cActivityManager::GetChallenge`) picks the cameraman's line,
/// sent as `cMsgPlayCameraManCsisEvent` with unset parameters: a lost Demo (2), Best Trick (8) or
/// Jam (15) `T_405_BstLose`, an abandoned Film (7), Anywhere Film (39), Spot Film (40), Friend
/// Film (41), Pick-a-Pro Film (46) or Team Film (48) `T_801_FilmChalQuit`.
fn challenge_failed(world: &mut World, activity: u64, result: i32) {
    let Some(challenge) = world.activities.find(activity) else { return };
    let function = match (result, signup::challenge_type(world, challenge)) {
        (1, 2 | 8 | 15) => "T_405_BstLose",
        (2, 7 | 39 | 40 | 41 | 46 | 48) => "T_801_FilmChalQuit",
        _ => return,
    };
    world.output(Output::PlayCameraManCsisEvent { function });
}
