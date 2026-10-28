//! Catalog-backed construction of resident collection load drivers.
//!
//! This module binds registered XSM/XST metadata to the mount-backed load
//! driver. Resident collection storage and native asset materialization remain
//! explicit collaborators.

use crate::{
    collection_load_pump::{
        CollectionAssetMetadata, CollectionStreamFormat, LoadedAssetCollection,
    },
    collection_resident_streamer::{
        CollectionId, ResidentCollectionLoadDriver, ResidentCollectionLoadFactory,
    },
    mounted_collection_producer::{CollectionLogicalRangeSource, MountedCollectionStreamSnapshot},
    resident_collection_load_driver::{
        CollectionAssetBuilder, CollectionDecompressionWake, MountedResidentCollectionLoadDriver,
        MountedResidentCollectionLoadDriverError, MountedResidentCollectionLoadSpec,
    },
    world_stream_catalog::{
        AssetRegistrationSnapshot, CatalogError, CollectionRegistrationSnapshot, WorldStreamCatalog,
    },
};
use std::{error::Error, fmt, sync::Arc};

/// Typed failure from one live XST asset-metadata lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogAssetMetadataError {
    Catalog(CatalogError),
    IdentityMismatch { requested: u64, observed: u64 },
}

impl fmt::Display for CatalogAssetMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(source) => write!(formatter, "world-stream catalog failed: {source}"),
            Self::IdentityMismatch {
                requested,
                observed,
            } => write!(
                formatter,
                "catalog lookup for asset {requested:#018x} returned {observed:#018x}"
            ),
        }
    }
}

impl Error for CatalogAssetMetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(source) => Some(source),
            Self::IdentityMismatch { .. } => None,
        }
    }
}

/// Owning XST metadata adapter used by [`MountedResidentCollectionLoadDriver`].
#[derive(Clone, Debug)]
pub struct CatalogCollectionAssetMetadata {
    catalog: WorldStreamCatalog,
}

impl CatalogCollectionAssetMetadata {
    #[must_use]
    pub const fn new(catalog: WorldStreamCatalog) -> Self {
        Self { catalog }
    }

    #[must_use]
    pub const fn catalog(&self) -> &WorldStreamCatalog {
        &self.catalog
    }
}

impl CollectionAssetMetadata for CatalogCollectionAssetMetadata {
    type AssetInfo = AssetRegistrationSnapshot;
    type Error = CatalogAssetMetadataError;

    fn find_asset_info(&mut self, asset_id: u64) -> Result<Option<Self::AssetInfo>, Self::Error> {
        let snapshot = self
            .catalog
            .asset(asset_id)
            .map_err(CatalogAssetMetadataError::Catalog)?;
        match snapshot {
            Some(snapshot) if snapshot.entry.id != asset_id => {
                Err(CatalogAssetMetadataError::IdentityMismatch {
                    requested: asset_id,
                    observed: snapshot.entry.id,
                })
            }
            other => Ok(other),
        }
    }

    fn first_three_resource_sizes(&mut self, info: &Self::AssetInfo) -> [u32; 3] {
        [
            info.entry.resources[0].size,
            info.entry.resources[1].size,
            info.entry.resources[2].size,
        ]
    }
}

/// Lease retaining one host request identity for a mounted producer.
///
/// Exact native allocation/reuse policy has not been recovered. Implementors
/// must keep `raw()` unique among their live leases and must not return `-1`.
pub trait CollectionRequestIdLease {
    fn raw(&self) -> i32;
}

/// Result-bearing request-ID allocation boundary.
pub trait CollectionRequestIdAllocator {
    type Lease: CollectionRequestIdLease;
    type Error: Error + Send + Sync + 'static;

    /// Acquires one identity whose lifetime extends through the mounted load.
    ///
    /// # Errors
    ///
    /// Reports allocator exhaustion or another typed host failure.
    fn acquire(&mut self) -> Result<Self::Lease, Self::Error>;
}

/// Creates one opaque resource-construction session per physical collection.
///
/// The returned builder owns or borrows the still-unrecovered native resource
/// allocator boundary. This trait does not infer allocator selection.
pub trait CollectionAssetBuilderSessionFactory {
    type Asset;
    type Builder: CollectionAssetBuilder<AssetRegistrationSnapshot, Asset = Self::Asset>;
    type Error: Error + Send + Sync + 'static;

    /// # Errors
    ///
    /// Reports a typed session or allocator setup failure.
    fn begin_session(
        &mut self,
        collection: &CollectionRegistrationSnapshot,
    ) -> Result<Self::Builder, Self::Error>;
}

/// One-tick deferred completion for the native zero-asset path.
///
/// Native queues no file when the target count is zero. Retaining the writer
/// keeps the same resident collection identity alive until the active driver
/// is cleared by the outer streamer.
pub struct DeferredZeroAssetDriver<W> {
    writer: W,
    complete: bool,
}

impl<W> DeferredZeroAssetDriver<W> {
    #[must_use]
    pub const fn new(writer: W) -> Self {
        Self {
            writer,
            complete: false,
        }
    }

