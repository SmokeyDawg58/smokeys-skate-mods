//! `PhysicalPlayerHiLOD::UpdateCollisionStatus`82e016a0 live references.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    /// `kLWTestRadius` target (reference830e0cb4), read at 82e01740, 82e01894
    /// and 82e01b04 for the LivingWorld overlap spheres.
    pub(crate) fn try_lw_test_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(610)
    }
    /// `kbShowCollisionStateChecks` +0xcd (reference830e0e74): when set, each
    /// query asserts on the DebugOverlay singleton (82e018ec, 82e01b70, 82e01bf8).
    pub(crate) fn try_show_collision_state_checks(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(611)
    }
}
