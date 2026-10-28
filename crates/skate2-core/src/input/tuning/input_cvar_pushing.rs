//! OffBoardPushing runtime references. IDs append to the shared registry;
//! kEnableRepushingTimeLimit reuses its existing getter and index44.
use super::{InputCvarError, InputCvarReferences};

#[derive(Clone, Copy, Debug)]
#[expect(non_camel_case_types, reason = "native cvar labels")]
pub(crate) enum PushingFloat {
    kPushingMaxArmLengthForIK = 864,
    kOBP_MaxTimePartialRagdoll = 865,
    kMOVOBJ_ForceSmoothing = 866,
    kPushFromTurnScalar = 867,
    kOBP_CollisionVelScalar = 868,
    kMaxJerkPerFrame = 869,
    kMOVOBJ_Crouch_Scalar = 871,
    kMOVOBJ_Crouch_Scalar_Initial = 872,
    kMOVOBJ_HandsZOffset = 873,
    kMOVOBJ_MaxHeightAboveGround = 874,
    kMOVOBJ_ApproxSpeedBlend = 875,
    kMOVOBJ_worldPositionMinDelta = 876,
    kMOVOBJ_worldPositionMinBlend = 877,
    kMOVOBJ_NoSpeedTimeout = 878,
    kMOVOBJ_HandsLevelToBeginIKBlend = 879,
    kMOVOBJ_MinHookToHips = 880,
    kMOVOBJ_MinHookToHipsForExit = 881,
    kMOVOBJ_CollisionBlendIn = 882,
    kMOVOBJ_CollisionBlendOut = 884,
}

#[derive(Clone, Copy, Debug)]
#[expect(non_camel_case_types, reason = "native cvar labels")]
pub(crate) enum PushingBool {
    kOBPushingDebugDraw = 860,
    kStayInState = 861,
    kEnableRepushingTimeLimitOnExit = 862,
    kEnableEmergencyExit = 863,
    kOBP_EnableCollResponse = 870,
    kEnableOffBoardPushingWipeouts = 883,
}

impl InputCvarReferences {
    pub(crate) fn try_interactable_line_count(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(885)
    }
    pub(crate) fn try_interactable_fatness(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(886)
    }
    pub(crate) fn try_interactable_max_height(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(887)
    }
    pub(crate) fn try_interactable_distance(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(888)
    }
    pub(crate) fn try_draw_interactable_lines(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(889)
    }
    pub(crate) fn try_pushing_float(&self, cvar: PushingFloat) -> Result<f32, InputCvarError> {
        self.try_current_float(cvar as usize)
    }
    pub(crate) fn try_pushing_bool(&self, cvar: PushingBool) -> Result<bool, InputCvarError> {
        self.try_current_boolean(cvar as usize)
    }
}
