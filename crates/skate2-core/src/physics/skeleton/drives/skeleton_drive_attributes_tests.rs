//! Synthetic immutable Vault graphs, using the existing loaded-node fixtures.
//! No runtime reload/global singleton or original tuning is fabricated.
use super::*;
use crate::attribute_runtime::{
    AttributeRuntimeDataOrigin,
    tests::{collection, definition, graph, node},
};

const KEY: u64 = 0xe1a7_f2dc_fab0_b8d0;
const ABSENT: u64 = 0x1234_0000_5678_0000;
const NULL_LAYOUT: u64 = 0x9988_7766_5544_3322;

#[test]
fn update_global_lookup_is_separate_and_scalar_width_is_checked_only_on_load() {
    use crate::skeleton_drives::update::SkeletonDrivesUpdateAttributeRead as Update;
    let class = 0xbabb_3ec1_1084_9e08;
    let key = 0x0a58_9569_f5fa_0e6c;
    let mut vault = graph(class, vec![definition(key, 6, 4)]);
    collection(
        &mut vault,
        class,
        DEFAULT_COLLECTION,
        0,
        None,
        vec![node(key, 0x40, 0x8000_0000)],
    );
    let mut reader = SkeletonDriveAttributes::bind(&Arc::new(vault));
    let pointer = Update::lookup_global(&mut reader, key).unwrap();
    assert!(pointer.is_some());
    assert_eq!(
        Update::float(&mut reader, &pointer).unwrap().to_bits(),
        0x8000_0000
    );
    assert!(matches!(
        Update::byte(&mut reader, &pointer),
        Err(SkeletonDriveAttributeError::Runtime(
            AttributeRuntimeError::ScalarSize { size: 4 }
        ))
    ));
    assert!(
        SkeletonDriveAttributeRead::lookup_global(&mut reader, key)
            .unwrap()
            .is_none()
    );
    let absent = Update::lookup_global(&mut reader, ABSENT).unwrap();
    assert_eq!(Update::float(&mut reader, &absent).unwrap().to_bits(), 0);
    assert_eq!(Update::byte(&mut reader, &absent).unwrap(), 0);
}

#[test]
fn update_byte_retains_raw_value_and_is_not_a_four_byte_float() {
    use crate::skeleton_drives::update::SkeletonDrivesUpdateAttributeRead as Update;
    let class = 0xbabb_3ec1_1084_9e08;
    let key = 0xca9e_8fa1_ab74_fdd0;
    let mut vault = graph(class, vec![definition(key, 6, 1)]);
    collection(
        &mut vault,
        class,
        DEFAULT_COLLECTION,
        0,
        None,
        vec![node(key, 0x40, 0x8700_0000)],
    );
    let mut reader = SkeletonDriveAttributes::bind(&Arc::new(vault));
    let pointer = Update::lookup_global(&mut reader, key).unwrap();
    assert_eq!(Update::byte(&mut reader, &pointer).unwrap(), 0x87);
    assert!(matches!(
        Update::float(&mut reader, &pointer),
        Err(SkeletonDriveAttributeError::Runtime(
            AttributeRuntimeError::ScalarSize { size: 1 }
        ))
    ));
}

#[test]
fn update_captured_layout_survives_later_selection_and_distinguishes_null_layout() {
    use crate::skeleton_drives::update::SkeletonDrivesUpdateAttributeRead as Update;
    let mut vault = graph(ANIMATION_DRIVES_CLASS, vec![]);
    collection(
        &mut vault,
        ANIMATION_DRIVES_CLASS,
        LOCAL_COLLECTION,
        0,
        Some(64),
        vec![],
    );
    collection(
        &mut vault,
        ANIMATION_DRIVES_CLASS,
        LOCAL_ROOT_COLLECTION,
        0,
        Some(128),
        vec![],
    );
    collection(
        &mut vault,
        ANIMATION_DRIVES_CLASS,
        NULL_LAYOUT,
        0,
        None,
        vec![],
    );
    let mut reader = SkeletonDriveAttributes::bind(&Arc::new(vault));
    let initial = Update::capture_animation_layout(&mut reader, SkeletonDriveType::Local).unwrap();
    reader
        .find_and_change_animation_drive(SkeletonDriveType::Local)
        .unwrap();
    let captured = Update::capture_animation_layout(&mut reader, SkeletonDriveType::Local).unwrap();
    reader.change_animation(SkeletonDriveType::Local, LOCAL_ROOT_COLLECTION);
    let changed = Update::capture_animation_layout(&mut reader, SkeletonDriveType::Local).unwrap();
    for offset in [8, 12, 4, 0] {
        assert_eq!(
            Update::animation_float(&mut reader, &initial, offset)
                .unwrap()
                .to_bits(),
            0
        );
        assert_eq!(
            Update::animation_float(&mut reader, &captured, offset)
                .unwrap()
                .to_bits(),
            0x1111_1111
        );
        assert_eq!(
            Update::animation_float(&mut reader, &changed, offset)
                .unwrap()
                .to_bits(),
            0x2222_2222
        );
    }
    reader.change_animation(SkeletonDriveType::Local, NULL_LAYOUT);
    assert!(matches!(
        Update::capture_animation_layout(&mut reader, SkeletonDriveType::Local),
        Err(SkeletonDriveAttributeError::NullCachedAnimationLayout { .. })
    ));
    assert_eq!(
        Update::animation_float(&mut reader, &captured, 8)
            .unwrap()
            .to_bits(),
        0x1111_1111
    );
    assert!(matches!(
        Update::animation_float(&mut reader, &captured, 16),
        Err(SkeletonDriveAttributeError::LayoutOffset { offset: 16 })
    ));
}

