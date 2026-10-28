//! Global `physics_feet.default` scalar reads for foot force 0x82b28828.
//! Immutable retained Vault instances preserve original lookup/default behavior;
//! editor reload and replacement of the global singleton remain outside scope.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "foot attributes await the actual grounded board owner"
    )
)]

use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::{fmt, sync::Arc};

pub(crate) const FOOT_OFFSET: u64 = 0x481e_7fa6_1a3e_5ad0;
pub(crate) const REAR_ABSORPTION_SCALE: u64 = 0x6341_2dc2_3ce7_87d1;
pub(crate) const LANDING_DENOMINATOR: u64 = 0x7a04_29eb_688c_3919;
pub(crate) const SPEED_COEFFICIENT: u64 = 0x1a77_d90a_2eea_371c;
pub(crate) const INITIAL_COEFFICIENT: u64 = 0x1ef7_3284_bdb1_52de;

/// The source keeps the speed coefficient pointer across the next lookup.
pub(crate) trait SkateboardFootAttributeRead {
    type Pointer;
    type Error;
    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
}

pub(crate) struct SkateboardFootAttributePointer {
    instance: Arc<()>,
    data: Option<AttributeRuntimeData>,
}

#[derive(Debug)]
pub(crate) enum SkateboardFootAttributeError {
    ForeignPointer,
    Runtime(AttributeRuntimeError),
}
impl fmt::Display for SkateboardFootAttributeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignPointer => {
                f.write_str("foot attribute pointer belongs to another instance")
            }
            Self::Runtime(error) => write!(f, "foot attribute: {error}"),
        }
    }
}
impl std::error::Error for SkateboardFootAttributeError {}

pub(crate) struct SkateboardFootVaultAttributes {
    instance: AttributeRuntimeInstance,
    identity: Arc<()>,
}

impl SkateboardFootVaultAttributes {
    /// Generated `physics_feet` constructor 0x828b0e10 supplies this class hash.
    pub(crate) const CLASS: u64 = 0x5338_958c_408b_6009;
    pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(Self::CLASS, Self::DEFAULT_COLLECTION),
            identity: Arc::new(()),
        }
    }
}

impl SkateboardFootAttributeRead for SkateboardFootVaultAttributes {
    type Pointer = SkateboardFootAttributePointer;
    type Error = SkateboardFootAttributeError;

    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        let data = self
            .instance
            .try_get_data(key, 0)
            .map_err(SkateboardFootAttributeError::Runtime)?;
        Ok(SkateboardFootAttributePointer {
            instance: Arc::clone(&self.identity),
            data,
        })
    }

    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        if !Arc::ptr_eq(&pointer.instance, &self.identity) {
            return Err(SkateboardFootAttributeError::ForeignPointer);
        }
        // Only a native missing pointer selects DefaultDataArea(4).
        let Some(data) = &pointer.data else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] = data.bytes().try_into().map_err(|_| {
            SkateboardFootAttributeError::Runtime(AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            })
        })?;
        Ok(f32::from_be_bytes(bytes))
    }
}

#[cfg(test)]
#[path = "skateboard_foot_attributes_tests.rs"]
pub(crate) mod tests;

#[path = "../runtime/skateboard_wallride_attributes.rs"]
pub(crate) mod wallride;

#[path = "../control/skateboard_antiflip_attributes.rs"]
pub(crate) mod antiflip;
