//! Caller-owned `CameraMan::UpdateRig82e48494..82e48560` frame prefix.
use crate::{
    camera_frame::CameraFrame, camera_man::CameraMan,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

/// `82e47eb0..82e47ebc`: retained reciprocal made before other UpdateRig work.
pub(crate) fn inverse_frame_dt(dt: f32) -> f32 {
    if dt == 0.0 { 1.0 } else { 1.0 / dt }
}

impl CameraMan {
    /// Publishes the source writes before the first matrix-to-Euler call.
    pub(crate) fn publish_frame_prefix(&mut self, inverse_dt: f32) {
        publish_prefix_parts(
            &mut self.frame,
            &mut self.camera_velocity,
            self.composer.reference,
            self.composer.framing_x,
            self.composer.framing_y,
            self.rig.positioner.position,
            inverse_dt,
        );
    }
}

fn publish_prefix_parts(
    frame: &mut CameraFrame,
    camera_velocity: &mut Vector,
    reference: Vector,
    framing_x: f32,
    framing_y: f32,
    position: Vector,
    inverse_dt: f32,
) {
    frame.reference = reference;
    frame.framing_x = framing_x;
    frame.framing_y = framing_y;
    let previous_position = frame.matrix.columns()[3];
    // Raw 82e48540/44: current - previous, then multiply by splatted f26.
    *camera_velocity = position.zip(previous_position, |current, previous| {
        (current - previous) * inverse_dt
    });
}

#[cfg(test)]
#[path = "camera_frame_publication_prefix_tests.rs"]
mod tests;
