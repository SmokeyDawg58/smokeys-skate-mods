//! `LogicalCameraPositioner82e3f520`: real constructor writes and unwritten fields.
#[path = "camera_positioner_update.rs"]
pub(crate) mod update;
use crate::{
    camera_attributes::CameraAttributes,
    camera_collision::{CameraCollision, CameraLineResult},
    camera_subject::ZERO,
    camera_tracker::CameraTracker,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

pub(crate) struct CameraPositioner {
    pub(crate) distance: CameraTracker<f32>,
    pub(crate) near_plane: CameraTracker<f32>,
    pub(crate) null_result: CameraLineResult,
    pub(crate) collision: Arc<Mutex<CameraCollision>>,
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) collision_starts: [SkeletonRootVector; 3],
    pub(crate) current_collision_start: SkeletonRootVector,
    pub(crate) anchor: SkeletonRootVector,
    pub(crate) position: SkeletonRootVector,
    pub(crate) last_position: SkeletonRootVector,
    pub(crate) last_good_position: SkeletonRootVector,
    pub(crate) shake_offset: SkeletonRootVector,
    pub(crate) avoidance_heading: f32,
    pub(crate) avoidance_elevation: f32,
    pub(crate) heading: f32,
    pub(crate) elevation: f32,
    pub(crate) requested_distance: f32,
    pub(crate) near_clipping_distance: Option<f32>,
    pub(crate) collision_radius: f32,
    pub(crate) time_since_collision: f32,
    pub(crate) minimum_distance: f32,
    pub(crate) collision_start_count: u32,
    pub(crate) collision_start_index: u32,
    pub(crate) flags: PositionerFlags,
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "distinct native positioner flags"
)]
pub(crate) struct PositionerFlags {
    pub(crate) colliding: bool,
    pub(crate) request_avoidance: bool,
    pub(crate) using_avoidance_position: bool,
    pub(crate) uses_avoidance: bool,
    pub(crate) camera_position_invalid: bool,
    pub(crate) anchor_colliding: bool,
    pub(crate) retry_while_positioning: bool,
    pub(crate) heavyweight_hard_cut: bool,
    pub(crate) heading_elevation_continuous: bool,
    pub(crate) collision_check: bool,
    pub(crate) hard_cut: bool,
    pub(crate) changed_collision_start_index: Option<bool>,
}
impl CameraPositioner {
    pub(crate) fn constructed(
        collision: Arc<Mutex<CameraCollision>>,
        parameters: Arc<Mutex<CameraAttributes>>,
        uses_avoidance: bool,
    ) -> Self {
        Self {
            distance: CameraTracker::constructed(1.0, 0.0),
            near_plane: CameraTracker::constructed(f32::from_bits(0x3e4c_cccd), 0.0),
            null_result: CameraLineResult::constructed(),
            collision,
            parameters,
            collision_starts: [ZERO; 3],
            current_collision_start: ZERO,
            anchor: ZERO,
            position: ZERO,
            last_position: ZERO,
            last_good_position: ZERO,
            shake_offset: ZERO,
            avoidance_heading: 0.0,
            avoidance_elevation: 0.0,
            heading: 0.0,
            elevation: 0.0,
            requested_distance: 1.0,
            near_clipping_distance: None,
            collision_radius: f32::from_bits(0x3e0f_5c29),
            time_since_collision: f32::MAX,
            minimum_distance: 0.5,
            collision_start_count: 0,
            collision_start_index: 0,
            flags: PositionerFlags {
                colliding: false,
                request_avoidance: false,
                using_avoidance_position: false,
                uses_avoidance,
                camera_position_invalid: false,
                anchor_colliding: false,
                retry_while_positioning: false,
                heavyweight_hard_cut: false,
                heading_elevation_continuous: false,
                collision_check: true,
                hard_cut: false,
                changed_collision_start_index: None,
            },
        }
    }
    /// The positioner stores performed by `Rig.Teleport82e4239c..23ac`.
    pub(crate) fn teleport(&mut self) {
        self.distance.input_position = self.requested_distance;
        self.distance.output_position = self.requested_distance;
        self.distance.velocity = 0.0;
        self.distance.acceleration = 0.0;
    }
}
