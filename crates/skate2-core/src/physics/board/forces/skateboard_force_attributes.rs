//! Skateboard's OWN physicsdeck instance at +534, constructed at 0x82b38b6c.
//! Its collection differs from the global default used by body construction.
//! Immutable Vault bindings do not model editor reload or callback mutation of
//! the retained force queue across `ApplyForces`' first attribute lookup.
use crate::attribute_runtime::{
    AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::sync::Arc;

pub(crate) const CLASS: u64 = 0x9ed2_91a2_7445_3fc0;
pub(crate) const OWN_COLLECTION: u64 = 0x5507_40b0_0aa8_d4ac;
pub(crate) const LOCAL_Y_OFFSET: u64 = 0xde53_599e_9c53_fa4a;
pub(crate) const FINISH_LOOKUP: u64 = 0x93d8_4377_c4da_2d5b;

pub(crate) trait SkateboardQueuedForceAttributes {
    type Error;
    fn local_y_offset(&mut self) -> Result<f32, Self::Error>;
    fn finish_lookup(&mut self) -> Result<(), Self::Error>;
}

/// One retained binding, mirroring `FindCollection` at 0x82b38b5c and own
/// Instance construction at 0x82b38b6c. Native message port is zero. Timing
/// belongs to the actual outer board owner, not to a fresh Apply invocation.
pub(crate) struct SkateboardQueuedForceVaultAttributes {
    instance: AttributeRuntimeInstance,
}
impl SkateboardQueuedForceVaultAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(CLASS, OWN_COLLECTION),
        }
    }
}
impl SkateboardQueuedForceAttributes for SkateboardQueuedForceVaultAttributes {
    type Error = AttributeRuntimeError;
    /// Lookup 0x82b353c4 then scalar load 0x82b353dc. Only missing pointers
    /// select the verified zero-filled DefaultDataArea(4).
    fn local_y_offset(&mut self) -> Result<f32, Self::Error> {
        self.instance
            .try_scalar_word(LOCAL_Y_OFFSET, 0)
            .map(|word| f32::from_bits(word.unwrap_or(0)))
    }
    /// Lookup 0x82b35584 follows ALL queued body writes. Source selects
    /// DefaultDataArea(1) for absence but never loads a byte from either
    /// pointer. The shared runtime's checked pointer/schema boundary remains;
    /// there is no new bool validation, size-one requirement or payload decode.
    fn finish_lookup(&mut self) -> Result<(), Self::Error> {
        self.instance.try_get_data(FINISH_LOOKUP, 0)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "skateboard_force_attributes_tests.rs"]
mod tests;
