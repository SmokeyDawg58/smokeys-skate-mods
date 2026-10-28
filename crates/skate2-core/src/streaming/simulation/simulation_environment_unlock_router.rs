//! Simulation-side unlock admission for static Environment collision roots.
//!
//! `UnlockableLocationMan::AddEnviroInstance` (`0x828b0068`) looks up an
//! Environment by its full local GUID, combines a signed challenge index with
//! a physically 64-bit `bitset<16>`, and conditionally calls the Simulation world
//! hook. `UpdateEnviroInstanceLocks` (`0x828afe60`) performs those world calls
//! before recording the new state. `RemEnviroInstance` (`0x828b0188`) clears a
//! found context before unconditionally removing the input instance.
//!
//! Context population and native fixed-hash bucket order have not yet been
//! recovered. Callers must therefore supply both the current mask and contexts
//! already arranged in native re-evaluation order. This module does not claim
//! to reconstruct their boot/career producer.

use crate::{
    physics::{TrajectoryGroundTriangle, TrajectorySweepHit},
    static_ground_pool::{
        StaticEnvironmentSource, StaticGroundCapacityError, StaticGroundPool,
        StaticGroundPoolCreateError, StaticGroundSlotSnapshot,
    },
};
use skate2_formats::collision::CollisionError;
use std::{error::Error, fmt, sync::Arc, sync::Weak};

/// Inline context capacity of the native
/// `fixed_hash_map<uint64_t, tUnlockableEnviroInstanceContext, 128, 129,
/// false>`.
pub const ENVIRONMENT_UNLOCK_CONTEXT_CAPACITY: usize = 128;

/// Unattached context metadata supplied by the still-external state producer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnvironmentUnlockContextSeed {
    pub local_guid: u64,
    pub challenge_index: i8,
    /// Raw native bool byte; zero is false and every nonzero value is true.
    pub is_unlocked_env_instance_raw: u8,
}

/// Complete explicit input needed to construct the bounded router.
#[derive(Debug, Eq, PartialEq)]
pub struct EnvironmentUnlockSeed {
    /// Complete stored bitset word. Admission addresses only bits 0 through 15.
    pub locations_lock_status_raw: u64,
    /// Entries must already follow the native fixed-map iteration order.
    pub contexts_in_native_iteration_order: Vec<EnvironmentUnlockContextSeed>,
}

#[derive(Debug)]
struct EnvironmentUnlockContext {
    local_guid: u64,
    challenge_index: i8,
    is_unlocked_env_instance_raw: u8,
    source: Option<Weak<StaticEnvironmentSource>>,
    is_in_world: bool,
}

/// Bounded diagnostic view that does not expose the stored source handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnvironmentUnlockContextSnapshot {
    pub local_guid: u64,
    pub challenge_index: i8,
    pub is_unlocked_env_instance_raw: u8,
    pub has_attached_source: bool,
    pub is_in_world: bool,
}

/// Concrete Simulation router that keeps admission state and ground effects
/// inseparable.
///
/// The native context stores a non-owning `tInstance*`. A `Weak` source is the
/// safe host equivalent: the resident collection remains the lifetime owner,
/// while a temporary upgrade protects each re-evaluation call. The contained
/// pool is deliberately not exposed mutably.
pub struct SimulationEnvironmentUnlockRouter {
    locations_lock_status_raw: u64,
    contexts: Vec<EnvironmentUnlockContext>,
    ground_pool: StaticGroundPool,
}

impl fmt::Debug for SimulationEnvironmentUnlockRouter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationEnvironmentUnlockRouter")
            .field("locations_lock_status_raw", &self.locations_lock_status_raw)
            .field("contexts", &self.context_snapshots())
            .field("ground_pool", &self.ground_pool)
            .finish()
    }
}

impl SimulationEnvironmentUnlockRouter {
    /// Reconstructs the native constructor state: empty context map and a
    /// fully zeroed 64-bit lock-status word.
    ///
    /// # Errors
    ///
    /// Returns a typed host allocation error if the lower pool cannot be
    /// preallocated.
    pub fn try_new_empty(
        ground_capacity: usize,
    ) -> Result<Self, EnvironmentUnlockRouterCreateError> {
        Self::try_new_with_external_seed(
            EnvironmentUnlockSeed {
                locations_lock_status_raw: 0,
                contexts_in_native_iteration_order: Vec::new(),
            },
            ground_capacity,
        )
    }

