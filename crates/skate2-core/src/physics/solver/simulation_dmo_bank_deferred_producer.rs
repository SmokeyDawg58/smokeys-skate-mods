//! Dense deferred producer updates for the exact shipped main DMO bank.
//!
//! This private owner advances only the already-admitted dense 125-arena
//! producer. Pending performs Setup as the final substantive action of its
//! update and deliberately does not sample batch controls or emit an Arena.
//! A later Active update samples the caller's controls once, applies
//! `GetActiveCollectionRegIndex` with the bank's raw Presentation stream type
//! zero, and appends that authored-order Arena batch. The completing update
//! appends Finish and `RegistrationCallback` after its final Arena.
//!
//! It does not select admission, scan or truncate at a null Arena slot,
//! dispatch a marker, expose a permit, or claim global `EPostman` capacity,
//! draining, interleaving, or frame ownership. Native selection uses the full
//! vector and an indexed dereference; this fixed dense 125 route is a validated
//! shipped-corpus safe-host specialization. Typed failures, preflighted
//! allocation, retained ownership, and a no-op Terminal update are safe-host
//! policy; native producer calls are `void`.

#![allow(
    dead_code,
    clippy::result_large_err,
    clippy::type_complexity,
    reason = "staged owning producer awaits its private outer scheduler; unboxed refusals retain all exact capabilities"
)]

use super::marker_fifo::{
    MainDmoBankDeferredPendingSetup, MainDmoBankDeferredPendingSetupError,
    MainDmoBankDeferredPendingSetupFailure, MainDmoBankDeferredReady,
    MainDmoBankMarkerEnqueueRejectionKind,
};
use crate::{
    dmo_bank_processed_asset::{DMO_BANK_ASSET_COUNT, DmoBankProcessedArenaAsset},
    dmo_hull_acknowledgement::MainDmoSimulationRegistrationCallback,
    simulation_collection_adapter::{
        SimulationCollectionDmoPass, SimulationCollectionLocationPass,
    },
    simulation_deferred_batch_policy::{
        DeferredCollectionBatchControls, DeferredCollectionProgressError,
    },
    simulation_normal_arena_adapter::{
        SimulationNormalArenaDispatch, SimulationNormalArenaRegistrationAdapter,
        SimulationNormalEnvironmentDispatch,
    },
    simulation_resident_registry::SimulationResidentLookup,
};
use std::{error::Error, fmt};

const DMO_BANK_RAW_STREAM_TYPE: u32 = 0;

struct MainDmoBankDenseDeferredPending<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    inner: MainDmoBankDeferredPendingSetup<ND, NE, L, D>,
}

struct MainDmoBankDenseDeferredPendingFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    inner: MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>,
}

struct MainDmoBankDenseDeferredActive<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    inner: MainDmoBankDeferredReady<ND, NE, L, D>,
}

struct MainDmoBankDenseDeferredTerminal<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    // Retains the mature FIFO and all queued markers without granting any
    // dispatch or callback authority to this producer boundary.
    inner: MainDmoBankDeferredReady<ND, NE, L, D>,
}

enum MainDmoBankDenseDeferredProducerState<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    Pending(MainDmoBankDenseDeferredPending<ND, NE, L, D>),
    PendingFailure(MainDmoBankDenseDeferredPendingFailure<ND, NE, L, D>),
    Active(MainDmoBankDenseDeferredActive<ND, NE, L, D>),
    Terminal(MainDmoBankDenseDeferredTerminal<ND, NE, L, D>),
}

/// Opaque exact-main-bank producer owner. It is neither cloneable nor
/// debuggable and exposes no underlying Pending, Ready, FIFO, or permit.
#[must_use]
pub(super) struct MainDmoBankDenseDeferredProducer<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    state: MainDmoBankDenseDeferredProducerState<ND, NE, L, D>,
}

/// Observable result of one producer update. It describes only this owner's
/// local append progress and makes no global queue or dispatch claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankDenseDeferredProducerUpdate {
    SetupCompleted,
    ArenasQueued { count: u32 },
    TerminalQueued { arena_count: u32 },
    TerminalIdle,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankDenseDeferredProducerStage {
    Pending,
    PendingFailure,
    Active,
    Terminal,
}

