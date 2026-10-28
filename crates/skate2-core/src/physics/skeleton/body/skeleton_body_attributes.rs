//! Scalar routes consumed by `SkeletonBody::Init` (0x82b61738).
//! Local bindings and global-default bindings remain distinct. The concrete
//! Vault graph is immutable; this does not implement reload/message ports.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaiting live SkeletonBody admission")
)]

use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    skeleton_attributes::SkeletonAttributeClass,
};
use std::sync::Arc;

pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
pub(crate) const DEBUG_PHYSICS_CLASS: u64 = 0xbabb_3ec1_1084_9e08;
pub(crate) const ANIMATION_CLASS: u64 = 0x74bc_9c93_affe_6aac;
pub(crate) const WIPEOUT_CLASS: u64 = 0xeb84_4cfd_bf92_8efd;
pub(crate) const CAPSULE_RADIUS_SCALE: u64 = 0x0720_4f10_dd93_7907;
pub(crate) const ENABLE_JOINTS: u64 = 0xdff9_7208_0e90_f82f;
pub(crate) const RAGDOLL_INVERSE_MASS_NUMERATOR: u64 = 0x3689_013d_7adb_dd2a;
pub(crate) const CAPSULE_LENGTH_SCALE: u64 = 0x5fe9_f585_a201_62a2;
pub(crate) const PART_MASS_SCALE: u64 = 0x9f4a_03c7_5637_e7e9;
pub(crate) const ROOT_HALF_LENGTH: u64 = 0x5e43_a9eb_d2be_bbde;
pub(crate) const ROOT_RADIUS: u64 = 0x7e90_113b_05ba_cda6;
const DEBUG_MODE: u64 = 0x6ddd_9384_be03_a6ea;
const DEBUG_FACTOR: u64 = 0x425c_5017_5444_1f41;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonBodyAttributeRoute {
    LocalBones,
    LocalSkeleton,
    GlobalSkeleton,
    GlobalAnimation,
    GlobalWipeout,
}

impl SkeletonBodyAttributeRoute {
    pub(crate) const fn class_key(self) -> u64 {
        match self {
            Self::LocalBones => SkeletonAttributeClass::Bones.key(),
            Self::LocalSkeleton | Self::GlobalSkeleton => SkeletonAttributeClass::Skeleton.key(),
            Self::GlobalAnimation => ANIMATION_CLASS,
            Self::GlobalWipeout => WIPEOUT_CLASS,
        }
    }
}

/// A fresh generic Instance is constructed for EVERY Part at 0x82b61f88.
/// Its mode is a word, not a boolean; factor is fetched only for modes1/2.
pub(crate) trait SkeletonBodyDebugRead {
    type Error;
    fn mode(&mut self) -> Result<u32, Self::Error>;
    fn factor(&mut self) -> Result<f32, Self::Error>;
}

pub(crate) trait SkeletonBodyAttributeRead {
    type Error;
    type DebugInstance: SkeletonBodyDebugRead<Error = Self::Error>;
    fn float(&mut self, route: SkeletonBodyAttributeRoute, key: u64) -> Result<f32, Self::Error>;
    fn byte(&mut self, route: SkeletonBodyAttributeRoute, key: u64) -> Result<u8, Self::Error>;
    fn debug_instance(&mut self) -> Result<Self::DebugInstance, Self::Error>;
}

pub(crate) struct SkeletonBodyAttributes {
    vault: Arc<AttributeRuntimeVault>,
    local_bones: AttributeRuntimeInstance,
    local_skeleton: AttributeRuntimeInstance,
    global_skeleton: AttributeRuntimeInstance,
    global_animation: AttributeRuntimeInstance,
    global_wipeout: AttributeRuntimeInstance,
}

impl SkeletonBodyAttributes {
    pub(crate) fn global_physics_animation_instance(&self) -> &AttributeRuntimeInstance {
        &self.global_animation
    }
    pub(crate) fn global_skeleton_instance(&self) -> &AttributeRuntimeInstance {
        &self.global_skeleton
    }
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>, local_collection: u64) -> Self {
        use SkeletonBodyAttributeRoute as R;
        Self {
            vault: Arc::clone(vault),
            local_bones: vault.instance(R::LocalBones.class_key(), local_collection),
            local_skeleton: vault.instance(R::LocalSkeleton.class_key(), local_collection),
            global_skeleton: vault.instance(R::GlobalSkeleton.class_key(), DEFAULT_COLLECTION),
            global_animation: vault.instance(R::GlobalAnimation.class_key(), DEFAULT_COLLECTION),
            global_wipeout: vault.instance(R::GlobalWipeout.class_key(), DEFAULT_COLLECTION),
        }
    }

    fn instance(&self, route: SkeletonBodyAttributeRoute) -> &AttributeRuntimeInstance {
        match route {
            SkeletonBodyAttributeRoute::LocalBones => &self.local_bones,
            SkeletonBodyAttributeRoute::LocalSkeleton => &self.local_skeleton,
            SkeletonBodyAttributeRoute::GlobalSkeleton => &self.global_skeleton,
            SkeletonBodyAttributeRoute::GlobalAnimation => &self.global_animation,
            SkeletonBodyAttributeRoute::GlobalWipeout => &self.global_wipeout,
        }
    }
}

impl SkeletonBodyAttributeRead for SkeletonBodyAttributes {
    type Error = AttributeRuntimeError;
    type DebugInstance = SkeletonBodyDebugInstance;

    fn float(&mut self, route: SkeletonBodyAttributeRoute, key: u64) -> Result<f32, Self::Error> {
        Ok(f32::from_bits(
            self.instance(route).try_scalar_word(key, 0)?.unwrap_or(0),
        ))
    }

    fn byte(&mut self, route: SkeletonBodyAttributeRoute, key: u64) -> Result<u8, Self::Error> {
        let Some(data) = self.instance(route).try_get_data(key, 0)? else {
            return Ok(0); // Source DefaultDataArea, not a default tuning choice.
        };
        let [value] = data.bytes() else {
            return Err(AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            });
        };
        Ok(*value)
    }

    fn debug_instance(&mut self) -> Result<Self::DebugInstance, Self::Error> {
        Ok(SkeletonBodyDebugInstance(
            self.vault.instance(DEBUG_PHYSICS_CLASS, DEFAULT_COLLECTION),
        ))
    }
}

pub(crate) struct SkeletonBodyDebugInstance(AttributeRuntimeInstance);

impl SkeletonBodyDebugRead for SkeletonBodyDebugInstance {
    type Error = AttributeRuntimeError;
    fn mode(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0.try_scalar_word(DEBUG_MODE, 0)?.unwrap_or(0))
    }
    fn factor(&mut self) -> Result<f32, Self::Error> {
        Ok(f32::from_bits(
            self.0.try_scalar_word(DEBUG_FACTOR, 0)?.unwrap_or(0),
        ))
    }
}
