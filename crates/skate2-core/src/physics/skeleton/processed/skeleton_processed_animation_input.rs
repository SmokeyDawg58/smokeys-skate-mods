//! Same processed owner: `PhysicalPlayerHiLOD::ProcessAnimOut` 82e00030.
use super::SkeletonProcessedContact;
use crate::actor_construction::anim_out_phys_in::{AnimOutConstructionStage, AnimOutPhysInOwner};
use crate::skeleton_root_transforms::SkeletonRootVector;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProcessedAnimationInput {
    pub(crate) compression: Option<f32>,
    pub(crate) living_world_impulse: Option<[f32; 4]>,
    pub(crate) external_reset_transform: [Option<SkeletonRootVector>; 4],
    pub(crate) external_reset_on_board: Option<bool>,
    pub(crate) truck_tightness: Option<f32>,
    pub(crate) wheel_hardness: Option<f32>,
}
impl ProcessedAnimationInput {
    pub(super) fn initialize_sim_residuals(&mut self) {
        self.living_world_impulse = Some([0.0; 4]);
        self.external_reset_transform = [Some(SkeletonRootVector::known([0.0; 4])); 4];
        self.external_reset_on_board = Some(false);
    }
    pub(super) const fn new() -> Self {
        Self {
            compression: None,
            living_world_impulse: None,
            external_reset_transform: [None; 4],
            external_reset_on_board: None,
            truck_tightness: None,
            wheel_hardness: None,
        }
    }
    pub(super) fn reset(&mut self) {
        self.compression = Some(0.0); //82b39454
        self.truck_tightness = Some(0.0); //82b39514
        self.wheel_hardness = Some(0.0); //82b3951c
        // Neither Reset nor Processed ctor writes +5b0 or +5c0..600.
        // Their retained values, including initial unknowns, survive Reset.
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ProcessedAnimationInputError {
    InputNotConstructed,
    UnknownRidingSwitchWord,
}
impl std::fmt::Display for ProcessedAnimationInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputNotConstructed => f.write_str("animation physics input not constructed"),
            Self::UnknownRidingSwitchWord => f.write_str("processed riding-switch word unknown"),
        }
    }
}
impl SkeletonProcessedContact {
    pub(crate) const fn animation_input(&self) -> &ProcessedAnimationInput {
        &self.animation_input
    }
    /// Caller supplies the actual retained +2b4 input. This does not run Update1.
    /// All copies are unconditional, including inactive impulse/reset payloads.
    pub(crate) fn try_process_animation_input(
        &mut self,
        input: &AnimOutPhysInOwner,
    ) -> Result<(), ProcessedAnimationInputError> {
        if input.stage != AnimOutConstructionStage::Returned {
            return Err(ProcessedAnimationInputError::InputNotConstructed);
        }
        self.jump_events
            .process_board_flip_byte(u8::from(input.flags.board_flipped)); //00058
        self.physics_state.time_delta = Some(input.time_delta); //00068
        self.animation_input.compression = Some(input.compression); //00078
        self.jump_events
            .process_animation_flag(8, input.flags.living_world_impulse_active); //00090
        self.animation_input.living_world_impulse = Some(input.living_world_impulse); //000a0
        self.jump_events
            .process_animation_flag(4, input.external_reset_active); //000b8
        // 000cc..e8: four alternating SIMD loads/stores, in address order.
        for column in 0..4 {
            self.animation_input.external_reset_transform[column] =
                Some(input.external_reset_transform.columns()[column]);
        }
        self.animation_input.external_reset_on_board = Some(input.external_reset_on_board); //000f0
        self.animation_input.truck_tightness = Some(input.truck_tightness); //00100
        self.animation_input.wheel_hardness = Some(input.wheel_hardness); //00110
        // Unknown unrelated bits cannot be invented to complete this final RMW.
        let old = self
            .physics_state
            .flags_8cc
            .ok_or(ProcessedAnimationInputError::UnknownRidingSwitchWord)?;
        self.physics_state.flags_8cc =
            Some((old & !0x0040_0000) | (u32::from(input.flags.riding_switch) << 22)); //00128
        Ok(())
    }
}

#[cfg(test)]
#[path = "skeleton_processed_animation_input_tests.rs"]
mod tests;
