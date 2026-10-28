//! Ordered Simulation registration for the currently reconstructed static
//! arena subset.
//!
//! `Simulation::RegArena` (`0x828a01e0`) and `DRegArena` (`0x828a05f0`)
//! synchronously walk a `cArena` asset-pointer vector forward, stopping at its
//! end or first null pointer. This adapter preserves that direction and the
//! interleaving of the two kinds currently represented by
//! [`SimulationStaticArenaPlan`]: kind 6 grind splines and kind 10 Environment
//! instances.
//!
//! This is not a complete `RegArena` implementation. Kinds 11, 16, 22, 23,
//! 25, 27, 28, and 32, nullable native asset slots, `LocationManager` work, the
//! separate DMO pass, and collection-message timing remain external.

use crate::{
    grind::GrindAsset,
    simulation_environment_unlock_router::{
        EnvironmentUnlockRouterError, SimulationEnvironmentUnlockRouter,
    },
    simulation_static_arena_plan::{SimulationStaticArenaPlan, SimulationStaticOperation},
    static_ground_pool::{StaticEnvironmentSource, StaticGroundSourceError},
};
use std::{error::Error, fmt, sync::Arc, sync::Weak};

/// Kind-6 effects reached by the supported static arena walk.
///
/// Native `GrindData::AddSplineData` (`0x82cfd228`) and
/// `RemoveSplineData` (`0x82cfb980`) return `void`. This narrow collaborator
/// therefore has no invented recoverable failure result. Concrete static
/// `GrindData` ownership, duplicate-ID removal behavior, and capacity are still
/// separate reconstruction work.
pub trait SimulationStaticGrindDispatch {
    fn add_spline_data(&mut self, asset: &GrindAsset);
    fn remove_spline_data(&mut self, asset_id: u64);
}

#[derive(Debug)]
enum PreparedSimulationStaticOperation {
    Grind {
        grind_asset_index: usize,
    },
    Environment {
        source: Arc<StaticEnvironmentSource>,
    },
}

#[derive(Debug)]
struct PreparedSimulationStaticArenaInner {
    plan: Arc<SimulationStaticArenaPlan>,
    operations: Vec<PreparedSimulationStaticOperation>,
}

/// Fully validated, reusable source for the supported kind-6/kind-10 walk.
///
/// Preparation constructs every Environment source before any registration
/// effect. Registering the same prepared value more than once deliberately
/// creates another occurrence; Simulation has no arena-registration
/// refcounter.
pub struct PreparedSimulationStaticArena {
    inner: Arc<PreparedSimulationStaticArenaInner>,
}

impl fmt::Debug for PreparedSimulationStaticArena {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedSimulationStaticArena")
            .field("asset_id", &self.inner.plan.asset().decoded.asset_id)
            .field("operation_count", &self.inner.operations.len())
            .finish()
    }
}

impl PreparedSimulationStaticArena {
    /// Validates every supported operation and builds stable Environment
    /// sources before registration begins.
    ///
    /// # Errors
    ///
    /// Rejects an invalid grind index, an unsupported Environment collision
    /// source, or host operation-vector allocation before producing any world
    /// effect.
    pub fn try_new(
        plan: Arc<SimulationStaticArenaPlan>,
    ) -> Result<Self, SimulationStaticArenaPrepareError> {
        let requested = plan.operations().len();
        let mut operations = Vec::new();
        operations.try_reserve_exact(requested).map_err(|_| {
            SimulationStaticArenaPrepareError::OperationStorageAllocation { requested }
        })?;

        for (operation_index, operation) in plan.operations().iter().enumerate() {
            let prepared = match *operation {
                SimulationStaticOperation::Grind {
                    grind_asset_index, ..
                } => {
                    if plan.grind_assets().get(grind_asset_index).is_none() {
                        return Err(SimulationStaticArenaPrepareError::MissingGrindAsset {
                            operation_index,
                            grind_asset_index,
                        });
                    }
                    PreparedSimulationStaticOperation::Grind { grind_asset_index }
                }
                SimulationStaticOperation::Environment { .. } => {
                    let source =
                        StaticEnvironmentSource::from_operation(Arc::clone(&plan), operation_index)
                            .map_err(SimulationStaticArenaPrepareError::EnvironmentSource)?;
                    PreparedSimulationStaticOperation::Environment {
                        source: Arc::new(source),
                    }
                }
            };
            operations.push(prepared);
        }

        Ok(Self {
            inner: Arc::new(PreparedSimulationStaticArenaInner { plan, operations }),
        })
    }

    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.inner.operations.len()
    }
}

