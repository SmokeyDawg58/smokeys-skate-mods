//! Mounted typed-construction gate for `MotionGraph_OnBoard.stategraph`.
//!
//! This proves the recovered constructors, direct `Param` routing, and the
//! structural `Process` traversal. The other registered leaves deliberately
//! remain opaque, and this gate does not execute activation or a frame tick.

use crate::{play_animation_corpus::PlayAnimationCorpusSummary, state_graph_registry_catalog};
use skate2_core::{
    generated_onboard_state_graph::GeneratedOnboardEnablement,
    graph_asset_cache::GraphCacheKey,
    motion_graph_capability_preflight::{
        MotionGraphCapabilityDiagnostic, MotionGraphCapabilityUnsupportedReason,
        RequiredMotionGraphCapability, preflight_motion_graph_capabilities,
    },
    motion_graph_conditions::{
        ComparisonOperator, HAS_INTENT_CORPUS_ALWAYS_COND, HAS_INTENT_CORPUS_DISTINCT_INTENTS,
        HAS_INTENT_CORPUS_OCCURRENCES, HAS_INTENT_CORPUS_OCCURRENCES_SHA256,
        HAS_INTENT_CORPUS_OP_COUNTS, HAS_INTENT_CORPUS_PRE_COND, WILL_EXPIRE_CORPUS_OCCURRENCES,
        WILL_EXPIRE_CORPUS_OCCURRENCES_SHA256, WILL_EXPIRE_CORPUS_WAIT_DEFAULT_TRUE,
        WILL_EXPIRE_CORPUS_WAIT_FALSE,
    },
    motion_graph_leaf_factory::{
        MotionGraphLeafFactory, MotionGraphLeafPayload, MotionGraphLeafVariant,
        PLAY_ANIMATION_REGISTERED_NAME,
    },
    play_animation::AnimationTransitionPlacement,
    state_graph_controller_route::{
        DynamicStateGraphAsset, StateGraphControllerRoute, route_state_graph_controller,
    },
    state_graph_runtime::{
        BaseId, ConditionMask, ConstructionEvent, ExpressionChildId, ExpressionId,
        FactoryMissPolicy, LeafCreateRequest, LeafId, LeafKind, LeafProcessRequest, ParamRequest,
        ProcessEvent, StateGraphLeafFactory, StateGraphRuntime, StateId, TransitionId,
    },
};
use skate2_formats::{
    mount::MountSet,
    state_graph::{StateGraphBinaryAttribute, StateGraphBinaryDocument, StateGraphBinaryElement},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::Write as _,
    io,
};

const MOTION_GRAPH_PATH: &str = r"data\state\MotionGraph_OnBoard.stategraph";
const MOTION_GRAPH_LOGICAL_REQUEST: &str = "state/MotionGraph_OnBoard.xml";
const MOTION_GRAPH_BYTES: usize = 641_241;
const MOTION_GRAPH_SHA256: &str =
    "E5F317CCAC9FAD0D126B3DF6810C9602CBB081F23FE39FB44662C92D4F13F7ED";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MotionGraphTypedCorpusSummary {
    pub has_intent: usize,
    pub will_expire: usize,
    pub play_animation: usize,
    pub play_remapped_animation: usize,
    pub registered_opaque: usize,
    pub params: usize,
    pub factory_misses: usize,
    pub process_events: usize,
    pub processed_leaves: usize,
    pub unsupported_conditions: usize,
    pub unsupported_behaviours: usize,
    pub unsupported_hooks: usize,
    pub unsupported_selector_keys: usize,
}

impl MotionGraphTypedCorpusSummary {
    #[must_use]
    pub const fn matches_shipped(&self) -> bool {
        self.has_intent == 815
            && self.will_expire == 780
            && self.play_animation == 807
            && self.play_remapped_animation == 0
            && self.registered_opaque == 2_902
            && self.params == 165
            && self.factory_misses == 0
            && self.process_events == 10_437
            && self.processed_leaves == 5_304
            && self.unsupported_conditions == 1_173
            && self.unsupported_behaviours == 1_662
            && self.unsupported_hooks == 67
            && self.unsupported_selector_keys == 167
    }
}

