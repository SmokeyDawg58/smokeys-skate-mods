//! Proposed refs118/119; registration remains held for the parent gate.
use super::{InputCvarError, InputCvarReferences, Value};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    simulation_assembly_backend::constraint_session::pumping_force_source::PumpingForceDebug,
};
#[derive(Debug)]
pub(crate) enum PumpingForceDebugError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for PumpingForceDebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => error.fmt(f),
            Self::Overlay(error) => error.fmt(f),
        }
    }
}
pub(crate) struct PumpingForceCvars<'a> {
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
impl PumpingForceCvars<'_> {
    fn boolean(&self, index: usize, label: &'static str) -> Result<bool, PumpingForceDebugError> {
        let target = self
            .references
            .targets
            .lock()
            .map_err(|_| PumpingForceDebugError::Cvar(InputCvarError::Lock))?[index]
            .clone()
            .ok_or(PumpingForceDebugError::Cvar(InputCvarError::Unlinked(
                label,
            )))?;
        match *target
            .lock()
            .map_err(|_| PumpingForceDebugError::Cvar(InputCvarError::Lock))?
        {
            Value::Boolean(value) => Ok(value),
            _ => Err(PumpingForceDebugError::Cvar(InputCvarError::WrongType)),
        }
    }
}
impl PumpingForceDebug for PumpingForceCvars<'_> {
    type Error = PumpingForceDebugError;
    fn draw_absorption(&mut self) -> Result<bool, Self::Error> {
        self.boolean(119, "kDrawAbsorptionCalc")
    }
    fn draw_intention(&mut self) -> Result<bool, Self::Error> {
        self.boolean(118, "kDrawPumpForceIntention")
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}
