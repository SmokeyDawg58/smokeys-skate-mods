//! Actual active Ground jump inputs, board delta and existing selector consumers.
use crate::{
    debug_overlay::DebugOverlayError,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        board_construction::SimulationConstructedBoardError,
    },
    input_cvar_runtime::InputCvarError,
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{JumpInfo, StateKind, StatePayload},
    },
    physical_player_trajectory_owners::{
        RetainedTrajectoryLaunchInfo,
        skeleton_launch_info::{SkeletonLaunchInfoError, try_fill_launch_info},
    },
    reckoning_numeric::ReckoningHostNumericError,
    simulation_assembly_backend::constraint_session::{
        ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
        onboard_launch_runtime::OnboardLaunchRuntimeError,
        physical_board_forces::PhysicalPlayerBoardForceError,
        physical_selector_finish_runtime::SelectorCompletionError,
    },
    skeleton_processed_contact::{
        SkeletonProcessedContact,
        jump_events::{
            SkeletonJumpVaultError,
            ground_jump_attributes::{GroundJumpAttribute, GroundJumpGraph},
        },
    },
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
#[path = "../../board/runtime/skateboard_ground_jump.rs"]
pub(crate) mod kernel;
#[derive(Debug)]
pub(crate) enum SourceError {
    Missing(&'static str),
    Poisoned,
    Attribute(SkeletonJumpVaultError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Board(PhysicalPlayerBoardForceError),
}
impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "ground jump missing {v}"),
            Self::Poisoned => f.write_str("ground jump poisoned owner"),
            Self::Attribute(e) => write!(f, "ground jump attribute: {e}"),
            Self::Cvar(e) => write!(f, "ground jump cvar: {e}"),
            Self::Overlay(e) => write!(f, "ground jump overlay: {e:?}"),
            Self::Board(e) => write!(f, "ground jump board: {e}"),
        }
    }
}
struct Live<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    board: &'a SimulationSkaterAssemblyHandle,
    processed: Arc<Mutex<SkeletonProcessedContact>>,
}
impl kernel::Source for Live<'_> {
    type Error = SourceError;
    fn vector(&mut self, field: kernel::Vector) -> Result<V, Self::Error> {
        use kernel::Vector;
        if matches!(field, Vector::ReckoningUp) {
            return self
                .owner
                .reckoning()
                .lock()
                .map_err(|_| SourceError::Poisoned)?
                .reckoning()
                .map(crate::reckoning::Reckoning::up)
                .ok_or(SourceError::Missing("Reckoning up"));
        }
        let p = self.processed.lock().map_err(|_| SourceError::Poisoned)?;
        match field {
            Vector::Normal => p.toolkit_state().processed_jump_normal(),
            Vector::Prepared => p.prepare_output().velocity().map(|v| v.value),
            Vector::Velocity => p.prior_output().velocity().map(|v| v.value),
            Vector::Ground => p.ground_position_fields().ground_position,
            Vector::Com => p.prior_output().centre_of_mass().map(|v| v.value),
            Vector::BodyZ => p.toolkit_state().body_z(),
            Vector::ReckoningUp => unreachable!("handled above"),
        }
        .ok_or(SourceError::Missing("Processed jump vector"))
    }
    fn scalar(&mut self, field: kernel::Scalar) -> Result<f32, Self::Error> {
        use kernel::Scalar;
        let p = self.processed.lock().map_err(|_| SourceError::Poisoned)?;
        match field {
            Scalar::SurfaceSpeed => p.prior_output().surface_speed().map(|v| v.value),
            Scalar::Strength => p.jump_events().strength(),
            Scalar::Gravity => p.gravity().map(|v| v.value),
            Scalar::AdjustX => p.jump_events().adjustments().0,
            Scalar::AdjustZ => p.jump_events().adjustments().1,
        }
        .ok_or(SourceError::Missing("Processed jump scalar"))
    }
    fn attribute(&mut self, field: GroundJumpAttribute) -> Result<f32, Self::Error> {
        #[cfg(feature = "skate3-runtime")]
        if let Some(tuning) = &*self.owner.skate3_tuning.lock().map_err(|_| SourceError::Poisoned)? {
            match field { GroundJumpAttribute::MinimumHeight => return Ok(tuning.jump_minimum),
                GroundJumpAttribute::MaximumHeight => return Ok(tuning.jump_maximum), _ => {} }
        }
        self.owner
            .jump_attributes()
            .lock()
            .map_err(|_| SourceError::Poisoned)?
            .try_ground_jump_scalar(field)
            .map_err(SourceError::Attribute)
    }
    fn graph(&mut self, field: GroundJumpGraph, input: f32) -> Result<f32, Self::Error> {
        self.owner
            .jump_attributes()
            .lock()
            .map_err(|_| SourceError::Poisoned)?
            .try_ground_jump_graph(field, input)
            .map_err(SourceError::Attribute)
    }
    fn vertical_response_enabled(&mut self) -> Result<bool, Self::Error> {
        self.owner
            .jump_attributes()
            .lock()
            .map_err(|_| SourceError::Poisoned)?
            .try_ground_jump_vertical_response_enabled()
            .map_err(SourceError::Attribute)
    }
    fn publish_delta(&mut self, value: V) -> Result<(), Self::Error> {
        self.world
            .try_with_board_force_runtime(self.board, |r| r.jump_delta = value)
            .map_err(SourceError::Board)
    }
    fn debug(&mut self) -> Result<(), Self::Error> {
        if self
            .owner
            .inputs()
            .cvars
            .try_draw_jump_debug_info()
            .map_err(SourceError::Cvar)?
        {
            self.world
                .debug_overlay
                .require()
                .map_err(SourceError::Overlay)?;
        }
        Ok(())
    }
}
pub(crate) fn calculate(
    world: &mut PhysicalAssemblyWorld,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    maximum: f32,
    numeric: &mut HostGroundPreparationNumeric,
) -> Result<JumpInfo, kernel::Error<SourceError, ReckoningHostNumericError>> {
    let processed = Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| kernel::Error::Source(SourceError::Poisoned))?
            .processed(),
    );
    let velocity = kernel::calculate(
        maximum,
        &mut Live {
            world,
            owner,
            board,
            processed,
        },
        &mut numeric.reckoning,
    )?;
    Ok(JumpInfo {
        velocity,
        com_height: 0.0,
        do_jump: true,
    })
}
#[derive(Debug)]
pub(crate) enum LaunchError {
    Missing(&'static str),
    Poisoned,
    Overlay(DebugOverlayError),
    Board(PhysicalPlayerBoardForceError),
    Pool(SimulationSkaterPoolError),
    Attributes(SimulationConstructedBoardError),
    Info(SkeletonLaunchInfoError),
    Launch(OnboardLaunchRuntimeError<ReckoningHostNumericError, Infallible>),
    Selector(SelectorCompletionError<ReckoningHostNumericError, Infallible>),
}
impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "ground jump launch missing {v}"),
            Self::Poisoned => f.write_str("ground jump launch poisoned"),
            Self::Overlay(e) => write!(f, "ground launch overlay: {e:?}"),
            Self::Board(e) => write!(f, "{e}"),
            Self::Pool(e) => write!(f, "{e:?}"),
            Self::Attributes(e) => write!(f, "{e:?}"),
            Self::Info(e) => write!(f, "{e}"),
            Self::Launch(e) => write!(f, "{e}"),
            Self::Selector(e) => write!(f, "{e}"),
        }
    }
}
#[expect(
    clippy::too_many_arguments,
    reason = "same pair and source launch collaborators"
)]
pub(crate) fn launch(
    world: &mut PhysicalAssemblyWorld,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    velocity: V,
    ground: &StaticGroundPool,
    numeric: &mut HostGroundPreparationNumeric,
    policy: GroundPreparationGrindPolicy,
) -> Result<(), LaunchError> {
    launch_velocity(
        world, owner, board, skeleton, velocity, ground, numeric, policy, true,
    )
}