/// Retained, fully constructed mounted `MotionGraph` together with the exact
/// logical/cache/compiled route identities used to reach it.
///
/// This owner deliberately exposes neither the arena nor an unchecked
/// controller-construction path. Its capability report remains a refusal for
/// the current shipped corpus. The app reaches this owner through an explicit
/// diagnostic Motion-enable=0 Dynamic fallback, not the matched generated
/// default.
pub struct MountedProcessedMotionGraph {
    dynamic: DynamicStateGraphAsset<StateGraphRuntime<MotionGraphLeafPayload<OpaqueLeaf>>>,
    summary: MotionGraphTypedCorpusSummary,
    capability_report:
        skate2_core::motion_graph_capability_preflight::MotionGraphCapabilityPreflightReport,
}

impl MountedProcessedMotionGraph {
    #[must_use]
    pub const fn summary(&self) -> MotionGraphTypedCorpusSummary {
        self.summary
    }

    #[must_use]
    pub fn logical_request(&self) -> &str {
        self.dynamic.logical_request()
    }

    #[must_use]
    pub const fn cache_key(&self) -> &GraphCacheKey {
        self.dynamic.cache_key()
    }

    #[must_use]
    pub fn compiled_path(&self) -> &str {
        self.dynamic.compiled_path()
    }

    #[must_use]
    pub fn capability_report(
        &self,
    ) -> &skate2_core::motion_graph_capability_preflight::MotionGraphCapabilityPreflightReport {
        &self.capability_report
    }
}

#[derive(Debug)]
struct OpaqueLeaf {
    leaf: LeafId,
    kind: LeafKind,
    registered_name: Vec<u8>,
}

struct CatalogFallback {
    conditions: Vec<Vec<u8>>,
    behaviours: Vec<Vec<u8>>,
    hooks: Vec<Vec<u8>>,
    opaque_params: usize,
    processed: Vec<LeafId>,
}

impl CatalogFallback {
    fn new() -> Self {
        let owned = |kind| {
            state_graph_registry_catalog::registered_names(kind)
                .iter()
                .map(|name| name.to_vec())
                .collect()
        };
        Self {
            conditions: owned(state_graph_registry_catalog::StateGraphFactoryKind::Condition),
            behaviours: owned(state_graph_registry_catalog::StateGraphFactoryKind::Behaviour),
            hooks: owned(state_graph_registry_catalog::StateGraphFactoryKind::Hook),
            opaque_params: 0,
            processed: Vec::new(),
        }
    }
}

impl StateGraphLeafFactory for CatalogFallback {
    type Payload = OpaqueLeaf;
    type Error = io::Error;

    fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
        match kind {
            LeafKind::Condition => &self.conditions,
            LeafKind::Behaviour => &self.behaviours,
            LeafKind::Hook => &self.hooks,
        }
    }

    fn create(&mut self, request: LeafCreateRequest<'_>) -> Result<Self::Payload, Self::Error> {
        Ok(OpaqueLeaf {
            leaf: request.leaf,
            kind: request.kind,
            registered_name: request.registered_name.to_vec(),
        })
    }

    fn add_param(
        &mut self,
        _request: ParamRequest<'_, Self::Payload>,
    ) -> Result<bool, Self::Error> {
        self.opaque_params += 1;
        Ok(true)
    }

    fn process_leaf(
        &mut self,
        request: LeafProcessRequest<'_, Self::Payload>,
    ) -> Result<(), Self::Error> {
        if request.payload.leaf != request.leaf
            || request.payload.kind != request.kind
            || request.payload.registered_name != request.registered_name
        {
            return Err(io::Error::other(
                "opaque MotionGraph Process identity differed",
            ));
        }
        self.processed.push(request.leaf);
        Ok(())
    }
}

/// Forces the diagnostic Dynamic route, then constructs, structurally
/// processes, and retains the exact mounted onboard graph. The matched default
/// remains the generated Motion controller and is not substituted here.
///
/// # Errors
///
/// Returns mounted-data, format, typed-constructor, registry, attachment,
/// traversal, or frozen-corpus mismatch.
#[allow(clippy::too_many_lines)]
pub fn load(
    mounts: &MountSet,
    play_oracle: &PlayAnimationCorpusSummary,
) -> Result<MountedProcessedMotionGraph, Box<dyn Error>> {
    route_with(|compiled_path| load_dynamic(mounts, play_oracle, compiled_path))
}

