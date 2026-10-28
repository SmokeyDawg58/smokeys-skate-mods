//! Private ownership extension for drives created after Assembly admission.
//!
//! Skateboard `CreateDrivesWithOffset` (0x82b37b88) does not roll back native
//! capacity nulls. Its destructor (0x82b2b040) removes truck drives in ascending
//! order before removing the board assembly. This extension retains tokens in
//! the existing session without exposing body capabilities or inventing IDs.
//! `CreateHookDrive` (0x82b2e700) additionally connects two real assemblies;
//! shared host detach guards protect its independently owned child assembly.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaiting live skater construction")
)]

use super::{DynamicAssemblySession, DynamicAssemblySimulationBackend};
use crate::{
    simulation_assembly_backend::{SimulationAssemblyBackend, SimulationAssemblyBackendAddFailure},
    simulation_drive_pool::{SimulationDriveDynamics, SimulationDriveFrames},
};
use std::{
    fmt,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

#[cfg(test)]
#[path = "dynamic_assembly_cross_drives_tests.rs"]
mod cross_tests;

/// Shared host detach protection, not a body capability or a native field.
/// Dropping an owner does not clear this guard or implicitly remove a Drive.
#[derive(Clone)]
pub(super) struct DynamicAssemblyCrossDriveGuard(Arc<AtomicBool>);

impl DynamicAssemblyCrossDriveGuard {
    fn new() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }

    pub(super) fn is_live(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    fn release(&self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(super) struct DynamicAssemblyPostDrive<H> {
    drive: Option<H>,
    child_guard: Option<DynamicAssemblyCrossDriveGuard>,
}

#[derive(Debug)]
pub(crate) enum DynamicAssemblyPostDriveError {
    NotAttached,
    BackendAuthorityMismatch,
    PartIndex { index: usize, count: usize },
    StorageAllocation,
    Backend(SimulationAssemblyBackendAddFailure),
    DriveSlot(crate::simulation_drive_pool::SimulationDrivePoolError),
    DriveIndex { index: usize, count: usize },
    NullDrive { index: usize },
    ReleasedCrossDrive { index: usize },
    OutstandingDrives,
}

impl fmt::Display for DynamicAssemblyPostDriveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAttached => f.write_str("post-admission drive requires attached bodies"),
            Self::BackendAuthorityMismatch => f.write_str("post-admission drive backend mismatch"),
            Self::PartIndex { index, count } => {
                write!(f, "drive part {index} outside {count} parts")
            }
            Self::StorageAllocation => f.write_str("could not retain post-admission drive token"),
            Self::Backend(error) => write!(f, "post-admission drive: {error}"),
            Self::DriveSlot(error) => write!(f, "post-admission drive slot: {error}"),
            Self::DriveIndex { index, count } => {
                write!(f, "post-admission drive {index} outside {count} entries")
            }
            Self::NullDrive { index } => write!(f, "post-admission drive {index} is null"),
            Self::ReleasedCrossDrive { index } => write!(
                f,
                "cross-session drive {index} has been explicitly released"
            ),
            Self::OutstandingDrives => f.write_str("post-admission drive cleanup is incomplete"),
        }
    }
}
impl std::error::Error for DynamicAssemblyPostDriveError {}