#[derive(Debug)]
struct LiveSimulationStaticArenaRegistration {
    id: u64,
    inner: Arc<PreparedSimulationStaticArenaInner>,
    processed_operation_count: usize,
    complete: bool,
    userdata_raw: u32,
}

/// Owns the concrete kind-6 and kind-10 authorities used by every supported
/// arena registration occurrence.
///
/// Keeping the live ledger, grind collaborator, and Environment router in one
/// object prevents a registration token from being applied to a different
/// world. Neither this adapter nor its tokens perform implicit deregistration
/// from `Drop`.
pub struct SimulationStaticArenaRegistrationAdapter<G> {
    domain: Arc<()>,
    next_registration_id: u64,
    live: Vec<LiveSimulationStaticArenaRegistration>,
    faulted: bool,
    grind: G,
    environment: SimulationEnvironmentUnlockRouter,
}

impl<G: fmt::Debug> fmt::Debug for SimulationStaticArenaRegistrationAdapter<G> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationStaticArenaRegistrationAdapter")
            .field("next_registration_id", &self.next_registration_id)
            .field("live", &self.live)
            .field("faulted", &self.faulted)
            .field("grind", &self.grind)
            .field("environment", &self.environment)
            .finish_non_exhaustive()
    }
}

impl<G: SimulationStaticGrindDispatch> SimulationStaticArenaRegistrationAdapter<G> {
    #[must_use]
    pub fn new(grind: G, environment: SimulationEnvironmentUnlockRouter) -> Self {
        Self {
            domain: Arc::new(()),
            next_registration_id: 0,
            live: Vec::new(),
            faulted: false,
            grind,
            environment,
        }
    }

    /// Performs the supported subsequence of `Simulation::RegArena` in exact
    /// forward plan order.
    ///
    /// `userdata_raw` is retained on the occurrence even though native kinds
    /// 6 and 10 do not inspect it. It belongs to the wider arena boundary and
    /// must not be normalized before later kinds are reconstructed.
    ///
    /// # Errors
    ///
    /// Host ledger exhaustion is rejected before effects. An Environment
    /// routing error stops the walk, permanently faults this adapter domain,
    /// and returns a non-cloneable token for the retained processed prefix.
    /// No cleanup is offered because a failed repeated add cannot be
    /// distinguished safely from an earlier pointer-equivalent publication.
    pub fn register(
        &mut self,
        prepared: &PreparedSimulationStaticArena,
        userdata_raw: u32,
    ) -> Result<SimulationStaticArenaRegistration, SimulationStaticArenaRegistrationError> {
        if self.faulted {
            return Err(SimulationStaticArenaRegistrationError::AdapterFaulted);
        }
        self.live.try_reserve(1).map_err(|_| {
            SimulationStaticArenaRegistrationError::LiveStorageAllocation {
                requested: self.live.len().saturating_add(1),
            }
        })?;
        let registration_id = self.next_registration_id;
        self.next_registration_id = self
            .next_registration_id
            .checked_add(1)
            .ok_or(SimulationStaticArenaRegistrationError::RegistrationIdExhausted)?;
        self.live.push(LiveSimulationStaticArenaRegistration {
            id: registration_id,
            inner: Arc::clone(&prepared.inner),
            processed_operation_count: 0,
            complete: false,
            userdata_raw,
        });
        let live_index = self.live.len() - 1;
        let mut registration = SimulationStaticArenaRegistration {
            domain: Arc::downgrade(&self.domain),
            id: registration_id,
            processed_operation_count: 0,
            complete: false,
            userdata_raw,
        };

        for (operation_index, operation) in prepared.inner.operations.iter().enumerate() {
            // Mark the current operation first. A found Environment context
            // performs its native stores before a possible host capacity
            // error, so the faulted ledger must retain this prefix through
            // the explicit adapter-abandonment boundary.
            let processed_operation_count = operation_index + 1;
            self.live[live_index].processed_operation_count = processed_operation_count;
            registration.processed_operation_count = processed_operation_count;
            match operation {
                PreparedSimulationStaticOperation::Grind { grind_asset_index } => {
                    self.grind
                        .add_spline_data(&prepared.inner.plan.grind_assets()[*grind_asset_index]);
                }
                PreparedSimulationStaticOperation::Environment { source } => {
                    if let Err(source) = self.environment.register(source) {
                        self.faulted = true;
                        return Err(SimulationStaticArenaRegistrationError::Environment {
                            operation_index,
                            registration,
                            source,
                        });
                    }
                }
            }
        }
        self.live[live_index].complete = true;
        registration.complete = true;
        Ok(registration)
    }

