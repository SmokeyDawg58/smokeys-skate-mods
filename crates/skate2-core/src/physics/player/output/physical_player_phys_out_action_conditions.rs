//! Direct const `PhysOut` loads in actual Action predicates, not write accessors.
use super::{PhysOutAccessError, PhysOutGroundVector, PhysicalPlayerPhysOut};
impl PhysicalPlayerPhysOut {
    /// DisablePushBrake82c9a9d4..e4 reads Ground+50.Y without the output write latch.
    pub(crate) fn action_condition_ground_up_y(&self) -> Result<f32, PhysOutAccessError> {
        self.ground_vectors[PhysOutGroundVector::GroundUp as usize]
            .map(|value| value.value.xyz[1])
            .ok_or(PhysOutAccessError::NotConstructed)
    }
}
