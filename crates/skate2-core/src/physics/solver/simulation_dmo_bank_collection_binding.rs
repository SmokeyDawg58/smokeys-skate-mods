//! Opaque safe-host binding between one exact prepared main-DMO-bank model
//! source batch and the collection views derived from the same retained arena
//! owners.
//!
//! This does not claim a corresponding native owner, collection lifetime, or
//! dispatch boundary. It performs no registration effect and does not expose
//! retained assets or source rows.
//! `DMO_BANK_COLLECTION_ID` is fixed typed corpus metadata from the upstream
//! DMO-bank gate; the resident container itself carries no semantic collection
//! ID, so this is not independent owner-ID evidence.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "staged exact binding awaits the outer DMO collection coordinator"
    )
)]

use super::model_registration_batch::{
    SimulationDmoBankModelRegistrationBatch, SimulationDmoBankModelRegistrationBatchError,
    SimulationDmoBankPreparedCollectionError,
};
use crate::{
    collection_resident_streamer::{CollectionId, ResidentCollectionHoldAuthorityError},
    dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
    dmo_hull_acknowledgement::{
        MainDmoHullAcknowledgementCoordinator, MainDmoHullCallbackRejectionKind,
        MainDmoHullSubsystemPort, MainDmoResidentHoldReleaser,
        MainDmoSimulationDeregistrationCallback, MainDmoSimulationRegistrationCallback,
    },
    simulation_collection_adapter::{
        PreparedSimulationCollection, SimulationCollectionDmoPass,
        SimulationCollectionLocationPass, SimulationCollectionPrepareError,
    },
    simulation_deferred_batch_policy::{
        DeferredCollectionBatchControls, DeferredCollectionProgressError,
    },
    simulation_normal_arena_adapter::{
        SimulationNormalArenaDispatch, SimulationNormalArenaRegistrationAdapter,
        SimulationNormalEnvironmentDispatch,
    },
    simulation_resident_registry::{
        SimulationMainDmoBankResidentRevalidationError, SimulationMainDmoBankResidentSnapshot,
        SimulationMainDmoBankResidentSnapshotConsumeFailure,
        SimulationMainDmoBankResidentSnapshotError, SimulationMainDmoBankResidentSnapshotFailure,
        SimulationResidentArenaLookupError, SimulationResidentLookup,
        SimulationResidentLookupError,
    },
};
use std::{error::Error, fmt};

#[cfg(test)]
use crate::owned_resident_collection::OwnedResidentCollection;

#[path = "simulation_dmo_bank_collection_coordinator.rs"]
mod coordinator;
#[path = "simulation_dmo_bank_deferred_producer.rs"]
mod deferred_producer;
#[path = "simulation_dmo_bank_marker_fifo.rs"]
mod marker_fifo;
#[path = "simulation_dmo_bank_notification_receiver.rs"]
mod notification_receiver;

// Supplied `DMO_Pres/cPres_Global` corpus census. This is neither a native
// format restriction nor evidence that other collection banks omit kind 31.
const DMO_BANK_KIND_31_LOCATOR_OPERATION_COUNT: usize = 0;

/// Observable local effect of one exact-main-bank Simulation producer update.
/// This reports staged occurrence-local markers only; it makes no claim about
/// global postman dispatch or callback timing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoSimulationDeferredProducerUpdate {
    SetupCompleted,
    ArenasQueued { count: u32 },
    TerminalQueued { arena_count: u32 },
    TerminalIdle,
}

impl From<deferred_producer::MainDmoBankDenseDeferredProducerUpdate>
    for MainDmoSimulationDeferredProducerUpdate
{
    fn from(source: deferred_producer::MainDmoBankDenseDeferredProducerUpdate) -> Self {
        match source {
            deferred_producer::MainDmoBankDenseDeferredProducerUpdate::SetupCompleted => {
                Self::SetupCompleted
            }
            deferred_producer::MainDmoBankDenseDeferredProducerUpdate::ArenasQueued { count } => {
                Self::ArenasQueued { count }
            }
            deferred_producer::MainDmoBankDenseDeferredProducerUpdate::TerminalQueued {
                arena_count,
            } => Self::TerminalQueued { arena_count },
            deferred_producer::MainDmoBankDenseDeferredProducerUpdate::TerminalIdle => {
                Self::TerminalIdle
            }
        }
    }
}

/// Sanitized marker identity emitted by the sealed Simulation FIFO driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoSimulationDeferredMarkerKind {
    ImmediateCollection,
    Arena { ordinal: u32 },
    Finish,
    RegistrationCallback,
    DeregistrationCollection,
    DeregistrationCallback,
}

impl From<marker_fifo::MainDmoBankMarkerKind> for MainDmoSimulationDeferredMarkerKind {
    fn from(source: marker_fifo::MainDmoBankMarkerKind) -> Self {
        match source {
            marker_fifo::MainDmoBankMarkerKind::ImmediateCollection => Self::ImmediateCollection,
            marker_fifo::MainDmoBankMarkerKind::DeferredArena { ordinal } => Self::Arena {
                ordinal: u32::try_from(ordinal)
                    .expect("the pinned main-bank Arena ordinal fits u32"),
            },
            marker_fifo::MainDmoBankMarkerKind::DeferredFinish => Self::Finish,
            marker_fifo::MainDmoBankMarkerKind::RegistrationCallback => Self::RegistrationCallback,
            marker_fifo::MainDmoBankMarkerKind::DeregistrationCollection => {
                Self::DeregistrationCollection
            }
            marker_fifo::MainDmoBankMarkerKind::DeregistrationCallback => {
                Self::DeregistrationCallback
            }
        }
    }
}

/// Effect of one sealed FIFO dispatch operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoSimulationDeferredDispatch {
    Idle,
    ReceiverMarkerCompleted(MainDmoSimulationDeferredMarkerKind),
    RegistrationAcknowledged,
    DeregistrationCallbackReady,
}

/// Sanitized refusal from the sealed FIFO dispatcher.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoSimulationDeferredDispatchError {
    ReceiverFaulted(MainDmoSimulationDeferredMarkerKind),
    RegistrationAcknowledgement(MainDmoHullCallbackRejectionKind),
    DeregistrationCallbackInFlight,
    DuplicateRegistrationAcknowledgement,
}

impl fmt::Display for MainDmoSimulationDeferredDispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReceiverFaulted(marker) => {
                write!(formatter, "Simulation deferred marker {marker:?} faulted")
            }
            Self::RegistrationAcknowledgement(kind) => write!(
                formatter,
                "Simulation deferred registration callback was rejected: {kind:?}"
            ),
            Self::DeregistrationCallbackInFlight => formatter
                .write_str("Simulation deferred deregistration callback is already in flight"),
            Self::DuplicateRegistrationAcknowledgement => formatter.write_str(
                "Simulation deferred FIFO produced duplicate registration acknowledgement authority",
            ),
        }
    }
}

impl Error for MainDmoSimulationDeferredDispatchError {}

impl From<marker_fifo::MainDmoBankMarkerDispatchError> for MainDmoSimulationDeferredDispatchError {
    fn from(source: marker_fifo::MainDmoBankMarkerDispatchError) -> Self {
        match source {
            marker_fifo::MainDmoBankMarkerDispatchError::ReceiverFaulted(marker) => {
                Self::ReceiverFaulted(marker.into())
            }
            marker_fifo::MainDmoBankMarkerDispatchError::RegistrationAcknowledgement(kind) => {
                Self::RegistrationAcknowledgement(kind)
            }
            marker_fifo::MainDmoBankMarkerDispatchError::DeregistrationCallbackInFlight => {
                Self::DeregistrationCallbackInFlight
            }
        }
    }
}

/// Sanitized refusal while staging the exact Simulation deregistration pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoSimulationDeferredDeregistrationQueueError {
    RegistrationNotAcknowledged,
    ReceiverFaulted(MainDmoSimulationDeferredMarkerKind),
    ForeignDriver,
    WrongStage,
    QueueStorageAllocation,
}

impl From<marker_fifo::MainDmoBankMarkerEnqueueRejectionKind>
    for MainDmoSimulationDeferredDeregistrationQueueError
{
    fn from(source: marker_fifo::MainDmoBankMarkerEnqueueRejectionKind) -> Self {
        match source {
            marker_fifo::MainDmoBankMarkerEnqueueRejectionKind::ReceiverFaulted(marker) => {
                Self::ReceiverFaulted(marker.into())
            }
            marker_fifo::MainDmoBankMarkerEnqueueRejectionKind::ForeignDomain => {
                Self::ForeignDriver
            }
            marker_fifo::MainDmoBankMarkerEnqueueRejectionKind::WrongStage => Self::WrongStage,
            marker_fifo::MainDmoBankMarkerEnqueueRejectionKind::QueueStorageAllocation => {
                Self::QueueStorageAllocation
            }
        }
    }
}

impl fmt::Display for MainDmoSimulationDeferredDeregistrationQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Simulation deferred deregistration pair was refused: {self:?}"
        )
    }
}

impl Error for MainDmoSimulationDeferredDeregistrationQueueError {}

/// Owning staging refusal. The driver, callback, and retained private begin
/// permit remain inseparable until retry or explicit recovery.
#[must_use]
pub struct MainDmoSimulationDeferredDeregistrationQueueFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    kind: MainDmoSimulationDeferredDeregistrationQueueError,
    driver: MainDmoSimulationDeferredRegistrationDriver<ND, NE, L, D>,
    callback: MainDmoSimulationDeregistrationCallback,
}

/// Sanitized refusal while acknowledging the in-flight Simulation `DReg`
/// callback and releasing its resident hold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoSimulationDeferredDeregistrationAcknowledgementError {
    CallbackNotReady,
    Fifo(MainDmoSimulationDeferredDeregistrationQueueError),
    HullCallback(MainDmoHullCallbackRejectionKind),
    ResidentAuthority(ResidentCollectionHoldAuthorityError),
    ResidentUnavailable,
    ResidentRelease,
    ResidentReleasePanicked,
}

impl fmt::Display for MainDmoSimulationDeferredDeregistrationAcknowledgementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Simulation deferred deregistration acknowledgement was refused: {self:?}"
        )
    }
}

impl Error for MainDmoSimulationDeferredDeregistrationAcknowledgementError {}

/// Sealed dispatcher obtained only after the exact producer queues all 125
/// Arena markers, Finish, and its registration callback.
#[must_use]
pub struct MainDmoSimulationDeferredRegistrationDriver<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fifo: marker_fifo::MainDmoBankMarkerFifo<ND, NE, L, D>,
    deregistration_begin: Option<marker_fifo::MainDmoBankDeregistrationBeginPermit>,
    deregistration_callback_front: Option<marker_fifo::MainDmoBankDeregistrationCallbackFront>,
}

impl<ND, NE, L, D> MainDmoSimulationDeferredRegistrationDriver<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    /// Delivers one FIFO front through the exact receiver. Registration
    /// acknowledgement authority is retained privately for later teardown.
    ///
    /// # Errors
    ///
    /// Returns the sanitized sticky receiver or hull-callback refusal without
    /// exposing a marker permit or callback capability.
    pub fn dispatch_one<P: MainDmoHullSubsystemPort>(
        &mut self,
        hull: &MainDmoHullAcknowledgementCoordinator<P>,
    ) -> Result<MainDmoSimulationDeferredDispatch, MainDmoSimulationDeferredDispatchError> {
        match self
            .fifo
            .dispatch_one(hull)
            .map_err(MainDmoSimulationDeferredDispatchError::from)?
        {
            marker_fifo::MainDmoBankMarkerDispatch::Idle => {
                Ok(MainDmoSimulationDeferredDispatch::Idle)
            }
            marker_fifo::MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(marker) => Ok(
                MainDmoSimulationDeferredDispatch::ReceiverMarkerCompleted(marker.into()),
            ),
            marker_fifo::MainDmoBankMarkerDispatch::RegistrationAcknowledged(permit) => {
                if self.deregistration_begin.replace(permit).is_some() {
                    return Err(
                        MainDmoSimulationDeferredDispatchError::DuplicateRegistrationAcknowledgement,
                    );
                }
                Ok(MainDmoSimulationDeferredDispatch::RegistrationAcknowledged)
            }
            marker_fifo::MainDmoBankMarkerDispatch::DeregistrationCallbackReady(front) => {
                self.deregistration_callback_front = Some(front);
                Ok(MainDmoSimulationDeferredDispatch::DeregistrationCallbackReady)
            }
        }
    }

    /// Reports only whether registration acknowledgement produced and retained
    /// the private authority required to begin a later deregistration.
    #[must_use]
    pub const fn registration_acknowledged(&self) -> bool {
        self.deregistration_begin.is_some()
    }

    /// Consumes the retained acknowledgement authority and stages
    /// `DRegCollection` followed by the Simulation callback marker.
    ///
    /// # Errors
    ///
    /// Returns an owning refusal containing this driver and the callback. No
    /// caller authority is lost on a pre-effect or allocation refusal.
    #[allow(
        clippy::result_large_err,
        reason = "staging refusal must retain the complete driver and callback"
    )]
    pub fn try_queue_deregistration(
        mut self,
        callback: MainDmoSimulationDeregistrationCallback,
    ) -> Result<Self, MainDmoSimulationDeferredDeregistrationQueueFailure<ND, NE, L, D>> {
        let Some(permit) = self.deregistration_begin.take() else {
            return Err(MainDmoSimulationDeferredDeregistrationQueueFailure {
                kind:
                    MainDmoSimulationDeferredDeregistrationQueueError::RegistrationNotAcknowledged,
                driver: self,
                callback,
            });
        };
        match self
            .fifo
            .begin_and_enqueue_deregistration_pair(permit, callback)
        {
            Ok(()) => Ok(self),
            Err(rejection) => {
                let kind = rejection.kind().into();
                let (permit, callback) = rejection.into_parts();
                self.deregistration_begin = Some(permit);
                Err(MainDmoSimulationDeferredDeregistrationQueueFailure {
                    kind,
                    driver: self,
                    callback,
                })
            }
        }
    }

    /// Applies the existing resident-Down-before-latch acknowledgement to the
    /// `DReg` callback retained at the current FIFO front.
    ///
    /// # Errors
    ///
    /// Recoverable FIFO, hull, and resident-authority refusals retain the exact
    /// callback front inside this driver. Reached resident-release failures
    /// remain latched by the private FIFO.
    pub fn acknowledge_deregistration_callback<P, R>(
        &mut self,
        hull: &MainDmoHullAcknowledgementCoordinator<P>,
        releaser: &mut R,
    ) -> Result<bool, MainDmoSimulationDeferredDeregistrationAcknowledgementError>
    where
        P: MainDmoHullSubsystemPort,
        R: MainDmoResidentHoldReleaser,
    {
        let Some(front) = self.deregistration_callback_front.take() else {
            return Err(
                MainDmoSimulationDeferredDeregistrationAcknowledgementError::CallbackNotReady,
            );
        };
        match self
            .fifo
            .acknowledge_deregistration_callback(front, hull, releaser)
        {
            Ok(destroyed) => Ok(destroyed),
            Err(marker_fifo::MainDmoBankDeregistrationAcknowledgementError::Fifo {
                kind,
                front,
            }) => {
                self.deregistration_callback_front = Some(front);
                Err(
                    MainDmoSimulationDeferredDeregistrationAcknowledgementError::Fifo(kind.into()),
                )
            }
            Err(
                marker_fifo::MainDmoBankDeregistrationAcknowledgementError::HullCallback {
                    kind,
                    front,
                },
            ) => {
                self.deregistration_callback_front = Some(front);
                Err(
                    MainDmoSimulationDeferredDeregistrationAcknowledgementError::HullCallback(
                        kind,
                    ),
                )
            }
            Err(
                marker_fifo::MainDmoBankDeregistrationAcknowledgementError::ResidentAuthority {
                    kind,
                    front,
                },
            ) => {
                self.deregistration_callback_front = Some(front);
                Err(
                    MainDmoSimulationDeferredDeregistrationAcknowledgementError::ResidentAuthority(
                        kind,
                    ),
                )
            }
            Err(
                marker_fifo::MainDmoBankDeregistrationAcknowledgementError::ResidentUnavailable {
                    front,
                    ..
                },
            ) => {
                self.deregistration_callback_front = Some(front);
                Err(
                    MainDmoSimulationDeferredDeregistrationAcknowledgementError::ResidentUnavailable,
                )
            }
            Err(
                marker_fifo::MainDmoBankDeregistrationAcknowledgementError::ResidentRelease(_),
            ) => Err(
                MainDmoSimulationDeferredDeregistrationAcknowledgementError::ResidentRelease,
            ),
            Err(
                marker_fifo::MainDmoBankDeregistrationAcknowledgementError::ResidentReleasePanicked,
            ) => Err(
                MainDmoSimulationDeferredDeregistrationAcknowledgementError::ResidentReleasePanicked,
            ),
        }
    }
}

impl<ND, NE, L, D> MainDmoSimulationDeferredDeregistrationQueueFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    #[must_use]
    pub const fn kind(&self) -> MainDmoSimulationDeferredDeregistrationQueueError {
        self.kind
    }

    /// Retries the exact retained driver, begin permit, and callback.
    ///
    /// # Errors
    ///
    /// Returns another owning refusal if the same front remains unavailable.
    #[allow(
        clippy::result_large_err,
        reason = "retry must preserve the exact driver and callback authority"
    )]
    pub fn retry(self) -> Result<MainDmoSimulationDeferredRegistrationDriver<ND, NE, L, D>, Self> {
        self.driver.try_queue_deregistration(self.callback)
    }

    /// Recovers both caller-owned values without exposing any private permit.
    pub fn into_parts(
        self,
    ) -> (
        MainDmoSimulationDeferredRegistrationDriver<ND, NE, L, D>,
        MainDmoSimulationDeregistrationCallback,
    ) {
        (self.driver, self.callback)
    }
}

impl<ND, NE, L, D> fmt::Debug for MainDmoSimulationDeferredDeregistrationQueueFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoSimulationDeferredDeregistrationQueueFailure")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl<ND, NE, L, D> fmt::Display
    for MainDmoSimulationDeferredDeregistrationQueueFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl<ND, NE, L, D> Error for MainDmoSimulationDeferredDeregistrationQueueFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
}

/// Sanitized class of an owning producer refusal. Private Setup and queue
/// authorities never escape this façade; a caller-supplied controls error can
/// still be inspected by reference.
#[derive(Debug)]
pub enum MainDmoSimulationDeferredProducerFailureKind<'a, E> {
    Setup,
    Controls(&'a E),
    Progress(&'a DeferredCollectionProgressError),
    Queue,
}

/// Opaque exact-main-bank deferred Simulation producer.
///
/// The owner keeps the published-occurrence acquisition, reconstructed
/// registration collaborators, marker FIFO capabilities, and hull callback
/// inseparable. It can be moved through a manager payload without exposing any
/// private coordinator or marker permit.
#[must_use]
pub struct MainDmoSimulationDeferredProducer<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    inner: deferred_producer::MainDmoBankDenseDeferredProducer<ND, NE, L, D>,
}

/// Owning update refusal. The exact producer and every non-cloneable
/// capability remain inside this value for a same-stage retry.
#[must_use]
pub struct MainDmoSimulationDeferredProducerFailure<ND, NE, L, D, E>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    inner: deferred_producer::MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>,
}

impl<ND, NE, L, D> MainDmoSimulationDeferredProducer<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    /// Creates a Pending owner without performing lookup, Setup, registration,
    /// marker append, or callback work.
    pub fn new(
        lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> Self {
        Self {
            inner: deferred_producer::MainDmoBankDenseDeferredProducer::new(
                lookup, normal, location, dmo, callback,
            ),
        }
    }

    /// Advances exactly one local producer invocation. Pending performs Setup
    /// only; each Active call samples controls once; Terminal is inert.
    ///
    /// # Errors
    ///
    /// Returns an owning refusal that retains all exact retry authority.
    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "the owning refusal deliberately retains the exact private producer state"
    )]
    pub fn try_update_with<F, E>(
        self,
        controls: F,
    ) -> Result<
        (Self, MainDmoSimulationDeferredProducerUpdate),
        MainDmoSimulationDeferredProducerFailure<ND, NE, L, D, E>,
    >
    where
        F: FnOnce() -> Result<DeferredCollectionBatchControls, E>,
    {
        self.inner
            .try_update_with(controls)
            .map(|(inner, update)| (Self { inner }, update.into()))
            .map_err(|inner| MainDmoSimulationDeferredProducerFailure { inner })
    }

    /// Converts only a Terminal producer into the sealed registration FIFO
    /// driver. Earlier stages return the unchanged producer.
    ///
    /// # Errors
    ///
    /// Returns `self` unchanged when Setup or Arena production is incomplete.
    #[allow(
        clippy::result_large_err,
        reason = "conversion refusal must return the unchanged non-cloneable producer"
    )]
    pub fn try_into_registration_driver(
        self,
    ) -> Result<MainDmoSimulationDeferredRegistrationDriver<ND, NE, L, D>, Self> {
        self.inner
            .try_into_terminal_fifo()
            .map(|fifo| MainDmoSimulationDeferredRegistrationDriver {
                fifo,
                deregistration_begin: None,
                deregistration_callback_front: None,
            })
            .map_err(|inner| Self { inner })
    }
}

impl<ND, NE, L, D, E> MainDmoSimulationDeferredProducerFailure<ND, NE, L, D, E>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    /// Borrows a public refusal class without exposing retained private
    /// producer, coordinator, FIFO, or marker capabilities.
    #[must_use]
    pub const fn kind(&self) -> MainDmoSimulationDeferredProducerFailureKind<'_, E> {
        match self.inner.error() {
            deferred_producer::MainDmoBankDenseDeferredProducerUpdateError::Setup(_) => {
                MainDmoSimulationDeferredProducerFailureKind::Setup
            }
            deferred_producer::MainDmoBankDenseDeferredProducerUpdateError::Controls(source) => {
                MainDmoSimulationDeferredProducerFailureKind::Controls(source)
            }
            deferred_producer::MainDmoBankDenseDeferredProducerUpdateError::Progress(source) => {
                MainDmoSimulationDeferredProducerFailureKind::Progress(source)
            }
            deferred_producer::MainDmoBankDenseDeferredProducerUpdateError::Queue(_) => {
                MainDmoSimulationDeferredProducerFailureKind::Queue
            }
        }
    }

    /// Retries the exact retained producer with a fresh lazy control provider.
    ///
    /// # Errors
    ///
    /// Returns the same owning failure type with updated retained state if the
    /// exact retry is still refused.
    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "retry must preserve the same owning failure type and capabilities"
    )]
    pub fn retry_with<F>(
        self,
        controls: F,
    ) -> Result<
        (
            MainDmoSimulationDeferredProducer<ND, NE, L, D>,
            MainDmoSimulationDeferredProducerUpdate,
        ),
        Self,
    >
    where
        F: FnOnce() -> Result<DeferredCollectionBatchControls, E>,
    {
        self.inner
            .retry_with(controls)
            .map(|(inner, update)| (MainDmoSimulationDeferredProducer { inner }, update.into()))
            .map_err(|inner| Self { inner })
    }
}

impl<ND, NE, L, D, E: fmt::Debug> fmt::Debug
    for MainDmoSimulationDeferredProducerFailure<ND, NE, L, D, E>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoSimulationDeferredProducerFailure")
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

impl<ND, NE, L, D, E: fmt::Display> fmt::Display
    for MainDmoSimulationDeferredProducerFailure<ND, NE, L, D, E>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(formatter)
    }
}

impl<ND, NE, L, D, E: Error + 'static> Error
    for MainDmoSimulationDeferredProducerFailure<ND, NE, L, D, E>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}

/// Construction refusal for one exact main-DMO-bank prepared binding.
#[derive(Debug)]
pub(super) enum PreparedMainDmoBankSimulationCollectionError {
    ModelBatch(SimulationDmoBankModelRegistrationBatchError),
    CorpusAssetCountMismatch {
        actual: usize,
    },
    CorpusDmoInstanceCountMismatch {
        actual: usize,
    },
    Kind31LocatorOperationCountOverflow,
    CorpusKind31LocatorOperationCountMismatch {
        actual: usize,
    },
    CorpusAssetIdDuplicate {
        first_ordinal: usize,
        second_ordinal: usize,
    },
    CorpusAssetIdSequenceMismatch,
    PlanOwnerStorageAllocation {
        requested: usize,
    },
    PreparedCollection(SimulationCollectionPrepareError),
}

impl fmt::Display for PreparedMainDmoBankSimulationCollectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelBatch(source) => {
                write!(
                    formatter,
                    "could not prepare DMO bank model batch: {source}"
                )
            }
            Self::PreparedCollection(source) => {
                write!(
                    formatter,
                    "could not bind DMO bank collection views: {source}"
                )
            }
            Self::CorpusAssetCountMismatch { actual } => write!(
                formatter,
                "DMO bank binding retained {actual} assets instead of the pinned corpus count"
            ),
            Self::CorpusDmoInstanceCountMismatch { actual } => write!(
                formatter,
                "DMO bank binding retained {actual} kind-11 rows instead of the pinned corpus count"
            ),
            Self::Kind31LocatorOperationCountOverflow => formatter.write_str(
                "DMO bank binding kind-31 locator-operation count overflowed host usize",
            ),
            Self::CorpusKind31LocatorOperationCountMismatch { actual } => write!(
                formatter,
                "DMO bank binding retained {actual} kind-31 locator operations instead of the pinned zero count"
            ),
            Self::CorpusAssetIdDuplicate {
                first_ordinal,
                second_ordinal,
            } => write!(
                formatter,
                "DMO bank binding repeats one asset ID at ordinals {first_ordinal} and {second_ordinal}"
            ),
            Self::CorpusAssetIdSequenceMismatch => formatter
                .write_str("DMO bank binding asset-ID sequence does not match the pinned corpus"),
            Self::PlanOwnerStorageAllocation { requested } => write!(
                formatter,
                "could not retain {requested} DMO bank collection plan owners"
            ),
        }
    }
}

