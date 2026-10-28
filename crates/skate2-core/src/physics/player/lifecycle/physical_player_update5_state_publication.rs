//! Selected same-owner Update5 state publication spans, not Update5 completion.
use crate::physics_state_ids::{
    PHYSICS_STATE_BONELESS, PHYSICS_STATE_FOLLOW_PATH, PHYSICS_STATE_FOOTPLANT,
    PHYSICS_STATE_GROUND_ANIMATION, PHYSICS_STATE_HANDPLANT, PHYSICS_STATE_KNOWN_AIR,
    PHYSICS_STATE_LANDINGONDECK, PHYSICS_STATE_NONSPECIFIC, PHYSICS_STATE_OFFBOARD,
    PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_OFFBOARD_LOCOMOTION, PHYSICS_STATE_OFFBOARD_PUSHING,
    PHYSICS_STATE_REVERT_GROUND, PHYSICS_STATE_SKITCHING, PHYSICS_STATE_SLIDE_GROUND,
    PHYSICS_STATE_TELEPORTING,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::PhysOutAccessError,
    physical_player_phys_out::construct::update5_state::{
        Update5StateOutputError, Update5StateSource,
    },
    physical_player_physics_state::CurrentPhysicalState,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum Update5StateRuntimeError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    UnsupportedState(CurrentPhysicalState),
    Output(PhysOutAccessError),
}
impl std::fmt::Display for Update5StateRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "Update5 missing {field}"),
            Self::Poisoned => f.write_str("Update5 owner lock poisoned"),
            Self::ForeignOwner => f.write_str("Update5 state belongs to another pair"),
            Self::UnsupportedState(state) => write!(f, "Update5 state type pending: {state:?}"),
            Self::Output(error) => write!(f, "Teleporting FillPhysOut: {error:?}"),
        }
    }
}
struct Source<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
}
impl Update5StateSource for Source<'_> {
    type Error = Update5StateRuntimeError;
    fn current_type(&mut self) -> Result<i32, Self::Error> {
        let state = self
            .owner
            .physics_state()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?;
        // Grind GetType82dfe548..568 (400..404), Nonspecific82dfe5a0 (701).
        let remaining = match state.current {
            CurrentPhysicalState::Skitching => Some((
                crate::physical_player_assembly::remaining_states::StateKind::Skitching,
                PHYSICS_STATE_SKITCHING,
            )),
            // FollowPath.GetType82dfe5b8.
            CurrentPhysicalState::FollowPath => Some((
                crate::physical_player_assembly::remaining_states::StateKind::FollowPath,
                PHYSICS_STATE_FOLLOW_PATH,
            )),
            CurrentPhysicalState::Grind(grind) => Some((
                crate::physical_player_assembly::remaining_states::StateKind::grind(grind),
                grind.id(),
            )),
            CurrentPhysicalState::Nonspecific => Some((
                crate::physical_player_assembly::remaining_states::StateKind::Nonspecific,
                PHYSICS_STATE_NONSPECIFIC,
            )),
            _ => None,
        };
        if let Some((kind, id)) = remaining {
            let states = self
                .owner
                .remaining_states()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?;
            let bindings = states
                .published_bindings(kind)
                .ok_or(Self::Error::Missing("selected retained state"))?;
            if !bindings.board.same_as(self.board)
                || !bindings.skeleton.same_as(self.skeleton)
                || !bindings
                    .owner
                    .upgrade()
                    .is_some_and(|v| Arc::ptr_eq(&v, self.owner))
            {
                return Err(Self::Error::ForeignOwner);
            }
            return Ok(id);
        }
        if matches!(
            state.current,
            CurrentPhysicalState::GroundAnimation
                | CurrentPhysicalState::RevertGround
                | CurrentPhysicalState::SlideGround
                | CurrentPhysicalState::Teleporting
                | CurrentPhysicalState::OffBoard
                | CurrentPhysicalState::OffBoardPushing
                | CurrentPhysicalState::OffBoardAir
                | CurrentPhysicalState::OffBoardJump
                | CurrentPhysicalState::LandingOnDeck
                | CurrentPhysicalState::OffBoardLocomotion
                | CurrentPhysicalState::KnownAir
                | CurrentPhysicalState::Boneless
                | CurrentPhysicalState::FootPlant
                | CurrentPhysicalState::HandPlant
        ) {
            use crate::physical_player_assembly::remaining_states::{StateKind, StatePayload};
            let remaining = self
                .owner
                .remaining_states()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?;
            let (kind, id) = if state.current == CurrentPhysicalState::Teleporting {
                (StateKind::Teleporting, PHYSICS_STATE_TELEPORTING)
            } else if state.current == CurrentPhysicalState::RevertGround {
                (StateKind::RevertGround, PHYSICS_STATE_REVERT_GROUND)
            } else if state.current == CurrentPhysicalState::SlideGround {
                (StateKind::SlideGround, PHYSICS_STATE_SLIDE_GROUND)
            } else if state.current == CurrentPhysicalState::KnownAir {
                (StateKind::KnownAir, PHYSICS_STATE_KNOWN_AIR)
            } else if state.current == CurrentPhysicalState::OffBoardPushing {
                (StateKind::OffBoardPushing, PHYSICS_STATE_OFFBOARD_PUSHING)
            } else if state.current == CurrentPhysicalState::OffBoardLocomotion {
                (StateKind::OffBoardLocomotion, PHYSICS_STATE_OFFBOARD_LOCOMOTION)
            } else if state.current == CurrentPhysicalState::LandingOnDeck {
                (StateKind::LandingOnDeck, PHYSICS_STATE_LANDINGONDECK)
            } else if state.current == CurrentPhysicalState::OffBoardAir {
                (StateKind::OffBoardAir, PHYSICS_STATE_OFFBOARD_AIR)
            } else if state.current == CurrentPhysicalState::OffBoardJump {
                // OffBoard_Jump.GetType82d98590: the folded li r3,0x1f6.
                (StateKind::OffBoardJump, PHYSICS_STATE_OFFBOARD_AIR)
            } else if state.current == CurrentPhysicalState::OffBoard {
                (StateKind::OffBoard, PHYSICS_STATE_OFFBOARD)
            } else if state.current == CurrentPhysicalState::Boneless {
                // Boneless.GetType82d978c8: li r3,0x25a.
                (StateKind::Boneless, PHYSICS_STATE_BONELESS)
            } else if state.current == CurrentPhysicalState::HandPlant {
                (StateKind::HandPlant, PHYSICS_STATE_HANDPLANT)
            } else if state.current == CurrentPhysicalState::FootPlant {
                // FootPlant.GetType82d97698: li r3,0x259.
                (StateKind::FootPlant, PHYSICS_STATE_FOOTPLANT)
            } else {
                (StateKind::GroundAnimation, PHYSICS_STATE_GROUND_ANIMATION)
            };
            let Some(actual) = remaining
                .states
                .get(&kind)
                .filter(|s| s.published)
                .and_then(|s| s.value.as_ref())
            else {
                return Err(Self::Error::Missing("selected retained state"));
            };
            let bindings = match (kind, actual) {
                (StateKind::KnownAir, StatePayload::KnownAir(s)) => &s.base.physical,
                (StateKind::SlideGround, StatePayload::SlideGround(s)) => &s.base.physical,
                (StateKind::RevertGround, StatePayload::RevertGround(s)) => &s.base.physical,
                (StateKind::Teleporting, StatePayload::Teleporting(s)) => &s.base.physical,
                (StateKind::OffBoard, StatePayload::OffBoard(s)) => &s.base.physical,
                (StateKind::OffBoardPushing, StatePayload::OffBoardPushing(s)) => &s.base.physical,
                (StateKind::OffBoardAir, StatePayload::OffBoardAir(s)) => &s.base.physical,
                (StateKind::OffBoardJump, StatePayload::OffBoardJump(s)) => &s.base.physical,
                (StateKind::LandingOnDeck, StatePayload::LandingOnDeck(s)) => &s.base.physical,
                (StateKind::OffBoardLocomotion, StatePayload::OffBoardLocomotion(s)) => {
                    &s.base.physical
                }
                (StateKind::GroundAnimation, StatePayload::GroundAnimation(s)) => &s.base.physical,
                (StateKind::Boneless, StatePayload::Boneless(s)) => &s.base.physical,
                (StateKind::FootPlant, StatePayload::FootPlant(s)) => &s.base.physical,
                (StateKind::HandPlant, StatePayload::HandPlant(s)) => &s.base.physical,
                _ => return Err(Self::Error::Missing("selected retained state")),
            };
            if !bindings.board.same_as(self.board)
                || !bindings.skeleton.same_as(self.skeleton)
                || !bindings
                    .owner
                    .upgrade()
                    .is_some_and(|v| Arc::ptr_eq(&v, self.owner))
            {
                return Err(Self::Error::ForeignOwner);
            }
            // Teleporting.GetType82d91218 returns 702.
            return Ok(id);
        }
        let (bindings, kind) = match state.current {
            CurrentPhysicalState::Ground => (
                &state
                    .ground
                    .as_ref()
                    .ok_or(Self::Error::Missing("actual Ground instance"))?
                    .bindings,
                100,
            ),
            CurrentPhysicalState::Air => (
                &state
                    .air
                    .as_ref()
                    .ok_or(Self::Error::Missing("actual Air instance"))?
                    .bindings,
                200,
            ),
            // Wipeout vtable8234a6ec+14 ->82db40d8: li r3,300; blr.
            CurrentPhysicalState::WipeoutGround => (
                &state
                    .wipeout_ground
                    .as_ref()
                    .ok_or(Self::Error::Missing("actual WipeoutGround instance"))?
                    .bindings,
                300,
            ),
            other => return Err(Self::Error::UnsupportedState(other)),
        };
        if !bindings.board.same_as(self.board)
            || !bindings.skeleton.same_as(self.skeleton)
            || !bindings
                .owner
                .upgrade()
                .is_some_and(|owner| Arc::ptr_eq(&owner, self.owner))
        {
            return Err(Self::Error::ForeignOwner);
        }
        // Air ctor82d8aa5c installs vtable8234a2a8; +14 points82db40e0 (li r3,200).
        // This is the actual selected instance's GetType, freshly read per call.
        Ok(kind)
    }

    fn update_count(&mut self) -> Result<i32, Self::Error> {
        Ok(self
            .owner
            .prefix()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .as_ref()
            .ok_or(Self::Error::Missing("HiLOD prefix"))?
            .update_count)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .processed(),
        );
        let input = processed.lock().map_err(|_| Self::Error::Poisoned)?;
        input
            .physics_state_fields()
            .time_delta
            .ok_or(Self::Error::Missing("processed DT93c"))
    }
}

