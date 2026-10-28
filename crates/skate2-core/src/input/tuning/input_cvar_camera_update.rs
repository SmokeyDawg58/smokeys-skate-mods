//! Live references sampled by the actual camera update/drop prefix.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    /// Native Update82e48a40..50 compares both operands as unsigned words.
    pub(crate) fn try_camera_not_ready_duration(&self) -> Result<u32, InputCvarError> {
        self.try_current_integer(1245).map(|v| u32::from_ne_bytes(v.to_ne_bytes()))
    }
    pub(crate) fn try_camera_drop_test_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(316)
    }
    pub(crate) fn try_camera_show_mirroring(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(317)
    }
    pub(crate) fn try_camera_show_turning(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(318)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn native_defaults_and_live_values_are_bound_to_actual_references() {
        let references = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            references.try_camera_drop_test_radius(),
            Err(InputCvarError::Unlinked("kbDropTestRadius"))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&references).unwrap();
        assert_eq!(
            references.try_camera_drop_test_radius().unwrap().to_bits(),
            0x3dcc_cccd
        );
        assert!(!references.try_camera_show_mirroring().unwrap());
        assert!(!references.try_camera_show_turning().unwrap());
        manager
            .try_deserialize_startup(Some(
                b"[Config]\r\nkbDropTestRadius=-0\r\nkbShowMirroring=1\r\nkbShowTurning=1\r\n",
            ))
            .unwrap();
        assert_eq!(
            references.try_camera_drop_test_radius().unwrap().to_bits(),
            (-0.0_f32).to_bits()
        );
        assert!(references.try_camera_show_mirroring().unwrap());
        assert!(references.try_camera_show_turning().unwrap());
    }
}
