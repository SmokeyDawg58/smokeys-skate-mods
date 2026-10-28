//! Owner-bound runtime routing for Simulation's separate kind-31 DMO pass.
//!
//! `Simulation::RegDMOArena` (`0x828a0420`) samples
//! `kLoadDmoLocatorFromStream` once per kind-31 asset. `DRegDMOArena`
//! (`0x828a0828`) deliberately does not sample it. This adapter preserves that
//! asymmetry and is separate from the normal `RegArena` walk.

use crate::simulation_dmo_arena_plan::{SimulationDmoArenaPlan, SimulationDmoDataHandle};
use skate2_formats::pegasus::{DmoData, PegasusError};
use std::{
    error::Error,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Weak},
};

const CHALLENGE_MASK: u32 = 0x4;
const FALSE_POLICY_LIVING_WORLD_MASK: u32 = 0x2;
const DMO_REGISTRY_MASK: u32 = 0x7;

/// Exact void-returning calls reached by the kind-31 pass.
///
/// Route calls receive only the full asset ID and raw userdata. Registry calls
/// additionally receive the exact whole-object authority corresponding to the
/// native `tDMOData*`. Implementations must clone that handle if they retain
/// pointer-equivalent state beyond the call.
pub trait SimulationDmoArenaDispatch {
    fn add_living_world_locator_data(&mut self, asset_id: u64, userdata_raw: u32);
    fn add_challenge_dmos(&mut self, asset_id: u64, userdata_raw: u32);
    fn register_dmo_locator(&mut self, source: &SimulationDmoDataHandle, asset_id: u64);
    fn remove_living_world_locator_data(&mut self, asset_id: u64, userdata_raw: u32);
    fn remove_challenge_dmos(&mut self, asset_id: u64, userdata_raw: u32);
    fn deregister_dmo_locator(&mut self, source: &SimulationDmoDataHandle, asset_id: u64);
}

/// Live source for `kLoadDmoLocatorFromStream`.
pub trait SimulationDmoLoadPolicy {
    fn load_dmo_locator_from_stream(&mut self) -> bool;
}

#[derive(Clone, Debug)]
struct PreparedDmoOperation {
    asset_id: u64,
    source: SimulationDmoDataHandle,
}

#[derive(Debug)]
struct PreparedSimulationDmoArenaInner {
    plan: Arc<SimulationDmoArenaPlan>,
    operations: Vec<PreparedDmoOperation>,
}

/// Fully validated, reusable source for one separate DMO arena pass.
pub struct PreparedSimulationDmoArena {
    inner: Arc<PreparedSimulationDmoArenaInner>,
}

impl fmt::Debug for PreparedSimulationDmoArena {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedSimulationDmoArena")
            .field("operation_count", &self.inner.operations.len())
            .field("plan_operation_count", &self.inner.plan.operations().len())
            .finish_non_exhaustive()
    }
}

impl PreparedSimulationDmoArena {
    /// Pairs every operation with its whole-object owner before effects.
    ///
    /// # Errors
    ///
    /// Returns a typed allocation, missing-source, span, or descriptor error.
    pub fn try_new(
        plan: Arc<SimulationDmoArenaPlan>,
    ) -> Result<Self, SimulationDmoArenaPrepareError> {
        let requested = plan.operations().len();
        let mut operations = Vec::new();
        operations.try_reserve_exact(requested).map_err(|_| {
            SimulationDmoArenaPrepareError::OperationStorageAllocation { requested }
        })?;
        for (operation_index, operation) in plan.operations().iter().enumerate() {
            let source = plan
                .source_handle(operation_index)
                .ok_or(SimulationDmoArenaPrepareError::MissingSource { operation_index })?;
            if source.bytes().len()
                != usize::try_from(operation.source_object_len).map_err(|_| {
                    SimulationDmoArenaPrepareError::SourceLengthOverflow { operation_index }
                })?
            {
                return Err(SimulationDmoArenaPrepareError::SourceLengthMismatch {
                    operation_index,
                });
            }
            let parsed =
                DmoData::parse(source.bytes()).map_err(SimulationDmoArenaPrepareError::Pegasus)?;
            if parsed.dmos.len()
                != usize::try_from(operation.locator_count).map_err(|_| {
                    SimulationDmoArenaPrepareError::LocatorCountOverflow { operation_index }
                })?
                || parsed.type_id != operation.header_type_id
                || parsed.string_count != operation.string_count
                || parsed.dmos_offset != operation.dmos_offset
                || parsed.string_list_offset != operation.string_list_offset
            {
                return Err(SimulationDmoArenaPrepareError::DescriptorMismatch { operation_index });
            }
            operations.push(PreparedDmoOperation {
                asset_id: operation.asset_id,
                source,
            });
        }
        Ok(Self {
            inner: Arc::new(PreparedSimulationDmoArenaInner { plan, operations }),
        })
    }

    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.inner.operations.len()
    }

    #[must_use]
    pub fn plan(&self) -> &SimulationDmoArenaPlan {
        &self.inner.plan
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegisteredRoute {
    None,
    LivingWorld,
    Challenge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RegisteredEffect {
    route: RegisteredRoute,
    dmo_registry_called: bool,
}

impl Default for RegisteredEffect {
    fn default() -> Self {
        Self {
            route: RegisteredRoute::None,
            dmo_registry_called: false,
        }
    }
}

#[derive(Debug)]
struct LiveRegistration {
    id: u64,
    prepared: Arc<PreparedSimulationDmoArenaInner>,
    effects: Vec<RegisteredEffect>,
}

/// Routes one arena's separate DMO pass and retains all source lifetimes.
///
/// `dispatch` is declared first so it is destroyed before any arena bytes it
/// may reference. Neither this adapter nor its tokens issue calls from `Drop`.
pub struct SimulationDmoArenaRegistrationAdapter<P, D> {
    dispatch: D,
    policy: P,
    live: Vec<LiveRegistration>,
    orphaned: Vec<Arc<PreparedSimulationDmoArenaInner>>,
    domain: Arc<()>,
    next_registration_id: u64,
    fault: Option<SimulationDmoAdapterFault>,
}

impl<P: fmt::Debug, D: fmt::Debug> fmt::Debug for SimulationDmoArenaRegistrationAdapter<P, D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationDmoArenaRegistrationAdapter")
            .field("dispatch", &self.dispatch)
            .field("policy", &self.policy)
            .field("live_registration_count", &self.live.len())
            .field("orphaned_source_count", &self.orphaned.len())
            .field("next_registration_id", &self.next_registration_id)
            .field("fault", &self.fault)
            .finish_non_exhaustive()
    }
}

