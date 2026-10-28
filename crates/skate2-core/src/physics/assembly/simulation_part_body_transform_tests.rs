//! Real-pool tests for `Part::SetTransform` 0x82b25480. Private nonzero state
//! seeds are synthetic discriminators, never reconstructed gameplay inputs.
use super::*;
use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;

fn inertia() -> Arc<SimulationInertia> {
    Arc::new(SimulationInertia {
        inverse_tensor: [2.0, 5.0, 11.0],
        inverse_mass: 0.125,
        ..SimulationInertia::native_default()
    })
}
fn requested() -> [f32; 16] {
    [
        0.0, 3.0, 0.0, -0.0, -4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 7.0, -9.0, 13.0, 27.0,
    ]
}
fn seed_unrelated(pool: &mut SimulationRigidBodyPool, handle: SimulationRigidBodyHandle) {
    let index = handle.id().index() as usize;
    let slot = &mut pool.slots[index];
    slot.velocity = [3.0, -0.0, -7.0];
    slot.omega = [-11.0, 13.0, -0.0];
    slot.force = [17.0, -19.0, 23.0];
    slot.torque = [-29.0, 31.0, -37.0];
    slot.kinematic_value = -0.0;
    slot.cooldown = 0x1234_5678;
    slot.tag = 0x89ab_cdef;
    for (ordinal, row) in pool.solver_delta_rows.iter_mut().enumerate() {
        let ordinal = f32::from(u8::try_from(ordinal).unwrap());
        row.vectors = [
            [ordinal + 1.0, -2.0, -0.0, 4.0],
            [5.0, -6.0, 7.0, 8.0],
            [-9.0, 10.0, 11.0, -12.0],
            [13.0, 14.0, -15.0, 16.0],
        ];
    }
}
fn rf_bits(pool: &SimulationRigidBodyPool) -> Vec<[[u32; 4]; 4]> {
    pool.solver_delta_rows
        .iter()
        .map(|row| row.vectors.map(|vector| vector.map(f32::to_bits)))
        .collect()
}
fn restore_pose_fields(
    after: &mut SimulationRigidBodySlotSnapshot,
    before: &SimulationRigidBodySlotSnapshot,
) {
    after.right = before.right;
    after.up = before.up;
    after.at = before.at;
    after.center_of_mass = before.center_of_mass;
    after.quaternion = before.quaternion;
}

#[test]
fn active_and_frozen_transform_updates_only_pose_and_six_tensor_components() {
    for state in [2, 4] {
        let mut pool = SimulationRigidBodyPool::try_new(4).unwrap();
        pool.set_gravity([1.0, -2.0, 3.0]);
        pool.begin_dynamic_update_frame(1.0 / 120.0);
        let owner = inertia();
        let first = pool
            .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&owner), state)
            .unwrap()
            .unwrap();
        let second = pool
            .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&owner), 4)
            .unwrap()
            .unwrap();
        seed_unrelated(&mut pool, first);
        let index = first.id().index() as usize;
        pool.slots[index].inverse_inertia_full = [61.0, 67.0, 71.0];
        pool.slots[index].inverse_inertia_split = [73.0, 79.0, 83.0];
        pool.slots[index].inverse_mass = 89.0;
        let before = pool.snapshot();
        let rf = rf_bits(&pool);
        let frame = pool.frame_identity;
        let next_frame = pool.next_frame_identity;
        let observed = pool.try_observe(first).unwrap();
        pool.try_set_part_body_transform_host(first, requested())
            .unwrap();
        assert_eq!(pool.try_observe(first).unwrap(), observed);
        assert!(Arc::ptr_eq(
            pool.try_inertia(first).unwrap().unwrap(),
            &owner
        ));
        let after = pool.snapshot();
        let slot = &after.slots[index];
        assert_eq!(
            slot.center_of_mass.map(f32::to_bits),
            [7.0_f32.to_bits(), (-9.0_f32).to_bits(), 13.0_f32.to_bits()]
        );
        // Quarter-turn source outcome: tensor xx becomes5, yy2, zz11.
        for (actual, expected) in slot.inverse_inertia_full.into_iter().zip([5.0, 0.0, 0.0]) {
            assert!((actual - expected).abs() < 0.000_01);
        }
        for (actual, expected) in slot.inverse_inertia_split.into_iter().zip([11.0, 2.0, 0.0]) {
            assert!((actual - expected).abs() < 0.000_01);
        }
        assert_eq!(slot.inverse_mass.to_bits(), 0.125_f32.to_bits());
        assert!(
            (slot
                .quaternion
                .iter()
                .map(|value| value * value)
                .sum::<f32>()
                - 1.0)
                .abs()
                < 0.000_001
        );
        let normalized = orthonormalize_collision_affine(requested());
        assert_eq!(
            slot.right.map(f32::to_bits),
            [normalized[0], normalized[1], normalized[2]].map(f32::to_bits)
        );
        assert_eq!(
            slot.up.map(f32::to_bits),
            [normalized[4], normalized[5], normalized[6]].map(f32::to_bits)
        );
        assert_eq!(
            slot.at.map(f32::to_bits),
            [normalized[8], normalized[9], normalized[10]].map(f32::to_bits)
        );
        assert_eq!(
            slot.quaternion.map(f32::to_bits),
            simulation_quaternion_from_orthonormalized_matrix_host(normalized).map(f32::to_bits)
        );
        let mut unchanged = after.clone();
        restore_pose_fields(&mut unchanged.slots[index], &before.slots[index]);
        unchanged.slots[index].inverse_inertia_full = before.slots[index].inverse_inertia_full;
        unchanged.slots[index].inverse_inertia_split = before.slots[index].inverse_inertia_split;
        unchanged.slots[index].inverse_mass = before.slots[index].inverse_mass;
        assert_eq!(unchanged, before);
        assert_eq!(
            slot.velocity.map(f32::to_bits),
            before.slots[index].velocity.map(f32::to_bits)
        );
        assert_eq!(
            slot.omega.map(f32::to_bits),
            before.slots[index].omega.map(f32::to_bits)
        );
        assert_eq!(
            slot.kinematic_value.to_bits(),
            before.slots[index].kinematic_value.to_bits()
        );
        assert_eq!(rf_bits(&pool), rf);
        assert_eq!(pool.frame_identity, frame);
        assert_eq!(pool.next_frame_identity, next_frame);
        pool.try_remove_rigid_body(first).unwrap();
        pool.try_remove_rigid_body(second).unwrap();
    }
}