#[test]
fn two_cached_layouts_are_separate_from_parent_and_explicit_generic_nodes() {
    let mut vault = graph(ANIMATION_DRIVES_CLASS, vec![definition(KEY, 6, 4)]);
    collection(
        &mut vault,
        ANIMATION_DRIVES_CLASS,
        DEFAULT_COLLECTION,
        0,
        Some(192),
        vec![node(KEY, 0x40, 29)],
    );
    collection(
        &mut vault,
        ANIMATION_DRIVES_CLASS,
        LOCAL_COLLECTION,
        DEFAULT_COLLECTION,
        Some(64),
        vec![node(KEY, 0x40, 17)],
    );
    collection(
        &mut vault,
        ANIMATION_DRIVES_CLASS,
        LOCAL_ROOT_COLLECTION,
        DEFAULT_COLLECTION,
        Some(128),
        vec![],
    );
    let vault = Arc::new(vault);
    let mut reader = SkeletonDriveAttributes::bind(&vault);
    for kind in [SkeletonDriveType::Local, SkeletonDriveType::LocalRoot] {
        for offset in [0, 4, 8, 12] {
            assert_eq!(reader.animation_float(kind, offset).unwrap().to_bits(), 0);
        }
    }
    for _ in 0..24 {
        reader
            .find_and_change_animation_drive(SkeletonDriveType::Local)
            .unwrap();
        reader
            .find_and_change_animation_drive(SkeletonDriveType::LocalRoot)
            .unwrap();
        for (kind, word) in [
            (SkeletonDriveType::Local, 0x1111_1111),
            (SkeletonDriveType::LocalRoot, 0x2222_2222),
        ] {
            for offset in [8, 12, 4, 0] {
                assert_eq!(
                    reader.animation_float(kind, offset).unwrap().to_bits(),
                    word
                );
            }
        }
    }
    // Actual generic child then parent lookup disagrees with generated cache.
    assert_eq!(
        vault
            .instance(ANIMATION_DRIVES_CLASS, LOCAL_COLLECTION)
            .try_scalar_word(KEY, 0)
            .unwrap(),
        Some(17)
    );
    assert_eq!(
        vault
            .instance(ANIMATION_DRIVES_CLASS, LOCAL_ROOT_COLLECTION)
            .try_scalar_word(KEY, 0)
            .unwrap(),
        Some(29)
    );
    drop(vault);
    assert_eq!(
        reader
            .animation_float(SkeletonDriveType::Local, 8)
            .unwrap()
            .to_bits(),
        0x1111_1111
    );
    assert_eq!(
        reader
            .animation_float(SkeletonDriveType::LocalRoot, 8)
            .unwrap()
            .to_bits(),
        0x2222_2222
    );
}

#[test]
fn null_change_keeps_old_cache_but_real_collection_with_null_layout_never_defaults() {
    let mut vault = graph(ANIMATION_DRIVES_CLASS, vec![]);
    collection(
        &mut vault,
        ANIMATION_DRIVES_CLASS,
        LOCAL_COLLECTION,
        0,
        Some(64),
        vec![],
    );
    collection(
        &mut vault,
        ANIMATION_DRIVES_CLASS,
        NULL_LAYOUT,
        LOCAL_COLLECTION,
        None,
        vec![],
    );
    let vault = Arc::new(vault);
    let mut reader = SkeletonDriveAttributes::bind(&vault);
    let kind = SkeletonDriveType::Local;
    reader.change_animation(kind, ABSENT);
    assert_eq!(reader.animation_float(kind, 0).unwrap().to_bits(), 0);
    reader.find_and_change_animation_drive(kind).unwrap();
    assert_eq!(
        reader.animation_float(kind, 0).unwrap().to_bits(),
        0x1111_1111
    );
    reader.change_animation(kind, ABSENT);
    assert!(reader.animation[kind.index()].current.is_none());
    for _ in 0..2 {
        reader.change_animation(kind, ABSENT);
        assert_eq!(
            reader.animation_float(kind, 0).unwrap().to_bits(),
            0x1111_1111
        );
    }
    // A real nonnull collection replaces the cache even when its own layout
    // is null. Neither its parent's valid layout nor DefaultDataArea applies.
    reader.change_animation(kind, NULL_LAYOUT);
    assert!(reader.animation[kind.index()].current.is_some());
    assert!(matches!(
        reader.animation_float(kind, 0),
        Err(SkeletonDriveAttributeError::NullCachedAnimationLayout { .. })
    ));
    reader.change_animation(kind, ABSENT);
    assert!(matches!(
        reader.animation_float(kind, 0),
        Err(SkeletonDriveAttributeError::NullCachedAnimationLayout { .. })
    ));
    // Independent slot still retains its initial generated default16.
    assert_eq!(
        reader
            .animation_float(SkeletonDriveType::LocalRoot, 0)
            .unwrap()
            .to_bits(),
        0
    );
}

