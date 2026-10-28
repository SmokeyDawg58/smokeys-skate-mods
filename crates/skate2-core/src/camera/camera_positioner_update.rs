//! Native logical positioner, using its retained trackers and collision server.
use super::CameraPositioner;
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_collision::{CameraLineResult, query::CameraQueryError},
    camera_compass::runtime::math::{camera_angle, unit_fraction},
    camera_parameters::CameraParameters,
    camera_rig::desired_position::calculate_position_with_shake,
    camera_tracker::update::ScalarTrackerError,
    collision_line_collection::CollisionLineInput,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, try_length,
        try_normalize_unsafe,
    },
    simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

#[derive(Debug)]
pub(crate) enum PositionerError {
    OwnerLock,
    MissingParameter(&'static str),
    CollisionStartCount(u32),
    MissingFirstResultAssertion,
    Cvar(InputCvarError),
    Numeric(ReckoningHostNumericError),
    Tracker(ScalarTrackerError),
    Query(CameraQueryError),
}
impl std::fmt::Display for PositionerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera positioner owner lock"),
            Self::MissingParameter(name) => {
                write!(f, "camera positioner parameter {name} unproduced")
            }
            Self::CollisionStartCount(count) => {
                write!(f, "camera collision start count {count} exceeds storage")
            }
            Self::MissingFirstResultAssertion => {
                f.write_str("native first collision result assertion")
            }
            Self::Cvar(error) => write!(f, "{error}"),
            Self::Numeric(error) => write!(f, "{error}"),
            Self::Tracker(error) => write!(f, "{error}"),
            Self::Query(error) => write!(f, "{error}"),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PositionResult {
    Valid,
    Colliding,
    TooClose,
    InvalidAnchor,
}

pub(crate) fn clamp_elevation(value: f32) -> f32 {
    let lower = f32::from_bits(0xbfbc_430e);
    let upper = f32::from_bits(0x3fbc_430e);
    let low = if lower - value >= 0.0 { lower } else { value };
    if upper - low >= 0.0 { low } else { upper }
}
fn exceeds_epsilon(value: Vector) -> bool {
    value
        .xyz
        .iter()
        .any(|value| value.abs() > f32::from_bits(0x3780_0000))
}

impl CameraPositioner {
    #[expect(
        clippy::too_many_arguments,
        clippy::fn_params_excessive_bools,
        reason = "native SetParameters scalar and boolean ABI"
    )]
    pub(crate) fn set_parameters(
        &mut self,
        anchor: Vector,
        heading: f32,
        elevation: f32,
        distance: f32,
        avoidance_heading: f32,
        avoidance_elevation: f32,
        minimum: f32,
        collision: bool,
        heavy: bool,
        retry: bool,
        instant: bool,
    ) {
        self.anchor = anchor;
        self.heading = heading;
        self.elevation = elevation;
        self.requested_distance = if distance.is_nan() { 1.0 } else { distance };
        self.avoidance_heading = avoidance_heading;
        self.avoidance_elevation = avoidance_elevation;
        self.minimum_distance = minimum;
        self.flags.collision_check = collision;
        self.flags.heavyweight_hard_cut = heavy;
        self.flags.retry_while_positioning = retry;
        if instant {
            self.distance.input_position = self.requested_distance;
            self.distance.output_position = self.requested_distance;
        }
    }
    pub(crate) fn add_collision_start(&mut self, position: Vector) -> Result<(), PositionerError> {
        let count = self.collision_start_count;
        let slot = self
            .collision_starts
            .get_mut(count as usize)
            .ok_or(PositionerError::CollisionStartCount(count))?;
        *slot = position;
        self.collision_start_count = count.wrapping_add(1);
        Ok(())
    }
    fn parameter<T>(
        &self,
        name: &'static str,
        read: impl FnOnce(&CameraParameters) -> Option<T>,
    ) -> Result<T, PositionerError> {
        let owner = self
            .parameters
            .lock()
            .map_err(|_| PositionerError::OwnerLock)?;
        let CameraParameterPublication::Fields(fields) = &owner.parameters else {
            return Err(PositionerError::MissingParameter(name));
        };
        read(fields).ok_or(PositionerError::MissingParameter(name))
    }
    pub(crate) fn try_update(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), PositionerError> {
        self.near_plane.output_position = self.parameter("near_clipping_distance_min", |p| {
            p.near_clipping_distance_min
        })?;
        self.near_plane.velocity = 0.0;
        self.near_plane.acceleration = 0.0;
        self.collision_radius = cvars
            .try_camera_near_clip_distance()
            .map_err(PositionerError::Cvar)?
            * f32::from_bits(0x3fa6_6666);
        self.position_camera(cvars, services)?;
        if self.flags.colliding {
            self.teleport();
            self.time_since_collision = 0.0;
        } else {
            self.update_distance_tracker(dt)?;
            self.time_since_collision += dt;
        }
        Ok(())
    }
    fn update_distance_tracker(&mut self, dt: f32) -> Result<(), PositionerError> {
        if self.flags.hard_cut
            || (self.distance.output_position > self.requested_distance && self.flags.colliding)
        {
            self.teleport();
        } else {
            self.distance.parameters.speed_clamp =
                Some(self.parameter("distance_speed_clamp", |p| p.distance_speed_clamp)?);
            let acceleration = if self.distance.output_position < f32::from_bits(0x3f86_6666) {
                100.0
            } else {
                self.parameter("distance_acceleration_clamp_max", |p| {
                    p.distance_acceleration_clamp_max
                })?
            };
            self.distance.parameters.acceleration_max = Some(acceleration);
            self.distance.parameters.acceleration_min = Some(acceleration);
            let smoothing = self.parameter("distance_smoothing", |p| p.distance_smoothing)?;
            self.distance.parameters.smoothing_min = Some(smoothing);
            self.distance.parameters.smoothing_max = Some(smoothing);
            self.distance
                .try_update_scalar(dt, self.requested_distance)
                .map_err(PositionerError::Tracker)?;
        }
        Ok(())
    }
    fn calculate(&self) -> Vector {
        calculate_position_with_shake(
            self.anchor,
            self.shake_offset,
            self.elevation,
            self.heading,
            self.distance.output_position,
        )
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "native signed FCFID then FRSP before division"
    )]
    fn position_camera(
        &mut self,
        cvars: &InputCvarReferences,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), PositionerError> {
        use PositionerError as E;
        let previous_colliding = self.flags.colliding;
        let maximum = cvars.try_camera_max_position_retries().map_err(E::Cvar)?;
        self.flags.colliding = false;
        self.flags.using_avoidance_position = false;
        self.flags.camera_position_invalid = false;
        self.flags.anchor_colliding = false;
        self.flags.request_avoidance = false;
        self.flags.heading_elevation_continuous = true;
        let step = f32::from_bits(0x40c9_0fdb) / maximum as f32;
        self.position = self.calculate();
        self.collision_start_index = 0;
        self.current_collision_start = self.collision_starts[0];
        let mut result = self.attempt_batch(services)?;
        if result != PositionResult::Valid {
            self.flags.colliding = true;
        }
        for _ in 0..10 {
            if result != PositionResult::InvalidAnchor {
                break;
            }
            result = self.attempt_single(services)?;
            self.flags.anchor_colliding = true;
            self.flags.colliding = true;
        }
        if result == PositionResult::Colliding {
            self.flags.colliding = true;
            self.distance.velocity = 0.0;
            self.distance.acceleration = 0.0;
            let distance = try_length(
                self.position.zip(self.anchor, |a, b| a - b),
                &mut HostReckoningNumeric,
            )
            .map_err(E::Numeric)?;
            self.requested_distance = distance;
            self.distance.input_position = distance;
            self.distance.output_position = distance;
        }
        let mut retry_entered = false;
        if self.flags.retry_while_positioning && result == PositionResult::TooClose {
            self.flags.using_avoidance_position = false;
            retry_entered = true;
            self.flags.heading_elevation_continuous = false;
            let mut index = 0;
            let mut maximum = cvars.try_camera_max_position_retries().map_err(E::Cvar)?;
            while index < maximum {
                if result != PositionResult::TooClose {
                    break;
                }
                self.heading += step;
                self.heading =
                    camera_angle(self.heading, &mut HostReckoningNumeric).map_err(E::Numeric)?;
                index += 1;
                result = self.attempt_single(services)?;
                maximum = cvars.try_camera_max_position_retries().map_err(E::Cvar)?;
            }
        }
        if result == PositionResult::InvalidAnchor {
            self.position = self.last_good_position;
            self.flags.camera_position_invalid = true;
        } else {
            self.last_good_position = self.position;
        }
        if self.flags.colliding && !retry_entered {
            self.flags.request_avoidance = true;
        }
        if self.flags.heavyweight_hard_cut {
            self.update_hard_cut(services)?;
        } else {
            self.flags.hard_cut = self.flags.colliding && !previous_colliding;
        }
        self.last_position = self.position;
        Ok(())
    }
    fn attempt_batch(
        &mut self,
        services: &mut CameraLineServices<'_>,
    ) -> Result<PositionResult, PositionerError> {
        use PositionerError as E;
        if !self.flags.collision_check || self.collision_start_count == 0 {
            return Ok(PositionResult::Valid);
        }
        let starts = self
            .collision_starts
            .get(..self.collision_start_count as usize)
            .ok_or(E::CollisionStartCount(self.collision_start_count))?;
        let lines: Vec<_> = starts
            .iter()
            .map(|start| CollisionLineInput {
                start: *start,
                end: self.position,
                fatness: self.collision_radius,
            })
            .collect();
        let mut results = Vec::with_capacity(lines.len());
        self.collision
            .lock()
            .map_err(|_| E::OwnerLock)?
            .try_test_lines(&lines, &mut results, services)
            .map_err(E::Query)?;
        if results.iter().any(|result| !result.collided) {
            return Ok(PositionResult::Valid);
        }
        let first = results.first().ok_or(E::MissingFirstResultAssertion)?;
        if !first.collided {
            return Err(E::MissingFirstResultAssertion);
        }
        self.resolve_hit(*first, self.collision_starts[0])
    }
    fn attempt_single(
        &mut self,
        services: &mut CameraLineServices<'_>,
    ) -> Result<PositionResult, PositionerError> {
        use PositionerError as E;
        self.position = self.calculate();
        if !self.flags.collision_check {
            return Ok(PositionResult::Valid);
        }
        let start = self.current_collision_start;
        let result = if exceeds_epsilon(start.zip(self.position, |a, b| a - b)) {
            *self
                .collision
                .lock()
                .map_err(|_| E::OwnerLock)?
                .try_test_line(
                    CollisionLineInput {
                        start,
                        end: self.position,
                        fatness: self.collision_radius,
                    },
                    services,
                )
                .map_err(E::Query)?
        } else {
            self.null_result
        };
        if result.collided && result.position.xyz.iter().all(|value| !value.is_nan()) {
            self.resolve_hit(result, start)
        } else {
            self.distance_result(PositionResult::Valid)
        }
    }
    fn distance_result(&self, far: PositionResult) -> Result<PositionResult, PositionerError> {
        let delta = self
            .position
            .zip(self.current_collision_start, |a, b| a - b);
        let minimum = self.minimum_distance + self.collision_radius;
        let squared = HostReckoningNumeric
            .dot_vector(delta, delta)
            .map_err(PositionerError::Numeric)?;
        Ok(if squared > minimum * minimum {
            far
        } else {
            PositionResult::TooClose
        })
    }
    fn resolve_hit(
        &mut self,
        hit: CameraLineResult,
        start: Vector,
    ) -> Result<PositionResult, PositionerError> {
        use PositionerError as E;
        let t = unit_fraction(hit.parameter);
        self.position = self
            .position
            .zip(start, |end, start| (end - start).mul_add(t, start));
        let delta = start.zip(self.position, |a, b| a - b);
        let metric = HostReckoningNumeric
            .dot_vector(delta, delta)
            .map_err(E::Numeric)?
            * t;
        if metric > f32::from_bits(0x38d1_b717) {
            return self.distance_result(PositionResult::Colliding);
        }
        let delta = start.zip(hit.position, |a, b| a - b);
        let normalized =
            try_normalize_unsafe(delta, &mut HostReckoningNumeric).map_err(E::Numeric)?;
        let mut offset = normalized.zip(delta, |normal, delta| {
            normal.mul_add(f32::from_bits(0x3d4c_cccd), delta)
        });
        if 0.0
            > HostReckoningNumeric
                .dot_vector(offset, hit.normal)
                .map_err(E::Numeric)?
        {
            offset = offset.map(|x| -x);
        }
        self.current_collision_start = hit.position.zip(offset, |a, b| a + b);
        Ok(PositionResult::InvalidAnchor)
    }
    fn update_hard_cut(
        &mut self,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), PositionerError> {
        use PositionerError as E;
        let delta = self.last_position.zip(self.position, |a, b| a - b);
        if !exceeds_epsilon(delta) {
            self.flags.hard_cut = false;
            return Ok(());
        }
        if HostReckoningNumeric
            .dot_vector(delta, delta)
            .map_err(E::Numeric)?
            > 100.0
        {
            self.flags.hard_cut = true;
            return Ok(());
        }
        let hit = self
            .collision
            .lock()
            .map_err(|_| E::OwnerLock)?
            .try_test_line(
                CollisionLineInput {
                    start: self.last_position,
                    end: self.position,
                    fatness: self.collision_radius - f32::from_bits(0x3c23_d70a),
                },
                services,
            )
            .map_err(E::Query)?
            .collided;
        self.flags.hard_cut = hit;
        Ok(())
    }
}

#[cfg(test)]
#[path = "camera_positioner_update_tests.rs"]
mod tests;
