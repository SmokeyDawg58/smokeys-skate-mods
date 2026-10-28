//! Whole-corpus gate for the globally interleaved normal Simulation adapter.

use skate2_core::{
    grind::GrindAsset,
    simulation_environment_unlock_router::{
        EnvironmentUnlockRouterError, SimulationEnvironmentUnlockRouter,
    },
    simulation_normal_arena_adapter::{
        PreparedSimulationNormalArena, SimulationNormalArenaRegistrationAdapter,
        SimulationNormalEnvironmentDispatch,
    },
    simulation_normal_arena_plan::{
        SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRecordHandle,
        SimulationRegistryAssetKind,
    },
    simulation_pointer_arena_adapter::SimulationPointerArenaDispatch,
    simulation_registry_arena_adapter::SimulationRegistryArenaDispatch,
    simulation_static_arena_adapter::SimulationStaticGrindDispatch,
    simulation_static_arena_plan::SimulationStaticOperation,
    static_ground_pool::{StaticEnvironmentSource, StaticGroundSlotSnapshot},
};
use skate2_formats::pegasus::{
    InstanceData, TABLE_OF_CONTENTS_TYPE_ID, TableOfContents, TriggerInstanceData,
};
use std::{
    error::Error,
    io,
    sync::{Arc, Mutex},
};

