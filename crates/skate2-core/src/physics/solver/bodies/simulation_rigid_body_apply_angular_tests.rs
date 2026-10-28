use super::{
    ApplyAngularError, SimulationRigidBodyObservation, SimulationRigidBodyPool, SkeletonRootVector,
};
use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, host_dot3},
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
fn fixture() -> (SimulationRigidBodyPool, SimulationRigidBodyObservation) {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let body = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    let observed = pool.try_observe(body).unwrap();
    pool.slots[0].inverse_inertia_full = [2.0, 1.0, 0.5];
    pool.slots[0].inverse_inertia_split = [4.0, 3.0, 0.25];
    pool.slots[0].omega = [120.0, 240.0, 360.0];
    pool.slots[0].torque = [17.0, 19.0, 23.0];
    pool.slots[0].cooldown = 71;
    (pool, observed)
}
fn request() -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    }
}
#[test]
fn actual_tensor_direct_displacement_is_not_clamped_against_omega() {
    let (mut pool, body) = fixture();
    pool.try_apply_skateboard_angular_displacement(body, request(), &mut HostReckoningNumeric)
        .unwrap();
    for (actual, expected) in pool.slots[0]
        .torque
        .into_iter()
        .zip([3617.0, 7219.0, 10823.0])
    {
        assert!((actual - expected).abs() < 0.02);
    }
    assert_eq!(
        pool.slots[0].omega.map(f32::to_bits),
        [120.0_f32, 240.0, 360.0].map(f32::to_bits)
    );
    assert_eq!(pool.slots[0].cooldown, 0);
}
struct Numeric {
    events: Vec<&'static str>,
    count: usize,
    fail_last: bool,
    zero_last: bool,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.events.push("determinant dot");
        Ok(host_dot3(a, b))
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.count += 1;
        self.events.push(if self.count == 2 {
            "determinant estimate"
        } else {
            "DT estimate"
        });
        if self.count == 3 && self.fail_last {
            return Err("last estimate");
        }
        Ok(if self.count == 3 && self.zero_last {
            0.0
        } else {
            1.0 / v
        })
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("ApplyAngular does not normalize or project")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no integer conversion")
    }
}
#[test]
fn preserves_two_distinct_dt_estimates_and_determinant_call_order() {
    let (mut pool, body) = fixture();
    let mut n = Numeric {
        events: vec![],
        count: 0,
        fail_last: false,
        zero_last: true,
    };
    pool.try_apply_skateboard_angular_displacement(body, request(), &mut n)
        .unwrap();
    assert_eq!(
        n.events,
        [
            "DT estimate",
            "determinant dot",
            "determinant estimate",
            "DT estimate"
        ]
    );
    assert_eq!(
        pool.slots[0].torque.map(f32::to_bits),
        [17.0_f32, 19.0, 23.0].map(f32::to_bits)
    );
}
#[test]
fn last_estimate_failure_preserves_torque_and_cooldown() {
    let (mut pool, body) = fixture();
    let mut n = Numeric {
        events: vec![],
        count: 0,
        fail_last: true,
        zero_last: false,
    };
    assert!(matches!(
        pool.try_apply_skateboard_angular_displacement(body, request(), &mut n),
        Err(ApplyAngularError::Numeric("last estimate"))
    ));
    assert_eq!(
        pool.slots[0].torque.map(f32::to_bits),
        [17.0_f32, 19.0, 23.0].map(f32::to_bits)
    );
    assert_eq!(pool.slots[0].cooldown, 71);
}
#[test]
fn request_w_is_not_a_tensor_xyz_dependency() {
    let (mut first, a) = fixture();
    let (mut second, b) = fixture();
    first
        .try_apply_skateboard_angular_displacement(a, request(), &mut HostReckoningNumeric)
        .unwrap();
    let mut value = request();
    value.w = Some(f32::NAN);
    second
        .try_apply_skateboard_angular_displacement(b, value, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(
        first.slots[0].torque.map(f32::to_bits),
        second.slots[0].torque.map(f32::to_bits)
    );
}
