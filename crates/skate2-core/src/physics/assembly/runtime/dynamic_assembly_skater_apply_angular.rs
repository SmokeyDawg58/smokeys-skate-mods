//! Unregistered pool child: outer82b37e90 -> Body82b36798 on actual Part6.
use super::target_angular::TargetAngularOwnerError;
use super::{SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool};
use crate::{
    reckoning_numeric::ReckoningNumeric, skateboard_assembly::SkateboardSimulationBackend,
    skeleton_root_transforms::SkeletonRootVector,
};
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_apply_board_angular_displacement<N: ReckoningNumeric>(
        &self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        requested: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), TargetAngularOwnerError<N::Error>> {
        use TargetAngularOwnerError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let board = self.members[index].content.board().map_err(E::Pool)?.0;
        let body = board
            .session
            .try_part_contact_spy_observation(backend, 6)
            .map_err(E::Part)?;
        backend
            .try_apply_skateboard_angular_displacement(body, requested, numeric)
            .map_err(E::Calculate)
    }
}
