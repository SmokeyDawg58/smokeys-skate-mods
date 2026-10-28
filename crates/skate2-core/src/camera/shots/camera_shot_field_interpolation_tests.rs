use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
struct Trig {
    calls: usize,
    fail: usize,
}
impl ReckoningSinCosNumeric for Trig {
    type Error = &'static str;
    fn round_nearest_even(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.calls += 1;
        if self.calls == self.fail {
            Err("trig")
        } else {
            Ok(x.round_ties_even())
        }
    }
}
#[test]
fn second_scalar_failure_keeps_first_field_and_later_options() {
    let mut first = CameraShot::constructed();
    let mut second = CameraShot::constructed();
    let mut output = CameraShot::constructed();
    first.distance = 2.0;
    second.distance = 4.0;
    second.lens_length = 100.0;
    second.use_free_stick = true;
    let old_lens = output.lens_length;
    let mut trig = Trig { calls: 0, fail: 2 };
    assert!(matches!(
        publish(
            &first,
            &second,
            &mut output,
            0.5,
            true,
            &mut HostReckoningNumeric,
            &mut trig
        ),
        Err(ShotMathError::Trigonometry("trig"))
    ));
    assert_ne!(output.distance.to_bits(), first.distance.to_bits());
    assert_eq!(output.lens_length.to_bits(), old_lens.to_bits());
    assert!(!output.use_free_stick);
    assert_eq!(trig.calls, 2);
}

#[test]
fn quaternion_failure_keeps_options_and_scalar_prefix_but_retains_orientation() {
    let first = CameraShot::constructed();
    let mut second = CameraShot::constructed();
    let mut output = CameraShot::constructed();
    second.use_free_stick = true;
    second.blur_transition = 13.0;
    second.compass_north = 3;
    let old = output.world_arm_orientation;
    // 6 primary scalars + 9 weights + board/elevation/roll/yaw/pitch +
    // blur/opacity/world heading = 23 scalar sine calls, then quaternion easing.
    let mut trig = Trig { calls: 0, fail: 24 };
    assert!(matches!(
        publish(
            &first,
            &second,
            &mut output,
            0.5,
            true,
            &mut HostReckoningNumeric,
            &mut trig
        ),
        Err(ShotMathError::Trigonometry("trig"))
    ));
    assert!(output.use_free_stick);
    assert_eq!(output.blur_transition.to_bits(), 13.0_f32.to_bits());
    assert_eq!(output.compass_north, 3);
    assert_eq!(
        output.world_arm_orientation.map(f32::to_bits),
        old.map(f32::to_bits)
    );
}
