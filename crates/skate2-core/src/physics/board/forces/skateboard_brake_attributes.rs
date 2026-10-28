//! Global `physics_brakes.default` scalar reads for brake force 0x82b26df0/0x82b27078.
//! Immutable retained Vault instances preserve original lookup/default behavior;
//! editor reload and replacement of the global singleton remain outside scope.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "brake attributes await the actual grounded board owner"
    )
)]

use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::{fmt, sync::Arc};

pub(crate) const FOOT_COEFFICIENT: u64 = 0xd71f_1207_5243_2c0a;
pub(crate) const MANUAL_COEFFICIENT: u64 = 0xc19a_40df_5e7b_2662;
pub(crate) const STOP_SPEED: u64 = 0x92d0_c41a_3315_b63d;
pub(crate) const BALANCE_SPEED: u64 = 0x6546_8a36_a6e3_2f03;
pub(crate) const NORMAL_Y_THRESHOLD: u64 = 0x343f_6723_7b0b_f1bc;
pub(crate) const BALANCE_DRAG: u64 = 0x75a8_6a1f_8ff8_ab6d;

/// Lookup and value reads remain separate source stages.
pub(crate) trait SkateboardBrakeAttributeRead {
    type Pointer;
    type Error;
    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
}

pub(crate) struct SkateboardBrakeAttributePointer {
    instance: Arc<()>,
    data: Option<AttributeRuntimeData>,
}

#[derive(Debug)]
pub(crate) enum SkateboardBrakeAttributeError {
    ForeignPointer,
    Runtime(AttributeRuntimeError),
}
impl fmt::Display for SkateboardBrakeAttributeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignPointer => {
                f.write_str("brake attribute pointer belongs to another instance")
            }
            Self::Runtime(error) => write!(f, "brake attribute: {error}"),
        }
    }
}
impl std::error::Error for SkateboardBrakeAttributeError {}

pub(crate) struct SkateboardBrakeVaultAttributes {
    instance: AttributeRuntimeInstance,
    identity: Arc<()>,
}

impl SkateboardBrakeVaultAttributes {
    /// Generated `physics_brakes` constructor 0x828b24b4 supplies this class hash.
    pub(crate) const CLASS: u64 = 0x191b_ce2c_9e14_5f51;
    pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(Self::CLASS, Self::DEFAULT_COLLECTION),
            identity: Arc::new(()),
        }
    }
}

impl SkateboardBrakeAttributeRead for SkateboardBrakeVaultAttributes {
    type Pointer = SkateboardBrakeAttributePointer;
    type Error = SkateboardBrakeAttributeError;

    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        let data = self
            .instance
            .try_get_data(key, 0)
            .map_err(SkateboardBrakeAttributeError::Runtime)?;
        Ok(SkateboardBrakeAttributePointer {
            instance: Arc::clone(&self.identity),
            data,
        })
    }

    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        if !Arc::ptr_eq(&pointer.instance, &self.identity) {
            return Err(SkateboardBrakeAttributeError::ForeignPointer);
        }
        // Only a native missing pointer selects DefaultDataArea(4).
        let Some(data) = &pointer.data else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] = data.bytes().try_into().map_err(|_| {
            SkateboardBrakeAttributeError::Runtime(AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            })
        })?;
        Ok(f32::from_be_bytes(bytes))
    }
}

#[cfg(test)]
#[path = "skateboard_brake_attributes_tests.rs"]
pub(crate) mod tests;
