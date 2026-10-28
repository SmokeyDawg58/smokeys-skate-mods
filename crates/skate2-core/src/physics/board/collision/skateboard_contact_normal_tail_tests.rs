use super::*;
use std::{cell::RefCell, rc::Rc};
type Log = Rc<RefCell<Vec<String>>>;
struct Attributes {
    log: Log,
    angle: f32,
}
impl SkateboardGroundNormalAttributeRead for Attributes {
    type Global = ();
    type Pointer = ();
    type Error = &'static str;
    fn capture_global(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push("global".into());
        Ok(())
    }
    fn lookup(&mut self, (): &(), key: u64) -> Result<(), Self::Error> {
        assert_eq!(key, GROUND_NORMAL_ANGLE);
        self.log.borrow_mut().push("lookup".into());
        Ok(())
    }
    fn float(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("value".into());
        Ok(self.angle)
    }
}
struct Services {
    log: Log,
    enabled: bool,
    drag: [Option<f32>; 4],
    parts: Option<u8>,
    wheels: Option<u8>,
    fail_wheel: Option<usize>,
}
impl SkateboardContactNormalServices for Services {
    type Target = ();
    type Error = &'static str;
    fn read_free_wheel_enabled(&mut self) -> Result<bool, Self::Error> {
        self.log.borrow_mut().push("enabled".into());
        Ok(self.enabled)
    }
    fn capture_free_wheel_drag(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push("capture drag".into());
        Ok(())
    }
    fn free_wheel_drag(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("read drag".into());
        Ok(0.04)
    }
    fn store_num_parts(&mut self, count: u8) -> Result<(), Self::Error> {
        self.log.borrow_mut().push("parts".into());
        self.parts = Some(count);
        Ok(())
    }
    fn store_num_wheels(&mut self, count: u8) -> Result<(), Self::Error> {
        self.log.borrow_mut().push("wheels".into());
        self.wheels = Some(count);
        Ok(())
    }
    fn set_wheel_angular_drag(&mut self, wheel: usize, value: f32) -> Result<(), Self::Error> {
        self.log.borrow_mut().push(format!("drag{wheel}"));
        if self.fail_wheel == Some(wheel) {
            return Err("drag");
        }
        self.drag[wheel] = Some(value);
        Ok(())
    }
}
struct Numeric {
    log: Log,
    dots: Vec<([f32; 4], [f32; 4])>,
    fail_dot: Option<usize>,
    sqrt_count: usize,
    fail_sqrt: Option<usize>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("dot".into());
        self.dots.push((a, b));
        if self.fail_dot == Some(self.dots.len()) {
            return Err("dot");
        }
        Ok(crate::reckoning_numeric::host_dot3(a, b))
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("rsqrt".into());
        self.sqrt_count += 1;
        if self.fail_sqrt == Some(self.sqrt_count) {
            return Err("rsqrt");
        }
        Ok(1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("reciprocal".into());
        Ok(1.0 / value)
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("tail does not normalize an angle")
    }
}
struct Rounding {
    log: Log,
    fail: bool,
}
impl ReckoningSinCosNumeric for Rounding {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("round".into());
        if self.fail {
            Err("round")
        } else {
            Ok(value.round_ties_even())
        }
    }
}
struct Fixture {
    owner: SkateboardContactSurface,
    attributes: Attributes,
    services: Services,
    numeric: Numeric,
    rounding: Rounding,
    log: Log,
}
impl Fixture {
    fn new() -> Self {
        let log = Log::default();
        let mut owner = SkateboardContactSurface::new();
        owner.collision_state.reset();
        Self {
            owner,
            attributes: Attributes {
                log: log.clone(),
                angle: 80.0,
            },
            services: Services {
                log: log.clone(),
                enabled: false,
                drag: [None; 4],
                parts: None,
                wheels: None,
                fail_wheel: None,
            },
            numeric: Numeric {
                log: log.clone(),
                dots: Vec::new(),
                fail_dot: None,
                sqrt_count: 0,
                fail_sqrt: None,
            },
            rounding: Rounding {
                log: log.clone(),
                fail: false,
            },
            log,
        }
    }
    fn run(
        &mut self,
        best: i32,
    ) -> Result<
        SkateboardContactNormalOutcome,
        SkateboardContactNormalError<&'static str, &'static str, &'static str, &'static str>,
    > {
        self.owner.try_finish_ground_normals(
            best,
            UP,
            &mut self.attributes,
            &mut self.services,
            &mut self.numeric,
            &mut self.rounding,
        )
    }
    fn wheel(&mut self, index: usize, normal: SkeletonRootVector) {
        self.owner.collision_state.in_contact[index] = Some(true);
        self.owner.collision_state.surfaces.set_validity(index, 1);
        self.owner.collision_state.normal[index] = Some(normal);
    }
}
#[test]
fn accepted_wheel_updates_same_owner_and_keeps_unknown_w_after_four_drag_stores() {
    let mut f = Fixture::new();
    f.services.enabled = true;
    f.wheel(
        0,
        SkeletonRootVector {
            xyz: [0.0, 1.0, 0.0],
            w: None,
        },
    );
    let outcome = f.run(-1).unwrap();
    assert_eq!(
        outcome,
        SkateboardContactNormalOutcome {
            num_parts: 1,
            num_wheels: 1,
            updated_last_valid: true
        }
    );
    assert_eq!(
        f.owner.collision_state.average_normal,
        f.owner.collision_state.last_valid_normal
    );
    assert_eq!(f.owner.collision_state.last_valid_normal.unwrap().w, None);
    assert_eq!(
        f.services.drag[0].unwrap().to_bits(),
        (0.04 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(
        &f.log.borrow()[..7],
        &[
            "enabled",
            "parts",
            "capture drag",
            "read drag",
            "global",
            "lookup",
            "value"
        ]
    );
    let log = f.log.borrow();
    let last_drag = log.iter().position(|entry| entry == "drag3").unwrap();
    assert_eq!(log[last_drag + 1], "wheels");
    assert_eq!(log[last_drag + 2], "dot");
}
#[test]
fn first_dot_failure_precedes_best_index_validation_and_every_drag_write() {
    let mut f = Fixture::new();
    f.wheel(0, UP);
    f.numeric.fail_dot = Some(1);
    assert_eq!(f.run(99), Err(SkateboardContactNormalError::Numeric("dot")));
    assert_eq!(f.services.parts, Some(1));
    assert_eq!(f.services.wheels, None);
    assert_eq!(f.services.drag, [None; 4]);
    assert_eq!(f.owner.collision_state.last_valid_normal, Some(UP));
}
#[test]
fn late_drag_failure_preserves_prior_drag_but_suppresses_wheel_count_and_normal_commit() {
    let mut f = Fixture::new();
    f.services.fail_wheel = Some(2);
    assert_eq!(
        f.run(-1),
        Err(SkateboardContactNormalError::Service("drag"))
    );
    assert!(f.services.drag[0].is_some() && f.services.drag[1].is_some());
    assert_eq!(f.services.drag[2..], [None, None]);
    assert_eq!(f.services.wheels, None);
    assert_eq!(f.owner.collision_state.average_normal, Some(UP));
    assert!(f.numeric.dots.is_empty());
}
#[test]
fn failed_wheel_sum_estimate_occurs_after_count_and_preserves_both_normals() {
    let mut f = Fixture::new();
    f.wheel(0, UP);
    // Angle performs first/second normalization seeds, then its ACos seed.
    f.numeric.fail_sqrt = Some(4);
    assert_eq!(
        f.run(-1),
        Err(SkateboardContactNormalError::Numeric("rsqrt"))
    );
    assert_eq!(f.services.wheels, Some(1));
    assert!(f.services.drag.iter().all(Option::is_some));
    assert_eq!(f.owner.collision_state.average_normal, Some(UP));
    assert_eq!(f.owner.collision_state.last_valid_normal, Some(UP));
}
#[test]
fn zero_contact_fallback_still_estimates_zero_and_preserves_previous_last_valid() {
    let mut f = Fixture::new();
    let old = SkeletonRootVector::known([1.0, 0.0, 0.0, 7.0]);
    f.owner.collision_state.last_valid_normal = Some(old);
    let outcome = f.run(99).unwrap();
    assert!(!outcome.updated_last_valid);
    assert_eq!(f.numeric.sqrt_count, 1);
    assert_eq!(f.numeric.dots[0].0.map(f32::to_bits), [0; 4]);
    assert_eq!(f.owner.collision_state.average_normal, Some(UP));
    assert_eq!(f.owner.collision_state.last_valid_normal, Some(old));
    assert!(!f.log.borrow().iter().any(|entry| entry == "capture drag"));
}
#[test]
fn fallback_accumulates_deck_front_rear_and_scalar_left_normalizes_unknown_w() {
    let mut f = Fixture::new();
    for (part, x) in [(6, 1.0e20), (4, -1.0e20), (5, 1.0)] {
        f.owner.collision_state.in_contact[part] = Some(true);
        f.owner.collision_state.normal[part] = Some(SkeletonRootVector {
            xyz: [x, 0.0, 0.0],
            w: None,
        });
    }
    let outcome = f.run(-1).unwrap();
    assert_eq!(outcome.num_parts, 3);
    assert!(!outcome.updated_last_valid);
    assert_eq!(
        f.numeric.dots[0].0.map(f32::to_bits),
        [1.0_f32.to_bits(), 0, 0, 0]
    );
    assert_eq!(
        f.owner.collision_state.average_normal,
        Some(SkeletonRootVector {
            xyz: [1.0, 0.0, 0.0],
            w: None
        })
    );
    assert_eq!(f.owner.collision_state.last_valid_normal, Some(UP));
}
#[test]
fn cosine_rounding_failure_is_after_part_count_and_attributes_before_wheel_reads() {
    let mut f = Fixture::new();
    f.rounding.fail = true;
    assert_eq!(
        f.run(-1),
        Err(SkateboardContactNormalError::Rounding("round"))
    );
    assert_eq!(
        &*f.log.borrow(),
        &["enabled", "parts", "global", "lookup", "value", "round"]
    );
    assert_eq!(f.services.parts, Some(0));
    assert_eq!(f.services.wheels, None);
}
#[test]
fn invalid_best_is_lazy_and_small_best_angle_returns_zero_without_angle_estimates() {
    let mut f = Fixture::new();
    f.wheel(0, UP);
    f.owner.collision_state.last_valid_normal = Some(ZERO);
    assert!(f.run(-1).unwrap().updated_last_valid);
    assert_eq!(f.numeric.sqrt_count, 1); // only final sum normalization
    let mut f = Fixture::new();
    f.wheel(0, SkeletonRootVector::known([0.0, f32::NAN, 0.0, 0.0]));
    assert!(!f.run(99).unwrap().updated_last_valid); // unordered first dot skips best
}
#[test]
fn uninitialized_contact_owner_refuses_before_any_runtime_service() {
    let mut f = Fixture::new();
    f.owner = SkateboardContactSurface::new();
    assert_eq!(
        f.run(-1),
        Err(SkateboardContactNormalError::Unknown(
            SkateboardContactNormalUnknown::InContact(0)
        ))
    );
    assert!(f.log.borrow().is_empty());
}
