//! Resident collection scheduling and reference-count orchestration.
//!
//! This module reconstructs the outer, one-active-request behavior of
//! `assman::cAssetStreamer`. Collection parsing, decompression, asset
//! construction, metadata storage, and parent relationships remain explicit
//! collaborators.

use std::{
    collections::VecDeque,
    fmt,
    marker::PhantomData,
    mem,
    sync::{Arc, Weak},
};

/// Exact capacity of the native fixed resident-collection map.
pub const RESIDENT_COLLECTION_CAPACITY: usize = 512;

/// Full-width native `assman::cAssetID` identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CollectionId(pub u64);

impl CollectionId {
    /// Native bucket hash at `0x82d56990`; equality still compares all 64 bits.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // The native hash adds the two 32-bit halves.
    pub const fn native_hash(self) -> u32 {
        (self.0 as u32).wrapping_add((self.0 >> 32) as u32)
    }
}

/// Semantic request kinds created by the recovered enqueue paths.
///
/// The executable stores zero for load and one for unload. This deliberately
/// does not reuse the conflicting PDB enum whose exported unload value is 8.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ResidentCollectionRequestKind {
    Load = 0,
    Unload = 1,
}

/// State visible to a completion callback at its exact invocation point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentCallbackObservation {
    pub active_id: CollectionId,
    pub resident_ref_count: Option<u32>,
    pub wrapper_is_published: bool,
}

/// A queued native collection request.
#[derive(Debug)]
pub enum ResidentCollectionRequest<C> {
    Load {
        id: CollectionId,
        asset_count: i32,
        callback: Option<C>,
    },
    Unload {
        id: CollectionId,
        callback: Option<C>,
    },
}

impl<C> ResidentCollectionRequest<C> {
    #[must_use]
    pub const fn id(&self) -> CollectionId {
        match self {
            Self::Load { id, .. } | Self::Unload { id, .. } => *id,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> ResidentCollectionRequestKind {
        match self {
            Self::Load { .. } => ResidentCollectionRequestKind::Load,
            Self::Unload { .. } => ResidentCollectionRequestKind::Unload,
        }
    }

    #[must_use]
    pub const fn raw_type(&self) -> u32 {
        self.kind() as u32
    }

    fn callback_mut(&mut self) -> &mut Option<C> {
        match self {
            Self::Load { callback, .. } | Self::Unload { callback, .. } => callback,
        }
    }
}

/// Intrusive-list semantics represented as an owning FIFO.
#[derive(Debug)]
pub struct ResidentRequestQueue<C> {
    requests: VecDeque<ResidentCollectionRequest<C>>,
}

impl<C> Default for ResidentRequestQueue<C> {
    fn default() -> Self {
        Self {
            requests: VecDeque::new(),
        }
    }
}

impl<C> ResidentRequestQueue<C> {
    pub fn push_load(&mut self, id: CollectionId, asset_count: i32, callback: Option<C>) {
        self.requests.push_back(ResidentCollectionRequest::Load {
            id,
            asset_count,
            callback,
        });
    }

    pub fn push_unload(&mut self, id: CollectionId, callback: Option<C>) {
        self.requests
            .push_back(ResidentCollectionRequest::Unload { id, callback });
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.requests.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    fn pop_front(&mut self) -> Option<ResidentCollectionRequest<C>> {
        self.requests.pop_front()
    }
}

/// Typed safe-host replacement for requesting an active-resident increment
/// when no resident record exists at the callback point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentCallbackContextError {
    MissingActiveResident { id: CollectionId },
}

impl fmt::Display for ResidentCallbackContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingActiveResident { id } => {
                write!(
                    formatter,
                    "active collection {:#018x} is not resident",
                    id.0
                )
            }
        }
    }
}

impl std::error::Error for ResidentCallbackContextError {}

/// Opaque capabilities available during one resident request callback.
///
/// The active-only increment is sufficient for the recovered parent
/// registration callbacks at `0x82d5c468` and `0x82d5c680`. The optional
/// occurrence-bound hold is host safety for delayed subsystem completions.
/// Private fields prevent callbacks from extracting the queue, mutating an
/// arbitrary resident, decrementing a count, or retaining a collection owner.
pub struct ResidentCallbackContext<'a, C> {
    queue: &'a mut ResidentRequestQueue<C>,
    domain: &'a Arc<()>,
    active_id: CollectionId,
    active_generation: Option<u64>,
    active_ref_count: Option<&'a mut u32>,
    wrapper_is_published: bool,
}

impl<C> ResidentCallbackContext<'_, C> {
    /// Returns the full-width collection ID whose resident callback is active.
    ///
    /// This is an observation only: it does not require a resident record and
    /// does not mutate the loaded count. Outer notification bridges can use it
    /// to reject a mismatched prepared collection before the native-order
    /// increment is attempted.
    #[must_use]
    pub const fn active_collection_id(&self) -> CollectionId {
        self.active_id
    }

    /// Applies `UpCollectionLoadedCount` to the collection whose callback is
    /// currently running, preserving native `u32` wrapping.
    ///
    /// # Errors
    ///
    /// Returns a typed invalid-domain error if the active request has no
    /// resident record at this callback point.
    pub fn up_active_collection_loaded_count(
        &mut self,
    ) -> Result<ResidentCollectionObservation, ResidentCallbackContextError> {
        self.increment_active_collection()
    }

    /// Acquires one explicit, occurrence-bound resident hold.
    ///
    /// This performs the same wrapping `UpCollectionLoadedCount` mutation as
    /// [`Self::up_active_collection_loaded_count`] and returns a non-cloneable
    /// capability for the matching later decrement. The token is constructed
    /// before the increment and cannot address a different streamer instance
    /// or a later reload of the same full-width collection ID.
    ///
    /// # Errors
    ///
    /// Returns the same missing-active-resident error as the unscoped native
    /// increment bridge.
    pub fn acquire_active_collection_hold(
        &mut self,
    ) -> Result<(ResidentCollectionObservation, ResidentCollectionHold), ResidentCallbackContextError>
    {
        let generation = self
            .active_generation
            .ok_or(ResidentCallbackContextError::MissingActiveResident { id: self.active_id })?;
        let hold = ResidentCollectionHold {
            domain: Arc::downgrade(self.domain),
            id: self.active_id,
            generation,
        };
        let observation = self.increment_active_collection()?;
        Ok((observation, hold))
    }

    fn increment_active_collection(
        &mut self,
    ) -> Result<ResidentCollectionObservation, ResidentCallbackContextError> {
        let Some(ref_count) = self.active_ref_count.as_deref_mut() else {
            return Err(ResidentCallbackContextError::MissingActiveResident { id: self.active_id });
        };
        *ref_count = ref_count.wrapping_add(1);
        Ok(ResidentCollectionObservation {
            ref_count: *ref_count,
            wrapper_is_published: self.wrapper_is_published,
        })
    }

    /// Appends a load behind the active request without exposing queue
    /// ownership or pop/clear operations.
    pub fn enqueue_load(&mut self, id: CollectionId, asset_count: i32, callback: Option<C>) {
        self.queue.push_load(id, asset_count, callback);
    }

    /// Appends an unload behind the active request without exposing queue
    /// ownership or pop/clear operations.
    pub fn enqueue_unload(&mut self, id: CollectionId, callback: Option<C>) {
        self.queue.push_unload(id, callback);
    }
}

/// Request callback. The context permits only the recovered active-resident
/// increment and native-style reentrant enqueue capabilities, without holding
/// a resident-table lock or aliasing the streamer itself.
pub trait ResidentCollectionCallback<E>: Sized {
    /// # Errors
    ///
    /// Reports an implementation-defined callback failure.
    fn on_complete(
        &mut self,
        id: CollectionId,
        observation: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, Self>,
    ) -> Result<(), E>;
}

/// Opaque collection registration metadata.
pub trait ResidentCollectionMetadata<E> {
    type Descriptor;

    /// # Errors
    ///
    /// Reports an implementation-defined metadata backend failure.
    fn find_collection(&mut self, id: CollectionId) -> Result<Option<Self::Descriptor>, E>;
}

/// Unique resident collection storage plus a non-owning load writer.
pub trait ResidentCollectionStorage<E> {
    type Collection;
    type Writer;

    /// # Errors
    ///
    /// Reports an implementation-defined allocation/storage failure.
    fn create_empty(&mut self, reserve: i32) -> Result<Self::Collection, E>;

    /// Produces a non-owning writer after `collection` has acquired its stable
    /// resident-table address. Implementations must not transfer ownership.
    ///
    /// # Errors
    ///
    /// Reports an implementation-defined writer-construction failure.
    fn writer(&mut self, collection: &Self::Collection) -> Result<Self::Writer, E>;

    /// # Errors
    ///
    /// Reports an implementation-defined collection-enumeration failure.
    fn asset_ids_in_order(&mut self, collection: &Self::Collection)
    -> Result<Vec<CollectionId>, E>;

    fn destroy(&mut self, collection: Self::Collection);
}

/// Result-bearing physical collection-load request.
pub trait ResidentCollectionLoadDriver<E> {
    /// # Errors
    ///
    /// Reports an implementation-defined pump/driver failure.
    fn update(&mut self) -> Result<(), E>;
    fn is_complete(&self) -> bool;
    fn highest_stored_block(&self) -> u32;
    fn loaded_size(&self) -> u32;
}

/// Starts the opaque `tLoadCollectionRequest`/`CollectionLoadPump` boundary.
pub trait ResidentCollectionLoadFactory<W, D, E> {
    type Driver: ResidentCollectionLoadDriver<E>;

    /// # Errors
    ///
    /// Reports an implementation-defined physical-load startup failure.
    fn begin(
        &mut self,
        id: CollectionId,
        asset_count: i32,
        descriptor: D,
        writer: W,
    ) -> Result<Self::Driver, E>;
}

