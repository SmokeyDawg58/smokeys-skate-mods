//! Pumping owner child: actual same HiLOD+2c0 output callback82e04f24.
use super::{PhysicalPlayerPumpingOwner, PumpingError};
use crate::{
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::construct::pumping_fill::{PumpingFillError, PumpingFillField},
};
use std::sync::Arc;
impl PhysicalPlayerPumpingOwner {
    fn fill_value(&self, field: PumpingFillField) -> Result<f32, PumpingError> {
        let value = self.value.as_ref().ok_or(PumpingError::NotConstructed)?;
        Ok(match field {
            PumpingFillField::Absorption => value.absorption,
            PumpingFillField::GroundAbsorption => value.ground_normal_absorption,
            PumpingFillField::MinimumCrouch => value.ground_normal_minimum_crouch,
            PumpingFillField::DeckAbsorption => value.deck_angle_absorption,
            PumpingFillField::Acceleration => value.acceleration,
        })
    }
}
#[derive(Debug)]
pub(crate) enum PumpingFillRuntimeError {
    Poisoned,
    Pumping(PumpingError),
}
impl std::fmt::Display for PumpingFillRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Pumping.Fill owner poisoned"),
            Self::Pumping(e) => write!(f, "Pumping.Fill owner: {e:?}"),
        }
    }
}
/// Captures the actual member Arc once; source fields are read individually.
/// No constructor-ready preflight precedes the initial output +0 publication.
pub(crate) fn try_fill_pumping_output(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<(), PumpingFillError<PumpingFillRuntimeError>> {
    let pumping = Arc::clone(owner.pumping());
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| PumpingFillError::Source(PumpingFillRuntimeError::Poisoned))?;
    output.try_fill_pumping(|field| {
        pumping
            .lock()
            .map_err(|_| PumpingFillRuntimeError::Poisoned)?
            .fill_value(field)
            .map_err(PumpingFillRuntimeError::Pumping)
    })
}
