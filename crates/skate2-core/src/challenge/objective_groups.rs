//! `Objectives` natives on `cChallengeObjectiveGroup`s that `natives` does not hold.

use super::{
    attrib_lua::ObjectKind,
    binding::{get_int, get_int_or, get_string_or, int},
    keys::class,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    objectives,
    system::World,
};

type R = LuaResult<Vec<Value>>;

/// `challenges.Objectives` (`RefSpec[]` to `challenge_objective`).
const OBJECTIVES: u64 = 0xc4b3_4541_5e85_37be;

/// The group behind an objective group argument.
fn group_arg(lua: &Lua<World>, world: &World, value: &Value, native: &str) -> LuaResult<u32> {
    world.pointers.object(value, ObjectKind::ObjectiveGroup).ok_or_else(|| lua.error(format!("Objectives.{native}: expected an objective group")))
}

/// `Objectives.GetObjectiveHALText(group, index)` (`0x82d4a080`): the objective's
/// (`GetObjectiveInfo` `0x82d435d8`, which asserts the index) definition's HAL text (+12).
fn get_objective_hal_text(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = group_arg(lua, world, call.arg(0), "GetObjectiveHALText")?;
    let index = get_int(call, 1);
    let key = world
        .objectives
        .group_index(group)
        .and_then(|g| usize::try_from(index).ok().and_then(|i| world.objectives.groups[g].objectives.get(i)))
        .map(|o| o.key)
        .ok_or_else(|| lua.error(format!("Objectives.GetObjectiveHALText: no objective {index}")))?;
    Ok(vec![Value::string(&objectives::hal_string(&world.db, key).unwrap_or_default())])
}

/// `Objectives.MakeOnlineBonusObjective(player = -1, index = 0)` (`0x82d533c0`): the calling
/// activity's group for the player (`AddObjectiveChallenge` with flow {AllAtStart, 1}, both
/// masks all, active shown with notifications, an existing group of the player reused) gets the
/// challenge's `Objectives` entry at the index (0 past the end) added (`Bonus Obj-<name>`), and
/// is returned.
fn make_online_bonus_objective(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let activity = world.closure.ok_or_else(|| lua.error("Objectives.MakeOnlineBonusObjective: no calling activity".to_owned()))?;
    let player = get_int_or(call, 0, -1);
    let index = get_int_or(call, 1, 0);
    let group = objectives::add_objective_challenge(world, activity, player, (0, 1), (u32::MAX, u32::MAX, true));
    let key = world.activities.activities[activity].key;
    let bonus = world.db.attribute(class::CHALLENGES, key, OBJECTIVES);
    let count = bonus.map_or(0, |a| a.len());
    let index = usize::try_from(index).ok().filter(|&i| i < count).unwrap_or(0);
    let objective = bonus.and_then(|a| a.ref_spec(index)).map_or(0, |(_, objective)| objective);
    objectives::add_objective(world, group, objective);
    let handle = world.objectives.group_index(group).map_or(0, |i| world.objectives.groups[i].handle);
    Ok(vec![Value::LightUserData(handle)])
}

/// `Objectives.ChangeObjectiveText(group, index = 0, text = "")` (`0x82d49f40`): a text that is not
/// empty becomes the objective's `mTitle` (`cChallengeObjectiveGroup::SetObjectiveText`
/// `0x82d43ef0`: 127 characters, the objective dirtied); whether it was set. The native also takes
/// an index equal to the objective count, writing through the slot past its list; the port
/// refuses it.
fn change_objective_text(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = group_arg(lua, world, call.arg(0), "ChangeObjectiveText")?;
    let index = get_int_or(call, 1, 0);
    let text = get_string_or(call, 2, "");
    if text.is_empty() {
        return Ok(vec![Value::Boolean(false)]);
    }
    let objective = world
        .objectives
        .group_index(group)
        .and_then(|g| usize::try_from(index).ok().and_then(|i| world.objectives.groups[g].objectives.get_mut(i)));
    let set = objective.is_some_and(|o| {
        o.title.0 = text.chars().take(127).collect();
        o.dirty = true;
        true
    });
    Ok(vec![Value::Boolean(set)])
}

/// `Objectives.AddLastUniqueTrickToList(group)` (`0x82d517c8`):
/// `cChallengeObjectiveGroup::AddLastUniqueTrickToFilterList` (`0x82d4e8e8`), the sequence's
/// unique trick onto the group's completed list; the list's length.
fn add_last_unique_trick_to_list(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = group_arg(lua, world, call.arg(0), "AddLastUniqueTrickToList")?;
    let count = objectives::add_last_unique_trick_to_filter_list(world, group);
    Ok(vec![int(i32::try_from(count).unwrap_or(i32::MAX))])
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("Objectives", "GetObjectiveHALText", get_objective_hal_text),
        ("Objectives", "MakeOnlineBonusObjective", make_online_bonus_objective),
        ("Objectives", "AddLastUniqueTrickToList", add_last_unique_trick_to_list),
        ("Objectives", "ChangeObjectiveText", change_objective_text),
    ]
}
