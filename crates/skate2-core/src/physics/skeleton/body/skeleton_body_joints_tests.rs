//! Synthetic source graph discriminators, not original tuning or native traces.
#![expect(
    clippy::float_cmp,
    reason = "exact source stores and representable arithmetic"
)]
use super::*;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Quaternion(SkeletonJointQuaternionSite),
    Cache,
    Reciprocal(u32),
    Byte(SkeletonJointByteRoute, u64),
    Float(u64),
    Cosine(u32),
}
type Events = Rc<RefCell<Vec<Event>>>;
struct Reader {
    events: Events,
    emit: VecDeque<u8>,
    enabled: u8,
    free_swing: u8,
    free_twist: u8,
}
impl Reader {
    fn new(events: &Events, emit: &[u8]) -> Self {
        Self {
            events: Rc::clone(events),
            emit: emit.iter().copied().collect(),
            enabled: 1,
            free_swing: 0,
            free_twist: 0,
        }
    }
}
impl SkeletonJointAttributeRead for Reader {
    type Error = &'static str;
    fn cached_prism_velocity(&mut self) -> Result<[f32; 4], Self::Error> {
        self.events.borrow_mut().push(Event::Cache);
        Ok([1.0, -2.0, 0.0, 4.0])
    }
    fn byte(&mut self, route: SkeletonJointByteRoute, key: u64) -> Result<u8, Self::Error> {
        self.events.borrow_mut().push(Event::Byte(route, key));
        match (route, key) {
            (SkeletonJointByteRoute::GlobalAnimation, ENABLE) => Ok(self
                .emit
                .pop_front()
                .expect("one fresh emission sample per eligible Part")),
            (SkeletonJointByteRoute::LocalJoints, ENABLE) => Ok(self.enabled),
            (SkeletonJointByteRoute::LocalJoints, FREE_SWING) => Ok(self.free_swing),
            (SkeletonJointByteRoute::LocalJoints, FREE_TWIST) => Ok(self.free_twist),
            _ => Err("wrong byte route"),
        }
    }
    fn skeleton_float(&mut self, key: u64) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push(Event::Float(key));
        match key {
            SWING_VELOCITY => Ok(2.0),
            TWIST_VELOCITY => Ok(3.0),
            _ => Err("wrong float route"),
        }
    }
}
struct Numeric {
    events: Events,
    reciprocal_seed: f32,
    linear_seed: Option<[f32; 4]>,
    fail_site: Option<SkeletonJointQuaternionSite>,
}
impl Numeric {
    fn new(events: &Events) -> Self {
        Self {
            events: Rc::clone(events),
            reciprocal_seed: 30.0,
            linear_seed: None,
            fail_site: None,
        }
    }
}
impl SkeletonJointNumeric for Numeric {
    type Error = &'static str;
    fn quaternion_estimate(
        &mut self,
        site: SkeletonJointQuaternionSite,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        self.events.borrow_mut().push(Event::Quaternion(site));
        if self.fail_site == Some(site) {
            return Err("estimate failure");
        }
        if site == SkeletonJointQuaternionSite::ParentLinear {
            if let Some(seed) = self.linear_seed {
                return Ok(seed);
            }
        }
        Ok(magnitudes.map(|value| value.sqrt().recip()))
    }
    fn timestep_reciprocal_estimate(&mut self, timestep: f32) -> Result<f32, Self::Error> {
        self.events
            .borrow_mut()
            .push(Event::Reciprocal(timestep.to_bits()));
        Ok(self.reciprocal_seed)
    }
    fn cosine(&mut self, angle: f32) -> Result<f32, Self::Error> {
        self.events
            .borrow_mut()
            .push(Event::Cosine(angle.to_bits()));
        Ok(angle + 100.0)
    }
}

fn fixture(children: &[usize]) -> (SkeletonData, SkeletonBodyAttributeData) {
    let (mut hierarchy, mut parameters, pose) = crate::skeleton_data_tests::fixture();
    hierarchy.parents.fill(-1);
    for child in children {
        hierarchy.parents[*child] = 23;
    }
    for bone in &mut parameters.bones {
        bone.swing = 2.0;
        bone.twist = 3.0;
    }
    let data = crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap();
    let (_, mut attributes) = crate::skeleton_body_definition::tests::inputs();
    for (index, row) in attributes.joints.iter_mut().enumerate() {
        row.swing_angle = f32::from(u8::try_from(index + 1).unwrap()) * 0.25;
        row.twist_angle = f32::from(u8::try_from(index + 1).unwrap()) * 0.5;
    }
    (data, attributes)
}

