//! Source graph, service-prefix and actual-record/offset consumer discriminators.
//! Frozen finite arithmetic is a host source oracle, not a captured Xenon trace.
#![expect(
    clippy::float_cmp,
    reason = "exact fixture values; signed-zero and fused-rounding cases separately compare bits"
)]
use super::*;
use crate::{
    skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_animation_record::SkeletonAnimationRecordInput,
    skeleton_data_tests::{build, fixture},
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_physics_history::{
        HostSkeletonPhysicsHistoryNumeric, SkeletonHistoryInput, SkeletonPhysicsHistory,
    },
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

type S = SkeletonLandingAdjustScalar;
type K = SkeletonLandingAdjustKind;
type Error = SkeletonLandingAdjustError<&'static str, &'static str>;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Scalar(S),
    Manual(u32),
    Grind(u32),
    Coffin(u32),
    Debug,
    Overlay,
    Physics,
    Hack,
    Frames,
    Dot,
}

#[derive(Clone, Default)]
struct Log {
    events: Rc<RefCell<Vec<Event>>>,
    fail_at: Rc<RefCell<Option<usize>>>,
}
impl Log {
    fn emit(&self, event: Event) -> Result<(), &'static str> {
        let mut events = self.events.borrow_mut();
        let index = events.len();
        events.push(event);
        if *self.fail_at.borrow() == Some(index) {
            Err("injected")
        } else {
            Ok(())
        }
    }
    fn clear(&self) {
        self.events.borrow_mut().clear();
    }
    fn events(&self) -> Vec<Event> {
        self.events.borrow().clone()
    }
}

struct Attributes {
    log: Log,
    overrides: Vec<(S, f32)>,
    manual: f32,
    grind: f32,
    bounce: f32,
}
impl Attributes {
    fn set(&mut self, key: S, value: f32) {
        self.overrides.push((key, value));
    }
}
impl SkeletonLandingAdjustAttributes for Attributes {
    type Error = &'static str;
    fn scalar(&mut self, key: S) -> Result<f32, Self::Error> {
        self.log.emit(Event::Scalar(key))?;
        if let Some((_, value)) = self.overrides.iter().rev().find(|(k, _)| *k == key) {
            return Ok(*value);
        }
        Ok(match key {
            S::MaximumVelocity | S::CoffinVelocityDenominator => 8.0,
            S::CoffinDuration | S::GroundMinimumElapsed => 1.0,
            S::CoffinBase => 4.0,
            S::DesiredCom => 5.0,
            S::MinimumYPosition => -100.0,
            S::CoffinFrames => 12.0,
            S::GrindTargetSpeed => 0.25,
            S::GroundK1 | S::GrindK1 | S::ManualK1 => 2.0,
            S::GrindTargetAfterTime | S::GroundK2 | S::GrindK2 | S::ManualK2 => 0.5,
        })
    }
    fn manual_blend(&mut self, time: f32) -> Result<f32, Self::Error> {
        self.log.emit(Event::Manual(time.to_bits()))?;
        Ok(self.manual)
    }
    fn grind_blend(&mut self, time: f32) -> Result<f32, Self::Error> {
        self.log.emit(Event::Grind(time.to_bits()))?;
        Ok(self.grind)
    }
    fn coffin_bounce(&mut self, time: f32) -> Result<f32, Self::Error> {
        self.log.emit(Event::Coffin(time.to_bits()))?;
        Ok(self.bounce)
    }
}
struct Cvars {
    log: Log,
    debug: u8,
    physics: u8,
    hack: f32,
    frames: f32,
}
impl SkeletonLandingAdjustCvars for Cvars {
    type Error = &'static str;
    fn debug_landing_adjust(&mut self) -> Result<u8, Self::Error> {
        self.log.emit(Event::Debug)?;
        Ok(self.debug)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.log.emit(Event::Overlay)
    }
    fn use_physics_com(&mut self) -> Result<u8, Self::Error> {
        self.log.emit(Event::Physics)?;
        Ok(self.physics)
    }
    fn first_frame_hack(&mut self) -> Result<f32, Self::Error> {
        self.log.emit(Event::Hack)?;
        Ok(self.hack)
    }
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.log.emit(Event::Frames)?;
        Ok(self.frames)
    }
}
struct Numeric {
    log: Log,
    inputs: Vec<([f32; 4], [f32; 4])>,
}
impl SkeletonPhysicsHistoryNumeric for Numeric {
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> f32 {
        self.log.emit(Event::Dot).unwrap();
        self.inputs.push((a, b));
        HostSkeletonPhysicsHistoryNumeric.dot3(a, b)
    }
}
struct RecordInput;
impl SkeletonHistoryInput for RecordInput {
    type Error = &'static str;
    type Up = ();
    fn acquire_up_vector(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        Ok(1)
    }
    fn read_up_vector(&mut self, (): &()) -> Result<[f32; 4], Self::Error> {
        Ok([0.0, 1.0, 0.0, 0.0])
    }
}
impl SkeletonAnimationRecordInput for RecordInput {
    type Error = &'static str;
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        Ok(1)
    }
}
fn matrix(position: [f32; 4]) -> AnimationMatrix {
    let mut result = FRESH_VOLUME_TRANSFORM;
    result[12..].copy_from_slice(&position);
    result
}

