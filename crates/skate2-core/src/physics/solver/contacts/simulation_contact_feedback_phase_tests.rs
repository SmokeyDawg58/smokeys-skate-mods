use super::*;
use crate::simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyHandle};
use std::sync::Arc;

const MATRIX: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

fn add(pool: &mut SimulationRigidBodyPool, state: u32) -> SimulationRigidBodyHandle {
    pool.try_add_rigid_body(
        MATRIX,
        Some(&Arc::new(SimulationInertia::native_default())),
        state,
    )
    .expect("actual Add")
    .expect("capacity")
}

fn rf_words(pool: &SimulationRigidBodyPool) -> Vec<[[u32; 4]; 4]> {
    pool.solver_delta_rows
        .iter()
        .map(|row| row.vectors.map(|v| v.map(f32::to_bits)))
        .collect()
}

fn seed_rf(pool: &mut SimulationRigidBodyPool) {
    for (index, row) in pool.solver_delta_rows.iter_mut().enumerate() {
        // Deliberately synthetic RF words, including -0 and NaN: memset is
        // sixteen +zero words, not a numeric operation or a source trace.
        let word = 0x3f80_0000 + u32::try_from(index).expect("small fixture");
        row.vectors = [[f32::from_bits(word), -0.0, f32::from_bits(0x7fc0_1234), 7.0]; 4];
    }
}

#[test]
fn empty_feedback_rings_have_no_anchor_rf_payload_or_body_effects() {
    for capacity in [0, 3] {
        let mut pool = SimulationRigidBodyPool::try_new(capacity).unwrap();
        seed_rf(&mut pool);
        let before = pool.snapshot();
        let rows = rf_words(&pool);
        pool.try_reset_static_reaction_forces().unwrap();
        assert_eq!(pool.try_freeze_if_necessary(0).unwrap(), 0);
        assert_eq!(pool.snapshot(), before);
        assert_eq!(rf_words(&pool), rows);
    }
}

#[test]
fn static_reset_clears_exact_rows_including_registered_static_and_keeps_bodies() {
    let mut pool = SimulationRigidBodyPool::try_new(6).unwrap();
    let first = add(&mut pool, 1);
    let active = add(&mut pool, 4);
    let second = add(&mut pool, 1);
    let frozen = add(&mut pool, 2);
    pool.try_register_contact_spy(second).unwrap();
    pool.try_remove_rigid_body(first).unwrap();
    let last = add(&mut pool, 1);
    seed_rf(&mut pool);
    let before = pool.snapshot();
    let rows = rf_words(&pool);
    assert_eq!(before.static_order, vec![second.id(), last.id()]);
    pool.try_reset_static_reaction_forces().unwrap();
    assert_eq!(pool.snapshot(), before);
    for (index, row) in rf_words(&pool).iter().enumerate() {
        if index == second.id().0 as usize || index == last.id().0 as usize {
            assert_eq!(*row, [[0; 4]; 4]);
        } else {
            assert_eq!(*row, rows[index]);
        }
    }
    assert_eq!(pool.slots[second.id().0 as usize].state, 9);
    assert_eq!(pool.slots[active.id().0 as usize].state, 4);
    assert_eq!(pool.slots[frozen.id().0 as usize].state, 2);
}

#[test]
fn freeze_uses_unsigned_threshold_separate_from_current_replacement_cooldown() {
    let mut pool = SimulationRigidBodyPool::try_new(5).unwrap();
    let bodies = std::array::from_fn::<_, 5, _>(|_| add(&mut pool, 4));
    for (body, cooldown) in bodies
        .iter()
        .zip([0, 0x7fff_ffff, 0x8000_0000, 0xffff_fffe, u32::MAX])
    {
        pool.slots[body.id().0 as usize].cooldown = cooldown;
    }
    pool.set_cooldown(17);
    assert_eq!(pool.try_freeze_if_necessary(0x8000_0000).unwrap(), 3);
    assert_eq!(
        pool.snapshot().active_order,
        bodies[..2].iter().map(|body| body.id()).collect::<Vec<_>>()
    );
    assert_eq!(
        pool.snapshot().frozen_order,
        bodies[2..].iter().map(|body| body.id()).collect::<Vec<_>>()
    );
    for body in &bodies[2..] {
        assert_eq!(pool.slots[body.id().0 as usize].cooldown, 17);
    }
    assert_eq!(pool.try_freeze_if_necessary(u32::MAX).unwrap(), 0);
    assert_eq!(pool.try_freeze_if_necessary(0).unwrap(), 2);
    pool.validate_integrity().unwrap();
}