/// Stable wrapper ownership and global asset-registry aliasing.
pub trait ResidentCollectionRegistry<C, E> {
    /// Uniquely owned wrapper value. It must reference `C` non-owningly; the
    /// resident table remains the collection's sole owner.
    type Wrapper;
    /// Non-owning registry identity. It must not retain ownership of either
    /// the wrapper or its resident collection.
    type Token;

    /// # Errors
    ///
    /// Reports an implementation-defined wrapper-construction failure.
    fn create_wrapper(&mut self, id: CollectionId, collection: &C) -> Result<Self::Wrapper, E>;
    /// Derives the side-effect-free identity used by registry publication and
    /// exact removal after the wrapper has its stable boxed address.
    fn stable_token(wrapper: &Self::Wrapper) -> Self::Token;
    /// # Errors
    ///
    /// Reports an implementation-defined registry publication failure.
    fn publish(&mut self, id: CollectionId, token: &Self::Token) -> Result<(), E>;
    /// # Errors
    ///
    /// Reports an implementation-defined registry removal/identity failure.
    fn remove_exact(&mut self, id: CollectionId, token: &Self::Token) -> Result<(), E>;
}

/// Per-occurrence asset garbage collection during zero-count destruction.
pub trait ResidentAssetGarbageCollector<E> {
    /// # Errors
    ///
    /// Reports an implementation-defined garbage-collection failure.
    fn garbage_collect(&mut self, id: CollectionId) -> Result<(), E>;
}

/// Opaque native `StartDecompress` scheduling boundary.
pub trait ResidentDecompressionKick<E> {
    /// # Errors
    ///
    /// Reports an implementation-defined scheduler-kick failure.
    fn start_decompress(&mut self) -> Result<(), E>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentBackendStage {
    CreateCollection,
    FindMetadata,
    CreateWriter,
    BeginLoad,
    UpdateLoad,
    CreateWrapper,
    PublishWrapper,
    Callback,
    StartDecompress,
    ReadCollectionAssets,
    GarbageCollectAsset,
    RemoveWrapper,
}

/// Typed safe-host replacement for native assertion and invalid-memory paths.
#[derive(Debug)]
pub enum CollectionResidentStreamerError<E> {
    ResidentGenerationExhausted {
        id: CollectionId,
    },
    ResidentCapacityExceeded {
        id: CollectionId,
    },
    MissingCollectionMetadata {
        id: CollectionId,
    },
    MissingResidentCollection {
        id: CollectionId,
    },
    MissingPublishedWrapper {
        id: CollectionId,
    },
    Backend {
        stage: ResidentBackendStage,
        source: E,
    },
}

impl<E: fmt::Display> fmt::Display for CollectionResidentStreamerError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResidentGenerationExhausted { id } => write!(
                formatter,
                "resident generation space is exhausted before loading {:#018x}",
                id.0
            ),
            Self::ResidentCapacityExceeded { id } => {
                write!(
                    formatter,
                    "resident collection map is full for {:#018x}",
                    id.0
                )
            }
            Self::MissingCollectionMetadata { id } => {
                write!(formatter, "collection {:#018x} has no metadata", id.0)
            }
            Self::MissingResidentCollection { id } => {
                write!(formatter, "collection {:#018x} is not resident", id.0)
            }
            Self::MissingPublishedWrapper { id } => {
                write!(
                    formatter,
                    "collection {:#018x} has no published wrapper",
                    id.0
                )
            }
            Self::Backend { stage, source } => write!(formatter, "{stage:?} failed: {source}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for CollectionResidentStreamerError<E> {}

pub type CollectionResidentStreamerResult<T, E> =
    Result<T, Arc<CollectionResidentStreamerError<E>>>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentStreamerUpdate {
    pub cache_hits: u32,
    pub unloads_processed: u32,
    pub physical_loads_started: u32,
    pub physical_loads_completed: u32,
    pub decompression_kicks: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentCollectionObservation {
    pub ref_count: u32,
    pub wrapper_is_published: bool,
}

/// Non-cloneable authority for one additional resident-collection hold.
///
/// The token is bound to both the streamer instance and the resident
/// occurrence's nonreused generation. Dropping it is deliberately inert: the
/// recovered manager releases a hold only from an explicit subsystem
/// completion callback. Domain and generation are a high-confidence host
/// authority mechanism, not fields claimed for the native collection record.
pub struct ResidentCollectionHold {
    domain: Weak<()>,
    id: CollectionId,
    generation: u64,
}

impl ResidentCollectionHold {
    #[must_use]
    pub const fn id(&self) -> CollectionId {
        self.id
    }
}

impl fmt::Debug for ResidentCollectionHold {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResidentCollectionHold")
            .field("id", &self.id)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentCollectionHoldAuthorityError {
    ForeignDomain,
    UnknownOccurrence,
}

/// Hold release failure.
///
/// Authority failures return the untouched token so its owning coordinator
/// may retry against the correct streamer. A pre-existing terminal streamer
/// fault also returns the untouched token because no decrement was attempted.
/// Once a valid hold reaches the native-shaped decrement, it is consumed even
/// if zero-count teardown later enters the streamer's permanent safe-host fault
/// state.
#[derive(Debug)]
pub enum ResidentCollectionHoldReleaseError<E> {
    Authority {
        kind: ResidentCollectionHoldAuthorityError,
        hold: ResidentCollectionHold,
    },
    StreamerUnavailable {
        source: Arc<CollectionResidentStreamerError<E>>,
        hold: ResidentCollectionHold,
    },
    Streamer(Arc<CollectionResidentStreamerError<E>>),
}

impl<E> ResidentCollectionHoldReleaseError<E> {
    #[must_use]
    pub fn into_hold(self) -> Option<ResidentCollectionHold> {
        match self {
            Self::Authority { hold, .. } | Self::StreamerUnavailable { hold, .. } => Some(hold),
            Self::Streamer(_) => None,
        }
    }
}

impl<E: fmt::Display> fmt::Display for ResidentCollectionHoldReleaseError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authority { kind, hold } => write!(
                formatter,
                "cannot release resident hold for {:#018x}: {kind:?}",
                hold.id.0
            ),
            Self::StreamerUnavailable { source, .. } | Self::Streamer(source) => {
                source.fmt(formatter)
            }
        }
    }
}

impl<E> std::error::Error for ResidentCollectionHoldReleaseError<E>
where
    E: fmt::Debug + fmt::Display + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::StreamerUnavailable { source, .. } | Self::Streamer(source) => Some(source),
            Self::Authority { .. } => None,
        }
    }
}

/// Ownership retained when zero-count teardown entered an unrecovered error
/// after the resident map entry had already been erased.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetainedTeardownObservation<'a> {
    pub id: CollectionId,
    pub asset_ids: &'a [CollectionId],
    pub next_asset: usize,
}

struct PublishedWrapper<W, T> {
    owner: Box<W>,
    token: T,
    published: bool,
}

struct ResidentEntry<C, W, T> {
    id: CollectionId,
    generation: u64,
    collection: Box<C>,
    wrapper: Option<PublishedWrapper<W, T>>,
    ref_count: u32,
}

enum ActiveRequest<C, D> {
    Queued(ResidentCollectionRequest<C>),
    Physical {
        id: CollectionId,
        callback: Option<C>,
        driver: D,
    },
}

impl<C, D> ActiveRequest<C, D> {
    fn id(&self) -> CollectionId {
        match self {
            Self::Queued(request) => request.id(),
            Self::Physical { id, .. } => *id,
        }
    }

    fn callback_mut(&mut self) -> &mut Option<C> {
        match self {
            Self::Queued(request) => request.callback_mut(),
            Self::Physical { callback, .. } => callback,
        }
    }
}

struct FailedTeardown<C, W, T> {
    entry: ResidentEntry<C, W, T>,
    asset_ids: Vec<CollectionId>,
    next_asset: usize,
}

/// Typed resident half of `assman::cAssetStreamer`.
///
/// Exact control-flow anchors are `Update` at `0x82d60ec0`,
/// `UpCollectionLoadedCount` at `0x82d57d20`, and
/// `DownCollectionLoadedCount` at `0x82d57de0`. Permanent typed first-fault
/// replay replaces native asserts/null dereferences and is a high-confidence
/// safe policy. Dropping a streamer with live resident identities deliberately
/// leaks those identities rather than fabricating fallible native unload work
/// or leaving external registry aliases dangling; callers should drive proven
/// unload requests to zero for normal cleanup.
pub struct CollectionResidentStreamer<S, M, F, R, G, K, C, E>
where
    S: ResidentCollectionStorage<E>,
    M: ResidentCollectionMetadata<E>,
    F: ResidentCollectionLoadFactory<S::Writer, M::Descriptor, E>,
    R: ResidentCollectionRegistry<S::Collection, E>,
    G: ResidentAssetGarbageCollector<E>,
    K: ResidentDecompressionKick<E>,
    C: ResidentCollectionCallback<E>,
{
    storage: S,
    metadata: M,
    load_factory: F,
    registry: R,
    garbage_collector: G,
    decompression: K,
    queue: ResidentRequestQueue<C>,
    active: Option<ActiveRequest<C, F::Driver>>,
    residents: Vec<ResidentEntry<S::Collection, R::Wrapper, R::Token>>,
    failed_teardown: Option<FailedTeardown<S::Collection, R::Wrapper, R::Token>>,
    highest_stream_block_size: u32,
    terminal_error: Option<Arc<CollectionResidentStreamerError<E>>>,
    domain: Arc<()>,
    next_resident_generation: u64,
    error_marker: PhantomData<fn() -> E>,
}

