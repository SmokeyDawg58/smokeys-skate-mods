//! `Sk8::Service::cServiceManager`: Contacts requests, authored service activities,
//! Big Black's timers, and the sign-up / game-unload lifecycle.

use super::{
    binding::{get_bool, get_int},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    keys::{class, hash64},
    system::{Event, Field, Message, Output, World},
    timers::TimerKind,
};

type R = LuaResult<Vec<Value>>;

/// `Sk8::Service::kSecurityHUDGroupName`.
const SECURITY_HUD_GROUP: &str = "SecurityHUDGroup";
/// `Sk8::Service::kSecurityTimerHUDName` (`BigBlackTimerHUDData::kHUDName`).
const SECURITY_TIMER_HUD: &str = "bigblacktimer";
/// The security service's time limit: class `0x46a5_6add_f284_c381`, collection
/// `0xadd0_a256_14bf_f4ac`, float attribute `0xaf14_a38d_090e_ece2` (300 seconds).
const SECURITY_TIME: (u64, u64, u64) = (0x46a5_6add_f284_c381, 0xadd0_a256_14bf_f4ac, 0xaf14_a38d_090e_ece2);
/// The `fe_movie` keys `PlayServiceMovie` plays: `Service_BB01`, `Service_Uncap01` and
/// `Service_Drain01`.
const SERVICE_MOVIES: [u64; 3] = [0xe059_e90f_7dce_78ec, 0xd16c_07e6_507b_bde4, 0x377c_b480_7676_c190];

/// `Sk8::Service::eServiceType`.
pub mod service_type {
    pub const SECURITY: i32 = 0;
    pub const UNCAPPING: i32 = 1;
    pub const DRAINING: i32 = 2;
}

/// `cServiceManager` (constructor `0x8289db20`): the fields its natives read and write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceManager {
    /// `mSecurityTimerID`.
    pub security_timer_id: i32,
    /// `mSecurityTimeOutWarningTimerID`.
    pub security_time_out_warning_timer_id: i32,
    pub update_timer_id: i32,
    pub goodbye_speech_timer_id: i32,
    /// Readback of ILWManager::IsBigBlackServiceActive, published by the host
    /// after the actual pedestrian spawn/cull operation.
    pub big_black_active: bool,
    /// `mUncappingServiceKey`: the uncapping challenge on offer, 0 for none.
    pub uncapping_service_key: u64,
    /// `mDrainingServiceKey`: the pool draining challenge on offer, 0 for none.
    pub draining_service_key: u64,
    /// `mAllowCICAcceptToContinue`.
    pub allow_cic_accept_to_continue: bool,
    /// `mGameLoadFlag`.
    pub game_load_flag: u32,
    /// `mServiceIntroductionInProgress`.
    pub introduction_in_progress: bool,
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self {
            security_timer_id: -1,
            security_time_out_warning_timer_id: -1,
            update_timer_id: -1,
            goodbye_speech_timer_id: -1,
            big_black_active: false,
            uncapping_service_key: 0,
            draining_service_key: 0,
            allow_cic_accept_to_continue: true,
            game_load_flag: 0,
            introduction_in_progress: false,
        }
    }
}

/// `cServiceManager::GameLoad(gameTypeFlags)` (`0x8289c620`): a new game type resets the timers,
/// the offered services and the flags.
pub(crate) fn game_load(world: &mut World, flags: u32) {
    let services = &mut world.services;
    if flags != services.game_load_flag {
        *services = ServiceManager { game_load_flag: services.game_load_flag, ..ServiceManager::default() };
    }
    services.game_load_flag = flags;
}

/// `cSignPostMessageMan::GetMessageSetting(name, true)` (`0x825cd6a8`): the stored word is
/// nonzero, true for a setting the manager does not hold.
pub(crate) fn message_setting(world: &World, name: &str) -> bool {
    world.signpost.message_setting_value(name).is_none_or(|word| word != 0)
}

/// `cServiceManager::SpawnBigBlack` (`0x8289cf60`): `cMsgSpawnBigBlack` on the living world's
/// broadcaster, then his map icon.
fn spawn_big_black(world: &mut World) {
    world.output(Output::BigBlack { spawn: true });
    super::map_icons::activate_big_black_icon(world, true);
}

