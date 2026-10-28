//! Skeleton+3e34/+3e38, distinct from processed+954/+958 publication.
//! Selected ctor stores57478/57480 follow `IK5743c` and omitted wobble57468.
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    skeleton_processed_contact::jump_events::SkeletonJumpVaultAttributes,
};
use std::sync::Arc;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkeletonJumpAdjustmentError {
    ConstructorRequired,
    AlreadyConstructed,
}
impl std::fmt::Display for SkeletonJumpAdjustmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ConstructorRequired => "Skeleton jump adjustments require constructor",
            Self::AlreadyConstructed => "Skeleton jump adjustments already constructed",
        })
    }
}
/// No Clone: this selected projection moves from construction to the actual
/// retained Skeleton owner. Allocation does not establish numerical defaults.
pub(crate) struct SkeletonJumpAdjustments {
    values: Option<[f32; 2]>,
}
impl SkeletonJumpAdjustments {
    pub(crate) const fn uninitialized() -> Self {
        Self { values: None }
    }
    pub(crate) fn initialize_constructor(&mut self) -> Result<(), SkeletonJumpAdjustmentError> {
        if self.values.is_some() {
            return Err(SkeletonJumpAdjustmentError::AlreadyConstructed);
        }
        self.values = Some([0.0; 2]);
        Ok(())
    }
    pub(crate) fn x(&self) -> Result<f32, SkeletonJumpAdjustmentError> {
        Ok(self
            .values
            .ok_or(SkeletonJumpAdjustmentError::ConstructorRequired)?[0])
    }
    pub(crate) fn z(&self) -> Result<f32, SkeletonJumpAdjustmentError> {
        Ok(self
            .values
            .ok_or(SkeletonJumpAdjustmentError::ConstructorRequired)?[1])
    }
    pub(crate) fn store_x(&mut self, value: f32) -> Result<(), SkeletonJumpAdjustmentError> {
        self.values
            .as_mut()
            .ok_or(SkeletonJumpAdjustmentError::ConstructorRequired)?[0] = value;
        Ok(())
    }
    pub(crate) fn store_z(&mut self, value: f32) -> Result<(), SkeletonJumpAdjustmentError> {
        self.values
            .as_mut()
            .ok_or(SkeletonJumpAdjustmentError::ConstructorRequired)?[1] = value;
        Ok(())
    }
}
impl SkeletonJumpVaultAttributes {
    /// Actual `GlobalAttributeCollection` +124 (`anim_motion.jumping`) and +d0
    /// (`physics_jump.default`), ctor2cc8/2750. Bind once in retained caller;
    /// immutable Vault capture does not model singleton replacement/editor reload.
    pub(crate) fn bind_global(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self::from_retained_instances(
            vault.instance(0x90c3_decf_0105_631e, 0xac26_0b44_fa3c_a0a4),
            vault.instance(0xc2e0_a8b6_c139_90fd, 0xd7ed_bd36_2d7d_2152),
        )
    }
}
#[cfg(test)]
#[path = "skeleton_jump_adjustments_tests.rs"]
mod tests;
