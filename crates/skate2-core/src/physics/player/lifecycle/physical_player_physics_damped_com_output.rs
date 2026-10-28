//! Guarded actual `SystemReckoning` accesses for `UpdateDampedComPos`82e2a720.
use super::super::{
    PhysOutComponent, PhysOutSystemVector, PhysOutWriteGuard, PhysicalPlayerPhysOut,
    SkeletonRootVector,
};
use super::prior_input::PriorInputReadError as E;
impl PhysicalPlayerPhysOut {
    pub(crate) fn damped_com_check(&self, guard: &PhysOutWriteGuard) -> Result<(), E> {
        self.check_write_guard(guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::SystemReckoning)
            .map_err(E::Component)
    }
    pub(crate) fn damped_com_initial(
        &self,
        valid: impl FnOnce() -> bool,
    ) -> Result<(bool, SkeletonRootVector), E> {
        self.damped_com_check(&self.sample_write_guard().map_err(E::Access)?)?;
        let valid = valid();
        Ok((
            valid,
            self.system_vectors[PhysOutSystemVector::CentreOfMass as usize]
                .as_ref()
                .unwrap()
                .value,
        ))
    }
    /// Called only after `damped_com_check`, without another native guard sample.
    pub(crate) fn damped_com_velocity_after_guard(&self) -> SkeletonRootVector {
        self.system_vectors[PhysOutSystemVector::CentreOfMassVelocity as usize]
            .as_ref()
            .unwrap()
            .value
    }
    pub(crate) fn damped_com_store(
        &mut self,
        value: impl FnOnce() -> SkeletonRootVector,
    ) -> Result<(), E> {
        self.damped_com_check(&self.sample_write_guard().map_err(E::Access)?)?;
        self.concrete.system.as_mut().unwrap().damped_centre_of_mass = value();
        Ok(())
    }
}
