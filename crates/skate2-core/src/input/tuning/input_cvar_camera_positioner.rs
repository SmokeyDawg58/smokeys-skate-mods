//! Actual live references sampled by the camera positioner.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    pub(crate) fn try_camera_near_clip_distance(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(330)
    }
    pub(crate) fn try_camera_max_position_retries(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(331)
    }
    pub(crate) fn try_camera_heavyweight_hard_cut(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(332)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn actual_defaults_and_unclamped_live_retry_values() {
        let refs = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            refs.try_camera_max_position_retries(),
            Err(InputCvarError::Unlinked("kMaxPositionRetries"))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert_eq!(refs.try_camera_max_position_retries().unwrap(), 18);
        assert_eq!(
            refs.try_camera_near_clip_distance().unwrap().to_bits(),
            0x3dcc_cccd
        );
        assert!(!refs.try_camera_heavyweight_hard_cut().unwrap());
        manager.try_deserialize_startup(Some(b"[Config]\r\nkMaxPositionRetries=0\r\nkNearClipPlaneDistance=-0\r\nkbHeavyweightHardCutForAllCameras=1\r\n")).unwrap();
        assert_eq!(refs.try_camera_max_position_retries().unwrap(), 0);
        assert_eq!(
            refs.try_camera_near_clip_distance().unwrap().to_bits(),
            (-0.0_f32).to_bits()
        );
        assert!(refs.try_camera_heavyweight_hard_cut().unwrap());
    }
}
