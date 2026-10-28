use super::*;

#[derive(Default)]
struct Rounding {
    calls: Vec<u32>,
    result: Option<Result<f32, &'static str>>,
}

impl ReckoningSinCosNumeric for Rounding {
    type Error = &'static str;

    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push(value.to_bits());
        self.result
            .take()
            .unwrap_or_else(|| Ok(value.round_ties_even()))
    }
}

#[test]
fn frozen_source_polynomial_uses_independent_exact_f32_oracle() {
    // Derived BEFORE the Rust implementation from the raw coefficient tables
    // and documented power graph. The separate audit calculation used integer
    // significands for exact fused products/sums and nearest-even f32 rounding.
    // These are ordinary-host graph words, not native execution/VRFIN traces.
    for (angle, turns, sine, cosine) in [
        (0x0000_0000, 0x0000_0000, 0x0000_0000, 0x3f80_0000),
        (0x3f00_0000, 0x3da2_f983, 0x3ef5_7743, 0x3f60_a941),
        (0x3f80_0000, 0x3e22_f983, 0x3f57_6aa4, 0x3f0a_5140),
        (0x4049_0fdb, 0x3f00_0000, 0xb4a7_bec4, 0xbf7f_fffe),
        (0x4096_cbe4, 0x3f40_0000, 0xbf80_0000, 0xb3f5_e90e),
        (0x40c9_0fdb, 0x3f80_0000, 0x0000_0000, 0x3f80_0000),
        (0xc049_0fdb, 0xbf00_0000, 0x34a7_bec4, 0xbf7f_fffe),
        (0x447a_0000, 0x431f_27aa, 0x3f53_ad5b, 0x3f0f_f995),
        (0x3f33_3333, 0x3de4_2a1e, 0x3f24_eb73, 0x3f43_ccb3),
        (0x4039_999a, 0x3eec_5032, 0x3e74_fdb4, 0xbf78_90b9),
    ] {
        let mut numeric = Rounding::default();
        let actual = try_sin_cos(f32::from_bits(angle), &mut numeric).unwrap();
        assert_eq!((actual.0.to_bits(), actual.1.to_bits()), (sine, cosine));
        assert_eq!(numeric.calls, [turns]);
    }
}

#[test]
fn range_reduction_retains_fused_negative_product() {
    let actual = try_sin_cos(1000.0, &mut HostReckoningSinCosNumeric).unwrap();
    assert_eq!(
        (actual.0.to_bits(), actual.1.to_bits()),
        (0x3f53_ad5b, 0x3f0f_f995)
    );
    let two_pi = f32::from_bits(0x40c9_0fdb);
    let rounded_product = two_pi * 159.0;
    let split_reduced = 1000.0 - rounded_product;
    assert_eq!(split_reduced.to_bits(), 0x3f79_3800);
    assert_eq!((-two_pi).mul_add(159.0, 1000.0).to_bits(), 0x3f79_37d8);
    let split_result = sin_cos_from_reduced(split_reduced);
    assert_eq!(
        (split_result.0.to_bits(), split_result.1.to_bits()),
        (0x3f53_ad70, 0x3f0f_f974)
    );
    assert_ne!(actual.0.to_bits(), split_result.0.to_bits());
}

#[test]
fn fused_sums_and_nonsequential_power_dependencies_are_both_observable() {
    let actual = try_sin_cos(f32::from_bits(0x4049_0fdb), &mut HostReckoningSinCosNumeric).unwrap();
    assert_eq!(
        (actual.0.to_bits(), actual.1.to_bits()),
        (0xb4a7_bec4, 0xbf7f_fffe)
    );
    // Independently evaluated alternatives with all OTHER source choices kept:
    // split polynomial multiply/add => sine b294dd8b, cosine bf7ffffa;
    // sequential p[n]=rounded(p[n-1]*angle) => sine b2255165.
    assert_ne!(actual.0.to_bits(), 0xb294_dd8b);
    assert_ne!(actual.1.to_bits(), 0xbf7f_fffa);
    assert_ne!(actual.0.to_bits(), 0xb225_5165);
    let libc = f32::from_bits(0x4049_0fdb).sin_cos();
    assert_ne!(
        (actual.0.to_bits(), actual.1.to_bits()),
        (libc.0.to_bits(), libc.1.to_bits())
    );
}

#[test]
fn rounding_is_nearest_even_float_without_i32_range_restriction() {
    for (input, expected) in [
        (0.5_f32, 0.0_f32),
        (1.5, 2.0),
        (2.5, 2.0),
        (-0.5, -0.0),
        (-1.5, -2.0),
        (-2.5, -2.0),
        (2_147_483_648.0, 2_147_483_648.0),
        (-2_147_483_648.0, -2_147_483_648.0),
        (f32::INFINITY, f32::INFINITY),
    ] {
        assert_eq!(
            HostReckoningSinCosNumeric
                .round_nearest_even(input)
                .unwrap()
                .to_bits(),
            expected.to_bits()
        );
    }
    assert!(
        HostReckoningSinCosNumeric
            .round_nearest_even(f32::NAN)
            .unwrap()
            .is_nan()
    );
    // PI's rounded turns are exactly .5. Ties-away would choose the opposite
    // reduced angle and change the frozen sine sign, so this is not vacuous.
    let mut away = Rounding {
        result: Some(Ok(1.0)),
        ..Rounding::default()
    };
    let actual = try_sin_cos(f32::from_bits(0x4049_0fdb), &mut away).unwrap();
    assert_eq!(actual.0.to_bits(), 0x34a7_bec4);
    assert_eq!(away.calls, [0x3f00_0000]);
}

#[test]
fn one_rounding_service_failure_prevents_any_result_publication() {
    let mut numeric = Rounding {
        result: Some(Err("round")),
        ..Rounding::default()
    };
    assert_eq!(try_sin_cos(0.0, &mut numeric), Err("round"));
    assert_eq!(numeric.calls, [0]);
    let mut numeric = Rounding {
        result: Some(Ok(0.0)),
        ..Rounding::default()
    };
    let unreduced = try_sin_cos(f32::from_bits(0x40c9_0fdb), &mut numeric).unwrap();
    assert_ne!(
        (unreduced.0.to_bits(), unreduced.1.to_bits()),
        (0, 0x3f80_0000)
    );
    assert_eq!(numeric.calls, [0x3f80_0000]);
}

#[test]
fn exceptional_inputs_reach_rounding_without_finite_or_domain_repair() {
    for input in [f32::NAN, f32::INFINITY, -f32::INFINITY] {
        let mut numeric = Rounding::default();
        let (sine, cosine) = try_sin_cos(input, &mut numeric).unwrap();
        assert_eq!(numeric.calls.len(), 1);
        assert!(sine.is_nan());
        assert!(cosine.is_nan());
    }
    let result = try_sin_cos(2_147_483_648.0, &mut HostReckoningSinCosNumeric);
    assert!(result.is_ok()); // No implicit signed integer conversion gate.
}
