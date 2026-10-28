//! CameraMan.Update82e48870, including its required post-rig lifecycle suffix.
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_drop::update::DropUpdateError,
    camera_man::{
        CameraMan, free_cam_update::FreeCamUpdateError, preupdate::trajectory::is_on_ground,
    },
    camera_subject::ZERO,
    camera_tracker::update::ScalarTrackerError,
    debug_overlay::DebugOverlayError,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric},
    simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices,
};

#[derive(Debug)]
pub(crate) enum CameraManUpdateError {
    OwnerLock,
    MissingSubject,
    MissingParameter(&'static str),
    Tracker(ScalarTrackerError),
    Numeric(ReckoningHostNumericError),
    Drop(DropUpdateError),
    FreeCam(FreeCamUpdateError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    PendingTurningDebug,
    Rig(crate::camera_rig::update::RigUpdateError),
}
impl std::fmt::Display for CameraManUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera update owner lock"),
            Self::MissingSubject => f.write_str("camera update Subject absent"),
            Self::MissingParameter(name) => {
                write!(f, "camera update parameter {name} is unproduced")
            }
            Self::Tracker(error) => write!(f, "camera update tracker: {error}"),
            Self::Numeric(error) => write!(f, "camera update numeric: {error}"),
            Self::Drop(error) => write!(f, "{error}"),
            Self::FreeCam(error) => write!(f, "{error}"),
            Self::Cvar(error) => write!(f, "camera update cvar: {error}"),
            Self::Overlay(error) => write!(f, "camera update overlay: {error}"),
            Self::PendingTurningDebug => f.write_str("camera turning debug getters pending"),
            Self::Rig(error) => write!(f, "{error}"),
        }
    }
}

impl CameraMan {
    /// Original update order, including the post-rig teleport/ground/readiness stores.
    pub(crate) fn try_update_prefix(
        &mut self,
        dt: f32,
        camera_index: i32,
        shot_editor: bool,
        cvars: &InputCvarReferences,
        obstacles: crate::camera_prediction::CameraObstacleSources<'_>,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), CameraManUpdateError> {
        use CameraManUpdateError as E;
        let profile = std::env::var_os("SKATE2_PROFILE_REDRAW").map(|_| std::time::Instant::now());
        let mut drop_time = std::time::Duration::ZERO;
        let second = self.second_flags & !0x40;
        let counter = self.update_counter.wrapping_add(1);
        self.flags &= !0x40;
        self.second_flags = second;
        self.second_flags_known |= 0x40;
        self.update_counter = counter;
        // Current shot is an actual embedded object, so the native null-member
        // address test is known false without manufacturing a separate owner.
        if !shot_editor {
            self.update_mirroring(dt)?;
            if camera_index == 0 {
                let drop_start = profile.map(|_| std::time::Instant::now());
                self.update_drop(dt, cvars, services)?;
                drop_time = drop_start.map(|start| start.elapsed()).unwrap_or_default();
            }
            let stick = if self.shots.current.use_free_stick {
                self.subject
                    .as_ref()
                    .ok_or(E::MissingSubject)?
                    .lock()
                    .map_err(|_| E::OwnerLock)?
                    .look_stick
            } else {
                ZERO
            };
            self.free_cam.try_update(stick).map_err(E::FreeCam)?;
            self.rig.flags = (self.rig.flags & !0x40)
                | (u8::from(self.shots.interpolated.follow_subject_in_air) << 6);
            if cvars.try_camera_show_mirroring().map_err(E::Cvar)? {
                services.overlay.require().map_err(E::Overlay)?;
            }
            if cvars.try_camera_show_turning().map_err(E::Cvar)? {
                services.overlay.require().map_err(E::Overlay)?;
                return Err(E::PendingTurningDebug);
            }
        }
        let remaining = self.heading_follows_model_time - dt;
        self.heading_follows_model_time = if -remaining >= 0.0 { 0.0 } else { remaining };
        self.flags |= u8::from(self.heading_follows_model_time > 0.0) << 6;
        // Native caller4899c passes r7=0 independently of r6/shotEditor.
        let prefix_time = profile.map(|start| start.elapsed());
        self.try_update_rig_prefix(
            dt,
            camera_index,
            shot_editor,
            false,
            cvars,
            obstacles,
            services,
        )
        .map_err(E::Rig)?;
        //82e489c0..8a74. This must follow UpdateRig: the next PreUpdate's
        // landing-height blend depends on clearing the teleport latch here.
        self.flags |= u8::from(self.rig.positioner.flags.hard_cut) << 6;
        if self.second_flags & 0x20 != 0 {
            self.teleport(&mut HostReckoningNumeric).map_err(E::Numeric)?;
        } else {
            let subject=self.subject.as_ref().ok_or(E::MissingSubject)?
                .lock().map_err(|_|E::OwnerLock)?;
            if is_on_ground(&subject,self.rig.subject_state) {
                self.second_flags &= !0x80;
                self.second_flags_known |= 0x80;
            }
        }
        self.shots.instant_transition=self.heading_follows_model_time>0.0;
        if self.update_counter>cvars.try_camera_not_ready_duration().map_err(E::Cvar)? {
            self.second_flags |= 0x40;
            self.second_flags_known |= 0x40;
        }
        self.frame.hard_cut=self.flags & 0x40 != 0;
        let parameters=self.parameters.lock().map_err(|_|E::OwnerLock)?;
        let CameraParameterPublication::Fields(fields)=&parameters.parameters else {
            return Err(E::MissingParameter("near_clipping_distance_min"));
        };
        self.frame.near_plane=fields.near_clipping_distance_min
            .ok_or(E::MissingParameter("near_clipping_distance_min"))?;
        drop(parameters);
        if let Some(start) = profile {
            let prefix = prefix_time.unwrap_or_default();
            let message = format!(
                "diagnostic camera man: drop={}ms other_prefix={}ms rig={}ms",
                drop_time.as_secs_f64() * 1000.0,
                prefix.saturating_sub(drop_time).as_secs_f64() * 1000.0,
                start.elapsed().saturating_sub(prefix).as_secs_f64() * 1000.0
            );
            eprintln!("{message}");
        }
        Ok(())
    }

