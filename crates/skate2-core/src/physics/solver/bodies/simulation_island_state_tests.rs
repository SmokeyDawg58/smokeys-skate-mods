//! Synthetic prior-partition labels test persistence and source store scope.
//! Private label seeds below are NOT a recovered partitioning implementation.
//! Every body observation comes from an actual checked pool allocation.

use super::*;
use crate::simulation_rigid_body_pool::SimulationRigidBodyHandle;

#[test]
fn partition_pairs_inherit_merge_and_keep_duplicate_or_self_pairs_stable() {
    let mut labels = std::collections::BTreeMap::new();
    let mut next = 0;
    partition_pair(&mut labels, 10, 20, &mut next);
    partition_pair(&mut labels, 30, 40, &mut next);
    assert_eq!(next, 2);
    partition_pair(&mut labels, 50, 30, &mut next);
    partition_pair(&mut labels, 20, 60, &mut next);
    assert_eq!(labels[&50], 1);
    assert_eq!(labels[&60], 0);
    partition_pair(&mut labels, 20, 30, &mut next);
    assert!(labels.values().all(|&label| label == 0));
    partition_pair(&mut labels, 10, 20, &mut next);
    partition_pair(&mut labels, 10, 10, &mut next);
    assert_eq!(next, 2);
    partition_pair(&mut labels, 70, 70, &mut next);
    assert_eq!(labels[&70], 2);
    assert_eq!(next, 3);
}

#[test]
fn partition_merge_replaces_all_reused_labels_after_ten_island_wrap() {
    let mut labels = std::collections::BTreeMap::new();
    let mut next = 0;
    for i in 0..11 {
        partition_pair(&mut labels, 2 * i, 2 * i + 1, &mut next);
    }
    assert_eq!(next, 1);
    assert_eq!(labels[&0], 0);
    assert_eq!(labels[&20], 0);
    // Two otherwise unrelated pairs share island0 because the counter wraps.
    // Native replacement merges BOTH into first's island1 here.
    partition_pair(&mut labels, 2, 0, &mut next);
    for key in [0, 1, 2, 3, 20, 21] {
        assert_eq!(labels[&key], 1);
    }
    assert_eq!(labels[&4], 2);
}

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

#[test]
fn hook_store_overwrites_static_frozen_and_active_labels_without_state_filter() {
    let mut pool = SimulationRigidBodyPool::try_new(5).unwrap();
    let mut labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    let manager = add(&mut pool, 1);
    let members = [add(&mut pool, 1), add(&mut pool, 2), add(&mut pool, 4)];
    labels.labels.fill(91); // Synthetic previous partition bytes.
    pool.begin_dynamic_update_frame(1.0 / 60.0);
    labels
        .try_start_frame_labels(&pool, manager.observation())
        .unwrap();
    assert_eq!(labels.labels[slot(members[0])], 91);
    assert_eq!(labels.labels[slot(members[1])], 91);
    assert_eq!(labels.labels[slot(members[2])], 0);
    for body in members {
        labels
            .try_exclude_hook_body(&pool, body.observation())
            .unwrap();
    }
    for body in members {
        assert_eq!(labels.labels[slot(body)], -1);
    }
    assert_eq!(labels.labels[4], 91, "unvisited free slot stays untouched");
}

#[test]
fn hook_store_rejects_foreign_stale_frame_and_reused_body_without_changing_prefix() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let mut labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    let manager = add(&mut pool, 1);
    let body = add(&mut pool, 1);
    assert_eq!(
        labels.try_exclude_hook_body(&pool, body.observation()),
        Err(SimulationRigidBodyIslandStateError::StaleFrame)
    );
    labels
        .try_start_frame_labels(&pool, manager.observation())
        .unwrap();
    labels.labels[slot(body)] = 37;
    let foreign = SimulationRigidBodyPool::try_new(0).unwrap();
    assert_eq!(
        labels.try_exclude_hook_body(&foreign, body.observation()),
        Err(SimulationRigidBodyIslandStateError::ForeignPool)
    );
    pool.try_remove_rigid_body(body).unwrap();
    let replacement = add(&mut pool, 1);
    assert!(matches!(
        labels.try_exclude_hook_body(&pool, body.observation()),
        Err(SimulationRigidBodyIslandStateError::Body(_))
    ));
    assert_eq!(labels.labels[slot(replacement)], 37);
    labels
        .try_exclude_hook_body(&pool, replacement.observation())
        .unwrap();
    assert_eq!(labels.labels[slot(replacement)], -1);
    pool.begin_dynamic_update_frame(1.0 / 60.0);
    assert_eq!(
        labels.try_exclude_hook_body(&pool, replacement.observation()),
        Err(SimulationRigidBodyIslandStateError::StaleFrame)
    );
    assert_eq!(labels.labels[slot(replacement)], -1);
}

fn add(pool: &mut SimulationRigidBodyPool, state: u32) -> SimulationRigidBodyHandle {
    pool.try_add_rigid_body(IDENTITY, None, state)
        .unwrap()
        .unwrap()
}

