use super::*;
struct Numeric {
    rsqrt_calls: usize,
}
impl ReckoningNumeric for Numeric {
    type Error = crate::reckoning_numeric::ReckoningHostNumericError;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        HostReckoningNumeric.dot3(a, b)
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.rsqrt_calls += 1;
        HostReckoningNumeric.reciprocal_sqrt_estimate(x)
    }
    fn reciprocal_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric.reciprocal_estimate(x)
    }
    fn truncate_i32(&mut self, x: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric.truncate_i32(x)
    }
}
#[test]
fn low_speed_uses_k_and_high_speed_recomputes_normalization() {
    let acceleration = SkeletonRootVector::known([7.0, 0.0, 2.0, f32::NAN]);
    let mut numeric = Numeric { rsqrt_calls: 0 };
    let low = acceleration_along_velocity(ZERO, acceleration, &mut numeric).unwrap();
    assert_eq!(low.to_bits(), 2.0_f32.to_bits());
    assert_eq!(numeric.rsqrt_calls, 1);
    numeric.rsqrt_calls = 0;
    let high = acceleration_along_velocity(
        SkeletonRootVector {
            xyz: [4.0, 0.0, 0.0],
            w: None,
        },
        acceleration,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(high.to_bits(), 7.0_f32.to_bits());
    assert_eq!(numeric.rsqrt_calls, 2);
}
#[test]
fn velocity_gate_ignores_w_and_nan_lanes_but_accepts_other_xyz() {
    assert!(!velocity_exceeds_epsilon(SkeletonRootVector::known([
        0.0,
        f32::NAN,
        0.0,
        1.0
    ])));
    assert!(velocity_exceeds_epsilon(SkeletonRootVector {
        xyz: [f32::NAN, 0.0, 1.0],
        w: None
    }));
    assert_eq!(
        horizontal(SkeletonRootVector {
            xyz: [2.0, 9.0, 3.0],
            w: None
        }),
        SkeletonRootVector::known([2.0, 0.0, 3.0, 2.0])
    );
}
#[test]
fn teleport_comparison_ignores_previous_height_but_preserves_real_target() {
    let (target, delta) = anchor_target_and_difference(
        SkeletonRootVector::known([2.0, 30.0, 4.0, 9.0]),
        7.0,
        SkeletonRootVector {
            xyz: [2.0, -1000.0, 4.0],
            w: None,
        },
    );
    assert_eq!(target, SkeletonRootVector::known([2.0, 7.0, 4.0, 0.0]));
    assert_eq!(delta.xyz.map(f32::to_bits), [0.0_f32.to_bits(); 3]);
    assert!(delta.w.is_none());
}
#[test]
fn breadcrumb_strict_distance_gate_and_full_lane_ring_publication() {
    let mut ring = CameraBreadcrumbs {
        positions: vec![ZERO; 2],
        count: 2,
        last: 0,
        minimum_distance_squared: 1.0,
    };
    ring.try_set_position(
        SkeletonRootVector::known([1.0, 0.0, 0.0, 8.0]),
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(ring.last, 0);
    let first = SkeletonRootVector {
        xyz: [2.0, 0.0, 0.0],
        w: None,
    };
    ring.try_set_position(first, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(ring.last, 1);
    assert_eq!(ring.positions[1], first);
    let second = SkeletonRootVector::known([4.0, 0.0, 0.0, 7.0]);
    ring.try_set_position(second, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(ring.last, 0);
    assert_eq!(ring.positions[0], second);
    ring.try_set_position(
        SkeletonRootVector::known([f32::NAN; 4]),
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(ring.last, 0);
    assert_eq!(ring.positions[0], second);
}
