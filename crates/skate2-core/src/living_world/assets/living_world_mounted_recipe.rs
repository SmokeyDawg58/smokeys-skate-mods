//! Mounted LivingWorld/Vehicle load through the SAME CharacterAssetManager
//! registry as the local skater. Source loader82814900, UpdateRequest82d14ac0,
//! OnComplete82d0a140. Decoding may run on a host worker; registry admission
//! stays with the owner. Native worker timing is not claimed.
use super::{MountedRecipeError, MountedRecipeRegistry, RegisteredAsset};
use crate::{
    character::CharacterScene, input_cvar_runtime::InputCvarReferences,
    player::mounted_recipe::finalize::FinalizedRecipeScene, world::CollisionArena,
};
use skate2_formats::{hash::rw_hash64_string, mount::MountSet, recipe::CharacterRecipe};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) enum AmbientBegin {
    Cached(u64),
    Load(AmbientLoadRequest),
}

/// One missing ambient recipe. Decoding reads only the mounted archives and
/// the recipe itself, so it may run away from the registry owner.
pub(crate) struct AmbientLoadRequest {
    pub(crate) id: u64,
    category: u32,
    namespace: &'static str,
    name: String,
}

pub(crate) struct PreparedAmbientRecipe {
    id: u64,
    recipe: Arc<CharacterRecipe>,
    scene: Arc<CharacterScene>,
    assets: Arc<AmbientRecipeAssets>,
}

impl AmbientLoadRequest {
    pub(crate) fn prepare(self, mounts: &MountSet) -> Result<PreparedAmbientRecipe, MountedRecipeError> {
        use MountedRecipeError as E;
        let Self { id, category, namespace, name } = self;
        // GetRecipeFileName82cfdfc0 uses livingworld.big for BOTH namespaces.
        // MountSet resolves the original entry path inside that mounted archive.
        let path = format!("data\\content/recipe/{namespace}/{name}.recipe");
        let bytes = mounts.read_path(&path).map_err(E::Mount)?;
        let mut recipe = CharacterRecipe::parse(&bytes).map_err(E::Recipe)?;
        // Constructor NoClone remains defined during Unpack; requested category
        // overwrites the serialized field before registry admission82d15108.
        recipe.clone_type = 0;
        recipe.category = category;
        let recipe = Arc::new(recipe);
        let mut collision = BTreeMap::new();
        let mut embedded_splines = BTreeMap::new();
        let scene = crate::mounted_character_scene::own_character_scene_with_geometry(
            mounts,
            &recipe,
            namespace,
            &mut |request, arena, resources| {
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    collision.entry(request.loaded_asset_id)
                {
                    let projected = crate::simulation_collision_asset::project_collision_arena(
                        arena, resources,
                    )?;
                    let projected = Arc::new(projected);
                    let splines = crate::living_world_grind_services::embedded_splines::decode_character_arena(
                        arena, resources, &projected, request.loaded_asset_id,
                    )?;
                    embedded_splines.insert(request.loaded_asset_id, splines);
                    entry.insert(projected);
                }
                Ok(())
            },
        )
        .map_err(E::Scene)?;
        let scene = Arc::new(scene);
        let mut finalized = FinalizedRecipeScene::new(Arc::clone(&scene));
        finalized.try_finalize(0).map_err(E::Finalize)?;
        let assets = Arc::new(AmbientRecipeAssets {
            finalized: Arc::new(finalized),
            collision,
            embedded_splines,
        });
        Ok(PreparedAmbientRecipe { id, recipe, scene, assets })
    }
}

pub(crate) struct AmbientRecipeAssets {
    pub(crate) finalized: Arc<FinalizedRecipeScene>,
    /// Loaded arena identities used by the recipe, not the serialized source IDs.
    pub(crate) collision: BTreeMap<u64, Arc<CollisionArena>>,
    /// Same loaded arena ID, then original tInstance GUID. No spline IDs are
    /// allocated until the entity's InitSplines; model loading is read-only.
    pub(crate) embedded_splines: BTreeMap<
        u64,
        BTreeMap<u64, crate::living_world_grind_services::embedded_splines::ModelSplines>,
    >,
}
impl MountedRecipeRegistry {
    pub(crate) fn ambient_assets(
        &self,
        id: u64,
    ) -> Result<&Arc<AmbientRecipeAssets>, MountedRecipeError> {
        match self.assets.get(&id) {
            Some(RegisteredAsset::Recipe(entry)) => entry
                .ambient_assets
                .as_ref()
                .ok_or(MountedRecipeError::Missing("finalized ambient assets")),
            Some(RegisteredAsset::Unsupported(asset_type)) => {
                Err(MountedRecipeError::RegistryAssetType {
                    id,
                    asset_type: *asset_type,
                })
            }
            None => Err(MountedRecipeError::RegistryIdentity(id)),
        }
    }

    pub(crate) fn load_ambient(
        &mut self,
        category: u32,
        name: &str,
        mounts: &MountSet,
        cvars: &InputCvarReferences,
    ) -> Result<u64, MountedRecipeError> {
        match self.begin_ambient(category, name, cvars)? {
            AmbientBegin::Cached(id) => Ok(id),
            AmbientBegin::Load(request) => {
                let prepared = request.prepare(mounts)?;
                self.finish_ambient(prepared)
            }
        }
    }

