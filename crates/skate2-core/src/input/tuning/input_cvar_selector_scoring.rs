//! Actual selector debug/scoring references, initialized in table82fd5c70..88.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    pub(crate) fn try_show_trajectory_select(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(252)
    }
    pub(crate) fn try_show_raw_trajectory(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(253)
    }
    pub(crate) fn try_select_central_trajectory(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(254)
    }
    pub(crate) fn try_show_wall_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(255)
    }
}