fn route_with<F>(load_dynamic: F) -> Result<MountedProcessedMotionGraph, Box<dyn Error>>
where
    F: FnOnce(&str) -> Result<LoadedMotionGraph, Box<dyn Error>>,
{
    let route = route_state_graph_controller(
        MOTION_GRAPH_LOGICAL_REQUEST,
        true,
        false,
        GeneratedOnboardEnablement::onboard(1, 0),
        load_dynamic,
    )
    .map_err(|error| format!("MotionGraph controller route failed: {error}"))?;
    let StateGraphControllerRoute::Dynamic(dynamic) = route else {
        return Err("disabled Motion delegate did not select the Dynamic fallback".into());
    };
    if dynamic.logical_request() != MOTION_GRAPH_LOGICAL_REQUEST
        || dynamic.cache_key().normalized_path() != r"data\state\MotionGraph_OnBoard.xml"
        || dynamic.cache_key().hash() != 0x07d5_e6bc
        || dynamic.compiled_path() != MOTION_GRAPH_PATH
    {
        return Err("MotionGraph logical/cache/compiled route identity differed".into());
    }
    let mut summary = None;
    let mut capability_report = None;
    let dynamic = dynamic.map_payload(|loaded| {
        summary = Some(loaded.summary);
        capability_report = Some(loaded.capability_report);
        loaded.graph
    });
    Ok(MountedProcessedMotionGraph {
        dynamic,
        summary: summary.expect("Dynamic payload mapping always stores the summary"),
        capability_report: capability_report
            .expect("Dynamic payload mapping always stores the capability report"),
    })
}

struct LoadedMotionGraph {
    graph: StateGraphRuntime<MotionGraphLeafPayload<OpaqueLeaf>>,
    summary: MotionGraphTypedCorpusSummary,
    capability_report:
        skate2_core::motion_graph_capability_preflight::MotionGraphCapabilityPreflightReport,
}

#[allow(clippy::too_many_lines)]
fn load_dynamic(
    mounts: &MountSet,
    play_oracle: &PlayAnimationCorpusSummary,
    compiled_path: &str,
) -> Result<LoadedMotionGraph, Box<dyn Error>> {
    if compiled_path != MOTION_GRAPH_PATH {
        return Err(format!("MotionGraph compiled sibling differed: {compiled_path}").into());
    }
    if !play_oracle.matches_shipped() {
        return Err("PlayAnimation digest/distribution oracle was not verified".into());
    }
    let bytes = mounts.read_path(compiled_path)?;
    construct_exact_payload(&bytes, play_oracle)
}

