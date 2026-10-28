//! Checked bridge between the outer BAM controller and its inner stream files.
//!
//! `cAssetStreamSystem::UpdateStreamFileLoaded` at `0x824e1ad0` publishes
//! spatial records and then the path-sorted stream-file entry only after
//! `cStreamFile::Update` has exposed raw state four. This adapter keeps that
//! ordering explicit without reconstructing the native global containers.
//!
//! Global ID-map plus sorted-file publication is transactional at this safe PC
//! boundary. Presentation `AddHighCollection` notifications remain immediate,
//! as in the reference; recovery/discard ownership after a later construction
//! failure is unknown. The safe owner latches that failure and performs neither
//! a guessed rollback nor a retry; eventual discard remains outside this slice.

use crate::{
    game::{StreamFileState, StreamXmlState, WorldStreamBackend, WorldStreamType},
    game_stream::{
        InnerStreamBackend, InnerStreamError, ReadStage, StreamFile, StreamFileUnloadError,
        StreamKind, StreamPathError, StreamRegistrationContext,
    },
};
use skate2_formats::stream::CollectionSpaceEntry;
use std::{
    cmp::Ordering,
    fmt,
    sync::{Arc, LockResult, Mutex, MutexGuard},
};

/// Exact four-lane expansion vector built from `kCollectionBoxSize` at
/// `0x820126e0` and a zero fourth lane.
pub const COLLECTION_BOX_EXPANSION_WORDS: [u32; 4] =
    [0x4348_0000, 0x4348_0000, 0x4348_0000, 0x0000_0000];

/// One of the three native read-failure stalls hidden by the outer bool API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceStreamStall {
    CollectionMap {
        stream_file_state: u32,
        game_stream_state: u32,
    },
    AssetToc {
        stream_file_state: u32,
        game_stream_state: u32,
    },
    CollectionSpace {
        stream_file_state: u32,
        game_stream_state: u32,
    },
}

/// Fallible collaborator phase within inner processing or final publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationPhase {
    RetainStreamFileIdentity,
    CheckCollectionValidity { source_index: usize },
    GetStreamOffset { source_index: usize },
    GetTicksStreamSize { source_index: usize },
    GetRawStreamSize { source_index: usize },
    ConstructLocalSpatialIndex,
    InsertLocalSpatialRecord { compact_index: u32 },
    PublishLoadedStream,
    ReleaseLocalSpatial,
    UnpublishLoadedStream,
}

/// Observable adapter ownership stage used only for typed invariant reports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterPublicationStage {
    PreparingEmpty,
    PreparingLocal,
    PublishedLocalRetained,
    PublishedLocalReleased,
    PublishedReleaseFailed,
    Unpublished,
}

/// Error carried through [`StreamFile`] while the adapter is servicing an
/// inner callback. The indirection preserves the original shared adapter
/// error when local release fails before raw state 6.
#[derive(Debug)]
pub enum AdapterBackendError<InnerError, PublicationError> {
    Inner(InnerError),
    Adapter(Arc<WorldStreamAdapterError<InnerError, PublicationError>>),
}

/// Terminal adapter failure. The owning file latches the first value.
#[derive(Debug)]
pub enum WorldStreamAdapterError<InnerError, PublicationError> {
    TextureGate(PublicationError),
    Path(StreamPathError),
    Inner(Arc<InnerStreamError<AdapterBackendError<InnerError, PublicationError>>>),
    ReferenceStall {
        kind: StreamKind,
        stall: ReferenceStreamStall,
    },
    BasenameMismatch {
        retained: String,
        supplied: String,
    },
    MissingProcessedOctreeRecords {
        kind: StreamKind,
    },
    MissingLocalSpatialIndex {
        kind: StreamKind,
    },
    /// Safe deviation for VMX/libm inputs whose console conversion result is
    /// not established; ordinary finite corpus coordinates do not take it.
    UnsupportedSpatialCoordinate {
        id: u64,
        axis: usize,
        bits: u32,
    },
    /// Safe deviation instead of allowing a compact index to truncate.
    ProcessedIndexOverflow {
        index: usize,
    },
    ProcessedStorage(ProcessedOctreeStorageError),
    PublicationLifecycleMismatch {
        kind: StreamKind,
        stream_file_state: u32,
        stage: AdapterPublicationStage,
    },
    Publication {
        kind: StreamKind,
        phase: PublicationPhase,
        source: PublicationError,
    },
}

/// Rejection from the explicit adapter unload request. Request misuse leaves
/// the file unchanged and is not added to the poll-time terminal latch.
#[derive(Debug)]
pub enum WorldStreamUnloadRequestError<InnerError, PublicationError> {
    Terminal(Arc<WorldStreamAdapterError<InnerError, PublicationError>>),
    Request(StreamFileUnloadError),
}

type SharedAdapterError<I, P> = Arc<WorldStreamAdapterError<I, P>>;
type AdapterFile<I, P> = AdaptedStreamFile<
    <I as InnerStreamBackend>::Handle,
    <I as InnerStreamBackend>::Error,
    <P as LoadedStreamPublisher>::Error,
    <P as LoadedStreamPublisher>::StreamFileIdentity,
    <P as LoadedStreamPublisher>::LocalSpatialIndexIdentity,
    <P as LoadedStreamPublisher>::PublicationToken,
>;

/// Immutable metadata from a processed native `tOctreeInfo`, retained
/// independently of its raw XSS source entry. Field provenance is PDB
/// `tOctreeInfo`: bbox
/// `0x00`, geometry bbox `0x20`, ID `0x40`, offset `0x48`, position `0x50`,
/// stream order `0x60`, compact bbox index `0x64`, ticks `0x68`, size `0x6c`,
/// and stream type `0x70`. This safe Rust representation does not claim native
/// ABI layout.
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessedOctreeMetadata {
    pub bbox_min_words: [u32; 4],
    pub bbox_max_words: [u32; 4],
    pub geometry_min_words: [u32; 4],
    pub geometry_max_words: [u32; 4],
    pub id: u64,
    pub offset: u64,
    pub position_words: [u32; 4],
    pub stream_order: i32,
    pub bbox_index: i32,
    /// Signed PDB field populated from an unsigned quotient; raw bits retained.
    pub ticks: i32,
    /// Signed PDB field populated directly from a returned unsigned word.
    pub size: i32,
    pub stream_type: StreamKind,
}

/// Mutable PDB bytes at `tOctreeInfo +0x74..+0x76`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessedOctreeLifecycle {
    pub loaded_count: u8,
    pub activated: u8,
    pub activation: u8,
}

/// Stable processed-record identity with immutable metadata and synchronized
/// lifecycle bytes.
///
/// The reference mutates the three trailing bytes through pointers held by
/// global containers. A `Mutex` is the high-confidence safe PC ownership
/// choice; it does not claim that the console used this synchronization.
#[derive(Debug)]
pub struct ProcessedOctreeInfo {
    pub metadata: ProcessedOctreeMetadata,
    lifecycle: Mutex<ProcessedOctreeLifecycle>,
}

/// Native resized-vector semantics without inventing values for its unused
/// tail. `raw_logical_len` matches `resize(raw_count)`; `accepted_prefix`
/// contains only the compact records initialized before `mNum` advances.
/// Evidence is `0x824e0d14..0d24` and `0x824e1114..1120`; the resize helper's
/// inserted temporary is uninitialized, so the opaque tail has no value API.
#[derive(Debug)]
pub struct ProcessedOctreeStorage {
    raw_logical_len: u32,
    accepted_prefix: Arc<[ProcessedOctreeInfo]>,
    spatial_bounds: LocalSpatialIndexBounds,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessedOctreeStorageError {
    pub raw_logical_len: u32,
    pub accepted_len: usize,
}

impl fmt::Display for ProcessedOctreeStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "accepted processed prefix {} exceeds raw logical length {}",
            self.accepted_len, self.raw_logical_len
        )
    }
}

impl std::error::Error for ProcessedOctreeStorageError {}

impl ProcessedOctreeStorage {
    /// Builds storage while enforcing that the initialized prefix fits within
    /// the native raw logical length.
    ///
    /// # Errors
    ///
    /// Returns an error rather than synthesizing native uninitialized tail
    /// values when the accepted prefix is longer than the raw vector.
    pub fn new(
        raw_logical_len: u32,
        accepted_prefix: Arc<[ProcessedOctreeInfo]>,
    ) -> Result<Self, ProcessedOctreeStorageError> {
        if accepted_prefix.len() > usize::try_from(raw_logical_len).unwrap_or(usize::MAX) {
            return Err(ProcessedOctreeStorageError {
                raw_logical_len,
                accepted_len: accepted_prefix.len(),
            });
        }
        let spatial_bounds = local_spatial_index_bounds(&accepted_prefix);
        Ok(Self {
            raw_logical_len,
            accepted_prefix,
            spatial_bounds,
        })
    }

    /// The same accepted-prefix calculation supplied to local `Octree` creation.
    #[must_use]
    pub const fn spatial_bounds(&self) -> LocalSpatialIndexBounds {
        self.spatial_bounds
    }

    #[must_use]
    pub const fn raw_logical_len(&self) -> u32 {
        self.raw_logical_len
    }

    #[must_use]
    pub fn accepted_prefix(&self) -> &[ProcessedOctreeInfo] {
        &self.accepted_prefix
    }

    #[must_use]
    pub const fn accepted_prefix_identity(&self) -> &Arc<[ProcessedOctreeInfo]> {
        &self.accepted_prefix
    }

    #[must_use]
    pub fn opaque_tail_len(&self) -> usize {
        usize::try_from(self.raw_logical_len).unwrap_or(usize::MAX) - self.accepted_prefix.len()
    }
}

impl ProcessedOctreeInfo {
    #[must_use]
    pub const fn new(metadata: ProcessedOctreeMetadata) -> Self {
        Self {
            metadata,
            lifecycle: Mutex::new(ProcessedOctreeLifecycle {
                loaded_count: 0,
                activated: 0,
                activation: 0,
            }),
        }
    }

    /// Acquires the synchronized mutable lifecycle bytes.
    ///
    /// # Errors
    ///
    /// Returns the standard poison error if another owner panicked while
    /// mutating the lifecycle.
    pub fn lifecycle(&self) -> LockResult<MutexGuard<'_, ProcessedOctreeLifecycle>> {
        self.lifecycle.lock()
    }
}

/// Native C-path bytes retained without UTF-8 normalization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCPath {
    bytes: Arc<[u8]>,
}

impl NativeCPath {
    #[must_use]
    pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }

    #[must_use]
    pub fn stored_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Bytes observed by native C-string comparison, excluding the first NUL
    /// and all retained bytes after it.
    #[must_use]
    pub fn c_string_bytes(&self) -> &[u8] {
        let end = self
            .bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.bytes.len());
        &self.bytes[..end]
    }
}

/// File-local octree extent after aggregate bounds are expanded by 200.0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalSpatialBoundsWords {
    pub min: [u32; 4],
    pub max: [u32; 4],
}

/// Aggregate accepted bounds and the exact 200.0-expanded Octree extent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalSpatialIndexBounds {
    pub aggregate: LocalSpatialBoundsWords,
    pub expanded: LocalSpatialBoundsWords,
}

impl<I: fmt::Debug, P: fmt::Debug> fmt::Display for WorldStreamAdapterError<I, P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<I: fmt::Debug, P: fmt::Debug> fmt::Display for AdapterBackendError<I, P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<I: fmt::Debug, P: fmt::Debug> fmt::Display for WorldStreamUnloadRequestError<I, P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<I, P> std::error::Error for WorldStreamAdapterError<I, P>
where
    I: std::error::Error + 'static,
    P: std::error::Error + 'static,
{
}

impl<I, P> std::error::Error for AdapterBackendError<I, P>
where
    I: std::error::Error + 'static,
    P: std::error::Error + 'static,
{
}

impl<I, P> std::error::Error for WorldStreamUnloadRequestError<I, P>
where
    I: std::error::Error + 'static,
    P: std::error::Error + 'static,
{
}

/// Global side effects owned by `cAssetStreamSystem` in the reference.
///
/// During XSS completion, the adapter skips nonzero source LOD bytes, checks
/// remaining IDs against the live collection map, samples offset and size
/// metadata, and retains a processed record only for an ID not already valid.
/// After local spatial construction succeeds, one transaction publishes those
/// processed identities and the actual stream-file identity, matching
/// `0x824e0d64..0x824e1124` then `0x824e1b2c..0x824e1cbc`.
/// The successful publication token keeps the local-spatial release and global
/// unpublication capability paired with this exact file through raw states 5
/// and 6.
pub trait LoadedStreamPublisher {
    type Error;
    /// Stable identity of the native-equivalent stream-file object.
    type StreamFileIdentity;
    /// Stable identity of the file-local spatial index built before raw state 4.
    type LocalSpatialIndexIdentity;
    /// One-file capability returned by successful global publication.
    type PublicationToken;

    /// # Errors
    ///
    /// `context.id()` is the exact stable identity also supplied to the inner
    /// registration catalog. It prevents two simultaneous equal kind/path
    /// files from being correlated by path guesswork.
    ///
    /// Returns the collaborator error if a stable identity cannot be retained
    /// for this stream file.
    fn retain_stream_file_identity(
        &mut self,
        context: StreamRegistrationContext<'_>,
        file_path: &NativeCPath,
    ) -> Result<Self::StreamFileIdentity, Self::Error>;

    /// # Errors
    ///
    /// Returns the collaborator error if the current global collection map
    /// cannot be queried.
    fn is_collection_valid(&mut self, id: u64) -> Result<bool, Self::Error>;

    /// Returns the source collection's stream offset. Native calls this once
    /// for every accepted record.
    ///
    /// # Errors
    ///
    /// Returns the collaborator error if metadata is unavailable.
    fn stream_offset(&mut self, id: u64) -> Result<u64, Self::Error>;

    /// Returns the source collection's stream size. Native calls this twice
    /// independently per accepted record: first for ticks, then raw size.
    ///
    /// # Errors
    ///
    /// Returns the collaborator error if metadata is unavailable.
    fn stream_size(&mut self, id: u64) -> Result<u32, Self::Error>;

    /// Presentation-only notification issued after the compact processed count
    /// has advanced. The two coordinates preserve the native unsigned words.
    fn add_high_collection(&mut self, id: u64, x: u32, z: u32);

