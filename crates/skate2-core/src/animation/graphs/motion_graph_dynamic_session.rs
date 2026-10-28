//! Owning session for the evidence-bounded dynamic `MotionGraph` fallback.
//!
//! Construction requires complete full-arena coverage by the four recovered
//! typed leaves before creating the generic controller. Each session operation
//! then creates an ephemeral [`BorrowedMotionGraphBackend`] over the same owned
//! facade and allocator values. Runtime callbacks still revalidate their exact
//! identity; preflight is an implementation-coverage prerequisite, not proof
//! that callbacks will succeed.
//!
//! The session does not reconstruct stable Actor identity, UCOM/entity lookup,
//! the native six-slot controller `Context`, opaque leaves, transition hooks,
//! generated-controller dispatch, or mounted execution. Moving the session can
//! move its fields, and the facade may internally retarget itself. No `Drop`
//! cleanup is performed; callers must explicitly call [`Self::end_all_behaviours`].

use std::{error::Error, fmt};

use crate::{
    dynamic_state_graph_controller::{
        DynamicStateGraphController, DynamicStateGraphControllerError,
        DynamicStateGraphInvariantError,
    },
    motion_graph_capability_preflight::{
        MotionGraphCapabilityPreflightError, MotionGraphCapabilityPreflightReport,
        preflight_motion_graph_capabilities,
    },
    motion_graph_conditions::{HasIntentCollaborator, WillExpireCollaborator},
    motion_graph_dynamic_backend::{BorrowedMotionGraphBackend, MotionGraphDynamicBackendError},
    motion_graph_leaf_factory::MotionGraphLeafPayload,
    play_animation_lifecycle::{
        PlayAnimationInstanceAllocator, PlayAnimationLifecycleBackend, PlayAnimationRuntimeInstance,
    },
    play_remapped_animation::PlayRemappedAnimationCollaborator,
    state_graph_runtime::{LeafId, StateGraphRuntime, StateId},
};

pub type MotionGraphSessionController<O> =
    DynamicStateGraphController<MotionGraphLeafPayload<O>, PlayAnimationRuntimeInstance, ()>;

pub type MotionGraphSessionBackendError<E, A> = MotionGraphDynamicBackendError<
    <E as HasIntentCollaborator>::Error,
    <E as WillExpireCollaborator>::Error,
    <A as PlayAnimationInstanceAllocator>::Error,
    <E as PlayAnimationLifecycleBackend>::Error,
    <E as PlayRemappedAnimationCollaborator>::Error,
>;

pub type MotionGraphSessionTickError<E, A> =
    DynamicStateGraphControllerError<MotionGraphSessionBackendError<E, A>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionGraphSessionCreateError {
    Preflight(MotionGraphCapabilityPreflightError),
    UnsupportedCapabilities(MotionGraphCapabilityPreflightReport),
    Controller(DynamicStateGraphInvariantError),
}

impl fmt::Display for MotionGraphSessionCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preflight(error) => write!(formatter, "MotionGraph preflight failed: {error}"),
            Self::UnsupportedCapabilities(report) => write!(
                formatter,
                "MotionGraph has {} unsupported leaves among {} arena leaves",
                report.diagnostics.len(),
                report.leaf_count
            ),
            Self::Controller(error) => {
                write!(
                    formatter,
                    "MotionGraph controller construction failed: {error}"
                )
            }
        }
    }
}

impl Error for MotionGraphSessionCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Preflight(error) => Some(error),
            Self::Controller(error) => Some(error),
            Self::UnsupportedCapabilities(_) => None,
        }
    }
}

/// Owns one controller, collaborator facade, and allocator for a session.
///
/// The fields are private and no mutable component accessors are exposed, so
/// callers cannot replace the facade, allocator, or callback backend between
/// this type's `update` and `end_all_behaviours` calls. This is Rust ownership
/// stability only; it is not evidence of native entity identity.
pub struct OwningMotionGraphSession<O, E, A> {
    controller: MotionGraphSessionController<O>,
    facade: E,
    allocator: A,
}

