//! Real-corpus main-DMO resident/hull prefix probe.
//!
//! The resident-load prefix reaches raw state 8 with both subsystem
//! acknowledgements outstanding. An incremental host for the recovered
//! `cDMOBankManager::Update` boundary then advances the real-corpus Simulation
//! and Presentation registration effects and hull states through the exact
//! `SecondaryLoad01` check/update/raw-one-sleep cadence, stopping at raw state
//! 10. Its deterministic local producer/receiver subdivision is a probe policy,
//! not a native thread timeline. A separately named synthetic phase coordinates
//! physical unload with both reconstructed subsystem teardown routes; its local
//! scheduling is cleanup policy rather than a claimed native callback timeline.

use super::{DmoProbeRequestIdError, DmoProbeRequestIds, WORLD_STREAM_PROBE_TIMEOUT};
#[cfg(test)]
use skate2_core::collection_resident_streamer::ResidentCollectionStorage;
use skate2_core::{
    DmoBankResidentModelCatalog, MainDmoSimulationDeferredDispatch,
    MainDmoSimulationDeferredMarkerKind, MainDmoSimulationDeferredProducer,
    MainDmoSimulationDeferredProducerFailure, MainDmoSimulationDeferredProducerUpdate,
    MainDmoSimulationDeferredRegistrationDriver,
    catalog_resident_load_factory::{
        CatalogResidentCollectionLoadFactory, CatalogResidentLoadError,
    },
    collection_resident_streamer::{
        CollectionId, CollectionResidentStreamer, ResidentAssetGarbageCollector,
        ResidentCallbackContext, ResidentCallbackObservation, ResidentCollectionCallback,
        ResidentCollectionMetadata, ResidentCollectionRegistry, ResidentStreamerUpdate,
    },
    deferred_collection_reg_manager::{
        DeferredCollectionRegActive, DeferredCollectionRegActiveEffect,
        DeferredCollectionRegManager, DeferredCollectionRegManagerUpdate, DeferredCollectionRegRow,
        DeferredCollectionRegSetupEffect, DeferredCollectionRegType,
    },
    dmo_bank_processed_asset::{
        DMO_BANK_ASSET_COUNT, DMO_BANK_COLLECTION_ID, DMO_BANK_PARENT_ID,
        DmoBankProcessedArenaAsset, DmoBankProcessedArenaAssetBuilderSessions,
        DmoBankProcessedArenaBuildError, DmoBankProcessedArenaSessionError,
    },
    dmo_hull_acknowledgement::{
        MAIN_DMO_HULL_USERDATA_RAW, MainDmoHullAcknowledgementCoordinator,
        MainDmoHullDeregistrationRequest, MainDmoHullId, MainDmoHullRawState,
        MainDmoHullRegistrationRequest, MainDmoHullResidentCallback, MainDmoHullStream,
        MainDmoHullSubsystemPort, MainDmoHullUnloadOutcome,
        MainDmoPresentationDeregistrationCallback, MainDmoPresentationRegistrationCallback,
        MainDmoResidentHoldReleaser, MainDmoSimulationDeregistrationCallback,
        MainDmoSimulationRegistrationCallback,
    },
    game::{
        MainDmoBankLoadRuntime, run_game_load_main_dmo_barriers, run_secondary_load_main_dmo_bank,
    },
    owned_resident_collection::{
        OwnedResidentAssetsView, OwnedResidentCollection, OwnedResidentCollectionError,
        OwnedResidentCollectionStorage, ResidentAssetIdentity,
    },
    presentation_dmo_arena_adapter::{
        MainDmoBankPresentationArenaPrepareError, MainDmoBankPresentationDispatch,
        MainDmoBankPresentationRegistration, MainDmoBankPresentationRegistrationAdapter,
        PreparedMainDmoBankPresentationArena, PreparedMainDmoBankPresentationCollection,
        PresentationDmoDeferredCollectionRegistration, PresentationMaterialDataHandle,
    },
    presentation_dmo_deferred_producer::{
        MainDmoPresentationDeferredMarkerKind, MainDmoPresentationDeferredProducer,
        MainDmoPresentationDeferredProducerUpdate,
        MainDmoPresentationDeferredProducerUpdateFailure,
    },
    resident_collection_load_driver::CollectionDecompressionWake,
    simulation_collection_adapter::{
        SimulationCollectionDmoPass, SimulationCollectionLocationPass,
        SimulationCollectionNestedFailureKind, SimulationCollectionPassDeregistrationFailure,
    },
    simulation_deferred_batch_policy::DeferredCollectionBatchControls,
    simulation_dmo_arena_adapter::PreparedSimulationDmoArena,
    simulation_environment_unlock_router::EnvironmentUnlockRouterError,
    simulation_normal_arena_adapter::{
        SimulationNormalArenaRegistrationAdapter, SimulationNormalEnvironmentDispatch,
    },
    simulation_normal_arena_plan::SimulationNormalOperation,
    simulation_normal_arena_plan::SimulationRecordHandle,
    simulation_pointer_arena_adapter::SimulationPointerArenaDispatch,
    simulation_registry_arena_adapter::SimulationRegistryArenaDispatch,
    simulation_resident_registry::{
        SimulationResidentArenaLookup, SimulationResidentLookup, SimulationResidentRegistry,
        SimulationResidentRegistryError, SimulationResidentToken, SimulationResidentWrapper,
    },
    simulation_static_arena_adapter::SimulationStaticGrindDispatch,
    static_ground_pool::StaticEnvironmentSource,
    world_stream_catalog::{CollectionRegistrationSnapshot, WorldStreamCatalog},
};
use skate2_core::{grind::GrindAsset, location_arena_adapter::PreparedLocationArena};
use skate2_formats::mount::{MountError, MountSet};
use std::{
    cell::RefCell,
    convert::Infallible,
    error::Error,
    fmt,
    rc::Rc,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

type DmoGateCause = CatalogResidentLoadError<
    MountError,
    DmoProbeRequestIdError,
    DmoBankProcessedArenaSessionError,
    DmoBankProcessedArenaBuildError,
>;
type DmoGateError = Arc<DmoGateCause>;
type DmoGateStorage = OwnedResidentCollectionStorage<
    DmoBankProcessedArenaAsset,
    fn(OwnedResidentCollectionError) -> DmoGateError,
    DmoGateError,
>;
type DmoGateSimulationRegistry = SimulationResidentRegistry<
    DmoBankProcessedArenaAsset,
    fn(SimulationResidentRegistryError) -> DmoGateError,
    DmoGateError,
>;
type DmoGateFactory = CatalogResidentCollectionLoadFactory<
    MountSet,
    DmoProbeRequestIds,
    DmoBankProcessedArenaAssetBuilderSessions,
>;
type DmoGateStreamerRuntime = CollectionResidentStreamer<
    DmoGateStorage,
    DmoGateMetadata,
    DmoGateFactory,
    DmoGateRegistry,
    DmoGateGarbage,
    CollectionDecompressionWake,
    DmoGateCallback,
    DmoGateError,
>;

#[derive(Debug)]
struct DmoGateFailure(String);

impl fmt::Display for DmoGateFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for DmoGateFailure {}

fn gate_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(DmoGateFailure(message.into()))
}

fn external(error: impl Error + Send + Sync + 'static) -> DmoGateError {
    Arc::new(CatalogResidentLoadError::external(Arc::new(error)))
}

fn require(condition: bool, message: impl Into<String>) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(gate_error(message))
    }
}

struct DmoGateMetadata {
    catalog: WorldStreamCatalog,
}

impl ResidentCollectionMetadata<DmoGateError> for DmoGateMetadata {
    type Descriptor = CollectionRegistrationSnapshot;

