use super::*;
use crate::{
    simulation_rigid_body_pool::SimulationRigidBodyHandle,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
fn add(pool: &mut SimulationRigidBodyPool, state: u32) -> SimulationRigidBodyHandle {
    pool.try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, state)
        .unwrap()
        .unwrap()
}
#[test]
fn frozen_transition_appends_active_tail_preserves_payload_and_spy_bit() {
    let mut pool = SimulationRigidBodyPool::try_new(4).unwrap();
    let active = add(&mut pool, 4);
    let first = add(&mut pool, 2);
    let second = add(&mut pool, 2);
    let index = first.id().index() as usize;
    pool.slots[index].state = 2 | crate::simulation_rigid_body_pool::SPY_BODY;
    pool.slots[index].force = [3.0, 5.0, 7.0];
    pool.slots[index].tag = 42;
    let before = (
        pool.slots[index].occurrence,
        pool.slots[index].force,
        pool.slots[index].tag,
    );
    let observation = pool.try_observe(first).unwrap();
    pool.try_activate_frozen_body(observation).unwrap();
    assert_eq!(
        pool.list_order(SimulationRigidBodyList::Active),
        vec![active.id(), first.id()]
    );
    assert_eq!(
        pool.list_order(SimulationRigidBodyList::Frozen),
        vec![second.id()]
    );
    assert_eq!(pool.counts.frozen, 1);
    assert_eq!(pool.counts.active, 2);
    let after = &pool.slots[index];
    assert_eq!(after.state, 12);
    assert_eq!(after.cooldown, pool.cooldown - 1);
    assert_eq!(after.occurrence, before.0);
    assert_eq!(after.force.map(f32::to_bits), before.1.map(f32::to_bits));
    assert_eq!(after.tag, before.2);
    assert_eq!(pool.try_observe(first).unwrap(), observation);
    pool.validate_integrity().unwrap();
}
#[test]
fn zero_configured_cooldown_wraps_then_explicit_assembly_store_clears() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    pool.set_cooldown(0);
    let body = add(&mut pool, 2);
    let observation = pool.try_observe(body).unwrap();
    pool.try_activate_frozen_body(observation).unwrap();
    assert_eq!(pool.slots[body.id().index() as usize].cooldown, u32::MAX);
    pool.try_clear_body_cooldown(observation).unwrap();
    assert_eq!(pool.slots[body.id().index() as usize].cooldown, 0);
}
#[test]
fn active_and_static_refuse_frozen_primitive_but_accept_only_cooldown_store() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    for state in [1, 4] {
        let body = add(&mut pool, state);
        let observation = pool.try_observe(body).unwrap();
        let index = body.id().index() as usize;
        pool.slots[index].cooldown = 29;
        let order = pool.list_order(pool.slots[index].membership);
        assert_eq!(
            pool.try_activate_frozen_body(observation),
            Err(SimulationRigidBodyPoolError::BodyNotFrozen {
                slot: body.id().index(),
                state
            })
        );
        assert_eq!(pool.slots[index].cooldown, 29);
        pool.try_clear_body_cooldown(observation).unwrap();
        assert_eq!(pool.slots[index].cooldown, 0);
        assert_eq!(pool.list_order(pool.slots[index].membership), order);
    }
}
#[test]
fn foreign_free_and_reused_occurrences_cannot_mutate_activation_state() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let body = add(&mut pool, 2);
    let old = pool.try_observe(body).unwrap();
    let mut other = SimulationRigidBodyPool::try_new(1).unwrap();
    assert_eq!(
        other.try_activate_frozen_body(old),
        Err(SimulationRigidBodyPoolError::ForeignHandle)
    );
    pool.try_remove_rigid_body(body).unwrap();
    assert!(pool.try_activate_frozen_body(old).is_err());
    assert!(pool.try_clear_body_cooldown(old).is_err());
    let replacement = add(&mut pool, 2);
    assert_eq!(
        pool.try_activate_frozen_body(old),
        Err(SimulationRigidBodyPoolError::StaleHandle {
            slot: body.id().index()
        })
    );
    assert_eq!(pool.slots[replacement.id().index() as usize].state & 7, 2);
    assert_eq!(pool.counts.frozen, 1);
    assert_eq!(pool.counts.active, 0);
}

#[test]
fn unsupported_raw_state_bits_refuse_under_existing_pool_integrity_contract() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let body = add(&mut pool, 2);
    let observation = pool.try_observe(body).unwrap();
    let index = body.id().index() as usize;
    // No native producer in this pool publishes these bits. The native mask
    // still discards them, but host integrity refuses such an invented owner.
    pool.validated_topology.take(); // Test-only unsupported state injection.
    pool.slots[index].state = 0xabcd_001a;
    let cooldown = pool.slots[index].cooldown;
    assert_eq!(
        pool.try_activate_frozen_body(observation),
        Err(SimulationRigidBodyPoolError::CorruptStructure)
    );
    assert_eq!(pool.slots[index].state, 0xabcd_001a);
    assert_eq!(pool.slots[index].cooldown, cooldown);
    assert_eq!(pool.counts.frozen, 1);
    assert_eq!(pool.counts.active, 0);
}
