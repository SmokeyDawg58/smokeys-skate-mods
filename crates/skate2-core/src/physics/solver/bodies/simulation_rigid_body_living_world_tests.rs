//! Source-branch regressions; not yet compiled or run.
use super::*;
use crate::{
    simulation_rigid_body_pool::SimulationRigidBodyList, skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};

#[test]
fn target_drive_wakes_body_keeps_pose_and_cancels_gravity_without_torque() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let other = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    let body = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 2)
        .unwrap()
        .unwrap();
    let index = body.id().index() as usize;
    pool.slots[index].inverse_mass = 0.25;
    pool.slots[index].force = [2., 0., -3.];
    pool.slots[index].torque = [7., 8., 9.];
    let pose = pool.slots[index].center_of_mass;
    let observed = body.observation();
    pool.try_publish_living_world_part_drive(observed, [4., 5., 6.], [0., 2., 0.], true)
        .unwrap();
    let slot = &pool.slots[index];
    assert_eq!(slot.force, [2., f32::from_bits(0x411c_cccd), -3.]);
    assert_eq!(slot.torque, [7., 8., 9.]);
    assert_eq!(slot.velocity, [4., 5., 6.]);
    assert_eq!(slot.omega, [0., 2., 0.]);
    assert_eq!(slot.center_of_mass, pose);
    assert_eq!(slot.cooldown, 0);
    assert_eq!(
        pool.list_order(SimulationRigidBodyList::Active),
        vec![other.id(), body.id()]
    );
    assert_eq!(pool.try_observe(body).unwrap(), observed);
    pool.validate_integrity().unwrap();
}

#[test]
fn zero_inverse_mass_retains_original_divide_then_multiply_exception() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let body = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    pool.slots[0].inverse_mass = 0.;
    pool.try_publish_living_world_part_drive(body.observation(), [0.; 3], [0.; 3], true)
        .unwrap();
    assert!(pool.slots[0].force[1].is_nan());
    assert_eq!(pool.slots[0].force[0], 0.);
    assert_eq!(pool.slots[0].force[2], 0.);
    pool.slots[0].force = [1., 2., 3.];
    let links = pool.slots[0].links;
    pool.try_publish_living_world_part_drive(body.observation(), [1.; 3], [2.; 3], false)
        .unwrap();
    assert_eq!(pool.slots[0].force, [1., 2., 3.]);
    assert_eq!(pool.slots[0].links, links);
}
