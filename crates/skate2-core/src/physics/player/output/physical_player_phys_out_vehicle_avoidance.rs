//! VehiclesShouldAvoidSkater826f38c0 reads Scoring2's live bail records.
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_vehicle_avoidance(
        &self,
        disabled: [bool; 3],
    ) -> Result<bool, ConcretePhysOutError<std::convert::Infallible>> {
        let [any, air, all] = disabled;
        if all {
            return Ok(false);
        }
        if !any && !air {
            return Ok(true);
        }
        self.require_complete_component(PhysOutComponent::Scoring2)?;
        let scoring = self
            .concrete
            .scoring
            .as_deref()
            .ok_or(ConcretePhysOutError::NotConstructed)?;
        if !scoring.ho_m_bail_in_progress {
            return Ok(true);
        }
        if any {
            return Ok(false);
        }
        if scoring.ho_m_bail_data.current_air_drop.duration_in_frames == 0 {
            return Ok(true);
        }
        Ok(!air)
    }
}
