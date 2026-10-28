//! Proposed `input_cvar_runtime::power_sliding` child; shared Global+14c.
use super::InputCvarReferences;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::{MotionLoadError, runtime::MotionUpdateError},
    physics::PointGraph,
};
use std::sync::Arc;
pub(crate) struct PowerSlidingAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl PowerSlidingAttributes {
    /// Alloc828b2f74/ctor2f98/store2fa4; generated ctor828b1590.
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x90c3_decf_0105_631e, 0xd124_e672_1b8e_fc67);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self {
            vault: Arc::clone(vault),
            instance,
        })
    }
    pub(crate) fn check_vault(
        &self,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<(), MotionUpdateError> {
        if Arc::ptr_eq(&self.vault, vault) {
            Ok(())
        } else {
            Err(MotionUpdateError::ForeignOwner(
                "global power sliding Vault",
            ))
        }
    }
    /// Generic live field lookup, `DefaultDataArea4` only for an absent field.
    pub(crate) fn scalar(&self, key: u64) -> Result<f32, MotionUpdateError> {
        self.instance
            .try_scalar_word(key, 0)
            .map(|v| f32::from_bits(v.unwrap_or(0)))
            .map_err(|e| MotionUpdateError::Load(MotionLoadError::AttributeRuntime(e)))
    }
    /// Generated graph cached layout, exact four-point x+10/y+20.
    pub(crate) fn graph(&self, offset: usize, input: f32) -> Result<f32, MotionUpdateError> {
        if !matches!(offset, 0x4a0 | 0x500 | 0x4d0) {
            return Err(MotionUpdateError::MissingOwner("PowerSliding graph offset"));
        }
        let layout = self
            .instance
            .try_cached_layout(0x5c0)
            .map_err(|e| MotionUpdateError::Load(MotionLoadError::AttributeRuntime(e)))?;
        let word = |at: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[at..at + 4]
                        .try_into()
                        .expect("validated anim_motion layout"),
                )
            })
        };
        Ok(PointGraph::<4> {
            x: std::array::from_fn(|i| word(offset + 0x10 + i * 4)),
            y: std::array::from_fn(|i| word(offset + 0x20 + i * 4)),
        }
        .evaluate(input))
    }
    /// PowerSlideDecel82cb2238 / PowerSlideSpin82cb2504: cached eight-point
    /// graphs at layout+0 and +50, with x+10 and y+30.
    pub(crate) fn slide_graph(&self, offset: usize, input: f32) -> Result<f32, MotionUpdateError> {
        if !matches!(offset, 0 | 0x50) {
            return Err(MotionUpdateError::MissingOwner("slide graph offset"));
        }
        let layout = self.instance.try_cached_layout(0x5c0)
            .map_err(|e| MotionUpdateError::Load(MotionLoadError::AttributeRuntime(e)))?;
        let word = |at: usize| layout.as_ref().map_or(0.0, |data|
            f32::from_be_bytes(data.bytes()[at..at + 4].try_into().expect("checked layout")));
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| word(offset + 0x10 + i * 4)),
            y: std::array::from_fn(|i| word(offset + 0x30 + i * 4)),
        }.evaluate(input))
    }
}
impl InputCvarReferences {
    /// ref830a43c4, ctor82f85b70, table82fd4eac. Read at BOTH source debug gates.
    pub(crate) fn try_power_sliding_debug(&self) -> Result<bool, super::InputCvarError> {
        self.try_current_boolean(230)
    }
}
