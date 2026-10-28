//! Frozen-to-Active primitive 0x82ae3228 and Assembly cooldown store0x82ae0a18.
use super::{
    SimulationRigidBodyList, SimulationRigidBodyNode, SimulationRigidBodyObservation,
    SimulationRigidBodyPool, SimulationRigidBodyPoolError,
};

impl SimulationRigidBodyPool {
    /// Native callers establish Frozen state before calling. Host occurrence and
    /// ring checks refuse invalid inputs before the first native link store.
    pub(crate) fn try_activate_frozen_body(
        &mut self,
        body: SimulationRigidBodyObservation,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(body)?;
        self.validate_integrity()?;
        let state = self.slots[index].state;
        if state & 7 != 2 {
            return Err(SimulationRigidBodyPoolError::BodyNotFrozen {
                slot: self.slots[index].id.0,
                state,
            });
        }
        let node = SimulationRigidBodyNode::Body(self.slots[index].id);
        // Right.left, left.right, then append after the Active tail. Neither
        // body occurrence nor any pose/inertia/RF payload is reset.
        self.unlink(node);
        self.append_to_tail(node, SimulationRigidBodyList::Active);
        self.slots[index].state = (state & 8) | 4;
        self.slots[index].cooldown = self.cooldown.wrapping_sub(1);
        self.counts.frozen = self.counts.frozen.wrapping_sub(1);
        self.counts.active = self.counts.active.wrapping_add(1);
        Ok(())
    }

    /// Unconditional per-Part cooldown store after the conditional activation.
    pub(crate) fn try_clear_body_cooldown(
        &mut self,
        body: SimulationRigidBodyObservation,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(body)?;
        self.validate_integrity()?;
        self.slots[index].cooldown = 0;
        Ok(())
    }
}
#[cfg(test)]
#[path = "simulation_rigid_body_activation_tests.rs"]
mod tests;
