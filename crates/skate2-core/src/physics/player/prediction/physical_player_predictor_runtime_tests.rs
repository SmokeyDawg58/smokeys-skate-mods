#![expect(
    clippy::float_cmp,
    reason = "exact retained field copies and source sentinels"
)]
use super::*;
use crate::camera_prediction::{CameraPathInput, CameraPrediction};
use crate::collision_query_batches::trajectory::TrajectoryResult;
use crate::physical_player_trajectory_owners::RetainedPredictionFuture;
use crate::{
    collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig},
    physical_player_assembly::{PhysicalAssemblyWorld, tests::issued_owner_for_grind_tests},
    physical_player_trajectory_owners::{OnboardTrajectoryOwner, TrajectoryOwnerBindings},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig,
};
use std::sync::Mutex;

fn fixture(
    batch_count: u32,
) -> (
    PhysicalAssemblyWorld,
    Arc<PhysicalPlayerAssemblyOwner>,
    Arc<PhysicalPredictionOwner>,
) {
    let (mut world, owner) = issued_owner_for_grind_tests();
    let cap = CollisionQueryBatchCapacity {
        batches: batch_count,
        tests: 1,
    };
    let config = CollisionQueryBatchConfig {
        line: cap,
        boxes: cap,
        trajectory: cap,
        step_selector: cap,
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
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    owner.trajectories().lock().unwrap().onboard = Some(OnboardTrajectoryOwner::new(
        TrajectoryOwnerBindings::new(&owner, processed),
        false,
    ));
    // Controlled constructor fields; actual manager/query/DMO/future owners are
    // retained. Gravity is supplied per query; no runtime tuning is fabricated.
    let predictor = Arc::new(PhysicalPredictionOwner {
        manager: owner.manager(),
        physics_attributes: owner.inputs().vault.physics(0xd7ed_bd36_2d7d_2152),
        gravity: [0.0, -9.8, 0.0, 0.0],
        queue: Mutex::new(super::super::PredictorQueue {
            slots: std::array::from_fn(|_| None),
            length: 0,
        }),
    });
    (world, owner, predictor)
}
fn input() -> BallisticTrajectoryInput {
    BallisticTrajectoryInput {
        position: Vector {
            xyz: [0.0, 2.0, 0.0],
            w: None,
        },
        velocity: Vector {
            xyz: [1.0, 2.0, 0.0],
            w: None,
        },
        gravity: Vector {
            xyz: [0.0, -9.8, 0.0],
            w: None,
        },
        duration: 0.25,
        radius: 0.2,
        maximum_error_start: 0.1,
        maximum_error_end: 0.2,
    }
}
fn submit(
    world: &mut PhysicalAssemblyWorld,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    predictor: &PhysicalPredictionOwner,
    slot: usize,
    request: BallisticTrajectoryInput,
) -> Result<(), PredictionRuntimeError<ReckoningHostNumericError>> {
    predictor.try_add_onboard_query(
        owner,
        slot,
        request,
        true,
        world.world_line_queries.as_mut().unwrap(),
        &StaticGroundPool::try_new(1).unwrap(),
        world.dmos.as_ref().unwrap(),
        &mut HostReckoningNumeric,
    )
}
fn future(owner: &Arc<PhysicalPlayerAssemblyOwner>, slot: usize) -> RetainedPredictionFuture {
    owner
        .trajectories()
        .lock()
        .unwrap()
        .onboard
        .as_ref()
        .unwrap()
        .queries[slot]
}
fn harvest(
    world: &mut PhysicalAssemblyWorld,
    predictor: &PhysicalPredictionOwner,
) -> Result<(), PredictionRuntimeError<ReckoningHostNumericError>> {
    predictor.try_update_queries(
        world.world_line_queries.as_mut().unwrap(),
        &world.debug_overlay,
    )
}

fn submit_camera(
    world: &mut PhysicalAssemblyWorld,
    camera: &Arc<Mutex<CameraPrediction>>,
    in_air: bool,
) -> Result<(), PredictionRuntimeError<ReckoningHostNumericError>> {
    let request = input();
    let ground = StaticGroundPool::try_new(1).unwrap();
    let mut numeric = crate::collision_line_numeric::HostCollisionLineNumeric;
    let mut services = world.camera_line_services(&ground, &mut numeric);
    CameraPrediction::try_start_query(
        camera,
        &CameraPathInput {
            position: request.position,
            velocity: request.velocity,
            radius: request.radius,
            lookahead: request.duration,
            debug: false,
            in_air,
        },
        &mut services,
    )
}

#[test]
fn camera_queries_share_world_queue_and_publish_only_at_predictor_harvest() {
    let (mut world, player, predictor) = fixture(2);
    let camera = Arc::new(Mutex::new(CameraPrediction::constructed(Arc::clone(
        &predictor,
    ))));
    submit(&mut world, &player, &predictor, 0, input()).unwrap();
    submit_camera(&mut world, &camera, false).unwrap();
    assert_eq!(predictor.pending_count().unwrap(), 2);
    assert!(!future(&player, 0).ready);
    {
        let camera = camera.lock().unwrap();
        assert!(!camera.future.ready);
        assert_eq!(
            camera.future.trajectory.gravity.known_lanes(),
            Some([0.0; 4])
        );
        assert_eq!(camera.future.max_error_start, Some(1.0));
        assert_eq!(camera.future.max_error_end, Some(1.0));
        assert_eq!(camera.future.draw_true, Some(false));
    }
    harvest(&mut world, &predictor).unwrap();
    assert!(future(&player, 0).ready);
    {
        let camera = camera.lock().unwrap();
        assert!(camera.future.ready);
        assert_eq!(camera.future.collision_time, Some([-1.0; 4]));
        assert_eq!(camera.future.trajectory.position, input().position);
    }
    assert_eq!(predictor.pending_count().unwrap(), 0);

    submit_camera(&mut world, &camera, true).unwrap();
    {
        let camera = camera.lock().unwrap();
        assert!(!camera.future.ready);
        assert_eq!(
            camera.future.trajectory.gravity.known_lanes(),
            Some(predictor.gravity)
        );
    }
    harvest(&mut world, &predictor).unwrap();
    assert!(camera.lock().unwrap().future.ready);
}

#[test]
fn submitted_actual_jobs_remain_unready_until_harvest_and_keep_unknown_w() {
    let (mut world, owner, predictor) = fixture(1);
    submit(&mut world, &owner, &predictor, 0, input()).unwrap();
    assert_eq!(predictor.pending_count().unwrap(), 1);
    assert!(!future(&owner, 0).ready);
    assert_eq!(future(&owner, 0).result.trajectory.position.w, None);
    assert_eq!(future(&owner, 0).result.trajectory.velocity.w, None);
    assert_eq!(future(&owner, 0).result.trajectory.gravity.w, None);
    assert!(!future(&owner, 1).ready);
    harvest(&mut world, &predictor).unwrap();
    let completed = future(&owner, 0);
    assert!(completed.ready);
    assert_eq!(completed.result.time, [-1.0; 4]); // actual empty collection result
    assert_eq!(completed.result.trajectory.position, input().position);
    assert_eq!(predictor.pending_count().unwrap(), 0);
    assert!(!future(&owner, 1).ready);
    // The released source leases can actually be reacquired.
    submit(&mut world, &owner, &predictor, 1, input()).unwrap();
    harvest(&mut world, &predictor).unwrap();
}

#[test]
fn queue_capacity_failure_keeps_reset_future_without_acquiring_batch() {
    let (mut world, owner, predictor) = fixture(1);
    {
        let mut queue = predictor.queue.lock().unwrap();
        queue.slots = std::array::from_fn(|_| {
            Some(PendingPredictionQuery {
                destination: PredictionDestination::Player {
                    owner: Arc::downgrade(&owner),
                    target: PredictionTarget::Onboard(0),
                },
                batch: None,
            })
        });
        queue.length = 90;
    }
    owner
        .trajectories()
        .lock()
        .unwrap()
        .onboard
        .as_mut()
        .unwrap()
        .queries[0]
        .ready = true;
    assert!(matches!(
        submit(&mut world, &owner, &predictor, 0, input()),
        Err(PredictionRuntimeError::QueueFull)
    ));
    let reset = future(&owner, 0);
    assert!(!reset.ready);
    assert_eq!(reset.result.frame, -1);
    assert_eq!(reset.result.time, [-1.0; 4]);
    assert_eq!(
        reset.result.normal.known_lanes(),
        Some([0.0, 1.0, 0.0, 0.0])
    );
    assert_eq!(reset.result.trajectory.position, input().position);
    assert_eq!(reset.result.radius, input().radius);
    assert_eq!(predictor.pending_count().unwrap(), 90);
    assert!(
        predictor
            .queue
            .lock()
            .unwrap()
            .slots
            .iter()
            .all(|entry| entry.as_ref().unwrap().batch.is_none())
    );
    let mut exceptional = input();
    exceptional.position.xyz[0] = f32::INFINITY;
    exceptional.radius = f32::NAN;
    assert!(matches!(
        submit(&mut world, &owner, &predictor, 0, exceptional),
        Err(PredictionRuntimeError::QueueFull)
    ));
    assert!(future(&owner, 0).result.radius.is_nan());
    assert_eq!(
        future(&owner, 0).result.trajectory.position.xyz[0],
        f32::INFINITY
    );
    // Capacity rejection acquired nothing: the sole batch is still available.
    let batch = world
        .world_line_queries
        .as_mut()
        .unwrap()
        .try_acquire_trajectory(world.dmos.as_ref().unwrap(), "capacity check")
        .unwrap();
    world
        .world_line_queries
        .as_mut()
        .unwrap()
        .try_release_trajectory(&batch)
        .unwrap();
}

#[test]
fn foreign_world_and_nan_reject_before_future_reset() {
    let (mut world, owner, predictor) = fixture(1);
    owner
        .trajectories()
        .lock()
        .unwrap()
        .onboard
        .as_mut()
        .unwrap()
        .queries[0]
        .ready = true;
    let (mut foreign, _, _) = fixture(1);
    assert!(matches!(
        submit(&mut foreign, &owner, &predictor, 0, input()),
        Err(PredictionRuntimeError::ForeignManager)
    ));
    let mut invalid = input();
    invalid.position.xyz[1] = f32::NAN;
    assert!(matches!(
        submit(&mut world, &owner, &predictor, 0, invalid),
        Err(PredictionRuntimeError::InputNaN)
    ));
    assert!(future(&owner, 0).ready);
    assert_eq!(predictor.pending_count().unwrap(), 0);
}

#[test]
fn late_query_failure_preserves_completed_prefix_and_queue() {
    let (mut world, owner, predictor) = fixture(1);
    submit(&mut world, &owner, &predictor, 0, input()).unwrap();
    // The second queue entry exists before its real batch acquisition fails.
    assert!(matches!(
        submit(&mut world, &owner, &predictor, 1, input()),
        Err(PredictionRuntimeError::World(_))
    ));
    assert_eq!(predictor.pending_count().unwrap(), 2);
    assert!(matches!(
        harvest(&mut world, &predictor),
        Err(PredictionRuntimeError::Missing("queued batch"))
    ));
    assert!(future(&owner, 0).ready);
    assert!(!future(&owner, 1).ready);
    assert_eq!(predictor.pending_count().unwrap(), 2);
    assert!(matches!(
        harvest(&mut world, &predictor),
        Err(PredictionRuntimeError::AlreadyReady)
    ));
}

#[test]
fn collision_copy_does_not_publish_readiness_or_replace_trajectory() {
    let (_, owner, _) = fixture(1);
    let mut pending = future(&owner, 0);
    reset_future(
        &mut PredictionFutureMut::Onboard(&mut pending),
        input(),
        true,
    );
    let mut collision = TrajectoryResult::initialized();
    collision.position = Vector {
        xyz: [5.0, 6.0, 7.0],
        w: None,
    };
    collision.time = [0.25; 4];
    collision.frame = 15;
    collision.surface = 42;
    PredictionFutureMut::Onboard(&mut pending).copy_collision(collision);
    assert!(!pending.ready);
    assert_eq!(pending.result.position, collision.position);
    assert_eq!(pending.result.time, collision.time);
    assert_eq!(pending.result.frame, 15);
    assert_eq!(pending.result.surface, 42);
    assert_eq!(pending.result.trajectory.position, input().position);
    assert!(pending.result.draw_true);
}

#[test]
fn two_players_same_slot_keep_distinct_retained_futures() {
    let (mut world, first, predictor) = fixture(2);
    let second = {
        let construction = crate::physical_player_assembly::PhysicalPlayerAssemblyConstruction::new(
            &mut world,
            first.inputs().clone(),
        );
        Arc::clone(construction.owner())
    };
    let processed = Arc::clone(second.reckoning().lock().unwrap().processed());
    second.trajectories().lock().unwrap().onboard = Some(OnboardTrajectoryOwner::new(
        TrajectoryOwnerBindings::new(&second, processed),
        false,
    ));
    let mut other_input = input();
    other_input.position.xyz[0] = 42.0;
    submit(&mut world, &first, &predictor, 0, input()).unwrap();
    submit(&mut world, &second, &predictor, 0, other_input).unwrap();
    harvest(&mut world, &predictor).unwrap();
    assert!(future(&first, 0).ready && future(&second, 0).ready);
    assert_eq!(
        future(&first, 0).result.trajectory.position,
        input().position
    );
    assert_eq!(
        future(&second, 0).result.trajectory.position,
        other_input.position
    );
}

#[test]
fn only_nonempty_queue_requires_actual_debug_overlay() {
    let (mut world, owner, predictor) = fixture(1);
    let absent = DebugOverlaySlot::default();
    predictor
        .try_update_queries::<ReckoningHostNumericError>(
            world.world_line_queries.as_mut().unwrap(),
            &absent,
        )
        .unwrap();
    submit(&mut world, &owner, &predictor, 0, input()).unwrap();
    assert!(matches!(
        predictor.try_update_queries::<ReckoningHostNumericError>(
            world.world_line_queries.as_mut().unwrap(),
            &absent,
        ),
        Err(PredictionRuntimeError::Overlay(_))
    ));
    assert!(!future(&owner, 0).ready);
    assert_eq!(predictor.pending_count().unwrap(), 1);
    harvest(&mut world, &predictor).unwrap();
}

#[test]
fn offboard_future_uses_existing_queue_and_publication_schedule() {
    let (mut world, owner, predictor) = fixture(1);
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    owner.trajectories().lock().unwrap().offboard = Some(
        crate::physical_player_trajectory_owners::OffboardTrajectoryOwner::new(
            TrajectoryOwnerBindings::new(&owner, processed),
            &owner.inputs().vault,
        ),
    );
    predictor
        .try_add_query(
            &owner,
            PredictionTarget::Offboard(0),
            input(),
            false,
            world.world_line_queries.as_mut().unwrap(),
            &StaticGroundPool::try_new(1).unwrap(),
            world.dmos.as_ref().unwrap(),
            &mut HostReckoningNumeric,
        )
        .unwrap();
    assert!(
        !owner
            .trajectories()
            .lock()
            .unwrap()
            .offboard
            .as_ref()
            .unwrap()
            .queries[0]
            .ready
    );
    assert!(!future(&owner, 0).ready);
    harvest(&mut world, &predictor).unwrap();
    let trajectories = owner.trajectories().lock().unwrap();
    let future = &trajectories.offboard.as_ref().unwrap().queries[0];
    assert!(future.ready);
    assert_eq!(future.result.time, [-1.0; 4]);
    assert_eq!(future.result.trajectory.velocity.w, None);
    assert!(!trajectories.onboard.as_ref().unwrap().queries[0].ready);
}
