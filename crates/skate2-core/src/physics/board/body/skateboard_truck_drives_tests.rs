//! Synthetic callback traces and failure injection; no reference runtime trace.
#![expect(
    clippy::float_cmp,
    reason = "exact copied/source fields; exceptional values and fused discriminators compare explicit bits"
)]
use super::*;
use std::{cell::RefCell, rc::Rc};

#[test]
fn contact_turn_preserves_histories_unordered_timers_and_zero_signs() {
    let mut owner = SkateboardTruckDrives::new(levels());
    owner.turn_front = 2.0;
    owner.turn_back = -3.0;
    owner.time_front = 0.1;
    owner.time_back = -0.0;
    owner.set_force_turn_angle(0.0, false, false);
    assert!(owner.force_turn);
    assert_eq!(owner.turn_front.to_bits(), 2.0_f32.to_bits());
    assert_eq!(
        owner.turn_back.to_bits(),
        (-3.0 * f32::from_bits(0x3f7b_a5e3)).to_bits()
    );
    assert_eq!(owner.time_front.to_bits(), 0);
    assert_eq!(owner.time_back.to_bits(), 0);
    owner.time_front = f32::NAN;
    owner.turn_front = f32::INFINITY;
    owner.time_back = f32::INFINITY;
    owner.set_force_turn_angle(-0.0, true, true);
    assert!(owner.turn_front.is_nan());
    assert_eq!(owner.turn_back.to_bits(), 0x8000_0000);
    assert!(owner.time_front.is_nan());
    assert_eq!(owner.time_back.to_bits(), f32::INFINITY.to_bits());
    owner.time_back = f32::NAN;
    owner.turn_back = f32::NEG_INFINITY;
    owner.set_force_turn_angle(2.0, false, true);
    assert!(owner.turn_back.is_nan());
    assert!(owner.time_back.is_nan());
    owner.time_front = f32::NAN;
    owner.turn_front = 1.0;
    owner.set_force_turn_angle(-0.0, true, false);
    // Blend=1 executes +0 + -0, producing +0 rather than copying -0.
    assert_eq!(owner.turn_front.to_bits(), 0);
    owner.time_front = -1.0;
    owner.turn_front = 5.0;
    owner.set_force_turn_angle(9.0, true, false);
    assert_eq!(owner.turn_front.to_bits(), 5.0_f32.to_bits());
    owner.time_front = 0.0;
    owner.turn_front = 0.0;
    owner.set_force_turn_angle(1.0, true, false);
    assert_eq!(owner.time_front.to_bits(), 0x3c88_8889);
    assert!(owner.turn_front > 0.1 && owner.turn_front < 0.102);
}

