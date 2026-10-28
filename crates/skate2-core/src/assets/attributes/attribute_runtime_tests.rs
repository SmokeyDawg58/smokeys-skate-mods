//! Synthetic immutable loaded graphs exercise source lookup discriminators.
//! Mounted tests separately exercise actual metadata parsing and payloads.

use super::*;
use crate::skateboard_wheels::{
    SkateboardWheelAttribute, SkateboardWheelAttributes, SkateboardWheelTuningOrigin,
};
use skate2_formats::attrib::AttributeCollectionEntry;

const CLASS: u64 = 0x0123_4567_89ab_cdef;
const CHILD: u64 = 0xd7ed_bd36_2d7d_2152;
const PARENT: u64 = 0x1234_5678_9abc_def0;
const KEY: u64 = 0xf123_4567_89ab_cdef;
const OTHER: u64 = 0xf123_4567_89ab_cdee;

#[test]
fn cached_layout_text_follows_original_relocation_and_retains_bytes() {
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, CHILD, 0, Some(0), vec![]);
    let mut bytes = vec![0; 256];
    bytes[0..4].copy_from_slice(&0x1234_u32.to_be_bytes());
    bytes[32..39].copy_from_slice(b"skater\0");
    set_dependency_bytes(&mut vault, 1, bytes);
    vault.pointers.insert(
        location(1, 0),
        AttributePointerValue::Direct {
            dependency_index: 1,
            offset: 32,
        },
    );
    let vault = Arc::new(vault);
    let instance = vault.instance(CLASS, CHILD);
    let text = instance.try_cached_layout_text(0).unwrap().unwrap();
    assert_eq!(text.bytes(), b"skater");
    assert_eq!(text.origin(), AttributeRuntimeDataOrigin::StoredPointer);
    drop(instance);
    drop(vault);
    assert_eq!(text.bytes(), b"skater");
}

#[test]
fn cached_layout_text_distinguishes_null_pointer_from_empty_string() {
    for pointer in [None, Some(32)] {
        let mut vault = graph(CLASS, vec![]);
        collection(&mut vault, CLASS, CHILD, 0, Some(0), vec![]);
        set_dependency_bytes(&mut vault, 1, vec![0; 256]);
        if let Some(offset) = pointer {
            vault.pointers.insert(
                location(1, 0),
                AttributePointerValue::Direct {
                    dependency_index: 1,
                    offset,
                },
            );
        }
        let result = Arc::new(vault)
            .instance(CLASS, CHILD)
            .try_cached_layout_text(0)
            .unwrap();
        assert_eq!(result.is_some(), pointer.is_some());
        if let Some(text) = result {
            assert!(text.bytes().is_empty());
        }
    }
}

#[test]
fn cached_layout_text_rejects_relocation_at_terminating_zero() {
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, CHILD, 0, Some(0), vec![]);
    set_dependency_bytes(&mut vault, 1, vec![0; 256]);
    vault.pointers.insert(
        location(1, 0),
        AttributePointerValue::Direct {
            dependency_index: 1,
            offset: 32,
        },
    );
    vault.pointers.insert(
        location(1, 32),
        AttributePointerValue::Direct {
            dependency_index: 1,
            offset: 64,
        },
    );
    assert!(
        matches!(Arc::new(vault).instance(CLASS, CHILD).try_cached_layout_text(0), Err(AttributeRuntimeError::RelocatedPayload { slot }) if slot == location(1, 32))
    );
}

#[test]
fn cached_layout_text_rejects_missing_relocation_and_unterminated_data() {
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, CHILD, 0, Some(0), vec![]);
    let mut bytes = vec![1; 64];
    bytes[0..4].copy_from_slice(&123_u32.to_be_bytes());
    set_dependency_bytes(&mut vault, 1, bytes);
    let mut vault = Arc::new(vault);
    assert!(matches!(
        vault.instance(CLASS, CHILD).try_cached_layout_text(0),
        Err(AttributeRuntimeError::MissingRelocation { .. })
    ));
    Arc::get_mut(&mut vault).unwrap().pointers.insert(
        location(1, 0),
        AttributePointerValue::Direct {
            dependency_index: 1,
            offset: 32,
        },
    );
    assert!(matches!(
        vault.instance(CLASS, CHILD).try_cached_layout_text(0),
        Err(AttributeRuntimeError::UnterminatedText {
            dependency: 1,
            offset: 32
        })
    ));
}

