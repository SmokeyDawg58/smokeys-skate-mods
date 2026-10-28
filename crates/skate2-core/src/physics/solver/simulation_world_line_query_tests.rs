use super::*;
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchState},
    skeleton_root_transforms::SkeletonRootVector,
};

fn batches(count: u32, tests: u32) -> CollisionQueryBatchConfig {
    let capacity = CollisionQueryBatchCapacity {
        batches: count,
        tests,
    };
    CollisionQueryBatchConfig {
        line: capacity,
        boxes: capacity,
        trajectory: capacity,
        step_selector: CollisionQueryBatchCapacity {
            batches: count,
            tests: 1,
        },
    }
}

fn config() -> SimulationWorldLineQueryConfig {
    SimulationWorldLineQueryConfig {
        vehicle_update_in_parallel: false,
        ground: batches(1, 2),
        dmo: batches(1, 2),
        vehicle: batches(1, 2),
    }
}

fn owners() -> (
    SourceBackend,
    SimulationManagerContactDomain,
    SimulationDmoAssemblyPool,
) {
    // No synthetic source image or arena binding is needed for constructor
    // membership. Manager creation establishes the actual shared authorities.
    let mut backend = SourceBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let dmos = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    (backend, manager, dmos)
}

fn input() -> CollisionLineInput {
    CollisionLineInput {
        start: SkeletonRootVector::known([0.0, 1.0, 0.0, 3.0]),
        end: SkeletonRootVector::known([0.0, -1.0, 0.0, 7.0]),
        fatness: 0.0,
    }
}

#[test]
fn real_empty_constructor_owners_query_and_release_without_source_images() {
    let (backend, manager, dmos) = owners();
    let mut world =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    assert_eq!(world.vehicle.capacity(), 24);
    assert_eq!(world.vehicle.len(), 0);
    assert!(world.dmo_collection.slots.capacity() >= 50);
    for _ in 0..2 {
        assert!(
            world
                .try_line_test(
                    &ground,
                    &dmos,
                    input(),
                    "empty",
                    &mut HostCollisionLineNumeric
                )
                .unwrap()
                .is_none()
        );
    }
    assert!(world.retained_acquisitions.is_empty());
    for child in [
        SimulationWorldLineChild::Ground,
        SimulationWorldLineChild::Dmo,
        SimulationWorldLineChild::Vehicle,
    ] {
        let lease = world
            .batches_mut(child)
            .acquire(CollisionQueryBatchFamily::Line, "observe")
            .unwrap();
        assert_eq!(world.batches_mut(child).test_count(&lease).unwrap(), 1);
        assert_eq!(
            world.batches_mut(child).state(&lease).unwrap(),
            CollisionQueryBatchState::Idle
        );
    }
}

#[test]
fn simulation_only_omits_vehicle_query_but_constructs_its_capacity24_owner() {
    let (backend, manager, dmos) = owners();
    let mut cfg = config();
    cfg.vehicle = batches(0, 0);
    let mut world = SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 1, cfg).unwrap();
    assert_eq!(world.vehicle.capacity(), 24);
    let ground = StaticGroundPool::try_new(1).unwrap();
    assert!(
        world
            .try_line_test(
                &ground,
                &dmos,
                input(),
                "simulation",
                &mut HostCollisionLineNumeric
            )
            .unwrap()
            .is_none()
    );
    let mut both = SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, cfg).unwrap();
    assert!(matches!(
        both.try_line_test(
            &ground,
            &dmos,
            input(),
            "both",
            &mut HostCollisionLineNumeric
        ),
        Err(SimulationWorldLineError::Batch {
            child: SimulationWorldLineChild::Vehicle,
            source: CollisionQueryBatchError::Exhausted { .. }
        })
    ));
    assert_eq!(both.retained_acquisitions.len(), 2);
    for entry in &both.retained_acquisitions {
        let batches = match entry.child {
            SimulationWorldLineChild::Ground => &both.ground_batches,
            SimulationWorldLineChild::Dmo => &both.dmo_batches,
            SimulationWorldLineChild::Vehicle => unreachable!(),
        };
        assert_eq!(batches.test_count(&entry.lease).unwrap(), 0);
        assert_eq!(
            batches.state(&entry.lease).unwrap(),
            CollisionQueryBatchState::Idle
        );
    }
}

