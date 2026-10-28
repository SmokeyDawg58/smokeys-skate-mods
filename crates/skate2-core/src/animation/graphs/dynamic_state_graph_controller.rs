//! Evidence-bounded dynamic `StateGraph` fallback controller.
//!
//! This is the generic dynamic controller recovered at `0x82CF7EC0`; it is not
//! the generated ActionGraph/MotionGraph controller selected by default in the
//! matched build. Concrete condition, Behaviour, and `TransitionHook` effects
//! remain synchronous callback boundaries.

use crate::state_graph_runtime::{
    BaseId, ConditionActivationRequest, ConditionMask, Interruptable, LeafId, LeafKind,
    StateGraphActivationError, StateGraphConditionActivationBackend, StateGraphRuntime, StateId,
    TransitionId, TransitionPriority,
};
use std::{
    error::Error,
    fmt,
    sync::{Arc, Mutex, MutexGuard, TryLockError},
};

pub const DYNAMIC_STATE_GRAPH_HIERARCHY_CAPACITY: usize = 16;

#[derive(Clone, Copy, Debug)]
pub struct DynamicStateGraphControllerView<'a> {
    pub delta_time: f32,
    pub current: Option<StateId>,
    pub last: Option<StateId>,
    state_times: &'a [(StateId, f32)],
}

impl DynamicStateGraphControllerView<'_> {
    #[must_use]
    pub fn time_in_state(self, state: StateId) -> f32 {
        self.state_times
            .iter()
            .find_map(|(candidate, time)| (*candidate == state).then_some(*time))
            .unwrap_or(-1.0)
    }

    #[must_use]
    pub fn time_in_current_state(self) -> f32 {
        self.current
            .map_or(-1.0, |current| self.time_in_state(current))
    }
}

pub struct DynamicConditionActivationRequest<'a, P, C> {
    pub leaf: LeafId,
    pub registered_name: &'a [u8],
    pub payload: &'a mut P,
    pub context: &'a C,
    pub controller: DynamicStateGraphControllerView<'a>,
}

pub struct DynamicBehaviourCreateRequest<'a, P> {
    pub leaf: LeafId,
    pub registered_name: &'a [u8],
    pub payload: &'a mut P,
}

pub struct DynamicBehaviourCallbackRequest<'a, P, C, I> {
    pub leaf: LeafId,
    pub registered_name: &'a [u8],
    pub payload: &'a mut P,
    pub context: &'a mut C,
    pub controller: DynamicStateGraphControllerView<'a>,
    pub instance: Option<&'a mut I>,
}

/// Native-equivalent shallow copy boundary for the six-slot controller
/// `Context` passed to Behaviour callbacks.
///
/// Implementations must copy the direct slot/handle values. Mutating those
/// direct copied slots during a Behaviour callback must therefore be discarded
/// afterward, while mutations performed through copied shared pointers or
/// handles must remain visible through the original context. No blanket
/// `Clone` implementation is provided because a deep clone would overclaim the
/// recovered copy semantics.
pub trait DynamicStateGraphContext: Sized {
    #[must_use]
    fn copy_for_behaviour(&self) -> Self;
}

impl DynamicStateGraphContext for () {
    fn copy_for_behaviour(&self) -> Self {}
}

pub struct DynamicTransitionHookRequest<'a, P, C> {
    pub transition: TransitionId,
    pub hook: LeafId,
    pub registered_name: &'a [u8],
    pub payload: &'a mut P,
    pub context: &'a mut C,
    pub controller: DynamicStateGraphControllerView<'a>,
}

/// Concrete leaf boundary for the generic dynamic fallback controller.
///
/// `DynamicBehaviourCallbackRequest::context` and its separately supplied
/// `instance` jointly represent native's copied six-slot `Context` after its
/// `mInstance` slot at `+0x08` is replaced with the paired instance.
///
/// Native does not expose callback-failure recovery. This checked host boundary
/// nevertheless makes every collaborator call fallible; a failure stops after
/// the controller's already-scheduled and retained prefix, leaves any backend
/// effects performed before `Err` visible, and poisons the controller rather
/// than claiming that a partial native tick can be retried.
pub trait DynamicStateGraphBackend<P, C> {
    type Instance;
    type Error;

    /// Evaluates one enabled, matching-mask concrete condition.
    ///
    /// # Errors
    ///
    /// Returns the concrete condition backend's synchronous failure.
    fn condition_activation(
        &mut self,
        request: DynamicConditionActivationRequest<'_, P, C>,
    ) -> Result<bool, Self::Error>;

    /// Creates the native per-activation Behaviour instance, including a
    /// successful null instance.
    ///
    /// # Errors
    ///
    /// Returns the concrete Behaviour factory's synchronous failure.
    fn create_instance(
        &mut self,
        request: DynamicBehaviourCreateRequest<'_, P>,
    ) -> Result<Option<Self::Instance>, Self::Error>;

    /// Begins one already-appended active Behaviour entry.
    ///
    /// # Errors
    ///
    /// Returns the concrete Behaviour backend's synchronous failure.
    fn begin(
        &mut self,
        request: DynamicBehaviourCallbackRequest<'_, P, C, Self::Instance>,
    ) -> Result<(), Self::Error>;

    /// Updates one active Behaviour in forward insertion order.
    ///
    /// # Errors
    ///
    /// Returns the concrete Behaviour backend's synchronous failure.
    fn update(
        &mut self,
        request: DynamicBehaviourCallbackRequest<'_, P, C, Self::Instance>,
    ) -> Result<(), Self::Error>;

    /// Ends one active Behaviour in reverse insertion order.
    ///
    /// # Errors
    ///
    /// Returns the concrete Behaviour backend's synchronous failure.
    fn end(
        &mut self,
        request: DynamicBehaviourCallbackRequest<'_, P, C, Self::Instance>,
    ) -> Result<(), Self::Error>;

    /// Invokes one transition hook in stored order.
    ///
    /// # Errors
    ///
    /// Returns the concrete transition-hook backend's synchronous failure.
    fn on_transition(
        &mut self,
        request: DynamicTransitionHookRequest<'_, P, C>,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DynamicStateGraphInvariantError {
    GraphNotProcessed,
    GraphBusy,
    GraphPoisoned,
    MissingRoot,
    InvalidState(StateId),
    InvalidTransition(TransitionId),
    MissingTransitionTarget(TransitionId),
    MissingInterruptTarget(StateId),
    InvalidParentState {
        state: StateId,
        parent: StateId,
    },
    ParentCycle(StateId),
    InvalidAncestorBoundary {
        descendant: StateId,
        ancestor: StateId,
    },
    HierarchyCapacity {
        required: usize,
        maximum: usize,
    },
    NoNextState,
    InvalidBehaviourLeaf {
        state: StateId,
        leaf: LeafId,
    },
    InvalidHookLeaf {
        transition: TransitionId,
        leaf: LeafId,
    },
    UninitializedLeaf(LeafId),
    TimerCapacity {
        required: usize,
        maximum: usize,
    },
    MissingExitTimer(StateId),
    Poisoned,
}

impl fmt::Display for DynamicStateGraphInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GraphBusy => {
                formatter.write_str("dynamic graph is already borrowed by a callback or controller")
            }
            Self::GraphPoisoned => formatter.write_str("dynamic graph mutex is poisoned"),
            Self::GraphNotProcessed => formatter.write_str(
                "dynamic StateGraph requires a successfully completed structural Process",
            ),
            Self::MissingRoot => formatter.write_str("dynamic StateGraph has no root State"),
            Self::InvalidState(state) => {
                write!(formatter, "invalid dynamic StateGraph State {state:?}")
            }
            Self::InvalidTransition(transition) => {
                write!(
                    formatter,
                    "invalid dynamic StateGraph Transition {transition:?}"
                )
            }
            Self::MissingTransitionTarget(transition) => write!(
                formatter,
                "dynamic StateGraph Transition {transition:?} has no processed target"
            ),
            Self::MissingInterruptTarget(state) => write!(
                formatter,
                "dynamic StateGraph State {state:?} has no processed interrupt target"
            ),
            Self::InvalidParentState { state, parent } => write!(
                formatter,
                "dynamic StateGraph State {state:?} has invalid parent {parent:?}"
            ),
            Self::ParentCycle(state) => {
                write!(
                    formatter,
                    "dynamic StateGraph parent cycle reaches {state:?}"
                )
            }
            Self::InvalidAncestorBoundary {
                descendant,
                ancestor,
            } => write!(
                formatter,
                "State {ancestor:?} is not an ancestor boundary of {descendant:?}"
            ),
            Self::HierarchyCapacity { required, maximum } => write!(
                formatter,
                "dynamic StateGraph hierarchy slice needs {required} pointers; maximum is {maximum}"
            ),
            Self::NoNextState => formatter.write_str("dynamic StateGraph selected no next State"),
            Self::InvalidBehaviourLeaf { state, leaf } => write!(
                formatter,
                "State {state:?} has invalid Behaviour leaf {leaf:?}"
            ),
            Self::InvalidHookLeaf { transition, leaf } => write!(
                formatter,
                "Transition {transition:?} has invalid hook leaf {leaf:?}"
            ),
            Self::UninitializedLeaf(leaf) => {
                write!(formatter, "dynamic StateGraph leaf {leaf:?} has no payload")
            }
            Self::TimerCapacity { required, maximum } => write!(
                formatter,
                "dynamic StateGraph timer set needs {required} entries; maximum is {maximum}"
            ),
            Self::MissingExitTimer(state) => {
                write!(formatter, "exiting State {state:?} has no active timer")
            }
            Self::Poisoned => formatter.write_str("dynamic StateGraph controller is poisoned"),
        }
    }
}

impl Error for DynamicStateGraphInvariantError {}

#[derive(Debug)]
pub enum DynamicStateGraphLifecycleError<E> {
    CreateInstance {
        state: StateId,
        leaf: LeafId,
        source: E,
    },
    Begin {
        state: StateId,
        leaf: LeafId,
        source: E,
    },
    Update {
        state: StateId,
        leaf: LeafId,
        source: E,
    },
    ExitEnd {
        state: StateId,
        leaf: LeafId,
        source: E,
    },
    EndAllEnd {
        state: StateId,
        leaf: LeafId,
        source: E,
    },
    TransitionHook {
        transition: TransitionId,
        hook: LeafId,
        source: E,
    },
}

impl<E: fmt::Display> fmt::Display for DynamicStateGraphLifecycleError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateInstance {
                state,
                leaf,
                source,
            } => {
                write!(
                    formatter,
                    "State {state:?} Behaviour {leaf:?} CreateInstance failed: {source}"
                )
            }
            Self::Begin {
                state,
                leaf,
                source,
            } => {
                write!(
                    formatter,
                    "State {state:?} Behaviour {leaf:?} Begin failed: {source}"
                )
            }
            Self::Update {
                state,
                leaf,
                source,
            } => {
                write!(
                    formatter,
                    "State {state:?} Behaviour {leaf:?} Update failed: {source}"
                )
            }
            Self::ExitEnd {
                state,
                leaf,
                source,
            } => {
                write!(
                    formatter,
                    "exiting State {state:?} Behaviour {leaf:?} End failed: {source}"
                )
            }
            Self::EndAllEnd {
                state,
                leaf,
                source,
            } => {
                write!(
                    formatter,
                    "end-all State {state:?} Behaviour {leaf:?} End failed: {source}"
                )
            }
            Self::TransitionHook {
                transition,
                hook,
                source,
            } => write!(
                formatter,
                "Transition {transition:?} hook {hook:?} failed: {source}"
            ),
        }
    }
}

impl<E: Error + 'static> Error for DynamicStateGraphLifecycleError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::CreateInstance { source, .. }
            | Self::Begin { source, .. }
            | Self::Update { source, .. }
            | Self::ExitEnd { source, .. }
            | Self::EndAllEnd { source, .. }
            | Self::TransitionHook { source, .. } => source,
        })
    }
}

