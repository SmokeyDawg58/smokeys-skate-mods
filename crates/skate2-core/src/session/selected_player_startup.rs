//! Retained selected-player construction using existing native constructor stages.
//! Host composition, not a replacement for native whole-manager scheduling.
use super::{ActorChaseFormationOwner, ActorConstruction, ActorConstructionError, ActorIdentity};
use crate::{
    actor_construction::{
        graph_startup::{ActorGraphStartupError, ActorGraphStartupOwners},
        input_generation::runtime::ActorInputRuntimeServices,
        physical_animation_inputs::ActorPhysicalAnimationInputs,
        player_animation::{ActorAnimationPlayerServices, PlayerAnimationError},
        player_startup::ActorPlayerStartupError,
    },
    character_face_manager::CharacterFaceManager,
    game_input_frame_session::GameInputFrameSession,
    game_input_manager::GameInputCommandManager,
    gameplay_input_update_manager::{GameplayInputLifecycleError, GameplayInputUpdateManager},
    generated_action_load::GeneratedActionLoadError,
    input_cvar_runtime::InputCvarReferences,
    input_map::{InputMapParseError, SelectedInputMap},
    physical_player_assembly::PhysicalAssemblyWorld,
    player::{PlayerLoaderSlot, attributes_runtime::PlayerAttributeServices},
    reckoning_numeric::ReckoningNumeric,
    simulation_animation_startup::{AnimationStartupSource, SimulationAnimationStartup},
    skeleton_root_transforms::SkeletonRootMatrix,
};
use skate2_formats::mount::{MountError, MountSet};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

/// These are the actual owners supplied after asset completion and attribute
/// application. The loader remains in its real phase; construction does not
/// synthesize a presentation-registration callback.
pub(crate) struct SelectedPlayerStartupOwners {
    pub(crate) mounts: Arc<MountSet>,
    pub(crate) focus: Arc<Mutex<crate::selected_sim_command_queue::focus::SelectedPlayerFocus>>,
    pub(crate) juice: crate::juice_trick_input::JuiceTrickInput,
    pub(crate) pause: Arc<crate::selected_pause_manager::SelectedPauseManager>,
    pub(crate) recipe_registry: Option<crate::player::mounted_recipe::MountedRecipeRegistry>,
    pub(crate) recipe_load: Option<crate::player::mounted_recipe::MountedPlayerRecipeLoad>,
    pub(crate) living_world_recipes:
        Arc<Mutex<crate::selected_living_world_recipes::SelectedLivingWorldRecipes>>,
    pub(crate) presentation:
        Option<crate::actor_face_playback::presentation::startup::SelectedPresentationStartup>,
    pub(crate) loader: PlayerLoaderSlot,
    pub(crate) services: PlayerAttributeServices,
    pub(crate) animation: SimulationAnimationStartup,
    pub(crate) pedestrian_animation: crate::selected_living_world_pedestrian_visual::PedestrianPlaybackStartup,
    pub(crate) faces: CharacterFaceManager,
    pub(crate) cvars: InputCvarReferences,
    pub(crate) graphs: Option<Arc<Mutex<ActorGraphStartupOwners>>>,
    pub(crate) loaded_graphs: Option<[bool; 2]>,
    pub(crate) formation: Option<Arc<Mutex<ActorChaseFormationOwner>>>,
    pub(crate) map: Option<Arc<SelectedInputMap>>,
}
pub(crate) struct SelectedPlayerStartup {
    pub(crate) owners: SelectedPlayerStartupOwners,
    pub(crate) input: GameInputFrameSession<ActorIdentity>,
    pub(crate) actor: Option<Arc<ActorConstruction>>,
    pub(crate) physical_animation: Option<ActorPhysicalAnimationInputs>,
    mounted_attempted: bool,
    attempted: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct SelectedPlayerConstructionRequest {
    pub(crate) spawn: SkeletonRootMatrix,
    pub(crate) actor_flags: u32,
}
#[derive(Debug)]
pub(crate) enum SelectedPlayerStartupError<S, N> {
    AlreadyAttempted,
    Mount(MountError),
    Utf8(std::str::Utf8Error),
    Map(InputMapParseError),
    InputConstruct(GameplayInputLifecycleError<MountError>),
    InputReset(GameplayInputLifecycleError<Infallible>),
    MissingPlayer(u32),
    Actor(ActorConstructionError),
    Prefix(ActorConstructionError<N>),
    AnimationBinding(PlayerAnimationError),
    Animation(ActorPlayerStartupError<S>),
    GraphOwners(GeneratedActionLoadError),
    Graphs(ActorGraphStartupError),
    WorldLock,
    FormationLock,
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display
    for SelectedPlayerStartupError<S, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => f.write_str("selected player construction already attempted"),
            Self::Mount(e) => write!(f, "selected input mount: {e}"),
            Self::Utf8(e) => write!(f, "selected input text: {e}"),
            Self::Map(e) => write!(f, "selected input map: {e:?}"),
            Self::InputConstruct(e) => write!(f, "selected input construction: {e}"),
            Self::InputReset(e) => write!(f, "selected input reset: {e}"),
            Self::MissingPlayer(index) => write!(f, "selected input player {index} absent"),
            Self::Actor(e) => write!(f, "selected Actor construction: {e:?}"),
            Self::Prefix(e) => write!(f, "selected Actor prefix: {e:?}"),
            Self::AnimationBinding(e) => write!(f, "selected animation binding: {e}"),
            Self::Animation(e) => write!(f, "selected animation construction: {e}"),
            Self::GraphOwners(e) => write!(f, "selected graph owners: {e:?}"),
            Self::Graphs(e) => write!(f, "selected graph startup: {e:?}"),
            Self::WorldLock => f.write_str("selected player world lock poisoned"),
            Self::FormationLock => f.write_str("selected chase formation lock poisoned"),
        }
    }
}
impl SelectedPlayerStartup {
    pub(crate) fn new(owners: SelectedPlayerStartupOwners) -> Self {
        Self {
            owners,
            input: GameInputFrameSession {
                commands: GameInputCommandManager::new(),
                gameplay: GameplayInputUpdateManager::new(),
            },
            actor: None,
            physical_animation: None,
            mounted_attempted: false,
            attempted: false,
        }
    }

