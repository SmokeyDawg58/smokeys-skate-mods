//! Wallride debug target capture and its later live value read.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    simulation_assembly_backend::constraint_session::ground_wallride::source::WallRideDebug,
};

#[derive(Debug)]
pub(crate) enum WallRideDebugError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for WallRideDebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => error.fmt(f),
            Self::Overlay(error) => error.fmt(f),
        }
    }
}
pub(crate) struct WallRideCvars<'a> {
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
pub(crate) struct WallRideTarget(Target);
impl WallRideDebug for WallRideCvars<'_> {
    type Error = WallRideDebugError;
    type Target = WallRideTarget;
    fn capture(&mut self) -> Result<Self::Target, Self::Error> {
        self.references
            .targets
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?[117]
            .clone()
            .map(WallRideTarget)
            .ok_or(Self::Error::Cvar(InputCvarError::Unlinked(
                DEFINITIONS[117].label,
            )))
    }
    fn load(&mut self, target: &Self::Target) -> Result<bool, Self::Error> {
        match *target
            .0
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?
        {
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
