use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric, reckoning_sin_cos::HostReckoningSinCosNumeric,
};

struct FailingTrig {
    calls: usize,
    fail: usize,
}
impl ReckoningSinCosNumeric for FailingTrig {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls += 1;
        if self.calls == self.fail {
            Err("trigonometry")
        } else {
            Ok(value.round_ties_even())
        }
    }
}

#[test]
fn scalar_instant_bypasses_trig_but_quaternion_instant_does_not() {
    let mut trig = FailingTrig { calls: 0, fail: 1 };
    assert_eq!(
        scalar(3.0, -0.0, 1, 0.5, &mut trig).unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(trig.calls, 0);
    assert!(matches!(
        quaternion(
            [0.0; 4],
            [1.0; 4],
            1,
            0.5,
            &mut HostReckoningNumeric,
            &mut trig
        ),
        Err(ShotMathError::Trigonometry("trigonometry"))
    ));
}

#[test]
fn normal_orientation_still_calls_zero_roll_sincos() {
    let mut trig = FailingTrig { calls: 0, fail: 3 };
    assert!(matches!(
        normal_orientation(0.0, 0.0, &mut trig),
        Err("trigonometry")
    ));
    assert_eq!(trig.calls, 3);
}

#[test]
fn negative_dot_flips_first_and_normalizes_near_path() {
    let output = quaternion(
        [0.0, 0.0, 0.0, -1.0],
        [0.0, 0.0, 0.0, 1.0],
        0,
        0.5,
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
    )
    .unwrap();
    assert_eq!(output[3].to_bits(), 1.0_f32.to_bits());
}

#[test]
fn unknown_style_preserves_first_after_easing_call() {
    let mut trig = FailingTrig {
        calls: 0,
        fail: usize::MAX,
    };
    let first = [1.0, 2.0, 3.0, -0.0];
    let result = quaternion(
        first,
        [0.0; 4],
        u32::MAX,
        0.5,
        &mut HostReckoningNumeric,
        &mut trig,
    )
    .unwrap();
    assert_eq!(result.map(f32::to_bits), first.map(f32::to_bits));
    assert_eq!(trig.calls, 1);
}
