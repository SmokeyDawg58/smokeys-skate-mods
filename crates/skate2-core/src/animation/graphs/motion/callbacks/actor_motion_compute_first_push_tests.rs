use super::{ACTIVE, FIRST_UPDATE, FirstPushInstance, FirstPushSource, TELEPORT, begin, update};

struct Source {
    trace: Vec<&'static str>,
    teleport_time: f32,
    teleport_limit: f32,
    intent: Option<f32>,
    dt: f32,
    speed: f32,
    top_speed: f32,
    cap: bool,
    duration: f32,
    current_dv: f32,
    mapped_inputs: Vec<(usize, f32)>,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            trace: Vec::new(),
            teleport_time: 1.0,
            teleport_limit: 0.5,
            intent: Some(0.25),
            dt: 0.1,
            speed: 2.0,
            top_speed: 4.0,
            cap: false,
            duration: 0.5,
            current_dv: 0.0,
            mapped_inputs: Vec::new(),
        }
    }
}
impl FirstPushSource for Source {
    type Error = ();
    type Attribute = f32;
    fn resolve_begin(&mut self) -> Result<(), ()> {
        self.trace.push("resolve begin");
        Ok(())
    }
    fn resolve_update(&mut self) -> Result<(), ()> {
        self.trace.push("resolve update");
        Ok(())
    }
    fn time_since_teleport(&mut self) -> Result<f32, ()> {
        self.trace.push("teleport time");
        Ok(self.teleport_time)
    }
    fn capture_teleport_threshold(&mut self) -> Result<f32, ()> {
        self.trace.push("teleport threshold pointer");
        Ok(self.teleport_limit)
    }
    fn read_teleport_threshold(&mut self, value: &f32) -> Result<f32, ()> {
        self.trace.push("teleport threshold value");
        Ok(*value)
    }
    fn scalar(&mut self, key: u64) -> Result<f32, ()> {
        match key {
            0x4c4b_9cdf_5860_f565 => {
                self.trace.push("teleport threshold");
                Ok(self.teleport_limit)
            }
            0x1a4c_a176_9dcc_b3a8 => {
                self.trace.push("top speed");
                Ok(self.top_speed)
            }
            _ => panic!("unexpected scalar key"),
        }
    }
    fn has_pushing_intent(&mut self) -> Result<bool, ()> {
        self.trace.push("intent contains");
        Ok(self.intent.is_some())
    }
    fn pushing_intent_value(&mut self) -> Result<f32, ()> {
        self.trace.push("intent value");
        Ok(self.intent.unwrap_or(0.0))
    }
    fn dt(&mut self) -> Result<f32, ()> {
        self.trace.push("delta");
        Ok(self.dt)
    }
    fn z_speed(&mut self) -> Result<f32, ()> {
        self.trace.push("speed");
        Ok(self.speed)
    }
    fn cap_dv_at_top_speed(&mut self) -> Result<bool, ()> {
        self.trace.push("cap cvar");
        Ok(self.cap)
    }
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, ()> {
        self.mapped_inputs.push((offset, input));
        match offset {
            0x230 => {
                self.trace.push("duration graph");
                Ok(self.duration)
            }
            0x1e0 => {
                self.trace.push("dv graph");
                Ok(input * 4.0)
            }
            _ => panic!("unexpected graph"),
        }
    }
    fn current_push_dv(&mut self) -> Result<f32, ()> {
        self.trace.push("current dv");
        Ok(self.current_dv)
    }
    fn set_push_dv(&mut self, dv: f32) -> Result<(), ()> {
        self.trace.push("set dv");
        self.current_dv = dv;
        Ok(())
    }
}
#[test]
fn begin_resets_elapsed_and_samples_fresh_teleport_threshold_before_time() {
    let mut source = Source {
        teleport_time: 0.25,
        ..Default::default()
    };
    let mut instance = FirstPushInstance {
        elapsed: 3.0,
        flags: 0,
    };
    begin(&mut source, &mut instance).unwrap();
    assert_eq!(instance.elapsed.to_bits(), 0);
    assert_eq!(
        instance.flags & (FIRST_UPDATE | ACTIVE | TELEPORT),
        FIRST_UPDATE | ACTIVE | TELEPORT
    );
    assert_eq!(
        source.trace,
        [
            "resolve begin",
            "teleport threshold pointer",
            "teleport time",
            "teleport threshold value"
        ]
    );
    source.teleport_time = source.teleport_limit;
    begin(&mut source, &mut instance).unwrap();
    assert_eq!(instance.flags & TELEPORT, 0);
}

