//! Unified Simulation collection notification ordering.
//!
//! Immediate registration, deferred registration, deregistration, and
//! callback-only notifications share one outer FIFO. Deferred nodes are
//! represented by exact-order markers over [`crate::simulation_deferred_message_fifo`].
//! Callback capabilities are returned, never invoked or acknowledged here.
//!
//! This is a safe-host receiver/order layer, not the complete native notifier.
//! Registration starts require the caller to run the proven admission selector
//! after the native loaded-count increment and locked type lookup. `DReg`
//! instead requires its independent fresh locked lookup and pure classifier;
//! it performs no loaded-count mutation.
//! Missing-map assertions, queue locking/capacity, and loaded-hold release
//! remain outside this component. Resident-backed immediate messages retain
//! only their full collection ID until the receiver reaches the FIFO front;
//! registration and same-class deregistration then perform independent
//! resident lookups. The older prepared-plan entry points remain explicit
//! safe-host compatibility paths. The resident-backed deferred entry point
//! resolves at caller-invoked producer activation and resolves again at the
//! activation-zero receiver front. Each resident-backed individual Arena
//! message freezes the producer's full 64-bit Arena ID, resolves that ID from
//! the global Arena map only when the message reaches the FIFO front, and
//! keeps the selected asset alive across normal-Simulation then Location
//! dispatch. The current early-bound child ledger rejects a different plan
//! winner before effects; that rejection is explicit safe-host policy rather
//! than native behavior. Stream-class changes remain typed unsupported
//! boundaries rather than invented native receiver results.
//!
//! Native callbacks are nullable, non-owning pointers. `Option<RC>` and
//! `Option<DC>` safely own opaque host capabilities instead. They are frozen at
//! the corresponding Notify entry, retained until their separate FIFO send,
//! and returned to the host at dequeue; dropping the router never invokes them.

use crate::{
    collection_resident_streamer::CollectionId,
    simulation_collection_adapter::{
        PreparedSimulationCollection, SimulationCollectionDeregistrationError,
        SimulationCollectionDmoPass, SimulationCollectionFault, SimulationCollectionLocationPass,
        SimulationCollectionNormalPass, SimulationCollectionRegistration,
        SimulationCollectionRegistrationAdapter, SimulationCollectionRegistrationError,
    },
    simulation_collection_admission::SimulationCollectionDeregistrationRoute,
    simulation_deferred_message_fifo::{
        DeferredSimulationArenaProducer, DeferredSimulationBeginError,
        DeferredSimulationCallbackPermit, DeferredSimulationDispatchError,
        DeferredSimulationDispatchFault, DeferredSimulationEnqueueRejectionKind,
        DeferredSimulationEnqueueState, DeferredSimulationFinishProducer,
        DeferredSimulationMessageDispatch, DeferredSimulationMessageFifo,
        DeferredSimulationMessageKind, DeferredSimulationReceiverRejectionKind,
    },
    simulation_resident_collection::{
        PreparedSimulationResidentCollection, SimulationResidentArenaAsset,
        SimulationResidentCollectionPrepareFailure,
    },
    simulation_resident_registry::{
        SimulationResidentArenaLookup, SimulationResidentArenaLookupError,
        SimulationResidentLookup, SimulationResidentLookupError,
    },
};
use std::{
    collections::VecDeque,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Weak},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionStage {
    ImmediateQueued,
    Deferred,
    CallbackOnlyReady,
    Registered,
    Delivered,
    DeregistrationQueued,
    DeregistrationCallbackReady,
}

enum CallbackSlot<C> {
    Unbound,
    Bound(Option<C>),
}

impl<C> CallbackSlot<C> {
    const fn is_bound(&self) -> bool {
        matches!(self, Self::Bound(_))
    }
}

struct Session<RC, DC> {
    id: u64,
    collection_id: u64,
    has_collection_work: bool,
    stage: SessionStage,
    registration: Option<SimulationCollectionRegistration>,
    pending_registration_callback: CallbackSlot<RC>,
    fault_registration: Option<SimulationCollectionRegistration>,
    fault_callback: Option<RC>,
    pending_deregistration_callback: CallbackSlot<DC>,
    resident: Option<PreparedSimulationResidentCollection>,
    resident_prepare_failure: Option<SimulationResidentCollectionPrepareFailure>,
    fault_resident: Option<PreparedSimulationResidentCollection>,
    next_deferred_arena: usize,
}

#[derive(Clone, Copy)]
struct DeferredArenaIdentity {
    arena_index: usize,
    asset_id: CollectionId,
}

enum Message<RC, DC> {
    ImmediateRegistration {
        session_id: u64,
        prepared: Arc<PreparedSimulationCollection>,
        userdata_raw: u32,
    },
    ResidentImmediateRegistration {
        session_id: u64,
        collection_id: CollectionId,
        userdata_raw: u32,
    },
    DeferredMarker {
        session_id: u64,
        kind: DeferredSimulationMessageKind,
        arena: Option<DeferredArenaIdentity>,
    },
    RegistrationCallback {
        session_id: u64,
        callback: Option<RC>,
    },
    Deregistration {
        session_id: u64,
        registration: Option<SimulationCollectionRegistration>,
        userdata_raw: u32,
    },
    DeregistrationCallback {
        session_id: u64,
        callback: Option<DC>,
    },
}

/// Sticky safe-host router fault.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulationNotificationFault {
    session_id: u64,
    kind: SimulationNotificationFaultKind,
}

impl SimulationNotificationFault {
    #[must_use]
    pub const fn session_id(self) -> u64 {
        self.session_id
    }

    #[must_use]
    pub const fn kind(self) -> SimulationNotificationFaultKind {
        self.kind
    }
}

impl fmt::Display for SimulationNotificationFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Simulation notification router faulted in session {}: {:?}",
            self.session_id, self.kind
        )
    }
}

impl std::error::Error for SimulationNotificationFault {}

/// Exact reached boundary recorded by a sticky router fault.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationNotificationFaultKind {
    MissingSession,
    WrongSessionStage,
    MissingPreparedCollection,
    MissingRegistration,
    DeferredKindMismatch,
    DeferredReceiverRejected(DeferredSimulationReceiverRejectionKind),
    DeferredFault(DeferredSimulationDispatchFault),
    CollectionFault(SimulationCollectionFault),
    DispatchPanicked,
    RegistrationRejectedBeforeEffects,
    DeregistrationRejectedBeforeEffects,
    ResidentLookupUnavailable,
    RegistrationResidentMissing,
    RegistrationResidentLookup(SimulationResidentLookupError),
    RegistrationResidentPrepare,
    DeferredFinishResidentMissing,
    DeferredFinishResidentLookup(SimulationResidentLookupError),
    DeferredFinishResidentPrepare,
    DeferredFinishResidentOccurrenceMismatch,
    DeferredFinishResidentSnapshotMismatch,
    DeferredArenaResidentMissing,
    DeferredArenaResidentLookup(SimulationResidentArenaLookupError<std::convert::Infallible>),
    DeferredArenaMessageIdentityMismatch,
    DeferredArenaResolvedPlanMismatch,
    DeregistrationResidentMissing,
    DeregistrationResidentLookup(SimulationResidentLookupError),
    DeregistrationResidentPrepare,
    DeregistrationResidentOccurrenceMismatch,
    DeregistrationResidentSnapshotMismatch,
}

/// Pre-effect enqueue rejection class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationNotificationEnqueueRejectionKind {
    RouterFaulted(SimulationNotificationFault),
    ForeignRouter,
    UnknownSession,
    WrongStage,
    StreamClassMismatch,
    StorageAllocation,
    SessionIdExhausted,
    DeferredRejected(DeferredSimulationEnqueueRejectionKind),
    DeferredBeginFifoFaulted(DeferredSimulationDispatchFault),
    DeferredBeginSessionStorageAllocation { requested: usize },
    DeferredBeginSessionIdExhausted,
    DeferredBeginCollection(SimulationCollectionRegistrationError),
    DeferredBeginResidentMissing,
    DeferredBeginResidentLookup(SimulationResidentLookupError),
    DeferredBeginResidentPrepare,
    ResidentLookupUnavailable,
    DeferredArenaIdentityUnavailable,
}

/// Enqueue rejection that returns the exact consumed input.
pub struct SimulationNotificationEnqueueRejection<T> {
    kind: SimulationNotificationEnqueueRejectionKind,
    value: T,
}

impl<T> SimulationNotificationEnqueueRejection<T> {
    #[must_use]
    pub const fn kind(&self) -> SimulationNotificationEnqueueRejectionKind {
        self.kind
    }

    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for SimulationNotificationEnqueueRejection<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationNotificationEnqueueRejection")
            .field("kind", &self.kind)
            .field("value", &self.value)
            .finish()
    }
}

impl<T> fmt::Display for SimulationNotificationEnqueueRejection<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Simulation notification enqueue rejected: {:?}",
            self.kind
        )
    }
}

impl<T: fmt::Debug> std::error::Error for SimulationNotificationEnqueueRejection<T> {}

/// Exact resident state returned when deferred producer setup cannot finish.
#[derive(Debug)]
pub enum SimulationResidentDeferredBeginRecovery {
    /// Lookup did not produce an occurrence; the full input identity remains.
    Unresolved {
        collection_id: CollectionId,
        userdata_raw: u32,
    },
    /// Lookup produced an occurrence, but dense preparation failed.
    PreparationFailed {
        failure: SimulationResidentCollectionPrepareFailure,
        userdata_raw: u32,
    },
    /// Preparation succeeded, but later host session setup was rejected.
    Prepared {
        resident: PreparedSimulationResidentCollection,
        userdata_raw: u32,
    },
}

impl SimulationResidentDeferredBeginRecovery {
    #[must_use]
    pub fn collection_id(&self) -> CollectionId {
        match self {
            Self::Unresolved { collection_id, .. } => *collection_id,
            Self::PreparationFailed { failure, .. } => failure.occurrence().id(),
            Self::Prepared { resident, .. } => resident.collection_id(),
        }
    }

    #[must_use]
    pub const fn userdata_raw(&self) -> u32 {
        match self {
            Self::Unresolved { userdata_raw, .. }
            | Self::PreparationFailed { userdata_raw, .. }
            | Self::Prepared { userdata_raw, .. } => *userdata_raw,
        }
    }
}

/// Host diagnostic counts for one resident resolution point.
///
/// These counters observe calls made by this safe-host router. They are not
/// reconstructed native fields, counters, or synchronization state. Counts
/// saturate at `u64::MAX` so observation cannot introduce a new failure path.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SimulationResidentResolutionPointStats {
    lookup_attempts: u64,
    successful_preparations: u64,
}

impl SimulationResidentResolutionPointStats {
    #[must_use]
    pub const fn lookup_attempts(self) -> u64 {
        self.lookup_attempts
    }

    #[must_use]
    pub const fn successful_preparations(self) -> u64 {
        self.successful_preparations
    }

    /// Alias used by receiver points that perform a scoped lookup but do not
    /// construct a whole collection bundle.
    #[must_use]
    pub const fn successful_resolutions(self) -> u64 {
        self.successful_preparations
    }

    fn record_lookup(&mut self) {
        self.lookup_attempts = self.lookup_attempts.saturating_add(1);
    }

    fn record_preparation(&mut self) {
        self.successful_preparations = self.successful_preparations.saturating_add(1);
    }
}

/// Read-only host diagnostic snapshot of resident resolution activity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SimulationResidentResolutionStats {
    immediate: SimulationResidentResolutionPointStats,
    deferred_begin: SimulationResidentResolutionPointStats,
    deferred_arena: SimulationResidentResolutionPointStats,
    deferred_finish: SimulationResidentResolutionPointStats,
    deregistration: SimulationResidentResolutionPointStats,
}

impl SimulationResidentResolutionStats {
    #[must_use]
    pub const fn immediate(self) -> SimulationResidentResolutionPointStats {
        self.immediate
    }

    #[must_use]
    pub const fn deferred_begin(self) -> SimulationResidentResolutionPointStats {
        self.deferred_begin
    }

    #[must_use]
    pub const fn deferred_arena(self) -> SimulationResidentResolutionPointStats {
        self.deferred_arena
    }

    #[must_use]
    pub const fn deferred_finish(self) -> SimulationResidentResolutionPointStats {
        self.deferred_finish
    }

    #[must_use]
    pub const fn deregistration(self) -> SimulationResidentResolutionPointStats {
        self.deregistration
    }
}

/// Producer authority for the next deferred Arena message.
pub struct SimulationNotificationDeferredArenaProducer {
    domain: Weak<()>,
    session_id: u64,
    inner: DeferredSimulationArenaProducer,
}

/// Producer authority for the deferred activation-zero message.
pub struct SimulationNotificationDeferredFinishProducer {
    domain: Weak<()>,
    session_id: u64,
    inner: DeferredSimulationFinishProducer,
}

/// Next deferred producer state.
pub enum SimulationNotificationDeferredState {
    NextArena(SimulationNotificationDeferredArenaProducer),
    ReadyToFinish(SimulationNotificationDeferredFinishProducer),
}

/// Authority to append one separate registration callback message.
pub struct SimulationRegistrationCallbackPermit {
    domain: Weak<()>,
    session_id: u64,
    deferred: Option<DeferredSimulationCallbackPermit>,
}

/// Non-cloneable authority for one delivered registration occurrence.
pub struct SimulationCollectionOccurrence {
    domain: Weak<()>,
    session_id: u64,
    has_collection_work: bool,
}

impl SimulationCollectionOccurrence {
    #[must_use]
    pub const fn has_collection_work(&self) -> bool {
        self.has_collection_work
    }
}

/// Authority to append one separate deregistration callback message.
pub struct SimulationDeregistrationCallbackPermit {
    domain: Weak<()>,
    session_id: u64,
}

macro_rules! debug_authority {
    ($name:ident) => {
        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($name))
                    .field("session_id", &self.session_id)
                    .finish_non_exhaustive()
            }
        }
    };
}

debug_authority!(SimulationNotificationDeferredArenaProducer);
debug_authority!(SimulationNotificationDeferredFinishProducer);
debug_authority!(SimulationRegistrationCallbackPermit);
debug_authority!(SimulationCollectionOccurrence);
debug_authority!(SimulationDeregistrationCallbackPermit);

/// Registration callback-position output.
pub struct SimulationRegistrationNotification<RC> {
    collection_id: u64,
    occurrence: SimulationCollectionOccurrence,
    callback: Option<RC>,
}

impl<RC> SimulationRegistrationNotification<RC> {
    #[must_use]
    pub fn into_parts(self) -> (u64, SimulationCollectionOccurrence, Option<RC>) {
        (self.collection_id, self.occurrence, self.callback)
    }
}

