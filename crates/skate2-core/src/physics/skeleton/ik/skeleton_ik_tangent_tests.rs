use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Round(u32),
    Reciprocal(u32),
}
#[derive(Default)]
struct Trace {
    events: Vec<Event>,
    fail_at: Option<usize>,
}
impl Trace {
    fn push(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail_at == Some(self.events.len()) {
            Err("refused")
        } else {
            Ok(())
        }
    }
}
struct Numeric {
    trace: Rc<RefCell<Trace>>,
    seeds: [Option<f32>; 2],
    count: usize,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.trace
            .borrow_mut()
            .push(Event::Reciprocal(value.to_bits()))?;
        let result = self.seeds[self.count].unwrap_or(1.0 / value);
        self.count += 1;
        Ok(result)
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("tangent does not normalize")
    }
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("tangent has no dot reduction")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("scalar checked conversion is not native vector saturation")
    }
}
struct Rounding(Rc<RefCell<Trace>>);
impl ReckoningSinCosNumeric for Rounding {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.0.borrow_mut().push(Event::Round(value.to_bits()))?;
        Ok(value.round_ties_even())
    }
}
fn fixture() -> (Numeric, Rounding) {
    let trace = Rc::new(RefCell::new(Trace::default()));
    (
        Numeric {
            trace: Rc::clone(&trace),
            seeds: [None; 2],
            count: 0,
        },
        Rounding(trace),
    )
}

#[test]
fn both_signed_zeros_run_all_services_and_return_positive_zero() {
    for value in [0.0_f32, -0.0] {
        let (mut numeric, mut rounding) = fixture();
        assert_eq!(
            try_tangent(value, &mut numeric, &mut rounding)
                .unwrap()
                .to_bits(),
            0
        );
        let trace = numeric.trace.borrow();
        assert_eq!(trace.events.len(), 3);
        assert_eq!(trace.events[0], Event::Round(value.to_bits()));
        assert_eq!(trace.events[1], Event::Reciprocal(1.0_f32.to_bits()));
        // The source negative fused reduction's exact-zero sign is a stated
        // host boundary. Either sign still requires this second estimate.
        let Event::Reciprocal(bits) = trace.events[2] else {
            panic!("second estimate")
        };
        assert_eq!(bits & 0x7fff_ffff, 0);
    }
}

#[test]
fn independent_host_seed_snapshots_cover_both_parities_and_ctor_endpoints() {
    // Frozen Python/ucrtbase.fmaf evaluation of raw source, with f32 1/x seeds.
    // These prove this host DAG, not unknown Xenon estimate result words.
    for (input, expected) in [
        (0x3c23_d70a, 0x3c23_d871),
        (0x3fc7_c82d, 0x42c7_fe57),
        (0x3f00_0000, 0x3f0b_da7b),
        (0xbf00_0000, 0xbf0b_da7b),
        (0x3f80_0000, 0x3fc7_5921),
        (0xbf80_0000, 0xbfc7_5921),
        (0x3980_0000, 0x3980_0000),
    ] {
        let (mut numeric, mut rounding) = fixture();
        assert_eq!(
            try_tangent(f32::from_bits(input), &mut numeric, &mut rounding)
                .unwrap()
                .to_bits(),
            expected,
            "input {input:08x}"
        );
        assert_eq!(numeric.count, 2);
    }
}

#[test]
fn independent_geometry_oracle_covers_signs_and_reduction_regions() {
    for value in [-3.0_f32, -2.0, -1.0, -0.5, -0.01, 0.01, 0.5, 1.0, 2.0, 3.0] {
        let (mut numeric, mut rounding) = fixture();
        let actual = try_tangent(value, &mut numeric, &mut rounding).unwrap();
        let expected = f64::from(value).tan();
        assert!(
            (f64::from(actual) - expected).abs() < 0.000_002,
            "{value}: {actual} vs {expected}"
        );
    }
}

