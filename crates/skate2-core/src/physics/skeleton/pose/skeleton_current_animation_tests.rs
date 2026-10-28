use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GROUND, PHYSICS_CATEGORY_OFFBOARD, PHYSICS_STATE_LANDINGONDECK,
    PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_LOCOMOTION, PHYSICS_STATE_PHYSICS_GROUND,
};
use super::*;
use skate2_formats::andale::PackedFastString;
use std::{cell::RefCell, rc::Rc};

type Events = Rc<RefCell<Vec<&'static str>>>;
type Error = SkeletonCurrentAnimationError<&'static str, &'static str>;

#[path = "../ik/skeleton_ik_integration_tests.rs"]
mod ik_integration;

struct Attributes {
    events: Events,
    value: Result<f32, &'static str>,
    landing_blend: Result<f32, &'static str>,
    spring_failure: Option<SkeletonLandingAdjustScalar>,
}
impl SkeletonLandingAdjustAttributes for Attributes {
    type Error = &'static str;
    fn scalar(&mut self, attribute: SkeletonLandingAdjustScalar) -> Result<f32, Self::Error> {
        use SkeletonLandingAdjustScalar as S;
        let (name, value) = match attribute {
            S::MaximumVelocity => {
                self.events.borrow_mut().push("attribute");
                return self.value;
            }
            S::CoffinDuration => ("duration", 1.0),
            S::CoffinVelocityDenominator => ("coffin denominator", 10.0),
            S::CoffinBase => ("coffin base", 0.0),
            S::DesiredCom => ("desired", 0.0),
            S::MinimumYPosition => ("minimum", -1000.0),
            S::GroundMinimumElapsed => ("elapsed", 0.1),
            S::CoffinFrames => ("coffin frames", 3.0),
            S::GrindTargetSpeed => ("grind speed", 0.125),
            S::GrindTargetAfterTime => ("grind after", 0.25),
            S::GroundK1 => ("ground K1", 4.0),
            S::GroundK2 => ("ground K2", 2.0),
            S::GrindK1 => ("grind K1", 4.0),
            S::GrindK2 => ("grind K2", 2.0),
            S::ManualK1 => ("manual K1", 4.0),
            S::ManualK2 => ("manual K2", 2.0),
        };
        self.events.borrow_mut().push(name);
        if self.spring_failure == Some(attribute) {
            Err(name)
        } else {
            Ok(value)
        }
    }
    fn manual_blend(&mut self, _: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("manual curve");
        Ok(1.0)
    }
    fn grind_blend(&mut self, _: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("grind curve");
        Ok(1.0)
    }
    fn coffin_bounce(&mut self, _: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("coffin curve");
        Ok(0.25)
    }
}
impl SkeletonCurrentAnimationAttributeRead for Attributes {
    fn landing_on_board_blend(&mut self, _: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("landing blend");
        self.landing_blend
    }
}
struct Cvars {
    events: Events,
    debug: Result<u8, &'static str>,
    com: Result<u8, &'static str>,
    offboard: [Result<f32, &'static str>; 2],
    offboard_reads: usize,
    landing_reads: usize,
    landing_failure: Option<&'static str>,
}
impl Cvars {
    fn landing_sample(&mut self, name: &'static str, value: u8) -> Result<u8, &'static str> {
        self.events.borrow_mut().push(name);
        if self.landing_failure == Some(name) {
            Err(name)
        } else {
            Ok(value)
        }
    }
}
impl SkeletonAnimationRecordInput for Cvars {
    type Error = &'static str;
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.events.borrow_mut().push("com");
        self.com
    }
}
impl SkeletonCurrentAnimationCvars for Cvars {
    fn landing_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.events.borrow_mut().push("spring overlay");
        Err("overlay unavailable")
    }
    fn landing_use_physics_com(&mut self) -> Result<u8, Self::Error> {
        self.landing_sample("use physics COM", 0)
    }
    fn landing_first_frame_hack(&mut self) -> Result<f32, Self::Error> {
        self.landing_sample("spring first frame", 0)?;
        Ok(0.01)
    }
    fn landing_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.landing_sample("spring frames", 0)?;
        Ok(3.0)
    }
    fn landing_on_board_show_adjust(&mut self) -> Result<u8, Self::Error> {
        self.landing_sample("landing show", 0)
    }
    fn landing_on_board_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("show is disabled in the pipeline fixture")
    }
    fn landing_on_board_negate_y(&mut self) -> Result<u8, Self::Error> {
        self.landing_sample("landing negate", 1)
    }
    fn landing_on_board_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.landing_reads += 1;
        let (name, value) = match self.landing_reads {
            1 => ("landing full", 4.0),
            2 => ("landing Y", 6.0),
            _ => panic!("exactly two landing blend samples per attempt"),
        };
        self.landing_sample(name, 0)?;
        Ok(value)
    }
    fn debug_landing_adjust(&mut self) -> Result<u8, Self::Error> {
        self.events.borrow_mut().push("debug");
        self.debug
    }
    fn offboard_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        let index = self.offboard_reads;
        self.offboard_reads += 1;
        self.events.borrow_mut().push(if index == 0 {
            "offboard full"
        } else {
            "offboard Y"
        });
        self.offboard[index]
    }
}
struct Numeric {
    events: Events,
    result: f32,
    arguments: Vec<([f32; 4], [f32; 4])>,
}
impl SkeletonPhysicsHistoryNumeric for Numeric {
    fn dot3(&mut self, displacement: [f32; 4], up: [f32; 4]) -> f32 {
        self.events.borrow_mut().push("dot");
        self.arguments.push((displacement, up));
        self.result
    }
}

