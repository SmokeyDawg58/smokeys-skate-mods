//! Body velocity store at 0x82b24514, preserving the intrusive link in native W.
use super::{
    SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolError,
};
impl SimulationRigidBodyPool {
    pub(crate) fn try_set_part_linear_velocity(
        &mut self,
        body: SimulationRigidBodyObservation,
        velocity: [f32; 3],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(body)?;
        self.validate_integrity()?;
        self.slots[index].velocity = velocity;
        // Host freshness only: an old solved-frame certificate cannot describe
        // a mutated body. The next actual BeginFrame issues a new identity.
        // Do not advance native time or alter retained historical output values.
        self.frame_identity = None;
        Ok(())
    }

    /// `Skateboard::PostPhysics_AdjustVelocity` 0x82b32b28's store to the deck body's `mVel`
    /// after the frame's solve: the native post-physics readers that follow read the same
    /// solve's contacts, so the solved frame stays current.
    pub(crate) fn try_store_post_solve_linear_velocity(
        &mut self,
        body: SimulationRigidBodyObservation,
        velocity: [f32; 3],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(body)?;
        self.validate_integrity()?;
        self.slots[index].velocity = velocity;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
    #[test]
    fn xyz_store_keeps_native_link_and_all_other_body_fields() {
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let body = pool
            .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
            .unwrap()
            .unwrap();
        let observation = pool.try_observe(body).unwrap();
        pool.slots[0].omega = [2.0, 3.0, 4.0];
        pool.slots[0].force = [5.0, 6.0, 7.0];
        let fields = |slot: &super::super::RigidBodySlot| {
            (
                slot.links,
                slot.omega,
                slot.force,
                slot.torque,
                slot.state,
                slot.cooldown,
                slot.center_of_mass,
            )
        };
        let before = fields(&pool.slots[0]);
        let time = (pool.timestep.to_bits(), pool.frequency.to_bits());
        let value = [0x8000_0000, 0x7fc1_2345, 0x3f80_0000].map(f32::from_bits);
        pool.try_set_part_linear_velocity(observation, value)
            .unwrap();
        assert_eq!(
            pool.slots[0].velocity.map(f32::to_bits),
            value.map(f32::to_bits)
        );
        assert_eq!(fields(&pool.slots[0]), before);
        assert_eq!((pool.timestep.to_bits(), pool.frequency.to_bits()), time);
        assert_eq!(pool.frame_identity(), None);
        pool.begin_dynamic_update_frame(f32::from_bits(0x3c88_8889));
        assert!(pool.frame_identity().is_some());
        assert_eq!(
            pool.slots[0].velocity.map(f32::to_bits),
            value.map(f32::to_bits)
        );
    }
    #[test]
    fn foreign_body_refusal_keeps_frame_and_velocity() {
        let mut source = SimulationRigidBodyPool::try_new(1).unwrap();
        let body = source
            .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
            .unwrap()
            .unwrap();
        let observation = source.try_observe(body).unwrap();
        let mut other = SimulationRigidBodyPool::try_new(1).unwrap();
        let frame = other.frame_identity();
        assert!(
            other
                .try_set_part_linear_velocity(observation, [1.0; 3])
                .is_err()
        );
        assert_eq!(other.frame_identity(), frame);
    }
}
