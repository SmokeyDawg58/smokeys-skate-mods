use super::*;

fn same_bits(actual: f32, expected: f32) {
    assert_eq!(actual.to_bits(), expected.to_bits());
}

#[derive(Debug, PartialEq)]
enum Read {
    Resolve(BodySpinCvar),
    Float(usize),
}

struct Cvars {
    values: [f32; 5],
    reads: Vec<Read>,
    resolved: usize,
    fail_at: Option<usize>,
}
impl Cvars {
    fn new(values: [f32; 5]) -> Self {
        Self {
            values,
            reads: Vec::new(),
            resolved: 0,
            fail_at: None,
        }
    }
    fn record(&mut self, read: Read) -> Result<(), usize> {
        self.reads.push(read);
        let index = self.reads.len() - 1;
        if self.fail_at == Some(index) {
            Err(index)
        } else {
            Ok(())
        }
    }
}
impl BodySpinCvars for Cvars {
    type Pointer = usize;
    type Error = usize;
    fn resolve(&mut self, cvar: BodySpinCvar) -> Result<usize, usize> {
        self.record(Read::Resolve(cvar))?;
        let pointer = self.resolved;
        self.resolved += 1;
        Ok(pointer)
    }
    fn float(&mut self, pointer: &usize) -> Result<f32, usize> {
        self.record(Read::Float(*pointer))?;
        Ok(self.values[*pointer])
    }
}

fn scalars(state: &BodySpin) -> [f32; 12] {
    [
        state.input,
        state.input_derivative,
        state.input_derivative_filtered,
        state.last_input_smoothed,
        state.body_spin_target,
        state.body_spin_speed,
        state.body_spin_speed_prop,
        state.body_spin_speed_deriv,
        state.time_in_air,
        state.derivative_time,
        state.clamping_amount,
        state.auto_spin,
    ]
}

#[test]
fn source_constructor_is_positive_zero_without_attribute_reads() {
    let state = BodySpin::new();
    assert_eq!(scalars(&state).map(f32::to_bits), [0; 12]);
    assert_eq!(state.derivative_history.map(f32::to_bits), [0; 30]);
    assert_eq!(state.derivative_history_pos, 0);
}

#[test]
fn grounded_update_keeps_history_and_unwritten_air_state() {
    let mut state = BodySpin::new();
    state.body_spin_target = 17.0;
    state.body_spin_speed_deriv = 18.0;
    state.derivative_time = 19.0;
    state.clamping_amount = 20.0;
    state.body_spin_speed = 21.0;
    state.body_spin_speed_prop = 22.0;
    state.time_in_air = 23.0;
    let mut old_input = 0.0;
    let mut old_derivative = 0.0;
    let mut old_smoothed = 0.0;
    let mut history = [0.0; 30];
    for tick in 0..67 {
        let input = f32::from(u8::try_from(tick % 7).unwrap()) * 0.125;
        let mut cvars = Cvars::new([2.0, 0.25, -100.0, 0.5, 0.25]);
        state.try_update_grounded(input, 0.75, &mut cvars).unwrap();
        let delta = input - old_input;
        let derivative = (delta * 2.0).mul_add(0.25, 0.75 * old_derivative);
        history[tick % 30] = derivative; // Values in this sequence are inside [-1,1].
        old_smoothed = 0.25_f32.mul_add(input, 0.5 * old_smoothed);
        assert_eq!(state.input_derivative.to_bits(), derivative.to_bits());
        assert_eq!(state.last_input_smoothed.to_bits(), old_smoothed.to_bits());
        assert_eq!(
            state.derivative_history.map(f32::to_bits),
            history.map(f32::to_bits)
        );
        assert_eq!(state.derivative_history_pos, (tick + 1) % 30);
        old_input = input;
        old_derivative = derivative;
    }
    assert_eq!(
        [
            state.body_spin_target,
            state.body_spin_speed_deriv,
            state.derivative_time,
            state.clamping_amount
        ]
        .map(f32::to_bits),
        [17.0_f32, 18.0, 19.0, 20.0].map(f32::to_bits)
    );
    assert_eq!(
        [
            state.body_spin_speed,
            state.body_spin_speed_prop,
            state.time_in_air
        ]
        .map(f32::to_bits),
        [0; 3]
    );
    same_bits(state.auto_spin, 0.75);
}

