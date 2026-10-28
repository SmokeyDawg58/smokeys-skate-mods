//! Authority-bound owner for the recovered dynamic `MotionGraph` slice.
//!
//! This is the production-facing layer above the deliberately synthetic
//! `()`-Context backend. Construction installs the recovered six-slot Context
//! as `{controller, entity, null, null, null, null}` and supplies the same
//! issued Entity authority to every entity-targeted collaborator call. The API
//! supplies no replacement token, but arbitrary runtime implementations remain
//! trusted to honor the argument. `CreateInstance` is Context-free.
//!
//! The opaque identities are checked host containment, not native addresses or
//! proof of Actor lifetime/UCOM behavior. The generic dynamic controller is
//! still the fallback controller, not the generated onboard controller, and
//! only `HasIntent`, `WillExpire`, and `PlayAnimation` are implemented.

use std::{error::Error, fmt};

use crate::{
    dynamic_state_graph_controller::{
        DynamicBehaviourCallbackRequest, DynamicBehaviourCreateRequest,
        DynamicConditionActivationRequest, DynamicStateGraphBackend, DynamicStateGraphController,
        DynamicStateGraphControllerError, DynamicStateGraphInvariantError,
        DynamicTransitionHookRequest,
    },
    motion_graph_capability_preflight::{
        MotionGraphCapabilityPreflightError, MotionGraphCapabilityPreflightReport,
        preflight_motion_graph_capabilities,
    },
    motion_graph_dynamic_backend::{
        BorrowedMotionGraphBackend, MotionGraphDynamicBackendError, MotionGraphDynamicCallbackSite,
        MotionGraphDynamicIdentityError, validate_motion_graph_callback_identity,
    },
    motion_graph_entity_authority::{
        BoundMotionGraphContext, MotionGraphControllerToken, MotionGraphHasIntentRuntime,
        MotionGraphPlayAnimationRuntime, MotionGraphPlayRemappedAnimationRuntime,
        MotionGraphRuntimeFacade, MotionGraphWillExpireRuntime,
    },
    motion_graph_leaf_factory::MotionGraphLeafPayload,
    play_animation_lifecycle::{PlayAnimationInstanceAllocator, PlayAnimationRuntimeInstance},
    state_graph_context::{StateGraphContext, StateGraphContextHandle},
    state_graph_runtime::{LeafId, StateGraphRuntime, StateId},
};

pub type AuthorityBoundMotionGraphController<O, K> = DynamicStateGraphController<
    MotionGraphLeafPayload<O>,
    PlayAnimationRuntimeInstance,
    BoundMotionGraphContext<K>,
>;

pub type AuthorityBoundMotionGraphLeafError<K, R, A> = MotionGraphDynamicBackendError<
    <R as MotionGraphHasIntentRuntime<K>>::Error,
    <R as MotionGraphWillExpireRuntime<K>>::Error,
    <A as PlayAnimationInstanceAllocator>::Error,
    <R as MotionGraphPlayAnimationRuntime<K>>::Error,
    <R as MotionGraphPlayRemappedAnimationRuntime<K>>::Error,
>;

pub type AuthorityBoundMotionGraphTickError<K, R, A> = DynamicStateGraphControllerError<
    AuthorityBoundMotionGraphBackendError<AuthorityBoundMotionGraphLeafError<K, R, A>>,
>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorityBoundMotionGraphCreateError {
    Preflight(MotionGraphCapabilityPreflightError),
    UnsupportedCapabilities(MotionGraphCapabilityPreflightReport),
    Controller(DynamicStateGraphInvariantError),
}

impl fmt::Display for AuthorityBoundMotionGraphCreateError {
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

impl Error for AuthorityBoundMotionGraphCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Preflight(error) => Some(error),
            Self::Controller(error) => Some(error),
            Self::UnsupportedCapabilities(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionGraphContextBindingError {
    MissingController,
    ForeignController,
    MissingEntity,
    ForeignEntity,
}

impl fmt::Display for MotionGraphContextBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingController => "MotionGraph Context has no controller authority",
            Self::ForeignController => "MotionGraph Context names a foreign controller authority",
            Self::MissingEntity => "MotionGraph Context has no Entity authority",
            Self::ForeignEntity => "MotionGraph Context names a foreign Entity authority",
        })
    }
}

impl Error for MotionGraphContextBindingError {}

#[derive(Debug)]
pub enum AuthorityBoundMotionGraphBackendError<E> {
    Identity(MotionGraphDynamicIdentityError),
    Binding(MotionGraphContextBindingError),
    Leaf(E),
}

impl<E: fmt::Display> fmt::Display for AuthorityBoundMotionGraphBackendError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(error) => error.fmt(formatter),
            Self::Binding(error) => error.fmt(formatter),
            Self::Leaf(error) => error.fmt(formatter),
        }
    }
}

