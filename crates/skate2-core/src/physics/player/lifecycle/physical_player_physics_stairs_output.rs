//! Actual last `Physics.Update` callback: `WatchForStairs`82e2cb70.
use crate::physics_state_ids::PHYSICS_STATE_PHYSICS_GROUND;
use super::super::{PhysOutComponent, PhysOutMotionScalar, PhysicalPlayerPhysOut};
use super::prior_input::PriorInputReadError;
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
};
#[derive(Debug)]
pub(crate) enum StairsError {
    ForeignOutput,
    MissingPhysics,
    Output(PriorInputReadError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for StairsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("stairs continuation belongs to another output"),
            Self::MissingPhysics => f.write_str("stairs requires published Physics conditioner"),
            Self::Output(e) => write!(f, "stairs output: {e:?}"),
            Self::Cvar(e) => write!(f, "stairs cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "stairs overlay: {e}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn stairs_guard(&self, component: PhysOutComponent) -> Result<(), StairsError> {
        self.check_current_guard()
            .map_err(|e| StairsError::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(component)
            .map_err(|e| StairsError::Output(PriorInputReadError::Component(e)))
    }
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "native BO4/BI25 exits also for unordered comparisons"
    )]
    pub(crate) fn try_watch_for_stairs(
        &mut self,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), StairsError> {
        self.stairs_guard(PhysOutComponent::State)?; //cb8c
        if self.concrete.state.as_ref().unwrap().physics_state != PHYSICS_STATE_PHYSICS_GROUND {
            return Ok(());
        }
        self.stairs_guard(PhysOutComponent::State)?; //cbc0
        if self.concrete.state.as_ref().unwrap().is_manual {
            return Ok(());
        }
        self.stairs_guard(PhysOutComponent::Collision)?; //cbf4
        if self.collision_type.as_ref().unwrap().value != 8 {
            return Ok(());
        }
        self.stairs_guard(PhysOutComponent::Motion)?; //cc08
        let speed = self.motion_scalars[PhysOutMotionScalar::DeckSpeed as usize]
            .as_ref()
            .unwrap()
            .value; //cc10
        let minimum = cvars
            .try_min_stair_trick_speed()
            .map_err(StairsError::Cvar)?; //cc18..1c
        // Both native exits test !GT (BO4/BI25), including unordered.
        if !(speed > minimum) {
            return Ok(());
        }
        self.stairs_guard(PhysOutComponent::SkateboardReckoning)?; //cc2c
        let nose_up = self
            .concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .transform
            .columns()[2]
            .xyz[1]
            .abs(); //cc34..38
        let maximum = cvars
            .try_max_stair_trick_deck_nose_up()
            .map_err(StairsError::Cvar)?; //cc40..44
        if !(nose_up > maximum) {
            return Ok(());
        }
        self.stairs_guard(PhysOutComponent::Animation)?; //cc54
        self.concrete
            .animation
            .as_mut()
            .unwrap()
            .should_disallow_tricks_due_to_stairs = true; //cc60
        overlay.require().map_err(StairsError::Overlay)?; //cc84
        //cc9c calls folded BLR825187c0. Green debug text arguments have no effect.
        Ok(())
    }
}