struct Fixture {
    current: SkeletonCurrentAnimation,
    targets: SkeletonAnimationTargets,
    pose: Vec<AnimationMatrix>,
    masses: SkeletonPhysicsHistory,
    root: SkeletonRootMatrix,
    input: SkeletonAdjustmentInput,
    delta: f32,
    events: Events,
    attributes: Attributes,
    cvars: Cvars,
    numeric: Numeric,
    retained_com_y: f32,
    world_to_animation: AnimationMatrix,
}
impl Fixture {
    fn new() -> Self {
        Self::with_hand_offset([0.0; 4])
    }
    fn with_hand_offset(hand_offset: [f32; 4]) -> Self {
        let (mut hierarchy, mut parameters, mut pose) = crate::skeleton_data_tests::fixture();
        parameters.bones[3].position = hand_offset;
        parameters.bones[7].position = hand_offset;
        for name in [
            "LeftToeBase_Reparented",
            "RightToeBase_Reparented",
            "LeftHand_Reparented",
            "RightHand_Reparented",
        ] {
            hierarchy.bone_names.push(name.into());
            hierarchy
                .packed_bone_names
                .push(PackedFastString::encode(name).unwrap());
            hierarchy.parents.push(-1);
            hierarchy.bone_metadata.push(0);
            pose.push(FRESH_VOLUME_TRANSFORM);
        }
        let data =
            Arc::new(crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap());
        let mut current = SkeletonCurrentAnimation::new(data);
        current.initialize_ik_targets();
        let mut part_masses = [0.0; 24];
        part_masses[1] = 1.0;
        let mut masses = SkeletonPhysicsHistory::new(part_masses);
        masses.try_initialize_masses().unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        Self {
            current,
            targets: SkeletonAnimationTargets::new(),
            pose,
            masses,
            root: SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
            input: SkeletonAdjustmentInput {
                category: PHYSICS_CATEGORY_GROUND,
                state: PHYSICS_STATE_PHYSICS_GROUND,
                filtered_state: 1,
                flags_8c4: 0,
                flags_8c8: 0,
                flags_8cc: 0,
                balance: 0.0,
                centre_of_mass_velocity: [3.0, 5.0, 7.0, 11.0],
                up: [13.0, 17.0, 19.0, 23.0],
                landing_on_board: SkeletonLandingOnBoardInput {
                    flags_8d0: 0,
                    deck_transform: FRESH_VOLUME_TRANSFORM,
                    landing_on_deck_adjust: [0.0; 4],
                    time_to_land: 0.25,
                },
            },
            delta: -99.0,
            attributes: Attributes {
                events: Rc::clone(&events),
                value: Ok(50.0),
                landing_blend: Ok(1.0),
                spring_failure: None,
            },
            cvars: Cvars {
                events: Rc::clone(&events),
                debug: Ok(0),
                com: Ok(1),
                offboard: [Ok(3.0), Ok(5.0)],
                offboard_reads: 0,
                landing_reads: 0,
                landing_failure: None,
            },
            numeric: Numeric {
                events: Rc::clone(&events),
                result: 29.0,
                arguments: Vec::new(),
            },
            events,
            retained_com_y: 0.0,
            world_to_animation: FRESH_VOLUME_TRANSFORM,
        }
    }

    fn process(&mut self) -> Result<(), Error> {
        self.current.try_process(
            &mut self.targets,
            &self.pose,
            &self.input,
            &mut self.delta,
            &self.masses,
            &self.root,
            &self.world_to_animation,
            self.retained_com_y,
            &mut self.attributes,
            &mut self.cvars,
            &mut self.numeric,
        )
    }
    fn adjust(&mut self) -> Result<(), Error> {
        let mapped = self.current.raw_skateboard;
        self.current.try_adjust(
            &self.input,
            &mapped,
            &self.world_to_animation,
            self.retained_com_y,
            &self.masses,
            &mut self.attributes,
            &mut self.cvars,
            &mut self.numeric,
        )
    }
    fn assert_events(&self, expected: &[&str]) {
        assert_eq!(&*self.events.borrow(), expected);
    }
}

#[test]
fn selected_landing_uses_retained_velocity_then_flip_gate_without_consuming_services() {
    for retained in [0.0, -0.0, 1.0, f32::INFINITY, f32::NAN] {
        let mut f = Fixture::new();
        f.input.flags_8cc = 0x200;
        f.input.centre_of_mass_velocity[1] = -99.0;
        f.retained_com_y = retained;
        f.attributes.landing_blend = Err("must not fetch");
        f.adjust().unwrap();
        f.assert_events(&["attribute", "dot", "debug"]);
        assert_eq!(f.cvars.landing_reads, 0);
    }
    let mut f = Fixture::new();
    f.input.flags_8cc = 0x200;
    f.retained_com_y = -1.0;
    f.input.landing_on_board.flags_8d0 = 0x10;
    f.attributes.landing_blend = Err("must not fetch");
    f.adjust().unwrap();
    f.assert_events(&["attribute", "dot", "debug"]);
}

