//! Source `HasAcceleratedStrangely` live float reads; refs8308db58/8308db8c.
use super::{InputCvarError, InputCvarReferences};

impl InputCvarReferences {
    pub(crate) fn try_min_bad_velocity_y(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(157)
    }
    pub(crate) fn try_max_good_velocity_y(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(158)
    }
}
