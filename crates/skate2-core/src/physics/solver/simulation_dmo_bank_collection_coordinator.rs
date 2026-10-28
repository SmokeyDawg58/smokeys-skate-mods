//! Private main-DMO-bank collection/controller composition.
//!
//! The XEX proves the nested call order, not a native controller or Rust
//! authority object. This owner keeps the exact source cursor inseparable from
//! the prepared main-bank collection and finishes it only at recovered pass
//! boundaries.

use super::{
    MainDmoBankPublishedAcquisitionError, MainDmoBankPublishedArenaRevalidationError,
    MainDmoBankPublishedBindingOwner, PreparedMainDmoBankSimulationCollection,
};
use crate::{
    simulation_collection_adapter::{
        DeferredSimulationCollectionRegistration, SimulationCollectionActiveDmoContinuation,
        SimulationCollectionDeferredFinishError, SimulationCollectionDeferredFinishValidationError,
        SimulationCollectionDeferredStepError, SimulationCollectionDeregistrationCompletion,
        SimulationCollectionDeregistrationError, SimulationCollectionDmoModelActivePrefixError,
        SimulationCollectionDmoModelDeferredStepError,
        SimulationCollectionDmoModelDeregistrationError, SimulationCollectionDmoPass,
        SimulationCollectionLocationPass, SimulationCollectionRegistration,
        SimulationCollectionRegistrationAdapter, SimulationCollectionRegistrationError,
    },
    simulation_dmo_assembly_part_session::{
        SimulationDmoBankModelDeregisterPassError, SimulationDmoBankModelRegisterController,
        SimulationDmoBankModelRegisterPassError, SimulationDmoRegistrationRuntime,
        SimulationDmoRegistrationRuntimeInitError,
    },
    simulation_normal_arena_adapter::{
        SimulationNormalArenaDispatch, SimulationNormalArenaRegistrationAdapter,
        SimulationNormalEnvironmentDispatch,
    },
};
use std::{error::Error, fmt};

/// The raw userdata observed for the resident main DMO bank.
pub(super) const MAIN_DMO_BANK_USERDATA_RAW: u32 = 0x0a;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MainDmoBankCollectionControllerFault {
    RegistrationArena { arena_index: usize },
    RegistrationFinish,
    DeregistrationArena { arena_index: usize },
    DeregistrationFinish,
}

/// Complete outer authority for one main-bank occurrence.
pub(super) struct MainDmoBankCollectionRegistration {
    inner: SimulationCollectionRegistration,
}

impl fmt::Debug for MainDmoBankCollectionRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankCollectionRegistration")
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(super) enum MainDmoBankCollectionRegistrationError {
    ControllerFaulted(MainDmoBankCollectionControllerFault),
    Collection(SimulationCollectionRegistrationError),
    Controller {
        arena_index: usize,
        source: SimulationDmoBankModelRegisterPassError,
    },
    ControllerFinish(SimulationDmoBankModelRegisterPassError),
}

impl fmt::Display for MainDmoBankCollectionRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControllerFaulted(fault) => {
                write!(formatter, "main DMO bank coordinator is faulted: {fault:?}")
            }
            Self::Collection(source) => source.fmt(formatter),
            Self::Controller { arena_index, .. } => write!(
                formatter,
                "main DMO bank model registration refused in arena {arena_index}"
            ),
            Self::ControllerFinish(_) => {
                formatter.write_str("main DMO bank model registration cursor was incomplete")
            }
        }
    }
}

impl Error for MainDmoBankCollectionRegistrationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Collection(source) => Some(source),
            Self::Controller { source, .. } | Self::ControllerFinish(source) => Some(source),
            Self::ControllerFaulted(_) => None,
        }
    }
}

pub(super) struct MainDmoBankDeferredRegistration {
    inner: DeferredSimulationCollectionRegistration,
    controller: SimulationDmoBankModelRegisterController,
}

