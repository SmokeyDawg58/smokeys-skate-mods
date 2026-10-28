use super::*;
struct Source {
    state: i32,
    timer: f32,
    total: f32,
    events: Vec<String>,
    requests: Vec<usize>,
    all: bool,
    fail: Option<Attribute>,
    forces: [f32; 8],
    nan_vehicle: bool,
    scalar_override: Option<(Scalar, f32)>,
    attribute_override: Option<(Attribute, f32)>,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            state: 1,
            timer: 0.0,
            total: 0.0,
            events: Vec::new(),
            requests: Vec::new(),
            all: false,
            fail: None,
            forces: [0.0; 8],
            nan_vehicle: false,
            scalar_override: None,
            attribute_override: None,
        }
    }
}
impl GroundWipeoutSource for Source {
    type Error = &'static str;
    type AttributePointer = Attribute;
    type DrawTarget = ();
    fn state(&mut self) -> Result<i32, Self::Error> {
        self.events.push("state".into());
        Ok(self.state)
    }
    fn set_state(&mut self, value: i32) -> Result<(), Self::Error> {
        self.events.push("set_state".into());
        self.state = value;
        Ok(())
    }
    fn off_balance_total(&mut self) -> Result<f32, Self::Error> {
        self.events.push("total".into());
        Ok(self.total)
    }
    fn set_off_balance_total(&mut self, v: f32) -> Result<(), Self::Error> {
        self.events.push("set_total".into());
        self.total = v;
        Ok(())
    }
    fn invincible_timer(&mut self) -> Result<f32, Self::Error> {
        self.events.push("timer".into());
        Ok(self.timer)
    }
    fn set_invincible_timer(&mut self, v: f32) -> Result<(), Self::Error> {
        self.events.push("set_timer".into());
        self.timer = v;
        Ok(())
    }
    fn scalar(&mut self, f: Scalar) -> Result<f32, Self::Error> {
        self.events.push(format!("scalar:{f:?}"));
        if let Some((field, value)) = self.scalar_override {
            if field == f {
                return Ok(value);
            }
        }
        Ok(match f {
            Scalar::TimeDelta => 0.1,
            Scalar::MaxDisplacement if self.all => 2.0,
            Scalar::VehicleContact if self.nan_vehicle => f32::NAN,
            _ => 0.0,
        })
    }
    fn processed_flag(&mut self, o: u32, m: u32) -> Result<bool, Self::Error> {
        self.events.push(format!("flag:{o:x}:{m:x}"));
        Ok(false)
    }
    fn collision_flag(&mut self, f: CollisionFlag) -> Result<bool, Self::Error> {
        self.events.push(format!("collision:{f:?}"));
        Ok(self.all && f != CollisionFlag::HitVehicle)
    }
    fn board_contact_flag(&mut self, m: u32) -> Result<bool, Self::Error> {
        self.events.push(format!("board:{m:x}"));
        Ok(false)
    }
    fn num_parts_in_contact(&mut self) -> Result<u8, Self::Error> {
        self.events.push("parts".into());
        Ok(u8::from(self.all))
    }
    fn vector(&mut self, f: Vector) -> Result<SkeletonRootVector, Self::Error> {
        self.events.push(format!("vector:{f:?}"));
        Ok(SkeletonRootVector::known(match f {
            Vector::MaxContactVelocity => [2.0, 0.0, 0.0, 0.0],
            Vector::MostVerticalContactNormal => [0.0, 1.0, 0.0, 0.0],
            Vector::HookToHips => [0.0; 4],
        }))
    }
    fn matrix(&mut self, f: Matrix) -> Result<SkeletonRootMatrix, Self::Error> {
        self.events.push(format!("matrix:{f:?}"));
        Ok(SkeletonRootMatrix::known([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]))
    }
    fn contact_force(&mut self, i: usize) -> Result<f32, Self::Error> {
        self.events.push(format!("force:{i}"));
        Ok(self.forces[i])
    }
    fn attribute(&mut self, a: Attribute) -> Result<Attribute, Self::Error> {
        self.events.push(format!("lookup:{a:?}"));
        if self.fail == Some(a) {
            Err("attribute")
        } else {
            Ok(a)
        }
    }
    fn attribute_value(&mut self, a: &Attribute) -> Result<f32, Self::Error> {
        self.events.push(format!("value:{a:?}"));
        if let Some((attribute, value)) = self.attribute_override {
            if attribute == *a {
                return Ok(value);
            }
        }
        Ok(match a {
            Attribute::BalanceSpeed => 1.0,
            Attribute::BalanceDecay => 0.0,
            Attribute::Vehicle | Attribute::GroundFlip if self.all => -1.0,
            Attribute::BalanceLimit if self.all => 0.0,
            Attribute::Squash | Attribute::GroundXZ | Attribute::GroundY | Attribute::LeaningUp
                if self.all =>
            {
                1.0
            }
            _ => 10.0,
        })
    }
    fn capture_draw_target(&mut self) -> Result<(), Self::Error> {
        self.events.push("draw_capture".into());
        Ok(())
    }
    fn read_draw_target(&mut self, (): &()) -> Result<bool, Self::Error> {
        self.events.push("draw_value".into());
        Ok(false)
    }
    fn wipeout_on_collision(&mut self) -> Result<bool, Self::Error> {
        self.events.push("collision_cvar".into());
        Ok(false)
    }
    fn draw_squashed(&mut self) -> Result<(), Self::Error> {
        Err("unmounted debug")
    }
    fn draw_displacement(&mut self) -> Result<(), Self::Error> {
        Err("unmounted debug")
    }
    fn draw_contact_force(&mut self, _: f32, _: f32) -> Result<(), Self::Error> {
        Err("unmounted debug")
    }
    fn require_positive_balance_speed(&mut self, _: f32) -> Result<(), Self::Error> {
        Err("balance assertion")
    }
    fn dot(&mut self, a: SkeletonRootVector, b: SkeletonRootVector) -> Result<f32, Self::Error> {
        Ok((a.xyz[0] * b.xyz[0] + a.xyz[1] * b.xyz[1]) + a.xyz[2] * b.xyz[2])
    }
    fn length(&mut self, v: SkeletonRootVector) -> Result<f32, Self::Error> {
        Ok(self.dot(v, v)?.sqrt())
    }
    fn acos(&mut self, v: f32) -> Result<f32, Self::Error> {
        Ok(v.acos())
    }
    fn minimum(&mut self, a: f32, b: f32) -> Result<f32, Self::Error> {
        Ok(a.min(b))
    }
    fn maximum(&mut self, a: f32, b: f32) -> Result<f32, Self::Error> {
        Ok(a.max(b))
    }
    fn request(&mut self, i: usize) -> Result<(), Self::Error> {
        self.events.push(format!("request:{i}"));
        self.requests.push(i);
        Ok(())
    }
}
#[test]
fn positive_timer_crosses_zero_without_running_detectors() {
    {
        let timer = 0.04;
        let mut s = Source {
            state: 0,
            timer,
            total: 17.0,
            ..Source::default()
        };
        try_check_ground(&mut s).unwrap();
        assert_eq!(s.state, 1);
        assert_eq!(s.total.to_bits(), 0.0_f32.to_bits());
        assert_eq!(
            s.events,
            [
                "state",
                "set_total",
                "set_state",
                "timer",
                "scalar:TimeDelta",
                "set_timer"
            ]
        );
        assert!(s.requests.is_empty());
        assert!(s.timer < 0.0);
    }
}
#[test]
fn actual_detectors_continue_after_first_request_and_do_not_clear_ledger() {
    let mut s = Source {
        all: true,
        ..Source::default()
    };
    try_check_ground(&mut s).unwrap();
    assert_eq!(s.requests, [18, 7, 3, 2, 11, 16, 19]);
    assert!(!s.events.iter().any(|e| e == "lookup:GroundDisplacement"));
    try_check_ground(&mut s).unwrap();
    assert_eq!(&s.requests[7..], [18, 7, 3, 2, 11, 16, 19]);
}
#[test]
fn no_hit_path_loads_arm_pointer_then_body_before_arm_value() {
    let mut s = Source::default();
    try_check_ground(&mut s).unwrap();
    let i = s
        .events
        .iter()
        .position(|e| e == "lookup:GroundArms")
        .unwrap();
    assert_eq!(
        &s.events[i..i + 4],
        [
            "lookup:GroundArms",
            "lookup:GroundContact",
            "value:GroundArms",
            "value:GroundContact"
        ]
    );
    let i = s.events.iter().position(|e| e == "force:0").unwrap();
    assert_eq!(
        &s.events[i..i + 9],
        [
            "force:0",
            "force:1",
            "force:5",
            "force:4",
            "draw_value",
            "force:7",
            "force:6",
            "force:2",
            "force:3"
        ]
    );
    assert!(s.requests.is_empty());
}
#[test]
fn late_vault_failure_keeps_squash_and_vehicle_requests() {
    let mut s = Source {
        all: true,
        fail: Some(Attribute::GroundFlip),
        ..Source::default()
    };
    assert_eq!(try_check_ground(&mut s), Err("attribute"));
    assert_eq!(s.requests, [18, 7]);
}
#[test]
fn raw_gt_clear_branches_skip_unordered_vehicle_and_force_arms() {
    let mut s = Source {
        nan_vehicle: true,
        ..Source::default()
    };
    leaves::vehicle(&mut s).unwrap();
    assert!(s.requests.is_empty());
    s.forces[2] = f32::NAN;
    s.forces[3] = f32::NAN;
    assert!(!leaves::contact_force(&mut s, 10.0, 10.0).unwrap());
}

