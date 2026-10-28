//! Proposed `physical_player_phys_out::construct` child, const Motion+bc read.
use super::super::PhysicalPlayerPhysOut;
use super::{ConcretePhysOutError, PhysOutComponent};
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_motion_pump_potential(
        &self,
    ) -> Result<f32, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        Ok(self
            .concrete
            .motion
            .as_ref()
            .expect("checked component")
            .pump_potential)
    }
}
