//! Main-DMO collection acknowledgement and hull-state coordination.
//!
//! This is the narrow successful one-collection path recovered around
//! `cHullLoader`. Complete Simulation, Location, DMO-pass, and Presentation
//! effects remain opaque ports. Only a complete subsystem implementation may
//! return its callback capability. The pointer-only diagnostic adapter is not
//! such an implementation.

use crate::{
    collection_resident_streamer::{
        CollectionId, CollectionResidentStreamer, CollectionResidentStreamerError,
        ResidentAssetGarbageCollector, ResidentCallbackContext, ResidentCallbackContextError,
        ResidentCallbackObservation, ResidentCollectionCallback, ResidentCollectionHold,
        ResidentCollectionHoldAuthorityError, ResidentCollectionHoldReleaseError,
        ResidentCollectionLoadFactory, ResidentCollectionMetadata, ResidentCollectionRegistry,
        ResidentCollectionStorage, ResidentDecompressionKick,
    },
    dmo_bank_processed_asset::DMO_BANK_COLLECTION_ID,
};
use std::{
    cell::{RefCell, RefMut},
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak as RcWeak},
    sync::{Arc, Weak as SyncWeak},
};

/// Exact main-bank userdata after `cHullLoader::LoadHull` ORs bit 1 at
/// `0x82eeee68` into the manager's asset flags 8.
pub const MAIN_DMO_HULL_USERDATA_RAW: u32 = 0x0a;

/// Exact string hashed by `cHullLoader::LoadHull` at `0x82eeee28..0x82eeee3c`
/// for `cDMOBankManager::LoadMainDMOBank` at `0x82e0e628`. The later
/// `GenerateUniqueID` path at `0x82eecbe0` reconstructs the same string.
pub const MAIN_DMO_HULL_ID_INPUT: &[u8] = b"data\\content\\world\\dmo\\DMO";

/// Raw `cHullLoader::eHullLoadState` values used by the successful main-bank
/// collection path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MainDmoHullRawState {
    LoadingPresentationCollections = 8,
    LoadingTextureCollections = 9,
    LoadCompleteNotify = 10,
    LoadComplete = 11,
    UnloadNotStarted = 12,
    UnloadingCollections = 13,
    UnloadingToc = 14,
    UnloadCompleteNotify = 15,
    UnloadComplete = 16,
}

impl MainDmoHullRawState {
    #[must_use]
    pub const fn raw(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoHullStream {
    Simulation,
    Presentation,
    Texture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MainDmoHullRegistrationRequest {
    pub collection_id: CollectionId,
    pub userdata_raw: u32,
    pub force_immediate: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MainDmoHullDeregistrationRequest {
    pub collection_id: CollectionId,
    pub userdata_raw: u32,
}

/// Opaque complete-subsystem dispatch boundary.
///
/// These calls are `void` in the recovered program. Implementations must
/// finish all subsystem work before returning the supplied callback to the
/// serialized coordinator owner. In particular, the Simulation registration
/// callback requires normal `RegArena`, Location registration, and the
/// restarted `RegDMOArena` pass.
pub trait MainDmoHullSubsystemPort {
    fn register_simulation(
        &mut self,
        request: MainDmoHullRegistrationRequest,
        callback: MainDmoSimulationRegistrationCallback,
    );

    fn register_presentation(
        &mut self,
        request: MainDmoHullRegistrationRequest,
        callback: MainDmoPresentationRegistrationCallback,
    );

    fn deregister_simulation(
        &mut self,
        request: MainDmoHullDeregistrationRequest,
        callback: MainDmoSimulationDeregistrationCallback,
    );

    fn deregister_presentation(
        &mut self,
        request: MainDmoHullDeregistrationRequest,
        callback: MainDmoPresentationDeregistrationCallback,
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoHullDispatchStage {
    SimulationRegistration,
    PresentationRegistration,
    SimulationDeregistration,
    PresentationDeregistration,
    CollectionUnload,
    StreamUnload,
    SimulationStreamUnloadPoll,
    PresentationStreamUnloadPoll,
    TextureStreamUnloadPoll,
    UnloadNotification,
    ResidentHoldRelease,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoHullPhysicalCallbackKind {
    Load,
    Unload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoHullUnloadOutcome {
    Complete,
    Failed,
}

/// The 32-bit ID returned by the hull loader's path/filename hash callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct MainDmoHullId(pub u32);

impl MainDmoHullId {
    /// Exact `MainBankHullID` stored at manager offset `+0x72c0` after the
    /// main DMO bank load request at `0x82e0e678..0x82e0e67c`.
    pub const MAIN_BANK: Self = Self(0x048f_3c1f);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoHullFault {
    CallbackGenerationExhausted,
    ReentrantPortAccess,
    ResidentContext(ResidentCallbackContextError),
    DispatchPanicked(MainDmoHullDispatchStage),
    PhysicalCallbackRejected(MainDmoHullPhysicalCallbackKind),
    ResidentReleaseFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoHullOperationError {
    CoordinatorDropped,
    CoordinatorBusy,
    RepeatedPhysicalCallback,
    WrongCollection {
        expected: CollectionId,
        actual: CollectionId,
    },
    WrongState {
        expected: MainDmoHullRawState,
        actual: Option<MainDmoHullRawState>,
    },
    AcknowledgementsPending {
        completed: u32,
        required: u32,
    },
    OperationInFlight(MainDmoHullDispatchStage),
    ResidentObservationMismatch,
    Faulted(MainDmoHullFault),
}

impl fmt::Display for MainDmoHullOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoordinatorDropped => {
                formatter.write_str("main DMO hull coordinator was dropped")
            }
            Self::CoordinatorBusy => formatter.write_str("main DMO hull coordinator is busy"),
            Self::RepeatedPhysicalCallback => {
                formatter.write_str("main DMO physical callback was repeated")
            }
            Self::WrongCollection { expected, actual } => write!(
                formatter,
                "main DMO callback received {:#018x}, expected {:#018x}",
                actual.0, expected.0
            ),
            Self::WrongState { expected, actual } => write!(
                formatter,
                "main DMO hull state is {actual:?}, expected {expected:?}"
            ),
            Self::AcknowledgementsPending {
                completed,
                required,
            } => write!(
                formatter,
                "main DMO hull has {completed} of {required} deregistration acknowledgements"
            ),
            Self::OperationInFlight(stage) => {
                write!(
                    formatter,
                    "main DMO hull operation is already in flight: {stage:?}"
                )
            }
            Self::ResidentObservationMismatch => formatter.write_str(
                "main DMO physical load callback did not observe a freshly published resident",
            ),
            Self::Faulted(fault) => {
                write!(formatter, "main DMO hull coordinator faulted: {fault:?}")
            }
        }
    }
}

impl std::error::Error for MainDmoHullOperationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainDmoHullCallbackRejectionKind {
    ForeignDomain,
    WrongCollection,
    WrongGeneration,
    WrongState,
    CoordinatorBusy,
    Faulted,
}

pub struct MainDmoHullCallbackRejection<C> {
    kind: MainDmoHullCallbackRejectionKind,
    callback: C,
}

impl<C> MainDmoHullCallbackRejection<C> {
    #[must_use]
    pub const fn kind(&self) -> MainDmoHullCallbackRejectionKind {
        self.kind
    }

    #[must_use]
    pub fn into_callback(self) -> C {
        self.callback
    }
}

impl<C: fmt::Debug> fmt::Debug for MainDmoHullCallbackRejection<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoHullCallbackRejection")
            .field("kind", &self.kind)
            .field("callback", &self.callback)
            .finish()
    }
}

impl<C> fmt::Display for MainDmoHullCallbackRejection<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "main DMO callback was rejected: {:?}", self.kind)
    }
}

impl<C: fmt::Debug> std::error::Error for MainDmoHullCallbackRejection<C> {}

struct CallbackIdentity {
    domain: SyncWeak<()>,
    collection_id: CollectionId,
    generation: u64,
}

impl fmt::Debug for CallbackIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CallbackIdentity")
            .field("collection_id", &self.collection_id)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

macro_rules! define_callback {
    ($name:ident) => {
        pub struct $name {
            identity: CallbackIdentity,
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($name))
                    .field("identity", &self.identity)
                    .finish()
            }
        }
    };
}

define_callback!(MainDmoSimulationRegistrationCallback);
define_callback!(MainDmoPresentationRegistrationCallback);
define_callback!(MainDmoSimulationDeregistrationCallback);
define_callback!(MainDmoPresentationDeregistrationCallback);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MainDmoHullObservation {
    pub state: Option<MainDmoHullRawState>,
    pub last_raw_state: MainDmoHullRawState,
    pub physical_load_remaining: u32,
    pub registration_remaining: u32,
    pub physical_unload_completed: u32,
    pub unregistration_completed: u32,
    pub unload_requested: bool,
    pub released: bool,
    pub fault: Option<MainDmoHullFault>,
}

mod resident_release_seal {
    use super::{
        CollectionResidentStreamer, ResidentAssetGarbageCollector, ResidentCollectionCallback,
        ResidentCollectionLoadFactory, ResidentCollectionMetadata, ResidentCollectionRegistry,
        ResidentCollectionStorage, ResidentDecompressionKick,
    };

    pub trait Sealed {}

    impl<S, M, F, R, G, K, C, E> Sealed for CollectionResidentStreamer<S, M, F, R, G, K, C, E>
    where
        S: ResidentCollectionStorage<E>,
        M: ResidentCollectionMetadata<E>,
        F: ResidentCollectionLoadFactory<S::Writer, M::Descriptor, E>,
        R: ResidentCollectionRegistry<S::Collection, E>,
        G: ResidentAssetGarbageCollector<E>,
        K: ResidentDecompressionKick<E>,
        C: ResidentCollectionCallback<E>,
    {
    }
}

/// Sealed authority for consuming an occurrence-bound resident hold.
///
/// Only [`CollectionResidentStreamer`] implements this trait, preventing an
/// integration from acknowledging `DReg` without executing the proven Down.
pub trait MainDmoResidentHoldReleaser: resident_release_seal::Sealed {
    type Error;

    /// Executes the streamer's occurrence-checked resident decrement.
    ///
    /// # Errors
    ///
    /// Returns the streamer's typed authority, availability, or teardown failure.
    fn release_main_dmo_hold(
        &mut self,
        hold: ResidentCollectionHold,
    ) -> Result<bool, ResidentCollectionHoldReleaseError<Self::Error>>;
}

impl<S, M, F, R, G, K, C, E> MainDmoResidentHoldReleaser
    for CollectionResidentStreamer<S, M, F, R, G, K, C, E>
