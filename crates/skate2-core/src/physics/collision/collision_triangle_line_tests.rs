#![expect(
    clippy::float_cmp,
    reason = "finite binary-exact fixtures check source output equality"
)]

use super::*;

#[derive(Debug)]
enum Call {
    Dot([f32; 4], [f32; 4]),
    Root(f32),
}

#[derive(Default)]
struct Numeric {
    calls: Vec<Call>,
    fail: Option<usize>,
    overrides: Vec<(usize, f32)>,
}

impl Numeric {
    fn finish(&self, value: f32) -> Result<f32, usize> {
        let index = self.calls.len() - 1;
        if self.fail == Some(index) {
            return Err(index);
        }
        Ok(self
            .overrides
            .iter()
            .find_map(|&(at, replacement)| (at == index).then_some(replacement))
            .unwrap_or(value))
    }
}

impl ReckoningNumeric for Numeric {
    type Error = usize;

    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls.push(Call::Dot(a, b));
        self.finish((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }

    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push(Call::Root(value));
        self.finish(1.0 / value.sqrt())
    }

    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("source thin path uses scalar division, never a reciprocal estimate")
    }

    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("source triangle line has no integer conversion")
    }
}

fn vector(value: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector::known(value)
}

fn input() -> CollisionTriangleLineInput {
    CollisionTriangleLineInput {
        start: vector([0.5, 0.75, 2.0, 7.0]),
        delta: vector([0.0, 0.0, -4.0, 6.0]),
        vertices: [
            vector([0.0, 0.0, 0.0, 1.0]),
            vector([2.0, 0.0, 0.0, 1.0]),
            vector([0.0, 3.0, 0.0, 1.0]),
        ],
    }
}

fn sentinel() -> CollisionTriangleLineOutput {
    CollisionTriangleLineOutput {
        position: Some(vector([91.0, 92.0, 93.0, 94.0])),
        normal: Some(vector([81.0, 82.0, 83.0, 84.0])),
        volume_parameter: Some([71.0, 72.0, 73.0, 74.0]),
        line_parameter: Some(61.0),
    }
}

#[test]
fn successful_wrapper_preserves_source_call_order_and_full_output() {
    let mut numeric = Numeric::default();
    let mut output = CollisionTriangleLineOutput::default();
    assert_eq!(
        try_triangle_line(&input(), 0.0, 0.0, &mut output, &mut numeric),
        Ok(true)
    );
    assert_eq!(output.position, Some(vector([0.5, 0.75, 0.0, 10.0])));
    assert_eq!(output.normal, Some(vector([0.0, 0.0, 1.0, 0.0])));
    assert_eq!(output.volume_parameter, Some([0.25, 0.25, 0.0, 0.0]));
    assert_eq!(output.line_parameter, Some(0.5));
    assert_eq!(numeric.calls.len(), 6);
    assert!(
        numeric.calls[..5]
            .iter()
            .all(|call| matches!(call, Call::Dot(..)))
    );
    assert!(matches!(numeric.calls[5], Call::Root(36.0)));
    let Call::Dot(a, b) = numeric.calls[0] else {
        panic!()
    };
    assert_eq!(a[..3], [2.0, 0.0, 0.0]);
    assert_eq!(b[..3], [12.0, 0.0, 0.0]);
}

#[test]
fn thin_success_never_touches_normal_or_calls_wrapper_services() {
    let mut output = sentinel();
    let old_normal = output.normal;
    let mut numeric = Numeric::default();
    assert_eq!(
        try_thin_triangle_line(&input(), &mut output, &mut numeric),
        Ok(true)
    );
    assert_eq!(output.normal, old_normal);
    assert_eq!(numeric.calls.len(), 4);
}

#[test]
fn reversed_winding_is_an_early_miss_without_output_writes() {
    let mut input = input();
    input.vertices.swap(1, 2);
    let mut output = sentinel();
    let mut numeric = Numeric::default();
    assert_eq!(
        try_triangle_line(&input, 0.0, 0.0, &mut output, &mut numeric),
        Ok(false)
    );
    assert_eq!(output, sentinel());
    assert_eq!(numeric.calls.len(), 1);
}

