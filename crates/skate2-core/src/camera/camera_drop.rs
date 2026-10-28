//! Actual `DropPredictor82e39b98`; no line batch is issued by construction.
#[path = "camera_drop_update.rs"]
pub(crate) mod update;

use crate::{
    camera_attributes::CameraAttributes, camera_collision::CameraCollision, camera_subject::ZERO,
    camera_tracker::CameraTracker,
    simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineBatch,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
pub(crate) struct CameraLine {
    pub(crate) start: SkeletonRootVector,
    pub(crate) end: SkeletonRootVector,
    pub(crate) radius: f32,
}
#[derive(Clone, Copy)]
pub(crate) struct CameraDropTest {
    pub(crate) start: SkeletonRootVector,
    pub(crate) end: SkeletonRootVector,
    pub(crate) distance: f32,
    pub(crate) valid: bool,
    pub(crate) hit: bool,
}
pub(crate) struct CameraDropPredictor {
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) smoothing_tracker: CameraTracker<f32>,
    pub(crate) batch: Option<SimulationWorldLineBatch>,
    pub(crate) collision: Arc<Mutex<CameraCollision>>,
    pub(crate) lines: [CameraLine; 10],
    pub(crate) tests: [CameraDropTest; 5],
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) distance_offset: f32,
    pub(crate) elevation_offset: f32,
    pub(crate) step_size: f32,
    pub(crate) smoothing: f32,
    pub(crate) target_elevation_offset: f32,
    pub(crate) current_drop: i32,
    pub(crate) largest_drop: i32,
    pub(crate) previous_valid_drops: i32,
    pub(crate) was_using_drop_curve: bool,
    pub(crate) waiting_for_results: bool,
}
impl CameraDropPredictor {
    pub(crate) fn constructed(
        parameters: Arc<Mutex<CameraAttributes>>,
        collision: Arc<Mutex<CameraCollision>>,
    ) -> Self {
        Self {
            parameters,
            smoothing_tracker: CameraTracker::constructed(0.0, 0.0),
            batch: None,
            collision,
            tests: [CameraDropTest {
                start: ZERO,
                end: ZERO,
                distance: 0.0,
                valid: false,
                hit: false,
            }; 5],
            lines: [CameraLine {
                start: ZERO,
                end: ZERO,
                radius: 0.0,
            }; 10],
            velocity: ZERO,
            distance_offset: 0.0,
            elevation_offset: 0.0,
            step_size: 0.0,
            smoothing: 0.0,
            target_elevation_offset: 0.0,
            current_drop: 0,
            largest_drop: 0,
            previous_valid_drops: 0,
            was_using_drop_curve: true,
            waiting_for_results: false,
        }
    }
}

/// `GameCameraManager82e37e24..7e54`, retained by the descriptor. This is only
/// construction of the camera's awareness owner, not parked grind behaviour.
pub(crate) struct CameraGrindDetector {
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) nearest_position: SkeletonRootVector,
    pub(crate) position_angle: f32,
    pub(crate) heading: f32,
    pub(crate) anticipation: f32,
    pub(crate) shaped_anticipation: f32,
    pub(crate) position_valid: bool,
    pub(crate) active: bool,
}
impl CameraGrindDetector {
    pub(crate) fn constructed(parameters: Arc<Mutex<CameraAttributes>>) -> Self {
        Self {
            parameters,
            nearest_position: ZERO,
            position_angle: 0.0,
            heading: 0.0,
            anticipation: 0.0,
            shaped_anticipation: 0.0,
            position_valid: false,
            active: false,
        }
    }
}
