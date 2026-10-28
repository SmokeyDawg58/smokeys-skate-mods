//! Grind state, investigator and jumper references (`Sk8::Physics::k*`),
//! appended to the registry at 612. Variants keep the native labels.
use super::{InputCvarError, InputCvarReferences};

const BASE: usize = 612;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(non_camel_case_types, reason = "native cvar labels")]
pub(crate) enum GrindCvar {
    k5050CenterForce,
    k5050ContactDist,
    k5050ControlEpsilon,
    k5050ControlMatBlend,
    k5050ControlStrength,
    k5050ExitLift,
    k5050ExitLift_Rail,
    k5050ExitNudgeForce,
    k5050ExitNudgeForce_Rail,
    k5050ExitNudgeSpeed,
    k5050Friction_FatRail,
    k5050Friction_Ledge,
    k5050Friction_ThinRail,
    k5050MaxSeparation,
    k5050MaxSteer,
    k5050SideJumpScalar,
    k5050TruckDown,
    k5_OCenterForce,
    k5_OCenterToTruck,
    k5_OContactDist,
    k5_OControlStrength,
    k5_OExitLift,
    k5_OExitLift_Rail,
    k5_OExitNudgeForce,
    k5_OExitNudgeForce_Rail,
    k5_OExitNudgeSpeed,
    k5_OFriction_FatRail,
    k5_OFriction_Ledge,
    k5_OFriction_ThinRail,
    k5_OGrabGrindHoldUpMultiplier,
    k5_OGravity,
    k5_OLeanForward,
    k5_OMatBlend,
    k5_OMaxDeckDown,
    k5_OMaxDeckUp,
    k5_OMinDecksideSideness,
    k5_ONoise,
    k5_OPryOffOffset,
    k5_OSideJumpScalar,
    k5_OTruckDown,
    kAboveEntryAngle,
    kAirEngagement5050,
    kAirEngagement5_O,
    kAirEngagementBackslash,
    kAirEngagementBoardslide,
    kAirEngagementTipslide,
    kAlignmentForceDamping,
    kAngularDamping,
    kAngularWheelDamping,
    kAnimWeightGroundFrames,
    kAvgCurbHeight,
    kBackslashCenterForce,
    kBackslashDeckToGrindPoint,
    kBackslashExitLift,
    kBackslashExitNudgeForce,
    kBackslashExitNudgeSpeed,
    kBackslashFriction_FatRail,
    kBackslashFriction_Ledge,
    kBackslashFriction_ThinRail,
    kBackslashGrindPointDown,
    kBackslashMatBlend,
    kBackslashNoise,
    kBackslashSideJump,
    kBackslashSideJumpScalar,
    kBackslashTiltAngleDeg,
    kBoardControlLRDeckControl,
    kBoardslideExitLift,
    kBoardslideExitNudgeForce,
    kBoardslideExitNudgeSpeed,
    kBoardslideFriction_FatRail,
    kBoardslideFriction_Ledge,
    kBoardslideFriction_ThinRail,
    kBoardslideJumpVert,
    kBoardslideMatBlend,
    kBoardslideNoise,
    kBoardslideSideJumpScalar,
    kBoardslideTranslationForce,
    kBoardslideWipeoutDeckSpeed,
    kBodyTwistLRDeckControl,
    kChangeEntryAngle,
    kCoarseCurbParam,
    kContactDist_Boardslide,
    kContactDist_Tipslide,
    kCopingExitStartTime,
    kCopingSupportForce,
    kCurbFrictionScalar,
    kCurbHeightRange,
    kCurbHighSideRange,
    kCurbNormalLikeness,
    kDeckControlLRDeadzone,
    kDisplayPhysOutGrinds,
    kDrawGrindGeometryTest,
    kDrawGrindIntersectors,
    kDrawNearbyGrindPrimatives,
    kExitAssistForce,
    kFairlySteepExitStartTime,
    kFineGeometryDepth,
    kFineGeometryDist,
    kFlipTruckGrindControl,
    kForceExitNormalThresh,
    kForceExitRideOutTiltDeg,
    kForceExitRollOutTiltDeg,
    kForceExitTestDown,
    kForceExitTestOut,
    kForcedGrindSurface,
    kGrindCommonExitSpeed,
    kGrindCommonFrictionScalar,
    kGrindCommonJumpSide,
    kGrindCommonJumpVert,
    kGrindCommonJumpVertScalar,
    kGrindCrouchScalar,
    kGrindExitFrames,
    kGrindFriction_Rough,
    kGrindFriction_Slippery,
    kGrindFriction_Slow,
    kGrindFriction_Smooth,
    kGrindFriction_Unrideable,
    kGrindFriction_VerySlow,
    kGrindReckoningSmoothnessInGrind,
    kGrindReckoningSmoothnessOnExit,
    kGroundEngagement5050,
    kGroundEngagement5_O,
    kGroundEngagementBackslash,
    kGroundEngagementBoardslide,
    kGroundEngagementTipslide,
    kGroundToBoardslideFrames,
    kGroundToBoardslideSpeed,
    kHorizontalGrindAvoidSpeed,
    kHorizontalGrindWipeoutSpeed,
    kJumpEnergyCost,
    kJumpEnergyRegeneration,
    kJumpNormalVertFraction,
    kLooseEndRiseThresh,
    kMaxBalanceAngleDeg,
    kMaxCopingDV0,
    kMaxCopingDV1,
    kMaxCos_5_O,
    kMaxDropInSpeed,
    kMaxInvExitFrames,
    kMaxTiltFor_5_O,
    kMinBoardslideDeckTilt,
    kMinCopingVert,
    kMinDeckDZTipslideDropIn,
    kMinGrindFrames,
    kMinGrindSpeedBoardslide,
    kMinNonGroundStairFrames,
    kMinSideJumpLowEnergy,
    kMinSpeedWeightIgnore,
    kNoiseFullSpeed,
    kNoiseScalar,
    kNormalSmoothness,
    kOffBalanceGrindThresh,
    kPinForce,
    kPrepareJumpClearance,
    kPrepareJumpForce,
    kShowTruckAvoid,
    kSideJumpEnergy,
    kSpamGrindEntryAngle,
    kStayEntryAngle,
    kTestHeightHeuristic,
    kTipslideExitLift,
    kTipslideExitNudgeForce,
    kTipslideExitNudgeSpeed,
    kTipslideFriction_FatRail,
    kTipslideFriction_Ledge,
    kTipslideFriction_ThinRail,
    kTipslideGrabFrames,
    kTipslideGrabNudgeForce,
    kTipslideGrabSlow,
    kTipslideJumpVert,
    kTipslideMatBlend,
    kTipslideNoise,
    kTipslideNormalSmoothness,
    kTipslideSideJump,
    kTipslideSideJumpScalar,
    kTipslideStabilityNudgeForce,
    kTipslideTiltAngleDeg,
    kTrajSelectorAvoidGrindFrames,
    kTruckAvoidForce,
    kTruckClearance,
    kTruckControlCompensationForce,
    kTruckGrindControlClampedSmoothness,
    kTruckGrindControlSmoothness,
    kUpVectorSmoothness,
    kVelDampingScalar,
    kWheelClearAngleDeg,
    kMinChromosomeEqualityFrames,
    kMinChromosomeEqualityFrames_Scoring,
    kbGrindConditionerDebugDraw,
    kDeckTiltForUp,
    kNonspecificSideJumpScalar,
    kNonspecificReckoningSmoothness,
    kNonspecificCollExitFrames,
    kNonspecificCollFreeStickyFrames,
    kNonspecificToGroundNearGrindFrames,
    kPlayerWeight,
    kDoCollisionResponse,
    kDoWipeoutsTest,
    kInsignificantGrindSpeed,
    kMinPairDist,
    kMaxPairDist,
    kMaxPairDistY,
    kDrawGrindPairDetection,
    kShowGrindInfo,
    kRenderGrindLandings,
    kRenderGrindLandingUp,
    kDrawGrindAssistInfo,
    kGrindAssistActive,
    kGrindAssistApplyOffset,
    kGrindAssistApplyAngleOffset,
    kFP_MaxDistToAdjustFootPlant,
    kFP_MaxDistBelowCollisionForGrind,
    kUsePhysInducedGrindCrouch,
    kShowGrindCrouchText,
    kShowGrindControlFade,
}

