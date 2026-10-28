//! Parent-collection request ordering and registration reference bookkeeping.
//!
//! This module reconstructs the parent-specific portion of
//! `cAssetStreamManager`. Physical resident ownership remains in
//! [`CollectionResidentStreamer`](crate::collection_resident_streamer::CollectionResidentStreamer),
//! while registration delivery remains an explicit effect.

use crate::collection_resident_streamer::{
    CollectionId, ResidentCallbackContext, ResidentCallbackContextError,
    ResidentCallbackObservation, ResidentCollectionCallback, ResidentCollectionObservation,
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    fmt,
    rc::{Rc, Weak},
};

/// Exact runtime value of `assman::kInvalidCollectionID`.
///
/// The guest initializes it at `0x82f760c0` by hashing `InvalidStringHash`.
/// Parent traversal at `0x82d57fdc` and `0x82d580ac` compares the complete
/// 64-bit value. Zero is therefore an ordinary collection ID.
pub const INVALID_PARENT_COLLECTION_ID: CollectionId = CollectionId(0xe171_e1ab_fedb_69ed);

/// Inline node count of native `mParentCollectionMap`.
pub const PARENT_RELATION_INLINE_CAPACITY: usize = 2048;
/// Bucket count of native `mParentCollectionMap`.
pub const PARENT_RELATION_BUCKET_COUNT: usize = 1024;
/// The native parent-relation table enables allocator overflow.
pub const PARENT_RELATION_OVERFLOW_ENABLED: bool = true;

/// Inline node count of native `mParentCollectionCountMap`.
pub const PARENT_USE_INLINE_CAPACITY: usize = 16;
/// Bucket count of native `mParentCollectionCountMap`.
pub const PARENT_USE_BUCKET_COUNT: usize = 32;
/// The native parent-use table enables allocator overflow.
pub const PARENT_USE_OVERFLOW_ENABLED: bool = true;

/// Exact non-overflow capacity of native `mUnloadParentCompleteVector`.
pub const PARENT_UNLOAD_COMPLETION_CAPACITY: usize = 32;

/// The callback attached to one planned resident request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlannedParentCallbackKind {
    /// Preserve the callback supplied for the originally requested collection.
    Requested,
    /// Use `cLoadParentCollectionRequestCallback` for an ancestor load.
    LoadParent,
    /// Use `cUnloadParentCollectionRequestCallback` for an ancestor unload.
    UnloadParent,
}

/// Kind of resident request emitted by parent traversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlannedParentRequestKind {
    Load,
    Unload,
}

/// One source-ordered resident request produced by parent traversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlannedParentRequest {
    pub id: CollectionId,
    pub kind: PlannedParentRequestKind,
    pub callback: PlannedParentCallbackKind,
}

/// Safe-host errors for malformed parent relationship input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParentTopologyError {
    Cycle {
        requested: CollectionId,
        repeated: CollectionId,
    },
}

impl fmt::Display for ParentTopologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cycle {
                requested,
                repeated,
            } => write!(
                formatter,
                "parent chain for {:#018x} repeats {:#018x}",
                requested.0, repeated.0
            ),
        }
    }
}

impl std::error::Error for ParentTopologyError {}

/// Parent registration or deregistration effect being attempted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionParentEffectStage {
    SimulationRegister,
    PresentationRegister,
    SimulationDeferUpdate,
    SimulationDeregister,
    PresentationDeregister,
}

/// Result-bearing boundary for effects owned outside parent bookkeeping.
pub trait CollectionParentEffects {
    type Error;

    /// Sends the parent Simulation registration effect. Native callers use
    /// userdata zero and non-immediate registration at `0x82d5ca78`.
    ///
    /// # Errors
    ///
    /// Reports an implementation-defined delivery failure.
    fn simulation_register(&mut self, id: CollectionId) -> Result<(), Self::Error>;

    /// Sends the parent Presentation registration effect. Native callers use
    /// an invalid proxy, userdata zero, and non-immediate registration at
    /// `0x82d5caa4`.
    ///
    /// # Errors
    ///
    /// Reports an implementation-defined delivery failure.
    fn presentation_register(&mut self, id: CollectionId) -> Result<(), Self::Error>;

    /// Runs `UpdateSimCollectionDefer`, which occurs between resident update
    /// and parent-unload FIFO processing at `0x82d61364`.
    ///
    /// # Errors
    ///
    /// Reports an implementation-defined deferred-registration failure.
    fn simulation_defer_update(&mut self) -> Result<(), Self::Error>;

    /// Sends the parent Simulation deregistration effect. A successful effect
    /// must arrange one later external resident `Down` completion.
    ///
    /// # Errors
    ///
    /// Reports an implementation-defined delivery failure.
    fn simulation_deregister(&mut self, id: CollectionId) -> Result<(), Self::Error>;

    /// Sends the parent Presentation deregistration effect. A successful
    /// effect must arrange a distinct later external resident `Down`
    /// completion.
    ///
    /// # Errors
    ///
    /// Reports an implementation-defined delivery failure.
    fn presentation_deregister(&mut self, id: CollectionId) -> Result<(), Self::Error>;
}

/// Typed parent-manager callback or state failure.
#[derive(Debug)]
pub enum CollectionParentManagerError<E> {
    ManagerDropped,
    ReentrantStateAccess,
    MissingParentUse {
        id: CollectionId,
    },
    UnloadCompletionCapacityExceeded {
        id: CollectionId,
    },
    ResidentContext(ResidentCallbackContextError),
    Effect {
        stage: CollectionParentEffectStage,
        source: E,
    },
}

