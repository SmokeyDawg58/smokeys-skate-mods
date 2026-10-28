//! Outer collection-operation scheduling for `cAssetStreamSystem`.
//!
//! This module stops at two explicit collaborator boundaries. Collection XSF
//! I/O is an opaque, result-bearing operation, while `LoadComplete` and
//! `UnloadComplete` retain their broader octree/registration side effects in a
//! lifecycle collaborator. It does not reuse the mounted manifest backend or
//! claim to decode collection resources.
//!
//! Exact control-flow anchors in the supplied debug build are
//! `UpdateAssetStreamManager` at `0x824dd1e0`, `LoadBestCollection` at
//! `0x824dbcc8`, `LoadNextCollection` at `0x824dbe78`, and
//! `UnloadNextCollection` at `0x824dbfc8`. Busy-state helpers are
//! `0x824d6d80..0x824d6e40`; completion callbacks are
//! `0x824ddce0`/`0x824ddd78`. The ordering and ordinary branch semantics here
//! are exact. Rust `Result` errors, permanent first-fault latching, and safe
//! exclusive borrowing in place of the native update interlock are explicitly
//! high-confidence host policies for native assert/unsafe domains.

use std::{collections::VecDeque, fmt, sync::Arc};

use crate::{
    game_stream::StreamKind,
    streaming::{CollectionRequest, CollectionRequestKind, CollectionRequestQueue},
    world_stream_catalog::{CatalogError, CollectionRegistrationSnapshot, WorldStreamCatalog},
};

/// Exact inline capacity of both completion vectors at `cAssetStreamSystem`
/// `+0x11550` and `+0x11660` in the supplied debug build.
pub const COLLECTION_COMPLETION_CAPACITY: usize = 32;

/// Entry state sampled before one outer update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollectionSchedulerControls {
    /// Raw `mLoading`; only the exact value one suppresses the update.
    pub loading: u32,
}

/// Direction of one globally serialized collection operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionOperationKind {
    Load,
    Unload,
}

/// Request chosen for the one global operation slot.
#[derive(Clone, Debug, PartialEq)]
pub struct CollectionDispatch {
    stream_kind: StreamKind,
    request: CollectionRequest,
}

impl CollectionDispatch {
    #[must_use]
    pub const fn stream_kind(&self) -> StreamKind {
        self.stream_kind
    }

    #[must_use]
    pub const fn request(&self) -> &CollectionRequest {
        &self.request
    }

    #[must_use]
    pub fn operation_kind(&self) -> CollectionOperationKind {
        match self.request.kind {
            CollectionRequestKind::Load => CollectionOperationKind::Load,
            CollectionRequestKind::Unload => CollectionOperationKind::Unload,
            CollectionRequestKind::Update => {
                unreachable!("update requests are never operation dispatches")
            }
        }
    }
}

/// Mutable view passed to the opaque manager poll.
pub struct ActiveCollectionOperation<'a, O> {
    pub dispatch: &'a CollectionDispatch,
    pub operation: &'a mut O,
}

/// Result of one `cAssetStreamManager::Update`-shaped collaborator poll.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionOperationPoll {
    Pending,
    Complete,
}

/// Opaque collection load/unload operation boundary.
///
/// Native manager calls are void and expose no recoverable error branch.
/// Returning `Result` is a safe host policy: the first error is permanently
/// latched by [`CollectionStreamScheduler`].
pub trait CollectionOperationBackend {
    type Operation;
    type Error;

    /// # Errors
    ///
    /// Reports an unrecoverable host collaborator failure.
    fn poll_manager(
        &mut self,
        active: Option<ActiveCollectionOperation<'_, Self::Operation>>,
    ) -> Result<CollectionOperationPoll, Self::Error>;

    /// Samples the manager garbage-release word at `+0xea24`. Native load
    /// selection compares it unsigned against `mLoadDelay` at `0x824dbd90`
    /// and `0x824dbf3c`.
    fn garbage_release_index(&mut self) -> u32;

    /// Samples the live FE-order cVar after the first manager poll and active
    /// operation check. Only the exact value one enables the FE pass.
    fn front_end_collection_order(&mut self) -> u32;

    /// Samples the live defer cVar only when scheduling reaches the regular
    /// pass. Zero selects the nondeferred policy.
    fn collection_defer(&mut self) -> u32;

    /// Inner manager pending state, consulted only when every outer source is
    /// quiescent.
    ///
    /// # Errors
    ///
    /// Reports an unrecoverable host collaborator failure.
    fn has_pending_requests(&mut self) -> Result<bool, Self::Error>;

    /// Begins one opaque collection load.
    ///
    /// # Errors
    ///
    /// Reports an unrecoverable host collaborator failure.
    fn begin_load(
        &mut self,
        dispatch: &CollectionDispatch,
        metadata: &CollectionRegistrationSnapshot,
    ) -> Result<Self::Operation, Self::Error>;

    /// Begins one opaque collection unload.
    ///
    /// # Errors
    ///
    /// Reports an unrecoverable host collaborator failure.
    fn begin_unload(
        &mut self,
        dispatch: &CollectionDispatch,
    ) -> Result<Self::Operation, Self::Error>;
}

/// In-flight activation/deactivation target stored by the native outer system.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollectionLifecycleProgress {
    pub id: u64,
    pub target_count: u32,
}

/// State returned by the opaque `LoadComplete` boundary.
///
/// The helper mutates per-kind loaded counts through processed-record state.
/// Returning the target count avoids recreating those still-unrecovered
/// internals while the scheduler retains the exact completed collection ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollectionLoadResolution {
    pub target_count: Option<u32>,
    pub loaded_counts: [i32; 3],
}

/// State returned only by the opaque `UnloadComplete` boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollectionUnloadResolution {
    pub target_count: Option<u32>,
    pub loaded_counts: [i32; 3],
    /// Newly sampled manager garbage-head word at `+0xea1c`.
    pub garbage_head: u32,
}

/// Opaque activation/deactivation collaborator.
pub trait CollectionLifecycleBackend {
    type Error;

    /// Reads the zero-extended lifecycle byte at `tOctreeInfo + 0x76`.
    /// `None` represents the native missing-record clearing branch.
    ///
    /// # Errors
    ///
    /// Reports an unrecoverable lifecycle collaborator failure.
    fn lifecycle_count(&mut self, id: u64) -> Result<Option<u8>, Self::Error>;

    /// Owns all broader `LoadComplete` behavior at `0x824dc208`.
    ///
    /// # Errors
    ///
    /// Reports an unrecoverable lifecycle collaborator failure.
    fn load_complete(
        &mut self,
        id: u64,
        loaded_counts: [i32; 3],
    ) -> Result<CollectionLoadResolution, Self::Error>;

    /// Owns all broader `UnloadComplete` behavior at `0x824dc3e8`.
    ///
    /// # Errors
    ///
    /// Reports an unrecoverable lifecycle collaborator failure.
    fn unload_complete(
        &mut self,
        id: u64,
        loaded_counts: [i32; 3],
    ) -> Result<CollectionUnloadResolution, Self::Error>;
}

