use super::*;
use crate::{
    collision_query_batches::trajectory::{
        TrajectoryResult, linear_collision_time, skeleton_segment,
    },
    collision_query_batches::{
        CollisionQueryBatchCapacity, CollisionQueryBatchConfig, CollisionQueryBatchFamily,
        CollisionQueryBatches,
    },
    reckoning_numeric::HostReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
};

fn config() -> CollisionQueryBatchConfig {
    let value = CollisionQueryBatchCapacity {
        batches: 3,
        tests: 1,
    };
    CollisionQueryBatchConfig {
        line: value,
        boxes: value,
        trajectory: value,
        step_selector: value,
    }
}
fn input() -> SkeletonTrajectoryInput {
    SkeletonTrajectoryInput {
        position: SkeletonRootVector::known([0.0, 10.0, 0.0, 7.0]),
        velocity: SkeletonRootVector::known([0.0, -2.0, 0.0, 0.0]),
        radius: 0.03,
    }
}
fn hit() -> TrajectoryResult {
    TrajectoryResult {
        position: SkeletonRootVector {
            xyz: [0.0, 9.0, 0.0],
            w: None,
        },
        normal: SkeletonRootVector::known([1.0, 0.0, 0.0, 23.0]),
        ground_normal: SkeletonRootVector {
            xyz: [0.0, 1.0, 0.0],
            w: None,
        },
        time: [0.01; 4],
        frame: 0,
        surface: 0xffff_ffff,
    }
}

#[test]
fn native_linear_time_retains_fraction_while_frame_is_floored() {
    let (time, frame) =
        linear_collision_time(input(), [0.0, 9.0, 0.0], &mut HostReckoningNumeric).unwrap();
    assert_eq!(frame, 0);
    assert_eq!(
        time.to_bits(),
        (0.5 * f32::from_bits(0x3c88_8889)).to_bits()
    );
    let mut short = input();
    short.velocity.xyz = [0.0, -0.1, 0.0];
    short.radius = 1000.0;
    // Native errorStart0 threshold0; generic radius-squared threshold would skip.
    assert!(
        skeleton_segment(short, &mut HostReckoningNumeric)
            .unwrap()
            .is_some()
    );
}

#[test]
fn completed_new_job_does_not_replace_old_surface_before_later_sync() {
    let mut batches = CollisionQueryBatches::<()>::new(config());
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Trajectory, "skeleton")
        .unwrap();
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    // 39284..98 clear stack var_100;3932c replaces r7 before393a0/ac.
    // The earlier gJ load therefore does not initialize the left normal.
    for surface in [
        processed.line_surfaces().left,
        processed.line_surfaces().right,
        processed.line_surfaces().hips,
    ] {
        assert_eq!(surface.unwrap().normal, SkeletonRootVector::known([0.0; 4]));
    }
    let mut previous = std::array::from_fn(|_| result_or_clear(None));
    previous[1].surface_id = 71;
    previous[1].valid = true;
    batches.begin(&lease).unwrap();
    batches.add_skeleton_trajectory(&lease, input()).unwrap();
    batches
        .end_skeleton_trajectory(&lease, |_| Ok::<_, ()>(hit()))
        .unwrap();
    processed.fill_skeleton_line_surfaces(&previous);
    assert_eq!(processed.line_surfaces().left_id, Some(71));
    assert!(batches.get_trajectory_result(&lease, 0).is_err());
    batches.sync(&lease).unwrap();
    previous[1] = result_or_clear(batches.get_trajectory_result(&lease, 0).unwrap());
    assert_eq!(previous[1].normal, hit().ground_normal);
    assert_eq!(previous[1].position.w, None);
    assert_eq!(previous[1].surface_id, -1);
    batches.release(&lease).unwrap();
    // End changes retained results only; the next Fill observes them.
    assert_eq!(processed.line_surfaces().left_id, Some(71));
    processed.fill_skeleton_line_surfaces(&previous);
    assert_eq!(processed.line_surfaces().left_id, Some(-1));
    previous[1] = result_or_clear(None);
    processed.fill_skeleton_line_surfaces(&previous);
    assert_eq!(processed.line_surfaces().left_id, Some(-1));
    assert!(!processed.line_surfaces().left.unwrap().valid);
}

