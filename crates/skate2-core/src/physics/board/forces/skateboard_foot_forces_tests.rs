use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Lookup(u64),
    Read(u64),
    Balance,
    Speed,
    BodyY,
    Up,
    Velocity,
    Dot(u32),
    Seed,
    Target,
    Draw,
    Overlay,
}
type Log = Rc<RefCell<Vec<Event>>>;

struct Inputs {
    log: Log,
    balance: f32,
    speed: f32,
    y: SkeletonRootVector,
    up: SkeletonRootVector,
    velocity: SkeletonRootVector,
}
impl SkateboardFootInputRead for Inputs {
    type Error = &'static str;
    fn balance(&mut self) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(Event::Balance);
        Ok(self.balance)
    }
    fn surface_speed(&mut self) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(Event::Speed);
        Ok(self.speed)
    }
    fn body_y_direction(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.log.borrow_mut().push(Event::BodyY);
        Ok(self.y)
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.log.borrow_mut().push(Event::Velocity);
        Ok(self.velocity)
    }
    fn reckoning_up(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.log.borrow_mut().push(Event::Up);
        Ok(self.up)
    }
}
struct Attributes {
    log: Log,
    values: BTreeMap<u64, f32>,
    fail: Option<Event>,
}
impl SkateboardFootAttributeRead for Attributes {
    type Pointer = u64;
    type Error = &'static str;
    fn lookup(&mut self, key: u64) -> Result<u64, Self::Error> {
        let event = Event::Lookup(key);
        self.log.borrow_mut().push(event.clone());
        if self.fail.as_ref() == Some(&event) {
            Err("lookup")
        } else {
            Ok(key)
        }
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        let event = Event::Read(*key);
        self.log.borrow_mut().push(event.clone());
        if self.fail.as_ref() == Some(&event) {
            Err("read")
        } else {
            Ok(self.values[key])
        }
    }
}
struct Debug {
    log: Log,
    draw: bool,
    fail_overlay: bool,
}
impl SkateboardFootDebugRead for Debug {
    type Target = u32;
    type Error = &'static str;
    fn capture_draw_absorption_target(&mut self) -> Result<u32, Self::Error> {
        self.log.borrow_mut().push(Event::Target);
        Ok(7)
    }
    fn draw_absorption(&mut self, target: &u32) -> Result<bool, Self::Error> {
        assert_eq!(*target, 7);
        self.log.borrow_mut().push(Event::Draw);
        Ok(self.draw)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push(Event::Overlay);
        if self.fail_overlay {
            Err("overlay")
        } else {
            Ok(())
        }
    }
}
struct Numeric {
    log: Log,
    dots: u32,
    fail_dot: Option<u32>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.dots += 1;
        self.log.borrow_mut().push(Event::Dot(self.dots));
        if self.fail_dot == Some(self.dots) {
            Err("dot")
        } else {
            HostReckoningNumeric.dot3(a, b).map_err(|_| "host")
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(Event::Seed);
        HostReckoningNumeric
            .reciprocal_sqrt_estimate(v)
            .map_err(|_| "host")
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric
            .reciprocal_estimate(v)
            .map_err(|_| "host")
    }
    fn truncate_i32(&mut self, v: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric.truncate_i32(v).map_err(|_| "host")
    }
}
struct Fixture {
    input: Inputs,
    attributes: Attributes,
    debug: Debug,
    numeric: Numeric,
    log: Log,
}
impl Fixture {
    fn new() -> Self {
        let log = Rc::new(RefCell::new(Vec::new()));
        Self {
            input: Inputs {
                log: Rc::clone(&log),
                balance: 0.0,
                speed: 8.0,
                y: SkeletonRootVector::known([0.0, 1.0, 0.0, 2.0]),
                up: SkeletonRootVector::known([0.0, 1.0, 0.0, 3.0]),
                velocity: SkeletonRootVector::known([1.0, 0.0, 0.0, 4.0]),
            },
            attributes: Attributes {
                log: Rc::clone(&log),
                values: BTreeMap::from([
                    (FOOT_OFFSET, 0.25),
                    (REAR_ABSORPTION_SCALE, 0.5),
                    (LANDING_DENOMINATOR, 0.5),
                    (SPEED_COEFFICIENT, 20.0),
                    (INITIAL_COEFFICIENT, 100.0),
                ]),
                fail: None,
            },
            debug: Debug {
                log: Rc::clone(&log),
                draw: false,
                fail_overlay: false,
            },
            numeric: Numeric {
                log: Rc::clone(&log),
                dots: 0,
                fail_dot: None,
            },
            log,
        }
    }
    fn services(&mut self) -> SkateboardFootForceServices<'_, Inputs, Attributes, Debug, Numeric> {
        SkateboardFootForceServices {
            input: &mut self.input,
            attributes: &mut self.attributes,
            debug: &mut self.debug,
            numeric: &mut self.numeric,
        }
    }
}
fn sentinel() -> SkateboardFootForce {
    SkateboardFootForce {
        world_space_force: SkeletonRootVector::known([11.0, 12.0, 13.0, 14.0]),
        local_space_position: SkeletonRootVector::known([15.0, 16.0, 17.0, 18.0]),
    }
}
fn assert_prefix(output: SkateboardFootForce, displacement: f32) {
    assert_eq!(output.world_space_force, ZERO);
    assert_eq!(
        output.local_space_position,
        SkeletonRootVector::known([0.0, 0.0, displacement, 0.0])
    );
}