/// Typed refusal from one dense producer update.
#[derive(Debug)]
pub(super) enum MainDmoBankDenseDeferredProducerUpdateError<E> {
    Setup(MainDmoBankDeferredPendingSetupError),
    Controls(E),
    Progress(DeferredCollectionProgressError),
    Queue(MainDmoBankMarkerEnqueueRejectionKind),
}

impl<E: fmt::Display> fmt::Display for MainDmoBankDenseDeferredProducerUpdateError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(source) => source.fmt(formatter),
            Self::Controls(source) => {
                write!(formatter, "could not sample deferred controls: {source}")
            }
            Self::Progress(source) => source.fmt(formatter),
            Self::Queue(_) => formatter.write_str(
                "could not reserve the complete main DMO bank deferred update marker batch",
            ),
        }
    }
}

impl<E: Error + 'static> Error for MainDmoBankDenseDeferredProducerUpdateError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Setup(source) => Some(source),
            Self::Controls(source) => Some(source),
            Self::Progress(source) => Some(source),
            Self::Queue(_) => None,
        }
    }
}

/// Owning update refusal. Retry remains on the exact Pending failure or Active
/// FIFO; no callback, producer permit, or ready half can be extracted.
#[must_use]
pub(super) struct MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    error: MainDmoBankDenseDeferredProducerUpdateError<E>,
    producer: MainDmoBankDenseDeferredProducer<ND, NE, L, D>,
}

impl<ND, NE, L, D, E> MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    pub(super) const fn error(&self) -> &MainDmoBankDenseDeferredProducerUpdateError<E> {
        &self.error
    }

    pub(super) fn retry_with<F>(
        self,
        controls: F,
    ) -> Result<
        (
            MainDmoBankDenseDeferredProducer<ND, NE, L, D>,
            MainDmoBankDenseDeferredProducerUpdate,
        ),
        Self,
    >
    where
        F: FnOnce() -> Result<DeferredCollectionBatchControls, E>,
    {
        self.producer.try_update_with(controls)
    }

    #[cfg(test)]
    pub(super) const fn stage_for_test(&self) -> MainDmoBankDenseDeferredProducerStage {
        self.producer.stage_for_test()
    }

    #[cfg(test)]
    pub(super) fn queued_marker_kinds_for_test(
        &self,
    ) -> Vec<super::marker_fifo::MainDmoBankMarkerKind> {
        self.producer.queued_marker_kinds_for_test()
    }

    #[cfg(test)]
    pub(super) fn next_ordinal_for_test(&self) -> usize {
        self.producer.next_ordinal_for_test()
    }

    #[cfg(test)]
    pub(super) fn last_reservation_size_for_test(&self) -> Option<usize> {
        self.producer.last_reservation_size_for_test()
    }
}

impl<ND, NE, L, D, E: fmt::Debug> fmt::Debug
    for MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankDenseDeferredProducerUpdateFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<ND, NE, L, D, E: fmt::Display> fmt::Display
    for MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>
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

impl<ND, NE, L, D, E: Error + 'static> Error
    for MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

