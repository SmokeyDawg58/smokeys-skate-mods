//! Synthetic storage/lifecycle tests using the actual Joint pool.
#![expect(
    clippy::float_cmp,
    reason = "source values and untouched fields are copied exactly"
)]
use super::*;
use crate::simulation_rigid_body_pool::SimulationRigidBodyPool;

type Pool = SimulationJointPool<SimulationJointFrames, SimulationJointLimits>;

fn bodies() -> (SimulationRigidBodyPool, [SimulationRigidBodyHandle; 2]) {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let handles = std::array::from_fn(|_| {
        pool.try_add_rigid_body(crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM, None, 4)
            .unwrap()
            .unwrap()
    });
    (pool, handles)
}

fn frames() -> Arc<SimulationJointFrames> {
    Arc::new(SimulationJointFrames {
        quaternion_a: [0.0, 0.0, 0.0, 1.0],
        position_a: [2.0; 4],
        quaternion_b: [0.0, 0.0, 0.0, 1.0],
        position_b: [3.0; 4],
        quaternion_l: [0.0, 0.0, 0.0, 1.0],
    })
}

fn limits() -> SimulationJointLimits {
    SimulationJointLimits {
        p_prism: [1.0, 2.0, 3.0, -0.0],
        v_prism: [4.0, 5.0, 6.0, 7.0],
        v_twist: 8.0,
        v_swing: 9.0,
        swing_a: 0.25,
        twist_a: 0.5,
        swing_c: 0.75,
        twist_c: 1.0,
        raw_swing_type: 3,
        raw_twist_type: 2,
    }
}

fn add(
    pool: &mut Pool,
    bodies: [SimulationRigidBodyHandle; 2],
    frames: &Arc<SimulationJointFrames>,
    owner: &SimulationMutableJointLimits,
) -> SimulationJointHandle {
    pool.try_add_joint_mutable_limits(bodies[0], bodies[1], frames, owner)
        .unwrap()
        .unwrap()
}

fn active_limits(pool: &Pool) -> Vec<SimulationJointLimits> {
    pool.try_with_active_build_occurrences_and_reservation(
        |occurrences| {
            occurrences
                .map(|(_, occurrence)| occurrence.try_limits())
                .collect::<Result<Vec<_>, _>>()
        },
        |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
    )
    .unwrap_or_else(|_| panic!("readable actual Active occurrences"))
}

pub(crate) fn poison(owner: &SimulationMutableJointLimits) {
    assert!(
        std::panic::catch_unwind(|| {
            let _guard = owner.0.lock().unwrap();
            panic!("synthetic private poison fixture");
        })
        .is_err()
    );
}

#[test]
fn immutable_generic_api_keeps_original_arc_and_mutable_binding_refuses_borrowed_view() {
    let (_bodies, body) = bodies();
    let mut pool = Pool::try_new(2).unwrap();
    let frames = frames();
    let original = Arc::new(limits());
    let first = pool
        .try_add_joint(body[0], body[1], &frames, &original)
        .unwrap()
        .unwrap();
    let binding = pool.try_binding(first).unwrap();
    assert!(Arc::ptr_eq(binding.frames, &frames));
    assert!(Arc::ptr_eq(binding.limits, &original));
    let owner = SimulationMutableJointLimits::new(*original);
    let second = add(&mut pool, body, &frames, &owner);
    assert!(matches!(
        pool.try_binding(second),
        Err(SimulationJointPoolError::MutableLimitsRequireSnapshot { slot: 1 })
    ));
    assert_eq!(
        pool.try_write_mutable_limits(first, &owner, limits()),
        Err(SimulationJointPoolError::ImmutableLimits { slot: 0 })
    );
    assert_eq!(pool.try_limits(first).unwrap(), *original);
    assert_eq!(pool.try_limits(second).unwrap(), *original);
}