    /// Constructs an empty-attached router from explicit external state.
    ///
    /// The context vector is treated as the already-recovered native
    /// fixed-map iteration order. A linear lookup preserves full-width equality
    /// without pretending that the unknown insertion history and bucket chains
    /// have been reconstructed.
    ///
    /// # Errors
    ///
    /// Rejects more than 128 contexts, duplicate local GUIDs, or either host
    /// storage allocation failure without constructing a router.
    pub fn try_new_with_external_seed(
        seed: EnvironmentUnlockSeed,
        ground_capacity: usize,
    ) -> Result<Self, EnvironmentUnlockRouterCreateError> {
        let requested = seed.contexts_in_native_iteration_order.len();
        if requested > ENVIRONMENT_UNLOCK_CONTEXT_CAPACITY {
            return Err(EnvironmentUnlockRouterCreateError::ContextCapacity {
                capacity: ENVIRONMENT_UNLOCK_CONTEXT_CAPACITY,
                requested,
            });
        }
        for (index, context) in seed.contexts_in_native_iteration_order.iter().enumerate() {
            if seed.contexts_in_native_iteration_order[..index]
                .iter()
                .any(|prior| prior.local_guid == context.local_guid)
            {
                return Err(EnvironmentUnlockRouterCreateError::DuplicateLocalGuid(
                    context.local_guid,
                ));
            }
        }

        let mut contexts = Vec::new();
        contexts.try_reserve_exact(requested).map_err(|_| {
            EnvironmentUnlockRouterCreateError::ContextStorageAllocation { requested }
        })?;
        contexts.extend(
            seed.contexts_in_native_iteration_order
                .into_iter()
                .map(|context| EnvironmentUnlockContext {
                    local_guid: context.local_guid,
                    challenge_index: context.challenge_index,
                    is_unlocked_env_instance_raw: context.is_unlocked_env_instance_raw,
                    source: None,
                    is_in_world: false,
                }),
        );
        let ground_pool = StaticGroundPool::try_new(ground_capacity)
            .map_err(EnvironmentUnlockRouterCreateError::GroundPool)?;
        Ok(Self {
            locations_lock_status_raw: seed.locations_lock_status_raw,
            contexts,
            ground_pool,
        })
    }

    /// [`StaticGroundPool::warm_decodes_in_background`] for this router's pool.
    pub fn warm_ground_decodes(&mut self) {
        self.ground_pool.warm_decodes_in_background();
    }

    /// Mirrors the public immediate-add path at `0x828b02d8`/`0x828b0068`.
    ///
    /// An absent context adds immediately and is not inserted. A present
    /// context overwrites its non-owning source and recorded world state before
    /// the optional add, matching stores at `0x828b011c` and `0x828b0134`.
    /// Capacity is host-preflighted after those proven stores but before the
    /// lower add because native overflow behavior is unknown.
    ///
    /// # Errors
    ///
    /// Returns a typed capacity error when an admitted source cannot fit. A
    /// found context retains its native store-before-call prefix; an absent
    /// context remains untracked.
    pub fn register(
        &mut self,
        source: &Arc<StaticEnvironmentSource>,
    ) -> Result<(), EnvironmentUnlockRouterError> {
        let context_index = self.context_index(source.local_guid());
        let desired = context_index
            .is_none_or(|index| self.contexts[index].desired_state(self.locations_lock_status_raw));
        if let Some(index) = context_index {
            let context = &mut self.contexts[index];
            context.source = Some(Arc::downgrade(source));
            context.is_in_world = desired;
        }
        if desired {
            self.preflight_source(source)?;
            self.ground_pool
                .add(source)
                .map_err(EnvironmentUnlockRouterError::GroundCapacity)?;
        }
        Ok(())
    }

    /// Mirrors `RemEnviroInstance` (`0x828b0188`).
    ///
    /// A found context is cleared first. Removal then always receives the input
    /// source, even when the context was absent, its add was suppressed, or a
    /// later same-GUID registration overwrote its stored source.
    pub fn deregister(&mut self, source: &StaticEnvironmentSource) {
        if let Some(index) = self.context_index(source.local_guid()) {
            let context = &mut self.contexts[index];
            context.source = None;
            context.is_in_world = false;
        }
        self.ground_pool.remove(source);
    }

    /// Replaces the externally produced raw bitset word without performing
    /// world effects. Call [`Self::reevaluate`] for the separate native update.
    pub const fn set_locations_lock_status_raw(&mut self, status: u64) {
        self.locations_lock_status_raw = status;
    }