impl<E: Error + 'static> Error for AuthorityBoundMotionGraphBackendError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Identity(error) => error,
            Self::Binding(error) => error,
            Self::Leaf(error) => error,
        })
    }
}

struct MotionGraphSessionBinding<K> {
    controller: MotionGraphControllerToken,
    entity: crate::motion_graph_entity_authority::MotionGraphEntityAuthority<K>,
}

impl<K> MotionGraphSessionBinding<K> {
    fn validate(
        &self,
        context: &BoundMotionGraphContext<K>,
    ) -> Result<(), MotionGraphContextBindingError> {
        let controller = context
            .controller()
            .ok_or(MotionGraphContextBindingError::MissingController)?;
        if controller != &self.controller {
            return Err(MotionGraphContextBindingError::ForeignController);
        }
        let entity = context
            .entity()
            .ok_or(MotionGraphContextBindingError::MissingEntity)?;
        if entity != &self.entity {
            return Err(MotionGraphContextBindingError::ForeignEntity);
        }
        Ok(())
    }
}

struct AuthorityBoundBackend<'binding, 'backend, K, B> {
    binding: &'binding MotionGraphSessionBinding<K>,
    backend: &'backend mut B,
}

impl<O, K, B, E> DynamicStateGraphBackend<MotionGraphLeafPayload<O>, BoundMotionGraphContext<K>>
    for AuthorityBoundBackend<'_, '_, K, B>
where
    B: DynamicStateGraphBackend<
            MotionGraphLeafPayload<O>,
            (),
            Instance = PlayAnimationRuntimeInstance,
            Error = E,
        >,
{
    type Instance = PlayAnimationRuntimeInstance;
    type Error = AuthorityBoundMotionGraphBackendError<E>;

    fn condition_activation(
        &mut self,
        request: DynamicConditionActivationRequest<
            '_,
            MotionGraphLeafPayload<O>,
            BoundMotionGraphContext<K>,
        >,
    ) -> Result<bool, Self::Error> {
        validate_motion_graph_callback_identity(
            MotionGraphDynamicCallbackSite::Condition,
            request.leaf,
            request.registered_name,
            request.payload,
        )
        .map_err(AuthorityBoundMotionGraphBackendError::Identity)?;
        self.binding
            .validate(request.context)
            .map_err(AuthorityBoundMotionGraphBackendError::Binding)?;
        self.backend
            .condition_activation(DynamicConditionActivationRequest {
                leaf: request.leaf,
                registered_name: request.registered_name,
                payload: request.payload,
                context: &(),
                controller: request.controller,
            })
            .map_err(AuthorityBoundMotionGraphBackendError::Leaf)
    }

    fn create_instance(
        &mut self,
        request: DynamicBehaviourCreateRequest<'_, MotionGraphLeafPayload<O>>,
    ) -> Result<Option<Self::Instance>, Self::Error> {
        validate_motion_graph_callback_identity(
            MotionGraphDynamicCallbackSite::CreateInstance,
            request.leaf,
            request.registered_name,
            request.payload,
        )
        .map_err(AuthorityBoundMotionGraphBackendError::Identity)?;
        self.backend
            .create_instance(request)
            .map_err(AuthorityBoundMotionGraphBackendError::Leaf)
    }

    fn begin(
        &mut self,
        request: DynamicBehaviourCallbackRequest<
            '_,
            MotionGraphLeafPayload<O>,
            BoundMotionGraphContext<K>,
            Self::Instance,
        >,
    ) -> Result<(), Self::Error> {
        self.behaviour_callback(
            MotionGraphDynamicCallbackSite::Begin,
            request,
            DynamicStateGraphBackend::begin,
        )
    }

    fn update(
        &mut self,
        request: DynamicBehaviourCallbackRequest<
            '_,
            MotionGraphLeafPayload<O>,
            BoundMotionGraphContext<K>,
            Self::Instance,
        >,
    ) -> Result<(), Self::Error> {
        self.behaviour_callback(
            MotionGraphDynamicCallbackSite::Update,
            request,
            DynamicStateGraphBackend::update,
        )
    }

    fn end(
        &mut self,
        request: DynamicBehaviourCallbackRequest<
            '_,
            MotionGraphLeafPayload<O>,
            BoundMotionGraphContext<K>,
            Self::Instance,
        >,
    ) -> Result<(), Self::Error> {
        self.behaviour_callback(
            MotionGraphDynamicCallbackSite::End,
            request,
            DynamicStateGraphBackend::end,
        )
    }

    fn on_transition(
        &mut self,
        request: DynamicTransitionHookRequest<
            '_,
            MotionGraphLeafPayload<O>,
            BoundMotionGraphContext<K>,
        >,
    ) -> Result<(), Self::Error> {
        let error = validate_motion_graph_callback_identity(
            MotionGraphDynamicCallbackSite::TransitionHook(request.transition),
            request.hook,
            request.registered_name,
            request.payload,
        )
        .expect_err("no MotionGraph transition hook is reconstructed");
        Err(AuthorityBoundMotionGraphBackendError::Identity(error))
    }
}

