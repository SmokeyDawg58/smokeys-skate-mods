//! Selected SkaterManager.InitPlayer826000b0 enablement and Action-load stages.
use super::{ActorConstruction, ActorConstructionError, Arc};
use crate::{
    actor_graph_construction::{ActorGraphAllocation, load::ActorGraphLoadError},
    generated_action_load::{
        GeneratedActionFactories, GeneratedActionLoadError, GeneratedActionStatics,
    },
    generated_onboard_state_graph::GeneratedOnboardEnablement,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_state_graph_preload::{
        GeneratedStateGraphDelegateStore, enable_generated_state_graph,
    },
};
use std::sync::Mutex;

const ACTION_FILE: &str = "state/ActionGraph_OnBoard.xml";
const MOTION_FILE: &str = "state/MotionGraph_OnBoard.xml";

/// Actual assets shared by the existing session cache and dynamic controllers.
pub(crate) enum CachedDynamicGraph {
    Camera(crate::camera_graph::SharedCameraGraph),
}

/// Boot-owned factory bindings and shared statics outlive individual players.
/// Only the registered classes required by the generated Action declaration
/// are represented; other graph families remain explicit unsupported routes.
pub(crate) struct ActorGraphStartupOwners {
    pub(crate) plugin_waypoint_bindings: [std::sync::OnceLock<Result<Arc<crate::living_world_grind_services::waypoints::plugin::waypoint::WaypointBindings>,String>>;5],
    pub(crate) plugin_spectate_bindings: [std::sync::OnceLock<Result<Arc<crate::living_world_grind_services::waypoints::plugin::spectate::SpectateBindings>,String>>;2],
    pub(crate) plugin_sources: Mutex<Vec<u32>>,
    pub(crate) plugin_mounts: Option<Arc<skate2_formats::mount::MountSet>>,
    pub(crate) plugin_cache: Mutex<crate::graph_asset_cache::BehaviourGraphCache<crate::living_world_grind_services::waypoints::plugin::load::ProcessedTransferable>>,
    pub(crate) plugin_lookat_bindings: std::sync::OnceLock<Result<Arc<crate::living_world_grind_services::waypoints::plugin::lookat::LookAtBindings>,String>>,
    pub(crate) plugin_trash_bindings: std::sync::OnceLock<Result<Arc<crate::living_world_grind_services::waypoints::plugin::trash::TrashBindings>,String>>,
    cache: Option<Arc<Mutex<crate::graph_asset_cache::StateGraphCache<CachedDynamicGraph>>>>,
    enabled: [u8; 20],
    living_world_controllers_ready: bool,
    pub(crate) factories: GeneratedActionFactories,
    pub(crate) statics: GeneratedActionStatics,
    motion_statics: Option<crate::generated_motion_load::GeneratedMotionStatics>,
    pedestrian_conditions: std::sync::OnceLock<Result<
        Arc<crate::pedestrian_graph::condition_binding::PedestrianConditions>, String>>,
}
impl ActorGraphStartupOwners {
    pub(crate) fn retained_dynamic_cache(&self) -> Option<crate::camera_graph::CameraGraphCache> {
        self.cache.as_ref().map(Arc::clone)
    }

