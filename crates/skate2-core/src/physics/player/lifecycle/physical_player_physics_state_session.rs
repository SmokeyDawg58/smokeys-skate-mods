//! Source-ordered selected Sleeping -> `PhysicsGround` transition and entry.
//! This borrows the retained pair state; no caller-supplied success callbacks for
//! board/Skeleton/Pumping/Wipeout mutations and no direct gesture-to-jump path.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GROUND, PHYSICS_CATEGORY_OFFBOARD,
    PHYSICS_CATEGORY_OTHER, PHYSICS_STATE_LANDINGONDECK, PHYSICS_STATE_PHYSICS_GROUND,
    PHYSICS_STATE_SLEEPING,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_physics_state::{
        CurrentPhysicalState, PhysicalPlayerPhysicsStateOwner, PhysicalStateActorQuery,
        PhysicalStateBindings, PhysicalStateConstructionError, PhysicalStateStage,
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::{
        ground_entry_skeleton::GroundEntrySkeletonError,
        physical_board_forces::PhysicalPlayerBoardForceError,
        physical_skeleton_controller::{
            PhysicalSkeletonControllerError, SkeletonCollisionTransition,
        },
    },
};
use std::sync::Arc;

#[path = "../air/physical_player_air_state_session.rs"]
pub(crate) mod air;
#[path = "../ground/physical_player_ground_animation_entry.rs"]
pub(crate) mod ground_animation_entry;
#[path = "../ground/physical_player_ground_animation_exit.rs"]
pub(crate) mod ground_animation_exit;
#[path = "../ground/physical_player_ground_exit.rs"]
pub(crate) mod ground_exit;
#[path = "../ground/physical_player_ground_landing_force.rs"]
pub(crate) mod ground_landing_force;
#[path = "../ground/physical_player_ground_to_air.rs"]
pub(crate) mod ground_to_air;
#[path = "../board/physical_player_manual_velocity.rs"]
pub(crate) mod manual_velocity;
#[path = "../offboard/physical_player_offboard_entry_20260923.rs"]
pub(crate) mod offboard_entry;
#[path = "../wipeout/physical_player_wipeout_state_session.rs"]
pub(crate) mod wipeout;
#[path = "../wipeout/physical_player_wipeout_helpers.rs"]
pub(crate) mod wipeout_helpers;

#[derive(Debug)]
pub(crate) enum PhysicalStateSessionError<A, N> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Poisoned,
    Construction(PhysicalStateConstructionError),
    RemainingConstruction(crate::physical_player_assembly::remaining_states::StateConstructorError),
    WrongPhase,
    ForeignBindings,
    Controller(crate::skateboard_controller::SkateboardControllerError<std::convert::Infallible>),
    Actor(A),
    UnknownJPhyState,
    UnknownProcessedField(&'static str),
    Board(PhysicalPlayerBoardForceError),
    Skeleton(GroundEntrySkeletonError),
    Collision(PhysicalSkeletonControllerError),
    Reckoning(crate::physical_player_reckoning_owner::PhysicalPlayerReckoningStalePhase),
    Entry(crate::dynamic_assembly_session::skater_pool::board_entry::SkateboardEntrySessionError),
    NumericEntry(
        crate::skateboard_force_runtime::entry::ResetSpeedModelError<
            crate::dynamic_assembly_session::skater_pool::board_entry::SkateboardEntrySessionError,
            N,
        >,
    ),
    ManualVelocity(manual_velocity::ManualVelocityError<N>),
    Cvar(crate::input_cvar_runtime::InputCvarError),
    PendingManualVelocityDebug,
    LandingWobble(crate::dynamic_assembly_session::skater_pool::SkeletonLandingWobbleError),
    LandingForce(ground_landing_force::LandingForceError<N>),
    Pumping(crate::physical_player_pumping::PumpingError),
    Wipeout(crate::physical_player_wipeout::WipeoutOwnerError),
}

