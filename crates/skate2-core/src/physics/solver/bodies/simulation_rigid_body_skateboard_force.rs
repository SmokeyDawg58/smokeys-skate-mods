//! One actual deck entry from `Skateboard::ApplyForces` 0x82b35438..0x82b35564.
//! The queue owner supplies its retained entry and already read local-Y offset.
//! This operation neither clears the queue nor advances pre/post physics.

use super::{
    SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolError,
};
use crate::{reckoning_numeric::cross_full4, skeleton_root_transforms::SkeletonRootVector};

impl SimulationRigidBodyPool {
    /// Source cached body orientation/inverse mass, then force XYZ commits,
    /// then cached tensor/torque reads, torque Y/X/Z commits and cooldown zero.
    /// Native per-entry arithmetic has no callbacks or fallible late resources;
    /// host observation/ring validation occurs before selecting its actual slot.
    pub(crate) fn try_apply_skateboard_queued_force(
        &mut self,
        observation: SimulationRigidBodyObservation,
        force: SkeletonRootVector,
        local_position: SkeletonRootVector,
        local_y_offset: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        let slot = &mut self.slots[index];
        // Source adds +0 on X/Z as well; preserve signed-zero arithmetic.
        let local = [
            local_position.xyz[0] + 0.0,
            local_position.xyz[1] + local_y_offset,
            local_position.xyz[2] + 0.0,
        ];
        let inverse_mass = slot.inverse_mass;
        let right = slot.right;
        let up = slot.up;
        let at = slot.at;
        let old_force = slot.force;
        let first = right.map(|value| value * local[0]);
        let second: [f32; 3] = std::array::from_fn(|lane| up[lane].mul_add(local[1], first[lane]));
        let arm: [f32; 3] = std::array::from_fn(|lane| at[lane].mul_add(local[2], second[lane]));
        let acceleration = force.xyz.map(|value| value * inverse_mass);
        let force_z = old_force[2] + acceleration[2];
        let force_y = old_force[1] + acceleration[1];
        let force_x = acceleration[0] + old_force[0];
        slot.force[0] = force_x;
        slot.force[1] = force_y;
        slot.force[2] = force_z;

        // Late source loads, after ALL three force stores. This consumes cached
        // WORLD inverse inertia, not the mutable source inertia owner/diagonal.
        let split = slot.inverse_inertia_split;
        let full = slot.inverse_inertia_full;
        let old_torque_y = slot.torque[1];
        let old_torque_z = slot.torque[2];
        let old_torque_x = slot.torque[0];

        // YZXW permutations never route W into XYZ. Scratch W is explicitly
        // unobserved: cross_full4's W result is discarded, with no write of a
        // fabricated source W into orientation, force, torque or queued data.
        // Torque uses original force, not the inverse-mass-scaled acceleration.
        let moment = cross_full4(
            [arm[0], arm[1], arm[2], 0.0],
            [force.xyz[0], force.xyz[1], force.xyz[2], 0.0],
        );
        let angular_acceleration = cached_tensor_product(full, split, moment);
        let torque_z = old_torque_z + angular_acceleration[2];
        let torque_y = old_torque_y + angular_acceleration[1];
        slot.torque[1] = torque_y;
        let torque_x = old_torque_x + angular_acceleration[0];
        slot.torque[0] = torque_x;
        slot.torque[2] = torque_z;
        slot.cooldown = 0;
        Ok(())
    }
}

/// Source X product then Y/Z FMA, shared by deck and Body force paths.
pub(super) fn cached_tensor_product(full: [f32; 3], split: [f32; 3], moment: [f32; 4]) -> [f32; 3] {
    let column_y = [full[1], split[1], split[2]];
    let column_z = [full[2], split[2], split[0]];
    let tensor_first = full.map(|value| value * moment[0]);
    let tensor_second: [f32; 3] =
        std::array::from_fn(|lane| column_y[lane].mul_add(moment[1], tensor_first[lane]));
    let angular_acceleration: [f32; 3] =
        std::array::from_fn(|lane| column_z[lane].mul_add(moment[2], tensor_second[lane]));
    angular_acceleration
}
impl SimulationRigidBodyPool {
    /// `ApplyWorldSpaceForceToDeck`0x82b355d8. Point and Part6 position are world space.
    pub(crate) fn try_apply_skateboard_world_force(
        &mut self,
        observation: SimulationRigidBodyObservation,
        force: SkeletonRootVector,
        point: SkeletonRootVector,
        part_position: SkeletonRootVector,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        let slot = &mut self.slots[index];
        let inverse_mass = slot.inverse_mass;
        let arm = point.zip(part_position, |a, b| a - b);
        let old_force = slot.force;
        let moment = cross_full4(
            [arm.xyz[0], arm.xyz[1], arm.xyz[2], arm.w.unwrap_or(0.0)],
            [
                force.xyz[0],
                force.xyz[1],
                force.xyz[2],
                force.w.unwrap_or(0.0),
            ],
        );
        let acceleration = force.xyz.map(|v| v * inverse_mass);
        let force_x = old_force[0] + acceleration[0];
        let force_z = old_force[2] + acceleration[2];
        slot.force[0] = force_x;
        let force_y = old_force[1] + acceleration[1];
        slot.force[1] = force_y;
        slot.force[2] = force_z;
        // Late cached WORLD inverse inertia, after all force publications.
        let split = slot.inverse_inertia_split;
        let full = slot.inverse_inertia_full;
        let old_y = slot.torque[1];
        let old_x = slot.torque[0];
        let old_z = slot.torque[2];
        let angular = cached_tensor_product(full, split, moment);
        let torque_x = old_x + angular[0];
        let torque_y = old_y + angular[1];
        slot.torque[0] = torque_x;
        let torque_z = old_z + angular[2];
        slot.torque[1] = torque_y;
        slot.torque[2] = torque_z;
        slot.cooldown = 0;
        Ok(())
    }
}

#[cfg(test)]
#[path = "simulation_rigid_body_skateboard_force_tests.rs"]
mod tests;
