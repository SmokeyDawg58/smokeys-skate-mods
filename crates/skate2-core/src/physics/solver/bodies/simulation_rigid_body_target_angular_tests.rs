use super::{SimulationRigidBodyPool, SkeletonRootVector, TargetAngularError};
use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, host_dot3},
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
fn fixture(omega: [f32; 3]) -> (SimulationRigidBodyPool, SimulationRigidBodyObservation) {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let body = pool
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
        .unwrap()
        .unwrap();
    let observation = pool.try_observe(body).unwrap();
    let slot = &mut pool.slots[0];
    slot.omega = omega;
    slot.inverse_inertia_full = [2.0, 0.0, 0.0];
    slot.inverse_inertia_split = [8.0, 4.0, 0.0];
    slot.torque = [17.0, 19.0, 23.0];
    slot.cooldown = 71;
    (pool, observation)
}
fn request() -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [1.0, 0.0, 0.0],
        w: None,
    }
}

#[test]
fn exact_target_brakes_overspeed_and_preserves_determinant_first_numeric_order() {
    for (omega, expected_x) in [(120.0, -3583.0), (-120.0, 10817.0)] {
        let (mut pool, body) = fixture([omega, 0., 0.]);
        let mut numeric = Numeric {
            reciprocals: Vec::new(),
            fail_at: None,
            zero_second_dt: false,
        };
        pool.try_target_skateboard_angular_displacement_exact(body, request(), &mut numeric)
            .unwrap();
        assert_eq!(
            numeric.reciprocals,
            [64.0f32.to_bits(), 0x3c888889, 0x3c888889]
        );
        assert!((pool.slots[0].torque[0] - expected_x).abs() < 0.01);
        assert_eq!(pool.slots[0].omega, [omega, 0., 0.]);
        assert_eq!(pool.slots[0].cooldown, 0);
    }
}

#[test]
fn exact_target_failure_before_final_tensor_product_preserves_body() {
    let (mut pool, body) = fixture([120., 0., 0.]);
    let mut numeric = Numeric {
        reciprocals: Vec::new(),
        fail_at: Some(3),
        zero_second_dt: false,
    };
    assert!(matches!(
        pool.try_target_skateboard_angular_displacement_exact(body, request(), &mut numeric),
        Err(TargetAngularError::Numeric("late reciprocal"))
    ));
    assert_eq!(pool.slots[0].torque, [17., 19., 23.]);
    assert_eq!(pool.slots[0].cooldown, 71);
}

#[test]
fn overspeed_clamps_to_zero_but_opposing_rotation_keeps_original_request() {
    let (mut pool, body) = fixture([120.0, 0.0, 0.0]);
    pool.try_target_skateboard_angular_displacement(body, request(), &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(
        pool.slots[0].torque.map(f32::to_bits),
        [17.0f32, 19.0, 23.0].map(f32::to_bits)
    );
    assert_eq!(pool.slots[0].cooldown, 0);
    let (mut pool, body) = fixture([-120.0, 0.0, 0.0]);
    pool.try_target_skateboard_angular_displacement(body, request(), &mut HostReckoningNumeric)
        .unwrap();
    assert!((pool.slots[0].torque[0] - 3617.0).abs() < 0.01);
    assert_eq!(
        pool.slots[0].omega.map(f32::to_bits),
        [-120.0f32, 0.0, 0.0].map(f32::to_bits)
    );
}

struct Numeric {
    reciprocals: Vec<u32>,
    fail_at: Option<usize>,
    zero_second_dt: bool,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(host_dot3(a, b))
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        Ok(1.0 / v.sqrt())
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.reciprocals.push(v.to_bits());
        if self.fail_at == Some(self.reciprocals.len()) {
            return Err("late reciprocal");
        }
        if self.zero_second_dt && self.reciprocals.len() == 3 {
            return Ok(0.0);
        }
        Ok(1.0 / v)
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("source contains no integer conversion")
    }
}
#[test]
fn second_fixed_dt_estimate_is_fresh_and_not_algebraically_cancelled() {
    let (mut pool, body) = fixture([0.0; 3]);
    let mut numeric = Numeric {
        reciprocals: Vec::new(),
        fail_at: None,
        zero_second_dt: true,
    };
    pool.try_target_skateboard_angular_displacement(body, request(), &mut numeric)
        .unwrap();
    assert_eq!(
        numeric.reciprocals,
        [0x3c88_8889, 64.0f32.to_bits(), 0x3c88_8889]
    );
    assert_eq!(
        pool.slots[0].torque.map(f32::to_bits),
        [17.0f32, 19.0, 23.0].map(f32::to_bits)
    );
}
#[test]
fn last_numeric_failure_commits_neither_torque_nor_cooldown() {
    let (mut pool, body) = fixture([0.0; 3]);
    let mut numeric = Numeric {
        reciprocals: Vec::new(),
        fail_at: Some(3),
        zero_second_dt: false,
    };
    assert!(matches!(
        pool.try_target_skateboard_angular_displacement(body, request(), &mut numeric),
        Err(TargetAngularError::Numeric("late reciprocal"))
    ));
    assert_eq!(
        pool.slots[0].torque.map(f32::to_bits),
        [17.0f32, 19.0, 23.0].map(f32::to_bits)
    );
    assert_eq!(pool.slots[0].cooldown, 71);
}
#[test]
fn off_diagonal_cached_tensor_is_inverted_and_applied_without_reading_source_inertia() {
    let (mut pool, body) = fixture([0.0; 3]);
    // Source Inertia is None; only the real body's current cached tensor matters.
    pool.slots[0].inverse_inertia_full = [2.0, 1.0, 0.5];
    pool.slots[0].inverse_inertia_split = [4.0, 3.0, 0.25];
    let input = SkeletonRootVector {
        xyz: [1.0, 2.0, 3.0],
        w: Some(f32::NAN),
    };
    pool.try_target_skateboard_angular_displacement(body, input, &mut HostReckoningNumeric)
        .unwrap();
    for (actual, expected) in pool.slots[0]
        .torque
        .into_iter()
        .zip([3617.0, 7219.0, 10823.0])
    {
        assert!((actual - expected).abs() < 0.01);
    }
}
