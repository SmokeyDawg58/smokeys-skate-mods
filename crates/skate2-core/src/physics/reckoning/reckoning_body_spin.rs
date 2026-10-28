//! Retained `BodySpin` constructor (0x82dcb940) and grounded Update path
//! (0x82dcfaf8..0x82dcfc04 -> 0x82dd0058..0x82dd0094).
//!
//! This is the actual history-producing ground branch, not an air-spin model.
//! `FixedHeading` (0x82ddd4dc) supplies current `ProcessedPhysIn.mPhysBodySpin`,
//! auto-spin +0 and in-air false. The complete Reckoning caller is still pending.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained grounded BodySpin awaits the complete Reckoning caller"
    )
)]

/// Current cvar references; native values are target+0xd8, targets ref+0x28.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BodySpinCvar {
    /// 0x830db76c.
    DerivativeScalar,
    /// 0x830db3f8.
    DerivativeBlend,
    /// 0x830db984.
    DerivativeThreshold,
    /// 0x830db374; resolved twice in the ground suffix.
    InputBlend,
}

/// Keep reference resolution separate from the subsequent scalar load. In
/// particular, both derivative targets are resolved before either value loads.
/// Host failures preserve the source writes already completed; they are not
/// native exception behavior. No state lock is held across these services.
pub(crate) trait BodySpinCvars {
    type Pointer;
    type Error;
    fn resolve(&mut self, cvar: BodySpinCvar) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
}

/// PDB `BodySpin` size0xac. Padding and allocation failure are not represented
/// as fabricated initialized native bytes. This owner is intentionally not Clone.
#[expect(
    clippy::struct_field_names,
    reason = "retain the recovered PDB state names across ground and future air branches"
)]
pub(crate) struct BodySpin {
    derivative_history: [f32; 30],
    input: f32,
    input_derivative: f32,
    input_derivative_filtered: f32,
    last_input_smoothed: f32,
    body_spin_target: f32,
    body_spin_speed: f32,
    body_spin_speed_prop: f32,
    body_spin_speed_deriv: f32,
    time_in_air: f32,
    derivative_time: f32,
    clamping_amount: f32,
    auto_spin: f32,
    derivative_history_pos: usize,
}

impl BodySpin {
    /// Read-only test oracle over all source state, not an address/identity proxy.
    #[cfg(test)]
    pub(crate) fn state_words(&self) -> ([u32; 30], [u32; 12], usize) {
        (
            self.derivative_history.map(f32::to_bits),
            [
                self.input,
                self.input_derivative,
                self.input_derivative_filtered,
                self.last_input_smoothed,
                self.body_spin_target,
                self.body_spin_speed,
                self.body_spin_speed_prop,
                self.body_spin_speed_deriv,
                self.time_in_air,
                self.derivative_time,
                self.clamping_amount,
                self.auto_spin,
            ]
            .map(f32::to_bits),
            self.derivative_history_pos,
        )
    }

    /// `BodySpin::BodySpin`, 0x82dcb940..0x82dcb99c: all 30 history entries,
    /// twelve scalars and the history cursor start at positive zero.
    pub(crate) fn new() -> Self {
        Self {
            derivative_history: [0.0; 30],
            input: 0.0,
            input_derivative: 0.0,
            input_derivative_filtered: 0.0,
            last_input_smoothed: 0.0,
            body_spin_target: 0.0,
            body_spin_speed: 0.0,
            body_spin_speed_prop: 0.0,
            body_spin_speed_deriv: 0.0,
            time_in_air: 0.0,
            derivative_time: 0.0,
            clamping_amount: 0.0,
            auto_spin: 0.0,
            derivative_history_pos: 0,
        }
    }

    /// Shared source prefix82dcfb2c..fc00. Both branches publish the same history.
    fn try_update_input<C: BodySpinCvars>(
        &mut self,
        input: f32,
        auto_spin: f32,
        cvars: &mut C,
    ) -> Result<(), C::Error> {
        self.auto_spin = auto_spin; // 0x82dcfb2c, before any cvar resolution.
        let scalar_pointer = cvars.resolve(BodySpinCvar::DerivativeScalar)?;
        let old_input = self.input;
        let old_derivative = self.input_derivative;
        let blend_pointer = cvars.resolve(BodySpinCvar::DerivativeBlend)?;
        let delta = input - old_input;
        let scalar = cvars.float(&scalar_pointer)?;
        let blend = cvars.float(&blend_pointer)?;
        let old_contribution = (1.0 - blend) * old_derivative;
        self.input = input;
        let scaled_delta = delta * scalar;
        let derivative = scaled_delta.mul_add(blend, old_contribution);
        self.input_derivative = derivative;

        let threshold_pointer = cvars.resolve(BodySpinCvar::DerivativeThreshold)?;
        let product = derivative * input;
        let threshold = cvars.float(&threshold_pointer)?;
        // Raw bgt at 0x82dcfb8c: equality and unordered both select positive zero.
        self.input_derivative_filtered = if product > threshold { derivative } else { 0.0 };
        let filtered = self.input_derivative_filtered;
        // Native fsel, including its unordered and signed-zero choices. Do not
        // replace these two operations with clamp/min/max.
        let lower = if -1.0 - filtered >= 0.0 {
            -1.0
        } else {
            filtered
        };
        let bounded = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
        self.input_derivative_filtered = bounded;
        self.derivative_history[self.derivative_history_pos] = bounded;
        // The private constructor/update invariant is 0..30. This matches native
        // signed remainder for every reachable index, not corrupt-index repair.
        self.derivative_history_pos = (self.derivative_history_pos + 1) % 30;

        Ok(())
    }

    /// Grounded branch of `BodySpin::Update`, 0x82dcfaf8. No dt, Vault read,
    /// air graph or derivative-history reset belongs to this branch.
    pub(crate) fn try_update_grounded<C: BodySpinCvars>(
        &mut self,
        input: f32,
        auto_spin: f32,
        cvars: &mut C,
    ) -> Result<(), C::Error> {
        self.try_update_input(input, auto_spin, cvars)?;
        self.time_in_air = 0.0;
        self.body_spin_speed = 0.0;
        self.body_spin_speed_prop = 0.0;
        let old_smoothed = self.last_input_smoothed;
        let first_input_pointer = cvars.resolve(BodySpinCvar::InputBlend)?;
        let first_blend = cvars.float(&first_input_pointer)?;
        let intermediate = (1.0 - first_blend) * old_smoothed;
        self.last_input_smoothed = intermediate; // 0x82dd0080 survives late failure.
        let second_input_pointer = cvars.resolve(BodySpinCvar::InputBlend)?;
        let second_blend = cvars.float(&second_input_pointer)?;
        self.last_input_smoothed = second_blend.mul_add(input, intermediate);
        Ok(())
    }
}

#[cfg(feature = "skate3-runtime")]
#[path = "reckoning_body_spin_skate3.rs"]
mod skate3_assists;

#[cfg(test)]
#[path = "reckoning_body_spin_tests.rs"]
mod tests;

#[path = "reckoning_body_spin_air.rs"]
pub(crate) mod air;