impl<O, E, A> OwningMotionGraphSession<O, E, A> {
    /// Runs immutable full-arena capability preflight before constructing the
    /// generic controller or invoking any collaborator.
    ///
    /// `facade` and `allocator` are consumed and therefore dropped on failure;
    /// this function guarantees that it invokes no methods on either value,
    /// not that arbitrary user-defined destructors are effect-free.
    ///
    /// # Errors
    ///
    /// Returns the exact preflight error, the complete ordered unsupported
    /// report, or the subsequent generic-controller invariant error.
    pub fn try_new(
        graph: StateGraphRuntime<MotionGraphLeafPayload<O>>,
        facade: E,
        allocator: A,
    ) -> Result<Self, MotionGraphSessionCreateError>
    where
        E: HasIntentCollaborator + WillExpireCollaborator + PlayAnimationLifecycleBackend,
        A: PlayAnimationInstanceAllocator,
    {
        let report = preflight_motion_graph_capabilities(&graph)
            .map_err(MotionGraphSessionCreateError::Preflight)?;
        if !report.has_full_leaf_coverage() {
            return Err(MotionGraphSessionCreateError::UnsupportedCapabilities(
                report,
            ));
        }
        let controller = DynamicStateGraphController::new(graph, ())
            .map_err(MotionGraphSessionCreateError::Controller)?;
        Ok(Self {
            controller,
            facade,
            allocator,
        })
    }

    #[must_use]
    pub const fn delta_time(&self) -> f32 {
        self.controller.delta_time()
    }

    #[must_use]
    pub const fn current_state(&self) -> Option<StateId> {
        self.controller.current_state()
    }

    #[must_use]
    pub const fn last_state(&self) -> Option<StateId> {
        self.controller.last_state()
    }

    #[must_use]
    pub fn time_in_state(&self, state: StateId) -> f32 {
        self.controller.time_in_state(state)
    }

    #[must_use]
    pub fn time_in_current_state(&self) -> f32 {
        self.controller.time_in_current_state()
    }

    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        self.controller.is_poisoned()
    }

    /// Borrows the retained graph; recursive access and poison remain typed.
    ///
    /// # Errors
    /// Returns the shared controller graph access failure.
    pub fn graph(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, StateGraphRuntime<MotionGraphLeafPayload<O>>>,
        crate::dynamic_state_graph_controller::DynamicStateGraphInvariantError,
    > {
        self.controller.graph()
    }

    #[must_use]
    pub fn active_behaviours(
        &self,
    ) -> impl ExactSizeIterator<Item = (LeafId, Option<&PlayAnimationRuntimeInstance>)> {
        self.controller.active_behaviours()
    }
}

