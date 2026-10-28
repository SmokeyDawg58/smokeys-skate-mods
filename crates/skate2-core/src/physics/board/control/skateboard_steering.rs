//! `Skateboard::UpdateSteeringTilt` 0x82b29de0, on retained outer-board state.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "grounded steering awaits full Actor scheduling")
)]
use crate::attribute_runtime::{
    AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::sync::Arc;
#[path = "../runtime/skateboard_physics_error.rs"]
pub(crate) mod physics_error;

pub(crate) const STEERING_CLASS: u64 = 0x3b76_fd33_2f0d_4d06;
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
const BLEND: u64 = 0xc324_9a9d_f266_00c9;

pub(crate) trait SkateboardSteeringAttributes {
    type Error;
    fn blend_coefficient(&mut self) -> Result<f32, Self::Error>;
}
pub(crate) struct SkateboardSteeringVaultAttributes(AttributeRuntimeInstance);
impl SkateboardSteeringVaultAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self(vault.instance(STEERING_CLASS, DEFAULT))
    }
}
impl SkateboardSteeringAttributes for SkateboardSteeringVaultAttributes {
    type Error = AttributeRuntimeError;
    fn blend_coefficient(&mut self) -> Result<f32, Self::Error> {
        // The load immediately follows the lookup/default branch at 29e48..60.
        let Some(data) = self.0.try_get_data(BLEND, 0)? else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] =
            data.bytes()
                .try_into()
                .map_err(|_| AttributeRuntimeError::ScalarSize {
                    size: data.bytes().len(),
                })?;
        Ok(f32::from_be_bytes(bytes))
    }
}

pub(crate) trait SkateboardSteeringInput {
    type Processed;
    type Error;
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error>;
    fn flags_8c4(&mut self, processed: &Self::Processed) -> Result<u32, Self::Error>;
    fn flags_8c8(&mut self, processed: &Self::Processed) -> Result<u32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SkateboardSteeringError<A, I> {
    Attribute(A),
    Input(I),
}
impl<A: std::fmt::Debug, I: std::fmt::Debug> std::fmt::Display for SkateboardSteeringError<A, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "steering attribute: {e:?}"),
            Self::Input(e) => write!(f, "steering input: {e:?}"),
        }
    }
}
pub(crate) struct SkateboardSteeringState {
    deck_tilt: f32,
    physics_error: [f32; 4],
    physics_error_blending: bool,
}
impl SkateboardSteeringState {
    /// Outer constructor store 0x82b38b08 writes the proven positive-zero f0.
    pub(crate) const fn new() -> Self {
        Self {
            deck_tilt: 0.0,
            // Outer constructor38b00 copies gL;38b04 clears the blend byte.
            physics_error: [0.0, 0.0, 0.0, 1.0],
            physics_error_blending: false,
        }
    }
    pub(crate) const fn physics_error(&self) -> [f32; 4] {
        self.physics_error
    }
    pub(crate) const fn physics_error_blending(&self) -> bool {
        self.physics_error_blending
    }
    pub(crate) const fn deck_tilt(&self) -> f32 {
        self.deck_tilt
    }
    /// Outer Skateboard::TeleportReset 0x82b3613c, before Body turn stores.
    pub(crate) fn reset_teleport_deck_tilt(&mut self) {
        self.deck_tilt = 0.0;
    }
    /// Outer board 0x82b36180 restores only the error quaternion; the
    /// independent error-blending flag is not written by this store.
    pub(crate) fn reset_teleport_physics_error(&mut self) {
        self.physics_error = [0.0, 0.0, 0.0, 1.0];
    }
    pub(crate) fn try_update<A: SkateboardSteeringAttributes, I: SkateboardSteeringInput>(
        &mut self,
        target: f32,
        attributes: &mut A,
        input: &mut I,
        set_actual_body_turn: impl FnOnce(f32, bool, bool),
    ) -> Result<(), SkateboardSteeringError<A::Error, I::Error>> {
        use SkateboardSteeringError as E;
        let coefficient = attributes.blend_coefficient().map_err(E::Attribute)?;
        let weighted = coefficient * target;
        let processed = input.capture_processed().map_err(E::Input)?;
        let old = self.deck_tilt;
        let remaining = 1.0 - coefficient;
        self.deck_tilt = remaining.mul_add(old, weighted);
        let flipped = input.flags_8c4(&processed).map_err(E::Input)? & 0x0010_0000 != 0;
        let flags = input.flags_8c8(&processed).map_err(E::Input)?;
        let mut front = flags & 0x1000_0000 != 0;
        let mut back = flags & 0x0800_0000 != 0;
        if flipped {
            std::mem::swap(&mut front, &mut back);
        }
        set_actual_body_turn(self.deck_tilt, front, back);
        Ok(())
    }
}
#[cfg(test)]
#[path = "skateboard_steering_tests.rs"]
mod tests;

#[path = "skateboard_steering_calculate.rs"]
pub(crate) mod calculate;

#[path = "skateboard_steering_calculate_attributes.rs"]
pub(crate) mod calculate_attributes;

#[path = "../runtime/skateboard_animated_blend.rs"]
pub(crate) mod animated;
