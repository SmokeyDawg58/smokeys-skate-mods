//! The dynamic hulls (gates, pool water, skate stoppers) from the shipped data.

use super::super::{
    assets, characters, dynamic_hull,
    keys::{self, hash64},
    presentation::{self, PresentationEvent, event},
    signpost_progress,
    stuff_menu::{self, ChallengeMap, MapAction, MapInput, MapPopup, MenuText},
    system::Output,
    unlockables,
};
use super::{ChallengeSystem, career_system, load_locations};

fn location(system: &ChallengeSystem, name: &str) -> [f32; 3] {
    let transform = system.world().locations.get(&keys::hash_value32(name)).unwrap_or_else(|| panic!("location {name}"));
    [transform[12], transform[13], transform[14]]
}

/// The career's system with the global locations, game-loaded as the career session does it.
fn loaded_career() -> ChallengeSystem {
    let mut system = career_system();
    load_locations(&mut system);
    system.game_load();
    system
}

/// The manager's update with the local skater's tester at `point`, and what it asked for.
fn update_at(system: &mut ChallengeSystem, point: [f32; 3]) -> Vec<Output> {
    system.world_mut().player.tester_base = Some(point);
    dynamic_hull::update(system.world_mut());
    system.take_outputs()
}

fn loads(outputs: &[Output]) -> Vec<(String, bool, u32)> {
    outputs
        .iter()
        .filter_map(|o| match o {
            Output::LoadDynamicHull { file_name, callback, asset_flags, .. } => Some((file_name.clone(), *callback, *asset_flags)),
            _ => None,
        })
        .collect()
}

fn unloads(outputs: &[Output]) -> Vec<u32> {
    outputs
        .iter()
        .filter_map(|o| match o {
            Output::UnloadDynamicHull { hull } => Some(*hull),
            _ => None,
        })
        .collect()
}

fn names(outputs: &[Output]) -> Vec<String> {
    let mut names: Vec<String> = loads(outputs).into_iter().map(|(name, _, _)| name).collect();
    names.sort();
    names
}

/// A new career starts at `J_Gamestart`, inside the boxes of Slappy's gate, the hotel alley,
/// the end-game unlock and two pools: each default state's hull loads (career load mask,
/// DMO flags `0xe0`), and nothing more on the next update.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn a_new_career_loads_the_hulls_around_its_start() {
    let mut system = loaded_career();
    let start = location(&system, "J_Gamestart");
    let outputs = update_at(&mut system, start);
    assert_eq!(
        names(&outputs),
        ["PJ03_EndGameUnlock", "PJ03_HotelAlley", "PJ03_SlappysGate", "UR04_Hotel_Pool", "UR05_Community_Pool"],
        "{outputs:#?}"
    );
    assert!(loads(&outputs).iter().all(|&(_, callback, flags)| !callback && flags & 0xef == 0xe0), "{outputs:#?}");
    assert!(names(&update_at(&mut system, start)).is_empty());
}

/// A new state unloads the loaded hull of the old one (Slappy's "Unlock" maps none, so it stays
/// open), and leaving the boxes unloads the rest. The state comes from the progression script's
/// own call, run in the progression state.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn a_state_change_or_leaving_the_box_unloads_the_hull() {
    let mut system = loaded_career();
    let start = location(&system, "J_Gamestart");
    update_at(&mut system, start);
    let ChallengeSystem { progression, world, .. } = &mut system;
    let script = progression.load_source(b"DynamicHull.SetupDynamicHullState('slappys', 'Unlock')", "slappys").unwrap();
    progression.call(world, &script, &[]).unwrap();
    let outputs = system.take_outputs();
    assert_eq!(unloads(&outputs), [assets::hull_id("PJ03_SlappysGate")], "{outputs:#?}");
    assert!(names(&update_at(&mut system, start)).is_empty());
    assert_eq!(dynamic_hull::test_state(system.world(), hash64("slappys"), "Unlock"), Some(true));
    let fun_track = location(&system, "F_SVM_FunTrack");
    let outputs = update_at(&mut system, fun_track);
    assert_eq!(unloads(&outputs).len(), 4, "{outputs:#?}");
    assert_eq!(names(&outputs), ["megapark_extension_gates"], "{outputs:#?}");
}