#[test]
fn invalid_selected_flags_and_foreign_owners_refuse_before_acquisition() {
    let (backend, manager, dmos) = owners();
    let (other_backend, _, other_dmos) = owners();
    assert!(matches!(
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 2, config()),
        Err(SimulationWorldLineInitError::UnsupportedIslandFlags(2))
    ));
    assert!(matches!(
        SimulationWorldLineQueries::try_new(&manager, &other_backend, &dmos, 3, config()),
        Err(SimulationWorldLineInitError::ForeignBackend)
    ));
    let mut world =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    assert!(matches!(
        world.try_line_test(
            &ground,
            &other_dmos,
            input(),
            "foreign",
            &mut HostCollisionLineNumeric
        ),
        Err(SimulationWorldLineError::ForeignDmoOwner)
    ));
    assert!(world.retained_acquisitions.is_empty());
    assert!(
        world
            .try_line_test(
                &ground,
                &dmos,
                input(),
                "actual",
                &mut HostCollisionLineNumeric
            )
            .unwrap()
            .is_none()
    );
}

#[test]
fn ignored_add_failure_still_visits_later_children_and_aggregate_skips_absent_index() {
    let (backend, manager, dmos) = owners();
    let mut cfg = config();
    cfg.ground = batches(1, 0);
    let mut world = SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, cfg).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    assert!(
        world
            .try_line_test(
                &ground,
                &dmos,
                input(),
                "full",
                &mut HostCollisionLineNumeric
            )
            .unwrap()
            .is_none()
    );
    assert!(world.retained_acquisitions.is_empty());
    for child in [
        SimulationWorldLineChild::Dmo,
        SimulationWorldLineChild::Vehicle,
    ] {
        let batches = world.batches_mut(child);
        let lease = batches
            .acquire(CollisionQueryBatchFamily::Line, "later child")
            .unwrap();
        assert_eq!(batches.test_count(&lease).unwrap(), 1);
        assert_eq!(
            batches.state(&lease).unwrap(),
            CollisionQueryBatchState::Idle
        );
    }
}

#[test]
fn nearest_preserves_first_ties_unordered_and_full_result_lanes() {
    let mut first = CollisionMeshLineNearest::new();
    first.hit_count = 2;
    first.line_parameter = 0.5;
    first.position = SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]);
    first.normal = SkeletonRootVector::known([5.0, 6.0, 7.0, 8.0]);
    first.surface_id = 9;
    let mut nearest = None;
    select_nearest(&mut nearest, Some(&first));
    let mut later = CollisionMeshLineNearest::new();
    later.hit_count = 1;
    later.surface_id = 10;
    for parameter in [0.5, f32::NAN, 0.75] {
        later.line_parameter = parameter;
        select_nearest(&mut nearest, Some(&later));
        assert_eq!(nearest.as_ref(), Some(&first));
    }
    later.line_parameter = 0.25;
    select_nearest(&mut nearest, Some(&later));
    assert_eq!(nearest.as_ref(), Some(&later));
    first.line_parameter = f32::NAN;
    nearest = None;
    select_nearest(&mut nearest, Some(&first));
    select_nearest(&mut nearest, Some(&later));
    assert_eq!(nearest.unwrap().surface_id, 9);
}

#[test]
fn another_empty_pool_under_same_manager_cannot_replace_bound_dmo_owner() {
    let (backend, manager, dmos) = owners();
    let substitute = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let mut world =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    assert!(matches!(
        world.try_line_test(
            &ground,
            &substitute,
            input(),
            "substitute",
            &mut HostCollisionLineNumeric
        ),
        Err(SimulationWorldLineError::ForeignDmoOwner)
    ));
    assert!(world.retained_acquisitions.is_empty());
    assert!(
        world
            .try_line_test(
                &ground,
                &dmos,
                input(),
                "bound",
                &mut HostCollisionLineNumeric
            )
            .unwrap()
            .is_none()
    );
}

