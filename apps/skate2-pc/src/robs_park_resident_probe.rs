//! Diagnostic Rob's Park resident-collection composition.
//!
//! This module first drives the reconstructed resident streamer directly, then
//! runs a fresh parent-expanded composition through the reconstructed parent
//! manager. The outer collection scheduler remains unwired. Its raw-0 request
//! identity, serial polling, owned assets, weak wrapper registry, and telemetry
//! are safe PC probe policies, not native allocator or asset-manager
//! implementations. Presentation and Texture retain complete decoded arenas;
//! Simulation additionally applies the recovered collision/grind projection,
//! builds the supported normal Simulation subsequence, and routes its bounded
//! kind-6/kind-10 prefix. That adapter still runs at diagnostic build/Drop
//! timing rather than native message timing.

use skate2_core::{
    catalog_resident_load_factory::{
        CatalogResidentCollectionLoadDriver, CatalogResidentCollectionLoadFactory,
        CatalogResidentLoadError, CollectionAssetBuilderSessionFactory,
        CollectionRequestIdAllocator, CollectionRequestIdLease,
    },
    collection_load_pump::LoadedAssetCollection,
    collection_parent_manager::{
        CollectionParentEffects, CollectionParentManager, CollectionParentManagerError,
        ParentCollectionCallback, PlannedParentCallbackKind, PlannedParentRequestKind,
    },
    collection_resident_streamer::{
        CollectionId, CollectionResidentStreamer, ResidentAssetGarbageCollector,
        ResidentCallbackContext, ResidentCallbackObservation, ResidentCollectionCallback,
        ResidentCollectionLoadDriver, ResidentCollectionLoadFactory, ResidentCollectionMetadata,
        ResidentCollectionRegistry, ResidentStreamerUpdate,
    },
    decoded_arena_asset::{DecodedArenaAsset, DecodedArenaAssetBuilder, DecodedArenaAssetError},
    game_stream::StreamKind,
    mounted_collection_producer::{CollectionLogicalRangeSource, MountedCollectionStreamSnapshot},
    owned_resident_collection::{
        OwnedResidentCollection, OwnedResidentCollectionError, OwnedResidentCollectionStorage,
        OwnedResidentCollectionView, ResidentAssetIdentity,
    },
    resident_collection_load_driver::{CollectionAssetBuildInput, CollectionAssetBuilder},
    simulation_collision_asset::{
        SimulationCollisionAsset, SimulationCollisionAssetBuildError,
        SimulationCollisionAssetBuilder, SimulationCollisionAssetBuilderSessions,
        SimulationCollisionAssetSessionError,
    },
    simulation_environment_unlock_router::{
        EnvironmentUnlockRouterError, SimulationEnvironmentUnlockRouter,
    },
    simulation_normal_arena_plan::{
        SimulationNormalArenaPlan, SimulationNormalArenaPlanError, SimulationNormalOperation,
        SimulationRegistryAssetKind,
    },
    simulation_static_arena_adapter::{
        PreparedSimulationStaticArena, SimulationStaticArenaPrepareError,
        SimulationStaticArenaRegistration, SimulationStaticArenaRegistrationAdapter,
        SimulationStaticArenaRegistrationError, SimulationStaticGrindDispatch,
    },
    simulation_static_arena_plan::{
        SimulationStaticArenaPlan, SimulationStaticArenaPlanError, SimulationStaticOperation,
    },
    streaming::INVALID_COLLECTION_ID,
    world_stream_catalog::{
        AssetRegistrationSnapshot, CatalogCounts, CatalogError, CollectionRegistrationSnapshot,
        WorldStreamCatalog,
    },
};
use skate2_formats::{
    location::LocationTable,
    mount::{MountError, MountSet},
    pegasus::{
        C_MODEL_DATA_TYPE_ID, CollisionModelData, CollisionModelMeshes, INSTANCE_DATA_TYPE_ID,
        INSTANCE_DMO_ASSET_TYPE_ID, InstanceData, SPLINE_DATA_TYPE_ID, SplineData,
        TABLE_OF_CONTENTS_TYPE_ID, TRIGGER_INSTANCE_DATA_TYPE_ID, TableOfContents,
        TriggerInstanceData,
    },
    stream::{
        CollectionSpace, DEFAULT_PRESENTATION_FOCUS_COLLECTIONS,
        DEFAULT_SIMULATION_FOCUS_COLLECTIONS, DEFAULT_TEXTURE_FOCUS_COLLECTIONS,
        TEXTURE_FOCUS_RADIUS, WORLD_FOCUS_RADIUS,
    },
};
use std::{
    collections::VecDeque,
    error::Error,
    fmt, io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(120);
const FNV64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV64_PRIME: u64 = 0x0000_0100_0000_01b3;
const PRESENTATION_PARENT_ID: u64 = 0xe872_38ef_d7f0_2b45;
const PRESENTATION_PARENT_ASSET_COUNT: u32 = 35;
const PRESENTATION_PARENT_PAYLOAD_SIZE: u32 = 1_298_304;
const PRESENTATION_PARENT_STORED_BYTES: u64 = 1_291_608;
const PRESENTATION_PARENT_DECODED_BYTES: u64 = 3_485_480;
const PRESENTATION_PARENT_HIGHEST_STORED_BLOCK: u32 = 173_599;
const PRESENTATION_PARENT_DESCRIPTOR_HASH: u64 = 0x8120_a5fc_fe64_2d8d;
const PRESENTATION_PARENT_ASSET_HASH: u64 = 0x015d_8bde_e33f_f917;
const EXPANDED_PRESENTATION_ASSET_HASH: u64 = 0x282d_f7c3_c106_bc0e;
const EXPECTED_STATIC_GROUND_VOLUMES: usize = 26;

#[derive(Clone, Copy)]
struct ExpectedCollection {
    kind: StreamKind,
    id: u64,
    name: &'static str,
    asset_count: u32,
    payload_size: u32,
    parent_id: u64,
}

const EXPECTED_COLLECTIONS: [ExpectedCollection; 14] = [
    ExpectedCollection {
        kind: StreamKind::Presentation,
        id: 0xf14f_738b_2b3d_06a5,
        name: "cPres_500_900_high",
        asset_count: 576,
        payload_size: 5_104_512,
        parent_id: PRESENTATION_PARENT_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Presentation,
        id: 0x750c_1728_658d_7302,
        name: "cPres_500_1100_high",
        asset_count: 286,
        payload_size: 3_872_640,
        parent_id: PRESENTATION_PARENT_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Presentation,
        id: 0x72f1_6010_bbe6_718b,
        name: "cPres_700_900_high",
        asset_count: 509,
        payload_size: 5_600_512,
        parent_id: PRESENTATION_PARENT_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Presentation,
        id: 0x4edf_804a_09b2_8a94,
        name: "cPres_700_1100_high",
        asset_count: 337,
        payload_size: 6_619_648,
        parent_id: PRESENTATION_PARENT_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Presentation,
        id: 0x00f4_95df_3371_ff5f,
        name: "cPres_300_900_high",
        asset_count: 563,
        payload_size: 4_704_512,
        parent_id: PRESENTATION_PARENT_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Presentation,
        id: 0x749a_f9fe_bb82_3410,
        name: "cPres_300_1100_high",
        asset_count: 257,
        payload_size: 2_777_728,
        parent_id: PRESENTATION_PARENT_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Simulation,
        id: 0x823c_4449_fb31_63b4,
        name: "cSim_500_900_high",
        asset_count: 21,
        payload_size: 1_108_096,
        parent_id: INVALID_COLLECTION_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Simulation,
        id: 0x4874_96a5_4277_ffe1,
        name: "cSim_500_1100_high",
        asset_count: 14,
        payload_size: 1_422_080,
        parent_id: INVALID_COLLECTION_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Simulation,
        id: 0xfcf8_e2e9_2985_e3ae,
        name: "cSim_700_900_high",
        asset_count: 20,
        payload_size: 1_383_296,
        parent_id: INVALID_COLLECTION_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Simulation,
        id: 0x188d_56bf_e3d8_034f,
        name: "cSim_700_1100_high",
        asset_count: 13,
        payload_size: 2_983_936,
        parent_id: INVALID_COLLECTION_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Texture,
        id: 0xe956_b56e_c099_d8ef,
        name: "cTex_525_975_high",
        asset_count: 155,
        payload_size: 8_455_168,
        parent_id: INVALID_COLLECTION_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Texture,
        id: 0x9f88_e118_dc09_052b,
        name: "cTex_675_975_high",
        asset_count: 220,
        payload_size: 11_696_768,
        parent_id: INVALID_COLLECTION_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Texture,
        id: 0x463b_f6fd_0933_99a9,
        name: "cTex_525_1125_high",
        asset_count: 133,
        payload_size: 7_259_008,
        parent_id: INVALID_COLLECTION_ID,
    },
    ExpectedCollection {
        kind: StreamKind::Texture,
        id: 0x1454_1e27_3652_c0f5,
        name: "cTex_675_1125_high",
        asset_count: 177,
        payload_size: 11_142_016,
        parent_id: INVALID_COLLECTION_ID,
    },
];

const EXPECTED_HIGHEST_STORED_BLOCKS: [u32; 14] = [
    378_486, 475_265, 556_184, 658_214, 299_978, 517_168, 296_595, 484_465, 348_405, 734_333,
    451_218, 456_404, 470_288, 470_288,
];

#[derive(Clone, Copy)]
struct ExpectedKindTotals {
    collections: u32,
    assets: u32,
    stored_bytes: u64,
    decoded_bytes: u64,
    descriptor_hash: u64,
    asset_hash: u64,
}

const EXPECTED_KIND_TOTALS: [ExpectedKindTotals; 3] = [
    ExpectedKindTotals {
        collections: 6,
        assets: 2_528,
        stored_bytes: 28_203_546,
        decoded_bytes: 62_162_080,
        descriptor_hash: 0x9a13_cd9f_3c05_4061,
        asset_hash: 0x130b_afc8_3013_4c44,
    },
    ExpectedKindTotals {
        collections: 4,
        assets: 68,
        stored_bytes: 6_884_133,
        decoded_bytes: 11_164_760,
        descriptor_hash: 0x4597_d1e8_b52e_9f83,
        asset_hash: 0x93ce_bde3_07b9_f54e,
    },
    ExpectedKindTotals {
        collections: 4,
        assets: 685,
        stored_bytes: 38_420_692,
        decoded_bytes: 68_791_160,
        descriptor_hash: 0x40c2_fdbd_c3d2_571e,
        asset_hash: 0x34ba_9d3d_7686_264b,
    },
];

const LOADED_CATALOG_COUNTS: CatalogCounts = CatalogCounts {
    files: 3,
    collections: 907,
    asset_entries: 23_667,
    asset_references: 23_667,
    collection_spaces: 3,
    stream_xml_files: 2,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SimulationProjectionTotals {
    arenas: u64,
    roots: u64,
    primitive_sets: u64,
    primitives: u64,
    meshes: u64,
    triangles: u64,
    models: u64,
    instances: u64,
    collision_instances: u64,
    grind_assets: u64,
    grind_primitives: u64,
    dictionary_entries: u64,
}

const EXPECTED_SIMULATION_PROJECTION: SimulationProjectionTotals = SimulationProjectionTotals {
    arenas: 68,
    roots: 71,
    primitive_sets: 45,
    primitives: 451,
    meshes: 26,
    triangles: 354_382,
    models: 26,
    instances: 477,
    collision_instances: 26,
    grind_assets: 42,
    grind_primitives: 10_314,
    dictionary_entries: 954,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationStaticTotals {
    operations: u64,
    grind: u64,
    environment: u64,
    signature: u64,
}

impl Default for SimulationStaticTotals {
    fn default() -> Self {
        Self {
            operations: 0,
            grind: 0,
            environment: 0,
            signature: FNV64_OFFSET,
        }
    }
}

const EXPECTED_SIMULATION_STATIC: SimulationStaticTotals = SimulationStaticTotals {
    operations: 68,
    grind: 42,
    environment: 26,
    signature: 0x3941_3e2d_647f_8631,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationNormalTotals {
    operations: u64,
    grind: u64,
    environment: u64,
    dmo_instance: u64,
    trigger_instance: u64,
    world_painter: u64,
    nav_mesh: u64,
    rain: u64,
    ai_path: u64,
    waypoint: u64,
    spatial_map: u64,
    signature: u64,
}

impl Default for SimulationNormalTotals {
    fn default() -> Self {
        Self {
            operations: 0,
            grind: 0,
            environment: 0,
            dmo_instance: 0,
            trigger_instance: 0,
            world_painter: 0,
            nav_mesh: 0,
            rain: 0,
            ai_path: 0,
            waypoint: 0,
            spatial_map: 0,
            signature: FNV64_OFFSET,
        }
    }
}

const EXPECTED_SIMULATION_NORMAL: SimulationNormalTotals = SimulationNormalTotals {
    operations: 269,
    grind: 42,
    environment: 26,
    dmo_instance: 0,
    trigger_instance: 0,
    world_painter: 145,
    nav_mesh: 0,
    rain: 4,
    ai_path: 4,
    waypoint: 3,
    spatial_map: 45,
    signature: 0x415b_55e7_90aa_bfb0,
};

#[derive(Clone, Debug)]
struct KindBuildTelemetry {
    collections: u32,
    assets: u32,
    stored_bytes: u64,
    decoded_bytes: u64,
    asset_hash: u64,
}

impl Default for KindBuildTelemetry {
    fn default() -> Self {
        Self {
            collections: 0,
            assets: 0,
            stored_bytes: 0,
            decoded_bytes: 0,
            asset_hash: FNV64_OFFSET,
        }
    }
}

#[derive(Debug)]
struct BuildTelemetry {
    kinds: [KindBuildTelemetry; 3],
    asset_ids: Vec<CollectionId>,
    projection: SimulationProjectionTotals,
    simulation_static: SimulationStaticTotals,
    simulation_normal: SimulationNormalTotals,
    simulation_static_adapter: SimulationStaticArenaRegistrationAdapter<ProbeStaticGrindDispatch>,
    registered_static_operations: u64,
    deregistered_static_operations: u64,
    dropped_assets: u32,
}

impl Default for BuildTelemetry {
    fn default() -> Self {
        Self {
            kinds: std::array::from_fn(|_| KindBuildTelemetry::default()),
            asset_ids: Vec::new(),
            projection: SimulationProjectionTotals::default(),
            simulation_static: SimulationStaticTotals::default(),
            simulation_normal: SimulationNormalTotals::default(),
            simulation_static_adapter: SimulationStaticArenaRegistrationAdapter::new(
                ProbeStaticGrindDispatch::default(),
                SimulationEnvironmentUnlockRouter::try_new_empty(
                    EXPECTED_STATIC_GROUND_VOLUMES,
                )
                .expect(
                    "the fixed Rob's Park diagnostic Environment router allocation must succeed",
                ),
            ),
            registered_static_operations: 0,
            deregistered_static_operations: 0,
            dropped_assets: 0,
        }
    }
}

#[derive(Debug, Default)]
struct ProbeStaticGrindDispatch {
    added_ids: Vec<u64>,
    removed_ids: Vec<u64>,
}

impl SimulationStaticGrindDispatch for ProbeStaticGrindDispatch {
    fn add_spline_data(&mut self, asset: &skate2_core::grind::GrindAsset) {
        self.added_ids.push(asset.asset_id);
    }

    fn remove_spline_data(&mut self, asset_id: u64) {
        self.removed_ids.push(asset_id);
    }
}

enum ProbeResidentAsset {
    Presentation {
        decoded: DecodedArenaAsset,
        telemetry: Arc<Mutex<BuildTelemetry>>,
    },
    Simulation {
        plan: Arc<SimulationNormalArenaPlan>,
        registration: Option<SimulationStaticArenaRegistration>,
        telemetry: Arc<Mutex<BuildTelemetry>>,
    },
    Texture {
        decoded: DecodedArenaAsset,
        telemetry: Arc<Mutex<BuildTelemetry>>,
    },
}

impl ProbeResidentAsset {
    fn decoded(&self) -> &DecodedArenaAsset {
        match self {
            Self::Presentation { decoded, .. } | Self::Texture { decoded, .. } => decoded,
            Self::Simulation { plan, .. } => &plan.static_plan().asset().decoded,
        }
    }

    fn telemetry(&self) -> &Arc<Mutex<BuildTelemetry>> {
        match self {
            Self::Presentation { telemetry, .. }
            | Self::Simulation { telemetry, .. }
            | Self::Texture { telemetry, .. } => telemetry,
        }
    }
}

impl ResidentAssetIdentity for ProbeResidentAsset {
    fn resident_asset_id(&self) -> CollectionId {
        CollectionId(self.decoded().asset_id)
    }
}

impl Drop for ProbeResidentAsset {
    fn drop(&mut self) {
        let telemetry_owner = Arc::clone(self.telemetry());
        let mut telemetry = telemetry_owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Self::Simulation { registration, .. } = self {
            if let Some(registration) = registration.take() {
                let operation_count = registration.processed_operation_count();
                telemetry
                    .simulation_static_adapter
                    .deregister(registration)
                    .expect("resident registration must return to its owning adapter exactly once");
                let operation_count = u64::try_from(operation_count)
                    .expect("supported operation count must fit diagnostic u64 telemetry");
                telemetry.deregistered_static_operations = telemetry
                    .deregistered_static_operations
                    .checked_add(operation_count)
                    .expect("supported deregistration telemetry must not overflow");
            }
        }
        telemetry.dropped_assets = telemetry.dropped_assets.wrapping_add(1);
    }
}

#[derive(Debug)]
enum ProbeBuilderInner {
    Decoded(DecodedArenaAssetBuilder),
    Simulation(SimulationCollisionAssetBuilder),
}

#[derive(Debug)]
struct ProbeAssetBuilder {
    kind: StreamKind,
    inner: ProbeBuilderInner,
    telemetry: Arc<Mutex<BuildTelemetry>>,
}

impl CollectionAssetBuilder<AssetRegistrationSnapshot> for ProbeAssetBuilder {
    type Asset = ProbeResidentAsset;
    type Error = ProbeBuilderError;

    fn build(
        &mut self,
        input: CollectionAssetBuildInput<'_, AssetRegistrationSnapshot>,
    ) -> Result<Option<Self::Asset>, Self::Error> {
        let stored_size = input.stored_size;
        let built = match &mut self.inner {
            ProbeBuilderInner::Decoded(builder) => {
                let decoded = builder
                    .build(input)
                    .map_err(ProbeBuilderError::Decoded)?
                    .ok_or(ProbeBuilderError::UnexpectedNullAsset)?;
                observe_decoded(&self.telemetry, self.kind, stored_size, &decoded, None)?;
                match self.kind {
                    StreamKind::Presentation => ProbeResidentAsset::Presentation {
                        decoded,
                        telemetry: Arc::clone(&self.telemetry),
                    },
                    StreamKind::Texture => ProbeResidentAsset::Texture {
                        decoded,
                        telemetry: Arc::clone(&self.telemetry),
                    },
                    StreamKind::Simulation => {
                        unreachable!("canonical Simulation sessions install the projected builder")
                    }
                }
            }
            ProbeBuilderInner::Simulation(builder) => {
                let projected = builder
                    .build(input)
                    .map_err(ProbeBuilderError::Simulation)?
                    .ok_or(ProbeBuilderError::UnexpectedNullAsset)?;
                let static_plan = Arc::new(
                    SimulationStaticArenaPlan::from_collision_asset(projected)
                        .map_err(ProbeBuilderError::SimulationStatic)?,
                );
                let prepared = PreparedSimulationStaticArena::try_new(Arc::clone(&static_plan))
                    .map_err(ProbeBuilderError::SimulationStaticPrepare)?;
                let plan = Arc::new(
                    SimulationNormalArenaPlan::from_static_plan(static_plan)
                        .map_err(ProbeBuilderError::SimulationNormal)?,
                );
                let operation_count = u64::try_from(prepared.operation_count())
                    .map_err(|_| ProbeBuilderError::TelemetryOverflow)?;
                observe_decoded(
                    &self.telemetry,
                    self.kind,
                    stored_size,
                    &plan.static_plan().asset().decoded,
                    Some(&plan),
                )?;
                let registration = {
                    let mut telemetry = self
                        .telemetry
                        .lock()
                        .map_err(|_| ProbeBuilderError::TelemetryPoisoned)?;
                    let next_registered_static_operations = telemetry
                        .registered_static_operations
                        .checked_add(operation_count)
                        .ok_or(ProbeBuilderError::TelemetryOverflow)?;
                    match telemetry.simulation_static_adapter.register(&prepared, 0) {
                        Ok(registration) => {
                            telemetry.registered_static_operations =
                                next_registered_static_operations;
                            registration
                        }
                        Err(SimulationStaticArenaRegistrationError::Environment {
                            operation_index,
                            registration,
                            source,
                        }) => {
                            // Host capacity failure has no native rollback
                            // contract. The adapter remains faulted with its
                            // partial ledger entry; dropping this authority
                            // performs no potentially ambiguous removal.
                            drop(registration);
                            return Err(ProbeBuilderError::SimulationStaticRegistration {
                                operation_index,
                                source,
                            });
                        }
                        Err(source) => {
                            return Err(ProbeBuilderError::SimulationStaticRegistrationStart(
                                source,
                            ));
                        }
                    }
                };
                ProbeResidentAsset::Simulation {
                    plan,
                    registration: Some(registration),
                    telemetry: Arc::clone(&self.telemetry),
                }
            }
        };
        Ok(Some(built))
    }
}

fn observe_decoded(
    telemetry: &Arc<Mutex<BuildTelemetry>>,
    kind: StreamKind,
    stored_size: u32,
    decoded: &DecodedArenaAsset,
    simulation: Option<&SimulationNormalArenaPlan>,
) -> Result<(), ProbeBuilderError> {
    let decoded_bytes = decoded
        .resources
        .resources
        .iter()
        .try_fold(0_u64, |total, resource| {
            let length =
                u64::try_from(resource.len()).map_err(|_| ProbeBuilderError::TelemetryOverflow)?;
            total
                .checked_add(length)
                .ok_or(ProbeBuilderError::TelemetryOverflow)
        })?;
    let mut telemetry = telemetry
        .lock()
        .map_err(|_| ProbeBuilderError::TelemetryPoisoned)?;
    let index = kind_index(kind);
    let item = &mut telemetry.kinds[index];
    item.assets = item
        .assets
        .checked_add(1)
        .ok_or(ProbeBuilderError::TelemetryOverflow)?;
    item.stored_bytes = item
        .stored_bytes
        .checked_add(u64::from(stored_size))
        .ok_or(ProbeBuilderError::TelemetryOverflow)?;
    item.decoded_bytes = item
        .decoded_bytes
        .checked_add(decoded_bytes)
        .ok_or(ProbeBuilderError::TelemetryOverflow)?;
    fnv_u64(&mut item.asset_hash, decoded.asset_id);
    telemetry.asset_ids.push(CollectionId(decoded.asset_id));
    if let Some(plan) = simulation {
        add_projection(&mut telemetry.projection, plan.static_plan().asset())?;
        add_simulation_static(&mut telemetry.simulation_static, plan.static_plan())?;
        add_simulation_normal(&mut telemetry.simulation_normal, plan)?;
    }
    Ok(())
}

fn add_simulation_static(
    totals: &mut SimulationStaticTotals,
    plan: &SimulationStaticArenaPlan,
) -> Result<(), ProbeBuilderError> {
    let asset = plan.asset();
    let arena = &asset.decoded.arena;
    let resources = &asset.decoded.resources.resources;
    static_fnv_bytes(&mut totals.signature, b"A");
    static_fnv_u64(&mut totals.signature, asset.decoded.asset_id);
    static_fnv_u32(&mut totals.signature, arena.id);
    let toc = arena
        .dictionary
        .iter()
        .find(|entry| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID)
        .map(|entry| {
            let bytes = arena.object_bytes(resources, entry).ok_or_else(|| {
                ProbeBuilderError::SimulationStaticSignature(
                    "TOC object lies outside its retained resource".to_owned(),
                )
            })?;
            TableOfContents::parse(bytes).map_err(|source| {
                ProbeBuilderError::SimulationStaticSignature(format!(
                    "cannot parse signature TOC: {source}"
                ))
            })
        })
        .transpose()?;

    for operation in plan.operations() {
        totals.operations = totals
            .operations
            .checked_add(1)
            .ok_or(ProbeBuilderError::TelemetryOverflow)?;
        match operation {
            SimulationStaticOperation::Grind {
                source_toc_index,
                source_dictionary_index,
                grind_asset_index,
            } => {
                totals.grind = totals
                    .grind
                    .checked_add(1)
                    .ok_or(ProbeBuilderError::TelemetryOverflow)?;
                hash_static_grind(
                    &mut totals.signature,
                    plan,
                    toc.as_ref(),
                    *source_toc_index,
                    *source_dictionary_index,
                    *grind_asset_index,
                )?;
            }
            SimulationStaticOperation::Environment {
                source_toc_index,
                source_dictionary_index,
                source_inline_index,
                instance_index,
                ..
            } => {
                totals.environment = totals
                    .environment
                    .checked_add(1)
                    .ok_or(ProbeBuilderError::TelemetryOverflow)?;
                hash_static_environment(
                    &mut totals.signature,
                    plan,
                    toc.as_ref(),
                    *source_toc_index,
                    *source_dictionary_index,
                    *source_inline_index,
                    *instance_index,
                )?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Keep the complete frozen descriptor schema auditable together.
fn add_simulation_normal(
    totals: &mut SimulationNormalTotals,
    plan: &SimulationNormalArenaPlan,
) -> Result<(), ProbeBuilderError> {
    let static_plan = plan.static_plan();
    let asset = static_plan.asset();
    let arena = &asset.decoded.arena;
    let resources = &asset.decoded.resources.resources;
    static_fnv_bytes(&mut totals.signature, b"A");
    static_fnv_u64(&mut totals.signature, asset.decoded.asset_id);
    static_fnv_u32(&mut totals.signature, arena.id);
    let toc = arena
        .dictionary
        .iter()
        .find(|entry| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID)
        .map(|entry| {
            let bytes = arena.object_bytes(resources, entry).ok_or_else(|| {
                ProbeBuilderError::SimulationNormalSignature(
                    "TOC object lies outside its retained resource".to_owned(),
                )
            })?;
            TableOfContents::parse(bytes).map_err(|source| {
                ProbeBuilderError::SimulationNormalSignature(format!(
                    "cannot parse normal-plan signature TOC: {source}"
                ))
            })
        })
        .transpose()?;

    for (operation_index, operation) in plan.operations().iter().enumerate() {
        let (kind, asset_id, toc_index, dictionary_index, inline_index, raw_type, raw_id) =
            match *operation {
                SimulationNormalOperation::Grind {
                    static_operation_index,
                } => {
                    let operation = static_plan
                        .operations()
                        .get(static_operation_index)
                        .ok_or_else(|| {
                            ProbeBuilderError::SimulationNormalSignature(
                                "normal grind operation indexes missing static operation"
                                    .to_owned(),
                            )
                        })?;
                    let SimulationStaticOperation::Grind {
                        source_toc_index,
                        source_dictionary_index,
                        grind_asset_index,
                    } = *operation
                    else {
                        return Err(ProbeBuilderError::SimulationNormalSignature(
                            "normal grind operation indexes a non-grind static operation"
                                .to_owned(),
                        ));
                    };
                    let grind = static_plan
                        .grind_assets()
                        .get(grind_asset_index)
                        .ok_or_else(|| {
                            ProbeBuilderError::SimulationNormalSignature(
                                "normal grind operation indexes missing grind asset".to_owned(),
                            )
                        })?;
                    let entry = normal_toc_entry(toc.as_ref(), source_toc_index)?;
                    validate_normal_source(
                        arena,
                        resources,
                        entry,
                        source_dictionary_index,
                        SPLINE_DATA_TYPE_ID,
                    )?;
                    if entry.guid != grind.asset_id {
                        return Err(ProbeBuilderError::SimulationNormalSignature(
                            "normal grind ID disagrees with its direct TOC GUID".to_owned(),
                        ));
                    }
                    (
                        6,
                        grind.asset_id,
                        source_toc_index,
                        source_dictionary_index,
                        u32::MAX,
                        entry.object_type,
                        entry.object.0,
                    )
                }
                SimulationNormalOperation::Environment {
                    static_operation_index,
                } => {
                    let operation = static_plan
                        .operations()
                        .get(static_operation_index)
                        .ok_or_else(|| {
                            ProbeBuilderError::SimulationNormalSignature(
                                "normal Environment operation indexes missing static operation"
                                    .to_owned(),
                            )
                        })?;
                    let SimulationStaticOperation::Environment {
                        source_toc_index,
                        source_dictionary_index,
                        source_inline_index,
                        instance_index,
                        ..
                    } = *operation
                    else {
                        return Err(ProbeBuilderError::SimulationNormalSignature(
                            "normal Environment operation indexes a non-Environment static operation"
                                .to_owned(),
                        ));
                    };
                    let instance =
                        asset
                            .collision
                            .instances
                            .get(instance_index)
                            .ok_or_else(|| {
                                ProbeBuilderError::SimulationNormalSignature(
                            "normal Environment operation indexes missing collision instance"
                                .to_owned(),
                        )
                            })?;
                    let entry = normal_toc_entry(toc.as_ref(), source_toc_index)?;
                    validate_normal_source(
                        arena,
                        resources,
                        entry,
                        source_dictionary_index,
                        INSTANCE_DATA_TYPE_ID,
                    )?;
                    (
                        10,
                        instance.guid,
                        source_toc_index,
                        source_dictionary_index,
                        source_inline_index,
                        entry.object_type,
                        entry.object.0,
                    )
                }
                SimulationNormalOperation::DmoInstance {
                    asset_id,
                    local_guid,
                    source_toc_index,
                    source_dictionary_index,
                    source_inline_index,
                    source_record_offset,
                    raw_object_id,
                } => {
                    let entry = normal_toc_entry(toc.as_ref(), source_toc_index)?;
                    validate_normal_source(
                        arena,
                        resources,
                        entry,
                        source_dictionary_index,
                        INSTANCE_DATA_TYPE_ID,
                    )?;
                    let dictionary = normal_dictionary_entry(
                        arena,
                        source_dictionary_index,
                        INSTANCE_DATA_TYPE_ID,
                    )?;
                    let bytes = arena.object_bytes(resources, dictionary).ok_or_else(|| {
                        ProbeBuilderError::SimulationNormalSignature(
                            "DMO instance container lies outside retained storage".to_owned(),
                        )
                    })?;
                    let records = InstanceData::record_table(bytes).map_err(|source| {
                        ProbeBuilderError::SimulationNormalSignature(format!(
                            "cannot parse DMO instance record table: {source}"
                        ))
                    })?;
                    let record = records
                        .record(source_inline_index as usize)
                        .ok_or_else(|| {
                            ProbeBuilderError::SimulationNormalSignature(
                                "DMO operation indexes a missing inline record".to_owned(),
                            )
                        })?;
                    let retained = plan.record_source(operation_index).ok_or_else(|| {
                        ProbeBuilderError::SimulationNormalSignature(
                            "DMO operation has no owner-bound record source".to_owned(),
                        )
                    })?;
                    if records.type_id() != INSTANCE_DMO_ASSET_TYPE_ID
                        || entry.object.0 != raw_object_id
                        || record.object_offset() != source_record_offset
                        || record.guid() != asset_id
                        || record.local_guid() != local_guid
                        || retained.bytes() != record.bytes()
                    {
                        return Err(ProbeBuilderError::SimulationNormalSignature(
                            "DMO operation disagrees with its retained source record".to_owned(),
                        ));
                    }
                    (
                        11,
                        asset_id,
                        source_toc_index,
                        source_dictionary_index,
                        source_inline_index,
                        INSTANCE_DATA_TYPE_ID,
                        raw_object_id,
                    )
                }
                SimulationNormalOperation::TriggerInstance {
                    asset_id,
                    primary_guid,
                    source_toc_index,
                    source_dictionary_index,
                    source_inline_index,
                    source_record_offset,
                    raw_object_id,
                } => {
                    let entry = normal_toc_entry(toc.as_ref(), source_toc_index)?;
                    validate_normal_source(
                        arena,
                        resources,
                        entry,
                        source_dictionary_index,
                        TRIGGER_INSTANCE_DATA_TYPE_ID,
                    )?;
                    let dictionary = normal_dictionary_entry(
                        arena,
                        source_dictionary_index,
                        TRIGGER_INSTANCE_DATA_TYPE_ID,
                    )?;
                    let bytes = arena.object_bytes(resources, dictionary).ok_or_else(|| {
                        ProbeBuilderError::SimulationNormalSignature(
                            "trigger container lies outside retained storage".to_owned(),
                        )
                    })?;
                    let records = TriggerInstanceData::record_table(bytes).map_err(|source| {
                        ProbeBuilderError::SimulationNormalSignature(format!(
                            "cannot parse trigger record table: {source}"
                        ))
                    })?;
                    let record = records
                        .record(source_inline_index as usize)
                        .ok_or_else(|| {
                            ProbeBuilderError::SimulationNormalSignature(
                                "trigger operation indexes a missing inline record".to_owned(),
                            )
                        })?;
                    let retained = plan.record_source(operation_index).ok_or_else(|| {
                        ProbeBuilderError::SimulationNormalSignature(
                            "trigger operation has no owner-bound record source".to_owned(),
                        )
                    })?;
                    if entry.object.0 != raw_object_id
                        || record.object_offset() != source_record_offset
                        || record.local_guid() != asset_id
                        || record.guid() != primary_guid
                        || retained.bytes() != record.bytes()
                    {
                        return Err(ProbeBuilderError::SimulationNormalSignature(
                            "trigger operation disagrees with its retained source record"
                                .to_owned(),
                        ));
                    }
                    (
                        16,
                        asset_id,
                        source_toc_index,
                        source_dictionary_index,
                        source_inline_index,
                        TRIGGER_INSTANCE_DATA_TYPE_ID,
                        raw_object_id,
                    )
                }
                SimulationNormalOperation::RegistryAsset {
                    kind,
                    asset_id,
                    source_toc_index,
                    source_dictionary_index,
                    raw_object_type,
                    raw_object_id,
                } => {
                    let entry = normal_toc_entry(toc.as_ref(), source_toc_index)?;
                    validate_normal_source(
                        arena,
                        resources,
                        entry,
                        source_dictionary_index,
                        raw_object_type,
                    )?;
                    if entry.guid != asset_id
                        || entry.object_type != raw_object_type
                        || entry.object.0 != raw_object_id
                        || !normal_kind_matches_type(kind, raw_object_type)
                    {
                        return Err(ProbeBuilderError::SimulationNormalSignature(
                            "normal registry operation disagrees with its TOC source".to_owned(),
                        ));
                    }
                    (
                        kind as u32,
                        asset_id,
                        source_toc_index,
                        source_dictionary_index,
                        u32::MAX,
                        raw_object_type,
                        raw_object_id,
                    )
                }
                _ => {
                    return Err(ProbeBuilderError::SimulationNormalSignature(
                        "normal plan contains an operation unsupported by this frozen probe"
                            .to_owned(),
                    ));
                }
            };
        totals.operations = totals
            .operations
            .checked_add(1)
            .ok_or(ProbeBuilderError::TelemetryOverflow)?;
        let kind_total = match kind {
            6 => &mut totals.grind,
            10 => &mut totals.environment,
            11 => &mut totals.dmo_instance,
            16 => &mut totals.trigger_instance,
            22 => &mut totals.world_painter,
            23 => &mut totals.nav_mesh,
            25 => &mut totals.rain,
            27 => &mut totals.ai_path,
            28 => &mut totals.waypoint,
            32 => &mut totals.spatial_map,
            _ => {
                return Err(ProbeBuilderError::SimulationNormalSignature(format!(
                    "unsupported normal Simulation kind {kind}"
                )));
            }
        };
        *kind_total = kind_total
            .checked_add(1)
            .ok_or(ProbeBuilderError::TelemetryOverflow)?;
        static_fnv_bytes(&mut totals.signature, b"O");
        static_fnv_u32(&mut totals.signature, kind);
        static_fnv_u64(&mut totals.signature, asset_id);
        static_fnv_u32(&mut totals.signature, toc_index);
        static_fnv_u32(&mut totals.signature, dictionary_index);
        static_fnv_u32(&mut totals.signature, inline_index);
        static_fnv_u32(&mut totals.signature, raw_type);
        static_fnv_u32(&mut totals.signature, raw_id);
    }
    Ok(())
}

fn validate_normal_source(
    arena: &skate2_formats::rw4::Arena,
    resources: &[Vec<u8>; 5],
    entry: &skate2_formats::pegasus::TocEntry,
    dictionary_index: u32,
    expected_type: u32,
) -> Result<(), ProbeBuilderError> {
    if entry.object.section_index() != 0 || entry.object.entry_index() != dictionary_index {
        return Err(ProbeBuilderError::SimulationNormalSignature(
            "normal operation disagrees with its local TOC target".to_owned(),
        ));
    }
    if entry.object_type != expected_type {
        return Err(ProbeBuilderError::SimulationNormalSignature(format!(
            "normal operation expected TOC type {expected_type:08x}, got {:08x}",
            entry.object_type
        )));
    }
    let dictionary = normal_dictionary_entry(arena, dictionary_index, expected_type)?;
    arena.object_bytes(resources, dictionary).ok_or_else(|| {
        ProbeBuilderError::SimulationNormalSignature(
            "normal operation target lies outside its retained resource".to_owned(),
        )
    })?;
    Ok(())
}

fn normal_toc_entry(
    toc: Option<&TableOfContents>,
    source_toc_index: u32,
) -> Result<&skate2_formats::pegasus::TocEntry, ProbeBuilderError> {
    toc.and_then(|toc| toc.entries.get(source_toc_index as usize))
        .ok_or_else(|| {
            ProbeBuilderError::SimulationNormalSignature(
                "normal operation indexes a missing TOC entry".to_owned(),
            )
        })
}

fn normal_dictionary_entry(
    arena: &skate2_formats::rw4::Arena,
    dictionary_index: u32,
    expected_type: u32,
) -> Result<&skate2_formats::rw4::DictionaryEntry, ProbeBuilderError> {
    let entry = arena
        .dictionary
        .get(dictionary_index as usize)
        .ok_or_else(|| {
            ProbeBuilderError::SimulationNormalSignature(
                "normal operation indexes a missing dictionary entry".to_owned(),
            )
        })?;
    if entry.type_id != expected_type {
        return Err(ProbeBuilderError::SimulationNormalSignature(format!(
            "normal operation expected dictionary type {expected_type:08x}, got {:08x}",
            entry.type_id
        )));
    }
    Ok(entry)
}

const fn normal_kind_matches_type(kind: SimulationRegistryAssetKind, raw_type: u32) -> bool {
    use skate2_core::simulation_normal_arena_plan::{
        AI_PATH_DATA_TYPE_ID, NAV_MESH_2_DATA_TYPE_ID, RAIN_DATA_TYPE_ID, SPATIAL_MAP_DATA_TYPE_ID,
        WAYPOINT_DATA_TYPE_ID, WORLD_PAINTER_DATA_TYPE_ID_0F, WORLD_PAINTER_DATA_TYPE_ID_10,
        WORLD_PAINTER_DATA_TYPE_ID_11,
    };
    match kind {
        SimulationRegistryAssetKind::WorldPainter => matches!(
            raw_type,
            WORLD_PAINTER_DATA_TYPE_ID_0F
                | WORLD_PAINTER_DATA_TYPE_ID_10
                | WORLD_PAINTER_DATA_TYPE_ID_11
        ),
        SimulationRegistryAssetKind::NavMesh => raw_type == NAV_MESH_2_DATA_TYPE_ID,
        SimulationRegistryAssetKind::Rain => raw_type == RAIN_DATA_TYPE_ID,
        SimulationRegistryAssetKind::AiPath => raw_type == AI_PATH_DATA_TYPE_ID,
        SimulationRegistryAssetKind::Waypoint => raw_type == WAYPOINT_DATA_TYPE_ID,
        SimulationRegistryAssetKind::SpatialMap => raw_type == SPATIAL_MAP_DATA_TYPE_ID,
    }
}

fn hash_static_grind(
    signature: &mut u64,
    plan: &SimulationStaticArenaPlan,
    toc: Option<&TableOfContents>,
    source_toc_index: u32,
    source_dictionary_index: u32,
    grind_asset_index: usize,
) -> Result<(), ProbeBuilderError> {
    let asset = plan.asset();
    let arena = &asset.decoded.arena;
    let resources = &asset.decoded.resources.resources;
    let entry = signature_toc_entry(toc, source_toc_index)?;
    let dictionary =
        signature_dictionary_entry(arena, source_dictionary_index, SPLINE_DATA_TYPE_ID)?;
    if entry.object.entry_index() != source_dictionary_index || entry.object.section_index() != 0 {
        return Err(ProbeBuilderError::SimulationStaticSignature(
            "grind operation disagrees with its TOC target".to_owned(),
        ));
    }
    let spline = SplineData::parse(arena.object_bytes(resources, dictionary).ok_or_else(|| {
        ProbeBuilderError::SimulationStaticSignature(
            "spline object lies outside its retained resource".to_owned(),
        )
    })?)
    .map_err(|source| {
        ProbeBuilderError::SimulationStaticSignature(format!(
            "cannot parse signature spline: {source}"
        ))
    })?;
    let grind = plan.grind_assets().get(grind_asset_index).ok_or_else(|| {
        ProbeBuilderError::SimulationStaticSignature(
            "grind operation indexes a missing native grind asset".to_owned(),
        )
    })?;
    if grind.asset_id != entry.guid
        || grind.source_dictionary_index != source_dictionary_index
        || grind.primitives.iter().any(|primitive| {
            primitive.source_asset_id != grind.asset_id
                || primitive.source_dictionary_index != source_dictionary_index
        })
    {
        return Err(ProbeBuilderError::SimulationStaticSignature(
            "native grind identity does not match its source locator".to_owned(),
        ));
    }
    static_fnv_bytes(signature, b"G");
    static_fnv_u64(signature, grind.asset_id);
    static_fnv_bytes(signature, entry.name.as_bytes());
    static_fnv_bytes(signature, &[0]);
    static_fnv_u32(signature, entry.object.0);
    static_fnv_u32(
        signature,
        u32::try_from(spline.splines.len()).map_err(|_| ProbeBuilderError::TelemetryOverflow)?,
    );
    static_fnv_u32(
        signature,
        u32::try_from(spline.segments.len()).map_err(|_| ProbeBuilderError::TelemetryOverflow)?,
    );
    Ok(())
}

fn hash_static_environment(
    signature: &mut u64,
    plan: &SimulationStaticArenaPlan,
    toc: Option<&TableOfContents>,
    source_toc_index: u32,
    source_dictionary_index: u32,
    source_inline_index: u32,
    instance_index: usize,
) -> Result<(), ProbeBuilderError> {
    let asset = plan.asset();
    let arena = &asset.decoded.arena;
    let resources = &asset.decoded.resources.resources;
    let entry = signature_toc_entry(toc, source_toc_index)?;
    let dictionary =
        signature_dictionary_entry(arena, source_dictionary_index, INSTANCE_DATA_TYPE_ID)?;
    if entry.object.entry_index() != source_dictionary_index || entry.object.section_index() != 0 {
        return Err(ProbeBuilderError::SimulationStaticSignature(
            "Environment operation disagrees with its TOC target".to_owned(),
        ));
    }
    let instances =
        InstanceData::parse(arena.object_bytes(resources, dictionary).ok_or_else(|| {
            ProbeBuilderError::SimulationStaticSignature(
                "instance object lies outside its retained resource".to_owned(),
            )
        })?)
        .map_err(|source| {
            ProbeBuilderError::SimulationStaticSignature(format!(
                "cannot parse signature instances: {source}"
            ))
        })?;
    let source = instances
        .instances
        .get(
            usize::try_from(source_inline_index)
                .map_err(|_| ProbeBuilderError::TelemetryOverflow)?,
        )
        .ok_or_else(|| {
            ProbeBuilderError::SimulationStaticSignature(
                "Environment operation indexes a missing inline instance".to_owned(),
            )
        })?;
    let projected = asset
        .collision
        .instances
        .get(instance_index)
        .ok_or_else(|| {
            ProbeBuilderError::SimulationStaticSignature(
                "Environment operation indexes a missing projected instance".to_owned(),
            )
        })?;
    if projected.guid != source.guid || projected.local_guid != source.local_guid {
        return Err(ProbeBuilderError::SimulationStaticSignature(
            "Environment projected identity disagrees with its source".to_owned(),
        ));
    }
    static_fnv_bytes(signature, b"E");
    static_fnv_u64(signature, projected.guid);
    static_fnv_u64(signature, projected.local_guid);
    static_fnv_u32(signature, entry.object.0);
    static_fnv_u32(signature, source_inline_index);
    static_fnv_u32(signature, source.render_model.map_or(0, |id| id.0));
    static_fnv_u32(signature, source.collision_model.map_or(0, |id| id.0));
    hash_static_collision_roots(signature, arena, resources, source.collision_model)
}

fn hash_static_collision_roots(
    signature: &mut u64,
    arena: &skate2_formats::rw4::Arena,
    resources: &[Vec<u8>; 5],
    collision_model: Option<skate2_formats::pegasus::ObjectId>,
) -> Result<(), ProbeBuilderError> {
    let collision_id = collision_model.ok_or_else(|| {
        ProbeBuilderError::SimulationStaticSignature(
            "frozen Environment corpus item has no collision model".to_owned(),
        )
    })?;
    if collision_id.section_index() != 0 {
        return Err(ProbeBuilderError::SimulationStaticSignature(
            "signature collision model is nonlocal".to_owned(),
        ));
    }
    let dictionary =
        signature_dictionary_entry(arena, collision_id.entry_index(), C_MODEL_DATA_TYPE_ID)?;
    let model =
        CollisionModelData::parse(arena.object_bytes(resources, dictionary).ok_or_else(|| {
            ProbeBuilderError::SimulationStaticSignature(
                "collision model lies outside its retained resource".to_owned(),
            )
        })?)
        .map_err(|source| {
            ProbeBuilderError::SimulationStaticSignature(format!(
                "cannot parse signature collision model: {source}"
            ))
        })?;
    match model.meshes {
        CollisionModelMeshes::Assembly(id) => {
            static_fnv_u32(signature, u32::MAX);
            static_fnv_u32(signature, id.0);
        }
        CollisionModelMeshes::Meshes(meshes) => {
            static_fnv_u32(
                signature,
                u32::try_from(meshes.len()).map_err(|_| ProbeBuilderError::TelemetryOverflow)?,
            );
            for mesh in meshes {
                static_fnv_u32(signature, mesh.mesh.0);
            }
        }
    }
    Ok(())
}

fn signature_toc_entry(
    toc: Option<&TableOfContents>,
    source_toc_index: u32,
) -> Result<&skate2_formats::pegasus::TocEntry, ProbeBuilderError> {
    toc.and_then(|toc| toc.entries.get(source_toc_index as usize))
        .ok_or_else(|| {
            ProbeBuilderError::SimulationStaticSignature(
                "static operation indexes a missing TOC entry".to_owned(),
            )
        })
}

fn signature_dictionary_entry(
    arena: &skate2_formats::rw4::Arena,
    dictionary_index: u32,
    expected_type: u32,
) -> Result<&skate2_formats::rw4::DictionaryEntry, ProbeBuilderError> {
    let entry = arena
        .dictionary
        .get(dictionary_index as usize)
        .ok_or_else(|| {
            ProbeBuilderError::SimulationStaticSignature(
                "static operation indexes a missing dictionary entry".to_owned(),
            )
        })?;
    if entry.type_id != expected_type {
        return Err(ProbeBuilderError::SimulationStaticSignature(format!(
            "static operation expected dictionary type {expected_type:08x}, got {:08x}",
            entry.type_id
        )));
    }
    Ok(entry)
}

fn add_projection(
    totals: &mut SimulationProjectionTotals,
    asset: &SimulationCollisionAsset,
) -> Result<(), ProbeBuilderError> {
    let collision = &asset.collision;
    let add = |target: &mut u64, value: usize| -> Result<(), ProbeBuilderError> {
        let value = u64::try_from(value).map_err(|_| ProbeBuilderError::TelemetryOverflow)?;
        *target = target
            .checked_add(value)
            .ok_or(ProbeBuilderError::TelemetryOverflow)?;
        Ok(())
    };
    add(&mut totals.arenas, 1)?;
    add(&mut totals.roots, collision.aggregate_roots.len())?;
    add(&mut totals.primitive_sets, collision.primitive_sets.len())?;
    add(
        &mut totals.primitives,
        collision
            .primitive_sets
            .iter()
            .map(|set| set.volumes.len())
            .sum(),
    )?;
    add(&mut totals.meshes, collision.meshes.len())?;
    add(
        &mut totals.triangles,
        collision
            .meshes
            .iter()
            .map(|mesh| mesh.triangles.len())
            .sum(),
    )?;
    add(&mut totals.models, collision.collision_models.len())?;
    add(&mut totals.instances, collision.instances.len())?;
    add(
        &mut totals.collision_instances,
        collision
            .instances
            .iter()
            .filter(|instance| instance.collision_model.is_some())
            .count(),
    )?;
    add(&mut totals.grind_assets, collision.grind_assets.len())?;
    add(
        &mut totals.grind_primitives,
        collision
            .grind_assets
            .iter()
            .map(|asset| asset.primitives.len())
            .sum(),
    )?;
    add(
        &mut totals.dictionary_entries,
        asset.decoded.arena.dictionary.len(),
    )
}

#[derive(Debug)]
enum ProbeBuilderError {
    Decoded(DecodedArenaAssetError),
    Simulation(SimulationCollisionAssetBuildError),
    SimulationStatic(SimulationStaticArenaPlanError),
    SimulationNormal(SimulationNormalArenaPlanError),
    SimulationStaticPrepare(SimulationStaticArenaPrepareError),
    SimulationStaticRegistrationStart(SimulationStaticArenaRegistrationError),
    SimulationStaticRegistration {
        operation_index: usize,
        source: EnvironmentUnlockRouterError,
    },
    SimulationStaticSignature(String),
    SimulationNormalSignature(String),
    UnexpectedNullAsset,
    TelemetryPoisoned,
    TelemetryOverflow,
}

impl fmt::Display for ProbeBuilderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decoded(source) => source.fmt(formatter),
            Self::Simulation(source) => source.fmt(formatter),
            Self::SimulationStatic(source) => source.fmt(formatter),
            Self::SimulationNormal(source) => source.fmt(formatter),
            Self::SimulationStaticPrepare(source) => source.fmt(formatter),
            Self::SimulationStaticRegistrationStart(source) => source.fmt(formatter),
            Self::SimulationStaticRegistration {
                operation_index,
                source,
            } => write!(
                formatter,
                "supported Simulation registration failed at operation {operation_index}: {source}"
            ),
            Self::SimulationStaticSignature(message) | Self::SimulationNormalSignature(message) => {
                formatter.write_str(message)
            }
            Self::UnexpectedNullAsset => {
                formatter.write_str("arena builder returned no resident asset")
            }
            Self::TelemetryPoisoned => formatter.write_str("build telemetry was poisoned"),
            Self::TelemetryOverflow => formatter.write_str("build telemetry overflowed"),
        }
    }
}

impl Error for ProbeBuilderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decoded(source) => Some(source),
            Self::Simulation(source) => Some(source),
            Self::SimulationStatic(source) => Some(source),
            Self::SimulationNormal(source) => Some(source),
            Self::SimulationStaticPrepare(source) => Some(source),
            Self::SimulationStaticRegistrationStart(source) => Some(source),
            Self::SimulationStaticRegistration { source, .. } => Some(source),
            Self::SimulationStaticSignature(_)
            | Self::SimulationNormalSignature(_)
            | Self::UnexpectedNullAsset
            | Self::TelemetryPoisoned
            | Self::TelemetryOverflow => None,
        }
    }
}

#[derive(Clone)]
struct ProbeBuilderSessions {
    catalog: WorldStreamCatalog,
    simulation: SimulationCollisionAssetBuilderSessions,
    telemetry: Arc<Mutex<BuildTelemetry>>,
}

impl ProbeBuilderSessions {
    fn new(catalog: WorldStreamCatalog, telemetry: Arc<Mutex<BuildTelemetry>>) -> Self {
        Self {
            simulation: SimulationCollisionAssetBuilderSessions::new(catalog.clone()),
            catalog,
            telemetry,
        }
    }
}

impl CollectionAssetBuilderSessionFactory for ProbeBuilderSessions {
    type Asset = ProbeResidentAsset;
    type Builder = ProbeAssetBuilder;
    type Error = ProbeSessionError;

    fn begin_session(
        &mut self,
        collection: &CollectionRegistrationSnapshot,
    ) -> Result<Self::Builder, Self::Error> {
        let registered = self
            .catalog
            .collection(collection.id)
            .map_err(ProbeSessionError::Catalog)?
            .ok_or(ProbeSessionError::UnknownCollection(collection.id))?;
        if &registered != collection {
            return Err(ProbeSessionError::CollectionSnapshotMismatch(collection.id));
        }
        let file = self
            .catalog
            .file(&registered.owner)
            .map_err(ProbeSessionError::Catalog)?
            .ok_or(ProbeSessionError::UnknownFileOwner(collection.id))?;
        let inner = match file.kind {
            StreamKind::Presentation | StreamKind::Texture => {
                ProbeBuilderInner::Decoded(DecodedArenaAssetBuilder)
            }
            StreamKind::Simulation => ProbeBuilderInner::Simulation(
                self.simulation
                    .begin_session(collection)
                    .map_err(ProbeSessionError::Simulation)?,
            ),
        };
        let simulation_name = if file.kind == StreamKind::Simulation {
            Some(
                EXPECTED_COLLECTIONS
                    .iter()
                    .find(|expected| {
                        expected.kind == StreamKind::Simulation && expected.id == collection.id
                    })
                    .ok_or(ProbeSessionError::UnexpectedSimulationCollection(
                        collection.id,
                    ))?
                    .name,
            )
        } else {
            None
        };
        let mut telemetry = self
            .telemetry
            .lock()
            .map_err(|_| ProbeSessionError::TelemetryPoisoned)?;
        let item = &mut telemetry.kinds[kind_index(file.kind)];
        item.collections = item
            .collections
            .checked_add(1)
            .ok_or(ProbeSessionError::TelemetryOverflow)?;
        fnv_u64(&mut item.asset_hash, collection.id);
        fnv_u32(&mut item.asset_hash, collection.asset_count);
        if let Some(name) = simulation_name {
            static_fnv_bytes(&mut telemetry.simulation_static.signature, b"C");
            static_fnv_bytes(&mut telemetry.simulation_static.signature, name.as_bytes());
            static_fnv_bytes(&mut telemetry.simulation_static.signature, &[0]);
            static_fnv_bytes(&mut telemetry.simulation_normal.signature, b"C");
            static_fnv_bytes(&mut telemetry.simulation_normal.signature, name.as_bytes());
            static_fnv_bytes(&mut telemetry.simulation_normal.signature, &[0]);
        }
        drop(telemetry);
        Ok(ProbeAssetBuilder {
            kind: file.kind,
            inner,
            telemetry: Arc::clone(&self.telemetry),
        })
    }
}

#[derive(Clone, Debug)]
enum ProbeSessionError {
    Catalog(CatalogError),
    UnknownCollection(u64),
    CollectionSnapshotMismatch(u64),
    UnknownFileOwner(u64),
    UnexpectedSimulationCollection(u64),
    Simulation(SimulationCollisionAssetSessionError),
    TelemetryPoisoned,
    TelemetryOverflow,
}

impl fmt::Display for ProbeSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(source) => source.fmt(formatter),
            Self::UnknownCollection(id) => write!(formatter, "unknown collection {id:#018x}"),
            Self::CollectionSnapshotMismatch(id) => {
                write!(
                    formatter,
                    "collection {id:#018x} differs from canonical catalog state"
                )
            }
            Self::UnknownFileOwner(id) => {
                write!(
                    formatter,
                    "collection {id:#018x} has no registered file owner"
                )
            }
            Self::UnexpectedSimulationCollection(id) => write!(
                formatter,
                "Simulation collection {id:#018x} is outside the frozen Rob's Park selection"
            ),
            Self::Simulation(source) => source.fmt(formatter),
            Self::TelemetryPoisoned => formatter.write_str("session telemetry was poisoned"),
            Self::TelemetryOverflow => formatter.write_str("session telemetry overflowed"),
        }
    }
}

