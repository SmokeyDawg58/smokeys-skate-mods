//! Global `physics_reckoning.default` normal-angle lookup, 0x82b37164..0x82b371cc.
//! Capturing the immutable retained instance does not perform attribute lookup
//! or scalar decode. Runtime singleton replacement/editor reload is not modeled.
use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::{fmt, sync::Arc};

pub(crate) const GROUND_NORMAL_ANGLE: u64 = 0x0e50_8708_cc7f_5249;

pub(crate) trait SkateboardGroundNormalAttributeRead {
    type Global;
    type Pointer;
    type Error;
    fn capture_global(&mut self) -> Result<Self::Global, Self::Error>;
    fn lookup(&mut self, global: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
}

/// Both tokens retain the actual immutable instance lifetime, and its distinct
/// occurrence even when another provider selects the same Vault collection.
pub(crate) struct SkateboardGroundNormalGlobal {
    identity: Arc<()>,
    instance: AttributeRuntimeInstance,
}
pub(crate) struct SkateboardGroundNormalPointer {
    identity: Arc<()>,
    data: Option<AttributeRuntimeData>,
}
#[derive(Debug)]
pub(crate) enum SkateboardGroundNormalAttributeError {
    ForeignGlobal,
    ForeignPointer,
    Runtime(AttributeRuntimeError),
}
impl fmt::Display for SkateboardGroundNormalAttributeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignGlobal => f.write_str("ground-normal global belongs to another instance"),
            Self::ForeignPointer => {
                f.write_str("ground-normal pointer belongs to another instance")
            }
            Self::Runtime(error) => write!(f, "ground-normal attribute: {error}"),
        }
    }
}
impl std::error::Error for SkateboardGroundNormalAttributeError {}

pub(crate) struct SkateboardGroundNormalVaultAttributes {
    instance: AttributeRuntimeInstance,
    identity: Arc<()>,
}
impl SkateboardGroundNormalVaultAttributes {
    pub(crate) const CLASS: u64 = 0x7ce1_b6b5_8170_9c6f;
    pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
    pub(crate) fn occurrence(&self) -> &Arc<()> {
        &self.identity
    }
    pub(crate) fn checked_instance<'a>(
        &self,
        global: &'a SkateboardGroundNormalGlobal,
    ) -> Result<&'a AttributeRuntimeInstance, SkateboardGroundNormalAttributeError> {
        if !Arc::ptr_eq(&global.identity, &self.identity) {
            return Err(SkateboardGroundNormalAttributeError::ForeignGlobal);
        }
        Ok(&global.instance)
    }
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(Self::CLASS, Self::DEFAULT_COLLECTION),
            identity: Arc::new(()),
        }
    }
}
impl SkateboardGroundNormalAttributeRead for SkateboardGroundNormalVaultAttributes {
    type Global = SkateboardGroundNormalGlobal;
    type Pointer = SkateboardGroundNormalPointer;
    type Error = SkateboardGroundNormalAttributeError;
    fn capture_global(&mut self) -> Result<Self::Global, Self::Error> {
        Ok(SkateboardGroundNormalGlobal {
            identity: Arc::clone(&self.identity),
            instance: self.instance.clone(),
        })
    }
    fn lookup(&mut self, global: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error> {
        if !Arc::ptr_eq(&global.identity, &self.identity) {
            return Err(Self::Error::ForeignGlobal);
        }
        let data = global
            .instance
            .try_get_data(key, 0)
            .map_err(Self::Error::Runtime)?;
        Ok(SkateboardGroundNormalPointer {
            identity: Arc::clone(&self.identity),
            data,
        })
    }
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        if !Arc::ptr_eq(&pointer.identity, &self.identity) {
            return Err(Self::Error::ForeignPointer);
        }
        // Source DefaultDataArea(4) is selected only for a missing pointer.
        let Some(data) = &pointer.data else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] = data.bytes().try_into().map_err(|_| {
            Self::Error::Runtime(AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            })
        })?;
        Ok(f32::from_be_bytes(bytes))
    }
}
#[cfg(test)]
#[path = "skateboard_ground_normal_attributes_tests.rs"]
pub(crate) mod tests;
