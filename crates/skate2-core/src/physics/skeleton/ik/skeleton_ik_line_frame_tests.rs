use super::*;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Dot([u32; 4]),
    ReciprocalSqrt(u32),
}
#[derive(Default)]
struct Numeric {
    events: Vec<Event>,
    fail_at: Option<usize>,
    dot_results: BTreeMap<usize, f32>,
    forced_seed: Option<f32>,
}
impl Numeric {
    fn record(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail_at == Some(self.events.len()) {
            Err("refused")
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits));
        self.record(Event::Dot(a.map(f32::to_bits)))?;
        Ok(self
            .dot_results
            .get(&self.events.len())
            .copied()
            .unwrap_or((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]))
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.record(Event::ReciprocalSqrt(value.to_bits()))?;
        Ok(self.forced_seed.unwrap_or(1.0 / value.sqrt()))
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("clamp uses scalar division")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("line helpers have no conversion")
    }
}
fn v(value: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector::known(value)
}
fn sentinel() -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [11.0, 12.0, 13.0],
        w: None,
    }
}
fn sentinel_matrix() -> SkeletonRootMatrix {
    SkeletonRootMatrix::from_columns([sentinel(); 4])
}
fn known_bits(value: SkeletonRootVector) -> [u32; 4] {
    [value.xyz[0], value.xyz[1], value.xyz[2], value.w.unwrap()].map(f32::to_bits)
}
fn line() -> [SkeletonRootVector; 2] {
    [v([2.0, 3.0, 4.0, 2.0]), v([2.0, 5.0, 4.0, 6.0])]
}
fn normal() -> SkeletonRootVector {
    v([0.0, 0.0, 1.0, 8.0])
}

#[test]
fn negative_range_refuses_before_numeric_calls_and_does_not_write_output() {
    let mut output = sentinel();
    let mut numeric = Numeric::default();
    assert_eq!(
        try_clamp_vector_within_range(ZERO, ZERO, -1.0, &mut output, &mut numeric),
        Err(SkeletonIkClampError::NegativeRange {
            bits: (-1.0_f32).to_bits()
        })
    );
    assert_eq!(output, sentinel());
    assert!(numeric.events.is_empty());
}

#[test]
fn clamp_zero_and_range_equality_still_execute_both_services_and_preserve_original_w() {
    for (a, b, range) in [
        (sentinel(), sentinel(), -0.0),
        (
            SkeletonRootVector {
                xyz: [3.0, 4.0, 0.0],
                w: None,
            },
            ZERO,
            5.0,
        ),
    ] {
        let mut output = ZERO;
        let mut numeric = Numeric::default();
        try_clamp_vector_within_range(a, b, range, &mut output, &mut numeric).unwrap();
        assert_eq!(output, a);
        assert_eq!(numeric.events.len(), 2);
        assert!(matches!(numeric.events[0], Event::Dot(_)));
        assert!(matches!(numeric.events[1], Event::ReciprocalSqrt(_)));
    }
}

#[test]
fn clamp_scales_full_vector_about_b_with_actual_scalar_division() {
    let a = v([4.0, 6.0, 3.0, 10.0]);
    let b = v([1.0, 2.0, 3.0, 2.0]);
    let mut output = sentinel();
    try_clamp_vector_within_range(a, b, 2.5, &mut output, &mut Numeric::default()).unwrap();
    assert_eq!(
        known_bits(output),
        [2.5_f32, 4.0, 3.0, 6.0].map(f32::to_bits)
    );
    let a = SkeletonRootVector { w: None, ..a };
    try_clamp_vector_within_range(a, b, 2.5, &mut output, &mut Numeric::default()).unwrap();
    assert_eq!(
        output.xyz.map(f32::to_bits),
        [2.5_f32, 4.0, 3.0].map(f32::to_bits)
    );
    assert_eq!(output.w, None);
}