#[test]
fn active_landing_sets_distinct_offsets_before_processed_velocity_history() {
    let mut f = Fixture::new();
    f.input.flags_8cc = 0x200;
    f.retained_com_y = -3.0;
    f.input.centre_of_mass_velocity[1] = 17.0;
    f.input.landing_on_board.deck_transform[13] = 2.0;
    f.input.landing_on_board.landing_on_deck_adjust = [3.0, 99.0, 4.0, 777.0];
    f.adjust().unwrap();
    f.assert_events(&[
        "landing blend",
        "landing show",
        "landing negate",
        "landing full",
        "landing Y",
        "attribute",
        "dot",
        "debug",
    ]);
    assert_eq!(
        f.current.offset.transform[12..15]
            .iter()
            .copied()
            .map(f32::to_bits)
            .collect::<Vec<_>>(),
        [3.0_f32, -2.0, 4.0].map(f32::to_bits)
    );
    assert_eq!(f.current.offset.full_count.to_bits(), 4.0_f32.to_bits());
    assert_eq!(f.current.offset.vertical_count.to_bits(), 6.0_f32.to_bits());
    assert!(f.current.offset.full_just_set && f.current.offset.vertical_just_set);
    assert_eq!(f.numeric.arguments[0].0[1].to_bits(), 17.0_f32.to_bits());
    assert_eq!(
        f.current.second_last_com_velocity().to_bits(),
        29.0_f32.to_bits()
    );
}

#[test]
fn landing_second_blend_failure_keeps_zero_y_full_prefix_and_old_history() {
    let mut f = Fixture::new();
    f.input.flags_8cc = 0x200;
    f.retained_com_y = -1.0;
    f.input.landing_on_board.deck_transform[13] = 2.0;
    f.input.landing_on_board.landing_on_deck_adjust[0] = 3.0;
    f.cvars.landing_failure = Some("landing Y");
    assert!(matches!(
        f.process(),
        Err(SkeletonCurrentAnimationError::LandingOnBoard(
            SkeletonLandingOnBoardError::Cvar("landing Y")
        ))
    ));
    f.assert_events(&[
        "landing blend",
        "landing show",
        "landing negate",
        "landing full",
        "landing Y",
    ]);
    assert_eq!(f.current.offset.transform[13].to_bits(), 0);
    assert_eq!(f.current.offset.full_count.to_bits(), 4.0_f32.to_bits());
    assert_eq!(f.current.offset.vertical_count.to_bits(), 0);
    assert!(f.current.offset.full_just_set && !f.current.offset.vertical_just_set);
    assert_eq!(f.current.last_filtered_state(), 0);
    assert!(f.current.root_input(1.0).is_none());
}

#[test]
fn landing_curve_reads_inherited_graph_payload_and_only_missing_uses_zero_default() {
    use crate::attribute_runtime::tests::{
        collection, definition, graph, node, set_dependency_bytes,
    };
    const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
    let mut vault = graph(
        PHYSICS_ANIMATION_CLASS,
        vec![definition(LANDING_ON_BOARD_BLEND, 0, 0x50)],
    );
    let mut bytes = vec![0; 256];
    // Metadata is deliberately NaN: this caller only reads x/y points.
    for lane in 0..4 {
        bytes[64 + 4 * lane..68 + 4 * lane].copy_from_slice(&f32::NAN.to_bits().to_be_bytes());
    }
    for index in 0..8_u8 {
        let offset = usize::from(index) * 4;
        bytes[80 + offset..84 + offset].copy_from_slice(&f32::from(index).to_bits().to_be_bytes());
        bytes[112 + offset..116 + offset]
            .copy_from_slice(&(f32::from(index) * 2.0).to_bits().to_be_bytes());
    }
    set_dependency_bytes(&mut vault, 1, bytes);
    collection(
        &mut vault,
        PHYSICS_ANIMATION_CLASS,
        7,
        0,
        Some(64),
        vec![node(LANDING_ON_BOARD_BLEND, 0x10, 0)],
    );
    collection(
        &mut vault,
        PHYSICS_ANIMATION_CLASS,
        DEFAULT,
        7,
        None,
        vec![],
    );
    let mut attributes = SkeletonCurrentAnimationAttributes::bind(&Arc::new(vault));
    assert_eq!(
        attributes.landing_on_board_blend(2.5).unwrap().to_bits(),
        5.0_f32.to_bits()
    );
    assert_eq!(
        attributes.landing_on_board_blend(4.5).unwrap().to_bits(),
        9.0_f32.to_bits()
    );
    let empty = Arc::new(graph(PHYSICS_ANIMATION_CLASS, vec![]));
    assert_eq!(
        SkeletonCurrentAnimationAttributes::bind(&empty)
            .landing_on_board_blend(f32::NAN)
            .unwrap()
            .to_bits(),
        0
    );
    let mut malformed = graph(
        PHYSICS_ANIMATION_CLASS,
        vec![definition(LANDING_ON_BOARD_BLEND, 0, 4)],
    );
    collection(
        &mut malformed,
        PHYSICS_ANIMATION_CLASS,
        DEFAULT,
        0,
        Some(64),
        vec![node(LANDING_ON_BOARD_BLEND, 0x10, 0)],
    );
    assert!(matches!(
        SkeletonCurrentAnimationAttributes::bind(&Arc::new(malformed)).landing_on_board_blend(0.0),
        Err(AttributeRuntimeError::PayloadSize {
            expected: 0x50,
            actual: 4,
            ..
        })
    ));
}