    fn find_collection(
        &mut self,
        id: CollectionId,
    ) -> Result<Option<Self::Descriptor>, DmoGateError> {
        self.catalog.collection(id.0).map_err(external)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DmoGateEvent {
    WrapperCreated(CollectionId),
    WrapperPublished(CollectionId),
    RegisterSimulation(MainDmoHullRegistrationRequest),
    RegisterPresentation(MainDmoHullRegistrationRequest),
    DeregisterSimulation(MainDmoHullDeregistrationRequest),
    DeregisterPresentation(MainDmoHullDeregistrationRequest),
    WrapperRemoved(CollectionId),
}

struct DmoGateRegistryTelemetry {
    events: Rc<RefCell<Vec<DmoGateEvent>>>,
    asset_ids: Vec<CollectionId>,
    operation_count: usize,
    dmo_operation_count: usize,
    model_routes: Option<DmoBankResidentModelCatalog>,
    assets_view: Option<OwnedResidentAssetsView<DmoBankProcessedArenaAsset>>,
    prepared_presentation: Option<PreparedMainDmoBankPresentationCollection>,
    active: Option<(CollectionId, u64)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DmoPresentationGateEvent {
    MaterialAdd(u64),
    LivingWorldAdd(u64),
    DmoRegister { primary_guid: u64, local_guid: u64 },
    MaterialRemove(u64),
    LivingWorldRemove(u64),
    DmoDeregister { primary_guid: u64, local_guid: u64 },
}

struct DmoPresentationGateDispatch {
    events: Rc<RefCell<Vec<DmoPresentationGateEvent>>>,
}

impl MainDmoBankPresentationDispatch for DmoPresentationGateDispatch {
    fn add_material(&mut self, source: &PresentationMaterialDataHandle) {
        self.events
            .borrow_mut()
            .push(DmoPresentationGateEvent::MaterialAdd(
                source.diagnostic_guid(),
            ));
    }

    fn add_living_world_dmo_model(&mut self, asset_id: u64) {
        self.events
            .borrow_mut()
            .push(DmoPresentationGateEvent::LivingWorldAdd(asset_id));
    }

    fn register_presentation_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64) {
        let (primary_guid, local_guid) = dmo_source_ids(source);
        assert_eq!(primary_guid, asset_id, "validated DMO primary ID changed");
        self.events
            .borrow_mut()
            .push(DmoPresentationGateEvent::DmoRegister {
                primary_guid,
                local_guid,
            });
    }

    fn remove_living_world_dmo_model(&mut self, asset_id: u64) {
        self.events
            .borrow_mut()
            .push(DmoPresentationGateEvent::LivingWorldRemove(asset_id));
    }

    fn remove_material(&mut self, source: &PresentationMaterialDataHandle) {
        self.events
            .borrow_mut()
            .push(DmoPresentationGateEvent::MaterialRemove(
                source.diagnostic_guid(),
            ));
    }

    fn deregister_presentation_dmo_model(
        &mut self,
        source: &SimulationRecordHandle,
        asset_id: u64,
    ) {
        let (primary_guid, local_guid) = dmo_source_ids(source);
        assert_eq!(primary_guid, asset_id, "validated DMO primary ID changed");
        self.events
            .borrow_mut()
            .push(DmoPresentationGateEvent::DmoDeregister {
                primary_guid,
                local_guid,
            });
    }
}

fn dmo_source_ids(source: &SimulationRecordHandle) -> (u64, u64) {
    let bytes = source.bytes();
    (
        u64::from_be_bytes(
            bytes[96..104]
                .try_into()
                .expect("validated DMO primary GUID"),
        ),
        u64::from_be_bytes(
            bytes[104..112]
                .try_into()
                .expect("validated DMO local GUID"),
        ),
    )
}

type DmoPresentationGateAdapter =
    MainDmoBankPresentationRegistrationAdapter<DmoPresentationGateDispatch>;
const DMO_PRESENTATION_REGISTRATION_EVENT_SIGNATURE: u64 = 0x93ec_8ff6_b3ec_727b;
const DMO_PRESENTATION_LIFECYCLE_EVENT_SIGNATURE: u64 = 0xb142_8a65_1a6d_70a1;
const DMO_ONLY_REGISTRATION_EVENT_SIGNATURE: u64 = 0xebfd_ce16_a6a5_2bef;
const DMO_ONLY_LIFECYCLE_EVENT_SIGNATURE: u64 = 0x03d8_b9ef_5e47_9295;
const MAIN_DMO_MATERIAL_GUID: u64 = 0x8f51_812c_ef1b_09a4;
const DMO_RESIDENT_MODEL_ROUTE_SIGNATURE_V1: u64 = 0xa54d_a455_1bdf_2f49;

struct DmoDeferredGateRuntime {
    adapter: DmoPresentationGateAdapter,
    registration: Option<MainDmoBankPresentationRegistration>,
    registration_callback: Option<MainDmoPresentationRegistrationCallback>,
    events: Rc<RefCell<Vec<DmoPresentationGateEvent>>>,
    simulation_driver: Option<DmoSimulationDeferredRegistrationDriver>,
    simulation_events: Rc<RefCell<Vec<DmoSimulationGateEvent>>>,
}

type DmoPresentationDeferredProducer =
    MainDmoPresentationDeferredProducer<MainDmoPresentationRegistrationCallback>;
type DmoPresentationDeferredProducerFailure = MainDmoPresentationDeferredProducerUpdateFailure<
    MainDmoPresentationRegistrationCallback,
    Infallible,
>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DmoSimulationGateEvent {
    DmoModelAdd(u64),
    DmoModelRemove(u64),
    LocationRegister {
        ordinal: u32,
        container_count: usize,
    },
    DmoRegister {
        ordinal: u32,
        operation_count: usize,
    },
    DmoDeregister {
        ordinal: u32,
    },
    LocationDeregister {
        ordinal: u32,
    },
}

struct DmoSimulationGateDispatch {
    events: Rc<RefCell<Vec<DmoSimulationGateEvent>>>,
}

impl SimulationStaticGrindDispatch for DmoSimulationGateDispatch {
    fn add_spline_data(&mut self, _asset: &GrindAsset) {
        panic!("main DMO bank unexpectedly contained a grind registration")
    }

    fn remove_spline_data(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly reached grind deregistration")
    }
}

impl SimulationPointerArenaDispatch for DmoSimulationGateDispatch {
    fn add_dmo_model(&mut self, asset_id: u64) {
        self.events
            .borrow_mut()
            .push(DmoSimulationGateEvent::DmoModelAdd(asset_id));
    }

    fn register_dmo_model(&mut self, _source: &SimulationRecordHandle, _asset_id: u64) {
        panic!("exact main-bank model controller did not replace the legacy DMO callback")
    }

    fn remove_dmo_model(&mut self, asset_id: u64) {
        self.events
            .borrow_mut()
            .push(DmoSimulationGateEvent::DmoModelRemove(asset_id));
    }

    fn deregister_dmo_model(&mut self, _source: &SimulationRecordHandle, _asset_id: u64) {
        panic!("exact main-bank model controller did not replace the legacy DMO callback")
    }

    fn add_trigger_volume(&mut self, _source: &SimulationRecordHandle) {
        panic!("main DMO bank unexpectedly contained a trigger registration")
    }

    fn remove_trigger_volume(&mut self, _source: &SimulationRecordHandle) {
        panic!("main DMO bank unexpectedly reached trigger deregistration")
    }
}

impl SimulationRegistryArenaDispatch for DmoSimulationGateDispatch {
    fn register_streamed_tile(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly contained a streamed tile")
    }
    fn unregister_streamed_tile(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly reached streamed-tile deregistration")
    }
    fn add_nav_mesh_data(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly contained nav-mesh data")
    }
    fn remove_nav_mesh_data(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly reached nav-mesh deregistration")
    }
    fn add_rain_data(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly contained rain data")
    }
    fn remove_rain_data(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly reached rain-data deregistration")
    }
    fn add_ai_path_asset(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly contained AI path data")
    }
    fn remove_ai_path_asset(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly reached AI-path deregistration")
    }
    fn register_waypoint_data(&mut self, _asset_id: u64, _streamed: bool) {
        panic!("main DMO bank unexpectedly contained waypoint data")
    }
    fn deregister_waypoint_data(&mut self, _asset_id: u64, _streamed: bool) {
        panic!("main DMO bank unexpectedly reached waypoint deregistration")
    }
    fn add_spatial_map_asset(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly contained spatial-map data")
    }
    fn remove_spatial_map_asset(&mut self, _asset_id: u64) {
        panic!("main DMO bank unexpectedly reached spatial-map deregistration")
    }
}

struct DmoSimulationGateEnvironment;

impl SimulationNormalEnvironmentDispatch for DmoSimulationGateEnvironment {
    fn register_environment(
        &mut self,
        _source: &Arc<StaticEnvironmentSource>,
    ) -> Result<(), EnvironmentUnlockRouterError> {
        panic!("main DMO bank unexpectedly contained an Environment registration")
    }

    fn deregister_environment(&mut self, _source: &Arc<StaticEnvironmentSource>) {
        panic!("main DMO bank unexpectedly reached Environment deregistration")
    }
}

struct DmoSimulationGateLocation {
    events: Rc<RefCell<Vec<DmoSimulationGateEvent>>>,
    next_ordinal: u32,
}

impl SimulationCollectionLocationPass for DmoSimulationGateLocation {
    type Registration = u32;

    fn register_location(
        &mut self,
        prepared: &PreparedLocationArena,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
        let ordinal = self.next_ordinal;
        self.next_ordinal = self
            .next_ordinal
            .checked_add(1)
            .expect("the pinned 125-Arena collection fits u32");
        self.events
            .borrow_mut()
            .push(DmoSimulationGateEvent::LocationRegister {
                ordinal,
                container_count: prepared.container_count(),
            });
        Ok(ordinal)
    }

    fn deregister_location(
        &mut self,
        registration: Self::Registration,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
        self.events
            .borrow_mut()
            .push(DmoSimulationGateEvent::LocationDeregister {
                ordinal: registration,
            });
        Ok(())
    }
}

struct DmoSimulationGateDmo {
    events: Rc<RefCell<Vec<DmoSimulationGateEvent>>>,
    next_ordinal: u32,
}

impl SimulationCollectionDmoPass for DmoSimulationGateDmo {
    type Registration = u32;

    fn register_dmo(
        &mut self,
        prepared: &PreparedSimulationDmoArena,
        userdata_raw: u32,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
        assert_eq!(userdata_raw, MAIN_DMO_HULL_USERDATA_RAW);
        let ordinal = self.next_ordinal;
        self.next_ordinal = self
            .next_ordinal
            .checked_add(1)
            .expect("the pinned 125-Arena collection fits u32");
        self.events
            .borrow_mut()
            .push(DmoSimulationGateEvent::DmoRegister {
                ordinal,
                operation_count: prepared.operation_count(),
            });
        Ok(ordinal)
    }

    fn deregister_dmo(
        &mut self,
        registration: Self::Registration,
        userdata_raw: u32,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
        assert_eq!(userdata_raw, MAIN_DMO_HULL_USERDATA_RAW);
        self.events
            .borrow_mut()
            .push(DmoSimulationGateEvent::DmoDeregister {
                ordinal: registration,
            });
        Ok(())
    }
}

type DmoSimulationDeferredProducer = MainDmoSimulationDeferredProducer<
    DmoSimulationGateDispatch,
    DmoSimulationGateEnvironment,
    DmoSimulationGateLocation,
    DmoSimulationGateDmo,
>;
type DmoSimulationDeferredProducerFailure = MainDmoSimulationDeferredProducerFailure<
    DmoSimulationGateDispatch,
    DmoSimulationGateEnvironment,
    DmoSimulationGateLocation,
    DmoSimulationGateDmo,
    Infallible,
>;
type DmoSimulationDeferredRegistrationDriver = MainDmoSimulationDeferredRegistrationDriver<
    DmoSimulationGateDispatch,
    DmoSimulationGateEnvironment,
    DmoSimulationGateLocation,
    DmoSimulationGateDmo,
>;

enum DmoDeferredProducerState {
    SimulationProducer(DmoSimulationDeferredProducer),
    SimulationFailure(DmoSimulationDeferredProducerFailure),
    PresentationProducer(DmoPresentationDeferredProducer),
    PresentationFailure(DmoPresentationDeferredProducerFailure),
}

type DmoDeferredManager = DeferredCollectionRegManager<Option<DmoDeferredProducerState>>;
type DmoDeferredRow = DeferredCollectionRegRow<Option<DmoDeferredProducerState>>;

struct DmoGateWrapper {
    domain: Weak<()>,
    simulation: SimulationResidentWrapper<DmoBankProcessedArenaAsset>,
    token: u64,
}

struct DmoGateWrapperToken {
    domain: Weak<()>,
    simulation: SimulationResidentToken,
    telemetry: u64,
}

struct DmoGateRegistry {
    domain: Arc<()>,
    telemetry: Rc<RefCell<DmoGateRegistryTelemetry>>,
    simulation: DmoGateSimulationRegistry,
}

impl DmoGateRegistry {
    /// Builds one probe-only safe-host composite. `simulation` is the sole
    /// published-occurrence authority; the legacy telemetry `active` row and
    /// fixed token `1` remain diagnostics, not a second native registry or
    /// independently meaningful publication identity.
    fn new(
        telemetry: Rc<RefCell<DmoGateRegistryTelemetry>>,
    ) -> (Self, SimulationResidentLookup<DmoBankProcessedArenaAsset>) {
        let (simulation, lookup): (DmoGateSimulationRegistry, _) = SimulationResidentRegistry::new(
            map_simulation_registry_error as fn(SimulationResidentRegistryError) -> DmoGateError,
        );
        let domain = Arc::new(());
        (
            Self {
                domain,
                telemetry,
                simulation,
            },
            lookup,
        )
    }
}

impl ResidentCollectionRegistry<OwnedResidentCollection<DmoBankProcessedArenaAsset>, DmoGateError>
    for DmoGateRegistry
{
    type Wrapper = DmoGateWrapper;
    type Token = DmoGateWrapperToken;

    fn create_wrapper(
        &mut self,
        id: CollectionId,
        collection: &OwnedResidentCollection<DmoBankProcessedArenaAsset>,
    ) -> Result<Self::Wrapper, DmoGateError> {
        let assets_view = collection.resident_assets_view();
        let mut asset_ids = Vec::new();
        let mut operation_count = 0_usize;
        let mut dmo_operation_count = 0_usize;
        let mut presentation_arenas = Vec::new();
        let mut presentation_costs = Vec::new();
        assets_view
            .try_for_each(|asset| {
                asset_ids.push(asset.resident_asset_id());
                for operation in asset.plan().operations() {
                    operation_count += 1;
                    if matches!(operation, SimulationNormalOperation::DmoInstance { .. }) {
                        dmo_operation_count += 1;
                    }
                }
                let prepared = PreparedMainDmoBankPresentationArena::try_new(asset)?;
                presentation_costs.push(prepared.cost());
                presentation_arenas.push(prepared);
                Ok::<_, MainDmoBankPresentationArenaPrepareError>(())
            })
            .map_err(external)?;
        if let Some((ordinal, cost)) = presentation_costs
            .iter()
            .copied()
            .enumerate()
            .find(|(_, cost)| *cost != 0)
        {
            return Err(external(DmoGateFailure(format!(
                "main DMO Presentation Arena {ordinal} had processed cost {cost}, expected zero"
            ))));
        }
        let prepared_presentation =
            PreparedMainDmoBankPresentationCollection::try_from_prepared(&presentation_arenas)
                .map_err(external)?;
        let model_routes = DmoBankResidentModelCatalog::try_new(collection).map_err(external)?;

        let mut telemetry = self.telemetry.borrow_mut();
        if telemetry.assets_view.is_some()
            || telemetry.model_routes.is_some()
            || telemetry.active.is_some()
        {
            return Err(external(DmoGateFailure(
                "DMO gate wrapper was created more than once".to_owned(),
            )));
        }
        let simulation = self.simulation.create_wrapper(id, collection)?;
        telemetry.asset_ids = asset_ids;
        telemetry.operation_count = operation_count;
        telemetry.dmo_operation_count = dmo_operation_count;
        telemetry.model_routes = Some(model_routes);
        telemetry.assets_view = Some(assets_view);
        telemetry.prepared_presentation = Some(prepared_presentation);
        telemetry
            .events
            .borrow_mut()
            .push(DmoGateEvent::WrapperCreated(id));
        Ok(DmoGateWrapper {
            domain: Arc::downgrade(&self.domain),
            simulation,
            token: 1,
        })
    }

    fn stable_token(wrapper: &Self::Wrapper) -> Self::Token {
        assert!(
            wrapper.simulation.is_collection_attached(),
            "DMO gate wrapper detached early"
        );
        DmoGateWrapperToken {
            domain: Weak::clone(&wrapper.domain),
            simulation: DmoGateSimulationRegistry::stable_token(&wrapper.simulation),
            telemetry: wrapper.token,
        }
    }

    fn publish(&mut self, id: CollectionId, token: &Self::Token) -> Result<(), DmoGateError> {
        let mut telemetry = self.telemetry.borrow_mut();
        if telemetry.active.is_some() {
            return Err(external(DmoGateFailure(
                "DMO gate wrapper publication replaced an active identity".to_owned(),
            )));
        }
        if !Weak::ptr_eq(&token.domain, &Arc::downgrade(&self.domain)) {
            return Err(external(DmoGateFailure(
                "DMO gate wrapper publication token belongs to another registry".to_owned(),
            )));
        }
        self.simulation.publish(id, &token.simulation)?;
        telemetry.active = Some((id, token.telemetry));
        telemetry
            .events
            .borrow_mut()
            .push(DmoGateEvent::WrapperPublished(id));
        Ok(())
    }

    fn remove_exact(&mut self, id: CollectionId, token: &Self::Token) -> Result<(), DmoGateError> {
        let mut telemetry = self.telemetry.borrow_mut();
        if telemetry.active != Some((id, token.telemetry)) {
            return Err(external(DmoGateFailure(
                "DMO gate wrapper removal identity differed".to_owned(),
            )));
        }
        if !Weak::ptr_eq(&token.domain, &Arc::downgrade(&self.domain)) {
            return Err(external(DmoGateFailure(
                "DMO gate wrapper removal token belongs to another registry".to_owned(),
            )));
        }
        self.simulation.remove_exact(id, &token.simulation)?;
        telemetry.active = None;
        telemetry
            .events
            .borrow_mut()
            .push(DmoGateEvent::WrapperRemoved(id));
        Ok(())
    }
}

struct DmoGateGarbage {
    asset_ids: Rc<RefCell<Vec<CollectionId>>>,
}

impl ResidentAssetGarbageCollector<DmoGateError> for DmoGateGarbage {
    fn garbage_collect(&mut self, id: CollectionId) -> Result<(), DmoGateError> {
        self.asset_ids.borrow_mut().push(id);
        Ok(())
    }
}

#[derive(Default)]
struct DmoGatePortState {
    simulation_registration: Option<MainDmoSimulationRegistrationCallback>,
    presentation_registration: Option<MainDmoPresentationRegistrationCallback>,
    simulation_deregistration: Option<MainDmoSimulationDeregistrationCallback>,
    presentation_deregistration: Option<MainDmoPresentationDeregistrationCallback>,
}

struct DmoGatePort {
    state: Rc<RefCell<DmoGatePortState>>,
    events: Rc<RefCell<Vec<DmoGateEvent>>>,
}

impl MainDmoHullSubsystemPort for DmoGatePort {
    fn register_simulation(
        &mut self,
        request: MainDmoHullRegistrationRequest,
        callback: MainDmoSimulationRegistrationCallback,
    ) {
        self.events
            .borrow_mut()
            .push(DmoGateEvent::RegisterSimulation(request));
        self.state.borrow_mut().simulation_registration = Some(callback);
    }

    fn register_presentation(
        &mut self,
        request: MainDmoHullRegistrationRequest,
        callback: MainDmoPresentationRegistrationCallback,
    ) {
        self.events
            .borrow_mut()
            .push(DmoGateEvent::RegisterPresentation(request));
        self.state.borrow_mut().presentation_registration = Some(callback);
    }

    fn deregister_simulation(
        &mut self,
        request: MainDmoHullDeregistrationRequest,
        callback: MainDmoSimulationDeregistrationCallback,
    ) {
        self.events
            .borrow_mut()
            .push(DmoGateEvent::DeregisterSimulation(request));
        self.state.borrow_mut().simulation_deregistration = Some(callback);
    }

    fn deregister_presentation(
        &mut self,
        request: MainDmoHullDeregistrationRequest,
        callback: MainDmoPresentationDeregistrationCallback,
    ) {
        self.events
            .borrow_mut()
            .push(DmoGateEvent::DeregisterPresentation(request));
        self.state.borrow_mut().presentation_deregistration = Some(callback);
    }
}

struct DmoGateCallback(MainDmoHullResidentCallback<DmoGatePort>);

impl ResidentCollectionCallback<DmoGateError> for DmoGateCallback {
    fn on_complete(
        &mut self,
        id: CollectionId,
        observation: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, Self>,
    ) -> Result<(), DmoGateError> {
        self.0.dispatch_mapped(id, observation, context, external)
    }
}

#[derive(Debug, Default)]
struct DmoGateDriverTotals {
    cache_hits: u32,
    unloads_processed: u32,
    physical_loads_started: u32,
    physical_loads_completed: u32,
    decompression_kicks: u32,
}

impl DmoGateDriverTotals {
    fn add(&mut self, update: ResidentStreamerUpdate) {
        self.cache_hits = self.cache_hits.wrapping_add(update.cache_hits);
        self.unloads_processed = self
            .unloads_processed
            .wrapping_add(update.unloads_processed);
        self.physical_loads_started = self
            .physical_loads_started
            .wrapping_add(update.physical_loads_started);
        self.physical_loads_completed = self
            .physical_loads_completed
            .wrapping_add(update.physical_loads_completed);
        self.decompression_kicks = self
            .decompression_kicks
            .wrapping_add(update.decompression_kicks);
    }
}

fn map_storage_error(error: OwnedResidentCollectionError) -> DmoGateError {
    external(error)
}

fn map_simulation_registry_error(error: SimulationResidentRegistryError) -> DmoGateError {
    external(error)
}

fn expected_registration_request() -> MainDmoHullRegistrationRequest {
    MainDmoHullRegistrationRequest {
        collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
        userdata_raw: MAIN_DMO_HULL_USERDATA_RAW,
        force_immediate: false,
    }
}

fn expected_deregistration_request() -> MainDmoHullDeregistrationRequest {
    MainDmoHullDeregistrationRequest {
        collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
        userdata_raw: MAIN_DMO_HULL_USERDATA_RAW,
    }
}

fn drive_to_idle<S>(streamer: &mut S) -> Result<DmoGateDriverTotals, Box<dyn Error>>
where
    S: DmoGateStreamer,
{
    let deadline = Instant::now() + WORLD_STREAM_PROBE_TIMEOUT;
    let mut totals = DmoGateDriverTotals::default();
    while streamer.gate_pending_requests() {
        if Instant::now() >= deadline {
            return Err(gate_error("DMO hull resident streamer timed out"));
        }
        totals.add(streamer.gate_update()?);
        thread::yield_now();
    }
    Ok(totals)
}

trait DmoGateStreamer {
    fn gate_pending_requests(&self) -> bool;
    fn gate_update(&mut self) -> Result<ResidentStreamerUpdate, Box<dyn Error>>;
}

impl<S, M, F, R, G, K> DmoGateStreamer
    for CollectionResidentStreamer<S, M, F, R, G, K, DmoGateCallback, DmoGateError>
where
    S: skate2_core::collection_resident_streamer::ResidentCollectionStorage<DmoGateError>,
    M: ResidentCollectionMetadata<DmoGateError>,
    F: skate2_core::collection_resident_streamer::ResidentCollectionLoadFactory<
            S::Writer,
            M::Descriptor,
            DmoGateError,
        >,
    R: ResidentCollectionRegistry<S::Collection, DmoGateError>,
    G: ResidentAssetGarbageCollector<DmoGateError>,
    K: skate2_core::collection_resident_streamer::ResidentDecompressionKick<DmoGateError>,
{
    fn gate_pending_requests(&self) -> bool {
        self.pending_requests()
    }

    fn gate_update(&mut self) -> Result<ResidentStreamerUpdate, Box<dyn Error>> {
        self.update()
            .map_err(|error| Box::new(error) as Box<dyn Error>)
    }
}

/// Mounted manifest collaborator retained by the application-level main-bank
/// owner. Construction may allocate host workers, but `begin_load` is the sole
/// operation authorized to initiate the DMO manifest state machine.
pub(super) trait DmoManifestLoadSession {
    fn catalog(&self) -> &WorldStreamCatalog;
    fn begin_load(&mut self) -> Result<(), Box<dyn Error>>;
    fn is_loaded(&self) -> bool;
    fn update_load(&mut self) -> Result<(), Box<dyn Error>>;
    fn loaded_collection(&mut self) -> Result<CollectionRegistrationSnapshot, Box<dyn Error>>;
    fn unload(&mut self) -> Result<(), Box<dyn Error>>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DmoMainBankLoadPhase {
    Dormant,
    ManifestLoading,
    PrepareResidentLoad,
    ResidentLoading,
    VerifyResidentPrefix,
    DeferredRegistration,
    FinalizeDeferredRegistration,
    AcknowledgePresentation,
    AdvanceHull,
}

/// Probe host for the recovered `cDMOBankManager::Update` boundary.
///
/// One call advances one resident-streamer update, one deferred-registration
/// step, one callback acknowledgement, or one hull state. That deterministic
/// subdivision is host scheduling around the recovered state machines, not a
/// claim about native worker/postman interleaving.
struct DmoMainBankGateRuntime<'a, M> {
    phase: DmoMainBankLoadPhase,
    manifest: &'a mut M,
    coordinator: &'a MainDmoHullAcknowledgementCoordinator<DmoGatePort>,
    callback: Option<MainDmoHullResidentCallback<DmoGatePort>>,
    streamer: &'a mut DmoGateStreamerRuntime,
    port_state: Rc<RefCell<DmoGatePortState>>,
    events: Rc<RefCell<Vec<DmoGateEvent>>>,
    registry: Rc<RefCell<DmoGateRegistryTelemetry>>,
    simulation_lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    garbage: Rc<RefCell<Vec<CollectionId>>>,
    catalog: &'a WorldStreamCatalog,
    collection: Option<CollectionRegistrationSnapshot>,
    request_live: Arc<AtomicBool>,
    load_totals: DmoGateDriverTotals,
    registration_session: Option<DmoRegistrationGateSession>,
    deferred_runtime: Option<DmoDeferredGateRuntime>,
    deadline: Option<Instant>,
}

impl<'a, M> DmoMainBankGateRuntime<'a, M>
where
    M: DmoManifestLoadSession,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        manifest: &'a mut M,
        coordinator: &'a MainDmoHullAcknowledgementCoordinator<DmoGatePort>,
        callback: MainDmoHullResidentCallback<DmoGatePort>,
        streamer: &'a mut DmoGateStreamerRuntime,
        port_state: Rc<RefCell<DmoGatePortState>>,
        events: Rc<RefCell<Vec<DmoGateEvent>>>,
        registry: Rc<RefCell<DmoGateRegistryTelemetry>>,
        simulation_lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
        garbage: Rc<RefCell<Vec<CollectionId>>>,
        catalog: &'a WorldStreamCatalog,
        request_live: Arc<AtomicBool>,
    ) -> Self {
        Self {
            phase: DmoMainBankLoadPhase::Dormant,
            manifest,
            coordinator,
            callback: Some(callback),
            streamer,
            port_state,
            events,
            registry,
            simulation_lookup,
            garbage,
            catalog,
            collection: None,
            request_live,
            load_totals: DmoGateDriverTotals::default(),
            registration_session: None,
            deferred_runtime: None,
            deadline: None,
        }
    }

