//! Mounted two-tick diagnostic for the exact living-world test graph.
//!
//! Uses the production authority-bound session with observing helper services.
//! This proves typed dispatch, not LWAnimHelper, UCOM, visible animation, an
//! Actor spawn, generated-controller selection, or a production frame.

impl MotionGraphHasIntentRuntime<()> for ObservingRuntime {
 type Error = io::Error;
 type MotionGraph = (); type Intents = ();
fn cached_motion_graph(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<Option<()>, io::Error> { self.unrelated() }
fn find_motion_graph(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<Option<()>, io::Error> { self.unrelated() }
fn motion_graph_intents(&mut self, _entity: &MotionGraphEntityAuthority<()>, _motion_graph: &()) -> Result<(), io::Error> { self.unrelated() }
fn lookup_intent(&mut self, _entity: &MotionGraphEntityAuthority<()>, _intents: &(), _intent_name: &[u8]) -> Result<Option<f32>, io::Error> { self.unrelated() }
}

impl MotionGraphWillExpireRuntime<()> for ObservingRuntime {
 type Error = io::Error;
 type AnimInfo = (); type Animatable = ();
fn find_anim_info(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<Option<()>, io::Error> { self.unrelated() }
fn resolve_special_tag(&mut self, _entity: &MotionGraphEntityAuthority<()>, _anim_info: &(), _tag: &[u8]) -> Result<f32, io::Error> { self.unrelated() }
fn cached_animatable(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<Option<()>, io::Error> { self.unrelated() }
fn find_animatable(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<Option<()>, io::Error> { self.unrelated() }
fn transitioning(&mut self, _entity: &MotionGraphEntityAuthority<()>, _animatable: &()) -> Result<bool, io::Error> { self.unrelated() }
fn anim_property(&mut self, _entity: &MotionGraphEntityAuthority<()>, _animatable: &()) -> Result<Option<AnimProperty>, io::Error> { self.unrelated() }
}

impl MotionGraphPlayAnimationRuntime<()> for ObservingRuntime {
 type Error = io::Error;
 
fn resolve_interface(&mut self, _entity: &MotionGraphEntityAuthority<()>, _kind: PlayAnimationInterfaceKind) -> Result<PlayAnimationInterfaceResolution, io::Error> { self.unrelated() }
fn motion_intent(&mut self, _entity: &MotionGraphEntityAuthority<()>, _name: FastString36) -> Result<Option<u32>, io::Error> { self.unrelated() }
fn filtered_motion_intent(&mut self, _entity: &MotionGraphEntityAuthority<()>, _name: FastString36) -> Result<Option<u32>, io::Error> { self.unrelated() }
fn tree_attribute(&mut self, _entity: &MotionGraphEntityAuthority<()>, _name: FastString30) -> Result<Option<TreeAttributeSample>, io::Error> { self.unrelated() }
fn add_settable_attribute(&mut self, _entity: &MotionGraphEntityAuthority<()>, _name: FastString30, _value_bits: u32, _normalize: bool, _sequence: i32) -> Result<(), io::Error> { self.unrelated() }
fn selected_pro_skater(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<FastString30, io::Error> { self.unrelated() }
fn set_construction_attribute(&mut self, _entity: &MotionGraphEntityAuthority<()>, _name: FastString30, _value: FastString30) -> Result<(), io::Error> { self.unrelated() }
fn next_transition_override(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<Option<RawAnimationTransitionParams>, io::Error> { self.unrelated() }
fn clear_transition_override(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<(), io::Error> { self.unrelated() }
fn is_switch(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<bool, io::Error> { self.unrelated() }
fn is_mirrored(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<bool, io::Error> { self.unrelated() }
fn physical_output(&mut self, _entity: &MotionGraphEntityAuthority<()>) -> Result<PhysicalOutputObservation, io::Error> { self.unrelated() }
fn dispatch(&mut self, _entity: &MotionGraphEntityAuthority<()>, _request: AnimationDispatch) -> Result<u8, io::Error> { self.unrelated() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn altered_payload_is_rejected_before_parsing() {
        assert!(construct_exact(&[]).is_err());
        assert!(construct_exact(&[0; 183]).is_err());
    }

    #[test]
    #[ignore = "requires external SKATE2_MOTION_GRAPH_TEST_PATH"]
    fn exact_external_test_payload_runs_authority_bound_two_tick_gate() {
        let path = env::var_os("SKATE2_MOTION_GRAPH_TEST_PATH")
            .expect("SKATE2_MOTION_GRAPH_TEST_PATH must name the extracted real payload");
        let bytes = std::fs::read(path).unwrap();
        probe_exact_payload(&bytes).unwrap();
    }
}


use crate::state_graph_registry_catalog::sha256_hex;
use skate2_core::{
    motion_graph_authority_session::AuthorityBoundMotionGraphSession,
    motion_graph_capability_preflight::preflight_motion_graph_capabilities,
    motion_graph_conditions::AnimProperty,
    motion_graph_entity_authority::{
        ActorGraphAuthority, MotionGraphEntityAuthority, MotionGraphHasIntentRuntime,
        MotionGraphPlayAnimationRuntime, MotionGraphPlayRemappedAnimationRuntime,
        MotionGraphWillExpireRuntime,
    },
    motion_graph_leaf_factory::{MotionGraphLeafFactory, MotionGraphLeafPayload},
    play_animation::{FastString30, FastString36},
    play_animation_lifecycle::{
        AnimationDispatch, PhysicalOutputObservation, PlayAnimationInstanceAllocator,
        PlayAnimationInterfaceKind, PlayAnimationInterfaceResolution,
        RawAnimationTransitionParams, TreeAttributeSample,
    },
    play_remapped_animation::{
        RemappedAnimationSetup, RemappedAnimationTransitionType, RemappedAnimationUpdate,
    },
    state_graph_context::StateGraphContextHandle,
    state_graph_runtime::{
        BaseId, ConstructionEvent, FactoryMissPolicy, Interruptable, LeafCreateRequest,
        LeafId, LeafKind, ProcessEvent, StateGraphLeafFactory, StateGraphRuntime, StateId,
    },
};
use skate2_formats::{
    mount::{MountedBig, SKATE2_ARCHIVES},
    state_graph::{StateGraphBinaryDocument, StateGraphBinaryElement, state_graph_string_hash32},
};
use std::{cell::RefCell, env, error::Error, io, path::PathBuf, rc::Rc};

const GRAPH_PATH: &str =
    r"data\state\livingworldentities\pedestrian\motiongraph_test.stategraph";
const GRAPH_SHA256: &str =
    "23FFF26C4BB98ED4BD821F49DBDE9937EE925D605D4F5B8D3CECFF3AC2460A9D";
type ProbeResult<T> = Result<T, Box<dyn Error>>;
type Graph = StateGraphRuntime<MotionGraphLeafPayload<()>>;
type Session = AuthorityBoundMotionGraphSession<(), (), ObservingRuntime, RejectAllocator>;

pub fn run() -> ProbeResult<()> {
    let root = env::var_os("SKATE2_GAME_ROOT")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("SKATE2_GAME_ROOT is not set"))?;
    let spec = SKATE2_ARCHIVES[11];
    let archive = MountedBig::open(spec.path_under(&root), spec.use_external_header)?;
    let entry = archive.archive().find_path(GRAPH_PATH)
        .ok_or_else(|| io::Error::other("test graph is absent from miscload.big"))?;
    require(
        entry.header_index == 143 && entry.offset == 0x35980 && entry.stored_size == 183,
        "test graph archive locator differed",
    )?;
    let bytes = archive.read_entry(entry)?;
    probe_exact_payload(&bytes)?;
    println!("MotionGraph test: miscload.big entry 143, offset 0x35980; 183 bytes; SHA-256 {GRAPH_SHA256}");
    println!("Motion -> Locomotion -> PlayRemappedAnimation: exact tree/defaults; Process [State(0), State(1), Leaf(0)]; 1 typed leaf, 0 preflight diagnostics");
    println!("tick 1 (0.25): Setup -> Update, active Leaf(0)/None, timers [0, 0]; tick 2 (0.5): Update, timers [0.5, 0.5]");
    println!("same issued Entity on all 3 helper calls; 0 allocator, mirror, or unrelated runtime calls");
    println!("Typed dispatch diagnostic only: LWAnimHelper, UCOM, visible animation, spawned actors, and production frames remain unimplemented.");
    Ok(())
}

fn require(condition: bool, message: &str) -> ProbeResult<()> {
    if condition { Ok(()) } else { Err(io::Error::other(message).into()) }
}

fn probe_exact_payload(bytes: &[u8]) -> ProbeResult<()> {
    let graph = construct_exact(bytes)?;
    let observations = Rc::new(RefCell::new(Observations::default()));
    let owner = ActorGraphAuthority::new(());
    let facade = owner.motion_graph_facade(ObservingRuntime(Rc::clone(&observations)));
    let entity = facade.entity_authority().shallow_copy();
    let mut session = AuthorityBoundMotionGraphSession::try_new(
        graph, facade, RejectAllocator(Rc::clone(&observations)),
    )?;
    require(observations.borrow().events.is_empty(), "session admission invoked a service")?;
    require(session.current_state().is_none() && session.last_state().is_none()
        && session.active_behaviours().len() == 0, "initial controller state differed")?;
    session.update(0.25)?;
    validate_tick(&session, None, 0.25f32.to_bits(), 0)?;
    validate_observations(&observations.borrow(), &entity, &[Effect::Setup, Effect::Update])?;
    session.update(0.5)?;
    validate_tick(&session, Some(StateId(1)), 0.5f32.to_bits(), 0.5f32.to_bits())?;
    validate_observations(&observations.borrow(), &entity,
        &[Effect::Setup, Effect::Update, Effect::Update])?;
    // Explicit EndAll and ordinary owner destruction must add no helper effects.
    session.end_all_behaviours()?;
    require(session.active_behaviours().len() == 0, "EndAll retained an active behaviour")?;
    drop(session);
    validate_observations(&observations.borrow(), &entity,
        &[Effect::Setup, Effect::Update, Effect::Update])
}

fn validate_tick(session: &Session, last: Option<StateId>, delta: u32, timer: u32) -> ProbeResult<()> {
    let active = session.active_behaviours().map(|(id, instance)| (id, instance.is_none()))
        .collect::<Vec<_>>();
    require(!session.is_poisoned() && session.current_state() == Some(StateId(1))
        && session.last_state() == last && session.delta_time().to_bits() == delta
        && session.time_in_state(StateId(0)).to_bits() == timer
        && session.time_in_state(StateId(1)).to_bits() == timer
        && session.time_in_current_state().to_bits() == timer
        && active == [(LeafId(0), true)], "two-tick controller oracle differed")
}

fn validate_observations(
    observations: &Observations, entity: &MotionGraphEntityAuthority<()>, effects: &[Effect],
) -> ProbeResult<()> {
    require(observations.allocator_calls == 0 && observations.mirror_calls == 0
        && observations.unrelated_calls == 0, "probe invoked a forbidden collaborator")?;
    require(observations.events.len() == effects.len()
        && observations.events.iter().zip(effects).all(|((effect, issued), expected)|
            effect == expected && issued.is_same_issuance(entity)),
        "helper effect order or Entity issuance differed")
}

fn construct_exact(bytes: &[u8]) -> ProbeResult<Graph> {
    require(bytes.len() == 183 && sha256_hex(bytes) == GRAPH_SHA256,
        "test graph byte identity differed")?;
    let document = StateGraphBinaryDocument::parse_exact(bytes)?;
    validate_wire_tree(&document.root)?;
    let mut factory = MotionGraphLeafFactory::new(RejectFallback)?;
    let mut graph = StateGraphRuntime::build(&document.root, &mut factory, FactoryMissPolicy::Strict)?;
    validate_arena(&graph)?;
    require(graph.construction_events() == construction_oracle(),
        "test graph construction order differed")?;
    let process = graph.process_after_build(&mut factory)?;
    require(process == [ProcessEvent::State(StateId(0)),
        ProcessEvent::State(StateId(1)), ProcessEvent::Leaf(LeafId(0))],
        "test graph Process order differed")?;
    let report = preflight_motion_graph_capabilities(&graph)?;
    require(report.leaf_count == 1 && report.diagnostics.is_empty()
        && report.has_full_leaf_coverage(), "test graph capability coverage differed")?;
    Ok(graph)
}

fn validate_wire_tree(root: &StateGraphBinaryElement) -> ProbeResult<()> {
    require(root.node_count() == 3 && root.attribute_count() == 6
        && root.maximum_depth() == 2 && root.children.len() == 1,
        "test graph wire census differed")?;
    let child = &root.children[0];
    require(child.children.len() == 1 && child.children[0].children.is_empty(),
        "test graph wire hierarchy differed")?;
    for (node, name, attributes) in [
        (root, b"State".as_slice(), [(b"name".as_slice(), b"Motion".as_slice()), (b"policy", b"Priority")]),
        (child, b"State", [(b"name", b"Locomotion"), (b"policy", b"Priority")]),
        (&child.children[0], b"Behaviour", [(b"name", b"PlayRemappedAnimation"), (b"anim", b"FACE_HAND_TEST")]),
    ] {
        require(node.name == name && node.attributes.len() == 2,
            "test graph wire element differed")?;
        for (attribute, (key, text)) in node.attributes.iter().zip(attributes) {
            require(attribute.key == key && attribute.text == text
                && attribute.key_hash == state_graph_string_hash32(key)
                && attribute.float_bits == 0 && attribute.boolean_raw == 0,
                "test graph ordered wire attributes differed")?;
        }
    }
    Ok(())
}

fn validate_arena(graph: &Graph) -> ProbeResult<()> {
    require(graph.root() == Some(StateId(0)) && graph.states().len() == 2
        && graph.leaves().len() == 1 && graph.expressions().is_empty()
        && graph.transitions().is_empty() && graph.params().is_empty(),
        "test graph arena census differed")?;
    for (index, name, path, parent, children, behaviours, depth) in [
        (0, b"Motion".as_slice(), b"Motion".as_slice(), None, vec![StateId(1)], vec![], 0),
        (1, b"Locomotion", b"Motion.Locomotion", Some(StateId(0)), vec![], vec![LeafId(0)], 1),
    ] {
        let state = &graph.states()[index];
        require(state.base.name == name && state.base.path == path
            && state.base.parent == parent.map(BaseId::State)
            && state.base.cast_parent == parent.is_none()
            && state.base.enabled && state.active && state.parent_state == parent
            && state.expression.is_none() && state.interruptable == Interruptable::RootInterruptable
            && state.children == children && state.behaviours == behaviours
            && state.transitions.is_empty() && state.depth == depth, "test graph State fields differed")?;
    }
    let leaf = &graph.leaves()[0];
    require(leaf.base.name == b"PlayRemappedAnimation"
        && leaf.base.path == b"Motion.Locomotion.PlayRemappedAnimation"
        && leaf.base.parent == Some(BaseId::State(StateId(1))) && leaf.base.enabled
        && !leaf.base.cast_parent && leaf.kind == LeafKind::Behaviour
        && leaf.registered_name == b"PlayRemappedAnimation" && leaf.mask.is_none(),
        "test graph leaf identity differed")?;
    let Some(MotionGraphLeafPayload::PlayRemappedAnimation(payload)) = leaf.payload() else {
        return Err("test graph lacked typed remapped payload".into());
    };
    require(payload.animation_name() == b"FACE_HAND_TEST"
        && payload.transition_type == RemappedAnimationTransitionType::Blend
        && payload.playback_speed_bits == 0x3f80_0000 && payload.frame_offset_bits == 0
        && payload.frame_offset_tag().is_empty() && payload.cycle_percentage_bits == 0
        && payload.cycle_percentage_tag().is_empty() && payload.blend_length_bits == 0x3dcc_cccd
        && payload.mirror_override_raw.is_none(), "test graph remapped defaults differed")
}

fn construction_oracle() -> Vec<ConstructionEvent> {
    let root = BaseId::State(StateId(0));
    let child = BaseId::State(StateId(1));
    let leaf = BaseId::Leaf(LeafId(0));
    vec![
        ConstructionEvent::Constructed(root),
        ConstructionEvent::Push { value: Some(root), depth: 1 },
        ConstructionEvent::Constructed(child),
        ConstructionEvent::Attached { parent: root, child },
        ConstructionEvent::Push { value: Some(child), depth: 2 },
        ConstructionEvent::Constructed(leaf),
        ConstructionEvent::Attached { parent: child, child: leaf },
        ConstructionEvent::FactoryCreated { kind: LeafKind::Behaviour, leaf: LeafId(0) },
        ConstructionEvent::Push { value: Some(leaf), depth: 3 },
        ConstructionEvent::Pop { value: Some(leaf), depth: 2 },
        ConstructionEvent::Pop { value: Some(child), depth: 1 },
        ConstructionEvent::Pop { value: Some(root), depth: 0 },
    ]
}

struct RejectFallback;
impl StateGraphLeafFactory for RejectFallback {
    type Payload = ();
    type Error = io::Error;
    fn registered_names(&self, _kind: LeafKind) -> &[Vec<u8>] { &[] }
    fn create(&mut self, _request: LeafCreateRequest<'_>) -> Result<(), io::Error> {
        Err(io::Error::other("test graph unexpectedly invoked fallback"))
    }
}

#[derive(Debug, PartialEq)]
enum Effect { Setup, Update }

#[derive(Default)]
struct Observations {
    events: Vec<(Effect, MotionGraphEntityAuthority<()>)>,
    allocator_calls: usize,
    mirror_calls: usize,
    unrelated_calls: usize,
}
struct ObservingRuntime(Rc<RefCell<Observations>>);
impl ObservingRuntime {
    fn unrelated<T>(&self) -> Result<T, io::Error> {
        self.0.borrow_mut().unrelated_calls += 1;
        Err(io::Error::other("test graph invoked unrelated runtime service"))
    }
}
struct RejectAllocator(Rc<RefCell<Observations>>);
impl PlayAnimationInstanceAllocator for RejectAllocator {
    type Error = io::Error;
    fn allocate(&mut self, _size: usize, _name: &[u8], _alignment: usize) -> Result<bool, io::Error> {
        self.0.borrow_mut().allocator_calls += 1;
        Err(io::Error::other("null-instance remapped behaviour invoked allocator"))
    }
}

impl MotionGraphPlayRemappedAnimationRuntime<()> for ObservingRuntime {
    type Error = io::Error;
    fn write_remapped_mirror_override_raw(&mut self, _entity: &MotionGraphEntityAuthority<()>, _value: u8) -> Result<(), io::Error> {
        self.0.borrow_mut().mirror_calls += 1;
        Err(io::Error::other("mirror-absent test graph invoked mirror service"))
    }
    fn setup_remapped_animation(&mut self, entity: &MotionGraphEntityAuthority<()>, request: RemappedAnimationSetup<'_>) -> Result<(), io::Error> {
        if request != (RemappedAnimationSetup {
            animation_name: b"FACE_HAND_TEST", playback_speed_bits: 0x3f80_0000,
            frame_offset_tag: b"", frame_offset_bits: 0,
            cycle_percentage_tag: b"", cycle_percentage_bits: 0,
            transition_type: RemappedAnimationTransitionType::Blend, blend_length_bits: 0x3dcc_cccd,
        }) { return Err(io::Error::other("Setup remapped fields differed")); }
        self.0.borrow_mut().events.push((Effect::Setup, entity.shallow_copy()));
        Ok(())
    }
    fn update_remapped_animation(&mut self, entity: &MotionGraphEntityAuthority<()>, request: RemappedAnimationUpdate<'_>) -> Result<(), io::Error> {
        if request != (RemappedAnimationUpdate { animation_name: b"FACE_HAND_TEST", playback_speed_bits: 0x3f80_0000 }) {
            return Err(io::Error::other("Update remapped fields differed"));
        }
        self.0.borrow_mut().events.push((Effect::Update, entity.shallow_copy()));
        Ok(())
    }
}