#[test]
fn cvar_targets_and_values_follow_source_order_and_input_blend_is_not_cached() {
    let mut state = BodySpin::new();
    state.last_input_smoothed = 8.0;
    let mut cvars = Cvars::new([1.0, 1.0, -1.0, 0.25, 0.75]);
    state.try_update_grounded(2.0, 0.0, &mut cvars).unwrap();
    same_bits(state.last_input_smoothed, 7.5);
    assert_eq!(
        cvars.reads,
        vec![
            Read::Resolve(BodySpinCvar::DerivativeScalar),
            Read::Resolve(BodySpinCvar::DerivativeBlend),
            Read::Float(0),
            Read::Float(1),
            Read::Resolve(BodySpinCvar::DerivativeThreshold),
            Read::Float(2),
            Read::Resolve(BodySpinCvar::InputBlend),
            Read::Float(3),
            Read::Resolve(BodySpinCvar::InputBlend),
            Read::Float(4),
        ]
    );
    same_bits(state.derivative_history[0], 1.0);
}

#[test]
fn threshold_is_strict_and_unordered_selects_positive_zero_before_clamping() {
    for (input, scalar, threshold, expected) in [
        (2.0, 1.0, 4.0, 0.0_f32), // Equal product fails, not >=.
        (2.0, 1.0, 3.0, 1.0),
        (-2.0, 1.0, 3.0, -1.0),
        (f32::NAN, 1.0, -1.0, 0.0),
        (2.0, 1.0, f32::NAN, 0.0),
        (1.0, f32::INFINITY, 0.0, 1.0),
        (-1.0, f32::INFINITY, 0.0, -1.0),
    ] {
        let mut state = BodySpin::new();
        let mut cvars = Cvars::new([scalar, 1.0, threshold, 0.0, 0.0]);
        state.try_update_grounded(input, 0.0, &mut cvars).unwrap();
        assert_eq!(
            state.input_derivative_filtered.to_bits(),
            expected.to_bits()
        );
        assert_eq!(state.derivative_history[0].to_bits(), expected.to_bits());
    }
}

#[test]
fn derivative_and_smoothing_keep_source_fused_rounding_residual() {
    let one_plus = f32::from_bits(0x3f80_0001);
    let one_plus_twice = f32::from_bits(0x3f80_0002);
    let mut state = BodySpin::new();
    state.input_derivative = 8_388_610.0;
    state.last_input_smoothed = -one_plus_twice;
    let mut cvars = Cvars::new([1.0, one_plus, -1.0, 0.0, one_plus]);
    state
        .try_update_grounded(one_plus, 0.0, &mut cvars)
        .unwrap();
    assert_eq!(state.input_derivative.to_bits(), 0x2880_0000); // 2^-46.
    assert_eq!(state.last_input_smoothed.to_bits(), 0x2880_0000);
    let rounded_product = one_plus * one_plus;
    assert_eq!((rounded_product - one_plus_twice).to_bits(), 0);
}

#[test]
fn fallible_cvar_services_preserve_every_completed_source_prefix() {
    for failure in 0..10 {
        let mut state = BodySpin::new();
        state.input = 0.5;
        state.input_derivative = 0.25;
        state.input_derivative_filtered = -0.5;
        state.last_input_smoothed = 8.0;
        state.body_spin_speed = 3.0;
        state.body_spin_speed_prop = 4.0;
        state.time_in_air = 5.0;
        let mut cvars = Cvars::new([2.0, 0.5, -10.0, 0.25, 0.75]);
        cvars.fail_at = Some(failure);
        assert_eq!(
            state.try_update_grounded(1.0, 0.75, &mut cvars),
            Err(failure)
        );
        same_bits(state.auto_spin, 0.75);
        assert_eq!(cvars.reads.len(), failure + 1);
        if failure < 4 {
            same_bits(state.input, 0.5);
            same_bits(state.input_derivative, 0.25);
        } else {
            same_bits(state.input, 1.0);
            same_bits(state.input_derivative, 0.625);
        }
        if failure < 6 {
            same_bits(state.input_derivative_filtered, -0.5);
            assert_eq!(state.derivative_history_pos, 0);
            assert_eq!(state.derivative_history.map(f32::to_bits), [0; 30]);
            same_bits(state.time_in_air, 5.0);
            same_bits(state.body_spin_speed, 3.0);
            same_bits(state.body_spin_speed_prop, 4.0);
        } else {
            same_bits(state.input_derivative_filtered, 0.625);
            assert_eq!(state.derivative_history_pos, 1);
            same_bits(state.derivative_history[0], 0.625);
            assert_eq!(state.time_in_air.to_bits(), 0);
            assert_eq!(state.body_spin_speed.to_bits(), 0);
            assert_eq!(state.body_spin_speed_prop.to_bits(), 0);
        }
        same_bits(
            state.last_input_smoothed,
            if failure < 8 { 8.0 } else { 6.0 },
        );
    }
}
