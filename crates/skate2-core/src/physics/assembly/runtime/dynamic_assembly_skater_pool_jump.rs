//! Checked same-pair access for selected animation jump/contact records.
//! Adjustment writes do not alter bodies, volume publication or contact frames.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::SkeletonNormalAssembly,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
};
use std::{fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum SimulationSkeletonJumpPairError {
    Pool(SimulationSkaterPoolError),
    DifferentPhysicalOwner,
    Detached,
}
impl fmt::Display for SimulationSkeletonJumpPairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "jump pair: {e}"),
            Self::DifferentPhysicalOwner => {
                f.write_str("jump board and Skeleton belong to different physical players")
            }
            Self::Detached => f.write_str("jump pair needs attached actual bodies"),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    /// Same-pair collision writes require publication invalidation. First
    /// validate foreign/detached inputs before touching any freshness state.
    pub(crate) fn try_with_collision_pair<T>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        operation: impl FnOnce(&mut SkeletonNormalAssembly, &Arc<PhysicalPlayerAssemblyOwner>) -> T,
    ) -> Result<T, SimulationSkeletonJumpPairError> {
        self.try_with_jump_pair(backend, board, skeleton, |_, _| ())?;
        self.invalidate_refresh();
        self.try_with_jump_pair(backend, board, skeleton, operation)
    }
    /// Source Part body pointer read for controller cross-assembly drives.
    /// Admission performs body liveness checks at its later native stage.
    pub(crate) fn try_controller_part_body(
        &self,
        backend: &SkateboardSimulationBackend,
        assembly: &SimulationSkaterAssemblyHandle,
        part: usize,
    ) -> Result<
        crate::simulation_rigid_body_pool::SimulationRigidBodyHandle,
        SimulationSkaterPoolError,
    > {
        self.check_backend(backend)?;
        let index = self.member_index(assembly)?;
        match &self.members[index].content {
            MemberContent::Board { candidate, .. } => {
                candidate.session.part_contact_spy_handle(backend, part)
            }
            MemberContent::Skeleton { candidate, .. } => candidate
                .owner
                .session()
                .part_contact_spy_handle(backend, part),
        }
        .map_err(SimulationSkaterPoolError::PhysicalPart)
    }
    pub(crate) fn try_with_jump_pair<T>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        operation: impl FnOnce(&mut SkeletonNormalAssembly, &Arc<PhysicalPlayerAssemblyOwner>) -> T,
    ) -> Result<T, SimulationSkeletonJumpPairError> {
        use crate::dynamic_assembly_session::skater_pool::physical::SimulationPhysicalBodyKind;
        use SimulationSkeletonJumpPairError as E;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let (kind, board_owner) = self
            .try_physical_member_owner(backend, board)
            .map_err(E::Pool)?;
        if kind != SimulationPhysicalBodyKind::Board {
            return Err(E::Pool(SimulationSkaterPoolError::WrongMemberKind));
        }
        let (_, skeleton_owner) = self
            .try_physical_member_owner(backend, skeleton)
            .map_err(E::Pool)?;
        if !Arc::ptr_eq(board_owner, skeleton_owner) {
            return Err(E::DifferentPhysicalOwner);
        }
        if board_owner.manager() != self.manager {
            return Err(E::Pool(SimulationSkaterPoolError::ForeignManager));
        }
        let owner = Arc::clone(board_owner);
        if !self.session(board).map_err(E::Pool)?.is_attached() {
            return Err(E::Detached);
        }
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("validated Skeleton kind")
        };
        if !candidate.owner.session().is_attached() {
            return Err(E::Detached);
        }
        Ok(operation(&mut candidate.owner, &owner))
    }
}
