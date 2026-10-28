use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
const IDENTITY: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];
// Quaternion [+1/2,-1/2,+1/2,+1/2], with all diagonal candidates tied.
const TIED: AnimationMatrix = [
    0.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];
const SENTINEL: [u16; 4] = [0xdead; 4];
#[derive(Default)]
struct Numeric {
    estimates: Vec<u32>,
    integers: Vec<u32>,
    fail_integer: Option<usize>,
    fail_estimate: Option<usize>,
    results: Option<[i32; 4]>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("Q4 does not normalize")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("Q4 only estimates reciprocal square roots")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        let index = self.estimates.len();
        self.estimates.push(value.to_bits());
        if self.fail_estimate == Some(index) {
            return Err("estimate");
        }
        Ok(value.sqrt().recip())
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        let index = self.integers.len();
        self.integers.push(value.to_bits());
        if self.fail_integer == Some(index) {
            return Err("integer");
        }
        if let Some(results) = self.results {
            return Ok(results[index % 4]);
        }
        HostReckoningNumeric
            .truncate_i32(value)
            .map_err(|_| "host integer")
    }
}
#[test]
fn four_estimates_then_xywz_conversion_preserve_low_halfwords() {
    let mut numeric = Numeric {
        results: Some([65_537, -2, 131_075, -65_540]),
        ..Numeric::default()
    };
    let mut destination = [SENTINEL];
    try_compress_matrix_q4(
        &[TIED],
        &[0],
        &mut destination,
        [2.0, 4.0, 6.0, 8.0],
        &mut numeric,
    )
    .unwrap();
    assert_eq!(numeric.estimates, [1.0_f32.to_bits(); 4]);
    assert_eq!(
        numeric.integers,
        [1.0_f32, -2.0, 4.0, 3.0].map(f32::to_bits)
    );
    assert_eq!(destination, [[1, 0xfffe, 0xfffc, 3]]);
}
#[test]
fn no_final_normalization_or_axis_w_consumption_and_original_index_order() {
    let mut scaled = IDENTITY;
    scaled[0] = 2.0;
    scaled[5] = 2.0;
    scaled[10] = 2.0;
    for index in [3, 7, 11, 12, 13, 14, 15] {
        scaled[index] = f32::NAN;
    }
    let mut output = [SENTINEL; 2];
    let mut numeric = Numeric::default();
    try_compress_matrix_q4(
        &[IDENTITY, scaled],
        &[1, 0],
        &mut output,
        [100.0; 4],
        &mut numeric,
    )
    .unwrap();
    // sqrt(7)/2*100 truncates to132; a normalized substitute would produce100.
    assert_eq!(output[0], [0, 0, 0, 132]);
    assert_eq!(output[1], [0, 0, 0, 100]);
    assert_eq!(
        &numeric.estimates[..4],
        [7.0_f32, -1.0, -1.0, -1.0].map(f32::to_bits)
    );
    assert_eq!(
        &numeric.estimates[4..],
        [4.0_f32, 0.0, 0.0, 0.0].map(f32::to_bits)
    );
}
#[test]
fn each_conversion_failure_preserves_the_entire_current_record() {
    for failure in 0..4 {
        let mut numeric = Numeric {
            fail_integer: Some(4 + failure),
            ..Numeric::default()
        };
        let mut destination = [SENTINEL; 2];
        assert_eq!(
            try_compress_matrix_q4(&[TIED], &[0, 0], &mut destination, [8.0; 4], &mut numeric),
            Err(MatrixQ4CompressionError::Numeric("integer"))
        );
        assert_eq!(destination[0], [4, 0xfffc, 4, 4]);
        assert_eq!(destination[1], SENTINEL);
        assert_eq!(numeric.integers.len(), 5 + failure);
    }
}
#[test]
fn unselected_estimate_failure_is_not_skipped() {
    let mut numeric = Numeric {
        fail_estimate: Some(3),
        ..Numeric::default()
    };
    let mut destination = [SENTINEL];
    assert_eq!(
        try_compress_matrix_q4(&[IDENTITY], &[0], &mut destination, [1.0; 4], &mut numeric),
        Err(MatrixQ4CompressionError::Numeric("estimate"))
    );
    assert_eq!(numeric.estimates.len(), 4);
    assert!(numeric.integers.is_empty());
    assert_eq!(destination, [SENTINEL]);
}
#[test]
fn late_matrix_bounds_preserve_prior_output_but_capacity_is_admission() {
    let mut numeric = Numeric::default();
    let mut destination = [SENTINEL; 2];
    assert_eq!(
        try_compress_matrix_q4(&[TIED], &[0, 7], &mut destination, [8.0; 4], &mut numeric),
        Err(MatrixQ4CompressionError::MatrixBounds {
            ordinal: 1,
            index: 7
        })
    );
    assert_eq!(destination[0], [4, 0xfffc, 4, 4]);
    assert_eq!(destination[1], SENTINEL);
    let mut numeric = Numeric::default();
    assert_eq!(
        try_compress_matrix_q4(
            &[TIED],
            &[0, 0],
            &mut destination[..1],
            [8.0; 4],
            &mut numeric
        ),
        Err(MatrixQ4CompressionError::DestinationBounds {
            required: 2,
            available: 1
        })
    );
    assert!(numeric.estimates.is_empty());
}
#[test]
fn empty_original_offset_list_skips_source_and_numeric_work() {
    let mut numeric = Numeric::default();
    let mut destination = [SENTINEL];
    try_compress_matrix_q4(&[], &[], &mut destination, [f32::NAN; 4], &mut numeric).unwrap();
    assert_eq!(destination, [SENTINEL]);
    assert!(numeric.estimates.is_empty());
    assert!(numeric.integers.is_empty());
}
