//! Explicit source-order and host-numeric discriminators, not Xenon traces.
#![expect(
    clippy::float_cmp,
    reason = "exact copied values and representable arithmetic discriminate mutation scope"
)]
use super::*;
use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;

#[derive(Debug, PartialEq)]
enum Event {
    Dot([u32; 4], [u32; 4]),
    Axis(SkateboardWheelInertiaAxis, u32),
    Debug(u32),
}
#[derive(Default)]
struct Numeric {
    events: Vec<Event>,
    dot: Option<f32>,
    debug_seed: Option<f32>,
}
impl SkateboardWheelMassNumeric for Numeric {
    fn reciprocal_estimate(&mut self, axis: SkateboardWheelInertiaAxis, value: f32) -> f32 {
        self.events.push(Event::Axis(axis, value.to_bits()));
        value.recip()
    }
}
impl SkeletonBodyMassNumeric for Numeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> f32 {
        self.events
            .push(Event::Dot(left.map(f32::to_bits), right.map(f32::to_bits)));
        self.dot
            .unwrap_or_else(|| HostSkeletonBodyMassNumeric.sum_three_products(left, right))
    }
    fn debug_factor_reciprocal_estimate(&mut self, value: f32) -> f32 {
        self.events.push(Event::Debug(value.to_bits()));
        self.debug_seed.unwrap_or_else(|| value.recip())
    }
}

#[test]
fn constructor_preserves_uninitialized_inverse_storage_as_absence() {
    let state = BodyMassState::native_default();
    assert_eq!(state.inertia, SimulationInertia::native_default());
    assert_eq!(state.packed_inverse_tensor(), [1.0; 4]);
    assert!(!state.has_inverse_body_transform);
    assert_eq!(state.inverse_body_transform_storage, None);
}

#[test]
fn principal_uses_dot_before_yxz_seeds_and_preserves_unrelated_fields() {
    let mut state = BodyMassState::native_default();
    state.inertia.max_velocity = 91.0;
    state.inertia.max_omega = 73.0;
    state.inertia.linear_drag = -0.0;
    state.inertia.angular_drag = 0.25;
    let mut numeric = Numeric::default();
    state.apply_principal(
        [2.0, 4.0, 8.0],
        99.0,
        FRESH_VOLUME_TRANSFORM,
        2.0,
        &mut numeric,
    );
    assert_eq!(
        numeric.events,
        [
            Event::Dot([0; 4], [0; 4]),
            Event::Axis(SkateboardWheelInertiaAxis::Y, 8.0_f32.to_bits()),
            Event::Axis(SkateboardWheelInertiaAxis::X, 4.0_f32.to_bits()),
            Event::Axis(SkateboardWheelInertiaAxis::Z, 16.0_f32.to_bits()),
        ]
    );
    assert_eq!(state.packed_inverse_tensor(), [0.25, 0.125, 0.0625, 0.25]);
    assert_eq!(state.inertia.inverse_mass, 0.5);
    assert_eq!(state.inertia.spherical, 16.0);
    assert_eq!(state.inertia.max_velocity, 91.0);
    assert_eq!(state.inertia.max_omega, 73.0);
    assert_eq!(state.inertia.linear_drag.to_bits(), 0x8000_0000);
    assert_eq!(state.inertia.angular_drag, 0.25);
    assert!(!state.has_inverse_body_transform);
}

#[test]
fn disabling_inverse_keeps_prior_known_storage_bitwise() {
    let mut state = BodyMassState::native_default();
    let storage =
        std::array::from_fn(|lane| f32::from_bits(0x7fc1_0000 + u32::try_from(lane).unwrap()));
    state.inverse_body_transform_storage = Some(storage);
    state.has_inverse_body_transform = true;
    state.apply_principal(
        [1.0; 3],
        1.0,
        FRESH_VOLUME_TRANSFORM,
        1.0,
        &mut Numeric::default(),
    );
    assert!(!state.has_inverse_body_transform);
    assert_eq!(
        state
            .inverse_body_transform_storage
            .unwrap()
            .map(f32::to_bits),
        storage.map(f32::to_bits)
    );
}

