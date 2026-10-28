//! Actual `Rig.DoUpdate82e47c40` prefix through the two angle trackers.
use super::{CameraRig, speed::SubjectSpeedError};
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_compass::runtime::math::euler_angles,
    camera_parameters::CameraParameters,
    camera_prediction::{CameraObstacleSources, CameraPathEvaluatorError, CameraPathInput},
    camera_tracker::angle_update::AngleTrackerError,
    debug_overlay::DebugOverlayError,
    input_cvar_runtime::{
        InputCvarError, InputCvarReferences, camera_dynamics::CameraDynamicsBoolean,
    },
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric,
        normalize_from_squared_and_estimate, try_normalize_angle,
    },
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Debug)]
pub(crate) enum RigDynamicsError {
    OwnerLock,
    MissingSubject,
    MissingParameter(&'static str),
    Numeric(ReckoningHostNumericError),
    Speed(SubjectSpeedError<ReckoningHostNumericError>),
    Angle(AngleTrackerError<ReckoningHostNumericError>),
    Cvar(InputCvarError),
    Evaluator(CameraPathEvaluatorError),
    Overlay(DebugOverlayError),
    PendingAvoidanceQueries,
    Vector(crate::camera_tracker::vector_update::VectorTrackerError<ReckoningHostNumericError>),
    BreadcrumbIndex(u32),
    Positioner(crate::camera_positioner::update::PositionerError),
    Orientation(crate::camera_rig::orientation::RigOrientationError),
    SubjectReference(usize),
}
impl std::fmt::Display for RigDynamicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("Rig dynamics owner lock"),
            Self::MissingSubject => f.write_str("Rig dynamics Subject missing"),
            Self::MissingParameter(name) => write!(f, "Rig dynamics parameter {name} unproduced"),
            Self::Numeric(error) => write!(f, "{error}"),
            Self::Speed(error) => write!(f, "{error}"),
            Self::Angle(error) => write!(f, "{error}"),
            Self::Cvar(error) => write!(f, "{error}"),
            Self::Evaluator(error) => write!(f, "{error}"),
            Self::Overlay(error) => write!(f, "{error}"),
            Self::PendingAvoidanceQueries => f.write_str(
                "Rig avoidance requires native camera cvars, retained predictor queries and obstacle server",
            ),
            Self::Vector(error) => write!(f, "{error}"),
            Self::BreadcrumbIndex(index) => write!(f, "invalid native breadcrumb index {index}"),
            Self::Positioner(error) => write!(f, "{error}"),
            Self::Orientation(error) => write!(f, "{error}"),
            Self::SubjectReference(index) => write!(f, "camera Subject reference {index} absent"),
        }
    }
}
impl CameraRig {
    pub(crate) fn try_do_update_prefix(
        &mut self,
        dt: f32,
        _shot_editor: bool,
        retry_positioning: bool,
        cvars: &InputCvarReferences,
        obstacles: CameraObstacleSources<'_>,
        services: &mut crate::simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices<'_>,
    ) -> Result<(), RigDynamicsError> {
        use RigDynamicsError as E;
        let overlay = services.overlay;
        self.update_counter = self.update_counter.wrapping_add(1);
        overlay.require().map_err(E::Overlay)?;
        if cvars
            .try_camera_dynamics_boolean(CameraDynamicsBoolean::ShowTrajectory)
            .map_err(E::Cvar)?
        {
            let subject = self
                .subject
                .as_ref()
                .ok_or(E::MissingSubject)?
                .lock()
                .map_err(|_| E::OwnerLock)?;
            if subject.flags.trajectory_valid {
                // These native virtual getters return references; the stripped debug path
                // does not dereference their payload or submit drawing work.
                let _start = &subject.trajectory_start_position;
                let _apex = &subject.trajectory_apex_position;
                let _end = &subject.trajectory_end_position;
            }
        }
        if self.initialization_time > 0.0 {
            self.initialization_time -= dt;
        }
        if self.collision_avoidance_off_time > 0.0 {
            self.collision_avoidance_off_time -= dt;
        }
        if retry_positioning {
            self.update_angle_trackers_instant(&mut HostReckoningNumeric)
                .map_err(E::Numeric)?;
        } else {
            self.update_subject_state()?;
            self.update_avoidance_prefix(dt, cvars, obstacles, services)?;
            self.update_reference_height(dt);
            self.update_interpolated_heading(dt, cvars)?;
            self.update_interpolated_elevation(dt, cvars)?;
        }
        self.try_update_anchor_dynamics(dt)?;
        self.try_update_positioner(dt, retry_positioning, cvars, services)?;
        self.try_debug_tail(cvars, services.overlay)
    }

