//! Deferred Presentation producer ownership for the exact main DMO bank.
//!
//! `cAssetStreamManager::UpdatePresCollectionDefer`
//! (`0x82d5b5f0..0x82d5b8a0`) performs Setup as the final substantive action of
//! a Pending update. A later matching Active update samples the shared batch
//! controls, applies the recovered `cArena::mCost` boundary, and sends the
//! selected Arena messages in authored order. An update that actually reaches
//! the maximum sends the primary activation-zero Collection, optional proxy
//! `DRegCollection`, and registration callback before calling Setup again.
//!
//! This owner stages an occurrence-local safe-host queue and can deliver an
//! Arena FIFO front through the separately reconstructed refcount-first
//! receiver gate. It can also accept the exact activation-zero Collection
//! front and extract the following callback capability without invoking it.
//! It does not process proxy fronts, model Setup's pending-vector mutation, or
//! reproduce global `EPostman` capacity/interleaving and frame scheduling.
//! Pre-reservation, typed retry, and Terminal idle behavior are safe-host
//! policy rather than native allocation/error behavior.

use crate::{
    collection_parent_manager::INVALID_PARENT_COLLECTION_ID,
    collection_resident_streamer::CollectionId,
    dmo_bank_processed_asset::DMO_BANK_COLLECTION_ID,
    dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
    presentation_dmo_arena_adapter::{
        PreparedMainDmoBankPresentationCollection, PresentationDmoAdapterFault,
        PresentationDmoArenaCostUnavailable, PresentationDmoArenaDispatch,
        PresentationDmoArenaRegistrationAdapter, PresentationDmoCollectionRegistration,
        PresentationDmoDeferredArenaReceiveError, PresentationDmoDeferredArenaTransition,
        PresentationDmoDeferredCollectionRegistration, PresentationDmoDeferredFinishErrorKind,
    },
    simulation_deferred_batch_policy::{
        DeferredCollectionBatchControls, PresentationDeferredBatchSelectionError,
    },
    simulation_resident_registry::SimulationResidentArenaLookup,
};
use std::{error::Error, fmt};

#[derive(Debug)]
enum PresentationDeferredMarker<C> {
    Arena {
        ordinal: u32,
        arena_id: CollectionId,
    },
    ActivationZeroCollection {
        collection_id: CollectionId,
    },
    ProxyDeregistrationCollection {
        collection_id: CollectionId,
    },
    RegistrationCallback(C),
}

/// Sanitized kind of one locally staged producer marker. This is not a native
/// message layout; Arena dequeue is exposed only through the checked receiver
/// delivery boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoPresentationDeferredMarkerKind {
    Arena {
        ordinal: u32,
        arena_id: CollectionId,
    },
    ActivationZeroCollection {
        collection_id: CollectionId,
    },
    ProxyDeregistrationCollection {
        collection_id: CollectionId,
    },
    RegistrationCallback,
}

impl<C> PresentationDeferredMarker<C> {
    const fn kind(&self) -> MainDmoPresentationDeferredMarkerKind {
        match self {
            Self::Arena { ordinal, arena_id } => MainDmoPresentationDeferredMarkerKind::Arena {
                ordinal: *ordinal,
                arena_id: *arena_id,
            },
            Self::ActivationZeroCollection { collection_id } => {
                MainDmoPresentationDeferredMarkerKind::ActivationZeroCollection {
                    collection_id: *collection_id,
                }
            }
            Self::ProxyDeregistrationCollection { collection_id } => {
                MainDmoPresentationDeferredMarkerKind::ProxyDeregistrationCollection {
                    collection_id: *collection_id,
                }
            }
            Self::RegistrationCallback(_) => {
                MainDmoPresentationDeferredMarkerKind::RegistrationCallback
            }
        }
    }
}

struct Pending<C> {
    prepared: PreparedMainDmoBankPresentationCollection,
    proxy_collection_id: CollectionId,
    callback: C,
    queue: Vec<PresentationDeferredMarker<C>>,
    reservation_limit: Option<usize>,
}

struct Active<C> {
    prepared: PreparedMainDmoBankPresentationCollection,
    proxy_collection_id: CollectionId,
    callback: Option<C>,
    next_index: u32,
    queue: Vec<PresentationDeferredMarker<C>>,
    reservation_limit: Option<usize>,
}

struct Terminal<C> {
    _prepared: PreparedMainDmoBankPresentationCollection,
    next_index: u32,
    queue: Vec<PresentationDeferredMarker<C>>,
}

enum State<C> {
    Pending(Pending<C>),
    Active(Active<C>),
    Terminal(Terminal<C>),
}

/// Opaque owner for one exact main-bank deferred Presentation registration.
#[must_use]
pub struct MainDmoPresentationDeferredProducer<C> {
    state: State<C>,
}

