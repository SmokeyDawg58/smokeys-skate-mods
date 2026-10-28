//! Synthetic ownership/numeric discriminators on the actual pool, not native traces.
#![expect(
    clippy::float_cmp,
    reason = "exact copied values and representable source arithmetic"
)]
use super::*;
use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM as FRAME;

fn add(
    pool: &mut SimulationRigidBodyPool,
    owner: &SimulationMutableInertia,
    state: u32,
) -> SimulationRigidBodyHandle {
    pool.try_add_rigid_body_mutable(FRAME, Some(owner), state)
        .unwrap()
        .unwrap()
}

fn poison(owner: &SimulationMutableInertia) {
    let result = std::panic::catch_unwind(|| {
        let _guard = owner.0.lock().unwrap();
        panic!("synthetic private fixture poison");
    });
    assert!(result.is_err());
}

#[test]
fn immutable_add_keeps_exact_arc_and_mutable_accessor_refuses_explicitly() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let immutable = Arc::new(SimulationInertia::native_default());
    let first = pool
        .try_add_rigid_body(FRAME, Some(&immutable), 4)
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(
        pool.try_inertia(first).unwrap().unwrap(),
        &immutable
    ));
    let owner = SimulationMutableInertia::new(*immutable);
    let second = add(&mut pool, &owner, 4);
    assert_eq!(
        pool.try_inertia(second),
        Err(SimulationRigidBodyPoolError::MutableInertiaRequiresSnapshot { slot: second.id.0 })
    );
    assert_eq!(
        pool.try_write_mutable_inertia(first, &owner, *immutable),
        Err(SimulationRigidBodyPoolError::ImmutableInertia { slot: first.id.0 })
    );
}

#[test]
fn live_write_updates_shared_storage_only_not_cached_body_or_rf() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let first = add(&mut pool, &owner, 4);
    let second = add(&mut pool, &owner, 2);
    pool.solver_delta_rows[0].vectors = [[17.0; 4]; 4];
    let before = pool.snapshot();
    let rf = pool.solver_delta_rows.clone();
    let mut changed = owner.try_read().unwrap();
    changed.inverse_tensor = [2.0, 4.0, 8.0];
    changed.inverse_mass = 0.25;
    changed.linear_drag = 0.5;
    pool.try_write_mutable_inertia(first, &owner, changed)
        .unwrap();
    assert_eq!(owner.try_read().unwrap(), changed);
    let mut expected = before;
    expected.slots[first.id.0 as usize].inertia = Some(changed);
    expected.slots[second.id.0 as usize].inertia = Some(changed);
    assert_eq!(pool.snapshot(), expected);
    assert_eq!(pool.solver_delta_rows, rf);
}

#[test]
fn actual_dynamic_update_uses_new_drag_and_tensor_but_retains_cached_inverse_mass() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    pool.set_gravity([0.0; 3]);
    pool.begin_dynamic_update_frame(1.0);
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let body = add(&mut pool, &owner, 4);
    let index = body.id.0 as usize;
    pool.slots[index].velocity = [4.0, 0.0, 0.0];
    let mut changed = owner.try_read().unwrap();
    changed.linear_drag = 0.5;
    changed.inverse_tensor = [2.0; 3];
    changed.inverse_mass = 0.125;
    pool.try_write_mutable_inertia(body, &owner, changed)
        .unwrap();
    let phase = pool.try_prepare_dynamic_update_phase().unwrap();
    assert_eq!(phase.active[0].inertia, changed);
    phase.integrate_ordinary_finite();
    let slot = &pool.snapshot().slots[index];
    assert_eq!(slot.velocity, [2.0, 0.0, 0.0]);
    assert_eq!(slot.center_of_mass, [4.0, 0.0, 0.0]);
    assert_eq!(
        slot.inverse_mass, 1.0,
        "inertia owner write is not a live mInvm store"
    );
    assert_eq!(slot.inverse_inertia_full, [2.0, 0.0, 0.0]);
    assert_eq!(slot.inverse_inertia_split, [2.0, 2.0, 0.0]);
    assert_eq!(pool.solver_delta_rows[index].vectors, [[0.0; 4]; 4]);
}

