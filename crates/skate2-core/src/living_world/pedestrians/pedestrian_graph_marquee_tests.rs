use super::{attributes, is_facing_skater, pitch_to_target};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    generated_action_load::{
        BehaviourDeclaration, StateDeclaration, runtime::ActionUpdateError as Error,
    },
    generated_graph_selection::Mask,
    generated_onboard_state_graph::{
        GeneratedOnboardEnablement, OnboardControllerSelection, select_onboard_controller,
    },
    living_world_grind_services::dmo_objects::ENTITIES_CLASS,
    living_world_pedestrian_resources::{ANIMATION_DATA, PedestrianResources},
    motion_graph_entity_authority::{
        ActorGraphAuthority, BoundMotionGraphContext, MotionGraphInstanceToken,
    },
    pedestrian_animation::{PedestrianRegisteredClips, remap_animation},
    pedestrian_graph::{
        AiDelegate, PedestrianGraph, PedestrianGraphBackend, PedestrianGraphBehaviours,
        behaviour_binding::LocalBehaviourBindings, condition_binding::PedestrianConditions,
        lifecycle, marquee_data, updates,
    },
    pedestrian_motion_delegate::{
        ActiveBehaviour, ControllerState, MotionDelegate, PedestrianMotionBackend,
        PedestrianMotionDelegate, backend::MotionObjects,
    },
    play_animation::FastString36,
};
use skate2_formats::{
    andale::AndaleDatabase,
    attrib::string_hash64,
    mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
};
use std::collections::BTreeSet;

/// `livingworld_entities` text: the entity's AI graph file.
const AI_GRAPH: u64 = 0xca4d_0ad4_ea45_965b;

/// Answers conditions from a set and records the TemplateHC callbacks.
#[derive(Default)]
struct Recorder {
    conditions: BTreeSet<u16>,
    calls: Vec<String>,
}

impl PedestrianGraphBackend for Recorder {
    fn condition(
        &mut self,
        _: &PedestrianGraph,
        _: Mask,
        condition: u16,
        _: u16,
    ) -> Result<bool, Error> {
        Ok(self.conditions.contains(&condition))
    }
    fn enabled(&self, _: u16) -> Result<bool, Error> {
        Ok(true)
    }
    fn create_instance(&mut self, _: &PedestrianGraph, _: u16) -> Result<(), Error> {
        Ok(())
    }
    fn begin(&mut self, _: &PedestrianGraph, occurrence: u16) -> Result<(), Error> {
        self.calls.push(format!("begin {occurrence}"));
        Ok(())
    }
    fn end_and_remove(&mut self, _: &PedestrianGraph, occurrence: u16) -> Result<(), Error> {
        self.calls.push(format!("end {occurrence}"));
        Ok(())
    }
    fn transition_hook(&mut self, _: &PedestrianGraph, hook: u16, _: u16) -> Result<(), Error> {
        Err(Error::PendingTransitionHook(hook))
    }
    fn generated_update(
        &mut self,
        _: &PedestrianGraph,
        source: u32,
        active: &[u16],
    ) -> Result<(), Error> {
        self.calls.push(format!("update {source:#x} {active:?}"));
        Ok(())
    }
}

impl Recorder {
    fn step(
        &mut self,
        graph: &mut PedestrianGraph,
        instances: &mut PedestrianGraphBehaviours,
        delegate: AiDelegate,
        conditions: &[u16],
    ) -> Vec<String> {
        self.conditions = conditions.iter().copied().collect();
        self.calls.clear();
        graph
            .update_delegate(delegate, 1. / 30., instances, self)
            .unwrap();
        std::mem::take(&mut self.calls)
    }
}

/// marquee.xml conditions: 0 NeedsToBeginColliding, 1 IsPedestrianColliding, 2 IsFacingSkater.
#[test]
fn marquee_holds_at_spawn_tracks_a_facing_skater_and_re_enters_after_a_collision() {
    let (mut graph, mut instances, mut backend) = (
        PedestrianGraph::default(),
        PedestrianGraphBehaviours::default(),
        Recorder::default(),
    );
    let delegate = AiDelegate::Marquee;
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[]),
        [
            "begin 0",
            "begin 1",
            "begin 4",
            "update 0x828f8b98 [0, 1, 4]"
        ]
    );
    assert_eq!(graph.current, 4);
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[2]),
        ["end 4", "begin 3", "update 0x828f8950 [0, 1, 3]"]
    );
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[]),
        ["end 3", "begin 4", "update 0x828f8b98 [0, 1, 4]"]
    );
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[0, 1]),
        ["end 4", "begin 2", "update 0x828f8a20 [0, 1, 2]"]
    );
    // Colliding sustains unconditionally; its transition re-enters Marquee once the
    // pedestrian stops colliding.
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[1]),
        ["update 0x828f8a20 [0, 1, 2]"]
    );
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[]),
        [
            "end 2",
            "end 1",
            "end 0",
            "begin 0",
            "begin 1",
            "begin 4",
            "update 0x828f8b98 [0, 1, 4]"
        ]
    );
}

