//! Native `CameraMan.UpdateRig82e47fe0..482e0` orientation and control stores.
use crate::{
    camera_compass::runtime::math::{camera_angle, euler_angles},
    camera_man::CameraMan,
    camera_rig::CameraRig,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    simulation_drive_jacobian::drive_basis_host,
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Debug)]
pub(crate) enum RigOrientationError {
    OwnerLock,
    MissingSubject,
    Numeric(ReckoningHostNumericError),
    Cvar(InputCvarError),
}
impl CameraMan {
    pub(crate) fn try_update_rig_orientation(
        &mut self,
        shot_editor: bool,
        transitioning: bool,
        cvars: &InputCvarReferences,
    ) -> Result<(), RigOrientationError> {
        use RigOrientationError as E;
        let shot = if shot_editor {
            &self.shots.current
        } else {
            &self.shots.interpolated
        };
        let basis = drive_basis_host(shot.world_arm_orientation);
        let (pitch, heading) = euler_angles(
            SkeletonRootVector::known(basis[2]),
            &mut HostReckoningNumeric,
        )
        .map_err(E::Numeric)?;
        let elevation = -pitch;
        let fakie = self
            .subject
            .as_ref()
            .ok_or(E::MissingSubject)?
            .lock()
            .map_err(|_| E::OwnerLock)?
            .flags
            .riding_fakie;
        if self.rig.avoidance_type != 0 {
            let steep = self.absolute_incline_angle > f32::from_bits(0x3f49_0fdb);
            let across = self.incline_alignment > f32::from_bits(0x3f49_0fdb)
                && self.incline_alignment < f32::from_bits(0x4016_cbe4);
            let wiping = self
                .subject
                .as_ref()
                .ok_or(E::MissingSubject)?
                .lock()
                .map_err(|_| E::OwnerLock)?
                .flags
                .wiping_out;
            self.rig.avoidance_type = if wiping {
                4
            } else if steep && across {
                2
            } else {
                1
            };
        }
        self.rig.flags = (self.rig.flags & !2) | (u8::from(shot.compass_north == 2) << 1);
        self.rig.flags = (self.rig.flags & !0x20) | (u8::from(transitioning) << 5);
        self.rig.pre_avoidance_heading = heading;
        if self.second_flags & 0x20 != 0 {
            self.rig.requested_heading = heading;
        } else {
            self.rig.requested_heading = self.rig.avoidance_heading;
            interpolate_angle(
                &mut self.rig.requested_heading,
                heading,
                1.0 - self.rig.avoidance_heading_amount,
            )?;
        }
        self.rig.second_flags = (self.rig.second_flags & !0x80) | (u8::from(fakie) << 7);
        self.rig.second_flags_known |= 0x80;
        let free = self.free_cam.elevation_offset;
        let drop_elevation = self.drop.lock().map_err(|_| E::OwnerLock)?.elevation_offset;
        let combined_elevation = drop_elevation + free;
        let drop_distance = self.drop.lock().map_err(|_| E::OwnerLock)?.distance_offset;
        self.rig.distance = shot.distance + drop_distance;
        self.rig.try_set_requested_elevation(elevation, cvars)?;
        self.rig.elevation_offset = combined_elevation;
        self.rig.second_flags =
            (self.rig.second_flags & !0x20) | (u8::from(shot.compass_north != 5) << 5);
        self.rig.second_flags_known |= 0x20;
        self.motion_blur = if self.heading_follows_model_time > 0.0 {
            0.0
        } else if transitioning {
            shot.blur_transition
        } else {
            shot.blur_shot
        };
        let heading_mirror = if shot.compass_north == 2 {
            1.0
        } else {
            self.heading_mirroring.output_position
        };
        let framing_mirror = if shot.compass_north == 2 {
            1.0
        } else {
            self.framing_mirroring.output_position
        };
        self.rig.heading_mirror = heading_mirror;
        self.rig.framing_mirror = framing_mirror;
        self.rig.collision_hint = u32::from_ne_bytes(shot.collision_hint.to_ne_bytes());
        self.rig.heading_speed = 1.0 - smoothing_complement(shot.smoothing_direction);
        self.rig.elevation_speed = 1.0 - smoothing_complement(shot.smoothing_elevation);
        self.rig.pan_smoothing = smoothing_complement(shot.smoothing_yaw);
        self.rig.tilt_smoothing = smoothing_complement(shot.smoothing_pitch);
        self.rig.roll = self.composer.last_roll;
        Ok(())
    }
}

/// Native FNMSUBS retains the fused subtraction, even for exceptional inputs.
fn smoothing_complement(smoothing: f32) -> f32 {
    let difference = 1.0 - smoothing;
    -difference.mul_add(difference, -1.0)
}

fn interpolate_angle(
    destination: &mut f32,
    target: f32,
    amount: f32,
) -> Result<(), RigOrientationError> {
    crate::camera_compass::runtime::math::interpolate_angle(
        destination,
        target,
        amount,
        &mut HostReckoningNumeric,
    )
    .map_err(RigOrientationError::Numeric)
}

impl CameraRig {
    /// `SetRequestedElevation82e3fd40`: both angle stores precede live cvar.
    pub(super) fn try_set_requested_elevation(
        &mut self,
        elevation: f32,
        cvars: &InputCvarReferences,
    ) -> Result<(), RigOrientationError> {
        use RigOrientationError as E;
        self.requested_elevation = elevation;
        self.pre_avoidance_elevation = elevation;
        if cvars
            .try_camera_avoidance_elevation_enabled()
            .map_err(E::Cvar)?
        {
            let trajectory = self
                .subject
                .as_ref()
                .ok_or(E::MissingSubject)?
                .lock()
                .map_err(|_| E::OwnerLock)?
                .flags
                .trajectory_valid;
            if trajectory
                || self.positioner.flags.request_avoidance
                || self.incline.abs() > f32::from_bits(0x3f06_0a92)
            {
                self.flags |= 1;
            }
        }
        let old = self.flags;
        let using = self.positioner.flags.using_avoidance_position;
        self.flags = old | u8::from(using);
        if using || old & 1 != 0 {
            interpolate_angle(
                &mut self.requested_elevation,
                self.avoidance_elevation,
                self.avoidance_elevation_amount,
            )?;
            self.requested_elevation =
                camera_angle(self.requested_elevation, &mut HostReckoningNumeric)
                    .map_err(E::Numeric)?;
        }
        Ok(())
    }
}
impl std::fmt::Display for RigOrientationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("Rig orientation owner lock"),
            Self::MissingSubject => f.write_str("Rig orientation Subject missing"),
            Self::Numeric(error) => write!(f, "{error}"),
            Self::Cvar(error) => write!(f, "{error}"),
        }
    }
}

#[cfg(test)]
#[path = "camera_rig_orientation_tests.rs"]
mod tests;