impl Error for PreparedMainDmoBankSimulationCollectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ModelBatch(source) => Some(source),
            Self::PreparedCollection(source) => Some(source),
            Self::CorpusAssetCountMismatch { .. }
            | Self::CorpusDmoInstanceCountMismatch { .. }
            | Self::Kind31LocatorOperationCountOverflow
            | Self::CorpusKind31LocatorOperationCountMismatch { .. }
            | Self::CorpusAssetIdDuplicate { .. }
            | Self::CorpusAssetIdSequenceMismatch
            | Self::PlanOwnerStorageAllocation { .. } => None,
        }
    }
}

/// Inseparable prepared collection and model-source batch for one exact
/// resident main DMO bank snapshot.
pub(super) struct PreparedMainDmoBankSimulationCollection {
    prepared: PreparedSimulationCollection,
    model_batch: SimulationDmoBankModelRegistrationBatch,
}

/// Sanitized reason one effect-free receiver-front acquisition was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankPublishedAcquisitionError {
    RegistryDetached,
    RegistryPoisoned,
    PublishedKindMismatch,
    MissingCollection,
    StaleOccurrence,
    WrongCollection,
    ResidentUnavailable,
    ResidentAssetSequenceMismatch,
    ModelBatch,
    CollectionBinding,
}

impl fmt::Display for MainDmoBankPublishedAcquisitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::RegistryDetached => "Simulation resident registry is detached",
            Self::RegistryPoisoned => "Simulation resident registry is poisoned",
            Self::PublishedKindMismatch => {
                "main DMO collection ID is occupied by another published asset kind"
            }
            Self::MissingCollection => "main DMO collection is not currently published",
            Self::StaleOccurrence => "main DMO resident occurrence is stale",
            Self::WrongCollection => "published occurrence is not the exact main DMO bank",
            Self::ResidentUnavailable => "main DMO resident storage is unavailable",
            Self::ResidentAssetSequenceMismatch => {
                "main DMO resident wrapper sequence no longer matches"
            }
            Self::ModelBatch => "main DMO model-source batch preparation failed",
            Self::CollectionBinding => "main DMO prepared collection binding failed",
        };
        formatter.write_str(message)
    }
}

impl Error for MainDmoBankPublishedAcquisitionError {}

/// Sanitized reason one deferred Arena front could not verify its current
/// full-ID winner against the exact owner retained at Setup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankPublishedArenaRevalidationError {
    PreparedArenaUnavailable,
    RegistryDetached,
    RegistryPoisoned,
    MissingArena,
    PublishedKindMismatch,
    PublishedAssetExpired,
    RetainedOwnerMismatch,
    RetainedPlanMismatch,
}

impl fmt::Display for MainDmoBankPublishedArenaRevalidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::PreparedArenaUnavailable => {
                "main DMO bank prepared Arena identity is unavailable"
            }
            Self::RegistryDetached => "Simulation resident registry is detached",
            Self::RegistryPoisoned => "Simulation resident registry is poisoned",
            Self::MissingArena => "main DMO bank Arena is not currently published",
            Self::PublishedKindMismatch => {
                "main DMO bank Arena ID is occupied by another published asset kind"
            }
            Self::PublishedAssetExpired => "main DMO bank published Arena owner is unavailable",
            Self::RetainedOwnerMismatch => {
                "main DMO bank Arena winner is not the owner retained at Setup"
            }
            Self::RetainedPlanMismatch => {
                "main DMO bank Arena winner does not retain the Setup plan"
            }
        };
        formatter.write_str(message)
    }
}

impl Error for MainDmoBankPublishedArenaRevalidationError {}

fn map_lookup_error(source: SimulationResidentLookupError) -> MainDmoBankPublishedAcquisitionError {
    match source {
        SimulationResidentLookupError::RegistryDetached => {
            MainDmoBankPublishedAcquisitionError::RegistryDetached
        }
        SimulationResidentLookupError::Poisoned => {
            MainDmoBankPublishedAcquisitionError::RegistryPoisoned
        }
        SimulationResidentLookupError::PublishedKindMismatch { .. } => {
            MainDmoBankPublishedAcquisitionError::PublishedKindMismatch
        }
    }
}

fn map_arena_lookup_error(
    source: SimulationResidentArenaLookupError<MainDmoBankPublishedArenaRevalidationError>,
) -> MainDmoBankPublishedArenaRevalidationError {
    match source {
        SimulationResidentArenaLookupError::RegistryDetached => {
            MainDmoBankPublishedArenaRevalidationError::RegistryDetached
        }
        SimulationResidentArenaLookupError::Poisoned => {
            MainDmoBankPublishedArenaRevalidationError::RegistryPoisoned
        }
        SimulationResidentArenaLookupError::PublishedKindMismatch { .. } => {
            MainDmoBankPublishedArenaRevalidationError::PublishedKindMismatch
        }
        SimulationResidentArenaLookupError::PublishedAssetExpired { .. } => {
            MainDmoBankPublishedArenaRevalidationError::PublishedAssetExpired
        }
        SimulationResidentArenaLookupError::Visitor(source) => source,
    }
}

fn map_snapshot_error(
    source: SimulationMainDmoBankResidentSnapshotError,
) -> MainDmoBankPublishedAcquisitionError {
    match source {
        SimulationMainDmoBankResidentSnapshotError::RegistryDetached => {
            MainDmoBankPublishedAcquisitionError::RegistryDetached
        }
        SimulationMainDmoBankResidentSnapshotError::RegistryPoisoned => {
            MainDmoBankPublishedAcquisitionError::RegistryPoisoned
        }
        SimulationMainDmoBankResidentSnapshotError::UnknownOccurrence => {
            MainDmoBankPublishedAcquisitionError::StaleOccurrence
        }
        SimulationMainDmoBankResidentSnapshotError::WrongCollection => {
            MainDmoBankPublishedAcquisitionError::WrongCollection
        }
        SimulationMainDmoBankResidentSnapshotError::Resident(_) => {
            MainDmoBankPublishedAcquisitionError::ResidentUnavailable
        }
        SimulationMainDmoBankResidentSnapshotError::ResidentAssetSequenceMismatch => {
            MainDmoBankPublishedAcquisitionError::ResidentAssetSequenceMismatch
        }
    }
}

fn map_binding_error(
    source: &PreparedMainDmoBankPublishedBindingError<'_>,
) -> MainDmoBankPublishedAcquisitionError {
    match source {
        PreparedMainDmoBankPublishedBindingError::ModelBatch(_) => {
            MainDmoBankPublishedAcquisitionError::ModelBatch
        }
        PreparedMainDmoBankPublishedBindingError::Collection(_) => {
            MainDmoBankPublishedAcquisitionError::CollectionBinding
        }
    }
}

/// Lazy receiver-front acquisition for one exact published main DMO bank.
///
/// Construction only retains the caller's non-owning lookup and the internally
/// fixed main-bank ID. It performs no lookup, resident mutation, callback, or
/// binding work. This is a safe-host owner used to place the already-recovered
/// lookup at a later receiver boundary; it is not a native object or lifetime
/// contract.
pub(super) struct MainDmoBankPublishedAcquisition {
    lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    id: CollectionId,
}

impl MainDmoBankPublishedAcquisition {
    #[must_use]
    pub(super) const fn new(lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>) -> Self {
        Self {
            lookup,
            id: CollectionId(crate::dmo_bank_processed_asset::DMO_BANK_COLLECTION_ID),
        }
    }

    /// Performs the explicit fresh lookup, exact occurrence snapshot, and
    /// effect-free binding preparation in that order.
    pub(super) fn try_resolve(
        self,
    ) -> Result<MainDmoBankPublishedBindingOwner, MainDmoBankPublishedAcquisitionFailure> {
        try_resolve_lookup(self.lookup, self.id)
    }
}

enum MainDmoBankPublishedAcquisitionRecovery {
    Lookup,
    Snapshot(SimulationMainDmoBankResidentSnapshotFailure),
    Binding(PreparedMainDmoBankPublishedBindingFailure),
}

/// Owning acquisition refusal. Retry authority remains sealed with the same
/// lookup and internally fixed ID; Debug and diagnostics reveal no occurrence,
/// snapshot, binding, retained owner, or record source.
pub(super) struct MainDmoBankPublishedAcquisitionFailure {
    error: MainDmoBankPublishedAcquisitionError,
    lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    id: CollectionId,
    recovery: MainDmoBankPublishedAcquisitionRecovery,
}

impl MainDmoBankPublishedAcquisitionFailure {
    #[must_use]
    pub(super) const fn error(&self) -> MainDmoBankPublishedAcquisitionError {
        self.error
    }

    /// Retries from the earliest still-owned boundary. An initial missing or
    /// lookup refusal performs another lookup; later refusals retain and reuse
    /// the exact occurrence/snapshot-derived authority instead of rebinding to
    /// a same-ID reload.
    pub(super) fn retry(
        self,
    ) -> Result<MainDmoBankPublishedBindingOwner, MainDmoBankPublishedAcquisitionFailure> {
        match self.recovery {
            MainDmoBankPublishedAcquisitionRecovery::Lookup => {
                try_resolve_lookup(self.lookup, self.id)
            }
            MainDmoBankPublishedAcquisitionRecovery::Snapshot(source) => {
                try_resolve_occurrence(self.lookup, self.id, source.into_occurrence())
            }
            MainDmoBankPublishedAcquisitionRecovery::Binding(source) => {
                try_resolve_binding_retry(self.lookup, self.id, source)
            }
        }
    }
}

impl fmt::Debug for MainDmoBankPublishedAcquisitionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankPublishedAcquisitionFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for MainDmoBankPublishedAcquisitionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for MainDmoBankPublishedAcquisitionFailure {}

fn try_resolve_lookup(
    lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    id: CollectionId,
) -> Result<MainDmoBankPublishedBindingOwner, MainDmoBankPublishedAcquisitionFailure> {
    let occurrence = match lookup.lookup(id) {
        Ok(Some(occurrence)) => occurrence,
        Ok(None) => {
            return Err(MainDmoBankPublishedAcquisitionFailure {
                error: MainDmoBankPublishedAcquisitionError::MissingCollection,
                lookup,
                id,
                recovery: MainDmoBankPublishedAcquisitionRecovery::Lookup,
            });
        }
        Err(source) => {
            return Err(MainDmoBankPublishedAcquisitionFailure {
                error: map_lookup_error(source),
                lookup,
                id,
                recovery: MainDmoBankPublishedAcquisitionRecovery::Lookup,
            });
        }
    };
    try_resolve_occurrence(lookup, id, occurrence)
}

fn try_resolve_occurrence(
    lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    id: CollectionId,
    occurrence: crate::simulation_resident_registry::SimulationResidentOccurrence<
        DmoBankProcessedArenaAsset,
    >,
) -> Result<MainDmoBankPublishedBindingOwner, MainDmoBankPublishedAcquisitionFailure> {
    let snapshot = match occurrence.try_snapshot_main_dmo_bank() {
        Ok(snapshot) => snapshot,
        Err(source) => {
            let error = map_snapshot_error(source.error());
            return Err(MainDmoBankPublishedAcquisitionFailure {
                error,
                lookup,
                id,
                recovery: MainDmoBankPublishedAcquisitionRecovery::Snapshot(source),
            });
        }
    };
    match PreparedMainDmoBankSimulationCollection::try_from_published_snapshot(snapshot) {
        Ok(binding) => Ok(MainDmoBankPublishedBindingOwner {
            lookup,
            id,
            binding,
        }),
        Err(source) => {
            let error = map_binding_error(&source.error());
            Err(MainDmoBankPublishedAcquisitionFailure {
                error,
                lookup,
                id,
                recovery: MainDmoBankPublishedAcquisitionRecovery::Binding(source),
            })
        }
    }
}

fn try_resolve_binding_retry(
    lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    id: CollectionId,
    source: PreparedMainDmoBankPublishedBindingFailure,
) -> Result<MainDmoBankPublishedBindingOwner, MainDmoBankPublishedAcquisitionFailure> {
    match source.retry() {
        Ok(binding) => Ok(MainDmoBankPublishedBindingOwner {
            lookup,
            id,
            binding,
        }),
        Err(source) => {
            let error = map_binding_error(&source.error());
            Err(MainDmoBankPublishedAcquisitionFailure {
                error,
                lookup,
                id,
                recovery: MainDmoBankPublishedAcquisitionRecovery::Binding(source),
            })
        }
    }
}

/// Exact resolved binding paired inseparably with its lookup and fixed ID.
/// No part is cloneable, debuggable, or independently accessible.
pub(super) struct MainDmoBankPublishedBindingOwner {
    lookup: SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    id: CollectionId,
    binding: PreparedMainDmoBankSimulationCollection,
}

impl MainDmoBankPublishedBindingOwner {
    /// Freshly verifies that the registry still publishes the same occurrence
    /// and exact wrapper-time sequence. This immutable pre-effect check leaves
    /// the owner intact on every refusal and gives no guarantee after return.
    pub(super) fn try_revalidate(&self) -> Result<(), MainDmoBankPublishedAcquisitionError> {
        let occurrence = self
            .lookup
            .lookup(self.id)
            .map_err(map_lookup_error)?
            .ok_or(MainDmoBankPublishedAcquisitionError::MissingCollection)?;
        self.binding
            .model_batch
            .try_revalidate_published_occurrence(&occurrence)
            .map_err(|source| match source {
                SimulationMainDmoBankResidentRevalidationError::DifferentOccurrence => {
                    MainDmoBankPublishedAcquisitionError::StaleOccurrence
                }
                SimulationMainDmoBankResidentRevalidationError::Snapshot(source) => {
                    map_snapshot_error(source)
                }
                SimulationMainDmoBankResidentRevalidationError::RetainedAssetSequenceMismatch => {
                    MainDmoBankPublishedAcquisitionError::ResidentAssetSequenceMismatch
                }
            })
    }

    /// Resolves one authored full Arena ID at its actual receiver front,
    /// verifies the exact outer owner and plan retained at Setup, then invokes
    /// the effect while the current registry winner remains scoped alive. The
    /// registry lock is released before the visitor runs. This point-in-time
    /// safe-host check provides no guarantee after the visitor returns.
    pub(super) fn try_with_revalidated_deferred_arena<R>(
        &self,
        ordinal: usize,
        effect: impl FnOnce() -> R,
    ) -> Result<R, MainDmoBankPublishedArenaRevalidationError> {
        let (asset_id, _) = self
            .binding
            .prepared
            .arena_identity(ordinal)
            .ok_or(MainDmoBankPublishedArenaRevalidationError::PreparedArenaUnavailable)?;
        let expected_plan = self
            .binding
            .prepared
            .arena_plan_owner(ordinal)
            .ok_or(MainDmoBankPublishedArenaRevalidationError::PreparedArenaUnavailable)?;
        self.lookup
            .arena_lookup()
            .try_with_asset(CollectionId(asset_id), |current| {
                if !self
                    .binding
                    .model_batch
                    .retained_arena_owner_matches(ordinal, current)
                {
                    return Err(MainDmoBankPublishedArenaRevalidationError::RetainedOwnerMismatch);
                }
                if !std::ptr::eq(expected_plan.as_ref(), current.plan()) {
                    return Err(MainDmoBankPublishedArenaRevalidationError::RetainedPlanMismatch);
                }
                Ok(effect())
            })
            .map_err(map_arena_lookup_error)?
            .ok_or(MainDmoBankPublishedArenaRevalidationError::MissingArena)
    }
}

/// Owning refusal while deriving the exact main-bank binding from one
/// published occurrence. Retrying never requires an occurrence, asset owner,
/// or model batch to escape this subtree as a separate capability.
pub(super) struct PreparedMainDmoBankPublishedBindingFailure {
    recovery: PreparedMainDmoBankPublishedBindingRecovery,
}

enum PreparedMainDmoBankPublishedBindingRecovery {
    ModelBatch(SimulationMainDmoBankResidentSnapshotConsumeFailure),
    Collection {
        error: PreparedMainDmoBankSimulationCollectionError,
        model_batch: SimulationDmoBankModelRegistrationBatch,
    },
}

pub(super) enum PreparedMainDmoBankPublishedBindingError<'a> {
    ModelBatch(&'a SimulationDmoBankModelRegistrationBatchError),
    Collection(&'a PreparedMainDmoBankSimulationCollectionError),
}

impl fmt::Debug for PreparedMainDmoBankPublishedBindingFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.recovery {
            PreparedMainDmoBankPublishedBindingRecovery::ModelBatch(source) => {
                formatter.debug_tuple("ModelBatch").field(source).finish()
            }
            PreparedMainDmoBankPublishedBindingRecovery::Collection { error, .. } => formatter
                .debug_struct("Collection")
                .field("error", error)
                .finish_non_exhaustive(),
        }
    }
}

impl PreparedMainDmoBankPublishedBindingFailure {
    /// Borrows only the typed refusal, never retained recovery authority.
    pub(super) fn error(&self) -> PreparedMainDmoBankPublishedBindingError<'_> {
        match &self.recovery {
            PreparedMainDmoBankPublishedBindingRecovery::ModelBatch(source) => {
                PreparedMainDmoBankPublishedBindingError::ModelBatch(source.error())
            }
            PreparedMainDmoBankPublishedBindingRecovery::Collection { error, .. } => {
                PreparedMainDmoBankPublishedBindingError::Collection(error)
            }
        }
    }

    /// Retries the same exact occurrence ownership without exposing either
    /// recovery representation.
    pub(super) fn retry(self) -> Result<PreparedMainDmoBankSimulationCollection, Self> {
        match self.recovery {
            PreparedMainDmoBankPublishedBindingRecovery::ModelBatch(source) => {
                PreparedMainDmoBankSimulationCollection::try_from_published_snapshot(
                    source.into_snapshot(),
                )
            }
            PreparedMainDmoBankPublishedBindingRecovery::Collection { model_batch, .. } => {
                PreparedMainDmoBankSimulationCollection::try_from_model_batch(model_batch).map_err(
                    |(error, model_batch)| Self {
                        recovery: PreparedMainDmoBankPublishedBindingRecovery::Collection {
                            error,
                            model_batch,
                        },
                    },
                )
            }
        }
    }
}

impl PreparedMainDmoBankSimulationCollection {
    /// Consumes one opaque exact published-occurrence snapshot. Every
    /// construction refusal retains all retry authority inside the returned
    /// owning failure.
    pub(super) fn try_from_published_snapshot(
        snapshot: SimulationMainDmoBankResidentSnapshot,
    ) -> Result<Self, PreparedMainDmoBankPublishedBindingFailure> {
        let model_batch =
            SimulationDmoBankModelRegistrationBatch::try_from_published_snapshot(snapshot)
                .map_err(|source| PreparedMainDmoBankPublishedBindingFailure {
                    recovery: PreparedMainDmoBankPublishedBindingRecovery::ModelBatch(source),
                })?;
        Self::try_from_model_batch(model_batch).map_err(|(error, model_batch)| {
            PreparedMainDmoBankPublishedBindingFailure {
                recovery: PreparedMainDmoBankPublishedBindingRecovery::Collection {
                    error,
                    model_batch,
                },
            }
        })
    }

    /// Builds the effect-free model batch before deriving collection views
    /// from that batch's retained exact arena owners.
    #[cfg(test)]
    pub(super) fn try_new(
        collection: &OwnedResidentCollection<DmoBankProcessedArenaAsset>,
    ) -> Result<Self, PreparedMainDmoBankSimulationCollectionError> {
        let model_batch = SimulationDmoBankModelRegistrationBatch::try_new(collection)
            .map_err(PreparedMainDmoBankSimulationCollectionError::ModelBatch)?;
        Self::try_from_model_batch(model_batch).map_err(|(error, _model_batch)| error)
    }

