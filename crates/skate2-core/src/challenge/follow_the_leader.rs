//! `Sk8::Challenge::ChallengeFTLManager` (ctor `0x825b35e8`): the distance between a Follow
//! challenge's two skaters, measured each challenge update while in use, and the warning info
//! its script stores and reads back. The distance callbacks the script registers belong to the
//! AI skater manager (`AISkaterManager::RegisterSkaterDistanceCallbackMessage`).

use super::{
    attrib_lua::attrib_location,
    binding::{get_int, int},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::World,
};
use crate::ai::vmx;

type R = LuaResult<Vec<Value>>;

/// `tFollowTheLeaderWarningInfo` (`0x24` bytes).
mod warning_info {
    pub const MIN_DISTANCE: u32 = 0x0;
    pub const MAX_DISTANCE: u32 = 0x4;
    /// `warningSpeechInfo` (`tFollowMeSpeechInfo`): character, speech ID (short), followee.
    pub const WARNING_CHARACTER: u32 = 0x8;
    pub const WARNING_SPEECH_ID: u32 = 0xc;
    pub const WARNING_FOLLOWEE: u32 = 0x10;
    /// `recoverySpeechInfo` (`tSpeechInfo`): character, speech ID (short).
    pub const RECOVERY_CHARACTER: u32 = 0x14;
    pub const RECOVERY_SPEECH_ID: u32 = 0x18;
    pub const TIME_BEFORE_WARNING_PLAYED: u32 = 0x1c;
    pub const REPEAT_TIME: u32 = 0x20;
}

/// `ChallengeFTLManager` (`0x40` bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FtlManager {
    /// `FollowTheLeaderData` (`tFollowTheLeaderStats`).
    pub follower: i32,
    pub followee: i32,
    pub in_use: bool,
    pub distance_between: f32,
    pub min_distance: i32,
    pub max_distance: i32,
    pub warning_speech_id: i32,
    pub warning_character: i32,
    pub warning_followee: i32,
    pub recovery_speech_id: i32,
    pub recovery_character: i32,
    pub time_before_warning_played: i32,
    pub repeat_time: i32,
}

impl Default for FtlManager {
    fn default() -> Self {
        Self {
            follower: -1,
            followee: -1,
            in_use: false,
            distance_between: 0.0,
            min_distance: 1,
            max_distance: 1,
            warning_speech_id: 0,
            warning_character: 0,
            warning_followee: 0,
            recovery_speech_id: 0,
            recovery_character: 0,
            time_before_warning_played: 0,
            repeat_time: 0,
        }
    }
}

impl FtlManager {
    /// `StopFollowTheLeaderStats` (`0x825ae620`).
    fn stop(&mut self) {
        self.follower = -1;
        self.in_use = false;
        self.followee = -1;
        self.distance_between = 0.0;
    }
}

/// `GetDistanceBetweenSkaters` (`0x825ae7b0`): the distance between the base positions
/// (`cTriggerTester::GetBasePosition`) of the skater manager's skaters at the two indices; -1
/// without both.
fn distance_between_skaters(world: &World, first: i32, second: i32) -> f32 {
    let base = |index: i32| usize::try_from(index).ok().and_then(|index| world.skaters.get(index)).map(|frame| vmx::v3(frame.tester_base));
    match (base(first), base(second)) {
        (Some(first), Some(second)) => vmx::length3(vmx::sub(second, first)),
        _ => -1.0,
    }
}

/// `ChallengeFTLManager::Update` (`0x825b3630`).
pub(crate) fn update(world: &mut World) {
    if world.follow_the_leader.in_use {
        let ftl = world.follow_the_leader;
        world.follow_the_leader.distance_between = distance_between_skaters(world, ftl.followee, ftl.follower);
    }
}

/// `ChallengeFTLManager::CompleteChallenge` (`0x825ae648`).
pub(crate) fn complete_challenge(world: &mut World, is_main: bool) {
    if is_main {
        world.follow_the_leader.stop();
    }
}

fn data_u32(world: &World, info: super::attrib_db::Location, offset: u32) -> u32 {
    world.db.bytes(info.offset_by(offset), 4).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

#[expect(clippy::cast_possible_wrap, reason = "the native int fields")]
fn data_i32(world: &World, info: super::attrib_db::Location, offset: u32) -> i32 {
    data_u32(world, info, offset) as i32
}

fn data_i16(world: &World, info: super::attrib_db::Location, offset: u32) -> i32 {
    world.db.bytes(info.offset_by(offset), 2).map_or(0, |b| i32::from(i16::from_be_bytes([b[0], b[1]])))
}

/// `FollowTheLeader.SetActorIDs(follower)` (`0x828934e8`): `SetupFollowParticipants`
/// (`0x825ae688`) with the local skater as the followee.
fn set_actor_ids(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let follower = get_int(call, 0);
    world.follow_the_leader.followee = world.local_player;
    world.follow_the_leader.follower = follower;
    Ok(Vec::new())
}

/// `FollowTheLeader.StartDistanceUpdates()` (`0x828935d8`): `StartFollowTheLeaderStats`.
fn start_distance_updates(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.follow_the_leader.in_use = true;
    Ok(Vec::new())
}

/// `FollowTheLeader.StopDistanceUpdates()` (`0x82893658`).
fn stop_distance_updates(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.follow_the_leader.stop();
    Ok(Vec::new())
}

/// `FollowTheLeader.GetDistance()` (`0x828936d8`).
fn get_distance(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(world.follow_the_leader.distance_between)])
}

