//! Concrete small conditioner payloads. Source Sim allocation supplies zero
//! storage, while explicit resets preserve the named histories documented below.
use super::super::{FORWARD, RIGHT, SkeletonRootMatrix, SkeletonRootVector, UP, ZERO};
use crate::{
    physics::{FilteredPhysicsState, GrindType},
    play_animation::{EMPTY_FAST_STRING_30, FastString30},
    reckoning_pid::PhysicsControllerData,
};

/// `PhysOutConditioner_Physics`82e2f8f0 / Reset82e2d6d0.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "retain independent original physics conditioner flags"
)]
pub(super) struct Physics {
    pub(super) damped_system_velocity: SkeletonRootVector,
    pub(super) deck_position: SkeletonRootVector,
    pub(super) last_deck_position: SkeletonRootVector,
    pub(super) velocity: SkeletonRootVector,
    pub(super) last_grind_position: SkeletonRootVector,
    pub(super) last_damped_com_position: SkeletonRootVector,
    pub(super) heavily_damped_com_velocity: SkeletonRootVector,
    pub(super) accumulated_com_error: SkeletonRootVector,
    pub(super) last_filtered_state: FilteredPhysicsState,
    pub(super) filtered_state: FilteredPhysicsState,
    pub(super) last_physics_state: i32,
    pub(super) grind_type: GrindType,
    pub(super) grind_scorable_id: i32,
    pub(super) frames_since_reset: i32,
    pub(super) air_count: i32,
    pub(super) nonspecific_count: i32,
    pub(super) nonspecific_collision_free_count: i32,
    pub(super) nonspecific_collision_count: i32,
    pub(super) frames_since_ground_stairs: i32,
    pub(super) must_change_filtered_state: bool,
    pub(super) is_grind_on_front: bool,
    pub(super) was_grinding: bool,
    pub(super) last_smoothed_com_position_valid: bool,
    pub(super) current_grind_distance: f32,
    pub(super) last_grind_distance: f32,
    pub(super) grind_crouch: f32,
    pub(super) grind_pathed_guid: u64,
    pub(super) grind_local_guid: u64,
    pub(super) skating_grind_name: FastString30,
    pub(super) scoring_grind_name: FastString30,
}
impl Physics {
    pub(super) fn reset_values() -> Self {
        Self {
            damped_system_velocity: ZERO,
            deck_position: ZERO,
            last_deck_position: ZERO,
            velocity: ZERO,
            last_grind_position: ZERO,
            last_damped_com_position: ZERO,
            heavily_damped_com_velocity: ZERO,
            accumulated_com_error: ZERO,
            last_filtered_state: FilteredPhysicsState::Invalid,
            filtered_state: FilteredPhysicsState::Invalid,
            last_physics_state: 0,
            grind_type: GrindType::None,
            grind_scorable_id: -1,
            frames_since_reset: 0,
            air_count: 0,
            nonspecific_count: 0,
            nonspecific_collision_free_count: 0,
            nonspecific_collision_count: 0,
            frames_since_ground_stairs: 0,
            must_change_filtered_state: true,
            is_grind_on_front: false,
            was_grinding: false,
            last_smoothed_com_position_valid: false,
            current_grind_distance: 0.0,
            last_grind_distance: 0.0,
            grind_crouch: 0.0,
            grind_pathed_guid: u64::MAX,
            grind_local_guid: u64::MAX,
            skating_grind_name: EMPTY_FAST_STRING_30,
            scoring_grind_name: EMPTY_FAST_STRING_30,
        }
    }
}

/// `PIDFilter<float>`82e48e08 copies its control data; Reset preserves it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ScalarPid {
    pub(super) control: PhysicsControllerData,
    pub(super) value: f32,
    pub(super) last_target: f32,
    pub(super) integral: f32,
    pub(super) derivative: f32,
    pub(super) derivative_input: f32,
}
impl ScalarPid {
    pub(super) fn new(control: PhysicsControllerData) -> Self {
        Self {
            control,
            value: 0.0,
            last_target: 0.0,
            integral: 0.0,
            derivative: 0.0,
            derivative_input: 0.0,
        }
    }
    pub(super) fn reset(&mut self) {
        self.value = 0.0;
        self.last_target = 0.0;
        self.integral = 0.0;
        self.derivative = 0.0;
        self.derivative_input = 0.0;
    }
}