    /// Whether a skater's load (the player's, a party or AI skater's) holds the
    /// schema's identity. One entry stands for a schema here, so an ambient load of
    /// the same schema cannot share it as the native asset manager's would.
    pub(crate) fn holds_skater_recipe(&self, id: u64) -> bool {
        matches!(self.assets.get(&id), Some(RegisteredAsset::Recipe(entry)) if entry.ambient_assets.is_none())
    }

    /// Registry half of LoadImpl: a resident recipe gains a reference; a miss
    /// returns the decode request, which touches no registry state.
    pub(crate) fn begin_ambient(
        &mut self,
        category: u32,
        name: &str,
        cvars: &InputCvarReferences,
    ) -> Result<AmbientBegin, MountedRecipeError> {
        use MountedRecipeError as E;
        // Sk8LWCharacterLoader::LoadImpl admits precisely LivingWorld and Vehicle;
        // PresCharacterManager::ReceiveSimCharacterLoadRequest8281b6f0 gives a
        // Marquee request to a MarqueeLoader, whose LoadImpl82d09ef0 runs the
        // same LoadSchemaFromFile in gCategoryMapping[0], NoClone for a request
        // that is not the local player's (82814a48..4a58, 82d0b130..b158).
        let namespace = match category {
            0 => "marquee",
            1 => "livingworld",
            2 => "vehicle",
            _ => return Err(E::AmbientCategory(category)),
        };
        let id = rw_hash64_string(&name.to_ascii_lowercase());
        if let Some(entry) = self.assets.get_mut(&id) {
            return match entry {
                RegisteredAsset::Recipe(entry) => {
                    // A present-but-loading recipe cannot be declared ready.
                    entry
                        .ambient_assets
                        .as_ref()
                        .ok_or(E::Missing("cached ambient completion"))?;
                    entry.references = entry.references.wrapping_add(1);
                    Ok(AmbientBegin::Cached(id))
                }
                RegisteredAsset::Unsupported(asset_type) => Err(E::RegistryAssetType {
                    id,
                    asset_type: *asset_type,
                }),
            };
        }
        if !cvars.try_schema_load_from_binary().map_err(E::Cvar)? {
            return Err(E::XmlLoadPending);
        }
        Ok(AmbientBegin::Load(AmbientLoadRequest {
            id,
            category,
            namespace,
            name: name.to_owned(),
        }))
    }

    /// Admits a decoded recipe: parsed publication, scene retention, then the
    /// finalized ambient assets, as one registry update.
    pub(crate) fn finish_ambient(
        &mut self,
        prepared: PreparedAmbientRecipe,
    ) -> Result<u64, MountedRecipeError> {
        let PreparedAmbientRecipe { id, recipe, scene, assets } = prepared;
        self.publish_parsed(id, Arc::clone(&recipe))?;
        self.retain_scene(id, &recipe, scene)?;
        let Some(RegisteredAsset::Recipe(entry)) = self.assets.get_mut(&id) else {
            return Err(MountedRecipeError::RegistryIdentity(id));
        };
        entry.ambient_assets = Some(assets);
        // OnComplete82d0a220..230 only calls FaceMan for category0 or3.
        // Category1/2 proceeds directly to LoadCompleteAction.
        Ok(id)
    }

    /// Native UnloadRequested82d15554 decrements the recipe reference before
    /// notifying observers. The caller must deliver NotifyOfUnload and complete
    /// its wait before calling finish_ambient_release on the zero-count route.
    pub(crate) fn begin_ambient_release(&mut self, id: u64) -> Result<bool, MountedRecipeError> {
        self.ambient_assets(id)?;
        let Some(RegisteredAsset::Recipe(entry)) = self.assets.get_mut(&id) else {
            return Err(MountedRecipeError::RegistryIdentity(id));
        };
        let previous = entry.references;
        entry.references = previous.wrapping_sub(1);
        Ok(previous == 1)
    }
    /// Unload82d155d8 removes registry membership before TrackUnloadedRecipe.
    /// Returning the ownership hold lets the asset manager finish arena release
    /// before delivering its unload callback; this is NOT that callback itself.
    pub(crate) fn finish_ambient_release(
        &mut self,
        id: u64,
    ) -> Result<Arc<AmbientRecipeAssets>, MountedRecipeError> {
        self.ambient_assets(id)?;
        let Some(RegisteredAsset::Recipe(entry)) = self.assets.get(&id) else {
            return Err(MountedRecipeError::RegistryIdentity(id));
        };
        if entry.references != 0 {
            return Err(MountedRecipeError::ReferencedRecipe(id));
        }
        let Some(RegisteredAsset::Recipe(entry)) = self.assets.remove(&id) else {
            unreachable!()
        };
        self.tracked
            .retain(|recipe| !Arc::ptr_eq(recipe, &entry.recipe));
        Ok(entry.ambient_assets.expect("validated ambient assets"))
    }
}
