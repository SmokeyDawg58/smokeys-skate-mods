//! Live flag4 drive settings; initializers82f7d758/82f7d828.
use super::{InputCvarError, InputCvarReferences};
use crate::skeleton_drives::{
    ragdoll_update::RagdollDriveCvars, update::SkeletonDrivesUpdateCvars,
};

pub(crate) struct RagdollDriveSettings<'a>(pub &'a InputCvarReferences);
impl SkeletonDrivesUpdateCvars for RagdollDriveSettings<'_> {
    type Error = InputCvarError;
    fn offboard_drive_scalar(&mut self) -> Result<f32, Self::Error> {
        self.0.try_current_float(93)
    }
    fn soft_damping(&mut self) -> Result<f32, Self::Error> {
        self.0.try_current_float(94)
    }
}
impl RagdollDriveCvars for RagdollDriveSettings<'_> {
    fn follow_animation(&mut self) -> Result<bool, Self::Error> {
        self.0.try_current_boolean(257)
    }
    fn rigid_ragdoll(&mut self) -> Result<bool, Self::Error> {
        self.0.try_current_boolean(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn ragdoll_settings_read_current_targets_and_preserve_defaults() {
        let references = InputCvarReferences::new_static_initialized();
        let mut settings = RagdollDriveSettings(&references);
        assert!(settings.rigid_ragdoll().is_err());
        let mut manager = InputCvarManager::new();
        manager.try_link(&references).unwrap();
        assert!(!settings.rigid_ragdoll().unwrap());
        assert!(!settings.follow_animation().unwrap());
        manager
            .try_deserialize_startup(Some(
                b"[Config]\r\nkRigidRagdoll=1\r\nkFollowAnimInRagdoll=1\r\nkSoftDamping=17\r\n",
            ))
            .unwrap();
        assert!(settings.rigid_ragdoll().unwrap());
        assert!(settings.follow_animation().unwrap());
        assert_eq!(
            settings.soft_damping().unwrap().to_bits(),
            17.0_f32.to_bits()
        );
    }
}
