//! Registered selected-surface child; no new surface/global instance.
use super::{SkateboardSelectedSurface, SkateboardSlideCollisionAttributeError};
use crate::attribute_runtime::AttributeRuntimeData;
impl SkateboardSelectedSurface {
    /// `SpeedModel` a29c captures existing layout then loads +104 at a2a0.
    pub(crate) fn try_speed_model_maximum_difference(
        &self,
    ) -> Result<f32, SkateboardSlideCollisionAttributeError> {
        let layout = self
            .instance
            .try_cached_layout(0x10c)
            .map_err(SkateboardSlideCollisionAttributeError::Runtime)?;
        let bytes = layout
            .as_ref()
            .map_or(&[0; 0x10c][..], AttributeRuntimeData::bytes);
        Ok(f32::from_be_bytes(
            bytes[0x104..0x108]
                .try_into()
                .expect("checked surface layout"),
        ))
    }
}
