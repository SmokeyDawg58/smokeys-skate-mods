//! Load-only `cAssetActivationManager` owner for global Location arenas.
//!
//! This composes the exact eight fixed-slot admission and reachable global
//! filename-load states with stable asynchronous completion authority. Unload
//! states 8 through 14 belong to the shared native base but are intentionally
//! absent from this narrow owner until a caller requires them.

use crate::{
    arena_registration_ref_counter::{ArenaRegistrationAddTransition, ArenaRegistrationRefCounter},
    collection_resident_streamer::CollectionId,
};
use skate2_formats::hash::rw_hash64_string;
use std::{
    error::Error,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

pub const GLOBAL_ACTIVATION_SLOT_COUNT: usize = 8;
const NATIVE_PATH_PAYLOAD_MAXIMUM: usize = 255;
const RETIRED_SIGNAL_STATE: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum GlobalLocationActivationState {
    Load = 0,
    LoadingArenaAsset = 4,
    LoadingArenaAssetComplete = 5,
    ActArenaAsset = 6,
    ActArenaAssetComplete = 7,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalLocationLoadRequest {
    path: String,
    asset_id: u64,
}

impl GlobalLocationLoadRequest {
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub const fn asset_id(&self) -> u64 {
        self.asset_id
    }
}

#[derive(Debug)]
struct SlotSignal {
    state: AtomicU32,
}

#[derive(Debug)]
struct ActiveGlobalLocationLoad {
    request: GlobalLocationLoadRequest,
    generation: u64,
    signal: Arc<SlotSignal>,
}

#[derive(Debug)]
struct CompletionKey {
    slot: u8,
    generation: u64,
    asset_id: u64,
    signal: Arc<SlotSignal>,
}

#[derive(Debug)]
pub struct ArenaLoadCompletion(CompletionKey);

impl ArenaLoadCompletion {
    /// Publishes the native load-complete callback transition `4 -> 5`.
    ///
    /// # Errors
    ///
    /// Returns a safe-host stale/phase error instead of allowing an old
    /// callback to mutate a reused fixed slot.
    pub fn complete(self) -> Result<(), ActivationCompletionError> {
        self.0.transition(
            GlobalLocationActivationState::LoadingArenaAsset,
            GlobalLocationActivationState::LoadingArenaAssetComplete,
        )
    }

    /// Publishes the native load-failed callback transition `4 -> 3`.
    ///
    /// Raw state three is kept outside the successful-state enum because the
    /// global manager treats it as a fatal missing-asset path.
    ///
    /// # Errors
    ///
    /// Returns a safe-host stale/phase error.
    pub fn fail(self) -> Result<(), ActivationCompletionError> {
        self.0
            .transition_raw(GlobalLocationActivationState::LoadingArenaAsset as u32, 3)
    }
}

#[derive(Debug)]
pub struct ArenaRegistrationCompletion(CompletionKey);

impl ArenaRegistrationCompletion {
    /// Publishes the native registration callback transition `6 -> 7`.
    ///
    /// # Errors
    ///
    /// Returns a safe-host stale/phase error.
    pub fn complete(self) -> Result<(), ActivationCompletionError> {
        self.0.transition(
            GlobalLocationActivationState::ActArenaAsset,
            GlobalLocationActivationState::ActArenaAssetComplete,
        )
    }
}

impl CompletionKey {
    fn transition(
        self,
        expected: GlobalLocationActivationState,
        next: GlobalLocationActivationState,
    ) -> Result<(), ActivationCompletionError> {
        self.transition_raw(expected as u32, next as u32)
    }

    fn transition_raw(self, expected: u32, next: u32) -> Result<(), ActivationCompletionError> {
        match self
            .signal
            .state
            .compare_exchange(expected, next, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => Ok(()),
            Err(RETIRED_SIGNAL_STATE) => Err(ActivationCompletionError::Retired {
                slot: self.slot,
                generation: self.generation,
                asset_id: self.asset_id,
            }),
            Err(actual) => Err(ActivationCompletionError::WrongPhase {
                slot: self.slot,
                generation: self.generation,
                asset_id: self.asset_id,
                expected,
                actual,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationCompletionError {
    Retired {
        slot: u8,
        generation: u64,
        asset_id: u64,
    },
    WrongPhase {
        slot: u8,
        generation: u64,
        asset_id: u64,
        expected: u32,
        actual: u32,
    },
}

impl fmt::Display for ActivationCompletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "global Location activation callback rejected: {self:?}"
        )
    }
}

impl Error for ActivationCompletionError {}

pub trait GlobalLocationActivationEffects {
    type Error;

    /// Starts Arena asset loading after raw state four has been stored.
    ///
    /// # Errors
    ///
    /// Returns a checked host backend failure.
    fn begin_arena_load(
        &mut self,
        request: &GlobalLocationLoadRequest,
        completion: ArenaLoadCompletion,
    ) -> Result<(), Self::Error>;

    /// Starts simulation registration after raw state six has been stored.
    ///
    /// # Errors
    ///
    /// Returns a checked host backend failure.
    fn begin_sim_registration(
        &mut self,
        asset_id: u64,
        completion: ArenaRegistrationCompletion,
    ) -> Result<(), Self::Error>;

    /// Mirrors the global manager's fatal `AssetMissing` hook.
    ///
    /// # Errors
    ///
    /// Returns a checked host backend failure before the owner latches its own
    /// missing-asset fault.
    fn asset_missing(&mut self, asset_id: u64) -> Result<(), Self::Error>;

    /// Mirrors `AssetLoaded`, which increments the allocated-asset count.
    ///
    /// # Errors
    ///
    /// Returns a checked host backend failure.
    fn asset_loaded(&mut self, asset_id: u64) -> Result<(), Self::Error>;

    /// Mirrors `AssetAdded`, which sets the manager record's `Loaded` byte.
    ///
    /// # Errors
    ///
    /// Returns a checked host backend failure.
    fn asset_added(&mut self, asset_id: u64) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlobalLocationActivationEffect {
    BeginArenaLoad,
    BeginSimRegistration,
    AssetMissing,
    AssetLoaded,
    AssetAdded,
}

#[derive(Debug)]
pub enum GlobalLocationActivationError<E> {
    GenerationExhausted,
    FreeSlotInvariant,
    MissingReference {
        asset_id: u64,
    },
    MissingAsset {
        asset_id: u64,
    },
    Runtime {
        asset_id: u64,
        effect: GlobalLocationActivationEffect,
        source: E,
    },
}

impl<E: fmt::Display> fmt::Display for GlobalLocationActivationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GenerationExhausted => {
                formatter.write_str("global Location activation generation exhausted")
            }
            Self::FreeSlotInvariant => {
                formatter.write_str("global Location activation free-slot invariant failed")
            }
            Self::MissingReference { asset_id } => write!(
                formatter,
                "global Location activation reference {asset_id:#018x} was absent"
            ),
            Self::MissingAsset { asset_id } => {
                write!(
                    formatter,
                    "global Location arena {asset_id:#018x} failed to load"
                )
            }
            Self::Runtime {
                asset_id,
                effect,
                source,
            } => write!(
                formatter,
                "global Location activation {effect:?} failed for {asset_id:#018x}: {source}"
            ),
        }
    }
}

impl<E: Error + 'static> Error for GlobalLocationActivationError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime { source, .. } => Some(source),
            Self::GenerationExhausted
            | Self::FreeSlotInvariant
            | Self::MissingReference { .. }
            | Self::MissingAsset { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlobalLocationEnqueueError {
    InteriorNul,
    PathTooLong { byte_len: usize },
    Allocation,
}

impl fmt::Display for GlobalLocationEnqueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InteriorNul => formatter.write_str("global Location path contains interior NUL"),
            Self::PathTooLong { byte_len } => write!(
                formatter,
                "global Location path has {byte_len} bytes; maximum is {NATIVE_PATH_PAYLOAD_MAXIMUM}"
            ),
            Self::Allocation => formatter.write_str("global Location request allocation failed"),
        }
    }
}

