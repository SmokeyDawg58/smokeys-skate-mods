//! `PrepareToolkit` target capture and later value reads, 82b26ccc..26d84.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::debug_overlay::{DebugOverlayError, DebugOverlaySlot};
use crate::skeleton_processed_contact::toolkit::{
    SkeletonProcessedToolkitJumpCvars, ToolkitJumpFloat,
};

pub(crate) struct ToolkitFloatTarget(Target);
pub(crate) struct ToolkitBoolTarget(Target);
pub(crate) struct ToolkitCvars<'a> {
    references: &'a InputCvarReferences,
    overlay: DebugOverlaySlot,
}
impl<'a> ToolkitCvars<'a> {
    pub(crate) const fn new(
        references: &'a InputCvarReferences,
        overlay: DebugOverlaySlot,
    ) -> Self {
        Self {
            references,
            overlay,
        }
    }
    fn capture(&self, index: usize) -> Result<Target, ToolkitCvarError> {
        self.references
            .targets
            .lock()
            .map_err(|_| ToolkitCvarError::Cvar(InputCvarError::Lock))?[index]
            .clone()
            .ok_or(ToolkitCvarError::Cvar(InputCvarError::Unlinked(
                DEFINITIONS[index].label,
            )))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ToolkitCvarError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for ToolkitCvarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => write!(f, "toolkit cvar: {error:?}"),
            Self::Overlay(error) => write!(f, "toolkit debug overlay: {error}"),
        }
    }
}

impl SkeletonProcessedToolkitJumpCvars for ToolkitCvars<'_> {
    type FloatTarget = ToolkitFloatTarget;
    type BoolTarget = ToolkitBoolTarget;
    type Error = ToolkitCvarError;
    fn capture_float(&mut self, field: ToolkitJumpFloat) -> Result<Self::FloatTarget, Self::Error> {
        self.capture(match field {
            ToolkitJumpFloat::MaxCosAngleDelta => 54,
            ToolkitJumpFloat::ProportionalBlend => 55,
        })
        .map(ToolkitFloatTarget)
    }
    fn float(&mut self, target: &Self::FloatTarget) -> Result<f32, Self::Error> {
        match *target
            .0
            .lock()
            .map_err(|_| ToolkitCvarError::Cvar(InputCvarError::Lock))?
        {
            Value::Float(value) => Ok(value),
            _ => Err(ToolkitCvarError::Cvar(InputCvarError::WrongType)),
        }
    }
    fn capture_draw_jump(&mut self) -> Result<Self::BoolTarget, Self::Error> {
        self.capture(56).map(ToolkitBoolTarget)
    }
    fn draw_jump(&mut self, target: &Self::BoolTarget) -> Result<bool, Self::Error> {
        match *target
            .0
            .lock()
            .map_err(|_| ToolkitCvarError::Cvar(InputCvarError::Lock))?
        {
            Value::Boolean(value) => Ok(value),
            _ => Err(ToolkitCvarError::Cvar(InputCvarError::WrongType)),
        }
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        // 26d94 reads the CURRENT singleton after the board-normal store.
        // A present instance branches directly to final copy26ddc; no draw call.
        self.overlay
            .require()
            .map(|_| ())
            .map_err(ToolkitCvarError::Overlay)
    }
}

#[cfg(test)]
mod tests {
    use super::{ToolkitCvarError, ToolkitCvars};
    use crate::{
        debug_overlay::{DebugOverlayError, DebugOverlaySlot},
        input_cvar_runtime::{InputCvarError, InputCvarManager, InputCvarReferences},
        skeleton_processed_contact::toolkit::{
            SkeletonProcessedToolkitJumpCvars, ToolkitJumpFloat,
        },
    };

    #[test]
    fn capture_keeps_old_target_but_reads_its_later_live_value() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut adapter = ToolkitCvars::new(&refs, DebugOverlaySlot::default());
        assert!(matches!(
            adapter.capture_float(ToolkitJumpFloat::MaxCosAngleDelta),
            Err(ToolkitCvarError::Cvar(InputCvarError::Unlinked(
                "kGroundMaxCosAngleDelta"
            )))
        ));
        let mut old = InputCvarManager::new();
        old.try_link(&refs).unwrap();
        let max = adapter
            .capture_float(ToolkitJumpFloat::MaxCosAngleDelta)
            .unwrap();
        let blend = adapter
            .capture_float(ToolkitJumpFloat::ProportionalBlend)
            .unwrap();
        let draw = adapter.capture_draw_jump().unwrap();
        assert_eq!(adapter.float(&max).unwrap().to_bits(), 0x3f6f_5c29);
        assert_eq!(adapter.float(&blend).unwrap().to_bits(), 0x3dcc_cccd);
        assert!(!adapter.draw_jump(&draw).unwrap());
        old.try_deserialize_startup(Some(b"[Config]\r\nkGroundMaxCosAngleDelta=-2\r\nkGroundProportionalBlend=3\r\nkDrawJump=1\r\n")).unwrap();
        let mut replacement = InputCvarManager::new();
        replacement.try_link(&refs).unwrap();
        assert_eq!(adapter.float(&max).unwrap().to_bits(), (-2.0_f32).to_bits());
        assert_eq!(adapter.float(&blend).unwrap().to_bits(), 3.0_f32.to_bits());
        assert!(adapter.draw_jump(&draw).unwrap());
        let new_max = adapter
            .capture_float(ToolkitJumpFloat::MaxCosAngleDelta)
            .unwrap();
        assert_eq!(adapter.float(&new_max).unwrap().to_bits(), 0x3f6f_5c29);
        assert_eq!(
            adapter.require_overlay(),
            Err(ToolkitCvarError::Overlay(DebugOverlayError::Missing))
        );
    }

    #[test]
    fn enabled_debug_reads_current_shared_holder_after_adapter_construction() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let slot = DebugOverlaySlot::default();
        // Construction must neither require nor create the singleton.
        let mut adapter = ToolkitCvars::new(&refs, slot.clone());
        let draw = adapter.capture_draw_jump().unwrap();
        assert!(!adapter.draw_jump(&draw).unwrap());
        assert!(matches!(slot.require(), Err(DebugOverlayError::Missing)));
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkDrawJump=1\r\n"))
            .unwrap();
        assert!(adapter.draw_jump(&draw).unwrap());
        assert_eq!(
            adapter.require_overlay(),
            Err(ToolkitCvarError::Overlay(DebugOverlayError::Missing))
        );
        slot.try_construct().unwrap();
        let actual = slot.require().unwrap();
        assert_eq!(adapter.require_overlay(), Ok(()));
        assert!(std::sync::Arc::ptr_eq(&actual, &slot.require().unwrap()));
    }
}
