//! Actual `SkeletonState82b50d08` members before any `SkeletonData` or Body Init.
//! The enclosing skeleton moves these same owners into its runtime projections.
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    ragdoll::SKELETON_PART_COUNT,
    skeleton_animation_record::SkeletonAnimationRecord,
    skeleton_physics_history::SkeletonPhysicsHistory,
};
use std::sync::Arc;

pub(crate) struct SkeletonStateConstructorParts {
    pub(crate) attributes: AttributeRuntimeInstance,
    /// Generated physics wrapper827b2660 retains actual cached layout or the
    /// shared zero `DefaultDataArea32`. None here means that constructed fallback.
    pub(crate) layout: Option<AttributeRuntimeData>,
    pub(crate) animation_record: SkeletonAnimationRecord,
    pub(crate) history: SkeletonPhysicsHistory,
    pub(crate) linear_error: [[f32; 4]; SKELETON_PART_COUNT],
    pub(crate) proxy_target: crate::skeleton_root_transforms::SkeletonRootVector,
}
impl SkeletonStateConstructorParts {
    /// Source82b50d30 attributes, d38 `AnimRecord`, d40 `PhysRecord`, then five
    /// vectors, three scalar stores and mass/error `MemClear` calls in order.
    /// Existing record projections own the four COM vectors and reset scalar.
    pub(crate) fn try_construct(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let attributes = vault.instance(0x0429_3d0f_2adc_fbb4, 0xd7ed_bd36_2d7d_2152);
        let layout = attributes.try_cached_layout(32)?;
        let animation_record = SkeletonAnimationRecord::new();
        let history = SkeletonPhysicsHistory::new([0.0; SKELETON_PART_COUNT]);
        Ok(Self {
            attributes,
            layout,
            animation_record,
            history,
            linear_error: [[0.0; 4]; SKELETON_PART_COUNT],
            proxy_target: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
        })
    }
}
