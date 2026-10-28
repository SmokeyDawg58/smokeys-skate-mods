//! Selected Ground/Air/GroundAnimation transitions and `Update2_SystemLogic`.
//! References: `CheckForStateChange`82e005f8, `SetPhysicsState`82dff180.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GRIND, PHYSICS_CATEGORY_GROUND,
    PHYSICS_CATEGORY_GROUNDTRICK, PHYSICS_CATEGORY_OFFBOARD, PHYSICS_CATEGORY_OTHER,
    PHYSICS_CATEGORY_WIPEOUT, PHYSICS_STATE_BONELESS, PHYSICS_STATE_FOLLOW_PATH,
    PHYSICS_STATE_FOOTPLANT, PHYSICS_STATE_GRIND_BACKSLASH, PHYSICS_STATE_GRIND_BOARDSLIDE,
    PHYSICS_STATE_GROUND_ANIMATION, PHYSICS_STATE_HANDPLANT, PHYSICS_STATE_KNOWN_AIR,
    PHYSICS_STATE_LANDINGONDECK, PHYSICS_STATE_NONSPECIFIC, PHYSICS_STATE_OFFBOARD,
    PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_OFFBOARD_JUMP, PHYSICS_STATE_OFFBOARD_LOCOMOTION,
    PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_PHYSICS_AIR, PHYSICS_STATE_PHYSICS_GROUND,
    PHYSICS_STATE_REVERT_GROUND, PHYSICS_STATE_SKITCHING, PHYSICS_STATE_SLIDE_GROUND,
    PHYSICS_STATE_TELEPORTING, PHYSICS_STATE_WIPEOUT_GROUND,
};
use super::{PhysicalStateSessionError, air::AirEntryError, ground_exit::GroundExitError};
use crate::{
    actor_construction::{
        ActorConstruction, ActorConstructionError, physical_mount::ActorPhysicalMountError,
    },
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{
        PhysicalAssemblyWorld,
        remaining_states::{OwnerMember, StateKind, StatePayload},
    },
    physical_player_physics_state::{CurrentPhysicalState, GrindKind, PhysicalStateStage},
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::physical_board_forces::PhysicalPlayerBoardForceError,
    simulation_assembly_backend::constraint_session::system_logic::SystemLogicError,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum GroundToAirError<N> {
    Selection(SystemLogicError<InputCvarError>),
    Session(PhysicalStateSessionError<ActorConstructionError, N>),
    Actor(ActorPhysicalMountError),
    ForeignActor,
    WrongStage,
    Exit(GroundExitError<N>),
    Entry(AirEntryError<N>),
    WipeoutEntry(super::wipeout::WipeoutEntryError<N, N>),
    GroundAnimationEntry(super::ground_animation_entry::GroundAnimationEntryError<N>),
    GroundAnimationExit(super::ground_animation_exit::GroundAnimationExitError<N>),
    OffBoardEntry(super::offboard_entry::OffBoardEntryError<N>),
    OffBoardPushing(crate::physical_player_grind_manager::view::GrindError),
    HandPlant(crate::physical_player_grind_manager::view::GrindError),
    Skitching(crate::physical_player_grind_manager::view::GrindError),
    FollowPath(crate::physical_player_grind_manager::view::GrindError),
    OffBoardAir(crate::physical_player_assembly::offboard_air_state::AirStateError),
    OffBoardJump(crate::physical_player_assembly::offboard_jump_state::JumpStateError),
    LandingOnDeck(crate::physical_player_assembly::landing_on_deck_state::LandingStateError),
    OffBoardLocomotion(
        crate::physical_player_assembly::offboard_locomotion_state::LocomotionStateError,
    ),
    MissingAirGround,
    Revert(crate::simulation_assembly_backend::constraint_session::revert_ground::RevertError),
    Slide(crate::simulation_assembly_backend::constraint_session::slide_ground::SlideError),
    KnownAir(crate::physical_player_assembly::known_air_state::KnownAirStateError),
    OffBoardStart(
        crate::skateboard_controller::SkateboardControllerError<
            crate::physical_player_assembly::offboard_controller_start::OffBoardStartServiceError<
                N,
            >,
        >,
    ),
    OffBoardStop(
        crate::skateboard_controller::SkateboardControllerError<
            crate::physical_player_assembly::offboard_controller_stop::OffBoardStopServiceError,
        >,
    ),
    WipeoutExit(PhysicalPlayerBoardForceError),
    OffBoardExit(&'static str),
    Boneless(crate::simulation_assembly_backend::constraint_session::boneless_state::BonelessError),
    FootPlant(
        crate::simulation_assembly_backend::constraint_session::footplant_air::FootPlantUpdateError,
    ),
    OffBoardJumping(
        crate::physical_player_remaining_constructor_owners::jumping_runtime::JumpingError<
            N,
            crate::input_cvar_runtime::InputCvarError,
        >,
    ),
    Grind(crate::simulation_assembly_backend::constraint_session::grind_states::GrindStateError),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundToAirError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GroundAnimationEntry(error) => write!(f, "state103 entry: {error}"),
            Self::GroundAnimationExit(error) => write!(f, "state103 exit: {error}"),
            Self::OffBoardEntry(error) => write!(f, "state500 entry: {error}"),
            Self::OffBoardStart(error) => write!(f, "state500 controller: {error}"),
            Self::OffBoardStop(error) => write!(f, "state500 stop controller: {error}"),
            Self::WipeoutExit(error) => write!(f, "state300 exit: {error}"),
            Self::OffBoardExit(error) => write!(f, "state500 exit: {error}"),
            Self::Boneless(error) => write!(f, "state602 entry: {error}"),
            Self::FootPlant(error) => write!(f, "state601 transition: {error}"),
            Self::Selection(error) => write!(f, "Ground/Air selection: {error}"),
            Self::Session(error) => write!(f, "Ground/Air transition: {error}"),
            Self::Actor(error) => write!(f, "Ground/Air Actor: {error}"),
            Self::Exit(error) => write!(f, "Ground/Air exit: {error}"),
            Self::Entry(error) => write!(f, "Ground/Air entry: {error}"),
            Self::WipeoutEntry(error) => write!(f, "Ground/Wipeout entry: {error}"),
            other => write!(f, "Ground/Air transition: {other:?}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroundDestination {
    Ground,
    Air,
    KnownAir,
    Wipeout,
    GroundAnimation,
    Skitching,
    FollowPath,
    SlideGround,
    RevertGround,
    OffBoard,
    OffBoardPushing,
    OffBoardAir,
    OffBoardJump,
    LandingOnDeck,
    OffBoardLocomotion,
    Teleporting,
    Boneless,
    FootPlant,
    HandPlant,
    Grind(GrindKind),
    Nonspecific,
}
impl GroundDestination {
    const fn id(self) -> i32 {
        match self {
            Self::Grind(kind) => kind.id(),
            Self::Nonspecific => PHYSICS_STATE_NONSPECIFIC,
            Self::Ground => PHYSICS_STATE_PHYSICS_GROUND,
            Self::SlideGround => PHYSICS_STATE_SLIDE_GROUND,
            Self::RevertGround => PHYSICS_STATE_REVERT_GROUND,
            Self::Air => PHYSICS_STATE_PHYSICS_AIR,
            Self::KnownAir => PHYSICS_STATE_KNOWN_AIR,
            Self::Wipeout => PHYSICS_STATE_WIPEOUT_GROUND,
            Self::GroundAnimation => PHYSICS_STATE_GROUND_ANIMATION,
            Self::Skitching => PHYSICS_STATE_SKITCHING,
            Self::FollowPath => PHYSICS_STATE_FOLLOW_PATH,
            Self::OffBoard => PHYSICS_STATE_OFFBOARD,
            Self::OffBoardPushing => PHYSICS_STATE_OFFBOARD_PUSHING,
            Self::OffBoardAir => PHYSICS_STATE_OFFBOARD_AIR,
            Self::OffBoardJump => PHYSICS_STATE_OFFBOARD_JUMP,
            Self::LandingOnDeck => PHYSICS_STATE_LANDINGONDECK,
            Self::OffBoardLocomotion => PHYSICS_STATE_OFFBOARD_LOCOMOTION,
            Self::Teleporting => PHYSICS_STATE_TELEPORTING,
            Self::Boneless => PHYSICS_STATE_BONELESS,
            Self::FootPlant => PHYSICS_STATE_FOOTPLANT,
            Self::HandPlant => PHYSICS_STATE_HANDPLANT,
        }
    }
}

impl PhysicalAssemblyWorld {
    /// Uses the actual selector once. Its pending-transition return retains the
    /// source prefix; this continuation performs the missing transition before
    /// the sole request-clear suffix. No repeated selection/counter update.
    #[cfg_attr(
        test,
        expect(
            clippy::too_many_arguments,
            reason = "test-only controlled transition policy"
        )
    )]
    pub(crate) fn try_update_pair_system_logic_with_actor<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        actor: &ActorConstruction,
        refs: &mut InputCvarReferences,
        ground: Option<&crate::static_ground_pool::StaticGroundPool>,
        numeric: &mut N,
        cosine: &mut impl FnMut(f32) -> Result<f32, N::Error>,
        #[cfg(test)] requested_transition: &mut Option<GroundDestination>,
    ) -> Result<(), GroundToAirError<N::Error>>
    where
        N::Error: std::fmt::Debug,
    {
        use GroundToAirError as E;
        // Reject foreign host ownership before any selection effects.
        let owner = self
            .ground_state_pair_owner(board, skeleton)
            .map_err(E::Session)?;
        let registered = actor.registered_physical_allocation().map_err(E::Actor)?;
        if !Arc::ptr_eq(registered.owner(), &owner) {
            return Err(E::ForeignActor);
        }
        let destination = match self.try_update_pair_system_logic(board, skeleton, refs) {
            Ok(()) => return Ok(()),
            Err(SystemLogicError::PendingTransition(next)) => {
                let current = owner
                    .physics_state()
                    .lock()
                    .map_err(|_| E::Selection(SystemLogicError::Poisoned))?
                    .current;
                selected_transition(current, next)
                    .ok_or(E::Selection(SystemLogicError::PendingTransition(next)))?
            }
            Err(error) => return Err(E::Selection(error)),
        };
        // Controlled corpus override AFTER actual selection; native Exit/Enter and
        // the sole success-only request-clear suffix remain unchanged.
        #[cfg(test)]
        let destination = requested_transition.take().unwrap_or(destination);
        self.try_transition_pair_ground(
            board,
            skeleton,
            actor,
            destination,
            refs,
            ground,
            numeric,
            cosine,
        )?;
        owner
            .wipeout()
            .lock()
            .map_err(|_| E::Selection(SystemLogicError::Poisoned))?
            .try_clear_requests()
            .map_err(|error| E::Selection(SystemLogicError::Wipeout(error)))
    }

    /// Selected `SetPhysicsState` entry; caller-owned state selection/request
    /// clearing is not repeated here. Destination does not bypass Exit/Enter.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair, Actor, destination and primitive source services"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered transition fields and concrete Exit/Enter preserve failure prefixes"
    )]
    pub(crate) fn try_transition_pair_ground<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        mut actor: &ActorConstruction,
        destination: GroundDestination,
        refs: &InputCvarReferences,
        ground: Option<&crate::static_ground_pool::StaticGroundPool>,
        numeric: &mut N,
        cosine: &mut impl FnMut(f32) -> Result<f32, N::Error>,
    ) -> Result<(), GroundToAirError<N::Error>>
    where
        N::Error: std::fmt::Debug,
    {
        use GroundToAirError as E;
        let owner = self
            .ground_state_pair_owner(board, skeleton)
            .map_err(E::Session)?;
        if !Arc::ptr_eq(
            actor
                .registered_physical_allocation()
                .map_err(E::Actor)?
                .owner(),
            &owner,
        ) {
            return Err(E::ForeignActor);
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .processed(),
        );
        let previous = owner
            .physics_state()
            .lock()
            .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
            .current;
        let (previous_id, previous_category) = match previous {
            CurrentPhysicalState::Ground => (PHYSICS_STATE_PHYSICS_GROUND, PHYSICS_CATEGORY_GROUND),
            CurrentPhysicalState::SlideGround => (PHYSICS_STATE_SLIDE_GROUND, PHYSICS_CATEGORY_GROUND),
            CurrentPhysicalState::RevertGround => (PHYSICS_STATE_REVERT_GROUND, PHYSICS_CATEGORY_GROUND),
            CurrentPhysicalState::GroundAnimation => (PHYSICS_STATE_GROUND_ANIMATION, PHYSICS_CATEGORY_GROUND),
            CurrentPhysicalState::Skitching => (PHYSICS_STATE_SKITCHING, PHYSICS_CATEGORY_GROUND),
            CurrentPhysicalState::FollowPath => (PHYSICS_STATE_FOLLOW_PATH, PHYSICS_CATEGORY_GROUND),
            CurrentPhysicalState::Air => (PHYSICS_STATE_PHYSICS_AIR, PHYSICS_CATEGORY_AIR),
            CurrentPhysicalState::KnownAir => (PHYSICS_STATE_KNOWN_AIR, PHYSICS_CATEGORY_AIR),
            CurrentPhysicalState::WipeoutGround => (PHYSICS_STATE_WIPEOUT_GROUND, PHYSICS_CATEGORY_WIPEOUT),
            CurrentPhysicalState::OffBoard => (PHYSICS_STATE_OFFBOARD, PHYSICS_CATEGORY_OFFBOARD),
            CurrentPhysicalState::OffBoardPushing => (PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_CATEGORY_OFFBOARD),
            CurrentPhysicalState::OffBoardAir => (PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_CATEGORY_OFFBOARD),
            // SetPhysicsState82dff1e0 stores GetType and its category as last:
            // OffBoard_Jump.GetType82d98590 is the folded li r3,0x1f6.
            CurrentPhysicalState::OffBoardJump => (PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_CATEGORY_OFFBOARD),
            CurrentPhysicalState::LandingOnDeck => (PHYSICS_STATE_LANDINGONDECK, PHYSICS_CATEGORY_OFFBOARD),
            CurrentPhysicalState::OffBoardLocomotion => (PHYSICS_STATE_OFFBOARD_LOCOMOTION, PHYSICS_CATEGORY_OFFBOARD),
            CurrentPhysicalState::Teleporting => (PHYSICS_STATE_TELEPORTING, PHYSICS_CATEGORY_OTHER),
            CurrentPhysicalState::Boneless => (PHYSICS_STATE_BONELESS, PHYSICS_CATEGORY_GROUNDTRICK),
            CurrentPhysicalState::FootPlant => (PHYSICS_STATE_FOOTPLANT, PHYSICS_CATEGORY_GROUNDTRICK),
            CurrentPhysicalState::HandPlant => (PHYSICS_STATE_HANDPLANT, PHYSICS_CATEGORY_GROUNDTRICK),
            // GetPhysicsCategory82d83b80: 400..499 grind, 700.. other.
            CurrentPhysicalState::Grind(kind) => (kind.id(), PHYSICS_CATEGORY_GRIND),
            CurrentPhysicalState::Nonspecific => (PHYSICS_STATE_NONSPECIFIC, PHYSICS_CATEGORY_OTHER),
            _ => return Err(E::WrongStage),
        };
        {
            let mut state = owner
                .physics_state()
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?;
            if selected_transition(state.current, destination.id()).is_none()
                || state.stage != PhysicalStateStage::EntryComplete
            {
                return Err(E::WrongStage);
            }
            state.stage = PhysicalStateStage::Controller;
            // Native EnterNewPhysicsState82dd1d50 starts all500..504,
            // including transitions within the category (Start is idempotent).
            if (PHYSICS_STATE_OFFBOARD..=PHYSICS_STATE_OFFBOARD_LOCOMOTION).contains(&destination.id()) {
                self.try_start_pair_offboard_controller(&owner, board, skeleton, refs, numeric)
                    .map_err(E::OffBoardStart)?;
            } else if previous_category == PHYSICS_CATEGORY_OFFBOARD {
                self.try_stop_pair_offboard_controller(&owner, board, skeleton, refs)
                    .map_err(E::OffBoardStop)?;
            } else {
                owner
                    .skateboard_controller()
                    .lock()
                    .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                    .try_enter_physics_state(destination.id())
                    .map_err(|error| E::Session(PhysicalStateSessionError::Controller(error)))?;
            }
            state.stage = PhysicalStateStage::TransitionFields;
            state.state_count = Some(0);
            state.state_timer = Some(0.0);
            // Source82dff1cc..204 publishes transition fields BEFORE Ground.Exit.
            processed
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .physics_state_fields_mut()
                .state_timer = Some(0.0);
            processed
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .physics_state_fields_mut()
                .last = Some(previous_id);
            processed
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .physics_state_fields_mut()
                .last_category = Some(previous_category);
            processed
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .physics_state_fields_mut()
                .next = Some(destination.id());
            processed
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .physics_state_fields_mut()
                .current = Some(destination.id());
            state.stage = match previous {
                CurrentPhysicalState::Air => PhysicalStateStage::AirExit,
                CurrentPhysicalState::KnownAir => PhysicalStateStage::KnownAirExit,
                CurrentPhysicalState::GroundAnimation => PhysicalStateStage::GroundAnimationExit,
                CurrentPhysicalState::Skitching => PhysicalStateStage::SkitchingExit,
                CurrentPhysicalState::FollowPath => PhysicalStateStage::FollowPathExit,
                CurrentPhysicalState::SlideGround => PhysicalStateStage::SlideGroundExit,
                CurrentPhysicalState::RevertGround => PhysicalStateStage::RevertGroundExit,
                CurrentPhysicalState::WipeoutGround => PhysicalStateStage::WipeoutExit,
                CurrentPhysicalState::OffBoard => PhysicalStateStage::OffBoardExit,
                CurrentPhysicalState::OffBoardPushing => PhysicalStateStage::OffBoardPushingExit,
                CurrentPhysicalState::OffBoardAir => PhysicalStateStage::OffBoardAirExit,
                CurrentPhysicalState::OffBoardJump => PhysicalStateStage::OffBoardJumpExit,
                CurrentPhysicalState::LandingOnDeck => PhysicalStateStage::LandingOnDeckExit,
                CurrentPhysicalState::OffBoardLocomotion => {
                    PhysicalStateStage::OffBoardLocomotionExit
                }
                CurrentPhysicalState::Teleporting => PhysicalStateStage::TeleportingExit,
                CurrentPhysicalState::Boneless => PhysicalStateStage::BonelessExit,
                CurrentPhysicalState::FootPlant => PhysicalStateStage::FootPlantExit,
                CurrentPhysicalState::HandPlant => PhysicalStateStage::HandPlantExit,
                CurrentPhysicalState::Grind(_) => PhysicalStateStage::GrindExit,
                CurrentPhysicalState::Nonspecific => PhysicalStateStage::NonspecificExit,
                _ => PhysicalStateStage::GroundExit,
            };
        }
        if previous == CurrentPhysicalState::KnownAir {
            self.try_exit_pair_known_air(&owner, board, destination.id(), numeric)
                .map_err(E::KnownAir)?;
        } else if previous == CurrentPhysicalState::Air {
            // Air.Exit82d84190: clear useCOM before the same Reckoning reset.
            owner
                .physics_state()
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .air
                .as_mut()
                .ok_or(E::WrongStage)?
                .fields
                .use_com_velocity = false;
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .try_reset_body_spin()
                .map_err(|e| E::Session(PhysicalStateSessionError::Reckoning(e)))?;
        } else if previous == CurrentPhysicalState::SlideGround {
            self.try_exit_pair_slide_ground(&owner, board, skeleton)
                .map_err(E::Slide)?;
        } else if previous == CurrentPhysicalState::GroundAnimation {
            self.try_exit_pair_ground_animation(board, skeleton, numeric)
                .map_err(E::GroundAnimationExit)?;
        } else if previous == CurrentPhysicalState::WipeoutGround {
            // WipeoutGround.Exit82d84d70: board SetStandard precedes the flag clear.
            self.try_set_board_standard(board).map_err(E::WipeoutExit)?;
            owner
                .physics_state()
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .wipeout_ground
                .as_mut()
                .ok_or(E::WrongStage)?
                .fields
                .prevent_respawn = false;
        } else if previous == CurrentPhysicalState::OffBoardPushing {
            self.try_exit_pair_offboard_pushing(&owner, board, skeleton)
                .map_err(E::OffBoardPushing)?;
        } else if previous == CurrentPhysicalState::OffBoardLocomotion {
            self.try_exit_pair_offboard_locomotion(&owner, board, skeleton)
                .map_err(E::OffBoardLocomotion)?;
        } else if previous == CurrentPhysicalState::LandingOnDeck {
            self.try_exit_pair_landing_state(&owner, board, skeleton)
                .map_err(E::LandingOnDeck)?;
        } else if previous == CurrentPhysicalState::OffBoardAir {
            self.try_exit_pair_offboard_air(&owner, board, skeleton)
                .map_err(E::OffBoardAir)?;
        } else if previous == CurrentPhysicalState::OffBoard {
            // OffBoard.Exit82d86cc8: Location.ClearData precedes the
            // JumpingTest finalizer. The reached original-owner path has
            // constructor-null batches; non-null batches need their native
            // Sync/Release or CalculateResults services before proceeding.
            {
                let states = owner
                    .remaining_states()
                    .lock()
                    .map_err(|_| E::OffBoardExit("state owner lock poisoned"))?;
                let Some(StatePayload::OffBoard(offboard)) = states
                    .states
                    .get(&StateKind::OffBoard)
                    .filter(|state| state.published)
                    .and_then(|state| state.value.as_ref())
                else {
                    return Err(E::OffBoardExit("published OffBoard state unavailable"));
                };
                let binding = &offboard.base.physical;
                if !binding.board.same_as(board)
                    || !binding.skeleton.same_as(skeleton)
                    || !binding
                        .owner
                        .upgrade()
                        .is_some_and(|bound| Arc::ptr_eq(&bound, &owner))
                    || !Arc::ptr_eq(&offboard.base.processed, &processed)
                {
                    return Err(E::OffBoardExit("foreign OffBoard state"));
                }
                for (link, member) in [
                    (
                        &offboard.offboard_location_test,
                        OwnerMember::OffboardLocationTest,
                    ),
                    (
                        &offboard.offboard_jumping_test,
                        OwnerMember::OffboardJumpingTest,
                    ),
                ] {
                    let Some(link) = link.as_ref().and_then(Option::as_ref) else {
                        return Err(E::OffBoardExit("OffBoard collaborator link unavailable"));
                    };
                    if link.member != member
                        || !link
                            .owner
                            .upgrade()
                            .is_some_and(|bound| Arc::ptr_eq(&bound, &owner))
                    {
                        return Err(E::OffBoardExit("foreign OffBoard collaborator link"));
                    }
                }
            }
            let mut remaining = owner
                .remaining()
                .lock()
                .map_err(|_| E::OffBoardExit("retained owner lock poisoned"))?;
            let location = remaining
                .location
                .as_mut()
                .ok_or(E::OffBoardExit("location test unavailable"))?;
            if !location.world.matches(&owner) {
                return Err(E::OffBoardExit("foreign location test"));
            }
            if location.line_batch.is_some() || location.trajectory_batch.is_some() {
                let queries = self
                    .world_line_queries
                    .as_mut()
                    .ok_or(E::OffBoardExit("location query world unavailable"))?;
                if !queries.belongs_to(owner.manager()) {
                    return Err(E::OffBoardExit("foreign location query world"));
                }
                location
                    .clear_data(queries)
                    .map_err(|_| E::OffBoardExit("location query Sync/Release failed"))?;
            } else {
                location.sync_allowed = false;
            }
            let jumping = remaining
                .jumping
                .as_mut()
                .ok_or(E::OffBoardExit("jumping test unavailable"))?;
            if !jumping.world.matches(&owner) {
                return Err(E::OffBoardExit("foreign jumping test"));
            }
            // ~OffboardJumpingTest82dd5fd0: a pending batch completes through
            // CalculateResults82dd5660; a null batch returns immediately.
            if jumping.line_batch.is_some() {
                let queries = self
                    .world_line_queries
                    .as_mut()
                    .ok_or(E::OffBoardExit("jumping query world unavailable"))?;
                if !queries.belongs_to(owner.manager()) {
                    return Err(E::OffBoardExit("foreign jumping query world"));
                }
                jumping
                    .finish_pending(queries, &mut refs.clone(), numeric)
                    .map_err(E::OffBoardJumping)?;
            }
        } else if previous == CurrentPhysicalState::OffBoardJump {
            // OffBoard_Jump.Exit vtable8234a60c+c is the shared BLR825187c0.
        } else if previous == CurrentPhysicalState::Skitching {
            // Skitching vtable8234a6a8+c: shared BLR825187c0.
        } else if previous == CurrentPhysicalState::FollowPath {
            // FollowPath vtable82371678+c: shared BLR825187c0.
        } else if previous == CurrentPhysicalState::RevertGround {
            // Revert.Exit8234a3d8 points to the native BLR825187c0.
        } else if previous == CurrentPhysicalState::Teleporting {
            // Teleporting.Exit resolves to the shared empty BLR.
        } else if let CurrentPhysicalState::Grind(kind) = previous {
            self.try_exit_pair_grind(&owner, board, skeleton, kind)
                .map_err(E::Grind)?;
        } else if previous == CurrentPhysicalState::Nonspecific {
            // Nonspecific.Exit vtable82371630+c is the shared BLR825187c0.
        } else if previous == CurrentPhysicalState::Boneless {
            // Boneless.Exit vtable8234a578+c is the shared BLR825187c0.
        } else if previous == CurrentPhysicalState::HandPlant {
            // HandPlant.Exit82d893b0 is empty; manager survives until Ground Launch.
        } else if previous == CurrentPhysicalState::FootPlant {
            // FootPlant.Exit82d89468 tail-calls FootPlantManager::Reset82db6760;
            // the future's readiness is retained.
            owner
                .footplant()
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
                .try_reset(&owner, skeleton)
                .map_err(|e| {
                    E::FootPlant(crate::simulation_assembly_backend::constraint_session::footplant_air::FootPlantUpdateError::Service(
                        "FootPlantManager.Reset",
                        format!("{e:?}"),
                    ))
                })?;
        } else {
            self.try_exit_pair_ground(board, skeleton, numeric)
                .map_err(E::Exit)?;
        }
        {
            let mut state = owner
                .physics_state()
                .lock()
                .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?;
            state.stage = PhysicalStateStage::ActorQuery;
            super::transition_actor_queries(&mut state, &processed, &mut actor)
                .map_err(E::Session)?;
            // Source82dff3b8 (Air) /82dff748 (Wipeout), before shared Enter.
            match destination {
                GroundDestination::Ground => {
                    state.current = CurrentPhysicalState::Ground;
                    state.stage = PhysicalStateStage::GroundSelected;
                }
                GroundDestination::RevertGround => {
                    state.current = CurrentPhysicalState::RevertGround;
                    state.stage = PhysicalStateStage::RevertGroundSelected;
                }
                GroundDestination::SlideGround => {
                    state.current = CurrentPhysicalState::SlideGround;
                    state.stage = PhysicalStateStage::SlideGroundSelected;
                }
                GroundDestination::GroundAnimation => {
                    state.current = CurrentPhysicalState::GroundAnimation;
                    state.stage = PhysicalStateStage::GroundAnimationSelected;
                }
                GroundDestination::Skitching => {
                    state.current = CurrentPhysicalState::Skitching;
                    state.stage = PhysicalStateStage::SkitchingSelected;
                }
                GroundDestination::FollowPath => {
                    state.current = CurrentPhysicalState::FollowPath;
                    state.stage = PhysicalStateStage::FollowPathSelected;
                }
                GroundDestination::KnownAir => {
                    state.current = CurrentPhysicalState::KnownAir;
                    state.stage = PhysicalStateStage::KnownAirSelected;
                }
                GroundDestination::Air => {
                    state.current = CurrentPhysicalState::Air;
                    state.stage = PhysicalStateStage::AirSelected;
                }
                GroundDestination::Wipeout => {
                    state.current = CurrentPhysicalState::WipeoutGround;
                    state.stage = PhysicalStateStage::WipeoutSelected;
                }
                GroundDestination::OffBoardPushing => {
                    state.current = CurrentPhysicalState::OffBoardPushing;
                    state.stage = PhysicalStateStage::OffBoardPushingSelected;
                }
                GroundDestination::OffBoardLocomotion => {
                    state.current = CurrentPhysicalState::OffBoardLocomotion;
                    state.stage = PhysicalStateStage::OffBoardLocomotionSelected;
                }
                GroundDestination::LandingOnDeck => {
                    state.current = CurrentPhysicalState::LandingOnDeck;
                    state.stage = PhysicalStateStage::LandingOnDeckSelected;
                }
                GroundDestination::OffBoardAir => {
                    state.current = CurrentPhysicalState::OffBoardAir;
                    state.stage = PhysicalStateStage::OffBoardAirSelected;
                }
                GroundDestination::OffBoardJump => {
                    state.current = CurrentPhysicalState::OffBoardJump;
                    state.stage = PhysicalStateStage::OffBoardJumpSelected;
                }
                GroundDestination::OffBoard => {
                    state.current = CurrentPhysicalState::OffBoard;
                    state.stage = PhysicalStateStage::OffBoardSelected;
                }
                GroundDestination::Teleporting => {
                    state.current = CurrentPhysicalState::Teleporting;
                    state.stage = PhysicalStateStage::TeleportingSelected;
                }
                GroundDestination::Boneless => {
                    state.current = CurrentPhysicalState::Boneless;
                    state.stage = PhysicalStateStage::BonelessSelected;
                }
                GroundDestination::HandPlant => {
                    state.current = CurrentPhysicalState::HandPlant;
                    state.stage = PhysicalStateStage::HandPlantSelected;
                }
                GroundDestination::FootPlant => {
                    state.current = CurrentPhysicalState::FootPlant;
                    state.stage = PhysicalStateStage::FootPlantSelected;
                }
                GroundDestination::Grind(kind) => {
                    state.current = CurrentPhysicalState::Grind(kind);
                    state.stage = PhysicalStateStage::GrindSelected;
                }
                GroundDestination::Nonspecific => {
                    state.current = CurrentPhysicalState::Nonspecific;
                    state.stage = PhysicalStateStage::NonspecificSelected;
                }
            }
        }
        match destination {
            GroundDestination::Ground => {
                let mut state = owner
                    .physics_state()
                    .lock()
                    .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?;
                let ground = state.ground.as_ref().ok_or(E::WrongStage)?;
                if !ground.bindings.board.same_as(board)
                    || !ground.bindings.skeleton.same_as(skeleton)
                    || !ground
                        .bindings
                        .owner
                        .upgrade()
                        .is_some_and(|p| Arc::ptr_eq(&p, &owner))
                {
                    return Err(E::Session(PhysicalStateSessionError::ForeignBindings));
                }
                self.enter_selected_ground(&owner, &mut state, board, skeleton, refs, numeric)
                    .map_err(E::Session)?;
            }
            GroundDestination::RevertGround => self
                .try_enter_pair_revert_ground(&owner, board, skeleton)
                .map_err(E::Revert)?,
            GroundDestination::SlideGround => self
                .try_enter_pair_slide_ground(&owner, board, skeleton)
                .map_err(E::Slide)?,
            GroundDestination::GroundAnimation => self
                .try_enter_pair_ground_animation(board, skeleton, refs, numeric)
                .map_err(E::GroundAnimationEntry)?,
            GroundDestination::Skitching => self
                .try_enter_pair_skitch(&owner, board, skeleton)
                .map_err(E::Skitching)?,
            GroundDestination::FollowPath => self
                .try_enter_pair_follow_path(&owner, board, skeleton)
                .map_err(E::FollowPath)?,
            GroundDestination::KnownAir => self
                .try_enter_pair_known_air(&owner, board, skeleton, refs, numeric)
                .map_err(E::KnownAir)?,
            GroundDestination::Air => self
                .try_enter_pair_air(board, skeleton, refs, numeric)
                .map_err(E::Entry)?,
            GroundDestination::Wipeout => self
                .try_enter_pair_wipeout(board, skeleton, refs, numeric, cosine)
                .map_err(E::WipeoutEntry)?,
            GroundDestination::OffBoardPushing => self
                .try_enter_pair_offboard_pushing(&owner, board, skeleton, refs, numeric)
                .map_err(E::OffBoardPushing)?,
            GroundDestination::OffBoardLocomotion => self
                .try_enter_pair_offboard_locomotion(&owner, board, skeleton, numeric)
                .map_err(E::OffBoardLocomotion)?,
            GroundDestination::LandingOnDeck => self
                .try_enter_pair_landing_state(&owner, board, skeleton, refs, numeric)
                .map_err(E::LandingOnDeck)?,
            GroundDestination::OffBoardAir => self
                .try_enter_pair_offboard_air(
                    &owner,
                    board,
                    skeleton,
                    ground.ok_or(E::MissingAirGround)?,
                    refs,
                    numeric,
                )
                .map_err(E::OffBoardAir)?,
            GroundDestination::OffBoardJump => self
                .try_enter_pair_offboard_jump(&owner, board, skeleton, numeric)
                .map_err(E::OffBoardJump)?,
            GroundDestination::OffBoard => self
                .try_enter_pair_offboard(board, skeleton, refs, numeric)
                .map_err(E::OffBoardEntry)?,
            GroundDestination::Boneless => self
                .try_enter_pair_boneless(&owner, board, skeleton, refs, numeric)
                .map_err(E::Boneless)?,
            GroundDestination::HandPlant => self
                .try_enter_pair_handplant(
                    &owner,
                    board,
                    skeleton,
                    ground.ok_or(E::MissingAirGround)?,
                    refs,
                    numeric,
                )
                .map_err(E::HandPlant)?,
            GroundDestination::FootPlant => self
                .try_enter_pair_foot_plant(&owner, board, skeleton, refs, numeric)
                .map_err(E::FootPlant)?,
            GroundDestination::Grind(kind) => self
                .try_enter_pair_grind(&owner, board, skeleton, kind, refs)
                .map_err(E::Grind)?,
            GroundDestination::Nonspecific => self
                .try_enter_pair_nonspecific(&owner, board, skeleton)
                .map_err(E::Grind)?,
            GroundDestination::Teleporting => {
                let mut states = owner
                    .remaining_states()
                    .lock()
                    .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?;
                let Some(StatePayload::Teleporting(teleporting)) = states
                    .states
                    .get_mut(&StateKind::Teleporting)
                    .filter(|state| state.published)
                    .and_then(|state| state.value.as_mut())
                else {
                    return Err(E::WrongStage);
                };
                let binding = &teleporting.base.physical;
                if !binding.board.same_as(board)
                    || !binding.skeleton.same_as(skeleton)
                    || !binding
                        .owner
                        .upgrade()
                        .is_some_and(|p| Arc::ptr_eq(&p, &owner))
                    || !Arc::ptr_eq(&teleporting.base.processed, &processed)
                {
                    return Err(E::Session(PhysicalStateSessionError::ForeignBindings));
                }
                teleporting.enter();
            }
        }
        owner
            .physics_state()
            .lock()
            .map_err(|_| E::Session(PhysicalStateSessionError::Poisoned))?
            .stage = PhysicalStateStage::EntryComplete;
        Ok(())
    }
}

