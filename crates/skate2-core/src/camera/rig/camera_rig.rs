//! Actual `Iris::Rig82e455b0`, constructor Reset, and `Teleport82e42310`.
#[path = "camera_rig_anchor_dynamics.rs"]
mod anchor_dynamics;
#[path = "camera_rig_angle_trackers.rs"]
mod angle_trackers;
#[path = "camera_rig_dynamics.rs"]
pub(crate) mod dynamics;
#[path = "camera_rig_framing_update.rs"]
pub(crate) mod framing_update;
#[path = "camera_rig_orientation.rs"]
pub(crate) mod orientation;
#[path = "camera_rig_positioner.rs"]
mod positioner;
#[path = "camera_rig_publication_prefix.rs"]
pub(crate) mod publication_prefix;
#[path = "camera_rig_subject_matrix.rs"]
pub(crate) mod subject_matrix;
#[path = "camera_rig_update.rs"]
pub(crate) mod update;

#[path = "camera_rig_desired_position.rs"]
pub(crate) mod desired_position;
#[path = "camera_rig_speed.rs"]
pub(crate) mod speed;

use crate::{
    camera_attributes::CameraAttributes,
    camera_collision::CameraCollision,
    camera_positioner::CameraPositioner,
    camera_prediction::{CameraObstacles, CameraPathEvaluator, CameraPrediction},
    camera_subject::{CameraSubject, ZERO, identity},
    camera_tracker::CameraTracker,
    reckoning_numeric::{ReckoningNumeric, try_normalize_angle},
    simulation_assembly_backend::constraint_session::prediction::PhysicalPredictionOwner,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex};

pub(crate) struct CameraBreadcrumbs {
    pub(crate) positions: Vec<SkeletonRootVector>,
    pub(crate) count: u32,
    pub(crate) last: u32,
    pub(crate) minimum_distance_squared: f32,
}
#[derive(Debug)]
pub(crate) enum RigConstructionError<E> {
    Allocation,
    Numeric(E),
}

