use super::*;
use std::{cell::RefCell, rc::Rc};

const KEYS: [u64; 4] = [
    0xd76d_9db5_ee1f_d008,
    0xc94d_6509_1dcb_828b,
    0x1d53_bc53_9321_90e7,
    0x8020_3de9_f9b7_32ca,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Lookup(u64),
    Load(u64),
    Round(u32),
    Reciprocal(u32),
}
#[derive(Default)]
struct Trace {
    events: Vec<Event>,
    fail_at: Option<usize>,
}
impl Trace {
    fn push(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail_at == Some(self.events.len()) {
            Err("refused")
        } else {
            Ok(())
        }
    }
}
struct Deck {
    trace: Rc<RefCell<Trace>>,
    values: [f32; 4],
}
impl SkateboardAttributeRead for Deck {
    type Pointer = u64;
    type Error = &'static str;
    fn lookup(&mut self, class: SkateboardAttributeClass, key: u64) -> Result<u64, Self::Error> {
        assert_eq!(class, SkateboardAttributeClass::Deck);
        self.trace.borrow_mut().push(Event::Lookup(key))?;
        assert!(KEYS.contains(&key));
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        self.trace.borrow_mut().push(Event::Load(*key))?;
        Ok(self.values[KEYS.iter().position(|candidate| candidate == key).unwrap()])
    }
    fn byte(&mut self, _: &u64) -> Result<u8, Self::Error> {
        panic!("no byte geometry")
    }
    fn integer(&mut self, _: &u64) -> Result<i32, Self::Error> {
        panic!("no integer geometry")
    }
}
struct Numeric(Rc<RefCell<Trace>>);
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.0
            .borrow_mut()
            .push(Event::Reciprocal(value.to_bits()))?;
        Ok(1.0 / value)
    }
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("no dot")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no rsqrt")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no scalar conversion")
    }
}
struct Rounding(Rc<RefCell<Trace>>);
impl ReckoningSinCosNumeric for Rounding {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.0.borrow_mut().push(Event::Round(value.to_bits()))?;
        Ok(value.round_ties_even())
    }
}
fn fixture(degrees: f32) -> (Deck, Numeric, Rounding) {
    let trace = Rc::new(RefCell::new(Trace::default()));
    (
        Deck {
            trace: Rc::clone(&trace),
            values: [0.5, 0.75, 0.125, degrees],
        },
        Numeric(Rc::clone(&trace)),
        Rounding(trace),
    )
}
fn snapshot(owner: &SkeletonIkGeometry) -> [Option<u32>; 4] {
    [
        owner.box_half_width(),
        owner.box_half_length(),
        owner.total_half_length(),
        owner.end_slope(),
    ]
    .map(|value| value.map(f32::to_bits))
}

#[test]
fn fresh_geometry_has_no_invented_scalar_values() {
    let geometry = SkeletonIkGeometry::new();
    assert_eq!(snapshot(&geometry), [None; 4]);
    assert_eq!(geometry.progress(), SkeletonIkGeometryProgress::Fresh);
}

#[test]
fn four_deck_pointer_load_pairs_precede_tangent_and_retain_source_geometry() {
    let (mut deck, mut numeric, mut rounding) = fixture(30.0);
    let mut geometry = SkeletonIkGeometry::new();
    geometry
        .try_construct(&mut deck, &mut numeric, &mut rounding)
        .unwrap();
    assert_eq!(geometry.progress(), SkeletonIkGeometryProgress::Complete);
    assert_eq!(
        geometry.box_half_width().unwrap().to_bits(),
        0.25_f32.to_bits()
    );
    assert_eq!(
        geometry.box_half_length().unwrap().to_bits(),
        0.375_f32.to_bits()
    );
    assert_eq!(
        geometry.total_half_length().unwrap().to_bits(),
        0.5_f32.to_bits()
    );
    let expected = (30.0_f64 * std::f64::consts::PI / 180.0).tan();
    assert!((f64::from(geometry.end_slope().unwrap()) - expected).abs() < 0.000_001);
    let trace = deck.trace.borrow();
    let pairs: Vec<_> = KEYS
        .into_iter()
        .flat_map(|key| [Event::Lookup(key), Event::Load(key)])
        .collect();
    assert_eq!(&trace.events[..8], pairs.as_slice());
    assert!(matches!(trace.events[8], Event::Round(_)));
    assert!(matches!(trace.events[9], Event::Reciprocal(_)));
    assert!(matches!(trace.events[10], Event::Reciprocal(_)));
}

