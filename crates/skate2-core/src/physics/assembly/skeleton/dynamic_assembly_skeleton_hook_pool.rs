//! Actual two-Part skeleton hooks in the SAME nonserialized `HookPool`.
//! `InitHooks0x82b50868`, Release0x82b47ce0. Original existing members retain
//! their complete typed session/template; no erased replacement is constructed.
use super::{
    Backend, HookMemberCandidate, Member, Session, SimulationHookAssemblyCandidate,
    SimulationHookAssemblyHandle, SimulationHookAssemblyPool, SimulationHookInsertFailure,
    SimulationHookPoolError, SimulationManagerContactDomain, SkeletonHookDefinition,
};
use crate::simulation_drive_pool::{SimulationDriveDynamics, SimulationDriveFrames};
use std::sync::{Arc, RwLock};

impl<M, C> SimulationHookAssemblyPool<M, C> {
    pub(crate) fn try_insert_skeleton_hook(
        &mut self,
        backend: &mut Backend<M>,
        manager: &mut SimulationManagerContactDomain,
        mut candidate: SimulationHookAssemblyCandidate<M, SkeletonHookDefinition>,
    ) -> Result<
        SimulationHookAssemblyHandle,
        Box<SimulationHookInsertFailure<M, SkeletonHookDefinition>>,
    > {
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
                    candidate: HookMemberCandidate::Skeleton(candidate),
                    bounds: [[0.0; 4]; 2],
                });
                Ok(handle)
            }
            Err(error) => Err(Box::new(SimulationHookInsertFailure { error, candidate })),
        }
    }

    pub(crate) fn skeleton_hook_session(
        &self,
        handle: &SimulationHookAssemblyHandle,
    ) -> Result<&Session<M, SkeletonHookDefinition>, SimulationHookPoolError> {
        match &self.members[self.member_index(handle)?].candidate {
            HookMemberCandidate::Skeleton(candidate) => Ok(&candidate.session),
            HookMemberCandidate::Existing(_) => Err(SimulationHookPoolError::WrongMemberKind),
        }
    }

    fn skeleton_hook_mut(
        &mut self,
        handle: &SimulationHookAssemblyHandle,
    ) -> Result<
        &mut SimulationHookAssemblyCandidate<M, SkeletonHookDefinition>,
        SimulationHookPoolError,
    > {
        let index = self.member_index(handle)?;
        match &mut self.members[index].candidate {
            HookMemberCandidate::Skeleton(candidate) => Ok(candidate),
            HookMemberCandidate::Existing(_) => Err(SimulationHookPoolError::WrongMemberKind),
        }
    }

    /// Parent hook retains the actual Drive and guards the actual child owner.
    /// The two source hook calls store nullable results; they do not write spy1.
    #[expect(
        clippy::too_many_arguments,
        reason = "explicit actual owner endpoints and retained frame/dynamics payloads"
    )]
    pub(crate) fn try_connect_skeleton<P>(
        &mut self,
        backend: &mut Backend<M>,
        hook: &SimulationHookAssemblyHandle,
        parent: usize,
        skeleton: &mut Session<M, P>,
        child: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, SimulationHookPoolError> {
        self.check_backend(backend.authority())?;
        self.skeleton_hook_mut(hook)?
            .session
            .try_add_cross_attachment_drive(backend, parent, skeleton, child, frames, dynamics)
            .map_err(SimulationHookPoolError::Drive)
    }

    pub(crate) fn try_set_skeleton_hook_part(
        &mut self,
        backend: &mut Backend<M>,
        hook: &SimulationHookAssemblyHandle,
        part: usize,
        transform: [f32; 16],
    ) -> Result<(), SimulationHookPoolError> {
        self.check_backend(backend.authority())?;
        self.skeleton_hook_mut(hook)?
            .session
            .try_set_part_transform_host(backend, part, transform)
            .map_err(SimulationHookPoolError::Transform)
    }

    /// Move an actual hook Part while retaining unknown requested cache W.
    pub(crate) fn try_set_skeleton_hook_part_typed(
        &mut self,
        backend: &mut Backend<M>,
        hook: &SimulationHookAssemblyHandle,
        part: usize,
        transform: &crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) -> Result<(), SimulationHookPoolError> {
        self.check_backend(backend.authority())?;
        self.skeleton_hook_mut(hook)?
            .session
            .try_set_part_transform_typed(backend, part, transform)
            .map_err(SimulationHookPoolError::Transform)
    }

    pub(crate) fn try_skeleton_hook_part_transform(
        &self,
        backend: &Backend<M>,
        hook: &SimulationHookAssemblyHandle,
        part: usize,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, SimulationHookPoolError> {
        self.check_backend(backend.authority())?;
        self.skeleton_hook_session(hook)?
            .try_live_part_transform_host(backend, part)
            .map(crate::skeleton_root_transforms::SkeletonRootMatrix::known)
            .map_err(SimulationHookPoolError::Transform)
    }

    /// Release removes hook0 then1 before its Assembly. Null attempts use the
    /// already documented host recovery policy, not native null-removal parity.
    pub(crate) fn try_remove_skeleton_hook_drives(
        &mut self,
        backend: &mut Backend<M>,
        hook: &SimulationHookAssemblyHandle,
    ) -> Result<(), SimulationHookPoolError> {
        self.check_backend(backend.authority())?;
        self.skeleton_hook_mut(hook)?
            .session
            .remove_post_attachment_drives(backend)
            .map_err(SimulationHookPoolError::Drive)
    }

    pub(crate) fn try_remove_skeleton_hook(
        &mut self,
        backend: &mut Backend<M>,
        hook: &SimulationHookAssemblyHandle,
    ) -> Result<SimulationHookAssemblyCandidate<M, SkeletonHookDefinition>, SimulationHookPoolError>
    {
        self.check_backend(backend.authority())?;
        let index = self.member_index(hook)?;
        self.skeleton_hook_mut(hook)?
            .session
            .detach(backend)
            .map_err(SimulationHookPoolError::Detach)?;
        match self.members.swap_remove(index).candidate {
            HookMemberCandidate::Skeleton(candidate) => Ok(candidate),
            HookMemberCandidate::Existing(_) => unreachable!("kind checked before detach"),
        }
    }
}
