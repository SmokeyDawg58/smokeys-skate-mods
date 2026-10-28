//! Latched `PhysOut_Grinds` stores of the Grinds and GameCam conditioners:
//! one `mIsLockedForWrite` check per native `mGrinds` access.
use super::{PhysOutComponent, PhysicalPlayerPhysOut, grinds, prior_input::PriorInputReadError};

impl PhysicalPlayerPhysOut {
    pub(crate) fn try_guarded_grinds_mut(
        &mut self,
    ) -> Result<&mut grinds::Grinds, PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(PhysOutComponent::Grinds)
            .map_err(PriorInputReadError::Component)?;
        Ok(self
            .concrete
            .grinds
            .as_deref_mut()
            .expect("checked component"))
    }
}
