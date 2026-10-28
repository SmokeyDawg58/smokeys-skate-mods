use super::*;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

type Log = Rc<RefCell<Vec<String>>>;
struct Input {
    log: Log,
    flags: VecDeque<u32>,
    balances: VecDeque<f32>,
    speed: f32,
    z: f32,
    push: f32,
    direction: SkeletonRootVector,
    fail: &'static str,
}
impl Input {
    fn read(&self, name: &'static str) -> Result<(), &'static str> {
        self.log.borrow_mut().push(name.into());
        if self.fail == name { Err(name) } else { Ok(()) }
    }
}
impl SkateboardBrakeDragInputRead for Input {
    type Error = &'static str;
    type Surface = u32;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        self.read("flags")?;
        Ok(self.flags.pop_front().unwrap_or(0))
    }
    fn foot_brake(&mut self) -> Result<f32, Self::Error> {
        self.read("foot")?;
        Ok(0.5)
    }
    fn capture_surface(&mut self) -> Result<u32, Self::Error> {
        self.read("surface")?;
        Ok(17)
    }
    fn z_speed(&mut self) -> Result<f32, Self::Error> {
        self.read("z")?;
        Ok(self.z)
    }
    fn surface_braking_scalar(&mut self, surface: &u32) -> Result<f32, Self::Error> {
        assert_eq!(*surface, 17);
        self.read("scalar")?;
        Ok(2.0)
    }
    fn abs_body_speed(&mut self) -> Result<f32, Self::Error> {
        self.read("speed")?;
        Ok(self.speed)
    }
    fn balance(&mut self) -> Result<f32, Self::Error> {
        self.read("balance")?;
        Ok(self.balances.pop_front().unwrap_or(0.0))
    }
    fn push(&mut self) -> Result<f32, Self::Error> {
        self.read("push")?;
        Ok(self.push)
    }
    fn body_z_direction(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("direction")?;
        Ok(self.direction)
    }
    fn reckoning_ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("normal")?;
        Ok(SkeletonRootVector {
            xyz: [0.0, 1.0, 0.0],
            w: None,
        })
    }
}
struct Attributes {
    log: Log,
    fail: Option<u64>,
    stop: f32,
}
impl SkateboardBrakeAttributeRead for Attributes {
    type Pointer = u64;
    type Error = u64;
    fn lookup(&mut self, key: u64) -> Result<u64, u64> {
        self.log.borrow_mut().push(format!("lookup:{key:x}"));
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, u64> {
        self.log.borrow_mut().push(format!("value:{key:x}"));
        if self.fail == Some(*key) {
            return Err(*key);
        }
        Ok(match *key {
            FOOT_COEFFICIENT => 50.0,
            MANUAL_COEFFICIENT => 35.0,
            STOP_SPEED => self.stop,
            BALANCE_SPEED => 1.0,
            NORMAL_Y_THRESHOLD => 0.9,
            BALANCE_DRAG => 0.125,
            _ => panic!(),
        })
    }
}
fn fixture() -> (Input, Attributes, SkateboardFootForce) {
    let log = Log::default();
    (
        Input {
            log: log.clone(),
            flags: VecDeque::from([0x6000_0000; 2]),
            balances: VecDeque::new(),
            speed: 0.1,
            z: 1.0,
            push: 0.0,
            direction: SkeletonRootVector::known([1.0, -0.0, 2.0, 3.0]),
            fail: "",
        },
        Attributes {
            log,
            fail: None,
            stop: 0.01,
        },
        SkateboardFootForce {
            world_space_force: SkeletonRootVector::known([7.0; 4]),
            local_space_position: SkeletonRootVector::known([8.0; 4]),
        },
    )
}

#[test]
fn both_flags_sample_in_order_and_manual_overrides_with_surface_capture() {
    let (mut input, mut attributes, mut output) = fixture();
    SkateboardBrakeDragServices {
        input: &mut input,
        attributes: &mut attributes,
    }
    .try_braking(&mut output)
    .unwrap();
    assert_eq!(
        output.world_space_force.xyz.map(f32::to_bits),
        [-70.0_f32, 0.0, -140.0].map(f32::to_bits)
    );
    assert_eq!(output.world_space_force.w, Some(-210.0));
    assert_eq!(
        input.log.borrow().as_slice(),
        [
            "flags",
            "lookup:d71f120752432c0a",
            "value:d71f120752432c0a",
            "foot",
            "flags",
            "lookup:c19a40df5e7b2662",
            "value:c19a40df5e7b2662",
            "surface",
            "z",
            "scalar",
            "lookup:92d0c41a3315b63d",
            "value:92d0c41a3315b63d",
            "speed",
            "direction"
        ]
    );
}

#[test]
fn failures_preserve_source_output_prefix_and_no_flags_still_reads_surface() {
    for failure in ["surface", "z", "scalar", "speed", "direction"] {
        let (mut input, mut attributes, mut output) = fixture();
        let original = output;
        input.fail = failure;
        input.flags = VecDeque::from([0, 0]);
        assert_eq!(
            SkateboardBrakeDragServices {
                input: &mut input,
                attributes: &mut attributes
            }
            .try_braking(&mut output),
            Err(SkateboardBrakeDragError::Input(failure))
        );
        if failure == "direction" {
            assert_eq!(
                output.world_space_force,
                SkeletonRootVector::known([0.0; 4])
            );
            assert_eq!(
                output.local_space_position,
                SkeletonRootVector::known([0.0; 4])
            );
        } else {
            assert_eq!(output, original);
        }
    }
    let (mut input, mut attributes, mut output) = fixture();
    let original = output;
    attributes.fail = Some(FOOT_COEFFICIENT);
    assert!(
        SkateboardBrakeDragServices {
            input: &mut input,
            attributes: &mut attributes
        }
        .try_braking(&mut output)
        .is_err()
    );
    assert_eq!(output, original);
    assert!(!input.log.borrow().iter().any(|s| s == "foot"));
}

#[test]
fn braking_nan_threshold_sign_zero_and_unknown_w_follow_raw_branches() {
    for z in [-1.0, 0.0, -0.0, f32::NAN] {
        let (mut input, mut attributes, mut output) = fixture();
        input.z = z;
        attributes.stop = f32::NAN;
        SkateboardBrakeDragServices {
            input: &mut input,
            attributes: &mut attributes,
        }
        .try_braking(&mut output)
        .unwrap();
        assert_eq!(
            output.world_space_force.xyz[0].to_bits(),
            (if z < 0.0 { 70.0_f32 } else { -70.0 }).to_bits()
        );
    }
    let (mut input, mut attributes, mut output) = fixture();
    input.speed = 0.0;
    input.direction = SkeletonRootVector {
        xyz: [f32::NAN, 1.0, -1.0],
        w: None,
    };
    SkateboardBrakeDragServices {
        input: &mut input,
        attributes: &mut attributes,
    }
    .try_braking(&mut output)
    .unwrap();
    assert!(output.world_space_force.xyz[0].is_nan());
    assert_eq!(output.world_space_force.xyz[2].to_bits(), 0x8000_0000);
    assert_eq!(output.world_space_force.w, None);
}

#[test]
fn drag_repeated_balance_can_trigger_late_error_before_stopped_override() {
    let (mut input, mut attributes, _) = fixture();
    input.speed = 0.0;
    input.push = f32::NAN;
    input.balances = VecDeque::from([-0.0, 1.0]);
    attributes.fail = Some(BALANCE_DRAG);
    assert_eq!(
        SkateboardBrakeDragServices {
            input: &mut input,
            attributes: &mut attributes
        }
        .try_linear_drag(),
        Err(SkateboardBrakeDragError::Attribute(BALANCE_DRAG))
    );
    assert_eq!(
        input
            .log
            .borrow()
            .iter()
            .filter(|s| s.as_str() == "balance")
            .count(),
        2
    );
    attributes.fail = None;
    input.flags.push_back(0x4000_0000);
    input.balances = VecDeque::from([0.0, 1.0]);
    assert_eq!(
        SkateboardBrakeDragServices {
            input: &mut input,
            attributes: &mut attributes
        }
        .try_linear_drag()
        .unwrap()
        .to_bits(),
        1.0_f32.to_bits()
    );
}

#[test]
fn drag_unordered_speed_short_circuits_but_still_reads_balance() {
    let (mut input, mut attributes, _) = fixture();
    input.speed = f32::NAN;
    input.balances = VecDeque::from([f32::NAN]);
    assert_eq!(
        SkateboardBrakeDragServices {
            input: &mut input,
            attributes: &mut attributes
        }
        .try_linear_drag()
        .unwrap()
        .to_bits(),
        0
    );
    assert_eq!(
        input.log.borrow().as_slice(),
        [
            "lookup:92d0c41a3315b63d",
            "value:92d0c41a3315b63d",
            "speed",
            "balance",
            "lookup:65468a36a6e32f03",
            "value:65468a36a6e32f03",
            "speed"
        ]
    );
}
