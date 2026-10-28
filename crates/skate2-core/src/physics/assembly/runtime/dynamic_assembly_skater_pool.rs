//! Real serialized `AssemblyPool` membership for reconstructed boards/skaters.
//!
//! Insert 0x82e0dbb0, Remove 0x82e0c1c8, nonparallel Begin/EndUpdate
//! 0x82e0dc98/0x82e0bb10. This is not Island BodyID/list ownership, a humanoid
//! producer, parallel scheduling, or flags3's adjacent board/skater pair path.
//! Host capabilities replace pointer identity without exposing native addresses.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "board pool awaits live Island ownership")
)]

use super::{
    DynamicAssemblyAttachError, DynamicAssemblyDetachError, DynamicAssemblyPostDriveError,
    hook_pool::{
        SimulationHookAssemblyHandle, SimulationHookAssemblyPool, SimulationHookPoolError,
    },
    skateboard_publication::{SkateboardCollisionPublication, SkateboardCollisionPublicationError},
    skeleton_collision_publication::{
        SkeletonCollisionPublication, SkeletonCollisionPublicationError,
    },
    skeleton_normal::{SkeletonNormalAssembly, SkeletonNormalSetupError, SkeletonPlacementError},
};
use crate::{
    authored_collision_publication::AuthoredCollisionPublication,
    octree::AabbWords,
    simulation_contact_jacobian::network_preparation::{
        SimulationManagerContactAuthority, SimulationManagerContactDomain,
    },
    simulation_drive_pool::{SimulationDriveDynamics, SimulationDriveFrames},
    simulation_island_map::persistent::SimulationRigidBodyIslandStateError,
    simulation_rigid_body_pool::{
        SimulationRigidBodyFrameIdentity, SimulationRigidBodyPoolAuthority,
    },
    skateboard_assembly::{
        SkateboardAssemblyDriveAdmission, SkateboardAssemblySession, SkateboardSimulationBackend,
    },
    skateboard_attributes::SkateboardAttributeRead,
    skateboard_truck_drives::{
        SkateboardTruckDriveAdmission, SkateboardTruckDriveError, SkateboardTruckDriveNumeric,
        SkateboardTruckDrives,
    },
    skeleton_body_attributes::SkeletonBodyAttributeRead,
};
use skate2_formats::collision::AssemblyPartMaterial;
pub(crate) use skeleton_members::force_physics;
pub(crate) use skeleton_members::wipeout;
use std::{
    fmt,
    sync::{Arc, RwLock},
};

pub(crate) use skeleton_members::hip_hook_soft;
pub(crate) use skeleton_members::ragdoll;

#[path = "dynamic_assembly_skater_board_collision_mode.rs"]
pub(crate) mod board_collision_mode;
#[path = "dynamic_assembly_skater_board_output.rs"]
pub(crate) mod board_output;
#[path = "dynamic_assembly_skater_board_wipeout.rs"]
pub(crate) mod board_wipeout;
#[path = "dynamic_assembly_controller_board_target.rs"]
pub(crate) mod controller_target;
#[path = "dynamic_assembly_skater_local_torque.rs"]
pub(crate) mod local_torque;
#[path = "dynamic_assembly_skater_target_angular.rs"]
pub(crate) mod target_angular;

#[derive(Clone, Debug)]
pub(crate) struct SimulationSkaterAssemblyHandle(Arc<()>);

impl SimulationSkaterAssemblyHandle {
    #[cfg(test)]
    pub(crate) fn landing_manager_test_handle() -> Self {
        Self(Arc::new(()))
    }

    pub(crate) fn same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(test)]
#[path = "dynamic_assembly_skater_pool_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "dynamic_assembly_skater_pool_mixed_tests.rs"]
pub(crate) mod mixed_tests;

/// Owns the actual board through preflight and partially completed admission.
/// Its index is Assembly Initialize's ffff, not a manager allocation, until Insert.
pub(crate) struct SimulationSkaterAssemblyCandidate {
    session: SkateboardAssemblySession,
    world_index: u32,
    requested_state: Option<u32>,
    construction: Option<Box<crate::skateboard_body_construction::SkateboardBodyConstruction>>,
    force_runtime: Option<Box<crate::skateboard_force_runtime::SkateboardForceRuntime>>,
    truck_drives: Option<SkateboardTruckDrives>,
    truck_drive_occurrence: Option<Arc<()>>,
    /// Same SkateboardBody hook Assembly admitted beside this board.
    hook: Option<SimulationHookAssemblyHandle>,
    /// PC Object Dropper board with no skater: see [`riderless`].
    riderless: bool,
}

