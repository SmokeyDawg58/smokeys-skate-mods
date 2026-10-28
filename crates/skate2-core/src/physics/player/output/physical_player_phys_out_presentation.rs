//! Direct presentation getters from the same retained `PhysOut` components.
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
use std::convert::Infallible;
impl PhysicalPlayerPhysOut {
    /// `GetBoardSpeed825f0420`: direct Motion surface-speed field, no write latch.
    pub(crate) fn try_presentation_board_speed(
        &self,
    ) -> Result<f32, ConcretePhysOutError<Infallible>> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        self.motion_scalars
            [crate::physical_player_phys_out::PhysOutMotionScalar::SurfaceSpeed as usize]
            .map(|v| v.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    /// `GetCharacterDamageData825f05b8`: seven existing packed signed-level/byte pairs.
    pub(crate) fn try_presentation_character_damage(
        &self,
    ) -> Result<[u8; 14], ConcretePhysOutError<Infallible>> {
        self.require_complete_component(PhysOutComponent::Scoring2)?;
        let areas = &self
            .concrete
            .scoring
            .as_ref()
            .expect("checked component")
            .character_damage_data
            .areas;
        Ok(std::array::from_fn(|i| {
            if i & 1 == 0 {
                areas[i / 2].damage_level.to_ne_bytes()[0]
            } else {
                areas[i / 2].intensity
            }
        }))
    }
}