/// Deregistration callback-position output.
pub struct SimulationDeregistrationNotification<DC> {
    collection_id: u64,
    callback: Option<DC>,
}

impl<DC> SimulationDeregistrationNotification<DC> {
    #[must_use]
    pub fn into_parts(self) -> (u64, Option<DC>) {
        (self.collection_id, self.callback)
    }
}

/// Result of dispatching at most one outer FIFO front.
pub enum SimulationNotificationDispatch<RC, DC> {
    Idle,
    ReceiverReturned,
    RegistrationCallbackDequeued(SimulationRegistrationNotification<RC>),
    DeregistrationCallbackDequeued(SimulationDeregistrationNotification<DC>),
}

/// One ordering domain for all modeled Simulation collection notifications.
pub struct SimulationCollectionNotificationRouter<RC, DC, N, L, D>
where
    N: SimulationCollectionNormalPass,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    queue: VecDeque<Message<RC, DC>>,
    deferred: DeferredSimulationMessageFifo<RC>,
    sessions: Vec<Session<RC, DC>>,
    domain: Arc<()>,
    next_session_id: u64,
    fault: Option<SimulationNotificationFault>,
    adapter: SimulationCollectionRegistrationAdapter<N, L, D>,
    resident_lookup: Option<SimulationResidentLookup<SimulationResidentArenaAsset>>,
    resident_arena_lookup: Option<SimulationResidentArenaLookup<SimulationResidentArenaAsset>>,
    resident_resolution_stats: SimulationResidentResolutionStats,
}