struct Harness {
    log: Log,
    input: SkeletonLandingAdjustInput,
    physics: SkeletonPhysicsHistory,
    animation: SkeletonAnimationRecord,
    mapped: AnimationMatrix,
    offset: SkeletonSkateboardOffset,
    attributes: Attributes,
    cvars: Cvars,
    numeric: Numeric,
}
impl Harness {
    fn new() -> Self {
        let log = Log::default();
        let mut masses = [0.0; 24];
        masses[1] = 1.0;
        let mut physics = SkeletonPhysicsHistory::new(masses);
        physics.try_initialize_masses().unwrap();
        physics
            .try_update(
                matrix([1.0, 2.0, 3.0, 4.0]),
                &(),
                &mut |_| Ok::<_, &'static str>(matrix([5.0, 8.0, 11.0, 14.0])),
                &mut RecordInput,
                &mut HostSkeletonPhysicsHistoryNumeric,
            )
            .unwrap();
        let mut animation = SkeletonAnimationRecord::new();
        animation
            .try_update(
                &[matrix([20.0, 30.0, 40.0, 50.0]); 24],
                &physics,
                &SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
                &mut RecordInput,
            )
            .unwrap();
        Self {
            attributes: Attributes {
                log: log.clone(),
                overrides: vec![],
                manual: 0.75,
                grind: 0.5,
                bounce: 2.0,
            },
            cvars: Cvars {
                log: log.clone(),
                debug: 0,
                physics: 0,
                hack: 0.0,
                frames: 15.0,
            },
            numeric: Numeric {
                log: log.clone(),
                inputs: vec![],
            },
            input: SkeletonLandingAdjustInput {
                filtered_state: 1,
                flags_8c4: 0,
                flags_8c8: 0,
                flags_8cc: 0x4000_0000,
                balance: 0.0,
                centre_of_mass_velocity: [100.0, -3.0, 200.0, 300.0],
                up: [0.0, 1.0, 0.0, 0.0],
                processed_deck_y: 100.0,
            },
            physics,
            animation,
            mapped: matrix([3.0, 4.0, 5.0, 6.0]),
            offset: SkeletonSkateboardOffset::new(),
            log,
        }
    }
    fn update(&mut self, owner: &mut SkeletonLandingAdjust) -> Result<(), Error> {
        owner.try_update(
            &self.input,
            self.physics.record(),
            &self.animation,
            &self.mapped,
            &mut self.offset,
            &mut self.attributes,
            &mut self.cvars,
            &mut self.numeric,
        )
    }
    fn fail(&mut self, index: usize) {
        *self.log.fail_at.borrow_mut() = Some(index);
    }
}
fn start() -> SkeletonLandingAdjust {
    let mut owner = SkeletonLandingAdjust::new();
    owner.seed_history(false, 2, -7.0);
    owner
}
fn active(kind: K) -> SkeletonLandingAdjust {
    let mut owner = start();
    owner.kind = kind;
    owner.adjusting = true;
    owner.y_position = 6.0;
    owner.y_velocity = -3.0;
    owner.last_filtered_state = 1;
    owner
}
fn bits(value: f32) -> u32 {
    value.to_bits()
}