    #[must_use]
    pub const fn writer(&self) -> &W {
        &self.writer
    }

    #[must_use]
    pub fn into_writer(self) -> W {
        self.writer
    }
}

impl<W, E> ResidentCollectionLoadDriver<E> for DeferredZeroAssetDriver<W> {
    fn update(&mut self) -> Result<(), E> {
        self.complete = true;
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn highest_stored_block(&self) -> u32 {
        0
    }

    fn loaded_size(&self) -> u32 {
        0
    }
}

/// Unified backend error used as the resident streamer's generic `E`.
#[derive(Debug)]
pub enum CatalogResidentLoadError<SE, IE, SFE, BE> {
    DescriptorIdentityMismatch {
        requested: CollectionId,
        registered: u64,
    },
    AssetCountMismatch {
        queued_bits: u32,
        registered: u32,
    },
    RequestId(Arc<IE>),
    InvalidRequestId(i32),
    BuilderSession(Arc<SFE>),
    Mounted(Arc<MountedResidentCollectionLoadDriverError<SE, CatalogAssetMetadataError, BE>>),
    /// Error from another resident-streamer collaborator. The outer streamer
    /// records its precise [`crate::collection_resident_streamer::ResidentBackendStage`].
    External(Arc<dyn Error + Send + Sync>),
}

impl<SE, IE, SFE, BE> CatalogResidentLoadError<SE, IE, SFE, BE> {
    #[must_use]
    pub fn external(source: Arc<dyn Error + Send + Sync>) -> Self {
        Self::External(source)
    }
}

impl<SE, IE, SFE, BE> fmt::Display for CatalogResidentLoadError<SE, IE, SFE, BE>
where
    SE: fmt::Display,
    IE: fmt::Display,
    SFE: fmt::Display,
    BE: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DescriptorIdentityMismatch {
                requested,
                registered,
            } => write!(
                formatter,
                "collection request {:#018x} received descriptor {registered:#018x}",
                requested.0
            ),
            Self::AssetCountMismatch {
                queued_bits,
                registered,
            } => write!(
                formatter,
                "queued collection count bits {queued_bits:#010x} differ from registered {registered:#010x}"
            ),
            Self::RequestId(source) => write!(formatter, "request-ID allocation failed: {source}"),
            Self::InvalidRequestId(raw) => {
                write!(
                    formatter,
                    "request-ID allocator returned ended sentinel {raw}"
                )
            }
            Self::BuilderSession(source) => {
                write!(formatter, "asset builder session failed: {source}")
            }
            Self::Mounted(source) => write!(formatter, "mounted collection load failed: {source}"),
            Self::External(source) => write!(formatter, "resident collaborator failed: {source}"),
        }
    }
}

impl<SE, IE, SFE, BE> Error for CatalogResidentLoadError<SE, IE, SFE, BE>
where
    SE: Error + Send + Sync + 'static,
    IE: Error + Send + Sync + 'static,
    SFE: Error + Send + Sync + 'static,
    BE: Error + Send + Sync + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RequestId(source) => Some(source.as_ref()),
            Self::BuilderSession(source) => Some(source.as_ref()),
            Self::Mounted(source) => Some(source.as_ref()),
            Self::External(source) => Some(source.as_ref()),
            Self::DescriptorIdentityMismatch { .. }
            | Self::AssetCountMismatch { .. }
            | Self::InvalidRequestId(_) => None,
        }
    }
}

type SharedFactoryError<R, I, BF> = Arc<
    CatalogResidentLoadError<
        <R as CollectionLogicalRangeSource>::Error,
        <I as CollectionRequestIdAllocator>::Error,
        <BF as CollectionAssetBuilderSessionFactory>::Error,
        <<BF as CollectionAssetBuilderSessionFactory>::Builder as CollectionAssetBuilder<
            AssetRegistrationSnapshot,
        >>::Error,
    >,
>;

type MountedDriver<R, W, B> =
    MountedResidentCollectionLoadDriver<R, CatalogCollectionAssetMetadata, W, B>;

type MappedDriverError<R, B, IE, SFE> = Arc<
    CatalogResidentLoadError<
        <R as CollectionLogicalRangeSource>::Error,
        IE,
        SFE,
        <B as CollectionAssetBuilder<AssetRegistrationSnapshot>>::Error,
    >,
>;

/// Sealed associated driver covering native zero and mounted paths.
///
/// Internal state is deliberately private: callers cannot extract the request
/// lease, bypass unified error mapping to drive the mounted implementation, or
/// clear its first mapped fault.
pub struct CatalogResidentCollectionLoadDriver<R, W, B, L, IE, SFE>
where
    R: CollectionLogicalRangeSource,
    B: CollectionAssetBuilder<AssetRegistrationSnapshot>,
    W: LoadedAssetCollection<B::Asset>,
{
    state: CatalogResidentCollectionLoadDriverState<R, W, B, L, IE, SFE>,
}

