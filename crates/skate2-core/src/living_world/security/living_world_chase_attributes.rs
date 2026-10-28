//! Authored global livingworld_entities_chase used by ChaseMan and Chasee.
use crate::attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault};
use std::sync::Arc;
pub(crate) const CHASE_CLASS: u64 = 0x1477_b0d5_343d_29fd;
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChaseSettings {
    pub(crate) formation_min_radius: f32,
    pub(crate) formation_max_radius: f32,
    pub(crate) formation_velocity_scalar: f32,
    pub(crate) auto_switch_default: f32,
    pub(crate) auto_switch_improvement: f32,
    pub(crate) intercept_time_improvement: f32,
    pub(crate) capture_suppression_time: f32,
    pub(crate) max_chasers: usize,
}
impl ChaseSettings {
    /// globalKey initialized by StringToKey("global")82f64d78/82f9d030.
    pub(crate) fn load(vault: &Arc<AttributeRuntimeVault>) -> Result<Self, String> {
        let instance = vault.instance(CHASE_CLASS, skate2_formats::attrib::string_hash64("global"));
        if instance.is_null() {
            return Err("ChaseMan global chase collection absent (native assertion)".into());
        }
        let scalar = |key| -> Result<f32, String> {
            Ok(f32::from_bits(
                instance
                    .try_scalar_word(key, 0)
                    .map_err(|e| e.to_string())?
                    .unwrap_or(0),
            ))
        };
        let suppression = instance
            .try_cached_layout_field(4, 4)
            .map_err(|e| e.to_string())?
            .map_or(0.0, |data| {
                f32::from_be_bytes(data.bytes().try_into().expect("four-byte chase field"))
            });
        Ok(Self {
            formation_min_radius: scalar(0x09c7_ba07_59e1_3dde)?,
            formation_max_radius: scalar(0x4cf3_9d0d_047c_8cc0)?,
            formation_velocity_scalar: scalar(0xfdcd_97d0_5887_5e31)?,
            auto_switch_default: scalar(0xb21f_d5a3_4d5b_f776)?,
            auto_switch_improvement: scalar(0xf570_f5ed_bee3_b9d9)?,
            intercept_time_improvement: scalar(0x1fcd_3fdf_4c3b_5f06)?,
            capture_suppression_time: suppression,
            max_chasers: max_chasers(&instance)?,
        })
    }
    pub(crate) fn chasee(self) -> super::ChaseeState {
        super::ChaseeState::new(
            self.formation_min_radius,
            self.formation_max_radius,
            self.formation_velocity_scalar,
        )
    }
}
/// GetMaxNumChasers82de1bd8: signed byte promoted to unsigned and capped5;
/// absent attribute DefaultDataArea supplies0. A missing collection asserts.
pub(crate) fn max_chasers(instance: &AttributeRuntimeInstance) -> Result<usize, String> {
    let data = instance
        .try_get_data(0xcea5_d982_fd05_be0b, 0)
        .map_err(|e| e.to_string())?;
    let value = if let Some(data) = data {
        let [byte] = data.bytes() else {
            return Err("chase MaxNumChasers is not one byte".into());
        };
        i32::from(*byte as i8) as u32
    } else {
        0
    };
    Ok(value.min(5) as usize)
}