#[allow(clippy::too_many_lines)]
fn construct_exact_payload(
    bytes: &[u8],
    play_oracle: &PlayAnimationCorpusSummary,
) -> Result<LoadedMotionGraph, Box<dyn Error>> {
    let sha = state_graph_registry_catalog::sha256_hex(bytes);
    if bytes.len() != MOTION_GRAPH_BYTES || sha != MOTION_GRAPH_SHA256 {
        return Err(format!(
            "{MOTION_GRAPH_PATH} identity differed: {} bytes / {sha}",
            bytes.len()
        )
        .into());
    }
    let document = StateGraphBinaryDocument::parse_exact(bytes)?;
    if document.root.node_count() != 10_602
        || document.root.attribute_count() != 20_419
        || document.root.maximum_depth() != 12
    {
        return Err("MotionGraph_OnBoard decoded tree census differed".into());
    }
    let has_intent_sha = condition_occurrence_sha(&document.root, b"HasIntent")?;
    let will_expire_sha = condition_occurrence_sha(&document.root, b"WillExpire")?;
    if has_intent_sha != HAS_INTENT_CORPUS_OCCURRENCES_SHA256
        || will_expire_sha != WILL_EXPIRE_CORPUS_OCCURRENCES_SHA256
    {
        return Err(format!(
            "MotionGraph condition occurrence digests differed: {has_intent_sha} / {will_expire_sha}"
        )
        .into());
    }

    let fallback = CatalogFallback::new();
    let mut factory = MotionGraphLeafFactory::new(fallback)?;
    let mut graph = StateGraphRuntime::build(
        &document.root,
        &mut factory,
        FactoryMissPolicy::NativeSafeNull,
    )?;
    let mut summary = summarize_and_validate_payloads(&graph, play_oracle)?;

    if graph.states().len() != 1_354
        || graph.transitions().len() != 1_724
        || graph.expressions().len() != 2_055
        || graph.leaves().len() != 5_304
        || graph.params().len() != 165
    {
        return Err("MotionGraph_OnBoard typed arena census differed".into());
    }
    summary.params = graph.params().len();
    summary.factory_misses = graph
        .construction_events()
        .iter()
        .filter(|event| matches!(event, ConstructionEvent::FactoryMiss { .. }))
        .count();
    validate_param_routing(&graph, factory.fallback().opaque_params)?;

    let expected_events = expected_process_events(&graph)?;
    let actual_events = graph.process_after_build(&mut factory)?;
    if actual_events != expected_events {
        return Err("MotionGraph_OnBoard Process traversal order differed".into());
    }
    let actual_leaf_order = actual_events
        .iter()
        .filter_map(|event| match event {
            ProcessEvent::Leaf(leaf) => Some(*leaf),
            _ => None,
        })
        .collect::<Vec<_>>();
    let opaque_leaf_order = actual_leaf_order
        .iter()
        .copied()
        .filter(|leaf| {
            matches!(
                graph.leaves()[leaf.0].payload(),
                Some(MotionGraphLeafPayload::RegisteredOpaque(_))
            )
        })
        .collect::<Vec<_>>();
    if factory.fallback().processed != opaque_leaf_order {
        return Err("opaque MotionGraph Process delegation order differed".into());
    }
    summary.process_events = actual_events.len();
    summary.processed_leaves = actual_leaf_order.len();
    let (capability, capability_report) = validate_capability_refusal(&graph)?;
    summary.unsupported_conditions = capability.conditions;
    summary.unsupported_behaviours = capability.behaviours;
    summary.unsupported_hooks = capability.hooks;
    summary.unsupported_selector_keys = capability.selector_keys;
    Ok(LoadedMotionGraph {
        graph,
        summary,
        capability_report,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CapabilityRefusalSummary {
    conditions: usize,
    behaviours: usize,
    hooks: usize,
    selector_keys: usize,
}

const UNSUPPORTED_GROUPED_SHA256: &str =
    "067590A558AA8EE01919FB67EE7B70045C814804E1B83943FE858D68A84BE456";
const UNSUPPORTED_ORDERED_SHA256: &str =
    "3FD010B7611FF8662AB21779B51B7290828C0FBE9EAC3DF2178CC782C6458600";

#[allow(clippy::too_many_lines)]
fn validate_capability_refusal(
    graph: &StateGraphRuntime<MotionGraphLeafPayload<OpaqueLeaf>>,
) -> Result<
    (
        CapabilityRefusalSummary,
        skate2_core::motion_graph_capability_preflight::MotionGraphCapabilityPreflightReport,
    ),
    Box<dyn Error>,
> {
    let report = preflight_motion_graph_capabilities(graph)?;
    if report.leaf_count != 5_304 || report.has_full_leaf_coverage() {
        return Err("MotionGraph capability preflight did not produce the required refusal".into());
    }

    let mut grouped = [
        BTreeMap::<Vec<u8>, usize>::new(),
        BTreeMap::<Vec<u8>, usize>::new(),
        BTreeMap::<Vec<u8>, usize>::new(),
    ];
    let mut ordered_rows = Vec::new();
    for diagnostic in &report.diagnostics {
        validate_capability_diagnostic(graph, diagnostic)?;
        let kind_index = capability_kind_index(diagnostic.kind);
        *grouped[kind_index]
            .entry(diagnostic.canonical_registered_name.clone())
            .or_default() += 1;
        ordered_rows.extend_from_slice(diagnostic.leaf.0.to_string().as_bytes());
        ordered_rows.push(b'\t');
        ordered_rows.extend_from_slice(capability_kind_name(diagnostic.kind));
        ordered_rows.push(b'\t');
        ordered_rows.extend_from_slice(&diagnostic.canonical_registered_name);
        ordered_rows.push(b'\n');
    }

    let opaque_ids = graph
        .leaves()
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            matches!(
                node.payload(),
                Some(MotionGraphLeafPayload::RegisteredOpaque(_))
            )
            .then_some(LeafId(index))
        })
        .collect::<Vec<_>>();
    let diagnostic_ids = report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.leaf)
        .collect::<Vec<_>>();
    if diagnostic_ids != opaque_ids
        || diagnostic_ids.first() != Some(&LeafId(0))
        || diagnostic_ids.last() != Some(&LeafId(5_303))
        || report
            .diagnostics
            .first()
            .map(|item| item.canonical_registered_name.as_slice())
            != Some(b"PhysFilteredState")
        || report
            .diagnostics
            .last()
            .map(|item| item.canonical_registered_name.as_slice())
            != Some(b"IsInDebugAnimationsMode")
    {
        return Err("MotionGraph capability refusal membership/order differed".into());
    }

    let counts = grouped.each_ref().map(|entries| entries.values().sum());
    let selector_counts = grouped.each_ref().map(BTreeMap::len);
    if counts != [1_173, 1_662, 67] || selector_counts != [73, 90, 4] {
        return Err("MotionGraph capability refusal census differed".into());
    }
    let expected_hooks = [
        (b"GrabSlide".as_slice(), 2_usize),
        (b"InitializeLocomotion".as_slice(), 6),
        (b"MongoPushToAntic".as_slice(), 2),
        (b"OverideNextAnimTransitionHook".as_slice(), 57),
    ];
    if grouped[2]
        .iter()
        .map(|(name, count)| (name.as_slice(), *count))
        .ne(expected_hooks)
    {
        return Err("MotionGraph unsupported Hook groups differed".into());
    }

    let mut grouped_rows = Vec::new();
    for (kind_index, entries) in grouped.iter().enumerate() {
        for (name, count) in entries {
            grouped_rows.extend_from_slice(capability_kind_name_from_index(kind_index));
            grouped_rows.push(b'\t');
            grouped_rows.extend_from_slice(name);
            grouped_rows.push(b'\t');
            grouped_rows.extend_from_slice(count.to_string().as_bytes());
            grouped_rows.push(b'\n');
        }
    }
    if grouped_rows.len() != 4_820
        || state_graph_registry_catalog::sha256_hex(&grouped_rows) != UNSUPPORTED_GROUPED_SHA256
        || ordered_rows.len() != 86_183
        || state_graph_registry_catalog::sha256_hex(&ordered_rows) != UNSUPPORTED_ORDERED_SHA256
    {
        return Err("MotionGraph capability refusal digest differed".into());
    }

    Ok((
        CapabilityRefusalSummary {
            conditions: counts[0],
            behaviours: counts[1],
            hooks: counts[2],
            selector_keys: selector_counts.into_iter().sum(),
        },
        report,
    ))
}

