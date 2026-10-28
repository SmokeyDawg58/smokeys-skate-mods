//! Selected terrain's separate kind-31 pass: `ReceiveRegCollectionMsg`
//! 0x8277eb10..eba0 runs `Simulation::RegDMOArena` over every arena after the
//! normal `RegArena` loop, whether or not the message activates. The asset
//! lookup is `ProcessDMOData` 0x82d5f4f0's `AddSimRegistryAsset` (kind 0x1f)
//! with the registry's first full-ID winner.
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_grind_services::{
        DmoLocatorRouteError, LivingWorldGrindServices, dmo_locators::LocatorServices,
    },
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric},
    simulation_dmo_arena_adapter::{
        PreparedSimulationDmoArena, SimulationDmoArenaDispatch,
        SimulationDmoArenaRegistrationAdapter, SimulationDmoLoadPolicy,
    },
    simulation_dmo_arena_plan::SimulationDmoDataHandle,
};
use skate2_formats::pegasus::{Dmo, DmoData, PegasusError};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug)]
pub(crate) enum DmoServiceError {
    Cvar(InputCvarError),
    Pegasus(PegasusError),
    Numeric(ReckoningHostNumericError),
}

impl std::fmt::Display for DmoServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => write!(f, "DMO cvar read: {error:?}"),
            Self::Pegasus(error) => write!(f, "DMO locator chunk: {error}"),
            Self::Numeric(error) => write!(f, "DMO locator numeric: {error:?}"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum DmoPassError {
    Policy(InputCvarError),
    LivingWorldLock,
    SharedLock,
    Route(DmoLocatorRouteError<DmoServiceError>),
    Lifecycle(String),
}

impl std::fmt::Display for DmoPassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy(error) => write!(f, "kLoadDmoLocatorFromStream read: {error:?}"),
            Self::LivingWorldLock => f.write_str("LivingWorld owner lock poisoned"),
            Self::SharedLock => f.write_str("DMO pass state lock poisoned"),
            Self::Route(error) => error.fmt(f),
            Self::Lifecycle(error) => write!(f, "DMO streaming lifecycle: {error}"),
        }
    }
}

/// Kind-31 registry rows and the pass's first failure, shared by the policy,
/// the dispatch and the terrain owner.
#[derive(Default)]
struct PassState {
    registry: HashMap<u64, SimulationDmoDataHandle>,
    failure: Option<DmoPassError>,
    cull_context: Option<CullContext>,
    /// `cChallengeDMOManager::ChallengeDMOGroups` (+0x688): the challenge hulls' DMO assets
    /// in arrival order.
    challenge_groups: Vec<u64>,
    /// `Dmo::Registration`'s locators: the DMO assets registered with userdata `& 7 == 7`, an
    /// online game's challenge hulls (`LoadChallengeAssets` asset flags 7), in arrival order.
    static_groups: Vec<u64>,
}

/// Resolved for this synchronous deregistration, while the caller holds the
/// actual RecipeMan. Avoid reacquiring that mutex inside arena callbacks.
struct CullContext {
    vault: Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    painter: Arc<Mutex<crate::world_painter_layer::WorldPainterLayerMan>>,
    avoid: crate::pedestrian_navigation::wander::ResolvedAvoidVolumes,
}

type SharedPass = Arc<Mutex<PassState>>;

fn latch(shared: &SharedPass, error: DmoPassError) {
    if let Ok(mut state) = shared.lock() {
        state.failure.get_or_insert(error);
    }
}

pub(crate) struct SelectedDmoLoadPolicy {
    cvars: InputCvarReferences,
    shared: SharedPass,
}

impl SimulationDmoLoadPolicy for SelectedDmoLoadPolicy {
    fn load_dmo_locator_from_stream(&mut self) -> bool {
        self.cvars
            .try_load_dmo_locator_from_stream()
            .unwrap_or_else(|error| {
                // The latched failure ends the pass before any route runs.
                latch(&self.shared, DmoPassError::Policy(error));
                false
            })
    }
}

