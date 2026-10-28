//! Mounted structural construction/finalization gate for all 36 `StateGraph`s.
//!
//! Opaque leaves prove registered construction, attachment, exact callback
//! inputs, Param routing, and structural Process order. They do not implement
//! concrete gameplay semantics for the 466 selector names exercised by this
//! corpus. Target resolution must succeed, but the frozen oracle does not
//! independently identify every resolved target ID.

use crate::{state_graph_corpus::EXPECTED, state_graph_registry_catalog};
use skate2_core::state_graph_runtime::{
    BaseId, ConditionMask, ConstructionEvent, ExpressionChildId, ExpressionId, ExpressionOperator,
    FactoryMissPolicy, Interruptable, LeafCreateRequest, LeafId, LeafKind, LeafProcessRequest,
    ParamRequest, ProcessEvent, StateGraphLeafFactory, StateGraphRuntime, StateId, TransitionId,
    TransitionPriority,
};
use skate2_formats::{
    mount::MountSet,
    state_graph::{StateGraphBinaryAttribute, StateGraphBinaryDocument, StateGraphBinaryElement},
};
use std::{collections::BTreeMap, error::Error, io};

const PAYLOAD_SHA256: [&str; 36] = [
    "4318484E1F05A2962990AD993B474EF780CB280796EFA2C01F5AC789D011C699",
    "1809C28F42879910C5E1B4197AD9908B0EC8DC4CEE11553B3C3EFE6CE2FE69A6",
    "9CFD68BEDD4601C1888C4281F70F7F463E4E644AEDB3FD5B9978C116B96B5D28",
    "727ACE23D1A32D771F052B6250C7D04A4685237D4C04301596577629E39398B2",
    "0313612B87F8409708F2ECF965EFE064181B22E8C8C01ACE6CB0847BB905CC1A",
    "92145A90CF3F7B1ED43DFB74374E0B089BF47C6402E6569124A5F166A2D0F8CD",
    "E6B4D5B58D4660CE4B85501DE33E7FEA44D6B06F8AA27916B5235D6FBC081773",
    "C0CC29B30F24B12BF4BC1C7D2C2BE29CA5162991BC6BDCC94B23F1AD16CA9555",
    "8A523FC8C35216F180F806D2F9D77FFF90E853AFFD200A6212F629648A12571A",
    "D850237961D585C0AAC404F3F6D93063C2CBAA47988FA869F74E62F7DF5B7770",
    "4DD2988F045C597B80532C15E50A09C2D999A906E255318AA05C50ED9A5FCEB8",
    "E5F317CCAC9FAD0D126B3DF6810C9602CBB081F23FE39FB44662C92D4F13F7ED",
    "E8814EC53DC5CC8A40B5F3E0E8CE9A59B2BF04DCD5E1864A3741A62143A13726",
    "9FFD7EF3879342CF73502F10FD984A718CF353B0AAF52937AFA2ADCEF53AB200",
    "18EF3838FB6FCEF85D4B3270D7963FCED962CC2DA207DF993B2CDB7688EA16A6",
    "23FFF26C4BB98ED4BD821F49DBDE9937EE925D605D4F5B8D3CECFF3AC2460A9D",
    "FB0F9E37B57C06D9C7DF012BE32B9D4084EA03671D15DB6EBC844B96FA251BE7",
    "4318484E1F05A2962990AD993B474EF780CB280796EFA2C01F5AC789D011C699",
    "088822D87142B9F56D29836ABB073256389873D496671D4F30C504A015DEEC17",
    "9CFD68BEDD4601C1888C4281F70F7F463E4E644AEDB3FD5B9978C116B96B5D28",
    "B7CDD15890E5C31DFB39E4D8979F7E02B6786459487395B24BC5AB85C0DF34C2",
    "0313612B87F8409708F2ECF965EFE064181B22E8C8C01ACE6CB0847BB905CC1A",
    "ABEA8D36CBB99FF3FCCD0CB03EA595A4B7B02BCD06E8E291717984A0D2353E30",
    "726C5E22060298285BCD70E235B672840227BEB95C17A22611147FD0B2986CB6",
    "1689685A74620F67151CB9233F6D045B5E4F85B193385C36BD596BBF5197381D",
    "72E5FC37F4CD788F5762A544C6D22F78D6CFF5356967F18C9F8E651C1072B82C",
    "27324039422A722D9C8491294894172CD77B9E209A7C8A0F169A452A9E543DCF",
    "4FAFFDFE4C68275BAB04A553B15D8E1C0A7A6E3BCA2E856B9EBAF0E87B45ED39",
    "28F487B1BE9DD9E0C3D3B573D459A76DA841BE74B86C9C655B31A62CDABB9791",
    "8254386CA7FC8517E209B4D6A0FC73D2B21EF0359C43119A0E351E7CBECBC4C3",
    "E93DA6DF84796E262D80D6F327C4931857CCBD5C20069729EA67F78DCA279C16",
    "F16CE5EFC132DAF69284C494616A4318AC54EE91AF01875797625131748037A9",
    "DA1B2CB4994F6D53669466EB1220F4108BB5BF7E11041E9981DEF32EE355C91E",
    "94AEE8B4A9C6E058D92ABDAA9270C451568740BDF94EEB7A01EFCABF53B4AFB7",
    "E77DD560CF27CD0D775E04096FBD6C3BC4CB1D7401F4B1813CCBE58A0CEC76A0",
    "FE0A3029A4532FDEEB975A7CDF402B9A9ABC9398F898E8BFA21A1394CD339B93",
];