/// Result of one local producer update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoPresentationDeferredProducerUpdate {
    SetupCompleted,
    ArenasQueued {
        count: u32,
    },
    TerminalQueued {
        arena_count: u32,
        proxy_collection_queued: bool,
    },
    TerminalIdle,
}

/// Refusal to deliver the current FIFO front as an Arena receiver message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoPresentationDeferredArenaDeliveryError {
    QueueEmpty,
    FrontIsNotArena {
        actual: MainDmoPresentationDeferredMarkerKind,
    },
    Receiver(PresentationDmoDeferredArenaReceiveError),
}

/// Typed reason the activation-zero FIFO front could not be accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoPresentationDeferredActivationDeliveryError {
    QueueEmpty,
    FrontIsNotActivationZero {
        actual: MainDmoPresentationDeferredMarkerKind,
    },
    UnexpectedCollectionId {
        expected: CollectionId,
        actual: CollectionId,
    },
    Receiver {
        kind: PresentationDmoDeferredFinishErrorKind,
        fault: Option<PresentationDmoAdapterFault>,
    },
}

impl fmt::Display for MainDmoPresentationDeferredActivationDeliveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueEmpty => formatter.write_str("Presentation deferred queue is empty"),
            Self::FrontIsNotActivationZero { actual } => write!(
                formatter,
                "Presentation deferred queue front is {actual:?}, not activation-zero Collection"
            ),
            Self::UnexpectedCollectionId { expected, actual } => write!(
                formatter,
                "activation-zero Collection ID {:#018x} differs from expected {:#018x}",
                actual.0, expected.0
            ),
            Self::Receiver { kind, fault } => write!(
                formatter,
                "activation-zero Presentation receiver refused {kind:?} with fault {fault:?}"
            ),
        }
    }
}

impl Error for MainDmoPresentationDeferredActivationDeliveryError {}

/// Effect-free refusal which returns the exact deferred occurrence authority.
#[must_use]
pub struct MainDmoPresentationDeferredActivationDeliveryFailure {
    error: MainDmoPresentationDeferredActivationDeliveryError,
    registration: PresentationDmoDeferredCollectionRegistration,
}

impl MainDmoPresentationDeferredActivationDeliveryFailure {
    #[must_use]
    pub const fn error(&self) -> &MainDmoPresentationDeferredActivationDeliveryError {
        &self.error
    }

    #[must_use]
    pub fn into_registration(self) -> PresentationDmoDeferredCollectionRegistration {
        self.registration
    }
}

impl fmt::Debug for MainDmoPresentationDeferredActivationDeliveryFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoPresentationDeferredActivationDeliveryFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for MainDmoPresentationDeferredActivationDeliveryFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for MainDmoPresentationDeferredActivationDeliveryFailure {}

/// Refusal to extract the callback at the current FIFO front.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoPresentationDeferredCallbackTakeError {
    QueueEmpty,
    FrontIsNotRegistrationCallback {
        actual: MainDmoPresentationDeferredMarkerKind,
    },
}

impl fmt::Display for MainDmoPresentationDeferredCallbackTakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueEmpty => formatter.write_str("Presentation deferred queue is empty"),
            Self::FrontIsNotRegistrationCallback { actual } => write!(
                formatter,
                "Presentation deferred queue front is {actual:?}, not RegistrationCallback"
            ),
        }
    }
}

impl Error for MainDmoPresentationDeferredCallbackTakeError {}

impl fmt::Display for MainDmoPresentationDeferredArenaDeliveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueEmpty => formatter.write_str("Presentation deferred queue is empty"),
            Self::FrontIsNotArena { actual } => write!(
                formatter,
                "Presentation deferred queue front is {actual:?}, not an Arena"
            ),
            Self::Receiver(source) => source.fmt(formatter),
        }
    }
}

impl Error for MainDmoPresentationDeferredArenaDeliveryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Receiver(source) => Some(source),
            Self::QueueEmpty | Self::FrontIsNotArena { .. } => None,
        }
    }
}

/// Typed reason an Active update could not be staged.
#[derive(Debug)]
pub enum MainDmoPresentationDeferredProducerUpdateError<E> {
    Controls(E),
    Selection(PresentationDeferredBatchSelectionError<PresentationDmoArenaCostUnavailable>),
    QueueAllocation { requested: usize },
}

impl<E: fmt::Display> fmt::Display for MainDmoPresentationDeferredProducerUpdateError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Controls(source) => {
                write!(formatter, "could not sample deferred controls: {source}")
            }
            Self::Selection(source) => source.fmt(formatter),
            Self::QueueAllocation { requested } => write!(
                formatter,
                "could not reserve {requested} Presentation deferred markers"
            ),
        }
    }
}

