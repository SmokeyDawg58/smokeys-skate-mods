use super::*;
#[derive(Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source branch controls in focused test probe"
)]
struct Probe {
    events: Vec<String>,
    fail: Option<&'static str>,
    velocity: f32,
    attribute: Option<f32>,
    prepare: bool,
    full: Option<f32>,
    custom: bool,
    collision: f32,
    earlier: f32,
    offboard: bool,
    dt: f32,
    enabled: bool,
    speed: Option<f32>,
    minimum: Option<f32>,
}
impl Probe {
    fn event(&mut self, name: impl Into<String>) -> Result<(), &'static str> {
        let name = name.into();
        self.events.push(name.clone());
        if self.fail == Some(name.as_str()) {
            Err("injected")
        } else {
            Ok(())
        }
    }
}
impl Source for Probe {
    type Error = &'static str;
    fn resolve(&mut self, s: Resolve) -> Result<(), Self::Error> {
        self.event(format!("resolve:{s:?}"))
    }
    fn value(&mut self, v: Value) -> Result<f32, Self::Error> {
        self.event(format!("value:{v:?}"))?;
        Ok(match v {
            Value::VelocityY => self.velocity,
            Value::ComToDeck => 1.0,
            Value::CollisionTime => self.collision,
        })
    }
    fn drive_attribute(&mut self) -> Result<Option<f32>, Self::Error> {
        self.event("drive")?;
        Ok(self.attribute)
    }
    fn initial_distance(&mut self, d: &mut AirLegInstance) -> Result<(), Self::Error> {
        d.distance = 3.0;
        self.event("geometry")
    }
    fn tuning(&mut self, t: Tuning) -> Result<f32, Self::Error> {
        self.event(format!("tuning:{t:?}"))?;
        Ok(match t {
            Tuning::Minimum => self.minimum.unwrap_or(0.25),
            Tuning::FinalDistance => 2.0,
            _ => self.speed.unwrap_or(1.0),
        })
    }
    fn cvar(&mut self, c: Cvar) -> Result<bool, Self::Error> {
        self.event(format!("cvar:{c:?}"))?;
        Ok(match c {
            Cvar::Debug => false,
            Cvar::Custom => self.custom,
            _ => self.enabled,
        })
    }
    fn delta_time(&mut self) -> Result<f32, Self::Error> {
        self.event("dt")?;
        Ok(self.dt)
    }
    fn downward_projection(&mut self) -> Result<(f32, f32), Self::Error> {
        self.event("projection")?;
        Ok((0.5, 1.0))
    }
    fn prepare_land(&mut self) -> Result<bool, Self::Error> {
        self.event("prepare")?;
        Ok(self.prepare)
    }
    fn full_extension(&mut self) -> Result<Option<FullValue>, Self::Error> {
        self.event("full")?;
        Ok(self.full.map(FullValue::Known))
    }
    fn custom_earlier(&mut self) -> Result<f32, Self::Error> {
        self.event("earlier")?;
        Ok(self.earlier)
    }
    fn offboard_bone(&self) -> bool {
        self.offboard
    }
    fn write_drive(&mut self, _: f32) -> Result<(), Self::Error> {
        self.event("write_drive")
    }
    fn extend_graph(&mut self, _: f32) -> Result<f32, Self::Error> {
        self.event("graph")?;
        Ok(0.0)
    }
    fn write_extend(&mut self, _: f32) -> Result<(), Self::Error> {
        self.event("write_extend")
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.event("overlay")
    }
    fn missing_attribute_word(&self) -> Self::Error {
        "attribute word"
    }
    fn missing_instance(&self) -> Self::Error {
        "instance"
    }
}
#[test]
fn constructor_and_failed_initial_attribute_keep_native_phase_prefix() {
    assert!(AirLegInstance::after_allocation(false).is_none());
    let mut d = AirLegInstance::after_allocation(true).unwrap();
    assert_eq!(d.phase, 0);
    assert_eq!(d.distance.to_bits(), 0);
    assert_eq!(d.down_time.to_bits(), 0);
    let mut p = Probe {
        velocity: 1.0,
        fail: Some("drive"),
        ..Probe::default()
    };
    assert_eq!(update(Some(&mut d), &mut p), Err("injected"));
    assert_eq!(d.phase, 1);
    assert_eq!(p.events, ["resolve:Update", "value:VelocityY", "drive"]);
}
#[test]
fn initial_distance_partial_store_and_late_graph_failure_are_retained() {
    let mut d = AirLegInstance::default();
    let mut p = Probe {
        fail: Some("geometry"),
        ..Probe::default()
    };
    assert!(update(Some(&mut d), &mut p).is_err());
    assert_eq!(d.phase, 2);
    assert_eq!(d.distance.to_bits(), 3.0f32.to_bits());
    let mut d = AirLegInstance::default();
    let mut p = Probe {
        attribute: Some(7.0),
        dt: 0.125,
        fail: Some("graph"),
        ..Probe::default()
    };
    assert!(update(Some(&mut d), &mut p).is_err());
    assert_eq!(d.distance.to_bits(), 7.0f32.to_bits());
    assert_eq!(d.down_time.to_bits(), 0.125f32.to_bits());
    assert_eq!(
        p.events,
        [
            "resolve:Update",
            "value:VelocityY",
            "drive",
            "write_drive",
            "dt",
            "graph"
        ]
    );
}
#[test]
fn up_nan_stays_up_and_fsel_nan_candidate_chooses_minimum() {
    let mut d = AirLegInstance {
        phase: 1,
        distance: f32::NAN,
        down_time: 0.0,
    };
    let mut p = Probe {
        velocity: f32::NAN,
        enabled: true,
        dt: 0.1,
        ..Probe::default()
    };
    update(Some(&mut d), &mut p).unwrap();
    assert_eq!(d.phase, 1);
    assert_eq!(d.distance.to_bits(), 0.25f32.to_bits());
    assert!(!p.events.iter().any(|s| s == "prepare"));
}
#[test]
fn disabled_up_reads_both_attributes_but_no_inner_dt() {
    let mut d = AirLegInstance {
        phase: 1,
        distance: 2.0,
        down_time: 4.0,
    };
    let mut p = Probe {
        velocity: 1.0,
        ..Probe::default()
    };
    update(Some(&mut d), &mut p).unwrap();
    assert_eq!(
        p.events,
        [
            "resolve:Update",
            "value:VelocityY",
            "resolve:Up",
            "value:VelocityY",
            "tuning:UpSpeed",
            "tuning:Minimum",
            "cvar:Up",
            "cvar:Debug",
            "write_drive",
            "graph",
            "write_extend"
        ]
    );
    assert_eq!(d.distance.to_bits(), 2.0f32.to_bits());
    assert_eq!(d.down_time.to_bits(), 0);
}
#[test]
fn custom_preland_unordered_comparison_is_false_without_overlay() {
    let mut p = Probe {
        prepare: true,
        full: Some(2.0),
        custom: true,
        collision: f32::NAN,
        ..Probe::default()
    };
    assert!(!should_prepare(&mut p).unwrap());
    assert!(!p.events.iter().any(|s| s == "overlay"));
    p.collision = 0.0;
    p.events.clear();
    assert!(should_prepare(&mut p).unwrap());
    assert_eq!(p.events.last().unwrap(), "overlay");
}
#[test]
fn down_to_preland_keeps_phase_if_later_tuning_fails() {
    let mut d = AirLegInstance {
        phase: 2,
        distance: 2.0,
        down_time: 0.0,
    };
    let mut p = Probe {
        prepare: true,
        fail: Some("tuning:PrelandSpeed"),
        ..Probe::default()
    };
    assert!(update(Some(&mut d), &mut p).is_err());
    assert_eq!(d.phase, 3);
    assert_eq!(d.distance.to_bits(), 2.0f32.to_bits());
    assert!(!p.events.iter().any(|s| s == "write_drive"));
}

#[test]
fn upward_adjustment_keeps_single_fused_subtract_rounding() {
    let factor = f32::from_bits(0x3f80_0001);
    let old = f32::from_bits(0x3f80_0002);
    let mut data = AirLegInstance {
        phase: 1,
        distance: old,
        down_time: 0.0,
    };
    let mut probe = Probe {
        velocity: 1.0,
        enabled: true,
        dt: factor,
        speed: Some(factor),
        minimum: Some(-1.0),
        ..Probe::default()
    };
    update(Some(&mut data), &mut probe).unwrap();
    let fused = (-factor).mul_add(factor, old);
    assert_eq!(data.distance.to_bits(), fused.to_bits());
    assert_ne!(data.distance.to_bits(), (old - factor * factor).to_bits());
}
