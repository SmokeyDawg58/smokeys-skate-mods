//! Typed construction seam for the four recovered `MotionGraph` leaves.
//!
//! All other registered leaves remain owned by the supplied fallback factory.
//! This module reconstructs construction, `AddParam`, and `Process` only; it
//! does not claim generated-controller dispatch or per-frame activation.

use std::{error::Error, fmt};

use crate::{
    motion_graph_conditions::{HasIntentCondition, MotionConditionParseError, WillExpireCondition},
    play_animation::{PlayAnimationBehaviour, PlayAnimationParseError},
    play_remapped_animation::{PlayRemappedAnimationBehaviour, PlayRemappedAnimationParseError},
    state_graph_runtime::{
        LeafCreateRequest, LeafId, LeafKind, LeafProcessRequest, ParamRequest,
        StateGraphLeafFactory,
    },
};

pub const HAS_INTENT_REGISTERED_NAME: &[u8] = b"HasIntent";
pub const WILL_EXPIRE_REGISTERED_NAME: &[u8] = b"WillExpire";
pub const PLAY_ANIMATION_REGISTERED_NAME: &[u8] = b"PlayAnimation";
pub const PLAY_REMAPPED_ANIMATION_REGISTERED_NAME: &[u8] = b"PlayRemappedAnimation";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionGraphLeafPayload<O> {
    HasIntent(HasIntentCondition),
    WillExpire(WillExpireCondition),
    PlayAnimation(PlayAnimationBehaviour),
    PlayRemappedAnimation(PlayRemappedAnimationBehaviour),
    RegisteredOpaque(O),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionGraphLeafVariant {
    HasIntent,
    WillExpire,
    PlayAnimation,
    PlayRemappedAnimation,
    RegisteredOpaque,
}

impl<O> MotionGraphLeafPayload<O> {
    #[must_use]
    pub const fn variant(&self) -> MotionGraphLeafVariant {
        match self {
            Self::HasIntent(_) => MotionGraphLeafVariant::HasIntent,
            Self::WillExpire(_) => MotionGraphLeafVariant::WillExpire,
            Self::PlayAnimation(_) => MotionGraphLeafVariant::PlayAnimation,
            Self::PlayRemappedAnimation(_) => MotionGraphLeafVariant::PlayRemappedAnimation,
            Self::RegisteredOpaque(_) => MotionGraphLeafVariant::RegisteredOpaque,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionGraphLeafFactoryConfigurationError {
    EmbeddedNul {
        kind: LeafKind,
        name: Vec<u8>,
    },
    AsciiCaseDuplicate {
        kind: LeafKind,
        first: Vec<u8>,
        second: Vec<u8>,
    },
    MissingCanonical {
        kind: LeafKind,
        name: &'static [u8],
    },
}

impl fmt::Display for MotionGraphLeafFactoryConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmbeddedNul { kind, name } => write!(
                formatter,
                "{kind:?} registration {} contains an embedded NUL",
                String::from_utf8_lossy(name)
            ),
            Self::AsciiCaseDuplicate {
                kind,
                first,
                second,
            } => write!(
                formatter,
                "{kind:?} registrations {} and {} are ASCII-case-equivalent",
                String::from_utf8_lossy(first),
                String::from_utf8_lossy(second)
            ),
            Self::MissingCanonical { kind, name } => write!(
                formatter,
                "fallback is missing canonical {kind:?} registration {}",
                String::from_utf8_lossy(name)
            ),
        }
    }
}

impl Error for MotionGraphLeafFactoryConfigurationError {}

#[derive(Debug)]
pub enum MotionGraphLeafFactoryError<E> {
    Fallback {
        leaf: LeafId,
        source: E,
    },
    HasIntent {
        leaf: LeafId,
        source: MotionConditionParseError,
    },
    WillExpire {
        leaf: LeafId,
        source: MotionConditionParseError,
    },
    PlayAnimation {
        leaf: LeafId,
        source: PlayAnimationParseError,
    },
    PlayRemappedAnimation {
        leaf: LeafId,
        source: PlayRemappedAnimationParseError,
    },
    PlayRemappedAnimationParamUnsupported {
        leaf: LeafId,
    },
    PayloadIdentityMismatch {
        leaf: LeafId,
        kind: LeafKind,
        registered_name: Vec<u8>,
        actual: MotionGraphLeafVariant,
    },
}

impl<E: fmt::Display> fmt::Display for MotionGraphLeafFactoryError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fallback { leaf, source } => {
                write!(formatter, "fallback leaf {leaf:?} failed: {source}")
            }
            Self::HasIntent { leaf, source } => {
                write!(formatter, "HasIntent leaf {leaf:?} failed: {source}")
            }
            Self::WillExpire { leaf, source } => {
                write!(formatter, "WillExpire leaf {leaf:?} failed: {source}")
            }
            Self::PlayAnimation { leaf, source } => {
                write!(formatter, "PlayAnimation leaf {leaf:?} failed: {source}")
            }
            Self::PlayRemappedAnimation { leaf, source } => write!(
                formatter,
                "PlayRemappedAnimation leaf {leaf:?} failed: {source}"
            ),
            Self::PlayRemappedAnimationParamUnsupported { leaf } => write!(
                formatter,
                "PlayRemappedAnimation leaf {leaf:?} Param reaches the inherited native assertion"
            ),
            Self::PayloadIdentityMismatch {
                leaf,
                kind,
                registered_name,
                actual,
            } => write!(
                formatter,
                "leaf {leaf:?} callback identity {kind:?}/{} does not match {actual:?}",
                String::from_utf8_lossy(registered_name)
            ),
        }
    }
}

