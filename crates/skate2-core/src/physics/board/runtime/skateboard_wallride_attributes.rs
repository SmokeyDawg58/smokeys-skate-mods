//! Child of actual feet attributes; global trajectory is a distinct class.
use super::SkateboardFootVaultAttributes;
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    physics::PointGraph,
};
use std::sync::Arc;
impl SkateboardFootVaultAttributes {
    /// Generated feet ctor828b0e10 uses layout0xf0; `WallRide295a4` selects+10.
    pub(crate) fn try_wallride_graph(&self, time: f32) -> Result<f32, AttributeRuntimeError> {
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
            x: std::array::from_fn(|i| word(0x10 + i * 4)),
            y: std::array::from_fn(|i| word(0x30 + i * 4)),
        }
        .evaluate(time))
    }
}
/// Retained projection of actual GlobalAttributeCollection+e8; cannot use the
/// different `HandPlant` embedded trajectory class2656db32bef0b68d.
#[derive(Clone)]
pub(crate) struct WallRideTrajectoryAttributes(AttributeRuntimeInstance);
impl WallRideTrajectoryAttributes {
    pub(crate) const CLASS: u64 = 0x429a_8f43_85dd_539d;
    /// Global constructor calls generated ctor828b1068 at828b291c. Bind once at
    /// owner admission, not per calculation. This is not full singleton Init.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self(vault.instance(Self::CLASS, 0xd7ed_bd36_2d7d_2152))
    }
    pub(crate) fn capture(&self) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        self.0.try_cached_layout(0x24c)
    }
    /// Native FETCH followed by a float load; only null selects `DefaultDataArea4`.
    pub(crate) fn try_scalar(&self, key: u64) -> Result<f32, AttributeRuntimeError> {
        let Some(data) = self.0.try_get_data(key, 0)? else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] =
            data.bytes()
                .try_into()
                .map_err(|_| AttributeRuntimeError::ScalarSize {
                    size: data.bytes().len(),
                })?;
        Ok(f32::from_be_bytes(bytes))
    }
    /// `GetAttributePointer(key, 0)`; null selects `DefaultDataArea`.
    pub(crate) fn try_data(
        &self,
        key: u64,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        self.0.try_get_data(key, 0)
    }
    pub(crate) fn height(data: Option<&AttributeRuntimeData>) -> f32 {
        data.map_or(0.0, |d| {
            f32::from_be_bytes(
                d.bytes()[0x1a0..0x1a4]
                    .try_into()
                    .expect("captured trajectory layout"),
            )
        })
    }
}
