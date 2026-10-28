//! selected-surface child: source `StraightenOutTorque82b286bc`.
use super::{SkateboardSelectedSurface, SkateboardSlideCollisionAttributeError};
impl SkateboardSelectedSurface {
    pub(crate) fn try_straighten_strength(
        &self,
    ) -> Result<f32, SkateboardSlideCollisionAttributeError> {
        let Some(data) = self
            .instance
            .try_cached_layout(0x10c)
            .map_err(SkateboardSlideCollisionAttributeError::Runtime)?
        else {
            return Ok(0.0);
        };
        Ok(f32::from_be_bytes(
            data.bytes()[0xe8..0xec]
                .try_into()
                .expect("surface layout range"),
        ))
    }
}
