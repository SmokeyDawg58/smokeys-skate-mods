//! Lazy global Vault reads for `SkeletonCollision::Update` 0x82b4b8f0 and
//! `CalculateIsGroundPlane` 0x82b3e720. These are ordinary index0 attribute
//! lookups, not generated-layout snapshots or cached runtime tuning.
//!
//! The actual immutable Vault/instances are retained. This does not model
//! replacement of the native global collection singleton or editor reload.
//! Delayed pointer loading remains visible to the consuming source branches;
//! only a genuinely absent pointer receives `DefaultDataArea4` positive zero.

use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    skeleton_attributes::SkeletonAttributeClass,
    skeleton_collision_attributes::SkeletonCollisionAttributes,
};
use std::sync::Arc;

#[cfg(test)]
#[path = "skeleton_collision_update_attributes_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonCollisionUpdateAttribute {
    TimerDuration,
    SmallObjectMassThreshold,
    GroundMaximumDistance,
    GroundMinimumAlignment,
    AiGroupSixForceMultiplier,
    GroupSixForceMultiplier,
}

impl SkeletonCollisionUpdateAttribute {
    pub(crate) const fn key(self) -> u64 {
        match self {
            Self::TimerDuration => 0x12ea_5351_a38f_d32e,
            Self::SmallObjectMassThreshold => 0x9ab5_7b38_d5bb_9449,
            Self::GroundMaximumDistance => 0x30ef_9709_5d6f_a22c,
            Self::GroundMinimumAlignment => 0xe6b3_72a5_b8ec_c397,
            Self::AiGroupSixForceMultiplier => 0x98db_b06a_81ec_4f67,
            Self::GroupSixForceMultiplier => 0x3f30_79e0_8606_7292,
        }
    }
}

/// Source timer lookup/load: ba28/ba40. Small-object lookup c0c4 precedes
/// its gate; load c0e0 happens only if the original other-mass test passes.
/// Ground alignment lookup e800 also precedes its distance/offboard tests;
/// a skipped load must not produce a malformed-scalar error eagerly.
/// Group6 selects exactly one AI/non-AI route before lookup c31c/load c330.
pub(crate) trait SkeletonCollisionUpdateAttributeRead {
    type Pointer;
    type Error;
    fn lookup(
        &mut self,
        route: SkeletonCollisionUpdateAttribute,
    ) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
}

pub(crate) struct SkeletonCollisionUpdateAttributes {
    skeleton: AttributeRuntimeInstance,
    collision: AttributeRuntimeInstance,
}

impl SkeletonCollisionUpdateAttributes {
    pub(crate) const SKELETON_CLASS: u64 = SkeletonAttributeClass::Skeleton.key();
    pub(crate) const COLLISION_CLASS: u64 = SkeletonCollisionAttributes::CLASS;
    pub(crate) const DEFAULT_COLLECTION: u64 = SkeletonCollisionAttributes::DEFAULT_COLLECTION;

    /// Source singleton fields +ec/+10c are the global skeleton/collision
    /// defaults, not the local body profile. Binding performs no scalar FETCH.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            skeleton: vault.instance(Self::SKELETON_CLASS, Self::DEFAULT_COLLECTION),
            collision: vault.instance(Self::COLLISION_CLASS, Self::DEFAULT_COLLECTION),
        }
    }
}

impl SkeletonCollisionUpdateAttributeRead for SkeletonCollisionUpdateAttributes {
    type Pointer = Option<AttributeRuntimeData>;
    type Error = AttributeRuntimeError;

    fn lookup(
        &mut self,
        route: SkeletonCollisionUpdateAttribute,
    ) -> Result<Self::Pointer, Self::Error> {
        let instance = match route {
            SkeletonCollisionUpdateAttribute::TimerDuration
            | SkeletonCollisionUpdateAttribute::SmallObjectMassThreshold => &self.skeleton,
            SkeletonCollisionUpdateAttribute::GroundMaximumDistance
            | SkeletonCollisionUpdateAttribute::GroundMinimumAlignment
            | SkeletonCollisionUpdateAttribute::AiGroupSixForceMultiplier
            | SkeletonCollisionUpdateAttribute::GroupSixForceMultiplier => &self.collision,
        };
        instance.try_get_data(route.key(), 0)
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

#[path = "skeleton_collision_audio_attributes.rs"]
pub(crate) mod audio_attributes;
