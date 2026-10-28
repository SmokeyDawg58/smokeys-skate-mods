//! Actual RAIN constructor state required by the first camera service capture.
//! Traffic initialization, runtime updates, and native global publication are
//! separate producers. Road queries use the same world `MeshMan` child.
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    living_world_mesh_store::{MeshStore, MeshStoreError},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{collections::BTreeMap, mem::MaybeUninit, sync::Arc};
#[path = "living_world_rain_asset.rs"]
pub(crate) mod asset;
#[path = "living_world_rain_chunks.rs"]
mod chunks;
#[path = "living_world_rain_curve.rs"]
pub(crate) mod curve;
#[path = "living_world_rain_dynamic_passage.rs"]
pub(crate) mod dynamic_passage;
#[path = "living_world_rain_geometry.rs"]
pub(crate) mod geometry;
#[path = "living_world_rain_intersection_vehicles.rs"]
pub(crate) mod intersection_vehicles;
#[path = "living_world_nav_targets.rs"]
pub(crate) mod nav_targets;
#[path = "living_world_rain_network.rs"]
pub(crate) mod network;
#[path = "living_world_rain_ped_nodes.rs"]
pub(crate) mod ped_nodes;
use network::{Intersection, Road, SuperRoad};
#[path = "living_world_rain_removal.rs"]
pub(crate) mod removal;
#[path = "living_world_rain_spatial.rs"]
pub(crate) mod spatial;
use crate::living_world_grind_services::dmo_locators::LocatorKdTree;
#[path = "living_world_traffic_light.rs"]
pub(crate) mod traffic;
#[path = "living_world_rain_vehicle_lanes.rs"]
pub(crate) mod vehicle_lanes;
use traffic::LightState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RainServiceError {
    MissingConstruction,
    Mesh(MeshStoreError),
}

/// `MemStore82a6dd08` constructor state, before its distinct Init. In particular,
/// free-list index is unwritten; initialized `LivingWorldMemStoreSlots` would
/// manufacture a valid allocation policy at this earlier boundary.
pub(crate) struct RainMemStoreConstructor {
    pub(crate) maximum_member_size: u32,
    pub(crate) maximum_members: u32,
    pub(crate) free_list: Option<Box<[u32]>>,
    pub(crate) managed_memory: Option<Box<[MaybeUninit<u8>]>>,
    pub(crate) free_list_index: Option<i32>,
}
impl RainMemStoreConstructor {
    fn construct() -> Self {
        Self {
            maximum_member_size: 0,
            maximum_members: 0,
            free_list: None,
            managed_memory: None,
            free_list_index: None,
        }
    }
}

/// Actual fixed-vector inline storage: no light-state record is initialized by
/// the constructor. Native capacity is eight records of sixteen bytes each.
pub(crate) struct LightStates {
    pub(crate) records: [Option<LightState>; 8],
    pub(crate) count: usize,
}
impl LightStates {
    fn construct() -> Self {
        Self {
            records: [None; 8],
            count: 0,
        }
    }
}
pub(crate) struct TrafficLight {
    pub(crate) light_states: LightStates,
    pub(crate) pedestrian_states: LightStates,
    pub(crate) current_state: i32,
    pub(crate) current_pedestrian_state: i32,
    pub(crate) brisk_walk_duration_percentage: f32,
    pub(crate) sync_to_red_state: Option<u32>,
    pub(crate) sync_to_red_pedestrian_state: Option<u32>,
    pub(crate) sync_to_green_state: Option<u32>,
    pub(crate) sync_to_green_pedestrian_state: Option<u32>,
}
impl TrafficLight {
    /// `TrafficLight82e5de10`: two empty fixed vectors, indices -1, brisk walk 0.
    /// The four synchronization fields at +130..13c are not initialized here.
    fn construct() -> Self {
        Self {
            light_states: LightStates::construct(),
            pedestrian_states: LightStates::construct(),
            current_state: -1,
            current_pedestrian_state: -1,
            brisk_walk_duration_percentage: 0.0,
            sync_to_red_state: None,
            sync_to_red_pedestrian_state: None,
            sync_to_green_state: None,
            sync_to_green_pedestrian_state: None,
        }
    }
}

/// PDB LWRainPoint: point+0, VecFloat radius+10, compulsoryStop+20,
/// signed pedestrian ID+24. The constructor still admits no points.
#[derive(Clone, Copy)]
pub(crate) struct RainPoint {
    pub(crate) position: [f32; 4],
    pub(crate) radius: [f32; 4],
    pub(crate) compulsory_stop: bool,
    pub(crate) pedestrian_id: i32,
}

/// Class/collection identities reconstructed from raw 64-bit arguments at
/// `8271e31c/33c/35c`; the decompiler splits these arguments incorrectly.
pub(crate) const LIVING_WORLD_CLASS: u64 =
    crate::living_world_recipe_manager::LIVING_WORLD_CONFIG_CLASS_ID;