#[test]
fn constructor_retains_all_nine_source_defaults() {
    let owner = SkeletonLandingAdjust::new();
    assert!(!owner.adjusting());
    assert_eq!(owner.kind(), K::Ground);
    assert_eq!(
        [
            owner.time(),
            owner.y_position(),
            owner.y_velocity(),
            owner.second_last_com_velocity(),
            owner.desired_com_for_grind()
        ]
        .map(bits),
        [0; 5]
    );
    assert_eq!(bits(owner.last_animation_position()), 0x7f7f_ffff);
    assert_eq!(owner.last_filtered_state(), 0);
}

#[test]
fn all_four_start_modes_and_priority_commit_before_active_services() {
    for (filtered, coffin, balance, expected, expected_velocity) in [
        (1, 0, 0.0, K::Ground, -3.0),
        (1, 0, 1.0, K::Manual, -3.0),
        (1, 4, 1.0, K::Coffin, -7.0),
        (3, 0, 0.0, K::Grind, -7.0),
        (3, 4, 0.0, K::Coffin, -7.0),
        (1, 0, f32::NAN, K::Manual, -3.0),
    ] {
        let mut h = Harness::new();
        let mut owner = start();
        h.input.filtered_state = filtered;
        h.input.flags_8c8 = coffin;
        h.input.balance = balance;
        h.fail(5); // max,dot,debug,startdot,startdebug,active-physics-cvar
        assert_eq!(h.update(&mut owner), Err(Error::Cvar("injected")));
        assert!(owner.adjusting());
        assert_eq!(owner.kind(), expected);
        assert_eq!(bits(owner.y_velocity()), bits(expected_velocity));
        assert_eq!(owner.y_position(), 6.0);
        assert_eq!(bits(owner.time()), 0);
        assert_eq!(bits(owner.last_animation_position()), 0x7f7f_ffff);
        assert_eq!(owner.last_filtered_state(), 2);
        assert_eq!(owner.second_last_com_velocity(), -7.0);
        assert!(!h.offset.vertical_just_set);
    }
}

#[test]
fn maximum_attribute_precedes_disable_and_debug_failure_preserves_disable() {
    let mut h = Harness::new();
    let mut owner = active(K::Ground);
    h.input.flags_8c8 = 1;
    h.fail(0);
    assert_eq!(h.update(&mut owner), Err(Error::Attribute("injected")));
    assert!(owner.adjusting());
    assert!(h.numeric.inputs.is_empty());
    h.log.clear();
    h.fail(2);
    assert_eq!(h.update(&mut owner), Err(Error::Cvar("injected")));
    assert!(!owner.adjusting());
    assert_eq!(owner.second_last_com_velocity(), -7.0);
    assert_eq!(owner.y_position(), 6.0);
}

#[test]
fn inactive_updates_only_processed_velocity_and_filtered_history() {
    let mut h = Harness::new();
    let mut owner = SkeletonLandingAdjust::new();
    h.input.filtered_state = 2;
    h.update(&mut owner).unwrap();
    assert_eq!(
        h.log.events(),
        vec![Event::Scalar(S::MaximumVelocity), Event::Dot, Event::Debug]
    );
    assert_eq!(owner.second_last_com_velocity(), -3.0);
    assert_eq!(owner.last_filtered_state(), 2);
    assert_eq!(bits(owner.last_animation_position()), 0x7f7f_ffff);
}

