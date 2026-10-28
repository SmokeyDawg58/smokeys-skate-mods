//! Unregistered Body-attributes child, actual Manual global+c8 cached layout.
use super::{SkateboardAttributeError, SkateboardAttributes};
use crate::physics::PointGraph;
impl SkateboardAttributes {
    /// Source32704..24 captures layout before its absolute-speed input load.
    pub(crate) fn capture_manual_effect_speed_graph(
        &self,
    ) -> Result<PointGraph<8>, SkateboardAttributeError> {
        let data = self
            .manual
            .try_cached_layout(0xa0)
            .map_err(SkateboardAttributeError::Runtime)?;
        let word = |offset: usize| {
            data.as_ref().map_or(0.0, |v| {
                f32::from_be_bytes(
                    v.bytes()[offset..offset + 4]
                        .try_into()
                        .expect("manual layout bounds"),
                )
            })
        };
        Ok(PointGraph {
            x: std::array::from_fn(|i| word(0x60 + i * 4)),
            y: std::array::from_fn(|i| word(0x80 + i * 4)),
        })
    }
}
