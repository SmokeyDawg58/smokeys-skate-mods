use super::*;
use crate::challenge::{
    ChallengeSystem,
    attrib_db::{AttribDb, CAREER_VAULTS},
    characters::take_ops,
    keys::hash64,
};
use crate::living_world_grind_services::dmo_objects::DYNAMIC_OBJECTS_KEY;
use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
use std::sync::Arc;

/// The career's challenge system, as the career session builds it.
fn system() -> ChallengeSystem {
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let mut mounts = MountSet::new();
    for path in [
        "data\\db\\db.big",
        "data\\big\\miscload.big",
        "data\\content\\createacharacter.big",
    ] {
        let archive = SKATE2_ARCHIVES
            .iter()
            .find(|a| a.relative_path == path)
            .unwrap();
        mounts.push(
            MountedBig::open(
                archive.path_under(std::path::Path::new(&root)),
                archive.use_external_header,
            )
            .unwrap(),
        );
    }
    let global = mounts
        .read_path("data\\script\\challenge\\global.lua")
        .unwrap();
    ChallengeSystem::new_career(
        Arc::new(AttribDb::load(&mounts, &CAREER_VAULTS).unwrap()),
        &global,
    )
    .unwrap()
}

fn activity(world: &World, name: &str) -> usize {
    world.activities.find(hash64(name)).expect("activity")
}

/// Party Play's S.K.A.T.E. at the snakes saves and clears its volume's dynamic objects, places
/// the challenge DMO groups and takes its census range; its completion puts the range back and
/// restores the saved locators.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn party_play_runs_the_dmo_operations_with_the_groups() {
    let mut system = system();
    let world = system.world_mut();
    game_load(world, 16);
    let snakes = activity(world, "t_5b_01_skate_snakes");
    run_main_activity(world, snakes);
    let volume = 0x0007_21d2_03e3_8706;
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::DmoReset {
                kind: reset_type::SAVE,
                volume
            },
            LivingWorldOp::AddChallengeDmos {
                treat_as_static: false
            },
            LivingWorldOp::DynamicObjectsCensusRange(Some(DYNAMIC_OBJECTS_KEY)),
        ]
    );
    complete_challenge(world, snakes, true);
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::DynamicObjectsCensusRange(None),
            LivingWorldOp::UnwindDmoReset {
                kind: reset_type::SAVE,
                volume
            },
        ]
    );
}

/// A career challenge's start only sets its census range (its operations wait for its sign-up);
/// a Party Play location's `DmoTreatAsStatic` reaches the placement of its groups.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn career_start_sets_only_the_census_range() {
    let mut system = system();
    let world = system.world_mut();
    game_load(world, 4);
    let barbee = activity(world, "m_4i_01_spotfilm_barbee");
    run_main_activity(world, barbee);
    let [LivingWorldOp::DynamicObjectsCensusRange(Some(range))] = take_ops(world)[..] else {
        panic!("one census range");
    };
    assert_ne!(range, DYNAMIC_OBJECTS_KEY);
    let mut system = self::system();
    let world = system.world_mut();
    game_load(world, 16);
    let stairset = activity(world, "f_1a_36_ots_wf08_stairset_ol");
    run_main_activity(world, stairset);
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::AddChallengeDmos {
                treat_as_static: true
            },
            LivingWorldOp::DynamicObjectsCensusRange(Some(DYNAMIC_OBJECTS_KEY)),
        ]
    );
}

/// A career sign-up's acceptance runs the challenge's operations with its groups
/// (`OnChallengeAccept`), then the sign-up waits three updates for the objects
/// (`IsChallengeAcceptOperationDone`).
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn career_acceptance_runs_the_operations_then_waits_three_updates() {
    let mut system = system();
    let world = system.world_mut();
    game_load(world, 4);
    let bowl = activity(world, "c_1a_01_bowl_series");
    on_challenge_accept(world, bowl);
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::DmoReset {
                kind: reset_type::CLEAR,
                volume: 0x0006_4fe7_03e3_8706
            },
            LivingWorldOp::AddChallengeDmos {
                treat_as_static: true
            },
        ]
    );
    let done: Vec<bool> = (0..4)
        .map(|_| is_challenge_accept_operation_done(world, bowl))
        .collect();
    assert_eq!(done, [false, false, false, true]);
    assert!(take_ops(world).is_empty());
}

/// `RestartChallenge` redoes the operations of a challenge that asks for it: a Reset again, a
/// Save as a Clear, then the groups on the next update; others are left alone.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn restart_redoes_the_operations_of_challenges_that_ask() {
    let mut system = system();
    let world = system.world_mut();
    game_load(world, 4);
    let slappys = activity(world, "m_1a_01_goto_slappys");
    restart_challenge(world, slappys);
    assert_eq!(
        take_ops(world),
        [LivingWorldOp::DmoReset {
            kind: reset_type::RESET,
            volume: 0x0007_4441_03e3_8706
        }]
    );
    update(world);
    assert_eq!(
        take_ops(world),
        [LivingWorldOp::AddChallengeDmos {
            treat_as_static: false
        }]
    );
    let navarrette = activity(world, "m_4b_01_pro_navarrette_1");
    restart_challenge(world, navarrette);
    update(world);
    assert_eq!(
        take_ops(world),
        [
            LivingWorldOp::DmoReset {
                kind: reset_type::CLEAR,
                volume: 0x0007_5d41_03e3_8706
            },
            LivingWorldOp::AddChallengeDmos {
                treat_as_static: false
            },
        ]
    );
    let bowl = activity(world, "c_1a_01_bowl_series");
    restart_challenge(world, bowl);
    update(world);
    assert!(take_ops(world).is_empty());
}
