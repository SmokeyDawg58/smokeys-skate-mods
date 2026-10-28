//! Retained `Physics.Update` fifth call, `CalcFootSideOfDeck`82e25b28.
use super::{Conditioners, PhysicalPlayerPhysOut, damped_com::PhysicsBeforeFootSideOfDeck};
use crate::{
    physical_player_phys_out::construct::foot_side::FootSideError,
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;

/// The next source call is `CalcLandingQuality`82e2aa88, not yet executed.
pub(crate) struct PhysicsBeforeLandingQuality {
    output: Arc<()>,
}
impl PhysicsBeforeLandingQuality {
    pub(crate) fn matches_output(&self, output: &PhysicalPlayerPhysOut) -> bool {
        Arc::ptr_eq(&self.output, output.occurrence())
    }
}
impl Conditioners {
    pub(crate) fn try_update_physics_foot_side<N: ReckoningNumeric>(
        &mut self,
        before: PhysicsBeforeFootSideOfDeck,
        output: &mut PhysicalPlayerPhysOut,
        numeric: &mut N,
    ) -> Result<PhysicsBeforeLandingQuality, FootSideError<N::Error>> {
        if !before.matches_output(output) || !self.matches_output(output) {
            return Err(FootSideError::ForeignOutput);
        }
        drop(before);
        let physics = self
            .physics
            .published_value_mut()
            .ok_or(FootSideError::MissingPhysics)?;
        if !Arc::ptr_eq(&physics.output, output.occurrence()) {
            return Err(FootSideError::ForeignOutput);
        }
        // Native method only reads this->mPhysOut; no Physics history changes.
        output.try_calc_foot_side(numeric)?;
        Ok(PhysicsBeforeLandingQuality {
            output: Arc::clone(output.occurrence()),
        })
    }
}
#[cfg(test)]
#[path = "physical_player_physics_foot_side_tests.rs"]
mod tests;
