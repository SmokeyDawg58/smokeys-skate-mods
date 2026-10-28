//! Actual retained sixth `Physics.Update` call: `CalcLandingQuality`82e2aa88.
use super::{Conditioners, PhysicalPlayerPhysOut, foot_side::PhysicsBeforeLandingQuality};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    input_cvar_runtime::{InputCvarReferences, landing_quality::LandingCvars},
    physical_player_phys_out::construct::landing_quality::LandingQualityError,
    physics::FilteredPhysicsState,
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;
pub(crate) struct PhysicsBeforeStairs {
    output: Arc<()>,
}
impl PhysicsBeforeStairs {
    pub(crate) fn matches_output(&self, output: &PhysicalPlayerPhysOut) -> bool {
        Arc::ptr_eq(&self.output, output.occurrence())
    }
}
impl Conditioners {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "consume the prerequisite token to prevent replay"
    )]
    pub(crate) fn try_update_physics_landing_quality<N: ReckoningNumeric>(
        &mut self,
        before: PhysicsBeforeLandingQuality,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        global_physics_animation: Option<&AttributeRuntimeInstance>,
        numeric: &mut N,
    ) -> Result<PhysicsBeforeStairs, LandingQualityError<N::Error>> {
        use LandingQualityError as E;
        if !before.matches_output(output) || !self.matches_output(output) {
            return Err(E::ForeignOutput);
        }
        let physics = self
            .physics
            .published_value_mut()
            .ok_or(E::MissingPhysics)?;
        if !Arc::ptr_eq(&physics.output, output.occurrence()) {
            return Err(E::ForeignOutput);
        }
        //82e2aab8..cc: current-state read is lazy after last-state comparison.
        if physics.payload.last_filtered_state != FilteredPhysicsState::Ground
            && physics.payload.filtered_state == FilteredPhysicsState::Ground
        {
            output.try_calc_landing_quality(
                &LandingCvars(cvars),
                global_physics_animation,
                numeric,
            )?;
        }
        Ok(PhysicsBeforeStairs {
            output: Arc::clone(output.occurrence()),
        })
    }
}
#[cfg(test)]
#[path = "physical_player_physics_landing_quality_tests.rs"]
mod tests;