/// Photographer.xml conditions: 2 IsPhotographerTakingPictures, 3 IsPhotographerReviewingPictures,
/// 4 IsFacingSkater.
#[test]
fn photographer_takes_pictures_then_reviews_them() {
    let (mut graph, mut instances, mut backend) = (
        PedestrianGraph::default(),
        PedestrianGraphBehaviours::default(),
        Recorder::default(),
    );
    let delegate = AiDelegate::Photographer;
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[4]),
        [
            "begin 0",
            "begin 1",
            "begin 7",
            "update 0x828f8950 [0, 1, 7]"
        ]
    );
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[2, 4]),
        [
            "end 7",
            "begin 3",
            "begin 4",
            "begin 5",
            "update 0x828f8a90 [0, 1, 3, 4, 5]"
        ]
    );
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[3]),
        [
            "end 5",
            "end 4",
            "end 3",
            "begin 6",
            "update 0x828f8b28 [0, 1, 6]"
        ]
    );
    assert_eq!(
        backend.step(&mut graph, &mut instances, delegate, &[]),
        ["end 6", "begin 8", "update 0x828f8b98 [0, 1, 8]"]
    );
}

fn active_behaviours(states: &[StateDeclaration], mut state: u16) -> Vec<u16> {
    let mut active = Vec::new();
    while state != u16::MAX {
        let section = states[usize::from(state)].behaviours;
        active.extend(section.index..section.index + section.count);
        state = states[usize::from(state)].parent;
    }
    active.sort_unstable();
    active
}

/// Every leaf's StateUpdate calls the update of the behaviour in each slot it names and passes
/// over the slots whose update is the empty 0x825187c0.
#[test]
fn marquee_state_updates_match_the_active_behaviours() {
    for delegate in [AiDelegate::Marquee, AiDelegate::Photographer] {
        let behaviours: &[BehaviourDeclaration] = match delegate {
            AiDelegate::Marquee => &marquee_data::marquee::BEHAVIOURS,
            _ => &marquee_data::photographer::BEHAVIOURS,
        };
        let states = delegate.states();
        for (index, state) in states
            .iter()
            .enumerate()
            .filter(|(_, s)| s.children.count == 0)
        {
            let active = active_behaviours(states, u16::try_from(index).unwrap());
            let steps = updates::sequence(state.update_address).expect("state update recovered");
            assert_eq!(steps.len(), active.len(), "{delegate:?} {}", state.name);
            for &(slot, callback) in steps {
                let name = behaviours[usize::from(active[slot])].name;
                let update = lifecycle::lookup(name).expect("lifecycle").update;
                if callback == updates::UpdateCallback::Noop {
                    assert_eq!(update, None, "{delegate:?} {} slot {slot}", state.name);
                } else {
                    assert_eq!(format!("{callback:?}"), name);
                    assert!(update.is_some());
                }
            }
        }
    }
}

#[test]
fn marquee_declarations_bind_their_behaviours_and_conditions() {
    for delegate in [AiDelegate::Marquee, AiDelegate::Photographer] {
        LocalBehaviourBindings::for_delegate(delegate).unwrap();
        PedestrianConditions::for_delegate(delegate).unwrap();
    }
    for delegate in [MotionDelegate::Marquee, MotionDelegate::Photographer] {
        MotionObjects::bind(delegate).unwrap();
    }
    assert_eq!(
        AiDelegate::from_delegate_index(16),
        Some(AiDelegate::Marquee)
    );
    assert_eq!(
        AiDelegate::from_delegate_index(19),
        Some(AiDelegate::Photographer)
    );
    assert_eq!(
        MotionDelegate::from_delegate_index(17),
        Some(MotionDelegate::Marquee)
    );
    assert_eq!(
        MotionDelegate::from_delegate_index(18),
        Some(MotionDelegate::Photographer)
    );
}

