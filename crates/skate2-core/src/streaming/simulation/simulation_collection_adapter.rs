//! Active Simulation collection receiver composition.
//!
//! `SimThreadMsgMan::ReceiveRegCollectionMsg` (`0x8277e988`) walks the
//! first-null-bounded arena prefix forward, calling normal Simulation then
//! Location for every arena. It restarts the same prefix for the separate DMO
//! pass. `ReceiveDRegCollectionMsg` (`0x8277ebb8`) performs the complete DMO
//! pass first, restarts, then calls normal Simulation and Location per arena.
//!
//! This module covers the activation-nonzero receiver and the valid deferred
//! producer sequence that ends in an activation-zero DMO pass. Collection
//! lookup/assert behavior, production of native null slots, native update
//! cadence, FIFO callbacks, and hull acknowledgement remain separate.

use crate::{
    location_arena_adapter::{
        LocationArenaRegistration, LocationArenaRegistrationAdapter, LocationArenaRuntimeSource,
        LocationRegistrationError, PreparedLocationArena,
    },
    location_arena_plan::{LocationArenaPlan, LocationArenaPlanError},
    simulation_dmo_arena_adapter::{
        PreparedSimulationDmoArena, SimulationDmoArenaDispatch, SimulationDmoArenaRegistration,
        SimulationDmoArenaRegistrationAdapter, SimulationDmoLoadPolicy,
        SimulationDmoRegistrationError,
    },
    simulation_dmo_arena_plan::{SimulationDmoArenaPlan, SimulationDmoArenaPlanError},
    simulation_dmo_assembly_part_session::{
        SimulationDmoBankModelDeregisterController, SimulationDmoBankModelDeregisterPassError,
        SimulationDmoBankModelRegisterController, SimulationDmoBankModelRegisterPassError,
        SimulationDmoRegistrationRuntime,
    },
    simulation_normal_arena_adapter::{
        PreparedSimulationNormalArena, SimulationNormalArenaDeregistrationError,
        SimulationNormalArenaDispatch, SimulationNormalArenaDmoDeregistrationError,
        SimulationNormalArenaDmoRegistrationError, SimulationNormalArenaRegistration,
        SimulationNormalArenaRegistrationAdapter, SimulationNormalArenaRegistrationError,
        SimulationNormalEnvironmentDispatch,
    },
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
};
use std::{
    convert::Infallible,
    error::Error,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Weak},
};

#[derive(Debug)]
struct PreparedSimulationCollectionArena {
    normal: PreparedSimulationNormalArena,
    location: PreparedLocationArena,
    dmo: PreparedSimulationDmoArena,
}

#[derive(Debug)]
struct PreparedSimulationCollectionInner {
    collection_id: u64,
    arenas: Vec<PreparedSimulationCollectionArena>,
}

/// One fully prepared activation-nonzero Simulation collection prefix.
pub struct PreparedSimulationCollection {
    inner: Arc<PreparedSimulationCollectionInner>,
}

impl fmt::Debug for PreparedSimulationCollection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedSimulationCollection")
            .field("collection_id", &self.inner.collection_id)
            .field("arena_count", &self.inner.arenas.len())
            .finish_non_exhaustive()
    }
}

impl PreparedSimulationCollection {
    /// Builds all three arena views from the same normal-plan owners before
    /// any runtime effect.
    ///
    /// `arena_prefix` must contain exactly the non-null entries preceding the
    /// native collection vector's first null, or every entry if no null occurs.
    /// Duplicate and zero-operation arenas remain distinct entries.
    ///
    /// # Errors
    ///
    /// Returns a typed allocation, derived-plan, or arena-preparation error.
    pub fn try_new(
        collection_id: u64,
        arena_prefix: Vec<Arc<SimulationNormalArenaPlan>>,
    ) -> Result<Self, SimulationCollectionPrepareError> {
        let requested = arena_prefix.len();
        let mut arenas = Vec::new();
        arenas
            .try_reserve_exact(requested)
            .map_err(|_| SimulationCollectionPrepareError::ArenaStorageAllocation { requested })?;
        for (arena_index, normal_plan) in arena_prefix.into_iter().enumerate() {
            let location_plan = Arc::new(
                LocationArenaPlan::from_normal_plan(Arc::clone(&normal_plan)).map_err(
                    |source| SimulationCollectionPrepareError::LocationPlan {
                        arena_index,
                        source,
                    },
                )?,
            );
            let dmo_plan = Arc::new(
                SimulationDmoArenaPlan::from_normal_plan(Arc::clone(&normal_plan)).map_err(
                    |source| SimulationCollectionPrepareError::DmoPlan {
                        arena_index,
                        source,
                    },
                )?,
            );
            let normal = PreparedSimulationNormalArena::try_new(normal_plan).map_err(|source| {
                SimulationCollectionPrepareError::NormalArena {
                    arena_index,
                    source,
                }
            })?;
            let location = PreparedLocationArena::try_new(location_plan).map_err(|source| {
                SimulationCollectionPrepareError::LocationArena {
                    arena_index,
                    source,
                }
            })?;
            let dmo = PreparedSimulationDmoArena::try_new(dmo_plan).map_err(|source| {
                SimulationCollectionPrepareError::DmoArena {
                    arena_index,
                    source,
                }
            })?;
            arenas.push(PreparedSimulationCollectionArena {
                normal,
                location,
                dmo,
            });
        }
        Ok(Self {
            inner: Arc::new(PreparedSimulationCollectionInner {
                collection_id,
                arenas,
            }),
        })
    }

    #[must_use]
    pub fn collection_id(&self) -> u64 {
        self.inner.collection_id
    }

    #[must_use]
    pub fn arena_count(&self) -> usize {
        self.inner.arenas.len()
    }

    #[must_use]
    pub fn location_arena(&self, arena_index: usize) -> Option<&PreparedLocationArena> {
        Some(&self.inner.arenas.get(arena_index)?.location)
    }

    /// Returns the outer asset ID and decoded inner arena ID for one slot.
    #[must_use]
    pub fn arena_identity(&self, arena_index: usize) -> Option<(u64, u32)> {
        let plan = self.inner.arenas.get(arena_index)?.normal.plan();
        Some((
            plan.static_plan().asset().decoded.asset_id,
            plan.static_plan().asset().decoded.arena.id,
        ))
    }

    /// Returns the immutable normal plan selected for one prepared slot.
    ///
    /// This crate-private identity seam lets the deferred message router
    /// verify that a fresh global Arena lookup still names the early-bound
    /// safe-host continuation before any child effect begins.
    pub(crate) fn arena_plan_owner(
        &self,
        arena_index: usize,
    ) -> Option<Arc<SimulationNormalArenaPlan>> {
        Some(self.inner.arenas.get(arena_index)?.normal.plan_owner())
    }

    /// Counts prepared kind-31 DMO-data container operations across every
    /// retained arena without exposing the individual DMO plans. Contained
    /// 128-byte locator records are not separate operations.
    pub(crate) fn checked_dmo_locator_operation_count(&self) -> Option<usize> {
        self.inner.arenas.iter().try_fold(0usize, |count, arena| {
            count.checked_add(arena.dmo.operation_count())
        })
    }

    pub(crate) fn has_same_plan_sequence(&self, other: &Self) -> bool {
        self.collection_id() == other.collection_id()
            && self.arena_count() == other.arena_count()
            && self
                .inner
                .arenas
                .iter()
                .zip(&other.inner.arenas)
                .all(|(left, right)| std::ptr::eq(left.normal.plan(), right.normal.plan()))
    }
}

/// Whether a nested pass rejected before effects or faulted after entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationCollectionNestedFailureKind {
    RejectedBeforeEffects,
    ReachedFault,
    /// A pass unwound after entry; effects may already have occurred.
    Panicked,
    /// A collection invariant was missing a child token before dispatch.
    MissingAuthority,
}

/// A nested deregistration failure plus any still-valid child authority.
pub struct SimulationCollectionPassDeregistrationFailure<T> {
    kind: SimulationCollectionNestedFailureKind,
    registration: Option<T>,
}

impl<T> SimulationCollectionPassDeregistrationFailure<T> {
    #[must_use]
    pub fn before_effects(registration: T) -> Self {
        Self {
            kind: SimulationCollectionNestedFailureKind::RejectedBeforeEffects,
            registration: Some(registration),
        }
    }

