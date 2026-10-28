//! Synthetic owned Part definitions admitted through the real pool. These
//! fixtures test capability/state ordering, not original physical-player spawn.
use super::*;
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyDetachError, DynamicAssemblyPartAttachmentMetadata,
        DynamicAssemblyPartAttachmentTemplate, DynamicAssemblyPartPayload,
        DynamicAssemblyRuntimeTemplate, DynamicAssemblySimulationBackend,
    },
    simulation_drive_pool::{
        SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
    },
    simulation_rigid_body_pool::SimulationInertia,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
use std::sync::{Arc, RwLock};

type Backend = SimulationAssemblyBackend<(), (), ()>;
type Session = DynamicAssemblySession<Backend, ()>;

fn fixture(count: usize) -> Session {
    let metadata = (0..count)
        .map(|_| DynamicAssemblyPartAttachmentMetadata {
            inverse_body_transform: FRESH_VOLUME_TRANSFORM,
            has_inverse_body_transform: 0,
            is_static_body: 0,
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
        Arc::new(()),
        Arc::new(attachment),
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    Session::try_new(Arc::new(template), vec![FRESH_VOLUME_TRANSFORM; count]).unwrap()
}

#[test]
fn ascending_registration_changes_only_state_and_keeps_actual_occurrences() {
    let mut backend = Backend::try_new(3, 0, 0).unwrap();
    let mut session = fixture(3);
    session.attach(&mut backend, 4).unwrap();
    let original = backend.rigid_body_snapshot();
    let mut expected = original.clone();
    for part in 0..3 {
        let body = session.part_attachment.parts[part].body.unwrap();
        let observed = session
            .try_part_contact_spy_observation(&backend, part)
            .unwrap();
        assert_eq!(observed, body.observation());
        assert_eq!(
            session
                .try_register_part_contact_spy(&mut backend, part)
                .unwrap(),
            observed
        );
        expected
            .slots
            .iter_mut()
            .find(|slot| slot.id == body.id())
            .unwrap()
            .state |= 8;
        assert_eq!(backend.rigid_body_snapshot(), expected);
        assert_eq!(
            session
                .try_register_part_contact_spy(&mut backend, part)
                .unwrap(),
            observed
        );
        assert_eq!(backend.rigid_body_snapshot(), expected);
    }
    assert_eq!(session.try_validate_detach(&backend), Ok(()));
    assert_eq!(backend.rigid_body_snapshot(), expected);
    for part in 0..3 {
        let body = session.part_attachment.parts[part].body.unwrap();
        assert_eq!(
            session
                .try_remove_part_contact_spy(&mut backend, part)
                .unwrap(),
            body.observation()
        );
        expected
            .slots
            .iter_mut()
            .find(|slot| slot.id == body.id())
            .unwrap()
            .state &= 7;
        assert_eq!(backend.rigid_body_snapshot(), expected);
    }
    assert_eq!(backend.rigid_body_snapshot(), original);
    session.detach(&mut backend).unwrap();
}

#[test]
fn detached_foreign_index_and_missing_handle_fail_before_state_effects() {
    use DynamicAssemblyPartContactSpyError as Error;
    let mut backend = Backend::try_new(1, 0, 0).unwrap();
    let mut foreign = Backend::try_new(1, 0, 0).unwrap();
    let mut session = fixture(1);
    assert_eq!(
        session.try_register_part_contact_spy(&mut backend, 99),
        Err(Error::NotAttached)
    );
    assert_eq!(
        session.try_validate_detach(&foreign),
        Err(DynamicAssemblyDetachError::NotAttached)
    );
    session.attach(&mut backend, 4).unwrap();
    let original = backend.rigid_body_snapshot();
    let foreign_before = foreign.rigid_body_snapshot();
    assert_eq!(
        session.try_register_part_contact_spy(&mut foreign, 99),
        Err(Error::BackendAuthorityMismatch)
    );
    assert_eq!(
        session.try_remove_part_contact_spy(&mut foreign, 0),
        Err(Error::BackendAuthorityMismatch)
    );
    assert_eq!(
        session.try_validate_detach(&foreign),
        Err(DynamicAssemblyDetachError::BackendAuthorityMismatch)
    );
    assert_eq!(
        session.try_register_part_contact_spy(&mut backend, 1),
        Err(Error::PartIndex { index: 1, count: 1 })
    );
    // Explicit malformed private fixture; no production owner can lose its handle this way.
    let body = session.part_attachment.parts[0].body.take();
    assert_eq!(
        session.try_remove_part_contact_spy(&mut backend, 0),
        Err(Error::MissingPartBody { index: 0 })
    );
    session.part_attachment.parts[0].body = body;
    assert_eq!(backend.rigid_body_snapshot(), original);
    assert_eq!(foreign.rigid_body_snapshot(), foreign_before);
    session.detach(&mut backend).unwrap();
}

#[test]
fn actual_remove_reuse_rejects_stale_part_without_touching_replacement() {
    let mut backend = Backend::try_new(1, 0, 0).unwrap();
    let mut session = fixture(1);
    session.attach(&mut backend, 4).unwrap();
    let old = session.part_attachment.parts[0].body.unwrap();
    // Controlled private corruption: retire a real capability behind its owner,
    // then obtain an actual replacement from the same real pool.
    backend.remove_rigid_body(&old);
    let inertia = Arc::new(SimulationInertia::native_default()).into();
    let replacement = backend
        .add_rigid_body(&FRESH_VOLUME_TRANSFORM, &inertia, 4)
        .unwrap();
    assert_eq!(old.id(), replacement.id());
    assert_ne!(old.observation(), replacement.observation());
    let before = backend.rigid_body_snapshot();
    for result in [
        session.try_part_contact_spy_observation(&backend, 0),
        session.try_register_part_contact_spy(&mut backend, 0),
        session.try_remove_part_contact_spy(&mut backend, 0),
    ] {
        assert!(matches!(
            result,
            Err(DynamicAssemblyPartContactSpyError::Body(
                SimulationRigidBodyPoolError::StaleHandle { .. }
            ))
        ));
    }
    assert_eq!(backend.rigid_body_snapshot(), before);
    session.part_attachment.parts[0].body = Some(replacement);
    session.detach(&mut backend).unwrap();
}

#[test]
fn capacity_null_cross_drive_still_blocks_both_detach_preflights_without_effects() {
    let mut backend = Backend::try_new(2, 0, 0).unwrap();
    let mut parent = fixture(1);
    let mut child = fixture(1);
    parent.attach(&mut backend, 4).unwrap();
    child.attach(&mut backend, 4).unwrap();
    parent
        .try_register_part_contact_spy(&mut backend, 0)
        .unwrap();
    child
        .try_register_part_contact_spy(&mut backend, 0)
        .unwrap();
    let frames = Arc::new(RwLock::new(SimulationDriveFrames {
        quaternion_a: [0.0, 0.0, 0.0, 1.0],
        position_a: [0.0; 4],
        quaternion_b: [0.0, 0.0, 0.0, 1.0],
        position_b: [0.0; 4],
    }));
    let params = SimulationDriveDynamicsParams {
        spring: 0.0,
        damping: 0.0,
        strength: 0.0,
        raw_type: 0,
    };
    let dynamics = Arc::new(RwLock::new(SimulationDriveDynamics {
        linear: params,
        angular: params,
    }));
    assert!(
        !parent
            .try_add_cross_attachment_drive(&mut backend, 0, &mut child, 0, &frames, &dynamics)
            .unwrap()
    );
    let before = backend.rigid_body_snapshot();
    assert_eq!(
        parent.try_validate_detach(&backend),
        Err(DynamicAssemblyDetachError::PostAttachmentDrivesPresent)
    );
    assert_eq!(
        child.try_validate_detach(&backend),
        Err(DynamicAssemblyDetachError::IncomingCrossDrivesPresent)
    );
    assert_eq!(
        parent.detach(&mut backend),
        Err(DynamicAssemblyDetachError::PostAttachmentDrivesPresent)
    );
    assert_eq!(
        child.detach(&mut backend),
        Err(DynamicAssemblyDetachError::IncomingCrossDrivesPresent)
    );
    assert_eq!(backend.rigid_body_snapshot(), before);
    parent.remove_post_attachment_drives(&mut backend).unwrap();
    assert_eq!(parent.try_validate_detach(&backend), Ok(()));
    assert_eq!(child.try_validate_detach(&backend), Ok(()));
    parent.try_remove_part_contact_spy(&mut backend, 0).unwrap();
    child.try_remove_part_contact_spy(&mut backend, 0).unwrap();
    parent.detach(&mut backend).unwrap();
    child.detach(&mut backend).unwrap();
}
