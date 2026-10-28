//! Borrowed dynamic-controller adapter for the four recovered `MotionGraph` leaves.
//!
//! The adapter borrows one collaborator facade for one adapter session, so
//! `HasIntent`, `WillExpire`, `PlayAnimation`, and `PlayRemappedAnimation` use that same facade during
//! the borrow. This does not prove stable entity identity across controller
//! ticks: callers can replace the adapter, and `()` contains no Actor slot.
//! The facade still owns all concrete
//! interface lookup and gameplay effects: this module does not reconstruct
//! UCOM, entity discovery, the native six-slot controller `Context`, opaque
//! leaves, transition hooks, generated-controller dispatch, or a live animation
//! backend. Consequently, this first seam deliberately implements the generic
//! controller only for `()` context and does not make the mounted graph runnable.

use std::{error::Error, fmt};

use crate::{
    dynamic_state_graph_controller::{
        DynamicBehaviourCallbackRequest, DynamicBehaviourCreateRequest,
        DynamicConditionActivationRequest, DynamicStateGraphBackend, DynamicTransitionHookRequest,
    },
    motion_graph_conditions::{
        HasIntentActivationError, HasIntentCollaborator, WillExpireActivationError,
        WillExpireCollaborator,
    },
    motion_graph_leaf_factory::{
        HAS_INTENT_REGISTERED_NAME, MotionGraphLeafPayload, MotionGraphLeafVariant,
        PLAY_ANIMATION_REGISTERED_NAME, PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
        WILL_EXPIRE_REGISTERED_NAME,
    },
    play_animation::PlayAnimationBehaviour,
    play_animation_lifecycle::{
        PlayAnimationInstanceAllocator, PlayAnimationLifecycleBackend, PlayAnimationRuntimeError,
        PlayAnimationRuntimeInstance,
    },
    play_remapped_animation::{PlayRemappedAnimationBehaviour, PlayRemappedAnimationCollaborator},
    state_graph_runtime::{LeafId, LeafKind, TransitionId},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionGraphDynamicCallbackSite {
    Condition,
    CreateInstance,
    Begin,
    Update,
    End,
    TransitionHook(TransitionId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionGraphDynamicIdentityReason {
    RegisteredOpaque,
    UnsupportedIdentity,
    CallbackKindMismatch {
        expected: LeafKind,
        actual: LeafKind,
    },
    PayloadVariantMismatch {
        expected: MotionGraphLeafVariant,
        actual: MotionGraphLeafVariant,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionGraphDynamicIdentityError {
    pub site: MotionGraphDynamicCallbackSite,
    pub leaf: LeafId,
    pub registered_name: Vec<u8>,
    pub actual: MotionGraphLeafVariant,
    pub reason: MotionGraphDynamicIdentityReason,
}

impl fmt::Display for MotionGraphDynamicIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "MotionGraph leaf {:?} at {:?} has unsupported callback identity {}/ {:?}: ",
            self.leaf,
            self.site,
            String::from_utf8_lossy(&self.registered_name),
            self.actual
        )?;
        match self.reason {
            MotionGraphDynamicIdentityReason::RegisteredOpaque => {
                formatter.write_str("registered payload has no recovered runtime body")
            }
            MotionGraphDynamicIdentityReason::UnsupportedIdentity => {
                formatter.write_str("no recovered runtime triple")
            }
            MotionGraphDynamicIdentityReason::CallbackKindMismatch { expected, actual } => {
                write!(formatter, "expected {expected:?} callback, got {actual:?}")
            }
            MotionGraphDynamicIdentityReason::PayloadVariantMismatch { expected, actual } => {
                write!(formatter, "expected {expected:?}, got {actual:?}")
            }
        }
    }
}

impl Error for MotionGraphDynamicIdentityError {}

#[derive(Debug)]
pub enum MotionGraphDynamicBackendError<HE, WE, AE, PE, RE> {
    Identity(MotionGraphDynamicIdentityError),
    HasIntent {
        leaf: LeafId,
        source: HasIntentActivationError<HE>,
    },
    WillExpire {
        leaf: LeafId,
        source: WillExpireActivationError<WE>,
    },
    PlayAnimationCreate {
        leaf: LeafId,
        source: PlayAnimationRuntimeError<AE>,
    },
    PlayAnimationBegin {
        leaf: LeafId,
        source: PlayAnimationRuntimeError<PE>,
    },
    PlayAnimationUpdate {
        leaf: LeafId,
        source: PlayAnimationRuntimeError<PE>,
    },
    PlayRemappedAnimationBegin {
        leaf: LeafId,
        source: RE,
    },
    PlayRemappedAnimationUpdate {
        leaf: LeafId,
        source: RE,
    },
}

impl<HE, WE, AE, PE, RE> fmt::Display for MotionGraphDynamicBackendError<HE, WE, AE, PE, RE>
where
    HE: fmt::Display,
    WE: fmt::Display,
    AE: fmt::Display,
    PE: fmt::Display,
    RE: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(error) => error.fmt(formatter),
            Self::HasIntent { leaf, source } => {
                write!(
                    formatter,
                    "HasIntent leaf {leaf:?} activation failed: {source}"
                )
            }
            Self::WillExpire { leaf, source } => {
                write!(
                    formatter,
                    "WillExpire leaf {leaf:?} activation failed: {source}"
                )
            }
            Self::PlayAnimationCreate { leaf, source } => {
                write!(
                    formatter,
                    "PlayAnimation leaf {leaf:?} CreateInstance failed: {source}"
                )
            }
            Self::PlayAnimationBegin { leaf, source } => {
                write!(
                    formatter,
                    "PlayAnimation leaf {leaf:?} Begin failed: {source}"
                )
            }
            Self::PlayAnimationUpdate { leaf, source } => {
                write!(
                    formatter,
                    "PlayAnimation leaf {leaf:?} Update failed: {source}"
                )
            }
            Self::PlayRemappedAnimationBegin { leaf, source } => write!(
                formatter,
                "PlayRemappedAnimation leaf {leaf:?} Begin failed: {source}"
            ),
            Self::PlayRemappedAnimationUpdate { leaf, source } => write!(
                formatter,
                "PlayRemappedAnimation leaf {leaf:?} Update failed: {source}"
            ),
        }
    }
}

impl<HE, WE, AE, PE, RE> Error for MotionGraphDynamicBackendError<HE, WE, AE, PE, RE>
where
    HE: Error + 'static,
    WE: Error + 'static,
    AE: Error + 'static,
    PE: Error + 'static,
    RE: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Identity(error) => error,
            Self::HasIntent { source, .. } => source,
            Self::WillExpire { source, .. } => source,
            Self::PlayAnimationCreate { source, .. } => source,
            Self::PlayAnimationBegin { source, .. } | Self::PlayAnimationUpdate { source, .. } => {
                source
            }
            Self::PlayRemappedAnimationBegin { source, .. }
            | Self::PlayRemappedAnimationUpdate { source, .. } => source,
        })
    }
}

/// A generic dynamic-controller backend using one facade during this borrow.
///
/// The separate allocator reflects the recovered `CreateInstance` boundary;
/// it does not participate in conditions or `Begin`/`Update`/`End` callbacks.
pub struct BorrowedMotionGraphBackend<'facade, 'allocator, E, A> {
    facade: &'facade mut E,
    allocator: &'allocator mut A,
}

