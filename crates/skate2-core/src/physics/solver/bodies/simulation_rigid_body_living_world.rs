//! Existing body-pool publication for LivingWorld collision target velocities.
use super::{
    SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolError,
};

#[cfg(test)]
#[path = "simulation_rigid_body_living_world_tests.rs"]
mod tests;

impl SimulationRigidBodyPool {
    /// cLivingWorldCollisionBody.SetMasses82e50c10 mutates the shared Definition
    /// before constructing another Assembly. Existing Parts retain its Inertia
    /// pointer: update this SAME semantic allocation, not a per-car copy. Cached
    /// body inertia fields are not eagerly rewritten by the original call.
    pub(crate) fn try_write_living_world_definition_inertia(
        &mut self,
        owner: &super::SimulationMutableInertia,
        value: super::SimulationInertia,
        w: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.validate_integrity()?;
        let mut state = owner
            .0
            .lock()
            .map_err(|_| SimulationRigidBodyPoolError::InertiaLockPoisoned)?;
        if state.pool.is_some_and(|pool| pool != self.authority) {
            return Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch);
        }
        state.value = value;
        state.inverse_tensor_w = Some(w);
        Ok(())
    }

    /// Body.AddForce82de8218 WORLD_SPACE, called for an Island-attached body.
    /// This force is applied at the body origin and produces no torque. Reuse
    /// the existing activation rings; body/observation identity stays unchanged.
    pub(crate) fn try_add_body_world_force(
        &mut self,
        body: SimulationRigidBodyObservation,
        force: [f32; 3],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(body)?;
        self.validate_integrity()?;
        if self.slots[index].state & 7 == 2 {
            self.try_activate_frozen_body(body)?;
        }
        self.try_clear_body_cooldown(body)?;
        let slot = &mut self.slots[index];
        let acceleration = force.map(|value| value * slot.inverse_mass);
        let y = slot.force[1] + acceleration[1];
        let z = slot.force[2] + acceleration[2];
        slot.force[0] += acceleration[0];
        slot.force[1] = y;
        slot.force[2] = z;
        slot.cooldown = 0;
        Ok(())
    }

    /// UpdateCollisionBodyPartsFromTransform82e509d4..50ac0, after its numeric
    /// prefix. Preserve the adjacent W fields (right/left ring links), then
    /// activate if Frozen and clear cooldown even when already Active.
    pub(crate) fn try_publish_living_world_part_drive(
        &mut self,
        body: SimulationRigidBodyObservation,
        velocity: [f32; 3],
        omega: [f32; 3],
        ignore_gravity: bool,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(body)?;
        self.validate_integrity()?;
        if ignore_gravity {
            let magnitude = f32::from_bits(0x411c_cccd) / self.slots[index].inverse_mass;
            // Do not simplify to +9.8 acceleration: source divides by inverse
            // mass here, then Body.AddForce multiplies by it again.
            self.try_add_body_world_force(body, [0.0, magnitude, 0.0])?;
        }
        self.try_set_part_linear_velocity(body, velocity)?;
        self.try_set_part_angular_velocity(body, omega)?;
        if self.slots[index].state & 7 == 2 {
            self.try_activate_frozen_body(body)?;
        }
        self.try_clear_body_cooldown(body)
    }
}
