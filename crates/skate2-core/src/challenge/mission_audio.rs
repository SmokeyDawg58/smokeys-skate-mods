//! The challenge natives that only post to the audio system: `ChallengeSFX.PlayChallengeSFX` and
//! the `ChallengeSpeech` notifications. Each sends one Hermes message on
//! `AudioSystem::AudioEventBroadcaster` (queue `eEQAudio`); the audio thread's handlers play the
//! mission sound effects and contest/throw-down announcer lines.

use super::{
    attrib_lua::activity_of,
    binding::{get_bool, get_bool_or, get_int, get_int_or},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    natives::skater_character_from_index,
    system::{Output, World},
};

type R = LuaResult<Vec<Value>>;

/// A message on the audio broadcaster.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionAudio {
    /// cMsgLeaderBoardOverallScreen966045f2, PlayerOverallResultHandler825007b8.
    LeaderboardOverall { character: i32 },
    /// `cMsgPlayMissionSFX` (`0xE6170A35`, 32 bytes): `fSFX` (`eSk8MissionSFX`) and `fID`, the
    /// challenge's unique ID (`AudioSystem::PlayMissionSFX` `0x82504e70`).
    Sfx { sfx: i32, challenge: u64 },
    /// `cMsgContestPointsEarned` (`0x667DF414`, 28 bytes).
    ContestPointsEarned { player: i32, character: i32, points: i32 },
    /// `cMsgOneUpSuccessNotification` (`0x3896C078`, 24 bytes).
    OneUpSuccess { character: i32, turn_position: i32 },
    /// `cMsgOneUpFailureNotification` (`0x14D90F8E`, 28 bytes).
    OneUpFailure { character: i32, bailed: bool, turn_position: i32 },
    /// `cMsgStartThrowdownChallengeNotification` (`0x7B119E4A`, 28 bytes).
    StartThrowdownChallenge { characters: [i32; 3] },
    /// `cMsgStartTurnNotification` (`0x0D0720C2`, 28 bytes).
    StartTurn { character: i32, turn_position: i32, is_picking_object: bool },
}

/// `ChallengeSFX.PlayChallengeSFX(data, sfx)` (`0x82889fb8`): `ChallengeAudioManager::
/// PlayMissionSFX` (identical-code-folded at `0x82a35158`), `IAudioInterface::PlayMissionSFX`
/// (slot 14) with the activity's unique ID.
fn play_challenge_sfx(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let challenge = activity_of(lua, call.arg(0)).map_or(0, |activity| world.activities.activities[activity].key);
    let sfx = get_int(call, 1);
    world.output(Output::MissionAudio(MissionAudio::Sfx { sfx, challenge }));
    Ok(Vec::new())
}

/// `ChallengeSpeech.SendContestPointsEarnedMsg(player = -1, points = 0)` (`0x828953f0`): the
/// player, its character (`GetSkaterCharacterFromIndex` `0x82888ed8`) and the points.
fn send_contest_points_earned(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int_or(call, 0, -1);
    let points = get_int_or(call, 1, 0);
    let character = skater_character_from_index(world, player);
    world.output(Output::MissionAudio(MissionAudio::ContestPointsEarned { player, character, points }));
    Ok(Vec::new())
}

/// `ChallengeSpeech.OneUpSuccessNotification(player, turnPosition)` (`0x82895298`).
fn one_up_success(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let character = skater_character_from_index(world, get_int(call, 0));
    let turn_position = get_int(call, 1);
    world.output(Output::MissionAudio(MissionAudio::OneUpSuccess { character, turn_position }));
    Ok(Vec::new())
}

/// `ChallengeSpeech.OneUpFailureNotification(player, bailed, turnPosition)` (`0x82895338`).
fn one_up_failure(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let bailed = get_bool(call, 1);
    let turn_position = get_int(call, 2);
    let character = skater_character_from_index(world, player);
    world.output(Output::MissionAudio(MissionAudio::OneUpFailure { character, bailed, turn_position }));
    Ok(Vec::new())
}

/// `ChallengeSpeech.StartThrowdownChallenge(first, second = -1, third = -1)` (`0x82895108`):
/// each skater's character.
fn start_throwdown_challenge(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let players = [get_int(call, 0), get_int_or(call, 1, -1), get_int_or(call, 2, -1)];
    let characters = players.map(|player| skater_character_from_index(world, player));
    world.output(Output::MissionAudio(MissionAudio::StartThrowdownChallenge { characters }));
    Ok(Vec::new())
}

/// `ChallengeSpeech.StartTurnNotification(player, turnPosition, isPickingObject = false)`
/// (`0x828951e0`).
fn start_turn_notification(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let turn_position = get_int(call, 1);
    let is_picking_object = get_bool_or(call, 2, false);
    let character = skater_character_from_index(world, player);
    world.output(Output::MissionAudio(MissionAudio::StartTurn { character, turn_position, is_picking_object }));
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("ChallengeSFX", "PlayChallengeSFX", play_challenge_sfx),
        ("ChallengeSpeech", "SendContestPointsEarnedMsg", send_contest_points_earned),
        ("ChallengeSpeech", "OneUpSuccessNotification", one_up_success),
        ("ChallengeSpeech", "OneUpFailureNotification", one_up_failure),
        ("ChallengeSpeech", "StartThrowdownChallenge", start_throwdown_challenge),
        ("ChallengeSpeech", "StartTurnNotification", start_turn_notification),
    ]
}