pub(crate) struct PassServices<'a> {
    cvars: &'a InputCvarReferences,
    registry: &'a HashMap<u64, SimulationDmoDataHandle>,
}

/// `cChallengeDMOManager::ChallengeDMOGroups` with the kind-31 records their locators come
/// from, taken for the challenge DMO manager's calls into the locator manager.
pub(crate) struct ChallengeDmoChunks {
    pub(crate) groups: Vec<u64>,
    registry: HashMap<u64, SimulationDmoDataHandle>,
    cvars: InputCvarReferences,
}

impl ChallengeDmoChunks {
    /// The locator services the streamed calls read: the records and the locator cvars.
    pub(crate) fn services(&self) -> PassServices<'_> {
        PassServices {
            cvars: &self.cvars,
            registry: &self.registry,
        }
    }
}

impl LocatorServices for PassServices<'_> {
    type Error = DmoServiceError;
    fn enable_locator_loading(&mut self) -> Result<bool, DmoServiceError> {
        self.cvars
            .try_enable_dmo_locator_loading()
            .map_err(DmoServiceError::Cvar)
    }
    fn remove_bad_locators(&mut self) -> Result<bool, DmoServiceError> {
        self.cvars
            .try_enable_the_hack_to_remove_bad_locators()
            .map_err(DmoServiceError::Cvar)
    }
    fn locator_chunk(&mut self, asset_id: u64) -> Result<Option<Vec<Dmo>>, DmoServiceError> {
        let Some(handle) = self.registry.get(&asset_id) else {
            return Ok(None);
        };
        DmoData::parse(handle.bytes())
            .map(|data| Some(data.dmos))
            .map_err(DmoServiceError::Pegasus)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, DmoServiceError> {
        HostReckoningNumeric
            .reciprocal_sqrt_estimate(value)
            .map_err(DmoServiceError::Numeric)
    }
}

pub(crate) struct SelectedDmoDispatch {
    physical_world:Arc<Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>,
    living_world: Arc<Mutex<LivingWorldGrindServices>>,
    cvars: InputCvarReferences,
    shared: SharedPass,
}

impl SelectedDmoDispatch {
    fn failed(&self) -> bool {
        self.shared
            .lock()
            .map_or(true, |state| state.failure.is_some())
    }

