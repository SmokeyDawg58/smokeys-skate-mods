//! Focused source tests for the dynamic half of `Iris::PathEvaluator`.
use super::*;
use crate::physical_player_assembly::{PhysicalAssemblyWorld, tests::issued_owner_for_grind_tests};

fn vector(x: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [x, 0.0, 0.0],
        w: None,
    }
}

#[test]
fn pedestrian_candidates_filter_direction_and_keep_partial_capacity_prefix() {
    let mut server = CameraObstacles::constructed();
    server.velocity = vector(1.);
    let mut list = CameraDynamicObstacleList::new();
    assert!(server.add_body(&mut list, [-5.,0.,0.,0.], [0.;4], 0.6));
    assert!(server.add_body(&mut list, [5.,0.,0.,0.], [2.,0.,0.,0.], 0.6));
    assert!(list.records().is_empty());
    assert!(server.add_body(&mut list, [5.,2.,0.,0.], [0.;4], 0.6));
    assert_eq!(list.records().len(), 3);
    for (entry, y) in list.records().iter().zip([2.,2.7,3.4]) {
        assert_eq!(entry.position.xyz, [5.,y,0.]);
        assert_eq!(entry.radius, 0.6); // Full model width, not half width.
    }
    while list.records().len() < 49 { assert!(list.add(obstacle(9.,0.))); }
    assert!(!server.add_body(&mut list, [6.,0.,0.,0.], [0.;4], 0.6));
    assert_eq!(list.records().len(), 50);
    assert_eq!(list.records()[49].position.xyz, [6.,0.,0.]);
}

fn obstacle(position: f32, velocity: f32) -> CameraDynamicObstacle {
    CameraDynamicObstacle {
        position: vector(position),
        velocity: vector(velocity),
        radius: 0.5,
    }
}

fn evaluator() -> (PhysicalAssemblyWorld, CameraPathEvaluator) {
    let (mut world, player) = issued_owner_for_grind_tests();
    world
        .try_initialize_prediction_owner(&player.inputs().vault, true)
        .unwrap();
    let prediction = Arc::new(Mutex::new(CameraPrediction::constructed(Arc::clone(
        world.prediction_owner().unwrap(),
    ))));
    let mut evaluator = CameraPathEvaluator::constructed(
        Arc::new(Mutex::new(CameraObstacles::constructed())),
        prediction,
    );
    evaluator.input = Some(CameraPathInput {
        position: vector(0.0),
        velocity: vector(1.0),
        radius: 0.5,
        lookahead: 6.0,
        debug: false,
        in_air: false,
    });
    (world, evaluator)
}

#[test]
fn sphere_test_uses_closest_approach_and_strict_overlap() {
    let (_, evaluator) = evaluator();
    let input = evaluator.input.as_ref().unwrap();
    assert_eq!(sphere_collision_time(input, &obstacle(5.0, 0.0)), Some(5.0));
    assert_eq!(sphere_collision_time(input, &obstacle(0.5, 0.0)), Some(0.0));
    assert_eq!(sphere_collision_time(input, &obstacle(5.0, 2.0)), None);
    let tangent = CameraDynamicObstacle {
        position: SkeletonRootVector {
            xyz: [5.0, 1.0, 0.0],
            w: None,
        },
        ..obstacle(5.0, 0.0)
    };
    assert_eq!(sphere_collision_time(input, &tangent), None);
    assert_eq!(sphere_collision_time(input, &obstacle(f32::NAN, 0.0)), None);
}

#[test]
fn dynamic_evaluator_takes_earliest_strict_lookahead_and_static_minimum() {
    let (_, mut evaluator) = evaluator();
    let mut obstacles = CameraDynamicObstacleList::new();
    assert!(obstacles.add(obstacle(5.0, 0.0)));
    assert!(obstacles.add(obstacle(3.0, 0.0)));
    evaluator.update_dynamic_evaluator(&obstacles).unwrap();
    assert!(evaluator.valid);
    assert_eq!(evaluator.collision_time, 3.0);

    evaluator.collision_time = 2.0;
    evaluator.valid = false;
    evaluator.update_dynamic_evaluator(&obstacles).unwrap();
    assert!(evaluator.valid);
    assert_eq!(evaluator.collision_time, 2.0);

    evaluator.input.as_mut().unwrap().lookahead = 3.0;
    evaluator.collision_time = f32::MAX;
    evaluator.valid = false;
    evaluator.update_dynamic_evaluator(&obstacles).unwrap();
    assert!(!evaluator.valid);
    assert_eq!(evaluator.collision_time, f32::MAX);

    evaluator.input.as_mut().unwrap().lookahead = f32::NAN;
    evaluator.update_dynamic_evaluator(&obstacles).unwrap();
    assert!(!evaluator.valid);
}

#[test]
fn native_obstacle_list_caps_at_fifty() {
    let mut obstacles = CameraDynamicObstacleList::new();
    for index in 0..50 {
        assert!(obstacles.add(obstacle(index as f32, 0.0)));
    }
    assert!(!obstacles.add(obstacle(51.0, 0.0)));
    assert_eq!(obstacles.records().len(), 50);
}

#[test]
fn live_server_requires_published_pedestrian_owner_then_reads_empty_roster() {
    use crate::{
        input_cvar_runtime::InputCvarManager,
        simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo,
        skater_manager_local_identity::LocalBackendOwner,
    };

    let refs = InputCvarReferences::new_static_initialized();
    let mut cvar_manager = InputCvarManager::new();
    cvar_manager.try_link(&refs).unwrap();
    let living_world = Arc::new(Mutex::new(LivingWorldGrindServices::new()));
    let game_info = PhysicalGameInfo::construct_network_field();
    let skaters = SkaterManagerLocalIdentity::construct_local_fields(
        LocalBackendOwner::construct_static_fields(game_info),
    );
    let sources = CameraObstacleSources {
        cvars: &refs,
        living_world: &living_world,
        skaters: &skaters,
    };
    let input = CameraPathInput {
        position: SkeletonRootVector::known([3.0, 7.0, 11.0, 3.0]),
        velocity: vector(2.0),
        radius: 0.25,
        lookahead: 1.0,
        debug: false,
        in_air: false,
    };
    let mut server = CameraObstacles::constructed();
    assert!(matches!(
        server.try_find(&input, sources),
        Err(CameraObstacleError::Pedestrian(
            PedestrianNearQueryError::UnpublishedManager
        ))
    ));
    // FindObstacles publishes all three input fields before the first CVar.
    assert_eq!(server.position, input.position);
    assert_eq!(server.velocity, input.velocity);
    assert_eq!(server.radius, input.radius);

    let vault = Arc::new(crate::attribute_runtime::tests::graph(0, vec![]));
    living_world
        .lock()
        .unwrap()
        .try_initialize_selected_entities(&vault)
        .unwrap();
    let obstacles = server.try_find(&input, sources).unwrap();
    assert!(obstacles.records().is_empty());
    assert_eq!(skaters.try_count().unwrap(), 0);
}
