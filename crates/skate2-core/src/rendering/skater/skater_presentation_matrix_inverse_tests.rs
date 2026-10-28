use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;

pub(super) struct Numeric {
    pub(super) calls: Vec<&'static str>,
    pub(super) fail_estimate: bool,
}
impl Numeric {
    pub(super) fn new() -> Self {
        Self {
            calls: vec![],
            fail_estimate: false,
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("general inverse must use dot4")
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push("reciprocal");
        if self.fail_estimate {
            Err("estimate")
        } else {
            Ok(value.recip())
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push("rsqrt");
        Ok(value.sqrt().recip())
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric
            .truncate_i32(value)
            .map_err(|_| "integer")
    }
}
impl PresentationMatrixNumeric for Numeric {
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls.push("dot4");
        Ok(((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]) + a[3] * b[3])
    }
}

#[test]
fn general_inverse_consumes_bottom_row_and_four_lane_determinant() {
    let matrix = [
        2.0, 0.0, 0.0, 1.0, 0.0, 4.0, 0.0, 0.0, 0.0, 0.0, 8.0, 0.0, 6.0, 8.0, 16.0, 2.0,
    ];
    let mut numeric = Numeric::new();
    let inverse = try_inverse(matrix, &mut numeric).unwrap();
    // Independent exact rational inverse; this nonaffine matrix excludes a
    // transpose or affine-inverse implementation passing the ordinary fixture.
    let expected: [f32; 16] = [
        -1.0, -1.0, -1.0, 0.5, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 3.0, 2.0, 2.0, -1.0,
    ];
    for (actual, expected) in inverse.into_iter().zip(expected) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }
    assert_eq!(numeric.calls, ["dot4", "reciprocal"]);
}

#[test]
fn singular_matrix_still_calls_actual_estimate_and_does_not_return_identity() {
    let mut numeric = Numeric::new();
    let result = try_inverse([0.0; 16], &mut numeric).unwrap();
    assert!(result.into_iter().all(f32::is_nan));
    assert_eq!(numeric.calls, ["dot4", "reciprocal"]);
}

#[test]
fn estimate_failure_is_propagated_after_four_lane_dot() {
    let mut numeric = Numeric::new();
    numeric.fail_estimate = true;
    assert_eq!(
        try_inverse([0.0; 16], &mut numeric).unwrap_err(),
        "estimate"
    );
    assert_eq!(numeric.calls, ["dot4", "reciprocal"]);
}
