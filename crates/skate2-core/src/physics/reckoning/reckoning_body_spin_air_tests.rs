#![expect(
    clippy::float_cmp,
    reason = "exact source state and partial write fixtures"
)]
use super::{
    AirCvar, BodySpin, BodySpinAirAttributes, BodySpinAirCvars, BodySpinAirError, BodySpinCvar,
    BodySpinCvars, DERIVATIVE, STEP,
};
use crate::reckoning_attributes::{ReckoningAttributeRead, ReckoningAttributeScope};
#[derive(Default)]
struct Attributes {
    calls: Vec<(u64, f32)>,
    fail: Option<u64>,
    fail_graph: Option<usize>,
}
impl ReckoningAttributeRead for Attributes {
    type Layout = ();
    type Pointer = u64;
    type Error = &'static str;
    fn capture_layout(&mut self, _: ReckoningAttributeScope) -> Result<(), Self::Error> {
        panic!("spin uses pointers")
    }
    fn layout_float(&mut self, (): &(), _: usize) -> Result<f32, Self::Error> {
        panic!("spin uses pointers")
    }
    fn evaluate_graph8(&mut self, (): &(), _: usize, _: f32) -> Result<f32, Self::Error> {
        panic!("spin uses pointers")
    }
    fn lookup(&mut self, _: ReckoningAttributeScope, _: u64) -> Result<u64, Self::Error> {
        panic!("spin global binding")
    }
    fn float(&mut self, p: &u64) -> Result<f32, Self::Error> {
        Ok(if *p == 0xccc5_fcd8_9fba_be6d {
            0.25
        } else {
            0.05
        })
    }
    fn signed_word(&mut self, _: &u64) -> Result<i32, Self::Error> {
        panic!("no word")
    }
}
impl BodySpinAirAttributes for Attributes {
    fn spin_lookup(&mut self, key: u64) -> Result<u64, Self::Error> {
        if self.fail == Some(key) {
            return Err("selected lookup");
        }
        Ok(key)
    }
    fn spin_graph(&mut self, key: &u64, time: f32) -> Result<f32, Self::Error> {
        if self.fail_graph == Some(self.calls.len()) {
            return Err("graph");
        }
        self.calls.push((*key, time));
        Ok(match *key {
            DERIVATIVE => 1.0,
            0x2e75_8b51_c2cb_7b76 => 0.1,
            0x76ea_f2a3_15d0_822f => 0.2,
            0x2103_b1e0_3535_f897 => 2.0,
            _ => panic!("unexpected graph"),
        })
    }
}
struct Cvars {
    blends: usize,
    fail_second_blend: bool,
    force: f32,
    max_reads: usize,
}
impl Cvars {
    fn new() -> Self {
        Self {
            blends: 0,
            fail_second_blend: false,
            force: 0.02,
            max_reads: 0,
        }
    }
}
impl BodySpinCvars for Cvars {
    type Pointer = f32;
    type Error = &'static str;
    fn resolve(&mut self, field: BodySpinCvar) -> Result<f32, Self::Error> {
        if field == BodySpinCvar::InputBlend {
            self.blends += 1;
            if self.blends == 2 && self.fail_second_blend {
                return Err("second blend");
            }
        }
        Ok(match field {
            BodySpinCvar::DerivativeScalar | BodySpinCvar::DerivativeBlend => 1.0,
            BodySpinCvar::DerivativeThreshold => 0.0,
            BodySpinCvar::InputBlend => 0.2,
        })
    }
    fn float(&mut self, p: &f32) -> Result<f32, Self::Error> {
        Ok(*p)
    }
}
impl BodySpinAirCvars for Cvars {
    fn resolve_air(&mut self, field: AirCvar) -> Result<f32, Self::Error> {
        Ok(match field {
            AirCvar::InputFade => 0.96,
            AirCvar::ForceAutoSpeed => self.force,
            AirCvar::MaxAutoSpin => {
                self.max_reads += 1;
                2.0
            }
        })
    }
}
#[test]
fn first_air_update_scans_twenty_nine_negative_history_times_then_current_graphs() {
    let mut state = BodySpin::new();
    let mut a = Attributes {
        calls: vec![],
        fail: None,
        fail_graph: None,
    };
    state
        .try_update_air(1.0, 0.0, &mut a, &mut Cvars::new())
        .unwrap();
    assert_eq!(a.calls.len(), 33);
    assert!(
        a.calls[..29]
            .iter()
            .all(|(key, time)| *key == DERIVATIVE && *time < 0.0)
    );
    assert_eq!(a.calls[0].1.to_bits(), (-STEP).to_bits());
    assert_eq!(state.derivative_time.to_bits(), (-STEP).to_bits());
    assert_eq!(a.calls[29].1.to_bits(), 0);
    assert_eq!(a.calls[32].1.to_bits(), STEP.to_bits());
    assert_eq!(state.body_spin_speed.to_bits(), (-0.1f32).to_bits());
    assert_eq!(state.last_input_smoothed, 0.2);
    a.calls.clear();
    state
        .try_update_air(0.0, 0.0, &mut a, &mut Cvars::new())
        .unwrap();
    assert_eq!(a.calls.len(), 4); //No new takeoff-history scan.
    assert_eq!(state.input.to_bits(), (0.96f32 * 0.2).to_bits());
}
#[test]
fn late_air_attribute_failure_preserves_prefix_and_history_without_advancing_time() {
    let mut state = BodySpin::new();
    let mut a = Attributes {
        calls: vec![],
        fail: Some(0xccc5_fcd8_9fba_be6d),
        fail_graph: None,
    };
    assert!(matches!(
        state.try_update_air(1.0, 0.0, &mut a, &mut Cvars::new()),
        Err(BodySpinAirError::Attribute("selected lookup"))
    ));
    assert_eq!(state.derivative_history_pos, 1);
    assert_eq!(state.body_spin_speed_deriv, 1.0);
    assert_eq!(state.derivative_time.to_bits(), (-STEP).to_bits());
    assert_eq!(state.time_in_air.to_bits(), 0);
    assert_eq!(state.body_spin_speed.to_bits(), 0);
}

