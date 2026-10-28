//! Actual `CameraMan82e45bd8` construction followed by Reset82e3b380.
#[path = "../camera_free_cam_update.rs"]
pub(crate) mod free_cam_update;
#[path = "camera_man_update.rs"]
pub(crate) mod update;

#[path = "camera_man_preupdate.rs"]
pub(crate) mod preupdate;

use crate::{
    camera_attributes::CameraAttributes,
    camera_collision::CameraCollision,
    camera_drop::{CameraDropPredictor, CameraGrindDetector},
    camera_frame::{CameraFrame, CameraFrameComposer},
    camera_prediction::CameraObstacles,
    camera_rig::{CameraRig, RigConstructionError},
    camera_shake::{ShakeKeyframes, ShakeLoadError},
    camera_shot_manager::{CameraShotDatabase, CameraShotManager},
    camera_subject::{CameraSubject, ZERO},
    camera_tracker::CameraTracker,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::prediction::PhysicalPredictionOwner,
    skeleton_root_transforms::SkeletonRootVector,
};
use skate2_formats::mount::{MountError, MountSet};
use std::sync::{Arc, Mutex};

pub(crate) struct CameraManDescriptor {
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) collision: Arc<Mutex<CameraCollision>>,
    pub(crate) predictor: Arc<PhysicalPredictionOwner>,
    pub(crate) obstacles: Arc<Mutex<CameraObstacles>>,
    pub(crate) database: Arc<Mutex<CameraShotDatabase>>,
    pub(crate) grind: Arc<Mutex<CameraGrindDetector>>,
    pub(crate) drop: Arc<Mutex<CameraDropPredictor>>,
}
#[derive(Debug)]
pub(crate) enum CameraManError<E> {
    Allocation,
    Rig(RigConstructionError<E>),
    Mount(MountError),
    Shake(ShakeLoadError),
}
pub(crate) struct CameraShake {
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) keyframes: ShakeKeyframes,
    pub(crate) offset: SkeletonRootVector,
    pub(crate) time: f32,
    pub(crate) one_shot_effect: f32,
    pub(crate) multiplier: f32,
    pub(crate) wait_for_drop: bool,
}
pub(crate) struct CameraFreeCam {
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) elevation_offset: f32,
    pub(crate) heading_offset: f32,
}
pub(crate) struct CameraMan {
    pub(crate) frame: CameraFrame,
    pub(crate) physics_frame: CameraFrame,
    pub(crate) heading_mirroring: CameraTracker<f32>,
    pub(crate) framing_mirroring: CameraTracker<f32>,
    pub(crate) grind: Arc<Mutex<CameraGrindDetector>>,
    pub(crate) drop: Arc<Mutex<CameraDropPredictor>>,
    pub(crate) free_cam: CameraFreeCam,
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) shots: CameraShotManager,
    pub(crate) composer: CameraFrameComposer,
    pub(crate) rig: CameraRig,
    pub(crate) shake: CameraShake,
    pub(crate) subject: Option<Arc<Mutex<CameraSubject>>>,
    pub(crate) anchor_position: SkeletonRootVector,
    pub(crate) previous_anchor_position: SkeletonRootVector,
    pub(crate) anchor_velocity: SkeletonRootVector,
    pub(crate) world_anchor_position: SkeletonRootVector,
    pub(crate) camera_velocity: SkeletonRootVector,
    pub(crate) camera_angular_velocity: Option<SkeletonRootVector>,
    pub(crate) landing_normal: SkeletonRootVector,
    pub(crate) launch_position: SkeletonRootVector,
    pub(crate) incline_normal: SkeletonRootVector,
    pub(crate) previous_subject_velocity: SkeletonRootVector,
    pub(crate) current_anchor: u32,
    pub(crate) previous_anchor: Option<u32>,
    pub(crate) frontside_angle: f32,
    pub(crate) incline_downhill_heading: f32,
    pub(crate) incline_angle: f32,
    pub(crate) absolute_incline_angle: f32,
    pub(crate) absolute_launch_incline: f32,
    pub(crate) landing_incline: f32,
    pub(crate) absolute_landing_incline: f32,
    pub(crate) incline_alignment: f32,
    pub(crate) grind_incline: f32,
    pub(crate) anchor_time: Option<f32>,
    pub(crate) vertical_field_of_view: Option<f32>,
    pub(crate) heading_follows_model_time: f32,
    pub(crate) maximum_prediction_height: f32,
    pub(crate) maximum_prediction_height_time: Option<f32>,
    pub(crate) time_since_air: f32,
    pub(crate) landing_relative_height: f32,
    pub(crate) trajectory_height_distance_ratio: f32,
    pub(crate) transfer_angle: f32,
    pub(crate) launch_distance: f32,
    pub(crate) turning_time: f32,
    pub(crate) turning_centred_time: f32,
    pub(crate) opacity: f32,
    pub(crate) aspect_ratio: f32,
    pub(crate) previous_heading: f32,
    pub(crate) motion_blur: f32,
    pub(crate) update_counter: u32,
    pub(crate) flags: u8,
    pub(crate) second_flags: u8,
    pub(crate) second_flags_known: u8,
}
impl CameraMan {
    pub(crate) fn try_construct<N: ReckoningNumeric>(
        descriptor: CameraManDescriptor,
        mounts: &MountSet,
        numeric: &mut N,
    ) -> Result<Self, CameraManError<N::Error>> {
        let frame = CameraFrame::constructed();
        let physics_frame = CameraFrame::constructed();
        let heading_mirroring = CameraTracker::constructed(1.0, 0.0);
        let framing_mirroring = CameraTracker::constructed(1.0, 0.0);
        let shots = CameraShotManager::try_construct(descriptor.database)
            .map_err(|()| CameraManError::Allocation)?;
        let composer = CameraFrameComposer::constructed(Arc::clone(&descriptor.parameters));
        let mut shake = CameraShake {
            parameters: Arc::clone(&descriptor.parameters),
            keyframes: ShakeKeyframes::constructed(),
            offset: ZERO,
            time: 0.0,
            one_shot_effect: 0.0,
            multiplier: 1.0,
            wait_for_drop: false,
        };
        let rig = CameraRig::try_construct(
            Arc::clone(&descriptor.parameters),
            descriptor.collision,
            descriptor.predictor,
            &descriptor.obstacles,
            numeric,
        )
        .map_err(CameraManError::Rig)?;
        let free_cam = CameraFreeCam {
            parameters: Arc::clone(&descriptor.parameters),
            elevation_offset: 0.0,
            heading_offset: 0.0,
        };
        // Real mounted file read belongs here, after Rig/FreeCam and before Reset.
        let bytes = mounts
            .read_path("data\\camera\\1.shk")
            .map_err(CameraManError::Mount)?;
        shake
            .keyframes
            .load(&bytes)
            .map_err(CameraManError::Shake)?;
        let up = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
        Ok(Self {
            frame,
            physics_frame,
            heading_mirroring,
            framing_mirroring,
            grind: descriptor.grind,
            drop: descriptor.drop,
            free_cam,
            parameters: descriptor.parameters,
            shots,
            composer,
            rig,
            shake,
            subject: None,
            anchor_position: ZERO,
            previous_anchor_position: ZERO,
            anchor_velocity: ZERO,
            world_anchor_position: ZERO,
            camera_velocity: ZERO,
            camera_angular_velocity: None,
            landing_normal: up,
            launch_position: ZERO,
            incline_normal: up,
            previous_subject_velocity: ZERO,
            current_anchor: 0,
            previous_anchor: None,
            frontside_angle: 0.0,
            incline_downhill_heading: 0.0,
            incline_angle: 0.0,
            absolute_incline_angle: 0.0,
            absolute_launch_incline: 0.0,
            landing_incline: 0.0,
            absolute_landing_incline: 0.0,
            incline_alignment: 0.0,
            grind_incline: 0.0,
            anchor_time: None,
            vertical_field_of_view: None,
            heading_follows_model_time: 5.0,
            maximum_prediction_height: 0.0,
            maximum_prediction_height_time: None,
            time_since_air: f32::MAX,
            landing_relative_height: 0.0,
            trajectory_height_distance_ratio: 0.0,
            transfer_angle: 0.0,
            launch_distance: 0.0,
            turning_time: 0.0,
            turning_centred_time: 0.0,
            opacity: 1.0,
            aspect_ratio: f32::from_bits(0x3fe3_8e39),
            previous_heading: 0.0,
            motion_blur: 1.0,
            update_counter: 0,
            flags: 0x40,
            second_flags: 0x80,
            second_flags_known: 0xe0,
        })
    }
    /// `SetSubject82e3c198` publishes the same Subject to all three consumers.
    pub(crate) fn set_subject(&mut self, subject: &Arc<Mutex<CameraSubject>>) {
        self.subject = Some(Arc::clone(subject));
        self.rig.subject = Some(Arc::clone(subject));
        self.composer.subject = Some(Arc::clone(subject));
        self.second_flags |= 0x20;
        self.second_flags_known |= 0x20;
    }
    /// `CameraMan.Teleport82e454f8`: earlier shot writes survive numeric failure.
    pub(crate) fn teleport<N: ReckoningNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        self.heading_follows_model_time = f32::from_bits(0x3e99_999a);
        self.shots.teleport();
        self.rig.teleport(numeric)?;
        self.flags |= 0x40;
        self.second_flags = (self.second_flags & 0x5f) | 0x80;
        self.second_flags_known |= 0xa0;
        Ok(())
    }
}
