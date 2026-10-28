//! Late reads from the SAME constructor-bound speed-wobble attribute instance.
use super::{super::SkateboardSpeedWobbleAttributes, WobbleAttributes};
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    physics::PointGraph,
};

pub(crate) struct WobbleVault<'a>(pub(crate) &'a SkateboardSpeedWobbleAttributes);
impl WobbleAttributes for WobbleVault<'_> {
    type Error = AttributeRuntimeError;
    type Scalar = Option<AttributeRuntimeData>;
    fn capture(&mut self, key: u64) -> Result<Self::Scalar, Self::Error> {
        self.0.instance.try_get_data(key, 0)
    }
    fn load(&mut self, scalar: &Self::Scalar) -> Result<f32, Self::Error> {
        let Some(data) = scalar else {
            return Ok(0.0);
        };
        let bytes = data.bytes();
        let word = bytes
            .try_into()
            .map_err(|_| AttributeRuntimeError::ScalarSize { size: bytes.len() })?;
        Ok(f32::from_be_bytes(word))
    }
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error> {
        assert!(
            matches!(offset, 0 | 0x40 | 0x80 | 0xc0),
            "native wobble graph offset"
        );
        let layout = self.0.try_capture_layout()?;
        let bytes = &layout.bytes()[offset..offset + 0x40];
        let word =
            |at| f32::from_be_bytes(bytes[at..at + 4].try_into().expect("checked wobble layout"));
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| word(i * 4)),
            y: std::array::from_fn(|i| word(0x20 + i * 4)),
        }
        .evaluate(input))
    }
}