impl Error for GlobalLocationEnqueueError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveGlobalLocationSnapshot {
    pub slot: u8,
    pub generation: u64,
    pub asset_id: u64,
    pub path: String,
    pub raw_state: u32,
}

/// Stable eight-slot global-location activation owner.
///
/// Reconstructs the global filename-load subset of
/// `cAssetActivationManager::UpdateRequests`
/// (`0x82d5d468..0x82d5d8dc`).
#[derive(Debug)]
pub struct GlobalLocationActivationMachine<E> {
    queued: Vec<GlobalLocationLoadRequest>,
    active: Vec<u8>,
    slots: [Option<ActiveGlobalLocationLoad>; GLOBAL_ACTIVATION_SLOT_COUNT],
    free: Vec<u8>,
    references: ArenaRegistrationRefCounter,
    next_generation: u64,
    fault: Option<Arc<GlobalLocationActivationError<E>>>,
}

impl<E> Default for GlobalLocationActivationMachine<E> {
    fn default() -> Self {
        Self {
            queued: Vec::with_capacity(4),
            active: Vec::with_capacity(GLOBAL_ACTIVATION_SLOT_COUNT),
            slots: std::array::from_fn(|_| None),
            free: (0_u8..8).collect(),
            references: ArenaRegistrationRefCounter::default(),
            next_generation: 0,
            fault: None,
        }
    }
}