where
    S: ResidentCollectionStorage<E>,
    M: ResidentCollectionMetadata<E>,
    F: ResidentCollectionLoadFactory<S::Writer, M::Descriptor, E>,
    R: ResidentCollectionRegistry<S::Collection, E>,
    G: ResidentAssetGarbageCollector<E>,
    K: ResidentDecompressionKick<E>,
    C: ResidentCollectionCallback<E>,
{
    type Error = E;

    fn release_main_dmo_hold(
        &mut self,
        hold: ResidentCollectionHold,
    ) -> Result<bool, ResidentCollectionHoldReleaseError<Self::Error>> {
        self.release_collection_hold(hold)
    }
}

struct MainDmoHullState {
    domain: Arc<()>,
    hull_id: MainDmoHullId,
    state: Option<MainDmoHullRawState>,
    last_raw_state: MainDmoHullRawState,
    next_callback_generation: u64,
    physical_load_remaining: u32,
    registration_remaining: u32,
    physical_unload_completed: u32,
    unregistration_completed: u32,
    unload_requested: bool,
    in_flight: Option<MainDmoHullDispatchStage>,
    sim_hold: Option<ResidentCollectionHold>,
    pres_hold: Option<ResidentCollectionHold>,
    sim_reg_generation: Option<u64>,
    pres_reg_generation: Option<u64>,
    sim_dreg_generation: Option<u64>,
    pres_dreg_generation: Option<u64>,
    fault: Option<MainDmoHullFault>,
}

impl MainDmoHullState {
    fn reserve_callback_pair(&mut self) -> Result<(u64, u64), MainDmoHullFault> {
        let first = self.next_callback_generation;
        let second = first
            .checked_add(1)
            .ok_or(MainDmoHullFault::CallbackGenerationExhausted)?;
        self.next_callback_generation = first
            .checked_add(2)
            .ok_or(MainDmoHullFault::CallbackGenerationExhausted)?;
        Ok((first, second))
    }

    fn observation(&self) -> MainDmoHullObservation {
        MainDmoHullObservation {
            state: self.state,
            last_raw_state: self.last_raw_state,
            physical_load_remaining: self.physical_load_remaining,
            registration_remaining: self.registration_remaining,
            physical_unload_completed: self.physical_unload_completed,
            unregistration_completed: self.unregistration_completed,
            unload_requested: self.unload_requested,
            released: self.state.is_none(),
            fault: self.fault,
        }
    }
}

/// Owner-thread coordinator for the successful main-DMO hull path.
///
/// Callback capabilities are `Send`, but acknowledgements are deliberately
/// applied through this non-`Send` serialized owner. The subsystem port is
/// dropped before resident-hold state if this coordinator is abandoned.
pub struct MainDmoHullAcknowledgementCoordinator<P> {
    port: Rc<RefCell<P>>,
    state: Rc<RefCell<MainDmoHullState>>,
}

impl<P: MainDmoHullSubsystemPort> MainDmoHullAcknowledgementCoordinator<P> {
    #[must_use]
    pub fn new(port: P, hull_id: MainDmoHullId) -> (Self, MainDmoHullResidentCallback<P>) {
        let state = Rc::new(RefCell::new(MainDmoHullState {
            domain: Arc::new(()),
            hull_id,
            state: Some(MainDmoHullRawState::LoadingPresentationCollections),
            last_raw_state: MainDmoHullRawState::LoadingPresentationCollections,
            next_callback_generation: 0,
            physical_load_remaining: 1,
            registration_remaining: 2,
            physical_unload_completed: 0,
            unregistration_completed: 0,
            unload_requested: false,
            in_flight: None,
            sim_hold: None,
            pres_hold: None,
            sim_reg_generation: None,
            pres_reg_generation: None,
            sim_dreg_generation: None,
            pres_dreg_generation: None,
            fault: None,
        }));
        let port = Rc::new(RefCell::new(port));
        let callback = MainDmoHullResidentCallback::Load(MainDmoHullLoadCallback {
            state: Rc::downgrade(&state),
            port: Rc::downgrade(&port),
            completed: false,
        });
        (Self { port, state }, callback)
    }

    /// Issues an exact-domain Simulation registration capability for private
    /// cross-module ownership tests without adding a production minting seam.
    #[cfg(test)]
    pub(crate) fn issue_simulation_registration_callback_for_test(
        &self,
    ) -> MainDmoSimulationRegistrationCallback {
        let mut state = self.state.borrow_mut();
        assert!(
            state.sim_reg_generation.is_none(),
            "test Simulation registration callback is already outstanding"
        );
        let (generation, _) = state
            .reserve_callback_pair()
            .expect("test callback generation remains available");
        state.sim_reg_generation = Some(generation);
        MainDmoSimulationRegistrationCallback {
            identity: callback_identity(&state, generation),
        }
    }

    /// Issues an exact-domain opaque Simulation deregistration capability for
    /// private handoff tests. The production capability remains physical-load
    /// callback owned.
    #[cfg(test)]
    pub(crate) fn issue_simulation_deregistration_callback_for_test(
        &self,
    ) -> MainDmoSimulationDeregistrationCallback {
        let mut state = self.state.borrow_mut();
        assert!(
            state.sim_dreg_generation.is_none(),
            "test Simulation deregistration callback is already outstanding"
        );
        let (generation, _) = state
            .reserve_callback_pair()
            .expect("test callback generation remains available");
        state.sim_dreg_generation = Some(generation);
        MainDmoSimulationDeregistrationCallback {
            identity: callback_identity(&state, generation),
        }
    }

    /// Returns the coordinator's typed counters, state, and terminal fault.
    ///
    /// # Errors
    ///
    /// Returns a typed reentrant-access fault while coordinator state is borrowed.
    pub fn observation(&self) -> Result<MainDmoHullObservation, MainDmoHullOperationError> {
        Ok(self.borrow_state()?.observation())
    }

    /// Returns the current native raw state, or `None` after map-entry erasure.
    ///
    /// # Errors
    ///
    /// Returns a typed reentrant-access fault while coordinator state is borrowed.
    pub fn raw_state(&self) -> Result<Option<MainDmoHullRawState>, MainDmoHullOperationError> {
        Ok(self.borrow_state()?.state)
    }

    /// Implements the recovered state-10/state-11 loaded predicate.
    ///
    /// # Errors
    ///
    /// Returns a typed reentrant-access fault while coordinator state is borrowed.
    pub fn is_loaded(&self) -> Result<bool, MainDmoHullOperationError> {
        Ok(matches!(
            self.borrow_state()?.state,
            Some(MainDmoHullRawState::LoadCompleteNotify | MainDmoHullRawState::LoadComplete)
        ))
    }

    /// Implements the recovered absent/state-15/state-16 unloaded predicate.
    ///
    /// # Errors
    ///
    /// Returns a typed reentrant-access fault while coordinator state is borrowed.
    pub fn is_unloaded(&self) -> Result<bool, MainDmoHullOperationError> {
        Ok(matches!(
            self.borrow_state()?.state,
            None | Some(
                MainDmoHullRawState::UnloadCompleteNotify | MainDmoHullRawState::UnloadComplete
            )
        ))
    }

    /// Runs at most one raw-state case for states 8 through 12, anchored in
    /// `cHullLoader::Update` at `0x82eedc40`.
    ///
    /// # Errors
    ///
    /// Returns the latched fault or a typed reentrant-access fault.
    pub fn update(&self) -> Result<MainDmoHullObservation, MainDmoHullOperationError> {
        let mut state = self.borrow_state()?;
        check_fault(&state)?;
        match state.state {
            Some(MainDmoHullRawState::LoadingPresentationCollections)
                if state.physical_load_remaining == 0 && state.registration_remaining == 0 =>
            {
                set_raw_state(&mut state, MainDmoHullRawState::LoadingTextureCollections);
            }
            Some(MainDmoHullRawState::LoadingTextureCollections) => {
                set_raw_state(&mut state, MainDmoHullRawState::LoadCompleteNotify);
            }
            Some(MainDmoHullRawState::LoadCompleteNotify) => {
                set_raw_state(&mut state, MainDmoHullRawState::LoadComplete);
            }
            Some(MainDmoHullRawState::LoadComplete) if state.unload_requested => {
                set_raw_state(&mut state, MainDmoHullRawState::UnloadNotStarted);
            }
            _ => {}
        }
        Ok(state.observation())
    }

    /// Records the explicit successful-main-path equivalent of `UnloadHull`
    /// at `0x82eee9c0`.
    /// No automatic main-bank shutdown policy is inferred.
    ///
    /// # Errors
    ///
    /// Returns the latched fault, a typed reentrant-access fault, or a wrong-state error.
    pub fn request_unload(&self) -> Result<(), MainDmoHullOperationError> {
        let mut state = self.borrow_state()?;
        check_fault(&state)?;
        require_state(&state, MainDmoHullRawState::LoadComplete)?;
        state.unload_requested = true;
        Ok(())
    }

    /// Executes the frozen main bank's Presentation-only state-12 collection
    /// unload submission (`0x82eee2fc`) before publishing state 13.
    ///
    /// This is a high-confidence bounded composition of the native
    /// Simulation/Presentation/Texture arbitration, whose complete policy is
    /// not represented by this coordinator.
    ///
    /// # Errors
    ///
    /// Returns a state/fault error or latches a panic from the submission boundary.
    pub fn start_collection_unload(
        &self,
        enqueue: impl FnOnce(CollectionId, MainDmoHullResidentCallback<P>),
    ) -> Result<(), MainDmoHullOperationError> {
        {
            let mut state = self.borrow_state()?;
            check_fault(&state)?;
            require_state(&state, MainDmoHullRawState::UnloadNotStarted)?;
            if let Some(stage) = state.in_flight {
                return Err(MainDmoHullOperationError::OperationInFlight(stage));
            }
            state.in_flight = Some(MainDmoHullDispatchStage::CollectionUnload);
        }
        let callback = MainDmoHullResidentCallback::Unload(MainDmoHullUnloadCallback {
            state: Rc::downgrade(&self.state),
            port: Rc::downgrade(&self.port),
            completed: false,
        });
        if catch_unwind(AssertUnwindSafe(|| {
            enqueue(CollectionId(DMO_BANK_COLLECTION_ID), callback);
        }))
        .is_err()
        {
            let fault = self.latch_fault(MainDmoHullFault::DispatchPanicked(
                MainDmoHullDispatchStage::CollectionUnload,
            ))?;
            return Err(MainDmoHullOperationError::Faulted(fault));
        }
        let mut state = self.borrow_state()?;
        check_fault(&state)?;
        require_state(&state, MainDmoHullRawState::UnloadNotStarted)?;
        state.in_flight = None;
        set_raw_state(&mut state, MainDmoHullRawState::UnloadingCollections);
        Ok(())
    }

