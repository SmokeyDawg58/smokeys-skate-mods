//! Native full-lane `Tracker<Vector3>.Update82e40ae8` and its magnitude clamp.
use super::{
    CameraTracker,
    update::{ScalarTrackerError, distance_weight, interpolate_range, required},
};
use crate::{
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate, try_length},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Debug)]
pub(crate) enum VectorTrackerError<E> {
    Numeric(E),
    Parameter(ScalarTrackerError),
}
impl<E: std::fmt::Display> std::fmt::Display for VectorTrackerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "{error}"),
            Self::Parameter(error) => write!(f, "{error}"),
        }
    }
}

impl CameraTracker<SkeletonRootVector> {
    pub(crate) fn try_update_vector<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        target: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), VectorTrackerError<N::Error>> {
        use VectorTrackerError::{Numeric, Parameter};
        self.input_position = target;
        let position = self.output_position;
        let delta = target.zip(position, |a, b| a - b);
        if self.parameters.overshoot_zeroes_velocity
            && numeric.dot_vector(delta, self.velocity).map_err(Numeric)? < 0.0
        {
            self.velocity = self.velocity.map(|x| x * 0.0);
        }
        let squared = numeric.dot_vector(delta, delta).map_err(Numeric)?;
        let velocity = self.velocity;
        let length_seed = numeric.reciprocal_sqrt_estimate(squared).map_err(Numeric)?;
        let first_seed = numeric.reciprocal_estimate(dt).map_err(Numeric)?;
        let second_seed = numeric.reciprocal_estimate(dt).map_err(Numeric)?;
        let first = refine_drive_reciprocal(dt, first_seed);
        let second = refine_drive_reciprocal(dt, second_seed);
        let difference = delta.map(|x| first * x).zip(velocity, |a, b| a - b);
        let length = length_from_squared_and_estimate(squared, length_seed);
        self.acceleration = difference.map(|x| second * x);
        let weight = distance_weight(length, self.parameters.umbra, self.parameters.penumbra);
        if numeric
            .dot_vector(self.acceleration, delta)
            .map_err(Numeric)?
            > 0.0
        {
            let minimum = required(self.parameters.acceleration_min, "acceleration_min")
                .map_err(Parameter)?;
            let maximum = required(self.parameters.acceleration_max, "acceleration_max")
                .map_err(Parameter)?;
            let maximum = interpolate_range(minimum, maximum, weight).map_err(Parameter)?;
            self.acceleration_clamp = maximum;
            self.acceleration =
                clamp_magnitude(self.acceleration, maximum, numeric).map_err(Numeric)?;
        }
        self.velocity = self
            .acceleration
            .zip(self.velocity, |a, b| a.mul_add(dt, b));
        let maximum = required(self.parameters.speed_clamp, "speed_clamp").map_err(Parameter)?;
        self.velocity = clamp_magnitude(self.velocity, maximum, numeric).map_err(Numeric)?;
        let maximum =
            required(self.parameters.smoothing_max, "smoothing_max").map_err(Parameter)?;
        let minimum =
            required(self.parameters.smoothing_min, "smoothing_min").map_err(Parameter)?;
        let prospective = self
            .velocity
            .zip(self.output_position, |a, b| a.mul_add(dt, b));
        let smoothing = interpolate_range(minimum, maximum, weight).map_err(Parameter)?;
        let displacement = prospective.zip(self.output_position, |a, b| a - b);
        self.smoothing = smoothing;
        self.output_position =
            displacement.zip(self.output_position, |a, b| a.mul_add(1.0 - smoothing, b));
        Ok(())
    }
}

/// Unlike general `Math::Clamp`, this native helper has no epsilon or assertion.
fn clamp_magnitude<N: ReckoningNumeric>(
    value: SkeletonRootVector,
    maximum: f32,
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    let length = try_length(value, numeric)?;
    Ok(if length > maximum {
        let scale = maximum / length;
        value.map(|x| x * scale)
    } else {
        value
    })
}

#[cfg(test)]
#[path = "camera_tracker_vector_update_tests.rs"]
mod tests;