#[test]
fn starting_height_uses_full_difference_of_actual_physics_records() {
    let mut h = Harness::new();
    let mut owner = start();
    h.input.up = [1.0, 2.0, 3.0, f32::NAN];
    h.fail(5);
    assert_eq!(h.update(&mut owner), Err(Error::Cvar("injected")));
    assert_eq!(owner.y_position(), 40.0); // 4*1 + 6*2 + 8*3
    assert_eq!(
        h.numeric.inputs[1].0.map(bits),
        [4.0, 6.0, 8.0, 10.0].map(bits)
    );
    assert!(h.numeric.inputs[1].1[3].is_nan());
    // The processed dot is +694 and clamps to zero; physical COM velocity is unrelated.
    assert_eq!(bits(owner.y_velocity()), 0);
}

#[test]
fn ground_two_frames_match_frozen_source_rounding_and_prior_animation_com() {
    let mut h = Harness::new();
    let mut owner = start();
    h.update(&mut owner).unwrap();
    assert_eq!(bits(owner.y_velocity()), 0xc040_8889);
    assert_eq!(bits(owner.y_position()), 0x40be_65d5);
    assert_eq!(bits(h.offset.transform[13]), 0x41a0_668b);
    assert_eq!(owner.last_animation_position(), 26.0);
    assert_eq!(bits(owner.time()), 0x3c88_8889);
    h.log.clear();
    h.update(&mut owner).unwrap();
    assert_eq!(bits(owner.y_velocity()), 0xc040_f496);
    assert_eq!(bits(owner.y_position()), 0x40bc_caa5);
    assert_eq!(bits(h.offset.transform[13]), 0x41a0_cd57);
    assert!(!h.log.events().contains(&Event::Hack));
    assert_eq!(h.offset.vertical_count, 15.0);
}

#[test]
fn manual_allows_upward_spring_velocity_while_ground_caps_it_at_zero() {
    for kind in [K::Ground, K::Manual] {
        let mut h = Harness::new();
        let mut owner = active(kind);
        owner.y_position = 0.0;
        owner.y_velocity = 0.0;
        h.update(&mut owner).unwrap();
        if kind == K::Ground {
            assert_eq!(bits(owner.y_velocity()), 0);
        } else {
            assert_eq!(bits(owner.y_velocity()), bits(10.0 * UPDATE_TIME));
        }
        assert_eq!(
            h.log.events().iter().any(|e| matches!(e, Event::Manual(0))),
            kind == K::Manual
        );
    }
}

#[test]
fn grind_uses_strict_target_time_and_per_frame_speed_not_dt() {
    let mut h = Harness::new();
    let mut owner = active(K::Grind);
    h.input.filtered_state = 3;
    owner.time = 0.5;
    owner.desired_com_for_grind = 10.0;
    h.update(&mut owner).unwrap();
    assert_eq!(owner.desired_com_for_grind(), 5.0);
    assert!(!h.log.events().contains(&Event::Scalar(S::GrindTargetSpeed)));
    h.log.clear();
    h.update(&mut owner).unwrap();
    assert_eq!(owner.desired_com_for_grind(), 5.25);
    assert!(h.log.events().contains(&Event::Scalar(S::GrindTargetSpeed)));
    assert!(
        h.log
            .events()
            .contains(&Event::Grind(bits(0.5 + UPDATE_TIME)))
    );
    h.log.clear();
    owner.time = f32::NAN;
    h.update(&mut owner).unwrap();
    assert_eq!(owner.desired_com_for_grind(), 5.0);
    assert!(!h.log.events().contains(&Event::Scalar(S::GrindTargetSpeed)));
}

#[test]
fn coffin_scales_graph_by_clamped_retained_speed_and_uses_its_own_frames() {
    let mut h = Harness::new();
    let mut owner = active(K::Coffin);
    h.input.flags_8c8 = 4;
    owner.time = 0.25;
    owner.y_velocity = -4.0;
    h.attributes.set(S::CoffinDuration, 0.5);
    h.update(&mut owner).unwrap();
    assert_eq!(owner.y_position(), 5.0);
    assert_eq!(owner.y_velocity(), -4.0);
    assert_eq!(h.offset.transform[13], 21.0);
    assert_eq!(h.offset.vertical_count, 12.0);
    assert!(h.log.events().contains(&Event::Coffin(bits(0.5))));
    assert!(!h.log.events().contains(&Event::Frames));
    assert!(!h.log.events().contains(&Event::Scalar(S::MinimumYPosition)));
    assert_eq!(
        h.log
            .events()
            .iter()
            .filter(|e| **e == Event::Debug)
            .count(),
        2
    );
}