impl<RC, DC, N, L, D> SimulationCollectionNotificationRouter<RC, DC, N, L, D>
where
    N: SimulationCollectionNormalPass,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    /// Creates a fresh router and a fresh collection receiver from the three
    /// child passes. Existing collection registrations therefore cannot enter
    /// this ordering domain without a router occurrence.
    #[must_use]
    pub fn new(normal: N, location: L, dmo: D) -> Self {
        Self {
            queue: VecDeque::new(),
            deferred: DeferredSimulationMessageFifo::new(),
            sessions: Vec::new(),
            domain: Arc::new(()),
            next_session_id: 0,
            fault: None,
            adapter: SimulationCollectionRegistrationAdapter::new(normal, location, dmo),
            resident_lookup: None,
            resident_arena_lookup: None,
            resident_resolution_stats: SimulationResidentResolutionStats::default(),
        }
    }

    /// Creates a router with resident-backed lookup. Immediate, terminal
    /// deferred, and deregistration receiver work resolves at the FIFO front;
    /// deferred producer activation resolves when the caller begins that work.
    #[must_use]
    pub fn new_with_resident_lookup(
        normal: N,
        location: L,
        dmo: D,
        resident_lookup: SimulationResidentLookup<SimulationResidentArenaAsset>,
    ) -> Self {
        let mut router = Self::new(normal, location, dmo);
        router.resident_arena_lookup = Some(resident_lookup.arena_lookup());
        router.resident_lookup = Some(resident_lookup);
        router
    }

    /// Appends the immediate registration receiver message and returns the
    /// authority for its separately queued callback message.
    ///
    /// # Errors
    ///
    /// Returns the unchanged prepared collection and frozen callback if the
    /// router is faulted or cannot reserve the required host state.
    pub fn enqueue_immediate_registration(
        &mut self,
        prepared: PreparedSimulationCollection,
        userdata_raw: u32,
        callback: Option<RC>,
    ) -> Result<
        SimulationRegistrationCallbackPermit,
        SimulationNotificationEnqueueRejection<(PreparedSimulationCollection, Option<RC>)>,
    > {
        if let Some(fault) = self.fault {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::RouterFaulted(fault),
                (prepared, callback),
            ));
        }
        if self.sessions.try_reserve(1).is_err() || self.queue.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                (prepared, callback),
            ));
        }
        let session_id = match self.allocate_session_id() {
            Ok(id) => id,
            Err(kind) => return Err(rejection(kind, (prepared, callback))),
        };
        let collection_id = prepared.collection_id();
        self.sessions.push(Session {
            id: session_id,
            collection_id,
            has_collection_work: true,
            stage: SessionStage::ImmediateQueued,
            registration: None,
            pending_registration_callback: CallbackSlot::Bound(callback),
            fault_registration: None,
            fault_callback: None,
            pending_deregistration_callback: CallbackSlot::Unbound,
            resident: None,
            resident_prepare_failure: None,
            fault_resident: None,
            next_deferred_arena: 0,
        });
        self.queue.push_back(Message::ImmediateRegistration {
            session_id,
            prepared: Arc::new(prepared),
            userdata_raw,
        });
        Ok(self.registration_callback_permit(session_id, None))
    }

    /// Appends an immediate registration message containing only the native
    /// full-width collection ID. Resident lookup and dense preparation are
    /// deferred until this message reaches the receiver at the FIFO front.
    ///
    /// # Errors
    ///
    /// Returns the unchanged ID and frozen callback if the router has no
    /// resident lookup domain, is faulted, or cannot reserve host state.
    pub fn enqueue_resident_immediate_registration(
        &mut self,
        collection_id: CollectionId,
        userdata_raw: u32,
        callback: Option<RC>,
    ) -> Result<
        SimulationRegistrationCallbackPermit,
        SimulationNotificationEnqueueRejection<(CollectionId, Option<RC>)>,
    > {
        if let Some(fault) = self.fault {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::RouterFaulted(fault),
                (collection_id, callback),
            ));
        }
        if self.resident_lookup.is_none() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::ResidentLookupUnavailable,
                (collection_id, callback),
            ));
        }
        if self.sessions.try_reserve(1).is_err() || self.queue.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                (collection_id, callback),
            ));
        }
        let session_id = match self.allocate_session_id() {
            Ok(id) => id,
            Err(kind) => return Err(rejection(kind, (collection_id, callback))),
        };
        self.sessions.push(Session {
            id: session_id,
            collection_id: collection_id.0,
            has_collection_work: true,
            stage: SessionStage::ImmediateQueued,
            registration: None,
            pending_registration_callback: CallbackSlot::Bound(callback),
            fault_registration: None,
            fault_callback: None,
            pending_deregistration_callback: CallbackSlot::Unbound,
            resident: None,
            resident_prepare_failure: None,
            fault_resident: None,
            next_deferred_arena: 0,
        });
        self.queue
            .push_back(Message::ResidentImmediateRegistration {
                session_id,
                collection_id,
                userdata_raw,
            });
        Ok(self.registration_callback_permit(session_id, None))
    }

    /// Retains a deferred registration and returns its first producer state.
    ///
    /// # Errors
    ///
    /// Returns the unchanged prepared collection and frozen callback if the
    /// router or child FIFO rejects the begin operation before a deferred
    /// session is established.
    pub fn begin_deferred_registration(
        &mut self,
        prepared: PreparedSimulationCollection,
        userdata_raw: u32,
        callback: Option<RC>,
    ) -> Result<
        SimulationNotificationDeferredState,
        SimulationNotificationEnqueueRejection<(PreparedSimulationCollection, Option<RC>)>,
    > {
        if let Some(fault) = self.fault {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::RouterFaulted(fault),
                (prepared, callback),
            ));
        }
        if self.sessions.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                (prepared, callback),
            ));
        }
        let session_id = match self.allocate_session_id() {
            Ok(id) => id,
            Err(kind) => return Err(rejection(kind, (prepared, callback))),
        };
        let collection_id = prepared.collection_id();
        let inner = match self
            .deferred
            .begin(&mut self.adapter, &prepared, userdata_raw)
        {
            Ok(state) => state,
            Err(error) => {
                return Err(rejection(
                    match error {
                        DeferredSimulationBeginError::FifoFaulted(fault) => {
                            SimulationNotificationEnqueueRejectionKind::DeferredBeginFifoFaulted(
                                fault,
                            )
                        }
                        DeferredSimulationBeginError::SessionStorageAllocation { requested } => {
                            SimulationNotificationEnqueueRejectionKind::DeferredBeginSessionStorageAllocation {
                                requested,
                            }
                        }
                        DeferredSimulationBeginError::SessionIdExhausted => {
                            SimulationNotificationEnqueueRejectionKind::DeferredBeginSessionIdExhausted
                        }
                        DeferredSimulationBeginError::Collection(error) => {
                            SimulationNotificationEnqueueRejectionKind::DeferredBeginCollection(
                                error,
                            )
                        }
                    },
                    (prepared, callback),
                ));
            }
        };
        self.sessions.push(Session {
            id: session_id,
            collection_id,
            has_collection_work: true,
            stage: SessionStage::Deferred,
            registration: None,
            pending_registration_callback: CallbackSlot::Bound(callback),
            fault_registration: None,
            fault_callback: None,
            pending_deregistration_callback: CallbackSlot::Unbound,
            resident: None,
            resident_prepare_failure: None,
            fault_resident: None,
            next_deferred_arena: 0,
        });
        Ok(self.wrap_deferred_state(session_id, inner))
    }

    /// Resolves and retains the collection when a deferred pending row becomes
    /// active, then returns its first producer state.
    ///
    /// This lookup corresponds to `SetupNextActiveCollectionReg`, not to the
    /// earlier Notify admission and not to the later per-Arena receivers. No
    /// Arena, activation-zero, or Callback node is appended by this method.
    ///
    /// # Errors
    ///
    /// Returns the frozen callback with the exact latest recoverable resident
    /// state. Lookup failures retain the unresolved full ID, preparation
    /// failures retain their occurrence, and later setup failures retain the
    /// complete prepared resident bundle.
    #[allow(clippy::result_large_err)] // Exact non-allocating recovery retains the owned resident state.
    #[allow(clippy::too_many_lines)] // Keeps native setup order and each recovery state visible.
    pub fn begin_resident_deferred_registration(
        &mut self,
        collection_id: CollectionId,
        userdata_raw: u32,
        callback: Option<RC>,
    ) -> Result<
        SimulationNotificationDeferredState,
        SimulationNotificationEnqueueRejection<(
            SimulationResidentDeferredBeginRecovery,
            Option<RC>,
        )>,
    > {
        let unresolved = || SimulationResidentDeferredBeginRecovery::Unresolved {
            collection_id,
            userdata_raw,
        };
        if let Some(fault) = self.fault {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::RouterFaulted(fault),
                (unresolved(), callback),
            ));
        }
        let Some(lookup) = self.resident_lookup.clone() else {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::ResidentLookupUnavailable,
                (unresolved(), callback),
            ));
        };
        if self.sessions.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                (unresolved(), callback),
            ));
        }
        self.resident_resolution_stats
            .deferred_begin
            .record_lookup();
        let occurrence = match lookup.lookup(collection_id) {
            Ok(Some(occurrence)) => occurrence,
            Ok(None) => {
                return Err(rejection(
                    SimulationNotificationEnqueueRejectionKind::DeferredBeginResidentMissing,
                    (unresolved(), callback),
                ));
            }
            Err(error) => {
                return Err(rejection(
                    SimulationNotificationEnqueueRejectionKind::DeferredBeginResidentLookup(error),
                    (unresolved(), callback),
                ));
            }
        };
        let resident = match occurrence.prepare_collection() {
            Ok(resident) => resident,
            Err(failure) => {
                return Err(rejection(
                    SimulationNotificationEnqueueRejectionKind::DeferredBeginResidentPrepare,
                    (
                        SimulationResidentDeferredBeginRecovery::PreparationFailed {
                            failure,
                            userdata_raw,
                        },
                        callback,
                    ),
                ));
            }
        };
        self.resident_resolution_stats
            .deferred_begin
            .record_preparation();
        let session_id = match self.allocate_session_id() {
            Ok(id) => id,
            Err(kind) => {
                return Err(rejection(
                    kind,
                    (
                        SimulationResidentDeferredBeginRecovery::Prepared {
                            resident,
                            userdata_raw,
                        },
                        callback,
                    ),
                ));
            }
        };
        let inner = match self
            .deferred
            .begin(&mut self.adapter, resident.prepared(), userdata_raw)
        {
            Ok(state) => state,
            Err(error) => {
                let kind = match error {
                    DeferredSimulationBeginError::FifoFaulted(fault) => {
                        SimulationNotificationEnqueueRejectionKind::DeferredBeginFifoFaulted(fault)
                    }
                    DeferredSimulationBeginError::SessionStorageAllocation { requested } => {
                        SimulationNotificationEnqueueRejectionKind::DeferredBeginSessionStorageAllocation {
                            requested,
                        }
                    }
                    DeferredSimulationBeginError::SessionIdExhausted => {
                        SimulationNotificationEnqueueRejectionKind::DeferredBeginSessionIdExhausted
                    }
                    DeferredSimulationBeginError::Collection(error) => {
                        SimulationNotificationEnqueueRejectionKind::DeferredBeginCollection(error)
                    }
                };
                return Err(rejection(
                    kind,
                    (
                        SimulationResidentDeferredBeginRecovery::Prepared {
                            resident,
                            userdata_raw,
                        },
                        callback,
                    ),
                ));
            }
        };
        self.sessions.push(Session {
            id: session_id,
            collection_id: collection_id.0,
            has_collection_work: true,
            stage: SessionStage::Deferred,
            registration: None,
            pending_registration_callback: CallbackSlot::Bound(callback),
            fault_registration: None,
            fault_callback: None,
            pending_deregistration_callback: CallbackSlot::Unbound,
            resident: Some(resident),
            resident_prepare_failure: None,
            fault_resident: None,
            next_deferred_arena: 0,
        });
        Ok(self.wrap_deferred_state(session_id, inner))
    }

    /// Begins the callback-only path used for a non-Simulation stream class.
    ///
    /// # Errors
    ///
    /// Returns the collection ID and frozen callback if the router is faulted
    /// or cannot retain the callback-only session.
    pub fn begin_callback_only_registration(
        &mut self,
        collection_id: u64,
        callback: Option<RC>,
    ) -> Result<
        SimulationRegistrationCallbackPermit,
        SimulationNotificationEnqueueRejection<(u64, Option<RC>)>,
    > {
        if let Some(fault) = self.fault {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::RouterFaulted(fault),
                (collection_id, callback),
            ));
        }
        if self.sessions.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                (collection_id, callback),
            ));
        }
        let session_id = match self.allocate_session_id() {
            Ok(id) => id,
            Err(kind) => return Err(rejection(kind, (collection_id, callback))),
        };
        self.sessions.push(Session {
            id: session_id,
            collection_id,
            has_collection_work: false,
            stage: SessionStage::CallbackOnlyReady,
            registration: None,
            pending_registration_callback: CallbackSlot::Bound(callback),
            fault_registration: None,
            fault_callback: None,
            pending_deregistration_callback: CallbackSlot::Unbound,
            resident: None,
            resident_prepare_failure: None,
            fault_resident: None,
            next_deferred_arena: 0,
        });
        Ok(self.registration_callback_permit(session_id, None))
    }

    /// Appends one deferred Arena marker to the unified FIFO.
    ///
    /// # Errors
    ///
    /// Returns the exact producer authority if it is foreign, stale, out of
    /// sequence, or cannot be appended without effects.
    pub fn enqueue_deferred_arena(
        &mut self,
        producer: SimulationNotificationDeferredArenaProducer,
    ) -> Result<
        SimulationNotificationDeferredState,
        SimulationNotificationEnqueueRejection<SimulationNotificationDeferredArenaProducer>,
    > {
        let session_index = match self.validate_authority(
            &producer.domain,
            producer.session_id,
            SessionStage::Deferred,
        ) {
            Ok(index) => index,
            Err(kind) => return Err(rejection(kind, producer)),
        };
        if self.queue.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                producer,
            ));
        }
        let session_id = producer.session_id;
        let arena_index = self.sessions[session_index].next_deferred_arena;
        let arena = if let Some(resident) = self.sessions[session_index].resident.as_ref() {
            let Some((asset_id, _)) = resident.prepared().arena_identity(arena_index) else {
                return Err(rejection(
                    SimulationNotificationEnqueueRejectionKind::DeferredArenaIdentityUnavailable,
                    producer,
                ));
            };
            Some(DeferredArenaIdentity {
                arena_index,
                asset_id: CollectionId(asset_id),
            })
        } else {
            None
        };
        let inner = match self.deferred.enqueue_next_arena(producer.inner) {
            Ok(state) => state,
            Err(error) => {
                let kind =
                    SimulationNotificationEnqueueRejectionKind::DeferredRejected(error.kind());
                return Err(rejection(
                    kind,
                    SimulationNotificationDeferredArenaProducer {
                        domain: producer.domain,
                        session_id,
                        inner: error.into_authority(),
                    },
                ));
            }
        };
        self.queue.push_back(Message::DeferredMarker {
            session_id,
            kind: DeferredSimulationMessageKind::Arena,
            arena,
        });
        self.sessions[session_index].next_deferred_arena = arena_index + 1;
        Ok(self.wrap_deferred_state(session_id, inner))
    }

    /// Appends the deferred activation-zero collection marker.
    ///
    /// # Errors
    ///
    /// Returns the exact producer authority if it is foreign, stale, out of
    /// sequence, or cannot be appended without effects.
    pub fn enqueue_deferred_finish(
        &mut self,
        producer: SimulationNotificationDeferredFinishProducer,
    ) -> Result<
        SimulationRegistrationCallbackPermit,
        SimulationNotificationEnqueueRejection<SimulationNotificationDeferredFinishProducer>,
    > {
        if let Err(kind) = self.validate_authority(
            &producer.domain,
            producer.session_id,
            SessionStage::Deferred,
        ) {
            return Err(rejection(kind, producer));
        }
        if self.queue.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                producer,
            ));
        }
        let session_id = producer.session_id;
        let inner = match self.deferred.enqueue_finish(producer.inner) {
            Ok(permit) => permit,
            Err(error) => {
                let kind =
                    SimulationNotificationEnqueueRejectionKind::DeferredRejected(error.kind());
                return Err(rejection(
                    kind,
                    SimulationNotificationDeferredFinishProducer {
                        domain: producer.domain,
                        session_id,
                        inner: error.into_authority(),
                    },
                ));
            }
        };
        self.queue.push_back(Message::DeferredMarker {
            session_id,
            kind: DeferredSimulationMessageKind::ActivationZeroFinish,
            arena: None,
        });
        Ok(self.registration_callback_permit(session_id, Some(inner)))
    }

    /// Appends the registration callback as a distinct outer FIFO message.
    ///
    /// # Errors
    ///
    /// Returns the exact permit when the append is rejected before effects;
    /// the callback remains frozen in its router session.
    pub fn enqueue_registration_callback(
        &mut self,
        permit: SimulationRegistrationCallbackPermit,
    ) -> Result<(), SimulationNotificationEnqueueRejection<SimulationRegistrationCallbackPermit>>
    {
        if let Err(kind) = self.validate_callback_permit(&permit) {
            return Err(rejection(kind, permit));
        }
        if self.queue.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                permit,
            ));
        }
        let session_id = permit.session_id;
        let Some(session_index) = self.session_index(session_id) else {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::UnknownSession,
                permit,
            ));
        };
        let callback = match std::mem::replace(
            &mut self.sessions[session_index].pending_registration_callback,
            CallbackSlot::Unbound,
        ) {
            CallbackSlot::Bound(callback) => callback,
            CallbackSlot::Unbound => {
                return Err(rejection(
                    SimulationNotificationEnqueueRejectionKind::WrongStage,
                    permit,
                ));
            }
        };
        if let Some(inner) = permit.deferred {
            if let Err(error) = self.deferred.enqueue_callback(inner, callback) {
                let kind =
                    SimulationNotificationEnqueueRejectionKind::DeferredRejected(error.kind());
                let (inner, callback) = error.into_parts();
                self.sessions[session_index].pending_registration_callback =
                    CallbackSlot::Bound(callback);
                return Err(rejection(
                    kind,
                    self.registration_callback_permit(session_id, Some(inner)),
                ));
            }
            self.queue.push_back(Message::DeferredMarker {
                session_id,
                kind: DeferredSimulationMessageKind::Callback,
                arena: None,
            });
        } else {
            self.queue.push_back(Message::RegistrationCallback {
                session_id,
                callback,
            });
        }
        Ok(())
    }

    /// Consumes a delivered occurrence and stages the route chosen from the
    /// freshly resolved deregistration stream class.
    ///
    /// # Errors
    ///
    /// Returns the exact occurrence and frozen callback if its authority or
    /// state is invalid, the route disagrees with its registration class, or
    /// storage reservation fails before effects.
    pub fn enqueue_deregistration(
        &mut self,
        occurrence: SimulationCollectionOccurrence,
        userdata_raw: u32,
        route: SimulationCollectionDeregistrationRoute,
        callback: Option<DC>,
    ) -> Result<
        SimulationDeregistrationCallbackPermit,
        SimulationNotificationEnqueueRejection<(SimulationCollectionOccurrence, Option<DC>)>,
    > {
        let session_index = match self.validate_occurrence(&occurrence) {
            Ok(index) => index,
            Err(kind) => return Err(rejection(kind, (occurrence, callback))),
        };
        let expects_work = matches!(
            route,
            SimulationCollectionDeregistrationRoute::DeregistrationThenCallback
        );
        if occurrence.has_collection_work != expects_work {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StreamClassMismatch,
                (occurrence, callback),
            ));
        }
        if expects_work && self.queue.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                (occurrence, callback),
            ));
        }
        if expects_work {
            let registration = self.sessions[session_index].registration.take();
            let Some(registration) = registration else {
                return Err(rejection(
                    SimulationNotificationEnqueueRejectionKind::WrongStage,
                    (occurrence, callback),
                ));
            };
            self.queue.push_back(Message::Deregistration {
                session_id: occurrence.session_id,
                registration: Some(registration),
                userdata_raw,
            });
            self.sessions[session_index].stage = SessionStage::DeregistrationQueued;
        } else {
            self.sessions[session_index].stage = SessionStage::DeregistrationCallbackReady;
        }
        self.sessions[session_index].pending_deregistration_callback =
            CallbackSlot::Bound(callback);
        Ok(SimulationDeregistrationCallbackPermit {
            domain: occurrence.domain,
            session_id: occurrence.session_id,
        })
    }

    /// Appends the deregistration callback as a distinct outer FIFO message.
    ///
    /// # Errors
    ///
    /// Returns the exact permit when the append is rejected before effects;
    /// the callback remains frozen in its router session.
    pub fn enqueue_deregistration_callback(
        &mut self,
        permit: SimulationDeregistrationCallbackPermit,
    ) -> Result<(), SimulationNotificationEnqueueRejection<SimulationDeregistrationCallbackPermit>>
    {
        let session_index = match self.authority_session_index(&permit.domain, permit.session_id) {
            Ok(index) => index,
            Err(kind) => return Err(rejection(kind, permit)),
        };
        if !matches!(
            self.sessions[session_index].stage,
            SessionStage::DeregistrationQueued | SessionStage::DeregistrationCallbackReady
        ) || !self.sessions[session_index]
            .pending_deregistration_callback
            .is_bound()
        {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::WrongStage,
                permit,
            ));
        }
        if self.queue.try_reserve(1).is_err() {
            return Err(rejection(
                SimulationNotificationEnqueueRejectionKind::StorageAllocation,
                permit,
            ));
        }
        let callback = match std::mem::replace(
            &mut self.sessions[session_index].pending_deregistration_callback,
            CallbackSlot::Unbound,
        ) {
            CallbackSlot::Bound(callback) => callback,
            CallbackSlot::Unbound => {
                return Err(rejection(
                    SimulationNotificationEnqueueRejectionKind::WrongStage,
                    permit,
                ));
            }
        };
        self.queue.push_back(Message::DeregistrationCallback {
            session_id: permit.session_id,
            callback,
        });
        Ok(())
    }

    /// Dispatches at most one front message from the unified FIFO.
    ///
    /// # Errors
    ///
    /// Returns the first sticky router fault if a receiver enters a child
    /// fault or an internal ordering invariant does not hold.
    pub fn dispatch_one(
        &mut self,
    ) -> Result<SimulationNotificationDispatch<RC, DC>, SimulationNotificationFault> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        let Some((session_id, class)) = self.front_descriptor() else {
            return Ok(SimulationNotificationDispatch::Idle);
        };
        let session_index = self.session_index(session_id).ok_or_else(|| {
            self.latch_fault(session_id, SimulationNotificationFaultKind::MissingSession)
        })?;
        match class {
            0 => self.dispatch_immediate(session_index, session_id),
            1 => self.dispatch_deferred(session_index, session_id),
            2 => self.dispatch_registration_callback(session_index, session_id),
            3 => self.dispatch_deregistration(session_index, session_id),
            4 => self.dispatch_deregistration_callback(session_index, session_id),
            _ => unreachable!(),
        }
    }

    fn dispatch_immediate(
        &mut self,
        session_index: usize,
        session_id: u64,
    ) -> Result<SimulationNotificationDispatch<RC, DC>, SimulationNotificationFault> {
        if self.sessions[session_index].stage != SessionStage::ImmediateQueued {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::WrongSessionStage,
            ));
        }
        if let Some(Message::ResidentImmediateRegistration {
            collection_id,
            userdata_raw,
            ..
        }) = self.queue.front()
        {
            return self.dispatch_resident_immediate(
                session_index,
                session_id,
                *collection_id,
                *userdata_raw,
            );
        }
        let (prepared, userdata_raw) = match self.queue.front() {
            Some(Message::ImmediateRegistration {
                prepared,
                userdata_raw,
                ..
            }) => (Arc::clone(prepared), *userdata_raw),
            _ => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::MissingPreparedCollection,
                ));
            }
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.adapter.register_active(&prepared, userdata_raw)
        }));
        match result {
            Ok(registration) => match registration {
                Ok(registration) => {
                    self.sessions[session_index].registration = Some(registration);
                    self.sessions[session_index].stage = SessionStage::Registered;
                    self.queue.pop_front();
                    Ok(SimulationNotificationDispatch::ReceiverReturned)
                }
                Err(error) => Err(self.latch_registration_error(session_id, error)),
            },
            Err(_) => Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DispatchPanicked,
            )),
        }
    }

    fn dispatch_resident_immediate(
        &mut self,
        session_index: usize,
        session_id: u64,
        collection_id: CollectionId,
        userdata_raw: u32,
    ) -> Result<SimulationNotificationDispatch<RC, DC>, SimulationNotificationFault> {
        let lookup = self.resident_lookup.clone().ok_or_else(|| {
            self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::ResidentLookupUnavailable,
            )
        })?;
        self.resident_resolution_stats.immediate.record_lookup();
        let occurrence = match lookup.lookup(collection_id) {
            Ok(Some(occurrence)) => occurrence,
            Ok(None) => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::RegistrationResidentMissing,
                ));
            }
            Err(error) => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::RegistrationResidentLookup(error),
                ));
            }
        };
        let resident = match occurrence.prepare_collection() {
            Ok(resident) => resident,
            Err(failure) => {
                self.sessions[session_index].resident_prepare_failure = Some(failure);
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::RegistrationResidentPrepare,
                ));
            }
        };
        self.resident_resolution_stats
            .immediate
            .record_preparation();
        self.sessions[session_index].resident = Some(resident);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let prepared = self.sessions[session_index]
                .resident
                .as_ref()
                .expect("resident was retained before receiver dispatch")
                .prepared();
            self.adapter.register_active(prepared, userdata_raw)
        }));
        match result {
            Ok(Ok(registration)) => {
                self.sessions[session_index].registration = Some(registration);
                self.sessions[session_index].stage = SessionStage::Registered;
                self.queue.pop_front();
                Ok(SimulationNotificationDispatch::ReceiverReturned)
            }
            Ok(Err(error)) => Err(self.latch_registration_error(session_id, error)),
            Err(_) => Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DispatchPanicked,
            )),
        }
    }

    fn dispatch_deferred(
        &mut self,
        session_index: usize,
        session_id: u64,
    ) -> Result<SimulationNotificationDispatch<RC, DC>, SimulationNotificationFault> {
        if self.sessions[session_index].stage != SessionStage::Deferred {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::WrongSessionStage,
            ));
        }
        let expected = match self.queue.front() {
            Some(Message::DeferredMarker { kind, .. }) => *kind,
            _ => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeferredKindMismatch,
                ));
            }
        };
        if expected == DeferredSimulationMessageKind::Arena
            && self.sessions[session_index].resident.is_some()
        {
            return self.dispatch_resident_deferred_arena(session_index, session_id);
        }
        if expected == DeferredSimulationMessageKind::ActivationZeroFinish
            && self.sessions[session_index].resident.is_some()
        {
            self.resolve_resident_deferred_finish(session_index, session_id)?;
        }
        match self.deferred.dispatch_one(&mut self.adapter) {
            Ok(DeferredSimulationMessageDispatch::ArenaMessageReturned)
                if expected == DeferredSimulationMessageKind::Arena =>
            {
                self.queue.pop_front();
                Ok(SimulationNotificationDispatch::ReceiverReturned)
            }
            Ok(DeferredSimulationMessageDispatch::ActivationZeroMessageReturned)
                if expected == DeferredSimulationMessageKind::ActivationZeroFinish =>
            {
                if self.sessions[session_index].resident.is_some() {
                    self.sessions[session_index].resident =
                        self.sessions[session_index].fault_resident.take();
                }
                self.queue.pop_front();
                Ok(SimulationNotificationDispatch::ReceiverReturned)
            }
            Ok(DeferredSimulationMessageDispatch::CallbackMessageDequeued(completion)) => {
                let (collection_id, registration, callback) = completion.into_parts();
                if expected != DeferredSimulationMessageKind::Callback
                    || collection_id != self.sessions[session_index].collection_id
                {
                    self.sessions[session_index].fault_registration = Some(registration);
                    self.sessions[session_index].fault_callback = callback;
                    return Err(self.latch_fault(
                        session_id,
                        SimulationNotificationFaultKind::DeferredKindMismatch,
                    ));
                }
                self.sessions[session_index].registration = Some(registration);
                self.sessions[session_index].stage = SessionStage::Delivered;
                self.queue.pop_front();
                Ok(
                    SimulationNotificationDispatch::RegistrationCallbackDequeued(
                        SimulationRegistrationNotification {
                            collection_id,
                            occurrence: self.occurrence(session_id, true),
                            callback,
                        },
                    ),
                )
            }
            Ok(_) => Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredKindMismatch,
            )),
            Err(DeferredSimulationDispatchError::ReceiverRejected(kind)) => Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredReceiverRejected(kind),
            )),
            Err(DeferredSimulationDispatchError::Faulted(fault)) => Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredFault(fault),
            )),
        }
    }

    fn dispatch_resident_deferred_arena(
        &mut self,
        session_index: usize,
        session_id: u64,
    ) -> Result<SimulationNotificationDispatch<RC, DC>, SimulationNotificationFault> {
        let identity = match self.queue.front() {
            Some(Message::DeferredMarker {
                kind: DeferredSimulationMessageKind::Arena,
                arena: Some(identity),
                ..
            }) => *identity,
            _ => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeferredArenaMessageIdentityMismatch,
                ));
            }
        };
        let Some(resident) = self.sessions[session_index].resident.as_ref() else {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::MissingPreparedCollection,
            ));
        };
        let Some((expected_id, _)) = resident.prepared().arena_identity(identity.arena_index)
        else {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredArenaMessageIdentityMismatch,
            ));
        };
        if CollectionId(expected_id) != identity.asset_id {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredArenaMessageIdentityMismatch,
            ));
        }
        let expected_plan = resident
            .prepared()
            .arena_plan_owner(identity.arena_index)
            .expect("identity lookup and plan lookup share one prepared arena vector");
        let lookup = self.resident_arena_lookup.clone().ok_or_else(|| {
            self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::ResidentLookupUnavailable,
            )
        })?;
        self.resident_resolution_stats
            .deferred_arena
            .record_lookup();
        let mut plan_mismatch = false;
        let mut dispatch = None;
        let lookup_result =
            lookup.try_with_asset::<_, std::convert::Infallible>(identity.asset_id, |asset| {
                if !std::ptr::eq(expected_plan.as_ref(), asset.plan()) {
                    plan_mismatch = true;
                    return Ok(());
                }
                dispatch = Some(self.deferred.dispatch_one(&mut self.adapter));
                Ok(())
            });
        match lookup_result {
            Ok(None) => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeferredArenaResidentMissing,
                ));
            }
            Err(error) => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeferredArenaResidentLookup(error),
                ));
            }
            Ok(Some(())) => self
                .resident_resolution_stats
                .deferred_arena
                .record_preparation(),
        }
        if plan_mismatch {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredArenaResolvedPlanMismatch,
            ));
        }
        match dispatch.expect("matching Arena lookup dispatches the retained child front") {
            Ok(DeferredSimulationMessageDispatch::ArenaMessageReturned) => {
                self.queue.pop_front();
                Ok(SimulationNotificationDispatch::ReceiverReturned)
            }
            Ok(_) => Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredKindMismatch,
            )),
            Err(DeferredSimulationDispatchError::ReceiverRejected(kind)) => Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredReceiverRejected(kind),
            )),
            Err(DeferredSimulationDispatchError::Faulted(fault)) => Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredFault(fault),
            )),
        }
    }

    fn resolve_resident_deferred_finish(
        &mut self,
        session_index: usize,
        session_id: u64,
    ) -> Result<(), SimulationNotificationFault> {
        let lookup = self.resident_lookup.clone().ok_or_else(|| {
            self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::ResidentLookupUnavailable,
            )
        })?;
        let collection_id = CollectionId(self.sessions[session_index].collection_id);
        self.resident_resolution_stats
            .deferred_finish
            .record_lookup();
        let occurrence = match lookup.lookup(collection_id) {
            Ok(Some(occurrence)) => occurrence,
            Ok(None) => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeferredFinishResidentMissing,
                ));
            }
            Err(error) => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeferredFinishResidentLookup(error),
                ));
            }
        };
        let fresh = match occurrence.prepare_collection() {
            Ok(resident) => resident,
            Err(failure) => {
                self.sessions[session_index].resident_prepare_failure = Some(failure);
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeferredFinishResidentPrepare,
                ));
            }
        };
        self.resident_resolution_stats
            .deferred_finish
            .record_preparation();
        let (same_occurrence, same_snapshot) = self.sessions[session_index]
            .resident
            .as_ref()
            .map_or((false, false), |resident| {
                (
                    resident.occurrence().is_same_occurrence(fresh.occurrence()),
                    resident.has_same_plan_snapshot(&fresh),
                )
            });
        self.sessions[session_index].fault_resident = Some(fresh);
        if !same_occurrence {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredFinishResidentOccurrenceMismatch,
            ));
        }
        if !same_snapshot {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeferredFinishResidentSnapshotMismatch,
            ));
        }
        Ok(())
    }

    fn dispatch_registration_callback(
        &mut self,
        session_index: usize,
        session_id: u64,
    ) -> Result<SimulationNotificationDispatch<RC, DC>, SimulationNotificationFault> {
        let stage = self.sessions[session_index].stage;
        if !matches!(
            stage,
            SessionStage::Registered | SessionStage::CallbackOnlyReady
        ) {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::WrongSessionStage,
            ));
        }
        let Some(Message::RegistrationCallback { callback, .. }) = self.queue.pop_front() else {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::WrongSessionStage,
            ));
        };
        let has_work = self.sessions[session_index].has_collection_work;
        self.sessions[session_index].stage = SessionStage::Delivered;
        Ok(
            SimulationNotificationDispatch::RegistrationCallbackDequeued(
                SimulationRegistrationNotification {
                    collection_id: self.sessions[session_index].collection_id,
                    occurrence: self.occurrence(session_id, has_work),
                    callback,
                },
            ),
        )
    }

    fn dispatch_deregistration(
        &mut self,
        session_index: usize,
        session_id: u64,
    ) -> Result<SimulationNotificationDispatch<RC, DC>, SimulationNotificationFault> {
        if self.sessions[session_index].stage != SessionStage::DeregistrationQueued {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::WrongSessionStage,
            ));
        }
        if self.sessions[session_index].resident.is_some() {
            self.resolve_resident_deregistration(session_index, session_id)?;
        }
        let (registration, userdata_raw) = match self.queue.front_mut() {
            Some(Message::Deregistration {
                registration,
                userdata_raw,
                ..
            }) => (registration.take(), *userdata_raw),
            _ => (None, 0),
        };
        let Some(registration) = registration else {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::MissingRegistration,
            ));
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.adapter.deregister(registration, userdata_raw)
        }));
        match result {
            Ok(Ok(())) => {
                self.sessions[session_index].resident = None;
                self.sessions[session_index].fault_resident = None;
                self.sessions[session_index].stage = SessionStage::DeregistrationCallbackReady;
                self.queue.pop_front();
                Ok(SimulationNotificationDispatch::ReceiverReturned)
            }
            Ok(Err(error)) => {
                let (kind, registration) = match error {
                    SimulationCollectionDeregistrationError::ForeignDomain(registration)
                    | SimulationCollectionDeregistrationError::UnknownRegistration(registration) => {
                        (
                            SimulationNotificationFaultKind::DeregistrationRejectedBeforeEffects,
                            Some(registration),
                        )
                    }
                    SimulationCollectionDeregistrationError::AdapterFaulted {
                        fault,
                        registration,
                    } => (
                        SimulationNotificationFaultKind::CollectionFault(fault),
                        Some(registration),
                    ),
                    SimulationCollectionDeregistrationError::NestedFault(fault) => (
                        SimulationNotificationFaultKind::CollectionFault(fault),
                        None,
                    ),
                };
                if let (
                    Some(registration),
                    Some(Message::Deregistration {
                        registration: slot, ..
                    }),
                ) = (registration, self.queue.front_mut())
                {
                    *slot = Some(registration);
                }
                Err(self.latch_fault(session_id, kind))
            }
            Err(_) => Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DispatchPanicked,
            )),
        }
    }

    fn resolve_resident_deregistration(
        &mut self,
        session_index: usize,
        session_id: u64,
    ) -> Result<(), SimulationNotificationFault> {
        let lookup = self.resident_lookup.clone().ok_or_else(|| {
            self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::ResidentLookupUnavailable,
            )
        })?;
        let collection_id = CollectionId(self.sessions[session_index].collection_id);
        self.resident_resolution_stats
            .deregistration
            .record_lookup();
        let occurrence = match lookup.lookup(collection_id) {
            Ok(Some(occurrence)) => occurrence,
            Ok(None) => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeregistrationResidentMissing,
                ));
            }
            Err(error) => {
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeregistrationResidentLookup(error),
                ));
            }
        };
        let fresh = match occurrence.prepare_collection() {
            Ok(resident) => resident,
            Err(failure) => {
                self.sessions[session_index].resident_prepare_failure = Some(failure);
                return Err(self.latch_fault(
                    session_id,
                    SimulationNotificationFaultKind::DeregistrationResidentPrepare,
                ));
            }
        };
        self.resident_resolution_stats
            .deregistration
            .record_preparation();
        let (same_occurrence, same_snapshot) = self.sessions[session_index]
            .resident
            .as_ref()
            .map_or((false, false), |resident| {
                (
                    resident.occurrence().is_same_occurrence(fresh.occurrence()),
                    resident.has_same_plan_snapshot(&fresh),
                )
            });
        self.sessions[session_index].fault_resident = Some(fresh);
        if !same_occurrence {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeregistrationResidentOccurrenceMismatch,
            ));
        }
        if !same_snapshot {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::DeregistrationResidentSnapshotMismatch,
            ));
        }
        Ok(())
    }

    fn dispatch_deregistration_callback(
        &mut self,
        session_index: usize,
        session_id: u64,
    ) -> Result<SimulationNotificationDispatch<RC, DC>, SimulationNotificationFault> {
        if self.sessions[session_index].stage != SessionStage::DeregistrationCallbackReady {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::WrongSessionStage,
            ));
        }
        let Some(Message::DeregistrationCallback { callback, .. }) = self.queue.pop_front() else {
            return Err(self.latch_fault(
                session_id,
                SimulationNotificationFaultKind::WrongSessionStage,
            ));
        };
        let session = self.sessions.swap_remove(session_index);
        Ok(
            SimulationNotificationDispatch::DeregistrationCallbackDequeued(
                SimulationDeregistrationNotification {
                    collection_id: session.collection_id,
                    callback,
                },
            ),
        )
    }

    fn validate_callback_permit(
        &self,
        permit: &SimulationRegistrationCallbackPermit,
    ) -> Result<usize, SimulationNotificationEnqueueRejectionKind> {
        let index = self.authority_session_index(&permit.domain, permit.session_id)?;
        let session = &self.sessions[index];
        if permit.deferred.is_some() != (session.stage == SessionStage::Deferred)
            || !session.pending_registration_callback.is_bound()
            || (!matches!(
                session.stage,
                SessionStage::ImmediateQueued
                    | SessionStage::Registered
                    | SessionStage::Deferred
                    | SessionStage::CallbackOnlyReady
            ))
        {
            return Err(SimulationNotificationEnqueueRejectionKind::WrongStage);
        }
        Ok(index)
    }

    fn validate_occurrence(
        &self,
        occurrence: &SimulationCollectionOccurrence,
    ) -> Result<usize, SimulationNotificationEnqueueRejectionKind> {
        let index = self.authority_session_index(&occurrence.domain, occurrence.session_id)?;
        let session = &self.sessions[index];
        if session.stage != SessionStage::Delivered
            || session.has_collection_work != occurrence.has_collection_work
        {
            return Err(SimulationNotificationEnqueueRejectionKind::WrongStage);
        }
        Ok(index)
    }

    fn validate_authority(
        &self,
        domain: &Weak<()>,
        session_id: u64,
        stage: SessionStage,
    ) -> Result<usize, SimulationNotificationEnqueueRejectionKind> {
        let index = self.authority_session_index(domain, session_id)?;
        if self.sessions[index].stage != stage {
            return Err(SimulationNotificationEnqueueRejectionKind::WrongStage);
        }
        Ok(index)
    }

    fn authority_session_index(
        &self,
        domain: &Weak<()>,
        session_id: u64,
    ) -> Result<usize, SimulationNotificationEnqueueRejectionKind> {
        if let Some(fault) = self.fault {
            return Err(SimulationNotificationEnqueueRejectionKind::RouterFaulted(
                fault,
            ));
        }
        if !Weak::ptr_eq(domain, &Arc::downgrade(&self.domain)) {
            return Err(SimulationNotificationEnqueueRejectionKind::ForeignRouter);
        }
        self.session_index(session_id)
            .ok_or(SimulationNotificationEnqueueRejectionKind::UnknownSession)
    }

    fn session_index(&self, session_id: u64) -> Option<usize> {
        self.sessions
            .iter()
            .position(|session| session.id == session_id)
    }

    fn allocate_session_id(&mut self) -> Result<u64, SimulationNotificationEnqueueRejectionKind> {
        let id = self.next_session_id;
        self.next_session_id = id
            .checked_add(1)
            .ok_or(SimulationNotificationEnqueueRejectionKind::SessionIdExhausted)?;
        Ok(id)
    }

    fn registration_callback_permit(
        &self,
        session_id: u64,
        deferred: Option<DeferredSimulationCallbackPermit>,
    ) -> SimulationRegistrationCallbackPermit {
        SimulationRegistrationCallbackPermit {
            domain: Arc::downgrade(&self.domain),
            session_id,
            deferred,
        }
    }

    fn wrap_deferred_state(
        &self,
        session_id: u64,
        state: DeferredSimulationEnqueueState,
    ) -> SimulationNotificationDeferredState {
        match state {
            DeferredSimulationEnqueueState::NextArena(inner) => {
                SimulationNotificationDeferredState::NextArena(
                    SimulationNotificationDeferredArenaProducer {
                        domain: Arc::downgrade(&self.domain),
                        session_id,
                        inner,
                    },
                )
            }
            DeferredSimulationEnqueueState::ReadyToFinish(inner) => {
                SimulationNotificationDeferredState::ReadyToFinish(
                    SimulationNotificationDeferredFinishProducer {
                        domain: Arc::downgrade(&self.domain),
                        session_id,
                        inner,
                    },
                )
            }
        }
    }

    fn occurrence(
        &self,
        session_id: u64,
        has_collection_work: bool,
    ) -> SimulationCollectionOccurrence {
        SimulationCollectionOccurrence {
            domain: Arc::downgrade(&self.domain),
            session_id,
            has_collection_work,
        }
    }

    fn front_descriptor(&self) -> Option<(u64, u8)> {
        Some(match self.queue.front()? {
            Message::ImmediateRegistration { session_id, .. }
            | Message::ResidentImmediateRegistration { session_id, .. } => (*session_id, 0),
            Message::DeferredMarker { session_id, .. } => (*session_id, 1),
            Message::RegistrationCallback { session_id, .. } => (*session_id, 2),
            Message::Deregistration { session_id, .. } => (*session_id, 3),
            Message::DeregistrationCallback { session_id, .. } => (*session_id, 4),
        })
    }

    fn latch_registration_error(
        &mut self,
        session_id: u64,
        error: SimulationCollectionRegistrationError,
    ) -> SimulationNotificationFault {
        let kind = match error {
            SimulationCollectionRegistrationError::AdapterFaulted(fault)
            | SimulationCollectionRegistrationError::NestedFault(fault) => {
                SimulationNotificationFaultKind::CollectionFault(fault)
            }
            _ => SimulationNotificationFaultKind::RegistrationRejectedBeforeEffects,
        };
        self.latch_fault(session_id, kind)
    }

    fn latch_fault(
        &mut self,
        session_id: u64,
        kind: SimulationNotificationFaultKind,
    ) -> SimulationNotificationFault {
        let fault = SimulationNotificationFault { session_id, kind };
        *self.fault.get_or_insert(fault)
    }

    #[must_use]
    pub fn pending_message_count(&self) -> usize {
        self.queue.len()
    }

    #[must_use]
    pub fn retained_session_count(&self) -> usize {
        self.sessions.len()
    }

    #[must_use]
    pub const fn fault(&self) -> Option<SimulationNotificationFault> {
        self.fault
    }

    /// Number of retained child-adapter collection registrations, including
    /// deferred and complete entries.
    #[must_use]
    pub fn live_collection_registration_count(&self) -> usize {
        self.adapter.live_registration_count()
    }

    /// Number of retained collection registrations still in deferred setup.
    #[must_use]
    pub fn deferred_collection_registration_count(&self) -> usize {
        self.adapter.deferred_registration_count()
    }

    /// Number of collection registrations whose receiver pass is complete.
    #[must_use]
    pub fn completed_collection_registration_count(&self) -> usize {
        self.adapter.completed_registration_count()
    }

    /// Exact nested collection fault, if a child receiver reached one.
    #[must_use]
    pub const fn collection_fault(&self) -> Option<SimulationCollectionFault> {
        self.adapter.fault()
    }

    /// Returns safe-host resident lookup/preparation diagnostics.
    ///
    /// This observation performs no lookup, preparation, or manager effect.
    #[must_use]
    pub const fn resident_resolution_stats(&self) -> SimulationResidentResolutionStats {
        self.resident_resolution_stats
    }

    /// Exact retained resident preparation failure at the sticky front, if
    /// lookup succeeded but dense preparation did not.
    #[must_use]
    pub fn resident_prepare_failure(&self) -> Option<&SimulationResidentCollectionPrepareFailure> {
        let fault = self.fault?;
        self.sessions
            .iter()
            .find(|session| session.id == fault.session_id)
            .and_then(|session| session.resident_prepare_failure.as_ref())
    }
}

