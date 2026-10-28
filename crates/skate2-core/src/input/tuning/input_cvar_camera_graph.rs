//! Live native camera condition and shot-selection references.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    /// `ChooseShot.Begin82e3b008` reads this only with a nonnull `CameraMan` context.
    pub(crate) fn try_camera_lock_shot(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(314)
    }
    /// `UnderThreat82e34788` checks the cvar before actual `ChaseMan.ThreatIsOn`.
    pub(crate) fn try_camera_npc_threat_test(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(315)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn actual_targets_are_unlinked_until_link_and_remain_live() {
        let references = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            references.try_camera_lock_shot(),
            Err(InputCvarError::Unlinked("kbLockShot"))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&references).unwrap();
        assert!(!references.try_camera_lock_shot().unwrap());
        assert!(!references.try_camera_npc_threat_test().unwrap());
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkbLockShot=1\r\nkbNPCThreatTest=1\r\n"))
            .unwrap();
        assert!(references.try_camera_lock_shot().unwrap());
        assert!(references.try_camera_npc_threat_test().unwrap());
    }
}
