//! `Iris::Rig::DoFramingUpdate82e42ee0` after the composed target quaternion.
use super::CameraRig;
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_compass::runtime::math::{camera_angle, euler_angles},
    camera_frame::composition_math::framing_basis,
    camera_parameters::CameraParameters,
    camera_tracker::{CameraTracker, angle_update::AngleTrackerError},
    input_cvar_runtime::{
        InputCvarError, InputCvarReferences, camera_framing::CameraFramingBoolean as C,
    },
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, try_normalize_angle},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, sine::try_sine, try_cosine},
    simulation_drive_jacobian::drive_basis_host,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
use std::sync::Arc;

const RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
const NINE_TENTHS: f32 = f32::from_bits(0x3f66_6666);

#[derive(Debug)]
pub(crate) enum RigFramingError {
    OwnerLock,
    UnknownTargetQuaternionW,
    MissingParameter(&'static str),
    Cvar(InputCvarError),
    Numeric(ReckoningHostNumericError),
    Angle(AngleTrackerError<ReckoningHostNumericError>),
}
impl std::fmt::Display for RigFramingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("Rig framing parameter owner lock"),
            Self::UnknownTargetQuaternionW => f.write_str("Rig target quaternion W unproduced"),
            Self::MissingParameter(name) => write!(f, "Rig framing parameter {name} unproduced"),
            Self::Cvar(error) => write!(f, "{error}"),
            Self::Numeric(error) => write!(f, "{error}"),
            Self::Angle(error) => write!(f, "{error}"),
        }
    }
}

fn required(value: Option<f32>, name: &'static str) -> Result<f32, RigFramingError> {
    value.ok_or(RigFramingError::MissingParameter(name))
}
fn fsel_min(current: f32, minimum: f32) -> f32 {
    if minimum - current >= 0.0 {
        current
    } else {
        minimum
    }
}
fn cap_smoothing(value: f32) -> f32 {
    if value >= 1.0 {
        1.0
    } else if NINE_TENTHS - value >= 0.0 {
        value
    } else {
        NINE_TENTHS
    }
}

/// `Rig::EvaluateUmbra82e3cc58`: both FCMPU/BLT gates require ordered less-than.
fn evaluate_umbra(
    value: f32,
    umbra: f32,
    penumbra: f32,
    numeric: &mut HostReckoningNumeric,
) -> Result<f32, RigFramingError> {
    if value < umbra {
        return Ok(1.0);
    }
    if value < penumbra {
        let numerator = camera_angle(value - umbra, numeric).map_err(RigFramingError::Numeric)?;
        let denominator =
            camera_angle(penumbra - umbra, numeric).map_err(RigFramingError::Numeric)?;
        return Ok(1.0 - numerator / denominator);
    }
    Ok(0.0)
}

trait FramingCvarSource {
    fn read_framing_boolean(&self, kind: C) -> Result<bool, InputCvarError>;
}
impl FramingCvarSource for InputCvarReferences {
    fn read_framing_boolean(&self, kind: C) -> Result<bool, InputCvarError> {
        self.try_camera_framing_boolean(kind)
    }
}