    /// Replaces one externally produced unlock flag without performing world
    /// effects or inserting a missing context.
    ///
    /// # Errors
    ///
    /// Rejects an unknown full local GUID without mutation.
    pub fn set_is_unlocked_env_instance_raw(
        &mut self,
        local_guid: u64,
        value: u8,
    ) -> Result<(), EnvironmentUnlockInputError> {
        let index = self
            .context_index(local_guid)
            .ok_or(EnvironmentUnlockInputError::MissingContext(local_guid))?;
        self.contexts[index].is_unlocked_env_instance_raw = value;
        Ok(())
    }

    /// Mirrors `UpdateEnviroInstanceLocks` (`0x828afe60`) in the explicitly
    /// supplied native context order.
    ///
    /// Each attached changed context performs its add/remove before recording
    /// the new state. Failure retains the completed prefix, leaves the current
    /// state unchanged, and does not visit later entries. An unchanged stale
    /// weak source is not dereferenced, matching the native no-call path.
    ///
    /// # Errors
    ///
    /// Returns a typed stale-source or host-capacity failure at the first
    /// context that cannot complete its required world transition.
    pub fn reevaluate(&mut self) -> Result<(), EnvironmentUnlockRouterError> {
        for index in 0..self.contexts.len() {
            let desired = self.contexts[index].desired_state(self.locations_lock_status_raw);
            if self.contexts[index].source.is_none() || desired == self.contexts[index].is_in_world
            {
                continue;
            }
            let local_guid = self.contexts[index].local_guid;
            let source = self.contexts[index]
                .source
                .as_ref()
                .and_then(Weak::upgrade)
                .ok_or(EnvironmentUnlockRouterError::StaleSource { local_guid })?;

            if desired {
                self.preflight_source(&source)?;
                self.ground_pool
                    .add(&source)
                    .map_err(EnvironmentUnlockRouterError::GroundCapacity)?;
            } else {
                self.ground_pool.remove(&source);
            }
            self.contexts[index].is_in_world = desired;
        }
        Ok(())
    }

    /// Mirrors `UnlockableLocationMan::Flush` (`0x828b0230`) using the
    /// explicitly supplied native context order.
    ///
    /// Only attached contexts whose raw recorded in-world byte is nonzero are
    /// removed. Each matching context is cleared before its stored source is
    /// removed, and context records are not erased.
    ///
    /// # Errors
    ///
    /// A stale source required by the first removable context stops the
    /// ordered walk without clearing that context or visiting later entries.
    pub fn flush(&mut self) -> Result<(), EnvironmentUnlockRouterError> {
        for index in 0..self.contexts.len() {
            if self.contexts[index].source.is_none() || !self.contexts[index].is_in_world {
                continue;
            }
            let local_guid = self.contexts[index].local_guid;
            let source = self.contexts[index]
                .source
                .as_ref()
                .and_then(Weak::upgrade)
                .ok_or(EnvironmentUnlockRouterError::StaleSource { local_guid })?;
            self.contexts[index].source = None;
            self.contexts[index].is_in_world = false;
            self.ground_pool.remove(&source);
        }
        Ok(())
    }

    /// Mirrors the proven state effects of `DeInit` (`0x828b02f8`): flush
    /// first, then remove every context and clear the complete raw mask word.
    ///
    /// # Errors
    ///
    /// Propagates the first flush failure without clearing the remaining map or
    /// mask as a safe host fail-stop. Native Flush/DeInit expose no recoverable
    /// failure result, so this error behavior is not claimed as native parity.
    pub fn deinitialize(&mut self) -> Result<(), EnvironmentUnlockRouterError> {
        self.flush()?;
        self.contexts.clear();
        self.locations_lock_status_raw = 0;
        Ok(())
    }

    #[must_use]
    pub const fn locations_lock_status_raw(&self) -> u64 {
        self.locations_lock_status_raw
    }

    #[must_use]
    /// Borrow the same admitted ground used by Environment registration.
    pub(crate) const fn ground_pool(&self) -> &StaticGroundPool {
        &self.ground_pool
    }
    #[must_use]
    pub fn published_volume_count(&self) -> usize {
        self.ground_pool.published_volume_count()
    }