    /// Consumes one same-domain occurrence and performs its supported
    /// deregistration prefix in forward order.
    ///
    /// # Errors
    ///
    /// A foreign, already-consumed, otherwise unknown, or faulted-domain token
    /// is returned unchanged without effects. These are typed safe-host
    /// authority checks, not native error paths.
    pub fn deregister(
        &mut self,
        registration: SimulationStaticArenaRegistration,
    ) -> Result<(), SimulationStaticArenaDeregistrationError> {
        self.cleanup_registration(registration)
    }

    fn cleanup_registration(
        &mut self,
        registration: SimulationStaticArenaRegistration,
    ) -> Result<(), SimulationStaticArenaDeregistrationError> {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(SimulationStaticArenaDeregistrationError {
                kind: SimulationStaticArenaDeregistrationErrorKind::ForeignDomain,
                registration,
            });
        }
        if self.faulted {
            return Err(SimulationStaticArenaDeregistrationError {
                kind: SimulationStaticArenaDeregistrationErrorKind::AdapterFaulted,
                registration,
            });
        }
        let Some(live_index) = self.live.iter().position(|live| {
            live.id == registration.id
                && live.processed_operation_count == registration.processed_operation_count
                && live.complete == registration.complete
                && live.userdata_raw == registration.userdata_raw
        }) else {
            return Err(SimulationStaticArenaDeregistrationError {
                kind: SimulationStaticArenaDeregistrationErrorKind::UnknownRegistration,
                registration,
            });
        };
        let inner = Arc::clone(&self.live[live_index].inner);
        let processed_operation_count = self.live[live_index].processed_operation_count;
        for operation in inner.operations.iter().take(processed_operation_count) {
            match operation {
                PreparedSimulationStaticOperation::Grind { grind_asset_index } => {
                    self.grind
                        .remove_spline_data(inner.plan.grind_assets()[*grind_asset_index].asset_id);
                }
                PreparedSimulationStaticOperation::Environment { source } => {
                    self.environment.deregister(source);
                }
            }
        }
        self.live.swap_remove(live_index);
        Ok(())
    }

    #[must_use]
    pub fn live_registration_count(&self) -> usize {
        self.live.len()
    }

    #[must_use]
    pub const fn is_faulted(&self) -> bool {
        self.faulted
    }

    #[must_use]
    pub fn published_volume_count(&self) -> usize {
        self.environment.published_volume_count()
    }

    #[must_use]
    pub const fn grind(&self) -> &G {
        &self.grind
    }

    #[must_use]
    pub const fn environment(&self) -> &SimulationEnvironmentUnlockRouter {
        &self.environment
    }
}

/// Authority for exactly one supported registration occurrence.
///
/// Its owner adapter's live ledger retains the prepared plan and Environment
/// sources strongly until explicit deregistration. The token is intentionally
/// neither `Clone` nor equipped with a `Drop` cleanup hook.
pub struct SimulationStaticArenaRegistration {
    domain: Weak<()>,
    id: u64,
    processed_operation_count: usize,
    complete: bool,
    userdata_raw: u32,
}

impl fmt::Debug for SimulationStaticArenaRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationStaticArenaRegistration")
            .field("id", &self.id)
            .field("processed_operation_count", &self.processed_operation_count)
            .field("complete", &self.complete)
            .field("userdata_raw", &self.userdata_raw)
            .finish_non_exhaustive()
    }
}