#[test]
fn unordered_timer_enters_detector_without_decrementing_or_clearing_requests() {
    let mut s = Source {
        timer: f32::NAN,
        requests: vec![21],
        fail: Some(Attribute::Squash),
        ..Source::default()
    };
    assert_eq!(try_check_ground(&mut s), Err("attribute"));
    assert!(s.timer.is_nan());
    assert_eq!(s.requests, [21]);
    assert!(
        !s.events
            .iter()
            .any(|e| e == "set_timer" || e == "scalar:TimeDelta")
    );
    assert!(s.events.iter().any(|e| e == "lookup:Squash"));
}
#[test]
fn unordered_thresholds_and_carried_total_do_not_request_shared_wipeouts() {
    let mut s = Source {
        all: true,
        attribute_override: Some((Attribute::Vehicle, f32::NAN)),
        ..Source::default()
    };
    leaves::vehicle(&mut s).unwrap();
    leaves::deck_acceleration(&mut s, 10.0, f32::NAN).unwrap();
    s.attribute_override = None;
    s.total = f32::NAN;
    leaves::off_balance(&mut s).unwrap();
    s.scalar_override = Some((Scalar::UpY, f32::NAN));
    s.events.clear();
    leaves::leaning(&mut s).unwrap();
    assert!(s.requests.is_empty());
    assert!(!s.events.iter().any(|e| e.starts_with("collision:")));
    s.scalar_override = None;
    s.attribute_override = Some((Attribute::LeaningUp, f32::NAN));
    leaves::leaning(&mut s).unwrap();
    assert!(s.requests.is_empty());
    s.forces = [0.0; 8];
    assert!(!leaves::contact_force(&mut s, f32::NAN, f32::NAN).unwrap());
    s.forces[0] = 20.0;
    assert!(leaves::contact_force(&mut s, 10.0, f32::NAN).unwrap());
}
