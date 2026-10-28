//! Unregistered child reusing the actual retained global+c8 Manual instance.
use super::{SkateboardAttributeError, SkateboardAttributes};
use crate::physics::PointGraph;
impl SkateboardAttributes {
    /// Generated constructor828b0ed8 requests DefaultDataArea(0xa0).
    /// Capture graph X10/Y30 before Heading's later deck-speed read.
    pub(crate) fn capture_heading_manual_graph(
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
                        .expect("manual graph range"),
                )
            })
        };
        Ok(PointGraph {
            x: std::array::from_fn(|i| word(0x10 + i * 4)),
            y: std::array::from_fn(|i| word(0x30 + i * 4)),
        })
    }
}
