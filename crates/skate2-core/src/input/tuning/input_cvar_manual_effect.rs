//! Unregistered references144/145 for the actual manual calculation.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    /// Ref8308d8d8, init82f7b438, table4438: `Physics_Manual`, Draw Manual Info, false.
    pub(crate) fn try_show_manual_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(144)
    }
    /// Ref8308ddb0, init82f7b5e8, table444c: `Physics_Manual`, Wrong Wheel Scalar, .5 [0,1].
    pub(crate) fn try_wrong_wheel_scalar(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(145)
    }
}
