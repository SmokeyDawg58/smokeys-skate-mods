//! Actual retained drop predictor update, batches and scalar camera offsets.
use super::CameraDropPredictor;

#[cfg(test)]
#[path = "camera_drop_update_tests.rs"]
mod tests;
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_parameters::CameraParameters,
    camera_tracker::update::ScalarTrackerError,
    collision_line_collection::CollisionLineInput,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric,
        length_from_squared_and_estimate, try_length, try_normalize_unsafe,
    },
    simulation_assembly_backend::constraint_session::{
        physical_world_line::camera::CameraLineServices,
        world_line_query::{SimulationWorldLineError, SimulationWorldLineLeaseError},
    },
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

#[derive(Debug)]
pub(crate) enum DropUpdateError {
    OwnerLock,
    MissingParameter(&'static str),
    MissingQueryOwner,
    MissingDmoOwner,
    MissingBatchAssertion,
    NotWaitingAssertion,
    DropIndex(i32),
    Numeric(ReckoningHostNumericError),
    Tracker(ScalarTrackerError),
    Cvar(InputCvarError),
    Lease(SimulationWorldLineLeaseError),
    Query(SimulationWorldLineError<ReckoningHostNumericError>),
}
impl std::fmt::Display for DropUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera drop parameter lock"),
            Self::MissingParameter(name) => write!(f, "camera drop parameter {name} is unproduced"),
            Self::MissingQueryOwner => f.write_str("camera drop query owner absent"),
            Self::MissingDmoOwner => f.write_str("camera drop DMO owner absent"),
            Self::MissingBatchAssertion => f.write_str("native drop batch assertion"),
            Self::NotWaitingAssertion => f.write_str("native drop waiting assertion"),
            Self::DropIndex(index) => write!(f, "native drop index {index} outside retained slots"),
            Self::Numeric(error) => write!(f, "camera drop numeric: {error}"),
            Self::Tracker(error) => write!(f, "camera drop tracker: {error}"),
            Self::Cvar(error) => write!(f, "camera drop cvar: {error}"),
            Self::Lease(error) => write!(f, "camera drop lease: {error}"),
            Self::Query(error) => write!(f, "camera drop query: {error}"),
        }
    }
}

fn required<T>(value: Option<T>, name: &'static str) -> Result<T, DropUpdateError> {
    value.ok_or(DropUpdateError::MissingParameter(name))
}
fn unit_clamp(value: f32) -> f32 {
    let low = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - low >= 0.0 { low } else { 1.0 }
}

impl CameraDropPredictor {
    fn read<T>(
        &self,
        name: &'static str,
        get: impl FnOnce(&CameraParameters) -> Option<T>,
    ) -> Result<T, DropUpdateError> {
        let owner = self
            .parameters
            .lock()
            .map_err(|_| DropUpdateError::OwnerLock)?;
        let CameraParameterPublication::Fields(fields) = &owner.parameters else {
            return Err(DropUpdateError::MissingParameter(name));
        };
        required(get(fields), name)
    }

    pub(crate) fn try_update(
        &mut self,
        position: Vector,
        velocity: Vector,
        dt: f32,
        active: bool,
        cvars: &InputCvarReferences,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), DropUpdateError> {
        self.velocity = Vector::known([velocity.xyz[0], 0.0, velocity.xyz[2], 0.0]);
        let mut numeric = HostReckoningNumeric;
        if active {
            let squared = numeric
                .dot_vector(self.velocity, self.velocity)
                .map_err(DropUpdateError::Numeric)?;
            let total_time = self.read("drop_total_test_time", |p| p.drop_total_test_time)?;
            let maximum = self.read("max_look_ahead_distance", |p| p.max_look_ahead_distance)?;
            let waiting = self.waiting_for_results;
            let estimate = numeric
                .reciprocal_sqrt_estimate(squared)
                .map_err(DropUpdateError::Numeric)?;
            let length = length_from_squared_and_estimate(squared, estimate);
            let distance = length * total_time;
            let lower = if -distance >= 0.0 { 0.0 } else { distance };
            let clamped = if maximum - lower >= 0.0 {
                lower
            } else {
                maximum
            };
            self.step_size = clamped / 5.0;
            if waiting {
                self.analyze(services)?;
            }
            self.gather(position, cvars, services)?;
            self.largest_drop = self.find_largest();
        } else {
            self.largest_drop = 0;
            self.current_drop = 0;
            for test in &mut self.tests {
                test.valid = false;
                test.distance = 0.0;
            }
        }
        self.update_camera_values(dt, active)
    }

