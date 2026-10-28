//! Base AssemblyPool for pedestrians, InitWorld827b42c0; no DynamicPool
//! world-line collection, mesh slots or vehicle capacity is added.
use super::*;

struct Member {
    candidate: CharacterAssemblyCandidate,
    publication: CharacterCollisionPublication,
    bounds: AabbWords,
}
pub(crate) struct PedestrianAssemblyPool {
    manager: SimulationManagerContactAuthority,
    backend: SimulationRigidBodyPoolAuthority,
    capacity: usize,
    members: Vec<Member>,
    active_ordinals: Vec<usize>,
    active_frame: Option<crate::simulation_rigid_body_pool::SimulationRigidBodyFrameIdentity>,
}
impl PedestrianAssemblyPool {
    /// LWCollisionBody.SetCollisionGroup82e4c9d8 changes the Assembly and every
    /// Part userData. SerialisedAssembly records are copied at normal refresh.
    pub(crate) fn try_set_collision_group(&mut self, handle: &SimulationDmoAssemblyHandle, group: u32)
        -> Result<(), SimulationDmoPoolError<Infallible>> {
        let owner = &mut self.members.iter_mut().find(|m| m.candidate.handle.same_as(handle))
            .ok_or(SimulationDmoPoolError::MissingMember)?.candidate.assembly;
        owner.group = group;
        owner.part_user_data.fill(group);
        Ok(())
    }
    pub(crate) fn len(&self) -> usize { self.members.len() }
    pub(crate) fn publications(&self) -> impl ExactSizeIterator<Item = &CharacterCollisionPublication> {
        self.members.iter().map(|member| &member.publication)
    }
    pub(crate) fn active_ordinals(&self) -> Option<&[usize]> {
        self.active_frame.map(|_| self.active_ordinals.as_slice())
    }
    pub(crate) fn active_frame(&self) -> Option<crate::simulation_rigid_body_pool::SimulationRigidBodyFrameIdentity> {
        self.active_frame
    }
    pub(crate) fn matches_authorities(&self, manager: &SimulationManagerContactAuthority,
        backend: &SimulationRigidBodyPoolAuthority) -> bool {
        self.manager == *manager && self.backend == *backend
    }
    pub(crate) fn island_body(&self, handle: &SimulationDmoAssemblyHandle) -> Option<DynamicIslandBody<'_>> {
        let candidate = self.candidate(handle)?;
        Some(DynamicIslandBody { parts: &candidate.assembly.parts,
            entity_id: Some(candidate.entity_id), body_id: candidate.body_id,
            requested_state: candidate.requested_state, group: candidate.assembly.group })
    }
    pub(crate) fn try_set_island_body_id(&mut self, backend: &SourceBackend,
        handle: &SimulationDmoAssemblyHandle, id: u32,
    ) -> Result<(), SimulationDmoPoolError<Infallible>> {
        if backend.authority() != self.backend { return Err(SimulationDmoPoolError::ForeignBackend); }
        self.try_set_body_id(handle, id)
    }
    /// Raw827b4378/4388 writes maxAssemblies, not numAssembliesPerJob.
    pub(crate) fn try_new(manager: &SimulationManagerContactDomain,
        backend: &SourceBackend, island_flags: u32, update_in_parallel: bool,
    ) -> Result<Self, SimulationDmoPoolError<Infallible>> {
        if manager.assembly_body_pool_authority() != backend.authority() {
            return Err(SimulationDmoPoolError::ForeignBackend);
        }
        if update_in_parallel { return Err(SimulationDmoPoolError::UnsupportedParallel); }
        let capacity = if island_flags & 1 != 0 { 148 } else { 16 };
        let mut members = Vec::new();
        members.try_reserve_exact(capacity).map_err(|_| SimulationDmoPoolError::Allocation)?;
        let mut active_ordinals = Vec::new();
        active_ordinals.try_reserve_exact(capacity).map_err(|_| SimulationDmoPoolError::Allocation)?;
        Ok(Self { manager: manager.assembly_pool_authority(), backend: backend.authority(),
            capacity, members, active_ordinals, active_frame: None })
    }
    pub(crate) fn try_insert(&mut self, manager: &mut SimulationManagerContactDomain,
        backend: &mut SourceBackend, mut candidate: CharacterAssemblyCandidate, state: u32,
    ) -> Result<SimulationDmoAssemblyHandle, Box<CharacterInsertFailure>> {
        let result = (|| {
            let fail = |error| CharacterPoolError::Pool(error);
            if backend.authority() != self.backend || candidate.assembly.backend_authority() != self.backend {
                return Err(fail(SimulationDmoPoolError::ForeignBackend));
            }
            if manager.assembly_pool_authority() != self.manager {
                return Err(fail(SimulationDmoPoolError::ForeignManager));
            }
            if candidate.assembly.parts.is_attached() {
                return Err(fail(SimulationDmoPoolError::AlreadyAttached));
            }
            if self.members.len() >= self.capacity { return Err(fail(SimulationDmoPoolError::Full)); }
            candidate.attach_serialised(manager, backend, state)
        })();
        match result {
            Ok(publication) => {
                let handle = candidate.handle.clone();
                let bounds = publication.bounds();
                self.members.push(Member { candidate, publication, bounds });
                self.active_frame = None;
                Ok(handle)
            }
            Err(error) => Err(Box::new(CharacterInsertFailure { error, candidate })),
        }
    }
    pub(crate) fn candidate(&self, handle: &SimulationDmoAssemblyHandle) -> Option<&CharacterAssemblyCandidate> {
        self.members.iter().find(|m| m.candidate.handle.same_as(handle)).map(|m| &m.candidate)
    }
    pub(crate) fn publication(&self, handle: &SimulationDmoAssemblyHandle) -> Option<&CharacterCollisionPublication> {
        self.members.iter().find(|m| m.candidate.handle.same_as(handle)).map(|m| &m.publication)
    }
    pub(crate) fn try_set_body_id(&mut self, handle: &SimulationDmoAssemblyHandle, id: u32)
        -> Result<(), SimulationDmoPoolError<Infallible>> {
        self.members.iter_mut().find(|m| m.candidate.handle.same_as(handle))
            .ok_or(SimulationDmoPoolError::MissingMember)?.candidate.body_id = Some(id);
        Ok(())
    }
    /// Base Remove82e0c1c8. Caller must synchronize pool update work first.
    pub(crate) fn try_remove(&mut self, backend: &mut SourceBackend, handle: &SimulationDmoAssemblyHandle)
        -> Result<CharacterAssemblyCandidate, SimulationDmoPoolError<Infallible>> {
        if backend.authority() != self.backend { return Err(SimulationDmoPoolError::ForeignBackend); }
        let index = self.members.iter().position(|m| m.candidate.handle.same_as(handle))
            .ok_or(SimulationDmoPoolError::MissingMember)?;
        self.members[index].candidate.try_detach(backend).map_err(SimulationDmoPoolError::Detach)?;
        self.active_frame = None;
        Ok(self.members.swap_remove(index).candidate)
    }
    /// Synchronous host execution of BeginUpdate82e0dc98/EndUpdate82e0bb10:
    /// all volume updates, all bounds jobs, all header copies, live active scan.
    /// No background job survives this call, so removal has no outstanding job.
    pub(crate) fn try_refresh(&mut self, backend: &SourceBackend) -> Result<(), CharacterPoolError> {
        self.invalidate_refresh();
        let result = (|| {
            if backend.authority() != self.backend {
                return Err(CharacterPoolError::Pool(SimulationDmoPoolError::ForeignBackend));
            }
            let frame = backend.collision_body_pool().frame_identity()
                .ok_or(CharacterPoolError::Publication(CharacterPublicationError::FrameUnavailable))?;
            for member in &mut self.members {
                member.publication.try_update_volumes(&member.candidate.assembly, backend)
                    .map_err(CharacterPoolError::Publication)?;
            }
            for member in &mut self.members {
                member.bounds = member.publication.try_stage_bounds(&member.candidate.assembly, backend)
                    .map_err(CharacterPoolError::Publication)?;
            }
            for member in &mut self.members {
                member.publication.publish_unstamped_bounds(member.bounds);
                member.publication.stamp_bounds(frame);
            }
            self.active_ordinals.clear();
            for (index, member) in self.members.iter().enumerate() {
                if !member.publication.try_all_parts_frozen(&member.candidate.assembly, backend)
                    .map_err(CharacterPoolError::Publication)? {
                    self.active_ordinals.push(index);
                }
            }
            self.active_frame = Some(frame);
            Ok(())
        })();
        if result.is_err() { self.invalidate_refresh(); }
        result
    }
    pub(crate) fn invalidate_refresh(&mut self) {
        self.active_frame = None;
        for member in &mut self.members { member.publication.invalidate_refresh(); }
    }
}
