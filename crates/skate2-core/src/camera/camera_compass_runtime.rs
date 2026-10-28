//! Retained compass input publication and native heading update chain.
use super::{CameraCompass, CameraCompassInput};
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_parameters::CameraParameters,
    camera_subject::{CameraSubject, ZERO},
    reckoning_numeric::{
        ReckoningNumeric, length_from_squared_and_estimate, try_length, try_normalize_unsafe,
    },
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
#[path = "camera_compass_math.rs"]
pub(crate) mod math;
use math::{PI, TWO_PI, camera_angle, euler_heading, heading, unit_fraction};

#[derive(Debug)]
pub(crate) enum CameraCompassError<E> {
    OwnerLock,
    UnwrittenParameter(&'static str),
    Numeric(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for CameraCompassError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera compass parameter owner lock poisoned"),
            Self::UnwrittenParameter(name) => {
                write!(f, "camera compass parameter {name} has not been published")
            }
            Self::Numeric(error) => write!(f, "camera compass numeric boundary: {error:?}"),
        }
    }
}

impl CameraCompass {
    /// Native local input is completed before its assignment at `82e38308`.
    pub(crate) fn try_update_subject<N: ReckoningNumeric>(
        &mut self,
        subject: &mut CameraSubject,
        transform: SkeletonRootMatrix,
        published_output: &mut [f32; 4],
        dt: f32,
        numeric: &mut N,
    ) -> Result<(), CameraCompassError<N::Error>> {
        let mut input = CameraCompassInput::constructed();
        input.incline_normal = subject.last_valid_ground_normal;
        input.grinding = subject.flags.grinding;
        input.grind_direction = subject.grind_direction;
        input.mirrored = subject.flags.mirrored;
        input.landing_normal = if subject.flags.trajectory_valid {
            subject.trajectory_end_normal
        } else {
            subject.ground_normal
        };
        input.transform = transform;
        input.zero_velocity = false;
        input.hips_direction = subject.hips_look;
        input.angular_velocity = subject.board_angular_velocity;
        input.on_ground = !subject.flags.trajectory_valid;
        input.override_velocity = subject.flags.ground_animation;
        input.wiping_out = subject.flags.wiping_out;
        input.wipeout_over = subject.flags.wipeout_over;
        if subject.flags.trajectory_valid {
            input.trajectory_fraction =
                unit_fraction(subject.trajectory_elapsed / subject.trajectory_end_time);
            input.trajectory_start = subject.trajectory_start_position;
            input.trajectory_end = subject.trajectory_end_position;
        } else {
            input.trajectory_fraction = 0.0;
            input.trajectory_start = ZERO;
            input.trajectory_end = ZERO;
        }
        input.trajectory_heading = if subject.flags.grinding {
            subject.grind_direction
        } else if subject.flags.trajectory_valid {
            subject.trajectory_target_heading
        } else {
            try_normalize_unsafe(subject.board_velocity, numeric)
                .map_err(CameraCompassError::Numeric)?
        };
        self.input = input;
        self.try_update(if subject.flags.teleported { 0.0 } else { dt }, numeric)?;
        // Separate manager-owned output publication, then Subject stores.
        published_output[0] = self.output[0];
        published_output[1] = self.output[1];
        published_output[2] = self.output[2];
        published_output[3] = self.output[3];
        subject.compass_norths[0] = published_output[0];
        subject.compass_norths[1] = published_output[1];
        subject.compass_norths[2] = 0.0;
        subject.compass_norths[3] = published_output[3];
        subject.compass_norths[4] = published_output[2];
        Ok(())
    }

