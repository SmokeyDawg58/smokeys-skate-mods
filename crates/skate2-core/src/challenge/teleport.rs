//! `Sk8::Challenge::cChallengeTeleportManager`: challenge teleports. A
//! teleport with a transition, or one asked for while a transition runs, is
//! queued behind the fade: the fade-out runs for half the transition, the
//! queued players move, 30 ticks later the fade-in runs for the other half,
//! and its end sends `cMsgTeleportTransitionFinished`. Without a transition
//! the player moves at once and nothing is sent.

use super::{
    effects, keys,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::{Event, Field, Message, Output, World},
};

/// `tTeleportInfo`: `mCachedPlayerID`, `mCachedLocationID` (kept as the
/// name; the native caches `GetHashValue32` of it), `mCachedPosID`,
/// `mCachedRepositionMarker`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TeleportInfo {
    player: i32,
    location: String,
    pos_id: i32,
    reposition_marker: bool,
}

/// The manager's state (`0x1fc` bytes).
#[derive(Debug)]
pub struct TeleportManager {
    pub game_type_flags: u32,
    pub fade_in_progress: bool,
    /// Half the transition time of the fade that runs.
    transition_time: f32,
    teleport_wait_ticks: i32,
    set_camera_player: i32,
    /// `mIsPickAPro`: the next challenge selection does not teleport.
    pub is_pick_a_pro: bool,
    teleports: Vec<TeleportInfo>,
    /// `mSetSessionMarkerOverrides` and `mUseSessionMarkerOverrides` (`+0x44`, `+0x94`): the
    /// activities that keep the local player from setting or using the session marker
    /// (`SessionMarker::VerifyChallengeOkToSet` `0x828ab5c0`, `VerifyChallengeOkToUse`
    /// `0x828ab6d0`).
    pub set_session_marker_overrides: Vec<usize>,
    pub use_session_marker_overrides: Vec<usize>,
}

impl Default for TeleportManager {
    /// The constructor (`0x82dabd28`).
    fn default() -> Self {
        Self {
            game_type_flags: 0,
            fade_in_progress: false,
            transition_time: 0.0,
            teleport_wait_ticks: 0,
            set_camera_player: -1,
            is_pick_a_pro: false,
            teleports: Vec::new(),
            set_session_marker_overrides: Vec::new(),
            use_session_marker_overrides: Vec::new(),
        }
    }
}

/// `GameLoad` (`0x82da4cf0`).
pub fn game_load(world: &mut World, flags: u32) {
    world.teleport.game_type_flags = flags;
    world.teleport.teleports.clear();
}

/// `TeleportPlayer(playerID, locationID, posID, repositionMarker,
/// transitionTime, setCamera)` (`0x82dad8e0`).
pub fn teleport_player(
    world: &mut World,
    player: i32,
    location: &str,
    pos_id: i32,
    reposition_marker: bool,
    transition_time: f32,
    set_camera: bool,
) {
    if transition_time > 0.001 || world.teleport.fade_in_progress {
        world.teleport.teleports.push(TeleportInfo { player, location: location.to_owned(), pos_id, reposition_marker });
        if set_camera {
            world.teleport.set_camera_player = player;
        }
        if !world.teleport.fade_in_progress {
            let half = transition_time * 0.5;
            world.teleport.transition_time = half;
            effects::start_effect(world, effects::FADE_OUT, half);
            world.teleport.fade_in_progress = true;
        }
    } else if transition_time < 0.001 {
        teleport_player_now(world, player, location, pos_id, reposition_marker);
        if set_camera {
            set_local_index(world, player);
            if world.teleport.game_type_flags == 16 {
                // The native sends the pending camera player here, not this one.
                let pending = world.teleport.set_camera_player;
                world.output(Output::SetPlayer { player: pending });
            }
        }
    }
}

/// `ISkaterManager::SetLocalIndex`.
fn set_local_index(world: &mut World, player: i32) {
    world.local_player = player;
    world.output(Output::SetLocalIndex { player });
}

/// `TeleportPlayerNow` (`0x82da6a00`): `ITeleportManager::TeleportPlayer` of
/// a skater that exists, after `cMsgSetPlayer` to it in Party Play (game type
/// 16); game type 8 teleports without one. With `repositionMarker` the local
/// player's session marker moves to the spawn point (`cMsgSetSessionMarker`).
fn teleport_player_now(world: &mut World, player: i32, location: &str, pos_id: i32, reposition_marker: bool) {
    let exists = super::skate::skater(world, player).is_some();
    if exists {
        if world.teleport.game_type_flags == 16 {
            world.output(Output::SetPlayer { player });
        }
    } else if world.teleport.game_type_flags != 8 {
        return;
    }
    // TeleportPlayerNow82da6adc calls INetworkGameQuery::GetLocalPlayerID,
    // not SkaterManager::GetLocalIndex (the AI followed by the camera).
    let set_session_marker = reposition_marker && exists && player == super::players::network_local_player(world);
    world.output(Output::TeleportPlayer { player, location: location.to_owned(), pos_id, set_session_marker });
}

