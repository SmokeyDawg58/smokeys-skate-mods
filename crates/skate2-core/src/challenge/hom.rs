//! `Sk8::Challenge::ChallengeHOMManager`'s career half: the Hall of Meat challenges
//! (`ChallengeType` 33) a bail completes, the local skater's bail score and the replay the Hall
//! of Meat flow manager plays of it. Includes manager eligibility/objective updates825b30a8,
//! session-marker allowance, and StatsBackEnd.UpdateBailStats8257da60's record ordering.

use super::{
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::{Event, Field, Output, World},
};

/// `eChallengeTypes` of a Hall of Meat challenge.
const HOM_CHALLENGE: i32 = 33;
/// `mManagerChallengeKey`: `hall_of_meat_sp_flow_manager`'s unique ID (its challenge key).
const MANAGER_CHALLENGE: u64 = 0x3ed0_0d2e_fa75_9fe5;
/// `challenge_global_data`: an activity only debug builds create.
const DEBUG_ONLY: u64 = 0xfd40_e4a3_1027_c984;

/// The manager's state (constructor `0x825bc7c0`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomManager {
    /// `mGameTypeFlags`, which `GameLoad` sets.
    game_type_flags: u32,
    pub visuals_enabled: bool,
    pub visual_activity: Option<usize>,
    /// mTick (+0x34), set when the replay manager starts (825b3280).
    pub tick:u32,
    pub enabled: bool,
    pub signing_up: bool,
    pub allow_marker: bool,
    /// `mWipeoutStartFrame`: the replay frame the local skater's last bail started on.
    pub wipeout_start_frame: u32,
    /// `mReadyToStartReplay`.
    pub ready_to_start_replay: bool,
    /// `mTrackedChallenges`: the Hall of Meat activities.
    tracked: Vec<usize>,
    /// `mCompletedChallenges`: those completed since the flow manager last completed.
    pub(super) completed: Vec<usize>,
}

impl Default for HomManager {
    fn default()->Self {Self {game_type_flags:0,visuals_enabled:true,visual_activity:None,
        enabled:true,tick:0,signing_up:false,allow_marker:true,wipeout_start_frame:0,
        ready_to_start_replay:false,tracked:Vec::new(),completed:Vec::new()}}
}
impl HomManager { pub fn completed_count(&self)->usize {self.completed.len()} }

/// StatsBackEnd.UpdateBailStats8257da60, IsBailScoreAPersonalBest82569678.
#[derive(Clone,Debug,Default,PartialEq)]
pub struct HomStats {
    pub last: Option<super::mission::HomBail>,
    pub top: Vec<super::mission::HomBail>,
    pub top_metadata:Vec<HomMetadata>,
    pub last_metadata:Option<HomMetadata>,
    pub last_personal_best: bool,
    pub last_top_bail: bool,
    pub total_bails:u32,
    pub time_bailing:f32,
    pub entity_hits:[u32;2],
    pub injuries:[u32;5],
    pub tweaks:u32,
}
/// HOMResultsData+0x4f0 date fields and +0x520 location; the host supplies local time.
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct HomMetadata {pub calendar:Option<[u16;6]>,pub location:String,pub location_key:u64}
impl HomStats {
    pub fn personal_best(&self,score:u32)->bool {score>=20000 && self.top.first().is_none_or(|b|score>b.score)}
    fn update(&mut self,bail:super::mission::HomBail,enabled:bool,metadata:HomMetadata) {
        self.last_personal_best=false;self.last_top_bail=false;
        self.total_bails=self.total_bails.wrapping_add(1);self.time_bailing+=bail.overall_duration;
        for i in 0..2 {self.entity_hits[i]=self.entity_hits[i].wrapping_add(bail.entity_counts[i]);}
        if !enabled {return;}
        self.last=Some(bail);self.last_metadata=Some(metadata.clone());
        for level in bail.bone_damage_levels {if (0..=5).contains(&level) {let level=(level as usize).min(4);self.injuries[level]=self.injuries[level].wrapping_add(1);}}
        self.tweaks=self.tweaks.wrapping_add(bail.num_body_tweaks_performed);
        if bail.score<20000 {return;}
        let index=self.top.iter().position(|b|b.score<bail.score).unwrap_or(self.top.len());
        if index==self.top.len() && self.top.len()>=10 {return;}
        self.last_top_bail=true;self.last_personal_best=index==0;
        self.top.insert(index,bail);self.top.truncate(10);
        self.top_metadata.insert(index,metadata);self.top_metadata.truncate(10);
    }
}