#[test]
fn determinant_threshold_rejects_tie_and_unordered_but_accepts_next_bit() {
    for value in [0.0, -0.0, -1.0, DETERMINANT_MINIMUM, f32::NAN] {
        let mut numeric = Numeric {
            overrides: vec![(0, value)],
            ..Numeric::default()
        };
        let mut output = sentinel();
        assert_eq!(
            try_thin_triangle_line(&input(), &mut output, &mut numeric),
            Ok(false)
        );
        assert_eq!(output, sentinel());
        assert_eq!(numeric.calls.len(), 1);
    }
    let determinant = f32::from_bits(DETERMINANT_MINIMUM.to_bits() + 1);
    let mut numeric = Numeric {
        overrides: vec![
            (0, determinant),
            (1, determinant * 0.25),
            (2, determinant * 0.25),
            (3, determinant * 0.5),
        ],
        ..Numeric::default()
    };
    assert_eq!(
        try_thin_triangle_line(&input(), &mut sentinel(), &mut numeric),
        Ok(true)
    );
    assert_eq!(numeric.calls.len(), 4);
}

#[test]
fn barycentric_rejections_leave_every_field_untouched() {
    for (at, value, expected_calls) in [(1, -1.0, 2), (1, 25.0, 2), (2, -1.0, 3), (2, 19.0, 3)] {
        let mut numeric = Numeric {
            overrides: vec![(at, value)],
            ..Numeric::default()
        };
        let mut output = sentinel();
        assert_eq!(
            try_thin_triangle_line(&input(), &mut output, &mut numeric),
            Ok(false)
        );
        assert_eq!(output, sentinel());
        assert_eq!(numeric.calls.len(), expected_calls);
    }
}

#[test]
fn tolerance_accepts_exact_u_and_v_sum_boundaries() {
    let lower = 24.0 * f32::from_bits(0xb727_c5ac);
    let upper = 24.0 - lower;
    for (u, v) in [(lower, 6.0), (upper, 0.0), (6.0, lower), (0.0, upper)] {
        let mut numeric = Numeric {
            overrides: vec![(1, u), (2, v)],
            ..Numeric::default()
        };
        let mut output = CollisionTriangleLineOutput::default();
        assert_eq!(
            try_thin_triangle_line(&input(), &mut output, &mut numeric),
            Ok(true)
        );
        let coordinates = output.volume_parameter.unwrap();
        assert_eq!(coordinates[0].to_bits(), (u * (1.0_f32 / 24.0)).to_bits());
        assert_eq!(coordinates[1].to_bits(), (v * (1.0_f32 / 24.0)).to_bits());
    }
}

#[test]
fn late_segment_misses_store_unscaled_numerator_only() {
    for start_z in [-1.0, 5.0] {
        let mut input = input();
        input.start.xyz[2] = start_z;
        let mut numeric = Numeric::default();
        let mut output = sentinel();
        assert_eq!(
            try_triangle_line(&input, 0.0, 0.0, &mut output, &mut numeric),
            Ok(false)
        );
        let mut expected = sentinel();
        expected.line_parameter = Some(start_z * 6.0);
        assert_eq!(output, expected);
        assert_eq!(numeric.calls.len(), 4);
    }
}

#[test]
fn endpoints_and_tolerated_outside_parameters_are_not_clamped_by_thin() {
    let lower = 24.0 * f32::from_bits(0xb727_c5ac);
    let upper = 24.0 - lower;
    for numerator in [0.0, 24.0, lower, upper] {
        let mut numeric = Numeric {
            overrides: vec![(3, numerator)],
            ..Numeric::default()
        };
        let mut output = CollisionTriangleLineOutput::default();
        assert_eq!(
            try_thin_triangle_line(&input(), &mut output, &mut numeric),
            Ok(true)
        );
        let parameter = numerator * (1.0_f32 / 24.0);
        assert_eq!(
            output.line_parameter.unwrap().to_bits(),
            parameter.to_bits()
        );
        assert_eq!(
            output.position.unwrap().xyz[2].to_bits(),
            (-4.0_f32).mul_add(parameter, 2.0).to_bits()
        );
    }
}

