//! Actual `CameraFrame829d2ec8` and `FrameComposer82e45d0c` constructor fields.
#[path = "camera_frame_angular_publication.rs"]
pub(crate) mod angular_publication;
#[path = "camera_frame_composition.rs"]
pub(crate) mod composition;
#[path = "camera_frame_composition_math.rs"]
pub(crate) mod composition_math;
#[path = "camera_frame_euler.rs"]
pub(crate) mod euler;
#[path = "camera_frame_publication_prefix.rs"]
pub(crate) mod publication_prefix;
#[path = "camera_frame_reference.rs"]
mod reference;

use crate::{
    camera_attributes::CameraAttributes,
    camera_subject::{CameraSubject, ZERO, identity},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug)]
pub(crate) struct CameraFrame {
    pub(crate) matrix: SkeletonRootMatrix,
    pub(crate) previous_matrix: SkeletonRootMatrix,
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) angular_velocity: SkeletonRootVector,
    pub(crate) anchor: SkeletonRootVector,
    pub(crate) reference: SkeletonRootVector,
    pub(crate) field_of_view: f32,
    pub(crate) framing_x: f32,
    pub(crate) framing_y: f32,
    pub(crate) near_plane: f32,
    pub(crate) motion_blur: f32,
    pub(crate) index_override: i32,
    pub(crate) hard_cut: bool,
}
impl CameraFrame {
    pub(crate) fn constructed() -> Self {
        Self {
            matrix: identity(),
            previous_matrix: identity(),
            velocity: ZERO,
            angular_velocity: ZERO,
            anchor: ZERO,
            reference: ZERO,
            field_of_view: 60.0,
            framing_x: 0.0,
            framing_y: 0.0,
            near_plane: f32::from_bits(0x3e4c_cccd),
            motion_blur: 1.0,
            index_override: -1,
            hard_cut: false,
        }
    }
}
pub(crate) struct CameraFrameComposer {
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) subject: Option<Arc<Mutex<CameraSubject>>>,
    pub(crate) last_roll: f32,
    pub(crate) last_rotation: SkeletonRootVector,
    pub(crate) anchor: SkeletonRootVector,
    pub(crate) reference: SkeletonRootVector,
    pub(crate) framing_x: f32,
    pub(crate) framing_y: f32,
}
impl CameraFrameComposer {
    pub(crate) fn constructed(parameters: Arc<Mutex<CameraAttributes>>) -> Self {
        Self {
            parameters,
            subject: None,
            last_roll: 0.0,
            last_rotation: SkeletonRootVector::known([0.0, 0.0, 0.0, 1.0]),
            anchor: ZERO,
            reference: ZERO,
            framing_x: 0.0,
            framing_y: 0.0,
        }
    }
}