impl SimulationStaticArenaRegistration {
    #[must_use]
    pub const fn processed_operation_count(&self) -> usize {
        self.processed_operation_count
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.complete
    }

    #[must_use]
    pub const fn userdata_raw(&self) -> u32 {
        self.userdata_raw
    }
}

#[derive(Debug)]
pub enum SimulationStaticArenaPrepareError {
    OperationStorageAllocation {
        requested: usize,
    },
    MissingGrindAsset {
        operation_index: usize,
        grind_asset_index: usize,
    },
    EnvironmentSource(StaticGroundSourceError),
}

impl fmt::Display for SimulationStaticArenaPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationStorageAllocation { requested } => write!(
                formatter,
                "could not allocate {requested} supported Simulation arena operations"
            ),
            Self::MissingGrindAsset {
                operation_index,
                grind_asset_index,
            } => write!(
                formatter,
                "Simulation operation {operation_index} references missing grind asset {grind_asset_index}"
            ),
            Self::EnvironmentSource(source) => source.fmt(formatter),
        }
    }
}

impl Error for SimulationStaticArenaPrepareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EnvironmentSource(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum SimulationStaticArenaRegistrationError {
    AdapterFaulted,
    LiveStorageAllocation {
        requested: usize,
    },
    RegistrationIdExhausted,
    Environment {
        operation_index: usize,
        registration: SimulationStaticArenaRegistration,
        source: EnvironmentUnlockRouterError,
    },
}

impl SimulationStaticArenaRegistrationError {
    #[must_use]
    pub const fn operation_index(&self) -> Option<usize> {
        match self {
            Self::Environment {
                operation_index, ..
            } => Some(*operation_index),
            _ => None,
        }
    }
}

impl fmt::Display for SimulationStaticArenaRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdapterFaulted => {
                formatter.write_str("supported Simulation registration adapter is faulted")
            }
            Self::LiveStorageAllocation { requested } => write!(
                formatter,
                "could not allocate supported Simulation registration ledger entry {requested}"
            ),
            Self::RegistrationIdExhausted => {
                formatter.write_str("supported Simulation registration IDs are exhausted")
            }
            Self::Environment {
                operation_index,
                source,
                ..
            } => write!(
                formatter,
                "supported Simulation arena registration failed at operation {operation_index}: {source}"
            ),
        }
    }
}

impl Error for SimulationStaticArenaRegistrationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Environment { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationStaticArenaDeregistrationErrorKind {
    ForeignDomain,
    AdapterFaulted,
    UnknownRegistration,
}

/// Rejected host-side cleanup authority. The token is returned so the caller
/// can present it to its owning adapter or retain it at a faulted-domain
/// abandonment boundary.
#[derive(Debug)]
pub struct SimulationStaticArenaDeregistrationError {
    kind: SimulationStaticArenaDeregistrationErrorKind,
    registration: SimulationStaticArenaRegistration,
}

impl SimulationStaticArenaDeregistrationError {
    #[must_use]
    pub const fn kind(&self) -> SimulationStaticArenaDeregistrationErrorKind {
        self.kind
    }

    #[must_use]
    pub fn into_registration(self) -> SimulationStaticArenaRegistration {
        self.registration
    }
}

impl fmt::Display for SimulationStaticArenaDeregistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind {
            SimulationStaticArenaDeregistrationErrorKind::ForeignDomain => {
                "registration belongs to a different adapter domain"
            }
            SimulationStaticArenaDeregistrationErrorKind::AdapterFaulted => {
                "registration adapter is faulted"
            }
            SimulationStaticArenaDeregistrationErrorKind::UnknownRegistration => {
                "registration is not live in this adapter"
            }
        };
        formatter.write_str(message)
    }
}

impl Error for SimulationStaticArenaDeregistrationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        simulation_environment_unlock_router::{
            EnvironmentUnlockContextSeed, EnvironmentUnlockSeed,
        },
        simulation_static_arena_plan::SimulationStaticOperation,
        static_ground_pool::tests::environment_plan_for_test,
    };
    use skate2_formats::pegasus::SplineData;

    #[derive(Debug, Eq, PartialEq)]
    enum GrindEvent {
        Add(u64),
        Remove(u64),
    }

    #[derive(Default)]
    struct GrindRecorder {
        events: Vec<GrindEvent>,
    }

    impl SimulationStaticGrindDispatch for GrindRecorder {
        fn add_spline_data(&mut self, asset: &GrindAsset) {
            self.events.push(GrindEvent::Add(asset.asset_id));
        }

        fn remove_spline_data(&mut self, asset_id: u64) {
            self.events.push(GrindEvent::Remove(asset_id));
        }
    }

    fn mixed_plan(
        local_guid: u64,
        grind_ids: &[u64],
        operations: Vec<SimulationStaticOperation>,
    ) -> Arc<SimulationStaticArenaPlan> {
        let base = Arc::try_unwrap(environment_plan_for_test(0xaaaa, local_guid, &[0])).unwrap();
        let asset = base.into_asset();
        let empty_spline = SplineData {
            splines_offset: 0,
            segments_offset: 0,
            splines: Vec::new(),
            segments: Vec::new(),
        };
        let grind_assets = grind_ids
            .iter()
            .enumerate()
            .map(|(index, &id)| {
                GrindAsset::from_spline_data(id, u32::try_from(index).unwrap(), &empty_spline)
            })
            .collect();
        Arc::new(SimulationStaticArenaPlan::from_parts_with_grind_for_test(
            asset,
            grind_assets,
            operations,
        ))
    }

    fn grind(index: usize) -> SimulationStaticOperation {
        SimulationStaticOperation::Grind {
            source_toc_index: u32::try_from(index).unwrap(),
            source_dictionary_index: u32::try_from(index).unwrap(),
            grind_asset_index: index,
        }
    }

    fn environment(source_toc_index: u32) -> SimulationStaticOperation {
        SimulationStaticOperation::Environment {
            source_toc_index,
            source_dictionary_index: 7,
            source_inline_index: 0,
            source_resource_index: 0,
            source_record_start: 20,
            instance_index: 0,
        }
    }

    fn empty_router(capacity: usize) -> SimulationEnvironmentUnlockRouter {
        SimulationEnvironmentUnlockRouter::try_new_empty(capacity).unwrap()
    }

    fn adapter(
        router: SimulationEnvironmentUnlockRouter,
    ) -> SimulationStaticArenaRegistrationAdapter<GrindRecorder> {
        SimulationStaticArenaRegistrationAdapter::new(GrindRecorder::default(), router)
    }

    #[test]
    fn empty_plan_registers_and_deregisters_as_a_complete_no_op() {
        let prepared =
            PreparedSimulationStaticArena::try_new(mixed_plan(7, &[], Vec::new())).unwrap();
        let mut adapter = adapter(empty_router(0));

        let registration = adapter.register(&prepared, u32::MAX).unwrap();
        assert!(registration.is_complete());
        assert_eq!(registration.processed_operation_count(), 0);
        assert_eq!(registration.userdata_raw(), u32::MAX);
        adapter.deregister(registration).unwrap();

        assert!(adapter.grind().events.is_empty());
        assert_eq!(adapter.published_volume_count(), 0);
        assert_eq!(adapter.live_registration_count(), 0);
    }

    #[test]
    fn mixed_supported_operations_use_forward_add_and_forward_remove() {
        let first = 0x1234_5678_0000_0001;
        let second = 0xfedc_ba98_0000_0002;
        let prepared = PreparedSimulationStaticArena::try_new(mixed_plan(
            7,
            &[first, second],
            vec![grind(0), environment(1), grind(1)],
        ))
        .unwrap();
        let router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            EnvironmentUnlockSeed {
                locations_lock_status_raw: 1,
                contexts_in_native_iteration_order: vec![EnvironmentUnlockContextSeed {
                    local_guid: 7,
                    challenge_index: 0,
                    is_unlocked_env_instance_raw: 0,
                }],
            },
            1,
        )
        .unwrap();
        let mut adapter = adapter(router);

        let registration = adapter.register(&prepared, 8).unwrap();
        assert_eq!(adapter.published_volume_count(), 1);
        assert_eq!(
            adapter.grind().events,
            [GrindEvent::Add(first), GrindEvent::Add(second)]
        );

        adapter.deregister(registration).unwrap();
        assert_eq!(adapter.published_volume_count(), 0);
        assert_eq!(
            adapter.grind().events,
            [
                GrindEvent::Add(first),
                GrindEvent::Add(second),
                GrindEvent::Remove(first),
                GrindEvent::Remove(second),
            ]
        );
    }

    #[test]
    fn registration_failure_faults_domain_and_retains_current_prefix() {
        let first = 1;
        let later = 2;
        let prepared = PreparedSimulationStaticArena::try_new(mixed_plan(
            7,
            &[first, later],
            vec![grind(0), environment(1), grind(1)],
        ))
        .unwrap();
        let router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            EnvironmentUnlockSeed {
                locations_lock_status_raw: 1,
                contexts_in_native_iteration_order: vec![EnvironmentUnlockContextSeed {
                    local_guid: 7,
                    challenge_index: 0,
                    is_unlocked_env_instance_raw: 0,
                }],
            },
            0,
        )
        .unwrap();
        let mut adapter = adapter(router);

        let failure = adapter.register(&prepared, 0).unwrap_err();
        assert_eq!(failure.operation_index(), Some(1));
        let SimulationStaticArenaRegistrationError::Environment {
            registration: partial,
            ..
        } = failure
        else {
            panic!("expected Environment registration failure")
        };
        assert_eq!(partial.processed_operation_count(), 2);
        assert!(!partial.is_complete());
        assert!(adapter.is_faulted());
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.grind().events, [GrindEvent::Add(first)]);
        assert!(adapter.environment().context_snapshots()[0].has_attached_source);
        assert!(adapter.environment().context_snapshots()[0].is_in_world);

        let rejection = adapter.deregister(partial).unwrap_err();
        assert_eq!(
            rejection.kind(),
            SimulationStaticArenaDeregistrationErrorKind::AdapterFaulted
        );
        assert_eq!(adapter.grind().events, [GrindEvent::Add(first)]);
        assert!(adapter.environment().context_snapshots()[0].has_attached_source);
        assert!(adapter.environment().context_snapshots()[0].is_in_world);
        drop(rejection.into_registration());
        assert!(matches!(
            adapter.register(&prepared, 0),
            Err(SimulationStaticArenaRegistrationError::AdapterFaulted)
        ));
        assert_eq!(adapter.grind().events, [GrindEvent::Add(first)]);
    }

    #[test]
    fn absent_context_failure_is_untracked_and_faults_without_cleanup() {
        let prepared = PreparedSimulationStaticArena::try_new(mixed_plan(
            7,
            &[9],
            vec![environment(0), grind(0)],
        ))
        .unwrap();
        let mut adapter = adapter(empty_router(0));

        let failure = adapter.register(&prepared, 0).unwrap_err();
        assert_eq!(failure.operation_index(), Some(0));
        let SimulationStaticArenaRegistrationError::Environment {
            registration: partial,
            ..
        } = failure
        else {
            panic!("expected Environment registration failure")
        };
        assert_eq!(partial.processed_operation_count(), 1);
        assert!(adapter.is_faulted());
        assert_eq!(adapter.live_registration_count(), 1);
        assert!(adapter.environment().context_snapshots().is_empty());
        assert!(adapter.grind().events.is_empty());

        let rejection = adapter.deregister(partial).unwrap_err();
        assert_eq!(
            rejection.kind(),
            SimulationStaticArenaDeregistrationErrorKind::AdapterFaulted
        );
        drop(rejection.into_registration());
        assert_eq!(adapter.published_volume_count(), 0);
        assert!(adapter.grind().events.is_empty());
    }

    #[test]
    fn all_ones_local_guid_is_context_data_not_a_terminator() {
        let plan = mixed_plan(u64::MAX, &[], vec![environment(0)]);
        assert_eq!(plan.asset().collision.instances[0].guid, 0x1111);
        assert_eq!(plan.asset().collision.instances[0].local_guid, u64::MAX);
        let prepared = PreparedSimulationStaticArena::try_new(plan).unwrap();
        let router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            EnvironmentUnlockSeed {
                locations_lock_status_raw: 1,
                contexts_in_native_iteration_order: vec![EnvironmentUnlockContextSeed {
                    local_guid: u64::MAX,
                    challenge_index: 0,
                    is_unlocked_env_instance_raw: 0,
                }],
            },
            1,
        )
        .unwrap();
        let mut adapter = adapter(router);

        let registration = adapter.register(&prepared, 0).unwrap();
        assert_eq!(adapter.published_volume_count(), 1);
        assert!(adapter.environment().context_snapshots()[0].has_attached_source);
        adapter.deregister(registration).unwrap();
        assert_eq!(adapter.published_volume_count(), 0);
        assert!(!adapter.environment().context_snapshots()[0].has_attached_source);
    }

    #[test]
    fn suppressed_environment_is_retained_for_unconditional_deregistration() {
        let prepared =
            PreparedSimulationStaticArena::try_new(mixed_plan(7, &[], vec![environment(0)]))
                .unwrap();
        let router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            EnvironmentUnlockSeed {
                locations_lock_status_raw: 0,
                contexts_in_native_iteration_order: vec![EnvironmentUnlockContextSeed {
                    local_guid: 7,
                    challenge_index: 0,
                    is_unlocked_env_instance_raw: 0,
                }],
            },
            0,
        )
        .unwrap();
        let mut adapter = adapter(router);

        let registration = adapter.register(&prepared, 0).unwrap();
        assert_eq!(adapter.published_volume_count(), 0);
        assert!(adapter.environment().context_snapshots()[0].has_attached_source);
        adapter.deregister(registration).unwrap();
        assert!(!adapter.environment().context_snapshots()[0].has_attached_source);
        assert_eq!(adapter.published_volume_count(), 0);
    }

    #[test]
    fn repeated_registration_occurrences_are_not_coalesced() {
        let id = 0xffff_ffff_0000_0001;
        let prepared = PreparedSimulationStaticArena::try_new(mixed_plan(
            7,
            &[id],
            vec![grind(0), environment(1)],
        ))
        .unwrap();
        let mut adapter = adapter(empty_router(2));

        let first = adapter.register(&prepared, 0).unwrap();
        let second = adapter.register(&prepared, 0).unwrap();
        assert_eq!(adapter.published_volume_count(), 2);
        assert_eq!(
            adapter.grind().events,
            [GrindEvent::Add(id), GrindEvent::Add(id)]
        );

        adapter.deregister(first).unwrap();
        assert_eq!(adapter.published_volume_count(), 1);
        adapter.deregister(second).unwrap();
        assert_eq!(adapter.published_volume_count(), 0);
        assert_eq!(
            adapter.grind().events,
            [
                GrindEvent::Add(id),
                GrindEvent::Add(id),
                GrindEvent::Remove(id),
                GrindEvent::Remove(id),
            ]
        );
    }

    #[test]
    fn dropping_occurrence_performs_no_hidden_deregistration() {
        let prepared =
            PreparedSimulationStaticArena::try_new(mixed_plan(7, &[], vec![environment(0)]))
                .unwrap();
        let mut adapter = adapter(empty_router(1));

        let registration = adapter.register(&prepared, 0).unwrap();
        drop(registration);

        assert_eq!(adapter.published_volume_count(), 1);
        assert_eq!(adapter.live_registration_count(), 1);
    }

    #[test]
    fn preparation_rejects_missing_grind_asset_before_effects_exist() {
        let error =
            PreparedSimulationStaticArena::try_new(mixed_plan(7, &[], vec![grind(0)])).unwrap_err();
        assert!(matches!(
            error,
            SimulationStaticArenaPrepareError::MissingGrindAsset {
                operation_index: 0,
                grind_asset_index: 0,
            }
        ));
    }

    #[test]
    fn foreign_domain_rejection_returns_token_without_effects() {
        let id = 0x55;
        let prepared =
            PreparedSimulationStaticArena::try_new(mixed_plan(7, &[id], vec![grind(0)])).unwrap();
        let mut owner = adapter(empty_router(0));
        let mut foreign = adapter(empty_router(0));
        let registration = owner.register(&prepared, 0).unwrap();

        let rejection = foreign.deregister(registration).unwrap_err();
        assert_eq!(
            rejection.kind(),
            SimulationStaticArenaDeregistrationErrorKind::ForeignDomain
        );
        assert!(foreign.grind().events.is_empty());
        assert_eq!(foreign.live_registration_count(), 0);
        let registration = rejection.into_registration();
        owner.deregister(registration).unwrap();
        assert_eq!(
            owner.grind().events,
            [GrindEvent::Add(id), GrindEvent::Remove(id)]
        );
    }

    #[test]
    fn repeated_source_capacity_failure_cannot_remove_prior_occurrence() {
        let prepared =
            PreparedSimulationStaticArena::try_new(mixed_plan(7, &[], vec![environment(0)]))
                .unwrap();
        let mut adapter = adapter(empty_router(1));
        let first = adapter.register(&prepared, 0).unwrap();
        assert_eq!(adapter.published_volume_count(), 1);

        let failure = adapter.register(&prepared, 0).unwrap_err();
        let SimulationStaticArenaRegistrationError::Environment {
            registration: partial,
            ..
        } = failure
        else {
            panic!("expected Environment registration failure")
        };
        assert!(adapter.is_faulted());
        assert_eq!(adapter.live_registration_count(), 2);
        assert_eq!(adapter.published_volume_count(), 1);

        let first_rejection = adapter.deregister(first).unwrap_err();
        assert_eq!(
            first_rejection.kind(),
            SimulationStaticArenaDeregistrationErrorKind::AdapterFaulted
        );
        assert_eq!(adapter.published_volume_count(), 1);
        let partial_rejection = adapter.deregister(partial).unwrap_err();
        assert_eq!(
            partial_rejection.kind(),
            SimulationStaticArenaDeregistrationErrorKind::AdapterFaulted
        );
        assert_eq!(adapter.published_volume_count(), 1);
        drop(first_rejection.into_registration());
        drop(partial_rejection.into_registration());
    }

    #[test]
    fn shared_root_failure_within_one_occurrence_remains_fail_stop() {
        let prepared = PreparedSimulationStaticArena::try_new(mixed_plan(
            7,
            &[],
            vec![environment(0), environment(1)],
        ))
        .unwrap();
        let mut adapter = adapter(empty_router(1));

        let failure = adapter.register(&prepared, 0).unwrap_err();
        let SimulationStaticArenaRegistrationError::Environment {
            operation_index,
            registration: partial,
            ..
        } = failure
        else {
            panic!("expected Environment registration failure")
        };
        assert_eq!(operation_index, 1);
        assert_eq!(partial.processed_operation_count(), 2);
        assert!(adapter.is_faulted());
        assert_eq!(adapter.published_volume_count(), 1);

        let rejection = adapter.deregister(partial).unwrap_err();
        assert_eq!(
            rejection.kind(),
            SimulationStaticArenaDeregistrationErrorKind::AdapterFaulted
        );
        assert_eq!(adapter.published_volume_count(), 1);
        drop(rejection.into_registration());
    }

    #[test]
    fn live_ledger_keeps_prepared_sources_alive_until_deregistration() {
        let prepared =
            PreparedSimulationStaticArena::try_new(mixed_plan(7, &[], vec![environment(0)]))
                .unwrap();
        let router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            EnvironmentUnlockSeed {
                locations_lock_status_raw: 1,
                contexts_in_native_iteration_order: vec![EnvironmentUnlockContextSeed {
                    local_guid: 7,
                    challenge_index: 0,
                    is_unlocked_env_instance_raw: 0,
                }],
            },
            1,
        )
        .unwrap();
        let mut adapter = adapter(router);
        let registration = adapter.register(&prepared, 0).unwrap();
        drop(prepared);

        assert!(adapter.environment().context_snapshots()[0].has_attached_source);
        adapter.deregister(registration).unwrap();
        assert!(!adapter.environment().context_snapshots()[0].has_attached_source);
        assert_eq!(adapter.live_registration_count(), 0);
    }
}
