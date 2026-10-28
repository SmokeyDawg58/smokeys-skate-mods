use super::*;
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Trace {
    events: Vec<&'static str>,
    fail: Option<(&'static str, usize)>,
}
#[derive(Clone, Default)]
struct Log(Rc<RefCell<Trace>>);
impl Log {
    fn event(&self, name: &'static str) -> Result<(), &'static str> {
        let mut log = self.0.borrow_mut();
        log.events.push(name);
        let count = log.events.iter().filter(|&&e| e == name).count();
        if log.fail == Some((name, count)) {
            Err(name)
        } else {
            Ok(())
        }
    }
    fn fail(&self, name: &'static str, count: usize) {
        self.0.borrow_mut().fail = Some((name, count));
    }
    fn events(&self) -> Vec<&'static str> {
        self.0.borrow().events.clone()
    }
}
struct Input {
    log: Log,
    normal: SkeletonRootVector,
    velocity: SkeletonRootVector,
    right: SkeletonRootVector,
    hook: SkeletonRootVector,
    surface_velocity: SkeletonRootVector,
    axis: SkeletonRootVector,
    up: SkeletonRootVector,
    collision: bool,
    mass: f32,
    dt: f32,
}
macro_rules! vector_get {
    ($method:ident,$field:ident) => {
        fn $method(&mut self) -> Result<SkeletonRootVector, Self::Error> {
            self.log.event(stringify!($method))?;
            Ok(self.$field)
        }
    };
}
impl SkateboardSlideCollisionInputRead for Input {
    type Surface = u64;
    type Error = &'static str;
    vector_get!(ground_normal, normal);
    vector_get!(velocity, velocity);
    vector_get!(deck_right, right);
    vector_get!(reckoning_ground_normal, normal);
    vector_get!(collision_hook_to_hips, hook);
    vector_get!(surface_velocity, surface_velocity);
    vector_get!(z_axis_velocity_dir, axis);
    vector_get!(up_vector, up);
    fn surface_speed(&mut self) -> Result<f32, Self::Error> {
        self.log.event("surface_speed")?;
        Ok(5.0)
    }
    fn surface_collection(&mut self) -> Result<u64, Self::Error> {
        self.log.event("surface_collection")?;
        Ok(19)
    }
    fn wheel_hardness(&mut self) -> Result<f32, Self::Error> {
        self.log.event("wheel_hardness")?;
        Ok(0.25)
    }
    fn is_skeleton_in_collision(&mut self) -> Result<bool, Self::Error> {
        self.log.event("collision_flag")?;
        Ok(self.collision)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.log.event("time_delta")?;
        Ok(self.dt)
    }
    fn skateboard_mass(&mut self) -> Result<f32, Self::Error> {
        self.log.event("mass")?;
        Ok(self.mass)
    }
}
struct Attributes {
    log: Log,
    wheel: f32,
    limit: f32,
    response: f32,
    normal_speed: f32,
    curve_inputs: Vec<(SlideCollisionCurve, f32)>,
}
impl SkateboardSlideCollisionAttributeRead for Attributes {
    type Global = SlideCollisionGlobal;
    type Pointer = u64;
    type Surface = u64;
    type Layout = u64;
    type Error = &'static str;
    fn capture_global(&mut self, g: Self::Global) -> Result<Self::Global, Self::Error> {
        self.log.event(match g {
            SlideCollisionGlobal::Friction => "capture_friction",
            SlideCollisionGlobal::Heading => "capture_heading",
            SlideCollisionGlobal::Wheels => "capture_wheels",
            SlideCollisionGlobal::Collision => "capture_collision",
        })?;
        Ok(g)
    }
    fn graph(
        &mut self,
        _: &Self::Global,
        c: SlideCollisionCurve,
        x: f32,
    ) -> Result<f32, Self::Error> {
        self.log.event(match c {
            SlideCollisionCurve::SlideVsNormal => "normal_curve",
            SlideCollisionCurve::SlideVsSpeed => "speed_curve",
            SlideCollisionCurve::SideFrictionFactor => "heading_curve",
            SlideCollisionCurve::CollisionTorqueVsAngle => "torque_curve",
        })?;
        self.curve_inputs.push((c, x));
        Ok(match c {
            SlideCollisionCurve::SlideVsNormal => 0.25,
            SlideCollisionCurve::SlideVsSpeed | SlideCollisionCurve::SideFrictionFactor => 0.5,
            SlideCollisionCurve::CollisionTorqueVsAngle => 2.0,
        })
    }
    fn lookup(&mut self, _: &Self::Global, key: u64) -> Result<u64, Self::Error> {
        self.log.event("lookup")?;
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        self.log.event("float")?;
        Ok(match *key {
            WHEEL_HARDNESS_BLEND => self.wheel,
            COLLISION_LIMIT => self.limit,
            COLLISION_LOCAL_Y => -0.05,
            COLLISION_RESPONSE => self.response,
            COLLISION_NORMAL_SPEED => self.normal_speed,
            _ => panic!("unexpected key"),
        })
    }
    fn capture_surface_layout(&mut self, surface: &u64) -> Result<u64, Self::Error> {
        self.log.event("capture_surface_layout")?;
        Ok(*surface)
    }
    fn layout_float(&mut self, layout: &u64, field: SlideSurfaceField) -> Result<f32, Self::Error> {
        self.surface_float(layout, field)
    }
    fn surface_float(
        &mut self,
        surface: &u64,
        field: SlideSurfaceField,
    ) -> Result<f32, Self::Error> {
        assert_eq!(*surface, 19);
        self.log.event(match field {
            SlideSurfaceField::StraightenOutForceTime => "force_time",
            SlideSurfaceField::SideFrictionScalar => "surface_scalar",
            SlideSurfaceField::BrakingScalar => "braking_scalar",
            SlideSurfaceField::WheelDynamicFriction => "dynamic_friction",
            SlideSurfaceField::WheelStaticFriction => "static_friction",
            SlideSurfaceField::SlideOrientationStrength | SlideSurfaceField::SlideAngularDamping => panic!("collision calculator must not sample slide orientation tuning"),
        })?;
        Ok(match field {
            SlideSurfaceField::StraightenOutForceTime => 2.0,
            SlideSurfaceField::SideFrictionScalar => 4.0,
            SlideSurfaceField::BrakingScalar => 7.0,
            SlideSurfaceField::WheelDynamicFriction => 0.3,
            SlideSurfaceField::WheelStaticFriction => 0.5,
            SlideSurfaceField::SlideOrientationStrength | SlideSurfaceField::SlideAngularDamping => unreachable!("orientation tuning rejected above"),
        })
    }
}
struct Numeric {
    log: Log,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.log.event("dot")?;
        Ok(crate::reckoning_numeric::host_dot3(a, b))
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.log.event("rsqrt")?;
        Ok(1.0 / v.sqrt())
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.log.event("reciprocal")?;
        Ok(1.0 / v)
    }
    fn truncate_i32(&mut self, v: f32) -> Result<i32, Self::Error> {
        self.log.event("truncate")?;
        if !v.is_finite() {
            return Err("nonfinite truncate");
        }
        crate::reckoning_numeric::HostReckoningNumeric
            .truncate_i32(v)
            .map_err(|_| "nonfinite truncate")
    }
}
fn v(x: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector::known(x)
}
fn fixture() -> (Log, Input, Attributes, Numeric) {
    let log = Log::default();
    (
        log.clone(),
        Input {
            log: log.clone(),
            normal: v([0.0, 1.0, 0.0, 0.0]),
            velocity: v([2.0, 3.0, 4.0, 7.0]),
            right: v([1.0, 0.0, 0.0, 2.0]),
            hook: v([1.0, 0.0, 0.0, 0.0]),
            surface_velocity: v([-2.0, 0.0, 3.0, 7.0]),
            axis: v([1.0, 0.0, 0.0, 0.0]),
            up: v([0.0, 1.0, 0.0, 3.0]),
            collision: true,
            mass: 2.0,
            dt: 0.5,
        },
        Attributes {
            log: log.clone(),
            wheel: 2.0,
            limit: 1.0,
            response: 0.5,
            normal_speed: 1.0,
            curve_inputs: vec![],
        },
        Numeric { log },
    )
}
fn marked() -> SkateboardFootForce {
    SkateboardFootForce {
        world_space_force: v([11.0; 4]),
        local_space_position: v([13.0; 4]),
    }
}
fn before(events: &[&str], a: &str, b: &str) {
    assert!(
        events.iter().position(|&x| x == a).unwrap() < events.iter().position(|&x| x == b).unwrap(),
        "{events:?}"
    );
}

