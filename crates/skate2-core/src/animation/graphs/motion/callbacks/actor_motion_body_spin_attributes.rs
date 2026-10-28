//! Shared Global+130 `anim_motion_body_spin`, child `input_cvar_runtime::body_spin_motion`.
use super::InputCvarReferences;
use crate::{
    actor_construction::motion_runtime_session::body_spin::{BodySpinTuning, kernel::SpinTuning},
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::{MotionLoadError, runtime::MotionUpdateError},
    physics::PointGraph,
};
use std::sync::Arc;
pub(crate) struct BodySpinMotionAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl BodySpinMotionAttributes {
    /// Original allocation828b2d7c, generated ctor2da0, publication+130 at2dac.
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x90c3_decf_0105_631e, 0xdcba_de97_ca66_5643);
        instance.try_cached_layout(0x5c0)?; // generated ctor supplies zero fallback5c0
        Ok(Self {
            vault: Arc::clone(vault),
            instance,
        })
    }
}
pub(crate) struct BodySpinRuntime<'a> {
    pub(crate) attributes: Option<&'a BodySpinMotionAttributes>,
    pub(crate) cvars: Option<&'a InputCvarReferences>,
    pub(crate) physical_vault: &'a Arc<AttributeRuntimeVault>,
}
fn attribute_error(error: AttributeRuntimeError) -> MotionUpdateError {
    MotionUpdateError::Load(MotionLoadError::AttributeRuntime(error))
}
impl BodySpinRuntime<'_> {
    fn attributes(&self) -> Result<&BodySpinMotionAttributes, MotionUpdateError> {
        let attributes = self.attributes.ok_or(MotionUpdateError::MissingOwner(
            "Global.anim_motion_body_spin+130",
        ))?;
        if !Arc::ptr_eq(&attributes.vault, self.physical_vault) {
            return Err(MotionUpdateError::ForeignOwner("BodySpin global Vault"));
        }
        Ok(attributes)
    }
}
impl BodySpinTuning for BodySpinRuntime<'_> {
    fn enable_landing_on_board(&mut self) -> Result<bool, MotionUpdateError> {
        self.cvars
            .ok_or(MotionUpdateError::MissingOwner("BodySpin cvars"))?
            .try_current_boolean(231)
            .map_err(MotionUpdateError::Cvar)
    }
    fn input_graph(&mut self, magnitude: f32) -> Result<f32, MotionUpdateError> {
        let layout = self
            .attributes()?
            .instance
            .try_cached_layout(0x5c0)
            .map_err(attribute_error)?;
        let word = |offset| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[offset..offset + 4]
                        .try_into()
                        .expect("anim_motion graph field"),
                )
            })
        };
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| word(0x360 + i * 4)),
            y: std::array::from_fn(|i| word(0x380 + i * 4)),
        }
        .evaluate(magnitude))
    }
    fn scalar(&mut self, field: SpinTuning) -> Result<f32, MotionUpdateError> {
        let key = match field {
            SpinTuning::MaximumStep => 0xd2cf_959d_0679_2567,
            SpinTuning::MaximumDeltaChange => 0xc215_e767_43ae_5d4a,
            SpinTuning::Blend => 0x803b_5ae1_9b41_b6c8,
            SpinTuning::AlternateBlend => 0x4ad6_62ad_587d_f578,
        };
        Ok(f32::from_bits(
            self.attributes()?
                .instance
                .try_scalar_word(key, 0)
                .map_err(attribute_error)?
                .unwrap_or(0),
        ))
    }
}
