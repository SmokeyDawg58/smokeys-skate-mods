//! `Sk8::Challenge::cChallengeCalloutManager` (`0xc8` bytes): contest callouts, the tricks a
//! contest calls out for every registered player to land first. A triggered callout gives each
//! player an objective group with the callout's objective; the first group to complete wins the
//! callout (`cMsgCalloutComplete`) and fails the others.
//!
//! The manager's trigger list (`TriggerList`, the timed and score-triggered callouts that
//! `Update` and `SequenceCompleteMsg` fire) is filled only by `Callouts.AddRoundTriggers`
//! (`SetTriggers` `0x82daf888`), which no shipped script calls: it stays empty, so `Update`,
//! `PossibleCallout` and the natives that look an activity's trigger data up change nothing.

use super::{
    attrib_lua::{ObjectKind, Pointee},
    binding::{get_bool, get_int, get_string},
    keys::hash32,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    objectives::{self, Status},
    system::{Event, Field, Message, World},
};
use std::collections::{BTreeMap, BTreeSet};

type R = LuaResult<Vec<Value>>;

/// The contest callout class `FindCollection` takes `TriggerCallout`'s key in.
const CONTEST_CALLOUTS: u64 = 0xae50_cbfe_7158_3652;
/// The callout's `RefSpec` to its `challenge_objective`.
const CALLOUT_OBJECTIVE: u64 = 0xfe03_bcff_45b9_a8fd;

/// `CalloutData::CalloutStatus`.
pub mod status {
    pub const NONE: i32 = 0;
    pub const TRIGGERED: i32 = 1;
    pub const COMPLETE: i32 = 2;
}

/// `CalloutData` (16 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalloutData {
    pub status: i32,
    pub callout: u64,
}

#[derive(Debug, Default)]
pub struct CalloutManager {
    /// `CalloutList`: each activity's callouts in insertion order.
    pub callouts: Vec<(usize, CalloutData)>,
    /// `HUDGroupList`: the HUD group `SetHUDGroup` found by name (its hash), `None` for a name
    /// `HUDManager2::GetGroup` did not find.
    pub hud_groups: BTreeMap<usize, Option<u32>>,
    /// `HUDPlayerList`: the player whose callout objectives the HUD group shows.
    pub hud_players: BTreeMap<usize, i32>,
    /// `GroupList`: the callout objective groups, by activity in insertion order.
    pub groups: Vec<(usize, u32)>,
    /// `PlayerList`: each activity's registered players (`fixed_set<int, 8>`).
    pub players: BTreeMap<usize, BTreeSet<i32>>,
}

impl CalloutManager {
    /// `equal_range(activity)` over a multimap kept in activity order with equal keys in
    /// insertion order.
    fn groups_of(&self, activity: usize) -> Vec<u32> {
        self.groups.iter().filter(|(a, _)| *a == activity).map(|(_, g)| *g).collect()
    }

    /// `multimap::insert`: after the last entry of the same activity.
    fn insert_group(&mut self, activity: usize, group: u32) {
        let at = self.groups.iter().position(|(a, _)| *a > activity).unwrap_or(self.groups.len());
        self.groups.insert(at, (activity, group));
    }

    fn insert_callout(&mut self, activity: usize, callout: CalloutData) {
        let at = self.callouts.iter().position(|(a, _)| *a > activity).unwrap_or(self.callouts.len());
        self.callouts.insert(at, (activity, callout));
    }
}

/// The activity of the running handler: the natives' closure upvalue.
fn closure_activity(lua: &Lua<World>, world: &World) -> LuaResult<usize> {
    world.closure.ok_or_else(|| lua.error("Callouts: no calling activity".to_owned()))
}

/// `ObjectiveHUDData` of the activity's HUD group, as `GetHUDData<ObjectiveHUDData>` finds it.
fn objective_hud(world: &mut World, activity: usize) -> Option<&mut super::hud::ObjectiveHud> {
    let hash = (*world.callouts.hud_groups.get(&activity)?)?;
    world.hud.groups.iter_mut().find(|g| g.hash == hash)?.objective.as_mut()
}

/// `DeleteObjectiveGroup` of each of the activity's callout groups, then their removal from the
/// group list (`ClearCallouts`, `TriggerCallout` and `ShutdownActivity` share it).
fn delete_groups(world: &mut World, activity: usize) {
    for group in world.callouts.groups_of(activity) {
        objectives::delete_group(world, group);
    }
    world.callouts.groups.retain(|(a, _)| *a != activity);
}

/// `ClearCallouts` (`0x82dac6a8`): the HUD group's objective data loses its group
/// (`MakeDirty`), the callout groups are deleted and the activity's callouts dropped.
pub fn clear_callouts(world: &mut World, activity: usize) {
    if let Some(data) = objective_hud(world, activity) {
        data.group = None;
        data.dirty = true;
    }
    delete_groups(world, activity);
    world.callouts.callouts.retain(|(a, _)| *a != activity);
}

/// `RegisterPlayer` (`0x82db52c0`).
pub fn register_player(world: &mut World, activity: usize, player: i32) {
    world.callouts.players.entry(activity).or_default().insert(player);
}

