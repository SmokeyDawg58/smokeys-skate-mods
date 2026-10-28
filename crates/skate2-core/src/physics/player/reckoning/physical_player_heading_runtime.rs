//! `CalculateHeadingAdjustFactor`82e00288 with the retained global projection.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    physics::PointGraph,
    reckoning_numeric::{ReckoningNumeric, try_acos},
};
use std::sync::Arc;
pub(crate) struct HeadingAttributes {
    instance: AttributeRuntimeInstance,
}
impl HeadingAttributes {
    /// Global+fc constructor828b2a34; selected projection, not full singleton Init.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(0x3a2a_8b50_4536_22bb, 0xd7ed_bd36_2d7d_2152),
        }
    }
    pub(crate) fn graph(&self, offset: usize, input: f32) -> Result<f32, AttributeRuntimeError> {
        let layout = self.instance.try_cached_layout(0x250)?;
        let word = |i: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[i..i + 4]
                        .try_into()
                        .expect("heading layout range"),
                )
            })
        };
        let graph = PointGraph {
            x: std::array::from_fn::<_, 8, _>(|i| word(offset + i * 4)),
            y: std::array::from_fn::<_, 8, _>(|i| word(offset + 32 + i * 4)),
        };
        Ok(graph.evaluate(input))
    }
    pub(crate) fn speed_limit(&self) -> Result<f32, AttributeRuntimeError> {
        let value = self.instance.try_get_data(0x97bb_45bb_a1c3_68c7, 0)?;
        let Some(value) = value else {
            return Ok(0.0);
        }; // actual DefaultDataArea4
        Ok(f32::from_be_bytes(value.bytes().try_into().map_err(
            |_| AttributeRuntimeError::ScalarSize {
                size: value.bytes().len(),
            },
        )?))
    }
}
pub(crate) fn normalized_angle<N: ReckoningNumeric>(
    normal_y: f32,
    seed: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let divisor = f32::from_bits(0x3fc9_0fdb);
    let residual = -seed.mul_add(divisor, -1.0);
    let refined = seed.mul_add(residual, seed);
    // VMX min/max propagate unordered input; no host max() NaN elimination.
    let bounded = if normal_y.is_nan() {
        normal_y
    } else {
        normal_y.clamp(-1.0, 1.0)
    };
    let angle = try_acos(bounded, numeric)?;
    let residual = -refined.mul_add(divisor, -1.0);
    Ok(refined.mul_add(residual, refined) * angle)
}
/// Scalar fsel chooses its positive arm for either signed zero; NaN selects negative.
pub(crate) fn clamp_speed(speed: f32, maximum: f32) -> f32 {
    let absolute = speed.abs();
    let lower = if -absolute >= 0.0 { 0.0 } else { absolute };
    if maximum - lower >= 0.0 {
        lower
    } else {
        maximum
    }
}
#[cfg(test)]
mod tests {
    use super::clamp_speed;
    #[test]
    fn live_debug_default_and_relinked_value() {
        use crate::input_cvar_runtime::{InputCvarManager, InputCvarReferences};
        let refs = InputCvarReferences::new_static_initialized();
        assert!(refs.try_draw_vert_turning_input().is_err());
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert!(!refs.try_draw_vert_turning_input().unwrap());
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkDrawVertTurningInput=1\r\n"))
            .unwrap();
        assert!(refs.try_draw_vert_turning_input().unwrap());
    }
    #[test]
    fn native_acos_endpoint_residual_is_preserved() {
        use crate::reckoning_numeric::HostReckoningNumeric;
        let value = super::normalized_angle(
            1.0,
            1.0 / f32::from_bits(0x3fc9_0fdb),
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert!(value > 0.0 && value < 0.001);
    }
    #[test]
    fn scalar_fsel_preserves_unordered_and_negative_limit_semantics() {
        assert_eq!(clamp_speed(f32::NAN, 7.0).to_bits(), 7.0_f32.to_bits());
        assert!(clamp_speed(3.0, f32::NAN).is_nan());
        assert_eq!(clamp_speed(-0.0, 7.0).to_bits(), 0);
        assert_eq!(clamp_speed(3.0, -2.0).to_bits(), (-2.0_f32).to_bits());
    }
}

#[path = "physical_player_heading_torque_attributes.rs"]
pub(crate) mod torque_attributes;
