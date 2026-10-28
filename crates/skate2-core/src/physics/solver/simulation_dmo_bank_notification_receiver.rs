//! Private caller-asserted notification boundary for the exact main DMO bank.
//!
//! Admission, message allocation, FIFO ownership, dispatch cadence, callback
//! invocation, and hull acknowledgement remain outside this module. The two
//! constructors and `begin_deregistration` only stage caller-selected routes
//! and retained callback capabilities. Each effectful `receive_*` method
//! accepts the caller's assertion that the corresponding FIFO front has been
//! reached. This module neither owns nor dequeues message nodes and neither
//! invokes nor acknowledges callbacks.
//! Typed stages, wrong-stage refusals, sticky fault retention, and retryable
//! owning errors are safe-host policy; the native `void` interfaces do not
//! establish a recoverable failure or retry contract.

use super::{
    MainDmoBankPublishedAcquisitionError, MainDmoBankPublishedArenaRevalidationError,
    coordinator::{
        MainDmoBankCollectionCoordinator, MainDmoBankCollectionDeregistrationError,
        MainDmoBankCollectionRegistration, MainDmoBankCollectionRegistrationError,
        MainDmoBankDeferredFinishError, MainDmoBankDeferredRegistration, MainDmoBankDeferredStep,
        MainDmoBankDeferredStepError, MainDmoBankRevalidatedDeferredStepError,
    },
};
use crate::{
    simulation_collection_adapter::{
        SimulationCollectionDmoPass, SimulationCollectionLocationPass,
    },
    simulation_normal_arena_adapter::{
        SimulationNormalArenaDispatch, SimulationNormalEnvironmentDispatch,
    },
};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankNotificationStage {
    ImmediateCollection,
    DeferredArena,
    DeferredFinish,
    RegistrationCallback,
    Registered,
    DeregistrationCollection,
    DeregistrationCallback,
    Complete,
    Faulted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MainDmoBankNotificationWrongStage {
    expected: MainDmoBankNotificationStage,
    actual: MainDmoBankNotificationStage,
}

impl MainDmoBankNotificationWrongStage {
    pub(super) const fn expected(self) -> MainDmoBankNotificationStage {
        self.expected
    }

    pub(super) const fn actual(self) -> MainDmoBankNotificationStage {
        self.actual
    }
}

pub(super) struct MainDmoBankDeferredArenaReceiverFailure {
    source: MainDmoBankDeferredStepError,
}

impl fmt::Debug for MainDmoBankDeferredArenaReceiverFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("MainDmoBankDeferredArenaReceiverFailure")
            .field(&self.source)
            .finish()
    }
}

pub(super) struct MainDmoBankDeferredFinishReceiverFailure {
    source: MainDmoBankDeferredFinishError,
}

impl fmt::Debug for MainDmoBankDeferredFinishReceiverFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("MainDmoBankDeferredFinishReceiverFailure")
            .field(&self.source)
            .finish()
    }
}

pub(super) struct MainDmoBankDeregistrationReceiverFailure {
    source: MainDmoBankCollectionDeregistrationError,
}

/// Effect-free deferred-begin refusal retaining both the exact coordinator and
/// its still-withheld callback. The only recovery operation retries the same
/// caller-selected route; no binding, controller, callback, or inner token can
/// be extracted.
pub(super) struct MainDmoBankDeferredBeginRejection<ND, NE, L, D, RC>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    source: MainDmoBankCollectionRegistrationError,
    registration_callback: RC,
    coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
}

