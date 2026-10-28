//! Actual late `UpdateAnchor` debug flag, separate from standing-offset input.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    pub(crate) fn try_camera_show_anchor_debug(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(320)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn late_anchor_debug_uses_real_link_and_live_boolean() {
        let references = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            references.try_camera_show_anchor_debug(),
            Err(InputCvarError::Unlinked("kbShowAnchorDebug"))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&references).unwrap();
        assert!(!references.try_camera_show_anchor_debug().unwrap());
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkbShowAnchorDebug=1\r\n"))
            .unwrap();
        assert!(references.try_camera_show_anchor_debug().unwrap());
    }
}
