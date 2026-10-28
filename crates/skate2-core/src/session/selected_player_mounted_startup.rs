//! Selected mounted startup composition over supplied world and platform owners.
use super::{
    SelectedPlayerConstructionRequest, SelectedPlayerStartup, SelectedPlayerStartupError,
    SelectedPlayerStartupOwners,
};
use crate::{
    actor_construction::player_animation::PlayerAnimationError,
    actor_face_playback::{FaceAnimationManagers, presentation::startup::PresentationStartupError},
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    character_face_manager::CharacterFaceManager,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    player::{
        CharacterRegistrationRequest, MarqueeDefinition, NO_PEER_ID, PlayerLoadAction,
        PlayerLoaderSlot,
        attributes_runtime::PlayerAttributeServices,
        mounted_recipe::{MountedPlayerRecipeLoad, MountedRecipeError, MountedRecipeRegistry},
    },
    players_backend_settings::{
        PlayersBackendSettings,
        create::{PlayerCreateError, ProfileNameOwner},
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_animation_startup::{
        AnimationStartupError, AnimationStartupSource, SimulationAnimationStartup,
    },
    skater_manager_local_identity::{LocalIdentityError, SkaterManagerLocalIdentity},
};
use skate2_formats::mount::MountSet;
use std::sync::{Arc, Mutex};

/// Already selected host owners. No archive reopening, world construction,
/// platform thread answer or input readiness is supplied by this composition.
pub(crate) struct MountedSelectedOwners {
    pub(crate) graphs: Arc<Mutex<crate::actor_construction::graph_startup::ActorGraphStartupOwners>>,
    pub(crate) living_world_recipes: Arc<Mutex<crate::selected_living_world_recipes::SelectedLivingWorldRecipes>>,
    pub(crate) mounts: Arc<MountSet>,
    pub(crate) vault: Arc<AttributeRuntimeVault>,
    pub(crate) cvars: InputCvarReferences,
    pub(crate) skaters: Arc<SkaterManagerLocalIdentity>,
    pub(crate) animation_managers: FaceAnimationManagers,
    pub(crate) juice: crate::juice_trick_input::JuiceTrickInput,
    pub(crate) pause: Arc<crate::selected_pause_manager::SelectedPauseManager>,
}

#[derive(Debug)]
pub(crate) enum MountedSelectedError<S, N> {
    AlreadyAttempted,
    ForeignWorld,
    LocalIndex { selected: u32, actual: i32 },
    WorldLock,
    Identity(LocalIdentityError),
    Create(PlayerCreateError),
    Profile(crate::edit_skater_profile::EditSkaterProfileError),
    Attribute(AttributeRuntimeError),
    Utf8(std::str::Utf8Error),
    Missing(&'static str),
    UnexpectedLoadAction(PlayerLoadAction),
    Recipe(MountedRecipeError),
    LivingWorldRecipes(crate::selected_living_world_recipes::SelectedLivingWorldRecipeError),
    Animation(AnimationStartupError<S>),
    Selected(Box<SelectedPlayerStartupError<S, N>>),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for MountedSelectedError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => f.write_str("selected mounted startup already attempted"),
            Self::ForeignWorld => f.write_str("selected skater owner differs from world"),
            Self::LocalIndex { selected, actual } => write!(
                f,
                "selected player {selected} differs from local index {actual}"
            ),
            Self::WorldLock => f.write_str("selected mounted world lock poisoned"),
            Self::Identity(e) => write!(f, "selected local identity: {e:?}"),
            Self::Create(e) => write!(f, "selected offline player: {e}"),
            Self::Profile(e) => write!(f, "selected profile: {e}"),
            Self::Attribute(e) => write!(f, "selected marquee attribute: {e:?}"),
            Self::Utf8(e) => write!(f, "selected marquee schema: {e}"),
            Self::Missing(name) => write!(f, "missing selected startup {name}"),
            Self::UnexpectedLoadAction(action) => {
                write!(f, "selected recipe continuation: {action:?}")
            }
            Self::Recipe(e) => write!(f, "selected mounted recipe: {e}"),
            Self::LivingWorldRecipes(e) => e.fmt(f),
            Self::Animation(e) => write!(f, "selected global animation: {e:?}"),
            Self::Selected(e) => write!(f, "selected Actor startup: {e}"),
        }
    }
}

