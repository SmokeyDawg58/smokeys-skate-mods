//! Reusable selected post-solve schedule; no fixture setup or numeric policy.
//! Mount under `simulation_assembly_backend::constraint_session`.
#[path = "../offboard/physical_player_offboard_postphysics.rs"]
mod offboard_post;
use crate::physics_state_ids::PHYSICS_CATEGORY_OFFBOARD;
use super::{
    board_fill_contacts, board_fill_deck, board_fill_final, board_fill_forward, board_fill_ground,
    board_fill_prefix, board_fill_pumping, board_fill_surface, board_fill_wheels,
    board_fill_wobble, board_post_outer_live, ground_wipeout, physical_board_forces,
    physical_island, physical_skeleton_lines, physical_update5_prefix, skeleton_post_wipeout,
    skeleton_postphysics_live, update5_controller, update5_managers, update5_outer_locality,
    update5_plants, update5_pumping, update5_skeleton_parts, update5_skeleton_prefix,
    update5_skeleton_suffix, update5_wipeout,
};
use crate::{
    actor_construction::ActorConstruction,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    dynamic_assembly_session::skater_pool::SimulationSkaterPoolError,
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        selected_animation_conditioning::{
            SelectedAnimationFinishError, SelectedGroundAnimationComplete,
        },
        selected_ground_conditioning::{
            SelectedGroundConditioningError, SelectedGroundPhysicsComplete,
            SelectedPhysicsFinishError, try_condition_selected_ground,
        },
    },
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    skateboard_truck_drives::SkateboardTruckDriveNumeric,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