    fn check_deadline(&self) -> Result<(), Box<dyn Error>> {
        require(
            self.deadline
                .is_some_and(|deadline| Instant::now() < deadline),
            "main DMO bank load runtime timed out",
        )
    }

    fn verify_and_begin_registration(&mut self) -> Result<(), Box<dyn Error>> {
        let collection = self
            .collection
            .as_ref()
            .ok_or_else(|| gate_error("DMO resident load lacks its manifest collection"))?;
        verify_evidence_backed_prefix(
            self.coordinator,
            self.streamer,
            &self.port_state,
            &self.events,
            &self.registry,
            &self.simulation_lookup,
            &self.garbage,
            self.catalog,
            collection,
            &self.request_live,
            &self.load_totals,
        )?;
        let (simulation_callback, presentation_callback) =
            take_registration_callbacks(&self.port_state)?;
        self.registration_session = Some(begin_dmo_registration_gate(
            &self.registry,
            &self.simulation_lookup,
            simulation_callback,
            presentation_callback,
            collection.size,
        )?);
        self.phase = DmoMainBankLoadPhase::DeferredRegistration;
        Ok(())
    }

    fn finalize_registration(&mut self) -> Result<(), Box<dyn Error>> {
        let session = self
            .registration_session
            .take()
            .ok_or_else(|| gate_error("DMO registration session was absent"))?;
        self.deferred_runtime = Some(session.finish(&self.registry)?);
        self.phase = DmoMainBankLoadPhase::AcknowledgePresentation;
        Ok(())
    }

    fn acknowledge_presentation(&mut self) -> Result<(), Box<dyn Error>> {
        let deferred = self
            .deferred_runtime
            .as_mut()
            .ok_or_else(|| gate_error("DMO deferred runtime was absent"))?;
        let callback = deferred
            .registration_callback
            .take()
            .ok_or_else(|| gate_error("Presentation FIFO callback was already consumed"))?;
        self.coordinator
            .acknowledge_presentation_registration(callback)
            .map_err(|error| gate_error(error.to_string()))?;
        self.phase = DmoMainBankLoadPhase::AdvanceHull;
        Ok(())
    }

    fn take_completed_runtime(
        &mut self,
    ) -> Result<(DmoDeferredGateRuntime, CollectionRegistrationSnapshot), Box<dyn Error>> {
        require(
            self.phase == DmoMainBankLoadPhase::AdvanceHull
                && self.coordinator.observation()?.state
                    == Some(MainDmoHullRawState::LoadCompleteNotify),
            "SecondaryLoad01 DMO wait did not stop at raw state 10",
        )?;
        let deferred = self
            .deferred_runtime
            .take()
            .ok_or_else(|| gate_error("completed DMO deferred runtime was absent"))?;
        let collection = self
            .collection
            .take()
            .ok_or_else(|| gate_error("completed DMO manifest collection was absent"))?;
        Ok((deferred, collection))
    }
}

impl<M> MainDmoBankLoadRuntime for DmoMainBankGateRuntime<'_, M>
where
    M: DmoManifestLoadSession,
{
    type Error = Box<dyn Error>;

    fn load_main_dmo_bank(&mut self) -> Result<(), Self::Error> {
        require(
            self.phase == DmoMainBankLoadPhase::Dormant,
            "main DMO bank load request was issued more than once",
        )?;
        self.manifest.begin_load()?;
        self.deadline = Some(Instant::now() + WORLD_STREAM_PROBE_TIMEOUT);
        self.phase = DmoMainBankLoadPhase::ManifestLoading;
        Ok(())
    }

    fn is_main_dmo_bank_loaded(&mut self) -> Result<bool, Self::Error> {
        self.coordinator
            .is_loaded()
            .map_err(|error| gate_error(error.to_string()))
    }

    fn update_main_dmo_bank(&mut self) -> Result<(), Self::Error> {
        self.check_deadline()?;
        match self.phase {
            DmoMainBankLoadPhase::Dormant => Err(gate_error(
                "main DMO bank runtime was updated before its load request",
            )),
            DmoMainBankLoadPhase::ManifestLoading => {
                self.manifest.update_load()?;
                if self.manifest.is_loaded() {
                    self.phase = DmoMainBankLoadPhase::PrepareResidentLoad;
                }
                Ok(())
            }
            DmoMainBankLoadPhase::PrepareResidentLoad => {
                self.collection = Some(self.manifest.loaded_collection()?);
                let callback = self
                    .callback
                    .take()
                    .ok_or_else(|| gate_error("main DMO resident callback was absent"))?;
                self.streamer.enqueue_load(
                    CollectionId(DMO_BANK_COLLECTION_ID),
                    i32::from_ne_bytes(DMO_BANK_ASSET_COUNT.to_ne_bytes()),
                    Some(DmoGateCallback(callback)),
                );
                self.phase = DmoMainBankLoadPhase::ResidentLoading;
                Ok(())
            }
            DmoMainBankLoadPhase::ResidentLoading => {
                self.load_totals.add(
                    self.streamer
                        .update()
                        .map_err(|error| Box::new(error) as Box<dyn Error>)?,
                );
                if !self.streamer.pending_requests() {
                    self.phase = DmoMainBankLoadPhase::VerifyResidentPrefix;
                }
                Ok(())
            }
            DmoMainBankLoadPhase::VerifyResidentPrefix => self.verify_and_begin_registration(),
            DmoMainBankLoadPhase::DeferredRegistration => {
                let session = self
                    .registration_session
                    .as_mut()
                    .ok_or_else(|| gate_error("DMO registration session was absent"))?;
                session.update(self.coordinator)?;
                if session.is_complete() {
                    self.phase = DmoMainBankLoadPhase::FinalizeDeferredRegistration;
                }
                Ok(())
            }
            DmoMainBankLoadPhase::FinalizeDeferredRegistration => self.finalize_registration(),
            DmoMainBankLoadPhase::AcknowledgePresentation => self.acknowledge_presentation(),
            DmoMainBankLoadPhase::AdvanceHull => {
                self.coordinator.update()?;
                Ok(())
            }
        }
    }
}