    fn gather(
        &mut self,
        position: Vector,
        cvars: &InputCvarReferences,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), DropUpdateError> {
        let _line_scope = crate::collision_line_collection::profile::begin("camera_drop");
        if self.batch.is_none() {
            let queries = services
                .queries
                .as_mut()
                .ok_or(DropUpdateError::MissingQueryOwner)?;
            let dmos = services.dmos.ok_or(DropUpdateError::MissingDmoOwner)?;
            self.batch = Some(
                queries
                    .try_acquire_line_batch(dmos, "DropPredictor", 7)
                    .map_err(DropUpdateError::Lease)?,
            );
        }
        let direction = try_normalize_unsafe(self.velocity, &mut HostReckoningNumeric)
            .map_err(DropUpdateError::Numeric)?;
        for index in 0..5 {
            let horizontal = index * 2;
            let vertical = horizontal + 1;
            self.lines[horizontal].start = position;
            let step = direction.map(|v| v * self.step_size);
            let number = f32::from(u8::try_from(index + 1).expect("five authored drop slots"));
            let end = step.zip(position, |v, p| v.mul_add(number, p));
            self.lines[horizontal].end = end;
            self.lines[horizontal].radius = 0.0;
            let drop_distance =
                self.read("drop_max_drop_distance", |p| p.drop_max_drop_distance)?;
            self.lines[vertical].start = end;
            let down = Vector::known([0.0, 1.0, 0.0, 0.0]).map(|v| v * drop_distance);
            self.lines[vertical].end = end.zip(down, |a, b| a - b);
            self.lines[vertical].radius = cvars
                .try_camera_drop_test_radius()
                .map_err(DropUpdateError::Cvar)?;
        }
        let batch = self
            .batch
            .as_ref()
            .ok_or(DropUpdateError::MissingBatchAssertion)?;
        let queries = services
            .queries
            .as_mut()
            .ok_or(DropUpdateError::MissingQueryOwner)?;
        queries.try_begin(batch).map_err(DropUpdateError::Lease)?;
        for line in &self.lines {
            // Native ignores AddTest's capacity bool, retaining child prefixes.
            let _ = queries
                .try_add_line(
                    batch,
                    CollisionLineInput {
                        start: line.start,
                        end: line.end,
                        fatness: line.radius,
                    },
                )
                .map_err(DropUpdateError::Lease)?;
        }
        let dmos = services.dmos.ok_or(DropUpdateError::MissingDmoOwner)?;
        queries
            .try_end(batch, services.ground, dmos, services.numeric)
            .map_err(DropUpdateError::Query)?;
        self.waiting_for_results = true;
        Ok(())
    }

    fn analyze(&mut self, services: &mut CameraLineServices<'_>) -> Result<(), DropUpdateError> {
        let batch = self
            .batch
            .as_ref()
            .ok_or(DropUpdateError::MissingBatchAssertion)?;
        if !self.waiting_for_results {
            return Err(DropUpdateError::NotWaitingAssertion);
        }
        let queries = services
            .queries
            .as_mut()
            .ok_or(DropUpdateError::MissingQueryOwner)?;
        queries.try_sync(batch).map_err(DropUpdateError::Lease)?;
        self.waiting_for_results = false;
        for index in 0..5 {
            if queries
                .try_get_result(batch, u32::try_from(index * 2).expect("ten authored lines"))
                .map_err(DropUpdateError::Lease)?
                .is_some()
            {
                for test in &mut self.tests[index..] {
                    test.valid = false;
                    test.distance = 0.0;
                }
                break;
            }
            let result = queries
                .try_get_result(
                    batch,
                    u32::try_from(index * 2 + 1).expect("ten authored lines"),
                )
                .map_err(DropUpdateError::Lease)?;
            self.tests[index].valid = true;
            self.tests[index].distance =
                self.read("drop_max_drop_distance", |p| p.drop_max_drop_distance)?;
            let line = self.lines[index * 2 + 1];
            self.tests[index].start = line.start;
            self.tests[index].end = line.end;
            self.tests[index].hit = result.is_some();
            if let Some(result) = result {
                let delta = result.position.zip(line.start, |a, b| a - b);
                self.tests[index].distance = try_length(delta, &mut HostReckoningNumeric)
                    .map_err(DropUpdateError::Numeric)?;
                self.tests[index].end = result.position;
            }
        }
        Ok(())
    }

    fn find_largest(&self) -> i32 {
        let mut chosen = 0;
        let mut distance = 0.0;
        for (index, test) in self.tests.iter().enumerate() {
            if !test.valid {
                break;
            }
            if test.distance > distance {
                chosen = i32::try_from(index).expect("five authored drop slots");
                distance = test.distance;
            }
        }
        chosen
    }
    fn first_drop(&self) -> Result<i32, DropUpdateError> {
        for (index, test) in self.tests.iter().enumerate() {
            if !test.valid {
                break;
            }
            let minimum = self.read("drop_min_drop_distance", |p| p.drop_min_drop_distance)?;
            if test.distance > minimum {
                return Ok(i32::try_from(index).expect("five authored drop slots"));
            }
        }
        Ok(4)
    }