pub(crate) fn definition(key: u64, flags: u8, size: u16) -> AttributeDefinition {
    AttributeDefinition {
        key,
        type_id: 16_294_631_463_297_616_344,
        offset: 0,
        size,
        maximum_count: 1,
        flags,
        alignment: 2,
    }
}

fn location(dependency_index: u16, offset: u32) -> AttributePointerLocation {
    AttributePointerLocation {
        dependency_index,
        offset,
    }
}

pub(crate) fn graph(class: u64, definitions: Vec<AttributeDefinition>) -> AttributeRuntimeVault {
    let static_data = PointerSlot {
        location: location(0, 2000),
        raw_word: 0,
    };
    let mut pointers = BTreeMap::new();
    pointers.insert(
        static_data.location,
        AttributePointerValue::Direct {
            dependency_index: 1,
            offset: 192,
        },
    );
    let mut payload = vec![0; 256];
    for (offset, word) in [
        (64, 0x1111_1111_u32),
        (128, 0x2222_2222),
        (192, 0x3333_3333),
    ] {
        for chunk in payload[offset..offset + 32].chunks_exact_mut(4) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
    }
    AttributeRuntimeVault {
        buffers: vec![Arc::from(vec![0; 2048]), Arc::from(payload)],
        pointers,
        classes: BTreeMap::from([(class, build_class(class, definitions).unwrap())]),
        collections: BTreeMap::new(),
        refspec_cache: refspec::RefSpecCache::default(),
        exported_classes: BTreeSet::from([class]),
        exported_collections: BTreeSet::new(),
    }
}

pub(crate) fn node(key: u64, flags: u8, value: u32) -> AttributeCollectionEntry {
    AttributeCollectionEntry {
        key,
        data_pointer: value,
        type_index: 0,
        node_flags: flags,
        entry_flags: 0xff,
    }
}

/// Add a controlled class before instances bind the immutable graph.
pub(crate) fn add_fixture_class(
    vault: &mut AttributeRuntimeVault,
    class: u64,
    definitions: Vec<AttributeDefinition>,
) {
    assert!(
        vault
            .classes
            .insert(class, build_class(class, definitions).unwrap())
            .is_none()
    );
}

/// Replace synthetic fixture storage BEFORE binding immutable instances.
/// Allows full generated layouts larger than this fixture's default256bytes.
pub(crate) fn set_dependency_bytes(
    vault: &mut AttributeRuntimeVault,
    dependency: usize,
    bytes: Vec<u8>,
) {
    vault.buffers[dependency] = bytes.into();
}

pub(crate) fn collection(
    vault: &mut AttributeRuntimeVault,
    class: u64,
    key: u64,
    parent: u64,
    layout: Option<u32>,
    entries: Vec<AttributeCollectionEntry>,
) {
    let export = u32::try_from(vault.collections.len() * 128).unwrap();
    let data = AttributeCollectionLoadData {
        collection_id: key,
        class_id: class,
        parent_id: parent,
        table_reserve: 0,
        table_key_shift: 0,
        entry_count: u32::try_from(entries.len()).unwrap(),
        type_count: 1,
        types_length: 1,
        layout_pointer: 0,
        types: vec![16_294_631_463_297_616_344],
        entries,
    };
    // Explicit inline storage is the real serialized node's word, not a copy
    // synthesized by GetData. This small byte buffer is synthetic test input.
    for (index, entry) in data.entries.iter().enumerate() {
        let offset = usize::try_from(export).unwrap() + 64 + index * 16;
        Arc::get_mut(&mut vault.buffers[0]).unwrap()[offset..offset + 4]
            .copy_from_slice(&entry.data_pointer.to_be_bytes());
    }
    let loaded = build_collection(export, &data).unwrap();
    if let Some(offset) = layout {
        vault.pointers.insert(
            loaded.layout.location,
            AttributePointerValue::Direct {
                dependency_index: 1,
                offset: u64::from(offset),
            },
        );
    }
    vault.collections.insert((class, key), loaded);
}