/// The map input of one press.
fn press(map: &mut ChallengeMap, system: &mut ChallengeSystem, input: MapInput) -> MapAction {
    map.update(system.world_mut(), input)
}

const DOWN: MapInput = MapInput { up: false, down: true, select: false, back: false, switch_list: false };
const SELECT: MapInput = MapInput { up: false, down: false, select: true, back: false, switch_list: false };

/// The career map, opened, down to the Property category and into its first item's info
/// component.
fn open_first_property(system: &mut ChallengeSystem) -> ChallengeMap {
    let mut map = ChallengeMap::open(system.world_mut());
    let rows = map.view(system.world()).rows;
    let row = rows
        .iter()
        .position(|row| *row == MenuText::Category { hal: "ID_STUFFMENU_CATEGORY_PROPERTY", count: 1 })
        .unwrap_or_else(|| panic!("{rows:?}"));
    for _ in 0..row {
        press(&mut map, system, DOWN);
    }
    press(&mut map, system, SELECT);
    press(&mut map, system, SELECT);
    map
}

/// Danny's megapark extension on the career map, once the progression makes it available (its
/// `Progression.ActivateChallengeByTitle` at Circulate Stage One): the Property category lists
/// it and its info component offers it at $50,000 beside the wallet. Short of the price the
/// purchase fails (OK); with it, Yes buys it (`ConfirmPurchaseCallback`) and the map teleports
/// to its `Location`, each menu step sounding as the pickers and the popup state do. The gates
/// unload and stay away, the owned property offers Teleport without a price, and the saved
/// state keeps the gates away in the next game.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn buying_the_megapark_opens_fun_track_and_the_save_keeps_it_open() {
    let mut system = loaded_career();
    let key = hash64("dannys_megapark_extension");
    let fun_track = location(&system, "F_SVM_FunTrack");
    assert_eq!(names(&update_at(&mut system, fun_track)), ["megapark_extension_gates"]);
    system.diagnostic_activate_challenge("dannys_megapark_extension").unwrap();
    let mut map = open_first_property(&mut system);
    map.take_sounds();
    let info = map.view(system.world()).info.expect("info component");
    assert_eq!(info.option, "ID_CIC_PURCHASE_PROPERTY");
    let property = info.property.expect("purchase mode");
    let cash = signpost_progress::cash(system.world());
    assert_eq!((property.price, property.cash), (Some(50_000), Some(cash)));
    assert_eq!(press(&mut map, &mut system, SELECT), MapAction::None);
    assert_eq!(map.view(system.world()).popup, Some((MapPopup::PurchaseFailed, 0)));
    assert_eq!(press(&mut map, &mut system, SELECT), MapAction::None);
    assert_eq!(map.view(system.world()).popup, None);
    system.diagnostic_add_cash(50_000 - cash);
    assert_eq!(press(&mut map, &mut system, SELECT), MapAction::None);
    assert_eq!(map.view(system.world()).popup, Some((MapPopup::PurchaseConfirm, 0)));
    press(&mut map, &mut system, DOWN);
    assert_eq!(map.view(system.world()).popup, Some((MapPopup::PurchaseConfirm, 1)));
    assert_eq!(press(&mut map, &mut system, SELECT), MapAction::TeleportChallenge(key));
    assert_eq!(
        map.take_sounds(),
        [
            "FE_SOUND_CORE_A_BUTTON",
            "FE_SOUND_CORE_POPUP",
            "FE_SOUND_CORE_A_BUTTON",
            "FE_SOUND_CORE_A_BUTTON",
            "FE_SOUND_CORE_POPUP",
            "FE_SOUND_CORE_NAV_DOWN",
            "FE_SOUND_CORE_A_BUTTON",
        ]
    );
    assert_eq!(signpost_progress::cash(system.world()), 0);
    assert_eq!(signpost_progress::challenge_save_data(system.world(), key, "PlayerOwns"), 1);
    let outputs = system.take_outputs();
    assert_eq!(unloads(&outputs), [assets::hull_id("megapark_extension_gates")], "{outputs:#?}");
    assert!(names(&update_at(&mut system, fun_track)).is_empty());
    let mut map = open_first_property(&mut system);
    let info = map.view(system.world()).info.expect("info component");
    assert_eq!(info.option, "ID_COMMON_TELEPORT");
    assert_eq!(info.property.map(|property| (property.price, property.cash)), Some((None, None)));
    assert_eq!(press(&mut map, &mut system, SELECT), MapAction::TeleportChallenge(key));
    let saved = system.world().dynamic_hulls.save();

    let mut next = career_system();
    load_locations(&mut next);
    next.world_mut().dynamic_hulls.load(Some((saved[0].1, &saved[0].2)));
    next.game_load();
    assert!(names(&update_at(&mut next, fun_track)).is_empty());
    assert_eq!(dynamic_hull::test_state(next.world(), hash64("dannys_megapark_extension"), "Unlocked"), Some(true));
}