impl InputCvarReferences {
    pub(crate) fn try_grind_float(&self, cvar: GrindCvar) -> Result<f32, InputCvarError> {
        self.try_current_float(BASE + cvar as usize)
    }
    pub(crate) fn try_grind_int(&self, cvar: GrindCvar) -> Result<i32, InputCvarError> {
        self.try_current_integer(BASE + cvar as usize)
    }
    pub(crate) fn try_grind_bool(&self, cvar: GrindCvar) -> Result<bool, InputCvarError> {
        self.try_current_boolean(BASE + cvar as usize)
    }
    pub(crate) fn try_grind_enum(&self, cvar: GrindCvar) -> Result<i32, InputCvarError> {
        self.try_current_enum(BASE + cvar as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::GrindCvar as C;
    use crate::input_cvar_runtime::{InputCvarManager, InputCvarReferences};
    #[test]
    fn appended_grind_references_link_with_native_defaults() {
        let refs = InputCvarReferences::new_static_initialized();
        InputCvarManager::new().try_link(&refs).unwrap();
        assert_eq!(refs.try_grind_float(C::k5050CenterForce).unwrap().to_bits(), 0x4448_0000);
        assert_eq!(refs.try_grind_float(C::k5050ContactDist).unwrap().to_bits(), 0x3e05_1eb8);
        assert_eq!(refs.try_grind_float(C::k5050ControlEpsilon).unwrap().to_bits(), 0x3df5_c28f);
        assert_eq!(refs.try_grind_int(C::kAnimWeightGroundFrames).unwrap(), 30);
        assert!(!refs.try_grind_bool(C::kDisplayPhysOutGrinds).unwrap());
        assert_eq!(refs.try_grind_enum(C::kForcedGrindSurface).unwrap(), 0);
    }
    #[test]
    fn conditioner_and_nonspecific_references_link_with_native_defaults() {
        let refs = InputCvarReferences::new_static_initialized();
        InputCvarManager::new().try_link(&refs).unwrap();
        assert_eq!(
            refs.try_grind_int(C::kMinChromosomeEqualityFrames).unwrap(),
            0
        );
        assert_eq!(
            refs.try_grind_int(C::kMinChromosomeEqualityFrames_Scoring)
                .unwrap(),
            12
        );
        assert!(!refs.try_grind_bool(C::kbGrindConditionerDebugDraw).unwrap());
        assert_eq!(
            refs.try_grind_float(C::kDeckTiltForUp).unwrap().to_bits(),
            0xbdcc_cccd
        );
        assert_eq!(
            refs.try_grind_float(C::kNonspecificSideJumpScalar)
                .unwrap()
                .to_bits(),
            0x3f33_3333
        );
        assert_eq!(
            refs.try_grind_float(C::kNonspecificReckoningSmoothness)
                .unwrap()
                .to_bits(),
            0x3f7a_e148
        );
        assert_eq!(
            refs.try_grind_int(C::kNonspecificCollExitFrames).unwrap(),
            60
        );
        assert_eq!(
            refs.try_grind_int(C::kNonspecificCollFreeStickyFrames)
                .unwrap(),
            2
        );
        assert_eq!(
            refs.try_grind_int(C::kNonspecificToGroundNearGrindFrames)
                .unwrap(),
            10
        );
        assert_eq!(
            refs.try_grind_float(C::kPlayerWeight).unwrap().to_bits(),
            0x4220_0000
        );
        assert!(refs.try_grind_bool(C::kDoCollisionResponse).unwrap());
        assert!(refs.try_grind_bool(C::kDoWipeoutsTest).unwrap());
    }
    #[test]
    fn camera_grind_ledge_reference_links_with_native_default() {
        let refs = InputCvarReferences::new_static_initialized();
        InputCvarManager::new().try_link(&refs).unwrap();
        assert_eq!(
            refs.try_grind_float(C::kInsignificantGrindSpeed)
                .unwrap()
                .to_bits(),
            0x3e99_999a
        );
    }
    #[test]
    fn selector_grind_references_link_with_native_defaults() {
        let refs = InputCvarReferences::new_static_initialized();
        InputCvarManager::new().try_link(&refs).unwrap();
        assert_eq!(refs.try_grind_float(C::kMinPairDist).unwrap().to_bits(), 0x3c23_d70a);
        assert_eq!(refs.try_grind_float(C::kMaxPairDist).unwrap().to_bits(), 0x3dcc_cccd);
        assert_eq!(refs.try_grind_float(C::kMaxPairDistY).unwrap().to_bits(), 0x3ca3_d70a);
        assert!(!refs.try_grind_bool(C::kDrawGrindPairDetection).unwrap());
        assert!(!refs.try_grind_bool(C::kShowGrindInfo).unwrap());
        assert!(!refs.try_grind_bool(C::kRenderGrindLandings).unwrap());
        assert!(!refs.try_grind_bool(C::kRenderGrindLandingUp).unwrap());
    }
    #[test]
    fn grind_air_assist_cvars_link_with_source_defaults() {
        let refs = InputCvarReferences::new_static_initialized();
        InputCvarManager::new().try_link(&refs).unwrap();
        assert!(!refs.try_grind_bool(C::kDrawGrindAssistInfo).unwrap());
        assert!(refs.try_grind_bool(C::kGrindAssistActive).unwrap());
        assert!(refs.try_grind_bool(C::kGrindAssistApplyOffset).unwrap());
        assert!(
            refs.try_grind_bool(C::kGrindAssistApplyAngleOffset)
                .unwrap()
        );
    }
    #[test]
    fn footplant_grind_cvars_link_with_source_defaults() {
        let refs = InputCvarReferences::new_static_initialized();
        InputCvarManager::new().try_link(&refs).unwrap();
        assert_eq!(
            refs.try_grind_float(C::kFP_MaxDistToAdjustFootPlant)
                .unwrap()
                .to_bits(),
            0x3e80_0000
        );
        assert_eq!(
            refs.try_grind_float(C::kFP_MaxDistBelowCollisionForGrind)
                .unwrap()
                .to_bits(),
            0x3cf5_c28f
        );
    }
    #[test]
    fn grind_motion_behaviour_cvars_link_with_source_defaults() {
        let refs = InputCvarReferences::new_static_initialized();
        InputCvarManager::new().try_link(&refs).unwrap();
        assert!(refs.try_grind_bool(C::kUsePhysInducedGrindCrouch).unwrap());
        assert!(!refs.try_grind_bool(C::kShowGrindCrouchText).unwrap());
        assert!(!refs.try_grind_bool(C::kShowGrindControlFade).unwrap());
    }
}
