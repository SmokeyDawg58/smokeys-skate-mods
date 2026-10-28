//! Actual Ground.Update82da0ea8 same-owner Skeleton continuation.
//! Uses the retained processed owner through the shared late-reader/sink body.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonGroundedError,
        SimulationSkeletonJumpPairError, SimulationSkeletonOffboardGeneralError,
    },
    input_cvar_runtime::{
        InputCvarReferences,
        ground_skateboard::{GroundSkateboardCvarError, GroundSkateboardCvars},
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, ReckoningSinCosNumeric},
    simulation_assembly_backend::constraint_session::physical_teleport_now_prefix::TeleportNowFlagError,
    skateboard_spawn_math::{HostSkateboardSpawnMathNumeric, SkateboardSpawnMathNumeric},
    skeleton_processed_contact::SkeletonProcessedContact,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};
type WorldCvarError = super::GroundedWorldServiceError<GroundSkateboardCvarError>;

#[derive(Debug)]
pub(crate) enum OffboardGeneralWorldError {
    MissingPool,
    Update(
        SimulationSkeletonOffboardGeneralError<
            ReckoningHostNumericError,
            std::convert::Infallible,
            WorldCvarError,
        >,
    ),
}

#[derive(Debug)]
pub(crate) enum TeleportGroundSkateboardError<N> {
    Flag(TeleportNowFlagError),
    Missing(&'static str),
    Poisoned,
    Update(SimulationSkeletonGroundedError<N, std::convert::Infallible, WorldCvarError>),
}

impl PhysicalAssemblyWorld {
    /// TeleportNow 0x82b5eb50..60 brackets the lower GroundSkateboard body.
    /// The ordinary Ground entry is unsuitable because physical state has
    /// not yet transitioned from Teleporting.
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_update_pair_teleport_ground_skateboard<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        numeric: &mut N,
    ) -> Result<
        crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGroundedProgress,
        TeleportGroundSkateboardError<N::Error>,
    > {
        use TeleportGroundSkateboardError as E;
        self.try_set_pair_teleporting_now(board, skeleton, owner, true)
            .map_err(E::Flag)?;
        let reckoning_guard = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
        let processed = Arc::clone(reckoning_guard.processed());
        let reckoning = reckoning_guard.reckoning().ok_or(E::Missing("Reckoning"))?;
        let mut cvars =
            GroundSkateboardCvars::new(&owner.inputs().cvars, self.debug_overlay.clone());
        let mut services = super::services::GroundedWorldServices {
            cvars: &mut cvars,
            queries: self.world_line_queries.as_mut(),
            dmos: self.dmos.as_ref(),
            ground,
        };
        let progress = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_update_paired_skeleton_grounded_live(
                &mut self.backend,
                &mut self.hooks,
                skeleton,
                board,
                owner,
                &processed,
                reckoning,
                numeric,
                &mut HostReckoningSinCosNumeric,
                &mut HostSkateboardSpawnMathNumeric,
                &mut services,
            )
            .map_err(E::Update)?;
        drop(reckoning_guard);
        self.try_set_pair_teleporting_now(board, skeleton, owner, false)
            .map_err(E::Flag)?;
        Ok(progress)
    }
}