    /// Constructs and stores the opaque file-local `rw::collision::Octree`
    /// identity before any accepted record is inserted.
    ///
    /// The collaborator preserves descriptor (`0x824e12fc..1300`), allocator
    /// (`0x824e1310..1354`), and Initialize (`0x824e1358..1364`) order. The
    /// adapter stores the returned identity in its file before inserts,
    /// corresponding to `0x824e1368..136c`.
    ///
    /// `raw_logical_len` is both the resized native vector's logical length and
    /// the Octree descriptor/constructor `maxEntries`. `bounds` retains
    /// both the vmin/vmax aggregate (or the zero-record `[-1,+1]` default) and
    /// the extent expanded by exact raw `200.0`. The Octree's internal
    /// representation is intentionally outside this boundary.
    ///
    /// # Errors
    ///
    /// A failure is a safe high-confidence PC deviation. Native asserts after
    /// a null result and may continue; partial allocation ownership is unknown.
    fn construct_local_spatial_index(
        &mut self,
        kind: StreamKind,
        raw_logical_len: u32,
        bounds: LocalSpatialIndexBounds,
    ) -> Result<Self::LocalSpatialIndexIdentity, Self::Error>;

    /// Inserts one compact accepted record into the already-stored local
    /// Octree. Native forwards the compact index and processed bbox at
    /// `0x824e1398..13c0`; all calls finish before raw state 4 at
    /// `0x824e13c4..13dc`.
    ///
    /// # Errors
    ///
    /// A failure is a safe high-confidence PC deviation because the reference
    /// has no recoverable insertion channel. Prior immediate `AddHigh` calls and
    /// any partially built local index remain owned until this file is dropped;
    /// native recovery ownership is unknown.
    fn insert_local_spatial_record(
        &mut self,
        local_spatial_index: &mut Self::LocalSpatialIndexIdentity,
        compact_index: u32,
        bbox_min_words: [u32; 4],
        bbox_max_words: [u32; 4],
    ) -> Result<(), Self::Error>;

    /// Atomically publishes `records.accepted_prefix()` in compact order,
    /// retaining the first processed identity when IDs repeat, then inserts
    /// `stream_file` into the kind vector by unsigned C-string `file_path`
    /// order, after existing equal paths. The implementation must retain `Arc`
    /// clones for every globally referenced identity. The complete storage is
    /// retained so its distinct raw logical length is not lost; its opaque tail
    /// is never read or synthesized.
    ///
    /// On `Err`, this method must leave no partial insertion in the global ID
    /// map or sorted file vector. Presentation `add_high_collection` calls are
    /// native-immediate and deliberately outside this transaction.
    /// [`native_stream_file_insert_index`] defines the exact stable ordering.
    /// Native first-wins ID insertion and publication of both colliding file
    /// identities are exact; atomic failure handling is a high-confidence Rust
    /// collaborator contract, not a claimed generic container implementation.
    /// `local_spatial_index` transfers the adapter's unique owned value into
    /// the publisher; no adapter-side strong alias remains afterward.
    ///
    /// # Errors
    ///
    /// Returns the collaborator error when the complete transaction cannot be
    /// committed.
    fn publish_loaded_stream_transactionally(
        &mut self,
        kind: StreamKind,
        stream_file: &Arc<Self::StreamFileIdentity>,
        local_spatial_index: Self::LocalSpatialIndexIdentity,
        file_path: &NativeCPath,
        records: &Arc<ProcessedOctreeStorage>,
    ) -> Result<Self::PublicationToken, Self::Error>;

    /// Synchronously releases the publisher-owned local spatial value during
    /// `cStreamFile::Update`'s final unload callback, before file raw state 6
    /// can be written (`0x824e1420..0x824e1498`).
    ///
    /// # Errors
    ///
    /// Returns the collaborator error without permitting raw state 6.
    fn release_local_spatial(&mut self, token: &Self::PublicationToken) -> Result<(), Self::Error>;

    /// Removes global ID-map/file-vector ownership only after the same update
    /// has exposed exact file raw state 6 (`0x824e1d2c..0x824e1d50`).
    ///
    /// # Errors
    ///
    /// Returns the collaborator error while retaining the token and the
    /// adapter's processed/file identities for terminal-state inspection.
    fn unpublish_loaded_stream(
        &mut self,
        token: &Self::PublicationToken,
    ) -> Result<(), Self::Error>;

    /// Samples the live texture cVar at the outer simulation-XML transition.
    /// # Errors
    /// Returns the live target failure before the outer state advances.
    fn texture_stream_enabled(&mut self) -> Result<bool, Self::Error>;

    fn finalize_world_bounds(&mut self);
}

/// File owned by the outer game-stream record.
pub struct AdaptedStreamFile<
    Handle,
    InnerError,
    PublicationError,
    StreamFileIdentity,
    LocalSpatialIndexIdentity,
    PublicationToken,
> {
    kind: StreamKind,
    basename: String,
    native_file_path: NativeCPath,
    inner: StreamFile<Handle, AdapterBackendError<InnerError, PublicationError>>,
    stream_file_identity: Arc<StreamFileIdentity>,
    processed_octree_storage: Option<Arc<ProcessedOctreeStorage>>,
    publication: AdapterPublication<LocalSpatialIndexIdentity, PublicationToken>,
    terminal_error: Option<SharedAdapterError<InnerError, PublicationError>>,
}

enum AdapterPublication<L, T> {
    Preparing {
        local_spatial_index: Option<L>,
    },
    Published {
        token: T,
        local_release: AdapterLocalRelease,
    },
    Unpublished,
}

impl<L, T> AdapterPublication<L, T> {
    const fn stage(&self) -> AdapterPublicationStage {
        match self {
            Self::Preparing {
                local_spatial_index: None,
            } => AdapterPublicationStage::PreparingEmpty,
            Self::Preparing {
                local_spatial_index: Some(_),
            } => AdapterPublicationStage::PreparingLocal,
            Self::Published {
                local_release: AdapterLocalRelease::Retained,
                ..
            } => AdapterPublicationStage::PublishedLocalRetained,
            Self::Published {
                local_release: AdapterLocalRelease::Released,
                ..
            } => AdapterPublicationStage::PublishedLocalReleased,
            Self::Published {
                local_release: AdapterLocalRelease::ReleaseAttemptFailed,
                ..
            } => AdapterPublicationStage::PublishedReleaseFailed,
            Self::Unpublished => AdapterPublicationStage::Unpublished,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AdapterLocalRelease {
    Retained,
    Released,
    ReleaseAttemptFailed,
}

impl<H, I, P, S, L, T> AdaptedStreamFile<H, I, P, S, L, T> {
    #[must_use]
    pub const fn kind(&self) -> StreamKind {
        self.kind
    }

    #[must_use]
    pub fn basename(&self) -> &str {
        &self.basename
    }

    #[must_use]
    pub const fn is_globally_published(&self) -> bool {
        matches!(self.publication, AdapterPublication::Published { .. })
    }

    /// Exact current `cStreamFile +0x28` raw state.
    #[must_use]
    pub const fn stream_file_state(&self) -> StreamFileState {
        StreamFileState::from_raw(self.inner.state().raw())
    }

    #[must_use]
    pub const fn stream_file_identity(&self) -> &Arc<S> {
        &self.stream_file_identity
    }

    #[must_use]
    pub fn processed_octree_records(&self) -> Option<&[ProcessedOctreeInfo]> {
        self.processed_octree_storage
            .as_deref()
            .map(ProcessedOctreeStorage::accepted_prefix)
    }

    #[must_use]
    pub const fn processed_octree_storage(&self) -> Option<&Arc<ProcessedOctreeStorage>> {
        self.processed_octree_storage.as_ref()
    }

    #[must_use]
    pub const fn pending_local_spatial_index_identity(&self) -> Option<&L> {
        match &self.publication {
            AdapterPublication::Preparing {
                local_spatial_index,
            } => local_spatial_index.as_ref(),
            AdapterPublication::Published { .. } | AdapterPublication::Unpublished => None,
        }
    }
}

/// Safe native adapter for `UpdateStreamFileLoaded` at `0x824e1ad0`.
pub struct WorldStreamAdapter<Inner, Publisher> {
    inner: Inner,
    publisher: Publisher,
}

impl<I, P> WorldStreamAdapter<I, P> {
    #[must_use]
    pub const fn new(inner: I, publisher: P) -> Self {
        Self { inner, publisher }
    }

    #[must_use]
    pub fn into_parts(self) -> (I, P) {
        (self.inner, self.publisher)
    }
}

/// Exhaustive mapping of `AssetPaths::tStreamType` values 0, 1, and 2.
#[must_use]
pub const fn stream_kind(stream_type: WorldStreamType) -> StreamKind {
    match stream_type {
        WorldStreamType::Presentation => StreamKind::Presentation,
        WorldStreamType::Simulation => StreamKind::Simulation,
        WorldStreamType::Texture => StreamKind::Texture,
    }
}

/// Native unsigned-byte C-string path comparison over byte-preserving storage.
#[must_use]
pub fn compare_native_stream_file_paths(left: &NativeCPath, right: &NativeCPath) -> Ordering {
    left.c_string_bytes().cmp(right.c_string_bytes())
}

/// Returns the native stable insertion point: before the first greater path
/// and after every existing equal path.
#[must_use]
pub fn native_stream_file_insert_index<'a>(
    existing: impl IntoIterator<Item = &'a NativeCPath>,
    candidate: &NativeCPath,
) -> usize {
    let mut count = 0;
    for path in existing {
        if compare_native_stream_file_paths(candidate, path) == Ordering::Less {
            return count;
        }
        count += 1;
    }
    count
}

impl<I, P> WorldStreamBackend for WorldStreamAdapter<I, P>
where
    I: InnerStreamBackend,
    P: LoadedStreamPublisher,
{
    type StreamFile = AdaptedStreamFile<
        I::Handle,
        I::Error,
        P::Error,
        P::StreamFileIdentity,
        P::LocalSpatialIndexIdentity,
        P::PublicationToken,
    >;
    type Error = SharedAdapterError<I::Error, P::Error>;

    fn begin_stream_file(
        &mut self,
        stream_path: &str,
        basename: &str,
        stream_type: WorldStreamType,
    ) -> Result<Self::StreamFile, Self::Error> {
        let kind = stream_kind(stream_type);
        let inner: StreamFile<I::Handle, AdapterBackendError<I::Error, P::Error>> =
            StreamFile::load(stream_path, basename, kind)
                .map_err(|error| Arc::new(WorldStreamAdapterError::Path(error)))?;
        let native_file_path =
            NativeCPath::from_bytes(Arc::<[u8]>::from(inner.paths().manifest_stem.as_bytes()));
        let stream_file_identity = self
            .publisher
            .retain_stream_file_identity(inner.registration_context(), &native_file_path)
            .map_err(|source| {
                Arc::new(WorldStreamAdapterError::Publication {
                    kind,
                    phase: PublicationPhase::RetainStreamFileIdentity,
                    source,
                })
            })?;
        Ok(AdaptedStreamFile {
            kind,
            basename: basename.to_owned(),
            native_file_path,
            inner,
            stream_file_identity: Arc::new(stream_file_identity),
            processed_octree_storage: None,
            publication: AdapterPublication::Preparing {
                local_spatial_index: None,
            },
            terminal_error: None,
        })
    }

    fn poll_stream_file(
        &mut self,
        stream_file: &mut Self::StreamFile,
    ) -> Result<StreamFileState, Self::Error> {
        self.with_file_latch(stream_file, |adapter, file| {
            adapter.poll_inner_and_publish(file)
        })
    }

    fn begin_stream_xml(
        &mut self,
        stream_file: &mut Self::StreamFile,
        basename: &str,
    ) -> Result<(), Self::Error> {
        self.with_file_latch(stream_file, |adapter, file| {
            if basename != file.basename {
                return Err(Arc::new(WorldStreamAdapterError::BasenameMismatch {
                    retained: file.basename.clone(),
                    supplied: basename.to_owned(),
                }));
            }
            adapter.begin_inner_stream_xml(file)
        })
    }

    fn stream_xml_state(&mut self, stream_file: &mut Self::StreamFile) -> StreamXmlState {
        StreamXmlState::from_raw(stream_file.inner.xml_state().raw())
    }

    fn update_stream_xml(&mut self, stream_file: &mut Self::StreamFile) -> Result<(), Self::Error> {
        self.with_file_latch(stream_file, |adapter, file| {
            adapter.poll_inner_and_publish(file).map(|_| ())
        })
    }

    fn texture_stream_enabled(&mut self) -> Result<bool, Self::Error> {
        self.publisher
            .texture_stream_enabled()
            .map_err(|source| Arc::new(WorldStreamAdapterError::TextureGate(source)))
    }

    fn finalize_world_bounds(&mut self) {
        self.publisher.finalize_world_bounds();
    }
}

impl<I, P> WorldStreamAdapter<I, P>
where
    I: InnerStreamBackend,
    P: LoadedStreamPublisher,
{
    /// Requests `cStreamFile::Unload` (`0x824d7838`) without changing the
    /// load-only [`WorldStreamBackend`] surface. A rejected request does not
    /// poison later polling; poll-time failures retain their existing latch.
    ///
    /// # Errors
    ///
    /// Returns the already-latched terminal adapter error, or the typed inner
    /// request rejection while leaving the file state unchanged.
    pub fn request_stream_file_unload(
        &mut self,
        stream_file: &mut AdapterFile<I, P>,
    ) -> Result<(), WorldStreamUnloadRequestError<I::Error, P::Error>> {
        if let Some(error) = stream_file.terminal_error.as_ref() {
            return Err(WorldStreamUnloadRequestError::Terminal(Arc::clone(error)));
        }
        stream_file
            .inner
            .request_unload()
            .map_err(WorldStreamUnloadRequestError::Request)
    }

    fn with_file_latch<T>(
        &mut self,
        stream_file: &mut AdapterFile<I, P>,
        operation: impl FnOnce(
            &mut Self,
            &mut AdapterFile<I, P>,
        ) -> Result<T, SharedAdapterError<I::Error, P::Error>>,
    ) -> Result<T, SharedAdapterError<I::Error, P::Error>> {
        if let Some(error) = stream_file.terminal_error.as_ref() {
            return Err(Arc::clone(error));
        }
        let result = operation(self, stream_file);
        if let Err(error) = &result {
            stream_file.terminal_error = Some(Arc::clone(error));
        }
        result
    }

    fn poll_inner_and_publish(
        &mut self,
        stream_file: &mut AdapterFile<I, P>,
    ) -> Result<StreamFileState, SharedAdapterError<I::Error, P::Error>> {
        let mut processing_error = None;
        {
            let mut backend = ProcessingBackend {
                inner: &mut self.inner,
                publisher: &mut self.publisher,
                processed: &mut stream_file.processed_octree_storage,
                publication: &mut stream_file.publication,
                processing_error: &mut processing_error,
            };
            stream_file
                .inner
                .poll(&mut backend)
                .map_err(Self::lift_inner_error)?;
        }
        if let Some(error) = processing_error {
            return Err(error);
        }

        if let Some(stage) = stream_file.inner.reference_read_failure() {
            let stream_file_state = stream_file.inner.state().raw();
            let game_stream_state = stream_file.inner.game_stream_state().raw();
            let stall = match stage {
                ReadStage::CollectionMap => ReferenceStreamStall::CollectionMap {
                    stream_file_state,
                    game_stream_state,
                },
                ReadStage::AssetToc => ReferenceStreamStall::AssetToc {
                    stream_file_state,
                    game_stream_state,
                },
                ReadStage::CollectionSpace => ReferenceStreamStall::CollectionSpace {
                    stream_file_state,
                    game_stream_state,
                },
                ReadStage::StreamXml => unreachable!("XML failure is native completion"),
            };
            return Err(Arc::new(WorldStreamAdapterError::ReferenceStall {
                kind: stream_file.kind,
                stall,
            }));
        }

        self.publish_if_loaded(stream_file)?;
        self.unpublish_if_done(stream_file)?;

        Ok(StreamFileState::from_raw(stream_file.inner.state().raw()))
    }

    fn publish_if_loaded(
        &mut self,
        stream_file: &mut AdapterFile<I, P>,
    ) -> Result<(), SharedAdapterError<I::Error, P::Error>> {
        if stream_file.inner.state() != crate::game_stream::StreamFileState::LOAD_DONE
            || !matches!(
                stream_file.publication,
                AdapterPublication::Preparing { .. }
            )
        {
            return Ok(());
        }
        let records = stream_file
            .processed_octree_storage
            .as_ref()
            .ok_or_else(|| {
                Arc::new(WorldStreamAdapterError::MissingProcessedOctreeRecords {
                    kind: stream_file.kind,
                })
            })?;
        let local_spatial_index = match &mut stream_file.publication {
            AdapterPublication::Preparing {
                local_spatial_index,
            } => local_spatial_index.take().ok_or_else(|| {
                Arc::new(WorldStreamAdapterError::MissingLocalSpatialIndex {
                    kind: stream_file.kind,
                })
            })?,
            AdapterPublication::Published { .. } | AdapterPublication::Unpublished => {
                unreachable!("the exact preparing gate was checked above")
            }
        };
        let token = self
            .publisher
            .publish_loaded_stream_transactionally(
                stream_file.kind,
                &stream_file.stream_file_identity,
                local_spatial_index,
                &stream_file.native_file_path,
                records,
            )
            .map_err(|source| {
                Arc::new(WorldStreamAdapterError::Publication {
                    kind: stream_file.kind,
                    phase: PublicationPhase::PublishLoadedStream,
                    source,
                })
            })?;
        stream_file.publication = AdapterPublication::Published {
            token,
            local_release: AdapterLocalRelease::Retained,
        };
        Ok(())
    }

    fn unpublish_if_done(
        &mut self,
        stream_file: &mut AdapterFile<I, P>,
    ) -> Result<(), SharedAdapterError<I::Error, P::Error>> {
        if stream_file.inner.state() != crate::game_stream::StreamFileState::UNLOAD_DONE {
            return Ok(());
        }
        let was_published = match &stream_file.publication {
            AdapterPublication::Published {
                token,
                local_release: AdapterLocalRelease::Released,
            } => self
                .publisher
                .unpublish_loaded_stream(token)
                .map_err(|source| {
                    Arc::new(WorldStreamAdapterError::Publication {
                        kind: stream_file.kind,
                        phase: PublicationPhase::UnpublishLoadedStream,
                        source,
                    })
                })
                .map(|()| true)?,
            AdapterPublication::Preparing {
                local_spatial_index: None,
            } => false,
            AdapterPublication::Unpublished => return Ok(()),
            AdapterPublication::Preparing { .. } | AdapterPublication::Published { .. } => {
                return Err(Arc::new(
                    WorldStreamAdapterError::PublicationLifecycleMismatch {
                        kind: stream_file.kind,
                        stream_file_state: 6,
                        stage: stream_file.publication.stage(),
                    },
                ));
            }
        };
        stream_file.publication = AdapterPublication::Unpublished;
        if was_published {
            // Native global removal precedes scalar destruction of the file
            // and its processed vector. The Rust file identity lives until its
            // outer owner drops it; processed storage does not need to remain.
            stream_file.processed_octree_storage = None;
        }
        Ok(())
    }

    fn begin_inner_stream_xml(
        &mut self,
        stream_file: &mut AdapterFile<I, P>,
    ) -> Result<(), SharedAdapterError<I::Error, P::Error>> {
        let mut processing_error = None;
        let result = {
            let mut backend = ProcessingBackend {
                inner: &mut self.inner,
                publisher: &mut self.publisher,
                processed: &mut stream_file.processed_octree_storage,
                publication: &mut stream_file.publication,
                processing_error: &mut processing_error,
            };
            stream_file.inner.begin_stream_xml(&mut backend)
        };
        result.map_err(Self::lift_inner_error)?;
        debug_assert!(processing_error.is_none());
        Ok(())
    }

    fn lift_inner_error(
        error: Arc<InnerStreamError<AdapterBackendError<I::Error, P::Error>>>,
    ) -> SharedAdapterError<I::Error, P::Error> {
        if let InnerStreamError::Backend(AdapterBackendError::Adapter(error)) = error.as_ref() {
            Arc::clone(error)
        } else {
            Arc::new(WorldStreamAdapterError::Inner(error))
        }
    }
}

struct ProcessingBackend<'a, I, P>
where
    I: InnerStreamBackend,
    P: LoadedStreamPublisher,
{
    inner: &'a mut I,
    publisher: &'a mut P,
    processed: &'a mut Option<Arc<ProcessedOctreeStorage>>,
    publication: &'a mut AdapterPublication<P::LocalSpatialIndexIdentity, P::PublicationToken>,
    processing_error: &'a mut Option<SharedAdapterError<I::Error, P::Error>>,
}

impl<I, P> InnerStreamBackend for ProcessingBackend<'_, I, P>
where
    I: InnerStreamBackend,
    P: LoadedStreamPublisher,
{
    type Handle = I::Handle;
    type Error = AdapterBackendError<I::Error, P::Error>;

    fn begin_read(
        &mut self,
        request: crate::game_stream::AsyncReadRequest,
    ) -> Result<Self::Handle, Self::Error> {
        self.inner
            .begin_read(request)
            .map_err(AdapterBackendError::Inner)
    }

    fn poll_read(
        &mut self,
        handle: &mut Self::Handle,
    ) -> Result<crate::game_stream::AsyncReadPoll, Self::Error> {
        self.inner
            .poll_read(handle)
            .map_err(AdapterBackendError::Inner)
    }

    fn collection_map_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        map: &skate2_formats::stream::CollectionMap,
    ) -> Result<(), Self::Error> {
        self.inner
            .collection_map_loaded(context, map)
            .map_err(AdapterBackendError::Inner)
    }

    fn asset_toc_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        toc: &skate2_formats::stream::AssetToc,
    ) -> Result<(), Self::Error> {
        self.inner
            .asset_toc_loaded(context, toc)
            .map_err(AdapterBackendError::Inner)
    }

