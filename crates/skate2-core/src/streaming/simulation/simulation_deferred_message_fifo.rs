//! Evidence-bounded deferred Simulation registration FIFO.
//!
//! Individual arena, terminal activation-zero collection, and callback
//! messages are appended separately. The FIFO owns continuation authority but
//! borrows the collection receiver for each synchronous front dispatch. A
//! callback node is returned to the application after dequeue; this module
//! neither invokes it nor labels it a hull acknowledgement.
//!
//! Native queueing proves FIFO append, full-queue draining under the
//! broadcaster mutex, and synchronous callback invocation before the front is
//! advanced. `dispatch_one`, callback dequeue, owned callback capabilities,
//! allocation errors, unwind containment, and sticky faults are explicit safe
//! host seams rather than claims about native recovery behavior.
//! The native messages copy IDs/userdata and resolve live registry state at
//! dispatch; this host layer instead keys a continuation by private session ID,
//! while the external receiver ledger retains prepared state. Queue capacity,
//! dispatch cadence, and cross-subsystem `EPostman` topology remain outside
//! this component.

use crate::simulation_collection_adapter::{
    DeferredSimulationCollectionRegistration, PreparedSimulationCollection,
    SimulationCollectionDeferredFinishError, SimulationCollectionDeferredStepError,
    SimulationCollectionDmoPass, SimulationCollectionFault, SimulationCollectionLocationPass,
    SimulationCollectionNormalPass, SimulationCollectionRegistration,
    SimulationCollectionRegistrationAdapter, SimulationCollectionRegistrationError,
};
use std::{
    collections::VecDeque,
    error::Error,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Weak},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProducerStage {
    ArenaMessages,
    FinishQueued,
    CallbackQueued,
}

enum ReceiverState {
    Deferred(Option<DeferredSimulationCollectionRegistration>),
    Complete(Option<SimulationCollectionRegistration>),
    Faulted,
}

struct DeferredSession {
    id: u64,
    collection_id: u64,
    arena_count: usize,
    next_enqueue_arena: usize,
    producer_stage: ProducerStage,
    receiver_state: ReceiverState,
}

enum QueuedMessage<C> {
    Arena {
        session_id: u64,
        arena_index: usize,
    },
    Finish {
        session_id: u64,
    },
    Callback {
        session_id: u64,
        callback: Option<C>,
    },
}

/// Exact deferred message class at the front of the modeled FIFO.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredSimulationMessageKind {
    Arena,
    ActivationZeroFinish,
    Callback,
}

/// Producer authority while individual arena messages remain to be appended.
pub struct DeferredSimulationArenaProducer {
    domain: Weak<()>,
    session_id: u64,
}

/// Producer authority for the terminal activation-zero collection message.
pub struct DeferredSimulationFinishProducer {
    domain: Weak<()>,
    session_id: u64,
}

/// Producer authority for the separate callback message.
pub struct DeferredSimulationCallbackPermit {
    domain: Weak<()>,
    session_id: u64,
}

macro_rules! impl_authority_debug {
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

impl_authority_debug!(DeferredSimulationArenaProducer);
impl_authority_debug!(DeferredSimulationFinishProducer);
impl_authority_debug!(DeferredSimulationCallbackPermit);

/// Next producer phase after beginning or appending an arena message.
pub enum DeferredSimulationEnqueueState {
    NextArena(DeferredSimulationArenaProducer),
    ReadyToFinish(DeferredSimulationFinishProducer),
}

impl DeferredSimulationEnqueueState {
    #[must_use]
    pub const fn is_ready_to_finish(&self) -> bool {
        matches!(self, Self::ReadyToFinish(_))
    }
}

impl fmt::Debug for DeferredSimulationEnqueueState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NextArena(authority) => {
                formatter.debug_tuple("NextArena").field(authority).finish()
            }
            Self::ReadyToFinish(authority) => formatter
                .debug_tuple("ReadyToFinish")
                .field(authority)
                .finish(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredSimulationEnqueueRejectionKind {
    FifoFaulted(DeferredSimulationDispatchFault),
    ForeignDomain,
    UnknownSession,
    WrongStage,
    QueueStorageAllocation,
}

/// A no-effect enqueue rejection that returns its producer authority.
pub struct DeferredSimulationEnqueueRejection<P> {
    kind: DeferredSimulationEnqueueRejectionKind,
    authority: P,
}

impl<P> DeferredSimulationEnqueueRejection<P> {
    #[must_use]
    pub const fn kind(&self) -> DeferredSimulationEnqueueRejectionKind {
        self.kind
    }

    #[must_use]
    pub fn into_authority(self) -> P {
        self.authority
    }
}

impl<P: fmt::Debug> fmt::Debug for DeferredSimulationEnqueueRejection<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeferredSimulationEnqueueRejection")
            .field("kind", &self.kind)
            .field("authority", &self.authority)
            .finish()
    }
}

impl<P> fmt::Display for DeferredSimulationEnqueueRejection<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "deferred Simulation enqueue rejected: {:?}",
            self.kind
        )
    }
}

impl<P: fmt::Debug> Error for DeferredSimulationEnqueueRejection<P> {}

/// A no-effect callback enqueue rejection that returns both owned values.
pub struct DeferredSimulationCallbackEnqueueRejection<C> {
    kind: DeferredSimulationEnqueueRejectionKind,
    permit: DeferredSimulationCallbackPermit,
    callback: Option<C>,
}

impl<C> DeferredSimulationCallbackEnqueueRejection<C> {
    #[must_use]
    pub const fn kind(&self) -> DeferredSimulationEnqueueRejectionKind {
        self.kind
    }

    #[must_use]
    pub fn into_parts(self) -> (DeferredSimulationCallbackPermit, Option<C>) {
        (self.permit, self.callback)
    }
}

impl<C: fmt::Debug> fmt::Debug for DeferredSimulationCallbackEnqueueRejection<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeferredSimulationCallbackEnqueueRejection")
            .field("kind", &self.kind)
            .field("permit", &self.permit)
            .field("callback", &self.callback)
            .finish()
    }
}

impl<C> fmt::Display for DeferredSimulationCallbackEnqueueRejection<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "deferred Simulation callback enqueue rejected: {:?}",
            self.kind
        )
    }
}

impl<C: fmt::Debug> Error for DeferredSimulationCallbackEnqueueRejection<C> {}

#[derive(Debug)]
pub enum DeferredSimulationBeginError {
    FifoFaulted(DeferredSimulationDispatchFault),
    SessionStorageAllocation { requested: usize },
    SessionIdExhausted,
    Collection(SimulationCollectionRegistrationError),
}

impl fmt::Display for DeferredSimulationBeginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FifoFaulted(source) => source.fmt(formatter),
            Self::SessionStorageAllocation { requested } => {
                write!(formatter, "could not retain deferred session {requested}")
            }
            Self::SessionIdExhausted => {
                formatter.write_str("deferred Simulation session IDs are exhausted")
            }
            Self::Collection(source) => source.fmt(formatter),
        }
    }
}

