//! The terrain adapter routes RAIN to the retained simulation LivingWorld.
use crate::{
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{
        LivingWorldGrindServices, dmo_objects::DmoObjectMan, entities::EntityMan,
    },
    living_world_rain::{RainMan, network::RainNetworkError, removal::RainRemovalServices},
    selected_simulation_asset_services::SelectedSimulationAssetServices,
};
use std::sync::{Arc, Mutex};

pub(crate) struct SelectedRainRoute {
    pub(crate) living_world: Arc<Mutex<LivingWorldGrindServices>>,
    pub(crate) assets: Arc<SelectedSimulationAssetServices>,
    pub(crate) cvars: InputCvarReferences,
}
/// Streamed road removal visits the actual vehicle roster and the same active
/// cull/destructor path used by the population census.
struct Removal<'a> {
    pedestrians: &'a mut EntityMan<40, crate::living_world_pedestrian_entity::PedestrianConstruction>,
    vehicles: &'a mut EntityMan<24, crate::living_world_grind_services::vehicle_entity::Vehicle>,
    objects: &'a mut DmoObjectMan,
    execution:
        &'a mut crate::living_world_grind_services::vehicle_entity::execution::VehicleExecutionMan,
    pending: &'a mut Vec<crate::living_world_grind_services::vehicle_entity::Vehicle>,
    mesh: &'a crate::living_world_mesh_store::MeshStore,
    random: &'a mut crate::native_skater_random::NativeSkaterRandom,
    cvars: &'a InputCvarReferences,
}
impl RainRemovalServices for Removal<'_> {
    fn vehicle_road(&mut self, id: u32) -> Result<u64, RainNetworkError> {
        self.vehicles
            .get(id)
            .map(|v| v.route.item_id)
            .ok_or(RainNetworkError::Vehicle(id))
    }
    fn remove_and_cull_vehicle(
        &mut self,
        rain: &mut RainMan,
        id: u32,
    ) -> Result<(), RainNetworkError> {
        // RemoveRAINData removes road membership before RequestCull's repeated
        // removal. A vehicle can occur in both parking and travelling lists;
        // an already-detached occurrence must not be destroyed twice.
        let Some(vehicle) = self.vehicles.get_mut(id) else {
            return if self.pending.iter().any(|v| v.id == id) {
                Ok(())
            } else {
                Err(RainNetworkError::Vehicle(id))
            };
        };
        let length = vehicle.length();
        vehicle
            .route
            .remove_from_road_network::<std::convert::Infallible, std::convert::Infallible>(
                &mut vehicle.motion,
                id,
                length,
                rain,
                self.mesh,
            )
            .map_err(|_| RainNetworkError::Vehicle(id))?;
        crate::living_world_grind_services::vehicle_entity::frame::request_cull(
            id,
            self.vehicles,
            self.execution,
            self.pending,
            rain,
            self.mesh,
            self.random,
            self.cvars,
        )
        .map_err(|_| RainNetworkError::Vehicle(id))
    }
    fn invalidate_entity_rain_data(&mut self) -> Result<(), RainNetworkError> {
        for id in self.pedestrians.ids() {
            self.pedestrians.get_mut(id).expect("pedestrian entity order").rain_data = None;
        }
        for id in self.vehicles.ids() {
            self.vehicles.get_mut(id).expect("entity order").rain_data = None;
        }
        // The selected prop roster also has no published entities.
        self.objects.invalidate_rain_data();
        Ok(())
    }
}
impl SelectedRainRoute {
    pub(crate) fn bind_assets(&self) -> Result<(), RainNetworkError> {
        let mut living = self
            .living_world
            .lock()
            .map_err(|_| RainNetworkError::OwnerLock)?;
        let rain = living.rain.as_mut().ok_or(RainNetworkError::MissingOwner)?;
        if let Some(existing) = &rain.assets {
            if !Arc::ptr_eq(existing, &self.assets) {
                return Err(RainNetworkError::AlreadyInitialized);
            }
        } else {
            rain.assets = Some(Arc::clone(&self.assets));
        }
        Ok(())
    }
    pub(crate) fn add(&self, id: u64) -> Result<(), RainNetworkError> {
        self.living_world
            .lock()
            .map_err(|_| RainNetworkError::OwnerLock)?
            .rain
            .as_mut()
            .ok_or(RainNetworkError::MissingOwner)?
            .add_data(id, &self.assets)
    }
    pub(crate) fn remove(&self, id: u64) -> Result<(), RainNetworkError> {
        let incremental = self
            .cvars
            .try_rain_enable_kd_tree_removals()
            .map_err(|_| RainNetworkError::CvarRead)?;
        let mut living = self
            .living_world
            .lock()
            .map_err(|_| RainNetworkError::OwnerLock)?;
        let mesh = living
            .try_mesh_store()
            .map_err(|_| RainNetworkError::MissingOwner)?;
        let LivingWorldGrindServices {
            rain,
            pedestrian_entities,
            grind_vehicle_entities,
            dmo_objects,
            vehicle_execution,
            vehicle_culls,
            random,
            ..
        } = &mut *living;
        let mut services = Removal {
            pedestrians: pedestrian_entities
                .as_mut()
                .ok_or(RainNetworkError::MissingOwner)?,
            vehicles: grind_vehicle_entities
                .as_mut()
                .ok_or(RainNetworkError::MissingOwner)?,
            objects: dmo_objects.as_mut().ok_or(RainNetworkError::MissingOwner)?,
            execution: vehicle_execution,
            pending: vehicle_culls,
            mesh: &mesh,
            random: random.as_mut().ok_or(RainNetworkError::MissingOwner)?,
            cvars: &self.cvars,
        };
        rain.as_mut()
            .ok_or(RainNetworkError::MissingOwner)?
            .remove_data(id, &self.assets, incremental, &mut services)
    }
}
