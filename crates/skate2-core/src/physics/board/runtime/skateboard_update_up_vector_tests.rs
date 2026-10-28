use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Body,
    Count,
    Collision,
    Contact(usize),
    Acceleration(usize),
    Processed,
    SurfaceSpeed,
    Average,
    PhysicsGlobal,
    PhysicsLayout,
    Gravity,
    Global,
    Lookup(u64),
    Float(u64),
    Layout,
    Graph,
    Dot,
    Estimate,
}
#[derive(Clone, Default)]
struct Log(Rc<RefCell<(Vec<Event>, Option<usize>)>>);
impl Log {
    fn call(&self, event: Event) -> Result<(), &'static str> {
        let mut log = self.0.borrow_mut();
        let index = log.0.len();
        log.0.push(event);
        if log.1 == Some(index) {
            Err("injected")
        } else {
            Ok(())
        }
    }
    fn events(&self) -> Vec<Event> {
        self.0.borrow().0.clone()
    }
}
struct Input {
    log: Log,
    count: u8,
    contacts: [bool; 4],
    unknown_w: bool,
}
impl SkateboardUpdateUpInput for Input {
    type Error = &'static str;
    type Body = ();
    type Collision = ();
    type Processed = ();
    fn capture_body(&mut self) -> Result<(), Self::Error> {
        self.log.call(Event::Body)
    }
    fn num_wheels(&mut self, (): &()) -> Result<u8, Self::Error> {
        self.log.call(Event::Count)?;
        Ok(self.count)
    }
    fn capture_collision(&mut self) -> Result<(), Self::Error> {
        self.log.call(Event::Collision)
    }
    fn in_contact(&mut self, (): &(), wheel: usize) -> Result<bool, Self::Error> {
        self.log.call(Event::Contact(wheel))?;
        Ok(self.contacts[wheel])
    }
    fn acceleration(&mut self, (): &(), wheel: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.log.call(Event::Acceleration(wheel))?;
        Ok(SkeletonRootVector {
            xyz: [0.0, 1.0, 0.0],
            w: if self.unknown_w { None } else { Some(0.0) },
        })
    }
    fn capture_processed(&mut self) -> Result<(), Self::Error> {
        self.log.call(Event::Processed)
    }
    fn surface_speed(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.log.call(Event::SurfaceSpeed)?;
        Ok(-2.0)
    }
    fn average_normal(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.log.call(Event::Average)?;
        Ok(SkeletonRootVector {
            xyz: [0.0, 0.8, 0.6],
            w: None,
        })
    }
}
struct Attributes {
    log: Log,
}
impl SkateboardGroundNormalAttributeRead for Attributes {
    type Global = ();
    type Pointer = u64;
    type Error = &'static str;
    fn capture_global(&mut self) -> Result<(), Self::Error> {
        self.log.call(Event::Global)
    }
    fn lookup(&mut self, (): &(), key: u64) -> Result<u64, Self::Error> {
        self.log.call(Event::Lookup(key))?;
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        self.log.call(Event::Float(*key))?;
        Ok(match *key {
            SPEED_DAMPING => 0.5,
            CORRECTION_SCALAR => 1.0,
            SPEED_DIVISOR => 2.0,
            DELTA_CAP_SCALAR => 0.25,
            _ => panic!("unexpected key"),
        })
    }
}
impl SkateboardUpdateUpAttributes for Attributes {
    type PhysicsGlobal = ();
    type PhysicsLayout = ();
    type ReckoningLayout = ();
    fn capture_physics_global(&mut self) -> Result<(), Self::Error> {
        self.log.call(Event::PhysicsGlobal)
    }
    fn capture_physics_layout(&mut self, (): &()) -> Result<(), Self::Error> {
        self.log.call(Event::PhysicsLayout)
    }
    fn world_gravity(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.log.call(Event::Gravity)?;
        Ok(SkeletonRootVector::known([0.0, -1.0, 0.0, 0.0]))
    }
    fn capture_reckoning_layout(&mut self, (): &()) -> Result<(), Self::Error> {
        self.log.call(Event::Layout)
    }
    fn evaluate_graph8(&mut self, (): &(), offset: usize, input: f32) -> Result<f32, Self::Error> {
        self.log.call(Event::Graph)?;
        assert_eq!(offset, 0x460);
        assert_eq!(input.to_bits(), 1.0_f32.to_bits());
        Ok(1.0)
    }
}
struct Numeric {
    log: Log,
    host: HostReckoningNumeric,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        self.log.call(Event::Dot)?;
        self.host.dot3(left, right).map_err(|_| "numeric")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.log.call(Event::Estimate)?;
        self.host
            .reciprocal_sqrt_estimate(value)
            .map_err(|_| "numeric")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("unused")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("unused")
    }
}
fn fixture() -> (SkateboardUpdateUpState, Input, Attributes, Numeric, Log) {
    let log = Log::default();
    let mut state = SkateboardUpdateUpState::uninitialized();
    state.initialize_constructor().unwrap();
    state.dynamic_up = Some(SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]));
    state.dynamic_speed = Some(SkeletonRootVector::known([0.0, 2.0, 0.0, 0.0]));
    (
        state,
        Input {
            log: log.clone(),
            count: 1,
            contacts: [true, false, false, false],
            unknown_w: false,
        },
        Attributes { log: log.clone() },
        Numeric {
            log: log.clone(),
            host: HostReckoningNumeric,
        },
        log,
    )
}
fn bits(value: Option<SkeletonRootVector>) -> ([u32; 3], Option<u32>) {
    let value = value.unwrap();
    (value.xyz.map(f32::to_bits), value.w.map(f32::to_bits))
}

