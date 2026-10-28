use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
use std::{cell::RefCell, rc::Rc};

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_push_holding_attribute() {
    let vault = crate::attribute_runtime_corpus::mounted_physics_attributes_with_classes(&[
        0x90c3_decf_0105_631e,
    ]);
    let attributes = PushingMotionAttributes::try_bind(&vault).unwrap();
    let key = skate2_formats::attrib::string_hash64("max_holding_acc");
    assert_eq!(key, 0x07a7_a486_d4c7_5e46);
    assert!(
        attributes
            .0
            .try_get_data(0xd4c7_5e46_07a7_a486, 0)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        attributes.max_holding_acc().unwrap().to_bits(),
        4.0_f32.to_bits()
    );
    for key in [0x4c4b_9cdf_5860_f565, 0x1a4c_a176_9dcc_b3a8] {
        let data = attributes.first_push_scalar_pointer(key).unwrap();
        assert!(
            data.is_some(),
            "native pushing scalar {key:016x} must resolve"
        );
        let value = PushingMotionAttributes::first_push_scalar_value(data.as_ref()).unwrap();
        eprintln!("native pushing scalar {key:016x} = {value}");
        assert!(value > 0.0);
    }
}
type Trace = Rc<RefCell<Vec<&'static str>>>;
struct Source {
    trace: Trace,
    y: f32,
    limit: f32,
    flag: bool,
    fail: Option<&'static str>,
}
impl Source {
    fn event(&self, name: &'static str) -> Result<(), &'static str> {
        self.trace.borrow_mut().push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl DisablePushSource for Source {
    type Error = &'static str;
    type Attribute = ();
    type Output = ();
    fn ground_up_y(&mut self) -> Result<f32, Self::Error> {
        self.event("normal")?;
        Ok(self.y)
    }
    fn capture_slope_limit(&mut self) -> Result<(), Self::Error> {
        self.event("lookup")
    }
    fn read_slope_limit(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.event("load")?;
        Ok(self.limit)
    }
    fn fresh_output(&mut self) -> Result<(), Self::Error> {
        self.event("fresh")
    }
    fn tell_anim_dont_push_or_brake(&mut self, (): &()) -> Result<bool, Self::Error> {
        self.event("flag")?;
        Ok(self.flag)
    }
}
struct Numeric {
    trace: Trace,
    fail: bool,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("asin has no dot")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.trace.borrow_mut().push("asin");
        if self.fail {
            Err("asin")
        } else {
            HostReckoningNumeric
                .reciprocal_sqrt_estimate(value)
                .map_err(|_| "estimate")
        }
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no reciprocal")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no integer")
    }
}
fn fixture() -> (Source, Numeric, Trace) {
    let trace = Trace::default();
    (
        Source {
            trace: trace.clone(),
            y: 0.0,
            limit: 45.0,
            flag: false,
            fail: None,
        },
        Numeric {
            trace: trace.clone(),
            fail: false,
        },
        trace,
    )
}
#[test]
fn steep_case_still_gets_fresh_output_but_skips_skeleton_byte() {
    let (mut source, mut numeric, trace) = fixture();
    assert!(try_disable_push(&mut source, &mut numeric).unwrap());
    assert_eq!(
        *trace.borrow(),
        ["normal", "lookup", "asin", "load", "fresh"]
    );
}
#[test]
fn fresh_output_failure_is_not_hidden_by_true_slope() {
    let (mut source, mut numeric, _) = fixture();
    source.fail = Some("fresh");
    assert_eq!(
        try_disable_push(&mut source, &mut numeric),
        Err(DisablePushError::Source("fresh"))
    );
}
#[test]
fn pointer_is_acquired_before_asin_but_float_is_read_after_it() {
    let (mut source, mut numeric, trace) = fixture();
    numeric.fail = true;
    assert_eq!(
        try_disable_push(&mut source, &mut numeric),
        Err(DisablePushError::Numeric("asin"))
    );
    assert_eq!(*trace.borrow(), ["normal", "lookup", "asin"]);
}
#[test]
fn equal_limit_is_not_steep_and_reads_current_skeleton_flag() {
    let (mut source, mut numeric, trace) = fixture();
    source.limit = 90.0;
    source.flag = true;
    assert!(try_disable_push(&mut source, &mut numeric).unwrap());
    assert_eq!(trace.borrow().last(), Some(&"flag"));
    source.flag = false;
    assert!(!try_disable_push(&mut source, &mut numeric).unwrap());
}
#[test]
fn nan_limit_retains_ordered_compare_and_late_flag() {
    let (mut source, mut numeric, trace) = fixture();
    source.limit = f32::NAN;
    source.flag = false;
    assert!(!try_disable_push(&mut source, &mut numeric).unwrap());
    assert_eq!(trace.borrow().last(), Some(&"flag"));
}
