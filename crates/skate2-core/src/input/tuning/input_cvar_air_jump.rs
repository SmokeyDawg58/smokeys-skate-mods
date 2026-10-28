//! Captured actual Air jump cvar targets; pointer and value reads stay separate.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use std::sync::Arc;
#[derive(Clone, Copy)]
pub(crate) enum AirJumpFloat {
    MaximumError,
    MinimumYError,
}
pub(crate) struct AirJumpFloatTarget(Target);
impl AirJumpFloatTarget {
    pub(crate) fn read(&self) -> Result<f32, InputCvarError> {
        match *self.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
impl InputCvarReferences {
    pub(crate) fn try_capture_air_jump_float(
        &self,
        field: AirJumpFloat,
    ) -> Result<AirJumpFloatTarget, InputCvarError> {
        let index = match field {
            AirJumpFloat::MaximumError => 246,
            AirJumpFloat::MinimumYError => 247,
        };
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(AirJumpFloatTarget(Arc::clone(
            targets[index]
                .as_ref()
                .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?,
        )))
    }
    /// `kMaxJumpError` (82fd4434), the only cvar `Toolkit_CalcGroundJump82b315a8`
    /// reads at 82b315e4. PhysicsAir's jump fix uses the separate 246 instead.
    pub(crate) fn try_capture_ground_jump_maximum_error(
        &self,
    ) -> Result<AirJumpFloatTarget, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(AirJumpFloatTarget(Arc::clone(
            targets[596]
                .as_ref()
                .ok_or(InputCvarError::Unlinked(DEFINITIONS[596].label))?,
        )))
    }
}
#[cfg(test)]
mod tests {
    use super::{AirJumpFloat, InputCvarReferences};
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn captured_target_retains_identity_across_relink_and_reloads_current_value() {
        let refs = InputCvarReferences::new_static_initialized();
        assert!(
            refs.try_capture_air_jump_float(AirJumpFloat::MaximumError)
                .is_err()
        );
        let mut old = InputCvarManager::new();
        old.try_link(&refs).unwrap();
        let maximum = refs
            .try_capture_air_jump_float(AirJumpFloat::MaximumError)
            .unwrap();
        let minimum = refs
            .try_capture_air_jump_float(AirJumpFloat::MinimumYError)
            .unwrap();
        assert_eq!(maximum.read().unwrap().to_bits(), 3.0f32.to_bits());
        assert_eq!(minimum.read().unwrap().to_bits(), (-3.0f32).to_bits());
        old.try_deserialize_startup(Some(
            b"[Config]\r\nkPhysicsAirJumpFixMaxError=7\r\nkJumpVelMinYError=-9\r\n",
        ))
        .unwrap();
        assert_eq!(maximum.read().unwrap().to_bits(), 7.0f32.to_bits());
        assert_eq!(minimum.read().unwrap().to_bits(), (-9.0f32).to_bits());
        let mut new = InputCvarManager::new();
        new.try_link(&refs).unwrap();
        assert_eq!(maximum.read().unwrap().to_bits(), 7.0f32.to_bits());
        assert_eq!(
            refs.try_capture_air_jump_float(AirJumpFloat::MaximumError)
                .unwrap()
                .read()
                .unwrap()
                .to_bits(),
            3.0f32.to_bits()
        );
    }
}