impl Error for ProbeSessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(source) => Some(source),
            Self::Simulation(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ProbeRequestIdError;

impl fmt::Display for ProbeRequestIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("diagnostic raw-0 request identity is already live")
    }
}

impl Error for ProbeRequestIdError {}

struct ProbeRequestLease {
    live: Arc<AtomicBool>,
}

impl CollectionRequestIdLease for ProbeRequestLease {
    fn raw(&self) -> i32 {
        0
    }
}

impl Drop for ProbeRequestLease {
    fn drop(&mut self) {
        self.live.store(false, Ordering::Release);
    }
}

struct ProbeRequestIds {
    live: Arc<AtomicBool>,
}

impl CollectionRequestIdAllocator for ProbeRequestIds {
    type Lease = ProbeRequestLease;
    type Error = ProbeRequestIdError;

    fn acquire(&mut self) -> Result<Self::Lease, Self::Error> {
        self.live
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| ProbeRequestIdError)?;
        Ok(ProbeRequestLease {
            live: Arc::clone(&self.live),
        })
    }
}

type ProbeResidentCause =
    CatalogResidentLoadError<MountError, ProbeRequestIdError, ProbeSessionError, ProbeBuilderError>;
type ProbeResidentError = Arc<ProbeResidentCause>;

fn resident_external(error: impl Error + Send + Sync + 'static) -> ProbeResidentError {
    Arc::new(CatalogResidentLoadError::external(Arc::new(error)))
}

