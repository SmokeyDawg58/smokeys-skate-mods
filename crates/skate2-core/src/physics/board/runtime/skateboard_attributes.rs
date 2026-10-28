//! Retained attribute bindings and typed reads used by authored board creation.
//!
//! The semantic port preserves lookup versus pointer-dereference ordering in
//! `CreateTrucks` (0x82b33da8) and `CreateTriangleDeck` (0x82b32e08). The concrete
//! provider owns the immutable Vault read graph, not a mutable editor service.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "authored board construction is awaiting live skater admission"
    )
)]

use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::{fmt, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardAttributeClass {
    Deck,
    World,
    Wheels,
    TrucksDrives,
    Trucks,
    Wipeout,
    Manual,
    Slide,
}

impl SkateboardAttributeClass {
    pub(crate) const fn key(self) -> u64 {
        match self {
            Self::Deck => 0x9ed2_91a2_7445_3fc0,
            Self::World => 0x485a_76b6_8ca6_5d6f,
            Self::Wheels => 0x46ad_d2ee_a90c_c062,
            Self::TrucksDrives => 0x6599_909c_97c5_7a89,
            Self::Trucks => 0xaf78_1d7b_614d_ab50,
            Self::Wipeout => 0xeb84_4cfd_bf92_8efd,
            Self::Manual => 0xa0d7_351e_fa17_6078,
            Self::Slide => 0xfd27_7110_3f9a_d3db, // physics_slide ctor828b1158
        }
    }
}

/// Each lookup uses source element index zero. A returned handle remains valid
/// through subsequent lookups; loads happen only when the constructor asks.
/// This distinction allows the source's delayed material/mass dereferences.
pub(crate) trait SkateboardAttributeRead {
    type Pointer;
    type Error;

    fn lookup(
        &mut self,
        class: SkateboardAttributeClass,
        key: u64,
    ) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn byte(&mut self, pointer: &Self::Pointer) -> Result<u8, Self::Error>;
    fn integer(&mut self, pointer: &Self::Pointer) -> Result<i32, Self::Error>;

    fn read_float(
        &mut self,
        class: SkateboardAttributeClass,
        key: u64,
    ) -> Result<f32, Self::Error> {
        let pointer = self.lookup(class, key)?;
        self.float(&pointer)
    }
}

#[derive(Debug)]
pub(crate) enum SkateboardAttributeError {
    Runtime(AttributeRuntimeError),
    ValueSize { expected: usize, actual: usize },
}

impl fmt::Display for SkateboardAttributeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "board attribute lookup: {error}"),
            Self::ValueSize { expected, actual } => write!(
                f,
                "board attribute requires {expected} bytes, found {actual}"
            ),
        }
    }
}

impl std::error::Error for SkateboardAttributeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::ValueSize { .. } => None,
        }
    }
}

/// The relevant retained slice of `GlobalAttributeCollection` (0x828b1608).
/// This does not initialize its other classes or implement notification ports.
pub(crate) struct SkateboardAttributes {
    deck: AttributeRuntimeInstance,
    world: AttributeRuntimeInstance,
    wheels: AttributeRuntimeInstance,
    trucks_drives: AttributeRuntimeInstance,
    trucks: AttributeRuntimeInstance,
    wipeout: AttributeRuntimeInstance,
    manual: AttributeRuntimeInstance,
    slide: AttributeRuntimeInstance,
}