impl<E: Error + 'static> Error for MotionGraphLeafFactoryError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Fallback { source, .. } => Some(source),
            Self::HasIntent { source, .. } | Self::WillExpire { source, .. } => Some(source),
            Self::PlayAnimation { source, .. } => Some(source),
            Self::PlayRemappedAnimation { source, .. } => Some(source),
            Self::PlayRemappedAnimationParamUnsupported { .. }
            | Self::PayloadIdentityMismatch { .. } => None,
        }
    }
}

pub struct MotionGraphLeafFactory<F> {
    fallback: F,
}

impl<F: StateGraphLeafFactory> MotionGraphLeafFactory<F> {
    /// Wraps a pre-populated canonical registry without changing its lists.
    ///
    /// # Errors
    ///
    /// Rejects registry input outside the core factory's documented C-string
    /// and case-unique domain, or one lacking a recovered canonical identity.
    pub fn new(fallback: F) -> Result<Self, MotionGraphLeafFactoryConfigurationError> {
        for kind in [LeafKind::Condition, LeafKind::Behaviour, LeafKind::Hook] {
            validate_registration_list(kind, fallback.registered_names(kind))?;
        }
        require_canonical(&fallback, LeafKind::Condition, HAS_INTENT_REGISTERED_NAME)?;
        require_canonical(&fallback, LeafKind::Condition, WILL_EXPIRE_REGISTERED_NAME)?;
        require_canonical(
            &fallback,
            LeafKind::Behaviour,
            PLAY_ANIMATION_REGISTERED_NAME,
        )?;
        require_canonical(
            &fallback,
            LeafKind::Behaviour,
            PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
        )?;
        Ok(Self { fallback })
    }

    #[must_use]
    pub const fn fallback(&self) -> &F {
        &self.fallback
    }

    #[must_use]
    pub fn into_inner(self) -> F {
        self.fallback
    }
}

fn validate_registration_list(
    kind: LeafKind,
    names: &[Vec<u8>],
) -> Result<(), MotionGraphLeafFactoryConfigurationError> {
    for (index, name) in names.iter().enumerate() {
        if name.contains(&0) {
            return Err(MotionGraphLeafFactoryConfigurationError::EmbeddedNul {
                kind,
                name: name.clone(),
            });
        }
        if let Some(first) = names[..index]
            .iter()
            .find(|first| first.eq_ignore_ascii_case(name))
        {
            return Err(
                MotionGraphLeafFactoryConfigurationError::AsciiCaseDuplicate {
                    kind,
                    first: first.clone(),
                    second: name.clone(),
                },
            );
        }
    }
    Ok(())
}