#[test]
fn clamp_frozen_fused_output_discriminates_separate_product_and_add() {
    // Independently evaluated with Python/ucrt fmaf, host 1/sqrt seeds and
    // f32 rounding. Splitting the final FMA gives X4007998c instead.
    let a = v([0xc005_1a86, 0xbf04_6461, 0x3fc7_f6c8, 0xbf6e_669e].map(f32::from_bits));
    let b = v([0x4031_a782, 0xbef7_d0f6, 0xbe61_b14e, 0x4026_6454].map(f32::from_bits));
    let mut output = ZERO;
    try_clamp_vector_within_range(a, b, 0.7, &mut output, &mut Numeric::default()).unwrap();
    assert_eq!(
        known_bits(output),
        [0x4007_998d, 0xbefa_1ccd, 0x3caa_b2f0, 0x4007_cf03]
    );
}

#[test]
fn nan_range_with_positive_length_produces_exceptional_output_but_zero_length_asserts() {
    let mut output = sentinel();
    let mut numeric = Numeric::default();
    try_clamp_vector_within_range(
        v([1.0, 0.0, 0.0, 1.0]),
        ZERO,
        f32::NAN,
        &mut output,
        &mut numeric,
    )
    .unwrap();
    assert!(output.xyz.iter().all(|x| x.is_nan()));
    assert!(output.w.unwrap().is_nan());
    assert_eq!(numeric.events.len(), 2);
    output = sentinel();
    numeric = Numeric::default();
    assert_eq!(
        try_clamp_vector_within_range(ZERO, ZERO, f32::NAN, &mut output, &mut numeric),
        Err(SkeletonIkClampError::NonpositiveLength { bits: 0 })
    );
    assert_eq!(output, sentinel());
    assert_eq!(numeric.events.len(), 2);
}

#[test]
fn clamp_numeric_refusals_and_unordered_length_leave_the_original_output() {
    for fail_at in 1..=2 {
        let mut numeric = Numeric {
            fail_at: Some(fail_at),
            ..Numeric::default()
        };
        let mut output = sentinel();
        assert_eq!(
            try_clamp_vector_within_range(
                v([3.0, 4.0, 0.0, 1.0]),
                ZERO,
                1.0,
                &mut output,
                &mut numeric
            ),
            Err(SkeletonIkClampError::Numeric("refused"))
        );
        assert_eq!(output, sentinel());
        assert_eq!(numeric.events.len(), fail_at);
    }
    let mut numeric = Numeric::default();
    numeric.dot_results.insert(1, f32::NAN);
    let mut output = sentinel();
    let error = try_clamp_vector_within_range(v([1.0; 4]), ZERO, 1.0, &mut output, &mut numeric)
        .unwrap_err();
    assert!(
        matches!(error,SkeletonIkClampError::NonpositiveLength{bits} if f32::from_bits(bits).is_nan())
    );
    assert_eq!(output, sentinel());
    assert_eq!(numeric.events.len(), 2);
}

#[test]
fn valid_frame_has_z_x_y_normalization_then_x_z_y_quality_and_retains_noncanonical_w() {
    let [a, b] = line();
    let mut numeric = Numeric::default();
    let mut output = sentinel_matrix();
    assert!(
        try_create_matrix_from_line_and_normal(a, b, normal(), &mut output, &mut numeric).unwrap()
    );
    assert_eq!(
        output.known_lanes().unwrap().map(f32::to_bits),
        [
            1.0_f32, -0.0, -0.0, -0.0, 0.0, 1.0, 0.0, 2.0, -0.0, -0.0, 1.0, -0.0, 2.0, 3.0, 4.0,
            2.0
        ]
        .map(f32::to_bits)
    );
    assert_eq!(
        numeric.events,
        vec![
            Event::Dot([-0.0_f32, -0.0, 4.0, -0.0].map(f32::to_bits)),
            Event::ReciprocalSqrt(16.0_f32.to_bits()),
            Event::Dot([2.0_f32, -0.0, -0.0, -0.0].map(f32::to_bits)),
            Event::ReciprocalSqrt(4.0_f32.to_bits()),
            Event::Dot([0.0_f32, 2.0, 0.0, 4.0].map(f32::to_bits)),
            Event::ReciprocalSqrt(4.0_f32.to_bits()),
            Event::Dot([1.0_f32, -0.0, -0.0, -0.0].map(f32::to_bits)),
            Event::Dot([-0.0_f32, -0.0, 1.0, -0.0].map(f32::to_bits)),
            Event::Dot([0.0_f32, 1.0, 0.0, 2.0].map(f32::to_bits))
        ]
    );
}