fn validate_capability_diagnostic(
    graph: &StateGraphRuntime<MotionGraphLeafPayload<OpaqueLeaf>>,
    diagnostic: &MotionGraphCapabilityDiagnostic,
) -> Result<(), Box<dyn Error>> {
    let node = graph
        .leaves()
        .get(diagnostic.leaf.0)
        .ok_or_else(|| io::Error::other("capability diagnostic LeafId was out of range"))?;
    if diagnostic.reason != MotionGraphCapabilityUnsupportedReason::RegisteredOpaque
        || diagnostic.kind != node.kind
        || diagnostic.path != node.base.path
        || diagnostic.parent != node.base.parent
        || diagnostic.requested_name != node.base.name
        || diagnostic.canonical_registered_name != node.registered_name
        || diagnostic.payload_variant != Some(MotionGraphLeafVariant::RegisteredOpaque)
        || diagnostic.required
            != (RequiredMotionGraphCapability::RegisteredLeaf {
                kind: node.kind,
                registered_name: node.registered_name.clone(),
            })
        || !matches!(
            node.payload(),
            Some(MotionGraphLeafPayload::RegisteredOpaque(_))
        )
    {
        return Err("MotionGraph capability diagnostic identity differed".into());
    }
    Ok(())
}

const fn capability_kind_index(kind: LeafKind) -> usize {
    match kind {
        LeafKind::Condition => 0,
        LeafKind::Behaviour => 1,
        LeafKind::Hook => 2,
    }
}

const fn capability_kind_name(kind: LeafKind) -> &'static [u8] {
    capability_kind_name_from_index(capability_kind_index(kind))
}

const fn capability_kind_name_from_index(index: usize) -> &'static [u8] {
    match index {
        0 => b"condition",
        1 => b"behaviour",
        2 => b"hook",
        _ => b"invalid",
    }
}

