//! Unregistered draft: the selected offline world/local-owner constructor prefix.
//!
//! This stops before world streaming, Actor construction, physical admission,
//! camera capture, input or rendering. It does not manufacture a PROXY result.

use crate::{
    attribute_runtime::AttributeRuntimeVault,
    debug_overlay::DebugOverlayError,
    dynamic_assembly_session::skater_pool::SimulationSkaterPoolError,
    hermes_challenge_queue::{ChallengeEventQueue, ChallengeQueueError},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    simulation_assembly_backend::constraint_session::{
        grab_world::GrabManagerAlreadyAttempted,
        grind_world::PhysicalGrindError,
        live_frame_services::LiveContactFeedbackError,
        prediction::PredictionInitError,
        world_services::{PhysicalGameInfo, PhysicalWorldServicesError, SelectedMainWorldError},
    },
    simulation_entity_roster::SimulationEntityRoster,
    skater_manager_local_identity::{
        LocalBackendOwner, LocalIdentityError, SkaterManagerLocalIdentity,
        observers::SkaterObserverError,
    },
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum SelectedSessionWorldStartupError {
    Cvar(InputCvarError),
    Backend(LocalIdentityError),
    World(SelectedMainWorldError),
    Feedback(LiveContactFeedbackError),
    Prediction(PredictionInitError),
    StepSelector(
        crate::simulation_assembly_backend::constraint_session::step_selector::StepSelectorError,
    ),
    Grab(GrabManagerAlreadyAttempted),
    Overlay(DebugOverlayError),
    Grind(PhysicalGrindError),
    Queue(ChallengeQueueError),
    LivingWorldEventOwnerLock,
    SimRate(crate::simulation_sim_rate::SimRateError),
    GraphOwners(crate::generated_action_load::GeneratedActionLoadError),
    GraphPreload(crate::actor_construction::graph_startup::ActorGraphStartupError),
    Recipes(crate::selected_living_world_recipes::SelectedLivingWorldRecipeError),
    Props(String),
    Observers(SkaterObserverError),
    Local(LocalIdentityError),
    Services(PhysicalWorldServicesError),
    SkaterPool(SimulationSkaterPoolError),
    PedestrianPool(crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoPoolError<std::convert::Infallible>),
}

/// Both handles point into the same source-selected world/local backend graph.
pub(crate) struct SelectedSessionWorldStartup {
    pub(crate) graphs: Arc<Mutex<crate::actor_construction::graph_startup::ActorGraphStartupOwners>>,
    pub(crate) living_world_recipes: Arc<Mutex<crate::selected_living_world_recipes::SelectedLivingWorldRecipes>>,
    pub(crate) game_sim_events: Arc<crate::hermes_game_sim_queue::GameSimEventRoute>,
    pub(crate) world: Arc<Mutex<PhysicalAssemblyWorld>>,
    pub(crate) local: Arc<SkaterManagerLocalIdentity>,
}

impl SelectedSessionWorldStartup {
    /// Source order is the mounted roster's `repeated_frames && native_roster`
    /// constructor prefix. `kbEnableGhostByDefault` and parallel-pool cvars are
    /// read live; unsupported ghost/parallel paths fail through their owners.
    /// An error ends this boot attempt; it does not replay partial operations.
    pub(crate) fn try_new(
        cvars: &InputCvarReferences,
        vault: &Arc<AttributeRuntimeVault>,
        mounts: &Arc<skate2_formats::mount::MountSet>,
    ) -> Result<Self, SelectedSessionWorldStartupError> {
        use SelectedSessionWorldStartupError as E;
        let game_sim_events=crate::hermes_game_sim_queue::GameSimEventRoute::try_construct().map_err(E::Queue)?;
        // Native App.Init constructs RecipeMan82734ff4 before Simulation;
        // SecondaryLoad03 initializes mappings before character preparation.
        let mut recipes=crate::selected_living_world_recipes::SelectedLivingWorldRecipes::new();
        recipes.initialize(vault,cvars).map_err(E::Recipes)?;
        let living_world_recipes=Arc::new(Mutex::new(recipes));
        let mut graph_owner=crate::actor_construction::graph_startup::ActorGraphStartupOwners::try_new_with_motion(Arc::clone(vault),cvars.clone()).map_err(E::GraphOwners)?;
        // PreLoadStateGraphs loads behaviours before InitPropData (82725480,
        // 8272550c). Its decoder must already have the session's actual mounts
        // when actor-tracker initialization below loads its plugin template.
        graph_owner.plugin_mounts=Some(Arc::clone(mounts));
        graph_owner.try_preload_living_world_controllers(cvars).map_err(E::GraphPreload)?;
        // Its BehaviourMan::Load of every plugin descriptor (82724c00..827254e4): a spawning
        // prop finds its descriptor in the cache. A family the port has no controller for fails
        // here as it would at the prop.
        for path in crate::living_world_state_graph_preload::LIVING_WORLD_BEHAVIOUR_GRAPH_PATHS {
            let _ = crate::living_world_grind_services::waypoints::plugin::load::load(&graph_owner, path);
        }
        let graphs=Arc::new(Mutex::new(graph_owner));
        let game_info = PhysicalGameInfo::construct_network_field();
        let backend = LocalBackendOwner::construct_static_fields(Arc::clone(&game_info));
        let ghost = cvars.try_enable_ghost_by_default().map_err(E::Cvar)?;
        backend
            .try_apply_debug_menu_ghost_sample(ghost)
            .map_err(E::Backend)?;
        let mut world = PhysicalAssemblyWorld::try_new_selected_main(&backend).map_err(E::World)?;
        world.sim_rate.lock().map_err(|_|E::LivingWorldEventOwnerLock)?
            .bind_game_sim_events(Arc::clone(&game_sim_events)).map_err(E::SimRate)?;
        world
            .try_initialize_world_feedback_parameters()
            .map_err(E::Feedback)?;
        world
            .try_initialize_prediction_owner(vault, true)
            .map_err(E::Prediction)?;
        world
            .try_initialize_step_selector(true, cvars)
            .map_err(E::StepSelector)?;
        world
            .try_initialize_grab_query_manager(true)
            .map_err(E::Grab)?;
        world.try_initialize_debug_overlay().map_err(E::Overlay)?;
        world
            .grind_services
            .try_initialize_static()
            .map_err(E::Grind)?;
        world.grind_services.living_world().lock().map_err(|_|E::LivingWorldEventOwnerLock)?
            .game_sim_events=Some(Arc::clone(&game_sim_events));
        world.grind_services.living_world().lock().map_err(|_|E::LivingWorldEventOwnerLock)?
            .recipe_runtime=Some(Arc::clone(&living_world_recipes));
        world.grind_services.living_world().lock().map_err(|_|E::LivingWorldEventOwnerLock)?
            .graph_runtime=Some(Arc::clone(&graphs));
        world
            .grind_services
            .try_initialize_entities(vault, cvars)
            .map_err(E::Grind)?;
        {
            // Source PreLoadStateGraphs8272550c ends with InitPropData. Host
            // graph preload precedes World creation, so finish that step as
            // soon as the same PropMan and CensusMan have been initialized.
            let graph_owner=graphs.lock().map_err(|_|E::LivingWorldEventOwnerLock)?;
            let recipe_owner=living_world_recipes.lock().map_err(|_|E::LivingWorldEventOwnerLock)?;
            let mut frame=crate::selected_living_world_recipes::props::PropRecipeFrame{
                graphs:&graph_owner,recipes:&recipe_owner.recipes,avoid_volumes:&recipe_owner.challenge_avoid_volumes,vault};
            let mut living=world.grind_services.living_world().lock().map_err(|_|E::LivingWorldEventOwnerLock)?;
            frame.initialize_actor_tracker(&mut living,cvars).map_err(E::Props)?;
        }
        world
            .grind_services
            .try_install_dynamic_provider()
            .map_err(E::Grind)?;
        world.entity_roster = Some(SimulationEntityRoster::construct(
            world.physical_manager_authority(),
        ));
        let local = SkaterManagerLocalIdentity::construct_local_fields(backend);
        world.local_skater_identity = Some(Arc::clone(&local));
        let queue = ChallengeEventQueue::try_construct().map_err(E::Queue)?;
        local
            .try_register_startup_observers(queue)
            .map_err(E::Observers)?;
        local
            .try_reset_game_before_first_spawn()
            .map_err(E::Local)?;
        world
            .try_initialize_physical_world_services(game_info, true)
            .map_err(E::Services)?;
        let pedestrian_parallel = cvars.try_pedestrian_pool_update_in_parallel().map_err(E::Cvar)?;
        world.try_initialize_pedestrian_pool(pedestrian_parallel).map_err(E::PedestrianPool)?;
        let parallel = cvars.try_skater_pool_parallel().map_err(E::Cvar)?;
        world
            .try_initialize_skater_pool(world.island_flags(), parallel)
            .map_err(E::SkaterPool)?;
        Ok(Self {
            graphs,
            game_sim_events,
            living_world_recipes,
            world: Arc::new(Mutex::new(world)),
            local,
        })
    }
}
