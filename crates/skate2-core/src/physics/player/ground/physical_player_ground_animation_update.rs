//! Actual state103 update composition, using the same animation/force owners.
use super::{
    ground_control_complete::SteeringInput, ground_pre_solve::HostGroundPreparationNumeric,
};
use crate::{
    dynamic_assembly_session::{
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
            board_construction::SimulationConstructedSteeringError,
        },
        skeleton_normal::general_owner::{
            SkeletonGeneralContinuation, animated::AnimatedSkeletonError,
        },
    },
    input_cvar_runtime::{
        InputCvarError, InputCvarReferences,
        ground_foot_force::{GroundFootCvars, GroundFootDebugError},
        ground_skateboard::GroundSkateboardCvarError,
        reckoning::ReckoningCvarAdapter,
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{JumpInfo, StateKind, StatePayload},
    },
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_reckoning_owner::{
        PhysicalPlayerReckoningPhase, PhysicalPlayerReckoningUpdateError,
    },
    reckoning_numeric::ReckoningHostNumericError,
    skeleton_processed_contact::ground_force_input::{GroundForceInput, GroundForceInputError},
    static_ground_pool::StaticGroundPool,
};
use std::{convert::Infallible, sync::Arc};
type NumericError = ReckoningHostNumericError;
#[derive(Debug)]
pub(crate) enum GroundAnimationUpdateError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pair(SimulationSkeletonJumpPairError),
    Reckoning(PhysicalPlayerReckoningUpdateError<Infallible, InputCvarError, NumericError>),
    Animated(
        AnimatedSkeletonError<
            NumericError,
            Infallible,
            super::physical_grounded::GroundedWorldServiceError<GroundSkateboardCvarError>,
        >,
    ),
    Steering(SimulationConstructedSteeringError<&'static str>),
    Material(super::physical_wheel_material::PhysicalPlayerWheelMaterialError<&'static str>),
    Cvar(InputCvarError),
    Forces(
        super::physical_board_forces::PhysicalPlayerGroundForceError<
            GroundForceInputError,
            GroundFootDebugError,
            NumericError,
        >,
    ),
    Jump(
        super::ground_jump_runtime::kernel::Error<
            super::ground_jump_runtime::SourceError,
            NumericError,
        >,
    ),
    Launch(super::ground_jump_runtime::LaunchError),
}
impl std::fmt::Display for GroundAnimationUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "state103 Update missing {v}"),
            Self::Poisoned => f.write_str("state103 owner poisoned"),
            Self::ForeignOwner => f.write_str("state103 foreign owner"),
            Self::Pair(e) => write!(f, "state103 pair: {e:?}"),
            Self::Reckoning(e) => write!(f, "state103 Reckoning: {e:?}"),
            Self::Animated(e) => write!(f, "state103 animated Skeleton: {e}"),
            Self::Steering(e) => write!(f, "state103 steering: {e:?}"),
            Self::Material(e) => write!(f, "state103 material: {e:?}"),
            Self::Cvar(e) => write!(f, "state103 jump cvar: {e}"),
            Self::Forces(e) => write!(f, "state103 force tail: {e:?}"),
            Self::Jump(e) => write!(f, "state103 jump: {e}"),
            Self::Launch(e) => write!(f, "state103 launch: {e}"),
        }
    }
}
pub(crate) struct GroundAnimationComplete {
    pub(super) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(super) reckoning: PhysicalPlayerReckoningPhase,
}
impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_lines,
        reason = "source state103 ordered producer calls and retained stores"
    )]
    pub(crate) fn try_update_pair_ground_animation(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
        grind_policy: super::ground_pre_solve::GroundPreparationGrindPolicy,
    ) -> Result<GroundAnimationComplete, GroundAnimationUpdateError> {
        use GroundAnimationUpdateError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        if owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .current
            != CurrentPhysicalState::GroundAnimation
        {
            return Err(E::ForeignOwner);
        }
        {
            let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::GroundAnimation(state)) = states
                .states
                .get_mut(&StateKind::GroundAnimation)
                .filter(|s| s.published)
                .and_then(|s| s.value.as_mut())
            else {
                return Err(E::Missing("retained GroundAnimation"));
            };
            if !state.base.physical.board.same_as(board)
                || !state.base.physical.skeleton.same_as(skeleton)
                || !state
                    .base
                    .physical
                    .owner
                    .upgrade()
                    .is_some_and(|p| Arc::ptr_eq(&p, &owner))
            {
                return Err(E::ForeignOwner);
            }
            state.jumped_this_frame = Some(false); //82d98d80, before any Reckoning reads
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let phase = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_update_grounded_fixed_heading_live(
                &processed,
                &mut ReckoningCvarAdapter(refs),
                &mut numeric.reckoning,
            )
            .map_err(E::Reckoning)?;
        let progress = self
            .try_update_pair_animated_skeleton(
                board,
                skeleton,
                ground,
                refs,
                true,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut numeric.spawn_math,
            )
            .map_err(E::Animated)?;
        if progress.next != SkeletonGeneralContinuation::Complete {
            return Err(E::Missing("complete animated Skeleton"));
        }
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.mark_add_skateboard_error();
            })
            .map_err(E::Pair)?;
        self.try_update_constructed_board_steering(board, 0.0, &mut SteeringInput(&processed))
            .map_err(E::Steering)?;
        self.try_set_board_wheel_material(board, || {
            processed
                .lock()
                .map_err(|_| "processed lock")?
                .selected_surface()
                .and_then(|s| s.material.clone())
                .ok_or("selected wheel material")
        })
        .map_err(E::Material)?;
        let result = ground_jump(
            || {
                refs.try_capture_ground_jump_maximum_error()
                    .map_err(E::Cvar)?
                    .read()
                    .map_err(E::Cvar)
            },
            || Ok(processed.lock().map_err(|_| E::Poisoned)?.known_flags_8c4()),
            |maximum| {
                super::ground_jump_runtime::calculate(self, &owner, board, maximum, numeric)
                    .map_err(E::Jump)
            },
        )?;
        let do_jump = result.do_jump;
        let jump_velocity = result.velocity;
        if do_jump && std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            let p = processed.lock().map_err(|_| E::Poisoned)?;
            eprintln!(
                "known air: pop velocity={:?} strength={:?} jump_normal={:?} com={:?} ground={:?}",
                jump_velocity.xyz,
                p.jump_events().strength(),
                p.toolkit_state().processed_jump_normal().map(|n| n.xyz),
                p.prior_output().centre_of_mass().map(|c| c.value.xyz),
                p.ground_position_fields().ground_position.map(|g| g.xyz)
            );
        }
        {
            let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::GroundAnimation(state)) = states
                .states
                .get_mut(&StateKind::GroundAnimation)
                .and_then(|s| s.value.as_mut())
            else {
                return Err(E::Missing("retained GroundAnimation"));
            };
            state.jump_info = Some(result); //a7f4/a800/a80c after completed calculator return
        }
        if do_jump {
            super::ground_jump_runtime::launch(
                self,
                &owner,
                board,
                skeleton,
                jump_velocity,
                ground,
                numeric,
                grind_policy,
            )
            .map_err(E::Launch)?;
        } else {
            let reckoning = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
            let mut input = GroundForceInput {
                processed: &processed,
                reckoning: reckoning.reckoning(),
            };
            let overlay = self.debug_overlay.clone();
            let mut debug = GroundFootCvars {
                references: refs,
                overlay: &overlay,
            };
            self.try_update_board_grounded_force_tail(
                board,
                &mut input,
                &mut debug,
                &mut numeric.reckoning,
            )
            .map_err(E::Forces)?;
        }
        Ok(GroundAnimationComplete {
            owner,
            reckoning: phase,
        })
    }
}

/// `CalcGroundJump82b315a8`: live maximum read precedes the local result stores
/// and actual `StartJump` gate. Only a completed calculator result is published.
fn ground_jump(
    maximum: impl FnOnce() -> Result<f32, GroundAnimationUpdateError>,
    flags: impl FnOnce() -> Result<(u32, u32), GroundAnimationUpdateError>,
    active: impl FnOnce(f32) -> Result<JumpInfo, GroundAnimationUpdateError>,
) -> Result<JumpInfo, GroundAnimationUpdateError> {
    use GroundAnimationUpdateError as E;
    let maximum = maximum()?;
    let result = JumpInfo {
        velocity: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
        com_height: 0.0,
        do_jump: false,
    };
    let (flags, known) = flags()?;
    if known & 0x0040_0000 == 0 {
        return Err(E::Missing("Processed StartJump bit"));
    }
    if flags & 0x0040_0000 != 0 {
        return active(maximum);
    }
    Ok(result)
}
#[cfg(test)]
#[path = "physical_player_ground_animation_update_tests.rs"]
mod tests;
