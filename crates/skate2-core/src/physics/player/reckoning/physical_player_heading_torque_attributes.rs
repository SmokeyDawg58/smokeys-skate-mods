//! Unregistered child of the retained Heading attributes owner.
use super::HeadingAttributes;
use crate::{attribute_runtime::AttributeRuntimeError, physics::PointGraph};

impl HeadingAttributes {
    /// Capture the actual generated layout once, before later input reads.
    pub(crate) fn capture_torque_graph<const N: usize>(
        &self,
        offset: usize,
    ) -> Result<PointGraph<N>, AttributeRuntimeError> {
        let layout = self.instance.try_cached_layout(0x250)?;
        let word = |index: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[index..index + 4]
                        .try_into()
                        .expect("heading graph range"),
                )
            })
        };
        Ok(PointGraph {
            x: std::array::from_fn(|i| word(offset + i * 4)),
            y: std::array::from_fn(|i| word(offset + N * 4 + i * 4)),
        })
    }
    pub(crate) fn torque_scalar(&self, key: u64) -> Result<f32, AttributeRuntimeError> {
        let Some(value) = self.instance.try_get_data(key, 0)? else {
            return Ok(0.0); // actual DefaultDataArea4
        };
        Ok(f32::from_be_bytes(value.bytes().try_into().map_err(
            |_| AttributeRuntimeError::ScalarSize {
                size: value.bytes().len(),
            },
        )?))
    }
}