#[test]
fn exact_collection_lookup_does_not_retry_default_or_truncate_keys() {
    let mut vault = graph(CLASS, vec![definition(KEY, 4, 4)]);
    collection(&mut vault, CLASS, CHILD, 0, None, vec![node(KEY, 0x40, 7)]);
    let vault = Arc::new(vault);
    assert_eq!(
        vault
            .instance(CLASS, CHILD)
            .try_scalar_word(KEY, 0)
            .unwrap(),
        Some(7)
    );
    for instance in [
        vault.instance(CLASS, CHILD ^ (1 << 60)),
        vault.instance(CLASS ^ (1 << 60), CHILD),
    ] {
        assert!(instance.is_null());
        assert_eq!(instance.try_scalar_word(KEY, 0).unwrap(), None);
    }
}

#[test]
fn child_then_parent_explicit_precedes_original_child_layout() {
    for child_override in [false, true] {
        let mut vault = graph(CLASS, vec![definition(KEY, 6, 4)]);
        collection(
            &mut vault,
            CLASS,
            PARENT,
            0,
            Some(128),
            vec![node(KEY, 0x40, 17)],
        );
        collection(
            &mut vault,
            CLASS,
            CHILD,
            PARENT,
            Some(64),
            if child_override {
                vec![node(KEY, 0x40, 29)]
            } else {
                vec![]
            },
        );
        vault.validate_parents().unwrap();
        let instance = Arc::new(vault).instance(CLASS, CHILD);
        assert_eq!(
            instance.try_scalar_word(KEY, u32::MAX).unwrap(),
            Some(if child_override { 29 } else { 17 })
        );
        assert_eq!(
            instance.try_get_data(KEY, 0).unwrap().unwrap().origin(),
            AttributeRuntimeDataOrigin::Inline
        );
    }
}

#[test]
fn only_original_child_layout_is_fallback_not_parent_layout() {
    for child_layout in [None, Some(64)] {
        let mut vault = graph(CLASS, vec![definition(KEY, 6, 4)]);
        collection(&mut vault, CLASS, PARENT, 0, Some(128), vec![]);
        collection(&mut vault, CLASS, CHILD, PARENT, child_layout, vec![]);
        let instance = Arc::new(vault).instance(CLASS, CHILD);
        assert_eq!(
            instance.try_scalar_word(KEY, 0).unwrap(),
            child_layout.map(|_| 0x1111_1111)
        );
    }
}

#[test]
fn inherited_layout_node_addresses_its_explicit_owner_not_requesting_child() {
    let mut vault = graph(CLASS, vec![definition(KEY, 6, 4)]);
    collection(
        &mut vault,
        CLASS,
        PARENT,
        0,
        Some(128),
        vec![node(KEY, 0x10, 0)],
    );
    collection(&mut vault, CLASS, CHILD, PARENT, Some(64), vec![]);
    let instance = Arc::new(vault).instance(CLASS, CHILD);
    assert_eq!(instance.try_scalar_word(KEY, 0).unwrap(), Some(0x2222_2222));
}

