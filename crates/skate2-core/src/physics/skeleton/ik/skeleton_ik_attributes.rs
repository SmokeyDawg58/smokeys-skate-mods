//! Retained `physics_skeletonik.default` Instance, constructed at 0x82b5513c.
//! The five active pre-physics read sites use keyed index-zero lookups, with
//! immediate scalar/vector loads and no extra generated cached-layout field.
//! Immutable Vault allocation ownership excludes native editor/alias races.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "active IK aggregate owns provider integration")
)]

use crate::attribute_runtime::{
    AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::sync::Arc;

#[cfg(test)]
#[path = "skeleton_ik_attributes_tests.rs"]
pub(crate) mod tests;

const CLASS: u64 = 0xc8be_3349_f2bd_493e;
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;

pub(crate) trait SkeletonIkAttributeRead {
    type Error;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn vector(&mut self, key: u64) -> Result<[f32; 4], Self::Error>;
}

/// Owns the native IK Instance lifetime. The constructor's global deck reads
/// borrow the existing board provider separately; no deck tuning is copied here.
pub(crate) struct SkeletonIkVaultAttributes {
    instance: AttributeRuntimeInstance,
}

impl SkeletonIkVaultAttributes {
    /// Selection only; malformed or absent payloads remain lazy per-call reads.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(CLASS, DEFAULT),
        }
    }

    fn bytes<const SIZE: usize>(&self, key: u64) -> Result<[u8; SIZE], AttributeRuntimeError> {
        let Some(data) = self.instance.try_get_data(key, 0)? else {
            return Ok([0; SIZE]); // Source DefaultDataArea(4/16), only on None.
        };
        data.bytes()
            .try_into()
            .map_err(|_| AttributeRuntimeError::PayloadSize {
                key,
                expected: SIZE,
                actual: data.bytes().len(),
            })
    }
}

impl SkeletonIkAttributeRead for SkeletonIkVaultAttributes {
    type Error = AttributeRuntimeError;

    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        Ok(f32::from_be_bytes(self.bytes(key)?))
    }

    fn vector(&mut self, key: u64) -> Result<[f32; 4], Self::Error> {
        let bytes = self.bytes::<16>(key)?;
        Ok(std::array::from_fn(|index| {
            f32::from_be_bytes(
                bytes[index * 4..index * 4 + 4]
                    .try_into()
                    .expect("checked vector word"),
            )
        }))
    }
}