fn summarize_and_validate_payloads(
    graph: &StateGraphRuntime<MotionGraphLeafPayload<OpaqueLeaf>>,
    play_oracle: &PlayAnimationCorpusSummary,
) -> Result<MotionGraphTypedCorpusSummary, Box<dyn Error>> {
    let mut summary = MotionGraphTypedCorpusSummary {
        has_intent: 0,
        will_expire: 0,
        play_animation: 0,
        play_remapped_animation: 0,
        registered_opaque: 0,
        params: 0,
        factory_misses: 0,
        process_events: 0,
        processed_leaves: 0,
        unsupported_conditions: 0,
        unsupported_behaviours: 0,
        unsupported_hooks: 0,
        unsupported_selector_keys: 0,
    };
    let mut intents = BTreeSet::new();
    let mut intent_operators = [0_usize; 7];
    let mut intent_masks = [0_usize; 2];
    let mut will_wait = [0_usize; 2];
    let mut transition_types = [0_usize; 5];
    let mut switches = 0;
    let mut mirrors = 0;
    let mut no_boards = 0;
    let mut blend_current = 0;
    let mut under = 0;
    let mut channel_weights = 0;
    let mut param_types = [0_usize; 3];

    for leaf in graph.leaves() {
        let payload = leaf
            .payload()
            .ok_or_else(|| io::Error::other("attached MotionGraph leaf lacked payload"))?;
        match payload {
            MotionGraphLeafPayload::HasIntent(payload) => {
                summary.has_intent += 1;
                intents.insert(payload.intent_name.clone());
                intent_operators[comparison_index(payload.comparison.operator)] += 1;
                match leaf.mask {
                    Some(ConditionMask::PreCond) => intent_masks[0] += 1,
                    Some(ConditionMask::Always) => intent_masks[1] += 1,
                    _ => return Err("HasIntent mask corpus differed".into()),
                }
            }
            MotionGraphLeafPayload::WillExpire(payload) => {
                summary.will_expire += 1;
                will_wait[usize::from(payload.wait_for_transitions())] += 1;
                if !payload.time_tag().is_empty() || leaf.mask != Some(ConditionMask::PreCond) {
                    return Err("WillExpire tag/mask corpus differed".into());
                }
            }
            MotionGraphLeafPayload::PlayAnimation(payload) => {
                summary.play_animation += 1;
                transition_types[payload.transition.transition_type as usize] += 1;
                switches += usize::from(payload.has_switch_anim);
                mirrors += usize::from(payload.has_mirror_anim);
                no_boards += usize::from(payload.has_no_board_anim);
                blend_current += usize::from(payload.transition.blend_with_current_frame);
                under += usize::from(
                    payload.transition.placement == AnimationTransitionPlacement::Under,
                );
                channel_weights += usize::from(payload.transition.use_channel_from_weights);
                for param in payload.params() {
                    param_types[param.source_type as usize] += 1;
                }
            }
            MotionGraphLeafPayload::PlayRemappedAnimation(_) => {
                summary.play_remapped_animation += 1;
            }
            MotionGraphLeafPayload::RegisteredOpaque(_) => summary.registered_opaque += 1,
        }
    }
    if summary.has_intent != HAS_INTENT_CORPUS_OCCURRENCES
        || intents.len() != HAS_INTENT_CORPUS_DISTINCT_INTENTS
        || intent_operators != HAS_INTENT_CORPUS_OP_COUNTS
        || intent_masks != [HAS_INTENT_CORPUS_PRE_COND, HAS_INTENT_CORPUS_ALWAYS_COND]
        || summary.will_expire != WILL_EXPIRE_CORPUS_OCCURRENCES
        || will_wait
            != [
                WILL_EXPIRE_CORPUS_WAIT_FALSE,
                WILL_EXPIRE_CORPUS_WAIT_DEFAULT_TRUE,
            ]
    {
        return Err("typed MotionGraph condition distribution differed".into());
    }
    if summary.play_animation != play_oracle.behaviours
        || transition_types != play_oracle.transition_type_counts
        || switches != play_oracle.switch_anim_present
        || mirrors != play_oracle.mirror_anim_present
        || no_boards != play_oracle.no_board_anim_present
        || blend_current != play_oracle.blend_with_current_frame_true
        || under != play_oracle.transition_under_true
        || channel_weights != play_oracle.use_channel_from_weights_true
        || param_types != play_oracle.param_type_counts
    {
        return Err("typed PlayAnimation distribution differed from digest gate".into());
    }
    Ok(summary)
}