#[test]
fn found_null_stops_parent_and_layout_fallback_and_scalar_index_is_ignored() {
    let mut vault = graph(CLASS, vec![definition(KEY, 6, 4), definition(OTHER, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        PARENT,
        0,
        None,
        vec![node(KEY, 0x40, 77)],
    );
    collection(
        &mut vault,
        CLASS,
        CHILD,
        PARENT,
        Some(64),
        vec![node(KEY, 0, 0), node(OTHER, 0x40, 0x8000_0000)],
    );
    let instance = Arc::new(vault).instance(CLASS, CHILD);
    assert_eq!(instance.try_scalar_word(KEY, 0).unwrap(), None);
    assert_eq!(
        instance.try_scalar_word(OTHER, u32::MAX).unwrap(),
        Some(0x8000_0000)
    );
}

#[test]
fn node_pointer_flags_preserve_supported_inline_layout_stored_precedence() {
    for (flags, raw, expected, origin) in [
        (
            0x70,
            0x4444_4444,
            0x4444_4444,
            AttributeRuntimeDataOrigin::Inline,
        ),
        (
            0x30,
            0,
            0x1111_1111,
            AttributeRuntimeDataOrigin::CollectionLayout,
        ),
        (
            0x00,
            0,
            0x2222_2222,
            AttributeRuntimeDataOrigin::StoredPointer,
        ),
    ] {
        let mut vault = graph(CLASS, vec![definition(KEY, 4, 4)]);
        collection(
            &mut vault,
            CLASS,
            CHILD,
            0,
            Some(64),
            vec![node(KEY, flags, raw)],
        );
        let slot = vault.collections[&(CLASS, CHILD)].nodes[&KEY]
            .stored_word
            .unwrap();
        if flags == 0 {
            vault.pointers.insert(
                slot.location,
                AttributePointerValue::Direct {
                    dependency_index: 1,
                    offset: 128,
                },
            );
        }
        let instance = Arc::new(vault).instance(CLASS, CHILD);
        let data = instance.try_get_data(KEY, 0).unwrap().unwrap();
        assert_eq!(data.origin(), origin);
        assert_eq!(instance.try_scalar_word(KEY, 0).unwrap(), Some(expected));
    }
}

#[test]
fn generated_physics_reads_cached_32_bytes_not_explicit_gravity_override() {
    let gravity_key = 0x48b8_137e_1b32_a2e4;
    let mut vault = graph(PHYSICS_CLASS, vec![definition(gravity_key, 6, 16)]);
    collection(
        &mut vault,
        PHYSICS_CLASS,
        CHILD,
        0,
        Some(64),
        vec![node(gravity_key, 0, 0)],
    );
    let slot = vault.collections[&(PHYSICS_CLASS, CHILD)].nodes[&gravity_key]
        .stored_word
        .unwrap();
    vault.pointers.insert(
        slot.location,
        AttributePointerValue::Direct {
            dependency_index: 1,
            offset: 128,
        },
    );
    let vault = Arc::new(vault);
    assert_eq!(
        vault
            .instance(PHYSICS_CLASS, CHILD)
            .try_get_data(gravity_key, 0)
            .unwrap()
            .unwrap()
            .bytes(),
        &[0x22; 16]
    );
    let physics = vault.physics(CHILD);
    let weak = Arc::downgrade(&vault);
    drop(vault);
    assert!(
        weak.upgrade().is_some(),
        "generated instance retains loaded owner"
    );
    let layout = physics.try_physics_layout().unwrap();
    assert_eq!(layout.origin, AttributePhysicsLayoutOrigin::CachedLayout);
    assert_eq!(layout.raw_bytes, [0x11; 32]);
}

#[test]
fn generated_physics_missing_own_layout_uses_zero_even_with_parent_layout() {
    let mut vault = graph(PHYSICS_CLASS, vec![definition(KEY, 6, 16)]);
    collection(&mut vault, PHYSICS_CLASS, PARENT, 0, Some(128), vec![]);
    collection(&mut vault, PHYSICS_CLASS, CHILD, PARENT, None, vec![]);
    let vault = Arc::new(vault);
    for physics in [vault.physics(CHILD), vault.physics(123)] {
        assert_eq!(
            physics.try_physics_layout().unwrap(),
            AttributePhysicsLayout {
                raw_bytes: [0; 32],
                origin: AttributePhysicsLayoutOrigin::DefaultZero32
            }
        );
    }
    let data = vault
        .instance(PHYSICS_CLASS, PARENT)
        .try_get_data(KEY, 0)
        .unwrap()
        .unwrap();
    assert_eq!(data.bytes(), &[0x22; 16]);
}

#[test]
fn generated_skeleton_layouts_bypass_explicit_nodes_and_never_borrow_parent_layouts() {
    use crate::skeleton_attributes::{
        SkeletonAttributeClass, SkeletonAttributeRead, SkeletonAttributes,
    };
    for (class, size) in [
        (SkeletonAttributeClass::Skeleton, 112),
        (SkeletonAttributeClass::Bones, 576),
        (SkeletonAttributeClass::Drives, 736),
        (SkeletonAttributeClass::Joints, 440),
    ] {
        let mut vault = graph(class.key(), vec![definition(KEY, 6, 4)]);
        vault.buffers[1] = Arc::from(vec![0x11; 1024]);
        collection(&mut vault, class.key(), PARENT, 0, Some(128), vec![]);
        collection(
            &mut vault,
            class.key(),
            CHILD,
            PARENT,
            Some(64),
            vec![node(KEY, 0x40, 7)],
        );
        collection(&mut vault, class.key(), 123, PARENT, None, vec![]);
        let vault = Arc::new(vault);
        let scalar = vault.instance(class.key(), CHILD);
        assert_eq!(scalar.try_scalar_word(KEY, 0).unwrap(), Some(7));
        let mut generated = SkeletonAttributes::bind(&vault, CHILD);
        let retained = generated.layout(class).unwrap();
        assert_eq!(retained.as_ref(), vec![0x11; size]);
        let mut missing = SkeletonAttributes::bind(&vault, 123);
        assert_eq!(missing.layout(class).unwrap().as_ref(), vec![0; size]);
        // The generated range owns real bytes beyond all instance/graph owners.
        drop(scalar);
        drop(generated);
        drop(missing);
        drop(vault);
        assert_eq!(retained.as_ref(), vec![0x11; size]);
    }
}

#[test]
fn null_and_live_instances_keep_original_graph_and_data_keeps_its_buffer() {
    let empty = Arc::new(graph(CLASS, vec![definition(KEY, 4, 4)]));
    let null = empty.instance(CLASS, CHILD);
    let mut loaded = graph(CLASS, vec![definition(KEY, 4, 4)]);
    collection(
        &mut loaded,
        CLASS,
        CHILD,
        0,
        None,
        vec![node(KEY, 0x40, 13)],
    );
    let loaded = Arc::new(loaded);
    let instance = loaded.instance(CLASS, CHILD);
    let data = instance.try_get_data(KEY, 0).unwrap().unwrap();
    drop(empty);
    drop(loaded);
    drop(instance);
    assert_eq!(
        null.try_scalar_word(KEY, 0).unwrap(),
        None,
        "later separate graph never rebinds an existing null instance"
    );
    assert_eq!(data.bytes(), &13_u32.to_be_bytes());
}

#[test]
fn layout_eligibility_and_lazy_unsupported_nodes_are_not_missing_values() {
    for (definition_flags, node_flags, expect_layout, expected_error) in [
        (4, None, false, 0),
        (6, None, true, 0),
        (14, None, false, 0),
        (7, None, true, 2),
        (0x26, None, true, 8),
        (4, Some(0x42), false, 2),
        (4, Some(0x48), false, 8),
    ] {
        let mut vault = graph(
            CLASS,
            vec![
                definition(KEY, definition_flags, 4),
                definition(OTHER, 4, 4),
            ],
        );
        assert_eq!(
            vault.classes[&CLASS].layout_nodes.contains_key(&KEY),
            expect_layout
        );
        let mut entries = vec![node(OTHER, 0x40, 99)];
        if let Some(flags) = node_flags {
            entries.push(node(KEY, flags, 0));
        }
        collection(&mut vault, CLASS, CHILD, 0, Some(64), entries);
        let instance = Arc::new(vault).instance(CLASS, CHILD);
        assert_eq!(
            instance.try_scalar_word(OTHER, 0).unwrap(),
            Some(99),
            "unrelated supported lookup remains available"
        );
        match expected_error {
            2 => assert!(matches!(
                instance.try_scalar_word(KEY, 0),
                Err(AttributeRuntimeError::UnsupportedArray { .. })
            )),
            8 => assert!(matches!(
                instance.try_scalar_word(KEY, 0),
                Err(AttributeRuntimeError::UnsupportedConstructed { .. })
            )),
            _ => assert_eq!(
                instance.try_scalar_word(KEY, 0).unwrap(),
                expect_layout.then_some(0x1111_1111)
            ),
        }
    }
}

#[test]
fn missing_parent_and_cycles_are_invalid_load_graphs_not_parentless_fallback() {
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, CHILD, PARENT, None, vec![]);
    assert!(matches!(
        vault.validate_parents(),
        Err(AttributeRuntimeError::MissingParent { .. })
    ));
    collection(&mut vault, CLASS, PARENT, CHILD, None, vec![]);
    assert!(matches!(
        vault.validate_parents(),
        Err(AttributeRuntimeError::ParentCycle { .. })
    ));
}

#[test]
fn dependency_identity_binding_checks_name_id_and_asset_flag() {
    let metadata = AttributeVaultMetadata {
        version: 0,
        dependencies: vec![skate2_formats::attrib::AttributeDependency {
            id: 7,
            name: "test.bin".to_owned(),
            is_asset: true,
        }],
        exports: vec![],
        pointer_fixups: vec![],
        pointer_terminator: skate2_formats::attrib::AttributePointerTerminator {
            target_delta: 0,
            encoded: 0,
            padding: [0; 16],
        },
        data: 0..0,
    };
    let valid = AttributeRuntimeDependency {
        id: 7,
        name: "test.bin".to_owned(),
        is_asset: true,
        bytes: Arc::from([1, 2, 3]),
    };
    for axis in 0..3 {
        let mut invalid = valid.clone();
        match axis {
            0 => invalid.id ^= 1 << 60,
            1 => invalid.name = "other.bin".to_owned(),
            _ => invalid.is_asset = false,
        }
        assert!(matches!(
            bind_dependencies(Arc::from([]), &[invalid], &metadata),
            Err(AttributeRuntimeError::DependencyBinding { index: 0 })
        ));
    }
    let buffers =
        bind_dependencies(Arc::from([]), std::slice::from_ref(&valid), &metadata).unwrap();
    assert!(Arc::ptr_eq(&buffers[1], &valid.bytes));
}

#[test]
fn wheel_consumer_defaults_only_missing_not_unsupported_found_nodes() {
    let attribute = SkateboardWheelAttribute::CapsuleLength;
    let class = attribute.class_key();
    let empty = Arc::new(graph(class, vec![definition(attribute.key(), 4, 4)]));
    let snapshot = SkateboardWheelAttributes::bind(&empty)
        .try_snapshot()
        .unwrap();
    for attribute in SkateboardWheelAttribute::ALL {
        assert_eq!(snapshot.word(attribute), 0);
        assert_eq!(
            snapshot.origin(attribute),
            SkateboardWheelTuningOrigin::DefaultDataArea
        );
    }
    for flags in [0x42, 0x48] {
        let mut vault = graph(class, vec![definition(attribute.key(), 4, 4)]);
        collection(
            &mut vault,
            class,
            SkateboardWheelAttribute::COLLECTION_KEY,
            0,
            None,
            vec![node(attribute.key(), flags, 0)],
        );
        let result = SkateboardWheelAttributes::bind(&Arc::new(vault)).try_snapshot();
        if flags == 0x42 {
            assert!(matches!(
                result,
                Err(AttributeRuntimeError::UnsupportedArray { .. })
            ));
        } else {
            assert!(matches!(
                result,
                Err(AttributeRuntimeError::UnsupportedConstructed { .. })
            ));
        }
    }
}

#[test]
fn selected_node_type_index_and_type_identity_are_checked_lazily() {
    for invalid_index in [false, true] {
        let mut vault = graph(CLASS, vec![definition(KEY, 4, 4), definition(OTHER, 4, 4)]);
        let mut invalid = node(KEY, 0x40, 7);
        if invalid_index {
            invalid.type_index = 9;
        }
        collection(
            &mut vault,
            CLASS,
            CHILD,
            0,
            None,
            vec![invalid, node(OTHER, 0x40, 11)],
        );
        if !invalid_index {
            vault
                .collections
                .get_mut(&(CLASS, CHILD))
                .unwrap()
                .nodes
                .get_mut(&KEY)
                .unwrap()
                .type_key = Some(0xdead_beef_dead_beef);
        }
        let instance = Arc::new(vault).instance(CLASS, CHILD);
        assert_eq!(instance.try_scalar_word(OTHER, 0).unwrap(), Some(11));
        if invalid_index {
            assert!(matches!(
                instance.try_scalar_word(KEY, 0),
                Err(AttributeRuntimeError::InvalidNodeTypeIndex { index: 9, .. })
            ));
        } else {
            assert!(matches!(
                instance.try_scalar_word(KEY, 0),
                Err(AttributeRuntimeError::NodeTypeMismatch { .. })
            ));
        }
    }
}

#[test]
fn scalar_and_generated_layout_never_return_raw_relocated_pointer_payloads() {
    for flags in [0x40, 0x10, 0] {
        let mut vault = graph(CLASS, vec![definition(KEY, 4, 4)]);
        collection(
            &mut vault,
            CLASS,
            CHILD,
            0,
            Some(64),
            vec![node(KEY, flags, 0)],
        );
        let stored = vault.collections[&(CLASS, CHILD)].nodes[&KEY]
            .stored_word
            .unwrap();
        let payload = match flags {
            0x40 => stored.location,
            0x10 => location(1, 64),
            _ => {
                vault.pointers.insert(
                    stored.location,
                    AttributePointerValue::Direct {
                        dependency_index: 1,
                        offset: 128,
                    },
                );
                location(1, 128)
            }
        };
        vault.pointers.insert(payload, AttributePointerValue::Null);
        let instance = Arc::new(vault).instance(CLASS, CHILD);
        assert!(matches!(
            instance.try_scalar_word(KEY, 0),
            Err(AttributeRuntimeError::RelocatedPayload { .. })
        ));
    }
    let mut vault = graph(PHYSICS_CLASS, vec![]);
    collection(&mut vault, PHYSICS_CLASS, CHILD, 0, Some(64), vec![]);
    vault.pointers.insert(
        location(1, 80),
        AttributePointerValue::Export {
            dependency_index: 0,
            object_id: 123,
        },
    );
    assert!(matches!(
        Arc::new(vault).physics(CHILD).try_physics_layout(),
        Err(AttributeRuntimeError::RelocatedPayload { .. })
    ));
}

#[test]
fn relative_node_offsets_cannot_ignore_relocation_of_their_union_word() {
    for flags in [0x10, 0x20, 0x30] {
        let mut vault = graph(CLASS, vec![definition(KEY, 4, 4)]);
        collection(
            &mut vault,
            CLASS,
            CHILD,
            0,
            Some(64),
            vec![node(KEY, flags, 0)],
        );
        let slot = vault.collections[&(CLASS, CHILD)].nodes[&KEY]
            .stored_word
            .unwrap();
        vault.pointers.insert(
            slot.location,
            AttributePointerValue::Direct {
                dependency_index: 1,
                offset: 128,
            },
        );
        assert!(matches!(
            Arc::new(vault)
                .instance(CLASS, CHILD)
                .try_scalar_word(KEY, 0),
            Err(AttributeRuntimeError::RelocatedPayload { .. })
        ));
    }
}

#[test]
fn class_static_fetch_cannot_substitute_serialized_payload_for_registered_storage() {
    let mut vault = graph(CLASS, vec![definition(KEY, 4, 4)]);
    collection(&mut vault, CLASS, CHILD, 0, None, vec![node(KEY, 0x20, 0)]);
    // graph() supplies a raw class static pointer to nonzero bytes, but the
    // native ClassExportPolicy requires registered destination storage. The
    // unsupported branch must not expose those bytes or silently default.
    assert!(matches!(
        Arc::new(vault)
            .instance(CLASS, CHILD)
            .try_scalar_word(KEY, 0),
        Err(AttributeRuntimeError::UnsupportedClassStaticStorage { .. })
    ));
}

#[test]
fn cached_layout_field_does_not_read_unrelated_relocated_pointer() {
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, CHILD, 0, Some(0), vec![]);
    let mut bytes = vec![0; 256];
    bytes[16] = 0x80;
    set_dependency_bytes(&mut vault, 1, bytes);
    vault.pointers.insert(
        location(1, 0),
        AttributePointerValue::Direct {
            dependency_index: 1,
            offset: 32,
        },
    );
    let instance = Arc::new(vault).instance(CLASS, CHILD);
    assert!(matches!(
        instance.try_cached_layout(20),
        Err(AttributeRuntimeError::RelocatedPayload { .. })
    ));
    assert_eq!(
        instance
            .try_cached_layout_field(16, 1)
            .unwrap()
            .unwrap()
            .bytes(),
        &[0x80]
    );
}