/// Safe-boundary scheduler failure.
#[derive(Debug)]
pub enum CollectionSchedulerError<BackendError, LifecycleError> {
    Backend(BackendError),
    Lifecycle(LifecycleError),
    Catalog(CatalogError),
    MissingCollectionMetadata(u64),
    CompletionQueueFull {
        kind: CollectionOperationKind,
        id: u64,
    },
    CompletionWithoutActiveOperation,
}

impl<B: fmt::Display, L: fmt::Display> fmt::Display for CollectionSchedulerError<B, L> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(formatter, "collection backend failed: {error}"),
            Self::Lifecycle(error) => write!(formatter, "collection lifecycle failed: {error}"),
            Self::Catalog(error) => write!(formatter, "collection catalog failed: {error}"),
            Self::MissingCollectionMetadata(id) => {
                write!(
                    formatter,
                    "collection {id:#018x} is absent from the catalog"
                )
            }
            Self::CompletionQueueFull { kind, id } => write!(
                formatter,
                "{kind:?} completion queue is full while completing {id:#018x}"
            ),
            Self::CompletionWithoutActiveOperation => {
                formatter.write_str("manager reported completion without an active operation")
            }
        }
    }
}

impl<B, L> std::error::Error for CollectionSchedulerError<B, L>
where
    B: std::error::Error + 'static,
    L: std::error::Error + 'static,
{
}

/// Shared first-fault type returned by a scheduler instance.
pub type SharedCollectionSchedulerError<B, L> = Arc<
    CollectionSchedulerError<
        <B as CollectionOperationBackend>::Error,
        <L as CollectionLifecycleBackend>::Error,
    >,
>;

/// Result returned by scheduler operations.
pub type CollectionSchedulerResult<T, B, L> = Result<T, SharedCollectionSchedulerError<B, L>>;

/// Observable result of one outer scheduler update.
#[derive(Clone, Debug, PartialEq)]
pub struct CollectionSchedulerUpdate {
    pub manager_polls: u8,
    pub dispatched: Option<CollectionDispatch>,
}

#[derive(Clone, Copy)]
enum LoadSelection {
    Best,
    Next,
}

#[derive(Clone, Copy)]
enum DispatchRule {
    Unload(StreamKind),
    Load {
        kind: StreamKind,
        selection: LoadSelection,
        require_loaded_below_one: bool,
    },
}

/// Readable outer scheduler recovered from
/// `cAssetStreamSystem::UpdateAssetStreamManager` (`0x824dd1e0`).
pub struct CollectionStreamScheduler<B, L>
where
    B: CollectionOperationBackend,
    L: CollectionLifecycleBackend,
{
    catalog: WorldStreamCatalog,
    backend: B,
    lifecycle: L,
    queues: [CollectionRequestQueue; 3],
    load_completions: VecDeque<u64>,
    unload_completions: VecDeque<u64>,
    active_dispatch: Option<CollectionDispatch>,
    active_operation: Option<B::Operation>,
    activating: Option<CollectionLifecycleProgress>,
    deactivating: Option<CollectionLifecycleProgress>,
    /// Signed native counters at `cAssetStreamSystem + 0x11930`.
    loaded_counts: [i32; 3],
    /// Persistent `mLoadDelay` at `+0x1192c`, refreshed by successful
    /// `UnloadComplete` processing.
    load_delay: u32,
    update_locked: bool,
    terminal_error: Option<SharedCollectionSchedulerError<B, L>>,
}

