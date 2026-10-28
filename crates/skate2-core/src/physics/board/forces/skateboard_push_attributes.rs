//! Retained global `physics_push.default` capability, not a per-call binding.
use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct PushAttributes(AttributeRuntimeInstance);
impl PushAttributes {
    pub(crate) const CLASS: u64 = 0xd4d0_4d2f_a87a_9a31;
    /// Global constructor828b2510..2544: class, default collection, then +b0.
    /// Admission of this selected capability does not certify full global Init.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self(vault.instance(Self::CLASS, 0xd7ed_bd36_2d7d_2152))
    }
    pub(crate) fn capture(
        &self,
        key: u64,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        self.0.try_get_data(key, 0)
    }
    pub(crate) fn load(data: Option<&AttributeRuntimeData>) -> Result<f32, AttributeRuntimeError> {
        let Some(data) = data else {
            return Ok(0.0); // Source null pointer alone selects DefaultDataArea(4).
        };
        let bytes = data
            .bytes()
            .try_into()
            .map_err(|_| AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            })?;
        Ok(f32::from_be_bytes(bytes))
    }
}
