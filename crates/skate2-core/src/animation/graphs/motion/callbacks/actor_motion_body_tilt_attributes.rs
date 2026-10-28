//! Proposed child `input_cvar_runtime::body_tilt`, so late reference reads use
//! the actual private shared target allocations. No eager cvar capture.
#[path = "actor_motion_prepare_land_attributes.rs"]
pub(crate) mod prepare_land;
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Value};
use crate::{
    actor_construction::motion_runtime_session::setting_body_tilt::BodyTiltTuning,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::{MotionLoadError, runtime::MotionUpdateError},
    physics::PointGraph,
};
use std::sync::Arc;

/// Actual GlobalAttributeCollection+148 generated `anim_motion` wrapper.
pub(crate) struct BodyTiltMotionAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl BodyTiltMotionAttributes {
    /// Alloc828b2f2c, selected collection constructor2f50, member store2f5c.
    /// Generated ctor828b1590 binds class and substitutes `DefaultDataArea5c0`.
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x90c3_decf_0105_631e, 0xddcd_c2a8_8e43_a628);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self {
            vault: Arc::clone(vault),
            instance,
        })
    }
}

/// Construct a short-lived view for each callback. `physical_vault` must come
/// from the same actual `IPhysical` owner's inputs, not another selected Vault.
/// Construction performs no reads/checks: even absent services only fail at
/// their first source use after the kernel's instance transitions/output reads.
pub(crate) struct BodyTiltRuntime<'a> {
    pub(crate) attributes: Option<&'a BodyTiltMotionAttributes>,
    pub(crate) cvars: Option<&'a InputCvarReferences>,
    pub(crate) physical_vault: &'a Arc<AttributeRuntimeVault>,
}
fn attribute_error(error: AttributeRuntimeError) -> MotionUpdateError {
    MotionUpdateError::Load(MotionLoadError::AttributeRuntime(error))
}
impl BodyTiltRuntime<'_> {
    fn attributes(&self) -> Result<&BodyTiltMotionAttributes, MotionUpdateError> {
        let owner = self.attributes.ok_or(MotionUpdateError::MissingOwner(
            "GlobalAttributeCollection anim_motion_body_tilt+148",
        ))?;
        if !Arc::ptr_eq(&owner.vault, self.physical_vault) {
            return Err(MotionUpdateError::ForeignOwner("global body tilt Vault"));
        }
        Ok(owner)
    }
    fn scalar(&self, key: u64) -> Result<f32, MotionUpdateError> {
        let word = self
            .attributes()?
            .instance
            .try_scalar_word(key, 0)
            .map_err(attribute_error)?
            .unwrap_or(0); // accessor DefaultDataArea4
        Ok(f32::from_bits(word))
    }
    fn cvars(&self) -> Result<&InputCvarReferences, MotionUpdateError> {
        self.cvars
            .ok_or(MotionUpdateError::MissingOwner("Motion body tilt cvars"))
    }
}
impl BodyTiltTuning for BodyTiltRuntime<'_> {
    ///82c9e604..e620: cached layout+560, x+10/y+20, four points.
    fn evaluate_spin_graph(&mut self, absolute_speed: f32) -> Result<f32, MotionUpdateError> {
        let layout = self
            .attributes()?
            .instance
            .try_cached_layout(0x5c0)
            .map_err(attribute_error)?;
        let word = |offset: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[offset..offset + 4]
                        .try_into()
                        .expect("validated anim_motion layout"),
                )
            })
        };
        Ok(PointGraph::<4> {
            x: std::array::from_fn(|i| word(0x570 + i * 4)),
            y: std::array::from_fn(|i| word(0x580 + i * 4)),
        }
        .evaluate(absolute_speed))
    }
    /// Dynamic node lookup at82c8abf8/82c8ac88, after filtered-state read.
    fn clamp_acceleration(&mut self, air: bool) -> Result<f32, MotionUpdateError> {
        self.scalar(if air {
            0xe7c8_77e2_20d6_fbc0
        } else {
            0x68bf_98bc_9357_99b9
        })
    }
    /// Separate later node lookup at82c8ac40/82c8acd0.
    fn clamp_velocity(&mut self, air: bool) -> Result<f32, MotionUpdateError> {
        self.scalar(if air {
            0x5285_5de4_2244_601b
        } else {
            0xd2b5_0fd1_2925_415d
        })
    }
    fn use_tilt(&mut self) -> Result<bool, MotionUpdateError> {
        self.cvars()?
            .try_current_boolean(227)
            .map_err(MotionUpdateError::Cvar)
    }
    fn set_tilt(&mut self) -> Result<bool, MotionUpdateError> {
        self.cvars()?
            .try_current_boolean(228)
            .map_err(MotionUpdateError::Cvar)
    }
    fn override_x(&mut self) -> Result<f32, MotionUpdateError> {
        let references = self.cvars()?;
        let targets = references
            .targets
            .lock()
            .map_err(|_| MotionUpdateError::Cvar(InputCvarError::Lock))?;
        let target =
            targets[229]
                .as_ref()
                .ok_or(MotionUpdateError::Cvar(InputCvarError::Unlinked(
                    DEFINITIONS[229].label,
                )))?;
        match *target
            .lock()
            .map_err(|_| MotionUpdateError::Cvar(InputCvarError::Lock))?
        {
            Value::Float(value) => Ok(value),
            _ => Err(MotionUpdateError::Cvar(InputCvarError::WrongType)),
        }
    }
}
