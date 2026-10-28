//! Live linked cvars read by HoldSkateboard and its Body/hand-drive leaves.

use super::{InputCvarError, InputCvarReferences};

impl InputCvarReferences {
    /// `Skateboard::SetFree` 0x82b307b8, reference 0x8308dd10.
    pub(crate) fn try_free_skateboard_angular_damping(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(388)
    }

    /// Fresh linked target read at each EnableHandDrive/UpdateDriveFrames gate.
    pub(crate) fn try_skc_use_hand_drives(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(6)
    }
    /// `HoldSkateboard` 0x82dcc75c, reference 0x830dbb5c.
    pub(crate) fn try_skc_use_soft_hook(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(363)
    }

    /// `EnableHandDrive` 0x82dcc6bc/0x82dcc6f0, reference 0x830db3a0.
    pub(crate) fn try_skc_hand_drive_strength(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(362)
    }

    /// `EnableHandDrive` 0x82dcc6c0/0x82dcc6f8, reference 0x830db92c.
    pub(crate) fn try_skc_hand_drive_displacement(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(361)
    }

    /// `HookEnableLinearDriveSoft` 0x82b24c14, reference 0x8308dadc.
    pub(crate) fn try_skateboard_linear_soft_drive_disp(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(359)
    }

    /// `HookEnableLinearDriveSoft` 0x82b24c40, reference 0x8308de30.
    pub(crate) fn try_skateboard_linear_soft_drive_strength(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(360)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;

    #[test]
    fn hold_cvars_read_current_linked_targets_and_relink() {
        let refs = InputCvarReferences::new_static_initialized();
        assert_eq!(
            refs.try_skc_use_soft_hook(),
            Err(InputCvarError::Unlinked("kSKC_UseSoftHook"))
        );
        assert_eq!(
            refs.try_free_skateboard_angular_damping(),
            Err(InputCvarError::Unlinked("kFreeSkateboardAngularDamping"))
        );
        let mut first = InputCvarManager::new();
        first.try_link(&refs).unwrap();
        assert!(refs.try_skc_use_soft_hook().unwrap());
        assert_eq!(refs.try_skc_hand_drive_strength().unwrap(), 5.0);
        assert_eq!(refs.try_free_skateboard_angular_damping().unwrap(), 0.25);
        assert_eq!(refs.try_skc_hand_drive_displacement().unwrap(), 5.0);
        assert_eq!(
            refs.try_skateboard_linear_soft_drive_disp().unwrap(),
            5000.0
        );
        assert_eq!(
            refs.try_skateboard_linear_soft_drive_strength().unwrap(),
            0.5
        );
        first.try_deserialize_startup(Some(b"[Config]\r\nkSKC_UseSoftHook=0\r\nkSKC_HandDriveStr=12\r\nkSKC_HandDriveDisp=-2\r\nkSkateboardLinSoftDriveDisp=17\r\nkSkateboardLinSoftDriveStr=3\r\nkFreeSkateboardAngularDamping=0.5\r\n")).unwrap();
        assert!(!refs.try_skc_use_soft_hook().unwrap());
        assert_eq!(refs.try_skc_hand_drive_strength().unwrap(), 12.0);
        assert_eq!(refs.try_skc_hand_drive_displacement().unwrap(), -2.0);
        assert_eq!(refs.try_free_skateboard_angular_damping().unwrap(), 0.5);
        assert_eq!(refs.try_skateboard_linear_soft_drive_disp().unwrap(), 17.0);
        assert_eq!(
            refs.try_skateboard_linear_soft_drive_strength().unwrap(),
            3.0
        );
        let mut second = InputCvarManager::new();
        second.try_link(&refs).unwrap();
        assert!(refs.try_skc_use_soft_hook().unwrap());
        assert_eq!(refs.try_skc_hand_drive_strength().unwrap(), 5.0);
        assert_eq!(refs.try_free_skateboard_angular_damping().unwrap(), 0.25);
        assert_eq!(
            refs.try_skateboard_linear_soft_drive_disp().unwrap(),
            5000.0
        );
    }
}
