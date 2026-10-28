//! ref142:8308dc44, initializer82f7b0e8, table82fd4410.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    pub(crate) fn try_apply_antiflip_slide(&self) -> Result<bool, InputCvarError> { self.try_current_boolean(533) }
    /// `Physics_Skateboard` / kDrawAntiFlipTorque / Draw Anti Flip Torque;
    /// bool false, null callback. Read before final `AntiFlip` output store.
    pub(crate) fn try_draw_antiflip_torque(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(142)
    }
}
