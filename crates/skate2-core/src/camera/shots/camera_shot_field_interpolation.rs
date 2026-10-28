//! The shared publication order of native blend and transition shot fields.
use super::runtime_math::{self, ShotMathError};
use crate::{
    camera_shot::CameraShot, reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric, reckoning_sin_cos::sine::try_sine,
};

/// `InterpolateShot82e441b8` uses a rounded PI product for each scalar sine;
/// `UpdateBlendedShot82e45fd0` calls `InterpolateFloat` with its already eased
/// amount, giving a second easing pass. Quaternions use the shared native leaf.
pub(super) fn publish<N: ReckoningNumeric, T: ReckoningSinCosNumeric>(
    first: &CameraShot,
    second: &CameraShot,
    destination: &mut CameraShot,
    amount: f32,
    transition: bool,
    numeric: &mut N,
    trig: &mut T,
) -> Result<(), ShotMathError<N::Error, T::Error>> {
    let rounded_pi_product = amount * f32::from_bits(0x4049_0fdb);
    let mut scalar = |a: f32, b: f32| {
        if transition {
            let angle = rounded_pi_product - f32::from_bits(0x3fc9_0fdb);
            let eased = try_sine(angle, trig)?.mul_add(0.5, 0.5);
            Ok((b - a).mul_add(eased, a))
        } else {
            runtime_math::scalar(a, b, 0, amount, trig)
        }
    };
    macro_rules! field {
        ($name:ident) => {
            destination.$name =
                scalar(first.$name, second.$name).map_err(ShotMathError::Trigonometry)?;
        };
    }
    field!(distance);
    field!(lens_length);
    field!(smoothing_direction);
    field!(smoothing_elevation);
    field!(smoothing_yaw);
    field!(smoothing_pitch);
    for index in 0..9 {
        destination.reference_weights[index] = scalar(
            first.reference_weights[index],
            second.reference_weights[index],
        )
        .map_err(ShotMathError::Trigonometry)?;
    }
    field!(board_offset);
    field!(elevation);
    field!(roll);
    field!(yaw);
    field!(pitch);
    destination.follow_subject_in_air = second.follow_subject_in_air;
    destination.mirror_for_stance = second.mirror_for_stance;
    destination.snap_to_reference = second.snap_to_reference;
    destination.use_previous = second.use_previous;
    destination.use_drop_predictor = second.use_drop_predictor;
    destination.use_free_stick = second.use_free_stick;
    field!(blur_shot);
    destination.blur_transition = second.blur_transition;
    field!(subject_opacity);
    destination.collision_hint = second.collision_hint;
    destination.anchor = second.anchor;
    destination.compass_north = second.compass_north;
    field!(world_heading);
    destination.world_arm_orientation = runtime_math::quaternion(
        first.world_arm_orientation,
        second.world_arm_orientation,
        0,
        amount,
        numeric,
        trig,
    )?;
    destination.world_camera_orientation = runtime_math::quaternion(
        first.world_camera_orientation,
        second.world_camera_orientation,
        0,
        amount,
        numeric,
        trig,
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "camera_shot_field_interpolation_tests.rs"]
mod tests;