    #[must_use]
    pub const fn reached_fault() -> Self {
        Self {
            kind: SimulationCollectionNestedFailureKind::ReachedFault,
            registration: None,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> SimulationCollectionNestedFailureKind {
        self.kind
    }

    #[must_use]
    pub fn into_registration(self) -> Option<T> {
        self.registration
    }
}

/// Arena-level normal Simulation port used by the collection receiver.
pub trait SimulationCollectionNormalPass {
    type Registration;

    /// # Errors
    ///
    /// Distinguishes rejection before this child call's effects from a reached
    /// child fault.
    fn register_normal(
        &mut self,
        prepared: &PreparedSimulationNormalArena,
        userdata_raw: u32,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind>;

    /// # Errors
    ///
    /// Returns any authority that remains valid because no child effect began.
    fn deregister_normal(
        &mut self,
        registration: Self::Registration,
        userdata_raw: u32,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>>;
}

impl<D, E> SimulationCollectionNormalPass for SimulationNormalArenaRegistrationAdapter<D, E>
where
    D: SimulationNormalArenaDispatch,
    E: SimulationNormalEnvironmentDispatch,
{
    type Registration = SimulationNormalArenaRegistration;

    fn register_normal(
        &mut self,
        prepared: &PreparedSimulationNormalArena,
        userdata_raw: u32,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
        self.register(prepared, userdata_raw)
            .map_err(|error| match error {
                SimulationNormalArenaRegistrationError::OperationFault(_)
                | SimulationNormalArenaRegistrationError::Waypoint { .. } => {
                    SimulationCollectionNestedFailureKind::ReachedFault
                }
                SimulationNormalArenaRegistrationError::AdapterFaulted(_)
                | SimulationNormalArenaRegistrationError::LiveStorageAllocation { .. }
                | SimulationNormalArenaRegistrationError::RegistrationIdExhausted => {
                    SimulationCollectionNestedFailureKind::RejectedBeforeEffects
                }
            })
    }

    fn deregister_normal(
        &mut self,
        registration: Self::Registration,
        userdata_raw: u32,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
        self.deregister(registration, userdata_raw)
            .map_err(|error| match error.into_registration() {
                Some(registration) => {
                    SimulationCollectionPassDeregistrationFailure::before_effects(registration)
                }
                None => SimulationCollectionPassDeregistrationFailure::reached_fault(),
            })
    }
}

/// Arena-level Location port used by the collection receiver.
pub trait SimulationCollectionLocationPass {
    type Registration;

    /// # Errors
    ///
    /// Distinguishes rejection before this child call's effects from a reached
    /// child fault.
    fn register_location(
        &mut self,
        prepared: &PreparedLocationArena,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind>;

    /// # Errors
    ///
    /// Returns any authority that remains valid because no child effect began.
    fn deregister_location(
        &mut self,
        registration: Self::Registration,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>>;
}

impl<R> SimulationCollectionLocationPass for LocationArenaRegistrationAdapter<R>
where
    R: LocationArenaRuntimeSource,
{
    type Registration = LocationArenaRegistration;

    fn register_location(
        &mut self,
        prepared: &PreparedLocationArena,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
        self.register(prepared).map_err(|error| match error {
            LocationRegistrationError::RuntimeFault(_) => {
                SimulationCollectionNestedFailureKind::ReachedFault
            }
            LocationRegistrationError::AdapterFaulted(_)
            | LocationRegistrationError::LiveStorageAllocation { .. }
            | LocationRegistrationError::SourceStorageAllocation { .. }
            | LocationRegistrationError::RegistrationIdExhausted => {
                SimulationCollectionNestedFailureKind::RejectedBeforeEffects
            }
        })
    }

    fn deregister_location(
        &mut self,
        registration: Self::Registration,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
        self.deregister(registration)
            .map_err(|error| match error.into_registration() {
                Some(registration) => {
                    SimulationCollectionPassDeregistrationFailure::before_effects(registration)
                }
                None => SimulationCollectionPassDeregistrationFailure::reached_fault(),
            })
    }
}

/// Arena-level separate DMO port used by the collection receiver.
pub trait SimulationCollectionDmoPass {
    type Registration;

    /// # Errors
    ///
    /// Distinguishes rejection before this child call's effects from a reached
    /// child fault.
    fn register_dmo(
        &mut self,
        prepared: &PreparedSimulationDmoArena,
        userdata_raw: u32,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind>;

    /// # Errors
    ///
    /// Returns any authority that remains valid because no child effect began.
    fn deregister_dmo(
        &mut self,
        registration: Self::Registration,
        userdata_raw: u32,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>>;
}

impl<P, D> SimulationCollectionDmoPass for SimulationDmoArenaRegistrationAdapter<P, D>
where
    P: SimulationDmoLoadPolicy,
    D: SimulationDmoArenaDispatch,
{
    type Registration = SimulationDmoArenaRegistration;

    fn register_dmo(
        &mut self,
        prepared: &PreparedSimulationDmoArena,
        userdata_raw: u32,
    ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
        self.register(prepared, userdata_raw)
            .map_err(|error| match error {
                SimulationDmoRegistrationError::DispatchPanicked(_) => {
                    SimulationCollectionNestedFailureKind::ReachedFault
                }
                SimulationDmoRegistrationError::AdapterFaulted(_)
                | SimulationDmoRegistrationError::LiveStorageAllocation { .. }
                | SimulationDmoRegistrationError::EffectStorageAllocation { .. }
                | SimulationDmoRegistrationError::RegistrationIdExhausted => {
                    SimulationCollectionNestedFailureKind::RejectedBeforeEffects
                }
            })
    }

    fn deregister_dmo(
        &mut self,
        registration: Self::Registration,
        userdata_raw: u32,
    ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
        self.deregister(registration, userdata_raw)
            .map_err(|error| match error.into_registration() {
                Some(registration) => {
                    SimulationCollectionPassDeregistrationFailure::before_effects(registration)
                }
                None => SimulationCollectionPassDeregistrationFailure::reached_fault(),
            })
    }
}

struct LiveSimulationCollection<N, L, D> {
    id: u64,
    prepared: Arc<PreparedSimulationCollectionInner>,
    registration_userdata_raw: u32,
    normal: Vec<Option<N>>,
    location: Vec<Option<L>>,
    dmo: Vec<Option<D>>,
    stage: SimulationCollectionRegistrationStage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationCollectionRegistrationStage {
    ActiveInProgress,
    Deferred { next_arena: usize },
    Complete,
    Deregistering,
}

enum SimulationCollectionNormalRegistrationCallError<C> {
    Nested(SimulationCollectionNestedFailureKind),
    Controlled(C),
}

#[derive(Debug)]
enum SimulationCollectionActivePrefixError<C> {
    Collection(SimulationCollectionRegistrationError),
    Controlled { arena_index: usize, source: C },
}

pub(crate) struct SimulationCollectionActiveDmoContinuation {
    registration: SimulationCollectionRegistration,
    userdata_raw: u32,
}

pub(crate) struct SimulationCollectionDeregistrationNormalContinuation {
    domain: Weak<()>,
    id: u64,
    userdata_raw: u32,
}

pub(crate) struct SimulationCollectionDeregistrationCompletion {
    domain: Weak<()>,
    id: u64,
}

enum SimulationCollectionDeferredStepRunError<C> {
    Collection(SimulationCollectionDeferredStepError),
    Controlled { arena_index: usize, source: C },
}

enum SimulationCollectionNormalDeregistrationCallError<T, C> {
    Nested(SimulationCollectionPassDeregistrationFailure<T>),
    Controlled { registration: Option<T>, source: C },
}

#[derive(Debug)]
enum SimulationCollectionNormalDeregistrationRunError<C> {
    Collection(SimulationCollectionDeregistrationError),
    Controlled { arena_index: usize, source: C },
}

#[derive(Debug)]
pub(crate) enum SimulationCollectionDmoModelActivePrefixError {
    Collection(SimulationCollectionRegistrationError),
    Controller {
        arena_index: usize,
        source: SimulationDmoBankModelRegisterPassError,
    },
}

#[derive(Debug)]
pub(crate) enum SimulationCollectionDmoModelDeferredStepError {
    Collection(SimulationCollectionDeferredStepError),
    Controller {
        arena_index: usize,
        source: SimulationDmoBankModelRegisterPassError,
    },
}

#[derive(Debug)]
pub(crate) enum SimulationCollectionDmoModelDeregistrationError {
    Collection(SimulationCollectionDeregistrationError),
    Controller {
        arena_index: usize,
        source: SimulationDmoBankModelDeregisterPassError,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationCollectionDeferredFinishValidationError {
    ForeignDomain,
    AdapterFaulted(SimulationCollectionFault),
    UnknownRegistration,
    AlreadyComplete,
    Incomplete { completed: usize, total: usize },
}

/// Exact active-collection pass phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationCollectionPhase {
    Registration,
    Deregistration,
}

/// Exact nested pass reached within the collection receiver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationCollectionPass {
    Normal,
    Location,
    Dmo,
}

/// Sticky safe-host fault at one nested arena-pass boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulationCollectionFault {
    phase: SimulationCollectionPhase,
    pass: SimulationCollectionPass,
    arena_index: usize,
    kind: SimulationCollectionNestedFailureKind,
}

impl SimulationCollectionFault {
    #[must_use]
    pub const fn phase(self) -> SimulationCollectionPhase {
        self.phase
    }

    #[must_use]
    pub const fn pass(self) -> SimulationCollectionPass {
        self.pass
    }

    #[must_use]
    pub const fn arena_index(self) -> usize {
        self.arena_index
    }

    #[must_use]
    pub const fn kind(self) -> SimulationCollectionNestedFailureKind {
        self.kind
    }
}

/// Composes the three proven arena adapters into the active collection order.
///
/// Nested adapters precede the outer owner ledger so collaborators are dropped
/// before the final collection owner. Neither this adapter nor its tokens runs
/// hidden cleanup from `Drop`.
pub struct SimulationCollectionRegistrationAdapter<N, L, D>
where
    N: SimulationCollectionNormalPass,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    normal: N,
    location: L,
    dmo: D,
    live: Vec<LiveSimulationCollection<N::Registration, L::Registration, D::Registration>>,
    domain: Arc<()>,
    next_registration_id: u64,
    fault: Option<SimulationCollectionFault>,
}

impl<N, L, D> SimulationCollectionRegistrationAdapter<N, L, D>
where
    N: SimulationCollectionNormalPass,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    #[must_use]
    pub fn new(normal: N, location: L, dmo: D) -> Self {
        Self {
            normal,
            location,
            dmo,
            live: Vec::new(),
            domain: Arc::new(()),
            next_registration_id: 0,
            fault: None,
        }
    }

    /// Retains one collection for the native deferred-registration route.
    /// This safe-host preparation step has no nested manager effects.
    ///
    /// The producer must call [`Self::register_next_deferred_arena`] until the
    /// prefix is exhausted, then call [`Self::finish_deferred`] to execute the
    /// activation-zero collection receiver's complete DMO pass.
    ///
    /// # Errors
    ///
    /// Returns before effects if this domain is faulted, storage cannot be
    /// retained, or the domain's monotonically increasing ID is exhausted.
    pub fn begin_deferred(
        &mut self,
        prepared: &PreparedSimulationCollection,
        userdata_raw: u32,
    ) -> Result<DeferredSimulationCollectionRegistration, SimulationCollectionRegistrationError>
    {
        if let Some(fault) = self.fault {
            return Err(SimulationCollectionRegistrationError::AdapterFaulted(fault));
        }
        self.live.try_reserve(1).map_err(|_| {
            SimulationCollectionRegistrationError::LiveStorageAllocation {
                requested: self.live.len().saturating_add(1),
            }
        })?;
        let arena_count = prepared.inner.arenas.len();
        let normal = empty_token_slots(arena_count).map_err(|()| {
            SimulationCollectionRegistrationError::TokenStorageAllocation { arena_count }
        })?;
        let location = empty_token_slots(arena_count).map_err(|()| {
            SimulationCollectionRegistrationError::TokenStorageAllocation { arena_count }
        })?;
        let dmo = empty_token_slots(arena_count).map_err(|()| {
            SimulationCollectionRegistrationError::TokenStorageAllocation { arena_count }
        })?;
        let id = self.next_registration_id;
        self.next_registration_id = id
            .checked_add(1)
            .ok_or(SimulationCollectionRegistrationError::RegistrationIdExhausted)?;
        self.live.push(LiveSimulationCollection {
            id,
            prepared: Arc::clone(&prepared.inner),
            registration_userdata_raw: userdata_raw,
            normal,
            location,
            dmo,
            stage: SimulationCollectionRegistrationStage::Deferred { next_arena: 0 },
        });
        Ok(DeferredSimulationCollectionRegistration {
            domain: Arc::downgrade(&self.domain),
            id,
        })
    }

    /// Executes the next deferred individual-arena message: normal Simulation
    /// followed by Location for the same authored arena.
    ///
    /// Returns `PrefixComplete` when that call completed the normal/Location
    /// prefix. Every step uses the userdata captured by [`Self::begin_deferred`]
    /// because the native producer copies that one stored word into every
    /// individual-arena message and the final collection message.
    ///
    /// # Errors
    ///
    /// Identity/stage errors occur before effects. Any entered child failure
    /// latches the reached prefix and permanently faults this adapter.
    pub fn register_next_deferred_arena(
        &mut self,
        registration: DeferredSimulationCollectionRegistration,
    ) -> Result<SimulationCollectionDeferredStep, SimulationCollectionDeferredStepError> {
        self.register_next_deferred_arena_with::<Infallible, _>(
            registration,
            |normal, prepared, userdata_raw| {
                normal
                    .register_normal(prepared, userdata_raw)
                    .map_err(SimulationCollectionNormalRegistrationCallError::Nested)
            },
        )
        .map_err(|source| match source {
            SimulationCollectionDeferredStepRunError::Collection(source) => source,
            SimulationCollectionDeferredStepRunError::Controlled { source, .. } => match source {},
        })
    }

    #[allow(clippy::too_many_lines)] // Keeps one deferred arena's exact pair visible.
    fn register_next_deferred_arena_with<C, F>(
        &mut self,
        registration: DeferredSimulationCollectionRegistration,
        mut register_normal: F,
    ) -> Result<SimulationCollectionDeferredStep, SimulationCollectionDeferredStepRunError<C>>
    where
        F: FnMut(
            &mut N,
            &PreparedSimulationNormalArena,
            u32,
        )
            -> Result<N::Registration, SimulationCollectionNormalRegistrationCallError<C>>,
    {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(SimulationCollectionDeferredStepRunError::Collection(
                SimulationCollectionDeferredStepError::ForeignDomain(registration),
            ));
        }
        if let Some(fault) = self.fault {
            return Err(SimulationCollectionDeferredStepRunError::Collection(
                SimulationCollectionDeferredStepError::AdapterFaulted {
                    fault,
                    registration,
                },
            ));
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            return Err(SimulationCollectionDeferredStepRunError::Collection(
                SimulationCollectionDeferredStepError::UnknownRegistration(registration),
            ));
        };
        let next_arena = match self.live[live_index].stage {
            SimulationCollectionRegistrationStage::Deferred { next_arena } => next_arena,
            SimulationCollectionRegistrationStage::ActiveInProgress
            | SimulationCollectionRegistrationStage::Complete
            | SimulationCollectionRegistrationStage::Deregistering => {
                return Err(SimulationCollectionDeferredStepRunError::Collection(
                    SimulationCollectionDeferredStepError::AlreadyComplete(registration),
                ));
            }
        };
        let arena_count = self.live[live_index].prepared.arenas.len();
        if next_arena == arena_count {
            return Err(SimulationCollectionDeferredStepRunError::Collection(
                SimulationCollectionDeferredStepError::PrefixComplete(registration),
            ));
        }
        let prepared = Arc::clone(&self.live[live_index].prepared);
        let userdata_raw = self.live[live_index].registration_userdata_raw;
        let arena = &prepared.arenas[next_arena];

        match catch_unwind(AssertUnwindSafe(|| {
            register_normal(&mut self.normal, &arena.normal, userdata_raw)
        })) {
            Ok(Ok(token)) => self.live[live_index].normal[next_arena] = Some(token),
            Ok(Err(SimulationCollectionNormalRegistrationCallError::Nested(kind))) => {
                let fault = self.latch_fault(
                    SimulationCollectionPhase::Registration,
                    SimulationCollectionPass::Normal,
                    next_arena,
                    kind,
                );
                return Err(SimulationCollectionDeferredStepRunError::Collection(
                    SimulationCollectionDeferredStepError::NestedFault(fault),
                ));
            }
            Ok(Err(SimulationCollectionNormalRegistrationCallError::Controlled(source))) => {
                self.latch_fault(
                    SimulationCollectionPhase::Registration,
                    SimulationCollectionPass::Normal,
                    next_arena,
                    SimulationCollectionNestedFailureKind::ReachedFault,
                );
                return Err(SimulationCollectionDeferredStepRunError::Controlled {
                    arena_index: next_arena,
                    source,
                });
            }
            Err(_) => {
                let fault = self.latch_fault(
                    SimulationCollectionPhase::Registration,
                    SimulationCollectionPass::Normal,
                    next_arena,
                    SimulationCollectionNestedFailureKind::Panicked,
                );
                return Err(SimulationCollectionDeferredStepRunError::Collection(
                    SimulationCollectionDeferredStepError::NestedFault(fault),
                ));
            }
        }
        match catch_unwind(AssertUnwindSafe(|| {
            self.location.register_location(&arena.location)
        })) {
            Ok(Ok(token)) => self.live[live_index].location[next_arena] = Some(token),
            Ok(Err(kind)) => {
                let fault = self.latch_fault(
                    SimulationCollectionPhase::Registration,
                    SimulationCollectionPass::Location,
                    next_arena,
                    kind,
                );
                return Err(SimulationCollectionDeferredStepRunError::Collection(
                    SimulationCollectionDeferredStepError::NestedFault(fault),
                ));
            }
            Err(_) => {
                let fault = self.latch_fault(
                    SimulationCollectionPhase::Registration,
                    SimulationCollectionPass::Location,
                    next_arena,
                    SimulationCollectionNestedFailureKind::Panicked,
                );
                return Err(SimulationCollectionDeferredStepRunError::Collection(
                    SimulationCollectionDeferredStepError::NestedFault(fault),
                ));
            }
        }

        let completed = next_arena + 1 == arena_count;
        self.live[live_index].stage = SimulationCollectionRegistrationStage::Deferred {
            next_arena: next_arena + 1,
        };
        if completed {
            Ok(SimulationCollectionDeferredStep::PrefixComplete(
                registration,
            ))
        } else {
            Ok(SimulationCollectionDeferredStep::Pending(registration))
        }
    }

    /// Executes the activation-zero collection receiver's DMO-only pass after
    /// all deferred individual-arena messages have completed.
    ///
    /// The returned authority owns the joined normal, Location, and DMO child
    /// registrations and is accepted by the existing full deregistration path.
    /// A native callback remains a separate FIFO message and is deliberately
    /// not emitted by this method.
    ///
    /// # Errors
    ///
    /// Foreign, unknown, and incomplete authorities are returned unchanged
    /// and cause no effects. An entered DMO failure consumes the deferred
    /// authority and permanently latches the exact reached prefix.
    pub fn finish_deferred(
        &mut self,
        registration: DeferredSimulationCollectionRegistration,
    ) -> Result<SimulationCollectionRegistration, SimulationCollectionDeferredFinishError> {
        match self.validate_deferred_finish(&registration) {
            Ok(()) => {}
            Err(source) => {
                return Err(match source {
                    SimulationCollectionDeferredFinishValidationError::ForeignDomain => {
                        SimulationCollectionDeferredFinishError::ForeignDomain(registration)
                    }
                    SimulationCollectionDeferredFinishValidationError::AdapterFaulted(fault) => {
                        SimulationCollectionDeferredFinishError::AdapterFaulted {
                            fault,
                            registration,
                        }
                    }
                    SimulationCollectionDeferredFinishValidationError::UnknownRegistration => {
                        SimulationCollectionDeferredFinishError::UnknownRegistration(registration)
                    }
                    SimulationCollectionDeferredFinishValidationError::AlreadyComplete => {
                        SimulationCollectionDeferredFinishError::AlreadyComplete(registration)
                    }
                    SimulationCollectionDeferredFinishValidationError::Incomplete {
                        completed,
                        total,
                    } => SimulationCollectionDeferredFinishError::Incomplete {
                        completed,
                        total,
                        registration,
                    },
                });
            }
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            return Err(SimulationCollectionDeferredFinishError::UnknownRegistration(registration));
        };
        let prepared = Arc::clone(&self.live[live_index].prepared);
        let userdata_raw = self.live[live_index].registration_userdata_raw;

        for (arena_index, arena) in prepared.arenas.iter().enumerate() {
            match catch_unwind(AssertUnwindSafe(|| {
                self.dmo.register_dmo(&arena.dmo, userdata_raw)
            })) {
                Ok(Ok(token)) => self.live[live_index].dmo[arena_index] = Some(token),
                Ok(Err(kind)) => {
                    return Err(SimulationCollectionDeferredFinishError::NestedFault(
                        self.latch_fault(
                            SimulationCollectionPhase::Registration,
                            SimulationCollectionPass::Dmo,
                            arena_index,
                            kind,
                        ),
                    ));
                }
                Err(_) => {
                    return Err(SimulationCollectionDeferredFinishError::NestedFault(
                        self.latch_fault(
                            SimulationCollectionPhase::Registration,
                            SimulationCollectionPass::Dmo,
                            arena_index,
                            SimulationCollectionNestedFailureKind::Panicked,
                        ),
                    ));
                }
            }
        }
        self.live[live_index].stage = SimulationCollectionRegistrationStage::Complete;
        Ok(SimulationCollectionRegistration {
            domain: registration.domain,
            id: registration.id,
        })
    }

    pub(crate) fn validate_deferred_finish(
        &self,
        registration: &DeferredSimulationCollectionRegistration,
    ) -> Result<(), SimulationCollectionDeferredFinishValidationError> {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(SimulationCollectionDeferredFinishValidationError::ForeignDomain);
        }
        if let Some(fault) = self.fault {
            return Err(SimulationCollectionDeferredFinishValidationError::AdapterFaulted(fault));
        }
        let Some(live) = self.live.iter().find(|live| live.id == registration.id) else {
            return Err(SimulationCollectionDeferredFinishValidationError::UnknownRegistration);
        };
        let next_arena = match live.stage {
            SimulationCollectionRegistrationStage::Deferred { next_arena } => next_arena,
            SimulationCollectionRegistrationStage::ActiveInProgress
            | SimulationCollectionRegistrationStage::Complete
            | SimulationCollectionRegistrationStage::Deregistering => {
                return Err(SimulationCollectionDeferredFinishValidationError::AlreadyComplete);
            }
        };
        let total = live.prepared.arenas.len();
        if next_arena != total {
            return Err(
                SimulationCollectionDeferredFinishValidationError::Incomplete {
                    completed: next_arena,
                    total,
                },
            );
        }
        Ok(())
    }

    /// Executes the activation-nonzero receiver: normal then Location per
    /// arena, followed by a restarted complete DMO pass.
    ///
    /// # Errors
    ///
    /// Pre-effect outer allocation/ID failures have no nested effects. Any
    /// entered nested failure latches its exact collection prefix and performs
    /// no rollback.
    #[allow(clippy::too_many_lines)] // Keeps the two native passes visibly ordered.
    pub fn register_active(
        &mut self,
        prepared: &PreparedSimulationCollection,
        userdata_raw: u32,
    ) -> Result<SimulationCollectionRegistration, SimulationCollectionRegistrationError> {
        let continuation = self
            .register_active_normal_location_pass_with::<Infallible, _>(
                prepared,
                userdata_raw,
                |normal, prepared, userdata_raw| {
                    normal
                        .register_normal(prepared, userdata_raw)
                        .map_err(SimulationCollectionNormalRegistrationCallError::Nested)
                },
            )
            .map_err(|source| match source {
                SimulationCollectionActivePrefixError::Collection(source) => source,
                SimulationCollectionActivePrefixError::Controlled { source, .. } => match source {},
            })?;
        self.finish_active_dmo_pass(continuation)
    }

    #[allow(clippy::too_many_lines)] // Keeps the exact normal/Location prefix visible.
    fn register_active_normal_location_pass_with<C, F>(
        &mut self,
        prepared: &PreparedSimulationCollection,
        userdata_raw: u32,
        mut register_normal: F,
    ) -> Result<SimulationCollectionActiveDmoContinuation, SimulationCollectionActivePrefixError<C>>
    where
        F: FnMut(
            &mut N,
            &PreparedSimulationNormalArena,
            u32,
        )
            -> Result<N::Registration, SimulationCollectionNormalRegistrationCallError<C>>,
    {
        if let Some(fault) = self.fault {
            return Err(SimulationCollectionActivePrefixError::Collection(
                SimulationCollectionRegistrationError::AdapterFaulted(fault),
            ));
        }
        self.live.try_reserve(1).map_err(|_| {
            SimulationCollectionActivePrefixError::Collection(
                SimulationCollectionRegistrationError::LiveStorageAllocation {
                    requested: self.live.len().saturating_add(1),
                },
            )
        })?;
        let arena_count = prepared.inner.arenas.len();
        let normal = empty_token_slots(arena_count).map_err(|()| {
            SimulationCollectionActivePrefixError::Collection(
                SimulationCollectionRegistrationError::TokenStorageAllocation { arena_count },
            )
        })?;
        let location = empty_token_slots(arena_count).map_err(|()| {
            SimulationCollectionActivePrefixError::Collection(
                SimulationCollectionRegistrationError::TokenStorageAllocation { arena_count },
            )
        })?;
        let dmo = empty_token_slots(arena_count).map_err(|()| {
            SimulationCollectionActivePrefixError::Collection(
                SimulationCollectionRegistrationError::TokenStorageAllocation { arena_count },
            )
        })?;
        let id = self.next_registration_id;
        self.next_registration_id =
            id.checked_add(1)
                .ok_or(SimulationCollectionActivePrefixError::Collection(
                    SimulationCollectionRegistrationError::RegistrationIdExhausted,
                ))?;
        let registration = SimulationCollectionRegistration {
            domain: Arc::downgrade(&self.domain),
            id,
        };
        self.live.push(LiveSimulationCollection {
            id,
            prepared: Arc::clone(&prepared.inner),
            registration_userdata_raw: userdata_raw,
            normal,
            location,
            dmo,
            stage: SimulationCollectionRegistrationStage::ActiveInProgress,
        });
        let live_index = self.live.len() - 1;

        for (arena_index, arena) in prepared.inner.arenas.iter().enumerate() {
            match catch_unwind(AssertUnwindSafe(|| {
                register_normal(&mut self.normal, &arena.normal, userdata_raw)
            })) {
                Ok(Ok(token)) => self.live[live_index].normal[arena_index] = Some(token),
                Ok(Err(SimulationCollectionNormalRegistrationCallError::Nested(kind))) => {
                    let fault = self.latch_fault(
                        SimulationCollectionPhase::Registration,
                        SimulationCollectionPass::Normal,
                        arena_index,
                        kind,
                    );
                    return Err(SimulationCollectionActivePrefixError::Collection(
                        SimulationCollectionRegistrationError::NestedFault(fault),
                    ));
                }
                Ok(Err(SimulationCollectionNormalRegistrationCallError::Controlled(source))) => {
                    self.latch_fault(
                        SimulationCollectionPhase::Registration,
                        SimulationCollectionPass::Normal,
                        arena_index,
                        SimulationCollectionNestedFailureKind::ReachedFault,
                    );
                    return Err(SimulationCollectionActivePrefixError::Controlled {
                        arena_index,
                        source,
                    });
                }
                Err(_) => {
                    let fault = self.latch_fault(
                        SimulationCollectionPhase::Registration,
                        SimulationCollectionPass::Normal,
                        arena_index,
                        SimulationCollectionNestedFailureKind::Panicked,
                    );
                    return Err(SimulationCollectionActivePrefixError::Collection(
                        SimulationCollectionRegistrationError::NestedFault(fault),
                    ));
                }
            }
            match catch_unwind(AssertUnwindSafe(|| {
                self.location.register_location(&arena.location)
            })) {
                Ok(Ok(token)) => self.live[live_index].location[arena_index] = Some(token),
                Ok(Err(kind)) => {
                    let fault = self.latch_fault(
                        SimulationCollectionPhase::Registration,
                        SimulationCollectionPass::Location,
                        arena_index,
                        kind,
                    );
                    return Err(SimulationCollectionActivePrefixError::Collection(
                        SimulationCollectionRegistrationError::NestedFault(fault),
                    ));
                }
                Err(_) => {
                    let fault = self.latch_fault(
                        SimulationCollectionPhase::Registration,
                        SimulationCollectionPass::Location,
                        arena_index,
                        SimulationCollectionNestedFailureKind::Panicked,
                    );
                    return Err(SimulationCollectionActivePrefixError::Collection(
                        SimulationCollectionRegistrationError::NestedFault(fault),
                    ));
                }
            }
        }
        Ok(SimulationCollectionActiveDmoContinuation {
            registration,
            userdata_raw,
        })
    }

    pub(crate) fn finish_active_dmo_pass(
        &mut self,
        continuation: SimulationCollectionActiveDmoContinuation,
    ) -> Result<SimulationCollectionRegistration, SimulationCollectionRegistrationError> {
        let SimulationCollectionActiveDmoContinuation {
            registration,
            userdata_raw,
        } = continuation;
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            let fault = self.latch_fault(
                SimulationCollectionPhase::Registration,
                SimulationCollectionPass::Dmo,
                0,
                SimulationCollectionNestedFailureKind::MissingAuthority,
            );
            return Err(SimulationCollectionRegistrationError::NestedFault(fault));
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            let fault = self.latch_fault(
                SimulationCollectionPhase::Registration,
                SimulationCollectionPass::Dmo,
                0,
                SimulationCollectionNestedFailureKind::MissingAuthority,
            );
            return Err(SimulationCollectionRegistrationError::NestedFault(fault));
        };
        if self.live[live_index].stage != SimulationCollectionRegistrationStage::ActiveInProgress {
            let fault = self.latch_fault(
                SimulationCollectionPhase::Registration,
                SimulationCollectionPass::Dmo,
                0,
                SimulationCollectionNestedFailureKind::MissingAuthority,
            );
            return Err(SimulationCollectionRegistrationError::NestedFault(fault));
        }
        let prepared = Arc::clone(&self.live[live_index].prepared);
        for (arena_index, arena) in prepared.arenas.iter().enumerate() {
            match catch_unwind(AssertUnwindSafe(|| {
                self.dmo.register_dmo(&arena.dmo, userdata_raw)
            })) {
                Ok(Ok(token)) => self.live[live_index].dmo[arena_index] = Some(token),
                Ok(Err(kind)) => {
                    return Err(SimulationCollectionRegistrationError::NestedFault(
                        self.latch_fault(
                            SimulationCollectionPhase::Registration,
                            SimulationCollectionPass::Dmo,
                            arena_index,
                            kind,
                        ),
                    ));
                }
                Err(_) => {
                    return Err(SimulationCollectionRegistrationError::NestedFault(
                        self.latch_fault(
                            SimulationCollectionPhase::Registration,
                            SimulationCollectionPass::Dmo,
                            arena_index,
                            SimulationCollectionNestedFailureKind::Panicked,
                        ),
                    ));
                }
            }
        }
        self.live[live_index].stage = SimulationCollectionRegistrationStage::Complete;
        Ok(registration)
    }

    /// Executes the unconditional deregistration receiver: all DMO arenas,
    /// then normal and Location per arena using one fresh userdata word.
    ///
    /// # Errors
    ///
    /// Foreign, unknown, and pre-fault outer tokens are returned unchanged.
    /// An entered nested failure consumes outer authority, restores any child
    /// authority returned before effects, and permanently latches the prefix.
    #[allow(clippy::too_many_lines)] // Keeps the two native passes visibly ordered.
    pub fn deregister(
        &mut self,
        registration: SimulationCollectionRegistration,
        userdata_raw: u32,
    ) -> Result<(), SimulationCollectionDeregistrationError> {
        let continuation = self.begin_deregister_dmo_pass(registration, userdata_raw)?;
        let completion = self
            .deregister_normal_location_pass_with::<Infallible, _>(
                continuation,
                |normal, registration, userdata_raw| {
                    normal
                        .deregister_normal(registration, userdata_raw)
                        .map_err(SimulationCollectionNormalDeregistrationCallError::Nested)
                },
            )
            .map_err(|source| match source {
                SimulationCollectionNormalDeregistrationRunError::Collection(source) => source,
                SimulationCollectionNormalDeregistrationRunError::Controlled { source, .. } => {
                    match source {}
                }
            })?;
        self.complete_deregistration(completion)?;
        Ok(())
    }

    pub(crate) fn begin_deregister_dmo_pass(
        &mut self,
        registration: SimulationCollectionRegistration,
        userdata_raw: u32,
    ) -> Result<
        SimulationCollectionDeregistrationNormalContinuation,
        SimulationCollectionDeregistrationError,
    > {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(SimulationCollectionDeregistrationError::ForeignDomain(
                registration,
            ));
        }
        if let Some(fault) = self.fault {
            return Err(SimulationCollectionDeregistrationError::AdapterFaulted {
                fault,
                registration,
            });
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            return Err(SimulationCollectionDeregistrationError::UnknownRegistration(registration));
        };
        if self.live[live_index].stage != SimulationCollectionRegistrationStage::Complete {
            return Err(SimulationCollectionDeregistrationError::UnknownRegistration(registration));
        }
        self.live[live_index].stage = SimulationCollectionRegistrationStage::Deregistering;
        let prepared = Arc::clone(&self.live[live_index].prepared);

        for (arena_index, _) in prepared.arenas.iter().enumerate() {
            let Some(token) = self.live[live_index].dmo[arena_index].take() else {
                return Err(SimulationCollectionDeregistrationError::NestedFault(
                    self.latch_fault(
                        SimulationCollectionPhase::Deregistration,
                        SimulationCollectionPass::Dmo,
                        arena_index,
                        SimulationCollectionNestedFailureKind::MissingAuthority,
                    ),
                ));
            };
            match catch_unwind(AssertUnwindSafe(|| {
                self.dmo.deregister_dmo(token, userdata_raw)
            })) {
                Ok(Ok(())) => {}
                Ok(Err(failure)) => {
                    if let Some(token) = failure.registration {
                        self.live[live_index].dmo[arena_index] = Some(token);
                    }
                    return Err(SimulationCollectionDeregistrationError::NestedFault(
                        self.latch_fault(
                            SimulationCollectionPhase::Deregistration,
                            SimulationCollectionPass::Dmo,
                            arena_index,
                            failure.kind,
                        ),
                    ));
                }
                Err(_) => {
                    return Err(SimulationCollectionDeregistrationError::NestedFault(
                        self.latch_fault(
                            SimulationCollectionPhase::Deregistration,
                            SimulationCollectionPass::Dmo,
                            arena_index,
                            SimulationCollectionNestedFailureKind::Panicked,
                        ),
                    ));
                }
            }
        }
        Ok(SimulationCollectionDeregistrationNormalContinuation {
            domain: registration.domain,
            id: registration.id,
            userdata_raw,
        })
    }

    #[allow(clippy::too_many_lines)] // Keeps the restarted exact pair order visible.
    fn deregister_normal_location_pass_with<C, F>(
        &mut self,
        continuation: SimulationCollectionDeregistrationNormalContinuation,
        mut deregister_normal: F,
    ) -> Result<
        SimulationCollectionDeregistrationCompletion,
        SimulationCollectionNormalDeregistrationRunError<C>,
    >
    where
        F: FnMut(
            &mut N,
            N::Registration,
            u32,
        ) -> Result<
            (),
            SimulationCollectionNormalDeregistrationCallError<N::Registration, C>,
        >,
    {
        let SimulationCollectionDeregistrationNormalContinuation {
            domain,
            id,
            userdata_raw,
        } = continuation;
        if !Weak::ptr_eq(&domain, &Arc::downgrade(&self.domain)) {
            let fault = self.latch_fault(
                SimulationCollectionPhase::Deregistration,
                SimulationCollectionPass::Normal,
                0,
                SimulationCollectionNestedFailureKind::MissingAuthority,
            );
            return Err(
                SimulationCollectionNormalDeregistrationRunError::Collection(
                    SimulationCollectionDeregistrationError::NestedFault(fault),
                ),
            );
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == id) else {
            let fault = self.latch_fault(
                SimulationCollectionPhase::Deregistration,
                SimulationCollectionPass::Normal,
                0,
                SimulationCollectionNestedFailureKind::MissingAuthority,
            );
            return Err(
                SimulationCollectionNormalDeregistrationRunError::Collection(
                    SimulationCollectionDeregistrationError::NestedFault(fault),
                ),
            );
        };
        if self.live[live_index].stage != SimulationCollectionRegistrationStage::Deregistering {
            let fault = self.latch_fault(
                SimulationCollectionPhase::Deregistration,
                SimulationCollectionPass::Normal,
                0,
                SimulationCollectionNestedFailureKind::MissingAuthority,
            );
            return Err(
                SimulationCollectionNormalDeregistrationRunError::Collection(
                    SimulationCollectionDeregistrationError::NestedFault(fault),
                ),
            );
        }
        let prepared = Arc::clone(&self.live[live_index].prepared);
        for (arena_index, _) in prepared.arenas.iter().enumerate() {
            let Some(normal) = self.live[live_index].normal[arena_index].take() else {
                let fault = self.latch_fault(
                    SimulationCollectionPhase::Deregistration,
                    SimulationCollectionPass::Normal,
                    arena_index,
                    SimulationCollectionNestedFailureKind::MissingAuthority,
                );
                return Err(
                    SimulationCollectionNormalDeregistrationRunError::Collection(
                        SimulationCollectionDeregistrationError::NestedFault(fault),
                    ),
                );
            };
            match catch_unwind(AssertUnwindSafe(|| {
                deregister_normal(&mut self.normal, normal, userdata_raw)
            })) {
                Ok(Ok(())) => {}
                Ok(Err(SimulationCollectionNormalDeregistrationCallError::Nested(failure))) => {
                    if let Some(token) = failure.registration {
                        self.live[live_index].normal[arena_index] = Some(token);
                    }
                    let fault = self.latch_fault(
                        SimulationCollectionPhase::Deregistration,
                        SimulationCollectionPass::Normal,
                        arena_index,
                        failure.kind,
                    );
                    return Err(
                        SimulationCollectionNormalDeregistrationRunError::Collection(
                            SimulationCollectionDeregistrationError::NestedFault(fault),
                        ),
                    );
                }
                Ok(Err(SimulationCollectionNormalDeregistrationCallError::Controlled {
                    registration,
                    source,
                })) => {
                    if let Some(token) = registration {
                        self.live[live_index].normal[arena_index] = Some(token);
                    }
                    self.latch_fault(
                        SimulationCollectionPhase::Deregistration,
                        SimulationCollectionPass::Normal,
                        arena_index,
                        SimulationCollectionNestedFailureKind::ReachedFault,
                    );
                    return Err(
                        SimulationCollectionNormalDeregistrationRunError::Controlled {
                            arena_index,
                            source,
                        },
                    );
                }
                Err(_) => {
                    let fault = self.latch_fault(
                        SimulationCollectionPhase::Deregistration,
                        SimulationCollectionPass::Normal,
                        arena_index,
                        SimulationCollectionNestedFailureKind::Panicked,
                    );
                    return Err(
                        SimulationCollectionNormalDeregistrationRunError::Collection(
                            SimulationCollectionDeregistrationError::NestedFault(fault),
                        ),
                    );
                }
            }
            let Some(location) = self.live[live_index].location[arena_index].take() else {
                let fault = self.latch_fault(
                    SimulationCollectionPhase::Deregistration,
                    SimulationCollectionPass::Location,
                    arena_index,
                    SimulationCollectionNestedFailureKind::MissingAuthority,
                );
                return Err(
                    SimulationCollectionNormalDeregistrationRunError::Collection(
                        SimulationCollectionDeregistrationError::NestedFault(fault),
                    ),
                );
            };
            match catch_unwind(AssertUnwindSafe(|| {
                self.location.deregister_location(location)
            })) {
                Ok(Ok(())) => {}
                Ok(Err(failure)) => {
                    if let Some(token) = failure.registration {
                        self.live[live_index].location[arena_index] = Some(token);
                    }
                    let fault = self.latch_fault(
                        SimulationCollectionPhase::Deregistration,
                        SimulationCollectionPass::Location,
                        arena_index,
                        failure.kind,
                    );
                    return Err(
                        SimulationCollectionNormalDeregistrationRunError::Collection(
                            SimulationCollectionDeregistrationError::NestedFault(fault),
                        ),
                    );
                }
                Err(_) => {
                    let fault = self.latch_fault(
                        SimulationCollectionPhase::Deregistration,
                        SimulationCollectionPass::Location,
                        arena_index,
                        SimulationCollectionNestedFailureKind::Panicked,
                    );
                    return Err(
                        SimulationCollectionNormalDeregistrationRunError::Collection(
                            SimulationCollectionDeregistrationError::NestedFault(fault),
                        ),
                    );
                }
            }
        }
        Ok(SimulationCollectionDeregistrationCompletion { domain, id })
    }

    #[allow(
        clippy::needless_pass_by_value,
        reason = "completion is a consuming, non-copyable teardown capability"
    )]
    pub(crate) fn complete_deregistration(
        &mut self,
        completion: SimulationCollectionDeregistrationCompletion,
    ) -> Result<(), SimulationCollectionDeregistrationError> {
        let SimulationCollectionDeregistrationCompletion { domain, id } = completion;
        if !Weak::ptr_eq(&domain, &Arc::downgrade(&self.domain)) {
            let fault = self.latch_fault(
                SimulationCollectionPhase::Deregistration,
                SimulationCollectionPass::Normal,
                0,
                SimulationCollectionNestedFailureKind::MissingAuthority,
            );
            return Err(SimulationCollectionDeregistrationError::NestedFault(fault));
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == id) else {
            let fault = self.latch_fault(
                SimulationCollectionPhase::Deregistration,
                SimulationCollectionPass::Normal,
                0,
                SimulationCollectionNestedFailureKind::MissingAuthority,
            );
            return Err(SimulationCollectionDeregistrationError::NestedFault(fault));
        };
        let live = &self.live[live_index];
        if live.stage != SimulationCollectionRegistrationStage::Deregistering
            || live.normal.iter().any(Option::is_some)
            || live.location.iter().any(Option::is_some)
            || live.dmo.iter().any(Option::is_some)
        {
            let fault = self.latch_fault(
                SimulationCollectionPhase::Deregistration,
                SimulationCollectionPass::Normal,
                0,
                SimulationCollectionNestedFailureKind::MissingAuthority,
            );
            return Err(SimulationCollectionDeregistrationError::NestedFault(fault));
        }
        self.live.swap_remove(live_index);
        Ok(())
    }

