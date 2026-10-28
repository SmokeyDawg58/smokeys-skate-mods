//! Retained scalar and cached-layout reads for `SkeletonBody::InitJoints`
//! (0x82b464f8). Concrete storage is immutable, not live Vault notification state.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "joint owner awaits physical skeleton admission")
)]
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    skeleton_attributes::SkeletonAttributeClass,
    skeleton_body_attributes::{ANIMATION_CLASS, DEFAULT_COLLECTION},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonJointByteRoute {
    LocalJoints,
    GlobalAnimation,
}

pub(crate) trait SkeletonJointAttributeRead {
    type Error;
    fn cached_prism_velocity(&mut self) -> Result<[f32; 4], Self::Error>;
    fn byte(&mut self, route: SkeletonJointByteRoute, key: u64) -> Result<u8, Self::Error>;
    fn skeleton_float(&mut self, key: u64) -> Result<f32, Self::Error>;
}

pub(crate) struct SkeletonJointAttributeProvider {
    skeleton: AttributeRuntimeInstance,
    joints: AttributeRuntimeInstance,
    animation: AttributeRuntimeInstance,
}

impl SkeletonJointAttributeProvider {
    /// Fresh local `mJointAttribs` read in `SetRagdollJointLimits`82b3bdd8/be28.
    pub(crate) fn joint_float(&mut self, key: u64) -> Result<f32, AttributeRuntimeError> {
        Ok(f32::from_bits(
            self.joints.try_scalar_word(key, 0)?.unwrap_or(0),
        ))
    }

    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>, local_collection: u64) -> Self {
        Self {
            skeleton: vault.instance(SkeletonAttributeClass::Skeleton.key(), local_collection),
            joints: vault.instance(SkeletonAttributeClass::Joints.key(), local_collection),
            animation: vault.instance(ANIMATION_CLASS, DEFAULT_COLLECTION),
        }
    }
}

impl SkeletonJointAttributeRead for SkeletonJointAttributeProvider {
    type Error = AttributeRuntimeError;
    fn cached_prism_velocity(&mut self) -> Result<[f32; 4], Self::Error> {
        let Some(layout) = self.skeleton.try_cached_layout(112)? else {
            return Ok([0.0; 4]); // Generated Skeleton DefaultDataArea112, not generic FETCH.
        };
        Ok(std::array::from_fn(|lane| {
            let offset = 0x30 + lane * 4;
            f32::from_be_bytes(
                layout.bytes()[offset..offset + 4]
                    .try_into()
                    .expect("bounded cached vector"),
            )
        }))
    }
    fn byte(&mut self, route: SkeletonJointByteRoute, key: u64) -> Result<u8, Self::Error> {
        let instance = match route {
            SkeletonJointByteRoute::LocalJoints => &self.joints,
            SkeletonJointByteRoute::GlobalAnimation => &self.animation,
        };
        let Some(data) = instance.try_get_data(key, 0)? else {
            return Ok(0);
        };
        let [value] = data.bytes() else {
            return Err(AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            });
        };
        Ok(*value)
    }
    fn skeleton_float(&mut self, key: u64) -> Result<f32, Self::Error> {
        Ok(f32::from_bits(
            self.skeleton.try_scalar_word(key, 0)?.unwrap_or(0),
        ))
    }
}