#[test]
fn first_update_uses_intent_then_only_increases_retained_push_dv() {
    let mut source = Source::default();
    let mut instance = FirstPushInstance::after_allocation(true).unwrap();
    begin(&mut source, &mut instance).unwrap();
    source.trace.clear();
    update(&mut source, &mut instance).unwrap();
    assert_eq!(source.current_dv, 2.0);
    assert_eq!(source.mapped_inputs, [(0x230, 2.0), (0x1e0, 0.5)]);
    assert_eq!(instance.flags & FIRST_UPDATE, 0);
    assert_eq!(
        source.trace,
        [
            "resolve update",
            "intent contains",
            "intent value",
            "speed",
            "top speed",
            "cap cvar",
            "duration graph",
            "dv graph",
            "current dv",
            "set dv"
        ]
    );
    source.current_dv = 3.0;
    source.trace.clear();
    update(&mut source, &mut instance).unwrap();
    assert_eq!(source.current_dv, 3.0);
    assert!(!source.trace.contains(&"set dv"));
}

#[test]
fn teleport_uses_previous_elapsed_and_advances_before_speed_lookup() {
    let mut source = Source {
        teleport_time: 0.0,
        intent: None,
        ..Default::default()
    };
    let mut instance = FirstPushInstance::after_allocation(true).unwrap();
    begin(&mut source, &mut instance).unwrap();
    source.trace.clear();
    update(&mut source, &mut instance).unwrap();
    assert_eq!(instance.flags & ACTIVE, 0);
    assert_eq!(instance.elapsed.to_bits(), 0.1_f32.to_bits());
    assert_eq!(source.mapped_inputs[1], (0x1e0, 0.0));
    assert_eq!(
        &source.trace[..4],
        ["resolve update", "intent contains", "delta", "speed"]
    );
    update(&mut source, &mut instance).unwrap();
    assert_eq!(source.mapped_inputs[3].1.to_bits(), 0.2_f32.to_bits());
}

#[test]
fn absent_intent_without_teleport_skips_all_motion_reads_but_clears_flags() {
    let mut source = Source {
        intent: None,
        ..Default::default()
    };
    let mut instance = FirstPushInstance::after_allocation(true).unwrap();
    update(&mut source, &mut instance).unwrap();
    assert_eq!(instance.flags & (ACTIVE | FIRST_UPDATE), 0);
    assert_eq!(source.trace, ["resolve update", "intent contains"]);
}

#[test]
fn top_speed_cap_uses_zero_time_and_negative_speed_clamps_to_zero() {
    let mut source = Source {
        cap: true,
        speed: 5.0,
        ..Default::default()
    };
    let mut instance = FirstPushInstance::after_allocation(true).unwrap();
    update(&mut source, &mut instance).unwrap();
    assert_eq!(source.mapped_inputs, [(0x230, 5.0), (0x1e0, 0.0)]);
    source.speed = -3.0;
    source.mapped_inputs.clear();
    update(&mut source, &mut instance).unwrap();
    assert_eq!(source.mapped_inputs, [(0x230, 0.0), (0x1e0, 0.5)]);
}

#[test]
fn unordered_duration_selects_one_and_unordered_dv_reaches_store() {
    let mut source = Source {
        duration: f32::NAN,
        current_dv: f32::NAN,
        ..Default::default()
    };
    let mut instance = FirstPushInstance::after_allocation(true).unwrap();
    update(&mut source, &mut instance).unwrap();
    assert_eq!(source.mapped_inputs[1], (0x1e0, 1.0));
    assert_eq!(source.current_dv, 4.0);
}