impl PhysicalAssemblyWorld {
    /// Enter only GeneralUpdate after UpdateOffBoard publishes its root and
    /// Processed orientation; the raw-Y branch skips ApplyBoardAnimation.
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_update_pair_offboard_general(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<
        crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGroundedProgress,
        OffboardGeneralWorldError,
    > {
        let mut cvars = GroundSkateboardCvars::new(refs, self.debug_overlay.clone());
        let mut services = super::services::GroundedWorldServices {
            cvars: &mut cvars,
            queries: self.world_line_queries.as_mut(),
            dmos: self.dmos.as_ref(),
            ground,
        };
        self.skaters
            .as_mut()
            .ok_or(OffboardGeneralWorldError::MissingPool)?
            .try_update_paired_skeleton_offboard_general(
                &mut self.backend,
                &mut self.hooks,
                skeleton,
                board,
                owner,
                processed,
                numeric,
                sin_cos,
                &mut services,
            )
            .map_err(OffboardGeneralWorldError::Update)
    }
}
#[derive(Debug)]
pub(crate) enum GroundSkateboardError<N, S> {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Update(SimulationSkeletonGroundedError<N, S, WorldCvarError>),
}
impl<N: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display for GroundSkateboardError<N, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "GroundSkateboard missing {v}"),
            Self::Poisoned => f.write_str("GroundSkateboard owner lock poisoned"),
            Self::ForeignState => {
                f.write_str("GroundSkateboard requires the selected same-owner Ground state")
            }
            Self::Pair(e) => write!(f, "GroundSkateboard pair: {e}"),
            Self::Update(e) => write!(f, "GroundSkateboard: {e}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_arguments,
        reason = "actual world/pair and distinct native numerical collaborators"
    )]
    pub(crate) fn try_update_pair_ground_skateboard<
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    >(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        references: &InputCvarReferences,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
    ) -> Result<
        crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGroundedProgress,
        GroundSkateboardError<N::Error, S::Error>,
    > {
        use GroundSkateboardError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if !matches!(
                state.current,
                CurrentPhysicalState::Ground
                    | CurrentPhysicalState::SlideGround
                    | CurrentPhysicalState::RevertGround
                    | CurrentPhysicalState::Skitching
                    | CurrentPhysicalState::FollowPath
                    | CurrentPhysicalState::Grind(_)
                    | CurrentPhysicalState::Nonspecific
            ) {
                return Err(E::ForeignState);
            }
            let slide_binding;
            let selected_binding = if state.current == CurrentPhysicalState::SlideGround {
                use crate::physical_player_assembly::remaining_states::{StateKind, StatePayload};
                let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                let Some(StatePayload::SlideGround(s)) = states
                    .states
                    .get(&StateKind::SlideGround)
                    .filter(|s| s.published)
                    .and_then(|s| s.value.as_ref())
                else {
                    return Err(E::Missing("SlideGround"));
                };
                slide_binding = s.base.physical.clone();
                &slide_binding
            } else if state.current == CurrentPhysicalState::RevertGround {
                use crate::physical_player_assembly::remaining_states::{StateKind, StatePayload};
                let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                let Some(StatePayload::RevertGround(s)) = states
                    .states
                    .get(&StateKind::RevertGround)
                    .filter(|s| s.published)
                    .and_then(|s| s.value.as_ref())
                else {
                    return Err(E::Missing("RevertGround"));
                };
                slide_binding = s.base.physical.clone();
                &slide_binding
            } else if let CurrentPhysicalState::Grind(grind) = state.current {
                // Grind UpdateSubstate82d9044c and Nonspecific82d9c094 call the
                // same UpdateGroundSkateboard.
                use crate::physical_player_assembly::remaining_states::StateKind;
                let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                slide_binding = states
                    .published_bindings(StateKind::grind(grind))
                    .cloned()
                    .ok_or(E::Missing("grind state"))?;
                &slide_binding
            } else if state.current == CurrentPhysicalState::Skitching {
                use crate::physical_player_assembly::remaining_states::StateKind;
                let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                slide_binding = states
                    .published_bindings(StateKind::Skitching)
                    .cloned()
                    .ok_or(E::Missing("Skitching state"))?;
                &slide_binding
            } else if state.current == CurrentPhysicalState::FollowPath {
                // FollowPath.Update82d868b8 calls the same UpdateGroundSkateboard.
                use crate::physical_player_assembly::remaining_states::StateKind;
                let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                slide_binding = states
                    .published_bindings(StateKind::FollowPath)
                    .cloned()
                    .ok_or(E::Missing("FollowPath state"))?;
                &slide_binding
            } else if state.current == CurrentPhysicalState::Nonspecific {
                use crate::physical_player_assembly::remaining_states::StateKind;
                let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                slide_binding = states
                    .published_bindings(StateKind::Nonspecific)
                    .cloned()
                    .ok_or(E::Missing("Nonspecific state"))?;
                &slide_binding
            } else {
                &state
                    .ground
                    .as_ref()
                    .ok_or(E::Missing("Ground state"))?
                    .bindings
            };
            if !selected_binding.board.same_as(board)
                || !selected_binding.skeleton.same_as(skeleton)
                || !selected_binding
                    .owner
                    .upgrade()
                    .is_some_and(|v| Arc::ptr_eq(&v, &owner))
            {
                return Err(E::ForeignState);
            }
        }
        let reckoning = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
        let processed = Arc::clone(reckoning.processed());
        let reckoning = reckoning
            .reckoning()
            .ok_or(E::Missing("constructed Reckoning"))?;
        let mut cvars = GroundSkateboardCvars::new(references, self.debug_overlay.clone());
        let mut services = super::services::GroundedWorldServices {
            cvars: &mut cvars,
            queries: self.world_line_queries.as_mut(),
            dmos: self.dmos.as_ref(),
            ground,
        };
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_update_paired_skeleton_grounded_live(
                &mut self.backend,
                &mut self.hooks,
                skeleton,
                board,
                &owner,
                &processed,
                reckoning,
                numeric,
                sin_cos,
                ortho,
                &mut services,
            )
            .map_err(E::Update)
    }
}

impl PhysicalAssemblyWorld {
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_apply_pair_offboard_board_animation(
        &mut self, owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle, skeleton: &SimulationSkaterAssemblyHandle,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        numeric: &mut HostReckoningNumeric, sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<(), crate::dynamic_assembly_session::skeleton_normal::general_owner::known_air::KnownAirSkeletonError<
    crate::reckoning_numeric::ReckoningHostNumericError, std::convert::Infallible, ()>>{
        use crate::dynamic_assembly_session::skeleton_normal::general_owner::known_air::KnownAirSkeletonError as E;
        let changer = owner.state_changer().lock().map_err(|_| E::Poisoned)?;
        let attributes = changer
            .attributes
            .as_ref()
            .ok_or(E::Missing("retained global Air attributes"))?;
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_apply_pair_offboard_board_animation(
                &mut self.backend,
                &mut self.hooks,
                skeleton,
                board,
                owner,
                processed,
                attributes,
                numeric,
                sin_cos,
            )
    }
}
