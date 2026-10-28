//! Typed publication of `Iris::GameCameraParameters` by `ReadAttribs825835a8`.
//! Every field starts unknown; failed reads preserve the earlier native stores.
use crate::{physics::PointGraph, skeleton_root_transforms::SkeletonRootMatrix};

#[derive(Default)]
pub(crate) struct CameraParameters {
    /// Native parameter offset 0x0.
    pub(crate) shake_amplitude_curve: Option<SkeletonRootMatrix>,
    /// Native parameter offset 0x40.
    pub(crate) shake_frequency_curve: Option<SkeletonRootMatrix>,
    /// Native parameter offset 0x80.
    pub(crate) drop_elevation_curve: Option<PointGraph<8>>,
    /// Native parameter offset 0xc0.
    pub(crate) drop_smoothing_curve: Option<PointGraph<8>>,
    /// Native parameter offset 0x100.
    pub(crate) incline_smoothing_curve: Option<PointGraph<8>>,
    /// Native parameter offset 0x140.
    pub(crate) push_lag_curve: Option<PointGraph<8>>,
    /// Native parameter offset 0x180.
    pub(crate) pan_smoothing_curve: Option<PointGraph<8>>,
    /// Native parameter offset 0x1c0.
    pub(crate) pump_lag_curve: Option<PointGraph<8>>,
    /// Native parameter offset 0x200.
    pub(crate) rig_y_offset: Option<f32>,
    /// Native parameter offset 0x204.
    pub(crate) heading_speed_clamp: Option<f32>,
    /// Native parameter offset 0x208.
    pub(crate) heading_acceleration_clamp_min: Option<f32>,
    /// Native parameter offset 0x20c.
    pub(crate) heading_acceleration_clamp_max: Option<f32>,
    /// Native parameter offset 0x210.
    pub(crate) heading_delta_umbra: Option<f32>,
    /// Native parameter offset 0x214.
    pub(crate) heading_delta_penumbra: Option<f32>,
    /// Native parameter offset 0x218.
    pub(crate) distance_speed_clamp: Option<f32>,
    /// Native parameter offset 0x21c.
    pub(crate) distance_smoothing: Option<f32>,
    /// Native parameter offset 0x220.
    pub(crate) distance_acceleration_clamp_max: Option<f32>,
    /// Native parameter offset 0x224.
    pub(crate) elevation_speed_clamp: Option<f32>,
    /// Native parameter offset 0x228.
    pub(crate) elevation_acceleration_clamp_min: Option<f32>,
    /// Native parameter offset 0x22c.
    pub(crate) elevation_acceleration_clamp_max: Option<f32>,
    /// Native parameter offset 0x230.
    pub(crate) elevation_delta_umbra: Option<f32>,
    /// Native parameter offset 0x234.
    pub(crate) elevation_delta_penumbra: Option<f32>,
    /// Native parameter offset 0x238.
    pub(crate) tracker_subject_acceleration_for_max_smoothing: Option<f32>,
    /// Native parameter offset 0x23c.
    pub(crate) tracker_subject_smoothing_duration: Option<f32>,
    /// Native parameter offset 0x240.
    pub(crate) tracker_subject_max_speed: Option<f32>,
    /// Native parameter offset 0x244.
    pub(crate) tracker_subject_max_acceleration: Option<f32>,
    /// Native parameter offset 0x248.
    pub(crate) push_lag_duration: Option<f32>,
    /// Native parameter offset 0x24c.
    pub(crate) push_lag_max_smoothing: Option<f32>,
    /// Native parameter offset 0x250.
    pub(crate) heading_change_response_speed: Option<f32>,
    /// Native parameter offset 0x254.
    pub(crate) mirroring_speed: Option<f32>,
    /// Native parameter offset 0x258.
    pub(crate) max_pitch: Option<f32>,
    /// Native parameter offset 0x25c.
    pub(crate) turning_centred_deadzone: Option<f32>,
    /// Native parameter offset 0x260.
    pub(crate) collision_interpolation_duration: Option<f32>,
    /// Native parameter offset 0x264.
    pub(crate) pumping_min_acceleration: Option<f32>,
    /// Native parameter offset 0x268.
    pub(crate) avoidance_lookahead: Option<f32>,
    /// Native parameter offset 0x26c.
    pub(crate) avoidance_heading_smoothing: Option<f32>,
    /// Native parameter offset 0x270.
    pub(crate) avoidance_elevation_smoothing: Option<f32>,
    /// Native parameter offset 0x274.
    pub(crate) avoidance_ease_out: Option<f32>,
    /// Native parameter offset 0x278.
    pub(crate) dutch_interpolation_speed: Option<f32>,
    /// Native parameter offset 0x27c.
    pub(crate) drop_elevation_offset: Option<f32>,
    /// Native parameter offset 0x280.
    pub(crate) drop_min_test_distance: Option<f32>,
    /// Native parameter offset 0x284.
    pub(crate) drop_total_test_time: Option<f32>,
    /// Native parameter offset 0x288.
    pub(crate) max_look_ahead_distance: Option<f32>,
    /// Native parameter offset 0x28c.
    pub(crate) drop_min_drop_distance: Option<f32>,
    /// Native parameter offset 0x290.
    pub(crate) drop_max_drop_distance: Option<f32>,
    /// Native parameter offset 0x294.
    pub(crate) drop_tracker_speed_clamp: Option<f32>,
    /// Native parameter offset 0x298.
    pub(crate) drop_tracker_acceleration_clamp: Option<f32>,
    /// Native parameter offset 0x29c.
    pub(crate) drop_tracker_smoothing: Option<f32>,
    /// Native parameter offset 0x2a0.
    pub(crate) drop_min_actor_velocity: Option<f32>,
    /// Native parameter offset 0x2a4.
    pub(crate) grind_min_lookahead_distance: Option<f32>,
    /// Native parameter offset 0x2a8.
    pub(crate) grind_max_lookahead_distance: Option<f32>,
    /// Native parameter offset 0x2ac.
    pub(crate) grind_lookahead_time: Option<f32>,
    /// Native parameter offset 0x2b0.
    pub(crate) grind_frustum_angle_horizontal: Option<f32>,
    /// Native parameter offset 0x2b4.
    pub(crate) grind_frustum_angle_vertical: Option<f32>,
    /// Native parameter offset 0x2b8.
    pub(crate) grind_max_relative_angle: Option<f32>,
    /// Native parameter offset 0x2bc.
    pub(crate) grind_opacity: Option<f32>,
    /// Native parameter offset 0x2c0.
    pub(crate) grind_frustum_pitch_offset: Option<f32>,
    /// Native parameter offset 0x2c4.
    pub(crate) grind_skateboard_offset: Option<f32>,
    /// Native parameter offset 0x2c8.
    pub(crate) grind_fade_in_speed: Option<f32>,
    /// Native parameter offset 0x2cc.
    pub(crate) grind_fade_out_speed: Option<f32>,
    /// Native parameter offset 0x2d0.
    pub(crate) grind_skater_offset: Option<f32>,
    /// Native parameter offset 0x2d4.
    pub(crate) grind_max_relative_height: Option<f32>,
    /// Native parameter offset 0x2d8.
    pub(crate) shake_grind_magnitude: Option<f32>,
    /// Native parameter offset 0x2dc.
    pub(crate) shake_one_shot_magnitude: Option<f32>,
    /// Native parameter offset 0x2e0.
    pub(crate) shake_one_shot_min_velocity: Option<f32>,
    /// Native parameter offset 0x2e4.
    pub(crate) shake_one_shot_max_velocity: Option<f32>,
    /// Native parameter offset 0x2e8.
    pub(crate) shake_one_shot_frequency: Option<f32>,
    /// Native parameter offset 0x2ec.
    pub(crate) shake_one_shot_decay: Option<f32>,
    /// Native parameter offset 0x2f0.
    pub(crate) shake_amplitude_min: Option<f32>,
    /// Native parameter offset 0x2f4.
    pub(crate) shake_amplitude_max: Option<f32>,
    /// Native parameter offset 0x2f8.
    pub(crate) shake_amplitude_top_skater_speed: Option<f32>,
    /// Native parameter offset 0x2fc.
    pub(crate) shake_frequency_min: Option<f32>,
    /// Native parameter offset 0x300.
    pub(crate) shake_frequency_max: Option<f32>,
    /// Native parameter offset 0x304.
    pub(crate) shake_frequency_top_skater_speed: Option<f32>,
    /// Native parameter offset 0x308.
    pub(crate) shake_data_fps: Option<f32>,
    /// Native parameter offset 0x30c.
    pub(crate) shake_translation_multiplier: Option<f32>,
    /// Native parameter offset 0x310.
    pub(crate) shake_rotation_multiplier: Option<f32>,
    /// Native parameter offset 0x314.
    pub(crate) shake_dutch_multiplier: Option<f32>,
    /// Native parameter offset 0x318.
    pub(crate) pump_lag_duration: Option<f32>,
    /// Native parameter offset 0x31c.
    pub(crate) pump_lag_max_smoothing: Option<f32>,
    /// Native parameter offset 0x320.
    pub(crate) pan_umbra: Option<f32>,
    /// Native parameter offset 0x324.
    pub(crate) pan_penumbra: Option<f32>,
    /// Native parameter offset 0x328.
    pub(crate) pan_min_smoothing: Option<f32>,
    /// Native parameter offset 0x32c.
    pub(crate) pan_smoothing_min: Option<f32>,
    /// Native parameter offset 0x330.
    pub(crate) pan_smoothing_max: Option<f32>,
    /// Native parameter offset 0x334.
    pub(crate) tilt_speed_clamp: Option<f32>,
    /// Native parameter offset 0x338.
    pub(crate) tilt_acceleration_clamp_min: Option<f32>,
    /// Native parameter offset 0x33c.
    pub(crate) tilt_smoothing_min: Option<f32>,
    /// Native parameter offset 0x340.
    pub(crate) tilt_smoothing_max: Option<f32>,
    /// Native parameter offset 0x344.
    pub(crate) tilt_acceleration_clamp_max: Option<f32>,
    /// Native parameter offset 0x348.
    pub(crate) tilt_delta_umbra: Option<f32>,
    /// Native parameter offset 0x34c.
    pub(crate) tilt_delta_penumbra: Option<f32>,
    /// Native parameter offset 0x350.
    pub(crate) pan_speed_clamp: Option<f32>,
    /// Native parameter offset 0x354.
    pub(crate) pan_acceleration_clamp_min: Option<f32>,
    /// Native parameter offset 0x358.
    pub(crate) pan_acceleration_clamp_max: Option<f32>,
    /// Native parameter offset 0x35c.
    pub(crate) pan_delta_umbra: Option<f32>,
    /// Native parameter offset 0x360.
    pub(crate) pan_delta_penumbra: Option<f32>,
    /// Native parameter offset 0x364.
    pub(crate) compass_time_before_auto_lineup: Option<f32>,
    /// Native parameter offset 0x368.
    pub(crate) compass_auto_lineup_speed: Option<f32>,
    /// Native parameter offset 0x36c.
    pub(crate) compass_min_deadzone_speed: Option<f32>,
    /// Native parameter offset 0x370.
    pub(crate) compass_max_deadzone_speed: Option<f32>,
    /// Native parameter offset 0x374.
    pub(crate) compass_max_deadzone_size: Option<f32>,
    /// Native parameter offset 0x378.
    pub(crate) compass_deadzone_size_smoothing: Option<f32>,
    /// Native parameter offset 0x37c.
    pub(crate) positioner_distance_smoothing: Option<f32>,
    /// Native parameter offset 0x380.
    pub(crate) positioner_min_distance: Option<f32>,
    /// Native parameter offset 0x384.
    pub(crate) positioner_min_distance_during_wipeout: Option<f32>,
    /// Native parameter offset 0x388.
    pub(crate) positioner_camera_radius: Option<f32>,
    /// Native parameter offset 0x38c.
    pub(crate) positioner_avoidance_collision_offset: Option<f32>,
    /// Native parameter offset 0x390.
    pub(crate) near_clipping_distance_min: Option<f32>,
    /// Native parameter offset 0x394.
    pub(crate) near_clipping_distance_max: Option<f32>,
    /// Native parameter offset 0x398.
    pub(crate) near_clipping_distance_step: Option<f32>,
    /// Native parameter offset 0x39c.
    pub(crate) free_cam_elevation_offset: Option<f32>,
    /// Native parameter offset 0x3a0.
    pub(crate) free_cam_elevation_smoothing: Option<f32>,
    /// Native parameter offset 0x3a4.
    pub(crate) free_cam_elevation_speed: Option<f32>,
    /// Native parameter offset 0x3a8.
    pub(crate) free_cam_heading_offset: Option<f32>,
    /// Native parameter offset 0x3ac.
    pub(crate) free_cam_heading_smoothing: Option<f32>,
    /// Native parameter offset 0x3b0.
    pub(crate) free_cam_heading_speed: Option<f32>,
    /// Native parameter offset 0x3b4.
    pub(crate) blend_function: Option<i32>,
    /// Host publication evidence only; no native memory field is invented.
    pub(crate) read_completed: bool,
}
