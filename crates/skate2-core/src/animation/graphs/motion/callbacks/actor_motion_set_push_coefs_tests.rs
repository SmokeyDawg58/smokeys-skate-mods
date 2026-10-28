use super::{SetPushCoefsInstance, SetPushCoefsSource, clamp_unit, ramp, update};

struct Source {
    trace: Vec<String>,
    push: [f32; 8],
    speed: f32,
    dt: f32,
    first_duration: f32,
    third_duration: f32,
    fail_at: Option<&'static str>,
    attributes: Vec<(String, f32)>,
}
impl Default for Source {
    fn default() -> Self {
        let mut push = [0.0; 8];
        push[2] = 0.2;
        push[3] = 0.8;
        push[4] = 0.4;
        push[5] = 0.8;
        push[6] = 0.2;
        push[7] = 0.6;
        Self {
            trace: Vec::new(),
            push,
            speed: 3.0,
            dt: 0.2,
            first_duration: 60.0,
            third_duration: 120.0,
            fail_at: None,
            attributes: Vec::new(),
        }
    }
}
impl Source {
    fn event(&mut self, value: String) -> Result<(), &'static str> {
        let fail = self.fail_at == Some(value.as_str());
        self.trace.push(value);
        if fail { Err("injected") } else { Ok(()) }
    }
}
impl SetPushCoefsSource for Source {
    type Error = &'static str;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
        self.event("resolve".into())
    }
    fn push(&mut self, index: usize) -> Result<f32, Self::Error> {
        self.event(format!("read {index}"))?;
        Ok(self.push[index])
    }
    fn speed(&mut self) -> Result<f32, Self::Error> {
        self.event("speed".into())?;
        Ok(self.speed)
    }
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error> {
        self.event(format!("graph {offset:03x} {input:?}"))?;
        Ok(match offset {
            0x280 => self.first_duration,
            0x2d0 => self.third_duration,
            _ => panic!("unexpected Pushing graph offset"),
        })
    }
    fn dt(&mut self) -> Result<f32, Self::Error> {
        self.event("delta".into())?;
        Ok(self.dt)
    }
    fn set_push(&mut self, index: usize, value: f32) -> Result<(), Self::Error> {
        self.event(format!("write {index}"))?;
        self.push[index] = value;
        Ok(())
    }
    fn animation_attribute(&mut self, name: &str, value: f32) -> Result<(), Self::Error> {
        self.event(format!("attribute {name}"))?;
        self.attributes.push((name.into(), value));
        Ok(())
    }
}

#[test]
fn ramp_reads_source_order_and_publishes_all_state_before_attributes() {
    let mut source = Source::default();
    ramp(&mut source).unwrap();
    assert!((source.push[2] - 0.4).abs() < 1.0e-6);
    assert!((source.push[3] - 0.6).abs() < 1.0e-6);
    assert!((source.push[4] - 0.5).abs() < 1.0e-6);
    assert_eq!(
        source.attributes,
        [
            ("HStr_Vel_B".into(), source.push[2]),
            ("LStr_Vel_B".into(), source.push[3]),
            ("Vel_E".into(), source.push[4])
        ]
    );
    assert_eq!(
        &source.trace[..8],
        [
            "resolve", "read 4", "read 5", "read 6", "read 7", "read 2", "read 3", "speed"
        ]
    );
    assert_eq!(
        &source.trace[8..11],
        ["graph 280 0.4", "graph 2d0 3.0", "delta"]
    );
    assert_eq!(
        &source.trace[11..],
        [
            "write 2",
            "write 3",
            "write 4",
            "attribute HStr_Vel_B",
            "attribute LStr_Vel_B",
            "attribute Vel_E"
        ]
    );
}

#[test]
fn negative_third_source_uses_target_triplet_and_zero_clamps() {
    let mut source = Source::default();
    source.push[2] = -1.0;
    source.push[3] = -1.0;
    source.push[4] = -1.0;
    source.push[5] = 0.3;
    source.push[6] = 0.5;
    source.push[7] = 0.7;
    source.speed = -3.0;
    ramp(&mut source).unwrap();
    assert_eq!(&source.trace[8..10], ["graph 280 0.7", "graph 2d0 0.0"]);
    for (actual, expected) in source.push[2..5].iter().zip([0.3_f32, 0.5, 0.7]) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

#[test]
fn unordered_third_source_keeps_current_triplet_before_native_clamp() {
    let mut source = Source::default();
    source.push[4] = f32::NAN;
    ramp(&mut source).unwrap();
    assert_eq!(source.trace[8], "graph 280 NaN");
    assert!((source.push[2] - 0.4).abs() < 1.0e-6);
    assert!((source.push[3] - 0.6).abs() < 1.0e-6);
    assert_eq!(source.push[4].to_bits(), 1.0_f32.to_bits());
}

#[test]
fn first_only_gate_clears_flag_after_first_success() {
    let mut source = Source::default();
    let mut instance = SetPushCoefsInstance::after_allocation(true).unwrap();
    update(&mut source, &mut instance, true).unwrap();
    assert!(!instance.first_update);
    source.trace.clear();
    update(&mut source, &mut instance, true).unwrap();
    assert!(source.trace.is_empty());
    update(&mut source, &mut instance, false).unwrap();
    assert!(!source.trace.is_empty());
    source.trace.clear();
    instance.begin();
    assert!(instance.first_update);
    update(&mut source, &mut instance, true).unwrap();
    assert!(!source.trace.is_empty());
}

#[test]
fn failed_ramp_keeps_instance_first_byte_and_prior_state_writes() {
    let mut source = Source {
        fail_at: Some("attribute LStr_Vel_B"),
        ..Default::default()
    };
    let mut instance = SetPushCoefsInstance::after_allocation(true).unwrap();
    assert_eq!(update(&mut source, &mut instance, true), Err("injected"));
    assert!(instance.first_update);
    assert_eq!(source.attributes.len(), 1);
    for (actual, expected) in source.push[2..5].iter().zip([0.4, 0.6, 0.5]) {
        assert!((actual - expected).abs() < 1.0e-6);
    }
}

#[test]
fn native_fsel_clamp_maps_unordered_to_one() {
    assert_eq!(clamp_unit(-0.5).to_bits(), 0);
    assert_eq!(clamp_unit(1.5).to_bits(), 1.0_f32.to_bits());
    assert_eq!(clamp_unit(f32::NAN).to_bits(), 1.0_f32.to_bits());
}
