//! Ordered `Rig.UpdateCameraPositioner82e47808`, on the same retained owners.
use super::{CameraRig, dynamics::RigDynamicsError};
use crate::{
    camera_compass::runtime::math::{camera_angle, interpolate_angle},
    camera_positioner::update::clamp_elevation,
    input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

impl CameraRig {
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered reference captures, positioner and continuity stores preserve failures"
    )]
    pub(super) fn try_update_positioner(
        &mut self,
        dt: f32,
        retry: bool,
        cvars: &InputCvarReferences,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), RigDynamicsError> {
        use RigDynamicsError as E;
        let wiping = self
            .subject
            .as_ref()
            .ok_or(E::MissingSubject)?
            .lock()
            .map_err(|_| E::OwnerLock)?
            .flags
            .wiping_out;
        let minimum = if wiping {
            self.dynamics_parameter("positioner_min_distance_during_wipeout", |p| {
                p.positioner_min_distance_during_wipeout
            })?
        } else {
            self.dynamics_parameter("positioner_min_distance", |p| p.positioner_min_distance)?
        };
        let collision = self.collision_hint != 2;
        let heavy = cvars.try_camera_heavyweight_hard_cut().map_err(E::Cvar)? || retry;
        let heading = camera_angle(
            self.heading_tracker.output_position + self.heading_offset,
            &mut HostReckoningNumeric,
        )
        .map_err(E::Numeric)?;
        let elevation =
            clamp_elevation(self.elevation_tracker.output_position + self.elevation_offset);
        let mut anchor = self.anchor_tracker.output_position;
        anchor.xyz[1] = self.reference_height;
        self.positioner.set_parameters(
            anchor,
            heading,
            elevation,
            self.distance,
            self.avoidance_heading,
            self.avoidance_elevation,
            minimum,
            collision,
            heavy,
            retry,
            self.second_flags & 0x40 != 0,
        );
        let reference = self.subject_reference(7)?;
        let root_y = self
            .subject
            .as_ref()
            .ok_or(E::MissingSubject)?
            .lock()
            .map_err(|_| E::OwnerLock)?
            .skeleton_root
            .columns()[1];
        self.positioner.collision_start_count = 0;
        let raised = root_y.zip(reference, |axis, point| {
            axis.mul_add(f32::from_bits(0x3e19_999a), point)
        });
        let reference = self.subject_reference(3)?;
        self.positioner
            .add_collision_start(reference)
            .map_err(E::Positioner)?;
        let reference = self.subject_reference(0)?;
        self.positioner
            .add_collision_start(reference)
            .map_err(E::Positioner)?;
        self.positioner
            .add_collision_start(raised)
            .map_err(E::Positioner)?;
        if self.second_flags & 0x40 != 0 || self.initialization_time > 0.0 {
            let distance = self.positioner.requested_distance;
            self.positioner.distance.acceleration = 0.0;
            self.positioner.distance.input_position = distance;
            self.positioner.distance.output_position = distance;
            self.positioner.distance.velocity = 0.0;
            self.second_flags &= !0x40;
        }
        self.positioner
            .try_update(dt, cvars, services)
            .map_err(E::Positioner)?;
        let position = self.positioner.position;
        let reference = self.subject_reference(3)?;
        let delta = position.zip(reference, |a, b| a - b);
        let squared = HostReckoningNumeric
            .dot_vector(delta, delta)
            .map_err(E::Numeric)?;
        let wiping = self
            .subject
            .as_ref()
            .ok_or(E::MissingSubject)?
            .lock()
            .map_err(|_| E::OwnerLock)?
            .flags
            .wiping_out;
        if wiping && squared < 2.25 {
            self.avoidance_heading_amount = 1.0;
            self.avoidance_elevation_amount = 1.0;
        }
        if !self.positioner.flags.heading_elevation_continuous {
            self.avoidance_elevation_amount = 1.0;
            self.avoidance_heading_amount = 1.0;
            let heading = self.positioner.heading;
            let elevation = self.positioner.elevation;
            self.avoidance_heading = heading;
            self.avoidance_elevation = elevation;
            for index in 0..self.breadcrumbs.count {
                *self
                    .breadcrumbs
                    .positions
                    .get_mut(index as usize)
                    .ok_or(E::BreadcrumbIndex(index))? = self.positioner.position;
            }
            self.pre_avoidance_heading = heading;
            self.requested_heading = self.avoidance_heading;
            interpolate_angle(
                &mut self.requested_heading,
                self.pre_avoidance_heading,
                1.0 - self.avoidance_heading_amount,
                &mut HostReckoningNumeric,
            )
            .map_err(E::Numeric)?;
            self.try_set_requested_elevation(elevation, cvars)
                .map_err(E::Orientation)?;
            self.heading_tracker
                .try_set_angle_instant(self.requested_heading, true, &mut HostReckoningNumeric)
                .map_err(E::Numeric)?;
            self.elevation_tracker
                .try_set_angle_instant(self.requested_elevation, true, &mut HostReckoningNumeric)
                .map_err(E::Numeric)?;
            self.flags |= 8;
        }
        Ok(())
    }
    fn subject_reference(&self, index: usize) -> Result<Vector, RigDynamicsError> {
        use RigDynamicsError as E;
        let subject = self
            .subject
            .as_ref()
            .ok_or(E::MissingSubject)?
            .lock()
            .map_err(|_| E::OwnerLock)?;
        subject
            .reference_positions
            .get(index)
            .copied()
            .ok_or(E::SubjectReference(index))
    }
}