#[test]
fn missing_third_ik_target_preserves_complete_mapping_raw_copy_and_two_targets() {
    let mut f = Fixture::new();
    f.pose[0][12] = 7.0;
    f.pose[23][13] = 23.0;
    f.pose[24][12] = 31.0;
    f.pose[25][12] = 32.0;
    f.pose.truncate(26);
    assert!(matches!(
        f.process(),
        Err(SkeletonCurrentAnimationError::Ik(
            SkeletonIkTargetError::PoseUnavailable {
                target: 2,
                bone: Some(26),
                pose_count: 26,
            }
        ))
    ));
    assert!(f.targets.completed_volumes().is_some());
    assert_eq!(f.current.raw_skateboard()[12].to_bits(), 7.0_f32.to_bits());
    assert_eq!(
        f.current.ik().targets()[0][12].to_bits(),
        31.0_f32.to_bits()
    );
    assert_eq!(
        f.current.ik().targets()[1][12].to_bits(),
        32.0_f32.to_bits()
    );
    assert_eq!(f.current.ik().targets()[2][12].to_bits(), 0);
    assert_eq!(f.current.hips()[13].to_bits(), 0);
    assert_eq!(f.current.record().poses()[23][13].to_bits(), 0);
    assert_eq!(f.delta.to_bits(), (-99.0_f32).to_bits());
    assert!(f.current.root_input(0.0).is_none());
    f.assert_events(&[]);
}

#[test]
fn short_mapped_pose_stops_before_raw_copy_and_ik() {
    let mut f = Fixture::new();
    f.pose[0][12] = 7.0;
    f.pose.truncate(3);
    assert!(matches!(
        f.process(),
        Err(SkeletonCurrentAnimationError::Targets(
            SkeletonAnimationTargetError {
                part: 3,
                bone: 3,
                pose_count: 3,
            }
        ))
    ));
    assert!(f.targets.completed_volumes().is_none());
    assert_eq!(
        f.targets.volume_rows()[0].unwrap()[12].to_bits(),
        7.0_f32.to_bits()
    );
    assert_eq!(f.current.raw_skateboard()[12].to_bits(), 0);
    assert_eq!(f.current.ik().targets()[0][12].to_bits(), 0);
    f.assert_events(&[]);
}

#[test]
fn repeated_stable_ground_keeps_com_deck_history_and_samples_services_each_time() {
    let mut f = Fixture::new();
    f.pose[0][9] = 0.25;
    f.pose[0][13] = 2.0;
    f.pose[1][13] = 8.0;
    f.pose[23][12] = 18.0;
    f.process().unwrap();
    f.assert_events(&["attribute", "dot", "debug", "com"]);
    assert_eq!(f.delta.to_bits(), 0.25_f32.to_bits());
    assert_eq!(
        f.current.record().centre_of_mass()[1].to_bits(),
        8.0_f32.to_bits()
    );
    assert_eq!(
        f.current.record().com_to_deck().xyz[1].to_bits(),
        6.0_f32.to_bits()
    );
    assert_eq!(f.current.record().com_to_deck_delta().xyz[1].to_bits(), 0);
    assert_eq!(f.current.hips()[12].to_bits(), 18.0_f32.to_bits());
    assert_eq!(f.current.last_filtered_state(), 1);
    assert_eq!(
        f.current.second_last_com_velocity().to_bits(),
        29.0_f32.to_bits()
    );
    assert_eq!(
        f.numeric.arguments[0].0.map(f32::to_bits),
        f.input.centre_of_mass_velocity.map(f32::to_bits)
    );
    assert_eq!(
        f.numeric.arguments[0].1.map(f32::to_bits),
        f.input.up.map(f32::to_bits)
    );
    f.events.borrow_mut().clear();
    f.pose[0][9] = -0.5;
    f.pose[0][13] = 3.0;
    f.pose[1][13] = 12.0;
    f.numeric.result = 37.0;
    f.process().unwrap();
    f.assert_events(&["attribute", "dot", "debug", "com"]);
    assert_eq!(f.delta.to_bits(), (-0.75_f32).to_bits());
    assert_eq!(f.current.last_deck_zy().to_bits(), (-0.5_f32).to_bits());
    assert_eq!(
        f.current.record().centre_of_mass_delta()[1].to_bits(),
        4.0_f32.to_bits()
    );
    assert_eq!(
        f.current.record().com_to_deck_delta().xyz[1].to_bits(),
        3.0_f32.to_bits()
    );
    assert_eq!(
        f.current.second_last_com_velocity().to_bits(),
        37.0_f32.to_bits()
    );
    let root_input = f.current.root_input(0.125).unwrap();
    assert_eq!(root_input.time_delta.to_bits(), 0.125_f32.to_bits());
    assert_eq!(
        root_input.skate_animation[9].to_bits(),
        (-0.5_f32).to_bits()
    );
}

#[test]
fn late_com_failure_retains_new_prefix_but_old_adjusted_copies_and_passed_delta() {
    let mut f = Fixture::new();
    f.pose[0][9] = 0.25;
    f.pose[1][13] = 5.0;
    f.pose[23][12] = 9.0;
    f.process().unwrap();
    f.events.borrow_mut().clear();
    f.delta = -71.0;
    f.pose[0][9] = 0.75;
    f.pose[1][13] = 15.0;
    f.pose[23][12] = 19.0;
    f.pose[27][12] = 41.0;
    f.numeric.result = 53.0;
    f.cvars.com = Err("COM unavailable");
    assert!(matches!(
        f.process(),
        Err(SkeletonCurrentAnimationError::Record(
            SkeletonAnimationRecordError::Cvar("COM unavailable")
        ))
    ));
    f.assert_events(&["attribute", "dot", "debug", "com"]);
    assert_eq!(f.current.raw_skateboard()[9].to_bits(), 0.75_f32.to_bits());
    assert_eq!(
        f.current.ik().targets()[3][12].to_bits(),
        41.0_f32.to_bits()
    );
    assert_eq!(
        f.current.record().poses()[23][12].to_bits(),
        19.0_f32.to_bits()
    );
    assert_eq!(
        f.current.record().centre_of_mass()[1].to_bits(),
        5.0_f32.to_bits()
    );
    assert_eq!(f.current.hips()[12].to_bits(), 9.0_f32.to_bits());
    assert_eq!(f.current.last_deck_zy().to_bits(), 0.25_f32.to_bits());
    assert_eq!(f.delta.to_bits(), (-71.0_f32).to_bits());
    assert_eq!(
        f.current.second_last_com_velocity().to_bits(),
        53.0_f32.to_bits()
    );
    assert!(f.current.root_input(1.0).is_none());
}