/// `cServiceManager::UnspawnBigBlack` (`0x8289cfc0`): the map icon goes first, then
/// `cMsgUnspawnBigBlack`.
fn unspawn_big_black(world: &mut World) {
    super::map_icons::activate_big_black_icon(world, false);
    world.output(Output::BigBlack { spawn: false });
}

/// `Service.SetServiceAvailable(data, serviceType, available)` (`0x8288ef28`):
/// `cServiceManager::SetServiceAvailable` (`0x8289c538`) with the activity's challenge key. An
/// uncapping or draining service offers the challenge, or nothing when not available; any other
/// type is the native's assertion and changes nothing.
fn set_service_available(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(activity) = super::attrib_lua::activity_of(lua, call.arg(0)) else {
        return Err(lua.error("SetServiceAvailable: no activity".to_owned()));
    };
    let key = world.activities.activities[activity].key;
    let service = get_int(call, 1);
    let key = if get_bool(call, 2) { key } else { 0 };
    match service {
        service_type::UNCAPPING => world.services.uncapping_service_key = key,
        service_type::DRAINING => world.services.draining_service_key = key,
        _ => {}
    }
    Ok(Vec::new())
}

/// `Service.IsSecurityServiceAvailable()` (`0x8288f008`): `cServiceManager::
/// IsSecurityServiceAvailable` (`0x8289c448`): the `Security Available` setting, no chase
/// (`ChaseMan::ChaseIsOn`), `cActivityManager::CanStartMainChallenge` (`0x82cd8798`: no main
/// activity and the AI manager's skaters released), Big Black's service not active
/// (`ILWManager::IsBigBlackServiceActive`) and no service
/// introduction in progress.
fn is_security_service_available(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(security_service_available(world))])
}

/// IsSecurityServiceAvailable8289c448.
pub fn security_service_available(world: &World) -> bool {
    let setting = message_setting(world, "Security Available");
    let chase = world.characters.facts().is_some_and(|facts| facts.chase_is_on());
    let can_start = world.activities.main.is_none() && super::ai::can_start_main_challenge(world);
    setting && !chase && can_start && !world.services.big_black_active && !world.services.introduction_in_progress
}

/// IsUncappingServiceAvailable8289c3e8 / IsDrainingServiceAvailable8289c388.
pub fn available(world: &World, service: i32) -> bool {
    match service {
        service_type::SECURITY => security_service_available(world),
        service_type::UNCAPPING => world.services.uncapping_service_key != 0
            && !world.services.introduction_in_progress && message_setting(world, "Uncapping Available"),
        service_type::DRAINING => world.services.draining_service_key != 0
            && !world.services.introduction_in_progress && message_setting(world, "Pool Draining Available"),
        _ => false,
    }
}

/// Native fixed service prices (8289d5c8 / 8289d6d0 / 8289d7f0).
pub const fn cost(service: i32) -> i32 {
    match service { service_type::SECURITY => 500, service_type::UNCAPPING => 100,
        service_type::DRAINING => 200, _ => 0 }
}

/// cServiceManager.Init8289e0d8 and GetNum[Drained]PoolsInWorld8289c890/958:
/// leaf descendants of challenges.service_draining, IsServiceableByUser only.
pub(crate) fn pool_counts(world: &World) -> (i32, i32) {
    let root = hash64("service_draining");
    let mut counts = (0, 0);
    for key in world.db.leaf_collections(class::CHALLENGES) {
        let mut parent = world.db.parent(class::CHALLENGES, key);
        while let Some(value) = parent.filter(|&p| p != 0 && p != root) {
            parent = world.db.parent(class::CHALLENGES, value);
        }
        if parent != Some(root) || !world.db.attribute(class::CHALLENGES, key, hash64("IsServiceableByUser"))
            .and_then(|a| a.bool(0)).unwrap_or(false) { continue; }
        counts.1 += 1;
        if super::dynamic_hull::test_state(world, key, "Drained") == Some(true) { counts.0 += 1; }
    }
    counts
}

