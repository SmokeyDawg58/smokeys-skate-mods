//! `Sk8::Challenge::cChallengeDMOManager`: a challenge's operations on the dynamic objects of
//! its trigger volumes (`tDMOResetInfo`), the locators of its hulls' DMO groups
//! (`ChallengeDMOGroups`, recorded as the hulls stream in), its dynamic object census range and,
//! as an `IDMOInterrogater`, the trigger proxies through which objectives ask where a DMO is.
//!
//! The locators and dynamic objects are the session's: the manager's calls go out as
//! [`LivingWorldOp`]s in the character manager's queue.

use super::{
    attrib_lua::activity_of,
    characters::{self, LivingWorldOp},
    keys::class,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::{Event, Field, Message, World},
};
use crate::living_world_grind_services::dmo_locators::LocatorData;
use std::collections::BTreeMap;

/// The challenge attributes the manager reads.
mod key {
    /// `challenges.DMOCensusRange`: a `livingworld_census_ranges` `RefSpec`.
    pub(super) const DMO_CENSUS_RANGE: u64 = 0x0958_4b61_3b77_4df4;
    /// The challenge's `tDMOResetInfo` array: `Type`, then `Volume` at +0x10.
    pub(super) const DMO_RESET_INFOS: u64 = 0x915c_7bf8_b06d_5a58;
    /// `challenges.DmoTreatAsStatic`.
    pub(super) const DMO_TREAT_AS_STATIC: u64 = 0x198a_0e27_6e56_a9d6;
    /// `challenges.DmoRedoOperationsOnRestart`.
    pub(super) const DMO_REDO_OPERATIONS_ON_RESTART: u64 = 0x5368_8086_474e_81eb;
}

/// `eDMOResetType`.
pub(crate) mod reset_type {
    /// `eDMOResetType_Reset`: the volume's dynamic objects culled, its locators reset.
    pub(crate) const RESET: u32 = 0;
    /// `eDMOResetType_Clear`: the volume's dynamic objects culled, its locators disabled.
    pub(crate) const CLEAR: u32 = 1;
    /// `eDMOResetType_Save`: the volume's locators saved, then as Clear.
    pub(crate) const SAVE: u32 = 2;
}

#[derive(Debug, Default)]
pub struct DmoManager {
    /// `Activated`: `GameLoad` (`0x825b03f8`) sets it for game types 4 and 16.
    pub activated: bool,
    /// `GameTypeFlags`.
    game_type_flags: u32,
    /// `NextDMOProxyID`.
    next_proxy: i32,
    /// `DMOProxyMap`: proxy id to locator id.
    proxies: BTreeMap<i32, u64>,
    /// `TriggerBufferIDMap` with its `SerializedDMOBufferMap` buffers: per trigger instance
    /// GUID, the locators an `eDMOResetType_Save` serialized (`SerializeDMOs` 0x825c2210).
    saved: BTreeMap<u64, Vec<LocatorData>>,
    /// `IssuedDMOOperations`: the accepted challenge's operations ran.
    issued_dmo_operations: bool,
    /// `WaitingForDMOSpawn`: the updates the sign-up still waits after them.
    waiting_for_dmo_spawn: u32,
    /// `AddChallengeGroupsNextFrame`: the restarted activity whose groups `Update` places.
    add_challenge_groups_next_frame: Option<usize>,
}

impl DmoManager {
    #[must_use]
    pub fn new(activated: bool) -> Self {
        Self { activated, ..Self::default() }
    }

    /// `RunDMOOperations`' save of a trigger's locators: a trigger already holding a buffer keeps
    /// it and the new one is released (`ReleaseSerializeBuffer` 0x825b73d0).
    pub(crate) fn save_locators(&mut self, trigger: u64, locators: Vec<LocatorData>) {
        self.saved.entry(trigger).or_insert(locators);
    }

    /// `UnwindDMOOperations`' buffer of a trigger, released and forgotten.
    pub(crate) fn take_saved_locators(&mut self, trigger: u64) -> Option<Vec<LocatorData>> {
        self.saved.remove(&trigger)
    }
}

/// `GameLoad` (`0x825b03f8`): career (4) and Party Play (16) games activate the manager over
/// the living world's managers; every game keeps its type flags.
pub(crate) fn game_load(world: &mut World, game_type_flags: u32) {
    let m = &mut world.dmos;
    if game_type_flags == 4 || game_type_flags == 16 {
        m.activated = true;
    }
    m.game_type_flags = game_type_flags;
}

/// The challenge's `tDMOResetInfo`s as (`Type`, `Volume`); a missing element reads the default
/// data area's zeros.
fn reset_infos(world: &World, key: u64) -> Vec<(u32, u64)> {
    let Some(infos) = world.db.attribute(class::CHALLENGES, key, key::DMO_RESET_INFOS) else {
        return Vec::new();
    };
    (0..infos.len())
        .map(|index| {
            let bytes = infos
                .data(index)
                .and_then(|at| world.db.bytes(at, 24))
                .map_or_else(|| vec![0; 24], <[u8]>::to_vec);
            (
                u32::from_be_bytes(bytes[0..4].try_into().expect("four bytes")),
                u64::from_be_bytes(bytes[16..24].try_into().expect("eight bytes")),
            )
        })
        .collect()
}

