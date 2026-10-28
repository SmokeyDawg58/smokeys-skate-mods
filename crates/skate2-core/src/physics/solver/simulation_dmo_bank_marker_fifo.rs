//! Private marker FIFO for one exact main-DMO-bank Simulation occurrence.
//!
//! This is a bounded owner for caller-submitted markers and the corresponding
//! receiver. It does not claim native queue capacity, global `EPostman`
//! ownership, frame cadence, or atomic multi-marker submission. Registration
//! and deregistration acknowledgements both remain at their callback fronts
//! while the existing hull coordinator applies them; `DReg` additionally
//! requires the real occurrence-bound resident-hold releaser. This FIFO does
//! not invoke the subsystem callback port. Each append reserves its single
//! slot before changing producer state. Dropping any owner or permit performs
//! no implicit protocol work.
//!
//! Weak-domain permits, typed stages and allocation refusals, sticky faults,
//! retryable hull rejection, and the opaque in-flight `DReg` front are
//! safe-host policy. Native queue and callback functions are `void`; they do
//! not expose these recovery contracts. The fixed 125-arena sequence is the
//! validated dense shipped-bank instance, not native first-null production.

#![allow(
    dead_code,
    reason = "staged private FIFO awaits the verified outer notification owner"
)]

#[cfg(test)]
use super::MainDmoBankPublishedArenaRevalidationError;
#[cfg(test)]
use super::coordinator::MainDmoBankCollectionRegistrationError;
#[cfg(test)]
use super::notification_receiver::MainDmoBankNotificationStage;
use super::{
    MainDmoBankPublishedAcquisition, MainDmoBankPublishedAcquisitionError,
    MainDmoBankPublishedAcquisitionFailure,
    coordinator::{MainDmoBankCollectionCoordinator, MainDmoBankCollectionCoordinatorInitFailure},
    notification_receiver::{
        MainDmoBankDeferredArenaReceiverError, MainDmoBankDeferredBeginRejection,
        MainDmoBankDeferredFinishReceiverError, MainDmoBankDeregistrationReceiverError,
        MainDmoBankImmediateReceiverError, MainDmoBankNotificationReceiver,
        MainDmoBankNotificationWrongStage,
    },
};
use crate::{
    collection_resident_streamer::{
        CollectionResidentStreamerError, ResidentCollectionHoldAuthorityError,
    },
    dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
    dmo_hull_acknowledgement::{
        MainDmoHullAcknowledgementCoordinator, MainDmoHullCallbackRejectionKind,
        MainDmoHullDeregistrationAckError, MainDmoHullSubsystemPort, MainDmoResidentHoldReleaser,
        MainDmoSimulationDeregistrationCallback, MainDmoSimulationRegistrationCallback,
    },
    simulation_collection_adapter::{
        SimulationCollectionDmoPass, SimulationCollectionLocationPass,
    },
    simulation_dmo_assembly_part_session::SimulationDmoRegistrationRuntimeInitError,
    simulation_normal_arena_adapter::{
        SimulationNormalArenaDispatch, SimulationNormalArenaRegistrationAdapter,
        SimulationNormalEnvironmentDispatch,
    },
    simulation_resident_registry::SimulationResidentLookup,
};
use std::{
    collections::VecDeque,
    error::Error,
    fmt,
    sync::{Arc, Weak},
};

const MAIN_DMO_BANK_ARENA_COUNT: usize = 125;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankMarkerKind {
    ImmediateCollection,
    DeferredArena { ordinal: usize },
    DeferredFinish,
    RegistrationCallback,
    DeregistrationCollection,
    DeregistrationCallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProducerStage {
    ImmediateCollection,
    DeferredArena,
    DeferredFinish,
    RegistrationCallback,
    RegistrationQueued,
    Registered,
    DeregistrationCollection,
    DeregistrationCallback,
    DeregistrationQueued,
    DeregistrationCallbackInFlight,
    Complete,
    Faulted,
}

struct PermitIdentity {
    domain: Weak<()>,
}

macro_rules! define_permit {
    ($name:ident) => {
        pub(super) struct $name(PermitIdentity);
    };
}

define_permit!(MainDmoBankImmediateMarkerProducer);
define_permit!(MainDmoBankDeferredArenaMarkerProducer);
define_permit!(MainDmoBankDeferredFinishMarkerProducer);
define_permit!(MainDmoBankRegistrationCallbackMarkerProducer);
define_permit!(MainDmoBankDeregistrationBeginPermit);
define_permit!(MainDmoBankDeregistrationCollectionMarkerProducer);
define_permit!(MainDmoBankDeregistrationCallbackMarkerProducer);
define_permit!(MainDmoBankDeregistrationCallbackFront);

pub(super) enum MainDmoBankDeferredMarkerProducerState {
    NextArena(MainDmoBankDeferredArenaMarkerProducer),
    ReadyToFinish(MainDmoBankDeferredFinishMarkerProducer),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankMarkerEnqueueRejectionKind {
    ReceiverFaulted(MainDmoBankMarkerKind),
    ForeignDomain,
    WrongStage,
    QueueStorageAllocation,
}

pub(super) struct MainDmoBankMarkerEnqueueRejection<P> {
    kind: MainDmoBankMarkerEnqueueRejectionKind,
    permit: P,
}

impl<P> MainDmoBankMarkerEnqueueRejection<P> {
    pub(super) const fn kind(&self) -> MainDmoBankMarkerEnqueueRejectionKind {
        self.kind
    }

    pub(super) fn into_permit(self) -> P {
        self.permit
    }
}

impl<P> fmt::Debug for MainDmoBankMarkerEnqueueRejection<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankMarkerEnqueueRejection")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

pub(super) struct MainDmoBankDeregistrationBeginRejection {
    kind: MainDmoBankMarkerEnqueueRejectionKind,
    permit: MainDmoBankDeregistrationBeginPermit,
    callback: MainDmoSimulationDeregistrationCallback,
}

impl MainDmoBankDeregistrationBeginRejection {
    pub(super) const fn kind(&self) -> MainDmoBankMarkerEnqueueRejectionKind {
        self.kind
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        MainDmoBankDeregistrationBeginPermit,
        MainDmoSimulationDeregistrationCallback,
    ) {
        (self.permit, self.callback)
    }
}

impl fmt::Debug for MainDmoBankDeregistrationBeginRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankDeregistrationBeginRejection")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

enum ReceiverFaultSource {
    Immediate(MainDmoBankImmediateReceiverError),
    DeferredArena(MainDmoBankDeferredArenaReceiverError),
    DeferredFinish(MainDmoBankDeferredFinishReceiverError),
    RegistrationCallback(MainDmoBankNotificationWrongStage),
    DeregistrationBegin(MainDmoBankNotificationWrongStage),
    DeregistrationCollection(MainDmoBankDeregistrationReceiverError),
    DeregistrationCallback(MainDmoBankNotificationWrongStage),
    DeregistrationAcknowledgementResidentRelease,
    DeregistrationAcknowledgementResidentReleasePanicked,
    DeferredCompletionMismatch {
        ordinal: usize,
        receiver_reported_complete: bool,
    },
}

impl fmt::Debug for ReceiverFaultSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Immediate(source) => formatter.debug_tuple("Immediate").field(source).finish(),
            Self::DeferredArena(source) => formatter
                .debug_tuple("DeferredArena")
                .field(source)
                .finish(),
            Self::DeferredFinish(source) => formatter
                .debug_tuple("DeferredFinish")
                .field(source)
                .finish(),
            Self::RegistrationCallback(source) => formatter
                .debug_tuple("RegistrationCallback")
                .field(source)
                .finish(),
            Self::DeregistrationBegin(source) => formatter
                .debug_tuple("DeregistrationBegin")
                .field(source)
                .finish(),
            Self::DeregistrationCollection(source) => formatter
                .debug_tuple("DeregistrationCollection")
                .field(source)
                .finish(),
            Self::DeregistrationCallback(source) => formatter
                .debug_tuple("DeregistrationCallback")
                .field(source)
                .finish(),
            Self::DeregistrationAcknowledgementResidentRelease => {
                formatter.write_str("DeregistrationAcknowledgementResidentRelease")
            }
            Self::DeregistrationAcknowledgementResidentReleasePanicked => {
                formatter.write_str("DeregistrationAcknowledgementResidentReleasePanicked")
            }
            Self::DeferredCompletionMismatch {
                ordinal,
                receiver_reported_complete,
            } => formatter
                .debug_struct("DeferredCompletionMismatch")
                .field("ordinal", ordinal)
                .field("receiver_reported_complete", receiver_reported_complete)
                .finish(),
        }
    }
}

pub(super) struct MainDmoBankMarkerReceiverFault {
    marker: MainDmoBankMarkerKind,
    source: ReceiverFaultSource,
}

impl MainDmoBankMarkerReceiverFault {
    pub(super) const fn marker(&self) -> MainDmoBankMarkerKind {
        self.marker
    }

