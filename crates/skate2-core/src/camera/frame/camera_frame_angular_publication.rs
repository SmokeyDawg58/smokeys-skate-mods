//! CameraMan::UpdateRig82e48564..82e4866c angular publication.
use crate::{
    camera_frame::euler::euler_from_matrix33,
    camera_man::CameraMan,
    reckoning_numeric::{ReckoningNumeric, try_normalize_angle},
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

impl CameraMan {
    /// Caller order: old Frame basis, new Rig basis, then X/Y/Z normalization.
    pub(crate) fn try_publish_frame_angular_velocity<N: ReckoningNumeric>(
        &mut self,
        inverse_dt: f32,
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        let old_basis = std::array::from_fn(|lane| self.frame.matrix.columns()[lane]);
        let old = euler_from_matrix33(&old_basis, numeric)?;
        let new = euler_from_matrix33(&self.rig.humanized_matrix, numeric)?;
        publish_angular_from_eulers(
            &mut self.camera_angular_velocity,
            old,
            new,
            inverse_dt,
            numeric,
        )
    }
}

/// Native publishes each intermediate vector before issuing the next Normalize.
fn publish_angular_from_eulers<N: ReckoningNumeric>(
    published: &mut Option<Vector>,
    old: Vector,
    new: Vector,
    inverse_dt: f32,
    numeric: &mut N,
) -> Result<(), N::Error> {
    let mut difference = new.zip(old, |new, old| new - old);
    *published = Some(difference); // 82e485ac: raw new-minus-old, all lanes.
    difference.xyz[0] = try_normalize_angle(difference.xyz[0], numeric)?;
    *published = Some(difference); // 82e485cc
    difference.xyz[1] = try_normalize_angle(difference.xyz[1], numeric)?;
    *published = Some(difference); // 82e485f0
    difference.xyz[2] = try_normalize_angle(difference.xyz[2], numeric)?;
    *published = Some(difference); // 82e48658; W is still raw delta-X.
    *published = Some(difference.map(|lane| lane * inverse_dt)); // 82e4866c
    Ok(())
}

#[cfg(test)]
#[path = "camera_frame_angular_publication_tests.rs"]
mod tests;