impl<E: Error + 'static> Error for MainDmoPresentationDeferredProducerUpdateError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Controls(source) => Some(source),
            Self::Selection(source) => Some(source),
            Self::QueueAllocation { .. } => None,
        }
    }
}

/// Owning refusal that keeps the exact callback, prepared collection, progress,
/// and already-staged prefix available for retry.
#[must_use]
pub struct MainDmoPresentationDeferredProducerUpdateFailure<C, E> {
    error: MainDmoPresentationDeferredProducerUpdateError<E>,
    producer: MainDmoPresentationDeferredProducer<C>,
}

impl<C, E> MainDmoPresentationDeferredProducerUpdateFailure<C, E> {
    #[must_use]
    pub const fn error(&self) -> &MainDmoPresentationDeferredProducerUpdateError<E> {
        &self.error
    }

    /// Reattempts the unchanged Active state with one newly sampled control
    /// snapshot.
    ///
    /// # Errors
    ///
    /// Returns another owning refusal when sampling, selection, or complete
    /// local update reservation is refused again.
    pub fn retry_with<F>(
        self,
        controls: F,
    ) -> Result<
        (
            MainDmoPresentationDeferredProducer<C>,
            MainDmoPresentationDeferredProducerUpdate,
        ),
        Self,
    >
    where
        F: FnOnce() -> Result<DeferredCollectionBatchControls, E>,
    {
        self.producer.try_update_with(controls)
    }
}

impl<C, E: fmt::Debug> fmt::Debug for MainDmoPresentationDeferredProducerUpdateFailure<C, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoPresentationDeferredProducerUpdateFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<C, E: fmt::Display> fmt::Display for MainDmoPresentationDeferredProducerUpdateFailure<C, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<C, E: Error + 'static> Error for MainDmoPresentationDeferredProducerUpdateFailure<C, E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

impl<C> MainDmoPresentationDeferredProducer<C> {
    /// Creates a Pending owner. The full proxy ID is retained even when it is
    /// the exact invalid sentinel; zero is an ordinary ID.
    pub fn new(
        prepared: PreparedMainDmoBankPresentationCollection,
        proxy_collection_id: CollectionId,
        callback: C,
    ) -> Self {
        Self {
            state: State::Pending(Pending {
                prepared,
                proxy_collection_id,
                callback,
                queue: Vec::new(),
                reservation_limit: None,
            }),
        }
    }

    /// Advances one local producer invocation. Pending and Terminal states do
    /// not call `controls`; every Active invocation calls it exactly once.
    ///
    /// # Errors
    ///
    /// Returns an owning refusal when control sampling, owner-bound Arena
    /// selection, or complete local marker reservation fails.
    pub fn try_update_with<F, E>(
        self,
        controls: F,
    ) -> Result<
        (Self, MainDmoPresentationDeferredProducerUpdate),
        MainDmoPresentationDeferredProducerUpdateFailure<C, E>,
    >
    where
        F: FnOnce() -> Result<DeferredCollectionBatchControls, E>,
    {
        match self.state {
            State::Pending(pending) => Ok((
                Self {
                    state: State::Active(Active {
                        prepared: pending.prepared,
                        proxy_collection_id: pending.proxy_collection_id,
                        callback: Some(pending.callback),
                        next_index: 0,
                        queue: pending.queue,
                        reservation_limit: pending.reservation_limit,
                    }),
                },
                MainDmoPresentationDeferredProducerUpdate::SetupCompleted,
            )),
            State::Active(active) => Self::update_active(active, controls),
            State::Terminal(terminal) => Ok((
                Self {
                    state: State::Terminal(terminal),
                },
                MainDmoPresentationDeferredProducerUpdate::TerminalIdle,
            )),
        }
    }