impl fmt::Debug for MainDmoBankDeferredRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankDeferredRegistration")
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(super) enum MainDmoBankDeferredStep {
    Pending(MainDmoBankDeferredRegistration),
    PrefixComplete(MainDmoBankDeferredRegistration),
}

impl MainDmoBankDeferredStep {
    pub(super) const fn is_prefix_complete(&self) -> bool {
        matches!(self, Self::PrefixComplete(_))
    }

    pub(super) fn into_registration(self) -> MainDmoBankDeferredRegistration {
        match self {
            Self::Pending(registration) | Self::PrefixComplete(registration) => registration,
        }
    }
}

pub(super) enum MainDmoBankDeferredStepError {
    ControllerFaulted {
        fault: MainDmoBankCollectionControllerFault,
        registration: MainDmoBankDeferredRegistration,
    },
    Collection {
        source: SimulationCollectionDeferredStepError,
        controller: SimulationDmoBankModelRegisterController,
    },
    Controller {
        arena_index: usize,
        source: SimulationDmoBankModelRegisterPassError,
    },
}

impl MainDmoBankDeferredStepError {
    pub(super) fn into_registration(self) -> Option<MainDmoBankDeferredRegistration> {
        match self {
            Self::ControllerFaulted { registration, .. } => Some(registration),
            Self::Collection { source, controller } => source
                .into_registration()
                .map(|inner| MainDmoBankDeferredRegistration { inner, controller }),
            Self::Controller { .. } => None,
        }
    }
}

impl fmt::Debug for MainDmoBankDeferredStepError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControllerFaulted { fault, .. } => formatter
                .debug_struct("ControllerFaulted")
                .field("fault", fault)
                .finish_non_exhaustive(),
            Self::Collection { source, .. } => {
                formatter.debug_tuple("Collection").field(source).finish()
            }
            Self::Controller {
                arena_index,
                source,
            } => formatter
                .debug_struct("Controller")
                .field("arena_index", arena_index)
                .field("source", source)
                .finish(),
        }
    }
}

pub(super) enum MainDmoBankDeferredFinishError {
    ControllerFaulted {
        fault: MainDmoBankCollectionControllerFault,
        registration: MainDmoBankDeferredRegistration,
    },
    Validation {
        source: SimulationCollectionDeferredFinishValidationError,
        registration: MainDmoBankDeferredRegistration,
    },
    ControllerFinish(SimulationDmoBankModelRegisterPassError),
    Collection(SimulationCollectionDeferredFinishError),
}

impl MainDmoBankDeferredFinishError {
    pub(super) fn into_registration(self) -> Option<MainDmoBankDeferredRegistration> {
        match self {
            Self::ControllerFaulted { registration, .. }
            | Self::Validation { registration, .. } => Some(registration),
            Self::ControllerFinish(_) | Self::Collection(_) => None,
        }
    }
}

impl fmt::Debug for MainDmoBankDeferredFinishError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControllerFaulted { fault, .. } => formatter
                .debug_struct("ControllerFaulted")
                .field("fault", fault)
                .finish_non_exhaustive(),
            Self::Validation { source, .. } => formatter
                .debug_struct("Validation")
                .field("source", source)
                .finish_non_exhaustive(),
            Self::ControllerFinish(source) => formatter
                .debug_tuple("ControllerFinish")
                .field(source)
                .finish(),
            Self::Collection(source) => formatter.debug_tuple("Collection").field(source).finish(),
        }
    }
}

#[derive(Debug)]
pub(super) enum MainDmoBankCollectionDeregistrationError {
    ControllerFaulted {
        fault: MainDmoBankCollectionControllerFault,
        registration: MainDmoBankCollectionRegistration,
    },
    Collection(SimulationCollectionDeregistrationError),
    Controller {
        arena_index: usize,
        source: SimulationDmoBankModelDeregisterPassError,
    },
    ControllerFinish(SimulationDmoBankModelDeregisterPassError),
}