#[expect(clippy::too_many_arguments)]
pub(crate) fn launch_velocity(
    world: &mut PhysicalAssemblyWorld,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    velocity: V,
    ground: &StaticGroundPool,
    numeric: &mut HostGroundPreparationNumeric,
    policy: GroundPreparationGrindPolicy,
    ground_animation: bool,
) -> Result<(), LaunchError> {
    launch_inner(
        world,
        owner,
        board,
        skeleton,
        velocity,
        ground,
        numeric,
        policy,
        if ground_animation {
            LaunchKind::GroundAnimation
        } else {
            LaunchKind::SlideGround
        },
    )
}

#[derive(Clone, Copy)]
enum LaunchKind {
    GroundAnimation,
    SlideGround,
    GroundWallride,
}

/// PhysicsGround.UpdateSkateboard82d9a444..a4d4. This uses the already-computed
/// WallRide jump velocity, not the animation-driven ollie velocity calculator.
#[expect(clippy::too_many_arguments)]
pub(crate) fn launch_ground_wallride(
    world: &mut PhysicalAssemblyWorld,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    velocity: V,
    ground: &StaticGroundPool,
    numeric: &mut HostGroundPreparationNumeric,
    policy: GroundPreparationGrindPolicy,
) -> Result<(), LaunchError> {
    launch_inner(
        world,
        owner,
        board,
        skeleton,
        velocity,
        ground,
        numeric,
        policy,
        LaunchKind::GroundWallride,
    )?;
    // a4a8..a4d0: the exit flag is committed only after selector Update.
    world
        .debug_overlay
        .require()
        .map_err(LaunchError::Overlay)?;
    owner
        .physics_state()
        .lock()
        .map_err(|_| LaunchError::Poisoned)?
        .ground
        .as_mut()
        .ok_or(LaunchError::Missing("Ground"))?
        .fields
        .exiting_wallride = true;
    Ok(())
}