impl SelectedPlayerStartup {
    /// Construct retained empty native fields; the explicit index is checked
    /// against the supplied local manager before any player loading occurs.
    pub(crate) fn new_mounted(player_index: u32, supplied: MountedSelectedOwners) -> Self {
        Self::new(SelectedPlayerStartupOwners {
            mounts: supplied.mounts,
            focus: Arc::new(Mutex::new(
                crate::selected_sim_command_queue::focus::SelectedPlayerFocus::construct_fields(
                    player_index,
                ),
            )),
            juice: supplied.juice,
            pause: supplied.pause,
            recipe_registry: Some(MountedRecipeRegistry::construct_fields()),
            recipe_load: Some(MountedPlayerRecipeLoad::new()),
            living_world_recipes:supplied.living_world_recipes,
            presentation: None,
            pedestrian_animation: Default::default(),
            loader: PlayerLoaderSlot::new(player_index),
            services: PlayerAttributeServices {
                profile: ProfileNameOwner::construct_name(),
                vault: supplied.vault,
                settings: PlayersBackendSettings::construct_fields(),
                skaters: supplied.skaters,
            },
            animation: SimulationAnimationStartup::new(supplied.animation_managers),
            faces: CharacterFaceManager::new(),
            cvars: supplied.cvars,
            graphs: Some(supplied.graphs),
            loaded_graphs: None,
            formation: None,
            map: None,
        })
    }

    /// Controlled corpus path: prepare the Register request, then construct
    /// Actor. Native `SpawnPlayer` scheduling relative to that request is still
    /// under review; the ordinary registration status needs a later
    /// Actor-backed presentation entity with a matching peer.
    pub(crate) fn try_initialize_mounted<S: AnimationStartupSource, N: ReckoningNumeric>(
        &mut self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        marquee: u64,
        request: SelectedPlayerConstructionRequest,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), MountedSelectedError<S::Error, N::Error>> {
        self.try_prepare_mounted_registration::<S, N>(world, marquee, source)?;
        let mounts = Arc::clone(&self.owners.mounts);
        self.try_initialize(world, &mounts, request, source, numeric)
            .map_err(|e| MountedSelectedError::Selected(Box::new(e)))
    }

    /// Retains the selected recipe, loader request, and initialized global
    /// animation, stopping before Actor construction. `Register` is a request,
    /// not proof that the presentation manager completed registration.
    pub(crate) fn try_prepare_mounted_registration<
        S: AnimationStartupSource,
        N: ReckoningNumeric,
    >(
        &mut self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        marquee: u64,
        source: &mut S,
    ) -> Result<CharacterRegistrationRequest, MountedSelectedError<S::Error, N::Error>> {
        use MountedSelectedError as E;
        if self.mounted_attempted || self.attempted {
            return Err(E::AlreadyAttempted);
        }
        self.mounted_attempted = true;
        // SecondaryLoad03 initializes RecipeMan before character preparation.
        // Init builds mappings/queues only; model IO runs at RecipeMan.Update.
        // The app's SAME RecipeMan was initialized before terrain admission.
        // Player preparation only retains it; rebuilding its queues loses
        // waypoint model selection and live loading references.
        let skaters = &self.owners.services.skaters;
        if !world
            .lock()
            .map_err(|_| E::WorldLock)?
            .local_skater_identity
            .as_ref()
            .is_some_and(|actual| Arc::ptr_eq(actual, skaters))
        {
            return Err(E::ForeignWorld);
        }
        let selected = self.owners.loader.player_index();
        let path=crate::edit_skater_profile::EditSkaterProfileStore::default_path()
            .map_err(|e|E::Profile(e.into()))?;
        if let Some(data)=crate::edit_skater_profile::EditSkaterProfileStore::open(path)
            .and_then(|s|s.load_profile_data()).map_err(E::Profile)? {
            self.owners.services.profile.set_saved_name(data.name).map_err(E::Create)?;
        }
        let actual = skaters.try_local_index().map_err(E::Identity)?;
        if u32::try_from(actual).ok() != Some(selected) {
            return Err(E::LocalIndex { selected, actual });
        }
        self.owners
            .services
            .settings
            .try_create_offline_human(selected, &self.owners.services.profile, skaters.backend())
            .map_err(E::Create)?;
        self.owners.loader.set_marquee(marquee, NO_PEER_ID);
        let instance = self
            .owners
            .services
            .vault
            .instance(0x91bc_a669_3efc_9aa7, marquee);
        let name = instance
            .try_cached_layout_text(0)
            .map_err(E::Attribute)?
            .ok_or(E::Missing("marquee schema name"))?;
        let definition = MarqueeDefinition {
            id: marquee,
            schema_name: std::str::from_utf8(name.bytes())
                .map_err(E::Utf8)?
                .to_owned(),
        };
        let mounts = Arc::clone(&self.owners.mounts);
        // Memcard CAC record precedes PlayerLoaderImpl::Load. A first-use
        // profile keeps the original default-marquee branch.
        let save_path=crate::edit_skater_profile::EditSkaterProfileStore::default_path()
            .map_err(|e|E::Recipe(MountedRecipeError::Scene(Box::new(e))))?;
        let profile=crate::edit_skater_profile::EditSkaterProfileStore::open(save_path)
            .map_err(|e|E::Recipe(MountedRecipeError::Scene(Box::new(e))))?;
        let record=profile.load_record().map_err(|e|E::Recipe(MountedRecipeError::Scene(Box::new(e))))?
            .filter(|r|!r.data.recipe.is_empty());
        let action = self
            .owners
            .recipe_load
            .as_mut()
            .ok_or(E::Missing("recipe load"))?
            .try_load_selected(
                &mut self.owners.loader,
                &definition,
                &self.owners.services,
                &mut self.owners.faces,
                &mounts,
                &self.owners.cvars,
                self.owners
                    .recipe_registry
                    .as_mut()
                    .ok_or(E::Missing("recipe registry"))?,
                record.as_ref(),
            )
            .map_err(E::Recipe)?;
        let registration = match action {
            PlayerLoadAction::Register(registration) => registration,
            action => return Err(E::UnexpectedLoadAction(action)),
        };
        // GameModule's loading barrier precedes gameplay. Previously these
        // required models only loaded in SIM ticks; an early service request
        // could start Big Black's timer while his census spawn returned null.
        {
            let mut recipes = self.owners.living_world_recipes.lock().map_err(|_| E::WorldLock)?;
            let mut world = world.lock().map_err(|_| E::WorldLock)?;
            recipes
                .finish_initial_load(
                    &mut world,
                    &self.owners.cvars,
                    &self.owners.services.vault,
                    self.owners.recipe_registry.as_mut().ok_or(E::Missing("recipe registry"))?,
                    self.owners.recipe_load.as_mut().ok_or(E::Missing("recipe load"))?.requests_mut(),
                    &mounts,
                )
                .map_err(E::LivingWorldRecipes)?;
            recipes.crowd.attach_pause(Arc::clone(&self.owners.pause));
        }
        self.owners
            .animation
            .try_initialize(source)
            .map_err(E::Animation)?;
        Ok(registration)
    }