fn create_dmo_gate_streamer(
    mounts: Arc<MountSet>,
    catalog: &WorldStreamCatalog,
    request_live: &Arc<AtomicBool>,
    registry: &Rc<RefCell<DmoGateRegistryTelemetry>>,
    garbage: &Rc<RefCell<Vec<CollectionId>>>,
) -> (
    DmoGateStreamerRuntime,
    SimulationResidentLookup<DmoBankProcessedArenaAsset>,
) {
    let factory = CatalogResidentCollectionLoadFactory::new(
        mounts,
        catalog.clone(),
        DmoProbeRequestIds {
            live: Arc::clone(request_live),
        },
        DmoBankProcessedArenaAssetBuilderSessions::new(catalog.clone()),
    );
    let wake = factory.decompression_kick();
    let storage = DmoGateStorage::new(map_storage_error);
    let (gate_registry, simulation_lookup) = DmoGateRegistry::new(Rc::clone(registry));
    let streamer = CollectionResidentStreamer::new(
        storage,
        DmoGateMetadata {
            catalog: catalog.clone(),
        },
        factory,
        gate_registry,
        DmoGateGarbage {
            asset_ids: Rc::clone(garbage),
        },
        wake,
    );
    (streamer, simulation_lookup)
}

/// Runs the real prefix, then a clearly non-native mechanical cleanup.
pub(super) fn run<M>(mounts: Arc<MountSet>, manifest: &mut M) -> Result<(), Box<dyn Error>>
where
    M: DmoManifestLoadSession,
{
    let catalog = manifest.catalog().clone();
    let request_live = Arc::new(AtomicBool::new(false));
    let events = Rc::new(RefCell::new(Vec::new()));
    let registry = Rc::new(RefCell::new(DmoGateRegistryTelemetry {
        events: Rc::clone(&events),
        asset_ids: Vec::new(),
        operation_count: 0,
        dmo_operation_count: 0,
        model_routes: None,
        assets_view: None,
        active: None,
        prepared_presentation: None,
    }));
    let garbage = Rc::new(RefCell::new(Vec::new()));
    let port_state = Rc::new(RefCell::new(DmoGatePortState::default()));
    let (coordinator, callback) = MainDmoHullAcknowledgementCoordinator::new(
        DmoGatePort {
            state: Rc::clone(&port_state),
            events: Rc::clone(&events),
        },
        MainDmoHullId::MAIN_BANK,
    );

    let (mut streamer, simulation_lookup) =
        create_dmo_gate_streamer(mounts, &catalog, &request_live, &registry, &garbage);

    let mut main_bank_runtime = DmoMainBankGateRuntime::new(
        manifest,
        &coordinator,
        callback,
        &mut streamer,
        Rc::clone(&port_state),
        Rc::clone(&events),
        Rc::clone(&registry),
        simulation_lookup.clone(),
        Rc::clone(&garbage),
        &catalog,
        Arc::clone(&request_live),
    );
    let mut retry_raws = Vec::new();
    let mut secondary_load_byte = 0;
    run_game_load_main_dmo_barriers(
        &mut secondary_load_byte,
        &mut main_bank_runtime,
        |runtime, sleep, stored_byte| {
            require(
                stored_byte == 1,
                "GameLoad did not store one before its SecondaryLoad group",
            )?;
            run_secondary_load_main_dmo_bank(runtime, sleep)
        },
        |raw| {
            retry_raws.push(raw);
            thread::sleep(Duration::from_millis(u64::from(raw)));
        },
    )?;
    require(
        secondary_load_byte == 1
            && !retry_raws.is_empty()
            && retry_raws.iter().all(|raw| *raw == 1),
        "SecondaryLoad01 DMO wait did not sleep with raw value one after every pending update",
    )?;
    let (deferred_runtime, collection) = main_bank_runtime.take_completed_runtime()?;
    drop(main_bank_runtime);

    synthetic_mechanical_cleanup_without_subsystem_effects(
        &coordinator,
        &mut streamer,
        &port_state,
        &events,
        &registry,
        &simulation_lookup,
        &garbage,
        &collection,
        deferred_runtime,
        || manifest.unload(),
    )?;
    println!(
        "main DMO bank load: SecondaryLoad01 issued one request and reached raw state 10 through {} checked Update/raw-one-sleep iterations; synthetic work begins only after the recovered wait",
        retry_raws.len()
    );
    println!(
        "main DMO resident model routes: 125 assets / 93 unique local-GUID routes / V1 signature {DMO_RESIDENT_MODEL_ROUTE_SIGNATURE_V1:016x}; weak catalog expired after teardown"
    );
    println!(
        "main DMO deferred routes: shared type-1 Simulation -> type-2 Presentation manager handoff / both batches 10 x 12 + 5 / Simulation 125 Arenas + Finish + callback and DRegCollection + callback dispatched through the sealed real-corpus receiver / Presentation 125 refcount-first Arena lookups / completed type-2 row retained / Simulation 343 registration + 343 deregistration events / Presentation 187 + 187"
    );
    Ok(())
}

struct DmoRegistrationGateSession {
    load: DmoRegistrationLoadSession,
    events: Rc<RefCell<Vec<DmoPresentationGateEvent>>>,
    simulation_events: Rc<RefCell<Vec<DmoSimulationGateEvent>>>,
}

impl DmoRegistrationGateSession {
    fn update(
        &mut self,
        coordinator: &MainDmoHullAcknowledgementCoordinator<DmoGatePort>,
    ) -> Result<(), Box<dyn Error>> {
        self.load.update(coordinator)
    }

    fn is_complete(&self) -> bool {
        self.load.is_complete()
    }

    fn finish(
        self,
        registry: &Rc<RefCell<DmoGateRegistryTelemetry>>,
    ) -> Result<DmoDeferredGateRuntime, Box<dyn Error>> {
        let (adapter, registration, registration_callback, simulation_driver) =
            self.load.into_parts()?;
        verify_simulation_registration_events(registry, &self.simulation_events.borrow())?;
        require(
            simulation_driver.registration_acknowledged(),
            "DMO Simulation sealed driver did not retain deregistration-begin authority",
        )?;
        let asset_ids = registry.borrow().asset_ids.clone();
        require(
            asset_ids
                .iter()
                .all(|id| adapter.arena_ref_count(*id) == Some(1)),
            "DMO Presentation first-use counts differed",
        )?;
        require(
            adapter.live_collection_count() == 1
                && adapter.first_use_owner_count() == 125
                && adapter.orphaned_owner_count() == 0,
            "DMO Presentation registration ownership differed",
        )?;
        verify_presentation_registration_events(&self.events.borrow())?;
        Ok(DmoDeferredGateRuntime {
            adapter,
            registration: Some(registration),
            registration_callback: Some(registration_callback),
            events: self.events,
            simulation_driver: Some(simulation_driver),
            simulation_events: self.simulation_events,
        })
    }
}

