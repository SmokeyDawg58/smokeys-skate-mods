//! Ground jump/collision branch82d9a430..a658.
use super::{ground_fake_torque::GroundBeforeJump, ground_wallride::GroundBeforeFakeTorque};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
        target_angular::TargetAngularOwnerError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::{ReckoningNumeric, try_normalize_safe_with_refinement},
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_slide_collision::{
        SkateboardSlideCollisionError, SkateboardSlideCollisionInputRead,
        SkateboardSlideCollisionServices,
    },
    skateboard_slide_collision_attributes::{
        SkateboardSlideCollisionAttributeError, SkateboardSlideCollisionAttributeRead,
        SlideCollisionGlobal,
    },
    skeleton_processed_contact::ground_force_input::{GroundForceInput, GroundForceInputError},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

/// The collision response branch exits this native `UpdateSkateboard` function.
/// The no-response branch continues to the actual feet threshold at a658.
pub(crate) enum GroundAfterCollision {
    JumpLaunch {
        before: GroundBeforeFakeTorque,
        velocity: SkeletonRootVector,
    },
    CollisionExit(GroundBeforeFakeTorque),
    BeforeBraking(GroundBeforeFakeTorque),
}
#[derive(Debug)]
pub(crate) enum GroundCollisionError<N> {
    Missing(&'static str),
    Poisoned,
    Foreign,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Collision(
        SkateboardSlideCollisionError<
            GroundForceInputError,
            SkateboardSlideCollisionAttributeError,
            N,
        >,
    ),
    Attribute(SkateboardSlideCollisionAttributeError),
    Angular(TargetAngularOwnerError<N>),
    Input(GroundForceInputError),
    Numeric(N),
    Cvar(InputCvarError),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundCollisionError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "Ground collision requires {v}"),
            Self::Poisoned => f.write_str("Ground collision owner poisoned"),
            Self::Foreign => {
                f.write_str("Ground collision requires the same selected Ground owner")
            }
            Self::Pair(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
            Self::Collision(e) => e.fmt(f),
            Self::Attribute(e) => e.fmt(f),
            Self::Angular(e) => e.fmt(f),
            Self::Input(e) => e.fmt(f),
            Self::Numeric(e) => write!(f, "Ground collision numeric: {e:?}"),
            Self::Cvar(e) => e.fmt(f),
        }
    }
}
fn normalize_refinement(squared: f32, seed: f32) -> f32 {
    let square = seed * seed;
    let residual = -(squared.mul_add(square, -1.0));
    let first = (seed * 0.5).mul_add(residual, seed);
    let square = first * first;
    let residual = -(squared.mul_add(square, -1.0));
    (first * 0.5).mul_add(residual, first)
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_ground_collision<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: GroundBeforeJump,
        references: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<GroundAfterCollision, GroundCollisionError<N::Error>> {
        use GroundCollisionError as E;
        let before = before.0;
        let owner = Arc::clone(&before.owner);
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let same = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::Foreign);
        }
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.current != CurrentPhysicalState::Ground {
            return Err(E::Foreign);
        }
        let ground = state.ground.as_mut().ok_or(E::Missing("Ground"))?;
        if !ground.bindings.board.same_as(board)
            || !ground.bindings.skeleton.same_as(skeleton)
            || !ground
                .bindings
                .owner
                .upgrade()
                .is_some_and(|actual| Arc::ptr_eq(&actual, &owner))
        {
            return Err(E::Foreign);
        }
        if ground.fields.do_jump {
            // a430: leave collision inputs untouched. The host continuation
            // completes a444..a4d4 through the existing trajectory services.
            return Ok(GroundAfterCollision::JumpLaunch {
                before,
                velocity: ground.fields.jump_direction,
            });
        }
        let reckoning_owner = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
        let processed = Arc::clone(reckoning_owner.processed());
        // Collision's early false branch never reads Reckoning. Preserve that
        // late requirement rather than turning it into an unconditional gate.
        let reckoning = reckoning_owner.reckoning();
        let mut input = GroundForceInput {
            processed: &processed,
            reckoning,
        };
        let zero = SkeletonRootVector::known([0.0; 4]);
        let mut force = SkateboardFootForce {
            world_space_force: zero,
            local_space_position: zero,
        };
        // This is write-only host backing, NOT a claimed native initialization.
        // Existing CollisionForce always overwrites torque before returning true;
        // both false routes discard it, and failures expose none of this scratch.
        let mut torque = SkeletonRootVector {
            xyz: [f32::NAN; 3],
            w: None,
        };
        let active = pool
            .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                SkateboardSlideCollisionServices {
                    input: &mut input,
                    attributes: &mut runtime.slide_collision_attributes,
                    numeric,
                }
                .try_collision(&mut force, &mut torque, None)
            })
            .map_err(E::Board)?
            .map_err(E::Collision)?;
        if !active {
            return Ok(GroundAfterCollision::BeforeBraking(before));
        }
        // Requires the proposed typed migration of these SAME fields; never
        // replace native unknown W by the former array's constructor zeros.
        ground.fields.collision_force[0] = force.world_space_force; //a560
        ground.fields.collision_force[1] = force.local_space_position; //a568
        ground.fields.collision_torque = torque; //a56c
        let duration = pool
            .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                let attrs = &mut runtime.slide_collision_attributes;
                let global = attrs.capture_global(SlideCollisionGlobal::Collision)?;
                let pointer = attrs.lookup(&global, 0xd613_692c_606f_e326)?;
                attrs.float(&pointer)
            })
            .map_err(E::Board)?
            .map_err(E::Attribute)?;
        ground.fields.collision_timer = duration; //a5d0
        pool.try_target_board_angular_displacement(&mut self.backend, board, torque, numeric)
            .map_err(E::Angular)?; //a5d8
        pool.try_with_ground_control_board(&self.backend, board, |runtime, _| {
            runtime.entry.mark_collision_response(); //a5ec, SAME530 owner
            runtime.queue.add(15, force); //a5f4; fixed21-capacity source behavior
        })
        .map_err(E::Board)?;
        let normal = try_normalize_safe_with_refinement(
            force.world_space_force,
            zero,
            numeric,
            normalize_refinement,
        )
        .map_err(E::Numeric)?;
        let velocity = SkateboardSlideCollisionInputRead::velocity(&mut input).map_err(E::Input)?; //a62c, fresh
        let dot = numeric.dot_vector(normal, velocity).map_err(E::Numeric)?;
        let threshold = references.try_dot_to_stop_push().map_err(E::Cvar)?;
        // NOTLT includes unordered. The store happens for BOTH paths, clearing
        // a previously true request for equality, ordered greater and NaN.
        ground.fields.request_push_stop = dot < threshold; //a650
        if ground.fields.request_push_stop
            && std::env::var_os("SKATE2_TRACE_PUSH_INTERRUPTION").is_some()
        {
            eprintln!(
                "push interruption collision: dot={dot} threshold={threshold} velocity={:?} force={:?}",
                velocity.xyz, force.world_space_force.xyz
            );
        }
        Ok(GroundAfterCollision::CollisionExit(before))
    }
}