impl<O, E, A> OwningMotionGraphSession<O, E, A>
where
    E: HasIntentCollaborator
        + WillExpireCollaborator
        + PlayAnimationLifecycleBackend
        + PlayRemappedAnimationCollaborator,
    A: PlayAnimationInstanceAllocator,
{
    /// Runs one generic-controller tick with an internal ephemeral backend.
    ///
    /// # Errors
    ///
    /// Preserves the controller's exact invariant, activation, lifecycle, and
    /// poison errors with the typed callback source.
    pub fn update(&mut self, delta_time: f32) -> Result<(), MotionGraphSessionTickError<E, A>> {
        let Self {
            controller,
            facade,
            allocator,
        } = self;
        let mut backend = BorrowedMotionGraphBackend::new(facade, allocator);
        controller.update(&mut backend, delta_time)
    }

    /// Ends all active Behaviours through an internal ephemeral backend.
    ///
    /// # Errors
    ///
    /// Preserves reverse-order removal, retained failing-prefix, and poison
    /// behavior from the generic controller.
    pub fn end_all_behaviours(&mut self) -> Result<(), MotionGraphSessionTickError<E, A>> {
        let Self {
            controller,
            facade,
            allocator,
        } = self;
        let mut backend = BorrowedMotionGraphBackend::new(facade, allocator);
        controller.end_all_behaviours(&mut backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dynamic_state_graph_controller::{
            DynamicStateGraphControllerError, DynamicStateGraphInvariantError,
            DynamicStateGraphLifecycleError,
        },
        motion_graph_conditions::{
            AnimProperty, HasIntentActivationError, HasIntentCondition, WillExpireCondition,
        },
        motion_graph_dynamic_backend::MotionGraphDynamicBackendError,
        motion_graph_leaf_factory::{
            HAS_INTENT_REGISTERED_NAME, PLAY_ANIMATION_REGISTERED_NAME, WILL_EXPIRE_REGISTERED_NAME,
        },
        play_animation::{FastString30, FastString36, PlayAnimationBehaviour},
        play_animation_lifecycle::{
            AnimationDispatch, PhysicalOutputObservation, PlayAnimationInterfaceKind,
            PlayAnimationInterfaceResolution, PlayAnimationRuntimeError,
            RawAnimationTransitionParams, TreeAttributeSample,
        },
        state_graph_runtime::{
            FactoryMissPolicy, LeafCreateRequest, LeafKind, StateGraphLeafFactory,
        },
    };
    use skate2_formats::state_graph::{
        StateGraphBinaryAttribute, StateGraphBinaryElement, state_graph_string_hash32,
    };
    use std::{cell::RefCell, convert::Infallible, fmt, rc::Rc};

    const OTHER_CONDITION: &[u8] = b"OtherCondition";

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct SpyError(&'static str);

    impl fmt::Display for SpyError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl Error for SpyError {}

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        Facade(u32, &'static str),
        Allocate(u32),
    }

    struct Facade {
        token: u32,
        events: Rc<RefCell<Vec<Event>>>,
    }

    impl Facade {
        fn record(&self, operation: &'static str) {
            self.events
                .borrow_mut()
                .push(Event::Facade(self.token, operation));
        }
    }

    impl HasIntentCollaborator for Facade {
        type Error = SpyError;
        type MotionGraph = u8;
        type Intents = u8;

        fn cached_motion_graph(&mut self) -> Result<Option<u8>, SpyError> {
            self.record("has.cached");
            Ok(Some(1))
        }

        fn find_motion_graph(&mut self) -> Result<Option<u8>, SpyError> {
            self.record("has.find");
            Ok(Some(1))
        }

        fn motion_graph_intents(&mut self, _: &u8) -> Result<u8, SpyError> {
            self.record("has.intents");
            Ok(2)
        }

        fn lookup_intent(&mut self, _: &u8, _: &[u8]) -> Result<Option<f32>, SpyError> {
            self.record("has.lookup");
            Ok(Some(1.0))
        }
    }

    impl WillExpireCollaborator for Facade {
        type Error = SpyError;
        type AnimInfo = u8;
        type Animatable = u8;

        fn find_anim_info(&mut self) -> Result<Option<u8>, SpyError> {
            self.record("will.anim_info");
            Ok(None)
        }

        fn resolve_special_tag(&mut self, _: &u8, _: &[u8]) -> Result<f32, SpyError> {
            self.record("will.tag");
            Ok(0.0)
        }

        fn cached_animatable(&mut self) -> Result<Option<u8>, SpyError> {
            self.record("will.cached");
            Ok(Some(1))
        }

        fn find_animatable(&mut self) -> Result<Option<u8>, SpyError> {
            self.record("will.find");
            Ok(Some(1))
        }

        fn transitioning(&mut self, _: &u8) -> Result<bool, SpyError> {
            self.record("will.transitioning");
            Ok(false)
        }

        fn anim_property(&mut self, _: &u8) -> Result<Option<AnimProperty>, SpyError> {
            self.record("will.property");
            Ok(Some(AnimProperty {
                is_playing_raw: 1,
                has_expired_or_looped_raw: 1,
                has_expired_or_looped_by_time: 0.0,
                time_to_expire_or_loop: 0.0,
            }))
        }
    }

    impl PlayAnimationLifecycleBackend for Facade {
        type Error = SpyError;

        fn resolve_interface(
            &mut self,
            _: PlayAnimationInterfaceKind,
        ) -> Result<PlayAnimationInterfaceResolution, SpyError> {
            self.record("play.resolve");
            Ok(PlayAnimationInterfaceResolution::Resolved)
        }

        fn motion_intent(&mut self, _: FastString36) -> Result<Option<u32>, SpyError> {
            self.record("play.intent");
            Ok(None)
        }

        fn filtered_motion_intent(&mut self, _: FastString36) -> Result<Option<u32>, SpyError> {
            self.record("play.filtered");
            Ok(None)
        }

        fn tree_attribute(
            &mut self,
            _: FastString30,
        ) -> Result<Option<TreeAttributeSample>, SpyError> {
            self.record("play.tree");
            Ok(None)
        }

        fn add_settable_attribute(
            &mut self,
            _: FastString30,
            _: u32,
            _: bool,
            _: i32,
        ) -> Result<(), SpyError> {
            self.record("play.add");
            Ok(())
        }

        fn selected_pro_skater(&mut self) -> Result<FastString30, SpyError> {
            self.record("play.pro");
            Ok(FastString30 { words: [0; 5] })
        }

        fn set_construction_attribute(
            &mut self,
            _: FastString30,
            _: FastString30,
        ) -> Result<(), SpyError> {
            self.record("play.construction");
            Ok(())
        }

        fn next_transition_override(
            &mut self,
        ) -> Result<Option<RawAnimationTransitionParams>, SpyError> {
            self.record("play.override");
            Ok(Some(RawAnimationTransitionParams {
                transition_type_raw: 0,
                blend_length_bits: 0,
                placement_raw: 0,
                flags_raw: 0,
            }))
        }

        fn clear_transition_override(&mut self) -> Result<(), SpyError> {
            self.record("play.clear");
            Ok(())
        }

        fn is_switch(&mut self) -> Result<bool, SpyError> {
            self.record("play.switch");
            Ok(false)
        }

        fn is_mirrored(&mut self) -> Result<bool, SpyError> {
            self.record("play.mirror");
            Ok(false)
        }

        fn physical_output(&mut self) -> Result<PhysicalOutputObservation, SpyError> {
            self.record("play.physical");
            Ok(PhysicalOutputObservation::HoldingSkateboardRaw(1))
        }

        fn dispatch(&mut self, _: AnimationDispatch) -> Result<u8, SpyError> {
            self.record("play.dispatch");
            Ok(1)
        }
    }

    impl PlayRemappedAnimationCollaborator for Facade {
        type Error = SpyError;

        fn write_mirror_override_raw(&mut self, _: u8) -> Result<(), SpyError> {
            self.record("remapped.mirror");
            Ok(())
        }

        fn setup_remapped_animation(
            &mut self,
            _: crate::play_remapped_animation::RemappedAnimationSetup<'_>,
        ) -> Result<(), SpyError> {
            self.record("remapped.setup");
            Ok(())
        }

        fn update_remapped_animation(
            &mut self,
            _: crate::play_remapped_animation::RemappedAnimationUpdate<'_>,
        ) -> Result<(), SpyError> {
            self.record("remapped.update");
            Ok(())
        }
    }

    struct Allocator {
        token: u32,
        allocated: bool,
        events: Rc<RefCell<Vec<Event>>>,
    }

    impl PlayAnimationInstanceAllocator for Allocator {
        type Error = SpyError;

        fn allocate(&mut self, _: usize, _: &[u8], _: usize) -> Result<bool, SpyError> {
            self.events.borrow_mut().push(Event::Allocate(self.token));
            Ok(self.allocated)
        }
    }

    struct Factory {
        conditions: Vec<Vec<u8>>,
        behaviours: Vec<Vec<u8>>,
        hooks: Vec<Vec<u8>>,
    }

    impl Default for Factory {
        fn default() -> Self {
            Self {
                conditions: vec![
                    HAS_INTENT_REGISTERED_NAME.to_vec(),
                    WILL_EXPIRE_REGISTERED_NAME.to_vec(),
                    OTHER_CONDITION.to_vec(),
                ],
                behaviours: vec![PLAY_ANIMATION_REGISTERED_NAME.to_vec()],
                hooks: Vec::new(),
            }
        }
    }

    impl StateGraphLeafFactory for Factory {
        type Payload = MotionGraphLeafPayload<()>;
        type Error = Infallible;

        fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
            match kind {
                LeafKind::Condition => &self.conditions,
                LeafKind::Behaviour => &self.behaviours,
                LeafKind::Hook => &self.hooks,
            }
        }

        fn create(&mut self, request: LeafCreateRequest<'_>) -> Result<Self::Payload, Infallible> {
            Ok(match request.registered_name {
                HAS_INTENT_REGISTERED_NAME => MotionGraphLeafPayload::HasIntent(
                    HasIntentCondition::from_attributes(request.attributes).unwrap(),
                ),
                WILL_EXPIRE_REGISTERED_NAME => MotionGraphLeafPayload::WillExpire(
                    WillExpireCondition::from_attributes(request.attributes).unwrap(),
                ),
                PLAY_ANIMATION_REGISTERED_NAME => MotionGraphLeafPayload::PlayAnimation(
                    PlayAnimationBehaviour::from_attributes(request.attributes).unwrap(),
                ),
                OTHER_CONDITION => MotionGraphLeafPayload::RegisteredOpaque(()),
                _ => unreachable!(),
            })
        }
    }

    fn attr(key: &[u8], text: &[u8]) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits: 0,
            boolean_raw: 0,
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

    fn processed_graph(
        condition: Option<&[u8]>,
        behaviour: bool,
    ) -> StateGraphRuntime<MotionGraphLeafPayload<()>> {
        let mut children = Vec::new();
        if let Some(name) = condition {
            let mut attributes = vec![attr(b"name", name)];
            if name == HAS_INTENT_REGISTERED_NAME {
                attributes.push(attr(b"intent", b"Push"));
            }
            children.push(element(
                b"expression",
                Vec::new(),
                vec![element(b"condition", attributes, Vec::new())],
            ));
        }
        if behaviour {
            children.push(element(
                b"behaviour",
                vec![
                    attr(b"name", PLAY_ANIMATION_REGISTERED_NAME),
                    attr(b"anim", b"Idle"),
                ],
                Vec::new(),
            ));
        }
        let root = element(b"state", vec![attr(b"name", b"Root")], children);
        let mut factory = Factory::default();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        graph.process_after_build(&mut factory).unwrap();
        graph
    }

    fn processed_graph_with_unsupported_tail() -> StateGraphRuntime<MotionGraphLeafPayload<()>> {
        let root = element(
            b"state",
            vec![attr(b"name", b"Root")],
            vec![
                element(
                    b"expression",
                    Vec::new(),
                    vec![
                        element(
                            b"condition",
                            vec![
                                attr(b"name", HAS_INTENT_REGISTERED_NAME),
                                attr(b"intent", b"Push"),
                            ],
                            Vec::new(),
                        ),
                        element(
                            b"condition",
                            vec![attr(b"name", OTHER_CONDITION)],
                            Vec::new(),
                        ),
                        element(
                            b"condition",
                            vec![attr(b"name", OTHER_CONDITION)],
                            Vec::new(),
                        ),
                    ],
                ),
                element(
                    b"behaviour",
                    vec![
                        attr(b"name", PLAY_ANIMATION_REGISTERED_NAME),
                        attr(b"anim", b"Idle"),
                    ],
                    Vec::new(),
                ),
            ],
        );
        let mut factory = Factory::default();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        graph.process_after_build(&mut factory).unwrap();
        graph
    }

    fn collaborators(events: &Rc<RefCell<Vec<Event>>>, allocated: bool) -> (Facade, Allocator) {
        (
            Facade {
                token: 0xface,
                events: Rc::clone(events),
            },
            Allocator {
                token: 0xa110,
                allocated,
                events: Rc::clone(events),
            },
        )
    }

    #[test]
    fn unsupported_full_arena_is_reported_before_any_collaborator_effect() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let (facade, allocator) = collaborators(&events, true);
        let result = OwningMotionGraphSession::try_new(
            processed_graph_with_unsupported_tail(),
            facade,
            allocator,
        );
        let Err(MotionGraphSessionCreateError::UnsupportedCapabilities(report)) = result else {
            panic!("expected unsupported capability report")
        };
        assert_eq!(report.leaf_count, 4);
        assert_eq!(report.diagnostics.len(), 2);
        assert_eq!(report.diagnostics[0].leaf, LeafId(1));
        assert_eq!(report.diagnostics[1].leaf, LeafId(2));
        assert!(report.diagnostics.iter().all(|diagnostic| {
            diagnostic.canonical_registered_name.as_slice() == OTHER_CONDITION
        }));
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn preflight_error_precedes_controller_construction() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let (facade, allocator) = collaborators(&events, true);
        let graph = StateGraphRuntime::<MotionGraphLeafPayload<()>>::new();
        let result = OwningMotionGraphSession::try_new(graph, facade, allocator);
        assert!(matches!(
            result,
            Err(MotionGraphSessionCreateError::Preflight(
                MotionGraphCapabilityPreflightError::GraphNotProcessed
            ))
        ));
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn same_owned_tokens_survive_session_move_ticks_and_end_all() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let (facade, allocator) = collaborators(&events, true);
        let session = OwningMotionGraphSession::try_new(
            processed_graph(Some(HAS_INTENT_REGISTERED_NAME), true),
            facade,
            allocator,
        )
        .unwrap();
        let mut moved_session = session;
        moved_session.update(1.0 / 60.0).unwrap();
        assert_eq!(moved_session.active_behaviours().len(), 1);
        assert!(
            !moved_session
                .active_behaviours()
                .next()
                .unwrap()
                .1
                .unwrap()
                .first_update()
        );
        moved_session.update(1.0 / 30.0).unwrap();
        let before_end = events.borrow().len();
        moved_session.end_all_behaviours().unwrap();
        assert_eq!(events.borrow().len(), before_end);
        assert_eq!(moved_session.active_behaviours().len(), 0);
        assert_eq!(
            moved_session.delta_time().to_bits(),
            (1.0f32 / 30.0).to_bits()
        );
        assert_eq!(moved_session.current_state(), Some(StateId(0)));
        assert_eq!(moved_session.last_state(), Some(StateId(0)));
        assert_eq!(
            moved_session.time_in_current_state().to_bits(),
            (1.0f32 / 30.0).to_bits()
        );
        assert_eq!(moved_session.graph().unwrap().leaves().len(), 2);

        let events = events.borrow();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::Allocate(_)))
                .count(),
            1
        );
        assert!(events.iter().all(|event| match event {
            Event::Facade(token, _) => *token == 0xface,
            Event::Allocate(token) => *token == 0xa110,
        }));
    }

    #[test]
    fn poisoned_session_neither_update_nor_end_all_retries_callbacks() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let (facade, allocator) = collaborators(&events, false);
        let mut session =
            OwningMotionGraphSession::try_new(processed_graph(None, true), facade, allocator)
                .unwrap();
        let error = session.update(0.5).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Lifecycle(DynamicStateGraphLifecycleError::Begin {
                source: MotionGraphDynamicBackendError::PlayAnimationBegin {
                    source: PlayAnimationRuntimeError::MissingInstance,
                    ..
                },
                ..
            })
        ));
        assert!(session.is_poisoned());
        assert_eq!(session.active_behaviours().len(), 1);
        assert!(session.active_behaviours().next().unwrap().1.is_none());
        let after_failure = events.borrow().clone();

        assert!(matches!(
            session.update(0.25),
            Err(DynamicStateGraphControllerError::Invariant(
                DynamicStateGraphInvariantError::Poisoned
            ))
        ));
        assert!(matches!(
            session.end_all_behaviours(),
            Err(DynamicStateGraphControllerError::Invariant(
                DynamicStateGraphInvariantError::Poisoned
            ))
        ));
        assert_eq!(*events.borrow(), after_failure);
        assert_eq!(session.active_behaviours().len(), 1);
    }

    #[test]
    fn dropping_live_session_performs_no_implicit_end_or_collaborator_call() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let (facade, allocator) = collaborators(&events, true);
        let mut session =
            OwningMotionGraphSession::try_new(processed_graph(None, true), facade, allocator)
                .unwrap();
        session.update(0.5).unwrap();
        assert_eq!(session.active_behaviours().len(), 1);
        events.borrow_mut().clear();
        drop(session);
        assert!(events.borrow().is_empty());
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Keep the complete forwarding facade local to this test.
    fn supported_condition_error_keeps_exact_nested_identity() {
        struct MissingFacade(Facade);

        impl HasIntentCollaborator for MissingFacade {
            type Error = SpyError;
            type MotionGraph = u8;
            type Intents = u8;

            fn cached_motion_graph(&mut self) -> Result<Option<u8>, SpyError> {
                self.0.record("has.cached");
                Ok(None)
            }

            fn find_motion_graph(&mut self) -> Result<Option<u8>, SpyError> {
                self.0.record("has.find");
                Ok(None)
            }

            fn motion_graph_intents(&mut self, _: &u8) -> Result<u8, SpyError> {
                unreachable!()
            }

            fn lookup_intent(&mut self, _: &u8, _: &[u8]) -> Result<Option<f32>, SpyError> {
                unreachable!()
            }
        }

        impl WillExpireCollaborator for MissingFacade {
            type Error = SpyError;
            type AnimInfo = u8;
            type Animatable = u8;

            fn find_anim_info(&mut self) -> Result<Option<u8>, SpyError> {
                self.0.find_anim_info()
            }
            fn resolve_special_tag(&mut self, info: &u8, tag: &[u8]) -> Result<f32, SpyError> {
                self.0.resolve_special_tag(info, tag)
            }
            fn cached_animatable(&mut self) -> Result<Option<u8>, SpyError> {
                self.0.cached_animatable()
            }
            fn find_animatable(&mut self) -> Result<Option<u8>, SpyError> {
                self.0.find_animatable()
            }
            fn transitioning(&mut self, value: &u8) -> Result<bool, SpyError> {
                self.0.transitioning(value)
            }
            fn anim_property(&mut self, value: &u8) -> Result<Option<AnimProperty>, SpyError> {
                self.0.anim_property(value)
            }
        }

        impl PlayAnimationLifecycleBackend for MissingFacade {
            type Error = SpyError;
            fn resolve_interface(
                &mut self,
                kind: PlayAnimationInterfaceKind,
            ) -> Result<PlayAnimationInterfaceResolution, SpyError> {
                self.0.resolve_interface(kind)
            }
            fn motion_intent(&mut self, name: FastString36) -> Result<Option<u32>, SpyError> {
                self.0.motion_intent(name)
            }
            fn filtered_motion_intent(
                &mut self,
                name: FastString36,
            ) -> Result<Option<u32>, SpyError> {
                self.0.filtered_motion_intent(name)
            }
            fn tree_attribute(
                &mut self,
                name: FastString30,
            ) -> Result<Option<TreeAttributeSample>, SpyError> {
                self.0.tree_attribute(name)
            }
            fn add_settable_attribute(
                &mut self,
                name: FastString30,
                value: u32,
                normalize: bool,
                sequence: i32,
            ) -> Result<(), SpyError> {
                self.0
                    .add_settable_attribute(name, value, normalize, sequence)
            }
            fn selected_pro_skater(&mut self) -> Result<FastString30, SpyError> {
                self.0.selected_pro_skater()
            }
            fn set_construction_attribute(
                &mut self,
                name: FastString30,
                value: FastString30,
            ) -> Result<(), SpyError> {
                self.0.set_construction_attribute(name, value)
            }
            fn next_transition_override(
                &mut self,
            ) -> Result<Option<RawAnimationTransitionParams>, SpyError> {
                self.0.next_transition_override()
            }
            fn clear_transition_override(&mut self) -> Result<(), SpyError> {
                self.0.clear_transition_override()
            }
            fn is_switch(&mut self) -> Result<bool, SpyError> {
                self.0.is_switch()
            }
            fn is_mirrored(&mut self) -> Result<bool, SpyError> {
                self.0.is_mirrored()
            }
            fn physical_output(&mut self) -> Result<PhysicalOutputObservation, SpyError> {
                self.0.physical_output()
            }
            fn dispatch(&mut self, request: AnimationDispatch) -> Result<u8, SpyError> {
                self.0.dispatch(request)
            }
        }

        impl PlayRemappedAnimationCollaborator for MissingFacade {
            type Error = SpyError;

            fn write_mirror_override_raw(&mut self, value: u8) -> Result<(), SpyError> {
                self.0.write_mirror_override_raw(value)
            }

            fn setup_remapped_animation(
                &mut self,
                request: crate::play_remapped_animation::RemappedAnimationSetup<'_>,
            ) -> Result<(), SpyError> {
                self.0.setup_remapped_animation(request)
            }

            fn update_remapped_animation(
                &mut self,
                request: crate::play_remapped_animation::RemappedAnimationUpdate<'_>,
            ) -> Result<(), SpyError> {
                self.0.update_remapped_animation(request)
            }
        }

        let events = Rc::new(RefCell::new(Vec::new()));
        let (facade, allocator) = collaborators(&events, true);
        let mut session = OwningMotionGraphSession::try_new(
            processed_graph(Some(HAS_INTENT_REGISTERED_NAME), false),
            MissingFacade(facade),
            allocator,
        )
        .unwrap();
        let error = session.update(0.25).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Activation(
                crate::state_graph_runtime::StateGraphActivationError::Backend(
                    MotionGraphDynamicBackendError::HasIntent {
                        leaf: LeafId(0),
                        source: HasIntentActivationError::MissingMotionGraph,
                    }
                )
            )
        ));
        assert_eq!(
            *events.borrow(),
            [
                Event::Facade(0xface, "has.cached"),
                Event::Facade(0xface, "has.find")
            ]
        );
    }
}
