//! GlobalAttributeCollection+184 retained BodySpin/default route.
//! Constructor828b3390..33c4 proves class05ec9cb03a5b2eee; `NegGraph8` has
//! metadata16 bytes then X[8]/Y[8], with native `DefaultDataArea80` on missing.
use super::{AttributeRuntimeData, AttributeRuntimeError, PointGraph, ReckoningVaultAttributes};
use crate::reckoning_body_spin::air::BodySpinAirAttributes;
impl BodySpinAirAttributes for ReckoningVaultAttributes {
    fn spin_lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        Ok(self.body_spin.try_get_data(key, 0)?)
    }
    fn spin_graph(&mut self, pointer: &Self::Pointer, input: f32) -> Result<f32, Self::Error> {
        let zeros = [0u8; 80];
        let bytes = pointer
            .as_ref()
            .map_or(&zeros[..], AttributeRuntimeData::bytes);
        if bytes.len() != 80 {
            return Err(AttributeRuntimeError::ScalarSize { size: bytes.len() }.into());
        }
        let word = |i: usize| {
            f32::from_be_bytes(
                bytes[i * 4..i * 4 + 4]
                    .try_into()
                    .expect("validated `NegGraph8`"),
            )
        };
        Ok(PointGraph {
            x: std::array::from_fn::<_, 8, _>(|i| word(i + 4)),
            y: std::array::from_fn::<_, 8, _>(|i| word(i + 12)),
        }
        .evaluate(input))
    }
}