#[derive(Debug)]
pub enum DynamicStateGraphControllerError<E> {
    Invariant(DynamicStateGraphInvariantError),
    Activation(StateGraphActivationError<E>),
    Lifecycle(DynamicStateGraphLifecycleError<E>),
}

impl<E: fmt::Display> fmt::Display for DynamicStateGraphControllerError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invariant(error) => error.fmt(formatter),
            Self::Activation(error) => error.fmt(formatter),
            Self::Lifecycle(error) => error.fmt(formatter),
        }
    }
}

impl<E: Error + 'static> Error for DynamicStateGraphControllerError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Invariant(error) => error,
            Self::Activation(error) => error,
            Self::Lifecycle(error) => error,
        })
    }
}

impl<E> From<DynamicStateGraphInvariantError> for DynamicStateGraphControllerError<E> {
    fn from(error: DynamicStateGraphInvariantError) -> Self {
        Self::Invariant(error)
    }
}

impl<E> From<DynamicStateGraphLifecycleError<E>> for DynamicStateGraphControllerError<E> {
    fn from(error: DynamicStateGraphLifecycleError<E>) -> Self {
        Self::Lifecycle(error)
    }
}

struct ActiveBehaviour<I> {
    leaf: LeafId,
    state: StateId,
    instance: Option<I>,
}

#[derive(Clone, Copy)]
enum BehaviourEndSite {
    Exit,
    EndAll,
}

pub struct DynamicStateGraphController<P, I, C> {
    graph: Arc<Mutex<StateGraphRuntime<P>>>,
    context: C,
    delta_time: f32,
    current: Option<StateId>,
    last: Option<StateId>,
    state_times: Vec<(StateId, f32)>,
    active_behaviours: Vec<ActiveBehaviour<I>>,
    poisoned: bool,
}

impl<P, I, C> DynamicStateGraphController<P, I, C> {
    /// Validates an already structurally processed graph and creates an empty
    /// dynamic-fallback controller.
    ///
    /// # Errors
    ///
    /// Returns a checked root, target, parent, or concrete leaf invariant.
    pub fn new(
        graph: StateGraphRuntime<P>,
        context: C,
    ) -> Result<Self, DynamicStateGraphInvariantError> {
        Self::new_shared(Arc::new(Mutex::new(graph)), context)
    }

    /// Uses the exact graph allocation retained by the native asset cache.
    /// Context, state timers and behaviour instances remain per controller.
    ///
    /// # Errors
    /// Returns existing graph invariants or an explicit shared access failure.
    pub fn new_shared(
        graph: Arc<Mutex<StateGraphRuntime<P>>>,
        context: C,
    ) -> Result<Self, DynamicStateGraphInvariantError> {
        {
            let retained = try_graph_lock(&graph)?;
            if !retained.process_succeeded() {
                return Err(DynamicStateGraphInvariantError::GraphNotProcessed);
            }
            validate_graph(&retained)?;
        }
        Ok(Self {
            graph,
            context,
            delta_time: 0.0,
            current: None,
            last: None,
            state_times: Vec::with_capacity(DYNAMIC_STATE_GRAPH_HIERARCHY_CAPACITY),
            active_behaviours: Vec::new(),
            poisoned: false,
        })
    }

    #[must_use]
    pub const fn delta_time(&self) -> f32 {
        self.delta_time
    }

    #[must_use]
    pub const fn current_state(&self) -> Option<StateId> {
        self.current
    }

    #[must_use]
    pub const fn last_state(&self) -> Option<StateId> {
        self.last
    }

    #[must_use]
    pub fn time_in_state(&self, state: StateId) -> f32 {
        self.view().time_in_state(state)
    }

    #[must_use]
    pub fn time_in_current_state(&self) -> f32 {
        self.view().time_in_current_state()
    }

    #[must_use]
    pub fn active_behaviours(&self) -> impl ExactSizeIterator<Item = (LeafId, Option<&I>)> {
        self.active_behaviours
            .iter()
            .map(|active| (active.leaf, active.instance.as_ref()))
    }

    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Borrows the same cached graph without waiting on recursive callbacks.
    ///
    /// # Errors
    /// Returns a typed busy/poisoned access boundary.
    pub fn graph(
        &self,
    ) -> Result<MutexGuard<'_, StateGraphRuntime<P>>, DynamicStateGraphInvariantError> {
        try_graph_lock(&self.graph)
    }

    #[must_use]
    pub fn shared_graph(&self) -> &Arc<Mutex<StateGraphRuntime<P>>> {
        &self.graph
    }

    #[must_use]
    pub const fn context(&self) -> &C {
        &self.context
    }

    /// Runs one exact generic dynamic-controller tick.
    ///
    /// # Errors
    ///
    /// Returns a checked activation, lifecycle collaborator failure, or
    /// fixed-container invariant. Any failure poisons the controller because
    /// retry semantics are not recovered.
    ///
    /// # Panics
    ///
    /// Backend callbacks may panic according to their implementation. The
    /// controller does not intercept unwinding or claim native recovery.
    #[allow(clippy::too_many_lines)] // Keep recovered per-frame order visibly contiguous.
    pub fn update<B>(
        &mut self,
        backend: &mut B,
        dt: f32,
    ) -> Result<(), DynamicStateGraphControllerError<B::Error>>
    where
        C: DynamicStateGraphContext,
        B: DynamicStateGraphBackend<P, C, Instance = I>,
    {
        if self.poisoned {
            return Err(DynamicStateGraphInvariantError::Poisoned.into());
        }
        self.delta_time = dt;
        let graph_owner = Arc::clone(&self.graph);
        let mut graph = match try_graph_lock(&graph_owner) {
            Ok(graph) => graph,
            Err(error) => return self.poison(error.into()),
        };

        let selection = {
            let view = DynamicStateGraphControllerView {
                delta_time: self.delta_time,
                current: self.current,
                last: self.last,
                state_times: &self.state_times,
            };
            let mut adapter = DynamicConditionAdapter { backend, view };
            choose_next(&mut graph, self.current, &self.context, &mut adapter)
        };
        let (transition, next) = match selection {
            Ok((transition, Some(next))) => (transition, next),
            Ok((_, None)) => {
                return self.poison(DynamicStateGraphInvariantError::NoNextState.into());
            }
            Err(error) => return self.poison(error),
        };
        let plan = match build_update_plan(&graph, self.current, transition, next)
            .and_then(|plan| self.validate_plan(&graph, &plan).map(|()| plan))
        {
            Ok(plan) => plan,
            Err(error) => return self.poison(error.into()),
        };

        self.last = self.current;
        self.current = Some(next);
        for (_, time) in &mut self.state_times {
            *time += dt;
        }

        if let Err(error) = self.end_exit_behaviours(&mut graph, backend, &plan.exit_states) {
            return self.poison(error.into());
        }
        for state in &plan.exit_states {
            let index = self
                .state_times
                .iter()
                .position(|(candidate, _)| candidate == state)
                .expect("validated exit timer must remain present");
            self.state_times.remove(index);
        }

        if let Some(transition) = transition {
            let hooks = graph.transitions()[transition.0].hooks.clone();
            for hook in hooks {
                let view = DynamicStateGraphControllerView {
                    delta_time: self.delta_time,
                    current: self.current,
                    last: self.last,
                    state_times: &self.state_times,
                };
                let (registered_name, payload) = graph
                    .leaf_callback_parts_mut(hook)
                    .expect("validated hook payload");
                if let Err(source) = backend.on_transition(DynamicTransitionHookRequest {
                    transition,
                    hook,
                    registered_name,
                    payload,
                    context: &mut self.context,
                    controller: view,
                }) {
                    return self.poison(
                        DynamicStateGraphLifecycleError::TransitionHook {
                            transition,
                            hook,
                            source,
                        }
                        .into(),
                    );
                }
            }
        }

        // Native `UpdateEnterStates` completes this whole timer pass before
        // invoking any Behaviour CreateInstance/Begin callback.
        for state in &plan.enter_states {
            if let Some((_, time)) = self
                .state_times
                .iter_mut()
                .find(|(candidate, _)| candidate == state)
            {
                *time = 0.0;
            } else {
                self.state_times.push((*state, 0.0));
            }
        }
        for state in &plan.enter_states {
            let behaviours = graph.states()[state.0].behaviours.clone();
            for leaf in behaviours {
                if !graph.leaves()[leaf.0].base.enabled {
                    continue;
                }
                let instance = {
                    let (registered_name, payload) = graph
                        .leaf_callback_parts_mut(leaf)
                        .expect("validated Behaviour payload");
                    backend.create_instance(DynamicBehaviourCreateRequest {
                        leaf,
                        registered_name,
                        payload,
                    })
                };
                let instance = match instance {
                    Ok(instance) => instance,
                    Err(source) => {
                        return self.poison(
                            DynamicStateGraphLifecycleError::CreateInstance {
                                state: *state,
                                leaf,
                                source,
                            }
                            .into(),
                        );
                    }
                };
                self.active_behaviours.push(ActiveBehaviour {
                    leaf,
                    state: *state,
                    instance,
                });
                let mut callback_context = self.context.copy_for_behaviour();
                let view = DynamicStateGraphControllerView {
                    delta_time: self.delta_time,
                    current: self.current,
                    last: self.last,
                    state_times: &self.state_times,
                };
                let active = self.active_behaviours.last_mut().expect("just appended");
                let (registered_name, payload) = graph
                    .leaf_callback_parts_mut(leaf)
                    .expect("validated Behaviour payload");
                if let Err(source) = backend.begin(DynamicBehaviourCallbackRequest {
                    leaf,
                    registered_name,
                    payload,
                    context: &mut callback_context,
                    controller: view,
                    instance: active.instance.as_mut(),
                }) {
                    return self.poison(
                        DynamicStateGraphLifecycleError::Begin {
                            state: *state,
                            leaf,
                            source,
                        }
                        .into(),
                    );
                }
            }
        }

        for index in 0..self.active_behaviours.len() {
            let leaf = self.active_behaviours[index].leaf;
            let state = self.active_behaviours[index].state;
            let mut callback_context = self.context.copy_for_behaviour();
            let view = DynamicStateGraphControllerView {
                delta_time: self.delta_time,
                current: self.current,
                last: self.last,
                state_times: &self.state_times,
            };
            let active = &mut self.active_behaviours[index];
            let (registered_name, payload) = graph
                .leaf_callback_parts_mut(leaf)
                .expect("validated Behaviour payload");
            if let Err(source) = backend.update(DynamicBehaviourCallbackRequest {
                leaf,
                registered_name,
                payload,
                context: &mut callback_context,
                controller: view,
                instance: active.instance.as_mut(),
            }) {
                return self.poison(
                    DynamicStateGraphLifecycleError::Update {
                        state,
                        leaf,
                        source,
                    }
                    .into(),
                );
            }
        }
        Ok(())
    }

    /// Ends every active Behaviour in reverse insertion order. Native does not
    /// clear the retained State timer map in this operation.
    ///
    /// # Errors
    ///
    /// Returns the first reverse-order `End` failure and poisons the
    /// controller. Successfully ended suffix entries remain removed; the
    /// failing entry and untouched prefix remain retained in the poisoned
    /// controller.
    pub fn end_all_behaviours<B>(
        &mut self,
        backend: &mut B,
    ) -> Result<(), DynamicStateGraphControllerError<B::Error>>
    where
        C: DynamicStateGraphContext,
        B: DynamicStateGraphBackend<P, C, Instance = I>,
    {
        if self.poisoned {
            return Err(DynamicStateGraphInvariantError::Poisoned.into());
        }
        let graph_owner = Arc::clone(&self.graph);
        let mut graph = match try_graph_lock(&graph_owner) {
            Ok(graph) => graph,
            Err(error) => return self.poison(error.into()),
        };
        for index in (0..self.active_behaviours.len()).rev() {
            if let Err(error) =
                self.end_behaviour_at(&mut graph, backend, index, BehaviourEndSite::EndAll)
            {
                return self.poison(error.into());
            }
            self.active_behaviours.remove(index);
        }
        Ok(())
    }

    fn view(&self) -> DynamicStateGraphControllerView<'_> {
        DynamicStateGraphControllerView {
            delta_time: self.delta_time,
            current: self.current,
            last: self.last,
            state_times: &self.state_times,
        }
    }

    fn poison<E, T>(
        &mut self,
        error: DynamicStateGraphControllerError<E>,
    ) -> Result<T, DynamicStateGraphControllerError<E>> {
        self.poisoned = true;
        Err(error)
    }

    fn validate_plan(
        &self,
        graph: &StateGraphRuntime<P>,
        plan: &UpdatePlan,
    ) -> Result<(), DynamicStateGraphInvariantError> {
        for state in &plan.exit_states {
            if !self
                .state_times
                .iter()
                .any(|(candidate, _)| candidate == state)
            {
                return Err(DynamicStateGraphInvariantError::MissingExitTimer(*state));
            }
        }
        let mut final_states = self
            .state_times
            .iter()
            .filter(|(state, _)| !plan.exit_states.contains(state))
            .map(|(state, _)| *state)
            .collect::<Vec<_>>();
        for state in &plan.enter_states {
            if !final_states.contains(state) {
                final_states.push(*state);
            }
        }
        let required = final_states.len();
        if required > DYNAMIC_STATE_GRAPH_HIERARCHY_CAPACITY {
            return Err(DynamicStateGraphInvariantError::TimerCapacity {
                required,
                maximum: DYNAMIC_STATE_GRAPH_HIERARCHY_CAPACITY,
            });
        }
        for state in &plan.enter_states {
            validate_state_behaviours(graph, *state)?;
        }
        if let Some(transition) = plan.transition {
            validate_transition_hooks(graph, transition)?;
        }
        Ok(())
    }

    fn end_exit_behaviours<B>(
        &mut self,
        graph: &mut StateGraphRuntime<P>,
        backend: &mut B,
        exit_states: &[StateId],
    ) -> Result<(), DynamicStateGraphLifecycleError<B::Error>>
    where
        C: DynamicStateGraphContext,
        B: DynamicStateGraphBackend<P, C, Instance = I>,
    {
        for index in (0..self.active_behaviours.len()).rev() {
            if exit_states.contains(&self.active_behaviours[index].state) {
                self.end_behaviour_at(graph, backend, index, BehaviourEndSite::Exit)?;
                self.active_behaviours.remove(index);
            }
        }
        Ok(())
    }

    fn end_behaviour_at<B>(
        &mut self,
        graph: &mut StateGraphRuntime<P>,
        backend: &mut B,
        index: usize,
        site: BehaviourEndSite,
    ) -> Result<(), DynamicStateGraphLifecycleError<B::Error>>
    where
        C: DynamicStateGraphContext,
        B: DynamicStateGraphBackend<P, C, Instance = I>,
    {
        let leaf = self.active_behaviours[index].leaf;
        let state = self.active_behaviours[index].state;
        let mut callback_context = self.context.copy_for_behaviour();
        let view = DynamicStateGraphControllerView {
            delta_time: self.delta_time,
            current: self.current,
            last: self.last,
            state_times: &self.state_times,
        };
        let active = &mut self.active_behaviours[index];
        let (registered_name, payload) = graph
            .leaf_callback_parts_mut(leaf)
            .expect("validated Behaviour payload");
        backend
            .end(DynamicBehaviourCallbackRequest {
                leaf,
                registered_name,
                payload,
                context: &mut callback_context,
                controller: view,
                instance: active.instance.as_mut(),
            })
            .map_err(|source| match site {
                BehaviourEndSite::Exit => DynamicStateGraphLifecycleError::ExitEnd {
                    state,
                    leaf,
                    source,
                },
                BehaviourEndSite::EndAll => DynamicStateGraphLifecycleError::EndAllEnd {
                    state,
                    leaf,
                    source,
                },
            })
    }
}

