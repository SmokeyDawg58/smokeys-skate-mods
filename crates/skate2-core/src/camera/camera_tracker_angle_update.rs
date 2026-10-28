//! Actual `Tracker<Math::Angle>::Update82e40e40` on the retained tracker.
use super::{
    CameraTracker,
    update::{ScalarTrackerError, distance_weight, interpolate_range, required},
};
use crate::reckoning_numeric::{ReckoningNumeric, try_normalize_angle};

#[derive(Debug)]
pub(crate) enum AngleTrackerError<E> {
    Numeric(E),
    Parameter(ScalarTrackerError),
}
impl<E: std::fmt::Display> std::fmt::Display for AngleTrackerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "{error}"),
            Self::Parameter(error) => write!(f, "{error}"),
        }
    }
}
impl CameraTracker<f32> {
    /// Shared stores of native heading/elevation instant helpers.
    pub(crate) fn try_set_angle_instant<N: ReckoningNumeric>(
        &mut self,
        target: f32,
        zero_rate: bool,
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        self.input_position = try_normalize_angle(target, numeric)?;
        self.output_position = try_normalize_angle(target, numeric)?;
        if zero_rate {
            self.velocity = 0.0;
            self.acceleration = 0.0;
        }
        Ok(())
    }
    /// The six angle calls and their intermediate stores differ from scalar tracking.
    pub(crate) fn try_update_angle<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        target: f32,
        numeric: &mut N,
    ) -> Result<(), AngleTrackerError<N::Error>> {
        use AngleTrackerError::{Numeric, Parameter};
        let position = self.output_position;
        self.input_position = target;
        let delta = try_normalize_angle(target - position, numeric).map_err(Numeric)?;
        if self.parameters.overshoot_zeroes_velocity && self.velocity * delta < 0.0 {
            self.velocity *= 0.0;
        }
        let velocity = self.velocity;
        let absolute_delta = delta.abs();
        let umbra = self.parameters.umbra;
        let penumbra = self.parameters.penumbra;
        let reciprocal_dt = 1.0 / dt;
        let acceleration = reciprocal_dt.mul_add(delta, -velocity) * reciprocal_dt;
        self.acceleration = acceleration;
        let weight = distance_weight(absolute_delta, umbra, penumbra);
        if acceleration * delta > 0.0 {
            let minimum = required(self.parameters.acceleration_min, "acceleration_min")
                .map_err(Parameter)?;
            let maximum = required(self.parameters.acceleration_max, "acceleration_max")
                .map_err(Parameter)?;
            let clamp = interpolate_range(minimum, maximum, weight).map_err(Parameter)?;
            let absolute_acceleration = acceleration.abs();
            self.acceleration_clamp = clamp;
            self.acceleration = if absolute_acceleration > clamp {
                (acceleration / absolute_acceleration) * clamp
            } else {
                acceleration
            };
        }
        let unclamped_velocity = self.acceleration.mul_add(dt, velocity);
        let speed_clamp = self.parameters.speed_clamp;
        self.velocity = unclamped_velocity;
        let absolute_velocity = unclamped_velocity.abs();
        let speed_clamp = required(speed_clamp, "speed_clamp").map_err(Parameter)?;
        self.velocity = if absolute_velocity > speed_clamp {
            (unclamped_velocity / absolute_velocity) * speed_clamp
        } else {
            unclamped_velocity
        };
        let step = try_normalize_angle(self.velocity * dt, numeric).map_err(Numeric)?;
        let prospective = try_normalize_angle(step + position, numeric).map_err(Numeric)?;
        let minimum =
            required(self.parameters.smoothing_min, "smoothing_min").map_err(Parameter)?;
        let maximum =
            required(self.parameters.smoothing_max, "smoothing_max").map_err(Parameter)?;
        self.smoothing = interpolate_range(minimum, maximum, weight).map_err(Parameter)?;
        let displacement = try_normalize_angle(prospective - position, numeric).map_err(Numeric)?;
        let smoothed =
            try_normalize_angle(displacement * (1.0 - self.smoothing), numeric).map_err(Numeric)?;
        self.output_position =
            try_normalize_angle(smoothed + position, numeric).map_err(Numeric)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "camera_tracker_angle_update_tests.rs"]
mod tests;