#[test]
fn failed_job_cannot_be_synchronized_or_published_as_a_miss() {
    let mut batches = CollisionQueryBatches::<()>::new(config());
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Trajectory, "failing")
        .unwrap();
    batches.begin(&lease).unwrap();
    batches.add_skeleton_trajectory(&lease, input()).unwrap();
    assert!(
        batches
            .end_skeleton_trajectory(&lease, |_| Err::<TrajectoryResult, _>("geometry"))
            .is_err()
    );
    assert!(batches.sync(&lease).is_err());
    assert!(batches.get_trajectory_result(&lease, 0).is_err());
}

#[test]
fn world_trajectory_uses_actual_constructor_collections_and_retained_leases() {
    use crate::simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineQueries, SimulationWorldLineQueryConfig,
    };
    use crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool;
    type Backend = crate::simulation_assembly_backend::SimulationAssemblyBackend<
        skate2_formats::collision::AssemblyPartMaterial,
        crate::simulation_joint_pool::SimulationJointFrames,
        crate::simulation_joint_pool::SimulationJointLimits,
    >;
    let mut backend = Backend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let dmos = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let mut world = SimulationWorldLineQueries::try_new(
        &manager,
        &backend,
        &dmos,
        3,
        SimulationWorldLineQueryConfig {
            vehicle_update_in_parallel: false,
            ground: config(),
            dmo: config(),
            vehicle: config(),
        },
    )
    .unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    let batch = world
        .try_acquire_skeleton_trajectory(&dmos, "actual empty owners")
        .unwrap();
    world.try_begin_trajectory(&batch).unwrap();
    assert!(
        world
            .try_add_skeleton_trajectory::<()>(&batch, input())
            .unwrap()
    );
    world
        .try_end_trajectory(&batch, &ground, &dmos, &mut HostReckoningNumeric)
        .unwrap();
    assert!(world.try_get_trajectory_result(&batch, 0).is_err());
    world.try_sync_trajectory(&batch).unwrap();
    assert!(
        world
            .try_get_trajectory_result(&batch, 0)
            .unwrap()
            .is_none()
    );
    world.try_release_trajectory(&batch).unwrap();
    assert!(world.try_sync_trajectory(&batch).is_err());
}

/// A separate controlled hips-shaped request guarantees positive real geometry
/// coverage even when the enclosing adjusted-pose fixture yields actual misses.
pub(crate) fn check_terrain_hit(
    world: &mut PhysicalAssemblyWorld,
    ground: &StaticGroundPool,
    terrain: [f32; 3],
) {
    use crate::{
        collision_query_batches::trajectory::SkeletonTrajectoryInput,
        skeleton_root_transforms::SkeletonRootVector,
    };
    let queries = world.world_line_queries.as_mut().unwrap();
    let dmos = world.dmos.as_ref().unwrap();
    let batch = queries
        .try_acquire_skeleton_trajectory(dmos, "original terrain positive skeleton-shaped query")
        .unwrap();
    queries.try_begin_trajectory(&batch).unwrap();
    queries
        .try_add_skeleton_trajectory::<std::convert::Infallible>(
            &batch,
            SkeletonTrajectoryInput {
                position: SkeletonRootVector::known([
                    terrain[0],
                    terrain[1] + 1.0,
                    terrain[2],
                    0.0,
                ]),
                velocity: SkeletonRootVector::known([0.0, -100.0, 0.0, 0.0]),
                radius: f32::from_bits(0x3cf5_c28f),
            },
        )
        .unwrap();
    queries
        .try_end_trajectory(&batch, ground, dmos, &mut HostReckoningNumeric)
        .unwrap();
    queries.try_sync_trajectory(&batch).unwrap();
    let hit = queries
        .try_get_trajectory_result(&batch, 0)
        .unwrap()
        .expect("actual resident park terrain");
    assert!(hit.valid());
    assert!(hit.position.xyz[1] < terrain[1] + 1.0);
    assert!(hit.ground_normal.xyz.iter().all(|v| v.is_finite()));
    assert_eq!(hit.position.w, None);
    assert_eq!(hit.ground_normal.w, None);
    queries.try_release_trajectory(&batch).unwrap();
    println!("original terrain positive trajectory: {hit:?}");
}
