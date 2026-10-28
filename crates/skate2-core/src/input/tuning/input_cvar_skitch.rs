//! Live native Physics_States_Skitching and shimmy references; shared registry.
use super::{InputCvarError, InputCvarReferences};
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum SkitchBool {
    kSkitchingUseBetterSteerForceNormals = 960,
    kForceSkitchStick = 961,
    kSkitchingAllowManuals = 962,
    kEnableFakeFollowPointPhysics = 964,
    kEnableCentrifugalAcceleration = 965,
    kSkitchEnableIK = 971,
    kSkitchForceLeftHand = 972,
    kSkitchForceRightHand = 973,
    kSkitchingApplyStandardTilt = 975,
    kSkitchingApplySpeedWobble = 976,
    kSkitchingUseTorque = 978,
    kbSkitchShimmyDebugDisplay = 981,
}
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum SkitchFloat {
    kSkitchMinTurnForExit = 966,
    kStickHandSlowDownEffect = 967,
    kDeckBlendAmount = 968,
    kDeckMaxOffsetDelta = 969,
    kMinDeckOffsetAngle = 970,
    kSkitchCrouchSpringScalar = 974,
    kSkitchDebugHandSpeed = 979,
    kSkitchMaxCrouchDelta = 980,
}
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum SkitchInt {
    kNumberOfFramesToDisableHandsAfterSkitch = 963,
    kFramesToExitSkitching = 977,
}
impl InputCvarReferences {
    pub(crate) fn try_skitch_offset_blend_frames(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(60)
    }
    pub(crate) fn try_skitch_bool(&self, cvar: SkitchBool) -> Result<bool, InputCvarError> {
        self.try_current_boolean(cvar as usize)
    }
    pub(crate) fn try_skitch_float(&self, cvar: SkitchFloat) -> Result<f32, InputCvarError> {
        self.try_current_float(cvar as usize)
    }
    pub(crate) fn try_skitch_int(&self, cvar: SkitchInt) -> Result<i32, InputCvarError> {
        self.try_current_integer(cvar as usize)
    }
}
