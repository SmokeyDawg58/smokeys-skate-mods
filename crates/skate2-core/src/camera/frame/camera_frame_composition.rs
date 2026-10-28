//! Actual `FrameComposer.DoFraming82e3f298` through caller Rig+80 publication.
#[path = "../camera_shake_apply_frame0.rs"]
mod shake_apply;
#[path = "../camera_shake_subject_inputs.rs"]
mod shake_subject_inputs;
use self::shake_apply::ShakeApplyError;
use self::shake_subject_inputs::{shake_active, subject_distance, subject_speed};
use super::CameraFrameComposer;
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_man::CameraMan,
    camera_shot::CameraShot,
    camera_shot_manager::runtime_math::{self, ShotMathError},
    input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

#[derive(Debug)]
pub(crate) enum FrameCompositionError {
    OwnerLock,
    MissingSubject,
    MissingParameter(&'static str),
    ReferenceCount(usize),
    ReferenceNaN(usize),
    Numeric(ReckoningHostNumericError),
    RigFraming(crate::camera_rig::framing_update::RigFramingError),
    Shake(ShakeApplyError),
}
impl std::fmt::Display for FrameCompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera composer owner lock"),
            Self::MissingSubject => f.write_str("camera composer Subject missing"),
            Self::MissingParameter(name) => {
                write!(f, "camera composer parameter {name} unproduced")
            }
            Self::ReferenceCount(count) => write!(f, "camera reference count assertion {count}"),
            Self::ReferenceNaN(lane) => write!(f, "camera reference NaN lane {lane}"),
            Self::Numeric(error) => write!(f, "{error}"),
            Self::RigFraming(error) => write!(f, "{error}"),
            Self::Shake(error) => write!(f, "{error}"),
        }
    }
}
impl CameraFrameComposer {
    pub(super) fn parameter(&self, name: &'static str) -> Result<f32, FrameCompositionError> {
        use FrameCompositionError as E;
        let owner = self.parameters.lock().map_err(|_| E::OwnerLock)?;
        let CameraParameterPublication::Fields(fields) = &owner.parameters else {
            return Err(E::MissingParameter(name));
        };
        let value = match name {
            "max_pitch" => fields.max_pitch,
            "dutch_interpolation_speed" => fields.dutch_interpolation_speed,
            _ => unreachable!("actual composer parameter names"),
        };
        value.ok_or(E::MissingParameter(name))
    }

    /// CURRENT first, PREVIOUS second; both publish this same composer.
    #[expect(
        clippy::too_many_arguments,
        reason = "native composer inputs, sharing retained owners"
    )]
    fn compose(
        &mut self,
        dt: f32,
        previous: &CameraShot,
        current: &CameraShot,
        amount: f32,
        subject_position: Vector,
        camera_position: Vector,
        mirror: f32,
    ) -> Result<[f32; 4], FrameCompositionError> {
        let current_q = self.single(current, subject_position, camera_position, mirror)?;
        let previous_q = self.single(previous, subject_position, camera_position, mirror)?;
        let result = runtime_math::quaternion_core(
            previous_q,
            current_q,
            amount,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .map_err(math_error)?;
        self.update_roll(dt, current.roll)?;
        Ok(result)
    }

    pub(super) fn update_roll(
        &mut self,
        dt: f32,
        desired: f32,
    ) -> Result<(), FrameCompositionError> {
        self.last_roll = if dt > 0.0 {
            let difference = desired - self.last_roll;
            let speed = self.parameter("dutch_interpolation_speed")?;
            let product = difference * speed;
            product.mul_add(dt, self.last_roll)
        } else {
            desired
        };
        Ok(())
    }
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "Result::map_err passes its error by value to this mapper"
)]
pub(super) fn math_error(
    error: ShotMathError<ReckoningHostNumericError, std::convert::Infallible>,
) -> FrameCompositionError {
    match error {
        ShotMathError::Numeric(error) => FrameCompositionError::Numeric(error),
        ShotMathError::Trigonometry(never) => match never {},
    }
}

