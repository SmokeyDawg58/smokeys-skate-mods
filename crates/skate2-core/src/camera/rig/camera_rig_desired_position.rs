//! `Rig::GetDesiredCameraPosition82e3fcb0` and its positioner calculation.
use crate::{
    camera_rig::CameraRig,
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    skeleton_root_transforms::SkeletonRootVector,
};

#[cfg(test)]
#[path = "camera_rig_desired_position_tests.rs"]
mod tests;

impl CameraRig {
    /// The selected numeric adapter uses the existing nearest-even host policy.
    /// Only the anchor's position reaches `CalculateCameraPosition82e3f690`.
    pub(crate) fn desired_camera_position(
        &self,
        elevation: f32,
        heading: f32,
        distance: f32,
    ) -> SkeletonRootVector {
        calculate_position(
            self.anchor_matrix.columns()[3],
            elevation,
            heading,
            distance,
        )
    }
}

fn calculate_position(
    anchor: SkeletonRootVector,
    elevation: f32,
    heading: f32,
    distance: f32,
) -> SkeletonRootVector {
    calculate_position_with_shake(
        anchor,
        crate::camera_subject::ZERO,
        elevation,
        heading,
        distance,
    )
}

pub(crate) fn calculate_position_with_shake(
    anchor: SkeletonRootVector,
    shake: SkeletonRootVector,
    elevation: f32,
    heading: f32,
    distance: f32,
) -> SkeletonRootVector {
    let mut numeric = HostReckoningSinCosNumeric;
    let (elevation_sine, elevation_cosine) =
        try_sin_cos(-elevation, &mut numeric).unwrap_or_else(|never| match never {});
    // The native <0,5,0,0> permute duplicates the X lane into W. These are
    // calculation temporaries, not an invented affine identity matrix.
    let elevation_basis = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, elevation_cosine, elevation_sine, 0.0],
        [0.0, -elevation_sine, elevation_cosine, 0.0],
    ];
    let (heading_sine, heading_cosine) =
        try_sin_cos(-heading, &mut numeric).unwrap_or_else(|never| match never {});
    let heading_basis = [
        [heading_cosine, 0.0, -heading_sine, heading_cosine],
        [0.0, 1.0, 0.0, 0.0],
        [heading_sine, 0.0, heading_cosine, heading_sine],
    ];
    let rotated: [[f32; 4]; 3] = elevation_basis.map(|column| {
        std::array::from_fn(|lane| {
            let x = heading_basis[0][lane] * column[0];
            let y = heading_basis[1][lane].mul_add(column[1], x);
            heading_basis[2][lane].mul_add(column[2], y)
        })
    });
    // Native still performs both zero products before the distance FMA.
    let local: [f32; 4] = std::array::from_fn(|lane| {
        let x = rotated[0][lane] * 0.0;
        let y = rotated[1][lane].mul_add(0.0, x);
        rotated[2][lane].mul_add(distance, y)
    });
    SkeletonRootVector::known(local)
        .zip(shake, |value, shake| value + shake)
        .zip(anchor, |value, anchor| value + anchor)
}