    fn latch_fault(
        &mut self,
        phase: SimulationCollectionPhase,
        pass: SimulationCollectionPass,
        arena_index: usize,
        kind: SimulationCollectionNestedFailureKind,
    ) -> SimulationCollectionFault {
        let fault = SimulationCollectionFault {
            phase,
            pass,
            arena_index,
            kind,
        };
        self.fault.get_or_insert(fault);
        self.fault.unwrap_or(fault)
    }

    #[must_use]
    /// Number of retained outer occurrences, including incomplete deferred
    /// registrations. This is a quiescence metric, not an acknowledgement
    /// eligibility count.
    pub fn live_registration_count(&self) -> usize {
        self.live.len()
    }

    #[must_use]
    pub fn deferred_registration_count(&self) -> usize {
        self.live
            .iter()
            .filter(|live| {
                matches!(
                    live.stage,
                    SimulationCollectionRegistrationStage::Deferred { .. }
                )
            })
            .count()
    }

    #[must_use]
    /// Number of occurrences that returned complete collection authority and
    /// have not begun teardown. This is receiver completion only, not FIFO
    /// callback delivery, hull acknowledgement, renderer, or GPU completion.
    pub fn completed_registration_count(&self) -> usize {
        self.live
            .iter()
            .filter(|live| live.stage == SimulationCollectionRegistrationStage::Complete)
            .count()
    }