impl<M, JF: Copy, JL: Copy, C> DynamicAssemblySession<SimulationAssemblyBackend<M, JF, JL>, C> {
    /// Owner-only guard for the exact existing same-session Drive. The result
    /// distinguishes a recorded capacity null from a live retained occurrence;
    /// neither case acquires a payload lock or changes the original ledger.
    pub(super) fn try_validate_post_attachment_drive(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
        parent: usize,
        child: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, DynamicAssemblyPostDriveError> {
        self.try_validate_cross_attachment_drive(
            backend, index, parent, self, child, frames, dynamics,
        )
    }

    /// Same guard for a source hook's retained cross-session entry. Source Part
    /// ordinals resolve through the two actual attached sessions; callers never
    /// supply numeric Body/Drive IDs. Both authorities are checked even for null.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual parent ledger and two distinct session endpoint owners"
    )]
    pub(super) fn try_validate_cross_attachment_drive<C2>(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
        parent: usize,
        child_session: &DynamicAssemblySession<SimulationAssemblyBackend<M, JF, JL>, C2>,
        child: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, DynamicAssemblyPostDriveError> {
        self.validate_post_drive_backend(backend)?;
        child_session.validate_post_drive_backend(backend)?;
        for (index, count) in [
            (parent, self.part_attachment.parts.len()),
            (child, child_session.part_attachment.parts.len()),
        ] {
            if index >= count {
                return Err(DynamicAssemblyPostDriveError::PartIndex { index, count });
            }
        }
        let entry = self.post_attachment_drives.get(index).ok_or(
            DynamicAssemblyPostDriveError::DriveIndex {
                index,
                count: self.post_attachment_drives.len(),
            },
        )?;
        if entry
            .child_guard
            .as_ref()
            .is_some_and(|guard| !guard.is_live())
        {
            return Err(DynamicAssemblyPostDriveError::ReleasedCrossDrive { index });
        }
        let parent_body = self.part_attachment.parts[parent]
            .body
            .ok_or(DynamicAssemblyPostDriveError::NotAttached)?;
        let child_body = child_session.part_attachment.parts[child]
            .body
            .ok_or(DynamicAssemblyPostDriveError::NotAttached)?;
        backend
            .try_validate_owned_drive_payloads(
                entry.drive,
                parent_body,
                child_body,
                frames,
                dynamics,
            )
            .map_err(DynamicAssemblyPostDriveError::DriveSlot)
    }

    /// Adds one source-ordered drive, retaining success BEFORE returning to a
    /// caller that may subsequently fail. False is a native capacity null;
    /// errors are separate, failure-atomic host refusals. Earlier drives remain.
    pub(crate) fn try_add_post_attachment_drive(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        parent: usize,
        child: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, DynamicAssemblyPostDriveError> {
        self.validate_post_drive_backend(backend)?;
        let count = self.part_attachment.parts.len();
        for index in [parent, child] {
            if index >= count {
                return Err(DynamicAssemblyPostDriveError::PartIndex { index, count });
            }
        }
        self.post_attachment_drives
            .try_reserve(1)
            .map_err(|_| DynamicAssemblyPostDriveError::StorageAllocation)?;
        let result = backend.add_drive(
            self.part_attachment.attached_body(parent),
            self.part_attachment.attached_body(child),
            frames,
            dynamics,
        );
        if let Some(error) = backend.take_last_add_failure() {
            return Err(DynamicAssemblyPostDriveError::Backend(error));
        }
        self.post_attachment_drives.push(DynamicAssemblyPostDrive {
            drive: result,
            child_guard: None,
        });
        Ok(result.is_some())
    }

    /// `CreateHookDrive` (0x82b2e700): parent/B and child/A belong to separate
    /// actual assemblies in the SAME backend. Both endpoint capabilities stay
    /// private. The owning parent retains the result before returning; a shared
    /// guard prevents child detach until explicit parent-side drive cleanup.
    ///
    /// Both session authorities, indices and vector reservations are host
    /// preflight. Arc allocation follows Rust allocation policy. Native capacity
    /// null remains false with no body rollback. Recording/guarding that null
    /// attempt until cleanup is conservative host recovery of a native-fault
    /// path, not evidence that a null result created a live constraint.
    pub(crate) fn try_add_cross_attachment_drive<C2>(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        parent: usize,
        child_session: &mut DynamicAssemblySession<SimulationAssemblyBackend<M, JF, JL>, C2>,
        child: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, DynamicAssemblyPostDriveError> {
        self.validate_post_drive_backend(backend)?;
        child_session.validate_post_drive_backend(backend)?;
        for (index, count) in [
            (parent, self.part_attachment.parts.len()),
            (child, child_session.part_attachment.parts.len()),
        ] {
            if index >= count {
                return Err(DynamicAssemblyPostDriveError::PartIndex { index, count });
            }
        }
        self.post_attachment_drives
            .try_reserve(1)
            .map_err(|_| DynamicAssemblyPostDriveError::StorageAllocation)?;
        child_session
            .incoming_cross_drives
            .try_reserve(1)
            .map_err(|_| DynamicAssemblyPostDriveError::StorageAllocation)?;
        let guard = DynamicAssemblyCrossDriveGuard::new();
        let result = backend.add_drive(
            self.part_attachment.attached_body(parent),
            child_session.part_attachment.attached_body(child),
            frames,
            dynamics,
        );
        if let Some(error) = backend.take_last_add_failure() {
            return Err(DynamicAssemblyPostDriveError::Backend(error));
        }
        self.post_attachment_drives.push(DynamicAssemblyPostDrive {
            drive: result,
            child_guard: Some(guard.clone()),
        });
        child_session.incoming_cross_drives.push(guard);
        Ok(result.is_some())
    }

    /// Source-success order is ordinal forward, before Assembly body removal.
    /// Skipping missing tokens and clearing the host list is recovery policy:
    /// native RemoveDrive(0x82ae3360) dereferences null, and the native destructor
    /// neither clears these pointers nor resets its count. Never called on Drop.
    pub(crate) fn remove_post_attachment_drives(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
    ) -> Result<(), DynamicAssemblyPostDriveError> {
        self.validate_post_drive_backend(backend)?;
        for entry in &mut self.post_attachment_drives {
            if let Some(drive) = &entry.drive {
                backend.remove_drive(drive);
                // Host bookkeeping follows each completed removal, so a
                // caught later panic cannot leave a stale successful prefix.
                entry.drive = None;
            }
            // Release only AFTER actual removal. A panic cannot release the
            // failed entry's child guard. Nulls use the documented host recovery.
            if let Some(guard) = &entry.child_guard {
                guard.release();
            }
        }
        self.post_attachment_drives.clear();
        Ok(())
    }

    /// Owner-only indexed store for source `BoneDrives`' unconditional spy write.
    /// A recorded capacity null is a typed host stop, not a successful drive.
    pub(super) fn try_set_post_attachment_drive_spy(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
        spy: u32,
    ) -> Result<(), DynamicAssemblyPostDriveError> {
        self.validate_post_drive_backend(backend)?;
        let entry = self.post_attachment_drives.get(index).ok_or(
            DynamicAssemblyPostDriveError::DriveIndex {
                index,
                count: self.post_attachment_drives.len(),
            },
        )?;
        let drive = entry
            .drive
            .ok_or(DynamicAssemblyPostDriveError::NullDrive { index })?;
        backend
            .try_set_owned_drive_spy(drive, spy)
            .map_err(DynamicAssemblyPostDriveError::DriveSlot)
    }

    /// Source-selected removal order; stable host ordinals are not native IDs.
    /// Successful removal releases the cross-owner guard before later entries.
    pub(super) fn try_remove_post_attachment_drive_at(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<(), DynamicAssemblyPostDriveError> {
        self.validate_post_drive_backend(backend)?;
        let count = self.post_attachment_drives.len();
        let entry = self
            .post_attachment_drives
            .get_mut(index)
            .ok_or(DynamicAssemblyPostDriveError::DriveIndex { index, count })?;
        if let Some(drive) = &entry.drive {
            backend.remove_drive(drive);
            entry.drive = None;
        }
        if let Some(guard) = &entry.child_guard {
            guard.release();
        }
        Ok(())
    }

    pub(super) fn try_finish_post_attachment_drive_cleanup(
        &mut self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
    ) -> Result<(), DynamicAssemblyPostDriveError> {
        self.validate_post_drive_backend(backend)?;
        if self.post_attachment_drives.iter().any(|entry| {
            entry.drive.is_some()
                || entry
                    .child_guard
                    .as_ref()
                    .is_some_and(DynamicAssemblyCrossDriveGuard::is_live)
        }) {
            return Err(DynamicAssemblyPostDriveError::OutstandingDrives);
        }
        self.post_attachment_drives.clear();
        Ok(())
    }

    fn validate_post_drive_backend(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
    ) -> Result<(), DynamicAssemblyPostDriveError> {
        let Some(authority) = self.attached_authority else {
            return Err(DynamicAssemblyPostDriveError::NotAttached);
        };
        if authority != backend.authority() {
            return Err(DynamicAssemblyPostDriveError::BackendAuthorityMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
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

    type Backend = SimulationAssemblyBackend<(), (), ()>;
    type Session = DynamicAssemblySession<Backend, ()>;

    fn session() -> Session {
        let metadata = (0..2)
            .map(|_| DynamicAssemblyPartAttachmentMetadata {
                inverse_body_transform: FRESH_VOLUME_TRANSFORM,
                has_inverse_body_transform: 0,
                is_static_body: 0,
            })
            .collect();
        let payloads = (0..2)
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
        Session::try_new(Arc::new(template), vec![FRESH_VOLUME_TRANSFORM; 2]).unwrap()
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

    #[test]
    fn live_guard_checks_exact_endpoints_and_payload_owners_without_mutation() {
        use crate::simulation_drive_pool::{SimulationDriveEndpoint, SimulationDrivePoolError};
        let mut backend = Backend::try_new(2, 0, 1).unwrap();
        let mut owner = session();
        owner.attach(&mut backend, 4).unwrap();
        let (frames, dynamics) = payloads();
        owner
            .try_add_post_attachment_drive(&mut backend, 0, 1, &frames, &dynamics)
            .unwrap();
        let before = backend.drive_snapshot();
        assert!(
            owner
                .try_validate_post_attachment_drive(&backend, 0, 0, 1, &frames, &dynamics)
                .unwrap()
        );
        assert!(matches!(
            owner.try_validate_post_attachment_drive(&backend, 0, 1, 0, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::DriveSlot(
                SimulationDrivePoolError::EndpointMismatch {
                    endpoint: SimulationDriveEndpoint::Parent,
                    ..
                }
            ))
        ));
        let (equal_frames, equal_dynamics) = payloads();
        assert!(matches!(
            owner.try_validate_post_attachment_drive(&backend, 0, 0, 1, &equal_frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::DriveSlot(
                SimulationDrivePoolError::FramesOwnerMismatch { .. }
            ))
        ));
        assert!(matches!(
            owner.try_validate_post_attachment_drive(&backend, 0, 0, 1, &frames, &equal_dynamics),
            Err(DynamicAssemblyPostDriveError::DriveSlot(
                SimulationDrivePoolError::DynamicsOwnerMismatch { .. }
            ))
        ));
        let foreign = Backend::try_new(0, 0, 0).unwrap();
        assert!(matches!(
            owner.try_validate_post_attachment_drive(&foreign, 0, 0, 1, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::BackendAuthorityMismatch)
        ));
        assert!(matches!(
            owner.try_validate_post_attachment_drive(&backend, 1, 0, 1, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::DriveIndex { index: 1, count: 1 })
        ));
        assert_eq!(backend.drive_snapshot(), before);
        owner.remove_post_attachment_drives(&mut backend).unwrap();
        owner.detach(&mut backend).unwrap();
        assert!(matches!(
            owner.try_validate_post_attachment_drive(&backend, 0, 0, 1, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::NotAttached)
        ));
    }

    #[test]
    fn nullable_cross_drive_is_absent_but_released_entry_is_not_a_fresh_null() {
        for capacity in [0, 1] {
            let mut backend = Backend::try_new(4, 0, capacity).unwrap();
            let mut parent = session();
            let mut child = session();
            parent.attach(&mut backend, 1).unwrap();
            child.attach(&mut backend, 4).unwrap();
            let (frames, dynamics) = payloads();
            assert_eq!(
                parent
                    .try_add_cross_attachment_drive(
                        &mut backend,
                        0,
                        &mut child,
                        1,
                        &frames,
                        &dynamics
                    )
                    .unwrap(),
                capacity != 0
            );
            assert_eq!(
                parent
                    .try_validate_cross_attachment_drive(
                        &backend, 0, 0, &child, 1, &frames, &dynamics
                    )
                    .unwrap(),
                capacity != 0
            );
            parent
                .try_remove_post_attachment_drive_at(&mut backend, 0)
                .unwrap();
            assert!(matches!(
                parent.try_validate_cross_attachment_drive(
                    &backend, 0, 0, &child, 1, &frames, &dynamics
                ),
                Err(DynamicAssemblyPostDriveError::ReleasedCrossDrive { index: 0 })
            ));
            parent
                .try_finish_post_attachment_drive_cleanup(&backend)
                .unwrap();
            child.detach(&mut backend).unwrap();
            parent.detach(&mut backend).unwrap();
        }
    }

    #[test]
    fn guard_never_reads_poisoned_payloads_and_rejects_retained_stale_occurrence() {
        use crate::simulation_drive_pool::SimulationDrivePoolError;
        let mut backend = Backend::try_new(2, 0, 1).unwrap();
        let mut owner = session();
        owner.attach(&mut backend, 4).unwrap();
        let (frames, dynamics) = payloads();
        owner
            .try_add_post_attachment_drive(&mut backend, 0, 1, &frames, &dynamics)
            .unwrap();
        let original = owner.post_attachment_drives[0].drive;
        assert!(
            std::panic::catch_unwind(|| {
                let _guard = frames.write().unwrap();
                panic!("controlled frames poison");
            })
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(|| {
                let _guard = dynamics.write().unwrap();
                panic!("controlled dynamics poison");
            })
            .is_err()
        );
        assert!(frames.is_poisoned() && dynamics.is_poisoned());
        assert!(
            owner
                .try_validate_post_attachment_drive(&backend, 0, 0, 1, &frames, &dynamics)
                .unwrap(),
            "identity preflight must not lock, read, repair or reject poisoned numeric storage"
        );
        owner
            .try_remove_post_attachment_drive_at(&mut backend, 0)
            .unwrap();
        // Controlled private stale-ledger corruption uses the actual old token,
        // not an invented slot/occurrence. Guard must reject without mutation.
        owner.post_attachment_drives[0].drive = original;
        assert!(matches!(
            owner.try_validate_post_attachment_drive(&backend, 0, 0, 1, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::DriveSlot(
                SimulationDrivePoolError::DriveIsFree { .. }
            ))
        ));
        owner.post_attachment_drives[0].drive = None;
        let (fresh_frames, fresh_dynamics) = payloads();
        owner
            .try_add_post_attachment_drive(&mut backend, 0, 1, &fresh_frames, &fresh_dynamics)
            .unwrap();
        owner.post_attachment_drives[0].drive = original;
        assert!(matches!(
            owner.try_validate_post_attachment_drive(&backend, 0, 0, 1, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::DriveSlot(
                SimulationDrivePoolError::StaleHandle { .. }
            ))
        ));
        owner.post_attachment_drives[0].drive = None;
        owner.remove_post_attachment_drives(&mut backend).unwrap();
        owner.detach(&mut backend).unwrap();
    }

    #[test]
    fn actual_pool_nulls_do_not_rollback_and_cleanup_precedes_bodies() {
        for capacity in 0..=2 {
            let mut backend = Backend::try_new(2, 0, capacity).unwrap();
            let mut session = session();
            session.attach(&mut backend, 4).unwrap();
            let (frames, dynamics) = payloads();
            for ordinal in 0..2 {
                assert_eq!(
                    session
                        .try_add_post_attachment_drive(&mut backend, 0, 1, &frames, &dynamics)
                        .unwrap(),
                    ordinal < capacity
                );
            }
            assert_eq!(
                backend.drive_snapshot().counts.active,
                u32::try_from(capacity).unwrap()
            );
            assert_eq!(session.post_attachment_drives.len(), 2);
            assert_eq!(
                session.detach(&mut backend),
                Err(DynamicAssemblyDetachError::PostAttachmentDrivesPresent)
            );
            assert_eq!(backend.rigid_body_snapshot().counts.active, 2);
            session.remove_post_attachment_drives(&mut backend).unwrap();
            assert_eq!(
                backend.drive_snapshot().counts.free,
                u32::try_from(capacity).unwrap()
            );
            assert_eq!(
                backend
                    .drive_snapshot()
                    .free_order
                    .iter()
                    .map(|id| id.index())
                    .collect::<Vec<_>>(),
                (0..u32::try_from(capacity).unwrap()).collect::<Vec<_>>()
            );
            session.detach(&mut backend).unwrap();
            assert_eq!(backend.rigid_body_snapshot().counts.free, 2);
        }
    }

    #[test]
    fn host_preflight_errors_have_no_effect_on_either_authority() {
        let mut owner = Backend::try_new(2, 0, 2).unwrap();
        let mut other = Backend::try_new(2, 0, 2).unwrap();
        let mut session = session();
        let (frames, dynamics) = payloads();
        assert!(matches!(
            session.try_add_post_attachment_drive(&mut owner, 0, 1, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::NotAttached)
        ));
        session.attach(&mut owner, 4).unwrap();
        assert!(matches!(
            session.try_add_post_attachment_drive(&mut other, 0, 1, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::BackendAuthorityMismatch)
        ));
        assert!(matches!(
            session.try_add_post_attachment_drive(&mut owner, 2, 1, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::PartIndex { index: 2, count: 2 })
        ));
        assert!(matches!(
            session.try_add_post_attachment_drive(&mut owner, 0, 2, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::PartIndex { index: 2, count: 2 })
        ));
        assert!(
            session
                .try_add_post_attachment_drive(&mut owner, 0, 1, &frames, &dynamics)
                .unwrap()
        );
        assert!(matches!(
            session.remove_post_attachment_drives(&mut other),
            Err(DynamicAssemblyPostDriveError::BackendAuthorityMismatch)
        ));
        assert_eq!(owner.drive_snapshot().counts.active, 1);
        assert_eq!(other.drive_snapshot().counts.active, 0);
        session.remove_post_attachment_drives(&mut owner).unwrap();
        session.detach(&mut owner).unwrap();
    }

    #[test]
    fn pool_reads_observe_mutation_and_removed_slots_retain_payload_owners() {
        let mut backend = Backend::try_new(2, 0, 2).unwrap();
        let mut session = session();
        session.attach(&mut backend, 4).unwrap();
        let (frames, dynamics) = payloads();
        let (second_frames, second_dynamics) = payloads();
        let retained = Arc::downgrade(&dynamics);
        assert!(
            session
                .try_add_post_attachment_drive(&mut backend, 0, 1, &frames, &dynamics)
                .unwrap()
        );
        assert!(
            session
                .try_add_post_attachment_drive(&mut backend, 0, 1, &second_frames, &second_dynamics)
                .unwrap()
        );
        dynamics.write().unwrap().angular.spring = 7.0;
        frames.write().unwrap().position_b[0] = 3.0;
        let observed = backend.active_drive_payloads_for_test().unwrap();
        assert_eq!(observed[0].1.angular.spring.to_bits(), 7.0_f32.to_bits());
        assert_eq!(observed[0].0.position_b[0].to_bits(), 3.0_f32.to_bits());
        assert_eq!(observed[1].1.angular.spring.to_bits(), 0);
        drop(dynamics);
        session.remove_post_attachment_drives(&mut backend).unwrap();
        assert!(retained.upgrade().is_some());
        session.detach(&mut backend).unwrap();
        drop(backend);
        assert!(retained.upgrade().is_none());
    }

    #[test]
    fn foreign_second_body_is_host_error_and_first_drive_stays_owned() {
        let mut backend = Backend::try_new(2, 0, 2).unwrap();
        let mut session = session();
        session.attach(&mut backend, 4).unwrap();
        let (frames, dynamics) = payloads();
        assert!(
            session
                .try_add_post_attachment_drive(&mut backend, 0, 1, &frames, &dynamics)
                .unwrap()
        );
        // Internal fault injection, never a production capability API: retain
        // the correct token, temporarily substitute a different authority.
        let mut other = Backend::try_new(2, 0, 0).unwrap();
        let mut other_session = super::tests::session();
        other_session.attach(&mut other, 4).unwrap();
        let original = session.part_attachment.parts[1].body;
        session.part_attachment.parts[1].body = other_session.part_attachment.parts[1].body;
        assert!(matches!(
            session.try_add_post_attachment_drive(&mut backend, 0, 1, &frames, &dynamics),
            Err(DynamicAssemblyPostDriveError::Backend(_))
        ));
        session.part_attachment.parts[1].body = original;
        assert_eq!(session.post_attachment_drives.len(), 1);
        assert_eq!(backend.drive_snapshot().counts.active, 1);
        assert_eq!(
            session.detach(&mut backend),
            Err(DynamicAssemblyDetachError::PostAttachmentDrivesPresent)
        );
        session.remove_post_attachment_drives(&mut backend).unwrap();
        session.detach(&mut backend).unwrap();
        other_session.detach(&mut other).unwrap();
    }
}