    /// `RemoveStreamedLocatorData(asset, userdata, true, false)` (0x82d3d618) of a challenge
    /// DMO group. The objects it culls get the before-cull services of the deregistration's
    /// cull context; without one only their monitors hear, as the living world removal's.
    fn try_remove_challenge_locator_data(&self, asset_id: u64, userdata: u32) -> Result<(), DmoPassError> {
        use crate::living_world_grind_services::dmo_objects::DmoCullEvent;
        let state = self.shared.lock().map_err(|_| DmoPassError::SharedLock)?;
        let mut physical = self.physical_world.lock().map_err(|_| DmoPassError::LivingWorldLock)?;
        let mut world = self.living_world.lock().map_err(|_| DmoPassError::LivingWorldLock)?;
        let mut services = PassServices {
            cvars: &self.cvars,
            registry: &state.registry,
        };
        let Some(context) = state.cull_context.as_ref() else {
            let world = &mut *world;
            let absent = || DmoPassError::Lifecycle("challenge locator removal managers absent".into());
            let pedestrians = world.pedestrian_entities.as_deref_mut().ok_or_else(absent)?;
            let objects = world.dmo_objects.as_deref_mut().ok_or_else(absent)?;
            let locators = world.dmo_locators.as_deref_mut().ok_or_else(absent)?;
            return locators
                .remove_streamed_locator_data_with_lifecycle(
                    asset_id, userdata, true, false, objects, &mut services, |objects, _, event| match event {
                        DmoCullEvent::BeforeCull(id) => {
                            if objects.object(id).is_some_and(|o| !o.contained_props.is_empty()) {
                                Err("streamed DMO child props require synchronous before-cull services".into())
                            } else {
                                Ok(())
                            }
                        }
                        DmoCullEvent::Monitor { entity, monitor } => {
                            if let Some(pedestrian) = pedestrians.get_mut(monitor) {
                                pedestrian.monitored_object_being_culled(entity, &mut physical, objects, &self.cvars)?;
                            }
                            Ok(())
                        }
                    })
                .map_err(|e| DmoPassError::Lifecycle(format!("challenge locator removal: {e:?}")));
        };
        let mut random = world.random.take()
            .ok_or_else(|| DmoPassError::Lifecycle("Manager RNG absent".into()))?;
        let result = (|| -> Result<(), String> {
            let actors = physical.entity_roster.as_ref().ok_or("stream cull Actor roster absent")?
                .snapshot().map_err(|e| format!("stream cull Actor roster: {e:?}"))?;
            let painter = context.painter.lock().map_err(|_| "stream cull painter lock")?;
            let (mut owners, objects, locators) =
                crate::pedestrian_graph::backend::frame::FrameOwners::split(&mut world)?;
            locators.remove_streamed_locator_data_with_lifecycle(
                asset_id, userdata, true, false, objects, &mut services, |objects, locators, event| match event {
                    DmoCullEvent::BeforeCull(id) => owners.before_dmo_cull(
                        &mut physical, objects, locators, id, &self.cvars,
                        &painter, &context.vault, &mut random, &actors, &context.avoid),
                    DmoCullEvent::Monitor { entity, monitor } => {
                        if let Some(pedestrian) = owners.pedestrians.get_mut(monitor) {
                            pedestrian.monitored_object_being_culled(entity, &mut physical, objects, &self.cvars)?;
                        }
                        Ok(())
                    }
                }).map_err(|e| format!("challenge locator removal: {e:?}"))
        })();
        world.random = Some(random);
        result.map_err(DmoPassError::Lifecycle)
    }

    fn try_add_living_world(&self, asset_id: u64, userdata_raw: u32) -> Result<(), DmoPassError> {
        let state = self.shared.lock().map_err(|_| DmoPassError::SharedLock)?;
        let mut living_world = self
            .living_world
            .lock()
            .map_err(|_| DmoPassError::LivingWorldLock)?;
        let mut services = PassServices {
            cvars: &self.cvars,
            registry: &state.registry,
        };
        living_world
            .try_add_streamed_locator_data(asset_id, userdata_raw, &mut services)
            .map_err(DmoPassError::Route)
    }
}