    fn update_subject_state(&mut self) -> Result<(), RigDynamicsError> {
        use RigDynamicsError as E;
        let subject = self
            .subject
            .as_ref()
            .ok_or(E::MissingSubject)?
            .lock()
            .map_err(|_| E::OwnerLock)?;
        let state = if subject.flags.trajectory_valid {
            Some(1)
        } else if subject.flags.grinding {
            Some(2)
        } else if subject.flags.ground_physics {
            Some(0)
        } else {
            None
        };
        if let Some(state) = state {
            if state != self.subject_state {
                self.previous_subject_state = self.subject_state;
                self.subject_state = state;
                self.subject_state_time = 0.0;
            }
        }
        Ok(())
    }

    /// `Rig::GetReferencePointVelocity82e3cf30`: copy the retained subject
    /// velocity, replacing only Y while the height blend is in progress.
    fn reference_point_velocity(&self) -> SkeletonRootVector {
        let mut velocity = self.subject_velocity;
        if self.blend_height_elapsed < self.blend_height_duration {
            velocity.xyz[1] =
                (self.blend_to_height - self.blend_from_height) / self.blend_height_duration;
        }
        velocity
    }

    /// `Rig::UpdateCollisionAvoidance82e46490` through the first retained
    /// PathEvaluator update. Later tuck-in evaluators and avoidance strategy
    /// remain explicit until their source-backed producers are connected.
    fn update_avoidance_prefix(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
        obstacles: CameraObstacleSources<'_>,
        services: &mut crate::simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices<'_>,
    ) -> Result<(), RigDynamicsError> {
        if self.avoidance_type == 0 {
            return Ok(());
        }
        if self.avoidance_type == 1
            && self
                .try_subject_speed(&mut HostReckoningNumeric)
                .map_err(RigDynamicsError::Speed)?
                < 0.5
        {
            return Ok(());
        }
        if self.flags & 2 != 0 || self.collision_avoidance_off_time > 0.0 {
            self.avoidance_heading_amount = 0.0;
            self.avoidance_elevation_amount = 0.0;
            return Ok(());
        }
        // Source samples the lookahead before selecting the avoidance direction.
        let lookahead =
            self.dynamics_parameter("avoidance_lookahead", |p| p.avoidance_lookahead)?;
        match self.avoidance_type {
            1 => {
                let last = self.breadcrumbs.last;
                let count = self.breadcrumbs.count;
                if count == 0 || last >= count {
                    return Err(RigDynamicsError::BreadcrumbIndex(last));
                }
                let index = (last + count - 1) % count;
                let breadcrumb = *self
                    .breadcrumbs
                    .positions
                    .get(index as usize)
                    .ok_or(RigDynamicsError::BreadcrumbIndex(index))?;
                let Some((heading, elevation)) = subject_path_angles(
                    breadcrumb,
                    self.anchor_tracker.output_position,
                    self.reference_height,
                )?
                else {
                    return Ok(());
                };
                self.avoidance_heading = heading;
                self.avoidance_elevation = elevation;
            }
            2 => {
                let normal = self
                    .subject
                    .as_ref()
                    .ok_or(RigDynamicsError::MissingSubject)?
                    .lock()
                    .map_err(|_| RigDynamicsError::OwnerLock)?
                    .last_valid_ground_normal;
                let (elevation, heading) = euler_angles(normal, &mut HostReckoningNumeric)
                    .map_err(RigDynamicsError::Numeric)?;
                self.avoidance_heading = f32::from_bits(heading.to_bits() ^ 0x8000_0000);
                self.avoidance_elevation = elevation;
            }
            3 => {
                self.avoidance_heading = self.pre_avoidance_heading;
                self.avoidance_elevation = f32::from_bits(0x3fbd_e44e);
            }
            4 => {
                self.avoidance_heading = try_normalize_angle(
                    self.requested_heading + f32::from_bits(0x4049_0fdb),
                    &mut HostReckoningNumeric,
                )
                .map_err(RigDynamicsError::Numeric)?;
                let minimum = f32::from_bits(0x3e32_b8c2);
                self.avoidance_elevation = if minimum - self.requested_elevation >= 0.0 {
                    minimum
                } else {
                    self.requested_elevation
                };
            }
            _ => {}
        }
        // Raw VX128 bytes at 82e46780 compare the selected elevation splat
        // against itself. The following branch returns only for NaN.
        if !source_elevation_self_equal(self.avoidance_elevation) {
            return Ok(());
        }
        // Native samples the desired query on grounded counter multiples of
        // ten, or on every pass while interleaving is enabled. The default
        // live CVar is true, but the short-circuit read order remains visible.
        let in_air = self.subject_state == 1;
        let desired_position = self.desired_camera_position(
            self.avoidance_elevation,
            self.avoidance_heading,
            self.distance,
        );
        if (!in_air && self.update_counter % 10 == 0)
            || cvars
                .try_interleave_avoidance()
                .map_err(RigDynamicsError::Cvar)?
        {
            let query_in_air = in_air && self.blend_height_elapsed >= self.blend_height_duration;
            let debug = cvars
                .try_show_avoidance_debug()
                .map_err(RigDynamicsError::Cvar)?;
            let offset = self.dynamics_parameter("positioner_avoidance_collision_offset", |p| {
                p.positioner_avoidance_collision_offset
            })?;
            self.evaluators[0].input = Some(CameraPathInput {
                position: desired_position,
                velocity: self.reference_point_velocity(),
                radius: self.positioner.collision_radius + offset,
                lookahead,
                debug,
                in_air: query_in_air,
            });
            self.evaluators[0]
                .try_update_live(services, obstacles)
                .map_err(RigDynamicsError::Evaluator)?;
        }
        let desired = self.evaluators[0].last_valid_collision_time;
        if (!in_air && self.update_counter % 10 == 5)
            || cvars
                .try_interleave_avoidance()
                .map_err(RigDynamicsError::Cvar)?
        {
            // Rig.cpp 82e46910..3c: air uses the desired camera point;
            // ground takes anchor X/Z and the positioner's camera Y.
            let heading_position = if in_air {
                desired_position
            } else {
                let mut point = self.anchor_matrix.columns()[3];
                point.xyz[1] = self.positioner.position.xyz[1];
                // `vperm128 <0,5,0,0>` leaves W as the anchor's X lane;
                // `vrlimi128 ...,2,0` replaces only Z.
                point.w = Some(point.xyz[0]);
                point
            };
            let query_in_air = in_air && self.blend_height_elapsed >= self.blend_height_duration;
            let debug = cvars
                .try_show_avoidance_debug()
                .map_err(RigDynamicsError::Cvar)?;
            let velocity = self.reference_point_velocity();
            self.evaluators[1].input = Some(CameraPathInput {
                position: heading_position,
                velocity,
                radius: f32::from_bits(0x3ca3_d70a),
                lookahead,
                debug,
                in_air: query_in_air,
            });
            self.evaluators[1]
                .try_update_live(services, obstacles)
                .map_err(RigDynamicsError::Evaluator)?;
            let hips = self
                .subject
                .as_ref()
                .ok_or(RigDynamicsError::MissingSubject)?
                .lock()
                .map_err(|_| RigDynamicsError::OwnerLock)?
                .hips_position;
            self.evaluators[2].input = Some(CameraPathInput {
                position: hips,
                velocity,
                radius: f32::from_bits(0x3ca3_d70a),
                lookahead,
                debug,
                in_air: query_in_air,
            });
            self.evaluators[2]
                .try_update_live(services, obstacles)
                .map_err(RigDynamicsError::Evaluator)?;
        }
        let heading = &self.evaluators[1];
        let elevation = &self.evaluators[2];
        let tucked_valid = heading.valid || elevation.valid;
        // PPC `fsel f30, heading-elevation, heading, elevation`: this is
        // the larger tuck clearance, with the elevation operand on NaN.
        let tucked_time = camera_fsel(
            heading.last_valid_collision_time - elevation.last_valid_collision_time,
            heading.last_valid_collision_time,
            elevation.last_valid_collision_time,
        );
        // `fcmpu` / `blt` at 82e46a6c..74 skips this only for ordered LT.
        if !(elevation.last_valid_collision_time < heading.last_valid_collision_time) {
            self.flags |= 1;
        }
        let (offboard, wiping) = {
            let subject = self
                .subject
                .as_ref()
                .ok_or(RigDynamicsError::MissingSubject)?
                .lock()
                .map_err(|_| RigDynamicsError::OwnerLock)?;
            (subject.flags.offboard, subject.flags.wiping_out)
        };
        if !offboard && self.positioner.flags.request_avoidance {
            self.avoidance_strategy = 2;
            self.avoidance_heading_amount += dt * 0.5;
            self.avoidance_elevation_amount += dt * 0.5;
        } else {
            let should_tuck = !offboard
                && !in_air
                && cvars
                    .try_enable_collision_avoidance()
                    .map_err(RigDynamicsError::Cvar)?
                && if tucked_valid {
                    tucked_time * 0.5 > desired
                } else {
                    // `ble` at 82e46b2c takes the unordered branch to AS_NONE.
                    tucked_time > desired
                };
            if should_tuck {
                self.avoidance_strategy = 2;
                let amount = 1.0 - desired / lookahead;
                let amount = camera_fsel(-amount, 0.0, amount);
                let amount = camera_fsel(1.0 - amount, amount, 1.0);
                let heading_target = camera_fsel(
                    self.avoidance_heading_amount - amount,
                    self.avoidance_heading_amount,
                    amount,
                );
                let elevation_target = camera_fsel(
                    self.avoidance_elevation_amount - amount,
                    self.avoidance_elevation_amount,
                    amount,
                );
                let heading_smoothing = self
                    .dynamics_parameter("avoidance_heading_smoothing", |p| {
                        p.avoidance_heading_smoothing
                    })?;
                let elevation_smoothing = self
                    .dynamics_parameter("avoidance_elevation_smoothing", |p| {
                        p.avoidance_elevation_smoothing
                    })?;
                self.avoidance_heading_amount +=
                    (1.0 - heading_smoothing) * (heading_target - self.avoidance_heading_amount);
                self.avoidance_elevation_amount += (1.0 - elevation_smoothing)
                    * (elevation_target - self.avoidance_elevation_amount);
            } else {
                self.avoidance_strategy = 0;
                let insignificant = cvars
                    .try_insignificant_speed()
                    .map_err(RigDynamicsError::Cvar)?;
                if (self
                    .try_subject_speed(&mut HostReckoningNumeric)
                    .map_err(RigDynamicsError::Speed)?
                    > insignificant
                    && !(self.initialization_time > 0.0))
                    || wiping
                {
                    let ease =
                        self.dynamics_parameter("avoidance_ease_out", |p| p.avoidance_ease_out)?;
                    let heading = self.avoidance_heading_amount - dt / ease;
                    let elevation = self.avoidance_elevation_amount - dt / ease;
                    self.avoidance_heading_amount = camera_fsel(heading, heading, 0.0);
                    self.avoidance_elevation_amount = camera_fsel(elevation, elevation, 0.0);
                    self.flags &= !1;
                }
            }
        }
        self.avoidance_heading_amount = camera_fsel(
            1.0 - camera_fsel(
                -self.avoidance_heading_amount,
                0.0,
                self.avoidance_heading_amount,
            ),
            camera_fsel(
                -self.avoidance_heading_amount,
                0.0,
                self.avoidance_heading_amount,
            ),
            1.0,
        );
        self.avoidance_elevation_amount = camera_fsel(
            1.0 - camera_fsel(
                -self.avoidance_elevation_amount,
                0.0,
                self.avoidance_elevation_amount,
            ),
            camera_fsel(
                -self.avoidance_elevation_amount,
                0.0,
                self.avoidance_elevation_amount,
            ),
            1.0,
        );
        Ok(())
    }

