use super::*;

#[derive(Default)]
struct Calls(Vec<(&'static str, f32)>);
impl ReckoningNumeric for Calls {
    type Error = ();
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, ()> {
        unreachable!("matrix helper forms its two-product horizontal square directly")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, ()> {
        self.0.push(("rsqrt", value));
        Ok(value.sqrt().recip())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, ()> {
        self.0.push(("reciprocal", value));
        Ok(value.recip())
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, ()> {
        unreachable!("matrix helper and XMVectorATan have no scalar integer conversion")
    }
}

#[test]
fn normal_branch_issues_three_ordered_atans_and_copies_pitch_into_w() {
    let basis = [
        Vector::known([0.0, 1.0, 0.0, f32::NAN]),
        Vector::known([-1.0, 0.0, 0.0, f32::NAN]),
        Vector::known([0.0, 0.0, 1.0, f32::NAN]),
    ];
    let mut calls = Calls::default();
    let euler = euler_from_matrix33(&basis, &mut calls).unwrap();
    assert!(euler.xyz[0].abs() < 0.0001);
    assert!(euler.xyz[1].abs() < 0.0001);
    assert!((euler.xyz[2] - f32::from_bits(0x3fc9_0fdb)).abs() < 0.0001);
    assert_eq!(euler.w.unwrap().to_bits(), euler.xyz[0].to_bits());
    assert_eq!(calls.0[0].0, "rsqrt");
    assert_eq!(calls.0.len(), 13); // one rsqrt, then 3 × (ratio + 3 atan reciprocals)
}

#[test]
fn near_vertical_branch_issues_two_atans_and_forces_positive_zero_z() {
    let basis = [
        Vector::known([0.0, 0.0, -1.0, f32::NAN]),
        Vector::known([0.0, 1.0, 0.0, f32::NAN]),
        Vector::known([1.0, 0.0, 0.0, f32::NAN]),
    ];
    let mut calls = Calls::default();
    let euler = euler_from_matrix33(&basis, &mut calls).unwrap();
    assert!(euler.xyz[0].abs() < 0.0001);
    assert!((euler.xyz[1] - f32::from_bits(0x3fc9_0fdb)).abs() < 0.0001);
    assert_eq!(euler.xyz[2].to_bits(), 0);
    assert_eq!(euler.w.unwrap().to_bits(), euler.xyz[0].to_bits());
    assert_eq!(calls.0[0].0, "rsqrt");
    assert_eq!(calls.0.len(), 9);
}

#[test]
fn negative_denominator_adds_signed_pi_after_atan() {
    let basis = [
        Vector::known([-1.0, 0.0, 0.0, 0.0]),
        Vector::known([0.0, 0.0, 0.0, 0.0]),
        Vector::known([0.0, 0.0, -1.0, 0.0]),
    ];
    let euler = euler_from_matrix33(&basis, &mut Calls::default()).unwrap();
    assert!((euler.xyz[0] - f32::from_bits(0x4049_0fdb)).abs() < 0.0001);
    assert!((euler.xyz[2] - f32::from_bits(0x4049_0fdb)).abs() < 0.0001);
}

#[test]
fn unordered_horizontal_length_takes_near_vertical_branch() {
    let basis = [
        Vector::known([f32::NAN, 0.0, -1.0, 0.0]),
        Vector::known([0.0, 1.0, 0.0, 0.0]),
        Vector::known([1.0, 0.0, 0.0, 0.0]),
    ];
    let mut calls = Calls::default();
    let result = euler_from_matrix33(&basis, &mut calls).unwrap();
    assert_eq!(result.xyz[2].to_bits(), 0);
    assert_eq!(calls.0.len(), 9);
}

#[test]
fn normal_and_near_vertical_ratio_denominators_follow_source_call_order() {
    let normal = [
        Vector::known([0.6, 0.8, -0.2, 0.0]),
        Vector::known([0.0, 2.0, 0.3, 0.0]),
        Vector::known([0.0, 0.0, 0.4, 0.0]),
    ];
    let mut calls = Calls::default();
    euler_from_matrix33(&normal, &mut calls).unwrap();
    assert!((calls.0[1].1 - 0.6).abs() < 0.0001); // Z denominator: X.x
    assert!((calls.0[5].1 - 1.0).abs() < 0.0001); // Y denominator: horizontal
    assert!((calls.0[9].1 - 0.4).abs() < 0.0001); // X denominator: Z.z

    let near = [
        Vector::known([0.0, 0.0, -1.0, 0.0]),
        Vector::known([0.0, 2.0, 0.0, 0.0]),
        Vector::known([1.0, 3.0, 0.0, 0.0]),
    ];
    let mut calls = Calls::default();
    euler_from_matrix33(&near, &mut calls).unwrap();
    assert_eq!(calls.0[1].1.to_bits(), 0); // Y denominator: horizontal zero
    assert_eq!(calls.0[5].1.to_bits(), 2.0_f32.to_bits()); // X: Y.y
}