    #[cfg(test)]
    pub(super) const fn published_binding_revalidation_for_test(
        &self,
    ) -> Option<MainDmoBankPublishedAcquisitionError> {
        match &self.source {
            ReceiverFaultSource::DeferredFinish(
                MainDmoBankDeferredFinishReceiverError::PublishedBindingRevalidation(source),
            )
            | ReceiverFaultSource::DeregistrationCollection(
                MainDmoBankDeregistrationReceiverError::PublishedBindingRevalidation(source),
            ) => Some(*source),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(super) const fn published_arena_revalidation_for_test(
        &self,
    ) -> Option<MainDmoBankPublishedArenaRevalidationError> {
        match &self.source {
            ReceiverFaultSource::DeferredArena(
                MainDmoBankDeferredArenaReceiverError::PublishedArenaRevalidation(source),
            ) => Some(*source),
            _ => None,
        }
    }
}

impl fmt::Debug for MainDmoBankMarkerReceiverFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankMarkerReceiverFault")
            .field("marker", &self.marker)
            .field("source", &self.source)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankMarkerDispatchError {
    ReceiverFaulted(MainDmoBankMarkerKind),
    RegistrationAcknowledgement(MainDmoHullCallbackRejectionKind),
    DeregistrationCallbackInFlight,
}

pub(super) enum MainDmoBankMarkerDispatch {
    Idle,
    ReceiverMarkerCompleted(MainDmoBankMarkerKind),
    RegistrationAcknowledged(MainDmoBankDeregistrationBeginPermit),
    DeregistrationCallbackReady(MainDmoBankDeregistrationCallbackFront),
}

pub(super) enum MainDmoBankDeregistrationAcknowledgementError<E> {
    Fifo {
        kind: MainDmoBankMarkerEnqueueRejectionKind,
        front: MainDmoBankDeregistrationCallbackFront,
    },
    HullCallback {
        kind: MainDmoHullCallbackRejectionKind,
        front: MainDmoBankDeregistrationCallbackFront,
    },
    ResidentAuthority {
        kind: ResidentCollectionHoldAuthorityError,
        front: MainDmoBankDeregistrationCallbackFront,
    },
    ResidentUnavailable {
        source: Arc<CollectionResidentStreamerError<E>>,
        front: MainDmoBankDeregistrationCallbackFront,
    },
    ResidentRelease(Arc<CollectionResidentStreamerError<E>>),
    ResidentReleasePanicked,
}

impl<E> fmt::Debug for MainDmoBankDeregistrationAcknowledgementError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fifo { kind, .. } => formatter.debug_tuple("Fifo").field(kind).finish(),
            Self::HullCallback { kind, .. } => {
                formatter.debug_tuple("HullCallback").field(kind).finish()
            }
            Self::ResidentAuthority { kind, .. } => formatter
                .debug_tuple("ResidentAuthority")
                .field(kind)
                .finish(),
            Self::ResidentUnavailable { .. } => formatter
                .debug_tuple("ResidentUnavailable")
                .field(&"streamer unavailable")
                .finish(),
            Self::ResidentRelease(_) => formatter
                .debug_tuple("ResidentRelease")
                .field(&"streamer fault")
                .finish(),
            Self::ResidentReleasePanicked => formatter.write_str("ResidentReleasePanicked"),
        }
    }
}

type ExactReceiver<ND, NE, L, D> = MainDmoBankNotificationReceiver<
    ND,
    NE,
    L,
    D,
    MainDmoSimulationRegistrationCallback,
    MainDmoSimulationDeregistrationCallback,
>;

/// Variant field order releases the callback before Normal, Location, and DMO
/// collaborators, and releases the exact acquisition/published owner last.
enum MainDmoBankImmediatePendingRecovery<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    Acquisition {
        callback: MainDmoSimulationRegistrationCallback,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        #[cfg(test)]
        injected_runtime_failure: Option<SimulationDmoRegistrationRuntimeInitError>,
        source: MainDmoBankPublishedAcquisition,
    },
    AcquisitionFault {
        callback: MainDmoSimulationRegistrationCallback,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        error: MainDmoBankPublishedAcquisitionError,
        source: MainDmoBankPublishedAcquisitionFailure,
    },
    Initialization {
        callback: MainDmoSimulationRegistrationCallback,
        #[cfg(test)]
        next_runtime_failure: Option<SimulationDmoRegistrationRuntimeInitError>,
        source: MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>,
    },
    InitializationRevalidationFault {
        callback: MainDmoSimulationRegistrationCallback,
        error: MainDmoBankPublishedAcquisitionError,
        coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
    },
    ReceiverFault {
        source: MainDmoBankImmediateReceiverError,
        receiver: ExactReceiver<ND, NE, L, D>,
    },
}

/// Sanitized refusal at the structurally retained Immediate Collection front.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankImmediatePendingDispatchError {
    Acquisition(MainDmoBankPublishedAcquisitionError),
    RuntimeInitialization(SimulationDmoRegistrationRuntimeInitError),
    PublishedBindingRevalidation(MainDmoBankPublishedAcquisitionError),
    Receiver,
}

impl fmt::Display for MainDmoBankImmediatePendingDispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Acquisition(_) => formatter
                .write_str("could not acquire the current published main DMO bank at its front"),
            Self::RuntimeInitialization(_) => formatter
                .write_str("could not initialize main DMO bank registration state at its front"),
            Self::PublishedBindingRevalidation(_) => formatter
                .write_str("retained main DMO bank publication was no longer current at its front"),
            Self::Receiver => {
                formatter.write_str("main DMO bank Immediate Collection receiver faulted")
            }
        }
    }
}

impl Error for MainDmoBankImmediatePendingDispatchError {}

/// Owning refusal which retains the exact structural front and every callback,
/// collaborator, and acquisition stage required by a same-front retry.
pub(super) struct MainDmoBankImmediatePendingDispatchFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    error: MainDmoBankImmediatePendingDispatchError,
    pending: MainDmoBankImmediatePendingFifo<ND, NE, L, D>,
}

impl<ND, NE, L, D> MainDmoBankImmediatePendingDispatchFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    #[must_use]
    pub(super) const fn error(&self) -> MainDmoBankImmediatePendingDispatchError {
        self.error
    }

    #[cfg(test)]
    pub(super) fn queue_len_for_test(&self) -> usize {
        self.pending.queue.len()
    }

    /// Re-enters the same structural marker front without exposing retained
    /// authority. Acquisition, revalidation, and receiver faults are sticky;
    /// only runtime initialization is retried.
    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "same-front recovery intentionally retains all non-reconstructible authority"
    )]
    pub(super) fn retry(
        self,
    ) -> Result<
        MainDmoBankImmediateReady<ND, NE, L, D>,
        MainDmoBankImmediatePendingDispatchFailure<ND, NE, L, D>,
    > {
        self.pending.dispatch_one()
    }

    /// Stages the next runtime-initialization refusal while retaining the
    /// ordinary production retry path. The following [`Self::retry`] performs
    /// the actual transition.
    #[cfg(test)]
    pub(super) fn with_next_runtime_failure_for_test(
        self,
        runtime_source: SimulationDmoRegistrationRuntimeInitError,
    ) -> Self {
        let MainDmoBankImmediatePendingDispatchFailure { error, pending } = self;
        let MainDmoBankImmediatePendingFifo {
            queue,
            registration_callback_producer,
            domain,
            recovery,
        } = pending;
        let MainDmoBankImmediatePendingRecovery::Initialization {
            callback,
            next_runtime_failure,
            source,
        } = recovery
        else {
            panic!("runtime refusal injection requires the retained Initialization stage")
        };
        assert!(
            next_runtime_failure.is_none(),
            "only one next runtime refusal may be staged"
        );
        MainDmoBankImmediatePendingFifo {
            queue,
            registration_callback_producer,
            domain,
            recovery: MainDmoBankImmediatePendingRecovery::Initialization {
                callback,
                next_runtime_failure: Some(runtime_source),
                source,
            },
        }
        .failure(error)
    }
}

impl<ND, NE, L, D> fmt::Debug for MainDmoBankImmediatePendingDispatchFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankImmediatePendingDispatchFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<ND, NE, L, D> fmt::Display for MainDmoBankImmediatePendingDispatchFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<ND, NE, L, D> Error for MainDmoBankImmediatePendingDispatchFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
}

/// One exact structurally queued Immediate Collection marker awaiting its
/// actual front. Construction performs no lookup, binding, runtime-manager, or
/// receiver effect and owns no resident hold. Recovery (including callback)
/// drops before marker-producer domain state.
pub(super) struct MainDmoBankImmediatePendingFifo<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    recovery: MainDmoBankImmediatePendingRecovery<ND, NE, L, D>,
    queue: VecDeque<MainDmoBankMarkerKind>,
    registration_callback_producer: MainDmoBankRegistrationCallbackMarkerProducer,
    domain: Arc<()>,
}

/// Owning safe-host queue-allocation refusal. No lookup or dispatch effect has
/// happened, and retry reconstructs the same structural pending owner. Field
/// order releases callback, collaborators, then the nonowning lookup.
pub(super) struct MainDmoBankImmediatePendingInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    callback: MainDmoSimulationRegistrationCallback,
    normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
    location: L,
    dmo: D,
    lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
}

impl<ND, NE, L, D> MainDmoBankImmediatePendingInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "the owning refusal preserves all constructor inputs without another allocation"
    )]
    pub(super) fn retry(
        self,
    ) -> Result<
        MainDmoBankImmediatePendingFifo<ND, NE, L, D>,
        MainDmoBankImmediatePendingInitFailure<ND, NE, L, D>,
    > {
        MainDmoBankImmediatePendingFifo::try_new(
            self.lookup,
            self.normal,
            self.location,
            self.dmo,
            self.callback,
        )
    }
}

impl<ND, NE, L, D> fmt::Debug for MainDmoBankImmediatePendingInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankImmediatePendingInitFailure")
            .field("error", &"queue storage allocation")
            .finish_non_exhaustive()
    }
}

impl<ND, NE, L, D> fmt::Display for MainDmoBankImmediatePendingInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("could not reserve the main DMO bank Immediate marker front")
    }
}

impl<ND, NE, L, D> Error for MainDmoBankImmediatePendingInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
}

/// Successful Immediate-front result. The empty FIFO and its next producer
/// remain inseparable here; later stages are intentionally outside this slice.
pub(super) struct MainDmoBankImmediateReady<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fifo: MainDmoBankMarkerFifo<ND, NE, L, D>,
    registration_callback_producer: MainDmoBankRegistrationCallbackMarkerProducer,
}