fn require_canonical<F: StateGraphLeafFactory>(
    fallback: &F,
    kind: LeafKind,
    name: &'static [u8],
) -> Result<(), MotionGraphLeafFactoryConfigurationError> {
    if fallback
        .registered_names(kind)
        .iter()
        .any(|registered| registered.as_slice() == name)
    {
        Ok(())
    } else {
        Err(MotionGraphLeafFactoryConfigurationError::MissingCanonical { kind, name })
    }
}

fn expected_variant(kind: LeafKind, name: &[u8]) -> MotionGraphLeafVariant {
    match (kind, name) {
        (LeafKind::Condition, HAS_INTENT_REGISTERED_NAME) => MotionGraphLeafVariant::HasIntent,
        (LeafKind::Condition, WILL_EXPIRE_REGISTERED_NAME) => MotionGraphLeafVariant::WillExpire,
        (LeafKind::Behaviour, PLAY_ANIMATION_REGISTERED_NAME) => {
            MotionGraphLeafVariant::PlayAnimation
        }
        (LeafKind::Behaviour, PLAY_REMAPPED_ANIMATION_REGISTERED_NAME) => {
            MotionGraphLeafVariant::PlayRemappedAnimation
        }
        _ => MotionGraphLeafVariant::RegisteredOpaque,
    }
}

fn validate_payload_identity<E, O>(
    leaf: LeafId,
    kind: LeafKind,
    registered_name: &[u8],
    payload: &MotionGraphLeafPayload<O>,
) -> Result<(), MotionGraphLeafFactoryError<E>> {
    let actual = payload.variant();
    if actual == expected_variant(kind, registered_name) {
        Ok(())
    } else {
        Err(MotionGraphLeafFactoryError::PayloadIdentityMismatch {
            leaf,
            kind,
            registered_name: registered_name.to_vec(),
            actual,
        })
    }
}

impl<F: StateGraphLeafFactory> StateGraphLeafFactory for MotionGraphLeafFactory<F> {
    type Payload = MotionGraphLeafPayload<F::Payload>;
    type Error = MotionGraphLeafFactoryError<F::Error>;

    fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
        self.fallback.registered_names(kind)
    }

    fn create(&mut self, request: LeafCreateRequest<'_>) -> Result<Self::Payload, Self::Error> {
        match (request.kind, request.registered_name) {
            (LeafKind::Condition, HAS_INTENT_REGISTERED_NAME) => {
                HasIntentCondition::from_attributes(request.attributes)
                    .map(MotionGraphLeafPayload::HasIntent)
                    .map_err(|source| MotionGraphLeafFactoryError::HasIntent {
                        leaf: request.leaf,
                        source,
                    })
            }
            (LeafKind::Condition, WILL_EXPIRE_REGISTERED_NAME) => {
                WillExpireCondition::from_attributes(request.attributes)
                    .map(MotionGraphLeafPayload::WillExpire)
                    .map_err(|source| MotionGraphLeafFactoryError::WillExpire {
                        leaf: request.leaf,
                        source,
                    })
            }
            (LeafKind::Behaviour, PLAY_ANIMATION_REGISTERED_NAME) => {
                PlayAnimationBehaviour::from_attributes(request.attributes)
                    .map(MotionGraphLeafPayload::PlayAnimation)
                    .map_err(|source| MotionGraphLeafFactoryError::PlayAnimation {
                        leaf: request.leaf,
                        source,
                    })
            }
            (LeafKind::Behaviour, PLAY_REMAPPED_ANIMATION_REGISTERED_NAME) => {
                PlayRemappedAnimationBehaviour::from_attributes(request.attributes)
                    .map(MotionGraphLeafPayload::PlayRemappedAnimation)
                    .map_err(
                        |source| MotionGraphLeafFactoryError::PlayRemappedAnimation {
                            leaf: request.leaf,
                            source,
                        },
                    )
            }
            _ => {
                let leaf = request.leaf;
                self.fallback
                    .create(request)
                    .map(MotionGraphLeafPayload::RegisteredOpaque)
                    .map_err(|source| MotionGraphLeafFactoryError::Fallback { leaf, source })
            }
        }
    }

    fn add_param(&mut self, request: ParamRequest<'_, Self::Payload>) -> Result<bool, Self::Error> {
        validate_payload_identity(
            request.leaf,
            request.kind,
            request.registered_name,
            request.payload,
        )?;
        match request.payload {
            MotionGraphLeafPayload::HasIntent(_) | MotionGraphLeafPayload::WillExpire(_) => {
                Ok(true)
            }
            MotionGraphLeafPayload::PlayAnimation(payload) => payload
                .add_param(request.attributes)
                .map(|()| true)
                .map_err(|source| MotionGraphLeafFactoryError::PlayAnimation {
                    leaf: request.leaf,
                    source,
                }),
            MotionGraphLeafPayload::PlayRemappedAnimation(_) => Err(
                MotionGraphLeafFactoryError::PlayRemappedAnimationParamUnsupported {
                    leaf: request.leaf,
                },
            ),
            MotionGraphLeafPayload::RegisteredOpaque(payload) => self
                .fallback
                .add_param(ParamRequest {
                    leaf: request.leaf,
                    kind: request.kind,
                    registered_name: request.registered_name,
                    payload,
                    attributes: request.attributes,
                })
                .map_err(|source| MotionGraphLeafFactoryError::Fallback {
                    leaf: request.leaf,
                    source,
                }),
        }
    }

    fn process_leaf(
        &mut self,
        request: LeafProcessRequest<'_, Self::Payload>,
    ) -> Result<(), Self::Error> {
        validate_payload_identity(
            request.leaf,
            request.kind,
            request.registered_name,
            request.payload,
        )?;
        match request.payload {
            MotionGraphLeafPayload::HasIntent(_)
            | MotionGraphLeafPayload::WillExpire(_)
            | MotionGraphLeafPayload::PlayAnimation(_)
            | MotionGraphLeafPayload::PlayRemappedAnimation(_) => Ok(()),
            MotionGraphLeafPayload::RegisteredOpaque(payload) => self
                .fallback
                .process_leaf(LeafProcessRequest {
                    leaf: request.leaf,
                    kind: request.kind,
                    registered_name: request.registered_name,
                    payload,
                })
                .map_err(|source| MotionGraphLeafFactoryError::Fallback {
                    leaf: request.leaf,
                    source,
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::state_graph::{StateGraphBinaryAttribute, state_graph_string_hash32};

    use crate::state_graph_runtime::BaseId;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Opaque {
        params: usize,
        processed: usize,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct FallbackError(&'static str);

    impl fmt::Display for FallbackError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl Error for FallbackError {}

    struct Fallback {
        conditions: Vec<Vec<u8>>,
        behaviours: Vec<Vec<u8>>,
        hooks: Vec<Vec<u8>>,
        create_calls: usize,
        param_calls: usize,
        process_calls: usize,
        param_result: bool,
        create_error: Option<&'static str>,
        param_error: Option<&'static str>,
        process_error: Option<&'static str>,
    }

    impl Fallback {
        fn valid() -> Self {
            Self {
                conditions: vec![
                    b"OtherCondition".to_vec(),
                    HAS_INTENT_REGISTERED_NAME.to_vec(),
                    WILL_EXPIRE_REGISTERED_NAME.to_vec(),
                ],
                behaviours: vec![
                    PLAY_ANIMATION_REGISTERED_NAME.to_vec(),
                    PLAY_REMAPPED_ANIMATION_REGISTERED_NAME.to_vec(),
                    b"OtherBehaviour".to_vec(),
                ],
                hooks: vec![b"OtherHook".to_vec()],
                create_calls: 0,
                param_calls: 0,
                process_calls: 0,
                param_result: true,
                create_error: None,
                param_error: None,
                process_error: None,
            }
        }
    }

    impl StateGraphLeafFactory for Fallback {
        type Payload = Opaque;
        type Error = FallbackError;

        fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
            match kind {
                LeafKind::Condition => &self.conditions,
                LeafKind::Behaviour => &self.behaviours,
                LeafKind::Hook => &self.hooks,
            }
        }

        fn create(&mut self, _request: LeafCreateRequest<'_>) -> Result<Opaque, FallbackError> {
            self.create_calls += 1;
            if let Some(error) = self.create_error {
                return Err(FallbackError(error));
            }
            Ok(Opaque {
                params: 0,
                processed: 0,
            })
        }

        fn add_param(&mut self, request: ParamRequest<'_, Opaque>) -> Result<bool, FallbackError> {
            self.param_calls += 1;
            if let Some(error) = self.param_error {
                return Err(FallbackError(error));
            }
            request.payload.params += 1;
            Ok(self.param_result)
        }

        fn process_leaf(
            &mut self,
            request: LeafProcessRequest<'_, Opaque>,
        ) -> Result<(), FallbackError> {
            self.process_calls += 1;
            if let Some(error) = self.process_error {
                return Err(FallbackError(error));
            }
            request.payload.processed += 1;
            Ok(())
        }
    }

    fn attr(key: &[u8], text: &[u8], float: f32, boolean_raw: u8) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits: float.to_bits(),
            boolean_raw,
        }
    }

    fn create_request<'a>(
        leaf: usize,
        kind: LeafKind,
        requested_name: &'a [u8],
        registered_name: &'a [u8],
        attributes: &'a [StateGraphBinaryAttribute],
    ) -> LeafCreateRequest<'a> {
        LeafCreateRequest {
            leaf: LeafId(leaf),
            kind,
            parent: BaseId::State(crate::state_graph_runtime::StateId(0)),
            requested_name,
            registered_name,
            attributes,
        }
    }

    #[test]
    fn remapped_exact_route_missing_anim_param_assertion_and_noop_process() {
        let mut factory = MotionGraphLeafFactory::new(Fallback::valid()).unwrap();
        let attributes = [attr(b"anim", b"FACE_HAND_TEST", 0.0, 0)];
        let mut payload = factory
            .create(create_request(
                23,
                LeafKind::Behaviour,
                b"playremappedanimation",
                PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                &attributes,
            ))
            .unwrap();
        assert!(matches!(
            payload,
            MotionGraphLeafPayload::PlayRemappedAnimation(_)
        ));
        let original = payload.clone();
        factory
            .process_leaf(LeafProcessRequest {
                leaf: LeafId(23),
                kind: LeafKind::Behaviour,
                registered_name: PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                payload: &mut payload,
            })
            .unwrap();
        assert_eq!(payload, original);
        let error = factory
            .add_param(ParamRequest {
                leaf: LeafId(23),
                kind: LeafKind::Behaviour,
                registered_name: PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                payload: &mut payload,
                attributes: &[],
            })
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::PlayRemappedAnimationParamUnsupported { leaf: LeafId(23) }
        ));
        assert!(error.source().is_none());
        assert_eq!(payload, original);
        let error = factory
            .create(create_request(
                24,
                LeafKind::Behaviour,
                PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                &[],
            ))
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::PlayRemappedAnimation {
                leaf: LeafId(24),
                source: PlayRemappedAnimationParseError::MissingAnimation,
            }
        ));
        assert!(
            error
                .source()
                .unwrap()
                .downcast_ref::<PlayRemappedAnimationParseError>()
                .is_some()
        );
        assert_eq!(
            (
                factory.fallback().create_calls,
                factory.fallback().param_calls,
                factory.fallback().process_calls
            ),
            (0, 0, 0)
        );
        for (kind, name) in [
            (LeafKind::Condition, PLAY_REMAPPED_ANIMATION_REGISTERED_NAME),
            (LeafKind::Behaviour, b"playremappedanimation".as_slice()),
        ] {
            assert!(matches!(
                factory
                    .create(create_request(25, kind, name, name, &attributes))
                    .unwrap(),
                MotionGraphLeafPayload::RegisteredOpaque(_)
            ));
        }
        assert_eq!(factory.fallback().create_calls, 2);
    }

    #[test]
    fn preserves_registry_order_and_rejects_invalid_registry_domains() {
        let factory = MotionGraphLeafFactory::new(Fallback::valid()).unwrap();
        assert_eq!(
            factory.registered_names(LeafKind::Condition),
            [
                b"OtherCondition".as_slice(),
                HAS_INTENT_REGISTERED_NAME,
                WILL_EXPIRE_REGISTERED_NAME,
            ]
        );
        assert_eq!(
            factory.registered_names(LeafKind::Behaviour),
            [
                PLAY_ANIMATION_REGISTERED_NAME,
                PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                b"OtherBehaviour".as_slice()
            ]
        );

        let mut missing = Fallback::valid();
        missing.behaviours.remove(0);
        assert!(matches!(
            MotionGraphLeafFactory::new(missing),
            Err(MotionGraphLeafFactoryConfigurationError::MissingCanonical {
                kind: LeafKind::Behaviour,
                name: PLAY_ANIMATION_REGISTERED_NAME,
            })
        ));

        let mut duplicate = Fallback::valid();
        duplicate.conditions.push(b"hasintent".to_vec());
        assert!(matches!(
            MotionGraphLeafFactory::new(duplicate),
            Err(
                MotionGraphLeafFactoryConfigurationError::AsciiCaseDuplicate {
                    kind: LeafKind::Condition,
                    ..
                }
            )
        ));

        let mut nul = Fallback::valid();
        nul.hooks.push(b"Bad\0Hook".to_vec());
        assert!(matches!(
            MotionGraphLeafFactory::new(nul),
            Err(MotionGraphLeafFactoryConfigurationError::EmbeddedNul {
                kind: LeafKind::Hook,
                ..
            })
        ));
    }

    #[test]
    fn exact_canonical_identity_routes_typed_and_other_pairs_delegate() {
        let mut factory = MotionGraphLeafFactory::new(Fallback::valid()).unwrap();
        let intent_attrs = [attr(b"intent", b"Pop", 0.0, 0)];
        let payload = factory
            .create(create_request(
                3,
                LeafKind::Condition,
                b"hAsInTeNt",
                HAS_INTENT_REGISTERED_NAME,
                &intent_attrs,
            ))
            .unwrap();
        assert!(matches!(payload, MotionGraphLeafPayload::HasIntent(_)));
        assert_eq!(factory.fallback().create_calls, 0);

        let opaque = factory
            .create(create_request(
                4,
                LeafKind::Hook,
                b"otherhook",
                b"OtherHook",
                &[],
            ))
            .unwrap();
        assert!(matches!(
            opaque,
            MotionGraphLeafPayload::RegisteredOpaque(_)
        ));
        assert_eq!(factory.fallback().create_calls, 1);

        let opaque = factory
            .create(create_request(
                5,
                LeafKind::Hook,
                b"HasIntent",
                HAS_INTENT_REGISTERED_NAME,
                &[],
            ))
            .unwrap();
        assert!(matches!(
            opaque,
            MotionGraphLeafPayload::RegisteredOpaque(_)
        ));
        assert_eq!(factory.fallback().create_calls, 2);
    }

    #[test]
    fn typed_constructor_errors_preserve_leaf_and_variant() {
        let mut factory = MotionGraphLeafFactory::new(Fallback::valid()).unwrap();
        let error = factory
            .create(create_request(
                17,
                LeafKind::Condition,
                HAS_INTENT_REGISTERED_NAME,
                HAS_INTENT_REGISTERED_NAME,
                &[],
            ))
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::HasIntent {
                leaf: LeafId(17),
                source: MotionConditionParseError::MissingAttribute { key: b"intent", .. },
            }
        ));

        let error = factory
            .create(create_request(
                18,
                LeafKind::Behaviour,
                PLAY_ANIMATION_REGISTERED_NAME,
                PLAY_ANIMATION_REGISTERED_NAME,
                &[],
            ))
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::PlayAnimation {
                leaf: LeafId(18),
                source: PlayAnimationParseError::MissingAttribute { key: b"anim", .. },
            }
        ));
    }

    #[test]
    fn add_param_and_process_follow_typed_noop_and_fallback_boundaries() {
        let mut factory = MotionGraphLeafFactory::new(Fallback::valid()).unwrap();
        let mut condition = MotionGraphLeafPayload::HasIntent(
            HasIntentCondition::from_attributes(&[attr(b"intent", b"Pop", 0.0, 0)]).unwrap(),
        );
        assert!(
            factory
                .add_param(ParamRequest {
                    leaf: LeafId(1),
                    kind: LeafKind::Condition,
                    registered_name: HAS_INTENT_REGISTERED_NAME,
                    payload: &mut condition,
                    attributes: &[],
                })
                .unwrap()
        );
        factory
            .process_leaf(LeafProcessRequest {
                leaf: LeafId(1),
                kind: LeafKind::Condition,
                registered_name: HAS_INTENT_REGISTERED_NAME,
                payload: &mut condition,
            })
            .unwrap();
        assert_eq!(factory.fallback().param_calls, 0);
        assert_eq!(factory.fallback().process_calls, 0);

        let anim_attrs = [attr(b"anim", b"Idle", 0.0, 0)];
        let mut animation = MotionGraphLeafPayload::PlayAnimation(
            PlayAnimationBehaviour::from_attributes(&anim_attrs).unwrap(),
        );
        let param_attrs = [
            attr(b"from", b"lastAnim", 0.0, 0),
            attr(b"attribute", b"Speed", 0.0, 0),
        ];
        assert!(
            factory
                .add_param(ParamRequest {
                    leaf: LeafId(2),
                    kind: LeafKind::Behaviour,
                    registered_name: PLAY_ANIMATION_REGISTERED_NAME,
                    payload: &mut animation,
                    attributes: &param_attrs,
                })
                .unwrap()
        );
        let MotionGraphLeafPayload::PlayAnimation(animation) = animation else {
            unreachable!();
        };
        assert_eq!(animation.params().len(), 1);

        let mut opaque = MotionGraphLeafPayload::RegisteredOpaque(Opaque {
            params: 0,
            processed: 0,
        });
        assert!(
            factory
                .add_param(ParamRequest {
                    leaf: LeafId(3),
                    kind: LeafKind::Behaviour,
                    registered_name: b"OtherBehaviour",
                    payload: &mut opaque,
                    attributes: &[],
                })
                .unwrap()
        );
        factory
            .process_leaf(LeafProcessRequest {
                leaf: LeafId(3),
                kind: LeafKind::Behaviour,
                registered_name: b"OtherBehaviour",
                payload: &mut opaque,
            })
            .unwrap();
        let mut false_fallback = Fallback::valid();
        false_fallback.param_result = false;
        let mut false_factory = MotionGraphLeafFactory::new(false_fallback).unwrap();
        assert!(
            !false_factory
                .add_param(ParamRequest {
                    leaf: LeafId(3),
                    kind: LeafKind::Behaviour,
                    registered_name: b"OtherBehaviour",
                    payload: &mut opaque,
                    attributes: &[],
                })
                .unwrap()
        );
        assert_eq!(factory.fallback().param_calls, 1);
        assert_eq!(factory.fallback().process_calls, 1);
        assert_eq!(false_factory.fallback().param_calls, 1);
        assert!(matches!(
            opaque,
            MotionGraphLeafPayload::RegisteredOpaque(Opaque {
                params: 2,
                processed: 1,
            })
        ));
    }

    #[test]
    fn remaining_recovered_callbacks_are_handled_without_fallback() {
        let mut factory = MotionGraphLeafFactory::new(Fallback::valid()).unwrap();
        let mut will_expire =
            MotionGraphLeafPayload::WillExpire(WillExpireCondition::from_attributes(&[]).unwrap());
        assert!(
            factory
                .add_param(ParamRequest {
                    leaf: LeafId(4),
                    kind: LeafKind::Condition,
                    registered_name: WILL_EXPIRE_REGISTERED_NAME,
                    payload: &mut will_expire,
                    attributes: &[],
                })
                .unwrap()
        );
        factory
            .process_leaf(LeafProcessRequest {
                leaf: LeafId(4),
                kind: LeafKind::Condition,
                registered_name: WILL_EXPIRE_REGISTERED_NAME,
                payload: &mut will_expire,
            })
            .unwrap();

        let anim_attrs = [attr(b"anim", b"Idle", 0.0, 0)];
        let mut animation = MotionGraphLeafPayload::PlayAnimation(
            PlayAnimationBehaviour::from_attributes(&anim_attrs).unwrap(),
        );
        factory
            .process_leaf(LeafProcessRequest {
                leaf: LeafId(2),
                kind: LeafKind::Behaviour,
                registered_name: PLAY_ANIMATION_REGISTERED_NAME,
                payload: &mut animation,
            })
            .unwrap();
        assert_eq!(factory.fallback().param_calls, 0);
        assert_eq!(factory.fallback().process_calls, 0);
    }

    #[test]
    fn play_animation_param_failure_preserves_leaf_and_existing_payload() {
        let mut factory = MotionGraphLeafFactory::new(Fallback::valid()).unwrap();
        let anim_attrs = [attr(b"anim", b"Idle", 0.0, 0)];
        let mut animation = MotionGraphLeafPayload::PlayAnimation(
            PlayAnimationBehaviour::from_attributes(&anim_attrs).unwrap(),
        );
        let error = factory
            .add_param(ParamRequest {
                leaf: LeafId(2),
                kind: LeafKind::Behaviour,
                registered_name: PLAY_ANIMATION_REGISTERED_NAME,
                payload: &mut animation,
                attributes: &[],
            })
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::PlayAnimation {
                leaf: LeafId(2),
                source: PlayAnimationParseError::MissingAttribute { key: b"from", .. },
            }
        ));
        let MotionGraphLeafPayload::PlayAnimation(animation) = animation else {
            unreachable!();
        };
        assert!(animation.params().is_empty());
    }

    #[test]
    fn callback_identity_mismatch_is_detected_before_delegation() {
        let mut factory = MotionGraphLeafFactory::new(Fallback::valid()).unwrap();
        let mut opaque = MotionGraphLeafPayload::RegisteredOpaque(Opaque {
            params: 0,
            processed: 0,
        });
        let error = factory
            .process_leaf(LeafProcessRequest {
                leaf: LeafId(9),
                kind: LeafKind::Condition,
                registered_name: HAS_INTENT_REGISTERED_NAME,
                payload: &mut opaque,
            })
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::PayloadIdentityMismatch {
                leaf: LeafId(9),
                actual: MotionGraphLeafVariant::RegisteredOpaque,
                ..
            }
        ));
        assert_eq!(factory.fallback().process_calls, 0);

        let error = factory
            .add_param(ParamRequest {
                leaf: LeafId(10),
                kind: LeafKind::Behaviour,
                registered_name: PLAY_ANIMATION_REGISTERED_NAME,
                payload: &mut opaque,
                attributes: &[],
            })
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::PayloadIdentityMismatch {
                leaf: LeafId(10),
                actual: MotionGraphLeafVariant::RegisteredOpaque,
                ..
            }
        ));
        assert_eq!(factory.fallback().param_calls, 0);
    }

    #[test]
    fn fallback_errors_preserve_leaf_source_and_single_call() {
        let mut fallback = Fallback::valid();
        fallback.create_error = Some("create");
        fallback.param_error = Some("param");
        fallback.process_error = Some("process");
        let mut factory = MotionGraphLeafFactory::new(fallback).unwrap();

        let error = factory
            .create(create_request(
                21,
                LeafKind::Behaviour,
                b"OtherBehaviour",
                b"OtherBehaviour",
                &[],
            ))
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::Fallback {
                leaf: LeafId(21),
                source: FallbackError("create"),
            }
        ));
        assert_eq!(factory.fallback().create_calls, 1);

        let mut opaque = MotionGraphLeafPayload::RegisteredOpaque(Opaque {
            params: 0,
            processed: 0,
        });
        let error = factory
            .add_param(ParamRequest {
                leaf: LeafId(22),
                kind: LeafKind::Behaviour,
                registered_name: b"OtherBehaviour",
                payload: &mut opaque,
                attributes: &[],
            })
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::Fallback {
                leaf: LeafId(22),
                source: FallbackError("param"),
            }
        ));
        assert_eq!(factory.fallback().param_calls, 1);

        let error = factory
            .process_leaf(LeafProcessRequest {
                leaf: LeafId(23),
                kind: LeafKind::Behaviour,
                registered_name: b"OtherBehaviour",
                payload: &mut opaque,
            })
            .unwrap_err();
        assert!(matches!(
            error,
            MotionGraphLeafFactoryError::Fallback {
                leaf: LeafId(23),
                source: FallbackError("process"),
            }
        ));
        assert_eq!(factory.fallback().process_calls, 1);
        assert!(matches!(
            opaque,
            MotionGraphLeafPayload::RegisteredOpaque(Opaque {
                params: 0,
                processed: 0,
            })
        ));
    }
}