    #[expect(clippy::float_cmp, reason = "native MAX_FLOAT sentinel equality")]
    fn update_reference_height(&mut self, dt: f32) {
        let old = self.reference_height;
        let flag = self.flags & 0x40 != 0;
        if flag {
            self.blend_height_duration = 0.0;
        }
        let blending = !flag && self.blend_height_elapsed < self.blend_height_duration;
        let mut target = old;
        if blending {
            let fraction = self.blend_height_elapsed / self.blend_height_duration;
            target = (self.blend_to_height - self.blend_from_height)
                .mul_add(fraction, self.blend_from_height);
        } else if self.subject_state == 0 || self.subject_state == 2 {
            self.reference_height_at_launch = old;
            target = self.anchor_matrix.columns()[3].xyz[1];
        } else if flag || old > self.anchor_matrix.columns()[3].xyz[1] {
            target = self.anchor_matrix.columns()[3].xyz[1];
        }
        self.reference_height = if old == f32::MAX || blending {
            target
        } else {
            let factor = if self.subject_state == 2 {
                f32::from_bits(0x3d4c_cccd)
            } else {
                f32::from_bits(0x3e99_999a)
            };
            (target - old).mul_add(factor, old)
        };
        self.blend_height_elapsed += dt;
    }

    pub(super) fn dynamics_parameter<T>(
        &self,
        name: &'static str,
        read: impl FnOnce(&CameraParameters) -> Option<T>,
    ) -> Result<T, RigDynamicsError> {
        let owner = self
            .parameters
            .lock()
            .map_err(|_| RigDynamicsError::OwnerLock)?;
        let CameraParameterPublication::Fields(fields) = &owner.parameters else {
            return Err(RigDynamicsError::MissingParameter(name));
        };
        read(fields).ok_or(RigDynamicsError::MissingParameter(name))
    }
}

