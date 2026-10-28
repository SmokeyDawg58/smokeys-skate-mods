//! Source Vehicle.InitModels82d1d5f0 through InitSplines82d17340.
//! Uses the retained Vault, ModelMan, global rw::math RNG, actual Part backend,
//! and VehicleMan's CollideableObjectMan. No entity publication occurs here.
use super::{
    Vehicle,
    collision::{VehicleCollision, VehicleCollisionError},
    factory::{VehicleFactory, VehicleFactoryError},
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{
        character_models::{CharacterModelError, CharacterModelKind, first_collision_instance},
        collideable_objects::{AssemblyOwnerError, CollideableObjectMan},
        dmo_models::DmoModelMan,
        dmo_objects::ENTITIES_CLASS,
        spline_registration::{GrabbableObjectMan, LivingWorldSplineIds},
        vehicle_graph::VehicleGraph,
    },
    living_world_recipe_manager::LIVING_WORLD_MODELS_CLASS_ID,
    native_skater_random::rw_math::RwRandomLockError,
    physical_player_assembly::PhysicalAssemblyWorld,
    player::mounted_recipe::{MountedRecipeError, MountedRecipeRegistry},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum VehicleModelInitError {
    Factory(VehicleFactoryError),
    AlreadyInitialized,
    Attribute(AttributeRuntimeError),
    Random(RwRandomLockError),
    EmptyColourArray(u64),
    Collision(VehicleCollisionError),
    AssemblyOwner(AssemblyOwnerError),
    WrongModelCategory(u64),
    MissingGraphName,
    UnsupportedGraph(Vec<u8>),
    DynamicGraphPending,
    Recipe(MountedRecipeError),
    Model(CharacterModelError),
    MissingSplineArena(u64),
}

impl Vehicle {
    /// Collideable.DeInit82e5e9c0 -> Vehicle.DestroyLivingWorldCollision82d1d310.
    /// Vehicle destructor calls this AFTER its RAIN departure; other entity
    /// destruction/manager notification is deliberately the caller's work.
    pub(crate) fn deinitialize_collision(
        &mut self,
        factory: &mut VehicleFactory,
        assemblies: &mut CollideableObjectMan,
        world: &mut PhysicalAssemblyWorld,
    ) -> Result<(), VehicleModelInitError> {
        if let Some(collision) = &mut self.collision {
            if let Some(assembly) = collision.assembly_handle() {
                assemblies.remove(assembly);
            }
            collision
                .destroy_bodies(world)
                .map_err(VehicleModelInitError::Collision)?;
        }
        factory
            .free_collision(&mut self.allocation)
            .map_err(VehicleModelInitError::Factory)?;
        self.collision = None;
        Ok(())
    }
    /// Caller MUST follow with RAIN/Island/manager admission. `generated_enabled` is the
    /// delegate setting established by Manager.PreLoadStateGraphs, not an
    /// invented per-vehicle behavior toggle. Failures preserve the native prefix.
    pub(crate) fn initialize_models(
        &mut self,
        factory: &mut VehicleFactory,
        recipe_id: u64,
        vault: &Arc<AttributeRuntimeVault>,
        models: &DmoModelMan,
        recipes: &MountedRecipeRegistry,
        assemblies: &mut CollideableObjectMan,
        grabbables: &mut GrabbableObjectMan,
        spline_ids: &mut LivingWorldSplineIds,
        world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
        generated_enabled: bool,
    ) -> Result<(), VehicleModelInitError> {
        use VehicleModelInitError as E;
        if self.collision.is_some() {
            return Err(E::AlreadyInitialized);
        }
        let entity = vault.instance(ENTITIES_CLASS, self.core.entity_key);
        let model = vault.instance(LIVING_WORLD_MODELS_CLASS_ID, self.core.model_key);
        self.core.recipe_id = recipe_id;
        // Full64 keys checked at insrdi82d1d648/690. Read/draw/store each
        // separately: neither missing model nor a one-entry array skips Random.
        self.core.initialize_model_colours(&model).map_err(|error| {
            use crate::living_world_grind_services::entity_core::ModelColourError as C;
            match error {
                C::Attribute(e) => E::Attribute(e),
                C::Random(e) => E::Random(e),
                C::EmptyArray(key) => E::EmptyColourArray(key),
            }
        })?;
        let model = models.character_model(recipe_id).cloned();
        factory
            .allocate_collision(&mut self.allocation)
            .map_err(E::Factory)?;
        self.collision = Some(VehicleCollision::new(vault));
        let collision = self
            .collision
            .as_mut()
            .expect("created collision controller");
        collision
            .initialize_bodies(model.clone(), self.id, world, cvars)
            .map_err(E::Collision)?;
        // Native vehicle has only the skater body. Collideable.Init registers
        // its actual Assembly even before the vehicle enters the active roster.
        if let Some(assembly) = collision.assembly_handle() {
            assemblies
                .register(assembly, self.id)
                .map_err(E::AssemblyOwner)?;
        }
        self.model = model;
        self.wheels.radius = match self.model.as_ref().map(|model| &model.kind) {
            Some(CharacterModelKind::Vehicle { wheel_radius }) => *wheel_radius,
            Some(_) => return Err(E::WrongModelCategory(recipe_id)),
            None => 1.,
        };
        let name = entity
            .try_get_text(0xca4d_0ad4_ea45_965b, 0)
            .map_err(E::Attribute)?;
        // LoadAIGraph82e4c340 destroys the previous controller first.
        self.graph = None;
        let name = name.ok_or(E::MissingGraphName)?;
        let path = name.bytes();
        // Create82951424 chooses the last '/' first, then '\\' only if no '/'
        // was found. Filename comparison is case-sensitive.
        let begin = path
            .iter()
            .rposition(|v| *v == b'/')
            .or_else(|| path.iter().rposition(|v| *v == b'\\'))
            .map_or(0, |i| i + 1);
        if &path[begin..] != b"Vehicle.xml" {
            return Err(E::UnsupportedGraph(path.to_vec()));
        }
        if !generated_enabled {
            return Err(E::DynamicGraphPending);
        }
        // HierarchicalController.Load82cef1e0 passes call_behaviours=true.
        self.graph = Some(VehicleGraph::new(true));
        // InitSplines is last, and has a model-null early return independent
        // of the graph load. Select exactly the model-creation collision
        // instance, not every rendered LOD's embedded data.
        if self.model.is_some() {
            let assets = recipes.ambient_assets(recipe_id).map_err(E::Recipe)?;
            if let Some((lod, instance, _, _)) =
                first_collision_instance(&assets.finalized, &assets.collision).map_err(E::Model)?
            {
                let arena = assets
                    .embedded_splines
                    .get(&lod.loaded_asset_id)
                    .ok_or(E::MissingSplineArena(lod.loaded_asset_id))?;
                if let Some((grinds, grabs)) = arena.get(&instance.guid) {
                    self.grab_splines = grabbables.register(self.id, grabs, spline_ids);
                    self.grind_splines = spline_ids.register_grinds(grinds);
                }
            }
        }
        Ok(())
    }
}
