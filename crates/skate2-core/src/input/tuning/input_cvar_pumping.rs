//! Pumping debug gates82dde784 and82de08d0.
use super::{InputCvarError, InputCvarReferences};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    physical_player_pumping::update::PumpingDebug,
};

#[derive(Debug)]
pub(crate) enum PumpingDebugError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for PumpingDebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => error.fmt(f),
            Self::Overlay(error) => error.fmt(f),
        }
    }
}

pub(crate) struct PumpingCvars<'a> {
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}

impl PumpingDebug for PumpingCvars<'_> {
    type Error = PumpingDebugError;
    fn draw_deck_angle(&mut self, _: f32, _: f32, _: f32, _: f32) -> Result<(), Self::Error> {
        if self
            .references
            .try_current_boolean(111)
            .map_err(PumpingDebugError::Cvar)?
        {
            // Each original debug call separately reacquires the singleton.
            // Folded empty drawing callees do not create a host rendering effect.
            self.overlay.require().map_err(PumpingDebugError::Overlay)?;
            self.overlay.require().map_err(PumpingDebugError::Overlay)?;
        }
        Ok(())
    }
    fn draw_pump(&mut self) -> Result<(), Self::Error> {
        if self
            .references
            .try_current_boolean(112)
            .map_err(PumpingDebugError::Cvar)?
        {
            // Update08e0 captures once and reuses it for four empty draw calls.
            self.overlay.require().map_err(PumpingDebugError::Overlay)?;
        }
        Ok(())
    }
}