#[test]
fn partial_history_failure_retains_chosen_time_but_not_final_derivative() {
    let mut state = BodySpin::new();
    let mut a = Attributes {
        fail_graph: Some(1),
        ..Attributes::default()
    };
    assert!(matches!(
        state.try_update_air(1.0, 0.0, &mut a, &mut Cvars::new()),
        Err(BodySpinAirError::Attribute("graph"))
    ));
    assert_eq!(state.derivative_time.to_bits(), (-STEP).to_bits());
    assert_eq!(state.body_spin_speed_deriv.to_bits(), 0);
    assert_eq!(state.time_in_air.to_bits(), 0);
}
#[test]
fn second_blend_failure_preserves_intermediate_and_elapsed_air_time() {
    let mut state = BodySpin::new();
    state.last_input_smoothed = 0.5;
    let mut a = Attributes::default();
    let mut c = Cvars::new();
    c.fail_second_blend = true;
    assert!(matches!(
        state.try_update_air(1.0, 0.0, &mut a, &mut c),
        Err(BodySpinAirError::Cvar("second blend"))
    ));
    assert_eq!(state.last_input_smoothed.to_bits(), 0.4f32.to_bits());
    assert_eq!(state.time_in_air.to_bits(), STEP.to_bits());
    assert_eq!(state.body_spin_speed.to_bits(), 0);
}
#[test]
fn unordered_force_auto_gate_does_not_read_maximum_and_clamping_keeps_both_subtractions() {
    let mut state = BodySpin::new();
    let mut a = Attributes::default();
    let mut c = Cvars::new();
    c.force = f32::NAN;
    state.try_update_air(1.0, 1.0, &mut a, &mut c).unwrap();
    assert_eq!(c.max_reads, 0);
    assert_eq!(state.body_spin_speed.to_bits(), (-0.1f32).to_bits());
    assert_eq!(state.clamping_amount.to_bits(), 0xbfe6_6666);
}
