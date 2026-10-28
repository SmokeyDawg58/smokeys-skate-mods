//! Same retained global in-air attribute view, child of `prepare_land`.
use super::{PrepareLandAttributes, attr_error};
use crate::{
    actor_construction::motion_runtime_session::air_leg::kernel::{Cvar, Tuning},
    generated_motion_load::runtime::MotionUpdateError as E,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physics::PointGraph,
};
impl PrepareLandAttributes {
    pub(crate) fn air_leg_scalar(&self, field: Tuning) -> Result<f32, E> {
        let key = match field {
            Tuning::UpSpeed => 0xfa94_c1bf_4091_04fa,
            Tuning::DownSpeed => 0xfff0_1289_f3da_3ef6,
            Tuning::Minimum => 0xde42_e61b_3197_ee74,
            Tuning::PrelandSpeed => 0x1747_76fa_8102_06ff,
            Tuning::FinalDistance => 0xa766_556a_9f24_a64b,
            Tuning::OffboardMultiplier => 0x98be_256a_aafe_fcd1,
        };
        Ok(f32::from_bits(
            self.inair
                .try_scalar_word(key, 0)
                .map_err(attr_error)?
                .unwrap_or(0),
        ))
    }
    pub(crate) fn air_leg_extend(&self, time: f32) -> Result<f32, E> {
        // Native generated layout+530 graph: x+10/y+20. Null generated
        // layout uses its existing DefaultDataArea5c0, hence zero graph words.
        let data = self
            .inair
            .try_cached_layout_field(0x540, 0x20)
            .map_err(attr_error)?;
        let word = |at: usize| {
            data.as_ref().map_or(0.0, |d| {
                f32::from_be_bytes(
                    d.bytes()[at..at + 4]
                        .try_into()
                        .expect("checked graph field"),
                )
            })
        };
        Ok(PointGraph::<4> {
            x: std::array::from_fn(|i| word(i * 4)),
            y: std::array::from_fn(|i| word(16 + i * 4)),
        }
        .evaluate(time))
    }
}
impl InputCvarReferences {
    pub(crate) fn try_air_leg_boolean(&self, field: Cvar) -> Result<bool, InputCvarError> {
        self.try_current_boolean(match field {
            Cvar::Debug => 308,
            Cvar::Up => 309,
            Cvar::Down => 310,
            Cvar::Preland => 311,
            Cvar::Custom => 313,
        })
    }
    pub(crate) fn try_air_leg_custom_earlier(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(312)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn live_air_refs_require_link_and_keep_authored_defaults_and_mutations() {
        let refs = InputCvarReferences::new_static_initialized();
        assert!(refs.try_air_leg_boolean(Cvar::Up).is_err());
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert!(!refs.try_air_leg_boolean(Cvar::Debug).unwrap());
        for field in [Cvar::Up, Cvar::Down, Cvar::Preland, Cvar::Custom] {
            assert!(refs.try_air_leg_boolean(field).unwrap());
        }
        assert_eq!(
            refs.try_air_leg_custom_earlier().unwrap().to_bits(),
            0x3d75_c28f
        );
        manager
            .try_deserialize_startup(Some(
                b"[Config]\r\nkLegAdjustUp=0\r\nkLegAdjustCustomPreLandHack=0.25\r\n",
            ))
            .unwrap();
        assert!(!refs.try_air_leg_boolean(Cvar::Up).unwrap());
        assert_eq!(
            refs.try_air_leg_custom_earlier().unwrap().to_bits(),
            0.25f32.to_bits()
        );
    }
}