const SELECTOR_COUNT_SHA256: [&str; 3] = [
    "8C2A75C832F25C526CD2610FA60D21EBC1F8440AD2C5DA49542F4CFAC25AE2CD",
    "70925CC8962C660B822332944723CED3BAEE7D811F0CF499E275E833016E785D",
    "A565576D98717B5476E64412814C5334D834FEF0490AF3ECE53C98F2030DB445",
];
const SELECTOR_DISTINCT: [usize; 3] = [205, 246, 15];
const NAMED_INTERRUPT_TARGETS: [(&[u8], usize); 13] = [
    (b"ActiveChase", 29),
    (b"Air", 2),
    (b"CanThrowHandProp", 2),
    (b"ConversationIsOn", 6),
    (b"Cycle", 4),
    (b"Flee", 2),
    (b"HandPlantGround", 2),
    (b"HoldBeforeChaseStart", 6),
    (b"PatrolZoneBoundary", 3),
    (b"PedestrianAddressWants", 7),
    (b"PedestrianWantToTaze", 1),
    (b"UseCrossWalk", 1),
    (b"WanderMode", 3),
];

const STATES: [usize; 36] = [
    8, 16, 3, 18, 8, 11, 12, 3, 13, 14, 224, 1_354, 9, 19, 572, 2, 134, 8, 16, 3, 15, 8, 11, 12, 3,
    13, 14, 5, 11, 17, 7, 92, 7, 5, 7, 7,
];
const TRANSITIONS: [usize; 36] = [
    10, 1, 1, 8, 9, 1, 1, 0, 2, 3, 129, 1_724, 13, 2, 441, 0, 59, 10, 2, 1, 5, 9, 2, 2, 0, 3, 4, 1,
    5, 8, 1, 0, 0, 0, 0, 0,
];
const EXPRESSIONS: [usize; 36] = [
    9, 11, 1, 14, 9, 8, 8, 1, 8, 9, 357, 2_055, 13, 12, 713, 0, 117, 9, 12, 1, 11, 9, 9, 9, 1, 9,
    10, 3, 10, 17, 5, 84, 5, 2, 5, 5,
];
const CONDITION_REQUESTS: [usize; 36] = [
    12, 11, 1, 14, 9, 8, 8, 1, 8, 9, 473, 2_768, 13, 18, 749, 0, 130, 12, 12, 1, 11, 9, 9, 9, 1, 9,
    10, 3, 10, 17, 5, 112, 7, 2, 7, 7,
];
const BEHAVIOUR_REQUESTS: [usize; 36] = [
    8, 16, 3, 17, 14, 11, 10, 2, 11, 12, 311, 2_469, 7, 34, 472, 1, 201, 8, 18, 3, 15, 14, 12, 12,
    3, 13, 13, 5, 8, 12, 9, 69, 4, 3, 4, 4,
];
const PARAMS: [usize; 36] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 165, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
];
const HOOK_REQUESTS: [usize; 36] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 67, 13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
];
const NAMED_INTERRUPTS: [usize; 36] = [
    0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 54, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
];
const CONDITION_MISSES: [usize; 36] = [
    0, 3, 0, 3, 0, 3, 3, 0, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateGraphRuntimeCorpusSummary {
    pub graphs: usize,
    pub states: usize,
    pub transitions: usize,
    pub expressions: usize,
    pub leaf_requests: usize,
    pub attached_leaves: usize,
    pub params: usize,
    pub factory_misses: usize,
}

impl StateGraphRuntimeCorpusSummary {
    #[must_use]
    pub const fn matches_shipped(&self) -> bool {
        self.graphs == 36
            && self.states == 2_681
            && self.transitions == 2_457
            && self.expressions == 3_561
            && self.leaf_requests == 8_393
            && self.attached_leaves == 8_375
            && self.params == 165
            && self.factory_misses == 18
    }
}

#[derive(Debug)]
struct OpaqueLeaf {
    ordinal: usize,
    leaf: LeafId,
    kind: LeafKind,
    parent: BaseId,
    requested_name: Vec<u8>,
    registered_name: Vec<u8>,
    attributes: Vec<StateGraphBinaryAttribute>,
    params: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CreateObservation {
    leaf: LeafId,
    kind: LeafKind,
    parent: BaseId,
    requested_name: Vec<u8>,
    registered_name: Vec<u8>,
    attributes: Vec<StateGraphBinaryAttribute>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParamObservation {
    leaf: LeafId,
    registered_name: Vec<u8>,
    attributes: Vec<StateGraphBinaryAttribute>,
}

struct CatalogFactory {
    conditions: Vec<Vec<u8>>,
    behaviours: Vec<Vec<u8>>,
    hooks: Vec<Vec<u8>>,
    creates: Vec<CreateObservation>,
    params: Vec<ParamObservation>,
    processed: Vec<LeafId>,
}

impl CatalogFactory {
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
            creates: Vec::new(),
            params: Vec::new(),
            processed: Vec::new(),
        }
    }
}

impl StateGraphLeafFactory for CatalogFactory {
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
        let expected_parent = match request.kind {
            LeafKind::Condition => matches!(request.parent, BaseId::Expression(_)),
            LeafKind::Behaviour => matches!(request.parent, BaseId::State(_)),
            LeafKind::Hook => matches!(request.parent, BaseId::Transition(_)),
        };
        if !expected_parent {
            return Err(io::Error::other("StateGraph leaf parent kind differed"));
        }
        let ordinal = self.creates.len();
        self.creates.push(CreateObservation {
            leaf: request.leaf,
            kind: request.kind,
            parent: request.parent,
            requested_name: request.requested_name.to_vec(),
            registered_name: request.registered_name.to_vec(),
            attributes: request.attributes.to_vec(),
        });
        Ok(OpaqueLeaf {
            ordinal,
            leaf: request.leaf,
            kind: request.kind,
            parent: request.parent,
            requested_name: request.requested_name.to_vec(),
            registered_name: request.registered_name.to_vec(),
            attributes: request.attributes.to_vec(),
            params: 0,
        })
    }

    fn add_param(&mut self, request: ParamRequest<'_, Self::Payload>) -> Result<bool, Self::Error> {
        if request.kind != LeafKind::Behaviour || request.payload.kind != LeafKind::Behaviour {
            return Err(io::Error::other(
                "StateGraph Param did not target a Behaviour",
            ));
        }
        request.payload.params += 1;
        self.params.push(ParamObservation {
            leaf: request.leaf,
            registered_name: request.registered_name.to_vec(),
            attributes: request.attributes.to_vec(),
        });
        Ok(true)
    }

    fn process_leaf(
        &mut self,
        request: LeafProcessRequest<'_, Self::Payload>,
    ) -> Result<(), Self::Error> {
        if request.payload.leaf != request.leaf
            || request.payload.kind != request.kind
            || request.payload.registered_name != request.registered_name
            || request.payload.ordinal >= self.creates.len()
            || request.payload.requested_name.is_empty()
            || request.payload.attributes.is_empty()
        {
            return Err(io::Error::other("StateGraph opaque leaf identity differed"));
        }
        self.processed.push(request.leaf);
        Ok(())
    }
}