fn rejection<T>(
    kind: SimulationNotificationEnqueueRejectionKind,
    value: T,
) -> SimulationNotificationEnqueueRejection<T> {
    SimulationNotificationEnqueueRejection { kind, value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collection_load_pump::LoadedAssetCollection,
        collection_resident_streamer::{ResidentCollectionRegistry, ResidentCollectionStorage},
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        location_arena_adapter::PreparedLocationArena,
        owned_resident_collection::{
            OwnedResidentAssetsVisitError, OwnedResidentCollection, OwnedResidentCollectionError,
            OwnedResidentCollectionStorage,
        },
        simulation_collection_adapter::{
            SimulationCollectionNestedFailureKind, SimulationCollectionPassDeregistrationFailure,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_dmo_arena_adapter::PreparedSimulationDmoArena,
        simulation_normal_arena_adapter::PreparedSimulationNormalArena,
        simulation_normal_arena_plan::SimulationNormalArenaPlan,
        simulation_resident_collection::{
            SimulationResidentArenaAsset, SimulationResidentCollectionPrepareError,
        },
        simulation_resident_registry::{
            SimulationResidentRegistry, SimulationResidentRegistryError,
            SimulationResidentVisitError,
        },
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::sync::Mutex;

    struct Normal;
    struct FaultNormal;
    struct UserdataNormal(Arc<Mutex<Vec<u32>>>);
    struct Location;
    struct Dmo;
    struct FaultDmo;

    impl SimulationCollectionNormalPass for Normal {
        type Registration = ();

        fn register_normal(
            &mut self,
            _: &PreparedSimulationNormalArena,
            _: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            Ok(())
        }

        fn deregister_normal(
            &mut self,
            (): Self::Registration,
            _: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            Ok(())
        }
    }

    impl SimulationCollectionNormalPass for FaultNormal {
        type Registration = ();

        fn register_normal(
            &mut self,
            _: &PreparedSimulationNormalArena,
            _: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            Err(SimulationCollectionNestedFailureKind::ReachedFault)
        }

        fn deregister_normal(
            &mut self,
            (): Self::Registration,
            _: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            Ok(())
        }
    }

    impl SimulationCollectionNormalPass for UserdataNormal {
        type Registration = ();

        fn register_normal(
            &mut self,
            _: &PreparedSimulationNormalArena,
            userdata_raw: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            self.0.lock().unwrap().push(userdata_raw);
            Ok(())
        }

        fn deregister_normal(
            &mut self,
            (): Self::Registration,
            userdata_raw: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            self.0.lock().unwrap().push(userdata_raw);
            Ok(())
        }
    }

    impl SimulationCollectionLocationPass for Location {
        type Registration = ();

        fn register_location(
            &mut self,
            _: &PreparedLocationArena,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            Ok(())
        }

        fn deregister_location(
            &mut self,
            (): Self::Registration,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            Ok(())
        }
    }

    impl SimulationCollectionDmoPass for Dmo {
        type Registration = ();

        fn register_dmo(
            &mut self,
            _: &PreparedSimulationDmoArena,
            _: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            Ok(())
        }

        fn deregister_dmo(
            &mut self,
            (): Self::Registration,
            _: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            Ok(())
        }
    }

    impl SimulationCollectionDmoPass for FaultDmo {
        type Registration = ();

        fn register_dmo(
            &mut self,
            _: &PreparedSimulationDmoArena,
            _: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            Err(SimulationCollectionNestedFailureKind::ReachedFault)
        }

        fn deregister_dmo(
            &mut self,
            (): Self::Registration,
            _: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            Ok(())
        }
    }

    type Router = SimulationCollectionNotificationRouter<u8, u16, Normal, Location, Dmo>;
    type ResidentStorage = OwnedResidentCollectionStorage<
        SimulationResidentArenaAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;
    type ResidentRegistry = SimulationResidentRegistry<
        SimulationResidentArenaAsset,
        fn(SimulationResidentRegistryError) -> SimulationResidentRegistryError,
        SimulationResidentRegistryError,
    >;

    fn router() -> Router {
        SimulationCollectionNotificationRouter::new(Normal, Location, Dmo)
    }

    fn resident_storage() -> ResidentStorage {
        OwnedResidentCollectionStorage::new(std::convert::identity)
    }

    fn resident_registry() -> (
        ResidentRegistry,
        SimulationResidentLookup<SimulationResidentArenaAsset>,
    ) {
        SimulationResidentRegistry::new(std::convert::identity)
    }

    fn assert_resolution_point(
        actual: SimulationResidentResolutionPointStats,
        lookup_attempts: u64,
        successful_preparations: u64,
    ) {
        assert_eq!(actual.lookup_attempts(), lookup_attempts);
        assert_eq!(actual.successful_preparations(), successful_preparations);
    }

    fn empty_resident_collection(
        storage: &mut ResidentStorage,
    ) -> OwnedResidentCollection<SimulationResidentArenaAsset> {
        storage.create_empty(0).unwrap()
    }

    fn one_resident_collection(
        storage: &mut ResidentStorage,
        asset_id: u64,
    ) -> OwnedResidentCollection<SimulationResidentArenaAsset> {
        let collection = storage.create_empty(1).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(SimulationResidentArenaAsset::from_plan_for_test(
            empty_normal_plan(asset_id),
        ));
        drop(writer);
        collection
    }

    fn empty(collection_id: u64) -> PreparedSimulationCollection {
        PreparedSimulationCollection::try_new(collection_id, Vec::new()).unwrap()
    }

    fn empty_normal_plan(asset_id: u64) -> Arc<SimulationNormalArenaPlan> {
        let descriptor = ResourceDescriptor {
            size: 0,
            alignment: 1,
        };
        let arena = Arena {
            file_header: FileHeader {
                magic: *b"\x89RW4xb2\0\r\n\x1a\n",
                is_big_endian: true,
                pointer_size_bits: 32,
                pointer_alignment: 4,
                unused: 0,
                major_version: *b"454\0",
                minor_version: *b"000\0",
                build_number: 0,
            },
            id: 9,
            entry_count: 0,
            used_entry_count: 0,
            alignment: 1,
            virtual_size: 0,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: [descriptor; 5],
            resources_used: [descriptor; 5],
            resource_offsets: [0; 5],
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
            SimulationCollisionAsset {
                decoded: DecodedArenaAsset::from_parts_for_test(
                    asset_id,
                    WORLD_STREAM_ARENA_ASSET_TYPE,
                    ArenaResources {
                        resources: std::array::from_fn(|_| Vec::new()),
                    },
                    arena,
                ),
                collision: CollisionArena {
                    id: 9,
                    grind_assets: Vec::new(),
                    aggregate_roots: Vec::new(),
                    primitive_sets: Vec::new(),
                    simple_primitive_sets: Vec::new(),
                    meshes: Vec::new(),
                    collision_models: Vec::new(),
                    instances: Vec::new(),
                },
            },
            Vec::new(),
        ));
        Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap())
    }

    fn nonempty(collection_id: u64, arena_count: usize) -> PreparedSimulationCollection {
        let plan = empty_normal_plan(collection_id.wrapping_add(0x1000));
        PreparedSimulationCollection::try_new(
            collection_id,
            (0..arena_count).map(|_| Arc::clone(&plan)).collect(),
        )
        .unwrap()
    }

    fn registration_delivery(
        dispatch: SimulationNotificationDispatch<u8, u16>,
    ) -> SimulationRegistrationNotification<u8> {
        let SimulationNotificationDispatch::RegistrationCallbackDequeued(delivery) = dispatch
        else {
            panic!("registration callback was not dequeued");
        };
        delivery
    }

    #[test]
    fn resident_deferred_begin_returns_exact_unresolved_and_preparation_state() {
        let id = CollectionId(0xaaaa_bbbb_cccc_dddd);
        let mut no_lookup = router();
        let Err(error) = no_lookup.begin_resident_deferred_registration(id, 0x1111_2222, Some(7))
        else {
            panic!("resident deferred setup unexpectedly found a lookup domain");
        };
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::ResidentLookupUnavailable
        );
        let (recovery, callback) = error.into_value();
        assert_eq!(callback, Some(7));
        assert_eq!(recovery.collection_id(), id);
        assert_eq!(recovery.userdata_raw(), 0x1111_2222);
        assert!(matches!(
            recovery,
            SimulationResidentDeferredBeginRecovery::Unresolved { .. }
        ));
        assert_eq!(no_lookup.retained_session_count(), 0);

        let mut storage = resident_storage();
        let collection = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        storage.destroy(collection);
        let mut detached: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let Err(error) = detached.begin_resident_deferred_registration(id, 3, Some(9)) else {
            panic!("detached resident unexpectedly prepared");
        };
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::DeferredBeginResidentPrepare
        );
        let (recovery, callback) = error.into_value();
        assert_eq!(callback, Some(9));
        let SimulationResidentDeferredBeginRecovery::PreparationFailed {
            failure,
            userdata_raw,
        } = recovery
        else {
            panic!("preparation failure did not retain its occurrence");
        };
        assert_eq!(userdata_raw, 3);
        assert_eq!(failure.occurrence().id(), id);
        assert!(matches!(
            failure.error(),
            SimulationResidentCollectionPrepareError::Visit(SimulationResidentVisitError::Assets(
                OwnedResidentAssetsVisitError::Detached
            ))
        ));
        assert_eq!(detached.retained_session_count(), 0);
        drop(wrapper);
    }

    #[test]
    fn resident_deferred_begin_registry_detached_returns_exact_unresolved_state() {
        let id = CollectionId(0x1020_3040_5060_7080);
        let mut storage = resident_storage();
        let collection = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        drop(registry);
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );

        let Err(error) = router.begin_resident_deferred_registration(id, 0x1122_3344, Some(0x55))
        else {
            panic!("detached resident registry unexpectedly began deferred setup");
        };
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::DeferredBeginResidentLookup(
                SimulationResidentLookupError::RegistryDetached
            )
        );
        let (recovery, callback) = error.into_value();
        assert_eq!(callback, Some(0x55));
        assert_eq!(recovery.collection_id(), id);
        assert_eq!(recovery.userdata_raw(), 0x1122_3344);
        assert!(matches!(
            recovery,
            SimulationResidentDeferredBeginRecovery::Unresolved { .. }
        ));
        assert_eq!(router.pending_message_count(), 0);
        assert_eq!(router.retained_session_count(), 0);
        assert_eq!(router.live_collection_registration_count(), 0);
        assert_eq!(router.deferred_collection_registration_count(), 0);
        assert_eq!(router.completed_collection_registration_count(), 0);
        assert!(router.fault().is_none());
        let stats = router.resident_resolution_stats();
        assert_resolution_point(stats.deferred_begin(), 1, 0);

        drop((wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn resident_deferred_missing_setup_returns_full_identity_and_callback() {
        let id = CollectionId(0xfedc_ba98_7654_3210);
        let (_registry, lookup) = resident_registry();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let Err(error) = router.begin_resident_deferred_registration(id, 5, Some(11)) else {
            panic!("missing resident unexpectedly began deferred setup");
        };
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::DeferredBeginResidentMissing
        );
        let (recovery, callback) = error.into_value();
        assert_eq!(callback, Some(11));
        assert_eq!(recovery.collection_id(), id);
        assert_eq!(recovery.userdata_raw(), 5);
        assert!(matches!(
            recovery,
            SimulationResidentDeferredBeginRecovery::Unresolved { .. }
        ));
        assert_eq!(router.retained_session_count(), 0);
        assert_eq!(router.deferred_collection_registration_count(), 0);
    }

    #[test]
    fn resident_deferred_outer_id_exhaustion_returns_prepared_bundle() {
        let id = CollectionId(0x1111_2222_3333_4444);
        let mut storage = resident_storage();
        let collection = one_resident_collection(&mut storage, 0x5555_6666_7777_8888);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        router.next_session_id = u64::MAX;

        let Err(error) = router.begin_resident_deferred_registration(id, 13, Some(17)) else {
            panic!("exhausted outer ID unexpectedly began a session");
        };
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::SessionIdExhausted
        );
        let (recovery, callback) = error.into_value();
        assert_eq!(callback, Some(17));
        let SimulationResidentDeferredBeginRecovery::Prepared {
            resident,
            userdata_raw,
        } = recovery
        else {
            panic!("post-preparation rejection did not return its bundle");
        };
        assert_eq!(userdata_raw, 13);
        assert_eq!(resident.collection_id(), id);
        assert_eq!(resident.prepared().arena_count(), 1);
        assert_eq!(router.retained_session_count(), 0);
        assert_eq!(router.deferred_collection_registration_count(), 0);

        registry.remove_exact(id, &token).unwrap();
        drop((resident, wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn resident_deferred_child_begin_rejection_returns_prepared_bundle() {
        let id = CollectionId(0x2222_3333_4444_5555);
        let mut storage = resident_storage();
        let collection = one_resident_collection(&mut storage, 0x6666_7777_8888_9999);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: SimulationCollectionNotificationRouter<
            u8,
            u16,
            FaultNormal,
            Location,
            Dmo,
        > = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            FaultNormal,
            Location,
            Dmo,
            lookup,
        );
        let adapter_error = router
            .adapter
            .register_active(&nonempty(1, 1), 1)
            .unwrap_err();
        assert!(matches!(
            adapter_error,
            SimulationCollectionRegistrationError::NestedFault(_)
        ));

        let Err(error) = router.begin_resident_deferred_registration(id, 19, Some(23)) else {
            panic!("faulted child adapter unexpectedly began a session");
        };
        assert!(matches!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::DeferredBeginCollection(
                SimulationCollectionRegistrationError::AdapterFaulted(_)
            )
        ));
        let (recovery, callback) = error.into_value();
        assert_eq!(callback, Some(23));
        let SimulationResidentDeferredBeginRecovery::Prepared {
            resident,
            userdata_raw,
        } = recovery
        else {
            panic!("child rejection did not return its prepared bundle");
        };
        assert_eq!(userdata_raw, 19);
        assert_eq!(resident.collection_id(), id);
        assert_eq!(resident.prepared().arena_count(), 1);
        assert_eq!(router.retained_session_count(), 0);
        assert_eq!(router.deferred_collection_registration_count(), 0);

        registry.remove_exact(id, &token).unwrap();
        drop((resident, wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn resident_deferred_success_relooks_up_finish_and_dreg() {
        let id = CollectionId(0x1234_5678_9abc_def0);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut storage = resident_storage();
        let collection = one_resident_collection(&mut storage, 0x1111_2222_3333_4444);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: SimulationCollectionNotificationRouter<
            u8,
            u16,
            UserdataNormal,
            Location,
            Dmo,
        > = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            UserdataNormal(Arc::clone(&seen)),
            Location,
            Dmo,
            lookup,
        );

        let SimulationNotificationDeferredState::NextArena(arena) = router
            .begin_resident_deferred_registration(id, 0x1111_2222, Some(7))
            .unwrap()
        else {
            panic!("one resident arena did not request its Arena message");
        };
        let stats = router.resident_resolution_stats();
        assert_eq!(
            stats.immediate(),
            SimulationResidentResolutionPointStats::default()
        );
        assert_resolution_point(stats.deferred_begin(), 1, 1);
        assert_eq!(
            stats.deferred_finish(),
            SimulationResidentResolutionPointStats::default()
        );
        assert_eq!(
            stats.deregistration(),
            SimulationResidentResolutionPointStats::default()
        );
        let SimulationNotificationDeferredState::ReadyToFinish(finish) =
            router.enqueue_deferred_arena(arena).unwrap()
        else {
            panic!("one resident arena did not reach Finish");
        };
        let callback = router.enqueue_deferred_finish(finish).unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let stats = router.resident_resolution_stats();
        assert_resolution_point(stats.deferred_arena(), 1, 1);
        assert_resolution_point(stats.deferred_finish(), 1, 1);
        assert_eq!(
            stats.deregistration(),
            SimulationResidentResolutionPointStats::default()
        );
        assert!(router.sessions[0].resident.is_some());
        assert!(router.sessions[0].fault_resident.is_none());
        let (collection_id, occurrence, callback) =
            registration_delivery(router.dispatch_one().unwrap()).into_parts();
        assert_eq!((collection_id, callback), (id.0, Some(7)));

        let callback = router
            .enqueue_deregistration(
                occurrence,
                0x3333_4444,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                Some(9),
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let stats = router.resident_resolution_stats();
        assert_resolution_point(stats.deregistration(), 1, 1);
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::DeregistrationCallbackDequeued(_)
        ));
        assert_eq!(*seen.lock().unwrap(), [0x1111_2222, 0x3333_4444]);
        assert_eq!(router.retained_session_count(), 0);
        assert_eq!(router.live_collection_registration_count(), 0);
        let stats = router.resident_resolution_stats();
        assert_eq!(
            stats.immediate(),
            SimulationResidentResolutionPointStats::default()
        );
        assert_resolution_point(stats.deferred_begin(), 1, 1);
        assert_resolution_point(stats.deferred_arena(), 1, 1);
        assert_resolution_point(stats.deferred_finish(), 1, 1);
        assert_resolution_point(stats.deregistration(), 1, 1);

        registry.remove_exact(id, &token).unwrap();
        drop((wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn resident_deferred_arena_lookup_occurs_only_at_its_outer_fifo_front() {
        let id = CollectionId(0x1111);
        let arena_id = 0xaaaa_bbbb_cccc_dddd;
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut storage = resident_storage();
        let collection = one_resident_collection(&mut storage, arena_id);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let expected_plan = {
            let occurrence = lookup.lookup(id).unwrap().unwrap();
            let mut expected_plan = None;
            occurrence
                .try_for_each::<std::convert::Infallible>(|asset| {
                    expected_plan = Some(Arc::downgrade(&asset.plan_owner()));
                    Ok(())
                })
                .unwrap();
            expected_plan.unwrap()
        };
        let mut router: SimulationCollectionNotificationRouter<
            u8,
            u16,
            UserdataNormal,
            Location,
            Dmo,
        > = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            UserdataNormal(Arc::clone(&seen)),
            Location,
            Dmo,
            lookup,
        );

        let callback_only = router
            .begin_callback_only_registration(0x9999, Some(3))
            .unwrap();
        router.enqueue_registration_callback(callback_only).unwrap();
        let SimulationNotificationDeferredState::NextArena(arena) = router
            .begin_resident_deferred_registration(id, 7, Some(5))
            .unwrap()
        else {
            panic!("one resident arena did not produce an Arena message");
        };
        let _finish = router.enqueue_deferred_arena(arena).unwrap();
        assert_resolution_point(router.resident_resolution_stats().deferred_arena(), 0, 0);
        registry.remove_exact(id, &token).unwrap();
        drop(wrapper);
        storage.destroy(collection);
        assert!(expected_plan.upgrade().is_some());

        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::RegistrationCallbackDequeued(_)
        ));
        assert_resolution_point(router.resident_resolution_stats().deferred_arena(), 0, 0);
        let Err(fault) = router.dispatch_one() else {
            panic!("removed Arena registry winner unexpectedly dispatched");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeferredArenaResidentMissing
        );
        assert_resolution_point(router.resident_resolution_stats().deferred_arena(), 1, 0);
        assert!(seen.lock().unwrap().is_empty());
        assert_eq!(router.pending_message_count(), 1);
        assert_eq!(router.deferred.pending_message_count(), 1);
        let resident_session = router
            .sessions
            .iter()
            .find(|session| session.collection_id == id.0)
            .unwrap();
        assert!(resident_session.resident.is_some());
        assert!(matches!(
            resident_session.pending_registration_callback,
            CallbackSlot::Bound(Some(5))
        ));

        let Err(repeated) = router.dispatch_one() else {
            panic!("sticky missing Arena fault unexpectedly cleared");
        };
        assert_eq!(repeated, fault);
        assert_resolution_point(router.resident_resolution_stats().deferred_arena(), 1, 0);
        assert!(seen.lock().unwrap().is_empty());
        assert_eq!(router.pending_message_count(), 1);
        assert_eq!(router.deferred.pending_message_count(), 1);
        drop(router);
        assert!(expected_plan.upgrade().is_none());
    }

    #[test]
    fn global_same_id_winner_mismatch_faults_before_deferred_child_effects() {
        let arena_id = 0xaaaa_bbbb_cccc_dddd;
        let first_id = CollectionId(1);
        let second_id = CollectionId(2);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut storage = resident_storage();
        let first = one_resident_collection(&mut storage, arena_id);
        let second = one_resident_collection(&mut storage, arena_id);
        let (mut registry, lookup) = resident_registry();
        let first_wrapper = registry.create_wrapper(first_id, &first).unwrap();
        let second_wrapper = registry.create_wrapper(second_id, &second).unwrap();
        let first_token = ResidentRegistry::stable_token(&first_wrapper);
        let second_token = ResidentRegistry::stable_token(&second_wrapper);
        registry.publish(first_id, &first_token).unwrap();
        registry.publish(second_id, &second_token).unwrap();
        let expected_plan = {
            let occurrence = lookup.lookup(second_id).unwrap().unwrap();
            let mut expected_plan = None;
            occurrence
                .try_for_each::<std::convert::Infallible>(|asset| {
                    expected_plan = Some(Arc::downgrade(&asset.plan_owner()));
                    Ok(())
                })
                .unwrap();
            expected_plan.unwrap()
        };
        let mut router: SimulationCollectionNotificationRouter<
            u8,
            u16,
            UserdataNormal,
            Location,
            Dmo,
        > = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            UserdataNormal(Arc::clone(&seen)),
            Location,
            Dmo,
            lookup,
        );

        let SimulationNotificationDeferredState::NextArena(arena) = router
            .begin_resident_deferred_registration(second_id, 0x1234, Some(7))
            .unwrap()
        else {
            panic!("one resident arena did not produce an Arena message");
        };
        let _finish = router.enqueue_deferred_arena(arena).unwrap();
        registry.remove_exact(second_id, &second_token).unwrap();
        drop(second_wrapper);
        storage.destroy(second);
        assert!(expected_plan.upgrade().is_some());
        let Err(fault) = router.dispatch_one() else {
            panic!("foreign global first-winner plan unexpectedly retargeted the child ledger");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeferredArenaResolvedPlanMismatch
        );
        assert!(seen.lock().unwrap().is_empty());
        assert_resolution_point(router.resident_resolution_stats().deferred_arena(), 1, 1);
        assert_eq!(router.pending_message_count(), 1);
        assert_eq!(router.deferred.pending_message_count(), 1);
        let resident_session = router
            .sessions
            .iter()
            .find(|session| session.collection_id == second_id.0)
            .unwrap();
        assert!(resident_session.resident.is_some());
        assert!(matches!(
            resident_session.pending_registration_callback,
            CallbackSlot::Bound(Some(7))
        ));

        let Err(repeated) = router.dispatch_one() else {
            panic!("sticky Arena plan mismatch unexpectedly cleared");
        };
        assert_eq!(repeated, fault);
        assert!(seen.lock().unwrap().is_empty());
        assert_resolution_point(router.resident_resolution_stats().deferred_arena(), 1, 1);
        assert_eq!(router.pending_message_count(), 1);
        assert_eq!(router.deferred.pending_message_count(), 1);

        registry.remove_exact(first_id, &first_token).unwrap();
        drop(first_wrapper);
        storage.destroy(first);
        drop(router);
        assert!(expected_plan.upgrade().is_none());
    }

    #[test]
    fn resident_deferred_finish_missing_is_sticky_before_dmo() {
        let id = CollectionId(41);
        let mut storage = resident_storage();
        let collection = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let SimulationNotificationDeferredState::ReadyToFinish(finish) = router
            .begin_resident_deferred_registration(id, 1, Some(2))
            .unwrap()
        else {
            panic!("empty resident collection requested an Arena message");
        };
        let callback = router.enqueue_deferred_finish(finish).unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        registry.remove_exact(id, &token).unwrap();

        let Err(fault) = router.dispatch_one() else {
            panic!("missing terminal resident unexpectedly reached DMO");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeferredFinishResidentMissing
        );
        let Err(repeated) = router.dispatch_one() else {
            panic!("sticky terminal lookup fault unexpectedly cleared");
        };
        assert_eq!(repeated, fault);
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.deferred_collection_registration_count(), 1);
        assert!(router.sessions[0].resident.is_some());
        assert!(router.sessions[0].fault_resident.is_none());

        drop((wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn resident_deferred_finish_registry_detached_retains_exact_front_and_owners() {
        let id = CollectionId(0x8090_a0b0_c0d0_e0f0);
        let mut storage = resident_storage();
        let collection = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let callback_owner = Arc::new(());
        let mut router: SimulationCollectionNotificationRouter<
            Arc<()>,
            u16,
            Normal,
            Location,
            Dmo,
        > = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let SimulationNotificationDeferredState::ReadyToFinish(finish) = router
            .begin_resident_deferred_registration(
                id,
                0x5566_7788,
                Some(Arc::clone(&callback_owner)),
            )
            .unwrap()
        else {
            panic!("empty resident collection requested an Arena message");
        };
        let session_id = finish.session_id;
        let callback = router.enqueue_deferred_finish(finish).unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        drop(registry);

        let Err(fault) = router.dispatch_one() else {
            panic!("detached terminal registry unexpectedly reached DMO");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeferredFinishResidentLookup(
                SimulationResidentLookupError::RegistryDetached
            )
        );
        let Err(repeated) = router.dispatch_one() else {
            panic!("sticky detached-registry fault unexpectedly cleared");
        };
        assert_eq!(repeated, fault);
        assert_eq!(Arc::strong_count(&callback_owner), 2);
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.deferred.pending_message_count(), 2);
        assert!(matches!(
            router.queue.front(),
            Some(Message::DeferredMarker {
                session_id: queued_session_id,
                kind: DeferredSimulationMessageKind::ActivationZeroFinish,
                ..
            }) if *queued_session_id == session_id
        ));
        assert!(matches!(
            router.queue.get(1),
            Some(Message::DeferredMarker {
                session_id: queued_session_id,
                kind: DeferredSimulationMessageKind::Callback,
                ..
            }) if *queued_session_id == session_id
        ));
        assert_eq!(router.retained_session_count(), 1);
        assert_eq!(router.live_collection_registration_count(), 1);
        assert_eq!(router.deferred_collection_registration_count(), 1);
        assert_eq!(router.completed_collection_registration_count(), 0);
        let session = &router.sessions[0];
        assert_eq!(session.id, session_id);
        assert_eq!(session.stage, SessionStage::Deferred);
        assert!(session.registration.is_none());
        assert!(session.fault_registration.is_none());
        assert!(session.fault_callback.is_none());
        assert!(session.resident_prepare_failure.is_none());
        assert!(session.fault_resident.is_none());
        let resident = session.resident.as_ref().unwrap();
        assert_eq!(resident.collection_id(), id);
        assert_eq!(resident.occurrence().id(), id);
        assert_eq!(resident.prepared().arena_count(), 0);
        let stats = router.resident_resolution_stats();
        assert_resolution_point(stats.deferred_begin(), 1, 1);
        assert_resolution_point(stats.deferred_finish(), 1, 0);

        drop((wrapper, router));
        assert_eq!(Arc::strong_count(&callback_owner), 1);
        storage.destroy(collection);
    }

    #[test]
    fn resident_deferred_finish_retains_exact_preparation_failure() {
        let id = CollectionId(42);
        let mut storage = resident_storage();
        let collection = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let SimulationNotificationDeferredState::ReadyToFinish(finish) = router
            .begin_resident_deferred_registration(id, 1, Some(2))
            .unwrap()
        else {
            panic!("empty resident collection requested an Arena message");
        };
        let callback = router.enqueue_deferred_finish(finish).unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        storage.destroy(collection);

        let Err(fault) = router.dispatch_one() else {
            panic!("detached terminal resident unexpectedly reached DMO");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeferredFinishResidentPrepare
        );
        assert!(matches!(
            router.resident_prepare_failure().unwrap().error(),
            SimulationResidentCollectionPrepareError::Visit(SimulationResidentVisitError::Assets(
                OwnedResidentAssetsVisitError::Detached
            ))
        ));
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.deferred_collection_registration_count(), 1);
        assert!(router.sessions[0].resident.is_some());
        assert!(router.sessions[0].fault_resident.is_none());
        let stats = router.resident_resolution_stats();
        assert_resolution_point(stats.deferred_begin(), 1, 1);
        assert_resolution_point(stats.deferred_finish(), 1, 0);

        registry.remove_exact(id, &token).unwrap();
        drop((wrapper, router));
    }

    #[test]
    fn resident_deferred_finish_rejects_same_id_replacement_before_dmo() {
        let id = CollectionId(43);
        let mut storage = resident_storage();
        let first = empty_resident_collection(&mut storage);
        let replacement = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let first_wrapper = registry.create_wrapper(id, &first).unwrap();
        let first_token = ResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let SimulationNotificationDeferredState::ReadyToFinish(finish) = router
            .begin_resident_deferred_registration(id, 1, None)
            .unwrap()
        else {
            panic!("empty resident collection requested an Arena message");
        };
        let callback = router.enqueue_deferred_finish(finish).unwrap();
        router.enqueue_registration_callback(callback).unwrap();

        registry.remove_exact(id, &first_token).unwrap();
        let replacement_wrapper = registry.create_wrapper(id, &replacement).unwrap();
        let replacement_token = ResidentRegistry::stable_token(&replacement_wrapper);
        registry.publish(id, &replacement_token).unwrap();
        let Err(fault) = router.dispatch_one() else {
            panic!("same-ID terminal replacement unexpectedly reached DMO");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeferredFinishResidentOccurrenceMismatch
        );
        assert!(router.sessions[0].resident.is_some());
        assert!(router.sessions[0].fault_resident.is_some());
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.deferred_collection_registration_count(), 1);

        registry.remove_exact(id, &replacement_token).unwrap();
        drop((first_wrapper, replacement_wrapper, router));
        storage.destroy(first);
        storage.destroy(replacement);
    }

    #[test]
    fn resident_deferred_finish_rejects_same_occurrence_append_before_dmo() {
        let id = CollectionId(47);
        let mut storage = resident_storage();
        let collection = storage.create_empty(2).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(SimulationResidentArenaAsset::from_plan_for_test(
            empty_normal_plan(0x1111),
        ));
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let SimulationNotificationDeferredState::NextArena(arena) = router
            .begin_resident_deferred_registration(id, 1, Some(2))
            .unwrap()
        else {
            panic!("one resident arena did not request its Arena message");
        };
        let SimulationNotificationDeferredState::ReadyToFinish(finish) =
            router.enqueue_deferred_arena(arena).unwrap()
        else {
            panic!("one resident arena did not reach Finish");
        };
        let callback = router.enqueue_deferred_finish(finish).unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        writer.add_asset(SimulationResidentArenaAsset::from_plan_for_test(
            empty_normal_plan(0x2222),
        ));

        let Err(fault) = router.dispatch_one() else {
            panic!("mutated terminal snapshot unexpectedly reached DMO");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeferredFinishResidentSnapshotMismatch
        );
        assert!(router.sessions[0].resident.is_some());
        assert!(router.sessions[0].fault_resident.is_some());
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.deferred_collection_registration_count(), 1);

        registry.remove_exact(id, &token).unwrap();
        drop((writer, wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn resident_deferred_dmo_fault_retains_setup_and_fresh_finish_bundles() {
        let id = CollectionId(49);
        let mut storage = resident_storage();
        let collection = one_resident_collection(&mut storage, 0x3333);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: SimulationCollectionNotificationRouter<
            u8,
            u16,
            Normal,
            Location,
            FaultDmo,
        > = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, FaultDmo, lookup,
        );
        let SimulationNotificationDeferredState::NextArena(arena) = router
            .begin_resident_deferred_registration(id, 1, Some(2))
            .unwrap()
        else {
            panic!("one resident arena did not request its Arena message");
        };
        let SimulationNotificationDeferredState::ReadyToFinish(finish) =
            router.enqueue_deferred_arena(arena).unwrap()
        else {
            panic!("one resident arena did not reach Finish");
        };
        let callback = router.enqueue_deferred_finish(finish).unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));

        let Err(fault) = router.dispatch_one() else {
            panic!("faulting terminal DMO unexpectedly completed");
        };
        let SimulationNotificationFaultKind::DeferredFault(deferred_fault) = fault.kind() else {
            panic!("terminal DMO did not retain the deferred child fault");
        };
        assert!(matches!(
            deferred_fault.kind(),
            crate::simulation_deferred_message_fifo::DeferredSimulationDispatchFaultKind::CollectionFault(_)
        ));
        assert!(router.sessions[0].resident.is_some());
        assert!(router.sessions[0].fault_resident.is_some());
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.deferred_collection_registration_count(), 1);

        registry.remove_exact(id, &token).unwrap();
        drop((wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn immediate_registration_and_deregistration_use_separate_callback_nodes() {
        let mut router = router();
        let callback = router
            .enqueue_immediate_registration(empty(1), 0x8000_0000, Some(7))
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert_eq!(router.pending_message_count(), 2);
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let (id, occurrence, callback) =
            registration_delivery(router.dispatch_one().unwrap()).into_parts();
        assert_eq!((id, callback), (1, Some(7)));
        let callback = router
            .enqueue_deregistration(
                occurrence,
                u32::MAX,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                Some(9),
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let SimulationNotificationDispatch::DeregistrationCallbackDequeued(delivery) =
            router.dispatch_one().unwrap()
        else {
            panic!("deregistration callback was not dequeued");
        };
        assert_eq!(delivery.into_parts(), (1, Some(9)));
        assert_eq!(router.retained_session_count(), 0);
        assert_eq!(router.live_collection_registration_count(), 0);
    }

    #[test]
    fn resident_immediate_resolves_only_at_fifo_dispatch_and_dreg_resolves_again() {
        let id = CollectionId(0xfedc_ba98_7654_3210);
        let mut storage = resident_storage();
        let collection = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );

        let callback = router
            .enqueue_resident_immediate_registration(id, 0x1111_2222, Some(7))
            .unwrap();
        assert_eq!(
            router.resident_resolution_stats(),
            SimulationResidentResolutionStats::default()
        );
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        router.enqueue_registration_callback(callback).unwrap();

        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let stats = router.resident_resolution_stats();
        assert_resolution_point(stats.immediate(), 1, 1);
        assert_eq!(
            stats.deregistration(),
            SimulationResidentResolutionPointStats::default()
        );
        let (collection_id, occurrence, callback) =
            registration_delivery(router.dispatch_one().unwrap()).into_parts();
        assert_eq!((collection_id, callback), (id.0, Some(7)));
        let callback = router
            .enqueue_deregistration(
                occurrence,
                0x3333_4444,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                Some(9),
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let stats = router.resident_resolution_stats();
        assert_resolution_point(stats.immediate(), 1, 1);
        assert_resolution_point(stats.deregistration(), 1, 1);
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::DeregistrationCallbackDequeued(_)
        ));
        assert_eq!(router.live_collection_registration_count(), 0);

        registry.remove_exact(id, &token).unwrap();
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn resident_arena_crosses_preparation_with_fresh_reg_and_dreg_userdata() {
        let id = CollectionId(0xaaaa_bbbb_cccc_dddd);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut storage = resident_storage();
        let collection = one_resident_collection(&mut storage, 0x1111_2222_3333_4444);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: SimulationCollectionNotificationRouter<
            u8,
            u16,
            UserdataNormal,
            Location,
            Dmo,
        > = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            UserdataNormal(Arc::clone(&seen)),
            Location,
            Dmo,
            lookup,
        );

        let callback = router
            .enqueue_resident_immediate_registration(id, 0x1111_2222, None)
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let SimulationNotificationDispatch::RegistrationCallbackDequeued(delivery) =
            router.dispatch_one().unwrap()
        else {
            panic!("resident registration callback was not dequeued");
        };
        let (_, occurrence, _) = delivery.into_parts();
        let callback = router
            .enqueue_deregistration(
                occurrence,
                0x3333_4444,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                None,
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        assert_eq!(*seen.lock().unwrap(), [0x1111_2222, 0x3333_4444]);
        assert_eq!(router.live_collection_registration_count(), 0);
        assert!(router.resident_prepare_failure().is_none());

        registry.remove_exact(id, &token).unwrap();
        drop((wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn detached_resident_preparation_failure_is_exact_and_sticky() {
        let id = CollectionId(17);
        let mut storage = resident_storage();
        let collection = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let callback = router
            .enqueue_resident_immediate_registration(id, 1, Some(2))
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        storage.destroy(collection);

        let Err(fault) = router.dispatch_one() else {
            panic!("detached resident unexpectedly prepared");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::RegistrationResidentPrepare
        );
        let failure = router.resident_prepare_failure().unwrap();
        assert!(matches!(
            failure.error(),
            SimulationResidentCollectionPrepareError::Visit(SimulationResidentVisitError::Assets(
                OwnedResidentAssetsVisitError::Detached
            ))
        ));
        let Err(repeated) = router.dispatch_one() else {
            panic!("sticky preparation fault unexpectedly cleared");
        };
        assert_eq!(repeated, fault);
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.live_collection_registration_count(), 0);
        drop(wrapper);
    }

    #[test]
    fn resident_child_fault_retains_bundle_and_front() {
        let id = CollectionId(19);
        let mut storage = resident_storage();
        let collection = one_resident_collection(&mut storage, 23);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: SimulationCollectionNotificationRouter<
            u8,
            u16,
            FaultNormal,
            Location,
            Dmo,
        > = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            FaultNormal,
            Location,
            Dmo,
            lookup,
        );
        let callback = router
            .enqueue_resident_immediate_registration(id, 1, None)
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();

        let Err(fault) = router.dispatch_one() else {
            panic!("faulting resident child unexpectedly registered");
        };
        assert!(matches!(
            fault.kind(),
            SimulationNotificationFaultKind::CollectionFault(_)
        ));
        let session = &router.sessions[0];
        assert!(session.resident.is_some());
        assert!(session.registration.is_none());
        assert_eq!(router.pending_message_count(), 2);
        let Err(repeated) = router.dispatch_one() else {
            panic!("sticky child fault unexpectedly cleared");
        };
        assert_eq!(repeated, fault);

        registry.remove_exact(id, &token).unwrap();
        drop((wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn sticky_fault_precedes_missing_resident_lookup_on_later_enqueue() {
        let mut router: SimulationCollectionNotificationRouter<
            u8,
            u16,
            FaultNormal,
            Location,
            Dmo,
        > = SimulationCollectionNotificationRouter::new(FaultNormal, Location, Dmo);
        let callback = router
            .enqueue_immediate_registration(nonempty(29, 1), 1, None)
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        let Err(fault) = router.dispatch_one() else {
            panic!("faulting compatibility child unexpectedly registered");
        };
        let id = CollectionId(0xaaaa_bbbb_cccc_dddd);
        let error = router
            .enqueue_resident_immediate_registration(id, 2, Some(7))
            .unwrap_err();
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::RouterFaulted(fault)
        );
        assert_eq!(error.into_value(), (id, Some(7)));
        assert_eq!(router.pending_message_count(), 2);
    }

    #[test]
    fn resident_removed_before_registration_front_latches_missing_without_effects() {
        let id = CollectionId(7);
        let mut storage = resident_storage();
        let collection = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let callback = router
            .enqueue_resident_immediate_registration(id, 1, Some(2))
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        registry.remove_exact(id, &token).unwrap();

        let Err(fault) = router.dispatch_one() else {
            panic!("removed resident unexpectedly dispatched");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::RegistrationResidentMissing
        );
        let Err(repeated) = router.dispatch_one() else {
            panic!("sticky resident fault unexpectedly cleared");
        };
        assert_eq!(repeated, fault);
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.live_collection_registration_count(), 0);

        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn resident_reloaded_before_registration_front_resolves_the_replacement() {
        let id = CollectionId(9);
        let mut storage = resident_storage();
        let first = empty_resident_collection(&mut storage);
        let replacement = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let first_wrapper = registry.create_wrapper(id, &first).unwrap();
        let first_token = ResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let callback = router
            .enqueue_resident_immediate_registration(id, 1, None)
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();

        registry.remove_exact(id, &first_token).unwrap();
        let replacement_wrapper = registry.create_wrapper(id, &replacement).unwrap();
        let replacement_token = ResidentRegistry::stable_token(&replacement_wrapper);
        registry.publish(id, &replacement_token).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let (_, occurrence, _) = registration_delivery(router.dispatch_one().unwrap()).into_parts();
        let callback = router
            .enqueue_deregistration(
                occurrence,
                2,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                None,
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));

        registry.remove_exact(id, &replacement_token).unwrap();
        drop((first_wrapper, replacement_wrapper, router));
        storage.destroy(first);
        storage.destroy(replacement);
    }

    #[test]
    fn resident_reload_before_dreg_is_a_sticky_unsupported_identity_boundary() {
        let id = CollectionId(11);
        let mut storage = resident_storage();
        let first = empty_resident_collection(&mut storage);
        let replacement = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let first_wrapper = registry.create_wrapper(id, &first).unwrap();
        let first_token = ResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let callback = router
            .enqueue_resident_immediate_registration(id, 1, None)
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let (_, occurrence, _) = registration_delivery(router.dispatch_one().unwrap()).into_parts();
        let callback = router
            .enqueue_deregistration(
                occurrence,
                2,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                Some(3),
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();

        registry.remove_exact(id, &first_token).unwrap();
        let replacement_wrapper = registry.create_wrapper(id, &replacement).unwrap();
        let replacement_token = ResidentRegistry::stable_token(&replacement_wrapper);
        registry.publish(id, &replacement_token).unwrap();
        let Err(fault) = router.dispatch_one() else {
            panic!("replacement resident unexpectedly deregistered old occurrence");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeregistrationResidentOccurrenceMismatch
        );
        let session = &router.sessions[0];
        assert!(session.resident.is_some());
        assert!(session.fault_resident.is_some());
        assert!(matches!(
            router.queue.front(),
            Some(Message::Deregistration {
                registration: Some(_),
                ..
            })
        ));
        let Err(repeated) = router.dispatch_one() else {
            panic!("sticky occurrence mismatch unexpectedly cleared");
        };
        assert_eq!(repeated, fault);
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.live_collection_registration_count(), 1);

        registry.remove_exact(id, &replacement_token).unwrap();
        drop((first_wrapper, replacement_wrapper, router));
        storage.destroy(first);
        storage.destroy(replacement);
    }

    #[test]
    fn resident_append_before_dreg_faults_on_exact_plan_snapshot() {
        let id = CollectionId(12);
        let mut storage = resident_storage();
        let collection = storage.create_empty(2).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(SimulationResidentArenaAsset::from_plan_for_test(
            empty_normal_plan(0x1111),
        ));
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let callback = router
            .enqueue_resident_immediate_registration(id, 1, None)
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let (_, occurrence, _) = registration_delivery(router.dispatch_one().unwrap()).into_parts();
        let callback = router
            .enqueue_deregistration(
                occurrence,
                2,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                Some(3),
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();

        writer.add_asset(SimulationResidentArenaAsset::from_plan_for_test(
            empty_normal_plan(0x2222),
        ));
        let Err(fault) = router.dispatch_one() else {
            panic!("mutated resident snapshot unexpectedly deregistered");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeregistrationResidentSnapshotMismatch
        );
        let session = &router.sessions[0];
        assert!(session.resident.is_some());
        assert!(session.fault_resident.is_some());
        assert!(matches!(
            router.queue.front(),
            Some(Message::Deregistration {
                registration: Some(_),
                ..
            })
        ));
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.live_collection_registration_count(), 1);

        registry.remove_exact(id, &token).unwrap();
        drop((writer, wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn resident_removed_before_dreg_front_faults_before_token_extraction() {
        let id = CollectionId(13);
        let mut storage = resident_storage();
        let collection = empty_resident_collection(&mut storage);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let mut router: Router = SimulationCollectionNotificationRouter::new_with_resident_lookup(
            Normal, Location, Dmo, lookup,
        );
        let callback = router
            .enqueue_resident_immediate_registration(id, 1, None)
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let (_, occurrence, _) = registration_delivery(router.dispatch_one().unwrap()).into_parts();
        let callback = router
            .enqueue_deregistration(
                occurrence,
                2,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                Some(3),
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();
        registry.remove_exact(id, &token).unwrap();

        let Err(fault) = router.dispatch_one() else {
            panic!("missing resident unexpectedly reached deregistration effects");
        };
        assert_eq!(
            fault.kind(),
            SimulationNotificationFaultKind::DeregistrationResidentMissing
        );
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.live_collection_registration_count(), 1);

        drop((wrapper, router));
        storage.destroy(collection);
    }

    #[test]
    fn callback_only_round_trip_never_creates_collection_registration() {
        let mut router = router();
        let callback = router.begin_callback_only_registration(2, None).unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        let (_, occurrence, callback) =
            registration_delivery(router.dispatch_one().unwrap()).into_parts();
        assert_eq!(callback, None);
        assert!(!occurrence.has_collection_work());
        assert_eq!(router.live_collection_registration_count(), 0);
        let callback = router
            .enqueue_deregistration(
                occurrence,
                11,
                SimulationCollectionDeregistrationRoute::CallbackOnly,
                None,
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::DeregistrationCallbackDequeued(_)
        ));
    }

    #[test]
    fn callbacks_may_be_sent_after_their_primary_receiver_returns() {
        let mut router = router();
        let callback = router
            .enqueue_immediate_registration(empty(3), 1, Some(4))
            .unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        router.enqueue_registration_callback(callback).unwrap();
        let (_, occurrence, _) = registration_delivery(router.dispatch_one().unwrap()).into_parts();
        let callback = router
            .enqueue_deregistration(
                occurrence,
                2,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                Some(5),
            )
            .unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        router.enqueue_deregistration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::DeregistrationCallbackDequeued(_)
        ));
    }

    #[test]
    fn empty_deferred_route_uses_finish_then_callback_markers() {
        let mut router = router();
        let SimulationNotificationDeferredState::ReadyToFinish(finish) = router
            .begin_deferred_registration(empty(4), 3, Some(6))
            .unwrap()
        else {
            panic!("empty deferred route requested an Arena node");
        };
        let callback = router.enqueue_deferred_finish(finish).unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert_eq!(router.pending_message_count(), 2);
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let (_, occurrence, callback) =
            registration_delivery(router.dispatch_one().unwrap()).into_parts();
        assert_eq!(callback, Some(6));
        assert!(occurrence.has_collection_work());
    }

    #[test]
    fn changed_stream_class_is_an_explicit_unsupported_boundary() {
        let mut router = router();
        let callback = router
            .enqueue_immediate_registration(empty(5), 1, Some(1))
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let (_, occurrence, _) = registration_delivery(router.dispatch_one().unwrap()).into_parts();
        let error = router
            .enqueue_deregistration(
                occurrence,
                2,
                SimulationCollectionDeregistrationRoute::CallbackOnly,
                None,
            )
            .unwrap_err();
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::StreamClassMismatch
        );
        let (occurrence, callback) = error.into_value();
        router
            .enqueue_deregistration(
                occurrence,
                2,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                callback,
            )
            .unwrap();
    }

    #[test]
    fn foreign_occurrence_is_returned_without_touching_either_router() {
        let mut first = router();
        let mut second = router();
        let callback = first.begin_callback_only_registration(6, Some(1)).unwrap();
        first.enqueue_registration_callback(callback).unwrap();
        let (_, occurrence, _) = registration_delivery(first.dispatch_one().unwrap()).into_parts();
        let error = second
            .enqueue_deregistration(
                occurrence,
                1,
                SimulationCollectionDeregistrationRoute::CallbackOnly,
                None,
            )
            .unwrap_err();
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::ForeignRouter
        );
        assert_eq!(second.retained_session_count(), 0);
        let (occurrence, callback) = error.into_value();
        first
            .enqueue_deregistration(
                occurrence,
                1,
                SimulationCollectionDeregistrationRoute::CallbackOnly,
                callback,
            )
            .unwrap();
    }

    #[test]
    fn nonempty_deferred_and_immediate_sessions_share_one_outer_fifo() {
        let mut router = router();
        let SimulationNotificationDeferredState::NextArena(arena) = router
            .begin_deferred_registration(nonempty(10, 1), 10, Some(1))
            .unwrap()
        else {
            panic!("nonempty deferred route did not request its Arena node");
        };
        let SimulationNotificationDeferredState::ReadyToFinish(finish) =
            router.enqueue_deferred_arena(arena).unwrap()
        else {
            panic!("one-arena deferred route did not reach finish");
        };
        let immediate_callback = router
            .enqueue_immediate_registration(empty(11), 10, Some(2))
            .unwrap();
        let deferred_callback = router.enqueue_deferred_finish(finish).unwrap();
        router
            .enqueue_registration_callback(deferred_callback)
            .unwrap();
        router
            .enqueue_registration_callback(immediate_callback)
            .unwrap();

        assert_eq!(router.pending_message_count(), 5);
        for _ in 0..3 {
            assert!(matches!(
                router.dispatch_one().unwrap(),
                SimulationNotificationDispatch::ReceiverReturned
            ));
        }
        let (deferred_id, deferred_occurrence, deferred_callback) =
            registration_delivery(router.dispatch_one().unwrap()).into_parts();
        let (immediate_id, immediate_occurrence, immediate_callback) =
            registration_delivery(router.dispatch_one().unwrap()).into_parts();
        assert_eq!((deferred_id, deferred_callback), (10, Some(1)));
        assert_eq!((immediate_id, immediate_callback), (11, Some(2)));

        let deferred_dreg = router
            .enqueue_deregistration(
                deferred_occurrence,
                20,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                Some(3),
            )
            .unwrap();
        router
            .enqueue_deregistration_callback(deferred_dreg)
            .unwrap();
        let immediate_dreg = router
            .enqueue_deregistration(
                immediate_occurrence,
                20,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                Some(4),
            )
            .unwrap();
        router
            .enqueue_deregistration_callback(immediate_dreg)
            .unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let SimulationNotificationDispatch::DeregistrationCallbackDequeued(first) =
            router.dispatch_one().unwrap()
        else {
            panic!("first DReg callback was not dequeued");
        };
        assert_eq!(first.into_parts(), (10, Some(3)));
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let SimulationNotificationDispatch::DeregistrationCallbackDequeued(second) =
            router.dispatch_one().unwrap()
        else {
            panic!("second DReg callback was not dequeued");
        };
        assert_eq!(second.into_parts(), (11, Some(4)));
        assert_eq!(router.pending_message_count(), 0);
        assert_eq!(router.retained_session_count(), 0);
        assert_eq!(router.live_collection_registration_count(), 0);
    }

    #[test]
    fn reached_registration_fault_is_sticky_and_retains_later_callback() {
        type FaultRouter =
            SimulationCollectionNotificationRouter<u8, u16, FaultNormal, Location, Dmo>;
        let mut router = FaultRouter::new(FaultNormal, Location, Dmo);
        let callback = router
            .enqueue_immediate_registration(nonempty(12, 1), 10, Some(7))
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        let Err(fault) = router.dispatch_one() else {
            panic!("faulting registration unexpectedly dispatched");
        };
        let SimulationNotificationFaultKind::CollectionFault(collection_fault) = fault.kind()
        else {
            panic!("nested registration fault lost its exact collection prefix");
        };
        assert_eq!(
            collection_fault.kind(),
            SimulationCollectionNestedFailureKind::ReachedFault
        );
        let Err(repeated) = router.dispatch_one() else {
            panic!("sticky router fault unexpectedly resumed dispatch");
        };
        assert_eq!(repeated, fault);
        assert_eq!(router.pending_message_count(), 2);
        assert_eq!(router.retained_session_count(), 1);
        assert_eq!(router.live_collection_registration_count(), 1);
        let error = router
            .begin_callback_only_registration(13, Some(8))
            .unwrap_err();
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::RouterFaulted(fault)
        );
        assert_eq!(error.into_value(), (13, Some(8)));
    }

    #[test]
    fn exhausted_session_id_returns_frozen_callback_without_effects() {
        let mut router = router();
        router.next_session_id = u64::MAX;
        let error = router
            .begin_callback_only_registration(14, Some(9))
            .unwrap_err();
        assert_eq!(
            error.kind(),
            SimulationNotificationEnqueueRejectionKind::SessionIdExhausted
        );
        assert_eq!(error.into_value(), (14, Some(9)));
        assert_eq!(router.pending_message_count(), 0);
        assert_eq!(router.retained_session_count(), 0);
    }

    #[test]
    fn deregistration_uses_its_fresh_userdata_not_the_registration_snapshot() {
        type UserdataRouter =
            SimulationCollectionNotificationRouter<u8, u16, UserdataNormal, Location, Dmo>;
        let observed = Arc::new(Mutex::new(Vec::new()));
        let mut router = UserdataRouter::new(UserdataNormal(Arc::clone(&observed)), Location, Dmo);
        let callback = router
            .enqueue_immediate_registration(nonempty(15, 1), 0x1111_1111, None)
            .unwrap();
        router.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        let (_, occurrence, _) = registration_delivery(router.dispatch_one().unwrap()).into_parts();
        let callback = router
            .enqueue_deregistration(
                occurrence,
                0x2222_2222,
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback,
                None,
            )
            .unwrap();
        router.enqueue_deregistration_callback(callback).unwrap();
        assert!(matches!(
            router.dispatch_one().unwrap(),
            SimulationNotificationDispatch::ReceiverReturned
        ));
        assert_eq!(*observed.lock().unwrap(), [0x1111_1111, 0x2222_2222]);
    }
}
