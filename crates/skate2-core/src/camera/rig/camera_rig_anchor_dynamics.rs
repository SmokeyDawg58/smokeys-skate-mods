//! Actual acceleration/anchor producers and ordered Rig tail before Positioner.
use super::{CameraBreadcrumbs, CameraRig, dynamics::RigDynamicsError};
use crate::{
    camera_compass::runtime::math::{euler_angles, unit_fraction},
    camera_subject::ZERO,
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningNumeric, length_from_squared_and_estimate, try_length,
        try_normalize_safe, try_normalize_unsafe,
    },
    skeleton_root_transforms::SkeletonRootVector,
};

impl CameraRig {
    pub(super) fn try_update_anchor_dynamics(&mut self, dt: f32) -> Result<(), RigDynamicsError> {
        use RigDynamicsError as E;
        let dot = acceleration_along_velocity(
            self.subject_velocity,
            self.subject_acceleration,
            &mut HostReckoningNumeric,
        )
        .map_err(E::Numeric)?;
        #[expect(
            clippy::manual_midpoint,
            reason = "native82e3ce18 rounds the addition before multiplication, including overflow"
        )]
        let scalar = (self.last_acceleration_scalar + dot) * 0.5;
        self.last_acceleration_scalar = dot;
        self.acceleration_scalar = scalar;
        self.update_anchor_tracker(dt)?;
        if self.collision_interpolation_time > 0.0 {
            self.collision_interpolation_time -= dt;
        }
        if self.collision_interpolation_time < 0.0 {
            self.collision_interpolation_time = 0.0;
        }
        if self.positioner.flags.colliding {
            self.collision_interpolation_time = self
                .dynamics_parameter("collision_interpolation_duration", |p| {
                    p.collision_interpolation_duration
                })?;
        }
        let mut position = self.anchor_tracker.output_position;
        position.xyz[1] = self.reference_height;
        self.breadcrumbs
            .try_set_position(position, &mut HostReckoningNumeric)?;
        self.update_anchor_change()?;
        Ok(())
    }

    fn update_anchor_tracker(&mut self, dt: f32) -> Result<(), RigDynamicsError> {
        use RigDynamicsError as E;
        let mut numeric = HostReckoningNumeric;
        let horizontal_acceleration = horizontal(self.subject_acceleration);
        let horizontal_forward = horizontal(self.anchor_matrix.columns()[2]);
        if 0.0
            > numeric
                .dot_vector(horizontal_acceleration, horizontal_forward)
                .map_err(E::Numeric)?
        {
            let squared = numeric
                .dot_vector(horizontal_acceleration, horizontal_acceleration)
                .map_err(E::Numeric)?;
            let threshold = self
                .dynamics_parameter("tracker_subject_acceleration_for_max_smoothing", |p| {
                    p.tracker_subject_acceleration_for_max_smoothing
                })?;
            let seed = numeric
                .reciprocal_sqrt_estimate(squared)
                .map_err(E::Numeric)?;
            let length = length_from_squared_and_estimate(squared, seed);
            if length > threshold {
                self.subject_tracker_smoothing_time = self
                    .dynamics_parameter("tracker_subject_smoothing_duration", |p| {
                        p.tracker_subject_smoothing_duration
                    })?;
            }
        }
        self.subject_tracker_smoothing_time -= dt;
        self.subject_tracker_smoothing_time = if self.subject_tracker_smoothing_time >= 0.0 {
            self.subject_tracker_smoothing_time
        } else {
            0.0
        };
        let push = self.calculate_push_smoothing(dt)?;
        let pump = self.calculate_pump_smoothing(dt)?;
        let largest = if push - pump >= 0.0 { push } else { pump };
        let distance = try_length(
            self.positioner
                .position
                .zip(self.anchor_matrix.columns()[3], |a, b| a - b),
            &mut numeric,
        )
        .map_err(E::Numeric)?;
        let mut smoothing = largest * unit_fraction((3.0 - distance) * 2.0);
        if velocity_exceeds_epsilon(self.subject_velocity) {
            let direction =
                try_normalize_unsafe(self.subject_velocity, &mut numeric).map_err(E::Numeric)?;
            let along = numeric
                .dot_vector(direction, self.humanized_matrix[2])
                .map_err(E::Numeric)?;
            #[expect(
                clippy::assign_op_pattern,
                reason = "retain native82e42d9c multiplier operand order"
            )]
            {
                smoothing = (1.0 - unit_fraction(-along)) * smoothing;
            }
        }
        let wiping = self
            .subject
            .as_ref()
            .ok_or(E::MissingSubject)?
            .lock()
            .map_err(|_| E::OwnerLock)?
            .flags
            .wiping_out;
        if wiping {
            smoothing = 0.0;
        }
        self.anchor_tracker.parameters.speed_clamp =
            Some(self.dynamics_parameter("tracker_subject_max_speed", |p| {
                p.tracker_subject_max_speed
            })?);
        let acceleration = self.dynamics_parameter("tracker_subject_max_acceleration", |p| {
            p.tracker_subject_max_acceleration
        })?;
        self.anchor_tracker.parameters.acceleration_min = Some(acceleration);
        self.anchor_tracker.parameters.acceleration_max = Some(acceleration);
        self.anchor_tracker.parameters.smoothing_min = Some(smoothing);
        self.anchor_tracker.parameters.smoothing_max = Some(smoothing);
        let (target, delta) = anchor_target_and_difference(
            self.anchor_matrix.columns()[3],
            self.reference_height,
            self.anchor_tracker.output_position,
        );
        if std::env::var_os("SKATE2_TRACE_VERT").is_some() {
            eprintln!(
                "vert camera anchor: push={push} pump={pump} smoothing={smoothing} max_speed={:?} max_acceleration={acceleration} camera_distance={distance} lag={:?} subject_velocity={:?}",
                self.anchor_tracker.parameters.speed_clamp, delta.xyz, self.subject_velocity.xyz
            );
        }
        if numeric.dot_vector(delta, delta).map_err(E::Numeric)? > 100.0 {
            self.teleport(&mut numeric).map_err(E::Numeric)
        } else {
            self.anchor_tracker
                .try_update_vector(dt, target, &mut numeric)
                .map_err(E::Vector)
        }
    }

    fn calculate_push_smoothing(&mut self, dt: f32) -> Result<f32, RigDynamicsError> {
        use RigDynamicsError as E;
        self.push_time -= dt;
        {
            let subject = self
                .subject
                .as_ref()
                .ok_or(E::MissingSubject)?
                .lock()
                .map_err(|_| E::OwnerLock)?;
            if subject.flags.physically_pushing {
                if subject.flags.pushable_speed {
                    self.flags |= 4;
                }
            } else {
                self.flags &= !4;
            }
        }
        if self.flags & 4 != 0 {
            self.push_time =
                self.dynamics_parameter("push_lag_duration", |p| p.push_lag_duration)?;
        }
        if self.push_time.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            self.push_time = 0.0;
        }
        let duration = self.dynamics_parameter("push_lag_duration", |p| p.push_lag_duration)?;
        let fraction = 1.0 - self.push_time / duration;
        let curve = self.dynamics_parameter("push_lag_curve", |p| p.push_lag_curve.clone())?;
        let value = unit_fraction(curve.evaluate(fraction));
        let maximum =
            self.dynamics_parameter("push_lag_max_smoothing", |p| p.push_lag_max_smoothing)?;
        Ok(value * maximum)
    }

    fn calculate_pump_smoothing(&mut self, dt: f32) -> Result<f32, RigDynamicsError> {
        use RigDynamicsError as E;
        self.pump_time -= dt;
        let acceleration = self
            .subject
            .as_ref()
            .ok_or(E::MissingSubject)?
            .lock()
            .map_err(|_| E::OwnerLock)?
            .pumping_acceleration;
        let minimum =
            self.dynamics_parameter("pumping_min_acceleration", |p| p.pumping_min_acceleration)?;
        if acceleration > minimum {
            self.pump_time =
                self.dynamics_parameter("pump_lag_duration", |p| p.pump_lag_duration)?;
        }
        let duration = self.dynamics_parameter("pump_lag_duration", |p| p.pump_lag_duration)?;
        let fraction = 1.0 - self.pump_time / duration;
        let curve = self.dynamics_parameter("pump_lag_curve", |p| p.pump_lag_curve.clone())?;
        let value = unit_fraction(curve.evaluate(fraction));
        let maximum =
            self.dynamics_parameter("pump_lag_max_smoothing", |p| p.pump_lag_max_smoothing)?;
        Ok(value * maximum)
    }

    fn update_anchor_change(&mut self) -> Result<(), RigDynamicsError> {
        use RigDynamicsError as E;
        if self.second_flags & 0x40 == 0 {
            return Ok(());
        }
        let mut numeric = HostReckoningNumeric;
        let mut anchor = self.anchor_tracker.output_position;
        anchor.xyz[1] = self.reference_height;
        let delta = self.positioner.position.zip(anchor, |a, b| a - b);
        let direction = try_normalize_safe(delta, ZERO, &mut numeric).map_err(E::Numeric)?;
        let (pitch, heading) = euler_angles(direction, &mut numeric).map_err(E::Numeric)?;
        let distance = try_length(delta, &mut numeric).map_err(E::Numeric)?;
        let heading = -heading - self.heading_offset;
        let elevation = pitch - self.elevation_offset;
        self.distance = distance;
        self.requested_heading = heading;
        self.requested_elevation = elevation;
        let mut position = self.anchor_matrix.columns()[3];
        position.xyz[1] = self.reference_height;
        self.anchor_matrix.set_column(3, position);
        self.heading_tracker
            .try_set_angle_instant(self.requested_heading, false, &mut numeric)
            .map_err(E::Numeric)?;
        self.elevation_tracker
            .try_set_angle_instant(self.requested_elevation, false, &mut numeric)
            .map_err(E::Numeric)?;
        Ok(())
    }
}