#[derive(Default)]
struct ExpectedInputTrace {
    creates: Vec<CreateObservation>,
    params: Vec<ParamObservation>,
    misses: Vec<(LeafKind, Vec<u8>)>,
    pushes: Vec<Option<BaseId>>,
    pops: Vec<Option<BaseId>>,
    next_state: usize,
    next_transition: usize,
    next_expression: usize,
    next_leaf: usize,
}

impl ExpectedInputTrace {
    fn from_root(root: &StateGraphBinaryElement) -> Result<Self, Box<dyn Error>> {
        let mut trace = Self::default();
        trace.visit(root, None)?;
        Ok(trace)
    }

    fn visit(
        &mut self,
        element: &StateGraphBinaryElement,
        top: Option<BaseId>,
    ) -> Result<(), Box<dyn Error>> {
        let value = match element.name.as_slice() {
            b"state" => {
                let id = StateId(self.next_state);
                self.next_state += 1;
                Some(BaseId::State(id))
            }
            b"transition" => {
                let id = TransitionId(self.next_transition);
                self.next_transition += 1;
                Some(BaseId::Transition(id))
            }
            b"expression" => {
                let id = ExpressionId(self.next_expression);
                self.next_expression += 1;
                Some(BaseId::Expression(id))
            }
            b"condition" | b"behaviour" | b"hook" => {
                let kind = match element.name.as_slice() {
                    b"condition" => LeafKind::Condition,
                    b"behaviour" => LeafKind::Behaviour,
                    b"hook" => LeafKind::Hook,
                    _ => unreachable!("outer match fixes the leaf tag"),
                };
                let requested_name = attribute_text(element, b"name")
                    .ok_or_else(|| io::Error::other("factory element lacked name"))?;
                let Some(registered_name) =
                    state_graph_registry_catalog::resolve(catalog_kind(kind), requested_name)
                else {
                    self.misses.push((kind, requested_name.to_vec()));
                    self.pushes.push(None);
                    for child in &element.children {
                        self.visit(child, None)?;
                    }
                    self.pops.push(None);
                    return Ok(());
                };
                let parent = top.ok_or_else(|| io::Error::other("factory leaf lacked parent"))?;
                let leaf = LeafId(self.next_leaf);
                self.next_leaf += 1;
                self.creates.push(CreateObservation {
                    leaf,
                    kind,
                    parent,
                    requested_name: requested_name.to_vec(),
                    registered_name: registered_name.to_vec(),
                    attributes: element.attributes.clone(),
                });
                Some(BaseId::Leaf(leaf))
            }
            b"param" => {
                let Some(BaseId::Leaf(leaf)) = top else {
                    return Err(io::Error::other("Param lacked leaf parent").into());
                };
                let registered_name = self
                    .creates
                    .get(leaf.0)
                    .ok_or_else(|| io::Error::other("Param parent leaf was not created"))?
                    .registered_name
                    .clone();
                self.params.push(ParamObservation {
                    leaf,
                    registered_name,
                    attributes: element.attributes.clone(),
                });
                None
            }
            other => {
                return Err(io::Error::other(format!(
                    "mounted StateGraph used unsupported trace tag {}",
                    String::from_utf8_lossy(other)
                ))
                .into());
            }
        };
        self.pushes.push(value);
        for child in &element.children {
            self.visit(child, value)?;
        }
        self.pops.push(value);
        Ok(())
    }
}

