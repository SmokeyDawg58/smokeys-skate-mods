//! Retained immutable-Vault reads for `BoneDrives::InitAttribs` (0x82b48890).
//!
//! Two retained slots model the observed collection/cached-layout
//! decisions of `Instance::Change` (0x82976260). Collection keys identify actual
//! collection objects only within this ONE retained immutable graph. This is
//! not shared global Change, editor Unmodify, reload or reference-count callback
//! parity. The producer must repeat both resolutions for each of 24 bones.
//! Generated animation fields read the selected cached16 layout; ordinary
//! `physics_skeleton_drives` FETCH uses the generic lookup path each time.
//! After construction the SAME selections survive in the actual drive owner;
//! Update reads `animation_default` lazily and captures each current cached layout.

use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    skeleton_drives::{SkeletonDriveAttributeRead, SkeletonDriveType},
};
use std::sync::Arc;

pub(crate) const ANIMATION_DRIVES_CLASS: u64 = 0x3432_360e_2c7d_1d8c;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "concrete Vault binding awaits live player construction"
    )
)]
pub(crate) const SKELETON_DRIVES_CLASS: u64 = 0xfdf6_1860_0d39_6c3b;
pub(crate) const LOCAL_COLLECTION: u64 = 0x0013_2d1e_84f0_bc32;
pub(crate) const LOCAL_ROOT_COLLECTION: u64 = 0x550b_2312_8e0c_3838;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "concrete Vault binding awaits live player construction"
    )
)]
pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

#[derive(Debug)]
pub(crate) enum SkeletonDriveAttributeError {
    Runtime(AttributeRuntimeError),
    NullCachedAnimationLayout { kind: SkeletonDriveType },
    LayoutOffset { offset: usize },
}

impl std::fmt::Display for SkeletonDriveAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "Skeleton drive attribute: {error}"),
            Self::NullCachedAnimationLayout { kind } => write!(
                f,
                "{kind:?} animation drive collection has a null cached layout"
            ),
            Self::LayoutOffset { offset } => write!(
                f,
                "animation drive scalar offset {offset} is outside the four layout words"
            ),
        }
    }
}
impl std::error::Error for SkeletonDriveAttributeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            _ => None,
        }
    }
}
impl From<AttributeRuntimeError> for SkeletonDriveAttributeError {
    fn from(value: AttributeRuntimeError) -> Self {
        Self::Runtime(value)
    }
}

#[derive(Default)]
struct AnimationDriveInstance {
    current: Option<(u64, AttributeRuntimeInstance)>,
    // None is the initializer's verified DefaultDataArea16, not an unknown
    // pointer. Some with a null cached layout is DIFFERENT and cannot default.
    cached_owner: Option<AttributeRuntimeInstance>,
}

/// No Clone/rebind-to-other-Vault operation: cached owners and current selection
/// have a single retained lifetime. Null Change can retain an older
/// cached allocation, so it is deliberately separate from current collection.
pub(crate) struct SkeletonDriveAttributes {
    vault: Arc<AttributeRuntimeVault>,
    animation: [AnimationDriveInstance; 2],
    global: AttributeRuntimeInstance,
    update_global: AttributeRuntimeInstance,
}

impl SkeletonDriveAttributes {
    pub(crate) fn global_animation_instance(&self) -> &AttributeRuntimeInstance {
        &self.update_global
    }
    /// Static initializer 0x82f7d7b0 starts both null slots at `DefaultDataArea16`.
    /// No animation collection is silently selected before `InitAttribs`.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "concrete Vault binding awaits live player construction"
        )
    )]
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            vault: Arc::clone(vault),
            animation: std::array::from_fn(|_| AnimationDriveInstance::default()),
            global: vault.instance(SKELETON_DRIVES_CLASS, DEFAULT_COLLECTION),
            update_global: vault.instance(0xbabb_3ec1_1084_9e08, DEFAULT_COLLECTION),
        }
    }

    fn change_animation(&mut self, kind: SkeletonDriveType, collection: u64) {
        // Re-resolve even when immutable identity will compare equal. Do not
        // collapse the 24 source InitAttribs calls to a constructor snapshot.
        let resolved = self.vault.instance(ANIMATION_DRIVES_CLASS, collection);
        let next_key = (!resolved.is_null()).then_some(collection);
        let slot = &mut self.animation[kind.index()];
        if slot.current.as_ref().map(|(key, _)| *key) == next_key {
            return; // Same collection pointer: native complete no-op.
        }
        slot.current = next_key.map(|key| (key, resolved.clone()));
        if next_key.is_some() {
            slot.cached_owner = Some(resolved);
        }
        // Null new collection leaves the previous cached layout untouched.
    }
}

