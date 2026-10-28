//! Live captured `BodySpin` targets and late Reckoning debug references.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::{
    reckoning::{ReckoningCvars, ReckoningDebug},
    reckoning_body_spin::{BodySpinCvar, BodySpinCvars},
};
use std::sync::Arc;
pub(crate) struct ReckoningCvarAdapter<'a>(pub(crate) &'a InputCvarReferences);
pub(crate) struct BodySpinTarget(Target);
impl BodySpinCvars for ReckoningCvarAdapter<'_> {
    type Pointer = BodySpinTarget;
    type Error = InputCvarError;
    fn resolve(&mut self, field: BodySpinCvar) -> Result<Self::Pointer, Self::Error> {
        let index = match field {
            BodySpinCvar::DerivativeScalar => 86,
            BodySpinCvar::DerivativeBlend => 87,
            BodySpinCvar::DerivativeThreshold => 88,
            BodySpinCvar::InputBlend => 89,
        };
        let targets = self.0.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(BodySpinTarget(Arc::clone(targets[index].as_ref().ok_or(
            InputCvarError::Unlinked(DEFINITIONS[index].label),
        )?)))
    }
    fn float(&mut self, target: &Self::Pointer) -> Result<f32, Self::Error> {
        match *target.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(v) => Ok(v),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
impl ReckoningCvars for ReckoningCvarAdapter<'_> {
    fn debug_enabled(&mut self, kind: ReckoningDebug) -> Result<bool, Self::Error> {
        self.0.try_current_boolean(match kind {
            ReckoningDebug::Reckoning => 90,
            ReckoningDebug::Coffin => 91,
            ReckoningDebug::BodyTilt => 92,
        })
    }
}

impl crate::reckoning_body_spin::air::BodySpinAirCvars for ReckoningCvarAdapter<'_> {
    fn resolve_air(
        &mut self,
        field: crate::reckoning_body_spin::air::AirCvar,
    ) -> Result<Self::Pointer, Self::Error> {
        use crate::reckoning_body_spin::air::AirCvar;
        let index = match field {
            AirCvar::InputFade => 242,
            AirCvar::ForceAutoSpeed => 243,
            AirCvar::MaxAutoSpin => 244,
        };
        let targets = self.0.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(BodySpinTarget(Arc::clone(targets[index].as_ref().ok_or(
            InputCvarError::Unlinked(DEFINITIONS[index].label),
        )?)))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BodySpinCvar, BodySpinCvars, ReckoningCvarAdapter, ReckoningCvars, ReckoningDebug,
    };
    use crate::input_cvar_runtime::{InputCvarManager, InputCvarReferences};
    #[test]
    fn captured_body_spin_targets_survive_relink_and_live_reads_keep_timing() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut first = InputCvarManager::new();
        first.try_link(&refs).unwrap();
        let mut c = ReckoningCvarAdapter(&refs);
        for (field, bits) in [
            (BodySpinCvar::DerivativeScalar, 0x3fc0_0000),
            (BodySpinCvar::DerivativeBlend, 0x3f80_0000),
            (BodySpinCvar::DerivativeThreshold, 0x3dcc_cccd),
            (BodySpinCvar::InputBlend, 0x3e4c_cccd),
        ] {
            let pointer = c.resolve(field).unwrap();
            assert_eq!(c.float(&pointer).unwrap().to_bits(), bits);
        }
        let old = c.resolve(BodySpinCvar::InputBlend).unwrap();
        first
            .try_deserialize_startup(Some(
                b"[Config]\r\nkBodySpinInputBlend=7\r\nkShowCoffinInfo=1\r\n",
            ))
            .unwrap();
        assert_eq!(c.float(&old).unwrap().to_bits(), 7.0f32.to_bits());
        assert!(c.debug_enabled(ReckoningDebug::Coffin).unwrap());
        let mut second = InputCvarManager::new();
        second.try_link(&refs).unwrap();
        assert_eq!(c.float(&old).unwrap().to_bits(), 7.0f32.to_bits());
        let fresh = c.resolve(BodySpinCvar::InputBlend).unwrap();
        assert_eq!(c.float(&fresh).unwrap().to_bits(), 0x3e4c_cccd);
        for debug in [
            ReckoningDebug::Reckoning,
            ReckoningDebug::Coffin,
            ReckoningDebug::BodyTilt,
        ] {
            assert!(!c.debug_enabled(debug).unwrap());
        }
    }
}
