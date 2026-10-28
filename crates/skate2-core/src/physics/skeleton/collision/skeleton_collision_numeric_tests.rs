//! Independent finite arithmetic/callback discriminators, not Xenon traces.
//! Injected dot results deliberately isolate helper branches from dot policy.
use super::*;

#[derive(Debug, Eq, PartialEq)]
enum Event {
    Dot([u32; 3], [u32; 3]),
    Root(u32),
    Reciprocal(u32),
}
struct Numeric {
    squared: f32,
    root: f32,
    reciprocal: f32,
    events: Vec<Event>,
    fail_at: Option<usize>,
}
impl Numeric {
    fn new(squared: f32, root: f32, reciprocal: f32) -> Self {
        Self {
            squared,
            root,
            reciprocal,
            events: Vec::new(),
            fail_at: None,
        }
    }
    fn event(&mut self, event: Event) -> Result<(), usize> {
        let index = self.events.len();
        self.events.push(event);
        if self.fail_at == Some(index) {
            Err(index)
        } else {
            Ok(())
        }
    }
}
impl SkeletonCollisionNumeric for Numeric {
    type Error = usize;
    fn dot3(&mut self, a: [f32; 3], b: [f32; 3]) -> Result<f32, usize> {
        self.event(Event::Dot(a.map(f32::to_bits), b.map(f32::to_bits)))?;
        Ok(self.squared)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, usize> {
        self.event(Event::Root(value.to_bits()))?;
        Ok(self.root)
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, usize> {
        self.event(Event::Reciprocal(value.to_bits()))?;
        Ok(self.reciprocal)
    }
}

#[test]
fn safe_normalize_strict_epsilon_and_unordered_branch_copy_fallback_bits() {
    assert_eq!(
        (NORMALIZE_EPSILON * NORMALIZE_EPSILON).to_bits(),
        0x2b8c_bccc
    );
    let vector = [3.0, 4.0, -0.0];
    let fallback = [0x7fc0_1234, 0x8000_0000, 1].map(f32::from_bits);
    for squared in [0.0, -0.0, f32::from_bits(0x2b8c_bccc), f32::NAN] {
        let mut numeric = Numeric::new(squared, f32::NAN, f32::NAN);
        assert_eq!(
            try_normalize_safe_xyz(vector, fallback, &mut numeric)
                .unwrap()
                .map(f32::to_bits),
            fallback.map(f32::to_bits)
        );
        assert_eq!(
            numeric.events,
            [Event::Dot(
                vector.map(f32::to_bits),
                vector.map(f32::to_bits)
            )]
        );
    }
    let mut numeric = Numeric::new(f32::from_bits(0x2b8c_bccd), 1.0, 0.0);
    try_normalize_safe_xyz(vector, fallback, &mut numeric).unwrap();
    assert_eq!(numeric.events[1], Event::Root(0x2b8c_bccd));
}

#[test]
fn normalize_and_length_use_two_refinements_of_the_supplied_seed() {
    // Exact binary rational oracle: x=4,e0=1/4; e1=11/32;
    // e2=7117/16384. This separates zero, one and two refinement rounds.
    let vector = [2.0, 0.0, -0.0];
    let mut numeric = Numeric::new(4.0, 0.25, 0.0);
    assert_eq!(
        try_normalize_safe_xyz(vector, [0.0; 3], &mut numeric)
            .unwrap()
            .map(f32::to_bits),
        [0x3f5e_6800, 0, 0x8000_0000]
    );
    assert_eq!(
        try_length_xyz(vector, &mut numeric).unwrap().to_bits(),
        0x3fde_6800
    );
    assert_eq!(
        length_from_squared_and_estimate(4.0, 0.25).to_bits(),
        0x3fde_6800
    );
    assert_eq!(
        numeric.events,
        [
            Event::Dot(vector.map(f32::to_bits), vector.map(f32::to_bits)),
            Event::Root(4.0_f32.to_bits()),
            Event::Dot(vector.map(f32::to_bits), vector.map(f32::to_bits)),
            Event::Root(4.0_f32.to_bits()),
        ]
    );
}

#[test]
fn zero_length_still_requests_estimate_then_selects_positive_zero_after_nan_refinement() {
    for squared in [0.0, -0.0] {
        let mut numeric = Numeric::new(squared, f32::NAN, 0.0);
        assert_eq!(try_length_xyz([0.0; 3], &mut numeric).unwrap().to_bits(), 0);
        assert_eq!(
            numeric.events,
            [Event::Dot([0; 3], [0; 3]), Event::Root(squared.to_bits())]
        );
        assert_eq!(
            length_from_squared_and_estimate(squared, f32::INFINITY).to_bits(),
            0
        );
    }
    assert!(length_from_squared_and_estimate(f32::INFINITY, 0.0).is_nan());
}

#[test]
fn clamp_rejects_only_ordered_negative_maximum_before_numeric_reads() {
    for maximum in [-1.0_f32, f32::NEG_INFINITY] {
        let mut numeric = Numeric::new(4.0, 0.5, 0.5);
        assert_eq!(
            try_clamp_xyz([2.0, 0.0, 0.0], maximum, &mut numeric),
            Err(SkeletonCollisionClampError::NegativeMaximum {
                bits: maximum.to_bits()
            })
        );
        assert!(numeric.events.is_empty());
    }
    let mut numeric = Numeric::new(4.0, 0.5, 0.5);
    let value = try_clamp_xyz([2.0, 0.0, -0.0], -0.0, &mut numeric).unwrap();
    assert_eq!(value.map(f32::to_bits), [0x8000_0000, 0x8000_0000, 0]);
    assert_eq!(numeric.events.len(), 3);
    let mut numeric = Numeric::new(4.0, 0.5, 0.5);
    assert!(
        try_clamp_xyz([2.0, 0.0, 0.0], f32::NAN, &mut numeric)
            .unwrap()
            .iter()
            .all(|value| value.is_nan())
    );
    assert_eq!(numeric.events.len(), 3);
}

#[test]
fn clamp_small_length_is_strict_and_does_not_skip_the_root_estimate() {
    let vector = [1.0, -0.0, f32::from_bits(0x7fc0_1234)];
    let mut below = Numeric::new(f32::from_bits(0x2e80_0000), 131_072.0, 0.0);
    assert_eq!(
        try_clamp_xyz(vector, 0.0, &mut below)
            .unwrap()
            .map(f32::to_bits),
        vector.map(f32::to_bits)
    );
    assert_eq!(below.events.len(), 2);
    let mut equal = Numeric::new(f32::from_bits(0x2f80_0000), 65_536.0, 65_536.0);
    try_clamp_xyz([1.0, 0.0, 0.0], 0.0, &mut equal).unwrap();
    assert_eq!(equal.events[2], Event::Reciprocal(0x3780_0000));
}

#[test]
fn clamp_refines_reciprocal_twice_and_keeps_input_times_selected_grouping() {
    // Length4, reciprocal seed1/8 ->3/16 ->15/64. Selected2 gives
    // (4*2)*(15/64)=15/8, not either earlier refinement or exact division.
    let mut numeric = Numeric::new(16.0, 0.25, 0.125);
    assert_eq!(
        try_clamp_xyz([4.0, 0.0, -0.0], 2.0, &mut numeric)
            .unwrap()
            .map(f32::to_bits),
        [0x3ff0_0000, 0, 0x8000_0000]
    );
    assert_eq!(numeric.events[2], Event::Reciprocal(4.0_f32.to_bits()));
    // Deliberately injected length isolates ordering: input*selected overflows
    // BEFORE multiplying reciprocal; input*(selected/length) would remain finite.
    let mut numeric = Numeric::new(16.0, 0.25, 0.25);
    assert_eq!(
        try_clamp_xyz([f32::MAX, 0.0, 0.0], 2.0, &mut numeric).unwrap()[0].to_bits(),
        f32::INFINITY.to_bits()
    );
    assert!((f32::MAX * 0.5).is_finite());
}

#[test]
fn numeric_errors_preserve_exact_callback_prefix() {
    for failed in 0..3 {
        let mut numeric = Numeric::new(16.0, 0.25, 0.25);
        numeric.fail_at = Some(failed);
        assert_eq!(
            try_clamp_xyz([4.0, 0.0, 0.0], 2.0, &mut numeric),
            Err(SkeletonCollisionClampError::Numeric(failed))
        );
        assert_eq!(numeric.events.len(), failed + 1);
    }
    let mut numeric = Numeric::new(4.0, 0.5, 0.5);
    numeric.fail_at = Some(1);
    assert_eq!(
        try_normalize_safe_xyz([2.0, 0.0, 0.0], [0.0; 3], &mut numeric),
        Err(1)
    );
    assert_eq!(numeric.events.len(), 2);
}

#[test]
fn cross_fuses_negative_product_after_rounded_positive_product() {
    assert_eq!(
        cross_xyz([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]).map(f32::to_bits),
        [-3.0_f32, 6.0, -3.0].map(f32::to_bits)
    );
    // e=2^-23: rounded (1+e)^2 = 1+2e, then subtract exactly1+2e.
    // Fusing the positive product instead would leave e^2=2^-46. The zero's
    // sign below is this explicit host-FMA policy, NOT a native VNMSUB oracle.
    let a = [0.0, f32::from_bits(0x3f80_0001), 1.0];
    let b = [
        0.0,
        f32::from_bits(0x3f80_0002),
        f32::from_bits(0x3f80_0001),
    ];
    assert_eq!(cross_xyz(a, b)[0].to_bits(), 0);
    assert_eq!(a[1].mul_add(b[2], -(a[2] * b[1])).to_bits(), 0x2880_0000);
    assert_eq!(
        cross_xyz([0.0, -0.0, 0.0], [0.0, 1.0, 1.0])[0].to_bits(),
        0x8000_0000
    );
}

#[test]
fn host_numeric_policy_is_explicit_left_association_and_unmodified_estimates() {
    let mut numeric = HostSkeletonCollisionNumeric;
    assert_eq!(
        numeric
            .dot3([16_777_216.0, -16_777_216.0, 1.0], [1.0; 3])
            .unwrap()
            .to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(
        numeric.reciprocal_sqrt_estimate(4.0).unwrap().to_bits(),
        0.5_f32.to_bits()
    );
    assert_eq!(
        numeric.reciprocal_estimate(-0.0).unwrap().to_bits(),
        f32::NEG_INFINITY.to_bits()
    );
    assert_eq!(try_length_xyz([0.0; 3], &mut numeric).unwrap().to_bits(), 0);
    let settled = [1.0e-20, 0.0, -1.0e-20];
    assert_eq!(try_length_xyz(settled, &mut numeric).unwrap().to_bits(), 0);
    assert_eq!(
        try_clamp_xyz(settled, 0.5, &mut numeric)
            .unwrap()
            .map(f32::to_bits),
        settled.map(f32::to_bits)
    );
}