    fn update_mirroring(&mut self, dt: f32) -> Result<(), CameraManUpdateError> {
        use CameraManUpdateError as E;
        let target = if self.shots.current.mirror_for_stance && self.flags & 2 != 0 {
            -1.0
        } else {
            1.0
        };
        let on_ground = if self.rig.subject_state == 2 {
            true
        } else {
            let subject = self
                .subject
                .as_ref()
                .ok_or(E::MissingSubject)?
                .lock()
                .map_err(|_| E::OwnerLock)?;
            is_on_ground(&subject, self.rig.subject_state)
        };
        if on_ground {
            self.heading_mirroring.parameters.smoothing_min = Some(f32::from_bits(0x3f59_999a));
            self.heading_mirroring.parameters.smoothing_max = Some(f32::from_bits(0x3f59_999a));
            self.heading_mirroring.parameters.acceleration_min = Some(500.0);
            self.heading_mirroring.parameters.acceleration_max = Some(500.0);
            self.heading_mirroring.parameters.speed_clamp = Some(100.0);
            self.heading_mirroring
                .try_update_scalar(dt, target)
                .map_err(E::Tracker)?;
            self.framing_mirroring.parameters.smoothing_min = Some(f32::from_bits(0x3f59_999a));
            self.framing_mirroring.parameters.smoothing_max = Some(f32::from_bits(0x3f59_999a));
            self.framing_mirroring.parameters.acceleration_min = Some(50.0);
            self.framing_mirroring.parameters.acceleration_max = Some(50.0);
            self.framing_mirroring.parameters.speed_clamp = Some(15.0);
            self.framing_mirroring
                .try_update_scalar(dt, target)
                .map_err(E::Tracker)?;
        }
        Ok(())
    }

    fn update_drop(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), CameraManUpdateError> {
        use CameraManUpdateError as E;
        let active = self.shots.current.use_drop_predictor;
        let mut velocity = self.rig.subject_velocity;
        let squared = HostReckoningNumeric
            .dot_vector(velocity, velocity)
            .map_err(E::Numeric)?;
        let minimum = {
            let attributes = self.parameters.lock().map_err(|_| E::OwnerLock)?;
            let CameraParameterPublication::Fields(fields) = &attributes.parameters else {
                return Err(E::MissingParameter("drop_min_test_distance"));
            };
            fields
                .drop_min_test_distance
                .ok_or(E::MissingParameter("drop_min_test_distance"))?
        };
        if minimum * minimum > squared {
            // Native re-reads the value before the virtual hips-look call.
            let minimum = {
                let attributes = self.parameters.lock().map_err(|_| E::OwnerLock)?;
                let CameraParameterPublication::Fields(fields) = &attributes.parameters else {
                    return Err(E::MissingParameter("drop_min_test_distance"));
                };
                fields
                    .drop_min_test_distance
                    .ok_or(E::MissingParameter("drop_min_test_distance"))?
            };
            let look = self
                .subject
                .as_ref()
                .ok_or(E::MissingSubject)?
                .lock()
                .map_err(|_| E::OwnerLock)?
                .hips_look;
            velocity = look.map(|lane| lane * minimum);
        }
        let drop_owner = std::sync::Arc::clone(&self.drop);
        let position = self
            .subject
            .as_ref()
            .ok_or(E::MissingSubject)?
            .lock()
            .map_err(|_| E::OwnerLock)?
            .board_position;
        drop_owner
            .lock()
            .map_err(|_| E::OwnerLock)?
            .try_update(position, velocity, dt, active, cvars, services)
            .map_err(E::Drop)
    }
}
