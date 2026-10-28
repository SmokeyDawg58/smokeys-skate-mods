//! Synthetic callback and numeric discriminators, not original tuning inputs.
#![expect(
    clippy::float_cmp,
    reason = "exact source-field and copied-value assertions; signed zero and fusion use explicit word comparisons"
)]
use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Lookup(Class, u64),
    Load(usize),
    SinCos(u32),
    Cos(u32),
    Quaternion,
    Reciprocal(u32),
}

struct Attributes {
    events: Rc<RefCell<Vec<Event>>>,
    reads: Vec<(Class, u64, f32)>,
    next: usize,
    mutate_length_on_offset_load: bool,
    fail_lookup: Option<usize>,
}

impl SkateboardAttributeRead for Attributes {
    type Pointer = usize;
    type Error = &'static str;
    fn lookup(&mut self, class: Class, key: u64) -> Result<usize, Self::Error> {
        self.events.borrow_mut().push(Event::Lookup(class, key));
        if self.fail_lookup == Some(self.next) {
            return Err("lookup refused");
        }
        assert_eq!(
            (class, key),
            (self.reads[self.next].0, self.reads[self.next].1)
        );
        let result = self.next;
        self.next += 1;
        Ok(result)
    }
    fn float(&mut self, pointer: &usize) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push(Event::Load(*pointer));
        if self.mutate_length_on_offset_load && self.reads[*pointer].1 == POSITIVE_OFFSET {
            let length = self
                .reads
                .iter_mut()
                .find(|read| read.1 == DECK_LENGTH)
                .unwrap();
            length.2 = 6.0;
        }
        Ok(self.reads[*pointer].2)
    }
    fn byte(&mut self, _: &usize) -> Result<u8, Self::Error> {
        panic!("CreateJoints has no enable read")
    }
    fn integer(&mut self, _: &usize) -> Result<i32, Self::Error> {
        panic!("no integer attributes")
    }
}

struct Numeric {
    events: Rc<RefCell<Vec<Event>>>,
    quaternion_calls: usize,
    reciprocal_calls: usize,
    varying_quaternion_seeds: bool,
    reciprocal_seeds: Option<[f32; 2]>,
    fail_quaternion: Option<usize>,
}

impl SkateboardJointNumeric for Numeric {
    type Error = &'static str;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        self.events
            .borrow_mut()
            .push(Event::SinCos(angle.to_bits()));
        Ok(angle.sin_cos())
    }
    fn cosine(&mut self, angle: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push(Event::Cos(angle.to_bits()));
        Ok(angle.cos())
    }
    fn quaternion_reciprocal_square_root_estimate(
        &mut self,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        self.events.borrow_mut().push(Event::Quaternion);
        let call = self.quaternion_calls;
        self.quaternion_calls += 1;
        if self.fail_quaternion == Some(call) {
            return Err("quaternion refused");
        }
        if self.varying_quaternion_seeds {
            let ordinal = u8::try_from(call).unwrap();
            Ok([0.1 + f32::from(ordinal) * 0.01; 4])
        } else {
            Ok(magnitudes.map(|value| value.sqrt().recip()))
        }
    }
    fn timestep_reciprocal_estimate(&mut self, timestep: f32) -> Result<f32, Self::Error> {
        self.events
            .borrow_mut()
            .push(Event::Reciprocal(timestep.to_bits()));
        let call = self.reciprocal_calls;
        self.reciprocal_calls += 1;
        Ok(self
            .reciprocal_seeds
            .map_or_else(|| timestep.recip(), |seeds| seeds[call]))
    }
}

fn fixture(reads: Vec<(Class, u64, f32)>) -> (Attributes, Numeric) {
    let events = Rc::new(RefCell::new(Vec::new()));
    (
        Attributes {
            events: Rc::clone(&events),
            reads,
            next: 0,
            mutate_length_on_offset_load: false,
            fail_lookup: None,
        },
        Numeric {
            events,
            quaternion_calls: 0,
            reciprocal_calls: 0,
            varying_quaternion_seeds: false,
            reciprocal_seeds: None,
            fail_quaternion: None,
        },
    )
}