impl<B, L> CollectionStreamScheduler<B, L>
where
    B: CollectionOperationBackend,
    L: CollectionLifecycleBackend,
{
    /// Constructs empty request/completion queues with caller-supplied live
    /// loaded counts and persistent load delay. No runtime default is inferred
    /// here.
    #[must_use]
    pub fn new(
        catalog: WorldStreamCatalog,
        backend: B,
        lifecycle: L,
        loaded_counts: [i32; 3],
        load_delay: u32,
    ) -> Self {
        Self {
            catalog,
            backend,
            lifecycle,
            queues: std::array::from_fn(|_| CollectionRequestQueue::default()),
            load_completions: VecDeque::with_capacity(COLLECTION_COMPLETION_CAPACITY),
            unload_completions: VecDeque::with_capacity(COLLECTION_COMPLETION_CAPACITY),
            active_dispatch: None,
            active_operation: None,
            activating: None,
            deactivating: None,
            loaded_counts,
            load_delay,
            update_locked: false,
            terminal_error: None,
        }
    }

    /// Applies one request to its per-stream queue.
    ///
    /// Updates retain the supplied stream kind because native owns three
    /// independent load/unload vector pairs at `+0x58` and `+0x88`.
    ///
    /// # Errors
    ///
    /// Replays the scheduler's previously latched terminal error.
    pub fn queue_request(
        &mut self,
        stream_kind: StreamKind,
        request: CollectionRequest,
    ) -> CollectionSchedulerResult<(), B, L> {
        self.check_terminal()?;
        let queue = &mut self.queues[kind_index(stream_kind)];
        match request.kind {
            CollectionRequestKind::Load => queue.add_load(request),
            CollectionRequestKind::Update => queue.update_load(&request),
            CollectionRequestKind::Unload => queue.add_unload(request),
        }
        Ok(())
    }

    #[must_use]
    pub fn queue(&self, kind: StreamKind) -> &CollectionRequestQueue {
        &self.queues[kind_index(kind)]
    }

    #[must_use]
    pub const fn loaded_counts(&self) -> [i32; 3] {
        self.loaded_counts
    }

    #[must_use]
    pub const fn load_delay(&self) -> u32 {
        self.load_delay
    }

    #[must_use]
    pub fn load_completions(&self) -> &VecDeque<u64> {
        &self.load_completions
    }

    #[must_use]
    pub fn unload_completions(&self) -> &VecDeque<u64> {
        &self.unload_completions
    }

    #[must_use]
    pub const fn activating(&self) -> Option<CollectionLifecycleProgress> {
        self.activating
    }

    #[must_use]
    pub const fn deactivating(&self) -> Option<CollectionLifecycleProgress> {
        self.deactivating
    }

    #[must_use]
    pub fn active_dispatch(&self) -> Option<&CollectionDispatch> {
        self.active_dispatch.as_ref()
    }

    #[must_use]
    pub fn terminal_error(&self) -> Option<&SharedCollectionSchedulerError<B, L>> {
        self.terminal_error.as_ref()
    }

    /// Reconstructs the outer `PendingRequests` short-circuit. The inner
    /// collaborator is queried only after the active slot, both lifecycle
    /// states, both completion FIFOs, and all six request queues are empty.
    ///
    /// # Errors
    ///
    /// Latches and returns an inner-backend failure, or replays the first
    /// previously latched error.
    pub fn pending_requests(&mut self) -> CollectionSchedulerResult<bool, B, L> {
        self.check_terminal()?;
        if self.active_dispatch.is_some()
            || self.activating.is_some()
            || self.deactivating.is_some()
            || !self.load_completions.is_empty()
            || !self.unload_completions.is_empty()
            || self.queues.iter().any(|queue| {
                !queue.pending_loads().is_empty() || !queue.pending_unloads().is_empty()
            })
        {
            return Ok(true);
        }
        match self.backend.has_pending_requests() {
            Ok(pending) => Ok(pending),
            Err(error) => self.latch(CollectionSchedulerError::Backend(error)),
        }
    }

    pub fn into_parts(self) -> (WorldStreamCatalog, B, L) {
        (self.catalog, self.backend, self.lifecycle)
    }

    /// Performs one outer update.
    ///
    /// Completion reconciliation runs deactivation then activation, admits at
    /// most one completion of each direction, polls the manager, selects at
    /// most one operation if the global slot is empty, and immediately polls
    /// again only after a dispatch. A callback observed during either poll is
    /// intentionally not reconciled until the next outer call.
    ///
    /// # Errors
    ///
    /// Permanently latches the first backend, lifecycle, catalog, completion
    /// capacity, or internal active-operation error.
    pub fn update(
        &mut self,
        controls: CollectionSchedulerControls,
    ) -> CollectionSchedulerResult<CollectionSchedulerUpdate, B, L> {
        self.check_terminal()?;
        if controls.loading == 1 || self.update_locked {
            return Ok(CollectionSchedulerUpdate {
                manager_polls: 0,
                dispatched: None,
            });
        }

        self.update_locked = true;
        let result = self.update_locked();
        self.update_locked = false;
        result
    }

    fn update_locked(&mut self) -> CollectionSchedulerResult<CollectionSchedulerUpdate, B, L> {
        self.reconcile_deactivating()?;
        self.reconcile_activating()?;
        self.admit_unload_completion()?;
        self.admit_load_completion()?;

        self.poll_manager()?;
        let mut result = CollectionSchedulerUpdate {
            manager_polls: 1,
            dispatched: None,
        };
        if self.active_dispatch.is_some() {
            return Ok(result);
        }

        if let Some(dispatch) = self.select_and_dispatch()? {
            result.dispatched = Some(dispatch);
            self.poll_manager()?;
            result.manager_polls = 2;
        }
        Ok(result)
    }

    fn check_terminal(&self) -> CollectionSchedulerResult<(), B, L> {
        match &self.terminal_error {
            Some(error) => Err(Arc::clone(error)),
            None => Ok(()),
        }
    }

    fn latch<T>(
        &mut self,
        error: CollectionSchedulerError<B::Error, L::Error>,
    ) -> CollectionSchedulerResult<T, B, L> {
        let error = Arc::new(error);
        self.terminal_error = Some(Arc::clone(&error));
        Err(error)
    }

    fn reconcile_deactivating(&mut self) -> CollectionSchedulerResult<(), B, L> {
        let Some(progress) = self.deactivating else {
            return Ok(());
        };
        let observed = match self.lifecycle.lifecycle_count(progress.id) {
            Ok(observed) => observed,
            Err(error) => return self.latch(CollectionSchedulerError::Lifecycle(error)),
        };
        if observed.is_none_or(|count| u32::from(count) == progress.target_count) {
            self.deactivating = None;
        }
        Ok(())
    }

    fn reconcile_activating(&mut self) -> CollectionSchedulerResult<(), B, L> {
        let Some(progress) = self.activating else {
            return Ok(());
        };
        let observed = match self.lifecycle.lifecycle_count(progress.id) {
            Ok(observed) => observed,
            Err(error) => return self.latch(CollectionSchedulerError::Lifecycle(error)),
        };
        if observed.is_none_or(|count| u32::from(count) == progress.target_count) {
            self.activating = None;
        }
        Ok(())
    }

    fn admit_unload_completion(&mut self) -> CollectionSchedulerResult<(), B, L> {
        if self.deactivating.is_some() {
            return Ok(());
        }
        let Some(&id) = self.unload_completions.front() else {
            return Ok(());
        };
        if self.active_id() == Some(id) || self.is_busy_activating(id) {
            return Ok(());
        }
        self.unload_completions.pop_front();
        let resolution = match self.lifecycle.unload_complete(id, self.loaded_counts) {
            Ok(resolution) => resolution,
            Err(error) => return self.latch(CollectionSchedulerError::Lifecycle(error)),
        };
        self.deactivating = resolution
            .target_count
            .map(|target_count| CollectionLifecycleProgress { id, target_count });
        self.loaded_counts = resolution.loaded_counts;
        self.load_delay = resolution.garbage_head;
        Ok(())
    }

    fn admit_load_completion(&mut self) -> CollectionSchedulerResult<(), B, L> {
        if self.activating.is_some() {
            return Ok(());
        }
        let Some(&id) = self.load_completions.front() else {
            return Ok(());
        };
        if self.active_id() == Some(id) || self.is_busy_deactivating(id) {
            return Ok(());
        }
        self.load_completions.pop_front();
        let resolution = match self.lifecycle.load_complete(id, self.loaded_counts) {
            Ok(resolution) => resolution,
            Err(error) => return self.latch(CollectionSchedulerError::Lifecycle(error)),
        };
        self.activating = resolution
            .target_count
            .map(|target_count| CollectionLifecycleProgress { id, target_count });
        self.loaded_counts = resolution.loaded_counts;
        Ok(())
    }

    fn poll_manager(&mut self) -> CollectionSchedulerResult<(), B, L> {
        let poll = match (&self.active_dispatch, &mut self.active_operation) {
            (Some(dispatch), Some(operation)) => {
                self.backend.poll_manager(Some(ActiveCollectionOperation {
                    dispatch,
                    operation,
                }))
            }
            (None, None) => self.backend.poll_manager(None),
            _ => return self.latch(CollectionSchedulerError::CompletionWithoutActiveOperation),
        };
        let poll = match poll {
            Ok(poll) => poll,
            Err(error) => return self.latch(CollectionSchedulerError::Backend(error)),
        };
        if poll == CollectionOperationPoll::Pending {
            return Ok(());
        }
        let Some(dispatch) = self.active_dispatch.as_ref() else {
            return self.latch(CollectionSchedulerError::CompletionWithoutActiveOperation);
        };
        let kind = dispatch.operation_kind();
        let queue_len = match kind {
            CollectionOperationKind::Load => self.load_completions.len(),
            CollectionOperationKind::Unload => self.unload_completions.len(),
        };
        if queue_len == COLLECTION_COMPLETION_CAPACITY {
            return self.latch(CollectionSchedulerError::CompletionQueueFull {
                kind,
                id: dispatch.request.id,
            });
        }
        let id = dispatch.request.id;
        match kind {
            CollectionOperationKind::Load => self.load_completions.push_back(id),
            CollectionOperationKind::Unload => self.unload_completions.push_back(id),
        }
        self.active_dispatch.take();
        self.active_operation.take();
        Ok(())
    }

    fn select_and_dispatch(
        &mut self,
    ) -> CollectionSchedulerResult<Option<CollectionDispatch>, B, L> {
        if self.backend.front_end_collection_order() == 1 {
            let (attempted, dispatch) = self.try_rules(&front_end_rules(), self.load_delay)?;
            if attempted {
                return Ok(dispatch);
            }
        }
        let collection_defer = self.backend.collection_defer();
        let (_, dispatch) = self.try_rules(
            &regular_rules(collection_defer, self.loaded_counts),
            self.load_delay,
        )?;
        Ok(dispatch)
    }

    fn try_rules(
        &mut self,
        rules: &[DispatchRule; 6],
        load_delay: u32,
    ) -> CollectionSchedulerResult<(bool, Option<CollectionDispatch>), B, L> {
        for &rule in rules {
            if !self.rule_is_eligible(rule) {
                continue;
            }
            let dispatch = match rule {
                DispatchRule::Unload(kind) => self.try_unload(kind)?,
                DispatchRule::Load {
                    kind, selection, ..
                } => self.try_load(kind, selection, load_delay)?,
            };
            return Ok((true, dispatch));
        }
        Ok((false, None))
    }

    fn rule_is_eligible(&self, rule: DispatchRule) -> bool {
        match rule {
            DispatchRule::Unload(kind) => !self.queue(kind).pending_unloads().is_empty(),
            DispatchRule::Load {
                kind,
                require_loaded_below_one,
                ..
            } => {
                !self.queue(kind).pending_loads().is_empty()
                    && (!require_loaded_below_one || self.loaded_counts[kind_index(kind)] < 1)
            }
        }
    }

    fn try_unload(
        &mut self,
        kind: StreamKind,
    ) -> CollectionSchedulerResult<Option<CollectionDispatch>, B, L> {
        let candidate = self.queue(kind).pending_unloads()[0].clone();
        if self.is_collection_active(candidate.id) {
            return Ok(None);
        }
        let request = self.queues[kind_index(kind)]
            .take_next_unload()
            .expect("eligible unload queue is nonempty");
        self.begin_dispatch(CollectionDispatch {
            stream_kind: kind,
            request,
        })
    }

    fn try_load(
        &mut self,
        kind: StreamKind,
        selection: LoadSelection,
        load_delay: u32,
    ) -> CollectionSchedulerResult<Option<CollectionDispatch>, B, L> {
        if !self.unload_completions.is_empty() || self.deactivating.is_some() {
            return Ok(None);
        }
        if load_delay > self.backend.garbage_release_index() {
            return Ok(None);
        }

        let candidate = {
            let mut preview = self.queue(kind).clone();
            match selection {
                LoadSelection::Best => preview.take_best_load(),
                LoadSelection::Next => preview.take_next_load(),
            }
        };
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        if self.is_collection_active(candidate.id) {
            return Ok(None);
        }
        let request = match selection {
            LoadSelection::Best => self.queues[kind_index(kind)].take_best_load(),
            LoadSelection::Next => self.queues[kind_index(kind)].take_next_load(),
        }
        .expect("previewed load remains present");
        debug_assert_eq!(request.id, candidate.id);
        self.begin_dispatch(CollectionDispatch {
            stream_kind: kind,
            request,
        })
    }

    fn begin_dispatch(
        &mut self,
        dispatch: CollectionDispatch,
    ) -> CollectionSchedulerResult<Option<CollectionDispatch>, B, L> {
        self.active_dispatch = Some(dispatch.clone());
        let operation = match dispatch.operation_kind() {
            CollectionOperationKind::Load => {
                let metadata = match self.catalog.collection(dispatch.request.id) {
                    Ok(Some(metadata)) => metadata,
                    Ok(None) => {
                        return self.latch(CollectionSchedulerError::MissingCollectionMetadata(
                            dispatch.request.id,
                        ));
                    }
                    Err(error) => return self.latch(CollectionSchedulerError::Catalog(error)),
                };
                self.backend.begin_load(&dispatch, &metadata)
            }
            CollectionOperationKind::Unload => self.backend.begin_unload(&dispatch),
        };
        match operation {
            Ok(operation) => self.active_operation = Some(operation),
            Err(error) => return self.latch(CollectionSchedulerError::Backend(error)),
        }
        Ok(Some(dispatch))
    }

    fn active_id(&self) -> Option<u64> {
        self.active_dispatch
            .as_ref()
            .map(|dispatch| dispatch.request.id)
    }

    fn is_busy_activating(&self, id: u64) -> bool {
        self.activating.is_some_and(|active| active.id == id) || self.load_completions.contains(&id)
    }

    fn is_busy_deactivating(&self, id: u64) -> bool {
        self.deactivating.is_some_and(|active| active.id == id)
            || self.unload_completions.contains(&id)
    }

    fn is_collection_active(&self, id: u64) -> bool {
        self.active_id() == Some(id) || self.is_busy_activating(id) || self.is_busy_deactivating(id)
    }
}