impl SimulationSkaterAssemblyCandidate {
    pub(crate) fn try_new(
        session: SkateboardAssemblySession,
    ) -> Result<Self, Box<SimulationSkaterCandidateFailure>> {
        if session.is_attached() {
            return Err(Box::new(SimulationSkaterCandidateFailure {
                error: SimulationSkaterPoolError::AlreadyAttached,
                session,
            }));
        }
        Ok(Self {
            session,
            world_index: 0xffff,
            requested_state: None,
            construction: None,
            force_runtime: None,
            truck_drives: None,
            truck_drive_occurrence: None,
            hook: None,
            riderless: false,
        })
    }

    pub(crate) const fn world_index(&self) -> u32 {
        self.world_index
    }
    pub(crate) fn requested_state(&self) -> Option<u32> {
        self.requested_state
    }
    pub(crate) fn session(&self) -> &SkateboardAssemblySession {
        &self.session
    }

    /// Only legacy session-only candidates may discard their wrapper. A real
    /// constructor's reader/cache must survive removal together with the session.
    pub(crate) fn into_session(self) -> Result<SkateboardAssemblySession, Box<Self>> {
        if self.construction.is_some()
            || self.force_runtime.is_some()
            || self.truck_drives.is_some()
            || self.hook.is_some()
        {
            return Err(Box::new(self));
        }
        Ok(self.session)
    }

    pub(crate) fn retain_force_runtime(
        &mut self,
        runtime: Box<crate::skateboard_force_runtime::SkateboardForceRuntime>,
    ) {
        assert!(
            self.force_runtime.is_none(),
            "Skateboard force owner already retained"
        );
        self.force_runtime = Some(runtime);
    }
}

#[path = "dynamic_assembly_skater_board_construction.rs"]
pub(crate) mod board_construction;

#[path = "dynamic_assembly_skater_board_entry.rs"]
pub(crate) mod board_entry;

#[path = "dynamic_assembly_skater_postphysics.rs"]
pub(crate) mod postphysics;

#[path = "dynamic_assembly_skater_forces.rs"]
pub(crate) mod forces;

#[path = "dynamic_assembly_skater_physical.rs"]
pub(crate) mod physical;

#[path = "dynamic_assembly_skater_riderless.rs"]
pub(crate) mod riderless;

pub(crate) struct SimulationSkaterCandidateFailure {
    pub(crate) error: SimulationSkaterPoolError,
    pub(crate) session: SkateboardAssemblySession,
}
impl fmt::Debug for SimulationSkaterCandidateFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationSkaterCandidateFailure")
            .field("error", &self.error)
            .field("attached", &self.session.is_attached())
            .finish_non_exhaustive()
    }
}