/// `fsel` chooses the second operand for nonnegative ordered values and the
/// third for negative or unordered values. Used for Rig.cpp's native clamps.
fn camera_fsel(condition: f32, nonnegative: f32, negative_or_nan: f32) -> f32 {
    if condition >= 0.0 {
        nonnegative
    } else {
        negative_or_nan
    }
}

fn source_elevation_self_equal(value: f32) -> bool {
    !value.is_nan()
}

/// SubjectPath case at `82e46560..82e4667c`. A short or unordered delta
/// returns from the native avoidance update without changing prior angles.
fn subject_path_angles(
    breadcrumb: SkeletonRootVector,
    anchor: SkeletonRootVector,
    reference_height: f32,
) -> Result<Option<(f32, f32)>, RigDynamicsError> {
    let mut anchor_at_height = anchor;
    anchor_at_height.xyz[1] = reference_height;
    let delta = breadcrumb.zip(anchor_at_height, |a, b| a - b);
    let mut numeric = HostReckoningNumeric;
    let squared = numeric
        .dot_vector(delta, delta)
        .map_err(RigDynamicsError::Numeric)?;
    let estimate = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(RigDynamicsError::Numeric)?;
    let length = crate::reckoning_numeric::length_from_squared_and_estimate(squared, estimate);
    if !(length > f32::from_bits(0x38d1_b717)) {
        return Ok(None);
    }
    let direction = normalize_from_squared_and_estimate(delta, squared, estimate);
    let (elevation, heading) =
        euler_angles(direction, &mut numeric).map_err(RigDynamicsError::Numeric)?;
    Ok(Some((
        f32::from_bits(heading.to_bits() ^ 0x8000_0000),
        elevation,
    )))
}

#[cfg(test)]
mod avoidance_tests {
    use super::*;
    use crate::camera_subject::ZERO;

    #[test]
    fn subject_path_uses_reference_height_and_previous_breadcrumb() {
        let anchor = SkeletonRootVector::known([0.0, 100.0, 0.0, 0.0]);
        let crumb = SkeletonRootVector::known([0.0, 2.0, 1.0, 0.0]);
        let (heading, elevation) = subject_path_angles(crumb, anchor, 2.0).unwrap().unwrap();
        assert_eq!(heading.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(elevation.to_bits(), 0.0_f32.to_bits());
        assert!(subject_path_angles(ZERO, ZERO, 0.0).unwrap().is_none());
    }

    #[test]
    fn source_elevation_self_compare_rejects_nan_but_keeps_infinity() {
        assert!(!source_elevation_self_equal(f32::NAN));
        assert!(source_elevation_self_equal(f32::INFINITY));
        assert!(source_elevation_self_equal(f32::NEG_INFINITY));
    }
}