#[test]
fn duration_zero_asserts_in_every_mode_after_start_but_nan_passes() {
    for kind in [K::Ground, K::Grind, K::Manual, K::Coffin] {
        for duration in [0.0, -0.0] {
            let mut h = Harness::new();
            let mut owner = active(kind);
            h.attributes.set(S::CoffinDuration, duration);
            assert_eq!(h.update(&mut owner), Err(Error::ZeroCoffinDuration));
            assert_eq!(
                h.log.events().last(),
                Some(&Event::Scalar(S::CoffinDuration))
            );
            assert_eq!(owner.y_velocity(), -3.0);
            assert_eq!(owner.y_position(), 6.0);
            assert_eq!(bits(owner.time()), 0);
            assert_eq!(owner.second_last_com_velocity(), -7.0);
        }
    }
    let mut h = Harness::new();
    let mut owner = start();
    h.attributes.set(S::CoffinDuration, f32::NAN);
    h.update(&mut owner).unwrap();
    assert!(owner.adjusting());
    assert_eq!(bits(owner.y_position()), 0x40be_65d5);
}

#[test]
fn continuation_gate_keeps_threshold_tie_and_nan_but_rejects_smaller_blend() {
    for (blend, keep) in [
        (f32::from_bits(0x3d4c_cccc), false),
        (f32::from_bits(0x3d4c_cccd), true),
        (f32::NAN, true),
    ] {
        let mut h = Harness::new();
        let mut owner = active(K::Manual);
        h.attributes.manual = blend;
        h.update(&mut owner).unwrap();
        assert_eq!(owner.adjusting(), keep);
        assert_eq!(h.offset.vertical_just_set, keep);
        assert_eq!(h.log.events().contains(&Event::Frames), keep);
        assert_eq!(bits(owner.time()), 0x3c88_8889);
        assert_eq!(owner.last_animation_position(), 26.0);
        if blend.is_nan() {
            assert!(h.offset.transform[13].is_nan());
        }
    }
}

#[test]
fn stop_conditions_still_integrate_and_commit_active_time_and_height() {
    for (kind, state, flags, time) in [
        (K::Manual, 2, 0, 0.0),
        (K::Grind, 3, 0x20, 0.0),
        (K::Manual, 1, 0, f32::from_bits(0x3fc0_0001)),
    ] {
        let mut h = Harness::new();
        let mut owner = active(kind);
        h.input.filtered_state = state;
        h.input.flags_8c4 = flags;
        owner.time = time;
        h.update(&mut owner).unwrap();
        assert!(!owner.adjusting());
        assert!(!h.offset.vertical_just_set);
        assert_eq!(bits(owner.time()), bits(time + UPDATE_TIME));
        assert_eq!(owner.last_animation_position(), 26.0);
        assert_eq!(bits(owner.y_position()), 0x40be_65d5);
    }
    let mut h = Harness::new();
    let mut owner = active(K::Manual);
    owner.time = 1.5;
    h.update(&mut owner).unwrap();
    assert!(owner.adjusting());
}

#[test]
fn prepare_jump_reads_count_before_deactivation_and_override() {
    for kind in [K::Manual, K::Coffin] {
        let mut h = Harness::new();
        let mut owner = active(kind);
        h.input.flags_8c4 = 0x20_0000;
        h.input.filtered_state = 2;
        h.input.flags_8c8 = 4;
        h.update(&mut owner).unwrap();
        assert!(!owner.adjusting());
        assert_eq!(h.offset.vertical_count, 6.0);
        let count = if kind == K::Coffin {
            Event::Scalar(S::CoffinFrames)
        } else {
            Event::Frames
        };
        let index = h.log.events().iter().position(|e| *e == count).unwrap();
        let mut h = Harness::new();
        let mut owner = active(kind);
        h.input.flags_8c4 = 0x20_0000;
        h.input.filtered_state = 2;
        h.input.flags_8c8 = 4;
        h.fail(index);
        let expected = if kind == K::Coffin {
            Error::Attribute("injected")
        } else {
            Error::Cvar("injected")
        };
        assert_eq!(h.update(&mut owner), Err(expected));
        assert!(owner.adjusting());
        assert!(!h.offset.vertical_just_set);
        assert_eq!(bits(owner.time()), 0);
    }
}

