//! Original global `physics_collision.default` reads used by
//! `SkeletonCollision::InitSpecificBodyParts` (0x82b3ec78).
//!
//! The provider retains the actual immutable Vault graph. This does not model
//! global singleton replacement, editor reload, or mutable cached-pointer races.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "collision data awaits physical skeleton ownership"
    )
)]

use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::sync::Arc;

pub(crate) const GROIN_RADIUS: u64 = 0xa945_308c_de00_2e9c;
pub(crate) const FACE_RADIUS: u64 = 0xa8ed_92d2_01be_e7dd;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonCollisionOffset {
    Groin,
    Face,
}

/// Both radius pointers are looked up before either is dereferenced. The
/// second pointer is loaded first, then the first; missing pointers independently
/// select the generated zero scalar default without hiding malformed storage.
pub(crate) trait SkeletonCollisionAttributeRead {
    type Pointer;
    type Error;
    fn cached_offset(&mut self, slot: SkeletonCollisionOffset) -> Result<[f32; 4], Self::Error>;
    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
}

pub(crate) struct SkeletonCollisionAttributes {
    instance: AttributeRuntimeInstance,
}

impl SkeletonCollisionAttributes {
    pub(crate) const CLASS: u64 = 0x7379_c446_7aa7_5df9;
    pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

    /// Generated ctor 0x828b1338 retains Instance layout or `DefaultDataArea112`.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(Self::CLASS, Self::DEFAULT_COLLECTION),
        }
    }
}

impl SkeletonCollisionAttributeRead for SkeletonCollisionAttributes {
    type Pointer = Option<AttributeRuntimeData>;
    type Error = AttributeRuntimeError;

    fn cached_offset(&mut self, slot: SkeletonCollisionOffset) -> Result<[f32; 4], Self::Error> {
        let Some(layout) = self.instance.try_cached_layout(112)? else {
            return Ok([0.0; 4]);
        };
        let start = match slot {
            SkeletonCollisionOffset::Groin => 0,
            SkeletonCollisionOffset::Face => 16,
        };
        Ok(std::array::from_fn(|lane| {
            let offset = start + lane * 4;
            f32::from_be_bytes(
                layout.bytes()[offset..offset + 4]
                    .try_into()
                    .expect("bounded cached vector"),
            )
        }))
    }

    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        self.instance.try_get_data(key, 0)
    }

    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        let Some(data) = pointer else {
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
