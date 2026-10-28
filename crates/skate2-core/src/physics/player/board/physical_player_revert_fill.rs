//! RevertGround.FillPhysOut82d86c60, after the common state publication.
use super::{PhysOutComponent, PhysicalPlayerPhysOut};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    simulation_assembly_backend::constraint_session::revert_ground::{
        RevertError, error, with_state,
    },
};
use std::sync::Arc;
impl PhysicalPlayerPhysOut {
    fn fill_revert(&mut self, value: bool) -> Result<(), RevertError> {
        self.check_current_guard().map_err(error)?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(error)?;
        self.concrete
            .state
            .as_mut()
            .expect("checked component")
            .is_reverting = value;
        Ok(())
    }
}
pub(crate) fn fill(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Result<(), RevertError> {
    let value = with_state(owner, board, skeleton, |s| {
        s.is_reverting.ok_or_else(|| error("revert flag"))
    })?;
    owner.phys_out().lock().map_err(error)?.fill_revert(value)
}