impl CameraRig {
    /// Reference: `Iris::Rig::DoFramingUpdate82e42ee0`.
    pub(crate) fn try_do_framing_update(
        &mut self,
        dt: f32,
        shot_editor: bool,
        cvars: &InputCvarReferences,
    ) -> Result<(), RigFramingError> {
        use RigFramingError as E;
        let mut numeric = HostReckoningNumeric;
        let parameters = Arc::clone(&self.parameters);
        let owner = parameters.lock().map_err(|_| E::OwnerLock)?;
        let CameraParameterPublication::Fields(p) = &owner.parameters else {
            return Err(E::MissingParameter("GameCameraParameters"));
        };
        let target = self
            .target_quaternion
            .known_lanes()
            .ok_or(E::UnknownTargetQuaternionW)?;
        let basis = drive_basis_host(target);
        let (raw_pitch, yaw) =
            euler_angles(Vector::known(basis[2]), &mut numeric).map_err(E::Numeric)?;
        // Raw 82e42fe0/e4/e8: Euler X splat, sign-XOR, insert X only.
        let pitch = f32::from_bits(raw_pitch.to_bits() ^ 0x8000_0000);
        let normalized_yaw = try_normalize_angle(yaw, &mut numeric).map_err(E::Numeric)?;
        let yaw_delta = try_normalize_angle(
            normalized_yaw - self.yaw_tracker.output_position,
            &mut numeric,
        )
        .map_err(E::Numeric)?;
        let zero = try_normalize_angle(0.0, &mut numeric).map_err(E::Numeric)?;
        let abs_yaw = if !(yaw_delta >= zero) {
            let neg = try_normalize_angle(-yaw_delta, &mut numeric).map_err(E::Numeric)?;
            try_normalize_angle(neg, &mut numeric).map_err(E::Numeric)?
        } else {
            yaw_delta
        };
        let umbra = required(p.pan_umbra, "pan_umbra")? * RADIANS;
        let penumbra = required(p.pan_penumbra, "pan_penumbra")? * RADIANS;
        let umbra_weight = evaluate_umbra(abs_yaw, umbra, penumbra, &mut numeric)?;
        let curve = p
            .pan_smoothing_curve
            .as_ref()
            .ok_or(E::MissingParameter("pan_smoothing_curve"))?;
        let curve_value = curve.evaluate(umbra_weight);
        let current_pan = self.pan_smoothing;
        let min = required(p.pan_min_smoothing, "pan_min_smoothing")?;
        let complement = 1.0 - min;
        let floor = -complement.mul_add(complement, -1.0);
        // Native BLT skips the curve; unordered inputs follow the curve path.
        let pan = if !(current_pan < floor) {
            (current_pan - floor).mul_add(curve_value, floor)
        } else {
            current_pan
        };
        let pan = cap_smoothing(pan);
        self.tilt_smoothing = cap_smoothing(self.tilt_smoothing);

        if self.flags & 8 != 0 || !(self.initialization_time <= 0.0) {
            self.pitch_tracker
                .try_set_angle_instant(pitch, true, &mut numeric)
                .map_err(E::Numeric)?;
            self.yaw_tracker
                .try_set_angle_instant(yaw, true, &mut numeric)
                .map_err(E::Numeric)?;
            self.flags &= !8;
        }
        if self.flags & 0x40 != 0 {
            update_pitch_tracker(
                &mut self.pitch_tracker,
                dt,
                pitch,
                self.tilt_smoothing,
                p,
                cvars,
                &mut numeric,
            )?;
        }
        update_yaw_tracker(&mut self.yaw_tracker, dt, yaw, pan, p, cvars, &mut numeric)?;

        let (matrix_pitch, matrix_yaw) = select_matrix_angles(
            shot_editor,
            pitch,
            yaw,
            self.pitch_tracker.output_position,
            self.yaw_tracker.output_position,
        );
        // Reuses all three native SinCos calls, including Z=+0, and W=X lanes.
        let matrix = framing_basis(matrix_pitch, matrix_yaw, &mut HostReckoningSinCosNumeric)
            .unwrap_or_else(|never| match never {});
        self.humanized_matrix = rotate_basis_about_z(matrix, self.roll * self.framing_mirror);
        Ok(())
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "source tracker inputs and retained services"
)]
fn update_pitch_tracker<S: FramingCvarSource>(
    tracker: &mut CameraTracker<f32>,
    dt: f32,
    target_pitch: f32,
    tilt_smoothing: f32,
    p: &CameraParameters,
    cvars: &S,
    numeric: &mut HostReckoningNumeric,
) -> Result<(), RigFramingError> {
    use RigFramingError as E;
    // Each cvar read is at its native use site; Acc and Smoothing read twice.
    tracker.parameters.speed_clamp = Some(
        if cvars.read_framing_boolean(C::PitchSpeed).map_err(E::Cvar)? {
            required(p.tilt_speed_clamp, "tilt_speed_clamp")? * RADIANS
        } else {
            f32::MAX
        },
    );
    tracker.parameters.acceleration_min = Some(
        if cvars
            .read_framing_boolean(C::PitchAcceleration)
            .map_err(E::Cvar)?
        {
            required(p.tilt_acceleration_clamp_min, "tilt_acceleration_clamp_min")? * RADIANS
        } else {
            f32::MAX
        },
    );
    tracker.parameters.acceleration_max = Some(
        if cvars
            .read_framing_boolean(C::PitchAcceleration)
            .map_err(E::Cvar)?
        {
            required(p.tilt_acceleration_clamp_max, "tilt_acceleration_clamp_max")? * RADIANS
        } else {
            f32::MAX
        },
    );
    if tilt_smoothing >= 1.0 {
        tracker.parameters.smoothing_min = Some(1.0);
        tracker.parameters.smoothing_max = Some(1.0);
    } else {
        tracker.parameters.smoothing_min = Some(
            if cvars
                .read_framing_boolean(C::PitchSmoothing)
                .map_err(E::Cvar)?
            {
                fsel_min(
                    tilt_smoothing,
                    required(p.tilt_smoothing_min, "tilt_smoothing_min")?,
                )
            } else {
                0.0
            },
        );
        tracker.parameters.smoothing_max = Some(
            if cvars
                .read_framing_boolean(C::PitchSmoothing)
                .map_err(E::Cvar)?
            {
                tilt_smoothing
            } else {
                0.0
            },
        );
    }
    tracker.parameters.umbra = Some(required(p.tilt_delta_umbra, "tilt_delta_umbra")? * RADIANS);
    tracker.parameters.penumbra =
        Some(required(p.tilt_delta_penumbra, "tilt_delta_penumbra")? * RADIANS);
    let target = try_normalize_angle(target_pitch, numeric).map_err(E::Numeric)?;
    tracker
        .try_update_angle(dt, target, numeric)
        .map_err(E::Angle)
}

