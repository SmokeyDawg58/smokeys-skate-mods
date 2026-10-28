//! The challenge icons on the minimap and the career map between missions, from the shipped
//! data.

use super::super::{
    keys::{self, class},
    map_icons,
    stuff_menu::{self, ChallengeMap, Letterbox, MapAction, MapInput, MenuText},
};
use super::{
    ChallengeSystem, after_first_mission, after_fliptricks, career_system, load_locations,
};

const FLIPTRICKS: &str = "m_1a_03_tutorial_fliptricks";
const FLIPTRICKS_LOCATION: &str = "m_1a_03_tutorial_fliptricks_challengelocator01";

fn title(system: &ChallengeSystem, name: &str) -> String {
    let db = &system.world.db;
    db.attribute(class::CHALLENGES, keys::hash64(name), keys::hash64("Title"))
        .and_then(|a| a.text(0))
        .expect("challenge title")
        .to_owned()
}

fn press(map: &mut ChallengeMap, system: &mut ChallengeSystem, input: MapInput) -> MapAction {
    map.update(system.world_mut(), input)
}

/// The icons the map draws: the map-visible ones, by challenge, with their map scale.
fn shown(system: &ChallengeSystem) -> Vec<(u64, f32)> {
    system
        .world
        .map_icons
        .icons
        .iter()
        .filter(|icon| icon.challenge_map_visible)
        .map(|icon| (icon.challenge_key, icon.map_scale))
        .collect()
}

/// After the first mission, `m_1a_03`'s `ChallengeIcon` (`mini_tutorial`) is on the minimap at
/// its `Location`, and the completed `m_1a_01` (type 0) has no icon.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_next_mission_shows_on_the_minimap() {
    let system = after_first_mission();
    let world = &system.world;
    let icons: Vec<_> = world.map_icons.back_end_icons().collect();
    let fliptricks = icons
        .iter()
        .find(|icon| icon.challenge_key == keys::hash64(FLIPTRICKS))
        .expect("m_1a_03's icon");
    assert_eq!(fliptricks.texture, "mini_tutorial.Texture");
    assert!(fliptricks.minimap_visible && fliptricks.visible);
    assert!(!fliptricks.pinch_point);
    let locator = world.locations[&keys::hash_value32(FLIPTRICKS_LOCATION)];
    assert_eq!(fliptricks.position, [locator[12], locator[14]]);
    assert!(
        !icons
            .iter()
            .any(|icon| icon.challenge_key == keys::hash64("m_1a_01_goto_slappys"))
    );
    let minimap = map_icons::minimap_icons(world);
    assert!(
        minimap
            .iter()
            .any(|icon| icon.texture == "mini_tutorial.Texture"
                && icon.visible
                && icon.position == fliptricks.position),
        "{minimap:?}"
    );
}

