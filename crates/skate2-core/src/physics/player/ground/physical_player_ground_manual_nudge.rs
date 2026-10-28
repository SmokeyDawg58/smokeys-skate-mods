//! Unregistered Ground82d9a774..a7e4, including nested Nudge's first queue append.
use super::{
    antiflip_nudge::{NudgeError, try_update_nudge},
    ground_ordinary_forces::GroundBeforeManualEffect,
    manual_effect_source::{ManualRuntime, ManualRuntimeError},
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
    },
    input_cvar_runtime::{
        InputCvarReferences,
        antiflip_nudge::{NudgeRuntime, NudgeRuntimeError},
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_brake_attributes::SkateboardBrakeAttributeError,
    skateboard_brake_drag::{SkateboardBrakeDragError, SkateboardBrakeDragServices},
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_force_runtime::entry::manual_effect::{ManualEffect, ManualEffectError},
    skeleton_processed_contact::ground_force_input::{GroundForceInput, GroundForceInputError},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;
pub(crate) struct GroundBeforeForceQueues {
    pub(crate) before: GroundBeforeManualEffect,
    pub(crate) manual: ManualEffect,
    pub(crate) linear_drag: f32,
    pub(crate) combined_push_brake: SkeletonRootVector,
}
#[derive(Debug)]
pub(crate) enum GroundManualNudgeError<N, R> {
    Missing(&'static str),
    Poisoned,
    Foreign,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Manual(ManualEffectError<ManualRuntimeError, N, R>),
    Drag(SkateboardBrakeDragError<GroundForceInputError, SkateboardBrakeAttributeError>),
    Numeric(N),
    Nudge(NudgeError<NudgeRuntimeError, N>),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display for GroundManualNudgeError<N, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "ground manual/nudge missing {s}"),
            Self::Poisoned => f.write_str("ground owner lock poisoned"),
            Self::Foreign => f.write_str("ground manual/nudge foreign pair or state"),
            Self::Pair(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
            Self::Manual(e) => e.fmt(f),
            Self::Drag(e) => write!(f, "ground linear drag: {e:?}"),
            Self::Numeric(e) => write!(f, "ground force numeric: {e:?}"),
            Self::Nudge(e) => e.fmt(f),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_ground_manual_nudge<
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: GroundBeforeManualEffect,
        references: &InputCvarReferences,
        numeric: &mut N,
        sine: &mut R,
    ) -> Result<GroundBeforeForceQueues, GroundManualNudgeError<N::Error, R::Error>> {
        use GroundManualNudgeError as E;
        let owner = Arc::clone(&before.before.owner);
        let overlay = self.debug_overlay.clone();
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
        if !ground.bindings.board.same_as(board) || !ground.bindings.skeleton.same_as(skeleton) {
            return Err(E::Foreign);
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let (manual, linear_drag, combined_push_brake) = pool
            .try_with_ground_force_board(&self.backend, board, |session, runtime, attributes| {
                let scratch = SkeletonRootVector {
                    xyz: [f32::NAN; 3],
                    w: None,
                };
                let mut manual = ManualEffect {
                    angular_displacement: scratch,
                    corrective_force: SkateboardFootForce {
                        world_space_force: scratch,
                        local_space_position: scratch,
                    },
                };
                runtime
                    .entry
                    .try_manual_effect(
                        &mut ManualRuntime {
                            owner: &owner,
                            processed: &processed,
                            board: session,
                            backend: &self.backend,
                            attributes,
                            references,
                            overlay: &overlay,
                        },
                        false,
                        numeric,
                        sine,
                        &mut manual,
                    )
                    .map_err(E::Manual)?;
                let linear_drag = {
                    // Scope ends before Nudge and any future owner callback.
                    let reckoning = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
                    SkateboardBrakeDragServices {
                        input: &mut GroundForceInput {
                            processed: &processed,
                            reckoning: reckoning.reckoning(),
                        },
                        attributes: &mut runtime.brake_attributes,
                    }
                    .try_linear_drag()
                    .map_err(E::Drag)?
                };
                let push = before.push.world_space_force;
                let squared = numeric.dot_vector(push, push).map_err(E::Numeric)?;
                let combined = push.zip(before.brake.world_space_force, |a, b| a + b);
                if squared > 1.0 {
                    ground.fields.is_push_accelerating = true;
                }
                try_update_nudge(
                    &mut ground.fields,
                    &mut NudgeRuntime {
                        processed: &processed,
                        references,
                        overlay: &overlay,
                    },
                    &mut runtime.queue,
                    numeric,
                )
                .map_err(E::Nudge)?;
                Ok::<_, E<N::Error, R::Error>>((manual, linear_drag, combined))
            })
            .map_err(E::Board)??;
        Ok(GroundBeforeForceQueues {
            before,
            manual,
            linear_drag,
            combined_push_brake,
        })
    }
}