#[expect(
    clippy::too_many_arguments,
    reason = "source tracker inputs and retained services"
)]
fn update_yaw_tracker<S: FramingCvarSource>(
    tracker: &mut CameraTracker<f32>,
    dt: f32,
    target_yaw: f32,
    pan: f32,
    p: &CameraParameters,
    cvars: &S,
    numeric: &mut HostReckoningNumeric,
) -> Result<(), RigFramingError> {
    use RigFramingError as E;
    tracker.parameters.speed_clamp = Some(
        if cvars.read_framing_boolean(C::YawSpeed).map_err(E::Cvar)? {
            required(p.pan_speed_clamp, "pan_speed_clamp")? * RADIANS
        } else {
            f32::MAX
        },
    );
    tracker.parameters.acceleration_min = Some(
        if cvars
            .read_framing_boolean(C::YawAcceleration)
            .map_err(E::Cvar)?
        {
            required(p.pan_acceleration_clamp_min, "pan_acceleration_clamp_min")? * RADIANS
        } else {
            f32::MAX
        },
    );
    tracker.parameters.acceleration_max = Some(
        if cvars
            .read_framing_boolean(C::YawAcceleration)
            .map_err(E::Cvar)?
        {
            required(p.pan_acceleration_clamp_max, "pan_acceleration_clamp_max")? * RADIANS
        } else {
            f32::MAX
        },
    );
    if pan >= 1.0 {
        tracker.parameters.smoothing_min = Some(1.0);
        tracker.parameters.smoothing_max = Some(1.0);
    } else {
        tracker.parameters.smoothing_min = Some(
            if cvars
                .read_framing_boolean(C::YawSmoothing)
                .map_err(E::Cvar)?
            {
                fsel_min(pan, required(p.pan_smoothing_min, "pan_smoothing_min")?)
            } else {
                0.0
            },
        );
        tracker.parameters.smoothing_max = Some(
            if cvars
                .read_framing_boolean(C::YawSmoothing)
                .map_err(E::Cvar)?
            {
                pan
            } else {
                0.0
            },
        );
    }
    tracker.parameters.umbra = Some(required(p.pan_delta_umbra, "pan_delta_umbra")? * RADIANS);
    tracker.parameters.penumbra =
        Some(required(p.pan_delta_penumbra, "pan_delta_penumbra")? * RADIANS);
    let target = try_normalize_angle(target_yaw, numeric).map_err(E::Numeric)?;
    tracker
        .try_update_angle(dt, target, numeric)
        .map_err(E::Angle)
}

fn select_matrix_angles(
    shot_editor: bool,
    target_pitch: f32,
    target_yaw: f32,
    tracked_pitch: f32,
    tracked_yaw: f32,
) -> (f32, f32) {
    if shot_editor {
        (target_pitch, target_yaw)
    } else {
        (tracked_pitch, tracked_yaw)
    }
}

/// `Math::RotateAroundArbitraryAxis82647348` about the matrix's own Z axis.
fn rotate_basis_about_z(matrix: [[f32; 4]; 3], angle: f32) -> [Vector; 3] {
    rotate_basis_about_axis(matrix, matrix[2], angle).map(Vector::known)
}

/// `Math::RotateAroundArbitraryAxis82647348`: general matrix input and axis.
/// Same cosine-then-sine, X multiply/Y FMA/Z FMA graph as the existing
/// identity-input specialization in `skeleton_normal_displacement.rs`; each
/// row's W is its X.
pub(crate) fn rotate_basis_about_axis(
    matrix: [[f32; 4]; 3],
    axis: [f32; 4],
    angle: f32,
) -> [[f32; 4]; 3] {
    let mut trig = HostReckoningSinCosNumeric;
    let cosine = try_cosine(angle, &mut trig).unwrap_or_else(|never| match never {});
    let sine = try_sine(angle, &mut trig).unwrap_or_else(|never| match never {});
    let [x, y, z, _] = axis;
    let complement = 1.0 - cosine;
    let wx = complement * x;
    let wy = complement * y;
    let wz = complement * z;
    let rotation = [
        [
            wx.mul_add(x, cosine),
            wx.mul_add(y, sine * z),
            wx * z - sine * y,
        ],
        [
            wy * x - sine * z,
            wy.mul_add(y, cosine),
            wy.mul_add(z, sine * x),
        ],
        [
            wz.mul_add(x, sine * y),
            wz * y - sine * x,
            wz.mul_add(z, cosine),
        ],
    ];
    std::array::from_fn(|column| {
        let input = matrix[column];
        let xyz: [f32; 3] = std::array::from_fn(|lane| {
            let first = rotation[0][lane] * input[0];
            let second = rotation[1][lane].mul_add(input[1], first);
            rotation[2][lane].mul_add(input[2], second)
        });
        [xyz[0], xyz[1], xyz[2], xyz[0]]
    })
}

#[cfg(test)]
#[path = "camera_rig_framing_update_tests.rs"]
mod tests;