impl<K, B> AuthorityBoundBackend<'_, '_, K, B> {
    fn behaviour_callback<O, I, E>(
        &mut self,
        site: MotionGraphDynamicCallbackSite,
        request: DynamicBehaviourCallbackRequest<
            '_,
            MotionGraphLeafPayload<O>,
            BoundMotionGraphContext<K>,
            PlayAnimationRuntimeInstance,
        >,
        invoke: I,
    ) -> Result<(), AuthorityBoundMotionGraphBackendError<E>>
    where
        B: DynamicStateGraphBackend<
                MotionGraphLeafPayload<O>,
                (),
                Instance = PlayAnimationRuntimeInstance,
                Error = E,
            >,
        I: FnOnce(
            &mut B,
            DynamicBehaviourCallbackRequest<
                '_,
                MotionGraphLeafPayload<O>,
                (),
                PlayAnimationRuntimeInstance,
            >,
        ) -> Result<(), E>,
    {
        validate_motion_graph_callback_identity(
            site,
            request.leaf,
            request.registered_name,
            request.payload,
        )
        .map_err(AuthorityBoundMotionGraphBackendError::Identity)?;
        self.binding
            .validate(request.context)
            .map_err(AuthorityBoundMotionGraphBackendError::Binding)?;
        let mut unit = ();
        invoke(
            self.backend,
            DynamicBehaviourCallbackRequest {
                leaf: request.leaf,
                registered_name: request.registered_name,
                payload: request.payload,
                context: &mut unit,
                controller: request.controller,
                instance: request.instance,
            },
        )
        .map_err(AuthorityBoundMotionGraphBackendError::Leaf)
    }
}

/// Owns one authority-bound dynamic Motion controller, runtime facade, and
/// instance allocator.
///
/// The Context and binding are constructed internally. No component or
/// external-backend escape is exposed, so all entity-targeted collaborator
/// calls receive the same issued Entity authority and every Context-bearing
/// callback validates the same controller token.
pub struct AuthorityBoundMotionGraphSession<O, K, R, A> {
    binding: MotionGraphSessionBinding<K>,
    controller: AuthorityBoundMotionGraphController<O, K>,
    facade: MotionGraphRuntimeFacade<K, R>,
    allocator: A,
}