fn front_end_rules() -> [DispatchRule; 6] {
    use DispatchRule::{Load, Unload};
    use LoadSelection::Best;
    use StreamKind::{Presentation, Simulation, Texture};

    [
        Unload(Texture),
        Unload(Presentation),
        Unload(Simulation),
        Load {
            kind: Simulation,
            selection: Best,
            require_loaded_below_one: true,
        },
        Load {
            kind: Presentation,
            selection: Best,
            require_loaded_below_one: true,
        },
        Load {
            kind: Texture,
            selection: Best,
            require_loaded_below_one: true,
        },
    ]
}

fn regular_rules(collection_defer: u32, loaded_counts: [i32; 3]) -> [DispatchRule; 6] {
    use DispatchRule::{Load, Unload};
    use LoadSelection::{Best, Next};
    use StreamKind::{Presentation, Simulation, Texture};

    if collection_defer != 0 {
        return [
            Unload(Presentation),
            Unload(Simulation),
            Load {
                kind: Simulation,
                selection: Best,
                require_loaded_below_one: false,
            },
            Load {
                kind: Presentation,
                selection: Best,
                require_loaded_below_one: false,
            },
            Unload(Texture),
            Load {
                kind: Texture,
                selection: Best,
                require_loaded_below_one: false,
            },
        ];
    }
    [
        Unload(Presentation),
        Unload(Simulation),
        Unload(Texture),
        Load {
            kind: Simulation,
            selection: Next,
            require_loaded_below_one: false,
        },
        Load {
            kind: Presentation,
            selection: Next,
            require_loaded_below_one: false,
        },
        Load {
            kind: Texture,
            selection: if loaded_counts[kind_index(Texture)] < 1 {
                Best
            } else {
                Next
            },
            require_loaded_below_one: false,
        },
    ]
}