    /// The host supplies the selected loader, mounted data, actual global owners,
    /// spawn, flags, source and numeric policy. Partial stores remain retained on
    /// failure; this function is a single construction attempt, not a retry loop.
    pub(crate) fn try_initialize<S: AnimationStartupSource, N: ReckoningNumeric>(
        &mut self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        mounts: &MountSet,
        request: SelectedPlayerConstructionRequest,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), SelectedPlayerStartupError<S::Error, N::Error>> {
        use SelectedPlayerStartupError as E;
        if self.attempted {
            return Err(E::AlreadyAttempted);
        }
        self.attempted = true;
        let map = if let Some(map) = &self.owners.map {
            Arc::clone(map)
        } else {
            let bytes = mounts
                .read_path("data\\config\\input.cfg")
                .map_err(E::Mount)?;
            Arc::new(
                SelectedInputMap::parse_with_player_ui_inputs(
                    std::str::from_utf8(&bytes).map_err(E::Utf8)?,
                )
                .map_err(E::Map)?,
            )
        };
        self.owners.map = Some(Arc::clone(&map));
        self.input
            .gameplay
            .try_construct(|path| mounts.read_path(path))
            .map_err(E::InputConstruct)?;
        self.input
            .gameplay
            .try_reset(&map, &self.owners.services.vault, &mut self.input.commands)
            .map_err(E::InputReset)?;
        let player_index = self.owners.loader.player_index();
        let player = self
            .input
            .gameplay
            .player(player_index as usize)
            .map(Arc::clone)
            .ok_or(E::MissingPlayer(player_index))?;
        let formation =
            self.owners
                .formation
                .insert(Arc::new(Mutex::new(ActorChaseFormationOwner::new(
                    Arc::clone(&self.owners.services.vault),
                ))));
        let mut formation_fields = formation.lock().map_err(|_| E::FormationLock)?;
        formation_fields
            .try_bind_selected_reset()
            .map_err(E::Actor)?;
        let actor = self.actor.insert(Arc::new(
            ActorConstruction::new(
                player,
                world
                    .lock()
                    .map_err(|_| E::WorldLock)?
                    .physical_manager_authority(),
                request.actor_flags,
                request.spawn,
            )
            .map_err(E::Actor)?,
        ));
        actor
            .try_construct_prefix(&formation_fields, numeric)
            .map_err(E::Prefix)?;
        drop(formation_fields);
        actor
            .try_construct_listener(self.input.gameplay.route_mut())
            .map_err(E::Actor)?;
        actor.try_construct_graphs().map_err(E::Actor)?;
        let input_services = ActorInputRuntimeServices {
            attributes: crate::action_graph_input_listener::fill::ActionInputFillAttributes::new(
                &self.owners.services.vault,
            ),
            cvars: self.owners.cvars.fill_provider(),
            juice: self.owners.juice.clone(),
        };
        let mut adapter = ActorAnimationPlayerServices::try_bind(
            &self.owners.services,
            &self.owners.animation,
            &self.owners.cvars,
        )
        .map_err(E::AnimationBinding)?;
        self.physical_animation = Some(
            actor
                .try_complete_player_animation(
                    world,
                    &mut adapter,
                    &self.owners.faces,
                    source,
                    input_services,
                )
                .map_err(E::Animation)?,
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "selected_player_startup_tests.rs"]
mod tests;

#[path = "selected_player_roster_startup.rs"]
pub(crate) mod roster_startup;

#[path = "selected_player_mounted_startup.rs"]
pub(crate) mod mounted;