/// Captured once at `EnableDrive_NormalAnimFollow` (0x82b3d828).
/// None is the verified initial `DefaultDataArea16`, not a missing live layout.
pub(crate) struct SkeletonDriveAnimationLayout(Option<AttributeRuntimeData>);

impl crate::skeleton_drives::update::SkeletonDrivesUpdateAttributeRead for SkeletonDriveAttributes {
    type Pointer = Option<AttributeRuntimeData>;
    type Layout = SkeletonDriveAnimationLayout;
    type Error = SkeletonDriveAttributeError;

    fn lookup_global(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        Ok(self.update_global.try_get_data(key, 0)?)
    }

    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        SkeletonDriveAttributeRead::float(self, pointer)
    }

    fn byte(&mut self, pointer: &Self::Pointer) -> Result<u8, Self::Error> {
        let Some(data) = pointer else { return Ok(0) };
        let [value] = data.bytes() else {
            return Err(AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            }
            .into());
        };
        Ok(*value)
    }

    fn capture_animation_layout(
        &mut self,
        kind: SkeletonDriveType,
    ) -> Result<Self::Layout, Self::Error> {
        let Some(instance) = &self.animation[kind.index()].cached_owner else {
            return Ok(SkeletonDriveAnimationLayout(None));
        };
        let layout = instance
            .try_cached_layout(16)?
            .ok_or(SkeletonDriveAttributeError::NullCachedAnimationLayout { kind })?;
        Ok(SkeletonDriveAnimationLayout(Some(layout)))
    }

    fn animation_float(
        &mut self,
        layout: &Self::Layout,
        byte_offset: usize,
    ) -> Result<f32, Self::Error> {
        if !matches!(byte_offset, 0 | 4 | 8 | 12) {
            return Err(SkeletonDriveAttributeError::LayoutOffset {
                offset: byte_offset,
            });
        }
        let Some(data) = &layout.0 else {
            return Ok(0.0);
        };
        Ok(f32::from_be_bytes(
            data.bytes()[byte_offset..byte_offset + 4]
                .try_into()
                .expect("captured cached16 scalar range"),
        ))
    }
}

impl SkeletonDriveAttributeRead for SkeletonDriveAttributes {
    type Pointer = Option<AttributeRuntimeData>;
    type Error = SkeletonDriveAttributeError;

    fn find_and_change_animation_drive(
        &mut self,
        kind: SkeletonDriveType,
    ) -> Result<(), Self::Error> {
        self.change_animation(
            kind,
            match kind {
                SkeletonDriveType::Local => LOCAL_COLLECTION,
                SkeletonDriveType::LocalRoot => LOCAL_ROOT_COLLECTION,
            },
        );
        Ok(())
    }

    fn lookup_global(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        // Each source call reruns actual child/parent/node resolution, index0.
        Ok(self.global.try_get_data(key, 0)?)
    }

    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        let Some(data) = pointer else {
            return Ok(0.0);
        }; // DefaultDataArea4
        let bytes = data
            .bytes()
            .try_into()
            .map_err(|_| AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            })?;
        Ok(f32::from_be_bytes(bytes))
    }

    fn animation_float(
        &mut self,
        kind: SkeletonDriveType,
        byte_offset: usize,
    ) -> Result<f32, Self::Error> {
        if !matches!(byte_offset, 0 | 4 | 8 | 12) {
            return Err(SkeletonDriveAttributeError::LayoutOffset {
                offset: byte_offset,
            });
        }
        let Some(instance) = &self.animation[kind.index()].cached_owner else {
            return Ok(0.0); // Initial generated DefaultDataArea16 only.
        };
        // Never substitute generic GetData, parent layout, or generated default
        // after changing to a nonnull collection with a null layout pointer.
        let layout = instance
            .try_cached_layout(16)?
            .ok_or(SkeletonDriveAttributeError::NullCachedAnimationLayout { kind })?;
        Ok(f32::from_be_bytes(
            layout.bytes()[byte_offset..byte_offset + 4]
                .try_into()
                .expect("checked cached16 scalar range"),
        ))
    }
}

#[cfg(test)]
#[path = "skeleton_drive_attributes_tests.rs"]
mod tests;

impl crate::skeleton_drives::ragdoll_update::RagdollDriveAttributes for SkeletonDriveAttributes {
    fn lookup_skeleton_drive(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        Ok(self.global.try_get_data(key, 0)?)
    }
}
