//! Actual `Skateboard::TargetAngularDisplacement82b37e98` owner route.
//! Unregistered skater-pool child; no new body, transform or inertia is made.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
};
use crate::{
    dynamic_assembly_session::part_contact_spy::DynamicAssemblyPartContactSpyError,
    reckoning_numeric::ReckoningNumeric,
    simulation_rigid_body_pool::target_angular::TargetAngularError,
    skateboard_assembly::SkateboardSimulationBackend, skeleton_root_transforms::SkeletonRootVector,
};
#[derive(Debug)]
pub(crate) enum TargetAngularOwnerError<N> {
    Pool(SimulationSkaterPoolError),
    Part(DynamicAssemblyPartContactSpyError),
    Calculate(TargetAngularError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for TargetAngularOwnerError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => e.fmt(f),
            Self::Part(e) => e.fmt(f),
            Self::Calculate(e) => e.fmt(f),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_target_board_angular_displacement_exact<N: ReckoningNumeric>(
        &self,
        backend: &mut SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        displacement: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), TargetAngularOwnerError<N::Error>> {
        use TargetAngularOwnerError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let board = self.members[index].content.board().map_err(E::Pool)?.0;
        let body = board
            .session
            .try_part_contact_spy_observation(backend, 6)
            .map_err(E::Part)?;
        backend
            .try_target_skateboard_angular_displacement_exact(body, displacement, numeric)
            .map_err(E::Calculate)
    }
    pub(crate) fn try_target_board_angular_displacement<N: ReckoningNumeric>(
        &self,
        backend: &mut SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        displacement: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), TargetAngularOwnerError<N::Error>> {
        use TargetAngularOwnerError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let board = self.members[index].content.board().map_err(E::Pool)?.0;
        let body = board
            .session
            .try_part_contact_spy_observation(backend, 6)
            .map_err(E::Part)?;
        backend
            .try_target_skateboard_angular_displacement(body, displacement, numeric)
            .map_err(E::Calculate)
    }
}
