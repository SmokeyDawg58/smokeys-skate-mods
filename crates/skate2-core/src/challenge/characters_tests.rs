use super::*;
use crate::challenge::{
    ChallengeSystem,
    attrib_db::{AttribDb, CAREER_VAULTS},
    keys::{hash_value32, hash64},
    living_world,
};
use skate2_formats::{
    location::LocationTable,
    mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
};
use std::sync::{Arc, Mutex};

fn mounts(paths: &[&str]) -> MountSet {
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let mut mounts = MountSet::new();
    for path in paths {
        let archive = SKATE2_ARCHIVES
            .iter()
            .find(|a| a.relative_path == *path)
            .unwrap();
        mounts.push(
            MountedBig::open(
                archive.path_under(std::path::Path::new(&root)),
                archive.use_external_header,
            )
            .unwrap(),
        );
    }
    mounts
}

/// The career's challenge system with the `BAM_Sim.loc` locations, as the career session
/// builds it.
fn career_system() -> ChallengeSystem {
    let db = mounts(&[
        "data\\db\\db.big",
        "data\\big\\miscload.big",
        "data\\content\\createacharacter.big",
    ]);
    let global = db.read_path("data\\script\\challenge\\global.lua").unwrap();
    let mut system = ChallengeSystem::new_career(
        Arc::new(AttribDb::load(&db, &CAREER_VAULTS).unwrap()),
        &global,
    )
    .unwrap();
    let world = mounts(&[
        "data\\content\\worldbam.big",
        "data\\big\\globallocators.big",
        "data\\big\\miscload.big",
    ]);
    let table = LocationTable::parse(
        &world
            .read_path("data\\content\\world\\stream\\BAM\\BAM_Sim.loc")
            .unwrap(),
    )
    .unwrap();
    for location in &table.locations {
        system
            .world_mut()
            .locations
            .insert(hash_value32(&location.name), location.transform);
    }
    system
}

/// A living world whose waypoint groups the test adds.
struct Facts {
    waypoints: Arc<Mutex<BTreeMap<u64, [f32; 4]>>>,
    tracked: bool,
}

impl LivingWorldFacts for Facts {
    fn waypoint_location(&self, guid: u64) -> Option<[f32; 4]> {
        self.waypoints.lock().unwrap().get(&guid).copied()
    }
    fn is_model_tracked(&self, _: u64) -> bool {
        self.tracked
    }
    fn chase_is_on(&self) -> bool {
        false
    }
}

fn activity(world: &World, name: &str) -> usize {
    world.activities.find(hash64(name)).expect("activity")
}

fn position(world: &World, name: &str) -> [f32; 4] {
    let t = location(world, name).expect("location");
    [t[12], t[13], t[14], t[15]]
}

fn at(position: [f32; 4]) -> OctreeBounds {
    tester_bounds([position[0], position[1], position[2]])
}

/// Two tester tests at the entry: the first submits its ground test, the second (its answer a
/// hit 0.25 below the location) spawns it. Returns the spawn's ops.
fn place(world: &mut World, tester: [f32; 4], ground: [f32; 4]) -> Vec<LivingWorldOp> {
    test_bbox_against_tree(world, at(tester));
    let ops = take_ops(world);
    let [LivingWorldOp::LineTests { serial, tests }] = ops.as_slice() else {
        panic!("{ops:?}")
    };
    assert_eq!(tests.len(), 1);
    line_tests_answered(
        world,
        *serial,
        vec![Some(LineHit {
            position: ground,
            owner: None,
        })],
    );
    test_bbox_against_tree(world, at(tester));
    take_ops(world)
}

