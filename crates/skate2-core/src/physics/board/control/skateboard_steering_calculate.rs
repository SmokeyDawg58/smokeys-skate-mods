//! Complete `Toolkit_CalcSteeringTilt82b27ae0`, separate from tilt application.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SteeringScalar {
    Turn,
    HardTurn,
    AbsoluteSpeed,
    TruckTightness,
    Balance,
    FlippedControls,
}
pub(crate) trait SteeringCalculationInput {
    type Error;
    fn scalar(&self, field: SteeringScalar) -> Result<f32, Self::Error>;
    fn pushing(&self) -> Result<bool, Self::Error>;
}
pub(crate) trait SteeringCalculationAttributes {
    type Error;
    fn steering_scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn manual_scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SteeringCalculationError<I, A> {
    Input(I),
    Attribute(A),
}
impl<I: std::fmt::Debug, A: std::fmt::Debug> std::fmt::Display for SteeringCalculationError<I, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(e) => write!(f, "steering calculation input: {e:?}"),
            Self::Attribute(e) => write!(f, "steering calculation attribute: {e:?}"),
        }
    }
}

/// Optional pointers are native caller choices. Ground passes BOTH actual fields;
/// each old value is read only at its source site and its write precedes later work.
pub(crate) fn try_calculate<I: SteeringCalculationInput, A: SteeringCalculationAttributes>(
    input: &I,
    attributes: &mut A,
    push_scalar: Option<&mut f32>,
    damped_turn: Option<&mut f32>,
) -> Result<f32, SteeringCalculationError<I::Error, A::Error>> {
    use SteeringCalculationError as E;
    use SteeringScalar as S;
    let mut turn = input.scalar(S::Turn).map_err(E::Input)?;
    let hard_scale = attributes
        .steering_scalar(0xdbd1_d2af_088b_a302)
        .map_err(E::Attribute)?;
    let hard_turn = input.scalar(S::HardTurn).map_err(E::Input)?;
    let sign = if turn >= 0.0 { 1.0 } else { -1.0 }; // fsel: unordered selects -1.
    let hard_multiplier = hard_turn.abs().mul_add(hard_scale, 1.0);
    if hard_turn != 0.0 {
        turn = sign;
    }
    if let Some(previous) = damped_turn {
        let coefficient = attributes
            .steering_scalar(0xc40b_e048_fab4_f0b3)
            .map_err(E::Attribute)?;
        let remaining = 1.0 - coefficient;
        let old = *previous;
        let current = coefficient * turn;
        turn = remaining.mul_add(old, current);
        *previous = turn;
    }
    // Ordered BLT, preserving negative zero and unordered sign bits.
    let absolute_turn = if turn < 0.0 { -turn } else { turn };
    let speed_divisor = attributes
        .steering_scalar(0x6481_a86a_7e6d_5efc)
        .map_err(E::Attribute)?;
    let speed = input.scalar(S::AbsoluteSpeed).map_err(E::Input)?;
    let fraction = speed / speed_divisor; // Native scalar fdivs7cac.
    let lower = if -fraction >= 0.0 { 0.0 } else { fraction };
    let fraction = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
    let speed_factor = attributes.graph(0, fraction).map_err(E::Attribute)?;
    let turn_factor = attributes
        .graph(0x40, absolute_turn)
        .map_err(E::Attribute)?;
    let push_factor = if let Some(previous) = push_scalar {
        let increase = attributes
            .steering_scalar(0xdb8f_95ac_3f0d_e414)
            .map_err(E::Attribute)?;
        let decrease = attributes
            .steering_scalar(0xe6b6_a5e8_74f4_5937)
            .map_err(E::Attribute)?;
        let minimum = attributes
            .steering_scalar(0xbe1a_0232_c2b2_358b)
            .map_err(E::Attribute)?;
        let old = *previous;
        let value = if input.pushing().map_err(E::Input)? {
            old - decrease
        } else {
            old + increase
        };
        // Native NOT GT then NOT LT: NaN remains the selected value.
        let value = if value > 1.0 {
            1.0
        } else if value < minimum {
            minimum
        } else {
            value
        };
        *previous = value;
        value
    } else {
        1.0
    };
    let tightness = input.scalar(S::TruckTightness).map_err(E::Input)?;
    let tightness_scalar = attributes
        .steering_scalar(0x3658_38a8_b87e_eda6)
        .map_err(E::Attribute)?;
    let tightness_base = tightness_scalar.mul_add(tightness, 1.0);
    let balance = input.scalar(S::Balance).map_err(E::Input)?;
    let tightness_factor = tightness_base - tightness;
    let manual = if balance == 0.0 {
        1.0
    } else {
        attributes
            .manual_scalar(0x4c9e_095b_dc2f_1de5)
            .map_err(E::Attribute)?
    };
    let scalar = attributes
        .steering_scalar(0xf0fc_892e_a704_30cb)
        .map_err(E::Attribute)?;
    let flipped = input.scalar(S::FlippedControls).map_err(E::Input)?;
    let result = flipped * scalar;
    let result = result * manual;
    let result = result * tightness_factor;
    let result = result * push_factor;
    let result = result * turn_factor;
    let result = result * speed_factor;
    let result = result * hard_multiplier;
    Ok(result * turn)
}

#[cfg(test)]
#[path = "skateboard_steering_calculate_tests.rs"]
mod tests;