/// The career map after the first mission: the root lists the tutorials (one), highlighting
/// them shows `m_1a_03`'s icon, selecting them lists it, its highlight draws the icon twice as
/// large, its info component's Teleport goes to its `Location`, and B goes back to the root, then
/// closes the map.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_career_map_lists_and_shows_the_next_mission() {
    let mut system = after_first_mission();
    let key = keys::hash64(FLIPTRICKS);
    let mut map = ChallengeMap::open(system.world_mut());
    let view = map.view(&system.world);
    assert_eq!(
        view.title,
        MenuText::Copied(stuff_menu::ROOT_HAL.to_owned())
    );
    assert_eq!(view.letterbox, Letterbox::Basic);
    let tutorials = MenuText::Category {
        hal: "ID_STUFFMENU_CATEGORY_TUTORIAL",
        count: 1,
    };
    let row = view
        .rows
        .iter()
        .position(|row| *row == tutorials)
        .unwrap_or_else(|| panic!("{:?}", view.rows));
    for _ in 0..row {
        press(
            &mut map,
            &mut system,
            MapInput {
                down: true,
                ..MapInput::default()
            },
        );
    }
    assert_eq!(shown(&system), [(key, 1.0)]);
    press(
        &mut map,
        &mut system,
        MapInput {
            select: true,
            ..MapInput::default()
        },
    );
    let view = map.view(&system.world);
    assert_eq!(view.title, tutorials);
    assert_eq!(view.rows, [MenuText::Formatted(title(&system, FLIPTRICKS))]);
    assert_eq!(shown(&system), [(key, 2.0)]);
    let drawn: Vec<_> = view
        .icons
        .iter()
        .map(|icon| (icon.texture.as_str(), icon.size))
        .collect();
    assert_eq!(drawn, [("mini_tutorial.Texture", 64.0)]);
    press(
        &mut map,
        &mut system,
        MapInput {
            select: true,
            ..MapInput::default()
        },
    );
    let info = map.view(&system.world).info.expect("info component");
    assert_eq!(info.title, MenuText::Formatted(title(&system, FLIPTRICKS)));
    assert_eq!(info.option, "ID_COMMON_TELEPORT");
    assert_eq!(
        press(
            &mut map,
            &mut system,
            MapInput {
                select: true,
                ..MapInput::default()
            }
        ),
        MapAction::TeleportChallenge(key)
    );
    assert_eq!(
        stuff_menu::challenge_location(&system.world, key).as_deref(),
        Some(FLIPTRICKS_LOCATION)
    );
    for _ in 0..2 {
        assert_eq!(
            press(
                &mut map,
                &mut system,
                MapInput {
                    back: true,
                    ..MapInput::default()
                }
            ),
            MapAction::None
        );
    }
    let view = map.view(&system.world);
    assert!(view.info.is_none());
    assert_eq!(
        view.title,
        MenuText::Copied(stuff_menu::ROOT_HAL.to_owned())
    );
    assert_eq!(
        press(
            &mut map,
            &mut system,
            MapInput {
                back: true,
                ..MapInput::default()
            }
        ),
        MapAction::Close
    );
}

/// `AddChallengeObjectIcon` takes the last `fe_map_icons` collection of the name in
/// `GetCollections` order, the class table's slot order: for presentation event 30's
/// `mini_teleport`, the map-only one, which `AddIcon` still shows on the minimap. Removing the
/// presentation's icons removes it.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_presentation_icon_is_the_tables_last_of_its_name() {
    let mut system = career_system();
    load_locations(&mut system);
    assert_eq!(
        system.world.db.table_order(class::MAP_ICON).first(),
        Some(&0x54b7_fd32_bc1a_6a3f)
    );
    let world = system.world_mut();
    map_icons::add_presentation_icon(world, "mini_teleport", FLIPTRICKS_LOCATION);
    let teleport = |world: &super::super::system::World| {
        world
            .map_icons
            .back_end_icons()
            .find(|icon| icon.texture == "mini_teleport.Texture")
            .map(|icon| (icon.icon_key, icon.minimap_visible))
    };
    assert_eq!(teleport(world), Some((0x3a6e_f9fb_9b03_81cf, true)));
    map_icons::remove_all_presentation_icons(world);
    assert_eq!(teleport(world), None);
}

/// After `m_1a_03`, `m_1a_05`'s icon (`mini_smag`) is a pulsing pinch point: its
/// `progression_pinchpoints` collection holds it alone, threshold 1, pulsing.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_photo_rail_icon_is_a_pulsing_pinch_point() {
    let system = after_fliptricks();
    let key = keys::hash64("m_1a_05_photo_rail");
    let icon = system
        .world
        .map_icons
        .back_end_icons()
        .find(|icon| icon.challenge_key == key)
        .cloned()
        .expect("m_1a_05's icon");
    assert_eq!(icon.texture, "mini_smag.Texture");
    assert!(icon.pinch_point && !icon.pinch_point_non_pulsing);
    assert!(icon.minimap_visible);
}