impl SkateboardAttributes {
    /// `BadLanding`82dd9844: direct cached layout, no `DefaultDataArea` fallback.
    pub(crate) fn try_wipeout_landing_graph(
        &self,
    ) -> Result<Option<crate::physics::PointGraph<8>>, SkateboardAttributeError> {
        let Some(data) = self
            .wipeout
            .try_cached_layout(64)
            .map_err(SkateboardAttributeError::Runtime)?
        else {
            return Ok(None);
        };
        let bytes = data.bytes();
        if bytes.len() != 64 {
            return Err(SkateboardAttributeError::ValueSize {
                expected: 64,
                actual: bytes.len(),
            });
        }
        let sample = |index: usize| {
            f32::from_be_bytes(
                bytes[index * 4..index * 4 + 4]
                    .try_into()
                    .expect("validated64-byte graph"),
            )
        };
        Ok(Some(crate::physics::PointGraph {
            x: std::array::from_fn(sample),
            y: std::array::from_fn(|i| sample(i + 8)),
        }))
    }
    pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            deck: vault.instance(
                SkateboardAttributeClass::Deck.key(),
                Self::DEFAULT_COLLECTION,
            ),
            world: vault.instance(
                SkateboardAttributeClass::World.key(),
                Self::DEFAULT_COLLECTION,
            ),
            wheels: vault.instance(
                SkateboardAttributeClass::Wheels.key(),
                Self::DEFAULT_COLLECTION,
            ),
            // Native selected binding order: +9c wheels, +a0 truck drives,
            // then +dc trucks (Instance at 0x828b23ec for the drive class).
            trucks_drives: vault.instance(
                SkateboardAttributeClass::TrucksDrives.key(),
                Self::DEFAULT_COLLECTION,
            ),
            trucks: vault.instance(
                SkateboardAttributeClass::Trucks.key(),
                Self::DEFAULT_COLLECTION,
            ),
            // Native retained default at GlobalAttributeCollection +0x108.
            wipeout: vault.instance(
                SkateboardAttributeClass::Wipeout.key(),
                Self::DEFAULT_COLLECTION,
            ),
            // Global +c8, wrapper828b0e88 class key assembled at0e9c..0eb4.
            slide: vault.instance(SkateboardAttributeClass::Slide.key(), Self::DEFAULT_COLLECTION),
            manual: vault.instance(
                SkateboardAttributeClass::Manual.key(),
                Self::DEFAULT_COLLECTION,
            ),
        }
    }
}

impl SkateboardAttributeRead for SkateboardAttributes {
    type Pointer = Option<AttributeRuntimeData>;
    type Error = SkateboardAttributeError;

    fn lookup(
        &mut self,
        class: SkateboardAttributeClass,
        key: u64,
    ) -> Result<Self::Pointer, Self::Error> {
        let instance = match class {
            SkateboardAttributeClass::Deck => &self.deck,
            SkateboardAttributeClass::World => &self.world,
            SkateboardAttributeClass::Wheels => &self.wheels,
            SkateboardAttributeClass::TrucksDrives => &self.trucks_drives,
            SkateboardAttributeClass::Trucks => &self.trucks,
            SkateboardAttributeClass::Wipeout => &self.wipeout,
            SkateboardAttributeClass::Manual => &self.manual,
            SkateboardAttributeClass::Slide => &self.slide,
        };
        instance
            .try_get_data(key, 0)
            .map_err(SkateboardAttributeError::Runtime)
    }

    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        read_bytes(pointer.as_ref()).map(f32::from_be_bytes)
    }

    fn byte(&mut self, pointer: &Self::Pointer) -> Result<u8, Self::Error> {
        read_bytes::<1>(pointer.as_ref()).map(|bytes| bytes[0])
    }

    fn integer(&mut self, pointer: &Self::Pointer) -> Result<i32, Self::Error> {
        read_bytes(pointer.as_ref()).map(i32::from_be_bytes)
    }
}

fn read_bytes<const N: usize>(
    pointer: Option<&AttributeRuntimeData>,
) -> Result<[u8; N], SkateboardAttributeError> {
    // Native typed callers use DefaultDataArea(1/4) only for a missing pointer.
    // Unsupported representations have already returned an error, not None.
    let Some(data) = pointer else {
        return Ok([0; N]);
    };
    data.bytes()
        .try_into()
        .map_err(|_| SkateboardAttributeError::ValueSize {
            expected: N,
            actual: data.bytes().len(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_spellings_match_all_native_keys_and_default_key() {
        use skate2_formats::attrib::string_hash64;
        for (class, name) in [
            (SkateboardAttributeClass::Deck, "physicsdeck"),
            (SkateboardAttributeClass::World, "physics_world"),
            (SkateboardAttributeClass::Wheels, "physicswheels"),
            (
                SkateboardAttributeClass::TrucksDrives,
                "physicstrucks_drives",
            ),
            (SkateboardAttributeClass::Trucks, "physicstrucks"),
            (SkateboardAttributeClass::Wipeout, "physics_wipeout"),
            (SkateboardAttributeClass::Manual, "physics_manual"),
        ] {
            assert_eq!(class.key(), string_hash64(name));
        }
        assert_eq!(
            SkateboardAttributes::DEFAULT_COLLECTION,
            string_hash64("default")
        );
    }

    #[test]
    fn missing_typed_pointer_selects_zero_without_other_tuning() {
        assert_eq!(read_bytes::<1>(None).unwrap(), [0]);
        assert_eq!(read_bytes::<4>(None).unwrap(), [0; 4]);
    }
}

#[path = "../control/skateboard_heading_manual_attributes.rs"]
pub(crate) mod heading_manual;

#[path = "../control/skateboard_manual_effect_attributes.rs"]
pub(crate) mod manual_effect_attributes;