/// `PhysOutConditioner_GameCam`82e2cf80 and its `PopSmoother82e2c538`.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct GameCam {
    pub(super) error: SkeletonRootVector,
    pub(super) current: SkeletonRootVector,
    pub(super) previous: SkeletonRootVector,
    pub(super) zero: SkeletonRootVector,
    pub(super) smoothness: f32,
    pub(super) last_primitive_midpoint: [f32; 3],
    pub(super) last_grind_type: GrindType,
    pub(super) was_grinding: bool,
}
impl GameCam {
    /// Reset only the five native grind/midpoint fields; retain smoothing.
    pub(super) fn reset(&mut self) {
        self.was_grinding = false;
        self.last_grind_type = GrindType::None;
        self.last_primitive_midpoint = [0.0; 3];
    }
    pub(super) fn new() -> Self {
        Self {
            error: ZERO,
            current: ZERO,
            previous: ZERO,
            zero: ZERO,
            smoothness: f32::from_bits(0x3f73_3333),
            last_primitive_midpoint: [0.0; 3],
            last_grind_type: GrindType::None,
            was_grinding: false,
        }
    }
}

/// `PhysOutConditioner_Audio`827c6188 / Reset827c4b28. The constructor samples
/// `is_wiping_out` from the real output before Reset; Reset preserves that sample.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "retain independent original audio contact flags"
)]
pub(super) struct Audio {
    pub(super) wheel_in_contact: [bool; 4],
    pub(super) frames_in_air: [i32; 4],
    pub(super) wheel_velocity: [SkeletonRootVector; 4],
    pub(super) deck_transform: SkeletonRootMatrix,
    pub(super) torso_angular_velocity: SkeletonRootVector,
    pub(super) skeleton_collision: [super::super::simple::CollisionAudio; 4],
    pub(super) collision_index: i32,
    pub(super) board_collision: [f32; 4],
    pub(super) board_collision_index: i32,
    pub(super) last_grind_impact_speed: f32,
    pub(super) is_wiping_out: bool,
    pub(super) grind_type: GrindType,
    pub(super) grind_audio_surface_type: i32,
    pub(super) com_y_velocity: [f32; 4],
    pub(super) com_index: i32,
    pub(super) last_left_foot_ground_position: SkeletonRootVector,
    pub(super) last_right_foot_ground_position: SkeletonRootVector,
    pub(super) left_foot_surface: u16,
    pub(super) right_foot_surface: u16,
    pub(super) left_foot_touching: bool,
    pub(super) left_foot_was_touching: bool,
    pub(super) right_foot_touching: bool,
    pub(super) right_foot_was_touching: bool,
    pub(super) left_foot_on_stairs: bool,
    pub(super) right_foot_on_stairs: bool,
    pub(super) last_raw_wheel_slides: [f32; 4],
    pub(super) raw_wheel_slide_index: i32,
    pub(super) last_average_raw_wheel_slide: f32,
}
impl Audio {
    pub(super) fn before_output_read() -> Self {
        let mut value = Self::new(false);
        value.wheel_in_contact = [false; 4];
        value.deck_transform = SkeletonRootMatrix::from_columns([ZERO; 4]);
        value.grind_type = GrindType::FiftyFifty;
        value
    }
    pub(super) fn new(is_wiping_out: bool) -> Self {
        Self {
            wheel_in_contact: [true; 4],
            frames_in_air: [0; 4],
            wheel_velocity: [ZERO; 4],
            deck_transform: SkeletonRootMatrix::from_columns([RIGHT, UP, FORWARD, ZERO]),
            torso_angular_velocity: ZERO,
            skeleton_collision: std::array::from_fn(|_| super::super::simple::CollisionAudio {
                contact_force: [0.0; 8],
                slide_speed: [0.0; 8],
                audio_surface_type: [0; 8],
                is_groin_contact: false,
                is_face_contact: false,
                vehicle_impact: 0.0,
                skater_impact: 0.0,
                pedestrian_impact: 0.0,
            }),
            collision_index: 0,
            board_collision: [0.0; 4],
            board_collision_index: 0,
            last_grind_impact_speed: 0.0,
            is_wiping_out,
            grind_type: GrindType::None,
            grind_audio_surface_type: 0,
            com_y_velocity: [0.0; 4],
            com_index: 0,
            last_left_foot_ground_position: ZERO,
            last_right_foot_ground_position: ZERO,
            left_foot_surface: 0,
            right_foot_surface: 0,
            left_foot_touching: false,
            left_foot_was_touching: false,
            right_foot_touching: false,
            right_foot_was_touching: false,
            left_foot_on_stairs: false,
            right_foot_on_stairs: false,
            last_raw_wheel_slides: [0.0; 4],
            raw_wheel_slide_index: 0,
            last_average_raw_wheel_slide: 0.0,
        }
    }
    pub(super) fn reset(&mut self) {
        *self = Self::new(self.is_wiping_out);
    }
}
