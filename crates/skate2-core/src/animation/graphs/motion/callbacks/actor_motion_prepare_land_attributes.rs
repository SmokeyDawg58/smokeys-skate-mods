//! Proposed child `input_cvar_runtime::body_tilt::prepare_land`. Reuses the
//! retained `BodyTiltMotionAttributes`; other readonly global views bind once.
#[path = "motion_air_leg_attributes.rs"]
mod air_leg;
use super::BodyTiltMotionAttributes;
use crate::{
    actor_construction::motion_runtime_session::prepare_land::kernel::Tuning,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::{MotionLoadError, runtime::MotionUpdateError as Error},
    physics::PointGraph,
};
use std::sync::Arc;

fn attr_error(e: AttributeRuntimeError) -> Error {
    Error::Load(MotionLoadError::AttributeRuntime(e))
}
pub(crate) struct PrepareLandAttributes {
    vault: Arc<AttributeRuntimeVault>,
    inair: AttributeRuntimeInstance,
    feet: AttributeRuntimeInstance,
    animation: AttributeRuntimeInstance,
}
impl PrepareLandAttributes {
    /// Global+150 ctor828b2fe0; +bc `physics_feet.default`; +d4 animation.default.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            vault: Arc::clone(vault),
            inair: vault.instance(0x90c3_decf_0105_631e, 0x995b_b006_13b8_645a),
            feet: vault.instance(
                crate::skateboard_foot_attributes::SkateboardFootVaultAttributes::CLASS,
                0xd7ed_bd36_2d7d_2152,
            ),
            // Global.animation_default (+0xd4), constructed at 0x828b2784..0x828b279c.
            animation: vault.instance(0xbabb_3ec1_1084_9e08, 0xd7ed_bd36_2d7d_2152),
        }
    }
    pub(crate) fn check_vault(&self, vault: &Arc<AttributeRuntimeVault>) -> Result<(), Error> {
        if Arc::ptr_eq(&self.vault, vault) {
            Ok(())
        } else {
            Err(Error::ForeignOwner("preland global Vault"))
        }
    }
    pub(crate) fn scalar(
        &self,
        field: Tuning,
        body_tilt: Option<&BodyTiltMotionAttributes>,
    ) -> Result<f32, Error> {
        let (instance, key) = match field {
            Tuning::FeetNormalLimit => (&self.feet, 0xbaba_9160_6102_af9a),
            Tuning::FinalDistance => (&self.inair, 0xa766_556a_9f24_a64b),
            Tuning::DistanceVelocity => (&self.inair, 0x1747_76fa_8102_06ff),
            Tuning::LandingOnDeckDistance => (&self.inair, 0xae54_b965_f21b_dac0),
            Tuning::TiltXLimit | Tuning::TiltVelocityLimit => {
                let body_tilt =
                    body_tilt.ok_or(Error::MissingOwner("preland Global body tilt+148"))?;
                if !Arc::ptr_eq(&body_tilt.vault, &self.vault) {
                    return Err(Error::ForeignOwner("preland body tilt Vault"));
                }
                (
                    &body_tilt.instance,
                    if field == Tuning::TiltXLimit {
                        0x87c7_cfb0_4f54_f9da
                    } else {
                        0xe571_887a_9967_e796
                    },
                )
            }
        };
        Ok(f32::from_bits(
            instance
                .try_scalar_word(key, 0)
                .map_err(attr_error)?
                .unwrap_or(0),
        ))
    }
    /// Generic `FETCH825ecef0`, not the generated cached-layout graph route.
    pub(crate) fn landing_graph(&self) -> Result<PointGraph<8>, Error> {
        let data = self
            .animation
            .try_get_data(0x29a7_3f04_7bef_8865, 0)
            .map_err(attr_error)?;
        let Some(data) = data else {
            return Ok(PointGraph {
                x: [0.0; 8],
                y: [0.0; 8],
            });
        };
        if data.bytes().len() != 0x40 {
            return Err(Error::MissingOwner("preland PointGraphData8 layout"));
        }
        let word = |at: usize| {
            f32::from_be_bytes(
                data.bytes()[at..at + 4]
                    .try_into()
                    .expect("validated graph"),
            )
        };
        Ok(PointGraph {
            x: std::array::from_fn(|i| word(i * 4)),
            y: std::array::from_fn(|i| word(0x20 + i * 4)),
        })
    }
}
impl crate::input_cvar_runtime::InputCvarReferences {
    /// Ref83047fdc init82f56930 table82fd1038, late sampled825f4e60/64.
    pub(crate) fn try_offboard_air_preland_time(
        &self,
    ) -> Result<f32, crate::input_cvar_runtime::InputCvarError> {
        self.try_current_float(239)
    }
}
