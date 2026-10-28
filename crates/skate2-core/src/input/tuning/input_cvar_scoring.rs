//! Native scoring references82f9d490..82f9da38, shared cvar registry.
use super::{InputCvarError, InputCvarReferences};
#[derive(Clone, Copy, Debug)]
#[expect(non_camel_case_types, reason = "native cvar names")]
pub(crate) enum ScoringBool {
    kOverrideScoreMultiplier = 982,
    kMakeAllLandingsSketchy = 985,
    kScoreEntityTypeInvalidDMOs = 986,
    kAllowSlowManualsToAlwaysScore = 990,
    kAllowSlowManualsToStillScoreOnceBegun = 991,
    kEnableBoneDamagePropogation = 996,
    kUseBoneAccelerationForForce = 999,
    kIgnoreHoMRotationWhenStationary = 1002,
    kIsHoMEffectEnabled = 1003,
    kIsHoMEffectForceEnabled = 1004,
    kbDoSpatialScoreTest = 1005,
}
impl InputCvarReferences {
    pub(crate) fn try_trick_display_tuning(&self) -> Result<crate::ui::trick_display::DisplayTuning, InputCvarError> {
        Ok(crate::ui::trick_display::DisplayTuning {
            allowed: self.try_current_boolean(1006)?,
            nollie_delay: self.try_current_integer(1007)?,
            jolt_threshold: self.try_current_float(1008)?,
            jolt_distance: self.try_current_float(1009)?,
            jolt_speed: self.try_current_float(1010)?,
            jolt_recovery: self.try_current_float(1011)?,
        })
    }
    pub(crate) fn try_scoring_bool(&self, key: ScoringBool) -> Result<bool, InputCvarError> {
        self.try_current_boolean(key as usize)
    }
}
#[derive(Clone, Copy, Debug)]
#[expect(non_camel_case_types, reason = "native cvar names")]
pub(crate) enum ScoringFloat {
    kMinAnimScalarForSketchyLanding = 984,
    kSkaterContextRadius = 987,
    kPedContextRadius = 988,
    kMinDistancePerFrameToAccumulate = 989,
    kDamageScalar = 995,
    kMinDamageToPropogate = 997,
    kDamageFadeInRate = 1000,
    kMinAngleToBankInDegrees = 1001,
}
impl InputCvarReferences {
    pub(crate) fn try_scoring_float(&self, key: ScoringFloat) -> Result<f32, InputCvarError> {
        self.try_current_float(key as usize)
    }
}
#[derive(Clone, Copy, Debug)]
#[expect(non_camel_case_types, reason = "native cvar names")]
pub(crate) enum ScoringInt {
    kOverrideScoreMultiplierValue = 983,
    kMinFramesBeforeEndingSequenceAfterLanding = 992,
    kExtraFramesThatOffGroundWheelsKeepSequenceActive = 993,
    kNumFramesToResetHeavyLandingData = 998,
}
impl InputCvarReferences {
    pub(crate) fn try_scoring_forced_damage_level(&self) -> Result<i32, InputCvarError> {
        self.try_current_enum(994)
    }
    pub(crate) fn try_scoring_int(&self, key: ScoringInt) -> Result<i32, InputCvarError> {
        self.try_current_integer(key as usize)
    }
}