    #[allow(clippy::too_many_lines)] // Keep the recovered per-record call order auditable.
    fn collection_space_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        space: &skate2_formats::stream::CollectionSpace,
    ) -> Result<(), Self::Error> {
        let kind = context.kind();
        let Ok(raw_logical_len) = u32::try_from(space.collections.len()) else {
            *self.processing_error =
                Some(Arc::new(WorldStreamAdapterError::ProcessedIndexOverflow {
                    index: space.collections.len(),
                }));
            return Ok(());
        };
        let mut processed = Vec::new();
        for (source_index, source) in space.collections.iter().enumerate() {
            if source.lod != 0 {
                continue;
            }
            let already_valid = match self.publisher.is_collection_valid(source.id) {
                Ok(value) => value,
                Err(source) => {
                    *self.processing_error = Some(Arc::new(WorldStreamAdapterError::Publication {
                        kind,
                        phase: PublicationPhase::CheckCollectionValidity { source_index },
                        source,
                    }));
                    return Ok(());
                }
            };
            if already_valid {
                continue;
            }
            let offset = match self.publisher.stream_offset(source.id) {
                Ok(value) => value,
                Err(source) => {
                    *self.processing_error = Some(Arc::new(WorldStreamAdapterError::Publication {
                        kind,
                        phase: PublicationPhase::GetStreamOffset { source_index },
                        source,
                    }));
                    return Ok(());
                }
            };
            let ticks_size = match self.publisher.stream_size(source.id) {
                Ok(value) => value,
                Err(source) => {
                    *self.processing_error = Some(Arc::new(WorldStreamAdapterError::Publication {
                        kind,
                        phase: PublicationPhase::GetTicksStreamSize { source_index },
                        source,
                    }));
                    return Ok(());
                }
            };
            let size = match self.publisher.stream_size(source.id) {
                Ok(value) => value,
                Err(source) => {
                    *self.processing_error = Some(Arc::new(WorldStreamAdapterError::Publication {
                        kind,
                        phase: PublicationPhase::GetRawStreamSize { source_index },
                        source,
                    }));
                    return Ok(());
                }
            };
            let Ok(bbox_index) = i32::try_from(processed.len()) else {
                *self.processing_error =
                    Some(Arc::new(WorldStreamAdapterError::ProcessedIndexOverflow {
                        index: processed.len(),
                    }));
                return Ok(());
            };
            let spatial = match processed_position_and_order(source) {
                Ok(value) => value,
                Err(error) => {
                    *self.processing_error = Some(Arc::new(error));
                    return Ok(());
                }
            };
            processed.push(ProcessedOctreeInfo::new(ProcessedOctreeMetadata {
                bbox_min_words: source_vec3_words(source.hull_min),
                bbox_max_words: source_vec3_words(source.hull_max),
                geometry_min_words: source_vec3_words(source.geometry_min),
                geometry_max_words: source_vec3_words(source.geometry_max),
                id: source.id,
                offset,
                position_words: spatial.position.map(f32::to_bits),
                stream_order: spatial.stream_order,
                bbox_index,
                ticks: i32::from_ne_bytes((ticks_size / 133_333).to_ne_bytes()),
                size: i32::from_ne_bytes(size.to_ne_bytes()),
                stream_type: kind,
            }));
            if kind == StreamKind::Presentation {
                // This is deliberately immediate, matching the native call
                // after mCount advances. If a later record fails, recovery or
                // discard of this proxy-world side effect remains unknown; it
                // is not falsely included in the global publication transaction.
                self.publisher.add_high_collection(
                    source.id,
                    u32::from_ne_bytes(spatial.center_x.to_ne_bytes()),
                    u32::from_ne_bytes(spatial.center_z.to_ne_bytes()),
                );
            }
        }
        let accepted_prefix: Arc<[ProcessedOctreeInfo]> = Arc::from(processed);
        let records = match ProcessedOctreeStorage::new(raw_logical_len, accepted_prefix) {
            Ok(records) => Arc::new(records),
            Err(error) => {
                *self.processing_error =
                    Some(Arc::new(WorldStreamAdapterError::ProcessedStorage(error)));
                return Ok(());
            }
        };
        let bounds = records.spatial_bounds();
        *self.processed = Some(Arc::clone(&records));
        let local_spatial_index =
            match self
                .publisher
                .construct_local_spatial_index(kind, raw_logical_len, bounds)
            {
                Ok(identity) => identity,
                Err(source) => {
                    *self.processing_error = Some(Arc::new(WorldStreamAdapterError::Publication {
                        kind,
                        phase: PublicationPhase::ConstructLocalSpatialIndex,
                        source,
                    }));
                    return Ok(());
                }
            };
        // Native stores the initialized pointer at +0x48 (0x824e1368..136c)
        // before the compact Insert loop. The safe identity likewise remains
        // uniquely owned here until the raw-4 publication transaction moves it.
        let local_slot = match self.publication {
            AdapterPublication::Preparing {
                local_spatial_index,
            } => local_spatial_index,
            AdapterPublication::Published { .. } | AdapterPublication::Unpublished => {
                unreachable!("collection-space construction precedes publication")
            }
        };
        *local_slot = Some(local_spatial_index);
        let local_spatial_index = local_slot
            .as_mut()
            .expect("local spatial index was just stored");
        for (compact_index, record) in records.accepted_prefix().iter().enumerate() {
            let compact_index = u32::try_from(compact_index)
                .expect("accepted prefix cannot exceed raw u32 logical length");
            if let Err(source) = self.publisher.insert_local_spatial_record(
                local_spatial_index,
                compact_index,
                record.metadata.bbox_min_words,
                record.metadata.bbox_max_words,
            ) {
                *self.processing_error = Some(Arc::new(WorldStreamAdapterError::Publication {
                    kind,
                    phase: PublicationPhase::InsertLocalSpatialRecord { compact_index },
                    source,
                }));
                return Ok(());
            }
        }
        self.inner
            .collection_space_loaded(context, space)
            .map_err(AdapterBackendError::Inner)?;
        Ok(())
    }

    fn stream_xml_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        bytes: &[u8],
    ) -> Result<(), Self::Error> {
        self.inner
            .stream_xml_loaded(context, bytes)
            .map_err(AdapterBackendError::Inner)
    }

    fn collection_map_unloaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        map: &skate2_formats::stream::CollectionMap,
    ) -> Result<(), Self::Error> {
        self.inner
            .collection_map_unloaded(context, map)
            .map_err(AdapterBackendError::Inner)
    }

    fn asset_toc_unloaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        toc: &skate2_formats::stream::AssetToc,
    ) -> Result<(), Self::Error> {
        self.inner
            .asset_toc_unloaded(context, toc)
            .map_err(AdapterBackendError::Inner)
    }

    fn stream_file_unloaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        space: Option<&skate2_formats::stream::CollectionSpace>,
        had_stream_xml: bool,
    ) -> Result<(), Self::Error> {
        match self.publication {
            AdapterPublication::Published {
                token,
                local_release: local_release @ AdapterLocalRelease::Retained,
            } => {
                if let Err(source) = self.publisher.release_local_spatial(token) {
                    *local_release = AdapterLocalRelease::ReleaseAttemptFailed;
                    return Err(AdapterBackendError::Adapter(Arc::new(
                        WorldStreamAdapterError::Publication {
                            kind: context.kind(),
                            phase: PublicationPhase::ReleaseLocalSpatial,
                            source,
                        },
                    )));
                }
                *local_release = AdapterLocalRelease::Released;
            }
            AdapterPublication::Published {
                local_release: AdapterLocalRelease::Released,
                ..
            }
            | AdapterPublication::Preparing {
                local_spatial_index: None,
            } => {}
            AdapterPublication::Preparing { .. }
            | AdapterPublication::Published { .. }
            | AdapterPublication::Unpublished => {
                return Err(AdapterBackendError::Adapter(Arc::new(
                    WorldStreamAdapterError::PublicationLifecycleMismatch {
                        kind: context.kind(),
                        stream_file_state: 5,
                        stage: self.publication.stage(),
                    },
                )));
            }
        }
        self.inner
            .stream_file_unloaded(context, space, had_stream_xml)
            .map_err(AdapterBackendError::Inner)
    }
}