#[test]
fn shared_alias_write_is_visible_to_actual_active_occurrences_without_other_mutation() {
    let (_bodies, body) = bodies();
    let mut pool = Pool::try_new(2).unwrap();
    let frames = frames();
    let owner = SimulationMutableJointLimits::new(limits());
    let first = add(&mut pool, body, &frames, &owner);
    let second = add(&mut pool, [body[1], body[0]], &frames, &owner);
    pool.slots[0].tag = 17;
    pool.slots[1].spy = 91;
    let before = pool.snapshot();
    let old = owner.try_read().unwrap();
    assert_eq!(active_limits(&pool), [old, old]);
    let changed = SimulationJointLimits {
        swing_a: 2.0,
        swing_c: -0.25,
        twist_a: 3.0,
        twist_c: -0.5,
        ..old
    };
    pool.try_write_mutable_limits(first, &owner, changed)
        .unwrap();
    assert_eq!(owner.try_read().unwrap(), changed);
    assert_eq!(pool.try_limits(second).unwrap(), changed);
    assert_eq!(active_limits(&pool), [changed, changed]);
    assert_eq!(pool.snapshot(), before);
    assert!(Arc::ptr_eq(pool.slots[0].frames.as_ref().unwrap(), &frames));
    assert_eq!(
        pool.try_limits(first).unwrap().p_prism.map(f32::to_bits),
        old.p_prism.map(f32::to_bits)
    );
}

#[test]
fn fifo_removal_reuse_retains_old_owner_until_overwrite_and_preserves_tag() {
    let (_bodies, body) = bodies();
    let mut pool = Pool::try_new(2).unwrap();
    let frames = frames();
    let old = SimulationMutableJointLimits::new(limits());
    let first = add(&mut pool, body, &frames, &old);
    let second = add(&mut pool, body, &frames, &old);
    pool.slots[0].tag = 37;
    assert_eq!(Arc::strong_count(&old.0), 3);
    pool.try_remove_joint(first).unwrap();
    pool.try_remove_joint(second).unwrap();
    assert_eq!(pool.snapshot().free_order, [first.id, second.id]);
    assert_eq!(Arc::strong_count(&old.0), 3);
    let new = SimulationMutableJointLimits::new(SimulationJointLimits {
        swing_a: 12.0,
        ..limits()
    });
    let reused = add(&mut pool, body, &frames, &new);
    assert_eq!(reused.id, first.id);
    assert_ne!(reused.occurrence, first.occurrence);
    assert_eq!(pool.snapshot().slots[0].tag, 37);
    assert_eq!(Arc::strong_count(&old.0), 2);
    assert_eq!(active_limits(&pool)[0].swing_a, 12.0);
    assert_eq!(
        pool.try_write_mutable_limits(reused, &old, limits()),
        Err(SimulationJointPoolError::LimitsOwnerMismatch { slot: 0 })
    );
}

#[test]
fn foreign_free_stale_and_wrong_owner_fail_without_payload_or_topology_changes() {
    let (_bodies, body) = bodies();
    let mut pool = Pool::try_new(1).unwrap();
    let frames = frames();
    let owner = SimulationMutableJointLimits::new(limits());
    let wrong = SimulationMutableJointLimits::new(limits());
    let joint = add(&mut pool, body, &frames, &owner);
    let before = pool.snapshot();
    assert_eq!(
        pool.try_write_mutable_limits(joint, &wrong, limits()),
        Err(SimulationJointPoolError::LimitsOwnerMismatch { slot: 0 })
    );
    let mut foreign = Pool::try_new(1).unwrap();
    assert_eq!(
        foreign.try_write_mutable_limits(joint, &owner, limits()),
        Err(SimulationJointPoolError::ForeignHandle)
    );
    assert_eq!(pool.snapshot(), before);
    pool.try_remove_joint(joint).unwrap();
    assert_eq!(
        pool.try_write_mutable_limits(joint, &owner, limits()),
        Err(SimulationJointPoolError::JointIsFree { slot: 0 })
    );
    add(&mut pool, body, &frames, &owner);
    assert_eq!(
        pool.try_write_mutable_limits(joint, &owner, limits()),
        Err(SimulationJointPoolError::StaleHandle { slot: 0 })
    );
    assert_eq!(owner.try_read().unwrap(), limits());
}