/// The other property the native sells, Dyrdek's skatepark (`Dyrdek's Skatepark`, $200,000),
/// goes through the same screens: listed once the progression makes it available, offered at its
/// price, bought with Yes, then offered as a teleport.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn dyrdeks_skatepark_sells_through_the_map_too() {
    let mut system = loaded_career();
    let key = hash64("dyrdek_skatepark");
    system.diagnostic_activate_challenge("Dyrdek's Skatepark").unwrap();
    let mut map = open_first_property(&mut system);
    let property = map.view(system.world()).info.and_then(|info| info.property).expect("purchase mode");
    assert_eq!(property.price, Some(200_000));
    let cash = signpost_progress::cash(system.world());
    system.diagnostic_add_cash(200_000 - cash);
    assert_eq!(press(&mut map, &mut system, SELECT), MapAction::None);
    assert_eq!(map.view(system.world()).popup, Some((MapPopup::PurchaseConfirm, 0)));
    press(&mut map, &mut system, DOWN);
    assert_eq!(press(&mut map, &mut system, SELECT), MapAction::TeleportChallenge(key));
    assert_eq!(signpost_progress::challenge_save_data(system.world(), key, "PlayerOwns"), 1);
    assert_eq!(dynamic_hull::test_state(system.world(), key, "Unlocked"), Some(true));
    assert_eq!(map.view(system.world()).info.map(|info| info.option), Some("ID_COMMON_TELEPORT"));
}

/// `SetCappingOnJacked`: a jack caps the owner's listed rails nearest the skater again
/// (downtown's `dt_2` lists three). The owners with cap lists spawn no characters, and the
/// challenges that spawn characters (the only pedestrians with a `mChallengeOwner`) list no caps.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn a_jack_caps_the_listed_rails_nearest_the_skater() {
    let mut system = loaded_career();
    let caps = ["dt_2_0", "dt_2_1", "dt_2_2"];
    for cap in caps {
        dynamic_hull::set_state(system.world_mut(), hash64(cap), "Uncapped", None);
    }
    let rails = stuff_menu::challenge_location(system.world(), hash64("dt_2_1")).expect("dt_2_1 location");
    let at = location(&system, &rails);
    let owner = system.world().activities.find(hash64("dt_2")).expect("dt_2");
    dynamic_hull::set_capping_on_jacked(system.world_mut(), owner, at);
    let capped = caps.map(|cap| dynamic_hull::test_state(system.world(), hash64(cap), "Capped"));
    assert_eq!(capped, [Some(false), Some(true), Some(false)]);

    let world = system.world();
    let listed = |class: u64, collection: u64, field: u64| world.db.attribute(class, collection, field).is_some_and(|a| !a.is_empty());
    let mut owners = Vec::new();
    for activity in &world.activities.activities {
        let caps = activity.local_data.is_some_and(|(c, k)| listed(c, k, dynamic_hull::CAP_LIST));
        let characters = listed(keys::class::CHALLENGES, activity.key, characters::key::CHARACTERS);
        assert!(!(caps && characters), "{}", activity.name);
        if caps {
            owners.push(activity.name.as_str());
        }
    }
    owners.sort_unstable();
    assert_eq!(owners, ["downtown_artdeco", "downtown_tiered", "dt_2", "dt_9_theatre", "dt_madridplaza", "ot_01_cityhall"]);
}