fn source_vec3_words(source: [f32; 4]) -> [u32; 4] {
    [
        source[0].to_bits(),
        source[1].to_bits(),
        source[2].to_bits(),
        1.0f32.to_bits(),
    ]
}

fn local_spatial_index_bounds(records: &[ProcessedOctreeInfo]) -> LocalSpatialIndexBounds {
    let (mut min, mut max) = if let Some(first) = records.first() {
        (first.metadata.bbox_min_words, first.metadata.bbox_max_words)
    } else {
        (
            [
                (-1.0f32).to_bits(),
                (-1.0f32).to_bits(),
                (-1.0f32).to_bits(),
                0,
            ],
            [1.0f32.to_bits(), 1.0f32.to_bits(), 1.0f32.to_bits(), 0],
        )
    };

    for record in records.iter().skip(1) {
        for lane in 0..4 {
            let current_min = f32::from_bits(min[lane]);
            let candidate_min = f32::from_bits(record.metadata.bbox_min_words[lane]);
            let current_max = f32::from_bits(max[lane]);
            let candidate_max = f32::from_bits(record.metadata.bbox_max_words[lane]);
            min[lane] = vmx_min_ordinary(current_min, candidate_min).to_bits();
            max[lane] = vmx_max_ordinary(current_max, candidate_max).to_bits();
        }
    }

    let aggregate = LocalSpatialBoundsWords { min, max };
    for (lane, expansion_word) in COLLECTION_BOX_EXPANSION_WORDS.into_iter().enumerate() {
        let expansion = f32::from_bits(expansion_word);
        min[lane] = (f32::from_bits(min[lane]) - expansion).to_bits();
        max[lane] = (f32::from_bits(max[lane]) + expansion).to_bits();
    }
    LocalSpatialIndexBounds {
        aggregate,
        expanded: LocalSpatialBoundsWords { min, max },
    }
}

// `vminfp`/`vmaxfp` are exact evidence for the aggregate operation. These
// scalar expressions preserve ordinary finite comparisons; exceptional NaN
// and signed-zero payload/tie parity remains outside the current proof.
pub(crate) fn vmx_min_ordinary(left: f32, right: f32) -> f32 {
    if left < right { left } else { right }
}

pub(crate) fn vmx_max_ordinary(left: f32, right: f32) -> f32 {
    if left > right { left } else { right }
}

struct ProcessedSpatial {
    position: [f32; 4],
    center_x: i32,
    center_z: i32,
    stream_order: i32,
}

// The VMX midpoint and scalar libm-floor sequence is proven. This native scalar
// expression is high-confidence for ordinary finite corpus values; exact
// exceptional payload and conversion behavior is not claimed.
fn processed_position_and_order<I, P>(
    source: &CollectionSpaceEntry,
) -> Result<ProcessedSpatial, WorldStreamAdapterError<I, P>> {
    let position = [
        (source.hull_max[0] - source.hull_min[0]).mul_add(0.5, source.hull_min[0]),
        (source.hull_max[1] - source.hull_min[1]).mul_add(0.5, source.hull_min[1]),
        (source.hull_max[2] - source.hull_min[2]).mul_add(0.5, source.hull_min[2]),
        1.0,
    ];
    let center_x = native_floor_coordinate::<I, P>(source.id, 0, position[0] + 0.5)?;
    let center_z = native_floor_coordinate::<I, P>(source.id, 2, position[2] + 0.5)?;
    let stream_order = center_x.wrapping_mul(10_000).wrapping_add(center_z / 10);
    Ok(ProcessedSpatial {
        position,
        center_x,
        center_z,
        stream_order,
    })
}

