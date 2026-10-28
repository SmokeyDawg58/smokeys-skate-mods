//! Actual scalar `Tracker::Update82e35800`; constructor-unknown tuning stays unknown.
use super::CameraTracker;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarTrackerError {
    MissingParameter(&'static str),
    UnknownDistanceWeight,
}
impl std::fmt::Display for ScalarTrackerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingParameter(name) => write!(f, "unproduced scalar tracker parameter {name}"),
            Self::UnknownDistanceWeight => f.write_str("unproduced scalar tracker distance weight"),
        }
    }
}

pub(super) fn required(value: Option<f32>, name: &'static str) -> Result<f32, ScalarTrackerError> {
    value.ok_or(ScalarTrackerError::MissingParameter(name))
}

/// Unknown scratch is harmless when both native range endpoints are equal.
/// The two source comparisons are ordered LT: NaN selects the final zero.
pub(super) fn distance_weight(
    distance: f32,
    umbra: Option<f32>,
    penumbra: Option<f32>,
) -> Option<f32> {
    if distance.is_nan() {
        return Some(0.0);
    }
    let umbra = umbra?;
    if distance < umbra {
        return Some(1.0);
    }
    let penumbra = penumbra?;
    Some(if distance < penumbra {
        1.0 - ((distance - umbra) / (penumbra - umbra))
    } else {
        0.0
    })
}

#[expect(
    clippy::float_cmp,
    reason = "native EQ skips interpolation, including both signed zeros"
)]
pub(super) fn interpolate_range(
    minimum: f32,
    maximum: f32,
    weight: Option<f32>,
) -> Result<f32, ScalarTrackerError> {
    if minimum == maximum {
        Ok(maximum)
    } else {
        Ok((maximum - minimum).mul_add(
            weight.ok_or(ScalarTrackerError::UnknownDistanceWeight)?,
            minimum,
        ))
    }
}

impl CameraTracker<f32> {
    pub(crate) fn try_update_scalar(
        &mut self,
        dt: f32,
        target: f32,
    ) -> Result<(), ScalarTrackerError> {
        let overshoot = self.parameters.overshoot_zeroes_velocity;
        let position = self.output_position;
        self.input_position = target;
        let delta = target - position;
        if overshoot && self.velocity * delta < 0.0 {
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
            let minimum = required(self.parameters.acceleration_min, "acceleration_min")?;
            let maximum = required(self.parameters.acceleration_max, "acceleration_max")?;
            let clamp = interpolate_range(minimum, maximum, weight)?;
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
        let speed_clamp = required(speed_clamp, "speed_clamp")?;
        self.velocity = if absolute_velocity > speed_clamp {
            (unclamped_velocity / absolute_velocity) * speed_clamp
        } else {
            unclamped_velocity
        };
        let prospective_position = self.velocity.mul_add(dt, position);
        let minimum = required(self.parameters.smoothing_min, "smoothing_min")?;
        let maximum = required(self.parameters.smoothing_max, "smoothing_max")?;
        let smoothing = interpolate_range(minimum, maximum, weight)?;
        let displacement = prospective_position - position;
        self.smoothing = smoothing;
        self.output_position = displacement.mul_add(1.0 - smoothing, position);
        Ok(())
    }
}

#[cfg(test)]
#[path = "camera_tracker_update_tests.rs"]
mod tests;