pub(crate) fn enable_visuals(world:&mut World,activity:Option<usize>,enable:bool) {
    world.hom.visuals_enabled=enable;
    world.hom.visual_activity=if enable {None}else{activity};
    if let Ok(mut scoring)=world.scoring.lock() {scoring.hom_visuals=enable;}
}

/// ChallengeHOMManager.Update825b30a8 (IActivitySubController this is +4).
pub(crate) fn update(world:&mut World) {
    if world.hom.game_type_flags!=4 {return;}
    let can_enable=!world.hom.signing_up && world.activities.main.is_none()
        && !world.characters.facts().is_some_and(|f|f.chase_is_on());
    let local=world.skaters.iter().find(|s|s.local);
    let bailing=local.is_some_and(|s|s.hom_bail_in_progress||s.hom_bail_completed);
    let score=local.map_or(0,|s|s.hom_score);
    if (!bailing||!can_enable) && world.hom.enabled!=can_enable {
        let groups:Vec<_>=world.objectives.groups.iter().filter(|g|world.hom.tracked.contains(&g.activity)).map(|g|g.id).collect();
        for group in groups {
            super::objectives::set_evaluation_status(world,group,can_enable);
            if !can_enable {super::objectives::reset_objectives(world,group,true);}
        }
        world.hom.enabled=can_enable;
    }
    world.hom.allow_marker=world.hom.completed.is_empty() && !world.hom_stats.lock().expect("HoM stats lock").personal_best(score);
    if world.hom.ready_to_start_replay {
        super::activity::start_main_challenge(world,MANAGER_CHALLENGE);
        world.hom.tick=1;
        world.hom.ready_to_start_replay=false;
    }
}

/// `cActivityManager::CreateActivity` (`0x82cdbca8`) passes every activity that is not debug-only
/// to `AddActivity` (`0x825bfe10`), which keeps the Hall of Meat challenges.
pub(crate) fn init(world: &mut World) {
    for activity in 0..world.activities.activities.len() {
        let debug_only = world.activities.activities[activity]
            .global_data
            .and_then(|(class, collection)| world.db.attribute(class, collection, DEBUG_ONLY))
            .and_then(|a| a.bool(0))
            .unwrap_or(false);
        if !debug_only && super::signup::challenge_type(world, activity) == HOM_CHALLENGE {
            world.hom.tracked.push(activity);
        }
    }
}

/// `GameLoad(gameTypeFlags)` (`0x825b89b8`): the flags; no bail frame, score or replay, sign-ups
/// off and the session marker allowed; with completed challenges the backpack and pause menu come
/// back; the completed list empties.
pub(crate) fn game_load(world: &mut World, flags: u32) {
    world.hom.game_type_flags = flags;
    world.hom.signing_up=false;world.hom.allow_marker=true;
    world.hom.ready_to_start_replay = false;
    world.hom.wipeout_start_frame = 0;
    world.hom_wipeout_score = 0;
    if !world.hom.completed.is_empty() {
        world.pause_menu_disabled = false;
    }
    world.hom.completed.clear();
}

/// `CompleteChallenge` (`0x825bfec8`) in the career: the flow manager's completion wins and
/// processes every completed Hall of Meat challenge and gives the backpack and pause menu back; a
/// tracked challenge's completion is kept and takes them away. The challenge whose HOM visuals
/// run (`mCurHOMVisualActivity`, set by `EnableHOMVisuals`) restores the visuals on completion.
pub(crate) fn complete_challenge(world: &mut World, activity: usize) {
    if world.hom.visual_activity==Some(activity) {enable_visuals(world,None,true);}
    if world.hom.game_type_flags != 4 {
        return;
    }
    if world.activities.activities[activity].key == MANAGER_CHALLENGE {
        let completed = world.hom.completed.clone();
        for &challenge in &completed {
            let key = world.activities.activities[challenge].key;
            super::signpost::win_challenge(world, key);
        }
        if !completed.is_empty() {
            super::signpost::process_challenge_results(world);
        }
        world.hom.completed.clear();
        world.pause_menu_disabled = false;
    }
    if world.hom.tracked.contains(&activity) {
        world.hom.completed.push(activity);
        world.pause_menu_disabled = true;
    }
}

