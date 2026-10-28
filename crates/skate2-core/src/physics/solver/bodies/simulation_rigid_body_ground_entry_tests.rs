use super::*;
use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;

#[test]
fn omega_store_retains_packed_links_and_invalidates_only_after_commit() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let body = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    let observation = pool.try_observe(body).unwrap();
    pool.slots[0].velocity = [3.0, 4.0, 5.0];
    let fields = |slot: &super::super::RigidBodySlot| {
        (
            slot.links,
            slot.velocity,
            slot.force,
            slot.torque,
            slot.state,
            slot.cooldown,
            slot.center_of_mass,
        )
    };
    let before = fields(&pool.slots[0]);
    pool.begin_dynamic_update_frame(f32::from_bits(0x3c88_8889));
    assert!(pool.frame_identity().is_some());
    let omega = [0x8000_0000, 0x7fc1_2345, 0x3f80_0000].map(f32::from_bits);
    pool.try_set_part_angular_velocity(observation, omega)
        .unwrap();
    assert_eq!(
        pool.slots[0].omega.map(f32::to_bits),
        omega.map(f32::to_bits)
    );
    assert_eq!(fields(&pool.slots[0]), before);
    assert!(pool.frame_identity().is_none());
    let mut other = SimulationRigidBodyPool::try_new(1).unwrap();
    other.begin_dynamic_update_frame(f32::from_bits(0x3c88_8889));
    let frame = other.frame_identity();
    assert!(
        other
            .try_set_part_angular_velocity(observation, omega)
            .is_err()
    );
    assert_eq!(other.frame_identity(), frame);
}