    #[must_use]
    pub const fn fault(&self) -> Option<SimulationCollectionFault> {
        self.fault
    }

    #[must_use]
    pub const fn normal(&self) -> &N {
        &self.normal
    }

    #[must_use]
    pub const fn location(&self) -> &L {
        &self.location
    }

    #[must_use]
    pub const fn dmo(&self) -> &D {
        &self.dmo
    }
}

impl<ND, NE, L, D>
    SimulationCollectionRegistrationAdapter<SimulationNormalArenaRegistrationAdapter<ND, NE>, L, D>
where
    ND: SimulationNormalArenaDispatch,
    NE: SimulationNormalEnvironmentDispatch,
    L: SimulationCollectionLocationPass,
    D: SimulationCollectionDmoPass,
{
    pub(crate) fn register_active_normal_location_with_dmo_controller(
        &mut self,
        prepared: &PreparedSimulationCollection,
        userdata_raw: u32,
        runtime: &mut SimulationDmoRegistrationRuntime,
        controller: &mut SimulationDmoBankModelRegisterController,
    ) -> Result<
        SimulationCollectionActiveDmoContinuation,
        SimulationCollectionDmoModelActivePrefixError,
    > {
        self.register_active_normal_location_pass_with(
            prepared,
            userdata_raw,
            |normal, prepared, userdata_raw| match normal.register_with_dmo_model_controller(
                prepared,
                userdata_raw,
                runtime,
                controller,
            ) {
                Ok(registration) => Ok(registration),
                Err(SimulationNormalArenaDmoRegistrationError::Arena(source)) => {
                    Err(SimulationCollectionNormalRegistrationCallError::Nested(
                        normal_registration_failure_kind(&source),
                    ))
                }
                Err(SimulationNormalArenaDmoRegistrationError::DmoController {
                    source, ..
                }) => Err(SimulationCollectionNormalRegistrationCallError::Controlled(
                    source,
                )),
            },
        )
        .map_err(|source| match source {
            SimulationCollectionActivePrefixError::Collection(source) => {
                SimulationCollectionDmoModelActivePrefixError::Collection(source)
            }
            SimulationCollectionActivePrefixError::Controlled {
                arena_index,
                source,
            } => SimulationCollectionDmoModelActivePrefixError::Controller {
                arena_index,
                source,
            },
        })
    }

    pub(crate) fn register_next_deferred_arena_with_dmo_controller(
        &mut self,
        registration: DeferredSimulationCollectionRegistration,
        runtime: &mut SimulationDmoRegistrationRuntime,
        controller: &mut SimulationDmoBankModelRegisterController,
    ) -> Result<SimulationCollectionDeferredStep, SimulationCollectionDmoModelDeferredStepError>
    {
        self.register_next_deferred_arena_with(registration, |normal, prepared, userdata_raw| {
            match normal.register_with_dmo_model_controller(
                prepared,
                userdata_raw,
                runtime,
                controller,
            ) {
                Ok(registration) => Ok(registration),
                Err(SimulationNormalArenaDmoRegistrationError::Arena(source)) => {
                    Err(SimulationCollectionNormalRegistrationCallError::Nested(
                        normal_registration_failure_kind(&source),
                    ))
                }
                Err(SimulationNormalArenaDmoRegistrationError::DmoController {
                    source, ..
                }) => Err(SimulationCollectionNormalRegistrationCallError::Controlled(
                    source,
                )),
            }
        })
        .map_err(|source| match source {
            SimulationCollectionDeferredStepRunError::Collection(source) => {
                SimulationCollectionDmoModelDeferredStepError::Collection(source)
            }
            SimulationCollectionDeferredStepRunError::Controlled {
                arena_index,
                source,
            } => SimulationCollectionDmoModelDeferredStepError::Controller {
                arena_index,
                source,
            },
        })
    }

    pub(crate) fn deregister_normal_location_with_dmo_controller(
        &mut self,
        continuation: SimulationCollectionDeregistrationNormalContinuation,
        runtime: &mut SimulationDmoRegistrationRuntime,
        controller: &mut SimulationDmoBankModelDeregisterController,
    ) -> Result<
        SimulationCollectionDeregistrationCompletion,
        SimulationCollectionDmoModelDeregistrationError,
    > {
        self.deregister_normal_location_pass_with(
            continuation,
            |normal, registration, userdata_raw| match normal.deregister_with_dmo_model_controller(
                registration,
                userdata_raw,
                runtime,
                controller,
            ) {
                Ok(()) => Ok(()),
                Err(SimulationNormalArenaDmoDeregistrationError::Arena(source)) => {
                    Err(SimulationCollectionNormalDeregistrationCallError::Nested(
                        normal_deregistration_failure(source),
                    ))
                }
                Err(SimulationNormalArenaDmoDeregistrationError::DmoController {
                    source, ..
                }) => Err(
                    SimulationCollectionNormalDeregistrationCallError::Controlled {
                        registration: None,
                        source,
                    },
                ),
            },
        )
        .map_err(|source| match source {
            SimulationCollectionNormalDeregistrationRunError::Collection(source) => {
                SimulationCollectionDmoModelDeregistrationError::Collection(source)
            }
            SimulationCollectionNormalDeregistrationRunError::Controlled {
                arena_index,
                source,
            } => SimulationCollectionDmoModelDeregistrationError::Controller {
                arena_index,
                source,
            },
        })
    }
}