impl<A: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for PhysicalStateSessionError<A, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("physical state skater pool unavailable"),
            Self::Pair(error) => write!(f, "physical state pair: {error:?}"),
            Self::Poisoned => f.write_str("physical state owner lock poisoned"),
            Self::Construction(error) => write!(f, "physical state construction: {error:?}"),
            Self::RemainingConstruction(error) => {
                write!(f, "remaining physical state construction: {error}")
            }
            Self::WrongPhase => f.write_str("physical state operation already attempted"),
            Self::ForeignBindings => f.write_str("physical state has different pair bindings"),
            Self::Controller(error) => write!(f, "physical state board controller: {error:?}"),
            Self::Actor(error) => write!(f, "physical state Actor query/telemetry: {error:?}"),
            Self::UnknownJPhyState => f.write_str("physical state JPhy origin unknown"),
            Self::UnknownProcessedField(field) => {
                write!(f, "physical state processed {field} unknown")
            }
            Self::Board(error) => write!(f, "physical state board force: {error}"),
            Self::Skeleton(error) => write!(f, "physical state Skeleton: {error}"),
            Self::Collision(error) => write!(f, "physical state collision: {error}"),
            Self::Reckoning(error) => write!(f, "physical state Reckoning: {error:?}"),
            Self::Entry(error) => write!(f, "physical state board entry: {error:?}"),
            Self::NumericEntry(error) => write!(f, "physical state numeric entry: {error:?}"),
            Self::ManualVelocity(error) => write!(f, "physical state Manual velocity: {error}"),
            Self::Cvar(error) => write!(f, "physical state cvar: {error}"),
            Self::PendingManualVelocityDebug => {
                f.write_str("physical state Manual debug overlay pending")
            }
            Self::LandingWobble(error) => write!(f, "physical state landing wobble: {error:?}"),
            Self::LandingForce(error) => write!(f, "physical state landing force: {error:?}"),
            Self::Pumping(error) => write!(f, "physical state Pumping: {error:?}"),
            Self::Wipeout(error) => write!(f, "physical state Wipeout: {error:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    fn ground_state_pair_owner<A, N>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<Arc<PhysicalPlayerAssemblyOwner>, PhysicalStateSessionError<A, N>> {
        use PhysicalStateSessionError as E;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)
    }
    /// All26 constructor sites in original order, using the same source bundle
    /// and actual pair. This does not execute state Update or the constructor tail.
    pub(crate) fn try_construct_pair_physics_states(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalStateSessionError<std::convert::Infallible, std::convert::Infallible>>
    {
        use crate::physical_player_assembly::remaining_states::{
            StateConstructorBindings, StateKind,
        };
        use PhysicalStateSessionError as E;
        let owner = self.ground_state_pair_owner(board, skeleton)?;
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.sleeping_attempted
            || state.air_attempted
            || state.wipeout_attempted
            || state.ground_attempted
        {
            return Err(E::Construction(
                PhysicalStateConstructionError::AlreadyAttempted,
            ));
        }
        let bindings = PhysicalStateBindings::new(&owner, board, skeleton);
        let systems = StateConstructorBindings::capture(bindings.clone())
            .map_err(E::RemainingConstruction)?;
        let mut remaining = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
        state
            .try_construct_sleeping(bindings.clone(), true)
            .map_err(E::Construction)?;
        for &kind in &StateKind::ORDER[..3] {
            remaining
                .try_construct(
                    kind,
                    systems.clone(),
                    &owner.inputs().vault,
                    &owner.inputs().cvars,
                    true,
                )
                .map_err(E::RemainingConstruction)?;
        }
        state
            .try_construct_air(bindings.clone(), &owner.inputs().vault, true)
            .map_err(E::Construction)?;
        for &kind in &StateKind::ORDER[3..9] {
            remaining
                .try_construct(
                    kind,
                    systems.clone(),
                    &owner.inputs().vault,
                    &owner.inputs().cvars,
                    true,
                )
                .map_err(E::RemainingConstruction)?;
        }
        state
            .try_construct_wipeout(bindings.clone(), &owner.inputs().vault, true)
            .map_err(E::Construction)?;
        for &kind in &StateKind::ORDER[9..20] {
            remaining
                .try_construct(
                    kind,
                    systems.clone(),
                    &owner.inputs().vault,
                    &owner.inputs().cvars,
                    true,
                )
                .map_err(E::RemainingConstruction)?;
        }
        state
            .try_construct_ground(bindings, owner.inputs().is_human, true)
            .map_err(E::Construction)?;
        for &kind in &StateKind::ORDER[20..] {
            remaining
                .try_construct(
                    kind,
                    systems.clone(),
                    &owner.inputs().vault,
                    &owner.inputs().cvars,
                    true,
                )
                .map_err(E::RemainingConstruction)?;
        }
        Ok(())
    }

    /// This is the genuine initial transition path through actual Actor queries.
    /// Missing source initialization/telemetry retains the preceding state stores.
    /// The host one-attempt guard prevents re-running partially committed entry.
    pub(crate) fn try_start_pair_ground_state<A: PhysicalStateActorQuery, N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        actor: &mut A,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), PhysicalStateSessionError<A::Error, N::Error>> {
        use PhysicalStateSessionError as E;
        let owner = self.ground_state_pair_owner(board, skeleton)?;
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.transition_attempted {
            return Err(E::WrongPhase);
        }
        state
            .try_select_initial_sleeping()
            .map_err(E::Construction)?;
        state.transition_attempted = true;
        let bindings = state.sleeping.as_ref().ok_or(E::Construction(
            PhysicalStateConstructionError::AllocationNull,
        ))?;
        if !bindings
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, &owner))
            || !bindings.board.same_as(board)
            || !bindings.skeleton.same_as(skeleton)
        {
            return Err(E::ForeignBindings);
        }
        state.stage = PhysicalStateStage::Controller;
        owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_enter_physics_state(PHYSICS_STATE_PHYSICS_GROUND)
            .map_err(E::Controller)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        state.stage = PhysicalStateStage::TransitionFields;
        state.state_count = Some(0);
        state.state_timer = Some(0.0);
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields_mut()
            .state_timer = Some(0.0);
        // Source independently calls old GetType (Sleeping700), then category
        // 82d83bbc..bc4 maps700->700. Separate processed captures follow each.
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields_mut()
            .last = Some(PHYSICS_STATE_SLEEPING);
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields_mut()
            .last_category = Some(PHYSICS_CATEGORY_OTHER);
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields_mut()
            .next = Some(PHYSICS_STATE_PHYSICS_GROUND);
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields_mut()
            .current = Some(PHYSICS_STATE_PHYSICS_GROUND);
        // Old Sleeping.Exit is the verified folded BLR825187c0.
        state.stage = PhysicalStateStage::ActorQuery;
        transition_actor_queries(&mut state, &processed, actor)?;
        state.current = CurrentPhysicalState::Ground;
        state.stage = PhysicalStateStage::GroundSelected;
        let ground = state.ground.as_ref().ok_or(E::Construction(
            PhysicalStateConstructionError::AllocationNull,
        ))?;
        if !ground
            .bindings
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, &owner))
            || !ground.bindings.board.same_as(board)
            || !ground.bindings.skeleton.same_as(skeleton)
        {
            return Err(E::ForeignBindings);
        }
        self.enter_selected_ground(&owner, &mut state, board, skeleton, cvars, numeric)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered Ground entry retains collaborator failure prefixes"
    )]
    fn enter_selected_ground<A, N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        state: &mut PhysicalPlayerPhysicsStateOwner,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), PhysicalStateSessionError<A, N::Error>> {
        use PhysicalStateSessionError as E;
        state.stage = PhysicalStateStage::SetStandard;
        self.try_set_board_standard(board).map_err(E::Board)?;
        state.stage = PhysicalStateStage::EnableIkAndTilt;
        self.try_ground_entry_skeleton_ik_and_tilt(board, skeleton)
            .map_err(E::Skeleton)?;
        state.stage = PhysicalStateStage::SkeletonCollision;
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(6),
        )
        .map_err(E::Collision)?;
        state.stage = PhysicalStateStage::LocalPrefix;
        state
            .ground
            .as_mut()
            .expect("selected ground owner")
            .fields
            .enter_local_prefix();
        state.stage = PhysicalStateStage::ResetSpin;
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_reset_body_spin()
            .map_err(E::Reckoning)?;
        state.stage = PhysicalStateStage::DisableAnimation;
        self.try_disable_board_animation(board).map_err(E::Entry)?;
        state.stage = PhysicalStateStage::ProjectOmega;
        self.try_project_board_ground_entry_omega(board, numeric)
            .map_err(E::NumericEntry)?;
        state.stage = PhysicalStateStage::ResetSpeed;
        self.try_reset_board_speed_model(board, numeric)
            .map_err(E::NumericEntry)?;
        state.stage = PhysicalStateStage::ResetWobble;
        self.try_reset_board_speed_wobble(board).map_err(E::Entry)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        state.stage = PhysicalStateStage::Manual;
        let category = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .category
            .ok_or(E::UnknownProcessedField("category"))?;
        if category == PHYSICS_CATEGORY_GROUND {
            self.try_scale_board_manual_for_ground_entry(board)
                .map_err(E::Entry)?;
        } else {
            self.try_reset_board_manual(board).map_err(E::Entry)?;
            state.stage = PhysicalStateStage::ManualVelocity;
            self.remove_ground_manual_velocity(&processed, board, cvars, numeric)?;
        }
        state.stage = PhysicalStateStage::LandingWobble;
        let last_category = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .last_category
            .ok_or(E::UnknownProcessedField("last category"))?;
        if last_category == PHYSICS_CATEGORY_AIR {
            // Skeleton.StartDeckLandingWobble82b420e8 samples the two live
            // ProcessedPhysIn words before StartNewWobble copies its graphs.
            let mirrored = {
                let p = processed.lock().map_err(|_| E::Poisoned)?;
                let (flags_8c4, known_8c4) = p.known_flags_8c4();
                if known_8c4 & 0x0010_0000 == 0 {
                    return Err(E::UnknownProcessedField("flags 8c4 bit20"));
                }
                let flags_8cc = p
                    .physics_state_fields()
                    .flags_8cc
                    .ok_or(E::UnknownProcessedField("flags 8cc"))?;
                ((flags_8c4 >> 20) ^ (flags_8cc >> 3)) & 1 != 0
            };
            self.skaters
                .as_mut()
                .ok_or(E::MissingPool)?
                .try_start_deck_landing_wobble(&self.backend, skeleton, owner, mirrored)
                .map_err(E::LandingWobble)?;
        }
        state.stage = PhysicalStateStage::LandingForce;
        let last = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .last
            .ok_or(E::UnknownProcessedField("last state"))?;
        if last == PHYSICS_STATE_LANDINGONDECK {
            self.try_ground_landing_force(board, &processed, numeric)
                .map_err(E::LandingForce)?;
        }
        state.stage = PhysicalStateStage::ResetPumping;
        owner
            .pumping()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_reset()
            .map_err(E::Pumping)?;
        state.stage = PhysicalStateStage::Wipeout;
        owner
            .wipeout()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_set_state(1)
            .map_err(E::Wipeout)?;
        state
            .ground
            .as_mut()
            .expect("selected ground owner")
            .fields
            .finish_enter();
        state.stage = PhysicalStateStage::EntryComplete;
        Ok(())
    }

    /// `RemoveVelIntoGroundForManual` 82d8d090: deck first, then retained front/back
    /// contact flags choose actual three-Part groups. Debug targets are sampled
    /// only after each selected group; missing debug service retains its writes.
    fn remove_ground_manual_velocity<A, N: ReckoningNumeric>(
        &mut self,
        processed: &Arc<
            std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>,
        >,
        board: &SimulationSkaterAssemblyHandle,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), PhysicalStateSessionError<A, N::Error>> {
        use PhysicalStateSessionError as E;
        let balance = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .balance
            .ok_or(E::UnknownProcessedField("balance"))?;
        if balance == 0.0 {
            return Ok(());
        }
        let normal = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .ground_normal()
            .ok_or(E::UnknownProcessedField("ground normal"))?
            .value;
        self.try_remove_board_part_velocity_along_normal(board, 6, normal, numeric)
            .map_err(E::ManualVelocity)?;
        let (flipped, contacts) = {
            let input = processed.lock().map_err(|_| E::Poisoned)?;
            (
                input
                    .board_flipped()
                    .ok_or(E::UnknownProcessedField("board flip"))?,
                input
                    .known_flags_8c8()
                    .ok_or(E::UnknownProcessedField("wheel contact flags"))?,
            )
        };
        let front = contacts & (1 << 27) != 0;
        let back = contacts & (1 << 28) != 0;
        let (first, second) = if flipped {
            (front, back)
        } else {
            (back, front)
        };
        for (selected, parts) in [(first, [0, 1, 4]), (second, [2, 3, 5])] {
            if selected {
                for part in parts {
                    self.try_remove_board_part_velocity_along_normal(board, part, normal, numeric)
                        .map_err(E::ManualVelocity)?;
                }
                if cvars.try_show_vel_removal().map_err(E::Cvar)? {
                    return Err(E::PendingManualVelocityDebug);
                }
            }
        }
        Ok(())
    }
}

