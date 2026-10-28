#![expect(
    clippy::float_cmp,
    reason = "exact retained quaternion copies and fixture transforms"
)]
use super::*;
use crate::reckoning_numeric::{HostReckoningNumeric, host_dot3};
use crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
use crate::skeleton_root_transforms::SkeletonRootVector;
use std::{cell::RefCell, rc::Rc};

fn matrix() -> SkeletonRootMatrix {
    SkeletonRootMatrix::translation(SkeletonRootVector::known([3.0, 5.0, 7.0, 0.0]))
}
fn active(q: [f32; 4]) -> SkateboardSteeringState {
    let mut state = SkateboardSteeringState::new();
    state.physics_error = q;
    state.physics_error_blending = true;
    state
}
struct Numeric {
    events: Rc<RefCell<Vec<&'static str>>>,
    reciprocals: usize,
    fail_reciprocal: Option<usize>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("dot");
        Ok(host_dot3(a, b))
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("rsqrt");
        Ok(1.0 / v.sqrt())
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("reciprocal");
        self.reciprocals += 1;
        if self.fail_reciprocal == Some(self.reciprocals) {
            Err("refused")
        } else {
            Ok(1.0 / v)
        }
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("floor is not truncate")
    }
}
struct Rounding(Rc<RefCell<Vec<&'static str>>>);
impl ReckoningSinCosNumeric for Rounding {
    type Error = &'static str;
    fn round_nearest_even(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.0.borrow_mut().push("trig");
        Ok(v.round_ties_even())
    }
}

#[test]
fn inactive_has_no_attribute_or_numeric_calls() {
    let mut state = SkateboardSteeringState::new();
    let result = state
        .try_blend_animated(
            matrix(),
            false,
            |_, _| -> Result<f32, ()> { panic!("unused graph") },
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    assert_eq!(result, matrix());
}

#[test]
fn threshold_and_unordered_complete_clear_only_flag() {
    for blend in [f32::from_bits(0x3f73_3333), 1.0, f32::NAN] {
        let q = [0.0, 0.6, 0.0, 0.8];
        let mut state = active(q);
        let result = state
            .try_blend_animated(
                matrix(),
                false,
                |fast, _| {
                    assert!(!fast);
                    Ok::<_, ()>(blend)
                },
                &mut HostReckoningNumeric,
                &mut HostReckoningSinCosNumeric,
            )
            .unwrap();
        assert!(!state.physics_error_blending);
        assert_eq!(state.physics_error, q);
        assert_eq!(result, matrix());
    }
}

#[test]
fn graph_failure_occurs_after_acos_and_preserves_owner() {
    let q = [0.0, 0.6, 0.0, 0.8];
    let mut state = active(q);
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut numeric = Numeric {
        events: events.clone(),
        reciprocals: 0,
        fail_reciprocal: None,
    };
    let result = state.try_blend_animated(
        matrix(),
        true,
        |fast, angle| {
            assert!(fast);
            assert!(angle > 1.0);
            assert!(!events.borrow().is_empty());
            Err::<f32, _>("graph")
        },
        &mut numeric,
        &mut HostReckoningSinCosNumeric,
    );
    assert!(matches!(
        result,
        Err(AnimatedBlendError::Attribute("graph"))
    ));
    assert_eq!(state.physics_error, q);
    assert!(state.physics_error_blending);
}

#[test]
fn second_reciprocal_failure_retains_quaternion_before_next_sine() {
    let q = [0.0, 0.6, 0.0, 0.8];
    let mut state = active(q);
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut numeric = Numeric {
        events: events.clone(),
        reciprocals: 0,
        fail_reciprocal: Some(2),
    };
    let result = state.try_blend_animated(
        matrix(),
        false,
        |_, _| Ok::<_, ()>(0.25),
        &mut numeric,
        &mut Rounding(events.clone()),
    );
    assert!(matches!(
        result,
        Err(AnimatedBlendError::Numeric("refused"))
    ));
    let e = events.borrow();
    assert_eq!(&e[e.len() - 2..], &["reciprocal", "reciprocal"]);
    assert_eq!(state.physics_error, q);
    assert!(state.physics_error_blending);
}

#[test]
fn spherical_branch_uses_both_estimates_then_each_sine_and_retains_unknown_w() {
    let mut state = active([0.0, 0.6, 0.0, 0.8]);
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut numeric = Numeric {
        events: events.clone(),
        reciprocals: 0,
        fail_reciprocal: None,
    };
    let mut columns = *matrix().columns();
    columns[0].w = None;
    let result = state
        .try_blend_animated(
            SkeletonRootMatrix::from_columns(columns),
            false,
            |_, _| Ok::<_, ()>(0.25),
            &mut numeric,
            &mut Rounding(events.clone()),
        )
        .unwrap();
    let e = events.borrow();
    let index = e.iter().position(|v| *v == "reciprocal").unwrap();
    assert_eq!(&e[index..], &["reciprocal", "reciprocal", "trig", "trig"]);
    assert!(result.columns().iter().all(|v| v.w.is_none()));
    assert!(state.physics_error[1] < 0.6);
    assert!(state.physics_error_blending);
}

#[test]
fn quaternion_identity_uses_linear_branch_and_keeps_translation() {
    let mut state = active([0.0, 0.0, 0.0, 1.0]);
    let result = state
        .try_blend_animated(
            matrix(),
            false,
            |_, _| Ok::<_, ()>(0.5),
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    assert_eq!(result.columns()[3].xyz, [3.0, 5.0, 7.0]);
    assert_eq!(state.physics_error, [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn composed_translation_executes_zero_times_nonfinite_axes() {
    let mut input = matrix();
    let mut x = input.columns()[0];
    x.xyz[0] = f32::INFINITY;
    input.set_column(0, x);
    let result = compose_error_rotation(input, [0.0, 0.0, 0.0, 1.0]);
    assert!(result.columns()[3].xyz[0].is_nan());
}