    fn update_camera_values(&mut self, dt: f32, active: bool) -> Result<(), DropUpdateError> {
        let largest = self
            .tests
            .get(
                usize::try_from(self.largest_drop)
                    .map_err(|_| DropUpdateError::DropIndex(self.largest_drop))?,
            )
            .ok_or(DropUpdateError::DropIndex(self.largest_drop))?
            .distance;
        let mut drop_fraction = 0.0;
        let mut lookahead_fraction = 0.0;
        if active {
            let minimum = self.read("drop_min_drop_distance", |p| p.drop_min_drop_distance)?;
            if largest > minimum {
                let minimum_velocity =
                    self.read("drop_min_actor_velocity", |p| p.drop_min_actor_velocity)?;
                let minimum_squared = minimum_velocity * minimum_velocity;
                let minimum = self.read("drop_min_drop_distance", |p| p.drop_min_drop_distance)?;
                let difference = largest - minimum;
                let maximum = self.read("drop_max_drop_distance", |p| p.drop_max_drop_distance)?;
                let width = maximum - minimum;
                let speed_squared = HostReckoningNumeric
                    .dot_vector(self.velocity, self.velocity)
                    .map_err(DropUpdateError::Numeric)?;
                drop_fraction = difference / width;
                if minimum_squared.partial_cmp(&speed_squared) != Some(std::cmp::Ordering::Greater)
                {
                    let step = self.step_size;
                    let index = self.first_drop()? + 1;
                    let denominator = step * 5.0;
                    let numerator = f32::from(
                        i16::try_from(index).expect("first drop is between zero and four"),
                    ) * step;
                    lookahead_fraction = numerator / denominator;
                }
            }
        }
        let curve = self.read("drop_elevation_curve", |p| p.drop_elevation_curve.clone())?;
        let curve_value = curve.evaluate(drop_fraction);
        let elevation = self.read("drop_elevation_offset", |p| p.drop_elevation_offset)?;
        self.target_elevation_offset = (elevation * f32::from_bits(0x3c8e_fa35)) * curve_value;
        let valid = i32::try_from(self.tests.iter().take_while(|test| test.valid).count())
            .expect("five authored drop slots");
        let smoothing = if self
            .target_elevation_offset
            .partial_cmp(&self.elevation_offset)
            != Some(std::cmp::Ordering::Greater)
            && active
            && valid <= self.previous_valid_drops
        {
            let curve = self.read("incline_smoothing_curve", |p| {
                p.incline_smoothing_curve.clone()
            })?;
            let value = curve.evaluate(lookahead_fraction);
            self.was_using_drop_curve = false;
            value
        } else {
            let curve = self.read("drop_smoothing_curve", |p| p.drop_smoothing_curve.clone())?;
            let value = curve.evaluate(lookahead_fraction);
            self.was_using_drop_curve = true;
            value
        };
        self.smoothing_tracker.parameters.speed_clamp =
            Some(self.read("drop_tracker_speed_clamp", |p| p.drop_tracker_speed_clamp)?);
        let acceleration = self.read("drop_tracker_acceleration_clamp", |p| {
            p.drop_tracker_acceleration_clamp
        })?;
        self.smoothing_tracker.parameters.acceleration_min = Some(acceleration);
        self.smoothing_tracker.parameters.acceleration_max = Some(acceleration);
        let tracker_smoothing =
            self.read("drop_tracker_smoothing", |p| p.drop_tracker_smoothing)?;
        self.smoothing_tracker.parameters.smoothing_min = Some(tracker_smoothing);
        self.smoothing_tracker.parameters.smoothing_max = Some(tracker_smoothing);
        self.smoothing_tracker
            .try_update_scalar(dt, smoothing)
            .map_err(DropUpdateError::Tracker)?;
        let output = self.smoothing_tracker.output_position;
        let old = self.elevation_offset;
        let difference = self.target_elevation_offset - old;
        self.previous_valid_drops = valid;
        self.smoothing = unit_clamp(output);
        let change = difference * (1.0 - self.smoothing);
        let maximum = f32::from_bits(0x3c0e_fa35);
        let lower = if -maximum - change >= 0.0 {
            -maximum
        } else {
            change
        };
        let clamped = if maximum - lower >= 0.0 {
            lower
        } else {
            maximum
        };
        self.elevation_offset = clamped + old;
        Ok(())
    }
}