impl<P: SimulationDmoLoadPolicy, D: SimulationDmoArenaDispatch>
    SimulationDmoArenaRegistrationAdapter<P, D>
{
    #[must_use]
    pub fn new(policy: P, dispatch: D) -> Self {
        Self {
            dispatch,
            policy,
            live: Vec::new(),
            orphaned: Vec::new(),
            domain: Arc::new(()),
            next_registration_id: 0,
            fault: None,
        }
    }

    /// Samples policy and dispatches each operation forward.
    ///
    /// All host storage and the source owner are installed before the first
    /// policy read. A collaborator unwind latches the exact first boundary,
    /// retains the reached call prefix and source owner, and performs no rollback.
    ///
    /// # Errors
    ///
    /// Returns a typed pre-effect host allocation/exhaustion error, replays an
    /// existing adapter fault, or reports and latches a collaborator unwind.
    pub fn register(
        &mut self,
        prepared: &PreparedSimulationDmoArena,
        userdata_raw: u32,
    ) -> Result<SimulationDmoArenaRegistration, SimulationDmoRegistrationError> {
        if let Some(fault) = self.fault {
            return Err(SimulationDmoRegistrationError::AdapterFaulted(fault));
        }
        self.live.try_reserve(1).map_err(|_| {
            SimulationDmoRegistrationError::LiveStorageAllocation {
                requested: self.live.len().saturating_add(1),
            }
        })?;
        let requested = prepared.inner.operations.len();
        let mut effects = Vec::new();
        effects
            .try_reserve_exact(requested)
            .map_err(|_| SimulationDmoRegistrationError::EffectStorageAllocation { requested })?;
        effects.resize(requested, RegisteredEffect::default());
        let registration_id = self.next_registration_id;
        self.next_registration_id = registration_id
            .checked_add(1)
            .ok_or(SimulationDmoRegistrationError::RegistrationIdExhausted)?;
        self.live.push(LiveRegistration {
            id: registration_id,
            prepared: Arc::clone(&prepared.inner),
            effects,
        });
        let live_index = self.live.len() - 1;
        let token = SimulationDmoArenaRegistration {
            domain: Arc::downgrade(&self.domain),
            id: registration_id,
        };

        for (operation_index, operation) in prepared.inner.operations.iter().enumerate() {
            let Ok(policy_enabled) = catch_unwind(AssertUnwindSafe(|| {
                self.policy.load_dmo_locator_from_stream()
            })) else {
                let fault = self.latch_fault(
                    SimulationDmoAdapterFaultPhase::Registration,
                    operation_index,
                    SimulationDmoAdapterFaultStage::Policy,
                );
                return Err(SimulationDmoRegistrationError::DispatchPanicked(fault));
            };
            let effect = registered_effect(policy_enabled, userdata_raw);
            self.live[live_index].effects[operation_index] = effect;
            if let Err(stage) =
                dispatch_registered_effect(&mut self.dispatch, operation, userdata_raw, effect)
            {
                let fault = self.latch_fault(
                    SimulationDmoAdapterFaultPhase::Registration,
                    operation_index,
                    stage,
                );
                return Err(SimulationDmoRegistrationError::DispatchPanicked(fault));
            }
        }
        Ok(token)
    }

    /// Dispatches the native asymmetric teardown using fresh userdata.
    ///
    /// Deregistration never reads the live policy. A high-confidence host
    /// orphan owner is retained when the successful calls cannot prove that a
    /// registration publication was symmetrically removed.
    ///
    /// # Errors
    ///
    /// Foreign, unknown, and pre-fault tokens are returned without effects.
    /// Host orphan storage failure is also pre-effect and recoverable. A
    /// collaborator unwind consumes the token and permanently faults the
    /// adapter while retaining its source owner.
    pub fn deregister(
        &mut self,
        registration: SimulationDmoArenaRegistration,
        userdata_raw: u32,
    ) -> Result<(), SimulationDmoDeregistrationError> {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(SimulationDmoDeregistrationError::ForeignDomain(
                registration,
            ));
        }
        if let Some(fault) = self.fault {
            return Err(SimulationDmoDeregistrationError::AdapterFaulted {
                fault,
                registration,
            });
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            return Err(SimulationDmoDeregistrationError::UnknownRegistration(
                registration,
            ));
        };
        let deregistration_route = if userdata_raw & CHALLENGE_MASK == CHALLENGE_MASK {
            RegisteredRoute::Challenge
        } else {
            RegisteredRoute::LivingWorld
        };
        let deregisters_registry = userdata_raw & DMO_REGISTRY_MASK == DMO_REGISTRY_MASK;
        let retain_orphan = self.live[live_index].effects.iter().any(|effect| {
            (effect.route != RegisteredRoute::None && effect.route != deregistration_route)
                || (effect.dmo_registry_called && !deregisters_registry)
        });
        if retain_orphan {
            self.orphaned.try_reserve(1).map_err(|_| {
                SimulationDmoDeregistrationError::OrphanStorageAllocation(registration)
            })?;
        }
        let prepared = Arc::clone(&self.live[live_index].prepared);
        for (operation_index, operation) in prepared.operations.iter().enumerate() {
            if let Err(stage) = dispatch_deregistration(&mut self.dispatch, operation, userdata_raw)
            {
                let fault = self.latch_fault(
                    SimulationDmoAdapterFaultPhase::Deregistration,
                    operation_index,
                    stage,
                );
                return Err(SimulationDmoDeregistrationError::DispatchPanicked(fault));
            }
        }
        if retain_orphan {
            self.orphaned.push(Arc::clone(&prepared));
        }
        self.live.swap_remove(live_index);
        Ok(())
    }

    fn latch_fault(
        &mut self,
        phase: SimulationDmoAdapterFaultPhase,
        operation_index: usize,
        stage: SimulationDmoAdapterFaultStage,
    ) -> SimulationDmoAdapterFault {
        let fault = SimulationDmoAdapterFault {
            phase,
            operation_index,
            stage,
        };
        self.fault.get_or_insert(fault);
        self.fault.unwrap_or(fault)
    }

    #[must_use]
    pub fn live_registration_count(&self) -> usize {
        self.live.len()
    }

    #[must_use]
    pub fn orphaned_source_count(&self) -> usize {
        self.orphaned.len()
    }

    #[must_use]
    pub const fn fault(&self) -> Option<SimulationDmoAdapterFault> {
        self.fault
    }

    #[must_use]
    pub const fn dispatch(&self) -> &D {
        &self.dispatch
    }

    #[must_use]
    pub const fn policy(&self) -> &P {
        &self.policy
    }
}