impl<ND, NE, L, D> MainDmoBankImmediatePendingFifo<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    #[allow(
        clippy::result_large_err,
        reason = "the owning refusal preserves all constructor inputs without another allocation"
    )]
    pub(super) fn try_new(
        lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> Result<Self, MainDmoBankImmediatePendingInitFailure<ND, NE, L, D>> {
        let mut queue = VecDeque::new();
        if queue.try_reserve(1).is_err() {
            return Err(MainDmoBankImmediatePendingInitFailure {
                callback,
                normal,
                location,
                dmo,
                lookup,
            });
        }
        queue.push_back(MainDmoBankMarkerKind::ImmediateCollection);
        let domain = Arc::new(());
        let registration_callback_producer =
            MainDmoBankRegistrationCallbackMarkerProducer(PermitIdentity {
                domain: Arc::downgrade(&domain),
            });
        Ok(Self {
            queue,
            registration_callback_producer,
            domain,
            recovery: MainDmoBankImmediatePendingRecovery::Acquisition {
                source: MainDmoBankPublishedAcquisition::new(lookup),
                normal,
                location,
                dmo,
                callback,
                #[cfg(test)]
                injected_runtime_failure: None,
            },
        })
    }

    #[cfg(test)]
    #[allow(
        clippy::result_large_err,
        reason = "the injected path preserves the production owning constructor refusal"
    )]
    pub(super) fn new_with_runtime_failure_for_test(
        lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        callback: MainDmoSimulationRegistrationCallback,
        runtime_source: SimulationDmoRegistrationRuntimeInitError,
    ) -> Result<Self, MainDmoBankImmediatePendingInitFailure<ND, NE, L, D>> {
        let mut pending = Self::try_new(lookup, normal, location, dmo, callback)?;
        let MainDmoBankImmediatePendingRecovery::Acquisition {
            injected_runtime_failure,
            ..
        } = &mut pending.recovery
        else {
            unreachable!("new pending owner starts at acquisition")
        };
        *injected_runtime_failure = Some(runtime_source);
        Ok(pending)
    }

    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        clippy::too_many_lines,
        reason = "the exhaustive ownership-state transition keeps each exact front capability in one auditable match; same-front recovery intentionally retains all non-reconstructible authority"
    )]
    pub(super) fn dispatch_one(
        self,
    ) -> Result<
        MainDmoBankImmediateReady<ND, NE, L, D>,
        MainDmoBankImmediatePendingDispatchFailure<ND, NE, L, D>,
    > {
        let Self {
            queue,
            registration_callback_producer,
            domain,
            recovery,
        } = self;
        match recovery {
            MainDmoBankImmediatePendingRecovery::Acquisition {
                source,
                normal,
                location,
                dmo,
                callback,
                #[cfg(test)]
                injected_runtime_failure,
            } => {
                let binding = match source.try_resolve() {
                    Ok(binding) => binding,
                    Err(source) => {
                        let acquisition_error = source.error();
                        let error = MainDmoBankImmediatePendingDispatchError::Acquisition(
                            acquisition_error,
                        );
                        return Err(Self {
                            queue,
                            registration_callback_producer,
                            domain,
                            recovery: MainDmoBankImmediatePendingRecovery::AcquisitionFault {
                                source,
                                normal,
                                location,
                                dmo,
                                callback,
                                error: acquisition_error,
                            },
                        }
                        .failure(error));
                    }
                };
                #[cfg(test)]
                let coordinator = if let Some(runtime_source) = injected_runtime_failure {
                    Err(
                        MainDmoBankCollectionCoordinator::fail_published_runtime_initialization_for_test(
                            binding,
                            normal,
                            location,
                            dmo,
                            runtime_source,
                        ),
                    )
                } else {
                    MainDmoBankCollectionCoordinator::try_new(binding, normal, location, dmo)
                };
                #[cfg(not(test))]
                let coordinator =
                    MainDmoBankCollectionCoordinator::try_new(binding, normal, location, dmo);
                match coordinator {
                    Ok(coordinator) => Self::dispatch_ready_coordinator(
                        queue,
                        registration_callback_producer,
                        domain,
                        coordinator,
                        callback,
                        false,
                    ),
                    Err(source) => {
                        let error = MainDmoBankImmediatePendingDispatchError::RuntimeInitialization(
                            source.source(),
                        );
                        Err(Self {
                            queue,
                            registration_callback_producer,
                            domain,
                            recovery: MainDmoBankImmediatePendingRecovery::Initialization {
                                callback,
                                #[cfg(test)]
                                next_runtime_failure: None,
                                source,
                            },
                        }
                        .failure(error))
                    }
                }
            }
            MainDmoBankImmediatePendingRecovery::AcquisitionFault {
                source,
                normal,
                location,
                dmo,
                callback,
                error,
            } => Err(Self {
                queue,
                registration_callback_producer,
                domain,
                recovery: MainDmoBankImmediatePendingRecovery::AcquisitionFault {
                    source,
                    normal,
                    location,
                    dmo,
                    callback,
                    error,
                },
            }
            .failure(MainDmoBankImmediatePendingDispatchError::Acquisition(error))),
            MainDmoBankImmediatePendingRecovery::Initialization {
                callback,
                #[cfg(test)]
                next_runtime_failure,
                source,
            } => {
                #[cfg(test)]
                let runtime = if let Some(runtime_source) = next_runtime_failure {
                    Err(source.retry_with_runtime_failure_for_test(runtime_source))
                } else {
                    source.retry()
                };
                #[cfg(not(test))]
                let runtime = source.retry();
                match runtime {
                    Ok(coordinator) => Self::dispatch_ready_coordinator(
                        queue,
                        registration_callback_producer,
                        domain,
                        coordinator,
                        callback,
                        true,
                    ),
                    Err(source) => {
                        let error = MainDmoBankImmediatePendingDispatchError::RuntimeInitialization(
                            source.source(),
                        );
                        Err(Self {
                            queue,
                            registration_callback_producer,
                            domain,
                            recovery: MainDmoBankImmediatePendingRecovery::Initialization {
                                callback,
                                #[cfg(test)]
                                next_runtime_failure: None,
                                source,
                            },
                        }
                        .failure(error))
                    }
                }
            }
            MainDmoBankImmediatePendingRecovery::InitializationRevalidationFault {
                callback,
                error,
                coordinator,
            } => Err(Self {
                queue,
                registration_callback_producer,
                domain,
                recovery: MainDmoBankImmediatePendingRecovery::InitializationRevalidationFault {
                    callback,
                    error,
                    coordinator,
                },
            }
            .failure(
                MainDmoBankImmediatePendingDispatchError::PublishedBindingRevalidation(error),
            )),
            MainDmoBankImmediatePendingRecovery::ReceiverFault { source, receiver } => Err(Self {
                queue,
                registration_callback_producer,
                domain,
                recovery: MainDmoBankImmediatePendingRecovery::ReceiverFault { source, receiver },
            }
            .failure(MainDmoBankImmediatePendingDispatchError::Receiver)),
        }
    }

    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "receiver failure deliberately retains the unboxed exact front and coordinator authority"
    )]
    fn dispatch_ready_coordinator(
        mut queue: VecDeque<MainDmoBankMarkerKind>,
        registration_callback_producer: MainDmoBankRegistrationCallbackMarkerProducer,
        domain: Arc<()>,
        coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
        callback: MainDmoSimulationRegistrationCallback,
        must_revalidate: bool,
    ) -> Result<
        MainDmoBankImmediateReady<ND, NE, L, D>,
        MainDmoBankImmediatePendingDispatchFailure<ND, NE, L, D>,
    > {
        if must_revalidate {
            if let Err(error) = coordinator.try_revalidate_published_binding() {
                return Err(Self {
                    queue,
                    registration_callback_producer,
                    domain,
                    recovery:
                        MainDmoBankImmediatePendingRecovery::InitializationRevalidationFault {
                            callback,
                            error,
                            coordinator,
                        },
                }
                .failure(
                    MainDmoBankImmediatePendingDispatchError::PublishedBindingRevalidation(error),
                ));
            }
        }
        let mut receiver = MainDmoBankNotificationReceiver::new_immediate(coordinator, callback);
        if let Err(source) = receiver.receive_immediate_collection() {
            return Err(Self {
                queue,
                registration_callback_producer,
                domain,
                recovery: MainDmoBankImmediatePendingRecovery::ReceiverFault { source, receiver },
            }
            .failure(MainDmoBankImmediatePendingDispatchError::Receiver));
        }
        debug_assert_eq!(
            queue.front(),
            Some(&MainDmoBankMarkerKind::ImmediateCollection)
        );
        queue.pop_front();
        let fifo = MainDmoBankMarkerFifo {
            queue,
            registration_callback_in_flight: None,
            deregistration_callback_in_flight: None,
            fault: None,
            receiver,
            domain,
            producer_stage: ProducerStage::RegistrationCallback,
            next_deferred_ordinal: 0,
        };
        Ok(MainDmoBankImmediateReady {
            fifo,
            registration_callback_producer,
        })
    }

    fn failure(
        self,
        error: MainDmoBankImmediatePendingDispatchError,
    ) -> MainDmoBankImmediatePendingDispatchFailure<ND, NE, L, D> {
        MainDmoBankImmediatePendingDispatchFailure {
            error,
            pending: self,
        }
    }

    #[cfg(test)]
    pub(super) fn queue_len_for_test(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
impl<ND, NE, L, D> MainDmoBankImmediateReady<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    pub(super) fn queue_len_for_test(&self) -> usize {
        self.fifo.queue.len()
    }

    pub(super) fn live_registration_count_for_test(&self) -> usize {
        self.fifo.receiver.live_registration_count_for_test()
    }

    pub(super) fn has_model_for_test(&self, model_id: u64) -> bool {
        self.fifo.receiver.has_model_for_test(model_id)
    }

    pub(super) fn producer_is_current_for_test(&self) -> bool {
        self.registration_callback_producer
            .0
            .domain
            .upgrade()
            .is_some_and(|domain| Arc::ptr_eq(&domain, &self.fifo.domain))
    }

    pub(super) fn into_queued_registration_driver_for_test(
        mut self,
    ) -> MainDmoBankReadyDriverForTest<ND, NE, L, D> {
        self.fifo
            .enqueue_registration_callback(self.registration_callback_producer)
            .expect("current Immediate Ready producer must queue its callback");
        MainDmoBankReadyDriverForTest {
            fifo: self.fifo,
            deregistration_begin: None,
        }
    }
}