    /// Runs the state-13 stream-unload request after both `DReg`
    /// acknowledgements (`0x82eee3a4` onward).
    ///
    /// # Errors
    ///
    /// Returns a state/fault/pending-acknowledgement error or latches a request panic.
    pub fn request_stream_unload(
        &self,
        request: impl FnOnce(MainDmoHullStream),
    ) -> Result<(), MainDmoHullOperationError> {
        {
            let mut state = self.borrow_state()?;
            check_fault(&state)?;
            require_state(&state, MainDmoHullRawState::UnloadingCollections)?;
            if state.unregistration_completed != 2 {
                return Err(MainDmoHullOperationError::AcknowledgementsPending {
                    completed: state.unregistration_completed,
                    required: 2,
                });
            }
            if let Some(stage) = state.in_flight {
                return Err(MainDmoHullOperationError::OperationInFlight(stage));
            }
            state.in_flight = Some(MainDmoHullDispatchStage::StreamUnload);
        }
        if catch_unwind(AssertUnwindSafe(|| {
            request(MainDmoHullStream::Presentation);
        }))
        .is_err()
        {
            let fault = self.latch_fault(MainDmoHullFault::DispatchPanicked(
                MainDmoHullDispatchStage::StreamUnload,
            ))?;
            return Err(MainDmoHullOperationError::Faulted(fault));
        }
        let mut state = self.borrow_state()?;
        check_fault(&state)?;
        require_state(&state, MainDmoHullRawState::UnloadingCollections)?;
        state.in_flight = None;
        set_raw_state(&mut state, MainDmoHullRawState::UnloadingToc);
        Ok(())
    }

    /// Runs the exact three state-14 unload polls without short-circuiting at
    /// `0x82eee4a0..0x82eee4ec`.
    ///
    /// # Errors
    ///
    /// Returns a state/fault error or latches a panic from any poll.
    pub fn poll_stream_unloaded(
        &self,
        mut poll: impl FnMut(MainDmoHullStream) -> bool,
    ) -> Result<bool, MainDmoHullOperationError> {
        {
            let mut state = self.borrow_state()?;
            check_fault(&state)?;
            require_state(&state, MainDmoHullRawState::UnloadingToc)?;
            if let Some(stage) = state.in_flight {
                return Err(MainDmoHullOperationError::OperationInFlight(stage));
            }
            state.in_flight = Some(MainDmoHullDispatchStage::SimulationStreamUnloadPoll);
        }
        let simulation = self.poll_one_stream(
            &mut poll,
            MainDmoHullStream::Simulation,
            MainDmoHullDispatchStage::SimulationStreamUnloadPoll,
        )?;
        let presentation = self.poll_one_stream(
            &mut poll,
            MainDmoHullStream::Presentation,
            MainDmoHullDispatchStage::PresentationStreamUnloadPoll,
        )?;
        let texture = self.poll_one_stream(
            &mut poll,
            MainDmoHullStream::Texture,
            MainDmoHullDispatchStage::TextureStreamUnloadPoll,
        )?;
        let values = [simulation, presentation, texture];
        let complete = values.into_iter().all(std::convert::identity);
        let mut state = self.borrow_state()?;
        check_fault(&state)?;
        require_state(&state, MainDmoHullRawState::UnloadingToc)?;
        state.in_flight = None;
        if complete {
            set_raw_state(&mut state, MainDmoHullRawState::UnloadCompleteNotify);
        }
        Ok(complete)
    }

    /// Runs the optional state-15 notification, then falls through raw 16 and
    /// map-entry erasure in one owner-thread call, matching
    /// `0x82eee4f8..0x82eee558`.
    ///
    /// Whether the main-bank unload caller supplies this callback remains
    /// open. `outcome` selects the recovered `OnComplete` versus `OnFail`
    /// dispatch; callers may deliberately select a bounded no-callback host
    /// composition with `None`.
    ///
    /// # Errors
    ///
    /// Returns the latched fault, a typed reentrant-access fault, or a wrong-state error.
    pub fn finish_unload_notification<N>(
        &self,
        outcome: MainDmoHullUnloadOutcome,
        notify: Option<N>,
    ) -> Result<(), MainDmoHullOperationError>
    where
        N: FnOnce(MainDmoHullId, MainDmoHullUnloadOutcome),
    {
        {
            let mut state = self.borrow_state()?;
            check_fault(&state)?;
            require_state(&state, MainDmoHullRawState::UnloadCompleteNotify)?;
            if let Some(stage) = state.in_flight {
                return Err(MainDmoHullOperationError::OperationInFlight(stage));
            }
            state.in_flight = Some(MainDmoHullDispatchStage::UnloadNotification);
        }
        let hull_id = self.borrow_state()?.hull_id;
        if let Some(notify) = notify {
            if catch_unwind(AssertUnwindSafe(|| notify(hull_id, outcome))).is_err() {
                let fault = self.latch_fault(MainDmoHullFault::DispatchPanicked(
                    MainDmoHullDispatchStage::UnloadNotification,
                ))?;
                return Err(MainDmoHullOperationError::Faulted(fault));
            }
        }
        let mut state = self.borrow_state()?;
        check_fault(&state)?;
        require_state(&state, MainDmoHullRawState::UnloadCompleteNotify)?;
        state.in_flight = None;
        state.last_raw_state = MainDmoHullRawState::UnloadComplete;
        state.state = None;
        Ok(())
    }

    /// Applies one Simulation registration completion from the shared-latch
    /// callback at `0x82eed548`.
    ///
    /// # Errors
    ///
    /// Returns the untouched capability when its authority, state, or generation is invalid.
    pub fn acknowledge_simulation_registration(
        &self,
        callback: MainDmoSimulationRegistrationCallback,
    ) -> Result<(), MainDmoHullCallbackRejection<MainDmoSimulationRegistrationCallback>> {
        self.acknowledge_registration(callback, |state| &mut state.sim_reg_generation)
    }

    /// Applies one Presentation registration completion from the shared-latch
    /// callback at `0x82eed548`.
    ///
    /// # Errors
    ///
    /// Returns the untouched capability when its authority, state, or generation is invalid.
    pub fn acknowledge_presentation_registration(
        &self,
        callback: MainDmoPresentationRegistrationCallback,
    ) -> Result<(), MainDmoHullCallbackRejection<MainDmoPresentationRegistrationCallback>> {
        self.acknowledge_registration(callback, |state| &mut state.pres_reg_generation)
    }

    /// Releases the Simulation occurrence hold, then advances the shared
    /// `DReg` latch in the order recovered at `0x82eed580`.
    ///
    /// # Errors
    ///
    /// Returns the callback on recoverable authority/availability failures and consumes it on a
    /// release-time streamer fault.
    pub fn acknowledge_simulation_deregistration<R>(
        &self,
        callback: MainDmoSimulationDeregistrationCallback,
        releaser: &mut R,
    ) -> Result<
        bool,
        MainDmoHullDeregistrationAckError<R::Error, MainDmoSimulationDeregistrationCallback>,
    >
    where
        R: MainDmoResidentHoldReleaser,
    {
        self.acknowledge_deregistration(
            callback,
            |state| &mut state.sim_dreg_generation,
            |state| &mut state.sim_hold,
            releaser,
        )
    }

    /// Releases the Presentation occurrence hold, then advances the shared
    /// `DReg` latch in the order recovered at `0x82eed580`.
    ///
    /// # Errors
    ///
    /// Returns the callback on recoverable authority/availability failures and consumes it on a
    /// release-time streamer fault.
    pub fn acknowledge_presentation_deregistration<R>(
        &self,
        callback: MainDmoPresentationDeregistrationCallback,
        releaser: &mut R,
    ) -> Result<
        bool,
        MainDmoHullDeregistrationAckError<R::Error, MainDmoPresentationDeregistrationCallback>,
    >
    where
        R: MainDmoResidentHoldReleaser,
    {
        self.acknowledge_deregistration(
            callback,
            |state| &mut state.pres_dreg_generation,
            |state| &mut state.pres_hold,
            releaser,
        )
    }