fn slot(body: SimulationRigidBodyHandle) -> usize {
    usize::try_from(body.id().index()).unwrap()
}

#[test]
fn constructor_initializes_all_minus_one_only_before_manager_static_add() {
    let mut pool = SimulationRigidBodyPool::try_new(3).unwrap();
    let labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    assert_eq!(&*labels.labels, &[-1, -1, -1]);
    assert_eq!(labels.body_pool, pool.authority());
    assert_eq!(labels.frame, None);
    let manager_static = add(&mut pool, 1);
    assert!(matches!(
        SimulationRigidBodyIslandState::try_new(&pool),
        Err(SimulationRigidBodyIslandStateError::BodyPoolNotPristine)
    ));
    pool.try_remove_rigid_body(manager_static).unwrap();
    assert_eq!(pool.counts().free, 3);
    assert!(
        matches!(
            SimulationRigidBodyIslandState::try_new(&pool),
            Err(SimulationRigidBodyIslandStateError::BodyPoolNotPristine)
        ),
        "empty after removal is not pristine"
    );
    let mut advanced = SimulationRigidBodyPool::try_new(3).unwrap();
    advanced.begin_dynamic_update_frame(1.0 / 60.0);
    assert!(matches!(
        SimulationRigidBodyIslandState::try_new(&advanced),
        Err(SimulationRigidBodyIslandStateError::BodyPoolNotPristine)
    ));
}

#[test]
fn initial_pool_epoch_is_not_a_completed_label_refresh() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let mut labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    let manager_static = add(&mut pool, 1);
    assert!(
        pool.frame_identity().is_some(),
        "pool already has a host construction epoch"
    );
    assert!(matches!(
        labels.try_snapshot(&pool),
        Err(SimulationRigidBodyIslandStateError::StaleFrame)
    ));
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    let snapshot = labels.try_snapshot(&pool).unwrap();
    assert_eq!(snapshot.body_pool_authority(), pool.authority());
    assert_eq!(Some(snapshot.frame_identity()), pool.frame_identity());
    assert_eq!(
        snapshot.assignment(manager_static.observation()),
        Some((manager_static.observation(), -1))
    );
}

#[test]
fn active_zero_and_manager_minus_one_leave_other_signed_slot_labels_untouched() {
    let mut pool = SimulationRigidBodyPool::try_new(7).unwrap();
    let mut labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    let manager_static = add(&mut pool, 1);
    let active = add(&mut pool, 4);
    let frozen_negative = add(&mut pool, 2);
    let general_static = add(&mut pool, 1);
    let frozen_positive = add(&mut pool, 2);
    // Deliberate synthetic prior-partition/corrupt raw bytes, including signed
    // labels outside the ten-island range. This owner must not sanitize them.
    labels.labels.fill(-7);
    labels.labels[slot(manager_static)] = 91;
    labels.labels[slot(active)] = 92;
    labels.labels[slot(frozen_negative)] = i8::MIN;
    labels.labels[slot(general_static)] = -2;
    labels.labels[slot(frozen_positive)] = i8::MAX;
    let mut expected = labels.labels.clone();
    expected[slot(active)] = 0;
    expected[slot(manager_static)] = -1;
    pool.begin_dynamic_update_frame(1.0 / 60.0);
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    assert_eq!(labels.labels, expected);
    let snapshot = labels.try_snapshot(&pool).unwrap();
    for (body, raw) in [
        (manager_static, -1),
        (active, 0),
        (frozen_negative, i8::MIN),
        (general_static, -2),
        (frozen_positive, i8::MAX),
    ] {
        assert_eq!(
            snapshot.assignment(body.observation()),
            Some((body.observation(), raw))
        );
    }
    assert_eq!(
        snapshot
            .slots
            .iter()
            .filter(|entry| entry.is_some())
            .count(),
        5
    );
    assert_eq!(
        snapshot
            .slots
            .iter()
            .filter(|entry| entry.is_none())
            .count(),
        2
    );
}

#[test]
fn empty_active_list_still_resets_only_manager_static() {
    let mut pool = SimulationRigidBodyPool::try_new(3).unwrap();
    let mut labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    let manager_static = add(&mut pool, 1);
    let frozen = add(&mut pool, 2);
    labels.labels.fill(31);
    assert!(pool.snapshot().active_order.is_empty());
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    assert_eq!(labels.labels[slot(manager_static)], -1);
    assert_eq!(labels.labels[slot(frozen)], 31);
    assert_eq!(
        labels.labels.iter().filter(|label| **label == 31).count(),
        2
    );
    // Repeating the source stores in one frame is allowed, not a new error.
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    assert_eq!(
        labels
            .try_snapshot(&pool)
            .unwrap()
            .assignment(frozen.observation()),
        Some((frozen.observation(), 31))
    );
}