fn joint_reads() -> Vec<(Class, u64, f32)> {
    vec![
        (Class::Trucks, ANGLE, 10.0),
        (Class::Deck, DECK_LENGTH, 2.0),
        (Class::Trucks, POSITIVE_OFFSET, 2.0),
        (Class::Trucks, VERTICAL, -0.25),
        (Class::Trucks, ANGLE, 20.0),
        (Class::Trucks, NEGATIVE_OFFSET, 3.0),
        (Class::Deck, DECK_LENGTH, 4.0),
        (Class::Trucks, VERTICAL, -0.5),
        (Class::Trucks, TWIST_ARC, 30.0),
        (Class::Trucks, TWIST_VELOCITY, 40.0),
        (Class::Wheels, WHEEL_HALF_WIDTH, 0.25),
        (Class::Wheels, WHEEL_HALF_WIDTH, 0.5),
        (Class::Wheels, AXLE_VELOCITY, 100.0),
    ]
}

fn transform_reads() -> Vec<(Class, u64, f32)> {
    vec![
        (Class::Deck, DECK_LENGTH, 2.0),
        (Class::Trucks, POSITIVE_OFFSET, 2.0),
        (Class::Trucks, NEGATIVE_OFFSET, 3.0),
        (Class::Deck, DECK_LENGTH, 4.0),
        (Class::Trucks, ANGLE, 10.0),
        (Class::Trucks, VERTICAL, -0.25),
        (Class::Trucks, ANGLE, 20.0),
        (Class::Trucks, VERTICAL, -0.5),
    ]
}

#[test]
fn thirteen_lookups_and_numeric_callbacks_follow_one_native_sequence() {
    use Event::{Cos, Load as F, Lookup as L, Quaternion as Q, Reciprocal as R, SinCos as S};
    let (mut attributes, mut numeric) = fixture(joint_reads());
    let joints = try_create_skateboard_joints(&mut attributes, &mut numeric).unwrap();
    assert_eq!(attributes.next, 13);
    assert_eq!(
        *attributes.events.borrow(),
        vec![
            S(flip_sign(HALF_PI).to_bits()),
            L(Class::Trucks, ANGLE),
            F(0),
            S(flip_sign(10.0 * DTR).to_bits()),
            L(Class::Deck, DECK_LENGTH),
            L(Class::Trucks, POSITIVE_OFFSET),
            F(2),
            F(1),
            Q,
            Q,
            Q,
            L(Class::Trucks, VERTICAL),
            F(3),
            S(HALF_PI.to_bits()),
            L(Class::Trucks, ANGLE),
            F(4),
            S((20.0 * DTR).to_bits()),
            L(Class::Trucks, NEGATIVE_OFFSET),
            L(Class::Deck, DECK_LENGTH),
            F(5),
            F(6),
            Q,
            Q,
            Q,
            L(Class::Trucks, VERTICAL),
            F(7),
            R(TIMESTEP.to_bits()),
            L(Class::Trucks, TWIST_ARC),
            F(8),
            Cos((30.0 * DTR).to_bits()),
            L(Class::Trucks, TWIST_VELOCITY),
            F(9),
            S(HALF_PI.to_bits()),
            Q,
            Q,
            Q,
            L(Class::Wheels, WHEEL_HALF_WIDTH),
            F(10),
            Q,
            Q,
            Q,
            L(Class::Wheels, WHEEL_HALF_WIDTH),
            F(11),
            R(TIMESTEP.to_bits()),
            L(Class::Wheels, AXLE_VELOCITY),
            F(12),
        ]
    );
    assert_eq!(
        joints.map(|joint| (joint.parent_part, joint.child_part)),
        [(6, 4), (6, 5), (4, 0), (4, 1), (5, 2), (5, 3)]
    );
}

#[test]
fn six_distinct_payload_owners_keep_copied_values_and_independent_setter_results() {
    let (mut attributes, mut numeric) = fixture(joint_reads());
    numeric.varying_quaternion_seeds = true;
    let joints = try_create_skateboard_joints(&mut attributes, &mut numeric).unwrap();
    assert_eq!(numeric.quaternion_calls, 12);
    for (index, joint) in joints.iter().enumerate() {
        for other in &joints[index + 1..] {
            assert!(!Arc::ptr_eq(&joint.frames, &other.frames));
            assert!(!Arc::ptr_eq(&joint.limits, &other.limits));
        }
        assert_eq!(joint.frames.position_a.map(f32::to_bits), [0; 4]);
    }
    assert_eq!(joints[2].frames, joints[4].frames);
    assert_eq!(joints[3].frames, joints[5].frames);
    assert_ne!(joints[0].frames.quaternion_a, joints[1].frames.quaternion_a);
    assert_ne!(joints[2].frames.quaternion_a, joints[2].frames.quaternion_b);
    assert_ne!(joints[2].frames.quaternion_b, joints[2].frames.quaternion_l);
    // Identity conversion remains an unnormalized candidate with this explicit
    // non-Xenon seed. A final normalization would produce W=1 instead.
    assert!(joints[0].frames.quaternion_a[3] < 0.5);
    assert_eq!(joints[2].frames.position_b, [0.0, 0.0, 0.25, 0.0]);
    assert_eq!(joints[3].frames.position_b, [0.0, 0.0, -0.5, 0.0]);
}

