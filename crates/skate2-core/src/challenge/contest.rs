//! The contest natives outside the callout and crowd managers: the `Challenge_Contest` debug
//! cvars `GameMode` reads, a skater's speed for speed contests, and the demo sponsor's
//! competitors.

use super::{
    ai,
    binding::{get_bool_or, get_int, int},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    skate::skater,
    sponsors,
    system::World,
};

type R = LuaResult<Vec<Value>>;

/// `GameMode.ContestSpeedRounds()` (`0x82885ea8`): cvar `kContestSpeedRounds` ("Short Contest
/// Rounds", `Challenge_Contest`, default false).
fn contest_speed_rounds(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(false)])
}

/// `GameMode.ContestPlayerScoreCheat()` (`0x82885ee0`): cvar `kContestPlayerScoreCheat`
/// (default false).
fn contest_player_score_cheat(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(false)])
}

/// `GameMode.ContestAISkaterScoreCheat()` (`0x82885f18`): cvar `kContestAIScoreCheat` (default
/// false).
fn contest_ai_skater_score_cheat(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(false)])
}

/// `GameMode.ContestDebugAIScoreAdjustmentEnabled()` (`0x82885f50`): cvar
/// `kContestDebugAIScoreAdjustmentEnable` (default false).
fn contest_debug_ai_score_adjustment_enabled(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(false)])
}

/// `GameMode.ContestDebugAIScoreAdjustmentRatio()` (`0x82885f88`): cvar
/// `kContestDebugAIScoreAdjustmentRatio` (default 1, range 0.01 to 100).
fn contest_debug_ai_score_adjustment_ratio(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(1.0)])
}

/// `Entity.GetSpeed(player)` (`0x82886f90`): the skater's (`ISkaterManager::FindByIndex`)
/// `PhysOut_SkateboardMotion::mSkateboardSurfaceSpeed` through `IActor::GetPhysOut`. The native
/// asserts and reads through a null actor for a player without a skater.
fn get_speed(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let frame = skater(world, player).ok_or_else(|| lua.error(format!("Entity.GetSpeed: no skater {player}")))?;
    Ok(vec![Value::Number(frame.surface_speed)])
}

/// `ChallengeProgression.GetNumberOfDemoCompetitors()` (`0x82891a40`):
/// `cSignPostSponsorMan::GetDemoCompetitorCount`.
fn get_number_of_demo_competitors(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(sponsors::demo_competitor_count(world))])
}

/// `ChallengeProgression.GetDemoSponsorName()` (`0x82891e70`): `cSignPostSponsorMan::
/// GetBoardSponsorName`.
fn get_demo_sponsor_name(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::string(&sponsors::board_sponsor_name(world))])
}

/// `ChallengeProgression.SelectDemoCompetitors(noAssetLoad = false)` (`0x82891b18`):
/// `ChallengeAIManager::SelectDemoCharacters(false, noAssetLoad)`.
fn select_demo_competitors(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let no_asset_load = get_bool_or(call, 0, false);
    if let Err(error) = ai::select_demo_characters(world, no_asset_load) {
        world.output(super::system::Output::ScriptError { context: "ChallengeAIManager::SelectDemoCharacters".to_owned(), error });
    }
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("GameMode", "ContestSpeedRounds", contest_speed_rounds),
        ("GameMode", "ContestPlayerScoreCheat", contest_player_score_cheat),
        ("GameMode", "ContestAISkaterScoreCheat", contest_ai_skater_score_cheat),
        ("GameMode", "ContestDebugAIScoreAdjustmentEnabled", contest_debug_ai_score_adjustment_enabled),
        ("GameMode", "ContestDebugAIScoreAdjustmentRatio", contest_debug_ai_score_adjustment_ratio),
        ("Entity", "GetSpeed", get_speed),
        ("ChallengeProgression", "GetNumberOfDemoCompetitors", get_number_of_demo_competitors),
        ("ChallengeProgression", "GetDemoSponsorName", get_demo_sponsor_name),
        ("ChallengeProgression", "SelectDemoCompetitors", select_demo_competitors),
    ]
}