    /// Separate continuation permits the real caller's intervening registration
    /// work. This does not load graphs or manufacture a registration callback.
    pub(crate) fn try_initialize_mounted_presentation<S: AnimationStartupSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), PresentationStartupError<S::Error, PlayerAnimationError>> {
        if self.physical_animation.is_none() {
            return Err(PresentationStartupError::Recipe(
                MountedRecipeError::Missing("constructed physical animation"),
            ));
        }
        let recipe = Arc::clone(
            self.owners
                .recipe_load
                .as_ref()
                .ok_or(PresentationStartupError::Recipe(
                    MountedRecipeError::Missing("recipe load"),
                ))?
                .recipe()
                .map_err(PresentationStartupError::Recipe)?,
        );
        self.owners
            .try_initialize_selected_presentation(recipe, source)
    }
}

impl SelectedPlayerStartupOwners {
    /// Caller supplies the actual Actor/animation pair; index equality alone
    /// is not an occurrence-identity proof. Snapshots of already-produced animation
    /// owners precede the actual ff2c0 hat getter. No fixture asset is copied in.
    pub(crate) fn try_physical_inputs(
        &self,
        actor: &crate::actor_construction::ActorConstruction,
        animation: &crate::actor_construction::physical_animation_inputs::ActorPhysicalAnimationInputs,
        spawn: [f32; 16],
    ) -> Result<
        crate::physical_player_assembly::PhysicalPlayerAssemblyInputs,
        crate::actor_construction::physical_animation_inputs::PhysicalAnimationInputsError,
    > {
        use crate::actor_construction::physical_animation_inputs::PhysicalAnimationInputsError as E;
        let player = actor
            .prefix()
            .map_err(E::Actor)?
            .fields()
            .ok_or(E::Missing("Actor constructor fields"))?
            .player_index;
        if player != self.loader.player_index() {
            return Err(E::Missing("same selected Actor player index"));
        }
        animation.try_assembly_inputs(
            self.cvars.clone(),
            Arc::clone(&self.services.vault),
            spawn,
            || self.services.settings.try_hat(player).map_err(E::Settings),
        )
    }
}