#[expect(clippy::too_many_arguments)]
fn launch_inner(
    world: &mut PhysicalAssemblyWorld,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    velocity: V,
    ground: &StaticGroundPool,
    numeric: &mut HostGroundPreparationNumeric,
    policy: GroundPreparationGrindPolicy,
    kind: LaunchKind,
) -> Result<(), LaunchError> {
    use LaunchError as E;
    let ground_animation = matches!(kind, LaunchKind::GroundAnimation);
    world
        .try_set_board_velocity(board, &velocity)
        .map_err(E::Board)?;
    if matches!(kind, LaunchKind::GroundWallride) {
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .publish_ground_launch_velocity(velocity); //82d9a45c
    }
    if ground_animation {
        let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
        let Some(StatePayload::GroundAnimation(state)) = states
            .states
            .get_mut(&StateKind::GroundAnimation)
            .and_then(|s| s.value.as_mut())
        else {
            return Err(E::Missing("GroundAnimation"));
        };
        state.jumped_this_frame = Some(true);
    }
    world
        .try_set_board_linear_drag(board, 0.0)
        .map_err(E::Board)?;
    let mut launch = RetainedTrajectoryLaunchInfo::constructed();
    let pool = world.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
    let trajectory = pool
        .try_with_ground_control_board(&world.backend, board, |r, _| r.wallride_trajectory.clone())
        .map_err(E::Attributes)?;
    let constructor = &pool
        .skeleton_construction(skeleton)
        .map_err(E::Pool)?
        .ok_or(E::Missing("Skeleton constructor"))?
        .constructor;
    try_fill_launch_info(constructor, owner, &trajectory, &mut launch).map_err(E::Info)?;
    launch.player_jumped = true;
    launch.start_velocity = velocity;
    if matches!(kind, LaunchKind::GroundWallride) {
        launch.velocity_before_jump = velocity; //82d9a498, distinct from ollie path
    }
    world
        .try_launch_pair_onboard(
            board,
            skeleton,
            &launch,
            ground,
            &mut numeric.reckoning,
            &mut numeric.sin_cos,
        )
        .map_err(E::Launch)?;
    let _ready = world
        .try_complete_pair_selector(
            board,
            skeleton,
            ground,
            &mut numeric.reckoning,
            &mut numeric.sin_cos,
            policy,
        )
        .map_err(E::Selector)?;
    if !ground_animation {
        return Ok(());
    }
    let velocity = owner
        .trajectories()
        .lock()
        .map_err(|_| E::Poisoned)?
        .onboard
        .as_ref()
        .ok_or(E::Missing("onboard selector"))?
        .launch
        .start_velocity;
    let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    let Some(StatePayload::GroundAnimation(state)) = states
        .states
        .get_mut(&StateKind::GroundAnimation)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(E::Missing("GroundAnimation"));
    };
    state.jump_vel = Some(velocity);
    Ok(())
}