    fn acknowledge_registration<C>(
        &self,
        callback: C,
        expected: impl Fn(&mut MainDmoHullState) -> &mut Option<u64>,
    ) -> Result<(), MainDmoHullCallbackRejection<C>>
    where
        C: CallbackCapability,
    {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return Err(MainDmoHullCallbackRejection {
                kind: MainDmoHullCallbackRejectionKind::CoordinatorBusy,
                callback,
            });
        };
        let expected_generation = *expected(&mut state);
        if let Err(kind) = validate_callback(
            &state,
            callback.identity(),
            expected_generation,
            MainDmoHullRawState::LoadingPresentationCollections,
        ) {
            return Err(MainDmoHullCallbackRejection { kind, callback });
        }
        *expected(&mut state) = None;
        state.registration_remaining = state.registration_remaining.wrapping_sub(1);
        Ok(())
    }

    fn acknowledge_deregistration<R, C>(
        &self,
        callback: C,
        expected: impl Fn(&mut MainDmoHullState) -> &mut Option<u64>,
        hold_slot: impl Fn(&mut MainDmoHullState) -> &mut Option<ResidentCollectionHold>,
        releaser: &mut R,
    ) -> Result<bool, MainDmoHullDeregistrationAckError<R::Error, C>>
    where
        R: MainDmoResidentHoldReleaser,
        C: CallbackCapability,
    {
        let hold = {
            let Ok(mut state) = self.state.try_borrow_mut() else {
                return Err(MainDmoHullDeregistrationAckError::Callback(
                    MainDmoHullCallbackRejection {
                        kind: MainDmoHullCallbackRejectionKind::CoordinatorBusy,
                        callback,
                    },
                ));
            };
            let expected_generation = *expected(&mut state);
            if let Err(kind) = validate_callback(
                &state,
                callback.identity(),
                expected_generation,
                MainDmoHullRawState::UnloadingCollections,
            ) {
                return Err(MainDmoHullDeregistrationAckError::Callback(
                    MainDmoHullCallbackRejection { kind, callback },
                ));
            }
            hold_slot(&mut state)
                .take()
                .expect("validated DMO deregistration callback must retain its resident hold")
        };

        let release_result =
            catch_unwind(AssertUnwindSafe(|| releaser.release_main_dmo_hold(hold)));
        let Ok(release_result) = release_result else {
            let mut state = self.state.borrow_mut();
            *expected(&mut state) = None;
            state.fault = Some(MainDmoHullFault::DispatchPanicked(
                MainDmoHullDispatchStage::ResidentHoldRelease,
            ));
            return Err(MainDmoHullDeregistrationAckError::ResidentReleasePanicked);
        };
        match release_result {
            Ok(destroyed) => {
                let mut state = self.state.borrow_mut();
                *expected(&mut state) = None;
                state.unregistration_completed = state.unregistration_completed.wrapping_add(1);
                Ok(destroyed)
            }
            Err(ResidentCollectionHoldReleaseError::Authority { kind, hold }) => {
                hold_slot(&mut self.state.borrow_mut()).replace(hold);
                Err(MainDmoHullDeregistrationAckError::ResidentAuthority { kind, callback })
            }
            Err(ResidentCollectionHoldReleaseError::StreamerUnavailable { source, hold }) => {
                hold_slot(&mut self.state.borrow_mut()).replace(hold);
                Err(MainDmoHullDeregistrationAckError::ResidentUnavailable { source, callback })
            }
            Err(ResidentCollectionHoldReleaseError::Streamer(source)) => {
                let mut state = self.state.borrow_mut();
                *expected(&mut state) = None;
                state.fault = Some(MainDmoHullFault::ResidentReleaseFailed);
                Err(MainDmoHullDeregistrationAckError::ResidentRelease(source))
            }
        }
    }

    fn poll_one_stream(
        &self,
        poll: &mut impl FnMut(MainDmoHullStream) -> bool,
        stream: MainDmoHullStream,
        dispatch_stage: MainDmoHullDispatchStage,
    ) -> Result<bool, MainDmoHullOperationError> {
        {
            let mut state = self.borrow_state()?;
            check_fault(&state)?;
            require_state(&state, MainDmoHullRawState::UnloadingToc)?;
            state.in_flight = Some(dispatch_stage);
        }
        if let Ok(value) = catch_unwind(AssertUnwindSafe(|| poll(stream))) {
            Ok(value)
        } else {
            let fault = self.latch_fault(MainDmoHullFault::DispatchPanicked(dispatch_stage))?;
            Err(MainDmoHullOperationError::Faulted(fault))
        }
    }

    fn borrow_state(&self) -> Result<RefMut<'_, MainDmoHullState>, MainDmoHullOperationError> {
        self.state
            .try_borrow_mut()
            .map_err(|_| MainDmoHullOperationError::CoordinatorBusy)
    }

    fn latch_fault(
        &self,
        fault: MainDmoHullFault,
    ) -> Result<MainDmoHullFault, MainDmoHullOperationError> {
        let mut state = self.borrow_state()?;
        if state.fault.is_none() {
            state.fault = Some(fault);
        }
        Ok(state.fault.expect("fault was installed or already present"))
    }
}

trait CallbackCapability {
    fn identity(&self) -> &CallbackIdentity;
}

macro_rules! impl_callback_capability {
    ($($name:ident),+ $(,)?) => {
        $(
            impl CallbackCapability for $name {
                fn identity(&self) -> &CallbackIdentity {
                    &self.identity
                }
            }
        )+
    };
}

impl_callback_capability!(
    MainDmoSimulationRegistrationCallback,
    MainDmoPresentationRegistrationCallback,
    MainDmoSimulationDeregistrationCallback,
    MainDmoPresentationDeregistrationCallback,
);

pub enum MainDmoHullDeregistrationAckError<E, C> {
    Callback(MainDmoHullCallbackRejection<C>),
    ResidentAuthority {
        kind: ResidentCollectionHoldAuthorityError,
        callback: C,
    },
    ResidentUnavailable {
        source: Arc<CollectionResidentStreamerError<E>>,
        callback: C,
    },
    ResidentRelease(Arc<CollectionResidentStreamerError<E>>),
    ResidentReleasePanicked,
}

impl<E, C> MainDmoHullDeregistrationAckError<E, C> {
    #[must_use]
    pub fn into_callback(self) -> Option<C> {
        match self {
            Self::Callback(rejection) => Some(rejection.into_callback()),
            Self::ResidentAuthority { callback, .. }
            | Self::ResidentUnavailable { callback, .. } => Some(callback),
            Self::ResidentRelease(_) | Self::ResidentReleasePanicked => None,
        }
    }
}

impl<E, C: fmt::Debug> fmt::Debug for MainDmoHullDeregistrationAckError<E, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Callback(source) => formatter.debug_tuple("Callback").field(source).finish(),
            Self::ResidentAuthority { kind, callback } => formatter
                .debug_struct("ResidentAuthority")
                .field("kind", kind)
                .field("callback", callback)
                .finish(),
            Self::ResidentUnavailable { callback, .. } => formatter
                .debug_struct("ResidentUnavailable")
                .field("callback", callback)
                .finish_non_exhaustive(),
            Self::ResidentRelease(_) => formatter
                .debug_tuple("ResidentRelease")
                .field(&"streamer fault")
                .finish(),
            Self::ResidentReleasePanicked => formatter.write_str("ResidentReleasePanicked"),
        }
    }
}

impl<E: fmt::Display, C> fmt::Display for MainDmoHullDeregistrationAckError<E, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Callback(source) => source.fmt(formatter),
            Self::ResidentAuthority { kind, .. } => {
                write!(formatter, "main DMO resident hold was rejected: {kind:?}")
            }
            Self::ResidentUnavailable { source, .. } | Self::ResidentRelease(source) => {
                source.fmt(formatter)
            }
            Self::ResidentReleasePanicked => {
                formatter.write_str("main DMO resident-hold release panicked")
            }
        }
    }
}

impl<E, C> std::error::Error for MainDmoHullDeregistrationAckError<E, C>
where
    E: fmt::Debug + fmt::Display + 'static,
    C: fmt::Debug + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Callback(source) => Some(source),
            Self::ResidentUnavailable { source, .. } | Self::ResidentRelease(source) => {
                Some(source)
            }
            Self::ResidentAuthority { .. } | Self::ResidentReleasePanicked => None,
        }
    }
}

pub enum MainDmoHullResidentCallback<P> {
    Load(MainDmoHullLoadCallback<P>),
    Unload(MainDmoHullUnloadCallback<P>),
}

pub struct MainDmoHullLoadCallback<P> {
    state: RcWeak<RefCell<MainDmoHullState>>,
    port: RcWeak<RefCell<P>>,
    completed: bool,
}

pub struct MainDmoHullUnloadCallback<P> {
    state: RcWeak<RefCell<MainDmoHullState>>,
    port: RcWeak<RefCell<P>>,
    completed: bool,
}

impl<P: MainDmoHullSubsystemPort> MainDmoHullResidentCallback<P> {
    /// Dispatches this DMO callback through an outer application callback
    /// enum while retaining the outer streamer's callback context.
    ///
    /// The adapter exposes no queue ownership. Its load variant can acquire
    /// the two occurrence-bound holds from the active outer callback context;
    /// its unload variant preserves the same physical-callback ordering.
    ///
    /// # Errors
    ///
    /// Maps the first DMO callback failure into the outer streamer's error
    /// domain without rolling back an already applied native prefix.
    pub fn dispatch_mapped<C, E, M>(
        &mut self,
        id: CollectionId,
        observation: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, C>,
        map_error: M,
    ) -> Result<(), E>
    where
        M: FnOnce(MainDmoHullOperationError) -> E,
    {
        let result = match self {
            Self::Load(callback) => callback.complete(id, observation, context),
            Self::Unload(callback) => callback.complete(id),
        };
        result.map_err(map_error)
    }
}

impl<P: MainDmoHullSubsystemPort> ResidentCollectionCallback<MainDmoHullOperationError>
    for MainDmoHullResidentCallback<P>
{
    fn on_complete(
        &mut self,
        id: CollectionId,
        observation: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, Self>,
    ) -> Result<(), MainDmoHullOperationError> {
        self.dispatch_mapped(id, observation, context, std::convert::identity)
    }
}

impl<P: MainDmoHullSubsystemPort> MainDmoHullLoadCallback<P> {
    fn complete<C>(
        &mut self,
        id: CollectionId,
        observation: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, C>,
    ) -> Result<(), MainDmoHullOperationError> {
        if self.completed {
            if let Some(state) = self.state.upgrade() {
                latch_shared_fault(
                    &state,
                    MainDmoHullFault::PhysicalCallbackRejected(
                        MainDmoHullPhysicalCallbackKind::Load,
                    ),
                );
            }
            return Err(MainDmoHullOperationError::RepeatedPhysicalCallback);
        }
        self.completed = true;
        let state_owner = self
            .state
            .upgrade()
            .ok_or(MainDmoHullOperationError::CoordinatorDropped)?;
        let port_owner = self
            .port
            .upgrade()
            .ok_or(MainDmoHullOperationError::CoordinatorDropped)?;
        let (sim_callback, pres_callback) = {
            let mut state = state_owner
                .try_borrow_mut()
                .map_err(|_| MainDmoHullOperationError::CoordinatorBusy)?;
            check_fault(&state)?;
            if let Err(error) =
                require_state(&state, MainDmoHullRawState::LoadingPresentationCollections)
            {
                state.fault = Some(MainDmoHullFault::PhysicalCallbackRejected(
                    MainDmoHullPhysicalCallbackKind::Load,
                ));
                return Err(error);
            }
            if let Err(error) = require_collection(id) {
                state.fault = Some(MainDmoHullFault::PhysicalCallbackRejected(
                    MainDmoHullPhysicalCallbackKind::Load,
                ));
                return Err(error);
            }
            state.physical_load_remaining = state.physical_load_remaining.wrapping_sub(1);
            // The native callback does not inspect this count. Requiring a
            // freshly published resident is a high-confidence host
            // integration precondition for the frozen main-bank load, not an
            // exact field check attributed to `cHullCollectionLoadingCallback`.
            if observation.resident_ref_count != Some(1) || !observation.wrapper_is_published {
                state.fault = Some(MainDmoHullFault::PhysicalCallbackRejected(
                    MainDmoHullPhysicalCallbackKind::Load,
                ));
                return Err(MainDmoHullOperationError::ResidentObservationMismatch);
            }
            let (sim_generation, pres_generation) = match state.reserve_callback_pair() {
                Ok(pair) => pair,
                Err(fault) => {
                    state.fault = Some(fault);
                    return Err(MainDmoHullOperationError::Faulted(fault));
                }
            };
            state.sim_reg_generation = Some(sim_generation);
            state.pres_reg_generation = Some(pres_generation);
            (
                MainDmoSimulationRegistrationCallback {
                    identity: callback_identity(&state, sim_generation),
                },
                MainDmoPresentationRegistrationCallback {
                    identity: callback_identity(&state, pres_generation),
                },
            )
        };

        let (_, sim_hold) = context.acquire_active_collection_hold().map_err(|source| {
            latch_shared_fault(&state_owner, MainDmoHullFault::ResidentContext(source));
            MainDmoHullOperationError::Faulted(MainDmoHullFault::ResidentContext(source))
        })?;
        state_owner.borrow_mut().sim_hold = Some(sim_hold);
        dispatch_port(
            &state_owner,
            &port_owner,
            MainDmoHullDispatchStage::SimulationRegistration,
            |port| port.register_simulation(registration_request(), sim_callback),
        )?;

        let (_, pres_hold) = context.acquire_active_collection_hold().map_err(|source| {
            latch_shared_fault(&state_owner, MainDmoHullFault::ResidentContext(source));
            MainDmoHullOperationError::Faulted(MainDmoHullFault::ResidentContext(source))
        })?;
        state_owner.borrow_mut().pres_hold = Some(pres_hold);
        dispatch_port(
            &state_owner,
            &port_owner,
            MainDmoHullDispatchStage::PresentationRegistration,
            |port| port.register_presentation(registration_request(), pres_callback),
        )
    }
}