fn normal_registration_failure_kind(
    source: &SimulationNormalArenaRegistrationError,
) -> SimulationCollectionNestedFailureKind {
    match source {
        SimulationNormalArenaRegistrationError::OperationFault(_)
        | SimulationNormalArenaRegistrationError::Waypoint { .. } => {
            SimulationCollectionNestedFailureKind::ReachedFault
        }
        SimulationNormalArenaRegistrationError::AdapterFaulted(_)
        | SimulationNormalArenaRegistrationError::LiveStorageAllocation { .. }
        | SimulationNormalArenaRegistrationError::RegistrationIdExhausted => {
            SimulationCollectionNestedFailureKind::RejectedBeforeEffects
        }
    }
}

fn normal_deregistration_failure(
    source: SimulationNormalArenaDeregistrationError,
) -> SimulationCollectionPassDeregistrationFailure<SimulationNormalArenaRegistration> {
    match source {
        SimulationNormalArenaDeregistrationError::OperationFault(_)
        | SimulationNormalArenaDeregistrationError::Waypoint { .. } => {
            SimulationCollectionPassDeregistrationFailure::reached_fault()
        }
        SimulationNormalArenaDeregistrationError::ForeignDomain(registration)
        | SimulationNormalArenaDeregistrationError::UnknownRegistration(registration)
        | SimulationNormalArenaDeregistrationError::OrphanStorageAllocation(registration)
        | SimulationNormalArenaDeregistrationError::AdapterFaulted { registration, .. } => {
            SimulationCollectionPassDeregistrationFailure::before_effects(registration)
        }
    }
}

fn empty_token_slots<T>(count: usize) -> Result<Vec<Option<T>>, ()> {
    let mut slots = Vec::new();
    slots.try_reserve_exact(count).map_err(|_| ())?;
    slots.resize_with(count, || None);
    Ok(slots)
}

