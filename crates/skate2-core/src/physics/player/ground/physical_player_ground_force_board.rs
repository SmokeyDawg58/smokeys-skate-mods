//! Unregistered board-construction child: one borrow of the actual force owners.
use super::{
    SimulationConstructedBoardError, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SkateboardSimulationBackend,
};
use crate::{
    skateboard_assembly::SkateboardAssemblySession, skateboard_attributes::SkateboardAttributes,
    skateboard_force_runtime::SkateboardForceRuntime,
};

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_with_ground_force_board<T>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        operation: impl FnOnce(
            &SkateboardAssemblySession,
            &mut SkateboardForceRuntime,
            &mut SkateboardAttributes,
        ) -> T,
    ) -> Result<T, SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let construction = candidate
            .construction
            .as_mut()
            .ok_or(E::MissingConstruction)?;
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(E::MissingConstruction)?;
        Ok(operation(
            &candidate.session,
            runtime,
            construction.attributes_mut(),
        ))
    }
}
