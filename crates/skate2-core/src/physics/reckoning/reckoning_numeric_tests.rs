use super::*;
use std::collections::VecDeque;

// Exact representable fixtures and explicit ordinary-host assertions.
#[allow(clippy::float_cmp)]
mod source {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum Event {
        Dot([u32; 4], [u32; 4]),
        Sqrt(u32),
        Reciprocal(u32),
        Integer(u32),
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Error {
        Injected(usize),
        Host(ReckoningHostNumericError),
    }

    #[derive(Default)]
    struct Numeric {
        events: Vec<Event>,
        dots: VecDeque<f32>,
        seeds: VecDeque<f32>,
        fail_at: Option<usize>,
    }
    impl Numeric {
        fn record(&mut self, event: Event) -> Result<(), Error> {
            let index = self.events.len();
            self.events.push(event);
            if self.fail_at == Some(index) {
                Err(Error::Injected(index))
            } else {
                Ok(())
            }
        }
    }
    impl ReckoningNumeric for Numeric {
        type Error = Error;
        fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Error> {
            self.record(Event::Dot(a.map(f32::to_bits), b.map(f32::to_bits)))?;
            if let Some(value) = self.dots.pop_front() {
                Ok(value)
            } else {
                HostReckoningNumeric.dot3(a, b).map_err(Error::Host)
            }
        }
        fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Error> {
            self.record(Event::Sqrt(value.to_bits()))?;
            if let Some(seed) = self.seeds.pop_front() {
                Ok(seed)
            } else {
                HostReckoningNumeric
                    .reciprocal_sqrt_estimate(value)
                    .map_err(Error::Host)
            }
        }
        fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Error> {
            self.record(Event::Reciprocal(value.to_bits()))?;
            HostReckoningNumeric
                .reciprocal_estimate(value)
                .map_err(Error::Host)
        }
        fn truncate_i32(&mut self, value: f32) -> Result<i32, Error> {
            self.record(Event::Integer(value.to_bits()))?;
            HostReckoningNumeric
                .truncate_i32(value)
                .map_err(Error::Host)
        }
    }

    const X: [f32; 4] = [1.0, 0.0, 0.0, 0.0];
    const Y: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
    const Z: [f32; 4] = [0.0, 0.0, 1.0, 0.0];

    #[test]
    fn inverse_trig_has_original_polynomial_not_host_libc_or_endpoint_repair() {
        // Independently evaluated raw coefficient polynomial with explicit seed
        // words, not production outputs or asserted Xenon estimate results.
        for (input, denominator, seed, expected_asin, expected_acos) in [
            (0.0, 0x3f80_0001, 0x3f7f_ffff, 0x0000_0000, 0x3fc9_0fdb),
            (0.25, 0x3f40_0002, 0x3f93_cd39, 0x3e81_5f52, 0x3fa8_b806),
            (0.5, 0x3f00_0002, 0x3fb5_04f2, 0x3f06_0a91, 0x3f86_0a92),
            (0.75, 0x3e80_0004, 0x3fff_fffc, 0x3f59_1a99, 0x3f39_051d),
            (1.0, 0x3400_0000, 0x4535_04f3, 0x3fc9_0fd7, 0x3500_0000),
            (-0.5, 0x3f00_0002, 0x3fb5_04f2, 0xbf06_0a91, 0x4006_0a92),
            (-1.0, 0x3400_0000, 0x4535_04f3, 0xbfc9_0fd7, 0x4049_0fd9),
        ] {
            let mut numeric = Numeric {
                seeds: [f32::from_bits(seed)].into(),
                ..Numeric::default()
            };
            assert_eq!(
                try_asin(input, &mut numeric).unwrap().to_bits(),
                expected_asin
            );
            assert_eq!(numeric.events, [Event::Sqrt(denominator)]);
            let mut numeric = Numeric {
                seeds: [f32::from_bits(seed)].into(),
                ..Numeric::default()
            };
            assert_eq!(
                try_acos(input, &mut numeric).unwrap().to_bits(),
                expected_acos
            );
            assert_eq!(numeric.events, [Event::Sqrt(denominator)]);
        }
        assert_ne!(
            try_acos(1.0, &mut HostReckoningNumeric).unwrap(),
            1.0_f32.acos()
        );
    }

    #[test]
    fn inverse_trig_does_not_clamp_domain_or_skip_estimate_for_zero() {
        let mut numeric = Numeric::default();
        assert!(try_asin(2.0, &mut numeric).unwrap().is_nan());
        assert_eq!(numeric.events.len(), 1);
        assert!(matches!(numeric.events[0], Event::Sqrt(bits) if f32::from_bits(bits) < 0.0));
        let mut numeric = Numeric {
            fail_at: Some(0),
            ..Numeric::default()
        };
        assert_eq!(try_acos(0.0, &mut numeric), Err(Error::Injected(0)));
        assert_eq!(numeric.events, [Event::Sqrt(0x3f80_0001)]);
    }

