//! Retained Global+f8 Pumping attributes; constructor828b29fc / wrapper828b11d0.
//! Runtime lookups retain the actual global instance and captured data pointers.
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    physics::PointGraph,
};
use std::sync::Arc;

const CLASS: u64 = 0xab3a_006f_3a29_56e2;
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
const LAYOUT_SIZE: usize = 0xc0;

pub(crate) struct PumpingVaultAttributes {
    instance: AttributeRuntimeInstance,
    #[cfg(feature = "skate3-runtime")]
    tuning: Option<Arc<crate::skate3_native::physics::player::tuning::Tuning>>,
}

pub(crate) trait PumpingAttributeRead {
    type Pointer;
    type Error;
    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn scalar(&mut self, pointer: &Self::Pointer, key: u64) -> Result<f32, Self::Error>;
    fn keyed_graph(&mut self, key: u64, input: f32) -> Result<f32, Self::Error>;
    fn layout_graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error>;
}

impl PumpingAttributeRead for PumpingVaultAttributes {
    type Pointer = Option<AttributeRuntimeData>;
    type Error = AttributeRuntimeError;
    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        PumpingVaultAttributes::lookup(self, key)
    }
    fn scalar(&mut self, pointer: &Self::Pointer, key: u64) -> Result<f32, Self::Error> {
        self.load_for_player(pointer.as_ref(), key)
    }
    fn keyed_graph(&mut self, key: u64, input: f32) -> Result<f32, Self::Error> {
        PumpingVaultAttributes::keyed_graph(self, key, input)
    }
    fn layout_graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error> {
        PumpingVaultAttributes::layout_graph(self, offset, input)
    }
}

impl PumpingVaultAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(CLASS, DEFAULT),
            #[cfg(feature = "skate3-runtime")]
            tuning: None,
        }
    }

    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn set_mod_tuning(&mut self, tuning: Option<Arc<crate::skate3_native::physics::player::tuning::Tuning>>) { self.tuning = tuning; }
    pub(crate) fn load_for_player(&self, pointer: Option<&AttributeRuntimeData>, key: u64) -> Result<f32, AttributeRuntimeError> {
        #[cfg(feature = "skate3-runtime")]
        if let Some(value) = self.tuning.as_ref().and_then(|tuning|tuning.pumping(key)) { return Ok(value); }
        Self::scalar(pointer, key)
    }
    /// Each invocation performs the actual runtime search at index0. Keeping
    /// the allocation separate from its value read preserves Calculate6a8..6e8.
    pub(crate) fn lookup(
        &self,
        key: u64,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        self.instance.try_get_data(key, 0)
    }

    pub(crate) fn scalar(
        pointer: Option<&AttributeRuntimeData>,
        key: u64,
    ) -> Result<f32, AttributeRuntimeError> {
        let Some(pointer) = pointer else {
            return Ok(0.0);
        };
        let bytes = pointer.bytes();
        let word = bytes
            .try_into()
            .map_err(|_| AttributeRuntimeError::PayloadSize {
                key,
                expected: 4,
                actual: bytes.len(),
            })?;
        Ok(f32::from_be_bytes(word))
    }

    /// `PointNegGraphData8` metadata is not consumed by Evaluate8.
    pub(crate) fn keyed_graph(&self, key: u64, input: f32) -> Result<f32, AttributeRuntimeError> {
        let pointer = self.lookup(key)?;
        let Some(pointer) = pointer else {
            return Ok(PointGraph::<8>::default().evaluate(input));
        };
        let bytes = pointer.bytes();
        if bytes.len() != 0x50 {
            return Err(AttributeRuntimeError::PayloadSize {
                key,
                expected: 0x50,
                actual: bytes.len(),
            });
        }
        Ok(evaluate_graph(&bytes[0x10..0x50], input))
    }

    /// Caller passes one of source X offsets00,40,80; each invocation captures
    /// the current generated layout independently, never a generic parent node.
    pub(crate) fn layout_graph(
        &self,
        offset: usize,
        input: f32,
    ) -> Result<f32, AttributeRuntimeError> {
        assert!(
            matches!(offset, 0 | 0x40 | 0x80),
            "source Pumping layout graph offset"
        );
        let layout = self.instance.try_cached_layout(LAYOUT_SIZE)?;
        let Some(layout) = layout else {
            return Ok(PointGraph::<8>::default().evaluate(input));
        };
        Ok(evaluate_graph(
            &layout.bytes()[offset..offset + 0x40],
            input,
        ))
    }
}

fn evaluate_graph(bytes: &[u8], input: f32) -> f32 {
    let word = |offset| {
        f32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("checked graph region"),
        )
    };
    PointGraph::<8> {
        x: std::array::from_fn(|i| word(i * 4)),
        y: std::array::from_fn(|i| word(0x20 + i * 4)),
    }
    .evaluate(input)
}