#[derive(Clone)]
struct ProbeMetadata {
    catalog: WorldStreamCatalog,
}

impl ResidentCollectionMetadata<ProbeResidentError> for ProbeMetadata {
    type Descriptor = CollectionRegistrationSnapshot;

    fn find_collection(
        &mut self,
        id: CollectionId,
    ) -> Result<Option<Self::Descriptor>, ProbeResidentError> {
        self.catalog.collection(id.0).map_err(resident_external)
    }
}

#[derive(Debug)]
struct DiagnosticFailure(String);

impl fmt::Display for DiagnosticFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for DiagnosticFailure {}

#[derive(Clone, Debug, Default)]
struct RegistryTelemetry {
    active: Vec<(CollectionId, u64)>,
    published: Vec<CollectionId>,
    removed: Vec<CollectionId>,
}

struct ProbeWrapper {
    view: OwnedResidentCollectionView<ProbeResidentAsset>,
    token: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProbeWrapperToken(u64);

struct ProbeRegistry {
    next_token: u64,
    telemetry: Arc<Mutex<RegistryTelemetry>>,
}

impl ResidentCollectionRegistry<OwnedResidentCollection<ProbeResidentAsset>, ProbeResidentError>
    for ProbeRegistry
{
    type Wrapper = ProbeWrapper;
    type Token = ProbeWrapperToken;

    fn create_wrapper(
        &mut self,
        _id: CollectionId,
        collection: &OwnedResidentCollection<ProbeResidentAsset>,
    ) -> Result<Self::Wrapper, ProbeResidentError> {
        let token = self.next_token;
        self.next_token = self.next_token.checked_add(1).ok_or_else(|| {
            resident_external(DiagnosticFailure("wrapper token overflow".to_owned()))
        })?;
        Ok(ProbeWrapper {
            view: collection.weak_view(),
            token,
        })
    }

    fn stable_token(wrapper: &Self::Wrapper) -> Self::Token {
        assert!(
            wrapper.view.is_attached(),
            "new wrapper lost its collection"
        );
        ProbeWrapperToken(wrapper.token)
    }

    fn publish(&mut self, id: CollectionId, token: &Self::Token) -> Result<(), ProbeResidentError> {
        let mut telemetry = self.telemetry.lock().map_err(|_| {
            resident_external(DiagnosticFailure("registry telemetry poisoned".to_owned()))
        })?;
        if telemetry.active.iter().any(|(existing, _)| *existing == id) {
            return Err(resident_external(DiagnosticFailure(format!(
                "duplicate wrapper publication for {:#018x}",
                id.0
            ))));
        }
        telemetry.active.push((id, token.0));
        telemetry.published.push(id);
        Ok(())
    }

    fn remove_exact(
        &mut self,
        id: CollectionId,
        token: &Self::Token,
    ) -> Result<(), ProbeResidentError> {
        let mut telemetry = self.telemetry.lock().map_err(|_| {
            resident_external(DiagnosticFailure("registry telemetry poisoned".to_owned()))
        })?;
        let Some(index) = telemetry
            .active
            .iter()
            .position(|entry| *entry == (id, token.0))
        else {
            return Err(resident_external(DiagnosticFailure(format!(
                "wrapper removal identity mismatch for {:#018x}",
                id.0
            ))));
        };
        telemetry.active.remove(index);
        telemetry.removed.push(id);
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
struct GarbageTelemetry {
    asset_ids: Vec<CollectionId>,
}

struct ProbeGarbageCollector {
    telemetry: Arc<Mutex<GarbageTelemetry>>,
}

impl ResidentAssetGarbageCollector<ProbeResidentError> for ProbeGarbageCollector {
    fn garbage_collect(&mut self, id: CollectionId) -> Result<(), ProbeResidentError> {
        self.telemetry
            .lock()
            .map_err(|_| {
                resident_external(DiagnosticFailure("garbage telemetry poisoned".to_owned()))
            })?
            .asset_ids
            .push(id);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CallbackPhase {
    Load,
    Unload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CallbackEvent {
    phase: CallbackPhase,
    id: CollectionId,
    observation: ResidentCallbackObservation,
}

enum ProbeCallback {
    Requested {
        phase: CallbackPhase,
        telemetry: Arc<Mutex<Vec<CallbackEvent>>>,
    },
    Parent {
        phase: CallbackPhase,
        callback: ParentCollectionCallback<ProbeParentEffects>,
        observations: Arc<Mutex<Vec<ParentCallbackObservation>>>,
    },
}

impl ResidentCollectionCallback<ProbeResidentError> for ProbeCallback {
    fn on_complete(
        &mut self,
        id: CollectionId,
        observation: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, Self>,
    ) -> Result<(), ProbeResidentError> {
        match self {
            Self::Requested { phase, telemetry } => {
                telemetry
                    .lock()
                    .map_err(|_| {
                        resident_external(DiagnosticFailure(
                            "callback telemetry poisoned".to_owned(),
                        ))
                    })?
                    .push(CallbackEvent {
                        phase: *phase,
                        id,
                        observation,
                    });
                Ok(())
            }
            Self::Parent {
                phase,
                callback,
                observations,
            } => {
                observations
                    .lock()
                    .map_err(|_| {
                        resident_external(DiagnosticFailure(
                            "parent callback telemetry poisoned".to_owned(),
                        ))
                    })?
                    .push(ParentCallbackObservation {
                        phase: *phase,
                        id,
                        observation,
                    });
                callback.dispatch_mapped(id, context, map_parent_callback_error)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParentCallbackObservation {
    phase: CallbackPhase,
    id: CollectionId,
    observation: ResidentCallbackObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParentEffectEvent {
    SimulationRegister(CollectionId),
    PresentationRegister(CollectionId),
    SimulationDeregister(CollectionId),
    PresentationDeregister(CollectionId),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ParentEffectTelemetry {
    defer_updates: u32,
    events: Vec<ParentEffectEvent>,
    pending_downs: VecDeque<CollectionId>,
}

struct ProbeParentEffects {
    telemetry: Arc<Mutex<ParentEffectTelemetry>>,
}

impl ProbeParentEffects {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ParentEffectTelemetry>, DiagnosticFailure> {
        self.telemetry
            .lock()
            .map_err(|_| DiagnosticFailure("parent effect telemetry poisoned".to_owned()))
    }
}

impl CollectionParentEffects for ProbeParentEffects {
    type Error = DiagnosticFailure;

    fn simulation_register(&mut self, id: CollectionId) -> Result<(), Self::Error> {
        self.lock()?
            .events
            .push(ParentEffectEvent::SimulationRegister(id));
        Ok(())
    }

    fn presentation_register(&mut self, id: CollectionId) -> Result<(), Self::Error> {
        self.lock()?
            .events
            .push(ParentEffectEvent::PresentationRegister(id));
        Ok(())
    }

    fn simulation_defer_update(&mut self) -> Result<(), Self::Error> {
        let mut telemetry = self.lock()?;
        telemetry.defer_updates = telemetry.defer_updates.wrapping_add(1);
        Ok(())
    }

    fn simulation_deregister(&mut self, id: CollectionId) -> Result<(), Self::Error> {
        let mut telemetry = self.lock()?;
        telemetry
            .events
            .push(ParentEffectEvent::SimulationDeregister(id));
        telemetry.pending_downs.push_back(id);
        Ok(())
    }

    fn presentation_deregister(&mut self, id: CollectionId) -> Result<(), Self::Error> {
        let mut telemetry = self.lock()?;
        telemetry
            .events
            .push(ParentEffectEvent::PresentationDeregister(id));
        telemetry.pending_downs.push_back(id);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DriverTelemetry {
    updates: u32,
    cache_hits: u32,
    unloads_processed: u32,
    physical_loads_started: u32,
    physical_loads_completed: u32,
    decompression_kicks: u32,
}

impl DriverTelemetry {
    fn add(&mut self, update: ResidentStreamerUpdate) {
        self.updates = self.updates.wrapping_add(1);
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct MountedDriverTelemetry {
    started: Vec<CollectionId>,
    completed: Vec<(CollectionId, MountedCollectionStreamSnapshot, u32)>,
}

trait MountedSnapshotView {
    fn mounted_snapshot(&self) -> Option<MountedCollectionStreamSnapshot>;
}

impl<R, W, B, L, IE, SFE> MountedSnapshotView
    for CatalogResidentCollectionLoadDriver<R, W, B, L, IE, SFE>
where
    R: CollectionLogicalRangeSource,
    B: CollectionAssetBuilder<AssetRegistrationSnapshot>,
    W: LoadedAssetCollection<B::Asset>,
    L: CollectionRequestIdLease,
{
    fn mounted_snapshot(&self) -> Option<MountedCollectionStreamSnapshot> {
        self.mounted_stream_snapshot()
    }
}

struct RecordingLoadDriver<D> {
    id: CollectionId,
    inner: D,
    telemetry: Arc<Mutex<MountedDriverTelemetry>>,
    recorded: bool,
}

impl<D> RecordingLoadDriver<D> {
    fn record_completion(&mut self) -> Result<(), ProbeResidentError>
    where
        D: MountedSnapshotView + ResidentCollectionLoadDriver<ProbeResidentError>,
    {
        if self.recorded {
            return Ok(());
        }
        let Some(snapshot) = self.inner.mounted_snapshot() else {
            return Ok(());
        };
        self.telemetry
            .lock()
            .map_err(|_| {
                resident_external(DiagnosticFailure(
                    "mounted-driver telemetry poisoned".to_owned(),
                ))
            })?
            .completed
            .push((self.id, snapshot, self.inner.highest_stored_block()));
        self.recorded = true;
        Ok(())
    }
}

impl<D> ResidentCollectionLoadDriver<ProbeResidentError> for RecordingLoadDriver<D>
where
    D: ResidentCollectionLoadDriver<ProbeResidentError> + MountedSnapshotView,
{
    fn update(&mut self) -> Result<(), ProbeResidentError> {
        self.inner.update()?;
        if self.inner.is_complete() {
            self.record_completion()?;
        }
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    fn highest_stored_block(&self) -> u32 {
        self.inner.highest_stored_block()
    }

    fn loaded_size(&self) -> u32 {
        self.inner.loaded_size()
    }
}

struct RecordingLoadFactory<F> {
    inner: F,
    telemetry: Arc<Mutex<MountedDriverTelemetry>>,
}

impl<F, W, D> ResidentCollectionLoadFactory<W, D, ProbeResidentError> for RecordingLoadFactory<F>
where
    F: ResidentCollectionLoadFactory<W, D, ProbeResidentError>,
    F::Driver: MountedSnapshotView,
{
    type Driver = RecordingLoadDriver<F::Driver>;

    fn begin(
        &mut self,
        id: CollectionId,
        asset_count: i32,
        descriptor: D,
        writer: W,
    ) -> Result<Self::Driver, ProbeResidentError> {
        let inner = self.inner.begin(id, asset_count, descriptor, writer)?;
        self.telemetry
            .lock()
            .map_err(|_| {
                resident_external(DiagnosticFailure(
                    "mounted-driver telemetry poisoned".to_owned(),
                ))
            })?
            .started
            .push(id);
        Ok(RecordingLoadDriver {
            id,
            inner,
            telemetry: Arc::clone(&self.telemetry),
            recorded: false,
        })
    }
}

type ProbeStorage = OwnedResidentCollectionStorage<
    ProbeResidentAsset,
    fn(OwnedResidentCollectionError) -> ProbeResidentError,
    ProbeResidentError,
>;

fn map_storage_error(error: OwnedResidentCollectionError) -> ProbeResidentError {
    resident_external(error)
}

/// Drives the exact 14 spatial selections directly through the reconstructed
/// resident streamer, then repeats them through parent expansion. Outer
/// collection scheduling and real Simulation/Presentation effects remain
/// deliberately unclaimed integration boundaries.
#[allow(clippy::too_many_lines)] // Keep the load/observe/unload sequence auditable in one place.
pub(super) fn run(
    mounts: Arc<MountSet>,
    catalog: WorldStreamCatalog,
) -> Result<(), Box<dyn Error>> {
    require(
        catalog.counts()? == LOADED_CATALOG_COUNTS,
        "resident probe requires the fully loaded BAM manifest catalog",
    )?;
    let verification_catalog = catalog.clone();
    let parent_mounts = Arc::clone(&mounts);
    let parent_catalog = catalog.clone();
    let selected = selected_collections(&mounts, &catalog)?;
    let expected_ids = EXPECTED_COLLECTIONS
        .iter()
        .map(|expected| CollectionId(expected.id))
        .collect::<Vec<_>>();

    let build_telemetry = Arc::new(Mutex::new(BuildTelemetry::default()));
    let registry_telemetry = Arc::new(Mutex::new(RegistryTelemetry::default()));
    let garbage_telemetry = Arc::new(Mutex::new(GarbageTelemetry::default()));
    let callback_telemetry = Arc::new(Mutex::new(Vec::new()));
    let mounted_driver_telemetry = Arc::new(Mutex::new(MountedDriverTelemetry::default()));
    let request_live = Arc::new(AtomicBool::new(false));

    let sessions = ProbeBuilderSessions::new(catalog.clone(), Arc::clone(&build_telemetry));
    let factory = CatalogResidentCollectionLoadFactory::new(
        mounts,
        catalog.clone(),
        ProbeRequestIds {
            live: Arc::clone(&request_live),
        },
        sessions,
    );
    let wake = factory.decompression_kick();
    let factory = RecordingLoadFactory {
        inner: factory,
        telemetry: Arc::clone(&mounted_driver_telemetry),
    };
    let storage = ProbeStorage::new(map_storage_error);
    let metadata = ProbeMetadata { catalog };
    let registry = ProbeRegistry {
        next_token: 1,
        telemetry: Arc::clone(&registry_telemetry),
    };
    let garbage = ProbeGarbageCollector {
        telemetry: Arc::clone(&garbage_telemetry),
    };
    let mut streamer =
        CollectionResidentStreamer::new(storage, metadata, factory, registry, garbage, wake);

    for collection in &selected {
        streamer.enqueue_load(
            CollectionId(collection.id),
            i32::from_ne_bytes(collection.asset_count.to_ne_bytes()),
            Some(ProbeCallback::Requested {
                phase: CallbackPhase::Load,
                telemetry: Arc::clone(&callback_telemetry),
            }),
        );
    }

    let mut driver = DriverTelemetry::default();
    drive_to_idle(&mut streamer, &mut driver)?;
    require(
        driver
            == DriverTelemetry {
                updates: 375,
                cache_hits: 0,
                unloads_processed: 0,
                physical_loads_started: 14,
                physical_loads_completed: 14,
                decompression_kicks: 361,
            },
        format!("resident load driver telemetry differed: {driver:?}"),
    )?;
    require(
        streamer.resident_count() == 14,
        "resident load did not retain 14 collections",
    )?;
    require(
        streamer.highest_stream_block_size() == 734_333,
        format!(
            "resident high-water block was {}, expected 734333",
            streamer.highest_stream_block_size()
        ),
    )?;
    require(
        !request_live.load(Ordering::Acquire),
        "raw-0 request lease remained live after load",
    )?;
    for id in &expected_ids {
        let observation = streamer
            .observe_resident(*id)
            .ok_or_else(|| probe_error(format!("missing resident collection {:#018x}", id.0)))?;
        require(
            observation.ref_count == 1 && observation.wrapper_is_published,
            format!("resident collection {:#018x} has {observation:?}", id.0),
        )?;
    }
    verify_loaded_telemetry(
        &build_telemetry,
        &registry_telemetry,
        &callback_telemetry,
        &mounted_driver_telemetry,
        &expected_ids,
    )?;
    let simulation_static_signature = lock_probe(&build_telemetry, "static signature")?
        .simulation_static
        .signature;
    let simulation_normal_signature = lock_probe(&build_telemetry, "normal signature")?
        .simulation_normal
        .signature;
    require(
        verification_catalog.counts()? == LOADED_CATALOG_COUNTS,
        "resident load mutated the manifest catalog",
    )?;

    // Same-order diagnostic teardown makes the per-occurrence build/GC order
    // directly comparable. Outer stream-kind unload priority is not claimed.
    for collection in &selected {
        streamer.enqueue_unload(
            CollectionId(collection.id),
            Some(ProbeCallback::Requested {
                phase: CallbackPhase::Unload,
                telemetry: Arc::clone(&callback_telemetry),
            }),
        );
    }
    drive_to_idle(&mut streamer, &mut driver)?;
    require(
        driver
            == DriverTelemetry {
                updates: 376,
                cache_hits: 0,
                unloads_processed: 14,
                physical_loads_started: 14,
                physical_loads_completed: 14,
                decompression_kicks: 361,
            },
        format!("resident unload driver telemetry differed: {driver:?}"),
    )?;
    require(
        streamer.resident_count() == 0,
        "resident unload left live collections",
    )?;
    require(
        !request_live.load(Ordering::Acquire),
        "raw-0 request lease remained live after unload",
    )?;
    verify_unloaded_telemetry(
        &build_telemetry,
        &registry_telemetry,
        &garbage_telemetry,
        &callback_telemetry,
        &mounted_driver_telemetry,
        &expected_ids,
    )?;
    require(
        verification_catalog.counts()? == LOADED_CATALOG_COUNTS,
        "resident unload mutated the manifest catalog",
    )?;
    drop(streamer);

    run_parent_expanded(parent_mounts, &parent_catalog, &selected)?;

    println!("Rob's Park resident selection: Pres 6/2528, Sim 4/68, Tex 4/685");
    println!(
        "resident load: 14 collections, 3281 arenas, 73508371 stored bytes, 142118000 decoded bytes"
    );
    println!(
        "simulation projection: 71 roots, 354382 triangles, 477 instances, 10314 grind primitives"
    );
    println!(
        "simulation static plan: 42 grind + 26 environment = 68 ordered operations; signature {simulation_static_signature:016x}"
    );
    println!(
        "supported normal Simulation plan: 269 ordered operations (6:42, 10:26, 11:0, 16:0, 22:145, 23:0, 25:4, 27:4, 28:3, 32:45); signature {simulation_normal_signature:016x}"
    );
    println!(
        "supported static adapter: 68 forward register/deregister operations; 26 identity-placed roots -> 0 across diagnostic build/drop timing"
    );
    println!("resident unload: 14 collections, 3281 ordered garbage callbacks, 0 residents");
    println!(
        "PASS: mounted resident ownership/materialization, native-order supported normal Simulation planning, static routing, and parent expansion; full manager/message registration remains unimplemented"
    );
    Ok(())
}

fn map_parent_callback_error(
    error: CollectionParentManagerError<DiagnosticFailure>,
) -> ProbeResidentError {
    resident_external(error)
}

#[allow(clippy::too_many_lines)]
fn run_parent_expanded(
    mounts: Arc<MountSet>,
    catalog: &WorldStreamCatalog,
    selected: &[CollectionRegistrationSnapshot],
) -> Result<(), Box<dyn Error>> {
    let parent = catalog
        .collection(PRESENTATION_PARENT_ID)?
        .ok_or_else(|| probe_error("catalog lacks BAM cPres_Global"))?;
    let parent_file = catalog
        .file(&parent.owner)?
        .ok_or_else(|| probe_error("BAM cPres_Global lacks a registered file owner"))?;
    require(
        parent.source_index == 0
            && parent.path.as_ref() == "data\\content\\world\\stream\\BAM\\cPres_Global.xsf"
            && parent.offset == 0x80
            && parent.size == PRESENTATION_PARENT_PAYLOAD_SIZE
            && parent.asset_count == PRESENTATION_PARENT_ASSET_COUNT
            && parent.stream_format == 2
            && parent.parent_id == INVALID_COLLECTION_ID
            && parent_file.kind == StreamKind::Presentation,
        "BAM cPres_Global canonical metadata differed",
    )?;
    let mut parent_descriptor_hash = FNV64_OFFSET;
    fnv_u64(&mut parent_descriptor_hash, parent.id);
    fnv_u32(&mut parent_descriptor_hash, u32::try_from(parent.offset)?);
    fnv_u32(&mut parent_descriptor_hash, parent.size);
    fnv_u32(&mut parent_descriptor_hash, parent.asset_count);
    fnv_bytes(&mut parent_descriptor_hash, b"cPres_Global\0");
    require(
        parent_descriptor_hash == PRESENTATION_PARENT_DESCRIPTOR_HASH,
        format!("BAM cPres_Global descriptor hash was {parent_descriptor_hash:016x}"),
    )?;

    let build_telemetry = Arc::new(Mutex::new(BuildTelemetry::default()));
    let registry_telemetry = Arc::new(Mutex::new(RegistryTelemetry::default()));
    let garbage_telemetry = Arc::new(Mutex::new(GarbageTelemetry::default()));
    let callback_telemetry = Arc::new(Mutex::new(Vec::new()));
    let parent_callback_telemetry = Arc::new(Mutex::new(Vec::new()));
    let parent_effect_telemetry = Arc::new(Mutex::new(ParentEffectTelemetry::default()));
    let mounted_driver_telemetry = Arc::new(Mutex::new(MountedDriverTelemetry::default()));
    let request_live = Arc::new(AtomicBool::new(false));

    let sessions = ProbeBuilderSessions::new(catalog.clone(), Arc::clone(&build_telemetry));
    let factory = CatalogResidentCollectionLoadFactory::new(
        mounts,
        catalog.clone(),
        ProbeRequestIds {
            live: Arc::clone(&request_live),
        },
        sessions,
    );
    let wake = factory.decompression_kick();
    let factory = RecordingLoadFactory {
        inner: factory,
        telemetry: Arc::clone(&mounted_driver_telemetry),
    };
    let storage = ProbeStorage::new(map_storage_error);
    let metadata = ProbeMetadata {
        catalog: catalog.clone(),
    };
    let registry = ProbeRegistry {
        next_token: 1,
        telemetry: Arc::clone(&registry_telemetry),
    };
    let garbage = ProbeGarbageCollector {
        telemetry: Arc::clone(&garbage_telemetry),
    };
    let mut streamer =
        CollectionResidentStreamer::new(storage, metadata, factory, registry, garbage, wake);
    let mut parent_manager = CollectionParentManager::new(ProbeParentEffects {
        telemetry: Arc::clone(&parent_effect_telemetry),
    });

    let mut edge_count = 0_usize;
    for collection in selected {
        if collection.parent_id != INVALID_COLLECTION_ID {
            require(
                collection.parent_id == PRESENTATION_PARENT_ID
                    && parent_file.kind == StreamKind::Presentation,
                format!(
                    "unexpected parent {:#018x} for selected collection {:#018x}",
                    collection.parent_id, collection.id
                ),
            )?;
            parent_manager.add_parent(
                CollectionId(collection.id),
                CollectionId(collection.parent_id),
            );
            edge_count += 1;
        }
    }
    require(
        edge_count == 6,
        format!("parent edge count was {edge_count}"),
    )?;

    let mut planned_load = Vec::new();
    for collection in selected {
        for request in parent_manager.plan_load(CollectionId(collection.id))? {
            require(
                request.kind == PlannedParentRequestKind::Load,
                "load planner emitted a non-load request",
            )?;
            let metadata = catalog
                .collection(request.id.0)?
                .ok_or_else(|| probe_error(format!("load plan lacks {:#018x}", request.id.0)))?;
            let callback = match request.callback {
                PlannedParentCallbackKind::Requested => ProbeCallback::Requested {
                    phase: CallbackPhase::Load,
                    telemetry: Arc::clone(&callback_telemetry),
                },
                PlannedParentCallbackKind::LoadParent => ProbeCallback::Parent {
                    phase: CallbackPhase::Load,
                    callback: parent_manager.load_parent_callback(),
                    observations: Arc::clone(&parent_callback_telemetry),
                },
                PlannedParentCallbackKind::UnloadParent => {
                    return Err(probe_error("load plan emitted an unload-parent callback"));
                }
            };
            planned_load.push((request.id, request.callback));
            streamer.enqueue_load(
                request.id,
                i32::from_ne_bytes(metadata.asset_count.to_ne_bytes()),
                Some(callback),
            );
        }
    }
    let expected_load_plan = selected
        .iter()
        .flat_map(|collection| {
            let mut requests = Vec::with_capacity(2);
            if collection.parent_id != INVALID_COLLECTION_ID {
                requests.push((
                    CollectionId(PRESENTATION_PARENT_ID),
                    PlannedParentCallbackKind::LoadParent,
                ));
            }
            requests.push((
                CollectionId(collection.id),
                PlannedParentCallbackKind::Requested,
            ));
            requests
        })
        .collect::<Vec<_>>();
    require(
        planned_load == expected_load_plan && planned_load.len() == 20,
        "parent-expanded load plan differed",
    )?;

    let mut driver = DriverTelemetry::default();
    let deadline = Instant::now() + PROBE_TIMEOUT;
    while streamer.pending_requests() {
        if Instant::now() >= deadline {
            return Err(probe_error("parent-expanded load timed out"));
        }
        let update = parent_manager.update(|| streamer.update())?;
        require(
            update.processed_parent_unload.is_none(),
            "load phase processed a parent unload completion",
        )?;
        driver.add(update.resident);
        thread::yield_now();
    }
    let load_updates = driver.updates;
    require(
        driver
            == DriverTelemetry {
                updates: 383,
                cache_hits: 5,
                unloads_processed: 0,
                physical_loads_started: 15,
                physical_loads_completed: 15,
                decompression_kicks: 368,
            },
        format!("parent-expanded load telemetry differed: {driver:?}"),
    )?;
    require(
        streamer.resident_count() == 15
            && streamer.highest_stream_block_size() == 734_333
            && parent_manager.parent_use_count(CollectionId(PRESENTATION_PARENT_ID)) == Some(6)
            && parent_manager.pending_parent_unloads() == 0
            && !request_live.load(Ordering::Acquire),
        "parent-expanded loaded state differed",
    )?;
    let parent_observation = streamer
        .observe_resident(CollectionId(PRESENTATION_PARENT_ID))
        .ok_or_else(|| probe_error("parent resident was not published"))?;
    require(
        parent_observation.ref_count == 8 && parent_observation.wrapper_is_published,
        format!("loaded parent resident state differed: {parent_observation:?}"),
    )?;
    for collection in selected {
        let observation = streamer
            .observe_resident(CollectionId(collection.id))
            .ok_or_else(|| {
                probe_error(format!("missing selected resident {:#018x}", collection.id))
            })?;
        require(
            observation.ref_count == 1 && observation.wrapper_is_published,
            format!("selected resident {:#018x} differed", collection.id),
        )?;
    }

    let mut physical_ids = Vec::with_capacity(15);
    physical_ids.push(CollectionId(PRESENTATION_PARENT_ID));
    physical_ids.extend(selected.iter().map(|item| CollectionId(item.id)));
    {
        let mounted = lock_probe(&mounted_driver_telemetry, "expanded mounted driver")?;
        require(
            mounted.started == physical_ids
                && mounted.completed.len() == physical_ids.len()
                && mounted
                    .completed
                    .iter()
                    .map(|(id, _, _)| *id)
                    .eq(physical_ids.iter().copied()),
            "parent-expanded physical driver order differed",
        )?;
        let mut expected_highs = Vec::with_capacity(15);
        expected_highs.push(PRESENTATION_PARENT_HIGHEST_STORED_BLOCK);
        expected_highs.extend(EXPECTED_HIGHEST_STORED_BLOCKS);
        let mut released_chunks = 0_u64;
        let mut accounted_bytes = 0_u64;
        for (((id, snapshot, high), expected_id), expected_high) in mounted
            .completed
            .iter()
            .zip(&physical_ids)
            .zip(expected_highs)
        {
            let metadata = catalog
                .collection(id.0)?
                .ok_or_else(|| probe_error(format!("completed driver lacks {:#018x}", id.0)))?;
            require(
                id == expected_id
                    && snapshot.queued_chunks == 0
                    && snapshot.retained_chunks == 0
                    && snapshot.retained_stride_bytes == 0
                    && snapshot.released_chunks == u64::from(metadata.asset_count)
                    && snapshot.accounted_stream_read_bytes == u64::from(metadata.size)
                    && snapshot.producer_finished
                    && !snapshot.producer_failed
                    && *high == expected_high,
                format!("expanded mounted snapshot differed for {:#018x}", id.0),
            )?;
            released_chunks = released_chunks.wrapping_add(snapshot.released_chunks);
            accounted_bytes = accounted_bytes.wrapping_add(snapshot.accounted_stream_read_bytes);
        }
        require(
            released_chunks == 3_316 && accounted_bytes == 75_428_224,
            format!(
                "expanded mounted totals were {released_chunks} chunks / {accounted_bytes} bytes"
            ),
        )?;
    }
    {
        let build = lock_probe(&build_telemetry, "expanded build")?;
        let presentation = &build.kinds[kind_index(StreamKind::Presentation)];
        let mut parent_asset_hash = FNV64_OFFSET;
        fnv_u64(&mut parent_asset_hash, PRESENTATION_PARENT_ID);
        fnv_u32(&mut parent_asset_hash, PRESENTATION_PARENT_ASSET_COUNT);
        for id in &build.asset_ids[..PRESENTATION_PARENT_ASSET_COUNT as usize] {
            fnv_u64(&mut parent_asset_hash, id.0);
        }
        require(
            presentation.collections == 7
                && presentation.assets == 2_563
                && presentation.stored_bytes
                    == EXPECTED_KIND_TOTALS[0].stored_bytes + PRESENTATION_PARENT_STORED_BYTES
                && presentation.decoded_bytes
                    == EXPECTED_KIND_TOTALS[0].decoded_bytes + PRESENTATION_PARENT_DECODED_BYTES
                && presentation.asset_hash == EXPANDED_PRESENTATION_ASSET_HASH
                && parent_asset_hash == PRESENTATION_PARENT_ASSET_HASH
                && build.kinds[1].collections == EXPECTED_KIND_TOTALS[1].collections
                && build.kinds[1].assets == EXPECTED_KIND_TOTALS[1].assets
                && build.kinds[1].stored_bytes == EXPECTED_KIND_TOTALS[1].stored_bytes
                && build.kinds[1].decoded_bytes == EXPECTED_KIND_TOTALS[1].decoded_bytes
                && build.kinds[1].asset_hash == EXPECTED_KIND_TOTALS[1].asset_hash
                && build.kinds[2].collections == EXPECTED_KIND_TOTALS[2].collections
                && build.kinds[2].assets == EXPECTED_KIND_TOTALS[2].assets
                && build.kinds[2].stored_bytes == EXPECTED_KIND_TOTALS[2].stored_bytes
                && build.kinds[2].decoded_bytes == EXPECTED_KIND_TOTALS[2].decoded_bytes
                && build.kinds[2].asset_hash == EXPECTED_KIND_TOTALS[2].asset_hash
                && build.asset_ids.len() == 3_316
                && build.projection == EXPECTED_SIMULATION_PROJECTION
                && build.simulation_static == EXPECTED_SIMULATION_STATIC
                && build.simulation_normal == EXPECTED_SIMULATION_NORMAL
                && build.simulation_static_adapter.published_volume_count()
                    == EXPECTED_STATIC_GROUND_VOLUMES
                && build.simulation_static_adapter.live_registration_count() == 68
                && build.registered_static_operations == EXPECTED_SIMULATION_STATIC.operations
                && build.deregistered_static_operations == 0
                && u64::try_from(build.simulation_static_adapter.grind().added_ids.len())
                    == Ok(EXPECTED_SIMULATION_STATIC.grind)
                && build
                    .simulation_static_adapter
                    .grind()
                    .removed_ids
                    .is_empty()
                && build.dropped_assets == 0,
            format!("parent-expanded build telemetry differed: {build:?}"),
        )?;
    }
    {
        let registry = lock_probe(&registry_telemetry, "expanded registry")?;
        require(
            registry.published == physical_ids
                && registry.active.len() == physical_ids.len()
                && registry.active.iter().zip(&physical_ids).enumerate().all(
                    |(index, ((id, token), expected))| {
                        id == expected
                            && u64::try_from(index).is_ok_and(|index| *token == index + 1)
                    },
                )
                && registry.removed.is_empty(),
            "parent-expanded publication order differed",
        )?;
    }
    {
        let callbacks = lock_probe(&callback_telemetry, "expanded requested callbacks")?;
        require(
            callbacks.len() == selected.len()
                && callbacks.iter().zip(selected).all(|(event, collection)| {
                    event.phase == CallbackPhase::Load
                        && event.id == CollectionId(collection.id)
                        && event.observation.resident_ref_count == Some(1)
                        && event.observation.wrapper_is_published
                }),
            "parent-expanded requested load callbacks differed",
        )?;
        let parent_callbacks = lock_probe(&parent_callback_telemetry, "parent callbacks")?;
        require(
            parent_callbacks.len() == 6
                && parent_callbacks
                    .iter()
                    .zip([1, 4, 5, 6, 7, 8])
                    .all(|(event, reference)| {
                        event.phase == CallbackPhase::Load
                            && event.id == CollectionId(PRESENTATION_PARENT_ID)
                            && event.observation.resident_ref_count == Some(reference)
                            && event.observation.wrapper_is_published
                    }),
            format!("parent load callback observations differed: {parent_callbacks:?}"),
        )?;
    }
    {
        let effects = lock_probe(&parent_effect_telemetry, "parent effects")?;
        require(
            effects.events
                == [
                    ParentEffectEvent::SimulationRegister(CollectionId(PRESENTATION_PARENT_ID)),
                    ParentEffectEvent::PresentationRegister(CollectionId(PRESENTATION_PARENT_ID)),
                ]
                && effects.pending_downs.is_empty()
                && effects.defer_updates == load_updates,
            format!("loaded parent effects differed: {effects:?}"),
        )?;
    }

    let mut planned_unload = Vec::new();
    for collection in selected {
        for request in parent_manager.plan_unload(CollectionId(collection.id))? {
            require(
                request.kind == PlannedParentRequestKind::Unload,
                "unload planner emitted a non-unload request",
            )?;
            let callback = match request.callback {
                PlannedParentCallbackKind::Requested => ProbeCallback::Requested {
                    phase: CallbackPhase::Unload,
                    telemetry: Arc::clone(&callback_telemetry),
                },
                PlannedParentCallbackKind::UnloadParent => ProbeCallback::Parent {
                    phase: CallbackPhase::Unload,
                    callback: parent_manager.unload_parent_callback(),
                    observations: Arc::clone(&parent_callback_telemetry),
                },
                PlannedParentCallbackKind::LoadParent => {
                    return Err(probe_error("unload plan emitted a load-parent callback"));
                }
            };
            planned_unload.push((request.id, request.callback));
            streamer.enqueue_unload(request.id, Some(callback));
        }
    }
    let expected_unload_plan = selected
        .iter()
        .flat_map(|collection| {
            let mut requests = vec![(
                CollectionId(collection.id),
                PlannedParentCallbackKind::Requested,
            )];
            if collection.parent_id != INVALID_COLLECTION_ID {
                requests.push((
                    CollectionId(PRESENTATION_PARENT_ID),
                    PlannedParentCallbackKind::UnloadParent,
                ));
            }
            requests
        })
        .collect::<Vec<_>>();
    require(
        planned_unload == expected_unload_plan && planned_unload.len() == 20,
        "parent-expanded unload plan differed",
    )?;

    let deadline = Instant::now() + PROBE_TIMEOUT;
    let mut parent_fifo_updates = 0_u32;
    while streamer.pending_requests() || parent_manager.pending_parent_unloads() != 0 {
        if Instant::now() >= deadline {
            return Err(probe_error("parent-expanded unload timed out"));
        }
        let update = parent_manager.update(|| streamer.update())?;
        if update.processed_parent_unload.is_some() {
            parent_fifo_updates = parent_fifo_updates.wrapping_add(1);
        }
        driver.add(update.resident);
    }
    require(
        parent_fifo_updates == 6
            && load_updates == 383
            && driver
                == DriverTelemetry {
                    updates: 389,
                    cache_hits: 5,
                    unloads_processed: 20,
                    physical_loads_started: 15,
                    physical_loads_completed: 15,
                    decompression_kicks: 368,
                },
        format!("parent-expanded unload cadence differed: {driver:?}"),
    )?;
    require(
        streamer.resident_count() == 1
            && parent_manager
                .parent_use_count(CollectionId(PRESENTATION_PARENT_ID))
                .is_none()
            && parent_manager.pending_parent_unloads() == 0,
        "parent-expanded pre-acknowledgement state differed",
    )?;
    let parent_observation = streamer
        .observe_resident(CollectionId(PRESENTATION_PARENT_ID))
        .ok_or_else(|| probe_error("parent disappeared before subsystem acknowledgements"))?;
    require(
        parent_observation.ref_count == 2 && parent_observation.wrapper_is_published,
        format!("pre-acknowledgement parent state differed: {parent_observation:?}"),
    )?;

    let pending_downs = {
        let mut effects = lock_probe(&parent_effect_telemetry, "pending parent downs")?;
        effects.pending_downs.drain(..).collect::<Vec<_>>()
    };
    require(
        pending_downs
            == [
                CollectionId(PRESENTATION_PARENT_ID),
                CollectionId(PRESENTATION_PARENT_ID),
            ],
        format!("parent subsystem acknowledgements differed: {pending_downs:?}"),
    )?;
    require(
        !streamer.down_collection_loaded_count(pending_downs[0])?
            && streamer.down_collection_loaded_count(pending_downs[1])?,
        "parent subsystem acknowledgement Down results differed",
    )?;
    require(
        streamer.resident_count() == 0 && !request_live.load(Ordering::Acquire),
        "parent-expanded teardown left resident or request ownership",
    )?;

    {
        let callbacks = lock_probe(&callback_telemetry, "expanded requested callbacks")?;
        require(
            callbacks.len() == selected.len() * 2
                && callbacks[selected.len()..]
                    .iter()
                    .zip(selected)
                    .all(|(event, collection)| {
                        event.phase == CallbackPhase::Unload
                            && event.id == CollectionId(collection.id)
                            && event.observation.resident_ref_count == Some(1)
                            && event.observation.wrapper_is_published
                    }),
            "parent-expanded requested unload callbacks differed",
        )?;
        let parent_callbacks = lock_probe(&parent_callback_telemetry, "parent callbacks")?;
        require(
            parent_callbacks.len() == 12
                && parent_callbacks[6..].iter().zip([8, 7, 6, 5, 4, 3]).all(
                    |(event, reference)| {
                        event.phase == CallbackPhase::Unload
                            && event.id == CollectionId(PRESENTATION_PARENT_ID)
                            && event.observation.resident_ref_count == Some(reference)
                            && event.observation.wrapper_is_published
                    },
                ),
            format!("parent unload callback observations differed: {parent_callbacks:?}"),
        )?;
    }
    {
        let effects = lock_probe(&parent_effect_telemetry, "parent effects")?;
        require(
            effects.events
                == [
                    ParentEffectEvent::SimulationRegister(CollectionId(PRESENTATION_PARENT_ID)),
                    ParentEffectEvent::PresentationRegister(CollectionId(PRESENTATION_PARENT_ID)),
                    ParentEffectEvent::SimulationDeregister(CollectionId(PRESENTATION_PARENT_ID)),
                    ParentEffectEvent::PresentationDeregister(CollectionId(PRESENTATION_PARENT_ID)),
                ]
                && effects.pending_downs.is_empty()
                && effects.defer_updates == driver.updates,
            format!("final parent effects differed: {effects:?}"),
        )?;
    }
    let build = lock_probe(&build_telemetry, "expanded build")?;
    let mut expected_garbage = build.asset_ids[PRESENTATION_PARENT_ASSET_COUNT as usize..].to_vec();
    expected_garbage
        .extend_from_slice(&build.asset_ids[..PRESENTATION_PARENT_ASSET_COUNT as usize]);
    require(
        build.dropped_assets == 3_316
            && build.simulation_static_adapter.published_volume_count() == 0
            && build.simulation_static_adapter.live_registration_count() == 0
            && build.registered_static_operations == EXPECTED_SIMULATION_STATIC.operations
            && build.deregistered_static_operations == EXPECTED_SIMULATION_STATIC.operations
            && build.simulation_static_adapter.grind().added_ids
                == build.simulation_static_adapter.grind().removed_ids
            && u64::try_from(build.simulation_static_adapter.grind().added_ids.len())
                == Ok(EXPECTED_SIMULATION_STATIC.grind),
        format!(
            "expanded teardown left {} dropped assets / {} static ground volumes",
            build.dropped_assets,
            build.simulation_static_adapter.published_volume_count()
        ),
    )?;
    drop(build);
    let garbage = lock_probe(&garbage_telemetry, "expanded garbage")?;
    require(
        garbage.asset_ids == expected_garbage,
        "parent-expanded garbage order differed",
    )?;
    drop(garbage);
    let registry = lock_probe(&registry_telemetry, "expanded registry")?;
    let mut expected_removal = selected
        .iter()
        .map(|item| CollectionId(item.id))
        .collect::<Vec<_>>();
    expected_removal.push(CollectionId(PRESENTATION_PARENT_ID));
    require(
        registry.active.is_empty() && registry.removed == expected_removal,
        "parent-expanded registry removal order differed",
    )?;
    drop(registry);
    require(
        catalog.counts()? == LOADED_CATALOG_COUNTS,
        "parent-expanded composition mutated the manifest catalog",
    )?;

    println!(
        "parent expansion: 20 requests / 15 physical collections / 5 cache hits; cPres_Global 35 arenas"
    );
    println!(
        "parent-expanded resident load: 3316 arenas, 74799979 stored bytes, 145603480 decoded bytes"
    );
    println!(
        "parent lifetime: use 6 / resident 8 -> unload hold 2 -> acknowledged 0; Sim then Pres effects"
    );
    Ok(())
}

fn selected_collections(
    mounts: &MountSet,
    catalog: &WorldStreamCatalog,
) -> Result<Vec<CollectionRegistrationSnapshot>, Box<dyn Error>> {
    let locations =
        LocationTable::parse(&mounts.read_path("data\\content\\world\\stream\\BAM\\BAM_Sim.loc")?)?;
    let location = locations
        .find("A_RobsPark")
        .ok_or_else(|| probe_error("BAM_Sim.loc has no A_RobsPark"))?;
    require(
        location.category.as_deref() == Some("DIST_Waterfront")
            && location.position().map(f32::to_bits) == [0x4413_3726, 0xc258_ed35, 0x4479_204f],
        "A_RobsPark identity or position differed",
    )?;
    let focus = location.position();
    require(
        WORLD_FOCUS_RADIUS.to_bits() == 0x4343_0000
            && TEXTURE_FOCUS_RADIUS.to_bits() == 0x428c_0000
            && DEFAULT_PRESENTATION_FOCUS_COLLECTIONS == 16
            && DEFAULT_SIMULATION_FOCUS_COLLECTIONS == 4
            && DEFAULT_TEXTURE_FOCUS_COLLECTIONS == 16,
        "Rob's Park focus radius or collection cap constants differed",
    )?;
    let plans = [
        (
            StreamKind::Presentation,
            "data\\content\\world\\stream\\BAM\\BAM_Pres.xss",
            WORLD_FOCUS_RADIUS,
            DEFAULT_PRESENTATION_FOCUS_COLLECTIONS,
        ),
        (
            StreamKind::Simulation,
            "data\\content\\world\\stream\\BAM\\BAM_Sim.xss",
            WORLD_FOCUS_RADIUS,
            DEFAULT_SIMULATION_FOCUS_COLLECTIONS,
        ),
        (
            StreamKind::Texture,
            "data\\content\\world\\stream\\BAM\\BAM_Tex.xss",
            TEXTURE_FOCUS_RADIUS,
            DEFAULT_TEXTURE_FOCUS_COLLECTIONS,
        ),
    ];
    let mut result = Vec::with_capacity(EXPECTED_COLLECTIONS.len());
    let mut expected_index = 0_usize;
    let mut descriptor_hashes = [FNV64_OFFSET; 3];
    for (kind, path, radius, cap) in plans {
        let space = CollectionSpace::parse(&mounts.read_path(path)?)?;
        let focused = space.focus_collections(focus, radius, cap);
        for item in focused {
            let expected = EXPECTED_COLLECTIONS.get(expected_index).ok_or_else(|| {
                probe_error(format!("unexpected extra {kind:?} focus collection"))
            })?;
            require(
                expected.kind == kind && expected.id == item.id,
                format!(
                    "focus collection {expected_index} was {:?}/{:016x}, expected {:?}/{:016x}",
                    kind, item.id, expected.kind, expected.id
                ),
            )?;
            let collection = catalog
                .collection(item.id)?
                .ok_or_else(|| probe_error(format!("catalog lacks collection {:016x}", item.id)))?;
            let file = catalog
                .file(&collection.owner)?
                .ok_or_else(|| probe_error(format!("collection {:016x} lacks file", item.id)))?;
            let expected_path = format!("data\\content\\world\\stream\\BAM\\{}.xsf", expected.name);
            require(
                file.kind == kind
                    && collection.path.as_ref() == expected_path
                    && collection.offset == 0x80
                    && collection.size == expected.payload_size
                    && collection.asset_count == expected.asset_count
                    && collection.stream_format == 2
                    && collection.parent_id == expected.parent_id,
                format!(
                    "canonical metadata differed for collection {:016x}",
                    item.id
                ),
            )?;
            let hash = &mut descriptor_hashes[kind_index(kind)];
            fnv_u64(hash, collection.id);
            fnv_u32(hash, u32::try_from(collection.offset)?);
            fnv_u32(hash, collection.size);
            fnv_u32(hash, collection.asset_count);
            fnv_bytes(hash, expected.name.as_bytes());
            fnv_bytes(hash, &[0]);
            result.push(collection);
            expected_index += 1;
        }
    }
    require(
        expected_index == EXPECTED_COLLECTIONS.len(),
        format!("focus selected {expected_index} collections, expected 14"),
    )?;
    for (index, hash) in descriptor_hashes.into_iter().enumerate() {
        require(
            hash == EXPECTED_KIND_TOTALS[index].descriptor_hash,
            format!("kind {index} descriptor order hash was {hash:016x}"),
        )?;
    }
    Ok(result)
}

fn drive_to_idle<S, M, F, R, G, K, C>(
    streamer: &mut CollectionResidentStreamer<S, M, F, R, G, K, C, ProbeResidentError>,
    telemetry: &mut DriverTelemetry,
) -> Result<(), Box<dyn Error>>
where
    S: skate2_core::collection_resident_streamer::ResidentCollectionStorage<ProbeResidentError>,
    M: ResidentCollectionMetadata<ProbeResidentError>,
    F: skate2_core::collection_resident_streamer::ResidentCollectionLoadFactory<
            S::Writer,
            M::Descriptor,
            ProbeResidentError,
        >,
    R: ResidentCollectionRegistry<S::Collection, ProbeResidentError>,
    G: ResidentAssetGarbageCollector<ProbeResidentError>,
    K: skate2_core::collection_resident_streamer::ResidentDecompressionKick<ProbeResidentError>,
    C: ResidentCollectionCallback<ProbeResidentError>,
{
    let deadline = Instant::now() + PROBE_TIMEOUT;
    while streamer.pending_requests() {
        if Instant::now() >= deadline {
            return Err(probe_error(format!(
                "resident probe timed out with active {:?} and {} queued",
                streamer.active_id(),
                streamer.queued_request_count()
            )));
        }
        telemetry.add(streamer.update()?);
        thread::yield_now();
    }
    Ok(())
}

fn verify_loaded_telemetry(
    build: &Arc<Mutex<BuildTelemetry>>,
    registry: &Arc<Mutex<RegistryTelemetry>>,
    callbacks: &Arc<Mutex<Vec<CallbackEvent>>>,
    mounted: &Arc<Mutex<MountedDriverTelemetry>>,
    expected_ids: &[CollectionId],
) -> Result<(), Box<dyn Error>> {
    let build = lock_probe(build, "build")?;
    for (index, observed) in build.kinds.iter().enumerate() {
        let expected = EXPECTED_KIND_TOTALS[index];
        require(
            observed.collections == expected.collections
                && observed.assets == expected.assets
                && observed.stored_bytes == expected.stored_bytes
                && observed.decoded_bytes == expected.decoded_bytes
                && observed.asset_hash == expected.asset_hash,
            format!("kind {index} build telemetry differed: {observed:?}"),
        )?;
    }
    require(
        build.asset_ids.len() == 3_281
            && build.projection == EXPECTED_SIMULATION_PROJECTION
            && build.simulation_static == EXPECTED_SIMULATION_STATIC
            && build.simulation_normal == EXPECTED_SIMULATION_NORMAL
            && build.simulation_static_adapter.published_volume_count()
                == EXPECTED_STATIC_GROUND_VOLUMES
            && build.simulation_static_adapter.live_registration_count() == 68
            && build.registered_static_operations == EXPECTED_SIMULATION_STATIC.operations
            && build.deregistered_static_operations == 0
            && u64::try_from(build.simulation_static_adapter.grind().added_ids.len())
                == Ok(EXPECTED_SIMULATION_STATIC.grind)
            && build
                .simulation_static_adapter
                .grind()
                .removed_ids
                .is_empty()
            && build.dropped_assets == 0,
        format!(
            "loaded asset/projection telemetry differed: assets {}, projection {:?}, static {:?}, normal {:?}, drops {}",
            build.asset_ids.len(),
            build.projection,
            build.simulation_static,
            build.simulation_normal,
            build.dropped_assets
        ),
    )?;
    drop(build);
    let registry = lock_probe(registry, "registry")?;
    require(
        registry.published == expected_ids
            && registry.removed.is_empty()
            && registry.active.len() == 14,
        format!("loaded registry telemetry differed: {registry:?}"),
    )?;
    drop(registry);
    let callbacks = lock_probe(callbacks, "callback")?;
    require(
        callbacks.len() == 14
            && callbacks.iter().zip(expected_ids).all(|(event, expected)| {
                event.phase == CallbackPhase::Load
                    && event.id == *expected
                    && event.observation.active_id == *expected
                    && event.observation.resident_ref_count == Some(1)
                    && event.observation.wrapper_is_published
            }),
        format!("loaded callback telemetry differed: {callbacks:?}"),
    )?;
    drop(callbacks);
    verify_mounted_driver_telemetry(mounted, expected_ids)
}

fn verify_unloaded_telemetry(
    build: &Arc<Mutex<BuildTelemetry>>,
    registry: &Arc<Mutex<RegistryTelemetry>>,
    garbage: &Arc<Mutex<GarbageTelemetry>>,
    callbacks: &Arc<Mutex<Vec<CallbackEvent>>>,
    mounted: &Arc<Mutex<MountedDriverTelemetry>>,
    expected_ids: &[CollectionId],
) -> Result<(), Box<dyn Error>> {
    let build = lock_probe(build, "build")?;
    let built_ids = build.asset_ids.clone();
    require(
        build.dropped_assets == 3_281
            && build.simulation_static_adapter.published_volume_count() == 0
            && build.simulation_static_adapter.live_registration_count() == 0
            && build.registered_static_operations == EXPECTED_SIMULATION_STATIC.operations
            && build.deregistered_static_operations == EXPECTED_SIMULATION_STATIC.operations
            && build.simulation_static_adapter.grind().added_ids
                == build.simulation_static_adapter.grind().removed_ids
            && u64::try_from(build.simulation_static_adapter.grind().added_ids.len())
                == Ok(EXPECTED_SIMULATION_STATIC.grind),
        format!(
            "resident teardown left {} dropped assets / {} static ground volumes",
            build.dropped_assets,
            build.simulation_static_adapter.published_volume_count()
        ),
    )?;
    drop(build);
    let garbage = lock_probe(garbage, "garbage")?;
    require(
        garbage.asset_ids == built_ids,
        format!(
            "garbage order/count differed: observed {}, expected {}",
            garbage.asset_ids.len(),
            built_ids.len()
        ),
    )?;
    drop(garbage);
    let registry = lock_probe(registry, "registry")?;
    require(
        registry.active.is_empty()
            && registry.published == expected_ids
            && registry.removed == expected_ids,
        format!("unloaded registry telemetry differed: {registry:?}"),
    )?;
    drop(registry);
    let callbacks = lock_probe(callbacks, "callback")?;
    require(
        callbacks.len() == 28
            && callbacks[14..]
                .iter()
                .zip(expected_ids)
                .all(|(event, expected)| {
                    event.phase == CallbackPhase::Unload
                        && event.id == *expected
                        && event.observation.active_id == *expected
                        && event.observation.resident_ref_count == Some(1)
                        && event.observation.wrapper_is_published
                }),
        format!("unloaded callback telemetry differed: {callbacks:?}"),
    )?;
    drop(callbacks);
    verify_mounted_driver_telemetry(mounted, expected_ids)
}

fn verify_mounted_driver_telemetry(
    mounted: &Arc<Mutex<MountedDriverTelemetry>>,
    expected_ids: &[CollectionId],
) -> Result<(), Box<dyn Error>> {
    let mounted = lock_probe(mounted, "mounted driver")?;
    let completed_ids = mounted
        .completed
        .iter()
        .map(|(id, _, _)| *id)
        .collect::<Vec<_>>();
    let released_chunks = mounted
        .completed
        .iter()
        .map(|(_, snapshot, _)| snapshot.released_chunks)
        .sum::<u64>();
    let accounted_bytes = mounted
        .completed
        .iter()
        .map(|(_, snapshot, _)| snapshot.accounted_stream_read_bytes)
        .sum::<u64>();
    require(
        mounted.started == expected_ids
            && completed_ids == expected_ids
            && mounted
                .completed
                .iter()
                .zip(EXPECTED_COLLECTIONS)
                .zip(EXPECTED_HIGHEST_STORED_BLOCKS)
                .all(|(((id, snapshot, highest), expected), expected_highest)| {
                    id.0 == expected.id
                        && snapshot.queued_chunks == 0
                        && snapshot.retained_chunks == 0
                        && snapshot.retained_stride_bytes == 0
                        && snapshot.released_chunks == u64::from(expected.asset_count)
                        && snapshot.accounted_stream_read_bytes == u64::from(expected.payload_size)
                        && snapshot.producer_finished
                        && !snapshot.producer_failed
                        && *highest == expected_highest
                })
            && released_chunks == 3_281
            && accounted_bytes == 74_129_920,
        format!(
            "mounted driver telemetry differed: starts {}, completions {}, released {released_chunks}, accounted {accounted_bytes}",
            mounted.started.len(),
            mounted.completed.len()
        ),
    )
}

fn lock_probe<'a, T>(
    value: &'a Arc<Mutex<T>>,
    label: &str,
) -> Result<std::sync::MutexGuard<'a, T>, Box<dyn Error>> {
    value
        .lock()
        .map_err(|_| probe_error(format!("{label} telemetry was poisoned")))
}

const fn kind_index(kind: StreamKind) -> usize {
    match kind {
        StreamKind::Presentation => 0,
        StreamKind::Simulation => 1,
        StreamKind::Texture => 2,
    }
}

fn fnv_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV64_PRIME);
    }
}

fn fnv_u32(hash: &mut u64, value: u32) {
    fnv_bytes(hash, &value.to_le_bytes());
}

fn fnv_u64(hash: &mut u64, value: u64) {
    fnv_bytes(hash, &value.to_le_bytes());
}

fn static_fnv_bytes(hash: &mut u64, bytes: &[u8]) {
    fnv_bytes(hash, bytes);
}

fn static_fnv_u32(hash: &mut u64, value: u32) {
    static_fnv_bytes(hash, &value.to_be_bytes());
}

fn static_fnv_u64(hash: &mut u64, value: u64) {
    static_fnv_bytes(hash, &value.to_be_bytes());
}

fn require(condition: bool, message: impl Into<String>) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(probe_error(message))
    }
}

fn probe_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_selection_has_exact_kind_counts_and_full_width_ids() {
        assert_eq!(EXPECTED_COLLECTIONS.len(), 14);
        assert_eq!(
            EXPECTED_COLLECTIONS
                .iter()
                .filter(|item| item.kind == StreamKind::Presentation)
                .count(),
            6
        );
        assert_eq!(
            EXPECTED_COLLECTIONS
                .iter()
                .filter(|item| item.kind == StreamKind::Simulation)
                .count(),
            4
        );
        assert_eq!(
            EXPECTED_COLLECTIONS
                .iter()
                .filter(|item| item.kind == StreamKind::Texture)
                .count(),
            4
        );
        assert_eq!(
            EXPECTED_COLLECTIONS
                .iter()
                .map(|item| u64::from(item.asset_count))
                .sum::<u64>(),
            3_281
        );
        assert_eq!(EXPECTED_COLLECTIONS[0].id, 0xf14f_738b_2b3d_06a5);
        assert_eq!(EXPECTED_COLLECTIONS[13].id, 0x1454_1e27_3652_c0f5);
    }

    #[test]
    fn raw_zero_request_identity_allows_exactly_one_live_lease() {
        let live = Arc::new(AtomicBool::new(false));
        let mut allocator = ProbeRequestIds {
            live: Arc::clone(&live),
        };
        let lease = allocator.acquire().unwrap();
        assert_eq!(lease.raw(), 0);
        assert!(allocator.acquire().is_err());
        drop(lease);
        assert_eq!(allocator.acquire().unwrap().raw(), 0);
    }

    #[test]
    fn fnv_helpers_preserve_external_corpus_byte_order() {
        let mut hash = FNV64_OFFSET;
        let first = EXPECTED_COLLECTIONS[0];
        fnv_u64(&mut hash, first.id);
        fnv_u32(&mut hash, 0x80);
        fnv_u32(&mut hash, first.payload_size);
        fnv_u32(&mut hash, first.asset_count);
        fnv_bytes(&mut hash, first.name.as_bytes());
        fnv_bytes(&mut hash, &[0]);
        assert_ne!(hash, FNV64_OFFSET);

        let mut static_hash = FNV64_OFFSET;
        static_fnv_u32(&mut static_hash, 0x0102_0304);
        let mut explicit = FNV64_OFFSET;
        fnv_bytes(&mut explicit, &[1, 2, 3, 4]);
        assert_eq!(static_hash, explicit);
    }

    #[test]
    fn parent_corpus_constants_and_expanded_totals_are_frozen() {
        let mut descriptor_hash = FNV64_OFFSET;
        fnv_u64(&mut descriptor_hash, PRESENTATION_PARENT_ID);
        fnv_u32(&mut descriptor_hash, 0x80);
        fnv_u32(&mut descriptor_hash, PRESENTATION_PARENT_PAYLOAD_SIZE);
        fnv_u32(&mut descriptor_hash, PRESENTATION_PARENT_ASSET_COUNT);
        fnv_bytes(&mut descriptor_hash, b"cPres_Global\0");

        assert_eq!(descriptor_hash, PRESENTATION_PARENT_DESCRIPTOR_HASH);
        assert_eq!(PRESENTATION_PARENT_ASSET_COUNT + 3_281, 3_316);
        assert_eq!(PRESENTATION_PARENT_STORED_BYTES + 73_508_371, 74_799_979);
        assert_eq!(
            u64::from(PRESENTATION_PARENT_PAYLOAD_SIZE) + 74_129_920,
            75_428_224
        );
        assert_eq!(PRESENTATION_PARENT_DECODED_BYTES + 142_118_000, 145_603_480);
    }
}
