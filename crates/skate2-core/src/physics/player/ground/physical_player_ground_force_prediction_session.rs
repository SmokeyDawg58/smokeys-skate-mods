//! Real Ground1024 queue-to-hook producer; stops before `RequestPushExit1028`.
use super::{
    ground_force_prediction::{
        GroundForcePredictionError, GroundForcePredictionSource, try_predict_force_offset,
    },
    ground_hang_pinning_session::GroundSkateboardComplete,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
use std::sync::{Arc, Mutex};
pub(crate) struct GroundBeforePushExit(pub(crate) GroundSkateboardComplete);
#[derive(Debug)]
pub(crate) enum ForcePredictionRuntimeError {
    Missing(&'static str),
    Poisoned,
    Foreign,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Hooks(SimulationSkaterPoolError),
}
impl std::fmt::Display for ForcePredictionRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "force prediction missing {s}"),
            Self::Poisoned => f.write_str("force prediction owner lock poisoned"),
            Self::Foreign => f.write_str("force prediction foreign pair or state"),
            Self::Pair(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
            Self::Hooks(e) => e.fmt(f),
        }
    }
}
struct Runtime<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    processed: Arc<Mutex<SkeletonProcessedContact>>,
}
impl GroundForcePredictionSource for Runtime<'_> {
    type Error = ForcePredictionRuntimeError;
    fn mass(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .physical_mass()
            .ok_or(Self::Error::Missing("mass970"))
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(Self::Error::Missing("DT93c"))
    }
    fn combined_forces(&mut self) -> Result<Vector, Self::Error> {
        self.world
            .skaters
            .as_mut()
            .ok_or(Self::Error::Missing("skater pool"))?
            .try_with_ground_force_board(&self.world.backend, self.board, |_, runtime, _| {
                runtime.queue.combined_forces()
            })
            .map_err(Self::Error::Board)
    }
    fn ground_normal(&mut self) -> Result<Vector, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .ground_normal()
            .map(|v| v.value)
            .ok_or(Self::Error::Missing("ground normal1d0"))
    }
    fn add_offset_to_hooks(&mut self, offset: Vector) -> Result<(), Self::Error> {
        self.world
            .skaters
            .as_mut()
            .ok_or(Self::Error::Missing("skater pool"))?
            .try_add_skeleton_hook_offset(
                &mut self.world.backend,
                &mut self.world.hooks,
                self.skeleton,
                offset,
            )
            .map_err(Self::Error::Hooks)
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_ground_force_prediction<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: GroundSkateboardComplete,
        numeric: &mut N,
    ) -> Result<
        GroundBeforePushExit,
        GroundForcePredictionError<ForcePredictionRuntimeError, N::Error>,
    > {
        use ForcePredictionRuntimeError as E;
        let owner = Arc::clone(&before.owner);
        let same = self
            .skaters
            .as_mut()
            .ok_or(GroundForcePredictionError::Source(E::Missing(
                "skater pool",
            )))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(|e| GroundForcePredictionError::Source(E::Pair(e)))?;
        if !same {
            return Err(GroundForcePredictionError::Source(E::Foreign));
        }
        let state = owner
            .physics_state()
            .lock()
            .map_err(|_| GroundForcePredictionError::Source(E::Poisoned))?;
        if state.current != CurrentPhysicalState::Ground {
            return Err(GroundForcePredictionError::Source(E::Foreign));
        }
        let ground = state
            .ground
            .as_ref()
            .ok_or(GroundForcePredictionError::Source(E::Missing("Ground")))?;
        if !ground.bindings.board.same_as(board) || !ground.bindings.skeleton.same_as(skeleton) {
            return Err(GroundForcePredictionError::Source(E::Foreign));
        }
        let exiting = ground.fields.exiting_wallride;
        drop(state);
        if exiting {
            return Ok(GroundBeforePushExit(before));
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| GroundForcePredictionError::Source(E::Poisoned))?
                .processed(),
        );
        try_predict_force_offset(
            exiting,
            &mut Runtime {
                world: self,
                board,
                skeleton,
                processed,
            },
            numeric,
        )?;
        Ok(GroundBeforePushExit(before))
    }
}