    fn update_active<F, E>(
        mut active: Active<C>,
        controls: F,
    ) -> Result<
        (Self, MainDmoPresentationDeferredProducerUpdate),
        MainDmoPresentationDeferredProducerUpdateFailure<C, E>,
    >
    where
        F: FnOnce() -> Result<DeferredCollectionBatchControls, E>,
    {
        let sampled = match controls() {
            Ok(sampled) => sampled,
            Err(error) => {
                return Err(Self::failure(
                    active,
                    MainDmoPresentationDeferredProducerUpdateError::Controls(error),
                ));
            }
        };
        let selection = match active
            .prepared
            .select_deferred_batch(sampled, active.next_index)
        {
            Ok(selection) => selection,
            Err(error) => {
                return Err(Self::failure(
                    active,
                    MainDmoPresentationDeferredProducerUpdateError::Selection(error),
                ));
            }
        };
        let proxy_collection_queued = selection.completed_this_update
            && active.proxy_collection_id != INVALID_PARENT_COLLECTION_ID;
        let terminal_count = usize::from(selection.completed_this_update)
            .checked_add(usize::from(proxy_collection_queued))
            .and_then(|count| count.checked_add(usize::from(selection.completed_this_update)))
            .expect("the fixed terminal suffix count fits usize");
        let selected_count = usize::try_from(selection.selected_arena_count)
            .expect("the fixed main-bank Arena count fits usize");
        let requested = selected_count
            .checked_add(terminal_count)
            .expect("the fixed main-bank update count fits usize");
        if active
            .reservation_limit
            .is_some_and(|limit| requested > limit)
            || active.queue.try_reserve_exact(requested).is_err()
        {
            return Err(Self::failure(
                active,
                MainDmoPresentationDeferredProducerUpdateError::QueueAllocation { requested },
            ));
        }

        for ordinal in active.next_index..selection.next_index {
            let arena_id = active
                .prepared
                .arena_id(usize::try_from(ordinal).expect("main-bank ordinal fits usize"))
                .expect("owner-bound selection cannot return an absent Arena ordinal");
            active
                .queue
                .push(PresentationDeferredMarker::Arena { ordinal, arena_id });
        }
        active.next_index = selection.next_index;
        if !selection.completed_this_update {
            return Ok((
                Self {
                    state: State::Active(active),
                },
                MainDmoPresentationDeferredProducerUpdate::ArenasQueued {
                    count: selection.selected_arena_count,
                },
            ));
        }

        active
            .queue
            .push(PresentationDeferredMarker::ActivationZeroCollection {
                collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
            });
        if proxy_collection_queued {
            active
                .queue
                .push(PresentationDeferredMarker::ProxyDeregistrationCollection {
                    collection_id: active.proxy_collection_id,
                });
        }
        active
            .queue
            .push(PresentationDeferredMarker::RegistrationCallback(
                active
                    .callback
                    .take()
                    .expect("Active Presentation producer retains its callback"),
            ));
        Ok((
            Self {
                state: State::Terminal(Terminal {
                    _prepared: active.prepared,
                    next_index: active.next_index,
                    queue: active.queue,
                }),
            },
            MainDmoPresentationDeferredProducerUpdate::TerminalQueued {
                arena_count: selection.selected_arena_count,
                proxy_collection_queued,
            },
        ))
    }

    fn failure<E>(
        active: Active<C>,
        error: MainDmoPresentationDeferredProducerUpdateError<E>,
    ) -> MainDmoPresentationDeferredProducerUpdateFailure<C, E> {
        MainDmoPresentationDeferredProducerUpdateFailure {
            error,
            producer: Self {
                state: State::Active(active),
            },
        }
    }

    #[must_use]
    pub fn queued_marker_count(&self) -> usize {
        self.queue().len()
    }

    #[must_use]
    pub fn next_arena_index(&self) -> u32 {
        match &self.state {
            State::Pending(_) => 0,
            State::Active(active) => active.next_index,
            State::Terminal(terminal) => terminal.next_index,
        }
    }

    #[must_use]
    pub fn front_marker_kind(&self) -> Option<MainDmoPresentationDeferredMarkerKind> {
        self.queue().first().map(PresentationDeferredMarker::kind)
    }

    #[must_use]
    pub fn back_marker_kind(&self) -> Option<MainDmoPresentationDeferredMarkerKind> {
        self.queue().last().map(PresentationDeferredMarker::kind)
    }

    #[must_use]
    pub fn marker_kind(&self, index: usize) -> Option<MainDmoPresentationDeferredMarkerKind> {
        self.queue()
            .get(index)
            .map(PresentationDeferredMarker::kind)
    }

    /// Delivers the current FIFO front to the exact refcount-first Arena
    /// receiver gate and removes it only after successful reception.
    ///
    /// A receiver failure leaves the marker at the front. Post-`AddAsset`
    /// failures also fault the adapter, so the retained marker is diagnostic
    /// state and cannot be replayed into a second increment.
    ///
    /// # Errors
    ///
    /// Refuses an empty/non-Arena front without effects, or preserves the
    /// receiver's typed failure and queue front.
    pub fn try_deliver_front_arena<D: PresentationDmoArenaDispatch>(
        &mut self,
        adapter: &mut PresentationDmoArenaRegistrationAdapter<D>,
        registration: &PresentationDmoDeferredCollectionRegistration,
        userdata_raw: u32,
        arena_lookup: &SimulationResidentArenaLookup<DmoBankProcessedArenaAsset>,
    ) -> Result<PresentationDmoDeferredArenaTransition, MainDmoPresentationDeferredArenaDeliveryError>
    {
        let (ordinal, arena_id) = match self.queue().first() {
            Some(PresentationDeferredMarker::Arena { ordinal, arena_id }) => (*ordinal, *arena_id),
            Some(marker) => {
                return Err(
                    MainDmoPresentationDeferredArenaDeliveryError::FrontIsNotArena {
                        actual: marker.kind(),
                    },
                );
            }
            None => return Err(MainDmoPresentationDeferredArenaDeliveryError::QueueEmpty),
        };
        let transition = adapter
            .receive_deferred_arena(registration, ordinal, arena_id, userdata_raw, arena_lookup)
            .map_err(MainDmoPresentationDeferredArenaDeliveryError::Receiver)?;
        let removed = self.queue_mut().remove(0);
        debug_assert!(matches!(
            removed,
            PresentationDeferredMarker::Arena {
                ordinal: removed_ordinal,
                arena_id: removed_id,
            } if removed_ordinal == ordinal && removed_id == arena_id
        ));
        Ok(transition)
    }