const fn catalog_kind(kind: LeafKind) -> state_graph_registry_catalog::StateGraphFactoryKind {
    match kind {
        LeafKind::Condition => state_graph_registry_catalog::StateGraphFactoryKind::Condition,
        LeafKind::Behaviour => state_graph_registry_catalog::StateGraphFactoryKind::Behaviour,
        LeafKind::Hook => state_graph_registry_catalog::StateGraphFactoryKind::Hook,
    }
}

fn attribute_text<'a>(element: &'a StateGraphBinaryElement, key: &[u8]) -> Option<&'a [u8]> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .map(|attribute| attribute.text.as_slice())
}

/// Builds and structurally finalizes every shipped compiled `StateGraph`.
///
/// # Errors
///
/// Returns a mounted-data, format, registry, construction, attachment,
/// resolution, traversal, or exact-corpus mismatch.
#[allow(clippy::too_many_lines)]
pub fn load(mounts: &MountSet) -> Result<StateGraphRuntimeCorpusSummary, Box<dyn Error>> {
    let catalog_counts = state_graph_registry_catalog::counts();
    let registration_provenance = [
        state_graph_registry_catalog::CONDITION_REGISTRATION_ROWS_SHA256,
        state_graph_registry_catalog::BEHAVIOUR_REGISTRATION_ROWS_SHA256,
        state_graph_registry_catalog::HOOK_REGISTRATION_ROWS_SHA256,
    ];
    if (
        catalog_counts.conditions,
        catalog_counts.behaviours,
        catalog_counts.hooks,
    ) != (286, 283, 18)
        || !state_graph_registry_catalog::key_catalog_matches_frozen()
        || state_graph_registry_catalog::resolve(
            state_graph_registry_catalog::StateGraphFactoryKind::Condition,
            b"DistanceFromWaypoint",
        )
        .is_some()
        || state_graph_registry_catalog::resolve(
            state_graph_registry_catalog::StateGraphFactoryKind::Condition,
            b"DistanceFromWaypointXZ",
        ) != Some(b"DistanceFromWaypointXZ".as_slice())
    {
        return Err(format!(
            "StateGraph linked registry census differed: {catalog_counts:?}; frozen registration-row provenance {registration_provenance:?}"
        )
        .into());
    }
    let mut summary = StateGraphRuntimeCorpusSummary {
        graphs: 0,
        states: 0,
        transitions: 0,
        expressions: 0,
        leaf_requests: 0,
        attached_leaves: 0,
        params: 0,
        factory_misses: 0,
    };
    let mut operators = [0_usize; 4];
    let mut priorities = [0_usize; 4];
    let mut masks = [0_usize; 4];
    let mut active = [0_usize; 2];
    let mut interruptable = [0_usize; 3];
    let mut selector_counts: [BTreeMap<Vec<u8>, usize>; 3] =
        std::array::from_fn(|_| BTreeMap::new());
    let mut interrupt_targets = BTreeMap::<Vec<u8>, usize>::new();
    for (index, &(path, _, _, _, maximum_depth)) in EXPECTED.iter().enumerate() {
        let bytes = mounts.read_path(path)?;
        let payload_sha = state_graph_registry_catalog::sha256_hex(&bytes);
        if payload_sha != PAYLOAD_SHA256[index] {
            return Err(format!(
                "compiled StateGraph {path} SHA-256 was {payload_sha}, expected {}",
                PAYLOAD_SHA256[index]
            )
            .into());
        }
        let document = StateGraphBinaryDocument::parse_exact(&bytes)?;
        accumulate_selector_counts(&document.root, &mut selector_counts)?;
        let expected_inputs = ExpectedInputTrace::from_root(&document.root)?;
        let mut factory = CatalogFactory::new();
        let mut graph = StateGraphRuntime::build(
            &document.root,
            &mut factory,
            FactoryMissPolicy::NativeSafeNull,
        )?;
        verify_built_graph(
            index,
            path,
            maximum_depth,
            &graph,
            &factory,
            &expected_inputs,
        )?;
        let process_events = graph.process_after_build(&mut factory)?;
        verify_processed_graph(index, path, &graph, &factory, &process_events)?;
        accumulate_fields(
            &graph,
            &mut operators,
            &mut priorities,
            &mut masks,
            &mut active,
            &mut interruptable,
            &mut interrupt_targets,
        );
        summary.graphs += 1;
        summary.states += graph.states().len();
        summary.transitions += graph.transitions().len();
        summary.expressions += graph.expressions().len();
        summary.leaf_requests +=
            CONDITION_REQUESTS[index] + BEHAVIOUR_REQUESTS[index] + HOOK_REQUESTS[index];
        summary.attached_leaves += graph.leaves().len();
        summary.params += graph.params().len();
        summary.factory_misses += CONDITION_MISSES[index];
    }
    if operators != [2_228, 335, 463, 535]
        || priorities != [2_335, 35, 81, 6]
        || masks != [3_624, 21, 822, 0]
        || active != [650, 2_031]
        || interruptable != [1_763, 850, 68]
        || interrupt_targets != expected_interrupt_targets()
    {
        return Err(format!(
            "StateGraph runtime field census differed: operators={operators:?} priorities={priorities:?} masks={masks:?} active={active:?} interruptable={interruptable:?}"
        )
        .into());
    }
    let selector_digests = selector_counts.each_ref().map(selector_count_sha256);
    let selector_distinct = selector_counts.each_ref().map(BTreeMap::len);
    if selector_digests != SELECTOR_COUNT_SHA256 || selector_distinct != SELECTOR_DISTINCT {
        return Err(format!(
            "StateGraph factory selector census differed: distinct={selector_distinct:?} digests={selector_digests:?}"
        )
        .into());
    }
    Ok(summary)
}

