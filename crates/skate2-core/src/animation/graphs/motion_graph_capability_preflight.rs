//! Immutable capability preflight for typed dynamic `MotionGraph` leaves.
//!
//! This pass does not invoke construction factories, lifecycle backends, or
//! leaf callbacks. It only inspects the already-owned runtime graph and
//! allocates an ordered host diagnostic report. It does not claim that a graph
//! passing these four recovered leaf capabilities is executable or otherwise
//! frame-ready. A future runtime adapter must still revalidate the exact leaf
//! triple at every callback boundary.

use std::{error::Error, fmt};

use crate::{
    motion_graph_leaf_factory::{
        HAS_INTENT_REGISTERED_NAME, MotionGraphLeafPayload, MotionGraphLeafVariant,
        PLAY_ANIMATION_REGISTERED_NAME, PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
        WILL_EXPIRE_REGISTERED_NAME,
    },
    state_graph_runtime::{BaseId, LeafId, LeafKind, StateGraphRuntime},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequiredMotionGraphCapability {
    HasIntentCondition,
    WillExpireCondition,
    PlayAnimationBehaviour,
    PlayRemappedAnimationBehaviour,
    RegisteredLeaf {
        kind: LeafKind,
        registered_name: Vec<u8>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionGraphCapabilityUnsupportedReason {
    RequestedEmbeddedNul,
    CanonicalEmbeddedNul,
    RequestedNameMismatch,
    MissingPayload,
    RegisteredOpaque,
    UnsupportedIdentity,
    PayloadVariantMismatch {
        expected: MotionGraphLeafVariant,
        actual: MotionGraphLeafVariant,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionGraphCapabilityDiagnostic {
    pub leaf: LeafId,
    pub path: Vec<u8>,
    pub parent: Option<BaseId>,
    pub kind: LeafKind,
    /// Raw `name` attribute retained in `BaseFields` by construction.
    pub requested_name: Vec<u8>,
    /// Exact registry entry selected by the construction factory.
    pub canonical_registered_name: Vec<u8>,
    pub payload_variant: Option<MotionGraphLeafVariant>,
    pub required: RequiredMotionGraphCapability,
    pub reason: MotionGraphCapabilityUnsupportedReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionGraphCapabilityPreflightReport {
    pub leaf_count: usize,
    pub diagnostics: Vec<MotionGraphCapabilityDiagnostic>,
}

impl MotionGraphCapabilityPreflightReport {
    /// Reports only complete coverage by the four typed leaf bodies. It is
    /// not permission or proof that the graph can execute.
    #[must_use]
    pub fn has_full_leaf_coverage(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionGraphCapabilityPreflightError {
    GraphNotProcessed,
}

impl fmt::Display for MotionGraphCapabilityPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GraphNotProcessed => formatter.write_str(
                "MotionGraph capability preflight requires completed structural Process",
            ),
        }
    }
}

impl Error for MotionGraphCapabilityPreflightError {}

/// Checks the exact four reconstructed dynamic `MotionGraph` leaf triples.
///
/// Every arena leaf is scanned, including disabled, unreachable, and replaced
/// subtrees. Diagnostics are emitted once per unsupported leaf in ascending
/// `LeafId` order. When a malformed public host graph violates multiple
/// construction invariants, reason precedence is requested NUL, canonical NUL,
/// requested ASCII-case mismatch, missing payload, known-identity payload
/// mismatch, then unsupported canonical identity (opaque or typed).
///
/// # Errors
///
/// Returns [`MotionGraphCapabilityPreflightError::GraphNotProcessed`] before
/// leaf classification when structural `Process` did not complete.
pub fn preflight_motion_graph_capabilities<O>(
    graph: &StateGraphRuntime<MotionGraphLeafPayload<O>>,
) -> Result<MotionGraphCapabilityPreflightReport, MotionGraphCapabilityPreflightError> {
    if !graph.process_succeeded() {
        return Err(MotionGraphCapabilityPreflightError::GraphNotProcessed);
    }
    let diagnostics = graph
        .leaves()
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            let payload_variant = node.payload().map(MotionGraphLeafPayload::variant);
            let (required, reason) = classify_leaf(
                node.kind,
                &node.base.name,
                &node.registered_name,
                payload_variant,
            )?;
            Some(MotionGraphCapabilityDiagnostic {
                leaf: LeafId(index),
                path: node.base.path.clone(),
                parent: node.base.parent,
                kind: node.kind,
                requested_name: node.base.name.clone(),
                canonical_registered_name: node.registered_name.clone(),
                payload_variant,
                required,
                reason,
            })
        })
        .collect();
    Ok(MotionGraphCapabilityPreflightReport {
        leaf_count: graph.leaves().len(),
        diagnostics,
    })
}

fn classify_leaf(
    kind: LeafKind,
    requested_name: &[u8],
    registered_name: &[u8],
    payload_variant: Option<MotionGraphLeafVariant>,
) -> Option<(
    RequiredMotionGraphCapability,
    MotionGraphCapabilityUnsupportedReason,
)> {
    let (required, expected) = required_capability(kind, registered_name);
    let reason = if requested_name.contains(&0) {
        MotionGraphCapabilityUnsupportedReason::RequestedEmbeddedNul
    } else if registered_name.contains(&0) {
        MotionGraphCapabilityUnsupportedReason::CanonicalEmbeddedNul
    } else if !requested_name.eq_ignore_ascii_case(registered_name) {
        MotionGraphCapabilityUnsupportedReason::RequestedNameMismatch
    } else {
        let Some(actual) = payload_variant else {
            return Some((
                required,
                MotionGraphCapabilityUnsupportedReason::MissingPayload,
            ));
        };
        match expected {
            Some(expected) if expected == actual => return None,
            Some(expected) => {
                MotionGraphCapabilityUnsupportedReason::PayloadVariantMismatch { expected, actual }
            }
            None if actual == MotionGraphLeafVariant::RegisteredOpaque => {
                MotionGraphCapabilityUnsupportedReason::RegisteredOpaque
            }
            None => MotionGraphCapabilityUnsupportedReason::UnsupportedIdentity,
        }
    };
    Some((required, reason))
}

fn required_capability(
    kind: LeafKind,
    registered_name: &[u8],
) -> (
    RequiredMotionGraphCapability,
    Option<MotionGraphLeafVariant>,
) {
    match (kind, registered_name) {
        (LeafKind::Condition, HAS_INTENT_REGISTERED_NAME) => (
            RequiredMotionGraphCapability::HasIntentCondition,
            Some(MotionGraphLeafVariant::HasIntent),
        ),
        (LeafKind::Condition, WILL_EXPIRE_REGISTERED_NAME) => (
            RequiredMotionGraphCapability::WillExpireCondition,
            Some(MotionGraphLeafVariant::WillExpire),
        ),
        (LeafKind::Behaviour, PLAY_ANIMATION_REGISTERED_NAME) => (
            RequiredMotionGraphCapability::PlayAnimationBehaviour,
            Some(MotionGraphLeafVariant::PlayAnimation),
        ),
        (LeafKind::Behaviour, PLAY_REMAPPED_ANIMATION_REGISTERED_NAME) => (
            RequiredMotionGraphCapability::PlayRemappedAnimationBehaviour,
            Some(MotionGraphLeafVariant::PlayRemappedAnimation),
        ),
        _ => (
            RequiredMotionGraphCapability::RegisteredLeaf {
                kind,
                registered_name: registered_name.to_vec(),
            },
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        motion_graph_conditions::{HasIntentCondition, WillExpireCondition},
        play_animation::PlayAnimationBehaviour,
        state_graph_runtime::{
            FactoryMissPolicy, LeafCreateRequest, StateGraphLeafFactory, StateGraphRuntime,
        },
    };
    use skate2_formats::state_graph::{
        StateGraphBinaryAttribute, StateGraphBinaryElement, state_graph_string_hash32,
    };
    use std::{collections::VecDeque, convert::Infallible};

    type Payload = MotionGraphLeafPayload<()>;

    struct Factory {
        conditions: Vec<Vec<u8>>,
        behaviours: Vec<Vec<u8>>,
        hooks: Vec<Vec<u8>>,
        payloads: VecDeque<Payload>,
    }

    impl Factory {
        fn with_payloads(payloads: impl IntoIterator<Item = Payload>) -> Self {
            Self {
                conditions: vec![
                    HAS_INTENT_REGISTERED_NAME.to_vec(),
                    WILL_EXPIRE_REGISTERED_NAME.to_vec(),
                    b"OtherCondition".to_vec(),
                ],
                behaviours: vec![
                    PLAY_ANIMATION_REGISTERED_NAME.to_vec(),
                    PLAY_REMAPPED_ANIMATION_REGISTERED_NAME.to_vec(),
                    b"OtherBehaviour".to_vec(),
                ],
                hooks: vec![b"OtherHook".to_vec()],
                payloads: payloads.into_iter().collect(),
            }
        }
    }

    impl StateGraphLeafFactory for Factory {
        type Payload = Payload;
        type Error = Infallible;

        fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
            match kind {
                LeafKind::Condition => &self.conditions,
                LeafKind::Behaviour => &self.behaviours,
                LeafKind::Hook => &self.hooks,
            }
        }

        fn create(&mut self, _request: LeafCreateRequest<'_>) -> Result<Payload, Infallible> {
            Ok(self.payloads.pop_front().expect("fixture payload"))
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
        tag: &[u8],
        attributes: Vec<StateGraphBinaryAttribute>,
        children: Vec<StateGraphBinaryElement>,
    ) -> StateGraphBinaryElement {
        StateGraphBinaryElement {
            name: tag.to_vec(),
            attributes,
            children,
        }
    }

    fn named_leaf(tag: &[u8], name: &[u8]) -> StateGraphBinaryElement {
        element(tag, vec![attr(b"name", name)], Vec::new())
    }

    fn state(children: Vec<StateGraphBinaryElement>) -> StateGraphBinaryElement {
        element(b"state", vec![attr(b"name", b"Root")], children)
    }

    fn expression(children: Vec<StateGraphBinaryElement>) -> StateGraphBinaryElement {
        element(b"expression", Vec::new(), children)
    }

    fn transition(children: Vec<StateGraphBinaryElement>) -> StateGraphBinaryElement {
        element(b"transition", vec![attr(b"target", b"Root")], children)
    }

    #[allow(clippy::needless_pass_by_value)] // Keeps nested binary fixtures readable.
    fn build(root: StateGraphBinaryElement, factory: &mut Factory) -> StateGraphRuntime<Payload> {
        let mut graph =
            StateGraphRuntime::build(&root, factory, FactoryMissPolicy::Strict).unwrap();
        assert!(factory.payloads.is_empty());
        graph.process_after_build(factory).unwrap();
        graph
    }

    fn has_intent() -> Payload {
        MotionGraphLeafPayload::HasIntent(
            HasIntentCondition::from_attributes(&[attr(b"intent", b"Idle")]).unwrap(),
        )
    }

    fn will_expire() -> Payload {
        MotionGraphLeafPayload::WillExpire(WillExpireCondition::from_attributes(&[]).unwrap())
    }

    fn play_animation() -> Payload {
        MotionGraphLeafPayload::PlayAnimation(
            PlayAnimationBehaviour::from_attributes(&[attr(b"anim", b"Idle")]).unwrap(),
        )
    }

    #[test]
    fn remapped_exact_preflight_and_wrong_variant_precedence() {
        use crate::play_remapped_animation::PlayRemappedAnimationBehaviour;
        let payload = MotionGraphLeafPayload::PlayRemappedAnimation(
            PlayRemappedAnimationBehaviour::from_attributes(&[attr(b"anim", b"FACE_HAND_TEST")])
                .unwrap(),
        );
        let root = state(vec![named_leaf(b"behaviour", b"playremappedanimation")]);
        let graph = build(root, &mut Factory::with_payloads([payload]));
        assert_eq!(
            preflight_motion_graph_capabilities(&graph).unwrap(),
            MotionGraphCapabilityPreflightReport {
                leaf_count: 1,
                diagnostics: vec![]
            }
        );
        for actual in [
            MotionGraphLeafVariant::PlayAnimation,
            MotionGraphLeafVariant::RegisteredOpaque,
        ] {
            assert_eq!(
                classify_leaf(
                    LeafKind::Behaviour,
                    PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                    PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                    Some(actual)
                ),
                Some((
                    RequiredMotionGraphCapability::PlayRemappedAnimationBehaviour,
                    MotionGraphCapabilityUnsupportedReason::PayloadVariantMismatch {
                        expected: MotionGraphLeafVariant::PlayRemappedAnimation,
                        actual
                    }
                ))
            );
            assert_eq!(
                classify_leaf(
                    LeafKind::Behaviour,
                    b"Other",
                    PLAY_REMAPPED_ANIMATION_REGISTERED_NAME,
                    Some(actual)
                ),
                Some((
                    RequiredMotionGraphCapability::PlayRemappedAnimationBehaviour,
                    MotionGraphCapabilityUnsupportedReason::RequestedNameMismatch
                ))
            );
        }
        for (kind, name) in [
            (LeafKind::Condition, PLAY_REMAPPED_ANIMATION_REGISTERED_NAME),
            (LeafKind::Behaviour, b"playremappedanimation".as_slice()),
        ] {
            assert!(matches!(
                classify_leaf(
                    kind,
                    name,
                    name,
                    Some(MotionGraphLeafVariant::PlayRemappedAnimation)
                ),
                Some((
                    RequiredMotionGraphCapability::RegisteredLeaf { .. },
                    MotionGraphCapabilityUnsupportedReason::UnsupportedIdentity
                ))
            ));
        }
    }

    #[test]
    fn accepts_only_exact_triples_while_requested_case_remains_raw() {
        let root = state(vec![
            expression(vec![
                named_leaf(b"condition", b"hAsInTeNt"),
                named_leaf(b"condition", b"WILLEXPIRE"),
            ]),
            named_leaf(b"behaviour", b"playanimation"),
        ]);
        let mut factory = Factory::with_payloads([has_intent(), will_expire(), play_animation()]);
        let graph = build(root, &mut factory);

        assert_eq!(graph.leaves()[0].base.name, b"hAsInTeNt");
        assert_eq!(
            graph.leaves()[0].registered_name,
            HAS_INTENT_REGISTERED_NAME
        );
        assert_eq!(graph.leaves()[1].base.name, b"WILLEXPIRE");
        assert_eq!(
            graph.leaves()[1].registered_name,
            WILL_EXPIRE_REGISTERED_NAME
        );
        assert_eq!(graph.leaves()[2].base.name, b"playanimation");
        assert_eq!(
            graph.leaves()[2].registered_name,
            PLAY_ANIMATION_REGISTERED_NAME
        );
        assert_eq!(
            preflight_motion_graph_capabilities(&graph).unwrap(),
            MotionGraphCapabilityPreflightReport {
                leaf_count: 3,
                diagnostics: Vec::new(),
            }
        );
    }

    #[test]
    fn swapped_typed_variants_are_reported_once_each_in_leaf_order() {
        let root = state(vec![
            expression(vec![
                named_leaf(b"condition", b"HasIntent"),
                named_leaf(b"condition", b"WillExpire"),
            ]),
            named_leaf(b"behaviour", b"PlayAnimation"),
        ]);
        let mut factory = Factory::with_payloads([will_expire(), has_intent(), has_intent()]);
        let graph = build(root, &mut factory);
        let report = preflight_motion_graph_capabilities(&graph).unwrap();

        assert_eq!(
            report
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.leaf)
                .collect::<Vec<_>>(),
            [LeafId(0), LeafId(1), LeafId(2)]
        );
        assert_eq!(
            report.diagnostics[0].reason,
            MotionGraphCapabilityUnsupportedReason::PayloadVariantMismatch {
                expected: MotionGraphLeafVariant::HasIntent,
                actual: MotionGraphLeafVariant::WillExpire,
            }
        );
        assert_eq!(
            report.diagnostics[1].reason,
            MotionGraphCapabilityUnsupportedReason::PayloadVariantMismatch {
                expected: MotionGraphLeafVariant::WillExpire,
                actual: MotionGraphLeafVariant::HasIntent,
            }
        );
        assert_eq!(
            report.diagnostics[2].reason,
            MotionGraphCapabilityUnsupportedReason::PayloadVariantMismatch {
                expected: MotionGraphLeafVariant::PlayAnimation,
                actual: MotionGraphLeafVariant::HasIntent,
            }
        );
    }

    #[test]
    fn opaque_and_unimplemented_identities_include_complete_raw_diagnostics() {
        let root = state(vec![
            expression(vec![
                named_leaf(b"condition", b"HasIntent"),
                named_leaf(b"condition", b"OtherCondition"),
            ]),
            named_leaf(b"behaviour", b"PlayAnimation"),
            named_leaf(b"behaviour", b"OtherBehaviour"),
            transition(vec![named_leaf(b"hook", b"OtherHook")]),
        ]);
        let mut factory = Factory::with_payloads([
            MotionGraphLeafPayload::RegisteredOpaque(()),
            has_intent(),
            MotionGraphLeafPayload::RegisteredOpaque(()),
            MotionGraphLeafPayload::RegisteredOpaque(()),
            MotionGraphLeafPayload::RegisteredOpaque(()),
        ]);
        let graph = build(root, &mut factory);
        let report = preflight_motion_graph_capabilities(&graph).unwrap();

        assert_eq!(report.leaf_count, 5);
        assert_eq!(report.diagnostics.len(), 5);
        assert_eq!(
            report
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.leaf)
                .collect::<Vec<_>>(),
            [LeafId(0), LeafId(1), LeafId(2), LeafId(3), LeafId(4)]
        );
        assert_eq!(
            report.diagnostics[0].reason,
            MotionGraphCapabilityUnsupportedReason::PayloadVariantMismatch {
                expected: MotionGraphLeafVariant::HasIntent,
                actual: MotionGraphLeafVariant::RegisteredOpaque,
            }
        );
        assert_eq!(
            report.diagnostics[1].reason,
            MotionGraphCapabilityUnsupportedReason::UnsupportedIdentity
        );
        assert_eq!(
            report.diagnostics[2].reason,
            MotionGraphCapabilityUnsupportedReason::PayloadVariantMismatch {
                expected: MotionGraphLeafVariant::PlayAnimation,
                actual: MotionGraphLeafVariant::RegisteredOpaque,
            }
        );
        assert_eq!(
            report.diagnostics[3].required,
            RequiredMotionGraphCapability::RegisteredLeaf {
                kind: LeafKind::Behaviour,
                registered_name: b"OtherBehaviour".to_vec(),
            }
        );
        assert_eq!(
            report.diagnostics[3].reason,
            MotionGraphCapabilityUnsupportedReason::RegisteredOpaque
        );
        assert_eq!(
            report.diagnostics[4].parent,
            Some(BaseId::Transition(
                crate::state_graph_runtime::TransitionId(0)
            ))
        );
        assert_eq!(report.diagnostics[4].path, b"Root.__unknown__.OtherHook");
        assert_eq!(report.diagnostics[4].kind, LeafKind::Hook);
        assert_eq!(report.diagnostics[4].requested_name, b"OtherHook");
        assert_eq!(
            report.diagnostics[4].canonical_registered_name,
            b"OtherHook"
        );
        assert_eq!(
            report.diagnostics[4].payload_variant,
            Some(MotionGraphLeafVariant::RegisteredOpaque)
        );
    }

    #[test]
    fn duplicate_paths_do_not_deduplicate_or_reorder_leaf_ids() {
        let root = state(vec![
            named_leaf(b"behaviour", b"PlayAnimation"),
            named_leaf(b"behaviour", b"PlayAnimation"),
        ]);
        let mut factory = Factory::with_payloads([
            MotionGraphLeafPayload::RegisteredOpaque(()),
            MotionGraphLeafPayload::RegisteredOpaque(()),
        ]);
        let graph = build(root, &mut factory);
        assert_eq!(graph.leaves()[0].base.path, graph.leaves()[1].base.path);

        let report = preflight_motion_graph_capabilities(&graph).unwrap();
        assert_eq!(
            report
                .diagnostics
                .iter()
                .map(|diagnostic| (diagnostic.leaf, diagnostic.path.clone()))
                .collect::<Vec<_>>(),
            [
                (LeafId(0), b"Root.PlayAnimation".to_vec()),
                (LeafId(1), b"Root.PlayAnimation".to_vec()),
            ]
        );
    }

    #[test]
    fn scans_disabled_and_replaced_unreachable_arena_leaves() {
        let disabled = element(
            b"behaviour",
            vec![attr(b"name", b"OtherBehaviour"), attr(b"enabled", b"false")],
            Vec::new(),
        );
        let root = state(vec![
            expression(vec![named_leaf(b"condition", b"HasIntent")]),
            expression(vec![named_leaf(b"condition", b"HasIntent")]),
            disabled,
        ]);
        let mut factory = Factory::with_payloads([
            MotionGraphLeafPayload::RegisteredOpaque(()),
            has_intent(),
            MotionGraphLeafPayload::RegisteredOpaque(()),
        ]);
        let graph = build(root, &mut factory);
        assert!(!graph.leaves()[2].base.enabled);

        let report = preflight_motion_graph_capabilities(&graph).unwrap();
        assert_eq!(report.leaf_count, 3);
        assert_eq!(
            report
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.leaf)
                .collect::<Vec<_>>(),
            [LeafId(0), LeafId(2)]
        );
    }

    #[test]
    fn unprocessed_graph_is_distinct_from_leaf_coverage_diagnostics() {
        let root = state(vec![named_leaf(b"behaviour", b"PlayAnimation")]);
        let mut factory = Factory::with_payloads([play_animation()]);
        let graph = StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict)
            .expect("fixture construction");

        assert_eq!(
            preflight_motion_graph_capabilities(&graph),
            Err(MotionGraphCapabilityPreflightError::GraphNotProcessed)
        );
    }

    #[test]
    fn malformed_identity_reason_precedence_is_total_and_deterministic() {
        let cases = [
            (
                b"Has\0Intent".as_slice(),
                b"Has\0Intent".as_slice(),
                None,
                MotionGraphCapabilityUnsupportedReason::RequestedEmbeddedNul,
            ),
            (
                b"HasIntent".as_slice(),
                b"HasIntent\0".as_slice(),
                None,
                MotionGraphCapabilityUnsupportedReason::CanonicalEmbeddedNul,
            ),
            (
                b"Other".as_slice(),
                HAS_INTENT_REGISTERED_NAME,
                None,
                MotionGraphCapabilityUnsupportedReason::RequestedNameMismatch,
            ),
            (
                HAS_INTENT_REGISTERED_NAME,
                HAS_INTENT_REGISTERED_NAME,
                None,
                MotionGraphCapabilityUnsupportedReason::MissingPayload,
            ),
        ];
        for (requested, canonical, payload, expected) in cases {
            let (_, actual) = classify_leaf(LeafKind::Condition, requested, canonical, payload)
                .expect("malformed leaf must be rejected");
            assert_eq!(actual, expected);
        }

        let (required, reason) = classify_leaf(
            LeafKind::Behaviour,
            HAS_INTENT_REGISTERED_NAME,
            HAS_INTENT_REGISTERED_NAME,
            Some(MotionGraphLeafVariant::HasIntent),
        )
        .expect("wrong-kind canonical identity must be rejected");
        assert_eq!(
            required,
            RequiredMotionGraphCapability::RegisteredLeaf {
                kind: LeafKind::Behaviour,
                registered_name: HAS_INTENT_REGISTERED_NAME.to_vec(),
            }
        );
        assert_eq!(
            reason,
            MotionGraphCapabilityUnsupportedReason::UnsupportedIdentity
        );
    }
}
