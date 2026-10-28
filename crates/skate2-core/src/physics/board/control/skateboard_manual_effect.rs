//! Unregistered Entry child: complete CalcManualEffect82b32130..32abc.
use super::SkateboardEntryState;
use crate::{
    reckoning_numeric::{ReckoningNumeric, try_normalize_angle, try_signed_angle},
    reckoning_sin_cos::{ReckoningSinCosNumeric, sine::try_sine},
    skateboard_foot_forces::SkateboardFootForce,
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};
const ZERO: Vector = Vector::known([0.0; 4]);
#[derive(Clone, Copy, Debug)]
pub(crate) struct ManualEffect {
    pub(crate) angular_displacement: Vector,
    pub(crate) corrective_force: SkateboardFootForce,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum ManualScalar {
    FlippedControls,
    Balance,
    DeckZyDelta,
    StateTimer,
    AbsoluteSpeed,
    TimeDelta,
}
pub(crate) trait ManualEffectSource {
    type Error;
    type Graph;
    fn scalar(&mut self, field: ManualScalar) -> Result<f32, Self::Error>;
    fn attribute(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn capture_speed_graph(&mut self) -> Result<Self::Graph, Self::Error>;
    fn evaluate_graph(&mut self, graph: &Self::Graph, input: f32) -> Result<f32, Self::Error>;
    fn ground_transform(&mut self) -> Result<Matrix, Self::Error>;
    fn part_six_transform(&mut self) -> Result<Matrix, Self::Error>;
    fn manual_flag(&mut self) -> Result<bool, Self::Error>;
    fn flags_8c8(&mut self) -> Result<u32, Self::Error>;
    fn show_manual(&mut self) -> Result<bool, Self::Error>;
    fn wrong_wheel_scalar(&mut self) -> Result<f32, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum ManualEffectError<S, N, R> {
    Source(S),
    Numeric(N),
    Sine(R),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display
    for ManualEffectError<S, N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "manual source: {e:?}"),
            Self::Numeric(e) => write!(f, "manual numeric: {e:?}"),
            Self::Sine(e) => write!(f, "manual sine: {e:?}"),
        }
    }
}
fn select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}
fn symmetric_clamp(value: f32, maximum: f32) -> f32 {
    let low = select(-maximum - value, -maximum, value);
    select(maximum - low, low, maximum)
}
impl SkateboardEntryState {
    /// Outputs and history writes commit at their original source positions.
    #[expect(
        clippy::type_complexity,
        clippy::too_many_lines,
        clippy::assign_op_pattern,
        reason = "one source-ordered manual history kernel; DT is the first addition operand"
    )]
    pub(crate) fn try_manual_effect<
        S: ManualEffectSource,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        source: &mut S,
        power_sliding: bool,
        numeric: &mut N,
        sine: &mut R,
        output: &mut ManualEffect,
    ) -> Result<(), ManualEffectError<S::Error, N::Error, R::Error>> {
        use ManualEffectError as E;
        // 321bc/c4 initialize corrective vectors, not angular displacement.
        output.corrective_force.world_space_force = ZERO;
        output.corrective_force.local_space_position = ZERO;
        let flipped = source
            .scalar(ManualScalar::FlippedControls)
            .map_err(E::Source)?;
        let balance = source.scalar(ManualScalar::Balance).map_err(E::Source)?;
        let keys = if power_sliding {
            [
                0x7ed1_8259_85b2_679a,
                0xee15_7c8b_8459_aeac,
                0xb1fb_fff9_8936_b84a,
            ]
        } else {
            [
                0xa054_d2d5_4eda_3c5c,
                0x4f0e_182c_fd45_a690,
                0x7228_8fb0_cee6_eda8,
            ]
        };
        let proportional = source.attribute(keys[0]).map_err(E::Source)?;
        let integral = source.attribute(keys[1]).map_err(E::Source)?;
        let derivative = source.attribute(keys[2]).map_err(E::Source)?;
        let ground = source.ground_transform().map_err(E::Source)?;
        let negative_x = ground.columns()[0].map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
        if source.show_manual().map_err(E::Source)? {
            source.require_overlay().map_err(E::Source)?;
        }
        let mut output_scale = 1.0;
        if balance == 0.0 {
            // Reset order differs from ResetManual: f8 before f4 here.
            self.manual_error_integral = 0.0;
            self.manual_last_target_angle = 0.0;
            self.manual_last_actual_angle = 0.0;
            self.manual_time_since_reset = 0.0;
            self.manual_torque = 0.0;
        } else {
            let directed = balance * flipped;
            let sign = if directed > 0.0 { 1.0 } else { -1.0 }; // NOTGT includes NaN
            let scaled = directed.abs().mul_add(0.75, 0.25) * sign;
            let derivative_active = if self.manual_time_since_reset == 0.0 {
                0.0
            } else {
                1.0
            };
            let degrees = source.attribute(0x1bd8_1d97_e88e_2232).map_err(E::Source)? * scaled;
            let radians = degrees * f32::from_bits(0x3c8e_fa35);
            let mut target = try_normalize_angle(radians, numeric).map_err(E::Numeric)?;
            if !source.manual_flag().map_err(E::Source)? {
                let delta_scale = source.attribute(0x89ed_27f8_244d_fa61).map_err(E::Source)?;
                let delta = source
                    .scalar(ManualScalar::DeckZyDelta)
                    .map_err(E::Source)?;
                let deck_term = (delta * delta_scale) * flipped;
                let frequency = source.attribute(0x7c52_f92b_c4a2_58b6).map_err(E::Source)?;
                let timer = source.scalar(ManualScalar::StateTimer).map_err(E::Source)?;
                let phase = (timer * frequency) * f32::from_bits(0x40c9_0fdb);
                let wave = try_sine(phase, sine).map_err(E::Sine)?;
                let amplitude = source.attribute(0x47fa_ac61_4476_f338).map_err(E::Source)?;
                let wave = wave * amplitude;
                let graph = source.capture_speed_graph().map_err(E::Source)?;
                let speed = source
                    .scalar(ManualScalar::AbsoluteSpeed)
                    .map_err(E::Source)?;
                let curve = source.evaluate_graph(&graph, speed).map_err(E::Source)?;
                let adjustment =
                    try_normalize_angle(curve * (wave + deck_term), numeric).map_err(E::Numeric)?;
                target = try_normalize_angle(adjustment + target, numeric).map_err(E::Numeric)?;
            }
            if self.manual_time_since_reset == 0.0 {
                let initial = source.attribute(0x710f_c1fe_f4c3_ca3f).map_err(E::Source)?;
                self.manual_torque =
                    try_normalize_angle(target * initial, numeric).map_err(E::Numeric)?;
            }
            self.manual_last_target_angle = target; //327b4 before Part6 query
            let part = source.part_six_transform().map_err(E::Source)?;
            let angle = try_signed_angle(
                part.columns()[2],
                ground.columns()[2],
                ground.columns()[0],
                numeric,
            )
            .map_err(E::Numeric)?;
            let actual = try_normalize_angle(angle, numeric).map_err(E::Numeric)?;
            let old_actual = self.manual_last_actual_angle;
            self.manual_last_actual_angle = actual;
            let difference = actual - old_actual;
            let error = try_normalize_angle(target - actual, numeric).map_err(E::Numeric)?;
            let maximum = source.attribute(0x0137_2180_f388_67cc).map_err(E::Source)?;
            let old_integral = self.manual_error_integral;
            let derivative_term = (difference * derivative_active) * derivative;
            let bounded = symmetric_clamp(error, maximum);
            let contribution = bounded * f32::from_bits(0x3d4c_cccd);
            let proportional_term = bounded * proportional;
            let filtered = old_integral.mul_add(f32::from_bits(0x3f73_3333), contribution);
            self.manual_error_integral = filtered; //32890 before late attribute
            let integral_term = filtered * integral;
            let maximum = source.attribute(0x966a_83ee_602b_0876).map_err(E::Source)?;
            let flags = source.flags_8c8().map_err(E::Source)?;
            let derivative_term = symmetric_clamp(derivative_term, maximum);
            let wheel_scale = if ((flags >> 1) ^ flags) & 0x0800_0000 != 0 {
                source.wrong_wheel_scalar().map_err(E::Source)?
            } else {
                1.0
            };
            let flags = source.flags_8c8().map_err(E::Source)?; // fresh after live cvar
            let wrong = (flags & 0x1000_0000 == 0 && balance > 0.0)
                || (flags & 0x0800_0000 == 0 && balance < 0.0);
            if wrong {
                let decay = source.attribute(0xe8c9_db10_77d1_d889).map_err(E::Source)?;
                self.manual_torque = (self.manual_torque * decay) * wheel_scale;
                let final_scale = source.attribute(0xc990_6592_1011_63bd).map_err(E::Source)?;
                output_scale = wheel_scale * final_scale;
            } else {
                self.manual_torque =
                    ((self.manual_torque + derivative_term) + integral_term) + proportional_term;
            }
            if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
                eprintln!(
                    "known air: manual effect balance={balance} flipped={flipped} target={target} actual={actual} error={error} p={proportional_term} i={integral_term} d={derivative_term} torque={} wrong={wrong} flags8c8={flags:#010x} scale={output_scale}",
                    self.manual_torque
                );
            }
            if source.show_manual().map_err(E::Source)? {
                source.require_overlay().map_err(E::Source)?;
            }
            let dt = source.scalar(ManualScalar::TimeDelta).map_err(E::Source)?;
            self.manual_time_since_reset = dt + self.manual_time_since_reset;
        }
        let scalar = self.manual_torque * output_scale;
        output.angular_displacement = negative_x.map(|v| v * scalar);
        output.corrective_force.local_space_position = ZERO; //32a98 before32a9c
        output.corrective_force.world_space_force = ZERO;
        Ok(())
    }
}

#[cfg(test)]
#[path = "skateboard_manual_effect_tests.rs"]
mod tests;
