use super::*;
use std::{cell::RefCell, rc::Rc};

struct Services {
    events: Rc<RefCell<Vec<String>>>,
    fail: Option<usize>,
}
impl SkateboardWheelCompressionServices for Services {
    type Error = &'static str;
    fn body_transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error> {
        self.events.borrow_mut().push(format!("part {part}"));
        if self.fail == Some(part) {
            return Err("part");
        }
        let vector = |xyz| SkeletonRootVector { xyz, w: None };
        if part == 6 {
            Ok(SkeletonRootMatrix::from_columns([
                vector([2.0, 0.0, 0.0]),
                vector([1.0, 4.0, 0.0]),
                vector([0.0, 0.0, 0.5]),
                vector([10.0, 20.0, 30.0]),
            ]))
        } else {
            let y = [1.0, 3.0, 5.0, 7.0][part];
            Ok(SkeletonRootMatrix::translation(vector([
                10.0 + y,
                20.0 + 4.0 * y,
                30.0,
            ])))
        }
    }
    fn wheel_distance(&mut self) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("distance".into());
        Ok(0.25)
    }
}
struct Numeric {
    events: Rc<RefCell<Vec<String>>>,
    fail: bool,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("dot".into());
        Ok(crate::reckoning_numeric::host_dot3(a, b))
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("reciprocal".into());
        if self.fail {
            Err("numeric")
        } else {
            Ok(1.0 / value)
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("general inverse uses reciprocal")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no conversion")
    }
}
fn fixture() -> (Services, Numeric) {
    let events = Rc::new(RefCell::new(Vec::new()));
    (
        Services {
            events: Rc::clone(&events),
            fail: None,
        },
        Numeric {
            events,
            fail: false,
        },
    )
}

#[test]
fn sheared_scaled_body_uses_general_inverse_and_late_distance_in_source_read_order() {
    let (mut services, mut numeric) = fixture();
    let mut state = SkateboardWheelCompressions::default();
    state.try_update(&mut services, &mut numeric).unwrap();
    assert_eq!(
        state.current().unwrap().map(f32::to_bits),
        [1.75_f32, 5.75].map(f32::to_bits)
    );
    assert_eq!(
        *services.events.borrow(),
        [
            "part 6",
            "dot",
            "reciprocal",
            "part 0",
            "part 1",
            "part 2",
            "part 3",
            "distance"
        ]
    );
}

#[test]
fn numeric_and_later_part_refusals_keep_previous_fields_but_revoke_readiness() {
    for numeric_failure in [true, false] {
        let (mut services, mut numeric) = fixture();
        let mut state = SkateboardWheelCompressions::default();
        state.try_update(&mut services, &mut numeric).unwrap();
        let old = (state.front, state.back);
        services.events.borrow_mut().clear();
        numeric.fail = numeric_failure;
        services.fail = (!numeric_failure).then_some(2);
        assert!(state.try_update(&mut services, &mut numeric).is_err());
        assert_eq!(state.current(), None);
        assert_eq!((state.front, state.back), old);
        assert!(
            !services
                .events
                .borrow()
                .iter()
                .any(|event| event == "distance")
        );
        if numeric_failure {
            assert_eq!(*services.events.borrow(), ["part 6", "dot", "reciprocal"]);
        }
    }
}