#[test]
fn truck_and_axle_limits_preserve_native_fields_and_separate_products() {
    let (mut attributes, mut numeric) = fixture(joint_reads());
    let joints = try_create_skateboard_joints(&mut attributes, &mut numeric).unwrap();
    let expected_truck = SimulationJointLimits {
        p_prism: [0.0; 4],
        v_prism: [0.0; 4],
        v_twist: (40.0 * DTR) * ANGULAR_FREQUENCY,
        v_swing: 0.0,
        swing_a: 0.0,
        twist_a: 30.0 * DTR,
        swing_c: 1.0,
        twist_c: (30.0 * DTR).cos(),
        raw_swing_type: 0,
        raw_twist_type: 1,
    };
    assert_eq!(*joints[0].limits, expected_truck);
    assert_eq!(*joints[1].limits, expected_truck);
    let expected_axle = SimulationJointLimits {
        p_prism: [0.0; 4],
        v_prism: [0.0; 4],
        v_twist: 0.0,
        v_swing: (100.0 * DTR) * ANGULAR_FREQUENCY,
        swing_a: 0.0,
        twist_a: 0.0,
        swing_c: 1.0,
        twist_c: 1.0,
        // Independent raw instruction evidence: li r10,3 at82b2de84,
        // then four stores to each wheel JointLimits+38.
        raw_swing_type: 3,
        raw_twist_type: 0,
    };
    for joint in &joints[2..] {
        assert_eq!(*joint.limits, expected_axle);
    }
}

#[test]
fn level_transforms_have_their_own_eight_lookup_and_load_chronology() {
    use Event::{Load as F, Lookup as L, SinCos as S};
    let (mut attributes, mut numeric) = fixture(transform_reads());
    let transforms = try_calc_truck_transforms(&mut attributes, &mut numeric).unwrap();
    assert_eq!(
        *attributes.events.borrow(),
        vec![
            L(Class::Deck, DECK_LENGTH),
            L(Class::Trucks, POSITIVE_OFFSET),
            F(0),
            F(1),
            L(Class::Trucks, NEGATIVE_OFFSET),
            L(Class::Deck, DECK_LENGTH),
            F(2),
            F(3),
            S(flip_sign(HALF_PI).to_bits()),
            L(Class::Trucks, ANGLE),
            F(4),
            S(flip_sign(10.0 * DTR).to_bits()),
            L(Class::Trucks, VERTICAL),
            F(5),
            S(HALF_PI.to_bits()),
            L(Class::Trucks, ANGLE),
            F(6),
            S((20.0 * DTR).to_bits()),
            L(Class::Trucks, VERTICAL),
            F(7),
        ]
    );
    assert_eq!(&transforms.back[12..], &[0.0, -0.25, 3.0, 0.0]);
    assert_eq!(&transforms.front[12..], &[0.0, -0.5, -5.0, 0.0]);
    assert_eq!(numeric.quaternion_calls, 0);
    assert_eq!(numeric.reciprocal_calls, 0);
}

#[test]
fn delayed_length_loads_are_not_folded_into_one_snapshot() {
    let (mut joint_attributes, mut joint_numeric) = fixture(joint_reads());
    joint_attributes.mutate_length_on_offset_load = true;
    let joints = try_create_skateboard_joints(&mut joint_attributes, &mut joint_numeric).unwrap();
    let (mut transform_attributes, mut transform_numeric) = fixture(transform_reads());
    transform_attributes.mutate_length_on_offset_load = true;
    let transforms =
        try_calc_truck_transforms(&mut transform_attributes, &mut transform_numeric).unwrap();
    assert_eq!(joints[0].frames.position_b[2], 5.0);
    assert_eq!(transforms.back[14], 3.0);
}

