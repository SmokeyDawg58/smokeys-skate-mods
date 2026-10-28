//! Constructor state of Iris scalar/angle trackers; runtime tuning stays unknown.
#[path = "camera_tracker_angle_update.rs"]
pub(crate) mod angle_update;
#[path = "camera_tracker_update.rs"]
pub(crate) mod update;
#[path = "camera_tracker_vector_update.rs"]
pub(crate) mod vector_update;

pub(crate) struct CameraTracker<T> {
    pub(crate) input_position: T,
    pub(crate) output_position: T,
    pub(crate) velocity: T,
    pub(crate) acceleration: T,
    pub(crate) acceleration_clamp: f32,
    pub(crate) smoothing: f32,
    pub(crate) parameters: TrackerParameters,
}
pub(crate) struct TrackerParameters {
    pub(crate) update_time: f32,
    pub(crate) umbra: Option<f32>,
    pub(crate) penumbra: Option<f32>,
    pub(crate) speed_clamp: Option<f32>,
    pub(crate) acceleration_min: Option<f32>,
    pub(crate) acceleration_max: Option<f32>,
    pub(crate) smoothing_min: Option<f32>,
    pub(crate) smoothing_max: Option<f32>,
    pub(crate) overshoot_zeroes_velocity: bool,
}
impl<T: Copy> CameraTracker<T> {
    /// Native constructor only writes time/overshoot in Parameters. The caller
    /// selects the source position (0,1,.2 or a normalized angle), not tuning.
    pub(crate) fn constructed(position: T, zero: T) -> Self {
        Self {
            input_position: position,
            output_position: position,
            velocity: zero,
            acceleration: zero,
            acceleration_clamp: 0.0,
            smoothing: 0.0,
            parameters: TrackerParameters {
                update_time: f32::from_bits(0x3c88_8889),
                umbra: None,
                penumbra: None,
                speed_clamp: None,
                acceleration_min: None,
                acceleration_max: None,
                smoothing_min: None,
                smoothing_max: None,
                overshoot_zeroes_velocity: true,
            },
        }
    }
}