#[test]
fn front_lookup_load_numeric_and_cvar_order_preserves_full_lanes() {
    let mut f = Fixture::new();
    let mut output = sentinel();
    f.services().try_front(1.0, 0.0, &mut output).unwrap();
    assert_eq!(
        output.world_space_force.xyz.map(f32::to_bits),
        [0, (-20.0_f32).to_bits(), 0]
    );
    assert_eq!(
        output.world_space_force.w.unwrap().to_bits(),
        (-60.0_f32).to_bits()
    );
    assert_eq!(
        output.local_space_position,
        SkeletonRootVector::known([0.0, 0.0, 0.25, 0.0])
    );
    assert_eq!(
        *f.log.borrow(),
        vec![
            Event::Balance,
            Event::Lookup(FOOT_OFFSET),
            Event::Read(FOOT_OFFSET),
            Event::Lookup(LANDING_DENOMINATOR),
            Event::Read(LANDING_DENOMINATOR),
            Event::Balance,
            Event::Speed,
            Event::Lookup(SPEED_COEFFICIENT),
            Event::Lookup(INITIAL_COEFFICIENT),
            Event::Read(SPEED_COEFFICIENT),
            Event::Read(INITIAL_COEFFICIENT),
            Event::BodyY,
            Event::Up,
            Event::Velocity,
            Event::Dot(1),
            Event::Seed,
            Event::Target,
            Event::Dot(2),
            Event::Draw
        ]
    );
}

#[test]
fn balance_branch_table_includes_front_nan_two_rear_nan_zero_and_signed_zeros() {
    for (balance, front, rear) in [
        (f32::NEG_INFINITY, 40.0_f32, 0.0_f32),
        (-1.0, 40.0, 0.0),
        (-0.0, 20.0, 20.0),
        (0.0, 20.0, 20.0),
        (1.0, 0.0, 40.0),
        (f32::INFINITY, 0.0, 40.0),
        (f32::NAN, 40.0, 0.0),
    ] {
        for (is_rear, expected) in [(false, front), (true, rear)] {
            let mut f = Fixture::new();
            f.input.balance = balance;
            let mut output = sentinel();
            if is_rear {
                f.services().try_rear(1.0, 0.0, &mut output).unwrap();
            } else {
                f.services().try_front(1.0, 0.0, &mut output).unwrap();
            }
            assert_eq!(
                output.world_space_force.xyz[1].abs().to_bits(),
                expected.to_bits()
            );
        }
    }
}

#[test]
fn rear_reads_scale_before_balance_and_preserves_negative_zero_offset() {
    let mut f = Fixture::new();
    f.attributes.values.insert(FOOT_OFFSET, 0.0);
    let mut output = sentinel();
    f.services().try_rear(1.0, 2.0, &mut output).unwrap();
    assert_eq!(output.local_space_position.xyz[2].to_bits(), 0x8000_0000);
    assert_eq!(
        output.world_space_force.xyz[1].to_bits(),
        (-21.0_f32).to_bits()
    );
    assert_eq!(
        &f.log.borrow()[..5],
        &[
            Event::Lookup(REAR_ABSORPTION_SCALE),
            Event::Read(REAR_ABSORPTION_SCALE),
            Event::Balance,
            Event::Lookup(FOOT_OFFSET),
            Event::Read(FOOT_OFFSET)
        ]
    );
}

#[test]
fn early_attribute_failure_keeps_caller_output_and_defers_speed_value_until_initial_lookup() {
    for event in [
        Event::Lookup(FOOT_OFFSET),
        Event::Lookup(INITIAL_COEFFICIENT),
        Event::Read(SPEED_COEFFICIENT),
    ] {
        let mut f = Fixture::new();
        f.attributes.fail = Some(event.clone());
        let mut output = sentinel();
        assert!(matches!(
            f.services().try_front(1.0, 0.0, &mut output),
            Err(SkateboardFootForceError::Attribute(_))
        ));
        assert_eq!(output, sentinel());
        assert_eq!(f.log.borrow().last(), Some(&event));
        if event == Event::Lookup(INITIAL_COEFFICIENT) {
            assert!(!f.log.borrow().contains(&Event::Read(SPEED_COEFFICIENT)));
        }
    }
}

