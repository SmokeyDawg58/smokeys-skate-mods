//! Controlled loaded-node fixtures, not runtime tuning or editor reloads.
use super::*;
use crate::attribute_runtime::{
    AttributeRuntimeDataOrigin,
    tests::{collection, definition, graph, node},
};

// Independent raw instruction/route oracle, not the implementation's key()
// used to manufacture its own expected keys or instance selection.
const SKELETON: u64 = 0x9f86_2c2e_aa01_6ce5;
const COLLISION: u64 = 0x7379_c446_7aa7_5df9;
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
const ROUTES: [(SkeletonCollisionUpdateAttribute, u64, u64); 6] = [
    (
        SkeletonCollisionUpdateAttribute::TimerDuration,
        SKELETON,
        0x12ea_5351_a38f_d32e,
    ),
    (
        SkeletonCollisionUpdateAttribute::SmallObjectMassThreshold,
        SKELETON,
        0x9ab5_7b38_d5bb_9449,
    ),
    (
        SkeletonCollisionUpdateAttribute::GroundMaximumDistance,
        COLLISION,
        0x30ef_9709_5d6f_a22c,
    ),
    (
        SkeletonCollisionUpdateAttribute::GroundMinimumAlignment,
        COLLISION,
        0xe6b3_72a5_b8ec_c397,
    ),
    (
        SkeletonCollisionUpdateAttribute::AiGroupSixForceMultiplier,
        COLLISION,
        0x98db_b06a_81ec_4f67,
    ),
    (
        SkeletonCollisionUpdateAttribute::GroupSixForceMultiplier,
        COLLISION,
        0x3f30_79e0_8606_7292,
    ),
];

#[test]
fn all_six_raw_routes_use_their_exact_global_class_and_preserve_unusual_scalar_words() {
    let words = [
        0x8000_0000,
        0x7fc1_2345,
        0x7f80_0000,
        0xff80_0000,
        1,
        0x7f7f_ffff,
    ];
    for selected_class in [SKELETON, COLLISION] {
        let definitions = ROUTES
            .iter()
            .map(|(_, _, key)| definition(*key, 4, 4))
            .collect();
        let mut vault = graph(selected_class, definitions);
        let entries = ROUTES
            .iter()
            .zip(words)
            .map(|((_, _, key), word)| node(*key, 0x40, word))
            .collect();
        // All six keys exist in each single-class fixture. A wrong-class route
        // cannot accidentally pass by finding the same key in this collection.
        collection(&mut vault, selected_class, DEFAULT, 0, Some(64), entries);
        let mut reader = SkeletonCollisionUpdateAttributes::bind(&Arc::new(vault));
        for ((route, expected_class, key), word) in ROUTES.into_iter().zip(words) {
            assert_eq!(route.key(), key);
            let pointer = reader.lookup(route).unwrap();
            if selected_class == expected_class {
                assert_eq!(
                    pointer.as_ref().unwrap().origin(),
                    AttributeRuntimeDataOrigin::Inline
                );
                assert_eq!(reader.float(&pointer).unwrap().to_bits(), word);
            } else {
                assert!(pointer.is_none());
                assert_eq!(reader.float(&pointer).unwrap().to_bits(), 0);
            }
        }
    }
}

#[test]
fn delayed_load_keeps_malformed_alignment_unread_until_the_caller_requests_it() {
    let distance = 0x30ef_9709_5d6f_a22c;
    let alignment = 0xe6b3_72a5_b8ec_c397;
    let mut vault = graph(
        COLLISION,
        vec![definition(distance, 4, 4), definition(alignment, 4, 1)],
    );
    collection(
        &mut vault,
        COLLISION,
        DEFAULT,
        0,
        None,
        vec![
            node(distance, 0x40, 0x3e80_0000),
            node(alignment, 0x40, 0x7f00_0000),
        ],
    );
    let vault = Arc::new(vault);
    let mut reader = SkeletonCollisionUpdateAttributes::bind(&vault);
    let distance_pointer = reader
        .lookup(SkeletonCollisionUpdateAttribute::GroundMaximumDistance)
        .unwrap();
    let alignment_pointer = reader
        .lookup(SkeletonCollisionUpdateAttribute::GroundMinimumAlignment)
        .unwrap();
    assert!(
        alignment_pointer.is_some(),
        "lookup must not eagerly perform scalar width validation"
    );
    assert_eq!(
        reader.float(&distance_pointer).unwrap().to_bits(),
        0x3e80_0000
    );
    // Caller may skip alignment after a failed distance test or offboard branch.
    // If it really dereferences the retained pointer, malformed data is not zero.
    assert!(matches!(
        reader.float(&alignment_pointer),
        Err(AttributeRuntimeError::ScalarSize { size: 1 })
    ));
}