/// Variant field order releases the callback before Normal, Location, and DMO
/// collaborators, and releases the exact acquisition/published owner last.
enum MainDmoBankDeferredPendingRecovery<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    Acquisition {
        callback: MainDmoSimulationRegistrationCallback,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        #[cfg(test)]
        injected_runtime_failure: Option<SimulationDmoRegistrationRuntimeInitError>,
        #[cfg(test)]
        injected_begin_failure: Option<MainDmoBankCollectionRegistrationError>,
        source: MainDmoBankPublishedAcquisition,
    },
    AcquisitionFault {
        callback: MainDmoSimulationRegistrationCallback,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        error: MainDmoBankPublishedAcquisitionError,
        source: MainDmoBankPublishedAcquisitionFailure,
    },
    Initialization {
        callback: MainDmoSimulationRegistrationCallback,
        #[cfg(test)]
        next_runtime_failure: Option<SimulationDmoRegistrationRuntimeInitError>,
        #[cfg(test)]
        injected_begin_failure: Option<MainDmoBankCollectionRegistrationError>,
        source: MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>,
    },
    InitializationRevalidationFault {
        callback: MainDmoSimulationRegistrationCallback,
        error: MainDmoBankPublishedAcquisitionError,
        coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
    },
    Begin {
        source:
            MainDmoBankDeferredBeginRejection<ND, NE, L, D, MainDmoSimulationRegistrationCallback>,
    },
    BeginRevalidationFault {
        error: MainDmoBankPublishedAcquisitionError,
        source:
            MainDmoBankDeferredBeginRejection<ND, NE, L, D, MainDmoSimulationRegistrationCallback>,
    },
}

/// Sanitized refusal from the exact deferred Setup boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankDeferredPendingSetupError {
    Acquisition(MainDmoBankPublishedAcquisitionError),
    RuntimeInitialization(SimulationDmoRegistrationRuntimeInitError),
    PublishedBindingRevalidation(MainDmoBankPublishedAcquisitionError),
    DeferredBegin,
}

impl fmt::Display for MainDmoBankDeferredPendingSetupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Acquisition(_) => formatter.write_str(
                "could not acquire the current published main DMO bank at deferred Setup",
            ),
            Self::RuntimeInitialization(_) => formatter.write_str(
                "could not initialize main DMO bank registration state at deferred Setup",
            ),
            Self::PublishedBindingRevalidation(_) => formatter.write_str(
                "retained main DMO bank publication was no longer current at deferred Setup",
            ),
            Self::DeferredBegin => {
                formatter.write_str("could not begin the main DMO bank deferred registration")
            }
        }
    }
}

impl Error for MainDmoBankDeferredPendingSetupError {}

/// Owning Setup refusal. No recovery authority, callback, binding, controller,
/// or marker producer can be extracted. Acquisition and revalidation faults
/// are sticky; runtime initialization and effect-free deferred begin may retry.
#[must_use]
pub(super) struct MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    error: MainDmoBankDeferredPendingSetupError,
    pending: MainDmoBankDeferredPendingSetup<ND, NE, L, D>,
}

impl<ND, NE, L, D> MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    pub(super) const fn error(&self) -> MainDmoBankDeferredPendingSetupError {
        self.error
    }

    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "Setup recovery intentionally retains all non-reconstructible authority"
    )]
    pub(super) fn retry(
        self,
    ) -> Result<
        MainDmoBankDeferredReady<ND, NE, L, D>,
        MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>,
    > {
        self.pending.try_setup()
    }

    /// Stages one further runtime refusal at the ordinary production retry
    /// point without bypassing its ordering relative to revalidation.
    #[cfg(test)]
    pub(super) fn with_next_runtime_failure_for_test(
        self,
        runtime_source: SimulationDmoRegistrationRuntimeInitError,
    ) -> Self {
        let Self { error, pending } = self;
        let MainDmoBankDeferredPendingSetup { recovery } = pending;
        let MainDmoBankDeferredPendingRecovery::Initialization {
            callback,
            next_runtime_failure,
            injected_begin_failure,
            source,
        } = recovery
        else {
            panic!("runtime refusal injection requires the retained Initialization stage")
        };
        assert!(
            next_runtime_failure.is_none(),
            "only one next runtime refusal may be staged"
        );
        MainDmoBankDeferredPendingSetup {
            recovery: MainDmoBankDeferredPendingRecovery::Initialization {
                callback,
                next_runtime_failure: Some(runtime_source),
                injected_begin_failure,
                source,
            },
        }
        .failure(error)
    }

    #[cfg(test)]
    pub(super) const fn stage_for_test(&self) -> MainDmoBankDeferredPendingStage {
        self.pending.stage_for_test()
    }
}

impl<ND, NE, L, D> fmt::Debug for MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankDeferredPendingSetupFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<ND, NE, L, D> fmt::Display for MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<ND, NE, L, D> Error for MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if matches!(
            self.error,
            MainDmoBankDeferredPendingSetupError::DeferredBegin
        ) {
            match &self.pending.recovery {
                MainDmoBankDeferredPendingRecovery::Begin { source } => {
                    return Some(source.source());
                }
                _ => return None,
            }
        }
        Some(&self.error)
    }
}

/// Lazy, exact-main-bank deferred Setup owner. Construction retains only its
/// callback, collaborators, and nonowning lookup. It creates no FIFO, domain,
/// marker permit, binding, runtime, deferred continuation, or dispatch effect.
pub(super) struct MainDmoBankDeferredPendingSetup<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    recovery: MainDmoBankDeferredPendingRecovery<ND, NE, L, D>,
}

/// Successful deferred Setup result. The empty mature FIFO and its first Arena
/// producer remain inseparable; this checkpoint exposes no marker authority.
#[must_use]
pub(super) struct MainDmoBankDeferredReady<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fifo: MainDmoBankMarkerFifo<ND, NE, L, D>,
    arena_producer: Option<MainDmoBankDeferredArenaMarkerProducer>,
    #[cfg(test)]
    fail_next_dense_reserve: bool,
    #[cfg(test)]
    last_dense_reservation_size: Option<usize>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankDeferredPendingStage {
    Acquisition,
    AcquisitionFault,
    Initialization,
    InitializationRevalidationFault,
    Begin,
    BeginRevalidationFault,
}

