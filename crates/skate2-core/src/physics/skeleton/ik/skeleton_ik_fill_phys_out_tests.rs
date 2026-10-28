use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead},
    skeleton_data_tests::{build, fixture},
    skeleton_ik_targets::SkeletonIkTargets,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
type Trace = Rc<RefCell<Vec<String>>>;
fn v(x: f32, y: f32, z: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [x, y, z],
        w: None,
    }
}
struct Deck;
impl SkateboardAttributeRead for Deck {
    type Error = &'static str;
    type Pointer = u64;
    fn lookup(&mut self, _: SkateboardAttributeClass, key: u64) -> Result<u64, Self::Error> {
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        Ok(match *key {
            0xd76d_9db5_ee1f_d008 => 2.0,
            0xc94d_6509_1dcb_828b => 4.0,
            _ => 0.0,
        })
    }
    fn byte(&mut self, _: &u64) -> Result<u8, Self::Error> {
        panic!("no byte")
    }
    fn integer(&mut self, _: &u64) -> Result<i32, Self::Error> {
        panic!("no integer")
    }
}
fn geometry() -> SkeletonIkGeometry {
    let mut g = SkeletonIkGeometry::new();
    g.try_construct(
        &mut Deck,
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
    )
    .unwrap();
    g
}
fn state() -> SkeletonIkState {
    let (h, p, pose) = fixture();
    let mut targets = SkeletonIkTargets::new(Arc::new(build(h, p, &pose).unwrap()));
    targets.initialize();
    SkeletonIkState::new(&mut targets)
}
struct Attributes(Trace);
impl SkeletonIkAttributeRead for Attributes {
    type Error = &'static str;
    fn scalar(&mut self, _: u64) -> Result<f32, Self::Error> {
        panic!("no scalar")
    }
    fn vector(&mut self, key: u64) -> Result<[f32; 4], Self::Error> {
        assert_eq!(key, 0x77dd_00c1_8946_c6cf);
        self.0.borrow_mut().push("feet_margin".into());
        Ok([0.0, 1.0, 0.0, 0.0])
    }
}
struct Services {
    trace: Trace,
    draw: bool,
    missing_global: bool,
}
impl SkeletonIkFillServices for Services {
    type Error = &'static str;
    type Global = ();
    fn capture_skeleton_global(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push("global".into());
        if self.missing_global {
            Err("global")
        } else {
            Ok(())
        }
    }
    fn hand_grab_margin(&mut self, (): &()) -> Result<[f32; 4], Self::Error> {
        self.trace.borrow_mut().push("hand_margin".into());
        Ok([0.0, 1.0, 0.0, 0.0])
    }
    fn physics_column(
        &mut self,
        part: usize,
        col: usize,
    ) -> Result<SkeletonRootVector, Self::Error> {
        self.trace.borrow_mut().push(format!("column{part}:{col}"));
        Ok(match (part, col) {
            (0, 0) => v(1.0, 0.0, 0.0),
            (0, 1) => v(0.0, 1.0, 0.0),
            (0, 2) => v(0.0, 0.0, 1.0),
            (0, 3) => v(10.0, 0.0, 0.0),
            (15, 3) => v(10.5, 0.0, 0.0),
            (19, 3) => v(9.5, 0.0, 0.0),
            _ => panic!("unexpected part"),
        })
    }
    fn delta_time(&mut self) -> Result<f32, Self::Error> {
        self.trace.borrow_mut().push("dt".into());
        Ok(0.5)
    }
    fn physics_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.trace.borrow_mut().push(format!("velocity{part}"));
        Ok(v(if part == 15 { 3.0 } else { 4.0 }, 0.0, 0.0))
    }
    fn draw_contact_info(&mut self) -> Result<bool, Self::Error> {
        self.trace.borrow_mut().push("draw".into());
        Ok(self.draw)
    }
    fn debug_overlay_noop(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push("overlay".into());
        Ok(())
    }
}
struct Output {
    trace: Trace,
    fail_vector: bool,
    vectors: Vec<(IkFillVector, SkeletonRootVector)>,
    flags: Vec<(IkFillFlag, bool)>,
}
impl SkeletonIkFillOutput for Output {
    type Error = &'static str;
    type Guard = ();
    fn sample_guard(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push("guard".into());
        Ok(())
    }
    fn vector(
        &mut self,
        (): (),
        field: IkFillVector,
        value: SkeletonRootVector,
    ) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push(format!("store{field:?}"));
        if self.fail_vector {
            return Err("locked");
        }
        self.vectors.push((field, value));
        Ok(())
    }
    fn flag(
        &mut self,
        (): (),
        field: IkFillFlag,
        value: impl FnOnce() -> bool,
    ) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push(format!("store{field:?}"));
        self.flags.push((field, value()));
        Ok(())
    }
}
struct Numeric {
    trace: Trace,
    calls: usize,
    fail_second: bool,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn reciprocal_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.calls += 1;
        self.trace.borrow_mut().push(format!("recip{}", self.calls));
        if self.calls == 2 && self.fail_second {
            Err("estimate")
        } else {
            Ok(1.0 / x)
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no rsqrt")
    }
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("no dot")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no conversion")
    }
}
fn collaborators() -> (Attributes, Services, Output, Numeric) {
    let t = Trace::default();
    (
        Attributes(t.clone()),
        Services {
            trace: t.clone(),
            draw: true,
            missing_global: false,
        },
        Output {
            trace: t.clone(),
            fail_vector: false,
            vectors: vec![],
            flags: vec![],
        },
        Numeric {
            trace: t,
            calls: 0,
            fail_second: false,
        },
    )
}
#[test]
fn full_leaf_order_uses_real_history_and_publishes_distinct_world_velocities() {
    let mut state = state();
    let (a, s, o, n) = &mut collaborators();
    state.try_fill_phys_out(&geometry(), a, s, o, n).unwrap();
    assert_eq!(
        state.last_left_foot_position_deck.xyz.map(f32::to_bits),
        [0.5f32, 0.0, 0.0].map(f32::to_bits)
    );
    assert_eq!(o.vectors[0].1.xyz[0].to_bits(), 1.0f32.to_bits());
    assert_eq!(o.vectors[1].1.xyz[0].to_bits(), (-1.0f32).to_bits());
    assert_eq!(o.vectors[2].1.xyz[0].to_bits(), 3.0f32.to_bits());
    assert!(o.vectors[2].1.w.is_none());
    assert_eq!(o.vectors[0].1.w.map(f32::to_bits), Some(0));
    let trace = s.trace.borrow();
    assert_eq!(
        &trace[6..15],
        &[
            "feet_margin",
            "column0:3",
            "column0:1",
            "column0:2",
            "column0:0",
            "dt",
            "column19:3",
            "column15:3",
            "velocity15"
        ]
    );
    assert_eq!(trace.iter().filter(|v| v.as_str() == "overlay").count(), 4);
    assert_eq!(
        o.flags.iter().map(|(field, _)| *field).collect::<Vec<_>>(),
        [
            IkFillFlag::GrabbingLeft,
            IkFillFlag::GrabbingRight,
            IkFillFlag::LeftOnDeck,
            IkFillFlag::RightOnDeck,
            IkFillFlag::DontPush,
            IkFillFlag::DontPushOrBrake
        ]
    );
}
#[test]
fn second_estimate_failure_retains_both_updated_histories_but_no_velocity_store() {
    let mut state = state();
    let (a, s, o, n) = &mut collaborators();
    n.fail_second = true;
    assert!(matches!(
        state.try_fill_phys_out(&geometry(), a, s, o, n),
        Err(SkeletonIkFillError::Numeric("estimate"))
    ));
    assert_eq!(
        state.last_left_foot_position_deck.xyz[0].to_bits(),
        0.5f32.to_bits()
    );
    assert_eq!(
        state.last_right_foot_position_deck.xyz[0].to_bits(),
        (-0.5f32).to_bits()
    );
    assert!(o.vectors.is_empty());
    assert_eq!(o.flags.len(), 2);
}
#[test]
fn output_rejection_is_after_history_mutation_and_both_estimates() {
    let mut state = state();
    let (a, s, o, n) = &mut collaborators();
    o.fail_vector = true;
    assert!(matches!(
        state.try_fill_phys_out(&geometry(), a, s, o, n),
        Err(SkeletonIkFillError::Output("locked"))
    ));
    assert_eq!(n.calls, 2);
    assert_eq!(
        state.last_left_foot_position_deck.xyz[0].to_bits(),
        0.5f32.to_bits()
    );
    assert!(!s.trace.borrow().iter().any(|v| v == "draw"));
}
#[test]
fn strict_bounds_reject_boundary_and_nan_but_ignore_packed_w() {
    assert!(hand_inside(v(0.0, 0.0, 0.0), [1.0; 4]));
    assert!(!hand_inside(v(1.0, 0.0, 0.0), [1.0; 4]));
    assert!(!hand_inside(v(0.0, f32::NAN, 0.0), [1.0; 4]));
    assert!(!foot_inside(v(0.0, 0.0, f32::NAN), [1.0; 4]));
    assert!(!foot_inside(v(-1.0, 0.0, 0.0), [1.0; 4]));
}
#[test]
fn actual_missing_global_precedes_any_output_or_feet_attribute_query() {
    let mut state = state();
    let (a, s, o, n) = &mut collaborators();
    s.missing_global = true;
    assert!(matches!(
        state.try_fill_phys_out(&geometry(), a, s, o, n),
        Err(SkeletonIkFillError::Source("global"))
    ));
    assert_eq!(&*s.trace.borrow(), &["global"]);
    assert!(o.flags.is_empty());
}