/// The queued teleports, in order.
fn teleport_queued(world: &mut World) {
    for t in std::mem::take(&mut world.teleport.teleports) {
        teleport_player_now(world, t.player, &t.location, t.pos_id, t.reposition_marker);
    }
}

/// The manager's Hermes handler (`mEffectFinishedHandler`).
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "enum value")]
pub fn receive(world: &mut World, event: &Event) {
    let Event::Challenge(message) = event else { return };
    if message.kind != keys::hash32("cMsgEffectFinished") {
        return;
    }
    let effect = message.fields.as_ref().and_then(|f| f.iter().find(|(n, _)| *n == "Effect")).and_then(|(_, v)| match v {
        Field::Number(n) => Some(*n as usize),
        _ => None,
    });
    if let Some(effect) = effect {
        on_effect_finished(world, effect);
    }
}

/// `OnEffectFinished` (`0x82da6cd8`).
pub fn on_effect_finished(world: &mut World, effect: usize) {
    if !world.teleport.fade_in_progress {
        return;
    }
    if effect == effects::FADE_OUT {
        teleport_queued(world);
        let camera = world.teleport.set_camera_player;
        if camera > -1 {
            set_local_index(world, camera);
            if world.teleport.game_type_flags == 16 {
                world.output(Output::SetPlayer { player: camera });
            }
            world.teleport.set_camera_player = -1;
        }
        world.teleport.teleport_wait_ticks = 30;
    } else if effect == effects::FADE_IN {
        world.teleport.fade_in_progress = false;
        world.teleport.teleport_wait_ticks = 0;
        teleport_queued(world);
        world.send(Event::Challenge(Message::new("cMsgTeleportTransitionFinished", Vec::new())));
    }
}

/// `Update` (`0x82da9a58`): the fade-in starts 30 ticks after the teleport.
pub fn update(world: &mut World) {
    let t = &mut world.teleport;
    if t.fade_in_progress && t.teleport_wait_ticks > 0 {
        t.teleport_wait_ticks -= 1;
        if t.teleport_wait_ticks == 0 {
            let time = t.transition_time;
            effects::start_effect(world, effects::FADE_IN, time);
        }
    }
}

/// `AddSetSessionMarkerOverride` (`0x82daefa8`) or `AddUseSessionMarkerOverride` (`0x82daf060`):
/// the activity, once.
pub fn add_session_marker_override(world: &mut World, activity: usize, use_marker: bool) {
    let overrides = session_marker_overrides(world, use_marker);
    if !overrides.contains(&activity) {
        overrides.push(activity);
    }
}

/// `RemoveSetSessionMarkerOverride` (`0x82dabee0`) or `RemoveUseSessionMarkerOverride`
/// (`0x82dabf78`).
pub fn remove_session_marker_override(world: &mut World, activity: usize, use_marker: bool) {
    let overrides = session_marker_overrides(world, use_marker);
    if let Some(at) = overrides.iter().position(|&a| a == activity) {
        overrides.remove(at);
    }
}

fn session_marker_overrides(world: &mut World, use_marker: bool) -> &mut Vec<usize> {
    if use_marker { &mut world.teleport.use_session_marker_overrides } else { &mut world.teleport.set_session_marker_overrides }
}

/// `CompleteChallenge` (`0x82dadad8`): the activity's overrides go.
pub fn complete_challenge(world: &mut World, activity: usize) {
    remove_session_marker_override(world, activity, false);
    remove_session_marker_override(world, activity, true);
}

/// `ClearTeleports` (`0x82da9b10`): drop the queue; a running transition
/// ends at once.
pub fn clear_teleports(world: &mut World) {
    world.teleport.teleports.clear();
    if world.teleport.fade_in_progress {
        effects::reset_effect(world, effects::FADE_OUT);
        effects::reset_effect(world, effects::FADE_IN);
        world.send(Event::Challenge(Message::new("cMsgTeleportTransitionFinished", Vec::new())));
    }
    world.teleport.fade_in_progress = false;
    world.teleport.teleport_wait_ticks = 0;
}

/// `Teleport.TeleportLocalPlayerToSessionMarker()` (`0x82da57d0`): the manager's
/// `ITeleportManager::TeleportLocalPlayerToLastSessionMarker` (`mTeleportManager` vtable +0x1c,
/// `cTeleportManager` `0x828a9b18`).
fn teleport_local_player_to_session_marker(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    world.output(Output::TeleportLocalPlayerToSessionMarker);
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![("Teleport", "TeleportLocalPlayerToSessionMarker", teleport_local_player_to_session_marker)]
}