#[test]
fn slide_uses_side_projection_fused_hardness_and_current_surface_timing() {
    let (log, mut input, mut attributes, mut numeric) = fixture();
    let mut out = marked();
    SkateboardSlideCollisionServices {
        input: &mut input,
        attributes: &mut attributes,
        numeric: &mut numeric,
    }
    .try_slide(1.0, &mut out)
    .unwrap();
    assert_eq!(
        out.world_space_force.xyz.map(f32::to_bits),
        [-5.25_f32, -0.0, -0.0].map(f32::to_bits)
    );
    assert_eq!(out.world_space_force.w, Some(-10.5));
    assert_eq!(out.local_space_position, ZERO);
    let e = log.events();
    assert_eq!(
        &e[..6],
        &[
            "ground_normal",
            "velocity",
            "dot",
            "ground_normal",
            "deck_right",
            "dot"
        ]
    );
    before(&e, "capture_heading", "surface_collection");
    before(&e, "capture_wheels", "wheel_hardness");
    assert_eq!(
        attributes.curve_inputs[2],
        (SlideCollisionCurve::SideFrictionFactor, 0.5)
    );
}

#[test]
fn slide_first_dot_and_late_surface_failures_preserve_output_and_reads() {
    for (name, count) in [("dot", 1), ("surface_scalar", 1)] {
        let (log, mut input, mut attributes, mut numeric) = fixture();
        log.fail(name, count);
        let mut out = marked();
        assert!(
            SkateboardSlideCollisionServices {
                input: &mut input,
                attributes: &mut attributes,
                numeric: &mut numeric
            }
            .try_slide(1.0, &mut out)
            .is_err()
        );
        assert_eq!(out, marked());
        if name == "dot" {
            assert_eq!(log.events(), ["ground_normal", "velocity", "dot"]);
        }
    }
}

