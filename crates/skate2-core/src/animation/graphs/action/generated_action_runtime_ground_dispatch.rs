//! Exact generated grounded idle dispatch; no generic ancestry callback walk.
use super::{
    ActionUpdateError, ActorConstruction, ActorIdentity, BoundMotionGraphContext, Callback,
    GeneratedActionController,
};
use crate::generated_action_load::ids::state as state_id;
impl GeneratedActionController<ActorIdentity> {
    /// IdleTurning94/51 uses828ed788 -> `OnGround828ed1f0` -> `OnBoard828ecfe0`
    /// -> root828ecbc0. Source packed slots0..30 execute in that order.
    pub(super) fn update_ground_idle(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), ActionUpdateError> {
        if self.current_state != state_id::IDLE_TURNING_94
            || self.states[usize::from(state_id::IDLE_TURNING_94)].update_address != 0x828e_d788
        {
            return Err(ActionUpdateError::PendingStateUpdate(
                self.states[usize::from(self.current_state)].update_address,
            ));
        }
        let expected: Vec<u16> = (0..11).chain(42..50).chain(68..80).collect();
        if self.current_behaviours != expected {
            return Err(ActionUpdateError::InvalidActiveSet);
        }
        for position in 0..31 {
            self.dispatch_callback(actor, context, position, Callback::Update)?;
        }
        Ok(())
    }
}
