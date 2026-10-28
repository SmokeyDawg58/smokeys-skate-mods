//! Actual mounted board definitions exercise the isolated `AssemblyPool` owner.
//! Controlled capacities/states are test inputs, not a full Island or pair job.
use super::*;
use crate::dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyPool;
use crate::simulation_assembly_backend::constraint_session::skateboard_pool::tests::run_pool_ground_job;
use crate::simulation_contact_jacobian::ground_preparation::{
    SimulationGroundRunError, cpu_job::SimulationStaticGroundJobError,
};
use crate::skateboard_assembly::SkateboardRuntimeTemplate;

const BEND: f32 = f32::from_bits(0x3f7f_be77);

pub(super) fn check(template: Arc<SkateboardRuntimeTemplate>, initial: &[[f32; 16]; 7]) {
    check_capacity_and_swap(&template, initial);
    check_failed_add_retains_issued_index(&template, initial);
    check_full_membership_and_active_subset(&template, initial);
    drop(template);
}

fn candidate(
    template: &Arc<SkateboardRuntimeTemplate>,
    initial: &[[f32; 16]; 7],
) -> SimulationSkaterAssemblyCandidate {
    let session =
        SkateboardAssemblySession::try_new(Arc::clone(template), initial.to_vec()).unwrap();
    session.board_collision_groups.try_initialize(7).unwrap();
    SimulationSkaterAssemblyCandidate::try_new(session).unwrap()
}

