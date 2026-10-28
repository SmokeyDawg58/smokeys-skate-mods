//! Selected ordinary-Ground `Update1`/`Update2`/`Update3` composition, before the solver.
//! This owns no startup, query-manager construction, or fixture policy.
use crate::{
    actor_construction::{
        ActorConstruction,
        anim_out_phys_in::publication::AnimationPhysicsPublicationError,
        breadcrumbs::{
            ActorAiRespawnQuery, ActorAppropriateTeleportError, LiveBreadCrumbSelectionError,
            LiveBreadCrumbSelectionSource,
        },
        physical_mount::ActorPhysicalMountError,
    },
    collision_line_numeric::HostCollisionLineNumeric,
    dynamic_assembly_session::skater_pool::{
        ProcessDataPrefixError, SimulationSkaterAssemblyHandle,
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld,
        remaining_states::{StateKind, StatePayload, TeleportingUpdate, TeleportingUpdateError},
    },
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_force_runtime::fake_torque::HostFakeTorqueNumeric,
    skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
    skeleton_processed_contact::prior_output::{body, runtime, tail, wheel_timers},
    static_ground_pool::StaticGroundPool,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

/// Explicit first-test scope, not a fabricated `GrindManager` query result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroundPreparationGrindPolicy {
    ParkedByUser,
    RunRecoveredPrefix,
}

/// The existing corpus's explicit host numerical policies, reusable per tick.
/// VMX estimate/reduction and host cosine accuracy limitations remain unchanged.
/// No duration or tuning values are retained here.
pub(crate) struct HostGroundPreparationNumeric {
    pub(crate) reckoning: HostReckoningNumeric,
    pub(crate) sin_cos: HostReckoningSinCosNumeric,
    pub(crate) spawn_math: HostSkateboardSpawnMathNumeric,
    pub(crate) fake_torque: HostFakeTorqueNumeric,
    pub(crate) collision_line: HostCollisionLineNumeric,
}
impl Default for HostGroundPreparationNumeric {
    fn default() -> Self {
        Self {
            reckoning: HostReckoningNumeric,
            sin_cos: HostReckoningSinCosNumeric,
            spawn_math: HostSkateboardSpawnMathNumeric,
            fake_torque: HostFakeTorqueNumeric,
            collision_line: HostCollisionLineNumeric,
        }
    }
}

type NumericError = ReckoningHostNumericError;