impl<'facade, 'allocator, E, A> BorrowedMotionGraphBackend<'facade, 'allocator, E, A> {
    #[must_use]
    pub const fn new(facade: &'facade mut E, allocator: &'allocator mut A) -> Self {
        Self { facade, allocator }
    }
}

#[derive(Debug)]
enum SupportedCondition<'a> {
    HasIntent(&'a mut crate::motion_graph_conditions::HasIntentCondition),
    WillExpire(&'a mut crate::motion_graph_conditions::WillExpireCondition),
}

fn identity_error<O>(
    site: MotionGraphDynamicCallbackSite,
    leaf: LeafId,
    registered_name: &[u8],
    payload: &MotionGraphLeafPayload<O>,
) -> MotionGraphDynamicIdentityError {
    identity_error_for_variant(site, leaf, registered_name, payload.variant())
}

fn identity_error_for_variant(
    site: MotionGraphDynamicCallbackSite,
    leaf: LeafId,
    registered_name: &[u8],
    actual: MotionGraphLeafVariant,
) -> MotionGraphDynamicIdentityError {
    let callback_kind = match site {
        MotionGraphDynamicCallbackSite::Condition => LeafKind::Condition,
        MotionGraphDynamicCallbackSite::CreateInstance
        | MotionGraphDynamicCallbackSite::Begin
        | MotionGraphDynamicCallbackSite::Update
        | MotionGraphDynamicCallbackSite::End => LeafKind::Behaviour,
        MotionGraphDynamicCallbackSite::TransitionHook(_) => LeafKind::Hook,
    };
    let expected = match registered_name {
        HAS_INTENT_REGISTERED_NAME => {
            Some((LeafKind::Condition, MotionGraphLeafVariant::HasIntent))
        }
        WILL_EXPIRE_REGISTERED_NAME => {
            Some((LeafKind::Condition, MotionGraphLeafVariant::WillExpire))
        }
        PLAY_ANIMATION_REGISTERED_NAME => {
            Some((LeafKind::Behaviour, MotionGraphLeafVariant::PlayAnimation))
        }
        PLAY_REMAPPED_ANIMATION_REGISTERED_NAME => Some((
            LeafKind::Behaviour,
            MotionGraphLeafVariant::PlayRemappedAnimation,
        )),
        _ => None,
    };
    let reason = match expected {
        Some((_, expected)) if actual != expected => {
            MotionGraphDynamicIdentityReason::PayloadVariantMismatch { expected, actual }
        }
        Some((expected, _)) if callback_kind != expected => {
            MotionGraphDynamicIdentityReason::CallbackKindMismatch {
                expected,
                actual: callback_kind,
            }
        }
        None if actual == MotionGraphLeafVariant::RegisteredOpaque => {
            MotionGraphDynamicIdentityReason::RegisteredOpaque
        }
        Some(_) | None => MotionGraphDynamicIdentityReason::UnsupportedIdentity,
    };
    MotionGraphDynamicIdentityError {
        site,
        leaf,
        registered_name: registered_name.to_vec(),
        actual,
        reason,
    }
}

pub(crate) fn validate_motion_graph_callback_identity<O>(
    site: MotionGraphDynamicCallbackSite,
    leaf: LeafId,
    registered_name: &[u8],
    payload: &MotionGraphLeafPayload<O>,
) -> Result<(), MotionGraphDynamicIdentityError> {
    let actual = payload.variant();
    let supported = match site {
        MotionGraphDynamicCallbackSite::Condition => matches!(
            (registered_name, actual),
            (
                HAS_INTENT_REGISTERED_NAME,
                MotionGraphLeafVariant::HasIntent
            ) | (
                WILL_EXPIRE_REGISTERED_NAME,
                MotionGraphLeafVariant::WillExpire
            )
        ),
        MotionGraphDynamicCallbackSite::CreateInstance
        | MotionGraphDynamicCallbackSite::Begin
        | MotionGraphDynamicCallbackSite::Update
        | MotionGraphDynamicCallbackSite::End => {
            matches!(
                (registered_name, actual),
                (
                    PLAY_ANIMATION_REGISTERED_NAME,
                    MotionGraphLeafVariant::PlayAnimation
                ) | (
                    PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                    MotionGraphLeafVariant::PlayRemappedAnimation
                )
            )
        }
        MotionGraphDynamicCallbackSite::TransitionHook(_) => false,
    };
    if supported {
        Ok(())
    } else {
        Err(identity_error_for_variant(
            site,
            leaf,
            registered_name,
            actual,
        ))
    }
}

fn supported_condition<'a, O>(
    leaf: LeafId,
    registered_name: &[u8],
    payload: &'a mut MotionGraphLeafPayload<O>,
) -> Result<SupportedCondition<'a>, MotionGraphDynamicIdentityError> {
    validate_motion_graph_callback_identity(
        MotionGraphDynamicCallbackSite::Condition,
        leaf,
        registered_name,
        payload,
    )?;
    match (registered_name, &mut *payload) {
        (HAS_INTENT_REGISTERED_NAME, MotionGraphLeafPayload::HasIntent(condition)) => {
            Ok(SupportedCondition::HasIntent(condition))
        }
        (WILL_EXPIRE_REGISTERED_NAME, MotionGraphLeafPayload::WillExpire(condition)) => {
            Ok(SupportedCondition::WillExpire(condition))
        }
        _ => unreachable!("validated MotionGraph condition identity"),
    }
}

fn supported_play_animation<'a, O>(
    site: MotionGraphDynamicCallbackSite,
    leaf: LeafId,
    registered_name: &[u8],
    payload: &'a mut MotionGraphLeafPayload<O>,
) -> Result<&'a mut PlayAnimationBehaviour, MotionGraphDynamicIdentityError> {
    validate_motion_graph_callback_identity(site, leaf, registered_name, payload)?;
    if let MotionGraphLeafPayload::PlayAnimation(behaviour) = payload {
        return Ok(behaviour);
    }
    unreachable!("validated PlayAnimation identity")
}

fn supported_play_remapped_animation<'a, O>(
    site: MotionGraphDynamicCallbackSite,
    leaf: LeafId,
    registered_name: &[u8],
    payload: &'a mut MotionGraphLeafPayload<O>,
) -> Result<&'a mut PlayRemappedAnimationBehaviour, MotionGraphDynamicIdentityError> {
    validate_motion_graph_callback_identity(site, leaf, registered_name, payload)?;
    if let MotionGraphLeafPayload::PlayRemappedAnimation(behaviour) = payload {
        return Ok(behaviour);
    }
    Err(identity_error(site, leaf, registered_name, payload))
}

fn end_play_animation<O>(
    leaf: LeafId,
    registered_name: &[u8],
    payload: &mut MotionGraphLeafPayload<O>,
    _instance: Option<&mut PlayAnimationRuntimeInstance>,
) -> Result<(), MotionGraphDynamicIdentityError> {
    validate_motion_graph_callback_identity(
        MotionGraphDynamicCallbackSite::End,
        leaf,
        registered_name,
        payload,
    )?;
    match payload {
        MotionGraphLeafPayload::PlayAnimation(behaviour) => behaviour.end_runtime(),
        MotionGraphLeafPayload::PlayRemappedAnimation(behaviour) => behaviour.end(),
        _ => unreachable!("validated MotionGraph Behaviour identity"),
    }
    Ok(())
}