pub(crate) fn record_service(world: &mut World, service: i32, paid: bool) {
    let pools = (service == service_type::DRAINING).then(|| {
        let (drained, total) = pool_counts(world);
        (drained + 1, total)
    });
    world.output(Output::ServiceStatistics { service, paid, pools });
}

/// ActivateSecurity/Uncapping/DrainingService8289d5c8/6d0/7f0. The caller
/// has checked availability and cash. Scripts own the speech/movie/work flow.
pub fn activate(world: &mut World, service: i32) {
    let key = match service {
        service_type::SECURITY => 0,
        service_type::UNCAPPING => world.services.uncapping_service_key,
        service_type::DRAINING => world.services.draining_service_key,
        _ => return,
    };
    world.services.introduction_in_progress = true;
    world.pause_menu_disabled = true;
    world.send(Event::Challenge(Message::new("cMsgServiceRequested", vec![("ServiceKey", Field::Key(key))])));
    super::signpost_progress::subtract_cash(world, cost(service), 0);
    record_service(world, service, true);
    match service {
        service_type::UNCAPPING => world.services.uncapping_service_key = 0,
        service_type::DRAINING => world.services.draining_service_key = 0,
        _ => {},
    }
}

/// DeactivateSecurityService8289d020: timers are invalidated, not deleted;
/// their later expiry must not dismiss a subsequently hired protector.
pub fn deactivate_security(world: &mut World) {
    world.hud.show(SECURITY_HUD_GROUP, SECURITY_TIMER_HUD, false);
    world.hud.remove_group_from_active(SECURITY_HUD_GROUP);
    world.services.security_timer_id = -1;
    world.services.security_time_out_warning_timer_id = -1;
    // 8289d070..a8 constructs kind82b365c4 (cMsgServiceEnded). The
    // linker's shared SendMsg body is labelled ServiceAboutToEnd in IDA;
    // that other kind, baccd82a, belongs only to the ten-second warning.
    world.send(Event::Challenge(Message::new("cMsgServiceEnded", vec![("ServiceKey", Field::Key(0))])));
    unspawn_big_black(world);
}

/// OnChallengeSelect8289c670 / OnChallengeDecline8289c6d0. Native pauses
/// the main timer only; the warning timer has its own unpaused lifetime.
pub(crate) fn on_select(world: &mut World) { pause_security_timer(world, true); }
pub(crate) fn on_decline(world: &mut World) { pause_security_timer(world, false); }
fn pause_security_timer(world: &mut World, pause: bool) {
    if world.services.big_black_active && world.services.security_timer_id != -1 {
        world.timers.set_paused(world.services.security_timer_id, pause);
    }
}

/// OnChallengeAccept8289d428: challenges explicitly opt into keeping the
/// protection service. Otherwise the goodbye completes before sign-up proceeds.
pub(crate) fn on_accept(world: &mut World, activity: usize) {
    let running = world.activities.find(hash64("service_security"))
        .is_some_and(|a| world.activities.activities[a].graph.running);
    if world.services.game_load_flag != 4 || !running { return; }
    if world.services.big_black_active {
        let allowed = super::signup_poster::local_bool(world, activity, hash64("AllowSecurityServiceToBeCalled"));
        if allowed { pause_security_timer(world, false); }
        else {
            world.services.allow_cic_accept_to_continue = false;
            deactivate_security(world);
            world.services.goodbye_speech_timer_id = world.timers.add(TimerKind::CountDown, 1.5, None);
        }
    } else {
        world.services.allow_cic_accept_to_continue = false;
        world.send(Event::Challenge(Message::new("cMsgReevaluateServicePrereqs", Vec::new())));
    }
}

/// CompleteChallenge8289c5c8, adjusted IActivitySubController this+4.
pub(crate) fn complete_challenge(world: &mut World, is_main: bool) {
    if world.services.game_load_flag == 4 && is_main {
        world.services.update_timer_id = world.timers.add(TimerKind::CountDown, 0.5, None);
    }
}

/// HideBigBlackIcon8289dde8 / RunMainActivity8289e050.
pub(crate) fn run_main_activity(world: &mut World) {
    if let Some(phone) = world.hud.group_mut("phonelist").and_then(|g| g.phonelist.as_mut()) {
        phone.show_icon[0] = false;
        phone.dirty = true;
    }
}