impl CameraMan {
    pub(crate) fn try_compose_frame(
        &mut self,
        dt: f32,
        inverse_dt: f32,
        shot_editor: bool,
        cvars: &InputCvarReferences,
    ) -> Result<(), FrameCompositionError> {
        let duration = match self.shots.current.transition_units.native_value() {
            1 => self.shots.current.transition_time * f32::from_bits(0x3c88_8889),
            2 => self.shots.shot_distance * 0.5,
            _ => self.shots.current.transition_time,
        };
        let amount = transition_amount(duration, self.shots.time_since_transition);
        let subject_position = self.rig.anchor_tracker.output_position;
        let camera_position = self.rig.positioner.position;
        let result = self.composer.compose(
            dt,
            &self.shots.previous,
            &self.shots.current,
            amount,
            subject_position,
            camera_position,
            self.rig.framing_mirror,
        )?;
        self.rig.target_quaternion = Vector::known(result);
        self.rig
            .try_do_framing_update(dt, shot_editor, cvars)
            .map_err(FrameCompositionError::RigFraming)?;
        self.publish_frame_prefix(inverse_dt);
        self.try_publish_frame_angular_velocity(inverse_dt, &mut HostReckoningNumeric)
            .map_err(FrameCompositionError::Numeric)?;
        self.try_publish_frame(dt, shot_editor)
    }

    /// Source 82e48670..48854: Apply, frame matrix, positioner offset, then
    /// previous matrix. The shake service retains typed exceptional numeric
    /// boundaries while admitting ordinary finite later frames.
    fn try_publish_frame(
        &mut self,
        dt: f32,
        shot_editor: bool,
    ) -> Result<(), FrameCompositionError> {
        use FrameCompositionError as E;
        let old_matrix = self.frame.matrix;
        let speed = subject_speed(self.rig.subject_velocity, &mut HostReckoningNumeric)
            .map_err(E::Numeric)?;
        let wiping_out = {
            let subject = self.subject.as_ref().ok_or(E::MissingSubject)?;
            let owner = subject.lock().map_err(|_| E::OwnerLock)?;
            owner.flags.wiping_out
        };
        let active = shake_active(wiping_out, self.rig.positioner.flags.colliding);
        let input = crate::skeleton_root_transforms::SkeletonRootMatrix::from_columns([
            self.rig.humanized_matrix[0],
            self.rig.humanized_matrix[1],
            self.rig.humanized_matrix[2],
            self.rig.positioner.position,
        ]);
        if shot_editor {
            self.frame.matrix = input;
        } else {
            let distance = subject_distance(
                &self.rig.anchor_matrix,
                self.rig.positioner.position,
                &mut HostReckoningNumeric,
            )
            .map_err(E::Numeric)?;
            let matrix = self
                .shake
                .try_apply(dt, speed, distance, input, active)
                .map_err(E::Shake)?;
            self.frame.matrix = matrix;
            let offset = self.shake.offset;
            for (lane, value) in offset.xyz.iter().enumerate() {
                if value.is_nan() {
                    return Err(E::Shake(ShakeApplyError::SetterNaN(lane)));
                }
            }
            self.rig.positioner.shake_offset = offset;
            let collision_time = self.rig.positioner.time_since_collision;
            if collision_time < 5.0 {
                let attenuation = collision_time * f32::from_bits(0x3e4c_cccd);
                self.rig.positioner.shake_offset = offset.map(|lane| lane * attenuation);
            }
        }
        let use_new_previous = if self.flags & 0x40 != 0 {
            true
        } else {
            let subject = self.subject.as_ref().ok_or(E::MissingSubject)?;
            subject.lock().map_err(|_| E::OwnerLock)?.flags.teleported
        };
        self.frame.previous_matrix = if use_new_previous {
            self.frame.matrix
        } else {
            old_matrix
        };
        Ok(())
    }
}
pub(super) fn transition_amount(duration: f32, elapsed: f32) -> f32 {
    if duration > 0.0 {
        let amount = elapsed / duration;
        let lower = if -amount >= 0.0 { 0.0 } else { amount };
        let clamped = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
        match runtime_math::ease(clamped, &mut HostReckoningSinCosNumeric) {
            Ok(value) => value,
            Err(never) => match never {},
        }
    } else {
        1.0
    }
}

#[cfg(test)]
#[path = "camera_frame_composition_tests.rs"]
mod tests;