#[test]
fn living_world_dmo_capsule_uses_retained_flag_two_line_batch_and_releases() {
    let (backend, manager, dmos) = owners();
    let mut world =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    let mut capsule = input();
    capsule.fatness = 0.75;
    for _ in 0..2 {
        assert!(
            !world
                .try_dmo_capsule_test(&ground, &dmos, capsule, &mut HostCollisionLineNumeric)
                .unwrap()
        );
    }
    assert_eq!(
        world.last_line_input_for_test().unwrap().fatness.to_bits(),
        0.75f32.to_bits()
    );
    assert!(world.retained_acquisitions.is_empty());
    for (child, expected_count) in [
        (SimulationWorldLineChild::Ground, 0),
        (SimulationWorldLineChild::Dmo, 1),
        (SimulationWorldLineChild::Vehicle, 0),
    ] {
        let batches = world.batches_mut(child);
        let lease = batches
            .acquire(CollisionQueryBatchFamily::Line, "inspect")
            .unwrap();
        assert_eq!(batches.test_count(&lease).unwrap(), expected_count);
        assert_eq!(
            batches.state(&lease).unwrap(),
            CollisionQueryBatchState::Idle
        );
        batches.release(&lease).unwrap();
    }
}

#[test]
fn added_dmo_without_begin_update_has_no_published_line_meshes() {
    use crate::{
        serialised_assembly_bbox::HostSerialisedAssemblyBBoxNumeric,
        simulation_dmo_assembly_part_session::dynamic_pool::{
            SimulationDmoAssemblyCandidate, tests::fixture,
        },
    };
    let (mut backend, mut manager, owner) = fixture(3, 1);
    let mut dmos = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let mut world =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    dmos.try_insert(
        &mut manager,
        &mut backend,
        SimulationDmoAssemblyCandidate::try_from_locator(&owner).unwrap(),
        0,
        &mut HostSerialisedAssemblyBBoxNumeric,
    )
    .unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    // Add does not execute DynamicPool::BeginUpdate. Both line routes read
    // the existing, still-empty mesh publication rather than assembly count.
    for _ in 0..2 {
        assert!(
            !world
                .try_dmo_capsule_test(&ground, &dmos, input(), &mut HostCollisionLineNumeric)
                .unwrap()
        );
        assert!(
            world
                .try_line_test(
                    &ground,
                    &dmos,
                    input(),
                    "before BeginUpdate",
                    &mut HostCollisionLineNumeric
                )
                .unwrap()
                .is_none()
        );
    }
    assert!(world.retained_acquisitions.is_empty());
    assert_eq!(dmos.len(), 1);
}

#[test]
fn staged_batch_retains_five_queries_until_explicit_release() {
    let (backend, manager, dmos) = owners();
    let cfg = SimulationWorldLineQueryConfig {
        vehicle_update_in_parallel: false,
        ground: batches(1, 5),
        dmo: batches(1, 5),
        vehicle: batches(1, 5),
    };
    let mut world = SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, cfg).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    let batch = world
        .try_acquire_line_batch(&dmos, "wheel/deck", 7)
        .unwrap();
    world.try_begin(&batch).unwrap();
    for _ in 0..5 {
        assert!(world.try_add_line(&batch, input()).unwrap());
    }
    world
        .try_end(&batch, &ground, &dmos, &mut HostCollisionLineNumeric)
        .unwrap();
    assert!(matches!(
        world.try_acquire_line_batch(&dmos, "held across solve", 7),
        Err(SimulationWorldLineLeaseError::Batch {
            child: SimulationWorldLineChild::Ground,
            source: CollisionQueryBatchError::Exhausted { .. }
        })
    ));
    world.try_sync(&batch).unwrap();
    for index in 0..5 {
        assert!(world.try_get_result(&batch, index).unwrap().is_none());
    }
    world.try_release(&batch).unwrap();
    assert_eq!(
        world.try_begin(&batch),
        Err(SimulationWorldLineLeaseError::ReleasedLease)
    );
    let next = world
        .try_acquire_line_batch(&dmos, "next frame", 7)
        .unwrap();
    world.try_release(&next).unwrap();
}