impl<E: fmt::Display> fmt::Display for CollectionParentManagerError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManagerDropped => formatter.write_str("parent collection manager was dropped"),
            Self::ReentrantStateAccess => {
                formatter.write_str("parent collection state is already mutably borrowed")
            }
            Self::MissingParentUse { id } => write!(
                formatter,
                "parent collection {:#018x} has no use-count entry",
                id.0
            ),
            Self::UnloadCompletionCapacityExceeded { id } => write!(
                formatter,
                "parent unload completion FIFO is full for {:#018x}",
                id.0
            ),
            Self::ResidentContext(source) => source.fmt(formatter),
            Self::Effect { stage, source } => write!(formatter, "{stage:?} failed: {source}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for CollectionParentManagerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ResidentContext(source) => Some(source),
            Self::ManagerDropped
            | Self::ReentrantStateAccess
            | Self::MissingParentUse { .. }
            | Self::UnloadCompletionCapacityExceeded { .. }
            | Self::Effect { .. } => None,
        }
    }
}

/// Error ordering for one outer manager update.
#[derive(Debug)]
pub enum CollectionParentUpdateError<R, E> {
    Resident(R),
    Parent(CollectionParentManagerError<E>),
}

impl<R: fmt::Display, E: fmt::Display> fmt::Display for CollectionParentUpdateError<R, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resident(source) => write!(formatter, "resident update failed: {source}"),
            Self::Parent(source) => source.fmt(formatter),
        }
    }
}

impl<R, E> std::error::Error for CollectionParentUpdateError<R, E>
where
    R: fmt::Debug + fmt::Display,
    E: fmt::Debug + fmt::Display,
{
}

/// Observable result from one outer manager update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionParentUpdate<T> {
    pub resident: T,
    pub processed_parent_unload: Option<CollectionId>,
}

struct ParentRuntime<Fx> {
    parent_uses: Vec<(CollectionId, u32)>,
    unload_completions: VecDeque<CollectionId>,
    effects: Fx,
}

/// Non-owning equivalent of native `cLoadParentCollectionRequestCallback`.
pub struct LoadParentCollectionCallback<Fx> {
    runtime: Weak<RefCell<ParentRuntime<Fx>>>,
}

/// Non-owning equivalent of native `cUnloadParentCollectionRequestCallback`.
pub struct UnloadParentCollectionCallback<Fx> {
    runtime: Weak<RefCell<ParentRuntime<Fx>>>,
}

/// Parent callback variants suitable for wrapping in a concrete streamer's
/// application callback enum.
pub enum ParentCollectionCallback<Fx> {
    Load(LoadParentCollectionCallback<Fx>),
    Unload(UnloadParentCollectionCallback<Fx>),
}

impl<Fx> ParentCollectionCallback<Fx>
where
    Fx: CollectionParentEffects,
{
    /// Dispatches a parent callback through an outer callback enum's context.
    ///
    /// This narrow adapter is required because the resident context owns a
    /// queue of the application's complete callback type, not a queue of this
    /// parent-only variant. Parent load completion can only increment the
    /// active resident; parent unload completion does not touch the context.
    /// No queue or manager state is exposed.
    ///
    /// # Errors
    ///
    /// Maps the first parent callback failure into the outer streamer's error
    /// domain. Mutations completed before that failure are not rolled back.
    pub fn dispatch_mapped<C, E, M>(
        &mut self,
        id: CollectionId,
        context: &mut ResidentCallbackContext<'_, C>,
        map_error: M,
    ) -> Result<(), E>
    where
        M: FnOnce(CollectionParentManagerError<Fx::Error>) -> E,
    {
        let result = match self {
            Self::Load(callback) => {
                callback.complete_with(id, || context.up_active_collection_loaded_count())
            }
            Self::Unload(callback) => callback.record_completion(id),
        };
        result.map_err(map_error)
    }
}

impl<Fx> LoadParentCollectionCallback<Fx>
where
    Fx: CollectionParentEffects,
{
    fn complete_with<U>(
        &mut self,
        id: CollectionId,
        mut up_active: U,
    ) -> Result<(), CollectionParentManagerError<Fx::Error>>
    where
        U: FnMut() -> Result<ResidentCollectionObservation, ResidentCallbackContextError>,
    {
        let runtime = self
            .runtime
            .upgrade()
            .ok_or(CollectionParentManagerError::ManagerDropped)?;

        let first_use = {
            let mut state = runtime
                .try_borrow_mut()
                .map_err(|_| CollectionParentManagerError::ReentrantStateAccess)?;
            if let Some((_, count)) = state
                .parent_uses
                .iter_mut()
                .find(|(existing, _)| *existing == id)
            {
                *count = count.wrapping_add(1);
                false
            } else {
                state.parent_uses.push((id, 1));
                true
            }
        };

        if !first_use {
            return Ok(());
        }

        up_active().map_err(CollectionParentManagerError::ResidentContext)?;
        {
            let mut state = runtime
                .try_borrow_mut()
                .map_err(|_| CollectionParentManagerError::ReentrantStateAccess)?;
            state.effects.simulation_register(id).map_err(|source| {
                CollectionParentManagerError::Effect {
                    stage: CollectionParentEffectStage::SimulationRegister,
                    source,
                }
            })?;
        }

        up_active().map_err(CollectionParentManagerError::ResidentContext)?;
        let mut state = runtime
            .try_borrow_mut()
            .map_err(|_| CollectionParentManagerError::ReentrantStateAccess)?;
        state.effects.presentation_register(id).map_err(|source| {
            CollectionParentManagerError::Effect {
                stage: CollectionParentEffectStage::PresentationRegister,
                source,
            }
        })
    }
}