impl Error for DeferredSimulationBeginError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FifoFaulted(source) => Some(source),
            Self::Collection(source) => Some(source),
            Self::SessionStorageAllocation { .. } | Self::SessionIdExhausted => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredSimulationReceiverRejectionKind {
    ForeignReceiver,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredSimulationDispatchFaultKind {
    MissingSession,
    WrongSessionState,
    MissingContinuation,
    MissingRegistration,
    ArenaCompletionMismatch,
    CollectionStateMismatch,
    CollectionFault(SimulationCollectionFault),
    DispatchPanicked,
}

/// Sticky fault after a front could not safely finish or a structural queue
/// invariant failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeferredSimulationDispatchFault {
    message: DeferredSimulationMessageKind,
    session_id: u64,
    kind: DeferredSimulationDispatchFaultKind,
}

impl DeferredSimulationDispatchFault {
    #[must_use]
    pub const fn message(self) -> DeferredSimulationMessageKind {
        self.message
    }

    #[must_use]
    pub const fn session_id(self) -> u64 {
        self.session_id
    }

    #[must_use]
    pub const fn kind(self) -> DeferredSimulationDispatchFaultKind {
        self.kind
    }
}

impl fmt::Display for DeferredSimulationDispatchFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "deferred Simulation {:?} handler faulted in session {}: {:?}",
            self.message, self.session_id, self.kind
        )
    }
}

impl Error for DeferredSimulationDispatchFault {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredSimulationDispatchError {
    ReceiverRejected(DeferredSimulationReceiverRejectionKind),
    Faulted(DeferredSimulationDispatchFault),
}

impl fmt::Display for DeferredSimulationDispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReceiverRejected(kind) => {
                write!(
                    formatter,
                    "Simulation collection receiver rejected front: {kind:?}"
                )
            }
            Self::Faulted(source) => source.fmt(formatter),
        }
    }
}

impl Error for DeferredSimulationDispatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Faulted(source) => Some(source),
            Self::ReceiverRejected(_) => None,
        }
    }
}

/// Complete registration authority paired at the callback's FIFO position.
///
/// The registration and generic callback capability are host ownership
/// additions: native callback messages carry only the full collection ID and a
/// nullable, non-owning callback pointer. This type performs no callback,
/// acknowledgement, or registration teardown, but dropping it does normally
/// drop the owned `C` payload.
pub struct SimulationReceiverCompletion<C> {
    collection_id: u64,
    registration: SimulationCollectionRegistration,
    callback: Option<C>,
}

impl<C> SimulationReceiverCompletion<C> {
    #[must_use]
    pub const fn collection_id(&self) -> u64 {
        self.collection_id
    }

    #[must_use]
    pub fn into_parts(self) -> (u64, SimulationCollectionRegistration, Option<C>) {
        (self.collection_id, self.registration, self.callback)
    }
}

impl<C: fmt::Debug> fmt::Debug for SimulationReceiverCompletion<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationReceiverCompletion")
            .field("collection_id", &self.collection_id)
            .field("registration", &self.registration)
            .field("callback", &self.callback)
            .finish()
    }
}

/// Result from one front-only dispatch.
pub enum DeferredSimulationMessageDispatch<C> {
    Idle,
    ArenaMessageReturned,
    ActivationZeroMessageReturned,
    CallbackMessageDequeued(SimulationReceiverCompletion<C>),
}

impl<C: fmt::Debug> fmt::Debug for DeferredSimulationMessageDispatch<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => formatter.write_str("Idle"),
            Self::ArenaMessageReturned => formatter.write_str("ArenaMessageReturned"),
            Self::ActivationZeroMessageReturned => {
                formatter.write_str("ActivationZeroMessageReturned")
            }
            Self::CallbackMessageDequeued(completion) => formatter
                .debug_tuple("CallbackMessageDequeued")
                .field(completion)
                .finish(),
        }
    }
}

/// Deferred-message FIFO. `Drop` performs no hidden dispatch or
/// registration teardown; it does normally drop any owned `C` payloads.
///
/// Tokens are bound to an external receiver by weak domain identity rather than
/// a Rust borrow. As a host lifetime policy, struct owners should declare the
/// FIFO field before the receiver field (struct fields drop in declaration
/// order). Local variables instead drop in reverse declaration order.
pub struct DeferredSimulationMessageFifo<C> {
    queue: VecDeque<QueuedMessage<C>>,
    sessions: Vec<DeferredSession>,
    domain: Arc<()>,
    next_session_id: u64,
    fault: Option<DeferredSimulationDispatchFault>,
}

