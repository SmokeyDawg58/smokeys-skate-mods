//! Synthetic typed assembly owners over real pools. Empty/two-Part definitions
//! discriminate membership traversal; the source hook itself has one Part.
use super::*;
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyPartAttachmentMetadata, DynamicAssemblyPartAttachmentTemplate,
        DynamicAssemblyPartPayload, DynamicAssemblyRuntimeTemplate,
    },
    simulation_drive_pool::SimulationDriveDynamicsParams,
    simulation_rigid_body_pool::SimulationInertia,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};

type Pool = SimulationHookAssemblyPool<(), ()>;
type Candidate = SimulationHookAssemblyCandidate<(), ()>;

fn session(count: usize) -> Session<(), ()> {
    let attachment = DynamicAssemblyPartAttachmentTemplate::try_from_parts(
        (0..count)
            .map(|_| DynamicAssemblyPartAttachmentMetadata {
                inverse_body_transform: FRESH_VOLUME_TRANSFORM,
                has_inverse_body_transform: 0,
                is_static_body: 0,
            })
            .collect(),
        (0..count)
            .map(|_| DynamicAssemblyPartPayload {
                inertia: Arc::new(SimulationInertia::native_default()).into(),
                material: (),
            })
            .collect(),
    )
    .unwrap();
    let template = DynamicAssemblyRuntimeTemplate::try_from_owned_parts(
        Arc::new(()),
        Arc::new(attachment),
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    Session::try_new(Arc::new(template), vec![FRESH_VOLUME_TRANSFORM; count]).unwrap()
}
fn candidate(count: usize) -> Candidate {
    Candidate::try_new(session(count)).unwrap()
}
fn domain(
    body_capacity: usize,
    drive_capacity: usize,
) -> (Backend<()>, SimulationManagerContactDomain) {
    let mut backend = Backend::try_new(body_capacity, 0, drive_capacity).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    (backend, manager)
}
fn payloads() -> (
    Arc<RwLock<SimulationDriveFrames>>,
    Arc<RwLock<SimulationDriveDynamics>>,
) {
    let params = SimulationDriveDynamicsParams {
        spring: 0.0,
        damping: 0.0,
        strength: 0.0,
        raw_type: 2,
    };
    (
        Arc::new(RwLock::new(SimulationDriveFrames {
            quaternion_a: [0.0, 0.0, 0.0, 1.0],
            position_a: [0.0; 4],
            quaternion_b: [0.0, 0.0, 0.0, 1.0],
            position_b: [0.0; 4],
        })),
        Arc::new(RwLock::new(SimulationDriveDynamics {
            linear: params,
            angular: params,
        })),
    )
}
fn member_bodies(
    pool: &Pool,
    backend: &Backend<()>,
    manager: &SimulationManagerContactDomain,
) -> Vec<SimulationRigidBodyObservation> {
    let mut visits = Vec::new();
    pool.try_for_each_member_body(
        manager.assembly_pool_authority(),
        backend.rigid_body_pool_for_test(),
        |body| {
            visits.push(body);
            Ok(())
        },
    )
    .unwrap();
    visits
}

#[test]
fn source_flag_bit_selects_sixteen_or_two_and_host_full_refuses_before_id_consumption() {
    for (flags, capacity) in [
        (0, 2),
        (2, 2),
        (0x8000_0000, 2),
        (1, 16),
        (3, 16),
        (0x8000_0001, 16),
    ] {
        let (mut backend, mut manager) = domain(1, 0);
        let mut pool = Pool::try_new(&manager, &backend, flags).unwrap();
        assert_eq!(pool.capacity, capacity);
        assert_eq!(pool.len(), 0);
        let mut handles = Vec::new();
        for ordinal in 0..capacity {
            let handle = pool
                .try_insert(&mut backend, &mut manager, candidate(0))
                .unwrap();
            assert_eq!(
                pool.try_snapshot(&handle).unwrap().world_index,
                u32::try_from(ordinal).unwrap()
            );
            handles.push(handle);
        }
        let before = backend.rigid_body_snapshot();
        let failure = pool
            .try_insert(&mut backend, &mut manager, candidate(0))
            .unwrap_err();
        assert!(
            matches!(failure.error, SimulationHookPoolError::Full { capacity: value } if value == capacity)
        );
        assert_eq!(failure.candidate.world_index(), 0xffff);
        assert!(!failure.candidate.session().is_attached());
        assert_eq!(backend.rigid_body_snapshot(), before);
        let mut second = Pool::try_new(&manager, &backend, flags).unwrap();
        let next = second
            .try_insert(&mut backend, &mut manager, failure.candidate)
            .unwrap();
        assert_eq!(
            second.try_snapshot(&next).unwrap().world_index,
            u32::try_from(capacity).unwrap()
        );
        for handle in handles {
            pool.try_remove(&mut backend, &handle).unwrap();
        }
        second.try_remove(&mut backend, &next).unwrap();
    }
}

#[test]
fn two_pools_share_manager_ids_and_keep_assembly_metadata_distinct_from_part_defaults() {
    let (mut backend, mut manager) = domain(4, 0);
    let mut a = Pool::try_new(&manager, &backend, 3).unwrap();
    let mut b = Pool::try_new(&manager, &backend, 3).unwrap();
    let first = a
        .try_insert(&mut backend, &mut manager, candidate(1))
        .unwrap();
    let second = b
        .try_insert(&mut backend, &mut manager, candidate(2))
        .unwrap();
    let third = a
        .try_insert(&mut backend, &mut manager, candidate(0))
        .unwrap();
    assert_eq!(a.try_snapshot(&first).unwrap().world_index, 0);
    let published = b.try_snapshot(&second).unwrap();
    assert_eq!(published.world_index, 1);
    assert_eq!(published.group_id, 1);
    assert_eq!(published.assembly_user_data, None);
    assert_eq!(
        published.parts,
        vec![
            SimulationHookPartPublication {
                world_index: 0,
                user_data: 1
            };
            2
        ]
    );
    assert_eq!(
        published.bounds.map(|row| row.map(f32::to_bits)),
        [[0; 4]; 2]
    );
    assert_eq!(a.try_snapshot(&third).unwrap().world_index, 2);
    assert_eq!(backend.rigid_body_snapshot().counts.static_bodies, 4);
    assert_eq!(backend.rigid_body_snapshot().counts.active, 0);
    let removed = b.try_remove(&mut backend, &second).unwrap();
    assert_eq!(removed.world_index(), 1);
    assert!(!removed.session().is_attached());
    a.try_remove(&mut backend, &first).unwrap();
    a.try_remove(&mut backend, &third).unwrap();
}

#[test]
fn backend_and_manager_binding_refusals_return_untouched_candidate_without_effects() {
    let (mut backend, mut manager) = domain(2, 0);
    let (mut foreign_backend, mut foreign_manager) = domain(2, 0);
    assert!(matches!(
        Pool::try_new(&manager, &foreign_backend, 3),
        Err(SimulationHookPoolError::ForeignBackend)
    ));
    let mut pool = Pool::try_new(&manager, &backend, 3).unwrap();
    let value = candidate(1);
    let owner = Arc::clone(value.session().template());
    let before = backend.rigid_body_snapshot();
    let foreign_before = foreign_backend.rigid_body_snapshot();
    let failure = pool
        .try_insert(&mut foreign_backend, &mut manager, value)
        .unwrap_err();
    assert!(matches!(
        failure.error,
        SimulationHookPoolError::ForeignBackend
    ));
    assert!(Arc::ptr_eq(failure.candidate.session().template(), &owner));
    let failure = pool
        .try_insert(&mut backend, &mut foreign_manager, failure.candidate)
        .unwrap_err();
    assert!(matches!(
        failure.error,
        SimulationHookPoolError::ForeignManager
    ));
    assert_eq!(failure.candidate.world_index(), 0xffff);
    assert_eq!(pool.len(), 0);
    let mut visits = 0;
    assert!(matches!(
        pool.try_for_each_member_body(
            foreign_manager.assembly_pool_authority(),
            backend.rigid_body_pool_for_test(),
            |_| {
                visits += 1;
                Ok(())
            }
        ),
        Err(SimulationHookPoolError::ForeignManager)
    ));
    assert!(matches!(
        pool.try_for_each_member_body(
            manager.assembly_pool_authority(),
            foreign_backend.rigid_body_pool_for_test(),
            |_| {
                visits += 1;
                Ok(())
            }
        ),
        Err(SimulationHookPoolError::ForeignBackend)
    ));
    assert_eq!(visits, 0);
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(foreign_backend.rigid_body_snapshot(), foreign_before);
    let accepted = pool
        .try_insert(&mut backend, &mut manager, failure.candidate)
        .unwrap();
    assert_eq!(pool.try_snapshot(&accepted).unwrap().world_index, 0);
    pool.try_remove(&mut backend, &accepted).unwrap();
}

#[test]
fn swap_remove_changes_member_traversal_but_preserves_each_members_part_order() {
    let (mut backend, mut manager) = domain(5, 0);
    let mut pool = Pool::try_new(&manager, &backend, 3).unwrap();
    let first = pool
        .try_insert(&mut backend, &mut manager, candidate(2))
        .unwrap();
    let empty = pool
        .try_insert(&mut backend, &mut manager, candidate(0))
        .unwrap();
    let last = pool
        .try_insert(&mut backend, &mut manager, candidate(2))
        .unwrap();
    let initial = member_bodies(&pool, &backend, &manager);
    let body_pool = backend.rigid_body_pool_for_test();
    let expected: Vec<_> = pool
        .members
        .iter()
        .flat_map(|member| {
            member
                .candidate
                .attachment()
                .parts
                .iter()
                .map(|part| body_pool.try_observe(part.body.unwrap()).unwrap())
        })
        .collect();
    assert_eq!(initial, expected);
    assert_eq!(initial.len(), 4);
    assert_eq!(
        initial
            .iter()
            .map(|body| body.identity().id().index())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    // Removing first moves the LAST member to index0, ahead of the empty one.
    pool.try_remove(&mut backend, &first).unwrap();
    assert!(Arc::ptr_eq(&pool.members[0].handle.0, &last.0));
    assert!(Arc::ptr_eq(&pool.members[1].handle.0, &empty.0));
    assert_eq!(member_bodies(&pool, &backend, &manager), initial[2..]);
    pool.try_remove(&mut backend, &empty).unwrap();
    assert_eq!(member_bodies(&pool, &backend, &manager), initial[2..]);
    pool.try_remove(&mut backend, &last).unwrap();
    assert!(member_bodies(&pool, &backend, &manager).is_empty());
}

#[test]
fn foreign_and_removed_membership_handles_cannot_remove_new_occurrences() {
    let (mut backend, mut manager) = domain(3, 0);
    let mut a = Pool::try_new(&manager, &backend, 3).unwrap();
    let mut b = Pool::try_new(&manager, &backend, 3).unwrap();
    let old = a
        .try_insert(&mut backend, &mut manager, candidate(1))
        .unwrap();
    let foreign = b
        .try_insert(&mut backend, &mut manager, candidate(1))
        .unwrap();
    let before = backend.rigid_body_snapshot();
    assert!(matches!(
        a.try_snapshot(&foreign),
        Err(SimulationHookPoolError::MissingMember)
    ));
    assert!(matches!(
        a.try_remove(&mut backend, &foreign),
        Err(SimulationHookPoolError::MissingMember)
    ));
    let (mut foreign_backend, _) = domain(1, 0);
    assert!(matches!(
        a.try_remove(&mut foreign_backend, &old),
        Err(SimulationHookPoolError::ForeignBackend)
    ));
    assert_eq!(backend.rigid_body_snapshot(), before);
    let returned = a.try_remove(&mut backend, &old).unwrap();
    let new = a.try_insert(&mut backend, &mut manager, returned).unwrap();
    assert!(!Arc::ptr_eq(&old.0, &new.0));
    let before = backend.rigid_body_snapshot();
    assert!(matches!(
        a.try_remove(&mut backend, &old),
        Err(SimulationHookPoolError::MissingMember)
    ));
    assert!(matches!(
        a.try_snapshot(&old),
        Err(SimulationHookPoolError::MissingMember)
    ));
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
    a.try_remove(&mut backend, &new).unwrap();
    b.try_remove(&mut backend, &foreign).unwrap();
}

#[test]
fn body_capacity_failure_consumes_id_and_returns_the_actual_rolled_back_owner() {
    let (mut backend, mut manager) = domain(2, 0);
    let mut pool = Pool::try_new(&manager, &backend, 3).unwrap();
    let mut attached = session(1);
    attached.attach(&mut backend, 4).unwrap();
    let attached_owner = Arc::clone(attached.template());
    let before = backend.rigid_body_snapshot();
    let Err(mut failure) = Candidate::try_new(attached) else {
        panic!("already-attached candidate must be refused");
    };
    assert!(matches!(
        failure.error,
        SimulationHookPoolError::AlreadyAttached
    ));
    assert!(failure.session.is_attached());
    assert!(Arc::ptr_eq(failure.session.template(), &attached_owner));
    assert_eq!(backend.rigid_body_snapshot(), before);
    failure.session.detach(&mut backend).unwrap();
    let value = candidate(2);
    let retained = Arc::clone(value.session().template());
    let failure = pool
        .try_insert(&mut backend, &mut manager, value)
        .unwrap_err();
    assert!(matches!(
        failure.error,
        SimulationHookPoolError::Attach(DynamicAssemblyAttachError::PartUnavailable { ordinal: 1 })
    ));
    assert_eq!(failure.candidate.world_index(), 0);
    assert!(!failure.candidate.session().is_attached());
    assert!(Arc::ptr_eq(
        failure.candidate.session().template(),
        &retained
    ));
    assert!(
        failure
            .candidate
            .session
            .part_attachment
            .parts
            .iter()
            .all(|part| part.body.is_none())
    );
    assert_eq!(failure.candidate.session().part_transforms().count(), 2);
    assert_eq!(pool.len(), 0);
    assert_eq!(backend.rigid_body_snapshot().counts.static_bodies, 1);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 1);
    let accepted = pool
        .try_insert(&mut backend, &mut manager, candidate(1))
        .unwrap();
    assert_eq!(pool.try_snapshot(&accepted).unwrap().world_index, 1);
    pool.try_remove(&mut backend, &accepted).unwrap();
}

#[test]
fn capacity_null_cross_drive_still_retains_member_and_detach_guard_until_explicit_cleanup() {
    for drive_capacity in [0, 1] {
        let (mut backend, mut manager) = domain(9, drive_capacity);
        let mut board = session(7);
        board.attach(&mut backend, 4).unwrap();
        let mut pool = Pool::try_new(&manager, &backend, 3).unwrap();
        let hook = pool
            .try_insert(&mut backend, &mut manager, candidate(1))
            .unwrap();
        let (frames, dynamics) = payloads();
        assert_eq!(
            pool.try_connect_board(&mut backend, &mut board, &hook, &frames, &dynamics)
                .unwrap(),
            drive_capacity != 0
        );
        assert_eq!(pool.len(), 1);
        assert_eq!(backend.rigid_body_snapshot().counts.static_bodies, 2);
        assert_eq!(backend.rigid_body_snapshot().counts.active, 7);
        let before = backend.rigid_body_snapshot();
        assert!(matches!(
            pool.try_remove(&mut backend, &hook),
            Err(SimulationHookPoolError::Detach(
                DynamicAssemblyDetachError::IncomingCrossDrivesPresent
            ))
        ));
        assert_eq!(
            board.detach(&mut backend),
            Err(DynamicAssemblyDetachError::PostAttachmentDrivesPresent)
        );
        assert_eq!(backend.rigid_body_snapshot(), before);
        board.remove_post_attachment_drives(&mut backend).unwrap();
        let removed = pool.try_remove(&mut backend, &hook).unwrap();
        assert!(!removed.session().is_attached());
        assert_eq!(backend.rigid_body_snapshot().counts.active, 7);
        board.detach(&mut backend).unwrap();
        assert_eq!(pool.len(), 0);
        assert_eq!(backend.drive_snapshot().counts.active, 0);
        assert_eq!(backend.rigid_body_snapshot().counts.static_bodies, 1);
    }
}

#[test]
fn source_external_drive_order_finishes_before_hook_body_then_board_body_removal() {
    let (mut backend, mut manager) = domain(9, 3);
    let mut board = session(7);
    board.attach(&mut backend, 4).unwrap();
    let (frames, dynamics) = payloads();
    for child in [4, 5] {
        assert!(
            board
                .try_add_post_attachment_drive(&mut backend, 6, child, &frames, &dynamics)
                .unwrap()
        );
    }
    let mut pool = Pool::try_new(&manager, &backend, 3).unwrap();
    let hook = pool
        .try_insert(&mut backend, &mut manager, candidate(1))
        .unwrap();
    assert!(
        pool.try_connect_board(&mut backend, &mut board, &hook, &frames, &dynamics)
            .unwrap()
    );
    let active_order = backend.drive_snapshot().active_order;
    assert!(matches!(
        pool.try_remove(&mut backend, &hook),
        Err(SimulationHookPoolError::Detach(
            DynamicAssemblyDetachError::IncomingCrossDrivesPresent
        ))
    ));
    board.remove_post_attachment_drives(&mut backend).unwrap();
    assert_eq!(backend.drive_snapshot().free_order, active_order);
    pool.try_remove(&mut backend, &hook).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.static_bodies, 1);
    assert_eq!(backend.rigid_body_snapshot().counts.active, 7);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 1);
    board.detach(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.free, 8);
}
