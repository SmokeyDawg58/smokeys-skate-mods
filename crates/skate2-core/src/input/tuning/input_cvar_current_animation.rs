//! Live `ProcessData` adjustment/record cvars, sampled by their original callees.
use super::{InputCvarError, InputCvarReferences};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    skeleton_animation_record::SkeletonAnimationRecordInput,
    skeleton_current_animation::SkeletonCurrentAnimationCvars,
};

pub(crate) struct CurrentAnimationCvars<'a> {
    references: &'a InputCvarReferences,
    overlay: DebugOverlaySlot,
}
impl<'a> CurrentAnimationCvars<'a> {
    pub(crate) const fn new(
        references: &'a InputCvarReferences,
        overlay: DebugOverlaySlot,
    ) -> Self {
        Self {
            references,
            overlay,
        }
    }
    fn boolean(&self, index: usize) -> Result<u8, CurrentAnimationCvarError> {
        self.references
            .try_current_boolean(index)
            .map(u8::from)
            .map_err(CurrentAnimationCvarError::Cvar)
    }
    fn float(&self, index: usize) -> Result<f32, CurrentAnimationCvarError> {
        self.references
            .try_current_float(index)
            .map_err(CurrentAnimationCvarError::Cvar)
    }
    fn require_overlay(&self) -> Result<(), CurrentAnimationCvarError> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(CurrentAnimationCvarError::Overlay)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CurrentAnimationCvarError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for CurrentAnimationCvarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => write!(f, "current animation cvar: {error:?}"),
            Self::Overlay(error) => write!(f, "current animation overlay: {error}"),
        }
    }
}
impl SkeletonAnimationRecordInput for CurrentAnimationCvars<'_> {
    type Error = CurrentAnimationCvarError;
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.boolean(66)
    }
}
impl SkeletonCurrentAnimationCvars for CurrentAnimationCvars<'_> {
    fn debug_landing_adjust(&mut self) -> Result<u8, Self::Error> {
        self.boolean(61)
    }
    fn landing_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.require_overlay()
    }
    fn landing_use_physics_com(&mut self) -> Result<u8, Self::Error> {
        self.boolean(62)
    }
    fn landing_first_frame_hack(&mut self) -> Result<f32, Self::Error> {
        self.float(63)
    }
    fn landing_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.float(60)
    }
    fn offboard_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.float(60)
    }
    fn landing_on_board_show_adjust(&mut self) -> Result<u8, Self::Error> {
        self.boolean(65)
    }
    fn landing_on_board_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.require_overlay()
    }
    fn landing_on_board_negate_y(&mut self) -> Result<u8, Self::Error> {
        self.boolean(64)
    }
    fn landing_on_board_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.float(60)
    }
}

#[cfg(test)]
mod tests {
    use super::{CurrentAnimationCvarError, CurrentAnimationCvars};
    use crate::{
        debug_overlay::{DebugOverlayError, DebugOverlaySlot},
        input_cvar_runtime::{InputCvarManager, InputCvarReferences},
        skeleton_animation_record::SkeletonAnimationRecordInput,
        skeleton_current_animation::SkeletonCurrentAnimationCvars,
    };
    #[test]
    fn live_relink_and_overlay_acquisition_preserve_source_defaults() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let overlay = DebugOverlaySlot::default();
        let mut cvars = CurrentAnimationCvars::new(&refs, overlay.clone());
        assert_eq!(cvars.debug_landing_adjust().unwrap(), 0);
        assert_eq!(cvars.landing_use_physics_com().unwrap(), 0);
        assert_eq!(
            cvars.landing_first_frame_hack().unwrap().to_bits(),
            0x3c75_c28f
        );
        assert_eq!(
            cvars
                .offboard_frames_to_blend_away_offset()
                .unwrap()
                .to_bits(),
            0x4170_0000
        );
        assert_eq!(cvars.landing_on_board_show_adjust().unwrap(), 0);
        assert_eq!(cvars.landing_on_board_negate_y().unwrap(), 1);
        assert_eq!(cvars.use_xyz_centre_of_mass().unwrap(), 1);
        manager.try_deserialize_startup(Some(b"[Config]\r\nkLandingAdjustFramesToBlendAwayOffset=-7\r\nkLandingAdjustFirstFrameHack=2\r\nkDebugLandingAdjust=1\r\nkUseXYZCentreOfMass=0\r\n")).unwrap();
        assert_eq!(
            cvars
                .landing_frames_to_blend_away_offset()
                .unwrap()
                .to_bits(),
            (-7.0_f32).to_bits()
        );
        assert_eq!(
            cvars
                .landing_on_board_frames_to_blend_away_offset()
                .unwrap()
                .to_bits(),
            (-7.0_f32).to_bits()
        );
        assert_eq!(
            cvars.landing_first_frame_hack().unwrap().to_bits(),
            2.0_f32.to_bits()
        );
        assert_eq!(cvars.debug_landing_adjust().unwrap(), 1);
        assert_eq!(cvars.use_xyz_centre_of_mass().unwrap(), 0);
        assert_eq!(
            cvars.landing_require_debug_overlay(),
            Err(CurrentAnimationCvarError::Overlay(
                DebugOverlayError::Missing
            ))
        );
        overlay.try_construct().unwrap();
        assert_eq!(cvars.landing_require_debug_overlay(), Ok(()));
        assert_eq!(cvars.landing_on_board_require_debug_overlay(), Ok(()));
        let mut relinked = InputCvarManager::new();
        relinked.try_link(&refs).unwrap();
        assert_eq!(
            cvars
                .landing_frames_to_blend_away_offset()
                .unwrap()
                .to_bits(),
            0x4170_0000
        );
        assert_eq!(cvars.use_xyz_centre_of_mass().unwrap(), 1);
    }
}
