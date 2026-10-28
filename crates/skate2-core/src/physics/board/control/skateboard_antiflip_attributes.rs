//! child of actual feet attributes: `AntiFlip`'s two generated graphs.
use super::SkateboardFootVaultAttributes;
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    physics::PointGraph,
};
impl SkateboardFootVaultAttributes {
    pub(crate) fn try_antiflip_graph(
        &self,
        x_offset: usize,
        input: f32,
    ) -> Result<f32, AttributeRuntimeError> {
        assert!(
            matches!(x_offset, 0xb0 | 0x60),
            "source AntiFlip graph offset"
        );
        let layout = self.instance.try_cached_layout(0xf0)?;
        let bytes = layout
            .as_ref()
            .map_or(&[0; 0xf0][..], AttributeRuntimeData::bytes);
        let word = |offset| {
            f32::from_be_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("checked feet layout"),
            )
        };
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| word(x_offset + i * 4)),
            y: std::array::from_fn(|i| word(x_offset + 32 + i * 4)),
        }
        .evaluate(input))
    }
}