#[test]
fn unordered_later_numerators_pass_ordered_rejection_gates() {
    for index in [1, 2, 3] {
        let mut numeric = Numeric {
            overrides: vec![(index, f32::NAN)],
            ..Numeric::default()
        };
        let mut output = CollisionTriangleLineOutput::default();
        assert_eq!(
            try_triangle_line(&input(), 0.0, 0.0, &mut output, &mut numeric),
            Ok(true)
        );
        assert_eq!(numeric.calls.len(), 6);
        assert_eq!(output.normal.unwrap().xyz, [0.0, 0.0, 1.0]);
        if index == 3 {
            assert!(output.line_parameter.unwrap().is_nan());
            assert!(output.position.unwrap().xyz.into_iter().all(f32::is_nan));
        } else {
            assert!(output.volume_parameter.unwrap()[index - 1].is_nan());
            assert_eq!(output.line_parameter, Some(0.5));
        }
    }
}

#[test]
fn numeric_failures_preserve_the_exact_thin_or_raw_normal_prefix() {
    for failure in 0..6 {
        let mut numeric = Numeric {
            fail: Some(failure),
            ..Numeric::default()
        };
        let mut output = sentinel();
        assert_eq!(
            try_triangle_line(&input(), 0.0, 0.0, &mut output, &mut numeric),
            Err(CollisionTriangleLineError::Numeric(failure))
        );
        assert_eq!(numeric.calls.len(), failure + 1);
        if failure < 4 {
            assert_eq!(output, sentinel());
        } else {
            assert_eq!(output.position, Some(vector([0.5, 0.75, 0.0, 10.0])));
            assert_eq!(output.line_parameter, Some(0.5));
            assert_eq!(output.volume_parameter, Some([0.25, 0.25, 0.0, 0.0]));
            assert_eq!(output.normal, Some(vector([0.0, 0.0, 6.0, 0.0])));
        }
    }
}

#[test]
fn nonzero_and_unordered_fatness_enter_wrapper_normal_before_fat_helper() {
    for (line, triangle) in [
        (0.25, 0.0),
        (0.0, -1.0),
        (f32::NAN, 0.0),
        (f32::INFINITY, f32::NEG_INFINITY),
    ] {
        let mut numeric = Numeric {
            fail: Some(0),
            ..Numeric::default()
        };
        let mut output = sentinel();
        let error =
            try_triangle_line(&input(), line, triangle, &mut output, &mut numeric).unwrap_err();
        assert_eq!(error, CollisionTriangleLineError::Numeric(0));
        let mut expected = sentinel();
        expected.normal = Some(vector([0.0, 0.0, 6.0, 0.0]));
        assert_eq!(output, expected);
        assert_eq!(numeric.calls.len(), 1);
    }
}

#[test]
fn signed_zero_and_cancelling_nonzero_fatness_use_thin_arm() {
    for (line, triangle) in [(0.0, -0.0), (-0.0, -0.0), (2.0, -2.0)] {
        let mut numeric = Numeric::default();
        assert_eq!(
            try_triangle_line(&input(), line, triangle, &mut sentinel(), &mut numeric),
            Ok(true)
        );
        assert_eq!(numeric.calls.len(), 6);
    }
}

#[test]
fn unknown_vertex_w_preserves_known_position_but_unknown_normal_w() {
    let mut input = input();
    input.vertices[1].w = None;
    let mut numeric = Numeric::default();
    let mut output = CollisionTriangleLineOutput::default();
    assert_eq!(
        try_triangle_line(&input, 0.0, 0.0, &mut output, &mut numeric),
        Ok(true)
    );
    assert_eq!(output.position.unwrap().w, Some(10.0));
    assert_eq!(output.normal.unwrap().w, None);
    assert_eq!(output.normal.unwrap().xyz, [0.0, 0.0, 1.0]);
    assert_eq!(numeric.calls.len(), 6);
}