#[test]
fn permanent_pool_binding_rejects_cross_pool_alias_even_after_removal() {
    let (_bodies, body) = bodies();
    let frames = frames();
    let owner = SimulationMutableJointLimits::new(limits());
    let mut pool = Pool::try_new(1).unwrap();
    let joint = add(&mut pool, body, &frames, &owner);
    let mut other = Pool::try_new(1).unwrap();
    let before = other.snapshot();
    for remove in [false, true] {
        if remove {
            pool.try_remove_joint(joint).unwrap();
        }
        assert_eq!(
            other.try_add_joint_mutable_limits(body[0], body[1], &frames, &owner),
            Err(SimulationJointPoolError::LimitsOwnerPoolMismatch)
        );
        assert_eq!(other.snapshot(), before);
    }
}

#[test]
fn capacity_precedes_host_poison_check_and_nonempty_refusal_has_no_effects() {
    let (_bodies, body) = bodies();
    let owner = SimulationMutableJointLimits::new(limits());
    poison(&owner);
    let frames = frames();
    let mut zero = Pool::try_new(0).unwrap();
    assert_eq!(
        zero.try_add_joint_mutable_limits(body[0], body[1], &frames, &owner)
            .unwrap(),
        None
    );
    let mut pool = Pool::try_new(1).unwrap();
    let before = pool.snapshot();
    assert_eq!(
        pool.try_add_joint_mutable_limits(body[0], body[1], &frames, &owner),
        Err(SimulationJointPoolError::LimitsLockPoisoned)
    );
    assert_eq!(pool.snapshot(), before);
    assert_eq!(Arc::strong_count(&frames), 1);
    assert_eq!(Arc::strong_count(&owner.0), 1);
}

#[test]
fn structural_occurrence_visit_does_not_read_poisoned_limits_until_selected_read() {
    let (_bodies, body) = bodies();
    let owner = SimulationMutableJointLimits::new(limits());
    let mut pool = Pool::try_new(1).unwrap();
    let frames = frames();
    let joint = add(&mut pool, body, &frames, &owner);
    let before = pool.snapshot();
    poison(&owner);
    // The actual Build caller can state-filter this occurrence without reading
    // payload. This test exercises the same borrowed iterator, not a fake row.
    pool.try_with_active_build_occurrences_and_reservation(
        |mut occurrences| {
            let (_, occurrence) = occurrences.next().unwrap();
            assert_eq!(occurrence.body_a(), body[1].observation());
            assert_eq!(occurrence.frames(), *frames);
            Ok::<_, SimulationJointPoolError>(())
        },
        |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
    )
    .unwrap_or_else(|_| panic!("non-payload visit must remain available"));
    assert_eq!(
        pool.try_limits(joint),
        Err(SimulationJointPoolError::LimitsLockPoisoned)
    );
    assert_eq!(
        pool.try_write_mutable_limits(joint, &owner, limits()),
        Err(SimulationJointPoolError::LimitsLockPoisoned)
    );
    assert_eq!(pool.snapshot(), before);
    // Native Remove likewise never dereferences the retained limits pointer.
    pool.try_remove_joint(joint).unwrap();
    assert_eq!(pool.counts().free, 1);
}

#[test]
fn generic_copy_payloads_share_the_same_mutable_admission_graph() {
    let (_bodies, body) = bodies();
    let mut pool = SimulationJointPool::<u32, u64>::try_new(1).unwrap();
    let frames = Arc::new(23);
    let owner = SimulationMutableJointLimits::new(0x1234_5678_9abc_def0_u64);
    let joint = pool
        .try_add_joint_mutable_limits(body[0], body[1], &frames, &owner)
        .unwrap()
        .unwrap();
    pool.try_write_mutable_limits(joint, &owner, u64::MAX)
        .unwrap();
    assert_eq!(pool.try_limits(joint).unwrap(), u64::MAX);
    assert_eq!(owner.try_read().unwrap(), u64::MAX);
}