#[test]
fn frame_gate_equality_unordered_and_axis_strict_threshold_are_distinct() {
    let threshold = 3.0 * f32::from_bits(0x3586_37be);
    for dot in [threshold, f32::NAN] {
        let mut state = BodyMassState::native_default();
        state.apply_principal(
            [1.0; 3],
            1.0,
            FRESH_VOLUME_TRANSFORM,
            1.0,
            &mut Numeric {
                dot: Some(dot),
                ..Numeric::default()
            },
        );
        assert!(state.has_inverse_body_transform);
    }
    for (lane, expected) in [
        (0x3a83_126f, false),
        (0x3a83_1270, true),
        (0x7fc1_2345, false),
    ] {
        let mut frame = FRESH_VOLUME_TRANSFORM;
        frame[1] = f32::from_bits(lane);
        let mut state = BodyMassState::native_default();
        state.apply_principal([1.0; 3], 1.0, frame, 1.0, &mut Numeric::default());
        assert_eq!(state.has_inverse_body_transform, expected);
    }
}

#[test]
fn frame_gate_ignores_all_w_lanes_and_uses_unscaled_diagonal_threshold() {
    let mut frame = FRESH_VOLUME_TRANSFORM;
    for lane in [3, 7, 11, 15] {
        frame[lane] = f32::NAN;
    }
    let mut state = BodyMassState::native_default();
    state.apply_principal([1.0; 3], 1.0, frame, 1.0, &mut HostSkeletonBodyMassNumeric);
    assert!(!state.has_inverse_body_transform);
    let mut numeric = Numeric {
        dot: Some(0.001),
        ..Numeric::default()
    };
    state.apply_principal(
        [1.0; 3],
        1.0,
        FRESH_VOLUME_TRANSFORM,
        1_000_000.0,
        &mut numeric,
    );
    assert!(
        state.has_inverse_body_transform,
        "mass must not inflate the principal-frame threshold"
    );
}

#[test]
fn selected_inverse_is_rigid_transpose_not_normalized_or_general_inverse() {
    let mut frame = FRESH_VOLUME_TRANSFORM;
    frame[0] = 2.0;
    frame[5] = 3.0;
    frame[10] = 4.0;
    frame[12..16].copy_from_slice(&[1.0, 2.0, 3.0, 99.0]);
    let mut state = BodyMassState::native_default();
    state.apply_principal([1.0; 3], 1.0, frame, 1.0, &mut HostSkeletonBodyMassNumeric);
    let inverse = state.inverse_body_transform_storage.unwrap();
    assert_eq!([inverse[0], inverse[5], inverse[10]], [2.0, 3.0, 4.0]);
    assert_eq!(&inverse[12..15], &[-2.0, -6.0, -12.0]);
    assert_eq!(
        [inverse[3], inverse[7], inverse[11], inverse[15]].map(f32::to_bits),
        [0, 0, 0, 0x8000_0000]
    );
    frame[12..16].fill(0.0);
    state.apply_principal([1.0; 3], 1.0, frame, 1.0, &mut HostSkeletonBodyMassNumeric);
    assert_eq!(
        state.inverse_body_transform_storage.unwrap()[12..16]
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        [0; 4]
    );
}