fn unsupported_transition_hook<O>(
    transition: TransitionId,
    hook: LeafId,
    registered_name: &[u8],
    payload: &MotionGraphLeafPayload<O>,
) -> MotionGraphDynamicIdentityError {
    identity_error(
        MotionGraphDynamicCallbackSite::TransitionHook(transition),
        hook,
        registered_name,
        payload,
    )
}

impl<O, E, A> DynamicStateGraphBackend<MotionGraphLeafPayload<O>, ()>
    for BorrowedMotionGraphBackend<'_, '_, E, A>
where
    E: HasIntentCollaborator
        + WillExpireCollaborator
        + PlayAnimationLifecycleBackend
        + PlayRemappedAnimationCollaborator,
    A: PlayAnimationInstanceAllocator,
{
    type Instance = PlayAnimationRuntimeInstance;
    type Error = MotionGraphDynamicBackendError<
        <E as HasIntentCollaborator>::Error,
        <E as WillExpireCollaborator>::Error,
        <A as PlayAnimationInstanceAllocator>::Error,
        <E as PlayAnimationLifecycleBackend>::Error,
        <E as PlayRemappedAnimationCollaborator>::Error,
    >;

    fn condition_activation(
        &mut self,
        request: DynamicConditionActivationRequest<'_, MotionGraphLeafPayload<O>, ()>,
    ) -> Result<bool, Self::Error> {
        let leaf = request.leaf;
        match supported_condition(request.leaf, request.registered_name, request.payload)
            .map_err(MotionGraphDynamicBackendError::Identity)?
        {
            SupportedCondition::HasIntent(condition) => condition
                .activate(&mut *self.facade)
                .map_err(|source| MotionGraphDynamicBackendError::HasIntent { leaf, source }),
            SupportedCondition::WillExpire(condition) => condition
                .activate(&mut *self.facade)
                .map_err(|source| MotionGraphDynamicBackendError::WillExpire { leaf, source }),
        }
    }

    fn create_instance(
        &mut self,
        request: DynamicBehaviourCreateRequest<'_, MotionGraphLeafPayload<O>>,
    ) -> Result<Option<Self::Instance>, Self::Error> {
        let leaf = request.leaf;
        validate_motion_graph_callback_identity(
            MotionGraphDynamicCallbackSite::CreateInstance,
            request.leaf,
            request.registered_name,
            request.payload,
        )
        .map_err(MotionGraphDynamicBackendError::Identity)?;
        match request.payload {
            MotionGraphLeafPayload::PlayAnimation(behaviour) => behaviour
                .create_runtime_instance(&mut *self.allocator)
                .map_err(
                    |source| MotionGraphDynamicBackendError::PlayAnimationCreate { leaf, source },
                ),
            MotionGraphLeafPayload::PlayRemappedAnimation(_) => Ok(None),
            _ => unreachable!("validated MotionGraph Behaviour identity"),
        }
    }

    fn begin(
        &mut self,
        request: DynamicBehaviourCallbackRequest<'_, MotionGraphLeafPayload<O>, (), Self::Instance>,
    ) -> Result<(), Self::Error> {
        let leaf = request.leaf;
        match request.payload {
            MotionGraphLeafPayload::PlayAnimation(_) => {
                let behaviour = supported_play_animation(
                    MotionGraphDynamicCallbackSite::Begin,
                    request.leaf,
                    request.registered_name,
                    request.payload,
                )
                .map_err(MotionGraphDynamicBackendError::Identity)?;
                behaviour
                    .begin_runtime(request.instance, &mut *self.facade)
                    .map_err(
                        |source| MotionGraphDynamicBackendError::PlayAnimationBegin {
                            leaf,
                            source,
                        },
                    )
            }
            MotionGraphLeafPayload::PlayRemappedAnimation(_) => {
                let behaviour = supported_play_remapped_animation(
                    MotionGraphDynamicCallbackSite::Begin,
                    request.leaf,
                    request.registered_name,
                    request.payload,
                )
                .map_err(MotionGraphDynamicBackendError::Identity)?;
                behaviour.begin(&mut *self.facade).map_err(|source| {
                    MotionGraphDynamicBackendError::PlayRemappedAnimationBegin { leaf, source }
                })
            }
            _ => Err(MotionGraphDynamicBackendError::Identity(identity_error(
                MotionGraphDynamicCallbackSite::Begin,
                request.leaf,
                request.registered_name,
                request.payload,
            ))),
        }
    }

    fn update(
        &mut self,
        request: DynamicBehaviourCallbackRequest<'_, MotionGraphLeafPayload<O>, (), Self::Instance>,
    ) -> Result<(), Self::Error> {
        let leaf = request.leaf;
        match request.payload {
            MotionGraphLeafPayload::PlayAnimation(_) => {
                let behaviour = supported_play_animation(
                    MotionGraphDynamicCallbackSite::Update,
                    request.leaf,
                    request.registered_name,
                    request.payload,
                )
                .map_err(MotionGraphDynamicBackendError::Identity)?;
                behaviour
                    .update_runtime(request.instance, &mut *self.facade)
                    .map_err(
                        |source| MotionGraphDynamicBackendError::PlayAnimationUpdate {
                            leaf,
                            source,
                        },
                    )
            }
            MotionGraphLeafPayload::PlayRemappedAnimation(_) => {
                let behaviour = supported_play_remapped_animation(
                    MotionGraphDynamicCallbackSite::Update,
                    request.leaf,
                    request.registered_name,
                    request.payload,
                )
                .map_err(MotionGraphDynamicBackendError::Identity)?;
                behaviour.update(&mut *self.facade).map_err(|source| {
                    MotionGraphDynamicBackendError::PlayRemappedAnimationUpdate { leaf, source }
                })
            }
            _ => Err(MotionGraphDynamicBackendError::Identity(identity_error(
                MotionGraphDynamicCallbackSite::Update,
                request.leaf,
                request.registered_name,
                request.payload,
            ))),
        }
    }

    fn end(
        &mut self,
        request: DynamicBehaviourCallbackRequest<'_, MotionGraphLeafPayload<O>, (), Self::Instance>,
    ) -> Result<(), Self::Error> {
        end_play_animation(
            request.leaf,
            request.registered_name,
            request.payload,
            request.instance,
        )
        .map_err(MotionGraphDynamicBackendError::Identity)
    }

    fn on_transition(
        &mut self,
        request: DynamicTransitionHookRequest<'_, MotionGraphLeafPayload<O>, ()>,
    ) -> Result<(), Self::Error> {
        Err(MotionGraphDynamicBackendError::Identity(
            unsupported_transition_hook(
                request.transition,
                request.hook,
                request.registered_name,
                request.payload,
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dynamic_state_graph_controller::{
            DynamicStateGraphController, DynamicStateGraphControllerError,
            DynamicStateGraphLifecycleError,
        },
        motion_graph_conditions::{AnimProperty, HasIntentCondition, WillExpireCondition},
        play_animation::{FastString30, FastString36},
        play_animation_lifecycle::{
            AnimationDispatch, PhysicalOutputObservation, PlayAnimationBackendOperation,
            PlayAnimationInterfaceKind, PlayAnimationInterfaceResolution,
            RawAnimationTransitionParams, TreeAttributeSample,
        },
        state_graph_runtime::{
            FactoryMissPolicy, LeafCreateRequest, LeafKind, StateGraphLeafFactory,
            StateGraphRuntime,
        },
    };
    use skate2_formats::state_graph::{
        StateGraphBinaryAttribute, StateGraphBinaryElement, state_graph_string_hash32,
    };
    use std::{convert::Infallible, fmt};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct SpyError(&'static str);

    impl fmt::Display for SpyError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl Error for SpyError {}

    #[derive(Default)]
    struct Facade {
        calls: Vec<String>,
        fail_dispatch: bool,
        fail_has_cached: bool,
        fail_remapped: Option<&'static str>,
    }

    impl Facade {
        fn call(&mut self, value: impl Into<String>) {
            self.calls.push(value.into());
        }
    }

    impl HasIntentCollaborator for Facade {
        type Error = SpyError;
        type MotionGraph = u8;
        type Intents = u8;

        fn cached_motion_graph(&mut self) -> Result<Option<u8>, SpyError> {
            self.call("has.cached");
            if self.fail_has_cached {
                Err(SpyError("cached motion graph"))
            } else {
                Ok(Some(1))
            }
        }

        fn find_motion_graph(&mut self) -> Result<Option<u8>, SpyError> {
            self.call("has.find");
            Ok(Some(1))
        }

        fn motion_graph_intents(&mut self, _: &u8) -> Result<u8, SpyError> {
            self.call("has.intents");
            Ok(2)
        }

        fn lookup_intent(&mut self, _: &u8, name: &[u8]) -> Result<Option<f32>, SpyError> {
            self.call(format!("has.lookup:{}", String::from_utf8_lossy(name)));
            Ok(Some(1.0))
        }
    }

    impl WillExpireCollaborator for Facade {
        type Error = SpyError;
        type AnimInfo = u8;
        type Animatable = u8;

        fn find_anim_info(&mut self) -> Result<Option<u8>, SpyError> {
            self.call("will.anim_info");
            Ok(Some(3))
        }

        fn resolve_special_tag(&mut self, _: &u8, tag: &[u8]) -> Result<f32, SpyError> {
            self.call(format!("will.tag:{}", String::from_utf8_lossy(tag)));
            Ok(0.5)
        }

        fn cached_animatable(&mut self) -> Result<Option<u8>, SpyError> {
            self.call("will.cached");
            Ok(Some(4))
        }

        fn find_animatable(&mut self) -> Result<Option<u8>, SpyError> {
            self.call("will.find");
            Ok(Some(4))
        }

        fn transitioning(&mut self, _: &u8) -> Result<bool, SpyError> {
            self.call("will.transitioning");
            Ok(false)
        }

        fn anim_property(&mut self, _: &u8) -> Result<Option<AnimProperty>, SpyError> {
            self.call("will.property");
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
            kind: PlayAnimationInterfaceKind,
        ) -> Result<PlayAnimationInterfaceResolution, SpyError> {
            self.call(format!("play.resolve:{kind:?}"));
            Ok(PlayAnimationInterfaceResolution::Resolved)
        }

        fn motion_intent(&mut self, _: FastString36) -> Result<Option<u32>, SpyError> {
            self.call("play.motion_intent");
            Ok(None)
        }

        fn filtered_motion_intent(&mut self, _: FastString36) -> Result<Option<u32>, SpyError> {
            self.call("play.filtered_intent");
            Ok(None)
        }

        fn tree_attribute(
            &mut self,
            _: FastString30,
        ) -> Result<Option<TreeAttributeSample>, SpyError> {
            self.call("play.tree_attribute");
            Ok(None)
        }

        fn add_settable_attribute(
            &mut self,
            _: FastString30,
            _: u32,
            _: bool,
            _: i32,
        ) -> Result<(), SpyError> {
            self.call("play.add_settable");
            Ok(())
        }

        fn selected_pro_skater(&mut self) -> Result<FastString30, SpyError> {
            self.call("play.pro_skater");
            Ok(FastString30 { words: [0; 5] })
        }

        fn set_construction_attribute(
            &mut self,
            _: FastString30,
            _: FastString30,
        ) -> Result<(), SpyError> {
            self.call("play.construction_attribute");
            Ok(())
        }

        fn next_transition_override(
            &mut self,
        ) -> Result<Option<RawAnimationTransitionParams>, SpyError> {
            self.call("play.next_override");
            Ok(Some(RawAnimationTransitionParams {
                transition_type_raw: 0,
                blend_length_bits: 0xdead_beef,
                placement_raw: 0xfeed_beef,
                flags_raw: 0x1234_5678,
            }))
        }

        fn clear_transition_override(&mut self) -> Result<(), SpyError> {
            self.call("play.clear_override");
            Ok(())
        }

        fn is_switch(&mut self) -> Result<bool, SpyError> {
            self.call("play.switch");
            Ok(false)
        }

        fn is_mirrored(&mut self) -> Result<bool, SpyError> {
            self.call("play.mirrored");
            Ok(false)
        }

        fn physical_output(&mut self) -> Result<PhysicalOutputObservation, SpyError> {
            self.call("play.physical");
            Ok(PhysicalOutputObservation::HoldingSkateboardRaw(1))
        }

        fn dispatch(&mut self, _: AnimationDispatch) -> Result<u8, SpyError> {
            self.call("play.dispatch");
            if self.fail_dispatch {
                Err(SpyError("dispatch"))
            } else {
                Ok(1)
            }
        }
    }

    impl PlayRemappedAnimationCollaborator for Facade {
        type Error = SpyError;

        fn write_mirror_override_raw(&mut self, value: u8) -> Result<(), SpyError> {
            self.call(format!("remapped.mirror:{value}"));
            if self.fail_remapped == Some("mirror") {
                Err(SpyError("mirror"))
            } else {
                Ok(())
            }
        }

        fn setup_remapped_animation(
            &mut self,
            request: crate::play_remapped_animation::RemappedAnimationSetup<'_>,
        ) -> Result<(), SpyError> {
            self.call(format!(
                "remapped.setup:{}",
                String::from_utf8_lossy(request.animation_name)
            ));
            if self.fail_remapped == Some("setup") {
                Err(SpyError("setup"))
            } else {
                Ok(())
            }
        }

        fn update_remapped_animation(
            &mut self,
            request: crate::play_remapped_animation::RemappedAnimationUpdate<'_>,
        ) -> Result<(), SpyError> {
            self.call(format!(
                "remapped.update:{}",
                String::from_utf8_lossy(request.animation_name)
            ));
            if self.fail_remapped == Some("update") {
                Err(SpyError("update"))
            } else {
                Ok(())
            }
        }
    }

    #[derive(Default)]
    struct Allocator {
        calls: usize,
        allocated: bool,
        failure: Option<SpyError>,
    }

    impl PlayAnimationInstanceAllocator for Allocator {
        type Error = SpyError;

        fn allocate(
            &mut self,
            size: usize,
            name: &[u8],
            alignment: usize,
        ) -> Result<bool, SpyError> {
            assert_eq!((size, name, alignment), (0x0c, b"unknown".as_slice(), 0x10));
            self.calls += 1;
            self.failure.map_or(Ok(self.allocated), Err)
        }
    }

    fn attr(
        key: &[u8],
        text: &[u8],
        float_bits: u32,
        boolean_raw: u8,
    ) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits,
            boolean_raw,
        }
    }

    fn has_intent() -> MotionGraphLeafPayload<()> {
        MotionGraphLeafPayload::HasIntent(
            HasIntentCondition::from_attributes(&[attr(b"intent", b"Push", 0, 0)]).unwrap(),
        )
    }

    fn will_expire(with_tag: bool) -> MotionGraphLeafPayload<()> {
        let mut attributes = Vec::new();
        if with_tag {
            attributes.push(attr(b"InTimeTag", b"Exit", 0, 0));
        }
        MotionGraphLeafPayload::WillExpire(
            WillExpireCondition::from_attributes(&attributes).unwrap(),
        )
    }

    fn play_animation() -> MotionGraphLeafPayload<()> {
        MotionGraphLeafPayload::PlayAnimation(
            PlayAnimationBehaviour::from_attributes(&[attr(b"anim", b"Idle", 0, 0)]).unwrap(),
        )
    }

    #[test]
    fn exact_identity_helpers_accept_only_the_four_callback_triples() {
        assert!(matches!(
            supported_condition(LeafId(1), HAS_INTENT_REGISTERED_NAME, &mut has_intent()),
            Ok(SupportedCondition::HasIntent(_))
        ));
        assert!(matches!(
            supported_condition(
                LeafId(2),
                WILL_EXPIRE_REGISTERED_NAME,
                &mut will_expire(false)
            ),
            Ok(SupportedCondition::WillExpire(_))
        ));
        assert!(
            supported_play_remapped_animation(
                MotionGraphDynamicCallbackSite::Begin,
                LeafId(4),
                PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                &mut MotionGraphLeafPayload::<()>::PlayRemappedAnimation(
                    PlayRemappedAnimationBehaviour::from_attributes(&[attr(
                        b"anim", b"Idle", 0, 0
                    )])
                    .unwrap()
                )
            )
            .is_ok()
        );
        assert!(
            supported_play_animation(
                MotionGraphDynamicCallbackSite::Begin,
                LeafId(3),
                PLAY_ANIMATION_REGISTERED_NAME,
                &mut play_animation()
            )
            .is_ok()
        );
    }

    #[test]
    fn identity_helpers_reject_variant_name_and_callback_kind_mismatches() {
        for (name, mut payload, expected) in [
            (
                HAS_INTENT_REGISTERED_NAME,
                will_expire(false),
                MotionGraphDynamicIdentityReason::PayloadVariantMismatch {
                    expected: MotionGraphLeafVariant::HasIntent,
                    actual: MotionGraphLeafVariant::WillExpire,
                },
            ),
            (
                b"hasintent".as_slice(),
                has_intent(),
                MotionGraphDynamicIdentityReason::UnsupportedIdentity,
            ),
            (
                b"HasIntent\0junk".as_slice(),
                has_intent(),
                MotionGraphDynamicIdentityReason::UnsupportedIdentity,
            ),
            (
                b"Opaque".as_slice(),
                MotionGraphLeafPayload::RegisteredOpaque(()),
                MotionGraphDynamicIdentityReason::RegisteredOpaque,
            ),
            (
                PLAY_ANIMATION_REGISTERED_NAME,
                play_animation(),
                MotionGraphDynamicIdentityReason::CallbackKindMismatch {
                    expected: LeafKind::Behaviour,
                    actual: LeafKind::Condition,
                },
            ),
        ] {
            let error = supported_condition(LeafId(9), name, &mut payload).unwrap_err();
            assert_eq!(error.registered_name, name);
            assert_eq!(error.reason, expected);
        }

        for (name, mut payload, expected) in [
            (
                PLAY_ANIMATION_REGISTERED_NAME,
                has_intent(),
                MotionGraphDynamicIdentityReason::PayloadVariantMismatch {
                    expected: MotionGraphLeafVariant::PlayAnimation,
                    actual: MotionGraphLeafVariant::HasIntent,
                },
            ),
            (
                b"playanimation".as_slice(),
                play_animation(),
                MotionGraphDynamicIdentityReason::UnsupportedIdentity,
            ),
            (
                b"PlayAnimation\0".as_slice(),
                play_animation(),
                MotionGraphDynamicIdentityReason::UnsupportedIdentity,
            ),
            (
                b"Opaque".as_slice(),
                MotionGraphLeafPayload::RegisteredOpaque(()),
                MotionGraphDynamicIdentityReason::RegisteredOpaque,
            ),
            (
                HAS_INTENT_REGISTERED_NAME,
                has_intent(),
                MotionGraphDynamicIdentityReason::CallbackKindMismatch {
                    expected: LeafKind::Condition,
                    actual: LeafKind::Behaviour,
                },
            ),
        ] {
            let error = supported_play_animation(
                MotionGraphDynamicCallbackSite::Update,
                LeafId(10),
                name,
                &mut payload,
            )
            .unwrap_err();
            assert_eq!(error.reason, expected);
        }
    }

    #[test]
    fn identity_rejection_is_effect_free_and_hook_is_always_unsupported() {
        let mut facade = Facade::default();
        let mut allocator = Allocator {
            allocated: true,
            ..Allocator::default()
        };
        let mut payload = has_intent();
        {
            let mut backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
            let error = backend
                .create_instance(DynamicBehaviourCreateRequest {
                    leaf: LeafId(7),
                    registered_name: PLAY_ANIMATION_REGISTERED_NAME,
                    payload: &mut payload,
                })
                .unwrap_err();
            assert!(matches!(error, MotionGraphDynamicBackendError::Identity(_)));
        }
        assert!(facade.calls.is_empty());
        assert_eq!(allocator.calls, 0);

        let hook = unsupported_transition_hook(
            TransitionId(4),
            LeafId(8),
            b"AnyHook",
            &MotionGraphLeafPayload::RegisteredOpaque(()),
        );
        assert_eq!(
            hook.site,
            MotionGraphDynamicCallbackSite::TransitionHook(TransitionId(4))
        );
        assert_eq!(
            hook.reason,
            MotionGraphDynamicIdentityReason::RegisteredOpaque
        );
        let wrong_kind = unsupported_transition_hook(
            TransitionId(5),
            LeafId(9),
            HAS_INTENT_REGISTERED_NAME,
            &has_intent(),
        );
        assert_eq!(
            wrong_kind.reason,
            MotionGraphDynamicIdentityReason::CallbackKindMismatch {
                expected: LeafKind::Condition,
                actual: LeafKind::Hook,
            }
        );
        assert!(facade.calls.is_empty());
        assert_eq!(allocator.calls, 0);
    }

    #[test]
    fn create_returns_distinct_latches_and_allocation_false_stays_none() {
        let mut facade = Facade::default();
        let mut allocator = Allocator {
            allocated: true,
            ..Allocator::default()
        };
        let mut payload = play_animation();
        let (mut first, second) = {
            let mut backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
            let first = backend
                .create_instance(DynamicBehaviourCreateRequest {
                    leaf: LeafId(0),
                    registered_name: PLAY_ANIMATION_REGISTERED_NAME,
                    payload: &mut payload,
                })
                .unwrap()
                .unwrap();
            let second = backend
                .create_instance(DynamicBehaviourCreateRequest {
                    leaf: LeafId(0),
                    registered_name: PLAY_ANIMATION_REGISTERED_NAME,
                    payload: &mut payload,
                })
                .unwrap()
                .unwrap();
            (first, second)
        };
        assert_eq!(allocator.calls, 2);
        assert!(first.first_update());
        assert!(second.first_update());
        let MotionGraphLeafPayload::PlayAnimation(behaviour) = &payload else {
            unreachable!()
        };
        behaviour
            .update_runtime(Some(&mut first), &mut facade)
            .unwrap();
        assert!(!first.first_update());
        assert!(second.first_update());

        allocator.allocated = false;
        let mut backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
        assert!(
            backend
                .create_instance(DynamicBehaviourCreateRequest {
                    leaf: LeafId(0),
                    registered_name: PLAY_ANIMATION_REGISTERED_NAME,
                    payload: &mut payload,
                })
                .unwrap()
                .is_none()
        );
        assert_eq!(allocator.calls, 3);
    }

    #[test]
    fn end_accepts_none_and_performs_no_facade_or_allocator_effect() {
        let mut facade = Facade::default();
        let mut allocator = Allocator::default();
        let mut payload = play_animation();
        end_play_animation(
            LeafId(2),
            PLAY_ANIMATION_REGISTERED_NAME,
            &mut payload,
            None,
        )
        .unwrap();
        assert!(facade.calls.is_empty());
        assert_eq!(allocator.calls, 0);
        let _backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
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
                ],
                behaviours: vec![
                    PLAY_ANIMATION_REGISTERED_NAME.to_vec(),
                    PLAY_REMAPPED_ANIMATION_REGISTERED_NAME.to_vec(),
                ],
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

        fn create(&mut self, request: LeafCreateRequest<'_>) -> Result<Self::Payload, Self::Error> {
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
                PLAY_REMAPPED_ANIMATION_REGISTERED_NAME => {
                    MotionGraphLeafPayload::PlayRemappedAnimation(
                        PlayRemappedAnimationBehaviour::from_attributes(request.attributes)
                            .unwrap(),
                    )
                }
                _ => unreachable!(),
            })
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

    fn build_graph(
        condition: Option<(&[u8], Vec<StateGraphBinaryAttribute>)>,
        behaviour: bool,
    ) -> StateGraphRuntime<MotionGraphLeafPayload<()>> {
        let mut children = Vec::new();
        if let Some((name, mut attributes)) = condition {
            attributes.insert(0, attr(b"name", name, 0, 0));
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
                    attr(b"name", PLAY_ANIMATION_REGISTERED_NAME, 0, 0),
                    attr(b"anim", b"Idle", 0, 0),
                ],
                Vec::new(),
            ));
        }
        let root = element(b"state", vec![attr(b"name", b"Root", 0, 0)], children);
        let mut factory = Factory::default();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        graph.process_after_build(&mut factory).unwrap();
        graph
    }

    #[test]
    fn tiny_controller_preserves_has_intent_play_lifecycle_and_effect_free_end() {
        let graph = build_graph(
            Some((
                HAS_INTENT_REGISTERED_NAME,
                vec![attr(b"intent", b"Push", 0, 0)],
            )),
            true,
        );
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        let mut facade = Facade::default();
        let mut allocator = Allocator {
            allocated: true,
            ..Allocator::default()
        };
        {
            let mut backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
            controller.update(&mut backend, 1.0 / 60.0).unwrap();
            let active = controller.active_behaviours().collect::<Vec<_>>();
            assert_eq!(active.len(), 1);
            assert!(!active[0].1.unwrap().first_update());
            controller.end_all_behaviours(&mut backend).unwrap();
        }
        assert_eq!(allocator.calls, 1);
        assert_eq!(
            facade.calls,
            [
                "has.cached",
                "has.find",
                "has.intents",
                "has.lookup:Push",
                "play.resolve:Physical",
                "play.resolve:Animatable",
                "play.resolve:SkaterAnim",
                "play.resolve:MotionGraph",
                "play.resolve:Animatable",
                "play.resolve:MotionGraph",
                "play.resolve:SkaterMotionGraph",
                "play.pro_skater",
                "play.construction_attribute",
                "play.next_override",
                "play.clear_override",
                "play.dispatch",
            ]
        );
        assert_eq!(controller.active_behaviours().len(), 0);
    }

    fn remapped_graph(mirror: Option<u8>) -> StateGraphRuntime<MotionGraphLeafPayload<()>> {
        let mut attributes = vec![
            attr(b"name", PLAY_REMAPPED_ANIMATION_REGISTERED_NAME, 0, 0),
            attr(b"anim", b"FACE_HAND_TEST", 0, 0),
        ];
        if let Some(raw) = mirror {
            attributes.push(attr(b"mirrorOverride", b"", 0, raw));
        }
        let root = element(
            b"state",
            vec![attr(b"name", b"Motion", 0, 0)],
            vec![element(
                b"state",
                vec![attr(b"name", b"Locomotion", 0, 0)],
                vec![element(b"behaviour", attributes, vec![])],
            )],
        );
        let mut factory = Factory::default();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        graph.process_after_build(&mut factory).unwrap();
        graph
    }

    type TestBackendError =
        MotionGraphDynamicBackendError<SpyError, SpyError, SpyError, SpyError, SpyError>;

    #[derive(Debug, PartialEq)]
    enum RemappedTrace {
        CreateNone(LeafId),
        Callback(
            MotionGraphDynamicCallbackSite,
            LeafId,
            Option<crate::state_graph_runtime::StateId>,
            Option<crate::state_graph_runtime::StateId>,
            [u32; 2],
            bool,
        ),
    }

    struct TracedRemappedBackend<'a> {
        inner: BorrowedMotionGraphBackend<'a, 'a, Facade, Allocator>,
        trace: Vec<RemappedTrace>,
        wrong_identity_at: Option<MotionGraphDynamicCallbackSite>,
    }

    impl TracedRemappedBackend<'_> {
        fn callback(
            &mut self,
            site: MotionGraphDynamicCallbackSite,
            request: DynamicBehaviourCallbackRequest<
                '_,
                MotionGraphLeafPayload<()>,
                (),
                PlayAnimationRuntimeInstance,
            >,
        ) -> Result<(), TestBackendError> {
            use crate::state_graph_runtime::StateId;
            self.trace.push(RemappedTrace::Callback(
                site,
                request.leaf,
                request.controller.current,
                request.controller.last,
                [
                    request.controller.time_in_state(StateId(0)).to_bits(),
                    request.controller.time_in_state(StateId(1)).to_bits(),
                ],
                request.instance.is_none(),
            ));
            let request = DynamicBehaviourCallbackRequest {
                registered_name: if self.wrong_identity_at == Some(site) {
                    PLAY_ANIMATION_REGISTERED_NAME
                } else {
                    request.registered_name
                },
                ..request
            };
            match site {
                MotionGraphDynamicCallbackSite::Begin => self.inner.begin(request),
                MotionGraphDynamicCallbackSite::Update => self.inner.update(request),
                MotionGraphDynamicCallbackSite::End => self.inner.end(request),
                _ => unreachable!(),
            }
        }
    }

    impl DynamicStateGraphBackend<MotionGraphLeafPayload<()>, ()> for TracedRemappedBackend<'_> {
        type Instance = PlayAnimationRuntimeInstance;
        type Error = TestBackendError;

        fn condition_activation(
            &mut self,
            _: DynamicConditionActivationRequest<'_, MotionGraphLeafPayload<()>, ()>,
        ) -> Result<bool, Self::Error> {
            panic!("fixture has no conditions")
        }

        fn create_instance(
            &mut self,
            request: DynamicBehaviourCreateRequest<'_, MotionGraphLeafPayload<()>>,
        ) -> Result<Option<Self::Instance>, Self::Error> {
            let leaf = request.leaf;
            let request = DynamicBehaviourCreateRequest {
                registered_name: if self.wrong_identity_at
                    == Some(MotionGraphDynamicCallbackSite::CreateInstance)
                {
                    PLAY_ANIMATION_REGISTERED_NAME
                } else {
                    request.registered_name
                },
                ..request
            };
            let instance = self.inner.create_instance(request)?;
            assert!(instance.is_none());
            self.trace.push(RemappedTrace::CreateNone(leaf));
            Ok(instance)
        }

        fn begin(
            &mut self,
            request: DynamicBehaviourCallbackRequest<
                '_,
                MotionGraphLeafPayload<()>,
                (),
                Self::Instance,
            >,
        ) -> Result<(), Self::Error> {
            self.callback(MotionGraphDynamicCallbackSite::Begin, request)
        }
        fn update(
            &mut self,
            request: DynamicBehaviourCallbackRequest<
                '_,
                MotionGraphLeafPayload<()>,
                (),
                Self::Instance,
            >,
        ) -> Result<(), Self::Error> {
            self.callback(MotionGraphDynamicCallbackSite::Update, request)
        }
        fn end(
            &mut self,
            request: DynamicBehaviourCallbackRequest<
                '_,
                MotionGraphLeafPayload<()>,
                (),
                Self::Instance,
            >,
        ) -> Result<(), Self::Error> {
            self.callback(MotionGraphDynamicCallbackSite::End, request)
        }
        fn on_transition(
            &mut self,
            _: DynamicTransitionHookRequest<'_, MotionGraphLeafPayload<()>, ()>,
        ) -> Result<(), Self::Error> {
            panic!("fixture has no hooks")
        }
    }

    #[test]
    fn remapped_two_ticks_retain_null_instance_exact_timers_and_noop_end() {
        use crate::state_graph_runtime::StateId;
        use MotionGraphDynamicCallbackSite::{Begin, End, Update};
        let mut controller = DynamicStateGraphController::new(remapped_graph(None), ()).unwrap();
        let mut facade = Facade::default();
        let mut allocator = Allocator {
            failure: Some(SpyError("must never allocate")),
            ..Allocator::default()
        };
        let mut backend = TracedRemappedBackend {
            inner: BorrowedMotionGraphBackend::new(&mut facade, &mut allocator),
            trace: vec![],
            wrong_identity_at: None,
        };
        controller.update(&mut backend, 0.25).unwrap();
        assert_eq!(
            backend.trace,
            [
                RemappedTrace::CreateNone(LeafId(0)),
                RemappedTrace::Callback(Begin, LeafId(0), Some(StateId(1)), None, [0; 2], true),
                RemappedTrace::Callback(Update, LeafId(0), Some(StateId(1)), None, [0; 2], true)
            ]
        );
        assert_eq!(
            backend.inner.facade.calls,
            [
                "remapped.setup:FACE_HAND_TEST",
                "remapped.update:FACE_HAND_TEST"
            ]
        );
        assert_eq!(
            controller.active_behaviours().collect::<Vec<_>>(),
            [(LeafId(0), None)]
        );
        assert_eq!(
            [
                controller.time_in_state(StateId(0)).to_bits(),
                controller.time_in_state(StateId(1)).to_bits()
            ],
            [0; 2]
        );
        backend.trace.clear();
        controller.update(&mut backend, 0.5).unwrap();
        assert_eq!(
            backend.trace,
            [RemappedTrace::Callback(
                Update,
                LeafId(0),
                Some(StateId(1)),
                Some(StateId(1)),
                [0x3f00_0000; 2],
                true
            )]
        );
        assert_eq!(controller.delta_time().to_bits(), 0x3f00_0000);
        assert_eq!(
            [
                controller.time_in_state(StateId(0)).to_bits(),
                controller.time_in_state(StateId(1)).to_bits()
            ],
            [0x3f00_0000; 2]
        );
        assert_eq!(
            controller.active_behaviours().collect::<Vec<_>>(),
            [(LeafId(0), None)]
        );
        controller.end_all_behaviours(&mut backend).unwrap();
        assert_eq!(
            backend.trace.last(),
            Some(&RemappedTrace::Callback(
                End,
                LeafId(0),
                Some(StateId(1)),
                Some(StateId(1)),
                [0x3f00_0000; 2],
                true
            ))
        );
        assert_eq!(
            backend.inner.facade.calls,
            [
                "remapped.setup:FACE_HAND_TEST",
                "remapped.update:FACE_HAND_TEST",
                "remapped.update:FACE_HAND_TEST"
            ]
        );
        assert_eq!(backend.inner.allocator.calls, 0);
        assert_eq!(controller.active_behaviours().len(), 0);
    }

    #[test]
    fn remapped_identity_rejection_precedes_each_callback_effect() {
        use MotionGraphDynamicCallbackSite::{Begin, CreateInstance, End, Update};
        for site in [CreateInstance, Begin, Update, End] {
            let mut controller =
                DynamicStateGraphController::new(remapped_graph(Some(0x80)), ()).unwrap();
            let mut facade = Facade::default();
            let mut allocator = Allocator::default();
            let mut backend = TracedRemappedBackend {
                inner: BorrowedMotionGraphBackend::new(&mut facade, &mut allocator),
                trace: vec![],
                wrong_identity_at: Some(site),
            };
            let error = if site == End {
                controller.update(&mut backend, 0.25).unwrap();
                backend.inner.facade.calls.clear();
                controller.end_all_behaviours(&mut backend).unwrap_err()
            } else {
                controller.update(&mut backend, 0.25).unwrap_err()
            };
            let identity = error
                .source()
                .unwrap()
                .source()
                .unwrap()
                .source()
                .unwrap()
                .downcast_ref::<MotionGraphDynamicIdentityError>()
                .unwrap();
            assert_eq!(identity.site, site);
            assert_eq!(identity.leaf, LeafId(0));
            assert_eq!(
                identity.reason,
                MotionGraphDynamicIdentityReason::PayloadVariantMismatch {
                    expected: MotionGraphLeafVariant::PlayAnimation,
                    actual: MotionGraphLeafVariant::PlayRemappedAnimation
                }
            );
            let expected: &[&str] = if site == Update {
                &["remapped.mirror:128", "remapped.setup:FACE_HAND_TEST"]
            } else {
                &[]
            };
            assert_eq!(backend.inner.facade.calls, expected);
            assert_eq!(backend.inner.allocator.calls, 0);
        }
    }

    #[test]
    fn remapped_begin_update_errors_preserve_nested_source_and_completed_prefix() {
        for (failure, expected) in [
            ("mirror", vec!["remapped.mirror:128"]),
            (
                "setup",
                vec!["remapped.mirror:128", "remapped.setup:FACE_HAND_TEST"],
            ),
            (
                "update",
                vec![
                    "remapped.mirror:128",
                    "remapped.setup:FACE_HAND_TEST",
                    "remapped.update:FACE_HAND_TEST",
                ],
            ),
        ] {
            let mut controller =
                DynamicStateGraphController::new(remapped_graph(Some(0x80)), ()).unwrap();
            let mut facade = Facade {
                fail_remapped: Some(failure),
                ..Facade::default()
            };
            let mut allocator = Allocator::default();
            let error = controller
                .update(
                    &mut BorrowedMotionGraphBackend::new(&mut facade, &mut allocator),
                    0.25,
                )
                .unwrap_err();
            let adapter = error.source().unwrap().source().unwrap();
            let typed = adapter.downcast_ref::<TestBackendError>().unwrap();
            if failure == "update" {
                assert!(matches!(
                    typed,
                    MotionGraphDynamicBackendError::PlayRemappedAnimationUpdate {
                        leaf: LeafId(0),
                        ..
                    }
                ));
            } else {
                assert!(matches!(
                    typed,
                    MotionGraphDynamicBackendError::PlayRemappedAnimationBegin {
                        leaf: LeafId(0),
                        ..
                    }
                ));
            }
            assert_eq!(
                adapter.source().unwrap().downcast_ref::<SpyError>(),
                Some(&SpyError(failure))
            );
            assert_eq!(facade.calls, expected);
            assert_eq!(allocator.calls, 0);
            assert!(controller.is_poisoned());
            assert_eq!(
                controller.active_behaviours().collect::<Vec<_>>(),
                [(LeafId(0), None)]
            );
        }
    }

    #[test]
    fn will_expire_success_uses_exact_tag_and_animatable_order() {
        let graph = build_graph(
            Some((
                WILL_EXPIRE_REGISTERED_NAME,
                vec![attr(b"InTimeTag", b"Exit", 0, 0)],
            )),
            false,
        );
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        let mut facade = Facade::default();
        let mut allocator = Allocator::default();
        let mut backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
        controller.update(&mut backend, 0.25).unwrap();
        assert_eq!(
            facade.calls,
            [
                "will.anim_info",
                "will.tag:Exit",
                "will.cached",
                "will.find",
                "will.transitioning",
                "will.property",
            ]
        );
        assert_eq!(allocator.calls, 0);
    }

    #[test]
    fn allocation_null_causes_begin_failure_after_retaining_active_pair_and_poisoning() {
        let graph = build_graph(None, true);
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        let mut facade = Facade::default();
        let mut allocator = Allocator::default();
        {
            let mut backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
            let error = controller.update(&mut backend, 0.5).unwrap_err();
            assert!(matches!(
                error,
                DynamicStateGraphControllerError::Lifecycle(
                    DynamicStateGraphLifecycleError::Begin {
                        source: MotionGraphDynamicBackendError::PlayAnimationBegin {
                            source: PlayAnimationRuntimeError::MissingInstance,
                            ..
                        },
                        ..
                    }
                )
            ));
            assert!(controller.is_poisoned());
            let active = controller.active_behaviours().collect::<Vec<_>>();
            assert_eq!(active.len(), 1);
            assert!(active[0].1.is_none());

            let retry = controller.update(&mut backend, 0.25).unwrap_err();
            assert!(matches!(
                retry,
                DynamicStateGraphControllerError::Invariant(
                    crate::dynamic_state_graph_controller::DynamicStateGraphInvariantError::Poisoned
                )
            ));
            assert_eq!(controller.active_behaviours().len(), 1);
            assert!(controller.active_behaviours().next().unwrap().1.is_none());
        }
        assert!(facade.calls.is_empty());
        assert_eq!(allocator.calls, 1);
    }

    #[test]
    fn has_intent_failure_preserves_leaf_and_adapter_source() {
        let graph = build_graph(
            Some((
                HAS_INTENT_REGISTERED_NAME,
                vec![attr(b"intent", b"Push", 0, 0)],
            )),
            false,
        );
        let condition_leaf = graph
            .leaves()
            .iter()
            .position(|leaf| leaf.registered_name.as_slice() == HAS_INTENT_REGISTERED_NAME);
        let condition_leaf = LeafId(condition_leaf.unwrap());
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        let mut facade = Facade {
            fail_has_cached: true,
            ..Facade::default()
        };
        let mut allocator = Allocator::default();
        let mut backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
        let error = controller.update(&mut backend, 0.25).unwrap_err();
        assert!(matches!(
            &error,
            DynamicStateGraphControllerError::Activation(
                crate::state_graph_runtime::StateGraphActivationError::Backend(
                    MotionGraphDynamicBackendError::HasIntent {
                        leaf,
                        source: HasIntentActivationError::Backend(SpyError("cached motion graph")),
                    }
                )
            ) if *leaf == condition_leaf
        ));
        let activation = error.source().expect("activation source");
        let adapter = activation.source().expect("adapter source");
        let condition = adapter.source().expect("HasIntent source");
        assert!(
            condition
                .downcast_ref::<HasIntentActivationError<SpyError>>()
                .is_some()
        );
        assert_eq!(facade.calls, ["has.cached"]);
        assert_eq!(allocator.calls, 0);
    }

    #[test]
    fn nested_play_animation_backend_source_is_preserved() {
        let graph = build_graph(None, true);
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        let mut facade = Facade {
            fail_dispatch: true,
            ..Facade::default()
        };
        let mut allocator = Allocator {
            allocated: true,
            ..Allocator::default()
        };
        let mut backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
        let error = controller.update(&mut backend, 0.5).unwrap_err();
        let lifecycle = error.source().expect("controller lifecycle source");
        let adapter = lifecycle.source().expect("adapter source");
        let runtime = adapter.source().expect("PlayAnimation runtime source");
        let backend = runtime.source().expect("PlayAnimation backend source");
        assert_eq!(
            backend.downcast_ref::<SpyError>(),
            Some(&SpyError("dispatch"))
        );
    }

    #[test]
    fn allocation_error_source_is_preserved_without_appending_active_pair() {
        let graph = build_graph(None, true);
        let mut controller = DynamicStateGraphController::new(graph, ()).unwrap();
        let mut facade = Facade::default();
        let mut allocator = Allocator {
            allocated: true,
            failure: Some(SpyError("allocate")),
            ..Allocator::default()
        };
        let mut backend = BorrowedMotionGraphBackend::new(&mut facade, &mut allocator);
        let error = controller.update(&mut backend, 0.5).unwrap_err();
        assert!(matches!(
            error,
            DynamicStateGraphControllerError::Lifecycle(
                DynamicStateGraphLifecycleError::CreateInstance {
                    source: MotionGraphDynamicBackendError::PlayAnimationCreate {
                        source: PlayAnimationRuntimeError::Backend {
                            operation: PlayAnimationBackendOperation::AllocateInstance,
                            source: SpyError("allocate"),
                        },
                        ..
                    },
                    ..
                }
            )
        ));
        assert!(controller.active_behaviours().next().is_none());
        assert!(facade.calls.is_empty());
    }
}
