//! Proposed child of `physical_player_phys_out::construct`; const source reads.
use super::super::{PhysOutMotionVector, PhysicalPlayerPhysOut};
use super::{ConcretePhysOutError, PhysOutComponent};
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};
type Error = ConcretePhysOutError<core::convert::Infallible>;
impl PhysicalPlayerPhysOut {
    /// `GetGroundHeadingCosine825f4f7c` Motion+50; no write latch sample.
    pub(crate) fn try_motion_deck_velocity(&self) -> Result<SkeletonRootVector, Error> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        self.motion_vectors[PhysOutMotionVector::DeckVelocity as usize]
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    /// Ground transform at PhysOut+20, read only after squared-speed gate.
    pub(crate) fn try_motion_ground_heading_transform(&self) -> Result<SkeletonRootMatrix, Error> {
        self.require_complete_component(PhysOutComponent::Ground)?;
        self.ground_transform
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    /// Motion+111, queried after Ground transform capture at825f500c.
    pub(crate) fn try_motion_skateboard_flipped(&self) -> Result<bool, Error> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        Ok(self
            .concrete
            .motion
            .as_ref()
            .expect("checked component")
            .skateboard_flipped)
    }
}