const CORPUS_GROUND_CAPACITY: usize = 20_000;
const NORMAL_USERDATA_RAW: u32 = 8;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const EXPECTED_RAW_SIGNATURES: RawSignatures = RawSignatures {
    scope: 0x6022_910b_e547_1fcf,
    operations: 0x45fd_ebeb_d34b_65b3,
    unified: 0xbf12_e381_2595_15f3,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SimulationNormalRuntimeInventory {
    pub collections: usize,
    pub arenas: usize,
    pub toc_entries: usize,
    pub operations: usize,
    pub prepared_operations: usize,
    pub grind: usize,
    pub environment: usize,
    pub dmo: usize,
    pub trigger: usize,
    pub world_painter: usize,
    pub nav_mesh: usize,
    pub rain: usize,
    pub ai_path: usize,
    pub waypoint: usize,
    pub spatial_map: usize,
    pub registrations: usize,
    pub deregistrations: usize,
    pub registration_callbacks: usize,
    pub deregistration_callbacks: usize,
    pub sequence_mismatches: usize,
    pub source_identity_mismatches: usize,
    pub ground_mismatches: usize,
}

const EXPECTED_SIMULATION_NORMAL_RUNTIME: SimulationNormalRuntimeInventory =
    SimulationNormalRuntimeInventory {
        collections: 284,
        arenas: 2_135,
        toc_entries: 62_810,
        operations: 9_045,
        prepared_operations: 9_045,
        grind: 819,
        environment: 790,
        dmo: 0,
        trigger: 10,
        world_painter: 6_122,
        nav_mesh: 0,
        rain: 75,
        ai_path: 101,
        waypoint: 95,
        spatial_map: 1_033,
        registrations: 2_135,
        deregistrations: 2_135,
        registration_callbacks: 9_045,
        deregistration_callbacks: 9_045,
        sequence_mismatches: 0,
        source_identity_mismatches: 0,
        ground_mismatches: 0,
    };

#[derive(Clone, Debug)]
enum ExpectedCallback {
    Grind {
        registration: bool,
        asset_id: u64,
    },
    Environment {
        registration: bool,
        source: Arc<StaticEnvironmentSource>,
    },
    DmoModel {
        registration: bool,
        asset_id: u64,
    },
    DmoRegistry {
        registration: bool,
        source: SimulationRecordHandle,
        asset_id: u64,
    },
    Trigger {
        registration: bool,
        source: SimulationRecordHandle,
    },
    Registry {
        registration: bool,
        kind: SimulationRegistryAssetKind,
        asset_id: u64,
        waypoint_streamed: Option<bool>,
    },
}

impl ExpectedCallback {
    fn registration(&self) -> bool {
        match self {
            Self::Grind { registration, .. }
            | Self::Environment { registration, .. }
            | Self::DmoModel { registration, .. }
            | Self::DmoRegistry { registration, .. }
            | Self::Trigger { registration, .. }
            | Self::Registry { registration, .. } => *registration,
        }
    }

    fn matches(&self, actual: &Self) -> bool {
        match (self, actual) {
            (
                Self::Grind {
                    registration: expected_registration,
                    asset_id: expected_id,
                },
                Self::Grind {
                    registration,
                    asset_id,
                },
            )
            | (
                Self::DmoModel {
                    registration: expected_registration,
                    asset_id: expected_id,
                },
                Self::DmoModel {
                    registration,
                    asset_id,
                },
            ) => expected_registration == registration && expected_id == asset_id,
            (
                Self::Environment {
                    registration: expected_registration,
                    source: expected_source,
                },
                Self::Environment {
                    registration,
                    source,
                },
            ) => {
                expected_registration == registration
                    && expected_source.is_same_backing_record(source)
            }
            (
                Self::DmoRegistry {
                    registration: expected_registration,
                    source: expected_source,
                    asset_id: expected_id,
                },
                Self::DmoRegistry {
                    registration,
                    source,
                    asset_id,
                },
            ) => {
                expected_registration == registration
                    && expected_id == asset_id
                    && expected_source.is_same_backing_record(source)
            }
            (
                Self::Trigger {
                    registration: expected_registration,
                    source: expected_source,
                },
                Self::Trigger {
                    registration,
                    source,
                },
            ) => {
                expected_registration == registration
                    && expected_source.is_same_backing_record(source)
            }
            (
                Self::Registry {
                    registration: expected_registration,
                    kind: expected_kind,
                    asset_id: expected_id,
                    waypoint_streamed: expected_streamed,
                },
                Self::Registry {
                    registration,
                    kind,
                    asset_id,
                    waypoint_streamed,
                },
            ) => {
                expected_registration == registration
                    && expected_kind == kind
                    && expected_id == asset_id
                    && expected_streamed == waypoint_streamed
            }
            _ => false,
        }
    }

    fn source_identity_disagrees(&self, actual: &Self) -> bool {
        match (self, actual) {
            (
                Self::Environment {
                    source: expected, ..
                },
                Self::Environment { source: actual, .. },
            ) => !expected.is_same_backing_record(actual),
            (
                Self::DmoRegistry {
                    source: expected, ..
                }
                | Self::Trigger {
                    source: expected, ..
                },
                Self::DmoRegistry { source: actual, .. } | Self::Trigger { source: actual, .. },
            ) => !expected.is_same_backing_record(actual),
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RawSignatures {
    scope: u64,
    operations: u64,
    unified: u64,
}

impl Default for RawSignatures {
    fn default() -> Self {
        let mut signatures = Self {
            scope: FNV_OFFSET,
            operations: FNV_OFFSET,
            unified: FNV_OFFSET,
        };
        fnv_bytes(
            &mut signatures.scope,
            b"SKATE2/BAM_SIM/NORMAL_RAW_SCOPE\0V1\0",
        );
        fnv_bytes(
            &mut signatures.operations,
            b"SKATE2/BAM_SIM/NORMAL_RAW_OPERATIONS\0V1\0",
        );
        fnv_bytes(
            &mut signatures.unified,
            b"SKATE2/BAM_SIM/NORMAL_RAW_UNIFIED\0V1\0",
        );
        signatures
    }
}

#[derive(Debug, Default)]
struct SimulationNormalRuntimeState {
    inventory: SimulationNormalRuntimeInventory,
    raw_signatures: RawSignatures,
    expected: Vec<ExpectedCallback>,
    next_expected: usize,
}

impl SimulationNormalRuntimeState {
    fn begin_phase(&mut self, expected: Vec<ExpectedCallback>) {
        if self.next_expected != self.expected.len() {
            self.inventory.sequence_mismatches += 1;
        }
        self.expected = expected;
        self.next_expected = 0;
    }

    fn observe(&mut self, actual: &ExpectedCallback) {
        if actual.registration() {
            self.inventory.registration_callbacks += 1;
        } else {
            self.inventory.deregistration_callbacks += 1;
        }
        match self.expected.get(self.next_expected) {
            Some(expected) if expected.matches(actual) => {}
            Some(expected) => {
                self.inventory.sequence_mismatches += 1;
                if expected.source_identity_disagrees(actual) {
                    self.inventory.source_identity_mismatches += 1;
                }
            }
            None => self.inventory.sequence_mismatches += 1,
        }
        self.next_expected += 1;
    }

    fn finish_phase(&mut self) {
        if self.next_expected != self.expected.len() {
            self.inventory.sequence_mismatches += 1;
        }
        self.expected.clear();
        self.next_expected = 0;
    }
}

#[derive(Clone, Debug)]
struct SimulationNormalCorpusDispatch {
    state: Arc<Mutex<SimulationNormalRuntimeState>>,
}

impl SimulationNormalCorpusDispatch {
    fn observe(&self, actual: &ExpectedCallback) {
        self.state
            .lock()
            .expect("normal runtime corpus state poisoned")
            .observe(actual);
    }

    fn registry(
        &self,
        registration: bool,
        kind: SimulationRegistryAssetKind,
        asset_id: u64,
        waypoint_streamed: Option<bool>,
    ) {
        self.observe(&ExpectedCallback::Registry {
            registration,
            kind,
            asset_id,
            waypoint_streamed,
        });
    }
}

#[derive(Debug)]
struct SimulationNormalCorpusEnvironment {
    router: SimulationEnvironmentUnlockRouter,
    state: Arc<Mutex<SimulationNormalRuntimeState>>,
}

impl SimulationNormalEnvironmentDispatch for SimulationNormalCorpusEnvironment {
    fn register_environment(
        &mut self,
        source: &Arc<StaticEnvironmentSource>,
    ) -> Result<(), EnvironmentUnlockRouterError> {
        self.router.register(source)?;
        self.state
            .lock()
            .expect("normal runtime corpus state poisoned")
            .observe(&ExpectedCallback::Environment {
                registration: true,
                source: Arc::clone(source),
            });
        Ok(())
    }

    fn deregister_environment(&mut self, source: &Arc<StaticEnvironmentSource>) {
        self.router.deregister(source);
        self.state
            .lock()
            .expect("normal runtime corpus state poisoned")
            .observe(&ExpectedCallback::Environment {
                registration: false,
                source: Arc::clone(source),
            });
    }
}

impl SimulationStaticGrindDispatch for SimulationNormalCorpusDispatch {
    fn add_spline_data(&mut self, asset: &GrindAsset) {
        self.observe(&ExpectedCallback::Grind {
            registration: true,
            asset_id: asset.asset_id,
        });
    }

    fn remove_spline_data(&mut self, asset_id: u64) {
        self.observe(&ExpectedCallback::Grind {
            registration: false,
            asset_id,
        });
    }
}

impl SimulationPointerArenaDispatch for SimulationNormalCorpusDispatch {
    fn add_dmo_model(&mut self, asset_id: u64) {
        self.observe(&ExpectedCallback::DmoModel {
            registration: true,
            asset_id,
        });
    }

    fn register_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64) {
        self.observe(&ExpectedCallback::DmoRegistry {
            registration: true,
            source: source.clone(),
            asset_id,
        });
    }

    fn remove_dmo_model(&mut self, asset_id: u64) {
        self.observe(&ExpectedCallback::DmoModel {
            registration: false,
            asset_id,
        });
    }

    fn deregister_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64) {
        self.observe(&ExpectedCallback::DmoRegistry {
            registration: false,
            source: source.clone(),
            asset_id,
        });
    }

    fn add_trigger_volume(&mut self, source: &SimulationRecordHandle) {
        self.observe(&ExpectedCallback::Trigger {
            registration: true,
            source: source.clone(),
        });
    }

    fn remove_trigger_volume(&mut self, source: &SimulationRecordHandle) {
        self.observe(&ExpectedCallback::Trigger {
            registration: false,
            source: source.clone(),
        });
    }
}

impl SimulationRegistryArenaDispatch for SimulationNormalCorpusDispatch {
    fn register_streamed_tile(&mut self, asset_id: u64) {
        self.registry(
            true,
            SimulationRegistryAssetKind::WorldPainter,
            asset_id,
            None,
        );
    }

    fn unregister_streamed_tile(&mut self, asset_id: u64) {
        self.registry(
            false,
            SimulationRegistryAssetKind::WorldPainter,
            asset_id,
            None,
        );
    }

    fn add_nav_mesh_data(&mut self, asset_id: u64) {
        self.registry(true, SimulationRegistryAssetKind::NavMesh, asset_id, None);
    }

    fn remove_nav_mesh_data(&mut self, asset_id: u64) {
        self.registry(false, SimulationRegistryAssetKind::NavMesh, asset_id, None);
    }

    fn add_rain_data(&mut self, asset_id: u64) {
        self.registry(true, SimulationRegistryAssetKind::Rain, asset_id, None);
    }

    fn remove_rain_data(&mut self, asset_id: u64) {
        self.registry(false, SimulationRegistryAssetKind::Rain, asset_id, None);
    }

    fn add_ai_path_asset(&mut self, asset_id: u64) {
        self.registry(true, SimulationRegistryAssetKind::AiPath, asset_id, None);
    }

    fn remove_ai_path_asset(&mut self, asset_id: u64) {
        self.registry(false, SimulationRegistryAssetKind::AiPath, asset_id, None);
    }

    fn register_waypoint_data(&mut self, asset_id: u64, streamed: bool) {
        self.registry(
            true,
            SimulationRegistryAssetKind::Waypoint,
            asset_id,
            Some(streamed),
        );
    }

    fn deregister_waypoint_data(&mut self, asset_id: u64, streamed: bool) {
        self.registry(
            false,
            SimulationRegistryAssetKind::Waypoint,
            asset_id,
            Some(streamed),
        );
    }

    fn add_spatial_map_asset(&mut self, asset_id: u64) {
        self.registry(
            true,
            SimulationRegistryAssetKind::SpatialMap,
            asset_id,
            None,
        );
    }

    fn remove_spatial_map_asset(&mut self, asset_id: u64) {
        self.registry(
            false,
            SimulationRegistryAssetKind::SpatialMap,
            asset_id,
            None,
        );
    }
}

pub(crate) struct SimulationNormalCorpusRuntime {
    state: Arc<Mutex<SimulationNormalRuntimeState>>,
    adapter: SimulationNormalArenaRegistrationAdapter<
        SimulationNormalCorpusDispatch,
        SimulationNormalCorpusEnvironment,
    >,
}

impl SimulationNormalCorpusRuntime {
    pub(crate) fn try_new() -> Result<Self, Box<dyn Error>> {
        let state = Arc::new(Mutex::new(SimulationNormalRuntimeState::default()));
        let dispatch = SimulationNormalCorpusDispatch {
            state: Arc::clone(&state),
        };
        let environment = SimulationNormalCorpusEnvironment {
            router: SimulationEnvironmentUnlockRouter::try_new_empty(CORPUS_GROUND_CAPACITY)?,
            state: Arc::clone(&state),
        };
        Ok(Self {
            state,
            adapter: SimulationNormalArenaRegistrationAdapter::with_environment(
                dispatch,
                environment,
            ),
        })
    }

    pub(crate) fn add_collection(
        &self,
        collection_ordinal: usize,
        collection_id: u64,
        filename: &str,
        asset_count: u32,
    ) -> Result<(), Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .expect("normal runtime corpus state poisoned");
        hash_raw_collection(
            &mut state.raw_signatures,
            collection_ordinal,
            collection_id,
            filename,
            asset_count,
        )?;
        state.inventory.collections += 1;
        Ok(())
    }

    pub(crate) fn exercise(
        &mut self,
        collection_ordinal: usize,
        arena_ordinal: usize,
        plan: &Arc<SimulationNormalArenaPlan>,
    ) -> Result<(), Box<dyn Error>> {
        let prepared = PreparedSimulationNormalArena::try_new(Arc::clone(plan))?;
        let expected_ground = expected_environment_ground(plan)?;
        {
            let mut state = self
                .state
                .lock()
                .expect("normal runtime corpus state poisoned");
            let toc_entries = hash_raw_arena(
                &mut state.raw_signatures,
                collection_ordinal,
                arena_ordinal,
                plan,
            )?;
            state.inventory.toc_entries += toc_entries;
            tally_plan(&mut state.inventory, plan)?;
            state.inventory.prepared_operations += prepared.operation_count();
            state.begin_phase(expected_callbacks(plan, true)?);
        }
        let registration = self.adapter.register(&prepared, NORMAL_USERDATA_RAW)?;
        {
            let mut state = self
                .state
                .lock()
                .expect("normal runtime corpus state poisoned");
            state.finish_phase();
            state.inventory.registrations += 1;
            if self.adapter.environment().router.ground_snapshots() != expected_ground {
                state.inventory.ground_mismatches += 1;
            }
            state.begin_phase(expected_callbacks(plan, false)?);
        }
        self.adapter.deregister(registration, NORMAL_USERDATA_RAW)?;
        {
            let mut state = self
                .state
                .lock()
                .expect("normal runtime corpus state poisoned");
            state.finish_phase();
            state.inventory.deregistrations += 1;
            if self.adapter.environment().router.published_volume_count() != 0 {
                state.inventory.ground_mismatches += 1;
            }
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<SimulationNormalRuntimeInventory, Box<dyn Error>> {
        if self.adapter.live_registration_count() != 0
            || self.adapter.orphaned_source_count() != 0
            || self.adapter.fault().is_some()
        {
            return Err(probe_error(
                "normal runtime adapter did not finish quiescent",
            ));
        }
        let state = self
            .state
            .lock()
            .expect("normal runtime corpus state poisoned");
        if !state.expected.is_empty() || state.next_expected != 0 {
            return Err(probe_error("normal runtime callback phase remains pending"));
        }
        let inventory = state.inventory;
        let raw_signatures = finish_raw_signatures(state.raw_signatures, inventory)?;
        if raw_signatures != EXPECTED_RAW_SIGNATURES {
            return Err(probe_error(format!(
                "normal raw descriptor signatures differ: expected {EXPECTED_RAW_SIGNATURES:?}, got {raw_signatures:?}"
            )));
        }
        if inventory != EXPECTED_SIMULATION_NORMAL_RUNTIME {
            return Err(probe_error(format!(
                "normal runtime corpus mismatch: expected {EXPECTED_SIMULATION_NORMAL_RUNTIME:?}, got {inventory:?}"
            )));
        }
        Ok(inventory)
    }
}

fn tally_plan(
    inventory: &mut SimulationNormalRuntimeInventory,
    plan: &SimulationNormalArenaPlan,
) -> Result<(), Box<dyn Error>> {
    inventory.arenas += 1;
    inventory.operations += plan.operations().len();
    for operation in plan.operations() {
        match operation {
            SimulationNormalOperation::Grind { .. } => inventory.grind += 1,
            SimulationNormalOperation::Environment { .. } => inventory.environment += 1,
            SimulationNormalOperation::DmoInstance { .. } => inventory.dmo += 1,
            SimulationNormalOperation::TriggerInstance { .. } => inventory.trigger += 1,
            SimulationNormalOperation::RegistryAsset { kind, .. } => match kind {
                SimulationRegistryAssetKind::WorldPainter => inventory.world_painter += 1,
                SimulationRegistryAssetKind::NavMesh => inventory.nav_mesh += 1,
                SimulationRegistryAssetKind::Rain => inventory.rain += 1,
                SimulationRegistryAssetKind::AiPath => inventory.ai_path += 1,
                SimulationRegistryAssetKind::Waypoint => inventory.waypoint += 1,
                SimulationRegistryAssetKind::SpatialMap => inventory.spatial_map += 1,
            },
            _ => {
                return Err(probe_error(
                    "unsupported future normal operation in corpus gate",
                ));
            }
        }
    }
    Ok(())
}

fn hash_raw_collection(
    signatures: &mut RawSignatures,
    collection_ordinal: usize,
    collection_id: u64,
    filename: &str,
    asset_count: u32,
) -> Result<(), Box<dyn Error>> {
    let mut record = Vec::new();
    record.push(b'C');
    push_u32(&mut record, u32::try_from(collection_ordinal)?);
    push_u64(&mut record, collection_id);
    push_u32(&mut record, u32::try_from(filename.len())?);
    record.extend_from_slice(filename.as_bytes());
    push_u32(&mut record, asset_count);
    fnv_bytes(&mut signatures.scope, &record);
    fnv_bytes(&mut signatures.unified, &record);
    Ok(())
}

fn hash_raw_arena(
    signatures: &mut RawSignatures,
    collection_ordinal: usize,
    arena_ordinal: usize,
    plan: &SimulationNormalArenaPlan,
) -> Result<usize, Box<dyn Error>> {
    let decoded = &plan.static_plan().asset().decoded;
    let arena = &decoded.arena;
    let resources = &decoded.resources.resources;
    let (toc_dictionary_index, toc_dictionary) = arena
        .dictionary
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID)
        .ok_or_else(|| probe_error("normal raw signature arena has no TOC"))?;
    let toc = TableOfContents::parse(
        arena
            .object_bytes(resources, toc_dictionary)
            .ok_or_else(|| probe_error("normal raw signature TOC lies outside storage"))?,
    )?;

    let mut arena_record = Vec::new();
    arena_record.push(b'A');
    push_u32(&mut arena_record, u32::try_from(collection_ordinal)?);
    push_u32(&mut arena_record, u32::try_from(arena_ordinal)?);
    push_u64(&mut arena_record, decoded.asset_id);
    push_u32(&mut arena_record, arena.id);
    push_u32(&mut arena_record, u32::try_from(toc_dictionary_index)?);
    push_u32(&mut arena_record, u32::try_from(toc.entries.len())?);
    fnv_bytes(&mut signatures.scope, &arena_record);
    fnv_bytes(&mut signatures.unified, &arena_record);

    for (operation_ordinal, operation) in plan.operations().iter().enumerate() {
        let operation_record = raw_operation_record(plan, &toc, operation_ordinal, operation)?;
        fnv_bytes(&mut signatures.operations, &operation_record);
        fnv_bytes(&mut signatures.unified, &operation_record);
    }
    Ok(toc.entries.len())
}

#[allow(clippy::too_many_lines)] // Keep the frozen raw descriptor protocol auditable together.
fn raw_operation_record(
    plan: &SimulationNormalArenaPlan,
    toc: &TableOfContents,
    operation_ordinal: usize,
    operation: &SimulationNormalOperation,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let (kind, source_toc_index, source_dictionary_index, payload) = match *operation {
        SimulationNormalOperation::Grind {
            static_operation_index,
        } => {
            let Some(SimulationStaticOperation::Grind {
                source_toc_index,
                source_dictionary_index,
                grind_asset_index,
            }) = plan.static_plan().operations().get(static_operation_index)
            else {
                return Err(probe_error("normal raw grind lost its static operation"));
            };
            let asset_id = plan
                .static_plan()
                .grind_assets()
                .get(*grind_asset_index)
                .ok_or_else(|| probe_error("normal raw grind lost its asset"))?
                .asset_id;
            (
                6,
                *source_toc_index,
                *source_dictionary_index,
                raw_direct_payload(b'G', asset_id),
            )
        }
        SimulationNormalOperation::Environment {
            static_operation_index,
        } => {
            let Some(SimulationStaticOperation::Environment {
                source_toc_index,
                source_dictionary_index,
                source_inline_index,
                ..
            }) = plan.static_plan().operations().get(static_operation_index)
            else {
                return Err(probe_error(
                    "normal raw Environment lost its static operation",
                ));
            };
            let payload =
                raw_instance_payload(plan, *source_dictionary_index, *source_inline_index, false)?;
            (10, *source_toc_index, *source_dictionary_index, payload)
        }
        SimulationNormalOperation::DmoInstance {
            asset_id,
            local_guid,
            source_toc_index,
            source_dictionary_index,
            source_inline_index,
            source_record_offset,
            ..
        } => {
            let payload =
                raw_instance_payload(plan, source_dictionary_index, source_inline_index, false)?;
            validate_instance_payload(&payload, source_record_offset, asset_id, local_guid, "DMO")?;
            (11, source_toc_index, source_dictionary_index, payload)
        }
        SimulationNormalOperation::TriggerInstance {
            asset_id,
            primary_guid,
            source_toc_index,
            source_dictionary_index,
            source_inline_index,
            source_record_offset,
            ..
        } => {
            let payload =
                raw_instance_payload(plan, source_dictionary_index, source_inline_index, true)?;
            validate_instance_payload(
                &payload,
                source_record_offset,
                primary_guid,
                asset_id,
                "trigger",
            )?;
            (16, source_toc_index, source_dictionary_index, payload)
        }
        SimulationNormalOperation::RegistryAsset {
            kind,
            asset_id,
            source_toc_index,
            source_dictionary_index,
            raw_object_type,
            raw_object_id,
        } => {
            let raw_toc = toc
                .entries
                .get(source_toc_index as usize)
                .ok_or_else(|| probe_error("normal registry operation indexes missing TOC"))?;
            if raw_toc.guid != asset_id
                || raw_toc.object_type != raw_object_type
                || raw_toc.object.0 != raw_object_id
            {
                return Err(probe_error(
                    "normal registry descriptor disagrees with raw TOC",
                ));
            }
            (
                registry_kind_value(kind),
                source_toc_index,
                source_dictionary_index,
                raw_direct_payload(b'R', asset_id),
            )
        }
        _ => return Err(probe_error("unsupported future normal raw operation")),
    };

    let decoded = &plan.static_plan().asset().decoded;
    let arena = &decoded.arena;
    let raw_toc = toc
        .entries
        .get(source_toc_index as usize)
        .ok_or_else(|| probe_error("normal operation indexes missing raw TOC entry"))?;
    let dictionary = arena
        .dictionary
        .get(source_dictionary_index as usize)
        .ok_or_else(|| probe_error("normal operation indexes missing raw dictionary entry"))?;
    if raw_toc.object.section_index() != 0
        || raw_toc.object.entry_index() != source_dictionary_index
        || raw_toc.object_type != dictionary.type_id
    {
        return Err(probe_error(
            "normal operation raw TOC/dictionary identity disagrees",
        ));
    }
    if matches!(payload.first(), Some(b'G' | b'R')) {
        let payload_id = u64::from_be_bytes(payload[1..9].try_into()?);
        if payload_id != raw_toc.guid {
            return Err(probe_error("normal direct payload GUID disagrees with TOC"));
        }
    }

    let mut record = Vec::new();
    record.push(b'O');
    push_u32(&mut record, u32::try_from(operation_ordinal)?);
    push_u32(&mut record, kind);
    push_u32(&mut record, source_toc_index);
    push_u32(&mut record, raw_toc.object_type);
    push_u32(&mut record, raw_toc.object.0);
    push_u32(&mut record, source_dictionary_index);
    push_u32(&mut record, u32::try_from(dictionary.resource_index)?);
    push_u32(&mut record, dictionary.object_offset);
    push_u32(&mut record, dictionary.size);
    record.extend_from_slice(&payload);
    Ok(record)
}

fn raw_direct_payload(variant: u8, asset_id: u64) -> Vec<u8> {
    let mut payload = vec![variant];
    push_u64(&mut payload, asset_id);
    payload
}

fn raw_instance_payload(
    plan: &SimulationNormalArenaPlan,
    source_dictionary_index: u32,
    source_inline_index: u32,
    trigger: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let decoded = &plan.static_plan().asset().decoded;
    let arena = &decoded.arena;
    let dictionary = arena
        .dictionary
        .get(source_dictionary_index as usize)
        .ok_or_else(|| probe_error("normal inline operation lost dictionary source"))?;
    let bytes = arena
        .object_bytes(&decoded.resources.resources, dictionary)
        .ok_or_else(|| probe_error("normal inline source lies outside retained storage"))?;
    let (subtype, record_offset, record_len, primary, local) = if trigger {
        let records = TriggerInstanceData::record_table(bytes)?;
        let record = records
            .record(source_inline_index as usize)
            .ok_or_else(|| probe_error("normal trigger inline index is out of range"))?;
        (
            u32::from_be_bytes(bytes[0..4].try_into()?),
            record.object_offset(),
            record.bytes().len(),
            record.guid(),
            record.local_guid(),
        )
    } else {
        let records = InstanceData::record_table(bytes)?;
        let record = records
            .record(source_inline_index as usize)
            .ok_or_else(|| probe_error("normal instance inline index is out of range"))?;
        (
            records.type_id(),
            record.object_offset(),
            record.bytes().len(),
            record.guid(),
            record.local_guid(),
        )
    };
    let mut payload = vec![b'I'];
    push_u32(&mut payload, subtype);
    push_u32(&mut payload, source_inline_index);
    push_u32(&mut payload, record_offset);
    push_u32(&mut payload, u32::try_from(record_len)?);
    push_u64(&mut payload, primary);
    push_u64(&mut payload, local);
    Ok(payload)
}

fn validate_instance_payload(
    payload: &[u8],
    expected_offset: u32,
    expected_primary: u64,
    expected_local: u64,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let offset = u32::from_be_bytes(payload[9..13].try_into()?);
    let primary = u64::from_be_bytes(payload[17..25].try_into()?);
    let local = u64::from_be_bytes(payload[25..33].try_into()?);
    if offset != expected_offset || primary != expected_primary || local != expected_local {
        return Err(probe_error(format!(
            "normal {label} payload disagrees with plan identity"
        )));
    }
    Ok(())
}

const fn registry_kind_value(kind: SimulationRegistryAssetKind) -> u32 {
    match kind {
        SimulationRegistryAssetKind::WorldPainter => 22,
        SimulationRegistryAssetKind::NavMesh => 23,
        SimulationRegistryAssetKind::Rain => 25,
        SimulationRegistryAssetKind::AiPath => 27,
        SimulationRegistryAssetKind::Waypoint => 28,
        SimulationRegistryAssetKind::SpatialMap => 32,
    }
}

fn finish_raw_signatures(
    mut signatures: RawSignatures,
    inventory: SimulationNormalRuntimeInventory,
) -> Result<RawSignatures, Box<dyn Error>> {
    let collections = u32::try_from(inventory.collections)?;
    let arenas = u32::try_from(inventory.arenas)?;
    let toc_entries = u32::try_from(inventory.toc_entries)?;
    let operations = u32::try_from(inventory.operations)?;
    let kind_counts = [
        (6, inventory.grind),
        (10, inventory.environment),
        (11, inventory.dmo),
        (16, inventory.trigger),
        (22, inventory.world_painter),
        (23, inventory.nav_mesh),
        (25, inventory.rain),
        (27, inventory.ai_path),
        (28, inventory.waypoint),
        (32, inventory.spatial_map),
    ];

    let mut scope_trailer = vec![b'Z'];
    push_u32(&mut scope_trailer, collections);
    push_u32(&mut scope_trailer, arenas);
    push_u32(&mut scope_trailer, toc_entries);
    fnv_bytes(&mut signatures.scope, &scope_trailer);

    let mut operation_fields = Vec::new();
    push_u32(&mut operation_fields, operations);
    for (kind, count) in kind_counts {
        push_u32(&mut operation_fields, kind);
        push_u32(&mut operation_fields, u32::try_from(count)?);
    }
    fnv_bytes(&mut signatures.operations, b"Z");
    fnv_bytes(&mut signatures.operations, &operation_fields);

    fnv_bytes(&mut signatures.unified, &scope_trailer);
    fnv_bytes(&mut signatures.unified, &operation_fields);
    Ok(signatures)
}

fn fnv_bytes(signature: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *signature ^= u64::from(*byte);
        *signature = signature.wrapping_mul(FNV_PRIME);
    }
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn expected_callbacks(
    plan: &SimulationNormalArenaPlan,
    registration: bool,
) -> Result<Vec<ExpectedCallback>, Box<dyn Error>> {
    let mut expected = Vec::new();
    expected.try_reserve_exact(plan.operations().len())?;
    for (operation_index, operation) in plan.operations().iter().enumerate() {
        match *operation {
            SimulationNormalOperation::Grind {
                static_operation_index,
            } => {
                let Some(SimulationStaticOperation::Grind {
                    grind_asset_index, ..
                }) = plan.static_plan().operations().get(static_operation_index)
                else {
                    return Err(probe_error("normal grind operation lost its static source"));
                };
                let asset_id = plan
                    .static_plan()
                    .grind_assets()
                    .get(*grind_asset_index)
                    .ok_or_else(|| probe_error("normal grind operation lost its grind asset"))?
                    .asset_id;
                expected.push(ExpectedCallback::Grind {
                    registration,
                    asset_id,
                });
            }
            SimulationNormalOperation::Environment {
                static_operation_index,
            } => {
                let source = StaticEnvironmentSource::from_operation(
                    plan.static_plan_owner(),
                    static_operation_index,
                )?;
                expected.push(ExpectedCallback::Environment {
                    registration,
                    source: Arc::new(source),
                });
            }
            SimulationNormalOperation::DmoInstance { asset_id, .. } => {
                let source = plan.record_handle(operation_index).ok_or_else(|| {
                    probe_error("normal DMO operation lost its retained record source")
                })?;
                expected.push(ExpectedCallback::DmoModel {
                    registration,
                    asset_id,
                });
                expected.push(ExpectedCallback::DmoRegistry {
                    registration,
                    source,
                    asset_id,
                });
            }
            SimulationNormalOperation::TriggerInstance { .. } => {
                let source = plan.record_handle(operation_index).ok_or_else(|| {
                    probe_error("normal trigger operation lost its retained record source")
                })?;
                expected.push(ExpectedCallback::Trigger {
                    registration,
                    source,
                });
            }
            SimulationNormalOperation::RegistryAsset { kind, asset_id, .. } => {
                expected.push(ExpectedCallback::Registry {
                    registration,
                    kind,
                    asset_id,
                    waypoint_streamed: (kind == SimulationRegistryAssetKind::Waypoint)
                        .then_some(registration),
                });
            }
            _ => {
                return Err(probe_error(
                    "unsupported future normal operation in corpus gate",
                ));
            }
        }
    }
    Ok(expected)
}

fn expected_environment_ground(
    plan: &SimulationNormalArenaPlan,
) -> Result<Vec<StaticGroundSlotSnapshot>, Box<dyn Error>> {
    let mut router = SimulationEnvironmentUnlockRouter::try_new_empty(CORPUS_GROUND_CAPACITY)?;
    for operation in plan.operations() {
        if let SimulationNormalOperation::Environment {
            static_operation_index,
        } = *operation
        {
            let source = StaticEnvironmentSource::from_operation(
                plan.static_plan_owner(),
                static_operation_index,
            )?;
            router.register(&Arc::new(source))?;
        }
    }
    Ok(router.ground_snapshots())
}

fn probe_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_signature_domains_match_the_independent_oracle() {
        assert_eq!(
            RawSignatures::default(),
            RawSignatures {
                scope: 0xf45c_b0d5_58b7_90c1,
                operations: 0x48fb_ce06_e7ac_3c6b,
                unified: 0x3d09_cb11_4b5a_28ef,
            }
        );
    }
}
