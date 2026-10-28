//! Opt-in real-BAM resident/deferred Simulation notification gate.
//!
//! The existing whole-BAM V1 inventory remains the independent compatibility
//! gate. This module separately composes the mounted collection loader, durable
//! Simulation resident assets, safe-host weak resident registry, loaded-count hold, and
//! resident-backed notification router for the 79 deferred-eligible BAM
//! collections. One collection is loaded, registered, deregistered, and
//! destroyed at a time. That cadence and all telemetry are host diagnostics,
//! not claims about the native pending-vector scheduler.

use crate::simulation_collection_runtime_probe::ResidentNotificationRuntime;
use skate2_core::{
    catalog_resident_load_factory::{
        CatalogResidentCollectionLoadFactory, CatalogResidentLoadError,
        CollectionRequestIdAllocator, CollectionRequestIdLease,
    },
    collection_resident_streamer::{
        CollectionId, CollectionResidentStreamer, ResidentAssetGarbageCollector,
        ResidentCallbackContext, ResidentCallbackObservation, ResidentCollectionCallback,
        ResidentCollectionHold, ResidentCollectionMetadata, ResidentStreamerUpdate,
    },
    game_stream::{GameStreamState, ManifestOnlyStream, StreamKind},
    mounted_stream_backend::MountedStreamBackend,
    owned_resident_collection::{
        OwnedResidentCollectionError, OwnedResidentCollectionStorage, ResidentAssetIdentity,
    },
    simulation_collection_admission::{
        SimulationCollectionAdmissionInputs, SimulationCollectionRegistrationRoute,
        select_simulation_collection_registration_route,
    },
    simulation_collision_asset::SimulationCollisionAssetSessionError,
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_resident_collection::{
        SimulationResidentArenaAsset, SimulationResidentArenaAssetBuilderSessions,
        SimulationResidentArenaBuildError,
    },
    simulation_resident_registry::{SimulationResidentRegistry, SimulationResidentRegistryError},
    world_stream_catalog::{CatalogCounts, CollectionRegistrationSnapshot, WorldStreamCatalog},
};
use skate2_formats::mount::{MountError, MountSet};
use std::{
    convert::Infallible,
    error::Error,
    fmt,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(120);
const DEFER_THRESHOLD: u32 = 0x0007_d000;
const EXPECTED_COLLECTIONS: usize = 79;
const EXPECTED_ARENAS: usize = 1_392;

#[derive(Clone, Copy, Debug)]
struct RequestIdError;

impl fmt::Display for RequestIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("resident-router diagnostic request identity is already live")
    }
}

impl Error for RequestIdError {}

struct RequestIdLease {
    live: Arc<AtomicBool>,
}

impl CollectionRequestIdLease for RequestIdLease {
    fn raw(&self) -> i32 {
        0
    }
}

impl Drop for RequestIdLease {
    fn drop(&mut self) {
        self.live.store(false, Ordering::Release);
    }
}

struct RequestIds {
    live: Arc<AtomicBool>,
}

impl CollectionRequestIdAllocator for RequestIds {
    type Lease = RequestIdLease;
    type Error = RequestIdError;

    fn acquire(&mut self) -> Result<Self::Lease, Self::Error> {
        self.live
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| RequestIdError)?;
        Ok(RequestIdLease {
            live: Arc::clone(&self.live),
        })
    }
}

type ResidentCause = CatalogResidentLoadError<
    MountError,
    RequestIdError,
    SimulationCollisionAssetSessionError,
    SimulationResidentArenaBuildError,
>;
type ResidentError = Arc<ResidentCause>;
type ResidentStorage = OwnedResidentCollectionStorage<
    SimulationResidentArenaAsset,
    fn(OwnedResidentCollectionError) -> ResidentError,
    ResidentError,
>;
type ResidentRegistry = SimulationResidentRegistry<
    SimulationResidentArenaAsset,
    fn(SimulationResidentRegistryError) -> ResidentError,
    ResidentError,
>;

fn external(error: impl Error + Send + Sync + 'static) -> ResidentError {
    Arc::new(CatalogResidentLoadError::external(Arc::new(error)))
}

fn map_storage_error(error: OwnedResidentCollectionError) -> ResidentError {
    external(error)
}

fn map_registry_error(error: SimulationResidentRegistryError) -> ResidentError {
    external(error)
}

