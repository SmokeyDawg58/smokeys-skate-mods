//! Completion of the seven-call `Physics.Update`82e2f970, not `UpdateAll`.
use super::{Conditioners, PhysicalPlayerPhysOut, landing_quality::PhysicsBeforeStairs};
use crate::{
    debug_overlay::DebugOverlaySlot, input_cvar_runtime::InputCvarReferences,
    physical_player_phys_out::construct::stairs::StairsError,
};
use std::sync::Arc;
pub(crate) struct PhysicsUpdateComplete {
    output: Arc<()>,
}
impl PhysicsUpdateComplete {
    pub(crate) fn matches_output(&self, output: &PhysicalPlayerPhysOut) -> bool {
        Arc::ptr_eq(&self.output, output.occurrence())
    }
}
impl Conditioners {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "consume the prerequisite token to prevent replay"
    )]
    pub(crate) fn try_finish_physics_update(
        &mut self,
        before: PhysicsBeforeStairs,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<PhysicsUpdateComplete, StairsError> {
        if !before.matches_output(output) || !self.matches_output(output) {
            return Err(StairsError::ForeignOutput);
        }
        let physics = self
            .physics
            .published_value_mut()
            .ok_or(StairsError::MissingPhysics)?;
        if !Arc::ptr_eq(&physics.output, output.occurrence()) {
            return Err(StairsError::ForeignOutput);
        }
        output.try_watch_for_stairs(cvars, overlay)?;
        Ok(PhysicsUpdateComplete {
            output: Arc::clone(output.occurrence()),
        })
    }
}
#[cfg(test)]
#[path = "physical_player_physics_stairs_tests.rs"]
mod tests;
