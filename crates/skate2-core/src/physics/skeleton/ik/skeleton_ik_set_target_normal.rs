//! `SkeletonIK::SetTargetNormal` 0x82b3da78, reached from offboard feet IK.
use super::SkeletonIkState;
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning::air::{AirVectorError, try_clamp_air_angle},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::convert::Infallible;

#[derive(Debug)]
pub(crate) enum SetTargetNormalError {
    Cvar(InputCvarError),
    Clamp(AirVectorError<ReckoningHostNumericError, Infallible>),
}

impl SkeletonIkState {
    /// The previous blend gates the clamp and its angle cvar. The set byte
    /// precedes the blend-speed read; a failed later read keeps that byte.
    pub(crate) fn try_set_foot_target_normal(
        &mut self,
        left: bool,
        normal: SkeletonRootVector,
        cvars: &InputCvarReferences,
    ) -> Result<(), SetTargetNormalError> {
        use SetTargetNormalError as E;
        let entry = usize::from(!left);
        // Native fcmpu; ble skips the clamp only for ordered <= 0. NaN is
        // unordered and enters the clamp branch.
        if !(self.external_target_normal_blends[entry] <= 0.0) {
            let range = cvars.try_max_normal_angle_delta().map_err(E::Cvar)?;
            let clamped = try_clamp_air_angle(
                normal,
                self.external_target_normals[entry],
                range,
                &mut HostReckoningNumeric,
                &mut HostReckoningSinCosNumeric,
            )
            .map_err(E::Clamp)?;
            self.external_target_normals[entry] = clamped;
        }
        self.external_target_normal_set[entry] = true;
        let speed = cvars.try_target_normal_blend_speed().map_err(E::Cvar)?;
        let sum = speed + self.external_target_normal_blends[entry];
        self.external_target_normal_blends[entry] = if sum - 1.0 >= 0.0 { 1.0 } else { sum };
        Ok(())
    }
}