#[test]
fn spawn_tree_reuses_the_latest_freed_slot() {
    let mut tree = SpawnTree::default();
    let bounds = OctreeBounds {
        minimum: [0.0; 4],
        maximum: [1.0, 1.0, 1.0, 0.0],
    };
    let info = SpawnInfo::Waypoint {
        entity: (LIVINGWORLD_ENTITIES, 1),
        waypoint: 1,
        on_start: on_start::KEEP,
    };
    for character in 0..3 {
        tree.insert(SpawnData::new(0, character, info.clone()), bounds);
    }
    let mut previous = vec![0, 1, 2];
    tree.remove(0, &mut previous);
    tree.remove(2, &mut previous);
    assert_eq!(previous, [1]);
    assert_eq!(tree.insert(SpawnData::new(1, 0, info.clone()), bounds), 2);
    assert_eq!(tree.insert(SpawnData::new(1, 1, info), bounds), 0);
    assert_eq!(tree.find(1, 1), Some(0));
    assert_eq!(tree.collisions(bounds), [0, 1, 2]);
}

#[test]
fn tester_box_is_the_trigger_character_box() {
    let bounds = tester_bounds([1.0, 2.0, 3.0]);
    assert_eq!(bounds.minimum, [1.0 - 0.29, 2.0, 3.0 - 0.29, 0.0]);
    assert_eq!(bounds.maximum, [1.0 + 0.29, 2.0 + 1.74, 3.0 + 0.29, 0.0]);
    assert!(is_zero([1.0e-7, -1.0e-7, 0.0, 5.0]));
    assert!(!is_zero([0.0, 2.0e-7, 0.0, 0.0]));
}

/// `m_1a_07_follow_slappy`'s Slappy: his entry's box is the pedestrian census's larger cull
/// radius plus ten metres (a marquee model); the tester's first visit tests the ground, the next
/// spawns him there (loading his model first), and leaving culls him and unloads the model.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn slappy_spawns_on_the_ground_and_goes_when_the_tester_leaves() {
    let mut system = career_system();
    let world = system.world_mut();
    let follow = activity(world, "m_1a_07_follow_slappy");
    assert!(world.characters.all.contains(&follow));
    add_activity_entries(world, follow);
    let slappy = (LIVINGWORLD_ENTITIES, hash64("slappy"));
    let entries: Vec<(u32, SpawnData)> = world
        .characters
        .entries()
        .map(|(i, d)| (i, d.clone()))
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].1.info,
        SpawnInfo::Location {
            entity: slappy,
            location: Some("m_1a_07_follow_slappy_challengelocator01".to_owned()),
            on_start: on_start::REMOVE,
            // The shipped AlternateLocation is an empty string, not absent.
            alternate: Some(String::new()),
        }
    );
    let model = entity_model(world, slappy);
    assert_eq!(model, 0x1997_931f_1aa0_4f1f);
    assert_eq!(model_category(world, model), MARQUEE_CATEGORY);
    let locator = position(world, "m_1a_07_follow_slappy_challengelocator01");
    let bounds = world.characters.tree.entries[0].as_ref().unwrap().1;
    assert_eq!(
        (bounds.minimum[0], bounds.maximum[1]),
        (locator[0] - 100.0, locator[1] + 100.0)
    );

    test_bbox_against_tree(world, at(locator));
    let ops = take_ops(world);
    let fatness = f32::from_bits(0x3ea8_f5c3);
    let start = [locator[0], locator[1] + 1.0, locator[2], locator[3]];
    let end = [locator[0], locator[1] - 2.5, locator[2], locator[3]];
    assert_eq!(
        ops,
        [LivingWorldOp::LineTests {
            serial: 0,
            tests: vec![(start, end, fatness)]
        }]
    );
    assert_eq!(world.characters.entry(0).unwrap().line_test, 0);

    let ground = [locator[0], locator[1] - 0.25, locator[2], locator[3]];
    line_tests_answered(
        world,
        0,
        vec![Some(LineHit {
            position: ground,
            owner: None,
        })],
    );
    test_bbox_against_tree(world, at(locator));
    let mut transform = rows(&location(world, "m_1a_07_follow_slappy_challengelocator01").unwrap());
    transform[3] = ground;
    assert_eq!(
        take_ops(world),
        [LivingWorldOp::Spawn {
            token: 0,
            entity: slappy.1,
            transform,
            model,
            load_model: true,
            spawn: true,
            waypoint: None,
            challenge_flags: false,
        }]
    );
    assert_eq!(
        world.characters.entry(0).unwrap().entity,
        Some(CharacterRef::Pending(0))
    );

    spawn_answered(world, 0, Some(true), Some((42, 0xabc)));
    let entry = world.characters.entry(0).unwrap();
    assert_eq!(
        (entry.entity, entry.recipe, entry.requested_model_load),
        (Some(CharacterRef::Spawned(42)), 0xabc, true)
    );
    assert!(is_challenge_character(world, 42));
    assert_eq!(world.characters.recipes, [(0xabc, 0)]);

    test_bbox_against_tree(world, at(locator));
    assert!(take_ops(world).is_empty());

    test_bbox_against_tree(
        world,
        at([locator[0] + 500.0, locator[1], locator[2], locator[3]]),
    );
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::RequestExplicitCull(CharacterRef::Spawned(42)),
            LivingWorldOp::UnloadModel { model, after: None }
        ]
    );
    let entry = world.characters.entry(0).unwrap();
    assert_eq!(
        (
            entry.entity,
            entry.spawn_position,
            entry.requested_model_load
        ),
        (None, [0.0; 4], false)
    );
    assert!(!is_challenge_character(world, 42));
    assert!(world.characters.recipes.is_empty());
    assert!(world.characters.previous_collisions().is_empty());
}