fn begin_dmo_registration_gate(
    registry: &Rc<RefCell<DmoGateRegistryTelemetry>>,
    simulation_lookup: &SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    simulation_callback: MainDmoSimulationRegistrationCallback,
    presentation_callback: MainDmoPresentationRegistrationCallback,
    stream_size: u32,
) -> Result<DmoRegistrationGateSession, Box<dyn Error>> {
    let prepared = registry
        .borrow_mut()
        .prepared_presentation
        .take()
        .ok_or_else(|| gate_error("DMO resident did not retain its Presentation preparation"))?;
    require(
        prepared.arena_count() == 125
            && prepared.operation_count() == 94
            && prepared.material_operation_count() == 1
            && prepared.dmo_operation_count() == 93,
        format!(
            "DMO Presentation preparation differed: {} arenas, {} operations",
            prepared.arena_count(),
            prepared.operation_count()
        ),
    )?;
    let events = Rc::new(RefCell::new(Vec::new()));
    let adapter = MainDmoBankPresentationRegistrationAdapter::new(DmoPresentationGateDispatch {
        events: Rc::clone(&events),
    });
    let simulation_events = Rc::new(RefCell::new(Vec::new()));
    let simulation_producer = MainDmoSimulationDeferredProducer::new(
        simulation_lookup.clone(),
        SimulationNormalArenaRegistrationAdapter::with_environment(
            DmoSimulationGateDispatch {
                events: Rc::clone(&simulation_events),
            },
            DmoSimulationGateEnvironment,
        ),
        DmoSimulationGateLocation {
            events: Rc::clone(&simulation_events),
            next_ordinal: 0,
        },
        DmoSimulationGateDmo {
            events: Rc::clone(&simulation_events),
            next_ordinal: 0,
        },
        simulation_callback,
    );
    let load = DmoRegistrationLoadSession::new(
        simulation_producer,
        &prepared,
        adapter,
        simulation_lookup,
        presentation_callback,
        stream_size,
    )?;
    Ok(DmoRegistrationGateSession {
        load,
        events,
        simulation_events,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DmoRegistrationLoadPhase {
    SimulationProduction,
    PresentationProduction,
    SimulationReception,
    PresentationTerminal,
    Complete,
}

/// Probe-owned incremental worker composition for the two recovered deferred
/// subsystem producers.
///
/// Each `update` advances one matching manager batch, one Simulation FIFO
/// front, or the Presentation terminal pair. This is an explicit deterministic
/// host schedule around exact local state machines; it is not a claim about
/// native cross-thread/postman timing.
struct DmoRegistrationLoadSession {
    phase: DmoRegistrationLoadPhase,
    manager: DmoDeferredManager,
    adapter: DmoPresentationGateAdapter,
    deferred: Option<PresentationDmoDeferredCollectionRegistration>,
    arena_lookup: SimulationResidentArenaLookup<DmoBankProcessedArenaAsset>,
    stream_size: u32,
    simulation_batches: Vec<u32>,
    presentation_batches: Vec<u32>,
    retired_simulation: Option<DmoDeferredRow>,
    simulation_driver: Option<DmoSimulationDeferredRegistrationDriver>,
    registration: Option<MainDmoBankPresentationRegistration>,
    registration_callback: Option<MainDmoPresentationRegistrationCallback>,
}

impl DmoRegistrationLoadSession {
    fn new(
        simulation_producer: DmoSimulationDeferredProducer,
        prepared: &PreparedMainDmoBankPresentationCollection,
        mut adapter: DmoPresentationGateAdapter,
        simulation_lookup: &SimulationResidentLookup<DmoBankProcessedArenaAsset>,
        presentation_callback: MainDmoPresentationRegistrationCallback,
        stream_size: u32,
    ) -> Result<Self, Box<dyn Error>> {
        let presentation_producer = MainDmoPresentationDeferredProducer::new(
            prepared.clone(),
            CollectionId(DMO_BANK_PARENT_ID),
            presentation_callback,
        );
        let deferred = adapter.begin_deferred_collection(prepared)?;
        let arena_lookup = simulation_lookup.arena_lookup();
        let manager =
            setup_dmo_deferred_manager(simulation_producer, presentation_producer, stream_size)?;
        Ok(Self {
            phase: DmoRegistrationLoadPhase::SimulationProduction,
            manager,
            adapter,
            deferred: Some(deferred),
            arena_lookup,
            stream_size,
            simulation_batches: Vec::new(),
            presentation_batches: Vec::new(),
            retired_simulation: None,
            simulation_driver: None,
            registration: None,
            registration_callback: None,
        })
    }

    fn is_complete(&self) -> bool {
        self.phase == DmoRegistrationLoadPhase::Complete
    }

    fn update(
        &mut self,
        coordinator: &MainDmoHullAcknowledgementCoordinator<DmoGatePort>,
    ) -> Result<(), Box<dyn Error>> {
        match self.phase {
            DmoRegistrationLoadPhase::SimulationProduction => {
                self.update_simulation_production()?;
            }
            DmoRegistrationLoadPhase::PresentationProduction => {
                self.update_presentation_production()?;
            }
            DmoRegistrationLoadPhase::SimulationReception => {
                self.update_simulation_reception(coordinator)?;
            }
            DmoRegistrationLoadPhase::PresentationTerminal => {
                self.finish_presentation_terminal()?;
            }
            DmoRegistrationLoadPhase::Complete => {}
        }
        Ok(())
    }

    fn update_simulation_production(&mut self) -> Result<(), Box<dyn Error>> {
        let mut produced_update = None;
        let manager_update = self
            .manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |active| {
                    let (count, update) = update_simulation_deferred_active(active)?;
                    produced_update = Some(update);
                    Ok(count)
                },
                |row| setup_dmo_deferred_row(row, self.stream_size),
            )
            .map_err(|error| gate_error(error.to_string()))?;
        let update = produced_update
            .ok_or_else(|| gate_error("Simulation manager skipped its matching active row"))?;
        let (count, complete) = match update {
            MainDmoSimulationDeferredProducerUpdate::ArenasQueued { count } => (count, false),
            MainDmoSimulationDeferredProducerUpdate::TerminalQueued { arena_count } => {
                (arena_count, true)
            }
            MainDmoSimulationDeferredProducerUpdate::SetupCompleted
            | MainDmoSimulationDeferredProducerUpdate::TerminalIdle => {
                return Err(gate_error(format!(
                    "DMO Simulation deferred producer repeated stage {update:?}"
                )));
            }
        };
        self.simulation_batches.push(count);
        if complete {
            let retired = match manager_update {
                DeferredCollectionRegManagerUpdate::Entered {
                    active:
                        DeferredCollectionRegActiveEffect::Advanced {
                            registration_type: DeferredCollectionRegType::Simulation,
                            count: actual_count,
                            reached_maximum: true,
                        },
                    setup:
                        DeferredCollectionRegSetupEffect::Activated {
                            registration_type: DeferredCollectionRegType::Presentation,
                            maximum: DMO_BANK_ASSET_COUNT,
                        },
                    retired: Some(retired),
                } if actual_count == count => retired,
                _ => {
                    return Err(gate_error(
                        "DMO mixed manager Simulation-to-Presentation handoff differed",
                    ));
                }
            };
            require(
                self.manager.pending_count() == 0
                    && self.manager.active_snapshot().is_some_and(|active| {
                        active.registration_type == DeferredCollectionRegType::Presentation
                            && active.index == 0
                            && active.maximum == DMO_BANK_ASSET_COUNT
                    }),
                "DMO mixed manager did not activate an untouched Presentation row",
            )?;
            self.retired_simulation = Some(retired);
            self.phase = DmoRegistrationLoadPhase::PresentationProduction;
        } else {
            require(
                matches!(
                    manager_update,
                    DeferredCollectionRegManagerUpdate::Entered {
                        active: DeferredCollectionRegActiveEffect::Advanced {
                            registration_type: DeferredCollectionRegType::Simulation,
                            count: actual_count,
                            reached_maximum: false,
                        },
                        setup: DeferredCollectionRegSetupEffect::ActiveIncomplete,
                        retired: None,
                    } if actual_count == count
                ),
                "DMO Simulation manager progress differed from its producer",
            )?;
        }
        Ok(())
    }

    fn update_presentation_production(&mut self) -> Result<(), Box<dyn Error>> {
        let mut produced_update = None;
        let manager_update = self
            .manager
            .try_update(
                DeferredCollectionRegType::Presentation,
                |active| {
                    let (count, update) = update_presentation_deferred_active(active)?;
                    produced_update = Some(update);
                    Ok(count)
                },
                |_| -> Result<u32, Box<dyn Error>> {
                    Err(gate_error(
                        "Presentation manager unexpectedly selected a second pending row",
                    ))
                },
            )
            .map_err(|error| gate_error(error.to_string()))?;
        let update = produced_update
            .ok_or_else(|| gate_error("Presentation manager skipped its matching active row"))?;
        let (count, complete, proxy_collection_queued) = match update {
            MainDmoPresentationDeferredProducerUpdate::ArenasQueued { count } => {
                (count, false, false)
            }
            MainDmoPresentationDeferredProducerUpdate::TerminalQueued {
                arena_count,
                proxy_collection_queued,
            } => (arena_count, true, proxy_collection_queued),
            MainDmoPresentationDeferredProducerUpdate::SetupCompleted
            | MainDmoPresentationDeferredProducerUpdate::TerminalIdle => {
                return Err(gate_error(format!(
                    "DMO Presentation deferred producer repeated stage {update:?}"
                )));
            }
        };
        require(
            matches!(
                manager_update,
                DeferredCollectionRegManagerUpdate::Entered {
                    active: DeferredCollectionRegActiveEffect::Advanced {
                        registration_type: DeferredCollectionRegType::Presentation,
                        count: actual_count,
                        reached_maximum,
                    },
                    retired: None,
                    ..
                } if actual_count == count && reached_maximum == complete
            ),
            "DMO Presentation manager progress differed from its producer",
        )?;
        let state = self
            .manager
            .active_payload_mut()
            .and_then(Option::as_mut)
            .ok_or_else(|| gate_error("Presentation manager lost its active producer"))?;
        let DmoDeferredProducerState::PresentationProducer(producer) = state else {
            return Err(gate_error(
                "Presentation manager retained a failure after successful update",
            ));
        };
        let deferred = self
            .deferred
            .as_ref()
            .ok_or_else(|| gate_error("Presentation deferred authority was already consumed"))?;
        drain_presentation_arena_fronts(producer, &mut self.adapter, deferred, &self.arena_lookup)?;
        self.presentation_batches.push(count);
        if complete {
            require(
                !proxy_collection_queued,
                "main DMO invalid parent unexpectedly queued a proxy deregistration",
            )?;
            let simulation_state = self
                .retired_simulation
                .take()
                .ok_or_else(|| gate_error("retired Simulation manager row was absent"))?
                .into_payload()
                .ok_or_else(|| gate_error("retired Simulation manager payload was absent"))?;
            let DmoDeferredProducerState::SimulationProducer(simulation_producer) =
                simulation_state
            else {
                return Err(gate_error(
                    "retired Simulation manager row retained a wrong or failed producer",
                ));
            };
            self.simulation_driver = Some(
                simulation_producer
                    .try_into_registration_driver()
                    .map_err(|_| gate_error("retired Simulation producer was not Terminal"))?,
            );
            self.phase = DmoRegistrationLoadPhase::SimulationReception;
        }
        Ok(())
    }

    fn update_simulation_reception(
        &mut self,
        coordinator: &MainDmoHullAcknowledgementCoordinator<DmoGatePort>,
    ) -> Result<(), Box<dyn Error>> {
        let driver = self
            .simulation_driver
            .as_mut()
            .ok_or_else(|| gate_error("Simulation registration driver was absent"))?;
        match driver.dispatch_one(coordinator)? {
            MainDmoSimulationDeferredDispatch::ReceiverMarkerCompleted(_)
            | MainDmoSimulationDeferredDispatch::RegistrationAcknowledged => {}
            MainDmoSimulationDeferredDispatch::Idle => {
                require(
                    driver.registration_acknowledged(),
                    "DMO Simulation FIFO became idle without registration acknowledgement",
                )?;
                self.phase = DmoRegistrationLoadPhase::PresentationTerminal;
            }
            MainDmoSimulationDeferredDispatch::DeregistrationCallbackReady => {
                return Err(gate_error(
                    "DMO Simulation registration phase reached a DReg callback",
                ));
            }
        }
        Ok(())
    }

    fn finish_presentation_terminal(&mut self) -> Result<(), Box<dyn Error>> {
        require(
            self.simulation_batches == [12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 5]
                && self.presentation_batches == [12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 5],
            format!(
                "DMO deferred batches differed: Simulation {:?}, Presentation {:?}",
                self.simulation_batches, self.presentation_batches
            ),
        )?;
        require(
            self.manager.active_snapshot().is_some_and(|active| {
                active.registration_type == DeferredCollectionRegType::Presentation
                    && active.index == DMO_BANK_ASSET_COUNT
                    && active.maximum == DMO_BANK_ASSET_COUNT
            }),
            "DMO Presentation manager did not retain its completed active row",
        )?;
        let state = self
            .manager
            .active_payload_mut()
            .and_then(Option::take)
            .ok_or_else(|| gate_error("Presentation terminal manager payload was absent"))?;
        let DmoDeferredProducerState::PresentationProducer(mut producer) = state else {
            return Err(gate_error(
                "Presentation terminal manager retained a wrong or failed producer",
            ));
        };
        require(
            producer.next_arena_index() == 125
                && producer.queued_marker_count() == 2
                && producer.front_marker_kind()
                    == Some(
                        MainDmoPresentationDeferredMarkerKind::ActivationZeroCollection {
                            collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
                        },
                    )
                && producer.back_marker_kind()
                    == Some(MainDmoPresentationDeferredMarkerKind::RegistrationCallback),
            "DMO Presentation deferred terminal marker sequence differed",
        )?;
        let deferred = self
            .deferred
            .take()
            .ok_or_else(|| gate_error("Presentation deferred authority was already consumed"))?;
        let registration =
            producer.try_deliver_front_activation_zero(&mut self.adapter, deferred)?;
        let callback = producer.try_take_front_registration_callback()?;
        require(
            producer.queued_marker_count() == 0,
            "DMO Presentation deferred queue did not drain through callback",
        )?;
        self.registration = Some(registration);
        self.registration_callback = Some(callback);
        self.phase = DmoRegistrationLoadPhase::Complete;
        Ok(())
    }

    fn into_parts(
        self,
    ) -> Result<
        (
            DmoPresentationGateAdapter,
            MainDmoBankPresentationRegistration,
            MainDmoPresentationRegistrationCallback,
            DmoSimulationDeferredRegistrationDriver,
        ),
        Box<dyn Error>,
    > {
        require(
            self.is_complete(),
            "DMO registration session was taken before completion",
        )?;
        Ok((
            self.adapter,
            self.registration
                .ok_or_else(|| gate_error("Presentation registration was absent"))?,
            self.registration_callback
                .ok_or_else(|| gate_error("Presentation callback was absent"))?,
            self.simulation_driver
                .ok_or_else(|| gate_error("Simulation driver was absent"))?,
        ))
    }
}

fn drain_presentation_arena_fronts(
    producer: &mut DmoPresentationDeferredProducer,
    adapter: &mut DmoPresentationGateAdapter,
    deferred: &PresentationDmoDeferredCollectionRegistration,
    arena_lookup: &SimulationResidentArenaLookup<DmoBankProcessedArenaAsset>,
) -> Result<(), Box<dyn Error>> {
    while matches!(
        producer.front_marker_kind(),
        Some(MainDmoPresentationDeferredMarkerKind::Arena { .. })
    ) {
        producer.try_deliver_front_arena(
            adapter,
            deferred,
            MAIN_DMO_HULL_USERDATA_RAW,
            arena_lookup,
        )?;
    }
    Ok(())
}

fn setup_dmo_deferred_manager(
    simulation: DmoSimulationDeferredProducer,
    presentation: DmoPresentationDeferredProducer,
    stream_size: u32,
) -> Result<DmoDeferredManager, Box<dyn Error>> {
    let mut manager = DeferredCollectionRegManager::new();
    manager.enqueue(DeferredCollectionRegRow::simulation(
        CollectionId(DMO_BANK_COLLECTION_ID),
        MAIN_DMO_HULL_USERDATA_RAW,
        stream_size,
        0,
        Some(DmoDeferredProducerState::SimulationProducer(simulation)),
    ))?;
    manager.enqueue(DeferredCollectionRegRow::presentation(
        CollectionId(DMO_BANK_COLLECTION_ID),
        CollectionId(DMO_BANK_PARENT_ID),
        MAIN_DMO_HULL_USERDATA_RAW,
        stream_size,
        0,
        Some(DmoDeferredProducerState::PresentationProducer(presentation)),
    ))?;
    let setup = manager
        .try_update(
            DeferredCollectionRegType::Simulation,
            |_| -> Result<u32, Box<dyn Error>> {
                Err(gate_error(
                    "Simulation manager advanced before installing its active row",
                ))
            },
            |row| setup_dmo_deferred_row(row, stream_size),
        )
        .map_err(|error| gate_error(error.to_string()))?;
    require(
        matches!(
            setup,
            DeferredCollectionRegManagerUpdate::Entered {
                active: DeferredCollectionRegActiveEffect::NoActiveRow,
                setup: DeferredCollectionRegSetupEffect::Activated {
                    registration_type: DeferredCollectionRegType::Simulation,
                    maximum: DMO_BANK_ASSET_COUNT,
                },
                retired: None,
            }
        ),
        "DMO Simulation manager did not perform one Setup-only activation",
    )?;
    require(
        manager.pending_count() == 1,
        "DMO mixed manager did not retain exactly the Presentation row after Simulation Setup",
    )?;
    Ok(manager)
}

fn setup_dmo_deferred_row(
    row: &mut DeferredCollectionRegRow<Option<DmoDeferredProducerState>>,
    stream_size: u32,
) -> Result<u32, Box<dyn Error>> {
    require(
        row.collection_id() == CollectionId(DMO_BANK_COLLECTION_ID)
            && row.userdata_raw() == MAIN_DMO_HULL_USERDATA_RAW
            && row.stream_size() == stream_size
            && row.stream_type_raw() == 0,
        "DMO deferred pending manager row fields differed",
    )?;
    if row.registration_type() == DeferredCollectionRegType::Presentation {
        require(
            row.proxy_collection_id() == CollectionId(DMO_BANK_PARENT_ID),
            "DMO Presentation pending manager proxy field differed",
        )?;
    }
    let state = row
        .payload_mut()
        .take()
        .ok_or_else(|| gate_error("DMO manager Setup payload was absent"))?;
    match (row.registration_type(), state) {
        (
            DeferredCollectionRegType::Simulation,
            DmoDeferredProducerState::SimulationProducer(producer),
        ) => match producer.try_update_with(
            || -> Result<DeferredCollectionBatchControls, Infallible> {
                panic!("Pending Simulation producer sampled controls")
            },
        ) {
            Ok((producer, update)) => {
                require(
                    update == MainDmoSimulationDeferredProducerUpdate::SetupCompleted,
                    "DMO Simulation deferred producer did not begin with Setup only",
                )?;
                *row.payload_mut() = Some(DmoDeferredProducerState::SimulationProducer(producer));
                Ok(DMO_BANK_ASSET_COUNT)
            }
            Err(failure) => {
                let error = gate_error(failure.to_string());
                *row.payload_mut() = Some(DmoDeferredProducerState::SimulationFailure(failure));
                Err(error)
            }
        },
        (
            DeferredCollectionRegType::Presentation,
            DmoDeferredProducerState::PresentationProducer(producer),
        ) => match producer.try_update_with(
            || -> Result<DeferredCollectionBatchControls, Infallible> {
                panic!("Pending Presentation producer sampled controls")
            },
        ) {
            Ok((producer, update)) => {
                require(
                    update == MainDmoPresentationDeferredProducerUpdate::SetupCompleted,
                    "DMO Presentation deferred producer did not begin with Setup only",
                )?;
                *row.payload_mut() = Some(DmoDeferredProducerState::PresentationProducer(producer));
                Ok(DMO_BANK_ASSET_COUNT)
            }
            Err(failure) => {
                let error = gate_error(failure.to_string());
                *row.payload_mut() = Some(DmoDeferredProducerState::PresentationFailure(failure));
                Err(error)
            }
        },
        (registration_type, state) => {
            *row.payload_mut() = Some(state);
            Err(gate_error(format!(
                "DMO {registration_type:?} manager reached Setup with a wrong or failed producer"
            )))
        }
    }
}

fn update_simulation_deferred_active(
    mut active: DeferredCollectionRegActive<'_, Option<DmoDeferredProducerState>>,
) -> Result<(u32, MainDmoSimulationDeferredProducerUpdate), Box<dyn Error>> {
    let state = active
        .payload_mut()
        .take()
        .ok_or_else(|| gate_error("Simulation active manager payload was absent"))?;
    let result = match state {
        DmoDeferredProducerState::SimulationProducer(producer) => producer.try_update_with(|| {
            Ok::<DeferredCollectionBatchControls, Infallible>(
                DeferredCollectionBatchControls::NATIVE_DEFAULT,
            )
        }),
        DmoDeferredProducerState::SimulationFailure(failure) => failure.retry_with(|| {
            Ok::<DeferredCollectionBatchControls, Infallible>(
                DeferredCollectionBatchControls::NATIVE_DEFAULT,
            )
        }),
        state => {
            *active.payload_mut() = Some(state);
            return Err(gate_error(
                "Simulation active manager retained a Presentation producer",
            ));
        }
    };
    match result {
        Ok((producer, update)) => {
            let count = match update {
                MainDmoSimulationDeferredProducerUpdate::ArenasQueued { count }
                | MainDmoSimulationDeferredProducerUpdate::TerminalQueued { arena_count: count } => {
                    count
                }
                MainDmoSimulationDeferredProducerUpdate::SetupCompleted
                | MainDmoSimulationDeferredProducerUpdate::TerminalIdle => 0,
            };
            *active.payload_mut() = Some(DmoDeferredProducerState::SimulationProducer(producer));
            Ok((count, update))
        }
        Err(failure) => {
            let error = gate_error(failure.to_string());
            *active.payload_mut() = Some(DmoDeferredProducerState::SimulationFailure(failure));
            Err(error)
        }
    }
}

fn update_presentation_deferred_active(
    mut active: DeferredCollectionRegActive<'_, Option<DmoDeferredProducerState>>,
) -> Result<(u32, MainDmoPresentationDeferredProducerUpdate), Box<dyn Error>> {
    let state = active
        .payload_mut()
        .take()
        .ok_or_else(|| gate_error("Presentation active manager payload was absent"))?;
    let result = match state {
        DmoDeferredProducerState::PresentationProducer(producer) => {
            producer.try_update_with(|| {
                Ok::<DeferredCollectionBatchControls, Infallible>(
                    DeferredCollectionBatchControls::NATIVE_DEFAULT,
                )
            })
        }
        DmoDeferredProducerState::PresentationFailure(failure) => failure.retry_with(|| {
            Ok::<DeferredCollectionBatchControls, Infallible>(
                DeferredCollectionBatchControls::NATIVE_DEFAULT,
            )
        }),
        state => {
            *active.payload_mut() = Some(state);
            return Err(gate_error(
                "Presentation active manager retained a Simulation producer",
            ));
        }
    };
    match result {
        Ok((producer, update)) => {
            let count = match update {
                MainDmoPresentationDeferredProducerUpdate::ArenasQueued { count } => count,
                MainDmoPresentationDeferredProducerUpdate::TerminalQueued {
                    arena_count, ..
                } => arena_count,
                MainDmoPresentationDeferredProducerUpdate::SetupCompleted
                | MainDmoPresentationDeferredProducerUpdate::TerminalIdle => 0,
            };
            *active.payload_mut() = Some(DmoDeferredProducerState::PresentationProducer(producer));
            Ok((count, update))
        }
        Err(failure) => {
            let error = gate_error(failure.to_string());
            *active.payload_mut() = Some(DmoDeferredProducerState::PresentationFailure(failure));
            Err(error)
        }
    }
}

fn verify_simulation_registration_events(
    registry: &Rc<RefCell<DmoGateRegistryTelemetry>>,
    events: &[DmoSimulationGateEvent],
) -> Result<(), Box<dyn Error>> {
    let registry = registry.borrow();
    let expected = expected_simulation_events(&registry, false)?;
    require(
        expected.len() == 343 && events == expected && registry.dmo_operation_count == 93,
        format!(
            "DMO Simulation registration stream differed: {} events / {} DMO operations",
            events.len(),
            registry.dmo_operation_count
        ),
    )
}

fn verify_simulation_deregistration_events(
    registry: &Rc<RefCell<DmoGateRegistryTelemetry>>,
    events: &[DmoSimulationGateEvent],
) -> Result<(), Box<dyn Error>> {
    let registry = registry.borrow();
    let expected = expected_simulation_events(&registry, true)?;
    require(
        expected.len() == 686 && events == expected,
        format!(
            "DMO Simulation full lifecycle stream differed: {} events",
            events.len()
        ),
    )
}

fn expected_simulation_events(
    registry: &DmoGateRegistryTelemetry,
    include_deregistration: bool,
) -> Result<Vec<DmoSimulationGateEvent>, Box<dyn Error>> {
    let assets = registry
        .assets_view
        .as_ref()
        .ok_or_else(|| gate_error("DMO Simulation verification lost its resident asset view"))?;
    let mut expected = Vec::with_capacity(if include_deregistration { 686 } else { 343 });
    let mut ordinal = 0_u32;
    assets
        .try_for_each(|asset| {
            for operation in asset.plan().operations() {
                if let SimulationNormalOperation::DmoInstance { asset_id, .. } = *operation {
                    expected.push(DmoSimulationGateEvent::DmoModelAdd(asset_id));
                }
            }
            expected.push(DmoSimulationGateEvent::LocationRegister {
                ordinal,
                container_count: 0,
            });
            ordinal = ordinal
                .checked_add(1)
                .expect("the pinned 125-Arena collection fits u32");
            Ok::<_, Infallible>(())
        })
        .map_err(|error| gate_error(error.to_string()))?;
    expected.extend(
        (0..DMO_BANK_ASSET_COUNT).map(|ordinal| DmoSimulationGateEvent::DmoRegister {
            ordinal,
            operation_count: 0,
        }),
    );
    require(
        ordinal == DMO_BANK_ASSET_COUNT,
        "DMO Simulation expected stream did not visit 125 Arenas",
    )?;
    if include_deregistration {
        expected.extend(
            (0..DMO_BANK_ASSET_COUNT)
                .map(|ordinal| DmoSimulationGateEvent::DmoDeregister { ordinal }),
        );
        ordinal = 0;
        assets
            .try_for_each(|asset| {
                for operation in asset.plan().operations() {
                    if let SimulationNormalOperation::DmoInstance { asset_id, .. } = *operation {
                        expected.push(DmoSimulationGateEvent::DmoModelRemove(asset_id));
                    }
                }
                expected.push(DmoSimulationGateEvent::LocationDeregister { ordinal });
                ordinal = ordinal
                    .checked_add(1)
                    .expect("the pinned 125-Arena collection fits u32");
                Ok::<_, Infallible>(())
            })
            .map_err(|error| gate_error(error.to_string()))?;
        require(
            ordinal == DMO_BANK_ASSET_COUNT,
            "DMO Simulation expected teardown did not visit 125 Arenas",
        )?;
    }
    Ok(expected)
}

fn verify_presentation_registration_events(
    events: &[DmoPresentationGateEvent],
) -> Result<(), Box<dyn Error>> {
    require(
        events.len() == 187,
        format!(
            "DMO Presentation registration emitted {} events, expected 187",
            events.len()
        ),
    )?;
    require(
        events.first()
            == Some(&DmoPresentationGateEvent::MaterialAdd(
                MAIN_DMO_MATERIAL_GUID,
            )),
        format!(
            "DMO Presentation material registration differed: {:?}",
            events.first()
        ),
    )?;
    for pair in events[1..].chunks_exact(2) {
        let [
            DmoPresentationGateEvent::LivingWorldAdd(asset_id),
            DmoPresentationGateEvent::DmoRegister { primary_guid, .. },
        ] = pair
        else {
            return Err(gate_error(format!(
                "DMO Presentation registration pair order differed: {pair:?}"
            )));
        };
        require(
            asset_id == primary_guid,
            format!(
                "DMO Presentation registration IDs differed: {asset_id:016x} != {primary_guid:016x}"
            ),
        )?;
    }
    let signature = dmo_presentation_event_signature(events);
    require(
        signature == DMO_PRESENTATION_REGISTRATION_EVENT_SIGNATURE,
        format!("DMO Presentation registration signature differed: {signature:016x}"),
    )?;
    let dmo_signature = dmo_only_presentation_event_signature(events);
    require(
        dmo_signature == DMO_ONLY_REGISTRATION_EVENT_SIGNATURE,
        format!("DMO-only Presentation registration signature differed: {dmo_signature:016x}"),
    )?;
    Ok(())
}

fn dmo_presentation_event_signature(events: &[DmoPresentationGateEvent]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in b"PRES_DMO_EFFECT_STREAM_V1\0" {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for event in events {
        let (tag, first, second) = match *event {
            DmoPresentationGateEvent::MaterialAdd(id) => (0_u8, id, 0),
            DmoPresentationGateEvent::LivingWorldAdd(id) => (1, id, 0),
            DmoPresentationGateEvent::DmoRegister {
                primary_guid,
                local_guid,
            } => (2, primary_guid, local_guid),
            DmoPresentationGateEvent::MaterialRemove(id) => (0x80, id, 0),
            DmoPresentationGateEvent::LivingWorldRemove(id) => (0x81, id, 0),
            DmoPresentationGateEvent::DmoDeregister {
                primary_guid,
                local_guid,
            } => (0x82, primary_guid, local_guid),
        };
        for byte in [tag]
            .into_iter()
            .chain(first.to_be_bytes())
            .chain(second.to_be_bytes())
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn dmo_only_presentation_event_signature(events: &[DmoPresentationGateEvent]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for event in events {
        let Some((tag, first, second)) = (match *event {
            DmoPresentationGateEvent::LivingWorldAdd(id) => Some((0_u8, id, 0)),
            DmoPresentationGateEvent::DmoRegister {
                primary_guid,
                local_guid,
            } => Some((1, primary_guid, local_guid)),
            DmoPresentationGateEvent::LivingWorldRemove(id) => Some((2, id, 0)),
            DmoPresentationGateEvent::DmoDeregister {
                primary_guid,
                local_guid,
            } => Some((3, primary_guid, local_guid)),
            DmoPresentationGateEvent::MaterialAdd(_)
            | DmoPresentationGateEvent::MaterialRemove(_) => None,
        }) else {
            continue;
        };
        for byte in [tag]
            .into_iter()
            .chain(first.to_be_bytes())
            .chain(second.to_be_bytes())
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

#[allow(clippy::too_many_arguments)]
fn verify_evidence_backed_prefix<S>(
    coordinator: &MainDmoHullAcknowledgementCoordinator<DmoGatePort>,
    streamer: &S,
    port_state: &Rc<RefCell<DmoGatePortState>>,
    events: &Rc<RefCell<Vec<DmoGateEvent>>>,
    registry: &Rc<RefCell<DmoGateRegistryTelemetry>>,
    simulation_lookup: &SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    garbage: &Rc<RefCell<Vec<CollectionId>>>,
    catalog: &WorldStreamCatalog,
    collection: &CollectionRegistrationSnapshot,
    request_live: &Arc<AtomicBool>,
    load_totals: &DmoGateDriverTotals,
) -> Result<(), Box<dyn Error>>
where
    S: DmoGateObservation,
{
    let id = CollectionId(DMO_BANK_COLLECTION_ID);
    let request = expected_registration_request();
    require(
        events.borrow().as_slice()
            == [
                DmoGateEvent::WrapperCreated(id),
                DmoGateEvent::WrapperPublished(id),
                DmoGateEvent::RegisterSimulation(request),
                DmoGateEvent::RegisterPresentation(request),
            ],
        format!("DMO hull prefix ordering differed: {:?}", events.borrow()),
    )?;
    let coordinator_observation = coordinator.observation()?;
    require(
        coordinator_observation.state == Some(MainDmoHullRawState::LoadingPresentationCollections)
            && coordinator_observation.last_raw_state
                == MainDmoHullRawState::LoadingPresentationCollections
            && coordinator_observation.physical_load_remaining == 0
            && coordinator_observation.registration_remaining == 2
            && coordinator_observation.physical_unload_completed == 0
            && coordinator_observation.unregistration_completed == 0
            && !coordinator_observation.unload_requested
            && coordinator_observation.fault.is_none()
            && !coordinator_observation.released
            && !coordinator.is_loaded()?
            && !coordinator.is_unloaded()?,
        format!("DMO hull prefix coordinator differed: {coordinator_observation:?}"),
    )?;
    require(
        coordinator.update()? == coordinator_observation,
        "DMO hull coordinator advanced without subsystem acknowledgements",
    )?;
    let resident = streamer
        .gate_observe_resident(id)
        .ok_or_else(|| gate_error("DMO hull prefix did not publish its resident"))?;
    require(
        resident.ref_count == 3 && resident.wrapper_is_published,
        format!("DMO hull prefix resident differed: {resident:?}"),
    )?;
    require(
        streamer.gate_resident_count() == 1 && streamer.gate_highest_stream_block_size() == 597_388,
        "DMO hull prefix resident count or high-water block differed",
    )?;
    require(
        load_totals.cache_hits == 0
            && load_totals.unloads_processed == 0
            && load_totals.physical_loads_started == 1
            && load_totals.physical_loads_completed == 1
            && load_totals.decompression_kicks > 0,
        format!("DMO hull prefix driver totals differed: {load_totals:?}"),
    )?;
    require(
        !request_live.load(Ordering::Acquire),
        "DMO hull prefix request-ID lease remained live",
    )?;
    let file = catalog
        .file(&collection.owner)?
        .ok_or_else(|| gate_error("DMO hull prefix owner disappeared"))?;
    let registry = registry.borrow();
    let model_routes = registry
        .model_routes
        .as_ref()
        .ok_or_else(|| gate_error("DMO hull prefix lost its resident model routes"))?;
    require(
        registry.asset_ids.len() == 125
            && registry.asset_ids.len() == file.asset_ids.len()
            && registry
                .asset_ids
                .iter()
                .zip(file.asset_ids.iter())
                .all(|(actual, expected)| actual.0 == *expected)
            && registry.operation_count == 93
            && registry.dmo_operation_count == 93
            && model_routes.asset_count() == registry.asset_ids.len()
            && model_routes.model_count() == registry.dmo_operation_count
            && model_routes.is_resident()
            && model_routes.signature() == DMO_RESIDENT_MODEL_ROUTE_SIGNATURE_V1
            && registry.active == Some((id, 1)),
        format!(
            "DMO hull prefix resident assets, operations, or model routes differed: {} assets / {} routes / {:016x}",
            model_routes.asset_count(),
            model_routes.model_count(),
            model_routes.signature()
        ),
    )?;
    verify_simulation_resident_prefix(
        simulation_lookup,
        id,
        &registry.asset_ids,
        registry.dmo_operation_count,
    )?;
    require(
        garbage.borrow().is_empty(),
        "DMO hull prefix performed garbage collection before cleanup",
    )?;
    verify_prefix_callback_capabilities(port_state)
}

fn verify_prefix_callback_capabilities(
    port_state: &Rc<RefCell<DmoGatePortState>>,
) -> Result<(), Box<dyn Error>> {
    let port = port_state.borrow();
    require(
        port.simulation_registration.is_some()
            && port.presentation_registration.is_some()
            && port.simulation_deregistration.is_none()
            && port.presentation_deregistration.is_none(),
        "DMO hull prefix did not retain exactly two registration capabilities",
    )
}

fn take_registration_callbacks(
    port_state: &Rc<RefCell<DmoGatePortState>>,
) -> Result<
    (
        MainDmoSimulationRegistrationCallback,
        MainDmoPresentationRegistrationCallback,
    ),
    Box<dyn Error>,
> {
    let mut port = port_state.borrow_mut();
    let simulation = port
        .simulation_registration
        .take()
        .ok_or_else(|| gate_error("Simulation deferred route lacks its hull callback"))?;
    let presentation = port
        .presentation_registration
        .take()
        .ok_or_else(|| gate_error("Presentation deferred route lacks its hull callback"))?;
    Ok((simulation, presentation))
}

struct DmoSimulationResidentObservation {
    asset_ids: Vec<CollectionId>,
    dmo_operation_count: usize,
}

/// Read-only real-corpus validation performed after `drive_to_idle` has run
/// the hull load callback and acquired both subsystem resident holds. It does
/// not construct the sealed main-bank binding, dispatch its marker/receiver,
/// or claim the native `GetRegAsset` call at `0x8277ea08` occurs here.
fn observe_simulation_resident(
    lookup: &SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    id: CollectionId,
) -> Result<Option<DmoSimulationResidentObservation>, Box<dyn Error>> {
    let Some(occurrence) = lookup
        .lookup(id)
        .map_err(|error| Box::new(error) as Box<dyn Error>)?
    else {
        return Ok(None);
    };
    let mut asset_ids = Vec::new();
    let mut dmo_operation_count = 0_usize;
    occurrence
        .try_for_each::<Infallible>(|asset| {
            asset_ids.push(asset.resident_asset_id());
            dmo_operation_count += asset
                .plan()
                .operations()
                .iter()
                .filter(|operation| {
                    matches!(operation, SimulationNormalOperation::DmoInstance { .. })
                })
                .count();
            Ok(())
        })
        .map_err(|error| gate_error(error.to_string()))?;
    Ok(Some(DmoSimulationResidentObservation {
        asset_ids,
        dmo_operation_count,
    }))
}

fn verify_simulation_resident_prefix(
    lookup: &SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    id: CollectionId,
    expected_asset_ids: &[CollectionId],
    expected_dmo_operation_count: usize,
) -> Result<(), Box<dyn Error>> {
    let observation = observe_simulation_resident(lookup, id)?
        .ok_or_else(|| gate_error("DMO hull prefix lacked its Simulation resident occurrence"))?;
    require(
        observation.asset_ids == expected_asset_ids
            && observation.dmo_operation_count == expected_dmo_operation_count,
        "Simulation resident occurrence differed from the exact DMO gate sequence",
    )
}

trait DmoGateObservation {
    fn gate_resident_count(&self) -> usize;
    fn gate_highest_stream_block_size(&self) -> u32;
    fn gate_observe_resident(
        &self,
        id: CollectionId,
    ) -> Option<skate2_core::collection_resident_streamer::ResidentCollectionObservation>;
}

impl<S, M, F, R, G, K> DmoGateObservation
    for CollectionResidentStreamer<S, M, F, R, G, K, DmoGateCallback, DmoGateError>
where
    S: skate2_core::collection_resident_streamer::ResidentCollectionStorage<DmoGateError>,
    M: ResidentCollectionMetadata<DmoGateError>,
    F: skate2_core::collection_resident_streamer::ResidentCollectionLoadFactory<
            S::Writer,
            M::Descriptor,
            DmoGateError,
        >,
    R: ResidentCollectionRegistry<S::Collection, DmoGateError>,
    G: ResidentAssetGarbageCollector<DmoGateError>,
    K: skate2_core::collection_resident_streamer::ResidentDecompressionKick<DmoGateError>,
{
    fn gate_resident_count(&self) -> usize {
        self.resident_count()
    }

    fn gate_highest_stream_block_size(&self) -> u32 {
        self.highest_stream_block_size()
    }

    fn gate_observe_resident(
        &self,
        id: CollectionId,
    ) -> Option<skate2_core::collection_resident_streamer::ResidentCollectionObservation> {
        self.observe_resident(id)
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn synthetic_mechanical_cleanup_without_subsystem_effects<S>(
    coordinator: &MainDmoHullAcknowledgementCoordinator<DmoGatePort>,
    streamer: &mut S,
    port_state: &Rc<RefCell<DmoGatePortState>>,
    events: &Rc<RefCell<Vec<DmoGateEvent>>>,
    registry: &Rc<RefCell<DmoGateRegistryTelemetry>>,
    simulation_lookup: &SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    garbage: &Rc<RefCell<Vec<CollectionId>>>,
    collection: &CollectionRegistrationSnapshot,
    mut deferred_runtime: DmoDeferredGateRuntime,
    unload_presentation_manifest: impl FnOnce() -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>>
where
    S: DmoGateCleanupStreamer + MainDmoResidentHoldReleaser<Error = DmoGateError>,
{
    require(
        port_state.borrow().simulation_registration.is_none()
            && deferred_runtime.registration_callback.is_none()
            && deferred_runtime
                .simulation_driver
                .as_ref()
                .is_some_and(DmoSimulationDeferredRegistrationDriver::registration_acknowledged),
        "SecondaryLoad01 wait returned before both registration callbacks were consumed",
    )?;
    require(
        coordinator.observation()?.state == Some(MainDmoHullRawState::LoadCompleteNotify),
        "SecondaryLoad01 wait did not return at raw state 10",
    )?;
    require(
        coordinator.update()?.state == Some(MainDmoHullRawState::LoadComplete),
        "synthetic cleanup did not advance raw state 10 to 11",
    )?;
    coordinator.request_unload()?;
    require(
        coordinator.update()?.state == Some(MainDmoHullRawState::UnloadNotStarted),
        "synthetic cleanup did not reach raw state 12",
    )?;
    coordinator.start_collection_unload(|id, callback| {
        streamer.gate_enqueue_unload(id, DmoGateCallback(callback));
    })?;
    let unload_totals = streamer.gate_drive_to_idle()?;
    require(
        unload_totals.unloads_processed == 1
            && unload_totals.physical_loads_started == 0
            && unload_totals.physical_loads_completed == 0,
        format!("synthetic cleanup unload totals differed: {unload_totals:?}"),
    )?;
    let id = CollectionId(DMO_BANK_COLLECTION_ID);
    let after_base_down = streamer
        .gate_observe_resident(id)
        .ok_or_else(|| gate_error("synthetic cleanup lost resident before hold releases"))?;
    require(
        after_base_down.ref_count == 2 && after_base_down.wrapper_is_published,
        format!("synthetic cleanup base Down differed: {after_base_down:?}"),
    )?;
    require(
        observe_simulation_resident(simulation_lookup, id)?.is_some(),
        "Simulation resident occurrence expired before either subsystem hold was released",
    )?;
    require(
        registry
            .borrow()
            .model_routes
            .as_ref()
            .is_some_and(DmoBankResidentModelCatalog::is_resident),
        "DMO model routes expired before either subsystem hold was released",
    )?;
    let after_physical_unload = coordinator.observation()?;
    require(
        after_physical_unload.state == Some(MainDmoHullRawState::UnloadingCollections)
            && after_physical_unload.physical_unload_completed == 1
            && after_physical_unload.unregistration_completed == 0,
        format!("synthetic cleanup physical-unload counters differed: {after_physical_unload:?}"),
    )?;
    let dreg = expected_deregistration_request();
    require(
        events.borrow().as_slice()
            == [
                DmoGateEvent::WrapperCreated(id),
                DmoGateEvent::WrapperPublished(id),
                DmoGateEvent::RegisterSimulation(expected_registration_request()),
                DmoGateEvent::RegisterPresentation(expected_registration_request()),
                DmoGateEvent::DeregisterSimulation(dreg),
                DmoGateEvent::DeregisterPresentation(dreg),
            ],
        format!(
            "synthetic cleanup deregistration order differed: {:?}",
            events.borrow()
        ),
    )?;

    let simulation_deregistration = port_state
        .borrow_mut()
        .simulation_deregistration
        .take()
        .ok_or_else(|| gate_error("synthetic cleanup lacks Simulation DReg token"))?;
    let simulation_driver = deferred_runtime
        .simulation_driver
        .take()
        .ok_or_else(|| gate_error("Simulation deferred driver was already consumed"))?;
    let mut simulation_driver = simulation_driver
        .try_queue_deregistration(simulation_deregistration)
        .map_err(|failure| gate_error(failure.to_string()))?;
    let collection_dispatch = simulation_driver.dispatch_one(coordinator)?;
    require(
        collection_dispatch
            == MainDmoSimulationDeferredDispatch::ReceiverMarkerCompleted(
                MainDmoSimulationDeferredMarkerKind::DeregistrationCollection,
            ),
        format!("DMO Simulation DRegCollection dispatch differed: {collection_dispatch:?}"),
    )?;
    verify_simulation_deregistration_events(
        registry,
        &deferred_runtime.simulation_events.borrow(),
    )?;
    require(
        simulation_driver.dispatch_one(coordinator)?
            == MainDmoSimulationDeferredDispatch::DeregistrationCallbackReady,
        "DMO Simulation DReg callback did not reach the sealed front",
    )?;
    require(
        !simulation_driver
            .acknowledge_deregistration_callback(coordinator, streamer)
            .map_err(|error| gate_error(error.to_string()))?,
        "DMO Simulation callback destroyed resident after the first subsystem hold",
    )?;
    require(
        simulation_driver.dispatch_one(coordinator)? == MainDmoSimulationDeferredDispatch::Idle,
        "DMO Simulation FIFO was not complete after its DReg callback",
    )?;
    deferred_runtime.simulation_driver = Some(simulation_driver);
    let after_first_hold = streamer
        .gate_observe_resident(id)
        .ok_or_else(|| gate_error("synthetic cleanup lost resident after first hold"))?;
    require(
        after_first_hold.ref_count == 1 && coordinator.observation()?.unregistration_completed == 1,
        format!("synthetic cleanup first hold differed: {after_first_hold:?}"),
    )?;
    require(
        observe_simulation_resident(simulation_lookup, id)?.is_some(),
        "Simulation resident occurrence expired after only the first subsystem hold",
    )?;
    require(
        registry
            .borrow()
            .model_routes
            .as_ref()
            .is_some_and(DmoBankResidentModelCatalog::is_resident),
        "DMO model routes expired after only the first subsystem hold",
    )?;
    let presentation_deregistration = port_state
        .borrow_mut()
        .presentation_deregistration
        .take()
        .ok_or_else(|| gate_error("synthetic cleanup lacks Presentation DReg token"))?;
    let presentation_adapter_registration = deferred_runtime
        .registration
        .take()
        .ok_or_else(|| gate_error("DMO Presentation registration token was consumed"))?;
    deferred_runtime
        .adapter
        .deregister_collection_after_resident_lookup(
            id,
            presentation_adapter_registration,
            MAIN_DMO_HULL_USERDATA_RAW,
            simulation_lookup,
        )?;
    verify_presentation_deregistration_gate(&deferred_runtime)?;
    require(
        coordinator
            .acknowledge_presentation_deregistration(presentation_deregistration, streamer)
            .map_err(|error| gate_error(error.to_string()))?,
        "synthetic cleanup did not destroy resident after final hold",
    )?;
    require(
        streamer.gate_resident_count() == 0 && streamer.gate_observe_resident(id).is_none(),
        "synthetic cleanup retained the DMO resident",
    )?;
    require(
        observe_simulation_resident(simulation_lookup, id)?.is_none(),
        "Simulation resident occurrence remained published after final resident removal",
    )?;
    require(
        garbage.borrow().len() == 125
            && garbage
                .borrow()
                .iter()
                .zip(registry.borrow().asset_ids.iter())
                .all(|(actual, expected)| actual == expected),
        "synthetic cleanup garbage order differed",
    )?;
    {
        let registry = registry.borrow();
        require(
            registry.active.is_none()
                && registry.events.borrow().as_slice()
                    == [
                        DmoGateEvent::WrapperCreated(id),
                        DmoGateEvent::WrapperPublished(id),
                        DmoGateEvent::RegisterSimulation(expected_registration_request()),
                        DmoGateEvent::RegisterPresentation(expected_registration_request()),
                        DmoGateEvent::DeregisterSimulation(dreg),
                        DmoGateEvent::DeregisterPresentation(dreg),
                        DmoGateEvent::WrapperRemoved(id),
                    ],
            "synthetic cleanup did not remove the exact wrapper",
        )?;
        let detached = registry
            .assets_view
            .as_ref()
            .ok_or_else(|| gate_error("synthetic cleanup lost its weak asset view"))?
            .try_for_each::<Infallible>(|_| Ok(()));
        require(
            matches!(
                detached,
                Err(
                    skate2_core::owned_resident_collection::OwnedResidentAssetsVisitError::Detached
                )
            ),
            "synthetic cleanup weak asset view remained attached",
        )?;
        let model_routes = registry
            .model_routes
            .as_ref()
            .ok_or_else(|| gate_error("synthetic cleanup lost its weak model catalog"))?;
        require(
            !model_routes.is_resident()
                && model_routes.asset_count() == 125
                && model_routes.model_count() == 93
                && model_routes.signature() == DMO_RESIDENT_MODEL_ROUTE_SIGNATURE_V1,
            "synthetic cleanup model catalog did not expire with stable diagnostics",
        )?;
    }

    let requested = Rc::new(RefCell::new(Vec::new()));
    let requested_capture = Rc::clone(&requested);
    coordinator.request_stream_unload(move |stream| {
        requested_capture.borrow_mut().push(stream);
    })?;
    require(
        requested.borrow().as_slice() == [MainDmoHullStream::Presentation],
        "synthetic cleanup requested a non-Presentation stream unload",
    )?;
    unload_presentation_manifest()?;
    let poll_order = Rc::new(RefCell::new(Vec::new()));
    let poll_capture = Rc::clone(&poll_order);
    // Presentation is now physically released. The absent Simulation and
    // Texture streams return true only as an explicit host cleanup policy;
    // these values are not a native timing or subsystem-completion trace.
    require(
        coordinator.poll_stream_unloaded(move |stream| {
            poll_capture.borrow_mut().push(stream);
            true
        })?,
        "synthetic cleanup stream polls did not complete",
    )?;
    require(
        poll_order.borrow().as_slice()
            == [
                MainDmoHullStream::Simulation,
                MainDmoHullStream::Presentation,
                MainDmoHullStream::Texture,
            ],
        "synthetic cleanup stream poll order differed",
    )?;
    coordinator.finish_unload_notification(
        MainDmoHullUnloadOutcome::Complete,
        None::<fn(MainDmoHullId, MainDmoHullUnloadOutcome)>,
    )?;
    let final_observation = coordinator.observation()?;
    require(
        final_observation.released
            && final_observation.state.is_none()
            && final_observation.last_raw_state == MainDmoHullRawState::UnloadComplete
            && coordinator.is_unloaded()?,
        format!("synthetic cleanup final coordinator differed: {final_observation:?}"),
    )?;
    let port = port_state.borrow();
    require(
        port.simulation_registration.is_none()
            && port.presentation_registration.is_none()
            && port.simulation_deregistration.is_none()
            && port.presentation_deregistration.is_none(),
        "synthetic cleanup retained a subsystem callback capability",
    )?;
    require(
        collection.id == DMO_BANK_COLLECTION_ID,
        "synthetic cleanup collection identity changed",
    )?;
    require(
        deferred_runtime.simulation_events.borrow().len() == 686
            && deferred_runtime.simulation_driver.is_some(),
        "synthetic cleanup did not retain the completed Simulation lifecycle trace",
    )
}

fn verify_presentation_deregistration_gate(
    runtime: &DmoDeferredGateRuntime,
) -> Result<(), Box<dyn Error>> {
    require(
        runtime.adapter.live_collection_count() == 0
            && runtime.adapter.first_use_owner_count() == 0
            && runtime.adapter.orphaned_owner_count() == 0,
        "DMO Presentation deregistration retained matched owners",
    )?;
    let events = runtime.events.borrow();
    require(
        events.len() == 374,
        format!(
            "DMO Presentation full lifecycle emitted {} events, expected 374",
            events.len()
        ),
    )?;
    let (registration, deregistration) = events.split_at(187);
    require(
        registration.first()
            == Some(&DmoPresentationGateEvent::MaterialAdd(
                MAIN_DMO_MATERIAL_GUID,
            ))
            && deregistration.first()
                == Some(&DmoPresentationGateEvent::MaterialRemove(
                    MAIN_DMO_MATERIAL_GUID,
                )),
        "DMO Presentation material lifecycle identity differed",
    )?;
    for (registered, deregistered) in registration[1..]
        .chunks_exact(2)
        .zip(deregistration[1..].chunks_exact(2))
    {
        let [
            DmoPresentationGateEvent::LivingWorldAdd(add_id),
            DmoPresentationGateEvent::DmoRegister {
                primary_guid: reg_primary,
                local_guid: reg_local,
            },
        ] = registered
        else {
            return Err(gate_error("DMO Presentation registration sequence changed"));
        };
        let [
            DmoPresentationGateEvent::LivingWorldRemove(remove_id),
            DmoPresentationGateEvent::DmoDeregister {
                primary_guid: dreg_primary,
                local_guid: dreg_local,
            },
        ] = deregistered
        else {
            return Err(gate_error(format!(
                "DMO Presentation deregistration pair order differed: {deregistered:?}"
            )));
        };
        require(
            add_id == remove_id
                && reg_primary == dreg_primary
                && reg_local == dreg_local
                && add_id == reg_primary,
            "DMO Presentation registration/deregistration identity differed",
        )?;
    }
    let signature = dmo_presentation_event_signature(&events);
    require(
        signature == DMO_PRESENTATION_LIFECYCLE_EVENT_SIGNATURE,
        format!("DMO Presentation lifecycle signature differed: {signature:016x}"),
    )?;
    let dmo_signature = dmo_only_presentation_event_signature(&events);
    require(
        dmo_signature == DMO_ONLY_LIFECYCLE_EVENT_SIGNATURE,
        format!("DMO-only Presentation lifecycle signature differed: {dmo_signature:016x}"),
    )?;
    Ok(())
}

trait DmoGateCleanupStreamer: DmoGateObservation {
    fn gate_enqueue_unload(&mut self, id: CollectionId, callback: DmoGateCallback);
    fn gate_drive_to_idle(&mut self) -> Result<DmoGateDriverTotals, Box<dyn Error>>;
}

impl<S, M, F, R, G, K> DmoGateCleanupStreamer
    for CollectionResidentStreamer<S, M, F, R, G, K, DmoGateCallback, DmoGateError>
where
    S: skate2_core::collection_resident_streamer::ResidentCollectionStorage<DmoGateError>,
    M: ResidentCollectionMetadata<DmoGateError>,
    F: skate2_core::collection_resident_streamer::ResidentCollectionLoadFactory<
            S::Writer,
            M::Descriptor,
            DmoGateError,
        >,
    R: ResidentCollectionRegistry<S::Collection, DmoGateError>,
    G: ResidentAssetGarbageCollector<DmoGateError>,
    K: skate2_core::collection_resident_streamer::ResidentDecompressionKick<DmoGateError>,
{
    fn gate_enqueue_unload(&mut self, id: CollectionId, callback: DmoGateCallback) {
        self.enqueue_unload(id, Some(callback));
    }

    fn gate_drive_to_idle(&mut self) -> Result<DmoGateDriverTotals, Box<dyn Error>> {
        drive_to_idle(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn telemetry() -> Rc<RefCell<DmoGateRegistryTelemetry>> {
        Rc::new(RefCell::new(DmoGateRegistryTelemetry {
            events: Rc::new(RefCell::new(Vec::new())),
            asset_ids: Vec::new(),
            operation_count: 0,
            dmo_operation_count: 0,
            model_routes: None,
            assets_view: None,
            prepared_presentation: None,
            active: None,
        }))
    }

    fn direct_wrapper(
        registry: &mut DmoGateRegistry,
        id: CollectionId,
        collection: &OwnedResidentCollection<DmoBankProcessedArenaAsset>,
    ) -> DmoGateWrapper {
        let simulation = registry.simulation.create_wrapper(id, collection).unwrap();
        DmoGateWrapper {
            domain: Arc::downgrade(&registry.domain),
            simulation,
            token: 1,
        }
    }

    fn empty_collection(
        storage: &mut DmoGateStorage,
    ) -> OwnedResidentCollection<DmoBankProcessedArenaAsset> {
        storage.create_empty(0).unwrap()
    }

    #[test]
    fn foreign_composite_token_cannot_publish_pending_row() {
        let first_telemetry = telemetry();
        let second_telemetry = telemetry();
        let (mut first, _) = DmoGateRegistry::new(Rc::clone(&first_telemetry));
        let (mut second, _) = DmoGateRegistry::new(Rc::clone(&second_telemetry));
        let mut storage = DmoGateStorage::new(
            map_storage_error as fn(OwnedResidentCollectionError) -> DmoGateError,
        );
        let collection = empty_collection(&mut storage);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = direct_wrapper(&mut first, id, &collection);
        let second_wrapper = direct_wrapper(&mut second, id, &collection);
        let own = DmoGateRegistry::stable_token(&first_wrapper);
        let foreign = DmoGateRegistry::stable_token(&second_wrapper);
        assert_ne!(own.simulation, foreign.simulation);

        let error = first.publish(id, &foreign).unwrap_err();
        assert!(error.to_string().contains("another registry"));
        assert!(first_telemetry.borrow().active.is_none());
        assert!(first_telemetry.borrow().events.borrow().is_empty());
        first.publish(id, &own).unwrap();
        assert_eq!(first_telemetry.borrow().active, Some((id, 1)));
        first.remove_exact(id, &own).unwrap();

        drop((first_wrapper, second_wrapper, first, second));
        storage.destroy(collection);
    }

    #[test]
    fn foreign_composite_token_cannot_remove_active_row() {
        let first_telemetry = telemetry();
        let second_telemetry = telemetry();
        let (mut first, _) = DmoGateRegistry::new(Rc::clone(&first_telemetry));
        let (mut second, _) = DmoGateRegistry::new(Rc::clone(&second_telemetry));
        let mut storage = DmoGateStorage::new(
            map_storage_error as fn(OwnedResidentCollectionError) -> DmoGateError,
        );
        let collection = empty_collection(&mut storage);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = direct_wrapper(&mut first, id, &collection);
        let second_wrapper = direct_wrapper(&mut second, id, &collection);
        let own = DmoGateRegistry::stable_token(&first_wrapper);
        let foreign = DmoGateRegistry::stable_token(&second_wrapper);
        assert_ne!(own.simulation, foreign.simulation);
        first.publish(id, &own).unwrap();

        let error = first.remove_exact(id, &foreign).unwrap_err();
        assert!(error.to_string().contains("another registry"));
        assert_eq!(first_telemetry.borrow().active, Some((id, 1)));
        assert_eq!(
            first_telemetry.borrow().events.borrow().as_slice(),
            [DmoGateEvent::WrapperPublished(id)]
        );
        first.remove_exact(id, &own).unwrap();
        assert!(first_telemetry.borrow().active.is_none());

        drop((first_wrapper, second_wrapper, first, second));
        storage.destroy(collection);
    }
}