impl<O, K, R, A> AuthorityBoundMotionGraphSession<O, K, R, A> {
    /// Performs complete capability preflight before controller construction
    /// and before invoking any runtime or allocator method.
    ///
    /// # Errors
    ///
    /// Returns the exact preflight error, complete unsupported report, or
    /// generic-controller invariant failure.
    pub fn try_new(
        graph: StateGraphRuntime<MotionGraphLeafPayload<O>>,
        facade: MotionGraphRuntimeFacade<K, R>,
        allocator: A,
    ) -> Result<Self, AuthorityBoundMotionGraphCreateError>
    where
        R: MotionGraphHasIntentRuntime<K>
            + MotionGraphWillExpireRuntime<K>
            + MotionGraphPlayAnimationRuntime<K>
            + MotionGraphPlayRemappedAnimationRuntime<K>,
        A: PlayAnimationInstanceAllocator,
    {
        let report = preflight_motion_graph_capabilities(&graph)
            .map_err(AuthorityBoundMotionGraphCreateError::Preflight)?;
        if !report.has_full_leaf_coverage() {
            return Err(AuthorityBoundMotionGraphCreateError::UnsupportedCapabilities(report));
        }
        let binding = MotionGraphSessionBinding {
            controller: MotionGraphControllerToken::issue(),
            entity: facade.entity_authority().shallow_copy(),
        };
        let context = StateGraphContext::new(
            Some(binding.controller.shallow_copy()),
            Some(binding.entity.shallow_copy()),
            None,
            None,
            None,
            None,
        );
        let controller = DynamicStateGraphController::new(graph, context)
            .map_err(AuthorityBoundMotionGraphCreateError::Controller)?;
        Ok(Self {
            binding,
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

impl<O, K, R, A> AuthorityBoundMotionGraphSession<O, K, R, A>
where
    R: MotionGraphHasIntentRuntime<K>
        + MotionGraphWillExpireRuntime<K>
        + MotionGraphPlayAnimationRuntime<K>
        + MotionGraphPlayRemappedAnimationRuntime<K>,
    A: PlayAnimationInstanceAllocator,
{
    /// Runs one generic-controller tick through an authority-aware ephemeral
    /// runtime adapter.
    ///
    /// # Errors
    ///
    /// Preserves binding, identity, callback, lifecycle, and controller errors.
    pub fn update(
        &mut self,
        delta_time: f32,
    ) -> Result<(), AuthorityBoundMotionGraphTickError<K, R, A>> {
        let Self {
            binding,
            controller,
            facade,
            allocator,
        } = self;
        let mut facade = facade.borrow();
        let mut leaf_backend = BorrowedMotionGraphBackend::new(&mut facade, allocator);
        let mut backend = AuthorityBoundBackend {
            binding,
            backend: &mut leaf_backend,
        };
        controller.update(&mut backend, delta_time)
    }

    /// Ends all active Behaviours through the same authority-aware adapter.
    ///
    /// # Errors
    ///
    /// Preserves reverse-order removal, retained failing-prefix, and poison
    /// behavior from the generic controller.
    pub fn end_all_behaviours(
        &mut self,
    ) -> Result<(), AuthorityBoundMotionGraphTickError<K, R, A>> {
        let Self {
            binding,
            controller,
            facade,
            allocator,
        } = self;
        let mut facade = facade.borrow();
        let mut leaf_backend = BorrowedMotionGraphBackend::new(&mut facade, allocator);
        let mut backend = AuthorityBoundBackend {
            binding,
            backend: &mut leaf_backend,
        };
        controller.end_all_behaviours(&mut backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dynamic_state_graph_controller::DynamicStateGraphLifecycleError,
        motion_graph_conditions::{AnimProperty, HasIntentCondition},
        motion_graph_entity_authority::{ActorGraphAuthority, MotionGraphEntityAuthority},
        motion_graph_leaf_factory::{HAS_INTENT_REGISTERED_NAME, PLAY_ANIMATION_REGISTERED_NAME},
        play_animation::{FastString30, FastString36, PlayAnimationBehaviour},
        play_animation_lifecycle::{
            AnimationDispatch, PhysicalOutputObservation, PlayAnimationInterfaceKind,
            PlayAnimationInterfaceResolution, RawAnimationTransitionParams, TreeAttributeSample,
        },
        state_graph_runtime::{
            FactoryMissPolicy, LeafCreateRequest, LeafKind, StateGraphActivationError,
            StateGraphLeafFactory,
        },
    };
    use skate2_formats::state_graph::{
        StateGraphBinaryAttribute, StateGraphBinaryElement, state_graph_string_hash32,
    };
    use std::{cell::RefCell, convert::Infallible, rc::Rc};

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ObservedCall {
        operation: &'static str,
        key: u32,
        same_as_first: bool,
    }

    struct Runtime {
        first: Option<MotionGraphEntityAuthority<u32>>,
        calls: Rc<RefCell<Vec<ObservedCall>>>,
    }

    impl Runtime {
        fn record(&mut self, entity: &MotionGraphEntityAuthority<u32>, operation: &'static str) {
            let same_as_first = self.first.as_ref().is_none_or(|first| first == entity);
            if self.first.is_none() {
                self.first = Some(entity.shallow_copy());
            }
            self.calls.borrow_mut().push(ObservedCall {
                operation,
                key: *entity.key(),
                same_as_first,
            });
        }
    }

    impl MotionGraphHasIntentRuntime<u32> for Runtime {
        type Error = Infallible;
        type MotionGraph = u8;
        type Intents = u8;

        fn cached_motion_graph(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<Option<u8>, Infallible> {
            self.record(entity, "has.cached");
            Ok(Some(1))
        }

        fn find_motion_graph(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<Option<u8>, Infallible> {
            self.record(entity, "has.find");
            Ok(Some(1))
        }

        fn motion_graph_intents(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: &u8,
        ) -> Result<u8, Infallible> {
            self.record(entity, "has.intents");
            Ok(2)
        }

        fn lookup_intent(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: &u8,
            _: &[u8],
        ) -> Result<Option<f32>, Infallible> {
            self.record(entity, "has.lookup");
            Ok(Some(1.0))
        }
    }

    impl MotionGraphWillExpireRuntime<u32> for Runtime {
        type Error = Infallible;
        type AnimInfo = u8;
        type Animatable = u8;

        fn find_anim_info(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<Option<u8>, Infallible> {
            self.record(entity, "will.info");
            Ok(None)
        }

        fn resolve_special_tag(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: &u8,
            _: &[u8],
        ) -> Result<f32, Infallible> {
            self.record(entity, "will.tag");
            Ok(0.0)
        }

        fn cached_animatable(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<Option<u8>, Infallible> {
            self.record(entity, "will.cached");
            Ok(Some(1))
        }

        fn find_animatable(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<Option<u8>, Infallible> {
            self.record(entity, "will.find");
            Ok(Some(1))
        }

        fn transitioning(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: &u8,
        ) -> Result<bool, Infallible> {
            self.record(entity, "will.transitioning");
            Ok(false)
        }

        fn anim_property(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: &u8,
        ) -> Result<Option<AnimProperty>, Infallible> {
            self.record(entity, "will.property");
            Ok(Some(AnimProperty {
                is_playing_raw: 1,
                has_expired_or_looped_raw: 1,
                has_expired_or_looped_by_time: 0.0,
                time_to_expire_or_loop: 0.0,
            }))
        }
    }

    impl MotionGraphPlayAnimationRuntime<u32> for Runtime {
        type Error = Infallible;

        fn resolve_interface(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: PlayAnimationInterfaceKind,
        ) -> Result<PlayAnimationInterfaceResolution, Infallible> {
            self.record(entity, "play.resolve");
            Ok(PlayAnimationInterfaceResolution::Resolved)
        }

        fn motion_intent(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: FastString36,
        ) -> Result<Option<u32>, Infallible> {
            self.record(entity, "play.intent");
            Ok(None)
        }

        fn filtered_motion_intent(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: FastString36,
        ) -> Result<Option<u32>, Infallible> {
            self.record(entity, "play.filtered");
            Ok(None)
        }

        fn tree_attribute(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: FastString30,
        ) -> Result<Option<TreeAttributeSample>, Infallible> {
            self.record(entity, "play.tree");
            Ok(None)
        }

        fn add_settable_attribute(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: FastString30,
            _: u32,
            _: bool,
            _: i32,
        ) -> Result<(), Infallible> {
            self.record(entity, "play.add");
            Ok(())
        }

        fn selected_pro_skater(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<FastString30, Infallible> {
            self.record(entity, "play.pro");
            Ok(FastString30 { words: [0; 5] })
        }

        fn set_construction_attribute(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: FastString30,
            _: FastString30,
        ) -> Result<(), Infallible> {
            self.record(entity, "play.construction");
            Ok(())
        }

        fn next_transition_override(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<Option<RawAnimationTransitionParams>, Infallible> {
            self.record(entity, "play.override");
            Ok(Some(RawAnimationTransitionParams {
                transition_type_raw: 0,
                blend_length_bits: 0,
                placement_raw: 0,
                flags_raw: 0,
            }))
        }

        fn clear_transition_override(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<(), Infallible> {
            self.record(entity, "play.clear");
            Ok(())
        }

        fn is_switch(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<bool, Infallible> {
            self.record(entity, "play.switch");
            Ok(false)
        }

        fn is_mirrored(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<bool, Infallible> {
            self.record(entity, "play.mirror");
            Ok(false)
        }

        fn physical_output(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
        ) -> Result<PhysicalOutputObservation, Infallible> {
            self.record(entity, "play.physical");
            Ok(PhysicalOutputObservation::HoldingSkateboardRaw(1))
        }

        fn dispatch(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: AnimationDispatch,
        ) -> Result<u8, Infallible> {
            self.record(entity, "play.dispatch");
            Ok(1)
        }
    }

    impl MotionGraphPlayRemappedAnimationRuntime<u32> for Runtime {
        type Error = Infallible;

        fn write_remapped_mirror_override_raw(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: u8,
        ) -> Result<(), Infallible> {
            self.record(entity, "remapped.mirror");
            Ok(())
        }

        fn setup_remapped_animation(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: crate::play_remapped_animation::RemappedAnimationSetup<'_>,
        ) -> Result<(), Infallible> {
            self.record(entity, "remapped.setup");
            Ok(())
        }

        fn update_remapped_animation(
            &mut self,
            entity: &MotionGraphEntityAuthority<u32>,
            _: crate::play_remapped_animation::RemappedAnimationUpdate<'_>,
        ) -> Result<(), Infallible> {
            self.record(entity, "remapped.update");
            Ok(())
        }
    }

    struct Allocator {
        calls: Rc<RefCell<usize>>,
    }

    impl PlayAnimationInstanceAllocator for Allocator {
        type Error = Infallible;

        fn allocate(&mut self, _: usize, _: &[u8], _: usize) -> Result<bool, Infallible> {
            *self.calls.borrow_mut() += 1;
            Ok(true)
        }
    }

    struct Factory;

    impl StateGraphLeafFactory for Factory {
        type Payload = MotionGraphLeafPayload<()>;
        type Error = Infallible;

        fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
            static CONDITIONS: std::sync::LazyLock<Vec<Vec<u8>>> =
                std::sync::LazyLock::new(|| vec![HAS_INTENT_REGISTERED_NAME.to_vec()]);
            static BEHAVIOURS: std::sync::LazyLock<Vec<Vec<u8>>> =
                std::sync::LazyLock::new(|| vec![PLAY_ANIMATION_REGISTERED_NAME.to_vec()]);
            static HOOKS: Vec<Vec<u8>> = Vec::new();
            match kind {
                LeafKind::Condition => &CONDITIONS,
                LeafKind::Behaviour => &BEHAVIOURS,
                LeafKind::Hook => &HOOKS,
            }
        }

        fn create(&mut self, request: LeafCreateRequest<'_>) -> Result<Self::Payload, Infallible> {
            Ok(match request.registered_name {
                HAS_INTENT_REGISTERED_NAME => MotionGraphLeafPayload::HasIntent(
                    HasIntentCondition::from_attributes(request.attributes).unwrap(),
                ),
                PLAY_ANIMATION_REGISTERED_NAME => MotionGraphLeafPayload::PlayAnimation(
                    PlayAnimationBehaviour::from_attributes(request.attributes).unwrap(),
                ),
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

    fn processed_graph() -> StateGraphRuntime<MotionGraphLeafPayload<()>> {
        let root = element(
            b"state",
            vec![attr(b"name", b"Root")],
            vec![
                element(
                    b"expression",
                    Vec::new(),
                    vec![element(
                        b"condition",
                        vec![
                            attr(b"name", HAS_INTENT_REGISTERED_NAME),
                            attr(b"intent", b"Push"),
                        ],
                        Vec::new(),
                    )],
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
        let mut factory = Factory;
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        graph.process_after_build(&mut factory).unwrap();
        graph
    }

    struct MismatchedFactory;

    impl StateGraphLeafFactory for MismatchedFactory {
        type Payload = MotionGraphLeafPayload<()>;
        type Error = Infallible;

        fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
            static CONDITIONS: std::sync::LazyLock<Vec<Vec<u8>>> =
                std::sync::LazyLock::new(|| vec![HAS_INTENT_REGISTERED_NAME.to_vec()]);
            static EMPTY: Vec<Vec<u8>> = Vec::new();
            match kind {
                LeafKind::Condition => &CONDITIONS,
                LeafKind::Behaviour | LeafKind::Hook => &EMPTY,
            }
        }

        fn create(&mut self, _: LeafCreateRequest<'_>) -> Result<Self::Payload, Infallible> {
            Ok(MotionGraphLeafPayload::PlayAnimation(
                PlayAnimationBehaviour::from_attributes(&[attr(b"anim", b"Idle")]).unwrap(),
            ))
        }
    }

    fn mismatched_condition_graph() -> StateGraphRuntime<MotionGraphLeafPayload<()>> {
        let root = element(
            b"state",
            vec![attr(b"name", b"Root")],
            vec![element(
                b"expression",
                Vec::new(),
                vec![element(
                    b"condition",
                    vec![
                        attr(b"name", HAS_INTENT_REGISTERED_NAME),
                        attr(b"intent", b"Push"),
                    ],
                    Vec::new(),
                )],
            )],
        );
        let mut factory = MismatchedFactory;
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        graph.process_after_build(&mut factory).unwrap();
        graph
    }

    struct MismatchedBehaviourFactory;

    impl StateGraphLeafFactory for MismatchedBehaviourFactory {
        type Payload = MotionGraphLeafPayload<()>;
        type Error = Infallible;

        fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
            static BEHAVIOURS: std::sync::LazyLock<Vec<Vec<u8>>> =
                std::sync::LazyLock::new(|| vec![PLAY_ANIMATION_REGISTERED_NAME.to_vec()]);
            static EMPTY: Vec<Vec<u8>> = Vec::new();
            match kind {
                LeafKind::Behaviour => &BEHAVIOURS,
                LeafKind::Condition | LeafKind::Hook => &EMPTY,
            }
        }

        fn create(&mut self, _: LeafCreateRequest<'_>) -> Result<Self::Payload, Infallible> {
            Ok(MotionGraphLeafPayload::HasIntent(
                HasIntentCondition::from_attributes(&[attr(b"intent", b"Push")]).unwrap(),
            ))
        }
    }

    fn mismatched_behaviour_graph() -> StateGraphRuntime<MotionGraphLeafPayload<()>> {
        let root = element(
            b"state",
            vec![attr(b"name", b"Root")],
            vec![element(
                b"behaviour",
                vec![
                    attr(b"name", PLAY_ANIMATION_REGISTERED_NAME),
                    attr(b"anim", b"Idle"),
                ],
                Vec::new(),
            )],
        );
        let mut factory = MismatchedBehaviourFactory;
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        graph.process_after_build(&mut factory).unwrap();
        graph
    }

    type TestParts = (
        MotionGraphRuntimeFacade<u32, Runtime>,
        Allocator,
        Rc<RefCell<Vec<ObservedCall>>>,
        Rc<RefCell<usize>>,
    );

    fn parts(key: u32) -> TestParts {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let allocations = Rc::new(RefCell::new(0));
        (
            ActorGraphAuthority::new(key).motion_graph_facade(Runtime {
                first: None,
                calls: Rc::clone(&calls),
            }),
            Allocator {
                calls: Rc::clone(&allocations),
            },
            calls,
            allocations,
        )
    }

    #[test]
    fn constructor_installs_exact_bound_context_shape() {
        let (facade, allocator, _, _) = parts(0xaced);
        let expected_entity = facade.entity_authority().shallow_copy();
        let session =
            AuthorityBoundMotionGraphSession::try_new(processed_graph(), facade, allocator)
                .unwrap();
        let context = session.controller.context();

        assert_eq!(context.controller(), Some(&session.binding.controller));
        assert_eq!(context.entity(), Some(&expected_entity));
        assert_eq!(context.entity(), Some(&session.binding.entity));
        assert!(context.instance().is_none());
        assert!(context.camera_man().is_none());
        assert!(context.behaviour_source().is_none());
        assert!(context.behaviour_sink().is_none());
    }

    #[test]
    fn same_authority_survives_move_ticks_and_end_all() {
        let (facade, allocator, calls, allocations) = parts(0xaced);
        let session =
            AuthorityBoundMotionGraphSession::try_new(processed_graph(), facade, allocator)
                .unwrap();
        let mut moved = session;

        moved.update(1.0 / 60.0).unwrap();
        moved.update(1.0 / 30.0).unwrap();
        moved.end_all_behaviours().unwrap();

        assert_eq!(*allocations.borrow(), 1);
        assert!(!calls.borrow().is_empty());
        assert!(
            calls
                .borrow()
                .iter()
                .all(|call| call.key == 0xaced && call.same_as_first)
        );
        assert_eq!(moved.delta_time().to_bits(), (1.0_f32 / 30.0).to_bits());
        assert_eq!(moved.current_state(), Some(StateId(0)));
        assert!(moved.active_behaviours().next().is_none());
    }

    #[test]
    fn equal_external_keys_do_not_alias_entity_or_controller_issuance() {
        let (first_facade, first_allocator, _, _) = parts(7);
        let (second_facade, second_allocator, _, _) = parts(7);
        let first = AuthorityBoundMotionGraphSession::try_new(
            processed_graph(),
            first_facade,
            first_allocator,
        )
        .unwrap();
        let second = AuthorityBoundMotionGraphSession::try_new(
            processed_graph(),
            second_facade,
            second_allocator,
        )
        .unwrap();

        assert_ne!(first.binding.entity, second.binding.entity);
        assert_ne!(first.binding.controller, second.binding.controller);
    }

    #[test]
    fn one_actor_authority_shares_entity_but_sessions_issue_distinct_controllers() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let actor = ActorGraphAuthority::new(11_u32);
        let runtime = || Runtime {
            first: None,
            calls: Rc::clone(&calls),
        };
        let allocations = Rc::new(RefCell::new(0));
        let allocator = || Allocator {
            calls: Rc::clone(&allocations),
        };
        let first = AuthorityBoundMotionGraphSession::try_new(
            processed_graph(),
            actor.motion_graph_facade(runtime()),
            allocator(),
        )
        .unwrap();
        let second = AuthorityBoundMotionGraphSession::try_new(
            processed_graph(),
            actor.motion_graph_facade(runtime()),
            allocator(),
        )
        .unwrap();

        assert_eq!(first.binding.entity, second.binding.entity);
        assert_ne!(first.binding.controller, second.binding.controller);
    }

    #[test]
    fn binding_validator_distinguishes_all_missing_and_foreign_slots() {
        let controller = MotionGraphControllerToken::issue();
        let entity = MotionGraphEntityAuthority::issue(1_u32);
        let binding = MotionGraphSessionBinding {
            controller: controller.shallow_copy(),
            entity: entity.shallow_copy(),
        };
        let context =
            |controller, entity| StateGraphContext::new(controller, entity, None, None, None, None);

        assert_eq!(
            binding.validate(&context(None, Some(entity.shallow_copy()))),
            Err(MotionGraphContextBindingError::MissingController)
        );
        assert_eq!(
            binding.validate(&context(
                Some(MotionGraphControllerToken::issue()),
                Some(entity.shallow_copy())
            )),
            Err(MotionGraphContextBindingError::ForeignController)
        );
        assert_eq!(
            binding.validate(&context(Some(controller.shallow_copy()), None)),
            Err(MotionGraphContextBindingError::MissingEntity)
        );
        assert_eq!(
            binding.validate(&context(
                Some(controller.shallow_copy()),
                Some(MotionGraphEntityAuthority::issue(1_u32))
            )),
            Err(MotionGraphContextBindingError::ForeignEntity)
        );
    }

    #[test]
    fn identity_error_precedes_bad_context_and_all_effects() {
        let actor = ActorGraphAuthority::new(19_u32);
        let calls = Rc::new(RefCell::new(Vec::new()));
        let allocations = Rc::new(RefCell::new(0));
        let mut facade = actor.motion_graph_facade(Runtime {
            first: None,
            calls: Rc::clone(&calls),
        });
        let mut allocator = Allocator {
            calls: Rc::clone(&allocations),
        };
        let binding = MotionGraphSessionBinding {
            controller: MotionGraphControllerToken::issue(),
            entity: facade.entity_authority().shallow_copy(),
        };
        let bad_context: BoundMotionGraphContext<u32> = StateGraphContext::empty();
        let mut controller =
            DynamicStateGraphController::new(mismatched_condition_graph(), bad_context).unwrap();
        let mut facade = facade.borrow();
        let mut leaf_backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
        let mut backend = AuthorityBoundBackend {
            binding: &binding,
            backend: &mut leaf_backend,
        };

        let error = controller.update(&mut backend, 0.25).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Activation(StateGraphActivationError::Backend(
                AuthorityBoundMotionGraphBackendError::Identity(_)
            ))
        ));
        assert!(calls.borrow().is_empty());
        assert_eq!(*allocations.borrow(), 0);
    }

    #[test]
    fn binding_error_precedes_entity_runtime_effects() {
        let actor = ActorGraphAuthority::new(23_u32);
        let calls = Rc::new(RefCell::new(Vec::new()));
        let allocations = Rc::new(RefCell::new(0));
        let mut facade = actor.motion_graph_facade(Runtime {
            first: None,
            calls: Rc::clone(&calls),
        });
        let mut allocator = Allocator {
            calls: Rc::clone(&allocations),
        };
        let binding = MotionGraphSessionBinding {
            controller: MotionGraphControllerToken::issue(),
            entity: facade.entity_authority().shallow_copy(),
        };
        let context: BoundMotionGraphContext<u32> = StateGraphContext::new(
            Some(binding.controller.shallow_copy()),
            None,
            None,
            None,
            None,
            None,
        );
        let mut controller = DynamicStateGraphController::new(processed_graph(), context).unwrap();
        let mut facade = facade.borrow();
        let mut leaf_backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
        let mut backend = AuthorityBoundBackend {
            binding: &binding,
            backend: &mut leaf_backend,
        };

        let error = controller.update(&mut backend, 0.25).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Activation(StateGraphActivationError::Backend(
                AuthorityBoundMotionGraphBackendError::Binding(
                    MotionGraphContextBindingError::MissingEntity
                )
            ))
        ));
        assert!(calls.borrow().is_empty());
        assert_eq!(*allocations.borrow(), 0);
    }

    #[test]
    fn create_instance_identity_error_precedes_allocator_effect() {
        let actor = ActorGraphAuthority::new(29_u32);
        let calls = Rc::new(RefCell::new(Vec::new()));
        let allocations = Rc::new(RefCell::new(0));
        let mut facade = actor.motion_graph_facade(Runtime {
            first: None,
            calls: Rc::clone(&calls),
        });
        let mut allocator = Allocator {
            calls: Rc::clone(&allocations),
        };
        let binding = MotionGraphSessionBinding {
            controller: MotionGraphControllerToken::issue(),
            entity: facade.entity_authority().shallow_copy(),
        };
        let context: BoundMotionGraphContext<u32> = StateGraphContext::new(
            Some(binding.controller.shallow_copy()),
            Some(binding.entity.shallow_copy()),
            None,
            None,
            None,
            None,
        );
        let mut controller =
            DynamicStateGraphController::new(mismatched_behaviour_graph(), context).unwrap();
        let mut facade = facade.borrow();
        let mut leaf_backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
        let mut backend = AuthorityBoundBackend {
            binding: &binding,
            backend: &mut leaf_backend,
        };

        let error = controller.update(&mut backend, 0.25).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Lifecycle(
                DynamicStateGraphLifecycleError::CreateInstance {
                    source: AuthorityBoundMotionGraphBackendError::Identity(_),
                    ..
                }
            )
        ));
        assert!(calls.borrow().is_empty());
        assert_eq!(*allocations.borrow(), 0);
    }

    #[test]
    fn unprocessed_graph_fails_before_runtime_or_allocator_effects() {
        let (facade, allocator, calls, allocations) = parts(3);
        let graph = StateGraphRuntime::<MotionGraphLeafPayload<()>>::new();
        let result = AuthorityBoundMotionGraphSession::try_new(graph, facade, allocator);

        assert!(matches!(
            result,
            Err(AuthorityBoundMotionGraphCreateError::Preflight(
                MotionGraphCapabilityPreflightError::GraphNotProcessed
            ))
        ));
        assert!(calls.borrow().is_empty());
        assert_eq!(*allocations.borrow(), 0);
    }
}
