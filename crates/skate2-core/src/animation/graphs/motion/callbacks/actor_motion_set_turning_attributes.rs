//! Shared Global+174 `anim_carving_default`, child `input_cvar_runtime::set_turning`.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::{MotionLoadError, runtime::MotionUpdateError},
    physics::PointGraph,
};
use std::sync::Arc;
impl super::InputCvarReferences {
    pub(crate) fn try_turning_reshape(&self, index: usize) -> Result<bool, super::InputCvarError> {
        self.try_current_boolean(index)
    }
}
pub(crate) struct TurningAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl TurningAttributes {
    /// Allocation828b3258/ctor326c/publication3278; generated ctor828b1518.
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x705b_d1b1_2aef_b73a, 0xd7ed_bd36_2d7d_2152);
        instance.try_cached_layout(0x2e0)?;
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
            Err(MotionUpdateError::ForeignOwner("global carving Vault"))
        }
    }
    pub(crate) fn scalar(&self, key: u64) -> Result<f32, MotionUpdateError> {
        self.instance
            .try_scalar_word(key, 0)
            .map(|v| f32::from_bits(v.unwrap_or(0)))
            .map_err(|e| MotionUpdateError::Load(MotionLoadError::AttributeRuntime(e)))
    }
    pub(crate) fn graph(&self, offset: usize, input: f32) -> Result<f32, MotionUpdateError> {
        if !matches!(offset, 0 | 0x90 | 0x120 | 0x1b0 | 0x240 | 0x290) {
            return Err(MotionUpdateError::MissingOwner("carving graph offset"));
        }
        let layout = self
            .instance
            .try_cached_layout(0x2e0)
            .map_err(|e| MotionUpdateError::Load(MotionLoadError::AttributeRuntime(e)))?;
        // Generated ctor828b1568 supplies DefaultDataArea2e0 when absent.
        let word = |at: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[at..at + 4]
                        .try_into()
                        .expect("validated carving layout"),
                )
            })
        };
        Ok(if offset >= 0x240 {
            PointGraph::<8> {
                x: std::array::from_fn(|i| word(offset + 0x10 + i * 4)),
                y: std::array::from_fn(|i| word(offset + 0x30 + i * 4)),
            }
            .evaluate(input)
        } else {
            PointGraph::<16> {
                x: std::array::from_fn(|i| word(offset + 0x10 + i * 4)),
                y: std::array::from_fn(|i| word(offset + 0x50 + i * 4)),
            }
            .evaluate(input)
        })
    }
}