    /// Accepts the activation-zero Collection only at the FIFO front and
    /// removes it after the adapter confirms that every Arena was received.
    ///
    /// Native activation zero returns before Presentation singleton access,
    /// registry lookup, iteration, and refcounter mutation. The adapter call
    /// therefore changes only safe-host capability state. The following
    /// callback remains queued and is never invoked by this method.
    ///
    /// # Errors
    ///
    /// Returns an owning refusal with the exact deferred registration when the
    /// front is absent/wrong or the adapter cannot complete it.
    pub fn try_deliver_front_activation_zero<D: PresentationDmoArenaDispatch>(
        &mut self,
        adapter: &mut PresentationDmoArenaRegistrationAdapter<D>,
        registration: PresentationDmoDeferredCollectionRegistration,
    ) -> Result<
        PresentationDmoCollectionRegistration,
        MainDmoPresentationDeferredActivationDeliveryFailure,
    > {
        let collection_id = match self.queue().first() {
            Some(PresentationDeferredMarker::ActivationZeroCollection { collection_id }) => {
                *collection_id
            }
            Some(marker) => {
                return Err(MainDmoPresentationDeferredActivationDeliveryFailure {
                    error: MainDmoPresentationDeferredActivationDeliveryError::FrontIsNotActivationZero {
                        actual: marker.kind(),
                    },
                    registration,
                });
            }
            None => {
                return Err(MainDmoPresentationDeferredActivationDeliveryFailure {
                    error: MainDmoPresentationDeferredActivationDeliveryError::QueueEmpty,
                    registration,
                });
            }
        };
        let expected = CollectionId(DMO_BANK_COLLECTION_ID);
        if collection_id != expected {
            return Err(MainDmoPresentationDeferredActivationDeliveryFailure {
                error: MainDmoPresentationDeferredActivationDeliveryError::UnexpectedCollectionId {
                    expected,
                    actual: collection_id,
                },
                registration,
            });
        }
        let completed = match adapter.finish_deferred_collection(registration) {
            Ok(completed) => completed,
            Err(error) => {
                let kind = error.kind();
                let fault = error.fault();
                return Err(MainDmoPresentationDeferredActivationDeliveryFailure {
                    error: MainDmoPresentationDeferredActivationDeliveryError::Receiver {
                        kind,
                        fault,
                    },
                    registration: error.into_registration(),
                });
            }
        };
        let removed = self.queue_mut().remove(0);
        debug_assert!(matches!(
            removed,
            PresentationDeferredMarker::ActivationZeroCollection {
                collection_id: removed_id,
            } if removed_id == collection_id
        ));
        Ok(completed)
    }

    /// Removes and returns the registration callback only when it is the FIFO
    /// front. Invocation and hull acknowledgement remain caller-owned effects.
    ///
    /// # Errors
    ///
    /// An empty or different front is rejected without changing the queue.
    pub fn try_take_front_registration_callback(
        &mut self,
    ) -> Result<C, MainDmoPresentationDeferredCallbackTakeError> {
        match self.queue().first() {
            Some(PresentationDeferredMarker::RegistrationCallback(_)) => {}
            Some(marker) => {
                return Err(
                    MainDmoPresentationDeferredCallbackTakeError::FrontIsNotRegistrationCallback {
                        actual: marker.kind(),
                    },
                );
            }
            None => return Err(MainDmoPresentationDeferredCallbackTakeError::QueueEmpty),
        }
        let PresentationDeferredMarker::RegistrationCallback(callback) = self.queue_mut().remove(0)
        else {
            unreachable!("front kind was checked before callback extraction")
        };
        Ok(callback)
    }

    fn queue(&self) -> &[PresentationDeferredMarker<C>] {
        match &self.state {
            State::Pending(pending) => &pending.queue,
            State::Active(active) => &active.queue,
            State::Terminal(terminal) => &terminal.queue,
        }
    }

    fn queue_mut(&mut self) -> &mut Vec<PresentationDeferredMarker<C>> {
        match &mut self.state {
            State::Pending(pending) => &mut pending.queue,
            State::Active(active) => &mut active.queue,
            State::Terminal(terminal) => &mut terminal.queue,
        }
    }

