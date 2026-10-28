//! Actual `PhysicsAir::CalculateBodySpinSpeed`82d841a8.
use super::PhysicsAirState;
use crate::{attribute_runtime::AttributeRuntimeError, physics::PointGraph};

#[derive(Debug)]
pub(crate) enum AirSpinTargetError {
    Missing(&'static str),
    Poisoned,
    Attribute(AttributeRuntimeError),
}
impl std::fmt::Display for AirSpinTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("AirSpinTargetError: Poisoned"),
            Self::Missing(error) => write!(f, "AirSpinTargetError Missing: {error:?}"),
            Self::Attribute(error) => write!(f, "AirSpinTargetError Attribute: {error:?}"),
        }
    }
}

impl PhysicsAirState {
    pub(crate) fn try_air_scalar(&self, key: u64) -> Result<f32, AttributeRuntimeError> {
        self.attributes
            .try_scalar_word(key, 0)
            .map(|value| value.map_or(0.0, f32::from_bits))
    }

    /// Source FETCH precedes the Processed+960 read; cached graph has no fallback.
    pub(crate) fn try_body_spin_target(&self) -> Result<f32, AirSpinTargetError> {
        use AirSpinTargetError as E;
        let owner = self
            .bindings
            .owner
            .upgrade()
            .ok_or(E::Missing("physical owner"))?;
        let processed = std::sync::Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let scale = self
            .attributes
            .try_scalar_word(0xc84e_b5b5_a3d7_dd4a, 0)
            .map_err(E::Attribute)?
            .map_or(0.0, f32::from_bits);
        let spin = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .animation_attribute_fields()
            .body_spin
            .ok_or(E::Missing("Processed body spin"))?;
        let degrees = spin * scale;
        let layout = self
            .attributes
            .try_cached_layout(0x130)
            .map_err(E::Attribute)?
            .ok_or(E::Missing("Air cached body-spin graph"))?;
        let radians = degrees * f32::from_bits(0x3c8e_fa35);
        let word = |offset: usize| {
            f32::from_be_bytes(
                layout.bytes()[offset..offset + 4]
                    .try_into()
                    .expect("bounded cached graph"),
            )
        };
        let graph = PointGraph::<8> {
            x: std::array::from_fn(|i| word(0xf0 + 4 * i)),
            y: std::array::from_fn(|i| word(0x110 + 4 * i)),
        };
        let blend = graph.evaluate(self.fields.time_in_state * 2.0);
        Ok(blend * radians)
    }
}
