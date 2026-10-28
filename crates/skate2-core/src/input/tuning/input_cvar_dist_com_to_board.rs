//! Proposed `input_cvar_runtime` child; definition237 is root-owned.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    /// kDistComToBoard, initializer82f86328/table82fd4f00; sampled unconditionally.
    pub(crate) fn try_motion_dist_com_to_board(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(237)
    }
}