#[expect(
    clippy::too_many_lines,
    reason = "one retained two-member lifetime proves capacity, swap-last and index chronology"
)]
fn check_capacity_and_swap(template: &Arc<SkateboardRuntimeTemplate>, initial: &[[f32; 16]; 7]) {
    let mut backend = SkateboardSimulationBackend::try_new(15, 12, 0).unwrap();
    let mut manager = backend.try_manager_for_test(0).unwrap();
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 0, false).unwrap();
    assert_eq!(pool.capacity(), 2);
    let first = pool
        .try_insert(
            &mut backend,
            &mut manager,
            candidate(template, initial),
            4,
            BEND,
        )
        .unwrap();
    let second = pool
        .try_insert(
            &mut backend,
            &mut manager,
            candidate(template, initial),
            4,
            BEND,
        )
        .unwrap();
    assert_eq!(pool.world_index(&first).unwrap(), 0);
    assert_eq!(pool.world_index(&second).unwrap(), 1);
    pool.try_refresh(&backend).unwrap();
    assert_eq!(pool.active_ordinals(), Some([0, 1].as_slice()));
    let moved_observations = pool.publication(&second).unwrap().rigid_bodies().to_vec();
    let moved_records = pool.publication(&second).unwrap().records().to_vec();
    let moved_bounds = pool.publication(&second).unwrap().bounds();
    let before = backend.rigid_body_snapshot();
    let failure = pool
        .try_insert(
            &mut backend,
            &mut manager,
            candidate(template, initial),
            4,
            BEND,
        )
        .err()
        .unwrap();
    assert!(matches!(
        failure.error,
        SimulationSkaterPoolError::Full { capacity: 2 }
    ));
    assert_eq!(failure.candidate.world_index(), 0xffff);
    assert!(!failure.candidate.session().is_attached());
    assert!(Arc::ptr_eq(
        failure.candidate.session().template(),
        template
    ));
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(pool.len(), 2);
    assert_eq!(pool.active_ordinals(), Some([0, 1].as_slice()));
    let removed = pool.try_remove(&mut backend, &first).unwrap();
    assert!(!removed.session().is_attached());
    assert_eq!(removed.world_index(), 0);
    assert!(matches!(
        pool.session(&first),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    assert_eq!(
        pool.active_ordinals(),
        None,
        "Remove does not publish a new active list"
    );
    assert_eq!(
        pool.handles()
            .map(|handle| pool.world_index(handle).unwrap())
            .collect::<Vec<_>>(),
        [1]
    );
    assert!(Arc::ptr_eq(
        pool.session(&second).unwrap().template(),
        template
    ));
    let moved = pool.publication(&second).unwrap();
    assert_eq!(moved.rigid_bodies(), moved_observations);
    assert_eq!(moved.records(), moved_records);
    assert_eq!(moved.bounds(), moved_bounds);
    let replacement = pool
        .try_insert(&mut backend, &mut manager, failure.candidate, 4, BEND)
        .unwrap();
    assert_eq!(
        pool.world_index(&replacement).unwrap(),
        2,
        "Full did not burn an ID and Remove did not refund one"
    );
    assert_eq!(
        pool.handles()
            .map(|handle| pool.world_index(handle).unwrap())
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(
        pool.publications().next().unwrap().rigid_bodies(),
        moved_observations
    );
    pool.try_refresh(&backend).unwrap();
    assert_eq!(pool.active_ordinals(), Some([0, 1].as_slice()));
    for handle in [&second, &replacement] {
        assert!(
            !pool
                .try_remove(&mut backend, handle)
                .unwrap()
                .session()
                .is_attached()
        );
    }
    assert_eq!(pool.len(), 0);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 14);
    assert_eq!(backend.joint_snapshot().counts.free, 12);
}

fn check_failed_add_retains_issued_index(
    template: &Arc<SkateboardRuntimeTemplate>,
    initial: &[[f32; 16]; 7],
) {
    // The manager occupies one body slot. The pool has two membership slots,
    // but only one actual seven-Part board fits in this controlled backend.
    let mut backend = SkateboardSimulationBackend::try_new(8, 12, 0).unwrap();
    let mut manager = backend.try_manager_for_test(0).unwrap();
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 0, false).unwrap();
    let first = pool
        .try_insert(
            &mut backend,
            &mut manager,
            candidate(template, initial),
            4,
            BEND,
        )
        .unwrap();
    let before = backend.rigid_body_snapshot();
    let failure = pool
        .try_insert(
            &mut backend,
            &mut manager,
            candidate(template, initial),
            4,
            BEND,
        )
        .err()
        .unwrap();
    assert!(matches!(
        failure.error,
        SimulationSkaterPoolError::Attach(DynamicAssemblyAttachError::PartUnavailable {
            ordinal: 0
        })
    ));
    assert_eq!(failure.candidate.world_index(), 1);
    assert!(!failure.candidate.session().is_attached());
    assert!(Arc::ptr_eq(
        failure.candidate.session().template(),
        template
    ));
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(pool.len(), 1);

    // A larger foreign backend does not repair a pool bound to the original
    // manager. Both failed preflight and the real returned owner remain visible.
    let mut foreign = SkateboardSimulationBackend::try_new(15, 12, 0).unwrap();
    let mut foreign_manager = foreign.try_manager_for_test(0).unwrap();
    let foreign_before = foreign.rigid_body_snapshot();
    let rejected = pool
        .try_insert(
            &mut foreign,
            &mut foreign_manager,
            failure.candidate,
            4,
            BEND,
        )
        .err()
        .unwrap();
    assert!(matches!(
        rejected.error,
        SimulationSkaterPoolError::ForeignBackend
    ));
    assert_eq!(rejected.candidate.world_index(), 1);
    assert_eq!(foreign.rigid_body_snapshot(), foreign_before);
    assert_eq!(backend.rigid_body_snapshot(), before);
    let rejected = pool
        .try_insert(
            &mut backend,
            &mut foreign_manager,
            rejected.candidate,
            4,
            BEND,
        )
        .err()
        .unwrap();
    assert!(matches!(
        rejected.error,
        SimulationSkaterPoolError::ForeignManager
    ));
    assert_eq!(rejected.candidate.world_index(), 1);
    assert_eq!(backend.rigid_body_snapshot(), before);
    drop(pool.try_remove(&mut backend, &first).unwrap());
    let retried = pool
        .try_insert(&mut backend, &mut manager, rejected.candidate, 4, BEND)
        .unwrap();
    assert_eq!(
        pool.world_index(&retried).unwrap(),
        2,
        "the failed Add consumed its real manager ID"
    );
    assert!(Arc::ptr_eq(
        pool.session(&retried).unwrap().template(),
        template
    ));
    assert_eq!(pool.publication(&retried).unwrap().records().len(), 19);
    drop(pool.try_remove(&mut backend, &retried).unwrap());
    assert_eq!(backend.rigid_body_snapshot().counts.free, 7);
    assert_eq!(backend.joint_snapshot().counts.free, 12);
}