    #[test]
    fn angle_gates_are_lazy_ordered_and_do_not_use_normalize_safe_threshold() {
        for norm in [0.0, f32::from_bits(0x38d1_b717), f32::NAN] {
            let mut numeric = Numeric {
                dots: [norm].into(),
                ..Numeric::default()
            };
            assert_eq!(
                try_signed_angle(X, Z, Y, &mut numeric).unwrap().to_bits(),
                0
            );
            assert_eq!(numeric.events.len(), 1);
        }
        let mut numeric = Numeric {
            dots: [1.0, 0.0].into(),
            ..Numeric::default()
        };
        assert_eq!(try_unsigned_angle(X, Z, &mut numeric).unwrap().to_bits(), 0);
        assert_eq!(numeric.events.len(), 2);
    }

    #[test]
    fn signed_and_unsigned_angles_keep_one_refinement_and_wind_after_polynomial() {
        let a = [1.0, 0.0, 0.0, 8.0];
        let b = [0.0, 0.0, 1.0, -4.0];
        for signed in [false, true] {
            let mut numeric = Numeric {
                seeds: [0.5, 0.5].into(),
                ..Numeric::default()
            };
            let angle = if signed {
                try_signed_angle(a, b, Y, &mut numeric).unwrap()
            } else {
                try_unsigned_angle(a, b, &mut numeric).unwrap()
            };
            assert_eq!(numeric.events.len(), if signed { 7 } else { 6 });
            assert_eq!(numeric.events[2], Event::Sqrt(1.0_f32.to_bits()));
            assert_eq!(numeric.events[3], Event::Sqrt(1.0_f32.to_bits()));
            // One Newton round from seed .5 gives .6875, not two-round .8687744.
            assert_eq!(
                numeric.events[4],
                Event::Dot(
                    [0.6875_f32, 0.0, 0.0, 5.5].map(f32::to_bits),
                    [0.0_f32, 0.0, 0.6875, -2.75].map(f32::to_bits),
                )
            );
            assert_eq!(numeric.events[5], Event::Sqrt(0x3f80_0001));
            if signed {
                assert!(
                    matches!(numeric.events[6], Event::Dot(_, wind) if wind == Y.map(f32::to_bits))
                );
                assert_eq!(angle, TWO_PI - PI * 0.5);
            } else {
                assert_eq!(angle, PI * 0.5);
            }
        }
        for failure in 0..7 {
            let mut numeric = Numeric {
                fail_at: Some(failure),
                ..Numeric::default()
            };
            assert_eq!(
                try_signed_angle(X, Z, Y, &mut numeric),
                Err(Error::Injected(failure))
            );
            assert_eq!(numeric.events.len(), failure + 1);
        }
    }

    #[test]
    fn angle_normalization_has_fast_half_open_interval_and_checked_conversion() {
        for input in [-PI, -0.0, 0.0, 1.0] {
            let mut numeric = Numeric::default();
            assert_eq!(
                try_normalize_angle(input, &mut numeric).unwrap().to_bits(),
                input.to_bits()
            );
            assert!(numeric.events.is_empty());
        }
        let mut numeric = Numeric::default();
        assert_eq!(try_normalize_angle(PI, &mut numeric).unwrap(), -PI);
        assert!(matches!(numeric.events.as_slice(), [Event::Integer(_)]));
        for (input, expected) in [(4.0, 4.0 - TWO_PI), (-4.0, TWO_PI - 4.0)] {
            assert_eq!(
                try_normalize_angle(input, &mut HostReckoningNumeric).unwrap(),
                expected
            );
        }
        for input in [f32::NAN, f32::INFINITY, -f32::INFINITY, f32::MAX] {
            assert!(matches!(
                try_normalize_angle(input, &mut HostReckoningNumeric),
                Err(ReckoningHostNumericError::IntegerConversion { .. })
            ));
        }
        assert_eq!(
            HostReckoningNumeric.truncate_i32(-2_147_483_648.0).unwrap(),
            i32::MIN
        );
        assert!(HostReckoningNumeric.truncate_i32(2_147_483_648.0).is_err());
        assert_eq!(HostReckoningNumeric.truncate_i32(-1.75).unwrap(), -1);
    }

