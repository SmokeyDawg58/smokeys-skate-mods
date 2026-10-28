//! Actual retained `SpeedWobble::CalculateSpeedWobble82b2eb68`.
use super::SkateboardSpeedWobble;

#[path = "skateboard_speed_wobble_calculate_attributes.rs"]
pub(crate) mod attributes;
#[cfg(test)]
#[path = "skateboard_speed_wobble_calculate_tests.rs"]
mod tests;

/// Lookup and scalar load are distinct source operations. A returned handle
/// retains its actual data even across later lookups of the same key.
pub(crate) trait WobbleAttributes {
    type Error;
    type Scalar;
    fn capture(&mut self, key: u64) -> Result<Self::Scalar, Self::Error>;
    fn load(&mut self, scalar: &Self::Scalar) -> Result<f32, Self::Error>;
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error>;
}
pub(crate) trait WobbleServices {
    type Error;
    fn sine(&mut self, angle: f32) -> Result<f32, Self::Error>;
    fn draw_enabled(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum WobbleCalculateError<A, S> {
    Attribute(A),
    Service(S),
}
impl<A: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display for WobbleCalculateError<A, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "speed wobble attribute: {e:?}"),
            Self::Service(e) => write!(f, "speed wobble service: {e:?}"),
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct WobbleInput {
    pub(crate) tilt: f32,
    pub(crate) speed: f32,
    pub(crate) com_to_deck_length: f32,
    pub(crate) truck_tightness: f32,
    pub(crate) dt: f32,
}

/// Scalar fsel clamp ec78..84: NaN selects one; negative zero selects +zero.
fn unit_interval(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

impl SkateboardSpeedWobble {
    /// Writes stay visible on a later service error. The amplitude member+14
    /// remains untouched; frequency here is a local, not the retained member+18.
    pub(crate) fn try_calculate<A: WobbleAttributes, S: WobbleServices>(
        &mut self,
        input: WobbleInput,
        attributes: &mut A,
        services: &mut S,
    ) -> Result<f32, WobbleCalculateError<A::Error, S::Error>> {
        use WobbleCalculateError as E;
        let first_minimum = attributes
            .capture(0x804b_14d3_f9a1_40bb)
            .map_err(E::Attribute)?;
        let maximum = attributes
            .capture(0x1956_9b9f_9dc7_f291)
            .map_err(E::Attribute)?;
        let second_minimum = attributes
            .capture(0x804b_14d3_f9a1_40bb)
            .map_err(E::Attribute)?;
        let denominator_minimum = attributes.load(&second_minimum).map_err(E::Attribute)?;
        let numerator_minimum = attributes.load(&first_minimum).map_err(E::Attribute)?;
        let maximum = attributes.load(&maximum).map_err(E::Attribute)?;
        let numerator = input.com_to_deck_length - numerator_minimum;
        let denominator = maximum - denominator_minimum;
        let com_fraction = unit_interval(1.0 - numerator / denominator);
        let truck_scale = attributes
            .capture(0xf702_ddd8_a4c7_8a7e)
            .map_err(E::Attribute)?;
        let base_speed = attributes
            .capture(0xed44_4c4e_5219_26a0)
            .map_err(E::Attribute)?;
        let com_scale = attributes
            .capture(0x53e8_75b9_61b7_f0d0)
            .map_err(E::Attribute)?;
        let truck_scale = attributes.load(&truck_scale).map_err(E::Attribute)?;
        let was_active = self.active;
        let truck_contribution = truck_scale * input.truck_tightness;
        let com_scale = attributes.load(&com_scale).map_err(E::Attribute)?;
        let base_speed = attributes.load(&base_speed).map_err(E::Attribute)?;
        let threshold = com_scale.mul_add(com_fraction, truck_contribution) + base_speed;
        if !was_active && input.speed > threshold {
            self.time = 0.0;
            self.frequency = 0.0;
            self.last_tilt = 0.0;
            self.active = true;
            self.angle_position = 0.0;
            self.effect = 0.0;
            self.wobble_tilt = 0.0;
        }
        if std::env::var_os("SKATE2_TRACE_SPEED").is_some() && input.speed.abs() > 8.0 {
            eprintln!(
                "speed wobble: speed={} threshold={threshold} com_to_deck={} truck={} active={} time={} effect={}",
                input.speed,
                input.com_to_deck_length,
                input.truck_tightness,
                self.active,
                self.time,
                self.effect
            );
        }
        if !self.active {
            return Ok(input.tilt);
        }
        let speed_range = attributes
            .capture(0x3959_677c_e772_299e)
            .map_err(E::Attribute)?;
        let excess_speed = input.speed - threshold;
        let speed_range = attributes.load(&speed_range).map_err(E::Attribute)?;
        let speed_fraction = unit_interval(excess_speed / speed_range);
        let time_range = attributes
            .capture(0x0f49_4df5_beb3_f126)
            .map_err(E::Attribute)?;
        let time_range = attributes.load(&time_range).map_err(E::Attribute)?;
        let time_fraction = unit_interval(self.time / time_range);
        let frequency_scale = attributes
            .capture(0xfb39_97a3_5341_210c)
            .map_err(E::Attribute)?;
        let speed_frequency = attributes
            .graph(0x40, speed_fraction)
            .map_err(E::Attribute)?;
        let time_frequency = attributes.graph(0, time_fraction).map_err(E::Attribute)?;
        let frequency_scale = attributes.load(&frequency_scale).map_err(E::Attribute)?;
        let frequency = (frequency_scale * time_frequency) * speed_frequency;
        let speed_effect = attributes
            .graph(0xc0, speed_fraction)
            .map_err(E::Attribute)?;
        let time_effect = attributes
            .graph(0x80, time_fraction)
            .map_err(E::Attribute)?;
        self.effect = time_effect * speed_effect;
        let amplitude = attributes
            .capture(0x74ac_73d3_2ac5_c98c)
            .map_err(E::Attribute)?;
        let frequency_step = frequency * input.dt;
        let amplitude = attributes.load(&amplitude).map_err(E::Attribute)? * self.effect;
        self.angle_position =
            frequency_step.mul_add(f32::from_bits(0x40c9_0fdb), self.angle_position);
        let wobble = services.sine(self.angle_position).map_err(E::Service)? * amplitude;
        let combined = wobble + input.tilt;
        self.wobble_tilt = wobble; // ef50 stores f0, not combined f7.
        let output = combined * 0.5;
        if services.draw_enabled().map_err(E::Service)? {
            services.require_overlay().map_err(E::Service)?;
        }
        self.time += input.dt;
        // Raw BO4 BI24 at efa4 skips this reset for unordered comparison.
        if input.speed < threshold {
            self.time = 0.0;
            self.active = false;
            self.frequency = 0.0;
            self.last_tilt = 0.0;
            self.angle_position = 0.0;
            self.effect = 0.0;
            self.wobble_tilt = 0.0;
        }
        Ok(output)
    }
}
