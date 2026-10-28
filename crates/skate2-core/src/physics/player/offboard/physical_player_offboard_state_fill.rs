//! Same-pair source adapter for OffBoard.FillPhysOut 0x82d87358.
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::{
        PhysicalPlayerAssemblyOwner,
        remaining_states::{OffBoard, OwnerMember, StateKind, StatePayload},
    },
    physical_player_phys_out::{
        PhysOutAccessError, PhysOutPublicationError, offboard_fill::OffBoardFillSource,
    },
    physical_player_physics_state::CurrentPhysicalState,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum OffBoardFillReadError {
    Poisoned,
    Missing(&'static str),
    ForeignOwner,
    UnsupportedState(CurrentPhysicalState),
}
#[derive(Debug)]
pub(crate) enum OffBoardFillError {
    Read(OffBoardFillReadError),
    Output(PhysOutAccessError),
}
impl std::fmt::Display for OffBoardFillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffBoard.FillPhysOut: {self:?}")
    }
}

struct Source<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
}
impl Source<'_> {
    fn offboard<T>(
        &self,
        read: impl FnOnce(&OffBoard) -> Option<T>,
        field: &'static str,
    ) -> Result<T, OffBoardFillReadError> {
        let states = self
            .owner
            .remaining_states()
            .lock()
            .map_err(|_| OffBoardFillReadError::Poisoned)?;
        let Some(StatePayload::OffBoard(state)) = states
            .states
            .get(&StateKind::OffBoard)
            .filter(|s| s.published)
            .and_then(|s| s.value.as_ref())
        else {
            return Err(OffBoardFillReadError::Missing("published OffBoard"));
        };
        read(state).ok_or(OffBoardFillReadError::Missing(field))
    }
}
impl OffBoardFillSource for Source<'_> {
    type Error = OffBoardFillReadError;
    fn grabbing(&mut self) -> Result<bool, Self::Error> {
        self.offboard(|state| state.grab_object, "OffBoard grab object")
    }
    fn locomotion_viable(&mut self) -> Result<bool, Self::Error> {
        self.offboard(
            |state| state.is_locomotion_viable,
            "OffBoard locomotion viable",
        )
    }
    fn grab_descriptor(&mut self) -> Result<(i32, i32), Self::Error> {
        self.offboard(
            |state| {
                state
                    .grab_spline_desc
                    .map(|desc| (desc.object_type, desc.object_id))
            },
            "OffBoard grab spline descriptor",
        )
    }
    fn left_foot_down(&mut self) -> Result<bool, Self::Error> {
        let linked = self.offboard(
            |state| {
                state
                    .off_board_feet_ik
                    .as_ref()
                    .and_then(Option::as_ref)
                    .map(|link| {
                        link.member == OwnerMember::OffBoardFeetIK
                            && link
                                .owner
                                .upgrade()
                                .is_some_and(|owner| Arc::ptr_eq(&owner, self.owner))
                    })
            },
            "OffBoard FeetIK link",
        )?;
        if !linked {
            return Err(Self::Error::ForeignOwner);
        }
        Ok(self
            .owner
            .remaining()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .feet_ik
            .as_ref()
            .ok_or(Self::Error::Missing("OffBoardFeetIK"))?
            .left_foot_down)
    }
    fn right_foot_down(&mut self) -> Result<bool, Self::Error> {
        Ok(self
            .owner
            .remaining()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .feet_ik
            .as_ref()
            .ok_or(Self::Error::Missing("OffBoardFeetIK"))?
            .right_foot_down)
    }
}

pub(crate) fn try_fill_current_offboard(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Result<(), OffBoardFillError> {
    use OffBoardFillError as E;
    let selected = owner
        .physics_state()
        .lock()
        .map_err(|_| E::Read(OffBoardFillReadError::Poisoned))?
        .current;
    if selected != CurrentPhysicalState::OffBoard {
        return Err(E::Read(OffBoardFillReadError::UnsupportedState(selected)));
    }
    {
        let states = owner
            .remaining_states()
            .lock()
            .map_err(|_| E::Read(OffBoardFillReadError::Poisoned))?;
        let Some(StatePayload::OffBoard(state)) = states
            .states
            .get(&StateKind::OffBoard)
            .filter(|s| s.published)
            .and_then(|s| s.value.as_ref())
        else {
            return Err(E::Read(OffBoardFillReadError::Missing(
                "published OffBoard",
            )));
        };
        let bindings = &state.base.physical;
        if !bindings.board.same_as(board)
            || !bindings.skeleton.same_as(skeleton)
            || !bindings
                .owner
                .upgrade()
                .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
        {
            return Err(E::Read(OffBoardFillReadError::ForeignOwner));
        }
    }
    owner
        .phys_out()
        .lock()
        .map_err(|_| E::Read(OffBoardFillReadError::Poisoned))?
        .try_publish_offboard_fill(&mut Source { owner })
        .map_err(|error| match error {
            PhysOutPublicationError::Access(error) => E::Output(error),
            PhysOutPublicationError::Source(error) => E::Read(error),
        })
}
