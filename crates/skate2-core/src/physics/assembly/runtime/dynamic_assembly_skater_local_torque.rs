//! Actual cached deck occurrence for 82b35728; unregistered skater-pool child.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
};
use crate::{
    dynamic_assembly_session::skater_pool::forces::SkateboardQueuedBodyError,
    skateboard_assembly::SkateboardSimulationBackend, skeleton_root_transforms::SkeletonRootVector,
};
#[derive(Debug)]
pub(crate) enum LocalTorqueError {
    Pool(SimulationSkaterPoolError),
    Body(SkateboardQueuedBodyError),
}
impl std::fmt::Display for LocalTorqueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => e.fmt(f),
            Self::Body(e) => e.fmt(f),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_apply_board_local_torque(
        &self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        torque: SkeletonRootVector,
    ) -> Result<(), LocalTorqueError> {
        self.check_backend(backend)
            .map_err(LocalTorqueError::Pool)?;
        let index = self.member_index(handle).map_err(LocalTorqueError::Pool)?;
        let board = self.members[index]
            .content
            .board()
            .map_err(LocalTorqueError::Pool)?
            .0;
        let body = board
            .session
            .try_part_contact_spy_observation(backend, 6)
            .map_err(|e| LocalTorqueError::Body(SkateboardQueuedBodyError::Part(e)))?;
        // Proposed narrow backend bridge delegates to the actual rigid-body
        // pool child; it must not convert this to a force-queue entry.
        backend
            .try_apply_skateboard_local_torque(body, torque)
            .map_err(|e| LocalTorqueError::Body(SkateboardQueuedBodyError::Body(e)))
    }
}