/// `SetHUDGroup` (`0x82dafaf8`): the named group (`HUDManager2::GetGroup`) for the activity, and
/// the player it shows, which a negative player clears.
pub fn set_hud_group(world: &mut World, activity: usize, name: &str, player: i32) {
    let hash = world.hud.group(name).map(|g| g.hash);
    world.callouts.hud_groups.insert(activity, hash);
    world.callouts.hud_players.remove(&activity);
    if player >= 0 {
        world.callouts.hud_players.insert(activity, player);
    }
}

/// `TriggerCallout` (`0x82dafba0`): the activity's callout groups replaced by one per registered
/// player (`AddObjectiveChallenge` with flow {AllAtStart, 1}, both masks all, active shown with
/// notifications, an existing group of the player reused) holding the callout's objective; the
/// HUD player's group shown on the activity's HUD group; every group reset; the callout's first
/// entry triggered; `cMsgCalloutTriggered`.
pub fn trigger_callout(world: &mut World, activity: usize, callout: u64) {
    delete_groups(world, activity);
    if let Some(players) = world.callouts.players.get(&activity).cloned() {
        let objective = world
            .db
            .attribute(CONTEST_CALLOUTS, callout, CALLOUT_OBJECTIVE)
            .and_then(|a| a.ref_spec(0))
            .map_or(0, |(_, key)| key);
        for player in players {
            let group = objectives::add_objective_challenge(world, activity, player, (0, 1), (u32::MAX, u32::MAX, true));
            objectives::add_objective(world, group, objective);
            world.callouts.insert_group(activity, group);
            if world.callouts.hud_players.get(&activity) == Some(&player) {
                let challenge = world.activities.activities[activity].key;
                if let Some(data) = objective_hud(world, activity) {
                    data.challenge = challenge;
                    data.group = Some(group);
                    data.dirty = true;
                }
            }
        }
    }
    for group in world.callouts.groups_of(activity) {
        objectives::reset_objectives(world, group, true);
    }
    if let Some((_, entry)) = world.callouts.callouts.iter_mut().find(|(a, c)| *a == activity && c.callout == callout) {
        entry.status = status::TRIGGERED;
    }
    let challenge = world.activities.activities[activity].key;
    world.send(Event::Challenge(Message::new(
        "cMsgCalloutTriggered",
        vec![("ChallengeKey", Field::Key(challenge)), ("CalloutKey", Field::Key(callout))],
    )));
}

/// `AllObjectiveCompleteMsg` (`0x82da7c88`): a callout group whose objectives are all complete
/// wins its activity's triggered callout for the first registered player the group is valid
/// for (`IsPlayerValid`: its active player), sends `cMsgCalloutComplete` and fails and stops
/// evaluating the activity's other callout groups.
#[expect(clippy::cast_precision_loss, reason = "lua_Number")]
fn all_objective_complete(world: &mut World, group: u32) {
    let all_complete = world
        .objectives
        .group_index(group)
        .is_some_and(|i| world.objectives.groups[i].objectives.iter().all(|o| o.state == Status::Complete));
    if !all_complete {
        return;
    }
    let Some(&(activity, _)) = world.callouts.groups.iter().find(|(_, g)| *g == group) else { return };
    let active_player = world.objectives.group_index(group).map_or(-1, |i| world.objectives.groups[i].player);
    let winner = world
        .callouts
        .players
        .get(&activity)
        .and_then(|players| players.iter().copied().find(|&player| player == active_player))
        .unwrap_or(-1);
    if let Some((_, entry)) = world.callouts.callouts.iter_mut().find(|(a, c)| *a == activity && c.status == status::TRIGGERED) {
        entry.status = status::COMPLETE;
        let callout = entry.callout;
        let challenge = world.activities.activities[activity].key;
        world.send(Event::Challenge(Message::new(
            "cMsgCalloutComplete",
            vec![("ChallengeKey", Field::Key(challenge)), ("CalloutKey", Field::Key(callout)), ("PlayerID", Field::Number(winner as f32))],
        )));
    }
    for other in world.callouts.groups_of(activity) {
        if other != group {
            let count = world.objectives.group_index(other).map_or(0, |i| world.objectives.groups[i].objectives.len());
            for index in 0..count {
                objectives::set_status(world, other, index, Status::Failed);
            }
            objectives::set_evaluation_status(world, other, false);
        }
    }
}

/// The manager's handlers on the challenge broadcaster (`Init` `0x82db19b8`):
/// `cMsgAllObjectiveComplete`. Its `cMsgSequenceComplete` handler only feeds the empty trigger
/// list.
pub fn receive(world: &mut World, event: &Event) {
    let Event::Challenge(message) = event else { return };
    if message.kind != hash32("cMsgAllObjectiveComplete") {
        return;
    }
    let Some(fields) = &message.fields else { return };
    let Some(Field::Pointer(handle)) = fields.iter().find(|(name, _)| *name == "ObjectiveGroup").map(|(_, f)| f) else { return };
    if let Some(Pointee::Object { kind: ObjectKind::ObjectiveGroup, id }) = world.pointers.get(*handle) {
        all_objective_complete(world, id);
    }
}