/// `RunDMOOperations` (`0x825c3368`): an activated manager runs each `tDMOResetInfo` on its
/// volume's trigger instance, then with `add_challenge_dmos` places the `ChallengeDMOGroups`
/// locators (`AddStreamedLocatorData(group, 0, false, false, DmoTreatAsStatic)`);
/// `cMsgDMOOperationsComplete` with the activity's unique ID follows either way.
pub(crate) fn run_dmo_operations(world: &mut World, activity: usize, add_challenge_dmos: bool) {
    let key = world.activities.activities[activity].key;
    if world.dmos.activated {
        for (kind, volume) in reset_infos(world, key) {
            characters::op(world, LivingWorldOp::DmoReset { kind, volume });
        }
        if add_challenge_dmos {
            let treat_as_static = treat_as_static(world, key);
            characters::op(world, LivingWorldOp::AddChallengeDmos { treat_as_static });
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "(float)_u64tod")]
    let activity_key = key as f64 as f32;
    world.send(Event::Challenge(Message::new(
        "cMsgDMOOperationsComplete",
        vec![("ActivityKey", Field::Number(activity_key))],
    )));
}

/// `RunMainActivity` (`0x825c3a00`) of an activated manager: Party Play (16) runs the DMO
/// operations and places the challenge DMO groups (a career challenge's run when its sign-up is
/// accepted, [`on_challenge_accept`]); then the challenge's `DMOCensusRange`
/// becomes the dynamic object census range (`SetDynamicObjectsCensusRangeKey`, zero when
/// absent).
pub(crate) fn run_main_activity(world: &mut World, activity: usize) {
    if !world.dmos.activated {
        return;
    }
    if world.dmos.game_type_flags == 16 {
        run_dmo_operations(world, activity, true);
    }
    let key = world.activities.activities[activity].key;
    let range = world
        .db
        .attribute(class::CHALLENGES, key, key::DMO_CENSUS_RANGE)
        .and_then(|a| a.ref_spec(0))
        .map_or(0, |(_, collection)| collection);
    characters::op(world, LivingWorldOp::DynamicObjectsCensusRange(Some(range)));
}

/// `CompleteChallenge` (`0x825bbd70`) of an activated manager: the main challenge's completion
/// puts the census range back (`ResetDynamicObjectsCensusRangeKey`), then
/// `UnwindDMOOperations`.
pub(crate) fn complete_challenge(world: &mut World, activity: usize, is_main: bool) {
    if !world.dmos.activated {
        return;
    }
    if is_main {
        characters::op(world, LivingWorldOp::DynamicObjectsCensusRange(None));
    }
    unwind_dmo_operations(world, activity);
}

/// `ShutdownActivity` (`0x825bbd68`): `UnwindDMOOperations`.
pub(crate) fn shutdown_activity(world: &mut World, activity: usize) {
    unwind_dmo_operations(world, activity);
}

/// The challenge's `DmoTreatAsStatic` (`DefaultDataArea`: false when absent).
fn treat_as_static(world: &World, key: u64) -> bool {
    world
        .db
        .attribute(class::CHALLENGES, key, key::DMO_TREAT_AS_STATIC)
        .and_then(|a| a.bool(0))
        .unwrap_or(false)
}

/// `OnChallengeAccept` (`0x825c3728`), the manager's low-priority sign-up answer to an accepted
/// challenge: an activated manager whose challenge hulls are in (`AreChallengeHullsLoaded`
/// `0x82e0fa60`) runs its operations with the challenge DMO groups and waits three updates for
/// the objects to spawn.
pub(crate) fn on_challenge_accept(world: &mut World, activity: usize) {
    if !world.dmos.activated {
        return;
    }
    world.dmos.issued_dmo_operations = false;
    if super::assets::are_hulls_loaded(world, activity) {
        run_dmo_operations(world, activity, true);
        world.dmos.issued_dmo_operations = true;
        world.dmos.waiting_for_dmo_spawn = 3;
    }
}

/// `IsChallengeAcceptOperationDone` (`0x825c37a8`): done when not activated, or once the issued
/// operations' wait ran out (one update less per call); operations not yet issued run as soon as
/// the hulls are in.
pub(crate) fn is_challenge_accept_operation_done(world: &mut World, activity: usize) -> bool {
    let m = &mut world.dmos;
    if !m.activated {
        return true;
    }
    if m.issued_dmo_operations {
        if m.waiting_for_dmo_spawn == 0 {
            return true;
        }
        m.waiting_for_dmo_spawn -= 1;
    } else if super::assets::are_hulls_loaded(world, activity) {
        run_dmo_operations(world, activity, true);
        world.dmos.waiting_for_dmo_spawn = 3;
        world.dmos.issued_dmo_operations = true;
    }
    false
}

