//! Late Ground collision cvar at82d9a634..a638, unregistered child.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    pub(crate) fn try_dot_to_stop_push(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(128)
    }
}