fn validate_param_routing(
    graph: &StateGraphRuntime<MotionGraphLeafPayload<OpaqueLeaf>>,
    opaque_param_calls: usize,
) -> Result<(), Box<dyn Error>> {
    let params_target_play_animation = graph.params().iter().all(|param| {
        let Some(BaseId::Leaf(leaf)) = param.parent else {
            return false;
        };
        param.handled_by_override
            && graph.leaves()[leaf.0].registered_name == PLAY_ANIMATION_REGISTERED_NAME
            && matches!(
                graph.leaves()[leaf.0].payload(),
                Some(MotionGraphLeafPayload::PlayAnimation(_))
            )
    });
    let stored_params = graph
        .leaves()
        .iter()
        .filter_map(|leaf| match leaf.payload() {
            Some(MotionGraphLeafPayload::PlayAnimation(payload)) => Some(payload.params().len()),
            _ => None,
        })
        .sum::<usize>();
    if !params_target_play_animation || stored_params != 165 || opaque_param_calls != 0 {
        return Err("MotionGraph Param routing differed".into());
    }
    Ok(())
}

const fn comparison_index(operator: ComparisonOperator) -> usize {
    match operator {
        ComparisonOperator::Nop => 0,
        ComparisonOperator::Equals => 1,
        ComparisonOperator::NotEqual => 2,
        ComparisonOperator::GreaterThan => 3,
        ComparisonOperator::LessThan => 4,
        ComparisonOperator::GreaterEqual => 5,
        ComparisonOperator::LessEqual => 6,
    }
}

fn expected_process_events<P>(
    graph: &StateGraphRuntime<P>,
) -> Result<Vec<ProcessEvent>, Box<dyn Error>> {
    fn expression<P>(
        graph: &StateGraphRuntime<P>,
        id: ExpressionId,
        events: &mut Vec<ProcessEvent>,
    ) {
        events.push(ProcessEvent::Expression(id));
        for child in &graph.expressions()[id.0].children {
            match child {
                ExpressionChildId::Expression(child) => expression(graph, *child, events),
                ExpressionChildId::Condition(child) => events.push(ProcessEvent::Leaf(*child)),
            }
        }
    }
    fn transition<P>(
        graph: &StateGraphRuntime<P>,
        id: TransitionId,
        events: &mut Vec<ProcessEvent>,
    ) {
        events.push(ProcessEvent::Transition(id));
        if let Some(root) = graph.transitions()[id.0].expression {
            expression(graph, root, events);
        }
        events.extend(
            graph.transitions()[id.0]
                .hooks
                .iter()
                .copied()
                .map(ProcessEvent::Leaf),
        );
    }
    fn state<P>(graph: &StateGraphRuntime<P>, id: StateId, events: &mut Vec<ProcessEvent>) {
        events.push(ProcessEvent::State(id));
        if let Some(root) = graph.states()[id.0].expression {
            expression(graph, root, events);
        }
        for child in &graph.states()[id.0].children {
            state(graph, *child, events);
        }
        events.extend(
            graph.states()[id.0]
                .behaviours
                .iter()
                .copied()
                .map(ProcessEvent::Leaf),
        );
        for transition_id in &graph.states()[id.0].transitions {
            transition(graph, *transition_id, events);
        }
    }

    let root = graph
        .root()
        .ok_or_else(|| io::Error::other("MotionGraph_OnBoard lacked root"))?;
    let mut events = Vec::new();
    state(graph, root, &mut events);
    Ok(events)
}

