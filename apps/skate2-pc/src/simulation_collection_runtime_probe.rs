//! Whole-BAM gate for active and staged deferred Simulation collection order.

use skate2_core::{
    collection_resident_streamer::ResidentCollectionHold,
    grind::GrindAsset,
    location_arena_adapter::{
        LocationArenaRegistration, LocationArenaRegistrationAdapter, LocationArenaRuntimeSource,
    },
    location_arena_plan::LocationDataHandle,
    simulation_collection_adapter::{
        PreparedSimulationCollection, SimulationCollectionDmoPass,
        SimulationCollectionLocationPass, SimulationCollectionNestedFailureKind,
        SimulationCollectionNormalPass, SimulationCollectionPassDeregistrationFailure,
        SimulationCollectionRegistration, SimulationCollectionRegistrationAdapter,
    },
    simulation_collection_admission::{
        SimulationCollectionAdmissionInputs, SimulationCollectionDeregistrationRoute,
        SimulationCollectionRegistrationRoute, select_simulation_collection_deregistration_route,
        select_simulation_collection_registration_route,
    },
    simulation_collection_notification_router::{
        SimulationCollectionNotificationRouter, SimulationNotificationDeferredState,
        SimulationNotificationDispatch,
    },
    simulation_deferred_message_fifo::{
        DeferredSimulationEnqueueState, DeferredSimulationMessageDispatch,
        DeferredSimulationMessageFifo,
    },
    simulation_dmo_arena_adapter::{
        PreparedSimulationDmoArena, SimulationDmoArenaDispatch, SimulationDmoArenaRegistration,
        SimulationDmoArenaRegistrationAdapter, SimulationDmoLoadPolicy,
    },
    simulation_environment_unlock_router::SimulationEnvironmentUnlockRouter,
    simulation_normal_arena_adapter::{
        PreparedSimulationNormalArena, SimulationNormalArenaRegistration,
        SimulationNormalArenaRegistrationAdapter,
    },
    simulation_normal_arena_plan::{SimulationNormalArenaPlan, SimulationRecordHandle},
    simulation_pointer_arena_adapter::SimulationPointerArenaDispatch,
    simulation_registry_arena_adapter::SimulationRegistryArenaDispatch,
    simulation_resident_collection::{
        PreparedSimulationResidentCollection, SimulationResidentArenaAsset,
    },
    simulation_resident_registry::SimulationResidentLookup,
    simulation_static_arena_adapter::SimulationStaticGrindDispatch,
    world_stream_catalog::CollectionRegistrationSnapshot,
};
use std::{
    collections::BTreeMap,
    error::Error,
    io,
    sync::{Arc, Mutex},
};

const USERDATA_RAW: u32 = 7;
const NOTIFICATION_USERDATA_RAW: u32 = 10;
const RESIDENT_NOTIFICATION_REG_USERDATA_RAW: u32 = 0xa5a5_0008;
const RESIDENT_NOTIFICATION_DREG_USERDATA_RAW: u32 = 0x5a5a_0008;
const GROUND_CAPACITY: usize = 20_000;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const EXPECTED: Inventory = Inventory {
    collections: 284,
    arenas: 2_135,
    immediate_eligible: 205,
    deferred_eligible: 79,
    normal_calls: 2_135,
    location_calls: 2_135,
    dmo_calls: 2_135,
    sequence_mismatches: 0,
    scope: 0x13a9_8da6_233e_cb1d,
    registration: 0x26dc_539b_e7ee_aabc,
    zero_registration: 0x42f9_2d5d_50c7_4ec9,
    deferred_registration: 0x8b3a_8a23_042f_f350,
    deregistration: 0x350c_8e48_6be7_1e16,
    unified: 0x0497_8e44_d600_02fd,
    zero_unified: 0xb1af_e10e_126b_a608,
};

const EXPECTED_NOTIFICATION: NotificationInventory = NotificationInventory {
    collections: 284,
    immediate_collections: 205,
    deferred_collections: 79,
    immediate_arenas: 743,
    deferred_arenas: 1_392,
    immediate_registration_nodes: 205,
    deferred_arena_nodes: 1_392,
    deferred_finish_nodes: 79,
    registration_callbacks: 284,
    deregistration_nodes: 284,
    deregistration_callbacks: 284,
    nodes: 2_528,
    signature: 0x84ef_de6b_9be3_a067,
};