pub(crate) struct CameraRig {
    pub(crate) anchor_matrix: SkeletonRootMatrix,
    pub(crate) humanized_matrix: [SkeletonRootVector; 3],
    pub(crate) previous_shot_quaternion: SkeletonRootVector,
    pub(crate) target_quaternion: SkeletonRootVector,
    /// Desired, tucked-in heading, tucked-in elevation in native allocation order.
    pub(crate) predictors: [Arc<Mutex<CameraPrediction>>; 3],
    pub(crate) evaluators: [CameraPathEvaluator; 3],
    pub(crate) heading_tracker: CameraTracker<f32>,
    pub(crate) elevation_tracker: CameraTracker<f32>,
    pub(crate) pitch_tracker: CameraTracker<f32>,
    pub(crate) yaw_tracker: CameraTracker<f32>,
    pub(crate) anchor_tracker: CameraTracker<SkeletonRootVector>,
    pub(crate) positioner: CameraPositioner,
    pub(crate) breadcrumbs: CameraBreadcrumbs,
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) subject: Option<Arc<Mutex<CameraSubject>>>,
    pub(crate) collision: Arc<Mutex<CameraCollision>>,
    pub(crate) world_predictor: Arc<PhysicalPredictionOwner>,
    pub(crate) subject_velocity: SkeletonRootVector,
    pub(crate) subject_acceleration: SkeletonRootVector,
    pub(crate) subject_angular_velocity: SkeletonRootVector,
    pub(crate) last_camera_position: SkeletonRootVector,
    pub(crate) last_desired_position: SkeletonRootVector,
    pub(crate) requested_heading: f32,
    pub(crate) pre_avoidance_heading: f32,
    pub(crate) pre_avoidance_elevation: f32,
    pub(crate) requested_elevation: f32,
    pub(crate) elevation_offset: f32,
    pub(crate) heading_offset: f32,
    pub(crate) incline: f32,
    pub(crate) avoidance_heading: f32,
    pub(crate) avoidance_elevation: f32,
    pub(crate) acceleration_scalar: f32,
    pub(crate) last_acceleration_scalar: f32,
    pub(crate) collision_interpolation_time: f32,
    pub(crate) initialization_time: f32,
    pub(crate) collision_avoidance_off_time: f32,
    pub(crate) blend_from_height: f32,
    pub(crate) blend_to_height: f32,
    pub(crate) blend_height_duration: f32,
    pub(crate) blend_height_elapsed: f32,
    pub(crate) heading_mirror: f32,
    pub(crate) framing_mirror: f32,
    pub(crate) reference_height: f32,
    pub(crate) reference_height_at_launch: f32,
    pub(crate) interpolated_height: f32,
    pub(crate) push_time: f32,
    pub(crate) pump_time: f32,
    pub(crate) distance: f32,
    pub(crate) arm_extension: f32,
    pub(crate) heading_speed: f32,
    pub(crate) elevation_speed: f32,
    pub(crate) pan_smoothing: f32,
    pub(crate) tilt_smoothing: f32,
    pub(crate) subject_tracker_smoothing_time: f32,
    pub(crate) roll: f32,
    pub(crate) avoidance_heading_amount: f32,
    pub(crate) avoidance_elevation_amount: f32,
    pub(crate) subject_state_time: f32,
    pub(crate) update_counter: u32,
    pub(crate) collision_hint: u32,
    pub(crate) avoidance_strategy: u32,
    pub(crate) avoidance_type: u32,
    pub(crate) subject_state: u32,
    pub(crate) previous_subject_state: u32,
    pub(crate) flags: u8,
    pub(crate) second_flags: u8,
    pub(crate) second_flags_known: u8,
}
impl CameraRig {
    /// Only the actual selected `SubjectPath` descriptor is constructed here.
    /// No camera parameter read and no predictor query occurs in this chain.
    #[expect(
        clippy::too_many_lines,
        reason = "single native constructor publication"
    )]
    pub(crate) fn try_construct<N: ReckoningNumeric>(
        parameters: Arc<Mutex<CameraAttributes>>,
        collision: Arc<Mutex<CameraCollision>>,
        world_predictor: Arc<PhysicalPredictionOwner>,
        obstacles: &Arc<Mutex<CameraObstacles>>,
        numeric: &mut N,
    ) -> Result<Self, RigConstructionError<N::Error>> {
        let positioner =
            CameraPositioner::constructed(Arc::clone(&collision), Arc::clone(&parameters), true);
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(2)
            .map_err(|_| RigConstructionError::Allocation)?;
        positions.resize(2, ZERO);
        let breadcrumbs = CameraBreadcrumbs {
            positions,
            count: 2,
            last: 0,
            minimum_distance_squared: 1.0,
        };
        let predictors = std::array::from_fn(|_| {
            Arc::new(Mutex::new(CameraPrediction::constructed(Arc::clone(
                &world_predictor,
            ))))
        });
        let evaluators = std::array::from_fn(|i| {
            CameraPathEvaluator::constructed(Arc::clone(obstacles), Arc::clone(&predictors[i]))
        });
        let mut angle_tracker = || {
            try_normalize_angle(0.0, numeric)
                .map(|angle| CameraTracker::constructed(angle, 0.0))
                .map_err(RigConstructionError::Numeric)
        };
        let heading_tracker = angle_tracker()?;
        let elevation_tracker = angle_tracker()?;
        let pitch_tracker = angle_tracker()?;
        let yaw_tracker = angle_tracker()?;
        let basis = identity();
        let mut rig = Self {
            anchor_matrix: basis,
            humanized_matrix: [basis.columns()[0], basis.columns()[1], basis.columns()[2]],
            previous_shot_quaternion: ZERO,
            target_quaternion: ZERO,
            predictors,
            evaluators,
            heading_tracker,
            elevation_tracker,
            pitch_tracker,
            yaw_tracker,
            anchor_tracker: CameraTracker::constructed(ZERO, ZERO),
            positioner,
            breadcrumbs,
            parameters,
            subject: None,
            collision,
            world_predictor,
            subject_velocity: ZERO,
            subject_acceleration: ZERO,
            subject_angular_velocity: ZERO,
            last_camera_position: ZERO,
            last_desired_position: ZERO,
            requested_heading: f32::from_bits(0x4049_0fdb),
            pre_avoidance_heading: f32::from_bits(0x4049_0fdb),
            pre_avoidance_elevation: 0.0,
            requested_elevation: 0.0,
            elevation_offset: 0.0,
            heading_offset: 0.0,
            incline: 0.0,
            avoidance_heading: 0.0,
            avoidance_elevation: 0.0,
            acceleration_scalar: 0.0,
            last_acceleration_scalar: 0.0,
            collision_interpolation_time: 5.0,
            initialization_time: 2.0,
            collision_avoidance_off_time: 0.0,
            blend_from_height: 0.0,
            blend_to_height: 0.0,
            blend_height_duration: 0.0,
            blend_height_elapsed: 0.0,
            heading_mirror: 1.0,
            framing_mirror: 1.0,
            reference_height: 1000.0,
            reference_height_at_launch: f32::MAX,
            interpolated_height: 0.0,
            push_time: 0.0,
            pump_time: 0.0,
            distance: 1.0,
            arm_extension: 0.0,
            heading_speed: 1.0,
            elevation_speed: 1.0,
            pan_smoothing: 0.0,
            tilt_smoothing: 0.0,
            subject_tracker_smoothing_time: 0.0,
            roll: 0.0,
            avoidance_heading_amount: 0.0,
            avoidance_elevation_amount: 0.0,
            subject_state_time: 0.0,
            update_counter: 0,
            collision_hint: 0,
            avoidance_strategy: 0,
            avoidance_type: 1,
            subject_state: 0,
            previous_subject_state: 0,
            flags: 0x88,
            second_flags: 0x20,
            second_flags_known: 0xe0,
        };
        rig.update_angle_trackers_instant(numeric)
            .map_err(RigConstructionError::Numeric)?;
        Ok(rig)
    }
    fn update_angle_trackers_instant<N: ReckoningNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        self.heading_tracker
            .try_set_angle_instant(self.requested_heading, true, numeric)?;
        self.elevation_tracker
            .try_set_angle_instant(self.requested_elevation, true, numeric)?;
        Ok(())
    }
    pub(crate) fn teleport<N: ReckoningNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        self.flags &= !1;
        self.update_angle_trackers_instant(numeric)?;
        self.anchor_tracker.input_position = self.anchor_matrix.columns()[3];
        self.anchor_tracker.output_position = self.anchor_matrix.columns()[3];
        self.anchor_tracker.velocity = ZERO;
        self.anchor_tracker.acceleration = ZERO;
        self.reference_height = self.anchor_matrix.columns()[3].xyz[1];
        self.avoidance_heading_amount = 0.0;
        self.avoidance_elevation_amount = 0.0;
        self.positioner.teleport();
        self.flags |= 8;
        self.initialization_time = f32::from_bits(0x3dcc_cccd);
        self.collision_avoidance_off_time = 2.0;
        self.blend_height_duration = 0.0;
        Ok(())
    }
}
