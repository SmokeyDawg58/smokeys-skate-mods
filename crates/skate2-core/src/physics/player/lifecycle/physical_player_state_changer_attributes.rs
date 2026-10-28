//! Retained global `physics_airstates.default` slice used by `StateChanger`.
use crate::attribute_runtime::{
    AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::sync::Arc;

pub(crate) const AIR_STATES_CLASS: u64 = 0x28ab_b872_cb8c_b9ed;
pub(crate) const CLEAR_HEIGHT: u64 = 0x0fc2_2cb4_43a7_63ac;
pub(crate) const RECENT_HEIGHT: u64 = 0x8e8b_8588_435b_6fe8;
pub(crate) const CONTACT_GRACE: u64 = 0xc6e6_e6bd_74df_fb65;

pub(crate) struct StateChangerAttributes {
    air: AttributeRuntimeInstance,
    skitch: AttributeRuntimeInstance,
}
impl StateChangerAttributes {
    /// Global+104; wrapper828b0c30 assembles class at0c50/54/5c.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            air: vault.instance(AIR_STATES_CLASS, 0xd7ed_bd36_2d7d_2152),
            skitch: vault.instance(0x032c_d3de_521b_4ecd, 0xd7ed_bd36_2d7d_2152),
        }
    }
    /// BlendToAnimatedBoard82b25cac/dc reads the global+104 cached layout.
    /// Generated ctor828b0c74..88 establishes DefaultDataArea(0x130) on null.
    pub(crate) fn try_board_animation_blend(
        &self,
        fast: bool,
        angle: f32,
    ) -> Result<f32, AttributeRuntimeError> {
        let layout = self.air.try_cached_layout(0x130)?;
        let word = |offset: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[offset..offset + 4]
                        .try_into()
                        .expect("Air layout"),
                )
            })
        };
        let base = if fast { 0x50 } else { 0 };
        Ok(crate::physics::PointGraph::<8> {
            x: std::array::from_fn(|i| word(base + 0x10 + i * 4)),
            y: std::array::from_fn(|i| word(base + 0x30 + i * 4)),
        }
        .evaluate(angle))
    }
    /// All reads fetch again at their source call; only null uses `DefaultDataArea4`.
    pub(crate) fn scalar(&self, key: u64) -> Result<f32, AttributeRuntimeError> {
        scalar(&self.air, key)
    }
    pub(crate) fn skitch_scalar(&self, key: u64) -> Result<f32, AttributeRuntimeError> {
        scalar(&self.skitch, key)
    }
}
fn scalar(instance: &AttributeRuntimeInstance, key: u64) -> Result<f32, AttributeRuntimeError> {
    let data = instance.try_get_data(key, 0)?;
    let bytes = data.as_ref().map_or(&[0; 4][..], |data| data.bytes());
    if bytes.len() != 4 {
        return Err(AttributeRuntimeError::PayloadSize {
            key,
            expected: 4,
            actual: bytes.len(),
        });
    }
    Ok(f32::from_bits(u32::from_be_bytes(
        bytes.try_into().expect("checked scalar"),
    )))
}