    /// Sweeps only the static Environment roots currently admitted to ground.
    ///
    /// This is deliberately narrower than the complete Simulation-world
    /// trajectory query: DMO and vehicle providers are not routed here.
    ///
    /// # Errors
    ///
    /// Returns the ground pool's first collision-format error unchanged.
    pub fn sweep_static_ground_sphere_segment(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> Result<Option<TrajectorySweepHit>, CollisionError> {
        self.ground_pool
            .trajectory_sweep_sphere_segment(start, end, radius)
    }

    /// Collects trajectory ground-normal triangles only from the currently
    /// admitted static Environment roots.
    ///
    /// This is not the complete Simulation `TrajectoryCollisionWorld`: DMO
    /// and vehicle collections, plus their cross-provider ordering, are not
    /// routed through this narrow forwarding seam.
    ///
    /// # Errors
    ///
    /// Returns the ground pool's first collision-format error unchanged.
    pub fn static_ground_trajectory_triangles_overlapping_aabb(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
    ) -> Result<Vec<TrajectoryGroundTriangle>, CollisionError> {
        self.ground_pool
            .trajectory_ground_triangles_overlapping_aabb(minimum, maximum)
    }

    /// Ordered diagnostic view of the exact currently published ground roots.
    #[must_use]
    pub fn ground_snapshots(&self) -> Vec<StaticGroundSlotSnapshot> {
        self.ground_pool.snapshot()
    }

    #[must_use]
    pub fn context_count(&self) -> usize {
        self.contexts.len()
    }

    #[must_use]
    pub fn context_snapshots(&self) -> Vec<EnvironmentUnlockContextSnapshot> {
        self.contexts
            .iter()
            .map(|context| EnvironmentUnlockContextSnapshot {
                local_guid: context.local_guid,
                challenge_index: context.challenge_index,
                is_unlocked_env_instance_raw: context.is_unlocked_env_instance_raw,
                has_attached_source: context.source.is_some(),
                is_in_world: context.is_in_world,
            })
            .collect()
    }

    fn context_index(&self, local_guid: u64) -> Option<usize> {
        self.contexts
            .iter()
            .position(|context| context.local_guid == local_guid)
    }

    fn preflight_source(
        &self,
        source: &StaticEnvironmentSource,
    ) -> Result<(), EnvironmentUnlockRouterError> {
        let occupied = self.ground_pool.published_volume_count();
        let requested = source.root_count();
        if occupied
            .checked_add(requested)
            .is_none_or(|total| total > self.ground_pool.capacity())
        {
            return Err(EnvironmentUnlockRouterError::GroundCapacity(
                StaticGroundCapacityError {
                    capacity: self.ground_pool.capacity(),
                    occupied,
                    requested,
                },
            ));
        }
        Ok(())
    }
}

impl EnvironmentUnlockContext {
    fn desired_state(&self, locations_lock_status_raw: u64) -> bool {
        let mask_bit = u32::try_from(self.challenge_index)
            .ok()
            .filter(|&index| index < 16)
            .is_some_and(|index| locations_lock_status_raw & (1_u64 << index) != 0);
        mask_bit != (self.is_unlocked_env_instance_raw != 0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvironmentUnlockRouterCreateError {
    ContextCapacity { capacity: usize, requested: usize },
    DuplicateLocalGuid(u64),
    ContextStorageAllocation { requested: usize },
    GroundPool(StaticGroundPoolCreateError),
}

impl fmt::Display for EnvironmentUnlockRouterCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot construct Simulation Environment unlock router: {self:?}"
        )
    }
}

impl Error for EnvironmentUnlockRouterCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::GroundPool(source) => Some(source),
            Self::ContextCapacity { .. }
            | Self::DuplicateLocalGuid(_)
            | Self::ContextStorageAllocation { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvironmentUnlockRouterError {
    GroundCapacity(StaticGroundCapacityError),
    StaleSource { local_guid: u64 },
}

impl fmt::Display for EnvironmentUnlockRouterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Simulation Environment unlock transition failed: {self:?}"
        )
    }
}

impl Error for EnvironmentUnlockRouterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::GroundCapacity(source) => Some(source),
            Self::StaleSource { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvironmentUnlockInputError {
    MissingContext(u64),
}

impl fmt::Display for EnvironmentUnlockInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingContext(local_guid) => {
                write!(
                    formatter,
                    "no Environment unlock context for {local_guid:#018x}"
                )
            }
        }
    }
}