#[test]
fn unknown_segment_w_never_blocks_xyz_or_fabricates_position_w() {
    for unknown_start in [true, false] {
        let mut input = input();
        if unknown_start {
            input.start.w = None;
        } else {
            input.delta.w = None;
        }
        let mut output = CollisionTriangleLineOutput::default();
        assert_eq!(
            try_triangle_line(&input, 0.0, 0.0, &mut output, &mut Numeric::default()),
            Ok(true)
        );
        assert_eq!(output.position.unwrap().xyz, [0.5, 0.75, 0.0]);
        assert_eq!(output.position.unwrap().w, None);
        assert_eq!(output.normal.unwrap().w, Some(0.0));
    }
}

#[test]
fn full_w_frozen_fma_position_and_cross_residual_are_not_canonicalized() {
    // Independent ucrtbase.fmaf source-form snapshot: 4097*(1/3)+rounded
    // negative product gives b82aa000, while defusing gives positive zero.
    // Normal cross W is rounded(4097^2)-exact(4097^2) = -1, then /6.
    let mut input = input();
    input.start = vector([0.5, 0.75, 1.0, f32::from_bits(0xc4aa_b556)]);
    input.delta = vector([0.0, 0.0, -3.0, 4097.0]);
    input.vertices[1].w = Some(4098.0);
    input.vertices[2].w = Some(4098.0);
    let mut output = CollisionTriangleLineOutput::default();
    assert_eq!(
        try_triangle_line(&input, 0.0, 0.0, &mut output, &mut Numeric::default()),
        Ok(true)
    );
    assert_eq!(output.line_parameter.unwrap().to_bits(), 0x3eaa_aaab);
    assert_eq!(output.position.unwrap().w.unwrap().to_bits(), 0xb82a_a000);
    assert_eq!(output.normal.unwrap().w.unwrap().to_bits(), 0xbe2a_aaab);
}

#[test]
fn wrapper_xyz_cross_has_frozen_fused_subtraction_discriminator() {
    let mut input = input();
    input.vertices[1] = vector([4096.0, 4097.0, 0.0, 1.0]);
    input.vertices[2] = vector([4097.0, 4098.0, 0.0, 1.0]);
    // Controlled dot primitive admits this ill-conditioned triangle so the
    // wrapper's independent cross can be observed before its next dot service.
    let mut numeric = Numeric {
        fail: Some(4),
        overrides: vec![(0, 1.0), (1, 0.25), (2, 0.25), (3, 0.5)],
        ..Numeric::default()
    };
    let mut output = CollisionTriangleLineOutput::default();
    assert_eq!(
        try_triangle_line(&input, 0.0, 0.0, &mut output, &mut numeric),
        Err(CollisionTriangleLineError::Numeric(4))
    );
    // 4096*4098 - 4097*4097 is -1 with the source FMA, zero defused.
    // Native VNMSUB negates the rounded zero residual in the other two lanes.
    assert_eq!(
        output.normal.unwrap().xyz.map(f32::to_bits),
        [0x8000_0000, 0x8000_0000, 0xbf80_0000]
    );
}

#[test]
fn zero_squared_normal_still_calls_root_and_runs_unsafe_refinements() {
    let mut numeric = Numeric {
        overrides: vec![(4, 0.0)],
        ..Numeric::default()
    };
    let mut output = CollisionTriangleLineOutput::default();
    assert_eq!(
        try_triangle_line(&input(), 0.0, 0.0, &mut output, &mut numeric),
        Ok(true)
    );
    assert!(matches!(numeric.calls[5], Call::Root(0.0)));
    assert!(output.normal.unwrap().xyz.into_iter().all(f32::is_nan));
    assert!(output.normal.unwrap().w.unwrap().is_nan());
}

#[test]
fn exceptional_known_w_is_arithmetic_data_not_a_preflight_refusal() {
    let mut input = input();
    input.start.w = Some(f32::NAN);
    input.vertices[1].w = Some(f32::INFINITY);
    let mut output = CollisionTriangleLineOutput::default();
    assert_eq!(
        try_triangle_line(&input, 0.0, 0.0, &mut output, &mut Numeric::default()),
        Ok(true)
    );
    assert_eq!(output.position.unwrap().xyz, [0.5, 0.75, 0.0]);
    assert!(output.position.unwrap().w.unwrap().is_nan());
    assert!(output.normal.unwrap().w.unwrap().is_nan());
    assert_eq!(output.line_parameter, Some(0.5));
}
