//! Shared Global+134; proposed `input_cvar_runtime::motion_pumping` child.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::{MotionLoadError, runtime::MotionUpdateError as Error},
    physics::PointGraph,
};
use std::sync::Arc;
pub(crate) struct PumpingAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl PumpingAttributes {
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x90c3_decf_0105_631e, 0xd40d_1dc0_3536_79d7);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self {
            vault: Arc::clone(vault),
            instance,
        })
    }
    pub(crate) fn check_vault(&self, vault: &Arc<AttributeRuntimeVault>) -> Result<(), Error> {
        if Arc::ptr_eq(&self.vault, vault) {
            Ok(())
        } else {
            Err(Error::ForeignOwner("Pumping shared global Vault"))
        }
    }
    pub(crate) fn scalar(&self, key: u64) -> Result<f32, Error> {
        self.instance
            .try_scalar_word(key, 0)
            .map(|v| f32::from_bits(v.unwrap_or(0)))
            .map_err(|e| Error::Load(MotionLoadError::AttributeRuntime(e)))
    }
    pub(crate) fn graph(&self, input: f32) -> Result<f32, Error> {
        let layout = self
            .instance
            .try_cached_layout(0x5c0)
            .map_err(|e| Error::Load(MotionLoadError::AttributeRuntime(e)))?;
        let word = |offset: usize| {
            layout.as_ref().map_or(0.0, |d| {
                f32::from_be_bytes(
                    d.bytes()[offset..offset + 4]
                        .try_into()
                        .expect("checked layout"),
                )
            })
        };
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| word(0x3e0 + i * 4)),
            y: std::array::from_fn(|i| word(0x400 + i * 4)),
        }
        .evaluate(input))
    }
}
impl super::InputCvarReferences {
    /// kUsePumping238 init82f85f98/table82fd4ed4; sampled after both lengths.
    pub(crate) fn try_motion_pumping_enabled(&self) -> Result<bool, super::InputCvarError> {
        self.try_current_boolean(238)
    }
}
