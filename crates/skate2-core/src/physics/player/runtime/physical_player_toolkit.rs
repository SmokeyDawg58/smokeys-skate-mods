//! Update1 `PrepareToolkitInput82e036cc` on the actual paired processed owner.
use crate::{
    dynamic_assembly_session::{
        part_transform::DynamicAssemblyPartTransformError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonJumpPairError,
        },
    },
    input_cvar_runtime::toolkit::{ToolkitCvarError, ToolkitCvars},
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::physical_board_forces::PhysicalPlayerBoardForceError,
    skateboard_body_mass::{SkateboardBodyMassError, try_skateboard_body_mass},
    skeleton_processed_contact::toolkit::{
        SkeletonProcessedToolkitError, SkeletonProcessedToolkitJumpError,
        runtime::{ToolkitPhysicalBoardRead, ToolkitPhysicalReadError, ToolkitRetainedInputError},
    },
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::{convert::Infallible, fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum ToolkitBoardReadError {
    Pool(SimulationSkaterPoolError),
    Mass(SkateboardBodyMassError),
    Transform(DynamicAssemblyPartTransformError),
}
impl fmt::Display for ToolkitBoardReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "toolkit board owner: {e}"),
            Self::Mass(e) => e.fmt(f),
            Self::Transform(e) => write!(f, "toolkit Part6 transform: {e}"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum PhysicalPlayerToolkitError<N> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Poisoned,
    Prefix(ToolkitPhysicalReadError<ToolkitBoardReadError>),
    Directions(SkeletonProcessedToolkitError<ToolkitRetainedInputError, N>),
    Force(PhysicalPlayerBoardForceError),
    Jump(
        SkeletonProcessedToolkitJumpError<
            ToolkitRetainedInputError,
            Infallible,
            ToolkitCvarError,
            N,
        >,
    ),
}
impl<N: fmt::Debug> fmt::Display for PhysicalPlayerToolkitError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPool => f.write_str("toolkit requires the actual skater pool"),
            Self::Pair(e) => write!(f, "toolkit physical pair: {e:?}"),
            Self::Poisoned => f.write_str("toolkit processed owner lock poisoned"),
            Self::Prefix(e) => e.fmt(f),
            Self::Directions(e) => e.fmt(f),
            Self::Force(e) => e.fmt(f),
            Self::Jump(e) => e.fmt(f),
        }
    }
}

struct Board<'a> {
    world: &'a PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
}
impl ToolkitPhysicalBoardRead for Board<'_> {
    type Error = ToolkitBoardReadError;
    fn mass(&mut self) -> Result<f32, Self::Error> {
        try_skateboard_body_mass(
            self.world.board(self.board).map_err(Self::Error::Pool)?,
            self.world.backend(),
        )
        .map_err(Self::Error::Mass)
    }
    fn deck_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.world
            .board(self.board)
            .map_err(Self::Error::Pool)?
            .try_live_part_transform_typed(self.world.backend(), 6)
            .map_err(Self::Error::Transform)
    }
}

impl PhysicalAssemblyWorld {
    /// Runs full toolkit before the separate OLD skeleton-line result Fill.
    /// Does not fetch new `PhysOut` motion or mark Skeleton.ProcessData complete.
    pub(crate) fn try_prepare_player_toolkit<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerToolkitError<N::Error>> {
        use PhysicalPlayerToolkitError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        // Drop the Reckoning lock before holding processed. The physical board
        // reads and later force-runtime callback never reacquire processed.
        let mut destination = processed.lock().map_err(|_| E::Poisoned)?;
        destination
            .try_sample_live_toolkit_prefix(&mut Board { world: self, board })
            .map_err(E::Prefix)?;
        destination
            .try_continue_live_toolkit_directions(numeric)
            .map_err(E::Directions)?;
        let mut cvars = ToolkitCvars::new(&owner.inputs().cvars, self.debug_overlay.clone());
        self.try_with_board_force_runtime(board, |runtime| {
            destination.try_continue_live_toolkit_jump_normal(runtime, &mut cvars, numeric)
        })
        .map_err(E::Force)?
        .map_err(E::Jump)
    }
}
