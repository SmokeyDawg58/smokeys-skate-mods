//! Public `TrajectorySelector::Launch`82dc4098 over actual retained collaborators.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_trajectory_owners::{
        RetainedTrajectoryLaunchInfo,
        adjust_start::{AdjustLiveError, AdjustStartError},
        launch_continue::{LaunchContinueError, try_continue_private_launch},
        launch_prefix::OnboardLaunchPrefix,
        launch_prepare::{PrepareError, PrepareLiveError},
    },
    reckoning_attributes::ReckoningAttributeError,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum OnboardLaunchRuntimeError<N, R> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Adjust(AdjustStartError<AdjustLiveError<ReckoningAttributeError>, N>),
    Prepare(PrepareError<PrepareLiveError<ReckoningAttributeError>, N, R>),
    Continue(LaunchContinueError<ReckoningAttributeError, N>),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display for OnboardLaunchRuntimeError<N, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("OnboardLaunchRuntimeError: Poisoned"),
            Self::ForeignOwner => f.write_str("OnboardLaunchRuntimeError: ForeignOwner"),
            Self::Missing(error) => write!(f, "OnboardLaunchRuntimeError Missing: {error:?}"),
            Self::Pair(error) => write!(f, "OnboardLaunchRuntimeError Pair: {error:?}"),
            Self::Board(error) => write!(f, "OnboardLaunchRuntimeError Board: {error:?}"),
            Self::Adjust(error) => write!(f, "OnboardLaunchRuntimeError Adjust: {error:?}"),
            Self::Prepare(error) => write!(f, "OnboardLaunchRuntimeError Prepare: {error:?}"),
            Self::Continue(error) => write!(f, "OnboardLaunchRuntimeError Continue: {error:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    /// Ordinary public Launch composes reviewed prefix, adjustment, private
    /// preparation and actual submission. Predictor harvesting remains scheduled.
    pub(crate) fn try_launch_pair_onboard<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        launch: &RetainedTrajectoryLaunchInfo,
        ground: &StaticGroundPool,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), OnboardLaunchRuntimeError<N::Error, R::Error>> {
        use OnboardLaunchRuntimeError as E;
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
        {
            let mut trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .onboard
                .as_mut()
                .ok_or(E::Missing("onboard selector"))?;
            if !selector.bindings.matches(&owner, &processed) {
                return Err(E::ForeignOwner);
            }
            let prefix = selector.begin_launch(launch);
            if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
                eprintln!(
                    "known air: selector launch {prefix:?} player_jumped={} start_velocity={:?} trajectories={}",
                    launch.player_jumped, launch.start_velocity.xyz, launch.num_trajectories
                );
            }
            if prefix == OnboardLaunchPrefix::AlreadyStartedReturn {
                return Ok(());
            }
        }
        // Clone only immutable Vault bindings; no Reckoning state is copied and
        // its mutex is released before helpers reacquire the Processed binding.
        let mut attributes = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .reckoning()
            .ok_or(E::Missing("Reckoning"))?
            .cloned_attribute_bindings();
        pool.try_with_ground_control_board(&self.backend, board, |runtime, _| {
            let trajectory = &runtime.wallride_trajectory;
            {
                let mut trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
                let selector = trajectories
                    .onboard
                    .as_mut()
                    .ok_or(E::Missing("onboard selector"))?;
                selector
                    .try_adjust_starting_velocity(
                        &owner,
                        &mut attributes,
                        trajectory,
                        &self.debug_overlay,
                        numeric,
                    )
                    .map_err(E::Adjust)?;
                selector
                    .try_prepare_private_launch(
                        &owner,
                        &mut attributes,
                        trajectory,
                        numeric,
                        rounding,
                    )
                    .map_err(E::Prepare)?;
                if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
                    eprintln!(
                        "known air: selector start_position={:?} com={:?} adjusted_velocity={:?}",
                        selector.start_trajectory_position.xyz,
                        selector.launch.com_position.xyz,
                        selector.launch.start_velocity.xyz
                    );
                }
            }
            // No selector/Reckoning lock survives submission into the same future.
            try_continue_private_launch(
                &owner,
                &mut attributes,
                trajectory,
                &owner.inputs().cvars,
                &self.debug_overlay,
                self.predictor
                    .value
                    .as_ref()
                    .ok_or(E::Missing("World predictor"))?,
                self.world_line_queries
                    .as_mut()
                    .ok_or(E::Missing("World queries"))?,
                ground,
                self.dmos.as_ref().ok_or(E::Missing("DMO collection"))?,
                numeric,
            )
            .map_err(E::Continue)
        })
        .map_err(E::Board)?
    }
}