/// GameUnload8289d9d0. The physical owner consumes the unspawn before the
/// next game load; service activities and timers belong to this CareerSession.
pub(crate) fn game_unload(world: &mut World) {
    world.services.introduction_in_progress = false;
    world.pause_menu_disabled = false;
    if world.services.big_black_active { deactivate_security(world); }
}

/// OnTimerExpired8289d198. Reemit painter notifications even when their
/// cached keys have not changed: the service globals just resumed.
pub(crate) fn receive(world: &mut World, event: &Event) {
    let Event::TimerExpired { timer } = event else { return };
    if *timer == world.services.security_timer_id { deactivate_security(world); }
    else if *timer == world.services.security_time_out_warning_timer_id {
        world.send(Event::Challenge(Message::new("cMsgServiceAboutToEnd", vec![("ServiceKey", Field::Key(0))])));
    } else if *timer == world.services.update_timer_id {
        super::world_painter::reemit_service_notifications(world);
    } else if *timer == world.services.goodbye_speech_timer_id {
        world.services.allow_cic_accept_to_continue = true;
    }
}

/// `Service.IsLocalPlayerInNoSkateZone()` (`0x8288f158`): `cFilmManager::IsInNoSkateZone`, which
/// `WorldPainterKeyChangeMessageHandler` (`0x82da7838`) keeps from the security guard areas.
fn is_local_player_in_no_skate_zone(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(world.world_painter.in_no_skate_zone)])
}

/// `IsSignupAvailable` (`0x8289c660`): no service introduction is in progress.
pub(crate) fn is_signup_available(world: &World) -> bool {
    !world.services.introduction_in_progress
}

/// `SuspendGlobalActivity` (`0x8289da38`): a service introduction in progress ends, giving the
/// backpack and pause menu back.
pub(crate) fn suspend_global_activity(world: &mut World) {
    if world.services.introduction_in_progress {
        world.services.introduction_in_progress = false;
        world.pause_menu_disabled = false;
    }
}

/// `Service.PlayServiceMovie(serviceType)` (`0x8288f200`): `cServiceManager::PlayServiceMovie`
/// (`0x8289d0f8`): the service's `fe_movie` as `FrontEndState_MoviePlayer::mMovieKey`, then
/// `PushState(28)`. Another type is the native's assertion; it would replay the last movie, which
/// the port does not keep, so nothing plays.
fn play_service_movie(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(&key) = usize::try_from(get_int(call, 0)).ok().and_then(|index| SERVICE_MOVIES.get(index)) {
        let movie = world.db.layout_text(crate::ui::movie_player::FE_MOVIE, key, 0).unwrap_or_default().to_owned();
        world.output(Output::PlayMovie { message: String::new(), movie, key });
    }
    Ok(Vec::new())
}

/// `Service.HasDrainingServiceBeenUnlockedViaProgression()` (`0x8288f2a8` → `0x8289c358`): the
/// `Pool Draining Available` setting.
fn has_draining_service_been_unlocked(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(message_setting(world, "Pool Draining Available"))])
}

/// `Service.HasUncappingServiceBeenUnlockedViaProgression()` (`0x8288f350` → `0x8289c370`): the
/// `Uncapping Available` setting.
fn has_uncapping_service_been_unlocked(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(message_setting(world, "Uncapping Available"))])
}

/// `Service.LoadSecurityTimerHUD()` (`0x8288f3f8`): `cServiceManager::LoadSecurityTimerHUD`
/// (`0x8289e190`): the security group with its timer, or the timer added to the existing group.
fn load_security_timer_hud(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if !world.hud.group_exists(SECURITY_HUD_GROUP) {
        world.hud.create_group(SECURITY_HUD_GROUP, &[SECURITY_TIMER_HUD]);
    } else if !world.hud.has_component(SECURITY_HUD_GROUP, SECURITY_TIMER_HUD) {
        world.hud.add_component(SECURITY_HUD_GROUP, SECURITY_TIMER_HUD);
    }
    Ok(Vec::new())
}