fn transition_actor_queries<A: PhysicalStateActorQuery, N>(
    state: &mut PhysicalPlayerPhysicsStateOwner,
    processed: &Arc<std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>>,
    actor: &mut A,
) -> Result<(), PhysicalStateSessionError<A::Error, N>> {
    use PhysicalStateSessionError as E;
    let first = actor.player_id().map_err(E::Actor)?;
    if first == 0 && state.jphy_state.ok_or(E::UnknownJPhyState)? != PHYSICS_CATEGORY_OFFBOARD {
        let category = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .category
            .ok_or(E::UnknownProcessedField("category"))?;
        if category == PHYSICS_CATEGORY_OFFBOARD {
            state.stage = PhysicalStateStage::Telemetry;
            actor.log_board_transition(true).map_err(E::Actor)?;
            state.jphy_state = Some(
                processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .physics_state_fields()
                    .category
                    .ok_or(E::UnknownProcessedField("category"))?,
            );
            return Ok(());
        }
    }
    let second = actor.player_id().map_err(E::Actor)?;
    if second == 0 && state.jphy_state.ok_or(E::UnknownJPhyState)? == PHYSICS_CATEGORY_OFFBOARD {
        let category = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .category
            .ok_or(E::UnknownProcessedField("category"))?;
        if category != PHYSICS_CATEGORY_OFFBOARD {
            state.stage = PhysicalStateStage::Telemetry;
            actor.log_board_transition(false).map_err(E::Actor)?;
            state.jphy_state = Some(
                processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .physics_state_fields()
                    .category
                    .ok_or(E::UnknownProcessedField("category"))?,
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton_processed_contact::SkeletonProcessedContact;
    use std::sync::Mutex;

    struct ActorQueries {
        ids: std::collections::VecDeque<i32>,
        calls: usize,
        logs: Vec<bool>,
        fail_log: bool,
        publish_category: Option<(Arc<Mutex<SkeletonProcessedContact>>, i32)>,
    }
    impl PhysicalStateActorQuery for ActorQueries {
        type Error = &'static str;
        fn player_id(&mut self) -> Result<i32, Self::Error> {
            self.calls += 1;
            self.ids.pop_front().ok_or("unexpected player query")
        }
        fn log_board_transition(&mut self, offboard: bool) -> Result<(), Self::Error> {
            self.logs.push(offboard);
            if let Some((processed, category)) = &self.publish_category {
                processed
                    .lock()
                    .unwrap()
                    .physics_state_fields_mut()
                    .category = Some(*category);
            }
            if self.fail_log {
                Err("telemetry unavailable")
            } else {
                Ok(())
            }
        }
    }
    fn actor(ids: &[i32]) -> ActorQueries {
        ActorQueries {
            ids: ids.iter().copied().collect(),
            calls: 0,
            logs: vec![],
            fail_log: false,
            publish_category: None,
        }
    }
    fn processed() -> Arc<Mutex<SkeletonProcessedContact>> {
        let mut value = SkeletonProcessedContact::new();
        value.reset();
        Arc::new(Mutex::new(value))
    }
    #[test]
    fn local_first_query_preserves_unwritten_jphy_and_preceding_state() {
        let mut state = PhysicalPlayerPhysicsStateOwner::new();
        state.current = CurrentPhysicalState::Sleeping;
        let processed = processed();
        processed.lock().unwrap().physics_state_fields_mut().current = Some(PHYSICS_STATE_PHYSICS_GROUND);
        let mut actor = actor(&[0]);
        assert!(matches!(
            transition_actor_queries::<_, ()>(&mut state, &processed, &mut actor),
            Err(PhysicalStateSessionError::UnknownJPhyState)
        ));
        assert_eq!(actor.calls, 1);
        assert!(actor.logs.is_empty());
        assert_eq!(state.current, CurrentPhysicalState::Sleeping);
        assert_eq!(state.jphy_state, None);
        assert_eq!(
            processed.lock().unwrap().physics_state_fields().current,
            Some(PHYSICS_STATE_PHYSICS_GROUND)
        );
    }
    #[test]
    fn nonlocal_queries_do_not_read_unknown_jphy_or_category() {
        let mut state = PhysicalPlayerPhysicsStateOwner::new();
        let processed = Arc::new(Mutex::new(SkeletonProcessedContact::new()));
        let mut actor = actor(&[1, -1]);
        transition_actor_queries::<_, ()>(&mut state, &processed, &mut actor).unwrap();
        assert_eq!(actor.calls, 2);
        assert_eq!(state.jphy_state, None);
    }
    #[test]
    fn actual_sim_cleared_local_start_queries_twice_without_telemetry() {
        let mut state = PhysicalPlayerPhysicsStateOwner::from_sim_zeroed_allocation();
        let processed = processed();
        let mut actor = actor(&[0, 0]);
        transition_actor_queries::<_, ()>(&mut state, &processed, &mut actor).unwrap();
        assert_eq!(actor.calls, 2);
        assert!(actor.logs.is_empty());
        assert_eq!(state.jphy_state, Some(0));
    }
    #[test]
    fn telemetry_failure_precedes_jphy_store_and_success_recaptures_category() {
        let mut state = PhysicalPlayerPhysicsStateOwner::new();
        state.jphy_state = Some(PHYSICS_CATEGORY_OFFBOARD); // Explicit old-state fixture, not ctor evidence.
        let processed = processed();
        let mut actor = actor(&[0, 0]);
        actor.fail_log = true;
        assert!(matches!(
            transition_actor_queries::<_, ()>(&mut state, &processed, &mut actor),
            Err(PhysicalStateSessionError::Actor("telemetry unavailable"))
        ));
        assert_eq!(actor.calls, 2);
        assert_eq!(actor.logs, [false]);
        assert_eq!(state.jphy_state, Some(PHYSICS_CATEGORY_OFFBOARD));
        assert_eq!(state.stage, PhysicalStateStage::Telemetry);
        actor.ids.extend([0, 0]);
        actor.fail_log = false;
        actor.publish_category = Some((Arc::clone(&processed), PHYSICS_CATEGORY_AIR));
        transition_actor_queries::<_, ()>(&mut state, &processed, &mut actor).unwrap();
        assert_eq!(state.jphy_state, Some(PHYSICS_CATEGORY_AIR));
    }
}
