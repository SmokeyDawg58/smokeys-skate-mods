//! Actual Air.Update82d9f6f0..f814 continuation on the retained pair.
use super::{
    air_skateboard_runtime::AirSkateboardError, air_update_prefix::AirBeforeComCorrection,
    ground_pre_solve::HostGroundPreparationNumeric,
};
use crate::{
    dynamic_assembly_session::{
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonJumpPairError, board_entry::SkateboardEntrySessionError,
        },
        skeleton_normal::general_owner::{
            SkeletonGeneralContinuation, animated::AnimatedSkeletonError,
        },
    },
    input_cvar_runtime::ground_skateboard::GroundSkateboardCvarError,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::air_post_solve::{AirStateAccessError, validate_air},
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    reckoning_numeric::ReckoningHostNumericError,
    simulation_assembly_backend::constraint_session::air_com_correction::AirComCorrectionError,
    static_ground_pool::StaticGroundPool,
};
use std::{convert::Infallible, sync::Arc};
type NumericError = ReckoningHostNumericError;
#[derive(Debug)]
pub(crate) enum AirUpdateFinishError {
    Missing(&'static str), Poisoned, ForeignOwner,
    Pair(SimulationSkeletonJumpPairError), Pool(SimulationSkaterPoolError),
    State(AirStateAccessError), Transform(SkateboardEntrySessionError),
    Com(AirComCorrectionError<NumericError>),
    KnownAir(crate::dynamic_assembly_session::skeleton_normal::general_owner::known_air::KnownAirSkeletonError<NumericError, Infallible,
        super::physical_grounded::GroundedWorldServiceError<GroundSkateboardCvarError>>),
    StaleReckoning(crate::physical_player_reckoning_owner::PhysicalPlayerReckoningStalePhase),
    Animated(AnimatedSkeletonError<NumericError, Infallible,
        super::physical_grounded::GroundedWorldServiceError<GroundSkateboardCvarError>>),
    SkeletonIncomplete,
    Board(AirSkateboardError<NumericError>),
}
impl std::fmt::Display for AirUpdateFinishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("AirUpdateFinishError: Poisoned"),
            Self::ForeignOwner => f.write_str("AirUpdateFinishError: ForeignOwner"),
            Self::SkeletonIncomplete => f.write_str("AirUpdateFinishError: SkeletonIncomplete"),
            Self::Missing(error) => write!(f, "AirUpdateFinishError Missing: {error:?}"),
            Self::Pair(error) => write!(f, "AirUpdateFinishError Pair: {error:?}"),
            Self::Pool(error) => write!(f, "AirUpdateFinishError Pool: {error:?}"),
            Self::State(error) => write!(f, "AirUpdateFinishError State: {error:?}"),
            Self::Transform(error) => write!(f, "AirUpdateFinishError Transform: {error:?}"),
            Self::Com(error) => write!(f, "AirUpdateFinishError Com: {error:?}"),
            Self::KnownAir(error) => write!(f, "AirUpdateFinishError KnownAir: {error:?}"),
            Self::StaleReckoning(error) => {
                write!(f, "AirUpdateFinishError StaleReckoning: {error:?}")
            }
            Self::Animated(error) => write!(f, "AirUpdateFinishError Animated: {error:?}"),
            Self::Board(error) => write!(f, "AirUpdateFinishError Board: {error:?}"),
        }
    }
}

pub(crate) struct AirUpdateComplete {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) reckoning: PhysicalPlayerReckoningPhase,
}
impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered selected Air suffix with lazy reads and partial writes"
    )]
    pub(crate) fn try_finish_pair_air_update(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: AirBeforeComCorrection,
        ground: &StaticGroundPool,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<AirUpdateComplete, AirUpdateFinishError> {
        use AirUpdateFinishError as E;
        let owner = before.owner;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            validate_air(&state, &owner, board, skeleton).map_err(E::State)?;
        }
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_completed(&before.reckoning)
            .map_err(E::StaleReckoning)?;
        self.try_add_pair_air_com_correction(board, skeleton, &mut numeric.reckoning)
            .map_err(E::Com)?;
        let use_com = owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .air
            .as_ref()
            .ok_or(E::Missing("Air state"))?
            .fields
            .use_com_velocity;
        let progress = if use_com {
            self.try_update_pair_known_air_skeleton(
                board,
                skeleton,
                None,
                ground,
                &owner.inputs().cvars,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut numeric.spawn_math,
            )
            .map_err(E::KnownAir)?
        } else {
            self.try_update_pair_animated_skeleton(
                board,
                skeleton,
                ground,
                &owner.inputs().cvars,
                false,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut numeric.spawn_math,
            )
            .map_err(E::Animated)?
        };
        if progress.next != SkeletonGeneralContinuation::Complete {
            return Err(E::SkeletonIncomplete);
        }
        self.try_update_pair_air_skateboard(board, skeleton, &mut numeric.reckoning)
            .map_err(E::Board)?;
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.mark_add_skateboard_error();
            })
            .map_err(E::Pair)?;
        let transform = self
            .skaters
            .as_ref()
            .ok_or(E::Missing("skater pool"))?
            .session(board)
            .map_err(E::Pool)?
            .try_live_part_transform_typed(&self.backend, 6)
            .map_err(|e| E::Transform(SkateboardEntrySessionError::Transform(e)))?;
        let height = transform.columns()[3].xyz[1];
        let old_time = {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let fields = &mut state.air.as_mut().ok_or(E::Missing("Air state"))?.fields;
            let max_y = if fields.max_y > height {
                fields.max_y
            } else {
                height
            };
            let old_time = fields.time_in_state;
            fields.max_y = max_y;
            old_time
        };
        let dt = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .processed()
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("DT93c"))?;
        owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .air
            .as_mut()
            .ok_or(E::Missing("Air state"))?
            .fields
            .time_in_state = dt + old_time;
        Ok(AirUpdateComplete {
            owner,
            reckoning: before.reckoning,
        })
    }
}