impl<ND, NE, L, D> MainDmoBankDeferredPendingSetup<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    pub(super) fn new(
        lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> Self {
        Self {
            recovery: MainDmoBankDeferredPendingRecovery::Acquisition {
                callback,
                normal,
                location,
                dmo,
                #[cfg(test)]
                injected_runtime_failure: None,
                #[cfg(test)]
                injected_begin_failure: None,
                source: MainDmoBankPublishedAcquisition::new(lookup),
            },
        }
    }

    #[cfg(test)]
    pub(super) fn new_with_runtime_failure_for_test(
        lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        callback: MainDmoSimulationRegistrationCallback,
        runtime_source: SimulationDmoRegistrationRuntimeInitError,
    ) -> Self {
        let mut pending = Self::new(lookup, normal, location, dmo, callback);
        let MainDmoBankDeferredPendingRecovery::Acquisition {
            injected_runtime_failure,
            ..
        } = &mut pending.recovery
        else {
            unreachable!("new deferred Setup starts at acquisition")
        };
        *injected_runtime_failure = Some(runtime_source);
        pending
    }

    #[cfg(test)]
    pub(super) fn new_with_begin_failure_for_test(
        lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        callback: MainDmoSimulationRegistrationCallback,
        begin_source: MainDmoBankCollectionRegistrationError,
    ) -> Self {
        let mut pending = Self::new(lookup, normal, location, dmo, callback);
        let MainDmoBankDeferredPendingRecovery::Acquisition {
            injected_begin_failure,
            ..
        } = &mut pending.recovery
        else {
            unreachable!("new deferred Setup starts at acquisition")
        };
        *injected_begin_failure = Some(begin_source);
        pending
    }

    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        clippy::too_many_lines,
        reason = "the exhaustive Setup transition retains each exact recovery capability without boxing or exposing it"
    )]
    pub(super) fn try_setup(
        self,
    ) -> Result<
        MainDmoBankDeferredReady<ND, NE, L, D>,
        MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>,
    > {
        match self.recovery {
            MainDmoBankDeferredPendingRecovery::Acquisition {
                callback,
                normal,
                location,
                dmo,
                #[cfg(test)]
                injected_runtime_failure,
                #[cfg(test)]
                injected_begin_failure,
                source,
            } => {
                let binding = match source.try_resolve() {
                    Ok(binding) => binding,
                    Err(source) => {
                        let acquisition_error = source.error();
                        return Err(Self {
                            recovery: MainDmoBankDeferredPendingRecovery::AcquisitionFault {
                                callback,
                                normal,
                                location,
                                dmo,
                                error: acquisition_error,
                                source,
                            },
                        }
                        .failure(
                            MainDmoBankDeferredPendingSetupError::Acquisition(acquisition_error),
                        ));
                    }
                };
                #[cfg(test)]
                let coordinator = if let Some(runtime_source) = injected_runtime_failure {
                    Err(
                        MainDmoBankCollectionCoordinator::fail_published_runtime_initialization_for_test(
                            binding,
                            normal,
                            location,
                            dmo,
                            runtime_source,
                        ),
                    )
                } else {
                    MainDmoBankCollectionCoordinator::try_new(binding, normal, location, dmo)
                };
                #[cfg(not(test))]
                let coordinator =
                    MainDmoBankCollectionCoordinator::try_new(binding, normal, location, dmo);
                match coordinator {
                    Ok(coordinator) => Self::begin_ready(
                        coordinator,
                        callback,
                        false,
                        #[cfg(test)]
                        injected_begin_failure,
                    ),
                    Err(source) => {
                        let error = MainDmoBankDeferredPendingSetupError::RuntimeInitialization(
                            source.source(),
                        );
                        Err(Self {
                            recovery: MainDmoBankDeferredPendingRecovery::Initialization {
                                callback,
                                #[cfg(test)]
                                next_runtime_failure: None,
                                #[cfg(test)]
                                injected_begin_failure,
                                source,
                            },
                        }
                        .failure(error))
                    }
                }
            }
            MainDmoBankDeferredPendingRecovery::AcquisitionFault {
                callback,
                normal,
                location,
                dmo,
                error,
                source,
            } => Err(Self {
                recovery: MainDmoBankDeferredPendingRecovery::AcquisitionFault {
                    callback,
                    normal,
                    location,
                    dmo,
                    error,
                    source,
                },
            }
            .failure(MainDmoBankDeferredPendingSetupError::Acquisition(error))),
            MainDmoBankDeferredPendingRecovery::Initialization {
                callback,
                #[cfg(test)]
                next_runtime_failure,
                #[cfg(test)]
                injected_begin_failure,
                source,
            } => {
                #[cfg(test)]
                let runtime = if let Some(runtime_source) = next_runtime_failure {
                    Err(source.retry_with_runtime_failure_for_test(runtime_source))
                } else {
                    source.retry()
                };
                #[cfg(not(test))]
                let runtime = source.retry();
                match runtime {
                    Ok(coordinator) => Self::begin_ready(
                        coordinator,
                        callback,
                        true,
                        #[cfg(test)]
                        injected_begin_failure,
                    ),
                    Err(source) => {
                        let error = MainDmoBankDeferredPendingSetupError::RuntimeInitialization(
                            source.source(),
                        );
                        Err(Self {
                            recovery: MainDmoBankDeferredPendingRecovery::Initialization {
                                callback,
                                #[cfg(test)]
                                next_runtime_failure: None,
                                #[cfg(test)]
                                injected_begin_failure,
                                source,
                            },
                        }
                        .failure(error))
                    }
                }
            }
            MainDmoBankDeferredPendingRecovery::InitializationRevalidationFault {
                callback,
                error,
                coordinator,
            } => Err(Self {
                recovery: MainDmoBankDeferredPendingRecovery::InitializationRevalidationFault {
                    callback,
                    error,
                    coordinator,
                },
            }
            .failure(MainDmoBankDeferredPendingSetupError::PublishedBindingRevalidation(error))),
            MainDmoBankDeferredPendingRecovery::Begin { source } => {
                if let Err(error) = source.try_revalidate_published_binding() {
                    return Err(Self {
                        recovery: MainDmoBankDeferredPendingRecovery::BeginRevalidationFault {
                            error,
                            source,
                        },
                    }
                    .failure(
                        MainDmoBankDeferredPendingSetupError::PublishedBindingRevalidation(error),
                    ));
                }
                match source.retry_deferred::<MainDmoSimulationDeregistrationCallback>() {
                    Ok(receiver) => Ok(Self::ready(receiver)),
                    Err(source) => Err(Self {
                        recovery: MainDmoBankDeferredPendingRecovery::Begin { source },
                    }
                    .failure(MainDmoBankDeferredPendingSetupError::DeferredBegin)),
                }
            }
            MainDmoBankDeferredPendingRecovery::BeginRevalidationFault { error, source } => {
                Err(Self {
                    recovery: MainDmoBankDeferredPendingRecovery::BeginRevalidationFault {
                        error,
                        source,
                    },
                }
                .failure(MainDmoBankDeferredPendingSetupError::PublishedBindingRevalidation(error)))
            }
        }
    }

    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "deferred-begin refusal deliberately retains the exact coordinator and callback"
    )]
    fn begin_ready(
        coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
        callback: MainDmoSimulationRegistrationCallback,
        must_revalidate: bool,
        #[cfg(test)] injected_begin_failure: Option<MainDmoBankCollectionRegistrationError>,
    ) -> Result<
        MainDmoBankDeferredReady<ND, NE, L, D>,
        MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>,
    > {
        if must_revalidate {
            if let Err(error) = coordinator.try_revalidate_published_binding() {
                return Err(Self {
                    recovery: MainDmoBankDeferredPendingRecovery::InitializationRevalidationFault {
                        callback,
                        error,
                        coordinator,
                    },
                }
                .failure(
                    MainDmoBankDeferredPendingSetupError::PublishedBindingRevalidation(error),
                ));
            }
        }
        #[cfg(test)]
        let receiver = if let Some(source) = injected_begin_failure {
            MainDmoBankDeferredBeginRejection::fail_for_test::<
                MainDmoSimulationDeregistrationCallback,
            >(coordinator, callback, source)
        } else {
            ExactReceiver::<ND, NE, L, D>::try_new_deferred(coordinator, callback)
        };
        #[cfg(not(test))]
        let receiver = ExactReceiver::<ND, NE, L, D>::try_new_deferred(coordinator, callback);
        match receiver {
            Ok(receiver) => Ok(Self::ready(receiver)),
            Err(source) => Err(Self {
                recovery: MainDmoBankDeferredPendingRecovery::Begin { source },
            }
            .failure(MainDmoBankDeferredPendingSetupError::DeferredBegin)),
        }
    }

    fn ready(receiver: ExactReceiver<ND, NE, L, D>) -> MainDmoBankDeferredReady<ND, NE, L, D> {
        let (fifo, first_arena_producer) = MainDmoBankMarkerFifo::from_deferred_receiver(receiver);
        MainDmoBankDeferredReady {
            fifo,
            arena_producer: Some(first_arena_producer),
            #[cfg(test)]
            fail_next_dense_reserve: false,
            #[cfg(test)]
            last_dense_reservation_size: None,
        }
    }

    fn failure(
        self,
        error: MainDmoBankDeferredPendingSetupError,
    ) -> MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D> {
        MainDmoBankDeferredPendingSetupFailure {
            error,
            pending: self,
        }
    }

    #[cfg(test)]
    const fn stage_for_test(&self) -> MainDmoBankDeferredPendingStage {
        match self.recovery {
            MainDmoBankDeferredPendingRecovery::Acquisition { .. } => {
                MainDmoBankDeferredPendingStage::Acquisition
            }
            MainDmoBankDeferredPendingRecovery::AcquisitionFault { .. } => {
                MainDmoBankDeferredPendingStage::AcquisitionFault
            }
            MainDmoBankDeferredPendingRecovery::Initialization { .. } => {
                MainDmoBankDeferredPendingStage::Initialization
            }
            MainDmoBankDeferredPendingRecovery::InitializationRevalidationFault { .. } => {
                MainDmoBankDeferredPendingStage::InitializationRevalidationFault
            }
            MainDmoBankDeferredPendingRecovery::Begin { .. } => {
                MainDmoBankDeferredPendingStage::Begin
            }
            MainDmoBankDeferredPendingRecovery::BeginRevalidationFault { .. } => {
                MainDmoBankDeferredPendingStage::BeginRevalidationFault
            }
        }
    }
}

impl<ND, NE, L, D> MainDmoBankDeferredReady<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    /// Current authored ordinal at the producer boundary. This scalar is used
    /// only by the sealed dense main-bank update owner; it carries no marker,
    /// receiver, or registration authority.
    pub(super) const fn next_deferred_ordinal(&self) -> usize {
        self.fifo.next_deferred_ordinal
    }

    /// Preflights and appends one complete dense producer update. Allocation
    /// refusal occurs before the producer permit or FIFO changes. Once the
    /// complete slot count is reserved, the sealed internal appends cannot
    /// encounter another allocation boundary.
    pub(super) fn try_enqueue_dense_deferred_update(
        &mut self,
        arena_count: usize,
    ) -> Result<bool, MainDmoBankMarkerEnqueueRejectionKind> {
        let Some(producer) = self.arena_producer.as_ref() else {
            return Err(MainDmoBankMarkerEnqueueRejectionKind::WrongStage);
        };
        self.fifo
            .validate(&producer.0, ProducerStage::DeferredArena)?;

        let Some(remaining) =
            MAIN_DMO_BANK_ARENA_COUNT.checked_sub(self.fifo.next_deferred_ordinal)
        else {
            return Err(MainDmoBankMarkerEnqueueRejectionKind::WrongStage);
        };
        if arena_count == 0 || arena_count > remaining {
            return Err(MainDmoBankMarkerEnqueueRejectionKind::WrongStage);
        }
        let completes = arena_count == remaining;
        let additional = arena_count + if completes { 2 } else { 0 };
        #[cfg(test)]
        {
            self.last_dense_reservation_size = Some(additional);
        }
        #[cfg(test)]
        if std::mem::take(&mut self.fail_next_dense_reserve) {
            return Err(MainDmoBankMarkerEnqueueRejectionKind::QueueStorageAllocation);
        }
        self.fifo
            .queue
            .try_reserve(additional)
            .map_err(|_| MainDmoBankMarkerEnqueueRejectionKind::QueueStorageAllocation)?;

        let mut state = MainDmoBankDeferredMarkerProducerState::NextArena(
            self.arena_producer
                .take()
                .expect("validated Deferred Ready retains its Arena producer"),
        );
        for _ in 0..arena_count {
            let MainDmoBankDeferredMarkerProducerState::NextArena(producer) = state else {
                unreachable!("preflighted dense update reached Finish before its final Arena")
            };
            state = self.fifo.enqueue_deferred_arena_reserved(producer);
        }

        match state {
            MainDmoBankDeferredMarkerProducerState::NextArena(producer) => {
                debug_assert!(!completes);
                self.arena_producer = Some(producer);
                Ok(false)
            }
            MainDmoBankDeferredMarkerProducerState::ReadyToFinish(finish) => {
                debug_assert!(completes);
                let callback = self.fifo.enqueue_deferred_finish_reserved(finish);
                self.fifo.enqueue_registration_callback_reserved(callback);
                Ok(true)
            }
        }
    }

    /// Consumes a producer-complete Ready owner into its already-staged FIFO.
    /// Only the dense producer's Terminal state may call this boundary.
    pub(super) fn into_terminal_fifo(self) -> MainDmoBankMarkerFifo<ND, NE, L, D> {
        debug_assert!(self.arena_producer.is_none());
        debug_assert_eq!(self.fifo.next_deferred_ordinal, MAIN_DMO_BANK_ARENA_COUNT);
        debug_assert_eq!(self.fifo.producer_stage, ProducerStage::RegistrationQueued);
        self.fifo
    }
}