#[test]
fn global_fetch_uses_actual_inline_pointer_and_preserves_raw_scalar_bits() {
    for bits in [0x8000_0000, 0x7fc1_2345, 0x46c3_5000] {
        let mut vault = graph(SKELETON_DRIVES_CLASS, vec![definition(KEY, 4, 4)]);
        collection(
            &mut vault,
            SKELETON_DRIVES_CLASS,
            DEFAULT_COLLECTION,
            0,
            Some(64),
            vec![node(KEY, 0x40, bits)],
        );
        let vault = Arc::new(vault);
        let mut reader = SkeletonDriveAttributes::bind(&vault);
        for _ in 0..24 {
            let pointer = reader.lookup_global(KEY).unwrap();
            assert_eq!(
                pointer.as_ref().unwrap().origin(),
                AttributeRuntimeDataOrigin::Inline
            );
            assert_eq!(reader.float(&pointer).unwrap().to_bits(), bits);
        }
        let retained = reader.lookup_global(KEY).unwrap();
        drop(vault);
        assert_eq!(reader.float(&retained).unwrap().to_bits(), bits);
        let missing = reader.lookup_global(ABSENT).unwrap();
        assert!(missing.is_none());
        assert_eq!(reader.float(&missing).unwrap().to_bits(), 0);
    }
}

#[test]
fn found_unsupported_global_nodes_and_wrong_width_are_errors_not_missing_defaults() {
    for (flags, size) in [(0x42, 4), (0x48, 4), (0x40, 1)] {
        let mut vault = graph(SKELETON_DRIVES_CLASS, vec![definition(KEY, 4, size)]);
        collection(
            &mut vault,
            SKELETON_DRIVES_CLASS,
            DEFAULT_COLLECTION,
            0,
            None,
            vec![node(KEY, flags, 0)],
        );
        let mut reader = SkeletonDriveAttributes::bind(&Arc::new(vault));
        match flags {
            0x42 => assert!(
                matches!(reader.lookup_global(KEY), Err(SkeletonDriveAttributeError::Runtime(AttributeRuntimeError::UnsupportedArray { key })) if key == KEY)
            ),
            0x48 => assert!(
                matches!(reader.lookup_global(KEY), Err(SkeletonDriveAttributeError::Runtime(AttributeRuntimeError::UnsupportedConstructed { key })) if key == KEY)
            ),
            _ => {
                let pointer = reader.lookup_global(KEY).unwrap();
                assert!(pointer.is_some());
                assert!(matches!(
                    reader.float(&pointer),
                    Err(SkeletonDriveAttributeError::Runtime(
                        AttributeRuntimeError::ScalarSize { size: 1 }
                    ))
                ));
            }
        }
    }
}

#[test]
fn cached_layout_access_checks_full_range_and_aligned_scalar_offset_without_fallback() {
    let mut vault = graph(ANIMATION_DRIVES_CLASS, vec![]);
    // Existing fixture dependency1 has256bytes: a16byte layout at250 is invalid.
    collection(
        &mut vault,
        ANIMATION_DRIVES_CLASS,
        LOCAL_COLLECTION,
        0,
        Some(250),
        vec![],
    );
    let mut reader = SkeletonDriveAttributes::bind(&Arc::new(vault));
    reader
        .find_and_change_animation_drive(SkeletonDriveType::Local)
        .unwrap();
    assert!(matches!(
        reader.animation_float(SkeletonDriveType::Local, 0),
        Err(SkeletonDriveAttributeError::Runtime(
            AttributeRuntimeError::ByteRange { .. }
        ))
    ));
    for offset in [1, 16, usize::MAX] {
        assert!(
            matches!(reader.animation_float(SkeletonDriveType::LocalRoot, offset), Err(SkeletonDriveAttributeError::LayoutOffset { offset: actual }) if actual == offset)
        );
    }
}
