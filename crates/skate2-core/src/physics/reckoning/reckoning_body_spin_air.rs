//! Actual inAir=true suffix of BodySpin.Update82dcfaf8; shares the history
//! prefix with grounded Update. Global class binding belongs to retained attrs.
use super::{BodySpin, BodySpinCvar, BodySpinCvars};
use crate::reckoning_attributes::ReckoningAttributeRead;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AirCvar {
    InputFade,
    ForceAutoSpeed,
    MaxAutoSpin,
}
pub(crate) trait BodySpinAirCvars: BodySpinCvars {
    fn resolve_air(&mut self, field: AirCvar) -> Result<Self::Pointer, Self::Error>;
}
pub(crate) trait BodySpinAirAttributes: ReckoningAttributeRead {
    fn spin_lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn spin_graph(&mut self, pointer: &Self::Pointer, input: f32) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum BodySpinAirError<A, C> {
    Attribute(A),
    Cvar(C),
}
const DERIVATIVE: u64 = 0x8ace_eb8a_3116_48db;
const STEP: f32 = f32::from_bits(0x3c88_8889);
fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn graph<A: BodySpinAirAttributes>(a: &mut A, key: u64, time: f32) -> Result<f32, A::Error> {
    let p = a.spin_lookup(key)?;
    a.spin_graph(&p, time)
}
fn scalar<A: BodySpinAirAttributes>(a: &mut A, key: u64) -> Result<f32, A::Error> {
    let p = a.spin_lookup(key)?;
    a.float(&p)
}
impl BodySpin {
    /// History82dcb9a0 excludes the current insertion cursor, visits29 entries
    /// backwards, and samples the same captured `NegGraph` pointer at negative time.
    fn try_air_history<A: BodySpinAirAttributes>(&mut self, a: &mut A) -> Result<(), A::Error> {
        self.body_spin_speed_deriv = 0.0;
        self.derivative_time = 0.0;
        let mut index = if self.derivative_history_pos == 0 {
            29
        } else {
            self.derivative_history_pos - 1
        };
        let p = a.spin_lookup(DERIVATIVE)?;
        let mut time = 0.0;
        let mut best = 0.0f32;
        while index != self.derivative_history_pos {
            time -= STEP;
            let value = self.derivative_history[index];
            let candidate = a.spin_graph(&p, time)? * value;
            if candidate.abs() > best.abs() {
                self.derivative_time = time;
                best = candidate;
            }
            index = if index == 0 { 29 } else { index - 1 };
        }
        self.body_spin_speed_deriv = best;
        Ok(())
    }
    pub(crate) fn speed(&self) -> f32 {
        self.body_spin_speed
    }
    pub(crate) fn try_update_air<A: BodySpinAirAttributes, C: BodySpinAirCvars>(
        &mut self,
        input: f32,
        auto_spin: f32,
        a: &mut A,
        c: &mut C,
    ) -> Result<(), BodySpinAirError<A::Error, C::Error>> {
        use BodySpinAirError as E;
        self.try_update_input(input, auto_spin, c)
            .map_err(E::Cvar)?;
        if self.time_in_air == 0.0 {
            self.try_air_history(a).map_err(E::Attribute)?;
        }
        let mix = scalar(a, 0xccc5_fcd8_9fba_be6d).map_err(E::Attribute)?;
        let mut acceleration =
            graph(a, 0x2e75_8b51_c2cb_7b76, self.time_in_air).map_err(E::Attribute)?;
        let auto_acceleration =
            graph(a, 0x76ea_f2a3_15d0_822f, self.time_in_air).map_err(E::Attribute)?;
        let derivative_scale = graph(a, DERIVATIVE, self.time_in_air).map_err(E::Attribute)?;
        let derivative = self.input_derivative_filtered * derivative_scale;
        if derivative.abs() > self.body_spin_speed_deriv.abs() {
            self.body_spin_speed_deriv = derivative;
            self.derivative_time = self.time_in_air;
        }
        let old_smoothed = self.last_input_smoothed;
        self.time_in_air += STEP;
        // VECFLOAT_SMALL initializer82f76340 uses SMALL_FLOAT_82=37800000.
        if f32::from_bits(0x3780_0000) > self.input.abs() {
            let p = c.resolve_air(AirCvar::InputFade).map_err(E::Cvar)?;
            let fade = c.float(&p).map_err(E::Cvar)?;
            let smoothed = fade * old_smoothed;
            self.last_input_smoothed = smoothed;
            self.input = smoothed;
        } else {
            let p = c.resolve(BodySpinCvar::InputBlend).map_err(E::Cvar)?;
            let first = c.float(&p).map_err(E::Cvar)?;
            let intermediate = (1.0 - first) * old_smoothed;
            self.last_input_smoothed = intermediate;
            let p = c.resolve(BodySpinCvar::InputBlend).map_err(E::Cvar)?;
            let second = c.float(&p).map_err(E::Cvar)?;
            self.last_input_smoothed = second.mul_add(input, intermediate);
        }
        let proportional =
            graph(a, 0x2103_b1e0_3535_f897, self.time_in_air).map_err(E::Attribute)?;
        self.body_spin_speed_prop = self.input * proportional;
        //ff20 raw40980008 branches on LT-clear, so unordered preserves derivative.
        if self.body_spin_speed_deriv * self.body_spin_speed_prop < 0.0 {
            self.body_spin_speed_deriv = 0.0;
        }
        let amplitude = self.body_spin_speed_deriv.abs().mul_add(1.0 - mix, mix);
        let mut target = -(amplitude * self.body_spin_speed_prop);
        if auto_spin.abs() > f32::from_bits(0x3780_0000) {
            let use_auto = if self.body_spin_speed * auto_spin > 0.0 {
                true
            } else {
                let p = c.resolve_air(AirCvar::ForceAutoSpeed).map_err(E::Cvar)?;
                self.body_spin_speed.abs() < c.float(&p).map_err(E::Cvar)?
            };
            if use_auto {
                acceleration = auto_acceleration;
                let p = c.resolve_air(AirCvar::MaxAutoSpin).map_err(E::Cvar)?;
                let max = c.float(&p).map_err(E::Cvar)?;
                let lower = select(-max - auto_spin, -max, auto_spin);
                target = select(max - lower, lower, max);
            }
        }
        let braking = scalar(a, 0x88a5_185d_7c2c_d589).map_err(E::Attribute)?;
        let chosen = select(braking - acceleration, acceleration, braking);
        let old_speed = self.body_spin_speed;
        let (lower, upper) = if old_speed > 0.0 {
            (-chosen, acceleration)
        } else {
            (-acceleration, chosen)
        };
        let delta = target - old_speed;
        let lower_bounded = select(lower - delta, lower, delta);
        let bounded = select(upper - lower_bounded, lower_bounded, upper);
        let next = old_speed + bounded;
        self.body_spin_speed = next;
        self.clamping_amount = (target - next) - bounded;
        Ok(())
    }
}

#[cfg(test)]
#[path = "reckoning_body_spin_air_tests.rs"]
mod tests;