#[test]
fn source_order_runs_all_services_even_for_unemitted_joint_and_reuses_attribute_row() {
    let (data, attributes) = fixture(&[1, 2]);
    let events = Events::default();
    let mut reader = Reader::new(&events, &[0, 0x80]);
    let mut numeric = Numeric::new(&events);
    let output =
        SkeletonBodyJoints::try_create(&data, &attributes, 22, &mut reader, &mut numeric).unwrap();
    assert_eq!(output.allocated_count, 22);
    assert_eq!(output.joints.len(), 1); // No fabricated initialized21-record tail.
    assert_eq!(output.emitted_for_part[1], None);
    assert_eq!(output.emitted_for_part[2], Some(0));
    let joint = &output.joints[0];
    assert_eq!((joint.parent_part, joint.child_part), (23, 2));
    assert_eq!(joint.limits.swing_a, 0.5); // Row0, not row1 from skipped child1.
    assert_eq!(joint.limits.twist_a, 1.5);
    assert_eq!(joint.limits.swing_c, 100.5);
    assert_eq!(joint.limits.twist_c, 101.5);
    assert_eq!(joint.limits.v_swing, 2.0 * FREQUENCY);
    assert_eq!(joint.limits.v_twist, 3.0 * FREQUENCY);
    let mut reciprocal = 30.0_f32;
    for _ in 0..2 {
        reciprocal = reciprocal.mul_add((-reciprocal).mul_add(TIMESTEP, 1.0), reciprocal);
    }
    assert_eq!(
        joint.limits.v_prism,
        [reciprocal, reciprocal * -2.0, 0.0, reciprocal * 4.0]
    );
    let one = vec![
        Event::Quaternion(SkeletonJointQuaternionSite::ParentAngular),
        Event::Quaternion(SkeletonJointQuaternionSite::ChildAngular),
        Event::Quaternion(SkeletonJointQuaternionSite::ParentLinear),
        Event::Cache,
        Event::Reciprocal(0x3c88_8889),
        Event::Byte(SkeletonJointByteRoute::LocalJoints, ENABLE),
        Event::Cosine(0.5_f32.to_bits()),
        Event::Cosine(1.5_f32.to_bits()),
        Event::Byte(SkeletonJointByteRoute::LocalJoints, FREE_SWING),
        Event::Byte(SkeletonJointByteRoute::LocalJoints, FREE_TWIST),
        Event::Float(SWING_VELOCITY),
        Event::Float(TWIST_VELOCITY),
        Event::Byte(SkeletonJointByteRoute::GlobalAnimation, ENABLE),
    ];
    assert_eq!(*events.borrow(), [one.clone(), one].concat());
}

#[test]
fn zero_allocation_does_not_skip_later_reads_but_refuses_first_actual_write() {
    for emitted in [0, 1] {
        let (data, attributes) = fixture(&[1]);
        let events = Events::default();
        let result = SkeletonBodyJoints::try_create(
            &data,
            &attributes,
            0,
            &mut Reader::new(&events, &[emitted]),
            &mut Numeric::new(&events),
        );
        assert_eq!(
            events.borrow().last(),
            Some(&Event::Byte(
                SkeletonJointByteRoute::GlobalAnimation,
                ENABLE
            ))
        );
        if emitted == 0 {
            assert!(result.unwrap().joints.is_empty());
        } else {
            assert!(matches!(
                result,
                Err(SkeletonJointError::Capacity {
                    child: SkeletonPartIndex::Neck1,
                    allocated: 0
                })
            ));
        }
    }
}

#[test]
fn disabled_or_ball_defaults_skip_cosines_but_overrides_do_not_erase_computed_limits() {
    for (enabled, ball) in [(0, 0), (1, 0x80)] {
        let (data, mut attributes) = fixture(&[1]);
        attributes.joints[0].ball_joint = ball;
        let events = Events::default();
        let mut reader = Reader::new(&events, &[1]);
        reader.enabled = enabled;
        let result = SkeletonBodyJoints::try_create(
            &data,
            &attributes,
            22,
            &mut reader,
            &mut Numeric::new(&events),
        )
        .unwrap();
        let limits = result.joints[0].limits;
        assert_eq!((limits.raw_swing_type, limits.raw_twist_type), (4, 2));
        assert_eq!(
            [
                limits.swing_a,
                limits.twist_a,
                limits.swing_c,
                limits.twist_c
            ],
            [0.0, 0.0, 1.0, 1.0]
        );
        assert!(
            !events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::Cosine(_)))
        );
    }
    let (data, attributes) = fixture(&[1]);
    let events = Events::default();
    let mut reader = Reader::new(&events, &[1]);
    reader.free_swing = 0x80;
    reader.free_twist = 0xff;
    let result = SkeletonBodyJoints::try_create(
        &data,
        &attributes,
        22,
        &mut reader,
        &mut Numeric::new(&events),
    )
    .unwrap();
    let limits = result.joints[0].limits;
    assert_eq!((limits.raw_swing_type, limits.raw_twist_type), (4, 2));
    assert_eq!(
        [
            limits.swing_a,
            limits.twist_a,
            limits.swing_c,
            limits.twist_c
        ],
        [0.5, 1.5, 100.5, 101.5]
    );
}