#[test]
fn principal_mass_fallback_and_nan_are_not_sanitized() {
    for mass in [0.0, -1.0, f32::MIN_POSITIVE * 0.5] {
        let mut state = BodyMassState::native_default();
        state.apply_principal(
            [1.0; 3],
            8.0,
            FRESH_VOLUME_TRANSFORM,
            mass,
            &mut HostSkeletonBodyMassNumeric,
        );
        assert_eq!(state.inertia.inverse_mass, 0.125);
    }
    let mut state = BodyMassState::native_default();
    state.apply_principal(
        [1.0; 3],
        8.0,
        FRESH_VOLUME_TRANSFORM,
        f32::NAN,
        &mut HostSkeletonBodyMassNumeric,
    );
    assert!(state.inertia.inverse_mass.is_nan());
    assert!(
        state
            .packed_inverse_tensor()
            .iter()
            .all(|value| value.is_nan())
    );
    state.apply_principal(
        [f32::NAN, 1.0, 1.0],
        8.0,
        FRESH_VOLUME_TRANSFORM,
        1.0,
        &mut HostSkeletonBodyMassNumeric,
    );
    assert!(
        state.has_inverse_body_transform,
        "unordered threshold follows source NOT_LT"
    );
}

#[test]
fn debug_one_two_refinements_scale_existing_four_lanes_without_mass_or_frame_writes() {
    let mut state = BodyMassState::native_default();
    state.inertia.inverse_tensor = [2.0, 4.0, 8.0];
    state.inverse_tensor_w = 16.0;
    state.inertia.inverse_mass = 7.0;
    state.inverse_body_transform_storage = Some(FRESH_VOLUME_TRANSFORM);
    state.has_inverse_body_transform = true;
    let before = state.clone();
    let mut numeric = Numeric {
        debug_seed: Some(0.25),
        ..Numeric::default()
    };
    state.debug_multiply(2.0, &mut numeric);
    assert_eq!(numeric.events, [Event::Debug(2.0_f32.to_bits())]);
    assert_eq!(state.packed_inverse_tensor(), [0.9375, 1.875, 3.75, 7.5]);
    let mut expected = before;
    expected.inertia.inverse_tensor = [0.9375, 1.875, 3.75];
    expected.inverse_tensor_w = 7.5;
    expected.inertia.spherical = 1.0 / 0.9375;
    assert_eq!(state, expected);
}

#[test]
fn debug_two_uses_reciprocal_times_ordered_sum_and_zeroes_only_packed_w() {
    let mut state = BodyMassState::native_default();
    state.inertia.inverse_tensor = [2.0, 4.0, 8.0];
    state.inverse_tensor_w = f32::NAN;
    state.inertia.inverse_mass = 11.0;
    state.debug_multiply_isotropic(2.0);
    assert_eq!(
        state.packed_inverse_tensor().map(f32::to_bits),
        [0x4015_5556, 0x4015_5556, 0x4015_5556, 0]
    );
    assert_ne!(
        state.inertia.inverse_tensor[0].to_bits(),
        (14.0_f32 / 6.0).to_bits()
    );
    assert_eq!(state.inertia.inverse_mass, 11.0);
    assert_eq!(state.inertia.spherical, 1.0 / f32::from_bits(0x4015_5556));
}

#[test]
fn spherical_selection_keeps_later_signed_zero_and_nan_lane_behavior() {
    let mut state = BodyMassState::native_default();
    state.inertia.inverse_tensor = [0.0, -0.0, 1.0];
    state.inverse_tensor_w = 23.0;
    state.debug_multiply(1.0, &mut HostSkeletonBodyMassNumeric);
    assert_eq!(state.inertia.spherical, f32::NEG_INFINITY);
    state.inertia.inverse_tensor = [f32::NAN, 2.0, 4.0];
    state.debug_multiply(1.0, &mut HostSkeletonBodyMassNumeric);
    assert_eq!(state.inertia.spherical, 0.5);
    state.inertia.inverse_tensor = [2.0, 4.0, f32::NAN];
    state.debug_multiply(1.0, &mut HostSkeletonBodyMassNumeric);
    assert!(state.inertia.spherical.is_nan());
    state.debug_multiply_isotropic(0.0);
    assert!(
        state
            .inertia
            .inverse_tensor
            .iter()
            .all(|value| value.is_nan())
    );
    assert_eq!(state.inverse_tensor_w.to_bits(), 0);
}