#[derive(Debug)]
pub(crate) enum GroundPreparationError {
    Actor(ActorPhysicalMountError),
    Input(AnimationPhysicsPublicationError),
    Poisoned(&'static str),
    Begin(super::physical_update1_prefix::Update1PrefixError<NumericError>),
    Controls(super::physical_input_controls::ProcessInputControlsError),
    PriorInput(runtime::PriorInputRuntimeError),
    PriorTail(tail::PriorOutputTailError),
    PriorBody(body::PriorOutputBodyError),
    PriorWheels(wheel_timers::PriorOutputWheelTimersError),
    GroundInput(super::physical_process_input_ground::ProcessInputGroundError<NumericError>),
    Toolkit(super::physical_toolkit::PhysicalPlayerToolkitError<NumericError>),
    LineSurfaces(super::physical_skeleton_lines::SkeletonLineTestsError<Infallible>),
    Skeleton(ProcessDataPrefixError<NumericError>),
    AfterSkeleton(
        super::physical_process_input_post_skeleton::PostSkeletonInputError<NumericError>,
    ),
    Grind(super::physical_grind::PhysicalGrindPrefixError),
    GrindState(super::grind_states::GrindStateError),
    JumpTiming(super::physical_process_input_jump_publication::JumpPublicationError),
    Selector(
        super::physical_selector_finish_runtime::SelectorCompletionError<
            NumericError,
            std::convert::Infallible,
        >,
    ),
    Heading(super::physical_heading::HeadingError<NumericError>),
    Permissions(super::physical_grind_permission::PermissionSessionError),
    FinishInput(super::physical_input_finish::InputFinishError),
    SystemLogic(super::physical_state::ground_to_air::GroundToAirError<NumericError>),
    Prediction(super::physical_update3_prefix::Update3PrefixError),
    Update0(super::physical_update0_collision_status::Update0Error),
    GrabQueries(crate::physical_player_grind_manager::view::GrindError),
    GrabMessages(crate::physical_player_grind_manager::view::GrindError),
    OffBoardAir(crate::physical_player_assembly::offboard_air_state::AirStateError),
    OffBoardJump(crate::physical_player_assembly::offboard_jump_state::JumpStateError),
    LandingOnDeck(crate::physical_player_assembly::landing_on_deck_state::LandingStateError),
    OffBoardLocomotion(
        crate::physical_player_assembly::offboard_locomotion_state::LocomotionStateError,
    ),
    OffBoardPushing(crate::physical_player_grind_manager::view::GrindError),
    HandPlant(crate::physical_player_grind_manager::view::GrindError),
    Skitching(crate::physical_player_grind_manager::view::GrindError),
    FollowPath(crate::physical_player_grind_manager::view::GrindError),
    OffBoardFeetIk(
        crate::physical_player_assembly::offboard_feet_ik_session::OffboardGroundFeetIkError,
    ),
    TeleportingMissingState,
    TeleportingForeignState,
    TeleportingUpdate(TeleportingUpdateError),
    TeleportingBreadcrumbOwner(crate::actor_construction::ActorConstructionError),
    TeleportingBreadcrumbLock,
    TeleportingActor(ActorAppropriateTeleportError<LiveBreadCrumbSelectionError>),
    Reckoning(super::physical_ground_update_prefix::GroundUpdatePrefixError<NumericError>),
    AirPrefix(super::air_update_prefix::AirUpdatePrefixError<NumericError, Infallible>),
    AirFinish(super::air_update_finish::AirUpdateFinishError),
    KnownAir(crate::physical_player_assembly::known_air_state::KnownAirStateError),
    Wipeout(super::physical_grounded::wipeout::WipeoutUpdateError),
    Handplants(super::physical_ground_handplants::GroundHandplantsError),
    Skateboard(super::physical_grounded::live::GroundSkateboardError<NumericError, Infallible>),
    Pumping(super::physical_ground_pumping::GroundPumpingError<NumericError>),
    Control(super::ground_control_complete::GroundControlCompleteError<NumericError, Infallible>),
    Wallride(super::ground_wallride::GroundWallRideError<NumericError>),
    FakeTorque(super::ground_fake_torque::GroundFakeTorqueError<Infallible>),
    Collision(super::ground_collision::GroundCollisionError<NumericError>),
    GroundLaunch(super::ground_jump_runtime::LaunchError),
    GroundAnimation(super::ground_animation_update::GroundAnimationUpdateError),
    Slide(super::slide_ground::SlideError),
    Revert(super::revert_ground::RevertError),
    Boneless(super::boneless_state::BonelessError),
    FootPlant(super::footplant_air::FootPlantUpdateError),
    OrdinaryForces(super::ground_ordinary_forces::GroundOrdinaryForceError<NumericError>),
    Manual(super::ground_manual_nudge::GroundManualNudgeError<NumericError, Infallible>),
    Queue(super::ground_queue_torque_prefix::GroundQueueTorqueError<NumericError>),
    HangPinning(
        super::ground_hang_pinning::HangPinError<
            super::ground_hang_pinning_session::HangPinRuntimeError,
            NumericError,
        >,
    ),
    ForcePrediction(
        super::ground_force_prediction::GroundForcePredictionError<
            super::ground_force_prediction_session::ForcePredictionRuntimeError,
            NumericError,
        >,
    ),
    FinishGround(
        super::ground_finish::GroundFinishError<
            super::ground_finish_session::GroundFinishRuntimeError,
        >,
    ),
    Controller(super::update3_controller::Update3ControllerError),
    PrePhysics(super::update3_prephysics_tail::Update3PrePhysicsTailError<NumericError>),
}
impl std::fmt::Display for GroundPreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "selected Ground pre-solve Actor: {error:?}"),
            Self::Input(error) => write!(f, "selected Ground pre-solve Input: {error:?}"),
            Self::Poisoned(error) => write!(f, "selected Ground pre-solve Poisoned: {error:?}"),
            Self::Begin(error) => write!(f, "selected Ground pre-solve Begin: {error:?}"),
            Self::Controls(error) => write!(f, "selected Ground pre-solve Controls: {error:?}"),
            Self::PriorInput(error) => write!(f, "selected Ground pre-solve PriorInput: {error:?}"),
            Self::PriorTail(error) => write!(f, "selected Ground pre-solve PriorTail: {error:?}"),
            Self::PriorBody(error) => write!(f, "selected Ground pre-solve PriorBody: {error:?}"),
            Self::PriorWheels(error) => {
                write!(f, "selected Ground pre-solve PriorWheels: {error:?}")
            }
            Self::GroundInput(error) => {
                write!(f, "selected Ground pre-solve GroundInput: {error:?}")
            }
            Self::Toolkit(error) => write!(f, "selected Ground pre-solve Toolkit: {error:?}"),
            Self::LineSurfaces(error) => {
                write!(f, "selected Ground pre-solve LineSurfaces: {error:?}")
            }
            Self::Skeleton(error) => write!(f, "selected Ground pre-solve Skeleton: {error:?}"),
            Self::AfterSkeleton(error) => {
                write!(f, "selected Ground pre-solve AfterSkeleton: {error:?}")
            }
            Self::HandPlant(error) => write!(f, "state600 Update: {error}"),
            Self::Grind(error) => write!(f, "selected Ground pre-solve Grind: {error:?}"),
            Self::GrindState(error) => write!(f, "state400..404/701 Update: {error}"),
            Self::JumpTiming(error) => write!(f, "selected Ground pre-solve JumpTiming: {error:?}"),
            Self::Selector(error) => write!(f, "selected Ground pre-solve Selector: {error:?}"),
            Self::Heading(error) => write!(f, "selected Ground pre-solve Heading: {error:?}"),
            Self::Permissions(error) => {
                write!(f, "selected Ground pre-solve Permissions: {error:?}")
            }
            Self::FinishInput(error) => {
                write!(f, "selected Ground pre-solve FinishInput: {error:?}")
            }
            Self::SystemLogic(error) => {
                write!(f, "selected Ground pre-solve SystemLogic: {error:?}")
            }
            Self::Prediction(error) => write!(f, "selected Ground pre-solve Prediction: {error:?}"),
            Self::Update0(error) => write!(f, "selected Ground pre-solve Update0: {error}"),
            Self::GrabQueries(error) => write!(f, "selected grab query update: {error}"),
            Self::GrabMessages(error) => write!(f, "selected grab pre-simulation message: {error}"),
            Self::OffBoardAir(error) => write!(f, "state502 Update: {error}"),
            Self::OffBoardJump(error) => write!(f, "state503 Update: {error}"),
            Self::OffBoardLocomotion(error) => write!(f, "state504 Update: {error}"),
            Self::OffBoardPushing(error) => write!(f, "state501 Update: {error}"),
            Self::Skitching(error) => write!(f, "state104 Update: {error}"),
            Self::FollowPath(error) => write!(f, "state105 Update: {error}"),
            Self::LandingOnDeck(error) => write!(f, "state505 Update: {error}"),
            Self::OffBoardFeetIk(error) => write!(f, "state500 Update: {error}"),
            Self::TeleportingMissingState => {
                f.write_str("state702 Update: missing Teleporting state")
            }
            Self::TeleportingForeignState => {
                f.write_str("state702 Update: foreign Teleporting owner")
            }
            Self::TeleportingUpdate(error) => write!(f, "state702 Update: {error:?}"),
            Self::TeleportingBreadcrumbOwner(error) => {
                write!(f, "state702 Actor breadcrumb owner: {error:?}")
            }
            Self::TeleportingBreadcrumbLock => {
                f.write_str("state702 Actor breadcrumb lock poisoned")
            }
            Self::TeleportingActor(error) => write!(f, "state702 Actor respawn: {error:?}"),
            Self::AirPrefix(error) => write!(f, "selected Air pre-solve prefix: {error:?}"),
            Self::Wipeout(error) => write!(f, "selected Wipeout pre-solve: {error}"),
            Self::KnownAir(error) => write!(f, "KnownAir pre-solve: {error}"),
            Self::AirFinish(error) => write!(f, "selected Air pre-solve finish: {error:?}"),
            Self::Reckoning(error) => write!(f, "selected Ground pre-solve Reckoning: {error:?}"),
            Self::Handplants(error) => write!(f, "selected Ground pre-solve Handplants: {error:?}"),
            Self::Skateboard(error) => write!(f, "selected Ground pre-solve Skateboard: {error:?}"),
            Self::Pumping(error) => write!(f, "selected Ground pre-solve Pumping: {error:?}"),
            Self::Control(error) => write!(f, "selected Ground pre-solve Control: {error:?}"),
            Self::Wallride(error) => write!(f, "selected Ground pre-solve Wallride: {error:?}"),
            Self::FakeTorque(error) => write!(f, "selected Ground pre-solve FakeTorque: {error:?}"),
            Self::Collision(error) => write!(f, "selected Ground pre-solve Collision: {error:?}"),
            Self::GroundLaunch(error) => write!(f, "selected Ground launch: {error}"),
            Self::OrdinaryForces(error) => {
                write!(f, "selected Ground pre-solve OrdinaryForces: {error:?}")
            }
            Self::Manual(error) => write!(f, "selected Ground pre-solve Manual: {error:?}"),
            Self::Queue(error) => write!(f, "selected Ground pre-solve Queue: {error:?}"),
            Self::HangPinning(error) => {
                write!(f, "selected Ground pre-solve HangPinning: {error:?}")
            }
            Self::ForcePrediction(error) => {
                write!(f, "selected Ground pre-solve ForcePrediction: {error:?}")
            }
            Self::FinishGround(error) => {
                write!(f, "selected Ground pre-solve FinishGround: {error:?}")
            }
            Self::Controller(error) => write!(f, "selected Ground pre-solve Controller: {error:?}"),
            Self::PrePhysics(error) => write!(f, "selected Ground pre-solve PrePhysics: {error:?}"),
            Self::Revert(error) => write!(f, "selected revert: {error}"),
            Self::Slide(error) => write!(f, "selected slide: {error}"),
            Self::GroundAnimation(error) => write!(f, "state103 Update: {error}"),
            Self::Boneless(error) => write!(f, "state602 Update: {error}"),
            Self::FootPlant(error) => write!(f, "state601 Update: {error}"),
        }
    }
}