fn try_graph_lock<P>(
    graph: &Mutex<StateGraphRuntime<P>>,
) -> Result<MutexGuard<'_, StateGraphRuntime<P>>, DynamicStateGraphInvariantError> {
    graph.try_lock().map_err(|error| match error {
        TryLockError::WouldBlock => DynamicStateGraphInvariantError::GraphBusy,
        TryLockError::Poisoned(_) => DynamicStateGraphInvariantError::GraphPoisoned,
    })
}

struct DynamicConditionAdapter<'backend, 'state, B> {
    backend: &'backend mut B,
    view: DynamicStateGraphControllerView<'state>,
}

impl<P, C, B> StateGraphConditionActivationBackend<P, C> for DynamicConditionAdapter<'_, '_, B>
where
    B: DynamicStateGraphBackend<P, C>,
{
    type Error = B::Error;

    fn activation_func(
        &mut self,
        request: ConditionActivationRequest<'_, P, C>,
    ) -> Result<bool, Self::Error> {
        self.backend
            .condition_activation(DynamicConditionActivationRequest {
                leaf: request.leaf,
                registered_name: request.registered_name,
                payload: request.payload,
                context: request.context,
                controller: self.view,
            })
    }
}

#[derive(Debug)]
struct UpdatePlan {
    transition: Option<TransitionId>,
    exit_states: Vec<StateId>,
    enter_states: Vec<StateId>,
}

fn validate_graph<P>(graph: &StateGraphRuntime<P>) -> Result<(), DynamicStateGraphInvariantError> {
    let root = graph
        .root()
        .ok_or(DynamicStateGraphInvariantError::MissingRoot)?;
    let mut visiting = vec![false; graph.states().len()];
    let mut visited = vec![false; graph.states().len()];
    validate_reachable_state(graph, root, &mut visiting, &mut visited)
}

fn validate_reachable_state<P>(
    graph: &StateGraphRuntime<P>,
    id: StateId,
    visiting: &mut [bool],
    visited: &mut [bool],
) -> Result<(), DynamicStateGraphInvariantError> {
    let node = state(graph, id)?;
    if visiting[id.0] {
        return Err(DynamicStateGraphInvariantError::ParentCycle(id));
    }
    if visited[id.0] {
        return Ok(());
    }
    visiting[id.0] = true;
    validate_parent_chain(graph, id)?;
    validate_state_behaviours(graph, id)?;
    if let Interruptable::StateInterruptable { target, .. } = node.interruptable {
        let target = target.ok_or(DynamicStateGraphInvariantError::MissingInterruptTarget(id))?;
        state(graph, target)?;
    }
    for transition_id in &node.transitions {
        let transition = transition(graph, *transition_id)?;
        if transition.parent_state != id {
            return Err(DynamicStateGraphInvariantError::InvalidTransition(
                *transition_id,
            ));
        }
        state(
            graph,
            transition
                .target
                .ok_or(DynamicStateGraphInvariantError::MissingTransitionTarget(
                    *transition_id,
                ))?,
        )?;
        validate_transition_hooks(graph, *transition_id)?;
    }
    let children = node.children.clone();
    for child in children {
        if state(graph, child)?.parent_state != Some(id) {
            return Err(DynamicStateGraphInvariantError::InvalidParentState {
                state: child,
                parent: id,
            });
        }
        validate_reachable_state(graph, child, visiting, visited)?;
    }
    visiting[id.0] = false;
    visited[id.0] = true;
    Ok(())
}

fn validate_parent_chain<P>(
    graph: &StateGraphRuntime<P>,
    start: StateId,
) -> Result<(), DynamicStateGraphInvariantError> {
    let mut current = Some(start);
    let mut visited = 0;
    while let Some(candidate) = current {
        visited += 1;
        if visited > graph.states().len() {
            return Err(DynamicStateGraphInvariantError::ParentCycle(candidate));
        }
        current = state(graph, candidate)?.parent_state;
    }
    Ok(())
}

fn validate_state_behaviours<P>(
    graph: &StateGraphRuntime<P>,
    state_id: StateId,
) -> Result<(), DynamicStateGraphInvariantError> {
    for leaf in &state(graph, state_id)?.behaviours {
        let Some(node) = graph.leaves().get(leaf.0) else {
            return Err(DynamicStateGraphInvariantError::InvalidBehaviourLeaf {
                state: state_id,
                leaf: *leaf,
            });
        };
        if node.kind != LeafKind::Behaviour || node.base.parent != Some(BaseId::State(state_id)) {
            return Err(DynamicStateGraphInvariantError::InvalidBehaviourLeaf {
                state: state_id,
                leaf: *leaf,
            });
        }
        if node.payload().is_none() {
            return Err(DynamicStateGraphInvariantError::UninitializedLeaf(*leaf));
        }
    }
    Ok(())
}

fn validate_transition_hooks<P>(
    graph: &StateGraphRuntime<P>,
    transition_id: TransitionId,
) -> Result<(), DynamicStateGraphInvariantError> {
    for leaf in &transition(graph, transition_id)?.hooks {
        let Some(node) = graph.leaves().get(leaf.0) else {
            return Err(DynamicStateGraphInvariantError::InvalidHookLeaf {
                transition: transition_id,
                leaf: *leaf,
            });
        };
        if node.kind != LeafKind::Hook
            || node.base.parent != Some(BaseId::Transition(transition_id))
        {
            return Err(DynamicStateGraphInvariantError::InvalidHookLeaf {
                transition: transition_id,
                leaf: *leaf,
            });
        }
        if node.payload().is_none() {
            return Err(DynamicStateGraphInvariantError::UninitializedLeaf(*leaf));
        }
    }
    Ok(())
}

fn state<P>(
    graph: &StateGraphRuntime<P>,
    id: StateId,
) -> Result<&crate::state_graph_runtime::StateNode, DynamicStateGraphInvariantError> {
    graph
        .states()
        .get(id.0)
        .ok_or(DynamicStateGraphInvariantError::InvalidState(id))
}

fn transition<P>(
    graph: &StateGraphRuntime<P>,
    id: TransitionId,
) -> Result<&crate::state_graph_runtime::TransitionNode, DynamicStateGraphInvariantError> {
    graph
        .transitions()
        .get(id.0)
        .ok_or(DynamicStateGraphInvariantError::InvalidTransition(id))
}

fn is_ancestor<P>(
    graph: &StateGraphRuntime<P>,
    ancestor: StateId,
    descendant: Option<StateId>,
) -> Result<bool, DynamicStateGraphInvariantError> {
    let mut current = descendant;
    let mut visited = 0;
    while let Some(candidate) = current {
        if candidate == ancestor {
            return Ok(true);
        }
        visited += 1;
        if visited > graph.states().len() {
            return Err(DynamicStateGraphInvariantError::ParentCycle(candidate));
        }
        current = state(graph, candidate)?.parent_state;
    }
    Ok(false)
}

fn hierarchy_slice<P>(
    graph: &StateGraphRuntime<P>,
    descendant: Option<StateId>,
    ancestor: Option<StateId>,
) -> Result<Vec<StateId>, DynamicStateGraphInvariantError> {
    let original = descendant;
    let mut current = descendant;
    let mut reverse = Vec::new();
    let mut visited = 0;
    while current != ancestor {
        let Some(candidate) = current else {
            if let (Some(descendant), Some(ancestor)) = (original, ancestor) {
                return Err(DynamicStateGraphInvariantError::InvalidAncestorBoundary {
                    descendant,
                    ancestor,
                });
            }
            break;
        };
        reverse.push(candidate);
        if reverse.len() > DYNAMIC_STATE_GRAPH_HIERARCHY_CAPACITY {
            return Err(DynamicStateGraphInvariantError::HierarchyCapacity {
                required: reverse.len(),
                maximum: DYNAMIC_STATE_GRAPH_HIERARCHY_CAPACITY,
            });
        }
        visited += 1;
        if visited > graph.states().len() {
            return Err(DynamicStateGraphInvariantError::ParentCycle(candidate));
        }
        current = state(graph, candidate)?.parent_state;
    }
    reverse.reverse();
    Ok(reverse)
}