/// A character released before the session answered its spawn: its cull names the pending
/// pedestrian and its model unload waits on that spawn's load.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn a_release_before_the_answer_names_the_pending_spawn() {
    let mut system = career_system();
    let world = system.world_mut();
    let follow = activity(world, "m_1a_07_follow_slappy");
    add_activity_entries(world, follow);
    let locator = position(world, "m_1a_07_follow_slappy_challengelocator01");
    let ops = place(world, locator, locator);
    assert!(
        matches!(
            ops.as_slice(),
            [LivingWorldOp::Spawn {
                token: 0,
                load_model: true,
                ..
            }]
        ),
        "{ops:?}"
    );
    let model = entity_model(world, (LIVINGWORLD_ENTITIES, hash64("slappy")));
    test_bbox_against_tree(
        world,
        at([locator[0], locator[1] + 500.0, locator[2], locator[3]]),
    );
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::RequestExplicitCull(CharacterRef::Pending(0)),
            LivingWorldOp::UnloadModel {
                model,
                after: Some(0)
            }
        ]
    );
    spawn_answered(world, 0, Some(true), Some((9, 0xabc)));
    assert!(!is_challenge_character(world, 9));
    assert_eq!(world.characters.entry(0).unwrap().entity, None);
}

/// `cMsgAssetSimDRegLWSchema` of a character's recipe releases it.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn a_recipe_deregistration_releases_its_characters() {
    let mut system = career_system();
    let world = system.world_mut();
    let follow = activity(world, "m_1a_07_follow_slappy");
    add_activity_entries(world, follow);
    let locator = position(world, "m_1a_07_follow_slappy_challengelocator01");
    place(world, locator, locator);
    spawn_answered(world, 0, Some(true), Some((42, 0xabc)));
    let model = entity_model(world, (LIVINGWORLD_ENTITIES, hash64("slappy")));
    receive_dreg_lw_schema(world, 0xdef);
    assert!(take_ops(world).is_empty());
    receive_dreg_lw_schema(world, 0xabc);
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::RequestExplicitCull(CharacterRef::Spawned(42)),
            LivingWorldOp::UnloadModel { model, after: None }
        ]
    );
    assert_eq!(world.characters.entry(0).unwrap().entity, None);
}

