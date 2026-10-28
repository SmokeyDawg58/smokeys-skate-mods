//! Registered `InputCvar` child: feet capture/read same absorption reference119.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    skateboard_foot_forces::SkateboardFootDebugRead,
};

#[derive(Debug)]
pub(crate) enum GroundFootDebugError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for GroundFootDebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
        }
    }
}
pub(crate) struct GroundFootCvars<'a> {
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
pub(crate) struct GroundFootTarget(Target);
impl SkateboardFootDebugRead for GroundFootCvars<'_> {
    type Target = GroundFootTarget;
    type Error = GroundFootDebugError;
    fn capture_draw_absorption_target(&mut self) -> Result<Self::Target, Self::Error> {
        self.references
            .targets
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?[119]
            .clone()
            .map(GroundFootTarget)
            .ok_or(Self::Error::Cvar(InputCvarError::Unlinked(
                DEFINITIONS[119].label,
            )))
    }
    fn draw_absorption(&mut self, target: &Self::Target) -> Result<bool, Self::Error> {
        let value = *target
            .0
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?;
        match value {
            Value::Boolean(value) => Ok(value),
            _ => Err(Self::Error::Cvar(InputCvarError::WrongType)),
        }
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}

#[cfg(test)]
mod tests {
    use super::{GroundFootCvars, GroundFootDebugError};
    use crate::{
        debug_overlay::{DebugOverlayError, DebugOverlaySlot},
        input_cvar_runtime::{InputCvarError, InputCvarManager, InputCvarReferences},
        skateboard_foot_forces::SkateboardFootDebugRead,
    };

    #[test]
    fn captured_absorption_target_survives_relink_and_overlay_is_live() {
        let references = InputCvarReferences::new_static_initialized();
        let overlay = DebugOverlaySlot::default();
        let mut reader = GroundFootCvars {
            references: &references,
            overlay: &overlay,
        };
        assert!(matches!(
            reader.capture_draw_absorption_target(),
            Err(GroundFootDebugError::Cvar(InputCvarError::Unlinked(
                "kDrawAbsorptionCalc"
            )))
        ));
        let mut original = InputCvarManager::new();
        original.try_link(&references).unwrap();
        let captured = reader.capture_draw_absorption_target().unwrap();
        assert!(!reader.draw_absorption(&captured).unwrap());
        original
            .try_deserialize_startup(Some(b"[Config]\r\nkDrawAbsorptionCalc=1\r\n"))
            .unwrap();
        let mut replacement = InputCvarManager::new();
        replacement.try_link(&references).unwrap();
        assert!(reader.draw_absorption(&captured).unwrap());
        let fresh = reader.capture_draw_absorption_target().unwrap();
        assert!(!reader.draw_absorption(&fresh).unwrap());
        assert!(matches!(
            reader.require_overlay(),
            Err(GroundFootDebugError::Overlay(DebugOverlayError::Missing))
        ));
        overlay.try_construct().unwrap();
        reader.require_overlay().unwrap();
    }
}
