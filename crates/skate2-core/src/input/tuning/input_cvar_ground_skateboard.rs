//! Live `GroundSkateboard` IK/drive settings; native table4628..4698.
//! Unregistered draft pending the coordinated shared reader/service split.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralCvars,
    skeleton_drives::update::SkeletonDrivesUpdateCvars,
    skeleton_ik::SkeletonIkSuffixServices,
    skeleton_ik_prephysics::SkeletonIkPrePhysicsServices,
    skeleton_ik_state::{
        contact::{
            SkeletonIkContactLineQuery, SkeletonIkContactLineResult, SkeletonIkContactServices,
        },
        drive_frames::SkeletonIkDriveFramesServices,
    },
};
#[derive(Debug)]
pub(crate) enum GroundSkateboardCvarError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    MissingCapturedSidewaysDebug,
    WorldQueryRequired,
}
impl std::fmt::Display for GroundSkateboardCvarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(e) => write!(f, "GroundSkateboard cvar: {e}"),
            Self::Overlay(e) => write!(f, "GroundSkateboard overlay: {e}"),
            Self::MissingCapturedSidewaysDebug => {
                f.write_str("sideways debug target was not captured")
            }
            Self::WorldQueryRequired => {
                f.write_str("IK query requires the same retained World query adapter")
            }
        }
    }
}
pub(crate) struct GroundSkateboardCvars<'a> {
    references: &'a InputCvarReferences,
    overlay: DebugOverlaySlot,
    sideways_debug: Option<Target>,
}
impl<'a> GroundSkateboardCvars<'a> {
    pub(crate) const fn new(
        references: &'a InputCvarReferences,
        overlay: DebugOverlaySlot,
    ) -> Self {
        Self {
            references,
            overlay,
            sideways_debug: None,
        }
    }
    fn float(&self, index: usize) -> Result<f32, GroundSkateboardCvarError> {
        self.references
            .try_current_float(index)
            .map_err(GroundSkateboardCvarError::Cvar)
    }
    fn boolean(&self, index: usize) -> Result<bool, GroundSkateboardCvarError> {
        self.references
            .try_current_boolean(index)
            .map_err(GroundSkateboardCvarError::Cvar)
    }
    fn overlay(&self) -> Result<(), GroundSkateboardCvarError> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(GroundSkateboardCvarError::Overlay)
    }
}
impl SkeletonDrivesUpdateCvars for GroundSkateboardCvars<'_> {
    type Error = GroundSkateboardCvarError;
    fn offboard_drive_scalar(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(93)
    }
    fn soft_damping(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(94)
    }
}
impl SkeletonGeneralCvars for GroundSkateboardCvars<'_> {
    fn ik_draw_feet_on_deck(&mut self) -> Result<u8, GroundSkateboardCvarError> {
        self.boolean(96).map(u8::from)
    }
    fn ik_maximum_external_normal_angle(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(102)
    }
    fn ik_target_normal_blend_speed(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(98)
    }
    fn ik_maximum_local_delta(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(101)
    }
    fn ik_maximum_world_offset(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(100)
    }
    fn ik_require_debug_overlay(&mut self) -> Result<(), GroundSkateboardCvarError> {
        self.overlay()
    }
}
impl SkeletonIkPrePhysicsServices for GroundSkateboardCvars<'_> {
    type Error = GroundSkateboardCvarError;
    fn draw_pre_feet_ik(&mut self) -> Result<bool, GroundSkateboardCvarError> {
        self.boolean(95)
    }
    fn require_debug_overlay(&mut self) -> Result<(), GroundSkateboardCvarError> {
        self.overlay()
    }
}
impl SkeletonIkDriveFramesServices for GroundSkateboardCvars<'_> {
    fn enable_prephysics(&mut self) -> Result<bool, GroundSkateboardCvarError> {
        self.boolean(105)
    }
    fn maximum_leg_length(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(106)
    }
    fn maximum_arm_length(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(107)
    }
}
impl SkeletonIkSuffixServices for GroundSkateboardCvars<'_> {
    fn enable_contact_offset(&mut self) -> Result<bool, GroundSkateboardCvarError> {
        self.boolean(99)
    }
}
impl SkeletonIkContactServices for GroundSkateboardCvars<'_> {
    type Error = GroundSkateboardCvarError;
    fn draw_ik_contact_debug(&mut self) -> Result<bool, GroundSkateboardCvarError> {
        self.boolean(103)
    }
    fn max_ik_contact_penetration_offset(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(108)
    }
    fn sideways_offset(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(109)
    }
    fn max_ik_contact_offset(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(104)
    }
    fn toe_contact_threshold(&mut self) -> Result<f32, GroundSkateboardCvarError> {
        self.float(97)
    }
    fn require_debug_overlay(&mut self) -> Result<(), GroundSkateboardCvarError> {
        self.overlay()
    }
    fn capture_sideways_offset_draw(&mut self) -> Result<(), Self::Error> {
        self.sideways_debug = Some(
            self.references
                .targets
                .lock()
                .map_err(|_| GroundSkateboardCvarError::Cvar(InputCvarError::Lock))?[110]
                .clone()
                .ok_or(GroundSkateboardCvarError::Cvar(InputCvarError::Unlinked(
                    DEFINITIONS[110].label,
                )))?,
        );
        Ok(())
    }
    fn sideways_offset_draw(&mut self) -> Result<bool, Self::Error> {
        let target = self
            .sideways_debug
            .as_ref()
            .ok_or(Self::Error::MissingCapturedSidewaysDebug)?;
        match *target
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?
        {
            Value::Boolean(v) => Ok(v),
            _ => Err(Self::Error::Cvar(InputCvarError::WrongType)),
        }
    }
    fn try_line_query(
        &mut self,
        _query: &SkeletonIkContactLineQuery,
    ) -> Result<SkeletonIkContactLineResult, Self::Error> {
        Err(Self::Error::WorldQueryRequired)
    }
}

