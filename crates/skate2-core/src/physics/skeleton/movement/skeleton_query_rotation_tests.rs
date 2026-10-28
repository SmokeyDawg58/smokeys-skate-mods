#![expect(
    clippy::float_cmp,
    reason = "fixed source results and retained sentinel values"
)]
use super::*;
use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;

#[derive(Debug, PartialEq)]
enum Event {
    Dot([f32; 4]),
    Reciprocal(f32),
    ReciprocalSqrt(f32),
}
#[derive(Default)]
struct Numeric {
    events: Vec<Event>,
    fail_at: Option<usize>,
    forced_reciprocal: Option<f32>,
}
impl Numeric {
    fn record(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail_at == Some(self.events.len()) {
            Err("numeric")
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.record(Event::Dot(a))?;
        Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.record(Event::Reciprocal(value))?;
        Ok(self.forced_reciprocal.unwrap_or(1.0 / value))
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.record(Event::ReciprocalSqrt(value))?;
        Ok(1.0 / value.sqrt())
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("QueryRotate has no integer conversion")
    }
}
fn sixty_degrees() -> AnimationMatrix {
    let sine = f32::from_bits(0x3f5d_b3d7);
    [
        0.5, sine, 0.0, 0.0, -sine, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 4.0, 6.0, 0.0,
    ]
}
fn sentinel() -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [101.0, 102.0, 103.0],
        w: Some(104.0),
    }
}
fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000_002,
        "{actual} vs {expected}"
    );
}

#[test]
fn atan_zero_and_negative_zero_still_execute_all_three_reciprocals() {
    for input in [0.0, -0.0] {
        let mut numeric = Numeric::default();
        assert_eq!(try_atan(input, &mut numeric).unwrap().to_bits(), 0);
        assert_eq!(
            numeric.events,
            [
                Event::Reciprocal(0.0),
                Event::Reciprocal(f32::from_bits(0x3fdd_b3d7)),
                Event::Reciprocal(f32::from_bits(0x4224_43e6))
            ]
        );
    }
}

#[test]
fn atan_rational_graph_matches_independent_angles_across_reduction_regions() {
    // The library atan is an independent test oracle only; production uses the
    // recovered rational graph. Values cover all native range-reduction arms.
    for value in [
        -100.0_f32, -2.0, -1.0, -0.3, -0.0001, 0.0001, 0.25, 0.3, 1.0, 2.0, 100.0,
    ] {
        let mut numeric = Numeric::default();
        close(try_atan(value, &mut numeric).unwrap(), value.atan());
        assert_eq!(numeric.events.len(), 3);
    }
}

#[test]
fn atan_frozen_host_words_discriminate_the_fused_reduction_from_sqrt_three_multiply() {
    // Frozen independent C fmaf/f32 scalar calculation of the raw graph, with
    // host 1/x seeds. These are host-graph words, not a Xenon runtime trace.
    // Replacing FMA(y,0x3f3b67af,y)-1 with y*0x3fddb3d7-1 gives
    // 0x3eed6338 here, one bit below the required recovered graph.
    let mut numeric = Numeric::default();
    assert_eq!(try_atan(0.5, &mut numeric).unwrap().to_bits(), 0x3eed_6339);
    assert!(
        matches!(numeric.events[2], Event::Reciprocal(value) if value.to_bits() == 0x4225_828c)
    );
}

#[test]
fn atan_extreme_clamps_and_nan_do_not_skip_estimate_calls() {
    for (value, expected) in [
        (f32::INFINITY, HALF_PI),
        (f32::NEG_INFINITY, -HALF_PI),
        (f32::MAX, HALF_PI),
        (-f32::MAX, -HALF_PI),
    ] {
        let mut numeric = Numeric::default();
        assert_eq!(
            try_atan(value, &mut numeric).unwrap().to_bits(),
            expected.to_bits()
        );
        assert_eq!(numeric.events.len(), 3);
    }
    let mut numeric = Numeric::default();
    assert!(try_atan(f32::NAN, &mut numeric).unwrap().is_nan());
    assert_eq!(numeric.events.len(), 3);
}