const EXPECTED_RESIDENT_NOTIFICATION: ResidentNotificationInventory =
    ResidentNotificationInventory {
        collections: 79,
        arenas: 1_392,
        setup_lookups: 79,
        terminal_lookups: 79,
        dreg_lookups: 79,
        preparations: 237,
        preparation_asset_visits: 4_176,
        arena_nodes: 1_392,
        finish_nodes: 79,
        registration_callbacks: 79,
        dreg_nodes: 79,
        deregistration_callbacks: 79,
        outer_nodes: 1_708,
        child_fifo_nodes: 1_550,
        normal_reg_calls: 1_392,
        location_reg_calls: 1_392,
        dmo_reg_calls: 1_392,
        dmo_dreg_calls: 1_392,
        normal_dreg_calls: 1_392,
        location_dreg_calls: 1_392,
        arena_lookup_attempts: 1_392,
        arena_lookup_hits: 1_392,
        arena_lookup_misses: 0,
        arena_identity_mismatches: 0,
        protocol_bytes: 277_522,
        signature: 0x9216_4ff9_ad83_bb59,
    };

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Inventory {
    collections: u32,
    arenas: u32,
    immediate_eligible: u32,
    deferred_eligible: u32,
    normal_calls: u32,
    location_calls: u32,
    dmo_calls: u32,
    sequence_mismatches: u32,
    scope: u64,
    registration: u64,
    zero_registration: u64,
    deferred_registration: u64,
    deregistration: u64,
    unified: u64,
    zero_unified: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NotificationInventory {
    collections: u32,
    immediate_collections: u32,
    deferred_collections: u32,
    immediate_arenas: u32,
    deferred_arenas: u32,
    immediate_registration_nodes: u32,
    deferred_arena_nodes: u32,
    deferred_finish_nodes: u32,
    registration_callbacks: u32,
    deregistration_nodes: u32,
    deregistration_callbacks: u32,
    nodes: u32,
    signature: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResidentNotificationInventory {
    pub(crate) collections: u32,
    pub(crate) arenas: u32,
    pub(crate) setup_lookups: u32,
    pub(crate) terminal_lookups: u32,
    pub(crate) dreg_lookups: u32,
    pub(crate) preparations: u32,
    pub(crate) preparation_asset_visits: u32,
    pub(crate) arena_nodes: u32,
    pub(crate) finish_nodes: u32,
    pub(crate) registration_callbacks: u32,
    pub(crate) dreg_nodes: u32,
    pub(crate) deregistration_callbacks: u32,
    pub(crate) outer_nodes: u32,
    pub(crate) child_fifo_nodes: u32,
    pub(crate) normal_reg_calls: u32,
    pub(crate) location_reg_calls: u32,
    pub(crate) dmo_reg_calls: u32,
    pub(crate) dmo_dreg_calls: u32,
    pub(crate) normal_dreg_calls: u32,
    pub(crate) location_dreg_calls: u32,
    pub(crate) arena_lookup_attempts: u32,
    pub(crate) arena_lookup_hits: u32,
    pub(crate) arena_lookup_misses: u32,
    pub(crate) arena_identity_mismatches: u32,
    pub(crate) protocol_bytes: u32,
    pub(crate) signature: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Registration,
    Deregistration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExpectedCall {
    tag: u8,
    arena_ordinal: u32,
    outer_id: u64,
    inner_id: u32,
    userdata_raw: Option<u32>,
}

#[derive(Debug)]
struct State {
    collection_ordinal: u32,
    collection_id: u64,
    arenas: Vec<(u64, u32)>,
    phase: Option<Phase>,
    expected: Vec<ExpectedCall>,
    cursor: usize,
    scope: u64,
    registration: u64,
    zero_registration: u64,
    deferred_registration: u64,
    deregistration: u64,
    scope_stream: Vec<u8>,
    registration_stream: Vec<u8>,
    zero_registration_stream: Vec<u8>,
    deferred_registration_stream: Vec<u8>,
    deregistration_stream: Vec<u8>,
    collections: u32,
    arena_count: u32,
    immediate_eligible: u32,
    deferred_eligible: u32,
    normal_calls: u32,
    location_calls: u32,
    dmo_calls: u32,
    sequence_mismatches: u32,
    deferred_arena_messages: u32,
    deferred_normal_calls: u32,
    deferred_location_calls: u32,
    deferred_zero_messages: u32,
    deferred_dmo_calls: u32,
    deferred_callbacks: u32,
}

impl State {
    fn new() -> Self {
        Self {
            collection_ordinal: 0,
            collection_id: 0,
            arenas: Vec::new(),
            phase: None,
            expected: Vec::new(),
            cursor: 0,
            scope: domain(b"SKATE2/BAM_SIM/OUTER_SCOPE\0V1\0"),
            registration: domain(b"SKATE2/BAM_SIM/OUTER_REG\0V1\0"),
            zero_registration: domain(b"SKATE2/BAM_SIM/OUTER_REG_ZERO\0V1\0"),
            deferred_registration: domain(b"SKATE2/BAM_SIM/OUTER_DEFERRED_REG\0V1\0"),
            deregistration: domain(b"SKATE2/BAM_SIM/OUTER_DREG\0V1\0"),
            scope_stream: Vec::new(),
            registration_stream: Vec::new(),
            zero_registration_stream: Vec::new(),
            deferred_registration_stream: Vec::new(),
            deregistration_stream: Vec::new(),
            collections: 0,
            arena_count: 0,
            immediate_eligible: 0,
            deferred_eligible: 0,
            normal_calls: 0,
            location_calls: 0,
            dmo_calls: 0,
            sequence_mismatches: 0,
            deferred_arena_messages: 0,
            deferred_normal_calls: 0,
            deferred_location_calls: 0,
            deferred_zero_messages: 0,
            deferred_dmo_calls: 0,
            deferred_callbacks: 0,
        }
    }

    fn begin_collection(
        &mut self,
        collection_ordinal: u32,
        collection_id: u64,
        filename: &str,
        declared_asset_count: u32,
        payload_size: u32,
        prepared: &PreparedSimulationCollection,
    ) -> Result<(), Box<dyn Error>> {
        self.collections = self
            .collections
            .checked_add(1)
            .ok_or("collection overflow")?;
        self.collection_ordinal = collection_ordinal;
        self.collection_id = collection_id;
        match select_simulation_collection_registration_route(
            SimulationCollectionAdmissionInputs {
                stream_type_raw: 1,
                defer_cvar_enabled: true,
                manager_defer_raw: 1,
                force_immediate: false,
            },
            || payload_size,
        ) {
            SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback => {
                self.immediate_eligible += 1;
            }
            SimulationCollectionRegistrationRoute::DeferredCollectionThenCallback => {
                self.deferred_eligible += 1;
            }
            SimulationCollectionRegistrationRoute::CallbackOnly => {
                return Err(probe_error("Simulation stream classified callback-only"));
            }
        }
        self.arenas.clear();
        self.arenas.try_reserve_exact(prepared.arena_count())?;

        let mut collection = vec![b'C'];
        push_u32(&mut collection, collection_ordinal);
        push_u64(&mut collection, collection_id);
        push_u32(&mut collection, u32::try_from(filename.len())?);
        collection.extend_from_slice(filename.as_bytes());
        push_u32(&mut collection, declared_asset_count);
        feed(&mut self.scope, &mut self.scope_stream, &collection);

        for arena_index in 0..prepared.arena_count() {
            let (outer_id, inner_id) = prepared
                .arena_identity(arena_index)
                .ok_or_else(|| probe_error("prepared collection lost arena identity"))?;
            let arena_ordinal = u32::try_from(arena_index)?;
            self.arenas.push((outer_id, inner_id));
            self.arena_count = self.arena_count.checked_add(1).ok_or("arena overflow")?;
            let record = arena_record(b'A', collection_ordinal, arena_ordinal, outer_id, inner_id);
            feed(&mut self.scope, &mut self.scope_stream, &record);
        }
        Ok(())
    }

    fn begin_phase(&mut self, phase: Phase, userdata_raw: u32) -> Result<(), Box<dyn Error>> {
        if self.phase.is_some() || self.cursor != 0 || !self.expected.is_empty() {
            return Err(probe_error("outer collection phase overlap"));
        }
        self.phase = Some(phase);
        self.expected.try_reserve_exact(self.arenas.len() * 3)?;
        let append = |expected: &mut Vec<ExpectedCall>, tag, arena_ordinal, outer_id, inner_id| {
            expected.push(ExpectedCall {
                tag,
                arena_ordinal,
                outer_id,
                inner_id,
                userdata_raw: (tag != b'L').then_some(userdata_raw),
            });
        };
        match phase {
            Phase::Registration => {
                for (index, &(outer_id, inner_id)) in self.arenas.iter().enumerate() {
                    let ordinal = u32::try_from(index)?;
                    append(&mut self.expected, b'N', ordinal, outer_id, inner_id);
                    append(&mut self.expected, b'L', ordinal, outer_id, inner_id);
                }
                for (index, &(outer_id, inner_id)) in self.arenas.iter().enumerate() {
                    append(
                        &mut self.expected,
                        b'D',
                        u32::try_from(index)?,
                        outer_id,
                        inner_id,
                    );
                }
            }
            Phase::Deregistration => {
                for (index, &(outer_id, inner_id)) in self.arenas.iter().enumerate() {
                    append(
                        &mut self.expected,
                        b'D',
                        u32::try_from(index)?,
                        outer_id,
                        inner_id,
                    );
                }
                for (index, &(outer_id, inner_id)) in self.arenas.iter().enumerate() {
                    let ordinal = u32::try_from(index)?;
                    append(&mut self.expected, b'N', ordinal, outer_id, inner_id);
                    append(&mut self.expected, b'L', ordinal, outer_id, inner_id);
                }
            }
        }
        let mut record = vec![b'C'];
        push_u32(&mut record, self.collection_ordinal);
        push_u64(&mut record, self.collection_id);
        push_u32(&mut record, u32::try_from(self.arenas.len())?);
        self.feed_phase(&record);
        if phase == Phase::Registration {
            feed(
                &mut self.zero_registration,
                &mut self.zero_registration_stream,
                &record,
            );
            feed(
                &mut self.deferred_registration,
                &mut self.deferred_registration_stream,
                &record,
            );
        }
        Ok(())
    }

    fn deferred_arena_message(&mut self, arena_index: usize) -> Result<(), Box<dyn Error>> {
        let &(outer_id, inner_id) = self
            .arenas
            .get(arena_index)
            .ok_or_else(|| probe_error("deferred arena message index is outside prefix"))?;
        let record = arena_record(
            b'A',
            self.collection_ordinal,
            u32::try_from(arena_index)?,
            outer_id,
            inner_id,
        );
        feed(
            &mut self.deferred_registration,
            &mut self.deferred_registration_stream,
            &record,
        );
        self.deferred_arena_messages += 1;
        Ok(())
    }

    fn deferred_activation_zero(&mut self) -> Result<(), Box<dyn Error>> {
        let mut record = vec![b'0'];
        push_u32(&mut record, self.collection_ordinal);
        push_u64(&mut record, self.collection_id);
        push_u32(&mut record, u32::try_from(self.arenas.len())?);
        feed(
            &mut self.deferred_registration,
            &mut self.deferred_registration_stream,
            &record,
        );
        self.deferred_zero_messages += 1;
        Ok(())
    }

    fn deferred_callback(&mut self) {
        let mut record = vec![b'K'];
        push_u32(&mut record, self.collection_ordinal);
        push_u64(&mut record, self.collection_id);
        feed(
            &mut self.deferred_registration,
            &mut self.deferred_registration_stream,
            &record,
        );
        self.deferred_callbacks += 1;
    }

    fn observe(&mut self, tag: u8, outer_id: u64, inner_id: u32, userdata_raw: Option<u32>) {
        let arena_ordinal = self
            .arenas
            .iter()
            .position(|identity| *identity == (outer_id, inner_id))
            .and_then(|index| u32::try_from(index).ok())
            .unwrap_or(u32::MAX);
        let actual = ExpectedCall {
            tag,
            arena_ordinal,
            outer_id,
            inner_id,
            userdata_raw,
        };
        if self.expected.get(self.cursor) != Some(&actual) {
            self.sequence_mismatches += 1;
        }
        self.cursor += 1;
        match tag {
            b'N' => {
                self.normal_calls += 1;
                if self.phase == Some(Phase::Registration) {
                    self.deferred_normal_calls += 1;
                }
            }
            b'L' => {
                self.location_calls += 1;
                if self.phase == Some(Phase::Registration) {
                    self.deferred_location_calls += 1;
                }
            }
            b'D' => {
                self.dmo_calls += 1;
                if self.phase == Some(Phase::Registration) {
                    self.deferred_dmo_calls += 1;
                }
            }
            _ => self.sequence_mismatches += 1,
        }
        self.feed_phase(&arena_record(
            tag,
            self.collection_ordinal,
            arena_ordinal,
            outer_id,
            inner_id,
        ));
        if self.phase == Some(Phase::Registration) {
            let record = arena_record(
                tag,
                self.collection_ordinal,
                arena_ordinal,
                outer_id,
                inner_id,
            );
            if tag == b'D' {
                feed(
                    &mut self.zero_registration,
                    &mut self.zero_registration_stream,
                    &record,
                );
            }
            feed(
                &mut self.deferred_registration,
                &mut self.deferred_registration_stream,
                &record,
            );
        }
    }

    fn finish_phase(&mut self) {
        if self.cursor != self.expected.len() {
            self.sequence_mismatches += 1;
        }
        let mut trailer = vec![b'Z'];
        push_u32(&mut trailer, self.collection_ordinal);
        self.feed_phase(&trailer);
        if self.phase == Some(Phase::Registration) {
            feed(
                &mut self.zero_registration,
                &mut self.zero_registration_stream,
                &trailer,
            );
            feed(
                &mut self.deferred_registration,
                &mut self.deferred_registration_stream,
                &trailer,
            );
        }
        self.phase = None;
        self.expected.clear();
        self.cursor = 0;
    }

    fn finish_corpus_streams(&mut self) {
        let mut scope_trailer = vec![b'Z'];
        push_u32(&mut scope_trailer, self.collections);
        push_u32(&mut scope_trailer, self.arena_count);
        let mut scope_stream = std::mem::take(&mut self.scope_stream);
        feed(&mut self.scope, &mut scope_stream, &scope_trailer);
        self.scope_stream = scope_stream;

        let mut phase_trailer = vec![b'Q'];
        push_u32(&mut phase_trailer, self.collections);
        push_u32(&mut phase_trailer, self.arena_count);
        push_u32(&mut phase_trailer, self.normal_calls / 2);
        push_u32(&mut phase_trailer, self.location_calls / 2);
        push_u32(&mut phase_trailer, self.dmo_calls / 2);
        let mut registration_stream = std::mem::take(&mut self.registration_stream);
        feed(
            &mut self.registration,
            &mut registration_stream,
            &phase_trailer,
        );
        self.registration_stream = registration_stream;

        let mut zero_trailer = vec![b'Q'];
        push_u32(&mut zero_trailer, self.collections);
        push_u32(&mut zero_trailer, self.arena_count);
        push_u32(&mut zero_trailer, 0);
        push_u32(&mut zero_trailer, 0);
        push_u32(&mut zero_trailer, self.deferred_dmo_calls);
        let mut zero_registration_stream = std::mem::take(&mut self.zero_registration_stream);
        feed(
            &mut self.zero_registration,
            &mut zero_registration_stream,
            &zero_trailer,
        );
        self.zero_registration_stream = zero_registration_stream;

        let mut deferred_trailer = vec![b'Q'];
        push_u32(&mut deferred_trailer, self.collections);
        push_u32(&mut deferred_trailer, self.arena_count);
        push_u32(&mut deferred_trailer, self.deferred_arena_messages);
        push_u32(&mut deferred_trailer, self.deferred_normal_calls);
        push_u32(&mut deferred_trailer, self.deferred_location_calls);
        push_u32(&mut deferred_trailer, self.deferred_zero_messages);
        push_u32(&mut deferred_trailer, self.deferred_dmo_calls);
        push_u32(&mut deferred_trailer, self.deferred_callbacks);
        let mut deferred_registration_stream =
            std::mem::take(&mut self.deferred_registration_stream);
        feed(
            &mut self.deferred_registration,
            &mut deferred_registration_stream,
            &deferred_trailer,
        );
        self.deferred_registration_stream = deferred_registration_stream;

        let mut deregistration_stream = std::mem::take(&mut self.deregistration_stream);
        feed(
            &mut self.deregistration,
            &mut deregistration_stream,
            &phase_trailer,
        );
        self.deregistration_stream = deregistration_stream;
    }

    fn feed_phase(&mut self, bytes: &[u8]) {
        match self.phase {
            Some(Phase::Registration) => {
                feed(&mut self.registration, &mut self.registration_stream, bytes);
            }
            Some(Phase::Deregistration) => feed(
                &mut self.deregistration,
                &mut self.deregistration_stream,
                bytes,
            ),
            None => self.sequence_mismatches += 1,
        }
    }
}

#[derive(Default)]
struct NoopNormalDispatch;

impl SimulationStaticGrindDispatch for NoopNormalDispatch {
    fn add_spline_data(&mut self, _: &GrindAsset) {}
    fn remove_spline_data(&mut self, _: u64) {}
}

impl SimulationPointerArenaDispatch for NoopNormalDispatch {
    fn add_dmo_model(&mut self, _: u64) {}
    fn register_dmo_model(&mut self, _: &SimulationRecordHandle, _: u64) {}
    fn remove_dmo_model(&mut self, _: u64) {}
    fn deregister_dmo_model(&mut self, _: &SimulationRecordHandle, _: u64) {}
    fn add_trigger_volume(&mut self, _: &SimulationRecordHandle) {}
    fn remove_trigger_volume(&mut self, _: &SimulationRecordHandle) {}
}

impl SimulationRegistryArenaDispatch for NoopNormalDispatch {
    fn register_streamed_tile(&mut self, _: u64) {}
    fn unregister_streamed_tile(&mut self, _: u64) {}
    fn add_nav_mesh_data(&mut self, _: u64) {}
    fn remove_nav_mesh_data(&mut self, _: u64) {}
    fn add_rain_data(&mut self, _: u64) {}
    fn remove_rain_data(&mut self, _: u64) {}
    fn add_ai_path_asset(&mut self, _: u64) {}
    fn remove_ai_path_asset(&mut self, _: u64) {}
    fn register_waypoint_data(&mut self, _: u64, _: bool) {}
    fn deregister_waypoint_data(&mut self, _: u64, _: bool) {}
    fn add_spatial_map_asset(&mut self, _: u64) {}
    fn remove_spatial_map_asset(&mut self, _: u64) {}
}

type NormalInner =
    SimulationNormalArenaRegistrationAdapter<NoopNormalDispatch, SimulationEnvironmentUnlockRouter>;

struct ObservedNormal {
    inner: NormalInner,
    state: Arc<Mutex<State>>,
}

struct ObservedNormalRegistration {
    inner: SimulationNormalArenaRegistration,
    outer_id: u64,
    inner_id: u32,
}

impl SimulationCollectionNormalPass for ObservedNormal {
    type Registration = ObservedNormalRegistration;

    fn register_normal(
        &mut self,
        prepared: &PreparedSimulationNormalArena,
        userdata_raw: u32,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
        let plan = prepared.plan();
        let outer_id = plan.static_plan().asset().decoded.asset_id;
        let inner_id = plan.static_plan().asset().decoded.arena.id;
        let inner = self.inner.register_normal(prepared, userdata_raw)?;
        observe_identity(&self.state, b'N', outer_id, inner_id, Some(userdata_raw));
        Ok(ObservedNormalRegistration {
            inner,
            outer_id,
            inner_id,
        })
    }

    fn deregister_normal(
        &mut self,
        registration: Self::Registration,
        userdata_raw: u32,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
        let ObservedNormalRegistration {
            inner,
            outer_id,
            inner_id,
        } = registration;
        match self.inner.deregister_normal(inner, userdata_raw) {
            Ok(()) => {
                observe_identity(&self.state, b'N', outer_id, inner_id, Some(userdata_raw));
                Ok(())
            }
            Err(failure) => match failure.into_registration() {
                Some(inner) => Err(
                    SimulationCollectionPassDeregistrationFailure::before_effects(
                        ObservedNormalRegistration {
                            inner,
                            outer_id,
                            inner_id,
                        },
                    ),
                ),
                None => Err(SimulationCollectionPassDeregistrationFailure::reached_fault()),
            },
        }
    }
}

#[derive(Default)]
struct LocationRuntime {
    sources: Arc<Mutex<BTreeMap<u64, LocationDataHandle>>>,
}

impl LocationArenaRuntimeSource for LocationRuntime {
    fn resolve_location_data(&mut self, asset_id: u64) -> Option<LocationDataHandle> {
        self.sources.lock().ok()?.get(&asset_id).cloned()
    }

    fn high_priority_raw(&mut self) -> u8 {
        0
    }
}

type LocationInner = LocationArenaRegistrationAdapter<LocationRuntime>;

struct ObservedLocation {
    inner: LocationInner,
    state: Arc<Mutex<State>>,
}

struct ObservedLocationRegistration {
    inner: LocationArenaRegistration,
    outer_id: u64,
    inner_id: u32,
}

impl SimulationCollectionLocationPass for ObservedLocation {
    type Registration = ObservedLocationRegistration;

    fn register_location(
        &mut self,
        prepared: &skate2_core::location_arena_adapter::PreparedLocationArena,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
        let plan = prepared.plan().normal_plan();
        let outer_id = plan.static_plan().asset().decoded.asset_id;
        let inner_id = plan.static_plan().asset().decoded.arena.id;
        let inner = self.inner.register_location(prepared)?;
        observe_identity(&self.state, b'L', outer_id, inner_id, None);
        Ok(ObservedLocationRegistration {
            inner,
            outer_id,
            inner_id,
        })
    }

    fn deregister_location(
        &mut self,
        registration: Self::Registration,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
        let ObservedLocationRegistration {
            inner,
            outer_id,
            inner_id,
        } = registration;
        match self.inner.deregister_location(inner) {
            Ok(()) => {
                observe_identity(&self.state, b'L', outer_id, inner_id, None);
                Ok(())
            }
            Err(failure) => match failure.into_registration() {
                Some(inner) => Err(
                    SimulationCollectionPassDeregistrationFailure::before_effects(
                        ObservedLocationRegistration {
                            inner,
                            outer_id,
                            inner_id,
                        },
                    ),
                ),
                None => Err(SimulationCollectionPassDeregistrationFailure::reached_fault()),
            },
        }
    }
}

#[derive(Default)]
struct DmoPolicy;

impl SimulationDmoLoadPolicy for DmoPolicy {
    fn load_dmo_locator_from_stream(&mut self) -> bool {
        true
    }
}

#[derive(Default)]
struct NoopDmoDispatch;

impl SimulationDmoArenaDispatch for NoopDmoDispatch {
    fn add_living_world_locator_data(&mut self, _: u64, _: u32) {}
    fn add_challenge_dmos(&mut self, _: u64, _: u32) {}
    fn register_dmo_locator(
        &mut self,
        _: &skate2_core::simulation_dmo_arena_plan::SimulationDmoDataHandle,
        _: u64,
    ) {
    }
    fn remove_living_world_locator_data(&mut self, _: u64, _: u32) {}
    fn remove_challenge_dmos(&mut self, _: u64, _: u32) {}
    fn deregister_dmo_locator(
        &mut self,
        _: &skate2_core::simulation_dmo_arena_plan::SimulationDmoDataHandle,
        _: u64,
    ) {
    }
}

type DmoInner = SimulationDmoArenaRegistrationAdapter<DmoPolicy, NoopDmoDispatch>;

struct ObservedDmo {
    inner: DmoInner,
    state: Arc<Mutex<State>>,
}

struct ObservedDmoRegistration {
    inner: SimulationDmoArenaRegistration,
    outer_id: u64,
    inner_id: u32,
}

impl SimulationCollectionDmoPass for ObservedDmo {
    type Registration = ObservedDmoRegistration;

    fn register_dmo(
        &mut self,
        prepared: &PreparedSimulationDmoArena,
        userdata_raw: u32,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
        let plan = prepared.plan().normal_plan();
        let outer_id = plan.static_plan().asset().decoded.asset_id;
        let inner_id = plan.static_plan().asset().decoded.arena.id;
        let inner = self.inner.register_dmo(prepared, userdata_raw)?;
        observe_identity(&self.state, b'D', outer_id, inner_id, Some(userdata_raw));
        Ok(ObservedDmoRegistration {
            inner,
            outer_id,
            inner_id,
        })
    }

    fn deregister_dmo(
        &mut self,
        registration: Self::Registration,
        userdata_raw: u32,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
        let ObservedDmoRegistration {
            inner,
            outer_id,
            inner_id,
        } = registration;
        match self.inner.deregister_dmo(inner, userdata_raw) {
            Ok(()) => {
                observe_identity(&self.state, b'D', outer_id, inner_id, Some(userdata_raw));
                Ok(())
            }
            Err(failure) => match failure.into_registration() {
                Some(inner) => Err(
                    SimulationCollectionPassDeregistrationFailure::before_effects(
                        ObservedDmoRegistration {
                            inner,
                            outer_id,
                            inner_id,
                        },
                    ),
                ),
                None => Err(SimulationCollectionPassDeregistrationFailure::reached_fault()),
            },
        }
    }
}

fn observe_identity(
    state: &Arc<Mutex<State>>,
    tag: u8,
    outer_id: u64,
    inner_id: u32,
    userdata_raw: Option<u32>,
) {
    state
        .lock()
        .expect("outer collection state poisoned")
        .observe(tag, outer_id, inner_id, userdata_raw);
}

type Adapter =
    SimulationCollectionRegistrationAdapter<ObservedNormal, ObservedLocation, ObservedDmo>;

pub(crate) struct Runtime {
    // Drop queued callbacks/continuations before their borrowed receiver domain.
    fifo: DeferredSimulationMessageFifo<(u32, u64)>,
    adapter: Adapter,
    state: Arc<Mutex<State>>,
    location_sources: Arc<Mutex<BTreeMap<u64, LocationDataHandle>>>,
}

impl Runtime {
    pub(crate) fn try_new() -> Result<Self, Box<dyn Error>> {
        let state = Arc::new(Mutex::new(State::new()));
        let normal = ObservedNormal {
            inner: SimulationNormalArenaRegistrationAdapter::new(
                NoopNormalDispatch,
                SimulationEnvironmentUnlockRouter::try_new_empty(GROUND_CAPACITY)?,
            ),
            state: Arc::clone(&state),
        };
        let location_sources = Arc::new(Mutex::new(BTreeMap::new()));
        let location = ObservedLocation {
            inner: LocationArenaRegistrationAdapter::new(LocationRuntime {
                sources: Arc::clone(&location_sources),
            }),
            state: Arc::clone(&state),
        };
        let dmo = ObservedDmo {
            inner: SimulationDmoArenaRegistrationAdapter::new(DmoPolicy, NoopDmoDispatch),
            state: Arc::clone(&state),
        };
        Ok(Self {
            fifo: DeferredSimulationMessageFifo::new(),
            adapter: SimulationCollectionRegistrationAdapter::new(normal, location, dmo),
            state,
            location_sources,
        })
    }

    fn exercise_deferred_fifo(
        &mut self,
        collection_ordinal: usize,
        collection_id: u64,
        prepared: &PreparedSimulationCollection,
    ) -> Result<SimulationCollectionRegistration, Box<dyn Error>> {
        let mut enqueue = self.fifo.begin(&mut self.adapter, prepared, USERDATA_RAW)?;
        let mut enqueued_arenas = 0;
        let finish = loop {
            match enqueue {
                DeferredSimulationEnqueueState::NextArena(authority) => {
                    if self.fifo.pending_message_count() != 0 {
                        return Err(probe_error("deferred arena queue did not begin empty"));
                    }
                    enqueue = self.fifo.enqueue_next_arena(authority)?;
                    self.state
                        .lock()
                        .expect("outer collection state poisoned")
                        .deferred_arena_message(enqueued_arenas)?;
                    enqueued_arenas += 1;
                    if self.fifo.pending_message_count() != 1
                        || !matches!(
                            self.fifo.dispatch_one(&mut self.adapter)?,
                            DeferredSimulationMessageDispatch::ArenaMessageReturned
                        )
                        || self.fifo.pending_message_count() != 0
                    {
                        return Err(probe_error(
                            "deferred arena did not make one exact FIFO round trip",
                        ));
                    }
                }
                DeferredSimulationEnqueueState::ReadyToFinish(authority) => break authority,
            }
        };
        if enqueued_arenas != prepared.arena_count() {
            return Err(probe_error("deferred producer arena count mismatch"));
        }
        let callback = self.fifo.enqueue_finish(finish)?;
        self.state
            .lock()
            .expect("outer collection state poisoned")
            .deferred_activation_zero()?;
        let occurrence = (u32::try_from(collection_ordinal)?, collection_id);
        self.fifo.enqueue_callback(callback, Some(occurrence))?;
        if self.fifo.pending_message_count() != 2
            || !matches!(
                self.fifo.dispatch_one(&mut self.adapter)?,
                DeferredSimulationMessageDispatch::ActivationZeroMessageReturned
            )
            || self.fifo.pending_message_count() != 1
        {
            return Err(probe_error(
                "activation-zero message did not precede its callback",
            ));
        }
        let DeferredSimulationMessageDispatch::CallbackMessageDequeued(completion) =
            self.fifo.dispatch_one(&mut self.adapter)?
        else {
            return Err(probe_error("deferred callback was not dequeued last"));
        };
        let (completed_id, registration, callback) = completion.into_parts();
        if completed_id != collection_id || callback != Some(occurrence) {
            return Err(probe_error(
                "deferred callback occurrence identity mismatch",
            ));
        }
        self.state
            .lock()
            .expect("outer collection state poisoned")
            .deferred_callback();
        if self.fifo.pending_message_count() != 0
            || self.fifo.retained_session_count() != 0
            || self.fifo.fault().is_some()
            || !matches!(
                self.fifo.dispatch_one(&mut self.adapter)?,
                DeferredSimulationMessageDispatch::Idle
            )
        {
            return Err(probe_error("deferred FIFO retained completed work"));
        }
        Ok(registration)
    }

    pub(crate) fn exercise(
        &mut self,
        collection_ordinal: usize,
        collection_id: u64,
        filename: &str,
        declared_asset_count: u32,
        payload_size: u32,
        plans: Vec<Arc<SimulationNormalArenaPlan>>,
    ) -> Result<(), Box<dyn Error>> {
        let prepared = PreparedSimulationCollection::try_new(collection_id, plans)?;
        for arena_index in 0..prepared.arena_count() {
            let location = prepared
                .location_arena(arena_index)
                .ok_or_else(|| probe_error("prepared collection lost Location arena"))?;
            for container_index in 0..location.container_count() {
                let operation = location
                    .operation(container_index)
                    .ok_or_else(|| probe_error("prepared Location operation disappeared"))?;
                let source = location
                    .planned_source(container_index)
                    .ok_or_else(|| probe_error("prepared Location source disappeared"))?;
                let mut sources = self
                    .location_sources
                    .lock()
                    .map_err(|_| probe_error("Location source store poisoned"))?;
                if sources.contains_key(&operation.asset_id) {
                    return Err(probe_error("duplicate Location container ID in collection"));
                }
                sources.insert(operation.asset_id, source);
            }
        }
        {
            let mut state = self.state.lock().expect("outer collection state poisoned");
            state.begin_collection(
                u32::try_from(collection_ordinal)?,
                collection_id,
                filename,
                declared_asset_count,
                payload_size,
                &prepared,
            )?;
            state.begin_phase(Phase::Registration, USERDATA_RAW)?;
        }
        let registration =
            self.exercise_deferred_fifo(collection_ordinal, collection_id, &prepared)?;
        self.state
            .lock()
            .expect("outer collection state poisoned")
            .finish_phase();
        self.state
            .lock()
            .expect("outer collection state poisoned")
            .begin_phase(Phase::Deregistration, USERDATA_RAW)?;
        self.adapter.deregister(registration, USERDATA_RAW)?;
        self.state
            .lock()
            .expect("outer collection state poisoned")
            .finish_phase();
        self.location_sources
            .lock()
            .map_err(|_| probe_error("Location source store poisoned"))?
            .clear();
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<Inventory, Box<dyn Error>> {
        if self.fifo.pending_message_count() != 0
            || self.fifo.retained_session_count() != 0
            || self.fifo.fault().is_some()
            || self.adapter.live_registration_count() != 0
            || self.adapter.fault().is_some()
            || self.adapter.normal().inner.live_registration_count() != 0
            || self.adapter.location().inner.live_registration_count() != 0
            || self.adapter.dmo().inner.live_registration_count() != 0
            || self.adapter.normal().inner.orphaned_source_count() != 0
            || self.adapter.dmo().inner.orphaned_source_count() != 0
            || !self.adapter.location().inner.entries().is_empty()
            || !self
                .location_sources
                .lock()
                .map_err(|_| probe_error("Location source store poisoned"))?
                .is_empty()
            || self
                .adapter
                .normal()
                .inner
                .environment()
                .published_volume_count()
                != 0
        {
            return Err(probe_error(
                "outer collection runtime did not finish quiescent",
            ));
        }
        let mut state = self.state.lock().expect("outer collection state poisoned");
        state.finish_corpus_streams();
        let mut unified = domain(b"SKATE2/BAM_SIM/OUTER_UNIFIED\0V1\0");
        hash(&mut unified, &state.scope_stream);
        hash(&mut unified, b"R");
        hash(&mut unified, &state.registration_stream);
        hash(&mut unified, b"D");
        hash(&mut unified, &state.deregistration_stream);
        let mut zero_unified = domain(b"SKATE2/BAM_SIM/OUTER_ZERO_UNIFIED\0V1\0");
        hash(&mut zero_unified, &state.scope_stream);
        hash(&mut zero_unified, b"R");
        hash(&mut zero_unified, &state.zero_registration_stream);
        let inventory = Inventory {
            collections: state.collections,
            arenas: state.arena_count,
            immediate_eligible: state.immediate_eligible,
            deferred_eligible: state.deferred_eligible,
            normal_calls: state.normal_calls / 2,
            location_calls: state.location_calls / 2,
            dmo_calls: state.dmo_calls / 2,
            sequence_mismatches: state.sequence_mismatches,
            scope: state.scope,
            registration: state.registration,
            zero_registration: state.zero_registration,
            deferred_registration: state.deferred_registration,
            deregistration: state.deregistration,
            unified,
            zero_unified,
        };
        if inventory != EXPECTED {
            return Err(probe_error(format!(
                "outer collection corpus mismatch: expected {EXPECTED:?}, got {inventory:?}"
            )));
        }
        Ok(inventory)
    }
}

type NotificationCallback = (u32, u64);
type NotificationRouter = SimulationCollectionNotificationRouter<
    NotificationCallback,
    NotificationCallback,
    ObservedNormal,
    ObservedLocation,
    ObservedDmo,
>;

pub(crate) struct NotificationRuntime {
    // Router owns every opaque collection authority and is dropped before the
    // independent observer/source stores below.
    router: NotificationRouter,
    state: Arc<Mutex<State>>,
    location_sources: Arc<Mutex<BTreeMap<u64, LocationDataHandle>>>,
    inventory: NotificationInventory,
}

impl NotificationRuntime {
    pub(crate) fn try_new() -> Result<Self, Box<dyn Error>> {
        let state = Arc::new(Mutex::new(State::new()));
        let normal = ObservedNormal {
            inner: SimulationNormalArenaRegistrationAdapter::new(
                NoopNormalDispatch,
                SimulationEnvironmentUnlockRouter::try_new_empty(GROUND_CAPACITY)?,
            ),
            state: Arc::clone(&state),
        };
        let location_sources = Arc::new(Mutex::new(BTreeMap::new()));
        let location = ObservedLocation {
            inner: LocationArenaRegistrationAdapter::new(LocationRuntime {
                sources: Arc::clone(&location_sources),
            }),
            state: Arc::clone(&state),
        };
        let dmo = ObservedDmo {
            inner: SimulationDmoArenaRegistrationAdapter::new(DmoPolicy, NoopDmoDispatch),
            state: Arc::clone(&state),
        };
        Ok(Self {
            router: SimulationCollectionNotificationRouter::new(normal, location, dmo),
            state,
            location_sources,
            inventory: NotificationInventory {
                collections: 0,
                immediate_collections: 0,
                deferred_collections: 0,
                immediate_arenas: 0,
                deferred_arenas: 0,
                immediate_registration_nodes: 0,
                deferred_arena_nodes: 0,
                deferred_finish_nodes: 0,
                registration_callbacks: 0,
                deregistration_nodes: 0,
                deregistration_callbacks: 0,
                nodes: 0,
                signature: domain(b"SKATE2/BAM_SIM/NOTIFICATION_ROUTER\0V1\0"),
            },
        })
    }

    fn count_node(&mut self) -> Result<(), Box<dyn Error>> {
        self.inventory.nodes = self
            .inventory
            .nodes
            .checked_add(1)
            .ok_or("notification node count overflow")?;
        Ok(())
    }

    fn publish_location_sources(
        &self,
        prepared: &PreparedSimulationCollection,
    ) -> Result<(), Box<dyn Error>> {
        for arena_index in 0..prepared.arena_count() {
            let location = prepared
                .location_arena(arena_index)
                .ok_or_else(|| probe_error("notification collection lost Location arena"))?;
            for container_index in 0..location.container_count() {
                let operation = location
                    .operation(container_index)
                    .ok_or_else(|| probe_error("notification Location operation disappeared"))?;
                let source = location
                    .planned_source(container_index)
                    .ok_or_else(|| probe_error("notification Location source disappeared"))?;
                let mut sources = self
                    .location_sources
                    .lock()
                    .map_err(|_| probe_error("notification Location source store poisoned"))?;
                if sources.insert(operation.asset_id, source).is_some() {
                    return Err(probe_error(
                        "duplicate notification Location container ID in collection",
                    ));
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(crate) fn exercise(
        &mut self,
        collection_ordinal: usize,
        collection_id: u64,
        filename: &str,
        declared_asset_count: u32,
        payload_size: u32,
        plans: Vec<Arc<SimulationNormalArenaPlan>>,
    ) -> Result<(), Box<dyn Error>> {
        let prepared = PreparedSimulationCollection::try_new(collection_id, plans)?;
        self.publish_location_sources(&prepared)?;
        let ordinal = u32::try_from(collection_ordinal)?;
        let callback = (ordinal, collection_id);
        let arena_count = u32::try_from(prepared.arena_count())?;
        let mut arenas = Vec::new();
        arenas.try_reserve_exact(prepared.arena_count())?;
        for arena_index in 0..prepared.arena_count() {
            arenas.push(
                prepared
                    .arena_identity(arena_index)
                    .ok_or_else(|| probe_error("notification collection lost arena identity"))?,
            );
        }
        let route = select_simulation_collection_registration_route(
            SimulationCollectionAdmissionInputs {
                stream_type_raw: 1,
                defer_cvar_enabled: true,
                manager_defer_raw: 1,
                force_immediate: false,
            },
            || payload_size,
        );
        let route_raw = match route {
            SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback => 0,
            SimulationCollectionRegistrationRoute::DeferredCollectionThenCallback => 1,
            SimulationCollectionRegistrationRoute::CallbackOnly => {
                return Err(probe_error(
                    "raw Simulation type selected callback-only route",
                ));
            }
        };
        let mut collection = vec![b'C'];
        push_u32(&mut collection, ordinal);
        push_u64(&mut collection, collection_id);
        push_u32(&mut collection, 1);
        collection.push(1);
        push_u32(&mut collection, 1);
        collection.push(0);
        push_u32(&mut collection, NOTIFICATION_USERDATA_RAW);
        push_u32(&mut collection, payload_size);
        push_u32(&mut collection, arena_count);
        collection.push(route_raw);
        hash(&mut self.inventory.signature, &collection);
        self.inventory.collections += 1;

        {
            let mut state = self.state.lock().expect("notification state poisoned");
            state.begin_collection(
                ordinal,
                collection_id,
                filename,
                declared_asset_count,
                payload_size,
                &prepared,
            )?;
            state.begin_phase(Phase::Registration, NOTIFICATION_USERDATA_RAW)?;
        }

        let occurrence = match route {
            SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback => {
                self.inventory.immediate_collections += 1;
                self.inventory.immediate_arenas = self
                    .inventory
                    .immediate_arenas
                    .checked_add(arena_count)
                    .ok_or("immediate arena count overflow")?;
                let callback_permit = self.router.enqueue_immediate_registration(
                    prepared,
                    NOTIFICATION_USERDATA_RAW,
                    Some(callback),
                )?;
                self.router.enqueue_registration_callback(callback_permit)?;
                if !matches!(
                    self.router.dispatch_one()?,
                    SimulationNotificationDispatch::ReceiverReturned
                ) {
                    return Err(probe_error("immediate registration node was not first"));
                }
                let mut record = vec![b'I'];
                push_u32(&mut record, ordinal);
                push_u64(&mut record, collection_id);
                hash(&mut self.inventory.signature, &record);
                self.inventory.immediate_registration_nodes += 1;
                self.count_node()?;
                self.state
                    .lock()
                    .expect("notification state poisoned")
                    .finish_phase();
                let SimulationNotificationDispatch::RegistrationCallbackDequeued(delivery) =
                    self.router.dispatch_one()?
                else {
                    return Err(probe_error(
                        "immediate registration callback was not second",
                    ));
                };
                let (delivered_id, occurrence, delivered_callback) = delivery.into_parts();
                if delivered_id != collection_id || delivered_callback != Some(callback) {
                    return Err(probe_error(
                        "immediate registration callback identity mismatch",
                    ));
                }
                occurrence
            }
            SimulationCollectionRegistrationRoute::DeferredCollectionThenCallback => {
                self.inventory.deferred_collections += 1;
                self.inventory.deferred_arenas = self
                    .inventory
                    .deferred_arenas
                    .checked_add(arena_count)
                    .ok_or("deferred arena count overflow")?;
                let mut producer = self.router.begin_deferred_registration(
                    prepared,
                    NOTIFICATION_USERDATA_RAW,
                    Some(callback),
                )?;
                let finish = loop {
                    match producer {
                        SimulationNotificationDeferredState::NextArena(arena) => {
                            producer = self.router.enqueue_deferred_arena(arena)?;
                            let arena_ordinal = self.inventory.deferred_arena_nodes;
                            let local_ordinal = arena_ordinal
                                .checked_sub(self.inventory.deferred_arenas - arena_count)
                                .ok_or("deferred local arena ordinal underflow")?;
                            let (outer_id, inner_id) = arenas
                                .get(usize::try_from(local_ordinal)?)
                                .copied()
                                .ok_or_else(|| probe_error("deferred arena identity overflow"))?;
                            if !matches!(
                                self.router.dispatch_one()?,
                                SimulationNotificationDispatch::ReceiverReturned
                            ) {
                                return Err(probe_error("deferred Arena marker did not dispatch"));
                            }
                            let record =
                                arena_record(b'A', ordinal, local_ordinal, outer_id, inner_id);
                            hash(&mut self.inventory.signature, &record);
                            self.inventory.deferred_arena_nodes += 1;
                            self.count_node()?;
                        }
                        SimulationNotificationDeferredState::ReadyToFinish(finish) => break finish,
                    }
                };
                let callback_permit = self.router.enqueue_deferred_finish(finish)?;
                self.router.enqueue_registration_callback(callback_permit)?;
                if !matches!(
                    self.router.dispatch_one()?,
                    SimulationNotificationDispatch::ReceiverReturned
                ) {
                    return Err(probe_error("deferred finish marker did not dispatch"));
                }
                let mut record = vec![b'0'];
                push_u32(&mut record, ordinal);
                push_u64(&mut record, collection_id);
                push_u32(&mut record, arena_count);
                hash(&mut self.inventory.signature, &record);
                self.inventory.deferred_finish_nodes += 1;
                self.count_node()?;
                self.state
                    .lock()
                    .expect("notification state poisoned")
                    .finish_phase();
                let SimulationNotificationDispatch::RegistrationCallbackDequeued(delivery) =
                    self.router.dispatch_one()?
                else {
                    return Err(probe_error("deferred registration callback was not last"));
                };
                let (delivered_id, occurrence, delivered_callback) = delivery.into_parts();
                if delivered_id != collection_id || delivered_callback != Some(callback) {
                    return Err(probe_error(
                        "deferred registration callback identity mismatch",
                    ));
                }
                occurrence
            }
            SimulationCollectionRegistrationRoute::CallbackOnly => unreachable!(),
        };

        let mut record = vec![b'R'];
        push_u32(&mut record, ordinal);
        push_u64(&mut record, collection_id);
        hash(&mut self.inventory.signature, &record);
        self.inventory.registration_callbacks += 1;
        self.count_node()?;
        self.state
            .lock()
            .expect("notification state poisoned")
            .begin_phase(Phase::Deregistration, NOTIFICATION_USERDATA_RAW)?;
        let dreg_route = select_simulation_collection_deregistration_route(1);
        if dreg_route != SimulationCollectionDeregistrationRoute::DeregistrationThenCallback {
            return Err(probe_error("raw Simulation type skipped DRegCollection"));
        }
        let dreg_callback = self.router.enqueue_deregistration(
            occurrence,
            NOTIFICATION_USERDATA_RAW,
            dreg_route,
            Some(callback),
        )?;
        self.router.enqueue_deregistration_callback(dreg_callback)?;
        if !matches!(
            self.router.dispatch_one()?,
            SimulationNotificationDispatch::ReceiverReturned
        ) {
            return Err(probe_error("DRegCollection node was not first"));
        }
        let mut record = vec![b'D'];
        push_u32(&mut record, ordinal);
        push_u64(&mut record, collection_id);
        hash(&mut self.inventory.signature, &record);
        self.inventory.deregistration_nodes += 1;
        self.count_node()?;
        self.state
            .lock()
            .expect("notification state poisoned")
            .finish_phase();
        let SimulationNotificationDispatch::DeregistrationCallbackDequeued(delivery) =
            self.router.dispatch_one()?
        else {
            return Err(probe_error("deregistration callback was not second"));
        };
        let (delivered_id, delivered_callback) = delivery.into_parts();
        if delivered_id != collection_id || delivered_callback != Some(callback) {
            return Err(probe_error("deregistration callback identity mismatch"));
        }
        let mut record = vec![b'U'];
        push_u32(&mut record, ordinal);
        push_u64(&mut record, collection_id);
        hash(&mut self.inventory.signature, &record);
        self.inventory.deregistration_callbacks += 1;
        self.count_node()?;
        self.location_sources
            .lock()
            .map_err(|_| probe_error("notification Location source store poisoned"))?
            .clear();
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<NotificationInventory, Box<dyn Error>> {
        if self.router.pending_message_count() != 0
            || self.router.retained_session_count() != 0
            || self.router.live_collection_registration_count() != 0
            || self.router.deferred_collection_registration_count() != 0
            || self.router.completed_collection_registration_count() != 0
            || self.router.fault().is_some()
            || self.router.collection_fault().is_some()
            || !self
                .location_sources
                .lock()
                .map_err(|_| probe_error("notification Location source store poisoned"))?
                .is_empty()
            || !matches!(
                self.router.dispatch_one()?,
                SimulationNotificationDispatch::Idle
            )
        {
            return Err(probe_error("notification router did not finish quiescent"));
        }
        let state = self.state.lock().expect("notification state poisoned");
        if state.phase.is_some()
            || state.cursor != 0
            || !state.expected.is_empty()
            || state.sequence_mismatches != 0
            || state.collections != self.inventory.collections
            || state.arena_count != self.inventory.immediate_arenas + self.inventory.deferred_arenas
            || state.normal_calls != state.arena_count * 2
            || state.location_calls != state.arena_count * 2
            || state.dmo_calls != state.arena_count * 2
        {
            return Err(probe_error(
                "notification child receiver sequence did not finish exactly",
            ));
        }
        drop(state);
        let mut trailer = vec![b'Q'];
        for value in [
            self.inventory.collections,
            self.inventory.immediate_collections,
            self.inventory.deferred_collections,
            self.inventory.immediate_arenas,
            self.inventory.deferred_arenas,
            self.inventory.immediate_registration_nodes,
            self.inventory.deferred_arena_nodes,
            self.inventory.deferred_finish_nodes,
            self.inventory.registration_callbacks,
            self.inventory.deregistration_nodes,
            self.inventory.deregistration_callbacks,
        ] {
            push_u32(&mut trailer, value);
        }
        hash(&mut self.inventory.signature, &trailer);
        if self.inventory != EXPECTED_NOTIFICATION {
            return Err(probe_error(format!(
                "notification router corpus mismatch: expected {EXPECTED_NOTIFICATION:?}, got {:?}",
                self.inventory
            )));
        }
        Ok(self.inventory)
    }
}

#[derive(Debug)]
struct ResidentRegistrationCallback {
    collection_ordinal: u32,
    collection_id: u64,
    hold: ResidentCollectionHold,
}

type ResidentNotificationRouter = SimulationCollectionNotificationRouter<
    ResidentRegistrationCallback,
    NotificationCallback,
    ObservedNormal,
    ObservedLocation,
    ObservedDmo,
>;

pub(crate) struct ResidentNotificationRuntime {
    // Router owns every opaque collection authority and is dropped before the
    // independent observer/source stores below.
    router: ResidentNotificationRouter,
    state: Arc<Mutex<State>>,
    location_sources: Arc<Mutex<BTreeMap<u64, LocationDataHandle>>>,
    inventory: ResidentNotificationInventory,
}

impl ResidentNotificationRuntime {
    pub(crate) fn try_new(
        lookup: SimulationResidentLookup<SimulationResidentArenaAsset>,
    ) -> Result<Self, Box<dyn Error>> {
        let state = Arc::new(Mutex::new(State::new()));
        let normal = ObservedNormal {
            inner: SimulationNormalArenaRegistrationAdapter::new(
                NoopNormalDispatch,
                SimulationEnvironmentUnlockRouter::try_new_empty(GROUND_CAPACITY)?,
            ),
            state: Arc::clone(&state),
        };
        let location_sources = Arc::new(Mutex::new(BTreeMap::new()));
        let location = ObservedLocation {
            inner: LocationArenaRegistrationAdapter::new(LocationRuntime {
                sources: Arc::clone(&location_sources),
            }),
            state: Arc::clone(&state),
        };
        let dmo = ObservedDmo {
            inner: SimulationDmoArenaRegistrationAdapter::new(DmoPolicy, NoopDmoDispatch),
            state: Arc::clone(&state),
        };
        Ok(Self {
            router: SimulationCollectionNotificationRouter::new_with_resident_lookup(
                normal, location, dmo, lookup,
            ),
            state,
            location_sources,
            inventory: ResidentNotificationInventory {
                collections: 0,
                arenas: 0,
                setup_lookups: 0,
                terminal_lookups: 0,
                dreg_lookups: 0,
                preparations: 0,
                preparation_asset_visits: 0,
                arena_nodes: 0,
                finish_nodes: 0,
                registration_callbacks: 0,
                dreg_nodes: 0,
                deregistration_callbacks: 0,
                outer_nodes: 0,
                child_fifo_nodes: 0,
                normal_reg_calls: 0,
                location_reg_calls: 0,
                dmo_reg_calls: 0,
                dmo_dreg_calls: 0,
                normal_dreg_calls: 0,
                location_dreg_calls: 0,
                arena_lookup_attempts: 0,
                arena_lookup_hits: 0,
                arena_lookup_misses: 0,
                arena_identity_mismatches: 0,
                protocol_bytes: 49,
                signature: domain(b"SKATE2/BAM_SIM/RESIDENT_DEFERRED_NOTIFICATION\0V2\0"),
            },
        })
    }

    fn add(
        &mut self,
        field: fn(&mut ResidentNotificationInventory) -> &mut u32,
        amount: u32,
        label: &str,
    ) -> Result<(), Box<dyn Error>> {
        let value = field(&mut self.inventory);
        *value = value
            .checked_add(amount)
            .ok_or_else(|| probe_error(format!("resident notification {label} count overflow")))?;
        Ok(())
    }

    fn hash_record(&mut self, record: &[u8]) -> Result<(), Box<dyn Error>> {
        hash(&mut self.inventory.signature, record);
        let record_bytes = u32::try_from(record.len())?;
        self.inventory.protocol_bytes = self
            .inventory
            .protocol_bytes
            .checked_add(record_bytes)
            .ok_or_else(|| probe_error("resident notification protocol byte count overflow"))?;
        Ok(())
    }

    fn publish_location_sources(
        &self,
        prepared: &PreparedSimulationCollection,
    ) -> Result<(), Box<dyn Error>> {
        for arena_index in 0..prepared.arena_count() {
            let location = prepared.location_arena(arena_index).ok_or_else(|| {
                probe_error("resident notification collection lost Location arena")
            })?;
            for container_index in 0..location.container_count() {
                let operation = location.operation(container_index).ok_or_else(|| {
                    probe_error("resident notification Location operation disappeared")
                })?;
                let source = location.planned_source(container_index).ok_or_else(|| {
                    probe_error("resident notification Location source disappeared")
                })?;
                let mut sources = self.location_sources.lock().map_err(|_| {
                    probe_error("resident notification Location source store poisoned")
                })?;
                if sources.insert(operation.asset_id, source).is_some() {
                    return Err(probe_error(
                        "duplicate resident notification Location container ID in collection",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Runs one already-resident deferred registration and deregistration
    /// lifecycle, returning the exact loaded-count hold delivered at the
    /// registration callback position.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(crate) fn exercise(
        &mut self,
        collection_ordinal: usize,
        collection: &CollectionRegistrationSnapshot,
        inspection: PreparedSimulationResidentCollection,
        hold: ResidentCollectionHold,
    ) -> Result<ResidentCollectionHold, Box<dyn Error>> {
        let collection_id = collection.id;
        let declared_asset_count = collection.asset_count;
        let payload_size = collection.size;
        if inspection.collection_id().0 != collection_id
            || collection.source_index != collection_ordinal
            || collection.stream_format != 2
        {
            return Err(probe_error(
                "resident notification inspection/catalog identity mismatch",
            ));
        }
        let prepared = inspection.prepared();
        if prepared.arena_count() != usize::try_from(declared_asset_count)? {
            return Err(probe_error(
                "resident notification prepared count differs from collection metadata",
            ));
        }
        self.publish_location_sources(prepared)?;
        let ordinal = u32::try_from(collection_ordinal)?;
        let arena_count = u32::try_from(prepared.arena_count())?;
        let mut arenas = Vec::new();
        arenas.try_reserve_exact(prepared.arena_count())?;
        for arena_index in 0..prepared.arena_count() {
            arenas.push(
                prepared
                    .arena_identity(arena_index)
                    .ok_or_else(|| probe_error("resident notification lost arena identity"))?,
            );
        }
        let route = select_simulation_collection_registration_route(
            SimulationCollectionAdmissionInputs {
                stream_type_raw: 1,
                defer_cvar_enabled: true,
                manager_defer_raw: 1,
                force_immediate: false,
            },
            || payload_size,
        );
        if route != SimulationCollectionRegistrationRoute::DeferredCollectionThenCallback {
            return Err(probe_error(
                "resident notification collection did not select deferred registration",
            ));
        }

        let mut collection_record = vec![b'C'];
        push_u32(&mut collection_record, ordinal);
        push_u64(&mut collection_record, collection_id);
        push_u32(&mut collection_record, payload_size);
        push_u32(&mut collection_record, declared_asset_count);
        push_u32(&mut collection_record, arena_count);
        push_u32(&mut collection_record, collection.stream_format);
        push_u32(&mut collection_record, 1);
        collection_record.push(1);
        push_u32(&mut collection_record, 1);
        collection_record.push(0);
        push_u32(
            &mut collection_record,
            RESIDENT_NOTIFICATION_REG_USERDATA_RAW,
        );
        push_u32(
            &mut collection_record,
            RESIDENT_NOTIFICATION_DREG_USERDATA_RAW,
        );
        self.hash_record(&collection_record)?;
        self.add(|inventory| &mut inventory.collections, 1, "collection")?;
        self.add(|inventory| &mut inventory.arenas, arena_count, "arena")?;

        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| probe_error("resident notification state poisoned"))?;
            state.begin_collection(
                ordinal,
                collection_id,
                &collection.path,
                declared_asset_count,
                payload_size,
                prepared,
            )?;
            state.begin_phase(Phase::Registration, RESIDENT_NOTIFICATION_REG_USERDATA_RAW)?;
        }
        drop(inspection);

        let callback = ResidentRegistrationCallback {
            collection_ordinal: ordinal,
            collection_id,
            hold,
        };
        let mut producer = self.router.begin_resident_deferred_registration(
            skate2_core::collection_resident_streamer::CollectionId(collection_id),
            RESIDENT_NOTIFICATION_REG_USERDATA_RAW,
            Some(callback),
        )?;
        self.add(|inventory| &mut inventory.setup_lookups, 1, "setup lookup")?;
        self.add(|inventory| &mut inventory.preparations, 1, "preparation")?;
        self.add(
            |inventory| &mut inventory.preparation_asset_visits,
            arena_count,
            "preparation asset visit",
        )?;
        let mut setup_record = vec![b'S'];
        push_u32(&mut setup_record, ordinal);
        push_u64(&mut setup_record, collection_id);
        push_u32(&mut setup_record, arena_count);
        self.hash_record(&setup_record)?;
        let mut local_ordinal = 0_u32;
        let finish = loop {
            match producer {
                SimulationNotificationDeferredState::NextArena(arena) => {
                    producer = self.router.enqueue_deferred_arena(arena)?;
                    if !matches!(
                        self.router.dispatch_one()?,
                        SimulationNotificationDispatch::ReceiverReturned
                    ) {
                        return Err(probe_error(
                            "resident deferred Arena marker did not dispatch",
                        ));
                    }
                    let (outer_id, inner_id) = arenas
                        .get(usize::try_from(local_ordinal)?)
                        .copied()
                        .ok_or_else(|| probe_error("resident deferred Arena identity overflow"))?;
                    self.hash_record(&arena_record(
                        b'A',
                        ordinal,
                        local_ordinal,
                        outer_id,
                        inner_id,
                    ))?;
                    self.hash_record(&arena_lookup_record(
                        ordinal,
                        local_ordinal,
                        outer_id,
                        outer_id,
                        inner_id,
                    ))?;
                    self.add(
                        |inventory| &mut inventory.arena_lookup_attempts,
                        1,
                        "Arena lookup attempt",
                    )?;
                    self.add(
                        |inventory| &mut inventory.arena_lookup_hits,
                        1,
                        "Arena lookup hit",
                    )?;
                    self.state
                        .lock()
                        .map_err(|_| probe_error("resident notification state poisoned"))?
                        .deferred_arena_message(usize::try_from(local_ordinal)?)?;
                    self.hash_record(&arena_userdata_record(
                        b'N',
                        ordinal,
                        local_ordinal,
                        outer_id,
                        inner_id,
                        RESIDENT_NOTIFICATION_REG_USERDATA_RAW,
                    ))?;
                    self.hash_record(&arena_record(
                        b'L',
                        ordinal,
                        local_ordinal,
                        outer_id,
                        inner_id,
                    ))?;
                    self.add(|inventory| &mut inventory.arena_nodes, 1, "Arena node")?;
                    self.add(
                        |inventory| &mut inventory.child_fifo_nodes,
                        1,
                        "child FIFO node",
                    )?;
                    self.add(
                        |inventory| &mut inventory.normal_reg_calls,
                        1,
                        "Normal Reg call",
                    )?;
                    self.add(
                        |inventory| &mut inventory.location_reg_calls,
                        1,
                        "Location Reg call",
                    )?;
                    self.add(|inventory| &mut inventory.outer_nodes, 1, "outer node")?;
                    local_ordinal = local_ordinal
                        .checked_add(1)
                        .ok_or("resident deferred local Arena ordinal overflow")?;
                }
                SimulationNotificationDeferredState::ReadyToFinish(finish) => break finish,
            }
        };
        if local_ordinal != arena_count {
            return Err(probe_error(
                "resident deferred producer Arena count differs from prepared metadata",
            ));
        }

        let callback_permit = self.router.enqueue_deferred_finish(finish)?;
        self.router.enqueue_registration_callback(callback_permit)?;
        if !matches!(
            self.router.dispatch_one()?,
            SimulationNotificationDispatch::ReceiverReturned
        ) {
            return Err(probe_error(
                "resident deferred Finish marker did not dispatch",
            ));
        }
        self.add(
            |inventory| &mut inventory.terminal_lookups,
            1,
            "terminal lookup",
        )?;
        self.add(|inventory| &mut inventory.preparations, 1, "preparation")?;
        self.add(
            |inventory| &mut inventory.preparation_asset_visits,
            arena_count,
            "preparation asset visit",
        )?;
        let mut finish_record = vec![b'F'];
        push_u32(&mut finish_record, ordinal);
        push_u64(&mut finish_record, collection_id);
        push_u32(&mut finish_record, arena_count);
        self.hash_record(&finish_record)?;
        for (arena_ordinal, &(outer_id, inner_id)) in arenas.iter().enumerate() {
            self.hash_record(&arena_userdata_record(
                b'M',
                ordinal,
                u32::try_from(arena_ordinal)?,
                outer_id,
                inner_id,
                RESIDENT_NOTIFICATION_REG_USERDATA_RAW,
            ))?;
        }
        self.add(|inventory| &mut inventory.finish_nodes, 1, "Finish node")?;
        self.add(
            |inventory| &mut inventory.child_fifo_nodes,
            1,
            "child FIFO node",
        )?;
        self.add(
            |inventory| &mut inventory.dmo_reg_calls,
            arena_count,
            "DMO Reg call",
        )?;
        self.add(|inventory| &mut inventory.outer_nodes, 1, "outer node")?;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| probe_error("resident notification state poisoned"))?;
            state.deferred_activation_zero()?;
            state.finish_phase();
        }

        let SimulationNotificationDispatch::RegistrationCallbackDequeued(delivery) =
            self.router.dispatch_one()?
        else {
            return Err(probe_error(
                "resident deferred registration callback was not last",
            ));
        };
        let (delivered_id, occurrence, delivered_callback) = delivery.into_parts();
        let ResidentRegistrationCallback {
            collection_ordinal: delivered_ordinal,
            collection_id: callback_collection_id,
            hold,
        } = delivered_callback
            .ok_or_else(|| probe_error("resident deferred registration lost its loaded hold"))?;
        if delivered_id != collection_id
            || delivered_ordinal != ordinal
            || callback_collection_id != collection_id
        {
            return Err(probe_error(
                "resident deferred registration callback identity mismatch",
            ));
        }
        let mut registration_record = vec![b'R'];
        push_u32(&mut registration_record, ordinal);
        push_u64(&mut registration_record, collection_id);
        self.hash_record(&registration_record)?;
        self.add(
            |inventory| &mut inventory.registration_callbacks,
            1,
            "registration callback",
        )?;
        self.add(
            |inventory| &mut inventory.child_fifo_nodes,
            1,
            "child FIFO node",
        )?;
        self.add(|inventory| &mut inventory.outer_nodes, 1, "outer node")?;
        self.state
            .lock()
            .map_err(|_| probe_error("resident notification state poisoned"))?
            .deferred_callback();

        self.state
            .lock()
            .map_err(|_| probe_error("resident notification state poisoned"))?
            .begin_phase(
                Phase::Deregistration,
                RESIDENT_NOTIFICATION_DREG_USERDATA_RAW,
            )?;
        let dreg_route = select_simulation_collection_deregistration_route(1);
        if dreg_route != SimulationCollectionDeregistrationRoute::DeregistrationThenCallback {
            return Err(probe_error(
                "raw resident Simulation type skipped DRegCollection",
            ));
        }
        let dreg_callback = (ordinal, collection_id);
        let callback_permit = self.router.enqueue_deregistration(
            occurrence,
            RESIDENT_NOTIFICATION_DREG_USERDATA_RAW,
            dreg_route,
            Some(dreg_callback),
        )?;
        self.router
            .enqueue_deregistration_callback(callback_permit)?;
        if !matches!(
            self.router.dispatch_one()?,
            SimulationNotificationDispatch::ReceiverReturned
        ) {
            return Err(probe_error(
                "resident deferred DRegCollection node was not first",
            ));
        }
        self.add(|inventory| &mut inventory.dreg_lookups, 1, "DReg lookup")?;
        self.add(|inventory| &mut inventory.preparations, 1, "preparation")?;
        self.add(
            |inventory| &mut inventory.preparation_asset_visits,
            arena_count,
            "preparation asset visit",
        )?;
        let mut dreg_record = vec![b'D'];
        push_u32(&mut dreg_record, ordinal);
        push_u64(&mut dreg_record, collection_id);
        push_u32(&mut dreg_record, arena_count);
        push_u32(&mut dreg_record, RESIDENT_NOTIFICATION_DREG_USERDATA_RAW);
        self.hash_record(&dreg_record)?;
        for (arena_ordinal, &(outer_id, inner_id)) in arenas.iter().enumerate() {
            self.hash_record(&arena_userdata_record(
                b'X',
                ordinal,
                u32::try_from(arena_ordinal)?,
                outer_id,
                inner_id,
                RESIDENT_NOTIFICATION_DREG_USERDATA_RAW,
            ))?;
        }
        for (arena_ordinal, &(outer_id, inner_id)) in arenas.iter().enumerate() {
            let arena_ordinal = u32::try_from(arena_ordinal)?;
            self.hash_record(&arena_userdata_record(
                b'n',
                ordinal,
                arena_ordinal,
                outer_id,
                inner_id,
                RESIDENT_NOTIFICATION_DREG_USERDATA_RAW,
            ))?;
            self.hash_record(&arena_record(
                b'l',
                ordinal,
                arena_ordinal,
                outer_id,
                inner_id,
            ))?;
        }
        self.add(|inventory| &mut inventory.dreg_nodes, 1, "DReg node")?;
        self.add(
            |inventory| &mut inventory.dmo_dreg_calls,
            arena_count,
            "DMO DReg call",
        )?;
        self.add(
            |inventory| &mut inventory.normal_dreg_calls,
            arena_count,
            "Normal DReg call",
        )?;
        self.add(
            |inventory| &mut inventory.location_dreg_calls,
            arena_count,
            "Location DReg call",
        )?;
        self.add(|inventory| &mut inventory.outer_nodes, 1, "outer node")?;
        self.state
            .lock()
            .map_err(|_| probe_error("resident notification state poisoned"))?
            .finish_phase();

        let SimulationNotificationDispatch::DeregistrationCallbackDequeued(delivery) =
            self.router.dispatch_one()?
        else {
            return Err(probe_error(
                "resident deferred deregistration callback was not second",
            ));
        };
        let (delivered_id, delivered_callback) = delivery.into_parts();
        if delivered_id != collection_id || delivered_callback != Some(dreg_callback) {
            return Err(probe_error(
                "resident deferred deregistration callback identity mismatch",
            ));
        }
        let mut callback_record = vec![b'U'];
        push_u32(&mut callback_record, ordinal);
        push_u64(&mut callback_record, collection_id);
        self.hash_record(&callback_record)?;
        self.add(
            |inventory| &mut inventory.deregistration_callbacks,
            1,
            "deregistration callback",
        )?;
        self.add(|inventory| &mut inventory.outer_nodes, 1, "outer node")?;
        self.location_sources
            .lock()
            .map_err(|_| probe_error("resident notification Location source store poisoned"))?
            .clear();
        Ok(hold)
    }

    #[allow(clippy::too_many_lines)] // Keep the complete final ownership/count audit adjacent.
    pub(crate) fn finish(mut self) -> Result<ResidentNotificationInventory, Box<dyn Error>> {
        if self.router.pending_message_count() != 0
            || self.router.retained_session_count() != 0
            || self.router.live_collection_registration_count() != 0
            || self.router.deferred_collection_registration_count() != 0
            || self.router.completed_collection_registration_count() != 0
            || self.router.fault().is_some()
            || self.router.collection_fault().is_some()
            || !self
                .location_sources
                .lock()
                .map_err(|_| probe_error("resident notification Location source store poisoned"))?
                .is_empty()
            || !matches!(
                self.router.dispatch_one()?,
                SimulationNotificationDispatch::Idle
            )
        {
            return Err(probe_error(
                "resident notification router did not finish quiescent",
            ));
        }
        let resolution = self.router.resident_resolution_stats();
        let immediate = resolution.immediate();
        let setup = resolution.deferred_begin();
        let arena = resolution.deferred_arena();
        let terminal = resolution.deferred_finish();
        let dreg = resolution.deregistration();
        let successful_preparations = setup
            .successful_preparations()
            .checked_add(terminal.successful_preparations())
            .and_then(|count| count.checked_add(dreg.successful_preparations()))
            .ok_or_else(|| probe_error("resident resolution preparation count overflow"))?;
        if immediate.lookup_attempts() != 0
            || immediate.successful_preparations() != 0
            || setup.lookup_attempts() != u64::from(self.inventory.setup_lookups)
            || arena.lookup_attempts() != u64::from(self.inventory.arena_lookup_attempts)
            || arena.successful_resolutions() != u64::from(self.inventory.arena_lookup_hits)
            || terminal.lookup_attempts() != u64::from(self.inventory.terminal_lookups)
            || dreg.lookup_attempts() != u64::from(self.inventory.dreg_lookups)
            || successful_preparations != u64::from(self.inventory.preparations)
            || setup.lookup_attempts() != setup.successful_preparations()
            || self.inventory.arena_lookup_misses != 0
            || self.inventory.arena_identity_mismatches != 0
            || terminal.lookup_attempts() != terminal.successful_preparations()
            || dreg.lookup_attempts() != dreg.successful_preparations()
        {
            return Err(probe_error(format!(
                "resident resolution telemetry differs from the clean protocol: {resolution:?}"
            )));
        }
        let state = self
            .state
            .lock()
            .map_err(|_| probe_error("resident notification state poisoned"))?;
        if state.phase.is_some()
            || state.cursor != 0
            || !state.expected.is_empty()
            || state.immediate_eligible != 0
            || state.deferred_eligible != self.inventory.collections
            || state.collections != self.inventory.collections
            || state.arena_count != self.inventory.arenas
            || state.sequence_mismatches != 0
            || state.deferred_arena_messages != self.inventory.arena_nodes
            || state.deferred_zero_messages != self.inventory.finish_nodes
            || state.deferred_callbacks != self.inventory.registration_callbacks
            || state.deferred_normal_calls != self.inventory.arenas
            || state.deferred_location_calls != self.inventory.arenas
            || state.deferred_dmo_calls != self.inventory.arenas
            || state.normal_calls != self.inventory.arenas * 2
            || state.location_calls != self.inventory.arenas * 2
            || state.dmo_calls != self.inventory.arenas * 2
        {
            return Err(probe_error(
                "resident notification child receiver sequence did not finish exactly",
            ));
        }
        if self.inventory.normal_reg_calls != state.deferred_normal_calls
            || self.inventory.location_reg_calls != state.deferred_location_calls
            || self.inventory.dmo_reg_calls != state.deferred_dmo_calls
            || self.inventory.normal_dreg_calls != state.normal_calls - state.deferred_normal_calls
            || self.inventory.location_dreg_calls
                != state.location_calls - state.deferred_location_calls
            || self.inventory.dmo_dreg_calls != state.dmo_calls - state.deferred_dmo_calls
        {
            return Err(probe_error(
                "resident notification pass counters differ from receiver observations",
            ));
        }
        drop(state);

        let mut trailer = vec![b'Q'];
        for value in [
            self.inventory.collections,
            self.inventory.arenas,
            self.inventory.setup_lookups,
            self.inventory.terminal_lookups,
            self.inventory.dreg_lookups,
            self.inventory.preparations,
            self.inventory.preparation_asset_visits,
            self.inventory.arena_nodes,
            self.inventory.finish_nodes,
            self.inventory.registration_callbacks,
            self.inventory.dreg_nodes,
            self.inventory.deregistration_callbacks,
            self.inventory.outer_nodes,
            self.inventory.child_fifo_nodes,
            self.inventory.normal_reg_calls,
            self.inventory.location_reg_calls,
            self.inventory.dmo_reg_calls,
            self.inventory.dmo_dreg_calls,
            self.inventory.normal_dreg_calls,
            self.inventory.location_dreg_calls,
            self.inventory.arena_lookup_attempts,
            self.inventory.arena_lookup_hits,
            self.inventory.arena_lookup_misses,
            self.inventory.arena_identity_mismatches,
        ] {
            push_u32(&mut trailer, value);
        }
        self.hash_record(&trailer)?;
        if self.inventory != EXPECTED_RESIDENT_NOTIFICATION {
            return Err(probe_error(format!(
                "resident notification corpus mismatch: expected {EXPECTED_RESIDENT_NOTIFICATION:?}, got {:?}",
                self.inventory
            )));
        }
        Ok(self.inventory)
    }
}
fn arena_userdata_record(
    tag: u8,
    collection: u32,
    arena: u32,
    outer_id: u64,
    inner_id: u32,
    userdata_raw: u32,
) -> Vec<u8> {
    let mut record = arena_record(tag, collection, arena, outer_id, inner_id);
    push_u32(&mut record, userdata_raw);
    record
}

fn arena_record(tag: u8, collection: u32, arena: u32, outer_id: u64, inner_id: u32) -> Vec<u8> {
    let mut record = vec![tag];
    push_u32(&mut record, collection);
    push_u32(&mut record, arena);
    push_u64(&mut record, outer_id);
    push_u32(&mut record, inner_id);
    record
}

fn arena_lookup_record(
    collection: u32,
    arena: u32,
    requested_outer_id: u64,
    resolved_outer_id: u64,
    resolved_inner_id: u32,
) -> Vec<u8> {
    let mut record = vec![b'K'];
    push_u32(&mut record, collection);
    push_u32(&mut record, arena);
    push_u64(&mut record, requested_outer_id);
    push_u64(&mut record, resolved_outer_id);
    push_u32(&mut record, resolved_inner_id);
    record
}

fn domain(bytes: &[u8]) -> u64 {
    let mut signature = FNV_OFFSET;
    hash(&mut signature, bytes);
    signature
}

fn hash(signature: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        *signature ^= u64::from(byte);
        *signature = signature.wrapping_mul(FNV_PRIME);
    }
}

fn feed(signature: &mut u64, stream: &mut Vec<u8>, bytes: &[u8]) {
    hash(signature, bytes);
    stream.extend_from_slice(bytes);
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn probe_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}