    fn try_from_model_batch(
        model_batch: SimulationDmoBankModelRegistrationBatch,
    ) -> Result<
        Self,
        (
            PreparedMainDmoBankSimulationCollectionError,
            SimulationDmoBankModelRegistrationBatch,
        ),
    > {
        let prepared = match model_batch.try_prepare_collection() {
            Ok(prepared) => prepared,
            Err(source) => {
                let error = match source {
                SimulationDmoBankPreparedCollectionError::CorpusAssetCountMismatch { actual } => {
                    PreparedMainDmoBankSimulationCollectionError::CorpusAssetCountMismatch {
                        actual,
                    }
                }
                SimulationDmoBankPreparedCollectionError::CorpusDmoInstanceCountMismatch {
                    actual,
                } => PreparedMainDmoBankSimulationCollectionError::CorpusDmoInstanceCountMismatch {
                    actual,
                },
                SimulationDmoBankPreparedCollectionError::CorpusAssetIdDuplicate {
                    first_ordinal,
                    second_ordinal,
                } => PreparedMainDmoBankSimulationCollectionError::CorpusAssetIdDuplicate {
                    first_ordinal,
                    second_ordinal,
                },
                SimulationDmoBankPreparedCollectionError::CorpusAssetIdSequenceMismatch => {
                    PreparedMainDmoBankSimulationCollectionError::CorpusAssetIdSequenceMismatch
                }
                SimulationDmoBankPreparedCollectionError::PlanOwnerStorageAllocation {
                    requested,
                } => PreparedMainDmoBankSimulationCollectionError::PlanOwnerStorageAllocation {
                    requested,
                },
                SimulationDmoBankPreparedCollectionError::Collection(source) => {
                    PreparedMainDmoBankSimulationCollectionError::PreparedCollection(source)
                }
                };
                return Err((error, model_batch));
            }
        };
        let Some(kind31_locator_operation_count) = prepared.checked_dmo_locator_operation_count()
        else {
            return Err((
                PreparedMainDmoBankSimulationCollectionError::Kind31LocatorOperationCountOverflow,
                model_batch,
            ));
        };
        if kind31_locator_operation_count != DMO_BANK_KIND_31_LOCATOR_OPERATION_COUNT {
            return Err((
                PreparedMainDmoBankSimulationCollectionError::CorpusKind31LocatorOperationCountMismatch {
                    actual: kind31_locator_operation_count,
                },
                model_batch,
            ));
        }
        Ok(Self {
            prepared,
            model_batch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::coordinator::*;
    use super::deferred_producer::*;
    use super::marker_fifo::*;
    use super::notification_receiver::*;
    use super::*;
    use crate::{
        collection_load_pump::LoadedAssetCollection,
        collection_resident_streamer::{
            CollectionId, ResidentCollectionRegistry, ResidentCollectionStorage,
        },
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        deferred_collection_reg_manager::{
            DeferredCollectionRegActive, DeferredCollectionRegActiveEffect,
            DeferredCollectionRegManager, DeferredCollectionRegManagerUpdate,
            DeferredCollectionRegRow, DeferredCollectionRegSetupEffect, DeferredCollectionRegType,
        },
        dmo_bank_processed_asset::{
            DMO_BANK_ASSET_COUNT, DMO_BANK_COLLECTION_ID, DMO_BANK_PARENT_ID,
        },
        dmo_hull_acknowledgement::tests::MainDmoDregIntegrationHarness,
        dmo_hull_acknowledgement::{
            MainDmoHullAcknowledgementCoordinator, MainDmoHullCallbackRejectionKind,
            MainDmoHullDeregistrationRequest, MainDmoHullId, MainDmoHullRegistrationRequest,
            MainDmoHullSubsystemPort, MainDmoPresentationDeregistrationCallback,
            MainDmoPresentationRegistrationCallback, MainDmoSimulationDeregistrationCallback,
            MainDmoSimulationRegistrationCallback,
        },
        grind::GrindAsset,
        owned_resident_collection::{OwnedResidentCollectionError, OwnedResidentCollectionStorage},
        presentation_dmo_arena_adapter::{
            PreparedMainDmoBankPresentationArena, PreparedMainDmoBankPresentationCollection,
        },
        presentation_dmo_deferred_producer::{
            MainDmoPresentationDeferredMarkerKind, MainDmoPresentationDeferredProducer,
            MainDmoPresentationDeferredProducerUpdate,
        },
        simulation_collection_adapter::{
            SimulationCollectionDeferredFinishValidationError,
            SimulationCollectionDeferredStepError, SimulationCollectionDeregistrationError,
            SimulationCollectionDmoPass, SimulationCollectionLocationPass,
            SimulationCollectionNestedFailureKind, SimulationCollectionPassDeregistrationFailure,
            SimulationCollectionRegistrationError,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_deferred_batch_policy::DeferredCollectionBatchControls,
        simulation_dmo_arena_adapter::PreparedSimulationDmoArena,
        simulation_dmo_arena_plan::SimulationDmoArenaPlan,
        simulation_dmo_assembly_part_session::{
            SimulationDmoBankModelDeregisterPassError, SimulationDmoBankModelRegisterPassError,
            SimulationDmoBankModelRegistrationBatchError,
            SimulationDmoBankModelRegistrationPassError, SimulationDmoLocatorRegistryInitError,
            SimulationDmoModelRegistrationSourceError, SimulationDmoModelRegistryInitError,
            SimulationDmoRegistrationManagerInitError, SimulationDmoRegistrationRuntimeInitError,
        },
        simulation_environment_unlock_router::EnvironmentUnlockRouterError,
        simulation_normal_arena_adapter::{
            SimulationNormalArenaRegistrationAdapter, SimulationNormalEnvironmentDispatch,
        },
        simulation_normal_arena_plan::{
            SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRecordHandle,
            SimulationRecordSourceKind,
        },
        simulation_pointer_arena_adapter::SimulationPointerArenaDispatch,
        simulation_registry_arena_adapter::SimulationRegistryArenaDispatch,
        simulation_resident_registry::SimulationResidentRegistry,
        simulation_static_arena_adapter::SimulationStaticGrindDispatch,
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        static_ground_pool::StaticEnvironmentSource,
        world::CollisionArena,
    };
    use skate2_formats::{
        pegasus::{DMO_DATA_TYPE_ID, ObjectId, TABLE_OF_CONTENTS_TYPE_ID},
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::{ArenaResources, BaseResourceDescriptor},
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::{convert::Infallible, ops::Range};

    type TestStorage = OwnedResidentCollectionStorage<
        DmoBankProcessedArenaAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;
    type TestResidentRegistry = SimulationResidentRegistry<
        DmoBankProcessedArenaAsset,
        fn(
            crate::simulation_resident_registry::SimulationResidentRegistryError,
        ) -> crate::simulation_resident_registry::SimulationResidentRegistryError,
        crate::simulation_resident_registry::SimulationResidentRegistryError,
    >;

    struct CallbackDropProbe(Arc<AtomicUsize>);

    impl Drop for CallbackDropProbe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    struct MarkerHullPort;

    impl MainDmoHullSubsystemPort for MarkerHullPort {
        fn register_simulation(
            &mut self,
            _: MainDmoHullRegistrationRequest,
            _: MainDmoSimulationRegistrationCallback,
        ) {
        }

        fn register_presentation(
            &mut self,
            _: MainDmoHullRegistrationRequest,
            _: MainDmoPresentationRegistrationCallback,
        ) {
        }

        fn deregister_simulation(
            &mut self,
            _: MainDmoHullDeregistrationRequest,
            _: MainDmoSimulationDeregistrationCallback,
        ) {
        }

        fn deregister_presentation(
            &mut self,
            _: MainDmoHullDeregistrationRequest,
            _: MainDmoPresentationDeregistrationCallback,
        ) {
        }
    }

    fn marker_hull() -> MainDmoHullAcknowledgementCoordinator<MarkerHullPort> {
        MainDmoHullAcknowledgementCoordinator::new(MarkerHullPort, MainDmoHullId(0x1234)).0
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum CoordinatorEvent {
        Add(u64),
        Remove(u64),
        LocationRegister(usize),
        DmoRegister(usize),
        DmoDeregister(usize),
        LocationDeregister(usize),
    }

    #[derive(Clone, Default)]
    struct CoordinatorDispatch {
        events: Arc<Mutex<Vec<CoordinatorEvent>>>,
    }

    impl SimulationStaticGrindDispatch for CoordinatorDispatch {
        fn add_spline_data(&mut self, _asset: &GrindAsset) {}

        fn remove_spline_data(&mut self, _asset_id: u64) {}
    }

    impl SimulationPointerArenaDispatch for CoordinatorDispatch {
        fn add_dmo_model(&mut self, asset_id: u64) {
            self.events
                .lock()
                .unwrap()
                .push(CoordinatorEvent::Add(asset_id));
        }

        fn register_dmo_model(&mut self, _source: &SimulationRecordHandle, _asset_id: u64) {
            panic!("the exact controller must replace the legacy DMO callback")
        }

        fn remove_dmo_model(&mut self, asset_id: u64) {
            self.events
                .lock()
                .unwrap()
                .push(CoordinatorEvent::Remove(asset_id));
        }

        fn deregister_dmo_model(&mut self, _source: &SimulationRecordHandle, _asset_id: u64) {
            panic!("the exact controller must replace the legacy DMO callback")
        }

        fn add_trigger_volume(&mut self, _source: &SimulationRecordHandle) {}

        fn remove_trigger_volume(&mut self, _source: &SimulationRecordHandle) {}
    }

    impl SimulationRegistryArenaDispatch for CoordinatorDispatch {
        fn register_streamed_tile(&mut self, _asset_id: u64) {}
        fn unregister_streamed_tile(&mut self, _asset_id: u64) {}
        fn add_nav_mesh_data(&mut self, _asset_id: u64) {}
        fn remove_nav_mesh_data(&mut self, _asset_id: u64) {}
        fn add_rain_data(&mut self, _asset_id: u64) {}
        fn remove_rain_data(&mut self, _asset_id: u64) {}
        fn add_ai_path_asset(&mut self, _asset_id: u64) {}
        fn remove_ai_path_asset(&mut self, _asset_id: u64) {}
        fn register_waypoint_data(&mut self, _asset_id: u64, _streamed: bool) {}
        fn deregister_waypoint_data(&mut self, _asset_id: u64, _streamed: bool) {}
        fn add_spatial_map_asset(&mut self, _asset_id: u64) {}
        fn remove_spatial_map_asset(&mut self, _asset_id: u64) {}
    }

    #[derive(Default)]
    struct CoordinatorEnvironment;

    impl SimulationNormalEnvironmentDispatch for CoordinatorEnvironment {
        fn register_environment(
            &mut self,
            _source: &Arc<StaticEnvironmentSource>,
        ) -> Result<(), EnvironmentUnlockRouterError> {
            Ok(())
        }

        fn deregister_environment(&mut self, _source: &Arc<StaticEnvironmentSource>) {}
    }

    struct CoordinatorLocation {
        events: Arc<Mutex<Vec<CoordinatorEvent>>>,
        next_register: usize,
        next_deregister: usize,
    }

    impl SimulationCollectionLocationPass for CoordinatorLocation {
        type Registration = usize;

        fn register_location(
            &mut self,
            _prepared: &crate::location_arena_adapter::PreparedLocationArena,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            let ordinal = self.next_register;
            self.next_register += 1;
            self.events
                .lock()
                .unwrap()
                .push(CoordinatorEvent::LocationRegister(ordinal));
            Ok(ordinal)
        }

        fn deregister_location(
            &mut self,
            registration: Self::Registration,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            let ordinal = self.next_deregister;
            self.next_deregister += 1;
            assert_eq!(registration, ordinal);
            self.events
                .lock()
                .unwrap()
                .push(CoordinatorEvent::LocationDeregister(ordinal));
            Ok(())
        }
    }

    struct CoordinatorDmo {
        events: Arc<Mutex<Vec<CoordinatorEvent>>>,
        next_register: usize,
        next_deregister: usize,
    }

    impl SimulationCollectionDmoPass for CoordinatorDmo {
        type Registration = usize;

        fn register_dmo(
            &mut self,
            _prepared: &PreparedSimulationDmoArena,
            userdata_raw: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            assert_eq!(userdata_raw, MAIN_DMO_BANK_USERDATA_RAW);
            let ordinal = self.next_register;
            self.next_register += 1;
            self.events
                .lock()
                .unwrap()
                .push(CoordinatorEvent::DmoRegister(ordinal));
            Ok(ordinal)
        }

        fn deregister_dmo(
            &mut self,
            registration: Self::Registration,
            userdata_raw: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            assert_eq!(userdata_raw, MAIN_DMO_BANK_USERDATA_RAW);
            let ordinal = self.next_deregister;
            self.next_deregister += 1;
            assert_eq!(registration, ordinal);
            self.events
                .lock()
                .unwrap()
                .push(CoordinatorEvent::DmoDeregister(ordinal));
            Ok(())
        }
    }

    const CORPUS_STREAM_SIZE: u32 = 0x2f_3180;

    const CORPUS_ASSET_IDS: [u64; 125] = [
        0x91d7_2407_5ae4_4972,
        0x481c_e9c9_3689_053a,
        0x032b_dc34_e46b_1e49,
        0xa10b_857f_1d05_fa70,
        0x1eb5_6dd9_bd90_aa1b,
        0x9497_78a9_f3fa_a7ab,
        0xc02a_26bc_abe6_b236,
        0x4d96_cee4_4cdc_d6aa,
        0xf616_360f_ba85_7ca0,
        0x87cf_1be7_5daf_7ad3,
        0x1f79_43be_bbf8_7c4c,
        0x2d79_8ac5_a1f8_11dd,
        0x8e20_9aad_0b22_d463,
        0xa98e_9af5_0714_8c55,
        0x1a59_b04a_3a8d_bf8f,
        0x1141_752b_f78b_cf16,
        0x3614_5f36_2ec4_e0b8,
        0x9fed_5ac5_7b24_304e,
        0x5f69_ecf6_fa27_2f32,
        0xff73_3a73_9f43_b480,
        0x02e9_1e77_a92b_52e1,
        0x0b18_4a55_b238_5b58,
        0x8cbb_cb34_9ccd_6556,
        0x6aa9_e7ff_e421_f7cf,
        0x9903_98f8_eaf7_f7e3,
        0x2c5a_b382_ea93_cc73,
        0xbde6_c556_d47b_8b2c,
        0xbf90_da2b_b628_f28a,
        0x43ab_e52b_f152_a143,
        0x54f1_f289_19d4_efc1,
        0xae02_cdaa_1991_fc2a,
        0x3293_1179_a857_b7f9,
        0x24cc_24bc_8c5d_70df,
        0x0aef_9abd_fcf2_d0f4,
        0xb9f5_9db5_be14_2591,
        0x7bc3_6dee_9732_963e,
        0xb51e_6e2b_a194_6383,
        0x830a_7260_e870_1109,
        0xc64c_e327_ca60_b757,
        0x8bd2_d7d0_536f_53de,
        0x5422_3877_1eca_0fb1,
        0xd4a0_ac81_742f_ccc7,
        0x61a9_49e0_f0c8_640e,
        0x112b_2338_5b0b_6621,
        0xf7a5_2c39_b8bc_9460,
        0x1b05_2c07_29d5_1845,
        0xc979_98d9_cebb_7807,
        0x96fa_000b_532a_c104,
        0xae64_6a36_7c57_7c3c,
        0x0a26_a1c5_8b81_2479,
        0x4e38_d302_c4b4_079a,
        0x8e4e_294a_ad6e_8e1d,
        0xf962_be13_d72e_ff1b,
        0xa3a0_716f_363d_13bc,
        0xa306_4c3c_68ff_8ceb,
        0xc8b7_c29e_e3f5_5ab1,
        0x21e9_74a3_1e85_25ba,
        0x850b_df6f_2df0_93cd,
        0x7534_9dfe_ed84_9633,
        0x2407_6e7e_af2d_36c4,
        0x4936_36a9_395b_17c7,
        0x43df_b4a4_762e_cbe4,
        0x1f52_1747_55e9_51e7,
        0x8f47_95d3_436f_8992,
        0x69a2_5c79_82ef_08d1,
        0x05fe_19de_e8c7_7534,
        0x8097_0294_7d70_a7a2,
        0x149a_a6c3_4df2_045b,
        0x4974_a950_df7c_02a9,
        0xc1c0_5417_3f8a_572f,
        0x6fa9_a5e7_c8a5_077b,
        0x2b63_a9ec_4e76_5fb9,
        0x2a7c_ef92_763d_b245,
        0x0515_bc8f_9dc3_901e,
        0xa9af_c6d8_f8c4_fe57,
        0x41b4_96cf_936d_06d3,
        0x588a_02f4_35d0_8d44,
        0x3132_d9e2_50ee_95ab,
        0x5229_748f_f6d0_868c,
        0x3955_148b_1984_71d2,
        0x8778_7321_88ca_150b,
        0x046f_1980_105c_8188,
        0x4567_ad3c_9415_3701,
        0xe79f_67d4_05dc_c475,
        0xf66e_bf7d_3798_347c,
        0xd9e4_7543_7911_afc2,
        0x2f7b_5f74_14d1_1f94,
        0x50d4_072f_0f78_4db6,
        0xaab9_13f2_bfc1_32a9,
        0x2541_591c_578c_432b,
        0x252a_6005_5870_6ffd,
        0x3101_82cc_2944_db00,
        0x8d20_1285_3cf1_37c3,
        0xc20c_449b_9cf0_7cec,
        0x0e7a_e1bc_affb_0cef,
        0x3e9b_42c4_a756_0a76,
        0x9fac_1750_564e_e669,
        0x2141_b8dc_33c7_fc68,
        0x3e4f_f528_0a43_72eb,
        0xaa3c_fb39_0d17_2232,
        0x277f_b02f_ae54_3e84,
        0xdebe_8327_1fdd_851a,
        0x51d7_663e_9988_7f74,
        0x00dd_6936_5aa9_d411,
        0xc2ab_396f_33c8_44be,
        0x5c72_4b52_5853_4fa7,
        0x77da_c98f_14b0_ac6e,
        0x38c6_bf89_9cd2_6581,
        0x8c81_1b78_5559_f7a4,
        0x67f0_f6e8_97f9_af68,
        0xb91a_ea06_27b6_9d9a,
        0x214f_f499_3824_4e16,
        0xedec_8964_4b72_a709,
        0x6839_3d74_6aa6_2291,
        0xb933_3a7c_a984_cdf4,
        0x461a_5765_2fd9_d39a,
        0x278a_14ec_ae0d_880b,
        0xf196_31ce_8e05_b68e,
        0x0d28_4010_ae19_017e,
        0xfa92_a6c8_4dd6_b0e0,
        0x98fa_f4f6_b5a0_f4a3,
        0x46cd_d92b_76de_72a8,
        0xc2c6_990e_cdc6_ce2b,
        0xee59_4721_85b2_d8b6,
        0xd309_6f13_cf1c_0aab,
    ];

    fn identity_error(error: OwnedResidentCollectionError) -> OwnedResidentCollectionError {
        error
    }

    fn identity_registry_error(
        error: crate::simulation_resident_registry::SimulationResidentRegistryError,
    ) -> crate::simulation_resident_registry::SimulationResidentRegistryError {
        error
    }

    fn dmo_record(primary_asset_id: u64, local_guid: u64) -> Vec<u8> {
        let mut record = vec![0; 0xa0];
        record[0x60..0x68].copy_from_slice(&primary_asset_id.to_be_bytes());
        record[0x68..0x70].copy_from_slice(&local_guid.to_be_bytes());
        record
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn two_locator_dmo_data(guid: u64, local_guid: u64, instance_guid: u64) -> Vec<u8> {
        const DMOS_OFFSET: usize = 20;
        const RECORD_COUNT: usize = 2;
        const RECORD_END: usize = DMOS_OFFSET + 128 * RECORD_COUNT;
        let mut bytes = vec![0; RECORD_END + 3];
        put_u32(&mut bytes, 0, DMO_DATA_TYPE_ID);
        put_u32(&mut bytes, 4, u32::try_from(RECORD_COUNT).unwrap());
        put_u32(&mut bytes, 12, u32::try_from(DMOS_OFFSET).unwrap());
        put_u32(&mut bytes, 16, u32::try_from(RECORD_END).unwrap());
        for record_index in 0..RECORD_COUNT {
            let record_offset = DMOS_OFFSET + 128 * record_index;
            put_u64(&mut bytes, record_offset + 96, guid);
            put_u64(&mut bytes, record_offset + 104, local_guid);
            put_u64(&mut bytes, record_offset + 112, instance_guid);
            put_u32(
                &mut bytes,
                record_offset + 124,
                u32::try_from(RECORD_END).unwrap(),
            );
        }
        bytes[RECORD_END] = b'x';
        bytes
    }

    fn one_dmo_toc(asset_id: u64, object: ObjectId) -> Vec<u8> {
        const NAME_OFFSET: usize = 20 + 24;
        let mut bytes = vec![0; NAME_OFFSET + 1];
        put_u32(&mut bytes, 0, 1);
        put_u32(&mut bytes, 4, 20);
        put_u32(&mut bytes, 8, 44);
        put_u32(&mut bytes, 16, 44);
        put_u32(&mut bytes, 20, 44);
        put_u64(&mut bytes, 28, asset_id);
        put_u32(&mut bytes, 36, DMO_DATA_TYPE_ID);
        put_u32(&mut bytes, 40, object.0);
        bytes
    }

    fn append_dictionary_object(
        resource: &mut Vec<u8>,
        dictionary: &mut Vec<DictionaryEntry>,
        type_id: u32,
        object: &[u8],
    ) -> ObjectId {
        let index = dictionary.len();
        let start = resource.len();
        resource.extend_from_slice(object);
        let end = resource.len();
        dictionary.push(DictionaryEntry {
            resource_index: 0,
            object_offset: u32::try_from(start).unwrap(),
            relocation_offset: 0,
            size: u32::try_from(object.len()).unwrap(),
            alignment: 1,
            type_index: 0,
            type_id,
            object_range: Range { start, end },
        });
        ObjectId(u32::try_from(index).unwrap())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the exact test owner keeps the arena resources and record span coherent"
    )]
    fn asset(
        decoded_asset_id: u64,
        records: &[(u64, u64, u64)],
    ) -> (DmoBankProcessedArenaAsset, Arc<SimulationNormalArenaPlan>) {
        asset_with_kind31(decoded_asset_id, records, None)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the exact test owner keeps the arena resources and record span coherent"
    )]
    fn asset_with_kind31(
        decoded_asset_id: u64,
        records: &[(u64, u64, u64)],
        kind31: Option<(u64, u64, u64, u64)>,
    ) -> (DmoBankProcessedArenaAsset, Arc<SimulationNormalArenaPlan>) {
        let mut resource_zero = Vec::new();
        let mut dictionary = Vec::new();
        let mut operations = Vec::new();
        let mut sources = Vec::new();
        for &(primary_asset_id, operation_local_guid, record_local_guid) in records {
            let record = dmo_record(primary_asset_id, record_local_guid);
            let start = resource_zero.len();
            resource_zero.extend_from_slice(&record);
            operations.push(SimulationNormalOperation::DmoInstance {
                asset_id: primary_asset_id,
                local_guid: operation_local_guid,
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_record_offset: u32::try_from(start).unwrap(),
                raw_object_id: 0,
            });
            sources.push(Some((
                SimulationRecordSourceKind::DmoInstance,
                0,
                start,
                record.len(),
            )));
        }
        if let Some((asset_id, guid, local_guid, instance_guid)) = kind31 {
            let dmo = append_dictionary_object(
                &mut resource_zero,
                &mut dictionary,
                DMO_DATA_TYPE_ID,
                &two_locator_dmo_data(guid, local_guid, instance_guid),
            );
            append_dictionary_object(
                &mut resource_zero,
                &mut dictionary,
                TABLE_OF_CONTENTS_TYPE_ID,
                &one_dmo_toc(asset_id, dmo),
            );
        }
        let resources = [
            resource_zero,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let resource_descriptor = std::array::from_fn(|index| ResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
        let asset_resource_descriptors = std::array::from_fn(|index| BaseResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
        let collision = SimulationCollisionAsset {
            decoded: DecodedArenaAsset {
                asset_id: decoded_asset_id,
                asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
                asset_resource_descriptors,
                resources: ArenaResources { resources },
                arena: Arena {
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
                    id: 0x1234_5678,
                    entry_count: u32::try_from(dictionary.len()).unwrap(),
                    used_entry_count: u32::try_from(dictionary.len()).unwrap(),
                    alignment: 0x10,
                    virtual_size: resource_descriptor[0].size,
                    dictionary_offset: 0,
                    sections_offset: 0,
                    base_offset: 0,
                    unfix_context_offset: 0,
                    fix_context_offset: 0,
                    resource_descriptor,
                    resources_used: resource_descriptor,
                    resource_offsets: [u32::MAX; 5],
                    arena_group_offset: 0,
                    dictionary,
                },
            },
            collision: CollisionArena {
                id: 0x1234_5678,
                grind_assets: Vec::new(),
                aggregate_roots: Vec::new(),
                primitive_sets: Vec::new(),
                simple_primitive_sets: Vec::new(),
                meshes: Vec::new(),
                collision_models: Vec::new(),
                instances: Vec::new(),
            },
        };
        let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
            collision,
            Vec::new(),
        ));
        let plan = Arc::new(
            SimulationNormalArenaPlan::from_parts_with_record_sources_for_test(
                static_plan,
                operations,
                sources,
            ),
        );
        (
            DmoBankProcessedArenaAsset::from_parts_for_test(Arc::clone(&plan), Vec::new()),
            plan,
        )
    }

    fn collection(
        assets: Vec<DmoBankProcessedArenaAsset>,
    ) -> (
        TestStorage,
        OwnedResidentCollection<DmoBankProcessedArenaAsset>,
    ) {
        let mut storage = TestStorage::new(identity_error);
        let collection = storage
            .create_empty(i32::try_from(assets.len()).unwrap())
            .unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        for asset in assets {
            writer.add_asset(asset);
        }
        drop(writer);
        (storage, collection)
    }

    fn published_snapshot(
        collection: &OwnedResidentCollection<DmoBankProcessedArenaAsset>,
    ) -> (
        TestResidentRegistry,
        crate::simulation_resident_registry::SimulationResidentWrapper<DmoBankProcessedArenaAsset>,
        crate::simulation_resident_registry::SimulationResidentToken,
        SimulationMainDmoBankResidentSnapshot,
    ) {
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let snapshot = lookup
            .lookup(id)
            .unwrap()
            .unwrap()
            .try_snapshot_main_dmo_bank()
            .unwrap();
        (registry, wrapper, token, snapshot)
    }

    type TestCoordinator = MainDmoBankCollectionCoordinator<
        CoordinatorDispatch,
        CoordinatorEnvironment,
        CoordinatorLocation,
        CoordinatorDmo,
    >;

    type TestImmediatePending = MainDmoBankImmediatePendingFifo<
        CoordinatorDispatch,
        CoordinatorEnvironment,
        CoordinatorLocation,
        CoordinatorDmo,
    >;

    type TestDeferredPending = MainDmoBankDeferredPendingSetup<
        CoordinatorDispatch,
        CoordinatorEnvironment,
        CoordinatorLocation,
        CoordinatorDmo,
    >;

    type TestDenseDeferredProducer = MainDmoBankDenseDeferredProducer<
        CoordinatorDispatch,
        CoordinatorEnvironment,
        CoordinatorLocation,
        CoordinatorDmo,
    >;

    type TestPublicDeferredProducer = MainDmoSimulationDeferredProducer<
        CoordinatorDispatch,
        CoordinatorEnvironment,
        CoordinatorLocation,
        CoordinatorDmo,
    >;

    enum TestMixedDeferredPayload {
        Simulation(Box<Option<TestDenseDeferredProducer>>),
        Presentation(Box<Option<MainDmoPresentationDeferredProducer<()>>>),
    }

    type TestMixedDeferredManager = DeferredCollectionRegManager<TestMixedDeferredPayload>;

    type TestNotificationReceiver<RC, DC> = MainDmoBankNotificationReceiver<
        CoordinatorDispatch,
        CoordinatorEnvironment,
        CoordinatorLocation,
        CoordinatorDmo,
        RC,
        DC,
    >;

    fn coordinator(
        binding: PreparedMainDmoBankSimulationCollection,
        events: &Arc<Mutex<Vec<CoordinatorEvent>>>,
    ) -> TestCoordinator {
        MainDmoBankCollectionCoordinator::try_new_synthetic(
            binding,
            SimulationNormalArenaRegistrationAdapter::with_environment(
                CoordinatorDispatch {
                    events: Arc::clone(events),
                },
                CoordinatorEnvironment,
            ),
            CoordinatorLocation {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            CoordinatorDmo {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
        )
        .unwrap()
    }

    fn immediate_pending(
        lookup: crate::simulation_resident_registry::SimulationResidentLookup<
            DmoBankProcessedArenaAsset,
        >,
        events: &Arc<Mutex<Vec<CoordinatorEvent>>>,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> TestImmediatePending {
        MainDmoBankImmediatePendingFifo::try_new(
            lookup,
            SimulationNormalArenaRegistrationAdapter::with_environment(
                CoordinatorDispatch {
                    events: Arc::clone(events),
                },
                CoordinatorEnvironment,
            ),
            CoordinatorLocation {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            CoordinatorDmo {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            callback,
        )
        .unwrap()
    }

    fn immediate_pending_with_runtime_failure(
        lookup: crate::simulation_resident_registry::SimulationResidentLookup<
            DmoBankProcessedArenaAsset,
        >,
        events: &Arc<Mutex<Vec<CoordinatorEvent>>>,
        callback: MainDmoSimulationRegistrationCallback,
        source: SimulationDmoRegistrationRuntimeInitError,
    ) -> TestImmediatePending {
        MainDmoBankImmediatePendingFifo::new_with_runtime_failure_for_test(
            lookup,
            SimulationNormalArenaRegistrationAdapter::with_environment(
                CoordinatorDispatch {
                    events: Arc::clone(events),
                },
                CoordinatorEnvironment,
            ),
            CoordinatorLocation {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            CoordinatorDmo {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            callback,
            source,
        )
        .unwrap()
    }

    fn deferred_pending(
        lookup: crate::simulation_resident_registry::SimulationResidentLookup<
            DmoBankProcessedArenaAsset,
        >,
        events: &Arc<Mutex<Vec<CoordinatorEvent>>>,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> TestDeferredPending {
        MainDmoBankDeferredPendingSetup::new(
            lookup,
            SimulationNormalArenaRegistrationAdapter::with_environment(
                CoordinatorDispatch {
                    events: Arc::clone(events),
                },
                CoordinatorEnvironment,
            ),
            CoordinatorLocation {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            CoordinatorDmo {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            callback,
        )
    }

    fn dense_deferred_producer(
        lookup: crate::simulation_resident_registry::SimulationResidentLookup<
            DmoBankProcessedArenaAsset,
        >,
        events: &Arc<Mutex<Vec<CoordinatorEvent>>>,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> TestDenseDeferredProducer {
        MainDmoBankDenseDeferredProducer::new(
            lookup,
            SimulationNormalArenaRegistrationAdapter::with_environment(
                CoordinatorDispatch {
                    events: Arc::clone(events),
                },
                CoordinatorEnvironment,
            ),
            CoordinatorLocation {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            CoordinatorDmo {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            callback,
        )
    }

    fn public_deferred_producer(
        lookup: crate::simulation_resident_registry::SimulationResidentLookup<
            DmoBankProcessedArenaAsset,
        >,
        events: &Arc<Mutex<Vec<CoordinatorEvent>>>,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> TestPublicDeferredProducer {
        MainDmoSimulationDeferredProducer::new(
            lookup,
            SimulationNormalArenaRegistrationAdapter::with_environment(
                CoordinatorDispatch {
                    events: Arc::clone(events),
                },
                CoordinatorEnvironment,
            ),
            CoordinatorLocation {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            CoordinatorDmo {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            callback,
        )
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "the helper directly supplies the manager's fallible Setup boundary"
    )]
    fn setup_mixed_deferred_row(
        row: &mut DeferredCollectionRegRow<TestMixedDeferredPayload>,
    ) -> Result<u32, Infallible> {
        let registration_type = row.registration_type();
        match (registration_type, row.payload_mut()) {
            (DeferredCollectionRegType::Simulation, TestMixedDeferredPayload::Simulation(slot)) => {
                let producer = slot.take().expect("Simulation row retains its producer");
                let (producer, update) = producer
                    .try_update_with(|| -> Result<_, Infallible> {
                        panic!("Simulation Setup must not sample deferred controls")
                    })
                    .unwrap();
                **slot = Some(producer);
                match update {
                    MainDmoBankDenseDeferredProducerUpdate::SetupCompleted => {}
                    other => panic!("Simulation row Setup returned {other:?}"),
                }
            }
            (
                DeferredCollectionRegType::Presentation,
                TestMixedDeferredPayload::Presentation(slot),
            ) => {
                let producer = slot.take().expect("Presentation row retains its producer");
                let (producer, update) = producer
                    .try_update_with(|| -> Result<_, Infallible> {
                        panic!("Presentation Setup must not sample deferred controls")
                    })
                    .unwrap();
                **slot = Some(producer);
                match update {
                    MainDmoPresentationDeferredProducerUpdate::SetupCompleted => {}
                    other => panic!("Presentation row Setup returned {other:?}"),
                }
            }
            _ => panic!("deferred row discriminator and payload kind diverged"),
        }
        Ok(DMO_BANK_ASSET_COUNT)
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "the helper directly supplies the manager's fallible active boundary"
    )]
    fn advance_mixed_simulation(
        mut active: DeferredCollectionRegActive<'_, TestMixedDeferredPayload>,
    ) -> Result<u32, Infallible> {
        let TestMixedDeferredPayload::Simulation(slot) = active.payload_mut() else {
            panic!("Simulation updater received a Presentation payload")
        };
        let producer = slot.take().expect("Simulation row retains its producer");
        let (producer, update) = producer
            .try_update_with(|| Ok::<_, Infallible>(DeferredCollectionBatchControls::default()))
            .unwrap();
        **slot = Some(producer);
        match update {
            MainDmoBankDenseDeferredProducerUpdate::ArenasQueued { count }
            | MainDmoBankDenseDeferredProducerUpdate::TerminalQueued { arena_count: count } => {
                Ok(count)
            }
            MainDmoBankDenseDeferredProducerUpdate::SetupCompleted => {
                panic!("active Simulation row repeated Setup")
            }
            MainDmoBankDenseDeferredProducerUpdate::TerminalIdle => {
                panic!("manager advanced a completed Simulation row")
            }
        }
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "the helper directly supplies the manager's fallible active boundary"
    )]
    fn advance_mixed_presentation(
        mut active: DeferredCollectionRegActive<'_, TestMixedDeferredPayload>,
    ) -> Result<u32, Infallible> {
        let TestMixedDeferredPayload::Presentation(slot) = active.payload_mut() else {
            panic!("Presentation updater received a Simulation payload")
        };
        let producer = slot.take().expect("Presentation row retains its producer");
        let (producer, update) = producer
            .try_update_with(|| Ok::<_, Infallible>(DeferredCollectionBatchControls::default()))
            .unwrap();
        **slot = Some(producer);
        match update {
            MainDmoPresentationDeferredProducerUpdate::ArenasQueued { count }
            | MainDmoPresentationDeferredProducerUpdate::TerminalQueued {
                arena_count: count,
                ..
            } => Ok(count),
            MainDmoPresentationDeferredProducerUpdate::SetupCompleted => {
                panic!("active Presentation row repeated Setup")
            }
            MainDmoPresentationDeferredProducerUpdate::TerminalIdle => {
                panic!("manager advanced a completed Presentation row")
            }
        }
    }

    fn deferred_pending_with_runtime_failure(
        lookup: crate::simulation_resident_registry::SimulationResidentLookup<
            DmoBankProcessedArenaAsset,
        >,
        events: &Arc<Mutex<Vec<CoordinatorEvent>>>,
        callback: MainDmoSimulationRegistrationCallback,
        source: SimulationDmoRegistrationRuntimeInitError,
    ) -> TestDeferredPending {
        MainDmoBankDeferredPendingSetup::new_with_runtime_failure_for_test(
            lookup,
            SimulationNormalArenaRegistrationAdapter::with_environment(
                CoordinatorDispatch {
                    events: Arc::clone(events),
                },
                CoordinatorEnvironment,
            ),
            CoordinatorLocation {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            CoordinatorDmo {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            callback,
            source,
        )
    }

    fn deferred_pending_with_begin_failure(
        lookup: crate::simulation_resident_registry::SimulationResidentLookup<
            DmoBankProcessedArenaAsset,
        >,
        events: &Arc<Mutex<Vec<CoordinatorEvent>>>,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> TestDeferredPending {
        MainDmoBankDeferredPendingSetup::new_with_begin_failure_for_test(
            lookup,
            SimulationNormalArenaRegistrationAdapter::with_environment(
                CoordinatorDispatch {
                    events: Arc::clone(events),
                },
                CoordinatorEnvironment,
            ),
            CoordinatorLocation {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            CoordinatorDmo {
                events: Arc::clone(events),
                next_register: 0,
                next_deregister: 0,
            },
            callback,
            MainDmoBankCollectionRegistrationError::Collection(
                SimulationCollectionRegistrationError::LiveStorageAllocation { requested: 1 },
            ),
        )
    }

    fn corpus_assets(
        asset_ids: &[u64; 125],
    ) -> (
        Vec<DmoBankProcessedArenaAsset>,
        Vec<Arc<SimulationNormalArenaPlan>>,
    ) {
        corpus_assets_with_dmo_count(asset_ids, 93)
    }

    fn corpus_assets_with_dmo_count(
        asset_ids: &[u64; 125],
        dmo_count: usize,
    ) -> (
        Vec<DmoBankProcessedArenaAsset>,
        Vec<Arc<SimulationNormalArenaPlan>>,
    ) {
        let mut assets = Vec::with_capacity(asset_ids.len());
        let mut plans = Vec::with_capacity(asset_ids.len());
        for (ordinal, &asset_id) in asset_ids.iter().enumerate() {
            let records = if ordinal < dmo_count {
                vec![(
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                    0x2000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                    0x2000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                )]
            } else {
                Vec::new()
            };
            let (asset, plan) = asset(asset_id, &records);
            assets.push(asset);
            plans.push(plan);
        }
        (assets, plans)
    }

    fn corpus_assets_with_shared_first_model() -> Vec<DmoBankProcessedArenaAsset> {
        let mut assets = Vec::with_capacity(CORPUS_ASSET_IDS.len());
        for (ordinal, &asset_id) in CORPUS_ASSET_IDS.iter().enumerate() {
            let records = if ordinal < 93 {
                let local_guid = if ordinal < 2 {
                    0x2000_0000_0000_0000
                } else {
                    0x2000_0000_0000_0000 | u64::try_from(ordinal).unwrap()
                };
                vec![(
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                    local_guid,
                    local_guid,
                )]
            } else {
                Vec::new()
            };
            assets.push(asset(asset_id, &records).0);
        }
        assets
    }

    fn corpus_assets_with_model_prefix(prefix: u64) -> Vec<DmoBankProcessedArenaAsset> {
        CORPUS_ASSET_IDS
            .iter()
            .enumerate()
            .map(|(ordinal, &asset_id)| {
                let records = if ordinal < 93 {
                    let model_id = prefix | u64::try_from(ordinal).unwrap();
                    vec![(
                        0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                        model_id,
                        model_id,
                    )]
                } else {
                    Vec::new()
                };
                asset(asset_id, &records).0
            })
            .collect()
    }

    #[test]
    fn published_occurrence_builds_exact_binding_and_removal_invalidates_fresh_pass() {
        let (assets, plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, wrapper, token, snapshot) = published_snapshot(&collection);

        let binding =
            PreparedMainDmoBankSimulationCollection::try_from_published_snapshot(snapshot).unwrap();
        let source = plans[0].record_handle(0).unwrap();
        let mut live = binding.model_batch.begin_pass();
        live.try_issue(&source, 0x1000_0000_0000_0000).unwrap();

        registry
            .remove_exact(CollectionId(DMO_BANK_COLLECTION_ID), &token)
            .unwrap();
        let mut stale = binding.model_batch.begin_pass();
        assert!(matches!(
            stale.try_issue(&source, 0x1000_0000_0000_0000),
            Err(SimulationDmoBankModelRegistrationPassError::ResidentExpired)
        ));

        drop(live);
        drop(stale);
        drop(binding);
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn opaque_snapshot_exact_asset_comparison_rejects_order_and_owner_changes() {
        let (assets, plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, wrapper, token, snapshot) = published_snapshot(&collection);
        let mut exact: Vec<_> = collection
            .weak_assets_in_order()
            .unwrap()
            .into_iter()
            .map(|asset| asset.weak().upgrade().unwrap())
            .collect();
        assert!(snapshot.has_same_assets(exact.iter()));

        exact.swap(0, 1);
        assert!(!snapshot.has_same_assets(exact.iter()));
        exact.swap(0, 1);
        let replacement = Arc::new(DmoBankProcessedArenaAsset::from_parts_for_test(
            Arc::clone(&plans[0]),
            Vec::new(),
        ));
        exact[0] = replacement;
        assert!(!snapshot.has_same_assets(exact.iter()));

        drop(snapshot);
        registry
            .remove_exact(CollectionId(DMO_BANK_COLLECTION_ID), &token)
            .unwrap();
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn published_batch_preparation_failure_retains_opaque_snapshot_for_retry() {
        let (bad_asset, _plan) = asset(CORPUS_ASSET_IDS[0], &[(0x1111_2222_3333_4444, 0x55, 0x66)]);
        let (mut storage, collection) = collection(vec![bad_asset]);
        let (mut registry, wrapper, token, snapshot) = published_snapshot(&collection);

        let Err(failure) =
            PreparedMainDmoBankSimulationCollection::try_from_published_snapshot(snapshot)
        else {
            panic!("mismatched DMO record identity must refuse the model batch")
        };
        assert!(matches!(
            failure.error(),
            PreparedMainDmoBankPublishedBindingError::ModelBatch(
                SimulationDmoBankModelRegistrationBatchError::SourcePreparation {
                    asset_index: 0,
                    operation_index: 0,
                    source: SimulationDmoModelRegistrationSourceError::LocalGuidMismatch {
                        operation_index: 0,
                    },
                }
            )
        ));
        let Err(retry) = failure.retry() else {
            panic!("retry must revalidate the unchanged malformed source")
        };
        assert!(matches!(
            retry.error(),
            PreparedMainDmoBankPublishedBindingError::ModelBatch(
                SimulationDmoBankModelRegistrationBatchError::SourcePreparation {
                    asset_index: 0,
                    operation_index: 0,
                    ..
                }
            )
        ));

        drop(retry);
        registry
            .remove_exact(CollectionId(DMO_BANK_COLLECTION_ID), &token)
            .unwrap();
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn published_collection_gate_failure_retains_exact_prepared_batch_for_retry() {
        let mut asset_ids = CORPUS_ASSET_IDS;
        asset_ids.swap(17, 18);
        let (assets, _plans) = corpus_assets(&asset_ids);
        let (mut storage, collection) = collection(assets);
        let (mut registry, wrapper, token, snapshot) = published_snapshot(&collection);

        let Err(failure) =
            PreparedMainDmoBankSimulationCollection::try_from_published_snapshot(snapshot)
        else {
            panic!("wrong corpus order must refuse the combined binding")
        };
        assert!(matches!(
            failure.error(),
            PreparedMainDmoBankPublishedBindingError::Collection(
                PreparedMainDmoBankSimulationCollectionError::CorpusAssetIdSequenceMismatch
            )
        ));
        let Err(retry) = failure.retry() else {
            panic!("retry must preserve the same wrong exact asset sequence")
        };
        assert!(matches!(
            retry.error(),
            PreparedMainDmoBankPublishedBindingError::Collection(
                PreparedMainDmoBankSimulationCollectionError::CorpusAssetIdSequenceMismatch
            )
        ));

        drop(retry);
        registry
            .remove_exact(CollectionId(DMO_BANK_COLLECTION_ID), &token)
            .unwrap();
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn published_acquisition_is_lazy_and_missing_retry_uses_the_same_lookup() {
        let (assets, _plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let acquisition = MainDmoBankPublishedAcquisition::new(lookup);

        let Err(missing) = acquisition.try_resolve() else {
            panic!("an unpublished main bank must remain missing")
        };
        assert_eq!(
            missing.error(),
            MainDmoBankPublishedAcquisitionError::MissingCollection
        );
        assert_eq!(
            format!("{missing:?}"),
            "MainDmoBankPublishedAcquisitionFailure { error: MissingCollection, .. }"
        );

        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let owner = missing.retry().unwrap();
        owner.try_revalidate().unwrap();

        registry.remove_exact(id, &token).unwrap();
        drop((owner, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn acquisition_construction_does_not_touch_a_detached_lookup() {
        let (registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        drop(registry);
        let acquisition = MainDmoBankPublishedAcquisition::new(lookup);

        let Err(failure) = acquisition.try_resolve() else {
            panic!("explicit resolution must observe the detached registry")
        };
        assert_eq!(
            failure.error(),
            MainDmoBankPublishedAcquisitionError::RegistryDetached
        );
    }

    #[test]
    fn resolved_acquisition_refuses_a_same_id_reload_without_losing_itself() {
        let (first_assets, _plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let mut storage = TestStorage::new(identity_error);
        let first = storage.create_empty(125).unwrap();
        let second = storage.create_empty(125).unwrap();
        for (collection, assets) in [(&first, first_assets), (&second, second_assets)] {
            let mut writer = storage.writer(collection).unwrap();
            for asset in assets {
                writer.add_asset(asset);
            }
        }
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let owner = MainDmoBankPublishedAcquisition::new(lookup)
            .try_resolve()
            .unwrap();
        registry.remove_exact(id, &first_token).unwrap();

        let second_wrapper = registry.create_wrapper(id, &second).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();
        assert_eq!(
            owner.try_revalidate(),
            Err(MainDmoBankPublishedAcquisitionError::StaleOccurrence)
        );
        assert_eq!(
            owner.try_revalidate(),
            Err(MainDmoBankPublishedAcquisitionError::StaleOccurrence)
        );

        registry.remove_exact(id, &second_token).unwrap();
        drop((owner, first_wrapper, second_wrapper));
        storage.destroy(first);
        storage.destroy(second);
    }

    #[test]
    fn malformed_binding_retry_cannot_hop_to_a_valid_same_id_reload() {
        let mut malformed_ids = CORPUS_ASSET_IDS;
        malformed_ids.swap(17, 18);
        let (malformed_assets, _plans) = corpus_assets(&malformed_ids);
        let (valid_assets, _plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut malformed_storage, malformed_collection) = collection(malformed_assets);
        let (mut valid_storage, valid_collection) = collection(valid_assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let malformed_wrapper = registry.create_wrapper(id, &malformed_collection).unwrap();
        let malformed_token = TestResidentRegistry::stable_token(&malformed_wrapper);
        registry.publish(id, &malformed_token).unwrap();
        let Err(failure) = MainDmoBankPublishedAcquisition::new(lookup).try_resolve() else {
            panic!("the malformed corpus must not publish an acquisition owner")
        };
        assert_eq!(
            failure.error(),
            MainDmoBankPublishedAcquisitionError::CollectionBinding
        );
        registry.remove_exact(id, &malformed_token).unwrap();

        let valid_wrapper = registry.create_wrapper(id, &valid_collection).unwrap();
        let valid_token = TestResidentRegistry::stable_token(&valid_wrapper);
        registry.publish(id, &valid_token).unwrap();
        let Err(retry) = failure.retry() else {
            panic!("retry must retain the malformed snapshot-derived batch")
        };
        assert_eq!(
            retry.error(),
            MainDmoBankPublishedAcquisitionError::CollectionBinding
        );

        registry.remove_exact(id, &valid_token).unwrap();
        drop((retry, malformed_wrapper, valid_wrapper));
        malformed_storage.destroy(malformed_collection);
        valid_storage.destroy(valid_collection);
    }

    #[test]
    fn resolved_acquisition_refuses_same_occurrence_sequence_append() {
        let (assets, _plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let mut storage = TestStorage::new(identity_error);
        let collection = storage.create_empty(126).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        for asset in assets {
            writer.add_asset(asset);
        }
        drop(writer);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let owner = MainDmoBankPublishedAcquisition::new(lookup)
            .try_resolve()
            .unwrap();

        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(0xfeed_face_cafe_beef, &[]).0);
        drop(writer);
        assert_eq!(
            owner.try_revalidate(),
            Err(MainDmoBankPublishedAcquisitionError::ResidentAssetSequenceMismatch)
        );

        registry.remove_exact(id, &token).unwrap();
        drop((owner, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn resolved_acquisition_rejects_a_foreign_registry_domain() {
        let (first_assets, _plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut first_storage, first_collection) = collection(first_assets);
        let (mut second_storage, second_collection) = collection(second_assets);
        let (mut first_registry, first_lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let (mut second_registry, second_lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = first_registry
            .create_wrapper(id, &first_collection)
            .unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        first_registry.publish(id, &first_token).unwrap();
        let mut owner = MainDmoBankPublishedAcquisition::new(first_lookup)
            .try_resolve()
            .unwrap();
        let second_wrapper = second_registry
            .create_wrapper(id, &second_collection)
            .unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        second_registry.publish(id, &second_token).unwrap();

        owner.lookup = second_lookup;
        assert_eq!(
            owner.try_revalidate(),
            Err(MainDmoBankPublishedAcquisitionError::StaleOccurrence)
        );

        first_registry.remove_exact(id, &first_token).unwrap();
        second_registry.remove_exact(id, &second_token).unwrap();
        drop((owner, first_wrapper, second_wrapper));
        first_storage.destroy(first_collection);
        second_storage.destroy(second_collection);
    }

    #[test]
    fn binding_uses_fixed_id_and_exact_pinned_plan_order() {
        let (assets, plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);

        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let prepared = &binding.prepared;
        assert_eq!(prepared.collection_id(), DMO_BANK_COLLECTION_ID);
        assert_eq!(prepared.arena_count(), 125);
        for (ordinal, expected) in plans.iter().enumerate() {
            assert!(Arc::ptr_eq(
                &prepared.arena_plan_owner(ordinal).unwrap(),
                expected
            ));
        }
    }

    #[test]
    fn main_bank_rejects_a_native_selected_kind31_operation_after_existing_corpus_gates() {
        const FIRST_MODEL: (u64, u64, u64) = (
            0x1000_0000_0000_0000,
            0x2000_0000_0000_0000,
            0x2000_0000_0000_0000,
        );
        const KIND31: (u64, u64, u64, u64) = (
            0x3000_0000_0000_0000,
            0x4000_0000_0000_0000,
            0x5000_0000_0000_0000,
            0x6000_0000_0000_0000,
        );

        let (mut assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (asset, plan) = asset_with_kind31(CORPUS_ASSET_IDS[0], &[FIRST_MODEL], Some(KIND31));
        let dmo_plan = SimulationDmoArenaPlan::from_normal_plan(Arc::clone(&plan)).unwrap();
        assert_eq!(dmo_plan.operations().len(), 1);
        assert_eq!(dmo_plan.operations()[0].asset_id, KIND31.0);
        assert_eq!(dmo_plan.operations()[0].locator_count, 2);
        assets[0] = asset;
        let (_storage, resident_collection) = collection(assets);
        assert!(matches!(
            PreparedMainDmoBankSimulationCollection::try_new(&resident_collection),
            Err(
                PreparedMainDmoBankSimulationCollectionError::CorpusKind31LocatorOperationCountMismatch {
                    actual: 1
                }
            )
        ));

        let (mut assets, _) = corpus_assets_with_dmo_count(&CORPUS_ASSET_IDS, 92);
        assets[0] = asset_with_kind31(CORPUS_ASSET_IDS[0], &[FIRST_MODEL], Some(KIND31)).0;
        let (_storage, resident_collection) = collection(assets);
        assert!(matches!(
            PreparedMainDmoBankSimulationCollection::try_new(&resident_collection),
            Err(
                PreparedMainDmoBankSimulationCollectionError::CorpusDmoInstanceCountMismatch {
                    actual: 92
                }
            )
        ));
    }

    #[test]
    fn empty_duplicate_and_reordered_corpora_are_rejected_in_that_order() {
        let (_storage, resident_collection) = collection(Vec::new());
        assert!(matches!(
            PreparedMainDmoBankSimulationCollection::try_new(&resident_collection),
            Err(
                PreparedMainDmoBankSimulationCollectionError::CorpusAssetCountMismatch {
                    actual: 0
                }
            )
        ));

        let mut duplicate_ids = CORPUS_ASSET_IDS;
        duplicate_ids[1] = duplicate_ids[0];
        let (assets, _) = corpus_assets(&duplicate_ids);
        let (_storage, resident_collection) = collection(assets);
        assert!(matches!(
            PreparedMainDmoBankSimulationCollection::try_new(&resident_collection),
            Err(
                PreparedMainDmoBankSimulationCollectionError::CorpusAssetIdDuplicate {
                    first_ordinal: 0,
                    second_ordinal: 1
                }
            )
        ));

        let mut reordered_ids = CORPUS_ASSET_IDS;
        reordered_ids.swap(1, 2);
        let (assets, _) = corpus_assets(&reordered_ids);
        let (_storage, resident_collection) = collection(assets);
        assert!(matches!(
            PreparedMainDmoBankSimulationCollection::try_new(&resident_collection),
            Err(PreparedMainDmoBankSimulationCollectionError::CorpusAssetIdSequenceMismatch)
        ));
    }

    #[test]
    fn exact_asset_corpus_rejects_both_adjacent_kind_11_count_mutations() {
        for actual in [92, 94] {
            let (assets, _) = corpus_assets_with_dmo_count(&CORPUS_ASSET_IDS, actual);
            let (_storage, resident_collection) = collection(assets);
            assert!(matches!(
                PreparedMainDmoBankSimulationCollection::try_new(&resident_collection),
                Err(
                    PreparedMainDmoBankSimulationCollectionError::CorpusDmoInstanceCountMismatch {
                        actual: rejected
                    }
                ) if rejected == actual
            ));
        }

        let mut reordered_ids = CORPUS_ASSET_IDS;
        reordered_ids.swap(1, 2);
        let (assets, _) = corpus_assets_with_dmo_count(&reordered_ids, 92);
        let (_storage, resident_collection) = collection(assets);
        assert!(matches!(
            PreparedMainDmoBankSimulationCollection::try_new(&resident_collection),
            Err(
                PreparedMainDmoBankSimulationCollectionError::CorpusDmoInstanceCountMismatch {
                    actual: 92
                }
            )
        ));
    }

    #[test]
    fn prepared_collection_derivation_uses_batch_assets_after_resident_tombstone() {
        const FIRST: u64 = 0x1000_0000_0000_0000;
        let (assets, plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, resident_collection) = collection(assets);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&resident_collection).unwrap();
        resident_collection.tombstone_without_releasing_assets_for_test();

        let prepared = batch.try_prepare_collection().unwrap();
        assert_eq!(prepared.collection_id(), DMO_BANK_COLLECTION_ID);
        assert_eq!(prepared.arena_count(), 125);
        assert!(Arc::ptr_eq(
            &prepared.arena_plan_owner(0).unwrap(),
            &plans[0]
        ));
        let mut pass = batch.begin_pass();
        assert!(matches!(
            pass.try_issue(&plans[0].record_handle(0).unwrap(), FIRST),
            Err(SimulationDmoBankModelRegistrationPassError::ResidentExpired)
        ));
    }

    #[test]
    fn bound_cursor_keeps_exact_sequence_and_samples_residence_only_on_issue() {
        const FIRST: u64 = 0x1111_0000_dead_beef;
        const SECOND: u64 = 0x2222_0000_dead_beef;
        let (mut assets, plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (first, first_plan) = asset(CORPUS_ASSET_IDS[0], &[(FIRST, 11, 11)]);
        let (second, second_plan) = asset(CORPUS_ASSET_IDS[1], &[(SECOND, 22, 22)]);
        assets[0] = first;
        assets[1] = second;
        let (foreign, foreign_plan) = asset(CORPUS_ASSET_IDS[0], &[(FIRST, 11, 11)]);
        drop(foreign);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();

        let mut pass = binding.model_batch.begin_pass();
        assert!(matches!(
            pass.try_issue(&foreign_plan.record_handle(0).unwrap(), FIRST),
            Err(SimulationDmoBankModelRegistrationPassError::SourceMismatch { ordinal: 0 })
        ));
        drop(
            pass.try_issue(&first_plan.record_handle(0).unwrap(), FIRST)
                .unwrap(),
        );
        drop(
            pass.try_issue(&second_plan.record_handle(0).unwrap(), SECOND)
                .unwrap(),
        );
        for (ordinal, plan) in plans.iter().enumerate().skip(2).take(91) {
            drop(
                pass.try_issue(
                    &plan.record_handle(0).unwrap(),
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                )
                .unwrap(),
            );
        }
        pass.finish().unwrap();

        collection.tombstone_without_releasing_assets_for_test();
        assert_eq!(binding.prepared.arena_count(), 125);
        let mut expired = binding.model_batch.begin_pass();
        assert!(matches!(
            expired.try_issue(&first_plan.record_handle(0).unwrap(), FIRST),
            Err(SimulationDmoBankModelRegistrationPassError::ResidentExpired)
        ));
    }

    #[test]
    fn model_batch_failure_precedes_collection_publication_and_preserves_source() {
        let (malformed, _) = asset(0xa0, &[(1, 11, 22)]);
        let (_storage, collection) = collection(vec![malformed]);

        assert!(matches!(
            PreparedMainDmoBankSimulationCollection::try_new(&collection),
            Err(PreparedMainDmoBankSimulationCollectionError::ModelBatch(
                SimulationDmoBankModelRegistrationBatchError::SourcePreparation {
                    asset_index: 0,
                    operation_index: 0,
                    source: SimulationDmoModelRegistrationSourceError::LocalGuidMismatch {
                        operation_index: 0
                    }
                }
            ))
        ));
    }

    #[test]
    fn coordinator_init_failure_preserves_every_input_for_effect_free_retry() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut first_storage, first_collection) = collection(first_assets);
        let (mut second_storage, second_collection) = collection(second_assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first_collection).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let binding = MainDmoBankPublishedAcquisition::new(lookup)
            .try_resolve()
            .unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let normal = SimulationNormalArenaRegistrationAdapter::with_environment(
            CoordinatorDispatch {
                events: Arc::clone(&events),
            },
            CoordinatorEnvironment,
        );
        let location = CoordinatorLocation {
            events: Arc::clone(&events),
            next_register: 0,
            next_deregister: 0,
        };
        let dmo = CoordinatorDmo {
            events: Arc::clone(&events),
            next_register: 0,
            next_deregister: 0,
        };
        let source = SimulationDmoRegistrationRuntimeInitError::Manager(
            SimulationDmoRegistrationManagerInitError::Models(
                SimulationDmoModelRegistryInitError::StorageAllocation { requested: 256 },
            ),
        );
        assert_eq!(Arc::strong_count(&events), 4);

        let failure =
            MainDmoBankCollectionCoordinator::fail_published_runtime_initialization_for_test(
                binding, normal, location, dmo, source,
            );
        assert_eq!(failure.source(), source);
        assert_eq!(Arc::strong_count(&events), 4);
        assert!(events.lock().unwrap().is_empty());
        assert_eq!(
            format!("{failure:?}"),
            "MainDmoBankCollectionCoordinatorInitFailure { source: Manager(Models(StorageAllocation { requested: 256 })), .. }"
        );

        let locator_source = SimulationDmoRegistrationRuntimeInitError::Manager(
            SimulationDmoRegistrationManagerInitError::Locators(
                SimulationDmoLocatorRegistryInitError::StorageAllocation { requested: 50 },
            ),
        );
        let failure = failure.retry_with_runtime_failure_for_test(locator_source);
        assert_eq!(failure.source(), locator_source);
        assert_eq!(Arc::strong_count(&events), 4);
        assert!(events.lock().unwrap().is_empty());
        assert_eq!(
            format!("{failure:?}"),
            "MainDmoBankCollectionCoordinatorInitFailure { source: Manager(Locators(StorageAllocation { requested: 50 })), .. }"
        );

        let mut coordinator = failure.retry().unwrap();
        assert_eq!(Arc::strong_count(&events), 4);
        assert!(events.lock().unwrap().is_empty());
        coordinator.try_revalidate_published_binding().unwrap();
        let registration = coordinator.register_active().unwrap();
        assert!(
            events
                .lock()
                .unwrap()
                .contains(&CoordinatorEvent::LocationRegister(0))
        );
        assert!(
            events
                .lock()
                .unwrap()
                .contains(&CoordinatorEvent::DmoRegister(0))
        );
        assert!(coordinator.has_model(0x2000_0000_0000_0000));
        coordinator.deregister(registration).unwrap();
        assert!(!coordinator.has_model(0x2000_0000_0000_0000));

        registry.remove_exact(id, &first_token).unwrap();
        let second_wrapper = registry.create_wrapper(id, &second_collection).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();
        assert_eq!(
            coordinator.try_revalidate_published_binding(),
            Err(MainDmoBankPublishedAcquisitionError::StaleOccurrence)
        );

        registry.remove_exact(id, &second_token).unwrap();
        drop((coordinator, first_wrapper, second_wrapper));
        first_storage.destroy(first_collection);
        second_storage.destroy(second_collection);
    }

    #[test]
    fn abandoned_coordinator_init_failure_is_inert_and_releases_retained_owners() {
        let (assets, plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let first_plan = Arc::downgrade(&plans[0]);
        drop(plans);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let binding = MainDmoBankPublishedAcquisition::new(lookup)
            .try_resolve()
            .unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let failure =
            MainDmoBankCollectionCoordinator::fail_published_runtime_initialization_for_test(
                binding,
                SimulationNormalArenaRegistrationAdapter::with_environment(
                    CoordinatorDispatch {
                        events: Arc::clone(&events),
                    },
                    CoordinatorEnvironment,
                ),
                CoordinatorLocation {
                    events: Arc::clone(&events),
                    next_register: 0,
                    next_deregister: 0,
                },
                CoordinatorDmo {
                    events: Arc::clone(&events),
                    next_register: 0,
                    next_deregister: 0,
                },
                SimulationDmoRegistrationRuntimeInitError::Manager(
                    SimulationDmoRegistrationManagerInitError::Locators(
                        SimulationDmoLocatorRegistryInitError::StorageAllocation { requested: 50 },
                    ),
                ),
            );

        registry.remove_exact(id, &token).unwrap();
        drop(wrapper);
        storage.destroy(collection);
        assert!(first_plan.upgrade().is_some());
        assert_eq!(Arc::strong_count(&events), 4);
        assert!(events.lock().unwrap().is_empty());

        drop(failure);
        assert!(first_plan.upgrade().is_none());
        assert_eq!(Arc::strong_count(&events), 1);
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn immediate_front_lazily_selects_the_current_same_id_winner_once() {
        let (mut first_storage, first_collection) =
            collection(corpus_assets_with_model_prefix(0x2000_0000_0000_0000));
        let (mut second_storage, second_collection) =
            collection(corpus_assets_with_model_prefix(0x3000_0000_0000_0000));
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first_collection).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let pending = immediate_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        assert_eq!(lookup_observer.count(), 0);
        assert_eq!(pending.queue_len_for_test(), 1);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &first_token).unwrap();
        drop(first_wrapper);
        first_storage.destroy(first_collection);
        let second_wrapper = registry.create_wrapper(id, &second_collection).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();
        let ready = pending.dispatch_one().unwrap();
        assert_eq!(lookup_observer.count(), 1);
        assert_eq!(ready.queue_len_for_test(), 0);
        assert_eq!(ready.live_registration_count_for_test(), 1);
        assert!(ready.producer_is_current_for_test());
        assert!(ready.has_model_for_test(0x3000_0000_0000_0000));
        assert!(!ready.has_model_for_test(0x2000_0000_0000_0000));

        registry.remove_exact(id, &second_token).unwrap();
        drop((ready, second_wrapper));
        second_storage.destroy(second_collection);
    }

    #[test]
    fn immediate_front_missing_acquisition_is_sticky_after_later_publish_and_detach() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let pending = immediate_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let Err(failure) = pending.dispatch_one() else {
            panic!("an unpublished actual front unexpectedly dispatched")
        };
        assert_eq!(failure.queue_len_for_test(), 1);
        assert_eq!(
            failure.error(),
            MainDmoBankImmediatePendingDispatchError::Acquisition(
                MainDmoBankPublishedAcquisitionError::MissingCollection
            )
        );
        assert!(events.lock().unwrap().is_empty());

        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        drop(registry);
        let Err(repeated) = failure.retry() else {
            panic!("a sticky actual-front lookup refusal unexpectedly retried")
        };
        assert_eq!(repeated.queue_len_for_test(), 1);
        assert_eq!(
            repeated.error(),
            MainDmoBankImmediatePendingDispatchError::Acquisition(
                MainDmoBankPublishedAcquisitionError::MissingCollection
            )
        );
        assert!(events.lock().unwrap().is_empty());

        drop((repeated, wrapper, token));
        storage.destroy(collection);
    }

    #[test]
    fn immediate_front_init_delay_refuses_same_id_reload_without_effect_or_hop() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut first_storage, first_collection) = collection(first_assets);
        let (mut second_storage, second_collection) = collection(second_assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first_collection).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let source = SimulationDmoRegistrationRuntimeInitError::Manager(
            SimulationDmoRegistrationManagerInitError::Locators(
                SimulationDmoLocatorRegistryInitError::StorageAllocation { requested: 50 },
            ),
        );
        let pending = immediate_pending_with_runtime_failure(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
            source,
        );
        let Err(failure) = pending.dispatch_one() else {
            panic!("injected runtime refusal unexpectedly initialized")
        };
        assert_eq!(lookup_observer.count(), 1);
        assert_eq!(
            failure.error(),
            MainDmoBankImmediatePendingDispatchError::RuntimeInitialization(source)
        );
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &first_token).unwrap();
        let second_wrapper = registry.create_wrapper(id, &second_collection).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();
        let second_source = SimulationDmoRegistrationRuntimeInitError::Manager(
            SimulationDmoRegistrationManagerInitError::Models(
                SimulationDmoModelRegistryInitError::StorageAllocation { requested: 256 },
            ),
        );
        let failure = failure.with_next_runtime_failure_for_test(second_source);
        let Err(failure) = failure.retry() else {
            panic!("second injected runtime refusal unexpectedly initialized")
        };
        assert_eq!(
            failure.error(),
            MainDmoBankImmediatePendingDispatchError::RuntimeInitialization(second_source)
        );
        assert_eq!(lookup_observer.count(), 1);
        assert!(events.lock().unwrap().is_empty());
        let Err(stale) = failure.retry() else {
            panic!("same-ID reload unexpectedly replaced the retained occurrence")
        };
        assert_eq!(lookup_observer.count(), 2);
        let expected = MainDmoBankImmediatePendingDispatchError::PublishedBindingRevalidation(
            MainDmoBankPublishedAcquisitionError::StaleOccurrence,
        );
        assert_eq!(stale.error(), expected);
        registry.remove_exact(id, &second_token).unwrap();
        let Err(sticky) = stale.retry() else {
            panic!("stale revalidation refusal unexpectedly retried")
        };
        assert_eq!(sticky.error(), expected);
        assert_eq!(lookup_observer.count(), 2);
        assert_eq!(sticky.queue_len_for_test(), 1);
        assert!(events.lock().unwrap().is_empty());
        drop(registry);

        drop((
            sticky,
            first_wrapper,
            first_token,
            second_wrapper,
            second_token,
        ));
        first_storage.destroy(first_collection);
        second_storage.destroy(second_collection);
    }

    #[test]
    fn immediate_front_unchanged_init_retry_executes_and_pops_exactly_once() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let source = SimulationDmoRegistrationRuntimeInitError::Manager(
            SimulationDmoRegistrationManagerInitError::Models(
                SimulationDmoModelRegistryInitError::StorageAllocation { requested: 256 },
            ),
        );
        let pending = immediate_pending_with_runtime_failure(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
            source,
        );
        let Err(failure) = pending.dispatch_one() else {
            panic!("injected runtime refusal unexpectedly initialized")
        };
        assert_eq!(failure.queue_len_for_test(), 1);
        assert!(events.lock().unwrap().is_empty());

        let ready = failure.retry().unwrap();
        assert_eq!(ready.queue_len_for_test(), 0);
        assert_eq!(ready.live_registration_count_for_test(), 1);
        assert!(ready.producer_is_current_for_test());
        assert!(ready.has_model_for_test(0x2000_0000_0000_0000));
        let observed = events.lock().unwrap();
        assert_eq!(observed.len(), 343);
        assert_eq!(
            observed
                .iter()
                .filter(|event| matches!(event, CoordinatorEvent::Add(_)))
                .count(),
            93
        );
        assert_eq!(
            observed
                .iter()
                .filter(|event| matches!(event, CoordinatorEvent::LocationRegister(_)))
                .count(),
            125
        );
        assert_eq!(
            observed
                .iter()
                .filter(|event| matches!(event, CoordinatorEvent::DmoRegister(_)))
                .count(),
            125
        );
        drop(observed);

        registry.remove_exact(id, &token).unwrap();
        drop((ready, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn deferred_setup_is_lazy_and_selects_the_current_same_id_winner_once() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut first_storage, first_collection) = collection(first_assets);
        let (mut second_storage, second_collection) = collection(second_assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first_collection).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let pending = deferred_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        assert_eq!(lookup_observer.count(), 0);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &first_token).unwrap();
        drop(first_wrapper);
        first_storage.destroy(first_collection);
        let second_wrapper = registry.create_wrapper(id, &second_collection).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();
        let ready = pending.try_setup().unwrap();
        assert_eq!(lookup_observer.count(), 1);
        assert_eq!(ready.queue_len_for_test(), 0);
        assert_eq!(ready.next_deferred_ordinal_for_test(), 0);
        assert!(ready.producer_is_current_for_test());
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &second_token).unwrap();
        drop((ready, second_wrapper));
        second_storage.destroy(second_collection);
    }

    #[test]
    fn deferred_setup_missing_acquisition_is_sticky_after_destructive_selection() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let pending = deferred_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let Err(failure) = pending.try_setup() else {
            panic!("missing deferred Setup acquisition unexpectedly succeeded")
        };
        let expected = MainDmoBankDeferredPendingSetupError::Acquisition(
            MainDmoBankPublishedAcquisitionError::MissingCollection,
        );
        assert_eq!(failure.error(), expected);
        assert_eq!(
            failure.stage_for_test(),
            MainDmoBankDeferredPendingStage::AcquisitionFault
        );
        assert_eq!(lookup_observer.count(), 1);
        assert!(events.lock().unwrap().is_empty());

        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let Err(sticky) = failure.retry() else {
            panic!("destructively selected Setup unexpectedly hopped to a later publication")
        };
        assert_eq!(sticky.error(), expected);
        assert_eq!(lookup_observer.count(), 1);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((sticky, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn deferred_setup_runtime_retry_precedes_fresh_revalidation() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut first_storage, first_collection) = collection(first_assets);
        let (mut second_storage, second_collection) = collection(second_assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first_collection).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let first_source = SimulationDmoRegistrationRuntimeInitError::Manager(
            SimulationDmoRegistrationManagerInitError::Locators(
                SimulationDmoLocatorRegistryInitError::StorageAllocation { requested: 50 },
            ),
        );
        let pending = deferred_pending_with_runtime_failure(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
            first_source,
        );
        let Err(failure) = pending.try_setup() else {
            panic!("injected deferred Setup runtime refusal unexpectedly succeeded")
        };
        assert_eq!(lookup_observer.count(), 1);
        assert_eq!(
            failure.error(),
            MainDmoBankDeferredPendingSetupError::RuntimeInitialization(first_source)
        );

        registry.remove_exact(id, &first_token).unwrap();
        let second_wrapper = registry.create_wrapper(id, &second_collection).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();
        let second_source = SimulationDmoRegistrationRuntimeInitError::Manager(
            SimulationDmoRegistrationManagerInitError::Models(
                SimulationDmoModelRegistryInitError::StorageAllocation { requested: 256 },
            ),
        );
        let failure = failure.with_next_runtime_failure_for_test(second_source);
        let Err(failure) = failure.retry() else {
            panic!("second injected runtime refusal unexpectedly succeeded")
        };
        assert_eq!(
            failure.error(),
            MainDmoBankDeferredPendingSetupError::RuntimeInitialization(second_source)
        );
        assert_eq!(lookup_observer.count(), 1);
        let Err(stale) = failure.retry() else {
            panic!("same-ID reload unexpectedly replaced delayed Setup authority")
        };
        let expected = MainDmoBankDeferredPendingSetupError::PublishedBindingRevalidation(
            MainDmoBankPublishedAcquisitionError::StaleOccurrence,
        );
        assert_eq!(stale.error(), expected);
        assert_eq!(
            stale.stage_for_test(),
            MainDmoBankDeferredPendingStage::InitializationRevalidationFault
        );
        assert_eq!(lookup_observer.count(), 2);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &second_token).unwrap();
        let Err(sticky) = stale.retry() else {
            panic!("Setup revalidation fault unexpectedly retried")
        };
        assert_eq!(sticky.error(), expected);
        assert_eq!(lookup_observer.count(), 2);
        drop((sticky, first_wrapper, second_wrapper));
        first_storage.destroy(first_collection);
        second_storage.destroy(second_collection);
    }

    #[test]
    fn deferred_setup_unchanged_runtime_retry_mints_empty_ready_once() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let source = SimulationDmoRegistrationRuntimeInitError::Manager(
            SimulationDmoRegistrationManagerInitError::Models(
                SimulationDmoModelRegistryInitError::StorageAllocation { requested: 256 },
            ),
        );
        let pending = deferred_pending_with_runtime_failure(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
            source,
        );
        let Err(failure) = pending.try_setup() else {
            panic!("injected runtime refusal unexpectedly succeeded")
        };
        assert_eq!(lookup_observer.count(), 1);
        let ready = failure.retry().unwrap();
        assert_eq!(lookup_observer.count(), 2);
        assert_eq!(ready.queue_len_for_test(), 0);
        assert_eq!(ready.next_deferred_ordinal_for_test(), 0);
        assert!(ready.producer_is_current_for_test());
        assert!(!ready.has_model_for_test(0x1000_0000_0000_0000));
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((ready, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn deferred_setup_begin_retry_revalidates_before_minting_ready() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let pending = deferred_pending_with_begin_failure(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let Err(failure) = pending.try_setup() else {
            panic!("injected deferred-begin refusal unexpectedly succeeded")
        };
        assert_eq!(
            failure.error(),
            MainDmoBankDeferredPendingSetupError::DeferredBegin
        );
        assert_eq!(
            failure.stage_for_test(),
            MainDmoBankDeferredPendingStage::Begin
        );
        assert_eq!(lookup_observer.count(), 1);
        assert!(events.lock().unwrap().is_empty());

        let ready = failure.retry().unwrap();
        assert_eq!(lookup_observer.count(), 2);
        assert_eq!(ready.queue_len_for_test(), 0);
        assert_eq!(ready.next_deferred_ordinal_for_test(), 0);
        assert!(ready.producer_is_current_for_test());
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((ready, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn deferred_setup_begin_delay_rejects_same_id_reload_stickily() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut first_storage, first_collection) = collection(first_assets);
        let (mut second_storage, second_collection) = collection(second_assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first_collection).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let pending = deferred_pending_with_begin_failure(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let Err(failure) = pending.try_setup() else {
            panic!("injected deferred-begin refusal unexpectedly succeeded")
        };
        assert_eq!(lookup_observer.count(), 1);
        registry.remove_exact(id, &first_token).unwrap();
        let second_wrapper = registry.create_wrapper(id, &second_collection).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();

        let Err(stale) = failure.retry() else {
            panic!("same-ID reload unexpectedly replaced delayed begin authority")
        };
        let expected = MainDmoBankDeferredPendingSetupError::PublishedBindingRevalidation(
            MainDmoBankPublishedAcquisitionError::StaleOccurrence,
        );
        assert_eq!(stale.error(), expected);
        assert_eq!(
            stale.stage_for_test(),
            MainDmoBankDeferredPendingStage::BeginRevalidationFault
        );
        assert_eq!(lookup_observer.count(), 2);
        assert!(events.lock().unwrap().is_empty());
        registry.remove_exact(id, &second_token).unwrap();
        let Err(sticky) = stale.retry() else {
            panic!("delayed-begin revalidation fault unexpectedly retried")
        };
        assert_eq!(sticky.error(), expected);
        assert_eq!(lookup_observer.count(), 2);

        drop((sticky, first_wrapper, second_wrapper));
        first_storage.destroy(first_collection);
        second_storage.destroy(second_collection);
    }

    #[test]
    fn dense_deferred_pending_update_sets_up_only_without_sampling_controls() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let collection_observer = lookup.attached_lookup_attempt_observer_for_test();
        let arena_observer = lookup
            .arena_lookup()
            .attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let producer = dense_deferred_producer(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        assert_eq!(
            producer.stage_for_test(),
            MainDmoBankDenseDeferredProducerStage::Pending
        );
        let mut samples = 0;
        let (producer, update) = producer
            .try_update_with(|| -> Result<_, std::convert::Infallible> {
                samples += 1;
                panic!("Setup must not sample deferred batch controls")
            })
            .unwrap();
        assert_eq!(
            update,
            MainDmoBankDenseDeferredProducerUpdate::SetupCompleted
        );
        assert_eq!(samples, 0);
        assert_eq!(
            producer.stage_for_test(),
            MainDmoBankDenseDeferredProducerStage::Active
        );
        assert_eq!(producer.next_ordinal_for_test(), 0);
        assert!(producer.queued_marker_kinds_for_test().is_empty());
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 0);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((producer, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn public_deferred_producer_facade_preserves_lazy_setup_failure_and_retry() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let producer = public_deferred_producer(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let (producer, setup) = producer
            .try_update_with(|| -> Result<_, &'static str> {
                panic!("public Pending façade must not sample controls")
            })
            .unwrap();
        assert_eq!(
            setup,
            MainDmoSimulationDeferredProducerUpdate::SetupCompleted
        );

        let Err(failure) = producer.try_update_with(|| Err::<_, _>("controls unavailable")) else {
            panic!("public Active façade unexpectedly accepted failed controls")
        };
        assert!(matches!(
            failure.kind(),
            MainDmoSimulationDeferredProducerFailureKind::Controls(source)
                if *source == "controls unavailable"
        ));
        let (producer, update) = failure
            .retry_with(|| Ok(DeferredCollectionBatchControls::NATIVE_DEFAULT))
            .unwrap();
        assert_eq!(
            update,
            MainDmoSimulationDeferredProducerUpdate::ArenasQueued { count: 12 }
        );
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((producer, wrapper));
        storage.destroy(collection);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one end-to-end public-driver test preserves both capability chains"
    )]
    fn public_terminal_driver_dispatches_registration_and_deregistration_fifos() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let producer = public_deferred_producer(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let (mut producer, setup) = producer
            .try_update_with(|| -> Result<_, Infallible> {
                panic!("public Pending façade must not sample controls")
            })
            .unwrap();
        assert_eq!(
            setup,
            MainDmoSimulationDeferredProducerUpdate::SetupCompleted
        );
        let Err(active) = producer.try_into_registration_driver() else {
            panic!("Active producer converted before its terminal markers existed")
        };
        producer = active;

        for _ in 0..10 {
            let (next, update) = producer
                .try_update_with(|| {
                    Ok::<_, Infallible>(DeferredCollectionBatchControls::NATIVE_DEFAULT)
                })
                .unwrap();
            producer = next;
            assert_eq!(
                update,
                MainDmoSimulationDeferredProducerUpdate::ArenasQueued { count: 12 }
            );
        }
        let (producer, terminal) = producer
            .try_update_with(|| {
                Ok::<_, Infallible>(DeferredCollectionBatchControls::NATIVE_DEFAULT)
            })
            .unwrap();
        assert_eq!(
            terminal,
            MainDmoSimulationDeferredProducerUpdate::TerminalQueued { arena_count: 5 }
        );
        let Ok(mut driver) = producer.try_into_registration_driver() else {
            panic!("Terminal producer did not yield its sealed FIFO")
        };

        for ordinal in 0..DMO_BANK_ASSET_COUNT {
            assert_eq!(
                driver.dispatch_one(harness.coordinator()).unwrap(),
                MainDmoSimulationDeferredDispatch::ReceiverMarkerCompleted(
                    MainDmoSimulationDeferredMarkerKind::Arena { ordinal }
                )
            );
        }
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoSimulationDeferredDispatch::ReceiverMarkerCompleted(
                MainDmoSimulationDeferredMarkerKind::Finish
            )
        );
        assert!(!driver.registration_acknowledged());
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoSimulationDeferredDispatch::RegistrationAcknowledged
        );
        assert!(driver.registration_acknowledged());
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoSimulationDeferredDispatch::Idle
        );
        assert_eq!(events.lock().unwrap().len(), 343);

        let callback = harness.advance_and_take_simulation_deregistration_callback();
        let mut driver = driver
            .try_queue_deregistration(callback)
            .unwrap_or_else(|failure| {
                panic!("public DReg pair staging failed: {:?}", failure.kind())
            });
        assert!(!driver.registration_acknowledged());
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoSimulationDeferredDispatch::ReceiverMarkerCompleted(
                MainDmoSimulationDeferredMarkerKind::DeregistrationCollection
            )
        );
        assert_eq!(events.lock().unwrap().len(), 686);
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoSimulationDeferredDispatch::DeregistrationCallbackReady
        );
        let (coordinator, releaser) = harness.acknowledgement_parts();
        assert!(
            driver
                .acknowledge_deregistration_callback(coordinator, releaser)
                .unwrap()
        );
        assert_eq!(harness.resident_count(), 0);
        assert_eq!(harness.unregistration_completed(), 2);
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoSimulationDeferredDispatch::Idle
        );

        registry.remove_exact(id, &token).unwrap();
        drop((driver, wrapper));
        storage.destroy(collection);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one end-to-end mixed-row assertion keeps both exact producer owners visible across the handoff"
    )]
    fn mixed_manager_hands_exact_simulation_terminal_owner_to_presentation_without_draining_it() {
        let (assets, plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let presentation_plans: Vec<_> = plans
            .iter()
            .map(|plan| {
                Arc::new(
                    SimulationNormalArenaPlan::from_static_plan(plan.static_plan_owner()).unwrap(),
                )
            })
            .collect();
        let presentation_arenas: Vec<_> = presentation_plans
            .iter()
            .map(|plan| PreparedMainDmoBankPresentationArena::from_plan(Arc::clone(plan)).unwrap())
            .collect();
        // The Simulation test corpus intentionally omits the real bank's one
        // material plus final-Arena DMO layout. Keep the owner/order/cost
        // shape for this manager coupling test; production census validation
        // remains exercised by the original-data probe.
        let presentation_collection =
            PreparedMainDmoBankPresentationCollection::from_parts_for_deferred_producer_test(
                &presentation_arenas,
            );
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let simulation = dense_deferred_producer(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let presentation = MainDmoPresentationDeferredProducer::new(
            presentation_collection,
            CollectionId(DMO_BANK_PARENT_ID),
            (),
        );
        let mut manager = TestMixedDeferredManager::new();
        manager
            .enqueue(DeferredCollectionRegRow::simulation(
                id,
                8,
                CORPUS_STREAM_SIZE,
                0,
                TestMixedDeferredPayload::Simulation(Box::new(Some(simulation))),
            ))
            .unwrap();
        manager
            .enqueue(DeferredCollectionRegRow::presentation(
                id,
                CollectionId(DMO_BANK_PARENT_ID),
                8,
                CORPUS_STREAM_SIZE,
                0,
                TestMixedDeferredPayload::Presentation(Box::new(Some(presentation))),
            ))
            .unwrap();

        let first = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Infallible> {
                    panic!("the first manager invocation is Setup-only")
                },
                setup_mixed_deferred_row,
            )
            .unwrap();
        assert!(matches!(
            first,
            DeferredCollectionRegManagerUpdate::Entered {
                active: DeferredCollectionRegActiveEffect::NoActiveRow,
                setup: DeferredCollectionRegSetupEffect::Activated {
                    registration_type: DeferredCollectionRegType::Simulation,
                    maximum: DMO_BANK_ASSET_COUNT,
                },
                retired: None,
            }
        ));
        assert_eq!(manager.pending_count(), 1);

        let wrong_updater = manager
            .try_update(
                DeferredCollectionRegType::Presentation,
                |_| -> Result<u32, Infallible> {
                    panic!("Presentation must not advance a Simulation row")
                },
                |_| -> Result<u32, Infallible> {
                    panic!("an incomplete Simulation row cannot run Setup")
                },
            )
            .unwrap();
        assert!(matches!(
            wrong_updater,
            DeferredCollectionRegManagerUpdate::Entered {
                active: DeferredCollectionRegActiveEffect::OtherRegistrationType {
                    active: DeferredCollectionRegType::Simulation,
                },
                setup: DeferredCollectionRegSetupEffect::ActiveIncomplete,
                retired: None,
            }
        ));

        for expected_index in 1..=10 {
            let update = manager
                .try_update(
                    DeferredCollectionRegType::Simulation,
                    advance_mixed_simulation,
                    setup_mixed_deferred_row,
                )
                .unwrap();
            assert!(matches!(
                update,
                DeferredCollectionRegManagerUpdate::Entered {
                    active: DeferredCollectionRegActiveEffect::Advanced {
                        registration_type: DeferredCollectionRegType::Simulation,
                        count: 12,
                        reached_maximum: false,
                    },
                    setup: DeferredCollectionRegSetupEffect::ActiveIncomplete,
                    retired: None,
                }
            ));
            assert_eq!(
                manager.active_snapshot().unwrap().index,
                expected_index * 12
            );
        }

        let terminal_handoff = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                advance_mixed_simulation,
                setup_mixed_deferred_row,
            )
            .unwrap();
        let retired_simulation = match terminal_handoff {
            DeferredCollectionRegManagerUpdate::Entered {
                active:
                    DeferredCollectionRegActiveEffect::Advanced {
                        registration_type: DeferredCollectionRegType::Simulation,
                        count: 5,
                        reached_maximum: true,
                    },
                setup:
                    DeferredCollectionRegSetupEffect::Activated {
                        registration_type: DeferredCollectionRegType::Presentation,
                        maximum: DMO_BANK_ASSET_COUNT,
                    },
                retired: Some(retired),
            } => retired,
            other => panic!("unexpected Simulation-to-Presentation handoff: {other:?}"),
        };
        assert_eq!(manager.pending_count(), 0);
        let active = manager.active_snapshot().unwrap();
        assert_eq!(
            active.registration_type,
            DeferredCollectionRegType::Presentation
        );
        assert_eq!(active.index, 0);

        let old_updater = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Infallible> {
                    panic!("Simulation must not advance a Presentation row")
                },
                |_| -> Result<u32, Infallible> {
                    panic!("an incomplete Presentation row cannot run Setup")
                },
            )
            .unwrap();
        assert!(matches!(
            old_updater,
            DeferredCollectionRegManagerUpdate::Entered {
                active: DeferredCollectionRegActiveEffect::OtherRegistrationType {
                    active: DeferredCollectionRegType::Presentation,
                },
                setup: DeferredCollectionRegSetupEffect::ActiveIncomplete,
                retired: None,
            }
        ));

        for expected_index in 1..=10 {
            let update = manager
                .try_update(
                    DeferredCollectionRegType::Presentation,
                    advance_mixed_presentation,
                    setup_mixed_deferred_row,
                )
                .unwrap();
            assert!(matches!(
                update,
                DeferredCollectionRegManagerUpdate::Entered {
                    active: DeferredCollectionRegActiveEffect::Advanced {
                        registration_type: DeferredCollectionRegType::Presentation,
                        count: 12,
                        reached_maximum: false,
                    },
                    setup: DeferredCollectionRegSetupEffect::ActiveIncomplete,
                    retired: None,
                }
            ));
            assert_eq!(
                manager.active_snapshot().unwrap().index,
                expected_index * 12
            );
        }
        let presentation_terminal = manager
            .try_update(
                DeferredCollectionRegType::Presentation,
                advance_mixed_presentation,
                setup_mixed_deferred_row,
            )
            .unwrap();
        assert!(matches!(
            presentation_terminal,
            DeferredCollectionRegManagerUpdate::Entered {
                active: DeferredCollectionRegActiveEffect::Advanced {
                    registration_type: DeferredCollectionRegType::Presentation,
                    count: 5,
                    reached_maximum: true,
                },
                setup: DeferredCollectionRegSetupEffect::PendingEmpty,
                retired: None,
            }
        ));
        let active = manager.active_snapshot().unwrap();
        assert_eq!(
            active.registration_type,
            DeferredCollectionRegType::Presentation
        );
        assert_eq!((active.index, active.maximum), (125, 125));

        let TestMixedDeferredPayload::Presentation(presentation) =
            manager.active_payload_mut().unwrap()
        else {
            panic!("completed manager row lost its Presentation producer")
        };
        let presentation = presentation
            .as_ref()
            .as_ref()
            .expect("completed manager row retains its Presentation producer");
        assert_eq!(presentation.next_arena_index(), DMO_BANK_ASSET_COUNT);
        assert_eq!(presentation.queued_marker_count(), 127);
        assert_eq!(
            presentation.marker_kind(124),
            Some(MainDmoPresentationDeferredMarkerKind::Arena {
                ordinal: 124,
                arena_id: CollectionId(CORPUS_ASSET_IDS[124]),
            })
        );
        assert_eq!(
            presentation.marker_kind(125),
            Some(
                MainDmoPresentationDeferredMarkerKind::ActivationZeroCollection {
                    collection_id: id,
                }
            )
        );
        assert_eq!(
            presentation.back_marker_kind(),
            Some(MainDmoPresentationDeferredMarkerKind::RegistrationCallback)
        );

        let TestMixedDeferredPayload::Simulation(simulation) = retired_simulation.payload() else {
            panic!("retired row lost its Simulation producer")
        };
        let simulation = simulation
            .as_ref()
            .as_ref()
            .expect("retired row retains its Simulation producer");
        let markers = simulation.queued_marker_kinds_for_test();
        assert_eq!(markers.len(), 127);
        assert_eq!(
            markers[124],
            MainDmoBankMarkerKind::DeferredArena { ordinal: 124 }
        );
        assert_eq!(markers[125], MainDmoBankMarkerKind::DeferredFinish);
        assert_eq!(markers[126], MainDmoBankMarkerKind::RegistrationCallback);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((manager, retired_simulation, wrapper, presentation_arenas));
        storage.destroy(collection);
    }

    #[test]
    fn dense_deferred_active_uses_live_raw_presentation_controls_once_per_update() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let collection_observer = lookup.attached_lookup_attempt_observer_for_test();
        let arena_observer = lookup
            .arena_lookup()
            .attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let producer = dense_deferred_producer(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let (producer, _) = producer
            .try_update_with(|| -> Result<_, std::convert::Infallible> {
                panic!("Setup must not sample deferred batch controls")
            })
            .unwrap();

        let mut samples = 0;
        let first_controls = DeferredCollectionBatchControls::new(1, 25, 5, 1).unwrap();
        let (producer, first) = producer
            .try_update_with(|| {
                samples += 1;
                Ok::<_, std::convert::Infallible>(first_controls)
            })
            .unwrap();
        assert_eq!(
            first,
            MainDmoBankDenseDeferredProducerUpdate::ArenasQueued { count: 25 }
        );
        assert_eq!(samples, 1);
        assert_eq!(producer.next_ordinal_for_test(), 25);
        assert_eq!(
            producer.queued_marker_kinds_for_test(),
            (0..25)
                .map(|ordinal| MainDmoBankMarkerKind::DeferredArena { ordinal })
                .collect::<Vec<_>>()
        );

        let second_controls = DeferredCollectionBatchControls::new(1, 100, 10, 2).unwrap();
        let (producer, second) = producer
            .try_update_with(|| {
                samples += 1;
                Ok::<_, std::convert::Infallible>(second_controls)
            })
            .unwrap();
        assert_eq!(
            second,
            MainDmoBankDenseDeferredProducerUpdate::ArenasQueued { count: 12 }
        );
        assert_eq!(samples, 2);
        assert_eq!(producer.next_ordinal_for_test(), 37);
        assert_eq!(
            producer.queued_marker_kinds_for_test(),
            (0..37)
                .map(|ordinal| MainDmoBankMarkerKind::DeferredArena { ordinal })
                .collect::<Vec<_>>()
        );
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 0);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((producer, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn dense_deferred_defaults_queue_ten_twelves_then_five_and_terminal_suffix() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let collection_observer = lookup.attached_lookup_attempt_observer_for_test();
        let arena_observer = lookup
            .arena_lookup()
            .attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let producer = dense_deferred_producer(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let (mut producer, setup) = producer
            .try_update_with(|| -> Result<_, std::convert::Infallible> {
                panic!("Setup must not sample deferred batch controls")
            })
            .unwrap();
        assert_eq!(
            setup,
            MainDmoBankDenseDeferredProducerUpdate::SetupCompleted
        );

        let mut samples = 0;
        for update_index in 0..10 {
            let (next, update) = producer
                .try_update_with(|| {
                    samples += 1;
                    Ok::<_, std::convert::Infallible>(DeferredCollectionBatchControls::default())
                })
                .unwrap();
            producer = next;
            assert_eq!(
                update,
                MainDmoBankDenseDeferredProducerUpdate::ArenasQueued { count: 12 }
            );
            assert_eq!(producer.next_ordinal_for_test(), (update_index + 1) * 12);
            assert_eq!(producer.queued_markers_for_test(), (update_index + 1) * 12);
            assert_eq!(
                producer.stage_for_test(),
                MainDmoBankDenseDeferredProducerStage::Active
            );
        }
        let (producer, terminal) = producer
            .try_update_with(|| {
                samples += 1;
                Ok::<_, std::convert::Infallible>(DeferredCollectionBatchControls::default())
            })
            .unwrap();
        assert_eq!(
            terminal,
            MainDmoBankDenseDeferredProducerUpdate::TerminalQueued { arena_count: 5 }
        );
        assert_eq!(samples, 11);
        assert_eq!(
            producer.stage_for_test(),
            MainDmoBankDenseDeferredProducerStage::Terminal
        );
        assert_eq!(producer.next_ordinal_for_test(), 125);
        let mut expected = (0..125)
            .map(|ordinal| MainDmoBankMarkerKind::DeferredArena { ordinal })
            .collect::<Vec<_>>();
        expected.push(MainDmoBankMarkerKind::DeferredFinish);
        expected.push(MainDmoBankMarkerKind::RegistrationCallback);
        assert_eq!(producer.queued_marker_kinds_for_test(), expected);

        let (producer, idle) = producer
            .try_update_with(|| -> Result<_, std::convert::Infallible> {
                samples += 1;
                panic!("Terminal must not resample deferred batch controls")
            })
            .unwrap();
        assert_eq!(idle, MainDmoBankDenseDeferredProducerUpdate::TerminalIdle);
        assert_eq!(samples, 11);
        assert_eq!(producer.queued_marker_kinds_for_test(), expected);
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 0);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((producer, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn dense_deferred_reserve_refusal_retains_active_owner_for_exact_retry() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let collection_observer = lookup.attached_lookup_attempt_observer_for_test();
        let arena_observer = lookup
            .arena_lookup()
            .attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let producer = dense_deferred_producer(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let (mut producer, _) = producer
            .try_update_with(|| -> Result<_, std::convert::Infallible> {
                panic!("Setup must not sample deferred batch controls")
            })
            .unwrap();
        assert!(producer.fail_next_queue_reserve_for_test());
        let mut samples = 0;
        let Err(failure) = producer.try_update_with(|| {
            samples += 1;
            Ok::<_, std::convert::Infallible>(DeferredCollectionBatchControls::default())
        }) else {
            panic!("injected dense queue reserve refusal unexpectedly succeeded")
        };
        assert!(matches!(
            failure.error(),
            MainDmoBankDenseDeferredProducerUpdateError::Queue(
                MainDmoBankMarkerEnqueueRejectionKind::QueueStorageAllocation
            )
        ));
        assert_eq!(samples, 1);
        assert_eq!(
            failure.stage_for_test(),
            MainDmoBankDenseDeferredProducerStage::Active
        );
        assert_eq!(failure.next_ordinal_for_test(), 0);
        assert_eq!(failure.last_reservation_size_for_test(), Some(12));
        assert!(failure.queued_marker_kinds_for_test().is_empty());
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 0);
        assert!(events.lock().unwrap().is_empty());

        let (producer, update) = failure
            .retry_with(|| {
                samples += 1;
                Ok::<_, std::convert::Infallible>(DeferredCollectionBatchControls::default())
            })
            .unwrap();
        assert_eq!(
            update,
            MainDmoBankDenseDeferredProducerUpdate::ArenasQueued { count: 12 }
        );
        assert_eq!(samples, 2);
        assert_eq!(producer.next_ordinal_for_test(), 12);
        assert_eq!(
            producer.queued_marker_kinds_for_test(),
            (0..12)
                .map(|ordinal| MainDmoBankMarkerKind::DeferredArena { ordinal })
                .collect::<Vec<_>>()
        );
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 0);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((producer, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn dense_deferred_terminal_reserve_refusal_preserves_entire_suffix_for_retry() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let collection_observer = lookup.attached_lookup_attempt_observer_for_test();
        let arena_observer = lookup
            .arena_lookup()
            .attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let producer = dense_deferred_producer(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        );
        let (mut producer, _) = producer
            .try_update_with(|| -> Result<_, std::convert::Infallible> {
                panic!("Setup must not sample deferred batch controls")
            })
            .unwrap();
        for _ in 0..10 {
            let (next, update) = producer
                .try_update_with(|| {
                    Ok::<_, std::convert::Infallible>(DeferredCollectionBatchControls::default())
                })
                .unwrap();
            producer = next;
            assert_eq!(
                update,
                MainDmoBankDenseDeferredProducerUpdate::ArenasQueued { count: 12 }
            );
        }
        let prefix = (0..120)
            .map(|ordinal| MainDmoBankMarkerKind::DeferredArena { ordinal })
            .collect::<Vec<_>>();
        assert_eq!(producer.next_ordinal_for_test(), 120);
        assert_eq!(producer.queued_marker_kinds_for_test(), prefix);
        assert!(producer.fail_next_queue_reserve_for_test());

        let mut samples = 0;
        let Err(failure) = producer.try_update_with(|| {
            samples += 1;
            Ok::<_, std::convert::Infallible>(DeferredCollectionBatchControls::default())
        }) else {
            panic!("injected terminal reserve refusal unexpectedly succeeded")
        };
        assert!(matches!(
            failure.error(),
            MainDmoBankDenseDeferredProducerUpdateError::Queue(
                MainDmoBankMarkerEnqueueRejectionKind::QueueStorageAllocation
            )
        ));
        assert_eq!(samples, 1);
        assert_eq!(
            failure.stage_for_test(),
            MainDmoBankDenseDeferredProducerStage::Active
        );
        assert_eq!(failure.next_ordinal_for_test(), 120);
        assert_eq!(failure.last_reservation_size_for_test(), Some(7));
        assert_eq!(failure.queued_marker_kinds_for_test(), prefix);
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 0);
        assert!(events.lock().unwrap().is_empty());

        let (producer, update) = failure
            .retry_with(|| {
                samples += 1;
                Ok::<_, std::convert::Infallible>(DeferredCollectionBatchControls::default())
            })
            .unwrap();
        assert_eq!(
            update,
            MainDmoBankDenseDeferredProducerUpdate::TerminalQueued { arena_count: 5 }
        );
        assert_eq!(samples, 2);
        assert_eq!(
            producer.stage_for_test(),
            MainDmoBankDenseDeferredProducerStage::Terminal
        );
        assert_eq!(producer.next_ordinal_for_test(), 125);
        let mut expected = (0..125)
            .map(|ordinal| MainDmoBankMarkerKind::DeferredArena { ordinal })
            .collect::<Vec<_>>();
        expected.push(MainDmoBankMarkerKind::DeferredFinish);
        expected.push(MainDmoBankMarkerKind::RegistrationCallback);
        assert_eq!(producer.queued_marker_kinds_for_test(), expected);
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 0);
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(id, &token).unwrap();
        drop((producer, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn published_deferred_ready_looks_up_each_arena_and_revalidates_collection_at_terminals() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut storage, collection) = collection(assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let arena_lookup_observer = lookup
            .arena_lookup()
            .attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestResidentRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let ready = deferred_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        )
        .try_setup()
        .unwrap();
        assert_eq!(lookup_observer.count(), 1);
        assert_eq!(arena_lookup_observer.count(), 0);
        let mut driver = ready.into_dense_queued_registration_driver_for_test();
        assert_eq!(arena_lookup_observer.count(), 0);
        for ordinal in 0..125 {
            assert!(matches!(
                driver.dispatch_one(harness.coordinator()).unwrap(),
                MainDmoBankReadyTestDispatch::ReceiverMarkerCompleted(
                    MainDmoBankMarkerKind::DeferredArena { ordinal: actual }
                ) if actual == ordinal
            ));
            assert_eq!(lookup_observer.count(), 1);
            assert_eq!(arena_lookup_observer.count(), ordinal + 1);
        }
        assert_eq!(
            driver.stage_for_test(),
            MainDmoBankNotificationStage::DeferredFinish
        );
        assert_eq!(events.lock().unwrap().len(), 218);
        assert!(matches!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoBankReadyTestDispatch::ReceiverMarkerCompleted(
                MainDmoBankMarkerKind::DeferredFinish
            )
        ));
        assert_eq!(lookup_observer.count(), 2);
        assert_eq!(arena_lookup_observer.count(), 125);
        assert_eq!(events.lock().unwrap().len(), 343);
        assert!(matches!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoBankReadyTestDispatch::RegistrationAcknowledged
        ));
        assert_eq!(lookup_observer.count(), 2);
        assert_eq!(arena_lookup_observer.count(), 125);

        let callback = harness.advance_and_take_simulation_deregistration_callback();
        driver.queue_deregistration(callback);
        assert_eq!(lookup_observer.count(), 2);
        let before_dreg = events.lock().unwrap().len();
        assert!(matches!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoBankReadyTestDispatch::ReceiverMarkerCompleted(
                MainDmoBankMarkerKind::DeregistrationCollection
            )
        ));
        assert_eq!(lookup_observer.count(), 3);
        assert_eq!(arena_lookup_observer.count(), 125);
        assert_eq!(events.lock().unwrap().len() - before_dreg, 343);
        assert_eq!(driver.live_registration_count_for_test(), 0);
        assert!(matches!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoBankReadyTestDispatch::DeregistrationCallbackReady
        ));
        assert_eq!(lookup_observer.count(), 3);
        assert_eq!(arena_lookup_observer.count(), 125);

        registry.remove_exact(id, &token).unwrap();
        drop((driver, wrapper));
        storage.destroy(collection);
    }

    #[test]
    fn published_first_deferred_arena_rejects_foreign_outer_owner_with_same_plan() {
        let (assets, plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut main_storage, main_collection) = collection(assets);
        let shadow_asset =
            DmoBankProcessedArenaAsset::from_parts_for_test(Arc::clone(&plans[0]), Vec::new());
        let (mut shadow_storage, shadow_collection) = collection(vec![shadow_asset]);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let collection_observer = lookup.attached_lookup_attempt_observer_for_test();
        let arena_observer = lookup
            .arena_lookup()
            .attached_lookup_attempt_observer_for_test();
        let shadow_id = CollectionId(0xfeed_face_cafe_beef);
        let shadow_wrapper = registry
            .create_wrapper(shadow_id, &shadow_collection)
            .unwrap();
        let shadow_token = TestResidentRegistry::stable_token(&shadow_wrapper);
        registry.publish(shadow_id, &shadow_token).unwrap();
        let main_id = CollectionId(DMO_BANK_COLLECTION_ID);
        let main_wrapper = registry.create_wrapper(main_id, &main_collection).unwrap();
        let main_token = TestResidentRegistry::stable_token(&main_wrapper);
        registry.publish(main_id, &main_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let ready = deferred_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        )
        .try_setup()
        .unwrap();
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 0);
        let mut driver = ready.into_dense_queued_registration_driver_for_test();
        assert_eq!(arena_observer.count(), 0);
        let expected =
            MainDmoBankMarkerDispatchError::ReceiverFaulted(MainDmoBankMarkerKind::DeferredArena {
                ordinal: 0,
            });
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 1);
        assert_eq!(
            driver.arena_revalidation_fault_for_test(),
            Some(MainDmoBankPublishedArenaRevalidationError::RetainedOwnerMismatch)
        );
        assert_eq!(
            driver.stage_for_test(),
            MainDmoBankNotificationStage::DeferredArena
        );
        assert!(driver.registration_callback_retained_for_test());
        assert_eq!(driver.live_registration_count_for_test(), 1);
        assert_eq!(driver.queue_for_test().len(), 127);
        assert_eq!(
            driver.queue_for_test().first(),
            Some(&MainDmoBankMarkerKind::DeferredArena { ordinal: 0 })
        );
        assert!(events.lock().unwrap().is_empty());

        registry.remove_exact(shadow_id, &shadow_token).unwrap();
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(arena_observer.count(), 1);
        assert!(events.lock().unwrap().is_empty());
        registry.remove_exact(main_id, &main_token).unwrap();
        drop((driver, shadow_wrapper, main_wrapper));
        shadow_storage.destroy(shadow_collection);
        main_storage.destroy(main_collection);
    }

    #[test]
    fn published_middle_deferred_arena_missing_preserves_exact_prefix_and_sticks() {
        const TARGET: usize = 62;

        let (assets, plans) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut main_storage, main_collection) = collection(assets);
        let shadow_asset =
            DmoBankProcessedArenaAsset::from_parts_for_test(Arc::clone(&plans[TARGET]), Vec::new());
        let (mut shadow_storage, shadow_collection) = collection(vec![shadow_asset]);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let collection_observer = lookup.attached_lookup_attempt_observer_for_test();
        let arena_observer = lookup
            .arena_lookup()
            .attached_lookup_attempt_observer_for_test();
        let shadow_id = CollectionId(0xfeed_face_cafe_beee);
        let shadow_wrapper = registry
            .create_wrapper(shadow_id, &shadow_collection)
            .unwrap();
        let shadow_token = TestResidentRegistry::stable_token(&shadow_wrapper);
        registry.publish(shadow_id, &shadow_token).unwrap();
        let main_id = CollectionId(DMO_BANK_COLLECTION_ID);
        let main_wrapper = registry.create_wrapper(main_id, &main_collection).unwrap();
        let main_token = TestResidentRegistry::stable_token(&main_wrapper);
        registry.publish(main_id, &main_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let ready = deferred_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        )
        .try_setup()
        .unwrap();
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), 0);
        let mut driver = ready.into_dense_queued_registration_driver_for_test();
        assert_eq!(arena_observer.count(), 0);

        for ordinal in 0..TARGET {
            assert_eq!(
                driver.dispatch_one(harness.coordinator()).unwrap(),
                MainDmoBankReadyTestDispatch::ReceiverMarkerCompleted(
                    MainDmoBankMarkerKind::DeferredArena { ordinal }
                )
            );
        }
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), TARGET);
        let prefix = events.lock().unwrap().clone();
        let mut expected_prefix = Vec::with_capacity(TARGET * 2);
        for ordinal in 0..TARGET {
            expected_prefix.push(CoordinatorEvent::Add(
                0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
            ));
            expected_prefix.push(CoordinatorEvent::LocationRegister(ordinal));
        }
        assert_eq!(prefix, expected_prefix);
        assert!(!driver.has_model_for_test(0x2000_0000_0000_0000 | u64::try_from(TARGET).unwrap()));

        registry.remove_exact(shadow_id, &shadow_token).unwrap();
        let expected =
            MainDmoBankMarkerDispatchError::ReceiverFaulted(MainDmoBankMarkerKind::DeferredArena {
                ordinal: TARGET,
            });
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(collection_observer.count(), 1);
        assert_eq!(arena_observer.count(), TARGET + 1);
        assert_eq!(
            driver.arena_revalidation_fault_for_test(),
            Some(MainDmoBankPublishedArenaRevalidationError::MissingArena)
        );
        assert_eq!(
            driver.stage_for_test(),
            MainDmoBankNotificationStage::DeferredArena
        );
        assert!(driver.registration_callback_retained_for_test());
        assert_eq!(driver.live_registration_count_for_test(), 1);
        assert_eq!(driver.queue_for_test().len(), 127 - TARGET);
        assert_eq!(
            driver.queue_for_test().first(),
            Some(&MainDmoBankMarkerKind::DeferredArena { ordinal: TARGET })
        );
        assert_eq!(*events.lock().unwrap(), prefix);

        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(arena_observer.count(), TARGET + 1);
        assert_eq!(*events.lock().unwrap(), expected_prefix);
        registry.remove_exact(main_id, &main_token).unwrap();
        drop((driver, shadow_wrapper, main_wrapper));
        shadow_storage.destroy(shadow_collection);
        main_storage.destroy(main_collection);
    }

    #[test]
    fn published_deferred_finish_stale_latches_front_before_receiver_work() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut first_storage, first_collection) = collection(first_assets);
        let (mut second_storage, second_collection) = collection(second_assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first_collection).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let ready = deferred_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        )
        .try_setup()
        .unwrap();
        let mut driver = ready.into_dense_queued_registration_driver_for_test();
        for _ in 0..125 {
            driver.dispatch_one(harness.coordinator()).unwrap();
        }
        assert_eq!(lookup_observer.count(), 1);
        assert_eq!(
            driver.stage_for_test(),
            MainDmoBankNotificationStage::DeferredFinish
        );
        assert!(driver.registration_callback_retained_for_test());
        assert_eq!(driver.live_registration_count_for_test(), 1);
        let prefix = events.lock().unwrap().clone();
        assert_eq!(prefix.len(), 218);

        registry.remove_exact(id, &first_token).unwrap();
        let second_wrapper = registry.create_wrapper(id, &second_collection).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();
        let expected =
            MainDmoBankMarkerDispatchError::ReceiverFaulted(MainDmoBankMarkerKind::DeferredFinish);
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(lookup_observer.count(), 2);
        assert_eq!(
            driver.revalidation_fault_for_test(),
            Some(MainDmoBankPublishedAcquisitionError::StaleOccurrence)
        );
        assert_eq!(
            driver.stage_for_test(),
            MainDmoBankNotificationStage::DeferredFinish
        );
        assert!(driver.registration_callback_retained_for_test());
        assert_eq!(driver.live_registration_count_for_test(), 1);
        assert_eq!(
            driver.queue_for_test(),
            [
                MainDmoBankMarkerKind::DeferredFinish,
                MainDmoBankMarkerKind::RegistrationCallback,
            ]
        );
        assert_eq!(*events.lock().unwrap(), prefix);

        registry.remove_exact(id, &second_token).unwrap();
        drop(registry);
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(lookup_observer.count(), 2);
        assert_eq!(*events.lock().unwrap(), prefix);
        drop((driver, first_wrapper, second_wrapper));
        first_storage.destroy(first_collection);
        second_storage.destroy(second_collection);
    }

    #[test]
    fn published_deferred_dreg_stale_latches_before_any_teardown() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut first_storage, first_collection) = collection(first_assets);
        let (mut second_storage, second_collection) = collection(second_assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first_collection).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let ready = deferred_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        )
        .try_setup()
        .unwrap();
        let mut driver = ready.into_dense_queued_registration_driver_for_test();
        for _ in 0..126 {
            driver.dispatch_one(harness.coordinator()).unwrap();
        }
        assert_eq!(lookup_observer.count(), 2);
        assert!(matches!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoBankReadyTestDispatch::RegistrationAcknowledged
        ));
        let callback = harness.advance_and_take_simulation_deregistration_callback();
        driver.queue_deregistration(callback);
        assert_eq!(
            driver.stage_for_test(),
            MainDmoBankNotificationStage::DeregistrationCollection
        );
        assert!(driver.deregistration_callback_retained_for_test());
        assert_eq!(driver.live_registration_count_for_test(), 1);
        let registered = events.lock().unwrap().clone();
        assert_eq!(registered.len(), 343);

        registry.remove_exact(id, &first_token).unwrap();
        let second_wrapper = registry.create_wrapper(id, &second_collection).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();
        let expected = MainDmoBankMarkerDispatchError::ReceiverFaulted(
            MainDmoBankMarkerKind::DeregistrationCollection,
        );
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(lookup_observer.count(), 3);
        assert_eq!(
            driver.revalidation_fault_for_test(),
            Some(MainDmoBankPublishedAcquisitionError::StaleOccurrence)
        );
        assert_eq!(
            driver.stage_for_test(),
            MainDmoBankNotificationStage::DeregistrationCollection
        );
        assert!(driver.deregistration_callback_retained_for_test());
        assert_eq!(driver.live_registration_count_for_test(), 1);
        assert_eq!(
            driver.queue_for_test(),
            [
                MainDmoBankMarkerKind::DeregistrationCollection,
                MainDmoBankMarkerKind::DeregistrationCallback,
            ]
        );
        assert_eq!(*events.lock().unwrap(), registered);

        registry.remove_exact(id, &second_token).unwrap();
        drop(registry);
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(lookup_observer.count(), 3);
        assert_eq!(*events.lock().unwrap(), registered);
        drop((driver, first_wrapper, second_wrapper));
        first_storage.destroy(first_collection);
        second_storage.destroy(second_collection);
    }

    #[test]
    fn published_immediate_dreg_uses_the_same_sticky_front_revalidation() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut first_storage, first_collection) = collection(first_assets);
        let (mut second_storage, second_collection) = collection(second_assets);
        let (mut registry, lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let lookup_observer = lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first_wrapper = registry.create_wrapper(id, &first_collection).unwrap();
        let first_token = TestResidentRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let ready = immediate_pending(
            lookup,
            &events,
            harness.take_simulation_registration_callback(),
        )
        .dispatch_one()
        .unwrap();
        assert_eq!(lookup_observer.count(), 1);
        let mut driver = ready.into_queued_registration_driver_for_test();
        assert!(matches!(
            driver.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoBankReadyTestDispatch::RegistrationAcknowledged
        ));
        assert_eq!(lookup_observer.count(), 1);
        let callback = harness.advance_and_take_simulation_deregistration_callback();
        driver.queue_deregistration(callback);
        let registered = events.lock().unwrap().clone();
        assert_eq!(registered.len(), 343);

        registry.remove_exact(id, &first_token).unwrap();
        let second_wrapper = registry.create_wrapper(id, &second_collection).unwrap();
        let second_token = TestResidentRegistry::stable_token(&second_wrapper);
        registry.publish(id, &second_token).unwrap();
        let expected = MainDmoBankMarkerDispatchError::ReceiverFaulted(
            MainDmoBankMarkerKind::DeregistrationCollection,
        );
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(lookup_observer.count(), 2);
        assert_eq!(
            driver.revalidation_fault_for_test(),
            Some(MainDmoBankPublishedAcquisitionError::StaleOccurrence)
        );
        assert_eq!(
            driver.stage_for_test(),
            MainDmoBankNotificationStage::DeregistrationCollection
        );
        assert!(driver.deregistration_callback_retained_for_test());
        assert_eq!(driver.live_registration_count_for_test(), 1);
        assert_eq!(*events.lock().unwrap(), registered);
        registry.remove_exact(id, &second_token).unwrap();
        drop(registry);
        assert_eq!(
            driver.dispatch_one(harness.coordinator()).unwrap_err(),
            expected
        );
        assert_eq!(lookup_observer.count(), 2);
        assert_eq!(*events.lock().unwrap(), registered);

        drop((driver, first_wrapper, second_wrapper));
        first_storage.destroy(first_collection);
        second_storage.destroy(second_collection);
    }

    #[test]
    fn published_wrong_stage_refusals_precede_revalidation_lookup() {
        let (deferred_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut deferred_storage, deferred_collection) = collection(deferred_assets);
        let (mut deferred_registry, deferred_lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let deferred_observer = deferred_lookup.attached_lookup_attempt_observer_for_test();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let deferred_wrapper = deferred_registry
            .create_wrapper(id, &deferred_collection)
            .unwrap();
        let deferred_token = TestResidentRegistry::stable_token(&deferred_wrapper);
        deferred_registry.publish(id, &deferred_token).unwrap();
        let deferred_events = Arc::new(Mutex::new(Vec::new()));
        let mut deferred_harness = MainDmoDregIntegrationHarness::new();
        let ready = deferred_pending(
            deferred_lookup,
            &deferred_events,
            deferred_harness.take_simulation_registration_callback(),
        )
        .try_setup()
        .unwrap();
        let mut deferred_driver = ready.into_dense_queued_registration_driver_for_test();
        assert_eq!(deferred_observer.count(), 1);
        assert!(deferred_driver.probe_deferred_finish_wrong_stage_for_test());
        assert_eq!(deferred_observer.count(), 1);
        assert_eq!(
            deferred_driver.stage_for_test(),
            MainDmoBankNotificationStage::DeferredArena
        );
        assert!(deferred_driver.registration_callback_retained_for_test());
        assert!(deferred_events.lock().unwrap().is_empty());

        deferred_registry.remove_exact(id, &deferred_token).unwrap();
        drop((deferred_driver, deferred_wrapper));
        deferred_storage.destroy(deferred_collection);

        let (immediate_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (mut immediate_storage, immediate_collection) = collection(immediate_assets);
        let (mut immediate_registry, immediate_lookup): (TestResidentRegistry, _) =
            SimulationResidentRegistry::new(identity_registry_error);
        let immediate_observer = immediate_lookup.attached_lookup_attempt_observer_for_test();
        let immediate_arena_observer = immediate_lookup
            .arena_lookup()
            .attached_lookup_attempt_observer_for_test();
        let immediate_wrapper = immediate_registry
            .create_wrapper(id, &immediate_collection)
            .unwrap();
        let immediate_token = TestResidentRegistry::stable_token(&immediate_wrapper);
        immediate_registry.publish(id, &immediate_token).unwrap();
        let immediate_events = Arc::new(Mutex::new(Vec::new()));
        let mut immediate_harness = MainDmoDregIntegrationHarness::new();
        let ready = immediate_pending(
            immediate_lookup,
            &immediate_events,
            immediate_harness.take_simulation_registration_callback(),
        )
        .dispatch_one()
        .unwrap();
        let mut immediate_driver = ready.into_queued_registration_driver_for_test();
        assert_eq!(immediate_observer.count(), 1);
        assert_eq!(immediate_arena_observer.count(), 0);
        assert!(immediate_driver.probe_deferred_arena_wrong_stage_for_test(0));
        assert_eq!(immediate_arena_observer.count(), 0);
        assert!(immediate_driver.probe_deregistration_collection_wrong_stage_for_test());
        assert_eq!(immediate_observer.count(), 1);
        assert_eq!(
            immediate_driver.stage_for_test(),
            MainDmoBankNotificationStage::RegistrationCallback
        );
        assert!(immediate_driver.registration_callback_retained_for_test());
        assert_eq!(immediate_events.lock().unwrap().len(), 343);

        immediate_registry
            .remove_exact(id, &immediate_token)
            .unwrap();
        drop((immediate_driver, immediate_wrapper));
        immediate_storage.destroy(immediate_collection);
    }

    #[test]
    fn coordinator_active_and_deregister_preserve_both_restarted_forward_passes() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut coordinator = coordinator(binding, &events);

        let registration = coordinator.register_active().unwrap();
        let mut expected = Vec::new();
        for ordinal in 0..125 {
            if ordinal < 93 {
                expected.push(CoordinatorEvent::Add(
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                ));
            }
            expected.push(CoordinatorEvent::LocationRegister(ordinal));
        }
        expected.extend((0..125).map(CoordinatorEvent::DmoRegister));
        assert_eq!(*events.lock().unwrap(), expected);
        assert_eq!(coordinator.live_registration_count(), 1);
        assert!(coordinator.has_model(0x2000_0000_0000_0000));
        assert!(coordinator.has_model(0x2000_0000_0000_005c));

        events.lock().unwrap().clear();
        coordinator.deregister(registration).unwrap();
        let mut expected = (0..125)
            .map(CoordinatorEvent::DmoDeregister)
            .collect::<Vec<_>>();
        for ordinal in 0..125 {
            if ordinal < 93 {
                expected.push(CoordinatorEvent::Remove(
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                ));
            }
            expected.push(CoordinatorEvent::LocationDeregister(ordinal));
        }
        assert_eq!(*events.lock().unwrap(), expected);
        assert_eq!(coordinator.live_registration_count(), 0);
        assert!(!coordinator.has_model(0x2000_0000_0000_0000));
        assert!(!coordinator.has_model(0x2000_0000_0000_005c));
    }

    #[test]
    fn coordinator_deferred_keeps_one_cursor_and_finishes_before_dmo_pass() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut coordinator = coordinator(binding, &events);

        let deferred = coordinator.begin_deferred().unwrap();
        assert!(events.lock().unwrap().is_empty());
        let error = coordinator.finish_deferred(deferred).unwrap_err();
        assert!(matches!(
            &error,
            MainDmoBankDeferredFinishError::Validation {
                source: SimulationCollectionDeferredFinishValidationError::Incomplete {
                    completed: 0,
                    total: 125
                },
                ..
            }
        ));
        let mut deferred = error.into_registration().unwrap();

        for ordinal in 0..125 {
            let step = coordinator.register_next_deferred_arena(deferred).unwrap();
            assert_eq!(step.is_prefix_complete(), ordinal == 124);
            deferred = step.into_registration();
        }
        let mut expected = Vec::new();
        for ordinal in 0..125 {
            if ordinal < 93 {
                expected.push(CoordinatorEvent::Add(
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                ));
            }
            expected.push(CoordinatorEvent::LocationRegister(ordinal));
        }
        assert_eq!(*events.lock().unwrap(), expected);

        let registration = coordinator.finish_deferred(deferred).unwrap();
        expected.extend((0..125).map(CoordinatorEvent::DmoRegister));
        assert_eq!(*events.lock().unwrap(), expected);
        assert_eq!(coordinator.live_registration_count(), 1);
        coordinator.deregister(registration).unwrap();
        assert_eq!(coordinator.live_registration_count(), 0);
    }

    #[test]
    fn coordinator_foreign_deferred_and_complete_authorities_are_recoverable() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_first_storage, first_collection) = collection(first_assets);
        let (_second_storage, second_collection) = collection(second_assets);
        let first_binding =
            PreparedMainDmoBankSimulationCollection::try_new(&first_collection).unwrap();
        let second_binding =
            PreparedMainDmoBankSimulationCollection::try_new(&second_collection).unwrap();
        let first_events = Arc::new(Mutex::new(Vec::new()));
        let second_events = Arc::new(Mutex::new(Vec::new()));
        let mut first = coordinator(first_binding, &first_events);
        let mut second = coordinator(second_binding, &second_events);

        let deferred = first.begin_deferred().unwrap();
        let error = second.register_next_deferred_arena(deferred).unwrap_err();
        assert!(matches!(
            &error,
            MainDmoBankDeferredStepError::Collection {
                source: SimulationCollectionDeferredStepError::ForeignDomain(_),
                ..
            }
        ));
        let deferred = error.into_registration().unwrap();
        assert!(second_events.lock().unwrap().is_empty());
        let mut deferred = first
            .register_next_deferred_arena(deferred)
            .unwrap()
            .into_registration();
        for _ in 1..125 {
            deferred = first
                .register_next_deferred_arena(deferred)
                .unwrap()
                .into_registration();
        }
        let registration = first.finish_deferred(deferred).unwrap();

        let error = second.deregister(registration).unwrap_err();
        assert!(matches!(
            &error,
            MainDmoBankCollectionDeregistrationError::Collection(
                SimulationCollectionDeregistrationError::ForeignDomain(_)
            )
        ));
        let registration = error.into_registration().unwrap();
        first.deregister(registration).unwrap();
        assert_eq!(first.live_registration_count(), 0);
    }

    #[test]
    fn coordinator_resident_expiry_occurs_after_add_without_suffix_or_rollback() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        collection.tombstone_without_releasing_assets_for_test();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut coordinator = coordinator(binding, &events);

        let error = coordinator.register_active().unwrap_err();
        assert!(matches!(
            error,
            MainDmoBankCollectionRegistrationError::Controller {
                arena_index: 0,
                source: SimulationDmoBankModelRegisterPassError::Source(
                    SimulationDmoBankModelRegistrationPassError::ResidentExpired
                )
            }
        ));
        assert_eq!(
            *events.lock().unwrap(),
            [CoordinatorEvent::Add(0x1000_0000_0000_0000)]
        );
        assert_eq!(coordinator.live_registration_count(), 1);
        assert!(!coordinator.has_model(0x2000_0000_0000_0000));
        assert!(matches!(
            coordinator.register_active(),
            Err(MainDmoBankCollectionRegistrationError::ControllerFaulted(
                MainDmoBankCollectionControllerFault::RegistrationArena { arena_index: 0 }
            ))
        ));
    }

    #[test]
    fn coordinator_deregistration_samples_expiry_after_complete_dmo_pass_and_remove() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut coordinator = coordinator(binding, &events);
        let registration = coordinator.register_active().unwrap();
        assert!(coordinator.has_model(0x2000_0000_0000_0000));
        events.lock().unwrap().clear();
        collection.tombstone_without_releasing_assets_for_test();

        let error = coordinator.deregister(registration).unwrap_err();
        assert!(matches!(
            &error,
            MainDmoBankCollectionDeregistrationError::Controller {
                arena_index: 0,
                source: SimulationDmoBankModelDeregisterPassError::Source(
                    SimulationDmoBankModelRegistrationPassError::ResidentExpired
                )
            }
        ));
        assert!(error.into_registration().is_none());
        let mut expected = (0..125)
            .map(CoordinatorEvent::DmoDeregister)
            .collect::<Vec<_>>();
        expected.push(CoordinatorEvent::Remove(0x1000_0000_0000_0000));
        assert_eq!(*events.lock().unwrap(), expected);
        assert!(coordinator.has_model(0x2000_0000_0000_0000));
        assert_eq!(coordinator.live_registration_count(), 1);
        assert!(matches!(
            coordinator.register_active(),
            Err(MainDmoBankCollectionRegistrationError::ControllerFaulted(
                MainDmoBankCollectionControllerFault::DeregistrationArena { arena_index: 0 }
            ))
        ));
    }

    #[test]
    fn coordinator_repeated_occurrences_keep_independent_cursors_with_duplicate_model_key() {
        let (_storage, collection) = collection(corpus_assets_with_shared_first_model());
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut coordinator = coordinator(binding, &events);

        let first = coordinator.register_active().unwrap();
        let second = coordinator.register_active().unwrap();
        assert_eq!(coordinator.live_registration_count(), 2);
        assert!(coordinator.has_model(0x2000_0000_0000_0000));

        coordinator.deregister(first).unwrap();
        assert_eq!(coordinator.live_registration_count(), 1);
        assert!(!coordinator.has_model(0x2000_0000_0000_0000));

        coordinator.deregister(second).unwrap();
        assert_eq!(coordinator.live_registration_count(), 0);
        assert!(!coordinator.has_model(0x2000_0000_0000_0000));
        assert_eq!(
            events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| {
                    matches!(event, CoordinatorEvent::Add(asset_id) if
                        *asset_id == 0x1000_0000_0000_0000
                            || *asset_id == 0x1000_0000_0000_0001)
                })
                .count(),
            4
        );
    }

    #[test]
    fn notification_immediate_and_dreg_callbacks_follow_separate_receiver_boundaries() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver: TestNotificationReceiver<u32, u64> =
            MainDmoBankNotificationReceiver::new_immediate(
                coordinator(binding, &events),
                0x1357_9bdf,
            );

        let rejection = receiver
            .begin_deregistration(0x0123_4567_89ab_cdef)
            .unwrap_err();
        assert_eq!(
            rejection.stage().expected(),
            MainDmoBankNotificationStage::Registered
        );
        assert_eq!(
            rejection.stage().actual(),
            MainDmoBankNotificationStage::ImmediateCollection
        );
        let deregistration_callback = rejection.into_callback();
        let error = receiver.receive_registration_callback().unwrap_err();
        assert_eq!(
            (error.expected(), error.actual()),
            (
                MainDmoBankNotificationStage::RegistrationCallback,
                MainDmoBankNotificationStage::ImmediateCollection,
            )
        );

        receiver.receive_immediate_collection().unwrap();
        assert_eq!(
            receiver.stage_for_test(),
            MainDmoBankNotificationStage::RegistrationCallback
        );
        let mut expected = Vec::new();
        for ordinal in 0..125 {
            if ordinal < 93 {
                expected.push(CoordinatorEvent::Add(
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                ));
            }
            expected.push(CoordinatorEvent::LocationRegister(ordinal));
        }
        expected.extend((0..125).map(CoordinatorEvent::DmoRegister));
        assert_eq!(*events.lock().unwrap(), expected);
        assert_eq!(receiver.live_registration_count_for_test(), 1);
        assert_eq!(
            receiver.receive_registration_callback().unwrap(),
            0x1357_9bdf
        );

        receiver
            .begin_deregistration(deregistration_callback)
            .unwrap();
        let error = receiver.receive_deregistration_callback().unwrap_err();
        assert_eq!(
            (error.expected(), error.actual()),
            (
                MainDmoBankNotificationStage::DeregistrationCallback,
                MainDmoBankNotificationStage::DeregistrationCollection,
            )
        );
        events.lock().unwrap().clear();
        receiver.receive_deregistration_collection().unwrap();
        let mut expected = (0..125)
            .map(CoordinatorEvent::DmoDeregister)
            .collect::<Vec<_>>();
        for ordinal in 0..125 {
            if ordinal < 93 {
                expected.push(CoordinatorEvent::Remove(
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                ));
            }
            expected.push(CoordinatorEvent::LocationDeregister(ordinal));
        }
        assert_eq!(*events.lock().unwrap(), expected);
        assert_eq!(receiver.live_registration_count_for_test(), 0);
        assert_eq!(
            receiver.receive_deregistration_callback().unwrap(),
            0x0123_4567_89ab_cdef
        );
        assert_eq!(
            receiver.stage_for_test(),
            MainDmoBankNotificationStage::Complete
        );
    }

    #[test]
    fn notification_deferred_requires_every_arena_then_finish_then_callback() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver: TestNotificationReceiver<u16, u8> =
            MainDmoBankNotificationReceiver::try_new_deferred(
                coordinator(binding, &events),
                0x2468,
            )
            .unwrap();
        assert!(events.lock().unwrap().is_empty());

        let error = receiver.receive_deferred_finish().unwrap_err();
        assert!(matches!(
            error,
            MainDmoBankDeferredFinishReceiverError::WrongStage(stage)
                if stage.expected() == MainDmoBankNotificationStage::DeferredFinish
                    && stage.actual() == MainDmoBankNotificationStage::DeferredArena
        ));
        assert!(receiver.receive_registration_callback().is_err());
        for ordinal in 0..125 {
            assert_eq!(
                receiver.receive_deferred_arena(ordinal).unwrap(),
                ordinal == 124
            );
        }
        assert!(matches!(
            receiver.receive_deferred_arena(125),
            Err(MainDmoBankDeferredArenaReceiverError::WrongStage(stage))
                if stage.actual() == MainDmoBankNotificationStage::DeferredFinish
        ));

        let mut expected = Vec::new();
        for ordinal in 0..125 {
            if ordinal < 93 {
                expected.push(CoordinatorEvent::Add(
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                ));
            }
            expected.push(CoordinatorEvent::LocationRegister(ordinal));
        }
        assert_eq!(*events.lock().unwrap(), expected);
        assert!(receiver.receive_registration_callback().is_err());

        receiver.receive_deferred_finish().unwrap();
        expected.extend((0..125).map(CoordinatorEvent::DmoRegister));
        assert_eq!(*events.lock().unwrap(), expected);
        assert_eq!(receiver.receive_registration_callback().unwrap(), 0x2468);
        assert_eq!(
            receiver.stage_for_test(),
            MainDmoBankNotificationStage::Registered
        );
    }

    #[test]
    fn notification_receiver_failure_withholds_callback_and_drop_keeps_exact_prefix() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        collection.tombstone_without_releasing_assets_for_test();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver: TestNotificationReceiver<u32, u32> =
            MainDmoBankNotificationReceiver::new_immediate(
                coordinator(binding, &events),
                0xfeed_beef,
            );

        assert!(matches!(
            receiver.receive_immediate_collection(),
            Err(MainDmoBankImmediateReceiverError::Receiver(
                MainDmoBankCollectionRegistrationError::Controller {
                    arena_index: 0,
                    source: SimulationDmoBankModelRegisterPassError::Source(
                        SimulationDmoBankModelRegistrationPassError::ResidentExpired
                    )
                }
            ))
        ));
        assert_eq!(
            receiver.stage_for_test(),
            MainDmoBankNotificationStage::Faulted
        );
        let error = receiver.receive_registration_callback().unwrap_err();
        assert_eq!(error.actual(), MainDmoBankNotificationStage::Faulted);
        assert!(matches!(
            receiver.receive_immediate_collection(),
            Err(MainDmoBankImmediateReceiverError::WrongStage(stage))
                if stage.actual() == MainDmoBankNotificationStage::Faulted
        ));
        assert_eq!(
            *events.lock().unwrap(),
            [CoordinatorEvent::Add(0x1000_0000_0000_0000)]
        );
        drop(receiver);
        assert_eq!(
            *events.lock().unwrap(),
            [CoordinatorEvent::Add(0x1000_0000_0000_0000)]
        );
    }

    #[test]
    fn notification_dreg_failure_withholds_callback_after_exact_reached_prefix() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut receiver: TestNotificationReceiver<u8, u32> =
            MainDmoBankNotificationReceiver::new_immediate(coordinator(binding, &events), 0x5a);
        receiver.receive_immediate_collection().unwrap();
        assert_eq!(receiver.receive_registration_callback().unwrap(), 0x5a);
        receiver.begin_deregistration(0xfeed_beef).unwrap();
        collection.tombstone_without_releasing_assets_for_test();
        events.lock().unwrap().clear();

        assert!(matches!(
            receiver.receive_deregistration_collection(),
            Err(MainDmoBankDeregistrationReceiverError::Receiver(_))
        ));
        assert_eq!(
            receiver.stage_for_test(),
            MainDmoBankNotificationStage::Faulted
        );
        let error = receiver.receive_deregistration_callback().unwrap_err();
        assert_eq!(error.actual(), MainDmoBankNotificationStage::Faulted);
        let mut expected = (0..125)
            .map(CoordinatorEvent::DmoDeregister)
            .collect::<Vec<_>>();
        expected.push(CoordinatorEvent::Remove(0x1000_0000_0000_0000));
        assert_eq!(*events.lock().unwrap(), expected);
        drop(receiver);
        assert_eq!(*events.lock().unwrap(), expected);
    }

    #[test]
    fn notification_deferred_begin_rejection_retains_coordinator_and_callback_for_retry() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        collection.tombstone_without_releasing_assets_for_test();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut coordinator = coordinator(binding, &events);
        assert!(matches!(
            coordinator.register_active(),
            Err(MainDmoBankCollectionRegistrationError::Controller {
                arena_index: 0,
                source: SimulationDmoBankModelRegisterPassError::Source(
                    SimulationDmoBankModelRegistrationPassError::ResidentExpired
                )
            })
        ));
        events.lock().unwrap().clear();
        let retained_event_owners = Arc::strong_count(&events);
        let callback_drops = Arc::new(AtomicUsize::new(0));

        let result: Result<TestNotificationReceiver<CallbackDropProbe, u8>, _> =
            MainDmoBankNotificationReceiver::try_new_deferred(
                coordinator,
                CallbackDropProbe(Arc::clone(&callback_drops)),
            );
        let Err(rejection) = result else {
            panic!("faulted coordinator unexpectedly began a deferred receiver");
        };
        assert!(matches!(
            rejection.source(),
            MainDmoBankCollectionRegistrationError::ControllerFaulted(
                MainDmoBankCollectionControllerFault::RegistrationArena { arena_index: 0 }
            )
        ));
        assert_eq!(callback_drops.load(Ordering::Relaxed), 0);
        assert_eq!(Arc::strong_count(&events), retained_event_owners);

        let result: Result<TestNotificationReceiver<CallbackDropProbe, u8>, _> =
            rejection.retry_deferred();
        let Err(rejection) = result else {
            panic!("faulted coordinator unexpectedly recovered during retry");
        };
        assert!(matches!(
            rejection.source(),
            MainDmoBankCollectionRegistrationError::ControllerFaulted(
                MainDmoBankCollectionControllerFault::RegistrationArena { arena_index: 0 }
            )
        ));
        assert_eq!(callback_drops.load(Ordering::Relaxed), 0);
        assert_eq!(Arc::strong_count(&events), retained_event_owners);
        assert!(events.lock().unwrap().is_empty());

        drop(rejection);
        assert_eq!(callback_drops.load(Ordering::Relaxed), 1);
        assert_eq!(Arc::strong_count(&events), 1);
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn marker_fifo_immediate_ack_retries_exact_callback_without_replaying_receiver() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let hull = marker_hull();
        let foreign_hull = marker_hull();
        let callback = hull.issue_simulation_registration_callback_for_test();
        let (mut fifo, immediate) =
            MainDmoBankMarkerFifo::new_immediate(coordinator(binding, &events), callback);
        let callback = fifo.enqueue_immediate_collection(immediate).unwrap();
        assert_eq!(fifo.queue_len_for_test(), 1);

        assert!(matches!(
            fifo.dispatch_one(&hull).unwrap(),
            MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(
                MainDmoBankMarkerKind::ImmediateCollection
            )
        ));
        assert_eq!(fifo.queue_len_for_test(), 0);
        let receiver_effects = events.lock().unwrap().clone();
        assert_eq!(fifo.live_registration_count_for_test(), 1);
        assert_eq!(hull.observation().unwrap().registration_remaining, 2);
        assert!(matches!(
            fifo.dispatch_one(&hull).unwrap(),
            MainDmoBankMarkerDispatch::Idle
        ));

        fifo.enqueue_registration_callback(callback).unwrap();
        assert_eq!(fifo.queue_len_for_test(), 1);

        let Err(rejection) = fifo.dispatch_one(&foreign_hull) else {
            panic!("foreign hull unexpectedly acknowledged the callback");
        };
        assert_eq!(
            rejection,
            MainDmoBankMarkerDispatchError::RegistrationAcknowledgement(
                MainDmoHullCallbackRejectionKind::ForeignDomain
            )
        );
        assert_eq!(fifo.queue_len_for_test(), 1);
        assert_eq!(*events.lock().unwrap(), receiver_effects);
        assert_eq!(fifo.live_registration_count_for_test(), 1);
        assert_eq!(hull.observation().unwrap().registration_remaining, 2);
        assert_eq!(
            foreign_hull.observation().unwrap().registration_remaining,
            2
        );

        let MainDmoBankMarkerDispatch::RegistrationAcknowledged(deregistration) =
            fifo.dispatch_one(&hull).unwrap()
        else {
            panic!("registration callback did not acknowledge");
        };
        assert_eq!(fifo.queue_len_for_test(), 0);
        assert_eq!(*events.lock().unwrap(), receiver_effects);
        assert_eq!(hull.observation().unwrap().registration_remaining, 1);
        drop(deregistration);
    }

    #[test]
    fn marker_fifo_deferred_requires_exact_125_ordinals_finish_then_ack() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let hull = marker_hull();
        let callback = hull.issue_simulation_registration_callback_for_test();
        let (mut fifo, first) =
            MainDmoBankMarkerFifo::try_new_deferred(coordinator(binding, &events), callback)
                .unwrap();
        let mut state = MainDmoBankDeferredMarkerProducerState::NextArena(first);
        for ordinal in 0..125 {
            let producer = match state {
                MainDmoBankDeferredMarkerProducerState::NextArena(producer) => producer,
                MainDmoBankDeferredMarkerProducerState::ReadyToFinish(_) => {
                    panic!("deferred producer finished before ordinal {ordinal}")
                }
            };
            state = fifo.enqueue_deferred_arena(producer).unwrap();
            assert_eq!(fifo.queue_len_for_test(), ordinal + 1);
        }
        let finish = match state {
            MainDmoBankDeferredMarkerProducerState::ReadyToFinish(finish) => finish,
            MainDmoBankDeferredMarkerProducerState::NextArena(_) => {
                panic!("125 deferred arenas did not finish the prefix")
            }
        };
        let callback = fifo.enqueue_deferred_finish(finish).unwrap();
        fifo.enqueue_registration_callback(callback).unwrap();
        assert_eq!(fifo.queue_len_for_test(), 127);

        for ordinal in 0..125 {
            assert!(matches!(
                fifo.dispatch_one(&hull).unwrap(),
                MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(
                    MainDmoBankMarkerKind::DeferredArena { ordinal: actual }
                ) if actual == ordinal
            ));
        }
        assert!(matches!(
            fifo.dispatch_one(&hull).unwrap(),
            MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(
                MainDmoBankMarkerKind::DeferredFinish
            )
        ));
        assert_eq!(hull.observation().unwrap().registration_remaining, 2);
        let MainDmoBankMarkerDispatch::RegistrationAcknowledged(deregistration) =
            fifo.dispatch_one(&hull).unwrap()
        else {
            panic!("deferred callback did not acknowledge");
        };
        assert_eq!(fifo.queue_len_for_test(), 0);
        assert_eq!(hull.observation().unwrap().registration_remaining, 1);
        drop(deregistration);
    }