/// `SuspendGlobalActivity` (`0x82da5220`) and `ResumeGlobalActivity` (`0x82da5278`): the
/// activity's first callout objective group (`GroupList.find`) stops or resumes its evaluation.
fn set_global_activity_evaluation(world: &mut World, activity: usize, status: bool) {
    if let Some(&group) = world.callouts.groups_of(activity).first() {
        objectives::set_evaluation_status(world, group, status);
    }
}

pub(crate) fn suspend_global_activity(world: &mut World, activity: usize) {
    set_global_activity_evaluation(world, activity, false);
}

pub(crate) fn resume_global_activity(world: &mut World, activity: usize) {
    set_global_activity_evaluation(world, activity, true);
}

/// `ShutdownActivity` (`0x82db4ba8`): the activity's callout groups deleted, its callouts,
/// trigger data and registered players dropped.
pub fn shutdown_activity(world: &mut World, activity: usize) {
    delete_groups(world, activity);
    world.callouts.callouts.retain(|(a, _)| *a != activity);
    world.callouts.players.remove(&activity);
}

/// `Callouts.ClearCallouts()` (`0x82dadd28`).
fn clear_callouts_native(lua: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let activity = closure_activity(lua, world)?;
    clear_callouts(world, activity);
    Ok(Vec::new())
}

/// `Callouts.ClearTriggers()` (`0x82da9ed0`): `TriggerList.erase(activity)`, which holds none.
fn clear_triggers(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(Vec::new())
}

/// `Callouts.RegisterPlayer(player)` (`0x82db53a0`).
fn register_player_native(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let activity = closure_activity(lua, world)?;
    register_player(world, activity, get_int(call, 0));
    Ok(Vec::new())
}

/// `Callouts.UnregisterPlayer(player)` (`0x82dac600`): out of the activity's registered players.
fn unregister_player(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let activity = closure_activity(lua, world)?;
    let player = get_int(call, 0);
    if let Some(players) = world.callouts.players.get_mut(&activity) {
        players.remove(&player);
    }
    Ok(Vec::new())
}

/// `Callouts.SetPlayerLeading(leading)` (`0x82da7bc8`): the activity's `TriggerData::
/// playerLeading`; without trigger data (always) nothing.
fn set_player_leading(_: &mut Lua<World>, _: &mut World, call: &NativeCall) -> R {
    let _leading = get_bool(call, 0);
    Ok(Vec::new())
}

/// `Callouts.SetupCalloutHUDData(groupName, player)` (`0x82db1920`): `SetHUDGroup`.
fn setup_callout_hud_data(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let activity = closure_activity(lua, world)?;
    let name = get_string(call, 0).unwrap_or_default();
    set_hud_group(world, activity, &name, get_int(call, 1));
    Ok(Vec::new())
}

/// `Callouts.StartTimers()` (`0x82da7a98`): `StartTimer` (`0x82da5120`) zeroes the activity's
/// `TriggerData::timeElapsed`; without trigger data (always) nothing.
fn start_timers(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(Vec::new())
}

/// `Callouts.StopTimers()` (`0x82da7b30`): `StopTimer` (`0x82da51a0`) sets the activity's
/// `TriggerData::timeElapsed` to -1; without trigger data (always) nothing.
fn stop_timers(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(Vec::new())
}

/// `Callouts.TriggerCallout(tContestCalloutDefinition)` (`0x82db1820`): the definition's callout
/// (its `RefSpec` collection) inserted into the activity's callouts with no status, then
/// `TriggerCallout`.
fn trigger_callout_native(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let activity = closure_activity(lua, world)?;
    let callout = match call.arg(0) {
        Value::LightUserData(handle) => match world.pointers.get(*handle) {
            Some(Pointee::Attrib(definition)) => world
                .db
                .bytes(definition.offset_by(8), 8)
                .map_or(0, |b| u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])),
            Some(Pointee::DefaultData) => 0,
            _ => return Err(lua.error("Callouts.TriggerCallout: expected a tContestCalloutDefinition".to_owned())),
        },
        _ => return Err(lua.error("Callouts.TriggerCallout: expected a tContestCalloutDefinition".to_owned())),
    };
    world.callouts.insert_callout(activity, CalloutData { status: status::NONE, callout });
    trigger_callout(world, activity, callout);
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("Callouts", "ClearCallouts", clear_callouts_native),
        ("Callouts", "ClearTriggers", clear_triggers),
        ("Callouts", "RegisterPlayer", register_player_native),
        ("Callouts", "UnregisterPlayer", unregister_player),
        ("Callouts", "SetPlayerLeading", set_player_leading),
        ("Callouts", "SetupCalloutHUDData", setup_callout_hud_data),
        ("Callouts", "StartTimers", start_timers),
        ("Callouts", "StopTimers", stop_timers),
        ("Callouts", "TriggerCallout", trigger_callout_native),
    ]
}