#[test]
fn constructor_is_explicit_speed_is_up_and_zero_wheels_only_clear_contact_acceleration() {
    let mut state = SkateboardUpdateUpState::uninitialized();
    let (_, mut input, mut attributes, mut numeric, log) = fixture();
    assert_eq!(
        state.try_update(&mut input, &mut attributes, &mut numeric),
        Err(SkateboardUpdateUpError::ConstructorRequired)
    );
    assert!(log.events().is_empty());
    state.initialize_constructor().unwrap();
    assert_eq!(bits(state.dynamic_up), bits(Some(UP)));
    assert_eq!(bits(state.dynamic_speed), bits(Some(UP)));
    state.contact_acceleration = Some(SkeletonRootVector::known([9.0; 4]));
    state.average_normal = Some(SkeletonRootVector::known([7.0; 4]));
    input.count = 0;
    state
        .try_update(&mut input, &mut attributes, &mut numeric)
        .unwrap();
    assert_eq!(log.events(), [Event::Body, Event::Count]);
    assert_eq!(bits(state.contact_acceleration), bits(Some(ZERO)));
    assert_eq!(bits(state.dynamic_speed), bits(Some(UP)));
    assert_eq!(bits(state.dynamic_up), bits(Some(UP)));
    assert_eq!(
        bits(state.average_normal),
        bits(Some(SkeletonRootVector::known([7.0; 4])))
    );
    assert_eq!(
        state.initialize_constructor(),
        Err(SkateboardUpAlreadyInitialized)
    );
    assert_eq!(
        bits(state.average_normal),
        bits(Some(SkeletonRootVector::known([7.0; 4])))
    );
}

#[test]
fn fourth_wheel_does_not_contribute_and_gravity_is_read_after_false_first_contact() {
    let (mut state, mut input, mut attributes, mut numeric, log) = fixture();
    input.contacts = [false, false, false, true];
    state
        .try_update(&mut input, &mut attributes, &mut numeric)
        .unwrap();
    let events = log.events();
    assert_eq!(
        &events[..9],
        &[
            Event::Body,
            Event::Count,
            Event::Collision,
            Event::PhysicsGlobal,
            Event::Contact(0),
            Event::PhysicsLayout,
            Event::Gravity,
            Event::Contact(1),
            Event::Contact(2)
        ]
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Acceleration(_) | Event::Contact(3)))
    );
}