/// `m_1a_06_photo_puzzle`'s Atiba moves to his alternate location when his challenge starts:
/// the challenge flags his pro challenge started, culls him to spawn again at
/// `m_1a_06_photo_puzzle_startlocator01` taking pictures, and its completion culls him back.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_photographer_moves_to_his_alternate_location() {
    let mut system = career_system();
    let world = system.world_mut();
    // Neither locator is in BAM_Sim.loc: the session's location manager answers them once the
    // area's arenas stream in.
    let placed = |x: f32, z: f32| [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., x, 100.82, z, 1.];
    world.host = Box::new(Locations(vec![
        (hash_value32("m_1a_06_photo_puzzle_challengelocator01"), placed(1633.17, -531.83)),
        (hash_value32("m_1a_06_photo_puzzle_startlocator01"), placed(1645.17, -531.83)),
    ]));
    install_facts(
        world,
        Box::new(Facts {
            waypoints: Arc::default(),
            tracked: true,
        }),
    );
    let puzzle = activity(world, "m_1a_06_photo_puzzle");
    add_activity_entries(world, puzzle);
    let atiba = (LIVINGWORLD_ENTITIES, hash64("attiba_jefferson"));
    assert_eq!(
        world.characters.entry(0).unwrap().info,
        SpawnInfo::Location {
            entity: atiba,
            location: Some("m_1a_06_photo_puzzle_challengelocator01".to_owned()),
            on_start: on_start::ALTERNATE_LOCATION,
            alternate: Some("m_1a_06_photo_puzzle_startlocator01".to_owned()),
        }
    );
    let locator = position(world, "m_1a_06_photo_puzzle_challengelocator01");
    place(world, locator, locator);
    spawn_answered(world, 0, Some(true), Some((5, 0xabc)));

    world.activities.main = Some(puzzle);
    force_cull_challenge_spawned_living_world(world, puzzle);
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::StateFlags {
                character: CharacterRef::Spawned(5),
                set: [0, 0x40],
                clear: [0, 0]
            },
            LivingWorldOp::RequestExplicitCull(CharacterRef::Spawned(5)),
        ]
    );
    let entry = world.characters.entry(0).unwrap();
    assert!(entry.use_alternate_location && entry.entity.is_none() && entry.requested_model_load);

    let alternate = position(world, "m_1a_06_photo_puzzle_startlocator01");
    test_bbox_against_tree(world, at(locator));
    let ops = take_ops(world);
    let [LivingWorldOp::LineTests { serial, tests }] = ops.as_slice() else {
        panic!("{ops:?}")
    };
    assert_eq!(
        tests[0].0,
        [alternate[0], alternate[1] + 1.0, alternate[2], alternate[3]]
    );
    line_tests_answered(
        world,
        *serial,
        vec![Some(LineHit {
            position: alternate,
            owner: None,
        })],
    );
    test_bbox_against_tree(world, at(locator));
    let ops = take_ops(world);
    let [
        LivingWorldOp::Spawn {
            token: 1,
            load_model: false,
            spawn: true,
            challenge_flags: true,
            transform,
            ..
        },
    ] = ops.as_slice()
    else {
        panic!("{ops:?}")
    };
    assert_eq!(transform[3], alternate);
    spawn_answered(world, 1, None, Some((6, 0xabc)));

    complete_challenge(world, puzzle, true);
    assert_eq!(
        take_ops(world),
        [LivingWorldOp::RequestExplicitCull(CharacterRef::Spawned(6))]
    );
    let entry = world.characters.entry(0).unwrap();
    assert!(!entry.use_alternate_location && entry.entity.is_none());
}