impl<P: MainDmoHullSubsystemPort> MainDmoHullUnloadCallback<P> {
    fn complete(&mut self, id: CollectionId) -> Result<(), MainDmoHullOperationError> {
        if self.completed {
            if let Some(state) = self.state.upgrade() {
                latch_shared_fault(
                    &state,
                    MainDmoHullFault::PhysicalCallbackRejected(
                        MainDmoHullPhysicalCallbackKind::Unload,
                    ),
                );
            }
            return Err(MainDmoHullOperationError::RepeatedPhysicalCallback);
        }
        self.completed = true;
        let state_owner = self
            .state
            .upgrade()
            .ok_or(MainDmoHullOperationError::CoordinatorDropped)?;
        let port_owner = self
            .port
            .upgrade()
            .ok_or(MainDmoHullOperationError::CoordinatorDropped)?;
        let (sim_callback, pres_callback) = {
            let mut state = state_owner
                .try_borrow_mut()
                .map_err(|_| MainDmoHullOperationError::CoordinatorBusy)?;
            check_fault(&state)?;
            if state.state != Some(MainDmoHullRawState::UnloadingCollections)
                && !(state.state == Some(MainDmoHullRawState::UnloadNotStarted)
                    && state.in_flight == Some(MainDmoHullDispatchStage::CollectionUnload))
            {
                let error = MainDmoHullOperationError::WrongState {
                    expected: MainDmoHullRawState::UnloadingCollections,
                    actual: state.state,
                };
                state.fault = Some(MainDmoHullFault::PhysicalCallbackRejected(
                    MainDmoHullPhysicalCallbackKind::Unload,
                ));
                return Err(error);
            }
            if let Err(error) = require_collection(id) {
                state.fault = Some(MainDmoHullFault::PhysicalCallbackRejected(
                    MainDmoHullPhysicalCallbackKind::Unload,
                ));
                return Err(error);
            }
            state.physical_unload_completed = state.physical_unload_completed.wrapping_add(1);
            let (sim_generation, pres_generation) = match state.reserve_callback_pair() {
                Ok(pair) => pair,
                Err(fault) => {
                    state.fault = Some(fault);
                    return Err(MainDmoHullOperationError::Faulted(fault));
                }
            };
            state.sim_dreg_generation = Some(sim_generation);
            state.pres_dreg_generation = Some(pres_generation);
            (
                MainDmoSimulationDeregistrationCallback {
                    identity: callback_identity(&state, sim_generation),
                },
                MainDmoPresentationDeregistrationCallback {
                    identity: callback_identity(&state, pres_generation),
                },
            )
        };

        dispatch_port(
            &state_owner,
            &port_owner,
            MainDmoHullDispatchStage::SimulationDeregistration,
            |port| port.deregister_simulation(deregistration_request(), sim_callback),
        )?;
        dispatch_port(
            &state_owner,
            &port_owner,
            MainDmoHullDispatchStage::PresentationDeregistration,
            |port| port.deregister_presentation(deregistration_request(), pres_callback),
        )
    }
}

fn registration_request() -> MainDmoHullRegistrationRequest {
    MainDmoHullRegistrationRequest {
        collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
        userdata_raw: MAIN_DMO_HULL_USERDATA_RAW,
        force_immediate: false,
    }
}

fn deregistration_request() -> MainDmoHullDeregistrationRequest {
    MainDmoHullDeregistrationRequest {
        collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
        userdata_raw: MAIN_DMO_HULL_USERDATA_RAW,
    }
}

fn callback_identity(state: &MainDmoHullState, generation: u64) -> CallbackIdentity {
    CallbackIdentity {
        domain: Arc::downgrade(&state.domain),
        collection_id: CollectionId(DMO_BANK_COLLECTION_ID),
        generation,
    }
}

fn validate_callback(
    state: &MainDmoHullState,
    identity: &CallbackIdentity,
    expected_generation: Option<u64>,
    expected_state: MainDmoHullRawState,
) -> Result<(), MainDmoHullCallbackRejectionKind> {
    if !SyncWeak::ptr_eq(&identity.domain, &Arc::downgrade(&state.domain)) {
        return Err(MainDmoHullCallbackRejectionKind::ForeignDomain);
    }
    if identity.collection_id != CollectionId(DMO_BANK_COLLECTION_ID) {
        return Err(MainDmoHullCallbackRejectionKind::WrongCollection);
    }
    if state.fault.is_some() {
        return Err(MainDmoHullCallbackRejectionKind::Faulted);
    }
    if state.state != Some(expected_state) {
        return Err(MainDmoHullCallbackRejectionKind::WrongState);
    }
    if expected_generation != Some(identity.generation) {
        return Err(MainDmoHullCallbackRejectionKind::WrongGeneration);
    }
    Ok(())
}

fn require_collection(id: CollectionId) -> Result<(), MainDmoHullOperationError> {
    let expected = CollectionId(DMO_BANK_COLLECTION_ID);
    if id == expected {
        Ok(())
    } else {
        Err(MainDmoHullOperationError::WrongCollection {
            expected,
            actual: id,
        })
    }
}

fn require_state(
    state: &MainDmoHullState,
    expected: MainDmoHullRawState,
) -> Result<(), MainDmoHullOperationError> {
    if state.state == Some(expected) {
        Ok(())
    } else {
        Err(MainDmoHullOperationError::WrongState {
            expected,
            actual: state.state,
        })
    }
}

fn check_fault(state: &MainDmoHullState) -> Result<(), MainDmoHullOperationError> {
    match state.fault {
        Some(fault) => Err(MainDmoHullOperationError::Faulted(fault)),
        None => Ok(()),
    }
}

fn set_raw_state(state: &mut MainDmoHullState, raw: MainDmoHullRawState) {
    state.state = Some(raw);
    state.last_raw_state = raw;
}

fn latch_shared_fault(
    state: &Rc<RefCell<MainDmoHullState>>,
    fault: MainDmoHullFault,
) -> MainDmoHullFault {
    if let Ok(mut state) = state.try_borrow_mut() {
        if state.fault.is_none() {
            state.fault = Some(fault);
        }
        return state.fault.expect("fault was installed or already present");
    }
    fault
}