#[test]
fn freeze_captures_first_hundred_in_actual_active_order_before_mutating_rings() {
    let mut pool = SimulationRigidBodyPool::try_new(105).unwrap();
    let old_frozen = add(&mut pool, 2);
    let mut bodies: Vec<_> = (0..104).map(|_| add(&mut pool, 4)).collect();
    let removed = bodies.remove(3);
    pool.try_remove_rigid_body(removed).unwrap();
    let replacement = add(&mut pool, 4);
    assert_eq!(removed.id(), replacement.id());
    assert_ne!(removed.observation(), replacement.observation());
    bodies.push(replacement);
    for body in &bodies {
        pool.slots[body.id().0 as usize].cooldown = 2;
    }
    let expected: Vec<_> = bodies.iter().map(|body| body.id()).collect();
    assert_eq!(pool.snapshot().active_order, expected);
    pool.set_cooldown(0);
    assert_eq!(pool.try_freeze_if_necessary(2).unwrap(), 100);
    let after = pool.snapshot();
    assert_eq!(after.active_order, expected[100..]);
    assert_eq!(
        after.frozen_order,
        std::iter::once(old_frozen.id())
            .chain(expected[..100].iter().copied())
            .collect::<Vec<_>>()
    );
    assert_eq!(pool.try_freeze_if_necessary(2).unwrap(), 4);
    assert_eq!(
        pool.snapshot().frozen_order,
        std::iter::once(old_frozen.id())
            .chain(expected)
            .collect::<Vec<_>>()
    );
    assert_eq!(pool.try_freeze_if_necessary(0).unwrap(), 0);
    pool.validate_integrity().unwrap();
}

#[test]
fn freeze_preserves_occurrence_inertia_motion_rf_and_spy_except_source_write_set() {
    let mut pool = SimulationRigidBodyPool::try_new(4).unwrap();
    let fixed = add(&mut pool, 1);
    let old_frozen = add(&mut pool, 2);
    let plain = add(&mut pool, 4);
    let registered = add(&mut pool, 4);
    pool.try_register_contact_spy(registered).unwrap();
    for body in [plain, registered] {
        let slot = &mut pool.slots[body.id().0 as usize];
        slot.velocity = [2.0, -3.0, 4.0];
        slot.omega = [-1.0, 5.0, 2.0];
        slot.force = [4.0, 2.0, 1.0];
        slot.torque = [6.0, 8.0, 9.0];
        slot.kinematic_value = -2.0;
        slot.tag = 0x8123_4567;
        slot.cooldown = 5;
    }
    seed_rf(&mut pool);
    let before = pool.snapshot();
    let rows = rf_words(&pool);
    let plain_inertia = Arc::clone(pool.try_inertia(plain).unwrap().unwrap());
    let spy_inertia = Arc::clone(pool.try_inertia(registered).unwrap().unwrap());
    pool.set_cooldown(123);
    assert_eq!(pool.try_freeze_if_necessary(5).unwrap(), 2);
    let after = pool.snapshot();
    assert_eq!(after.static_order, vec![fixed.id()]);
    assert_eq!(
        after.frozen_order,
        vec![old_frozen.id(), plain.id(), registered.id()]
    );
    assert_eq!(after.next_occurrence, before.next_occurrence);
    assert_eq!(
        after.next_dynamic_update_phase_identity,
        before.next_dynamic_update_phase_identity
    );
    assert_eq!(rf_words(&pool), rows);
    assert!(Arc::ptr_eq(
        &plain_inertia,
        pool.try_inertia(plain).unwrap().unwrap()
    ));
    assert!(Arc::ptr_eq(
        &spy_inertia,
        pool.try_inertia(registered).unwrap().unwrap()
    ));
    for (old, new) in before.slots.iter().zip(&after.slots) {
        let mut expected = old.clone();
        // All neighbor-link changes are confined to the actual ring operation;
        // exact resulting ring order is independently asserted above.
        expected.left_link = new.left_link;
        expected.right_link = new.right_link;
        if [plain.id(), registered.id()].contains(&old.id) {
            expected.state = (old.state & 8) | 2;
            expected.membership = SimulationRigidBodyList::Frozen;
            expected.cooldown = 123;
        }
        assert_eq!(new, &expected);
    }
    assert_eq!(after.slots[plain.id().0 as usize].state, 2);
    assert_eq!(after.slots[registered.id().0 as usize].state, 10);
    pool.validate_integrity().unwrap();
}

#[test]
fn feedback_topology_refusal_precedes_every_rf_and_freeze_effect() {
    let mut pool = SimulationRigidBodyPool::try_new(3).unwrap();
    let fixed = add(&mut pool, 1);
    let active = add(&mut pool, 4);
    seed_rf(&mut pool);
    // Private structural corruption is only a refusal discriminator, never an
    // admitted production body or nullable RF capability.
    pool.slots[fixed.id().0 as usize].state = 4;
    let before = pool.snapshot();
    let rows = rf_words(&pool);
    assert_eq!(
        pool.try_reset_static_reaction_forces(),
        Err(SimulationRigidBodyPoolError::CorruptStructure)
    );
    assert_eq!(
        pool.try_freeze_if_necessary(0),
        Err(SimulationRigidBodyPoolError::CorruptStructure)
    );
    assert_eq!(pool.snapshot(), before);
    assert_eq!(rf_words(&pool), rows);
    assert_eq!(
        pool.slots[active.id().0 as usize].membership,
        SimulationRigidBodyList::Active
    );
}
