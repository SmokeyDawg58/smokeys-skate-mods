//! Actual `WipeoutGround` constructor82d9fcb8. Entry is separately scheduled.
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    physical_player_footplant::{FootPlantPrediction, RetainedTrajectory},
    physical_player_physics_state::PhysicalStateBindings,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

pub(crate) struct PhysicsWipeoutGroundState {
    pub(crate) bindings: PhysicalStateBindings,
    attributes: AttributeRuntimeInstance,
    pub(crate) fields: WipeoutGroundFields,
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "distinct native WipeoutGround entry flags"
)]
pub(crate) struct WipeoutGroundFields {
    pub(crate) time_in_state: f32,
    pub(crate) time_settled: f32,
    pub(crate) time_until_teleport: f32,
    pub(crate) time_impaled: f32,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained constructor direction awaits WipeoutGround Update"
        )
    )]
    pub(crate) direction: [f32; 4],
    pub(crate) impulse: Option<SkeletonRootVector>,
    pub(crate) body_angular_velocity: SkeletonRootVector,
    pub(crate) over: bool,
    pub(crate) slow: bool,
    pub(crate) request_teleport: bool,
    pub(crate) is_impulse: Option<bool>,
    pub(crate) time_over: f32,
    pub(crate) end_drive_weight: f32,
    pub(crate) controlled_drive_weight: f32,
    pub(crate) impulse_frames: Option<i32>,
    pub(crate) prevent_respawn: bool,
    pub(crate) ignore_reset_button: bool,
    pub(crate) reset_ever_pressed: bool,
    pub(crate) ever_at_rest: bool,
    pub(crate) enough_time_for_reset: bool,
    pub(crate) auto_teleport_pending: bool,
    pub(crate) has_been_impaled: bool,
    pub(crate) prediction: FootPlantPrediction,
    pub(crate) estimated_time_to_collision: f32,
}
impl PhysicsWipeoutGroundState {
    pub(crate) fn new(bindings: PhysicalStateBindings, vault: &Arc<AttributeRuntimeVault>) -> Self {
        let attributes = vault.instance(
            crate::skeleton_body_attributes::WIPEOUT_CLASS,
            crate::skeleton_body_attributes::DEFAULT_COLLECTION,
        );
        Self {
            bindings,
            attributes,
            fields: WipeoutGroundFields::constructed(),
        }
    }
}
#[cfg(test)]
#[path = "physical_player_wipeout_post_solve_tests.rs"]
mod post_solve_tests;

impl WipeoutGroundFields {
    pub(crate) fn constructed() -> Self {
        Self {
            time_in_state: 0.0,
            time_settled: 0.0,
            time_until_teleport: 0.0,
            time_impaled: 0.0,
            direction: [0.0; 4],
            impulse: None,
            body_angular_velocity: SkeletonRootVector::known([0.0; 4]),
            over: false,
            slow: false,
            request_teleport: false,
            is_impulse: None,
            time_over: 0.0,
            end_drive_weight: 0.0,
            controlled_drive_weight: 0.0,
            impulse_frames: None,
            prevent_respawn: false,
            ignore_reset_button: false,
            reset_ever_pressed: false,
            ever_at_rest: false,
            enough_time_for_reset: false,
            auto_teleport_pending: false,
            has_been_impaled: false,
            prediction: FootPlantPrediction {
                ready: false,
                trajectory: RetainedTrajectory::constructed(),
                collision_position: None,
                collision_normal: None,
                ground_normal: None,
                collision_time: None,
                frame: None,
                surface: None,
                radius: None,
                max_error_start: None,
                max_error_end: None,
                draw_true: None,
            },
            estimated_time_to_collision: f32::MAX,
        }
    }
    /// Entry82d8eb70..ebb8 preserves direction/impulse/flags later overwritten.
    pub(crate) fn enter_prefix(&mut self) {
        self.time_in_state = 0.0;
        self.request_teleport = false;
        self.time_settled = 0.0;
        self.over = false;
        self.time_until_teleport = 0.0;
        self.slow = false;
        self.time_impaled = 0.0;
        self.prevent_respawn = false;
        self.time_over = 0.0;
        self.ignore_reset_button = false;
        self.end_drive_weight = 0.0;
        self.reset_ever_pressed = false;
        self.controlled_drive_weight = 0.0;
        self.ever_at_rest = false;
        self.estimated_time_to_collision = f32::MAX;
        self.enough_time_for_reset = false;
        self.auto_teleport_pending = false;
        self.has_been_impaled = false;
        self.body_angular_velocity = SkeletonRootVector::known([0.0; 4]);
    }
}

impl PhysicsWipeoutGroundState {
    pub(crate) fn try_update_scalar(
        &self,
        key: u64,
    ) -> Result<f32, crate::attribute_runtime::AttributeRuntimeError> {
        let Some(data) = self.attributes.try_get_data(key, 0)? else {
            return Ok(0.0);
        };
        let bytes = data.bytes();
        let bytes: [u8; 4] = bytes.try_into().map_err(|_| {
            crate::attribute_runtime::AttributeRuntimeError::PayloadSize {
                key,
                expected: 4,
                actual: bytes.len(),
            }
        })?;
        Ok(f32::from_be_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_ctor_stack_stores_prove_zero_vectors_and_minus_one_time() {
        let mut value = WipeoutGroundFields::constructed();
        assert_eq!(value.direction.map(f32::to_bits), [0; 4]);
        assert_eq!(value.body_angular_velocity.xyz.map(f32::to_bits), [0; 3]);
        assert_eq!(value.body_angular_velocity.w.map(f32::to_bits), Some(0));
        assert_eq!(
            value.prediction.trajectory.time.map(f32::to_bits),
            [(-1.0_f32).to_bits(); 4]
        );
        assert_eq!(value.impulse, None);
        assert_eq!(value.is_impulse, None);
        assert_eq!(value.impulse_frames, None);
        value.direction = [2.0; 4];
        value.impulse = Some(SkeletonRootVector::known([3.0; 4]));
        value.prediction.ready = true;
        value.enter_prefix();
        assert_eq!(value.direction.map(f32::to_bits), [2.0_f32.to_bits(); 4]);
        assert_eq!(value.impulse, Some(SkeletonRootVector::known([3.0; 4])));
        assert!(value.prediction.ready);
    }
}

#[cfg(test)]
#[path = "physical_player_wipeout_recovery_tests.rs"]
mod recovery_tests;

#[cfg(test)]
#[path = "physical_player_wipeout_prediction_tests.rs"]
mod prediction_tests;