#[test]
fn both_offset_paths_retain_fma_subnormal_rounding_and_outer_rear_negation() {
    for transform_path in [false, true] {
        let mut reads = if transform_path {
            transform_reads()
        } else {
            joint_reads()
        };
        for (_, key, value) in &mut reads {
            if [DECK_LENGTH, POSITIVE_OFFSET, NEGATIVE_OFFSET].contains(key) {
                *value = f32::from_bits(1);
            }
        }
        let (mut attributes, mut numeric) = fixture(reads);
        let (positive, negative) = if transform_path {
            let result = try_calc_truck_transforms(&mut attributes, &mut numeric).unwrap();
            (result.back[14], result.front[14])
        } else {
            let result = try_create_skateboard_joints(&mut attributes, &mut numeric).unwrap();
            (
                result[0].frames.position_b[2],
                result[1].frames.position_b[2],
            )
        };
        assert_eq!(positive.to_bits(), 2);
        assert_eq!(negative.to_bits(), 0x8000_0002);
    }
    assert_eq!((f32::from_bits(1) * 0.5 + f32::from_bits(1)).to_bits(), 1);
}

#[test]
fn intrinsic_matrix_columns_keep_raw_w_and_xyz_fused_composition() {
    let y = rotation_y((2.0, 3.0));
    let x = rotation_x((5.0, 7.0));
    assert_eq!(
        y,
        [
            [3.0, 0.0, -2.0, 3.0],
            [0.0, 1.0, 0.0, 0.0],
            [2.0, 0.0, 3.0, 2.0]
        ]
    );
    assert_eq!(
        x,
        [
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 7.0, 5.0, 0.0],
            [0.0, -5.0, 7.0, 0.0]
        ]
    );
    assert_eq!(
        compose_x_y(x, y),
        [
            [3.0, 10.0, -14.0, 3.0],
            [0.0, 7.0, 5.0, 0.0],
            [2.0, -15.0, 21.0, 2.0]
        ]
    );
}

#[test]
fn exceptional_inputs_are_not_clamped_and_reciprocal_groups_are_not_coalesced() {
    let mut reads = joint_reads();
    reads[8].2 = f32::NAN;
    reads[9].2 = f32::NEG_INFINITY;
    reads[10].2 = -0.0;
    reads[11].2 = 0.0;
    reads[12].2 = f32::INFINITY;
    let (mut attributes, mut numeric) = fixture(reads);
    numeric.reciprocal_seeds = Some([f32::NAN, 0.0]);
    let joints = try_create_skateboard_joints(&mut attributes, &mut numeric).unwrap();
    assert_eq!(numeric.reciprocal_calls, 2);
    assert!(joints[0].limits.v_prism.iter().all(|value| value.is_nan()));
    assert_eq!(joints[2].limits.v_prism.map(f32::to_bits), [0; 4]);
    assert!(joints[0].limits.twist_a.is_nan());
    assert!(joints[0].limits.twist_c.is_nan());
    assert_eq!(joints[0].limits.v_twist, f32::NEG_INFINITY);
    assert_eq!(joints[2].limits.v_swing, f32::INFINITY);
    assert_eq!(joints[2].frames.position_b[2].to_bits(), 0x8000_0000);
    assert_eq!(joints[3].frames.position_b[2].to_bits(), 0x8000_0000);
}

#[test]
fn unsupported_attributes_and_numeric_failures_stop_before_later_callbacks() {
    let (mut attributes, mut numeric) = fixture(joint_reads());
    attributes.fail_lookup = Some(3);
    assert_eq!(
        try_create_skateboard_joints(&mut attributes, &mut numeric).unwrap_err(),
        SkateboardJointBuildError::Attribute("lookup refused")
    );
    assert_eq!(numeric.quaternion_calls, 3);
    assert_eq!(numeric.reciprocal_calls, 0);
    let (mut attributes, mut numeric) = fixture(joint_reads());
    numeric.fail_quaternion = Some(1);
    assert_eq!(
        try_create_skateboard_joints(&mut attributes, &mut numeric).unwrap_err(),
        SkateboardJointBuildError::Numeric("quaternion refused")
    );
    assert_eq!(attributes.next, 3);
    assert_eq!(numeric.quaternion_calls, 2);
    assert_eq!(attributes.events.borrow().last(), Some(&Event::Quaternion));
}

#[test]
fn host_provider_exposes_its_ordinary_host_estimates_explicitly() {
    let mut numeric = HostSkateboardJointNumeric;
    assert_eq!(numeric.sine_cosine(0.0).unwrap(), (0.0, 1.0));
    assert_eq!(numeric.cosine(0.0).unwrap(), 1.0);
    assert_eq!(
        numeric
            .quaternion_reciprocal_square_root_estimate([1.0, 4.0, 16.0, 64.0])
            .unwrap(),
        [1.0, 0.5, 0.25, 0.125]
    );
    assert_eq!(
        numeric.timestep_reciprocal_estimate(TIMESTEP).unwrap(),
        TIMESTEP.recip()
    );
}
