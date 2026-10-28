//! Constructed LivingWorld definitions enter the SAME DynamicPool membership.
//! AssemblyPool.Insert82e0dbb0 / Remove82e0c1c8. Island.AddVehicleBody827b6840
//! still must assign its Body ID/list and register spies outside this layer.
use super::*;
use crate::dynamic_assembly_session::{
    DynamicAssemblyBodyOnlyPartAttachError, character_publication::CharacterPublicationError,
};
use std::convert::Infallible;
#[path = "simulation_pedestrian_assembly_pool.rs"]
pub(crate) mod pedestrian_pool;

pub(crate) struct CharacterAssemblyCandidate {
    handle: SimulationDmoAssemblyHandle,
    pub(crate) assembly: CharacterCollisionAssembly,
    world_index: u32,
    requested_state: Option<u32>,
    entity_id: u32,
    body_id: Option<u32>,
}
impl CharacterAssemblyCandidate {
    /// Shared base AssemblyPool.Insert82e0dbb0 operation. Both pool kinds
    /// retain the same actual Parts; mesh-query publication is not done here.
    fn attach_serialised(
        &mut self, manager: &mut SimulationManagerContactDomain,
        backend: &mut SourceBackend, state: u32,
    ) -> Result<CharacterCollisionPublication, CharacterPoolError> {
        self.world_index = manager.try_allocate_assembly_world_index(backend.authority())
            .map_err(|error| CharacterPoolError::Pool(SimulationDmoPoolError::Labels(error)))?;
        self.requested_state = Some(state);
        self.assembly.parts.attach(backend, state).map_err(CharacterPoolError::Attach)?;
        CharacterCollisionPublication::try_create(&self.assembly, backend,
            SerialisedAssemblyParamsState::native_default().edge_cos_bend_normal_threshold)
            .map_err(CharacterPoolError::Publication)
    }
    pub(crate) fn new(assembly: CharacterCollisionAssembly, entity_id: u32) -> Self {
        Self {
            handle: SimulationDmoAssemblyHandle(Arc::new(())),
            assembly,
            world_index: 0xffff,
            requested_state: None,
            entity_id,
            body_id: None,
        }
    }
    pub(crate) fn entity_id(&self) -> u32 {
        self.entity_id
    }
    pub(crate) fn handle(&self) -> &SimulationDmoAssemblyHandle {
        &self.handle
    }
    pub(crate) fn body_id(&self) -> Option<u32> {
        self.body_id
    }
    pub(crate) fn world_index(&self) -> u32 {
        self.world_index
    }
    pub(crate) fn requested_state(&self) -> Option<u32> {
        self.requested_state
    }
    pub(crate) fn try_detach(
        &mut self,
        backend: &mut SourceBackend,
    ) -> Result<(), DynamicAssemblyBodyOnlyPartDetachError> {
        self.assembly.parts.detach(backend)
    }
}

#[derive(Debug)]
pub(crate) enum CharacterPoolError {
    Pool(SimulationDmoPoolError<Infallible>),
    Attach(DynamicAssemblyBodyOnlyPartAttachError),
    Publication(CharacterPublicationError),
}
pub(crate) struct CharacterInsertFailure {
    pub(crate) error: CharacterPoolError,
    /// Same candidate and any allocated world ID survive checked failure.
    /// An attached candidate requires explicit detach before disposal/retry.
    pub(crate) candidate: CharacterAssemblyCandidate,
}
impl fmt::Debug for CharacterInsertFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CharacterInsertFailure")
            .field("error", &self.error)
            .field("world_index", &self.candidate.world_index)
            .field("attached", &self.candidate.assembly.parts.is_attached())
            .finish()
    }
}

impl SimulationDmoAssemblyPool {
    /// Shared Assembly group and Part userData writes82e4c9d8. Existing refresh
    /// consumes these SAME live values for both serialized and character forms.
    pub(crate) fn try_set_collision_group(&mut self, handle: &SimulationDmoAssemblyHandle, group: u32)
        -> Result<(), SimulationDmoPoolError<Infallible>> {
        let member = self.members.iter_mut().find(|m| m.handle.same_as(handle))
            .ok_or(SimulationDmoPoolError::MissingMember)?;
        match &mut member.content {
            MemberContent::Source { candidate, .. } => {
                candidate.group = group;
                candidate.part_user_data.fill(group);
            }
            MemberContent::Character { candidate, .. } => {
                candidate.assembly.group = group;
                candidate.assembly.part_user_data.fill(group);
            }
        }
        Ok(())
    }
    /// Borrow actual retained Parts and Body metadata across both existing
    /// DynamicPool member forms. Island registration creates no second body.
    pub(crate) fn island_body(
        &self,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Option<DynamicIslandBody<'_>> {
        let member = self
            .members
            .iter()
            .find(|member| member.handle.same_as(handle))?;
        Some(match &member.content {
            MemberContent::Source { candidate, .. } => DynamicIslandBody {
                parts: &candidate.parts.parts,
                entity_id: candidate.entity_id,
                body_id: candidate.body_id,
                requested_state: candidate.requested_state,
                group: candidate.group,
            },
            MemberContent::Character { candidate, .. } => DynamicIslandBody {
                parts: &candidate.assembly.parts,
                entity_id: Some(candidate.entity_id),
                body_id: candidate.body_id,
                requested_state: candidate.requested_state,
                group: candidate.assembly.group,
            },
        })
    }

