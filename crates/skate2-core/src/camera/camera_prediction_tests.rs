//! Focused retained camera PathEvaluator query lifecycle tests.
use super::*;
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig},
    physical_player_assembly::{PhysicalAssemblyWorld, tests::issued_owner_for_grind_tests},
    simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig,
    static_ground_pool::StaticGroundPool,
};
use std::convert::Infallible;

fn fixture() -> (PhysicalAssemblyWorld, CameraPathEvaluator, StaticGroundPool) {
    let (mut world, player) = issued_owner_for_grind_tests();
    world
        .try_initialize_prediction_owner(&player.inputs().vault, true)
        .unwrap();
    let capacity = CollisionQueryBatchCapacity {
        batches: 1,
        tests: 1,
    };
    let config = CollisionQueryBatchConfig {
        line: capacity,
        boxes: capacity,
        trajectory: capacity,
        step_selector: capacity,
    };
    world
        .try_initialize_world_line_queries(
            SimulationWorldLineQueryConfig {
                vehicle_update_in_parallel: false,
                ground: config,
                dmo: config,
                vehicle: config,
            },
            false,
        )
        .unwrap();
    world.try_initialize_debug_overlay().unwrap();
    let predictor = Arc::clone(world.prediction_owner().unwrap());
    let prediction = Arc::new(Mutex::new(CameraPrediction::constructed(predictor)));
    let obstacles = Arc::new(Mutex::new(CameraObstacles::constructed()));
    let mut evaluator = CameraPathEvaluator::constructed(obstacles, prediction);
    evaluator.input = Some(CameraPathInput {
        position: SkeletonRootVector {
            xyz: [0.0, 2.0, 0.0],
            w: None,
        },
        velocity: SkeletonRootVector {
            xyz: [1.0, 2.0, 0.0],
            w: None,
        },
        radius: 0.2,
        lookahead: 0.25,
        debug: false,
        in_air: true,
    });
    (world, evaluator, StaticGroundPool::try_new(1).unwrap())
}

fn static_prefix(
    world: &mut PhysicalAssemblyWorld,
    evaluator: &mut CameraPathEvaluator,
    ground: &StaticGroundPool,
) {
    let mut numeric = HostCollisionLineNumeric;
    let mut services = world.camera_line_services(ground, &mut numeric);
    assert!(matches!(
        evaluator.try_update_prefix(&mut services),
        Err(CameraPathEvaluatorError::PendingDynamicEvaluator)
    ));
}

fn harvest(world: &mut PhysicalAssemblyWorld, ground: &StaticGroundPool) {
    let predictor = Arc::clone(world.prediction_owner().unwrap());
    let mut numeric = HostCollisionLineNumeric;
    let services = world.camera_line_services(ground, &mut numeric);
    predictor
        .try_update_queries::<Infallible>(services.queries.as_mut().unwrap(), services.overlay)
        .unwrap();
}

#[test]
fn static_path_submits_waits_then_consumes_retained_no_hit_future() {
    let (mut world, mut evaluator, ground) = fixture();
    let predictor = Arc::clone(world.prediction_owner().unwrap());
    evaluator.last_valid_collision_time = 42.0;

    static_prefix(&mut world, &mut evaluator, &ground);
    assert_eq!(evaluator.static_query_state, 1);
    assert!(evaluator.was_in_air);
    assert!(!evaluator.valid);
    assert_eq!(evaluator.collision_time, f32::MAX);
    assert_eq!(evaluator.last_valid_collision_time, 42.0);
    assert!(!evaluator.prediction.lock().unwrap().future.ready);
    assert_eq!(predictor.pending_count().unwrap(), 1);

    // A second camera pass before the World predictor harvest cannot consume
    // or submit again. Input can change while the saved in-air flag remains.
    evaluator.input.as_mut().unwrap().in_air = false;
    static_prefix(&mut world, &mut evaluator, &ground);
    assert_eq!(evaluator.static_query_state, 1);
    assert!(!evaluator.valid);
    assert_eq!(predictor.pending_count().unwrap(), 1);

    harvest(&mut world, &ground);
    assert!(evaluator.prediction.lock().unwrap().future.ready);
    assert_eq!(
        evaluator.prediction.lock().unwrap().future.collision_time,
        Some([-1.0; 4])
    );
    static_prefix(&mut world, &mut evaluator, &ground);
    assert_eq!(evaluator.static_query_state, 0);
    assert!(evaluator.valid);
    assert_eq!(evaluator.collision_time, f32::MAX);
    assert_eq!(evaluator.output_was_in_air, Some(true));
    assert_eq!(evaluator.last_valid_collision_time, 42.0);
    assert_eq!(predictor.pending_count().unwrap(), 0);

    // Completion consumes only; a subsequent pass starts the next query and
    // captures the new grounded flag.
    static_prefix(&mut world, &mut evaluator, &ground);
    assert_eq!(evaluator.static_query_state, 1);
    assert!(!evaluator.was_in_air);
    assert_eq!(predictor.pending_count().unwrap(), 1);
    harvest(&mut world, &ground);
    static_prefix(&mut world, &mut evaluator, &ground);
    assert_eq!(evaluator.output_was_in_air, Some(false));
}

#[test]
fn static_path_rejects_source_getter_assertion_boundary() {
    let (mut world, mut evaluator, ground) = fixture();
    evaluator.static_query_state = 1;
    {
        let mut prediction = evaluator.prediction.lock().unwrap();
        prediction.future.ready = true;
        prediction.future.collision_time = Some([100.0; 4]);
    }
    let mut numeric = HostCollisionLineNumeric;
    let mut services = world.camera_line_services(&ground, &mut numeric);
    assert!(matches!(
        evaluator.try_update_prefix(&mut services),
        Err(CameraPathEvaluatorError::CollisionTimeOutOfRange(value)) if value == 100.0
    ));
    assert_eq!(evaluator.static_query_state, 1);
    assert!(!evaluator.valid);
    assert_eq!(evaluator.collision_time, f32::MAX);
}
