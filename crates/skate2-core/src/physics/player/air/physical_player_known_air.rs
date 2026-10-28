//! Live Air.Update82d9f7b4 -> Skeleton.UpdateKnownAir, advancing retained COM.
use crate::{
    dynamic_assembly_session::{
        skater_pool::SimulationSkaterAssemblyHandle,
        skeleton_normal::general_owner::{
            SkeletonGroundedProgress, known_air::KnownAirSkeletonError,
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
    pub(crate) fn try_update_pair_known_air_skeleton<
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    >(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        selected_target: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut N,
        rounding: &mut S,
        ortho: &mut M,
    ) -> Result<SkeletonGroundedProgress, KnownAirSkeletonError<N::Error, S::Error, CvarError>>
    {
        use KnownAirSkeletonError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let target_com = if let Some(target) = selected_target {
            if owner
                .physics_state()
                .lock()
                .map_err(|_| E::Poisoned)?
                .current
                != CurrentPhysicalState::KnownAir
            {
                return Err(E::ForeignOwner);
            }
            target
        } else {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if state.current != CurrentPhysicalState::Air {
                return Err(E::ForeignOwner);
            }
            let air = state.air.as_mut().ok_or(E::Missing("Air state"))?;
            if !air.fields.use_com_velocity {
                return Err(E::Missing("KnownAir selected only by use-COM-velocity"));
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
            crate::physical_player_air_state::trajectory::advance(&mut air.fields.com_trajectory)
        };
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
            .try_update_paired_skeleton_known_air(
                &mut self.backend,
                &mut self.hooks,
                skeleton,
                board,
                &owner,
                &processed,
                reckoning,
                air_attributes,
                target_com,
                numeric,
                rounding,
                ortho,
                &mut services,
            )
    }
}
