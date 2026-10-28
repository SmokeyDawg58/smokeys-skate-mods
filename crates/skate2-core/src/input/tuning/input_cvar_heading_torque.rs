//! ref143:8308d8fc, initializer82f7b1d8, table82fd441c.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    /// `Physics_Skateboard` / kDrawVertTurning / Draw Vert Turning Info.
    /// Native boolean false, null callback; sampled before final output.
    pub(crate) fn try_draw_vert_turning(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(143)
    }
}
