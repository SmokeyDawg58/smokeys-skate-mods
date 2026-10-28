use super::*;
use crate::ragdoll::{DEBUG_REFERENCE_NORMAL_CULLING_WORDS, DEBUG_REFERENCE_RAGDOLL_CULLING_WORDS};

#[test]
fn both_owned_tables_match_independent_frozen_eighteen_word_oracles() {
    let mut inventory = SkeletonCullingInventory::default();
    for (key, expected, clear_count) in [
        (
            SkeletonCullingKey::Normal,
            DEBUG_REFERENCE_NORMAL_CULLING_WORDS,
            12,
        ),
        (
            SkeletonCullingKey::Ragdoll,
            DEBUG_REFERENCE_RAGDOLL_CULLING_WORDS,
            136,
        ),
    ] {
        let resource = inventory.try_get_or_create(key).unwrap();
        assert_eq!(resource.key(), key);
        let table = resource.table();
        assert_eq!((table.rows, table.columns, table.array_size), (24, 24, 18));
        assert_eq!(table.words, expected);
        assert_eq!(
            table
                .words
                .iter()
                .map(|word| word.count_zeros())
                .sum::<u32>(),
            clear_count
        );
        for row in 0..24_usize {
            let diagonal = row * 24 + row;
            assert_ne!(table.words[diagonal >> 5] & (1 << (diagonal & 31)), 0);
            for column in 0..24_usize {
                let forward = row * 24 + column;
                let reverse = column * 24 + row;
                assert_eq!(
                    (table.words[forward >> 5] >> (forward & 31)) & 1,
                    (table.words[reverse >> 5] >> (reverse & 31)) & 1
                );
            }
        }
    }
}

#[test]
fn repeated_creation_finds_same_resource_and_keys_never_alias() {
    let mut inventory = SkeletonCullingInventory::default();
    assert!(inventory.find(SkeletonCullingKey::Normal).is_none());
    assert!(inventory.find(SkeletonCullingKey::Ragdoll).is_none());
    let normal = inventory
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap();
    assert_eq!(Arc::strong_count(&normal), 2);
    assert!(inventory.find(SkeletonCullingKey::Ragdoll).is_none());
    let second = inventory
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap();
    assert!(Arc::ptr_eq(&normal, &second));
    assert_eq!(Arc::strong_count(&normal), 3);
    let found = inventory.find(SkeletonCullingKey::Normal).unwrap();
    assert!(Arc::ptr_eq(&normal, &found));
    let ragdoll = inventory
        .try_get_or_create(SkeletonCullingKey::Ragdoll)
        .unwrap();
    assert!(!Arc::ptr_eq(&normal, &ragdoll));
    assert_ne!(normal.table().words, ragdoll.table().words);
    assert_eq!(normal.key().source_name(), "cullTableNormal");
    assert_eq!(ragdoll.key().source_name(), "cullTableRagdoll");
}

#[test]
fn reference_survives_inventory_and_is_released_with_last_real_owner() {
    let mut inventory = SkeletonCullingInventory::default();
    let retained = inventory
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap();
    let weak = Arc::downgrade(&retained);
    drop(inventory);
    assert_eq!(retained.table().words, DEBUG_REFERENCE_NORMAL_CULLING_WORDS);
    assert!(weak.upgrade().is_some());
    drop(retained);
    assert!(weak.upgrade().is_none());
}

#[test]
fn independent_session_inventories_do_not_fabricate_process_global_sharing() {
    let mut first = SkeletonCullingInventory::default();
    let mut second = SkeletonCullingInventory::default();
    for key in [SkeletonCullingKey::Normal, SkeletonCullingKey::Ragdoll] {
        let left = first.try_get_or_create(key).unwrap();
        let right = second.try_get_or_create(key).unwrap();
        assert!(!Arc::ptr_eq(&left, &right));
        assert_eq!(left.table(), right.table());
    }
}