impl MainDmoBankCollectionDeregistrationError {
    pub(super) fn into_registration(self) -> Option<MainDmoBankCollectionRegistration> {
        match self {
            Self::ControllerFaulted { registration, .. } => Some(registration),
            Self::Collection(source) => source
                .into_registration()
                .map(|inner| MainDmoBankCollectionRegistration { inner }),
            Self::Controller { .. } | Self::ControllerFinish(_) => None,
        }
    }
}

impl fmt::Display for MainDmoBankCollectionDeregistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControllerFaulted { fault, .. } => {
                write!(formatter, "main DMO bank coordinator is faulted: {fault:?}")
            }
            Self::Collection(source) => source.fmt(formatter),
            Self::Controller { arena_index, .. } => write!(
                formatter,
                "main DMO bank model deregistration refused in arena {arena_index}"
            ),
            Self::ControllerFinish(_) => {
                formatter.write_str("main DMO bank model deregistration cursor was incomplete")
            }
        }
    }
}

impl Error for MainDmoBankCollectionDeregistrationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Collection(source) => Some(source),
            Self::Controller { source, .. } | Self::ControllerFinish(source) => Some(source),
            Self::ControllerFaulted { .. } => None,
        }
    }
}

/// Synchronous active receiver plus later unconditional deregistration for the
/// one pinned resident main DMO bank.
///
/// Field order drops child adapters and model-manager state before releasing
/// the retained exact binding owners.
pub(super) struct MainDmoBankCollectionCoordinator<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    adapter: SimulationCollectionRegistrationAdapter<
        SimulationNormalArenaRegistrationAdapter<ND, NE>,
        L,
        D,
    >,
    runtime: SimulationDmoRegistrationRuntime,
    binding: MainDmoBankCollectionBindingSource,
    controller_fault: Option<MainDmoBankCollectionControllerFault>,
    stranded_active: Option<SimulationCollectionActiveDmoContinuation>,
    stranded_deferred: Option<DeferredSimulationCollectionRegistration>,
    stranded_deregistration: Option<SimulationCollectionDeregistrationCompletion>,
}

/// Production retains the complete published occurrence owner. The synthetic
/// variant exists only to keep isolated collection/controller fixtures from
/// minting resident-registry authority.
enum MainDmoBankCollectionBindingSource {
    Published(MainDmoBankPublishedBindingOwner),
    #[cfg(test)]
    Synthetic(PreparedMainDmoBankSimulationCollection),
}

impl MainDmoBankCollectionBindingSource {
    fn binding(&self) -> &PreparedMainDmoBankSimulationCollection {
        match self {
            Self::Published(owner) => &owner.binding,
            #[cfg(test)]
            Self::Synthetic(binding) => binding,
        }
    }

    fn try_revalidate(&self) -> Result<(), MainDmoBankPublishedAcquisitionError> {
        match self {
            Self::Published(owner) => owner.try_revalidate(),
            #[cfg(test)]
            Self::Synthetic(_) => Ok(()),
        }
    }

    fn try_with_revalidated_deferred_arena<R>(
        &self,
        ordinal: usize,
        effect: impl FnOnce() -> R,
    ) -> Result<R, MainDmoBankPublishedArenaRevalidationError> {
        match self {
            Self::Published(owner) => owner.try_with_revalidated_deferred_arena(ordinal, effect),
            #[cfg(test)]
            Self::Synthetic(_) => Ok(effect()),
        }
    }
}

/// Owning split between an effect-free current-Arena refusal and the existing
/// effect failure. The former returns the untouched deferred continuation so
/// the receiver can retain its exact stage while the FIFO stores the fault.
pub(super) enum MainDmoBankRevalidatedDeferredStepError {
    PublishedArenaRevalidation {
        source: MainDmoBankPublishedArenaRevalidationError,
        registration: MainDmoBankDeferredRegistration,
    },
    Receiver(MainDmoBankDeferredStepError),
}