    pub(crate) fn try_new() -> Result<Self, GeneratedActionLoadError> {
        let mut factories = GeneratedActionFactories::new();
        factories.register_skater_classes()?;
        // Cache82cf4f40 constructs an empty32-node map. Generated Create does
        // not decode into it, but InitPlayer still requires the real owner.
        let cache = Arc::new(Mutex::new(crate::graph_asset_cache::StateGraphCache::new()));
        Ok(Self {
            plugin_waypoint_bindings: std::array::from_fn(|_|std::sync::OnceLock::new()),
            plugin_spectate_bindings: std::array::from_fn(|_|std::sync::OnceLock::new()),
            plugin_sources: Mutex::new(Vec::new()),
            plugin_mounts: None,
            plugin_cache: Mutex::new(crate::graph_asset_cache::BehaviourGraphCache::new()),
            plugin_lookat_bindings: std::sync::OnceLock::new(),
            plugin_trash_bindings: std::sync::OnceLock::new(),
            cache: Some(cache),
            enabled: [0; 20],
            living_world_controllers_ready: false,
            factories,
            statics: GeneratedActionStatics::new(),
            motion_statics: None,
            pedestrian_conditions: std::sync::OnceLock::new(),
        })
    }
    pub(crate) fn enablement(&self) -> GeneratedOnboardEnablement {
        GeneratedOnboardEnablement::from_delegate_raw(self.enabled)
    }
    /// HCDelegate_Pedestrian.Init82926b60 constructs its89 immutable
    /// condition occurrences once, shared by every pedestrian controller.
    pub(crate) fn pedestrian_conditions(&self) -> Result<
        Arc<crate::pedestrian_graph::condition_binding::PedestrianConditions>, String> {
        self.pedestrian_conditions.get_or_init(||
            crate::pedestrian_graph::condition_binding::PedestrianConditions::new().map(Arc::new))
            .clone()
    }
    pub(crate) fn living_world_enablement(
        &self,
    ) -> Result<GeneratedOnboardEnablement, ActorGraphStartupError> {
        if !self.living_world_controllers_ready {
            return Err(ActorGraphStartupError::LivingWorldControllersNotPreloaded);
        }
        Ok(self.enablement())
    }
    /// Selected PreLoadStateGraphs controller prefix on the SAME delegate
    /// owner used by InitPlayer. Does not claim descriptor/prop preload.
    pub(crate) fn try_preload_living_world_controllers(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<(), ActorGraphStartupError> {
        if !self.living_world_controllers_ready {
            crate::living_world_state_graph_preload::try_preload_living_world_controllers(
                &mut LivingWorldControllers {
                    owners: self,
                    cvars,
                },
            )?;
            self.living_world_controllers_ready = true;
        }
        Ok(())
    }
    pub(crate) fn try_new_with_motion(
        vault: Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        cvars: InputCvarReferences,
    ) -> Result<Self, GeneratedActionLoadError> {
        let mut owners = Self::try_new()?;
        owners.motion_statics = Some(crate::generated_motion_load::GeneratedMotionStatics::new(
            Some(vault),
            Some(cvars),
        ));
        Ok(owners)
    }
    /// Both flags are written before either graph is loaded. Determinism is
    /// sampled once; player ID or the generated-graph bool is read separately
    /// for each path. Failed second read retains the first delegate write.
    pub(crate) fn try_enable_player(
        &mut self,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
    ) -> Result<(), ActorGraphStartupError> {
        let _cache = self
            .cache
            .as_ref()
            .map(Arc::clone)
            .ok_or(ActorGraphStartupError::MissingStateCache)?;
        let deterministic = cvars
            .try_test_determinism()
            .map_err(ActorGraphStartupError::Cvar)?;
        for file in [ACTION_FILE, MOTION_FILE] {
            let enabled = if deterministic {
                actor
                    .prefix()?
                    .fields()
                    .ok_or(ActorConstructionError::ActorFieldsUninitialized)?
                    .player_index
                    == 8
            } else {
                cvars
                    .try_use_generated_state_graph()
                    .map_err(ActorGraphStartupError::Cvar)?
            };
            enable_generated_state_graph(self, Some(file), u8::from(enabled));
        }
        Ok(())
    }
}
impl GeneratedStateGraphDelegateStore for ActorGraphStartupOwners {
    fn store_generated_delegate_raw(&mut self, index: usize, _: &'static str, value: u8) {
        self.enabled[index] = value;
    }
}

struct LivingWorldControllers<'a> {
    owners: &'a mut ActorGraphStartupOwners,
    cvars: &'a InputCvarReferences,
}
impl GeneratedStateGraphDelegateStore for LivingWorldControllers<'_> {
    fn store_generated_delegate_raw(&mut self, index: usize, name: &'static str, raw: u8) {
        self.owners.store_generated_delegate_raw(index, name, raw);
    }
}
impl crate::living_world_state_graph_preload::LivingWorldControllerPreloadRuntime
    for LivingWorldControllers<'_>
{
    type Error = ActorGraphStartupError;
    type Target = crate::input_cvar_runtime::InputCvarBooleanTarget;
    fn acquire_target(&mut self) -> Result<Self::Target, Self::Error> {
        self.cvars
            .try_capture_living_world_generated_graph()
            .map_err(ActorGraphStartupError::Cvar)
    }
    fn read_target(target: &Self::Target) -> Result<u8, Self::Error> {
        target
            .try_value()
            .map(u8::from)
            .map_err(ActorGraphStartupError::Cvar)
    }
    fn load_controller(&mut self, path: &'static str, _flush: bool) -> Result<(), Self::Error> {
        Err(ActorGraphStartupError::LivingWorldDynamicControllerPending(
            path,
        ))
    }
}