impl<ND, NE, L, D, RC> MainDmoBankDeferredBeginRejection<ND, NE, L, D, RC>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    pub(super) const fn source(&self) -> &MainDmoBankCollectionRegistrationError {
        &self.source
    }

    /// Revalidates the retained published binding without exposing the
    /// coordinator or callback which must survive a deferred-begin retry.
    pub(super) fn try_revalidate_published_binding(
        &self,
    ) -> Result<(), MainDmoBankPublishedAcquisitionError> {
        self.coordinator.try_revalidate_published_binding()
    }

    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "the owning refusal deliberately retains the unboxed coordinator and callback without adding an allocation-failure boundary"
    )]
    pub(super) fn retry_deferred<DC>(
        self,
    ) -> Result<
        MainDmoBankNotificationReceiver<ND, NE, L, D, RC, DC>,
        MainDmoBankDeferredBeginRejection<ND, NE, L, D, RC>,
    > {
        let Self {
            mut coordinator,
            registration_callback,
            source: _,
        } = self;
        match coordinator.begin_deferred() {
            Ok(registration) => Ok(MainDmoBankNotificationReceiver {
                state: ReceiverState::DeferredArena(registration),
                registration_callback: Some(registration_callback),
                deregistration_callback: None,
                coordinator,
            }),
            Err(source) => Err(Self {
                source,
                registration_callback,
                coordinator,
            }),
        }
    }

    /// Creates a one-shot effect-free refusal which still owns the exact
    /// coordinator and callback. This only exercises the production retry
    /// transition; native code exposes no fallible deferred-begin API.
    #[cfg(test)]
    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "the test-only refusal preserves the production unboxed ownership shape"
    )]
    pub(super) fn fail_for_test<DC>(
        coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
        registration_callback: RC,
        source: MainDmoBankCollectionRegistrationError,
    ) -> Result<
        MainDmoBankNotificationReceiver<ND, NE, L, D, RC, DC>,
        MainDmoBankDeferredBeginRejection<ND, NE, L, D, RC>,
    > {
        Err(Self {
            source,
            registration_callback,
            coordinator,
        })
    }
}

impl<ND, NE, L, D, RC> fmt::Debug for MainDmoBankDeferredBeginRejection<ND, NE, L, D, RC>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankDeferredBeginRejection")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for MainDmoBankDeregistrationReceiverFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("MainDmoBankDeregistrationReceiverFailure")
            .field(&self.source)
            .finish()
    }
}

pub(super) enum MainDmoBankImmediateReceiverError {
    WrongStage(MainDmoBankNotificationWrongStage),
    Receiver(MainDmoBankCollectionRegistrationError),
}

impl fmt::Debug for MainDmoBankImmediateReceiverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongStage(stage) => formatter.debug_tuple("WrongStage").field(stage).finish(),
            Self::Receiver(source) => formatter.debug_tuple("Receiver").field(source).finish(),
        }
    }
}

pub(super) enum MainDmoBankDeferredArenaReceiverError {
    PublishedArenaRevalidation(MainDmoBankPublishedArenaRevalidationError),
    WrongStage(MainDmoBankNotificationWrongStage),
    Receiver(MainDmoBankDeferredArenaReceiverFailure),
}

impl fmt::Debug for MainDmoBankDeferredArenaReceiverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PublishedArenaRevalidation(source) => formatter
                .debug_tuple("PublishedArenaRevalidation")
                .field(source)
                .finish(),
            Self::WrongStage(stage) => formatter.debug_tuple("WrongStage").field(stage).finish(),
            Self::Receiver(source) => formatter.debug_tuple("Receiver").field(source).finish(),
        }
    }
}

pub(super) enum MainDmoBankDeferredFinishReceiverError {
    PublishedBindingRevalidation(MainDmoBankPublishedAcquisitionError),
    WrongStage(MainDmoBankNotificationWrongStage),
    Receiver(MainDmoBankDeferredFinishReceiverFailure),
}

impl fmt::Debug for MainDmoBankDeferredFinishReceiverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PublishedBindingRevalidation(source) => formatter
                .debug_tuple("PublishedBindingRevalidation")
                .field(source)
                .finish(),
            Self::WrongStage(stage) => formatter.debug_tuple("WrongStage").field(stage).finish(),
            Self::Receiver(source) => formatter.debug_tuple("Receiver").field(source).finish(),
        }
    }
}

pub(super) enum MainDmoBankDeregistrationReceiverError {
    PublishedBindingRevalidation(MainDmoBankPublishedAcquisitionError),
    WrongStage(MainDmoBankNotificationWrongStage),
    Receiver(MainDmoBankDeregistrationReceiverFailure),
}

impl fmt::Debug for MainDmoBankDeregistrationReceiverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PublishedBindingRevalidation(source) => formatter
                .debug_tuple("PublishedBindingRevalidation")
                .field(source)
                .finish(),
            Self::WrongStage(stage) => formatter.debug_tuple("WrongStage").field(stage).finish(),
            Self::Receiver(source) => formatter.debug_tuple("Receiver").field(source).finish(),
        }
    }
}

pub(super) struct MainDmoBankCallbackReceiverRejection<C> {
    stage: MainDmoBankNotificationWrongStage,
    callback: C,
}