#[test]
fn every_service_failure_stops_at_source_prefix_before_final_offset_and_history() {
    let mut success = Harness::new();
    success.cvars.debug = 1;
    success.update(&mut start()).unwrap();
    let events = success.log.events();
    assert_eq!(events.iter().filter(|e| **e == Event::Overlay).count(), 8);
    assert_eq!(events.iter().filter(|e| **e == Event::Debug).count(), 4);
    for (index, event) in events.iter().enumerate() {
        if *event == Event::Dot {
            continue;
        }
        let mut h = Harness::new();
        let mut owner = start();
        h.cvars.debug = 1;
        h.fail(index);
        assert!(h.update(&mut owner).is_err(), "index {index}: {event:?}");
        assert_eq!(h.log.events(), events[..=index]);
        assert_eq!(owner.second_last_com_velocity(), -7.0);
        assert_eq!(owner.last_filtered_state(), 2);
        assert_eq!(bits(owner.time()), 0);
        assert!(!h.offset.vertical_just_set);
        assert_eq!(bits(owner.last_animation_position()), 0x7f7f_ffff);
    }
}

#[test]
fn spring_velocity_commit_precedes_debug_and_minimum_position_services() {
    let mut success = Harness::new();
    success.update(&mut start()).unwrap();
    let events = success.log.events();
    let minimum = events
        .iter()
        .position(|e| *e == Event::Scalar(S::MinimumYPosition))
        .unwrap();
    for index in [minimum - 1, minimum] {
        let mut h = Harness::new();
        let mut owner = start();
        h.fail(index);
        assert!(h.update(&mut owner).is_err());
        assert_eq!(bits(owner.y_velocity()), 0xc040_8889);
        assert_eq!(owner.y_position(), 6.0);
        assert!(owner.adjusting());
        assert_eq!(owner.second_last_com_velocity(), -7.0);
    }
}

#[test]
fn physical_com_option_is_sampled_always_but_only_applies_offboard_to_ground() {
    for old_state in [2, 6] {
        let mut h = Harness::new();
        let mut owner = start();
        owner.last_filtered_state = old_state;
        h.cvars.physics = 1;
        h.input.processed_deck_y = 2.0;
        h.input.up = [1.0, 0.0, 0.0, 0.0]; // processed dot is100, processed velocity.Y is-3
        h.update(&mut owner).unwrap();
        let expected = if old_state == 6 {
            (-3.0_f32).mul_add(UPDATE_TIME, 6.0)
        } else {
            26.0
        };
        assert_eq!(bits(owner.last_animation_position()), bits(expected));
        assert!(h.log.events().contains(&Event::Physics));
    }
}

#[test]
fn first_ground_hack_clamps_to_negative_tenth_and_is_not_repeated() {
    let mut h = Harness::new();
    let mut owner = start();
    h.cvars.hack = 100.0;
    h.update(&mut owner).unwrap();
    assert_eq!(
        bits(h.offset.transform[13]),
        bits(f32::from_bits(0x41a0_668b) - 0.1)
    );
    h.log.clear();
    h.update(&mut owner).unwrap();
    assert!(!h.log.events().contains(&Event::Hack));
    assert_eq!(bits(h.offset.transform[13]), 0x41a0_cd57);
    // A low animation target is clamped only by the first-frame ground hack.
    let mut h = Harness::new();
    let mut owner = start();
    h.mapped[13] = 100.0;
    h.update(&mut owner).unwrap();
    assert_eq!(bits(h.offset.transform[13]), 0);
    h.update(&mut owner).unwrap();
    assert!(h.offset.transform[13] < 0.0);
}

