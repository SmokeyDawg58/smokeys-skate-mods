//! KnownAir variables retain the native linked-registry reads.
use super::{InputCvarError, InputCvarReferences};
#[derive(Clone, Copy)]
#[repr(usize)]
pub(crate) enum KnownAirCvar {
    FixSkateboardInAir = 523,
    DrawLandingInfo = 524,
    DrawKnownAir = 525,
    DrawUpsideWipeout = 526,
    AlignFrames = 527,
    AgainstSpinScalar = 528,
    FakieAlignAngle = 529,
    MinFootPlantTime = 530,
    WarningTime = 531,
    StateWarnings = 532,
}
impl InputCvarReferences {
    pub(crate) fn try_known_air_float(&self, key: KnownAirCvar) -> Result<f32, InputCvarError> {
        self.try_current_float(key as usize)
    }
    pub(crate) fn try_known_air_bool(&self, key: KnownAirCvar) -> Result<bool, InputCvarError> {
        self.try_current_boolean(key as usize)
    }
}