/// Retain these actual owners between ticks; do not recreate query residency.
pub(crate) struct PostSolveRequest<'a> {
    pub(crate) board: &'a SimulationSkaterAssemblyHandle,
    pub(crate) skeleton: &'a SimulationSkaterAssemblyHandle,
    pub(crate) owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) ground: &'a StaticGroundPool,
    pub(crate) ground_reckoning: &'a update5_managers::Update5ReckoningRead,
    pub(crate) actor: &'a ActorConstruction,
    pub(crate) grind_policy: update5_managers::GrindManagerFillPolicy,
    pub(crate) scoring: Option<crate::physical_player_assembly::scoring_input::ScoringFrameSchedule<'a>>,
}
pub(crate) struct PostSolveNumerics<'a, L, N, R, T, C> {
    pub(crate) line: &'a mut L,
    pub(crate) vector: &'a mut N,
    pub(crate) rounding: &'a mut R,
    pub(crate) truck: &'a mut T,
    pub(crate) collision: &'a mut C,
}
/// Errors preserve earlier source publications. Retrying the whole function is
/// not a rollback/recovery operation; the caller must retain the failed stage.
#[derive(Debug)]
pub(crate) enum PostSolveError<L, N, R, T, C> {
    Owner(physical_island::SimulationPhysicalBodyError),
    ForeignOwner,
    PoisonedState,
    UnsupportedState(crate::physical_player_physics_state::CurrentPhysicalState),
    AirPost(Box<crate::physical_player_phys_out::construct::air_post_solve::AirPostError<
        crate::physical_player_phys_out::construct::air_post_solve::runtime::AirPostRuntimeError<N>>>),
    PoisonedController,
    MissingControllerState,
    BoardBody(SimulationSkaterPoolError),
    MissingBoardPostFields,
    BoardReturnState(&'static str),
    BoardReturnRequest(crate::physical_player_wipeout::ground_access::GroundOwnerError),
    SkeletonLines(Box<physical_skeleton_lines::SkeletonLineTestsError<std::convert::Infallible>>),
    BoardPost(Box<board_post_outer_live::BoardPostOuterError<L, N, R, T>>),
    SkeletonPost(Box<skeleton_postphysics_live::SkeletonPostPhysicsLiveError<N>>),
    GroundPost(Box<ground_wipeout::GroundWipeoutRuntimeError<N>>),
    OffBoardPost(Box<offboard_post::OffBoardPostPhysicsError<N>>),
    KnownAir(Box<crate::physical_player_assembly::known_air_state::KnownAirStateError>),
    OffBoardAir(Box<crate::physical_player_assembly::offboard_air_state::AirStateError>),
    OffBoardJump(Box<crate::physical_player_assembly::offboard_jump_state::JumpStateError>),
    LandingOnDeck(Box<crate::physical_player_assembly::landing_on_deck_state::LandingStateError>),
    OffBoardLocomotion(Box<crate::physical_player_assembly::offboard_locomotion_state::LocomotionStateError>),
    OffBoardPushing(Box<crate::physical_player_grind_manager::view::GrindError>),
    Boneless(Box<crate::simulation_assembly_backend::constraint_session::boneless_state::BonelessError>),
    FootPlant(Box<crate::simulation_assembly_backend::constraint_session::footplant_air::FootPlantUpdateError>),
    WipeoutPost(Box<crate::simulation_assembly_backend::constraint_session::physical_grounded::wipeout::post_solve::WipeoutPostError<N>>),
    PostWipeout(Box<skeleton_post_wipeout::PostWipeoutLiveError<N, R>>),
    Update5(Box<physical_update5_prefix::Update5PrefixError>),
    BoardFill(Box<board_fill_prefix::BoardFillPrefixError<C>>),
    BoardPumping(Box<board_fill_pumping::BoardPumpingError>),
    BoardContacts(Box<board_fill_contacts::BoardContactOutputError>),
    BoardDeck(Box<board_fill_deck::BoardDeckOutputError>),
    BoardSurface(Box<board_fill_surface::BoardSurfaceOutputError<N>>),
    BoardWobble(Box<board_fill_wobble::BoardWobbleError>),
    BoardForward(Box<board_fill_forward::BoardForwardError<C>>),
    BoardWheels(Box<board_fill_wheels::BoardWheelPositionError>),
    BoardGround(Box<board_fill_ground::BoardGroundError<C>>),
    BoardFinal(Box<board_fill_final::BoardFinalError<C>>),
    SkeletonPrefix(Box<update5_skeleton_prefix::Update5SkeletonPrefixError<N>>),
    SkeletonParts(Box<update5_skeleton_parts::Update5SkeletonPartsError>),
    SkeletonSuffix(Box<update5_skeleton_suffix::Update5SkeletonSuffixError<N>>),
    Pumping(Box<update5_pumping::Update5PumpingError>),
    Managers(Box<update5_managers::Update5ManagersError>),
    Plants(Box<update5_plants::Update5PlantError>),
    Controller(Box<update5_controller::Update5ControllerError<N>>),
    Wipeout(Box<update5_wipeout::Update5WipeoutError>),
    OuterLocality(Box<update5_outer_locality::Update5OuterSessionError>),
    BoardForceRuntime(Box<physical_board_forces::PhysicalPlayerBoardForceError>),
    ConditionGround(Box<SelectedGroundConditioningError<N>>),
    FinishPhysics(Box<SelectedPhysicsFinishError<N>>),
    FinishAnimation(Box<SelectedAnimationFinishError>),
}
impl<
    L: std::fmt::Debug,
    N: std::fmt::Debug,
    R: std::fmt::Debug,
    T: std::fmt::Debug,
    C: std::fmt::Debug,
> std::fmt::Display for PostSolveError<L, N, R, T, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PoisonedState => f.write_str("post-solve state lock poisoned"),
            Self::UnsupportedState(state) => write!(f, "post-solve state pending: {state:?}"),
            Self::AirPost(error) => write!(f, "selected Air post-solve: {error}"),
            Self::Owner(error) => write!(f, "selected post-solve stage Owner: {error:?}"),
            Self::BoardReturnState(error) => write!(f, "board post-physics request: {error}"),
            Self::BoardReturnRequest(error) => write!(f, "board post-physics request: {error:?}"),
            Self::SkeletonLines(error) => {
                write!(f, "selected post-solve stage SkeletonLines: {error:?}")
            }
            Self::BoardPost(error) => write!(f, "selected post-solve stage BoardPost: {error:?}"),
            Self::SkeletonPost(error) => {
                write!(f, "selected post-solve stage SkeletonPost: {error:?}")
            }
            Self::GroundPost(error) => write!(f, "selected post-solve stage GroundPost: {error:?}"),
            Self::WipeoutPost(error) => write!(f, "selected Wipeout post-solve: {error:?}"),
            Self::PostWipeout(error) => {
                write!(f, "selected post-solve stage PostWipeout: {error:?}")
            }
            Self::Update5(error) => write!(f, "selected post-solve stage Update5: {error:?}"),
            Self::BoardFill(error) => write!(f, "selected post-solve stage BoardFill: {error:?}"),
            Self::BoardPumping(error) => {
                write!(f, "selected post-solve stage BoardPumping: {error:?}")
            }
            Self::BoardContacts(error) => {
                write!(f, "selected post-solve stage BoardContacts: {error:?}")
            }
            Self::BoardDeck(error) => write!(f, "selected post-solve stage BoardDeck: {error:?}"),
            Self::BoardSurface(error) => {
                write!(f, "selected post-solve stage BoardSurface: {error:?}")
            }
            Self::BoardWobble(error) => {
                write!(f, "selected post-solve stage BoardWobble: {error:?}")
            }
            Self::BoardForward(error) => {
                write!(f, "selected post-solve stage BoardForward: {error:?}")
            }
            Self::BoardWheels(error) => {
                write!(f, "selected post-solve stage BoardWheels: {error:?}")
            }
            Self::BoardGround(error) => {
                write!(f, "selected post-solve stage BoardGround: {error:?}")
            }
            Self::BoardFinal(error) => write!(f, "selected post-solve stage BoardFinal: {error:?}"),
            Self::SkeletonPrefix(error) => {
                write!(f, "selected post-solve stage SkeletonPrefix: {error:?}")
            }
            Self::SkeletonParts(error) => {
                write!(f, "selected post-solve stage SkeletonParts: {error:?}")
            }
            Self::SkeletonSuffix(error) => {
                write!(f, "selected post-solve stage SkeletonSuffix: {error:?}")
            }
            Self::Pumping(error) => write!(f, "selected post-solve stage Pumping: {error:?}"),
            Self::Managers(error) => write!(f, "selected post-solve stage Managers: {error:?}"),
            Self::Plants(error) => write!(f, "selected post-solve stage Plants: {error:?}"),
            Self::Controller(error) => write!(f, "selected post-solve stage Controller: {error:?}"),
            Self::Wipeout(error) => write!(f, "selected post-solve stage Wipeout: {error:?}"),
            Self::OuterLocality(error) => {
                write!(f, "selected post-solve stage OuterLocality: {error:?}")
            }
            Self::BoardForceRuntime(error) => {
                write!(f, "selected post-solve stage BoardForceRuntime: {error:?}")
            }
            Self::ConditionGround(error) => {
                write!(f, "selected post-solve stage ConditionGround: {error:?}")
            }
            Self::FinishPhysics(error) => {
                write!(f, "selected post-solve stage FinishPhysics: {error:?}")
            }
            Self::FinishAnimation(error) => {
                write!(f, "selected post-solve stage FinishAnimation: {error:?}")
            }
            other => write!(f, "selected post-solve stage: {other:?}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    /// Continue the same pair AFTER successful solver/contact feedback:
    /// `HiLOD` post-physics82e01d00 -> Update5 -> Physics -> Animation82e31970.
    /// The returned token retains the explicit grind policy and is not full-frame
    /// completion. Actor Motion/input and solver policies belong to the caller.
    #[expect(
        clippy::type_complexity,
        reason = "one source-ordered call chain over existing typed stage capabilities"
    )]
    pub(crate) fn try_post_solve_selected_ground<L, N, R, T, C>(
        &mut self,
        request: PostSolveRequest<'_>,
        numeric: &mut PostSolveNumerics<'_, L, N, R, T, C>,
    ) -> Result<
        SelectedGroundAnimationComplete,
        PostSolveError<<L as ReckoningNumeric>::Error, N::Error, R::Error, T::Error, C::Error>,
    >
    where
        L: AssemblyGroundNumeric + ReckoningNumeric,
        N: ReckoningNumeric,
        N::Error: std::fmt::Debug,
        R: ReckoningSinCosNumeric,
        T: SkateboardTruckDriveNumeric,
        C: SkeletonCollisionNumeric,
    {
        let (board, skeleton, scoring) = (request.board, request.skeleton, request.scoring);
        let physics = self.try_post_solve_selected_physics(request, numeric)?;
        self.try_finish_selected_animation(board, skeleton, physics, scoring)
            .map_err(|e| PostSolveError::FinishAnimation(Box::new(e)))
    }

    /// `World::Update3` 0x827af8f0 for one physical: its "4-Adjust" and "5-Output"
    /// jobs, through the Physics conditioner. `UpdateConditioners` (Animation and
    /// Scoring) follows only after every physical has run this.
    #[expect(
        clippy::too_many_lines,
        clippy::type_complexity,
        clippy::needless_pass_by_value,
        reason = "one source-ordered call chain over existing typed stage capabilities"
    )]
    pub(crate) fn try_post_solve_selected_physics<L, N, R, T, C>(
        &mut self,
        request: PostSolveRequest<'_>,
        numeric: &mut PostSolveNumerics<'_, L, N, R, T, C>,
    ) -> Result<
        SelectedGroundPhysicsComplete,
        PostSolveError<<L as ReckoningNumeric>::Error, N::Error, R::Error, T::Error, C::Error>,
    >
    where
        L: AssemblyGroundNumeric + ReckoningNumeric,
        N: ReckoningNumeric,
        N::Error: std::fmt::Debug,
        R: ReckoningSinCosNumeric,
        T: SkateboardTruckDriveNumeric,
        C: SkeletonCollisionNumeric,
    {
        use PostSolveError as E;
        let PostSolveRequest {
            board,
            skeleton,
            owner,
            ground,
            ground_reckoning,
            actor,
            grind_policy,
            scoring: _,
        } = request;
        // Host identity checks do not sample source branch inputs or mutate state.
        for handle in [board, skeleton] {
            if !Arc::ptr_eq(
                self.physical_body_owner_for_contacts(handle)
                    .map_err(E::Owner)?,
                owner,
            ) {
                return Err(E::ForeignOwner);
            }
        }
        self.try_end_skeleton_line_tests(board, skeleton)
            .map_err(|e| E::SkeletonLines(Box::new(e)))?;
        let board_return = self
            .try_update_board_postphysics_live(
                board,
                skeleton,
                ground,
                &mut board_post_outer_live::BoardPostNumerics {
                    line: numeric.line,
                    vector: numeric.vector,
                    rounding: numeric.rounding,
                    truck: numeric.truck,
                },
            )
            .map_err(|e| E::BoardPost(Box::new(e)))?;
        // Update4 82e01cb0..ce8: only BADACCELERATION (1) reads the
        // actual Processed category. Offboard ignores that result; other
        // categories request BadSkateboard before Skeleton.PostPhysics.
        if board_return == 1 {
            let processed = Arc::clone(
                owner
                    .reckoning()
                    .lock()
                    .map_err(|_| E::BoardReturnState("Reckoning lock"))?
                    .processed(),
            );
            let category = processed
                .lock()
                .map_err(|_| E::BoardReturnState("Processed lock"))?
                .physics_state_fields()
                .category
                .ok_or(E::BoardReturnState("Processed category+8ec"))?;
            if category != PHYSICS_CATEGORY_OFFBOARD {
                owner
                    .wipeout()
                    .lock()
                    .map_err(|_| E::BoardReturnState("Wipeout lock"))?
                    .try_ground_request(crate::wipeout::WipeoutType::BadSkateboard.index())
                    .map_err(E::BoardReturnRequest)?;
            }
        }
        // IsDeckInHand82dc8358 reads controller+1bc. Only state 1 reads
        // SkateboardBody CollisionInfo.numPartsInContact at Body+360.
        let deck_in_hand = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::PoisonedController)?
            .state()
            .ok_or(E::MissingControllerState)?
            == 1;
        let request_partial_ragdoll = if deck_in_hand {
            self.board_construction(board)
                .map_err(E::BoardBody)?
                .and_then(
                    crate::skateboard_body_construction::SkateboardBodyConstruction::post_fields,
                )
                .ok_or(E::MissingBoardPostFields)?
                .num_parts
                != 0
        } else {
            false
        };
        self.try_update_skeleton_postphysics_live(
            board,
            skeleton,
            &owner.inputs().cvars,
            request_partial_ragdoll,
            numeric.vector,
        )
        .map_err(|e| E::SkeletonPost(Box::new(e)))?;
        let overlay = self.debug_overlay.clone();
        let selected = owner
            .physics_state()
            .lock()
            .map_err(|_| E::PoisonedState)?
            .current;
        match selected {
            crate::physical_player_physics_state::CurrentPhysicalState::Ground
            | crate::physical_player_physics_state::CurrentPhysicalState::SlideGround => self
                .try_update_ground_postphysics(board, skeleton, owner, &overlay, numeric.vector)
                .map_err(|e| E::GroundPost(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::RevertGround => self
                .try_check_ground_wipeout(board, skeleton, owner, &overlay, numeric.vector)
                .map_err(|e| E::GroundPost(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::GroundAnimation => self
                .try_update_ground_animation_postphysics(
                    board,
                    skeleton,
                    owner,
                    &overlay,
                    numeric.vector,
                )
                .map_err(|e| E::GroundPost(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::OffBoardPushing => self
                .try_post_pair_offboard_pushing(
                    owner,
                    board,
                    skeleton,
                    &owner.inputs().cvars,
                    numeric.vector,
                )
                .map_err(|e| E::OffBoardPushing(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::OffBoardLocomotion => self
                .try_post_pair_offboard_locomotion(
                    owner,
                    board,
                    skeleton,
                    &owner.inputs().cvars,
                    numeric.vector,
                )
                .map_err(|e| E::OffBoardLocomotion(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::LandingOnDeck => self
                .try_post_pair_landing_state(
                    owner,
                    board,
                    skeleton,
                    &owner.inputs().cvars,
                    numeric.vector,
                )
                .map_err(|e| E::LandingOnDeck(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::OffBoardAir => self
                .try_post_pair_offboard_air(
                    owner,
                    board,
                    skeleton,
                    &owner.inputs().cvars,
                    numeric.vector,
                )
                .map_err(|e| E::OffBoardAir(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::OffBoardJump => self
                .try_post_pair_offboard_jump(
                    owner,
                    board,
                    skeleton,
                    &owner.inputs().cvars,
                    numeric.vector,
                )
                .map_err(|e| E::OffBoardJump(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::OffBoard => self
                .try_update_offboard_postphysics(board, skeleton, owner, numeric.vector)
                .map_err(|e| E::OffBoardPost(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::KnownAir => self
                .try_post_pair_known_air(owner, board, skeleton, numeric.vector)
                .map_err(|e| E::KnownAir(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::Air => self
                .try_update_air_postphysics(board, skeleton, owner, numeric.vector)
                .map_err(|e| E::AirPost(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::WipeoutGround => self
                .try_update_wipeout_postphysics(board, skeleton, owner, numeric.vector)
                .map_err(|e| E::WipeoutPost(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::Boneless => self
                .try_post_pair_boneless(owner, board, skeleton, &overlay, numeric.vector)
                .map_err(|e| E::Boneless(Box::new(e)))?,
            crate::physical_player_physics_state::CurrentPhysicalState::FootPlant => self
                .try_post_pair_foot_plant(owner, board, skeleton, &overlay, numeric.vector)
                .map_err(|e| E::FootPlant(Box::new(e)))?,
            // Teleporting's UpdatePostPhysics vtable slot is the shared empty
            // callback at 0x825187c0. The outer Wipeout check still follows.
            crate::physical_player_physics_state::CurrentPhysicalState::Teleporting => {}
            // HandPlant.UpdatePostPhysics82d893b8 is empty.
            crate::physical_player_physics_state::CurrentPhysicalState::HandPlant => {}
            // Skitching.UpdatePostPhysics is the shared BLR825187c0.
            crate::physical_player_physics_state::CurrentPhysicalState::Skitching => {}
            // FollowPath.UpdatePostPhysics82d91138: only the AI path following adjust.
            crate::physical_player_physics_state::CurrentPhysicalState::FollowPath => self
                .try_update_follow_path_postphysics(board, owner, numeric.vector)
                .map_err(|e| E::GroundPost(Box::new(e)))?,
            // PhysState_Grind_Common::UpdatePostPhysics82d85e78.
            crate::physical_player_physics_state::CurrentPhysicalState::Grind(_) => self
                .try_check_grind_wipeout(board, skeleton, owner, &overlay, numeric.vector)
                .map_err(|e| E::GroundPost(Box::new(e)))?,
            // Nonspecific.UpdatePostPhysics82d86768: only under kDoWipeoutsTest.
            crate::physical_player_physics_state::CurrentPhysicalState::Nonspecific => {
                if owner
                    .inputs()
                    .cvars
                    .try_grind_bool(
                        crate::input_cvar_runtime::grind_states::GrindCvar::kDoWipeoutsTest,
                    )
                    .map_err(|e| {
                        E::GroundPost(Box::new(ground_wipeout::GroundWipeoutRuntimeError::Cvar(e)))
                    })?
                {
                    self.try_check_grind_wipeout(board, skeleton, owner, &overlay, numeric.vector)
                        .map_err(|e| E::GroundPost(Box::new(e)))?;
                }
            }
            other => return Err(E::UnsupportedState(other)),
        }
        self.try_begin_post_wipeout(board, skeleton, owner, numeric.vector, numeric.rounding)
            .map_err(|e| E::PostWipeout(Box::new(e)))?;
        let stage = self
            .try_begin_player_update5(board, skeleton)
            .map_err(|e| E::Update5(Box::new(e)))?;
        let stage = self
            .try_continue_update5_board_fill(board, skeleton, stage, numeric.collision)
            .map_err(|e| E::BoardFill(Box::new(e)))?;
        let stage = self
            .try_continue_update5_board_pumping(board, skeleton, stage)
            .map_err(|e| E::BoardPumping(Box::new(e)))?;
        let stage = self
            .try_continue_update5_board_contacts(board, skeleton, stage)
            .map_err(|e| E::BoardContacts(Box::new(e)))?;
        let stage = self
            .try_continue_update5_board_deck(board, skeleton, stage)
            .map_err(|e| E::BoardDeck(Box::new(e)))?;
        let stage = self
            .try_continue_update5_board_surface(board, skeleton, stage, numeric.vector)
            .map_err(|e| E::BoardSurface(Box::new(e)))?;
        let stage = self
            .try_continue_update5_board_wobble(board, skeleton, stage)
            .map_err(|e| E::BoardWobble(Box::new(e)))?;
        let stage = self
            .try_continue_update5_board_forward(board, skeleton, stage, numeric.collision)
            .map_err(|e| E::BoardForward(Box::new(e)))?;
        let stage = self
            .try_continue_update5_board_wheel_positions(board, skeleton, stage)
            .map_err(|e| E::BoardWheels(Box::new(e)))?;
        let stage = self
            .try_continue_update5_board_ground(board, skeleton, stage, numeric.collision)
            .map_err(|e| E::BoardGround(Box::new(e)))?;
        let stage = self
            .try_finish_update5_board_fill(board, skeleton, stage, numeric.collision)
            .map_err(|e| E::BoardFinal(Box::new(e)))?;
        let stage = self
            .try_continue_update5_skeleton_prefix(board, skeleton, stage, numeric.vector)
            .map_err(|e| E::SkeletonPrefix(Box::new(e)))?;
        let stage = self
            .try_continue_update5_skeleton_parts(board, skeleton, stage)
            .map_err(|e| E::SkeletonParts(Box::new(e)))?;
        let stage = self
            .try_continue_update5_skeleton_suffix(
                board,
                skeleton,
                stage,
                &owner.inputs().cvars,
                numeric.vector,
            )
            .map_err(|e| E::SkeletonSuffix(Box::new(e)))?;
        let stage = self
            .try_continue_update5_pumping(board, skeleton, stage)
            .map_err(|e| E::Pumping(Box::new(e)))?;
        let stage = self
            .try_continue_update5_managers_with_reckoning(
                board,
                skeleton,
                stage,
                ground_reckoning,
                grind_policy,
            )
            .map_err(|e| E::Managers(Box::new(e)))?;
        let stage = self
            .try_continue_update5_plants(board, skeleton, stage)
            .map_err(|e| E::Plants(Box::new(e)))?;
        let stage = self
            .try_continue_update5_controller(board, skeleton, stage, numeric.vector)
            .map_err(|e| E::Controller(Box::new(e)))?;
        let stage = self
            .try_continue_update5_wipeout(board, skeleton, stage)
            .map_err(|e| E::Wipeout(Box::new(e)))?;
        let stage = self
            .try_continue_update5_outer_locality(board, skeleton, stage, actor)
            .map_err(|e| E::OuterLocality(Box::new(e)))?;
        let attributes = self
            .try_with_board_force_runtime(board, |runtime| runtime.push_attributes.clone())
            .map_err(|e| E::BoardForceRuntime(Box::new(e)))?;
        let stage = try_condition_selected_ground(
            self,
            stage,
            board,
            skeleton,
            Some(&attributes),
            &owner.inputs().cvars,
            &overlay,
            numeric.vector,
        )
        .map_err(|e| E::ConditionGround(Box::new(e)))?;
        self.try_finish_selected_physics(board, skeleton, stage, numeric.vector)
            .map_err(|e| E::FinishPhysics(Box::new(e)))
    }
}