#[derive(Clone)]
struct Metadata {
    catalog: WorldStreamCatalog,
}

impl ResidentCollectionMetadata<ResidentError> for Metadata {
    type Descriptor = CollectionRegistrationSnapshot;

    fn find_collection(
        &mut self,
        id: CollectionId,
    ) -> Result<Option<Self::Descriptor>, ResidentError> {
        self.catalog.collection(id.0).map_err(external)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LoadCallbackObservation {
    id: CollectionId,
    before: ResidentCallbackObservation,
    after: skate2_core::collection_resident_streamer::ResidentCollectionObservation,
}

struct LoadCallback {
    expected_id: CollectionId,
    hold: Arc<Mutex<Option<ResidentCollectionHold>>>,
    observations: Arc<Mutex<Vec<LoadCallbackObservation>>>,
}

impl ResidentCollectionCallback<ResidentError> for LoadCallback {
    fn on_complete(
        &mut self,
        id: CollectionId,
        observation: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, Self>,
    ) -> Result<(), ResidentError> {
        if id != self.expected_id || context.active_collection_id() != self.expected_id {
            return Err(external(ProbeFailure(format!(
                "load callback identity differed: expected {:#018x}, got {:#018x}",
                self.expected_id.0, id.0
            ))));
        }
        let expected_before = ResidentCallbackObservation {
            active_id: self.expected_id,
            resident_ref_count: Some(1),
            wrapper_is_published: true,
        };
        if observation != expected_before {
            return Err(external(ProbeFailure(format!(
                "load callback pre-hold observation differed for {:#018x}: {observation:?}",
                id.0
            ))));
        }
        let (after, hold) = context.acquire_active_collection_hold().map_err(external)?;
        if after
            != (skate2_core::collection_resident_streamer::ResidentCollectionObservation {
                ref_count: 2,
                wrapper_is_published: true,
            })
        {
            return Err(external(ProbeFailure(format!(
                "load callback post-hold observation differed for {:#018x}: {after:?}",
                id.0
            ))));
        }
        let mut slot = self
            .hold
            .lock()
            .map_err(|_| external(ProbeFailure("load hold slot poisoned".to_owned())))?;
        if slot.replace(hold).is_some() {
            return Err(external(ProbeFailure(
                "load callback produced a second hold".to_owned(),
            )));
        }
        self.observations
            .lock()
            .map_err(|_| external(ProbeFailure("load observations poisoned".to_owned())))?
            .push(LoadCallbackObservation {
                id,
                before: observation,
                after,
            });
        Ok(())
    }
}

#[derive(Clone)]
struct GarbageCollector {
    ids: Arc<Mutex<Vec<CollectionId>>>,
}

impl ResidentAssetGarbageCollector<ResidentError> for GarbageCollector {
    fn garbage_collect(&mut self, id: CollectionId) -> Result<(), ResidentError> {
        self.ids
            .lock()
            .map_err(|_| external(ProbeFailure("garbage telemetry poisoned".to_owned())))?
            .push(id);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DriverTotals {
    cache_hits: u32,
    unloads_processed: u32,
    physical_loads_started: u32,
    physical_loads_completed: u32,
    decompression_kicks: u32,
}

impl DriverTotals {
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

#[derive(Debug)]
struct ProbeFailure(String);

impl fmt::Display for ProbeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ProbeFailure {}

fn probe_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(ProbeFailure(message.into()))
}

fn require(condition: bool, message: impl Into<String>) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(probe_error(message))
    }
}

fn drive_to_idle<S, M, F, R, G, K, C>(
    streamer: &mut CollectionResidentStreamer<S, M, F, R, G, K, C, ResidentError>,
    totals: &mut DriverTotals,
) -> Result<(), Box<dyn Error>>
where
    S: skate2_core::collection_resident_streamer::ResidentCollectionStorage<ResidentError>,
    M: ResidentCollectionMetadata<ResidentError>,
    F: skate2_core::collection_resident_streamer::ResidentCollectionLoadFactory<
            S::Writer,
            M::Descriptor,
            ResidentError,
        >,
    R: skate2_core::collection_resident_streamer::ResidentCollectionRegistry<
            S::Collection,
            ResidentError,
        >,
    G: ResidentAssetGarbageCollector<ResidentError>,
    K: skate2_core::collection_resident_streamer::ResidentDecompressionKick<ResidentError>,
    C: ResidentCollectionCallback<ResidentError>,
{
    let deadline = Instant::now() + PROBE_TIMEOUT;
    while streamer.pending_requests() {
        if Instant::now() >= deadline {
            return Err(probe_error(format!(
                "resident-router probe timed out with active {:?} and {} queued",
                streamer.active_id(),
                streamer.queued_request_count()
            )));
        }
        totals.add(streamer.update()?);
        thread::yield_now();
    }
    Ok(())
}

/// Runs the isolated real-data resident/router lifecycle after the unchanged
/// V1 Simulation inventory has passed.
#[allow(clippy::too_many_lines)]
pub(super) fn run(mounts: &Arc<MountSet>) -> Result<(), Box<dyn Error>> {
    let catalog = WorldStreamCatalog::new();
    let mut backend = MountedStreamBackend::new(Arc::clone(mounts), catalog.clone())?;
    let mut manifest = ManifestOnlyStream::load(
        "data\\content\\world\\stream",
        "BAM",
        StreamKind::Simulation,
    )?;
    let deadline = Instant::now() + PROBE_TIMEOUT;
    while manifest.state() != GameStreamState::LOADED {
        if Instant::now() >= deadline {
            return Err(probe_error(format!(
                "BAM Simulation manifest timed out in state {}",
                manifest.state().raw()
            )));
        }
        manifest.poll(&mut backend)?;
        if let Some(stage) = manifest.reference_read_failure() {
            return Err(probe_error(format!(
                "BAM Simulation manifest read stalled at {stage:?}"
            )));
        }
        thread::yield_now();
    }
    require_manifest(&catalog, &manifest)?;

    let selected = selected_deferred_collections(&catalog, &manifest)?;
    let request_live = Arc::new(AtomicBool::new(false));
    let observations = Arc::new(Mutex::new(Vec::new()));
    let garbage_ids = Arc::new(Mutex::new(Vec::new()));
    let sessions = SimulationResidentArenaAssetBuilderSessions::new(catalog.clone());
    let factory = CatalogResidentCollectionLoadFactory::new(
        Arc::clone(mounts),
        catalog.clone(),
        RequestIds {
            live: Arc::clone(&request_live),
        },
        sessions,
    );
    let wake = factory.decompression_kick();
    let storage = ResidentStorage::new(map_storage_error);
    let (registry, lookup) = ResidentRegistry::new(map_registry_error);
    let observer_lookup = lookup.clone();
    let observer_arena_lookup = lookup.arena_lookup();
    let mut streamer = CollectionResidentStreamer::new(
        storage,
        Metadata {
            catalog: catalog.clone(),
        },
        factory,
        registry,
        GarbageCollector {
            ids: Arc::clone(&garbage_ids),
        },
        wake,
    );
    let mut runtime = ResidentNotificationRuntime::try_new(lookup)?;
    let mut totals = DriverTotals::default();
    let mut arena_total = 0_usize;

    for (original_ordinal, collection) in selected {
        let id = CollectionId(collection.id);
        let hold_slot = Arc::new(Mutex::new(None));
        let garbage_start = garbage_ids
            .lock()
            .map_err(|_| probe_error("garbage telemetry poisoned"))?
            .len();
        streamer.enqueue_load(
            id,
            i32::from_ne_bytes(collection.asset_count.to_ne_bytes()),
            Some(LoadCallback {
                expected_id: id,
                hold: Arc::clone(&hold_slot),
                observations: Arc::clone(&observations),
            }),
        );
        drive_to_idle(&mut streamer, &mut totals)?;
        require(
            !request_live.load(Ordering::Acquire),
            format!("request lease remained live after loading {:#018x}", id.0),
        )?;
        require(
            streamer.observe_resident(id)
                == Some(
                    skate2_core::collection_resident_streamer::ResidentCollectionObservation {
                        ref_count: 2,
                        wrapper_is_published: true,
                    },
                ),
            format!("loaded resident observation differed for {:#018x}", id.0),
        )?;
        let hold = hold_slot
            .lock()
            .map_err(|_| probe_error("load hold slot poisoned"))?
            .take()
            .ok_or_else(|| probe_error("load callback did not return its loaded-count hold"))?;

        let occurrence = observer_lookup
            .lookup(id)?
            .ok_or_else(|| probe_error(format!("published resident {:#018x} was absent", id.0)))?;
        let mut resident_asset_ids = Vec::new();
        let mut plan_weaks: Vec<Weak<SimulationNormalArenaPlan>> = Vec::new();
        occurrence.try_for_each::<Infallible>(|asset| {
            resident_asset_ids.push(asset.resident_asset_id());
            plan_weaks.push(Arc::downgrade(&asset.plan_owner()));
            Ok(())
        })?;
        let inspection = occurrence.prepare_collection()?;
        require(
            resident_asset_ids.len() == usize::try_from(collection.asset_count)?
                && inspection.prepared().arena_count() == resident_asset_ids.len(),
            format!("resident arena count differed for {:#018x}", id.0),
        )?;
        arena_total = arena_total
            .checked_add(resident_asset_ids.len())
            .ok_or_else(|| probe_error("resident arena total overflow"))?;

        let hold = runtime.exercise(original_ordinal, &collection, inspection, hold)?;
        require(
            !streamer.release_collection_hold(hold)?,
            format!("router hold unexpectedly destroyed {:#018x}", id.0),
        )?;
        require(
            streamer.observe_resident(id)
                == Some(
                    skate2_core::collection_resident_streamer::ResidentCollectionObservation {
                        ref_count: 1,
                        wrapper_is_published: true,
                    },
                ),
            format!(
                "post-router resident observation differed for {:#018x}",
                id.0
            ),
        )?;

        streamer.enqueue_unload(id, None);
        drive_to_idle(&mut streamer, &mut totals)?;
        require(
            streamer.resident_count() == 0
                && streamer.terminal_error().is_none()
                && observer_lookup.lookup(id)?.is_none(),
            format!("resident teardown did not quiesce {:#018x}", id.0),
        )?;
        for arena_id in &resident_asset_ids {
            require(
                observer_arena_lookup
                    .try_with_asset::<_, Infallible>(*arena_id, |_| Ok(()))?
                    .is_none(),
                format!(
                    "Arena registry retained {:#018x} after collection {:#018x} teardown",
                    arena_id.0, id.0
                ),
            )?;
        }
        let garbage = garbage_ids
            .lock()
            .map_err(|_| probe_error("garbage telemetry poisoned"))?;
        require(
            garbage.get(garbage_start..) == Some(resident_asset_ids.as_slice()),
            format!("garbage order differed for {:#018x}", id.0),
        )?;
        drop(garbage);
        require(
            plan_weaks.iter().all(|plan| plan.upgrade().is_none()),
            format!("plan owner survived resident teardown for {:#018x}", id.0),
        )?;
    }

    require(
        arena_total == EXPECTED_ARENAS,
        format!("resident arena total was {arena_total}, expected {EXPECTED_ARENAS}"),
    )?;
    let expected_collections_u32 = u32::try_from(EXPECTED_COLLECTIONS)?;
    require(
        totals.cache_hits == 0
            && totals.physical_loads_started == expected_collections_u32
            && totals.physical_loads_completed == expected_collections_u32
            && totals.unloads_processed == expected_collections_u32,
        format!("resident driver totals differed: {totals:?}"),
    )?;
    require(
        observations
            .lock()
            .map_err(|_| probe_error("load observations poisoned"))?
            .len()
            == EXPECTED_COLLECTIONS,
        "resident load callback count differed",
    )?;
    require(
        garbage_ids
            .lock()
            .map_err(|_| probe_error("garbage telemetry poisoned"))?
            .len()
            == EXPECTED_ARENAS,
        "resident garbage callback count differed",
    )?;
    let inventory = runtime.finish()?;
    require(
        streamer.resident_count() == 0
            && !streamer.pending_requests()
            && streamer.terminal_error().is_none()
            && !request_live.load(Ordering::Acquire),
        "resident streamer did not finish quiescent",
    )?;
    drop(streamer);
    require(
        matches!(
            observer_lookup.lookup(CollectionId(0)),
            Err(skate2_core::simulation_resident_registry::SimulationResidentLookupError::RegistryDetached)
        ),
        "registry lookup did not detach after empty streamer drop",
    )?;
    require(
        matches!(
            observer_arena_lookup.try_with_asset::<_, Infallible>(CollectionId(0), |_| Ok(())),
            Err(skate2_core::simulation_resident_registry::SimulationResidentArenaLookupError::RegistryDetached)
        ),
        "Arena registry lookup did not detach after empty streamer drop",
    )?;

    manifest
        .request_unload()
        .map_err(|error| probe_error(format!("manifest unload request failed: {error:?}")))?;
    let deadline = Instant::now() + PROBE_TIMEOUT;
    while !manifest.is_released() {
        if Instant::now() >= deadline {
            return Err(probe_error(format!(
                "BAM Simulation manifest unload timed out in state {}",
                manifest.state().raw()
            )));
        }
        manifest.poll(&mut backend)?;
        thread::yield_now();
    }
    require(
        catalog.counts()?
            == CatalogCounts {
                files: 0,
                collections: 0,
                asset_entries: 0,
                asset_references: 0,
                collection_spaces: 0,
                stream_xml_files: 0,
            },
        format!(
            "isolated Simulation manifest did not unload: {:?}",
            catalog.counts()?
        ),
    )?;
    println!("Simulation resident deferred router corpus: {inventory:?}");
    println!(
        "resident lifecycle: 79 physical loads / 79 exact unloads, 1392 dense arenas / ordered garbage callbacks, {decompression_kicks} decompression kicks",
        decompression_kicks = totals.decompression_kicks
    );
    println!(
        "PASS: mounted production-code resident construction, receiver-time per-Arena global lookup, and Setup/terminal/DReg collection re-resolution"
    );
    Ok(())
}

fn require_manifest<H, E>(
    catalog: &WorldStreamCatalog,
    manifest: &ManifestOnlyStream<H, E>,
) -> Result<(), Box<dyn Error>> {
    let counts = catalog.counts()?;
    require(
        counts.files == 1
            && counts.collections == 284
            && counts.asset_entries == 2_135
            && counts.asset_references == 2_135
            && counts.collection_spaces == 0
            && counts.stream_xml_files == 0,
        format!("isolated Simulation manifest counts differed: {counts:?}"),
    )?;
    let map = manifest
        .collection_map()
        .ok_or_else(|| probe_error("loaded Simulation manifest lacks XSM"))?;
    let toc = manifest
        .asset_toc()
        .ok_or_else(|| probe_error("loaded Simulation manifest lacks XST"))?;
    require(
        map.version == 3
            && map.stream_format == 2
            && map.collections.len() == 284
            && toc.version == 3
            && toc.assets.len() == 2_135,
        "isolated Simulation manifest identity differed",
    )
}

fn selected_deferred_collections<H, E>(
    catalog: &WorldStreamCatalog,
    manifest: &ManifestOnlyStream<H, E>,
) -> Result<Vec<(usize, CollectionRegistrationSnapshot)>, Box<dyn Error>> {
    let map = manifest
        .collection_map()
        .ok_or_else(|| probe_error("loaded Simulation manifest lacks XSM"))?;
    let mut selected = Vec::new();
    selected.try_reserve_exact(EXPECTED_COLLECTIONS)?;
    let mut arena_count = 0_usize;
    for (ordinal, entry) in map.collections.iter().enumerate() {
        let collection = catalog.collection(entry.id)?.ok_or_else(|| {
            probe_error(format!(
                "catalog lacks Simulation collection {:#018x}",
                entry.id
            ))
        })?;
        require(
            collection.source_index == ordinal
                && collection.size == entry.payload_size
                && collection.asset_count == entry.asset_count
                && collection.stream_format == 2
                && catalog
                    .file(&collection.owner)?
                    .is_some_and(|file| file.kind == StreamKind::Simulation),
            format!("collection snapshot differed at authored ordinal {ordinal}"),
        )?;
        let route = select_simulation_collection_registration_route(
            SimulationCollectionAdmissionInputs {
                stream_type_raw: 1,
                defer_cvar_enabled: true,
                manager_defer_raw: 1,
                force_immediate: false,
            },
            || collection.size,
        );
        if route == SimulationCollectionRegistrationRoute::DeferredCollectionThenCallback {
            require(
                collection.size > DEFER_THRESHOLD,
                "deferred selector admitted a non-large collection",
            )?;
            arena_count = arena_count
                .checked_add(usize::try_from(collection.asset_count)?)
                .ok_or_else(|| probe_error("selected arena count overflow"))?;
            selected.push((ordinal, collection));
        }
    }
    require(
        selected.len() == EXPECTED_COLLECTIONS && arena_count == EXPECTED_ARENAS,
        format!(
            "deferred selection was {} collections / {arena_count} arenas",
            selected.len()
        ),
    )?;
    Ok(selected)
}
