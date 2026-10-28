//! Actual nonserialized `HookPool` membership, not a collision address image.
//!
//! Insert 0x82e0dbb0 consumes the shared manager ID before Assembly admission;
//! Remove 0x82e0c1c8 detaches before swap removal; single-player setup 0x827b2880
//! walks current members/Parts for SetAssemblyIsland(-1). Host storage combines
//! native member/BBox arrays; allocator layout and unused job storage are not
//! reconstructed by this restricted, serial ownership path.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "HookPool awaits live skater scheduling")
)]

use super::{
    DynamicAssemblyAttachError, DynamicAssemblyDetachError, DynamicAssemblyPostDriveError,
    DynamicAssemblySession,
};
use crate::{
    simulation_assembly_backend::SimulationAssemblyBackend,
    simulation_contact_jacobian::network_preparation::{
        SimulationManagerContactAuthority, SimulationManagerContactDomain,
    },
    simulation_drive_pool::{SimulationDriveDynamics, SimulationDriveFrames},
    simulation_island_map::persistent::{
        SimulationRigidBodyIslandState, SimulationRigidBodyIslandStateError,
    },
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
    simulation_rigid_body_pool::{
        SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolAuthority,
    },
    skeleton_drives::SkeletonHookDefinition,
};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

#[cfg(test)]
#[path = "dynamic_assembly_hook_pool_tests.rs"]
mod tests;

type Backend<M> = SimulationAssemblyBackend<M, SimulationJointFrames, SimulationJointLimits>;
type Session<M, C> = DynamicAssemblySession<Backend<M>, C>;

/// A host membership capability. Arc identity is deliberately independent of
/// the wrapping native world index; no address is exposed or used as game data.
#[derive(Clone, Debug)]
pub(crate) struct SimulationHookAssemblyHandle(Arc<()>);

pub(crate) struct SimulationHookAssemblyCandidate<M, C> {
    session: Session<M, C>,
    world_index: u32,
    part_publication: Vec<SimulationHookPartPublication>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationHookPartPublication {
    pub(crate) world_index: u32,
    pub(crate) user_data: u32,
}

impl<M, C> SimulationHookAssemblyCandidate<M, C> {
    /// Explicit host cleanup of a returned admission candidate. The same
    /// authority/detach guards apply; an issued world index is never refunded.
    pub(crate) fn try_detach_failed_admission(
        &mut self,
        backend: &mut Backend<M>,
    ) -> Result<(), SimulationHookPoolError> {
        if self.session.is_attached() {
            self.session
                .detach(backend)
                .map_err(SimulationHookPoolError::Detach)?;
        }
        Ok(())
    }

    /// `CreateHookDrive` 0x82b2e698/6b0 supplies group1 and Part userData1.
    /// Part world index remains its Initialize default0; Assembly startsffff.
    /// No invented initialized Assembly userData is stored.
    pub(crate) fn try_new(
        session: Session<M, C>,
    ) -> Result<Self, Box<SimulationHookCandidateFailure<M, C>>> {
        let prepared = (|| {
            if session.is_attached() {
                return Err(SimulationHookPoolError::AlreadyAttached);
            }
            let mut parts = Vec::new();
            parts
                .try_reserve_exact(session.part_attachment.parts.len())
                .map_err(|_| SimulationHookPoolError::Allocation)?;
            parts.resize(
                session.part_attachment.parts.len(),
                SimulationHookPartPublication {
                    world_index: 0,
                    user_data: 1,
                },
            );
            Ok(parts)
        })();
        match prepared {
            Ok(part_publication) => Ok(Self {
                session,
                world_index: 0xffff,
                part_publication,
            }),
            Err(error) => Err(Box::new(SimulationHookCandidateFailure { error, session })),
        }
    }

    pub(crate) const fn world_index(&self) -> u32 {
        self.world_index
    }
    pub(crate) fn session(&self) -> &Session<M, C> {
        &self.session
    }
}

/// Construction refusal returns the input owner, including any live handles.
/// No invalid attached input is silently dropped/orphaned by a host preflight.
pub(crate) struct SimulationHookCandidateFailure<M, C> {
    pub(crate) error: SimulationHookPoolError,
    pub(crate) session: Session<M, C>,
}
impl<M, C> fmt::Debug for SimulationHookCandidateFailure<M, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationHookCandidateFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

struct HookBoardTransform<'a, M, C, P> {
    pool: &'a mut SimulationHookAssemblyPool<M, C>,
    backend: &'a mut Backend<M>,
    board: &'a mut Session<M, P>,
    hook: &'a SimulationHookAssemblyHandle,
}