#[test]
fn facing_skater_reads_the_flattened_offset_against_cos_fov_and_the_distance() {
    let at_origin = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    assert!(is_facing_skater(&at_origin, [0., 1.4, 3., 1.], 1.4, 6.));
    assert!(!is_facing_skater(&at_origin, [0., 0., -3., 1.], 1.4, 6.));
    assert!(!is_facing_skater(&at_origin, [0., 0., 7., 1.], 1.4, 6.));
    assert!(is_facing_skater(&at_origin, [0., 0., 70., 1.], 1.4, 0.));
    // 80 degrees off the facing passes a 1.4 rad (80.2 degree) cosine; 81 does not.
    let (s80, c80) = 80_f32.to_radians().sin_cos();
    let (s81, c81) = 81_f32.to_radians().sin_cos();
    assert!(is_facing_skater(
        &at_origin,
        [3. * s80, 0., 3. * c80, 1.],
        1.4,
        6.
    ));
    assert!(!is_facing_skater(
        &at_origin,
        [3. * s81, 0., 3. * c81, 1.],
        1.4,
        6.
    ));
}

#[test]
fn pitch_to_target_is_degrees_above_the_horizontal() {
    let pitch = pitch_to_target([0., 0., 0., 1.], [1., 1., 0., 1.]).unwrap();
    assert!((pitch - 45.).abs() < 1e-3, "{pitch}");
    let below = pitch_to_target([0., 2., 0., 1.], [0., 1., 3., 1.]).unwrap();
    assert!((below + 18.434_95).abs() < 1e-3, "{below}");
}

/// Answers motion conditions from a set and records the callbacks.
#[derive(Default)]
struct MotionRecorder {
    conditions: BTreeSet<u16>,
    calls: Vec<String>,
}

impl PedestrianMotionBackend<u32> for MotionRecorder {
    fn condition(
        &mut self,
        _: &BoundMotionGraphContext<u32>,
        _: &ControllerState<'_>,
        condition: u16,
        _: u16,
    ) -> Result<bool, Error> {
        Ok(self.conditions.contains(&condition))
    }
    fn enabled(&self, _: u16) -> Result<bool, Error> {
        Ok(true)
    }
    fn create_instance(&mut self, _: u16) -> Result<Option<MotionGraphInstanceToken>, Error> {
        Ok(None)
    }
    fn begin(
        &mut self,
        _: &BoundMotionGraphContext<u32>,
        _: &ControllerState<'_>,
        occurrence: u16,
    ) -> Result<(), Error> {
        self.calls.push(format!("begin {occurrence}"));
        Ok(())
    }
    fn end(
        &mut self,
        _: &BoundMotionGraphContext<u32>,
        _: &ControllerState<'_>,
        occurrence: u16,
    ) -> Result<(), Error> {
        self.calls.push(format!("end {occurrence}"));
        Ok(())
    }
    fn release_instance(
        &mut self,
        _: u16,
        _: Option<MotionGraphInstanceToken>,
    ) -> Result<(), Error> {
        Ok(())
    }
    fn generated_update(
        &mut self,
        _: &mut BoundMotionGraphContext<u32>,
        _: &ControllerState<'_>,
        source: u32,
        active: &[ActiveBehaviour],
    ) -> Result<(), Error> {
        let active: Vec<u16> = active.iter().map(|a| a.occurrence).collect();
        self.calls.push(format!("update {source:#x} {active:?}"));
        Ok(())
    }
}

impl MotionRecorder {
    fn step(
        &mut self,
        delegate: &mut PedestrianMotionDelegate<u32>,
        conditions: &[u16],
    ) -> Vec<String> {
        self.conditions = conditions.iter().copied().collect();
        self.calls.clear();
        delegate.update(1. / 30., self).unwrap();
        std::mem::take(&mut self.calls)
    }
}

/// motiongraph_marquee.xml conditions: 0 HasIntent Collision, 1 HasIntent FromFront,
/// 2 WillExpire.
#[test]
fn motiongraph_marquee_idles_and_plays_a_standing_collision() {
    let authority = ActorGraphAuthority::new(0_u32);
    let mut motion = PedestrianMotionDelegate::new(&authority, true, MotionDelegate::Marquee);
    let mut backend = MotionRecorder::default();
    assert_eq!(
        backend.step(&mut motion, &[]),
        ["begin 0", "update 0x828f5c28 [0]"]
    );
    assert_eq!(
        backend.step(&mut motion, &[0, 1]),
        ["end 0", "begin 1", "begin 4", "update 0x828f89c0 [1, 4]"]
    );
    assert_eq!(
        backend.step(&mut motion, &[0, 1, 2]),
        [
            "end 4",
            "end 1",
            "begin 2",
            "begin 3",
            "update 0x828f81e8 [2, 3]"
        ]
    );
    assert_eq!(
        backend.step(&mut motion, &[]),
        ["end 3", "end 2", "begin 0", "update 0x828f5c28 [0]"]
    );
}

