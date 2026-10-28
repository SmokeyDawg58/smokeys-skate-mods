use super::*;
use crate::world_painter_layer_asset::tests::fixture;

#[test]
fn existing_registry_first_winner_exact_removal_and_shared_namespace() {
    use crate::{
        collection_resident_streamer::CollectionId,
        simulation_inner_registry_asset::SimulationInnerRegistryAsset,
        simulation_resident_registry::{
            SimulationInnerRegistryError, SimulationResidentContainerAssetKind,
            SimulationResidentRegistry, SimulationResidentRegistryError,
        },
    };
    let (registry, _) = SimulationResidentRegistry::<_, _, SimulationResidentRegistryError>::new(
        std::convert::identity::<SimulationResidentRegistryError>,
    );
    let first_plan = fixture(9, LAYER_GUIDS[12], 1, Some([64.0; 8]));
    let second_plan = fixture(9, LAYER_GUIDS[0], 1, Some([64.0; 8]));
    let first =
        SimulationInnerRegistryAsset::try_from_operation(Arc::clone(&first_plan), 0).unwrap();
    let second = SimulationInnerRegistryAsset::try_from_operation(second_plan, 0).unwrap();
    registry.try_add_inner(&first).unwrap();
    registry.try_add_inner(&second).unwrap();
    registry.try_remove_inner(&second).unwrap();
    assert!(Arc::ptr_eq(
        &registry.try_inner(CollectionId(9)).unwrap().unwrap(),
        &first
    ));
    registry.try_remove_inner(&first).unwrap();
    assert!(registry.try_inner(CollectionId(9)).unwrap().is_none());
    registry
        .try_add_selected_arena(CollectionId(9), &first_plan)
        .unwrap();
    registry.try_add_inner(&second).unwrap();
    assert!(matches!(
        registry.try_inner(CollectionId(9)),
        Err(SimulationInnerRegistryError::KindMismatch {
            actual: SimulationResidentContainerAssetKind::Arena,
            ..
        })
    ));
}

#[test]
fn tile_id_uses_floor_and_both_signed_low_words() {
    assert_eq!(tile_id([0.0; 4], [128.0; 4]).unwrap(), 100_000);
    assert_eq!(
        tile_id([128.0, 256.0, 128.0, 256.0], [128.0; 4]).unwrap(),
        12_900_256
    );
    assert_eq!(
        tile_id([-0.25, 0.0, -0.25, 0.0], [128.0; 4]).unwrap(),
        0_u32.wrapping_sub(12_700_000)
    );
    assert_eq!(
        tile_id([f32::NAN; 4], [128.0; 4]),
        Err(WorldPainterLayerError::Vector2Assertion)
    );
    assert_eq!(
        tile_id([0.0; 4], [0.0; 4]),
        Err(WorldPainterLayerError::IntegerConversion)
    );
}

#[test]
fn constructor_and_real_registry_layer_publication_preserve_lazy_query() {
    let assets = SelectedSimulationAssetServices::construct();
    let mut layer = WorldPainterLayerMan::construct();
    assert!(layer.assets.is_none());
    assert!(layer.load_in_progress.lock().unwrap().is_empty());
    assert!(layer.tile_list.is_empty());
    assert_eq!(layer.broadcaster_queue, 9);
    assert_eq!(
        layer.broadcaster_port,
        skate2_formats::state_graph::state_graph_string_hash32(b"WorldPainterBroadcaster")
    );
    assert!(
        layer
            .key_cache
            .iter()
            .all(|entry| entry.hint_node.is_none() && entry.remaining_words == [None; 3])
    );
    assert_eq!(
        layer.try_register(5),
        Err(WorldPainterLayerError::MissingAssetManager)
    );
    layer.initialize(Arc::clone(&assets));
    let plan = fixture(5, LAYER_GUIDS[0], 1, Some([64.0; 8]));
    assets.try_process_plan(&plan).unwrap();
    layer.try_register(5).unwrap();
    assert_eq!(layer.tile_count(), 1);
    assert_eq!(layer.populated_layer_count(0), 1);
    assert_eq!(layer.tiles[&100_000].offset.map(f32::to_bits), [0; 4]);
    assert_eq!(layer.try_camera_region([1.0, 2.0]), Ok(None));
    // The populated tree is traversed, not a miss: the fixture root's first
    // child word (0x4280, from its 64.0 centre) points past the tree.
    assert_eq!(
        layer.try_region_name(0, [1.0, 2.0]),
        Err(WorldPainterLayerError::Source(LayerAssetError::Truncated {
            required: 170_242,
            actual: 40
        }))
    );
    layer.try_unregister(5).unwrap();
    assert_eq!(layer.tile_count(), 0);
}

#[test]
fn duplicate_full_id_keeps_first_registry_payload_and_no_layer_miss_for_present_tree() {
    let assets = SelectedSimulationAssetServices::construct();
    let first = fixture(9, LAYER_GUIDS[12], 2, Some([64.0; 8]));
    let second = fixture(9, LAYER_GUIDS[0], 0, Some([64.0; 8]));
    assets.try_process_plan(&first).unwrap();
    assets.try_process_plan(&second).unwrap();
    assert_eq!(assets.retained_counts(), (2, 2));
    let winner = assets.try_inner(9).unwrap().unwrap();
    assert!(Arc::ptr_eq(&winner.plan, &first));
    let mut layer = WorldPainterLayerMan::construct();
    layer.initialize(assets);
    layer.try_register(9).unwrap();
    assert_eq!(layer.populated_layer_count(12), 1);
    assert_eq!(
        layer.try_camera_region([1.0, 2.0]),
        Err(WorldPainterLayerError::Source(LayerAssetError::Truncated {
            required: 170_242,
            actual: 40
        }))
    );
}

#[test]
fn unknown_layer_guid_preserves_new_empty_tile_until_actual_teardown() {
    let assets = SelectedSimulationAssetServices::construct();
    let plan = fixture(11, 0x7777_7777_03e3_87c2, 1, Some([64.0; 8]));
    assets.try_process_plan(&plan).unwrap();
    let mut layer = WorldPainterLayerMan::construct();
    layer.initialize(assets);
    layer.try_register(11).unwrap();
    assert_eq!(layer.tile_count(), 1);
    assert_eq!(layer.try_camera_region([0.0; 2]), Ok(None));
    layer.try_unregister(11).unwrap();
    assert_eq!(
        layer.tile_count(),
        1,
        "unknown GUID returns before clear/empty-scan"
    );
}

#[test]
fn tile_assertion_checks_only_xy_nan_after_native_self_comparison() {
    assert_eq!(
        tile_id([128.0, 256.0, f32::NAN, -17.0], [128.0; 4]).unwrap(),
        12_900_256
    );
    assert_eq!(
        tile_id([128.0, f32::NAN, 128.0, 256.0], [128.0; 4]),
        Err(WorldPainterLayerError::Vector2Assertion)
    );
    assert_eq!(
        tile_id([f32::INFINITY, 0.0, 0.0, 0.0], [128.0; 4]),
        Err(WorldPainterLayerError::IntegerConversion)
    );
}