#[test]
fn landing_fetch_precedes_disable_then_dot_debug_and_final_history_stores() {
    let mut f = Fixture::new();
    f.current.landing.seed_history(true, 6, 101.0);
    f.input.flags_8c8 = 1;
    f.attributes.value = Err("missing service");
    assert!(matches!(
        f.adjust(),
        Err(SkeletonCurrentAnimationError::Attribute("missing service"))
    ));
    f.assert_events(&["attribute"]);
    assert!(f.current.landing.adjusting());
    f.events.borrow_mut().clear();
    f.attributes.value = Ok(50.0);
    f.cvars.debug = Err("debug unavailable");
    assert!(matches!(
        f.adjust(),
        Err(SkeletonCurrentAnimationError::Cvar("debug unavailable"))
    ));
    f.assert_events(&["attribute", "dot", "debug"]);
    assert!(!f.current.landing.adjusting());
    assert_eq!(f.current.last_filtered_state(), 6);
    assert_eq!(
        f.current.second_last_com_velocity().to_bits(),
        101.0_f32.to_bits()
    );
    f.events.borrow_mut().clear();
    f.cvars.debug = Ok(0);
    f.adjust().unwrap();
    f.assert_events(&["attribute", "dot", "debug"]);
    assert_eq!(f.current.last_filtered_state(), 1);
    assert_eq!(
        f.current.second_last_com_velocity().to_bits(),
        29.0_f32.to_bits()
    );
}

#[test]
fn landing_transition_flags_and_unordered_balance_keep_exact_start_boundary() {
    // old/current filtered state, balance, flags8c4, flags8c8, flags8cc, starts.
    let cases = [
        (2, 1, 0.0, 0, 0, 0, false),
        (2, 1, -0.0, 0, 0, 0, false),
        (2, 1, f32::NAN, 0, 0, 0, true),
        (6, 1, f32::NAN, 0, 0, 0, false),
        (6, 1, 0.0, 0, 0, 0x4000_0000, true),
        (2, 3, 0.0, 0, 0, 0, true),
        (2, 3, 0.0, 0x20, 0, 0, false),
        (2, 1, 0.0, 0, 4, 0, true),
        (2, 1, 1.0, 0, 1, 0, false),
        (2, 1, 1.0, 0, 5, 0, true),
        (1, 1, 1.0, 0, 4, 0x4000_0000, false),
    ];
    for (old, current, balance, flags_a, flags_b, flags_c, starts) in cases {
        let mut f = Fixture::new();
        f.current.landing.seed_history(false, old, 101.0);
        f.attributes.spring_failure = Some(SkeletonLandingAdjustScalar::CoffinDuration);
        f.input.filtered_state = current;
        f.input.balance = balance;
        f.input.flags_8c4 = flags_a;
        f.input.flags_8c8 = flags_b;
        f.input.flags_8cc = flags_c;
        let result = f.adjust();
        if starts {
            assert!(matches!(
                result,
                Err(SkeletonCurrentAnimationError::Attribute("duration"))
            ));
            assert!(f.current.landing.adjusting());
            assert_eq!(f.current.landing.time().to_bits(), 0);
            assert_eq!(
                f.current.landing.last_animation_position().to_bits(),
                f32::MAX.to_bits()
            );
            assert_eq!(f.current.last_filtered_state(), old);
            assert_eq!(
                f.current.second_last_com_velocity().to_bits(),
                101.0_f32.to_bits()
            );
        } else {
            result.unwrap();
            assert_eq!(f.current.last_filtered_state(), current);
            assert!(!f.current.landing.adjusting());
        }
        f.assert_events(if starts {
            &[
                "attribute",
                "dot",
                "debug",
                "dot",
                "debug",
                "use physics COM",
                "duration",
            ]
        } else {
            &["attribute", "dot", "debug"]
        });
    }
}

#[test]
fn landing_debug_precedes_existing_spring_and_preserves_histories() {
    let mut f = Fixture::new();
    f.current.landing.seed_history(true, 2, 0.0);
    f.cvars.debug = Ok(1);
    assert!(matches!(
        f.adjust(),
        Err(SkeletonCurrentAnimationError::LandingAdjust(
            SkeletonLandingAdjustError::DebugOverlay("overlay unavailable")
        ))
    ));
    assert!(f.current.landing.adjusting());
    assert_eq!(f.current.last_filtered_state(), 2);
    assert_eq!(f.current.second_last_com_velocity().to_bits(), 0);
    f.events.borrow_mut().clear();
    f.cvars.debug = Ok(0);
    f.attributes.spring_failure = Some(SkeletonLandingAdjustScalar::CoffinDuration);
    assert!(matches!(
        f.adjust(),
        Err(SkeletonCurrentAnimationError::Attribute("duration"))
    ));
    f.assert_events(&["attribute", "dot", "debug", "use physics COM", "duration"]);
    assert_eq!(f.current.last_filtered_state(), 2);
}