impl<C> DeferredSimulationMessageFifo<C> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            sessions: Vec::new(),
            domain: Arc::new(()),
            next_session_id: 0,
            fault: None,
        }
    }

    /// Begins host continuation state for an already-admitted deferred route.
    /// No FIFO node is appended by this preparation step.
    ///
    /// # Errors
    ///
    /// Every failure occurs before a new session or nested manager effect.
    pub fn begin<N, L, D>(
        &mut self,
        receiver: &mut SimulationCollectionRegistrationAdapter<N, L, D>,
        prepared: &PreparedSimulationCollection,
        userdata_raw: u32,
    ) -> Result<DeferredSimulationEnqueueState, DeferredSimulationBeginError>
    where
        N: SimulationCollectionNormalPass,
        L: SimulationCollectionLocationPass,
        D: SimulationCollectionDmoPass,
    {
        if let Some(fault) = self.fault {
            return Err(DeferredSimulationBeginError::FifoFaulted(fault));
        }
        self.sessions.try_reserve(1).map_err(|_| {
            DeferredSimulationBeginError::SessionStorageAllocation {
                requested: self.sessions.len().saturating_add(1),
            }
        })?;
        let session_id = self.next_session_id;
        let next_session_id = session_id
            .checked_add(1)
            .ok_or(DeferredSimulationBeginError::SessionIdExhausted)?;
        let continuation = receiver
            .begin_deferred(prepared, userdata_raw)
            .map_err(DeferredSimulationBeginError::Collection)?;
        self.next_session_id = next_session_id;
        self.sessions.push(DeferredSession {
            id: session_id,
            collection_id: prepared.collection_id(),
            arena_count: prepared.arena_count(),
            next_enqueue_arena: 0,
            producer_stage: ProducerStage::ArenaMessages,
            receiver_state: ReceiverState::Deferred(Some(continuation)),
        });
        if prepared.arena_count() == 0 {
            Ok(DeferredSimulationEnqueueState::ReadyToFinish(
                DeferredSimulationFinishProducer {
                    domain: Arc::downgrade(&self.domain),
                    session_id,
                },
            ))
        } else {
            Ok(DeferredSimulationEnqueueState::NextArena(
                DeferredSimulationArenaProducer {
                    domain: Arc::downgrade(&self.domain),
                    session_id,
                },
            ))
        }
    }

    /// Appends one individual-arena message.
    ///
    /// # Errors
    ///
    /// Rejections append nothing and return the unchanged authority.
    pub fn enqueue_next_arena(
        &mut self,
        authority: DeferredSimulationArenaProducer,
    ) -> Result<
        DeferredSimulationEnqueueState,
        DeferredSimulationEnqueueRejection<DeferredSimulationArenaProducer>,
    > {
        if let Some(fault) = self.fault {
            return Err(DeferredSimulationEnqueueRejection {
                kind: DeferredSimulationEnqueueRejectionKind::FifoFaulted(fault),
                authority,
            });
        }
        let session_index = match self.session_index(&authority.domain, authority.session_id) {
            Ok(index) => index,
            Err(kind) => {
                return Err(DeferredSimulationEnqueueRejection { kind, authority });
            }
        };
        let session = &self.sessions[session_index];
        if session.producer_stage != ProducerStage::ArenaMessages
            || session.next_enqueue_arena >= session.arena_count
        {
            return Err(DeferredSimulationEnqueueRejection {
                kind: DeferredSimulationEnqueueRejectionKind::WrongStage,
                authority,
            });
        }
        if self.queue.try_reserve(1).is_err() {
            return Err(DeferredSimulationEnqueueRejection {
                kind: DeferredSimulationEnqueueRejectionKind::QueueStorageAllocation,
                authority,
            });
        }
        let arena_index = self.sessions[session_index].next_enqueue_arena;
        self.queue.push_back(QueuedMessage::Arena {
            session_id: authority.session_id,
            arena_index,
        });
        self.sessions[session_index].next_enqueue_arena = arena_index + 1;
        if arena_index + 1 == self.sessions[session_index].arena_count {
            Ok(DeferredSimulationEnqueueState::ReadyToFinish(
                DeferredSimulationFinishProducer {
                    domain: authority.domain,
                    session_id: authority.session_id,
                },
            ))
        } else {
            Ok(DeferredSimulationEnqueueState::NextArena(authority))
        }
    }

    /// Appends the terminal activation-zero collection message.
    ///
    /// # Errors
    ///
    /// Rejections append nothing and return the unchanged authority.
    pub fn enqueue_finish(
        &mut self,
        authority: DeferredSimulationFinishProducer,
    ) -> Result<
        DeferredSimulationCallbackPermit,
        DeferredSimulationEnqueueRejection<DeferredSimulationFinishProducer>,
    > {
        if let Some(fault) = self.fault {
            return Err(DeferredSimulationEnqueueRejection {
                kind: DeferredSimulationEnqueueRejectionKind::FifoFaulted(fault),
                authority,
            });
        }
        let session_index = match self.session_index(&authority.domain, authority.session_id) {
            Ok(index) => index,
            Err(kind) => {
                return Err(DeferredSimulationEnqueueRejection { kind, authority });
            }
        };
        let session = &self.sessions[session_index];
        if session.producer_stage != ProducerStage::ArenaMessages
            || session.next_enqueue_arena != session.arena_count
        {
            return Err(DeferredSimulationEnqueueRejection {
                kind: DeferredSimulationEnqueueRejectionKind::WrongStage,
                authority,
            });
        }
        if self.queue.try_reserve(1).is_err() {
            return Err(DeferredSimulationEnqueueRejection {
                kind: DeferredSimulationEnqueueRejectionKind::QueueStorageAllocation,
                authority,
            });
        }
        self.queue.push_back(QueuedMessage::Finish {
            session_id: authority.session_id,
        });
        self.sessions[session_index].producer_stage = ProducerStage::FinishQueued;
        Ok(DeferredSimulationCallbackPermit {
            domain: authority.domain,
            session_id: authority.session_id,
        })
    }

    /// Appends the separate callback message. `None` is a null native callback
    /// pointer and still occupies its FIFO position.
    ///
    /// # Errors
    ///
    /// Rejections append nothing and return both the permit and callback.
    pub fn enqueue_callback(
        &mut self,
        permit: DeferredSimulationCallbackPermit,
        callback: Option<C>,
    ) -> Result<(), DeferredSimulationCallbackEnqueueRejection<C>> {
        if let Some(fault) = self.fault {
            return Err(DeferredSimulationCallbackEnqueueRejection {
                kind: DeferredSimulationEnqueueRejectionKind::FifoFaulted(fault),
                permit,
                callback,
            });
        }
        let session_index = match self.session_index(&permit.domain, permit.session_id) {
            Ok(index) => index,
            Err(kind) => {
                return Err(DeferredSimulationCallbackEnqueueRejection {
                    kind,
                    permit,
                    callback,
                });
            }
        };
        if self.sessions[session_index].producer_stage != ProducerStage::FinishQueued {
            return Err(DeferredSimulationCallbackEnqueueRejection {
                kind: DeferredSimulationEnqueueRejectionKind::WrongStage,
                permit,
                callback,
            });
        }
        if self.queue.try_reserve(1).is_err() {
            return Err(DeferredSimulationCallbackEnqueueRejection {
                kind: DeferredSimulationEnqueueRejectionKind::QueueStorageAllocation,
                permit,
                callback,
            });
        }
        self.queue.push_back(QueuedMessage::Callback {
            session_id: permit.session_id,
            callback,
        });
        self.sessions[session_index].producer_stage = ProducerStage::CallbackQueued;
        Ok(())
    }

    /// Dispatches at most one front node and never invokes callback code.
    /// Native-parity hosts must drain repeatedly until idle while holding their
    /// own exclusive broadcaster boundary; one node per frame is not proven.
    ///
    /// A wrong receiver leaves both front node and continuation available for
    /// retry. A reached nested fault or unwind latches permanently, retains the
    /// failed and later nodes, and exposes no completion.
    ///
    /// # Errors
    ///
    /// Returns a recoverable wrong-receiver rejection or the sticky first
    /// reached dispatch fault.
    pub fn dispatch_one<N, L, D>(
        &mut self,
        receiver: &mut SimulationCollectionRegistrationAdapter<N, L, D>,
    ) -> Result<DeferredSimulationMessageDispatch<C>, DeferredSimulationDispatchError>
    where
        N: SimulationCollectionNormalPass,
        L: SimulationCollectionLocationPass,
        D: SimulationCollectionDmoPass,
    {
        if let Some(fault) = self.fault {
            return Err(DeferredSimulationDispatchError::Faulted(fault));
        }
        let Some((kind, session_id, arena_index)) = self.front_descriptor() else {
            return Ok(DeferredSimulationMessageDispatch::Idle);
        };
        match kind {
            DeferredSimulationMessageKind::Arena => {
                self.dispatch_arena(receiver, session_id, arena_index)
            }
            DeferredSimulationMessageKind::ActivationZeroFinish => {
                self.dispatch_finish(receiver, session_id)
            }
            DeferredSimulationMessageKind::Callback => self.dispatch_callback(session_id),
        }
    }

    fn dispatch_arena<N, L, D>(
        &mut self,
        receiver: &mut SimulationCollectionRegistrationAdapter<N, L, D>,
        session_id: u64,
        arena_index: usize,
    ) -> Result<DeferredSimulationMessageDispatch<C>, DeferredSimulationDispatchError>
    where
        N: SimulationCollectionNormalPass,
        L: SimulationCollectionLocationPass,
        D: SimulationCollectionDmoPass,
    {
        let session_index =
            self.dispatch_session_index(DeferredSimulationMessageKind::Arena, session_id)?;
        let continuation = match &mut self.sessions[session_index].receiver_state {
            ReceiverState::Deferred(slot) => slot.take(),
            ReceiverState::Complete(_) | ReceiverState::Faulted => {
                return Err(self.latch_fault(
                    DeferredSimulationMessageKind::Arena,
                    session_id,
                    DeferredSimulationDispatchFaultKind::WrongSessionState,
                ));
            }
        };
        let Some(continuation) = continuation else {
            return Err(self.latch_fault(
                DeferredSimulationMessageKind::Arena,
                session_id,
                DeferredSimulationDispatchFaultKind::MissingContinuation,
            ));
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            receiver.register_next_deferred_arena(continuation)
        }));
        match result {
            Ok(Ok(step)) => {
                let expected_complete = arena_index + 1 == self.sessions[session_index].arena_count;
                if step.is_prefix_complete() != expected_complete {
                    self.sessions[session_index].receiver_state =
                        ReceiverState::Deferred(Some(step.into_registration()));
                    return Err(self.latch_fault(
                        DeferredSimulationMessageKind::Arena,
                        session_id,
                        DeferredSimulationDispatchFaultKind::ArenaCompletionMismatch,
                    ));
                }
                self.sessions[session_index].receiver_state =
                    ReceiverState::Deferred(Some(step.into_registration()));
                self.queue.pop_front();
                Ok(DeferredSimulationMessageDispatch::ArenaMessageReturned)
            }
            Ok(Err(error)) => self.handle_step_error(session_index, session_id, error),
            Err(_) => Err(self.latch_fault(
                DeferredSimulationMessageKind::Arena,
                session_id,
                DeferredSimulationDispatchFaultKind::DispatchPanicked,
            )),
        }
    }

    fn dispatch_finish<N, L, D>(
        &mut self,
        receiver: &mut SimulationCollectionRegistrationAdapter<N, L, D>,
        session_id: u64,
    ) -> Result<DeferredSimulationMessageDispatch<C>, DeferredSimulationDispatchError>
    where
        N: SimulationCollectionNormalPass,
        L: SimulationCollectionLocationPass,
        D: SimulationCollectionDmoPass,
    {
        let session_index = self.dispatch_session_index(
            DeferredSimulationMessageKind::ActivationZeroFinish,
            session_id,
        )?;
        let continuation = match &mut self.sessions[session_index].receiver_state {
            ReceiverState::Deferred(slot) => slot.take(),
            ReceiverState::Complete(_) | ReceiverState::Faulted => {
                return Err(self.latch_fault(
                    DeferredSimulationMessageKind::ActivationZeroFinish,
                    session_id,
                    DeferredSimulationDispatchFaultKind::WrongSessionState,
                ));
            }
        };
        let Some(continuation) = continuation else {
            return Err(self.latch_fault(
                DeferredSimulationMessageKind::ActivationZeroFinish,
                session_id,
                DeferredSimulationDispatchFaultKind::MissingContinuation,
            ));
        };
        let result = catch_unwind(AssertUnwindSafe(|| receiver.finish_deferred(continuation)));
        match result {
            Ok(Ok(registration)) => {
                self.sessions[session_index].receiver_state =
                    ReceiverState::Complete(Some(registration));
                self.queue.pop_front();
                Ok(DeferredSimulationMessageDispatch::ActivationZeroMessageReturned)
            }
            Ok(Err(error)) => self.handle_finish_error(session_index, session_id, error),
            Err(_) => Err(self.latch_fault(
                DeferredSimulationMessageKind::ActivationZeroFinish,
                session_id,
                DeferredSimulationDispatchFaultKind::DispatchPanicked,
            )),
        }
    }

    fn dispatch_callback(
        &mut self,
        session_id: u64,
    ) -> Result<DeferredSimulationMessageDispatch<C>, DeferredSimulationDispatchError> {
        let session_index =
            self.dispatch_session_index(DeferredSimulationMessageKind::Callback, session_id)?;
        let registration = match &mut self.sessions[session_index].receiver_state {
            ReceiverState::Complete(slot) => slot.take(),
            ReceiverState::Deferred(_) | ReceiverState::Faulted => {
                return Err(self.latch_fault(
                    DeferredSimulationMessageKind::Callback,
                    session_id,
                    DeferredSimulationDispatchFaultKind::WrongSessionState,
                ));
            }
        };
        let Some(registration) = registration else {
            return Err(self.latch_fault(
                DeferredSimulationMessageKind::Callback,
                session_id,
                DeferredSimulationDispatchFaultKind::MissingRegistration,
            ));
        };
        let Some(QueuedMessage::Callback { callback, .. }) = self.queue.pop_front() else {
            self.sessions[session_index].receiver_state =
                ReceiverState::Complete(Some(registration));
            return Err(self.latch_fault(
                DeferredSimulationMessageKind::Callback,
                session_id,
                DeferredSimulationDispatchFaultKind::WrongSessionState,
            ));
        };
        let session = self.sessions.swap_remove(session_index);
        Ok(DeferredSimulationMessageDispatch::CallbackMessageDequeued(
            SimulationReceiverCompletion {
                collection_id: session.collection_id,
                registration,
                callback,
            },
        ))
    }

    fn handle_step_error(
        &mut self,
        session_index: usize,
        session_id: u64,
        error: SimulationCollectionDeferredStepError,
    ) -> Result<DeferredSimulationMessageDispatch<C>, DeferredSimulationDispatchError> {
        match error {
            SimulationCollectionDeferredStepError::ForeignDomain(continuation) => {
                self.restore_rejected_continuation(session_index, continuation);
                Err(DeferredSimulationDispatchError::ReceiverRejected(
                    DeferredSimulationReceiverRejectionKind::ForeignReceiver,
                ))
            }
            SimulationCollectionDeferredStepError::UnknownRegistration(continuation)
            | SimulationCollectionDeferredStepError::PrefixComplete(continuation)
            | SimulationCollectionDeferredStepError::AlreadyComplete(continuation) => {
                self.restore_rejected_continuation(session_index, continuation);
                Err(self.latch_fault(
                    DeferredSimulationMessageKind::Arena,
                    session_id,
                    DeferredSimulationDispatchFaultKind::CollectionStateMismatch,
                ))
            }
            SimulationCollectionDeferredStepError::AdapterFaulted {
                fault,
                registration,
            } => {
                self.restore_rejected_continuation(session_index, registration);
                Err(self.latch_fault(
                    DeferredSimulationMessageKind::Arena,
                    session_id,
                    DeferredSimulationDispatchFaultKind::CollectionFault(fault),
                ))
            }
            SimulationCollectionDeferredStepError::NestedFault(fault) => Err(self.latch_fault(
                DeferredSimulationMessageKind::Arena,
                session_id,
                DeferredSimulationDispatchFaultKind::CollectionFault(fault),
            )),
        }
    }

    fn handle_finish_error(
        &mut self,
        session_index: usize,
        session_id: u64,
        error: SimulationCollectionDeferredFinishError,
    ) -> Result<DeferredSimulationMessageDispatch<C>, DeferredSimulationDispatchError> {
        match error {
            SimulationCollectionDeferredFinishError::ForeignDomain(continuation) => {
                self.restore_rejected_continuation(session_index, continuation);
                Err(DeferredSimulationDispatchError::ReceiverRejected(
                    DeferredSimulationReceiverRejectionKind::ForeignReceiver,
                ))
            }
            SimulationCollectionDeferredFinishError::UnknownRegistration(continuation) => {
                self.restore_rejected_continuation(session_index, continuation);
                Err(self.latch_fault(
                    DeferredSimulationMessageKind::ActivationZeroFinish,
                    session_id,
                    DeferredSimulationDispatchFaultKind::CollectionStateMismatch,
                ))
            }
            SimulationCollectionDeferredFinishError::AdapterFaulted {
                fault,
                registration,
            } => {
                self.restore_rejected_continuation(session_index, registration);
                Err(self.latch_fault(
                    DeferredSimulationMessageKind::ActivationZeroFinish,
                    session_id,
                    DeferredSimulationDispatchFaultKind::CollectionFault(fault),
                ))
            }
            SimulationCollectionDeferredFinishError::Incomplete { registration, .. }
            | SimulationCollectionDeferredFinishError::AlreadyComplete(registration) => {
                self.restore_rejected_continuation(session_index, registration);
                Err(self.latch_fault(
                    DeferredSimulationMessageKind::ActivationZeroFinish,
                    session_id,
                    DeferredSimulationDispatchFaultKind::CollectionStateMismatch,
                ))
            }
            SimulationCollectionDeferredFinishError::NestedFault(fault) => Err(self.latch_fault(
                DeferredSimulationMessageKind::ActivationZeroFinish,
                session_id,
                DeferredSimulationDispatchFaultKind::CollectionFault(fault),
            )),
        }
    }

    fn restore_rejected_continuation(
        &mut self,
        session_index: usize,
        continuation: DeferredSimulationCollectionRegistration,
    ) {
        self.sessions[session_index].receiver_state = ReceiverState::Deferred(Some(continuation));
    }

    fn dispatch_session_index(
        &mut self,
        message: DeferredSimulationMessageKind,
        session_id: u64,
    ) -> Result<usize, DeferredSimulationDispatchError> {
        self.sessions
            .iter()
            .position(|session| session.id == session_id)
            .ok_or_else(|| {
                self.latch_fault(
                    message,
                    session_id,
                    DeferredSimulationDispatchFaultKind::MissingSession,
                )
            })
    }

    fn session_index(
        &self,
        domain: &Weak<()>,
        session_id: u64,
    ) -> Result<usize, DeferredSimulationEnqueueRejectionKind> {
        if !Weak::ptr_eq(domain, &Arc::downgrade(&self.domain)) {
            return Err(DeferredSimulationEnqueueRejectionKind::ForeignDomain);
        }
        self.sessions
            .iter()
            .position(|session| session.id == session_id)
            .ok_or(DeferredSimulationEnqueueRejectionKind::UnknownSession)
    }

    fn front_descriptor(&self) -> Option<(DeferredSimulationMessageKind, u64, usize)> {
        Some(match self.queue.front()? {
            QueuedMessage::Arena {
                session_id,
                arena_index,
            } => (
                DeferredSimulationMessageKind::Arena,
                *session_id,
                *arena_index,
            ),
            QueuedMessage::Finish { session_id } => (
                DeferredSimulationMessageKind::ActivationZeroFinish,
                *session_id,
                0,
            ),
            QueuedMessage::Callback { session_id, .. } => {
                (DeferredSimulationMessageKind::Callback, *session_id, 0)
            }
        })
    }

    fn latch_fault(
        &mut self,
        message: DeferredSimulationMessageKind,
        session_id: u64,
        kind: DeferredSimulationDispatchFaultKind,
    ) -> DeferredSimulationDispatchError {
        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        {
            session.receiver_state = ReceiverState::Faulted;
        }
        let fault = DeferredSimulationDispatchFault {
            message,
            session_id,
            kind,
        };
        DeferredSimulationDispatchError::Faulted(*self.fault.get_or_insert(fault))
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
    pub const fn fault(&self) -> Option<DeferredSimulationDispatchFault> {
        self.fault
    }
}

