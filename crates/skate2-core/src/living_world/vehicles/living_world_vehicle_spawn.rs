//! VehicleCensus.Spawn82d27de0 through the returned, not-yet-attached vehicle.
//! All owners are borrowed from the retained LivingWorld/physics world.
//! RequestSpawn's AttachToManagers/OnSpawnComplete remains the outer caller.
use super::{
    Vehicle, VehicleLookupError,
    factory::{VehicleFactory, VehicleFactoryError},
    model_init::VehicleModelInitError,
    spawn_overlap::SpawnOverlapError,
    spawn_road::{SpawnRoadError, VehicleSpawnRoad},
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_grind_services::{
        attrib_cache::{AttribCacheError, CensusTargetCache},
        collideable_objects::CollideableObjectMan,
        dmo_models::DmoModelMan,
        dmo_objects::{DmoObjectMan, ENTITIES_CLASS},
        entities::EntityMan,
        spline_registration::{GrabbableObjectMan, LivingWorldSplineIds},
        vehicle_motion::{attributes::VehicleAttributeError, driving::DrivingError},
    },
    living_world_mesh_store::MeshStore,
    living_world_rain::{RainMan, geometry::Transform},
    native_skater_random::{
        NativeRandomGate, NativeRandomLocked, NativeSkaterRandom,
        rw_math::{RwRandomLockError, global_next},
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    player::mounted_recipe::MountedRecipeRegistry,
    reckoning_numeric::ReckoningNumeric,
    world::LivingWorldEntityIdAllocator,
};
use std::sync::Arc;

/// Fields actually read by this category's Spawn. The common spawn packet's
/// allowWorld/loadModel/initial flags are NOT gates in VehicleCensus.Spawn.
pub(crate) struct VehicleSpawnInfo {
    pub(crate) entity_key: u64,
    pub(crate) model_key: u64,
    pub(crate) recipe_id: u64,
    pub(crate) transform: Transform,
    pub(crate) allow_culling: bool,
    pub(crate) force_spawn: bool,
}

#[derive(Debug)]
pub(crate) enum VehicleSpawnError<N> {
    MissingOwner(&'static str),
    Mesh(crate::living_world_mesh_store::MeshStoreError),
    Graphs(crate::actor_construction::graph_startup::ActorGraphStartupError),
    UnfinishedSpawn,
    Entity(crate::living_world_grind_services::entities::EntityMutationError),
    Execution(super::execution::VehicleExecutionError),
    DuplicateVehicleId(u32),
    Road(SpawnRoadError<VehicleLookupError>),
    Cache(AttribCacheError),
    Factory(VehicleFactoryError),
    Model(VehicleModelInitError),
    Random(NativeRandomLocked),
    Driving(DrivingError<VehicleLookupError, N>),
    Overlap(SpawnOverlapError),
    Attributes(VehicleAttributeError<RwRandomLockError>),
    Collision(super::collision::VehicleCollisionError),
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    ContainedPropCapacity(u32),
    ContainedPropSpawnPending { entity_key: u64, count: u32 },
    PedestrianCullNotification { pedestrian: u32, source: String },
}

pub(crate) struct VehicleSpawnFrame<'a> {
    pub(crate) factory: &'a mut VehicleFactory,
    pub(crate) caches: &'a mut CensusTargetCache,
    pub(crate) ids: &'a mut LivingWorldEntityIdAllocator,
    pub(crate) vault: &'a Arc<AttributeRuntimeVault>,
    pub(crate) models: &'a DmoModelMan,
    pub(crate) recipes: &'a MountedRecipeRegistry,
    pub(crate) assemblies: &'a mut CollideableObjectMan,
    pub(crate) grabbables: &'a mut GrabbableObjectMan,
    pub(crate) spline_ids: &'a mut LivingWorldSplineIds,
    pub(crate) world: &'a mut PhysicalAssemblyWorld,
    pub(crate) rain: &'a mut RainMan,
    pub(crate) mesh: &'a MeshStore,
    pub(crate) vehicles: &'a mut EntityMan<24, Vehicle>,
    pub(crate) pedestrians: &'a mut EntityMan<40, crate::living_world_pedestrian_entity::PedestrianConstruction>,
    pub(crate) dmos: &'a DmoObjectMan,
    pub(crate) random: &'a mut NativeSkaterRandom,
    pub(crate) cvars: &'a InputCvarReferences,
    /// Captured delegate registration from PreLoadStateGraphs, not per-spawn tuning.
    pub(crate) enablement: crate::generated_onboard_state_graph::GeneratedOnboardEnablement,
    /// Juice::JuiceRPCNG::sIsOverrideModelName, distinct from force_spawn.
    pub(crate) override_model_name: bool,
    /// Only a diagnostic failure leaves a vehicle here. Its native prefix
    /// (including any admitted body) survives for explicit teardown/recovery.
    pub(crate) unfinished: &'a mut Option<Vehicle>,
}

impl VehicleSpawnFrame<'_> {
    pub(crate) fn spawn<N: ReckoningNumeric>(
        &mut self,
        info: Option<&VehicleSpawnInfo>,
        numeric: &mut N,
    ) -> Result<Option<Vehicle>, VehicleSpawnError<N::Error>> {
        use VehicleSpawnError as E;
        if self.unfinished.is_some() {
            return Err(E::UnfinishedSpawn);
        }
        let Some(info) = info else { return Ok(None) };
        let Some(road) = VehicleSpawnRoad::select(
            self.rain,
            info.transform[3],
            self.override_model_name,
            self.vehicles,
        )
        .map_err(E::Road)?
        else {
            return Ok(None);
        };
        let cache = self
            .caches
            .get(info.model_key, self.vault)
            .map_err(E::Cache)?;
        let Some(mut vehicle) = self
            .factory
            .create(self.ids, info.entity_key, info.model_key, cache, self.vault)
            .map_err(E::Factory)?
        else {
            return Ok(None);
        };
        // A checked host failure is fatal to this spawn. Do not erase its
        // prefix, drop a live body, or turn it into a normal null spawn result.
        match self.finish(&mut vehicle, info, &road, numeric) {
            Ok(true) => Ok(Some(vehicle)),
            Ok(false) => match self.destroy_rejected::<N::Error>(&mut vehicle) {
                Ok(()) => Ok(None),
                Err(error) => {
                    *self.unfinished = Some(vehicle);
                    Err(error)
                }
            },
            Err(error) => {
                *self.unfinished = Some(vehicle);
                Err(error)
            }
        }
    }

    fn finish<N: ReckoningNumeric>(
        &mut self,
        vehicle: &mut Vehicle,
        info: &VehicleSpawnInfo,
        road: &VehicleSpawnRoad,
        numeric: &mut N,
    ) -> Result<bool, VehicleSpawnError<N::Error>> {
        use VehicleSpawnError as E;
        vehicle
            .initialize_models(
                self.factory,
                info.recipe_id,
                self.vault,
                self.models,
                self.recipes,
                self.assemblies,
                self.grabbables,
                self.spline_ids,
                self.world,
                self.cvars,
                self.enablement.delegate_raw(2).is_some_and(|raw| raw != 0),
            )
            .map_err(E::Model)?;
        let collision = vehicle
            .collision
            .as_ref()
            .expect("InitModels creates controller");
        if collision.model_dimensions().is_none() {
            return Ok(false);
        }
        let lane = road
            .choose_lane(self.random, &NativeRandomGate::production_shared())
            .map_err(E::Random)?;
        vehicle
            .add_to_road_network(
                road.super_position,
                road.road_id,
                lane,
                self.rain,
                self.mesh,
                self.vehicles,
                &mut |id| Err(VehicleLookupError::CullPending(id)),
                numeric,
            )
            .map_err(E::Driving)?;
        if !vehicle
            .clear_of_spawn_dmos(info.force_spawn, self.dmos, self.cvars)
            .map_err(E::Overlap)?
        {
            return Ok(false);
        }
        let length = vehicle.length();
        vehicle
            .motion
            .update_attributes(
                length,
                &vehicle.characteristics,
                &vehicle.driver,
                &mut global_next,
            )
            .map_err(E::Attributes)?;
        vehicle
            .collision
            .as_mut()
            .expect("retained controller")
            .add_models_to_world(&mut vehicle.core, self.world, true, 4, 4)
            .map_err(E::Collision)?;
        vehicle.core.allow_culling = info.allow_culling;
        self.initialize_contained_props::<N::Error>(vehicle)?;
        Ok(true)
    }

    /// LWBehaviourSink.InitContainedProps82e615a0. Shipped Vehicle.xml entity
    /// collections currently have no refs. Nonempty data must reach SpawnProp,
    /// so reject that unconnected branch rather than silently omit children.
    fn initialize_contained_props<N>(&self, vehicle: &Vehicle) -> Result<(), VehicleSpawnError<N>> {
        use VehicleSpawnError as E;
        if !self.cvars.try_allow_contained_props().map_err(E::Cvar)? {
            return Ok(());
        }
        let attributes = self.vault.instance(ENTITIES_CLASS, vehicle.core.entity_key);
        let count = attributes
            .try_length(0x3eb7_7fb8_ca81_c99b)
            .map_err(E::Attribute)?;
        if count > 8 {
            return Err(E::ContainedPropCapacity(count));
        }
        // Native separately resolves GetLength again before the first loop.
        let count = attributes
            .try_length(0x3eb7_7fb8_ca81_c99b)
            .map_err(E::Attribute)?;
        if count != 0 {
            return Err(E::ContainedPropSpawnPending {
                entity_key: vehicle.core.entity_key,
                count,
            });
        }
        Ok(())
    }

    /// Rejected Spawn calls the real destructor before FreeBlock. This path
    /// precedes contained props/plugins/AttachToManagers, so none exist here.
    fn destroy_rejected<N>(&mut self, vehicle: &mut Vehicle) -> Result<(), VehicleSpawnError<N>> {
        vehicle.destroy_detached(
            self.pedestrians,
            self.rain,
            self.mesh,
            self.factory,
            self.assemblies,
            self.grabbables,
            self.world,
        )
    }
}

