//! Actual `Iris::Shot82e3d1d0`; authored shot data is loaded separately.
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
pub(crate) enum TransitionUnits {
    Seconds,
    Ticks,
    Speed,
    Other(i32),
}
impl TransitionUnits {
    pub(crate) fn from_native(value: i32) -> Self {
        match value {
            0 => Self::Seconds,
            1 => Self::Ticks,
            2 => Self::Speed,
            other => Self::Other(other),
        }
    }
    pub(crate) fn native_value(self) -> i32 {
        match self {
            Self::Seconds => 0,
            Self::Ticks => 1,
            Self::Speed => 2,
            Self::Other(value) => value,
        }
    }
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent native shot options"
)]
// Shot::operator=82e3d390 copies every represented field; child pointers remain shared.
#[derive(Clone)]
pub(crate) struct CameraShot {
    pub(crate) name: [Option<u8>; 32],
    pub(crate) name_hash: u32,
    pub(crate) blend_shots: [Option<Arc<Mutex<CameraShot>>>; 3],
    pub(crate) shot_type: i32,
    pub(crate) transition_time: f32,
    pub(crate) transition_units: TransitionUnits,
    pub(crate) distance: f32,
    pub(crate) lens_length: f32,
    pub(crate) smoothing_direction: f32,
    pub(crate) smoothing_elevation: f32,
    pub(crate) smoothing_yaw: f32,
    pub(crate) smoothing_pitch: f32,
    /// Source order Head,Hips,Grind,COM,Board,Trajectory,Anchor,Feet,DampedCOM.
    pub(crate) reference_weights: [f32; 9],
    pub(crate) board_offset: f32,
    pub(crate) heading: f32,
    pub(crate) elevation: f32,
    pub(crate) roll: f32,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) follow_subject_in_air: bool,
    pub(crate) mirror_for_stance: bool,
    pub(crate) snap_to_reference: bool,
    pub(crate) use_previous: bool,
    pub(crate) use_drop_predictor: bool,
    pub(crate) use_free_stick: bool,
    pub(crate) blur_shot: f32,
    pub(crate) blur_transition: f32,
    pub(crate) subject_opacity: f32,
    pub(crate) collision_hint: i32,
    pub(crate) anchor: u32,
    pub(crate) compass_north: i32,
    pub(crate) blend_names: [[Option<u8>; 32]; 3],
    pub(crate) blend_points: [f32; 3],
    pub(crate) blend_value: f32,
    pub(crate) blend_type: i32,
    pub(crate) blend_smoothing: f32,
    pub(crate) world_heading: f32,
    pub(crate) world_arm_orientation: [f32; 4],
    pub(crate) world_camera_orientation: [f32; 4],
}
impl CameraShot {
    pub(crate) fn constructed() -> Self {
        let mut empty_name = [None; 32];
        empty_name[0] = Some(0);
        Self {
            name: empty_name,
            name_hash: 0,
            blend_shots: [None, None, None],
            shot_type: 0,
            transition_time: 0.5,
            transition_units: TransitionUnits::Seconds,
            distance: 2.0,
            lens_length: 12.0,
            smoothing_direction: f32::from_bits(0x3f4c_cccd),
            smoothing_elevation: f32::from_bits(0x3f4c_cccd),
            smoothing_yaw: f32::from_bits(0x3f4c_cccd),
            smoothing_pitch: f32::from_bits(0x3f4c_cccd),
            reference_weights: [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            board_offset: 0.0,
            heading: f32::from_bits(0x4049_0fdb),
            elevation: 0.0,
            roll: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            follow_subject_in_air: true,
            mirror_for_stance: true,
            snap_to_reference: false,
            use_previous: false,
            use_drop_predictor: false,
            use_free_stick: false,
            blur_shot: 1.0,
            blur_transition: 1.0,
            subject_opacity: 1.0,
            collision_hint: 0,
            anchor: 0,
            compass_north: 0,
            blend_names: [empty_name; 3],
            blend_points: [0.0; 3],
            blend_value: 0.0,
            blend_type: 0,
            blend_smoothing: 0.0,
            world_heading: 0.0,
            world_arm_orientation: [0.0, 0.0, 0.0, 1.0],
            world_camera_orientation: [0.0, 0.0, 0.0, 1.0],
        }
    }
}