fn check_full_membership_and_active_subset(
    template: &Arc<SkateboardRuntimeTemplate>,
    initial: &[[f32; 16]; 7],
) {
    let mut backend = SkateboardSimulationBackend::try_new(22, 18, 0).unwrap();
    let mut manager = backend.try_manager_for_test(0).unwrap();
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 3, false).unwrap();
    assert_eq!(pool.capacity(), 16);
    let handles: Vec<_> = [4, 2, 1]
        .into_iter()
        .map(|state| {
            pool.try_insert(
                &mut backend,
                &mut manager,
                candidate(template, initial),
                state,
                BEND,
            )
            .unwrap()
        })
        .collect();
    let snapshot = backend.rigid_body_snapshot();
    // Exact Add states 4/2/1 survive for this actual nonstatic definition.
    // Do not substitute flags6, whose Add path normalizes to Active instead.
    for (ordinal, (handle, state)) in handles.iter().zip([4, 2, 1]).enumerate() {
        assert_eq!(
            pool.world_index(handle).unwrap(),
            u32::try_from(ordinal).unwrap()
        );
        for observation in pool.publication(handle).unwrap().rigid_bodies() {
            assert_eq!(
                snapshot.slots[observation.identity().id().index() as usize].state,
                state
            );
        }
    }
    assert_eq!(pool.active_ordinals(), None);
    backend.begin_dynamic_update_frame(f32::from_bits(0x3c88_8889));
    let ground = crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_robs_park_ground_for_board_test();
    assert!(
        matches!(
            run_pool_ground_job(&mut backend, &pool, &manager, &ground),
            Err(SimulationStaticGroundJobError::Run {
                assembly: 0,
                source: SimulationGroundRunError::StaleAssemblyBounds,
            })
        ),
        "actual ground consumer must refuse a stale owned publication"
    );
    pool.try_refresh(&backend).unwrap();
    assert_eq!(pool.active_ordinals(), Some([0, 2].as_slice()));
    assert_eq!(
        pool.active_frame(),
        backend.rigid_body_pool_for_test().frame_identity()
    );
    // Ground's full membership producer retains the frozen and Static boards;
    // its downstream job gates, not this active list, decide their processing.
    assert_eq!(pool.publications().len(), 3);
    assert_eq!(
        pool.publications()
            .map(|publication| (
                publication.num_static(),
                publication.all_parts_frozen(),
                publication.records().len()
            ))
            .collect::<Vec<_>>(),
        [(0, false, 19), (0, true, 19), (7, false, 19)]
    );
    for (publication, handle) in pool.publications().zip(&handles) {
        assert_eq!(
            publication.rigid_bodies(),
            pool.publication(handle).unwrap().rigid_bodies()
        );
        assert!(publication.has_bounds_for_frame(pool.active_frame().unwrap()));
        assert!(Arc::ptr_eq(
            pool.session(handle).unwrap().template(),
            template
        ));
    }
    let output = run_pool_ground_job(&mut backend, &pool, &manager, &ground).unwrap();
    assert_eq!(output.report.processed_assemblies, 1);
    assert_eq!(
        output.report.skipped_assemblies, 2,
        "full three-member ground traversal includes both frozen and static skips"
    );
    // All three boards remain at their source initial poses, far from this
    // resident test mesh. This distinguishes membership selection, not contacts.
    assert!(output.into_records().is_empty());
    for handle in handles {
        drop(pool.try_remove(&mut backend, &handle).unwrap());
    }
    pool.try_refresh(&backend).unwrap();
    assert_eq!(pool.active_ordinals(), Some([].as_slice()));
    assert_eq!(pool.publications().len(), 0);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 21);
    assert_eq!(backend.joint_snapshot().counts.free, 18);
}