impl Vehicle {
    /// Destructor shared by rejected construction and detached census culls.
    /// Active culls must FIRST execute OnCullRequested/DetachFromManagers.
    pub(crate) fn destroy_detached<N>(
        &mut self,
        pedestrians: &mut EntityMan<40, crate::living_world_pedestrian_entity::PedestrianConstruction>,
        rain: &mut RainMan,
        mesh: &MeshStore,
        factory: &mut VehicleFactory,
        assemblies: &mut CollideableObjectMan,
        grabbables: &mut GrabbableObjectMan,
        world: &mut PhysicalAssemblyWorld,
    ) -> Result<(), VehicleSpawnError<N>> {
        use VehicleSpawnError as E;
        // Manager ILWManager vtable820d4150+20 ->826f5500 (folded Prop name)
        // -> NotifyPedsAboutCulls82e84d90, in native EntityMan map order.
        for id in pedestrians.ids() {
            pedestrians.get_mut(id).expect("pedestrian entity order")
                .notify_entity_culled(self.id)
                .map_err(|source| E::PedestrianCullNotification { pedestrian: id, source })?;
        }
        let length = self.length();
        self.route
            .remove_from_road_network(&mut self.motion, self.id, length, rain, mesh)
            .map_err(E::Driving)?;
        self.deinitialize_collision(factory, assemblies, world)
            .map_err(E::Model)?;
        factory.free_core(&self.allocation).map_err(E::Factory)?;
        self.route.lane_change_passage.deinit();
        // Native grind DeInit82e62da0, then grab DeInit82e62c00: local erase
        // before global owner erase, ascending IDs captured before iteration.
        self.grind_splines.clear();
        let grabs: Vec<_> = self.grab_splines.keys().copied().collect();
        for id in grabs {
            self.grab_splines.remove(&id);
            grabbables.remove(id);
        }
        self.model = None;
        self.rain_data = None;
        self.graph = None;
        // Factory FreeBlock is last. The caller drops this rejected value;
        // it must never return it to the active roster or retry destruction.
        factory
            .free_entity_slot(&self.allocation)
            .map_err(E::Factory)?;
        Ok(())
    }
}