impl<M, C, P> crate::skateboard_spawn::SkateboardSpawnParts for HookBoardTransform<'_, M, C, P> {
    type Error = SimulationHookPoolError;

    fn live_board_part(&mut self, part: usize) -> Result<[f32; 16], Self::Error> {
        self.board
            .try_live_part_transform_host(self.backend, part)
            .map_err(SimulationHookPoolError::Transform)
    }

    fn set_board_part(&mut self, part: usize, matrix: [f32; 16]) -> Result<(), Self::Error> {
        self.board
            .try_set_part_transform_host(self.backend, part, matrix)
            .map_err(SimulationHookPoolError::Transform)
    }

    fn set_hook_part(&mut self, matrix: [f32; 16]) -> Result<(), Self::Error> {
        self.pool.check_backend(self.backend.authority())?;
        let index = self.pool.member_index(self.hook)?;
        self.pool.members[index]
            .candidate
            .existing_mut()?
            .session
            .try_set_part_transform_host(self.backend, 0, matrix)
            .map_err(SimulationHookPoolError::Transform)
    }
}

struct Member<M, C> {
    handle: SimulationHookAssemblyHandle,
    candidate: HookMemberCandidate<M, C>,
    bounds: [[f32; 4]; 2],
}

// Each variant retains its original complete session/template and publication.
// No erased or rebuilt definition is used to share the native pool.
enum HookMemberCandidate<M, C> {
    Existing(SimulationHookAssemblyCandidate<M, C>),
    Skeleton(SimulationHookAssemblyCandidate<M, SkeletonHookDefinition>),
}

impl<M, C> HookMemberCandidate<M, C> {
    fn existing_mut(
        &mut self,
    ) -> Result<&mut SimulationHookAssemblyCandidate<M, C>, SimulationHookPoolError> {
        match self {
            Self::Existing(value) => Ok(value),
            Self::Skeleton(_) => Err(SimulationHookPoolError::WrongMemberKind),
        }
    }
    fn world_index(&self) -> u32 {
        match self {
            Self::Existing(value) => value.world_index,
            Self::Skeleton(value) => value.world_index,
        }
    }
    fn publication(&self) -> &[SimulationHookPartPublication] {
        match self {
            Self::Existing(value) => &value.part_publication,
            Self::Skeleton(value) => &value.part_publication,
        }
    }
    fn attachment(&self) -> &super::DynamicAssemblyPartAttachmentTransaction<Backend<M>> {
        match self {
            Self::Existing(value) => &value.session.part_attachment,
            Self::Skeleton(value) => &value.session.part_attachment,
        }
    }
    fn authority(&self) -> Option<SimulationRigidBodyPoolAuthority> {
        match self {
            Self::Existing(value) => value.session.attached_authority,
            Self::Skeleton(value) => value.session.attached_authority,
        }
    }
}

#[path = "../skeleton/dynamic_assembly_skeleton_hook_pool.rs"]
mod skeleton_hooks;

pub(crate) struct SimulationHookAssemblyPool<M, C> {
    manager: SimulationManagerContactAuthority,
    body_pool: SimulationRigidBodyPoolAuthority,
    capacity: usize,
    members: Vec<Member<M, C>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SimulationHookPublicationSnapshot {
    pub(crate) world_index: u32,
    pub(crate) group_id: u32,
    /// None means not initialized by this native path, NOT a null pointer claim.
    pub(crate) assembly_user_data: Option<u32>,
    pub(crate) parts: Vec<SimulationHookPartPublication>,
    pub(crate) bounds: [[f32; 4]; 2],
}

#[derive(Debug)]
pub(crate) enum SimulationHookPoolError {
    SpawnCvar(crate::input_cvar_runtime::InputCvarError),
    Allocation,
    Full { capacity: usize },
    ForeignBackend,
    ForeignManager,
    AlreadyAttached,
    MissingMember,
    WrongMemberKind,
    Attach(DynamicAssemblyAttachError),
    Detach(DynamicAssemblyDetachError),
    Drive(DynamicAssemblyPostDriveError),
    Labels(SimulationRigidBodyIslandStateError),
    Transform(super::part_transform::DynamicAssemblyPartTransformError),
}
impl fmt::Display for SimulationHookPoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpawnCvar(error) => write!(f, "HookPool spawn cvar: {error:?}"),
            Self::Allocation => f.write_str("HookPool host allocation failed"),
            Self::Full { capacity } => write!(f, "HookPool capacity {capacity} reached"),
            Self::ForeignBackend => f.write_str("HookPool backend authority mismatch"),
            Self::ForeignManager => f.write_str("HookPool manager authority mismatch"),
            Self::AlreadyAttached => f.write_str("HookPool candidate is already attached"),
            Self::MissingMember => f.write_str("HookPool member is foreign or removed"),
            Self::WrongMemberKind => {
                f.write_str("HookPool operation requires a different member kind")
            }
            Self::Attach(error) => write!(f, "HookPool admission: {error}"),
            Self::Detach(error) => write!(f, "HookPool detach: {error}"),
            Self::Drive(error) => write!(f, "HookPool drive: {error}"),
            Self::Labels(error) => write!(f, "HookPool labels: {error:?}"),
            Self::Transform(error) => write!(f, "HookPool Part transform: {error}"),
        }
    }
}