#[test]
fn aggregate_add_visits_later_children_after_false_and_get_gates_each_count() {
    let (backend, manager, dmos) = owners();
    let cfg = SimulationWorldLineQueryConfig {
        vehicle_update_in_parallel: false,
        ground: batches(1, 2),
        dmo: batches(1, 5),
        vehicle: batches(1, 4),
    };
    let mut world = SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, cfg).unwrap();
    let batch = world
        .try_acquire_line_batch(&dmos, "unequal capacities", 7)
        .unwrap();
    world.try_begin(&batch).unwrap();
    for index in 0..5 {
        assert_eq!(world.try_add_line(&batch, input()).unwrap(), index < 2);
    }
    for (entry, count) in batch.children.iter().zip([2, 5, 4]) {
        let pool = world.batches_mut(entry.child);
        assert_eq!(pool.test_count(&entry.lease).unwrap(), count);
        // Synthetic child results isolate aggregate indexing/tie selection;
        // the separate staged test executes the actual constructor collections.
        pool.end_line(&entry.lease, |_| {
            let mut hit = CollisionMeshLineNearest::new();
            hit.hit_count = 1;
            hit.line_parameter = 0.5;
            hit.surface_id = u16::try_from(count).unwrap();
            Ok::<_, ()>(hit)
        })
        .unwrap();
    }
    world.try_sync(&batch).unwrap();
    assert_eq!(
        world.try_get_result(&batch, 0).unwrap().unwrap().surface_id,
        2
    );
    assert_eq!(
        world.try_get_result(&batch, 2).unwrap().unwrap().surface_id,
        5
    );
    assert_eq!(
        world.try_get_result(&batch, 4).unwrap().unwrap().surface_id,
        5
    );
    assert!(world.try_get_result(&batch, 5).unwrap().is_none());
    world.try_release(&batch).unwrap();
}

#[test]
fn direct_flags_and_selected_island_determine_retained_children() {
    let (backend, manager, dmos) = owners();
    for (island, flags, expected) in [
        (3, 1, vec![SimulationWorldLineChild::Ground]),
        (
            3,
            6,
            vec![
                SimulationWorldLineChild::Dmo,
                SimulationWorldLineChild::Vehicle,
            ],
        ),
        (
            1,
            7,
            vec![
                SimulationWorldLineChild::Ground,
                SimulationWorldLineChild::Dmo,
            ],
        ),
        (1, 4, vec![]),
    ] {
        let mut world =
            SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, island, config())
                .unwrap();
        let batch = world
            .try_acquire_line_batch(&dmos, "direct Island", flags)
            .unwrap();
        assert_eq!(
            batch
                .children
                .iter()
                .map(|child| child.child)
                .collect::<Vec<_>>(),
            expected
        );
        world.try_release(&batch).unwrap();
        assert_eq!(
            world.try_release(&batch),
            Err(SimulationWorldLineLeaseError::ReleasedLease)
        );
    }
}

