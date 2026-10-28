//! Actual `HandPlant` then `FootPlant` callbacks82e04f60/4f6c, before Controller.
use super::update5_managers::{GrindManagerFillPolicy, Update5BeforeHandPlant};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_footplant::{FootPlantError, FootPlantFields},
    physical_player_handplant::HandPlantManager,
    physical_player_phys_out::construct::plant_fill::{
        FootPlantFillSource, HandPlantFillSource, PlantFillError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum PlantRuntimeError {
    Missing(&'static str),
    Poisoned,
    Foot(FootPlantError),
}
impl std::fmt::Display for PlantRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "plant output missing {field}"),
            Self::Poisoned => f.write_str("plant output retained owner poisoned"),
            Self::Foot(error) => write!(f, "FootPlant owner: {error:?}"),
        }
    }
}
struct HandSource<'a>(&'a Arc<PhysicalPlayerAssemblyOwner>);
impl HandSource<'_> {
    fn read<T>(&self, read: impl FnOnce(&HandPlantManager) -> T) -> Result<T, PlantRuntimeError> {
        let manager = self
            .0
            .handplant()
            .lock()
            .map_err(|_| PlantRuntimeError::Poisoned)?;
        Ok(read(
            manager
                .as_ref()
                .ok_or(PlantRuntimeError::Missing("HandPlantManager"))?,
        ))
    }
}
impl HandPlantFillSource for HandSource<'_> {
    type Error = PlantRuntimeError;
    fn position(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read(|m| SkeletonRootVector::known(m.animation.position))
    }
    fn apex_time(&mut self) -> Result<f32, Self::Error> {
        self.read(|m| m.animation.estimated_time_to_apex)
    }
    fn animation_flags(&mut self) -> Result<u32, Self::Error> {
        self.read(|m| m.animation.flags)
    }
    fn ik_flags(&mut self) -> Result<u32, Self::Error> {
        self.read(|m| m.ik.flags)
    }
    fn surface(&mut self) -> Result<u32, Self::Error> {
        self.read(|m| m.surface_id)
    }
}
struct FootSource<'a>(&'a FootPlantFields);
impl FootPlantFillSource for FootSource<'_> {
    type Error = PlantRuntimeError;
    fn results_valid(&mut self) -> Result<bool, Self::Error> {
        Ok(self.0.results_valid)
    }
    fn time_to_land(&mut self) -> Result<f32, Self::Error> {
        Ok(self.0.time_to_land)
    }
    fn total_time(&mut self) -> Result<f32, Self::Error> {
        Ok(self.0.total_time)
    }
    fn landing(&mut self) -> Result<bool, Self::Error> {
        Ok(self.0.landing_next_frame)
    }
    fn planting(&mut self) -> Result<bool, Self::Error> {
        Ok(self.0.foot_planting)
    }
    fn foot_part(&mut self) -> Result<i32, Self::Error> {
        Ok(self.0.foot_part)
    }
    fn surface(&mut self) -> Result<u32, Self::Error> {
        self.0
            .surface_id
            .ok_or(PlantRuntimeError::Missing("FootPlant surface1d0"))
    }
}
#[derive(Debug)]
pub(crate) enum Update5PlantError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    Fill(PlantFillError<PlantRuntimeError>),
}
impl std::fmt::Display for Update5PlantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Update5 plants require actual skater pool"),
            Self::Pair(error) => error.fmt(f),
            Self::ForeignOwner => {
                f.write_str("Update5 plants continuation belongs to another pair")
            }
            Self::Fill(error) => error.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeControllerFill {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) grind_policy: GrindManagerFillPolicy,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_plants(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeHandPlant,
    ) -> Result<Update5BeforeControllerFill, Update5PlantError> {
        use Update5PlantError as E;
        let owner = before.owner;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        let source_error = |error| E::Fill(PlantFillError::Source(error));
        {
            let mut output = owner
                .phys_out()
                .lock()
                .map_err(|_| source_error(PlantRuntimeError::Poisoned))?;
            output
                .try_fill_handplant(&mut HandSource(&owner))
                .map_err(E::Fill)?;
        }
        {
            let foot = owner
                .footplant()
                .lock()
                .map_err(|_| source_error(PlantRuntimeError::Poisoned))?;
            foot.with_fill_fields(&owner, skeleton, |fields| {
                let mut output = owner
                    .phys_out()
                    .lock()
                    .map_err(|_| source_error(PlantRuntimeError::Poisoned))?;
                output
                    .try_fill_footplant(&mut FootSource(fields))
                    .map_err(E::Fill)
            })
            .map_err(|error| source_error(PlantRuntimeError::Foot(error)))??;
        }
        Ok(Update5BeforeControllerFill {
            owner,
            grind_policy: before.grind_policy,
        })
    }
}
