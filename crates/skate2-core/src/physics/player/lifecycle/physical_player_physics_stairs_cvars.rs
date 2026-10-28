//! Live `WatchForStairs` thresholds; initializer order220 then221.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    pub(crate) fn try_max_stair_trick_deck_nose_up(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(220)
    }
    pub(crate) fn try_min_stair_trick_speed(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(221)
    }
}
