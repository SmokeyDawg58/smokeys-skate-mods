//! Cross-session ownership tests over real body/Drive pools. Geometry and
//! inertia values are synthetic owned primitives, not a claim of board spawn.
use super::*;
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyDetachError, DynamicAssemblyPartAttachmentMetadata,
        DynamicAssemblyPartAttachmentTemplate, DynamicAssemblyPartPayload,
        DynamicAssemblyRuntimeTemplate,
    },
    simulation_drive_pool::SimulationDriveDynamicsParams,
    simulation_rigid_body_pool::SimulationInertia,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
use skate2_formats::collision::{Volume, VolumeType};

type Backend = SimulationAssemblyBackend<(), (), ()>;
type Parent = DynamicAssemblySession<Backend, Vec<Volume>>;
type Child = DynamicAssemblySession<Backend, Volume>;

fn primitive(kind: VolumeType) -> Volume {
    Volume {
        transform: FRESH_VOLUME_TRANSFORM,
        vtable_offset: kind as u32,
        shape_words: [0; 3],
        radius: 0.5,
        group_id: 0,
        surface_id: 0,
        flags: 1,
    }
}
fn session<C>(geometry: C, count: usize, is_static: u32) -> DynamicAssemblySession<Backend, C> {
    let metadata = (0..count)
        .map(|_| DynamicAssemblyPartAttachmentMetadata {
            inverse_body_transform: FRESH_VOLUME_TRANSFORM,
            has_inverse_body_transform: 0,
            is_static_body: is_static,
        })
        .collect();
    let payloads = (0..count)
        .map(|_| DynamicAssemblyPartPayload {
            inertia: Arc::new(SimulationInertia::native_default()).into(),
            material: (),
        })
        .collect();
    let attachment =
        DynamicAssemblyPartAttachmentTemplate::try_from_parts(metadata, payloads).unwrap();
    let template = DynamicAssemblyRuntimeTemplate::try_from_owned_parts(
        Arc::new(geometry),
        Arc::new(attachment),
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    DynamicAssemblySession::try_new(Arc::new(template), vec![FRESH_VOLUME_TRANSFORM; count])
        .unwrap()
}
fn parent() -> Parent {
    session(vec![primitive(VolumeType::Sphere); 2], 2, 0)
}
fn child() -> Child {
    session(primitive(VolumeType::Box), 1, 1)
}
fn payloads() -> (
    Arc<RwLock<SimulationDriveFrames>>,
    Arc<RwLock<SimulationDriveDynamics>>,
) {
    let params = SimulationDriveDynamicsParams {
        spring: 0.0,
        damping: 0.0,
        strength: 0.0,
        raw_type: 0,
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
fn attached(capacity: usize) -> (Backend, Parent, Child) {
    let mut backend = Backend::try_new(3, 0, capacity).unwrap();
    let mut parent = parent();
    let mut child = child();
    parent.attach(&mut backend, 4).unwrap();
    child.attach(&mut backend, 4).unwrap();
    (backend, parent, child)
}

#[test]
fn different_owned_geometry_types_bind_exact_parent_b_and_child_a_occurrences() {
    let (mut backend, mut parent, mut child) = attached(1);
    let (frames, dynamics) = payloads();
    let body_b = parent.part_attachment.parts[1].body.unwrap().identity();
    let body_a = child.part_attachment.parts[0].body.unwrap().identity();
    assert!(
        parent
            .try_add_cross_attachment_drive(&mut backend, 1, &mut child, 0, &frames, &dynamics)
            .unwrap()
    );
    let snapshot = backend.drive_snapshot();
    let slot = &snapshot.slots[snapshot.active_order[0].index() as usize];
    assert_eq!(slot.body_b, Some(body_b));
    assert_eq!(slot.body_a, Some(body_a));
    assert_eq!(backend.rigid_body_snapshot().counts.active, 2);
    assert!(
        parent.post_attachment_drives[0]
            .child_guard
            .as_ref()
            .unwrap()
            .is_live()
    );
    assert!(child.incoming_cross_drives[0].is_live());
    parent.remove_post_attachment_drives(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
    parent.detach(&mut backend).unwrap();
}

#[test]
fn every_cross_attempt_including_capacity_null_guards_both_sessions_until_cleanup() {
    for capacity in 0..=2 {
        let (mut backend, mut parent, mut child) = attached(capacity);
        let (frames, dynamics) = payloads();
        for ordinal in 0..2 {
            assert_eq!(
                parent
                    .try_add_cross_attachment_drive(
                        &mut backend,
                        ordinal,
                        &mut child,
                        0,
                        &frames,
                        &dynamics
                    )
                    .unwrap(),
                ordinal < capacity
            );
            assert_eq!(
                parent.post_attachment_drives[ordinal].drive.is_some(),
                ordinal < capacity
            );
        }
        assert_eq!(child.incoming_cross_drives.len(), 2);
        assert!(
            child
                .incoming_cross_drives
                .iter()
                .all(DynamicAssemblyCrossDriveGuard::is_live)
        );
        let body_before = backend.rigid_body_snapshot();
        let drives_before = backend.drive_snapshot();
        assert_eq!(
            parent.detach(&mut backend),
            Err(DynamicAssemblyDetachError::PostAttachmentDrivesPresent)
        );
        assert_eq!(
            child.detach(&mut backend),
            Err(DynamicAssemblyDetachError::IncomingCrossDrivesPresent)
        );
        assert_eq!(backend.rigid_body_snapshot(), body_before);
        assert_eq!(backend.drive_snapshot(), drives_before);
        parent.remove_post_attachment_drives(&mut backend).unwrap();
        assert!(parent.post_attachment_drives.is_empty());
        assert!(
            child
                .incoming_cross_drives
                .iter()
                .all(|guard| !guard.is_live())
        );
        assert_eq!(backend.drive_snapshot().counts.active, 0);
        child.detach(&mut backend).unwrap();
        parent.detach(&mut backend).unwrap();
        assert!(child.incoming_cross_drives.is_empty());
        assert_eq!(backend.rigid_body_snapshot().counts.free, 3);
    }
}

#[test]
fn multiple_parents_and_duplicate_edges_release_only_their_own_guards() {
    let mut backend = Backend::try_new(5, 0, 3).unwrap();
    let mut a = parent();
    let mut b = parent();
    let mut child = child();
    a.attach(&mut backend, 4).unwrap();
    b.attach(&mut backend, 4).unwrap();
    child.attach(&mut backend, 4).unwrap();
    let (frames, dynamics) = payloads();
    for _ in 0..2 {
        assert!(
            a.try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics)
                .unwrap()
        );
    }
    assert!(
        b.try_add_cross_attachment_drive(&mut backend, 1, &mut child, 0, &frames, &dynamics)
            .unwrap()
    );
    a.remove_post_attachment_drives(&mut backend).unwrap();
    assert!(!child.incoming_cross_drives[0].is_live());
    assert!(!child.incoming_cross_drives[1].is_live());
    assert!(child.incoming_cross_drives[2].is_live());
    a.detach(&mut backend).unwrap();
    assert_eq!(
        child.detach(&mut backend),
        Err(DynamicAssemblyDetachError::IncomingCrossDrivesPresent)
    );
    assert_eq!(backend.drive_snapshot().counts.active, 1);
    b.remove_post_attachment_drives(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
    b.detach(&mut backend).unwrap();
}

#[test]
fn cleanup_detach_and_reattach_never_reactivate_a_released_guard_or_old_occurrence() {
    let (mut backend, mut parent, mut child) = attached(1);
    let (frames, dynamics) = payloads();
    parent
        .try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics)
        .unwrap();
    let old_guard = child.incoming_cross_drives[0].clone();
    let old_drive = parent.post_attachment_drives[0].drive.unwrap();
    let old_child = child.part_attachment.parts[0].body.unwrap();
    parent.remove_post_attachment_drives(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
    parent.detach(&mut backend).unwrap();
    assert!(!old_guard.is_live());
    parent.attach(&mut backend, 4).unwrap();
    child.attach(&mut backend, 4).unwrap();
    parent
        .try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics)
        .unwrap();
    assert_eq!(child.incoming_cross_drives.len(), 1);
    assert!(child.incoming_cross_drives[0].is_live());
    assert!(!old_guard.is_live());
    assert!(!Arc::ptr_eq(
        &old_guard.0,
        &child.incoming_cross_drives[0].0
    ));
    assert_ne!(parent.post_attachment_drives[0].drive.unwrap(), old_drive);
    assert_ne!(child.part_attachment.parts[0].body.unwrap(), old_child);
    assert_eq!(
        child.detach(&mut backend),
        Err(DynamicAssemblyDetachError::IncomingCrossDrivesPresent)
    );
    parent.remove_post_attachment_drives(&mut backend).unwrap();
    parent.detach(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
}

#[test]
fn attachment_authority_and_index_preflights_publish_no_entries_or_guards() {
    let mut backend = Backend::try_new(3, 0, 0).unwrap();
    let mut parent = parent();
    let mut child = child();
    let (frames, dynamics) = payloads();
    assert!(matches!(
        parent.try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics),
        Err(DynamicAssemblyPostDriveError::NotAttached)
    ));
    parent.attach(&mut backend, 4).unwrap();
    assert!(matches!(
        parent.try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics),
        Err(DynamicAssemblyPostDriveError::NotAttached)
    ));
    child.attach(&mut backend, 4).unwrap();
    let mut other = Backend::try_new(1, 0, 0).unwrap();
    let mut foreign_child = super::cross_tests::child();
    foreign_child.attach(&mut other, 4).unwrap();
    let before = backend.rigid_body_snapshot();
    let drives = backend.drive_snapshot();
    let foreign = other.rigid_body_snapshot();
    assert!(matches!(
        parent.try_add_cross_attachment_drive(&mut other, 0, &mut child, 0, &frames, &dynamics),
        Err(DynamicAssemblyPostDriveError::BackendAuthorityMismatch)
    ));
    assert!(matches!(
        parent.try_add_cross_attachment_drive(
            &mut backend,
            0,
            &mut foreign_child,
            0,
            &frames,
            &dynamics
        ),
        Err(DynamicAssemblyPostDriveError::BackendAuthorityMismatch)
    ));
    assert!(matches!(
        parent.try_add_cross_attachment_drive(&mut backend, 2, &mut child, 0, &frames, &dynamics),
        Err(DynamicAssemblyPostDriveError::PartIndex { index: 2, count: 2 })
    ));
    assert!(matches!(
        parent.try_add_cross_attachment_drive(&mut backend, 0, &mut child, 1, &frames, &dynamics),
        Err(DynamicAssemblyPostDriveError::PartIndex { index: 1, count: 1 })
    ));
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(backend.drive_snapshot(), drives);
    assert_eq!(other.rigid_body_snapshot(), foreign);
    assert!(parent.post_attachment_drives.is_empty());
    assert!(child.incoming_cross_drives.is_empty());
    assert!(foreign_child.incoming_cross_drives.is_empty());
    parent.detach(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
    foreign_child.detach(&mut other).unwrap();
}

#[test]
fn failed_second_backend_admission_retains_first_prefix_but_no_new_guard() {
    let (mut backend, mut parent, mut child) = attached(2);
    let (frames, dynamics) = payloads();
    parent
        .try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics)
        .unwrap();
    let mut other = Backend::try_new(1, 0, 0).unwrap();
    let mut foreign = super::cross_tests::child();
    foreign.attach(&mut other, 4).unwrap();
    // Explicit private fault injection: session authority remains correct but
    // one body token is foreign. Restore it before either normal teardown.
    let actual = child.part_attachment.parts[0].body;
    child.part_attachment.parts[0].body = foreign.part_attachment.parts[0].body;
    assert!(matches!(
        parent.try_add_cross_attachment_drive(&mut backend, 1, &mut child, 0, &frames, &dynamics),
        Err(DynamicAssemblyPostDriveError::Backend(_))
    ));
    child.part_attachment.parts[0].body = actual;
    assert_eq!(parent.post_attachment_drives.len(), 1);
    assert_eq!(child.incoming_cross_drives.len(), 1);
    assert!(child.incoming_cross_drives[0].is_live());
    assert_eq!(backend.drive_snapshot().counts.active, 1);
    assert_eq!(
        parent.detach(&mut backend),
        Err(DynamicAssemblyDetachError::PostAttachmentDrivesPresent)
    );
    assert_eq!(
        child.detach(&mut backend),
        Err(DynamicAssemblyDetachError::IncomingCrossDrivesPresent)
    );
    parent.remove_post_attachment_drives(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
    parent.detach(&mut backend).unwrap();
    foreign.detach(&mut other).unwrap();
}

#[test]
fn poisoned_payloads_do_not_gate_pool_add_and_cleanup_does_not_read_them() {
    for poison_frames in [true, false] {
        let (mut backend, mut parent, mut child) = attached(1);
        let (frames, dynamics) = payloads();
        if poison_frames {
            let owner = Arc::clone(&frames);
            let _ = std::panic::catch_unwind(move || {
                let _guard = owner.write().unwrap();
                panic!("synthetic frame poison");
            });
        } else {
            let owner = Arc::clone(&dynamics);
            let _ = std::panic::catch_unwind(move || {
                let _guard = owner.write().unwrap();
                panic!("synthetic dynamics poison");
            });
        }
        assert!(
            parent
                .try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics)
                .unwrap()
        );
        assert_eq!(backend.drive_snapshot().counts.active, 1);
        assert!(child.incoming_cross_drives[0].is_live());
        assert!(backend.active_drive_payloads_for_test().is_err());
        parent.remove_post_attachment_drives(&mut backend).unwrap();
        assert!(!child.incoming_cross_drives[0].is_live());
        parent.detach(&mut backend).unwrap();
        child.detach(&mut backend).unwrap();
    }
}

#[test]
fn mixed_intra_and_cross_entries_remove_forward_and_release_guards_after_cleanup() {
    let (mut backend, mut parent, mut child) = attached(3);
    let (frames, dynamics) = payloads();
    parent
        .try_add_post_attachment_drive(&mut backend, 0, 1, &frames, &dynamics)
        .unwrap();
    parent
        .try_add_cross_attachment_drive(&mut backend, 1, &mut child, 0, &frames, &dynamics)
        .unwrap();
    parent
        .try_add_post_attachment_drive(&mut backend, 1, 0, &frames, &dynamics)
        .unwrap();
    assert!(parent.post_attachment_drives[0].child_guard.is_none());
    assert!(parent.post_attachment_drives[1].child_guard.is_some());
    assert!(parent.post_attachment_drives[2].child_guard.is_none());
    let expected = backend.drive_snapshot().active_order;
    parent.remove_post_attachment_drives(&mut backend).unwrap();
    assert_eq!(backend.drive_snapshot().free_order, expected);
    assert!(!child.incoming_cross_drives[0].is_live());
    parent.remove_post_attachment_drives(&mut backend).unwrap(); // cleared owner is idempotent
    assert_eq!(backend.drive_snapshot().free_order, expected);
    parent.detach(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
}

#[test]
fn wrong_backend_cleanup_cannot_remove_handle_or_release_child_guard() {
    let (mut backend, mut parent, mut child) = attached(1);
    let (frames, dynamics) = payloads();
    parent
        .try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics)
        .unwrap();
    let mut other = Backend::try_new(0, 0, 0).unwrap();
    let before = backend.drive_snapshot();
    assert!(matches!(
        parent.remove_post_attachment_drives(&mut other),
        Err(DynamicAssemblyPostDriveError::BackendAuthorityMismatch)
    ));
    assert_eq!(backend.drive_snapshot(), before);
    assert!(child.incoming_cross_drives[0].is_live());
    assert_eq!(
        child.detach(&mut backend),
        Err(DynamicAssemblyDetachError::IncomingCrossDrivesPresent)
    );
    parent.remove_post_attachment_drives(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
    parent.detach(&mut backend).unwrap();
}

#[test]
fn cleanup_panic_retains_failed_and_tail_guards_but_retry_skips_removed_prefix() {
    let (mut backend, mut parent, mut child) = attached(3);
    let (frames, dynamics) = payloads();
    for _ in 0..3 {
        assert!(
            parent
                .try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics)
                .unwrap()
        );
    }
    let (mut foreign_backend, mut foreign_parent, mut foreign_child) = attached(1);
    assert!(
        foreign_parent
            .try_add_cross_attachment_drive(
                &mut foreign_backend,
                0,
                &mut foreign_child,
                0,
                &frames,
                &dynamics
            )
            .unwrap()
    );
    let valid_second = parent.post_attachment_drives[1].drive;
    let foreign_handle = foreign_parent.post_attachment_drives[0].drive;
    // Explicit private fault injection of a real foreign occurrence. Production
    // admission cannot produce this entry; it tests interrupted host cleanup.
    parent.post_attachment_drives[1].drive = foreign_handle;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        parent.remove_post_attachment_drives(&mut backend).unwrap();
    }));
    assert!(result.is_err());
    assert!(parent.post_attachment_drives[0].drive.is_none());
    assert!(!child.incoming_cross_drives[0].is_live());
    assert_eq!(parent.post_attachment_drives[1].drive, foreign_handle);
    assert!(child.incoming_cross_drives[1].is_live());
    assert!(parent.post_attachment_drives[2].drive.is_some());
    assert!(child.incoming_cross_drives[2].is_live());
    assert_eq!(backend.drive_snapshot().counts.active, 2);
    assert_eq!(foreign_backend.drive_snapshot().counts.active, 1);
    assert_eq!(
        child.detach(&mut backend),
        Err(DynamicAssemblyDetachError::IncomingCrossDrivesPresent)
    );
    parent.post_attachment_drives[1].drive = valid_second;
    parent.remove_post_attachment_drives(&mut backend).unwrap();
    assert!(parent.post_attachment_drives.is_empty());
    assert!(
        child
            .incoming_cross_drives
            .iter()
            .all(|guard| !guard.is_live())
    );
    assert_eq!(backend.drive_snapshot().counts.active, 0);
    parent.detach(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
    foreign_parent
        .remove_post_attachment_drives(&mut foreign_backend)
        .unwrap();
    foreign_parent.detach(&mut foreign_backend).unwrap();
    foreign_child.detach(&mut foreign_backend).unwrap();
}