/// `Service.StartSecurityTimerHUDAndSpawnBigBlack()` (`0x8288f4a8`):
/// `cServiceManager::StartSecurityTimerHUDAndSpawnBigBlack` (`0x8289dc98`). With the security
/// group, the security time's countdown and one 10 seconds shorter (the time-out warning) start,
/// the group becomes active and its timer shows (the timer's `BigBlackTimerHUDData` takes the
/// first countdown's id); Big Black then spawns.
fn start_security_timer_hud_and_spawn_big_black(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if world.hud.group_exists(SECURITY_HUD_GROUP) {
        let (class, collection, attribute) = SECURITY_TIME;
        let time = world.db.attribute(class, collection, attribute).and_then(|a| a.f32(0)).unwrap_or(0.0);
        world.services.security_timer_id = world.timers.add(TimerKind::CountDown, time, None);
        world.services.security_time_out_warning_timer_id = world.timers.add(TimerKind::CountDown, time - 10.0, None);
        if let Some(group)=world.hud.group_mut(SECURITY_HUD_GROUP){group.big_black_timer=Some(world.services.security_timer_id);}
        world.hud.add_group_to_active(SECURITY_HUD_GROUP);
        world.hud.show(SECURITY_HUD_GROUP, SECURITY_TIMER_HUD, true);
    }
    spawn_big_black(world);
    Ok(Vec::new())
}

/// `Service.SetAllowCICAcceptToContinue(allow)` (`0x8288f558`): `mAllowCICAcceptToContinue`.
fn set_allow_cic_accept_to_continue(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.services.allow_cic_accept_to_continue = get_bool(call, 0);
    Ok(Vec::new())
}

/// `Service.SetServiceIntroductionInProgress(inProgress)` (`0x8288f600`):
/// `cServiceManager::SetServiceIntroductionInProgress` (`0x8289d530`), which also disables the
/// backpack and pause menu (`StateManagerFrontEnd::mbDisableBackpackAndPauseMenu`) for its
/// duration.
fn set_service_introduction_in_progress(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let in_progress = get_bool(call, 0);
    world.services.introduction_in_progress = in_progress;
    world.pause_menu_disabled = in_progress;
    Ok(Vec::new())
}

/// `Service.ActivateSecurityServiceForChallengeReplay()` (`0x8288f7f8`):
/// `cServiceManager::SpawnBigBlack`.
fn activate_security_service_for_challenge_replay(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    spawn_big_black(world);
    Ok(Vec::new())
}

/// `Service.DeactivateSecurityServiceForChallengeReplay()` (`0x8288f8a8`):
/// `cServiceManager::UnspawnBigBlack`.
fn deactivate_security_service_for_challenge_replay(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    unspawn_big_black(world);
    Ok(Vec::new())
}

/// `Telemetry.ExecutedService(data)` (`0x828947e0`): `cChallengeTelemetryManager::
/// ExecutedService` (`0x825afa78`) of the activity's `ServiceType` only logs a `Sk8::Net` event.
fn telemetry_executed_service(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("Service", "SetServiceAvailable", set_service_available),
        ("Service", "IsSecurityServiceAvailable", is_security_service_available),
        ("Service", "IsLocalPlayerInNoSkateZone", is_local_player_in_no_skate_zone),
        ("Service", "PlayServiceMovie", play_service_movie),
        ("Service", "HasDrainingServiceBeenUnlockedViaProgression", has_draining_service_been_unlocked),
        ("Service", "HasUncappingServiceBeenUnlockedViaProgression", has_uncapping_service_been_unlocked),
        ("Service", "LoadSecurityTimerHUD", load_security_timer_hud),
        ("Service", "StartSecurityTimerHUDAndSpawnBigBlack", start_security_timer_hud_and_spawn_big_black),
        ("Service", "SetAllowCICAcceptToContinue", set_allow_cic_accept_to_continue),
        ("Service", "SetServiceIntroductionInProgress", set_service_introduction_in_progress),
        ("Service", "ActivateSecurityServiceForChallengeReplay", activate_security_service_for_challenge_replay),
        ("Service", "DeactivateSecurityServiceForChallengeReplay", deactivate_security_service_for_challenge_replay),
        ("Telemetry", "ExecutedService", telemetry_executed_service),
    ]
}