fn registered_effect(policy_enabled: bool, userdata_raw: u32) -> RegisteredEffect {
    if policy_enabled {
        RegisteredEffect {
            route: if userdata_raw & CHALLENGE_MASK == CHALLENGE_MASK {
                RegisteredRoute::Challenge
            } else {
                RegisteredRoute::LivingWorld
            },
            dmo_registry_called: userdata_raw & DMO_REGISTRY_MASK == DMO_REGISTRY_MASK,
        }
    } else {
        RegisteredEffect {
            route: if userdata_raw & FALSE_POLICY_LIVING_WORLD_MASK
                == FALSE_POLICY_LIVING_WORLD_MASK
            {
                RegisteredRoute::LivingWorld
            } else {
                RegisteredRoute::None
            },
            dmo_registry_called: false,
        }
    }
}

fn dispatch_registered_effect<D: SimulationDmoArenaDispatch>(
    dispatch: &mut D,
    operation: &PreparedDmoOperation,
    userdata_raw: u32,
    effect: RegisteredEffect,
) -> Result<(), SimulationDmoAdapterFaultStage> {
    let route = match effect.route {
        RegisteredRoute::None => Ok(()),
        RegisteredRoute::LivingWorld => catch_unwind(AssertUnwindSafe(|| {
            dispatch.add_living_world_locator_data(operation.asset_id, userdata_raw);
        }))
        .map_err(|_| SimulationDmoAdapterFaultStage::LivingWorld),
        RegisteredRoute::Challenge => catch_unwind(AssertUnwindSafe(|| {
            dispatch.add_challenge_dmos(operation.asset_id, userdata_raw);
        }))
        .map_err(|_| SimulationDmoAdapterFaultStage::Challenge),
    };
    route?;
    if effect.dmo_registry_called {
        catch_unwind(AssertUnwindSafe(|| {
            dispatch.register_dmo_locator(&operation.source, operation.asset_id);
        }))
        .map_err(|_| SimulationDmoAdapterFaultStage::DmoRegistry)?;
    }
    Ok(())
}