    #[test]
    fn marker_fifo_receiver_fault_is_sticky_and_retains_failed_prefix_and_callback() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        collection.tombstone_without_releasing_assets_for_test();
        let events = Arc::new(Mutex::new(Vec::new()));
        let hull = marker_hull();
        let callback = hull.issue_simulation_registration_callback_for_test();
        let (mut fifo, immediate) =
            MainDmoBankMarkerFifo::new_immediate(coordinator(binding, &events), callback);
        let callback = fifo.enqueue_immediate_collection(immediate).unwrap();
        fifo.enqueue_registration_callback(callback).unwrap();

        let expected = MainDmoBankMarkerDispatchError::ReceiverFaulted(
            MainDmoBankMarkerKind::ImmediateCollection,
        );
        let Err(first) = fifo.dispatch_one(&hull) else {
            panic!("faulting marker unexpectedly completed");
        };
        let Err(second) = fifo.dispatch_one(&hull) else {
            panic!("sticky fault unexpectedly cleared");
        };
        assert_eq!(first, expected);
        assert_eq!(second, expected);
        assert_eq!(fifo.queue_len_for_test(), 2);
        assert_eq!(
            fifo.fault_for_test().unwrap().marker(),
            MainDmoBankMarkerKind::ImmediateCollection
        );
        assert_eq!(hull.observation().unwrap().registration_remaining, 2);
        assert_eq!(
            *events.lock().unwrap(),
            [CoordinatorEvent::Add(0x1000_0000_0000_0000)]
        );
        drop(fifo);
        assert_eq!(
            *events.lock().unwrap(),
            [CoordinatorEvent::Add(0x1000_0000_0000_0000)]
        );
    }

    #[test]
    fn marker_fifo_rejects_foreign_producer_without_consuming_or_appending_it() {
        let (first_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_first_storage, first_collection) = collection(first_assets);
        let first_binding =
            PreparedMainDmoBankSimulationCollection::try_new(&first_collection).unwrap();
        let (second_assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_second_storage, second_collection) = collection(second_assets);
        let second_binding =
            PreparedMainDmoBankSimulationCollection::try_new(&second_collection).unwrap();
        let first_events = Arc::new(Mutex::new(Vec::new()));
        let second_events = Arc::new(Mutex::new(Vec::new()));
        let first_hull = marker_hull();
        let second_hull = marker_hull();
        let (mut first, producer) = MainDmoBankMarkerFifo::new_immediate(
            coordinator(first_binding, &first_events),
            first_hull.issue_simulation_registration_callback_for_test(),
        );
        let (mut second, second_producer) = MainDmoBankMarkerFifo::new_immediate(
            coordinator(second_binding, &second_events),
            second_hull.issue_simulation_registration_callback_for_test(),
        );

        let Err(rejection) = second.enqueue_immediate_collection(producer) else {
            panic!("foreign producer unexpectedly appended");
        };
        assert_eq!(
            rejection.kind(),
            MainDmoBankMarkerEnqueueRejectionKind::ForeignDomain
        );
        assert_eq!(second.queue_len_for_test(), 0);
        let producer = rejection.into_permit();
        let callback = first.enqueue_immediate_collection(producer).unwrap();
        first.enqueue_registration_callback(callback).unwrap();
        assert_eq!(first.queue_len_for_test(), 2);
        assert_eq!(second.queue_len_for_test(), 0);
        drop(second_producer);
    }

    #[test]
    fn marker_fifo_dreg_collection_precedes_opaque_callback_front_and_drop_is_inert() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let hull = marker_hull();
        let callback = hull.issue_simulation_registration_callback_for_test();
        let (mut fifo, immediate) =
            MainDmoBankMarkerFifo::new_immediate(coordinator(binding, &events), callback);
        let callback = fifo.enqueue_immediate_collection(immediate).unwrap();
        fifo.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            fifo.dispatch_one(&hull).unwrap(),
            MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(
                MainDmoBankMarkerKind::ImmediateCollection
            )
        ));
        let MainDmoBankMarkerDispatch::RegistrationAcknowledged(deregistration) =
            fifo.dispatch_one(&hull).unwrap()
        else {
            panic!("registration callback did not acknowledge");
        };
        let callback = hull.issue_simulation_deregistration_callback_for_test();
        let collection_marker = fifo.begin_deregistration(deregistration, callback).unwrap();
        let callback_marker = fifo
            .enqueue_deregistration_collection(collection_marker)
            .unwrap();
        fifo.enqueue_deregistration_callback(callback_marker)
            .unwrap();
        assert_eq!(fifo.queue_len_for_test(), 2);
        let before_dreg = events.lock().unwrap().len();

        assert!(matches!(
            fifo.dispatch_one(&hull).unwrap(),
            MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(
                MainDmoBankMarkerKind::DeregistrationCollection
            )
        ));
        let mut expected_dreg = (0..125)
            .map(CoordinatorEvent::DmoDeregister)
            .collect::<Vec<_>>();
        for ordinal in 0..125 {
            if ordinal < 93 {
                expected_dreg.push(CoordinatorEvent::Remove(
                    0x1000_0000_0000_0000 | u64::try_from(ordinal).unwrap(),
                ));
            }
            expected_dreg.push(CoordinatorEvent::LocationDeregister(ordinal));
        }
        assert_eq!(
            &events.lock().unwrap()[before_dreg..],
            expected_dreg.as_slice()
        );
        assert_eq!(fifo.live_registration_count_for_test(), 0);
        assert_eq!(hull.observation().unwrap().unregistration_completed, 0);
        let MainDmoBankMarkerDispatch::DeregistrationCallbackReady(callback_front) =
            fifo.dispatch_one(&hull).unwrap()
        else {
            panic!("DReg callback front was not issued");
        };
        assert_eq!(fifo.queue_len_for_test(), 1);
        let Err(in_flight) = fifo.dispatch_one(&hull) else {
            panic!("DReg callback front was issued twice");
        };
        assert_eq!(
            in_flight,
            MainDmoBankMarkerDispatchError::DeregistrationCallbackInFlight
        );
        assert_eq!(hull.observation().unwrap().unregistration_completed, 0);
        let completed_events = events.lock().unwrap().clone();
        drop(callback_front);
        drop(fifo);
        assert_eq!(*events.lock().unwrap(), completed_events);
        assert_eq!(hull.observation().unwrap().unregistration_completed, 0);
    }

    #[test]
    fn marker_fifo_drop_with_queued_dreg_markers_performs_no_teardown() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let hull = marker_hull();
        let callback = hull.issue_simulation_registration_callback_for_test();
        let (mut fifo, immediate) =
            MainDmoBankMarkerFifo::new_immediate(coordinator(binding, &events), callback);
        let callback = fifo.enqueue_immediate_collection(immediate).unwrap();
        fifo.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            fifo.dispatch_one(&hull).unwrap(),
            MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(
                MainDmoBankMarkerKind::ImmediateCollection
            )
        ));
        let MainDmoBankMarkerDispatch::RegistrationAcknowledged(deregistration) =
            fifo.dispatch_one(&hull).unwrap()
        else {
            panic!("registration callback did not acknowledge");
        };
        let callback = hull.issue_simulation_deregistration_callback_for_test();
        let collection_marker = fifo.begin_deregistration(deregistration, callback).unwrap();
        let callback_marker = fifo
            .enqueue_deregistration_collection(collection_marker)
            .unwrap();
        fifo.enqueue_deregistration_callback(callback_marker)
            .unwrap();
        events.lock().unwrap().clear();

        drop(fifo);
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn marker_fifo_dreg_ack_retries_wrong_hull_and_streamer_then_pops_after_exact_down() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let callback = harness.take_simulation_registration_callback();
        let (mut fifo, immediate) =
            MainDmoBankMarkerFifo::new_immediate(coordinator(binding, &events), callback);
        let callback = fifo.enqueue_immediate_collection(immediate).unwrap();
        fifo.enqueue_registration_callback(callback).unwrap();
        assert!(matches!(
            fifo.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(
                MainDmoBankMarkerKind::ImmediateCollection
            )
        ));
        let MainDmoBankMarkerDispatch::RegistrationAcknowledged(deregistration) =
            fifo.dispatch_one(harness.coordinator()).unwrap()
        else {
            panic!("registration callback did not acknowledge");
        };
        let callback = harness.advance_and_take_simulation_deregistration_callback();
        let collection_marker = fifo.begin_deregistration(deregistration, callback).unwrap();
        let callback_marker = fifo
            .enqueue_deregistration_collection(collection_marker)
            .unwrap();
        fifo.enqueue_deregistration_callback(callback_marker)
            .unwrap();
        assert!(matches!(
            fifo.dispatch_one(harness.coordinator()).unwrap(),
            MainDmoBankMarkerDispatch::ReceiverMarkerCompleted(
                MainDmoBankMarkerKind::DeregistrationCollection
            )
        ));
        let MainDmoBankMarkerDispatch::DeregistrationCallbackReady(front) =
            fifo.dispatch_one(harness.coordinator()).unwrap()
        else {
            panic!("DReg callback front was not issued");
        };
        assert_eq!(fifo.queue_len_for_test(), 1);
        assert_eq!(harness.resident_count(), 1);
        assert_eq!(harness.unregistration_completed(), 1);

        let foreign_hull = marker_hull();
        let (_, releaser) = harness.acknowledgement_parts();
        let Err(MainDmoBankDeregistrationAcknowledgementError::HullCallback { kind, front }) =
            fifo.acknowledge_deregistration_callback(front, &foreign_hull, releaser)
        else {
            panic!("foreign hull unexpectedly acknowledged DReg");
        };
        assert_eq!(kind, MainDmoHullCallbackRejectionKind::ForeignDomain);
        assert_eq!(fifo.queue_len_for_test(), 1);
        assert_eq!(harness.resident_count(), 1);
        assert_eq!(harness.unregistration_completed(), 1);

        let mut foreign_streamer = MainDmoDregIntegrationHarness::new();
        let (_, foreign_releaser) = foreign_streamer.acknowledgement_parts();
        let Err(MainDmoBankDeregistrationAcknowledgementError::ResidentAuthority { kind, front }) =
            fifo.acknowledge_deregistration_callback(front, harness.coordinator(), foreign_releaser)
        else {
            panic!("foreign resident streamer unexpectedly released the hold");
        };
        assert_eq!(
            kind,
            crate::collection_resident_streamer::ResidentCollectionHoldAuthorityError::ForeignDomain
        );
        assert_eq!(fifo.queue_len_for_test(), 1);
        assert_eq!(harness.resident_count(), 1);
        assert_eq!(
            harness
                .coordinator()
                .observation()
                .unwrap()
                .unregistration_completed,
            1
        );

        let (hull, releaser) = harness.acknowledgement_parts();
        assert!(
            fifo.acknowledge_deregistration_callback(front, hull, releaser)
                .unwrap()
        );
        assert_eq!(fifo.queue_len_for_test(), 0);
        assert_eq!(harness.resident_count(), 0);
        assert_eq!(harness.unregistration_completed(), 2);
    }

    #[test]
    fn marker_fifo_terminal_resident_release_fault_is_sticky_without_popping_front() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let callback = harness.take_simulation_registration_callback();
        let (mut fifo, immediate) =
            MainDmoBankMarkerFifo::new_immediate(coordinator(binding, &events), callback);
        let callback = fifo.enqueue_immediate_collection(immediate).unwrap();
        fifo.enqueue_registration_callback(callback).unwrap();
        fifo.dispatch_one(harness.coordinator()).unwrap();
        let MainDmoBankMarkerDispatch::RegistrationAcknowledged(deregistration) =
            fifo.dispatch_one(harness.coordinator()).unwrap()
        else {
            panic!("registration callback did not acknowledge");
        };
        let callback = harness.advance_and_take_simulation_deregistration_callback();
        let collection_marker = fifo.begin_deregistration(deregistration, callback).unwrap();
        let callback_marker = fifo
            .enqueue_deregistration_collection(collection_marker)
            .unwrap();
        fifo.enqueue_deregistration_callback(callback_marker)
            .unwrap();
        fifo.dispatch_one(harness.coordinator()).unwrap();
        let MainDmoBankMarkerDispatch::DeregistrationCallbackReady(front) =
            fifo.dispatch_one(harness.coordinator()).unwrap()
        else {
            panic!("DReg callback front was not issued");
        };
        harness.fail_final_release();
        let (hull, releaser) = harness.acknowledgement_parts();
        assert!(matches!(
            fifo.acknowledge_deregistration_callback(front, hull, releaser),
            Err(MainDmoBankDeregistrationAcknowledgementError::ResidentRelease(_))
        ));
        assert_eq!(fifo.queue_len_for_test(), 1);
        assert_eq!(
            fifo.fault_for_test().unwrap().marker(),
            MainDmoBankMarkerKind::DeregistrationCallback
        );
        assert_eq!(harness.resident_count(), 0);
        assert_eq!(harness.unregistration_completed(), 1);
        let Err(sticky) = fifo.dispatch_one(harness.coordinator()) else {
            panic!("terminal resident failure unexpectedly cleared");
        };
        assert_eq!(
            sticky,
            MainDmoBankMarkerDispatchError::ReceiverFaulted(
                MainDmoBankMarkerKind::DeregistrationCallback
            )
        );
    }

    #[test]
    fn marker_fifo_resident_release_panic_is_distinct_and_keeps_callback_marker_front() {
        let (assets, _) = corpus_assets(&CORPUS_ASSET_IDS);
        let (_storage, collection) = collection(assets);
        let binding = PreparedMainDmoBankSimulationCollection::try_new(&collection).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = MainDmoDregIntegrationHarness::new();
        let callback = harness.take_simulation_registration_callback();
        let (mut fifo, immediate) =
            MainDmoBankMarkerFifo::new_immediate(coordinator(binding, &events), callback);
        let callback = fifo.enqueue_immediate_collection(immediate).unwrap();
        fifo.enqueue_registration_callback(callback).unwrap();
        fifo.dispatch_one(harness.coordinator()).unwrap();
        let MainDmoBankMarkerDispatch::RegistrationAcknowledged(deregistration) =
            fifo.dispatch_one(harness.coordinator()).unwrap()
        else {
            panic!("registration callback did not acknowledge");
        };
        let callback = harness.advance_and_take_simulation_deregistration_callback();
        let collection_marker = fifo.begin_deregistration(deregistration, callback).unwrap();
        let callback_marker = fifo
            .enqueue_deregistration_collection(collection_marker)
            .unwrap();
        fifo.enqueue_deregistration_callback(callback_marker)
            .unwrap();
        fifo.dispatch_one(harness.coordinator()).unwrap();
        let MainDmoBankMarkerDispatch::DeregistrationCallbackReady(front) =
            fifo.dispatch_one(harness.coordinator()).unwrap()
        else {
            panic!("DReg callback front was not issued");
        };
        harness.panic_final_release();
        let (hull, releaser) = harness.acknowledgement_parts();
        assert!(matches!(
            fifo.acknowledge_deregistration_callback(front, hull, releaser),
            Err(MainDmoBankDeregistrationAcknowledgementError::ResidentReleasePanicked)
        ));
        assert_eq!(fifo.queue_len_for_test(), 1);
        assert_eq!(
            fifo.fault_for_test().unwrap().marker(),
            MainDmoBankMarkerKind::DeregistrationCallback
        );
        assert_eq!(harness.resident_count(), 0);
        assert_eq!(harness.unregistration_completed(), 1);
        let Err(sticky) = fifo.dispatch_one(harness.coordinator()) else {
            panic!("resident-release panic unexpectedly cleared");
        };
        assert_eq!(
            sticky,
            MainDmoBankMarkerDispatchError::ReceiverFaulted(
                MainDmoBankMarkerKind::DeregistrationCallback
            )
        );
    }
}
