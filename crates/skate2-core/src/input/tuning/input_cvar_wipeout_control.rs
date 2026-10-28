//! Live control targets, native initializers 0x82f8fa80 and 0x82f8fd70..0x82f8ffb0.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use std::sync::Arc;
#[derive(Clone, Copy)]
pub(crate) enum WipeoutControlFloat {
    MaxAngleY,
    MaxAngleX,
    TorqueForce,
    MaxTorque,
    AutoZ,
    AutoY,
    MinimumControl,
}
pub(crate) struct WipeoutControlTarget(Target);
impl WipeoutControlTarget {
    pub(crate) fn read(&self) -> Result<f32, InputCvarError> {
        match *self.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
impl InputCvarReferences {
    pub(crate) fn try_wipeout_debug_display(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(258)
    }
    pub(crate) fn try_capture_wipeout_control_float(
        &self,
        field: WipeoutControlFloat,
    ) -> Result<WipeoutControlTarget, InputCvarError> {
        let index = match field {
            WipeoutControlFloat::MaxAngleY => 259,
            WipeoutControlFloat::MaxAngleX => 260,
            WipeoutControlFloat::TorqueForce => 261,
            WipeoutControlFloat::MaxTorque => 262,
            WipeoutControlFloat::AutoZ => 263,
            WipeoutControlFloat::AutoY => 264,
            WipeoutControlFloat::MinimumControl => 265,
        };
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(WipeoutControlTarget(Arc::clone(
            targets[index]
                .as_ref()
                .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::WipeoutControlFloat as Float;
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn control_fields_map_to_their_native_defaults_and_live_targets() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        for (field, initial, label) in [
            (Float::MaxAngleY, 8.0_f32, "kWipeControlMaxAngleRotY"),
            (Float::MaxAngleX, 8.0, "kWipeControlMaxAngleRotX"),
            (
                Float::TorqueForce,
                f32::from_bits(0x4019_999a),
                "kWipeoutTorqueForce",
            ),
            (Float::MaxTorque, 0.5, "kWipeControlMaxTorque"),
            (Float::AutoZ, 0.5, "kWipeAutoZAxisScalar"),
            (Float::AutoY, 0.5, "kWipeAutoYAxisScalar"),
            (Float::MinimumControl, 0.5, "kMinControlToStopAutoTwist"),
        ] {
            let target = refs.try_capture_wipeout_control_float(field).unwrap();
            assert_eq!(target.read().unwrap().to_bits(), initial.to_bits());
            let ini = format!("[Config]\r\n{label}=-0\r\n");
            manager
                .try_deserialize_startup(Some(ini.as_bytes()))
                .unwrap();
            assert_eq!(target.read().unwrap().to_bits(), (-0.0_f32).to_bits());
        }
    }

    #[test]
    fn captured_control_target_preserves_identity_and_reads_live_value() {
        let refs = InputCvarReferences::new_static_initialized();
        assert!(
            refs.try_capture_wipeout_control_float(Float::TorqueForce)
                .is_err()
        );
        let mut first = InputCvarManager::new();
        first.try_link(&refs).unwrap();
        let old = refs
            .try_capture_wipeout_control_float(Float::TorqueForce)
            .unwrap();
        first
            .try_deserialize_startup(Some(b"[Config]\r\nkWipeoutTorqueForce=3\r\n"))
            .unwrap();
        assert_eq!(old.read().unwrap().to_bits(), 3.0_f32.to_bits());
        let mut second = InputCvarManager::new();
        second.try_link(&refs).unwrap();
        assert_eq!(old.read().unwrap().to_bits(), 3.0_f32.to_bits());
        assert_eq!(
            refs.try_capture_wipeout_control_float(Float::TorqueForce)
                .unwrap()
                .read()
                .unwrap()
                .to_bits(),
            0x4019_999a
        );
        assert!(!refs.try_wipeout_debug_display().unwrap());
    }
}

#[derive(Clone, Copy)]
pub(crate) enum WipeoutDragFloat {
    Start,
    FadeTime,
    Maximum,
}
impl InputCvarReferences {
    pub(crate) fn try_wipeout_camera_root_blend(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(269)
    }
    pub(crate) fn try_wipeout_height_offset(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(270)
    }
    pub(crate) fn try_capture_wipeout_drag_float(
        &self,
        field: WipeoutDragFloat,
    ) -> Result<WipeoutControlTarget, InputCvarError> {
        let index = match field {
            WipeoutDragFloat::Start => 271,
            WipeoutDragFloat::FadeTime => 272,
            WipeoutDragFloat::Maximum => 273,
        };
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(WipeoutControlTarget(Arc::clone(
            targets[index]
                .as_ref()
                .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?,
        )))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WipeoutRecoveryFloat {
    MaximumAi,
    MinimumUser,
    MinimumImpaled,
}
impl InputCvarReferences {
    pub(crate) fn try_wipeout_recovery_float(
        &self,
        field: WipeoutRecoveryFloat,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(match field {
            WipeoutRecoveryFloat::MaximumAi => 274,
            WipeoutRecoveryFloat::MinimumUser => 275,
            WipeoutRecoveryFloat::MinimumImpaled => 276,
        })
    }
    pub(crate) fn try_max_wipeout_impulse_frames(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(277)
    }
}
#[cfg(test)]
mod recovery_tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn tail_defaults_and_live_values_use_all_four_current_targets() {
        let refs = InputCvarReferences::new_static_initialized();
        assert!(refs.try_max_wipeout_impulse_frames().is_err());
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        for (field, bits) in [
            (WipeoutRecoveryFloat::MaximumAi, 8.0_f32.to_bits()),
            (WipeoutRecoveryFloat::MinimumUser, 0x3fcc_cccd),
            (WipeoutRecoveryFloat::MinimumImpaled, 0x3f00_0000),
        ] {
            assert_eq!(
                refs.try_wipeout_recovery_float(field).unwrap().to_bits(),
                bits
            );
        }
        assert_eq!(refs.try_max_wipeout_impulse_frames().unwrap(), 21);
        manager.try_deserialize_startup(Some(b"[Config]\r\nkMaxAISkaterBailDurationInSeconds=7\r\nkMinUserWipeoutRecoverSec=-0\r\nkMinTimeToBeImpaledForTeleport=3\r\nkMaxWipeoutImpulseFrames=17\r\n")).unwrap();
        assert_eq!(
            refs.try_wipeout_recovery_float(WipeoutRecoveryFloat::MaximumAi)
                .unwrap()
                .to_bits(),
            7.0_f32.to_bits()
        );
        assert_eq!(
            refs.try_wipeout_recovery_float(WipeoutRecoveryFloat::MinimumUser)
                .unwrap()
                .to_bits(),
            0x8000_0000
        );
        assert_eq!(
            refs.try_wipeout_recovery_float(WipeoutRecoveryFloat::MinimumImpaled)
                .unwrap()
                .to_bits(),
            3.0_f32.to_bits()
        );
        assert_eq!(refs.try_max_wipeout_impulse_frames().unwrap(), 17);
    }
}

impl InputCvarReferences {
    /// `WipeoutGround.UpdatePostPhysics`82d8f738 samples the live target/value.
    pub(crate) fn try_wipeout_slow_speed(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(278)
    }
}
#[cfg(test)]
mod post_tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn post_speed_reads_default_then_live_value() {
        let refs = InputCvarReferences::new_static_initialized();
        assert!(refs.try_wipeout_slow_speed().is_err());
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert_eq!(
            refs.try_wipeout_slow_speed().unwrap().to_bits(),
            1.0_f32.to_bits()
        );
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkWipeout_SlowSpeed=-0\r\n"))
            .unwrap();
        assert_eq!(
            refs.try_wipeout_slow_speed().unwrap().to_bits(),
            0x8000_0000
        );
    }
}