/// The actual detached owner and already-consumed ID survive an attach failure.
/// Native Insert instead publishes even after failed Add; that unsafe path is
/// explicitly refused. Box/Arc allocation follows Rust allocation policy.
pub(crate) struct SimulationHookInsertFailure<M, C> {
    pub(crate) error: SimulationHookPoolError,
    pub(crate) candidate: SimulationHookAssemblyCandidate<M, C>,
}
impl<M, C> fmt::Debug for SimulationHookInsertFailure<M, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationHookInsertFailure")
            .field("error", &self.error)
            .field("world_index", &self.candidate.world_index)
            .finish_non_exhaustive()
    }
}

impl<M, C> SimulationHookAssemblyPool<M, C> {
    /// `InitWorld` 0x827b4340/4374/4384: no serialization, capacity16 if bit0,
    /// otherwise2. Source job size1/parallelfalse; only membership is used here.
    pub(crate) fn try_new(
        manager: &SimulationManagerContactDomain,
        backend: &Backend<M>,
        island_flags: u32,
    ) -> Result<Self, SimulationHookPoolError> {
        if manager.assembly_body_pool_authority() != backend.authority() {
            return Err(SimulationHookPoolError::ForeignBackend);
        }
        let capacity = if island_flags & 1 != 0 { 16 } else { 2 };
        let mut members = Vec::new();
        members
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationHookPoolError::Allocation)?;
        Ok(Self {
            manager: manager.assembly_pool_authority(),
            body_pool: backend.authority(),
            capacity,
            members,
        })
    }

    pub(crate) fn try_insert(
        &mut self,
        backend: &mut Backend<M>,
        manager: &mut SimulationManagerContactDomain,
        mut candidate: SimulationHookAssemblyCandidate<M, C>,
    ) -> Result<SimulationHookAssemblyHandle, Box<SimulationHookInsertFailure<M, C>>> {
        let result = (|| {
            self.check_authorities(manager.assembly_pool_authority(), backend.authority())?;
            if candidate.session.is_attached() {
                return Err(SimulationHookPoolError::AlreadyAttached);
            }
            if self.members.len() == self.capacity {
                return Err(SimulationHookPoolError::Full {
                    capacity: self.capacity,
                });
            }
            // All host storage is available before consuming a native ID/Add.
            let handle = SimulationHookAssemblyHandle(Arc::new(()));
            candidate.world_index = manager
                .try_allocate_assembly_world_index(self.body_pool)
                .map_err(SimulationHookPoolError::Labels)?;
            candidate
                .session
                .attach(backend, 1)
                .map_err(SimulationHookPoolError::Attach)?;
            Ok(handle)
        })();
        match result {
            Ok(handle) => {
                self.members.push(Member {
                    handle: handle.clone(),
                    candidate: HookMemberCandidate::Existing(candidate),
                    bounds: [[0.0; 4]; 2],
                });
                Ok(handle)
            }
            Err(error) => Err(Box::new(SimulationHookInsertFailure { error, candidate })),
        }
    }

    /// Narrow deck6 -> hook0 borrow; callers cannot detach or mutate membership
    /// through an exported mutable member session. The board retains the Drive.
    pub(crate) fn try_connect_board<P>(
        &mut self,
        backend: &mut Backend<M>,
        board: &mut Session<M, P>,
        hook: &SimulationHookAssemblyHandle,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, SimulationHookPoolError> {
        self.check_backend(backend.authority())?;
        let index = self.member_index(hook)?;
        board
            .try_add_cross_attachment_drive(
                backend,
                6,
                &mut self.members[index].candidate.existing_mut()?.session,
                0,
                frames,
                dynamics,
            )
            .map_err(SimulationHookPoolError::Drive)
    }

    /// `Skateboard::ApplyBoardAnimation` targets the separate hook Assembly's
    /// Part 0, not the wheel Part 0 in the skateboard Assembly (0x82b35a40..70).
    pub(crate) fn try_set_board_hook_part_typed(
        &mut self,
        backend: &mut Backend<M>,
        hook: &SimulationHookAssemblyHandle,
        transform: &crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) -> Result<(), SimulationHookPoolError> {
        self.check_backend(backend.authority())?;
        let index = self.member_index(hook)?;
        self.members[index]
            .candidate
            .existing_mut()?
            .session
            .try_set_part_transform_typed(backend, 0, transform)
            .map_err(SimulationHookPoolError::Transform)
    }

    /// Original board setter through owned live Parts; no mutable hook session
    /// escapes. Membership is resolved at the source's final hook write.
    pub(crate) fn try_set_board_transform<P>(
        &mut self,
        backend: &mut Backend<M>,
        board: &mut Session<M, P>,
        hook: &SimulationHookAssemblyHandle,
        requested: [f32; 16],
        services: &mut impl crate::skateboard_spawn::FallibleSkateboardSpawnServices,
    ) -> Result<(), SimulationHookPoolError> {
        crate::skateboard_spawn::try_set_skateboard_transform(
            &mut HookBoardTransform {
                pool: self,
                backend,
                board,
                hook,
            },
            requested,
            services,
            &mut crate::skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
        )
        .map_err(|error| match error {
            crate::skateboard_spawn::SkateboardSpawnError::Cvar(error) => {
                SimulationHookPoolError::SpawnCvar(error)
            }
            crate::skateboard_spawn::SkateboardSpawnError::Parts(error) => error,
        })
    }

    pub(crate) fn try_remove(
        &mut self,
        backend: &mut Backend<M>,
        hook: &SimulationHookAssemblyHandle,
    ) -> Result<SimulationHookAssemblyCandidate<M, C>, SimulationHookPoolError> {
        self.check_backend(backend.authority())?;
        // Host lookup preflight prevents native absent-member/underflow faults.
        // Actual detach still completes before the source swap/count change.
        let index = self.member_index(hook)?;
        self.members[index]
            .candidate
            .existing_mut()?
            .session
            .detach(backend)
            .map_err(SimulationHookPoolError::Detach)?;
        match self.members.swap_remove(index).candidate {
            HookMemberCandidate::Existing(candidate) => Ok(candidate),
            HookMemberCandidate::Skeleton(_) => unreachable!("kind checked before detach"),
        }
    }

    pub(crate) fn try_snapshot(
        &self,
        hook: &SimulationHookAssemblyHandle,
    ) -> Result<SimulationHookPublicationSnapshot, SimulationHookPoolError> {
        let member = &self.members[self.member_index(hook)?];
        Ok(SimulationHookPublicationSnapshot {
            world_index: member.candidate.world_index(),
            group_id: 1,
            assembly_user_data: None,
            parts: member.candidate.publication().to_vec(),
            bounds: member.bounds,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.members.len()
    }

    pub(crate) fn try_exclude_island_labels(
        &self,
        manager: SimulationManagerContactAuthority,
        pool: &SimulationRigidBodyPool,
        labels: &mut SimulationRigidBodyIslandState,
    ) -> Result<(), SimulationHookPoolError> {
        self.try_for_each_member_body(manager, pool, |body| {
            labels.try_exclude_hook_body(pool, body)
        })
    }

    fn try_for_each_member_body(
        &self,
        manager: SimulationManagerContactAuthority,
        pool: &SimulationRigidBodyPool,
        mut visit: impl FnMut(
            SimulationRigidBodyObservation,
        ) -> Result<(), SimulationRigidBodyIslandStateError>,
    ) -> Result<(), SimulationHookPoolError> {
        self.check_authorities(manager, pool.authority())?;
        let count = self.members.len();
        for index in 0..count {
            let candidate = &self.members[index].candidate;
            if candidate.authority() != Some(self.body_pool) {
                return Err(SimulationHookPoolError::ForeignBackend);
            }
            let attachment = candidate.attachment();
            let part_count = attachment.parts.len();
            for ordinal in 0..part_count {
                let body = pool
                    .try_observe(*attachment.attached_body(ordinal))
                    .map_err(|e| {
                        SimulationHookPoolError::Labels(SimulationRigidBodyIslandStateError::Body(
                            e,
                        ))
                    })?;
                visit(body).map_err(SimulationHookPoolError::Labels)?;
            }
        }
        Ok(())
    }

    fn member_index(
        &self,
        handle: &SimulationHookAssemblyHandle,
    ) -> Result<usize, SimulationHookPoolError> {
        self.members
            .iter()
            .position(|member| Arc::ptr_eq(&member.handle.0, &handle.0))
            .ok_or(SimulationHookPoolError::MissingMember)
    }
    fn check_backend(
        &self,
        body_pool: SimulationRigidBodyPoolAuthority,
    ) -> Result<(), SimulationHookPoolError> {
        if body_pool != self.body_pool {
            return Err(SimulationHookPoolError::ForeignBackend);
        }
        Ok(())
    }
    pub(super) fn check_authorities(
        &self,
        manager: SimulationManagerContactAuthority,
        body_pool: SimulationRigidBodyPoolAuthority,
    ) -> Result<(), SimulationHookPoolError> {
        self.check_backend(body_pool)?;
        if manager != self.manager {
            return Err(SimulationHookPoolError::ForeignManager);
        }
        Ok(())
    }
}