    fn parameter<E>(
        &self,
        name: &'static str,
        read: impl FnOnce(&CameraParameters) -> Option<f32>,
    ) -> Result<f32, CameraCompassError<E>> {
        let owner = self
            .parameters
            .lock()
            .map_err(|_| CameraCompassError::OwnerLock)?;
        let CameraParameterPublication::Fields(parameters) = &owner.parameters else {
            return Err(CameraCompassError::UnwrittenParameter(name));
        };
        read(parameters).ok_or(CameraCompassError::UnwrittenParameter(name))
    }
    fn try_update<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        numeric: &mut N,
    ) -> Result<(), CameraCompassError<N::Error>> {
        if self.teleported_last_frame {
            self.last_position = self.input.transform.columns()[3];
        }
        self.ground_heading(dt, numeric)?;
        self.incline_heading(numeric)?;
        self.velocity_heading(dt, numeric)?;
        self.offboard_heading =
            heading(self.input.hips_direction, numeric).map_err(CameraCompassError::Numeric)?;
        self.output[1] = self.incline_heading;
        self.output[0] = self.velocity_heading;
        self.output[2] = self.trajectory_heading;
        self.output[3] = self.offboard_heading;
        self.teleported_last_frame = false;
        if dt == 0.0 {
            self.teleported_last_frame = true;
        }
        Ok(())
    }

    fn incline_heading<N: ReckoningNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), CameraCompassError<N::Error>> {
        use CameraCompassError::Numeric;
        let mut ground = heading(self.input.incline_normal, numeric).map_err(Numeric)?;
        let mut landing = heading(self.input.landing_normal, numeric).map_err(Numeric)?;
        if (ground - landing).abs() > PI {
            if ground < landing {
                ground += TWO_PI;
            } else {
                landing += TWO_PI;
            }
        }
        self.incline_heading = (landing - ground).mul_add(self.input.trajectory_fraction, ground);
        self.incline_heading = camera_angle(self.incline_heading, numeric).map_err(Numeric)?;
        Ok(())
    }
    fn velocity_heading<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        numeric: &mut N,
    ) -> Result<(), CameraCompassError<N::Error>> {
        use CameraCompassError::Numeric;
        if self.input.override_velocity {
            return Ok(());
        }
        self.velocity_heading = self.ground_velocity_heading;
        let fraction = self.input.trajectory_fraction;
        let mut delta = self
            .input
            .trajectory_end
            .zip(self.input.trajectory_start, |end, start| end - start);
        delta.xyz[1] = 0.0;
        if fraction > 0.0 && try_length(delta, numeric).map_err(Numeric)? > 1.0 {
            self.trajectory_heading = if dt == 0.0 {
                euler_heading(self.input.hips_direction, numeric).map_err(Numeric)?
            } else {
                -euler_heading(self.input.trajectory_heading, numeric).map_err(Numeric)?
            };
            self.velocity_heading = self.ground_velocity_heading;
            math::interpolate_angle(
                &mut self.velocity_heading,
                self.trajectory_heading,
                fraction,
                numeric,
            )
            .map_err(Numeric)?;
            self.velocity_heading =
                camera_angle(self.velocity_heading, numeric).map_err(Numeric)?;
        }
        Ok(())
    }
    #[expect(
        clippy::assign_op_pattern,
        reason = "retain native dt-first floating operand order"
    )]
    fn ground_heading<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        numeric: &mut N,
    ) -> Result<(), CameraCompassError<N::Error>> {
        use CameraCompassError::Numeric;
        if !self.input.on_ground {
            return Ok(());
        }
        self.update_velocity(dt, numeric)?;
        let horizontal = [
            self.subject_velocity.xyz[0],
            0.0,
            self.subject_velocity.xyz[2],
            0.0,
        ];
        let speed = try_length(horizontal, numeric).map_err(Numeric)?;
        if self.input.grinding {
            let mut direction = self.input.grind_direction;
            direction.xyz[0] *= -1.0;
            self.ground_velocity_heading = euler_heading(direction, numeric).map_err(Numeric)?;
        } else if dt == 0.0 || self.teleported_last_frame {
            self.ground_velocity_heading =
                -euler_heading(self.input.hips_direction, numeric).map_err(Numeric)?;
        } else {
            let mut target = self.ground_velocity_heading;
            let mut response = 0.0;
            if self.input.zero_velocity {
                self.time_since_direction_change = self
                    .parameter("compass_time_before_auto_lineup", |p| {
                        p.compass_time_before_auto_lineup
                    })?;
            }
            if speed > 0.5 {
                let direction =
                    try_normalize_unsafe(self.deadzone_velocity, numeric).map_err(Numeric)?;
                response = unit_fraction(self.parameter("heading_change_response_speed", |p| {
                    p.heading_change_response_speed
                })?);
                let angle = euler_heading(direction, numeric).map_err(Numeric)?;
                self.time_since_direction_change = 0.0;
                target = -angle;
            } else if !self.input.override_velocity {
                self.time_since_direction_change += dt;
                if self.input.wiping_out {
                    self.time_since_direction_change = 0.0;
                }
                let before = self.parameter("compass_time_before_auto_lineup", |p| {
                    p.compass_time_before_auto_lineup
                })?;
                // Raw BO12 BI24 skips only ordered less-than, admitting NaN.
                if self.time_since_direction_change.partial_cmp(&before)
                    != Some(std::cmp::Ordering::Less)
                {
                    response = self
                        .parameter("compass_auto_lineup_speed", |p| p.compass_auto_lineup_speed)?;
                    target = -euler_heading(self.input.transform.columns()[2], numeric)
                        .map_err(Numeric)?;
                }
            }
            let delta = target - self.ground_velocity_heading;
            self.ground_velocity_heading = if self.input.zero_velocity {
                self.ground_velocity_heading + delta
            } else {
                delta.mul_add(response, self.ground_velocity_heading)
            };
            self.ground_velocity_heading =
                camera_angle(self.ground_velocity_heading, numeric).map_err(Numeric)?;
        }
        if self.input.on_ground {
            self.on_ground_counter = dt + self.on_ground_counter;
        } else {
            self.on_ground_counter = 0.0;
        }
        self.subject_on_ground = self.input.on_ground;
        Ok(())
    }
    fn update_velocity<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        numeric: &mut N,
    ) -> Result<(), CameraCompassError<N::Error>> {
        use CameraCompassError::Numeric;
        if self.input.zero_velocity || self.input.override_velocity || dt == 0.0 {
            self.subject_velocity = ZERO;
            self.deadzone_velocity = ZERO;
            return Ok(());
        }
        let previous = self.last_position;
        let position = self.input.transform.columns()[3];
        let delta = position.zip(previous, |a, b| a - b);
        let inverse =
            refine_drive_reciprocal(dt, numeric.reciprocal_estimate(dt).map_err(Numeric)?);
        self.subject_velocity = delta.map(|lane| inverse * (lane * 1.0));
        let old_size = self.deadzone_size;
        self.wipeout_over = false;
        let mut horizontal = self.subject_velocity;
        horizontal.xyz[1] = 0.0;
        let squared = numeric
            .dot_vector(horizontal, horizontal)
            .map_err(Numeric)?;
        let estimate = numeric.reciprocal_sqrt_estimate(squared).map_err(Numeric)?;
        let maximum = self.parameter("compass_max_deadzone_speed", |p| {
            p.compass_max_deadzone_speed
        })?;
        let minimum = self.parameter("compass_min_deadzone_speed", |p| {
            p.compass_min_deadzone_speed
        })?;
        let smoothing = self.parameter("compass_deadzone_size_smoothing", |p| {
            p.compass_deadzone_size_smoothing
        })?;
        let smoothing_factor = 1.0 - smoothing;
        let maximum_size =
            self.parameter("compass_max_deadzone_size", |p| p.compass_max_deadzone_size)?;
        let range = minimum - maximum;
        let speed = length_from_squared_and_estimate(squared, estimate);
        let fraction = unit_fraction((speed - maximum) / range);
        let size_delta = fraction.mul_add(maximum_size, -old_size);
        let size = size_delta.mul_add(smoothing_factor, old_size);
        self.deadzone_size = size;
        self.adaptive_size = size;
        let target = SkeletonRootVector::known([
            position.xyz[0],
            position.xyz[2],
            position.xyz[0],
            position.xyz[0],
        ]);
        self.adaptive_update(dt, target, ZERO, numeric)?;
        let velocity = self.adaptive_velocity;
        self.deadzone_velocity =
            SkeletonRootVector::known([velocity.xyz[0], 0.0, velocity.xyz[1], velocity.xyz[0]]);
        self.last_position = self.input.transform.columns()[3];
        Ok(())
    }
    fn adaptive_update<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        target: SkeletonRootVector,
        zero: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), CameraCompassError<N::Error>> {
        use CameraCompassError::Numeric;
        let old = self.adaptive_position;
        let delta = target.zip(old, |a, b| a - b);
        let direction_squared = delta.xyz[0] * delta.xyz[0] + delta.xyz[1] * delta.xyz[1];
        let length_squared = delta.xyz[0] * delta.xyz[0] + delta.xyz[1] * delta.xyz[1];
        let direction_seed = numeric
            .reciprocal_sqrt_estimate(direction_squared)
            .map_err(Numeric)?;
        let length_seed = numeric
            .reciprocal_sqrt_estimate(length_squared)
            .map_err(Numeric)?;
        let direction_first = (direction_seed * 0.5).mul_add(
            (-direction_squared).mul_add(direction_seed * direction_seed, 1.0),
            direction_seed,
        );
        let length = length_from_squared_and_estimate(length_squared, length_seed);
        if length > self.adaptive_size {
            let amount = length - self.adaptive_size;
            let direction_inverse = (direction_first * 0.5).mul_add(
                (-direction_squared).mul_add(direction_first * direction_first, 1.0),
                direction_first,
            );
            let normalized = delta.map(|v| v * direction_inverse);
            self.adaptive_position =
                normalized.zip(old, |direction, old| direction.mul_add(amount, old));
        }
        self.adaptive_velocity = if dt == 0.0 {
            zero
        } else {
            let displacement = self.adaptive_position.zip(old, |a, b| a - b);
            let inverse_dt = 1.0 / dt;
            displacement.map(|v| v * inverse_dt)
        };
        Ok(())
    }
}

#[cfg(test)]
#[path = "camera_compass_runtime_tests.rs"]
mod tests;
