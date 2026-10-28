//! `Body::SetLinearDrag` 0x82de7e6c..0x82de7ee4, actual retained inertia writes.
use super::{
    RetainedInertia, SimulationMutableInertia, SimulationRigidBodyObservation,
    SimulationRigidBodyPool, SimulationRigidBodyPoolError,
};

impl SimulationRigidBodyPool {
    /// Actual single-wheel inertia scalar store at 0x82b372e0. The caller
    /// performs source scaling; no copied full inertia replaces current fields.
    pub(crate) fn try_set_part_angular_drag(
        &mut self,
        body: SimulationRigidBodyObservation,
        scaled_drag: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let owner = self.try_capture_part_inertia(body)?;
        self.try_set_captured_angular_drag(body, &owner, scaled_drag)
    }

    /// Source inertia pointer read, separate from the subsequent scalar store.
    /// Retains the actual owner without sampling its numeric payload.
    pub(crate) fn try_capture_part_inertia(
        &self,
        body: SimulationRigidBodyObservation,
    ) -> Result<SimulationMutableInertia, SimulationRigidBodyPoolError> {
        let index = self.validate_observation(body)?;
        self.validate_integrity()?;
        let Some(RetainedInertia::Mutable(owner)) = &self.slots[index].inertia else {
            return Err(SimulationRigidBodyPoolError::ImmutableInertia { slot: body.id.0 });
        };
        Ok(owner.clone())
    }

    pub(crate) fn try_set_captured_angular_drag(
        &mut self,
        body: SimulationRigidBodyObservation,
        owner: &SimulationMutableInertia,
        scaled_drag: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let current = self.try_capture_part_inertia(body)?;
        if !std::sync::Arc::ptr_eq(&current.0, &owner.0) {
            return Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch);
        }
        let mut state = owner
            .0
            .lock()
            .map_err(|_| SimulationRigidBodyPoolError::InertiaLockPoisoned)?;
        if state.pool != Some(self.authority) {
            return Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch);
        }
        state.value.angular_drag = scaled_drag;
        Ok(())
    }

    /// PC Object Dropper (a port addition): `factor` times the inverse mass and
    /// inverse tensor of the Part's own retained inertia (factor > 1 = lighter).
    /// The body's cached tensors follow at its next transform write.
    pub(crate) fn try_scale_part_lightness(
        &mut self,
        body: SimulationRigidBodyObservation,
        factor: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let owner = self.try_capture_part_inertia(body)?;
        let mut state = owner
            .0
            .lock()
            .map_err(|_| SimulationRigidBodyPoolError::InertiaLockPoisoned)?;
        if state.pool != Some(self.authority) {
            return Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch);
        }
        state.value.inverse_mass *= factor;
        for value in &mut state.value.inverse_tensor {
            *value *= factor;
        }
        Ok(())
    }

    /// Caller groups four source Parts, followed by single tail Parts. Acquire
    /// all group's inertia owners before any store; each later store commits
    /// independently. Shared owner aliases retain native repeated-write order.
    pub(crate) fn try_set_part_group_linear_drag(
        &mut self,
        bodies: &[SimulationRigidBodyObservation],
        scaled_drag: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let mut owners = Vec::with_capacity(bodies.len());
        for &body in bodies {
            let index = self.validate_observation(body)?;
            self.validate_integrity()?;
            let Some(RetainedInertia::Mutable(owner)) = &self.slots[index].inertia else {
                return Err(SimulationRigidBodyPoolError::ImmutableInertia { slot: body.id.0 });
            };
            owners.push(owner.clone());
        }
        for owner in owners {
            let mut state = owner
                .0
                .lock()
                .map_err(|_| SimulationRigidBodyPoolError::InertiaLockPoisoned)?;
            if state.pool != Some(self.authority) {
                return Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch);
            }
            state.value.linear_drag = scaled_drag;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "simulation_rigid_body_linear_drag_tests.rs"]
mod tests;
