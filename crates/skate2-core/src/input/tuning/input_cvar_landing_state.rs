//! Native state505 live variables; all samples read the retained registry.
use super::{InputCvarError, InputCvarReferences};
#[derive(Clone, Copy)]
#[repr(usize)]
pub(crate) enum LandingCvar {
    Debug = 449,
    EnableFeetIk = 450,
    MinDotForFeetIk = 451,
    TimeToEnableIk = 452,
    EnableSpin = 453,
    AngleTransitionTime = 454,
    TimeForLandingWipeout = 455,
    DeckYOffset = 456,
    WipeoutTime = 457,
}
impl InputCvarReferences {
    pub(crate) fn try_landing_state_float(&self, key: LandingCvar) -> Result<f32, InputCvarError> {
        self.try_current_float(key as usize)
    }
    pub(crate) fn try_landing_state_bool(&self, key: LandingCvar) -> Result<bool, InputCvarError> {
        self.try_current_boolean(key as usize)
    }
}