impl<S, M, F, R, G, K, C, E> CollectionResidentStreamer<S, M, F, R, G, K, C, E>
where
    S: ResidentCollectionStorage<E>,
    M: ResidentCollectionMetadata<E>,
    F: ResidentCollectionLoadFactory<S::Writer, M::Descriptor, E>,
    R: ResidentCollectionRegistry<S::Collection, E>,
    G: ResidentAssetGarbageCollector<E>,
    K: ResidentDecompressionKick<E>,
    C: ResidentCollectionCallback<E>,
{
    #[must_use]
    pub fn new(
        storage: S,
        metadata: M,
        load_factory: F,
        registry: R,
        garbage_collector: G,
        decompression: K,
    ) -> Self {
        Self {
            storage,
            metadata,
            load_factory,
            registry,
            garbage_collector,
            decompression,
            queue: ResidentRequestQueue::default(),
            active: None,
            residents: Vec::new(),
            failed_teardown: None,
            highest_stream_block_size: 0,
            terminal_error: None,
            domain: Arc::new(()),
            next_resident_generation: 0,
            error_marker: PhantomData,
        }
    }

    pub fn enqueue_load(&mut self, id: CollectionId, asset_count: i32, callback: Option<C>) {
        self.queue.push_load(id, asset_count, callback);
    }

    pub fn enqueue_unload(&mut self, id: CollectionId, callback: Option<C>) {
        self.queue.push_unload(id, callback);
    }

    #[must_use]
    pub fn pending_requests(&self) -> bool {
        self.active.is_some() || !self.queue.is_empty()
    }

    #[must_use]
    pub fn queued_request_count(&self) -> usize {
        self.queue.len()
    }

    #[must_use]
    pub fn active_id(&self) -> Option<CollectionId> {
        self.active.as_ref().map(ActiveRequest::id)
    }

    #[must_use]
    pub fn resident_count(&self) -> usize {
        self.residents.len()
    }

    #[must_use]
    pub const fn highest_stream_block_size(&self) -> u32 {
        self.highest_stream_block_size
    }

    #[must_use]
    pub fn terminal_error(&self) -> Option<&Arc<CollectionResidentStreamerError<E>>> {
        self.terminal_error.as_ref()
    }

    #[must_use]
    pub fn has_retained_failed_teardown(&self) -> bool {
        self.failed_teardown.is_some()
    }

    #[must_use]
    pub fn retained_failed_teardown(&self) -> Option<RetainedTeardownObservation<'_>> {
        self.failed_teardown
            .as_ref()
            .map(|teardown| RetainedTeardownObservation {
                id: teardown.entry.id,
                asset_ids: &teardown.asset_ids,
                next_asset: teardown.next_asset,
            })
    }

    #[must_use]
    pub fn observe_resident(&self, id: CollectionId) -> Option<ResidentCollectionObservation> {
        self.find_resident(id)
            .map(|entry| ResidentCollectionObservation {
                ref_count: entry.ref_count,
                wrapper_is_published: entry
                    .wrapper
                    .as_ref()
                    .is_some_and(|wrapper| wrapper.published),
            })
    }

    /// Native `GetLoadedSize` behavior for the active physical load only.
    #[must_use]
    pub fn active_loaded_size(&self, id: CollectionId) -> u32 {
        match self.active.as_ref() {
            Some(ActiveRequest::Physical {
                id: active_id,
                driver,
                ..
            }) if *active_id == id => driver.loaded_size(),
            _ => 0,
        }
    }

    /// Runs one native outer update.
    ///
    /// # Errors
    ///
    /// Returns and permanently replays the first safe-host replacement for a
    /// native assertion/unsafe condition or collaborator failure.
    pub fn update(&mut self) -> CollectionResidentStreamerResult<ResidentStreamerUpdate, E> {
        self.check_terminal()?;
        let mut outcome = ResidentStreamerUpdate::default();

        if matches!(self.active, Some(ActiveRequest::Physical { .. })) {
            self.update_active_physical(&mut outcome)?;
            return Ok(outcome);
        }

        loop {
            if self.active.is_none() {
                let Some(request) = self.queue.pop_front() else {
                    return Ok(outcome);
                };
                self.active = Some(ActiveRequest::Queued(request));
            }

            let request_kind = match self.active.as_ref() {
                Some(ActiveRequest::Queued(request)) => request.kind(),
                Some(ActiveRequest::Physical { .. }) => return Ok(outcome),
                None => unreachable!("active request was installed above"),
            };

            match request_kind {
                ResidentCollectionRequestKind::Load => {
                    if self.process_queued_load(&mut outcome)? {
                        continue;
                    }
                    return Ok(outcome);
                }
                ResidentCollectionRequestKind::Unload => {
                    self.process_queued_unload(&mut outcome)?;
                }
            }
        }
    }

    /// Exact wrapping resident-count increment.
    ///
    /// # Errors
    ///
    /// Latches a missing resident entry, matching the native assert domain.
    pub fn up_collection_loaded_count(
        &mut self,
        id: CollectionId,
    ) -> CollectionResidentStreamerResult<ResidentCollectionObservation, E> {
        self.check_terminal()?;
        let Some(index) = self.find_resident_index(id) else {
            return self.latch(CollectionResidentStreamerError::MissingResidentCollection { id });
        };
        let entry = &mut self.residents[index];
        entry.ref_count = entry.ref_count.wrapping_add(1);
        Ok(ResidentCollectionObservation {
            ref_count: entry.ref_count,
            wrapper_is_published: entry
                .wrapper
                .as_ref()
                .is_some_and(|wrapper| wrapper.published),
        })
    }

    /// Exact wrapping resident-count decrement and zero-count teardown.
    /// Delayed asynchronous subsystem completions should instead consume a
    /// [`ResidentCollectionHold`] so an ID reuse cannot target a later resident
    /// occurrence.
    ///
    /// # Errors
    ///
    /// Latches missing entries, missing wrappers, or collaborator errors.
    pub fn down_collection_loaded_count(
        &mut self,
        id: CollectionId,
    ) -> CollectionResidentStreamerResult<bool, E> {
        self.check_terminal()?;
        self.down_collection_loaded_count_inner(id)
    }

    /// Uses one host streamer-and-occurrence-bound capability to perform the
    /// exact underlying resident decrement.
    ///
    /// Foreign or stale capabilities are rejected without decrementing and
    /// are returned intact. A valid capability is consumed once the decrement
    /// begins; if zero-count teardown then faults, the streamer's existing
    /// permanent error records the mutation prefix and no retry is invented.
    ///
    /// # Errors
    ///
    /// Returns the untouched token for a foreign streamer, an absent/reloaded
    /// occurrence, or a pre-existing permanent streamer fault. A fault entered
    /// during a valid decrement consumes the token and preserves that exact
    /// mutation prefix.
    pub fn release_collection_hold(
        &mut self,
        hold: ResidentCollectionHold,
    ) -> Result<bool, ResidentCollectionHoldReleaseError<E>> {
        if !Weak::ptr_eq(&hold.domain, &Arc::downgrade(&self.domain)) {
            return Err(ResidentCollectionHoldReleaseError::Authority {
                kind: ResidentCollectionHoldAuthorityError::ForeignDomain,
                hold,
            });
        }
        if !self
            .residents
            .iter()
            .any(|entry| entry.id == hold.id && entry.generation == hold.generation)
        {
            return Err(ResidentCollectionHoldReleaseError::Authority {
                kind: ResidentCollectionHoldAuthorityError::UnknownOccurrence,
                hold,
            });
        }
        if let Some(source) = &self.terminal_error {
            return Err(ResidentCollectionHoldReleaseError::StreamerUnavailable {
                source: Arc::clone(source),
                hold,
            });
        }
        self.down_collection_loaded_count_inner(hold.id)
            .map_err(ResidentCollectionHoldReleaseError::Streamer)
    }

    fn process_queued_load(
        &mut self,
        outcome: &mut ResidentStreamerUpdate,
    ) -> CollectionResidentStreamerResult<bool, E> {
        let (id, asset_count) = match self.active.as_ref() {
            Some(ActiveRequest::Queued(ResidentCollectionRequest::Load {
                id,
                asset_count,
                ..
            })) => (*id, *asset_count),
            _ => unreachable!("load dispatcher requires a queued load"),
        };

        if let Some(index) = self.find_resident_index(id) {
            self.residents[index].ref_count = self.residents[index].ref_count.wrapping_add(1);
            outcome.cache_hits = outcome.cache_hits.wrapping_add(1);
            self.invoke_active_callback()?;
            self.active = None;
            return Ok(true);
        }

        if self.residents.len() >= RESIDENT_COLLECTION_CAPACITY {
            return self.latch(CollectionResidentStreamerError::ResidentCapacityExceeded { id });
        }

        let generation = self.next_resident_generation;
        let Some(next_generation) = self.next_resident_generation.checked_add(1) else {
            return self.latch(CollectionResidentStreamerError::ResidentGenerationExhausted { id });
        };
        self.next_resident_generation = next_generation;

        let collection = match self.storage.create_empty(asset_count) {
            Ok(collection) => collection,
            Err(source) => {
                return self.latch_backend(ResidentBackendStage::CreateCollection, source);
            }
        };
        let descriptor = match self.metadata.find_collection(id) {
            Ok(Some(descriptor)) => descriptor,
            Ok(None) => {
                self.storage.destroy(collection);
                return self
                    .latch(CollectionResidentStreamerError::MissingCollectionMetadata { id });
            }
            Err(source) => {
                self.storage.destroy(collection);
                return self.latch_backend(ResidentBackendStage::FindMetadata, source);
            }
        };

        self.residents.push(ResidentEntry {
            id,
            generation,
            collection: Box::new(collection),
            wrapper: None,
            ref_count: 1,
        });
        let index = self.residents.len() - 1;
        let writer = match self.storage.writer(&self.residents[index].collection) {
            Ok(writer) => writer,
            Err(source) => return self.latch_backend(ResidentBackendStage::CreateWriter, source),
        };
        let driver = match self.load_factory.begin(id, asset_count, descriptor, writer) {
            Ok(driver) => driver,
            Err(source) => return self.latch_backend(ResidentBackendStage::BeginLoad, source),
        };

        let callback = match self.active.as_mut() {
            Some(ActiveRequest::Queued(request)) => request.callback_mut().take(),
            _ => unreachable!("physical load still owns its queued request"),
        };
        self.active = Some(ActiveRequest::Physical {
            id,
            callback,
            driver,
        });
        outcome.physical_loads_started = outcome.physical_loads_started.wrapping_add(1);
        Ok(false)
    }

    fn process_queued_unload(
        &mut self,
        outcome: &mut ResidentStreamerUpdate,
    ) -> CollectionResidentStreamerResult<(), E> {
        let id = self.active.as_ref().expect("queued unload is active").id();
        self.invoke_active_callback()?;
        self.down_collection_loaded_count_inner(id)?;
        outcome.unloads_processed = outcome.unloads_processed.wrapping_add(1);
        self.active = None;
        Ok(())
    }

    fn update_active_physical(
        &mut self,
        outcome: &mut ResidentStreamerUpdate,
    ) -> CollectionResidentStreamerResult<(), E> {
        let update_result = match self.active.as_mut() {
            Some(ActiveRequest::Physical { driver, .. }) => driver.update(),
            _ => unreachable!("physical updater requires a physical request"),
        };
        if let Err(source) = update_result {
            return self.latch_backend(ResidentBackendStage::UpdateLoad, source);
        }

        let complete = match self.active.as_ref() {
            Some(ActiveRequest::Physical { driver, .. }) => driver.is_complete(),
            _ => unreachable!("physical request remains active after update"),
        };
        if !complete {
            self.kick_decompression(outcome)?;
            return Ok(());
        }

        let (id, highest) = match self.active.as_ref() {
            Some(ActiveRequest::Physical { id, driver, .. }) => {
                (*id, driver.highest_stored_block())
            }
            _ => unreachable!("physical request remains active after update"),
        };
        if highest > self.highest_stream_block_size {
            self.highest_stream_block_size = highest;
        }

        let Some(index) = self.find_resident_index(id) else {
            return self.latch(CollectionResidentStreamerError::MissingResidentCollection { id });
        };
        let wrapper = match self
            .registry
            .create_wrapper(id, &self.residents[index].collection)
        {
            Ok(wrapper) => wrapper,
            Err(source) => return self.latch_backend(ResidentBackendStage::CreateWrapper, source),
        };
        let wrapper = Box::new(wrapper);
        let token = R::stable_token(&wrapper);
        self.residents[index].wrapper = Some(PublishedWrapper {
            owner: wrapper,
            token,
            published: false,
        });
        let publication = self.residents[index]
            .wrapper
            .as_ref()
            .expect("wrapper was installed before publication");
        if let Err(source) = self.registry.publish(id, &publication.token) {
            return self.latch_backend(ResidentBackendStage::PublishWrapper, source);
        }
        self.residents[index]
            .wrapper
            .as_mut()
            .expect("wrapper remains installed after publication")
            .published = true;

        self.invoke_active_callback()?;
        self.active = None;
        outcome.physical_loads_completed = outcome.physical_loads_completed.wrapping_add(1);
        self.kick_decompression(outcome)
    }

    fn invoke_active_callback(&mut self) -> CollectionResidentStreamerResult<(), E> {
        let id = self
            .active
            .as_ref()
            .expect("callback requires an active request")
            .id();
        let observation = self.callback_observation(id);
        let callback = self
            .active
            .as_mut()
            .expect("callback requires an active request")
            .callback_mut()
            .take();
        let Some(mut callback) = callback else {
            return Ok(());
        };

        let active_index = self.find_resident_index(id);
        let active_generation = active_index.map(|index| self.residents[index].generation);
        let active_ref_count = active_index.map(|index| &mut self.residents[index].ref_count);
        let result = {
            let mut context = ResidentCallbackContext {
                queue: &mut self.queue,
                domain: &self.domain,
                active_id: id,
                active_generation,
                active_ref_count,
                wrapper_is_published: observation.wrapper_is_published,
            };
            callback.on_complete(id, observation, &mut context)
        };
        self.active
            .as_mut()
            .expect("callback cannot clear the active request")
            .callback_mut()
            .replace(callback);
        if let Err(source) = result {
            return self.latch_backend(ResidentBackendStage::Callback, source);
        }
        Ok(())
    }

    fn callback_observation(&self, id: CollectionId) -> ResidentCallbackObservation {
        let resident = self.observe_resident(id);
        ResidentCallbackObservation {
            active_id: id,
            resident_ref_count: resident.map(|entry| entry.ref_count),
            wrapper_is_published: resident.is_some_and(|entry| entry.wrapper_is_published),
        }
    }

    fn kick_decompression(
        &mut self,
        outcome: &mut ResidentStreamerUpdate,
    ) -> CollectionResidentStreamerResult<(), E> {
        if let Err(source) = self.decompression.start_decompress() {
            return self.latch_backend(ResidentBackendStage::StartDecompress, source);
        }
        outcome.decompression_kicks = outcome.decompression_kicks.wrapping_add(1);
        Ok(())
    }

    fn down_collection_loaded_count_inner(
        &mut self,
        id: CollectionId,
    ) -> CollectionResidentStreamerResult<bool, E> {
        let Some(index) = self.find_resident_index(id) else {
            return self.latch(CollectionResidentStreamerError::MissingResidentCollection { id });
        };
        let new_count = self.residents[index].ref_count.wrapping_sub(1);
        self.residents[index].ref_count = new_count;
        if new_count != 0 {
            return Ok(false);
        }

        let mut entry = self.residents.remove(index);
        let asset_ids = match self.storage.asset_ids_in_order(&entry.collection) {
            Ok(asset_ids) => asset_ids,
            Err(source) => {
                self.failed_teardown = Some(FailedTeardown {
                    entry,
                    asset_ids: Vec::new(),
                    next_asset: 0,
                });
                return self.latch_backend(ResidentBackendStage::ReadCollectionAssets, source);
            }
        };

        for (next_asset, asset_id) in asset_ids.iter().copied().enumerate() {
            if let Err(source) = self.garbage_collector.garbage_collect(asset_id) {
                self.failed_teardown = Some(FailedTeardown {
                    entry,
                    asset_ids,
                    next_asset,
                });
                return self.latch_backend(ResidentBackendStage::GarbageCollectAsset, source);
            }
        }

        let Some(publication) = entry.wrapper.as_ref() else {
            let next_asset = asset_ids.len();
            self.failed_teardown = Some(FailedTeardown {
                entry,
                asset_ids,
                next_asset,
            });
            return self.latch(CollectionResidentStreamerError::MissingPublishedWrapper { id });
        };
        if let Err(source) = self.registry.remove_exact(id, &publication.token) {
            let next_asset = asset_ids.len();
            self.failed_teardown = Some(FailedTeardown {
                entry,
                asset_ids,
                next_asset,
            });
            return self.latch_backend(ResidentBackendStage::RemoveWrapper, source);
        }

        let publication = entry.wrapper.take().expect("published wrapper was checked");
        drop(publication.owner);
        drop(publication.token);
        self.storage.destroy(*entry.collection);
        Ok(true)
    }

    fn find_resident_index(&self, id: CollectionId) -> Option<usize> {
        self.residents.iter().position(|entry| entry.id == id)
    }

    fn find_resident(
        &self,
        id: CollectionId,
    ) -> Option<&ResidentEntry<S::Collection, R::Wrapper, R::Token>> {
        self.find_resident_index(id)
            .map(|index| &self.residents[index])
    }

    fn check_terminal(&self) -> CollectionResidentStreamerResult<(), E> {
        match self.terminal_error.as_ref() {
            Some(error) => Err(Arc::clone(error)),
            None => Ok(()),
        }
    }

    fn latch<T>(
        &mut self,
        error: CollectionResidentStreamerError<E>,
    ) -> CollectionResidentStreamerResult<T, E> {
        let error = Arc::new(error);
        self.terminal_error = Some(Arc::clone(&error));
        Err(error)
    }

    fn latch_backend<T>(
        &mut self,
        stage: ResidentBackendStage,
        source: E,
    ) -> CollectionResidentStreamerResult<T, E> {
        self.latch(CollectionResidentStreamerError::Backend { stage, source })
    }
}