#[test]
fn forced_dynamics_keep_live_arcs_source_reads_and_first_slot_on_late_error() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    owner
        .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
        .unwrap();
    owner.set_force_turn_angle(0.4, true, false);
    attributes.words = vec![
        (Class::Deck, DECK_PROFILE, 0x0100_0000),
        (Class::TrucksDrives, FORCED_STRENGTH, 2.0_f32.to_bits()),
        (Class::TrucksDrives, FORCED_DAMPING, 3.0_f32.to_bits()),
        (Class::TrucksDrives, FORCED_SPRING, 4.0_f32.to_bits()),
    ];
    attributes.next = 0;
    attributes.fail = Some(4);
    attributes.trace.borrow_mut().clear();
    let old_second = *owner.dynamics[1].read().unwrap();
    assert!(matches!(
        owner.try_update_dynamics(&mut attributes),
        Err(SkateboardTruckDriveError::Attribute("attribute"))
    ));
    let live = admission.calls[0].dynamics.read().unwrap();
    assert_eq!(live.angular.raw_type, 2);
    assert_eq!(
        live.angular.spring.to_bits(),
        (4.0 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(live.angular.damping.to_bits(), 3.0_f32.to_bits());
    assert_eq!(
        live.angular.strength.to_bits(),
        (2.0 * STRENGTH_SCALE).to_bits()
    );
    assert_eq!(*owner.dynamics[1].read().unwrap(), old_second);
    let floats: Vec<_> = attributes
        .trace
        .borrow()
        .iter()
        .filter_map(|e| {
            if let Event::Float(i) = e {
                Some(*i)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(floats, [3, 1, 2]);
}

#[test]
fn false_deck_forced_profile_reads_only_strength_and_preserves_linear_payload() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    owner
        .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
        .unwrap();
    owner.set_force_turn_angle(0.0, false, false);
    attributes.words = vec![
        (Class::Deck, DECK_PROFILE, 0),
        (Class::TrucksDrives, FORCED_STRENGTH, 1.0_f32.to_bits()),
        (Class::TrucksDrives, FORCED_STRENGTH, 2.0_f32.to_bits()),
    ];
    attributes.next = 0;
    owner.try_update_dynamics(&mut attributes).unwrap();
    for (slot, strength) in [1.0_f32, 2.0].into_iter().enumerate() {
        let dynamics = admission.calls[slot].dynamics.read().unwrap();
        assert_eq!(dynamics.linear, ZERO_PARAMS);
        assert_eq!(dynamics.angular.raw_type, 1);
        assert_eq!(dynamics.angular.spring.to_bits(), 0x4561_0000);
        assert_eq!(dynamics.angular.damping.to_bits(), 0x4279_999a);
        assert_eq!(
            dynamics.angular.strength.to_bits(),
            (strength * STRENGTH_SCALE).to_bits()
        );
    }
    assert_eq!(attributes.next, 3);
}

#[test]
fn false_deck_unforced_reads_strength_first_and_runtime_refuses_incomplete_owner() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    assert!(matches!(
        owner.try_update_dynamics(&mut attributes),
        Err(SkateboardTruckDriveError::NotComplete { .. })
    ));
    assert_eq!(attributes.next, 0);
    owner
        .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
        .unwrap();
    attributes.words = vec![(Class::Deck, DECK_PROFILE, 0)];
    for _ in 0..2 {
        attributes.words.extend([
            (Class::TrucksDrives, STRENGTH, 2.0_f32.to_bits()),
            (Class::TrucksDrives, DAMPING, 3.0_f32.to_bits()),
            (Class::TrucksDrives, SPEED_DAMPING, 4.0_f32.to_bits()),
            (Class::TrucksDrives, SPRING, 5.0_f32.to_bits()),
        ]);
    }
    attributes.next = 0;
    attributes.trace.borrow_mut().clear();
    // Controlled test state, not a claimed native damping-factor producer.
    owner.speed_damping_factor = 6.0;
    owner.try_update_dynamics(&mut attributes).unwrap();
    let floats: Vec<_> = attributes
        .trace
        .borrow()
        .iter()
        .filter_map(|e| {
            if let Event::Float(i) = e {
                Some(*i)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(floats, [1, 3, 2, 4, 5, 7, 6, 8]);
    for payload in owner.dynamics() {
        let dynamics = payload.read().unwrap();
        assert_eq!(dynamics.angular.raw_type, 1);
        assert_eq!(dynamics.angular.spring.to_bits(), 5.0_f32.to_bits());
        assert_eq!(dynamics.angular.damping.to_bits(), 27.0_f32.to_bits());
    }
}

#[test]
fn runtime_frames_use_retained_angles_and_keep_back_matrix_on_front_numeric_failure() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    owner
        .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
        .unwrap();
    owner.time_front = 1.0;
    owner.time_back = 1.0;
    owner.set_force_turn_angle(0.4, true, true);
    let old_front = owner.current_front;
    let old_frame = *admission.calls[0].frames.read().unwrap();
    numeric.fail_sin = Some(numeric.sin_count + 1);
    assert!(matches!(
        owner.try_update_frames(&mut numeric),
        Err(SkateboardTruckDriveError::Numeric("sine"))
    ));
    assert_ne!(owner.current_back, Some(levels().back));
    assert_eq!(owner.current_front, old_front);
    assert_eq!(*admission.calls[0].frames.read().unwrap(), old_frame);
    numeric.fail_sin = None;
    numeric.trace.borrow_mut().clear();
    owner.try_update_frames(&mut numeric).unwrap();
    assert!(Arc::ptr_eq(&owner.frames[0], &admission.calls[0].frames));
    assert_ne!(*admission.calls[0].frames.read().unwrap(), old_frame);
    assert_eq!(
        &numeric.trace.borrow()[..2],
        &[
            Event::Sin(0.4_f32.to_bits()),
            Event::Sin((-0.4_f32).to_bits())
        ]
    );
}

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Lookup(Class, u64),
    Byte(usize),
    Float(usize),
    Sin(u32),
    Quaternion,
    Add(usize, usize),
}
type Trace = Rc<RefCell<Vec<Event>>>;
struct Attributes {
    trace: Trace,
    words: Vec<(Class, u64, u32)>,
    next: usize,
    fail: Option<usize>,
    mutate_on_spring: bool,
}
impl SkateboardAttributeRead for Attributes {
    type Pointer = usize;
    type Error = &'static str;
    fn lookup(&mut self, class: Class, key: u64) -> Result<usize, Self::Error> {
        self.trace.borrow_mut().push(Event::Lookup(class, key));
        let index = self.next;
        if self.fail == Some(index) {
            return Err("attribute");
        }
        assert_eq!((class, key), (self.words[index].0, self.words[index].1));
        self.next += 1;
        if self.mutate_on_spring && key == SPRING {
            self.words[index - 3].2 = 3.0_f32.to_bits();
        }
        Ok(index)
    }
    fn float(&mut self, pointer: &usize) -> Result<f32, Self::Error> {
        self.trace.borrow_mut().push(Event::Float(*pointer));
        Ok(f32::from_bits(self.words[*pointer].2))
    }
    fn byte(&mut self, pointer: &usize) -> Result<u8, Self::Error> {
        self.trace.borrow_mut().push(Event::Byte(*pointer));
        Ok(self.words[*pointer].2.to_be_bytes()[0])
    }
    fn integer(&mut self, _: &usize) -> Result<i32, Self::Error> {
        panic!("no integer reads")
    }
}
struct Numeric {
    trace: Trace,
    count: usize,
    fail: Option<usize>,
    varying: bool,
    sin_count: usize,
    fail_sin: Option<usize>,
}
impl SkateboardTruckDriveNumeric for Numeric {
    type Error = &'static str;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        self.trace.borrow_mut().push(Event::Sin(angle.to_bits()));
        let index = self.sin_count;
        self.sin_count += 1;
        if self.fail_sin == Some(index) {
            return Err("sine");
        }
        Ok(angle.sin_cos())
    }
    fn quaternion_reciprocal_square_root_estimate(
        &mut self,
        values: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        self.trace.borrow_mut().push(Event::Quaternion);
        let index = self.count;
        self.count += 1;
        if self.fail == Some(index) {
            return Err("numeric");
        }
        if self.varying {
            Ok([0.1 + f32::from(u8::try_from(index).unwrap()) * 0.01; 4])
        } else {
            Ok(values.map(|value| value.sqrt().recip()))
        }
    }
}
struct Call {
    frames: Arc<RwLock<SimulationDriveFrames>>,
    dynamics: Arc<RwLock<SimulationDriveDynamics>>,
    initial_frames: SimulationDriveFrames,
    initial_dynamics: SimulationDriveDynamics,
}
struct Admission {
    trace: Trace,
    calls: Vec<Call>,
    outcomes: [Result<bool, &'static str>; 2],
    accepted: Vec<usize>,
}
impl SkateboardTruckDriveAdmission for Admission {
    type Error = &'static str;
    fn try_add_drive(
        &mut self,
        parent: usize,
        child: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, Self::Error> {
        self.trace.borrow_mut().push(Event::Add(parent, child));
        let index = self.calls.len();
        self.calls.push(Call {
            frames: Arc::clone(frames),
            dynamics: Arc::clone(dynamics),
            initial_frames: *frames.read().unwrap(),
            initial_dynamics: *dynamics.read().unwrap(),
        });
        if self.outcomes[index] == Ok(true) {
            self.accepted.push(index);
        }
        self.outcomes[index]
    }
}
fn levels() -> SkateboardTruckTransforms {
    let mut back = IDENTITY;
    back[3] = 2.0;
    back[7] = 3.0;
    back[11] = 4.0;
    back[12] = 1.0;
    back[13] = 2.0;
    back[14] = 3.0;
    let mut front = back;
    front[14] = -3.0;
    SkateboardTruckTransforms { front, back }
}
fn fixture() -> (SkateboardTruckDrives, Attributes, Numeric, Admission) {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let floats = [
        (STRENGTH, 1.0_f32),
        (DAMPING, 40.0),
        (SPEED_DAMPING, 1.0),
        (SPRING, 2222.0),
        (STRENGTH, 2.0),
        (DAMPING, 50.0),
        (SPEED_DAMPING, 2.0),
        (SPRING, 3333.0),
    ];
    let mut words = vec![(Class::Deck, DECK_PROFILE, 0x0100_0000); 2];
    words.extend(floats.map(|(key, value)| (Class::TrucksDrives, key, value.to_bits())));
    words.push((Class::Trucks, LINEAR_OVERRIDE, 0));
    (
        SkateboardTruckDrives::new(levels()),
        Attributes {
            trace: Rc::clone(&trace),
            words,
            next: 0,
            fail: None,
            mutate_on_spring: false,
        },
        Numeric {
            trace: Rc::clone(&trace),
            count: 0,
            fail: None,
            varying: false,
            sin_count: 0,
            fail_sin: None,
        },
        Admission {
            trace,
            calls: Vec::new(),
            outcomes: [Ok(true); 2],
            accepted: Vec::new(),
        },
    )
}

#[test]
fn combined_callbacks_preserve_both_frame_passes_zero_count_lookup_and_delayed_loads() {
    use Event::{Add, Byte as B, Float as F, Lookup as L, Quaternion as Q, Sin as S};
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    owner
        .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
        .unwrap();
    assert_eq!(
        *attributes.trace.borrow(),
        vec![
            S(0),
            S(0x8000_0000),
            Q,
            Q,
            Q,
            Q,
            L(Class::Deck, DECK_PROFILE),
            B(0),
            Add(6, 4),
            Add(6, 5),
            L(Class::Deck, DECK_PROFILE),
            B(1),
            L(Class::TrucksDrives, STRENGTH),
            L(Class::TrucksDrives, DAMPING),
            L(Class::TrucksDrives, SPEED_DAMPING),
            L(Class::TrucksDrives, SPRING),
            F(4),
            F(3),
            F(2),
            F(5),
            L(Class::TrucksDrives, STRENGTH),
            L(Class::TrucksDrives, DAMPING),
            L(Class::TrucksDrives, SPEED_DAMPING),
            L(Class::TrucksDrives, SPRING),
            F(8),
            F(7),
            F(6),
            F(9),
            S(0),
            S(0x8000_0000),
            Q,
            Q,
            Q,
            Q,
            L(Class::Trucks, LINEAR_OVERRIDE),
            B(10),
        ]
    );
    assert_eq!(owner.count(), 2);
    assert_eq!(owner.phase(), SkateboardTruckDrivePhase::Complete);
    assert_eq!(numeric.count, 8);
}

#[test]
fn insertion_retains_zero_dynamics_owners_then_observes_independent_live_updates() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    numeric.varying = true;
    owner
        .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
        .unwrap();
    assert!(!Arc::ptr_eq(&owner.frames()[0], &owner.frames()[1]));
    assert!(!Arc::ptr_eq(&owner.dynamics()[0], &owner.dynamics()[1]));
    for (slot, call) in admission.calls.iter().enumerate() {
        assert_eq!(
            call.initial_dynamics,
            SimulationDriveDynamics {
                linear: ZERO_PARAMS,
                angular: ZERO_PARAMS
            }
        );
        assert!(Arc::ptr_eq(&call.frames, &owner.frames()[slot]));
        assert!(Arc::ptr_eq(&call.dynamics, &owner.dynamics()[slot]));
        let frames = call.frames.read().unwrap();
        assert_ne!(frames.quaternion_a, call.initial_frames.quaternion_a);
        assert_eq!(frames.position_a.map(f32::to_bits), [0; 4]);
        let dynamics = call.dynamics.read().unwrap();
        assert_eq!(dynamics.linear, ZERO_PARAMS);
        let expected = if slot == 0 {
            (2222.0, 40.0, 1.0)
        } else {
            (3333.0, 50.0, 2.0)
        };
        assert_eq!(
            dynamics.angular,
            SimulationDriveDynamicsParams {
                spring: expected.0,
                damping: expected.1,
                strength: expected.2 * STRENGTH_SCALE,
                raw_type: 1,
            }
        );
    }
}

#[test]
fn all_native_null_combinations_attempt_both_and_advance_count_without_rollback() {
    for first in [false, true] {
        for second in [false, true] {
            let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
            admission.outcomes = [Ok(first), Ok(second)];
            owner
                .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
                .unwrap();
            assert_eq!(owner.add_results(), [Some(first), Some(second)]);
            assert_eq!(owner.count(), 2);
            assert_eq!(admission.calls.len(), 2);
            assert_eq!(
                admission.accepted.len(),
                usize::from(first) + usize::from(second)
            );
            assert_eq!(owner.dynamics()[1].read().unwrap().angular.raw_type, 1);
        }
    }
}

#[test]
fn second_admission_host_error_retains_first_result_and_count_before_increment() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    admission.outcomes[1] = Err("admission");
    assert_eq!(
        owner.try_create_fresh(&mut attributes, &mut numeric, &mut admission),
        Err(SkateboardTruckDriveError::Admission("admission"))
    );
    assert_eq!(owner.count(), 0);
    assert_eq!(owner.add_results(), [Some(true), None]);
    assert_eq!(admission.accepted, vec![0]);
    assert_eq!(owner.phase(), SkateboardTruckDrivePhase::SecondInsertion);
    let count = attributes.trace.borrow().len();
    assert_eq!(
        owner.try_create_fresh(&mut attributes, &mut numeric, &mut admission),
        Err(SkateboardTruckDriveError::NotFresh {
            phase: SkateboardTruckDrivePhase::SecondInsertion
        })
    );
    assert_eq!(attributes.trace.borrow().len(), count);
}

#[test]
fn postinsert_attribute_failure_preserves_first_completed_dynamics_slot() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    attributes.fail = Some(6);
    assert_eq!(
        owner.try_create_fresh(&mut attributes, &mut numeric, &mut admission),
        Err(SkateboardTruckDriveError::Attribute("attribute"))
    );
    assert_eq!(owner.count(), 2);
    assert_eq!(owner.phase(), SkateboardTruckDrivePhase::PostDynamics);
    assert_eq!(owner.dynamics()[0].read().unwrap().angular.spring, 2222.0);
    assert_eq!(owner.dynamics()[1].read().unwrap().angular, ZERO_PARAMS);
    assert_eq!(admission.accepted, vec![0, 1]);
    assert_eq!(numeric.count, 4);
}

#[test]
fn failed_parent_setter_keeps_completed_child_write_in_live_frame() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    numeric.varying = true;
    numeric.fail = Some(5);
    assert_eq!(
        owner.try_create_fresh(&mut attributes, &mut numeric, &mut admission),
        Err(SkateboardTruckDriveError::Numeric("numeric"))
    );
    assert_eq!(owner.phase(), SkateboardTruckDrivePhase::PostFrames);
    let current = owner.frames()[0].read().unwrap();
    assert_ne!(
        current.quaternion_a,
        admission.calls[0].initial_frames.quaternion_a
    );
    assert_eq!(
        current.quaternion_b,
        admission.calls[0].initial_frames.quaternion_b
    );
    assert_eq!(attributes.next, 10);
    assert_eq!(admission.accepted, vec![0, 1]);
}

