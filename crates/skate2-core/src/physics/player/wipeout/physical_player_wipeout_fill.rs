//! Same-owner WipeoutGround.FillPhysOut82d84e18 at Update5's existing state slot.
use super::prior_input::PriorInputReadError;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::{PhysOutComponent, PhysicalPlayerPhysOut},
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_wipeout_state::WipeoutGroundFields,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum WipeoutFillError {
    Poisoned,
    WrongState(CurrentPhysicalState),
    MissingState,
    ForeignOwner,
    Output(PriorInputReadError),
}
impl std::fmt::Display for WipeoutFillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Wipeout.Fill owner poisoned"),
            Self::WrongState(state) => write!(f, "Wipeout.Fill current state {state:?}"),
            Self::MissingState => f.write_str("Wipeout.Fill actual state missing"),
            Self::ForeignOwner => f.write_str("Wipeout.Fill foreign physical pair"),
            Self::Output(error) => write!(f, "Wipeout.Fill output: {error:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn wipeout_state_fill_guard(
        &self,
        component: PhysOutComponent,
    ) -> Result<(), WipeoutFillError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)
            .map_err(WipeoutFillError::Output)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
            .map_err(WipeoutFillError::Output)
    }
    fn try_fill_wipeout_state(
        &mut self,
        fields: &WipeoutGroundFields,
    ) -> Result<(), WipeoutFillError> {
        // Native short-circuit reads precede the first output write guard.
        let can_leave =
            fields.enough_time_for_reset && !fields.prevent_respawn && !fields.ignore_reset_button;
        self.wipeout_state_fill_guard(PhysOutComponent::Skeleton)?;
        self.concrete.skeleton.as_mut().unwrap().is_wipeout_over = fields.over;
        self.wipeout_state_fill_guard(PhysOutComponent::Skeleton)?;
        self.concrete
            .skeleton
            .as_mut()
            .unwrap()
            .time_in_wipeout_over = fields.time_over;
        self.wipeout_state_fill_guard(PhysOutComponent::State)?;
        self.concrete.state.as_mut().unwrap().can_leave_wipeout = can_leave;
        self.wipeout_state_fill_guard(PhysOutComponent::Animation)?;
        self.concrete
            .animation
            .as_mut()
            .unwrap()
            .wipeout_estimated_time_to_land = fields.estimated_time_to_collision;
        self.wipeout_state_fill_guard(PhysOutComponent::Skeleton)?;
        self.concrete
            .skeleton
            .as_mut()
            .unwrap()
            .time_until_wipeout_teleport = fields.time_until_teleport;
        self.wipeout_state_fill_guard(PhysOutComponent::Skeleton)?;
        self.concrete
            .skeleton
            .as_mut()
            .unwrap()
            .is_wipeout_teleport_pending = fields.auto_teleport_pending;
        if fields.request_teleport {
            self.wipeout_state_fill_guard(PhysOutComponent::State)?;
            self.concrete
                .state
                .as_mut()
                .unwrap()
                .physics_requested_teleport = true;
        }
        Ok(())
    }
}
pub(crate) fn try_fill_current_wipeout_state(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Result<(), WipeoutFillError> {
    use WipeoutFillError as E;
    let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
    if state.current != CurrentPhysicalState::WipeoutGround {
        return Err(E::WrongState(state.current));
    }
    let wipeout = state.wipeout_ground.as_ref().ok_or(E::MissingState)?;
    if !wipeout.bindings.board.same_as(board)
        || !wipeout.bindings.skeleton.same_as(skeleton)
        || !wipeout
            .bindings
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
    {
        return Err(E::ForeignOwner);
    }
    owner
        .phys_out()
        .lock()
        .map_err(|_| E::Poisoned)?
        .try_fill_wipeout_state(&wipeout.fields)
}
#[cfg(test)]
#[path = "physical_player_wipeout_fill_tests.rs"]
mod tests;
