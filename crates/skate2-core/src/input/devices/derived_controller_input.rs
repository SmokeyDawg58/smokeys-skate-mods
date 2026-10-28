//! Retained `DerivedControllerInput::Update` (`0x825f1c30`) and inline ctor.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    controller_input::{
        ControllerControl, ControllerInputActivity, RawControllerInput, TryMappedControllerInput,
    },
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
};
use std::sync::Arc;

#[cfg(test)]
#[path = "derived_controller_input_tests.rs"]
mod tests;

pub(crate) const INPUT_LISTENER_CLASS: u64 = 0x5149_ae9f_1619_0257;
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
const MAGNITUDE_THRESHOLD: u64 = 0x1f3c_1c1c_68af_4c23;

/// Selected actual global member `inputlistener_default` (`0x828b3510`).
/// This retains the original Vault instance; each Update fetches the scalar.
pub(crate) struct DerivedInputAttributes {
    instance: AttributeRuntimeInstance,
}
impl DerivedInputAttributes {
    pub(crate) fn new(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(INPUT_LISTENER_CLASS, DEFAULT_COLLECTION),
        }
    }
    fn threshold(&self) -> Result<f32, AttributeRuntimeError> {
        // `0x825f2084..2098`: missing pointer uses four-byte DefaultDataArea.
        self.instance
            .try_scalar_word(MAGNITUDE_THRESHOLD, 0)
            .map(|word| f32::from_bits(word.unwrap_or(0)))
    }
}
#[derive(Debug)]
pub(crate) enum DerivedInputError<I, N> {
    Input(I),
    Numeric(N),
    MissingGlobalAttributes,
    Attribute(AttributeRuntimeError),
}

/// Native size0x68 at listener+8; raw states0/+1c, timers+38..64.
/// Relevant raw fields are known; the unused low20 packed bits are not modeled.
pub(crate) struct DerivedControllerInput {
    pub(crate) previous: RawControllerInput,
    pub(crate) current: RawControllerInput,
    pub(crate) trigger_held: [f32; 2],
    /// Left rise/fall, right rise/fall.
    pub(crate) stick_edge_age: [f32; 4],
    /// A, B, X, Y, in source update/read order.
    pub(crate) face_held: [f32; 4],
    pub(crate) magnitude_held: [f32; 2],
}
impl DerivedControllerInput {
    /// Inlined ctor `0x825f9f64..9ff8`, before listener registrations.
    pub(crate) fn new() -> Self {
        Self {
            previous: RawControllerInput::default(),
            current: RawControllerInput::default(),
            trigger_held: [0.0; 2],
            stick_edge_age: [f32::from_bits(0x7eff_ffff); 4],
            face_held: [0.0; 4],
            magnitude_held: [0.0; 2],
        }
    }
    /// Source update preserves every completed raw/timer write on checked error.
    /// A missing global owner is checked only after both numeric estimates.
    #[expect(
        clippy::float_cmp,
        reason = "source trigger held predicate compares exactly with one"
    )]
    pub(crate) fn try_update<I: TryMappedControllerInput, N: ReckoningNumeric>(
        &mut self,
        input: &mut I,
        dt: f32,
        activity: ControllerInputActivity,
        attributes: Option<&DerivedInputAttributes>,
        numeric: &mut N,
    ) -> Result<(), DerivedInputError<I::Error, N::Error>> {
        self.previous = self.current;
        self.current
            .try_update(input, &self.previous, false, activity)
            .map_err(DerivedInputError::Input)?;
        self.trigger_held[0] =
            held_time(self.current.left_trigger == 1.0, self.trigger_held[0], dt);
        self.trigger_held[1] =
            held_time(self.current.right_trigger == 1.0, self.trigger_held[1], dt);
        let edges = [
            !self.previous.left_stick_in && self.current.left_stick_in,
            self.previous.left_stick_in && !self.current.left_stick_in,
            !self.previous.right_stick_in && self.current.right_stick_in,
            self.previous.right_stick_in && !self.current.right_stick_in,
        ];
        for (age, edge) in self.stick_edge_age.iter_mut().zip(edges) {
            *age = if edge { 0.0 } else { *age + dt };
        }
        for (timer, control) in self.face_held.iter_mut().zip([
            ControllerControl::FaceA,
            ControllerControl::FaceB,
            ControllerControl::FaceX,
            ControllerControl::FaceY,
        ]) {
            let down = input
                .try_mapped_value(control)
                .map_err(DerivedInputError::Input)?
                != 0.0;
            *timer = held_time(down, *timer, dt);
        }
        let lx = input
            .try_mapped_value(ControllerControl::LeftStickX)
            .map_err(DerivedInputError::Input)?;
        let ly = input
            .try_mapped_value(ControllerControl::LeftStickY)
            .map_err(DerivedInputError::Input)?;
        let rx = input
            .try_mapped_value(ControllerControl::RightStickX)
            .map_err(DerivedInputError::Input)?;
        let ry = input
            .try_mapped_value(ControllerControl::RightStickY)
            .map_err(DerivedInputError::Input)?;
        // `fmuls`1f74/80, captured singleton1fa8, `fmadds`1fac/1fc8.
        // Do not substitute dot3 or separately rounded y*y + x*x.
        let left_x_squared = lx * lx;
        let right_x_squared = rx * rx;
        let captured_attributes = attributes;
        let left_squared = ly.mul_add(ly, left_x_squared);
        let left_seed = numeric
            .reciprocal_sqrt_estimate(left_squared)
            .map_err(DerivedInputError::Numeric)?;
        let right_squared = ry.mul_add(ry, right_x_squared);
        let right_seed = numeric
            .reciprocal_sqrt_estimate(right_squared)
            .map_err(DerivedInputError::Numeric)?;
        let left = length_from_squared_and_estimate(left_squared, left_seed);
        let right = length_from_squared_and_estimate(right_squared, right_seed);
        let threshold = captured_attributes
            .ok_or(DerivedInputError::MissingGlobalAttributes)?
            .threshold()
            .map_err(DerivedInputError::Attribute)?;
        // 20a4=40990010,20c4=40990014: BO4/BI25 means !GT,
        // including unordered. Only ordered greater increments either age.
        self.magnitude_held[0] = if left > threshold {
            self.magnitude_held[0] + dt
        } else {
            0.0
        };
        self.magnitude_held[1] = if right > threshold {
            self.magnitude_held[1] + dt
        } else {
            0.0
        };
        Ok(())
    }
}
fn held_time(down: bool, previous: f32, dt: f32) -> f32 {
    if down { previous + dt } else { 0.0 }
}

impl<I: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for DerivedInputError<I, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(error) => write!(f, "derived input read: {error:?}"),
            Self::Numeric(error) => write!(f, "derived numerical service: {error:?}"),
            Self::MissingGlobalAttributes => f.write_str("derived global attributes unavailable"),
            Self::Attribute(error) => write!(f, "derived input attributes: {error}"),
        }
    }
}