/// The manager's Hermes handlers: `WipeoutMsg` (`0x825b3290`) keeps the replay frame the local
/// player's bail started on (`DataSysMan::GetLastFrameNumberWritten`); `WipeoutOverMsg`
/// (`0x825b6110`) the local skater's `mHoM_BailData.mScore`, and with completed challenges (or a
/// personal-best bail recorded by the stats back end) the replay is ready.
pub(crate) fn receive(world: &mut World, event: &Event) {
    let Event::Challenge(message) = event else { return };
    #[expect(clippy::cast_possible_truncation, reason = "the message's int")]
    let player = message.fields.as_ref().and_then(|fields| {
        fields.iter().find(|(name, _)| *name == "PlayerID").and_then(|(_, value)| match value {
            Field::Number(n) => Some(*n as i32),
            _ => None,
        })
    });
    if world.hom.game_type_flags != 4 || player != Some(0) {
        return;
    }
    if message.kind == super::keys::hash32("cMsgWipeout") {
        world.hom.wipeout_start_frame = world.replay.last_frame_number_written();
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            let groups: Vec<_> = world.objectives.groups.iter()
                .filter(|g| world.hom.tracked.contains(&g.activity))
                .map(|g| (&world.activities.activities[g.activity].name, g.player, g.evaluate,
                    g.objectives.iter().map(|o| o.state).collect::<Vec<_>>()))
                .collect();
            eprintln!("career: HoM start player={player:?} enabled={} cvar={} signing_up={} main={:?} chase={} fields={:?} groups={groups:?}",
                world.hom.enabled, world.hom_enabled, world.hom.signing_up,
                world.activities.main.map(|i| &world.activities.activities[i].name),
                world.characters.facts().is_some_and(|f| f.chase_is_on()), message.fields);
        }
    } else if message.kind == super::keys::hash32("cMsgWipeoutOver")
        && let Some(frame) = world.skaters.iter().find(|frame| frame.local)
    {
        world.hom_wipeout_score = frame.hom_score.cast_signed();
        let enabled=world.activities.main.is_none() && world.hom.enabled && frame.hom_effect_enabled;
        let mut stats=world.hom_stats.lock().expect("HoM stats lock");
        stats.update(frame.hom_bail,enabled,HomMetadata{calendar:world.hom_local_calendar.map(|clock|clock()),location:world.hom_location.clone(),location_key:world.hom_location_key});
        if !world.hom.completed.is_empty() || stats.last_personal_best {
            world.hom.ready_to_start_replay = true;
        }
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career: HoM bail score={} eligible={enabled} broken={} top={} personal_best={} completed={} replay_ready={} save_image={}",
                frame.hom_bail.score,frame.hom_bail.bone_damage_levels.iter().filter(|&&d|matches!(d,4|5)).count(),
                stats.last_top_bail,stats.last_personal_best,world.hom.completed.len(),world.hom.ready_to_start_replay,frame.hom_bail.save_image.is_some());
        }
        drop(stats);
        world.output(Output::TrackHomStatistics{bail:frame.hom_bail,enabled});
    }
}

/// `HOM.IsHOMReadyToStartReplay()` (`0x8288ee80`): `mReadyToStartReplay`.
fn is_hom_ready_to_start_replay(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    Ok(vec![Value::Boolean(world.hom.ready_to_start_replay)])
}

/// `HOM.GetHOMStartFrame()` (`0x8288ed30`): `mWipeoutStartFrame`.
fn get_hom_start_frame(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    Ok(vec![super::binding::int_u32(world.hom.wipeout_start_frame)])
}

/// `FrontEnd.OpenHOMResults()` (`0x82896c80`): `PushState(9)`, the Hall of Meat results.
fn open_hom_results(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    world.output(Output::OpenHomResults);
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("HOM", "IsHOMReadyToStartReplay", is_hom_ready_to_start_replay),
        ("HOM", "GetHOMStartFrame", get_hom_start_frame),
        ("FrontEnd", "OpenHOMResults", open_hom_results),
    ]
}