#[test]
fn unsupported_bytes_refuse_at_original_sampling_site_and_not_as_word_truthiness() {
    for profile_index in [0, 1] {
        let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
        attributes.words[profile_index].2 = 1; // first byte zero despite nonzero word
        assert_eq!(
            owner.try_create_fresh(&mut attributes, &mut numeric, &mut admission),
            Err(SkateboardTruckDriveError::UnsupportedDeckProfile { value: 0 })
        );
        assert_eq!(numeric.count, 4);
        assert_eq!(admission.calls.len(), profile_index * 2);
        assert_eq!(owner.count(), profile_index * 2);
    }
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    attributes.words[10].2 = 0x0200_0000;
    assert_eq!(
        owner.try_create_fresh(&mut attributes, &mut numeric, &mut admission),
        Err(SkateboardTruckDriveError::UnsupportedLinearOverride { value: 2 })
    );
    assert_eq!(owner.count(), 2);
    assert_eq!(numeric.count, 8);
    assert_eq!(owner.phase(), SkateboardTruckDrivePhase::OptionalLinear);
    assert_eq!(owner.dynamics()[0].read().unwrap().linear, ZERO_PARAMS);
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    attributes.words[0].2 = 0xff00_0000;
    attributes.words[10].2 = 1;
    owner
        .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
        .unwrap();
}