#[test]
fn active_offset_advances_before_com_failure_and_applies_on_final_count_tick() {
    let mut f = Fixture::new();
    f.current.offset.transform[12..].copy_from_slice(&[4.0, 3.0, 2.0, 0.0]);
    f.current.offset.full_count = 2.0;
    f.current.offset.full_just_set = true;
    f.process().unwrap();
    assert_eq!(f.current.offset.full_count.to_bits(), 2.0_f32.to_bits());
    assert!(!f.current.offset.full_just_set);
    assert_eq!(f.current.raw_skateboard()[12].to_bits(), 0);
    assert_eq!(
        f.current.record().poses()[0][12].to_bits(),
        4.0_f32.to_bits()
    );
    for target in f.current.ik().targets() {
        assert_eq!(
            target[12..15]
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            [4.0_f32, 3.0, 2.0].map(f32::to_bits)
        );
    }
    f.delta = 71.0;
    f.cvars.com = Err("late COM");
    assert!(matches!(
        f.process(),
        Err(SkeletonCurrentAnimationError::Record(
            SkeletonAnimationRecordError::Cvar("late COM")
        ))
    ));
    assert_eq!(f.current.offset.full_count.to_bits(), 1.0_f32.to_bits());
    assert_eq!(
        f.current.offset.transform[12..15]
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        [1.0_f32, 3.0, 0.5].map(f32::to_bits)
    );
    assert_eq!(
        f.current.record().poses()[0][12].to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(
        f.targets.completed_volumes().unwrap()[0][12].to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(f.current.ik().targets()[3][12].to_bits(), 1.0_f32.to_bits());
    assert_eq!(f.delta.to_bits(), 71.0_f32.to_bits());
    assert!(f.current.root_input(1.0).is_none());
    f.cvars.com = Ok(1);
    f.process().unwrap();
    assert_eq!(f.current.offset.full_count.to_bits(), 0);
    // The gate was active BEFORE decrement: retained Y still applies this call.
    assert_eq!(
        f.current.record().poses()[0][13].to_bits(),
        3.0_f32.to_bits()
    );
    assert_eq!(f.current.ik().targets()[0][13].to_bits(), 3.0_f32.to_bits());
    f.process().unwrap();
    // Next call neither counter is active, despite the same retained P.Y.
    assert_eq!(f.current.offset.transform[13].to_bits(), 3.0_f32.to_bits());
    assert_eq!(f.current.record().poses()[0][13].to_bits(), 0);
    assert_eq!(f.current.ik().targets()[0][13].to_bits(), 0);
}

#[test]
fn unordered_offset_counts_are_inactive_and_completed_stage_clears_just_set() {
    let mut f = Fixture::new();
    f.current.offset.full_count = f32::NAN;
    f.current.offset.vertical_count = f32::NAN;
    f.current.offset.full_just_set = true;
    f.current.offset.vertical_just_set = true;
    f.current.offset.transform[12] = 7.0;
    let original_transform = f.current.offset.transform.map(f32::to_bits);
    // Debug fails before the offset stage and preserves both JustSet bytes.
    f.cvars.debug = Err("early debug");
    assert!(matches!(
        f.process(),
        Err(SkeletonCurrentAnimationError::Cvar("early debug"))
    ));
    assert!(f.current.offset.full_just_set && f.current.offset.vertical_just_set);
    f.cvars.debug = Ok(0);
    f.process().unwrap();
    assert_eq!(f.current.record().poses()[0][12].to_bits(), 0);
    assert!(!f.current.offset.full_just_set && !f.current.offset.vertical_just_set);
    assert!(f.current.offset.full_count.is_nan() && f.current.offset.vertical_count.is_nan());
    f.process().unwrap();
    assert_eq!(
        f.current.offset.transform.map(f32::to_bits),
        original_transform
    );
    assert_eq!(f.current.record().poses()[0][12].to_bits(), 0);
    assert!(f.delta.is_finite());
    assert!(f.current.root_input(1.0).is_some());
    assert_eq!(f.current.last_filtered_state(), 1);
    assert_eq!(
        f.current.second_last_com_velocity().to_bits(),
        29.0_f32.to_bits()
    );
}

#[test]
fn offboard_gate_skips_two_cvar_reads_and_landing_on_board_remains_independent() {
    let mut f = Fixture::new();
    f.input.category = PHYSICS_CATEGORY_OFFBOARD;
    f.input.state = PHYSICS_STATE_LANDINGONDECK;
    f.process().unwrap();
    f.assert_events(&["attribute", "dot", "debug", "com"]);
    assert_eq!(f.cvars.offboard_reads, 0);
    f.events.borrow_mut().clear();
    f.input.category = 499;
    f.input.state = PHYSICS_STATE_OFFBOARD_LOCOMOTION;
    f.process().unwrap();
    f.assert_events(&["attribute", "dot", "debug", "com"]);
    assert_eq!(f.cvars.offboard_reads, 0);
    f.events.borrow_mut().clear();
    f.input.flags_8cc = 0x200;
    f.adjust().unwrap();
    f.assert_events(&["attribute", "dot", "debug"]);
}

fn prepare_offboard(f: &mut Fixture) {
    f.input.category = PHYSICS_CATEGORY_OFFBOARD;
    f.input.state = PHYSICS_STATE_OFFBOARD;
    f.input.filtered_state = 6;
    for (bone, position) in [
        (3, [10.0, 8.0, 6.0]),
        (7, [21.0, 18.0, 15.0]),
        (26, [1.0, 2.0, 3.0]),
        (27, [4.0, 5.0, 6.0]),
    ] {
        f.pose[bone] = FRESH_VOLUME_TRANSFORM;
        f.pose[bone][12..15].copy_from_slice(&position);
    }
}

#[test]
fn offboard_mirror_uses_raw_actual_hand_and_raw_target_without_bone_to_volume() {
    for (mirror, expected) in [(0, [17.0_f32, 13.0, 9.0]), (8, [9.0_f32, 6.0, 3.0])] {
        let mut f = Fixture::with_hand_offset([0.0, 101.0, 0.0, 0.0]);
        prepare_offboard(&mut f);
        f.input.flags_8cc = mirror;
        f.process().unwrap();
        f.assert_events(&[
            "offboard full",
            "offboard Y",
            "attribute",
            "dot",
            "debug",
            "com",
        ]);
        assert_eq!(
            f.current.offset.transform[12..15]
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected.map(f32::to_bits)
        );
        assert_eq!(f.current.offset.full_count.to_bits(), 3.0_f32.to_bits());
        assert_eq!(f.current.offset.vertical_count.to_bits(), 5.0_f32.to_bits());
        assert!(!f.current.offset.full_just_set && !f.current.offset.vertical_just_set);
        assert_eq!(
            f.current.record().poses()[0][13].to_bits(),
            expected[1].to_bits()
        );
        // Prepared left target contains the separate authored Part offset.
        assert_eq!(
            f.current.ik().targets()[2][13].to_bits(),
            (2.0 + 101.0 + expected[1]).to_bits()
        );
        assert_eq!(f.current.raw_skateboard()[13].to_bits(), 0);
        assert_eq!(f.current.last_filtered_state(), 6);
    }
}

#[test]
fn offboard_first_and_second_blend_failures_preserve_distinct_source_prefixes() {
    for failed_read in 0..2 {
        let mut f = Fixture::new();
        prepare_offboard(&mut f);
        f.current.offset.full_count = 11.0;
        f.current.offset.vertical_count = 13.0;
        f.current.offset.vertical_just_set = false;
        f.cvars.offboard[failed_read] = Err("offboard blend failed");
        assert!(matches!(
            f.process(),
            Err(SkeletonCurrentAnimationError::Offboard(
                crate::skeleton_offboard_adjust::SkeletonOffboardAdjustError::Cvar(
                    "offboard blend failed"
                )
            ))
        ));
        f.assert_events(if failed_read == 0 {
            &["offboard full"]
        } else {
            &["offboard full", "offboard Y"]
        });
        assert_eq!(f.current.offset.transform[12].to_bits(), 17.0_f32.to_bits());
        assert!(f.current.offset.full_just_set);
        assert_eq!(
            f.current.offset.full_count.to_bits(),
            if failed_read == 0 {
                11.0_f32.to_bits()
            } else {
                3.0_f32.to_bits()
            }
        );
        assert_eq!(
            f.current.offset.vertical_count.to_bits(),
            13.0_f32.to_bits()
        );
        assert!(!f.current.offset.vertical_just_set);
        assert_eq!(f.current.ik().targets()[3][12].to_bits(), 4.0_f32.to_bits());
        assert_eq!(f.current.record().poses()[0][12].to_bits(), 0);
        assert_eq!(f.current.last_filtered_state(), 0);
        assert_eq!(f.current.second_last_com_velocity().to_bits(), 0);
        assert_eq!(f.delta.to_bits(), (-99.0_f32).to_bits());
        assert!(f.current.root_input(1.0).is_none());
    }
}

#[test]
fn offboard_activation_then_ground_decays_full_and_y_independently() {
    let mut f = Fixture::new();
    prepare_offboard(&mut f);
    f.cvars.offboard = [Ok(2.0), Ok(4.0)];
    f.process().unwrap();
    assert_eq!(f.current.last_filtered_state(), 6);
    f.events.borrow_mut().clear();
    f.input.category = PHYSICS_CATEGORY_GROUND;
    f.input.state = PHYSICS_STATE_PHYSICS_GROUND;
    f.input.filtered_state = 1;
    f.process().unwrap();
    f.assert_events(&["attribute", "dot", "debug", "com"]);
    assert_eq!(f.cvars.offboard_reads, 2);
    assert_eq!(f.current.offset.full_count.to_bits(), 1.0_f32.to_bits());
    assert_eq!(f.current.offset.vertical_count.to_bits(), 3.0_f32.to_bits());
    assert_eq!(f.current.offset.transform[12].to_bits(), 4.25_f32.to_bits());
    assert_eq!(
        f.current.offset.transform[13].to_bits(),
        7.3125_f32.to_bits()
    );
    assert_eq!(f.current.offset.transform[14].to_bits(), 2.25_f32.to_bits());
    assert_eq!(
        f.current.record().poses()[0][13].to_bits(),
        7.3125_f32.to_bits()
    );
    assert_eq!(f.current.last_filtered_state(), 1);
}

#[test]
fn offboard_activation_survives_later_landing_attribute_failure() {
    let mut f = Fixture::new();
    prepare_offboard(&mut f);
    f.input.flags_8cc = 0x200;
    f.retained_com_y = -1.0;
    f.attributes.landing_blend = Err("landing attribute");
    assert!(matches!(
        f.process(),
        Err(SkeletonCurrentAnimationError::LandingOnBoard(
            SkeletonLandingOnBoardError::Attribute("landing attribute")
        ))
    ));
    f.assert_events(&["offboard full", "offboard Y", "landing blend"]);
    assert_eq!(f.current.offset.transform[12].to_bits(), 17.0_f32.to_bits());
    assert_eq!(f.current.offset.full_count.to_bits(), 3.0_f32.to_bits());
    assert_eq!(f.current.offset.vertical_count.to_bits(), 5.0_f32.to_bits());
    assert!(f.current.offset.full_just_set && f.current.offset.vertical_just_set);
    assert_eq!(f.current.ik().targets()[3][12].to_bits(), 4.0_f32.to_bits());
    assert_eq!(f.current.record().poses()[0][12].to_bits(), 0);
    assert_eq!(f.current.last_filtered_state(), 0);
}

#[test]
fn inactive_grind_clears_adjusting_but_selected_landing_skip_preserves_it() {
    let mut f = Fixture::new();
    f.current.grind_adjusting = true;
    f.adjust().unwrap();
    assert!(!f.current.grind_adjusting);
    f.events.borrow_mut().clear();
    f.current.grind_activated = true;
    f.current.grind_adjusting = false;
    f.adjust().unwrap();
    assert!(f.current.grind_adjusting);
    assert!(f.current.offset.full_just_set && f.current.offset.vertical_just_set);
    assert_eq!(f.current.offset.full_count.to_bits(), 3.0_f32.to_bits());
    assert_eq!(f.current.offset.vertical_count.to_bits(), 3.0_f32.to_bits());
    assert_eq!(f.events.borrow()[..2], ["spring frames", "spring frames"]);
    f.events.borrow_mut().clear();
    f.current.grind_activated = true;
    f.current.grind_adjusting = true;
    f.input.flags_8cc = 0x4_0200;
    f.adjust().unwrap();
    assert!(f.current.grind_activated && f.current.grind_adjusting);
    f.assert_events(&["attribute", "dot", "debug"]);
}

#[test]
fn spring_consumes_previous_animation_before_publishing_new_record_and_targets() {
    use crate::skeleton_landing_adjust::SkeletonLandingAdjustKind as K;
    for kind in [K::Ground, K::Grind, K::Manual, K::Coffin] {
        let mut f = Fixture::new();
        f.input.filtered_state = 2;
        f.numeric.result = -2.0;
        f.pose[1][13] = 8.0;
        f.process().unwrap();
        let raw_ik = *f.current.ik().targets();
        f.input.filtered_state = if kind == K::Grind { 3 } else { 1 };
        f.input.flags_8cc = if kind == K::Ground { 0x4000_0000 } else { 0 };
        f.input.balance = if kind == K::Manual { 1.0 } else { 0.0 };
        f.input.flags_8c8 = if kind == K::Coffin { 4 } else { 0 };
        for (previous_com, new_com) in [(8.0_f32, 20.0_f32), (20.0, -11.0)] {
            f.pose[1][13] = new_com;
            f.process().unwrap();
            let spring = f.current.landing_adjust();
            assert_eq!(spring.kind(), kind);
            assert!(spring.adjusting());
            assert_eq!(
                spring.last_animation_position().to_bits(),
                previous_com.to_bits()
            );
            let correction = f.current.offset().transform[13];
            assert!(correction > 0.0);
            assert_eq!(
                f.current.record().centre_of_mass()[1].to_bits(),
                new_com.to_bits()
            );
            assert_eq!(
                f.current.record().poses()[0][13].to_bits(),
                correction.to_bits()
            );
            assert_eq!(
                f.targets.completed_volumes().unwrap()[0][13].to_bits(),
                correction.to_bits()
            );
            for (actual, raw) in f.current.ik().targets().iter().zip(raw_ik) {
                assert_eq!(actual[13].to_bits(), (raw[13] + correction).to_bits());
                for lane in [12, 14, 15] {
                    assert_eq!(actual[lane].to_bits(), raw[lane].to_bits());
                }
            }
            assert_eq!(
                f.current.root_input(0.125).unwrap().skate_animation[13].to_bits(),
                correction.to_bits()
            );
            assert!(!f.current.offset().vertical_just_set);
        }
    }
}

#[test]
fn teleport_projection_resets_deck_history_and_ik_only() {
    let mut f = Fixture::new();
    f.pose[0][9] = 0.5;
    f.pose[1][13] = 8.0;
    f.pose[24][12] = 31.0;
    f.process().unwrap();
    let com = f.current.record().centre_of_mass().map(f32::to_bits);
    let hips = f.current.hips().map(f32::to_bits);
    let raw = f.current.raw_skateboard().map(f32::to_bits);
    let ids = *f.current.ik().target_bone_ids();
    f.current.landing.seed_history(
        true,
        f.current.last_filtered_state(),
        f.current.second_last_com_velocity(),
    );
    f.current.offset.full_count = 7.0;
    f.current.offset.full_just_set = true;
    f.current.grind_adjusting = true;
    f.current.reset_teleport_animation_projection();
    assert_eq!(f.current.last_deck_zy().to_bits(), 0);
    let identity = [
        0x3f80_0000,
        0,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0,
        0,
    ];
    for target in f.current.ik().targets() {
        assert_eq!(target.map(f32::to_bits), identity);
    }
    assert_eq!(f.current.ik().target_bone_ids(), &ids);
    assert!(f.current.ik().initialized());
    assert_eq!(f.current.record().centre_of_mass().map(f32::to_bits), com);
    assert_eq!(f.current.record().reset_scalar().to_bits(), 0x3f80_0000);
    assert_eq!(f.current.hips().map(f32::to_bits), hips);
    assert_eq!(f.current.raw_skateboard().map(f32::to_bits), raw);
    assert!(f.current.landing.adjusting() && f.current.grind_adjusting);
    assert_eq!(f.current.last_filtered_state(), 1);
    assert_eq!(
        f.current.second_last_com_velocity().to_bits(),
        29.0_f32.to_bits()
    );
    assert_eq!(f.current.offset.full_count.to_bits(), 7.0_f32.to_bits());
    assert!(f.current.offset.full_just_set);
}

#[path = "../movement/skeleton_process_data_continuation_tests.rs"]
mod live_read;
