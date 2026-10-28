use super::*;
use crate::attribute_runtime::tests::{add_fixture_class, collection, definition, graph, node};

fn fixture() -> Arc<AttributeRuntimeVault> {
    const KEY: u64 = 0x1122_3344_5566_7788;
    let mut vault = graph(LIVING_WORLD_CLASS, vec![definition(KEY, 4, 4)]);
    add_fixture_class(&mut vault, DEBUG_CLASS, vec![definition(KEY, 4, 4)]);
    for (class, name, value) in [
        (LIVING_WORLD_CLASS, TRAFFIC_LIGHT_COLLECTION, 11),
        (LIVING_WORLD_CLASS, RAIN_COLLECTION, 22),
        (DEBUG_CLASS, DEBUG_COLLECTION, 33),
    ] {
        collection(
            &mut vault,
            class,
            name,
            0,
            None,
            vec![node(KEY, 0x40, value)],
        );
    }
    Arc::new(vault)
}

#[test]
fn source_pools_recycle_and_spatial_reset_leaves_road_entity_pools_alone() {
    let mut rain = RainMan::construct(&fixture());
    rain.initialize_storage().unwrap();
    assert_eq!(
        rain.memory
            .each_ref()
            .map(|m| (m.maximum_member_size, m.maximum_members)),
        [(0xbf0, 72), (0xe0, 144), (0x98, 144), (8, 3168)]
    );
    assert_eq!(rain.memory[1].request().unwrap(), Some(0));
    rain.memory[1].release(0).unwrap();
    assert_eq!(rain.memory[1].request().unwrap(), Some(0));
    for slot in 0..3168 {
        assert_eq!(rain.memory[3].request().unwrap(), Some(slot));
    }
    assert_eq!(rain.memory[3].request().unwrap(), None);
    rain.reset_spatial().unwrap();
    assert_eq!(rain.memory[3].request().unwrap(), Some(0));
    assert_eq!(rain.memory[1].request().unwrap(), Some(1));
}

#[test]
fn actual_constructor_retains_unwritten_memory_and_light_sync_state() {
    let rain = RainMan::construct(&fixture());
    for memory in &rain.memory {
        assert_eq!((memory.maximum_member_size, memory.maximum_members), (0, 0));
        assert!(memory.free_list.is_none());
        assert!(memory.managed_memory.is_none());
        assert_eq!(memory.free_list_index, None);
    }
    for light in &rain.traffic_lights {
        assert_eq!(light.light_states.count, 0);
        assert_eq!(light.pedestrian_states.count, 0);
        assert_eq!(light.light_states.records.len(), 8);
        assert_eq!(light.pedestrian_states.records.len(), 8);
        assert_eq!(
            (light.current_state, light.current_pedestrian_state),
            (-1, -1)
        );
        assert_eq!(light.brisk_walk_duration_percentage.to_bits(), 0);
        assert_eq!(light.sync_to_red_state, None);
        assert_eq!(light.sync_to_red_pedestrian_state, None);
        assert_eq!(light.sync_to_green_state, None);
        assert_eq!(light.sync_to_green_pedestrian_state, None);
    }
    assert!(rain.chunks.is_empty());
    assert!(rain.pending_tree_chunks.is_empty());
    assert!(rain.intersections.is_empty());
    assert!(rain.roads.is_empty());
    assert!(rain.super_roads.is_empty());
    assert!(rain.intersection_points.is_none());
    assert!(rain.road_points.is_none());
    assert!(rain.spatial_items.is_empty());
    assert!(rain.entity_points.is_empty());
    assert!(rain.skater_points.is_empty());
    for vector in [rain.data_offset, rain.data_width] {
        assert_eq!(vector.xyz.map(f32::to_bits), [0; 3]);
        assert_eq!(vector.w.map(f32::to_bits), Some(0));
    }
    assert!(!rain.vehicle_light_bias_registered);
    assert_eq!(rain.light_biased_vehicle_id, None);
    assert!(!rain.syncing_even_lights_green);
    assert!(!rain.syncing_odd_lights_green);
    assert!(!rain.lock_lights);
}

#[test]
fn three_real_bindings_retain_the_same_vault_and_distinct_source_collections() {
    let vault = fixture();
    let count = Arc::strong_count(&vault);
    let rain = RainMan::construct(&vault);
    assert_eq!(Arc::strong_count(&vault), count + 3);
    for (binding, value) in [
        (&rain.traffic_attributes, 11),
        (&rain.rain_attributes, 22),
        (&rain.debug_attributes, 33),
    ] {
        assert!(!binding.is_null());
        assert_eq!(
            binding.try_scalar_word(0x1122_3344_5566_7788, 0).unwrap(),
            Some(value)
        );
    }
    drop(rain);
    assert_eq!(Arc::strong_count(&vault), count);
}

#[test]
fn road_consumer_uses_supplied_store_and_keeps_pending_tile_failure() {
    let rain = RainMan::construct(&fixture());
    let mesh = MeshStore::construct();
    let position = SkeletonRootVector {
        xyz: [f32::NAN, 1.0, 2.0],
        w: None,
    };
    assert!(!rain.try_point_on_road_network(&mesh, position).unwrap());
    let error = MeshStoreError::PendingTileAdmission {
        asset_id: 0xfedc_ba98_7654_3210,
    };
    assert_eq!(mesh.try_add_tile(0xfedc_ba98_7654_3210), Err(error));
    assert_eq!(rain.try_point_on_road_network(&mesh, position), Err(error));
}