#[test]
fn each_service_refusal_stops_at_its_source_prefix_even_for_zero() {
    for fail_at in 1..=3 {
        let (mut numeric, mut rounding) = fixture();
        numeric.trace.borrow_mut().fail_at = Some(fail_at);
        let result = try_tangent(0.0, &mut numeric, &mut rounding);
        assert_eq!(
            result,
            Err(if fail_at == 1 {
                SkeletonIkTangentError::Rounding("refused")
            } else {
                SkeletonIkTangentError::Numeric("refused")
            })
        );
        assert_eq!(numeric.trace.borrow().events.len(), fail_at);
    }
}

#[test]
fn selected_even_path_still_requests_the_odd_estimate() {
    let (mut numeric, mut rounding) = fixture();
    numeric.seeds = [None, Some(f32::NAN)];
    assert_eq!(
        try_tangent(0.5, &mut numeric, &mut rounding)
            .unwrap()
            .to_bits(),
        0x3f0b_da7b
    );
    assert_eq!(numeric.count, 2);
}

#[test]
fn selected_odd_path_still_requests_the_even_estimate() {
    let (mut numeric, mut rounding) = fixture();
    numeric.seeds = [Some(f32::NAN), None];
    assert_eq!(
        try_tangent(1.0, &mut numeric, &mut rounding)
            .unwrap()
            .to_bits(),
        0x3fc7_5921
    );
    assert_eq!(numeric.count, 2);
}

#[test]
fn nan_first_refinement_retains_its_original_infinite_seed() {
    for (value, seeds) in [
        (0.5, [Some(f32::INFINITY), None]),
        (1.0, [None, Some(f32::INFINITY)]),
    ] {
        let (mut numeric, mut rounding) = fixture();
        numeric.seeds = seeds;
        assert_eq!(
            try_tangent(value, &mut numeric, &mut rounding)
                .unwrap()
                .to_bits(),
            f32::INFINITY.to_bits()
        );
        assert_eq!(numeric.count, 2);
    }
}

#[test]
fn bounds_mask_has_raw_high_bits_with_inclusive_edges_and_unordered() {
    let bound = f32::from_bits(0x3980_0000);
    for value in [-bound, -0.0, 0.0, bound] {
        assert_eq!(bounds_bits(value, bound), 0);
    }
    assert_eq!(bounds_bits(f32::from_bits(0x3980_0001), bound), 0x8000_0000);
    assert_eq!(bounds_bits(f32::from_bits(0xb980_0001), bound), 0x4000_0000);
    assert_eq!(bounds_bits(f32::INFINITY, bound), 0x8000_0000);
    assert_eq!(bounds_bits(f32::NEG_INFINITY, bound), 0x4000_0000);
    assert_eq!(bounds_bits(f32::NAN, bound), 0xc000_0000);
    assert_eq!(bounds_bits(0.0, f32::NAN), 0xc000_0000);
    // Partial mask must preserve the other thirty bits, unlike a bool select.
    assert_eq!(
        bitselect(f32::from_bits(0xff81_2345), 1.0, 0xc000_0000).to_bits(),
        0x3f81_2345
    );
}

#[test]
fn vector_conversion_truncates_saturates_and_maps_nan_to_zero() {
    for (value, expected) in [
        (1.75, 1),
        (-1.75, -1),
        (-0.0, 0),
        (f32::NAN, 0),
        (f32::INFINITY, i32::MAX),
        (f32::NEG_INFINITY, i32::MIN),
        (2_147_483_648.0, i32::MAX),
        (-2_147_483_648.0, i32::MIN),
    ] {
        assert_eq!(saturating_signed_word(value), expected);
    }
}

#[test]
fn exceptional_inputs_keep_full_service_sequence_without_checked_integer_refusal() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let (mut numeric, mut rounding) = fixture();
        assert!(
            try_tangent(value, &mut numeric, &mut rounding)
                .unwrap()
                .is_nan()
        );
        assert_eq!(numeric.count, 2);
        assert_eq!(numeric.trace.borrow().events.len(), 3);
    }
}