/// Owning safe-host refusal from the sole fallible coordinator initialization
/// boundary.
///
/// Field order preserves the successful owner's collaborator-before-published-
/// owner release order when this value is abandoned. Debug and diagnostics
/// expose only the typed runtime initializer error, never recovery authority or
/// a dispatcher.
pub(super) struct MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    runtime_source: SimulationDmoRegistrationRuntimeInitError,
    normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
    location: L,
    dmo: D,
    binding: MainDmoBankCollectionBindingSource,
}

impl<ND, NE, L, D> MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    #[must_use]
    pub(super) const fn source(&self) -> SimulationDmoRegistrationRuntimeInitError {
        self.runtime_source
    }

    /// Retries runtime initialization with the same complete published owner
    /// and collaborators. A repeated refusal returns another complete owner.
    #[allow(
        clippy::result_large_err,
        clippy::type_complexity,
        reason = "the owning refusal intentionally retains every non-reconstructible input"
    )]
    pub(super) fn retry(
        self,
    ) -> Result<
        MainDmoBankCollectionCoordinator<ND, NE, L, D>,
        MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>,
    > {
        MainDmoBankCollectionCoordinator::try_new_with_runtime(
            self.binding,
            self.normal,
            self.location,
            self.dmo,
            SimulationDmoRegistrationRuntime::try_new(),
        )
    }

    /// Deterministically repeats the same ownership-preserving refusal with a
    /// second injected runtime error. This is test-only and performs no
    /// dispatch or runtime-manager effect.
    #[cfg(test)]
    pub(super) fn retry_with_runtime_failure_for_test(
        self,
        runtime_source: SimulationDmoRegistrationRuntimeInitError,
    ) -> Self {
        let Err(failure) = MainDmoBankCollectionCoordinator::try_new_with_runtime(
            self.binding,
            self.normal,
            self.location,
            self.dmo,
            Err(runtime_source),
        ) else {
            unreachable!("injected runtime refusal must remain deterministic")
        };
        failure
    }
}

impl<ND, NE, L, D> fmt::Debug for MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MainDmoBankCollectionCoordinatorInitFailure")
            .field("source", &self.runtime_source)
            .finish_non_exhaustive()
    }
}

impl<ND, NE, L, D> fmt::Display for MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "main DMO bank coordinator initialization failed: {}",
            self.runtime_source
        )
    }
}

impl<ND, NE, L, D> Error for MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.runtime_source)
    }
}

fn register_next_deferred_arena_effect<ND, NE, L, D>(
    adapter: &mut SimulationCollectionRegistrationAdapter<
        SimulationNormalArenaRegistrationAdapter<ND, NE>,
        L,
        D,
    >,
    runtime: &mut SimulationDmoRegistrationRuntime,
    controller_fault: &mut Option<MainDmoBankCollectionControllerFault>,
    registration: MainDmoBankDeferredRegistration,
) -> Result<MainDmoBankDeferredStep, MainDmoBankDeferredStepError>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    if let Some(fault) = *controller_fault {
        return Err(MainDmoBankDeferredStepError::ControllerFaulted {
            fault,
            registration,
        });
    }
    let MainDmoBankDeferredRegistration {
        inner,
        mut controller,
    } = registration;
    match adapter.register_next_deferred_arena_with_dmo_controller(inner, runtime, &mut controller)
    {
        Ok(step) => {
            let complete = step.is_prefix_complete();
            let registration = MainDmoBankDeferredRegistration {
                inner: step.into_registration(),
                controller,
            };
            if complete {
                Ok(MainDmoBankDeferredStep::PrefixComplete(registration))
            } else {
                Ok(MainDmoBankDeferredStep::Pending(registration))
            }
        }
        Err(SimulationCollectionDmoModelDeferredStepError::Collection(source)) => {
            Err(MainDmoBankDeferredStepError::Collection { source, controller })
        }
        Err(SimulationCollectionDmoModelDeferredStepError::Controller {
            arena_index,
            source,
        }) => {
            *controller_fault =
                Some(MainDmoBankCollectionControllerFault::RegistrationArena { arena_index });
            Err(MainDmoBankDeferredStepError::Controller {
                arena_index,
                source,
            })
        }
    }
}