#[test]
fn part_transform_samples_current_owner_and_refreshes_derived_fields_at_that_call() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let body = add(&mut pool, &owner, 4);
    let mut changed = owner.try_read().unwrap();
    changed.inverse_tensor = [3.0; 3];
    changed.inverse_mass = 0.25;
    pool.try_write_mutable_inertia(body, &owner, changed)
        .unwrap();
    assert_eq!(pool.snapshot().slots[0].inverse_mass, 1.0);
    pool.try_set_part_body_transform_host(body, FRAME).unwrap();
    let snapshot = pool.snapshot();
    assert_eq!(snapshot.slots[0].inverse_mass, 0.25);
    assert_eq!(snapshot.slots[0].inverse_inertia_full, [3.0, 0.0, 0.0]);
    assert_eq!(snapshot.slots[0].inverse_inertia_split, [3.0, 3.0, 0.0]);
}

#[test]
fn wrong_owner_foreign_stale_and_free_handles_never_write() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let wrong = SimulationMutableInertia::new(SimulationInertia::native_default());
    let body = add(&mut pool, &owner, 4);
    let value = SimulationInertia {
        inverse_mass: 7.0,
        ..SimulationInertia::native_default()
    };
    let before = pool.snapshot();
    assert_eq!(
        pool.try_write_mutable_inertia(body, &wrong, value),
        Err(SimulationRigidBodyPoolError::InertiaOwnerMismatch { slot: 0 })
    );
    let mut other = SimulationRigidBodyPool::try_new(1).unwrap();
    assert_eq!(
        other.try_write_mutable_inertia(body, &owner, value),
        Err(SimulationRigidBodyPoolError::ForeignHandle)
    );
    assert_eq!(pool.snapshot(), before);
    pool.try_remove_rigid_body(body).unwrap();
    assert_eq!(
        pool.try_write_mutable_inertia(body, &owner, value),
        Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
    );
    let fresh = add(&mut pool, &owner, 4);
    assert_ne!(fresh.occurrence, body.occurrence);
    assert_eq!(
        pool.try_write_mutable_inertia(body, &owner, value),
        Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
    );
    assert_eq!(
        owner.try_read().unwrap(),
        SimulationInertia::native_default()
    );
}

#[test]
fn permanent_pool_binding_prevents_alias_mutation_across_phase_domains() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let body = add(&mut pool, &owner, 4);
    let mut other = SimulationRigidBodyPool::try_new(1).unwrap();
    let before = other.snapshot();
    assert_eq!(
        other.try_add_rigid_body_mutable(FRAME, Some(&owner), 4),
        Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch)
    );
    assert_eq!(other.snapshot(), before);
    pool.try_remove_rigid_body(body).unwrap();
    assert_eq!(
        other.try_add_rigid_body_mutable(FRAME, Some(&owner), 2),
        Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch)
    );
}

#[test]
fn capacity_and_static_skip_poisoned_payload_while_nonstatic_failure_is_atomic() {
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    poison(&owner);
    let mut empty = SimulationRigidBodyPool::try_new(0).unwrap();
    assert_eq!(
        empty
            .try_add_rigid_body_mutable(FRAME, Some(&owner), 4)
            .unwrap(),
        None
    );
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let static_body = add(&mut pool, &owner, 1);
    assert!(pool.try_inertia(static_body).unwrap().is_none());
    let before = pool.snapshot();
    assert_eq!(
        pool.try_add_rigid_body_mutable(FRAME, Some(&owner), 4),
        Err(SimulationRigidBodyPoolError::InertiaLockPoisoned)
    );
    assert_eq!(pool.snapshot(), before);
}