/// Successful preparation only. The caller must still run the actual solver and
/// post-solve path; it must not begin another `Update1` against these open queries.
pub(crate) struct GroundPreSolveComplete {
    pub(crate) update1: super::physical_update1_prefix::Update1AfterTeleportCheck,
    pub(crate) ground_reckoning: super::update5_managers::Update5ReckoningRead,
    pub(crate) update3: super::update3_prephysics_tail::OrdinaryUpdate3Complete,
    pub(crate) grind_policy: GroundPreparationGrindPolicy,
}

impl PhysicalAssemblyWorld {
    /// Shared Actor::TeleportSomewhereAppropriate dispatch for a trapped skater
    /// and the existing Teleporting state's recovery; same breadcrumb owner.
    fn try_actor_appropriate_teleport(
        &mut self,
        actor: &ActorConstruction,
        world_owner: &Arc<Mutex<PhysicalAssemblyWorld>>,
        ground: &StaticGroundPool,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut HostCollisionLineNumeric,
        ai_respawn: &mut ActorAiRespawnQuery<'_>,
    ) -> Result<(), GroundPreparationError> {
        use GroundPreparationError as E;
        let breadcrumbs = actor
            .retained_breadcrumbs()
            .map_err(E::TeleportingBreadcrumbOwner)?
            .ok_or(E::TeleportingMissingState)?;
        let mut source = {
            let manager = breadcrumbs
                .lock()
                .map_err(|_| E::TeleportingBreadcrumbLock)?;
            LiveBreadCrumbSelectionSource::new(
                &manager,
                actor,
                world_owner,
                self,
                ground,
                cvars,
                numeric,
            )
        };
        actor
            .try_teleport_somewhere_appropriate(&breadcrumbs, &mut source, ai_respawn)
            .map_err(E::TeleportingActor)?;
        Ok(())
    }