#[test]
fn retained_scalar_pointers_load_after_all_four_lookups_and_keep_exceptional_fma() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    attributes.mutate_on_spring = true;
    attributes.words[4].2 = f32::INFINITY.to_bits();
    attributes.words[5].2 = (-0.0_f32).to_bits();
    owner
        .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
        .unwrap();
    let dynamics = owner.dynamics()[0].read().unwrap();
    assert_eq!(dynamics.angular.strength, 3.0 * STRENGTH_SCALE);
    assert!(dynamics.angular.damping.is_nan()); // source 0*infinity+base, not just base
    assert_eq!(dynamics.angular.spring.to_bits(), 0x8000_0000);
    // Synthetic nonfresh operands discriminate the private authored FMA itself;
    // they do not add a live nonzero-speed tuning setter to the fresh owner.
    assert_eq!(
        speed_damping(f32::from_bits(1), 0.5, f32::from_bits(1)).to_bits(),
        2
    );
    assert_eq!((f32::from_bits(1) * 0.5 + f32::from_bits(1)).to_bits(), 1);
}

#[test]
fn turn_and_relative_matrices_preserve_raw_w_and_source_translation_operations() {
    let level = levels().back;
    let turned = apply_turn(level, (2.0, 3.0));
    assert_eq!(&turned[0..4], &[1.0, 0.0, 0.0, 2.0]);
    assert_eq!(&turned[4..8], &[0.0, 3.0, 2.0, 17.0]);
    assert_eq!(&turned[8..12], &[0.0, -2.0, 3.0, 6.0]);
    assert_eq!(&turned[12..], &[1.0, 2.0, 3.0, 0.0]);
    let relative = relative_frame(IDENTITY, turned);
    assert_eq!(
        [relative[3], relative[7], relative[11], relative[15]],
        [0.0; 4]
    );
    assert_eq!(&relative[12..], &[1.0, 2.0, 3.0, 0.0]);
    let mut exceptional = level;
    exceptional[0] = f32::INFINITY;
    assert!(apply_turn(exceptional, (0.0, 1.0))[12].is_nan());
    let mut parent = IDENTITY;
    parent[12] = 4.0;
    parent[13] = 5.0;
    parent[14] = 6.0;
    assert_eq!(
        &relative_frame(parent, level)[12..],
        &[-3.0, -3.0, -3.0, 0.0]
    );
}

