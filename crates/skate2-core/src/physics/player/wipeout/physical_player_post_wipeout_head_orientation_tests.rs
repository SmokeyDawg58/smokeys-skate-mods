use super::*;
use std::{cell::RefCell, rc::Rc};

type Events = Rc<RefCell<Vec<&'static str>>>;
const IDENTITY: [f32; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0.,
];
struct Cvars {
    events: Events,
    reads: usize,
}
impl HeadOrientationCvars for Cvars {
    type Error = &'static str;
    type HeadTarget = Rc<()>;
    fn capture_head_fraction(&mut self) -> Result<Self::HeadTarget, Self::Error> {
        self.events.borrow_mut().push("capture");
        Ok(Rc::new(()))
    }
    fn head_fraction(&mut self, target: &Self::HeadTarget) -> Result<f32, Self::Error> {
        assert_eq!(Rc::strong_count(target), 1);
        self.events.borrow_mut().push("head");
        self.reads += 1;
        // Distinct live values discriminate a single cached read.
        Ok(if self.reads == 1 { 0.2 } else { 0.25 })
    }
    fn spine_fraction(&mut self) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("spine");
        Ok(0.6)
    }
}
struct Numeric {
    events: Events,
    squared: f32,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("dot");
        Ok(self.squared)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("rsqrt");
        Ok(value.sqrt().recip())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("reciprocal");
        Ok(value.recip())
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("not reached")
    }
}
struct Rounding {
    events: Events,
    calls: usize,
    fail_at: Option<usize>,
}
impl ReckoningSinCosNumeric for Rounding {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("sincos");
        self.calls += 1;
        if self.fail_at == Some(self.calls) {
            Err("sincos")
        } else {
            Ok(value.round_ties_even())
        }
    }
}
struct Output {
    events: Events,
    writes: Vec<Matrix>,
    fail: Option<&'static str>,
}
impl Output {
    fn write(&mut self, event: &'static str, matrix: &Matrix) -> Result<(), &'static str> {
        self.events.borrow_mut().push(event);
        if self.fail == Some(event) {
            return Err(event);
        }
        self.writes.push(*matrix);
        Ok(())
    }
}
impl OrientationOutput for Output {
    type Error = &'static str;
    fn adjust_parent_record(&mut self, rotation: &Matrix) -> Result<(), Self::Error> {
        self.write("parent", rotation)
    }
    fn adjust_grandparent_record(&mut self, rotation: &Matrix) -> Result<(), Self::Error> {
        self.write("grandparent", rotation)
    }
    fn publish_head_global(&mut self, head: &Matrix) -> Result<(), Self::Error> {
        self.write("global", head)
    }
    fn publish_head_local(&mut self) -> Result<(), Self::Error> {
        self.events.borrow_mut().push("local");
        if self.fail == Some("local") {
            Err("local")
        } else {
            Ok(())
        }
    }
}
fn fixture(squared: f32) -> (Events, Cvars, Numeric, Rounding, Output) {
    let events = Rc::new(RefCell::new(Vec::new()));
    (
        Rc::clone(&events),
        Cvars {
            events: Rc::clone(&events),
            reads: 0,
        },
        Numeric {
            events: Rc::clone(&events),
            squared,
        },
        Rounding {
            events: Rc::clone(&events),
            calls: 0,
            fail_at: None,
        },
        Output {
            events,
            writes: Vec::new(),
            fail: None,
        },
    )
}
fn close(actual: f32, expected: f64) {
    assert!(
        (f64::from(actual) - expected).abs() < 2e-6,
        "{actual} != {expected}"
    );
}

#[test]
fn squared_threshold_equal_zero_and_nan_skip_every_later_service() {
    for squared in [0., f32::from_bits(0x3780_0000), f32::NAN] {
        let (events, mut c, mut n, mut r, mut o) = fixture(squared);
        assert!(
            orient(
                Matrix::known(IDENTITY),
                Vector::known([0.; 4]),
                &mut c,
                &mut n,
                &mut r,
                &mut o
            )
            .is_ok()
        );
        assert_eq!(*events.borrow(), ["dot"]);
    }
}

#[test]
fn distinct_head_reads_three_angles_and_late_spine_read_preserve_native_sequence() {
    let (events, mut c, mut n, mut r, mut o) = fixture(1.);
    let mut head = Matrix::known(IDENTITY);
    let position = Vector {
        xyz: [7., 8., 9.],
        w: None,
    };
    head.set_column(3, position);
    assert!(
        orient(
            head,
            Vector::known([0., 0., 1., f32::NAN]),
            &mut c,
            &mut n,
            &mut r,
            &mut o
        )
        .is_ok()
    );
    assert_eq!(
        *events.borrow(),
        [
            "dot",
            "rsqrt",
            "capture",
            "head",
            "head",
            "reciprocal",
            "sincos",
            "sincos",
            "spine",
            "sincos",
            "parent",
            "grandparent",
            "global",
            "local"
        ]
    );
    for (matrix, angle) in o.writes.iter().zip([0.8_f64, 0.48, 0.25]) {
        close(matrix.columns()[0].xyz[0], angle.cos());
        close(matrix.columns()[0].xyz[1], angle.sin());
        close(matrix.columns()[1].xyz[0], -angle.sin());
        assert_eq!(matrix.columns()[0].w, Some(matrix.columns()[0].xyz[0]));
        assert_eq!(matrix.columns()[1].w, Some(matrix.columns()[1].xyz[0]));
    }
    assert_eq!(o.writes[2].columns()[3], position);
}

#[test]
fn trigonometry_and_late_publication_failures_preserve_only_completed_prefixes() {
    let (events, mut c, mut n, mut r, mut o) = fixture(1.);
    r.fail_at = Some(2);
    assert!(matches!(
        orient(
            Matrix::known(IDENTITY),
            Vector::known([0., 0., 1., 0.]),
            &mut c,
            &mut n,
            &mut r,
            &mut o
        ),
        Err(HeadOrientationError::Trigonometry("sincos"))
    ));
    assert!(!events.borrow().contains(&"spine"));
    assert!(o.writes.is_empty());
    for (failure, count) in [("grandparent", 1), ("global", 2), ("local", 3)] {
        let (_, mut c, mut n, mut r, mut o) = fixture(1.);
        o.fail = Some(failure);
        assert!(matches!(
            orient(
                Matrix::known(IDENTITY),
                Vector::known([0., 0., 1., 0.]),
                &mut c,
                &mut n,
                &mut r,
                &mut o
            ),
            Err(HeadOrientationError::Output(_))
        ));
        assert_eq!(o.writes.len(), count);
    }
}

#[test]
fn record_orientation_ignores_frame_translation_preserves_record_p_and_unknown_basis_w() {
    let mut original = Matrix::known(IDENTITY);
    let position = Vector {
        xyz: [5., -2., 3.],
        w: None,
    };
    original.set_column(3, position);
    let mut frame = Matrix::known(IDENTITY);
    frame.set_column(3, Vector::known([f32::NAN; 4]));
    let mut to_world = frame;
    let mut axis = to_world.columns()[0];
    axis.w = None;
    to_world.set_column(0, axis);
    let rotation = normal_rotation([0., 0., 1.], 0.3_f32.sin(), 0.3_f32.cos());
    let result = adjust_record_orientation(original, &rotation, &frame, &to_world);
    assert_eq!(result.columns()[3], position);
    close(result.columns()[0].xyz[0], 0.3_f64.cos());
    close(result.columns()[0].xyz[1], 0.3_f64.sin());
    for column in &result.columns()[..3] {
        assert_eq!(column.w, None);
    }
}