    #[cfg(test)]
    fn set_reservation_limit_for_test(&mut self, limit: Option<usize>) {
        match &mut self.state {
            State::Pending(pending) => pending.reservation_limit = limit,
            State::Active(active) => active.reservation_limit = limit,
            State::Terminal(_) => panic!("Terminal producer has no further reservation"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collection_load_pump::LoadedAssetCollection,
        collection_resident_streamer::{ResidentCollectionRegistry, ResidentCollectionStorage},
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
        owned_resident_collection::{OwnedResidentCollectionError, OwnedResidentCollectionStorage},
        presentation_dmo_arena_adapter::{
            PreparedMainDmoBankPresentationArena, PresentationDmoCollectionPrepareError,
            PresentationMaterialDataHandle,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_normal_arena_plan::{SimulationNormalArenaPlan, SimulationRecordHandle},
        simulation_resident_registry::{
            SimulationResidentRegistry, SimulationResidentRegistryError,
        },
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::{cell::Cell, convert::Infallible, rc::Rc, sync::Arc};

    #[derive(Clone)]
    struct DropFlag(Rc<Cell<bool>>);

    #[derive(Default)]
    struct NoopDispatch;

    impl PresentationDmoArenaDispatch for NoopDispatch {
        fn add_material(&mut self, _source: &PresentationMaterialDataHandle) {}
        fn add_living_world_dmo_model(&mut self, _asset_id: u64) {}
        fn register_presentation_dmo_model(
            &mut self,
            _source: &SimulationRecordHandle,
            _asset_id: u64,
        ) {
        }
        fn remove_material(&mut self, _source: &PresentationMaterialDataHandle) {}
        fn remove_living_world_dmo_model(&mut self, _asset_id: u64) {}
        fn deregister_presentation_dmo_model(
            &mut self,
            _source: &SimulationRecordHandle,
            _asset_id: u64,
        ) {
        }
    }

    type DmoStorage = OwnedResidentCollectionStorage<
        DmoBankProcessedArenaAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;
    type DmoRegistry = SimulationResidentRegistry<
        DmoBankProcessedArenaAsset,
        fn(SimulationResidentRegistryError) -> SimulationResidentRegistryError,
        SimulationResidentRegistryError,
    >;

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    fn empty_plan(asset_id: u64) -> Arc<SimulationNormalArenaPlan> {
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
        let asset = SimulationCollisionAsset {
            decoded: DecodedArenaAsset::from_parts_for_test(
                asset_id,
                WORLD_STREAM_ARENA_ASSET_TYPE,
                ArenaResources {
                    resources: [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()],
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
        };
        Arc::new(
            SimulationNormalArenaPlan::from_static_plan(Arc::new(
                SimulationStaticArenaPlan::from_parts_for_test(asset, Vec::new()),
            ))
            .unwrap(),
        )
    }

    fn collection_and_plans() -> (
        PreparedMainDmoBankPresentationCollection,
        Vec<Arc<SimulationNormalArenaPlan>>,
    ) {
        // The production constructor deliberately certifies the real bank.
        // This module only needs the same retained-owner shape to test the
        // producer state machine, so its sibling test helper is unavailable.
        let plans: Vec<_> = (0_u64..125)
            .map(|ordinal| empty_plan(0x100 + ordinal))
            .collect();
        let arenas: Vec<_> = plans
            .iter()
            .map(|plan| PreparedMainDmoBankPresentationArena::from_plan(Arc::clone(plan)).unwrap())
            .collect();
        match PreparedMainDmoBankPresentationCollection::try_from_prepared(&arenas) {
            Err(PresentationDmoCollectionPrepareError::MainDmoBankCensusMismatch { .. }) => {}
            other => panic!("synthetic bank census unexpectedly changed: {other:?}"),
        }
        (
            PreparedMainDmoBankPresentationCollection::from_parts_for_deferred_producer_test(
                &arenas,
            ),
            plans,
        )
    }

    fn collection() -> PreparedMainDmoBankPresentationCollection {
        collection_and_plans().0
    }

    fn update(
        producer: MainDmoPresentationDeferredProducer<DropFlag>,
    ) -> (
        MainDmoPresentationDeferredProducer<DropFlag>,
        MainDmoPresentationDeferredProducerUpdate,
    ) {
        producer
            .try_update_with(|| {
                Ok::<DeferredCollectionBatchControls, Infallible>(
                    DeferredCollectionBatchControls::NATIVE_DEFAULT,
                )
            })
            .unwrap()
    }

    #[test]
    fn pending_is_setup_only_and_default_batches_end_in_exact_suffix() {
        let dropped = Rc::new(Cell::new(false));
        let producer = MainDmoPresentationDeferredProducer::new(
            collection(),
            INVALID_PARENT_COLLECTION_ID,
            DropFlag(Rc::clone(&dropped)),
        );
        let (mut producer, first) = producer
            .try_update_with(|| -> Result<DeferredCollectionBatchControls, Infallible> {
                panic!("Pending must not sample controls")
            })
            .unwrap();
        assert_eq!(
            first,
            MainDmoPresentationDeferredProducerUpdate::SetupCompleted
        );
        assert_eq!(producer.queued_marker_count(), 0);

        for _ in 0..10 {
            let (next, result) = update(producer);
            producer = next;
            assert_eq!(
                result,
                MainDmoPresentationDeferredProducerUpdate::ArenasQueued { count: 12 }
            );
        }
        let (next, result) = update(producer);
        producer = next;
        assert_eq!(
            result,
            MainDmoPresentationDeferredProducerUpdate::TerminalQueued {
                arena_count: 5,
                proxy_collection_queued: false,
            }
        );
        assert_eq!(producer.next_arena_index(), 125);
        assert_eq!(producer.queued_marker_count(), 127);
        assert_eq!(
            producer.front_marker_kind(),
            Some(MainDmoPresentationDeferredMarkerKind::Arena {
                ordinal: 0,
                arena_id: CollectionId(0x100),
            })
        );
        assert_eq!(
            producer.queue()[125].kind(),
            MainDmoPresentationDeferredMarkerKind::ActivationZeroCollection {
                collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
            }
        );
        assert_eq!(
            producer.back_marker_kind(),
            Some(MainDmoPresentationDeferredMarkerKind::RegistrationCallback)
        );
        assert!(!dropped.get());

        let (producer, idle) = producer
            .try_update_with(|| -> Result<DeferredCollectionBatchControls, Infallible> {
                panic!("Terminal must not sample controls")
            })
            .unwrap();
        assert_eq!(
            idle,
            MainDmoPresentationDeferredProducerUpdate::TerminalIdle
        );
        drop(producer);
        assert!(dropped.get());
    }

    #[test]
    fn proxy_deregistration_is_between_primary_and_callback() {
        let proxy = CollectionId(0x1234_5678_9abc_def0);
        let (mut producer, _) = update(MainDmoPresentationDeferredProducer::new(
            collection(),
            proxy,
            DropFlag(Rc::new(Cell::new(false))),
        ));
        while producer.next_arena_index() < 125 {
            (producer, _) = update(producer);
        }
        assert_eq!(producer.queued_marker_count(), 128);
        assert_eq!(
            producer.queue()[125].kind(),
            MainDmoPresentationDeferredMarkerKind::ActivationZeroCollection {
                collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
            }
        );
        assert_eq!(
            producer.queue()[126].kind(),
            MainDmoPresentationDeferredMarkerKind::ProxyDeregistrationCollection {
                collection_id: proxy,
            }
        );
        assert_eq!(
            producer.queue()[127].kind(),
            MainDmoPresentationDeferredMarkerKind::RegistrationCallback
        );
    }

    #[test]
    fn terminal_allocation_refusal_retains_prefix_and_retries() {
        let (mut producer, _) = update(MainDmoPresentationDeferredProducer::new(
            collection(),
            INVALID_PARENT_COLLECTION_ID,
            DropFlag(Rc::new(Cell::new(false))),
        ));
        for _ in 0..10 {
            (producer, _) = update(producer);
        }
        assert_eq!(producer.next_arena_index(), 120);
        assert_eq!(producer.queued_marker_count(), 120);
        producer.set_reservation_limit_for_test(Some(6));

        let Err(failure) = producer.try_update_with(|| {
            Ok::<DeferredCollectionBatchControls, Infallible>(
                DeferredCollectionBatchControls::NATIVE_DEFAULT,
            )
        }) else {
            panic!("terminal reservation limit unexpectedly admitted the update");
        };
        assert!(matches!(
            failure.error(),
            MainDmoPresentationDeferredProducerUpdateError::QueueAllocation { requested: 7 }
        ));
        let mut producer = failure.producer;
        assert_eq!(producer.next_arena_index(), 120);
        assert_eq!(producer.queued_marker_count(), 120);
        producer.set_reservation_limit_for_test(None);
        let (producer, result) = update(producer);
        assert_eq!(
            result,
            MainDmoPresentationDeferredProducerUpdate::TerminalQueued {
                arena_count: 5,
                proxy_collection_queued: false,
            }
        );
        assert_eq!(producer.queued_marker_count(), 127);
    }

    #[test]
    fn arena_delivery_pops_success_and_retains_a_post_add_failure_front() {
        let (prepared, plans) = collection_and_plans();
        let mut producer = MainDmoPresentationDeferredProducer::new(
            prepared.clone(),
            INVALID_PARENT_COLLECTION_ID,
            DropFlag(Rc::new(Cell::new(false))),
        );
        (producer, _) = update(producer);
        (producer, _) = update(producer);
        assert_eq!(producer.queued_marker_count(), 12);

        let mut storage = DmoStorage::new(std::convert::identity);
        let resident = storage.create_empty(1).unwrap();
        storage.writer(&resident).unwrap().add_asset(
            DmoBankProcessedArenaAsset::from_parts_for_test(Arc::clone(&plans[0]), Vec::new()),
        );
        let (mut registry, lookup): (DmoRegistry, _) =
            SimulationResidentRegistry::new(std::convert::identity);
        let arena_lookup = lookup.arena_lookup();
        let wrapper = registry
            .create_wrapper(CollectionId(0x900), &resident)
            .unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        registry.publish(CollectionId(0x900), &token).unwrap();
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(NoopDispatch);
        let deferred = adapter.begin_deferred_collection(&prepared).unwrap();

        assert_eq!(
            producer
                .try_deliver_front_arena(&mut adapter, &deferred, 8, &arena_lookup)
                .unwrap(),
            PresentationDmoDeferredArenaTransition::FirstUseDispatched
        );
        assert_eq!(producer.queued_marker_count(), 11);
        assert_eq!(
            producer.front_marker_kind(),
            Some(MainDmoPresentationDeferredMarkerKind::Arena {
                ordinal: 1,
                arena_id: CollectionId(0x101),
            })
        );

        let error = producer
            .try_deliver_front_arena(&mut adapter, &deferred, 8, &arena_lookup)
            .unwrap_err();
        assert!(matches!(
            error,
            MainDmoPresentationDeferredArenaDeliveryError::Receiver(
                PresentationDmoDeferredArenaReceiveError::PostTransitionFault(fault)
            ) if fault.stage()
                == crate::presentation_dmo_arena_adapter::PresentationDmoAdapterFaultStage::ArenaLookupMissing
        ));
        assert_eq!(producer.queued_marker_count(), 11);
        assert_eq!(
            producer.front_marker_kind(),
            Some(MainDmoPresentationDeferredMarkerKind::Arena {
                ordinal: 1,
                arena_id: CollectionId(0x101),
            })
        );
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
        assert_eq!(adapter.arena_ref_count(CollectionId(0x101)), Some(1));
        drop(wrapper);
        storage.destroy(resident);
    }

    #[test]
    fn activation_zero_and_callback_are_consumed_as_distinct_fifo_fronts() {
        let prepared =
            PreparedMainDmoBankPresentationCollection::from_parts_for_deferred_producer_test(&[]);
        let mut producer = MainDmoPresentationDeferredProducer {
            state: State::Terminal(Terminal {
                _prepared: prepared.clone(),
                next_index: 0,
                queue: vec![
                    PresentationDeferredMarker::Arena {
                        ordinal: 0,
                        arena_id: CollectionId(0x100),
                    },
                    PresentationDeferredMarker::ActivationZeroCollection {
                        collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
                    },
                    PresentationDeferredMarker::RegistrationCallback(9_u32),
                ],
            }),
        };
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(NoopDispatch);
        let deferred = adapter.begin_deferred_collection(&prepared).unwrap();

        assert_eq!(
            producer.try_take_front_registration_callback(),
            Err(
                MainDmoPresentationDeferredCallbackTakeError::FrontIsNotRegistrationCallback {
                    actual: MainDmoPresentationDeferredMarkerKind::Arena {
                        ordinal: 0,
                        arena_id: CollectionId(0x100),
                    },
                }
            )
        );
        let refusal = producer
            .try_deliver_front_activation_zero(&mut adapter, deferred)
            .unwrap_err();
        assert_eq!(
            refusal.error(),
            &MainDmoPresentationDeferredActivationDeliveryError::FrontIsNotActivationZero {
                actual: MainDmoPresentationDeferredMarkerKind::Arena {
                    ordinal: 0,
                    arena_id: CollectionId(0x100),
                },
            }
        );
        let deferred = refusal.into_registration();
        assert_eq!(producer.queued_marker_count(), 3);
        producer.queue_mut().remove(0);

        let registration = producer
            .try_deliver_front_activation_zero(&mut adapter, deferred)
            .unwrap();
        assert_eq!(
            producer.front_marker_kind(),
            Some(MainDmoPresentationDeferredMarkerKind::RegistrationCallback)
        );
        assert_eq!(producer.try_take_front_registration_callback().unwrap(), 9);
        assert_eq!(producer.queued_marker_count(), 0);
        adapter.deregister_collection(registration, 8).unwrap();
        assert_eq!(adapter.live_collection_count(), 0);
    }
}