/// Free skate (game type 2) loads only the hulls whose load mask has it, in the state the
/// profile's location unlock gives (`GetDynamicHullUnlockValue`); Party Play (16) loads none.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn free_skate_reads_the_location_unlocks_and_party_play_loads_nothing() {
    let free_skate = |unlock: Option<&str>| {
        let mut system = career_system();
        load_locations(&mut system);
        if let Some(name) = unlock {
            system.world_mut().unlockables.unlock(unlockables::category::LOCATIONS, unlockables::string_hash(name), false);
        }
        system.world_mut().game_mode_flags = 2;
        dynamic_hull::game_load(system.world_mut(), 2);
        system
    };
    let mut locked = free_skate(None);
    let fun_track = location(&locked, "F_SVM_FunTrack");
    assert_eq!(names(&update_at(&mut locked, fun_track)), ["megapark_extension_gates"]);
    // The career's Slappy's gate has no free-skate bit.
    let start = location(&locked, "J_Gamestart");
    assert_eq!(names(&update_at(&mut locked, start)), ["PJ03_EndGameUnlock"]);
    let mut unlocked = free_skate(Some("megapark"));
    assert!(names(&update_at(&mut unlocked, fun_track)).is_empty());

    let mut party = career_system();
    load_locations(&mut party);
    party.game_load_party_play();
    assert!(names(&update_at(&mut party, fun_track)).is_empty());
    assert!(names(&update_at(&mut party, start)).is_empty());
}

/// An online game (`GenerateGameModeSignature` 8) loads no gate whatever the profile has
/// unlocked, so Fun Track and Slappy's alley are open online.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn a_network_game_loads_no_gate() {
    let mut online = career_system();
    load_locations(&mut online);
    online.game_load_network_dynamic_hulls();
    let fun_track = location(&online, "F_SVM_FunTrack");
    assert!(names(&update_at(&mut online, fun_track)).is_empty());
    let start = location(&online, "J_Gamestart");
    assert!(names(&update_at(&mut online, start)).is_empty());
}

/// Presentation event 33 (`m_2b_01_goto_permission_pool`'s goto): the group waits for the new
/// state's hull, loaded with the director's callback, and plays on when the loader answers.
/// The gate's locator is also inside three pools' boxes.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_set_state_presentation_event_waits_for_its_hull() {
    let mut system = loaded_career();
    let gate = location(&system, "m_2b_01_goto_gate_locator");
    assert_eq!(
        names(&update_at(&mut system, gate)),
        ["PJ01_SchoolPool", "UR04_Hotel_Pool", "UR05_Community_Pool", "m_2b_01_goto_gate_closed"]
    );
    let world = system.world_mut();
    let handle = presentation::add_group(world, hash64("m_2b_01_goto_permission_pool"));
    let group = world.presentation.groups.iter().find(|g| g.handle == handle).map(|g| g.id).expect("group");
    let set = PresentationEvent { kind: event::SET_DYNAMIC_HULL_STATE, arg1: "permission_pool_gate".into(), arg2: "NIS".into() };
    let complete = PresentationEvent { kind: event::COMPLETE_CHALLENGE, arg1: "no_such_challenge".into(), arg2: String::new() };
    presentation::add_event(world, group, set);
    presentation::add_event(world, group, complete);
    presentation::start(world, group);
    let outputs = system.take_outputs();
    assert_eq!(unloads(&outputs), [assets::hull_id("m_2b_01_goto_gate_closed")], "{outputs:#?}");
    let counter = |system: &ChallengeSystem| system.world().presentation.groups.iter().find(|g| g.id == group).map(|g| g.counter);
    assert_eq!(counter(&system), Some(1));
    let outputs = update_at(&mut system, gate);
    assert_eq!(loads(&outputs), [("m_2b_01_goto_gate_NIS".to_owned(), true, 0xe0)], "{outputs:#?}");
    assert_eq!(counter(&system), Some(1));
    system.dynamic_hull_load_complete(assets::hull_id("m_2b_01_goto_gate_NIS"));
    // The completion event ran and the group finished.
    assert_eq!(counter(&system), Some(0));
}