impl<S, M, F, R, G, K, C, E> Drop for CollectionResidentStreamer<S, M, F, R, G, K, C, E>
where
    S: ResidentCollectionStorage<E>,
    M: ResidentCollectionMetadata<E>,
    F: ResidentCollectionLoadFactory<S::Writer, M::Descriptor, E>,
    R: ResidentCollectionRegistry<S::Collection, E>,
    G: ResidentAssetGarbageCollector<E>,
    K: ResidentDecompressionKick<E>,
    C: ResidentCollectionCallback<E>,
{
    fn drop(&mut self) {
        // Native cleanup is request-driven and can invoke externally visible,
        // result-bearing garbage and registry collaborators. Implicit Rust
        // drop has no evidence-backed way to execute or recover that protocol.
        // Deliberately leak resident identities so an external registry cannot
        // retain dangling aliases. This is a high-confidence abandonment
        // policy, not a claim about native shutdown behavior.
        for entry in self.residents.drain(..) {
            mem::forget(entry);
        }
        if let Some(teardown) = self.failed_teardown.take() {
            mem::forget(teardown);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        Create(i32),
        Metadata(CollectionId),
        Writer(usize),
        Begin(CollectionId, i32, usize),
        DriverUpdate(CollectionId),
        WrapperCreate(CollectionId, usize),
        Publish(CollectionId, usize),
        Callback(CollectionId, ResidentCallbackObservation),
        Kick,
        AssetIds(CollectionId),
        Garbage(CollectionId),
        Remove(CollectionId, usize),
        WrapperDrop(CollectionId),
        Destroy(CollectionId),
        CollectionDrop(CollectionId),
    }

    type Log = Rc<RefCell<Vec<Event>>>;

    struct TestCollection {
        identity: Rc<()>,
        id: Cell<Option<CollectionId>>,
        assets: RefCell<Vec<CollectionId>>,
        log: Log,
    }

    impl Drop for TestCollection {
        fn drop(&mut self) {
            self.log.borrow_mut().push(Event::CollectionDrop(
                self.id.get().unwrap_or(CollectionId(0)),
            ));
        }
    }

    #[derive(Clone)]
    struct TestWriter {
        identity: Rc<()>,
    }

    #[derive(Clone)]
    struct TestDescriptor;

    struct TestStorage {
        log: Log,
        next_assets: VecDeque<Vec<CollectionId>>,
        fail_create: bool,
        fail_writer: bool,
        fail_assets: bool,
    }

    impl ResidentCollectionStorage<TestError> for TestStorage {
        type Collection = TestCollection;
        type Writer = TestWriter;

        fn create_empty(&mut self, reserve: i32) -> Result<Self::Collection, TestError> {
            self.log.borrow_mut().push(Event::Create(reserve));
            if self.fail_create {
                return Err(TestError("create"));
            }
            Ok(TestCollection {
                identity: Rc::new(()),
                id: Cell::new(None),
                assets: RefCell::new(self.next_assets.pop_front().unwrap_or_default()),
                log: Rc::clone(&self.log),
            })
        }

        fn writer(&mut self, collection: &Self::Collection) -> Result<Self::Writer, TestError> {
            let identity = Rc::as_ptr(&collection.identity) as usize;
            self.log.borrow_mut().push(Event::Writer(identity));
            if self.fail_writer {
                return Err(TestError("writer"));
            }
            Ok(TestWriter {
                identity: Rc::clone(&collection.identity),
            })
        }

        fn asset_ids_in_order(
            &mut self,
            collection: &Self::Collection,
        ) -> Result<Vec<CollectionId>, TestError> {
            let id = collection.id.get().expect("published collection has an ID");
            self.log.borrow_mut().push(Event::AssetIds(id));
            if self.fail_assets {
                return Err(TestError("assets"));
            }
            Ok(collection.assets.borrow().clone())
        }

        fn destroy(&mut self, collection: Self::Collection) {
            self.log.borrow_mut().push(Event::Destroy(
                collection.id.get().unwrap_or(CollectionId(0)),
            ));
        }
    }

    struct TestMetadata {
        log: Log,
        missing: Vec<CollectionId>,
        fail: bool,
    }

    impl ResidentCollectionMetadata<TestError> for TestMetadata {
        type Descriptor = TestDescriptor;

        fn find_collection(
            &mut self,
            id: CollectionId,
        ) -> Result<Option<Self::Descriptor>, TestError> {
            self.log.borrow_mut().push(Event::Metadata(id));
            if self.fail {
                return Err(TestError("metadata"));
            }
            Ok((!self.missing.contains(&id)).then_some(TestDescriptor))
        }
    }

    #[derive(Clone, Copy)]
    struct DriverPlan {
        updates_until_complete: u32,
        highest: u32,
        loaded: u32,
    }

    struct TestDriver {
        id: CollectionId,
        remaining: u32,
        highest: u32,
        loaded: u32,
        log: Log,
        fail_update: bool,
    }

    impl ResidentCollectionLoadDriver<TestError> for TestDriver {
        fn update(&mut self) -> Result<(), TestError> {
            self.log.borrow_mut().push(Event::DriverUpdate(self.id));
            if self.fail_update {
                return Err(TestError("driver update"));
            }
            self.remaining = self.remaining.saturating_sub(1);
            Ok(())
        }

        fn is_complete(&self) -> bool {
            self.remaining == 0
        }

        fn highest_stored_block(&self) -> u32 {
            self.highest
        }

        fn loaded_size(&self) -> u32 {
            self.loaded
        }
    }

    struct TestFactory {
        log: Log,
        plans: VecDeque<DriverPlan>,
        fail_begin: bool,
        fail_update: bool,
        writer_identities: Rc<RefCell<HashMap<CollectionId, usize>>>,
    }

    impl ResidentCollectionLoadFactory<TestWriter, TestDescriptor, TestError> for TestFactory {
        type Driver = TestDriver;

        fn begin(
            &mut self,
            id: CollectionId,
            asset_count: i32,
            _descriptor: TestDescriptor,
            writer: TestWriter,
        ) -> Result<Self::Driver, TestError> {
            let identity = Rc::as_ptr(&writer.identity) as usize;
            self.log
                .borrow_mut()
                .push(Event::Begin(id, asset_count, identity));
            self.writer_identities.borrow_mut().insert(id, identity);
            if self.fail_begin {
                return Err(TestError("begin"));
            }
            let plan = self.plans.pop_front().unwrap_or(DriverPlan {
                updates_until_complete: 1,
                highest: 0,
                loaded: 0,
            });
            Ok(TestDriver {
                id,
                remaining: plan.updates_until_complete,
                highest: plan.highest,
                loaded: plan.loaded,
                log: Rc::clone(&self.log),
                fail_update: self.fail_update,
            })
        }
    }

    struct WrapperDrop {
        id: CollectionId,
        log: Log,
    }

    impl Drop for WrapperDrop {
        fn drop(&mut self) {
            self.log.borrow_mut().push(Event::WrapperDrop(self.id));
        }
    }

    struct TestWrapper {
        id: CollectionId,
        collection_identity: usize,
        _drop: WrapperDrop,
    }

    struct TestRegistry {
        log: Log,
        fail_stage: Option<ResidentBackendStage>,
        published: HashMap<CollectionId, usize>,
    }

    impl ResidentCollectionRegistry<TestCollection, TestError> for TestRegistry {
        type Wrapper = TestWrapper;
        type Token = usize;

        fn create_wrapper(
            &mut self,
            id: CollectionId,
            collection: &TestCollection,
        ) -> Result<Self::Wrapper, TestError> {
            let identity = Rc::as_ptr(&collection.identity) as usize;
            self.log
                .borrow_mut()
                .push(Event::WrapperCreate(id, identity));
            if self.fail_stage == Some(ResidentBackendStage::CreateWrapper) {
                return Err(TestError("wrapper"));
            }
            collection.id.set(Some(id));
            Ok(TestWrapper {
                id,
                collection_identity: identity,
                _drop: WrapperDrop {
                    id,
                    log: Rc::clone(&self.log),
                },
            })
        }

        fn stable_token(wrapper: &Self::Wrapper) -> Self::Token {
            std::ptr::from_ref(wrapper) as usize
        }

        fn publish(&mut self, id: CollectionId, token: &Self::Token) -> Result<(), TestError> {
            self.log.borrow_mut().push(Event::Publish(id, *token));
            if self.fail_stage == Some(ResidentBackendStage::PublishWrapper) {
                return Err(TestError("publish"));
            }
            self.published.insert(id, *token);
            Ok(())
        }

        fn remove_exact(&mut self, id: CollectionId, token: &Self::Token) -> Result<(), TestError> {
            self.log.borrow_mut().push(Event::Remove(id, *token));
            if self.fail_stage == Some(ResidentBackendStage::RemoveWrapper)
                || self.published.get(&id) != Some(token)
            {
                return Err(TestError("remove"));
            }
            self.published.remove(&id);
            Ok(())
        }
    }

    struct TestGarbage {
        log: Log,
        fail_on: Option<CollectionId>,
    }

    impl ResidentAssetGarbageCollector<TestError> for TestGarbage {
        fn garbage_collect(&mut self, id: CollectionId) -> Result<(), TestError> {
            self.log.borrow_mut().push(Event::Garbage(id));
            if self.fail_on == Some(id) {
                return Err(TestError("garbage"));
            }
            Ok(())
        }
    }

    struct TestKick {
        log: Log,
        fail: bool,
    }

    impl ResidentDecompressionKick<TestError> for TestKick {
        fn start_decompress(&mut self) -> Result<(), TestError> {
            self.log.borrow_mut().push(Event::Kick);
            if self.fail {
                return Err(TestError("kick"));
            }
            Ok(())
        }
    }

    #[derive(Clone)]
    struct TestCallback {
        log: Log,
        enqueue: Option<(CollectionId, ResidentCollectionRequestKind)>,
        up_active_times: u32,
        acquired_holds: Option<(u32, Rc<RefCell<Vec<ResidentCollectionHold>>>)>,
        fail: bool,
    }

    impl ResidentCollectionCallback<TestError> for TestCallback {
        fn on_complete(
            &mut self,
            id: CollectionId,
            observation: ResidentCallbackObservation,
            context: &mut ResidentCallbackContext<'_, Self>,
        ) -> Result<(), TestError> {
            self.log.borrow_mut().push(Event::Callback(id, observation));
            for _ in 0..self.up_active_times {
                context
                    .up_active_collection_loaded_count()
                    .map_err(|_| TestError("missing active resident"))?;
            }
            if let Some((count, slot)) = &self.acquired_holds {
                for _ in 0..*count {
                    let (_, hold) = context
                        .acquire_active_collection_hold()
                        .map_err(|_| TestError("missing active resident"))?;
                    slot.borrow_mut().push(hold);
                }
            }
            if let Some((queued_id, kind)) = self.enqueue.take() {
                match kind {
                    ResidentCollectionRequestKind::Load => {
                        context.enqueue_load(queued_id, 0, None);
                    }
                    ResidentCollectionRequestKind::Unload => {
                        context.enqueue_unload(queued_id, None);
                    }
                }
            }
            if self.fail {
                return Err(TestError("callback"));
            }
            Ok(())
        }
    }

    type Streamer = CollectionResidentStreamer<
        TestStorage,
        TestMetadata,
        TestFactory,
        TestRegistry,
        TestGarbage,
        TestKick,
        TestCallback,
        TestError,
    >;

    fn streamer(plans: impl IntoIterator<Item = DriverPlan>) -> (Streamer, Log) {
        let log = Rc::new(RefCell::new(Vec::new()));
        let streamer = CollectionResidentStreamer::new(
            TestStorage {
                log: Rc::clone(&log),
                next_assets: VecDeque::new(),
                fail_create: false,
                fail_writer: false,
                fail_assets: false,
            },
            TestMetadata {
                log: Rc::clone(&log),
                missing: Vec::new(),
                fail: false,
            },
            TestFactory {
                log: Rc::clone(&log),
                plans: plans.into_iter().collect(),
                fail_begin: false,
                fail_update: false,
                writer_identities: Rc::new(RefCell::new(HashMap::new())),
            },
            TestRegistry {
                log: Rc::clone(&log),
                fail_stage: None,
                published: HashMap::new(),
            },
            TestGarbage {
                log: Rc::clone(&log),
                fail_on: None,
            },
            TestKick {
                log: Rc::clone(&log),
                fail: false,
            },
        );
        (streamer, log)
    }

    fn callback(log: &Log) -> TestCallback {
        TestCallback {
            log: Rc::clone(log),
            enqueue: None,
            up_active_times: 0,
            acquired_holds: None,
            fail: false,
        }
    }

    fn load_to_completion(
        streamer: &mut Streamer,
        id: CollectionId,
        callback: Option<TestCallback>,
    ) {
        streamer.enqueue_load(id, 0, callback);
        let start = streamer.update().unwrap();
        assert_eq!(start.physical_loads_started, 1);
        let complete = streamer.update().unwrap();
        assert_eq!(complete.physical_loads_completed, 1);
    }

    fn load_with_hold(
        streamer: &mut Streamer,
        log: &Log,
        id: CollectionId,
    ) -> ResidentCollectionHold {
        let slot = Rc::new(RefCell::new(Vec::new()));
        let mut load_callback = callback(log);
        load_callback.acquired_holds = Some((1, Rc::clone(&slot)));
        load_to_completion(streamer, id, Some(load_callback));
        slot.borrow_mut()
            .pop()
            .expect("load callback should acquire one resident hold")
    }

    #[test]
    fn semantic_request_words_are_exact_zero_and_one() {
        let load: ResidentCollectionRequest<TestCallback> = ResidentCollectionRequest::Load {
            id: CollectionId(1),
            asset_count: 0,
            callback: None,
        };
        let unload: ResidentCollectionRequest<TestCallback> = ResidentCollectionRequest::Unload {
            id: CollectionId(1),
            callback: None,
        };
        assert_eq!(load.raw_type(), 0);
        assert_eq!(unload.raw_type(), 1);
    }

    #[test]
    fn callback_context_up_wraps_only_the_active_resident() {
        let id = CollectionId(0x1234_5678_9abc_def0);
        let mut queue = ResidentRequestQueue::<TestCallback>::default();
        let mut ref_count = u32::MAX;
        let domain = Arc::new(());
        let mut context = ResidentCallbackContext {
            queue: &mut queue,
            domain: &domain,
            active_id: id,
            active_generation: Some(7),
            active_ref_count: Some(&mut ref_count),
            wrapper_is_published: true,
        };

        let observation = context.up_active_collection_loaded_count().unwrap();

        assert_eq!(observation.ref_count, 0);
        assert!(observation.wrapper_is_published);
        assert_eq!(ref_count, 0);
    }

    #[test]
    fn callback_context_missing_active_is_typed_and_does_not_enqueue() {
        let id = CollectionId(99);
        let mut queue = ResidentRequestQueue::<TestCallback>::default();
        let domain = Arc::new(());
        let mut context = ResidentCallbackContext {
            queue: &mut queue,
            domain: &domain,
            active_id: id,
            active_generation: None,
            active_ref_count: None,
            wrapper_is_published: false,
        };

        assert_eq!(context.active_collection_id(), id);

        let error = context.up_active_collection_loaded_count().unwrap_err();

        assert_eq!(
            error,
            ResidentCallbackContextError::MissingActiveResident { id }
        );
        let hold_error = context.acquire_active_collection_hold().unwrap_err();
        assert_eq!(
            hold_error,
            ResidentCallbackContextError::MissingActiveResident { id }
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn occurrence_bound_hold_survives_base_unload_and_explicit_release_destroys() {
        let id = CollectionId(0x1234_5678_9abc_def0);
        let (mut streamer, log) = streamer([]);
        let hold = load_with_hold(&mut streamer, &log, id);
        assert_eq!(hold.id(), id);
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 2);

        streamer.enqueue_unload(id, None);
        let update = streamer.update().unwrap();
        assert_eq!(update.unloads_processed, 1);
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 1);

        assert!(streamer.release_collection_hold(hold).unwrap());
        assert_eq!(streamer.observe_resident(id), None);
    }

    #[test]
    fn two_independent_holds_each_authorize_one_decrement() {
        let id = CollectionId(0x2222_3333_4444_5555);
        let (mut streamer, log) = streamer([]);
        let holds = Rc::new(RefCell::new(Vec::new()));
        let mut load_callback = callback(&log);
        load_callback.acquired_holds = Some((2, Rc::clone(&holds)));
        load_to_completion(&mut streamer, id, Some(load_callback));
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 3);
        streamer.enqueue_unload(id, None);
        streamer.update().unwrap();
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 2);
        let mut holds = holds.borrow_mut();
        let second = holds.pop().unwrap();
        let first = holds.pop().unwrap();
        drop(holds);

        assert!(!streamer.release_collection_hold(first).unwrap());
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 1);
        assert!(streamer.release_collection_hold(second).unwrap());
        assert_eq!(streamer.observe_resident(id), None);
    }

    #[test]
    fn foreign_hold_is_zero_effect_and_recoverable_against_its_owner() {
        let id = CollectionId(0xfedc_ba98_7654_3210);
        let (mut owner, owner_log) = streamer([]);
        let hold = load_with_hold(&mut owner, &owner_log, id);
        let (mut foreign, _) = streamer([]);
        load_to_completion(&mut foreign, id, None);

        let error = foreign.release_collection_hold(hold).unwrap_err();
        assert!(matches!(
            &error,
            ResidentCollectionHoldReleaseError::Authority {
                kind: ResidentCollectionHoldAuthorityError::ForeignDomain,
                ..
            }
        ));
        assert_eq!(foreign.observe_resident(id).unwrap().ref_count, 1);

        let recovered = error.into_hold().unwrap();
        assert!(!owner.release_collection_hold(recovered).unwrap());
        assert_eq!(owner.observe_resident(id).unwrap().ref_count, 1);
    }

    #[test]
    fn stale_hold_cannot_decrement_a_reloaded_occurrence_with_the_same_id() {
        let id = CollectionId(0x1111_2222_3333_4444);
        let (mut streamer, log) = streamer([]);
        let stale = load_with_hold(&mut streamer, &log, id);

        assert!(!streamer.down_collection_loaded_count(id).unwrap());
        assert!(streamer.down_collection_loaded_count(id).unwrap());
        load_to_completion(&mut streamer, id, None);
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 1);

        let error = streamer.release_collection_hold(stale).unwrap_err();
        assert!(matches!(
            &error,
            ResidentCollectionHoldReleaseError::Authority {
                kind: ResidentCollectionHoldAuthorityError::UnknownOccurrence,
                ..
            }
        ));
        assert_eq!(error.into_hold().unwrap().id(), id);
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 1);
    }

    #[test]
    fn dropping_a_hold_is_inert_and_does_not_hide_a_resident_decrement() {
        let id = CollectionId(88);
        let (mut streamer, log) = streamer([]);
        let hold = load_with_hold(&mut streamer, &log, id);
        streamer.enqueue_unload(id, None);
        streamer.update().unwrap();
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 1);

        drop(hold);

        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 1);
        assert!(!log.borrow().iter().any(|event| matches!(
            event,
            Event::AssetIds(_) | Event::Destroy(_) | Event::CollectionDrop(_)
        )));
    }

    #[test]
    fn preexisting_terminal_fault_returns_valid_hold_without_decrement() {
        let id = CollectionId(90);
        let missing = CollectionId(900);
        let (mut streamer, log) = streamer([]);
        let hold = load_with_hold(&mut streamer, &log, id);
        streamer.metadata.missing.push(missing);
        streamer.enqueue_load(missing, 0, None);
        let terminal = streamer.update().unwrap_err();
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 2);

        let error = streamer.release_collection_hold(hold).unwrap_err();

        assert!(matches!(
            &error,
            ResidentCollectionHoldReleaseError::StreamerUnavailable { source, .. }
                if Arc::ptr_eq(source, &terminal)
        ));
        assert_eq!(error.into_hold().unwrap().id(), id);
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 2);
    }

    #[test]
    fn valid_hold_is_consumed_when_zero_count_teardown_faults() {
        let id = CollectionId(89);
        let (mut streamer, log) = streamer([]);
        let hold = load_with_hold(&mut streamer, &log, id);
        streamer.enqueue_unload(id, None);
        streamer.update().unwrap();
        streamer.registry.fail_stage = Some(ResidentBackendStage::RemoveWrapper);

        let error = streamer.release_collection_hold(hold).unwrap_err();
        let first_fault = match &error {
            ResidentCollectionHoldReleaseError::Streamer(source) => Arc::clone(source),
            _ => panic!("valid decrement should consume its hold"),
        };

        assert!(matches!(
            &error,
            ResidentCollectionHoldReleaseError::Streamer(source)
                if matches!(
                    &**source,
                    CollectionResidentStreamerError::Backend {
                        stage: ResidentBackendStage::RemoveWrapper,
                        source: TestError("remove")
                    }
                )
        ));
        assert!(error.into_hold().is_none());
        assert!(streamer.has_retained_failed_teardown());
        let replay = streamer.down_collection_loaded_count(id).unwrap_err();
        assert!(Arc::ptr_eq(&first_fault, &replay));
    }

    #[test]
    fn resident_hold_can_cross_the_future_subsystem_completion_boundary() {
        fn assert_send<T: Send>() {}
        assert_send::<ResidentCollectionHold>();
    }

    #[test]
    fn resident_generation_exhaustion_latches_before_storage_or_metadata_effects() {
        let id = CollectionId(91);
        let (mut streamer, log) = streamer([]);
        streamer.next_resident_generation = u64::MAX;
        streamer.enqueue_load(id, 0, None);

        let error = streamer.update().unwrap_err();

        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::ResidentGenerationExhausted { id: actual }
                if *actual == id
        ));
        assert!(log.borrow().is_empty());
        assert_eq!(streamer.resident_count(), 0);
    }

    #[test]
    fn physical_miss_inserts_before_writer_and_begin_then_returns() {
        let id = CollectionId(7);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 2,
            highest: 9,
            loaded: 12,
        }]);
        streamer.enqueue_load(id, 4, None);
        streamer.enqueue_load(CollectionId(8), 1, None);
        let result = streamer.update().unwrap();
        assert_eq!(result.physical_loads_started, 1);
        assert_eq!(streamer.queued_request_count(), 1);
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 1);
        assert_eq!(streamer.active_loaded_size(id), 12);
        assert_eq!(
            log.borrow().as_slice(),
            [
                Event::Create(4),
                Event::Metadata(id),
                Event::Writer(
                    *streamer
                        .load_factory
                        .writer_identities
                        .borrow()
                        .get(&id)
                        .unwrap()
                ),
                Event::Begin(
                    id,
                    4,
                    *streamer
                        .load_factory
                        .writer_identities
                        .borrow()
                        .get(&id)
                        .unwrap()
                ),
            ]
        );
    }

    #[test]
    fn incomplete_active_updates_kicks_and_never_pops_queue() {
        let id = CollectionId(1);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 2,
            highest: 0,
            loaded: 3,
        }]);
        streamer.enqueue_load(id, 1, None);
        streamer.enqueue_load(CollectionId(2), 1, None);
        streamer.update().unwrap();
        log.borrow_mut().clear();
        let result = streamer.update().unwrap();
        assert_eq!(result.decompression_kicks, 1);
        assert_eq!(streamer.queued_request_count(), 1);
        assert_eq!(
            log.borrow().as_slice(),
            [Event::DriverUpdate(id), Event::Kick]
        );
    }

    #[test]
    fn completed_active_publishes_callbacks_clears_and_kicks_without_next_pop() {
        let id = CollectionId(3);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 44,
            loaded: 8,
        }]);
        streamer.enqueue_load(id, 1, Some(callback(&log)));
        streamer.enqueue_load(CollectionId(4), 1, None);
        streamer.update().unwrap();
        log.borrow_mut().clear();
        let result = streamer.update().unwrap();
        assert_eq!(result.physical_loads_completed, 1);
        assert_eq!(streamer.highest_stream_block_size(), 44);
        assert_eq!(streamer.queued_request_count(), 1);
        assert_eq!(streamer.active_id(), None);
        let events = log.borrow();
        assert!(matches!(events[0], Event::DriverUpdate(got) if got == id));
        assert!(matches!(events[1], Event::WrapperCreate(got, _) if got == id));
        assert!(matches!(events[2], Event::Publish(got, _) if got == id));
        assert_eq!(
            events[3],
            Event::Callback(
                id,
                ResidentCallbackObservation {
                    active_id: id,
                    resident_ref_count: Some(1),
                    wrapper_is_published: true,
                }
            )
        );
        assert_eq!(events[4], Event::Kick);
    }

    #[test]
    fn cache_hits_increment_callback_and_continue_until_first_miss() {
        let cached = CollectionId(10);
        let missing = CollectionId(11);
        let (mut streamer, log) = streamer([
            DriverPlan {
                updates_until_complete: 1,
                highest: 0,
                loaded: 0,
            },
            DriverPlan {
                updates_until_complete: 1,
                highest: 0,
                loaded: 0,
            },
        ]);
        load_to_completion(&mut streamer, cached, None);
        log.borrow_mut().clear();
        streamer.enqueue_load(cached, 2, Some(callback(&log)));
        streamer.enqueue_load(cached, 2, None);
        streamer.enqueue_load(missing, 2, None);
        streamer.enqueue_load(CollectionId(12), 2, None);
        let result = streamer.update().unwrap();
        assert_eq!(result.cache_hits, 2);
        assert_eq!(result.physical_loads_started, 1);
        assert_eq!(streamer.observe_resident(cached).unwrap().ref_count, 3);
        assert_eq!(streamer.queued_request_count(), 1);
        assert_eq!(
            log.borrow()[0],
            Event::Callback(
                cached,
                ResidentCallbackObservation {
                    active_id: cached,
                    resident_ref_count: Some(2),
                    wrapper_is_published: true,
                }
            )
        );
    }

    #[test]
    fn unload_callback_precedes_decrement_and_requests_continue() {
        let id = CollectionId(20);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        load_to_completion(&mut streamer, id, None);
        streamer.up_collection_loaded_count(id).unwrap();
        log.borrow_mut().clear();
        streamer.enqueue_unload(id, Some(callback(&log)));
        streamer.enqueue_unload(id, None);
        let result = streamer.update().unwrap();
        assert_eq!(result.unloads_processed, 2);
        assert_eq!(
            log.borrow()[0],
            Event::Callback(
                id,
                ResidentCallbackObservation {
                    active_id: id,
                    resident_ref_count: Some(2),
                    wrapper_is_published: true,
                }
            )
        );
        assert!(streamer.observe_resident(id).is_none());
    }

    #[test]
    fn zero_count_erases_then_garbage_collects_duplicate_occurrences_in_order() {
        let id = CollectionId(30);
        let a = CollectionId(0x1_0000_0002);
        let b = CollectionId(0x2_0000_0001);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        streamer.storage.next_assets.push_back(vec![a, b, a]);
        load_to_completion(&mut streamer, id, None);
        log.borrow_mut().clear();
        assert!(streamer.down_collection_loaded_count(id).unwrap());
        assert!(streamer.observe_resident(id).is_none());
        let events = log.borrow();
        assert_eq!(events[0], Event::AssetIds(id));
        assert_eq!(events[1], Event::Garbage(a));
        assert_eq!(events[2], Event::Garbage(b));
        assert_eq!(events[3], Event::Garbage(a));
        assert!(matches!(events[4], Event::Remove(got, _) if got == id));
        assert_eq!(events[5], Event::WrapperDrop(id));
        assert_eq!(events[6], Event::Destroy(id));
        assert_eq!(events[7], Event::CollectionDrop(id));
    }

    #[test]
    fn wrapping_reference_counts_do_not_saturate() {
        let id = CollectionId(40);
        let (mut streamer, _) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        load_to_completion(&mut streamer, id, None);
        streamer.residents[0].ref_count = u32::MAX;
        assert_eq!(
            streamer.up_collection_loaded_count(id).unwrap().ref_count,
            0
        );
        assert!(!streamer.down_collection_loaded_count(id).unwrap());
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, u32::MAX);
    }

    #[test]
    fn full_u64_identity_survives_native_hash_collision() {
        let first = CollectionId(0x0000_0001_0000_0002);
        let second = CollectionId(0x0000_0002_0000_0001);
        assert_eq!(first.native_hash(), second.native_hash());
        let (mut streamer, _) = streamer([
            DriverPlan {
                updates_until_complete: 1,
                highest: 0,
                loaded: 0,
            },
            DriverPlan {
                updates_until_complete: 1,
                highest: 0,
                loaded: 0,
            },
        ]);
        load_to_completion(&mut streamer, first, None);
        load_to_completion(&mut streamer, second, None);
        assert_eq!(streamer.resident_count(), 2);
        streamer.up_collection_loaded_count(second).unwrap();
        assert_eq!(streamer.observe_resident(first).unwrap().ref_count, 1);
        assert_eq!(streamer.observe_resident(second).unwrap().ref_count, 2);
    }

    #[test]
    fn resident_capacity_rejects_only_new_ids_before_storage_mutation() {
        let (mut streamer, log) = streamer([]);
        for value in 0..RESIDENT_COLLECTION_CAPACITY as u64 {
            streamer.residents.push(ResidentEntry {
                id: CollectionId(value),
                generation: value,
                collection: Box::new(TestCollection {
                    identity: Rc::new(()),
                    id: Cell::new(Some(CollectionId(value))),
                    assets: RefCell::new(Vec::new()),
                    log: Rc::clone(&log),
                }),
                wrapper: None,
                ref_count: 1,
            });
        }
        streamer.enqueue_load(CollectionId(1), 1, None);
        assert_eq!(streamer.update().unwrap().cache_hits, 1);
        streamer.enqueue_load(CollectionId(999), 1, None);
        log.borrow_mut().clear();
        let error = streamer.update().unwrap_err();
        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::ResidentCapacityExceeded { id }
                if *id == CollectionId(999)
        ));
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn callback_can_reentrantly_enqueue_without_disturbing_active_request() {
        let id = CollectionId(50);
        let next = CollectionId(51);
        let (mut streamer, log) = streamer([
            DriverPlan {
                updates_until_complete: 1,
                highest: 0,
                loaded: 0,
            },
            DriverPlan {
                updates_until_complete: 1,
                highest: 0,
                loaded: 0,
            },
        ]);
        load_to_completion(&mut streamer, id, None);
        let mut reentrant = callback(&log);
        reentrant.enqueue = Some((next, ResidentCollectionRequestKind::Load));
        streamer.enqueue_load(id, 0, Some(reentrant));
        let result = streamer.update().unwrap();
        assert_eq!(result.cache_hits, 1);
        assert_eq!(result.physical_loads_started, 1);
        assert_eq!(streamer.active_id(), Some(next));
    }

    #[test]
    fn callback_failure_after_active_up_retains_the_increment() {
        let id = CollectionId(52);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        load_to_completion(&mut streamer, id, None);
        let mut failing = callback(&log);
        failing.up_active_times = 1;
        failing.fail = true;
        streamer.enqueue_load(id, 0, Some(failing));

        let error = streamer.update().unwrap_err();

        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::Backend {
                stage: ResidentBackendStage::Callback,
                ..
            }
        ));
        // Cache-hit Up runs first (1 -> 2), then the callback's recovered
        // active-only capability runs (2 -> 3). Failure does not roll either
        // mutation back.
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 3);
        let replay = streamer.update().unwrap_err();
        assert!(Arc::ptr_eq(&error, &replay));
    }

    #[test]
    fn zero_asset_driver_controls_native_deferred_completion_timing() {
        let zero = CollectionId(60);
        let next = CollectionId(61);
        let (mut streamer, _) = streamer([
            DriverPlan {
                updates_until_complete: 1,
                highest: 0,
                loaded: 0,
            },
            DriverPlan {
                updates_until_complete: 1,
                highest: 0,
                loaded: 0,
            },
        ]);
        streamer.enqueue_load(zero, 0, None);
        streamer.enqueue_load(next, 1, None);
        assert_eq!(streamer.update().unwrap().physical_loads_started, 1);
        assert_eq!(streamer.update().unwrap().physical_loads_completed, 1);
        assert_eq!(streamer.active_id(), None);
        assert_eq!(streamer.queued_request_count(), 1);
        assert_eq!(streamer.update().unwrap().physical_loads_started, 1);
        assert_eq!(streamer.active_id(), Some(next));
    }

    #[test]
    fn highest_block_uses_unsigned_strict_greater_and_retains_ties() {
        let (mut streamer, _) = streamer([
            DriverPlan {
                updates_until_complete: 1,
                highest: u32::MAX,
                loaded: 0,
            },
            DriverPlan {
                updates_until_complete: 1,
                highest: u32::MAX,
                loaded: 0,
            },
        ]);
        load_to_completion(&mut streamer, CollectionId(70), None);
        assert_eq!(streamer.highest_stream_block_size(), u32::MAX);
        load_to_completion(&mut streamer, CollectionId(71), None);
        assert_eq!(streamer.highest_stream_block_size(), u32::MAX);
    }

    #[test]
    fn pending_and_loaded_size_observe_queue_and_exact_active_id() {
        let id = CollectionId(80);
        let (mut streamer, _) = streamer([DriverPlan {
            updates_until_complete: 2,
            highest: 0,
            loaded: 123,
        }]);
        assert!(!streamer.pending_requests());
        streamer.enqueue_load(id, 2, None);
        assert!(streamer.pending_requests());
        streamer.update().unwrap();
        assert!(streamer.pending_requests());
        assert_eq!(streamer.active_loaded_size(id), 123);
        assert_eq!(streamer.active_loaded_size(CollectionId(81)), 0);
    }

    #[test]
    fn first_fault_replays_same_arc_and_preserves_partial_loading_entry() {
        let id = CollectionId(90);
        let (mut streamer, _) = streamer([]);
        streamer.load_factory.fail_begin = true;
        streamer.enqueue_load(id, 1, None);
        let first = streamer.update().unwrap_err();
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 1);
        assert!(!streamer.observe_resident(id).unwrap().wrapper_is_published);
        let second = streamer.update().unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(matches!(
            &*first,
            CollectionResidentStreamerError::Backend {
                stage: ResidentBackendStage::BeginLoad,
                ..
            }
        ));
    }

    #[test]
    fn driver_failure_retains_active_request_and_suppresses_kick_and_queue_pop() {
        let id = CollectionId(91);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        streamer.load_factory.fail_update = true;
        streamer.enqueue_load(id, 1, None);
        streamer.enqueue_load(CollectionId(92), 1, None);
        streamer.update().unwrap();
        log.borrow_mut().clear();
        let error = streamer.update().unwrap_err();
        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::Backend {
                stage: ResidentBackendStage::UpdateLoad,
                ..
            }
        ));
        assert_eq!(streamer.active_id(), Some(id));
        assert_eq!(streamer.queued_request_count(), 1);
        assert_eq!(log.borrow().as_slice(), [Event::DriverUpdate(id)]);
    }

    #[test]
    fn incomplete_kick_failure_retains_active_and_replays_without_more_driver_work() {
        let id = CollectionId(93);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 2,
            highest: 0,
            loaded: 0,
        }]);
        streamer.decompression.fail = true;
        streamer.enqueue_load(id, 1, None);
        streamer.update().unwrap();
        log.borrow_mut().clear();
        let first = streamer.update().unwrap_err();
        assert!(matches!(
            &*first,
            CollectionResidentStreamerError::Backend {
                stage: ResidentBackendStage::StartDecompress,
                ..
            }
        ));
        assert_eq!(
            log.borrow().as_slice(),
            [Event::DriverUpdate(id), Event::Kick]
        );
        let second = streamer.update().unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(
            log.borrow().as_slice(),
            [Event::DriverUpdate(id), Event::Kick]
        );
    }

    #[test]
    fn publish_failure_retains_installed_wrapper_and_active_request() {
        let id = CollectionId(94);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        streamer.enqueue_load(id, 1, None);
        streamer.update().unwrap();
        streamer.registry.fail_stage = Some(ResidentBackendStage::PublishWrapper);
        log.borrow_mut().clear();
        let error = streamer.update().unwrap_err();
        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::Backend {
                stage: ResidentBackendStage::PublishWrapper,
                ..
            }
        ));
        assert_eq!(streamer.active_id(), Some(id));
        assert!(!streamer.observe_resident(id).unwrap().wrapper_is_published);
        assert!(matches!(
            log.borrow().as_slice(),
            [
                Event::DriverUpdate(got),
                Event::WrapperCreate(created, _),
                Event::Publish(published, _)
            ] if *got == id && *created == id && *published == id
        ));
    }

    #[test]
    fn completion_kick_failure_preserves_publication_after_active_clear() {
        let id = CollectionId(95);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 6,
            loaded: 0,
        }]);
        streamer.enqueue_load(id, 1, None);
        streamer.update().unwrap();
        streamer.decompression.fail = true;
        log.borrow_mut().clear();
        let error = streamer.update().unwrap_err();
        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::Backend {
                stage: ResidentBackendStage::StartDecompress,
                ..
            }
        ));
        assert_eq!(streamer.active_id(), None);
        assert!(streamer.observe_resident(id).unwrap().wrapper_is_published);
        assert_eq!(streamer.highest_stream_block_size(), 6);
        assert!(matches!(log.borrow().last(), Some(Event::Kick)));
    }

    #[test]
    fn implicit_drop_deliberately_abandons_resident_identities_without_cleanup() {
        let id = CollectionId(96);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        load_to_completion(&mut streamer, id, None);
        log.borrow_mut().clear();
        drop(streamer);
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn garbage_failure_retains_erased_entry_and_stops_later_cleanup() {
        let id = CollectionId(100);
        let first = CollectionId(101);
        let second = CollectionId(102);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        streamer.storage.next_assets.push_back(vec![first, second]);
        load_to_completion(&mut streamer, id, None);
        streamer.garbage_collector.fail_on = Some(first);
        log.borrow_mut().clear();
        let error = streamer.down_collection_loaded_count(id).unwrap_err();
        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::Backend {
                stage: ResidentBackendStage::GarbageCollectAsset,
                ..
            }
        ));
        assert!(streamer.observe_resident(id).is_none());
        assert!(streamer.has_retained_failed_teardown());
        assert_eq!(
            log.borrow().as_slice(),
            [Event::AssetIds(id), Event::Garbage(first)]
        );
        let teardown = streamer.failed_teardown.as_ref().unwrap();
        assert_eq!(teardown.entry.id, id);
        assert_eq!(teardown.asset_ids, [first, second]);
        assert_eq!(teardown.next_asset, 0);
    }

    #[test]
    fn registry_remove_failure_retains_owner_after_all_garbage_calls() {
        let id = CollectionId(103);
        let asset = CollectionId(104);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        streamer.storage.next_assets.push_back(vec![asset]);
        load_to_completion(&mut streamer, id, None);
        streamer.registry.fail_stage = Some(ResidentBackendStage::RemoveWrapper);
        log.borrow_mut().clear();
        let error = streamer.down_collection_loaded_count(id).unwrap_err();
        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::Backend {
                stage: ResidentBackendStage::RemoveWrapper,
                ..
            }
        ));
        assert!(streamer.observe_resident(id).is_none());
        assert!(streamer.has_retained_failed_teardown());
        let retained = streamer.retained_failed_teardown().unwrap();
        assert_eq!(retained.id, id);
        assert_eq!(retained.asset_ids, [asset]);
        assert_eq!(retained.next_asset, 1);
        assert!(matches!(
            log.borrow().as_slice(),
            [Event::AssetIds(got), Event::Garbage(found), Event::Remove(removed, _)]
                if *got == id && *found == asset && *removed == id
        ));
    }

    #[test]
    fn missing_metadata_is_latched_after_create_with_safe_cleanup() {
        let id = CollectionId(110);
        let (mut streamer, log) = streamer([]);
        streamer.metadata.missing.push(id);
        streamer.enqueue_load(id, 7, None);
        let error = streamer.update().unwrap_err();
        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::MissingCollectionMetadata { id: got }
                if *got == id
        ));
        assert_eq!(
            log.borrow().as_slice(),
            [
                Event::Create(7),
                Event::Metadata(id),
                Event::Destroy(CollectionId(0)),
                Event::CollectionDrop(CollectionId(0))
            ]
        );
    }

    #[test]
    fn callback_failure_latches_before_unload_decrement() {
        let id = CollectionId(120);
        let (mut streamer, log) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        load_to_completion(&mut streamer, id, None);
        let mut failing = callback(&log);
        failing.fail = true;
        streamer.enqueue_unload(id, Some(failing));
        let error = streamer.update().unwrap_err();
        assert!(matches!(
            &*error,
            CollectionResidentStreamerError::Backend {
                stage: ResidentBackendStage::Callback,
                ..
            }
        ));
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 1);
        assert_eq!(streamer.active_id(), Some(id));
    }

    #[test]
    fn wrapper_and_writer_reference_the_same_stable_collection_identity() {
        let id = CollectionId(130);
        let (mut streamer, _) = streamer([DriverPlan {
            updates_until_complete: 1,
            highest: 0,
            loaded: 0,
        }]);
        load_to_completion(&mut streamer, id, None);
        let writer_identity = streamer.load_factory.writer_identities.borrow()[&id];
        let wrapper = streamer.residents[0].wrapper.as_ref().unwrap();
        assert_eq!(wrapper.owner.collection_identity, writer_identity);
        assert_eq!(wrapper.owner.id, id);
    }
}
