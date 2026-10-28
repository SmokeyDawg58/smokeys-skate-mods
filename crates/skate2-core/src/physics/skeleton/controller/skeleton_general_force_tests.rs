use super::*;

struct Numeric {
    calls: Vec<u32>,
    seed: Result<f32, &'static str>,
}
impl Numeric {
    fn host() -> Self {
        Self {
            calls: Vec::new(),
            seed: Ok(f32::from_bits(0x426f_ffff)),
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push(value.to_bits());
        self.seed
    }
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("force suffix has no dot product")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("force suffix has no reciprocal square root")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("force suffix has no integer conversion")
    }
}

#[test]
fn gravity_uses_three_separate_products_and_known_four_lanes() {
    // Frozen ordinary-f32 graph; combining half*dt*dt produces bc2141b9.
    assert_eq!(UPDATE_TIMESTEP.to_bits(), 0x3c88_8889);
    assert_eq!(
        displacement_from_gravity(f32::from_bits(0x428d_bac2)).map(f32::to_bits),
        [0, 0xbc21_41b8, 0, 0]
    );
    assert_eq!(
        displacement_from_gravity(9.8).map(f32::to_bits),
        [0, 0xbab2_6751, 0, 0]
    );
    assert_eq!(
        displacement_from_gravity(-600.0).map(f32::to_bits),
        [0, 0x3daa_aaac, 0, 0]
    );
}

#[test]
fn gravity_negation_preserves_zero_sign_and_does_not_sanitize_nonfinite_y() {
    assert_eq!(
        displacement_from_gravity(0.0).map(f32::to_bits),
        [0, 0x8000_0000, 0, 0]
    );
    assert_eq!(displacement_from_gravity(-0.0).map(f32::to_bits), [0; 4]);
    assert_eq!(
        displacement_from_gravity(f32::INFINITY)[1].to_bits(),
        0xff80_0000
    );
    assert_eq!(
        displacement_from_gravity(f32::NEG_INFINITY)[1].to_bits(),
        0x7f80_0000
    );
    let nan = displacement_from_gravity(f32::NAN);
    assert!(nan[1].is_nan());
    assert_eq!([nan[0], nan[2], nan[3]].map(f32::to_bits), [0; 3]);
}

#[test]
fn per_body_graph_uses_supplied_inverse_mass_and_old_force() {
    let displacement = [0x3eaa_aaab, 0xbaaa_aaab, 0x40f1_2345, 0x7fc0_0123].map(f32::from_bits);
    let old = [0x3f87_6543, 0xc055_5555, 0x3eaa_aaab].map(f32::from_bits);
    let mut numeric = Numeric::host();
    let result =
        try_accumulate_force(displacement, f32::from_bits(0x3e13_cd3a), old, &mut numeric).unwrap();
    assert_eq!(
        result.map(f32::to_bits),
        [0x407c_732a, 0xc056_0e16, 0x4283_2fb2]
    );
    assert_eq!(numeric.calls, [0x3c88_8889]);
}

#[test]
fn both_refinements_execute_on_the_supplied_estimate() {
    let mut numeric = Numeric {
        calls: Vec::new(),
        seed: Ok(30.0),
    };
    let result = try_accumulate_force([1.0; 4], 1.0, [0.0; 3], &mut numeric).unwrap();
    // One refinement would be about45; host replacement would be about60.
    assert_eq!(result.map(f32::to_bits), [0x4260_ffff; 3]);
    assert_eq!(numeric.calls, [0x3c88_8889]);
}

#[test]
fn rounded_mass_product_and_separate_force_add_are_not_fused_or_combined() {
    let mut numeric = Numeric::host();
    let result = try_accumulate_force(
        [f32::from_bits(0x3c74_4998); 4],
        f32::from_bits(0x3e06_1385),
        [f32::from_bits(0x3c44_b923); 3],
        &mut numeric,
    )
    .unwrap();
    // Both FMA(scaled,mass,old) and combined reciprocal*mass yield3e043da5.
    assert_eq!(result.map(f32::to_bits), [0x3e04_3da6; 3]);
}

#[test]
fn numeric_failure_occurs_even_for_zero_mass_and_zero_displacement() {
    let mut numeric = Numeric {
        calls: Vec::new(),
        seed: Err("reciprocal"),
    };
    assert_eq!(
        try_accumulate_force([0.0; 4], 0.0, [17.0, 19.0, 23.0], &mut numeric),
        Err("reciprocal")
    );
    assert_eq!(numeric.calls, [0x3c88_8889]);
}

#[test]
fn zero_mass_does_not_suppress_nonfinite_arithmetic_or_estimation() {
    let mut numeric = Numeric::host();
    let result = try_accumulate_force(
        [f32::INFINITY, 1.0, 2.0, 0.0],
        0.0,
        [7.0, 8.0, 9.0],
        &mut numeric,
    )
    .unwrap();
    assert!(result[0].is_nan());
    assert_eq!(
        [result[1], result[2]].map(f32::to_bits),
        [8.0_f32.to_bits(), 9.0_f32.to_bits()]
    );
    assert_eq!(numeric.calls, [0x3c88_8889]);
}

#[test]
fn nonfinite_estimate_has_no_fallback_to_host_reciprocal() {
    for seed in [f32::INFINITY, f32::NAN] {
        let mut numeric = Numeric {
            calls: Vec::new(),
            seed: Ok(seed),
        };
        let result = try_accumulate_force([0.0; 4], 1.0, [0.0; 3], &mut numeric).unwrap();
        assert!(result.iter().all(|value| value.is_nan()));
        assert_eq!(numeric.calls, [0x3c88_8889]);
    }
}