#[test]
fn slide_negative_wheel_asserts_but_nan_takes_native_nonnegative_test_arm() {
    for wheel in [-1.0, f32::NAN] {
        let (log, mut input, mut attributes, mut numeric) = fixture();
        attributes.wheel = wheel;
        let mut out = marked();
        let result = SkateboardSlideCollisionServices {
            input: &mut input,
            attributes: &mut attributes,
            numeric: &mut numeric,
        }
        .try_slide(1.0, &mut out);
        if wheel < 0.0 {
            assert!(matches!(
                result,
                Err(SkateboardSlideCollisionError::NegativeWheelBlend { .. })
            ));
            assert_eq!(out, marked());
            assert!(!log.events().contains(&"surface_scalar"));
        } else {
            result.unwrap();
            assert!(out.world_space_force.xyz[0].is_nan());
        }
    }
}

#[test]
fn slide_unknown_right_w_remains_unknown() {
    let (_, mut input, mut attributes, mut numeric) = fixture();
    input.right.w = None;
    let mut out = marked();
    SkateboardSlideCollisionServices {
        input: &mut input,
        attributes: &mut attributes,
        numeric: &mut numeric,
    }
    .try_slide(-0.0, &mut out)
    .unwrap();
    assert_eq!(out.world_space_force.w, None);
    assert_eq!(attributes.curve_inputs[2].1.to_bits(), 0);
}

