#![expect(clippy::float_cmp, reason = "exact source arithmetic regression")]
use super::*;
use crate::reckoning_numeric::host_dot3;
struct Numeric {
    square: f32,
    roots: usize,
}
impl ReckoningNumeric for Numeric {
    type Error = ();
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, ()> {
        Ok(self.square)
    }
    fn reciprocal_estimate(&mut self, x: f32) -> Result<f32, ()> {
        Ok(1.0 / x)
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, ()> {
        self.roots += 1;
        Ok(1.0 / x.sqrt())
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, ()> {
        unreachable!()
    }
}
#[test]
fn collision_nan_time_asserts_and_unknown_w_survives() {
    let mut r = RetainedPredictionResult::in_sim_allocation();
    r.time = [f32::NAN; 4];
    assert!(matches!(
        collision_position::<(), ()>(&r),
        Err(SelectorError::CollisionTime(_))
    ));
    r.time = [0.5; 4];
    r.trajectory.position = V {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    };
    r.trajectory.velocity = V::known([2.0; 4]);
    r.trajectory.gravity = V::known([0.0; 4]);
    assert_eq!(
        collision_position::<(), ()>(&r).unwrap(),
        V {
            xyz: [2.0, 3.0, 4.0],
            w: None
        }
    );
}
#[test]
fn landing_velocity_uses_frame_not_fractional_time() {
    let mut r = RetainedPredictionResult::in_sim_allocation();
    r.time = [10.0; 4];
    r.frame = 30;
    r.trajectory.gravity = V::known([0.0, -10.0, 0.0, 0.0]);
    r.trajectory.velocity = V::known([0.0; 4]);
    assert_eq!(landing_velocity(&r).xyz[1], -5.0);
}
#[test]
fn unordered_maximum_and_squared_skip_length_estimate() {
    for (max, squared) in [(f32::NAN, 4.0), (1.0, f32::NAN)] {
        let mut t = RetainedTrajectory::constructed();
        t.velocity = V::known([0.0; 4]);
        let mut n = Numeric {
            square: squared,
            roots: 0,
        };
        adjust::<(), _>(&mut t, 60, V::known([2.0, 0.0, 0.0, 0.0]), max, &mut n).unwrap();
        assert_eq!(n.roots, 0);
        assert_eq!(t.velocity.xyz[0], 2.0);
    }
}
#[test]
fn negative_maximum_asserts_before_nonpositive_frame() {
    let mut t = RetainedTrajectory::constructed();
    let mut n = Numeric {
        square: host_dot3([0.0; 4], [0.0; 4]),
        roots: 0,
    };
    assert!(matches!(
        adjust::<(), _>(&mut t, 0, V::known([0.0; 4]), -1.0, &mut n),
        Err(SelectorError::NegativeMaximum(_))
    ));
}
