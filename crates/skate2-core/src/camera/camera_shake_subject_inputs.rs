//! Source inputs to `CameraMan::UpdateRig`'s shake call, `82e48670..82e487a8`.
//! This module does not apply shake or publish a completed frame.
use crate::{
    reckoning_numeric::{ReckoningNumeric, try_length},
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

/// 82e4868c..9c copies velocity X (+80) and Z (+88) over a stack vector whose Y
/// is f27 (+0) and W is r25; `vmsum3fp128 v51,v63,v63` is its horizontal length.
pub(crate) fn subject_speed<N: ReckoningNumeric>(
    rig_subject_velocity: Vector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    try_length(
        Vector::known([
            rig_subject_velocity.xyz[0],
            0.0,
            rig_subject_velocity.xyz[2],
            0.0,
        ]),
        numeric,
    )
}

/// Native checks the Subject getter before the positioner's collision byte.
pub(crate) fn shake_active(wiping_out: bool, positioner_colliding: bool) -> bool {
    !wiping_out || positioner_colliding
}

/// Called only after the `shotEditor == false` branch, before `ShakeEffect::Apply`.
pub(crate) fn subject_distance<N: ReckoningNumeric>(
    rig_anchor_matrix: &Matrix,
    camera_position: Vector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let difference =
        rig_anchor_matrix.columns()[3].zip(camera_position, |anchor, camera| anchor - camera);
    try_length(difference, numeric)
}

#[cfg(test)]
#[path = "camera_shake_subject_inputs_tests.rs"]
mod tests;