impl SimulationDmoArenaDispatch for SelectedDmoDispatch {
    fn add_living_world_locator_data(&mut self, asset_id: u64, userdata_raw: u32) {
        if self.failed() {
            return;
        }
        if let Err(error) = self.try_add_living_world(asset_id, userdata_raw) {
            latch(&self.shared, error);
        }
    }
    /// `cChallengeDMOManager::AddChallengeDMOs` (0x825bdc48): only records the asset, once;
    /// `RunDMOOperations` places its locators. The manager is activated outside the front
    /// end (`GameLoad` 0x825b03f8), which is the only place hulls load.
    fn add_challenge_dmos(&mut self, asset_id: u64, _: u32) {
        let Ok(mut state) = self.shared.lock() else { return };
        if !state.challenge_groups.contains(&asset_id) {
            state.challenge_groups.push(asset_id);
        }
    }
    /// `Dmo::Registration::RegDmoLocator` (`0x825eac28`): the asset once, the first of its full
    /// ID kept; `SpawnStatic` places its locators.
    fn register_dmo_locator(&mut self, _: &SimulationDmoDataHandle, asset_id: u64) {
        let Ok(mut state) = self.shared.lock() else { return };
        if !state.static_groups.contains(&asset_id) {
            state.static_groups.push(asset_id);
        }
    }
    fn remove_living_world_locator_data(&mut self, asset_id: u64, userdata: u32) {
        if self.failed() {
            return;
        }
        let result = (|| {
            let state = self.shared.lock().map_err(|_| DmoPassError::SharedLock)?;
            // Streaming is outside the tick's World guard. Preserve the same
            // World->LivingWorld lock order as the simulation frame.
            let mut physical=self.physical_world.lock().map_err(|_|DmoPassError::LivingWorldLock)?;
            let mut world = self
                .living_world
                .lock()
                .map_err(|_| DmoPassError::LivingWorldLock)?;
            let mut services = PassServices {
                cvars: &self.cvars,
                registry: &state.registry,
            };
            let Some(context) = state.cull_context.as_ref() else {
                return world.try_remove_streamed_locator_data_with_world(
                    asset_id, userdata, &mut services, &mut physical, &self.cvars)
                    .map_err(DmoPassError::Route);
            };
            let mut random = world.random.take()
                .ok_or_else(||DmoPassError::Lifecycle("Manager RNG absent".into()))?;
            let result = (|| -> Result<(), String> {
                let actors = physical.entity_roster.as_ref().ok_or("stream cull Actor roster absent")?
                    .snapshot().map_err(|e|format!("stream cull Actor roster: {e:?}"))?;
                let painter = context.painter.lock().map_err(|_|"stream cull painter lock")?;
                let (mut owners, objects, locators) =
                    crate::pedestrian_graph::backend::frame::FrameOwners::split(&mut world)?;
                locators.remove_streamed_locator_data_with_lifecycle(
                    asset_id, userdata, false, true, objects, &mut services, |objects, locators, event| {
                        use crate::living_world_grind_services::dmo_objects::DmoCullEvent;
                        match event {
                            DmoCullEvent::BeforeCull(id) => owners.before_dmo_cull(
                                &mut physical, objects, locators, id, &self.cvars,
                                &painter, &context.vault, &mut random, &actors, &context.avoid),
                            DmoCullEvent::Monitor{entity, monitor} => {
                                if let Some(pedestrian) = owners.pedestrians.get_mut(monitor) {
                                    pedestrian.monitored_object_being_culled(
                                        entity, &mut physical, objects, &self.cvars)?;
                                }
                                Ok(())
                            }
                        }
                    }).map_err(|e|format!("streamed locator removal: {e:?}"))
            })();
            world.random = Some(random);
            result.map_err(DmoPassError::Lifecycle)
        })();
        if let Err(error) = result {
            latch(&self.shared, error);
        }
    }
    /// `cChallengeDMOManager::RemoveChallengeDMOs` (0x825bbde8): a recorded asset's locators
    /// go (`RemoveStreamedLocatorData(asset, userdata, true, false)`: their objects culled,
    /// none kept for persistence), then the record. The port unloads challenge hulls only
    /// while the manager is activated.
    fn remove_challenge_dmos(&mut self, asset_id: u64, userdata: u32) {
        if self.failed() {
            return;
        }
        let recorded = self
            .shared
            .lock()
            .is_ok_and(|state| state.challenge_groups.contains(&asset_id));
        if !recorded {
            return;
        }
        if let Err(error) = self.try_remove_challenge_locator_data(asset_id, userdata) {
            latch(&self.shared, error);
            return;
        }
        if let Ok(mut state) = self.shared.lock() {
            state.challenge_groups.retain(|&group| group != asset_id);
        }
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career DMOs: challenge DMO group {asset_id:016x} removed");
        }
    }
    /// `Dmo::Registration::DRegDmoLocator` (`0x825ea6b8`): the asset goes by its full ID.
    fn deregister_dmo_locator(&mut self, _: &SimulationDmoDataHandle, asset_id: u64) {
        if let Ok(mut state) = self.shared.lock() {
            state.static_groups.retain(|&group| group != asset_id);
        }
    }
}

