//! The main DMO bank for the selected session: `cDMOBankManager::
//! LoadMainDMOBank` 0x82e0e628 (gated by `kEnableMainDMOBank`) loads hull
//! `DMO` with register-on-load flag 8; the hull callback forwards the
//! completed `cPres_Global` collection to `Simulation::RegArena` with
//! userdata `0x0a` (evidence dmo-bank-processed-asset.md). This owner reads
//! the `data\content\world\dmo\DMO` manifest into its own catalog, loads the
//! collection resident through the processed-bank builder and keeps it held.
//! Hull scheduling and unload are not reproduced.
use crate::{
    catalog_resident_load_factory::{
        CatalogResidentCollectionLoadFactory, CatalogResidentLoadError,
        CollectionRequestIdAllocator, CollectionRequestIdLease,
    },
    collection_resident_streamer::{
        CollectionId, CollectionResidentStreamer, ResidentAssetGarbageCollector,
        ResidentCallbackContext, ResidentCallbackObservation, ResidentCollectionCallback,
        ResidentCollectionHold, ResidentCollectionMetadata,
    },
    dmo_bank_processed_asset::{
        DMO_BANK_COLLECTION_ID, DmoBankProcessedArenaAsset, DmoBankProcessedArenaAssetBuilderSessions,
        DmoBankProcessedArenaBuildError, DmoBankProcessedArenaSessionError,
    },
    game_stream::{GameStreamState, InnerStreamBackend, ManifestOnlyStream, StreamKind},
    mounted_stream_backend::MountedStreamBackend,
    owned_resident_collection::{OwnedResidentCollectionError, OwnedResidentCollectionStorage},
    resident_collection_load_driver::CollectionDecompressionWake,
    simulation_resident_registry::{
        SimulationMainDmoBankResidentSnapshot, SimulationResidentLookup,
        SimulationResidentRegistry, SimulationResidentRegistryError,
    },
    world_stream_catalog::{CollectionRegistrationSnapshot, WorldStreamCatalog},
};
use skate2_formats::mount::{MountError, MountSet};
use std::{
    error::Error,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

const DMO_BANK_BASE_PATH: &str = "data\\content\\world\\dmo";
const DMO_BANK_BASENAME: &str = "DMO";

struct RequestIds(Arc<AtomicBool>);
struct RequestLease(Arc<AtomicBool>);
#[derive(Debug)]
struct RequestBusy;
impl fmt::Display for RequestBusy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DMO bank request identity is still leased")
    }
}
impl Error for RequestBusy {}
impl CollectionRequestIdLease for RequestLease {
    fn raw(&self) -> i32 {
        0
    }
}
impl Drop for RequestLease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl CollectionRequestIdAllocator for RequestIds {
    type Lease = RequestLease;
    type Error = RequestBusy;
    fn acquire(&mut self) -> Result<RequestLease, RequestBusy> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| RequestBusy)?;
        Ok(RequestLease(Arc::clone(&self.0)))
    }
}

type Cause = CatalogResidentLoadError<
    MountError,
    RequestBusy,
    DmoBankProcessedArenaSessionError,
    DmoBankProcessedArenaBuildError,
>;
type LoadError = Arc<Cause>;
type Storage = OwnedResidentCollectionStorage<
    DmoBankProcessedArenaAsset,
    fn(OwnedResidentCollectionError) -> LoadError,
    LoadError,
>;
type Registry = SimulationResidentRegistry<
    DmoBankProcessedArenaAsset,
    fn(SimulationResidentRegistryError) -> LoadError,
    LoadError,
>;
type Factory = CatalogResidentCollectionLoadFactory<
    MountSet,
    RequestIds,
    DmoBankProcessedArenaAssetBuilderSessions,
>;
type Streamer = CollectionResidentStreamer<
    Storage,
    Metadata,
    Factory,
    Registry,
    RetainedOnly,
    CollectionDecompressionWake,
    KeepLoaded,
    LoadError,
>;
type Backend = MountedStreamBackend<WorldStreamCatalog>;
type Manifest = ManifestOnlyStream<
    <Backend as InnerStreamBackend>::Handle,
    <Backend as InnerStreamBackend>::Error,
>;

