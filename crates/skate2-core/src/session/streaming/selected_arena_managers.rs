//! Real selected arena collaborators; the adapter profile owns explicit exclusions.
//! These legacy void callbacks must never stand in for unreconstructed managers.
use crate::{
    living_world_mesh_store::{MeshStore, MeshStoreError},
    simulation_normal_arena_plan::SimulationRecordHandle,
    simulation_pointer_arena_adapter::SimulationPointerArenaDispatch,
    simulation_registry_arena_adapter::SimulationRegistryArenaDispatch,
    simulation_static_arena_adapter::SimulationStaticGrindDispatch,
    world_painter_layer::{WorldPainterLayerError, WorldPainterLayerMan},
};
use std::sync::{Arc, Mutex};
#[path = "../living_world/selected_living_world_rain.rs"]
pub(crate) mod rain;

pub(crate) struct SelectedArenaManagers(
    pub(crate) Arc<MeshStore>,
    pub(crate) Arc<Mutex<WorldPainterLayerMan>>,
    pub(crate) Arc<Mutex<crate::trigger_camera_group::CameraTriggerGroup>>,
    pub(crate) Option<rain::SelectedRainRoute>,
    /// `AIPath::PathManager` and the registry its `AddAsset` reads.
    pub(crate) Arc<Mutex<crate::ai::path_manager::PathManager>>,
    pub(crate) Arc<crate::selected_simulation_asset_services::SelectedSimulationAssetServices>,
);
impl SimulationStaticGrindDispatch for SelectedArenaManagers {
    fn add_spline_data(&mut self, _: &crate::grind::GrindAsset) {
        panic!("native grind route used legacy add");
    }
    fn remove_spline_data(&mut self, _: u64) {
        panic!("native grind route used legacy remove");
    }
}
impl SimulationPointerArenaDispatch for SelectedArenaManagers {
    fn add_dmo_model(&mut self, _id: u64) {
        unreachable!("selected profile must exclude this manager operation");
    }
    fn register_dmo_model(&mut self, _: &SimulationRecordHandle, _id: u64) {
        unreachable!("selected profile must exclude this manager operation");
    }
    fn remove_dmo_model(&mut self, _id: u64) {
        unreachable!("selected profile must exclude this manager operation");
    }
    fn deregister_dmo_model(&mut self, _: &SimulationRecordHandle, _id: u64) {
        unreachable!("selected profile must exclude this manager operation");
    }
    fn add_trigger_volume(&mut self, record: &SimulationRecordHandle) {
        self.2
            .lock()
            .expect("trigger manager lock")
            .try_register(record)
            .expect("trigger registration must not hide refusal");
    }
    fn remove_trigger_volume(&mut self, record: &SimulationRecordHandle) {
        self.2
            .lock()
            .expect("trigger manager lock")
            .try_remove(record)
            .expect("trigger removal must not hide refusal");
    }
}
impl SimulationRegistryArenaDispatch for SelectedArenaManagers {
    fn register_streamed_tile(&mut self, id: u64) {
        self.try_register_streamed_tile(id)
            .expect("legacy layer registration must not hide refusal");
    }
    fn unregister_streamed_tile(&mut self, id: u64) {
        self.try_unregister_streamed_tile(id)
            .expect("legacy layer removal must not hide refusal");
    }
    fn try_register_streamed_tile(&mut self, id: u64) -> Result<(), WorldPainterLayerError> {
        self.1
            .lock()
            .map_err(|_| WorldPainterLayerError::OwnerLock)?
            .try_register(id)
    }
    fn try_unregister_streamed_tile(&mut self, id: u64) -> Result<(), WorldPainterLayerError> {
        self.1
            .lock()
            .map_err(|_| WorldPainterLayerError::OwnerLock)?
            .try_unregister(id)
    }
    fn add_nav_mesh_data(&mut self, id: u64) {
        self.0
            .try_add_tile(id)
            .expect("legacy mesh add must not hide refusal");
    }
    fn remove_nav_mesh_data(&mut self, id: u64) {
        self.0
            .try_remove_tile(id)
            .expect("legacy mesh removal must not hide refusal");
    }
    fn try_add_nav_mesh_data(&mut self, id: u64) -> Result<(), MeshStoreError> {
        self.0.try_add_tile(id)
    }
    fn try_remove_nav_mesh_data(&mut self, id: u64) -> Result<(), MeshStoreError> {
        self.0.try_remove_tile(id)
    }
    fn selected_rain_enabled(&self) -> bool {
        self.3.is_some()
    }
    fn add_rain_data(&mut self, id: u64) {
        self.try_add_rain_data(id)
            .expect("legacy RAIN admission must not hide refusal");
    }
    fn remove_rain_data(&mut self, id: u64) {
        self.try_remove_rain_data(id)
            .expect("legacy RAIN removal must not hide refusal");
    }
    fn try_add_rain_data(
        &mut self,
        id: u64,
    ) -> Result<(), crate::simulation_registry_arena_adapter::RainNetworkError> {
        self.3
            .as_ref()
            .ok_or(crate::simulation_registry_arena_adapter::RainNetworkError::MissingOwner)?
            .add(id)
    }
    fn try_remove_rain_data(
        &mut self,
        id: u64,
    ) -> Result<(), crate::simulation_registry_arena_adapter::RainNetworkError> {
        self.3
            .as_ref()
            .ok_or(crate::simulation_registry_arena_adapter::RainNetworkError::MissingOwner)?
            .remove(id)
    }
    fn add_ai_path_asset(&mut self, id: u64) {
        self.4
            .lock()
            .expect("AI path manager lock")
            .try_add_asset(id, &self.5)
            .expect("AI path admission must not hide refusal");
    }
    fn remove_ai_path_asset(&mut self, id: u64) {
        self.4
            .lock()
            .expect("AI path manager lock")
            .try_remove_asset(id, &self.5)
            .expect("AI path removal must not hide refusal");
    }
    fn add_spatial_map_asset(&mut self, _id: u64) {
        unreachable!("selected profile must exclude this manager operation");
    }
    fn remove_spatial_map_asset(&mut self, _id: u64) {
        unreachable!("selected profile must exclude this manager operation");
    }
    fn register_waypoint_data(&mut self, _id: u64, _streamed: bool) {
        unreachable!("selected profile must exclude this manager operation");
    }
    fn deregister_waypoint_data(&mut self, _id: u64, _streamed: bool) {
        unreachable!("selected profile must exclude this manager operation");
    }
}
