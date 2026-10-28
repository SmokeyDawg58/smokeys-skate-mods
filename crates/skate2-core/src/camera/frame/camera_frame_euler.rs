//! `rw::math::vpu::EulerXYZFromMatrix33` at `82e3df20`.
use crate::{
    camera_compass::runtime::math::camera_atan2_pending,
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

const VERTICAL_THRESHOLD: f32 = f32::from_bits(0x3a83_126f);

/// Matrix columns X/Y/Z are sampled by the native helper as scalar splats.
/// The returned W lane is the same source bits as X, not homogeneous zero.
pub(crate) fn euler_from_matrix33<N: ReckoningNumeric>(
    basis: &[Vector; 3],
    numeric: &mut N,
) -> Result<Vector, N::Error> {
    let [x, y, z] = basis;
    let horizontal_squared = x.xyz[0].mul_add(x.xyz[0], x.xyz[1] * x.xyz[1]);
    let seed = numeric.reciprocal_sqrt_estimate(horizontal_squared)?;
    let horizontal = length_from_squared_and_estimate(horizontal_squared, seed);
    let negative_xz = f32::from_bits(x.xyz[2].to_bits() ^ 0x8000_0000);
    let (pitch, yaw, roll) = if horizontal > VERTICAL_THRESHOLD {
        // Native issues Z, Y, X atan producers in this order.
        let roll = camera_atan2_pending(x.xyz[1], x.xyz[0], numeric)?;
        let yaw = camera_atan2_pending(negative_xz, horizontal, numeric)?;
        let pitch = camera_atan2_pending(y.xyz[2], z.xyz[2], numeric)?;
        // Native then issues signed-π additions in Y, X, Z order.
        let yaw = yaw.finish();
        let pitch = pitch.finish();
        let roll = roll.finish();
        (pitch, yaw, roll)
    } else {
        // Native issues Y then X, and explicitly inserts +0 into Z.
        let yaw = camera_atan2_pending(negative_xz, horizontal, numeric)?;
        let negative_zy = f32::from_bits(z.xyz[1].to_bits() ^ 0x8000_0000);
        let pitch = camera_atan2_pending(negative_zy, y.xyz[1], numeric)?;
        let yaw = yaw.finish();
        let pitch = pitch.finish();
        (pitch, yaw, 0.0)
    };
    Ok(Vector::known([pitch, yaw, roll, pitch]))
}

#[cfg(test)]
#[path = "camera_frame_euler_tests.rs"]
mod tests;