fn dispatch_port<P>(
    state: &Rc<RefCell<MainDmoHullState>>,
    port: &Rc<RefCell<P>>,
    dispatch_stage: MainDmoHullDispatchStage,
    dispatch: impl FnOnce(&mut P),
) -> Result<(), MainDmoHullOperationError> {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut port = port.try_borrow_mut().map_err(|_| ())?;
        dispatch(&mut port);
        Ok::<(), ()>(())
    }));
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(())) => {
            let fault = latch_shared_fault(state, MainDmoHullFault::ReentrantPortAccess);
            Err(MainDmoHullOperationError::Faulted(fault))
        }
        Err(_) => {
            let fault = MainDmoHullFault::DispatchPanicked(dispatch_stage);
            let fault = latch_shared_fault(state, fault);
            Err(MainDmoHullOperationError::Faulted(fault))
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::collection_resident_streamer::{
        CollectionResidentStreamer, ResidentAssetGarbageCollector, ResidentCollectionLoadDriver,
        ResidentCollectionLoadFactory, ResidentCollectionMetadata, ResidentCollectionRegistry,
        ResidentCollectionStorage, ResidentDecompressionKick,
    };
    use std::cell::Cell;

    const TEST_HULL_ID: MainDmoHullId = MainDmoHullId(0x1234_abcd);

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum PortEvent {
        RegisterSimulation(MainDmoHullRegistrationRequest),
        RegisterPresentation(MainDmoHullRegistrationRequest),
        DeregisterSimulation(MainDmoHullDeregistrationRequest),
        DeregisterPresentation(MainDmoHullDeregistrationRequest),
    }

    #[derive(Default)]
    struct PortState {
        events: Vec<PortEvent>,
        sim_reg: Option<MainDmoSimulationRegistrationCallback>,
        pres_reg: Option<MainDmoPresentationRegistrationCallback>,
        sim_dreg: Option<MainDmoSimulationDeregistrationCallback>,
        pres_dreg: Option<MainDmoPresentationDeregistrationCallback>,
        panic_at: Option<MainDmoHullDispatchStage>,
    }

    pub(crate) struct TestPort {
        state: Rc<RefCell<PortState>>,
    }

    impl TestPort {
        fn maybe_panic(&self, stage: MainDmoHullDispatchStage) {
            assert_ne!(self.state.borrow().panic_at, Some(stage), "port panic");
        }
    }

    impl MainDmoHullSubsystemPort for TestPort {
        fn register_simulation(
            &mut self,
            request: MainDmoHullRegistrationRequest,
            callback: MainDmoSimulationRegistrationCallback,
        ) {
            self.state
                .borrow_mut()
                .events
                .push(PortEvent::RegisterSimulation(request));
            self.maybe_panic(MainDmoHullDispatchStage::SimulationRegistration);
            self.state.borrow_mut().sim_reg = Some(callback);
        }

        fn register_presentation(
            &mut self,
            request: MainDmoHullRegistrationRequest,
            callback: MainDmoPresentationRegistrationCallback,
        ) {
            self.state
                .borrow_mut()
                .events
                .push(PortEvent::RegisterPresentation(request));
            self.maybe_panic(MainDmoHullDispatchStage::PresentationRegistration);
            self.state.borrow_mut().pres_reg = Some(callback);
        }

        fn deregister_simulation(
            &mut self,
            request: MainDmoHullDeregistrationRequest,
            callback: MainDmoSimulationDeregistrationCallback,
        ) {
            self.state
                .borrow_mut()
                .events
                .push(PortEvent::DeregisterSimulation(request));
            self.maybe_panic(MainDmoHullDispatchStage::SimulationDeregistration);
            self.state.borrow_mut().sim_dreg = Some(callback);
        }

        fn deregister_presentation(
            &mut self,
            request: MainDmoHullDeregistrationRequest,
            callback: MainDmoPresentationDeregistrationCallback,
        ) {
            self.state
                .borrow_mut()
                .events
                .push(PortEvent::DeregisterPresentation(request));
            self.maybe_panic(MainDmoHullDispatchStage::PresentationDeregistration);
            self.state.borrow_mut().pres_dreg = Some(callback);
        }
    }

    struct TestCollection {
        id: Cell<Option<CollectionId>>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TestAssetReadFailure {
        None,
        Error,
        Panic,
    }

    struct TestStorage {
        asset_read_failure: Rc<Cell<TestAssetReadFailure>>,
    }

    impl ResidentCollectionStorage<MainDmoHullOperationError> for TestStorage {
        type Collection = TestCollection;
        type Writer = ();

        fn create_empty(
            &mut self,
            _reserve: i32,
        ) -> Result<Self::Collection, MainDmoHullOperationError> {
            Ok(TestCollection {
                id: Cell::new(None),
            })
        }

        fn writer(
            &mut self,
            _collection: &Self::Collection,
        ) -> Result<Self::Writer, MainDmoHullOperationError> {
            Ok(())
        }

        fn asset_ids_in_order(
            &mut self,
            _collection: &Self::Collection,
        ) -> Result<Vec<CollectionId>, MainDmoHullOperationError> {
            match self.asset_read_failure.get() {
                TestAssetReadFailure::None => Ok(Vec::new()),
                TestAssetReadFailure::Error => Err(MainDmoHullOperationError::Faulted(
                    MainDmoHullFault::ResidentReleaseFailed,
                )),
                TestAssetReadFailure::Panic => panic!("asset read panic"),
            }
        }

        fn destroy(&mut self, _collection: Self::Collection) {}
    }

    struct TestMetadata;

    impl ResidentCollectionMetadata<MainDmoHullOperationError> for TestMetadata {
        type Descriptor = ();

        fn find_collection(
            &mut self,
            id: CollectionId,
        ) -> Result<Option<Self::Descriptor>, MainDmoHullOperationError> {
            Ok((id == CollectionId(DMO_BANK_COLLECTION_ID)).then_some(()))
        }
    }

    struct TestDriver {
        remaining: u32,
    }

    impl ResidentCollectionLoadDriver<MainDmoHullOperationError> for TestDriver {
        fn update(&mut self) -> Result<(), MainDmoHullOperationError> {
            self.remaining = self.remaining.saturating_sub(1);
            Ok(())
        }

        fn is_complete(&self) -> bool {
            self.remaining == 0
        }

        fn highest_stored_block(&self) -> u32 {
            0
        }

        fn loaded_size(&self) -> u32 {
            0
        }
    }

    struct TestFactory;

    impl ResidentCollectionLoadFactory<(), (), MainDmoHullOperationError> for TestFactory {
        type Driver = TestDriver;

        fn begin(
            &mut self,
            _id: CollectionId,
            _asset_count: i32,
            (): (),
            (): (),
        ) -> Result<Self::Driver, MainDmoHullOperationError> {
            Ok(TestDriver { remaining: 1 })
        }
    }

    struct TestWrapper;

    struct TestRegistry;

    impl ResidentCollectionRegistry<TestCollection, MainDmoHullOperationError> for TestRegistry {
        type Wrapper = TestWrapper;
        type Token = CollectionId;

        fn create_wrapper(
            &mut self,
            id: CollectionId,
            collection: &TestCollection,
        ) -> Result<Self::Wrapper, MainDmoHullOperationError> {
            collection.id.set(Some(id));
            Ok(TestWrapper)
        }

        fn stable_token(_wrapper: &Self::Wrapper) -> Self::Token {
            CollectionId(DMO_BANK_COLLECTION_ID)
        }

        fn publish(
            &mut self,
            _id: CollectionId,
            _token: &Self::Token,
        ) -> Result<(), MainDmoHullOperationError> {
            Ok(())
        }

        fn remove_exact(
            &mut self,
            _id: CollectionId,
            _token: &Self::Token,
        ) -> Result<(), MainDmoHullOperationError> {
            Ok(())
        }
    }

    struct TestGarbage;

    impl ResidentAssetGarbageCollector<MainDmoHullOperationError> for TestGarbage {
        fn garbage_collect(&mut self, _id: CollectionId) -> Result<(), MainDmoHullOperationError> {
            Ok(())
        }
    }

    struct TestKick;

    impl ResidentDecompressionKick<MainDmoHullOperationError> for TestKick {
        fn start_decompress(&mut self) -> Result<(), MainDmoHullOperationError> {
            Ok(())
        }
    }

    type TestStreamer = CollectionResidentStreamer<
        TestStorage,
        TestMetadata,
        TestFactory,
        TestRegistry,
        TestGarbage,
        TestKick,
        MainDmoHullResidentCallback<TestPort>,
        MainDmoHullOperationError,
    >;

    enum OuterCallback {
        Dmo(MainDmoHullResidentCallback<TestPort>),
    }

    impl ResidentCollectionCallback<MainDmoHullOperationError> for OuterCallback {
        fn on_complete(
            &mut self,
            id: CollectionId,
            observation: ResidentCallbackObservation,
            context: &mut ResidentCallbackContext<'_, Self>,
        ) -> Result<(), MainDmoHullOperationError> {
            match self {
                Self::Dmo(callback) => {
                    callback.dispatch_mapped(id, observation, context, std::convert::identity)
                }
            }
        }
    }

    type OuterTestStreamer = CollectionResidentStreamer<
        TestStorage,
        TestMetadata,
        TestFactory,
        TestRegistry,
        TestGarbage,
        TestKick,
        OuterCallback,
        MainDmoHullOperationError,
    >;

    type TestSetup = (
        MainDmoHullAcknowledgementCoordinator<TestPort>,
        MainDmoHullResidentCallback<TestPort>,
        TestStreamer,
        Rc<RefCell<PortState>>,
        Rc<Cell<TestAssetReadFailure>>,
    );

    /// Opaque cross-module harness that reaches real resident holds through
    /// the production physical load/unload callback path.
    pub(crate) struct MainDmoDregIntegrationHarness {
        coordinator: MainDmoHullAcknowledgementCoordinator<TestPort>,
        load_callback: Option<MainDmoHullResidentCallback<TestPort>>,
        streamer: TestStreamer,
        port: Rc<RefCell<PortState>>,
        asset_read_failure: Rc<Cell<TestAssetReadFailure>>,
    }

    impl MainDmoDregIntegrationHarness {
        pub(crate) fn new() -> Self {
            let (coordinator, callback, streamer, port, asset_read_failure) = setup();
            Self {
                coordinator,
                load_callback: Some(callback),
                streamer,
                port,
                asset_read_failure,
            }
        }

        pub(crate) fn coordinator(&self) -> &MainDmoHullAcknowledgementCoordinator<TestPort> {
            &self.coordinator
        }

        pub(crate) fn take_simulation_registration_callback(
            &mut self,
        ) -> MainDmoSimulationRegistrationCallback {
            physically_load(
                &mut self.streamer,
                self.load_callback
                    .take()
                    .expect("physical load callback is single-use"),
            );
            let presentation = self
                .port
                .borrow_mut()
                .pres_reg
                .take()
                .expect("physical load emitted Presentation registration");
            self.coordinator
                .acknowledge_presentation_registration(presentation)
                .unwrap();
            self.port
                .borrow_mut()
                .sim_reg
                .take()
                .expect("physical load emitted Simulation registration")
        }

        pub(crate) fn advance_and_take_simulation_deregistration_callback(
            &mut self,
        ) -> MainDmoSimulationDeregistrationCallback {
            advance_to_loaded(&self.coordinator);
            self.coordinator.request_unload().unwrap();
            assert_eq!(
                self.coordinator.update().unwrap().state,
                Some(MainDmoHullRawState::UnloadNotStarted)
            );
            self.coordinator
                .start_collection_unload(|id, callback| {
                    self.streamer.enqueue_unload(id, Some(callback));
                })
                .unwrap();
            assert_eq!(self.streamer.update().unwrap().unloads_processed, 1);
            let presentation = self
                .port
                .borrow_mut()
                .pres_dreg
                .take()
                .expect("physical unload emitted Presentation DReg");
            assert!(
                !self
                    .coordinator
                    .acknowledge_presentation_deregistration(presentation, &mut self.streamer)
                    .unwrap()
            );
            self.port
                .borrow_mut()
                .sim_dreg
                .take()
                .expect("physical unload emitted Simulation DReg")
        }

        pub(crate) fn acknowledgement_parts(
            &mut self,
        ) -> (
            &MainDmoHullAcknowledgementCoordinator<TestPort>,
            &mut impl MainDmoResidentHoldReleaser<Error = MainDmoHullOperationError>,
        ) {
            (&self.coordinator, &mut self.streamer)
        }

        pub(crate) fn resident_count(&self) -> usize {
            self.streamer.resident_count()
        }

        pub(crate) fn unregistration_completed(&self) -> u32 {
            self.coordinator
                .observation()
                .unwrap()
                .unregistration_completed
        }

        pub(crate) fn fail_final_release(&self) {
            self.asset_read_failure.set(TestAssetReadFailure::Error);
        }

        pub(crate) fn panic_final_release(&self) {
            self.asset_read_failure.set(TestAssetReadFailure::Panic);
        }
    }

    #[test]
    fn main_bank_hull_id_matches_the_recovered_path_hash() {
        assert_eq!(MAIN_DMO_HULL_ID_INPUT, b"data\\content\\world\\dmo\\DMO");
        assert_eq!(MainDmoHullId::MAIN_BANK, MainDmoHullId(0x048f_3c1f));
        assert_eq!(
            MainDmoHullId::MAIN_BANK.0,
            skate2_formats::hash::sk8_hash_value_32(MAIN_DMO_HULL_ID_INPUT)
        );
    }

    fn setup() -> TestSetup {
        let port_state = Rc::new(RefCell::new(PortState::default()));
        let asset_read_failure = Rc::new(Cell::new(TestAssetReadFailure::None));
        let (coordinator, callback) = MainDmoHullAcknowledgementCoordinator::new(
            TestPort {
                state: Rc::clone(&port_state),
            },
            TEST_HULL_ID,
        );
        let streamer = CollectionResidentStreamer::new(
            TestStorage {
                asset_read_failure: Rc::clone(&asset_read_failure),
            },
            TestMetadata,
            TestFactory,
            TestRegistry,
            TestGarbage,
            TestKick,
        );
        (
            coordinator,
            callback,
            streamer,
            port_state,
            asset_read_failure,
        )
    }

    fn physically_load(
        streamer: &mut TestStreamer,
        callback: MainDmoHullResidentCallback<TestPort>,
    ) {
        streamer.enqueue_load(CollectionId(DMO_BANK_COLLECTION_ID), 0, Some(callback));
        assert_eq!(streamer.update().unwrap().physical_loads_started, 1);
        assert_eq!(streamer.update().unwrap().physical_loads_completed, 1);
    }

    fn acknowledge_registrations(
        coordinator: &MainDmoHullAcknowledgementCoordinator<TestPort>,
        port: &Rc<RefCell<PortState>>,
    ) {
        let sim = port.borrow_mut().sim_reg.take().unwrap();
        let pres = port.borrow_mut().pres_reg.take().unwrap();
        coordinator
            .acknowledge_presentation_registration(pres)
            .unwrap();
        coordinator
            .acknowledge_simulation_registration(sim)
            .unwrap();
    }

    fn advance_to_loaded(coordinator: &MainDmoHullAcknowledgementCoordinator<TestPort>) {
        assert_eq!(
            coordinator.update().unwrap().state,
            Some(MainDmoHullRawState::LoadingTextureCollections)
        );
        assert_eq!(
            coordinator.update().unwrap().state,
            Some(MainDmoHullRawState::LoadCompleteNotify)
        );
        assert!(coordinator.is_loaded().unwrap());
        assert_eq!(
            coordinator.update().unwrap().state,
            Some(MainDmoHullRawState::LoadComplete)
        );
    }

    fn verify_load_and_advance(
        coordinator: &MainDmoHullAcknowledgementCoordinator<TestPort>,
        callback: MainDmoHullResidentCallback<TestPort>,
        streamer: &mut TestStreamer,
        port: &Rc<RefCell<PortState>>,
    ) {
        physically_load(streamer, callback);
        assert_eq!(
            port.borrow().events,
            vec![
                PortEvent::RegisterSimulation(registration_request()),
                PortEvent::RegisterPresentation(registration_request()),
            ]
        );
        assert_eq!(
            streamer
                .observe_resident(CollectionId(DMO_BANK_COLLECTION_ID))
                .unwrap()
                .ref_count,
            3
        );
        assert_eq!(
            coordinator.observation().unwrap().physical_load_remaining,
            0
        );
        assert!(!coordinator.is_loaded().unwrap());
        acknowledge_registrations(coordinator, port);
        assert_eq!(coordinator.observation().unwrap().registration_remaining, 0);
        advance_to_loaded(coordinator);
    }

    fn accepts_simulation_registration(_: MainDmoSimulationRegistrationCallback) {}

    fn verify_stream_unload_and_finish(
        coordinator: &MainDmoHullAcknowledgementCoordinator<TestPort>,
    ) {
        let mut stream_requests = Vec::new();
        coordinator
            .request_stream_unload(|stream| stream_requests.push(stream))
            .unwrap();
        assert_eq!(stream_requests, vec![MainDmoHullStream::Presentation]);
        let mut polls = Vec::new();
        let mut checked_recursion = false;
        assert!(
            !coordinator
                .poll_stream_unloaded(|stream| {
                    polls.push(stream);
                    if !checked_recursion {
                        assert_eq!(
                            coordinator.poll_stream_unloaded(|_| panic!("nested poll ran")),
                            Err(MainDmoHullOperationError::OperationInFlight(
                                MainDmoHullDispatchStage::SimulationStreamUnloadPoll
                            ))
                        );
                        checked_recursion = true;
                    }
                    stream != MainDmoHullStream::Presentation
                })
                .unwrap()
        );
        assert!(checked_recursion);
        assert_eq!(
            polls,
            vec![
                MainDmoHullStream::Simulation,
                MainDmoHullStream::Presentation,
                MainDmoHullStream::Texture,
            ]
        );
        assert!(coordinator.poll_stream_unloaded(|_| true).unwrap());
        let mut notified = None;
        coordinator
            .finish_unload_notification(
                MainDmoHullUnloadOutcome::Complete,
                Some(|hull_id, outcome| {
                    assert_eq!(hull_id, TEST_HULL_ID);
                    assert_eq!(
                        coordinator.raw_state().unwrap(),
                        Some(MainDmoHullRawState::UnloadCompleteNotify)
                    );
                    notified = Some(outcome);
                }),
            )
            .unwrap();
        assert_eq!(notified, Some(MainDmoHullUnloadOutcome::Complete));
        let final_observation = coordinator.observation().unwrap();
        assert_eq!(final_observation.state, None);
        assert_eq!(
            final_observation.last_raw_state,
            MainDmoHullRawState::UnloadComplete
        );
        assert!(coordinator.is_unloaded().unwrap());
    }

    #[test]
    fn full_main_bank_path_preserves_native_order_counts_and_state_cadence() {
        let (coordinator, callback, mut streamer, port, _) = setup();
        verify_load_and_advance(&coordinator, callback, &mut streamer, &port);

        coordinator.request_unload().unwrap();
        assert_eq!(
            coordinator.raw_state().unwrap(),
            Some(MainDmoHullRawState::LoadComplete)
        );
        assert_eq!(
            coordinator.update().unwrap().state,
            Some(MainDmoHullRawState::UnloadNotStarted)
        );
        coordinator
            .start_collection_unload(|id, callback| streamer.enqueue_unload(id, Some(callback)))
            .unwrap();
        assert_eq!(
            coordinator.raw_state().unwrap(),
            Some(MainDmoHullRawState::UnloadingCollections)
        );
        assert_eq!(streamer.update().unwrap().unloads_processed, 1);
        assert_eq!(
            streamer
                .observe_resident(CollectionId(DMO_BANK_COLLECTION_ID))
                .unwrap()
                .ref_count,
            2
        );
        assert_eq!(
            &port.borrow().events[2..],
            &[
                PortEvent::DeregisterSimulation(deregistration_request()),
                PortEvent::DeregisterPresentation(deregistration_request()),
            ]
        );
        assert_eq!(
            coordinator
                .request_stream_unload(|_| panic!("zero acknowledgements must not dispatch"))
                .unwrap_err(),
            MainDmoHullOperationError::AcknowledgementsPending {
                completed: 0,
                required: 2,
            }
        );

        let pres = port.borrow_mut().pres_dreg.take().unwrap();
        let sim = port.borrow_mut().sim_dreg.take().unwrap();
        assert!(
            !coordinator
                .acknowledge_presentation_deregistration(pres, &mut streamer)
                .unwrap()
        );
        assert_eq!(
            coordinator.observation().unwrap().unregistration_completed,
            1
        );
        assert_eq!(
            coordinator
                .request_stream_unload(|_| panic!("one acknowledgement must not dispatch"))
                .unwrap_err(),
            MainDmoHullOperationError::AcknowledgementsPending {
                completed: 1,
                required: 2,
            }
        );
        assert!(
            coordinator
                .acknowledge_simulation_deregistration(sim, &mut streamer)
                .unwrap()
        );
        assert_eq!(streamer.resident_count(), 0);
        assert_eq!(
            coordinator.observation().unwrap().unregistration_completed,
            2
        );

        verify_stream_unload_and_finish(&coordinator);
    }

    #[test]
    fn registration_token_drop_stalls_state_eight_without_fabricating_completion() {
        let (coordinator, callback, mut streamer, port, _) = setup();
        physically_load(&mut streamer, callback);
        drop(port.borrow_mut().sim_reg.take().unwrap());
        let pres = port.borrow_mut().pres_reg.take().unwrap();
        coordinator
            .acknowledge_presentation_registration(pres)
            .unwrap();

        assert_eq!(coordinator.observation().unwrap().registration_remaining, 1);
        assert_eq!(
            coordinator.update().unwrap().state,
            Some(MainDmoHullRawState::LoadingPresentationCollections)
        );
    }

    #[test]
    fn foreign_registration_token_is_returned_and_has_zero_effect() {
        let (first, first_callback, mut first_streamer, first_port, _) = setup();
        let (second, second_callback, mut second_streamer, second_port, _) = setup();
        physically_load(&mut first_streamer, first_callback);
        physically_load(&mut second_streamer, second_callback);
        let foreign = first_port.borrow_mut().sim_reg.take().unwrap();

        let rejection = second
            .acknowledge_simulation_registration(foreign)
            .unwrap_err();
        assert_eq!(
            rejection.kind(),
            MainDmoHullCallbackRejectionKind::ForeignDomain
        );
        assert_eq!(second.observation().unwrap().registration_remaining, 2);
        first
            .acknowledge_simulation_registration(rejection.into_callback())
            .unwrap();
        assert_eq!(first.observation().unwrap().registration_remaining, 1);
        assert!(second_port.borrow().sim_reg.is_some());
    }

    #[test]
    fn synchronous_state_twelve_deregistration_is_rejected_then_retryable() {
        let (coordinator, callback, mut streamer, port, _) = setup();
        physically_load(&mut streamer, callback);
        acknowledge_registrations(&coordinator, &port);
        advance_to_loaded(&coordinator);
        coordinator.request_unload().unwrap();
        coordinator.update().unwrap();

        let rejected = Rc::new(RefCell::new(None));
        coordinator
            .start_collection_unload(|id, callback| {
                streamer.enqueue_unload(id, Some(callback));
                streamer.update().unwrap();
                let sim = port.borrow_mut().sim_dreg.take().unwrap();
                rejected.borrow_mut().replace(
                    coordinator
                        .acknowledge_simulation_deregistration(sim, &mut streamer)
                        .unwrap_err()
                        .into_callback()
                        .unwrap(),
                );
            })
            .unwrap();
        assert_eq!(
            coordinator.observation().unwrap().unregistration_completed,
            0
        );
        let sim = rejected.borrow_mut().take().unwrap();
        assert!(
            !coordinator
                .acknowledge_simulation_deregistration(sim, &mut streamer)
                .unwrap()
        );
    }

    #[test]
    fn teardown_fault_consumes_valid_token_without_incrementing_ack_latch() {
        let (coordinator, callback, mut streamer, port, asset_read_failure) = setup();
        physically_load(&mut streamer, callback);
        acknowledge_registrations(&coordinator, &port);
        advance_to_loaded(&coordinator);
        coordinator.request_unload().unwrap();
        coordinator.update().unwrap();
        coordinator
            .start_collection_unload(|id, callback| streamer.enqueue_unload(id, Some(callback)))
            .unwrap();
        streamer.update().unwrap();

        let sim = port.borrow_mut().sim_dreg.take().unwrap();
        let pres = port.borrow_mut().pres_dreg.take().unwrap();
        assert!(
            !coordinator
                .acknowledge_simulation_deregistration(sim, &mut streamer)
                .unwrap()
        );
        asset_read_failure.set(TestAssetReadFailure::Error);
        let error = coordinator
            .acknowledge_presentation_deregistration(pres, &mut streamer)
            .unwrap_err();
        assert!(matches!(
            error,
            MainDmoHullDeregistrationAckError::ResidentRelease(_)
        ));
        let observation = coordinator.observation().unwrap();
        assert_eq!(observation.unregistration_completed, 1);
        assert_eq!(
            observation.fault,
            Some(MainDmoHullFault::ResidentReleaseFailed)
        );
        assert_eq!(streamer.resident_count(), 0);
    }

    #[test]
    fn dispatch_panic_latches_exact_prefix_and_does_not_run_next_dispatch() {
        let (coordinator, callback, mut streamer, port, _) = setup();
        port.borrow_mut().panic_at = Some(MainDmoHullDispatchStage::SimulationRegistration);
        streamer.enqueue_load(CollectionId(DMO_BANK_COLLECTION_ID), 0, Some(callback));
        streamer.update().unwrap();
        assert!(streamer.update().is_err());

        let observation = coordinator.observation().unwrap();
        assert_eq!(
            observation.fault,
            Some(MainDmoHullFault::DispatchPanicked(
                MainDmoHullDispatchStage::SimulationRegistration
            ))
        );
        assert_eq!(observation.physical_load_remaining, 0);
        assert_eq!(
            streamer
                .observe_resident(CollectionId(DMO_BANK_COLLECTION_ID))
                .unwrap()
                .ref_count,
            2
        );
        assert_eq!(
            port.borrow().events,
            vec![PortEvent::RegisterSimulation(registration_request())]
        );
    }

    #[test]
    fn callback_capabilities_are_send_and_have_distinct_static_types() {
        fn require_send<T: Send>() {}
        require_send::<MainDmoSimulationRegistrationCallback>();
        require_send::<MainDmoPresentationRegistrationCallback>();
        require_send::<MainDmoSimulationDeregistrationCallback>();
        require_send::<MainDmoPresentationDeregistrationCallback>();

        let (coordinator, callback, mut streamer, port, _) = setup();
        physically_load(&mut streamer, callback);
        accepts_simulation_registration(port.borrow_mut().sim_reg.take().unwrap());
        assert_eq!(coordinator.observation().unwrap().registration_remaining, 2);
    }

    #[test]
    fn callback_generation_exhaustion_faults_before_first_resident_up() {
        let (coordinator, callback, mut streamer, _, _) = setup();
        coordinator.state.borrow_mut().next_callback_generation = u64::MAX - 1;
        streamer.enqueue_load(CollectionId(DMO_BANK_COLLECTION_ID), 0, Some(callback));
        streamer.update().unwrap();
        assert!(streamer.update().is_err());

        assert_eq!(
            coordinator.observation().unwrap().fault,
            Some(MainDmoHullFault::CallbackGenerationExhausted)
        );
        assert_eq!(
            streamer
                .observe_resident(CollectionId(DMO_BANK_COLLECTION_ID))
                .unwrap()
                .ref_count,
            1
        );
    }

    #[test]
    fn cache_hit_is_rejected_as_an_explicit_fresh_main_bank_precondition() {
        let (coordinator, callback, mut streamer, port, _) = setup();
        streamer.enqueue_load(CollectionId(DMO_BANK_COLLECTION_ID), 0, None);
        streamer.update().unwrap();
        streamer.update().unwrap();
        streamer.enqueue_load(CollectionId(DMO_BANK_COLLECTION_ID), 0, Some(callback));

        assert!(streamer.update().is_err());
        assert_eq!(
            coordinator.observation().unwrap().fault,
            Some(MainDmoHullFault::PhysicalCallbackRejected(
                MainDmoHullPhysicalCallbackKind::Load
            ))
        );
        assert_eq!(
            streamer
                .observe_resident(CollectionId(DMO_BANK_COLLECTION_ID))
                .unwrap()
                .ref_count,
            2
        );
        assert!(port.borrow().events.is_empty());
    }

    #[test]
    fn mapped_dispatch_acquires_holds_from_an_outer_callback_context() {
        let port = Rc::new(RefCell::new(PortState::default()));
        let (coordinator, callback) = MainDmoHullAcknowledgementCoordinator::new(
            TestPort {
                state: Rc::clone(&port),
            },
            TEST_HULL_ID,
        );
        let mut streamer: OuterTestStreamer = CollectionResidentStreamer::new(
            TestStorage {
                asset_read_failure: Rc::new(Cell::new(TestAssetReadFailure::None)),
            },
            TestMetadata,
            TestFactory,
            TestRegistry,
            TestGarbage,
            TestKick,
        );
        streamer.enqueue_load(
            CollectionId(DMO_BANK_COLLECTION_ID),
            0,
            Some(OuterCallback::Dmo(callback)),
        );
        streamer.update().unwrap();
        streamer.update().unwrap();

        assert_eq!(
            streamer
                .observe_resident(CollectionId(DMO_BANK_COLLECTION_ID))
                .unwrap()
                .ref_count,
            3
        );
        assert_eq!(port.borrow().events.len(), 2);
        assert_eq!(coordinator.observation().unwrap().registration_remaining, 2);

        acknowledge_registrations(&coordinator, &port);
        advance_to_loaded(&coordinator);
        coordinator.request_unload().unwrap();
        coordinator.update().unwrap();
        coordinator
            .start_collection_unload(|id, callback| {
                streamer.enqueue_unload(id, Some(OuterCallback::Dmo(callback)));
            })
            .unwrap();
        streamer.update().unwrap();
        assert_eq!(
            streamer
                .observe_resident(CollectionId(DMO_BANK_COLLECTION_ID))
                .unwrap()
                .ref_count,
            2
        );
        let sim = port.borrow_mut().sim_dreg.take().unwrap();
        let pres = port.borrow_mut().pres_dreg.take().unwrap();
        assert!(
            !coordinator
                .acknowledge_simulation_deregistration(sim, &mut streamer)
                .unwrap()
        );
        assert!(
            coordinator
                .acknowledge_presentation_deregistration(pres, &mut streamer)
                .unwrap()
        );
        assert_eq!(streamer.resident_count(), 0);
    }

    #[test]
    fn state_fifteen_forwards_hull_id_to_failure_and_supports_no_callback() {
        let (failed, _, _, _, _) = setup();
        set_raw_state(
            &mut failed.state.borrow_mut(),
            MainDmoHullRawState::UnloadCompleteNotify,
        );
        let mut notification = None;
        failed
            .finish_unload_notification(
                MainDmoHullUnloadOutcome::Failed,
                Some(|hull_id, outcome| notification = Some((hull_id, outcome))),
            )
            .unwrap();
        assert_eq!(
            notification,
            Some((TEST_HULL_ID, MainDmoHullUnloadOutcome::Failed))
        );

        let (without_callback, _, _, _, _) = setup();
        set_raw_state(
            &mut without_callback.state.borrow_mut(),
            MainDmoHullRawState::UnloadCompleteNotify,
        );
        without_callback
            .finish_unload_notification(
                MainDmoHullUnloadOutcome::Complete,
                None::<fn(MainDmoHullId, MainDmoHullUnloadOutcome)>,
            )
            .unwrap();
        assert!(without_callback.is_unloaded().unwrap());
    }

    #[test]
    fn recursive_collection_and_stream_dispatches_are_suppressed() {
        let (coordinator, callback, mut streamer, port, _) = setup();
        verify_load_and_advance(&coordinator, callback, &mut streamer, &port);
        coordinator.request_unload().unwrap();
        coordinator.update().unwrap();
        coordinator
            .start_collection_unload(|id, callback| {
                assert_eq!(
                    coordinator
                        .start_collection_unload(|_, _| panic!("nested dispatch ran"))
                        .unwrap_err(),
                    MainDmoHullOperationError::OperationInFlight(
                        MainDmoHullDispatchStage::CollectionUnload
                    )
                );
                streamer.enqueue_unload(id, Some(callback));
            })
            .unwrap();
        streamer.update().unwrap();
        let sim = port.borrow_mut().sim_dreg.take().unwrap();
        let pres = port.borrow_mut().pres_dreg.take().unwrap();
        coordinator
            .acknowledge_simulation_deregistration(sim, &mut streamer)
            .unwrap();
        coordinator
            .acknowledge_presentation_deregistration(pres, &mut streamer)
            .unwrap();
        coordinator
            .request_stream_unload(|stream| {
                assert_eq!(stream, MainDmoHullStream::Presentation);
                assert_eq!(
                    coordinator.request_stream_unload(|_| panic!("nested request ran")),
                    Err(MainDmoHullOperationError::OperationInFlight(
                        MainDmoHullDispatchStage::StreamUnload
                    ))
                );
            })
            .unwrap();
    }

    #[test]
    fn presentation_poll_panic_records_exact_completed_prefix() {
        let (coordinator, callback, mut streamer, port, _) = setup();
        verify_load_and_advance(&coordinator, callback, &mut streamer, &port);
        coordinator.request_unload().unwrap();
        coordinator.update().unwrap();
        coordinator
            .start_collection_unload(|id, callback| streamer.enqueue_unload(id, Some(callback)))
            .unwrap();
        streamer.update().unwrap();
        let sim = port.borrow_mut().sim_dreg.take().unwrap();
        let pres = port.borrow_mut().pres_dreg.take().unwrap();
        coordinator
            .acknowledge_simulation_deregistration(sim, &mut streamer)
            .unwrap();
        coordinator
            .acknowledge_presentation_deregistration(pres, &mut streamer)
            .unwrap();
        coordinator.request_stream_unload(|_| {}).unwrap();

        let mut polls = Vec::new();
        assert!(
            coordinator
                .poll_stream_unloaded(|stream| {
                    polls.push(stream);
                    assert_ne!(stream, MainDmoHullStream::Presentation, "poll panic");
                    true
                })
                .is_err()
        );
        assert_eq!(
            polls,
            vec![
                MainDmoHullStream::Simulation,
                MainDmoHullStream::Presentation,
            ]
        );
        assert_eq!(
            coordinator.observation().unwrap().fault,
            Some(MainDmoHullFault::DispatchPanicked(
                MainDmoHullDispatchStage::PresentationStreamUnloadPoll
            ))
        );
    }
}