enum CatalogResidentCollectionLoadDriverState<R, W, B, L, IE, SFE>
where
    R: CollectionLogicalRangeSource,
    B: CollectionAssetBuilder<AssetRegistrationSnapshot>,
    W: LoadedAssetCollection<B::Asset>,
{
    Zero(DeferredZeroAssetDriver<W>),
    Mounted {
        driver: Box<MountedDriver<R, W, B>>,
        request_id: L,
        mapped_fault: Option<MappedDriverError<R, B, IE, SFE>>,
    },
}

impl<R, W, B, L, IE, SFE> CatalogResidentCollectionLoadDriver<R, W, B, L, IE, SFE>
where
    R: CollectionLogicalRangeSource,
    B: CollectionAssetBuilder<AssetRegistrationSnapshot>,
    W: LoadedAssetCollection<B::Asset>,
    L: CollectionRequestIdLease,
{
    #[must_use]
    pub const fn is_zero_asset_path(&self) -> bool {
        matches!(
            self.state,
            CatalogResidentCollectionLoadDriverState::Zero(_)
        )
    }

    /// Returns the retained mounted request identity without exposing its
    /// ownership lease.
    #[must_use]
    pub fn mounted_request_id(&self) -> Option<i32> {
        match &self.state {
            CatalogResidentCollectionLoadDriverState::Zero(_) => None,
            CatalogResidentCollectionLoadDriverState::Mounted { request_id, .. } => {
                Some(request_id.raw())
            }
        }
    }

    /// Returns read-only ownership counters from the mounted producer stream.
    /// The zero-asset path has no physical stream and returns `None`.
    #[must_use]
    pub fn mounted_stream_snapshot(&self) -> Option<MountedCollectionStreamSnapshot> {
        match &self.state {
            CatalogResidentCollectionLoadDriverState::Zero(_) => None,
            CatalogResidentCollectionLoadDriverState::Mounted { driver, .. } => {
                Some(driver.pump().stream().snapshot())
            }
        }
    }
}

impl<R, W, B, L, IE, SFE>
    ResidentCollectionLoadDriver<Arc<CatalogResidentLoadError<R::Error, IE, SFE, B::Error>>>
    for CatalogResidentCollectionLoadDriver<R, W, B, L, IE, SFE>
where
    R: CollectionLogicalRangeSource,
    B: CollectionAssetBuilder<AssetRegistrationSnapshot>,
    W: LoadedAssetCollection<B::Asset>,
    IE: Error + Send + Sync + 'static,
    SFE: Error + Send + Sync + 'static,
{
    fn update(&mut self) -> Result<(), Arc<CatalogResidentLoadError<R::Error, IE, SFE, B::Error>>> {
        match &mut self.state {
            CatalogResidentCollectionLoadDriverState::Zero(driver) => driver.update(),
            CatalogResidentCollectionLoadDriverState::Mounted {
                driver,
                mapped_fault,
                ..
            } => {
                if let Some(error) = mapped_fault {
                    return Err(Arc::clone(error));
                }
                match driver.update_one_tick() {
                    Ok(_) => Ok(()),
                    Err(source) => {
                        let error = Arc::new(CatalogResidentLoadError::Mounted(source));
                        *mapped_fault = Some(Arc::clone(&error));
                        Err(error)
                    }
                }
            }
        }
    }

    fn is_complete(&self) -> bool {
        match &self.state {
            CatalogResidentCollectionLoadDriverState::Zero(driver) => driver.complete,
            CatalogResidentCollectionLoadDriverState::Mounted { driver, .. } => {
                driver.is_complete()
            }
        }
    }

    fn highest_stored_block(&self) -> u32 {
        match &self.state {
            CatalogResidentCollectionLoadDriverState::Zero(_) => 0,
            CatalogResidentCollectionLoadDriverState::Mounted { driver, .. } => {
                driver.highest_stored_block()
            }
        }
    }

    fn loaded_size(&self) -> u32 {
        match &self.state {
            CatalogResidentCollectionLoadDriverState::Zero(_) => 0,
            CatalogResidentCollectionLoadDriverState::Mounted { driver, .. } => {
                driver.loaded_size()
            }
        }
    }
}

/// Catalog-backed resident load factory.
///
/// Its wake allocation is created internally. [`Self::decompression_kick`]
/// is therefore guaranteed to return a handle paired with every mounted
/// driver created by this factory.
pub struct CatalogResidentCollectionLoadFactory<R, I, BF> {
    source: Arc<R>,
    catalog: WorldStreamCatalog,
    request_ids: I,
    builders: BF,
    wake: CollectionDecompressionWake,
}

impl<R, I, BF> CatalogResidentCollectionLoadFactory<R, I, BF> {
    #[must_use]
    pub fn new(source: Arc<R>, catalog: WorldStreamCatalog, request_ids: I, builders: BF) -> Self {
        Self {
            source,
            catalog,
            request_ids,
            builders,
            wake: CollectionDecompressionWake::default(),
        }
    }

    #[must_use]
    pub fn decompression_kick(&self) -> CollectionDecompressionWake {
        self.wake.clone()
    }

    #[must_use]
    pub const fn catalog(&self) -> &WorldStreamCatalog {
        &self.catalog
    }
}