#[test]
fn each_lookup_or_load_failure_preserves_exactly_the_completed_field_prefix() {
    for fail_at in 1..=8 {
        let (mut deck, mut numeric, mut rounding) = fixture(30.0);
        deck.trace.borrow_mut().fail_at = Some(fail_at);
        let mut geometry = SkeletonIkGeometry::new();
        assert_eq!(
            geometry.try_construct(&mut deck, &mut numeric, &mut rounding),
            Err(SkeletonIkGeometryError::Deck("refused"))
        );
        let completed = (fail_at - 1) / 2;
        let mut expected = [None; 4];
        for (index, value) in [0.25_f32, 0.375, 0.5]
            .into_iter()
            .enumerate()
            .take(completed)
        {
            expected[index] = Some(value.to_bits());
        }
        assert_eq!(snapshot(&geometry), expected);
        let phase = [
            SkeletonIkGeometryProgress::BoxHalfWidth,
            SkeletonIkGeometryProgress::BoxHalfLength,
            SkeletonIkGeometryProgress::TotalHalfLength,
            SkeletonIkGeometryProgress::EndSlope,
        ][completed];
        assert_eq!(geometry.progress(), phase);
        assert_eq!(deck.trace.borrow().events.len(), fail_at);
        assert_eq!(
            geometry.try_construct(&mut deck, &mut numeric, &mut rounding),
            Err(SkeletonIkGeometryError::AlreadyAttempted(phase))
        );
        assert_eq!(deck.trace.borrow().events.len(), fail_at);
    }
}

#[test]
fn tangent_failures_preserve_all_lengths_and_never_publish_slope() {
    for fail_at in 9..=11 {
        let (mut deck, mut numeric, mut rounding) = fixture(30.0);
        deck.trace.borrow_mut().fail_at = Some(fail_at);
        let mut geometry = SkeletonIkGeometry::new();
        let error = if fail_at == 9 {
            SkeletonIkTangentError::Rounding("refused")
        } else {
            SkeletonIkTangentError::Numeric("refused")
        };
        assert_eq!(
            geometry.try_construct(&mut deck, &mut numeric, &mut rounding),
            Err(SkeletonIkGeometryError::Tangent(error))
        );
        assert_eq!(
            snapshot(&geometry),
            [
                Some(0.25_f32.to_bits()),
                Some(0.375_f32.to_bits()),
                Some(0.5_f32.to_bits()),
                None
            ]
        );
        assert_eq!(geometry.progress(), SkeletonIkGeometryProgress::EndSlope);
        assert_eq!(deck.trace.borrow().events.len(), fail_at);
        assert!(matches!(
            geometry.try_construct(&mut deck, &mut numeric, &mut rounding),
            Err(SkeletonIkGeometryError::AlreadyAttempted(
                SkeletonIkGeometryProgress::EndSlope
            ))
        ));
        assert_eq!(deck.trace.borrow().events.len(), fail_at);
    }
}

#[test]
fn source_fsel_clamps_nonfinite_and_extreme_degrees_before_tangent() {
    for (degrees, expected) in [
        (0.0, 0x3c23_d871),
        (-30.0, 0x3c23_d871),
        (f32::NEG_INFINITY, 0x3c23_d871),
        (1000.0, 0x42c7_fe57),
        (f32::INFINITY, 0x42c7_fe57),
        (f32::NAN, 0x42c7_fe57),
    ] {
        let (mut deck, mut numeric, mut rounding) = fixture(degrees);
        let mut geometry = SkeletonIkGeometry::new();
        geometry
            .try_construct(&mut deck, &mut numeric, &mut rounding)
            .unwrap();
        assert_eq!(geometry.end_slope().unwrap().to_bits(), expected);
        assert_eq!(deck.trace.borrow().events.len(), 11);
    }
}

#[test]
fn length_values_are_not_sanitized_and_a_completed_constructor_cannot_replay() {
    let (mut deck, mut numeric, mut rounding) = fixture(30.0);
    deck.values[..3].copy_from_slice(&[-0.0, f32::INFINITY, f32::NEG_INFINITY]);
    let mut geometry = SkeletonIkGeometry::new();
    geometry
        .try_construct(&mut deck, &mut numeric, &mut rounding)
        .unwrap();
    assert_eq!(
        geometry.box_half_width().unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(
        geometry.box_half_length().unwrap().to_bits(),
        f32::INFINITY.to_bits()
    );
    assert!(geometry.total_half_length().unwrap().is_nan());
    let before = snapshot(&geometry);
    deck.values = [100.0; 4];
    assert_eq!(
        geometry.try_construct(&mut deck, &mut numeric, &mut rounding),
        Err(SkeletonIkGeometryError::AlreadyAttempted(
            SkeletonIkGeometryProgress::Complete
        ))
    );
    assert_eq!(snapshot(&geometry), before);
    assert_eq!(deck.trace.borrow().events.len(), 11);
}
