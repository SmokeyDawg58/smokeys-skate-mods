//! Borrowed existing steering+manual instances; no per-call global binding.
use super::{SkateboardSteeringVaultAttributes, calculate::SteeringCalculationAttributes};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    physics::PointGraph,
    skateboard_attributes::{
        SkateboardAttributeClass, SkateboardAttributeError, SkateboardAttributeRead,
        SkateboardAttributes,
    },
};
pub(crate) struct SteeringCalculateVault<'a> {
    pub(crate) steering: &'a mut SkateboardSteeringVaultAttributes,
    pub(crate) manual: &'a mut SkateboardAttributes,
}
#[derive(Debug)]
pub(crate) enum SteeringCalculateAttributeError {
    Steering(AttributeRuntimeError),
    Manual(SkateboardAttributeError),
}
impl std::fmt::Display for SteeringCalculateAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Steering(e) => write!(f, "steering Vault: {e}"),
            Self::Manual(e) => write!(f, "manual Vault: {e}"),
        }
    }
}
impl SteeringCalculationAttributes for SteeringCalculateVault<'_> {
    type Error = SteeringCalculateAttributeError;
    fn steering_scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        let Some(data) = self
            .steering
            .0
            .try_get_data(key, 0)
            .map_err(Self::Error::Steering)?
        else {
            return Ok(0.0);
        };
        let bytes = data.bytes();
        let word = bytes.try_into().map_err(|_| {
            Self::Error::Steering(AttributeRuntimeError::ScalarSize { size: bytes.len() })
        })?;
        Ok(f32::from_be_bytes(word))
    }
    fn manual_scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        self.manual
            .read_float(SkateboardAttributeClass::Manual, key)
            .map_err(Self::Error::Manual)
    }
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error> {
        assert!(matches!(offset, 0 | 0x40), "native steering graph offset");
        let layout = self
            .steering
            .0
            .try_cached_layout(0x80)
            .map_err(Self::Error::Steering)?;
        let Some(layout) = layout else {
            return Ok(PointGraph::<8>::default().evaluate(input));
        };
        let bytes = &layout.bytes()[offset..offset + 0x40];
        let word = |offset| {
            f32::from_be_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("checked steering layout"),
            )
        };
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| word(i * 4)),
            y: std::array::from_fn(|i| word(0x20 + i * 4)),
        }
        .evaluate(input))
    }
}