fn dispatch_deregistration<D: SimulationDmoArenaDispatch>(
    dispatch: &mut D,
    operation: &PreparedDmoOperation,
    userdata_raw: u32,
) -> Result<(), SimulationDmoAdapterFaultStage> {
    if userdata_raw & CHALLENGE_MASK == CHALLENGE_MASK {
        catch_unwind(AssertUnwindSafe(|| {
            dispatch.remove_challenge_dmos(operation.asset_id, userdata_raw);
        }))
        .map_err(|_| SimulationDmoAdapterFaultStage::Challenge)?;
    } else {
        catch_unwind(AssertUnwindSafe(|| {
            dispatch.remove_living_world_locator_data(operation.asset_id, userdata_raw);
        }))
        .map_err(|_| SimulationDmoAdapterFaultStage::LivingWorld)?;
    }
    if userdata_raw & DMO_REGISTRY_MASK == DMO_REGISTRY_MASK {
        catch_unwind(AssertUnwindSafe(|| {
            dispatch.deregister_dmo_locator(&operation.source, operation.asset_id);
        }))
        .map_err(|_| SimulationDmoAdapterFaultStage::DmoRegistry)?;
    }
    Ok(())
}

/// Non-cloneable authority for one live kind-31 pass occurrence.
pub struct SimulationDmoArenaRegistration {
    domain: Weak<()>,
    id: u64,
}

impl fmt::Debug for SimulationDmoArenaRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationDmoArenaRegistration")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum SimulationDmoArenaPrepareError {
    OperationStorageAllocation { requested: usize },
    MissingSource { operation_index: usize },
    SourceLengthOverflow { operation_index: usize },
    SourceLengthMismatch { operation_index: usize },
    LocatorCountOverflow { operation_index: usize },
    DescriptorMismatch { operation_index: usize },
    Pegasus(PegasusError),
}

impl fmt::Display for SimulationDmoArenaPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationStorageAllocation { requested } => {
                write!(
                    formatter,
                    "could not retain {requested} Simulation DMO operations"
                )
            }
            Self::MissingSource { operation_index } => {
                write!(
                    formatter,
                    "Simulation DMO operation {operation_index} has no source"
                )
            }
            Self::SourceLengthOverflow { operation_index } => write!(
                formatter,
                "Simulation DMO operation {operation_index} source length does not fit usize"
            ),
            Self::SourceLengthMismatch { operation_index } => write!(
                formatter,
                "Simulation DMO operation {operation_index} source length changed"
            ),
            Self::LocatorCountOverflow { operation_index } => write!(
                formatter,
                "Simulation DMO operation {operation_index} locator count does not fit usize"
            ),
            Self::DescriptorMismatch { operation_index } => write!(
                formatter,
                "Simulation DMO operation {operation_index} descriptor changed"
            ),
            Self::Pegasus(source) => write!(formatter, "prepared DMO data is invalid: {source}"),
        }
    }
}

impl Error for SimulationDmoArenaPrepareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Pegasus(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulationDmoAdapterFault {
    phase: SimulationDmoAdapterFaultPhase,
    operation_index: usize,
    stage: SimulationDmoAdapterFaultStage,
}

impl SimulationDmoAdapterFault {
    #[must_use]
    pub const fn phase(self) -> SimulationDmoAdapterFaultPhase {
        self.phase
    }

    #[must_use]
    pub const fn operation_index(self) -> usize {
        self.operation_index
    }

    #[must_use]
    pub const fn stage(self) -> SimulationDmoAdapterFaultStage {
        self.stage
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationDmoAdapterFaultPhase {
    Registration,
    Deregistration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationDmoAdapterFaultStage {
    Policy,
    LivingWorld,
    Challenge,
    DmoRegistry,
}

#[derive(Debug)]
pub enum SimulationDmoRegistrationError {
    AdapterFaulted(SimulationDmoAdapterFault),
    LiveStorageAllocation { requested: usize },
    EffectStorageAllocation { requested: usize },
    RegistrationIdExhausted,
    DispatchPanicked(SimulationDmoAdapterFault),
}

impl fmt::Display for SimulationDmoRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdapterFaulted(_) => formatter.write_str("Simulation DMO adapter is faulted"),
            Self::LiveStorageAllocation { requested } => write!(
                formatter,
                "could not allocate Simulation DMO registration {requested}"
            ),
            Self::EffectStorageAllocation { requested } => write!(
                formatter,
                "could not allocate {requested} Simulation DMO registration effects"
            ),
            Self::RegistrationIdExhausted => {
                formatter.write_str("Simulation DMO registration IDs are exhausted")
            }
            Self::DispatchPanicked(_) => {
                formatter.write_str("Simulation DMO registration collaborator panicked")
            }
        }
    }
}

impl Error for SimulationDmoRegistrationError {}

#[derive(Debug)]
pub enum SimulationDmoDeregistrationError {
    ForeignDomain(SimulationDmoArenaRegistration),
    UnknownRegistration(SimulationDmoArenaRegistration),
    AdapterFaulted {
        fault: SimulationDmoAdapterFault,
        registration: SimulationDmoArenaRegistration,
    },
    OrphanStorageAllocation(SimulationDmoArenaRegistration),
    DispatchPanicked(SimulationDmoAdapterFault),
}

impl fmt::Display for SimulationDmoDeregistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ForeignDomain(_) => "registration belongs to another Simulation DMO adapter",
            Self::UnknownRegistration(_) => "Simulation DMO registration is not live",
            Self::AdapterFaulted { .. } => "Simulation DMO adapter is faulted",
            Self::OrphanStorageAllocation(_) => {
                "could not retain an asymmetrically deregistered Simulation DMO source"
            }
            Self::DispatchPanicked(_) => "Simulation DMO deregistration collaborator panicked",
        })
    }
}