#[test]
fn every_frame_numeric_failure_precedes_any_caller_matrix_publication() {
    let [a, b] = line();
    for fail_at in 1..=9 {
        let mut numeric = Numeric {
            fail_at: Some(fail_at),
            ..Numeric::default()
        };
        let mut output = sentinel_matrix();
        assert_eq!(
            try_create_matrix_from_line_and_normal(a, b, normal(), &mut output, &mut numeric),
            Err("refused")
        );
        assert_eq!(output, sentinel_matrix());
        assert_eq!(numeric.events.len(), fail_at);
    }
}

#[test]
fn degenerate_frame_runs_all_quality_dots_then_publishes_full_identity() {
    let mut numeric = Numeric::default();
    let mut output = sentinel_matrix();
    assert!(
        !try_create_matrix_from_line_and_normal(
            sentinel(),
            sentinel(),
            normal(),
            &mut output,
            &mut numeric
        )
        .unwrap()
    );
    assert_eq!(numeric.events.len(), 6);
    assert!(
        numeric
            .events
            .iter()
            .all(|event| matches!(event, Event::Dot(_)))
    );
    assert_eq!(
        output.known_lanes().unwrap().map(f32::to_bits),
        IDENTITY.map(f32::to_bits)
    );
}

#[test]
fn quality_is_strict_at_the_original_literal_and_unordered_fails_after_all_dots() {
    let [a, b] = line();
    for (quality, success) in [
        (f32::from_bits(0x3780_0000), false),
        (f32::from_bits(0x3780_0001), true),
        (f32::NAN, false),
    ] {
        let mut numeric = Numeric::default();
        numeric.dot_results.insert(8, quality);
        let mut output = sentinel_matrix();
        assert_eq!(
            try_create_matrix_from_line_and_normal(a, b, normal(), &mut output, &mut numeric)
                .unwrap(),
            success
        );
        assert_eq!(numeric.events.len(), 9);
        if !success {
            assert_eq!(
                output.known_lanes().unwrap().map(f32::to_bits),
                IDENTITY.map(f32::to_bits)
            );
        }
    }
}

#[test]
fn normalization_fallback_recovers_zero_w_but_zero_seed_on_positive_input_keeps_unknown_w() {
    let mut numeric = Numeric::default();
    let value = normalize_safe(
        SkeletonRootVector {
            xyz: [0.0; 3],
            w: None,
        },
        &mut numeric,
    )
    .unwrap();
    assert_eq!(known_bits(value), [0; 4]);
    assert_eq!(numeric.events.len(), 1);
    let mut numeric = Numeric {
        forced_seed: Some(0.0),
        ..Numeric::default()
    };
    let value = normalize_safe(
        SkeletonRootVector {
            xyz: [1.0, 0.0, 0.0],
            w: None,
        },
        &mut numeric,
    )
    .unwrap();
    assert_eq!(value.xyz.map(f32::to_bits), [0; 3]);
    assert_eq!(value.w, None);
    assert_eq!(numeric.events.len(), 2);
}