#[test]
fn poisoned_live_payload_has_checked_errors_without_pose_rf_or_phase_effects() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let body = add(&mut pool, &owner, 4);
    let before_pose = pool.try_rigid_body_pose(body).unwrap();
    let before_phase = pool.next_dynamic_update_phase_identity;
    let before_rf = pool.solver_delta_rows.clone();
    poison(&owner);
    assert_eq!(
        pool.try_snapshot(),
        Err(SimulationRigidBodyPoolError::InertiaLockPoisoned)
    );
    assert!(std::panic::catch_unwind(|| pool.snapshot()).is_err());
    assert_eq!(
        pool.try_set_part_body_transform_host(body, [0.0; 16]),
        Err(SimulationRigidBodyPoolError::InertiaLockPoisoned)
    );
    assert_eq!(
        pool.try_write_mutable_inertia(body, &owner, SimulationInertia::native_default()),
        Err(SimulationRigidBodyPoolError::InertiaLockPoisoned)
    );
    assert!(matches!(
        pool.try_prepare_dynamic_update_phase(),
        Err(SimulationRigidBodyDynamicUpdatePrepareError::Pool(
            SimulationRigidBodyPoolError::InertiaLockPoisoned
        ))
    ));
    assert_eq!(pool.try_rigid_body_pose(body).unwrap(), before_pose);
    assert_eq!(pool.next_dynamic_update_phase_identity, before_phase);
    assert_eq!(pool.solver_delta_rows, before_rf);
}

#[test]
fn packed_owner_preserves_raw_w_across_xyz_writes_and_shared_body_reads() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let initial = SimulationInertia::native_default();
    let owner = SimulationMutableInertia::new_packed(initial, -0.0);
    let first = add(&mut pool, &owner, 4);
    let second = add(&mut pool, &owner, 2);
    assert_eq!(
        owner.try_read_packed().unwrap().unwrap().map(f32::to_bits),
        [
            1.0_f32.to_bits(),
            1.0_f32.to_bits(),
            1.0_f32.to_bits(),
            0x8000_0000
        ]
    );
    pool.solver_delta_rows[0].vectors = [[19.0; 4]; 4];
    let before = pool.snapshot();
    let before_rf = pool.solver_delta_rows.clone();
    let mut changed = initial;
    changed.inverse_tensor = [2.0, 4.0, 8.0];
    changed.inverse_mass = 0.25;
    pool.try_write_mutable_inertia(first, &owner, changed)
        .unwrap();
    assert_eq!(
        owner.try_read_packed().unwrap().unwrap().map(f32::to_bits),
        [
            2.0_f32.to_bits(),
            4.0_f32.to_bits(),
            8.0_f32.to_bits(),
            0x8000_0000
        ]
    );

    let payload_nan = f32::from_bits(0xffc1_2345);
    changed.angular_drag = 0.5;
    pool.try_write_mutable_packed_inertia(second, &owner, changed, payload_nan)
        .unwrap();
    changed.linear_drag = 0.75;
    pool.try_write_mutable_inertia(first, &owner, changed)
        .unwrap();
    assert_eq!(owner.try_read().unwrap(), changed);
    assert_eq!(
        owner.try_read_packed().unwrap().unwrap().map(f32::to_bits),
        [
            2.0_f32.to_bits(),
            4.0_f32.to_bits(),
            8.0_f32.to_bits(),
            0xffc1_2345
        ]
    );
    let mut expected = before;
    expected.slots[first.id.0 as usize].inertia = Some(changed);
    expected.slots[second.id.0 as usize].inertia = Some(changed);
    assert_eq!(pool.snapshot(), expected);
    assert_eq!(pool.solver_delta_rows, before_rf);
    let phase = pool.try_prepare_dynamic_update_phase().unwrap();
    assert_eq!(phase.active[0].inertia, changed);
    assert_eq!(
        owner.try_read_packed().unwrap().unwrap()[3].to_bits(),
        0xffc1_2345,
        "XYZ DynamicUpdate sampling neither includes nor overwrites packed W"
    );
}

