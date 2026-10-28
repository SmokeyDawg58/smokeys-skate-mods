//! Proposed `input_cvar_runtime::crouching` child; two retained shared globals.
use super::InputCvarReferences;
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    generated_motion_load::{MotionLoadError, runtime::MotionUpdateError as Error},
    physics::PointGraph,
};
use std::sync::Arc;
pub(crate) struct CrouchingAttributes {
    vault: Arc<AttributeRuntimeVault>,
    crouching: AttributeRuntimeInstance,
    auto_pump: AttributeRuntimeInstance,
}
impl CrouchingAttributes {
    /// Global ctor828b2d58/store2d64(+12c), ctor30b8/store30c4(+15c).
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let crouching = vault.instance(0x90c3_decf_0105_631e, 0x2d5b_a8be_3ccb_a8fb);
        crouching.try_cached_layout(0x5c0)?;
        let auto_pump = vault.instance(0x90c3_decf_0105_631e, 0x5e22_3c91_4f2f_5db3);
        auto_pump.try_cached_layout(0x5c0)?;
        Ok(Self {
            vault: Arc::clone(vault),
            crouching,
            auto_pump,
        })
    }
    pub(crate) fn check_vault(&self, vault: &Arc<AttributeRuntimeVault>) -> Result<(), Error> {
        if Arc::ptr_eq(&self.vault, vault) {
            Ok(())
        } else {
            Err(Error::ForeignOwner("Crouching shared globals Vault"))
        }
    }
    fn instance(&self, auto_pump: bool) -> &AttributeRuntimeInstance {
        if auto_pump {
            &self.auto_pump
        } else {
            &self.crouching
        }
    }
    pub(crate) fn capture(
        &self,
        auto_pump: bool,
        key: u64,
    ) -> Result<Option<AttributeRuntimeData>, Error> {
        self.instance(auto_pump)
            .try_get_data(key, 0)
            .map_err(|e| Error::Load(MotionLoadError::AttributeRuntime(e)))
    }
    pub(crate) fn read(value: Option<&AttributeRuntimeData>) -> Result<f32, Error> {
        let Some(data) = value else { return Ok(0.0) };
        let bytes = data.bytes().try_into().map_err(|_| {
            Error::Load(MotionLoadError::AttributeRuntime(
                AttributeRuntimeError::ScalarSize {
                    size: data.bytes().len(),
                },
            ))
        })?;
        Ok(f32::from_be_bytes(bytes))
    }
    pub(crate) fn graph(&self, auto_pump: bool, offset: usize, input: f32) -> Result<f32, Error> {
        if (auto_pump && offset != 0x590)
            || (!auto_pump && !matches!(offset, 0x190 | 0x390 | 0x450))
        {
            return Err(Error::MissingOwner("Crouching graph offset"));
        }
        let data = self
            .instance(auto_pump)
            .try_cached_layout(0x5c0)
            .map_err(|e| Error::Load(MotionLoadError::AttributeRuntime(e)))?;
        let word = |at: usize| {
            data.as_ref().map_or(0.0, |v| {
                f32::from_be_bytes(v.bytes()[at..at + 4].try_into().expect("checked layout"))
            })
        };
        if auto_pump {
            Ok(PointGraph::<4> {
                x: std::array::from_fn(|i| word(offset + 0x10 + i * 4)),
                y: std::array::from_fn(|i| word(offset + 0x20 + i * 4)),
            }
            .evaluate(input))
        } else {
            Ok(PointGraph::<8> {
                x: std::array::from_fn(|i| word(offset + 0x10 + i * 4)),
                y: std::array::from_fn(|i| word(offset + 0x30 + i * 4)),
            }
            .evaluate(input))
        }
    }
}
impl InputCvarReferences {
    pub(crate) fn try_crouching_auto_pump(&self) -> Result<bool, super::InputCvarError> {
        self.try_current_boolean(232)
    }
    pub(crate) fn try_crouching_pumping_time(&self) -> Result<f32, super::InputCvarError> {
        let targets = self
            .targets
            .lock()
            .map_err(|_| super::InputCvarError::Lock)?;
        let target = targets[233]
            .as_ref()
            .ok_or(super::InputCvarError::Unlinked(
                super::DEFINITIONS[233].label,
            ))?;
        match *target.lock().map_err(|_| super::InputCvarError::Lock)? {
            super::Value::Float(value) => Ok(value),
            _ => Err(super::InputCvarError::WrongType),
        }
    }
}
