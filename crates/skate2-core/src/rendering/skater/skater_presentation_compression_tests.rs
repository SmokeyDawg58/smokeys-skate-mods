use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;

const VECTORS: CompressionVectors = CompressionVectors {
    ratio: [1.0; 4],
    zero: [0.0; 4],
    minus_one: [-1.0; 4],
};
const SENTINEL: [u16; 3] = [0xdead; 3];
#[derive(Debug, PartialEq, Eq)]
enum Event {
    Integer(u32),
    Estimate(u32),
}
#[derive(Default)]
struct Numeric {
    events: Vec<Event>,
    fail_integer: Option<usize>,
    integers: usize,
    seed: Option<f32>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("compression has no dot")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("compression has no rsqrt")
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.events.push(Event::Estimate(value.to_bits()));
        Ok(self.seed.unwrap_or(1.0 / value))
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        self.events.push(Event::Integer(value.to_bits()));
        let index = self.integers;
        self.integers += 1;
        if self.fail_integer == Some(index) {
            return Err("conversion");
        }
        HostReckoningNumeric
            .truncate_i32(value)
            .map_err(|_| "integer")
    }
}
fn sqt(q: [f32; 4], t: [f32; 4]) -> AndaleSqt {
    AndaleSqt {
        scale: [7.0; 4],
        rotation: q,
        translation: t,
    }
}
fn values(events: &[Event]) -> Vec<f32> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Integer(bits) => Some(f32::from_bits(*bits)),
            Event::Estimate(_) => None,
        })
        .collect()
}
fn assert_values(events: &[Event], expected: &[f32]) {
    assert_eq!(
        values(events)
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
}

#[test]
fn q3_preserves_low_halfwords_without_clamping_or_normalization() {
    let source = [sqt([-1.75, 65_537.75, -32_768.5, 1.0], [0.0; 4])];
    let mut output = [SENTINEL];
    let mut numeric = Numeric::default();
    try_compress_q3(
        CompressionSource::Sqts(&source),
        1,
        &mut output,
        VECTORS,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(output, [[0xffff, 1, 0x8000]]);
    assert_values(&numeric.events, &[65_537.75, -1.75, -32_768.5]);
}

#[test]
fn q3_loop_and_tail_preserve_different_partial_store_boundaries() {
    let source = [sqt([1.0, 2.0, 3.0, 1.0], [0.0; 4]); 2];
    for (failure, expected) in [(2, [SENTINEL; 2]), (5, [[1, 2, 3], [0xdead, 2, 0xdead]])] {
        let mut output = [SENTINEL; 2];
        let mut numeric = Numeric {
            fail_integer: Some(failure),
            ..Numeric::default()
        };
        assert_eq!(
            try_compress_q3(
                CompressionSource::Sqts(&source),
                2,
                &mut output,
                VECTORS,
                &mut numeric
            ),
            Err(CompressionError::Numeric("conversion"))
        );
        assert_eq!(output, expected);
    }
}

#[test]
fn q3_sign_gate_uses_scaled_w_and_all_supplied_zero_lanes() {
    let source = [[1.0, 2.0, 3.0, 2.0]];
    let mut vectors = VECTORS;
    vectors.ratio[3] = -1.0;
    for (zero, expected) in [
        ([0.0; 4], [0xffff, 0xfffe, 0xfffd]),
        ([0.0, 0.0, f32::NAN, 0.0], [1, 2, 3]),
    ] {
        vectors.zero = zero;
        let mut output = [SENTINEL];
        try_compress_q3(
            CompressionSource::Vectors {
                quaternions: &source,
                translations: &[],
            },
            1,
            &mut output,
            vectors,
            &mut Numeric::default(),
        )
        .unwrap();
        assert_eq!(output, [expected]);
    }
}

#[test]
fn range_assertions_precede_estimate_and_ignore_w() {
    for (translation, range, axis) in [
        ([0.5, 0.0, 0.0, 1.0], 0.5, 0),
        ([0.0, -0.5, 0.0, 1.0], 0.5, 1),
        ([0.0, 0.0, f32::NAN, 1.0], 0.5, 2),
        ([0.0; 4], f32::NAN, 0),
    ] {
        let mut output = SENTINEL;
        let mut numeric = Numeric::default();
        assert!(
            matches!(try_compress_translation(translation, range, [1.0; 4], &mut output, &mut numeric), Err(CompressionError::RangeAssertion { axis: actual, .. }) if actual == axis)
        );
        assert!(numeric.events.is_empty());
        assert_eq!(output, SENTINEL);
    }
    let mut output = SENTINEL;
    try_compress_translation(
        [0.125, 0.25, -0.25, f32::NAN],
        0.5,
        [8.0; 4],
        &mut output,
        &mut Numeric::default(),
    )
    .unwrap();
    assert_eq!(output, [2, 4, 0xfffc]);
}

#[test]
fn standalone_translation_uses_two_refinements_and_all_conversions_before_stores() {
    let mut output = SENTINEL;
    let mut numeric = Numeric {
        seed: Some(1.75),
        ..Numeric::default()
    };
    try_compress_translation(
        [0.125, 0.25, -0.25, 1.0],
        0.5,
        [8.0; 4],
        &mut output,
        &mut numeric,
    )
    .unwrap();
    // Seed1.75 -> first1.96875 -> second1.99951171875. Distinguishes either
    // missing refinement or replacement with exact host division.
    assert_values(&numeric.events, &[-3.999_023_4, 1.999_511_7, 3.999_023_4]);
    assert_eq!(output, [1, 3, 0xfffd]);
    assert_eq!(numeric.events[0], Event::Estimate(0.5_f32.to_bits()));
    let mut output = SENTINEL;
    let mut numeric = Numeric {
        fail_integer: Some(2),
        ..Numeric::default()
    };
    assert!(
        try_compress_translation([0.125; 4], 0.5, [8.0; 4], &mut output, &mut numeric).is_err()
    );
    assert_eq!(output, SENTINEL);
}

#[test]
fn combined_uses_direct_division_and_loop_then_tail_conversion_order() {
    let source = [sqt([1.0, 2.0, 3.0, 1.0], [0.125, 0.25, -0.25, f32::NAN]); 2];
    let mut q = [SENTINEL; 2];
    let mut t = [SENTINEL; 2];
    let mut numeric = Numeric {
        seed: Some(0.0),
        ..Numeric::default()
    };
    try_compress_quaternion_translation(
        CompressionSource::Sqts(&source),
        2,
        &mut QuaternionTranslationDestination {
            quaternions: &mut q,
            translations: &mut t,
        },
        0.5,
        VECTORS,
        &mut numeric,
    )
    .unwrap();
    assert!(
        numeric
            .events
            .iter()
            .all(|e| matches!(e, Event::Integer(_)))
    );
    assert_values(
        &numeric.events,
        &[
            3.0, 8191.75, 16383.5, 1.0, 2.0, -16383.5, 2.0, 3.0, 1.0, 16383.5, -16383.5, 8191.75,
        ],
    );
    assert_eq!(q, [[1, 2, 3]; 2]);
    assert_eq!(t, [[8191, 16383, 0xc001]; 2]);
}

#[test]
fn combined_tail_failure_preserves_previous_record_and_only_current_qy() {
    let source = [sqt([1.0, 2.0, 3.0, 1.0], [0.125; 4]); 2];
    let mut q = [SENTINEL; 2];
    let mut t = [SENTINEL; 2];
    let mut numeric = Numeric {
        fail_integer: Some(9),
        ..Numeric::default()
    };
    assert_eq!(
        try_compress_quaternion_translation(
            CompressionSource::Sqts(&source),
            2,
            &mut QuaternionTranslationDestination {
                quaternions: &mut q,
                translations: &mut t
            },
            0.5,
            VECTORS,
            &mut numeric
        ),
        Err(CompressionError::Numeric("conversion"))
    );
    assert_eq!(q, [[1, 2, 3], [0xdead, 2, 0xdead]]);
    assert_eq!(t, [[8191; 3], SENTINEL]);
}

#[test]
fn later_range_failure_keeps_completed_record_and_prefetch_failure_precedes_current_writes() {
    let source = [
        sqt([1.0, 2.0, 3.0, 1.0], [0.125; 4]),
        sqt([4.0; 4], [0.5; 4]),
    ];
    let mut q = [SENTINEL; 2];
    let mut t = [SENTINEL; 2];
    assert!(matches!(
        try_compress_quaternion_translation(
            CompressionSource::Sqts(&source),
            2,
            &mut QuaternionTranslationDestination {
                quaternions: &mut q,
                translations: &mut t
            },
            0.5,
            VECTORS,
            &mut Numeric::default()
        ),
        Err(CompressionError::RangeAssertion { index: 1, .. })
    ));
    assert_eq!(q, [[1, 2, 3], SENTINEL]);
    assert_eq!(t, [[8191; 3], SENTINEL]);
    let mut q = [SENTINEL; 2];
    let mut numeric = Numeric::default();
    assert_eq!(
        try_compress_q3(
            CompressionSource::Sqts(&source[..1]),
            2,
            &mut q,
            VECTORS,
            &mut numeric
        ),
        Err(CompressionError::SourceBounds {
            translation: false,
            index: 1
        })
    );
    assert_eq!(q, [SENTINEL; 2]);
    assert!(numeric.events.is_empty());
}

#[test]
fn nonpositive_count_is_explicit_admission_failure_not_empty_success() {
    for count in [0, -1, i32::MIN] {
        let mut numeric = Numeric::default();
        assert_eq!(
            try_compress_q3(
                CompressionSource::Sqts(&[]),
                count,
                &mut [],
                VECTORS,
                &mut numeric
            ),
            Err(CompressionError::NonPositiveCount(count))
        );
        assert_eq!(
            try_compress_quaternion_translation(
                CompressionSource::Sqts(&[]),
                count,
                &mut QuaternionTranslationDestination {
                    quaternions: &mut [],
                    translations: &mut []
                },
                0.5,
                VECTORS,
                &mut numeric
            ),
            Err(CompressionError::NonPositiveCount(count))
        );
        assert!(numeric.events.is_empty());
    }
}
