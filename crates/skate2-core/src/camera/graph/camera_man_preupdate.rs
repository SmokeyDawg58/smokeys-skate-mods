//! Same-owner `CameraMan::PreUpdate82e453f8` publications and lean calculation.
use crate::{
    ai::vmx::{self, V4},
    camera_attributes::CameraParameterPublication,
    camera_man::CameraMan,
    camera_rig::framing_update::rotate_basis_about_axis,
    camera_subject::CameraSubject,
    grind_data_octree::OctreeBounds,
    grind_data_runtime::GrindDataQueryResult,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{ReckoningNumeric, ReckoningVector, try_length, try_normalize_safe},
    simulation_assembly_backend::constraint_session::{
        grind_world::PhysicalGrindError, physical_world_line::camera::CameraLineServices,
    },
};

#[path = "camera_man_trajectory.rs"]
pub(crate) mod trajectory;

#[path = "camera_grind_detector.rs"]
mod grind_detector;

#[derive(Debug)]
pub(crate) enum CameraPreUpdateContinuationError<E> {
    Prefix(CameraPreUpdateError),
    Numeric(E),
    Grind(CameraGrindUpdateError),
    Trajectory(trajectory::CameraTrajectoryError<E>),
}

#[derive(Debug)]
pub(crate) enum CameraGrindUpdateError {
    OwnerLock,
    Unwritten(&'static str),
    Query(PhysicalGrindError),
}

#[derive(Debug)]
pub(crate) enum CameraPreUpdateError {
    OwnerLock,
    MissingSubject,
    UnwrittenDeadzone,
    Cvar(InputCvarError),
}

impl std::fmt::Display for CameraGrindUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera grind detector owner lock poisoned"),
            Self::Unwritten(name) => write!(f, "camera grind parameter {name} is unpublished"),
            Self::Query(error) => write!(f, "camera grind BoxTest: {error}"),
        }
    }
}

impl std::fmt::Display for CameraPreUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera PreUpdate owner lock poisoned"),
            Self::MissingSubject => f.write_str("camera PreUpdate has no Subject"),
            Self::UnwrittenDeadzone => f.write_str("camera turning deadzone is unpublished"),
            Self::Cvar(error) => write!(f, "camera PreUpdate cvar: {error:?}"),
        }
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for CameraPreUpdateContinuationError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prefix(error) => write!(f, "{error}"),
            Self::Numeric(error) => write!(f, "camera grind speed numeric boundary: {error:?}"),
            Self::Grind(error) => write!(f, "{error}"),
            Self::Trajectory(error) => write!(f, "{error}"),
        }
    }
}

/// Native sign comparison classifies unordered values as negative.
fn lean_sign(value: f32) -> i8 {
    if value > 0.0 {
        1
    } else if value >= 0.0 {
        0
    } else {
        -1
    }
}

/// `GetAnimationInputLean82e3bc00`; the caller supplies the prior mirror flag.
pub(crate) fn animation_input_lean(subject: &CameraSubject, mirrored: bool) -> f32 {
    let turn = subject.input_turn;
    let kick = subject.input_kickturn;
    let animation = subject.animation_input_lean;
    let mut chosen = if kick.abs() > turn.abs() { kick } else { turn };
    if lean_sign(chosen) != lean_sign(animation) || chosen.abs() < animation.abs() {
        chosen = animation;
    }
    if mirrored == subject.flags.riding_fakie {
        chosen
    } else {
        -chosen
    }
}