const fn kind_index(kind: StreamKind) -> usize {
    match kind {
        StreamKind::Presentation => 0,
        StreamKind::Simulation => 1,
        StreamKind::Texture => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    use crate::{
        game_stream::StreamFile, mounted_stream_backend::StreamRegistrationSink,
        streaming::FocusHandle,
    };
    use skate2_formats::stream::{CollectionMap, CollectionMapEntry};

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        LifecycleCount(u64),
        LoadComplete(u64, [i32; 3]),
        UnloadComplete(u64, [i32; 3]),
        Poll(Option<u64>),
        PendingRequests,
        GarbageRelease,
        FrontEndOrder,
        CollectionDefer,
        BeginLoad(u64, Arc<str>, u64, u32),
        BeginUnload(u64),
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[derive(Debug)]
    struct TestOperation {
        complete: bool,
    }

    struct TestBackend {
        events: Rc<RefCell<Vec<Event>>>,
        garbage_release: u32,
        front_end_order: u32,
        collection_defer: u32,
        controls_after_next_poll: Option<(u32, u32)>,
        complete_new_operations: bool,
        fail_poll: bool,
    }

    impl CollectionOperationBackend for TestBackend {
        type Operation = TestOperation;
        type Error = TestError;

        fn poll_manager(
            &mut self,
            active: Option<ActiveCollectionOperation<'_, Self::Operation>>,
        ) -> Result<CollectionOperationPoll, Self::Error> {
            let id = active.as_ref().map(|active| active.dispatch.request.id);
            self.events.borrow_mut().push(Event::Poll(id));
            if let Some((front_end_order, collection_defer)) = self.controls_after_next_poll.take()
            {
                self.front_end_order = front_end_order;
                self.collection_defer = collection_defer;
            }
            if self.fail_poll {
                return Err(TestError("poll"));
            }
            Ok(match active {
                Some(active) if active.operation.complete => CollectionOperationPoll::Complete,
                _ => CollectionOperationPoll::Pending,
            })
        }

        fn garbage_release_index(&mut self) -> u32 {
            self.events.borrow_mut().push(Event::GarbageRelease);
            self.garbage_release
        }

        fn front_end_collection_order(&mut self) -> u32 {
            self.events.borrow_mut().push(Event::FrontEndOrder);
            self.front_end_order
        }

        fn collection_defer(&mut self) -> u32 {
            self.events.borrow_mut().push(Event::CollectionDefer);
            self.collection_defer
        }

        fn has_pending_requests(&mut self) -> Result<bool, Self::Error> {
            self.events.borrow_mut().push(Event::PendingRequests);
            Ok(false)
        }

        fn begin_load(
            &mut self,
            dispatch: &CollectionDispatch,
            metadata: &CollectionRegistrationSnapshot,
        ) -> Result<Self::Operation, Self::Error> {
            self.events.borrow_mut().push(Event::BeginLoad(
                dispatch.request.id,
                Arc::clone(&metadata.path),
                metadata.offset,
                metadata.size,
            ));
            Ok(TestOperation {
                complete: self.complete_new_operations,
            })
        }

        fn begin_unload(
            &mut self,
            dispatch: &CollectionDispatch,
        ) -> Result<Self::Operation, Self::Error> {
            self.events
                .borrow_mut()
                .push(Event::BeginUnload(dispatch.request.id));
            Ok(TestOperation {
                complete: self.complete_new_operations,
            })
        }
    }

    struct TestLifecycle {
        events: Rc<RefCell<Vec<Event>>>,
        observed: HashMap<u64, Option<u8>>,
        load_resolution: CollectionLoadResolution,
        unload_resolution: CollectionUnloadResolution,
        fail: bool,
    }

    impl CollectionLifecycleBackend for TestLifecycle {
        type Error = TestError;

        fn lifecycle_count(&mut self, id: u64) -> Result<Option<u8>, Self::Error> {
            self.events.borrow_mut().push(Event::LifecycleCount(id));
            if self.fail {
                return Err(TestError("lifecycle"));
            }
            Ok(self.observed.get(&id).copied().flatten())
        }

        fn load_complete(
            &mut self,
            id: u64,
            loaded_counts: [i32; 3],
        ) -> Result<CollectionLoadResolution, Self::Error> {
            self.events
                .borrow_mut()
                .push(Event::LoadComplete(id, loaded_counts));
            if self.fail {
                Err(TestError("lifecycle"))
            } else {
                Ok(self.load_resolution)
            }
        }

        fn unload_complete(
            &mut self,
            id: u64,
            loaded_counts: [i32; 3],
        ) -> Result<CollectionUnloadResolution, Self::Error> {
            self.events
                .borrow_mut()
                .push(Event::UnloadComplete(id, loaded_counts));
            if self.fail {
                Err(TestError("lifecycle"))
            } else {
                Ok(self.unload_resolution)
            }
        }
    }

    type Scheduler = CollectionStreamScheduler<TestBackend, TestLifecycle>;

    fn make_catalog(ids: &[u64]) -> WorldStreamCatalog {
        let mut catalog = WorldStreamCatalog::new();
        let file = StreamFile::<(), CatalogError>::load("world/", "test", StreamKind::Presentation)
            .unwrap();
        let map = CollectionMap {
            version: 3,
            stamp: 4,
            stream_format: 5,
            name_table_offset: 0,
            collections: ids
                .iter()
                .enumerate()
                .map(|(index, &id)| CollectionMapEntry {
                    id,
                    parent_id: id + 1000,
                    payload_offset: u32::try_from(index).unwrap() + 0x100,
                    payload_size: u32::try_from(index).unwrap() + 0x200,
                    asset_count: u32::try_from(index).unwrap(),
                    filename: format!("collection_{id}"),
                    name_offset: 0,
                    alignment_pad: 0,
                })
                .collect(),
        };
        catalog
            .collection_map_loaded(file.registration_context(), &map)
            .unwrap();
        catalog
    }

    fn scheduler(
        ids: &[u64],
        complete: bool,
        loaded_counts: [i32; 3],
    ) -> (Scheduler, Rc<RefCell<Vec<Event>>>) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let backend = TestBackend {
            events: Rc::clone(&events),
            garbage_release: 10,
            front_end_order: 0,
            collection_defer: 0,
            controls_after_next_poll: None,
            complete_new_operations: complete,
            fail_poll: false,
        };
        let lifecycle = TestLifecycle {
            events: Rc::clone(&events),
            observed: HashMap::new(),
            load_resolution: CollectionLoadResolution {
                target_count: None,
                loaded_counts,
            },
            unload_resolution: CollectionUnloadResolution {
                target_count: None,
                loaded_counts,
                garbage_head: 0,
            },
            fail: false,
        };
        (
            CollectionStreamScheduler::new(make_catalog(ids), backend, lifecycle, loaded_counts, 0),
            events,
        )
    }

    fn request(
        id: u64,
        kind: CollectionRequestKind,
        distance: f32,
        order: i32,
    ) -> CollectionRequest {
        CollectionRequest {
            handle: FocusHandle::new(),
            kind,
            id,
            distance_xz: distance,
            stream_order: order,
        }
    }

    const fn controls() -> CollectionSchedulerControls {
        CollectionSchedulerControls { loading: 0 }
    }

    #[test]
    fn fe_defer_and_normal_policies_choose_the_exact_first_branch() {
        let (mut fe, _) = scheduler(&[1, 2, 3], false, [0; 3]);
        fe.backend.front_end_order = 1;
        fe.queue_request(
            StreamKind::Presentation,
            request(1, CollectionRequestKind::Unload, 0.0, 0),
        )
        .unwrap();
        fe.queue_request(
            StreamKind::Texture,
            request(2, CollectionRequestKind::Unload, 0.0, 0),
        )
        .unwrap();
        fe.queue_request(
            StreamKind::Simulation,
            request(3, CollectionRequestKind::Unload, 0.0, 0),
        )
        .unwrap();
        assert_eq!(
            fe.update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            2
        );

        let (mut defer, _) = scheduler(&[4, 5], false, [0; 3]);
        defer.backend.collection_defer = 1;
        defer
            .queue_request(
                StreamKind::Texture,
                request(4, CollectionRequestKind::Unload, 0.0, 0),
            )
            .unwrap();
        defer
            .queue_request(
                StreamKind::Simulation,
                request(5, CollectionRequestKind::Load, 2.0, 0),
            )
            .unwrap();
        assert_eq!(
            defer
                .update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            5
        );

        let (mut normal, _) = scheduler(&[6, 7], false, [0; 3]);
        normal
            .queue_request(
                StreamKind::Texture,
                request(6, CollectionRequestKind::Unload, 0.0, 0),
            )
            .unwrap();
        normal
            .queue_request(
                StreamKind::Simulation,
                request(7, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert_eq!(
            normal
                .update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            6
        );
    }

    #[test]
    fn texture_normal_mode_switches_between_best_and_next() {
        let (mut best, _) = scheduler(&[1, 2], false, [0, 0, 0]);
        best.queue_request(
            StreamKind::Texture,
            request(1, CollectionRequestKind::Load, 9.0, 0),
        )
        .unwrap();
        best.queue_request(
            StreamKind::Texture,
            request(2, CollectionRequestKind::Load, 1.0, 1),
        )
        .unwrap();
        assert_eq!(
            best.update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            2
        );

        let (mut next, _) = scheduler(&[1, 2], false, [0, 0, 1]);
        next.queue_request(
            StreamKind::Texture,
            request(1, CollectionRequestKind::Load, 9.0, 0),
        )
        .unwrap();
        next.queue_request(
            StreamKind::Texture,
            request(2, CollectionRequestKind::Load, 1.0, 1),
        )
        .unwrap();
        assert_eq!(
            next.update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            1
        );
    }

    #[test]
    fn fe_loaded_count_gate_falls_through_to_next_kind() {
        let (mut scheduler, _) = scheduler(&[1, 2], false, [0, 1, 0]);
        scheduler.backend.front_end_order = 1;
        scheduler
            .queue_request(
                StreamKind::Simulation,
                request(1, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        scheduler
            .queue_request(
                StreamKind::Presentation,
                request(2, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert_eq!(
            scheduler
                .update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            2
        );
    }

    #[test]
    fn empty_fe_pass_falls_through_to_regular_pass_but_refusal_does_not() {
        let (mut fallthrough, _) = scheduler(&[1], false, [1, 1, 1]);
        fallthrough.backend.front_end_order = 1;
        fallthrough
            .queue_request(
                StreamKind::Simulation,
                request(1, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert_eq!(
            fallthrough
                .update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            1
        );

        let (mut refused, _) = scheduler(&[2, 3], true, [-1, 0, 0]);
        refused.backend.front_end_order = 1;
        refused
            .queue_request(
                StreamKind::Simulation,
                request(2, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        refused.update(controls()).unwrap();
        refused.lifecycle.load_resolution = CollectionLoadResolution {
            target_count: Some(4),
            loaded_counts: [-1, 0, 0],
        };
        refused.lifecycle.observed.insert(2, Some(3));
        refused.update(controls()).unwrap();
        refused
            .queue_request(
                StreamKind::Simulation,
                request(2, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        refused
            .queue_request(
                StreamKind::Presentation,
                request(3, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert!(refused.update(controls()).unwrap().dispatched.is_none());
        assert_eq!(
            refused.queue(StreamKind::Simulation).pending_loads().len(),
            1
        );
        assert_eq!(
            refused
                .queue(StreamKind::Presentation)
                .pending_loads()
                .len(),
            1
        );
    }

    #[test]
    fn unsigned_delay_guard_and_busy_first_candidate_leave_queue_unchanged() {
        let (mut scheduler, _) = scheduler(&[1, 2], true, [0; 3]);
        scheduler.load_delay = 11;
        scheduler
            .queue_request(
                StreamKind::Simulation,
                request(1, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert!(scheduler.update(controls()).unwrap().dispatched.is_none());
        scheduler.load_delay = 10;
        assert_eq!(
            scheduler
                .queue(StreamKind::Simulation)
                .pending_loads()
                .len(),
            1
        );

        assert_eq!(
            scheduler
                .update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            1
        );
        scheduler.lifecycle.load_resolution = CollectionLoadResolution {
            target_count: Some(9),
            loaded_counts: [0; 3],
        };
        scheduler.lifecycle.observed.insert(1, Some(8));
        scheduler
            .queue_request(
                StreamKind::Simulation,
                request(1, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        scheduler
            .queue_request(
                StreamKind::Simulation,
                request(2, CollectionRequestKind::Load, 1.0, 1),
            )
            .unwrap();
        assert!(scheduler.update(controls()).unwrap().dispatched.is_none());
        assert_eq!(
            scheduler
                .queue(StreamKind::Simulation)
                .pending_loads()
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn unsigned_delay_comparison_covers_less_equal_and_wrap_shaped_high() {
        let (mut less, _) = scheduler(&[1], false, [0; 3]);
        less.load_delay = 9;
        less.queue_request(
            StreamKind::Simulation,
            request(1, CollectionRequestKind::Load, 0.0, 0),
        )
        .unwrap();
        assert!(less.update(controls()).unwrap().dispatched.is_some());

        let (mut equal, _) = scheduler(&[1], false, [0; 3]);
        equal.load_delay = 10;
        equal
            .queue_request(
                StreamKind::Simulation,
                request(1, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert!(equal.update(controls()).unwrap().dispatched.is_some());

        let (mut high, _) = scheduler(&[1], false, [0; 3]);
        high.load_delay = u32::MAX;
        high.queue_request(
            StreamKind::Simulation,
            request(1, CollectionRequestKind::Load, 0.0, 0),
        )
        .unwrap();
        assert!(high.update(controls()).unwrap().dispatched.is_none());
        assert_eq!(high.queue(StreamKind::Simulation).pending_loads().len(), 1);
    }

    #[test]
    fn unload_completion_refreshes_persistent_delay_before_same_tick_selection() {
        let (mut permit_to_block, _) = scheduler(&[1], false, [0; 3]);
        permit_to_block.unload_completions.push_back(90);
        permit_to_block.lifecycle.unload_resolution = CollectionUnloadResolution {
            target_count: None,
            loaded_counts: [0; 3],
            garbage_head: 11,
        };
        permit_to_block
            .queue_request(
                StreamKind::Simulation,
                request(1, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert!(
            permit_to_block
                .update(controls())
                .unwrap()
                .dispatched
                .is_none()
        );
        assert_eq!(permit_to_block.load_delay(), 11);
        assert!(
            permit_to_block
                .update(controls())
                .unwrap()
                .dispatched
                .is_none()
        );
        assert_eq!(permit_to_block.load_delay(), 11);

        let (mut block_to_permit, _) = scheduler(&[2], false, [0; 3]);
        block_to_permit.load_delay = 11;
        block_to_permit.unload_completions.push_back(91);
        block_to_permit.lifecycle.unload_resolution = CollectionUnloadResolution {
            target_count: None,
            loaded_counts: [0; 3],
            garbage_head: 10,
        };
        block_to_permit
            .queue_request(
                StreamKind::Simulation,
                request(2, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert_eq!(
            block_to_permit
                .update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            2
        );
        assert_eq!(block_to_permit.load_delay(), 10);
    }

    #[test]
    fn live_policy_samples_follow_first_poll_and_control_fallback_rules() {
        let (mut fe, events) = scheduler(&[1, 2], false, [0; 3]);
        fe.backend.controls_after_next_poll = Some((1, 0));
        fe.queue_request(
            StreamKind::Presentation,
            request(1, CollectionRequestKind::Unload, 0.0, 0),
        )
        .unwrap();
        fe.queue_request(
            StreamKind::Texture,
            request(2, CollectionRequestKind::Unload, 0.0, 0),
        )
        .unwrap();
        assert_eq!(
            fe.update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            2
        );
        assert_eq!(
            &*events.borrow(),
            &[
                Event::Poll(None),
                Event::FrontEndOrder,
                Event::BeginUnload(2),
                Event::Poll(Some(2)),
            ]
        );

        let (mut defer, events) = scheduler(&[3, 4], false, [0; 3]);
        defer.backend.controls_after_next_poll = Some((0, 1));
        defer
            .queue_request(
                StreamKind::Texture,
                request(3, CollectionRequestKind::Unload, 0.0, 0),
            )
            .unwrap();
        defer
            .queue_request(
                StreamKind::Simulation,
                request(4, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert_eq!(
            defer
                .update(controls())
                .unwrap()
                .dispatched
                .unwrap()
                .request
                .id,
            4
        );
        assert_eq!(
            &events.borrow()[..4],
            &[
                Event::Poll(None),
                Event::FrontEndOrder,
                Event::CollectionDefer,
                Event::GarbageRelease,
            ]
        );
    }

    #[test]
    fn live_policy_is_not_sampled_while_active_or_after_attempted_fe_rule() {
        let (mut active, events) = scheduler(&[1], false, [0; 3]);
        active
            .queue_request(
                StreamKind::Simulation,
                request(1, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        active.update(controls()).unwrap();
        events.borrow_mut().clear();
        active.update(controls()).unwrap();
        assert_eq!(&*events.borrow(), &[Event::Poll(Some(1))]);

        let (mut refused, events) = scheduler(&[2], false, [0; 3]);
        refused.backend.front_end_order = 1;
        refused.load_delay = 11;
        refused
            .queue_request(
                StreamKind::Simulation,
                request(2, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        assert!(refused.update(controls()).unwrap().dispatched.is_none());
        assert_eq!(
            &*events.borrow(),
            &[
                Event::Poll(None),
                Event::FrontEndOrder,
                Event::GarbageRelease
            ]
        );
    }

    #[test]
    fn manager_poll_dispatch_and_immediate_poll_order_and_metadata_are_exact() {
        let (mut scheduler, events) = scheduler(&[7], false, [0; 3]);
        scheduler
            .queue_request(
                StreamKind::Simulation,
                request(7, CollectionRequestKind::Load, 1.0, 0),
            )
            .unwrap();
        let result = scheduler.update(controls()).unwrap();
        assert_eq!(result.manager_polls, 2);
        assert_eq!(
            &*events.borrow(),
            &[
                Event::Poll(None),
                Event::FrontEndOrder,
                Event::CollectionDefer,
                Event::GarbageRelease,
                Event::BeginLoad(7, Arc::from("world/\\test\\collection_7.xsf"), 0x100, 0x200),
                Event::Poll(Some(7)),
            ]
        );
    }

    #[test]
    fn completion_is_not_reconciled_until_next_outer_update() {
        let (mut scheduler, events) = scheduler(&[3], true, [0; 3]);
        scheduler.lifecycle.load_resolution = CollectionLoadResolution {
            target_count: Some(5),
            loaded_counts: [4, 5, 6],
        };
        scheduler
            .queue_request(
                StreamKind::Simulation,
                request(3, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        scheduler.update(controls()).unwrap();
        assert_eq!(
            scheduler
                .load_completions()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![3]
        );
        assert!(scheduler.activating().is_none());
        scheduler.update(controls()).unwrap();
        assert_eq!(
            scheduler.activating(),
            Some(CollectionLifecycleProgress {
                id: 3,
                target_count: 5
            })
        );
        assert_eq!(scheduler.loaded_counts(), [4, 5, 6]);
        assert!(events.borrow().contains(&Event::LoadComplete(3, [0; 3])));
    }

    #[test]
    fn lifecycle_progress_clears_only_on_missing_or_equal_counter() {
        let (mut scheduler, events) = scheduler(&[3], true, [0; 3]);
        scheduler.lifecycle.load_resolution = CollectionLoadResolution {
            target_count: Some(5),
            loaded_counts: [0; 3],
        };
        scheduler.lifecycle.observed.insert(3, Some(6));
        scheduler
            .queue_request(
                StreamKind::Simulation,
                request(3, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        scheduler.update(controls()).unwrap();
        scheduler.update(controls()).unwrap();
        scheduler.update(controls()).unwrap();
        assert!(scheduler.activating().is_some());

        scheduler.lifecycle.observed.insert(3, Some(5));
        events.borrow_mut().clear();
        scheduler.update(controls()).unwrap();
        assert!(scheduler.activating().is_none());
        assert_eq!(events.borrow()[0], Event::LifecycleCount(3));

        scheduler.activating = Some(CollectionLifecycleProgress {
            id: 31,
            target_count: 7,
        });
        scheduler.lifecycle.observed.insert(31, None);
        scheduler.update(controls()).unwrap();
        assert!(scheduler.activating().is_none());
    }

    #[test]
    fn deactivation_reconciles_before_activation_and_both_can_coexist() {
        let (mut scheduler, events) = scheduler(&[1, 2], true, [0; 3]);
        scheduler.lifecycle.load_resolution = CollectionLoadResolution {
            target_count: Some(3),
            loaded_counts: [1, 0, 0],
        };
        scheduler.lifecycle.unload_resolution = CollectionUnloadResolution {
            target_count: Some(4),
            loaded_counts: [0, 0, 0],
            garbage_head: 0,
        };
        scheduler.lifecycle.observed.insert(1, Some(2));
        scheduler.lifecycle.observed.insert(2, Some(3));
        scheduler
            .queue_request(
                StreamKind::Presentation,
                request(1, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        scheduler.update(controls()).unwrap();
        scheduler.update(controls()).unwrap();
        scheduler
            .queue_request(
                StreamKind::Presentation,
                request(2, CollectionRequestKind::Unload, 0.0, 0),
            )
            .unwrap();
        scheduler.update(controls()).unwrap();
        scheduler.update(controls()).unwrap();
        assert!(scheduler.activating().is_some());
        assert!(scheduler.deactivating().is_some());
        events.borrow_mut().clear();
        scheduler.update(controls()).unwrap();
        assert_eq!(
            &events.borrow()[..2],
            &[Event::LifecycleCount(2), Event::LifecycleCount(1)]
        );
    }

    #[test]
    fn both_completion_fronts_can_start_in_native_unload_then_load_order() {
        let (mut scheduler, events) = scheduler(&[], false, [3, 4, 5]);
        scheduler.unload_completions.push_back(10);
        scheduler.load_completions.push_back(20);
        scheduler.lifecycle.unload_resolution = CollectionUnloadResolution {
            target_count: Some(7),
            loaded_counts: [2, 4, 5],
            garbage_head: 0,
        };
        scheduler.lifecycle.load_resolution = CollectionLoadResolution {
            target_count: Some(8),
            loaded_counts: [2, 5, 5],
        };
        scheduler.update(controls()).unwrap();
        assert_eq!(
            &*events.borrow(),
            &[
                Event::UnloadComplete(10, [3, 4, 5]),
                Event::LoadComplete(20, [2, 4, 5]),
                Event::Poll(None),
                Event::FrontEndOrder,
                Event::CollectionDefer,
            ]
        );
        assert_eq!(scheduler.loaded_counts(), [2, 5, 5]);
        assert_eq!(scheduler.deactivating().unwrap().id, 10);
        assert_eq!(scheduler.activating().unwrap().id, 20);
    }

    #[test]
    fn busy_completion_front_blocks_without_scanning_later_ids() {
        let (mut scheduler, events) = scheduler(&[], false, [0; 3]);
        scheduler.deactivating = Some(CollectionLifecycleProgress {
            id: 10,
            target_count: 1,
        });
        scheduler.lifecycle.observed.insert(10, Some(0));
        scheduler.load_completions.extend([10, 20]);
        scheduler.update(controls()).unwrap();
        assert_eq!(
            scheduler
                .load_completions()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![10, 20]
        );
        assert!(
            !events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::LoadComplete(..)))
        );
    }

    #[test]
    fn loading_gate_calls_no_collaborator() {
        let (mut scheduler, events) = scheduler(&[], false, [0; 3]);
        let result = scheduler
            .update(CollectionSchedulerControls { loading: 1 })
            .unwrap();
        assert_eq!(result.manager_polls, 0);
        assert!(events.borrow().is_empty());

        let result = scheduler
            .update(CollectionSchedulerControls { loading: 2 })
            .unwrap();
        assert_eq!(result.manager_polls, 1);
        assert_eq!(
            &*events.borrow(),
            &[
                Event::Poll(None),
                Event::FrontEndOrder,
                Event::CollectionDefer
            ]
        );
    }

    #[test]
    fn nonzero_update_lock_gate_calls_no_collaborator_and_errors_release_lock() {
        let (mut scheduler, events) = scheduler(&[], false, [0; 3]);
        scheduler.update_locked = true;
        let result = scheduler.update(controls()).unwrap();
        assert_eq!(result.manager_polls, 0);
        assert!(events.borrow().is_empty());

        scheduler.update_locked = false;
        scheduler.backend.fail_poll = true;
        assert!(scheduler.update(controls()).is_err());
        assert!(!scheduler.update_locked);
    }

    #[test]
    fn pending_requests_short_circuits_outer_state_before_inner_backend() {
        let (mut scheduler, events) = scheduler(&[1], false, [0; 3]);
        assert!(!scheduler.pending_requests().unwrap());
        assert_eq!(&*events.borrow(), &[Event::PendingRequests]);
        scheduler
            .queue_request(
                StreamKind::Simulation,
                request(1, CollectionRequestKind::Load, 0.0, 0),
            )
            .unwrap();
        events.borrow_mut().clear();
        assert!(scheduler.pending_requests().unwrap());
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn first_fault_is_latched_and_prevents_later_calls() {
        let (mut scheduler, events) = scheduler(&[], false, [0; 3]);
        scheduler.backend.fail_poll = true;
        let first = scheduler.update(controls()).unwrap_err();
        let count = events.borrow().len();
        let second = scheduler.update(controls()).unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(events.borrow().len(), count);
        assert!(
            scheduler
                .queue_request(
                    StreamKind::Simulation,
                    request(1, CollectionRequestKind::Load, 0.0, 0)
                )
                .is_err()
        );
    }

    #[test]
    fn thirty_third_completion_is_a_typed_terminal_error() {
        let ids = (1..=34).collect::<Vec<_>>();
        let (mut scheduler, _) = scheduler(&ids, true, [0; 3]);
        scheduler.lifecycle.load_resolution = CollectionLoadResolution {
            target_count: Some(2),
            loaded_counts: [0; 3],
        };
        scheduler.lifecycle.observed.insert(1, Some(1));
        for &id in &ids {
            scheduler
                .queue_request(
                    StreamKind::Simulation,
                    request(
                        id,
                        CollectionRequestKind::Load,
                        0.0,
                        i32::try_from(id).unwrap(),
                    ),
                )
                .unwrap();
        }
        scheduler.update(controls()).unwrap();
        scheduler.update(controls()).unwrap();
        for _ in 0..31 {
            scheduler.update(controls()).unwrap();
        }
        assert_eq!(scheduler.load_completions().len(), 32);
        let error = scheduler.update(controls()).unwrap_err();
        assert!(matches!(
            &*error,
            CollectionSchedulerError::CompletionQueueFull {
                kind: CollectionOperationKind::Load,
                id: 34
            }
        ));
        assert_eq!(scheduler.active_dispatch().unwrap().request.id, 34);
        assert_eq!(scheduler.load_completions().len(), 32);
    }

    #[test]
    fn unload_completion_uses_its_independent_thirty_two_entry_cap() {
        let (mut scheduler, _) = scheduler(&[], true, [0; 3]);
        scheduler.unload_completions.extend(1..=32);
        scheduler.active_dispatch = Some(CollectionDispatch {
            stream_kind: StreamKind::Texture,
            request: request(33, CollectionRequestKind::Unload, 0.0, 0),
        });
        scheduler.active_operation = Some(TestOperation { complete: true });

        let error = scheduler.poll_manager().unwrap_err();
        assert!(matches!(
            &*error,
            CollectionSchedulerError::CompletionQueueFull {
                kind: CollectionOperationKind::Unload,
                id: 33
            }
        ));
        assert_eq!(scheduler.active_dispatch().unwrap().request.id, 33);
        assert_eq!(scheduler.unload_completions().len(), 32);
        assert_eq!(scheduler.load_completions().len(), 0);
    }
}
