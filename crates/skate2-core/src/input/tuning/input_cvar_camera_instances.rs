//! Live camera instance, grind detector and incline debug references.
use super::{InputCvarError, InputCvarReferences};

impl InputCvarReferences {
    /// `UpdateCameraManInstances82e36270` reads this integer as an unsigned count.
    pub(crate) fn try_camera_instance_count(&self) -> Result<u32, InputCvarError> {
        self.try_current_integer(305)
            .map(|value| u32::from_ne_bytes(value.to_ne_bytes()))
    }
    /// `PreUpdate82e453f8` reads this before checking the instance index.
    pub(crate) fn try_camera_grind_detector(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(306)
    }
    /// `UpdateInclineAngle82e41420` samples this after its angle publications.
    pub(crate) fn try_camera_show_frontside(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(307)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn actual_defaults_and_live_updates_are_retained() {
        let refs = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            refs.try_camera_instance_count(),
            Err(InputCvarError::Unlinked("kNumSimCameras"))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert_eq!(refs.try_camera_instance_count().unwrap(), 1);
        assert!(refs.try_camera_grind_detector().unwrap());
        assert!(!refs.try_camera_show_frontside().unwrap());
        manager.try_deserialize_startup(Some(b"[Config]\r\nkNumSimCameras=-1\r\nkbEnableGrindDetector=0\r\nkbShowFrontsideAngle=1\r\n")).unwrap();
        assert_eq!(refs.try_camera_instance_count().unwrap(), u32::MAX);
        assert!(!refs.try_camera_grind_detector().unwrap());
        assert!(refs.try_camera_show_frontside().unwrap());
    }
}