impl<E> Drop for GlobalLocationActivationMachine<E> {
    fn drop(&mut self) {
        for active in self.slots.iter().flatten() {
            active
                .signal
                .state
                .store(RETIRED_SIGNAL_STATE, Ordering::SeqCst);
        }
    }
}

impl<E> GlobalLocationActivationMachine<E> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconstructs the filename `LoadArenaAsset` overload
    /// (`0x82d5bf40..0x82d5bfe8`): hash the original path, convert forward
    /// slashes in the queued copy, and append without duplicate suppression.
    ///
    /// # Errors
    ///
    /// Rejects native fixed-buffer hazards and checked allocation failure.
    pub fn enqueue_filename_load(&mut self, path: &str) -> Result<u64, GlobalLocationEnqueueError> {
        if path.as_bytes().contains(&0) {
            return Err(GlobalLocationEnqueueError::InteriorNul);
        }
        if path.len() > NATIVE_PATH_PAYLOAD_MAXIMUM {
            return Err(GlobalLocationEnqueueError::PathTooLong {
                byte_len: path.len(),
            });
        }
        self.queued
            .try_reserve(1)
            .map_err(|_| GlobalLocationEnqueueError::Allocation)?;
        let asset_id = rw_hash64_string(path);
        self.queued.push(GlobalLocationLoadRequest {
            path: path.replace('/', "\\"),
            asset_id,
        });
        Ok(asset_id)
    }

    #[must_use]
    pub fn is_pending(&self) -> bool {
        !self.active.is_empty() || !self.queued.is_empty()
    }

    #[must_use]
    pub fn queued_count(&self) -> usize {
        self.queued.len()
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    #[must_use]
    pub fn free_slots(&self) -> &[u8] {
        &self.free
    }

    #[must_use]
    pub fn reference_count(&self, asset_id: u64) -> Option<u32> {
        self.references.count(CollectionId(asset_id))
    }

    #[must_use]
    pub fn fault(&self) -> Option<&Arc<GlobalLocationActivationError<E>>> {
        self.fault.as_ref()
    }

    #[must_use]
    /// # Panics
    ///
    /// Panics only if an internal active-vector/slot pairing invariant was
    /// violated by this owner.
    pub fn active_snapshot(&self) -> Vec<ActiveGlobalLocationSnapshot> {
        self.active
            .iter()
            .map(|&slot| {
                let active = self.slots[usize::from(slot)]
                    .as_ref()
                    .expect("active slot vector and storage remain paired");
                ActiveGlobalLocationSnapshot {
                    slot,
                    generation: active.generation,
                    asset_id: active.request.asset_id,
                    path: active.request.path.clone(),
                    raw_state: active.signal.state.load(Ordering::SeqCst),
                }
            })
            .collect()
    }

    /// Drives one native public `Update` call.
    ///
    /// Admission and active scan repeat only after at least one retirement.
    /// Synchronous callbacks alone do not cause a same-call revisit.
    ///
    /// # Errors
    ///
    /// Latches and returns the first checked generation, missing-asset, or
    /// backend failure. Later calls replay the same `Arc` and perform no work.
    /// On the native debug-fatal state-three path, Rust retains the failed
    /// active slot after `RemAsset`/`AssetMissing` instead of guessing how
    /// execution continues after `DebugBreak`.
    pub fn update<R>(
        &mut self,
        runtime: &mut R,
    ) -> Result<(), Arc<GlobalLocationActivationError<E>>>
    where
        R: GlobalLocationActivationEffects<Error = E>,
    {
        if let Some(fault) = &self.fault {
            return Err(Arc::clone(fault));
        }
        loop {
            self.admit()?;
            let mut removed_any = false;
            let mut active_index = 0;
            while active_index < self.active.len() {
                if self.update_active(active_index, runtime)? {
                    removed_any = true;
                } else {
                    active_index += 1;
                }
            }
            if !removed_any {
                return Ok(());
            }
        }
    }

    fn admit(&mut self) -> Result<(), Arc<GlobalLocationActivationError<E>>> {
        let mut queued_index = 0;
        while queued_index < self.queued.len() && self.active.len() < GLOBAL_ACTIVATION_SLOT_COUNT {
            let asset_id = self.queued[queued_index].asset_id;
            if self.active.iter().any(|&slot| {
                self.slots[usize::from(slot)]
                    .as_ref()
                    .is_some_and(|active| active.request.asset_id == asset_id)
            }) {
                queued_index += 1;
                continue;
            }
            let Some(slot) = self.free.pop() else {
                return Err(self.latch(GlobalLocationActivationError::FreeSlotInvariant));
            };
            let Some(generation) = self.next_generation.checked_add(1) else {
                self.free.push(slot);
                return Err(self.latch(GlobalLocationActivationError::GenerationExhausted));
            };
            self.next_generation = generation;
            let request = self.queued.remove(queued_index);
            self.slots[usize::from(slot)] = Some(ActiveGlobalLocationLoad {
                request,
                generation,
                signal: Arc::new(SlotSignal {
                    state: AtomicU32::new(GlobalLocationActivationState::Load as u32),
                }),
            });
            self.active.push(slot);
        }
        Ok(())
    }

    fn update_active<R>(
        &mut self,
        active_index: usize,
        runtime: &mut R,
    ) -> Result<bool, Arc<GlobalLocationActivationError<E>>>
    where
        R: GlobalLocationActivationEffects<Error = E>,
    {
        let slot = self.active[active_index];
        let (asset_id, raw_state, completion_key) = {
            let active = self.slots[usize::from(slot)]
                .as_ref()
                .expect("active slot vector and storage remain paired");
            (
                active.request.asset_id,
                active.signal.state.load(Ordering::SeqCst),
                CompletionKey {
                    slot,
                    generation: active.generation,
                    asset_id: active.request.asset_id,
                    signal: Arc::clone(&active.signal),
                },
            )
        };
        match raw_state {
            0 => {
                if self.references.add_asset(CollectionId(asset_id))
                    == ArenaRegistrationAddTransition::Retained
                {
                    self.call_effect(asset_id, GlobalLocationActivationEffect::AssetAdded, || {
                        runtime.asset_added(asset_id)
                    })?;
                    self.retire(active_index);
                    return Ok(true);
                }
                let result = {
                    let active = self.slots[usize::from(slot)]
                        .as_ref()
                        .expect("active slot remains installed");
                    active.signal.state.store(
                        GlobalLocationActivationState::LoadingArenaAsset as u32,
                        Ordering::SeqCst,
                    );
                    runtime.begin_arena_load(&active.request, ArenaLoadCompletion(completion_key))
                };
                if let Err(source) = result {
                    return Err(self.latch(GlobalLocationActivationError::Runtime {
                        asset_id,
                        effect: GlobalLocationActivationEffect::BeginArenaLoad,
                        source,
                    }));
                }
                Ok(false)
            }
            3 => {
                if self
                    .references
                    .remove_asset(CollectionId(asset_id))
                    .is_err()
                {
                    return Err(
                        self.latch(GlobalLocationActivationError::MissingReference { asset_id })
                    );
                }
                self.call_effect(
                    asset_id,
                    GlobalLocationActivationEffect::AssetMissing,
                    || runtime.asset_missing(asset_id),
                )?;
                Err(self.latch(GlobalLocationActivationError::MissingAsset { asset_id }))
            }
            5 => {
                let active = self.slots[usize::from(slot)]
                    .as_ref()
                    .expect("active slot remains installed");
                active.signal.state.store(
                    GlobalLocationActivationState::ActArenaAsset as u32,
                    Ordering::SeqCst,
                );
                self.call_effect(
                    asset_id,
                    GlobalLocationActivationEffect::BeginSimRegistration,
                    || {
                        runtime.begin_sim_registration(
                            asset_id,
                            ArenaRegistrationCompletion(completion_key),
                        )
                    },
                )?;
                Ok(false)
            }
            7 => {
                self.call_effect(
                    asset_id,
                    GlobalLocationActivationEffect::AssetLoaded,
                    || runtime.asset_loaded(asset_id),
                )?;
                self.call_effect(asset_id, GlobalLocationActivationEffect::AssetAdded, || {
                    runtime.asset_added(asset_id)
                })?;
                self.retire(active_index);
                Ok(true)
            }
            4 | 6 => Ok(false),
            _ => unreachable!("private completion handles produce only global-load states"),
        }
    }

    fn call_effect<F>(
        &mut self,
        asset_id: u64,
        effect: GlobalLocationActivationEffect,
        call: F,
    ) -> Result<(), Arc<GlobalLocationActivationError<E>>>
    where
        F: FnOnce() -> Result<(), E>,
    {
        call().map_err(|source| {
            self.latch(GlobalLocationActivationError::Runtime {
                asset_id,
                effect,
                source,
            })
        })
    }

    fn retire(&mut self, active_index: usize) {
        let slot = self.active.remove(active_index);
        let active = self.slots[usize::from(slot)]
            .take()
            .expect("active slot vector and storage remain paired");
        active
            .signal
            .state
            .store(RETIRED_SIGNAL_STATE, Ordering::SeqCst);
        self.free.push(slot);
    }

    fn latch(
        &mut self,
        fault: GlobalLocationActivationError<E>,
    ) -> Arc<GlobalLocationActivationError<E>> {
        let fault = Arc::new(fault);
        self.fault = Some(Arc::clone(&fault));
        fault
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl Error for TestError {}

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        BeginLoad(u64),
        BeginRegistration(u64),
        Missing(u64),
        Loaded(u64),
        Added(u64),
    }

    #[derive(Default)]
    struct Runtime {
        events: Vec<Event>,
        loads: BTreeMap<u64, ArenaLoadCompletion>,
        registrations: BTreeMap<u64, ArenaRegistrationCompletion>,
        synchronous_load: bool,
        synchronous_registration: bool,
        fail_effect: Option<GlobalLocationActivationEffect>,
    }

    impl Runtime {
        fn effect(&self, effect: GlobalLocationActivationEffect) -> Result<(), TestError> {
            if self.fail_effect == Some(effect) {
                Err(TestError("injected effect failure"))
            } else {
                Ok(())
            }
        }
    }

    impl GlobalLocationActivationEffects for Runtime {
        type Error = TestError;

        fn begin_arena_load(
            &mut self,
            request: &GlobalLocationLoadRequest,
            completion: ArenaLoadCompletion,
        ) -> Result<(), Self::Error> {
            self.effect(GlobalLocationActivationEffect::BeginArenaLoad)?;
            self.events.push(Event::BeginLoad(request.asset_id()));
            if self.synchronous_load {
                completion.complete().unwrap();
            } else {
                assert!(self.loads.insert(request.asset_id(), completion).is_none());
            }
            Ok(())
        }

        fn begin_sim_registration(
            &mut self,
            asset_id: u64,
            completion: ArenaRegistrationCompletion,
        ) -> Result<(), Self::Error> {
            self.effect(GlobalLocationActivationEffect::BeginSimRegistration)?;
            self.events.push(Event::BeginRegistration(asset_id));
            if self.synchronous_registration {
                completion.complete().unwrap();
            } else {
                assert!(self.registrations.insert(asset_id, completion).is_none());
            }
            Ok(())
        }

        fn asset_missing(&mut self, asset_id: u64) -> Result<(), Self::Error> {
            self.effect(GlobalLocationActivationEffect::AssetMissing)?;
            self.events.push(Event::Missing(asset_id));
            Ok(())
        }

        fn asset_loaded(&mut self, asset_id: u64) -> Result<(), Self::Error> {
            self.effect(GlobalLocationActivationEffect::AssetLoaded)?;
            self.events.push(Event::Loaded(asset_id));
            Ok(())
        }

        fn asset_added(&mut self, asset_id: u64) -> Result<(), Self::Error> {
            self.effect(GlobalLocationActivationEffect::AssetAdded)?;
            self.events.push(Event::Added(asset_id));
            Ok(())
        }
    }

    fn states(machine: &GlobalLocationActivationMachine<TestError>) -> Vec<(u8, u32)> {
        machine
            .active_snapshot()
            .into_iter()
            .map(|active| (active.slot, active.raw_state))
            .collect()
    }

    #[test]
    fn complete_async_load_registration_and_hooks_follow_exact_update_boundaries() {
        let path = "data\\content\\global_locators\\one.rx2";
        let id = rw_hash64_string(path);
        let mut machine = GlobalLocationActivationMachine::<TestError>::new();
        let mut runtime = Runtime::default();
        assert_eq!(machine.enqueue_filename_load(path).unwrap(), id);

        machine.update(&mut runtime).unwrap();
        assert_eq!(states(&machine), [(7, 4)]);
        assert_eq!(runtime.events, [Event::BeginLoad(id)]);
        assert_eq!(machine.reference_count(id), Some(1));

        runtime.loads.remove(&id).unwrap().complete().unwrap();
        machine.update(&mut runtime).unwrap();
        assert_eq!(states(&machine), [(7, 6)]);
        assert_eq!(
            runtime.events,
            [Event::BeginLoad(id), Event::BeginRegistration(id)]
        );

        runtime
            .registrations
            .remove(&id)
            .unwrap()
            .complete()
            .unwrap();
        machine.update(&mut runtime).unwrap();
        assert!(!machine.is_pending());
        assert_eq!(machine.free_slots(), [0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(
            runtime.events,
            [
                Event::BeginLoad(id),
                Event::BeginRegistration(id),
                Event::Loaded(id),
                Event::Added(id),
            ]
        );
        assert_eq!(machine.reference_count(id), Some(1));
    }

    #[test]
    fn synchronous_callbacks_wait_for_later_updates_without_a_retirement() {
        let mut machine = GlobalLocationActivationMachine::<TestError>::new();
        let mut runtime = Runtime {
            synchronous_load: true,
            synchronous_registration: true,
            ..Runtime::default()
        };
        machine.enqueue_filename_load("one.rx2").unwrap();

        machine.update(&mut runtime).unwrap();
        assert_eq!(states(&machine), [(7, 5)]);
        machine.update(&mut runtime).unwrap();
        assert_eq!(states(&machine), [(7, 7)]);
        machine.update(&mut runtime).unwrap();
        assert!(!machine.is_pending());
    }

    #[test]
    fn admission_uses_slot_seven_first_and_leaves_the_ninth_queued() {
        let mut machine = GlobalLocationActivationMachine::<TestError>::new();
        let mut runtime = Runtime::default();
        for ordinal in 0..9 {
            machine
                .enqueue_filename_load(&format!("locator_{ordinal}.rx2"))
                .unwrap();
        }

        machine.update(&mut runtime).unwrap();

        assert_eq!(machine.active_count(), 8);
        assert_eq!(machine.queued_count(), 1);
        assert_eq!(
            machine
                .active_snapshot()
                .iter()
                .map(|active| active.slot)
                .collect::<Vec<_>>(),
            [7, 6, 5, 4, 3, 2, 1, 0]
        );
        assert!(machine.free_slots().is_empty());
    }

    #[test]
    fn duplicate_id_waits_while_later_distinct_request_is_admitted() {
        let mut machine = GlobalLocationActivationMachine::<TestError>::new();
        let mut runtime = Runtime::default();
        let duplicate = machine.enqueue_filename_load("same.rx2").unwrap();
        machine.enqueue_filename_load("same.rx2").unwrap();
        let distinct = machine.enqueue_filename_load("later.rx2").unwrap();

        machine.update(&mut runtime).unwrap();

        assert_eq!(machine.active_count(), 2);
        assert_eq!(machine.queued_count(), 1);
        assert_eq!(
            machine
                .active_snapshot()
                .iter()
                .map(|active| active.asset_id)
                .collect::<Vec<_>>(),
            [duplicate, distinct]
        );
    }

    #[test]
    fn completed_distinct_requests_refill_duplicate_as_a_retained_use_same_call() {
        let mut machine = GlobalLocationActivationMachine::<TestError>::new();
        let mut runtime = Runtime::default();
        let duplicate = machine.enqueue_filename_load("same.rx2").unwrap();
        machine.enqueue_filename_load("same.rx2").unwrap();
        let distinct = machine.enqueue_filename_load("later.rx2").unwrap();
        machine.update(&mut runtime).unwrap();
        runtime
            .loads
            .remove(&duplicate)
            .unwrap()
            .complete()
            .unwrap();
        runtime.loads.remove(&distinct).unwrap().complete().unwrap();
        machine.update(&mut runtime).unwrap();
        runtime
            .registrations
            .remove(&duplicate)
            .unwrap()
            .complete()
            .unwrap();
        runtime
            .registrations
            .remove(&distinct)
            .unwrap()
            .complete()
            .unwrap();

        machine.update(&mut runtime).unwrap();

        assert!(!machine.is_pending());
        assert_eq!(machine.reference_count(duplicate), Some(2));
        assert_eq!(machine.reference_count(distinct), Some(1));
        assert_eq!(
            runtime
                .events
                .iter()
                .filter(|event| **event == Event::BeginLoad(duplicate))
                .count(),
            1
        );
        assert_eq!(runtime.events.last(), Some(&Event::Added(duplicate)));
        assert_eq!(machine.free_slots(), [0, 1, 2, 3, 4, 5, 7, 6]);
    }

    #[test]
    fn load_failure_removes_reference_calls_missing_and_replays_one_fault() {
        let mut machine = GlobalLocationActivationMachine::<TestError>::new();
        let mut runtime = Runtime::default();
        let id = machine.enqueue_filename_load("missing.rx2").unwrap();
        machine.update(&mut runtime).unwrap();
        runtime.loads.remove(&id).unwrap().fail().unwrap();

        let first = machine.update(&mut runtime).unwrap_err();
        let second = machine.update(&mut runtime).unwrap_err();

        assert!(Arc::ptr_eq(&first, &second));
        assert!(matches!(
            first.as_ref(),
            GlobalLocationActivationError::MissingAsset { asset_id } if *asset_id == id
        ));
        assert_eq!(runtime.events.last(), Some(&Event::Missing(id)));
        assert_eq!(machine.reference_count(id), None);
        assert_eq!(machine.active_count(), 1);
    }

    #[test]
    fn hashes_before_slash_conversion_and_rejects_native_buffer_hazards() {
        let mut machine = GlobalLocationActivationMachine::<TestError>::new();
        let original = "data/content/global_locators/one.rx2";
        let id = machine.enqueue_filename_load(original).unwrap();
        assert_eq!(id, rw_hash64_string(original));
        let mut runtime = Runtime::default();
        machine.update(&mut runtime).unwrap();
        assert_eq!(
            machine.active_snapshot()[0].path,
            original.replace('/', "\\")
        );

        assert_eq!(
            machine.enqueue_filename_load("bad\0path"),
            Err(GlobalLocationEnqueueError::InteriorNul)
        );
        assert_eq!(
            machine.enqueue_filename_load(&"x".repeat(256)),
            Err(GlobalLocationEnqueueError::PathTooLong { byte_len: 256 })
        );
    }

    #[test]
    fn completion_is_invalidated_when_the_activation_owner_is_dropped() {
        let mut machine = GlobalLocationActivationMachine::<TestError>::new();
        let mut runtime = Runtime::default();
        let id = machine.enqueue_filename_load("one.rx2").unwrap();
        machine.update(&mut runtime).unwrap();
        let completion = runtime.loads.remove(&id).unwrap();

        drop(machine);

        assert!(matches!(
            completion.complete(),
            Err(ActivationCompletionError::Retired { asset_id, .. }) if asset_id == id
        ));
    }
}
