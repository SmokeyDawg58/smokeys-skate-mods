//! Island registration through actual retained `SkaterPool` members. No mutable
//! session/body capability escapes; original Part ordinals remain explicit.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationPhysicalBodyKind {
    Board,
    Skeleton,
}

impl SimulationSkaterAssemblyPool {
    /// The actual Insert request is authoritative; effective per-Part state
    /// (including static definitions and spy bits) is not Body list selection.
    pub(crate) fn try_physical_requested_state(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<u32, SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        match &self.members[self.member_index(handle)?].content {
            MemberContent::Board { candidate, .. } => candidate
                .requested_state()
                .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction),
            MemberContent::Skeleton { .. } => Ok(4),
        }
    }

    /// Source Body+28 assignment persists with the actual constructor after
    /// Island removal. This is NOT the independent Assembly world index.
    pub(crate) fn try_set_physical_body_id(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        body_id: u32,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        let index = self.member_index(handle)?;
        match &mut self.members[index].content {
            MemberContent::Board { candidate, .. } => candidate
                .construction
                .as_mut()
                .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?
                .set_island_body_id(body_id),
            MemberContent::Skeleton { candidate, .. } => candidate
                .construction_mut()
                .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?
                .set_island_body_id(body_id),
        }
        Ok(())
    }

    pub(crate) fn try_physical_member_owner(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        (
            SimulationPhysicalBodyKind,
            &Arc<PhysicalPlayerAssemblyOwner>,
        ),
        SimulationSkaterPoolError,
    > {
        self.check_backend(backend)?;
        match &self.members[self.member_index(handle)?].content {
            MemberContent::Board { candidate, .. } => Ok((
                SimulationPhysicalBodyKind::Board,
                candidate
                    .construction()
                    .and_then(|owner| owner.physical_owner())
                    .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?,
            )),
            MemberContent::Skeleton { candidate, .. } => Ok((
                SimulationPhysicalBodyKind::Skeleton,
                candidate
                    .construction()
                    .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?
                    .owner(),
            )),
        }
    }

    pub(crate) fn try_physical_part_count(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<usize, SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        Ok(match &self.members[self.member_index(handle)?].content {
            MemberContent::Board { candidate, .. } => candidate.session().part_transforms().count(),
            MemberContent::Skeleton { candidate, .. } => {
                candidate.owner().session().part_transforms().count()
            }
        })
    }

    pub(crate) fn try_physical_part_observation(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        part: usize,
    ) -> Result<SimulationRigidBodyObservation, SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        match &self.members[self.member_index(handle)?].content {
            MemberContent::Board { candidate, .. } => candidate
                .session()
                .try_part_contact_spy_observation(backend, part),
            MemberContent::Skeleton { candidate, .. } => candidate
                .owner()
                .session()
                .try_part_contact_spy_observation(backend, part),
        }
        .map_err(SimulationSkaterPoolError::PhysicalPart)
    }

    pub(crate) fn try_register_physical_part(
        &self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        part: usize,
    ) -> Result<SimulationRigidBodyObservation, SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        match &self.members[self.member_index(handle)?].content {
            MemberContent::Board { candidate, .. } => candidate
                .session()
                .try_register_part_contact_spy(backend, part),
            MemberContent::Skeleton { candidate, .. } => candidate
                .owner()
                .session()
                .try_register_part_contact_spy(backend, part),
        }
        .map_err(SimulationSkaterPoolError::PhysicalPart)
    }

    pub(crate) fn try_remove_physical_part_target(
        &self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        part: usize,
    ) -> Result<SimulationRigidBodyObservation, SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        match &self.members[self.member_index(handle)?].content {
            MemberContent::Board { candidate, .. } => candidate
                .session()
                .try_remove_part_contact_spy(backend, part),
            MemberContent::Skeleton { candidate, .. } => candidate
                .owner()
                .session()
                .try_remove_part_contact_spy(backend, part),
        }
        .map_err(SimulationSkaterPoolError::PhysicalPart)
    }

    /// ALL original detach guards precede enclosing spy/map removal. Kind is
    /// checked first so calling the wrong removal API has no target side effects.
    pub(crate) fn try_validate_physical_detach(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        kind: SimulationPhysicalBodyKind,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        match (&self.members[self.member_index(handle)?].content, kind) {
            (MemberContent::Board { candidate, .. }, SimulationPhysicalBodyKind::Board) => {
                candidate
                    .session()
                    .try_validate_detach(backend)
                    .map_err(SimulationSkaterPoolError::Detach)
            }
            (MemberContent::Skeleton { candidate, .. }, SimulationPhysicalBodyKind::Skeleton) => {
                candidate
                    .owner()
                    .try_validate_detach(backend)
                    .map_err(SimulationSkaterPoolError::Detach)
            }
            _ => Err(SimulationSkaterPoolError::WrongMemberKind),
        }
    }

    pub(crate) fn try_mark_physical_board_completed(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        let index = self.member_index(handle)?;
        let construction = self.members[index]
            .content
            .board_mut()?
            .construction
            .as_mut()
            .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?;
        if construction.physical_owner().is_none() {
            return Err(SimulationSkaterPoolError::MissingPhysicalConstruction);
        }
        construction.mark_physical_board_completed();
        Ok(())
    }
}
