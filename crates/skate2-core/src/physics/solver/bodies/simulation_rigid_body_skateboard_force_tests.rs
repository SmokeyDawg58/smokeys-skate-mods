use super::*;
use crate::{
    simulation_rigid_body_pool::{SimulationRigidBodyHandle, SimulationRigidBodySlotSnapshot},
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};

fn fixture() -> (
    SimulationRigidBodyPool,
    SimulationRigidBodyHandle,
    SimulationRigidBodyObservation,
) {
    let mut pool = SimulationRigidBodyPool::try_new(3).unwrap();
    let handle = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    pool.try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    let observation = pool.try_observe(handle).unwrap();
    let slot = &mut pool.slots[handle.id().index() as usize];
    slot.right = [0.0, 2.0, 0.0];
    slot.up = [-3.0, 0.0, 0.0];
    slot.at = [0.0, 0.0, 4.0];
    slot.inverse_mass = 0.25;
    slot.inverse_inertia_full = [2.0, 3.0, 5.0];
    slot.inverse_inertia_split = [11.0, 7.0, 13.0];
    slot.force = [7.0, 11.0, 13.0];
    slot.torque = [17.0, 19.0, 23.0];
    slot.center_of_mass = [1000.0, -2000.0, 3000.0];
    slot.velocity = [29.0, 31.0, 37.0];
    slot.omega = [41.0, 43.0, 47.0];
    slot.kinematic_value = -0.0;
    slot.cooldown = 0x1234_5678;
    slot.tag = 0x8765_4321;
    (pool, handle, observation)
}
fn force(w: Option<f32>) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [2.0, -3.0, 5.0],
        w,
    }
}
fn position(w: Option<f32>) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [1.0, 2.0, 3.0],
        w,
    }
}
fn restore_mutations(
    after: &mut SimulationRigidBodySlotSnapshot,
    before: &SimulationRigidBodySlotSnapshot,
) {
    after.force = before.force;
    after.torque = before.torque;
    after.cooldown = before.cooldown;
}

#[test]
fn nonaxis_arm_original_force_and_cached_offdiagonal_tensor_reach_actual_body_only() {
    let (mut pool, handle, observation) = fixture();
    let before = pool.snapshot();
    pool.try_apply_skateboard_queued_force(observation, force(None), position(None), 0.5)
        .unwrap();
    let mut after = pool.snapshot();
    let index = handle.id().index() as usize;
    let slot = &after.slots[index];
    // Independent exact scalar oracle: arm[-7.5,2,12], arm cross F[46,61.5,18.5],
    // tensor [[2,3,5],[3,7,13],[5,13,11]] gives [369,809,1233].
    assert_eq!(
        slot.force.map(f32::to_bits),
        [7.5_f32, 10.25, 14.25].map(f32::to_bits)
    );
    assert_eq!(
        slot.torque.map(f32::to_bits),
        [386.0_f32, 828.0, 1256.0].map(f32::to_bits)
    );
    assert_eq!(slot.cooldown, 0);
    restore_mutations(&mut after.slots[index], &before.slots[index]);
    assert_eq!(after, before);
}

#[test]
fn unknown_nan_infinite_w_never_contaminates_xyz_or_overwrites_adjacent_fields() {
    let mut expected = None;
    for w in [
        None,
        Some(f32::NAN),
        Some(f32::INFINITY),
        Some(-0.0),
        Some(123.0),
    ] {
        let (mut pool, handle, observation) = fixture();
        let before = pool.snapshot();
        pool.try_apply_skateboard_queued_force(observation, force(w), position(w), 0.5)
            .unwrap();
        let mut after = pool.snapshot();
        let index = handle.id().index() as usize;
        let result = (
            after.slots[index].force.map(f32::to_bits),
            after.slots[index].torque.map(f32::to_bits),
        );
        if let Some(expected) = expected {
            assert_eq!(result, expected);
        } else {
            expected = Some(result);
        }
        restore_mutations(&mut after.slots[index], &before.slots[index]);
        assert_eq!(after, before);
    }
}

#[test]
fn stale_foreign_and_free_observations_refuse_before_body_mutation() {
    let (mut pool, handle, observation) = fixture();
    let (other, _, foreign) = fixture();
    let before = pool.snapshot();
    assert!(matches!(
        pool.try_apply_skateboard_queued_force(foreign, force(None), position(None), 0.5),
        Err(SimulationRigidBodyPoolError::ForeignHandle)
    ));
    assert_eq!(pool.snapshot(), before);
    drop(other);
    pool.try_remove_rigid_body(handle).unwrap();
    let removed = pool.snapshot();
    assert!(matches!(
        pool.try_apply_skateboard_queued_force(observation, force(None), position(None), 0.5),
        Err(SimulationRigidBodyPoolError::BodyIsFree { .. })
    ));
    assert_eq!(pool.snapshot(), removed);
    // Fill the remaining free slot then reuse the released numerical slot.
    pool.try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    let reused = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    assert_eq!(reused.id(), handle.id());
    let reused_snapshot = pool.snapshot();
    assert!(matches!(
        pool.try_apply_skateboard_queued_force(observation, force(None), position(None), 0.5),
        Err(SimulationRigidBodyPoolError::StaleHandle { .. })
    ));
    assert_eq!(pool.snapshot(), reused_snapshot);
}

