//! Complete HandPlant.Fill82db8580 and FootPlant.Fill82db6890 output kernels.
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut, SkeletonRootVector};
use super::prior_input::PriorInputReadError;

pub(crate) trait HandPlantFillSource {
    type Error;
    fn position(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn apex_time(&mut self) -> Result<f32, Self::Error>;
    fn animation_flags(&mut self) -> Result<u32, Self::Error>;
    fn ik_flags(&mut self) -> Result<u32, Self::Error>;
    fn surface(&mut self) -> Result<u32, Self::Error>;
}
pub(crate) trait FootPlantFillSource {
    type Error;
    fn results_valid(&mut self) -> Result<bool, Self::Error>;
    fn time_to_land(&mut self) -> Result<f32, Self::Error>;
    fn total_time(&mut self) -> Result<f32, Self::Error>;
    fn landing(&mut self) -> Result<bool, Self::Error>;
    fn planting(&mut self) -> Result<bool, Self::Error>;
    fn foot_part(&mut self) -> Result<i32, Self::Error>;
    fn surface(&mut self) -> Result<u32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum PlantFillError<E> {
    Output(PriorInputReadError),
    Source(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for PlantFillError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Output(error) => write!(f, "plant Fill output: {error:?}"),
            Self::Source(error) => write!(f, "plant Fill source: {error:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    /// PhysState_HandPlant.FillPhysOut82d893c8; distinct from manager Fill.
    pub(crate) fn try_fill_handplant_state(
        &mut self,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grind_manager::view::error;
        self.check_current_guard().map_err(error)?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(error)?;
        self.concrete
            .state
            .as_mut()
            .expect("checked")
            .is_hand_planting = true;
        Ok(())
    }

    fn plant_guard(&self) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_fill_handplant<S: HandPlantFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), PlantFillError<S::Error>> {
        use PlantFillError as E;
        self.plant_guard().map_err(E::Output)?;
        let position = source.position().map_err(E::Source)?;
        //85cc captures old destination flags before the vector store85d0.
        let old_flags = self
            .concrete
            .air
            .as_ref()
            .unwrap()
            .anim_hand_plant_info_struct
            .flags;
        self.concrete
            .air
            .as_mut()
            .unwrap()
            .anim_hand_plant_info_struct
            .position = position;
        self.concrete
            .air
            .as_mut()
            .unwrap()
            .anim_hand_plant_info_struct
            .estimated_time_to_apex = source.apex_time().map_err(E::Source)?;
        let mut flags = old_flags;
        for mask in [0x8000_0000, 0x4000_0000, 0x2000_0000, 0x1000_0000] {
            flags = (flags & !mask) | (source.animation_flags().map_err(E::Source)? & mask);
            self.concrete
                .air
                .as_mut()
                .unwrap()
                .anim_hand_plant_info_struct
                .flags = flags;
        }
        let found = source.animation_flags().map_err(E::Source)? >> 31 != 0;
        self.plant_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().found_hand_plant_pos = found;
        let grabbing = source.ik_flags().map_err(E::Source)? >> 31 != 0;
        self.plant_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().hand_plant_grabbing = grabbing;
        self.plant_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().hand_plant_surface_id =
            source.surface().map_err(E::Source)?;
        Ok(())
    }
    pub(crate) fn try_fill_footplant<S: FootPlantFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), PlantFillError<S::Error>> {
        use PlantFillError as E;
        if !source.results_valid().map_err(E::Source)? {
            return Ok(());
        }
        self.plant_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().time_to_land_foot_plant =
            source.time_to_land().map_err(E::Source)?;
        self.plant_guard().map_err(E::Output)?;
        self.concrete
            .air
            .as_mut()
            .unwrap()
            .foot_plant_total_absorb_time = source.total_time().map_err(E::Source)?;
        self.plant_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().is_landing_foot_plant =
            source.landing().map_err(E::Source)?;
        let planting =
            source.landing().map_err(E::Source)? || source.planting().map_err(E::Source)?;
        self.plant_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().is_foot_planting = planting;
        self.plant_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().foot_plant_surface_id =
            source.surface().map_err(E::Source)?;
        let left =
            source.planting().map_err(E::Source)? && source.foot_part().map_err(E::Source)? == 15;
        self.plant_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().is_left_foot_planting = left;
        let right =
            source.planting().map_err(E::Source)? && source.foot_part().map_err(E::Source)? == 19;
        self.plant_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().is_right_foot_planting = right;
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_plant_fill_tests.rs"]
mod tests;