impl SimulationDmoDeregistrationError {
    #[must_use]
    pub fn into_registration(self) -> Option<SimulationDmoArenaRegistration> {
        match self {
            Self::ForeignDomain(registration)
            | Self::UnknownRegistration(registration)
            | Self::OrphanStorageAllocation(registration)
            | Self::AdapterFaulted { registration, .. } => Some(registration),
            Self::DispatchPanicked(_) => None,
        }
    }
}

impl Error for SimulationDmoDeregistrationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_normal_arena_plan::SimulationNormalArenaPlan,
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        pegasus::{DMO_DATA_TYPE_ID, ObjectId, TABLE_OF_CONTENTS_TYPE_ID},
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::{
        collections::VecDeque,
        ops::Range,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        AddLiving(u64, u32),
        AddChallenge(u64, u32),
        Register(u64),
        RemoveLiving(u64, u32),
        RemoveChallenge(u64, u32),
        Deregister(u64),
    }

    #[derive(Default)]
    struct Recorder {
        events: Vec<Event>,
        registered_sources: Vec<SimulationDmoDataHandle>,
        deregistered_sources: Vec<SimulationDmoDataHandle>,
        panic_add_living: Arc<AtomicBool>,
        panic_add_challenge: Arc<AtomicBool>,
        panic_register: Arc<AtomicBool>,
        panic_remove_challenge: Arc<AtomicBool>,
        panic_deregister: Arc<AtomicBool>,
    }

    impl SimulationDmoArenaDispatch for Recorder {
        fn add_living_world_locator_data(&mut self, asset_id: u64, userdata_raw: u32) {
            self.events.push(Event::AddLiving(asset_id, userdata_raw));
            assert!(!self.panic_add_living.load(Ordering::SeqCst));
        }

        fn add_challenge_dmos(&mut self, asset_id: u64, userdata_raw: u32) {
            self.events
                .push(Event::AddChallenge(asset_id, userdata_raw));
            assert!(!self.panic_add_challenge.load(Ordering::SeqCst));
        }

        fn register_dmo_locator(&mut self, source: &SimulationDmoDataHandle, asset_id: u64) {
            self.events.push(Event::Register(asset_id));
            self.registered_sources.push(source.clone());
            assert!(!self.panic_register.load(Ordering::SeqCst));
        }

        fn remove_living_world_locator_data(&mut self, asset_id: u64, userdata_raw: u32) {
            self.events
                .push(Event::RemoveLiving(asset_id, userdata_raw));
        }

        fn remove_challenge_dmos(&mut self, asset_id: u64, userdata_raw: u32) {
            self.events
                .push(Event::RemoveChallenge(asset_id, userdata_raw));
            assert!(!self.panic_remove_challenge.load(Ordering::SeqCst));
        }

        fn deregister_dmo_locator(&mut self, source: &SimulationDmoDataHandle, asset_id: u64) {
            self.events.push(Event::Deregister(asset_id));
            self.deregistered_sources.push(source.clone());
            assert!(!self.panic_deregister.load(Ordering::SeqCst));
        }
    }

    #[derive(Default)]
    struct Policy {
        answers: VecDeque<bool>,
        samples: usize,
        panic: bool,
    }

    impl Policy {
        fn new(answers: impl IntoIterator<Item = bool>) -> Self {
            Self {
                answers: answers.into_iter().collect(),
                samples: 0,
                panic: false,
            }
        }
    }

    impl SimulationDmoLoadPolicy for Policy {
        fn load_dmo_locator_from_stream(&mut self) -> bool {
            self.samples += 1;
            assert!(!self.panic);
            self.answers.pop_front().unwrap_or(false)
        }
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn make_prepared(ids: &[u64]) -> (PreparedSimulationDmoArena, Weak<SimulationDmoArenaPlan>) {
        let mut resource = Vec::new();
        let mut dictionary = Vec::new();
        let mut objects = Vec::new();
        for _ in ids {
            let start = resource.len();
            let mut object = vec![0; 21];
            put_u32(&mut object, 12, 20);
            put_u32(&mut object, 16, 20);
            resource.extend_from_slice(&object);
            let end = resource.len();
            let index = dictionary.len();
            dictionary.push(DictionaryEntry {
                resource_index: 0,
                object_offset: u32::try_from(start).unwrap(),
                relocation_offset: 0,
                size: 21,
                alignment: 1,
                type_index: 0,
                type_id: DMO_DATA_TYPE_ID,
                object_range: Range { start, end },
            });
            objects.push(ObjectId(u32::try_from(index).unwrap()));
        }
        let toc_start = resource.len();
        let names = 20 + ids.len() * 24;
        let mut toc = vec![0; names + 1];
        put_u32(&mut toc, 0, u32::try_from(ids.len()).unwrap());
        put_u32(&mut toc, 4, 20);
        put_u32(&mut toc, 8, u32::try_from(names).unwrap());
        put_u32(&mut toc, 16, u32::try_from(names).unwrap());
        for (index, (&id, object)) in ids.iter().zip(&objects).enumerate() {
            let offset = 20 + index * 24;
            put_u32(&mut toc, offset, u32::try_from(names).unwrap());
            put_u64(&mut toc, offset + 8, id);
            put_u32(&mut toc, offset + 16, DMO_DATA_TYPE_ID);
            put_u32(&mut toc, offset + 20, object.0);
        }
        resource.extend_from_slice(&toc);
        let toc_end = resource.len();
        dictionary.push(DictionaryEntry {
            resource_index: 0,
            object_offset: u32::try_from(toc_start).unwrap(),
            relocation_offset: 0,
            size: u32::try_from(toc.len()).unwrap(),
            alignment: 1,
            type_index: 0,
            type_id: TABLE_OF_CONTENTS_TYPE_ID,
            object_range: Range {
                start: toc_start,
                end: toc_end,
            },
        });
        let normal = make_normal_plan(resource, dictionary);
        let plan = Arc::new(SimulationDmoArenaPlan::from_normal_plan(normal).unwrap());
        let weak = Arc::downgrade(&plan);
        let prepared = PreparedSimulationDmoArena::try_new(plan).unwrap();
        (prepared, weak)
    }

    fn make_normal_plan(
        resource: Vec<u8>,
        dictionary: Vec<DictionaryEntry>,
    ) -> Arc<SimulationNormalArenaPlan> {
        let descriptor = ResourceDescriptor {
            size: 0,
            alignment: 1,
        };
        let entry_count = u32::try_from(dictionary.len()).unwrap();
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
            id: 1,
            entry_count,
            used_entry_count: entry_count,
            alignment: 1,
            virtual_size: u32::try_from(resource.len()).unwrap(),
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: [descriptor; 5],
            resources_used: [descriptor; 5],
            resource_offsets: [0; 5],
            arena_group_offset: 0,
            dictionary,
        };
        let collision = SimulationCollisionAsset {
            decoded: DecodedArenaAsset::from_parts_for_test(
                2,
                WORLD_STREAM_ARENA_ASSET_TYPE,
                ArenaResources {
                    resources: [resource, Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                },
                arena,
            ),
            collision: CollisionArena {
                id: 1,
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
        Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap())
    }

    fn registration_events(policy: bool, userdata: u32) -> Vec<Event> {
        let id = 0xfedc_ba98_7654_3210;
        let (prepared, _) = make_prepared(&[id]);
        let mut adapter =
            SimulationDmoArenaRegistrationAdapter::new(Policy::new([policy]), Recorder::default());
        let _registration = adapter.register(&prepared, userdata).unwrap();
        adapter.dispatch().events.clone()
    }

    #[test]
    fn registration_truth_table_preserves_masks_and_full_userdata() {
        let id = 0xfedc_ba98_7654_3210;
        assert_eq!(
            registration_events(true, 0x100),
            vec![Event::AddLiving(id, 0x100)]
        );
        assert_eq!(
            registration_events(true, 0x102),
            vec![Event::AddLiving(id, 0x102)]
        );
        assert_eq!(
            registration_events(true, 0x104),
            vec![Event::AddChallenge(id, 0x104)]
        );
        assert_eq!(
            registration_events(true, 0x107),
            vec![Event::AddChallenge(id, 0x107), Event::Register(id)]
        );
        assert!(registration_events(false, 0x100).is_empty());
        assert_eq!(
            registration_events(false, 0x102),
            vec![Event::AddLiving(id, 0x102)]
        );
        assert!(registration_events(false, 0x104).is_empty());
        assert_eq!(
            registration_events(false, 0x107),
            vec![Event::AddLiving(id, 0x107)]
        );
    }

    #[test]
    fn samples_each_registration_operation_but_never_deregistration() {
        let ids = [0x1111_2222_3333_4444, 0xaaaa_bbbb_cccc_dddd];
        let (prepared, _) = make_prepared(&ids);
        let mut adapter = SimulationDmoArenaRegistrationAdapter::new(
            Policy::new([true, false]),
            Recorder::default(),
        );
        let registration = adapter.register(&prepared, 7).unwrap();
        assert_eq!(adapter.policy().samples, 2);
        assert_eq!(
            adapter.dispatch().events,
            vec![
                Event::AddChallenge(ids[0], 7),
                Event::Register(ids[0]),
                Event::AddLiving(ids[1], 7),
            ]
        );
        adapter.deregister(registration, 7).unwrap();
        assert_eq!(adapter.policy().samples, 2);
        assert_eq!(
            adapter.dispatch().events,
            vec![
                Event::AddChallenge(ids[0], 7),
                Event::Register(ids[0]),
                Event::AddLiving(ids[1], 7),
                Event::RemoveChallenge(ids[0], 7),
                Event::Deregister(ids[0]),
                Event::RemoveChallenge(ids[1], 7),
                Event::Deregister(ids[1]),
            ]
        );
        assert!(
            adapter.dispatch().registered_sources[0]
                .is_same_backing_object(&adapter.dispatch().deregistered_sources[0])
        );
    }

    #[test]
    fn symmetric_teardown_releases_owner_but_mismatch_retains_orphan() {
        let (prepared, weak) = make_prepared(&[1]);
        let static_weak = Arc::downgrade(&prepared.inner.plan.normal_plan().static_plan_owner());
        let mut adapter =
            SimulationDmoArenaRegistrationAdapter::new(Policy::new([true]), Recorder::default());
        let registration = adapter.register(&prepared, 0).unwrap();
        drop(prepared);
        adapter.deregister(registration, 0).unwrap();
        assert_eq!(adapter.orphaned_source_count(), 0);
        assert!(weak.upgrade().is_none());
        assert!(static_weak.upgrade().is_none());

        let (prepared, weak) = make_prepared(&[2]);
        let static_weak = Arc::downgrade(&prepared.inner.plan.normal_plan().static_plan_owner());
        let mut adapter =
            SimulationDmoArenaRegistrationAdapter::new(Policy::new([true]), Recorder::default());
        let registration = adapter.register(&prepared, 0).unwrap();
        drop(prepared);
        adapter.deregister(registration, 4).unwrap();
        assert_eq!(adapter.orphaned_source_count(), 1);
        assert!(weak.upgrade().is_some());
        assert!(static_weak.upgrade().is_some());
        drop(adapter);
        assert!(weak.upgrade().is_none());
        assert!(static_weak.upgrade().is_none());

        let (prepared, _) = make_prepared(&[3]);
        let mut adapter =
            SimulationDmoArenaRegistrationAdapter::new(Policy::new([true]), Recorder::default());
        let registration = adapter.register(&prepared, 7).unwrap();
        adapter.deregister(registration, 4).unwrap();
        assert_eq!(adapter.orphaned_source_count(), 1);
    }

    #[test]
    fn no_add_followed_by_forced_remove_does_not_invent_an_orphan() {
        let (prepared, weak) = make_prepared(&[3]);
        let mut adapter =
            SimulationDmoArenaRegistrationAdapter::new(Policy::new([false]), Recorder::default());
        let registration = adapter.register(&prepared, 0).unwrap();
        drop(prepared);
        adapter.deregister(registration, 4).unwrap();
        assert_eq!(
            adapter.dispatch().events,
            vec![Event::RemoveChallenge(3, 4)]
        );
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn foreign_token_is_zero_effect_and_recoverable_by_owner() {
        let (prepared, _) = make_prepared(&[4]);
        let mut owner =
            SimulationDmoArenaRegistrationAdapter::new(Policy::new([true]), Recorder::default());
        let mut foreign =
            SimulationDmoArenaRegistrationAdapter::new(Policy::default(), Recorder::default());
        let registration = owner.register(&prepared, 0).unwrap();
        let error = foreign.deregister(registration, 0).unwrap_err();
        assert!(foreign.dispatch().events.is_empty());
        owner
            .deregister(error.into_registration().unwrap(), 0)
            .unwrap();
        assert_eq!(owner.live_registration_count(), 0);
    }

    #[test]
    fn dropping_token_is_inert_and_keeps_live_owner() {
        let (prepared, weak) = make_prepared(&[9]);
        let mut adapter =
            SimulationDmoArenaRegistrationAdapter::new(Policy::new([true]), Recorder::default());
        let registration = adapter.register(&prepared, 0).unwrap();
        let events = adapter.dispatch().events.clone();
        drop(prepared);
        drop(registration);
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.dispatch().events, events);
        assert!(weak.upgrade().is_some());
        drop(adapter);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn registration_panic_latches_exact_stage_and_retains_owner() {
        let (prepared, weak) = make_prepared(&[5, 6]);
        let recorder = Recorder::default();
        recorder.panic_add_challenge.store(true, Ordering::SeqCst);
        let mut adapter =
            SimulationDmoArenaRegistrationAdapter::new(Policy::new([true, true]), recorder);
        let error = adapter.register(&prepared, 7).unwrap_err();
        let SimulationDmoRegistrationError::DispatchPanicked(fault) = error else {
            panic!("unexpected registration error");
        };
        assert_eq!(fault.phase(), SimulationDmoAdapterFaultPhase::Registration);
        assert_eq!(fault.operation_index(), 0);
        assert_eq!(fault.stage(), SimulationDmoAdapterFaultStage::Challenge);
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.dispatch().events, vec![Event::AddChallenge(5, 7)]);
        drop(prepared);
        assert!(weak.upgrade().is_some());
        assert!(matches!(
            adapter.register(&make_prepared(&[]).0, 0),
            Err(SimulationDmoRegistrationError::AdapterFaulted(value)) if value == fault
        ));
    }

    #[test]
    fn registration_faults_cover_policy_living_and_later_registry_boundaries() {
        let (prepared, _) = make_prepared(&[10]);
        let mut policy = Policy::new([true]);
        policy.panic = true;
        let mut adapter = SimulationDmoArenaRegistrationAdapter::new(policy, Recorder::default());
        let error = adapter.register(&prepared, 0).unwrap_err();
        let SimulationDmoRegistrationError::DispatchPanicked(fault) = error else {
            panic!("unexpected policy error");
        };
        assert_eq!(fault.stage(), SimulationDmoAdapterFaultStage::Policy);

        let recorder = Recorder::default();
        recorder.panic_add_living.store(true, Ordering::SeqCst);
        let mut adapter = SimulationDmoArenaRegistrationAdapter::new(Policy::new([true]), recorder);
        let error = adapter.register(&prepared, 0).unwrap_err();
        let SimulationDmoRegistrationError::DispatchPanicked(fault) = error else {
            panic!("unexpected Living World error");
        };
        assert_eq!(fault.stage(), SimulationDmoAdapterFaultStage::LivingWorld);

        let (prepared, _) = make_prepared(&[11, 12]);
        let recorder = Recorder::default();
        recorder.panic_register.store(true, Ordering::SeqCst);
        let mut adapter =
            SimulationDmoArenaRegistrationAdapter::new(Policy::new([false, true]), recorder);
        let error = adapter.register(&prepared, 7).unwrap_err();
        let SimulationDmoRegistrationError::DispatchPanicked(fault) = error else {
            panic!("unexpected registry error");
        };
        assert_eq!(fault.operation_index(), 1);
        assert_eq!(fault.stage(), SimulationDmoAdapterFaultStage::DmoRegistry);
        assert_eq!(
            adapter.dispatch().events,
            vec![
                Event::AddLiving(11, 7),
                Event::AddChallenge(12, 7),
                Event::Register(12),
            ]
        );
    }

    #[test]
    fn deregistration_route_panic_consumes_token_and_suppresses_registry() {
        let (prepared, _) = make_prepared(&[7]);
        let recorder = Recorder::default();
        let panic_remove = Arc::clone(&recorder.panic_remove_challenge);
        let mut adapter = SimulationDmoArenaRegistrationAdapter::new(Policy::new([true]), recorder);
        let registration = adapter.register(&prepared, 7).unwrap();
        panic_remove.store(true, Ordering::SeqCst);
        let error = adapter.deregister(registration, 7).unwrap_err();
        let SimulationDmoDeregistrationError::DispatchPanicked(fault) = error else {
            panic!("unexpected deregistration error");
        };
        assert_eq!(
            fault.phase(),
            SimulationDmoAdapterFaultPhase::Deregistration
        );
        assert_eq!(fault.operation_index(), 0);
        assert_eq!(fault.stage(), SimulationDmoAdapterFaultStage::Challenge);
        assert_eq!(adapter.live_registration_count(), 1);
        assert!(!adapter.dispatch().events.contains(&Event::Deregister(7)));
    }

    #[test]
    fn deregistration_registry_panic_latches_after_route_and_consumes_token() {
        let (prepared, _) = make_prepared(&[13]);
        let recorder = Recorder::default();
        let panic_deregister = Arc::clone(&recorder.panic_deregister);
        let mut adapter = SimulationDmoArenaRegistrationAdapter::new(Policy::new([true]), recorder);
        let registration = adapter.register(&prepared, 7).unwrap();
        panic_deregister.store(true, Ordering::SeqCst);
        let error = adapter.deregister(registration, 7).unwrap_err();
        let SimulationDmoDeregistrationError::DispatchPanicked(fault) = error else {
            panic!("unexpected deregistration error");
        };
        assert_eq!(fault.stage(), SimulationDmoAdapterFaultStage::DmoRegistry);
        assert_eq!(adapter.live_registration_count(), 1);
        assert!(
            adapter
                .dispatch()
                .events
                .ends_with(&[Event::RemoveChallenge(13, 7), Event::Deregister(13)])
        );
    }

    #[test]
    fn empty_plan_is_a_distinct_zero_effect_occurrence() {
        let (prepared, weak) = make_prepared(&[]);
        let mut adapter =
            SimulationDmoArenaRegistrationAdapter::new(Policy::default(), Recorder::default());
        let registration = adapter.register(&prepared, u32::MAX).unwrap();
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.policy().samples, 0);
        drop(prepared);
        adapter.deregister(registration, u32::MAX).unwrap();
        assert!(adapter.dispatch().events.is_empty());
        assert!(weak.upgrade().is_none());
    }

    struct DropOrderDispatch {
        owner: Weak<SimulationStaticArenaPlan>,
        owner_was_alive: Arc<AtomicBool>,
    }

    impl Drop for DropOrderDispatch {
        fn drop(&mut self) {
            self.owner_was_alive
                .store(self.owner.upgrade().is_some(), Ordering::SeqCst);
        }
    }

    impl SimulationDmoArenaDispatch for DropOrderDispatch {
        fn add_living_world_locator_data(&mut self, _asset_id: u64, _userdata_raw: u32) {}
        fn add_challenge_dmos(&mut self, _asset_id: u64, _userdata_raw: u32) {}
        fn register_dmo_locator(&mut self, _source: &SimulationDmoDataHandle, _asset_id: u64) {}
        fn remove_living_world_locator_data(&mut self, _asset_id: u64, _userdata_raw: u32) {}
        fn remove_challenge_dmos(&mut self, _asset_id: u64, _userdata_raw: u32) {}
        fn deregister_dmo_locator(&mut self, _source: &SimulationDmoDataHandle, _asset_id: u64) {}
    }

    #[test]
    fn adapter_drop_destroys_dispatch_before_retained_arena_bytes() {
        let (prepared, _) = make_prepared(&[8]);
        let owner = Arc::downgrade(&prepared.inner.plan.normal_plan().static_plan_owner());
        let observed = Arc::new(AtomicBool::new(false));
        let dispatch = DropOrderDispatch {
            owner: owner.clone(),
            owner_was_alive: Arc::clone(&observed),
        };
        let mut adapter = SimulationDmoArenaRegistrationAdapter::new(Policy::new([true]), dispatch);
        let _registration = adapter.register(&prepared, 0).unwrap();
        drop(prepared);
        drop(adapter);
        assert!(observed.load(Ordering::SeqCst));
        assert!(owner.upgrade().is_none());
    }
}