#[allow(clippy::cast_possible_truncation)] // Explicit bounds make this the native truncation.
fn native_floor_coordinate<I, P>(
    id: u64,
    axis: usize,
    value: f32,
) -> Result<i32, WorldStreamAdapterError<I, P>> {
    let floored = value.floor();
    if !floored.is_finite() || floored < -2_147_483_648.0 || floored >= 2_147_483_648.0 {
        return Err(WorldStreamAdapterError::UnsupportedSpatialCoordinate {
            id,
            axis,
            bits: value.to_bits(),
        });
    }
    Ok(floored as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::{WorldStreamLoads, WorldStreamOuterState, WorldStreamSpec},
        game_stream::{AsyncReadPoll, AsyncReadRequest},
    };
    use skate2_formats::stream::{AssetToc, CollectionMap, CollectionSpace};
    use std::{
        cell::{Cell, RefCell},
        collections::{HashMap, VecDeque},
        rc::Rc,
    };

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        Begin(ReadStage),
        Poll(ReadStage),
        Retain(StreamKind, u32, Vec<u8>),
        CheckValid(u64),
        StreamOffset(u64),
        StreamSize(u64),
        AddHigh(u64, u32, u32),
        LocalDescriptor(StreamKind, u32, LocalSpatialIndexBounds),
        LocalAllocate(u32),
        LocalInitializeAttempt(u32, u32, LocalSpatialIndexBounds),
        InsertIntoStoredLocal(u32, u32, [u32; 4], [u32; 4]),
        LocalSpaceBuilt(usize),
        PublishOctree(u32, usize, usize, u64),
        PublishSorted(StreamKind, u32, Vec<u8>),
        MapUnloaded(StreamKind),
        TocUnloaded(StreamKind),
        ReleaseLocal(u32),
        StreamFileUnloaded(StreamKind, bool, bool),
        Unpublish(StreamKind, u32),
        TextureGate,
        Finalize,
        ObservedPublicLoadedStateFour,
    }

    #[derive(Debug)]
    struct TestStreamFileIdentity {
        serial: u32,
    }

    #[derive(Debug)]
    struct TestLocalSpatialIndexIdentity {
        serial: u32,
        raw_logical_len: u32,
        bounds: LocalSpatialIndexBounds,
        insertions: Vec<(u32, [u32; 4], [u32; 4])>,
        drop_count: Option<Rc<Cell<usize>>>,
    }

    impl Drop for TestLocalSpatialIndexIdentity {
        fn drop(&mut self) {
            if let Some(drop_count) = self.drop_count.as_ref() {
                drop_count.set(drop_count.get() + 1);
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestPublicationToken(usize);

    struct TestPublication {
        kind: StreamKind,
        file: Arc<TestStreamFileIdentity>,
        records: Arc<ProcessedOctreeStorage>,
        local: Option<TestLocalSpatialIndexIdentity>,
    }

    struct TestHandle(ReadStage);

    struct TestInner {
        plans: [VecDeque<AsyncReadPoll>; 4],
        events: Rc<RefCell<Vec<Event>>>,
        fail_begin: bool,
        fail_final_unload: bool,
    }

    impl TestInner {
        fn new(events: Rc<RefCell<Vec<Event>>>) -> Self {
            Self {
                plans: std::array::from_fn(|_| VecDeque::new()),
                events,
                fail_begin: false,
                fail_final_unload: false,
            }
        }

        fn plan(&mut self, stage: ReadStage, polls: impl IntoIterator<Item = AsyncReadPoll>) {
            self.plans[stage_slot(stage)] = polls.into_iter().collect();
        }
    }

    impl InnerStreamBackend for TestInner {
        type Handle = TestHandle;
        type Error = TestError;

        fn begin_read(&mut self, request: AsyncReadRequest) -> Result<Self::Handle, Self::Error> {
            self.events.borrow_mut().push(Event::Begin(request.stage));
            if self.fail_begin {
                Err(TestError("begin"))
            } else {
                Ok(TestHandle(request.stage))
            }
        }

        fn poll_read(&mut self, handle: &mut Self::Handle) -> Result<AsyncReadPoll, Self::Error> {
            self.events.borrow_mut().push(Event::Poll(handle.0));
            Ok(self.plans[stage_slot(handle.0)]
                .pop_front()
                .unwrap_or(AsyncReadPoll::Pending))
        }

        fn collection_map_loaded(
            &mut self,
            _context: StreamRegistrationContext<'_>,
            _map: &CollectionMap,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn asset_toc_loaded(
            &mut self,
            _context: StreamRegistrationContext<'_>,
            _toc: &AssetToc,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn collection_space_loaded(
            &mut self,
            _context: StreamRegistrationContext<'_>,
            space: &CollectionSpace,
        ) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(Event::LocalSpaceBuilt(space.collections.len()));
            Ok(())
        }

        fn stream_xml_loaded(
            &mut self,
            _context: StreamRegistrationContext<'_>,
            _bytes: &[u8],
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn collection_map_unloaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            _map: &CollectionMap,
        ) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(Event::MapUnloaded(context.kind()));
            Ok(())
        }

        fn asset_toc_unloaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            _toc: &AssetToc,
        ) -> Result<(), Self::Error> {
            self.events
                .borrow_mut()
                .push(Event::TocUnloaded(context.kind()));
            Ok(())
        }

        fn stream_file_unloaded(
            &mut self,
            context: StreamRegistrationContext<'_>,
            space: Option<&CollectionSpace>,
            had_stream_xml: bool,
        ) -> Result<(), Self::Error> {
            self.events.borrow_mut().push(Event::StreamFileUnloaded(
                context.kind(),
                space.is_some(),
                had_stream_xml,
            ));
            if self.fail_final_unload {
                Err(TestError("final unload"))
            } else {
                Ok(())
            }
        }
    }

    struct TestPublisher {
        events: Rc<RefCell<Vec<Event>>>,
        fail_phase: Option<PublicationPhase>,
        valid_ids: Vec<u64>,
        fail_validity_id: Option<u64>,
        fail_metadata_call: Option<usize>,
        raw_size_override: Option<u32>,
        metadata_calls: usize,
        next_file_serial: u32,
        next_local_serial: u32,
        next_local_drop_count: Option<Rc<Cell<usize>>>,
        publish_attempts: usize,
        stream_files: [Vec<(NativeCPath, u32)>; 3],
        octree_map: HashMap<u64, (u32, usize)>,
        publications: Vec<Option<TestPublication>>,
        retained_registration_ids: Vec<crate::game_stream::RegistrationFileId>,
    }

    impl LoadedStreamPublisher for TestPublisher {
        type Error = TestError;
        type StreamFileIdentity = TestStreamFileIdentity;
        type LocalSpatialIndexIdentity = TestLocalSpatialIndexIdentity;
        type PublicationToken = TestPublicationToken;

        fn retain_stream_file_identity(
            &mut self,
            context: StreamRegistrationContext<'_>,
            file_path: &NativeCPath,
        ) -> Result<Self::StreamFileIdentity, Self::Error> {
            if self.fail_phase == Some(PublicationPhase::RetainStreamFileIdentity) {
                return Err(TestError("stream-file identity"));
            }
            let serial = self.next_file_serial;
            self.next_file_serial += 1;
            self.retained_registration_ids.push(context.id().clone());
            self.events.borrow_mut().push(Event::Retain(
                context.kind(),
                serial,
                file_path.stored_bytes().to_vec(),
            ));
            Ok(TestStreamFileIdentity { serial })
        }

        fn is_collection_valid(&mut self, id: u64) -> Result<bool, Self::Error> {
            self.events.borrow_mut().push(Event::CheckValid(id));
            if self.fail_validity_id == Some(id) {
                return Err(TestError("validity"));
            }
            Ok(self.valid_ids.contains(&id) || self.octree_map.contains_key(&id))
        }

        fn stream_offset(&mut self, id: u64) -> Result<u64, Self::Error> {
            self.events.borrow_mut().push(Event::StreamOffset(id));
            self.metadata_calls += 1;
            if self.fail_metadata_call == Some(self.metadata_calls) {
                Err(TestError("offset"))
            } else {
                Ok(0x1000 + id)
            }
        }

        fn stream_size(&mut self, id: u64) -> Result<u32, Self::Error> {
            self.events.borrow_mut().push(Event::StreamSize(id));
            self.metadata_calls += 1;
            if self.fail_metadata_call == Some(self.metadata_calls) {
                Err(TestError("size"))
            } else if self.metadata_calls % 3 == 2 {
                Ok(266_666)
            } else {
                Ok(self
                    .raw_size_override
                    .unwrap_or(0x5000 + u32::try_from(id).unwrap()))
            }
        }

        fn add_high_collection(&mut self, id: u64, x: u32, z: u32) {
            self.events.borrow_mut().push(Event::AddHigh(id, x, z));
        }

        fn construct_local_spatial_index(
            &mut self,
            kind: StreamKind,
            raw_logical_len: u32,
            bounds: LocalSpatialIndexBounds,
        ) -> Result<Self::LocalSpatialIndexIdentity, Self::Error> {
            let serial = self.next_local_serial;
            self.next_local_serial += 1;
            self.events.borrow_mut().extend([
                Event::LocalDescriptor(kind, raw_logical_len, bounds),
                Event::LocalAllocate(serial),
                Event::LocalInitializeAttempt(serial, raw_logical_len, bounds),
            ]);
            if self.fail_phase == Some(PublicationPhase::ConstructLocalSpatialIndex) {
                return Err(TestError("local spatial index"));
            }
            Ok(TestLocalSpatialIndexIdentity {
                serial,
                raw_logical_len,
                bounds,
                insertions: Vec::new(),
                drop_count: self.next_local_drop_count.clone(),
            })
        }

        fn insert_local_spatial_record(
            &mut self,
            local_spatial_index: &mut Self::LocalSpatialIndexIdentity,
            compact_index: u32,
            bbox_min_words: [u32; 4],
            bbox_max_words: [u32; 4],
        ) -> Result<(), Self::Error> {
            let phase = PublicationPhase::InsertLocalSpatialRecord { compact_index };
            if self.fail_phase == Some(phase) {
                return Err(TestError("local spatial insert"));
            }
            self.events.borrow_mut().push(Event::InsertIntoStoredLocal(
                local_spatial_index.serial,
                compact_index,
                bbox_min_words,
                bbox_max_words,
            ));
            local_spatial_index
                .insertions
                .push((compact_index, bbox_min_words, bbox_max_words));
            Ok(())
        }

        fn publish_loaded_stream_transactionally(
            &mut self,
            kind: StreamKind,
            stream_file: &Arc<Self::StreamFileIdentity>,
            local_spatial_index: Self::LocalSpatialIndexIdentity,
            file_path: &NativeCPath,
            records: &Arc<ProcessedOctreeStorage>,
        ) -> Result<Self::PublicationToken, Self::Error> {
            // Contract-checking test double only; this is not presented as a
            // reusable transactional container implementation.
            self.publish_attempts += 1;
            if self.fail_phase == Some(PublicationPhase::PublishLoadedStream) {
                return Err(TestError("loaded-stream transaction"));
            }
            for (processed_index, record) in records.accepted_prefix().iter().enumerate() {
                self.events.borrow_mut().push(Event::PublishOctree(
                    stream_file.serial,
                    processed_index,
                    usize::try_from(record.metadata.bbox_index).unwrap(),
                    record.metadata.id,
                ));
                self.octree_map
                    .entry(record.metadata.id)
                    .or_insert((stream_file.serial, processed_index));
            }
            let slot = kind as usize;
            let index = native_stream_file_insert_index(
                self.stream_files[slot].iter().map(|(path, _)| path),
                file_path,
            );
            self.stream_files[slot].insert(index, (file_path.clone(), stream_file.serial));
            self.events.borrow_mut().push(Event::PublishSorted(
                kind,
                stream_file.serial,
                file_path.stored_bytes().to_vec(),
            ));
            let token = TestPublicationToken(self.publications.len());
            self.publications.push(Some(TestPublication {
                kind,
                file: Arc::clone(stream_file),
                records: Arc::clone(records),
                local: Some(local_spatial_index),
            }));
            Ok(token)
        }

        fn release_local_spatial(
            &mut self,
            token: &Self::PublicationToken,
        ) -> Result<(), Self::Error> {
            let publication = self.publications[token.0]
                .as_mut()
                .expect("active publication token");
            let serial = publication
                .local
                .as_ref()
                .expect("local spatial value is released only once")
                .serial;
            self.events.borrow_mut().push(Event::ReleaseLocal(serial));
            if self.fail_phase == Some(PublicationPhase::ReleaseLocalSpatial) {
                return Err(TestError("local spatial release"));
            }
            drop(publication.local.take());
            Ok(())
        }

        fn unpublish_loaded_stream(
            &mut self,
            token: &Self::PublicationToken,
        ) -> Result<(), Self::Error> {
            let publication = self.publications[token.0]
                .as_ref()
                .expect("active publication token");
            assert!(publication.local.is_none());
            let kind = publication.kind;
            let serial = publication.file.serial;
            self.events
                .borrow_mut()
                .push(Event::Unpublish(kind, serial));
            if self.fail_phase == Some(PublicationPhase::UnpublishLoadedStream) {
                return Err(TestError("global unpublish"));
            }
            for record in publication.records.accepted_prefix() {
                if self
                    .octree_map
                    .get(&record.metadata.id)
                    .map(|entry| entry.0)
                    == Some(serial)
                {
                    self.octree_map.remove(&record.metadata.id);
                }
            }
            let slot = kind as usize;
            let index = self.stream_files[slot]
                .iter()
                .position(|(_, candidate)| *candidate == serial)
                .expect("published file identity");
            self.stream_files[slot].remove(index);
            self.publications[token.0] = None;
            Ok(())
        }

        fn texture_stream_enabled(&mut self) -> Result<bool, Self::Error> {
            self.events.borrow_mut().push(Event::TextureGate);
            Ok(true)
        }

        fn finalize_world_bounds(&mut self) {
            self.events.borrow_mut().push(Event::Finalize);
        }
    }

    impl TestPublisher {
        fn publication(&self, index: usize) -> &TestPublication {
            self.publications[index]
                .as_ref()
                .expect("active test publication")
        }

        fn published_local(&self, index: usize) -> &TestLocalSpatialIndexIdentity {
            self.publication(index)
                .local
                .as_ref()
                .expect("test local has not been released")
        }
    }

    fn take_pending_local(
        file: &mut AdaptedStreamFile<
            TestHandle,
            TestError,
            TestError,
            TestStreamFileIdentity,
            TestLocalSpatialIndexIdentity,
            TestPublicationToken,
        >,
    ) -> TestLocalSpatialIndexIdentity {
        match &mut file.publication {
            AdapterPublication::Preparing {
                local_spatial_index,
            } => local_spatial_index.take().expect("constructed local"),
            AdapterPublication::Published { .. } | AdapterPublication::Unpublished => {
                panic!("file must not already be published")
            }
        }
    }

    fn stage_slot(stage: ReadStage) -> usize {
        match stage {
            ReadStage::CollectionMap => 0,
            ReadStage::AssetToc => 1,
            ReadStage::CollectionSpace => 2,
            ReadStage::StreamXml => 3,
        }
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn put_f32(bytes: &mut [u8], offset: usize, value: f32) {
        put_u32(bytes, offset, value.to_bits());
    }

    fn collection_map_bytes() -> Vec<u8> {
        let mut bytes = vec![0; 32 + 104];
        bytes[..4].copy_from_slice(b"CMAP");
        put_u32(&mut bytes, 4, 3);
        put_u64(&mut bytes, 8, 7);
        put_u32(&mut bytes, 16, 1);
        put_u64(&mut bytes, 32, 1);
        put_u64(&mut bytes, 40, u64::MAX);
        bytes
    }

    fn asset_toc_bytes() -> Vec<u8> {
        let mut bytes = vec![0; 24 + 64];
        bytes[..4].copy_from_slice(b"ATOC");
        put_u32(&mut bytes, 4, 3);
        put_u64(&mut bytes, 8, 7);
        put_u32(&mut bytes, 16, 1);
        put_u64(&mut bytes, 24, 2);
        bytes
    }

    fn collection_space_bytes(count: usize) -> Vec<u8> {
        collection_space_entry_bytes(
            &(0..count)
                .map(|index| (0x2000 + index as u64, 0))
                .collect::<Vec<_>>(),
        )
    }

    fn collection_space_entry_bytes(entries: &[(u64, u8)]) -> Vec<u8> {
        let count = entries.len();
        let mut bytes = vec![0; 24 + count * 80];
        bytes[..4].copy_from_slice(b"CSPA");
        put_u32(&mut bytes, 4, 3);
        put_u64(&mut bytes, 8, 7);
        put_u32(&mut bytes, 16, u32::try_from(count).unwrap());
        for (index, (id, lod)) in entries.iter().copied().enumerate() {
            let start = 24 + index * 80;
            put_u64(&mut bytes, start, id);
            bytes[start + 8] = lod;
        }
        bytes
    }

    fn detailed_collection_space_bytes(id: u64) -> Vec<u8> {
        let mut bytes = collection_space_entry_bytes(&[(id, 0)]);
        for (base, values) in [
            (24 + 16, [-1.0, 2.0, 19.0, -99.0]),
            (24 + 32, [3.0, 6.0, 25.0, -98.0]),
            (24 + 48, [-4.0, -5.0, -6.0, -97.0]),
            (24 + 64, [7.0, 8.0, 9.0, -96.0]),
        ] {
            for (lane, value) in values.into_iter().enumerate() {
                put_f32(&mut bytes, base + lane * 4, value);
            }
        }
        bytes
    }

    fn adapter_with_plans(
        space_count: usize,
    ) -> (
        WorldStreamAdapter<TestInner, TestPublisher>,
        Rc<RefCell<Vec<Event>>>,
    ) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut inner = TestInner::new(Rc::clone(&events));
        inner.plan(
            ReadStage::CollectionMap,
            [AsyncReadPoll::Complete(collection_map_bytes())],
        );
        inner.plan(
            ReadStage::AssetToc,
            [AsyncReadPoll::Complete(asset_toc_bytes())],
        );
        inner.plan(
            ReadStage::CollectionSpace,
            [AsyncReadPoll::Complete(collection_space_bytes(space_count))],
        );
        let publisher = TestPublisher {
            events: Rc::clone(&events),
            fail_phase: None,
            valid_ids: Vec::new(),
            fail_validity_id: None,
            fail_metadata_call: None,
            raw_size_override: None,
            metadata_calls: 0,
            next_file_serial: 1,
            next_local_serial: 100,
            next_local_drop_count: None,
            publish_attempts: 0,
            stream_files: std::array::from_fn(|_| Vec::new()),
            octree_map: HashMap::new(),
            publications: Vec::new(),
            retained_registration_ids: Vec::new(),
        };
        (WorldStreamAdapter::new(inner, publisher), events)
    }

    fn begin_presentation(
        adapter: &mut WorldStreamAdapter<TestInner, TestPublisher>,
    ) -> AdaptedStreamFile<
        TestHandle,
        TestError,
        TestError,
        TestStreamFileIdentity,
        TestLocalSpatialIndexIdentity,
        TestPublicationToken,
    > {
        adapter
            .begin_stream_file("root", "BAM", WorldStreamType::Presentation)
            .unwrap()
    }

    fn poll_to_loaded(
        adapter: &mut WorldStreamAdapter<TestInner, TestPublisher>,
        file: &mut AdaptedStreamFile<
            TestHandle,
            TestError,
            TestError,
            TestStreamFileIdentity,
            TestLocalSpatialIndexIdentity,
            TestPublicationToken,
        >,
    ) {
        for _ in 0..3 {
            assert_ne!(
                adapter.poll_stream_file(file).unwrap(),
                StreamFileState::LOADED
            );
        }
        assert_eq!(
            adapter.poll_stream_file(file).unwrap(),
            StreamFileState::LOADED
        );
        adapter
            .publisher
            .events
            .borrow_mut()
            .push(Event::ObservedPublicLoadedStateFour);
    }

    #[test]
    fn maps_all_three_stream_kinds_exhaustively() {
        assert_eq!(
            stream_kind(WorldStreamType::Presentation),
            StreamKind::Presentation
        );
        assert_eq!(
            stream_kind(WorldStreamType::Simulation),
            StreamKind::Simulation
        );
        assert_eq!(stream_kind(WorldStreamType::Texture), StreamKind::Texture);

        let (mut adapter, _) = adapter_with_plans(0);
        for (outer, inner) in [
            (WorldStreamType::Presentation, StreamKind::Presentation),
            (WorldStreamType::Simulation, StreamKind::Simulation),
            (WorldStreamType::Texture, StreamKind::Texture),
        ] {
            let file = adapter.begin_stream_file("root", "BAM", outer).unwrap();
            assert_eq!(file.kind(), inner);
        }
    }

    #[test]
    fn native_path_order_is_unsigned_prefix_aware_and_stable_after_equals() {
        let path_7f = NativeCPath::from_bytes(&b"path\x7f"[..]);
        let path_80 = NativeCPath::from_bytes(&b"path\x80"[..]);
        let short = NativeCPath::from_bytes(&b"BAM"[..]);
        let long = NativeCPath::from_bytes(&b"BAM_Pres"[..]);
        let nul_left = NativeCPath::from_bytes(&b"B\0after"[..]);
        let nul_right = NativeCPath::from_bytes(&b"B\0different"[..]);
        assert_eq!(
            compare_native_stream_file_paths(&path_7f, &path_80),
            Ordering::Less
        );
        assert_eq!(
            compare_native_stream_file_paths(&short, &long),
            Ordering::Less
        );
        assert_eq!(
            compare_native_stream_file_paths(&long, &short),
            Ordering::Greater
        );
        assert_eq!(
            compare_native_stream_file_paths(&long, &long),
            Ordering::Equal
        );
        assert_eq!(
            compare_native_stream_file_paths(&nul_left, &nul_right),
            Ordering::Equal
        );
        assert_eq!(nul_left.stored_bytes(), b"B\0after");

        let existing = [
            NativeCPath::from_bytes(&b"A"[..]),
            NativeCPath::from_bytes(&b"B"[..]),
            NativeCPath::from_bytes(&b"B\0tail"[..]),
            NativeCPath::from_bytes(&b"C"[..]),
        ];
        let candidate = NativeCPath::from_bytes(&b"B"[..]);
        assert_eq!(
            native_stream_file_insert_index(existing.iter(), &candidate),
            3
        );

        let (mut adapter, _) = adapter_with_plans(0);
        adapter.publisher.stream_files[0] = vec![
            (NativeCPath::from_bytes(&b"A"[..]), 40),
            (NativeCPath::from_bytes(&b"B"[..]), 41),
            (NativeCPath::from_bytes(&b"B\0tail"[..]), 42),
            (NativeCPath::from_bytes(&b"C"[..]), 43),
        ];
        let identity = Arc::new(TestStreamFileIdentity { serial: 44 });
        let local_identity = TestLocalSpatialIndexIdentity {
            serial: 144,
            raw_logical_len: 0,
            bounds: local_spatial_index_bounds(&[]),
            insertions: Vec::new(),
            drop_count: None,
        };
        let records = Arc::new(ProcessedOctreeStorage::new(0, Arc::from([])).unwrap());
        adapter
            .publisher
            .publish_loaded_stream_transactionally(
                StreamKind::Presentation,
                &identity,
                local_identity,
                &candidate,
                &records,
            )
            .unwrap();
        assert_eq!(
            adapter.publisher.stream_files[0],
            [
                (NativeCPath::from_bytes(&b"A"[..]), 40),
                (NativeCPath::from_bytes(&b"B"[..]), 41),
                (NativeCPath::from_bytes(&b"B\0tail"[..]), 42),
                (NativeCPath::from_bytes(&b"B"[..]), 44),
                (NativeCPath::from_bytes(&b"C"[..]), 43)
            ]
        );
    }

    #[test]
    fn filters_lod_and_existing_ids_before_retaining_processed_identity_in_source_order() {
        let (mut adapter, events) = adapter_with_plans(0);
        adapter.inner.plan(
            ReadStage::CollectionSpace,
            [AsyncReadPoll::Complete(collection_space_entry_bytes(&[
                (0x3000, 1),
                (0x3001, 0),
                (0x3002, 0),
                (0x3003, 0),
            ]))],
        );
        adapter.publisher.valid_ids.push(0x3001);
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);

        let processed = file.processed_octree_records().unwrap();
        assert_eq!(
            processed
                .iter()
                .map(|record| (record.metadata.bbox_index, record.metadata.id))
                .collect::<Vec<_>>(),
            [(0, 0x3002), (1, 0x3003)]
        );
        let local = adapter.publisher.published_local(0);
        assert_eq!(local.raw_logical_len, 4);
        assert_eq!(local.insertions.len(), 2);
        let storage = file.processed_octree_storage().unwrap();
        assert_eq!(storage.raw_logical_len(), 4);
        assert_eq!(storage.accepted_prefix().len(), 2);
        assert_eq!(storage.opaque_tail_len(), 2);
        assert_eq!(
            ProcessedOctreeStorage::new(1, Arc::clone(storage.accepted_prefix_identity()))
                .unwrap_err(),
            ProcessedOctreeStorageError {
                raw_logical_len: 1,
                accepted_len: 2,
            }
        );
        let relevant = events
            .borrow()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::CheckValid(_)
                        | Event::StreamOffset(_)
                        | Event::StreamSize(_)
                        | Event::AddHigh(_, _, _)
                        | Event::LocalDescriptor(..)
                        | Event::LocalAllocate(..)
                        | Event::LocalInitializeAttempt(..)
                        | Event::InsertIntoStoredLocal(..)
                        | Event::LocalSpaceBuilt(_)
                        | Event::PublishOctree(_, _, _, _)
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            relevant,
            [
                Event::CheckValid(0x3001),
                Event::CheckValid(0x3002),
                Event::StreamOffset(0x3002),
                Event::StreamSize(0x3002),
                Event::StreamSize(0x3002),
                Event::AddHigh(0x3002, 0, 0),
                Event::CheckValid(0x3003),
                Event::StreamOffset(0x3003),
                Event::StreamSize(0x3003),
                Event::StreamSize(0x3003),
                Event::AddHigh(0x3003, 0, 0),
                Event::LocalDescriptor(StreamKind::Presentation, 4, local.bounds),
                Event::LocalAllocate(100),
                Event::LocalInitializeAttempt(100, 4, local.bounds),
                Event::InsertIntoStoredLocal(
                    100,
                    0,
                    [0, 0, 0, 1.0f32.to_bits()],
                    [0, 0, 0, 1.0f32.to_bits()],
                ),
                Event::InsertIntoStoredLocal(
                    100,
                    1,
                    [0, 0, 0, 1.0f32.to_bits()],
                    [0, 0, 0, 1.0f32.to_bits()],
                ),
                Event::LocalSpaceBuilt(4),
                Event::PublishOctree(1, 0, 0, 0x3002),
                Event::PublishOctree(1, 1, 1, 0x3003),
            ]
        );
    }

    #[test]
    fn local_index_uses_compact_records_and_exact_zero_accepted_extent() {
        assert_eq!(
            COLLECTION_BOX_EXPANSION_WORDS,
            [0x4348_0000, 0x4348_0000, 0x4348_0000, 0x0000_0000]
        );
        let (mut adapter, _) = adapter_with_plans(0);
        adapter.inner.plan(
            ReadStage::CollectionSpace,
            [AsyncReadPoll::Complete(collection_space_entry_bytes(&[
                (0x3100, 1),
                (0x3101, 1),
            ]))],
        );
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);

        let local = adapter.publisher.published_local(0);
        assert_eq!(local.raw_logical_len, 2);
        assert!(local.insertions.is_empty());
        let storage = file.processed_octree_storage().unwrap();
        assert_eq!(storage.raw_logical_len(), 2);
        assert!(storage.accepted_prefix().is_empty());
        assert_eq!(storage.opaque_tail_len(), 2);
        assert_eq!(
            local.bounds,
            LocalSpatialIndexBounds {
                aggregate: LocalSpatialBoundsWords {
                    min: [
                        (-1.0f32).to_bits(),
                        (-1.0f32).to_bits(),
                        (-1.0f32).to_bits(),
                        0,
                    ],
                    max: [1.0f32.to_bits(), 1.0f32.to_bits(), 1.0f32.to_bits(), 0],
                },
                expanded: LocalSpatialBoundsWords {
                    min: [
                        (-201.0f32).to_bits(),
                        (-201.0f32).to_bits(),
                        (-201.0f32).to_bits(),
                        0,
                    ],
                    max: [
                        201.0f32.to_bits(),
                        201.0f32.to_bits(),
                        201.0f32.to_bits(),
                        0,
                    ],
                },
            }
        );
    }

    #[test]
    fn local_index_aggregate_uses_only_accepted_bbox_minima_and_maxima() {
        let mut bytes = collection_space_entry_bytes(&[(0x3200, 1), (0x3201, 0), (0x3202, 0)]);
        for (index, min, max) in [
            (0, [-9_000.0, -9_000.0, -9_000.0, 0.0], [9_000.0; 4]),
            (1, [10.0, 20.0, 30.0, -7.0], [20.0, 30.0, 40.0, -8.0]),
            (2, [-5.0, 25.0, 10.0, -9.0], [15.0, 50.0, 60.0, -10.0]),
        ] {
            let start = 24 + index * 80;
            for (lane, value) in min.into_iter().enumerate() {
                put_f32(&mut bytes, start + 16 + lane * 4, value);
            }
            for (lane, value) in max.into_iter().enumerate() {
                put_f32(&mut bytes, start + 32 + lane * 4, value);
            }
        }

        let (mut adapter, _) = adapter_with_plans(0);
        adapter
            .inner
            .plan(ReadStage::CollectionSpace, [AsyncReadPoll::Complete(bytes)]);
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);
        let local = adapter.publisher.published_local(0);
        assert_eq!(local.raw_logical_len, 3);
        assert_eq!(local.insertions.len(), 2);
        assert_eq!(local.insertions[0].0, 0);
        assert_eq!(local.insertions[0].1[0], 10.0f32.to_bits());
        assert_eq!(local.insertions[1].0, 1);
        assert_eq!(local.insertions[1].1[0], (-5.0f32).to_bits());
        assert_eq!(
            local.bounds,
            LocalSpatialIndexBounds {
                aggregate: LocalSpatialBoundsWords {
                    min: [
                        (-5.0f32).to_bits(),
                        20.0f32.to_bits(),
                        10.0f32.to_bits(),
                        1.0f32.to_bits(),
                    ],
                    max: [
                        20.0f32.to_bits(),
                        50.0f32.to_bits(),
                        60.0f32.to_bits(),
                        1.0f32.to_bits(),
                    ],
                },
                expanded: LocalSpatialBoundsWords {
                    min: [
                        (-205.0f32).to_bits(),
                        (-180.0f32).to_bits(),
                        (-190.0f32).to_bits(),
                        1.0f32.to_bits(),
                    ],
                    max: [
                        220.0f32.to_bits(),
                        250.0f32.to_bits(),
                        260.0f32.to_bits(),
                        1.0f32.to_bits(),
                    ],
                },
            }
        );
    }

    #[test]
    fn same_file_duplicate_ids_are_processed_but_global_identity_remains_first() {
        let (mut adapter, _) = adapter_with_plans(0);
        adapter.inner.plan(
            ReadStage::CollectionSpace,
            [AsyncReadPoll::Complete(collection_space_entry_bytes(&[
                (0x4000, 0),
                (0x4000, 0),
            ]))],
        );
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);

        let processed = file.processed_octree_records().unwrap();
        assert_eq!(processed.len(), 2);
        assert_eq!(processed[0].metadata.bbox_index, 0);
        assert_eq!(processed[1].metadata.bbox_index, 1);
        assert_eq!(adapter.publisher.octree_map[&0x4000], (1, 0));
        assert_eq!(
            adapter
                .publisher
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::PublishOctree(_, _, _, 0x4000)))
                .count(),
            2
        );
    }

    #[test]
    fn publisher_contract_keeps_first_collision_but_publishes_both_files() {
        let id = 0x4400;
        let (mut adapter, events) = adapter_with_plans(0);
        let mut first = begin_presentation(&mut adapter);
        let mut second = begin_presentation(&mut adapter);
        let space = CollectionSpace::parse(&collection_space_entry_bytes(&[(id, 0)])).unwrap();

        for file in [&mut first, &mut second] {
            let context = file.inner.registration_context();
            let mut processing_error = None;
            let mut backend = ProcessingBackend {
                inner: &mut adapter.inner,
                publisher: &mut adapter.publisher,
                processed: &mut file.processed_octree_storage,
                publication: &mut file.publication,
                processing_error: &mut processing_error,
            };
            backend.collection_space_loaded(context, &space).unwrap();
            assert!(processing_error.is_none());
        }
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Event::CheckValid(id))
                .count(),
            2
        );

        let first_records = Arc::clone(first.processed_octree_storage().unwrap());
        let second_records = Arc::clone(second.processed_octree_storage().unwrap());
        let first_file = Arc::clone(first.stream_file_identity());
        let second_file = Arc::clone(second.stream_file_identity());
        let first_local = take_pending_local(&mut first);
        let second_local = take_pending_local(&mut second);

        adapter
            .publisher
            .publish_loaded_stream_transactionally(
                StreamKind::Presentation,
                &first_file,
                first_local,
                &first.native_file_path,
                &first_records,
            )
            .unwrap();
        adapter
            .publisher
            .publish_loaded_stream_transactionally(
                StreamKind::Presentation,
                &second_file,
                second_local,
                &second.native_file_path,
                &second_records,
            )
            .unwrap();

        assert_eq!(adapter.publisher.octree_map[&id], (1, 0));
        assert_eq!(adapter.publisher.stream_files[0].len(), 2);
        assert_eq!(adapter.publisher.stream_files[0][0].1, 1);
        assert_eq!(adapter.publisher.stream_files[0][1].1, 2);
        assert_eq!(adapter.publisher.publications.len(), 2);
        assert!(adapter.publisher.publications.iter().all(Option::is_some));
    }

    #[test]
    #[allow(clippy::too_many_lines)] // One audit keeps every processed field together.
    fn ordinary_finite_processed_fields_and_live_metadata_call_count_match() {
        let id = 0x55;
        let (mut adapter, events) = adapter_with_plans(0);
        adapter.inner.plan(
            ReadStage::CollectionSpace,
            [AsyncReadPoll::Complete(detailed_collection_space_bytes(id))],
        );
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);

        let record = &file.processed_octree_records().unwrap()[0];
        assert_eq!(
            record.metadata.bbox_min_words,
            [
                (-1.0f32).to_bits(),
                2.0f32.to_bits(),
                19.0f32.to_bits(),
                1.0f32.to_bits()
            ]
        );
        assert_eq!(
            record.metadata.bbox_max_words,
            [
                3.0f32.to_bits(),
                6.0f32.to_bits(),
                25.0f32.to_bits(),
                1.0f32.to_bits()
            ]
        );
        assert_eq!(record.metadata.geometry_min_words[3], 1.0f32.to_bits());
        assert_eq!(record.metadata.geometry_max_words[3], 1.0f32.to_bits());
        assert_eq!(
            record.metadata.position_words,
            [
                1.0f32.to_bits(),
                4.0f32.to_bits(),
                22.0f32.to_bits(),
                1.0f32.to_bits()
            ]
        );
        assert_eq!(record.metadata.id, id);
        assert_eq!(record.metadata.offset, 0x1000 + id);
        assert_eq!(record.metadata.stream_order, 10_002);
        assert_eq!(record.metadata.bbox_index, 0);
        assert_eq!(record.metadata.ticks, 2);
        assert_eq!(
            record.metadata.size,
            i32::from_ne_bytes((0x5000 + u32::try_from(id).unwrap()).to_ne_bytes())
        );
        assert_eq!(record.metadata.stream_type, StreamKind::Presentation);
        let lifecycle = record.lifecycle().unwrap();
        assert_eq!(
            (
                lifecycle.loaded_count,
                lifecycle.activated,
                lifecycle.activation
            ),
            (0, 0, 0)
        );
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Event::StreamSize(id))
                .count(),
            2
        );
        assert_eq!(
            adapter.publisher.published_local(0).bounds.expanded,
            LocalSpatialBoundsWords {
                min: [
                    (-201.0f32).to_bits(),
                    (-198.0f32).to_bits(),
                    (-181.0f32).to_bits(),
                    1.0f32.to_bits(),
                ],
                max: [
                    203.0f32.to_bits(),
                    206.0f32.to_bits(),
                    225.0f32.to_bits(),
                    1.0f32.to_bits(),
                ],
            }
        );
        assert_eq!(
            adapter.publisher.published_local(0).insertions,
            [(
                0,
                record.metadata.bbox_min_words,
                record.metadata.bbox_max_words,
            )]
        );
        let add_high = events
            .borrow()
            .iter()
            .position(|event| *event == Event::AddHigh(id, 1, 22))
            .unwrap();
        let published = events
            .borrow()
            .iter()
            .position(|event| matches!(event, Event::PublishOctree(_, 0, 0, value) if *value == id))
            .unwrap();
        assert!(add_high < published);
    }

    #[test]
    fn pdb_signed_words_and_synchronized_lifecycle_preserve_raw_bits() {
        let (mut adapter, _) = adapter_with_plans(1);
        adapter.publisher.raw_size_override = Some(0x8000_0001);
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);

        let record = &file.processed_octree_records().unwrap()[0];
        assert_eq!(record.metadata.ticks.to_ne_bytes(), 2u32.to_ne_bytes());
        assert_eq!(
            record.metadata.size.to_ne_bytes(),
            0x8000_0001u32.to_ne_bytes()
        );
        let original_id = record.metadata.id;
        {
            let mut lifecycle = record.lifecycle().unwrap();
            lifecycle.loaded_count = 0xfe;
            lifecycle.activated = 1;
            lifecycle.activation = 0x80;
        }
        assert_eq!(record.metadata.id, original_id);
        assert_eq!(
            *record.lifecycle().unwrap(),
            ProcessedOctreeLifecycle {
                loaded_count: 0xfe,
                activated: 1,
                activation: 0x80,
            }
        );
    }

    #[test]
    fn exceptional_spatial_coordinates_are_safe_explicit_deviations() {
        for (axis, hull_min, hull_max) in [
            (0, [f32::NAN, 0.0, 0.0, 0.0], [1.0, 0.0, 1.0, 0.0]),
            (2, [0.0, 0.0, f32::NEG_INFINITY, 0.0], [1.0, 0.0, 1.0, 0.0]),
        ] {
            let source = CollectionSpaceEntry {
                id: 0x9999,
                lod: 0,
                hull_min,
                hull_max,
                geometry_min: [0.0; 4],
                geometry_max: [0.0; 4],
            };
            assert!(matches!(
                processed_position_and_order::<TestError, TestError>(&source),
                Err(WorldStreamAdapterError::UnsupportedSpatialCoordinate {
                    id: 0x9999,
                    axis: actual_axis,
                    ..
                }) if actual_axis == axis
            ));
        }
    }

    #[test]
    fn add_high_collection_is_presentation_only() {
        for (outer, expect_high) in [
            (WorldStreamType::Presentation, true),
            (WorldStreamType::Simulation, false),
            (WorldStreamType::Texture, false),
        ] {
            let (mut adapter, events) = adapter_with_plans(1);
            let mut file = adapter.begin_stream_file("root", "BAM", outer).unwrap();
            poll_to_loaded(&mut adapter, &mut file);
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .any(|event| matches!(event, Event::AddHigh(..))),
                expect_high
            );
        }
    }

    #[test]
    fn publishes_records_then_sorted_file_once_before_reporting_loaded() {
        let (mut adapter, events) = adapter_with_plans(2);
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);
        assert!(file.is_globally_published());
        let publication = adapter.publisher.publication(0);
        assert!(Arc::ptr_eq(
            file.processed_octree_storage().unwrap(),
            &publication.records
        ));
        assert!(Arc::ptr_eq(file.stream_file_identity(), &publication.file));
        assert!(file.pending_local_spatial_index_identity().is_none());
        let local = adapter.publisher.published_local(0);
        assert_eq!(local.serial, 100);
        assert_eq!(local.raw_logical_len, 2);
        let storage = file.processed_octree_storage().unwrap();
        assert_eq!(storage.raw_logical_len(), 2);
        assert_eq!(storage.accepted_prefix().len(), 2);
        assert_eq!(storage.opaque_tail_len(), 0);
        assert_eq!(local.insertions.len(), 2);
        assert_eq!(
            events.borrow().as_slice(),
            &[
                Event::Retain(StreamKind::Presentation, 1, b"root\\BAM\\BAM_Pres".to_vec(),),
                Event::Begin(ReadStage::CollectionMap),
                Event::Poll(ReadStage::CollectionMap),
                Event::Begin(ReadStage::AssetToc),
                Event::Poll(ReadStage::AssetToc),
                Event::Begin(ReadStage::CollectionSpace),
                Event::Poll(ReadStage::CollectionSpace),
                Event::CheckValid(0x2000),
                Event::StreamOffset(0x2000),
                Event::StreamSize(0x2000),
                Event::StreamSize(0x2000),
                Event::AddHigh(0x2000, 0, 0),
                Event::CheckValid(0x2001),
                Event::StreamOffset(0x2001),
                Event::StreamSize(0x2001),
                Event::StreamSize(0x2001),
                Event::AddHigh(0x2001, 0, 0),
                Event::LocalDescriptor(StreamKind::Presentation, 2, local.bounds),
                Event::LocalAllocate(100),
                Event::LocalInitializeAttempt(100, 2, local.bounds),
                Event::InsertIntoStoredLocal(
                    100,
                    0,
                    [0, 0, 0, 1.0f32.to_bits()],
                    [0, 0, 0, 1.0f32.to_bits()],
                ),
                Event::InsertIntoStoredLocal(
                    100,
                    1,
                    [0, 0, 0, 1.0f32.to_bits()],
                    [0, 0, 0, 1.0f32.to_bits()],
                ),
                Event::LocalSpaceBuilt(2),
                Event::PublishOctree(1, 0, 0, 0x2000),
                Event::PublishOctree(1, 1, 1, 0x2001),
                Event::PublishSorted(StreamKind::Presentation, 1, b"root\\BAM\\BAM_Pres".to_vec(),),
                Event::ObservedPublicLoadedStateFour,
            ]
        );

        let event_count = events.borrow().len();
        assert_eq!(
            adapter.poll_stream_file(&mut file).unwrap(),
            StreamFileState::LOADED
        );
        assert_eq!(events.borrow().len(), event_count);
    }

    #[test]
    fn publication_error_is_latched_without_retrying_partial_work() {
        let (mut adapter, events) = adapter_with_plans(2);
        adapter.publisher.fail_phase = Some(PublicationPhase::PublishLoadedStream);
        let mut file = begin_presentation(&mut adapter);
        for _ in 0..3 {
            assert_ne!(
                adapter.poll_stream_file(&mut file).unwrap(),
                StreamFileState::LOADED
            );
        }
        let first = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(matches!(
            &*first,
            WorldStreamAdapterError::Publication {
                phase: PublicationPhase::PublishLoadedStream,
                ..
            }
        ));
        let event_count = events.borrow().len();
        let second = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(events.borrow().len(), event_count);
        assert!(!file.is_globally_published());
        assert!(adapter.publisher.stream_files[0].is_empty());
        assert!(adapter.publisher.octree_map.is_empty());
    }

    #[test]
    fn processing_failures_latch_before_local_or_global_publication() {
        for (fail_validity_id, fail_metadata_call, expected_phase) in [
            (
                Some(0x2000),
                None,
                PublicationPhase::CheckCollectionValidity { source_index: 0 },
            ),
            (
                None,
                Some(1),
                PublicationPhase::GetStreamOffset { source_index: 0 },
            ),
            (
                None,
                Some(2),
                PublicationPhase::GetTicksStreamSize { source_index: 0 },
            ),
            (
                None,
                Some(3),
                PublicationPhase::GetRawStreamSize { source_index: 0 },
            ),
        ] {
            let (mut adapter, events) = adapter_with_plans(1);
            adapter.publisher.fail_validity_id = fail_validity_id;
            adapter.publisher.fail_metadata_call = fail_metadata_call;
            let mut file = begin_presentation(&mut adapter);
            for _ in 0..3 {
                adapter.poll_stream_file(&mut file).unwrap();
            }
            let first = adapter.poll_stream_file(&mut file).unwrap_err();
            assert!(matches!(
                &*first,
                WorldStreamAdapterError::Publication { phase, .. } if *phase == expected_phase
            ));
            let event_count = events.borrow().len();
            let second = adapter.poll_stream_file(&mut file).unwrap_err();
            assert!(Arc::ptr_eq(&first, &second));
            assert_eq!(events.borrow().len(), event_count);
            assert!(!events.borrow().iter().any(|event| matches!(
                event,
                Event::LocalSpaceBuilt(_) | Event::PublishOctree(_, _, _, _)
            )));
        }
    }

    #[test]
    fn add_high_is_immediate_and_survives_a_later_record_failure() {
        let (mut adapter, events) = adapter_with_plans(2);
        // First record consumes metadata calls 1..3; the second offset is call 4.
        adapter.publisher.fail_metadata_call = Some(4);
        let mut file = begin_presentation(&mut adapter);
        for _ in 0..3 {
            assert_ne!(
                adapter.poll_stream_file(&mut file).unwrap(),
                StreamFileState::LOADED
            );
        }
        let first = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(matches!(
            &*first,
            WorldStreamAdapterError::Publication {
                phase: PublicationPhase::GetStreamOffset { source_index: 1 },
                ..
            }
        ));
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::AddHigh(0x2000, _, _)))
                .count(),
            1
        );
        assert!(!events.borrow().iter().any(|event| matches!(
            event,
            Event::LocalDescriptor(..)
                | Event::LocalAllocate(..)
                | Event::LocalInitializeAttempt(..)
                | Event::InsertIntoStoredLocal(..)
                | Event::LocalSpaceBuilt(_)
                | Event::PublishOctree(_, _, _, _)
                | Event::PublishSorted(_, _, _)
        )));
        let event_count = events.borrow().len();
        let second = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(events.borrow().len(), event_count);
    }

    #[test]
    fn local_index_construction_failure_is_a_latched_safe_deviation() {
        let (mut adapter, events) = adapter_with_plans(1);
        adapter.publisher.fail_phase = Some(PublicationPhase::ConstructLocalSpatialIndex);
        let mut file = begin_presentation(&mut adapter);
        for _ in 0..3 {
            assert_ne!(
                adapter.poll_stream_file(&mut file).unwrap(),
                StreamFileState::LOADED
            );
        }
        let error = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(matches!(
            &*error,
            WorldStreamAdapterError::Publication {
                phase: PublicationPhase::ConstructLocalSpatialIndex,
                ..
            }
        ));
        assert!(!file.is_globally_published());
        let storage = file.processed_octree_storage().unwrap();
        assert_eq!(storage.raw_logical_len(), 1);
        assert_eq!(storage.accepted_prefix().len(), 1);
        assert!(file.pending_local_spatial_index_identity().is_none());
        let local_events = events
            .borrow()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::LocalDescriptor(..)
                        | Event::LocalAllocate(..)
                        | Event::LocalInitializeAttempt(..)
                        | Event::InsertIntoStoredLocal(..)
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        assert!(matches!(
            local_events.as_slice(),
            [
                Event::LocalDescriptor(StreamKind::Presentation, 1, _),
                Event::LocalAllocate(100),
                Event::LocalInitializeAttempt(100, 1, _),
            ]
        ));
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::AddHigh(..)))
                .count(),
            1
        );
        assert!(!events.borrow().iter().any(|event| matches!(
            event,
            Event::LocalSpaceBuilt(_)
                | Event::PublishOctree(_, _, _, _)
                | Event::PublishSorted(_, _, _)
        )));
    }

    #[test]
    fn local_insert_failure_retains_partial_identity_without_exposing_loaded() {
        let (mut adapter, events) = adapter_with_plans(2);
        adapter.publisher.fail_phase =
            Some(PublicationPhase::InsertLocalSpatialRecord { compact_index: 1 });
        let mut file = begin_presentation(&mut adapter);
        for _ in 0..3 {
            assert_ne!(
                adapter.poll_stream_file(&mut file).unwrap(),
                StreamFileState::LOADED
            );
        }
        let error = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(matches!(
            &*error,
            WorldStreamAdapterError::Publication {
                phase: PublicationPhase::InsertLocalSpatialRecord { compact_index: 1 },
                ..
            }
        ));
        let storage = file.processed_octree_storage().unwrap();
        assert_eq!(storage.raw_logical_len(), 2);
        assert_eq!(storage.accepted_prefix().len(), 2);
        let local = file.pending_local_spatial_index_identity().unwrap();
        assert_eq!(local.insertions.len(), 1);
        assert_eq!(local.insertions[0].0, 0);
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::AddHigh(..)))
                .count(),
            2
        );
        assert!(!events.borrow().iter().any(|event| matches!(
            event,
            Event::LocalSpaceBuilt(_)
                | Event::PublishOctree(_, _, _, _)
                | Event::PublishSorted(_, _, _)
                | Event::ObservedPublicLoadedStateFour
        )));
    }

    #[test]
    fn real_outer_poll_publishes_before_same_poll_xml_start() {
        let (mut adapter, events) = adapter_with_plans(1);
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads
            .start(WorldStreamSpec::derive("BAM", None).unwrap())
            .unwrap();

        for _ in 0..5 {
            assert_eq!(
                loads.poll(handle, &mut adapter).unwrap(),
                crate::game::WorldStreamPoll::Pending
            );
        }
        assert_eq!(
            loads.state(handle).unwrap(),
            WorldStreamOuterState::POLL_PRESENTATION_XML
        );
        let events = events.borrow();
        let publish = events
            .iter()
            .position(|event| matches!(event, Event::PublishSorted(_, _, _)))
            .unwrap();
        let xml = events
            .iter()
            .position(|event| *event == Event::Begin(ReadStage::StreamXml))
            .unwrap();
        assert!(publish < xml);
    }

    #[test]
    fn adapter_drives_presentation_simulation_and_texture_outer_progression() {
        let (mut adapter, events) = adapter_with_plans(1);
        adapter.inner.plan(
            ReadStage::CollectionMap,
            (0..3).map(|_| AsyncReadPoll::Complete(collection_map_bytes())),
        );
        adapter.inner.plan(
            ReadStage::AssetToc,
            (0..3).map(|_| AsyncReadPoll::Complete(asset_toc_bytes())),
        );
        adapter.inner.plan(
            ReadStage::CollectionSpace,
            (0..3).map(|_| AsyncReadPoll::Complete(collection_space_bytes(1))),
        );
        adapter.inner.plan(
            ReadStage::StreamXml,
            (0..2).map(|_| AsyncReadPoll::Complete(Vec::new())),
        );
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads
            .start(WorldStreamSpec::derive("BAM", None).unwrap())
            .unwrap();

        let mut completed = false;
        for _ in 0..24 {
            if loads.poll(handle, &mut adapter).unwrap()
                == crate::game::WorldStreamPoll::CompletedThisPoll
            {
                completed = true;
                break;
            }
        }
        assert!(completed);
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter_map(|event| match event {
                    Event::PublishSorted(kind, _, _) => Some(*kind),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [
                StreamKind::Presentation,
                StreamKind::Simulation,
                StreamKind::Texture,
            ]
        );
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Event::Begin(ReadStage::StreamXml))
                .count(),
            2
        );
    }

    #[test]
    fn transactional_failure_suppresses_xml_and_outer_replays_original_cause() {
        let (mut adapter, events) = adapter_with_plans(1);
        adapter.publisher.fail_phase = Some(PublicationPhase::PublishLoadedStream);
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads
            .start(WorldStreamSpec::derive("BAM", None).unwrap())
            .unwrap();

        for _ in 0..4 {
            assert_eq!(
                loads.poll(handle, &mut adapter).unwrap(),
                crate::game::WorldStreamPoll::Pending
            );
        }
        let first = loads.poll(handle, &mut adapter).unwrap_err();
        assert!(matches!(
            &*first,
            crate::game::WorldStreamControllerError::Backend {
                operation: crate::game::WorldStreamOperation::PollStreamFile(
                    WorldStreamType::Presentation
                ),
                source,
                ..
            } if matches!(
                &**source,
                WorldStreamAdapterError::Publication {
                    phase: PublicationPhase::PublishLoadedStream,
                    ..
                }
            )
        ));
        assert_eq!(adapter.publisher.publish_attempts, 1);
        assert!(
            !events
                .borrow()
                .contains(&Event::Begin(ReadStage::StreamXml))
        );
        assert!(adapter.publisher.stream_files[0].is_empty());

        adapter.publisher.fail_phase = None;
        let event_count = events.borrow().len();
        let second = loads.poll(handle, &mut adapter).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(adapter.publisher.publish_attempts, 1);
        assert_eq!(events.borrow().len(), event_count);
    }

    #[test]
    fn stream_file_identity_failure_is_replayed_without_inner_reads() {
        let (mut adapter, events) = adapter_with_plans(1);
        adapter.publisher.fail_phase = Some(PublicationPhase::RetainStreamFileIdentity);
        let mut loads = WorldStreamLoads::debug_reference();
        let handle = loads
            .start(WorldStreamSpec::derive("BAM", None).unwrap())
            .unwrap();

        let first = loads.poll(handle, &mut adapter).unwrap_err();
        assert!(matches!(
            &*first,
            crate::game::WorldStreamControllerError::Backend {
                operation: crate::game::WorldStreamOperation::BeginStreamFile(
                    WorldStreamType::Presentation
                ),
                source,
                ..
            } if matches!(
                &**source,
                WorldStreamAdapterError::Publication {
                    phase: PublicationPhase::RetainStreamFileIdentity,
                    ..
                }
            )
        ));
        assert!(events.borrow().is_empty());
        adapter.publisher.fail_phase = None;
        let second = loads.poll(handle, &mut adapter).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn inner_error_is_latched_without_restarting_the_read() {
        let (mut adapter, events) = adapter_with_plans(0);
        adapter.inner.fail_begin = true;
        let mut file = begin_presentation(&mut adapter);
        let first = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(matches!(&*first, WorldStreamAdapterError::Inner(_)));
        let second = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| **event == Event::Begin(ReadStage::CollectionMap))
                .count(),
            1
        );
    }

    #[test]
    fn reports_each_native_manifest_stall_distinctly() {
        let cases = [
            (
                ReadStage::CollectionMap,
                2,
                ReferenceStreamStall::CollectionMap {
                    stream_file_state: 1,
                    game_stream_state: 8,
                },
            ),
            (
                ReadStage::AssetToc,
                3,
                ReferenceStreamStall::AssetToc {
                    stream_file_state: 1,
                    game_stream_state: 3,
                },
            ),
            (
                ReadStage::CollectionSpace,
                4,
                ReferenceStreamStall::CollectionSpace {
                    stream_file_state: 2,
                    game_stream_state: 4,
                },
            ),
        ];

        for (failed_stage, polls_until_error, expected) in cases {
            let (mut adapter, _) = adapter_with_plans(1);
            adapter.inner.plan(failed_stage, [AsyncReadPoll::Failed]);
            let mut file = begin_presentation(&mut adapter);
            for _ in 1..polls_until_error {
                adapter.poll_stream_file(&mut file).unwrap();
            }
            let error = adapter.poll_stream_file(&mut file).unwrap_err();
            assert!(matches!(
                &*error,
                WorldStreamAdapterError::ReferenceStall {
                    kind: StreamKind::Presentation,
                    stall,
                } if *stall == expected
            ));
        }
    }

    #[test]
    fn xml_failure_is_normal_completion_and_basename_mismatch_is_terminal() {
        let (mut adapter, _) = adapter_with_plans(1);
        adapter
            .inner
            .plan(ReadStage::StreamXml, [AsyncReadPoll::Failed]);
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);
        adapter.begin_stream_xml(&mut file, "BAM").unwrap();
        adapter.update_stream_xml(&mut file).unwrap();
        assert_eq!(
            adapter.stream_xml_state(&mut file),
            StreamXmlState::LOAD_DONE
        );

        let (mut adapter, _) = adapter_with_plans(1);
        let mut mismatch = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut mismatch);
        let first = adapter
            .begin_stream_xml(&mut mismatch, "NOT_BAM")
            .unwrap_err();
        assert!(matches!(
            &*first,
            WorldStreamAdapterError::BasenameMismatch { retained, supplied }
                if retained == "BAM" && supplied == "NOT_BAM"
        ));
        let second = adapter.begin_stream_xml(&mut mismatch, "BAM").unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn loaded_unload_releases_local_before_raw_six_then_unpublishes_once() {
        let (mut adapter, events) = adapter_with_plans(1);
        let drops = Rc::new(Cell::new(0));
        adapter.publisher.next_local_drop_count = Some(Rc::clone(&drops));
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);
        let records = Arc::downgrade(file.processed_octree_storage().unwrap());
        let file_identity = Arc::downgrade(file.stream_file_identity());
        assert_eq!(drops.get(), 0);
        assert!(file.pending_local_spatial_index_identity().is_none());

        events.borrow_mut().clear();
        adapter.request_stream_file_unload(&mut file).unwrap();
        assert_eq!(file.stream_file_state().raw(), 5);
        for expected in [5, 5] {
            assert_eq!(adapter.poll_stream_file(&mut file).unwrap().raw(), expected);
            assert!(file.is_globally_published());
            assert_eq!(drops.get(), 0);
            assert!(
                !events
                    .borrow()
                    .iter()
                    .any(|event| matches!(event, Event::ReleaseLocal(_) | Event::Unpublish(_, _)))
            );
        }
        assert_eq!(
            events.borrow().as_slice(),
            &[
                Event::MapUnloaded(StreamKind::Presentation),
                Event::TocUnloaded(StreamKind::Presentation),
            ]
        );

        assert_eq!(adapter.poll_stream_file(&mut file).unwrap().raw(), 6);
        assert_eq!(file.stream_file_state().raw(), 6);
        assert!(!file.is_globally_published());
        assert_eq!(drops.get(), 1);
        assert!(file.processed_octree_storage().is_none());
        assert!(records.upgrade().is_none());
        assert!(file_identity.upgrade().is_some());
        assert_eq!(
            events.borrow().as_slice(),
            &[
                Event::MapUnloaded(StreamKind::Presentation),
                Event::TocUnloaded(StreamKind::Presentation),
                Event::ReleaseLocal(100),
                Event::StreamFileUnloaded(StreamKind::Presentation, true, false),
                Event::Unpublish(StreamKind::Presentation, 1),
            ]
        );

        let event_count = events.borrow().len();
        assert_eq!(adapter.poll_stream_file(&mut file).unwrap().raw(), 6);
        assert_eq!(events.borrow().len(), event_count);
        drop(file);
        assert!(file_identity.upgrade().is_none());
    }

    #[test]
    fn local_release_failure_latches_before_raw_six_and_suppresses_cleanup() {
        let (mut adapter, events) = adapter_with_plans(1);
        let drops = Rc::new(Cell::new(0));
        adapter.publisher.next_local_drop_count = Some(Rc::clone(&drops));
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);
        adapter.publisher.fail_phase = Some(PublicationPhase::ReleaseLocalSpatial);
        events.borrow_mut().clear();
        adapter.request_stream_file_unload(&mut file).unwrap();
        adapter.poll_stream_file(&mut file).unwrap();
        adapter.poll_stream_file(&mut file).unwrap();
        events.borrow_mut().clear();

        let first = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(matches!(
            &*first,
            WorldStreamAdapterError::Publication {
                phase: PublicationPhase::ReleaseLocalSpatial,
                ..
            }
        ));
        assert_eq!(file.stream_file_state().raw(), 5);
        assert!(file.is_globally_published());
        assert_eq!(drops.get(), 0);
        assert_eq!(events.borrow().as_slice(), &[Event::ReleaseLocal(100)]);
        let second = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(events.borrow().as_slice(), &[Event::ReleaseLocal(100)]);
    }

    #[test]
    fn final_inner_cleanup_failure_keeps_raw_five_after_one_local_release() {
        let (mut adapter, events) = adapter_with_plans(1);
        let drops = Rc::new(Cell::new(0));
        adapter.publisher.next_local_drop_count = Some(Rc::clone(&drops));
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);
        adapter.inner.fail_final_unload = true;
        events.borrow_mut().clear();
        adapter.request_stream_file_unload(&mut file).unwrap();
        adapter.poll_stream_file(&mut file).unwrap();
        adapter.poll_stream_file(&mut file).unwrap();
        events.borrow_mut().clear();

        let first = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(matches!(&*first, WorldStreamAdapterError::Inner(_)));
        assert_eq!(file.stream_file_state().raw(), 5);
        assert!(file.is_globally_published());
        assert_eq!(drops.get(), 1);
        assert_eq!(
            events.borrow().as_slice(),
            &[
                Event::ReleaseLocal(100),
                Event::StreamFileUnloaded(StreamKind::Presentation, true, false),
            ]
        );
        let second = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(events.borrow().len(), 2);
    }

    #[test]
    fn global_unpublish_failure_is_latched_after_raw_six_without_retry() {
        let (mut adapter, events) = adapter_with_plans(1);
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);
        adapter.publisher.fail_phase = Some(PublicationPhase::UnpublishLoadedStream);
        events.borrow_mut().clear();
        adapter.request_stream_file_unload(&mut file).unwrap();
        adapter.poll_stream_file(&mut file).unwrap();
        adapter.poll_stream_file(&mut file).unwrap();
        events.borrow_mut().clear();

        let first = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(matches!(
            &*first,
            WorldStreamAdapterError::Publication {
                phase: PublicationPhase::UnpublishLoadedStream,
                ..
            }
        ));
        assert_eq!(file.stream_file_state().raw(), 6);
        assert!(file.is_globally_published());
        assert!(file.processed_octree_storage().is_some());
        assert_eq!(
            events.borrow().as_slice(),
            &[
                Event::ReleaseLocal(100),
                Event::StreamFileUnloaded(StreamKind::Presentation, true, false),
                Event::Unpublish(StreamKind::Presentation, 1),
            ]
        );
        let second = adapter.poll_stream_file(&mut file).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(events.borrow().len(), 3);
    }

    #[test]
    fn early_raw_one_unload_has_no_local_or_global_publication() {
        let (mut adapter, events) = adapter_with_plans(0);
        let mut file = begin_presentation(&mut adapter);
        events.borrow_mut().clear();
        adapter.request_stream_file_unload(&mut file).unwrap();
        for expected in [5, 5, 6] {
            assert_eq!(adapter.poll_stream_file(&mut file).unwrap().raw(), expected);
        }
        assert!(!file.is_globally_published());
        assert_eq!(file.stream_file_state().raw(), 6);
        assert_eq!(
            events.borrow().as_slice(),
            &[Event::StreamFileUnloaded(
                StreamKind::Presentation,
                false,
                false,
            )]
        );
        assert!(matches!(file.publication, AdapterPublication::Unpublished));
        let event_count = events.borrow().len();
        assert_eq!(adapter.poll_stream_file(&mut file).unwrap().raw(), 6);
        assert_eq!(events.borrow().len(), event_count);
    }

    #[test]
    fn rejected_active_xss_unload_is_not_latched_and_later_request_succeeds() {
        let (mut adapter, _) = adapter_with_plans(1);
        let mut file = begin_presentation(&mut adapter);
        for _ in 0..3 {
            adapter.poll_stream_file(&mut file).unwrap();
        }
        assert_eq!(file.stream_file_state().raw(), 2);
        assert!(matches!(
            adapter.request_stream_file_unload(&mut file),
            Err(WorldStreamUnloadRequestError::Request(
                StreamFileUnloadError::UnsupportedState(2)
            ))
        ));
        assert!(file.terminal_error.is_none());
        assert_eq!(adapter.poll_stream_file(&mut file).unwrap().raw(), 4);
        adapter.request_stream_file_unload(&mut file).unwrap();
        assert_eq!(file.stream_file_state().raw(), 5);
    }

    #[test]
    fn zero_record_file_still_releases_local_and_unpublishes() {
        let (mut adapter, events) = adapter_with_plans(0);
        let mut file = begin_presentation(&mut adapter);
        poll_to_loaded(&mut adapter, &mut file);
        assert_eq!(adapter.publisher.published_local(0).raw_logical_len, 0);
        events.borrow_mut().clear();
        adapter.request_stream_file_unload(&mut file).unwrap();
        for _ in 0..3 {
            adapter.poll_stream_file(&mut file).unwrap();
        }
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::ReleaseLocal(100)))
                .count(),
            1
        );
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::Unpublish(_, 1)))
                .count(),
            1
        );
    }

    #[test]
    fn xml_and_texture_unload_forward_exact_presence_flags() {
        let (mut presentation, presentation_events) = adapter_with_plans(0);
        presentation.inner.plan(
            ReadStage::StreamXml,
            [AsyncReadPoll::Complete(vec![1, 2, 3])],
        );
        let mut presentation_file = begin_presentation(&mut presentation);
        poll_to_loaded(&mut presentation, &mut presentation_file);
        presentation
            .begin_stream_xml(&mut presentation_file, "BAM")
            .unwrap();
        presentation
            .update_stream_xml(&mut presentation_file)
            .unwrap();
        presentation_events.borrow_mut().clear();
        presentation
            .request_stream_file_unload(&mut presentation_file)
            .unwrap();
        for _ in 0..3 {
            presentation
                .poll_stream_file(&mut presentation_file)
                .unwrap();
        }
        assert!(
            presentation_events
                .borrow()
                .contains(&Event::StreamFileUnloaded(
                    StreamKind::Presentation,
                    true,
                    true
                ))
        );

        let (mut texture, texture_events) = adapter_with_plans(0);
        let mut texture_file = texture
            .begin_stream_file("root", "BAM", WorldStreamType::Texture)
            .unwrap();
        poll_to_loaded(&mut texture, &mut texture_file);
        texture_events.borrow_mut().clear();
        texture
            .request_stream_file_unload(&mut texture_file)
            .unwrap();
        for _ in 0..3 {
            texture.poll_stream_file(&mut texture_file).unwrap();
        }
        assert!(texture_events.borrow().contains(&Event::StreamFileUnloaded(
            StreamKind::Texture,
            true,
            false,
        )));
    }

    #[test]
    fn two_publication_tokens_release_only_their_own_file() {
        let (mut adapter, events) = adapter_with_plans(0);
        adapter.inner.plan(
            ReadStage::CollectionMap,
            [
                AsyncReadPoll::Complete(collection_map_bytes()),
                AsyncReadPoll::Complete(collection_map_bytes()),
            ],
        );
        adapter.inner.plan(
            ReadStage::AssetToc,
            [
                AsyncReadPoll::Complete(asset_toc_bytes()),
                AsyncReadPoll::Complete(asset_toc_bytes()),
            ],
        );
        adapter.inner.plan(
            ReadStage::CollectionSpace,
            [
                AsyncReadPoll::Complete(collection_space_bytes(0)),
                AsyncReadPoll::Complete(collection_space_bytes(0)),
            ],
        );
        let mut first = begin_presentation(&mut adapter);
        let first_registration = first.inner.registration_id().clone();
        poll_to_loaded(&mut adapter, &mut first);
        let mut second = begin_presentation(&mut adapter);
        let second_registration = second.inner.registration_id().clone();
        poll_to_loaded(&mut adapter, &mut second);
        assert_ne!(first_registration, second_registration);
        assert_eq!(
            adapter.publisher.retained_registration_ids,
            [first_registration, second_registration]
        );
        events.borrow_mut().clear();

        adapter.request_stream_file_unload(&mut second).unwrap();
        for _ in 0..3 {
            adapter.poll_stream_file(&mut second).unwrap();
        }
        assert!(first.is_globally_published());
        assert!(!second.is_globally_published());
        adapter.request_stream_file_unload(&mut first).unwrap();
        for _ in 0..3 {
            adapter.poll_stream_file(&mut first).unwrap();
        }
        let ownership_events = events
            .borrow()
            .iter()
            .filter(|event| matches!(event, Event::ReleaseLocal(_) | Event::Unpublish(_, _)))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            ownership_events,
            [
                Event::ReleaseLocal(101),
                Event::Unpublish(StreamKind::Presentation, 2),
                Event::ReleaseLocal(100),
                Event::Unpublish(StreamKind::Presentation, 1),
            ]
        );
    }
}