#[test]
fn numerical_failure_preserves_output_prefix_and_cvar_target_order() {
    for dot in [1, 2] {
        let mut f = Fixture::new();
        f.numeric.fail_dot = Some(dot);
        let mut output = sentinel();
        assert!(matches!(
            f.services().try_front(1.0, 0.0, &mut output),
            Err(SkateboardFootForceError::Numeric("dot"))
        ));
        assert_prefix(output, 0.25);
        assert_eq!(f.log.borrow().contains(&Event::Target), dot == 2);
        assert!(!f.log.borrow().contains(&Event::Draw));
    }
}

#[test]
fn late_overlay_failure_retains_zero_force_local_position_after_numeric_completion() {
    let mut f = Fixture::new();
    f.debug.draw = true;
    f.debug.fail_overlay = true;
    let mut output = sentinel();
    assert!(matches!(
        f.services().try_rear(1.0, 0.0, &mut output),
        Err(SkateboardFootForceError::Debug("overlay"))
    ));
    assert_prefix(output, -0.25);
    let log = f.log.borrow();
    assert_eq!(
        &log[log.len() - 3..],
        &[Event::Dot(2), Event::Draw, Event::Overlay]
    );
}

#[test]
fn force_vector_uses_fma_after_separately_rounded_absorption_product() {
    let mut f = Fixture::new();
    f.attributes
        .values
        .insert(SPEED_COEFFICIENT, f32::from_bits(0x3f80_0001));
    f.input.up = SkeletonRootVector::known([0.0, f32::from_bits(0x3f80_0001), 0.0, 0.0]);
    f.input.y = SkeletonRootVector::known([0.0, -f32::from_bits(0x3f80_0002), 0.0, 0.0]);
    f.input.velocity = ZERO;
    let mut output = sentinel();
    f.services().try_front(1.0, 1.0, &mut output).unwrap();
    // (1+2^-23)*(-1-2^-23)+(1+2^-22) = -2^-46. An unfused multiply loses it.
    assert_eq!(output.world_space_force.xyz[1].to_bits(), 0xa880_0000);
    assert!(!f.log.borrow().contains(&Event::Seed));
}

#[test]
fn unknown_w_propagates_even_for_zero_absorption_and_normalize_fallback_recovers_zero() {
    for missing in 0..3 {
        let mut f = Fixture::new();
        match missing {
            0 => f.input.y.w = None,
            1 => f.input.up.w = None,
            _ => f.input.velocity.w = None,
        }
        let mut output = sentinel();
        f.services().try_front(1.0, 0.0, &mut output).unwrap();
        assert_eq!(output.world_space_force.w, None);
        assert_eq!(output.local_space_position.w.unwrap().to_bits(), 0);
    }
    let mut f = Fixture::new();
    f.input.velocity = SkeletonRootVector {
        xyz: [0.0; 3],
        w: None,
    };
    let mut output = sentinel();
    f.services().try_front(1.0, 0.0, &mut output).unwrap();
    assert_eq!(
        output.world_space_force.w.unwrap().to_bits(),
        (-60.0_f32).to_bits()
    );
}

#[test]
fn landing_blend_signed_zero_time_and_nan_denominator_follow_source_selects() {
    for time in [0.0, -0.0] {
        let mut f = Fixture::new();
        let mut output = sentinel();
        f.services().try_front(time, 0.0, &mut output).unwrap();
        assert_eq!(output.world_space_force.xyz[1].abs().to_bits(), 0);
        f.input.balance = -1.0;
        f.services().try_front(time, 0.0, &mut output).unwrap();
        assert_eq!(
            output.world_space_force.xyz[1].to_bits(),
            (-200.0_f32).to_bits()
        );
    }
    let mut f = Fixture::new();
    f.attributes.values.insert(LANDING_DENOMINATOR, f32::NAN);
    let mut output = sentinel();
    f.services().try_front(1.0, 0.0, &mut output).unwrap();
    assert_eq!(
        output.world_space_force.xyz[1].to_bits(),
        (-20.0_f32).to_bits()
    );
}

#[test]
fn landing_ratio_clamps_ordered_limits_but_nan_time_selects_upper_endpoint() {
    for (time, expected) in [
        (-1.0_f32, 100.0_f32),
        (f32::NEG_INFINITY, 100.0),
        (0.25, 60.0),
        (f32::INFINITY, 20.0),
        (f32::NAN, 20.0),
    ] {
        let mut fixture = Fixture::new();
        let mut output = sentinel();
        fixture
            .services()
            .try_front(time, 0.0, &mut output)
            .unwrap();
        assert_eq!(
            output.world_space_force.xyz[1].to_bits(),
            (-expected).to_bits()
        );
    }
    let mut fixture = Fixture::new();
    fixture.attributes.values.insert(LANDING_DENOMINATOR, 0.0);
    let mut output = sentinel();
    fixture.services().try_front(0.0, 0.0, &mut output).unwrap();
    assert_eq!(
        output.world_space_force.xyz[1].to_bits(),
        (-20.0_f32).to_bits()
    );
}
