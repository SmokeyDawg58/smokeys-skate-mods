use super::*;
struct Mock {
    calls: Vec<&'static str>,
    scalar_index: usize,
    scalars: [f32; 8],
    lean: f32,
    twist: f32,
    mirrored: bool,
    gesture: [Option<f32>; 2],
    writes: Vec<(FastString30, f32)>,
    published: Option<[f32; 2]>,
    controlled: bool,
    fail: Option<&'static str>,
}
impl Mock {
    fn new() -> Self {
        Self {
            calls: Vec::new(),
            scalar_index: 0,
            scalars: [1.0, 1.0, 10.0, 10.0, 10.0, 1.0, 1.0, 0.5],
            lean: 0.0,
            twist: 0.0,
            mirrored: false,
            gesture: [None, None],
            writes: Vec::new(),
            published: None,
            controlled: false,
            fail: None,
        }
    }
    fn call(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl Source for Mock {
    type Error = &'static str;
    fn missing_instance(&mut self) -> Self::Error {
        "instance"
    }
    fn resolve(&mut self, _: bool) -> Result<(), Self::Error> {
        self.call("resolve")
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        let keys = [
            0xe7cc_6d49_4e1e_cc67,
            0x5b35_2375_1e22_f6f0,
            0x1e1a_632b_229d_ae28,
            0x2c35_05e3_7b8a_f858,
            0x6a57_e32d_b7a5_261b,
            0x4385_8810_d1cd_e1d7,
            0x303b_7fcd_b850_0dcd,
            0x2f48_759c_d4b3_6916,
        ];
        assert_eq!(key, keys[self.scalar_index]);
        self.call("scalar")?;
        let value = self.scalars[self.scalar_index];
        self.scalar_index += 1;
        Ok(value)
    }
    fn lean(&mut self) -> Result<f32, Self::Error> {
        self.call("lean")?;
        Ok(self.lean)
    }
    fn twist(&mut self) -> Result<f32, Self::Error> {
        self.call("twist")?;
        Ok(self.twist)
    }
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        self.call("mirror")?;
        Ok(self.mirrored)
    }
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error> {
        self.call("set")?;
        self.writes.push((name, value));
        Ok(())
    }
    fn capture_intents(&mut self) -> Result<(), Self::Error> {
        self.call("intents")
    }
    fn gesture(&mut self, x: bool) -> Result<Option<f32>, Self::Error> {
        self.call(if x { "x" } else { "y" })?;
        Ok(self.gesture[usize::from(!x)])
    }
    fn controlled(&mut self) -> Result<(), Self::Error> {
        self.call("controlled")?;
        self.controlled = true;
        Ok(())
    }
    fn publish_gesture(&mut self, value: [f32; 2]) -> Result<(), Self::Error> {
        self.call("publish")?;
        self.published = Some(value);
        Ok(())
    }
}
fn names() -> Names {
    Names {
        lean: FastString30::encode("lean").unwrap(),
        twist: FastString30::encode("twist").unwrap(),
        gesture_x: FastString30::encode("x").unwrap(),
        gesture_y: FastString30::encode("y").unwrap(),
    }
}
#[test]
fn begin_reset_precedes_failed_resolution_and_preserves_unused_delta_lean() {
    let mut m = Mock::new();
    let mut d = WipeoutInstance::after_allocation(true).unwrap();
    d.updates = 9;
    d.lean = 7.0;
    d.delta_lean = 3.0;
    m.fail = Some("resolve");
    assert_eq!(begin(&mut m, Some(&mut d)), Err("resolve"));
    assert_eq!(d.updates, 0);
    assert_eq!(d.lean.to_bits(), 7.0_f32.to_bits());
    m.fail = None;
    m.twist = 1.0;
    m.mirrored = true;
    begin(&mut m, Some(&mut d)).unwrap();
    assert_eq!(d.twist.to_bits(), (TWO_PI - 1.0).to_bits());
    assert_eq!(d.delta_lean.to_bits(), 3.0_f32.to_bits());
}
#[test]
fn failed_lean_output_keeps_instance_prefix_before_twist_or_gestures() {
    let mut m = Mock::new();
    let mut d = WipeoutInstance::after_allocation(true).unwrap();
    m.lean = 2.0;
    m.fail = Some("set");
    assert_eq!(update(&mut m, Some(&mut d), names()), Err("set"));
    assert_eq!(d.lean.to_bits(), 2.0_f32.to_bits());
    assert_eq!(d.updates, 0);
    assert_eq!(m.scalar_index, 5);
    assert!(!m.calls.contains(&"intents"));
}
#[test]
fn absent_intents_publish_zero_and_nan_threshold_skips_controlled() {
    let mut m = Mock::new();
    let mut d = WipeoutInstance::after_allocation(true).unwrap();
    m.scalars[7] = f32::NAN;
    update(&mut m, Some(&mut d), names()).unwrap();
    assert!(!m.controlled);
    assert_eq!(m.published.unwrap().map(f32::to_bits), [0, 0]);
    assert_eq!(d.updates, 1);
    assert_eq!(m.writes.len(), 4);
}
#[test]
fn controlled_failure_keeps_four_outputs_and_withholds_final_payload_and_count() {
    let mut m = Mock::new();
    let mut d = WipeoutInstance::after_allocation(true).unwrap();
    m.gesture = [Some(1.0), None];
    m.fail = Some("controlled");
    assert_eq!(update(&mut m, Some(&mut d), names()), Err("controlled"));
    assert_eq!(m.writes.len(), 4);
    assert!(m.published.is_none());
    assert_eq!(d.updates, 0);
    assert_eq!(d.gesture[0].to_bits(), 1.0_f32.to_bits());
}
#[test]
fn second_derivative_limit_starts_at_unsigned_update_two() {
    for (updates, expected) in [(1, 1.0_f32), (2, 0.25_f32)] {
        let mut m = Mock::new();
        let mut d = WipeoutInstance::after_allocation(true).unwrap();
        m.twist = 1.0;
        m.scalars[4] = 0.25;
        d.updates = updates;
        update(&mut m, Some(&mut d), names()).unwrap();
        assert_eq!(d.delta_twist.to_bits(), expected.to_bits());
    }
}
