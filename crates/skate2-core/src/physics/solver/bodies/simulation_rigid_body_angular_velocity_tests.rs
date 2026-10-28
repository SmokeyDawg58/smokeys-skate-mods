use super::*;
use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
#[test]
fn late_read_uses_current_omega_and_refuses_foreign_observation() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let body = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    let observation = pool.try_observe(body).unwrap();
    pool.slots[0].omega = [-0.0, 3.0, 7.0];
    assert_eq!(
        pool.try_phys_out_angular_velocity(observation)
            .unwrap()
            .map(f32::to_bits),
        [-0.0f32, 3.0, 7.0].map(f32::to_bits)
    );
    let foreign = SimulationRigidBodyPool::try_new(1).unwrap();
    assert!(foreign.try_phys_out_angular_velocity(observation).is_err());
}