impl<C> Default for DeferredSimulationMessageFifo<C> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        location_arena_adapter::PreparedLocationArena,
        simulation_collection_adapter::{
            SimulationCollectionNestedFailureKind, SimulationCollectionPassDeregistrationFailure,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_dmo_arena_adapter::PreparedSimulationDmoArena,
        simulation_normal_arena_adapter::PreparedSimulationNormalArena,
        simulation_normal_arena_plan::SimulationNormalArenaPlan,
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        Normal(u64, u32),
        Location(u64),
        Dmo(u64, u32),
    }

    #[derive(Default)]
    struct PassState {
        events: Arc<Mutex<Vec<Event>>>,
        normal_calls: usize,
        location_calls: usize,
        dmo_calls: usize,
        panic_normal_at: Option<usize>,
        panic_location_at: Option<usize>,
        panic_dmo_at: Option<usize>,
    }

    struct Normal(PassState);
    struct Location(PassState);
    struct Dmo(PassState);

    fn normal_identity(prepared: &PreparedSimulationNormalArena) -> u64 {
        prepared.plan().static_plan().asset().decoded.asset_id
    }

    fn location_identity(prepared: &PreparedLocationArena) -> u64 {
        prepared
            .plan()
            .normal_plan()
            .static_plan()
            .asset()
            .decoded
            .asset_id
    }

    fn dmo_identity(prepared: &PreparedSimulationDmoArena) -> u64 {
        prepared
            .plan()
            .normal_plan()
            .static_plan()
            .asset()
            .decoded
            .asset_id
    }

    impl SimulationCollectionNormalPass for Normal {
        type Registration = u64;

        fn register_normal(
            &mut self,
            prepared: &PreparedSimulationNormalArena,
            userdata_raw: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            let call = self.0.normal_calls;
            self.0.normal_calls += 1;
            let id = normal_identity(prepared);
            self.0
                .events
                .lock()
                .unwrap()
                .push(Event::Normal(id, userdata_raw));
            assert_ne!(self.0.panic_normal_at, Some(call), "normal panic");
            Ok(id)
        }

        fn deregister_normal(
            &mut self,
            _: Self::Registration,
            _: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            Ok(())
        }
    }

    impl SimulationCollectionLocationPass for Location {
        type Registration = u64;

        fn register_location(
            &mut self,
            prepared: &PreparedLocationArena,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            let call = self.0.location_calls;
            self.0.location_calls += 1;
            let id = location_identity(prepared);
            self.0.events.lock().unwrap().push(Event::Location(id));
            assert_ne!(self.0.panic_location_at, Some(call), "Location panic");
            Ok(id)
        }

        fn deregister_location(
            &mut self,
            _: Self::Registration,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            Ok(())
        }
    }

    impl SimulationCollectionDmoPass for Dmo {
        type Registration = u64;

        fn register_dmo(
            &mut self,
            prepared: &PreparedSimulationDmoArena,
            userdata_raw: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            let call = self.0.dmo_calls;
            self.0.dmo_calls += 1;
            let id = dmo_identity(prepared);
            self.0
                .events
                .lock()
                .unwrap()
                .push(Event::Dmo(id, userdata_raw));
            assert_ne!(self.0.panic_dmo_at, Some(call), "DMO panic");
            Ok(id)
        }

        fn deregister_dmo(
            &mut self,
            _: Self::Registration,
            _: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            Ok(())
        }
    }

    type Receiver = SimulationCollectionRegistrationAdapter<Normal, Location, Dmo>;

    fn receiver(events: &Arc<Mutex<Vec<Event>>>) -> Receiver {
        receiver_with_panics(events, None, None, None)
    }

    fn receiver_with_dmo_panic(
        events: &Arc<Mutex<Vec<Event>>>,
        panic_dmo_at: Option<usize>,
    ) -> Receiver {
        receiver_with_panics(events, None, None, panic_dmo_at)
    }

    fn receiver_with_panics(
        events: &Arc<Mutex<Vec<Event>>>,
        panic_normal_at: Option<usize>,
        panic_location_at: Option<usize>,
        panic_dmo_at: Option<usize>,
    ) -> Receiver {
        let state = || PassState {
            events: Arc::clone(events),
            ..PassState::default()
        };
        SimulationCollectionRegistrationAdapter::new(
            Normal(PassState {
                panic_normal_at,
                ..state()
            }),
            Location(PassState {
                panic_location_at,
                ..state()
            }),
            Dmo(PassState {
                panic_dmo_at,
                ..state()
            }),
        )
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
            id: u32::try_from(asset_id).unwrap_or(0),
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
                    id: u32::try_from(asset_id).unwrap_or(0),
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

    fn prepared(collection_id: u64, arena_ids: &[u64]) -> PreparedSimulationCollection {
        PreparedSimulationCollection::try_new(
            collection_id,
            arena_ids.iter().copied().map(empty_normal_plan).collect(),
        )
        .unwrap()
    }

    fn enqueue_all<C: fmt::Debug>(
        fifo: &mut DeferredSimulationMessageFifo<C>,
        receiver: &mut Receiver,
        prepared: &PreparedSimulationCollection,
        userdata_raw: u32,
        callback: Option<C>,
    ) {
        let mut state = fifo.begin(receiver, prepared, userdata_raw).unwrap();
        loop {
            state = match state {
                DeferredSimulationEnqueueState::NextArena(authority) => {
                    fifo.enqueue_next_arena(authority).unwrap()
                }
                DeferredSimulationEnqueueState::ReadyToFinish(authority) => {
                    let permit = fifo.enqueue_finish(authority).unwrap();
                    fifo.enqueue_callback(permit, callback).unwrap();
                    break;
                }
            };
        }
    }

    #[test]
    fn queued_prefix_dispatches_one_node_at_a_time_and_dequeues_callback_last() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver(&events);
        let prepared = prepared(0x100, &[0x10, 0x20]);
        let mut fifo = DeferredSimulationMessageFifo::new();
        enqueue_all(&mut fifo, &mut receiver, &prepared, 7, Some(99));

        assert_eq!(fifo.pending_message_count(), 4);
        assert!(events.lock().unwrap().is_empty());
        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::ArenaMessageReturned
        ));
        assert_eq!(fifo.pending_message_count(), 3);
        assert_eq!(
            *events.lock().unwrap(),
            [Event::Normal(0x10, 7), Event::Location(0x10)]
        );
        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::ArenaMessageReturned
        ));
        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::ActivationZeroMessageReturned
        ));
        assert_eq!(receiver.completed_registration_count(), 1);
        assert_eq!(fifo.pending_message_count(), 1);

        let DeferredSimulationMessageDispatch::CallbackMessageDequeued(completion) =
            fifo.dispatch_one(&mut receiver).unwrap()
        else {
            panic!("callback was not dequeued last");
        };
        let (collection_id, registration, callback) = completion.into_parts();
        assert_eq!(collection_id, 0x100);
        assert_eq!(callback, Some(99));
        assert_eq!(fifo.pending_message_count(), 0);
        assert_eq!(fifo.retained_session_count(), 0);
        receiver.deregister(registration, 8).unwrap();
        assert_eq!(receiver.live_registration_count(), 0);
        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::Idle
        ));
    }

    #[test]
    fn wrong_receiver_preserves_front_and_continuation_for_retry() {
        let first_events = Arc::new(Mutex::new(Vec::new()));
        let second_events = Arc::new(Mutex::new(Vec::new()));
        let mut first = receiver(&first_events);
        let mut second = receiver(&second_events);
        let prepared = prepared(1, &[10]);
        let mut fifo = DeferredSimulationMessageFifo::<u8>::new();
        enqueue_all(&mut fifo, &mut first, &prepared, 3, Some(1));

        assert_eq!(
            fifo.dispatch_one(&mut second).unwrap_err(),
            DeferredSimulationDispatchError::ReceiverRejected(
                DeferredSimulationReceiverRejectionKind::ForeignReceiver
            )
        );
        assert_eq!(fifo.pending_message_count(), 3);
        assert!(second_events.lock().unwrap().is_empty());
        assert!(matches!(
            fifo.dispatch_one(&mut first).unwrap(),
            DeferredSimulationMessageDispatch::ArenaMessageReturned
        ));
        assert_eq!(fifo.pending_message_count(), 2);
        assert_eq!(first_events.lock().unwrap().len(), 2);
    }

    #[test]
    fn host_generalized_sessions_preserve_global_enqueue_order() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver(&events);
        let first_prepared = prepared(1, &[11]);
        let second_prepared = prepared(2, &[22]);
        let mut fifo = DeferredSimulationMessageFifo::new();

        let DeferredSimulationEnqueueState::NextArena(first) =
            fifo.begin(&mut receiver, &first_prepared, 1).unwrap()
        else {
            panic!("first arena producer missing");
        };
        let DeferredSimulationEnqueueState::NextArena(second) =
            fifo.begin(&mut receiver, &second_prepared, 2).unwrap()
        else {
            panic!("second arena producer missing");
        };
        let DeferredSimulationEnqueueState::ReadyToFinish(first) =
            fifo.enqueue_next_arena(first).unwrap()
        else {
            panic!("first finish producer missing");
        };
        let DeferredSimulationEnqueueState::ReadyToFinish(second) =
            fifo.enqueue_next_arena(second).unwrap()
        else {
            panic!("second finish producer missing");
        };
        let first = fifo.enqueue_finish(first).unwrap();
        let second = fifo.enqueue_finish(second).unwrap();
        fifo.enqueue_callback(first, Some(10)).unwrap();
        fifo.enqueue_callback(second, Some(20)).unwrap();

        let mut callbacks = Vec::new();
        while fifo.pending_message_count() != 0 {
            if let DeferredSimulationMessageDispatch::CallbackMessageDequeued(completion) =
                fifo.dispatch_one(&mut receiver).unwrap()
            {
                callbacks.push((completion.collection_id(), completion));
            }
        }
        assert_eq!(
            *events.lock().unwrap(),
            [
                Event::Normal(11, 1),
                Event::Location(11),
                Event::Normal(22, 2),
                Event::Location(22),
                Event::Dmo(11, 1),
                Event::Dmo(22, 2),
            ]
        );
        assert_eq!(
            callbacks.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [1, 2]
        );
        for (_, completion) in callbacks {
            let (_, registration, _) = completion.into_parts();
            receiver.deregister(registration, 9).unwrap();
        }
    }

    #[test]
    fn empty_collection_still_has_finish_then_null_callback_positions() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver(&events);
        let prepared = prepared(5, &[]);
        let mut fifo = DeferredSimulationMessageFifo::<u8>::new();
        enqueue_all(&mut fifo, &mut receiver, &prepared, 4, None);
        assert_eq!(fifo.pending_message_count(), 2);
        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::ActivationZeroMessageReturned
        ));
        let DeferredSimulationMessageDispatch::CallbackMessageDequeued(completion) =
            fifo.dispatch_one(&mut receiver).unwrap()
        else {
            panic!("null callback position disappeared");
        };
        let (_, registration, callback) = completion.into_parts();
        assert_eq!(callback, None);
        receiver.deregister(registration, 5).unwrap();
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn foreign_producer_authority_is_returned_without_enqueue() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver(&events);
        let prepared = prepared(5, &[7]);
        let mut first = DeferredSimulationMessageFifo::<u8>::new();
        let mut second = DeferredSimulationMessageFifo::<u8>::new();
        let DeferredSimulationEnqueueState::NextArena(authority) =
            first.begin(&mut receiver, &prepared, 4).unwrap()
        else {
            panic!("arena authority missing");
        };

        let error = second.enqueue_next_arena(authority).unwrap_err();
        assert_eq!(
            error.kind(),
            DeferredSimulationEnqueueRejectionKind::ForeignDomain
        );
        assert_eq!(second.pending_message_count(), 0);
        let state = first.enqueue_next_arena(error.into_authority()).unwrap();
        assert!(state.is_ready_to_finish());
        assert_eq!(first.pending_message_count(), 1);
    }

    #[test]
    fn finish_can_dispatch_before_its_separate_callback_send() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver(&events);
        let prepared = prepared(6, &[7]);
        let mut fifo = DeferredSimulationMessageFifo::new();
        let DeferredSimulationEnqueueState::NextArena(arena) =
            fifo.begin(&mut receiver, &prepared, 4).unwrap()
        else {
            panic!("arena authority missing");
        };
        let DeferredSimulationEnqueueState::ReadyToFinish(finish) =
            fifo.enqueue_next_arena(arena).unwrap()
        else {
            panic!("finish authority missing");
        };
        let callback = fifo.enqueue_finish(finish).unwrap();

        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::ArenaMessageReturned
        ));
        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::ActivationZeroMessageReturned
        ));
        assert_eq!(fifo.pending_message_count(), 0);
        assert_eq!(fifo.retained_session_count(), 1);

        fifo.enqueue_callback(callback, Some(12_u8)).unwrap();
        let DeferredSimulationMessageDispatch::CallbackMessageDequeued(completion) =
            fifo.dispatch_one(&mut receiver).unwrap()
        else {
            panic!("late callback was not dequeued");
        };
        let (_, registration, callback) = completion.into_parts();
        assert_eq!(callback, Some(12));
        receiver.deregister(registration, 5).unwrap();
    }

    #[test]
    fn foreign_finish_and_callback_rejections_return_exact_ownership() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver(&events);
        let prepared = prepared(6, &[]);
        let mut first = DeferredSimulationMessageFifo::new();
        let mut second = DeferredSimulationMessageFifo::new();
        let DeferredSimulationEnqueueState::ReadyToFinish(finish) =
            first.begin(&mut receiver, &prepared, 4).unwrap()
        else {
            panic!("finish authority missing");
        };

        let error = second.enqueue_finish(finish).unwrap_err();
        assert_eq!(
            error.kind(),
            DeferredSimulationEnqueueRejectionKind::ForeignDomain
        );
        let permit = first.enqueue_finish(error.into_authority()).unwrap();
        let error = second.enqueue_callback(permit, Some(0x55_u8)).unwrap_err();
        assert_eq!(
            error.kind(),
            DeferredSimulationEnqueueRejectionKind::ForeignDomain
        );
        let (permit, callback) = error.into_parts();
        assert_eq!(callback, Some(0x55));
        first.enqueue_callback(permit, callback).unwrap();
    }

    #[test]
    fn normal_fault_retains_exact_entered_prefix() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver_with_panics(&events, Some(0), None, None);
        let prepared = prepared(8, &[1]);
        let mut fifo = DeferredSimulationMessageFifo::new();
        enqueue_all(&mut fifo, &mut receiver, &prepared, 6, Some(3_u8));

        let DeferredSimulationDispatchError::Faulted(fault) =
            fifo.dispatch_one(&mut receiver).unwrap_err()
        else {
            panic!("normal panic was not terminal");
        };
        assert_eq!(fault.message(), DeferredSimulationMessageKind::Arena);
        assert_eq!(*events.lock().unwrap(), [Event::Normal(1, 6)]);
        assert_eq!(fifo.pending_message_count(), 3);
    }

    #[test]
    fn location_fault_retains_normal_then_location_prefix() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver_with_panics(&events, None, Some(0), None);
        let prepared = prepared(8, &[1]);
        let mut fifo = DeferredSimulationMessageFifo::new();
        enqueue_all(&mut fifo, &mut receiver, &prepared, 6, Some(3_u8));

        let DeferredSimulationDispatchError::Faulted(fault) =
            fifo.dispatch_one(&mut receiver).unwrap_err()
        else {
            panic!("Location panic was not terminal");
        };
        assert_eq!(fault.message(), DeferredSimulationMessageKind::Arena);
        assert_eq!(
            *events.lock().unwrap(),
            [Event::Normal(1, 6), Event::Location(1)]
        );
        assert_eq!(fifo.pending_message_count(), 3);
    }

    #[test]
    fn matching_receiver_preexisting_fault_is_terminal_not_retryable() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver_with_dmo_panic(&events, Some(0));
        let queued = prepared(8, &[1]);
        let faulting = prepared(9, &[2]);
        let mut fifo = DeferredSimulationMessageFifo::new();
        enqueue_all(&mut fifo, &mut receiver, &queued, 6, Some(3_u8));
        assert!(receiver.register_active(&faulting, 7).is_err());
        let receiver_fault = receiver.fault().expect("receiver did not latch fault");

        let DeferredSimulationDispatchError::Faulted(fault) =
            fifo.dispatch_one(&mut receiver).unwrap_err()
        else {
            panic!("faulted matching receiver was treated as retryable");
        };
        assert_eq!(
            fault.kind(),
            DeferredSimulationDispatchFaultKind::CollectionFault(receiver_fault)
        );
        assert_eq!(fifo.pending_message_count(), 3);
        assert_eq!(
            fifo.dispatch_one(&mut receiver).unwrap_err(),
            DeferredSimulationDispatchError::Faulted(fault)
        );
    }

    #[test]
    fn sticky_fault_rejects_later_producer_and_callback_without_consuming_them() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver_with_dmo_panic(&events, Some(0));
        let first_prepared = prepared(1, &[1]);
        let producer_prepared = prepared(2, &[2, 3]);
        let finish_prepared = prepared(3, &[]);
        let callback_prepared = prepared(4, &[]);
        let mut fifo = DeferredSimulationMessageFifo::new();
        enqueue_all(&mut fifo, &mut receiver, &first_prepared, 1, Some(10_u8));
        let DeferredSimulationEnqueueState::NextArena(producer) =
            fifo.begin(&mut receiver, &producer_prepared, 2).unwrap()
        else {
            panic!("later producer missing");
        };
        let DeferredSimulationEnqueueState::ReadyToFinish(finish) =
            fifo.begin(&mut receiver, &finish_prepared, 3).unwrap()
        else {
            panic!("later finish missing");
        };
        let DeferredSimulationEnqueueState::ReadyToFinish(callback_finish) =
            fifo.begin(&mut receiver, &callback_prepared, 4).unwrap()
        else {
            panic!("callback finish missing");
        };
        let callback_permit = fifo.enqueue_finish(callback_finish).unwrap();
        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::ArenaMessageReturned
        ));
        let DeferredSimulationDispatchError::Faulted(fault) =
            fifo.dispatch_one(&mut receiver).unwrap_err()
        else {
            panic!("DMO fault missing");
        };
        let pending = fifo.pending_message_count();

        let producer_error = fifo.enqueue_next_arena(producer).unwrap_err();
        assert_eq!(
            producer_error.kind(),
            DeferredSimulationEnqueueRejectionKind::FifoFaulted(fault)
        );
        let finish_error = fifo.enqueue_finish(finish).unwrap_err();
        assert_eq!(
            finish_error.kind(),
            DeferredSimulationEnqueueRejectionKind::FifoFaulted(fault)
        );
        let callback_error = fifo
            .enqueue_callback(callback_permit, Some(0x77_u8))
            .unwrap_err();
        assert_eq!(
            callback_error.kind(),
            DeferredSimulationEnqueueRejectionKind::FifoFaulted(fault)
        );
        let (_, callback) = callback_error.into_parts();
        assert_eq!(callback, Some(0x77));
        assert_eq!(fifo.pending_message_count(), pending);
    }

    #[test]
    fn dropping_each_producer_authority_is_inert_for_receiver_state() {
        let events = Arc::new(Mutex::new(Vec::new()));

        let mut arena_receiver = receiver(&events);
        let mut arena_fifo = DeferredSimulationMessageFifo::<u8>::new();
        let DeferredSimulationEnqueueState::NextArena(arena) = arena_fifo
            .begin(&mut arena_receiver, &prepared(1, &[1]), 1)
            .unwrap()
        else {
            panic!("arena authority missing");
        };
        drop(arena);
        assert_eq!(arena_fifo.pending_message_count(), 0);
        assert_eq!(arena_fifo.retained_session_count(), 1);
        assert_eq!(arena_receiver.deferred_registration_count(), 1);

        let mut finish_receiver = receiver(&events);
        let mut finish_fifo = DeferredSimulationMessageFifo::<u8>::new();
        let DeferredSimulationEnqueueState::ReadyToFinish(finish) = finish_fifo
            .begin(&mut finish_receiver, &prepared(2, &[]), 2)
            .unwrap()
        else {
            panic!("finish authority missing");
        };
        drop(finish);
        assert_eq!(finish_fifo.pending_message_count(), 0);
        assert_eq!(finish_fifo.retained_session_count(), 1);
        assert_eq!(finish_receiver.deferred_registration_count(), 1);

        let mut callback_receiver = receiver(&events);
        let mut callback_fifo = DeferredSimulationMessageFifo::<u8>::new();
        let DeferredSimulationEnqueueState::ReadyToFinish(finish) = callback_fifo
            .begin(&mut callback_receiver, &prepared(3, &[]), 3)
            .unwrap()
        else {
            panic!("callback finish authority missing");
        };
        let callback = callback_fifo.enqueue_finish(finish).unwrap();
        drop(callback);
        assert_eq!(callback_fifo.pending_message_count(), 1);
        assert_eq!(callback_fifo.retained_session_count(), 1);
        assert_eq!(callback_receiver.deferred_registration_count(), 1);
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn session_id_exhaustion_is_pre_effect() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver(&events);
        let mut fifo = DeferredSimulationMessageFifo::<u8>::new();
        fifo.next_session_id = u64::MAX;

        assert!(matches!(
            fifo.begin(&mut receiver, &prepared(1, &[1]), 1),
            Err(DeferredSimulationBeginError::SessionIdExhausted)
        ));
        assert_eq!(fifo.retained_session_count(), 0);
        assert_eq!(receiver.live_registration_count(), 0);
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn dmo_fault_retains_front_and_never_exposes_callback() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver = receiver_with_dmo_panic(&events, Some(1));
        let prepared = prepared(8, &[1, 2]);
        let mut fifo = DeferredSimulationMessageFifo::new();
        enqueue_all(&mut fifo, &mut receiver, &prepared, 6, Some(3));
        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::ArenaMessageReturned
        ));
        assert!(matches!(
            fifo.dispatch_one(&mut receiver).unwrap(),
            DeferredSimulationMessageDispatch::ArenaMessageReturned
        ));

        let error = fifo.dispatch_one(&mut receiver).unwrap_err();
        let DeferredSimulationDispatchError::Faulted(fault) = error else {
            panic!("wrong error: {error:?}");
        };
        assert_eq!(
            fault.message(),
            DeferredSimulationMessageKind::ActivationZeroFinish
        );
        assert!(matches!(
            fault.kind(),
            DeferredSimulationDispatchFaultKind::CollectionFault(_)
        ));
        assert_eq!(fifo.pending_message_count(), 2);
        assert_eq!(fifo.fault(), Some(fault));
        assert_eq!(
            fifo.dispatch_one(&mut receiver).unwrap_err(),
            DeferredSimulationDispatchError::Faulted(fault)
        );
    }

    #[derive(Debug)]
    struct DropProbe(Arc<Mutex<usize>>);

    impl Drop for DropProbe {
        fn drop(&mut self) {
            *self.0.lock().unwrap() += 1;
        }
    }

    #[test]
    fn dropping_fifo_drops_payload_without_dispatch_or_receiver_cleanup() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let drops = Arc::new(Mutex::new(0));
        let mut receiver = receiver(&events);
        let prepared = prepared(9, &[1]);
        let mut fifo = DeferredSimulationMessageFifo::new();
        enqueue_all(
            &mut fifo,
            &mut receiver,
            &prepared,
            1,
            Some(DropProbe(Arc::clone(&drops))),
        );

        drop(fifo);

        assert_eq!(*drops.lock().unwrap(), 1);
        assert!(events.lock().unwrap().is_empty());
        assert_eq!(receiver.live_registration_count(), 1);
        assert_eq!(receiver.deferred_registration_count(), 1);
        assert_eq!(receiver.completed_registration_count(), 0);
    }
}