pub(crate) struct SimulationSkaterInsertFailure {
    pub(crate) error: SimulationSkaterPoolError,
    pub(crate) candidate: SimulationSkaterAssemblyCandidate,
}
impl fmt::Debug for SimulationSkaterInsertFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationSkaterInsertFailure")
            .field("error", &self.error)
            .field("world_index", &self.candidate.world_index)
            .field("attached", &self.candidate.session.is_attached())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) enum SimulationSkaterPoolError {
    ControllerPoisoned,
    ControllerDrivesRemain { slots: [bool; 2] },
    Allocation,
    Uninitialized,
    AlreadyInitialized,
    WorldFlagsMismatch { expected: u32, actual: u32 },
    UnsupportedParallel,
    Full { capacity: usize },
    ForeignBackend,
    ForeignManager,
    AlreadyAttached,
    MissingMember,
    WrongMemberKind,
    MissingPhysicalConstruction,
    SkeletonIkUnconstructed,
    SkeletonConstructorIncomplete,
    PhysicalPart(super::part_contact_spy::DynamicAssemblyPartContactSpyError),
    Attach(DynamicAssemblyAttachError),
    Detach(DynamicAssemblyDetachError),
    Drive(DynamicAssemblyPostDriveError),
    Labels(SimulationRigidBodyIslandStateError),
    Publication(SkateboardCollisionPublicationError),
    SkeletonPublication(SkeletonCollisionPublicationError),
    SkeletonPlacement(SkeletonPlacementError),
    SkeletonDrive(super::skeleton_normal::drive_owner::SkeletonDriveOwnerError),
    Hook(SimulationHookPoolError),
    /// PC Object Dropper riderless board wake (a port addition).
    RiderlessWake(String),
}
impl fmt::Display for SimulationSkaterPoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControllerPoisoned => f.write_str("controller ownership lock poisoned"),
            Self::ControllerDrivesRemain { slots } => write!(
                f,
                "controller drives must be removed before bodies: {slots:?}"
            ),
            Self::Allocation => f.write_str("SkaterPool host allocation failed"),
            Self::Uninitialized => f.write_str("SkaterPool is not initialized"),
            Self::RiderlessWake(e) => write!(f, "riderless board wake: {e}"),
            Self::AlreadyInitialized => f.write_str("SkaterPool is already initialized"),
            Self::WorldFlagsMismatch { expected, actual } => write!(
                f,
                "SkaterPool world flags {actual:#x} differ from configured {expected:#x}"
            ),
            Self::UnsupportedParallel => {
                f.write_str("SkaterPool parallel update is not implemented")
            }
            Self::Full { capacity } => write!(f, "SkaterPool capacity {capacity} reached"),
            Self::ForeignBackend => f.write_str("SkaterPool backend authority mismatch"),
            Self::ForeignManager => f.write_str("SkaterPool manager authority mismatch"),
            Self::AlreadyAttached => f.write_str("SkaterPool candidate is already attached"),
            Self::MissingMember => f.write_str("SkaterPool member is foreign or removed"),
            Self::WrongMemberKind => {
                f.write_str("SkaterPool operation requires a different member kind")
            }
            Self::MissingPhysicalConstruction => {
                f.write_str("SkaterPool member has no retained physical-player construction")
            }
            Self::SkeletonIkUnconstructed => {
                f.write_str("Skeleton Init requires its constructor-owned IK")
            }
            Self::SkeletonConstructorIncomplete => {
                f.write_str("Skeleton constructor ownership or Init stages are incomplete")
            }
            Self::PhysicalPart(error) => write!(f, "SkaterPool physical Part: {error}"),
            Self::Attach(error) => write!(f, "SkaterPool admission: {error}"),
            Self::Detach(error) => write!(f, "SkaterPool detach: {error}"),
            Self::Drive(error) => write!(f, "SkaterPool drive: {error}"),
            Self::Labels(error) => write!(f, "SkaterPool manager: {error:?}"),
            Self::Publication(error) => write!(f, "SkaterPool publication: {error}"),
            Self::SkeletonPublication(error) => {
                write!(f, "SkaterPool skeleton publication: {error}")
            }
            Self::SkeletonPlacement(error) => write!(f, "SkaterPool skeleton placement: {error}"),
            Self::SkeletonDrive(error) => write!(f, "SkaterPool skeleton drive: {error}"),
            Self::Hook(error) => write!(f, "SkaterPool hook: {error}"),
        }
    }
}

struct Member {
    handle: SimulationSkaterAssemblyHandle,
    content: MemberContent,
    bounds: AabbWords,
}

// Owner and publication cannot disagree: each variant owns the complete pair.
enum MemberContent {
    Board {
        candidate: SimulationSkaterAssemblyCandidate,
        publication: SkateboardCollisionPublication,
    },
    Skeleton {
        candidate: SimulationSkeletonAssemblyCandidate,
        publication: SkeletonCollisionPublication,
    },
}