#[test]
fn mapping_first_false_skips_second_frame_and_second_false_publishes_identity() {
    for first_false in [true, false] {
        let old_line = if first_false { [ZERO; 2] } else { line() };
        let new_line = if first_false { line() } else { [ZERO; 2] };
        let mut numeric = Numeric::default();
        let mut output = sentinel_matrix();
        assert!(
            !try_calculate_matrix_from_line(
                old_line,
                normal(),
                new_line,
                normal(),
                &mut output,
                &mut numeric
            )
            .unwrap()
        );
        assert_eq!(numeric.events.len(), if first_false { 6 } else { 15 });
        assert_eq!(
            output.known_lanes().unwrap().map(f32::to_bits),
            IDENTITY.map(f32::to_bits)
        );
    }
}

#[test]
fn mapping_second_frame_refusal_keeps_original_output_despite_completed_first_local_frame() {
    for fail_at in 10..=18 {
        let mut numeric = Numeric {
            fail_at: Some(fail_at),
            ..Numeric::default()
        };
        let mut output = sentinel_matrix();
        assert_eq!(
            try_calculate_matrix_from_line(
                line(),
                normal(),
                line(),
                normal(),
                &mut output,
                &mut numeric
            ),
            Err("refused")
        );
        assert_eq!(output, sentinel_matrix());
        assert_eq!(numeric.events.len(), fail_at);
    }
}

#[test]
fn mapping_inverse_derives_old_w_while_new_frame_controls_known_full_output() {
    let old = [
        SkeletonRootVector {
            xyz: [1.0, 2.0, 3.0],
            w: None,
        },
        SkeletonRootVector {
            xyz: [1.0, 4.0, 3.0],
            w: None,
        },
    ];
    let new = [v([7.0, 8.0, 9.0, 5.0]), v([5.0, 8.0, 9.0, 9.0])];
    let n = v([0.0, 0.0, 1.0, 0.0]);
    let mut output = sentinel_matrix();
    assert!(
        try_calculate_matrix_from_line(old, n, new, n, &mut output, &mut Numeric::default())
            .unwrap()
    );
    assert_eq!(
        output.known_lanes().unwrap().map(f32::to_bits),
        [
            -0.0_f32, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 1.0, 0.0, 9.0, 7.0, 6.0, 1.0
        ]
        .map(f32::to_bits)
    );
    let new = new.map(|point| SkeletonRootVector { w: None, ..point });
    assert!(
        try_calculate_matrix_from_line(old, n, new, n, &mut output, &mut Numeric::default())
            .unwrap()
    );
    assert!(output.columns().iter().all(|column| column.w.is_none()));
    assert_eq!(
        output.columns()[3].xyz.map(f32::to_bits),
        [9.0_f32, 7.0, 6.0].map(f32::to_bits)
    );
}

#[test]
fn oblique_mapping_matches_independently_frozen_full_lane_source_graph() {
    // Python/ucrt fmaf source-form oracle with host dot/rsqrt seeds; no
    // production frame/inverse/compose helper was used to produce these words.
    let old = [v([1.0, 2.0, 3.0, 0.75]), v([2.0, 4.0, 6.0, 1.25])];
    let new = [v([-2.0, 1.0, 5.0, 2.0]), v([0.0, 4.0, 7.0, 3.0])];
    let mut output = sentinel_matrix();
    assert!(
        try_calculate_matrix_from_line(
            old,
            v([0.25, -0.5, 1.0, 0.625]),
            new,
            v([0.75, 0.25, -0.5, 1.5]),
            &mut output,
            &mut Numeric::default()
        )
        .unwrap()
    );
    assert_eq!(
        output.known_lanes().unwrap().map(f32::to_bits),
        [
            0xbeaa_e72f,
            0x3f5a_d2f3,
            0xbecb_7881,
            0x3d84_c08b,
            0xbe8d_e142,
            0x3ea0_c182,
            0x3f68_78a7,
            0x3e04_c08b,
            0x3f66_a795,
            0x3ed3_9443,
            0x3e07_3bc7,
            0x3e47_20d0,
            0xc074_287a,
            0xbfdc_796e,
            0x404b_d787,
            0x3f8b_d786
        ]
    );
}