#[test]
fn collision_scales_using_unscaled_impulse_length_and_keeps_full_lane_outputs() {
    let (log, mut input, mut attributes, mut numeric) = fixture();
    let mut out = marked();
    let mut torque = v([17.0; 4]);
    let mut new = v([19.0; 4]);
    let active = SkateboardSlideCollisionServices {
        input: &mut input,
        attributes: &mut attributes,
        numeric: &mut numeric,
    }
    .try_collision(&mut out, &mut torque, Some(&mut new))
    .unwrap();
    assert!(active);
    assert_eq!(new, v([1.0, 0.0, 3.0, 7.0]));
    assert_eq!(
        out.world_space_force.xyz.map(f32::to_bits),
        [4.0_f32, 0.0, 0.0].map(f32::to_bits)
    );
    assert_eq!(out.local_space_position, v([0.0, -0.05, 0.0, 0.0]));
    assert_eq!(torque, v([0.0, 2.0, 0.0, 6.0]));
    let e = log.events();
    let i = e
        .iter()
        .position(|&x| x == "reckoning_ground_normal")
        .unwrap();
    assert_eq!(
        &e[i..i + 4],
        &[
            "reckoning_ground_normal",
            "collision_hook_to_hips",
            "dot",
            "surface_velocity"
        ]
    );
    before(&e, "time_delta", "mass");
    before(&e, "mass", "reciprocal");
}

#[test]
fn collision_early_false_never_touches_outputs_or_late_services() {
    for flag in [false, true] {
        let (log, mut input, mut attributes, mut numeric) = fixture();
        input.collision = flag;
        input.hook = ZERO;
        let mut out = marked();
        let mut torque = v([17.0; 4]);
        let mut new = v([19.0; 4]);
        assert!(
            !SkateboardSlideCollisionServices {
                input: &mut input,
                attributes: &mut attributes,
                numeric: &mut numeric
            }
            .try_collision(&mut out, &mut torque, Some(&mut new))
            .unwrap()
        );
        assert_eq!(out, marked());
        assert_eq!(torque, v([17.0; 4]));
        assert_eq!(new, v([19.0; 4]));
        assert!(!log.events().contains(&"reciprocal"));
        if !flag {
            assert_eq!(log.events(), ["collision_flag"]);
        }
    }
}

#[test]
fn collision_first_dot_failure_does_not_read_surface_velocity() {
    let (log, mut input, mut attributes, mut numeric) = fixture();
    log.fail("dot", 1);
    let mut out = marked();
    let mut torque = ZERO;
    assert!(
        SkateboardSlideCollisionServices {
            input: &mut input,
            attributes: &mut attributes,
            numeric: &mut numeric
        }
        .try_collision(&mut out, &mut torque, None)
        .is_err()
    );
    assert!(!log.events().contains(&"surface_velocity"));
    assert_eq!(out, marked());
}

#[test]
fn collision_late_false_still_reciprocates_and_publishes_torque() {
    let (log, mut input, mut attributes, mut numeric) = fixture();
    input.surface_velocity = v([2.0, 0.0, 3.0, 0.0]);
    let mut out = marked();
    let mut torque = ZERO;
    assert!(
        !SkateboardSlideCollisionServices {
            input: &mut input,
            attributes: &mut attributes,
            numeric: &mut numeric
        }
        .try_collision(&mut out, &mut torque, None)
        .unwrap()
    );
    assert_eq!(out.world_space_force, ZERO);
    assert_eq!(torque, v([0.0, 2.0, 0.0, 6.0]));
    assert!(log.events().contains(&"reciprocal"));
    assert!(log.events().contains(&"torque_curve"));
}

#[test]
fn collision_reciprocal_failure_keeps_only_earlier_optional_velocity() {
    let (log, mut input, mut attributes, mut numeric) = fixture();
    log.fail("reciprocal", 1);
    let mut out = marked();
    let mut torque = v([17.0; 4]);
    let mut new = v([19.0; 4]);
    assert!(
        SkateboardSlideCollisionServices {
            input: &mut input,
            attributes: &mut attributes,
            numeric: &mut numeric
        }
        .try_collision(&mut out, &mut torque, Some(&mut new))
        .is_err()
    );
    assert_eq!(new, v([1.0, 0.0, 3.0, 7.0]));
    assert_eq!(out, marked());
    assert_eq!(torque, v([17.0; 4]));
}

#[test]
fn collision_angle_failure_retains_force_local_zero_torque_without_wind_read() {
    let (log, mut input, mut attributes, mut numeric) = fixture();
    log.fail("dot", 5);
    let mut out = marked();
    let mut torque = v([17.0; 4]);
    assert!(
        SkateboardSlideCollisionServices {
            input: &mut input,
            attributes: &mut attributes,
            numeric: &mut numeric
        }
        .try_collision(&mut out, &mut torque, None)
        .is_err()
    );
    assert_eq!(
        out.world_space_force.xyz.map(f32::to_bits),
        [4.0_f32, 0.0, 0.0].map(f32::to_bits)
    );
    assert_eq!(out.local_space_position, v([0.0, -0.05, 0.0, 0.0]));
    assert_eq!(torque, ZERO);
    assert!(!log.events().contains(&"up_vector"));
}