#[derive(Debug)]
pub(crate) enum ActorGraphStartupError {
    LivingWorldControllersNotPreloaded,
    LivingWorldDynamicControllerPending(&'static str),
    Actor(ActorConstructionError),
    Cvar(InputCvarError),
    MissingAnimation,
    MissingStateCache,
    MissingAction,
    MissingMotion,
    ForeignMotion,
    MissingMotionStatics,
    ForeignAction,
    MissingWrapper,
    Load(ActorGraphLoadError),
}
impl From<ActorConstructionError> for ActorGraphStartupError {
    fn from(error: ActorConstructionError) -> Self {
        Self::Actor(error)
    }
}
impl std::fmt::Display for ActorGraphStartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "Actor graph startup: {error}"),
            Self::Cvar(error) => write!(f, "graph startup cvar: {error:?}"),
            Self::Load(error) => write!(f, "Actor graph load: {error}"),
            other => write!(f, "Actor graph startup: {other:?}"),
        }
    }
}
impl ActorConstruction {
    /// Load through the registered Motion allocation and its original wrapper.
    /// Eager constructors complete before publishing the abstract controller;
    /// this does not run graph callbacks or complete `InitPlayer`'s later stages.
    pub(crate) fn try_load_motion_graph(
        &self,
        owners: &ActorGraphStartupOwners,
    ) -> Result<bool, ActorGraphStartupError> {
        let mut state = self.prefix()?;
        let _cache = owners
            .cache
            .as_ref()
            .ok_or(ActorGraphStartupError::MissingStateCache)?;
        if !state.animation_published {
            return Err(ActorGraphStartupError::MissingAnimation);
        }
        let registered = state.graph_interfaces[1]
            .as_ref()
            .map(Arc::clone)
            .ok_or(ActorGraphStartupError::MissingMotion)?;
        let motion = state
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .ok_or(ActorGraphStartupError::MissingMotion)?;
        if !Arc::ptr_eq(&registered, motion.occurrence()) {
            return Err(ActorGraphStartupError::ForeignMotion);
        }
        let wrapper = motion
            .controller
            .as_mut()
            .ok_or(ActorGraphStartupError::MissingWrapper)?;
        wrapper
            .try_load_motion(
                MOTION_FILE.as_bytes(),
                owners.enablement(),
                owners
                    .motion_statics
                    .as_ref()
                    .ok_or(ActorGraphStartupError::MissingMotionStatics)?,
                ActorGraphAllocation::Host,
            )
            .map_err(ActorGraphStartupError::Load)
    }
    /// Actual Action UCOM occurrence is checked against the published graph.
    /// Motion loading/observers/local-player publication remain later stages;
    /// this entry does not claim full `InitPlayer` or physical Actor admission.
    pub(crate) fn try_load_action_graph(
        &self,
        owners: &ActorGraphStartupOwners,
    ) -> Result<bool, ActorGraphStartupError> {
        let mut state = self.prefix()?;
        let _cache = owners
            .cache
            .as_ref()
            .ok_or(ActorGraphStartupError::MissingStateCache)?;
        if !state.animation_published {
            return Err(ActorGraphStartupError::MissingAnimation);
        }
        let registered = state.graph_interfaces[0]
            .as_ref()
            .map(Arc::clone)
            .ok_or(ActorGraphStartupError::MissingAction)?;
        let action = state
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.action_mut())
            .ok_or(ActorGraphStartupError::MissingAction)?;
        if !Arc::ptr_eq(&registered, action.occurrence()) {
            return Err(ActorGraphStartupError::ForeignAction);
        }
        let wrapper = action
            .controller
            .as_mut()
            .ok_or(ActorGraphStartupError::MissingWrapper)?;
        wrapper
            .try_load(
                ACTION_FILE.as_bytes(),
                owners.enablement(),
                &owners.statics,
                &owners.factories,
                ActorGraphAllocation::Host,
            )
            .map_err(ActorGraphStartupError::Load)
    }
}