// Only source-audited predecessor/destination pairs are admitted.
fn selected_transition(current: CurrentPhysicalState, next: i32) -> Option<GroundDestination> {
    match (current, next) {
        // CalcSuggestedState82ddbd58: an AI adjust without a loaded local world selects
        // FollowPath105 from any state after the shared teleport prefix; FollowPath leaves for
        // Ground once the AI input reports its local world loaded.
        (CurrentPhysicalState::Unselected | CurrentPhysicalState::Sleeping | CurrentPhysicalState::FollowPath, PHYSICS_STATE_FOLLOW_PATH) => {
            None
        }
        (_, PHYSICS_STATE_FOLLOW_PATH) => Some(GroundDestination::FollowPath),
        (CurrentPhysicalState::FollowPath, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::FollowPath, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        (CurrentPhysicalState::Ground, PHYSICS_STATE_SKITCHING) => Some(GroundDestination::Skitching),
        (CurrentPhysicalState::Skitching, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::Skitching, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::Skitching, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        // SetPhysicsState82dff180: state101 has independent Enter/Exit and uses
        // the same category100. Admit only destinations whose bodies are owned.
        (
            CurrentPhysicalState::Ground
            | CurrentPhysicalState::GroundAnimation
            | CurrentPhysicalState::Air
            | CurrentPhysicalState::KnownAir,
            PHYSICS_STATE_SLIDE_GROUND,
        ) => Some(GroundDestination::SlideGround),
        (CurrentPhysicalState::Ground | CurrentPhysicalState::SlideGround, PHYSICS_STATE_REVERT_GROUND) => {
            Some(GroundDestination::RevertGround)
        }
        (CurrentPhysicalState::RevertGround, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::RevertGround, PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::RevertGround, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::RevertGround, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        (CurrentPhysicalState::SlideGround, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::SlideGround, PHYSICS_STATE_GROUND_ANIMATION) => Some(GroundDestination::GroundAnimation),
        (CurrentPhysicalState::SlideGround, PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::SlideGround, PHYSICS_STATE_KNOWN_AIR) => Some(GroundDestination::KnownAir),
        (CurrentPhysicalState::SlideGround, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::SlideGround, PHYSICS_STATE_OFFBOARD) => Some(GroundDestination::OffBoard),
        (CurrentPhysicalState::SlideGround, PHYSICS_STATE_OFFBOARD_AIR) => Some(GroundDestination::OffBoardAir),
        (CurrentPhysicalState::SlideGround, PHYSICS_STATE_OFFBOARD_LOCOMOTION) => Some(GroundDestination::OffBoardLocomotion),
        (CurrentPhysicalState::SlideGround, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        (
            CurrentPhysicalState::Ground
            | CurrentPhysicalState::GroundAnimation
            | CurrentPhysicalState::Air,
            PHYSICS_STATE_KNOWN_AIR,
        ) => Some(GroundDestination::KnownAir),
        (CurrentPhysicalState::KnownAir, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::KnownAir, PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::KnownAir, PHYSICS_STATE_OFFBOARD) => Some(GroundDestination::OffBoard),
        (CurrentPhysicalState::KnownAir, PHYSICS_STATE_OFFBOARD_AIR) => Some(GroundDestination::OffBoardAir),
        (CurrentPhysicalState::KnownAir, PHYSICS_STATE_OFFBOARD_LOCOMOTION) => Some(GroundDestination::OffBoardLocomotion),
        (CurrentPhysicalState::KnownAir, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        (CurrentPhysicalState::KnownAir | CurrentPhysicalState::Air, PHYSICS_STATE_WIPEOUT_GROUND) => {
            Some(GroundDestination::Wipeout)
        }
        (
            CurrentPhysicalState::Ground
            | CurrentPhysicalState::GroundAnimation
            | CurrentPhysicalState::Air
            | CurrentPhysicalState::OffBoard
            | CurrentPhysicalState::OffBoardAir,
            PHYSICS_STATE_OFFBOARD_LOCOMOTION,
        ) => Some(GroundDestination::OffBoardLocomotion),
        (CurrentPhysicalState::OffBoardLocomotion, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        // CalcSuggestedState82ddcb0c/ccb8 enters Pushing from stationary or
        // locomotion. Pushing82ddcc60..cc8c only exits to OffBoard or wipeout;
        // the common teleport prefix remains applicable.
        (CurrentPhysicalState::OffBoard | CurrentPhysicalState::OffBoardLocomotion, PHYSICS_STATE_OFFBOARD_PUSHING) => {
            Some(GroundDestination::OffBoardPushing)
        }
        (CurrentPhysicalState::OffBoardPushing, PHYSICS_STATE_OFFBOARD) => Some(GroundDestination::OffBoard),
        (CurrentPhysicalState::OffBoardPushing, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::OffBoardPushing, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        (CurrentPhysicalState::OffBoardLocomotion, PHYSICS_STATE_OFFBOARD) => Some(GroundDestination::OffBoard),
        (CurrentPhysicalState::OffBoardLocomotion, PHYSICS_STATE_OFFBOARD_AIR) => Some(GroundDestination::OffBoardAir),
        (CurrentPhysicalState::OffBoardLocomotion, PHYSICS_STATE_LANDINGONDECK) => Some(GroundDestination::LandingOnDeck),
        (CurrentPhysicalState::OffBoardLocomotion, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (
            CurrentPhysicalState::Ground
            | CurrentPhysicalState::GroundAnimation
            | CurrentPhysicalState::Air
            | CurrentPhysicalState::OffBoard
            | CurrentPhysicalState::OffBoardAir,
            PHYSICS_STATE_LANDINGONDECK,
        ) => Some(GroundDestination::LandingOnDeck),
        (CurrentPhysicalState::LandingOnDeck, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::LandingOnDeck, PHYSICS_STATE_OFFBOARD_AIR) => Some(GroundDestination::OffBoardAir),
        (CurrentPhysicalState::LandingOnDeck, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (
            CurrentPhysicalState::Ground
            | CurrentPhysicalState::GroundAnimation
            | CurrentPhysicalState::Air,
            PHYSICS_STATE_OFFBOARD_AIR,
        ) => Some(GroundDestination::OffBoardAir),
        (CurrentPhysicalState::OffBoardAir, PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::OffBoardAir, PHYSICS_STATE_OFFBOARD) => Some(GroundDestination::OffBoard),
        (CurrentPhysicalState::OffBoardAir, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        // CalcSuggestedState82ddcb7c: OffBoard with OffboardJumpInfoReady (8d4
        // 0x20000000) and the standing-jump or OffBoardAir request (8cc 0x100000/0x100).
        (CurrentPhysicalState::OffBoard, PHYSICS_STATE_OFFBOARD_JUMP) => Some(GroundDestination::OffBoardJump),
        // OffBoard_Jump reports GetType502, so CheckForStateChange82e005a8 runs the
        // OffBoard_Air case and a 502 suggestion keeps the jump. Its Exit is the
        // shared BLR825187c0 for every destination.
        (CurrentPhysicalState::OffBoardJump, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::OffBoardJump, PHYSICS_STATE_OFFBOARD_LOCOMOTION) => Some(GroundDestination::OffBoardLocomotion),
        (CurrentPhysicalState::OffBoardJump, PHYSICS_STATE_OFFBOARD) => Some(GroundDestination::OffBoard),
        (CurrentPhysicalState::OffBoardJump, PHYSICS_STATE_LANDINGONDECK) => Some(GroundDestination::LandingOnDeck),
        (CurrentPhysicalState::OffBoardJump, PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::OffBoardJump, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        (CurrentPhysicalState::GroundAnimation, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::Air, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::Ground | CurrentPhysicalState::GroundAnimation, PHYSICS_STATE_PHYSICS_AIR) => {
            Some(GroundDestination::Air)
        }
        // CalcSuggestedState82ddc368..c370: GroundAnimation's wipeout flag
        // branches to the same return300 at82ddc104. SetPhysicsState82dff180
        // runs its existing Exit82d83ed0 before WipeoutGround.Enter82d8eb18.
        (CurrentPhysicalState::Ground | CurrentPhysicalState::GroundAnimation, PHYSICS_STATE_WIPEOUT_GROUND) => {
            Some(GroundDestination::Wipeout)
        }
        (CurrentPhysicalState::OffBoard, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        // CalcSuggestedState82ddcb7c: OffBoard with the board held and a valid
        // mount. StopController, OffBoard.Exit, then Air.Enter with last500.
        (CurrentPhysicalState::OffBoard, PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::WipeoutGround, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        // TeleportPlayerNow82e00994 explicitly calls SetPhysicsState(Ground)
        // after ResetAll and before restoring normal board/skeleton collision.
        (CurrentPhysicalState::Teleporting, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::Ground, PHYSICS_STATE_GROUND_ANIMATION) => Some(GroundDestination::GroundAnimation),
        (CurrentPhysicalState::Air, PHYSICS_STATE_GROUND_ANIMATION) => Some(GroundDestination::GroundAnimation),
        (CurrentPhysicalState::Ground | CurrentPhysicalState::GroundAnimation, PHYSICS_STATE_OFFBOARD) => {
            Some(GroundDestination::OffBoard)
        }
        // CalcSuggestedState82ddc384: GroundAnimation with 8d0 0x20000. Boneless
        // exits per its case82ddd070 or the shared teleport prefix82ddbf50.
        (CurrentPhysicalState::GroundAnimation, PHYSICS_STATE_BONELESS) => Some(GroundDestination::Boneless),
        (CurrentPhysicalState::Boneless, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::Boneless, PHYSICS_STATE_KNOWN_AIR) => Some(GroundDestination::KnownAir),
        (CurrentPhysicalState::Boneless, PHYSICS_STATE_OFFBOARD_AIR) => Some(GroundDestination::OffBoardAir),
        (CurrentPhysicalState::Boneless, PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::Boneless, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        // CalcSuggestedState82ddc88c: KnownAir with 8d0 0x04000000. FootPlant
        // exits per its case82ddd24c or the shared teleport prefix82ddbf50.
        (CurrentPhysicalState::Ground, PHYSICS_STATE_HANDPLANT) => Some(GroundDestination::HandPlant),
        (CurrentPhysicalState::HandPlant, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::HandPlant, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::HandPlant, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        (CurrentPhysicalState::KnownAir, PHYSICS_STATE_FOOTPLANT) => Some(GroundDestination::FootPlant),
        (CurrentPhysicalState::FootPlant, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::FootPlant, PHYSICS_STATE_KNOWN_AIR) => Some(GroundDestination::KnownAir),
        (CurrentPhysicalState::FootPlant, PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::FootPlant, PHYSICS_STATE_TELEPORTING) => Some(GroundDestination::Teleporting),
        // CalcSuggestedState82ddbf50: the shared teleport prefix returns 702 from
        // every state before the per-state switch. These Exits never read the
        // destination: PhysicsGround82d84a80, GroundAnimation82d83ed0,
        // PhysicsAir82d84190, OffBoard82d86cc8, OffBoard_Air82d88318,
        // LandingOnDeck82d897f8 and the Locomotion BLR825187c0.
        (
            CurrentPhysicalState::Ground
            | CurrentPhysicalState::GroundAnimation
            | CurrentPhysicalState::Air
            | CurrentPhysicalState::OffBoard
            | CurrentPhysicalState::OffBoardAir
            | CurrentPhysicalState::OffBoardLocomotion
            | CurrentPhysicalState::LandingOnDeck,
            PHYSICS_STATE_TELEPORTING,
        ) => Some(GroundDestination::Teleporting),
        // CalcSuggestedState CheckForGrind82dd3ac8 returns from Ground82ddc294,
        // SlideGround82ddc040, Air82ddc4f0, KnownAir82ddc948, every grind
        // type82ddcad8 and Nonspecific82ddd108.
        (
            CurrentPhysicalState::Ground
            | CurrentPhysicalState::SlideGround
            | CurrentPhysicalState::Air
            | CurrentPhysicalState::KnownAir
            | CurrentPhysicalState::Grind(_)
            | CurrentPhysicalState::Nonspecific,
            PHYSICS_STATE_GRIND_BOARDSLIDE..=PHYSICS_STATE_GRIND_BACKSLASH,
        ) => GrindKind::from_id(next).map(GroundDestination::Grind),
        // Grind cases82ddcaa8..cae0: wipeout, OffBoardAir, the grind jump's
        // Air/KnownAir, or Nonspecific when CheckForGrind returns zero.
        (CurrentPhysicalState::Grind(_), PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::Grind(_), PHYSICS_STATE_OFFBOARD_AIR) => Some(GroundDestination::OffBoardAir),
        (CurrentPhysicalState::Grind(_), PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::Grind(_), PHYSICS_STATE_KNOWN_AIR) => Some(GroundDestination::KnownAir),
        (CurrentPhysicalState::Grind(_), PHYSICS_STATE_NONSPECIFIC) => Some(GroundDestination::Nonspecific),
        // Nonspecific case82ddd0c4..d1a4.
        (CurrentPhysicalState::Nonspecific, PHYSICS_STATE_WIPEOUT_GROUND) => Some(GroundDestination::Wipeout),
        (CurrentPhysicalState::Nonspecific, PHYSICS_STATE_PHYSICS_AIR) => Some(GroundDestination::Air),
        (CurrentPhysicalState::Nonspecific, PHYSICS_STATE_KNOWN_AIR) => Some(GroundDestination::KnownAir),
        (CurrentPhysicalState::Nonspecific, PHYSICS_STATE_PHYSICS_GROUND) => Some(GroundDestination::Ground),
        (CurrentPhysicalState::Nonspecific, PHYSICS_STATE_GROUND_ANIMATION) => Some(GroundDestination::GroundAnimation),
        // The shared teleport prefix82ddbf50; grind and Nonspecific Exits never
        // read the destination.
        (CurrentPhysicalState::Grind(_) | CurrentPhysicalState::Nonspecific, PHYSICS_STATE_TELEPORTING) => {
            Some(GroundDestination::Teleporting)
        }
        _ => None,
    }
}
#[cfg(test)]
mod transition_tests {
    use super::*;
    #[test]
    fn offboard_air_admits_complete_entry_and_exit_routes_only() {
        for current in [
            CurrentPhysicalState::Ground,
            CurrentPhysicalState::GroundAnimation,
            CurrentPhysicalState::Air,
        ] {
            assert_eq!(
                selected_transition(current, PHYSICS_STATE_OFFBOARD_AIR),
                Some(GroundDestination::OffBoardAir)
            );
        }
        for (id, expected) in [
            (PHYSICS_STATE_PHYSICS_AIR, GroundDestination::Air),
            (PHYSICS_STATE_WIPEOUT_GROUND, GroundDestination::Wipeout),
            (PHYSICS_STATE_OFFBOARD, GroundDestination::OffBoard),
        ] {
            assert_eq!(
                selected_transition(CurrentPhysicalState::OffBoardAir, id),
                Some(expected)
            );
        }
        for pending in [PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_OFFBOARD_JUMP] {
            assert_eq!(
                selected_transition(CurrentPhysicalState::OffBoardAir, pending),
                None
            );
        }
    }

    #[test]
    fn audited_ground_animation_routes_do_not_admit_other_unrecovered_pairs() {
        assert_eq!(
            selected_transition(CurrentPhysicalState::Ground, PHYSICS_STATE_OFFBOARD),
            Some(GroundDestination::OffBoard)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::GroundAnimation, PHYSICS_STATE_OFFBOARD),
            Some(GroundDestination::OffBoard)
        );
        assert_eq!(selected_transition(CurrentPhysicalState::Air, PHYSICS_STATE_OFFBOARD), None);
        assert_eq!(
            selected_transition(CurrentPhysicalState::GroundAnimation, PHYSICS_STATE_PHYSICS_GROUND),
            Some(GroundDestination::Ground)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::Air, PHYSICS_STATE_PHYSICS_GROUND),
            Some(GroundDestination::Ground)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::GroundAnimation, PHYSICS_STATE_PHYSICS_AIR),
            Some(GroundDestination::Air)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::Air, PHYSICS_STATE_GROUND_ANIMATION),
            Some(GroundDestination::GroundAnimation)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::Ground, PHYSICS_STATE_GROUND_ANIMATION),
            Some(GroundDestination::GroundAnimation)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::Air, PHYSICS_STATE_WIPEOUT_GROUND),
            Some(GroundDestination::Wipeout)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::GroundAnimation, PHYSICS_STATE_WIPEOUT_GROUND),
            Some(GroundDestination::Wipeout)
        );
        for current in [
            CurrentPhysicalState::Ground,
            CurrentPhysicalState::GroundAnimation,
            CurrentPhysicalState::Air,
        ] {
            assert_eq!(
                selected_transition(current, PHYSICS_STATE_KNOWN_AIR),
                Some(GroundDestination::KnownAir)
            );
        }
        for (next, destination) in [
            (PHYSICS_STATE_PHYSICS_GROUND, GroundDestination::Ground),
            (PHYSICS_STATE_PHYSICS_AIR, GroundDestination::Air),
            (PHYSICS_STATE_WIPEOUT_GROUND, GroundDestination::Wipeout),
        ] {
            assert_eq!(
                selected_transition(CurrentPhysicalState::KnownAir, next),
                Some(destination)
            );
        }
        assert_eq!(
            selected_transition(CurrentPhysicalState::Ground, PHYSICS_STATE_PHYSICS_AIR),
            Some(GroundDestination::Air)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::Ground, PHYSICS_STATE_WIPEOUT_GROUND),
            Some(GroundDestination::Wipeout)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::OffBoard, PHYSICS_STATE_WIPEOUT_GROUND),
            Some(GroundDestination::Wipeout)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::OffBoard, PHYSICS_STATE_PHYSICS_AIR),
            Some(GroundDestination::Air)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::WipeoutGround, PHYSICS_STATE_TELEPORTING),
            Some(GroundDestination::Teleporting)
        );
        for current in [
            CurrentPhysicalState::Ground,
            CurrentPhysicalState::GroundAnimation,
            CurrentPhysicalState::Air,
            CurrentPhysicalState::OffBoard,
            CurrentPhysicalState::OffBoardAir,
            CurrentPhysicalState::OffBoardLocomotion,
            CurrentPhysicalState::LandingOnDeck,
        ] {
            assert_eq!(
                selected_transition(current, PHYSICS_STATE_TELEPORTING),
                Some(GroundDestination::Teleporting)
            );
        }
        assert_eq!(
            selected_transition(CurrentPhysicalState::Teleporting, PHYSICS_STATE_PHYSICS_GROUND),
            Some(GroundDestination::Ground)
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::WipeoutGround, PHYSICS_STATE_PHYSICS_GROUND),
            None
        );
    }

    #[test]
    fn offboard_jump_enters_from_offboard_and_leaves_through_the_folded_air_case() {
        assert_eq!(
            selected_transition(CurrentPhysicalState::OffBoard, PHYSICS_STATE_OFFBOARD_JUMP),
            Some(GroundDestination::OffBoardJump)
        );
        assert_eq!(GroundDestination::OffBoardJump.id(), PHYSICS_STATE_OFFBOARD_JUMP);
        for current in [
            CurrentPhysicalState::OffBoardAir,
            CurrentPhysicalState::OffBoardLocomotion,
            CurrentPhysicalState::OffBoardJump,
        ] {
            assert_eq!(selected_transition(current, PHYSICS_STATE_OFFBOARD_JUMP), None);
        }
        for (next, destination) in [
            (PHYSICS_STATE_WIPEOUT_GROUND, GroundDestination::Wipeout),
            (PHYSICS_STATE_OFFBOARD_LOCOMOTION, GroundDestination::OffBoardLocomotion),
            (PHYSICS_STATE_OFFBOARD, GroundDestination::OffBoard),
            (PHYSICS_STATE_LANDINGONDECK, GroundDestination::LandingOnDeck),
            (PHYSICS_STATE_PHYSICS_AIR, GroundDestination::Air),
            (PHYSICS_STATE_TELEPORTING, GroundDestination::Teleporting),
        ] {
            assert_eq!(
                selected_transition(CurrentPhysicalState::OffBoardJump, next),
                Some(destination)
            );
        }
        // SetPhysicsState82dff188 asserts on a same-type request; 502 stays.
        assert_eq!(
            selected_transition(CurrentPhysicalState::OffBoardJump, PHYSICS_STATE_OFFBOARD_AIR),
            None
        );
    }

    #[test]
    fn skitch_uses_native_ground_entry_and_release_destinations() {
        assert_eq!(
            selected_transition(CurrentPhysicalState::Ground, PHYSICS_STATE_SKITCHING),
            Some(GroundDestination::Skitching)
        );
        for (id, destination) in [
            (PHYSICS_STATE_PHYSICS_GROUND, GroundDestination::Ground),
            (PHYSICS_STATE_WIPEOUT_GROUND, GroundDestination::Wipeout),
            (PHYSICS_STATE_TELEPORTING, GroundDestination::Teleporting),
        ] {
            assert_eq!(
                selected_transition(CurrentPhysicalState::Skitching, id),
                Some(destination)
            );
        }
        // Skitch's off-ground condition first returns100, not an Air transition.
        assert_eq!(
            selected_transition(CurrentPhysicalState::Skitching, PHYSICS_STATE_PHYSICS_AIR),
            None
        );
        assert_eq!(
            selected_transition(CurrentPhysicalState::SlideGround, PHYSICS_STATE_SKITCHING),
            None
        );
    }
}
