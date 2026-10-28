#![expect(
    clippy::float_cmp,
    reason = "source branch fixtures use exact binary values"
)]
use super::super::{CollisionTriangleLineError, try_triangle_line};
use super::*;

#[derive(Default)]
struct Numeric {
    calls: usize,
    fail: Option<usize>,
    replace: Option<(usize, f32)>,
    scripted: Vec<f32>,
}
impl Numeric {
    fn value(&mut self, value: f32) -> Result<f32, usize> {
        let at = self.calls;
        self.calls += 1;
        if self.fail == Some(at) {
            return Err(at);
        }
        Ok(self.replace.filter(|(index, _)| *index == at).map_or_else(
            || self.scripted.get(at).copied().unwrap_or(value),
            |(_, value)| value,
        ))
    }
}
impl ReckoningNumeric for Numeric {
    type Error = usize;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, usize> {
        self.value((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_estimate(&mut self, x: f32) -> Result<f32, usize> {
        self.value(1.0 / x)
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, usize> {
        self.value(1.0 / x.sqrt())
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, usize> {
        panic!("no source conversion")
    }
}
fn vector(xyz: [f32; 3], w: f32) -> V {
    V { xyz, w: Some(w) }
}
fn input(start: [f32; 3], delta: [f32; 3]) -> CollisionTriangleLineInput {
    CollisionTriangleLineInput {
        start: vector(start, 7.0),
        delta: vector(delta, 6.0),
        vertices: [
            vector([0.0, 0.0, 0.0], 1.0),
            vector([2.0, 0.0, 0.0], 1.0),
            vector([0.0, 2.0, 0.0], 1.0),
        ],
    }
}
fn hit(
    input: &CollisionTriangleLineInput,
    line: f32,
    triangle: f32,
) -> CollisionTriangleLineOutput {
    let mut out = CollisionTriangleLineOutput::default();
    assert_eq!(
        try_triangle_line(input, line, triangle, &mut out, &mut Numeric::default()),
        Ok(true)
    );
    out
}

#[test]
fn offset_faces_use_sum_for_time_but_original_line_radius_for_position() {
    for sign in [1.0, -1.0] {
        let out = hit(
            &input([0.5, 0.5, sign * 2.0], [0.0, 0.0, sign * -4.0]),
            0.25,
            0.25,
        );
        assert_eq!(out.line_parameter, Some(0.375));
        assert_eq!(out.position.unwrap(), vector([0.5, 0.5, sign * 0.25], 9.25));
        assert_eq!(out.normal.unwrap().xyz, [0.0, 0.0, sign]);
        assert_eq!(out.volume_parameter, Some([0.25, 0.25, 0.0, 0.0]));
    }
}

#[test]
fn negative_and_nan_radius_follow_source_face_arithmetic_without_policy_guard() {
    let source = input([0.5, 0.5, 2.0], [0.0, 0.0, -4.0]);
    let negative = hit(&source, -0.5, 0.0);
    assert_eq!(negative.line_parameter, Some(0.625));
    assert_eq!(negative.position.unwrap().xyz, [0.5, 0.5, 0.0]);
    let unordered = hit(&source, f32::NAN, 0.0);
    assert!(unordered.line_parameter.unwrap().is_nan());
    assert!(unordered.position.unwrap().xyz.into_iter().all(f32::is_nan));
    assert!(unordered.position.unwrap().w.unwrap().is_nan());
    assert_eq!(unordered.normal.unwrap().xyz, [0.0, 0.0, 1.0]);
}

#[test]
fn stationary_overlap_keeps_squared_overlap_and_unknown_w() {
    let mut source = input([0.5, 0.5, 0.25], [0.0; 3]);
    source.vertices[1].w = None;
    let out = hit(&source, 0.5, 0.0);
    assert_eq!(out.line_parameter, Some(0.0));
    assert_eq!(out.position.unwrap().xyz, [0.5, 0.5, -0.25]);
    assert_eq!(out.position.unwrap().w, None);
    assert_eq!(out.normal.unwrap().w, None);
    assert_eq!(out.volume_parameter, Some([0.25, 0.25, 0.1875, 0.0]));
}

#[test]
fn rounded_edge_and_vertex_hits_publish_feature_normal_and_barycentrics() {
    for (start, expected_bary, normal_xy) in [
        ([1.0, -0.25, 2.0], [0.5, 0.0, 0.0, 0.0], [0.0, -0.5]),
        ([-0.25, -0.25, 2.0], [0.0; 4], [-0.5, -0.5]),
    ] {
        let out = hit(&input(start, [0.0, 0.0, -4.0]), 0.5, 0.0);
        assert_eq!(out.volume_parameter, Some(expected_bary));
        assert_eq!(&out.normal.unwrap().xyz[..2], &normal_xy);
        assert!(out.position.unwrap().xyz[1].abs() < 0.000_001);
        assert!(out.position.unwrap().xyz[2].abs() < 0.000_001);
        assert!(out.line_parameter.unwrap() > 0.375);
        assert!(out.line_parameter.unwrap() < 0.5);
    }
}

#[test]
fn nearest_face_retains_absolute_determinant_not_backface_sign() {
    let source = input([0.5, 0.5, -0.5], [0.0, 0.0, 1.0]);
    let mut numeric = Numeric {
        replace: Some((6, -1.0)),
        ..Numeric::default()
    };
    let mut out = CollisionTriangleLineOutput::default();
    // Numeric port forces the pre-nearest numerator branch; the retained
    // determinant is positive after the original backface sign adjustment.
    assert_eq!(
        try_triangle_line(&source, 0.1, 0.0, &mut out, &mut numeric),
        Ok(true)
    );
    assert_eq!(out.normal.unwrap().xyz, [0.0, 0.0, 1.0]);
    assert_eq!(out.line_parameter, Some(0.0));
}

#[test]
fn fat_numeric_refusal_retains_entry_and_late_face_parameter_prefixes() {
    let source = input([0.5, 0.5, 2.0], [0.0, 0.0, -4.0]);
    for failure in 0..9 {
        let mut numeric = Numeric {
            fail: Some(failure),
            ..Numeric::default()
        };
        let mut out = CollisionTriangleLineOutput::default();
        assert_eq!(
            try_triangle_line(&source, 0.5, 0.0, &mut out, &mut numeric),
            Err(CollisionTriangleLineError::Numeric(failure))
        );
        assert_eq!(numeric.calls, failure + 1);
        assert_eq!(out.position, None);
        assert_eq!(out.volume_parameter, None);
        assert_eq!(
            out.line_parameter,
            if failure < 2 {
                None
            } else if failure < 8 {
                Some(0.0)
            } else {
                Some(0.375)
            }
        );
        assert_eq!(
            out.normal.unwrap().xyz,
            if failure < 2 {
                [0.0, 0.0, 4.0]
            } else {
                [0.0, 0.0, 1.0]
            }
        );
    }
}

#[test]
fn sphere_unordered_approach_is_negative_return_without_fraction_stores() {
    let mut fraction = Fraction { num: 7.0, den: 8.0 };
    let mut numeric = Numeric {
        replace: Some((1, f32::NAN)),
        ..Numeric::default()
    };
    assert_eq!(
        sphere(
            &mut fraction,
            vector([0.0; 3], 1.0),
            vector([1.0, 0.0, 0.0], 0.0),
            vector([2.0, 0.0, 0.0], 1.0),
            0.5,
            &mut numeric
        ),
        Ok(-1)
    );
    assert_eq!((fraction.num, fraction.den), (7.0, 8.0));
    assert_eq!(numeric.calls, 2);
}

#[test]
fn boundary_ties_and_unordered_fraction_compare_replace_after_positive_gates() {
    for num in [2.0, f32::NAN] {
        let mut fraction = Fraction { num, den: 4.0 };
        assert!(boundary(&mut fraction, true, 1.0, 2.0));
        assert_eq!((fraction.num, fraction.den), (1.0, 2.0));
        assert!(!boundary(&mut fraction, true, 1.0, f32::NAN));
    }
}

#[test]
fn fifth_region_transition_success_refuses_unproduced_wrapper_position() {
    // A numeric-port branch probe, not a geometric oracle: visit vertex0,
    // edge0, vertex0, edge0, vertex0, then exhaust the source counter.
    let mut script = vec![4.0, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0];
    let vertex = [1.0, 1.0, 1.0, 1.0, 1.0, 0.1, 0.0];
    let edge = [1.0, 1.0, 1.0, 1.0, -1.0, 0.1];
    for step in 0..5 {
        script.extend_from_slice(if step % 2 == 0 { &vertex } else { &edge });
    }
    let mut numeric = Numeric {
        scripted: script,
        ..Numeric::default()
    };
    let mut out = CollisionTriangleLineOutput::default();
    assert_eq!(
        try_triangle_line(&input([1.0; 3], [1.0; 3]), 0.5, 0.0, &mut out, &mut numeric),
        Err(CollisionTriangleLineError::UnreadySuccess { field: "position" })
    );
    assert_eq!(numeric.calls, numeric.scripted.len());
    let mut expected = 0.0_f32;
    for _ in 0..5 {
        expected = (1.0 - expected).mul_add(0.1, expected);
    }
    assert_eq!(out.line_parameter, Some(expected));
    assert_eq!(out.position, None);
    assert_eq!(out.volume_parameter, None);
    assert_eq!(out.normal.unwrap().xyz, [0.0, 0.0, 2.0]);
}
