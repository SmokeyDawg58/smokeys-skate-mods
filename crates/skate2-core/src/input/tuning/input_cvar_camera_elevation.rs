//! Live native avoidance-elevation enable; no constructor or fallback value.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    pub(crate) fn try_camera_avoidance_elevation_enabled(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(321)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn real_avoidance_reference_retains_false_then_live_enable() {
        let references = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            references.try_camera_avoidance_elevation_enabled(),
            Err(InputCvarError::Unlinked("kbAvoidanceElevationEnabled"))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&references).unwrap();
        assert!(!references.try_camera_avoidance_elevation_enabled().unwrap());
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkbAvoidanceElevationEnabled=1\r\n"))
            .unwrap();
        assert!(references.try_camera_avoidance_elevation_enabled().unwrap());
    }
}
