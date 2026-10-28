//! Native `CameraMan.UpdateRig82e47e68` through the actual shot producer.
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_frame::publication_prefix::inverse_frame_dt, camera_man::CameraMan,
    camera_shot_manager::runtime::ShotUpdateError, input_cvar_runtime::InputCvarReferences,
};

#[derive(Debug)]
pub(crate) enum RigUpdateError {
    OwnerLock,
    MissingParameter(&'static str),
    Shots(ShotUpdateError),
    SubjectMatrix(crate::camera_rig::subject_matrix::RigSubjectMatrixError),
    Orientation(crate::camera_rig::orientation::RigOrientationError),
    Dynamics(crate::camera_rig::dynamics::RigDynamicsError),
    Publication(crate::camera_rig::publication_prefix::CameraPublicationError),
    Composition(crate::camera_frame::composition::FrameCompositionError),
}
impl CameraMan {
    pub(crate) fn try_update_rig_prefix(
        &mut self,
        dt: f32,
        _camera_index: i32,
        shot_editor: bool,
        retry_positioning: bool,
        cvars: &InputCvarReferences,
        obstacles: crate::camera_prediction::CameraObstacleSources<'_>,
        services: &mut crate::simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices<'_>,
    ) -> Result<(), RigUpdateError> {
        let inverse_dt = inverse_frame_dt(dt);
        let overlay = services.overlay;
        if self.rig.subject_state == 2
            && self.rig.previous_subject_state != 2
            && !self.rig.positioner.flags.colliding
        {
            let grind = {
                let owner = self
                    .parameters
                    .lock()
                    .map_err(|_| RigUpdateError::OwnerLock)?;
                let CameraParameterPublication::Fields(fields) = &owner.parameters else {
                    return Err(RigUpdateError::MissingParameter("shake_grind_magnitude"));
                };
                fields
                    .shake_grind_magnitude
                    .ok_or(RigUpdateError::MissingParameter("shake_grind_magnitude"))?
            };
            let one_shot = {
                let owner = self
                    .shake
                    .parameters
                    .lock()
                    .map_err(|_| RigUpdateError::OwnerLock)?;
                let CameraParameterPublication::Fields(fields) = &owner.parameters else {
                    return Err(RigUpdateError::MissingParameter("shake_one_shot_magnitude"));
                };
                fields
                    .shake_one_shot_magnitude
                    .ok_or(RigUpdateError::MissingParameter("shake_one_shot_magnitude"))?
            };
            let magnitude = grind * one_shot;
            if magnitude > self.shake.one_shot_effect {
                self.shake.one_shot_effect = magnitude;
            }
        }
        self.try_update_shots(dt, cvars)
            .map_err(RigUpdateError::Shots)?;
        let duration = match self.shots.current.transition_units.native_value() {
            1 => self.shots.current.transition_time * f32::from_bits(0x3c88_8889),
            2 => self.shots.shot_distance * 0.5,
            _ => self.shots.current.transition_time,
        };
        let transitioning = self.shots.time_since_transition < duration;
        if !shot_editor {
            self.try_update_rig_subject_matrix(dt, cvars, overlay)
                .map_err(RigUpdateError::SubjectMatrix)?;
        }
        self.composer.anchor = self.rig.anchor_tracker.output_position;
        self.frame.anchor = self.rig.anchor_tracker.output_position;
        self.try_update_rig_orientation(shot_editor, transitioning, cvars)
            .map_err(RigUpdateError::Orientation)?;
        self.rig
            .try_do_update_prefix(
                dt,
                shot_editor,
                retry_positioning,
                cvars,
                obstacles,
                services,
            )
            .map_err(RigUpdateError::Dynamics)?;
        self.try_publish_opacity_and_fov(shot_editor, cvars)
            .map_err(RigUpdateError::Publication)?;
        self.try_compose_frame(dt, inverse_dt, shot_editor, cvars)
            .map_err(RigUpdateError::Composition)
    }
}
impl std::fmt::Display for RigUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("Rig update owner lock"),
            Self::MissingParameter(name) => write!(f, "Rig update parameter {name} unproduced"),
            Self::Shots(error) => write!(f, "{error}"),
            Self::SubjectMatrix(error) => write!(f, "{error}"),
            Self::Orientation(error) => write!(f, "{error}"),
            Self::Dynamics(error) => write!(f, "{error}"),
            Self::Publication(error) => write!(f, "{error}"),
            Self::Composition(error) => write!(f, "{error}"),
        }
    }
}