    pub(crate) fn try_set_island_body_id(
        &mut self,
        backend: &SourceBackend,
        handle: &SimulationDmoAssemblyHandle,
        id: u32,
    ) -> Result<(), SimulationDmoPoolError<Infallible>> {
        if backend.authority() != self.backend {
            return Err(SimulationDmoPoolError::ForeignBackend);
        }
        let member = self
            .members
            .iter_mut()
            .find(|member| member.handle.same_as(handle))
            .ok_or(SimulationDmoPoolError::MissingMember)?;
        match &mut member.content {
            MemberContent::Source { candidate, .. } => candidate.body_id = Some(id),
            MemberContent::Character { candidate, .. } => candidate.body_id = Some(id),
        }
        Ok(())
    }
    pub(crate) fn try_insert_character(
        &mut self,
        manager: &mut SimulationManagerContactDomain,
        backend: &mut SourceBackend,
        mut candidate: CharacterAssemblyCandidate,
        state: u32,
    ) -> Result<SimulationDmoAssemblyHandle, Box<CharacterInsertFailure>> {
        let result = (|| {
            self.check_insert(
                manager,
                backend,
                candidate.assembly.backend_authority(),
                candidate.assembly.parts.is_attached(),
            )
            .map_err(CharacterPoolError::Pool)?;
            // Actual Assembly identity survives RemoveFromWorld/re-admission.
            let handle = candidate.handle.clone();
            let publication = candidate.attach_serialised(manager, backend, state)?;
            Ok((handle, publication))
        })();
        match result {
            Ok((handle, publication)) => {
                let bounds = publication.bounds();
                self.members.push(Member {
                    handle: handle.clone(),
                    content: MemberContent::Character {
                        candidate,
                        publication,
                    },
                    bounds,
                });
                Ok(handle)
            }
            Err(error) => Err(Box::new(CharacterInsertFailure { error, candidate })),
        }
    }

    pub(crate) fn character_candidate(
        &self,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Option<&CharacterAssemblyCandidate> {
        self.members
            .iter()
            .find(|member| member.handle.same_as(handle))
            .and_then(|member| match &member.content {
                MemberContent::Character { candidate, .. } => Some(candidate),
                _ => None,
            })
    }
    pub(crate) fn character_publication(
        &self,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Option<&CharacterCollisionPublication> {
        self.members
            .iter()
            .find(|member| member.handle.same_as(handle))
            .and_then(|member| match &member.content {
                MemberContent::Character { publication, .. } => Some(publication),
                _ => None,
            })
    }
    pub(crate) fn character_publications(
        &self,
    ) -> impl Iterator<Item = &CharacterCollisionPublication> {
        self.members
            .iter()
            .filter_map(|member| match &member.content {
                MemberContent::Character { publication, .. } => Some(publication),
                _ => None,
            })
    }

    /// Base Remove; caller performs original batch synchronization and then
    /// removes owned mesh slots, as with the existing serialized DMO route.
    pub(crate) fn try_remove_character(
        &mut self,
        backend: &mut SourceBackend,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<CharacterAssemblyCandidate, SimulationDmoPoolError<Infallible>> {
        if backend.authority() != self.backend {
            return Err(SimulationDmoPoolError::ForeignBackend);
        }
        let index = self
            .members
            .iter()
            .position(|member| member.handle.same_as(handle))
            .ok_or(SimulationDmoPoolError::MissingMember)?;
        let MemberContent::Character { candidate, .. } = &mut self.members[index].content else {
            return Err(SimulationDmoPoolError::WrongMemberKind);
        };
        candidate
            .try_detach(backend)
            .map_err(SimulationDmoPoolError::Detach)?;
        let MemberContent::Character { candidate, .. } = self.members.swap_remove(index).content
        else {
            unreachable!()
        };
        Ok(candidate)
    }
}

pub(crate) struct DynamicIslandBody<'a> {
    pub(crate) parts:
        &'a crate::dynamic_assembly_session::DynamicAssemblyBodyOnlyPartSession<SourceBackend>,
    pub(crate) entity_id: Option<u32>,
    pub(crate) body_id: Option<u32>,
    pub(crate) requested_state: Option<u32>,
    pub(crate) group: u32,
}

#[cfg(test)]
#[path = "simulation_dynamic_pool_character_tests.rs"]
mod tests;