#[test]
fn deferred_cap_decode_and_all_late_store_failure_prefixes_are_source_ordered() {
    let (mut complete, mut input, mut attributes, mut numeric, log) = fixture();
    complete
        .try_update(&mut input, &mut attributes, &mut numeric)
        .unwrap();
    let events = log.events();
    let cap_read = events
        .iter()
        .position(|event| *event == Event::Float(DELTA_CAP_SCALAR))
        .unwrap();
    assert_eq!(
        &events[cap_read - 2..cap_read + 2],
        &[
            Event::Graph,
            Event::Dot,
            Event::Float(DELTA_CAP_SCALAR),
            Event::Estimate
        ]
    );
    let divisor_read = events
        .iter()
        .position(|event| *event == Event::Float(SPEED_DIVISOR))
        .unwrap();
    assert_eq!(events[divisor_read - 1], Event::Global);
    let final_dot = events
        .iter()
        .rposition(|event| *event == Event::Dot)
        .unwrap();
    let final_average = events
        .iter()
        .position(|event| *event == Event::Average)
        .unwrap();
    for failure in [cap_read, final_dot, final_average] {
        let (mut state, mut input, mut attributes, mut numeric, log) = fixture();
        log.0.borrow_mut().1 = Some(failure);
        assert!(
            state
                .try_update(&mut input, &mut attributes, &mut numeric)
                .is_err()
        );
        assert_eq!(log.events(), events[..=failure]);
        assert_eq!(bits(state.average_normal), bits(Some(ZERO)));
        if failure == cap_read {
            assert_eq!(bits(state.dynamic_speed), bits(Some(UP)));
            assert_eq!(
                bits(state.dynamic_up),
                bits(Some(SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0])))
            );
        } else {
            assert_eq!(bits(state.dynamic_speed), bits(complete.dynamic_speed));
            if failure == final_dot {
                let speed = state.dynamic_speed.unwrap();
                assert_eq!(
                    bits(state.dynamic_up),
                    bits(Some(add(
                        SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
                        speed
                    )))
                );
                assert_ne!(bits(state.dynamic_up), bits(complete.dynamic_up));
            } else {
                assert_eq!(bits(state.dynamic_up), bits(complete.dynamic_up));
            }
        }
    }
}

#[test]
fn full_update_preserves_unknown_w_and_safe_fallback_establishes_zero() {
    let (mut state, mut input, mut attributes, mut numeric, _) = fixture();
    input.unknown_w = true;
    state
        .try_update(&mut input, &mut attributes, &mut numeric)
        .unwrap();
    assert_eq!(state.dynamic_speed.unwrap().w, None);
    assert_eq!(state.dynamic_up.unwrap().w, None);
    assert_eq!(state.average_normal.unwrap().w, None);
    let zero = normalize(
        SkeletonRootVector {
            xyz: [0.0; 3],
            w: None,
        },
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(bits(Some(zero)), bits(Some(ZERO)));
    let nonzero = normalize(
        SkeletonRootVector {
            xyz: [0.0, 1.0, 0.0],
            w: None,
        },
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(nonzero.w, None);
}

#[test]
fn source_select_keeps_unordered_and_signed_zero_arm_semantics() {
    assert_eq!(select(-0.0, 2.0, 3.0).to_bits(), 2.0_f32.to_bits());
    assert_eq!(select(f32::NAN, 2.0, 3.0).to_bits(), 3.0_f32.to_bits());
    let length = f32::NAN;
    let positive = select(-length, 0.0, length);
    assert!(positive.is_nan());
    let cap = -2.0;
    assert_eq!(
        select(cap - positive, positive, cap).to_bits(),
        cap.to_bits()
    );
}
