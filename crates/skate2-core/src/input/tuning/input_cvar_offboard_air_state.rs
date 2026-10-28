//! Live state502 cvars; static initialization order is retained by the parent.
use super::{InputCvarError, InputCvarReferences};
#[derive(Clone, Copy)]
#[repr(usize)]
pub(crate) enum AirCvar {
    RunoutUpwardVelocityNudge = 410,
    RunoutZXVelocityReduction = 411,
    JumpForwardDistance = 412,
    OffboardAirMaxLandOnBoardAssist = 413,
    StandingJumpMaxLandOnBoardAssist = 414,
    RunoutTransitionTime = 415,
    MinTimeInAirForRunoff = 416,
    RunoutTimeToLandScalar = 417,
    RunOutMinAngleForWipeoutAir = 418,
    RunOutMinAngleForWipeout = 419,
    OffBoarAirYSpeedForWipeout = 420,
    OBAirMinUpVectorY = 421,
    RunoutProportionalAngle = 422,
    RunOutMaxAngleForLocomotion = 423,
    MinTimeInStateForSkateboardWipeout = 424,
}
impl InputCvarReferences {
    pub(crate) fn try_offboard_air_state(&self, key: AirCvar) -> Result<f32, InputCvarError> {
        self.try_current_float(key as usize)
    }
}
