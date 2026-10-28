//! Same-owner animated Skeleton: Air82d9f7c8(false), GroundAnimation82d98da4(true).
use crate::{
    dynamic_assembly_session::{
        skater_pool::SimulationSkaterAssemblyHandle,
        skeleton_normal::general_owner::{
            SkeletonGroundedProgress, animated::AnimatedSkeletonError,
        },
    },
    input_cvar_runtime::{
        InputCvarReferences,
        ground_skateboard::{GroundSkateboardCvarError, GroundSkateboardCvars},
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_spawn_math::SkateboardSpawnMathNumeric,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;
type CvarError = super::GroundedWorldServiceError<GroundSkateboardCvarError>;
impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair and independent numeric services"
    )]
    pub(crate) fn try_update_pair_animated_skeleton<
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    >(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        fast_blend: bool,
        numeric: &mut N,
        rounding: &mut S,
        ortho: &mut M,
    ) -> Result<SkeletonGroundedProgress, AnimatedSkeletonError<N::Error, S::Error, CvarError>>
    {
        use AnimatedSkeletonError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            match (state.current, fast_blend) {
                (CurrentPhysicalState::Air, false) => {
                    let air = state.air.as_ref().ok_or(E::Missing("Air state"))?;
                    if air.fields.use_com_velocity {
                        return Err(E::Missing("selected KnownAir continuation"));
                    }
                    if !air.bindings.board.same_as(board)
                        || !air.bindings.skeleton.same_as(skeleton)
                        || !air
                            .bindings
                            .owner
                            .upgrade()
                            .is_some_and(|v| Arc::ptr_eq(&v, &owner))
                    {
                        return Err(E::ForeignOwner);
                    }
                }
                (CurrentPhysicalState::GroundAnimation, true) => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    let Some(StatePayload::GroundAnimation(s)) = states
                        .states
                        .get(&StateKind::GroundAnimation)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    else {
                        return Err(E::Missing("GroundAnimation state"));
                    };
                    if !s.base.physical.board.same_as(board)
                        || !s.base.physical.skeleton.same_as(skeleton)
                        || !s
                            .base
                            .physical
                            .owner
                            .upgrade()
                            .is_some_and(|v| Arc::ptr_eq(&v, &owner))
                    {
                        return Err(E::ForeignOwner);
                    }
                }
                // Grind UpdateSubstate82d90440 and Nonspecific82d9c040/9c084:
                // UpdateAnimatedSkateboard(processed, false).
                (CurrentPhysicalState::Grind(_) | CurrentPhysicalState::Nonspecific, false) => {
                    use crate::physical_player_assembly::remaining_states::StateKind;
                    let kind = match state.current {
                        CurrentPhysicalState::Grind(grind) => StateKind::grind(grind),
                        _ => StateKind::Nonspecific,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    let bindings = states
                        .published_bindings(kind)
                        .ok_or(E::Missing("grind state"))?;
                    if !bindings.board.same_as(board)
                        || !bindings.skeleton.same_as(skeleton)
                        || !bindings
                            .owner
                            .upgrade()
                            .is_some_and(|v| Arc::ptr_eq(&v, &owner))
                    {
                        return Err(E::ForeignOwner);
                    }
                }
                _ => return Err(E::ForeignOwner),
            }
        }
        let state_changer = owner.state_changer().lock().map_err(|_| E::Poisoned)?;
        let air_attributes = state_changer
            .attributes
            .as_ref()
            .ok_or(E::Missing("retained global Air attributes"))?;
        let reckoning = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
        let processed = Arc::clone(reckoning.processed());
        let reckoning = reckoning.reckoning().ok_or(E::Missing("Reckoning"))?;
        let mut cvars = GroundSkateboardCvars::new(refs, self.debug_overlay.clone());
        let mut services = super::services::GroundedWorldServices {
            cvars: &mut cvars,
            queries: self.world_line_queries.as_mut(),
            dmos: self.dmos.as_ref(),
            ground,
        };
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_update_paired_skeleton_animated(
                &mut self.backend,
                &mut self.hooks,
                skeleton,
                board,
                &owner,
                &processed,
                reckoning,
                air_attributes,
                fast_blend,
                numeric,
                rounding,
                ortho,
                &mut services,
            )
    }
}
