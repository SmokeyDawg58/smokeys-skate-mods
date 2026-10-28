//! Proposed intent-bridge child for actual condition98; no computed replacement.
use crate::physics_state_ids::{PHYSICS_STATE_LANDINGONDECK, PHYSICS_STATE_OFFBOARD_LOCOMOTION};
use super::{ActionMotionIntentError, ActorConstruction, ActorConstructionError, ActorIdentity};
use crate::motion_graph_entity_authority::BoundMotionGraphContext;
impl ActorConstruction {
    /// Both physical-state conditions resolve the same Action-bound IPhysical
    /// and compare raw PhysOut.State+0x10, not the filtered category.
    fn action_runtime_has_raw_physical_state(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        state: i32,
    ) -> Result<bool, ActionMotionIntentError> {
        use ActionMotionIntentError as E;
        self.check_action_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
        Ok(output.try_state_output().map_err(E::PhysOut)?.physics_state == state)
    }
    /// `IsLandingOnBoard`82c98d70 compares raw state 505.
    pub(crate) fn action_runtime_landing_on_board(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, ActionMotionIntentError> {
        self.action_runtime_has_raw_physical_state(context, PHYSICS_STATE_LANDINGONDECK)
    }
    /// `IsInLocomotion`82c98e78 compares raw state 504.
    pub(crate) fn action_runtime_is_in_locomotion(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, ActionMotionIntentError> {
        self.action_runtime_has_raw_physical_state(context, PHYSICS_STATE_OFFBOARD_LOCOMOTION)
    }
    /// `PhysicsSpeedAndSlopeCompare`82c99710 resolves physical, then Motion+c0.
    pub(crate) fn action_runtime_heading_adjust_factor(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<f32, ActionMotionIntentError> {
        use ActionMotionIntentError as E;
        self.check_action_runtime_context(context)?;
        let allocation = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = allocation
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
        output
            .try_action_heading_adjust_factor()
            .map_err(E::PhysOut)
    }
}
