//! Animation conditioner82e2fdc0 and Reset82e2ef78, including retained copied
//! controller tuning and the two actual debug allocations.
use super::{
    super::{RIGHT, SkeletonRootVector, ZERO, trick::AnimOutTrickData},
    simple::ScalarPid,
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    reckoning_pid::PhysicsControllerData,
};
use std::sync::Arc;

#[path = "../../conditioning/physical_player_conditioner_animation_turn.rs"]
pub(crate) mod turn;
#[path = "../../conditioning/physical_player_conditioner_animation_updates.rs"]
pub(crate) mod updates;

pub(crate) struct AnimationAttributes {
    global: AttributeRuntimeInstance,
}
impl AnimationAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            global: vault.instance(0x74bc_9c93_affe_6aac, 0xd7ed_bd36_2d7d_2152),
        }
    }
    /// Each source PID construction separately reads the current retained global
    /// instance's layout; the PID copies its four values and holds no pointer.
    pub(super) fn control(
        &self,
        index: usize,
    ) -> Result<PhysicsControllerData, AttributeRuntimeError> {
        let data = self.global.try_cached_layout(0x220)?;
        let values = if let Some(data) = data {
            std::array::from_fn(|lane| {
                let offset = index * 16 + lane * 4;
                f32::from_be_bytes(
                    data.bytes()[offset..offset + 4]
                        .try_into()
                        .expect("bounded controller data"),
                )
            })
        } else {
            [0.0; 4]
        };
        Ok(PhysicsControllerData {
            proportional: values[0],
            integral: values[1],
            derivative: values[2],
            integral_blend: values[3],
        })
    }
}

pub(super) struct Animation {
    pub(super) previous_frame_trick: AnimOutTrickData,
    pub(super) last_nose_direction: SkeletonRootVector,
    pub(super) last_body_tilt: SkeletonRootVector,
    pub(super) last_animation_lean: f32,
    pub(super) last_actual_lean: f32,
    pub(super) input_quickness: f32,
    pub(super) last_normalized_quickness: f32,
    pub(super) last_holding: f32,
    pub(super) holding_integral: f32,
    pub(super) last_holding_delta: f32,
    pub(super) last_output_lean: f32,
    pub(super) direction_filter: Option<ScalarPid>,
    pub(super) lean_filter: Option<ScalarPid>,
    pub(super) slow_lean_filter: Option<ScalarPid>,
    // DebugGraphXY's release-build PDB class has no payload fields. Presence
    // records the actual one-byte allocation result; this is not a graph API.
    pub(super) debug_graphs: [Option<Arc<()>>; 2],
}
impl Animation {
    pub(super) fn before_filters() -> Self {
        Self {
            previous_frame_trick: AnimOutTrickData::reset_values(),
            last_nose_direction: ZERO,
            last_body_tilt: ZERO,
            last_animation_lean: 0.0,
            last_actual_lean: 0.0,
            input_quickness: 0.0,
            last_normalized_quickness: 0.0,
            last_holding: 0.0,
            holding_integral: 0.0,
            last_holding_delta: 0.0,
            last_output_lean: 0.0,
            direction_filter: None,
            lean_filter: None,
            slow_lean_filter: None,
            debug_graphs: [None, None],
        }
    }
    /// Caller has completed all three filter constructors and debug allocations.
    pub(super) fn reset(&mut self) {
        self.last_nose_direction = RIGHT;
        self.last_body_tilt = ZERO;
        self.last_animation_lean = 0.0;
        self.last_actual_lean = 0.0;
        self.input_quickness = 0.0;
        self.last_normalized_quickness = 0.0;
        self.last_holding = 0.0;
        self.holding_integral = 0.0;
        self.last_holding_delta = 0.0;
        self.last_output_lean = 0.0;
        self.direction_filter
            .as_mut()
            .expect("source completed direction filter")
            .reset();
        self.lean_filter
            .as_mut()
            .expect("source completed lean filter")
            .reset();
        self.slow_lean_filter
            .as_mut()
            .expect("source completed slow lean filter")
            .reset();
        self.previous_frame_trick = AnimOutTrickData::reset_values();
    }
}
