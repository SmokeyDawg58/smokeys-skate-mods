//! Actual `PhysicsAir::UpdateSkateboard`82d990b8 on the same retained pair.
use super::{
    air_jump_velocity::{self, AirJumpVelocityError},
    ground_control_complete::SteeringInput,
    physical_board_forces::PhysicalPlayerBoardForceError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::{SimulationConstructedBoardError, SimulationConstructedSteeringError},
    },
    input_cvar_runtime::InputCvarError,
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_slide_collision::{SkateboardSlideCollisionError, SkateboardSlideCollisionServices},
    skateboard_slide_collision_attributes::SkateboardSlideCollisionAttributeError,
    skeleton_processed_contact::ground_force_input::{GroundForceInput, GroundForceInputError},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum AirSkateboardError<N> {
    Missing(&'static str),
    Poisoned,
    Foreign,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Steering(SimulationConstructedSteeringError<&'static str>),
    Collision(
        SkateboardSlideCollisionError<
            GroundForceInputError,
            SkateboardSlideCollisionAttributeError,
            N,
        >,
    ),
    Cvar(InputCvarError),
    Jump(AirJumpVelocityError<N>),
    Velocity(PhysicalPlayerBoardForceError),
}
impl<N: std::fmt::Debug> std::fmt::Display for AirSkateboardError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("AirSkateboardError: Poisoned"),
            Self::Foreign => f.write_str("AirSkateboardError: Foreign"),
            Self::Missing(error) => write!(f, "AirSkateboardError Missing: {error:?}"),
            Self::Pair(error) => write!(f, "AirSkateboardError Pair: {error:?}"),
            Self::Board(error) => write!(f, "AirSkateboardError Board: {error:?}"),
            Self::Steering(error) => write!(f, "AirSkateboardError Steering: {error:?}"),
            Self::Collision(error) => write!(f, "AirSkateboardError Collision: {error:?}"),
            Self::Cvar(error) => write!(f, "AirSkateboardError Cvar: {error:?}"),
            Self::Jump(error) => write!(f, "AirSkateboardError Jump: {error:?}"),
            Self::Velocity(error) => write!(f, "AirSkateboardError Velocity: {error:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_air_skateboard<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<(), AirSkateboardError<N::Error>> {
        use AirSkateboardError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if state.current != CurrentPhysicalState::Air {
                return Err(E::Foreign);
            }
            let air = state.air.as_ref().ok_or(E::Missing("Air state"))?;
            if !air.bindings.board.same_as(board)
                || !air.bindings.skeleton.same_as(skeleton)
                || !air
                    .bindings
                    .owner
                    .upgrade()
                    .is_some_and(|p| Arc::ptr_eq(&p, &owner))
            {
                return Err(E::Foreign);
            }
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        self.try_update_constructed_board_steering(board, 0.0, &mut SteeringInput(&processed))
            .map_err(E::Steering)?;
        let zero = SkeletonRootVector::known([0.0; 4]);
        let mut force = SkateboardFootForce {
            world_space_force: zero,
            local_space_position: zero,
        };
        // Native scratch is uninitialized; this backing is never consumed on false.
        let mut torque = SkeletonRootVector {
            xyz: [f32::NAN; 3],
            w: None,
        };
        let active = {
            let reckoning = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
            let mut input = GroundForceInput {
                processed: &processed,
                reckoning: reckoning.reckoning(),
            };
            self.skaters
                .as_mut()
                .ok_or(E::Missing("skater pool"))?
                .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                    SkateboardSlideCollisionServices {
                        input: &mut input,
                        attributes: &mut runtime.slide_collision_attributes,
                        numeric,
                    }
                    .try_collision(&mut force, &mut torque, None)
                })
                .map_err(E::Board)?
                .map_err(E::Collision)?
        };
        if active {
            // No Ground timer, angular target, entry flag, torque queue or push-stop store.
            self.skaters
                .as_mut()
                .ok_or(E::Missing("skater pool"))?
                .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                    runtime.queue.add(15, force);
                })
                .map_err(E::Board)?;
            return Ok(()); // Includes the source capped-queue rejection case.
        }
        let frames = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .frames_since_jump_correct
            .ok_or(E::Missing("frames920"))?;
        let limit = owner
            .inputs()
            .cvars
            .try_physics_air_jump_fix_frames()
            .map_err(E::Cvar)?;
        if frames < limit {
            let velocity =
                air_jump_velocity::try_calculate(&owner, frames, &self.debug_overlay, numeric)
                    .map_err(E::Jump)?;
            self.try_set_board_velocity(board, &velocity)
                .map_err(E::Velocity)?;
        }
        Ok(())
    }
}