#[allow(clippy::too_many_lines)]
fn verify_built_graph(
    index: usize,
    path: &str,
    maximum_depth: usize,
    graph: &StateGraphRuntime<OpaqueLeaf>,
    factory: &CatalogFactory,
    expected_inputs: &ExpectedInputTrace,
) -> Result<(), Box<dyn Error>> {
    let expected_leaves = CONDITION_REQUESTS[index] - CONDITION_MISSES[index]
        + BEHAVIOUR_REQUESTS[index]
        + HOOK_REQUESTS[index];
    let leaf_counts = graph
        .leaves()
        .iter()
        .fold([0_usize; 3], |mut counts, leaf| {
            counts[leaf_kind_index(leaf.kind)] += 1;
            counts
        });
    let misses = graph
        .construction_events()
        .iter()
        .filter_map(|event| match event {
            ConstructionEvent::FactoryMiss { kind, name } => Some((*kind, name.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    let pushes = graph
        .construction_events()
        .iter()
        .filter_map(|event| match event {
            ConstructionEvent::Push { value, .. } => Some(*value),
            _ => None,
        })
        .collect::<Vec<_>>();
    let pops = graph
        .construction_events()
        .iter()
        .filter_map(|event| match event {
            ConstructionEvent::Pop { value, .. } => Some(*value),
            _ => None,
        })
        .collect::<Vec<_>>();
    let maximum_stack = graph
        .construction_events()
        .iter()
        .filter_map(|event| match event {
            ConstructionEvent::Push { depth, .. } => Some(*depth),
            _ => None,
        })
        .max();
    let root_count = graph
        .states()
        .iter()
        .filter(|state| state.parent_state.is_none())
        .count();
    let payloads_match = graph.leaves().iter().enumerate().all(|(ordinal, leaf)| {
        let Some(payload) = leaf.payload() else {
            return false;
        };
        let Some(expected) = expected_inputs.creates.get(ordinal) else {
            return false;
        };
        let expected_param_count = expected_inputs
            .params
            .iter()
            .filter(|param| param.leaf == expected.leaf)
            .count();
        payload.ordinal == ordinal
            && payload.leaf == LeafId(ordinal)
            && payload.kind == expected.kind
            && payload.parent == expected.parent
            && payload.requested_name == expected.requested_name
            && payload.registered_name == expected.registered_name
            && payload.attributes == expected.attributes
            && payload.params == expected_param_count
            && leaf.base.parent == Some(expected.parent)
            && leaf.kind == expected.kind
            && leaf.registered_name == expected.registered_name
    });
    let param_records_match =
        graph
            .params()
            .iter()
            .zip(&expected_inputs.params)
            .all(|(actual, expected)| {
                actual.parent == Some(BaseId::Leaf(expected.leaf))
                    && actual.attributes == expected.attributes
                    && actual.handled_by_override
            });
    if graph.root() != Some(StateId(0))
        || root_count != 1
        || graph.states().len() != STATES[index]
        || graph.transitions().len() != TRANSITIONS[index]
        || graph.expressions().len() != EXPRESSIONS[index]
        || graph.params().len() != PARAMS[index]
        || graph.leaves().len() != expected_leaves
        || leaf_counts
            != [
                CONDITION_REQUESTS[index] - CONDITION_MISSES[index],
                BEHAVIOUR_REQUESTS[index],
                HOOK_REQUESTS[index],
            ]
        || misses != expected_inputs.misses
        || misses.len() != CONDITION_MISSES[index]
        || maximum_stack != Some(maximum_depth + 1)
        || expected_inputs.next_state != STATES[index]
        || expected_inputs.next_transition != TRANSITIONS[index]
        || expected_inputs.next_expression != EXPRESSIONS[index]
        || expected_inputs.next_leaf != expected_leaves
        || factory.creates != expected_inputs.creates
        || factory.params != expected_inputs.params
        || pushes != expected_inputs.pushes
        || pops != expected_inputs.pops
        || !payloads_match
        || !param_records_match
        || !construction_callback_windows_match(graph, expected_inputs)
    {
        return Err(format!("StateGraph construction corpus differed for {path}").into());
    }
    Ok(())
}

fn construction_callback_windows_match(
    graph: &StateGraphRuntime<OpaqueLeaf>,
    expected: &ExpectedInputTrace,
) -> bool {
    let events = graph.construction_events();
    let mut created = Vec::new();
    let mut misses = 0;
    let mut params = 0;
    for (index, event) in events.iter().enumerate() {
        match event {
            ConstructionEvent::FactoryCreated { kind, leaf } => {
                let Some(expected_create) = expected.creates.get(created.len()) else {
                    return false;
                };
                if (*kind, *leaf) != (expected_create.kind, expected_create.leaf)
                    || index < 2
                    || events.get(index - 2)
                        != Some(&ConstructionEvent::Constructed(BaseId::Leaf(*leaf)))
                    || events.get(index - 1)
                        != Some(&ConstructionEvent::Attached {
                            parent: expected_create.parent,
                            child: BaseId::Leaf(*leaf),
                        })
                    || !matches!(
                        events.get(index + 1),
                        Some(ConstructionEvent::Push {
                            value: Some(BaseId::Leaf(pushed)),
                            ..
                        }) if pushed == leaf
                    )
                {
                    return false;
                }
                created.push(*leaf);
            }
            ConstructionEvent::FactoryMiss { .. } => {
                misses += 1;
                if !matches!(
                    events.get(index + 1),
                    Some(ConstructionEvent::Push { value: None, .. })
                ) {
                    return false;
                }
            }
            ConstructionEvent::Param { parent, handled } => {
                let Some(expected_param) = expected.params.get(params) else {
                    return false;
                };
                params += 1;
                if *parent != Some(BaseId::Leaf(expected_param.leaf))
                    || !handled
                    || !matches!(
                        events.get(index + 1),
                        Some(ConstructionEvent::Push { value: None, .. })
                    )
                    || !matches!(
                        events.get(index + 2),
                        Some(ConstructionEvent::Pop { value: None, .. })
                    )
                {
                    return false;
                }
            }
            _ => {}
        }
    }
    created.len() == expected.creates.len()
        && misses == expected.misses.len()
        && params == expected.params.len()
}

fn verify_processed_graph(
    index: usize,
    path: &str,
    graph: &StateGraphRuntime<OpaqueLeaf>,
    factory: &CatalogFactory,
    events: &[ProcessEvent],
) -> Result<(), Box<dyn Error>> {
    let process_counts = events.iter().fold([0_usize; 4], |mut counts, event| {
        counts[match event {
            ProcessEvent::State(_) => 0,
            ProcessEvent::Transition(_) => 1,
            ProcessEvent::Expression(_) => 2,
            ProcessEvent::Leaf(_) => 3,
        }] += 1;
        counts
    });
    let named_interrupts = graph
        .states()
        .iter()
        .filter(|state| {
            matches!(
                state.interruptable,
                Interruptable::StateInterruptable {
                    target: Some(_),
                    ..
                }
            )
        })
        .count();
    let expected_events = expected_process_events(graph)?;
    let expected_leaf_callbacks = events
        .iter()
        .filter_map(|event| match event {
            ProcessEvent::Leaf(leaf) => Some(*leaf),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut leaf_seen = vec![false; graph.leaves().len()];
    let exactly_once = factory.processed.iter().all(|leaf| {
        let Some(seen) = leaf_seen.get_mut(leaf.0) else {
            return false;
        };
        if *seen {
            return false;
        }
        *seen = true;
        true
    }) && leaf_seen.into_iter().all(|seen| seen);
    if events != expected_events
        || factory.processed != expected_leaf_callbacks
        || !exactly_once
        || process_counts
            != [
                STATES[index],
                TRANSITIONS[index],
                EXPRESSIONS[index],
                graph.leaves().len(),
            ]
        || named_interrupts != NAMED_INTERRUPTS[index]
        || graph
            .transitions()
            .iter()
            .any(|transition| transition.target.is_none())
        || graph.states().iter().any(|state| !state.base.enabled)
    {
        return Err(format!("StateGraph Process corpus differed for {path}").into());
    }
    Ok(())
}

fn expected_process_events(
    graph: &StateGraphRuntime<OpaqueLeaf>,
) -> Result<Vec<ProcessEvent>, Box<dyn Error>> {
    fn expression(
        graph: &StateGraphRuntime<OpaqueLeaf>,
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

    fn transition(
        graph: &StateGraphRuntime<OpaqueLeaf>,
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

    fn state(graph: &StateGraphRuntime<OpaqueLeaf>, id: StateId, events: &mut Vec<ProcessEvent>) {
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
        for child in &graph.states()[id.0].transitions {
            transition(graph, *child, events);
        }
    }

    let root = graph
        .root()
        .ok_or_else(|| io::Error::other("mounted StateGraph lacked root"))?;
    let mut events = Vec::new();
    state(graph, root, &mut events);
    Ok(events)
}

fn accumulate_fields(
    graph: &StateGraphRuntime<OpaqueLeaf>,
    operators: &mut [usize; 4],
    priorities: &mut [usize; 4],
    masks: &mut [usize; 4],
    active: &mut [usize; 2],
    interruptable: &mut [usize; 3],
    interrupt_targets: &mut BTreeMap<Vec<u8>, usize>,
) {
    for expression in graph.expressions() {
        operators[match expression.operator {
            ExpressionOperator::None => 0,
            ExpressionOperator::And => 1,
            ExpressionOperator::Or => 2,
            ExpressionOperator::Not => 3,
        }] += 1;
    }
    for transition in graph.transitions() {
        priorities[match transition.priority {
            TransitionPriority::Low => 0,
            TransitionPriority::Medium => 1,
            TransitionPriority::High => 2,
            TransitionPriority::Urgent => 3,
        }] += 1;
    }
    for leaf in graph
        .leaves()
        .iter()
        .filter(|leaf| leaf.kind == LeafKind::Condition)
    {
        masks[match leaf.mask.expect("Condition base always stores a mask") {
            ConditionMask::PreCond => 0,
            ConditionMask::Sustain => 1,
            ConditionMask::Always => 2,
            ConditionMask::PostCond => 3,
        }] += 1;
    }
    for state in graph.states() {
        active[usize::from(state.active)] += 1;
        match &state.interruptable {
            Interruptable::NotInterruptable => interruptable[0] += 1,
            Interruptable::RootInterruptable => interruptable[1] += 1,
            Interruptable::StateInterruptable { target_name, .. } => {
                interruptable[2] += 1;
                *interrupt_targets.entry(target_name.clone()).or_default() += 1;
            }
        }
    }
}

fn expected_interrupt_targets() -> BTreeMap<Vec<u8>, usize> {
    NAMED_INTERRUPT_TARGETS
        .into_iter()
        .map(|(name, count)| (name.to_vec(), count))
        .collect()
}

fn accumulate_selector_counts(
    root: &StateGraphBinaryElement,
    counts: &mut [BTreeMap<Vec<u8>, usize>; 3],
) -> Result<(), Box<dyn Error>> {
    for element in root.preorder() {
        let kind = match element.name.as_slice() {
            b"condition" => Some(LeafKind::Condition),
            b"behaviour" => Some(LeafKind::Behaviour),
            b"hook" => Some(LeafKind::Hook),
            _ => None,
        };
        if let Some(kind) = kind {
            let name = attribute_text(element, b"name")
                .ok_or_else(|| io::Error::other("factory element lacked selector name"))?;
            *counts[leaf_kind_index(kind)]
                .entry(name.to_vec())
                .or_default() += 1;
        }
    }
    Ok(())
}

fn selector_count_sha256(counts: &BTreeMap<Vec<u8>, usize>) -> String {
    let mut canonical = Vec::new();
    for (name, count) in counts {
        canonical.extend_from_slice(name);
        canonical.push(b'\t');
        canonical.extend_from_slice(count.to_string().as_bytes());
        canonical.push(b'\n');
    }
    state_graph_registry_catalog::sha256_hex(&canonical)
}

const fn leaf_kind_index(kind: LeafKind) -> usize {
    match kind {
        LeafKind::Condition => 0,
        LeafKind::Behaviour => 1,
        LeafKind::Hook => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_file_runtime_oracles_sum_to_the_frozen_totals() {
        assert_eq!(STATES.iter().sum::<usize>(), 2_681);
        assert_eq!(TRANSITIONS.iter().sum::<usize>(), 2_457);
        assert_eq!(EXPRESSIONS.iter().sum::<usize>(), 3_561);
        assert_eq!(CONDITION_REQUESTS.iter().sum::<usize>(), 4_485);
        assert_eq!(BEHAVIOUR_REQUESTS.iter().sum::<usize>(), 3_828);
        assert_eq!(HOOK_REQUESTS.iter().sum::<usize>(), 80);
        assert_eq!(PARAMS.iter().sum::<usize>(), 165);
        assert_eq!(NAMED_INTERRUPTS.iter().sum::<usize>(), 68);
        assert_eq!(CONDITION_MISSES.iter().sum::<usize>(), 18);
    }
}