fn deepest_common_ancestor<P>(
    graph: &StateGraphRuntime<P>,
    a: Option<StateId>,
    b: Option<StateId>,
) -> Result<Option<StateId>, DynamicStateGraphInvariantError> {
    let mut a_ancestors = Vec::new();
    let mut cursor = a;
    let mut visited = 0;
    while let Some(candidate) = cursor {
        a_ancestors.push(candidate);
        visited += 1;
        if visited > graph.states().len() {
            return Err(DynamicStateGraphInvariantError::ParentCycle(candidate));
        }
        cursor = state(graph, candidate)?.parent_state;
    }
    cursor = b;
    visited = 0;
    while let Some(candidate) = cursor {
        if a_ancestors.contains(&candidate) {
            return Ok(Some(candidate));
        }
        visited += 1;
        if visited > graph.states().len() {
            return Err(DynamicStateGraphInvariantError::ParentCycle(candidate));
        }
        cursor = state(graph, candidate)?.parent_state;
    }
    Ok(None)
}

fn build_update_plan<P>(
    graph: &StateGraphRuntime<P>,
    current: Option<StateId>,
    selected_transition: Option<TransitionId>,
    next: StateId,
) -> Result<UpdatePlan, DynamicStateGraphInvariantError> {
    let common = if let Some(transition_id) = selected_transition {
        let target = transition(graph, transition_id)?.target.ok_or(
            DynamicStateGraphInvariantError::MissingTransitionTarget(transition_id),
        )?;
        let adjusted_target = if is_ancestor(graph, target, current)? {
            state(graph, target)?.parent_state
        } else {
            Some(target)
        };
        deepest_common_ancestor(graph, current, adjusted_target)?
    } else {
        deepest_common_ancestor(graph, current, Some(next))?
    };
    let exit_states = hierarchy_slice(graph, current, common)?;
    let enter_states = hierarchy_slice(graph, Some(next), common)?;
    Ok(UpdatePlan {
        transition: selected_transition,
        exit_states,
        enter_states,
    })
}

fn choose_next<P, C, A>(
    graph: &mut StateGraphRuntime<P>,
    current: Option<StateId>,
    context: &C,
    activation: &mut A,
) -> SelectionResult<A::Error>
where
    A: StateGraphConditionActivationBackend<P, C>,
{
    let root = graph
        .root()
        .ok_or(DynamicStateGraphInvariantError::MissingRoot)?;
    let Some(current) = current else {
        let next = evaluate_by_priority(
            graph,
            None,
            root,
            ConditionMask::PreCond,
            context,
            activation,
        )?;
        return Ok((None, next));
    };
    if let Some(found) = search_transitions(graph, current, context, activation)? {
        return Ok((Some(found.0), Some(found.1)));
    }
    let next = evaluate_state(graph, root, current, context, activation)?;
    Ok((None, next))
}

type SelectionResult<E> =
    Result<(Option<TransitionId>, Option<StateId>), DynamicStateGraphControllerError<E>>;

fn evaluate_state<P, C, A>(
    graph: &mut StateGraphRuntime<P>,
    root: StateId,
    current: StateId,
    context: &C,
    activation: &mut A,
) -> Result<Option<StateId>, DynamicStateGraphControllerError<A::Error>>
where
    A: StateGraphConditionActivationBackend<P, C>,
{
    let interruptable = state(graph, current)?.interruptable.clone();
    match interruptable {
        Interruptable::NotInterruptable => {
            if graph
                .state_activation_parent(current, ConditionMask::Sustain, context, activation)
                .map_err(DynamicStateGraphControllerError::Activation)?
            {
                Ok(Some(current))
            } else {
                evaluate_by_priority(
                    graph,
                    Some(current),
                    root,
                    ConditionMask::Sustain,
                    context,
                    activation,
                )
            }
        }
        Interruptable::RootInterruptable => evaluate_by_priority(
            graph,
            Some(current),
            root,
            ConditionMask::Sustain,
            context,
            activation,
        ),
        Interruptable::StateInterruptable { target, .. } => {
            let target = target.ok_or(DynamicStateGraphInvariantError::MissingInterruptTarget(
                current,
            ))?;
            if graph
                .state_activation_parent(target, ConditionMask::Sustain, context, activation)
                .map_err(DynamicStateGraphControllerError::Activation)?
                && let Some(next) = evaluate_by_priority(
                    graph,
                    Some(current),
                    target,
                    ConditionMask::Sustain,
                    context,
                    activation,
                )?
            {
                return Ok(Some(next));
            }
            evaluate_by_priority(
                graph,
                Some(current),
                root,
                ConditionMask::Sustain,
                context,
                activation,
            )
        }
    }
}

fn evaluate_by_priority<P, C, A>(
    graph: &mut StateGraphRuntime<P>,
    current: Option<StateId>,
    input: StateId,
    requested_mask: ConditionMask,
    context: &C,
    activation: &mut A,
) -> Result<Option<StateId>, DynamicStateGraphControllerError<A::Error>>
where
    A: StateGraphConditionActivationBackend<P, C>,
{
    let actual_mask =
        if requested_mask == ConditionMask::Sustain && is_ancestor(graph, input, current)? {
            ConditionMask::Sustain
        } else {
            ConditionMask::PreCond
        };
    if !graph
        .state_activation(input, actual_mask, context, activation)
        .map_err(DynamicStateGraphControllerError::Activation)?
    {
        return Ok(None);
    }
    let children = state(graph, input)?.children.clone();
    if children.is_empty() {
        return Ok(Some(input));
    }
    for child in children {
        let node = state(graph, child)?;
        if node.base.enabled
            && node.active
            && let Some(next) =
                evaluate_by_priority(graph, current, child, requested_mask, context, activation)?
        {
            return Ok(Some(next));
        }
    }
    Ok(None)
}

fn search_transitions<P, C, A>(
    graph: &mut StateGraphRuntime<P>,
    current: StateId,
    context: &C,
    activation: &mut A,
) -> Result<Option<(TransitionId, StateId)>, DynamicStateGraphControllerError<A::Error>>
where
    A: StateGraphConditionActivationBackend<P, C>,
{
    for priority in [
        TransitionPriority::Urgent,
        TransitionPriority::High,
        TransitionPriority::Medium,
        TransitionPriority::Low,
    ] {
        let mut owner = Some(current);
        while let Some(state_id) = owner {
            let transitions = state(graph, state_id)?.transitions.clone();
            for transition_id in transitions {
                let node = transition(graph, transition_id)?;
                let target =
                    node.target
                        .ok_or(DynamicStateGraphInvariantError::MissingTransitionTarget(
                            transition_id,
                        ))?;
                let candidate = node.priority == priority
                    && node.base.enabled
                    && state(graph, target)?.base.enabled;
                if candidate
                    && graph
                        .transition_activation(
                            transition_id,
                            ConditionMask::PreCond,
                            context,
                            activation,
                        )
                        .map_err(DynamicStateGraphControllerError::Activation)?
                    && valid_target(graph, current, target, context, activation)?
                    && let Some(next) = evaluate_by_priority(
                        graph,
                        Some(current),
                        target,
                        ConditionMask::PreCond,
                        context,
                        activation,
                    )?
                {
                    return Ok(Some((transition_id, next)));
                }
            }
            owner = state(graph, state_id)?.parent_state;
        }
    }
    Ok(None)
}