#[cfg(test)]
impl<ND, NE, L, D> MainDmoBankDeferredReady<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    pub(super) fn queue_len_for_test(&self) -> usize {
        self.fifo.queue.len()
    }

    pub(super) const fn next_deferred_ordinal_for_test(&self) -> usize {
        self.fifo.next_deferred_ordinal
    }

    pub(super) fn producer_is_current_for_test(&self) -> bool {
        self.arena_producer.as_ref().is_some_and(|producer| {
            producer
                .0
                .domain
                .upgrade()
                .is_some_and(|domain| Arc::ptr_eq(&domain, &self.fifo.domain))
        })
    }

    pub(super) fn has_model_for_test(&self, model_id: u64) -> bool {
        self.fifo.receiver.has_model_for_test(model_id)
    }

    pub(super) fn queued_marker_kinds_for_test(&self) -> Vec<MainDmoBankMarkerKind> {
        self.fifo.queue.iter().copied().collect()
    }

    pub(super) fn fail_next_dense_reserve_for_test(&mut self) {
        self.fail_next_dense_reserve = true;
    }

    pub(super) const fn last_dense_reservation_size_for_test(&self) -> Option<usize> {
        self.last_dense_reservation_size
    }

    pub(super) fn try_revalidate_published_binding_for_test(
        &self,
    ) -> Result<(), MainDmoBankPublishedAcquisitionError> {
        self.fifo
            .receiver
            .try_revalidate_published_binding_for_test()
    }

    pub(super) fn into_dense_queued_registration_driver_for_test(
        mut self,
    ) -> MainDmoBankReadyDriverForTest<ND, NE, L, D> {
        let mut state = MainDmoBankDeferredMarkerProducerState::NextArena(
            self.arena_producer
                .take()
                .expect("new Deferred Ready retains its first Arena producer"),
        );
        for ordinal in 0..MAIN_DMO_BANK_ARENA_COUNT {
            let producer = match state {
                MainDmoBankDeferredMarkerProducerState::NextArena(producer) => producer,
                MainDmoBankDeferredMarkerProducerState::ReadyToFinish(_) => {
                    panic!("deferred producer finished before ordinal {ordinal}")
                }
            };
            state = self
                .fifo
                .enqueue_deferred_arena(producer)
                .expect("current Deferred Ready producer must queue its Arena");
        }
        let finish = match state {
            MainDmoBankDeferredMarkerProducerState::ReadyToFinish(finish) => finish,
            MainDmoBankDeferredMarkerProducerState::NextArena(_) => {
                panic!("dense deferred producer did not reach Finish")
            }
        };
        let callback = self
            .fifo
            .enqueue_deferred_finish(finish)
            .expect("current Deferred Ready producer must queue Finish");
        self.fifo
            .enqueue_registration_callback(callback)
            .expect("current Deferred Ready producer must queue its callback");
        MainDmoBankReadyDriverForTest {
            fifo: self.fifo,
            deregistration_begin: None,
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankReadyTestDispatch {
    Idle,
    ReceiverMarkerCompleted(MainDmoBankMarkerKind),
    RegistrationAcknowledged,
    DeregistrationCallbackReady,
}

/// Test-only sealed driver for exercising production Ready owners without
/// exposing their FIFO or producer/callback capabilities to sibling modules.
#[cfg(test)]
pub(super) struct MainDmoBankReadyDriverForTest<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fifo: MainDmoBankMarkerFifo<ND, NE, L, D>,
    deregistration_begin: Option<MainDmoBankDeregistrationBeginPermit>,
}

#[cfg(test)]
impl<ND, NE, L, D> MainDmoBankReadyDriverForTest<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    pub(super) fn dispatch_one<P: MainDmoHullSubsystemPort>(
        &mut self,
        hull: &MainDmoHullAcknowledgementCoordinator<P>,
    ) -> Result<MainDmoBankReadyTestDispatch, MainDmoBankMarkerDispatchError> {
        match self.fifo.dispatch_one(hull)? {
            MainDmoBankMarkerDispatch::Idle => Ok(MainDmoBankReadyTestDispatch::Idle),
            MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(marker) => Ok(
                MainDmoBankReadyTestDispatch::ReceiverMarkerCompleted(marker),
            ),
            MainDmoBankMarkerDispatch::RegistrationAcknowledged(permit) => {
                assert!(
                    self.deregistration_begin.replace(permit).is_none(),
                    "test driver received duplicate DReg-begin authority"
                );
                Ok(MainDmoBankReadyTestDispatch::RegistrationAcknowledged)
            }
            MainDmoBankMarkerDispatch::DeregistrationCallbackReady(_front) => {
                Ok(MainDmoBankReadyTestDispatch::DeregistrationCallbackReady)
            }
        }
    }

    pub(super) fn queue_deregistration(
        &mut self,
        callback: MainDmoSimulationDeregistrationCallback,
    ) {
        let permit = self
            .deregistration_begin
            .take()
            .expect("registration acknowledgement must precede DReg staging");
        let collection = self
            .fifo
            .begin_deregistration(permit, callback)
            .expect("current Ready DReg-begin permit must stage its callback");
        let callback = self
            .fifo
            .enqueue_deregistration_collection(collection)
            .expect("current Ready DReg producer must queue Collection");
        self.fifo
            .enqueue_deregistration_callback(callback)
            .expect("current Ready DReg producer must queue callback");
    }

    pub(super) fn queue_for_test(&self) -> Vec<MainDmoBankMarkerKind> {
        self.fifo.queue.iter().copied().collect()
    }

    pub(super) fn stage_for_test(&self) -> MainDmoBankNotificationStage {
        self.fifo.receiver.stage_for_test()
    }

    pub(super) fn live_registration_count_for_test(&self) -> usize {
        self.fifo.live_registration_count_for_test()
    }

    pub(super) fn has_model_for_test(&self, model_id: u64) -> bool {
        self.fifo.receiver.has_model_for_test(model_id)
    }

    pub(super) fn registration_callback_retained_for_test(&self) -> bool {
        self.fifo.receiver.registration_callback_retained_for_test()
    }

    pub(super) fn deregistration_callback_retained_for_test(&self) -> bool {
        self.fifo
            .receiver
            .deregistration_callback_retained_for_test()
    }

    pub(super) fn revalidation_fault_for_test(
        &self,
    ) -> Option<MainDmoBankPublishedAcquisitionError> {
        self.fifo
            .fault_for_test()?
            .published_binding_revalidation_for_test()
    }

    pub(super) fn arena_revalidation_fault_for_test(
        &self,
    ) -> Option<MainDmoBankPublishedArenaRevalidationError> {
        self.fifo
            .fault_for_test()?
            .published_arena_revalidation_for_test()
    }

    pub(super) fn probe_deferred_finish_wrong_stage_for_test(&mut self) -> bool {
        matches!(
            self.fifo.receiver.receive_deferred_finish(),
            Err(MainDmoBankDeferredFinishReceiverError::WrongStage(_))
        )
    }

    pub(super) fn probe_deregistration_collection_wrong_stage_for_test(&mut self) -> bool {
        matches!(
            self.fifo.receiver.receive_deregistration_collection(),
            Err(MainDmoBankDeregistrationReceiverError::WrongStage(_))
        )
    }

    pub(super) fn probe_deferred_arena_wrong_stage_for_test(&mut self, ordinal: usize) -> bool {
        matches!(
            self.fifo.receiver.receive_deferred_arena(ordinal),
            Err(MainDmoBankDeferredArenaReceiverError::WrongStage(_))
        )
    }
}

/// One safe-host occurrence-local marker FIFO and receiver owner for the exact
/// main-bank corpus.
pub(super) struct MainDmoBankMarkerFifo<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    queue: VecDeque<MainDmoBankMarkerKind>,
    registration_callback_in_flight: Option<MainDmoSimulationRegistrationCallback>,
    deregistration_callback_in_flight: Option<MainDmoSimulationDeregistrationCallback>,
    fault: Option<MainDmoBankMarkerReceiverFault>,
    receiver: ExactReceiver<ND, NE, L, D>,
    domain: Arc<()>,
    producer_stage: ProducerStage,
    next_deferred_ordinal: usize,
}

