//! Selected `CalculateGroundPos` on the actual board and retained Reckoning.
//! Explicit current-constructor and completed-phase read routes; no category publication.
use crate::{
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        skater_pool::{SimulationSkaterAssemblyHandle, SimulationSkaterPoolError},
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    simulation_assembly_backend::constraint_session::physical_island::SimulationPhysicalBodyError,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_calculate_ground_position::{
        SkateboardGroundPositionSource, try_calculate_ground_position,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{fmt, sync::Arc};
#[derive(Debug)]
pub(crate) enum PhysicalPlayerGroundPositionError {
    Pool(SimulationSkaterPoolError),
    Owner(SimulationPhysicalBodyError),
    Part(DynamicAssemblyPartContactSpyError),
    Body(SimulationRigidBodyPoolError),
    PoisonedReckoning,
    StaleReckoning,
    UnconstructedReckoning,
    InvalidColumn,
}
impl fmt::Display for PhysicalPlayerGroundPositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "ground position board: {e}"),
            Self::Owner(e) => write!(f, "ground position owner: {e}"),
            Self::Part(e) => write!(f, "ground position Part: {e}"),
            Self::Body(e) => write!(f, "ground position body: {e}"),
            Self::PoisonedReckoning => f.write_str("ground position Reckoning owner poisoned"),
            Self::StaleReckoning => {
                f.write_str("ground position requires the same completed Reckoning phase")
            }
            Self::UnconstructedReckoning => {
                f.write_str("ground position Reckoning has not been constructed")
            }
            Self::InvalidColumn => f.write_str("ground position basis column outside R/U/A"),
        }
    }
}
struct Source<'a> {
    world: &'a PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    phase: Option<&'a PhysicalPlayerReckoningPhase>,
}
impl SkateboardGroundPositionSource for Source<'_> {
    type Ground = Arc<PhysicalPlayerAssemblyOwner>;
    type Body = ();
    type Error = PhysicalPlayerGroundPositionError;
    fn capture_ground(&mut self) -> Result<Self::Ground, Self::Error> {
        self.world
            .physical_body_owner_for_contacts(self.board)
            .map(Arc::clone)
            .map_err(Self::Error::Owner)
    }
    fn ground_column(
        &mut self,
        ground: &Self::Ground,
        column: usize,
    ) -> Result<SkeletonRootVector, Self::Error> {
        if column >= 3 {
            return Err(Self::Error::InvalidColumn);
        }
        let owner = ground
            .reckoning()
            .lock()
            .map_err(|_| Self::Error::PoisonedReckoning)?;
        let reckoning = match self.phase {
            Some(phase) => owner
                .try_completed(phase)
                .map_err(|_| Self::Error::StaleReckoning)?,
            None => owner
                .reckoning()
                .ok_or(Self::Error::UnconstructedReckoning)?,
        };
        let matrix = reckoning.ground_transform();
        Ok(matrix.columns()[column])
    }
    fn capture_body(&mut self) -> Result<(), Self::Error> {
        self.world.board(self.board).map_err(Self::Error::Pool)?;
        Ok(())
    }
    fn body_position(&mut self, (): &(), part: usize) -> Result<[f32; 3], Self::Error> {
        let observation = self
            .world
            .board(self.board)
            .map_err(Self::Error::Pool)?
            .try_part_contact_spy_observation(&self.world.backend, part)
            .map_err(Self::Error::Part)?;
        self.world
            .backend
            .collision_body_pool()
            .try_contact_spy_center_of_mass(observation)
            .map_err(Self::Error::Body)
    }
}
impl PhysicalAssemblyWorld {
    /// SAME board owner and phase; all numeric reads occur at leaf callbacks.
    /// Host kind validation precedes source entry, with no contact-frame gate.
    pub(crate) fn try_calculate_board_ground_position(
        &self,
        board: &SimulationSkaterAssemblyHandle,
        phase: &PhysicalPlayerReckoningPhase,
        result: &mut SkeletonRootVector,
    ) -> Result<(), PhysicalPlayerGroundPositionError> {
        self.board(board)
            .map_err(PhysicalPlayerGroundPositionError::Pool)?;
        try_calculate_ground_position(
            &mut Source {
                world: self,
                board,
                phase: Some(phase),
            },
            result,
        )
    }
    /// Update1 call82e03560 reads the current constructed Reckoning transform;
    /// no completed update is required by native `CalculateGroundPos82b2f1b0`.
    pub(crate) fn try_calculate_current_board_ground_position(
        &self,
        board: &SimulationSkaterAssemblyHandle,
        result: &mut SkeletonRootVector,
    ) -> Result<(), PhysicalPlayerGroundPositionError> {
        self.board(board)
            .map_err(PhysicalPlayerGroundPositionError::Pool)?;
        try_calculate_ground_position(
            &mut Source {
                world: self,
                board,
                phase: None,
            },
            result,
        )
    }
}