#[cfg(test)]
mod tests {
    use super::{GroundSkateboardCvarError, GroundSkateboardCvars};
    use crate::{
        debug_overlay::DebugOverlaySlot,
        input_cvar_runtime::{InputCvarManager, InputCvarReferences},
        skeleton_drives::update::SkeletonDrivesUpdateCvars,
        skeleton_ik_state::contact::SkeletonIkContactServices,
    };
    #[test]
    fn source_defaults_and_captured_sideways_target_preserve_live_load_timing() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut first = InputCvarManager::new();
        first.try_link(&refs).unwrap();
        let mut c = GroundSkateboardCvars::new(&refs, DebugOverlaySlot::default());
        for (index, bits) in [
            (93, 0x4000_0000),
            (94, 0x4348_0000),
            (97, 0x3f33_3333),
            (98, 0x3e4c_cccd),
            (100, 0x3f33_3333),
            (101, 0x3ccc_cccd),
            (102, 0x3f35_c28f),
            (104, 0x3e99_999a),
            (106, 0x3f4c_cccd),
            (107, 0x3f80_0000),
            (108, 0x3e99_999a),
            (109, 0x3e8f_5c29),
        ] {
            assert_eq!(c.float(index).unwrap().to_bits(), bits);
        }
        for (index, value) in [
            (95, false),
            (96, false),
            (99, true),
            (103, false),
            (105, true),
            (110, false),
        ] {
            assert_eq!(c.boolean(index).unwrap(), value);
        }
        assert!(matches!(
            c.sideways_offset_draw(),
            Err(GroundSkateboardCvarError::MissingCapturedSidewaysDebug)
        ));
        c.capture_sideways_offset_draw().unwrap();
        first
            .try_deserialize_startup(Some(
                b"[Config]\r\nkSidewaysOffsetDraw=1\r\nkOffboardDriveScalar=3\r\n",
            ))
            .unwrap();
        assert!(c.sideways_offset_draw().unwrap());
        assert_eq!(
            c.offboard_drive_scalar().unwrap().to_bits(),
            3.0_f32.to_bits()
        );
        let mut second = InputCvarManager::new();
        second.try_link(&refs).unwrap();
        assert!(c.sideways_offset_draw().unwrap());
        assert_eq!(
            c.offboard_drive_scalar().unwrap().to_bits(),
            2.0_f32.to_bits()
        );
        c.capture_sideways_offset_draw().unwrap();
        assert!(!c.sideways_offset_draw().unwrap());
        assert!(matches!(
            c.require_debug_overlay(),
            Err(GroundSkateboardCvarError::Overlay(_))
        ));
    }
}
