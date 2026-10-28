//! Typed retained Grind constructor payloads of the seven manager children.
use super::GrindBindings;
use std::sync::Arc;

pub(crate) struct GrindCollision {
    pub(crate) bindings: GrindBindings,
    /// SAME result owner shared by Manager, Materials and Investigator.
    /// Count starts zero; uninitialized slots and lower flag bits stay unknown.
    pub(crate) result: std::sync::Mutex<crate::grind_data_runtime::GrindDataQueryResult>,
}
pub(crate) struct GrindMaterials {
    pub(crate) bindings: GrindBindings,
    pub(crate) collision: Option<Arc<GrindCollision>>,
    pub(crate) was_targeting: bool,
}
pub(crate) struct GrindInvestigator {
    pub(crate) bindings: GrindBindings,
    pub(crate) collision: Option<Arc<GrindCollision>>,
    pub(crate) avoid_grind_frames: i32,
    pub(crate) avoid_five_o_frames: i32,
    pub(crate) ground_frames: i32,
    pub(crate) ground_to_boardslide_timer: i32,
    pub(crate) non_ground_frames: i32,
    pub(crate) hang_primitive_existed: bool,
    pub(crate) last_velocity: [f32; 4],
}
pub(crate) struct GrindBalance {
    pub(crate) bindings: GrindBindings,
    pub(crate) time_in_droppable_grind: f32,
    pub(crate) exit_tilt_angle: f32,
    pub(crate) exit_start_time: Option<f32>,
    pub(crate) non_grind_frames: Option<i32>,
    pub(crate) grind_normal: [f32; 4],
    pub(crate) high_side_direction: [f32; 4],
}
pub(crate) struct GrindEngagement {
    pub(crate) bindings: GrindBindings,
    pub(crate) tipslide_grab_timer: i32,
}
pub(crate) struct GrindControls {
    pub(crate) bindings: GrindBindings,
    pub(crate) lr: f32,
    pub(crate) ud: f32,
}
pub(crate) struct GrindJumper {
    pub(crate) bindings: GrindBindings,
    pub(crate) just_jumped: bool,
    pub(crate) frames_avoiding: i32,
    pub(crate) geometry_type: i32,
    pub(crate) last_grind_type: i32,
    pub(crate) energy_fraction: f32,
    pub(crate) high_side_direction: [f32; 4],
    pub(crate) grind_normal: [f32; 4],
    pub(crate) grind_direction: [f32; 4],
    pub(crate) most_vertical_normal: [f32; 4],
    pub(crate) primitive_location: [f32; 4],
}
impl GrindJumper {
    pub(crate) fn new(bindings: GrindBindings) -> Self {
        Self {
            bindings,
            just_jumped: false,
            frames_avoiding: 0,
            geometry_type: 0,
            last_grind_type: 3,
            energy_fraction: 1.0,
            high_side_direction: [1.0, 0.0, 0.0, 0.0],
            grind_normal: [0.0, 1.0, 0.0, 0.0],
            grind_direction: [1.0, 0.0, 0.0, 0.0],
            most_vertical_normal: [0.0, 1.0, 0.0, 0.0],
            primitive_location: [0.0; 4],
        }
    }
}