impl<ND, NE, L, D> MainDmoBankDenseDeferredProducer<ND, NE, L, D>
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
            state: MainDmoBankDenseDeferredProducerState::Pending(
                MainDmoBankDenseDeferredPending {
                    inner: MainDmoBankDeferredPendingSetup::new(
                        lookup, normal, location, dmo, callback,
                    ),
                },
            ),
        }
    }

    /// Advances exactly one producer update. The controls provider is lazy:
    /// Pending and Terminal updates never call it; each Active update calls it
    /// exactly once before any marker append.
    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "the owning refusal deliberately retains the exact private producer state"
    )]
    pub(super) fn try_update_with<F, E>(
        self,
        controls: F,
    ) -> Result<
        (Self, MainDmoBankDenseDeferredProducerUpdate),
        MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>,
    >
    where
        F: FnOnce() -> Result<DeferredCollectionBatchControls, E>,
    {
        match self.state {
            MainDmoBankDenseDeferredProducerState::Pending(pending) => {
                Self::update_pending(pending.inner)
            }
            MainDmoBankDenseDeferredProducerState::PendingFailure(pending) => {
                Self::update_pending_failure(pending.inner)
            }
            MainDmoBankDenseDeferredProducerState::Active(active) => {
                Self::update_active(active.inner, controls)
            }
            MainDmoBankDenseDeferredProducerState::Terminal(terminal) => Ok((
                Self {
                    state: MainDmoBankDenseDeferredProducerState::Terminal(terminal),
                },
                MainDmoBankDenseDeferredProducerUpdate::TerminalIdle,
            )),
        }
    }

    /// Extracts the complete staged FIFO only from Terminal. A Pending or
    /// Active caller receives the unchanged producer owner back.
    pub(super) fn try_into_terminal_fifo(
        self,
    ) -> Result<super::marker_fifo::MainDmoBankMarkerFifo<ND, NE, L, D>, Self> {
        match self.state {
            MainDmoBankDenseDeferredProducerState::Terminal(terminal) => {
                Ok(terminal.inner.into_terminal_fifo())
            }
            state => Err(Self { state }),
        }
    }

    fn update_pending<E>(
        pending: MainDmoBankDeferredPendingSetup<ND, NE, L, D>,
    ) -> Result<
        (Self, MainDmoBankDenseDeferredProducerUpdate),
        MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>,
    > {
        match pending.try_setup() {
            Ok(ready) => Ok((
                Self {
                    state: MainDmoBankDenseDeferredProducerState::Active(
                        MainDmoBankDenseDeferredActive { inner: ready },
                    ),
                },
                MainDmoBankDenseDeferredProducerUpdate::SetupCompleted,
            )),
            Err(source) => Self::pending_failure(source),
        }
    }

    fn update_pending_failure<E>(
        pending: MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>,
    ) -> Result<
        (Self, MainDmoBankDenseDeferredProducerUpdate),
        MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>,
    > {
        match pending.retry() {
            Ok(ready) => Ok((
                Self {
                    state: MainDmoBankDenseDeferredProducerState::Active(
                        MainDmoBankDenseDeferredActive { inner: ready },
                    ),
                },
                MainDmoBankDenseDeferredProducerUpdate::SetupCompleted,
            )),
            Err(source) => Self::pending_failure(source),
        }
    }

    fn pending_failure<E>(
        source: MainDmoBankDeferredPendingSetupFailure<ND, NE, L, D>,
    ) -> Result<
        (Self, MainDmoBankDenseDeferredProducerUpdate),
        MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>,
    > {
        let error = source.error();
        Err(MainDmoBankDenseDeferredProducerUpdateFailure {
            error: MainDmoBankDenseDeferredProducerUpdateError::Setup(error),
            producer: Self {
                state: MainDmoBankDenseDeferredProducerState::PendingFailure(
                    MainDmoBankDenseDeferredPendingFailure { inner: source },
                ),
            },
        })
    }

    fn update_active<F, E>(
        mut ready: MainDmoBankDeferredReady<ND, NE, L, D>,
        controls: F,
    ) -> Result<
        (Self, MainDmoBankDenseDeferredProducerUpdate),
        MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E>,
    >
    where
        F: FnOnce() -> Result<DeferredCollectionBatchControls, E>,
    {
        let controls = match controls() {
            Ok(controls) => controls,
            Err(error) => {
                return Err(Self::active_failure(
                    ready,
                    MainDmoBankDenseDeferredProducerUpdateError::Controls(error),
                ));
            }
        };
        let active_index = u32::try_from(ready.next_deferred_ordinal())
            .expect("the pinned 125-arena producer ordinal fits u32");
        let arena_count = match controls.arenas_for_update_raw(
            DMO_BANK_RAW_STREAM_TYPE,
            active_index,
            DMO_BANK_ASSET_COUNT,
        ) {
            Ok(count) => count,
            Err(error) => {
                return Err(Self::active_failure(
                    ready,
                    MainDmoBankDenseDeferredProducerUpdateError::Progress(error),
                ));
            }
        };
        let arena_count_usize =
            usize::try_from(arena_count).expect("the pinned 125-arena producer batch fits usize");
        match ready.try_enqueue_dense_deferred_update(arena_count_usize) {
            Ok(false) => Ok((
                Self {
                    state: MainDmoBankDenseDeferredProducerState::Active(
                        MainDmoBankDenseDeferredActive { inner: ready },
                    ),
                },
                MainDmoBankDenseDeferredProducerUpdate::ArenasQueued { count: arena_count },
            )),
            Ok(true) => Ok((
                Self {
                    state: MainDmoBankDenseDeferredProducerState::Terminal(
                        MainDmoBankDenseDeferredTerminal { inner: ready },
                    ),
                },
                MainDmoBankDenseDeferredProducerUpdate::TerminalQueued { arena_count },
            )),
            Err(error) => Err(Self::active_failure(
                ready,
                MainDmoBankDenseDeferredProducerUpdateError::Queue(error),
            )),
        }
    }

    fn active_failure<E>(
        ready: MainDmoBankDeferredReady<ND, NE, L, D>,
        error: MainDmoBankDenseDeferredProducerUpdateError<E>,
    ) -> MainDmoBankDenseDeferredProducerUpdateFailure<ND, NE, L, D, E> {
        MainDmoBankDenseDeferredProducerUpdateFailure {
            error,
            producer: Self {
                state: MainDmoBankDenseDeferredProducerState::Active(
                    MainDmoBankDenseDeferredActive { inner: ready },
                ),
            },
        }
    }

    #[cfg(test)]
    pub(super) const fn stage_for_test(&self) -> MainDmoBankDenseDeferredProducerStage {
        match self.state {
            MainDmoBankDenseDeferredProducerState::Pending(_) => {
                MainDmoBankDenseDeferredProducerStage::Pending
            }
            MainDmoBankDenseDeferredProducerState::PendingFailure(_) => {
                MainDmoBankDenseDeferredProducerStage::PendingFailure
            }
            MainDmoBankDenseDeferredProducerState::Active(_) => {
                MainDmoBankDenseDeferredProducerStage::Active
            }
            MainDmoBankDenseDeferredProducerState::Terminal(_) => {
                MainDmoBankDenseDeferredProducerStage::Terminal
            }
        }
    }

    #[cfg(test)]
    pub(super) fn queued_markers_for_test(&self) -> usize {
        match &self.state {
            MainDmoBankDenseDeferredProducerState::Pending(_)
            | MainDmoBankDenseDeferredProducerState::PendingFailure(_) => 0,
            MainDmoBankDenseDeferredProducerState::Active(active) => {
                active.inner.queue_len_for_test()
            }
            MainDmoBankDenseDeferredProducerState::Terminal(terminal) => {
                terminal.inner.queue_len_for_test()
            }
        }
    }

    #[cfg(test)]
    pub(super) fn next_ordinal_for_test(&self) -> usize {
        match &self.state {
            MainDmoBankDenseDeferredProducerState::Pending(_)
            | MainDmoBankDenseDeferredProducerState::PendingFailure(_) => 0,
            MainDmoBankDenseDeferredProducerState::Active(active) => {
                active.inner.next_deferred_ordinal()
            }
            MainDmoBankDenseDeferredProducerState::Terminal(terminal) => {
                terminal.inner.next_deferred_ordinal()
            }
        }
    }

    #[cfg(test)]
    pub(super) fn queued_marker_kinds_for_test(
        &self,
    ) -> Vec<super::marker_fifo::MainDmoBankMarkerKind> {
        match &self.state {
            MainDmoBankDenseDeferredProducerState::Pending(_)
            | MainDmoBankDenseDeferredProducerState::PendingFailure(_) => Vec::new(),
            MainDmoBankDenseDeferredProducerState::Active(active) => {
                active.inner.queued_marker_kinds_for_test()
            }
            MainDmoBankDenseDeferredProducerState::Terminal(terminal) => {
                terminal.inner.queued_marker_kinds_for_test()
            }
        }
    }

    #[cfg(test)]
    pub(super) fn fail_next_queue_reserve_for_test(&mut self) -> bool {
        let MainDmoBankDenseDeferredProducerState::Active(active) = &mut self.state else {
            return false;
        };
        active.inner.fail_next_dense_reserve_for_test();
        true
    }

    #[cfg(test)]
    fn last_reservation_size_for_test(&self) -> Option<usize> {
        match &self.state {
            MainDmoBankDenseDeferredProducerState::Pending(_)
            | MainDmoBankDenseDeferredProducerState::PendingFailure(_) => None,
            MainDmoBankDenseDeferredProducerState::Active(active) => {
                active.inner.last_dense_reservation_size_for_test()
            }
            MainDmoBankDenseDeferredProducerState::Terminal(terminal) => {
                terminal.inner.last_dense_reservation_size_for_test()
            }
        }
    }
}