    #[test]
    fn full_lane_cross_keeps_negative_fma_rounding_residual_in_w() {
        let one_plus = f32::from_bits(0x3f80_0001);
        let a = [one_plus, one_plus, 0.0, one_plus];
        let b = [one_plus, one_plus, 0.0, one_plus];
        let cross = cross_full4(a, b);
        assert_eq!(cross[2].to_bits(), 0xa880_0000); // -2^-46, not split zero.
        assert_eq!(cross[3].to_bits(), 0xa880_0000);
        let mut exceptional = a;
        exceptional[3] = f32::NAN;
        let poisoned = cross_full4(exceptional, b);
        assert_eq!(
            poisoned[..3]
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            cross[..3].iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
        assert!(poisoned[3].is_nan());
    }

    #[test]
    fn normalization_preserves_full_fallback_and_scales_w_without_length_contamination() {
        let fallback = [1.0, 2.0, 3.0, f32::from_bits(0x7fc0_4321)];
        let mut numeric = Numeric::default();
        assert_eq!(
            try_normalize_safe([0.0; 4], fallback, &mut numeric)
                .unwrap()
                .map(f32::to_bits),
            fallback.map(f32::to_bits)
        );
        assert_eq!(numeric.events.len(), 1);
        let normal = try_normalize_unsafe([0.0, 0.0, 2.0, 8.0], &mut HostReckoningNumeric).unwrap();
        assert_eq!(normal, [0.0, 0.0, 1.0, 4.0]);
        let poisoned =
            try_normalize_unsafe([0.0, 0.0, 2.0, f32::NAN], &mut HostReckoningNumeric).unwrap();
        assert_eq!(poisoned[..3], normal[..3]);
        assert!(poisoned[3].is_nan());
        assert!(
            try_normalize_unsafe([0.0; 4], &mut HostReckoningNumeric)
                .unwrap()
                .iter()
                .all(|x| x.is_nan())
        );
        assert_eq!(
            normalize_from_squared_and_estimate(X, 1.0, 0.5)[0],
            0.868_774_4
        );
    }

    #[test]
    fn length_zero_still_estimates_and_clamp_retains_source_full_lane_multiplications() {
        let mut numeric = Numeric::default();
        assert_eq!(try_length([0.0; 4], &mut numeric).unwrap().to_bits(), 0);
        assert_eq!(numeric.events.len(), 2);
        assert_eq!(numeric.events[1], Event::Sqrt(0));
        let mut numeric = Numeric::default();
        assert!(matches!(
            try_clamp(X, -1.0, &mut numeric),
            Err(ReckoningClampError::NegativeMaximum { .. })
        ));
        assert!(numeric.events.is_empty());
        let mut numeric = Numeric::default();
        assert_eq!(
            try_clamp([2.0, 0.0, 0.0, 8.0], 1.0, &mut numeric).unwrap(),
            [1.0, 0.0, 0.0, 4.0]
        );
        assert!(matches!(
            numeric.events.as_slice(),
            [Event::Dot(..), Event::Sqrt(_), Event::Reciprocal(_)]
        ));
        let input = [0.0, 0.0, 0.0, -0.0];
        assert_eq!(
            try_clamp(input, 1.0, &mut HostReckoningNumeric)
                .unwrap()
                .map(f32::to_bits),
            input.map(f32::to_bits)
        );
        assert!(
            try_clamp(X, f32::NAN, &mut HostReckoningNumeric)
                .unwrap()
                .iter()
                .all(|x| x.is_nan())
        );
    }
}

#[test]
fn cross_vnmsub_negates_the_rounded_fused_zero() {
    let result = super::cross_full4([0.0, 1.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]);
    assert_eq!(result.map(f32::to_bits), [0x8000_0000; 4]);
}

#[test]
fn denormal_squared_length_is_zero_so_clamp_returns_the_settled_vector() {
    let settled = [1.0e-20, -1.0e-20, 0.0, 0.0];
    assert_eq!(host_dot3(settled, settled).to_bits(), 0);
    assert_eq!(host_dot4(settled, settled).to_bits(), 0);
    let mut numeric = HostReckoningNumeric;
    assert_eq!(try_length(settled, &mut numeric).unwrap().to_bits(), 0);
    assert_eq!(
        try_clamp(settled, 0.25, &mut numeric)
            .unwrap()
            .map(f32::to_bits),
        settled.map(f32::to_bits)
    );
    assert_eq!(vmx_flush(-1.0e-40).to_bits(), 0x8000_0000);
    assert_eq!(
        host_dot3([1.0e-40, 0.0, 0.0, 0.0], [1.0e30, 0.0, 0.0, 0.0]).to_bits(),
        0
    );
}