#[test]
fn cached_layout_field_keeps_overlap_bounds_and_null_checks() {
    for pointer_offset in 13..=16 {
        let mut vault = graph(CLASS, vec![]);
        collection(&mut vault, CLASS, CHILD, 0, Some(0), vec![]);
        vault
            .pointers
            .insert(location(1, pointer_offset), AttributePointerValue::Null);
        let instance = Arc::new(vault).instance(CLASS, CHILD);
        assert!(matches!(instance.try_cached_layout_field(16, 1),
            Err(AttributeRuntimeError::RelocatedPayload { slot }) if slot == location(1, pointer_offset)));
    }
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, CHILD, 0, Some(0), vec![]);
    let instance = Arc::new(vault).instance(CLASS, CHILD);
    assert!(matches!(
        instance.try_cached_layout_field(256, 1),
        Err(AttributeRuntimeError::ByteRange { .. })
    ));
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, CHILD, 0, None, vec![]);
    assert!(
        Arc::new(vault)
            .instance(CLASS, CHILD)
            .try_cached_layout_field(16, 1)
            .unwrap()
            .is_none()
    );
}

pub(crate) fn fixture_text_pointer(
    vault: &mut AttributeRuntimeVault,
    dependency: u16,
    slot: u32,
    target: u64,
) {
    vault.pointers.insert(
        location(dependency, slot),
        AttributePointerValue::Direct {
            dependency_index: dependency,
            offset: target,
        },
    );
}
#[path = "attribute_runtime_text_tests.rs"]
mod text_projection;
