//! Component-only `StoreLandingData` reads; native const loads do not sample latch.
use super::super::{PhysOutSystemVector, PhysicalPlayerPhysOut};
use super::{ConcretePhysOutError, PhysOutComponent};
use crate::skeleton_root_transforms::SkeletonRootVector;
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_landing_system_vector(
        &self,
        field: PhysOutSystemVector,
    ) -> Result<SkeletonRootVector, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::SystemReckoning)?;
        self.system_vectors[field as usize]
            .map(|v| v.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
}