/// Motiongraph_Photographer.xml conditions: 2 WillExpire, 6 HasIntent PhotographSkater,
/// 7 LastState Photographer.Idle.
#[test]
fn motiongraph_photographer_tracks_its_target_from_idle() {
    let authority = ActorGraphAuthority::new(0_u32);
    let mut motion = PedestrianMotionDelegate::new(&authority, true, MotionDelegate::Photographer);
    let mut backend = MotionRecorder::default();
    assert_eq!(
        backend.step(&mut motion, &[]),
        ["begin 4", "update 0x828f5c28 [4]"]
    );
    assert_eq!(
        backend.step(&mut motion, &[6, 7]),
        ["end 4", "begin 0", "update 0x828f5c28 [0]"]
    );
    assert_eq!(
        backend.step(&mut motion, &[2, 6]),
        ["end 0", "begin 1", "update 0x828f5c28 [1]"]
    );
}

/// The challenge characters' entities name the marquee graphs, carry the pedestrian
/// resources the spawn binds, and every clip their graphs play remaps to an animation of
/// `PedestrianSkeletonPres.abin`, except the photographers' unreachable review clip, which
/// the shipped database does not hold.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn challenge_characters_name_the_marquee_graphs_and_their_clips() {
    let root =
        std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT"));
    let mut mounts = MountSet::new();
    for spec in SKATE2_ARCHIVES {
        let path = spec.path_under(&root);
        if path.is_file() {
            mounts.push(MountedBig::open(path, spec.use_external_header).unwrap());
        }
    }
    let vault = AttributeRuntimeVault::try_load_mounted(
        &mounts,
        "data/db/skater.vlt",
        &crate::selected_original_asset_classes::selected_classes(),
    )
    .unwrap();
    let database = AndaleDatabase::parse(
        &mounts
            .read_path("data\\anim/PedestrianSkeletonPres.abin")
            .unwrap(),
    )
    .unwrap();
    let delegate_index = |path: &[u8]| match select_onboard_controller(
        path,
        true,
        false,
        GeneratedOnboardEnablement::from_delegate_raw([1; 20]),
    )
    .unwrap()
    {
        OnboardControllerSelection::GeneratedOther { delegate_index } => delegate_index,
        other => panic!("{other:?}"),
    };
    for (name, ai, motion) in [
        ("slappy", AiDelegate::Marquee, MotionDelegate::Marquee),
        (
            "giovanni_reda",
            AiDelegate::Marquee,
            MotionDelegate::Marquee,
        ),
        ("cuz", AiDelegate::Marquee, MotionDelegate::Marquee),
        ("mike", AiDelegate::Marquee, MotionDelegate::Marquee),
        (
            "attiba_jefferson",
            AiDelegate::Photographer,
            MotionDelegate::Photographer,
        ),
        (
            "michael_burnette",
            AiDelegate::Photographer,
            MotionDelegate::Photographer,
        ),
    ] {
        let key = string_hash64(name);
        PedestrianResources::load(&vault, key, 0).unwrap();
        let entity = vault.instance(ENTITIES_CLASS, key);
        let path = entity.try_get_text(AI_GRAPH, 0).unwrap().expect("AI graph");
        assert_eq!(
            AiDelegate::from_delegate_index(delegate_index(path.bytes())),
            Some(ai),
            "{name}"
        );
        let data = entity
            .try_ref_spec_instance(ANIMATION_DATA)
            .unwrap()
            .expect("animation data");
        let path = data
            .try_cached_layout_text(0)
            .unwrap()
            .expect("motion graph");
        assert_eq!(
            MotionDelegate::from_delegate_index(delegate_index(path.bytes())),
            Some(motion),
            "{name}"
        );
        let declarations = motion.declarations();
        let mut clips = vec!["GreetChannel"];
        for behaviour in declarations
            .behaviours
            .iter()
            .filter(|b| b.name == "PlayRemappedAnimation")
        {
            let fields = attributes(declarations.attributes, behaviour.attributes).unwrap();
            clips.push(
                fields
                    .iter()
                    .find(|field| field.key == "anim")
                    .expect("anim")
                    .value,
            );
        }
        for clip in clips {
            let mapped =
                remap_animation(clip, &PedestrianRegisteredClips::default(), &data, || 0).unwrap();
            let encoded = FastString36::encode(&mapped).unwrap();
            let present = database.records.iter().any(|record| {
                record.header.name == encoded && matches!(record.header.data_type, 2 | 6..=10)
            });
            assert_eq!(
                present,
                clip != "PHOT_SHOOT_REVIEW_N_0_N",
                "{name} {clip} -> {mapped}"
            );
        }
    }
}