#[test]
fn source_fsel_exceptional_values_do_not_use_rust_min_max_nan_rules() {
    // NaN denominator yields ratioNaN, lower selectionNaN, upper selection1.
    let mut h = Harness::new();
    let mut owner = active(K::Coffin);
    h.input.flags_8c8 = 4;
    h.attributes.set(S::CoffinVelocityDenominator, f32::NAN);
    h.update(&mut owner).unwrap();
    assert_eq!(owner.y_position(), 6.0);
    // NaN candidate against a finite minimum selects the minimum.
    let mut h = Harness::new();
    let mut owner = active(K::Manual);
    owner.y_position = f32::NAN;
    h.update(&mut owner).unwrap();
    assert_eq!(owner.y_position(), -100.0);
    assert_eq!(owner.y_velocity(), 8.0);
}

#[test]
fn landing_y_preserves_full_state_and_reaches_mapped_deck_and_all_four_ik_targets() {
    let mut h = Harness::new();
    let mut owner = start();
    h.offset.transform[12] = 2.0;
    h.offset.transform[14] = 3.0;
    h.offset.transform[15] = 7.0;
    h.offset.full_count = -2.0;
    h.offset.full_just_set = true;
    h.update(&mut owner).unwrap();
    assert_eq!(
        [
            h.offset.transform[12],
            h.offset.transform[14],
            h.offset.transform[15]
        ],
        [2.0, 3.0, 7.0]
    );
    assert_eq!(h.offset.full_count, -2.0);
    assert!(h.offset.full_just_set);
    let correction = h.offset.transform[13];
    let (hierarchy, parameters, pose) = fixture();
    let mut ik = SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()));
    h.offset
        .update(&mut h.mapped, &mut ik, &mut HostSkateboardSpawnMathNumeric);
    assert_eq!(h.mapped[13], correction + 4.0);
    assert_eq!(h.mapped[15], 7.0);
    for target in ik.targets() {
        assert_eq!(target[12..], [2.0, correction, 3.0, 7.0]);
    }
    assert!(!h.offset.full_just_set);
    assert!(!h.offset.vertical_just_set);
    assert_eq!(h.offset.vertical_count, 15.0);
}

#[test]
fn full_kernel_fnmsubs_preserves_fused_cancellation_before_velocity_commit() {
    let mut h = Harness::new();
    let mut owner = active(K::Manual);
    owner.y_velocity = 4097.0;
    owner.y_position = -16_785_408.0;
    h.attributes.set(S::MaximumVelocity, 1_000_000.0);
    h.attributes.set(S::DesiredCom, 0.0);
    h.attributes.set(S::ManualK1, 1.0);
    h.attributes.set(S::ManualK2, 4097.0);
    // 4097*4097 = 16785409 exactly before rounding. Native fused subtraction
    // leaves acceleration-1; defusing rounds the product to16785408 and gives0.
    // Stop at the spring debug read after YVel, before the minimum-position read.
    h.fail(9);
    assert_eq!(h.update(&mut owner), Err(Error::Cvar("injected")));
    assert_eq!(bits(owner.y_velocity()), 0x4580_07de);
    assert_ne!(bits(owner.y_velocity()), 0x4580_0800);
    assert_eq!(owner.y_position(), -16_785_408.0);
    assert!(!h.offset.vertical_just_set);
}

#[test]
fn full_kernel_fnmsubs_outer_negation_preserves_negative_zero_velocity() {
    let mut h = Harness::new();
    let mut owner = active(K::Manual);
    owner.y_velocity = -0.0;
    owner.y_position = 0.0;
    h.attributes.set(S::DesiredCom, 0.0);
    h.attributes.set(S::ManualK1, 0.0);
    h.attributes.set(S::ManualK2, 0.0);
    h.fail(9);
    assert_eq!(h.update(&mut owner), Err(Error::Cvar("injected")));
    // -(oldVel*K2 - negativeElastic) gives -0. Negating oldVel before the
    // FMA instead gives +0 here, which survives to the retained velocity.
    assert_eq!(bits(owner.y_velocity()), 0x8000_0000);
    assert_eq!(bits(owner.y_position()), 0);
}