/// Non-cloneable authority for one collection whose deferred normal/Location
/// prefix is still being assembled.
///
/// Dropping this capability is inert: it does not continue, cancel, tear down,
/// or acknowledge the retained prefix. The adapter keeps that occurrence and
/// its prepared owners until the adapter itself is dropped.
pub struct DeferredSimulationCollectionRegistration {
    domain: Weak<()>,
    id: u64,
}

impl fmt::Debug for DeferredSimulationCollectionRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeferredSimulationCollectionRegistration")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Reissued deferred authority after one successful individual-arena message.
pub enum SimulationCollectionDeferredStep {
    Pending(DeferredSimulationCollectionRegistration),
    PrefixComplete(DeferredSimulationCollectionRegistration),
}

impl SimulationCollectionDeferredStep {
    #[must_use]
    pub const fn is_prefix_complete(&self) -> bool {
        matches!(self, Self::PrefixComplete(_))
    }

    #[must_use]
    pub fn into_registration(self) -> DeferredSimulationCollectionRegistration {
        match self {
            Self::Pending(registration) | Self::PrefixComplete(registration) => registration,
        }
    }
}

impl fmt::Debug for SimulationCollectionDeferredStep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending(registration) => formatter
                .debug_tuple("Pending")
                .field(registration)
                .finish(),
            Self::PrefixComplete(registration) => formatter
                .debug_tuple("PrefixComplete")
                .field(registration)
                .finish(),
        }
    }
}

#[derive(Debug)]
pub enum SimulationCollectionDeferredStepError {
    ForeignDomain(DeferredSimulationCollectionRegistration),
    UnknownRegistration(DeferredSimulationCollectionRegistration),
    AdapterFaulted {
        fault: SimulationCollectionFault,
        registration: DeferredSimulationCollectionRegistration,
    },
    PrefixComplete(DeferredSimulationCollectionRegistration),
    AlreadyComplete(DeferredSimulationCollectionRegistration),
    NestedFault(SimulationCollectionFault),
}

impl SimulationCollectionDeferredStepError {
    #[must_use]
    pub fn into_registration(self) -> Option<DeferredSimulationCollectionRegistration> {
        match self {
            Self::ForeignDomain(registration)
            | Self::UnknownRegistration(registration)
            | Self::PrefixComplete(registration)
            | Self::AlreadyComplete(registration)
            | Self::AdapterFaulted { registration, .. } => Some(registration),
            Self::NestedFault(_) => None,
        }
    }
}

impl fmt::Display for SimulationCollectionDeferredStepError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignDomain(_) => formatter.write_str(
                "deferred registration belongs to another Simulation collection adapter",
            ),
            Self::UnknownRegistration(_) => {
                formatter.write_str("deferred Simulation collection registration is not live")
            }
            Self::AdapterFaulted { .. } => {
                formatter.write_str("Simulation collection adapter is faulted")
            }
            Self::PrefixComplete(_) => formatter
                .write_str("deferred normal/Location prefix is complete; finish the collection"),
            Self::AlreadyComplete(_) => {
                formatter.write_str("Simulation collection registration is already complete")
            }
            Self::NestedFault(fault) => write!(
                formatter,
                "deferred Simulation registration faulted at arena {} {:?}: {:?}",
                fault.arena_index, fault.pass, fault.kind
            ),
        }
    }
}

impl Error for SimulationCollectionDeferredStepError {}

#[derive(Debug)]
pub enum SimulationCollectionDeferredFinishError {
    ForeignDomain(DeferredSimulationCollectionRegistration),
    UnknownRegistration(DeferredSimulationCollectionRegistration),
    AdapterFaulted {
        fault: SimulationCollectionFault,
        registration: DeferredSimulationCollectionRegistration,
    },
    Incomplete {
        completed: usize,
        total: usize,
        registration: DeferredSimulationCollectionRegistration,
    },
    AlreadyComplete(DeferredSimulationCollectionRegistration),
    NestedFault(SimulationCollectionFault),
}

impl SimulationCollectionDeferredFinishError {
    #[must_use]
    pub fn into_registration(self) -> Option<DeferredSimulationCollectionRegistration> {
        match self {
            Self::ForeignDomain(registration)
            | Self::UnknownRegistration(registration)
            | Self::AlreadyComplete(registration)
            | Self::AdapterFaulted { registration, .. }
            | Self::Incomplete { registration, .. } => Some(registration),
            Self::NestedFault(_) => None,
        }
    }
}

impl fmt::Display for SimulationCollectionDeferredFinishError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignDomain(_) => formatter.write_str(
                "deferred registration belongs to another Simulation collection adapter",
            ),
            Self::UnknownRegistration(_) => {
                formatter.write_str("deferred Simulation collection registration is not live")
            }
            Self::AdapterFaulted { .. } => {
                formatter.write_str("Simulation collection adapter is faulted")
            }
            Self::Incomplete {
                completed, total, ..
            } => write!(
                formatter,
                "deferred Simulation prefix is incomplete ({completed}/{total} arenas)"
            ),
            Self::AlreadyComplete(_) => {
                formatter.write_str("Simulation collection registration is already complete")
            }
            Self::NestedFault(fault) => write!(
                formatter,
                "deferred Simulation DMO pass faulted at arena {}: {:?}",
                fault.arena_index, fault.kind
            ),
        }
    }
}

impl Error for SimulationCollectionDeferredFinishError {}

/// Non-cloneable authority for one completely registered active collection.
pub struct SimulationCollectionRegistration {
    domain: Weak<()>,
    id: u64,
}

impl fmt::Debug for SimulationCollectionRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationCollectionRegistration")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum SimulationCollectionPrepareError {
    ArenaStorageAllocation {
        requested: usize,
    },
    LocationPlan {
        arena_index: usize,
        source: LocationArenaPlanError,
    },
    DmoPlan {
        arena_index: usize,
        source: SimulationDmoArenaPlanError,
    },
    NormalArena {
        arena_index: usize,
        source: crate::simulation_normal_arena_adapter::SimulationNormalArenaPrepareError,
    },
    LocationArena {
        arena_index: usize,
        source: crate::location_arena_adapter::LocationArenaPrepareError,
    },
    DmoArena {
        arena_index: usize,
        source: crate::simulation_dmo_arena_adapter::SimulationDmoArenaPrepareError,
    },
}

impl fmt::Display for SimulationCollectionPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArenaStorageAllocation { requested } => {
                write!(formatter, "could not retain {requested} collection arenas")
            }
            Self::LocationPlan { arena_index, .. } => {
                write!(
                    formatter,
                    "collection arena {arena_index} Location plan is invalid"
                )
            }
            Self::DmoPlan { arena_index, .. } => {
                write!(
                    formatter,
                    "collection arena {arena_index} DMO plan is invalid"
                )
            }
            Self::NormalArena { arena_index, .. } => write!(
                formatter,
                "collection arena {arena_index} normal Simulation preparation failed"
            ),
            Self::LocationArena { arena_index, .. } => write!(
                formatter,
                "collection arena {arena_index} Location preparation failed"
            ),
            Self::DmoArena { arena_index, .. } => write!(
                formatter,
                "collection arena {arena_index} DMO preparation failed"
            ),
        }
    }
}

impl Error for SimulationCollectionPrepareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ArenaStorageAllocation { .. } => None,
            Self::LocationPlan { source, .. } => Some(source),
            Self::DmoPlan { source, .. } => Some(source),
            Self::NormalArena { source, .. } => Some(source),
            Self::LocationArena { source, .. } => Some(source),
            Self::DmoArena { source, .. } => Some(source),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationCollectionRegistrationError {
    AdapterFaulted(SimulationCollectionFault),
    LiveStorageAllocation { requested: usize },
    TokenStorageAllocation { arena_count: usize },
    RegistrationIdExhausted,
    NestedFault(SimulationCollectionFault),
}

impl fmt::Display for SimulationCollectionRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdapterFaulted(_) => {
                formatter.write_str("Simulation collection adapter is faulted")
            }
            Self::LiveStorageAllocation { requested } => write!(
                formatter,
                "could not retain Simulation collection registration {requested}"
            ),
            Self::TokenStorageAllocation { arena_count } => write!(
                formatter,
                "could not retain child tokens for {arena_count} collection arenas"
            ),
            Self::RegistrationIdExhausted => {
                formatter.write_str("Simulation collection registration IDs are exhausted")
            }
            Self::NestedFault(fault) => write!(
                formatter,
                "Simulation collection registration faulted at arena {} {:?}: {:?}",
                fault.arena_index, fault.pass, fault.kind
            ),
        }
    }
}

impl Error for SimulationCollectionRegistrationError {}

#[derive(Debug)]
pub enum SimulationCollectionDeregistrationError {
    ForeignDomain(SimulationCollectionRegistration),
    UnknownRegistration(SimulationCollectionRegistration),
    AdapterFaulted {
        fault: SimulationCollectionFault,
        registration: SimulationCollectionRegistration,
    },
    NestedFault(SimulationCollectionFault),
}

impl SimulationCollectionDeregistrationError {
    #[must_use]
    pub fn into_registration(self) -> Option<SimulationCollectionRegistration> {
        match self {
            Self::ForeignDomain(registration)
            | Self::UnknownRegistration(registration)
            | Self::AdapterFaulted { registration, .. } => Some(registration),
            Self::NestedFault(_) => None,
        }
    }
}

impl fmt::Display for SimulationCollectionDeregistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignDomain(_) => {
                formatter.write_str("registration belongs to another Simulation collection adapter")
            }
            Self::UnknownRegistration(_) => {
                formatter.write_str("Simulation collection registration is not live")
            }
            Self::AdapterFaulted { .. } => {
                formatter.write_str("Simulation collection adapter is faulted")
            }
            Self::NestedFault(fault) => write!(
                formatter,
                "Simulation collection deregistration faulted at arena {} {:?}: {:?}",
                fault.arena_index, fault.pass, fault.kind
            ),
        }
    }
}

impl Error for SimulationCollectionDeregistrationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        simulation_collision_asset::SimulationCollisionAsset,
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
        NormalReg(usize, u32),
        LocationReg(usize),
        DmoReg(usize, u32),
        DmoDreg(usize, u32),
        NormalDreg(usize, u32),
        LocationDreg(usize),
    }

    #[derive(Default)]
    struct MockPass {
        events: Arc<Mutex<Vec<Event>>>,
        registrations: usize,
        deregistrations: usize,
        fail_registration_at: Option<(usize, SimulationCollectionNestedFailureKind)>,
        fail_deregistration_at: Option<(usize, SimulationCollectionNestedFailureKind)>,
        panic_registration_at: Option<usize>,
        panic_deregistration_before_at: Option<usize>,
        panic_deregistration_after_at: Option<usize>,
    }

    struct Normal(MockPass);
    struct Location(MockPass);
    struct Dmo(MockPass);

    impl SimulationCollectionNormalPass for Normal {
        type Registration = usize;

        fn register_normal(
            &mut self,
            _: &PreparedSimulationNormalArena,
            userdata_raw: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            let call = self.0.registrations;
            self.0.registrations += 1;
            self.0
                .events
                .lock()
                .unwrap()
                .push(Event::NormalReg(call, userdata_raw));
            assert_ne!(
                self.0.panic_registration_at,
                Some(call),
                "normal register panic"
            );
            if let Some((at, kind)) = self.0.fail_registration_at
                && at == call
            {
                return Err(kind);
            }
            Ok(call)
        }

        fn deregister_normal(
            &mut self,
            registration: Self::Registration,
            userdata_raw: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            let call = self.0.deregistrations;
            self.0.deregistrations += 1;
            assert_ne!(
                self.0.panic_deregistration_before_at,
                Some(call),
                "normal deregister pre-effect panic"
            );
            self.0
                .events
                .lock()
                .unwrap()
                .push(Event::NormalDreg(registration, userdata_raw));
            assert_ne!(
                self.0.panic_deregistration_after_at,
                Some(call),
                "normal deregister reached panic"
            );
            if let Some((at, kind)) = self.0.fail_deregistration_at
                && at == call
            {
                return Err(match kind {
                    SimulationCollectionNestedFailureKind::RejectedBeforeEffects => {
                        SimulationCollectionPassDeregistrationFailure::before_effects(registration)
                    }
                    SimulationCollectionNestedFailureKind::ReachedFault
                    | SimulationCollectionNestedFailureKind::Panicked
                    | SimulationCollectionNestedFailureKind::MissingAuthority => {
                        SimulationCollectionPassDeregistrationFailure::reached_fault()
                    }
                });
            }
            Ok(())
        }
    }

    impl SimulationCollectionLocationPass for Location {
        type Registration = usize;

        fn register_location(
            &mut self,
            _: &PreparedLocationArena,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            let call = self.0.registrations;
            self.0.registrations += 1;
            self.0.events.lock().unwrap().push(Event::LocationReg(call));
            assert_ne!(
                self.0.panic_registration_at,
                Some(call),
                "Location register panic"
            );
            if let Some((at, kind)) = self.0.fail_registration_at
                && at == call
            {
                return Err(kind);
            }
            Ok(call)
        }

        fn deregister_location(
            &mut self,
            registration: Self::Registration,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            let call = self.0.deregistrations;
            self.0.deregistrations += 1;
            assert_ne!(
                self.0.panic_deregistration_before_at,
                Some(call),
                "Location deregister pre-effect panic"
            );
            self.0
                .events
                .lock()
                .unwrap()
                .push(Event::LocationDreg(registration));
            assert_ne!(
                self.0.panic_deregistration_after_at,
                Some(call),
                "Location deregister reached panic"
            );
            if let Some((at, kind)) = self.0.fail_deregistration_at
                && at == call
            {
                return Err(match kind {
                    SimulationCollectionNestedFailureKind::RejectedBeforeEffects => {
                        SimulationCollectionPassDeregistrationFailure::before_effects(registration)
                    }
                    SimulationCollectionNestedFailureKind::ReachedFault
                    | SimulationCollectionNestedFailureKind::Panicked
                    | SimulationCollectionNestedFailureKind::MissingAuthority => {
                        SimulationCollectionPassDeregistrationFailure::reached_fault()
                    }
                });
            }
            Ok(())
        }
    }

    impl SimulationCollectionDmoPass for Dmo {
        type Registration = usize;

        fn register_dmo(
            &mut self,
            _: &PreparedSimulationDmoArena,
            userdata_raw: u32,
        ) -> Result<Self::Registration, SimulationCollectionNestedFailureKind> {
            let call = self.0.registrations;
            self.0.registrations += 1;
            self.0
                .events
                .lock()
                .unwrap()
                .push(Event::DmoReg(call, userdata_raw));
            assert_ne!(
                self.0.panic_registration_at,
                Some(call),
                "DMO register panic"
            );
            if let Some((at, kind)) = self.0.fail_registration_at
                && at == call
            {
                return Err(kind);
            }
            Ok(call)
        }

        fn deregister_dmo(
            &mut self,
            registration: Self::Registration,
            userdata_raw: u32,
        ) -> Result<(), SimulationCollectionPassDeregistrationFailure<Self::Registration>> {
            let call = self.0.deregistrations;
            self.0.deregistrations += 1;
            assert_ne!(
                self.0.panic_deregistration_before_at,
                Some(call),
                "DMO deregister pre-effect panic"
            );
            self.0
                .events
                .lock()
                .unwrap()
                .push(Event::DmoDreg(registration, userdata_raw));
            assert_ne!(
                self.0.panic_deregistration_after_at,
                Some(call),
                "DMO deregister reached panic"
            );
            if let Some((at, kind)) = self.0.fail_deregistration_at
                && at == call
            {
                return Err(match kind {
                    SimulationCollectionNestedFailureKind::RejectedBeforeEffects => {
                        SimulationCollectionPassDeregistrationFailure::before_effects(registration)
                    }
                    SimulationCollectionNestedFailureKind::ReachedFault
                    | SimulationCollectionNestedFailureKind::Panicked
                    | SimulationCollectionNestedFailureKind::MissingAuthority => {
                        SimulationCollectionPassDeregistrationFailure::reached_fault()
                    }
                });
            }
            Ok(())
        }
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

    fn prepared(count: usize) -> PreparedSimulationCollection {
        let plan = empty_normal_plan(0x1111);
        PreparedSimulationCollection::try_new(
            0x2222,
            (0..count).map(|_| Arc::clone(&plan)).collect(),
        )
        .unwrap()
    }

    fn adapter(
        events: &Arc<Mutex<Vec<Event>>>,
    ) -> SimulationCollectionRegistrationAdapter<Normal, Location, Dmo> {
        let pass = || MockPass {
            events: Arc::clone(events),
            ..MockPass::default()
        };
        SimulationCollectionRegistrationAdapter::new(Normal(pass()), Location(pass()), Dmo(pass()))
    }

    #[test]
    fn exact_plan_sequence_compares_owner_identity_and_order() {
        let first = empty_normal_plan(0x1111);
        let second = empty_normal_plan(0x2222);
        let original = PreparedSimulationCollection::try_new(
            0x3333,
            vec![Arc::clone(&first), Arc::clone(&second)],
        )
        .unwrap();
        let same_owners = PreparedSimulationCollection::try_new(
            0x3333,
            vec![Arc::clone(&first), Arc::clone(&second)],
        )
        .unwrap();
        assert!(original.has_same_plan_sequence(&same_owners));

        let equal_value_replacement = PreparedSimulationCollection::try_new(
            0x3333,
            vec![empty_normal_plan(0x1111), Arc::clone(&second)],
        )
        .unwrap();
        assert!(!original.has_same_plan_sequence(&equal_value_replacement));

        let reversed = PreparedSimulationCollection::try_new(0x3333, vec![second, first]).unwrap();
        assert!(!original.has_same_plan_sequence(&reversed));
    }

    #[test]
    fn active_registration_and_deregistration_restart_in_exact_forward_order() {
        let prepared = prepared(3);
        assert_eq!(prepared.collection_id(), 0x2222);
        assert_eq!(prepared.arena_count(), 3);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let registration = adapter.register_active(&prepared, 0x1234).unwrap();
        adapter.deregister(registration, 0xabcd).unwrap();

        assert_eq!(
            *events.lock().unwrap(),
            [
                Event::NormalReg(0, 0x1234),
                Event::LocationReg(0),
                Event::NormalReg(1, 0x1234),
                Event::LocationReg(1),
                Event::NormalReg(2, 0x1234),
                Event::LocationReg(2),
                Event::DmoReg(0, 0x1234),
                Event::DmoReg(1, 0x1234),
                Event::DmoReg(2, 0x1234),
                Event::DmoDreg(0, 0xabcd),
                Event::DmoDreg(1, 0xabcd),
                Event::DmoDreg(2, 0xabcd),
                Event::NormalDreg(0, 0xabcd),
                Event::LocationDreg(0),
                Event::NormalDreg(1, 0xabcd),
                Event::LocationDreg(1),
                Event::NormalDreg(2, 0xabcd),
                Event::LocationDreg(2),
            ]
        );
        assert_eq!(adapter.live_registration_count(), 0);
        assert_eq!(adapter.fault(), None);
    }

    #[test]
    fn registration_fault_preserves_exact_prefix_and_stops_domain() {
        let prepared = prepared(3);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        adapter.location.0.fail_registration_at =
            Some((1, SimulationCollectionNestedFailureKind::ReachedFault));

        let error = adapter.register_active(&prepared, 7).unwrap_err();
        let SimulationCollectionRegistrationError::NestedFault(fault) = error else {
            panic!("wrong error: {error:?}");
        };
        assert_eq!(fault.phase(), SimulationCollectionPhase::Registration);
        assert_eq!(fault.pass(), SimulationCollectionPass::Location);
        assert_eq!(fault.arena_index(), 1);
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.completed_registration_count(), 0);
        assert_eq!(
            *events.lock().unwrap(),
            [
                Event::NormalReg(0, 7),
                Event::LocationReg(0),
                Event::NormalReg(1, 7),
                Event::LocationReg(1),
            ]
        );
        assert!(matches!(
            adapter.register_active(&prepared, 7),
            Err(SimulationCollectionRegistrationError::AdapterFaulted(_))
        ));
    }

    #[test]
    fn dmo_deregistration_fault_prevents_normal_pass() {
        let prepared = prepared(2);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let registration = adapter.register_active(&prepared, 1).unwrap();
        adapter.dmo.0.fail_deregistration_at =
            Some((1, SimulationCollectionNestedFailureKind::ReachedFault));
        let error = adapter.deregister(registration, 2).unwrap_err();
        let SimulationCollectionDeregistrationError::NestedFault(fault) = error else {
            panic!("wrong error: {error:?}");
        };
        assert_eq!(fault.pass(), SimulationCollectionPass::Dmo);
        assert_eq!(fault.arena_index(), 1);
        assert_eq!(
            &events.lock().unwrap()[6..],
            [Event::DmoDreg(0, 2), Event::DmoDreg(1, 2)]
        );
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.completed_registration_count(), 0);
    }

    #[test]
    fn registration_unwind_latches_exact_reached_prefix() {
        let prepared = prepared(3);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        adapter.location.0.panic_registration_at = Some(1);

        let error = adapter.register_active(&prepared, 7).unwrap_err();
        let SimulationCollectionRegistrationError::NestedFault(fault) = error else {
            panic!("wrong error: {error:?}");
        };
        assert_eq!(fault.phase(), SimulationCollectionPhase::Registration);
        assert_eq!(fault.pass(), SimulationCollectionPass::Location);
        assert_eq!(fault.arena_index(), 1);
        assert_eq!(
            fault.kind(),
            SimulationCollectionNestedFailureKind::Panicked
        );
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(
            *events.lock().unwrap(),
            [
                Event::NormalReg(0, 7),
                Event::LocationReg(0),
                Event::NormalReg(1, 7),
                Event::LocationReg(1),
            ]
        );
        let call_count = events.lock().unwrap().len();
        assert!(matches!(
            adapter.register_active(&prepared, 8),
            Err(SimulationCollectionRegistrationError::AdapterFaulted(_))
        ));
        assert_eq!(events.lock().unwrap().len(), call_count);
    }

    #[test]
    fn dmo_deregistration_pre_effect_unwind_consumes_authority_and_stops() {
        let prepared = prepared(2);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let registration = adapter.register_active(&prepared, 1).unwrap();
        events.lock().unwrap().clear();
        adapter.dmo.0.panic_deregistration_before_at = Some(1);

        let error = adapter.deregister(registration, 2).unwrap_err();
        let SimulationCollectionDeregistrationError::NestedFault(fault) = error else {
            panic!("wrong error: {error:?}");
        };
        assert_eq!(fault.pass(), SimulationCollectionPass::Dmo);
        assert_eq!(fault.arena_index(), 1);
        assert_eq!(
            fault.kind(),
            SimulationCollectionNestedFailureKind::Panicked
        );
        assert_eq!(*events.lock().unwrap(), [Event::DmoDreg(0, 2)]);
        assert_eq!(adapter.live_registration_count(), 1);
        assert!(adapter.live[0].dmo[0].is_none());
        assert!(adapter.live[0].dmo[1].is_none());
    }

    #[test]
    fn normal_deregistration_reached_unwind_prevents_same_arena_location() {
        let prepared = prepared(2);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let registration = adapter.register_active(&prepared, 1).unwrap();
        events.lock().unwrap().clear();
        adapter.normal.0.panic_deregistration_after_at = Some(0);

        let error = adapter.deregister(registration, 3).unwrap_err();
        let SimulationCollectionDeregistrationError::NestedFault(fault) = error else {
            panic!("wrong error: {error:?}");
        };
        assert_eq!(fault.pass(), SimulationCollectionPass::Normal);
        assert_eq!(fault.arena_index(), 0);
        assert_eq!(
            fault.kind(),
            SimulationCollectionNestedFailureKind::Panicked
        );
        assert_eq!(
            *events.lock().unwrap(),
            [
                Event::DmoDreg(0, 3),
                Event::DmoDreg(1, 3),
                Event::NormalDreg(0, 3),
            ]
        );
        assert!(adapter.live[0].normal[0].is_none());
        assert!(adapter.live[0].location[0].is_some());
    }

    #[test]
    fn pre_effect_child_rejection_restores_its_exact_token_slot() {
        let prepared = prepared(2);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let registration = adapter.register_active(&prepared, 1).unwrap();
        adapter.dmo.0.fail_deregistration_at = Some((
            0,
            SimulationCollectionNestedFailureKind::RejectedBeforeEffects,
        ));

        let error = adapter.deregister(registration, 2).unwrap_err();
        let SimulationCollectionDeregistrationError::NestedFault(fault) = error else {
            panic!("wrong error: {error:?}");
        };
        assert_eq!(
            fault.kind(),
            SimulationCollectionNestedFailureKind::RejectedBeforeEffects
        );
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.live[0].dmo[0], Some(0));
        assert_eq!(adapter.live[0].dmo[1], Some(1));
    }

    #[test]
    fn deferred_registration_joins_arena_messages_and_activation_zero_dmo_pass() {
        let prepared = prepared(3);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let deferred = adapter.begin_deferred(&prepared, 0x10).unwrap();

        let step = adapter.register_next_deferred_arena(deferred).unwrap();
        assert!(!step.is_prefix_complete());
        let step = adapter
            .register_next_deferred_arena(step.into_registration())
            .unwrap();
        assert!(!step.is_prefix_complete());
        let step = adapter
            .register_next_deferred_arena(step.into_registration())
            .unwrap();
        assert!(step.is_prefix_complete());
        let registration = adapter.finish_deferred(step.into_registration()).unwrap();
        adapter.deregister(registration, 0x50).unwrap();

        assert_eq!(
            *events.lock().unwrap(),
            [
                Event::NormalReg(0, 0x10),
                Event::LocationReg(0),
                Event::NormalReg(1, 0x10),
                Event::LocationReg(1),
                Event::NormalReg(2, 0x10),
                Event::LocationReg(2),
                Event::DmoReg(0, 0x10),
                Event::DmoReg(1, 0x10),
                Event::DmoReg(2, 0x10),
                Event::DmoDreg(0, 0x50),
                Event::DmoDreg(1, 0x50),
                Event::DmoDreg(2, 0x50),
                Event::NormalDreg(0, 0x50),
                Event::LocationDreg(0),
                Event::NormalDreg(1, 0x50),
                Event::LocationDreg(1),
                Event::NormalDreg(2, 0x50),
                Event::LocationDreg(2),
            ]
        );
        assert_eq!(adapter.live_registration_count(), 0);
        assert_eq!(adapter.fault(), None);
    }

    #[test]
    fn incomplete_deferred_finish_returns_authority_without_dmo_effects() {
        let prepared = prepared(2);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let deferred = adapter.begin_deferred(&prepared, 1).unwrap();
        let step = adapter.register_next_deferred_arena(deferred).unwrap();
        assert!(!step.is_prefix_complete());
        let deferred = step.into_registration();

        let error = adapter.finish_deferred(deferred).unwrap_err();
        assert!(matches!(
            error,
            SimulationCollectionDeferredFinishError::Incomplete {
                completed: 1,
                total: 2,
                ..
            }
        ));
        let deferred = error.into_registration().unwrap();
        assert_eq!(adapter.deferred_registration_count(), 1);
        assert_eq!(adapter.completed_registration_count(), 0);
        assert_eq!(
            *events.lock().unwrap(),
            [Event::NormalReg(0, 1), Event::LocationReg(0)]
        );

        let step = adapter.register_next_deferred_arena(deferred).unwrap();
        assert!(step.is_prefix_complete());
        let registration = adapter.finish_deferred(step.into_registration()).unwrap();
        adapter.deregister(registration, 5).unwrap();
        assert_eq!(adapter.live_registration_count(), 0);
    }

    #[test]
    fn deferred_empty_prefix_runs_no_nested_effects() {
        let prepared = prepared(0);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let deferred = adapter.begin_deferred(&prepared, 1).unwrap();

        let error = adapter.register_next_deferred_arena(deferred).unwrap_err();
        assert!(matches!(
            error,
            SimulationCollectionDeferredStepError::PrefixComplete(_)
        ));
        let deferred = error.into_registration().unwrap();
        let registration = adapter.finish_deferred(deferred).unwrap();
        adapter.deregister(registration, 3).unwrap();

        assert!(events.lock().unwrap().is_empty());
        assert_eq!(adapter.live_registration_count(), 0);
    }

    #[test]
    fn completed_prefix_rejects_an_extra_step_without_effects() {
        let prepared = prepared(1);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let deferred = adapter.begin_deferred(&prepared, 9).unwrap();
        let step = adapter.register_next_deferred_arena(deferred).unwrap();
        assert!(step.is_prefix_complete());
        let event_count = events.lock().unwrap().len();

        let error = adapter
            .register_next_deferred_arena(step.into_registration())
            .unwrap_err();
        assert!(matches!(
            error,
            SimulationCollectionDeferredStepError::PrefixComplete(_)
        ));
        assert_eq!(events.lock().unwrap().len(), event_count);
        let registration = adapter
            .finish_deferred(error.into_registration().unwrap())
            .unwrap();
        adapter.deregister(registration, 10).unwrap();
    }

    #[test]
    fn dropping_pending_authority_is_inert_and_leaves_retained_prefix() {
        let prepared = prepared(2);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let deferred = adapter.begin_deferred(&prepared, 11).unwrap();
        let step = adapter.register_next_deferred_arena(deferred).unwrap();
        let event_count = events.lock().unwrap().len();

        drop(step);
        drop(prepared);

        assert_eq!(events.lock().unwrap().len(), event_count);
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.deferred_registration_count(), 1);
        assert_eq!(adapter.completed_registration_count(), 0);
        assert_eq!(adapter.live[0].prepared.arenas.len(), 2);
    }

    #[test]
    fn repeated_collection_occurrences_have_distinct_deferred_authority() {
        let prepared = prepared(1);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut adapter = adapter(&events);
        let first = adapter.begin_deferred(&prepared, 1).unwrap();
        let second = adapter.begin_deferred(&prepared, 2).unwrap();
        assert_ne!(first.id, second.id);

        let first = adapter
            .register_next_deferred_arena(first)
            .unwrap()
            .into_registration();
        let second = adapter
            .register_next_deferred_arena(second)
            .unwrap()
            .into_registration();
        let first = adapter.finish_deferred(first).unwrap();
        let second = adapter.finish_deferred(second).unwrap();
        assert_eq!(adapter.completed_registration_count(), 2);
        adapter.deregister(first, 3).unwrap();
        adapter.deregister(second, 4).unwrap();
        assert_eq!(adapter.live_registration_count(), 0);
    }

    #[test]
    fn deferred_foreign_and_dmo_fault_paths_do_not_fabricate_completion() {
        let prepared = prepared(2);
        let first_events = Arc::new(Mutex::new(Vec::new()));
        let second_events = Arc::new(Mutex::new(Vec::new()));
        let mut first = adapter(&first_events);
        let mut second = adapter(&second_events);
        let deferred = first.begin_deferred(&prepared, 1).unwrap();

        let error = second.register_next_deferred_arena(deferred).unwrap_err();
        assert!(matches!(
            error,
            SimulationCollectionDeferredStepError::ForeignDomain(_)
        ));
        let deferred = error.into_registration().unwrap();
        let error = second.finish_deferred(deferred).unwrap_err();
        let deferred = error.into_registration().unwrap();
        assert!(second_events.lock().unwrap().is_empty());

        let step = first.register_next_deferred_arena(deferred).unwrap();
        assert!(!step.is_prefix_complete());
        let step = first
            .register_next_deferred_arena(step.into_registration())
            .unwrap();
        assert!(step.is_prefix_complete());
        first.dmo.0.panic_registration_at = Some(1);
        let error = first.finish_deferred(step.into_registration()).unwrap_err();
        let SimulationCollectionDeferredFinishError::NestedFault(fault) = error else {
            panic!("wrong error: {error:?}");
        };
        assert_eq!(fault.pass(), SimulationCollectionPass::Dmo);
        assert_eq!(fault.arena_index(), 1);
        assert_eq!(
            fault.kind(),
            SimulationCollectionNestedFailureKind::Panicked
        );
        assert_eq!(first.live_registration_count(), 1);
        assert!(matches!(
            first.begin_deferred(&prepared, 2),
            Err(SimulationCollectionRegistrationError::AdapterFaulted(_))
        ));
    }

    #[test]
    fn foreign_authority_and_empty_collection_are_inert() {
        let prepared = prepared(0);
        let first_events = Arc::new(Mutex::new(Vec::new()));
        let second_events = Arc::new(Mutex::new(Vec::new()));
        let mut first = adapter(&first_events);
        let mut second = adapter(&second_events);
        let registration = first.register_active(&prepared, 1).unwrap();
        let unknown = SimulationCollectionRegistration {
            domain: Arc::downgrade(&first.domain),
            id: 99,
        };
        let unknown = first.deregister(unknown, 2).unwrap_err();
        assert!(matches!(
            unknown,
            SimulationCollectionDeregistrationError::UnknownRegistration(_)
        ));
        let error = second.deregister(registration, 2).unwrap_err();
        let registration = error.into_registration().unwrap();
        assert!(first_events.lock().unwrap().is_empty());
        assert!(second_events.lock().unwrap().is_empty());
        first.deregister(registration, 2).unwrap();
        assert_eq!(first.live_registration_count(), 0);
        let stale = SimulationCollectionRegistration {
            domain: Arc::downgrade(&first.domain),
            id: 0,
        };
        assert!(matches!(
            first.deregister(stale, 3),
            Err(SimulationCollectionDeregistrationError::UnknownRegistration(_))
        ));
    }

    #[test]
    fn split_phase_capabilities_cannot_cross_adapter_domains() {
        let prepared = prepared(1);
        let first_events = Arc::new(Mutex::new(Vec::new()));
        let second_events = Arc::new(Mutex::new(Vec::new()));
        let mut first = adapter(&first_events);
        let mut second = adapter(&second_events);

        let active = first
            .register_active_normal_location_pass_with::<Infallible, _>(
                &prepared,
                7,
                |normal, prepared, userdata_raw| {
                    normal
                        .register_normal(prepared, userdata_raw)
                        .map_err(SimulationCollectionNormalRegistrationCallError::Nested)
                },
            )
            .unwrap();
        let error = second.finish_active_dmo_pass(active).unwrap_err();
        assert!(matches!(
            error,
            SimulationCollectionRegistrationError::NestedFault(SimulationCollectionFault {
                kind: SimulationCollectionNestedFailureKind::MissingAuthority,
                ..
            })
        ));
        assert!(second_events.lock().unwrap().is_empty());
        assert_eq!(first.live_registration_count(), 1);

        let first_events = Arc::new(Mutex::new(Vec::new()));
        let second_events = Arc::new(Mutex::new(Vec::new()));
        let mut first = adapter(&first_events);
        let mut second = adapter(&second_events);
        let registration = first.register_active(&prepared, 1).unwrap();
        let continuation = first.begin_deregister_dmo_pass(registration, 9).unwrap();
        let completion = first
            .deregister_normal_location_pass_with::<Infallible, _>(
                continuation,
                |normal, registration, userdata_raw| {
                    normal
                        .deregister_normal(registration, userdata_raw)
                        .map_err(SimulationCollectionNormalDeregistrationCallError::Nested)
                },
            )
            .unwrap();
        let error = second.complete_deregistration(completion).unwrap_err();
        assert!(matches!(
            error,
            SimulationCollectionDeregistrationError::NestedFault(SimulationCollectionFault {
                kind: SimulationCollectionNestedFailureKind::MissingAuthority,
                ..
            })
        ));
        assert!(second_events.lock().unwrap().is_empty());
        assert_eq!(first.live_registration_count(), 1);
    }
}