#[test]
fn packed_write_refuses_unknown_w_without_promoting_or_changing_xyz() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let initial = SimulationInertia::native_default();
    let owner = SimulationMutableInertia::new(initial);
    let body = add(&mut pool, &owner, 4);
    assert_eq!(owner.try_read_packed().unwrap(), None);
    let before = pool.snapshot();
    let changed = SimulationInertia {
        inverse_tensor: [7.0; 3],
        inverse_mass: 0.125,
        ..initial
    };
    assert_eq!(
        pool.try_write_mutable_packed_inertia(body, &owner, changed, -0.0),
        Err(SimulationRigidBodyPoolError::PackedInertiaUnavailable { slot: 0 })
    );
    assert_eq!(owner.try_read().unwrap(), initial);
    assert_eq!(owner.try_read_packed().unwrap(), None);
    assert_eq!(pool.snapshot(), before);
    pool.try_write_mutable_inertia(body, &owner, changed)
        .unwrap();
    assert_eq!(owner.try_read().unwrap(), changed);
    assert_eq!(owner.try_read_packed().unwrap(), None);
}

#[test]
fn packed_write_reuses_identity_authority_and_occurrence_checks_atomically() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let initial = SimulationInertia::native_default();
    let owner = SimulationMutableInertia::new_packed(initial, f32::from_bits(0x7fc5_4321));
    let wrong = SimulationMutableInertia::new_packed(initial, 9.0);
    let body = add(&mut pool, &owner, 4);
    let changed = SimulationInertia {
        inverse_mass: 7.0,
        ..initial
    };
    let before = pool.snapshot();
    assert_eq!(
        pool.try_write_mutable_packed_inertia(body, &wrong, changed, 8.0),
        Err(SimulationRigidBodyPoolError::InertiaOwnerMismatch { slot: 0 })
    );
    let mut foreign = SimulationRigidBodyPool::try_new(1).unwrap();
    assert_eq!(
        foreign.try_write_mutable_packed_inertia(body, &owner, changed, 8.0),
        Err(SimulationRigidBodyPoolError::ForeignHandle)
    );
    assert_eq!(pool.snapshot(), before);
    pool.try_remove_rigid_body(body).unwrap();
    assert_eq!(
        pool.try_write_mutable_packed_inertia(body, &owner, changed, 8.0),
        Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
    );
    let replacement = add(&mut pool, &owner, 4);
    assert_ne!(replacement.occurrence, body.occurrence);
    let before = pool.snapshot();
    assert_eq!(
        pool.try_write_mutable_packed_inertia(body, &owner, changed, 8.0),
        Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
    );
    assert_eq!(pool.snapshot(), before);
    assert_eq!(owner.try_read().unwrap(), initial);
    assert_eq!(
        owner.try_read_packed().unwrap().unwrap()[3].to_bits(),
        0x7fc5_4321
    );
    assert_eq!(wrong.try_read_packed().unwrap().unwrap()[3], 9.0);
}

#[test]
fn poisoned_packed_owner_keeps_xyz_and_w_without_body_or_rf_stores() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let initial = SimulationInertia::native_default();
    let owner = SimulationMutableInertia::new_packed(initial, -0.0);
    let body = add(&mut pool, &owner, 4);
    let before_pose = pool.try_rigid_body_pose(body).unwrap();
    let before_rf = pool.solver_delta_rows.clone();
    poison(&owner);
    let changed = SimulationInertia {
        inverse_mass: 7.0,
        ..initial
    };
    assert_eq!(
        pool.try_write_mutable_packed_inertia(body, &owner, changed, 5.0),
        Err(SimulationRigidBodyPoolError::InertiaLockPoisoned)
    );
    assert_eq!(
        owner.try_read_packed(),
        Err(SimulationRigidBodyPoolError::InertiaLockPoisoned)
    );
    assert_eq!(
        owner.try_read(),
        Err(SimulationRigidBodyPoolError::InertiaLockPoisoned)
    );
    // Test-only inspection of poisoned storage, not a production recovery API.
    let state = owner.0.lock().unwrap_err().into_inner();
    assert_eq!(state.value, initial);
    assert_eq!(state.inverse_tensor_w.unwrap().to_bits(), 0x8000_0000);
    assert_eq!(pool.try_rigid_body_pose(body).unwrap(), before_pose);
    assert_eq!(pool.solver_delta_rows, before_rf);
}