impl<Fx> UnloadParentCollectionCallback<Fx> {
    fn record_completion<E>(
        &mut self,
        id: CollectionId,
    ) -> Result<(), CollectionParentManagerError<E>> {
        let runtime = self
            .runtime
            .upgrade()
            .ok_or(CollectionParentManagerError::ManagerDropped)?;
        let mut state = runtime
            .try_borrow_mut()
            .map_err(|_| CollectionParentManagerError::ReentrantStateAccess)?;
        if state.unload_completions.len() >= PARENT_UNLOAD_COMPLETION_CAPACITY {
            return Err(CollectionParentManagerError::UnloadCompletionCapacityExceeded { id });
        }
        state.unload_completions.push_back(id);
        Ok(())
    }
}

impl<Fx> ResidentCollectionCallback<CollectionParentManagerError<Fx::Error>>
    for ParentCollectionCallback<Fx>
where
    Fx: CollectionParentEffects,
{
    fn on_complete(
        &mut self,
        id: CollectionId,
        _observation: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, Self>,
    ) -> Result<(), CollectionParentManagerError<Fx::Error>> {
        self.dispatch_mapped(id, context, std::convert::identity)
    }
}

/// Parent topology and shared callback/effect state.
///
/// The PDB types are
/// `fixed_hash_map<cAssetID,cAssetID,2048,1024,true>` and
/// `fixed_hash_map<cAssetID,uint32_t,16,32,true>`. Because overflow is enabled,
/// the executable does not prove a total cardinality cap. Rust vectors model
/// full-width lookup and overflow growth without claiming native buckets or
/// allocation-failure behavior.
pub struct CollectionParentManager<Fx> {
    parent_relations: Vec<(CollectionId, CollectionId)>,
    runtime: Rc<RefCell<ParentRuntime<Fx>>>,
}

