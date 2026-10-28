//! Unregistered live captured refs120..123 in source static-table order.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    skateboard_force_runtime::entry::speed_model_update::SpeedCvar,
    skeleton_processed_contact::speed_model_source::SpeedModelDebugRead,
};
#[derive(Debug)]
pub(crate) enum SpeedCvarError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for SpeedCvarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
        }
    }
}
pub(crate) struct SpeedModelCvars<'a> {
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
pub(crate) struct SpeedTarget(Target);
impl SpeedModelDebugRead for SpeedModelCvars<'_> {
    type Error = SpeedCvarError;
    type Target = SpeedTarget;
    fn capture(&mut self, c: SpeedCvar) -> Result<Self::Target, Self::Error> {
        let i = match c {
            SpeedCvar::CheatSpeed => 120,
            SpeedCvar::CheatEnabled => 121,
            SpeedCvar::ManualFrames => 122,
            SpeedCvar::Debug => 123,
        };
        self.references
            .targets
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?[i]
            .clone()
            .map(SpeedTarget)
            .ok_or(Self::Error::Cvar(InputCvarError::Unlinked(
                DEFINITIONS[i].label,
            )))
    }
    fn integer(&mut self, t: &Self::Target) -> Result<i32, Self::Error> {
        match *t
            .0
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?
        {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(Self::Error::Cvar(InputCvarError::WrongType)),
        }
    }
    fn boolean(&mut self, t: &Self::Target) -> Result<bool, Self::Error> {
        match *t
            .0
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?
        {
            Value::Boolean(v) => Ok(v),
            _ => Err(Self::Error::Cvar(InputCvarError::WrongType)),
        }
    }
    fn float(&mut self, t: &Self::Target) -> Result<f32, Self::Error> {
        match *t
            .0
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?
        {
            Value::Float(v) => Ok(v),
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