#[test]
fn angle_floor_selects_original_product_or_epsilon_including_unordered() {
    assert_eq!(minimum_angle(0.5), 0.5);
    assert_eq!(minimum_angle(ANGLE_EPSILON), ANGLE_EPSILON);
    for value in [0.0, -0.0, -2.0, f32::NAN] {
        assert_eq!(minimum_angle(value).to_bits(), 0x3c23_d70a);
    }
    assert_eq!(minimum_angle(f32::INFINITY), f32::INFINITY);
}

#[test]
fn frame_positions_follow_authored_volume_offsets_and_relative_pose() {
    let (_, mut parameters, _) = crate::skeleton_data_tests::fixture();
    parameters.bones[1].position = [1.0, 2.0, 3.0, 123.0];
    parameters.bones[23].position = [0.5, 1.0, 1.5, 456.0];
    let mut child_pose = FRESH_VOLUME_TRANSFORM;
    child_pose[12..16].copy_from_slice(&[10.0, 20.0, 30.0, 789.0]);
    let mut parent_pose = FRESH_VOLUME_TRANSFORM;
    parent_pose[12..16].copy_from_slice(&[4.0, 6.0, 8.0, -99.0]);
    let result = try_frames(
        &parameters.bones[1],
        &parameters.bones[23],
        &child_pose,
        &parent_pose,
        &mut HostSkeletonJointNumeric,
    )
    .unwrap();
    assert_eq!(result.position_a, [-1.0, -2.0, -3.0, 0.0]);
    assert_eq!(result.position_b, [5.5, 13.0, 20.5, 0.0]);
    assert_eq!(result.quaternion_a, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(result.quaternion_b, [0.0, 0.0, 0.0, 1.0]);
    child_pose[0] = -1.0;
    child_pose[5] = -1.0;
    let rotated = try_frames(
        &parameters.bones[1],
        &parameters.bones[23],
        &child_pose,
        &parent_pose,
        &mut HostSkeletonJointNumeric,
    )
    .unwrap();
    assert_eq!(
        rotated, result,
        "relative pose rotation must not replace authored parent orientation"
    );
}

#[test]
fn parent_linear_identity_still_runs_two_refinements_and_failure_stops_at_actual_service() {
    let (data, attributes) = fixture(&[1]);
    let events = Events::default();
    let mut numeric = Numeric::new(&events);
    numeric.linear_seed = Some([0.25; 4]);
    let result = SkeletonBodyJoints::try_create(
        &data,
        &attributes,
        22,
        &mut Reader::new(&events, &[1]),
        &mut numeric,
    )
    .unwrap();
    // Magnitude4, initialestimate.25 -> .34375 -> .43438720703125.
    assert_eq!(result.joints[0].frames.quaternion_l[3], 0.868_774_4);
    for (site, calls) in [
        (SkeletonJointQuaternionSite::ParentAngular, 1),
        (SkeletonJointQuaternionSite::ChildAngular, 2),
        (SkeletonJointQuaternionSite::ParentLinear, 3),
    ] {
        let events = Events::default();
        let mut numeric = Numeric::new(&events);
        numeric.fail_site = Some(site);
        let result = SkeletonBodyJoints::try_create(
            &data,
            &attributes,
            22,
            &mut Reader::new(&events, &[]),
            &mut numeric,
        );
        assert!(matches!(
            result,
            Err(SkeletonJointError::Numeric("estimate failure"))
        ));
        assert_eq!(events.borrow().len(), calls);
    }
}

#[test]
fn explicit_zero_point_fmas_preserve_nonfinite_rotation_effects() {
    let mut frame = FRESH_VOLUME_TRANSFORM;
    frame[0] = f32::INFINITY;
    frame[12] = 7.0;
    assert!(
        transform_zero(&frame)[0].is_nan(),
        "returning translation directly silently removes infinity*zero"
    );
}

fn assert_quaternion_near(actual: [f32; 4], expected: [f32; 4]) {
    // Authored sqrt2 expansion and host candidate estimates introduce small
    // rounding. Accept either global quaternion sign, never independent signs.
    let same = actual
        .iter()
        .zip(expected)
        .all(|(a, b)| (*a - b).abs() < 0.000_01);
    let opposite = actual
        .iter()
        .zip(expected)
        .all(|(a, b)| (*a + b).abs() < 0.000_01);
    assert!(
        same || opposite,
        "actual {actual:?}, independent axis-rotation result {expected:?}"
    );
}

#[test]
fn authored_noncommuting_rotations_select_parent_and_child_products_not_pose_rotation() {
    struct Capture(Vec<(SkeletonJointQuaternionSite, [f32; 4])>);
    impl SkeletonJointNumeric for Capture {
        type Error = std::convert::Infallible;
        fn quaternion_estimate(
            &mut self,
            site: SkeletonJointQuaternionSite,
            magnitudes: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.0.push((site, magnitudes));
            HostSkeletonJointNumeric.quaternion_estimate(site, magnitudes)
        }
        fn timestep_reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("frame-only test")
        }
        fn cosine(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("frame-only test")
        }
    }
    let (_, mut parameters, _) = crate::skeleton_data_tests::fixture();
    let s = std::f32::consts::FRAC_1_SQRT_2;
    parameters.bones[23].orientation = [s, 0.0, 0.0, s]; // X+90
    parameters.bones[1].parent_orientation = [0.0, s, 0.0, s]; // Y+90
    parameters.bones[1].joint_orientation = [s, 0.0, 0.0, s]; // X+90
    parameters.bones[1].orientation = [0.0, 0.0, s, s]; // Z+90
    let mut numeric = Capture(Vec::new());
    let result = try_frames(
        &parameters.bones[1],
        &parameters.bones[23],
        &FRESH_VOLUME_TRANSFORM,
        &FRESH_VOLUME_TRANSFORM,
        &mut numeric,
    )
    .unwrap();
    // Independently multiply ideal quarter-turn rotations:
    // B = X-90 * Y-90 * X+90 = Z+90;
    // A = Z-90 * X+90 = quaternion [.5,-.5,-.5,.5].
    assert_quaternion_near(result.quaternion_b, [0.0, 0.0, s, s]);
    assert_quaternion_near(result.quaternion_a, [0.5, -0.5, -0.5, 0.5]);
    assert_eq!(numeric.0.len(), 3);
    for ((site, values), (expected_site, expected)) in numeric.0.iter().zip([
        (
            SkeletonJointQuaternionSite::ParentAngular,
            [2.0, 0.0, 0.0, 2.0],
        ),
        (SkeletonJointQuaternionSite::ChildAngular, [1.0; 4]),
        (
            SkeletonJointQuaternionSite::ParentLinear,
            [4.0, 0.0, 0.0, 0.0],
        ),
    ]) {
        assert_eq!(*site, expected_site);
        for (actual, expected) in values.iter().zip(expected) {
            assert!((*actual - expected).abs() < 0.000_01);
        }
    }
    // Deliberately different noncommuting live pose rotations are irrelevant
    // to angular frames; they supply relative POSITION only.
    let child_pose = [
        0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let parent_pose = [
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let posed = try_frames(
        &parameters.bones[1],
        &parameters.bones[23],
        &child_pose,
        &parent_pose,
        &mut HostSkeletonJointNumeric,
    )
    .unwrap();
    assert_eq!(
        posed.quaternion_a.map(f32::to_bits),
        result.quaternion_a.map(f32::to_bits)
    );
    assert_eq!(
        posed.quaternion_b.map(f32::to_bits),
        result.quaternion_b.map(f32::to_bits)
    );
}

#[test]
fn duplicate_authored_parent_uses_last_reverse_mapping_not_first_matching_part() {
    let (mut hierarchy, mut parameters, pose) = crate::skeleton_data_tests::fixture();
    hierarchy.parents.fill(-1);
    hierarchy.parents[1] = 20;
    parameters.bones[23].name = parameters.bones[20].name.clone();
    parameters.bones[23].packed_name = parameters.bones[20].packed_name;
    parameters.bones[20].position = [11.0, 0.0, 0.0, 0.0];
    parameters.bones[23].position = [7.0, 0.0, 0.0, 0.0];
    let data = crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap();
    assert_eq!(data.bone_ids()[20], 20);
    assert_eq!(data.bone_ids()[23], 20);
    assert_eq!(data.part_from_bone()[20], Some(SkeletonPartIndex::Hips));
    assert_eq!(data.parents()[1], Some(SkeletonPartIndex::Hips));
    let (_, attributes) = fixture(&[]);
    let events = Events::default();
    let output = SkeletonBodyJoints::try_create(
        &data,
        &attributes,
        22,
        &mut Reader::new(&events, &[1]),
        &mut Numeric::new(&events),
    )
    .unwrap();
    assert_eq!(output.joints.len(), 1);
    assert_eq!(
        (output.joints[0].parent_part, output.joints[0].child_part),
        (23, 1)
    );
    assert_eq!(output.joints[0].frames.position_b, [-7.0, 0.0, 0.0, 0.0]);
    assert_eq!(output.emitted_for_part[1], Some(0));
    // A producer-valid SkeletonData already records this last reverse mapping
    // in parents(). Do not fabricate an inconsistent private parent table just
    // to distinguish the emission's repeated reverse lookup from that input.
}