impl<C> MainDmoBankCallbackReceiverRejection<C> {
    pub(super) const fn stage(&self) -> MainDmoBankNotificationWrongStage {
        self.stage
    }

    pub(super) fn into_callback(self) -> C {
        self.callback
    }
}

impl<C> fmt::Debug for MainDmoBankCallbackReceiverRejection<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankCallbackReceiverRejection")
            .field("stage", &self.stage)
            .finish_non_exhaustive()
    }
}

enum ReceiverState {
    ImmediateCollection,
    DeferredArena(MainDmoBankDeferredRegistration),
    DeferredFinish(MainDmoBankDeferredRegistration),
    RegistrationCallback(MainDmoBankCollectionRegistration),
    Registered(MainDmoBankCollectionRegistration),
    DeregistrationCollection(MainDmoBankCollectionRegistration),
    DeregistrationCallback,
    Complete,
    Faulted,
}

impl ReceiverState {
    const fn stage(&self) -> MainDmoBankNotificationStage {
        match self {
            Self::ImmediateCollection => MainDmoBankNotificationStage::ImmediateCollection,
            Self::DeferredArena(_) => MainDmoBankNotificationStage::DeferredArena,
            Self::DeferredFinish(_) => MainDmoBankNotificationStage::DeferredFinish,
            Self::RegistrationCallback(_) => MainDmoBankNotificationStage::RegistrationCallback,
            Self::Registered(_) => MainDmoBankNotificationStage::Registered,
            Self::DeregistrationCollection(_) => {
                MainDmoBankNotificationStage::DeregistrationCollection
            }
            Self::DeregistrationCallback => MainDmoBankNotificationStage::DeregistrationCallback,
            Self::Complete => MainDmoBankNotificationStage::Complete,
            Self::Faulted => MainDmoBankNotificationStage::Faulted,
        }
    }
}

/// One exact main-bank receiver occurrence and its still-withheld callbacks.
///
/// This type is deliberately private to the source-binding hierarchy. It has
/// no accessors for the prepared binding, controller, collection registration,
/// or deferred continuation. Dropping it performs no callback or receiver
/// work.
pub(super) struct MainDmoBankNotificationReceiver<ND, NE, L, D, RC, DC>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    state: ReceiverState,
    registration_callback: Option<RC>,
    deregistration_callback: Option<DC>,
    coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
}