pub(crate) const TRAFFIC_LIGHT_COLLECTION: u64 = 0x8355_87ec_ad20_014e;
pub(crate) const RAIN_COLLECTION: u64 = 0x991d_f788_b8f6_1295;
pub(crate) const DEBUG_CLASS: u64 = 0xff19_68a5_c467_bcf9;
pub(crate) const DEBUG_COLLECTION: u64 = 0xf97b_9086_caa5_0c53;

#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent native constructor flags preserve separate publication and consumers"
)]
pub(crate) struct RainMan {
    pub(crate) assets:
        Option<Arc<crate::selected_simulation_asset_services::SelectedSimulationAssetServices>>,
    pub(crate) memory: [RainMemStoreConstructor; 4],
    pub(crate) traffic_lights: [TrafficLight; 4],
    pub(crate) data_offset: SkeletonRootVector,
    pub(crate) data_width: SkeletonRootVector,
    pub(crate) chunks: chunks::RainChunks,
    pub(crate) pending_tree_chunks: chunks::RainChunks,
    pub(crate) intersections: BTreeMap<u64, Box<Intersection>>,
    pub(crate) roads: BTreeMap<u64, Box<Road>>,
    pub(crate) super_roads: BTreeMap<u64, Box<SuperRoad>>,
    /// None until RAIN.Init; the shared KD implementation owns its node pool.
    pub(crate) intersection_points: Option<LocatorKdTree>,
    pub(crate) road_points: Option<LocatorKdTree>,
    pub(crate) spatial_items: Vec<Option<spatial::RainItem>>,
    pub(crate) entity_points: Vec<RainPoint>,
    pub(crate) skater_points: Vec<RainPoint>,
    pub(crate) traffic_attributes: AttributeRuntimeInstance,
    pub(crate) rain_attributes: AttributeRuntimeInstance,
    pub(crate) debug_attributes: AttributeRuntimeInstance,
    pub(crate) vehicle_light_bias_registered: bool,
    pub(crate) light_biased_vehicle_id: Option<u32>,
    pub(crate) syncing_even_lights_green: bool,
    pub(crate) syncing_odd_lights_green: bool,
    pub(crate) lock_lights: bool,
}
impl RainMan {
    pub(crate) fn clear_light_biased_vehicle(&mut self) {
        // ClearLightBiasedVehicle8271b4d0 leaves the stored ID unchanged.
        self.vehicle_light_bias_registered = false;
        self.lock_lights = false;
        self.syncing_even_lights_green = false;
        self.syncing_odd_lights_green = false;
    }
    /// `RAINMan8271e178`: constructor fields only, in native order. The four
    /// memory stores are nominal host inline ownership of the native vector's
    /// four constructed elements; no initialized store is substituted.
    pub(crate) fn construct(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            assets: None,
            memory: std::array::from_fn(|_| RainMemStoreConstructor::construct()),
            traffic_lights: std::array::from_fn(|_| TrafficLight::construct()),
            data_offset: SkeletonRootVector::known([0.0; 4]),
            data_width: SkeletonRootVector::known([0.0; 4]),
            chunks: chunks::RainChunks::new(),
            pending_tree_chunks: chunks::RainChunks::new(),
            intersections: BTreeMap::new(),
            roads: BTreeMap::new(),
            super_roads: BTreeMap::new(),
            intersection_points: None,
            road_points: None,
            spatial_items: Vec::new(),
            entity_points: Vec::new(),
            skater_points: Vec::new(),
            traffic_attributes: vault.instance(LIVING_WORLD_CLASS, TRAFFIC_LIGHT_COLLECTION),
            rain_attributes: vault.instance(LIVING_WORLD_CLASS, RAIN_COLLECTION),
            debug_attributes: vault.instance(DEBUG_CLASS, DEBUG_COLLECTION),
            vehicle_light_bias_registered: false,
            light_biased_vehicle_id: None,
            syncing_even_lights_green: false,
            syncing_odd_lights_green: false,
            lock_lights: false,
        }
    }

    /// Native virtual `PointOnRoadNetwork826f3a30` does not inspect RAIN fields;
    /// its caller supplies the SAME world `MeshMan` store that native LW captures.
    #[expect(
        clippy::unused_self,
        reason = "native RAIN virtual call reads only the captured world mesh owner"
    )]
    pub(crate) fn try_point_on_road_network(
        &self,
        mesh: &MeshStore,
        position: SkeletonRootVector,
    ) -> Result<bool, MeshStoreError> {
        mesh.try_point_on_road_network(position)
    }
}

#[cfg(test)]
#[path = "living_world_rain_tests.rs"]
mod tests;
