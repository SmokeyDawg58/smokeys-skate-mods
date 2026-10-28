//! Same retained deck body torque accumulation, 82b35728..357e8.
//! Uses the actual retained rigid-body pool occurrence.
use super::{
    SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolError,
};
use crate::skeleton_root_transforms::SkeletonRootVector;

impl SimulationRigidBodyPool {
    pub(crate) fn try_apply_skateboard_local_torque(
        &mut self,
        observation: SimulationRigidBodyObservation,
        torque: SkeletonRootVector,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        let slot = &mut self.slots[index];
        let first = slot.right.map(|value| value * torque.xyz[0]);
        let second: [f32; 3] =
            std::array::from_fn(|lane| slot.up[lane].mul_add(torque.xyz[1], first[lane]));
        let world: [f32; 3] =
            std::array::from_fn(|lane| slot.at[lane].mul_add(torque.xyz[2], second[lane]));
        // Source packed tensor [xx,xy,xz,invMass], [zz,yy,yz,state].
        // Both permutes preserve W only in W, which never enters XYZ stores.
        let full = slot.inverse_inertia_full;
        let split = slot.inverse_inertia_split;
        let y = [full[1], split[1], split[2]];
        let z = [full[2], split[2], split[0]];
        let first = full.map(|value| value * world[0]);
        let second: [f32; 3] = std::array::from_fn(|lane| y[lane].mul_add(world[1], first[lane]));
        let acceleration: [f32; 3] =
            std::array::from_fn(|lane| z[lane].mul_add(world[2], second[lane]));
        let old = slot.torque;
        let new_z = old[2] + acceleration[2];
        let new_y = old[1] + acceleration[1];
        slot.torque[1] = new_y;
        let new_x = old[0] + acceleration[0];
        slot.torque[0] = new_x;
        slot.torque[2] = new_z;
        slot.cooldown = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{SimulationRigidBodyPool, SkeletonRootVector};
    use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
    #[test]
    fn actual_tensor_and_body_basis_accumulate_torque_without_force_or_omega_changes() {
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let handle = pool
            .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
            .unwrap()
            .unwrap();
        let observation = pool.try_observe(handle).unwrap();
        let slot = &mut pool.slots[0];
        slot.right = [0.0, 2.0, 0.0];
        slot.up = [-3.0, 0.0, 0.0];
        slot.at = [0.0, 0.0, 4.0];
        slot.inverse_inertia_full = [2.0, 3.0, 5.0];
        slot.inverse_inertia_split = [11.0, 7.0, 13.0];
        slot.torque = [17.0, 19.0, 23.0];
        slot.cooldown = 31;
        let before_force = slot.force.map(f32::to_bits);
        let before_omega = slot.omega.map(f32::to_bits);
        pool.try_apply_skateboard_local_torque(
            observation,
            SkeletonRootVector {
                xyz: [2.0, -3.0, 5.0],
                w: None,
            },
        )
        .unwrap();
        let slot = &pool.slots[0];
        // World input [9,4,20]; tensor*world = [130,315,317].
        assert_eq!(
            slot.torque.map(f32::to_bits),
            [147.0f32, 334.0, 340.0].map(f32::to_bits)
        );
        assert_eq!(slot.cooldown, 0);
        assert_eq!(slot.force.map(f32::to_bits), before_force);
        assert_eq!(slot.omega.map(f32::to_bits), before_omega);
        let mut foreign = SimulationRigidBodyPool::try_new(1).unwrap();
        assert!(
            foreign
                .try_apply_skateboard_local_torque(observation, SkeletonRootVector::known([0.0; 4]))
                .is_err()
        );
    }
}