impl CameraMan {
    /// Native `PreUpdate` call chain. A pending branch never returns completion.
    pub(crate) fn try_preupdate<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        camera_index: u32,
        suppress_update: bool,
        cvars: &InputCvarReferences,
        overlay: &crate::debug_overlay::DebugOverlaySlot,
        services: &mut CameraLineServices<'_>,
        numeric: &mut N,
    ) -> Result<(), CameraPreUpdateContinuationError<N::Error>> {
        use CameraPreUpdateContinuationError as E;
        let detect = self
            .try_preupdate_prefix(dt, camera_index, suppress_update, cvars)
            .map_err(E::Prefix)?;
        if suppress_update {
            return Ok(());
        }
        if detect {
            self.try_update_grind_detection(dt, cvars, services, numeric)?;
        }
        self.try_update_trajectory(cvars, overlay, services, numeric)
            .map_err(E::Trajectory)?;
        self.try_update_incline(cvars, overlay, numeric)
            .map_err(E::Trajectory)
    }

    /// `UpdateGrindDetection82e3e8b0`: from 0.01 along the ground, the detector's frustum runs
    /// along the travel from the board `mGrindSkaterOffset` ahead, pitched down
    /// `mGrindFrustumPitchOffset` degrees about its side; then the anticipation steps against
    /// the camera's position (`GetFrame`) and the hips.
    fn try_update_grind_detection<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
        services: &CameraLineServices<'_>,
        numeric: &mut N,
    ) -> Result<(), CameraPreUpdateContinuationError<N::Error>> {
        use CameraPreUpdateContinuationError as E;
        const UP: V4 = [0.0, 1.0, 0.0, 0.0];
        let owner = self
            .subject
            .as_ref()
            .ok_or(E::Prefix(CameraPreUpdateError::MissingSubject))?;
        let subject = owner
            .lock()
            .map_err(|_| E::Prefix(CameraPreUpdateError::OwnerLock))?;
        // The board matrix is captured before the rig velocity is sampled.
        let board = subject.board_effective_transform.columns()[3].operand();
        let mut velocity = self.rig.subject_velocity;
        velocity.xyz[1] = 0.0;
        velocity.w = Some(0.0);
        let speed = try_length(velocity, numeric).map_err(E::Numeric)?;
        // blt: an unordered speed goes on.
        if speed < f32::from_bits(0x3c23_d70a) {
            return Ok(());
        }
        let forward =
            try_normalize_safe(velocity.operand(), [0.0; 4], numeric).map_err(E::Numeric)?;
        let side = vmx::cross(UP, forward);
        let (pitch_offset, skater_offset) = self.grind_frustum_offsets().map_err(E::Grind)?;
        let [x, y, z] = rotate_basis_about_axis(
            [side, UP, forward],
            side,
            pitch_offset * f32::from_bits(0xbc8e_fa35),
        );
        let position =
            std::array::from_fn(|lane| forward[lane].mul_add(skater_offset, board[lane]));
        let is_grinding = subject.flags.grinding;
        let hips = subject.hips_position.operand();
        drop(subject);
        let mut box_test = |bounds: OctreeBounds| -> Result<Vec<[V4; 2]>, CameraGrindUpdateError> {
            let mut result = GrindDataQueryResult::new();
            services
                .grinds
                .try_box_test(bounds, &mut result, false, cvars)
                .map_err(CameraGrindUpdateError::Query)?;
            Ok(result
                .results()
                .map(|primitive| [primitive.start, primitive.end])
                .collect())
        };
        let mut detector = self
            .grind
            .lock()
            .map_err(|_| E::Grind(CameraGrindUpdateError::OwnerLock))?;
        detector.try_update(
            [x, y, z, position],
            speed,
            is_grinding,
            &mut box_test,
            numeric,
        )?;
        let camera = self.frame.matrix.columns()[3].operand();
        detector.try_update_anticipation(dt, camera, hips, numeric)
    }

    /// `UpdateGrindDetection`'s own `mParameters` reads: `mGrindFrustumPitchOffset` and
    /// `mGrindSkaterOffset`.
    fn grind_frustum_offsets(&self) -> Result<(f32, f32), CameraGrindUpdateError> {
        let parameters = self
            .parameters
            .lock()
            .map_err(|_| CameraGrindUpdateError::OwnerLock)?;
        let fields = grind_detector::fields(&parameters)?;
        Ok((
            grind_detector::field(
                fields.grind_frustum_pitch_offset,
                "grind_frustum_pitch_offset",
            )?,
            grind_detector::field(fields.grind_skater_offset, "grind_skater_offset")?,
        ))
    }

    fn turning_deadzone(&self) -> Result<f32, CameraPreUpdateError> {
        let parameters = self
            .parameters
            .lock()
            .map_err(|_| CameraPreUpdateError::OwnerLock)?;
        match &parameters.parameters {
            CameraParameterPublication::Fields(fields) => fields
                .turning_centred_deadzone
                .ok_or(CameraPreUpdateError::UnwrittenDeadzone),
            CameraParameterPublication::Unwritten => Err(CameraPreUpdateError::UnwrittenDeadzone),
        }
    }

    /// Publishes the native prefix and returns whether grind detection is called.
    /// This is not a completed `PreUpdate`: trajectory and incline follow the caller.
    pub(crate) fn try_preupdate_prefix(
        &mut self,
        dt: f32,
        camera_index: u32,
        suppress_update: bool,
        cvars: &InputCvarReferences,
    ) -> Result<bool, CameraPreUpdateError> {
        let owner = self
            .subject
            .as_ref()
            .ok_or(CameraPreUpdateError::MissingSubject)?;
        let subject = owner.lock().map_err(|_| CameraPreUpdateError::OwnerLock)?;
        self.time_since_air = if subject.flags.trajectory_valid {
            0.0
        } else {
            dt + self.time_since_air
        };
        if subject.flags.teleported {
            self.second_flags |= 0x20;
            self.second_flags_known |= 0x20;
        }
        if suppress_update {
            return Ok(false);
        }
        let raw_lean = subject.animation_input_lean.abs();
        let deadzone = self.turning_deadzone()?;
        self.turning_time = if raw_lean > deadzone {
            self.turning_time + dt
        } else {
            0.0
        };
        let combined_lean = animation_input_lean(&subject, self.flags & 2 != 0).abs();
        let deadzone = self.turning_deadzone()?;
        self.turning_centred_time = if combined_lean < deadzone {
            dt + self.turning_centred_time
        } else {
            0.0
        };
        let fakie = subject.flags.riding_fakie;
        let mirrored = subject.flags.mirrored;
        self.flags = (self.flags & !2) | (u8::from(fakie != mirrored) << 1);
        let enabled = cvars
            .try_camera_grind_detector()
            .map_err(CameraPreUpdateError::Cvar)?;
        Ok(enabled && camera_index == 0)
    }
}

#[cfg(test)]
#[path = "camera_man_preupdate_tests.rs"]
mod tests;
