use super::*;
struct Numeric {
    calls: usize,
    fail_second: bool,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(HostReckoningNumeric.dot3(a, b).unwrap())
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        Ok(HostReckoningNumeric.reciprocal_sqrt_estimate(x).unwrap())
    }
    fn reciprocal_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.calls += 1;
        if self.fail_second && self.calls == 2 {
            return Err("second reciprocal");
        }
        Ok(HostReckoningNumeric.reciprocal_estimate(x).unwrap())
    }
    fn truncate_i32(&mut self, x: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric
            .truncate_i32(x)
            .map_err(|_| "conversion")
    }
}
#[test]
fn heading_smoothing_issues_both_reciprocals_even_for_zero_delta() {
    let mut numeric = Numeric {
        calls: 0,
        fail_second: true,
    };
    assert_eq!(
        heading_smoothing(0.0, 0.0, 0.5, &mut numeric),
        Err("second reciprocal")
    );
    assert_eq!(numeric.calls, 2);
}
#[test]
fn unordered_heading_uses_fsel_cap_and_lt_clear_branch() {
    let actual = heading_smoothing(0.0, f32::NAN, f32::NAN, &mut HostReckoningNumeric).unwrap();
    assert_eq!(actual.to_bits(), 0x3f4c_cccd);
    let near = heading_smoothing(0.0, 0.0, 0.5, &mut HostReckoningNumeric).unwrap();
    assert_eq!(near.to_bits(), 0.5_f32.to_bits());
}
