//! Native heading/elevation tuning and angle-tracker calls.
use super::{CameraRig, dynamics::RigDynamicsError};
use crate::{
    camera_compass::runtime::math::camera_angle_with_inverse,
    input_cvar_runtime::{InputCvarReferences, camera_dynamics::CameraDynamicsBoolean as B},
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, try_normalize_angle},
};
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

impl CameraRig {
    pub(super) fn update_interpolated_heading(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
    ) -> Result<(), RigDynamicsError> {
        use RigDynamicsError as E;
        if self.second_flags & 0x20 == 0 && self.avoidance_heading_amount == 0.0 {
            return Ok(());
        }
        let mut smoothing = heading_smoothing(
            self.heading_tracker.output_position + self.heading_offset,
            self.requested_heading,
            self.heading_speed,
            &mut HostReckoningNumeric,
        )
        .map_err(E::Numeric)?;
        if self.initialization_time > 0.0 {
            self.heading_tracker
                .try_set_angle_instant(self.requested_heading, true, &mut HostReckoningNumeric)
                .map_err(E::Numeric)?;
        }
        let speed = if cvars
            .try_camera_dynamics_boolean(B::HeadingSpeed)
            .map_err(E::Cvar)?
        {
            self.dynamics_parameter("heading_speed_clamp", |p| p.heading_speed_clamp)?
                * DEGREES_TO_RADIANS
        } else {
            f32::MAX
        };
        let acceleration_enabled = cvars
            .try_camera_dynamics_boolean(B::HeadingAcceleration)
            .map_err(E::Cvar)?;
        let mut minimum = if acceleration_enabled {
            self.dynamics_parameter("heading_acceleration_clamp_min", |p| {
                p.heading_acceleration_clamp_min
            })? * DEGREES_TO_RADIANS
        } else {
            f32::MAX
        };
        let mut maximum = if acceleration_enabled {
            self.dynamics_parameter("heading_acceleration_clamp_max", |p| {
                p.heading_acceleration_clamp_max
            })? * DEGREES_TO_RADIANS
        } else {
            f32::MAX
        };
        if self.avoidance_heading_amount > 0.0 {
            minimum *= 2.0;
            maximum *= 2.0;
        }
        if !cvars
            .try_camera_dynamics_boolean(B::HeadingSmoothing)
            .map_err(E::Cvar)?
        {
            smoothing = 0.0;
        }
        self.heading_tracker.parameters.speed_clamp = Some(speed);
        self.heading_tracker.parameters.acceleration_min = Some(minimum);
        self.heading_tracker.parameters.acceleration_max = Some(maximum);
        self.heading_tracker.parameters.umbra = Some(
            self.dynamics_parameter("heading_delta_umbra", |p| p.heading_delta_umbra)?
                * DEGREES_TO_RADIANS,
        );
        self.heading_tracker.parameters.penumbra = Some(
            self.dynamics_parameter("heading_delta_penumbra", |p| p.heading_delta_penumbra)?
                * DEGREES_TO_RADIANS,
        );
        self.heading_tracker.parameters.smoothing_max = Some(smoothing);
        self.heading_tracker.parameters.smoothing_min = Some(smoothing);
        self.heading_tracker.parameters.overshoot_zeroes_velocity = false;
        let target = try_normalize_angle(self.requested_heading, &mut HostReckoningNumeric)
            .map_err(E::Numeric)?;
        self.heading_tracker
            .try_update_angle(dt, target, &mut HostReckoningNumeric)
            .map_err(E::Angle)
    }

    pub(super) fn update_interpolated_elevation(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
    ) -> Result<(), RigDynamicsError> {
        use RigDynamicsError as E;
        if self.initialization_time > 0.0 {
            self.elevation_tracker
                .try_set_angle_instant(self.requested_elevation, true, &mut HostReckoningNumeric)
                .map_err(E::Numeric)?;
        }
        let speed = if cvars
            .try_camera_dynamics_boolean(B::ElevationSpeed)
            .map_err(E::Cvar)?
        {
            self.dynamics_parameter("elevation_speed_clamp", |p| p.elevation_speed_clamp)?
                * DEGREES_TO_RADIANS
        } else {
            f32::MAX
        };
        let acceleration_enabled = cvars
            .try_camera_dynamics_boolean(B::ElevationAcceleration)
            .map_err(E::Cvar)?;
        let minimum = if acceleration_enabled {
            self.dynamics_parameter("elevation_acceleration_clamp_min", |p| {
                p.elevation_acceleration_clamp_min
            })? * DEGREES_TO_RADIANS
        } else {
            f32::MAX
        };
        let maximum = if acceleration_enabled {
            self.dynamics_parameter("elevation_acceleration_clamp_max", |p| {
                p.elevation_acceleration_clamp_max
            })? * DEGREES_TO_RADIANS
        } else {
            f32::MAX
        };
        let smoothing = if cvars
            .try_camera_dynamics_boolean(B::ElevationSmoothing)
            .map_err(E::Cvar)?
        {
            1.0 - self.elevation_speed
        } else {
            0.0
        };
        self.elevation_tracker.parameters.speed_clamp = Some(speed);
        self.elevation_tracker.parameters.acceleration_min = Some(minimum);
        self.elevation_tracker.parameters.acceleration_max = Some(maximum);
        self.elevation_tracker.parameters.umbra = Some(
            self.dynamics_parameter("elevation_delta_umbra", |p| p.elevation_delta_umbra)?
                * DEGREES_TO_RADIANS,
        );
        self.elevation_tracker.parameters.penumbra = Some(
            self.dynamics_parameter("elevation_delta_penumbra", |p| p.elevation_delta_penumbra)?
                * DEGREES_TO_RADIANS,
        );
        self.elevation_tracker.parameters.smoothing_min = Some(smoothing);
        self.elevation_tracker.parameters.smoothing_max = Some(smoothing);
        let target = try_normalize_angle(self.requested_elevation, &mut HostReckoningNumeric)
            .map_err(E::Numeric)?;
        self.elevation_tracker
            .try_update_angle(dt, target, &mut HostReckoningNumeric)
            .map_err(E::Angle)
    }
}

/// Two independently seeded reciprocal graphs precede either remainder.
fn heading_smoothing<N: ReckoningNumeric>(
    position: f32,
    requested: f32,
    speed: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let first = numeric.reciprocal_estimate(TWO_PI)?;
    let base = 1.0 - speed;
    let second = numeric.reciprocal_estimate(TWO_PI)?;
    let cap = if f32::from_bits(0x3f4c_cccd) - base >= 0.0 {
        base
    } else {
        f32::from_bits(0x3f4c_cccd)
    };
    let first = first.mul_add((-first).mul_add(TWO_PI, 1.0), first);
    let second = second.mul_add((-second).mul_add(TWO_PI, 1.0), second);
    let first = first.mul_add((-first).mul_add(TWO_PI, 1.0), first);
    let second = second.mul_add((-second).mul_add(TWO_PI, 1.0), second);
    let first_angle = camera_angle_with_inverse(position, first);
    let delta = camera_angle_with_inverse(requested - first_angle, second).abs();
    Ok(if delta < f32::from_bits(0x3f49_0fdb) {
        (cap - base).mul_add(delta * f32::from_bits(0x3fa2_f984), base)
    } else {
        cap
    })
}
#[cfg(test)]
#[path = "camera_rig_angle_trackers_tests.rs"]
mod tests;
