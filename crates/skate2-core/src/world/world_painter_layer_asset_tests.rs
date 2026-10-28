use super::*;
use crate::{
    decoded_arena_asset::DecodedArenaAsset,
    simulation_collision_asset::SimulationCollisionAsset,
    simulation_normal_arena_plan::{SimulationNormalArenaPlan, SimulationNormalOperation},
    simulation_static_arena_plan::SimulationStaticArenaPlan,
    world::CollisionArena,
};
use skate2_formats::{
    rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
    stream::ArenaResources,
};

/// Test fixture containing an actual retained arena/normal operation and local
/// object-ID linkage. It does not issue gameplay or registration authority.
pub(crate) fn fixture(
    id: u64,
    guid: u64,
    nodes: u32,
    quad: Option<[f32; 8]>,
) -> Arc<SimulationNormalArenaPlan> {
    let mut bytes = vec![0; 56];
    if let Some(values) = quad {
        for (i, value) in values.iter().enumerate() {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
    }
    bytes[32..36].copy_from_slice(&nodes.to_be_bytes());
    bytes[40..44].copy_from_slice(&u32::from(quad.is_some()).to_be_bytes());
    bytes[48..56].copy_from_slice(&guid.to_be_bytes());
    let dictionary = vec![
        entry(0, 0, 0),
        entry(0, 40, 0x00eb_000d),
        entry(40, 16, 0x00eb_000f),
    ];
    let descriptor = ResourceDescriptor {
        size: 0,
        alignment: 1,
    };
    let arena = Arena {
        file_header: FileHeader {
            magic: *b"\x89RW4xb2\0\r\n\x1a\n",
            is_big_endian: true,
            pointer_size_bits: 32,
            pointer_alignment: 4,
            unused: 0,
            major_version: *b"454\0",
            minor_version: *b"000\0",
            build_number: 0,
        },
        id: 17,
        entry_count: 3,
        used_entry_count: 3,
        alignment: 1,
        virtual_size: 56,
        dictionary_offset: 0,
        sections_offset: 0,
        base_offset: 0,
        unfix_context_offset: 0,
        fix_context_offset: 0,
        resource_descriptor: [descriptor; 5],
        resources_used: [descriptor; 5],
        resource_offsets: [0; 5],
        arena_group_offset: 0,
        dictionary,
    };
    let decoded = DecodedArenaAsset::from_parts_for_test(
        id ^ 0x8000_0000_0000_0000,
        0,
        ArenaResources {
            resources: [bytes, Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        },
        arena,
    );
    let collision = CollisionArena {
        id: 17,
        grind_assets: Vec::new(),
        aggregate_roots: Vec::new(),
        primitive_sets: Vec::new(),
        simple_primitive_sets: Vec::new(),
        meshes: Vec::new(),
        collision_models: Vec::new(),
        instances: Vec::new(),
    };
    let base = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
        SimulationCollisionAsset { decoded, collision },
        Vec::new(),
    ));
    Arc::new(
        SimulationNormalArenaPlan::from_parts_with_record_sources_for_test(
            base,
            vec![SimulationNormalOperation::RegistryAsset {
                kind: SimulationRegistryAssetKind::WorldPainter,
                asset_id: id,
                source_toc_index: 0,
                source_dictionary_index: 2,
                raw_object_type: 0x00eb_000f,
                raw_object_id: 2,
            }],
            vec![None],
        ),
    )
}
fn entry(start: u32, size: u32, kind: u32) -> DictionaryEntry {
    DictionaryEntry {
        resource_index: 0,
        object_offset: start,
        relocation_offset: 0,
        size,
        alignment: 1,
        type_index: 0,
        type_id: kind,
        object_range: start as usize..(start + size) as usize,
    }
}

#[test]
fn linked_header_preserves_full_guid_and_same_arena_lifetime() {
    let plan = fixture(
        0xfedc_ba98_7654_3210,
        0x0000_0014_03e3_87c2,
        3,
        Some([64.0; 8]),
    );
    let weak = Arc::downgrade(&plan);
    let inner = SimulationInnerRegistryAsset::try_from_operation(Arc::clone(&plan), 0).unwrap();
    let layer = WorldPainterLayerAsset::try_new(inner).unwrap();
    drop(plan);
    assert!(weak.upgrade().is_some());
    assert_eq!(layer.layer_guid().unwrap(), 0x0000_0014_03e3_87c2);
    let (subtractor, half) = layer.registration_bounds().unwrap().unwrap();
    assert_eq!(subtractor.map(f32::to_bits), [64.0_f32.to_bits(); 4]);
    assert_eq!(half.map(f32::to_bits), [64.0_f32.to_bits(); 4]);
    assert_eq!(layer.quad_node_count().unwrap(), Some(3));
    drop(layer);
    assert!(weak.upgrade().is_none());
}