fn valid_target<P, C, A>(
    graph: &mut StateGraphRuntime<P>,
    current: StateId,
    target: StateId,
    context: &C,
    activation: &mut A,
) -> Result<bool, DynamicStateGraphControllerError<A::Error>>
where
    A: StateGraphConditionActivationBackend<P, C>,
{
    let current_path = hierarchy_slice(graph, Some(current), None)?;
    let target_path = hierarchy_slice(graph, Some(target), None)?;
    for candidate in target_path.into_iter().rev() {
        if current_path.contains(&candidate) && candidate != target {
            return graph
                .state_activation_parent(candidate, ConditionMask::Sustain, context, activation)
                .map_err(DynamicStateGraphControllerError::Activation);
        }
        if !graph
            .state_activation(candidate, ConditionMask::PreCond, context, activation)
            .map_err(DynamicStateGraphControllerError::Activation)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_graph_runtime::{
        ExpressionChildId, FactoryMissPolicy, LeafCreateRequest, StateGraphLeafFactory,
    };
    use skate2_formats::state_graph::{
        StateGraphBinaryAttribute, StateGraphBinaryElement, state_graph_string_hash32,
    };
    use std::{cell::Cell, collections::BTreeSet, convert::Infallible, rc::Rc};

    #[derive(Clone, Debug, Default)]
    struct Payload {
        calls: Vec<&'static str>,
    }

    struct Factory {
        condition: Vec<Vec<u8>>,
        behaviour: Vec<Vec<u8>>,
        hook: Vec<Vec<u8>>,
    }

    impl Default for Factory {
        fn default() -> Self {
            Self {
                condition: vec![b"Cond".to_vec()],
                behaviour: vec![b"Beh".to_vec()],
                hook: vec![b"Hook".to_vec()],
            }
        }
    }

    impl StateGraphLeafFactory for Factory {
        type Payload = Payload;
        type Error = Infallible;

        fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
            match kind {
                LeafKind::Condition => &self.condition,
                LeafKind::Behaviour => &self.behaviour,
                LeafKind::Hook => &self.hook,
            }
        }

        fn create(&mut self, _request: LeafCreateRequest<'_>) -> Result<Payload, Infallible> {
            Ok(Payload::default())
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum CallbackKind {
        Condition,
        Create,
        Begin,
        Update,
        End,
        Hook,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct CallbackEvent {
        kind: CallbackKind,
        leaf: LeafId,
        instance: Option<u32>,
        current: Option<StateId>,
        last: Option<StateId>,
        times: Vec<u32>,
    }

    #[derive(Default)]
    struct Backend {
        events: Vec<CallbackEvent>,
        watched_states: Vec<StateId>,
        false_conditions: BTreeSet<usize>,
        null_instances: BTreeSet<usize>,
        next_instance: u32,
    }

    impl Backend {
        fn capture(
            &mut self,
            kind: CallbackKind,
            leaf: LeafId,
            instance: Option<u32>,
            controller: DynamicStateGraphControllerView<'_>,
        ) {
            self.events.push(CallbackEvent {
                kind,
                leaf,
                instance,
                current: controller.current,
                last: controller.last,
                times: self
                    .watched_states
                    .iter()
                    .map(|state| controller.time_in_state(*state).to_bits())
                    .collect(),
            });
        }
    }

    impl DynamicStateGraphBackend<Payload, ()> for Backend {
        type Instance = u32;
        type Error = Infallible;

        fn condition_activation(
            &mut self,
            request: DynamicConditionActivationRequest<'_, Payload, ()>,
        ) -> Result<bool, Infallible> {
            self.capture(
                CallbackKind::Condition,
                request.leaf,
                None,
                request.controller,
            );
            Ok(!self.false_conditions.contains(&request.leaf.0))
        }

        fn create_instance(
            &mut self,
            request: DynamicBehaviourCreateRequest<'_, Payload>,
        ) -> Result<Option<u32>, Infallible> {
            let instance = if self.null_instances.contains(&request.leaf.0) {
                None
            } else {
                self.next_instance += 1;
                Some(self.next_instance)
            };
            self.events.push(CallbackEvent {
                kind: CallbackKind::Create,
                leaf: request.leaf,
                instance,
                current: None,
                last: None,
                times: Vec::new(),
            });
            Ok(instance)
        }

        fn begin(
            &mut self,
            request: DynamicBehaviourCallbackRequest<'_, Payload, (), u32>,
        ) -> Result<(), Infallible> {
            let instance = request.instance.map(|instance| *instance);
            self.capture(
                CallbackKind::Begin,
                request.leaf,
                instance,
                request.controller,
            );
            Ok(())
        }

        fn update(
            &mut self,
            request: DynamicBehaviourCallbackRequest<'_, Payload, (), u32>,
        ) -> Result<(), Infallible> {
            let instance = request.instance.map(|instance| *instance);
            self.capture(
                CallbackKind::Update,
                request.leaf,
                instance,
                request.controller,
            );
            Ok(())
        }

        fn end(
            &mut self,
            request: DynamicBehaviourCallbackRequest<'_, Payload, (), u32>,
        ) -> Result<(), Infallible> {
            let instance = request.instance.map(|instance| *instance);
            self.capture(
                CallbackKind::End,
                request.leaf,
                instance,
                request.controller,
            );
            Ok(())
        }

        fn on_transition(
            &mut self,
            request: DynamicTransitionHookRequest<'_, Payload, ()>,
        ) -> Result<(), Infallible> {
            self.capture(CallbackKind::Hook, request.hook, None, request.controller);
            Ok(())
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct InjectedBackendError {
        kind: CallbackKind,
        leaf: LeafId,
    }

    impl fmt::Display for InjectedBackendError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "injected {:?} failure for {:?}",
                self.kind, self.leaf
            )
        }
    }

    impl Error for InjectedBackendError {}

    struct FailingBackend {
        inner: Backend,
        failure: (CallbackKind, LeafId),
    }

    impl FailingBackend {
        fn result(&self, kind: CallbackKind, leaf: LeafId) -> Result<(), InjectedBackendError> {
            if self.failure == (kind, leaf) {
                Err(InjectedBackendError { kind, leaf })
            } else {
                Ok(())
            }
        }
    }

    impl DynamicStateGraphBackend<Payload, ()> for FailingBackend {
        type Instance = u32;
        type Error = InjectedBackendError;

        fn condition_activation(
            &mut self,
            request: DynamicConditionActivationRequest<'_, Payload, ()>,
        ) -> Result<bool, Self::Error> {
            Ok(self.inner.condition_activation(request).unwrap())
        }

        fn create_instance(
            &mut self,
            request: DynamicBehaviourCreateRequest<'_, Payload>,
        ) -> Result<Option<Self::Instance>, Self::Error> {
            let leaf = request.leaf;
            let instance = self.inner.create_instance(request).unwrap();
            self.result(CallbackKind::Create, leaf)?;
            Ok(instance)
        }

        fn begin(
            &mut self,
            request: DynamicBehaviourCallbackRequest<'_, Payload, (), Self::Instance>,
        ) -> Result<(), Self::Error> {
            let leaf = request.leaf;
            self.inner.begin(request).unwrap();
            self.result(CallbackKind::Begin, leaf)
        }

        fn update(
            &mut self,
            request: DynamicBehaviourCallbackRequest<'_, Payload, (), Self::Instance>,
        ) -> Result<(), Self::Error> {
            let leaf = request.leaf;
            self.inner.update(request).unwrap();
            self.result(CallbackKind::Update, leaf)
        }

        fn end(
            &mut self,
            request: DynamicBehaviourCallbackRequest<'_, Payload, (), Self::Instance>,
        ) -> Result<(), Self::Error> {
            let leaf = request.leaf;
            self.inner.end(request).unwrap();
            self.result(CallbackKind::End, leaf)
        }

        fn on_transition(
            &mut self,
            request: DynamicTransitionHookRequest<'_, Payload, ()>,
        ) -> Result<(), Self::Error> {
            let hook = request.hook;
            self.inner.on_transition(request).unwrap();
            self.result(CallbackKind::Hook, hook)
        }
    }

    #[derive(Debug, Default)]
    struct BoundContext {
        value: u32,
        shared: Rc<Cell<u32>>,
    }

    impl DynamicStateGraphContext for BoundContext {
        fn copy_for_behaviour(&self) -> Self {
            Self {
                value: self.value,
                shared: Rc::clone(&self.shared),
            }
        }
    }

    #[derive(Default)]
    struct ContextBackend {
        observed: Vec<(&'static str, u32, u32)>,
        next_instance: u32,
        failure: Option<(CallbackKind, LeafId)>,
    }

    impl ContextBackend {
        fn callback_result(
            &self,
            kind: CallbackKind,
            leaf: LeafId,
        ) -> Result<(), InjectedBackendError> {
            if self.failure == Some((kind, leaf)) {
                Err(InjectedBackendError { kind, leaf })
            } else {
                Ok(())
            }
        }
    }

    impl DynamicStateGraphBackend<Payload, BoundContext> for ContextBackend {
        type Instance = u32;
        type Error = InjectedBackendError;

        fn condition_activation(
            &mut self,
            request: DynamicConditionActivationRequest<'_, Payload, BoundContext>,
        ) -> Result<bool, Self::Error> {
            request.payload.calls.push("condition");
            self.observed.push((
                "condition",
                request.context.value,
                request.context.shared.get(),
            ));
            Ok(true)
        }

        fn create_instance(
            &mut self,
            request: DynamicBehaviourCreateRequest<'_, Payload>,
        ) -> Result<Option<u32>, Self::Error> {
            let leaf = request.leaf;
            request.payload.calls.push("create");
            self.next_instance += 1;
            self.callback_result(CallbackKind::Create, leaf)?;
            Ok(Some(self.next_instance))
        }

        fn begin(
            &mut self,
            request: DynamicBehaviourCallbackRequest<'_, Payload, BoundContext, u32>,
        ) -> Result<(), Self::Error> {
            let leaf = request.leaf;
            request.payload.calls.push("begin");
            self.observed
                .push(("begin", request.context.value, request.context.shared.get()));
            request.context.value += 100;
            request
                .context
                .shared
                .set(request.context.shared.get() + 100);
            if self.failure == Some((CallbackKind::Begin, leaf))
                && let Some(instance) = request.instance
            {
                *instance += 1_000;
            }
            self.callback_result(CallbackKind::Begin, leaf)
        }

        fn update(
            &mut self,
            request: DynamicBehaviourCallbackRequest<'_, Payload, BoundContext, u32>,
        ) -> Result<(), Self::Error> {
            let leaf = request.leaf;
            request.payload.calls.push("update");
            self.observed.push((
                "update",
                request.context.value,
                request.context.shared.get(),
            ));
            request.context.value += 200;
            request
                .context
                .shared
                .set(request.context.shared.get() + 200);
            self.callback_result(CallbackKind::Update, leaf)
        }

        fn end(
            &mut self,
            request: DynamicBehaviourCallbackRequest<'_, Payload, BoundContext, u32>,
        ) -> Result<(), Self::Error> {
            let leaf = request.leaf;
            request.payload.calls.push("end");
            self.observed
                .push(("end", request.context.value, request.context.shared.get()));
            request.context.value += 300;
            request
                .context
                .shared
                .set(request.context.shared.get() + 300);
            self.callback_result(CallbackKind::End, leaf)
        }

        fn on_transition(
            &mut self,
            request: DynamicTransitionHookRequest<'_, Payload, BoundContext>,
        ) -> Result<(), Self::Error> {
            let hook = request.hook;
            request.payload.calls.push("hook");
            self.observed
                .push(("hook", request.context.value, request.context.shared.get()));
            request.context.value += 1;
            request.context.shared.set(request.context.shared.get() + 1);
            self.callback_result(CallbackKind::Hook, hook)
        }
    }

    fn attribute(key: &[u8], text: &[u8], boolean_raw: u8) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits: 0,
            boolean_raw,
        }
    }

    fn element(
        name: &[u8],
        attributes: Vec<StateGraphBinaryAttribute>,
        children: Vec<StateGraphBinaryElement>,
    ) -> StateGraphBinaryElement {
        StateGraphBinaryElement {
            name: name.to_vec(),
            attributes,
            children,
        }
    }

    fn state(
        name: &[u8],
        attributes: Vec<StateGraphBinaryAttribute>,
        children: Vec<StateGraphBinaryElement>,
    ) -> StateGraphBinaryElement {
        let mut all_attributes = vec![attribute(b"name", name, 0)];
        all_attributes.extend(attributes);
        element(b"state", all_attributes, children)
    }

    fn expression(_value: bool) -> StateGraphBinaryElement {
        element(
            b"expression",
            Vec::new(),
            vec![element(
                b"condition",
                vec![attribute(b"name", b"Cond", 0)],
                Vec::new(),
            )],
        )
    }

    fn expression_mask(mask: &[u8]) -> StateGraphBinaryElement {
        element(
            b"expression",
            Vec::new(),
            vec![element(
                b"condition",
                vec![attribute(b"name", b"Cond", 0), attribute(b"mask", mask, 0)],
                Vec::new(),
            )],
        )
    }

    fn behaviour(enabled: bool) -> StateGraphBinaryElement {
        let mut attributes = vec![attribute(b"name", b"Beh", 0)];
        if !enabled {
            attributes.push(attribute(b"enabled", b"false", 0));
        }
        element(b"behaviour", attributes, Vec::new())
    }

    fn hook(enabled: bool) -> StateGraphBinaryElement {
        let mut attributes = vec![attribute(b"name", b"Hook", 0)];
        if !enabled {
            attributes.push(attribute(b"enabled", b"false", 0));
        }
        element(b"hook", attributes, Vec::new())
    }

    fn transition_element(
        target: &[u8],
        priority: &[u8],
        hook_count: usize,
    ) -> StateGraphBinaryElement {
        let mut children = vec![expression(true)];
        children.extend((0..hook_count).map(|_| hook(false)));
        element(
            b"transition",
            vec![
                attribute(b"target", target, 0),
                attribute(b"priority", priority, 0),
            ],
            children,
        )
    }

    fn named_chain(
        prefix: &str,
        count: usize,
        deepest_children: Vec<StateGraphBinaryElement>,
    ) -> StateGraphBinaryElement {
        let deepest_name = format!("{prefix}{}", count - 1);
        let mut node = state(deepest_name.as_bytes(), Vec::new(), deepest_children);
        for index in (0..count - 1).rev() {
            let name = format!("{prefix}{index}");
            node = state(name.as_bytes(), Vec::new(), vec![node]);
        }
        node
    }

    #[allow(clippy::needless_pass_by_value)] // Keeps nested fixture construction concise.
    fn build(root: StateGraphBinaryElement) -> StateGraphRuntime<Payload> {
        let mut factory = Factory::default();
        let mut graph = StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict)
            .expect("fixture construction");
        graph
            .process_after_build(&mut factory)
            .expect("fixture processing");
        graph
    }

    fn callback_kinds(events: &[CallbackEvent]) -> Vec<(CallbackKind, LeafId, Option<u32>)> {
        events
            .iter()
            .map(|event| (event.kind, event.leaf, event.instance))
            .collect()
    }

    fn only_condition(graph: &StateGraphRuntime<Payload>, state: StateId) -> LeafId {
        let expression = graph.states()[state.0].expression.unwrap();
        match graph.expressions()[expression.0].children[0] {
            ExpressionChildId::Condition(condition) => condition,
            ExpressionChildId::Expression(_) => panic!("fixture expected a Condition child"),
        }
    }

    fn active_leaf_ids(controller: &DynamicStateGraphController<Payload, u32, ()>) -> Vec<LeafId> {
        controller
            .active_behaviours()
            .map(|(leaf, _)| leaf)
            .collect()
    }

    #[test]
    fn first_tick_does_not_gate_input_and_resets_every_timer_before_begin() {
        let inactive = state(
            b"Inactive",
            vec![attribute(b"active", b"false", 0)],
            vec![expression(true)],
        );
        let rejected = state(b"Rejected", Vec::new(), vec![expression(true)]);
        let selected_leaf = state(
            b"SelectedLeaf",
            Vec::new(),
            vec![expression(true), behaviour(true), behaviour(true)],
        );
        let selected = state(
            b"Selected",
            Vec::new(),
            vec![behaviour(true), selected_leaf],
        );
        let root = state(
            b"Root",
            vec![attribute(b"active", b"false", 0)],
            vec![behaviour(true), inactive, rejected, selected],
        );
        let graph = build(root);
        let root_id = StateId(0);
        let rejected_id = StateId(2);
        let selected_id = StateId(3);
        let selected_leaf_id = StateId(4);
        let rejected_condition = only_condition(&graph, rejected_id);
        let selected_leaf_condition = only_condition(&graph, selected_leaf_id);
        let root_behaviour = graph.states()[root_id.0].behaviours[0];
        let selected_behaviour = graph.states()[selected_id.0].behaviours[0];
        let leaf_behaviours = graph.states()[selected_leaf_id.0].behaviours.clone();

        let mut backend = Backend {
            watched_states: vec![root_id, selected_id, selected_leaf_id],
            false_conditions: [rejected_condition.0].into_iter().collect(),
            null_instances: [leaf_behaviours[1].0].into_iter().collect(),
            ..Backend::default()
        };
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        controller.update(&mut backend, 0.25).unwrap();

        assert_eq!(controller.current_state(), Some(selected_leaf_id));
        assert_eq!(controller.last_state(), None);
        assert_eq!(
            controller.time_in_state(root_id).to_bits(),
            0.0f32.to_bits()
        );
        assert_eq!(
            controller.time_in_state(selected_id).to_bits(),
            0.0f32.to_bits()
        );
        assert_eq!(
            controller.time_in_state(selected_leaf_id).to_bits(),
            0.0f32.to_bits()
        );
        assert_eq!(
            callback_kinds(&backend.events),
            vec![
                (CallbackKind::Condition, rejected_condition, None),
                (CallbackKind::Condition, selected_leaf_condition, None),
                (CallbackKind::Create, root_behaviour, Some(1)),
                (CallbackKind::Begin, root_behaviour, Some(1)),
                (CallbackKind::Create, selected_behaviour, Some(2)),
                (CallbackKind::Begin, selected_behaviour, Some(2)),
                (CallbackKind::Create, leaf_behaviours[0], Some(3)),
                (CallbackKind::Begin, leaf_behaviours[0], Some(3)),
                (CallbackKind::Create, leaf_behaviours[1], None),
                (CallbackKind::Begin, leaf_behaviours[1], None),
                (CallbackKind::Update, root_behaviour, Some(1)),
                (CallbackKind::Update, selected_behaviour, Some(2)),
                (CallbackKind::Update, leaf_behaviours[0], Some(3)),
                (CallbackKind::Update, leaf_behaviours[1], None),
            ]
        );
        assert!(
            backend
                .events
                .iter()
                .filter(|event| event.kind == CallbackKind::Begin)
                .all(|event| event.times == [0.0f32.to_bits(); 3])
        );

        // A supplied leaf root is also selected even when its own inherited
        // enabled field is false. Its inherited-disabled Behaviour is skipped.
        let disabled_root = build(state(
            b"DisabledRoot",
            vec![attribute(b"enabled", b"false", 0)],
            vec![behaviour(true)],
        ));
        let mut disabled_controller = DynamicStateGraphController::new(disabled_root, ()).unwrap();
        let mut disabled_backend = Backend::default();
        disabled_controller
            .update(&mut disabled_backend, 1.0)
            .unwrap();
        assert_eq!(disabled_controller.current_state(), Some(StateId(0)));
        assert!(disabled_backend.events.is_empty());
    }

    #[test]
    #[allow(clippy::too_many_lines)] // One oracle keeps the full callback timeline visible.
    fn sibling_transition_preserves_exact_lifecycle_order_times_and_instances() {
        let root_id = StateId(0);
        let a_id = StateId(1);
        let a_leaf_id = StateId(2);
        let b_id = StateId(3);
        let b_leaf_id = StateId(4);

        // Append the transition to ALeaf in the fixture itself by rebuilding.
        let a_leaf = state(
            b"ALeaf",
            Vec::new(),
            vec![
                expression(true),
                behaviour(true),
                behaviour(true),
                transition_element(b"B", b"low", 2),
            ],
        );
        let a = state(b"A", Vec::new(), vec![behaviour(true), a_leaf]);
        let b_leaf = state(
            b"BLeaf",
            Vec::new(),
            vec![expression(true), behaviour(true)],
        );
        let b = state(
            b"B",
            Vec::new(),
            vec![expression(true), behaviour(true), b_leaf],
        );
        let graph = build(state(b"Root", Vec::new(), vec![behaviour(true), a, b]));

        let root_behaviour = graph.states()[root_id.0].behaviours[0];
        let a_behaviour = graph.states()[a_id.0].behaviours[0];
        let a_leaf_behaviours = graph.states()[a_leaf_id.0].behaviours.clone();
        let transition = graph.states()[a_leaf_id.0].transitions[0];
        let hooks = graph.transitions()[transition.0].hooks.clone();
        let b_behaviour = graph.states()[b_id.0].behaviours[0];
        let b_leaf_behaviour = graph.states()[b_leaf_id.0].behaviours[0];
        let mut backend = Backend {
            watched_states: vec![root_id, a_id, a_leaf_id, b_id, b_leaf_id],
            null_instances: [a_leaf_behaviours[1].0].into_iter().collect(),
            ..Backend::default()
        };
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        controller.update(&mut backend, 0.25).unwrap();
        assert_eq!(controller.current_state(), Some(a_leaf_id));
        backend.events.clear();

        controller.update(&mut backend, 0.5).unwrap();
        assert_eq!(controller.last_state(), Some(a_leaf_id));
        assert_eq!(controller.current_state(), Some(b_leaf_id));
        assert_eq!(
            callback_kinds(&backend.events)
                .into_iter()
                .filter(|(kind, _, _)| *kind != CallbackKind::Condition)
                .collect::<Vec<_>>(),
            vec![
                (CallbackKind::End, a_leaf_behaviours[1], None),
                (CallbackKind::End, a_leaf_behaviours[0], Some(3)),
                (CallbackKind::End, a_behaviour, Some(2)),
                (CallbackKind::Hook, hooks[0], None),
                (CallbackKind::Hook, hooks[1], None),
                (CallbackKind::Create, b_behaviour, Some(4)),
                (CallbackKind::Begin, b_behaviour, Some(4)),
                (CallbackKind::Create, b_leaf_behaviour, Some(5)),
                (CallbackKind::Begin, b_leaf_behaviour, Some(5)),
                (CallbackKind::Update, root_behaviour, Some(1)),
                (CallbackKind::Update, b_behaviour, Some(4)),
                (CallbackKind::Update, b_leaf_behaviour, Some(5)),
            ]
        );
        let end = backend
            .events
            .iter()
            .find(|event| event.kind == CallbackKind::End)
            .unwrap();
        assert_eq!(end.current, Some(b_leaf_id));
        assert_eq!(end.last, Some(a_leaf_id));
        assert_eq!(
            end.times,
            vec![
                0.5f32.to_bits(),
                0.5f32.to_bits(),
                0.5f32.to_bits(),
                (-1.0f32).to_bits(),
                (-1.0f32).to_bits(),
            ]
        );
        let hook_event = backend
            .events
            .iter()
            .find(|event| event.kind == CallbackKind::Hook)
            .unwrap();
        assert_eq!(
            hook_event.times,
            vec![
                0.5f32.to_bits(),
                (-1.0f32).to_bits(),
                (-1.0f32).to_bits(),
                (-1.0f32).to_bits(),
                (-1.0f32).to_bits(),
            ]
        );
        for event in backend
            .events
            .iter()
            .filter(|event| event.kind == CallbackKind::Begin)
        {
            assert_eq!(
                event.times,
                vec![
                    0.5f32.to_bits(),
                    (-1.0f32).to_bits(),
                    (-1.0f32).to_bits(),
                    0.0f32.to_bits(),
                    0.0f32.to_bits(),
                ]
            );
        }
    }

    #[test]
    fn ancestor_urgent_beats_current_high_and_source_order_breaks_ties() {
        let a = state(
            b"A",
            Vec::new(),
            vec![expression(true), transition_element(b"B", b"high", 1)],
        );
        let b = state(b"B", Vec::new(), vec![expression(true)]);
        let c = state(b"C", Vec::new(), vec![expression(true)]);
        let d = state(b"D", Vec::new(), vec![expression(true)]);
        let graph = build(state(
            b"Root",
            Vec::new(),
            vec![
                a,
                b,
                c,
                d,
                transition_element(b"C", b"urgent", 1),
                transition_element(b"D", b"urgent", 1),
            ],
        ));
        let a_id = StateId(1);
        let c_id = StateId(3);
        let first_urgent = graph.states()[0].transitions[0];
        let first_hook = graph.transitions()[first_urgent.0].hooks[0];
        let mut backend = Backend::default();
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        controller.update(&mut backend, 0.0).unwrap();
        assert_eq!(controller.current_state(), Some(a_id));
        backend.events.clear();
        controller.update(&mut backend, 0.0).unwrap();
        assert_eq!(controller.current_state(), Some(c_id));
        assert_eq!(
            backend
                .events
                .iter()
                .filter(|event| event.kind == CallbackKind::Hook)
                .map(|event| event.leaf)
                .collect::<Vec<_>>(),
            [first_hook]
        );
    }

    #[test]
    fn no_transition_interrupt_modes_preserve_sustain_and_precond_routing() {
        for (interruptable, expected_calls) in [(Some(b"false".as_slice()), 3usize), (None, 2usize)]
        {
            let mut attributes = Vec::new();
            if let Some(value) = interruptable {
                attributes.push(attribute(b"interruptable", value, 0));
            }
            let a = state(b"A", attributes, vec![expression_mask(b"sustain")]);
            let b = state(b"B", Vec::new(), vec![expression_mask(b"precond")]);
            let graph = build(state(b"Root", Vec::new(), vec![a, b]));
            let a_id = StateId(1);
            let b_id = StateId(2);
            let a_condition = only_condition(&graph, a_id);
            let b_condition = only_condition(&graph, b_id);
            let mut backend = Backend::default();
            let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
            controller.update(&mut backend, 0.0).unwrap();
            assert_eq!(controller.current_state(), Some(a_id));
            assert!(backend.events.is_empty());

            backend.false_conditions.insert(a_condition.0);
            controller.update(&mut backend, 0.0).unwrap();
            assert_eq!(controller.current_state(), Some(b_id));
            let conditions = backend
                .events
                .iter()
                .filter(|event| event.kind == CallbackKind::Condition)
                .map(|event| event.leaf)
                .collect::<Vec<_>>();
            let expected = if expected_calls == 3 {
                vec![a_condition, a_condition, b_condition]
            } else {
                vec![a_condition, b_condition]
            };
            assert_eq!(conditions, expected);
        }

        // STATE_INTERRUPTABLE first checks target+parents under Sustain, then
        // evaluates the linked subtree under Sustain. A self target is on the
        // current path, so both operations invoke the Sustain condition.
        let a = state(
            b"A",
            vec![attribute(b"interruptable", b"A", 0)],
            vec![expression_mask(b"sustain")],
        );
        let b = state(b"B", Vec::new(), vec![expression_mask(b"precond")]);
        let graph = build(state(b"Root", Vec::new(), vec![a, b]));
        let a_condition = only_condition(&graph, StateId(1));
        let mut backend = Backend::default();
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        controller.update(&mut backend, 0.0).unwrap();
        assert_eq!(controller.current_state(), Some(StateId(1)));
        backend.events.clear();
        controller.update(&mut backend, 0.0).unwrap();
        assert_eq!(controller.current_state(), Some(StateId(1)));
        assert_eq!(
            backend
                .events
                .iter()
                .filter(|event| event.kind == CallbackKind::Condition)
                .map(|event| event.leaf)
                .collect::<Vec<_>>(),
            [a_condition, a_condition]
        );
    }

    #[test]
    fn self_transition_forces_reentry_and_end_all_is_reverse_without_timer_clear() {
        let graph = build(state(
            b"Root",
            Vec::new(),
            vec![
                expression(true),
                behaviour(true),
                behaviour(true),
                transition_element(b"Root", b"low", 1),
            ],
        ));
        let state_id = StateId(0);
        let behaviours = graph.states()[0].behaviours.clone();
        let hook = graph.transitions()[0].hooks[0];
        let mut backend = Backend::default();
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        controller.update(&mut backend, 0.25).unwrap();
        backend.events.clear();
        controller.update(&mut backend, 0.5).unwrap();
        assert_eq!(
            callback_kinds(&backend.events)
                .into_iter()
                .filter(|(kind, _, _)| *kind != CallbackKind::Condition)
                .collect::<Vec<_>>(),
            vec![
                (CallbackKind::End, behaviours[1], Some(2)),
                (CallbackKind::End, behaviours[0], Some(1)),
                (CallbackKind::Hook, hook, None),
                (CallbackKind::Create, behaviours[0], Some(3)),
                (CallbackKind::Begin, behaviours[0], Some(3)),
                (CallbackKind::Create, behaviours[1], Some(4)),
                (CallbackKind::Begin, behaviours[1], Some(4)),
                (CallbackKind::Update, behaviours[0], Some(3)),
                (CallbackKind::Update, behaviours[1], Some(4)),
            ]
        );
        assert_eq!(
            controller.time_in_state(state_id).to_bits(),
            0.0f32.to_bits()
        );
        backend.events.clear();
        controller.end_all_behaviours(&mut backend).unwrap();
        assert_eq!(
            callback_kinds(&backend.events),
            vec![
                (CallbackKind::End, behaviours[1], Some(4)),
                (CallbackKind::End, behaviours[0], Some(3)),
            ]
        );
        assert_eq!(
            controller.time_in_state(state_id).to_bits(),
            0.0f32.to_bits()
        );
    }

    #[test]
    fn owned_graph_and_context_preserve_native_mutability_boundaries() {
        let graph = build(state(
            b"Root",
            Vec::new(),
            vec![
                expression(true),
                behaviour(true),
                transition_element(b"Root", b"low", 1),
            ],
        ));
        let condition = only_condition(&graph, StateId(0));
        let behaviour = graph.states()[0].behaviours[0];
        let hook = graph.transitions()[0].hooks[0];
        let mut controller =
            DynamicStateGraphController::new(graph, BoundContext::default()).unwrap();
        let mut backend = ContextBackend::default();

        controller.update(&mut backend, 0.0).unwrap();
        assert_eq!(controller.context().value, 0);
        assert_eq!(controller.context().shared.get(), 300);
        assert_eq!(
            backend.observed,
            [("condition", 0, 0), ("begin", 0, 0), ("update", 0, 100)]
        );
        backend.observed.clear();
        controller.update(&mut backend, 0.0).unwrap();
        assert_eq!(controller.context().value, 1);
        assert_eq!(controller.context().shared.get(), 901);
        assert_eq!(
            backend.observed,
            [
                ("condition", 0, 300),
                ("condition", 0, 300),
                ("condition", 0, 300),
                ("end", 0, 300),
                ("hook", 0, 600),
                ("begin", 1, 601),
                ("update", 1, 701),
            ]
        );
        assert_eq!(
            controller.graph().unwrap().leaves()[condition.0]
                .payload()
                .unwrap()
                .calls,
            ["condition", "condition", "condition"]
        );
        assert_eq!(
            controller.graph().unwrap().leaves()[behaviour.0]
                .payload()
                .unwrap()
                .calls,
            [
                "create", "begin", "update", "end", "create", "begin", "update"
            ]
        );
        assert_eq!(
            controller.graph().unwrap().leaves()[hook.0]
                .payload()
                .unwrap()
                .calls,
            ["hook"]
        );
    }

    #[test]
    fn returned_error_keeps_backend_mutations_at_copied_and_owned_context_boundaries() {
        let graph = build(state(b"Root", Vec::new(), vec![behaviour(true)]));
        let state_id = StateId(0);
        let behaviour = graph.states()[0].behaviours[0];
        let context = BoundContext {
            value: 7,
            shared: Rc::new(Cell::new(11)),
        };
        let mut controller = DynamicStateGraphController::new(graph, context).unwrap();
        let mut backend = ContextBackend {
            failure: Some((CallbackKind::Begin, behaviour)),
            ..ContextBackend::default()
        };

        let error = controller.update(&mut backend, 0.25).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Lifecycle(
                DynamicStateGraphLifecycleError::Begin {
                    state,
                    leaf,
                    source: InjectedBackendError {
                        kind: CallbackKind::Begin,
                        ..
                    }
                }
            ) if state == state_id && leaf == behaviour
        ));
        assert_eq!(controller.context().value, 7);
        assert_eq!(controller.context().shared.get(), 111);
        assert_eq!(
            controller
                .active_behaviours()
                .next()
                .and_then(|(_, instance)| instance.copied()),
            Some(1_001)
        );
        assert_eq!(
            controller.graph().unwrap().leaves()[behaviour.0]
                .payload()
                .unwrap()
                .calls,
            ["create", "begin"]
        );

        let a = state(b"A", Vec::new(), vec![transition_element(b"B", b"low", 1)]);
        let b = state(b"B", Vec::new(), Vec::new());
        let graph = build(state(b"Root", Vec::new(), vec![a, b]));
        let transition = graph.states()[1].transitions[0];
        let hook = graph.transitions()[transition.0].hooks[0];
        let context = BoundContext {
            value: 7,
            shared: Rc::new(Cell::new(11)),
        };
        let mut controller = DynamicStateGraphController::new(graph, context).unwrap();
        let mut startup = ContextBackend::default();
        controller.update(&mut startup, 0.25).unwrap();
        let mut backend = ContextBackend {
            failure: Some((CallbackKind::Hook, hook)),
            ..ContextBackend::default()
        };

        let error = controller.update(&mut backend, 0.5).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Lifecycle(
                DynamicStateGraphLifecycleError::TransitionHook {
                    transition: failed_transition,
                    hook: failed_hook,
                    source: InjectedBackendError {
                        kind: CallbackKind::Hook,
                        ..
                    }
                }
            ) if failed_transition == transition && failed_hook == hook
        ));
        assert_eq!(controller.context().value, 8);
        assert_eq!(controller.context().shared.get(), 12);
        assert_eq!(
            controller.graph().unwrap().leaves()[hook.0]
                .payload()
                .unwrap()
                .calls,
            ["hook"]
        );
    }

    #[test]
    fn controller_requires_successful_structural_process() {
        let mut factory = Factory::default();
        let unprocessed = StateGraphRuntime::build(
            &state(b"Root", Vec::new(), Vec::new()),
            &mut factory,
            FactoryMissPolicy::Strict,
        )
        .unwrap();
        assert!(matches!(
            DynamicStateGraphController::<Payload, u32, ()>::new(unprocessed, ()),
            Err(DynamicStateGraphInvariantError::GraphNotProcessed)
        ));

        let mut failed = StateGraphRuntime::build(
            &state(
                b"Root",
                Vec::new(),
                vec![transition_element(b"Missing", b"low", 0)],
            ),
            &mut factory,
            FactoryMissPolicy::Strict,
        )
        .unwrap();
        assert!(failed.process_after_build(&mut factory).is_err());
        assert!(matches!(
            DynamicStateGraphController::<Payload, u32, ()>::new(failed, ()),
            Err(DynamicStateGraphInvariantError::GraphNotProcessed)
        ));
    }

    #[test]
    fn timer_capacity_counts_exit_and_reenter_union() {
        let mut old = named_chain("O", 14, vec![transition_element(b"Target", b"low", 0)]);
        old.children.insert(0, expression_mask(b"always"));
        let new = named_chain("N", 15, Vec::new());
        let graph = build(state(
            b"Root",
            Vec::new(),
            vec![state(b"Target", Vec::new(), vec![old, new])],
        ));
        let old_condition = only_condition(&graph, StateId(2));
        let mut backend = Backend::default();
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        controller.update(&mut backend, 0.0).unwrap();
        assert_eq!(controller.current_state(), Some(StateId(15)));
        backend.events.clear();
        backend.false_conditions.insert(old_condition.0);
        let error = controller.update(&mut backend, 0.0).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Invariant(
                DynamicStateGraphInvariantError::TimerCapacity {
                    required: 17,
                    maximum: 16
                }
            )
        ));
        assert_eq!(controller.current_state(), Some(StateId(15)));
        assert!(controller.is_poisoned());
        assert!(
            backend
                .events
                .iter()
                .all(|event| event.kind == CallbackKind::Condition)
        );
    }

    fn chain(depth: usize) -> StateGraphBinaryElement {
        let mut node = state(b"Leaf", Vec::new(), Vec::new());
        for index in (0..depth - 1).rev() {
            let name = format!("S{index}");
            node = state(name.as_bytes(), Vec::new(), vec![node]);
        }
        node
    }

    #[test]
    fn hierarchy_slice_accepts_sixteen_and_rejects_seventeen_before_callbacks() {
        let graph16 = build(chain(16));
        let mut controller16 = DynamicStateGraphController::new(graph16, ()).unwrap();
        let mut backend16 = Backend::default();
        controller16.update(&mut backend16, 0.0).unwrap();
        assert_eq!(controller16.current_state(), Some(StateId(15)));
        assert_eq!(
            controller16.time_in_state(StateId(15)).to_bits(),
            0.0f32.to_bits()
        );
        assert!(backend16.events.is_empty());

        let graph17 = build(chain(17));
        let mut controller17 = DynamicStateGraphController::new(graph17, ()).unwrap();
        let mut backend17 = Backend::default();
        let error = controller17.update(&mut backend17, 0.0).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Invariant(
                DynamicStateGraphInvariantError::HierarchyCapacity {
                    required: 17,
                    maximum: 16
                }
            )
        ));
        assert!(controller17.is_poisoned());
        assert!(backend17.events.is_empty());
        assert!(matches!(
            controller17.update(&mut backend17, 0.0),
            Err(DynamicStateGraphControllerError::Invariant(
                DynamicStateGraphInvariantError::Poisoned
            ))
        ));
    }

    #[test]
    #[allow(clippy::too_many_lines)] // One table keeps all three adjacent phase prefixes comparable.
    fn create_begin_and_update_failures_preserve_their_exact_prefix_and_poison() {
        for phase in [
            CallbackKind::Create,
            CallbackKind::Begin,
            CallbackKind::Update,
        ] {
            let graph = build(state(
                b"Root",
                Vec::new(),
                vec![behaviour(true), behaviour(true), behaviour(true)],
            ));
            let state_id = StateId(0);
            let behaviours = graph.states()[0].behaviours.clone();
            let failing = behaviours[1];
            let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
            let mut backend = FailingBackend {
                inner: Backend::default(),
                failure: (phase, failing),
            };

            let error = controller.update(&mut backend, 0.25).unwrap_err();
            let lifecycle_source = error.source().expect("controller lifecycle source");
            assert!(
                lifecycle_source
                    .source()
                    .and_then(|source| source.downcast_ref::<InjectedBackendError>())
                    .is_some()
            );
            match (phase, error) {
                (
                    CallbackKind::Create,
                    DynamicStateGraphControllerError::Lifecycle(
                        DynamicStateGraphLifecycleError::CreateInstance {
                            state,
                            leaf,
                            source,
                        },
                    ),
                )
                | (
                    CallbackKind::Begin,
                    DynamicStateGraphControllerError::Lifecycle(
                        DynamicStateGraphLifecycleError::Begin {
                            state,
                            leaf,
                            source,
                        },
                    ),
                )
                | (
                    CallbackKind::Update,
                    DynamicStateGraphControllerError::Lifecycle(
                        DynamicStateGraphLifecycleError::Update {
                            state,
                            leaf,
                            source,
                        },
                    ),
                ) => {
                    assert_eq!((state, leaf), (state_id, failing));
                    assert_eq!(
                        source,
                        InjectedBackendError {
                            kind: phase,
                            leaf: failing
                        }
                    );
                }
                (_, unexpected) => panic!("unexpected lifecycle error: {unexpected:?}"),
            }

            let expected_active = if phase == CallbackKind::Create {
                vec![behaviours[0]]
            } else if phase == CallbackKind::Begin {
                behaviours[..2].to_vec()
            } else {
                behaviours.clone()
            };
            assert_eq!(active_leaf_ids(&controller), expected_active);
            assert_eq!(controller.current_state(), Some(state_id));
            assert_eq!(controller.last_state(), None);
            assert_eq!(
                controller.time_in_state(state_id).to_bits(),
                0.0f32.to_bits()
            );
            assert!(controller.is_poisoned());

            let expected_events = match phase {
                CallbackKind::Create => vec![
                    (CallbackKind::Create, behaviours[0], Some(1)),
                    (CallbackKind::Begin, behaviours[0], Some(1)),
                    (CallbackKind::Create, failing, Some(2)),
                ],
                CallbackKind::Begin => vec![
                    (CallbackKind::Create, behaviours[0], Some(1)),
                    (CallbackKind::Begin, behaviours[0], Some(1)),
                    (CallbackKind::Create, failing, Some(2)),
                    (CallbackKind::Begin, failing, Some(2)),
                ],
                CallbackKind::Update => vec![
                    (CallbackKind::Create, behaviours[0], Some(1)),
                    (CallbackKind::Begin, behaviours[0], Some(1)),
                    (CallbackKind::Create, failing, Some(2)),
                    (CallbackKind::Begin, failing, Some(2)),
                    (CallbackKind::Create, behaviours[2], Some(3)),
                    (CallbackKind::Begin, behaviours[2], Some(3)),
                    (CallbackKind::Update, behaviours[0], Some(1)),
                    (CallbackKind::Update, failing, Some(2)),
                ],
                _ => unreachable!(),
            };
            assert_eq!(callback_kinds(&backend.inner.events), expected_events);

            let event_count = backend.inner.events.len();
            assert!(matches!(
                controller.update(&mut backend, 1.0),
                Err(DynamicStateGraphControllerError::Invariant(
                    DynamicStateGraphInvariantError::Poisoned
                ))
            ));
            assert_eq!(backend.inner.events.len(), event_count);
            assert_eq!(controller.delta_time().to_bits(), 0.25f32.to_bits());
        }
    }

    #[test]
    fn exit_end_failure_keeps_failing_entry_and_all_exit_timers() {
        let a = state(
            b"A",
            Vec::new(),
            vec![
                behaviour(true),
                behaviour(true),
                behaviour(true),
                transition_element(b"B", b"low", 1),
            ],
        );
        let b = state(b"B", Vec::new(), vec![behaviour(true)]);
        let graph = build(state(b"Root", Vec::new(), vec![a, b]));
        let root_id = StateId(0);
        let a_id = StateId(1);
        let b_id = StateId(2);
        let a_behaviours = graph.states()[a_id.0].behaviours.clone();
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        let mut startup = Backend::default();
        controller.update(&mut startup, 0.25).unwrap();

        let mut backend = FailingBackend {
            inner: Backend::default(),
            failure: (CallbackKind::End, a_behaviours[1]),
        };
        let error = controller.update(&mut backend, 0.5).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Lifecycle(
                DynamicStateGraphLifecycleError::ExitEnd {
                    state,
                    leaf,
                    source: InjectedBackendError {
                        kind: CallbackKind::End,
                        ..
                    }
                }
            ) if state == a_id && leaf == a_behaviours[1]
        ));
        assert_eq!(active_leaf_ids(&controller), a_behaviours[..2]);
        assert_eq!(controller.current_state(), Some(b_id));
        assert_eq!(controller.last_state(), Some(a_id));
        assert_eq!(
            controller.time_in_state(root_id).to_bits(),
            0.5f32.to_bits()
        );
        assert_eq!(controller.time_in_state(a_id).to_bits(), 0.5f32.to_bits());
        assert_eq!(
            controller.time_in_state(b_id).to_bits(),
            (-1.0f32).to_bits()
        );
        assert_eq!(
            callback_kinds(&backend.inner.events)
                .into_iter()
                .filter(|(kind, _, _)| *kind != CallbackKind::Condition)
                .collect::<Vec<_>>(),
            vec![
                (CallbackKind::End, a_behaviours[2], Some(3)),
                (CallbackKind::End, a_behaviours[1], Some(2)),
            ]
        );
        assert!(controller.is_poisoned());
        let event_count = backend.inner.events.len();
        assert!(matches!(
            controller.update(&mut backend, 1.0),
            Err(DynamicStateGraphControllerError::Invariant(
                DynamicStateGraphInvariantError::Poisoned
            ))
        ));
        assert_eq!(backend.inner.events.len(), event_count);
        assert_eq!(controller.delta_time().to_bits(), 0.5f32.to_bits());
    }

    #[test]
    fn transition_hook_failure_stops_before_enter_timer_and_behaviour_work() {
        let a = state(
            b"A",
            Vec::new(),
            vec![behaviour(true), transition_element(b"B", b"low", 3)],
        );
        let b = state(b"B", Vec::new(), vec![behaviour(true)]);
        let graph = build(state(b"Root", Vec::new(), vec![a, b]));
        let a_id = StateId(1);
        let b_id = StateId(2);
        let a_behaviour = graph.states()[a_id.0].behaviours[0];
        let transition = graph.states()[a_id.0].transitions[0];
        let hooks = graph.transitions()[transition.0].hooks.clone();
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        controller.update(&mut Backend::default(), 0.25).unwrap();

        let mut backend = FailingBackend {
            inner: Backend::default(),
            failure: (CallbackKind::Hook, hooks[1]),
        };
        let error = controller.update(&mut backend, 0.5).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Lifecycle(
                DynamicStateGraphLifecycleError::TransitionHook {
                    transition: failed_transition,
                    hook,
                    source: InjectedBackendError {
                        kind: CallbackKind::Hook,
                        ..
                    }
                }
            ) if failed_transition == transition && hook == hooks[1]
        ));
        assert!(active_leaf_ids(&controller).is_empty());
        assert_eq!(controller.current_state(), Some(b_id));
        assert_eq!(controller.last_state(), Some(a_id));
        assert_eq!(
            controller.time_in_state(a_id).to_bits(),
            (-1.0f32).to_bits()
        );
        assert_eq!(
            controller.time_in_state(b_id).to_bits(),
            (-1.0f32).to_bits()
        );
        assert_eq!(
            callback_kinds(&backend.inner.events)
                .into_iter()
                .filter(|(kind, _, _)| *kind != CallbackKind::Condition)
                .collect::<Vec<_>>(),
            vec![
                (CallbackKind::End, a_behaviour, Some(1)),
                (CallbackKind::Hook, hooks[0], None),
                (CallbackKind::Hook, hooks[1], None),
            ]
        );
        assert!(controller.is_poisoned());
        let event_count = backend.inner.events.len();
        assert!(matches!(
            controller.update(&mut backend, 1.0),
            Err(DynamicStateGraphControllerError::Invariant(
                DynamicStateGraphInvariantError::Poisoned
            ))
        ));
        assert_eq!(backend.inner.events.len(), event_count);
        assert_eq!(controller.delta_time().to_bits(), 0.5f32.to_bits());
    }

    #[test]
    fn end_all_failure_removes_only_successful_reverse_suffix_and_poison_rejects_retry() {
        let graph = build(state(
            b"Root",
            Vec::new(),
            vec![behaviour(true), behaviour(true), behaviour(true)],
        ));
        let state_id = StateId(0);
        let behaviours = graph.states()[0].behaviours.clone();
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        controller.update(&mut Backend::default(), 0.25).unwrap();
        let before = (
            controller.current_state(),
            controller.last_state(),
            controller.delta_time().to_bits(),
        );
        let mut backend = FailingBackend {
            inner: Backend::default(),
            failure: (CallbackKind::End, behaviours[1]),
        };

        let error = controller.end_all_behaviours(&mut backend).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Lifecycle(
                DynamicStateGraphLifecycleError::EndAllEnd {
                    state,
                    leaf,
                    source: InjectedBackendError {
                        kind: CallbackKind::End,
                        ..
                    }
                }
            ) if state == state_id && leaf == behaviours[1]
        ));
        assert_eq!(active_leaf_ids(&controller), behaviours[..2]);
        assert_eq!(
            (
                controller.current_state(),
                controller.last_state(),
                controller.delta_time().to_bits(),
            ),
            before
        );
        assert_eq!(
            callback_kinds(&backend.inner.events),
            vec![
                (CallbackKind::End, behaviours[2], Some(3)),
                (CallbackKind::End, behaviours[1], Some(2)),
            ]
        );
        assert_eq!(
            controller.time_in_state(state_id).to_bits(),
            0.0f32.to_bits()
        );
        assert!(controller.is_poisoned());

        let event_count = backend.inner.events.len();
        assert!(matches!(
            controller.end_all_behaviours(&mut backend),
            Err(DynamicStateGraphControllerError::Invariant(
                DynamicStateGraphInvariantError::Poisoned
            ))
        ));
        assert_eq!(backend.inner.events.len(), event_count);
        assert_eq!(active_leaf_ids(&controller), behaviours[..2]);
        assert_eq!(
            (
                controller.current_state(),
                controller.last_state(),
                controller.delta_time().to_bits(),
            ),
            before
        );
    }

    #[test]
    fn cache_and_controllers_retain_same_graph_with_separate_contexts() {
        let graph = Arc::new(Mutex::new(build(state(
            b"Root",
            Vec::new(),
            vec![expression(true), behaviour(true)],
        ))));
        let mut cache = crate::graph_asset_cache::StateGraphCache::new();
        let cached = Arc::clone(
            cache
                .load_with("script/camera/Test.xml", false, |_| {
                    Ok::<_, std::convert::Infallible>(Some(Arc::clone(&graph)))
                })
                .unwrap()
                .unwrap(),
        );
        let mut first =
            DynamicStateGraphController::new_shared(Arc::clone(&cached), BoundContext::default())
                .unwrap();
        let second: DynamicStateGraphController<Payload, u32, BoundContext> =
            DynamicStateGraphController::new_shared(Arc::clone(&cached), BoundContext::default())
                .unwrap();
        assert!(Arc::ptr_eq(&graph, first.shared_graph()));
        assert!(Arc::ptr_eq(first.shared_graph(), second.shared_graph()));
        let mut backend = ContextBackend::default();
        first.update(&mut backend, 0.25).unwrap();
        assert_eq!(second.current_state(), None);
        assert_eq!(second.context().shared.get(), 0);
        assert_eq!(first.context().shared.get(), 300);
        let retained = second.graph().unwrap();
        let leaf = retained.states()[0].behaviours[0];
        assert_eq!(
            retained.leaves()[leaf.0].payload().unwrap().calls,
            ["create", "begin", "update"]
        );
        drop(retained);
        drop(first);
        cache.flush();
        assert!(Arc::ptr_eq(&graph, second.shared_graph()));
        assert_eq!(Arc::strong_count(&graph), 3);
    }

    #[test]
    fn held_graph_returns_typed_busy_without_recursive_mutex_wait() {
        let graph = Arc::new(Mutex::new(build(state(b"Root", Vec::new(), Vec::new()))));
        let mut controller: DynamicStateGraphController<Payload, u32, ()> =
            DynamicStateGraphController::new_shared(Arc::clone(&graph), ()).unwrap();
        let _borrow = graph.lock().unwrap();
        assert!(matches!(
            controller.graph(),
            Err(DynamicStateGraphInvariantError::GraphBusy)
        ));
        assert!(matches!(
            controller.update(&mut Backend::default(), 0.5),
            Err(DynamicStateGraphControllerError::Invariant(
                DynamicStateGraphInvariantError::GraphBusy
            ))
        ));
        assert!(controller.is_poisoned());
        assert_eq!(controller.delta_time().to_bits(), 0.5_f32.to_bits());
        assert_eq!(controller.current_state(), None);
    }
}