/// `dt_2`'s guards wait for their waypoint groups: a group's arrival adds its entry to the
/// tree on the next update, the guard spawns at its first waypoint, and a later arrival of the
/// group puts the spawned guard on it.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn waypoint_characters_wait_for_their_group() {
    let mut system = career_system();
    let world = system.world_mut();
    let waypoints = Arc::new(Mutex::new(BTreeMap::new()));
    install_facts(
        world,
        Box::new(Facts {
            waypoints: Arc::clone(&waypoints),
            tracked: false,
        }),
    );
    world.characters.game_loaded = true;
    let dt = activity(world, "dt_2");
    add_activity_entries(world, dt);
    assert_eq!(world.characters.pending().len(), 4);
    assert_eq!(world.characters.entries().count(), 0);

    let guid = 0x0007_14f4_03e3_8706;
    let point = [100.0, 20.0, -50.0, 1.0];
    waypoints.lock().unwrap().insert(guid, point);
    receive_waypoint_group_added(world, guid);
    assert!(take_ops(world).is_empty());
    update(world);
    assert_eq!(world.characters.pending().len(), 3);
    assert_eq!(world.characters.waypoints.get(&guid), Some(&0));

    let ops = place(world, point, point);
    let [
        LivingWorldOp::Spawn {
            token: 0,
            entity,
            waypoint: Some(group),
            load_model: false,
            transform,
            ..
        },
    ] = ops.as_slice()
    else {
        panic!("{ops:?}")
    };
    assert_eq!(
        (*entity, *group, transform[3]),
        (0x8894_6253_76ee_1158, guid, point)
    );
    spawn_answered(world, 0, None, Some((11, 0xabc)));
    receive_waypoint_group_added(world, guid);
    assert_eq!(
        take_ops(world),
        [LivingWorldOp::SetCurrentWaypoint {
            character: CharacterRef::Spawned(11),
            waypoint: guid
        }]
    );
}

/// `c_1a_01_bowl_series` (`LivingWorldManagement` 0x11) keeps only the census spawner and the
/// dynamic object spawner on while it runs; its completion turns every spawner back on.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn management_values_set_the_census_spawners() {
    let mut system = career_system();
    let world = system.world_mut();
    living_world::game_load(world, 4);
    let bowl = activity(world, "c_1a_01_bowl_series");
    living_world::run_main_activity(world, bowl);
    assert_eq!(
        take_ops(world),
        [LivingWorldOp::CensusEnable {
            mask: 0x7e,
            value: 0x44
        }]
    );
    living_world::complete_challenge(world, true);
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::CensusEnable {
                mask: 0x7e,
                value: 0x7e
            },
            LivingWorldOp::AvoidVolumesCleared
        ]
    );
}

/// `m_3a_01_pro_duffy` culls every census-managed character; `sponsor_me_03` clears its
/// boundary and keeps the census out of it until it completes, and a restart runs the
/// operations again, skipping the next initial spawn.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn living_world_operations_cull_and_avoid() {
    let mut system = career_system();
    let world = system.world_mut();
    living_world::game_load(world, 4);
    let duffy = activity(world, "m_3a_01_pro_duffy");
    living_world::run_main_activity(world, duffy);
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::CensusEnable {
                mask: 0x7e,
                value: 0x7e
            },
            LivingWorldOp::CensusManagedCullAll { keep: Vec::new() }
        ]
    );
    let sponsor = activity(world, "sponsor_me_03");
    let volume = 0x0005_74fc_03e3_8706;
    living_world::run_main_activity(world, sponsor);
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::CensusEnable {
                mask: 0x7e,
                value: 0x7e
            },
            LivingWorldOp::CensusManagedCullVolume {
                volume,
                keep: Vec::new()
            },
            LivingWorldOp::AvoidVolumeAdded(volume),
        ]
    );
    assert_eq!(world.living_world.avoid_volumes(), [volume]);
    living_world::restart_challenge(world, sponsor);
    assert_eq!(
        take_ops(world),
        [LivingWorldOp::CensusManagedCullVolume {
            volume,
            keep: Vec::new()
        }]
    );
    assert!(world.living_world.skip_initial_spawn());
    living_world::complete_challenge(world, true);
    take_ops(world);
    assert!(world.living_world.avoid_volumes().is_empty());
}

/// The session's `cLocationManager` with the given registered locations.
struct Locations(Vec<(u32, [f32; 16])>);

impl crate::challenge::host::Host for Locations {
    fn dmo_locator_from_lua_id(&mut self, _: f32) -> Option<u64> {
        None
    }
    fn is_dmo_in_volume(&mut self, _: u64, _: f32) -> bool {
        false
    }
    fn trigger_volume_center(&mut self, _: f32) -> Option<[f32; 3]> {
        None
    }
    fn location_orientation(&self, location: u32) -> Option<[f32; 16]> {
        self.0.iter().find(|(hash, _)| *hash == location).map(|(_, transform)| *transform)
    }
}