fn condition_occurrence_sha(
    root: &StateGraphBinaryElement,
    selector: &[u8],
) -> Result<String, Box<dyn Error>> {
    fn visit(
        element: &StateGraphBinaryElement,
        selector: &[u8],
        locator: &mut String,
        state_path: &mut Vec<Vec<u8>>,
        preorder_index: &mut usize,
        first: &mut bool,
        output: &mut String,
    ) -> Result<(), Box<dyn Error>> {
        let is_state = element.name == b"state";
        if is_state {
            state_path.push(
                exact_attribute_text(element, b"name")
                    .unwrap_or(b"__unknown__")
                    .to_vec(),
            );
        }
        let current_index = *preorder_index;
        *preorder_index += 1;
        if element.name == b"condition" && exact_attribute_text(element, b"name") == Some(selector)
        {
            if !*first {
                output.push(',');
            }
            *first = false;
            output.push_str("{\"attributes\":");
            write_attributes_json(output, &element.attributes)?;
            output.push_str(",\"children\":[");
            for (index, child) in element.children.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_json_ascii(output, &child.name)?;
            }
            output.push_str("],\"locator\":");
            write_json_ascii(output, locator.as_bytes())?;
            write!(
                output,
                ",\"preorder_index\":{current_index},\"state_path\":["
            )?;
            for (index, component) in state_path.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_json_ascii(output, component)?;
            }
            output.push_str("]}");
        }

        let parent_length = locator.len();
        for (child_index, child) in element.children.iter().enumerate() {
            write!(locator, ".{child_index}")?;
            visit(
                child,
                selector,
                locator,
                state_path,
                preorder_index,
                first,
                output,
            )?;
            locator.truncate(parent_length);
        }
        if is_state {
            state_path.pop();
        }
        Ok(())
    }

    let mut output = "[".to_owned();
    let mut locator = "0".to_owned();
    let mut state_path = Vec::new();
    let mut preorder_index = 0;
    let mut first = true;
    visit(
        root,
        selector,
        &mut locator,
        &mut state_path,
        &mut preorder_index,
        &mut first,
        &mut output,
    )?;
    output.push(']');
    Ok(state_graph_registry_catalog::sha256_hex(output.as_bytes()))
}

fn exact_attribute_text<'a>(element: &'a StateGraphBinaryElement, key: &[u8]) -> Option<&'a [u8]> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .map(|attribute| attribute.text.as_slice())
}

fn write_attributes_json(
    output: &mut String,
    attributes: &[StateGraphBinaryAttribute],
) -> Result<(), Box<dyn Error>> {
    output.push('[');
    for (index, attribute) in attributes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            output,
            "{{\"boolean_raw\":{},\"float_bits\":\"0x{:08X}\",\"key\":",
            attribute.boolean_raw, attribute.float_bits
        )?;
        write_json_ascii(output, &attribute.key)?;
        write!(
            output,
            ",\"key_hash\":\"0x{:08X}\",\"text\":",
            attribute.key_hash
        )?;
        write_json_ascii(output, &attribute.text)?;
        output.push('}');
    }
    output.push(']');
    Ok(())
}

fn write_json_ascii(output: &mut String, value: &[u8]) -> Result<(), Box<dyn Error>> {
    output.push('"');
    for &byte in value {
        match byte {
            b'"' => output.push_str("\\\""),
            b'\\' => output.push_str("\\\\"),
            0x08 => output.push_str("\\b"),
            0x09 => output.push_str("\\t"),
            0x0a => output.push_str("\\n"),
            0x0c => output.push_str("\\f"),
            0x0d => output.push_str("\\r"),
            0x00..=0x1f => write!(output, "\\u{byte:04x}")?,
            0x20..=0x7e => output.push(char::from(byte)),
            0x7f => output.push_str("\\u007f"),
            _ => {
                return Err(format!(
                    "compiled StateGraph JSON field contained non-ASCII byte {byte:02x}"
                )
                .into());
            }
        }
    }
    output.push('"');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};

    /// External real-data gate. The payload remains outside the repository.
    #[test]
    #[ignore = "requires SKATE2_MOTION_GRAPH_PATH"]
    fn exact_external_payload_runs_logical_route_and_retains_full_refusal() {
        let path = env::var("SKATE2_MOTION_GRAPH_PATH")
            .expect("SKATE2_MOTION_GRAPH_PATH must name the extracted real payload");
        let bytes = fs::read(path).unwrap();
        let play_oracle = crate::play_animation_corpus::load_exact_payload(&bytes).unwrap();
        let mounted = route_with(|compiled_path| {
            assert_eq!(compiled_path, MOTION_GRAPH_PATH);
            construct_exact_payload(&bytes, &play_oracle)
        })
        .unwrap();

        assert!(mounted.summary().matches_shipped());
        assert_eq!(mounted.logical_request(), MOTION_GRAPH_LOGICAL_REQUEST);
        assert_eq!(mounted.cache_key().hash(), 0x07d5_e6bc);
        assert_eq!(mounted.compiled_path(), MOTION_GRAPH_PATH);
        assert_eq!(mounted.capability_report().leaf_count, 5_304);
        assert_eq!(mounted.capability_report().diagnostics.len(), 2_902);
    }
}