impl<R, I, BF, W>
    ResidentCollectionLoadFactory<W, CollectionRegistrationSnapshot, SharedFactoryError<R, I, BF>>
    for CatalogResidentCollectionLoadFactory<R, I, BF>
where
    R: CollectionLogicalRangeSource,
    I: CollectionRequestIdAllocator,
    BF: CollectionAssetBuilderSessionFactory,
    W: LoadedAssetCollection<BF::Asset>,
{
    type Driver =
        CatalogResidentCollectionLoadDriver<R, W, BF::Builder, I::Lease, I::Error, BF::Error>;

    fn begin(
        &mut self,
        id: CollectionId,
        asset_count: i32,
        descriptor: CollectionRegistrationSnapshot,
        writer: W,
    ) -> Result<Self::Driver, SharedFactoryError<R, I, BF>> {
        if descriptor.id != id.0 {
            return Err(Arc::new(
                CatalogResidentLoadError::DescriptorIdentityMismatch {
                    requested: id,
                    registered: descriptor.id,
                },
            ));
        }
        let queued_bits = u32::from_ne_bytes(asset_count.to_ne_bytes());
        if descriptor.asset_count != queued_bits {
            return Err(Arc::new(CatalogResidentLoadError::AssetCountMismatch {
                queued_bits,
                registered: descriptor.asset_count,
            }));
        }
        if asset_count == 0 {
            return Ok(CatalogResidentCollectionLoadDriver {
                state: CatalogResidentCollectionLoadDriverState::Zero(
                    DeferredZeroAssetDriver::new(writer),
                ),
            });
        }

        let builder = self.builders.begin_session(&descriptor).map_err(|source| {
            Arc::new(CatalogResidentLoadError::BuilderSession(Arc::new(source)))
        })?;
        let request_id = self
            .request_ids
            .acquire()
            .map_err(|source| Arc::new(CatalogResidentLoadError::RequestId(Arc::new(source))))?;
        let raw_request_id = request_id.raw();
        if raw_request_id == -1 {
            return Err(Arc::new(CatalogResidentLoadError::InvalidRequestId(
                raw_request_id,
            )));
        }

        let spec = MountedResidentCollectionLoadSpec {
            path: descriptor.path,
            payload_offset: descriptor.offset,
            request_id: raw_request_id,
            target_assets: asset_count,
            format: CollectionStreamFormat(i32::from_ne_bytes(
                descriptor.stream_format.to_ne_bytes(),
            )),
        };
        let driver = MountedResidentCollectionLoadDriver::new(
            Arc::clone(&self.source),
            spec,
            CatalogCollectionAssetMetadata::new(self.catalog.clone()),
            writer,
            builder,
            self.wake.clone(),
        )
        .map_err(|source| Arc::new(CatalogResidentLoadError::Mounted(source)))?;

        Ok(CatalogResidentCollectionLoadDriver {
            state: CatalogResidentCollectionLoadDriverState::Mounted {
                driver: Box::new(driver),
                request_id,
                mapped_fault: None,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collection_resident_streamer::{
            CollectionResidentStreamer, ResidentAssetGarbageCollector, ResidentCallbackContext,
            ResidentCallbackObservation, ResidentCollectionCallback, ResidentCollectionMetadata,
            ResidentCollectionRegistry, ResidentCollectionStorage,
        },
        game_stream::{StreamFile, StreamKind},
        mounted_collection_producer::COLLECTION_STREAM_READ_SIZE,
        resident_collection_load_driver::CollectionAssetBuildInput,
    };
    use skate2_formats::{
        mount::{LogicalEntryRange, LogicalEntryStorage},
        stream::{
            AssetMapEntry, AssetToc, BaseResourceDescriptor, CollectionMap, CollectionMapEntry,
        },
    };
    use std::{
        collections::HashSet,
        ops::Range,
        sync::{
            Mutex, MutexGuard,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use crate::mounted_stream_backend::StreamRegistrationSink;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl Error for TestError {}

    struct MemorySource {
        bytes: Arc<[u8]>,
        calls: AtomicUsize,
    }

    impl MemorySource {
        fn new(bytes: Vec<u8>) -> Self {
            Self {
                bytes: Arc::from(bytes),
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl CollectionLogicalRangeSource for MemorySource {
        type Error = TestError;

        fn read_range(
            &self,
            _path: &str,
            range: Range<u64>,
        ) -> Result<LogicalEntryRange, Self::Error> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let bytes = if range.is_empty() {
                Vec::new()
            } else {
                self.bytes
                    [usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap()]
                    .to_vec()
            };
            Ok(LogicalEntryRange {
                bytes,
                logical_len: u64::try_from(self.bytes.len()).unwrap(),
                storage: LogicalEntryStorage::Raw,
            })
        }
    }

    struct TestLease {
        raw: i32,
        alive: Arc<AtomicUsize>,
    }

    impl CollectionRequestIdLease for TestLease {
        fn raw(&self) -> i32 {
            self.raw
        }
    }

    impl Drop for TestLease {
        fn drop(&mut self) {
            self.alive.fetch_sub(1, Ordering::Relaxed);
        }
    }

    struct TestIds {
        raw: i32,
        fail: bool,
        calls: Arc<AtomicUsize>,
        alive: Arc<AtomicUsize>,
    }

    impl CollectionRequestIdAllocator for TestIds {
        type Lease = TestLease;
        type Error = TestError;

        fn acquire(&mut self) -> Result<Self::Lease, Self::Error> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.fail {
                return Err(TestError("request id"));
            }
            self.alive.fetch_add(1, Ordering::Relaxed);
            Ok(TestLease {
                raw: self.raw,
                alive: Arc::clone(&self.alive),
            })
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct BuildEvent {
        id: u64,
        raw_format: i32,
        payload: Vec<u8>,
    }

    struct TestBuilder {
        events: Arc<Mutex<Vec<BuildEvent>>>,
        fail_on: HashSet<u64>,
    }

    impl CollectionAssetBuilder<AssetRegistrationSnapshot> for TestBuilder {
        type Asset = u64;
        type Error = TestError;

        fn build(
            &mut self,
            input: CollectionAssetBuildInput<'_, AssetRegistrationSnapshot>,
        ) -> Result<Option<Self::Asset>, Self::Error> {
            lock_recover(&self.events).push(BuildEvent {
                id: input.asset_id,
                raw_format: input.raw_format.0,
                payload: input.payload.to_vec(),
            });
            if self.fail_on.contains(&input.asset_id) {
                return Err(TestError("build"));
            }
            Ok(Some(input.asset_id))
        }
    }

    struct TestSessions {
        calls: Arc<AtomicUsize>,
        fail: bool,
        builder_events: Arc<Mutex<Vec<BuildEvent>>>,
        fail_build: HashSet<u64>,
    }

    impl CollectionAssetBuilderSessionFactory for TestSessions {
        type Asset = u64;
        type Builder = TestBuilder;
        type Error = TestError;

        fn begin_session(
            &mut self,
            _collection: &CollectionRegistrationSnapshot,
        ) -> Result<Self::Builder, Self::Error> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.fail {
                return Err(TestError("session"));
            }
            Ok(TestBuilder {
                events: Arc::clone(&self.builder_events),
                fail_on: self.fail_build.clone(),
            })
        }
    }

    #[derive(Clone)]
    struct TestWriter {
        assets: Arc<Mutex<Vec<u64>>>,
    }

    impl LoadedAssetCollection<u64> for TestWriter {
        fn add_asset(&mut self, asset: u64) {
            lock_recover(&self.assets).push(asset);
        }
    }

    struct TestParts {
        catalog: WorldStreamCatalog,
        descriptor: CollectionRegistrationSnapshot,
    }

    fn catalog_parts(
        collection_id: u64,
        asset_count: u32,
        stream_format: u32,
        assets: Vec<AssetMapEntry>,
    ) -> TestParts {
        let mut catalog = WorldStreamCatalog::new();
        let file = StreamFile::<(), CatalogError>::load("world", "boot", StreamKind::Presentation)
            .unwrap();
        let map = CollectionMap {
            version: 3,
            stamp: 1,
            stream_format,
            name_table_offset: 0,
            collections: vec![CollectionMapEntry {
                id: collection_id,
                parent_id: 0,
                payload_offset: 0,
                payload_size: u32::try_from(COLLECTION_STREAM_READ_SIZE).unwrap(),
                asset_count,
                filename: "collection".to_owned(),
                name_offset: 0,
                alignment_pad: 0,
            }],
        };
        catalog
            .collection_map_loaded(file.registration_context(), &map)
            .unwrap();
        catalog
            .asset_toc_loaded(
                file.registration_context(),
                &AssetToc {
                    version: 3,
                    stamp: 1,
                    name_table_offset: 0,
                    assets,
                },
            )
            .unwrap();
        let descriptor = catalog.collection(collection_id).unwrap().unwrap();
        TestParts {
            catalog,
            descriptor,
        }
    }

    fn asset(id: u64, sizes: [u32; 3]) -> AssetMapEntry {
        let mut resources = [BaseResourceDescriptor {
            size: 0,
            alignment: 1,
        }; 5];
        for (resource, size) in resources.iter_mut().zip(sizes) {
            resource.size = size;
        }
        AssetMapEntry {
            id,
            asset_type: 0,
            serialized_size: 0,
            resources,
            name_offset: 0,
            unused: 0,
        }
    }

    fn record(id: u64, payload: &[u8]) -> Vec<u8> {
        let stride = u32::try_from(24 + payload.len()).unwrap();
        let mut bytes = vec![0; usize::try_from(stride).unwrap()];
        bytes[0..8].copy_from_slice(&id.to_be_bytes());
        bytes[8..12].copy_from_slice(&u32::try_from(payload.len()).unwrap().to_be_bytes());
        bytes[12..16].copy_from_slice(&24_u32.to_be_bytes());
        bytes[16..20].copy_from_slice(&stride.to_be_bytes());
        bytes[24..].copy_from_slice(payload);
        bytes
    }

    struct FactoryHarness {
        factory: CatalogResidentCollectionLoadFactory<MemorySource, TestIds, TestSessions>,
        source: Arc<MemorySource>,
        id_calls: Arc<AtomicUsize>,
        leases: Arc<AtomicUsize>,
        session_calls: Arc<AtomicUsize>,
        build_events: Arc<Mutex<Vec<BuildEvent>>>,
        writer_assets: Arc<Mutex<Vec<u64>>>,
        wake: CollectionDecompressionWake,
    }

    fn factory_harness(
        catalog: WorldStreamCatalog,
        bytes: Vec<u8>,
        raw_request_id: i32,
        fail_id: bool,
        fail_session: bool,
        fail_build: &[u64],
    ) -> FactoryHarness {
        let source = Arc::new(MemorySource::new(bytes));
        let id_calls = Arc::new(AtomicUsize::new(0));
        let leases = Arc::new(AtomicUsize::new(0));
        let session_calls = Arc::new(AtomicUsize::new(0));
        let build_events = Arc::new(Mutex::new(Vec::new()));
        let writer_assets = Arc::new(Mutex::new(Vec::new()));
        let factory = CatalogResidentCollectionLoadFactory::new(
            Arc::clone(&source),
            catalog,
            TestIds {
                raw: raw_request_id,
                fail: fail_id,
                calls: Arc::clone(&id_calls),
                alive: Arc::clone(&leases),
            },
            TestSessions {
                calls: Arc::clone(&session_calls),
                fail: fail_session,
                builder_events: Arc::clone(&build_events),
                fail_build: fail_build.iter().copied().collect(),
            },
        );
        let wake = factory.decompression_kick();
        FactoryHarness {
            factory,
            source,
            id_calls,
            leases,
            session_calls,
            build_events,
            writer_assets,
            wake,
        }
    }

    fn writer(harness: &FactoryHarness) -> TestWriter {
        TestWriter {
            assets: Arc::clone(&harness.writer_assets),
        }
    }

    #[test]
    fn catalog_asset_metadata_preserves_full_identity_and_resource_words() {
        let id = 0xfedc_ba98_7654_3210;
        let parts = catalog_parts(
            1,
            1,
            3,
            vec![asset(id, [7, 11, u32::MAX]), asset(id, [1, 2, 3])],
        );
        let mut metadata = CatalogCollectionAssetMetadata::new(parts.catalog);
        let snapshot = metadata.find_asset_info(id).unwrap().unwrap();
        assert_eq!(snapshot.entry.id, id);
        assert_eq!(snapshot.reference_count, 2);
        assert_eq!(
            metadata.first_three_resource_sizes(&snapshot),
            [7, 11, u32::MAX]
        );
        assert!(metadata.find_asset_info(id ^ 1).unwrap().is_none());
    }

    #[test]
    fn identity_and_raw_count_validation_precede_all_factory_side_effects() {
        let parts = catalog_parts(10, 1, 0, vec![]);
        let mut harness = factory_harness(parts.catalog, Vec::new(), 9, false, false, &[]);
        let error = harness
            .factory
            .begin(
                CollectionId(11),
                1,
                parts.descriptor.clone(),
                writer(&harness),
            )
            .err()
            .unwrap();
        assert!(matches!(
            &*error,
            CatalogResidentLoadError::DescriptorIdentityMismatch { .. }
        ));

        let error = harness
            .factory
            .begin(CollectionId(10), -1, parts.descriptor, writer(&harness))
            .err()
            .unwrap();
        assert!(matches!(
            &*error,
            CatalogResidentLoadError::AssetCountMismatch {
                queued_bits: u32::MAX,
                registered: 1
            }
        ));
        assert_eq!(harness.session_calls.load(Ordering::Relaxed), 0);
        assert_eq!(harness.id_calls.load(Ordering::Relaxed), 0);
        assert_eq!(harness.source.calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn zero_path_defers_one_update_and_touches_no_physical_collaborator() {
        let parts = catalog_parts(20, 0, 0xffff_ffff, vec![]);
        let mut harness = factory_harness(parts.catalog, Vec::new(), 9, false, false, &[]);
        let mut driver = harness
            .factory
            .begin(CollectionId(20), 0, parts.descriptor, writer(&harness))
            .unwrap();
        assert!(driver.is_zero_asset_path());
        assert_eq!(driver.mounted_request_id(), None);
        assert_eq!(driver.mounted_stream_snapshot(), None);
        assert!(!driver.is_complete());
        assert_eq!(driver.highest_stored_block(), 0);
        assert_eq!(driver.loaded_size(), 0);
        driver.update().unwrap();
        assert!(driver.is_complete());
        assert_eq!(driver.mounted_stream_snapshot(), None);
        assert_eq!(harness.session_calls.load(Ordering::Relaxed), 0);
        assert_eq!(harness.id_calls.load(Ordering::Relaxed), 0);
        assert_eq!(harness.source.calls.load(Ordering::Relaxed), 0);
        assert!(!harness.wake.is_pending());
    }

    type HarnessBackendError = SharedFactoryError<MemorySource, TestIds, TestSessions>;

    struct OuterStorage;

    impl ResidentCollectionStorage<HarnessBackendError> for OuterStorage {
        type Collection = TestWriter;
        type Writer = TestWriter;

        fn create_empty(&mut self, _reserve: i32) -> Result<Self::Collection, HarnessBackendError> {
            Ok(TestWriter {
                assets: Arc::new(Mutex::new(Vec::new())),
            })
        }

        fn writer(
            &mut self,
            collection: &Self::Collection,
        ) -> Result<Self::Writer, HarnessBackendError> {
            Ok(collection.clone())
        }

        fn asset_ids_in_order(
            &mut self,
            collection: &Self::Collection,
        ) -> Result<Vec<CollectionId>, HarnessBackendError> {
            Ok(lock_recover(&collection.assets)
                .iter()
                .copied()
                .map(CollectionId)
                .collect())
        }

        fn destroy(&mut self, _collection: Self::Collection) {}
    }

    struct OuterMetadata(WorldStreamCatalog);

    impl ResidentCollectionMetadata<HarnessBackendError> for OuterMetadata {
        type Descriptor = CollectionRegistrationSnapshot;

        fn find_collection(
            &mut self,
            id: CollectionId,
        ) -> Result<Option<Self::Descriptor>, HarnessBackendError> {
            self.0
                .collection(id.0)
                .map_err(|source| Arc::new(CatalogResidentLoadError::External(Arc::new(source))))
        }
    }

    struct OuterRegistry;

    impl ResidentCollectionRegistry<TestWriter, HarnessBackendError> for OuterRegistry {
        type Wrapper = ();
        type Token = ();

        fn create_wrapper(
            &mut self,
            _id: CollectionId,
            _collection: &TestWriter,
        ) -> Result<Self::Wrapper, HarnessBackendError> {
            Ok(())
        }

        fn stable_token(_wrapper: &Self::Wrapper) -> Self::Token {}

        fn publish(
            &mut self,
            _id: CollectionId,
            _token: &Self::Token,
        ) -> Result<(), HarnessBackendError> {
            Ok(())
        }

        fn remove_exact(
            &mut self,
            _id: CollectionId,
            _token: &Self::Token,
        ) -> Result<(), HarnessBackendError> {
            Ok(())
        }
    }

    struct OuterGarbage;

    impl ResidentAssetGarbageCollector<HarnessBackendError> for OuterGarbage {
        fn garbage_collect(&mut self, _id: CollectionId) -> Result<(), HarnessBackendError> {
            Ok(())
        }
    }

    struct OuterCallback;

    impl ResidentCollectionCallback<HarnessBackendError> for OuterCallback {
        fn on_complete(
            &mut self,
            _id: CollectionId,
            _observation: ResidentCallbackObservation,
            _context: &mut ResidentCallbackContext<'_, Self>,
        ) -> Result<(), HarnessBackendError> {
            Ok(())
        }
    }

    #[test]
    fn zero_factory_preserves_three_update_outer_trace() {
        let parts = catalog_parts(21, 0, 0, vec![]);
        let harness = factory_harness(parts.catalog.clone(), Vec::new(), 9, false, false, &[]);
        let FactoryHarness {
            factory,
            source,
            id_calls,
            session_calls,
            wake,
            ..
        } = harness;
        let mut streamer = CollectionResidentStreamer::<_, _, _, _, _, _, OuterCallback, _>::new(
            OuterStorage,
            OuterMetadata(parts.catalog),
            factory,
            OuterRegistry,
            OuterGarbage,
            wake,
        );
        streamer.enqueue_load(CollectionId(21), 0, None);
        streamer.enqueue_load(CollectionId(21), 0, None);

        let first = streamer.update().unwrap();
        assert_eq!(first.physical_loads_started, 1);
        assert_eq!(first.physical_loads_completed, 0);
        assert_eq!(first.cache_hits, 0);
        assert_eq!(streamer.queued_request_count(), 1);

        let second = streamer.update().unwrap();
        assert_eq!(second.physical_loads_started, 0);
        assert_eq!(second.physical_loads_completed, 1);
        assert_eq!(second.cache_hits, 0);
        assert_eq!(streamer.queued_request_count(), 1);

        let third = streamer.update().unwrap();
        assert_eq!(third.physical_loads_started, 0);
        assert_eq!(third.physical_loads_completed, 0);
        assert_eq!(third.cache_hits, 1);
        assert_eq!(streamer.queued_request_count(), 0);
        assert_eq!(session_calls.load(Ordering::Relaxed), 0);
        assert_eq!(id_calls.load(Ordering::Relaxed), 0);
        assert_eq!(source.calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn matching_negative_count_and_high_format_bits_take_mounted_path() {
        let asset_id = 0x1000_0000_0000_0001;
        let parts = catalog_parts(30, u32::MAX, 0x8000_0000, vec![asset(asset_id, [1, 0, 0])]);
        let mut harness = factory_harness(
            parts.catalog,
            record(asset_id, &[0x5a]),
            -2,
            false,
            false,
            &[],
        );
        let mut driver = harness
            .factory
            .begin(CollectionId(30), -1, parts.descriptor, writer(&harness))
            .unwrap();
        assert!(!driver.is_zero_asset_path());
        assert_eq!(driver.mounted_request_id(), Some(-2));
        assert_eq!(harness.leases.load(Ordering::Relaxed), 1);
        driver.update().unwrap();
        assert!(lock_recover(&harness.build_events).is_empty());
        assert!(harness.wake.request());
        driver.update().unwrap();
        assert!(driver.is_complete());
        let snapshot = driver.mounted_stream_snapshot().unwrap();
        assert_eq!(snapshot.queued_chunks, 0);
        assert_eq!(snapshot.retained_chunks, 0);
        assert!(snapshot.producer_finished);
        assert!(!snapshot.producer_failed);
        assert_eq!(
            lock_recover(&harness.build_events).as_slice(),
            [BuildEvent {
                id: asset_id,
                raw_format: i32::MIN,
                payload: vec![0x5a],
            }]
        );
        assert_eq!(lock_recover(&harness.writer_assets).as_slice(), [asset_id]);
        assert_eq!(harness.leases.load(Ordering::Relaxed), 1);
        drop(driver);
        assert_eq!(harness.leases.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn producer_construction_failure_releases_lease_without_source_io() {
        let parts = catalog_parts(43, 1, 0, vec![]);
        let mut descriptor = parts.descriptor;
        descriptor.path = Arc::from("bad\0path.xsf");
        let mut harness = factory_harness(parts.catalog, Vec::new(), 6, false, false, &[]);
        let error = harness
            .factory
            .begin(CollectionId(43), 1, descriptor, writer(&harness))
            .err()
            .unwrap();
        assert!(matches!(
            &*error,
            CatalogResidentLoadError::Mounted(source)
                if matches!(
                    &**source,
                    MountedResidentCollectionLoadDriverError::Producer(producer)
                        if matches!(
                            &**producer,
                            crate::mounted_collection_producer::MountedCollectionProducerError::PathContainsNul
                        )
                )
        ));
        assert_eq!(harness.session_calls.load(Ordering::Relaxed), 1);
        assert_eq!(harness.id_calls.load(Ordering::Relaxed), 1);
        assert_eq!(harness.leases.load(Ordering::Relaxed), 0);
        assert_eq!(harness.source.calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn request_and_session_failures_are_distinct_and_release_any_lease() {
        let session_parts = catalog_parts(40, 1, 0, vec![]);
        let mut session = factory_harness(session_parts.catalog, Vec::new(), 5, false, true, &[]);
        let error = session
            .factory
            .begin(
                CollectionId(40),
                1,
                session_parts.descriptor,
                writer(&session),
            )
            .err()
            .unwrap();
        assert!(matches!(
            &*error,
            CatalogResidentLoadError::BuilderSession(_)
        ));
        assert_eq!(session.id_calls.load(Ordering::Relaxed), 0);

        let id_parts = catalog_parts(41, 1, 0, vec![]);
        let mut ids = factory_harness(id_parts.catalog, Vec::new(), 5, true, false, &[]);
        let error = ids
            .factory
            .begin(CollectionId(41), 1, id_parts.descriptor, writer(&ids))
            .err()
            .unwrap();
        assert!(matches!(&*error, CatalogResidentLoadError::RequestId(_)));
        assert_eq!(ids.session_calls.load(Ordering::Relaxed), 1);
        assert_eq!(ids.leases.load(Ordering::Relaxed), 0);

        let sentinel_parts = catalog_parts(42, 1, 0, vec![]);
        let mut sentinel =
            factory_harness(sentinel_parts.catalog, Vec::new(), -1, false, false, &[]);
        let error = sentinel
            .factory
            .begin(
                CollectionId(42),
                1,
                sentinel_parts.descriptor,
                writer(&sentinel),
            )
            .err()
            .unwrap();
        assert!(matches!(
            &*error,
            CatalogResidentLoadError::InvalidRequestId(-1)
        ));
        assert_eq!(sentinel.leases.load(Ordering::Relaxed), 0);
        assert_eq!(sentinel.source.calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn mounted_builder_failure_replays_same_unified_arc_and_retains_lease() {
        let asset_id = 77;
        let parts = catalog_parts(50, 1, 3, vec![asset(asset_id, [1, 0, 0])]);
        let mut harness = factory_harness(
            parts.catalog,
            record(asset_id, &[1]),
            4,
            false,
            false,
            &[asset_id],
        );
        let mut driver = harness
            .factory
            .begin(CollectionId(50), 1, parts.descriptor, writer(&harness))
            .unwrap();
        assert_eq!(driver.mounted_request_id(), Some(4));
        driver.update().unwrap();
        assert!(harness.wake.request());
        let first = driver.update().unwrap_err();
        assert!(matches!(&*first, CatalogResidentLoadError::Mounted(_)));
        let second = driver.update().unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(driver.mounted_request_id(), Some(4));
        assert_eq!(lock_recover(&harness.build_events).len(), 1);
        assert_eq!(harness.leases.load(Ordering::Relaxed), 1);
        drop(driver);
        assert_eq!(harness.leases.load(Ordering::Relaxed), 0);
    }

    fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