#[test]
fn null_live_inertia_preserves_existing_tensor_and_mass_even_with_source_definition_owner() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let definition = inertia();
    let body = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&definition), 1)
        .unwrap()
        .unwrap();
    assert!(pool.try_inertia(body).unwrap().is_none());
    seed_unrelated(&mut pool, body);
    let index = body.id().index() as usize;
    pool.slots[index].inverse_inertia_full = [41.0, -0.0, -43.0];
    pool.slots[index].inverse_inertia_split = [-47.0, 53.0, -0.0];
    pool.slots[index].inverse_mass = 59.0;
    let before = pool.snapshot();
    let rf = rf_bits(&pool);
    pool.try_set_part_body_transform_host(body, requested())
        .unwrap();
    let mut after = pool.snapshot();
    assert_eq!(
        after.slots[index].center_of_mass.map(f32::to_bits),
        [7.0_f32.to_bits(), (-9.0_f32).to_bits(), 13.0_f32.to_bits()]
    );
    assert_ne!(
        after.slots[index].quaternion.map(f32::to_bits),
        before.slots[index].quaternion.map(f32::to_bits)
    );
    assert_eq!(
        after.slots[index].inverse_inertia_full.map(f32::to_bits),
        before.slots[index].inverse_inertia_full.map(f32::to_bits)
    );
    assert_eq!(
        after.slots[index].inverse_inertia_split.map(f32::to_bits),
        before.slots[index].inverse_inertia_split.map(f32::to_bits)
    );
    assert_eq!(
        after.slots[index].inverse_mass.to_bits(),
        before.slots[index].inverse_mass.to_bits()
    );
    assert!(after.slots[index].inertia.is_none());
    restore_pose_fields(&mut after.slots[index], &before.slots[index]);
    assert_eq!(after, before);
    assert_eq!(rf_bits(&pool), rf);
    assert_eq!(
        definition.inverse_tensor.map(f32::to_bits),
        [2.0_f32.to_bits(), 5.0_f32.to_bits(), 11.0_f32.to_bits()]
    );
}

#[test]
fn foreign_free_and_reused_occurrences_fail_before_any_mutation() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let mut other = SimulationRigidBodyPool::try_new(1).unwrap();
    let owner = inertia();
    let old = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&owner), 4)
        .unwrap()
        .unwrap();
    let foreign = other
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&owner), 4)
        .unwrap()
        .unwrap();
    seed_unrelated(&mut pool, old);
    let before = pool.snapshot();
    let rf = rf_bits(&pool);
    assert_eq!(
        pool.try_set_part_body_transform_host(foreign, requested()),
        Err(SimulationRigidBodyPoolError::ForeignHandle)
    );
    assert_eq!(pool.snapshot(), before);
    assert_eq!(rf_bits(&pool), rf);
    pool.try_remove_rigid_body(old).unwrap();
    let before = pool.snapshot();
    assert_eq!(
        pool.try_set_part_body_transform_host(old, requested()),
        Err(SimulationRigidBodyPoolError::BodyIsFree {
            slot: old.id().index()
        })
    );
    assert_eq!(pool.snapshot(), before);
    let current = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&owner), 4)
        .unwrap()
        .unwrap();
    let before = pool.snapshot();
    let rf = rf_bits(&pool);
    assert_eq!(
        pool.try_set_part_body_transform_host(old, requested()),
        Err(SimulationRigidBodyPoolError::StaleHandle {
            slot: old.id().index()
        })
    );
    assert_eq!(pool.snapshot(), before);
    assert_eq!(rf_bits(&pool), rf);
    assert!(pool.try_observe(current).is_ok());
}

#[test]
fn corrupt_pool_structure_refuses_before_pose_writes() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let owner = inertia();
    let body = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&owner), 4)
        .unwrap()
        .unwrap();
    seed_unrelated(&mut pool, body);
    // Private structural fault, not a valid production mutation/capability.
    let other = SimulationRigidBodyPool::try_new(0).unwrap();
    pool.slots[0].stasis = other.authority();
    let before = pool.snapshot();
    let rf = rf_bits(&pool);
    assert_eq!(
        pool.try_set_part_body_transform_host(body, requested()),
        Err(SimulationRigidBodyPoolError::CorruptStructure)
    );
    assert_eq!(pool.snapshot(), before);
    assert_eq!(rf_bits(&pool), rf);
    pool.slots[0].stasis = pool.authority();
    pool.try_remove_rigid_body(body).unwrap();
}
