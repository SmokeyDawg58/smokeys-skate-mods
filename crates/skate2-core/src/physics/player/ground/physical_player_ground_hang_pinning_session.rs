//! Actual retained-owner continuation after queued Ground forces, before a998.
use super::{
    ground_hang_pinning::{
        GroundHangPinSource, HangPinError, try_consider_pinning, try_hang_and_halfpipe,
    },
    ground_queue_torque_prefix::GroundBeforeManageHangUps,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
use std::sync::{Arc, Mutex};
/// Both native UpdateSkateboard returns join before Ground.Update prediction.
/// Scratch force/manual values have no consumer after this native return.
pub(crate) struct GroundSkateboardComplete {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
}
#[derive(Debug)]
pub(crate) enum HangPinRuntimeError {
    Missing(&'static str),
    Poisoned,
    Foreign,
    Pair(SimulationSkeletonJumpPairError),
    Cvar(InputCvarError),
    Velocity(super::physical_board_forces::PhysicalPlayerBoardForceError),
}
impl std::fmt::Display for HangPinRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "Ground hang/pin missing {s}"),
            Self::Poisoned => f.write_str("Ground hang/pin owner lock poisoned"),
            Self::Foreign => f.write_str("Ground hang/pin foreign pair or state"),
            Self::Pair(e) => e.fmt(f),
            Self::Cvar(e) => e.fmt(f),
            Self::Velocity(e) => e.fmt(f),
        }
    }
}
struct Runtime<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    processed: Arc<Mutex<SkeletonProcessedContact>>,
    references: &'a InputCvarReferences,
}
impl GroundHangPinSource for Runtime<'_> {
    type Error = HangPinRuntimeError;
    fn hang_flags(&mut self) -> Result<u32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .retained_hang_flags()
            .ok_or(Self::Error::Missing("Processed hang flags5ac"))
    }
    fn hang_frames(&mut self) -> Result<i32, Self::Error> {
        self.references.try_hang_frames().map_err(Self::Error::Cvar)
    }
    fn deck_speed(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .deck_speed()
            .map(|v| v.value)
            .ok_or(Self::Error::Missing("deck speed968"))
    }
    fn wipeout_frames(&mut self) -> Result<i32, Self::Error> {
        self.references
            .try_hung_up_wipeout_frames()
            .map_err(Self::Error::Cvar)
    }
    fn teleport_frames(&mut self) -> Result<i32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .input_controls()
            .frames_since_teleport
            .ok_or(Self::Error::Missing("frames since teleport928"))
    }
    fn maximum_pin_frames(&mut self) -> Result<i32, Self::Error> {
        self.references
            .try_max_ground_pin_frames()
            .map_err(Self::Error::Cvar)
    }
    fn maximum_pin_start_frames(&mut self) -> Result<i32, Self::Error> {
        self.references
            .try_max_ground_pin_start_frames()
            .map_err(Self::Error::Cvar)
    }
    fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .known_flags_8c8()
            .ok_or(Self::Error::Missing("flags8c8"))
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(Self::Error::Missing("time delta93c"))
    }
    fn deck_y(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .deck_transform()
            .map(|m| m.columns()[3].xyz[1])
            .ok_or(Self::Error::Missing("deck position Y74"))
    }
    fn deck_position(&mut self) -> Result<Vector, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .deck_transform()
            .map(|m| m.columns()[3])
            .ok_or(Self::Error::Missing("deck position70"))
    }
    fn set_velocity(&mut self, velocity: Vector) -> Result<(), Self::Error> {
        self.world
            .try_set_board_velocity(self.board, &velocity)
            .map_err(Self::Error::Velocity)
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_ground_hang_pinning<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: GroundBeforeManageHangUps,
        references: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<GroundSkateboardComplete, HangPinError<HangPinRuntimeError, N::Error>> {
        use HangPinRuntimeError as E;
        let owner = Arc::clone(&before.0.before.before.owner);
        let same = self
            .skaters
            .as_mut()
            .ok_or(HangPinError::Source(E::Missing("skater pool")))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(|e| HangPinError::Source(E::Pair(e)))?;
        if !same {
            return Err(HangPinError::Source(E::Foreign));
        }
        let mut state = owner
            .physics_state()
            .lock()
            .map_err(|_| HangPinError::Source(E::Poisoned))?;
        if state.current != CurrentPhysicalState::Ground {
            return Err(HangPinError::Source(E::Foreign));
        }
        let ground = state
            .ground
            .as_mut()
            .ok_or(HangPinError::Source(E::Missing("Ground")))?;
        if !ground.bindings.board.same_as(board) || !ground.bindings.skeleton.same_as(skeleton) {
            return Err(HangPinError::Source(E::Foreign));
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| HangPinError::Source(E::Poisoned))?
                .processed(),
        );
        let mut runtime = Runtime {
            world: self,
            board,
            processed,
            references,
        };
        try_hang_and_halfpipe(&mut ground.fields, &mut runtime)?;
        try_consider_pinning(&mut ground.fields, &mut runtime, numeric)?;
        drop(state);
        Ok(GroundSkateboardComplete { owner })
    }
}
