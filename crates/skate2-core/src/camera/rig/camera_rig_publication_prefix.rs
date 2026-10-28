//! Actual Rig debug return, followed by camera opacity and FOV publication.
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_man::CameraMan,
    camera_rig::{CameraRig, dynamics::RigDynamicsError},
    debug_overlay::DebugOverlaySlot,
    input_cvar_runtime::{InputCvarError, InputCvarReferences, camera_rig_debug::RigDebugBoolean},
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric,
        length_from_squared_and_estimate,
    },
    skeleton_query_rotation::try_atan,
};

#[derive(Debug)]
pub(crate) enum CameraPublicationError {
    OwnerLock,
    MissingParameter(&'static str),
    Cvar(InputCvarError),
    Numeric(ReckoningHostNumericError),
}
impl std::fmt::Display for CameraPublicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera publication owner lock"),
            Self::MissingParameter(name) => {
                write!(f, "camera publication parameter {name} unproduced")
            }
            Self::Cvar(error) => write!(f, "{error}"),
            Self::Numeric(error) => write!(f, "{error}"),
        }
    }
}

impl CameraRig {
    pub(super) fn try_debug_tail(
        &self,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), RigDynamicsError> {
        try_debug_gates(cvars, overlay, |kind| {
            let (elevation, heading) = match kind {
                RigDebugBoolean::Intended => (self.requested_elevation, self.requested_heading),
                RigDebugBoolean::Avoidance => (self.avoidance_elevation, self.avoidance_heading),
                RigDebugBoolean::Tracker => {
                    unreachable!("tracker debug has no position computation")
                }
            };
            // ShowPositionDebug computes this real scratch result, then returns.
            // There is no visible draw submission in this executable.
            let _scratch = self.desired_camera_position(elevation, heading, self.distance);
        })
    }
}

fn try_debug_gates(
    cvars: &InputCvarReferences,
    overlay: &DebugOverlaySlot,
    mut compute_position: impl FnMut(RigDebugBoolean),
) -> Result<(), RigDynamicsError> {
    use RigDynamicsError as E;
    for kind in [RigDebugBoolean::Intended, RigDebugBoolean::Avoidance] {
        if cvars.try_camera_rig_debug(kind).map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
            compute_position(kind);
        }
    }
    if cvars
        .try_camera_rig_debug(RigDebugBoolean::Tracker)
        .map_err(E::Cvar)?
    {
        overlay.require().map_err(E::Overlay)?;
    }
    Ok(())
}

impl CameraMan {
    pub(crate) fn try_publish_opacity_and_fov(
        &mut self,
        shot_editor: bool,
        cvars: &InputCvarReferences,
    ) -> Result<(), CameraPublicationError> {
        use CameraPublicationError as E;
        let shot = if shot_editor {
            &self.shots.current
        } else {
            &self.shots.interpolated
        };
        if cvars.try_camera_grind_detector().map_err(E::Cvar)? {
            // Actual vtable+1c getter82e35248 loads +2c shaped anticipation.
            let anticipation = self
                .grind
                .lock()
                .map_err(|_| E::OwnerLock)?
                .shaped_anticipation;
            let shot_opacity = shot.subject_opacity;
            let minimum = {
                let owner = self.parameters.lock().map_err(|_| E::OwnerLock)?;
                let CameraParameterPublication::Fields(fields) = &owner.parameters else {
                    return Err(E::MissingParameter("grind_opacity"));
                };
                fields
                    .grind_opacity
                    .ok_or(E::MissingParameter("grind_opacity"))?
            };
            self.opacity = grind_opacity(shot_opacity, anticipation, minimum);
        } else {
            self.opacity = shot.subject_opacity;
        }
        let aspect = self.aspect_ratio;
        let fov = vertical_fov(shot.lens_length, 35.0, aspect, &mut HostReckoningNumeric)
            .map_err(E::Numeric)?;
        self.vertical_field_of_view = Some(fov);
        Ok(())
    }
}

fn grind_opacity(shot: f32, anticipation: f32, minimum: f32) -> f32 {
    let remaining = 1.0 - anticipation;
    let scale = 1.0 - minimum;
    let bound = scale.mul_add(remaining, minimum);
    if shot - bound >= 0.0 { bound } else { shot }
}

/// Exact scalar prefix of `GetVerticalFOVFromLensSize82e3c1d8`, followed by
/// the same two-refinement length graph used by the rest of the camera.
fn vertical_fov<N: ReckoningNumeric>(
    lens: f32,
    medium: f32,
    aspect: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let ratio = medium / (lens * 2.0);
    let angle = try_atan(ratio, numeric)? * f32::from_bits(0x42e5_2ee1);
    let square = angle * angle;
    let divisor = aspect + 1.0;
    let squared = square / divisor;
    let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
    Ok(length_from_squared_and_estimate(squared, estimate))
}

#[cfg(test)]
#[path = "camera_rig_publication_prefix_tests.rs"]
mod tests;
