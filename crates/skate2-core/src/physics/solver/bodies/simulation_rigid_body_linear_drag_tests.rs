use super::*;
use crate::{
    simulation_rigid_body_pool::{SimulationInertia, SimulationMutableInertia},
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
use std::sync::Arc;

fn add(
    pool: &mut SimulationRigidBodyPool,
) -> (SimulationMutableInertia, SimulationRigidBodyObservation) {
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let handle = pool
        .try_add_rigid_body_mutable(FRESH_VOLUME_TRANSFORM, Some(&owner), 4)
        .unwrap()
        .unwrap();
    (owner, pool.try_observe(handle).unwrap())
}

#[test]
fn actual_shared_inertia_changes_only_drag_and_preserves_unknown_tensor_lane() {
    let mut pool = SimulationRigidBodyPool::try_new(4).unwrap();
    let entries = std::array::from_fn::<_, 4, _>(|_| add(&mut pool));
    let bodies = entries.each_ref().map(|(_, body)| *body);
    for bits in [0x8000_0000, 0x3f80_0000, 0x7fc1_2345] {
        let before = pool.snapshot();
        pool.try_set_part_group_linear_drag(&bodies, f32::from_bits(bits))
            .unwrap();
        let mut after = pool.snapshot();
        for (slot, original) in after.slots.iter_mut().zip(&before.slots) {
            assert_eq!(slot.inertia.as_ref().unwrap().linear_drag.to_bits(), bits);
            slot.inertia.as_mut().unwrap().linear_drag =
                original.inertia.as_ref().unwrap().linear_drag;
        }
        // Last iteration starts from a finite drag so snapshot equality remains
        // an exact unchanged-fields discriminator even when the new drag is NaN.
        assert_eq!(after, before);
        for (owner, _) in &entries {
            assert_eq!(owner.try_read().unwrap().linear_drag.to_bits(), bits);
            assert!(owner.try_read_packed().unwrap().is_none());
        }
    }
}

#[test]
fn wheel_angular_store_changes_only_current_scalar_and_checks_occurrence() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let (owner, body) = add(&mut pool);
    let (_, untouched) = add(&mut pool);
    for bits in [0x8000_0000, 0x3f80_0000, 0x7fc1_2345] {
        let before = pool.snapshot();
        pool.try_set_part_angular_drag(body, f32::from_bits(bits))
            .unwrap();
        let mut after = pool.snapshot();
        assert_eq!(
            after.slots[0]
                .inertia
                .as_ref()
                .unwrap()
                .angular_drag
                .to_bits(),
            bits
        );
        after.slots[0].inertia.as_mut().unwrap().angular_drag =
            before.slots[0].inertia.as_ref().unwrap().angular_drag;
        assert_eq!(after, before);
        assert_eq!(owner.try_read().unwrap().angular_drag.to_bits(), bits);
        assert!(owner.try_read_packed().unwrap().is_none());
    }
    let mut foreign_pool = SimulationRigidBodyPool::try_new(1).unwrap();
    assert!(
        foreign_pool
            .try_set_part_angular_drag(untouched, 3.0)
            .is_err()
    );
}

#[test]
fn captured_inertia_rejects_other_live_body_owner_before_store() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let (_, first) = add(&mut pool);
    let (_, second) = add(&mut pool);
    let captured = pool.try_capture_part_inertia(first).unwrap();
    let before = pool.snapshot();
    assert_eq!(
        pool.try_set_captured_angular_drag(second, &captured, 9.0),
        Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch)
    );
    assert_eq!(pool.snapshot(), before);
    pool.try_set_captured_angular_drag(first, &captured, -0.0)
        .unwrap();
    assert_eq!(
        captured.try_read().unwrap().angular_drag.to_bits(),
        0x8000_0000
    );
}

#[test]
fn missing_fourth_mutable_owner_prevents_first_three_writes() {
    let mut pool = SimulationRigidBodyPool::try_new(4).unwrap();
    let entries = std::array::from_fn::<_, 3, _>(|_| add(&mut pool));
    let immutable = Arc::new(SimulationInertia::native_default());
    let last = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&immutable), 4)
        .unwrap()
        .unwrap();
    let before = pool.snapshot();
    let bodies = [
        entries[0].1,
        entries[1].1,
        entries[2].1,
        pool.try_observe(last).unwrap(),
    ];
    assert!(matches!(
        pool.try_set_part_group_linear_drag(&bodies, 17.0),
        Err(SimulationRigidBodyPoolError::ImmutableInertia { .. })
    ));
    assert_eq!(pool.snapshot(), before);
    let mut other = SimulationRigidBodyPool::try_new(1).unwrap();
    let (_, foreign) = add(&mut other);
    assert!(
        pool.try_set_part_group_linear_drag(&[entries[0].1, foreign], 17.0)
            .is_err()
    );
    assert_eq!(pool.snapshot(), before);
}

#[test]
fn later_write_lock_failure_retains_earlier_scalar_store() {
    let mut pool = SimulationRigidBodyPool::try_new(3).unwrap();
    let entries = std::array::from_fn::<_, 3, _>(|_| add(&mut pool));
    let poisoned = entries[1].0.clone();
    let _ = std::panic::catch_unwind(|| {
        let _guard = poisoned.0.lock().unwrap();
        panic!("controlled late write-lock poison");
    });
    assert!(matches!(
        pool.try_set_part_group_linear_drag(&entries.each_ref().map(|(_, body)| *body), 23.0),
        Err(SimulationRigidBodyPoolError::InertiaLockPoisoned)
    ));
    assert_eq!(
        entries[0].0.try_read().unwrap().linear_drag.to_bits(),
        23.0_f32.to_bits()
    );
    assert_eq!(entries[2].0.try_read().unwrap().linear_drag.to_bits(), 0);
}