/// `FollowTheLeader.ResetDistance()` (`0x82893d28`): `ResetDistanceBetween`.
fn reset_distance(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.follow_the_leader.distance_between = 0.0;
    Ok(Vec::new())
}

/// `FollowTheLeader.SetWarningInfo(tFollowTheLeaderWarningInfo)` (`0x82893768`).
fn set_warning_info(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(info) = attrib_location(&world.pointers, call.arg(0)) else { return Ok(Vec::new()) };
    world.follow_the_leader = FtlManager {
        min_distance: data_i32(world, info, warning_info::MIN_DISTANCE),
        max_distance: data_i32(world, info, warning_info::MAX_DISTANCE),
        warning_character: data_i32(world, info, warning_info::WARNING_CHARACTER),
        warning_followee: data_i32(world, info, warning_info::WARNING_FOLLOWEE),
        warning_speech_id: data_i16(world, info, warning_info::WARNING_SPEECH_ID),
        recovery_character: data_i32(world, info, warning_info::RECOVERY_CHARACTER),
        recovery_speech_id: data_i16(world, info, warning_info::RECOVERY_SPEECH_ID),
        time_before_warning_played: data_i32(world, info, warning_info::TIME_BEFORE_WARNING_PLAYED),
        repeat_time: data_i32(world, info, warning_info::REPEAT_TIME),
        ..world.follow_the_leader
    };
    Ok(Vec::new())
}

/// `FollowTheLeader.GetMinDistance()` (`0x82893858`).
fn get_min_distance(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.follow_the_leader.min_distance)])
}

/// `FollowTheLeader.GetMaxDistance()` (`0x828938e0`).
fn get_max_distance(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.follow_the_leader.max_distance)])
}

/// `FollowTheLeader.GetWarningSpeechID()` (`0x82893970`).
fn get_warning_speech_id(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.follow_the_leader.warning_speech_id)])
}

/// `FollowTheLeader.GetRecoverySpeechID()` (`0x828939f8`).
fn get_recovery_speech_id(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.follow_the_leader.recovery_speech_id)])
}

/// `FollowTheLeader.GetTimeBeforeWarningPlayed()` (`0x82893a80`).
fn get_time_before_warning_played(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.follow_the_leader.time_before_warning_played)])
}

/// `FollowTheLeader.GetRepeatTime()` (`0x82893b08`).
fn get_repeat_time(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.follow_the_leader.repeat_time)])
}

/// `FollowTheLeader.GetRecoveryCharacter()` (`0x82893b90`).
fn get_recovery_character(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.follow_the_leader.recovery_character)])
}

/// `FollowTheLeader.GetWarningCharacter()` (`0x82893c18`).
fn get_warning_character(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.follow_the_leader.warning_character)])
}

/// `FollowTheLeader.GetWarningFollowee()` (`0x82893ca0`).
fn get_warning_followee(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.follow_the_leader.warning_followee)])
}

/// `FollowTheLeader.RegisterDistanceCallback(skater, distance)` (`0x8288e388`): the AI skater
/// manager's callback slot for the skater at the (whole) distance, -1 when none is free.
fn register_distance_callback(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let skater = get_int(call, 0);
    #[expect(clippy::cast_precision_loss, reason = "the native (float) of GetInt")]
    let distance = get_int(call, 1) as f32;
    let slot = world.ai_link.as_ref().and_then(|link| link.manager.lock().ok()).map_or(-1, |mut manager| {
        manager.register_skater_distance_callback(skater, distance)
    });
    Ok(vec![int(slot)])
}

/// `FollowTheLeader.UnregisterDistanceCallback(id)` (`0x8288e418`).
fn unregister_distance_callback(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let id = get_int(call, 0);
    if let Some(mut manager) = world.ai_link.as_ref().and_then(|link| link.manager.lock().ok()) {
        manager.unregister_skater_distance_callback(u32::from_ne_bytes(id.to_ne_bytes()));
    }
    Ok(Vec::new())
}

pub fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("FollowTheLeader", "RegisterDistanceCallback", register_distance_callback),
        ("FollowTheLeader", "UnregisterDistanceCallback", unregister_distance_callback),
        ("FollowTheLeader", "SetActorIDs", set_actor_ids),
        ("FollowTheLeader", "StartDistanceUpdates", start_distance_updates),
        ("FollowTheLeader", "StopDistanceUpdates", stop_distance_updates),
        ("FollowTheLeader", "GetDistance", get_distance),
        ("FollowTheLeader", "SetWarningInfo", set_warning_info),
        ("FollowTheLeader", "GetMinDistance", get_min_distance),
        ("FollowTheLeader", "GetMaxDistance", get_max_distance),
        ("FollowTheLeader", "GetWarningSpeechID", get_warning_speech_id),
        ("FollowTheLeader", "GetRecoverySpeechID", get_recovery_speech_id),
        ("FollowTheLeader", "GetTimeBeforeWarningPlayed", get_time_before_warning_played),
        ("FollowTheLeader", "GetRepeatTime", get_repeat_time),
        ("FollowTheLeader", "GetRecoveryCharacter", get_recovery_character),
        ("FollowTheLeader", "GetWarningCharacter", get_warning_character),
        ("FollowTheLeader", "GetWarningFollowee", get_warning_followee),
        ("FollowTheLeader", "ResetDistance", reset_distance),
    ]
}