#[path = "dynamic_assembly_skater_pool_skeleton.rs"]
mod skeleton_members;
pub(crate) use skeleton_members::SimulationSkeletonOffboardTrajectoryResetError;
pub(crate) use skeleton_members::SimulationSkeletonPostRecordsError;
pub(crate) use skeleton_members::SimulationSkeletonTeleportResetError;
pub(crate) use skeleton_members::SkeletonLandingWobbleError;
pub(crate) use skeleton_members::current_animation::SimulationSkeletonAnimationError;
pub(crate) use skeleton_members::grounded::SimulationSkeletonGroundedError;
pub(crate) use skeleton_members::grounded::SimulationSkeletonOffboardGeneralError;
pub(crate) use skeleton_members::ik_construction;
pub(crate) use skeleton_members::jump::SimulationSkeletonJumpPairError;
pub(crate) use skeleton_members::process_data::ProcessDataPrefixError;
pub(crate) use skeleton_members::roots::SimulationSkeletonRootUpdateError;
pub(crate) use skeleton_members::roots::{LiveBoard, SkeletonRootBoardAccessError};
pub(crate) use skeleton_members::{
    SimulationSkeletonAssemblyCandidate, SimulationSkeletonCollisionError,
    SimulationSkeletonHistoryUpdateError, SimulationSkeletonInsertFailure,
    SimulationSkeletonMutationError,
};

impl MemberContent {
    fn board(
        &self,
    ) -> Result<
        (
            &SimulationSkaterAssemblyCandidate,
            &SkateboardCollisionPublication,
        ),
        SimulationSkaterPoolError,
    > {
        match self {
            Self::Board {
                candidate,
                publication,
            } => Ok((candidate, publication)),
            Self::Skeleton { .. } => Err(SimulationSkaterPoolError::WrongMemberKind),
        }
    }

    fn board_mut(
        &mut self,
    ) -> Result<&mut SimulationSkaterAssemblyCandidate, SimulationSkaterPoolError> {
        match self {
            Self::Board { candidate, .. } => Ok(candidate),
            Self::Skeleton { .. } => Err(SimulationSkaterPoolError::WrongMemberKind),
        }
    }

    fn publication(&self) -> AuthoredCollisionPublication<'_> {
        match self {
            Self::Board { publication, .. } => AuthoredCollisionPublication::Board(publication),
            Self::Skeleton { publication, .. } => {
                AuthoredCollisionPublication::Skeleton(publication)
            }
        }
    }

    fn world_index(&self) -> u32 {
        match self {
            Self::Board { candidate, .. } => candidate.world_index,
            Self::Skeleton { candidate, .. } => candidate.world_index(),
        }
    }

    fn invalidate_refresh(&mut self) {
        match self {
            Self::Board { publication, .. } => publication.invalidate_refresh(),
            Self::Skeleton { publication, .. } => publication.invalidate_refresh(),
        }
    }

    fn update(
        &mut self,
        backend: &SkateboardSimulationBackend,
    ) -> Result<(), SimulationSkaterPoolError> {
        match self {
            Self::Board {
                candidate,
                publication,
            } => publication
                .try_update_volumes(&candidate.session, backend)
                .map_err(SimulationSkaterPoolError::Publication),
            Self::Skeleton {
                candidate,
                publication,
            } => publication
                .try_update_volumes(candidate.owner(), backend)
                .map_err(SimulationSkaterPoolError::SkeletonPublication),
        }
    }

    fn stage_bounds(
        &self,
        backend: &SkateboardSimulationBackend,
    ) -> Result<AabbWords, SimulationSkaterPoolError> {
        match self {
            Self::Board {
                candidate,
                publication,
            } => publication
                .try_stage_bounds(&candidate.session, backend)
                .map_err(SimulationSkaterPoolError::Publication),
            Self::Skeleton {
                candidate,
                publication,
            } => publication
                .try_stage_bounds(candidate.owner(), backend)
                .map_err(SimulationSkaterPoolError::SkeletonPublication),
        }
    }

    fn publish_bounds(&mut self, bounds: AabbWords, frame: SimulationRigidBodyFrameIdentity) {
        match self {
            Self::Board { publication, .. } => publication.publish_bounds(bounds, frame),
            Self::Skeleton { publication, .. } => publication.publish_bounds(bounds, frame),
        }
    }

    fn all_frozen(
        &self,
        backend: &SkateboardSimulationBackend,
    ) -> Result<bool, SimulationSkaterPoolError> {
        match self {
            Self::Board {
                candidate,
                publication,
            } => publication
                .try_all_parts_frozen(&candidate.session, backend)
                .map_err(SimulationSkaterPoolError::Publication),
            Self::Skeleton {
                candidate,
                publication,
            } => publication
                .try_all_parts_frozen(candidate.owner(), backend)
                .map_err(SimulationSkaterPoolError::SkeletonPublication),
        }
    }
}