    /// Existing native chain from `Update1_ProcessInput82e01f98` to ordinary
    /// `Update3` suffix82dfe834. `ProcessAnimOut` is already inside Begin `Update1`.
    /// Call after same-Actor input/animation publication. Ground, query managers,
    /// grind residency and all world/Actor services must already be retained.
    /// Errors preserve earlier writes and query leases; restarting is not rollback.
    /// One skater's whole chain; several skaters run each part over all of them.
    #[cfg_attr(
        test,
        expect(
            clippy::too_many_arguments,
            reason = "test-only controlled transition policy"
        )
    )]
    pub(crate) fn try_prepare_ground_pre_solve(
        &mut self,
        world_owner: &Arc<Mutex<PhysicalAssemblyWorld>>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        actor: &ActorConstruction,
        ground: &StaticGroundPool,
        grind_policy: GroundPreparationGrindPolicy,
        numeric: &mut HostGroundPreparationNumeric,
        ai_respawn: &mut ActorAiRespawnQuery<'_>,
        #[cfg(test)] requested_transition: &mut Option<
            super::physical_state::ground_to_air::GroundDestination,
        >,
    ) -> Result<GroundPreSolveComplete, GroundPreparationError> {
        self.try_prepare_ground_pre_solve_world(actor, numeric)?;
        self.try_prepare_ground_pre_solve_update0(world_owner, board, skeleton, actor, ground, numeric, ai_respawn)?;
        self.try_prepare_ground_pre_solve_pair(
            world_owner,
            board,
            skeleton,
            actor,
            ground,
            grind_policy,
            numeric,
            ai_respawn,
            #[cfg(test)]
            requested_transition,
        )
    }

    /// `World::Update1` 0x827ae008 before its physicals: the grab spline query
    /// manager's `Update` runs once for the whole world.
    pub(crate) fn try_prepare_ground_pre_solve_world(
        &mut self,
        actor: &ActorConstruction,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<(), GroundPreparationError> {
        use GroundPreparationError as E;
        // Clone actual shared cvar references, never sampled tuning or defaults.
        let physical = actor.cached_physical_allocation().map_err(E::Actor)?;
        let cvars = physical.owner().inputs().cvars.clone();
        self.try_update_grab_queries(
            &cvars,
            &mut crate::physical_player_grind_manager::math::GrindNum {
                n: &mut numeric.reckoning,
                s: &mut numeric.sin_cos,
            },
        )
        .map_err(E::GrabQueries)
    }

    /// `World::Update0PhysicalGatherPrimitives`: one physical's
    /// `Update0_ProcessCollection` 0x82e01f18; every physical runs it before any
    /// `Update1` job.
    pub(crate) fn try_prepare_ground_pre_solve_update0(
        &mut self,
        world_owner: &Arc<Mutex<PhysicalAssemblyWorld>>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        actor: &ActorConstruction,
        ground: &StaticGroundPool,
        numeric: &mut HostGroundPreparationNumeric,
        ai_respawn: &mut ActorAiRespawnQuery<'_>,
    ) -> Result<(), GroundPreparationError> {
        use GroundPreparationError as E;
        let physical = actor.cached_physical_allocation().map_err(E::Actor)?;
        let cvars = physical.owner().inputs().cvars.clone();
        if self
            .try_update0_pair_process_collection(board, skeleton, &cvars)
            .map_err(E::Update0)?
        {
            self.try_actor_appropriate_teleport(
                actor,
                world_owner,
                ground,
                &cvars,
                &mut numeric.collision_line,
                ai_respawn,
            )?;
        }
        Ok(())
    }

    /// One physical's `Update1` through `Update3` jobs ("1-Input", "2-Logic",
    /// "3-State"), after every physical's `Update0`.
    #[expect(
        clippy::too_many_arguments,
        reason = "one physical's native preparation over shared world services"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "one ordered native preparation chain with typed failure boundaries"
    )]
    pub(crate) fn try_prepare_ground_pre_solve_pair(
        &mut self,
        world_owner: &Arc<Mutex<PhysicalAssemblyWorld>>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        actor: &ActorConstruction,
        ground: &StaticGroundPool,
        grind_policy: GroundPreparationGrindPolicy,
        numeric: &mut HostGroundPreparationNumeric,
        ai_respawn: &mut ActorAiRespawnQuery<'_>,
        #[cfg(test)] requested_transition: &mut Option<
            super::physical_state::ground_to_air::GroundDestination,
        >,
    ) -> Result<GroundPreSolveComplete, GroundPreparationError> {
        use super::ground_collision::GroundAfterCollision;
        use GroundPreparationError as E;
        let physical = actor.cached_physical_allocation().map_err(E::Actor)?;
        let mut cvars = physical.owner().inputs().cvars.clone();
        let input = actor
            .published_animation_physics_input()
            .map_err(E::Input)?;
        let continuation = self
            .try_begin_player_update1(
                board,
                skeleton,
                actor,
                &input,
                ground,
                &mut numeric.reckoning,
            )
            .map_err(E::Begin)?;
        let continuation = self
            .try_continue_player_update1_controls(board, skeleton, continuation)
            .map_err(E::Controls)?
            .continuation;
        {
            let input = continuation
                .input
                .lock()
                .map_err(|_| E::Poisoned("input"))?;
            let output = continuation
                .owner
                .phys_out()
                .lock()
                .map_err(|_| E::Poisoned("output"))?;
            continuation
                .processed
                .lock()
                .map_err(|_| E::Poisoned("processed"))?
                .try_copy_prior_input_runtime(&output, &input)
                .map_err(E::PriorInput)?;
        }
        continuation
            .processed
            .lock()
            .map_err(|_| E::Poisoned("processed"))?
            .try_copy_prior_output_tail(&continuation.owner)
            .map_err(E::PriorTail)?;
        continuation
            .processed
            .lock()
            .map_err(|_| E::Poisoned("processed"))?
            .try_copy_prior_output_body(&continuation.owner)
            .map_err(E::PriorBody)?;
        continuation
            .processed
            .lock()
            .map_err(|_| E::Poisoned("processed"))?
            .try_copy_prior_output_wheel_timers(&continuation.owner)
            .map_err(E::PriorWheels)?;
        self.try_continue_player_update1_ground(
            board,
            skeleton,
            &continuation,
            &mut numeric.reckoning,
        )
        .map_err(E::GroundInput)?;
        self.try_prepare_player_toolkit(board, skeleton, &mut numeric.reckoning)
            .map_err(E::Toolkit)?;
        self.try_fill_skeleton_line_surfaces(board, skeleton)
            .map_err(E::LineSurfaces)?;
        self.try_process_player_skeleton_data_prefix(
            board,
            skeleton,
            &continuation,
            &mut numeric.reckoning,
        )
        .map_err(E::Skeleton)?;
        self.try_process_input_after_skeleton(board, skeleton, &mut numeric.reckoning)
            .map_err(E::AfterSkeleton)?;
        if grind_policy == GroundPreparationGrindPolicy::RunRecoveredPrefix {
            self.try_update_player_grind_manager(
                board,
                skeleton,
                ground,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut numeric.collision_line,
            )
            .map_err(E::Grind)?;
        }
        self.try_publish_pair_jump_timing(board, skeleton, &cvars)
            .map_err(E::JumpTiming)?;
        let selector_ready = self
            .try_complete_pair_selector(
                board,
                skeleton,
                ground,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                grind_policy,
            )
            .map_err(E::Selector)?;
        continuation
            .processed
            .lock()
            .map_err(|_| E::Poisoned("processed"))?
            .publish_trajectory_selector_ready(selector_ready);
        self.try_update_pair_heading(board, skeleton, &cvars, &mut numeric.reckoning)
            .map_err(E::Heading)?;
        self.try_update_pair_permission_latches(board, skeleton, &cvars)
            .map_err(E::Permissions)?;
        self.try_finish_pair_process_input(board, skeleton, &cvars)
            .map_err(E::FinishInput)?;
        self.try_update_pair_system_logic_with_actor(
            board,
            skeleton,
            actor,
            &mut cvars,
            Some(ground),
            &mut numeric.reckoning,
            &mut |angle| Ok(angle.cos()),
            #[cfg(test)]
            requested_transition,
        )
        .map_err(E::SystemLogic)?;
        self.try_update_pair_prediction_before_state(
            board,
            skeleton,
            ground,
            &cvars,
            &mut crate::physical_player_grind_manager::math::GrindNum {
                n: &mut numeric.reckoning,
                s: &mut numeric.sin_cos,
            },
            &mut numeric.collision_line,
        )
        .map_err(E::Prediction)?;
        let selected = continuation
            .owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned("state"))?
            .current;
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::OffBoardPushing {
            self.try_update_pair_offboard_pushing(
                &continuation.owner,
                board,
                skeleton,
                ground,
                &cvars,
                &mut crate::physical_player_grind_manager::math::GrindNum {
                    n: &mut numeric.reckoning,
                    s: &mut numeric.sin_cos,
                },
            )
            .map_err(E::OffBoardPushing)?;
            let before = self
                .try_update3_controller_after_pushing(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            let retained = Arc::clone(&continuation.owner);
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                // Skeleton.UpdateOffBoardPushing82b5def0 does not run
                // Reckoning.UpdateGrindStates. Update5 still reads its owner.
                ground_reckoning: super::update5_managers::Update5ReckoningRead::RetainedPushing(
                    retained,
                ),
                update3,
                grind_policy,
            });
        }
        if selected
            == crate::physical_player_physics_state::CurrentPhysicalState::OffBoardLocomotion
        {
            let phase = self
                .try_update_pair_offboard_locomotion(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::OffBoardLocomotion)?;
            let completed =
                crate::physical_player_assembly::offboard_feet_ik_session::OffboardUpdateComplete {
                    owner: Arc::clone(&continuation.owner),
                    reckoning: phase,
                };
            let (before, phase) = self
                .try_update3_controller_after_offboard(
                    board,
                    skeleton,
                    completed,
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::LandingOnDeck {
            let phase = self
                .try_update_pair_landing_state(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::LandingOnDeck)?;
            let completed =
                crate::physical_player_assembly::offboard_feet_ik_session::OffboardUpdateComplete {
                    owner: Arc::clone(&continuation.owner),
                    reckoning: phase,
                };
            let (before, phase) = self
                .try_update3_controller_after_offboard(
                    board,
                    skeleton,
                    completed,
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::OffBoardAir {
            let phase = self
                .try_update_pair_offboard_air(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::OffBoardAir)?;
            let completed =
                crate::physical_player_assembly::offboard_feet_ik_session::OffboardUpdateComplete {
                    owner: Arc::clone(&continuation.owner),
                    reckoning: phase,
                };
            let (before, phase) = self
                .try_update3_controller_after_offboard(
                    board,
                    skeleton,
                    completed,
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::OffBoardJump {
            // PhysState_OffBoard_Jump.Update82d98960 through the same Update3
            // continuation as OffBoard_Air.
            let phase = self
                .try_update_pair_offboard_jump(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::OffBoardJump)?;
            let completed =
                crate::physical_player_assembly::offboard_feet_ik_session::OffboardUpdateComplete {
                    owner: Arc::clone(&continuation.owner),
                    reckoning: phase,
                };
            let (before, phase) = self
                .try_update3_controller_after_offboard(
                    board,
                    skeleton,
                    completed,
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::OffBoard {
            // PhysState_OffBoard.Update82da03c8 calls the same retained
            // OffBoardFeetIK. The reached prefix copies the last good
            // position, clamps foot normals, and sets animation foot-down
            // flags, queries four part transforms, then resolves foot-down
            // from valid surfaces and applies Processed foot-fix flags before
            // then advances FeetIK UpdateState before the pending IK target
            // calculation.
            let completed = self
                .try_begin_pair_offboard_ground_feet_ik(
                    &continuation.owner,
                    board,
                    skeleton,
                    &cvars,
                    ground,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::OffBoardFeetIk)?;
            let (before, phase) = self
                .try_update3_controller_after_offboard(
                    board,
                    skeleton,
                    completed,
                    &cvars,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::Teleporting {
            // Update3_State82dfe7d0 dispatches Teleporting.Update82d86940.
            // This first reached bail branch requests Actor respawn only when
            // Processed8c4 bit4 is clear. The Actor publishes a reset into its
            // existing owner; a following input frame must carry it back.
            let update = {
                let mut states = continuation
                    .owner
                    .remaining_states()
                    .lock()
                    .map_err(|_| E::Poisoned("Teleporting states"))?;
                let Some(StatePayload::Teleporting(actual)) = states
                    .states
                    .get_mut(&StateKind::Teleporting)
                    .filter(|state| state.published)
                    .and_then(|state| state.value.as_mut())
                else {
                    return Err(E::TeleportingMissingState);
                };
                let bindings = &actual.base.physical;
                if !bindings.board.same_as(board)
                    || !bindings.skeleton.same_as(skeleton)
                    || !bindings
                        .owner
                        .upgrade()
                        .is_some_and(|owner| Arc::ptr_eq(&owner, &continuation.owner))
                    || !Arc::ptr_eq(&actual.base.processed, &continuation.processed)
                {
                    return Err(E::TeleportingForeignState);
                }
                let processed = continuation
                    .processed
                    .lock()
                    .map_err(|_| E::Poisoned("Teleporting processed"))?;
                actual
                    .try_update_from_processed(&processed)
                    .map_err(E::TeleportingUpdate)?
            };
            if update == TeleportingUpdate::RequestActorRespawn {
                self.try_actor_appropriate_teleport(
                    actor,
                    world_owner,
                    ground,
                    &cvars,
                    &mut numeric.collision_line,
                    ai_respawn,
                )?;
            }
            // Update3_State82dfe7d4..e834 continues in this same frame. The
            // Actor's newly published reset is only read by a later Update1.
            let before = self
                .try_update3_controller_after_teleporting(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            let retained = Arc::clone(&continuation.owner);
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning:
                    super::update5_managers::Update5ReckoningRead::RetainedTeleporting(retained),
                update3,
                grind_policy,
            });
        }
        if matches!(
            selected,
            crate::physical_player_physics_state::CurrentPhysicalState::Air
                | crate::physical_player_physics_state::CurrentPhysicalState::KnownAir
        ) {
            let completed = if selected
                == crate::physical_player_physics_state::CurrentPhysicalState::KnownAir
            {
                self.try_update_pair_known_air(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    numeric,
                    grind_policy,
                )
                .map_err(E::KnownAir)?
            } else {
                let before = self
                    .try_begin_pair_air_update(
                        board,
                        skeleton,
                        ground,
                        grind_policy,
                        &mut numeric.reckoning,
                        &mut numeric.sin_cos,
                    )
                    .map_err(E::AirPrefix)?;
                self.try_finish_pair_air_update(board, skeleton, before, ground, numeric)
                    .map_err(E::AirFinish)?
            };
            let (before, ground_reckoning) = self
                .try_update3_controller_after_air(board, skeleton, completed)
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(
                    ground_reckoning,
                ),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::WipeoutGround {
            let completed = self
                .try_update_pair_wipeout(board, skeleton, ground, numeric)
                .map_err(E::Wipeout)?;
            let (before, ground_reckoning) = self
                .try_update3_controller_after_wipeout(board, skeleton, completed)
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning,
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::RevertGround {
            let phase = self
                .try_update_pair_revert_ground(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    numeric,
                )
                .map_err(E::Revert)?;
            let before = self
                .try_update3_controller_after_state(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if let crate::physical_player_physics_state::CurrentPhysicalState::Grind(kind) = selected {
            // PhysState_Grind_Common::Update82d9b3e8 through the grind type's vtable.
            let phase = self
                .try_update_pair_grind(
                    &continuation.owner,
                    board,
                    skeleton,
                    kind,
                    actor,
                    ground,
                    &cvars,
                    numeric,
                )
                .map_err(E::GrindState)?;
            let before = self
                .try_update3_controller_after_state(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::Nonspecific {
            let phase = self
                .try_update_pair_nonspecific(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    numeric,
                )
                .map_err(E::GrindState)?;
            let before = self
                .try_update3_controller_after_state(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::SlideGround {
            let phase = self
                .try_update_pair_slide_ground(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    numeric,
                    grind_policy,
                )
                .map_err(E::Slide)?;
            let before = self
                .try_update3_controller_after_state(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::Boneless {
            // Boneless.Update never runs Reckoning.Update; Update5 reads the retained one.
            self.try_update_pair_boneless(
                &continuation.owner,
                board,
                skeleton,
                ground,
                &cvars,
                numeric,
                grind_policy,
            )
            .map_err(E::Boneless)?;
            let before = self
                .try_update3_controller_after_state(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            let retained = Arc::clone(&continuation.owner);
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::RetainedPlant(
                    retained,
                    crate::physical_player_physics_state::CurrentPhysicalState::Boneless,
                ),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::FollowPath {
            let phase = self
                .try_update_pair_follow_path(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    numeric,
                )
                .map_err(E::FollowPath)?;
            let before = self
                .try_update3_controller_after_state(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::Skitching {
            let phase = self
                .try_update_pair_skitch(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    numeric,
                )
                .map_err(E::Skitching)?;
            let before = self
                .try_update3_controller_after_state(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::HandPlant {
            let phase = self
                .try_update_pair_handplant(
                    &continuation.owner,
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    numeric,
                )
                .map_err(E::HandPlant)?;
            let before = self
                .try_update3_controller_after_state(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::FootPlant {
            // FootPlant.Update never runs Reckoning.Update; Update5 reads the retained one.
            self.try_update_pair_foot_plant(
                &continuation.owner,
                board,
                skeleton,
                ground,
                &cvars,
                numeric,
                grind_policy,
            )
            .map_err(E::FootPlant)?;
            let before = self
                .try_update3_controller_after_state(
                    board,
                    skeleton,
                    Arc::clone(&continuation.owner),
                )
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            let retained = Arc::clone(&continuation.owner);
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::RetainedPlant(
                    retained,
                    crate::physical_player_physics_state::CurrentPhysicalState::FootPlant,
                ),
                update3,
                grind_policy,
            });
        }
        if selected == crate::physical_player_physics_state::CurrentPhysicalState::GroundAnimation {
            let completed = self
                .try_update_pair_ground_animation(
                    board,
                    skeleton,
                    ground,
                    &cvars,
                    numeric,
                    grind_policy,
                )
                .map_err(E::GroundAnimation)?;
            let (before, phase) = self
                .try_update3_controller_after_ground_animation(board, skeleton, completed)
                .map_err(E::Controller)?;
            let update3 = self
                .try_finish_update3_prephysics(
                    board,
                    skeleton,
                    before,
                    ground,
                    &mut numeric.collision_line,
                )
                .map_err(E::PrePhysics)?;
            return Ok(GroundPreSolveComplete {
                update1: continuation,
                ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(phase),
                update3,
                grind_policy,
            });
        }
        let ground_reckoning = self
            .try_update_pair_ground_reckoning(board, skeleton, &cvars, &mut numeric.reckoning)
            .map_err(E::Reckoning)?;
        self.try_update_pair_ground_handplants(board, skeleton)
            .map_err(E::Handplants)?;
        self.try_update_pair_ground_skateboard(
            board,
            skeleton,
            ground,
            &cvars,
            &mut numeric.reckoning,
            &mut numeric.sin_cos,
            &mut numeric.spawn_math,
        )
        .map_err(E::Skateboard)?;
        self.try_update_pair_ground_pumping(board, skeleton, &cvars, &mut numeric.reckoning)
            .map_err(E::Pumping)?;
        self.try_update_pair_ground_control(
            board,
            skeleton,
            &cvars,
            &mut numeric.reckoning,
            &mut numeric.sin_cos,
        )
        .map_err(E::Control)?;
        let before = self
            .try_update_pair_ground_wallride(
                board,
                skeleton,
                ground,
                &cvars,
                &mut numeric.reckoning,
            )
            .map_err(E::Wallride)?;
        let before = self
            .try_update_pair_ground_fake_torque(
                board,
                skeleton,
                before,
                &cvars,
                &mut numeric.fake_torque,
            )
            .map_err(E::FakeTorque)?;
        let after = self
            .try_update_pair_ground_collision(
                board,
                skeleton,
                before,
                &cvars,
                &mut numeric.reckoning,
            )
            .map_err(E::Collision)?;
        let completed = match after {
            GroundAfterCollision::JumpLaunch { before, velocity } => {
                super::ground_jump_runtime::launch_ground_wallride(
                    self,
                    &before.owner,
                    board,
                    skeleton,
                    velocity,
                    ground,
                    numeric,
                    grind_policy,
                )
                .map_err(E::GroundLaunch)?;
                super::ground_hang_pinning_session::GroundSkateboardComplete {
                    owner: before.owner,
                }
            }
            //82d9a654 branches to the UpdateSkateboard epilogue at a998.
            // All collision forces and requestPushStop have already committed;
            // skip ordinary braking/queues/hang and join Ground.Update1024.
            GroundAfterCollision::CollisionExit(before) => {
                super::ground_hang_pinning_session::GroundSkateboardComplete {
                    owner: before.owner,
                }
            }
            GroundAfterCollision::BeforeBraking(before) => {
                let forces = self
                    .try_update_pair_ground_ordinary_forces(
                        board,
                        skeleton,
                        GroundAfterCollision::BeforeBraking(before),
                        &cvars,
                        &mut numeric.reckoning,
                    )
                    .map_err(E::OrdinaryForces)?;
                let manual = self
                    .try_update_pair_ground_manual_nudge(
                        board,
                        skeleton,
                        forces,
                        &cvars,
                        &mut numeric.reckoning,
                        &mut numeric.sin_cos,
                    )
                    .map_err(E::Manual)?;
                let queued = self
                    .try_update_pair_ground_queue_torque_prefix(
                        board,
                        skeleton,
                        manual,
                        &mut numeric.reckoning,
                    )
                    .map_err(E::Queue)?;
                self.try_update_pair_ground_hang_pinning(
                    board,
                    skeleton,
                    queued,
                    &cvars,
                    &mut numeric.reckoning,
                )
                .map_err(E::HangPinning)?
            }
        };
        let predicted = self
            .try_update_pair_ground_force_prediction(
                board,
                skeleton,
                completed,
                &mut numeric.reckoning,
            )
            .map_err(E::ForcePrediction)?;
        let completed = self
            .try_finish_pair_ground_update(board, skeleton, predicted)
            .map_err(E::FinishGround)?;
        let before = self
            .try_update3_controller_after_ground(board, skeleton, completed)
            .map_err(E::Controller)?;
        let update3 = self
            .try_finish_update3_prephysics(
                board,
                skeleton,
                before,
                ground,
                &mut numeric.collision_line,
            )
            .map_err(E::PrePhysics)?;
        Ok(GroundPreSolveComplete {
            update1: continuation,
            ground_reckoning: super::update5_managers::Update5ReckoningRead::Completed(
                ground_reckoning,
            ),
            update3,
            grind_policy,
        })
    }
}