#[test]
fn collision_late_curve_failure_retains_zero_torque_after_late_wind() {
    let (log, mut input, mut attributes, mut numeric) = fixture();
    log.fail("torque_curve", 1);
    let mut out = marked();
    let mut torque = v([17.0; 4]);
    assert!(
        SkateboardSlideCollisionServices {
            input: &mut input,
            attributes: &mut attributes,
            numeric: &mut numeric
        }
        .try_collision(&mut out, &mut torque, None)
        .is_err()
    );
    assert_eq!(torque, ZERO);
    assert_eq!(
        log.events().iter().filter(|&&x| x == "up_vector").count(),
        1
    );
}

#[test]
fn collision_unknown_w_never_becomes_known_through_zero_coefficient() {
    let (_, mut input, mut attributes, mut numeric) = fixture();
    input.hook.w = None;
    input.up.w = None;
    let mut out = marked();
    let mut torque = ZERO;
    let mut new = ZERO;
    SkateboardSlideCollisionServices {
        input: &mut input,
        attributes: &mut attributes,
        numeric: &mut numeric,
    }
    .try_collision(&mut out, &mut torque, Some(&mut new))
    .unwrap();
    assert_eq!(new.w, None);
    assert_eq!(out.world_space_force.w, None);
    assert_eq!(torque.w, None);
}

#[test]
fn collision_nan_impulse_length_keeps_true_and_nan_limit_does_not_scale() {
    for exceptional_response in [true, false] {
        let (_, mut input, mut attributes, mut numeric) = fixture();
        if exceptional_response {
            attributes.response = f32::NAN;
        } else {
            attributes.limit = f32::NAN;
        }
        let mut out = marked();
        let mut torque = ZERO;
        let active = SkateboardSlideCollisionServices {
            input: &mut input,
            attributes: &mut attributes,
            numeric: &mut numeric,
        }
        .try_collision(&mut out, &mut torque, None)
        .unwrap();
        assert!(active);
        if exceptional_response {
            assert!(out.world_space_force.xyz[0].is_nan());
        } else {
            assert_eq!(out.world_space_force.xyz[0].to_bits(), 6.0_f32.to_bits());
        }
        assert_eq!(torque, v([0.0, 2.0, 0.0, 6.0]));
    }
}

#[test]
fn slide_nan_time_selects_authored_upper_duration_without_sanitizing_input() {
    let (_, mut input, mut attributes, mut numeric) = fixture();
    let mut out = marked();
    SkateboardSlideCollisionServices {
        input: &mut input,
        attributes: &mut attributes,
        numeric: &mut numeric,
    }
    .try_slide(f32::NAN, &mut out)
    .unwrap();
    assert_eq!(
        attributes.curve_inputs[2],
        (SlideCollisionCurve::SideFrictionFactor, 1.0)
    );
}

#[test]
fn shared_signed_angle_reads_wind_only_after_acos_and_never_on_rejected_axis() {
    for rejected in [true, false] {
        let log = Log::default();
        let mut numeric = Numeric { log: log.clone() };
        let axis = if rejected {
            [0.0; 4]
        } else {
            [1.0, 0.0, 0.0, 0.0]
        };
        let result = try_signed_angle_with_wind(
            axis,
            [0.0, 0.0, 1.0, 0.0],
            || {
                log.event("wind")?;
                Ok([0.0, 1.0, 0.0, 0.0])
            },
            &mut numeric,
        )
        .unwrap();
        if rejected {
            assert_eq!(result.to_bits(), 0);
            assert_eq!(log.events(), ["dot"]);
        } else {
            let e = log.events();
            assert_eq!(&e[e.len() - 2..], &["wind", "dot"]);
            assert!(e[..e.len() - 2].contains(&"rsqrt"));
        }
    }
}
