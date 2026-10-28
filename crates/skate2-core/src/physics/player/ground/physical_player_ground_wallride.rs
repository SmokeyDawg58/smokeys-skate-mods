//! Ground.UpdateSkateboard82d9a130..a1d0, retaining its local force for later use.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
    },
    input_cvar_runtime::{
        InputCvarReferences,
        wallride::{WallRideCvars, WallRideDebugError},
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_force_runtime::wallride::{WallRideError, try_calculate},
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

#[path = "../runtime/physical_player_wallride_source.rs"]
pub(crate) mod source;
use source::{WallRideRuntime, WallRideRuntimeError};

#[derive(Debug)]
pub(crate) enum GroundWallRideError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Calculate(WallRideError<WallRideRuntimeError<WallRideDebugError>, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundWallRideError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(value) => write!(f, "Ground wallride requires {value}"),
            Self::Poisoned => f.write_str("Ground wallride owner lock poisoned"),
            Self::ForeignState => f.write_str("Ground wallride requires same selected Ground pair"),
            Self::Pair(error) => error.fmt(f),
            Self::Board(error) => error.fmt(f),
            Self::Calculate(error) => error.fmt(f),
        }
    }
}

pub(crate) struct GroundBeforeFakeTorque {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) wallride_force: SkateboardFootForce,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_ground_wallride<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        static_ground: &StaticGroundPool,
        references: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<GroundBeforeFakeTorque, GroundWallRideError<N::Error>> {
        use GroundWallRideError as E;
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.current != CurrentPhysicalState::Ground {
            return Err(E::ForeignState);
        }
        let ground = state.ground.as_mut().ok_or(E::Missing("Ground state"))?;
        if !ground.bindings.board.same_as(board)
            || !ground.bindings.skeleton.same_as(skeleton)
            || !ground
                .bindings
                .owner
                .upgrade()
                .is_some_and(|value| Arc::ptr_eq(&value, &owner))
        {
            return Err(E::ForeignState);
        }
        let wall_time = ground.fields.time_wall_riding; //a138
        let zero = SkeletonRootVector::known([0.0; 4]);
        let mut force = SkateboardFootForce {
            world_space_force: zero,
            local_space_position: zero,
        };
        let ground_time = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .input_controls()
            .time_on_ground
            .ok_or(E::Missing("time on ground9c0"))?; //a19c
        let active = pool
            .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                try_calculate(
                    &mut WallRideRuntime {
                        processed: &processed,
                        jump_flag: &mut ground.fields.do_jump,
                        jump_height: &mut ground.fields.jump_com_height,
                        jump_velocity: &mut ground.fields.jump_direction,
                        feet: &mut runtime.foot_attributes,
                        trajectory: &runtime.wallride_trajectory,
                        queries: self.world_line_queries.as_mut(),
                        dmos: self.dmos.as_ref(),
                        static_ground,
                        debug: &mut WallRideCvars {
                            references,
                            overlay: &self.debug_overlay,
                        },
                    },
                    numeric,
                    ground_time,
                    wall_time,
                    &mut force,
                )
            })
            .map_err(E::Board)?
            .map_err(E::Calculate)?;
        if active {
            let previous = ground.fields.time_wall_riding; //a1b4, fresh
            let dt = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .time_delta
                .ok_or(E::Missing("DT93c"))?;
            ground.fields.time_wall_riding = dt + previous;
        } else {
            ground.fields.time_wall_riding = 0.0;
        }
        ground.fields.exiting_wallride = false; //a1d0
        drop(state);
        Ok(GroundBeforeFakeTorque {
            owner,
            wallride_force: force,
        })
    }
}
