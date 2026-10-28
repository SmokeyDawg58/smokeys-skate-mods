//! `Sk8::Challenge::cChallengeCompetitorsManager`'s dynamically chosen AI competitor: the
//! progression messages a pick-a-pro or throwdown challenge sends for the competitor the phone
//! list chose. StartPickAProChallenge8265de20 and StartThrowdownChallenge8265e168
//! also retain each selected profile and its root category for the AI manager.

use super::{
    binding::int,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::{Event, World},
};

type R = LuaResult<Vec<Value>>;

/// `challenges.IsAThrowdown` and `challenges.IsDynamicCompetitor`.
const IS_A_THROWDOWN: u64 = 0x4263_ed15_8a50_063e;
const IS_DYNAMIC_COMPETITOR: u64 = 0xf3a5_0805_8ae5_148d;

/// The manager's dynamic competitor state (constructor `0x825b47d0`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompetitorsManager {
    pub infos: [Option<CompetitorInfo>; 5],
    /// cSignPostSponsorMan::proFilmPro, assigned by SetProFilmPro825c3f70.
    pub pro_film_pro: Option<u64>,
    /// `mDynamicallyChosenAICompetitorCompletedMessage`: progression message keys, 0 for none.
    pub completed_messages: [u64; 5],
    /// `mDynamicallyChosenAICompetitorProBeatenMessage`.
    pub pro_beaten_message: u64,
    /// `mDynamicallyChosenAICompetitorAudioCharacter` (`eSk8Characters`), which only the phone list
    /// sets.
    pub audio_character: i32,
}

/// Original tCompetitorInfo (ProfileName pointer, intRootCategory).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompetitorInfo {
    pub profile_name: Option<String>,
    pub root_category: i32,
}

/// `ResetDynamicallyChosenAICompetitorInfo` (`0x825af2d8`): no competitor and no messages; the
/// debug competitor (`kUseDebugCompetitorData`) is off.
pub(crate) fn reset(world: &mut World) {
    let competitors = &mut world.competitors;
    competitors.infos = Default::default();
    competitors.completed_messages = [0; 5];
    competitors.pro_beaten_message = 0;
}

/// `ShutdownActivity` (`0x825af548`): a throwdown or dynamic-competitor challenge's end resets the
/// competitor.
pub(crate) fn shutdown_activity(world: &mut World, activity: usize) {
    let key = world.activities.activities[activity].key;
    let flag = |field| world.db.attribute(super::keys::class::CHALLENGES, key, field).and_then(|a| a.bool(0)).unwrap_or(false);
    if flag(IS_A_THROWDOWN) || flag(IS_DYNAMIC_COMPETITOR) {
        reset(world);
    }
}

/// `ChallengeProgression.SendDynamicallyChosenAICompetitorCompletedMessages()` (`0x828918e0`):
/// `SendDynamicallyChosenAICompetitorCompletedMessages` (`0x825b9ce0`), `cMsgMessageSent` of each
/// set message.
fn send_completed_messages(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    for message in world.competitors.completed_messages {
        if message != 0 {
            world.send(Event::MessageSent { message });
        }
    }
    Ok(Vec::new())
}

/// `ChallengeProgression.SendDynamicallyChosenAIProBeatenMessage()` (`0x82891990`):
/// `SendDynamicallyChosenAICompetitorProBeatenMessage` (`0x825b9da0`), the set message once.
fn send_pro_beaten_message(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let message = std::mem::take(&mut world.competitors.pro_beaten_message);
    if message != 0 {
        world.send(Event::MessageSent { message });
    }
    Ok(Vec::new())
}

/// `ChallengeProgression.GetPickAProCompetitorAudioID()` (`0x82891f48`): the competitor's audio
/// character.
fn get_pick_a_pro_competitor_audio_id(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.competitors.audio_character)])
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("ChallengeProgression", "SendDynamicallyChosenAICompetitorCompletedMessages", send_completed_messages),
        ("ChallengeProgression", "SendDynamicallyChosenAIProBeatenMessage", send_pro_beaten_message),
        ("ChallengeProgression", "GetPickAProCompetitorAudioID", get_pick_a_pro_competitor_audio_id),
    ]
}
