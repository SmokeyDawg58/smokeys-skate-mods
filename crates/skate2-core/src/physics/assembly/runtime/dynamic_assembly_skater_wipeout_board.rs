//! Source board turn/drive effects82d8ebdc/ebf4/ec0c on existing owners.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SkateboardEntrySessionError,
};
use crate::{
    dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError,
    skateboard_assembly::SkateboardSimulationBackend,
};
#[derive(Debug)]
pub(crate) enum WipeoutBoardControlError {
    Entry(SkateboardEntrySessionError),
    Truck(SimulationConstructedBoardError),
    Poisoned,
}
impl std::fmt::Display for WipeoutBoardControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Entry(e) => write!(f, "wipeout hook: {e}"),
            Self::Truck(e) => write!(f, "wipeout turn: {e:?}"),
            Self::Poisoned => f.write_str("wipeout hook dynamics lock poisoned"),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_wipeout_board_turn(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), WipeoutBoardControlError> {
        self.try_with_constructed_truck_drives(backend, board, |drives, _| {
            drives.set_wipeout_force_turn_angle(0.0);
        })
        .map_err(WipeoutBoardControlError::Truck)
    }
    pub(crate) fn try_wipeout_disable_board_hook(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        angular: bool,
    ) -> Result<(), WipeoutBoardControlError> {
        use WipeoutBoardControlError as E;
        self.check_backend(backend)
            .map_err(|e| E::Entry(SkateboardEntrySessionError::Pool(e)))?;
        let index = self
            .member_index(board)
            .map_err(|e| E::Entry(SkateboardEntrySessionError::Pool(e)))?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(|e| E::Entry(SkateboardEntrySessionError::Pool(e)))?;
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(E::Entry(SkateboardEntrySessionError::MissingConstruction))?;
        let hook = runtime
            .hook_dynamics
            .as_ref()
            .ok_or(E::Entry(SkateboardEntrySessionError::MissingHook))?;
        let mut dynamics = hook.write().map_err(|_| E::Poisoned)?;
        let parameters = if angular {
            &mut dynamics.angular
        } else {
            &mut dynamics.linear
        };
        parameters.raw_type = 2;
        parameters.spring = 0.0;
        parameters.damping = 0.0;
        parameters.strength = 0.0;
        Ok(())
    }
}