#[test]
fn remove_and_reuse_preserve_raw_slot_label_but_snapshot_current_occurrence() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let mut labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    let manager_static = add(&mut pool, 1);
    let old = add(&mut pool, 2);
    labels.labels[slot(old)] = 73; // synthetic result of an earlier partition
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    let before = labels.try_snapshot(&pool).unwrap();
    pool.try_remove_rigid_body(old).unwrap();
    let removed = labels.try_snapshot(&pool).unwrap();
    assert_eq!(removed.assignment(old.observation()), None);
    assert_eq!(labels.labels[slot(old)], 73);
    let fresh = add(&mut pool, 2);
    assert_eq!(fresh.id(), old.id());
    assert_ne!(fresh.observation(), old.observation());
    let reused = labels.try_snapshot(&pool).unwrap();
    assert_eq!(
        reused.assignment(fresh.observation()),
        Some((fresh.observation(), 73))
    );
    // The map accessor is slot-indexed: downstream compilers must compare the
    // returned observation, not assume querying with an old ID validates it.
    assert_eq!(
        reused.assignment(old.observation()),
        Some((fresh.observation(), 73))
    );
    assert_eq!(
        before.assignment(old.observation()),
        Some((old.observation(), 73))
    );
    pool.begin_dynamic_update_frame(1.0 / 60.0);
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    assert_eq!(
        labels
            .try_snapshot(&pool)
            .unwrap()
            .assignment(fresh.observation()),
        Some((fresh.observation(), 73))
    );
}

#[test]
fn foreign_pool_and_stale_static_refuse_before_any_label_or_frame_mutation() {
    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let mut labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    let manager_static = add(&mut pool, 1);
    let active = add(&mut pool, 4);
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    labels.labels[slot(active)] = 29;
    let old_labels = labels.labels.clone();
    let old_frame = labels.frame;
    let mut foreign = SimulationRigidBodyPool::try_new(2).unwrap();
    let foreign_static = add(&mut foreign, 1);
    assert_eq!(
        labels.try_start_frame_labels(&foreign, foreign_static.observation()),
        Err(SimulationRigidBodyIslandStateError::ForeignPool)
    );
    assert!(matches!(
        labels.try_snapshot(&foreign),
        Err(SimulationRigidBodyIslandStateError::ForeignPool)
    ));
    assert_eq!(labels.labels, old_labels);
    assert_eq!(labels.frame, old_frame);
    pool.try_remove_rigid_body(manager_static).unwrap();
    pool.begin_dynamic_update_frame(1.0 / 60.0);
    assert!(matches!(
        labels.try_start_frame_labels(&pool, manager_static.observation()),
        Err(SimulationRigidBodyIslandStateError::Body(_))
    ));
    assert_eq!(
        labels.labels, old_labels,
        "Active label was not partially zeroed"
    );
    assert_eq!(labels.frame, old_frame);
}

#[test]
fn advancing_pool_frame_requires_new_labels_before_snapshot() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let mut labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    let manager_static = add(&mut pool, 1);
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    let old = labels.try_snapshot(&pool).unwrap();
    pool.begin_dynamic_update_frame(1.0 / 60.0);
    assert!(matches!(
        labels.try_snapshot(&pool),
        Err(SimulationRigidBodyIslandStateError::StaleFrame)
    ));
    assert_ne!(Some(old.frame_identity()), pool.frame_identity());
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    assert_eq!(
        Some(labels.try_snapshot(&pool).unwrap().frame_identity()),
        pool.frame_identity()
    );
}

#[test]
fn active_callback_visits_exact_live_ring_order_after_remove_and_reuse() {
    let mut pool = SimulationRigidBodyPool::try_new(5).unwrap();
    let mut labels = SimulationRigidBodyIslandState::try_new(&pool).unwrap();
    let manager_static = add(&mut pool, 1);
    let first = add(&mut pool, 4);
    let middle = add(&mut pool, 4);
    let last = add(&mut pool, 4);
    let frozen = add(&mut pool, 2);
    pool.try_remove_rigid_body(middle).unwrap();
    let reused = add(&mut pool, 4);
    assert_eq!(reused.id(), middle.id());
    let mut visited = Vec::new();
    pool.try_for_each_active_observation(|body| visited.push(body))
        .unwrap();
    assert_eq!(
        visited
            .iter()
            .map(|body| body.identity().id())
            .collect::<Vec<_>>(),
        pool.snapshot().active_order
    );
    assert!(visited.contains(&first.observation()));
    assert!(visited.contains(&last.observation()));
    assert!(visited.contains(&reused.observation()));
    assert!(!visited.contains(&middle.observation()));
    assert!(!visited.contains(&frozen.observation()));
    assert_eq!(visited.len(), 3);
    labels.labels.fill(47);
    labels
        .try_start_frame_labels(&pool, manager_static.observation())
        .unwrap();
    let snapshot = labels.try_snapshot(&pool).unwrap();
    for body in visited {
        assert_eq!(snapshot.assignment(body), Some((body, 0)));
    }
    assert_eq!(
        snapshot.assignment(frozen.observation()),
        Some((frozen.observation(), 47))
    );
}