#[test]
fn sine_failure_preserves_first_matrix_store_and_poison_refuses_without_defaults() {
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    assert!(owner.current_back().is_none());
    assert!(owner.current_front().is_none());
    numeric.fail_sin = Some(1);
    assert_eq!(
        owner.try_create_fresh(&mut attributes, &mut numeric, &mut admission),
        Err(SkateboardTruckDriveError::Numeric("sine"))
    );
    assert!(owner.current_back().is_some());
    assert!(owner.current_front().is_none());
    assert!(admission.calls.is_empty());
    assert_eq!(attributes.next, 0);
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    let lock = Arc::clone(&owner.frames()[0]);
    let _ = std::panic::catch_unwind(move || {
        let _guard = lock.write().unwrap();
        panic!("synthetic poison");
    });
    assert_eq!(
        owner.try_create_fresh(&mut attributes, &mut numeric, &mut admission),
        Err(SkateboardTruckDriveError::Poisoned {
            slot: 0,
            payload: SkateboardTruckDrivePayload::Frames
        })
    );
    assert_eq!(numeric.count, 1);
    assert!(admission.calls.is_empty());
}

#[test]
fn noncommuting_relative_axes_and_fused_turn_cancellation_are_distinct() {
    // Parent Rz(+90), child Rx(+90): inverse(parent)*child has columns
    // (0,-1,0), (0,0,1), (-1,0,0). Raw input W lanes do not survive inverse.
    let parent = [
        0.0, 1.0, 0.0, 7.0, -1.0, 0.0, 0.0, 8.0, 0.0, 0.0, 1.0, 9.0, 4.0, 5.0, 6.0, 10.0,
    ];
    let child = [
        1.0, 0.0, 0.0, 11.0, 0.0, 0.0, 1.0, 12.0, 0.0, -1.0, 0.0, 13.0, 1.0, 2.0, 3.0, 14.0,
    ];
    assert_eq!(
        relative_frame(parent, child),
        [
            0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, -3.0, 3.0, -3.0, 0.0
        ]
    );
    // Last Z FMA computes (1+2^-23)*(1-2^-23)-1 = -2^-46 exactly.
    // An independently rounded product would produce 1 and cancel to zero.
    let mut level = IDENTITY;
    level[4] = -1.0;
    level[8] = f32::from_bits(0x3f80_0001);
    let sine = f32::from_bits(0x3f7f_fffe);
    assert_eq!(apply_turn(level, (sine, 1.0))[4].to_bits(), 0xa880_0000);
    assert_eq!((level[8] * sine - 1.0).to_bits(), 0);
}

#[test]
fn host_numeric_policy_and_completed_retry_refusal_are_explicit() {
    let mut host = HostSkateboardTruckDriveNumeric;
    assert_eq!(host.sine_cosine(-0.0).unwrap().0.to_bits(), 0x8000_0000);
    assert_eq!(
        host.quaternion_reciprocal_square_root_estimate([1.0, 4.0, 16.0, 64.0])
            .unwrap(),
        [1.0, 0.5, 0.25, 0.125]
    );
    let (mut owner, mut attributes, mut numeric, mut admission) = fixture();
    owner
        .try_create_fresh(&mut attributes, &mut numeric, &mut admission)
        .unwrap();
    assert_eq!(
        owner.try_create_fresh(&mut attributes, &mut numeric, &mut admission),
        Err(SkateboardTruckDriveError::NotFresh {
            phase: SkateboardTruckDrivePhase::Complete
        })
    );
    assert_eq!(admission.calls.len(), 2);
}