fn acceleration_along_velocity<N: ReckoningNumeric>(
    velocity: SkeletonRootVector,
    acceleration: SkeletonRootVector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let length = try_length(velocity, numeric)?;
    let direction = if length > f32::from_bits(0x38d1_b717) {
        try_normalize_unsafe(velocity, numeric)?
    } else {
        SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0])
    };
    numeric.dot_vector(acceleration, direction)
}

fn horizontal(value: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector::known([value.xyz[0], 0.0, value.xyz[2], value.xyz[0]])
}
fn velocity_exceeds_epsilon(velocity: SkeletonRootVector) -> bool {
    velocity
        .xyz
        .iter()
        .any(|x| x.abs() > f32::from_bits(0x3400_0000))
}

fn anchor_target_and_difference(
    anchor: SkeletonRootVector,
    reference_height: f32,
    mut previous: SkeletonRootVector,
) -> (SkeletonRootVector, SkeletonRootVector) {
    let target = SkeletonRootVector::known([anchor.xyz[0], reference_height, anchor.xyz[2], 0.0]);
    previous.xyz[1] = reference_height;
    (target, target.zip(previous, |a, b| a - b))
}

impl CameraBreadcrumbs {
    fn try_set_position<
        N: ReckoningNumeric<Error = crate::reckoning_numeric::ReckoningHostNumericError>,
    >(
        &mut self,
        position: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), RigDynamicsError> {
        use RigDynamicsError as E;
        let index = usize::try_from(self.last).map_err(|_| E::BreadcrumbIndex(self.last))?;
        let old = self
            .positions
            .get(index)
            .ok_or(E::BreadcrumbIndex(self.last))?;
        let delta = position.zip(*old, |a, b| a - b);
        if numeric.dot_vector(delta, delta).map_err(E::Numeric)? > self.minimum_distance_squared {
            self.last = self.last.wrapping_add(1);
            if self.last >= self.count {
                self.last = 0;
            }
            let index = usize::try_from(self.last).map_err(|_| E::BreadcrumbIndex(self.last))?;
            *self
                .positions
                .get_mut(index)
                .ok_or(E::BreadcrumbIndex(self.last))? = position;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "camera_rig_anchor_dynamics_tests.rs"]
mod tests;