#[test]
fn batch_cannot_cross_query_occurrences_and_drop_does_not_release() {
    let (backend, manager, dmos) = owners();
    let mut world =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let mut other =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let batch = world.try_acquire_line_batch(&dmos, "retained", 7).unwrap();
    assert_eq!(
        other.try_begin(&batch),
        Err(SimulationWorldLineLeaseError::ForeignLease)
    );
    assert_eq!(
        other.try_add_line(&batch, input()),
        Err(SimulationWorldLineLeaseError::ForeignLease)
    );
    assert_eq!(
        other.try_sync(&batch),
        Err(SimulationWorldLineLeaseError::ForeignLease)
    );
    assert_eq!(
        other.try_release(&batch),
        Err(SimulationWorldLineLeaseError::ForeignLease)
    );
    assert!(matches!(
        other.try_get_result(&batch, 0),
        Err(SimulationWorldLineLeaseError::ForeignLease)
    ));
    let ground = StaticGroundPool::try_new(1).unwrap();
    assert!(matches!(
        other.try_end(&batch, &ground, &dmos, &mut HostCollisionLineNumeric),
        Err(SimulationWorldLineError::ForeignLease)
    ));
    drop(batch);
    assert!(matches!(
        world.try_acquire_line_batch(&dmos, "not automatically released", 7),
        Err(SimulationWorldLineLeaseError::Batch {
            child: SimulationWorldLineChild::Ground,
            source: CollisionQueryBatchError::Exhausted { .. }
        })
    ));
    let independent = other
        .try_acquire_line_batch(&dmos, "independent", 7)
        .unwrap();
    other.try_release(&independent).unwrap();
}

#[test]
fn single_child_returns_its_own_index_refusal_without_aggregate_count_gate() {
    let (backend, manager, dmos) = owners();
    let mut world =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let batch = world
        .try_acquire_line_batch(&dmos, "direct ground", 1)
        .unwrap();
    world.try_begin(&batch).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    world
        .try_end(&batch, &ground, &dmos, &mut HostCollisionLineNumeric)
        .unwrap();
    world.try_sync(&batch).unwrap();
    assert!(matches!(
        world.try_get_result(&batch, 0),
        Err(SimulationWorldLineLeaseError::Batch {
            child: SimulationWorldLineChild::Ground,
            source: CollisionQueryBatchError::TestIndex { index: 0, count: 0 }
        })
    ));
    world.try_release(&batch).unwrap();
}

#[test]
fn zero_flags_assert_before_acquisition_but_unknown_nonzero_bits_are_allowed() {
    let (backend, manager, dmos) = owners();
    let mut world =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    assert!(matches!(
        world.try_acquire_line_batch(&dmos, "none", 0),
        Err(SimulationWorldLineLeaseError::ZeroFlags)
    ));
    assert!(world.retained_acquisitions.is_empty());
    let empty = world
        .try_acquire_line_batch(&dmos, "unknown nonzero", 8)
        .unwrap();
    assert!(empty.children.is_empty());
    world.try_release(&empty).unwrap();
    let actual = world
        .try_acquire_line_batch(&dmos, "all remain available", 7)
        .unwrap();
    assert_eq!(actual.children.len(), 3);
    world.try_release(&actual).unwrap();
}

#[test]
fn source_query_capacity_projection_distinguishes_ground_and_dynamic_trajectory_tests() {
    let refs = crate::input_cvar_runtime::InputCvarReferences::new_static_initialized();
    assert!(SimulationWorldLineQueryConfig::try_sample_selected(&refs, 3).is_err());
    let mut manager = crate::input_cvar_runtime::InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let config = SimulationWorldLineQueryConfig::try_sample_selected(&refs, 3).unwrap();
    assert_eq!(
        (config.ground.line.batches, config.ground.line.tests),
        (65, 16)
    );
    assert_eq!(
        (config.ground.boxes.batches, config.ground.boxes.tests),
        (8, 16)
    );
    assert_eq!(
        (
            config.ground.trajectory.batches,
            config.ground.trajectory.tests
        ),
        (117, 1)
    );
    for dynamic in [config.dmo, config.vehicle] {
        assert_eq!(
            (dynamic.trajectory.batches, dynamic.trajectory.tests),
            (117, 16)
        );
        assert_eq!(
            (dynamic.step_selector.batches, dynamic.step_selector.tests),
            (18, 1)
        );
    }
    let visual = SimulationWorldLineQueryConfig::try_sample_selected(&refs, 0).unwrap();
    assert_eq!(visual.dmo.trajectory.batches, 1);
    assert_eq!(visual.dmo.step_selector.batches, 1);
    assert_eq!(visual.vehicle.trajectory.batches, 0);
    assert_eq!(visual.vehicle.step_selector.batches, 0);
}