#[test]
fn repeated_fetch_uses_real_parent_resolution_instead_of_generated_cached_layout() {
    let key = 0x12ea_5351_a38f_d32e;
    let parent = 0x1100_2200_3300_4400;
    let mut vault = graph(SKELETON, vec![definition(key, 6, 4)]);
    collection(
        &mut vault,
        SKELETON,
        parent,
        0,
        Some(128),
        vec![node(key, 0x40, 0x3f40_0000)],
    );
    collection(&mut vault, SKELETON, DEFAULT, parent, Some(64), vec![]);
    let vault = Arc::new(vault);
    let mut reader = SkeletonCollisionUpdateAttributes::bind(&vault);
    for _ in 0..24 {
        let pointer = reader
            .lookup(SkeletonCollisionUpdateAttribute::TimerDuration)
            .unwrap();
        assert_eq!(
            pointer.as_ref().unwrap().origin(),
            AttributeRuntimeDataOrigin::Inline
        );
        assert_eq!(reader.float(&pointer).unwrap().to_bits(), 0x3f40_0000);
    }
    // The generated cached layout contains another actual word and must not
    // replace ordinary FETCH's explicit parent node.
    let cached = vault
        .instance(SKELETON, DEFAULT)
        .try_cached_layout(4)
        .unwrap()
        .unwrap();
    assert_eq!(cached.bytes(), &0x1111_1111_u32.to_be_bytes());
}

#[test]
fn ordinary_lookup_preserves_original_child_layout_fallback_and_true_absence() {
    let key = 0x9ab5_7b38_d5bb_9449;
    let mut vault = graph(SKELETON, vec![definition(key, 6, 4)]);
    collection(&mut vault, SKELETON, DEFAULT, 0, Some(64), vec![]);
    let mut reader = SkeletonCollisionUpdateAttributes::bind(&Arc::new(vault));
    let pointer = reader
        .lookup(SkeletonCollisionUpdateAttribute::SmallObjectMassThreshold)
        .unwrap();
    assert_eq!(
        pointer.as_ref().unwrap().origin(),
        AttributeRuntimeDataOrigin::CollectionLayout
    );
    assert_eq!(reader.float(&pointer).unwrap().to_bits(), 0x1111_1111);
    let absent = reader
        .lookup(SkeletonCollisionUpdateAttribute::TimerDuration)
        .unwrap();
    assert!(absent.is_none());
    assert_eq!(reader.float(&absent).unwrap().to_bits(), 0);
}

#[test]
fn found_unsupported_nodes_fail_lookup_instead_of_silently_becoming_default_zero() {
    let key = 0x9ab5_7b38_d5bb_9449;
    for (flags, size) in [(0x42, 4), (0x48, 4), (0x40, 1)] {
        let mut vault = graph(SKELETON, vec![definition(key, 4, size)]);
        collection(
            &mut vault,
            SKELETON,
            DEFAULT,
            0,
            None,
            vec![node(key, flags, 0)],
        );
        let mut reader = SkeletonCollisionUpdateAttributes::bind(&Arc::new(vault));
        let pointer = reader.lookup(SkeletonCollisionUpdateAttribute::SmallObjectMassThreshold);
        match flags {
            0x42 => assert!(
                matches!(pointer, Err(AttributeRuntimeError::UnsupportedArray { key: actual }) if actual == key)
            ),
            0x48 => assert!(
                matches!(pointer, Err(AttributeRuntimeError::UnsupportedConstructed { key: actual }) if actual == key)
            ),
            _ => assert!(matches!(
                reader.float(&pointer.unwrap()),
                Err(AttributeRuntimeError::ScalarSize { size: 1 })
            )),
        }
    }
}

#[test]
fn reader_and_delayed_pointer_retain_actual_immutable_storage_without_scalar_snapshot() {
    let key = 0x3f30_79e0_8606_7292;
    let mut vault = graph(COLLISION, vec![definition(key, 4, 4)]);
    collection(
        &mut vault,
        COLLISION,
        DEFAULT,
        0,
        None,
        vec![node(key, 0x40, 0x7fc0_4567)],
    );
    let vault = Arc::new(vault);
    let weak = Arc::downgrade(&vault);
    let mut reader = SkeletonCollisionUpdateAttributes::bind(&vault);
    let pointer = reader
        .lookup(SkeletonCollisionUpdateAttribute::GroupSixForceMultiplier)
        .unwrap();
    drop(vault);
    assert!(
        weak.upgrade().is_some(),
        "actual instances keep the selected immutable Vault alive"
    );
    assert_eq!(reader.float(&pointer).unwrap().to_bits(), 0x7fc0_4567);
    drop(reader);
    assert!(weak.upgrade().is_none());
    assert_eq!(
        pointer.as_ref().unwrap().bytes(),
        &0x7fc0_4567_u32.to_be_bytes(),
        "delayed pointer owns its original payload range, not an already-decoded tuning value"
    );
}