#[test]
fn repeated_application_rereads_actual_accumulators_and_invalid_next_entry_keeps_prefix() {
    let (mut pool, handle, observation) = fixture();
    let (_, _, foreign) = fixture();
    for _ in 0..2 {
        pool.try_apply_skateboard_queued_force(observation, force(None), position(None), 0.5)
            .unwrap();
    }
    let after = pool.snapshot();
    let slot = &after.slots[handle.id().index() as usize];
    assert_eq!(
        slot.force.map(f32::to_bits),
        [8.0_f32, 9.5, 15.5].map(f32::to_bits)
    );
    assert_eq!(
        slot.torque.map(f32::to_bits),
        [755.0_f32, 1637.0, 2489.0].map(f32::to_bits)
    );
    assert!(
        pool.try_apply_skateboard_queued_force(foreign, force(None), position(None), 0.5)
            .is_err()
    );
    assert_eq!(pool.snapshot(), after);
}

#[test]
fn source_arm_rotation_fma_preserves_cancellation_before_cross() {
    let (mut pool, handle, observation) = fixture();
    let slot = &mut pool.slots[handle.id().index() as usize];
    slot.at = [0.0; 3];
    slot.force = [0.0; 3];
    slot.torque = [0.0; 3];
    slot.inverse_inertia_full = [1.0, 0.0, 0.0];
    slot.inverse_inertia_split = [1.0, 1.0, 0.0];
    // The first product is rounded before cancellation in the fused
    // Up*localY + Ri*localX operation.
    slot.right = [1.0, 0.0, 0.0];
    slot.up = [f32::from_bits(0x3f80_0001), 0.0, 0.0];
    let local = SkeletonRootVector {
        xyz: [
            f32::from_bits(0x3f80_0002),
            -f32::from_bits(0x3f80_0001),
            0.0,
        ],
        w: None,
    };
    pool.try_apply_skateboard_queued_force(
        observation,
        SkeletonRootVector {
            xyz: [0.0, 1.0, 0.0],
            w: None,
        },
        local,
        0.0,
    )
    .unwrap();
    assert_eq!(
        pool.slots[handle.id().index() as usize].torque[2].to_bits(),
        0xa880_0000
    );
}

#[test]
fn entry_order_is_not_collapsed_into_a_single_force_sum() {
    let apply = |values: [f32; 3]| {
        let (mut pool, handle, observation) = fixture();
        let slot = &mut pool.slots[handle.id().index() as usize];
        slot.force = [0.0; 3];
        slot.inverse_mass = 1.0;
        for value in values {
            pool.try_apply_skateboard_queued_force(
                observation,
                SkeletonRootVector {
                    xyz: [value, 0.0, 0.0],
                    w: None,
                },
                SkeletonRootVector::known([0.0; 4]),
                0.0,
            )
            .unwrap();
        }
        pool.slots[handle.id().index() as usize].force[0].to_bits()
    };
    assert_eq!(apply([16_777_216.0, -16_777_216.0, 1.0]), 1.0_f32.to_bits());
    assert_eq!(apply([16_777_216.0, 1.0, -16_777_216.0]), 0);
}

#[test]
fn direct_world_force_uses_actual_part_position_not_body_com_or_local_rotation() {
    let (mut pool, handle, observation) = fixture();
    let before = pool.snapshot();
    pool.try_apply_skateboard_world_force(
        observation,
        force(None),
        SkeletonRootVector {
            xyz: [5.0, 8.0, 11.0],
            w: None,
        },
        SkeletonRootVector {
            xyz: [4.0, 6.0, 8.0],
            w: Some(f32::NAN),
        },
    )
    .unwrap();
    let mut after = pool.snapshot();
    let slot = &after.slots[handle.id().index() as usize];
    assert_eq!(
        slot.force.map(f32::to_bits),
        [7.5_f32, 10.25, 14.25].map(f32::to_bits)
    );
    // Independently arm[1,2,3] cross force[2,-3,5]=[19,1,-7], tensor result[6,-27,31].
    assert_eq!(
        slot.torque.map(f32::to_bits),
        [23.0_f32, -8.0, 54.0].map(f32::to_bits)
    );
    assert_eq!(slot.cooldown, 0);
    restore_mutations(
        &mut after.slots[handle.id().index() as usize],
        &before.slots[handle.id().index() as usize],
    );
    assert_eq!(after, before);
}