/// Teleporting.FillPhysOut82d869f0 exits when ready is clear. A ready state
/// publishes through the same retained PhysOut after the owner check.
pub(crate) fn try_fill_current_teleporting(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Result<(), Update5StateRuntimeError> {
    use crate::physical_player_assembly::remaining_states::{StateKind, StatePayload};
    use Update5StateRuntimeError as E;
    let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
    if state.current != CurrentPhysicalState::Teleporting {
        return Err(E::UnsupportedState(state.current));
    }
    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    let Some(StatePayload::Teleporting(actual)) = states
        .states
        .get(&StateKind::Teleporting)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_ref())
    else {
        return Err(E::Missing("Teleporting"));
    };
    let bindings = &actual.base.physical;
    if !bindings.board.same_as(board)
        || !bindings.skeleton.same_as(skeleton)
        || !bindings
            .owner
            .upgrade()
            .is_some_and(|p| Arc::ptr_eq(&p, owner))
    {
        return Err(E::ForeignOwner);
    }
    let ready = actual
        .teleport_ready
        .ok_or(E::Missing("Teleporting ready flag"))?;
    if ready == 0 {
        return Ok(());
    }
    let on_board = actual
        .teleport_to_on_board
        .ok_or(E::Missing("Teleporting on-board flag"))?
        != 0;
    let transform = actual
        .reset_transform
        .ok_or(E::Missing("Teleporting reset transform"))?;
    drop(states);
    drop(state);
    owner
        .phys_out()
        .lock()
        .map_err(|_| E::Poisoned)?
        .try_publish_teleporting_ready(on_board, transform)
        .map_err(E::Output)
}
pub(crate) fn try_publish_current_type(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Result<(), Update5StateOutputError<Update5StateRuntimeError>> {
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| Update5StateOutputError::Source(Update5StateRuntimeError::Poisoned))?;
    output.try_publish_update5_current_type(&mut Source {
        owner,
        board,
        skeleton,
    })
}
pub(crate) fn try_publish_category_and_dt(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Result<(), Update5StateOutputError<Update5StateRuntimeError>> {
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| Update5StateOutputError::Source(Update5StateRuntimeError::Poisoned))?;
    output.try_publish_update5_category_and_dt(&mut Source {
        owner,
        board,
        skeleton,
    })
}
