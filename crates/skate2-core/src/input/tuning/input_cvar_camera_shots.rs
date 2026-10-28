//! Actual shot database live-update reference, initialized false.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    pub(crate) fn try_camera_shot_database_live_changes(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(319)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn live_database_flag_requires_real_link_and_keeps_native_default() {
        let references = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            references.try_camera_shot_database_live_changes(),
            Err(InputCvarError::Unlinked("kbShotDatabaseLiveChanges"))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&references).unwrap();
        assert!(!references.try_camera_shot_database_live_changes().unwrap());
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkbShotDatabaseLiveChanges=1\r\n"))
            .unwrap();
        assert!(references.try_camera_shot_database_live_changes().unwrap());
    }
}