#[derive(Debug)]
struct HostFailure(&'static str);
impl fmt::Display for HostFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl Error for HostFailure {}

fn external(message: &'static str) -> LoadError {
    Arc::new(Cause::external(Arc::new(HostFailure(message))))
}
fn storage_error(error: OwnedResidentCollectionError) -> LoadError {
    Arc::new(Cause::external(Arc::new(error)))
}
fn registry_error(error: SimulationResidentRegistryError) -> LoadError {
    Arc::new(Cause::external(Arc::new(error)))
}

struct Metadata(WorldStreamCatalog);
impl ResidentCollectionMetadata<LoadError> for Metadata {
    type Descriptor = CollectionRegistrationSnapshot;
    fn find_collection(&mut self, id: CollectionId) -> Result<Option<Self::Descriptor>, LoadError> {
        self.0
            .collection(id.0)
            .map_err(|error| Arc::new(Cause::external(Arc::new(error))))
    }
}

/// The bank stays resident for the session; a reached zero-count GC is refused.
struct RetainedOnly;
impl ResidentAssetGarbageCollector<LoadError> for RetainedOnly {
    fn garbage_collect(&mut self, _: CollectionId) -> Result<(), LoadError> {
        Err(external("DMO bank garbage collection is not connected"))
    }
}

struct KeepLoaded {
    hold: Arc<Mutex<Option<ResidentCollectionHold>>>,
}
impl ResidentCollectionCallback<LoadError> for KeepLoaded {
    fn on_complete(
        &mut self,
        id: CollectionId,
        _: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, Self>,
    ) -> Result<(), LoadError> {
        if id != CollectionId(DMO_BANK_COLLECTION_ID)
            || context.active_collection_id() != CollectionId(DMO_BANK_COLLECTION_ID)
        {
            return Err(external("DMO bank callback belongs to another collection"));
        }
        let mut slot = self
            .hold
            .lock()
            .map_err(|_| external("DMO bank hold lock poisoned"))?;
        if slot.is_some() {
            return Err(external("DMO bank hold already acquired"));
        }
        let (_, hold) = context
            .acquire_active_collection_hold()
            .map_err(|error| Arc::new(Cause::external(Arc::new(error))))?;
        *slot = Some(hold);
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum SelectedDmoBankError {
    Host(&'static str),
    /// Manifest, backend, streamer or registry failure, retained as text.
    Load(String),
}

impl fmt::Display for SelectedDmoBankError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(message) => write!(f, "main DMO bank: {message}"),
            Self::Load(message) => write!(f, "main DMO bank load: {message}"),
        }
    }
}

impl Error for SelectedDmoBankError {}

fn load_error(error: impl fmt::Debug) -> SelectedDmoBankError {
    SelectedDmoBankError::Load(format!("{error:?}"))
}

enum Phase {
    Manifest,
    Resident {
        streamer: Box<Streamer>,
        lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    },
    Ready(SimulationMainDmoBankResidentSnapshot),
    Failed,
}

pub(crate) struct SelectedDmoBankStartup {
    mounts: Arc<MountSet>,
    catalog: WorldStreamCatalog,
    backend: Backend,
    manifest: Manifest,
    hold: Arc<Mutex<Option<ResidentCollectionHold>>>,
    phase: Phase,
}

impl SelectedDmoBankStartup {
    /// Starts the manifest read (`LoadHull("DMO", ..., 8)`).
    pub(crate) fn try_new(mounts: Arc<MountSet>) -> Result<Self, SelectedDmoBankError> {
        let catalog = WorldStreamCatalog::new();
        let backend = MountedStreamBackend::new(Arc::clone(&mounts), catalog.clone())
            .map_err(load_error)?;
        let manifest =
            ManifestOnlyStream::load(DMO_BANK_BASE_PATH, DMO_BANK_BASENAME, StreamKind::Presentation)
                .map_err(load_error)?;
        Ok(Self {
            mounts,
            catalog,
            backend,
            manifest,
            hold: Arc::new(Mutex::new(None)),
            phase: Phase::Manifest,
        })
    }

    /// One manifest or resident update. `Ok(true)` once the whole collection
    /// is held and its exact asset sequence is snapshotted.
    pub(crate) fn try_poll(&mut self) -> Result<bool, SelectedDmoBankError> {
        let result = self.try_poll_inner();
        if result.is_err() {
            self.phase = Phase::Failed;
        }
        result
    }

    fn try_poll_inner(&mut self) -> Result<bool, SelectedDmoBankError> {
        match &mut self.phase {
            Phase::Failed => Err(SelectedDmoBankError::Host("retained failure")),
            Phase::Ready(_) => Ok(true),
            Phase::Manifest => {
                if self.manifest.state() != GameStreamState::LOADED {
                    self.manifest.poll(&mut self.backend).map_err(load_error)?;
                    if let Some(stage) = self.manifest.reference_read_failure() {
                        return Err(load_error(stage));
                    }
                    return Ok(false);
                }
                let collection = self
                    .catalog
                    .collection(DMO_BANK_COLLECTION_ID)
                    .map_err(load_error)?
                    .ok_or(SelectedDmoBankError::Host("manifest has no cPres_Global"))?;
                let factory = Factory::new(
                    Arc::clone(&self.mounts),
                    self.catalog.clone(),
                    RequestIds(Arc::new(AtomicBool::new(false))),
                    DmoBankProcessedArenaAssetBuilderSessions::new(self.catalog.clone()),
                );
                let wake = factory.decompression_kick();
                let (registry, lookup) = Registry::new(registry_error);
                let mut streamer = Box::new(Streamer::new(
                    Storage::new(storage_error),
                    Metadata(self.catalog.clone()),
                    factory,
                    registry,
                    RetainedOnly,
                    wake,
                ));
                streamer.enqueue_load(
                    CollectionId(DMO_BANK_COLLECTION_ID),
                    i32::from_ne_bytes(collection.asset_count.to_ne_bytes()),
                    Some(KeepLoaded {
                        hold: Arc::clone(&self.hold),
                    }),
                );
                self.phase = Phase::Resident { streamer, lookup };
                Ok(false)
            }
            Phase::Resident { streamer, lookup } => {
                streamer.update().map_err(load_error)?;
                let ready = self
                    .hold
                    .lock()
                    .map_err(|_| SelectedDmoBankError::Host("hold lock poisoned"))?
                    .is_some();
                if !ready {
                    return Ok(false);
                }
                let occurrence = lookup
                    .lookup(CollectionId(DMO_BANK_COLLECTION_ID))
                    .map_err(load_error)?
                    .ok_or(SelectedDmoBankError::Host("loaded bank is not published"))?;
                let snapshot = occurrence
                    .try_snapshot_main_dmo_bank()
                    .map_err(|failure| load_error(failure.error()))?;
                self.phase = Phase::Ready(snapshot);
                Ok(true)
            }
        }
    }

    /// The held bank's processed arenas in collection order.
    pub(crate) fn assets(&self) -> Option<&[Arc<DmoBankProcessedArenaAsset>]> {
        match &self.phase {
            Phase::Ready(snapshot) => Some(snapshot.assets()),
            _ => None,
        }
    }
}