/// `RestartChallenge` (`0x825b5c18`): a challenge with `DmoRedoOperationsOnRestart` redoes its
/// operations.
pub(crate) fn restart_challenge(world: &mut World, activity: usize) {
    let key = world.activities.activities[activity].key;
    let redo = world
        .db
        .attribute(class::CHALLENGES, key, key::DMO_REDO_OPERATIONS_ON_RESTART)
        .and_then(|a| a.bool(0))
        .unwrap_or(false);
    if redo {
        redo_dmo_operations_on_restart(world, activity);
    }
}

/// `RedoDMOOperationsOnRestart` (`0x825b04f0`) of an activated manager: each Reset culls and resets
/// its volume again, each Clear and Save culls and disables it (nothing is saved again); then
/// `AddChallengeGroupsNextFrame` (set by the native when it holds groups, which the port's terrain
/// keeps) has the next `Update` place them.
fn redo_dmo_operations_on_restart(world: &mut World, activity: usize) {
    if !world.dmos.activated {
        return;
    }
    let key = world.activities.activities[activity].key;
    for (kind, volume) in reset_infos(world, key) {
        let kind = match kind {
            reset_type::RESET => reset_type::RESET,
            reset_type::CLEAR | reset_type::SAVE => reset_type::CLEAR,
            _ => continue,
        };
        characters::op(world, LivingWorldOp::DmoReset { kind, volume });
    }
    world.dmos.add_challenge_groups_next_frame = Some(activity);
}

/// `Update` (`0x825b0700`): an activated manager with `AddChallengeGroupsNextFrame` places the
/// challenge DMO groups (`AddStreamedLocatorData(group, 0, false, false, DmoTreatAsStatic)`).
pub(crate) fn update(world: &mut World) {
    if !world.dmos.activated {
        return;
    }
    let Some(activity) = world.dmos.add_challenge_groups_next_frame.take() else {
        return;
    };
    let treat_as_static = treat_as_static(world, world.activities.activities[activity].key);
    characters::op(world, LivingWorldOp::AddChallengeDmos { treat_as_static });
}

/// `UnwindDMOOperations` (`0x825ba848`) of an activated manager: each Clear enables its volume's
/// locators again and each Save restores what it saved (`RestoreFromSavedState(-1, buffer)`).
fn unwind_dmo_operations(world: &mut World, activity: usize) {
    if !world.dmos.activated {
        return;
    }
    let key = world.activities.activities[activity].key;
    for (kind, volume) in reset_infos(world, key) {
        if kind == reset_type::CLEAR || kind == reset_type::SAVE {
            characters::op(world, LivingWorldOp::UnwindDmoReset { kind, volume });
        }
    }
}

/// `AddDMOTriggerProxy` (`0x825c2498`): -1 when inactive or the Lua id names
/// no locator.
pub fn add_trigger_proxy(world: &mut World, lua_id: f32) -> i32 {
    if !world.dmos.activated {
        return -1;
    }
    let Some(locator) = world.host.dmo_locator_from_lua_id(lua_id) else { return -1 };
    let id = world.dmos.next_proxy;
    world.dmos.next_proxy = id.wrapping_add(1);
    world.dmos.proxies.insert(id, locator);
    id
}

/// `ReleaseDMOTriggerProxy` (`0x825ba9e8`).
pub fn release_trigger_proxy(world: &mut World, proxy: i32) {
    world.dmos.proxies.remove(&proxy);
}

/// `IsDMOInVolume` (`0x825b7518`).
pub fn is_in_volume(world: &mut World, proxy: i32, volume_id: f32) -> bool {
    let Some(&locator) = world.dmos.proxies.get(&proxy) else { return false };
    world.host.is_dmo_in_volume(locator, volume_id)
}

/// `DMO.ResetChallengeDMOs()` (`0x82892ba8`): `ResetChallengeDMOs` (`0x825b06a0`) resets the
/// streamed locators of each `ChallengeDMOGroups` asset (`LWDynamicObjectLocatorMan::
/// ResetStreamedLocatorData`).
fn reset_challenge_dmos(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    characters::op(world, LivingWorldOp::ResetChallengeDmos);
    Ok(Vec::new())
}

/// `DMO.RunDMOOperations(data)` (`0x82892988`): `RunDMOOperations(activity, true)`.
/// `RunGlobalActivity` (`0x825c3aa8`): `RunDMOOperations(activity, false)`.
pub(crate) fn run_global_activity(world: &mut World, activity: usize) {
    run_dmo_operations(world, activity, false);
}

fn run_dmo_operations_native(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let activity = activity_of(lua, call.arg(0)).ok_or_else(|| lua.error("DMO.RunDMOOperations: expected a challenge's data".to_owned()))?;
    run_dmo_operations(world, activity, true);
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![("DMO", "ResetChallengeDMOs", reset_challenge_dmos), ("DMO", "RunDMOOperations", run_dmo_operations_native)]
}

#[cfg(test)]
#[path = "dmo_tests.rs"]
mod tests;