impl<ND, NE, L, D, RC, DC> MainDmoBankNotificationReceiver<ND, NE, L, D, RC, DC>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    /// Fresh safe-host verification for an outer collection-message front.
    /// Callers must invoke this immediately before the complete Finish or `DReg`
    /// receiver operation, never while merely staging or enqueueing a marker.
    pub(super) fn try_revalidate_published_binding(
        &self,
    ) -> Result<(), MainDmoBankPublishedAcquisitionError> {
        self.coordinator.try_revalidate_published_binding()
    }

    /// Starts a caller-selected activation-nonzero route without executing it.
    pub(super) fn new_immediate(
        coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
        registration_callback: RC,
    ) -> Self {
        Self {
            coordinator,
            state: ReceiverState::ImmediateCollection,
            registration_callback: Some(registration_callback),
            deregistration_callback: None,
        }
    }

    /// Starts a caller-selected deferred route. This creates only the staged
    /// receiver/controller continuation; no Arena receiver is executed.
    #[allow(
        clippy::result_large_err,
        reason = "the owning refusal deliberately retains the unboxed coordinator and callback without adding an allocation-failure boundary"
    )]
    pub(super) fn try_new_deferred(
        mut coordinator: MainDmoBankCollectionCoordinator<ND, NE, L, D>,
        registration_callback: RC,
    ) -> Result<Self, MainDmoBankDeferredBeginRejection<ND, NE, L, D, RC>> {
        match coordinator.begin_deferred() {
            Ok(registration) => Ok(Self {
                state: ReceiverState::DeferredArena(registration),
                registration_callback: Some(registration_callback),
                deregistration_callback: None,
                coordinator,
            }),
            Err(source) => Err(MainDmoBankDeferredBeginRejection {
                source,
                registration_callback,
                coordinator,
            }),
        }
    }

    /// Executes one caller-asserted activation-nonzero Collection-message front.
    pub(super) fn receive_immediate_collection(
        &mut self,
    ) -> Result<(), MainDmoBankImmediateReceiverError> {
        if !matches!(self.state, ReceiverState::ImmediateCollection) {
            return Err(MainDmoBankImmediateReceiverError::WrongStage(
                self.wrong_stage(MainDmoBankNotificationStage::ImmediateCollection),
            ));
        }
        match self.coordinator.register_active() {
            Ok(registration) => {
                self.state = ReceiverState::RegistrationCallback(registration);
                Ok(())
            }
            Err(source) => {
                self.state = ReceiverState::Faulted;
                Err(MainDmoBankImmediateReceiverError::Receiver(source))
            }
        }
    }

    /// Executes one caller-asserted deferred Arena-message front. The authored
    /// full ID is freshly resolved and its current outer owner and plan are
    /// checked before the deferred continuation or any child effect is used.
    ///
    /// The returned Boolean is true only after the final exact arena prefix.
    pub(super) fn receive_deferred_arena(
        &mut self,
        ordinal: usize,
    ) -> Result<bool, MainDmoBankDeferredArenaReceiverError> {
        if !matches!(self.state, ReceiverState::DeferredArena(_)) {
            return Err(MainDmoBankDeferredArenaReceiverError::WrongStage(
                self.wrong_stage(MainDmoBankNotificationStage::DeferredArena),
            ));
        }
        let state = std::mem::replace(&mut self.state, ReceiverState::Faulted);
        let ReceiverState::DeferredArena(registration) = state else {
            let actual = state.stage();
            self.state = state;
            return Err(MainDmoBankDeferredArenaReceiverError::WrongStage(
                MainDmoBankNotificationWrongStage {
                    expected: MainDmoBankNotificationStage::DeferredArena,
                    actual,
                },
            ));
        };
        match self
            .coordinator
            .register_next_revalidated_deferred_arena(ordinal, registration)
        {
            Ok(MainDmoBankDeferredStep::Pending(registration)) => {
                self.state = ReceiverState::DeferredArena(registration);
                Ok(false)
            }
            Ok(MainDmoBankDeferredStep::PrefixComplete(registration)) => {
                self.state = ReceiverState::DeferredFinish(registration);
                Ok(true)
            }
            Err(MainDmoBankRevalidatedDeferredStepError::PublishedArenaRevalidation {
                source,
                registration,
            }) => {
                self.state = ReceiverState::DeferredArena(registration);
                Err(MainDmoBankDeferredArenaReceiverError::PublishedArenaRevalidation(source))
            }
            Err(MainDmoBankRevalidatedDeferredStepError::Receiver(source)) => {
                Err(MainDmoBankDeferredArenaReceiverError::Receiver(
                    MainDmoBankDeferredArenaReceiverFailure { source },
                ))
            }
        }
    }

    /// Executes the caller-asserted activation-zero Collection-message front.
    pub(super) fn receive_deferred_finish(
        &mut self,
    ) -> Result<(), MainDmoBankDeferredFinishReceiverError> {
        if !matches!(self.state, ReceiverState::DeferredFinish(_)) {
            return Err(MainDmoBankDeferredFinishReceiverError::WrongStage(
                self.wrong_stage(MainDmoBankNotificationStage::DeferredFinish),
            ));
        }
        self.try_revalidate_published_binding()
            .map_err(MainDmoBankDeferredFinishReceiverError::PublishedBindingRevalidation)?;
        let state = std::mem::replace(&mut self.state, ReceiverState::Faulted);
        let ReceiverState::DeferredFinish(registration) = state else {
            let actual = state.stage();
            self.state = state;
            return Err(MainDmoBankDeferredFinishReceiverError::WrongStage(
                MainDmoBankNotificationWrongStage {
                    expected: MainDmoBankNotificationStage::DeferredFinish,
                    actual,
                },
            ));
        };
        match self.coordinator.finish_deferred(registration) {
            Ok(registration) => {
                self.state = ReceiverState::RegistrationCallback(registration);
                Ok(())
            }
            Err(source) => Err(MainDmoBankDeferredFinishReceiverError::Receiver(
                MainDmoBankDeferredFinishReceiverFailure { source },
            )),
        }
    }

    /// Returns the retained registration callback at its caller-asserted front,
    /// only after receiver success.
    pub(super) fn receive_registration_callback(
        &mut self,
    ) -> Result<RC, MainDmoBankNotificationWrongStage> {
        let state = std::mem::replace(&mut self.state, ReceiverState::Faulted);
        let ReceiverState::RegistrationCallback(registration) = state else {
            let actual = state.stage();
            self.state = state;
            return Err(MainDmoBankNotificationWrongStage {
                expected: MainDmoBankNotificationStage::RegistrationCallback,
                actual,
            });
        };
        let callback = self
            .registration_callback
            .take()
            .expect("registration callback is retained until its exact boundary");
        self.state = ReceiverState::Registered(registration);
        Ok(callback)
    }

    /// Stages one later opaque `DReg` callback without executing `DReg` or
    /// asserting that a message front has been reached.
    pub(super) fn begin_deregistration(
        &mut self,
        callback: DC,
    ) -> Result<(), MainDmoBankCallbackReceiverRejection<DC>> {
        let state = std::mem::replace(&mut self.state, ReceiverState::Faulted);
        let ReceiverState::Registered(registration) = state else {
            let actual = state.stage();
            self.state = state;
            return Err(MainDmoBankCallbackReceiverRejection {
                stage: MainDmoBankNotificationWrongStage {
                    expected: MainDmoBankNotificationStage::Registered,
                    actual,
                },
                callback,
            });
        };
        debug_assert!(self.deregistration_callback.is_none());
        self.deregistration_callback = Some(callback);
        self.state = ReceiverState::DeregistrationCollection(registration);
        Ok(())
    }

    /// Executes one caller-asserted `DRegCollection`-message front.
    pub(super) fn receive_deregistration_collection(
        &mut self,
    ) -> Result<(), MainDmoBankDeregistrationReceiverError> {
        if !matches!(self.state, ReceiverState::DeregistrationCollection(_)) {
            return Err(MainDmoBankDeregistrationReceiverError::WrongStage(
                self.wrong_stage(MainDmoBankNotificationStage::DeregistrationCollection),
            ));
        }
        self.try_revalidate_published_binding()
            .map_err(MainDmoBankDeregistrationReceiverError::PublishedBindingRevalidation)?;
        let state = std::mem::replace(&mut self.state, ReceiverState::Faulted);
        let ReceiverState::DeregistrationCollection(registration) = state else {
            let actual = state.stage();
            self.state = state;
            return Err(MainDmoBankDeregistrationReceiverError::WrongStage(
                MainDmoBankNotificationWrongStage {
                    expected: MainDmoBankNotificationStage::DeregistrationCollection,
                    actual,
                },
            ));
        };
        match self.coordinator.deregister(registration) {
            Ok(()) => {
                self.state = ReceiverState::DeregistrationCallback;
                Ok(())
            }
            Err(source) => Err(MainDmoBankDeregistrationReceiverError::Receiver(
                MainDmoBankDeregistrationReceiverFailure { source },
            )),
        }
    }

    /// Returns the retained `DReg` callback at its caller-asserted front, only
    /// after receiver success.
    pub(super) fn receive_deregistration_callback(
        &mut self,
    ) -> Result<DC, MainDmoBankNotificationWrongStage> {
        if !matches!(self.state, ReceiverState::DeregistrationCallback) {
            return Err(self.wrong_stage(MainDmoBankNotificationStage::DeregistrationCallback));
        }
        let callback = self
            .deregistration_callback
            .take()
            .expect("DReg callback is retained until its exact boundary");
        self.state = ReceiverState::Complete;
        Ok(callback)
    }

    const fn wrong_stage(
        &self,
        expected: MainDmoBankNotificationStage,
    ) -> MainDmoBankNotificationWrongStage {
        MainDmoBankNotificationWrongStage {
            expected,
            actual: self.state.stage(),
        }
    }

    #[cfg(test)]
    pub(super) fn stage_for_test(&self) -> MainDmoBankNotificationStage {
        self.state.stage()
    }

    #[cfg(test)]
    pub(super) fn registration_callback_retained_for_test(&self) -> bool {
        self.registration_callback.is_some()
    }

    #[cfg(test)]
    pub(super) fn deregistration_callback_retained_for_test(&self) -> bool {
        self.deregistration_callback.is_some()
    }

    #[cfg(test)]
    pub(super) fn live_registration_count_for_test(&self) -> usize {
        self.coordinator.live_registration_count()
    }

    #[cfg(test)]
    pub(super) fn has_model_for_test(&self, model_id: u64) -> bool {
        self.coordinator.has_model(model_id)
    }

    #[cfg(test)]
    pub(super) fn try_revalidate_published_binding_for_test(
        &self,
    ) -> Result<(), MainDmoBankPublishedAcquisitionError> {
        self.try_revalidate_published_binding()
    }
}