impl<ND, NE, L, D> MainDmoBankCollectionCoordinator<ND, NE, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    #[allow(
        dead_code,
        clippy::result_large_err,
        reason = "staged production entry awaits the private published-owner coordinator seam; the owning refusal intentionally retains every non-reconstructible input"
    )]
    pub(super) fn try_new(
        binding: MainDmoBankPublishedBindingOwner,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
    ) -> Result<Self, MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>> {
        let runtime = SimulationDmoRegistrationRuntime::try_new();
        Self::try_new_with_runtime(
            MainDmoBankCollectionBindingSource::Published(binding),
            normal,
            location,
            dmo,
            runtime,
        )
    }

    #[allow(
        clippy::result_large_err,
        reason = "the owning refusal intentionally retains every non-reconstructible input"
    )]
    fn try_new_with_runtime(
        binding: MainDmoBankCollectionBindingSource,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        runtime: Result<
            SimulationDmoRegistrationRuntime,
            SimulationDmoRegistrationRuntimeInitError,
        >,
    ) -> Result<Self, MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>> {
        let runtime = match runtime {
            Ok(runtime) => runtime,
            Err(runtime_source) => {
                return Err(MainDmoBankCollectionCoordinatorInitFailure {
                    runtime_source,
                    normal,
                    location,
                    dmo,
                    binding,
                });
            }
        };
        Ok(Self {
            adapter: SimulationCollectionRegistrationAdapter::new(normal, location, dmo),
            runtime,
            binding,
            controller_fault: None,
            stranded_active: None,
            stranded_deferred: None,
            stranded_deregistration: None,
        })
    }

    /// Test-only construction for fixtures that deliberately have no resident
    /// registry or published-occurrence authority.
    #[cfg(test)]
    #[allow(
        clippy::result_large_err,
        reason = "the test-only path preserves the same owning failure shape as production"
    )]
    pub(super) fn try_new_synthetic(
        binding: PreparedMainDmoBankSimulationCollection,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
    ) -> Result<Self, MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D>> {
        Self::try_new_with_runtime(
            MainDmoBankCollectionBindingSource::Synthetic(binding),
            normal,
            location,
            dmo,
            SimulationDmoRegistrationRuntime::try_new(),
        )
    }

    /// Deterministic test-only entry to the production ownership-preserving
    /// initializer failure path.
    #[cfg(test)]
    pub(super) fn fail_published_runtime_initialization_for_test(
        binding: MainDmoBankPublishedBindingOwner,
        normal: SimulationNormalArenaRegistrationAdapter<ND, NE>,
        location: L,
        dmo: D,
        runtime_source: SimulationDmoRegistrationRuntimeInitError,
    ) -> MainDmoBankCollectionCoordinatorInitFailure<ND, NE, L, D> {
        let Err(failure) = Self::try_new_with_runtime(
            MainDmoBankCollectionBindingSource::Published(binding),
            normal,
            location,
            dmo,
            Err(runtime_source),
        ) else {
            unreachable!("injected runtime refusal must remain deterministic")
        };
        failure
    }

    /// Fresh safe-host verification of the retained published occurrence and
    /// its wrapper-time asset sequence. Synthetic test fixtures have no
    /// published identity and therefore perform no check.
    pub(super) fn try_revalidate_published_binding(
        &self,
    ) -> Result<(), MainDmoBankPublishedAcquisitionError> {
        self.binding.try_revalidate()
    }

    /// Retains the exact model cursor with one explicitly selected deferred
    /// occurrence. This does not choose the native deferred route or enqueue a
    /// message.
    pub(super) fn begin_deferred(
        &mut self,
    ) -> Result<MainDmoBankDeferredRegistration, MainDmoBankCollectionRegistrationError> {
        if let Some(fault) = self.controller_fault {
            return Err(MainDmoBankCollectionRegistrationError::ControllerFaulted(
                fault,
            ));
        }
        let inner = self
            .adapter
            .begin_deferred(&self.binding.binding().prepared, MAIN_DMO_BANK_USERDATA_RAW)
            .map_err(MainDmoBankCollectionRegistrationError::Collection)?;
        let controller = self
            .runtime
            .begin_model_register_controller(&self.binding.binding().model_batch);
        Ok(MainDmoBankDeferredRegistration { inner, controller })
    }

    /// Executes one already-admitted deferred individual-Arena receiver inside
    /// the scoped current-winner visit after exact Setup-owner validation.
    pub(super) fn register_next_revalidated_deferred_arena(
        &mut self,
        ordinal: usize,
        registration: MainDmoBankDeferredRegistration,
    ) -> Result<MainDmoBankDeferredStep, MainDmoBankRevalidatedDeferredStepError> {
        let mut registration = Some(registration);
        let binding = &self.binding;
        let adapter = &mut self.adapter;
        let runtime = &mut self.runtime;
        let controller_fault = &mut self.controller_fault;
        let result = binding.try_with_revalidated_deferred_arena(ordinal, || {
            let registration = registration
                .take()
                .expect("deferred Arena effect visitor runs at most once");
            register_next_deferred_arena_effect(adapter, runtime, controller_fault, registration)
        });
        match result {
            Ok(result) => result.map_err(MainDmoBankRevalidatedDeferredStepError::Receiver),
            Err(source) => Err(
                MainDmoBankRevalidatedDeferredStepError::PublishedArenaRevalidation {
                    source,
                    registration: registration
                        .expect("Arena revalidation refusal precedes deferred effect ownership"),
                },
            ),
        }
    }

    /// Test-only effect entry for fixtures with no resident-registry authority.
    #[cfg(test)]
    pub(super) fn register_next_deferred_arena(
        &mut self,
        registration: MainDmoBankDeferredRegistration,
    ) -> Result<MainDmoBankDeferredStep, MainDmoBankDeferredStepError> {
        register_next_deferred_arena_effect(
            &mut self.adapter,
            &mut self.runtime,
            &mut self.controller_fault,
            registration,
        )
    }

    /// Finishes the exact model cursor after the final deferred Location call,
    /// then executes the activation-zero receiver's DMO-only pass.
    pub(super) fn finish_deferred(
        &mut self,
        registration: MainDmoBankDeferredRegistration,
    ) -> Result<MainDmoBankCollectionRegistration, MainDmoBankDeferredFinishError> {
        if let Some(fault) = self.controller_fault {
            return Err(MainDmoBankDeferredFinishError::ControllerFaulted {
                fault,
                registration,
            });
        }
        if let Err(source) = self.adapter.validate_deferred_finish(&registration.inner) {
            return Err(MainDmoBankDeferredFinishError::Validation {
                source,
                registration,
            });
        }
        let MainDmoBankDeferredRegistration { inner, controller } = registration;
        if let Err(source) = controller.finish() {
            self.controller_fault = Some(MainDmoBankCollectionControllerFault::RegistrationFinish);
            self.stranded_deferred = Some(inner);
            return Err(MainDmoBankDeferredFinishError::ControllerFinish(source));
        }
        let inner = self
            .adapter
            .finish_deferred(inner)
            .map_err(MainDmoBankDeferredFinishError::Collection)?;
        Ok(MainDmoBankCollectionRegistration { inner })
    }

    /// Runs the activation-nonzero receiver using one exact model cursor across
    /// the complete normal/Location prefix, then the restarted DMO pass.
    pub(super) fn register_active(
        &mut self,
    ) -> Result<MainDmoBankCollectionRegistration, MainDmoBankCollectionRegistrationError> {
        if let Some(fault) = self.controller_fault {
            return Err(MainDmoBankCollectionRegistrationError::ControllerFaulted(
                fault,
            ));
        }
        let mut controller = self
            .runtime
            .begin_model_register_controller(&self.binding.binding().model_batch);
        let continuation: SimulationCollectionActiveDmoContinuation = self
            .adapter
            .register_active_normal_location_with_dmo_controller(
                &self.binding.binding().prepared,
                MAIN_DMO_BANK_USERDATA_RAW,
                &mut self.runtime,
                &mut controller,
            )
            .map_err(|source| match source {
                SimulationCollectionDmoModelActivePrefixError::Collection(source) => {
                    MainDmoBankCollectionRegistrationError::Collection(source)
                }
                SimulationCollectionDmoModelActivePrefixError::Controller {
                    arena_index,
                    source,
                } => {
                    self.controller_fault =
                        Some(MainDmoBankCollectionControllerFault::RegistrationArena {
                            arena_index,
                        });
                    MainDmoBankCollectionRegistrationError::Controller {
                        arena_index,
                        source,
                    }
                }
            })?;
        if let Err(source) = controller.finish() {
            self.controller_fault = Some(MainDmoBankCollectionControllerFault::RegistrationFinish);
            self.stranded_active = Some(continuation);
            return Err(MainDmoBankCollectionRegistrationError::ControllerFinish(
                source,
            ));
        }
        let inner = self
            .adapter
            .finish_active_dmo_pass(continuation)
            .map_err(MainDmoBankCollectionRegistrationError::Collection)?;
        Ok(MainDmoBankCollectionRegistration { inner })
    }

    /// Runs the receiver's unconditional DMO-first deregistration, then uses a
    /// fresh exact model cursor across the restarted normal/Location pass.
    pub(super) fn deregister(
        &mut self,
        registration: MainDmoBankCollectionRegistration,
    ) -> Result<(), MainDmoBankCollectionDeregistrationError> {
        if let Some(fault) = self.controller_fault {
            return Err(
                MainDmoBankCollectionDeregistrationError::ControllerFaulted {
                    fault,
                    registration,
                },
            );
        }
        let continuation = self
            .adapter
            .begin_deregister_dmo_pass(registration.inner, MAIN_DMO_BANK_USERDATA_RAW)
            .map_err(MainDmoBankCollectionDeregistrationError::Collection)?;
        let mut controller = self
            .runtime
            .begin_model_deregister_controller(&self.binding.binding().model_batch);
        let completion = self
            .adapter
            .deregister_normal_location_with_dmo_controller(
                continuation,
                &mut self.runtime,
                &mut controller,
            )
            .map_err(|source| match source {
                SimulationCollectionDmoModelDeregistrationError::Collection(source) => {
                    MainDmoBankCollectionDeregistrationError::Collection(source)
                }
                SimulationCollectionDmoModelDeregistrationError::Controller {
                    arena_index,
                    source,
                } => {
                    self.controller_fault =
                        Some(MainDmoBankCollectionControllerFault::DeregistrationArena {
                            arena_index,
                        });
                    MainDmoBankCollectionDeregistrationError::Controller {
                        arena_index,
                        source,
                    }
                }
            })?;
        if let Err(source) = controller.finish() {
            self.controller_fault =
                Some(MainDmoBankCollectionControllerFault::DeregistrationFinish);
            self.stranded_deregistration = Some(completion);
            return Err(MainDmoBankCollectionDeregistrationError::ControllerFinish(
                source,
            ));
        }
        self.adapter
            .complete_deregistration(completion)
            .map_err(MainDmoBankCollectionDeregistrationError::Collection)?;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn live_registration_count(&self) -> usize {
        self.adapter.live_registration_count()
    }

    #[cfg(test)]
    pub(super) fn has_model(&self, model_id: u64) -> bool {
        self.runtime.has_model_for_test(model_id)
    }
}