/// The live route: the retained LivingWorld Manager occurrence and cvars.
pub(crate) struct SelectedDmoRoute {
    pub(crate) physical_world:Arc<Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>,
    pub(crate) living_world: Arc<Mutex<LivingWorldGrindServices>>,
    pub(crate) cvars: InputCvarReferences,
}

pub(crate) struct SelectedDmoPass {
    adapter: SimulationDmoArenaRegistrationAdapter<SelectedDmoLoadPolicy, SelectedDmoDispatch>,
    shared: SharedPass,
    registrations: Vec<crate::simulation_dmo_arena_adapter::SimulationDmoArenaRegistration>,
}

impl SelectedDmoPass {
    /// ReceiveDRegCollectionMsg8277ebb8 visits every DMO arena before any
    /// normal arena. Range indexes match the terrain owner's ordered plans.
    pub(crate) fn try_deregister(
        &mut self,
        range: std::ops::Range<usize>,
        plans: &[Arc<crate::simulation_normal_arena_plan::SimulationNormalArenaPlan>],
        userdata: u32,
        props: Option<&crate::selected_living_world_recipes::props::PropRecipeFrame<'_>>,
        painter: &Arc<Mutex<crate::world_painter_layer::WorldPainterLayerMan>>,
        triggers: &Arc<Mutex<crate::trigger_camera_group::CameraTriggerGroup>>,
    ) -> Result<(), SelectedDmoPassError> {
        let context = props.map(|props| -> Result<_, DmoPassError> {
            let triggers = triggers.lock().map_err(|_|DmoPassError::Lifecycle("stream cull trigger lock".into()))?;
            Ok(CullContext {
                vault: Arc::clone(props.vault), painter: Arc::clone(painter),
                avoid: props.avoid_volumes.resolve(&triggers).map_err(DmoPassError::Lifecycle)?,
            })
        }).transpose().map_err(SelectedDmoPassError::Pass)?;
        self.shared.lock().map_err(|_|SelectedDmoPassError::Pass(DmoPassError::SharedLock))?.cull_context = context;
        let result = self.try_deregister_scoped(range, plans, userdata);
        self.shared.lock().map_err(|_|SelectedDmoPassError::Pass(DmoPassError::SharedLock))?.cull_context = None;
        result
    }

    fn try_deregister_scoped(
        &mut self,
        range: std::ops::Range<usize>,
        plans: &[Arc<crate::simulation_normal_arena_plan::SimulationNormalArenaPlan>],
        userdata: u32,
    ) -> Result<(), SelectedDmoPassError> {
        for registration in self.registrations.drain(range) {
            self.adapter
                .deregister(registration, userdata)
                .map_err(SelectedDmoPassError::Deregister)?;
            let state = self
                .shared
                .lock()
                .map_err(|_| SelectedDmoPassError::Pass(DmoPassError::SharedLock))?;
            if let Some(error) = state.failure.clone() {
                return Err(SelectedDmoPassError::Pass(error));
            }
        }
        let mut state = self
            .shared
            .lock()
            .map_err(|_| SelectedDmoPassError::Pass(DmoPassError::SharedLock))?;
        for plan in plans {
            let dmo = crate::simulation_dmo_arena_plan::SimulationDmoArenaPlan::from_normal_plan(
                Arc::clone(plan),
            )
            .map_err(SelectedDmoPassError::Plan)?;
            for (index, operation) in dmo.operations().iter().enumerate() {
                let source = dmo.source_handle(index).expect("existing DMO source");
                if state
                    .registry
                    .get(&operation.asset_id)
                    .is_some_and(|current| current.is_same_backing_object(&source))
                {
                    state.registry.remove(&operation.asset_id);
                }
            }
        }
        Ok(())
    }
    /// The recorded challenge DMO groups and the kind-31 records, taken.
    pub(crate) fn challenge_dmo_chunks(&self) -> Result<ChallengeDmoChunks, DmoPassError> {
        let state = self.shared.lock().map_err(|_| DmoPassError::SharedLock)?;
        Ok(ChallengeDmoChunks {
            groups: state.challenge_groups.clone(),
            registry: state.registry.clone(),
            cvars: self.adapter.dispatch().cvars.clone(),
        })
    }