pub(crate) struct SimulationSkaterAssemblyPool {
    manager: SimulationManagerContactAuthority,
    body_pool: SimulationRigidBodyPoolAuthority,
    capacity: usize,
    members: Vec<Member>,
    active_ordinals: Vec<usize>,
    active_frame: Option<SimulationRigidBodyFrameIdentity>,
}

struct TruckAdmission<'a>(SkateboardAssemblyDriveAdmission<'a>);
impl SkateboardTruckDriveAdmission for TruckAdmission<'_> {
    type Error = SimulationSkaterPoolError;
    fn try_add_drive(
        &mut self,
        parent: usize,
        child: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, Self::Error> {
        self.0
            .try_add_drive(parent, child, frames, dynamics)
            .map_err(SimulationSkaterPoolError::Drive)
    }
}

impl SimulationSkaterAssemblyPool {
    /// `InitWorld` 0x827b4370/4380 and 0x827b4354/435c. The caller supplies the
    /// actual sampled parallel setting; true is refused, never coerced to false.
    pub(crate) fn try_new(
        manager: &SimulationManagerContactDomain,
        backend: &SkateboardSimulationBackend,
        island_flags: u32,
        current_update_in_parallel: bool,
    ) -> Result<Self, SimulationSkaterPoolError> {
        if manager.assembly_body_pool_authority() != backend.authority() {
            return Err(SimulationSkaterPoolError::ForeignBackend);
        }
        if current_update_in_parallel {
            return Err(SimulationSkaterPoolError::UnsupportedParallel);
        }
        let capacity = if island_flags & 1 != 0 { 16 } else { 2 };
        let mut members = Vec::new();
        members
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationSkaterPoolError::Allocation)?;
        let mut active_ordinals = Vec::new();
        active_ordinals
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationSkaterPoolError::Allocation)?;
        Ok(Self {
            manager: manager.assembly_pool_authority(),
            body_pool: backend.authority(),
            capacity,
            members,
            active_ordinals,
            active_frame: None,
        })
    }

    /// Source Insert issues the shared ID BEFORE Add, and publishes a raw member
    /// pointer BEFORE Create. Host ownership is instead staged until Create
    /// succeeds: failure returns the actual candidate, possibly attached. There
    /// is no implicit detach, ID refund, or claimed native failure transaction.
    pub(crate) fn try_insert(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        manager: &mut SimulationManagerContactDomain,
        mut candidate: SimulationSkaterAssemblyCandidate,
        state: u32,
        bend: f32,
    ) -> Result<SimulationSkaterAssemblyHandle, Box<SimulationSkaterInsertFailure>> {
        let result = (|| {
            self.check_backend(backend)?;
            if manager.assembly_pool_authority() != self.manager {
                return Err(SimulationSkaterPoolError::ForeignManager);
            }
            if candidate
                .construction()
                .and_then(|c| c.physical_owner())
                .is_some_and(|owner| owner.manager() != self.manager)
            {
                return Err(SimulationSkaterPoolError::ForeignManager);
            }
            if candidate.session.is_attached() {
                return Err(SimulationSkaterPoolError::AlreadyAttached);
            }
            if self.members.len() == self.capacity {
                return Err(SimulationSkaterPoolError::Full {
                    capacity: self.capacity,
                });
            }
            let handle = SimulationSkaterAssemblyHandle(Arc::new(()));
            candidate.world_index = manager
                .try_allocate_assembly_world_index(self.body_pool)
                .map_err(SimulationSkaterPoolError::Labels)?;
            candidate.requested_state = Some(state);
            candidate
                .session
                .attach(backend, state)
                .map_err(SimulationSkaterPoolError::Attach)?;
            let publication =
                SkateboardCollisionPublication::try_create(&candidate.session, backend, bend)
                    .map_err(SimulationSkaterPoolError::Publication)?;
            Ok((handle, publication))
        })();
        match result {
            Ok((handle, publication)) => {
                let bounds = publication.bounds();
                self.members.push(Member {
                    handle: handle.clone(),
                    content: MemberContent::Board {
                        candidate,
                        publication,
                    },
                    bounds,
                });
                // Native does not immediately reconstruct its active pointers.
                self.active_frame = None;
                Ok(handle)
            }
            Err(error) => Err(Box::new(SimulationSkaterInsertFailure { error, candidate })),
        }
    }

    pub(crate) fn session(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&SkateboardAssemblySession, SimulationSkaterPoolError> {
        Ok(&self.members[self.member_index(handle)?]
            .content
            .board()?
            .0
            .session)
    }

    pub(crate) fn publication(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&SkateboardCollisionPublication, SimulationSkaterPoolError> {
        Ok(self.members[self.member_index(handle)?].content.board()?.1)
    }

    pub(crate) fn world_index(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<u32, SimulationSkaterPoolError> {
        Ok(self.members[self.member_index(handle)?]
            .content
            .world_index())
    }

    /// Full dense membership, NOT the active-list subset. Ground Run 0x827cab00.
    pub(crate) fn publications(
        &self,
    ) -> impl ExactSizeIterator<Item = AuthoredCollisionPublication<'_>> {
        self.members
            .iter()
            .map(|member| member.content.publication())
    }

    pub(crate) fn handles(&self) -> impl ExactSizeIterator<Item = &SimulationSkaterAssemblyHandle> {
        self.members.iter().map(|member| &member.handle)
    }

    pub(crate) fn len(&self) -> usize {
        self.members.len()
    }
    pub(crate) fn board_count(&self) -> usize {
        self.members
            .iter()
            .filter(|member| matches!(member.content, MemberContent::Board { .. }))
            .count()
    }
    pub(crate) const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Diagnostics for the last completed `EndUpdate`. The returned frame must
    /// match the body pool's current frame before treating these as current.
    pub(crate) fn active_ordinals(&self) -> Option<&[usize]> {
        self.active_frame.map(|_| self.active_ordinals.as_slice())
    }
    pub(crate) const fn active_frame(&self) -> Option<SimulationRigidBodyFrameIdentity> {
        self.active_frame
    }

    pub(crate) fn try_initialize_truck_drives<
        A: SkateboardAttributeRead,
        N: SkateboardTruckDriveNumeric,
    >(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        drives: &mut SkateboardTruckDrives,
        attributes: &mut A,
        numeric: &mut N,
    ) -> Result<(), SkateboardTruckDriveError<A::Error, N::Error, SimulationSkaterPoolError>> {
        self.check_backend(backend)
            .map_err(SkateboardTruckDriveError::Admission)?;
        let index = self
            .member_index(handle)
            .map_err(SkateboardTruckDriveError::Admission)?;
        drives.try_create_fresh(
            attributes,
            numeric,
            &mut TruckAdmission(SkateboardAssemblyDriveAdmission {
                session: &mut self.members[index]
                    .content
                    .board_mut()
                    .map_err(SkateboardTruckDriveError::Admission)?
                    .session,
                backend,
            }),
        )
    }

    pub(crate) fn try_connect_hook<C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, C>,
        hook: &SimulationHookAssemblyHandle,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(SimulationSkaterPoolError::Hook)?;
        let index = self.member_index(handle)?;
        let connected = hooks
            .try_connect_board(
                backend,
                &mut self.members[index].content.board_mut()?.session,
                hook,
                frames,
                dynamics,
            )
            .map_err(SimulationSkaterPoolError::Hook)?;
        self.members[index].content.board_mut()?.hook = Some(hook.clone());
        Ok(connected)
    }

    pub(crate) fn try_set_board_transform<C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, C>,
        hook: &SimulationHookAssemblyHandle,
        requested: [f32; 16],
        services: &mut impl crate::skateboard_spawn::FallibleSkateboardSpawnServices,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(SimulationSkaterPoolError::Hook)?;
        let index = self.member_index(handle)?;
        // A checked error can follow real partial setter writes, so invalidate
        // BEFORE entering it, including same-frame failures.
        self.members[index].content.board()?;
        self.invalidate_refresh();
        hooks
            .try_set_board_transform(
                backend,
                &mut self.members[index].content.board_mut()?.session,
                hook,
                requested,
                services,
            )
            .map_err(SimulationSkaterPoolError::Hook)
    }

    /// Reuse the hook already attached to this same admitted board.
    pub(crate) fn try_registered_board_hook(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<Option<SimulationHookAssemblyHandle>, SimulationSkaterPoolError> {
        let index = self.member_index(handle)?;
        Ok(self.members[index].content.board()?.0.hook.clone())
    }

    pub(crate) fn try_remove_post_drives(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        let index = self.member_index(handle)?;
        self.members[index]
            .content
            .board_mut()?
            .session
            .remove_post_attachment_drives(backend)
            .map_err(SimulationSkaterPoolError::Drive)
    }

    /// `RemoveFromSimulation` precedes swap-last of the whole member/publication/
    /// bounds owner. Existing post-drive and incoming-drive guards still apply.
    pub(crate) fn try_remove(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<SimulationSkaterAssemblyCandidate, SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        let index = self.member_index(handle)?;
        self.members[index]
            .content
            .board_mut()?
            .session
            .detach(backend)
            .map_err(SimulationSkaterPoolError::Detach)?;
        self.active_frame = None;
        // Swap-last would move the last member between an adjacent
        // board/Skeleton pair; a riderless board leaves the order intact.
        let riderless = matches!(&self.members[index].content,
            MemberContent::Board { candidate, .. } if candidate.riderless);
        let member = if riderless {
            self.members.remove(index)
        } else {
            self.members.swap_remove(index)
        };
        match member.content {
            MemberContent::Board { candidate, .. } => Ok(candidate),
            MemberContent::Skeleton { .. } => unreachable!("board kind checked before detach"),
        }
    }

    /// Only the source nonparallel order: all Update -> all bounds -> all
    /// Header publication -> fresh live-state active-list scan. No active-list
    /// filtering or per-member complete-refresh loop is introduced.
    pub(crate) fn try_refresh(
        &mut self,
        backend: &SkateboardSimulationBackend,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.invalidate_refresh();
        let result = (|| {
            self.check_backend(backend)?;
            let frame = backend.collision_body_pool().frame_identity().ok_or(
                SimulationSkaterPoolError::Publication(
                    SkateboardCollisionPublicationError::FrameUnavailable,
                ),
            )?;
            for member in &mut self.members {
                member.content.update(backend)?;
            }
            for member in &mut self.members {
                member.bounds = member.content.stage_bounds(backend)?;
            }
            for member in &mut self.members {
                member.content.publish_bounds(member.bounds, frame);
            }
            self.active_ordinals.clear();
            for (ordinal, member) in self.members.iter().enumerate() {
                if !member.content.all_frozen(backend)? {
                    self.active_ordinals.push(ordinal);
                }
            }
            self.active_frame = Some(frame);
            Ok(())
        })();
        if result.is_err() {
            self.invalidate_refresh();
        }
        result
    }

    pub(crate) fn invalidate_refresh(&mut self) {
        self.active_frame = None;
        for member in &mut self.members {
            member.content.invalidate_refresh();
        }
    }

    fn member_index(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<usize, SimulationSkaterPoolError> {
        self.members
            .iter()
            .position(|member| Arc::ptr_eq(&member.handle.0, &handle.0))
            .ok_or(SimulationSkaterPoolError::MissingMember)
    }

    fn check_backend(
        &self,
        backend: &SkateboardSimulationBackend,
    ) -> Result<(), SimulationSkaterPoolError> {
        if backend.authority() != self.body_pool {
            return Err(SimulationSkaterPoolError::ForeignBackend);
        }
        Ok(())
    }
}

#[path = "../../player/skeleton/physical_player_skeleton_fill_prefix.rs"]
pub(crate) mod skeleton_fill_prefix;

#[path = "../../player/skeleton/physical_player_skeleton_part_fill.rs"]
pub(crate) mod skeleton_part_fill;

pub(crate) use skeleton_members::pose_publication;
pub(crate) use skeleton_members::post_wipeout_live;
pub(crate) use skeleton_members::postphysics_live;

#[path = "dynamic_assembly_skater_apply_angular.rs"]
pub(crate) mod apply_angular;

pub(crate) use skeleton_members::skeleton_fill_suffix;