impl crate::living_world_grind_services::LivingWorldGrindServices {
    /// RequestSpawn826f5890 -> AttachToManagers82d26ef0 -> OnSpawnComplete.
    /// Vehicle LWBasicEntity vtable82326bc8+50 resolves to shared blr825187c0.
    /// Does not run population selection or the per-frame execution pipeline.
    pub(crate) fn request_spawn_vehicle<N: ReckoningNumeric>(
        &mut self,
        info: Option<&VehicleSpawnInfo>,
        vault: &Arc<AttributeRuntimeVault>,
        recipes: &MountedRecipeRegistry,
        world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
        graphs: &crate::actor_construction::graph_startup::ActorGraphStartupOwners,
        override_model_name: bool,
        numeric: &mut N,
    ) -> Result<Option<u32>, VehicleSpawnError<N::Error>> {
        use VehicleSpawnError as E;
        let Some(vehicle) = self.spawn_vehicle(
            info,
            vault,
            recipes,
            world,
            cvars,
            graphs,
            override_model_name,
            numeric,
        )?
        else {
            return Ok(None);
        };
        let id = vehicle.id;
        let losim = vehicle.core.current_losim;
        let mut unpublished = Some(vehicle);
        // The manager was checked by Spawn; keep the real object on any
        // checked prefix error, and leave successful map publication in place.
        let entities = self
            .grind_vehicle_entities
            .as_deref_mut()
            .expect("Spawn checked EntityMan");
        if let Err(error) = entities.add_retained(id, &mut unpublished) {
            self.vehicle_spawn_unfinished = unpublished;
            return Err(E::Entity(error));
        }
        if unpublished.is_some() {
            // Wrapped duplicate IDs cannot be represented as two live owners
            // by this typed map. Preserve the actual AddEntity prefix and fail.
            self.vehicle_spawn_unfinished = unpublished;
            return Err(E::DuplicateVehicleId(id));
        }
        self.vehicle_execution
            .add(losim, id)
            .map_err(E::Execution)?;
        // No-op OnSpawnComplete only AFTER both manager admissions.
        Ok(Some(id))
    }
    /// The actual retained-owner bridge, used by the category census caller.
    /// A successful result still requires AttachToManagers; never publish a
    /// pending/failed spawn. This method neither chooses population targets nor
    /// creates a substitute world, recipe registry or graph-delegate table.
    pub(crate) fn spawn_vehicle<N: ReckoningNumeric>(
        &mut self,
        info: Option<&VehicleSpawnInfo>,
        vault: &Arc<AttributeRuntimeVault>,
        recipes: &MountedRecipeRegistry,
        world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
        graphs: &crate::actor_construction::graph_startup::ActorGraphStartupOwners,
        override_model_name: bool,
        numeric: &mut N,
    ) -> Result<Option<Vehicle>, VehicleSpawnError<N::Error>> {
        use VehicleSpawnError as E;
        if info.is_none() {
            return Ok(None);
        }
        let mesh = self.try_mesh_store().map_err(E::Mesh)?;
        let enablement = graphs.living_world_enablement().map_err(E::Graphs)?;
        VehicleSpawnFrame {
            factory: self
                .vehicle_factory
                .as_mut()
                .ok_or(E::MissingOwner("vehicle factory"))?,
            caches: &mut self.census_targets,
            ids: &mut self.entity_ids,
            vault,
            models: &self.dmo_models,
            recipes,
            assemblies: &mut self.vehicle_collideables,
            grabbables: &mut self.vehicle_grabbables,
            spline_ids: &mut self.spline_ids,
            world,
            rain: self.rain.as_mut().ok_or(E::MissingOwner("RAIN"))?,
            mesh: &mesh,
            vehicles: self
                .grind_vehicle_entities
                .as_deref_mut()
                .ok_or(E::MissingOwner("vehicle EntityMan"))?,
            pedestrians: self
                .pedestrian_entities
                .as_deref_mut()
                .ok_or(E::MissingOwner("pedestrian EntityMan"))?,
            dmos: self
                .dmo_objects
                .as_deref()
                .ok_or(E::MissingOwner("DMO manager"))?,
            random: self
                .random
                .as_mut()
                .ok_or(E::MissingOwner("Manager random"))?,
            cvars,
            enablement,
            override_model_name,
            unfinished: &mut self.vehicle_spawn_unfinished,
        }
        .spawn(info, numeric)
    }
}