impl<ND, NE, L, D> MainDmoBankMarkerFifo<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    pub(super) fn new_immediate(
        coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> (Self, MainDmoBankImmediateMarkerProducer) {
        let domain = Arc::new(());
        let permit = MainDmoBankImmediateMarkerProducer(PermitIdentity {
            domain: Arc::downgrade(&domain),
        });
        (
            Self {
                queue: VecDeque::new(),
                registration_callback_in_flight: None,
                deregistration_callback_in_flight: None,
                fault: None,
                receiver: MainDmoBankNotificationReceiver::new_immediate(coordinator, callback),
                domain,
                producer_stage: ProducerStage::ImmediateCollection,
                next_deferred_ordinal: 0,
            },
            permit,
        )
    }

    #[cfg(test)]
    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "the existing owning deferred refusal preserves the coordinator and callback without an added allocation boundary"
    )]
    pub(super) fn try_new_deferred(
        coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> Result<
        (Self, MainDmoBankDeferredArenaMarkerProducer),
        MainDmoBankDeferredBeginRejection<ND, NE, L, D, MainDmoSimulationRegistrationCallback>,
    > {
        let receiver = ExactReceiver::<ND, NE, L, D>::try_new_deferred(coordinator, callback)?;
        Ok(Self::from_deferred_receiver(receiver))
    }

    fn from_deferred_receiver(
        receiver: ExactReceiver<ND, NE, L, D>,
    ) -> (Self, MainDmoBankDeferredArenaMarkerProducer) {
        let domain = Arc::new(());
        let permit = MainDmoBankDeferredArenaMarkerProducer(PermitIdentity {
            domain: Arc::downgrade(&domain),
        });
        (
            Self {
                queue: VecDeque::new(),
                registration_callback_in_flight: None,
                deregistration_callback_in_flight: None,
                fault: None,
                receiver,
                domain,
                producer_stage: ProducerStage::DeferredArena,
                next_deferred_ordinal: 0,
            },
            permit,
        )
    }

    pub(super) fn enqueue_immediate_collection(
        &mut self,
        permit: MainDmoBankImmediateMarkerProducer,
    ) -> Result<
        MainDmoBankRegistrationCallbackMarkerProducer,
        MainDmoBankMarkerEnqueueRejection<MainDmoBankImmediateMarkerProducer>,
    > {
        if let Err(kind) = self.validate_and_reserve(&permit.0, ProducerStage::ImmediateCollection)
        {
            return Err(MainDmoBankMarkerEnqueueRejection { kind, permit });
        }
        self.queue
            .push_back(MainDmoBankMarkerKind::ImmediateCollection);
        self.producer_stage = ProducerStage::RegistrationCallback;
        Ok(MainDmoBankRegistrationCallbackMarkerProducer(permit.0))
    }

    pub(super) fn enqueue_deferred_arena(
        &mut self,
        permit: MainDmoBankDeferredArenaMarkerProducer,
    ) -> Result<
        MainDmoBankDeferredMarkerProducerState,
        MainDmoBankMarkerEnqueueRejection<MainDmoBankDeferredArenaMarkerProducer>,
    > {
        if let Err(kind) = self.validate_and_reserve(&permit.0, ProducerStage::DeferredArena) {
            return Err(MainDmoBankMarkerEnqueueRejection { kind, permit });
        }
        Ok(self.enqueue_deferred_arena_reserved(permit))
    }

    pub(super) fn enqueue_deferred_finish(
        &mut self,
        permit: MainDmoBankDeferredFinishMarkerProducer,
    ) -> Result<
        MainDmoBankRegistrationCallbackMarkerProducer,
        MainDmoBankMarkerEnqueueRejection<MainDmoBankDeferredFinishMarkerProducer>,
    > {
        if let Err(kind) = self.validate_and_reserve(&permit.0, ProducerStage::DeferredFinish) {
            return Err(MainDmoBankMarkerEnqueueRejection { kind, permit });
        }
        Ok(self.enqueue_deferred_finish_reserved(permit))
    }

    pub(super) fn enqueue_registration_callback(
        &mut self,
        permit: MainDmoBankRegistrationCallbackMarkerProducer,
    ) -> Result<(), MainDmoBankMarkerEnqueueRejection<MainDmoBankRegistrationCallbackMarkerProducer>>
    {
        if let Err(kind) = self.validate_and_reserve(&permit.0, ProducerStage::RegistrationCallback)
        {
            return Err(MainDmoBankMarkerEnqueueRejection { kind, permit });
        }
        self.enqueue_registration_callback_reserved(permit);
        Ok(())
    }

    fn enqueue_deferred_arena_reserved(
        &mut self,
        permit: MainDmoBankDeferredArenaMarkerProducer,
    ) -> MainDmoBankDeferredMarkerProducerState {
        debug_assert!(
            self.validate(&permit.0, ProducerStage::DeferredArena)
                .is_ok()
        );
        debug_assert!(self.queue.len() < self.queue.capacity());
        let ordinal = self.next_deferred_ordinal;
        self.queue
            .push_back(MainDmoBankMarkerKind::DeferredArena { ordinal });
        self.next_deferred_ordinal = ordinal + 1;
        if self.next_deferred_ordinal == MAIN_DMO_BANK_ARENA_COUNT {
            self.producer_stage = ProducerStage::DeferredFinish;
            MainDmoBankDeferredMarkerProducerState::ReadyToFinish(
                MainDmoBankDeferredFinishMarkerProducer(permit.0),
            )
        } else {
            MainDmoBankDeferredMarkerProducerState::NextArena(permit)
        }
    }

    fn enqueue_deferred_finish_reserved(
        &mut self,
        permit: MainDmoBankDeferredFinishMarkerProducer,
    ) -> MainDmoBankRegistrationCallbackMarkerProducer {
        debug_assert!(
            self.validate(&permit.0, ProducerStage::DeferredFinish)
                .is_ok()
        );
        debug_assert!(self.queue.len() < self.queue.capacity());
        self.queue.push_back(MainDmoBankMarkerKind::DeferredFinish);
        self.producer_stage = ProducerStage::RegistrationCallback;
        MainDmoBankRegistrationCallbackMarkerProducer(permit.0)
    }

    fn enqueue_registration_callback_reserved(
        &mut self,
        permit: MainDmoBankRegistrationCallbackMarkerProducer,
    ) {
        let MainDmoBankRegistrationCallbackMarkerProducer(identity) = permit;
        debug_assert!(
            self.validate(&identity, ProducerStage::RegistrationCallback)
                .is_ok()
        );
        debug_assert!(self.queue.len() < self.queue.capacity());
        self.queue
            .push_back(MainDmoBankMarkerKind::RegistrationCallback);
        self.producer_stage = ProducerStage::RegistrationQueued;
    }

    pub(super) fn begin_deregistration(
        &mut self,
        permit: MainDmoBankDeregistrationBeginPermit,
        callback: MainDmoSimulationDeregistrationCallback,
    ) -> Result<
        MainDmoBankDeregistrationCollectionMarkerProducer,
        MainDmoBankDeregistrationBeginRejection,
    > {
        if let Err(kind) = self.validate(&permit.0, ProducerStage::Registered) {
            return Err(MainDmoBankDeregistrationBeginRejection {
                kind,
                permit,
                callback,
            });
        }
        match self.receiver.begin_deregistration(callback) {
            Ok(()) => {
                self.producer_stage = ProducerStage::DeregistrationCollection;
                Ok(MainDmoBankDeregistrationCollectionMarkerProducer(permit.0))
            }
            Err(source) => {
                let stage = source.stage();
                let callback = source.into_callback();
                let fault = MainDmoBankMarkerReceiverFault {
                    marker: MainDmoBankMarkerKind::DeregistrationCollection,
                    source: ReceiverFaultSource::DeregistrationBegin(stage),
                };
                self.producer_stage = ProducerStage::Faulted;
                self.fault = Some(fault);
                Err(MainDmoBankDeregistrationBeginRejection {
                    kind: MainDmoBankMarkerEnqueueRejectionKind::ReceiverFaulted(
                        MainDmoBankMarkerKind::DeregistrationCollection,
                    ),
                    permit,
                    callback,
                })
            }
        }
    }

    /// Reserves and stages the exact two-marker deregistration suffix as one
    /// public-facade transaction. Reservation is safe-host recovery policy;
    /// the resulting receiver stage and marker order are the same as the
    /// individually proven producer-capability path.
    pub(super) fn begin_and_enqueue_deregistration_pair(
        &mut self,
        permit: MainDmoBankDeregistrationBeginPermit,
        callback: MainDmoSimulationDeregistrationCallback,
    ) -> Result<(), MainDmoBankDeregistrationBeginRejection> {
        if let Err(kind) = self.validate(&permit.0, ProducerStage::Registered) {
            return Err(MainDmoBankDeregistrationBeginRejection {
                kind,
                permit,
                callback,
            });
        }
        if self.queue.try_reserve(2).is_err() {
            return Err(MainDmoBankDeregistrationBeginRejection {
                kind: MainDmoBankMarkerEnqueueRejectionKind::QueueStorageAllocation,
                permit,
                callback,
            });
        }
        if let Err(source) = self.receiver.begin_deregistration(callback) {
            let stage = source.stage();
            let callback = source.into_callback();
            self.producer_stage = ProducerStage::Faulted;
            self.fault = Some(MainDmoBankMarkerReceiverFault {
                marker: MainDmoBankMarkerKind::DeregistrationCollection,
                source: ReceiverFaultSource::DeregistrationBegin(stage),
            });
            return Err(MainDmoBankDeregistrationBeginRejection {
                kind: MainDmoBankMarkerEnqueueRejectionKind::ReceiverFaulted(
                    MainDmoBankMarkerKind::DeregistrationCollection,
                ),
                permit,
                callback,
            });
        }
        self.queue
            .push_back(MainDmoBankMarkerKind::DeregistrationCollection);
        self.queue
            .push_back(MainDmoBankMarkerKind::DeregistrationCallback);
        self.producer_stage = ProducerStage::DeregistrationQueued;
        Ok(())
    }

    pub(super) fn enqueue_deregistration_collection(
        &mut self,
        permit: MainDmoBankDeregistrationCollectionMarkerProducer,
    ) -> Result<
        MainDmoBankDeregistrationCallbackMarkerProducer,
        MainDmoBankMarkerEnqueueRejection<MainDmoBankDeregistrationCollectionMarkerProducer>,
    > {
        if let Err(kind) =
            self.validate_and_reserve(&permit.0, ProducerStage::DeregistrationCollection)
        {
            return Err(MainDmoBankMarkerEnqueueRejection { kind, permit });
        }
        self.queue
            .push_back(MainDmoBankMarkerKind::DeregistrationCollection);
        self.producer_stage = ProducerStage::DeregistrationCallback;
        Ok(MainDmoBankDeregistrationCallbackMarkerProducer(permit.0))
    }

    pub(super) fn enqueue_deregistration_callback(
        &mut self,
        permit: MainDmoBankDeregistrationCallbackMarkerProducer,
    ) -> Result<
        (),
        MainDmoBankMarkerEnqueueRejection<MainDmoBankDeregistrationCallbackMarkerProducer>,
    > {
        if let Err(kind) =
            self.validate_and_reserve(&permit.0, ProducerStage::DeregistrationCallback)
        {
            return Err(MainDmoBankMarkerEnqueueRejection { kind, permit });
        }
        self.queue
            .push_back(MainDmoBankMarkerKind::DeregistrationCallback);
        self.producer_stage = ProducerStage::DeregistrationQueued;
        Ok(())
    }

    pub(super) fn dispatch_one<P: MainDmoHullSubsystemPort>(
        &mut self,
        hull: &MainDmoHullAcknowledgementCoordinator<P>,
    ) -> Result<MainDmoBankMarkerDispatch, MainDmoBankMarkerDispatchError> {
        if let Some(fault) = &self.fault {
            return Err(MainDmoBankMarkerDispatchError::ReceiverFaulted(
                fault.marker,
            ));
        }
        let Some(marker) = self.queue.front().copied() else {
            return Ok(MainDmoBankMarkerDispatch::Idle);
        };
        match marker {
            MainDmoBankMarkerKind::ImmediateCollection => {
                if let Err(source) = self.receiver.receive_immediate_collection() {
                    return Err(self.latch_fault(marker, ReceiverFaultSource::Immediate(source)));
                }
                Ok(self.finish_front(marker))
            }
            MainDmoBankMarkerKind::DeferredArena { ordinal } => {
                let complete = match self.receiver.receive_deferred_arena(ordinal) {
                    Ok(complete) => complete,
                    Err(source) => {
                        return Err(
                            self.latch_fault(marker, ReceiverFaultSource::DeferredArena(source))
                        );
                    }
                };
                let expected_complete = ordinal + 1 == MAIN_DMO_BANK_ARENA_COUNT;
                if complete != expected_complete {
                    return Err(self.latch_fault(
                        marker,
                        ReceiverFaultSource::DeferredCompletionMismatch {
                            ordinal,
                            receiver_reported_complete: complete,
                        },
                    ));
                }
                Ok(self.finish_front(marker))
            }
            MainDmoBankMarkerKind::DeferredFinish => {
                if let Err(source) = self.receiver.receive_deferred_finish() {
                    return Err(
                        self.latch_fault(marker, ReceiverFaultSource::DeferredFinish(source))
                    );
                }
                Ok(self.finish_front(marker))
            }
            MainDmoBankMarkerKind::RegistrationCallback => {
                self.dispatch_registration_callback(hull)
            }
            MainDmoBankMarkerKind::DeregistrationCollection => {
                if let Err(source) = self.receiver.receive_deregistration_collection() {
                    return Err(self.latch_fault(
                        marker,
                        ReceiverFaultSource::DeregistrationCollection(source),
                    ));
                }
                Ok(self.finish_front(marker))
            }
            MainDmoBankMarkerKind::DeregistrationCallback => {
                if self.deregistration_callback_in_flight.is_some() {
                    return Err(MainDmoBankMarkerDispatchError::DeregistrationCallbackInFlight);
                }
                let callback = match self.receiver.receive_deregistration_callback() {
                    Ok(callback) => callback,
                    Err(source) => {
                        return Err(self.latch_fault(
                            marker,
                            ReceiverFaultSource::DeregistrationCallback(source),
                        ));
                    }
                };
                self.deregistration_callback_in_flight = Some(callback);
                self.producer_stage = ProducerStage::DeregistrationCallbackInFlight;
                Ok(MainDmoBankMarkerDispatch::DeregistrationCallbackReady(
                    MainDmoBankDeregistrationCallbackFront(PermitIdentity {
                        domain: Arc::downgrade(&self.domain),
                    }),
                ))
            }
        }
    }

    /// Applies the exact resident-Down-before-latch `DReg` acknowledgement for
    /// the callback held at the current marker front.
    ///
    /// Recoverable hull/streamer refusals restore the exact callback and
    /// return the front capability for retry. A reached release failure
    /// consumes callback authority, latches this FIFO, and retains the marker.
    /// Only complete Down plus hull-latch success removes the front marker.
    /// The returned boolean reports the safe-host streamer's resident-wrapper
    /// destruction; native `OnComplete` and queue dispatch are `void`.
    pub(super) fn acknowledge_deregistration_callback<P, R>(
        &mut self,
        front: MainDmoBankDeregistrationCallbackFront,
        hull: &MainDmoHullAcknowledgementCoordinator<P>,
        releaser: &mut R,
    ) -> Result<bool, MainDmoBankDeregistrationAcknowledgementError<R::Error>>
    where
        P: MainDmoHullSubsystemPort,
        R: MainDmoResidentHoldReleaser,
    {
        if let Err(kind) = self.validate(&front.0, ProducerStage::DeregistrationCallbackInFlight) {
            return Err(MainDmoBankDeregistrationAcknowledgementError::Fifo { kind, front });
        }
        if self.queue.front() != Some(&MainDmoBankMarkerKind::DeregistrationCallback)
            || self.deregistration_callback_in_flight.is_none()
        {
            return Err(MainDmoBankDeregistrationAcknowledgementError::Fifo {
                kind: MainDmoBankMarkerEnqueueRejectionKind::WrongStage,
                front,
            });
        }
        let callback = self
            .deregistration_callback_in_flight
            .take()
            .expect("validated DReg callback front retains its callback");
        match hull.acknowledge_simulation_deregistration(callback, releaser) {
            Ok(destroyed) => {
                self.queue.pop_front();
                self.producer_stage = ProducerStage::Complete;
                Ok(destroyed)
            }
            Err(MainDmoHullDeregistrationAckError::Callback(rejection)) => {
                let kind = rejection.kind();
                self.deregistration_callback_in_flight = Some(rejection.into_callback());
                Err(MainDmoBankDeregistrationAcknowledgementError::HullCallback { kind, front })
            }
            Err(MainDmoHullDeregistrationAckError::ResidentAuthority { kind, callback }) => {
                self.deregistration_callback_in_flight = Some(callback);
                Err(
                    MainDmoBankDeregistrationAcknowledgementError::ResidentAuthority {
                        kind,
                        front,
                    },
                )
            }
            Err(MainDmoHullDeregistrationAckError::ResidentUnavailable { source, callback }) => {
                self.deregistration_callback_in_flight = Some(callback);
                Err(
                    MainDmoBankDeregistrationAcknowledgementError::ResidentUnavailable {
                        source,
                        front,
                    },
                )
            }
            Err(MainDmoHullDeregistrationAckError::ResidentRelease(source)) => {
                self.latch_deregistration_acknowledgement_fault(
                    ReceiverFaultSource::DeregistrationAcknowledgementResidentRelease,
                );
                Err(MainDmoBankDeregistrationAcknowledgementError::ResidentRelease(source))
            }
            Err(MainDmoHullDeregistrationAckError::ResidentReleasePanicked) => {
                self.latch_deregistration_acknowledgement_fault(
                    ReceiverFaultSource::DeregistrationAcknowledgementResidentReleasePanicked,
                );
                Err(MainDmoBankDeregistrationAcknowledgementError::ResidentReleasePanicked)
            }
        }
    }

    fn dispatch_registration_callback<P: MainDmoHullSubsystemPort>(
        &mut self,
        hull: &MainDmoHullAcknowledgementCoordinator<P>,
    ) -> Result<MainDmoBankMarkerDispatch, MainDmoBankMarkerDispatchError> {
        let marker = MainDmoBankMarkerKind::RegistrationCallback;
        if self.registration_callback_in_flight.is_none() {
            let callback = match self.receiver.receive_registration_callback() {
                Ok(callback) => callback,
                Err(source) => {
                    return Err(
                        self.latch_fault(marker, ReceiverFaultSource::RegistrationCallback(source))
                    );
                }
            };
            self.registration_callback_in_flight = Some(callback);
        }
        let callback = self
            .registration_callback_in_flight
            .take()
            .expect("registration callback is retained at the FIFO front");
        match hull.acknowledge_simulation_registration(callback) {
            Ok(()) => {
                self.queue.pop_front();
                self.producer_stage = ProducerStage::Registered;
                Ok(MainDmoBankMarkerDispatch::RegistrationAcknowledged(
                    MainDmoBankDeregistrationBeginPermit(PermitIdentity {
                        domain: Arc::downgrade(&self.domain),
                    }),
                ))
            }
            Err(rejection) => {
                let kind = rejection.kind();
                self.registration_callback_in_flight = Some(rejection.into_callback());
                Err(MainDmoBankMarkerDispatchError::RegistrationAcknowledgement(
                    kind,
                ))
            }
        }
    }

    fn finish_front(&mut self, marker: MainDmoBankMarkerKind) -> MainDmoBankMarkerDispatch {
        self.queue.pop_front();
        MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(marker)
    }

    fn validate_and_reserve(
        &mut self,
        identity: &PermitIdentity,
        stage: ProducerStage,
    ) -> Result<(), MainDmoBankMarkerEnqueueRejectionKind> {
        self.validate(identity, stage)?;
        self.queue
            .try_reserve(1)
            .map_err(|_| MainDmoBankMarkerEnqueueRejectionKind::QueueStorageAllocation)
    }

    fn validate(
        &self,
        identity: &PermitIdentity,
        stage: ProducerStage,
    ) -> Result<(), MainDmoBankMarkerEnqueueRejectionKind> {
        if let Some(fault) = &self.fault {
            return Err(MainDmoBankMarkerEnqueueRejectionKind::ReceiverFaulted(
                fault.marker,
            ));
        }
        if !Weak::ptr_eq(&identity.domain, &Arc::downgrade(&self.domain)) {
            return Err(MainDmoBankMarkerEnqueueRejectionKind::ForeignDomain);
        }
        if self.producer_stage != stage {
            return Err(MainDmoBankMarkerEnqueueRejectionKind::WrongStage);
        }
        Ok(())
    }

    fn latch_fault(
        &mut self,
        marker: MainDmoBankMarkerKind,
        source: ReceiverFaultSource,
    ) -> MainDmoBankMarkerDispatchError {
        self.producer_stage = ProducerStage::Faulted;
        self.fault = Some(MainDmoBankMarkerReceiverFault { marker, source });
        MainDmoBankMarkerDispatchError::ReceiverFaulted(marker)
    }

    fn latch_deregistration_acknowledgement_fault(&mut self, source: ReceiverFaultSource) {
        let marker = MainDmoBankMarkerKind::DeregistrationCallback;
        self.producer_stage = ProducerStage::Faulted;
        self.fault = Some(MainDmoBankMarkerReceiverFault { marker, source });
    }

    #[cfg(test)]
    pub(super) fn queue_len_for_test(&self) -> usize {
        self.queue.len()
    }

    #[cfg(test)]
    pub(super) fn fault_for_test(&self) -> Option<&MainDmoBankMarkerReceiverFault> {
        self.fault.as_ref()
    }

    #[cfg(test)]
    pub(super) fn live_registration_count_for_test(&self) -> usize {
        self.receiver.live_registration_count_for_test()
    }
}