#[test]
fn null_quad_tree_does_not_read_quad_fields_or_hide_layer_guid() {
    let plan = fixture(3, 0x0000_0014_03e3_87c2, 9, None);
    let layer = WorldPainterLayerAsset::try_new(
        SimulationInnerRegistryAsset::try_from_operation(plan, 0).unwrap(),
    )
    .unwrap();
    assert!(layer.registration_bounds().unwrap().is_none());
    assert_eq!(layer.quad_node_count().unwrap(), None);
    assert_eq!(layer.layer_guid().unwrap(), 0x0000_0014_03e3_87c2);
}

/// Root centre (0, 0), half width 64: node 0 splits, child 1 (-x, +y) is a
/// leaf with lookup 7, child 2 (+x, -y) is a leaf with lookup 9.
fn two_level_tree() -> Vec<u8> {
    let mut tree = vec![0; 40];
    for (lane, value) in [0.0_f32, 0.0, 0.0, 0.0, 64.0, 64.0, 0.0, 0.0]
        .iter()
        .enumerate()
    {
        tree[lane * 4..lane * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    tree[32..36].copy_from_slice(&3_u32.to_be_bytes());
    tree[36..40].copy_from_slice(&40_u32.to_be_bytes());
    let node = |children: [u16; 4], lookup: u16| {
        children
            .iter()
            .chain(&[lookup])
            .flat_map(|value| value.to_be_bytes())
            .collect::<Vec<_>>()
    };
    tree.extend(node([1, 1, 2, 2], 0));
    tree.extend(node([0xffff, 0, 0, 0], 7));
    tree.extend(node([0xffff, 0, 0, 0], 9));
    tree
}

#[test]
fn quad_tree_descends_in_native_child_order_and_misses_outside_root() {
    let tree = two_level_tree();
    // (-10, 10) misses child 0 centred (-32, -32) (dy 42 > 32): child 1.
    assert_eq!(quad_tree_lookup(&tree, [-10.0, 10.0]).unwrap(), Some(7));
    // (-10, -10) is inside child 0, whose index is 1 as well.
    assert_eq!(quad_tree_lookup(&tree, [-10.0, -10.0]).unwrap(), Some(7));
    // (10, -10): child 0 misses, child 1 misses, child 2 wins.
    assert_eq!(quad_tree_lookup(&tree, [10.0, -10.0]).unwrap(), Some(9));
    // Boundaries are inclusive (`vcmpbfp`), and outside the root is -1.
    assert_eq!(quad_tree_lookup(&tree, [64.0, -64.0]).unwrap(), Some(9));
    assert_eq!(quad_tree_lookup(&tree, [64.5, 0.0]).unwrap(), None);
    assert_eq!(quad_tree_lookup(&tree, [f32::NAN, 0.0]).unwrap(), None);
}

#[test]
fn dictionary_region_word_follows_relative_slots_and_rejects_overflow() {
    let mut dictionary = vec![0; 20];
    dictionary[0..4].copy_from_slice(&2_u32.to_be_bytes());
    dictionary[4..8].copy_from_slice(&2_u32.to_be_bytes());
    dictionary[12..16].copy_from_slice(&20_u32.to_be_bytes());
    // Two eight-byte slots at +20 pointing at entries +36 and +40.
    dictionary.extend(36_u32.to_be_bytes());
    dictionary.extend([0; 4]);
    dictionary.extend(40_u32.to_be_bytes());
    dictionary.extend([0; 4]);
    dictionary.extend(0x19f_u32.to_be_bytes());
    dictionary.extend(0x1a0_u32.to_be_bytes());
    assert_eq!(dictionary_region_word(&dictionary, 0).unwrap(), 0x19f);
    assert_eq!(dictionary_region_word(&dictionary, 1).unwrap(), 0x1a0);
    assert_eq!(
        dictionary_region_word(&dictionary, 2),
        Err(LayerAssetError::DictionaryIndex { index: 2, total: 2 })
    );
}