#[test]
fn atan_reciprocal_fallback_observes_first_refinement_not_second() {
    // 0*infinity makes the first refinement NaN; source retains original seed.
    let mut numeric = Numeric::default();
    assert_eq!(
        try_atan_reciprocal(0.0, &mut numeric).unwrap(),
        f32::INFINITY
    );
    // Here first refinement overflows to infinity (self-equal), and second
    // becomes NaN. Source selects that NaN, not the original finite seed.
    let mut numeric = Numeric {
        forced_reciprocal: Some(f32::MAX),
        ..Numeric::default()
    };
    assert!(try_atan_reciprocal(0.0, &mut numeric).unwrap().is_nan());
}

#[test]
fn atan_masked_calculations_preserve_each_numeric_failure_prefix() {
    for fail_at in 1..=3 {
        let mut numeric = Numeric {
            fail_at: Some(fail_at),
            ..Numeric::default()
        };
        assert_eq!(try_atan(0.0, &mut numeric), Err("numeric"));
        assert_eq!(numeric.events.len(), fail_at);
    }
}

#[test]
fn ordinary_query_returns_axis_angle_and_center_but_preserves_unwritten_axis_w() {
    let mut axis = sentinel();
    let mut angle = 99.0;
    let mut center = sentinel();
    let mut numeric = Numeric::default();
    try_query_rotate(
        &sixty_degrees(),
        &mut axis,
        &mut angle,
        &mut center,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(axis.xyz, [0.0, 0.0, f32::from_bits(0x3f7f_ffff)]);
    assert_eq!(axis.w, None);
    close(angle, std::f32::consts::FRAC_PI_3);
    // Independent axis-angle geometry, not a second source-graph call.
    close(center.xyz[0], 1.0 - 2.0 * 3.0_f32.sqrt());
    close(center.xyz[1], 2.0 + 3.0_f32.sqrt());
    close(center.xyz[2], 0.0);
    assert_eq!(center.w, None);
    assert_eq!(numeric.events.len(), 8);
    assert_eq!(numeric.events.last(), Some(&Event::Reciprocal(1.0)));
}

#[test]
fn ordinary_query_failure_points_preserve_axis_then_angle_then_center_stores() {
    for fail_at in 1..=8 {
        let mut axis = sentinel();
        let mut angle = 99.0;
        let mut center = sentinel();
        let mut numeric = Numeric {
            fail_at: Some(fail_at),
            ..Numeric::default()
        };
        assert_eq!(
            try_query_rotate(
                &sixty_degrees(),
                &mut axis,
                &mut angle,
                &mut center,
                &mut numeric
            ),
            Err("numeric")
        );
        if fail_at <= 3 {
            assert_eq!(axis, sentinel());
        } else {
            assert_eq!(axis.xyz, [0.0, 0.0, f32::from_bits(0x3f7f_ffff)]);
            assert_eq!(axis.w, None);
        }
        if fail_at <= 7 {
            assert_eq!(angle, 99.0);
        } else {
            close(angle, std::f32::consts::FRAC_PI_3);
        }
        assert_eq!(center, sentinel());
        assert_eq!(numeric.events.len(), fail_at);
    }
}

#[test]
fn identity_still_executes_unused_center_and_preserves_its_exceptional_result() {
    let mut axis = sentinel();
    let mut angle = 99.0;
    let mut center = sentinel();
    let mut numeric = Numeric::default();
    try_query_rotate(
        &FRESH_VOLUME_TRANSFORM,
        &mut axis,
        &mut angle,
        &mut center,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(axis, ZERO);
    assert_eq!(angle.to_bits(), 0);
    assert!(center.xyz.iter().all(|lane| lane.is_nan()));
    assert!(center.w.unwrap().is_nan());
    assert_eq!(numeric.events.len(), 7);
    assert_eq!(numeric.events.last(), Some(&Event::Reciprocal(0.0)));
}

#[test]
fn quarter_turn_overrides_nan_atan_ratio_only_after_all_atan_callbacks() {
    let matrix = [
        0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let mut axis = sentinel();
    let mut angle = 99.0;
    let mut center = sentinel();
    let mut numeric = Numeric::default();
    try_query_rotate(&matrix, &mut axis, &mut angle, &mut center, &mut numeric).unwrap();
    assert_eq!(axis.xyz, [0.0, 0.0, 1.0]);
    assert_eq!(angle.to_bits(), HALF_PI.to_bits());
    assert!(matches!(numeric.events[4], Event::Reciprocal(v) if v.is_nan()));
    assert_eq!(numeric.events.len(), 8);
}

#[test]
fn half_turn_degenerate_axis_replacement_follows_angle_store_and_recovers_w() {
    let mut matrix = FRESH_VOLUME_TRANSFORM;
    matrix[5] = -1.0;
    matrix[10] = -1.0;
    for fail_at in [7, 8] {
        let mut axis = sentinel();
        let mut angle = 99.0;
        let mut center = sentinel();
        let mut numeric = Numeric {
            fail_at: Some(fail_at),
            ..Numeric::default()
        };
        assert_eq!(
            try_query_rotate(&matrix, &mut axis, &mut angle, &mut center, &mut numeric),
            Err("numeric")
        );
        assert_eq!(axis, ZERO);
        assert_eq!(angle.to_bits(), PI.to_bits());
        assert_eq!(center, sentinel());
    }
    let mut axis = sentinel();
    let mut angle = 99.0;
    let mut center = sentinel();
    let mut numeric = Numeric::default();
    try_query_rotate(&matrix, &mut axis, &mut angle, &mut center, &mut numeric).unwrap();
    assert_eq!(
        axis,
        SkeletonRootVector {
            xyz: [1.0, 0.0, 0.0],
            w: Some(1.0)
        }
    );
    assert_eq!(numeric.events[6], Event::Dot([4.0, 0.0, 0.0, 4.0]));
    assert_eq!(numeric.events.len(), 9);
}

#[test]
fn degenerate_principal_is_squared_and_each_selected_branch_publishes_w_equal_x() {
    for (diagonal, expected) in [
        ([0.5, 0.25, 0.0], [2.25, 5.0, 9.0, 2.25]),
        ([0.0, 0.5, 0.25], [5.0, 2.25, 13.0, 5.0]),
        ([0.0, 0.25, 0.5], [9.0, 13.0, 2.25, 9.0]),
    ] {
        let matrix = [
            diagonal[0],
            2.0,
            4.0,
            99.0,
            3.0,
            diagonal[1],
            6.0,
            99.0,
            5.0,
            7.0,
            diagonal[2],
            99.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        let mut numeric = Numeric::default();
        let result = try_degenerate_axis(&matrix, &mut numeric).unwrap();
        assert_eq!(numeric.events[0], Event::Dot(expected));
        assert_eq!(result.w.unwrap().to_bits(), result.xyz[0].to_bits());
        let squared = (result.xyz[0] * result.xyz[0] + result.xyz[1] * result.xyz[1])
            + result.xyz[2] * result.xyz[2];
        close(squared, 1.0);
    }
}

#[test]
fn degenerate_strict_ties_choose_z_and_unordered_x_can_select_y() {
    let mut matrix = FRESH_VOLUME_TRANSFORM;
    matrix[0] = 0.5;
    matrix[5] = 0.5;
    matrix[10] = 0.5;
    let mut numeric = Numeric::default();
    try_degenerate_axis(&matrix, &mut numeric).unwrap();
    assert_eq!(numeric.events[0], Event::Dot([0.0, 0.0, 2.25, 0.0]));
    matrix[0] = f32::NAN;
    matrix[5] = 1.0;
    matrix[10] = 0.0;
    let mut numeric = Numeric::default();
    let result = try_degenerate_axis(&matrix, &mut numeric).unwrap();
    assert_eq!(numeric.events[0], Event::Dot([0.0, 4.0, 0.0, 0.0]));
    assert_eq!(result.xyz, [0.0, 1.0, 0.0]);
}