impl Error for EnvironmentUnlockInputError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::static_ground_pool::tests::environment_plan_for_test;

    fn source(asset_id: u64, local_guid: u64, root_count: usize) -> Arc<StaticEnvironmentSource> {
        Arc::new(
            StaticEnvironmentSource::from_operation(
                environment_plan_for_test(
                    asset_id,
                    local_guid,
                    &(0..root_count).collect::<Vec<_>>(),
                ),
                0,
            )
            .unwrap(),
        )
    }

    fn seed(status: u64, contexts: &[(u64, i8, u8)]) -> EnvironmentUnlockSeed {
        EnvironmentUnlockSeed {
            locations_lock_status_raw: status,
            contexts_in_native_iteration_order: contexts
                .iter()
                .map(
                    |&(local_guid, challenge_index, is_unlocked_env_instance_raw)| {
                        EnvironmentUnlockContextSeed {
                            local_guid,
                            challenge_index,
                            is_unlocked_env_instance_raw,
                        }
                    },
                )
                .collect(),
        }
    }

    #[test]
    fn absent_context_adds_immediately_without_inserting_and_deregisters() {
        let mut router = SimulationEnvironmentUnlockRouter::try_new_empty(1).unwrap();
        let source = source(1, 0xffff_ffff_ffff_ffff, 1);
        assert_eq!(source.guid(), 0x1111);
        assert_ne!(source.guid(), source.local_guid());

        router.register(&source).unwrap();

        assert_eq!(router.published_volume_count(), 1);
        assert!(router.context_snapshots().is_empty());
        router.deregister(&source);
        assert_eq!(router.published_volume_count(), 0);
    }

    #[test]
    fn admission_preserves_all_four_mask_and_unlocked_cases() {
        for (mask, unlocked, expected) in [
            (0_u64, 0_u8, false),
            (0, 2, true),
            (1, 0, true),
            (1, u8::MAX, false),
        ] {
            let mut router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
                seed(mask, &[(7, 0, unlocked)]),
                0,
            )
            .unwrap();
            let source = source(1, 7, 0);
            router.register(&source).unwrap();
            assert_eq!(router.context_snapshots()[0].is_in_world, expected);
        }
    }

    #[test]
    fn signed_challenge_bounds_use_false_mask_bit() {
        let mut router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            seed(
                u64::MAX,
                &[
                    (1, i8::MIN, 0),
                    (2, -1, 0),
                    (3, 15, 0),
                    (4, 16, 0),
                    (5, i8::MAX, 0),
                ],
            ),
            0,
        )
        .unwrap();
        for (asset_id, local_guid) in [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)] {
            router.register(&source(asset_id, local_guid, 0)).unwrap();
        }

        assert_eq!(
            router
                .context_snapshots()
                .iter()
                .map(|context| context.is_in_world)
                .collect::<Vec<_>>(),
            [false, false, true, false, false]
        );
    }

    #[test]
    fn upper_lock_status_bits_are_retained_but_not_selected() {
        let raw = (1_u64 << 63) | (1_u64 << 31);
        let mut router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            seed(raw, &[(7, 0, 0)]),
            0,
        )
        .unwrap();
        router.register(&source(1, 7, 0)).unwrap();

        assert_eq!(router.locations_lock_status_raw(), raw);
        assert!(!router.context_snapshots()[0].is_in_world);
    }

    #[test]
    fn lookup_preserves_full_guid_and_zero_is_ordinary() {
        let low = 0x1111_2222_dead_beef;
        let high = 0x9999_aaaa_dead_beef;
        let mut router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            seed(0, &[(low, 0, 0), (high, 0, 1), (0, 0, 1)]),
            0,
        )
        .unwrap();
        router.register(&source(1, high, 0)).unwrap();
        router.register(&source(2, 0, 0)).unwrap();

        assert_eq!(
            router
                .context_snapshots()
                .iter()
                .map(|context| (context.local_guid, context.has_attached_source))
                .collect::<Vec<_>>(),
            [(low, false), (high, true), (0, true)]
        );
    }

    #[test]
    fn same_guid_registration_overwrites_context_without_removing_old_source() {
        let mut router =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(0, &[(7, 0, 1)]), 2)
                .unwrap();
        let first = source(1, 7, 1);
        let second = source(2, 7, 1);
        router.register(&first).unwrap();
        router.register(&second).unwrap();
        assert_eq!(router.published_volume_count(), 2);

        router.deregister(&first);

        assert_eq!(router.published_volume_count(), 1);
        assert_eq!(
            router.context_snapshots()[0],
            EnvironmentUnlockContextSnapshot {
                local_guid: 7,
                challenge_index: 0,
                is_unlocked_env_instance_raw: 1,
                has_attached_source: false,
                is_in_world: false,
            }
        );
        router.reevaluate().unwrap();
        assert_eq!(router.published_volume_count(), 1);
        router.deregister(&second);
        assert_eq!(router.published_volume_count(), 0);
    }

    #[test]
    fn suppressed_registration_attaches_and_unconditional_remove_is_a_noop() {
        let mut router =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(0, &[(7, 0, 0)]), 1)
                .unwrap();
        let source = source(1, 7, 1);
        router.register(&source).unwrap();
        assert_eq!(router.published_volume_count(), 0);
        assert!(router.context_snapshots()[0].has_attached_source);
        assert!(!router.context_snapshots()[0].is_in_world);

        router.deregister(&source);

        assert_eq!(router.published_volume_count(), 0);
        assert!(!router.context_snapshots()[0].has_attached_source);
    }

    #[test]
    fn static_ground_sweep_is_empty_for_empty_and_suppressed_routers() {
        let empty = SimulationEnvironmentUnlockRouter::try_new_empty(0).unwrap();
        assert_eq!(
            empty.sweep_static_ground_sphere_segment([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25,),
            Ok(None)
        );

        let mut suppressed =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(0, &[(7, 0, 0)]), 1)
                .unwrap();
        let source = source(1, 7, 1);
        suppressed.register(&source).unwrap();
        assert_eq!(suppressed.published_volume_count(), 0);
        assert_eq!(
            suppressed.sweep_static_ground_sphere_segment([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25,),
            Ok(None)
        );
    }

    #[test]
    fn static_ground_sweep_forwards_admitted_errors_and_observes_deregistration() {
        let mut router = SimulationEnvironmentUnlockRouter::try_new_empty(1).unwrap();
        let source = source(1, 7, 1);
        router.register(&source).unwrap();

        assert_eq!(
            router.sweep_static_ground_sphere_segment([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25,),
            Err(CollisionError::Truncated {
                required: 48,
                actual: 0,
            })
        );

        router.deregister(&source);
        assert_eq!(
            router.sweep_static_ground_sphere_segment([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25,),
            Ok(None)
        );
    }

    #[test]
    fn static_ground_trajectory_box_is_empty_for_empty_and_suppressed_routers() {
        let empty = SimulationEnvironmentUnlockRouter::try_new_empty(0).unwrap();
        assert_eq!(
            empty.static_ground_trajectory_triangles_overlapping_aabb([-1.0; 3], [1.0; 3]),
            Ok(Vec::new())
        );

        let mut suppressed =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(0, &[(7, 0, 0)]), 1)
                .unwrap();
        let source = source(1, 7, 1);
        suppressed.register(&source).unwrap();
        assert_eq!(suppressed.published_volume_count(), 0);
        assert_eq!(
            suppressed.static_ground_trajectory_triangles_overlapping_aabb([-1.0; 3], [1.0; 3]),
            Ok(Vec::new())
        );
    }

    #[test]
    fn static_ground_trajectory_box_forwards_errors_and_observes_deregistration() {
        let mut router = SimulationEnvironmentUnlockRouter::try_new_empty(1).unwrap();
        let source = source(1, 7, 1);
        router.register(&source).unwrap();

        assert_eq!(
            router.static_ground_trajectory_triangles_overlapping_aabb([-1.0; 3], [1.0; 3]),
            Err(CollisionError::Truncated {
                required: 48,
                actual: 0,
            })
        );

        router.deregister(&source);
        assert_eq!(
            router.static_ground_trajectory_triangles_overlapping_aabb([-1.0; 3], [1.0; 3]),
            Ok(Vec::new())
        );
    }

    #[test]
    fn registration_capacity_failure_preserves_found_and_absent_native_prefixes() {
        let mut router =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(0, &[(7, 0, 1)]), 0)
                .unwrap();
        let present_source = source(1, 7, 1);

        assert_eq!(
            router.register(&present_source),
            Err(EnvironmentUnlockRouterError::GroundCapacity(
                StaticGroundCapacityError {
                    capacity: 0,
                    occupied: 0,
                    requested: 1,
                }
            ))
        );
        assert_eq!(router.published_volume_count(), 0);
        assert!(router.context_snapshots()[0].has_attached_source);
        assert!(router.context_snapshots()[0].is_in_world);

        let mut absent = SimulationEnvironmentUnlockRouter::try_new_empty(0).unwrap();
        assert_eq!(
            absent.register(&source(2, 8, 1)),
            Err(EnvironmentUnlockRouterError::GroundCapacity(
                StaticGroundCapacityError {
                    capacity: 0,
                    occupied: 0,
                    requested: 1,
                }
            ))
        );
        assert!(absent.context_snapshots().is_empty());
    }

    #[test]
    fn input_setters_have_no_effect_before_explicit_reevaluation() {
        let mut router =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(0, &[(7, 0, 0)]), 1)
                .unwrap();
        let source = source(1, 7, 1);
        router.register(&source).unwrap();

        router.set_locations_lock_status_raw(1);
        assert_eq!(router.locations_lock_status_raw(), 1);
        assert_eq!(router.published_volume_count(), 0);
        assert!(!router.context_snapshots()[0].is_in_world);
        router.reevaluate().unwrap();
        assert_eq!(router.published_volume_count(), 1);

        router.set_is_unlocked_env_instance_raw(7, 1).unwrap();
        assert_eq!(router.published_volume_count(), 1);
        assert!(router.context_snapshots()[0].is_in_world);
        router.reevaluate().unwrap();
        assert_eq!(router.published_volume_count(), 0);
        assert!(!router.context_snapshots()[0].is_in_world);
        assert_eq!(
            router.set_is_unlocked_env_instance_raw(8, 0),
            Err(EnvironmentUnlockInputError::MissingContext(8))
        );
    }

    #[test]
    fn reevaluation_keeps_completed_ordered_prefix_on_capacity_failure() {
        let second_guid = 0x2222_0000_0000_0001;
        let first_guid = 0x1111_0000_0000_0002;
        let mut router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            seed(0, &[(second_guid, 0, 0), (first_guid, 1, 0)]),
            1,
        )
        .unwrap();
        let second_in_numeric_order = source(1, second_guid, 1);
        let first_in_numeric_order = source(2, first_guid, 1);
        router.register(&second_in_numeric_order).unwrap();
        router.register(&first_in_numeric_order).unwrap();
        router.set_locations_lock_status_raw(3);

        assert_eq!(
            router.reevaluate(),
            Err(EnvironmentUnlockRouterError::GroundCapacity(
                StaticGroundCapacityError {
                    capacity: 1,
                    occupied: 1,
                    requested: 1,
                }
            ))
        );
        assert_eq!(router.published_volume_count(), 1);
        assert_eq!(
            router
                .context_snapshots()
                .iter()
                .map(|context| context.is_in_world)
                .collect::<Vec<_>>(),
            [true, false]
        );
    }

    #[test]
    fn earlier_ordered_remove_frees_capacity_for_later_add() {
        let mut router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            seed(1, &[(1, 0, 0), (2, 1, 0)]),
            1,
        )
        .unwrap();
        let first = source(1, 1, 1);
        let second = source(2, 2, 1);
        router.register(&first).unwrap();
        router.register(&second).unwrap();
        assert_eq!(router.published_volume_count(), 1);
        router.set_locations_lock_status_raw(2);

        router.reevaluate().unwrap();

        assert_eq!(router.published_volume_count(), 1);
        assert_eq!(
            router
                .context_snapshots()
                .iter()
                .map(|context| context.is_in_world)
                .collect::<Vec<_>>(),
            [false, true]
        );
    }

    #[test]
    fn stale_weak_is_ignored_when_unchanged_and_errors_when_transition_requires_source() {
        let mut router =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(0, &[(7, 0, 0)]), 1)
                .unwrap();
        let source = source(1, 7, 1);
        router.register(&source).unwrap();
        drop(source);

        router.reevaluate().unwrap();
        router.set_locations_lock_status_raw(1);
        assert_eq!(
            router.reevaluate(),
            Err(EnvironmentUnlockRouterError::StaleSource { local_guid: 7 })
        );
        assert_eq!(router.published_volume_count(), 0);
        assert!(!router.context_snapshots()[0].is_in_world);
    }

    #[test]
    fn admitted_stale_source_fails_safely_before_required_remove() {
        let mut router =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(1, &[(7, 0, 0)]), 1)
                .unwrap();
        let source = source(1, 7, 1);
        router.register(&source).unwrap();
        assert_eq!(router.published_volume_count(), 1);
        drop(source);
        router.set_locations_lock_status_raw(0);

        assert_eq!(
            router.reevaluate(),
            Err(EnvironmentUnlockRouterError::StaleSource { local_guid: 7 })
        );
        assert_eq!(router.published_volume_count(), 1);
        assert!(router.context_snapshots()[0].is_in_world);
    }

    #[test]
    fn stale_flush_and_deinitialize_fail_without_clearing_host_state() {
        let raw = (1_u64 << 63) | 1;
        let mut router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            seed(raw, &[(7, 0, 0)]),
            1,
        )
        .unwrap();
        let source = source(1, 7, 1);
        router.register(&source).unwrap();
        drop(source);
        let before = router.context_snapshots();

        assert_eq!(
            router.flush(),
            Err(EnvironmentUnlockRouterError::StaleSource { local_guid: 7 })
        );
        assert_eq!(router.context_snapshots(), before);
        assert_eq!(router.published_volume_count(), 1);
        assert_eq!(router.locations_lock_status_raw(), raw);

        assert_eq!(
            router.deinitialize(),
            Err(EnvironmentUnlockRouterError::StaleSource { local_guid: 7 })
        );
        assert_eq!(router.context_snapshots(), before);
        assert_eq!(router.published_volume_count(), 1);
        assert_eq!(router.locations_lock_status_raw(), raw);
    }

    #[test]
    fn ordinary_drop_does_not_take_strong_source_ownership() {
        let source = source(1, 7, 1);
        let weak = Arc::downgrade(&source);
        let mut router =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(1, &[(7, 0, 0)]), 1)
                .unwrap();
        router.register(&source).unwrap();
        assert_eq!(Arc::strong_count(&source), 1);

        drop(router);

        assert_eq!(Arc::strong_count(&source), 1);
        drop(source);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn flush_removes_only_attached_in_world_sources_without_erasing_contexts() {
        let mut router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            seed(1, &[(1, 0, 0), (2, 1, 0)]),
            1,
        )
        .unwrap();
        let admitted = source(1, 1, 1);
        let suppressed = source(2, 2, 1);
        router.register(&admitted).unwrap();
        router.register(&suppressed).unwrap();

        router.flush().unwrap();

        assert_eq!(router.published_volume_count(), 0);
        assert_eq!(router.context_count(), 2);
        assert_eq!(
            router
                .context_snapshots()
                .iter()
                .map(|context| (context.has_attached_source, context.is_in_world))
                .collect::<Vec<_>>(),
            [(false, false), (true, false)]
        );
    }

    #[test]
    fn flush_uses_overwritten_stored_source_and_leaves_older_publication() {
        let mut router =
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(0, &[(7, 0, 1)]), 2)
                .unwrap();
        let first = source(1, 7, 1);
        let second = source(2, 7, 1);
        router.register(&first).unwrap();
        router.register(&second).unwrap();

        router.flush().unwrap();

        assert_eq!(router.published_volume_count(), 1);
        assert!(!router.context_snapshots()[0].has_attached_source);
        router.deregister(&first);
        assert_eq!(router.published_volume_count(), 0);
    }

    #[test]
    fn deinitialize_flushes_then_clears_contexts_and_full_status_word() {
        let raw = (1_u64 << 63) | 1;
        let mut router = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            seed(raw, &[(7, 0, 0)]),
            1,
        )
        .unwrap();
        let source = source(1, 7, 1);
        router.register(&source).unwrap();

        router.deinitialize().unwrap();

        assert_eq!(router.published_volume_count(), 0);
        assert_eq!(router.context_count(), 0);
        assert_eq!(router.locations_lock_status_raw(), 0);
    }

    #[test]
    fn deinitialize_does_not_invent_tracking_for_absent_context_publications() {
        let mut router = SimulationEnvironmentUnlockRouter::try_new_empty(1).unwrap();
        let source = source(1, 7, 1);
        router.register(&source).unwrap();

        router.deinitialize().unwrap();

        assert_eq!(router.published_volume_count(), 1);
        assert_eq!(router.context_count(), 0);
        router.deregister(&source);
        assert_eq!(router.published_volume_count(), 0);
    }

    #[test]
    fn seed_rejects_duplicates_and_more_than_native_capacity() {
        assert_eq!(
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
                seed(0, &[(7, 0, 0), (7, 1, 1)]),
                0,
            )
            .unwrap_err(),
            EnvironmentUnlockRouterCreateError::DuplicateLocalGuid(7)
        );
        let contexts = (0..=ENVIRONMENT_UNLOCK_CONTEXT_CAPACITY)
            .map(|index| (u64::try_from(index).unwrap(), 0, 0))
            .collect::<Vec<_>>();
        assert_eq!(
            SimulationEnvironmentUnlockRouter::try_new_with_external_seed(seed(0, &contexts), 0)
                .unwrap_err(),
            EnvironmentUnlockRouterCreateError::ContextCapacity {
                capacity: ENVIRONMENT_UNLOCK_CONTEXT_CAPACITY,
                requested: ENVIRONMENT_UNLOCK_CONTEXT_CAPACITY + 1,
            }
        );
    }
}
