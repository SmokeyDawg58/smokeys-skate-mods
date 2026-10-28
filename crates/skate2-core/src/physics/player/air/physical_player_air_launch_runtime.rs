//! Same retained pair `PhysicsAir::LaunchTrajectorySelector`82d99010.
use super::onboard_launch_runtime::OnboardLaunchRuntimeError;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
    },
    input_cvar_runtime::InputCvarError,
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_trajectory_owners::{
        RetainedTrajectoryLaunchInfo,
        skeleton_launch_info::{SkeletonLaunchInfoError, try_fill_launch_info},
    },
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum AirLaunchRuntimeError<N, R> {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Pool(SimulationSkaterPoolError),
    Info(SkeletonLaunchInfoError),
    Cvar(InputCvarError),
    Jump(super::air_jump_velocity::AirJumpVelocityError<N>),
    ComLaunch(super::air_com_launch::AirComLaunchError),
    Launch(OnboardLaunchRuntimeError<N, R>),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display for AirLaunchRuntimeError<N, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("AirLaunchRuntimeError: Poisoned"),
            Self::ForeignState => f.write_str("AirLaunchRuntimeError: ForeignState"),
            Self::Missing(error) => write!(f, "AirLaunchRuntimeError Missing: {error:?}"),
            Self::Pair(error) => write!(f, "AirLaunchRuntimeError Pair: {error:?}"),
            Self::Board(error) => write!(f, "AirLaunchRuntimeError Board: {error:?}"),
            Self::Pool(error) => write!(f, "AirLaunchRuntimeError Pool: {error:?}"),
            Self::Info(error) => write!(f, "AirLaunchRuntimeError Info: {error:?}"),
            Self::Cvar(error) => write!(f, "AirLaunchRuntimeError Cvar: {error:?}"),
            Self::Jump(error) => write!(f, "AirLaunchRuntimeError Jump: {error:?}"),
            Self::ComLaunch(error) => write!(f, "AirLaunchRuntimeError ComLaunch: {error:?}"),
            Self::Launch(error) => write!(f, "AirLaunchRuntimeError Launch: {error:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_launch_pair_air_trajectory<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), AirLaunchRuntimeError<N::Error, R::Error>> {
        use AirLaunchRuntimeError as E;
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if state.current != CurrentPhysicalState::Air {
                return Err(E::ForeignState);
            }
            let air = state.air.as_ref().ok_or(E::Missing("Air state"))?;
            if !air.bindings.board.same_as(board)
                || !air.bindings.skeleton.same_as(skeleton)
                || !air
                    .bindings
                    .owner
                    .upgrade()
                    .is_some_and(|actual| Arc::ptr_eq(&actual, &owner))
            {
                return Err(E::ForeignState);
            }
        }
        // Clone the existing immutable global binding, not a new selection.
        let trajectory = pool
            .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                runtime.wallride_trajectory.clone()
            })
            .map_err(E::Board)?;
        let constructor = &pool
            .skeleton_construction(skeleton)
            .map_err(E::Pool)?
            .ok_or(E::Missing("Skeleton constructor"))?
            .constructor;
        let mut launch = RetainedTrajectoryLaunchInfo::constructed();
        try_fill_launch_info(constructor, &owner, &trajectory, &mut launch).map_err(E::Info)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let frames = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .frames_since_jump_correct
            .ok_or(E::Missing("frames since jump correction"))?;
        let limit = owner
            .inputs()
            .cvars
            .try_physics_air_jump_fix_frames()
            .map_err(E::Cvar)?;
        if frames < limit {
            launch.start_velocity = super::air_jump_velocity::try_calculate(
                &owner,
                frames,
                &self.debug_overlay,
                numeric,
            )
            .map_err(E::Jump)?;
        } else {
            // Native early jump branch never samples this field.
            let use_com = owner
                .physics_state()
                .lock()
                .map_err(|_| E::Poisoned)?
                .air
                .as_ref()
                .ok_or(E::Missing("Air state"))?
                .fields
                .use_com_velocity;
            if use_com {
                super::air_com_launch::try_fill_com_launch_info(
                    constructor,
                    &owner,
                    &trajectory,
                    &mut launch,
                )
                .map_err(E::ComLaunch)?;
            }
        }
        launch.player_jumped = false;
        self.try_launch_pair_onboard(board, skeleton, &launch, ground, numeric, rounding)
            .map_err(E::Launch)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn jump_fix_frames_follow_actual_link_and_reload_without_gui_clamping() {
        use crate::input_cvar_runtime::{InputCvarManager, InputCvarReferences};
        let references = InputCvarReferences::new_static_initialized();
        assert!(references.try_physics_air_jump_fix_frames().is_err());
        let mut manager = InputCvarManager::new();
        manager.try_link(&references).unwrap();
        assert_eq!(references.try_physics_air_jump_fix_frames().unwrap(), 12);
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkPhysicsAirJumpFixFrames=-2\r\n"))
            .unwrap();
        assert_eq!(references.try_physics_air_jump_fix_frames().unwrap(), -2);
    }
}