impl<Fx> CollectionParentManager<Fx>
where
    Fx: CollectionParentEffects,
{
    #[must_use]
    pub fn new(effects: Fx) -> Self {
        Self {
            parent_relations: Vec::new(),
            runtime: Rc::new(RefCell::new(ParentRuntime {
                parent_uses: Vec::new(),
                unload_completions: VecDeque::with_capacity(PARENT_UNLOAD_COMPLETION_CAPACITY),
                effects,
            })),
        }
    }

    /// Adds one full-width child-to-parent relationship.
    ///
    /// A duplicate child overwrites its previous parent, matching the native
    /// fixed-map assignment in `AddParent`.
    pub fn add_parent(&mut self, child: CollectionId, parent: CollectionId) {
        if let Some((_, existing_parent)) = self
            .parent_relations
            .iter_mut()
            .find(|(existing, _)| *existing == child)
        {
            *existing_parent = parent;
            return;
        }
        self.parent_relations.push((child, parent));
    }

    /// Builds the exact ancestor-first load order from `0x82d57f58`.
    ///
    /// # Errors
    ///
    /// Returns a safe-host error for a cyclic parent graph. Native malformed
    /// cycle behavior remains unknown.
    pub fn plan_load(
        &self,
        requested: CollectionId,
    ) -> Result<Vec<PlannedParentRequest>, ParentTopologyError> {
        let mut chain = self.parent_chain(requested)?;
        chain.reverse();
        let requested_index = chain.len() - 1;
        Ok(chain
            .into_iter()
            .enumerate()
            .map(|(index, id)| PlannedParentRequest {
                id,
                kind: PlannedParentRequestKind::Load,
                callback: if index == requested_index {
                    PlannedParentCallbackKind::Requested
                } else {
                    PlannedParentCallbackKind::LoadParent
                },
            })
            .collect())
    }

    /// Builds the exact child-first unload order from `0x82d58018`.
    ///
    /// # Errors
    ///
    /// Returns a safe-host error for a cyclic parent graph. Native malformed
    /// cycle behavior remains unknown.
    pub fn plan_unload(
        &self,
        requested: CollectionId,
    ) -> Result<Vec<PlannedParentRequest>, ParentTopologyError> {
        Ok(self
            .parent_chain(requested)?
            .into_iter()
            .enumerate()
            .map(|(index, id)| PlannedParentRequest {
                id,
                kind: PlannedParentRequestKind::Unload,
                callback: if index == 0 {
                    PlannedParentCallbackKind::Requested
                } else {
                    PlannedParentCallbackKind::UnloadParent
                },
            })
            .collect())
    }

    /// Creates the non-owning callback used by ancestor load requests.
    #[must_use]
    pub fn load_parent_callback(&self) -> ParentCollectionCallback<Fx> {
        ParentCollectionCallback::Load(LoadParentCollectionCallback {
            runtime: Rc::downgrade(&self.runtime),
        })
    }

    /// Creates the non-owning callback used by ancestor unload requests.
    #[must_use]
    pub fn unload_parent_callback(&self) -> ParentCollectionCallback<Fx> {
        ParentCollectionCallback::Unload(UnloadParentCollectionCallback {
            runtime: Rc::downgrade(&self.runtime),
        })
    }

    /// Returns the manager use count for one parent.
    #[must_use]
    pub fn parent_use_count(&self, id: CollectionId) -> Option<u32> {
        self.runtime
            .borrow()
            .parent_uses
            .iter()
            .find_map(|(existing, count)| (*existing == id).then_some(*count))
    }

    /// Number of completed ancestor unloads waiting for manager processing.
    #[must_use]
    pub fn pending_parent_unloads(&self) -> usize {
        self.runtime.borrow().unload_completions.len()
    }

    /// Runs resident update, Simulation deferred registration, and at most one
    /// parent-unload completion in the exact order at `0x82d6135c..0x82d613c4`.
    ///
    /// Parent callbacks may append to the shared completion FIFO while
    /// `resident_update` runs. The front ID is removed before its manager use
    /// count is processed, matching the native vector erase.
    ///
    /// # Errors
    ///
    /// Returns the first resident, deferred-registration, parent-state, or
    /// deregistration error. Already-observable mutations are not rolled back.
    pub fn update<T, R, U>(
        &mut self,
        resident_update: U,
    ) -> Result<CollectionParentUpdate<T>, CollectionParentUpdateError<R, Fx::Error>>
    where
        U: FnOnce() -> Result<T, R>,
    {
        let resident = resident_update().map_err(CollectionParentUpdateError::Resident)?;
        {
            let mut state = self.runtime.try_borrow_mut().map_err(|_| {
                CollectionParentUpdateError::Parent(
                    CollectionParentManagerError::ReentrantStateAccess,
                )
            })?;
            state.effects.simulation_defer_update().map_err(|source| {
                CollectionParentUpdateError::Parent(CollectionParentManagerError::Effect {
                    stage: CollectionParentEffectStage::SimulationDeferUpdate,
                    source,
                })
            })?;
        }

        let completed = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| {
                CollectionParentUpdateError::Parent(
                    CollectionParentManagerError::ReentrantStateAccess,
                )
            })?
            .unload_completions
            .pop_front();
        if let Some(id) = completed {
            self.finish_parent_unload(id)
                .map_err(CollectionParentUpdateError::Parent)?;
        }

        Ok(CollectionParentUpdate {
            resident,
            processed_parent_unload: completed,
        })
    }

    fn parent_chain(
        &self,
        requested: CollectionId,
    ) -> Result<Vec<CollectionId>, ParentTopologyError> {
        let mut chain = vec![requested];
        let mut current = requested;
        loop {
            let parent = self
                .parent_relations
                .iter()
                .find_map(|(child, parent)| (*child == current).then_some(*parent))
                .unwrap_or(INVALID_PARENT_COLLECTION_ID);
            if parent == INVALID_PARENT_COLLECTION_ID {
                return Ok(chain);
            }
            if chain.contains(&parent) {
                return Err(ParentTopologyError::Cycle {
                    requested,
                    repeated: parent,
                });
            }
            chain.push(parent);
            current = parent;
        }
    }

    fn finish_parent_unload(
        &mut self,
        id: CollectionId,
    ) -> Result<(), CollectionParentManagerError<Fx::Error>> {
        let reached_zero = {
            let mut state = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| CollectionParentManagerError::ReentrantStateAccess)?;
            let Some(index) = state
                .parent_uses
                .iter()
                .position(|(existing, _)| *existing == id)
            else {
                return Err(CollectionParentManagerError::MissingParentUse { id });
            };
            let count = state.parent_uses[index].1.wrapping_sub(1);
            state.parent_uses[index].1 = count;
            if count == 0 {
                state.parent_uses.remove(index);
                true
            } else {
                false
            }
        };

        if !reached_zero {
            return Ok(());
        }

        {
            let mut state = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| CollectionParentManagerError::ReentrantStateAccess)?;
            state.effects.simulation_deregister(id).map_err(|source| {
                CollectionParentManagerError::Effect {
                    stage: CollectionParentEffectStage::SimulationDeregister,
                    source,
                }
            })?;
        }
        let mut state = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| CollectionParentManagerError::ReentrantStateAccess)?;
        state.effects.presentation_deregister(id).map_err(|source| {
            CollectionParentManagerError::Effect {
                stage: CollectionParentEffectStage::PresentationDeregister,
                source,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection_resident_streamer::{
        CollectionResidentStreamer, ResidentAssetGarbageCollector, ResidentCollectionLoadDriver,
        ResidentCollectionLoadFactory, ResidentCollectionMetadata, ResidentCollectionRegistry,
        ResidentCollectionStorage, ResidentDecompressionKick,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        ResidentUpdate,
        Up(u32),
        SimulationRegister(CollectionId),
        PresentationRegister(CollectionId),
        SimulationDefer,
        SimulationDeregister(CollectionId),
        PresentationDeregister(CollectionId),
        ResidentDown(CollectionId),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestError(CollectionParentEffectStage);

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{:?}", self.0)
        }
    }

    struct TestEffects {
        events: Rc<RefCell<Vec<Event>>>,
        fail: Option<CollectionParentEffectStage>,
    }

    #[derive(Debug)]
    enum OuterError {
        Parent(CollectionParentManagerError<TestError>),
    }

    impl fmt::Display for OuterError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Parent(source) => source.fmt(formatter),
            }
        }
    }

    struct OuterCollection;
    struct OuterWriter;
    struct OuterStorage;

    impl ResidentCollectionStorage<OuterError> for OuterStorage {
        type Collection = OuterCollection;
        type Writer = OuterWriter;

        fn create_empty(&mut self, _reserve: i32) -> Result<Self::Collection, OuterError> {
            Ok(OuterCollection)
        }

        fn writer(&mut self, _collection: &Self::Collection) -> Result<Self::Writer, OuterError> {
            Ok(OuterWriter)
        }

        fn asset_ids_in_order(
            &mut self,
            _collection: &Self::Collection,
        ) -> Result<Vec<CollectionId>, OuterError> {
            Ok(Vec::new())
        }

        fn destroy(&mut self, _collection: Self::Collection) {}
    }

    struct OuterMetadata;

    impl ResidentCollectionMetadata<OuterError> for OuterMetadata {
        type Descriptor = ();

        fn find_collection(
            &mut self,
            _id: CollectionId,
        ) -> Result<Option<Self::Descriptor>, OuterError> {
            Ok(Some(()))
        }
    }

    struct OuterDriver {
        complete: bool,
    }

    impl ResidentCollectionLoadDriver<OuterError> for OuterDriver {
        fn update(&mut self) -> Result<(), OuterError> {
            self.complete = true;
            Ok(())
        }

        fn is_complete(&self) -> bool {
            self.complete
        }

        fn highest_stored_block(&self) -> u32 {
            0
        }

        fn loaded_size(&self) -> u32 {
            0
        }
    }

    struct OuterFactory;

    impl ResidentCollectionLoadFactory<OuterWriter, (), OuterError> for OuterFactory {
        type Driver = OuterDriver;

        fn begin(
            &mut self,
            _id: CollectionId,
            _asset_count: i32,
            _descriptor: (),
            _writer: OuterWriter,
        ) -> Result<Self::Driver, OuterError> {
            Ok(OuterDriver { complete: false })
        }
    }

    struct OuterRegistry;

    impl ResidentCollectionRegistry<OuterCollection, OuterError> for OuterRegistry {
        type Wrapper = ();
        type Token = ();

        fn create_wrapper(
            &mut self,
            _id: CollectionId,
            _collection: &OuterCollection,
        ) -> Result<Self::Wrapper, OuterError> {
            Ok(())
        }

        fn stable_token(_wrapper: &Self::Wrapper) -> Self::Token {}

        fn publish(&mut self, _id: CollectionId, _token: &Self::Token) -> Result<(), OuterError> {
            Ok(())
        }

        fn remove_exact(
            &mut self,
            _id: CollectionId,
            _token: &Self::Token,
        ) -> Result<(), OuterError> {
            Ok(())
        }
    }

    struct OuterGarbage;

    impl ResidentAssetGarbageCollector<OuterError> for OuterGarbage {
        fn garbage_collect(&mut self, _id: CollectionId) -> Result<(), OuterError> {
            Ok(())
        }
    }

    struct OuterKick;

    impl ResidentDecompressionKick<OuterError> for OuterKick {
        fn start_decompress(&mut self) -> Result<(), OuterError> {
            Ok(())
        }
    }

    enum OuterCallback {
        Parent(ParentCollectionCallback<TestEffects>),
    }

    impl ResidentCollectionCallback<OuterError> for OuterCallback {
        fn on_complete(
            &mut self,
            id: CollectionId,
            _observation: ResidentCallbackObservation,
            context: &mut ResidentCallbackContext<'_, Self>,
        ) -> Result<(), OuterError> {
            match self {
                Self::Parent(callback) => callback.dispatch_mapped(id, context, OuterError::Parent),
            }
        }
    }

    impl TestEffects {
        fn effect(
            &mut self,
            stage: CollectionParentEffectStage,
            event: Event,
        ) -> Result<(), TestError> {
            self.events.borrow_mut().push(event);
            if self.fail == Some(stage) {
                return Err(TestError(stage));
            }
            Ok(())
        }
    }

    impl CollectionParentEffects for TestEffects {
        type Error = TestError;

        fn simulation_register(&mut self, id: CollectionId) -> Result<(), Self::Error> {
            self.effect(
                CollectionParentEffectStage::SimulationRegister,
                Event::SimulationRegister(id),
            )
        }

        fn presentation_register(&mut self, id: CollectionId) -> Result<(), Self::Error> {
            self.effect(
                CollectionParentEffectStage::PresentationRegister,
                Event::PresentationRegister(id),
            )
        }

        fn simulation_defer_update(&mut self) -> Result<(), Self::Error> {
            self.effect(
                CollectionParentEffectStage::SimulationDeferUpdate,
                Event::SimulationDefer,
            )
        }

        fn simulation_deregister(&mut self, id: CollectionId) -> Result<(), Self::Error> {
            self.effect(
                CollectionParentEffectStage::SimulationDeregister,
                Event::SimulationDeregister(id),
            )
        }

        fn presentation_deregister(&mut self, id: CollectionId) -> Result<(), Self::Error> {
            self.effect(
                CollectionParentEffectStage::PresentationDeregister,
                Event::PresentationDeregister(id),
            )
        }
    }

    fn manager(
        fail: Option<CollectionParentEffectStage>,
    ) -> (
        CollectionParentManager<TestEffects>,
        Rc<RefCell<Vec<Event>>>,
    ) {
        let events = Rc::new(RefCell::new(Vec::new()));
        (
            CollectionParentManager::new(TestEffects {
                events: Rc::clone(&events),
                fail,
            }),
            events,
        )
    }

    fn complete_load(
        manager: &CollectionParentManager<TestEffects>,
        id: CollectionId,
        resident_count: &mut u32,
        events: &Rc<RefCell<Vec<Event>>>,
    ) -> Result<(), CollectionParentManagerError<TestError>> {
        let ParentCollectionCallback::Load(mut callback) = manager.load_parent_callback() else {
            unreachable!();
        };
        callback.complete_with(id, || {
            *resident_count = resident_count.wrapping_add(1);
            events.borrow_mut().push(Event::Up(*resident_count));
            Ok(ResidentCollectionObservation {
                ref_count: *resident_count,
                wrapper_is_published: true,
            })
        })
    }

    fn complete_unload(
        manager: &CollectionParentManager<TestEffects>,
        id: CollectionId,
    ) -> Result<(), CollectionParentManagerError<TestError>> {
        let ParentCollectionCallback::Unload(mut callback) = manager.unload_parent_callback()
        else {
            unreachable!();
        };
        callback.record_completion(id)
    }

    #[test]
    fn exact_sentinel_terminates_but_zero_is_an_ordinary_parent() {
        let (mut manager, _) = manager(None);
        let child = CollectionId(0x1111_2222_3333_4444);
        manager.add_parent(child, CollectionId(0));
        manager.add_parent(CollectionId(0), INVALID_PARENT_COLLECTION_ID);

        assert_eq!(
            manager.plan_load(child).unwrap(),
            [
                PlannedParentRequest {
                    id: CollectionId(0),
                    kind: PlannedParentRequestKind::Load,
                    callback: PlannedParentCallbackKind::LoadParent,
                },
                PlannedParentRequest {
                    id: child,
                    kind: PlannedParentRequestKind::Load,
                    callback: PlannedParentCallbackKind::Requested,
                },
            ]
        );
        assert_eq!(
            manager.plan_unload(child).unwrap(),
            [
                PlannedParentRequest {
                    id: child,
                    kind: PlannedParentRequestKind::Unload,
                    callback: PlannedParentCallbackKind::Requested,
                },
                PlannedParentRequest {
                    id: CollectionId(0),
                    kind: PlannedParentRequestKind::Unload,
                    callback: PlannedParentCallbackKind::UnloadParent,
                },
            ]
        );
        assert_eq!(
            manager.plan_load(INVALID_PARENT_COLLECTION_ID).unwrap(),
            [PlannedParentRequest {
                id: INVALID_PARENT_COLLECTION_ID,
                kind: PlannedParentRequestKind::Load,
                callback: PlannedParentCallbackKind::Requested,
            }]
        );
    }

    #[test]
    fn loads_are_ancestor_first_and_unloads_are_child_first() {
        let (mut manager, _) = manager(None);
        let child = CollectionId(10);
        let parent = CollectionId(20);
        let grandparent = CollectionId(30);
        manager.add_parent(child, parent);
        manager.add_parent(parent, grandparent);

        let loads = manager.plan_load(child).unwrap();
        assert_eq!(
            loads.iter().map(|request| request.id).collect::<Vec<_>>(),
            [grandparent, parent, child]
        );
        assert_eq!(
            loads
                .iter()
                .map(|request| request.callback)
                .collect::<Vec<_>>(),
            [
                PlannedParentCallbackKind::LoadParent,
                PlannedParentCallbackKind::LoadParent,
                PlannedParentCallbackKind::Requested,
            ]
        );

        let unloads = manager.plan_unload(child).unwrap();
        assert_eq!(
            unloads.iter().map(|request| request.id).collect::<Vec<_>>(),
            [child, parent, grandparent]
        );
        assert_eq!(
            unloads
                .iter()
                .map(|request| request.callback)
                .collect::<Vec<_>>(),
            [
                PlannedParentCallbackKind::Requested,
                PlannedParentCallbackKind::UnloadParent,
                PlannedParentCallbackKind::UnloadParent,
            ]
        );
    }

    #[test]
    fn parent_lookup_preserves_full_width_ids_with_equal_native_hashes() {
        let (mut manager, _) = manager(None);
        let first = CollectionId(0x0000_0001_0000_0002);
        let second = CollectionId(0x0000_0002_0000_0001);
        let first_parent = CollectionId(0xaaaa_bbbb_cccc_dddd);
        let second_parent = CollectionId(0x1111_2222_3333_4444);
        assert_eq!(first.native_hash(), second.native_hash());
        manager.add_parent(first, first_parent);
        manager.add_parent(second, second_parent);

        assert_eq!(manager.plan_load(first).unwrap()[0].id, first_parent);
        assert_eq!(manager.plan_load(second).unwrap()[0].id, second_parent);
    }

    #[test]
    fn cyclic_topology_is_rejected_without_partial_request_output() {
        let (mut manager, _) = manager(None);
        manager.add_parent(CollectionId(1), CollectionId(2));
        manager.add_parent(CollectionId(2), CollectionId(1));
        assert_eq!(
            manager.plan_load(CollectionId(1)).unwrap_err(),
            ParentTopologyError::Cycle {
                requested: CollectionId(1),
                repeated: CollectionId(1),
            }
        );
        assert!(manager.plan_unload(CollectionId(1)).is_err());
    }

    #[test]
    fn duplicate_child_overwrites_the_previous_parent() {
        let (mut manager, _) = manager(None);
        let child = CollectionId(3);
        let first = CollectionId(4);
        let replacement = CollectionId(5);
        manager.add_parent(child, first);
        manager.add_parent(child, replacement);

        assert_eq!(manager.plan_load(child).unwrap()[0].id, replacement);
        assert_eq!(manager.parent_relations.len(), 1);
    }

    #[test]
    fn first_parent_load_holds_twice_around_ordered_registration() {
        let (manager, events) = manager(None);
        let id = CollectionId(40);
        let mut resident_count = 1;

        complete_load(&manager, id, &mut resident_count, &events).unwrap();

        assert_eq!(manager.parent_use_count(id), Some(1));
        assert_eq!(resident_count, 3);
        assert_eq!(
            events.borrow().as_slice(),
            [
                Event::Up(2),
                Event::SimulationRegister(id),
                Event::Up(3),
                Event::PresentationRegister(id),
            ]
        );
    }

    #[test]
    fn existing_parent_use_wraps_without_more_holds_or_registration() {
        let (manager, events) = manager(None);
        let id = CollectionId(41);
        let mut resident_count = 1;
        complete_load(&manager, id, &mut resident_count, &events).unwrap();
        events.borrow_mut().clear();

        // The resident streamer performs its cache-hit increment before this
        // callback. The callback then changes only the manager use count.
        resident_count = resident_count.wrapping_add(1);
        complete_load(&manager, id, &mut resident_count, &events).unwrap();

        assert_eq!(manager.parent_use_count(id), Some(2));
        assert_eq!(resident_count, 4);
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn existing_parent_use_increment_wraps_at_u32_max() {
        let (manager, events) = manager(None);
        let id = CollectionId(42);
        manager
            .runtime
            .borrow_mut()
            .parent_uses
            .push((id, u32::MAX));
        let mut resident_count = 9;

        complete_load(&manager, id, &mut resident_count, &events).unwrap();

        assert_eq!(manager.parent_use_count(id), Some(0));
        assert_eq!(resident_count, 9);
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn unload_callback_appends_before_the_resident_decrement() {
        let (manager, events) = manager(None);
        let id = CollectionId(50);
        let mut resident_count = 1;
        complete_load(&manager, id, &mut resident_count, &events).unwrap();
        events.borrow_mut().clear();

        complete_unload(&manager, id).unwrap();
        assert_eq!(manager.pending_parent_unloads(), 1);
        resident_count = resident_count.wrapping_sub(1);
        events.borrow_mut().push(Event::ResidentDown(id));

        assert_eq!(resident_count, 2);
        assert_eq!(events.borrow().as_slice(), [Event::ResidentDown(id)]);
    }

    #[test]
    fn outer_update_runs_resident_then_sim_defer_and_only_one_fifo_front() {
        let (mut manager, events) = manager(None);
        let first = CollectionId(60);
        let second = CollectionId(61);
        let mut first_count = 1;
        let mut second_count = 1;
        complete_load(&manager, first, &mut first_count, &events).unwrap();
        complete_load(&manager, second, &mut second_count, &events).unwrap();
        complete_unload(&manager, first).unwrap();
        complete_unload(&manager, second).unwrap();
        events.borrow_mut().clear();

        let update = manager
            .update(|| {
                events.borrow_mut().push(Event::ResidentUpdate);
                Ok::<_, TestError>(77)
            })
            .unwrap();

        assert_eq!(update.resident, 77);
        assert_eq!(update.processed_parent_unload, Some(first));
        assert_eq!(manager.pending_parent_unloads(), 1);
        assert_eq!(manager.parent_use_count(first), None);
        assert_eq!(manager.parent_use_count(second), Some(1));
        assert_eq!(
            events.borrow().as_slice(),
            [
                Event::ResidentUpdate,
                Event::SimulationDefer,
                Event::SimulationDeregister(first),
                Event::PresentationDeregister(first),
            ]
        );
    }

    #[test]
    fn resident_callback_can_append_during_outer_resident_update() {
        let (mut manager, events) = manager(None);
        let id = CollectionId(62);
        let mut resident_count = 1;
        complete_load(&manager, id, &mut resident_count, &events).unwrap();
        let ParentCollectionCallback::Unload(mut callback) = manager.unload_parent_callback()
        else {
            unreachable!();
        };
        events.borrow_mut().clear();

        let update = manager
            .update(|| {
                events.borrow_mut().push(Event::ResidentUpdate);
                callback.record_completion::<TestError>(id).unwrap();
                Ok::<_, TestError>(())
            })
            .unwrap();

        assert_eq!(update.processed_parent_unload, Some(id));
        assert_eq!(manager.pending_parent_unloads(), 0);
        assert_eq!(
            events.borrow().as_slice(),
            [
                Event::ResidentUpdate,
                Event::SimulationDefer,
                Event::SimulationDeregister(id),
                Event::PresentationDeregister(id),
            ]
        );
    }

    #[test]
    fn final_use_erases_before_ordered_deregistration_and_down_stays_external() {
        let (mut manager, events) = manager(None);
        let id = CollectionId(70);
        let mut resident_count = 1;
        complete_load(&manager, id, &mut resident_count, &events).unwrap();
        complete_unload(&manager, id).unwrap();
        resident_count = resident_count.wrapping_sub(1);
        events.borrow_mut().clear();

        manager.update(|| Ok::<_, TestError>(())).unwrap();

        assert_eq!(manager.parent_use_count(id), None);
        assert_eq!(resident_count, 2);
        assert_eq!(
            events.borrow().as_slice(),
            [
                Event::SimulationDefer,
                Event::SimulationDeregister(id),
                Event::PresentationDeregister(id),
            ]
        );

        resident_count = resident_count.wrapping_sub(1);
        resident_count = resident_count.wrapping_sub(1);
        assert_eq!(resident_count, 0);
    }

    #[test]
    fn registration_failure_retains_every_prior_mutation_without_rollback() {
        let (manager, events) = manager(Some(CollectionParentEffectStage::SimulationRegister));
        let id = CollectionId(80);
        let mut resident_count = 1;

        let error = complete_load(&manager, id, &mut resident_count, &events).unwrap_err();

        assert!(matches!(
            error,
            CollectionParentManagerError::Effect {
                stage: CollectionParentEffectStage::SimulationRegister,
                ..
            }
        ));
        assert_eq!(manager.parent_use_count(id), Some(1));
        assert_eq!(resident_count, 2);
        assert_eq!(
            events.borrow().as_slice(),
            [Event::Up(2), Event::SimulationRegister(id)]
        );
    }

    #[test]
    fn deregistration_failure_keeps_fifo_pop_and_erased_use() {
        let (mut manager, events) =
            manager(Some(CollectionParentEffectStage::SimulationDeregister));
        let id = CollectionId(81);
        let mut resident_count = 1;
        complete_load(&manager, id, &mut resident_count, &events).unwrap();
        complete_unload(&manager, id).unwrap();
        events.borrow_mut().clear();

        let error = manager.update(|| Ok::<_, TestError>(())).unwrap_err();

        assert!(matches!(
            error,
            CollectionParentUpdateError::Parent(CollectionParentManagerError::Effect {
                stage: CollectionParentEffectStage::SimulationDeregister,
                ..
            })
        ));
        assert_eq!(manager.parent_use_count(id), None);
        assert_eq!(manager.pending_parent_unloads(), 0);
        assert_eq!(
            events.borrow().as_slice(),
            [Event::SimulationDefer, Event::SimulationDeregister(id),]
        );
    }

    #[test]
    fn fixed_vector_capacity_is_typed_but_hash_overflow_is_enabled() {
        let (mut manager, events) = manager(None);
        for raw in 0..PARENT_UNLOAD_COMPLETION_CAPACITY {
            complete_unload(&manager, CollectionId(raw as u64)).unwrap();
        }
        let error = complete_unload(&manager, CollectionId(99)).unwrap_err();
        assert!(matches!(
            error,
            CollectionParentManagerError::UnloadCompletionCapacityExceeded {
                id: CollectionId(99)
            }
        ));

        for raw in 0..=PARENT_RELATION_INLINE_CAPACITY {
            manager.add_parent(
                CollectionId(0x1000_0000 + raw as u64),
                INVALID_PARENT_COLLECTION_ID,
            );
        }
        let mut resident_count = 1;
        events.borrow_mut().clear();
        for raw in 0..=PARENT_USE_INLINE_CAPACITY {
            complete_load(
                &manager,
                CollectionId(0x2000_0000 + raw as u64),
                &mut resident_count,
                &events,
            )
            .unwrap();
        }
        assert_eq!(
            manager.runtime.borrow().parent_uses.len(),
            PARENT_USE_INLINE_CAPACITY + 1
        );

        const {
            assert!(PARENT_RELATION_OVERFLOW_ENABLED);
            assert!(PARENT_USE_OVERFLOW_ENABLED);
        }
        assert_eq!(PARENT_RELATION_INLINE_CAPACITY, 2048);
        assert_eq!(PARENT_RELATION_BUCKET_COUNT, 1024);
        assert_eq!(PARENT_USE_INLINE_CAPACITY, 16);
        assert_eq!(PARENT_USE_BUCKET_COUNT, 32);
    }

    #[test]
    fn mapped_dispatch_composes_through_an_outer_callback_context() {
        let (mut manager, events) = manager(None);
        let id = CollectionId(0x90);
        let mut streamer = CollectionResidentStreamer::new(
            OuterStorage,
            OuterMetadata,
            OuterFactory,
            OuterRegistry,
            OuterGarbage,
            OuterKick,
        );
        streamer.enqueue_load(
            id,
            0,
            Some(OuterCallback::Parent(manager.load_parent_callback())),
        );

        assert_eq!(streamer.update().unwrap().physical_loads_started, 1);
        assert_eq!(streamer.update().unwrap().physical_loads_completed, 1);
        assert_eq!(manager.parent_use_count(id), Some(1));
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 3);
        assert_eq!(
            events.borrow().as_slice(),
            [
                Event::SimulationRegister(id),
                Event::PresentationRegister(id),
            ]
        );

        streamer.enqueue_unload(
            id,
            Some(OuterCallback::Parent(manager.unload_parent_callback())),
        );
        streamer.update().unwrap();
        assert_eq!(manager.pending_parent_unloads(), 1);
        assert_eq!(streamer.observe_resident(id).unwrap().ref_count, 2);

        events.borrow_mut().clear();
        let update = manager.update(|| Ok::<_, TestError>(())).unwrap();
        assert_eq!(update.processed_parent_unload, Some(id));
        assert_eq!(manager.parent_use_count(id), None);
        assert_eq!(
            events.borrow().as_slice(),
            [
                Event::SimulationDefer,
                Event::SimulationDeregister(id),
                Event::PresentationDeregister(id),
            ]
        );
    }

    #[test]
    fn parent_callback_enum_implements_the_resident_callback_contract() {
        fn assert_callback<C>()
        where
            C: ResidentCollectionCallback<CollectionParentManagerError<TestError>>,
        {
        }
        assert_callback::<ParentCollectionCallback<TestEffects>>();
    }
}