    /// `Dmo::Registration`'s locator groups.
    pub(crate) fn static_dmo_groups(&self) -> Result<Vec<u64>, DmoPassError> {
        Ok(self.shared.lock().map_err(|_| DmoPassError::SharedLock)?.static_groups.clone())
    }

    /// `Dmo::Registration`'s locator groups with the kind-31 records their locators come from,
    /// for `SpawnStatic`.
    pub(crate) fn static_dmo_chunks(&self) -> Result<ChallengeDmoChunks, DmoPassError> {
        let state = self.shared.lock().map_err(|_| DmoPassError::SharedLock)?;
        Ok(ChallengeDmoChunks {
            groups: state.static_groups.clone(),
            registry: state.registry.clone(),
            cvars: self.adapter.dispatch().cvars.clone(),
        })
    }

    pub(crate) fn new(route: SelectedDmoRoute) -> Self {
        let shared = SharedPass::default();
        let policy = SelectedDmoLoadPolicy {
            cvars: route.cvars.clone(),
            shared: Arc::clone(&shared),
        };
        let dispatch = SelectedDmoDispatch {
            physical_world:route.physical_world,
            living_world: route.living_world,
            cvars: route.cvars,
            shared: Arc::clone(&shared),
        };
        Self {
            adapter: SimulationDmoArenaRegistrationAdapter::new(policy, dispatch),
            shared,
            registrations: Vec::new(),
        }
    }

    /// `ProcessDMOData` for every arena of the message, then `RegDMOArena` in
    /// arena order. The registry keeps the first row per full ID.
    pub(crate) fn try_register(
        &mut self,
        arenas: &[PreparedSimulationDmoArena],
        userdata: u32,
    ) -> Result<(), SelectedDmoPassError> {
        {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| SelectedDmoPassError::Pass(DmoPassError::SharedLock))?;
            for arena in arenas {
                let plan = arena.plan();
                for (index, operation) in plan.operations().iter().enumerate() {
                    if state.registry.contains_key(&operation.asset_id) {
                        continue;
                    }
                    let handle = plan
                        .source_handle(index)
                        .expect("prepared DMO operations retain their sources");
                    state
                        .registry
                        .try_reserve(1)
                        .map_err(|_| SelectedDmoPassError::Allocation)?;
                    state.registry.insert(operation.asset_id, handle);
                }
            }
        }
        self.registrations
            .try_reserve(arenas.len())
            .map_err(|_| SelectedDmoPassError::Allocation)?;
        for arena in arenas {
            let registration = self
                .adapter
                .register(arena, userdata)
                .map_err(SelectedDmoPassError::Register)?;
            self.registrations.push(registration);
            let state = self
                .shared
                .lock()
                .map_err(|_| SelectedDmoPassError::Pass(DmoPassError::SharedLock))?;
            // The latched failure stays: every later route is refused too.
            if let Some(error) = state.failure.clone() {
                return Err(SelectedDmoPassError::Pass(error));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum SelectedDmoPassError {
    Allocation,
    Register(crate::simulation_dmo_arena_adapter::SimulationDmoRegistrationError),
    Deregister(crate::simulation_dmo_arena_adapter::SimulationDmoDeregistrationError),
    Plan(crate::simulation_dmo_arena_plan::SimulationDmoArenaPlanError),
    Pass(DmoPassError),
}

impl std::fmt::Display for SelectedDmoPassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allocation => f.write_str("DMO pass storage allocation failed"),
            Self::Register(error) => write!(f, "DMO arena registration: {error}"),
            Self::Deregister(error) => write!(f, "DMO arena deregistration: {error}"),
            Self::Plan(error) => write!(f, "DMO arena plan: {error}"),
            Self::Pass(error) => write!(f, "DMO arena pass: {error}"),
        }
    }
}
