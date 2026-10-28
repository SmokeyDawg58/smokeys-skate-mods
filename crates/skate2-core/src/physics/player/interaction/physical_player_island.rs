//! Source Island Body membership and actual Part spy targets (0x827b66d0,
//! 0x827b7830, 0x827b6988). This is a projection over the SAME `SkaterPool` and
//! retained physical-player construction, not a second body manager or Entity
//! token. Body IDs are separate from manager Assembly indices and never refunded.
//!
//! Exclusive manager access replaces the native lock. Checked host reservations
//! precede map stores; list-allocation failure retains an already issued ID and
//! membership prefix. No native pointer ordering is claimed for the hash map.
//! The actual contact-list child owns Body vectors; cleanup erases them here
//! after spy-target removal and before the actual assembly is detached.
use super::SimulationAssemblyConstraintSession;
use crate::{
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            physical::SimulationPhysicalBodyKind,
        },
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle,
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
    skateboard_hook::SkateboardHookDefinition,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{collections::HashMap, fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum SimulationPhysicalBodyError {
    Pool(SimulationSkaterPoolError),
    ForeignManager,
    LivingWorldContactOwnerPending,
    Vehicle(vehicle::VehicleIslandError),
    Dmo(vehicle::VehicleIslandError),
    Pedestrian(vehicle::VehicleIslandError),
    AlreadyMember,
    MissingMembership,
    Allocation,
    IncompleteRegistration,
    IncompleteMembership,
    TargetMismatch,
    RequestedStateMismatch { expected: u32, actual: u32 },
    MissingTarget { part: usize },
    TargetSpyBitMissing { part: u32, state: u32 },
    TargetCountMismatch { expected: usize, actual: usize },
}

/// Controlled corruption/cleanup checks over the mounted caller's ACTUAL
/// completed owner. No writable map or body handle is exposed to the caller.
#[cfg(test)]
#[path = "physical_player_island_tests.rs"]
pub(crate) mod tests;
impl fmt::Display for SimulationPhysicalBodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "physical Island member: {error}"),
            Self::ForeignManager => f.write_str("physical construction belongs to another manager"),
            Self::Vehicle(error) => write!(f, "vehicle Island owner: {error:?}"),
            Self::Dmo(error) => write!(f, "DMO Island owner: {error:?}"),
            Self::Pedestrian(error) => write!(f, "pedestrian Island owner: {error:?}"),
            Self::LivingWorldContactOwnerPending => {
                f.write_str("LivingWorld contact owner dispatch not connected")
            }
            Self::AlreadyMember => f.write_str("physical body already has Island membership"),
            Self::MissingMembership => {
                f.write_str("physical body has no retained Island membership")
            }
            Self::Allocation => f.write_str("physical Island host allocation failed"),
            Self::IncompleteRegistration => {
                f.write_str("physical body registration has not completed")
            }
            Self::IncompleteMembership => {
                f.write_str("physical body list publication has not completed")
            }
            Self::TargetMismatch => {
                f.write_str("physical target differs from its actual member or original Part")
            }
            Self::RequestedStateMismatch { expected, actual } => write!(
                f,
                "physical membership state {actual:#x} differs from actual Insert request {expected:#x}"
            ),
            Self::MissingTarget { part } => {
                write!(f, "physical Part {part} has no registered target")
            }
            Self::TargetSpyBitMissing { part, state } => write!(
                f,
                "physical Part {part} has no spy bit in live state {state:#x}"
            ),
            Self::TargetCountMismatch { expected, actual } => write!(
                f,
                "physical registration expects {expected} targets, found {actual}"
            ),
        }
    }
}
impl std::error::Error for SimulationPhysicalBodyError {}

/// Actual construction ownership shared by a paired board and skeleton. It is
/// deliberately not a native Body layout or a writable session capability.
pub(crate) struct SimulationPhysicalBodyMember {
    handle: SimulationSkaterAssemblyHandle,
    owner: Arc<PhysicalPlayerAssemblyOwner>,
    kind: SimulationPhysicalBodyKind,
    body_id: u32,
    requested_state: u32,
    list_published: bool,
    registered_parts: usize,
    registration_complete: bool,
    cleanup_completed_parts: Option<usize>,
}
impl SimulationPhysicalBodyMember {
    pub(crate) fn handle(&self) -> &SimulationSkaterAssemblyHandle {
        &self.handle
    }
    pub(crate) fn owner(&self) -> &Arc<PhysicalPlayerAssemblyOwner> {
        &self.owner
    }
    pub(crate) fn kind(&self) -> SimulationPhysicalBodyKind {
        self.kind
    }
    pub(crate) fn body_id(&self) -> u32 {
        self.body_id
    }
    pub(crate) fn requested_state(&self) -> u32 {
        self.requested_state
    }
    pub(crate) fn list_published(&self) -> bool {
        self.list_published
    }
    /// Current number of retained map entries, not an ascending prefix count.
    pub(crate) fn registered_parts(&self) -> usize {
        self.registered_parts
    }
    pub(crate) fn registration_complete(&self) -> bool {
        self.registration_complete
    }
    /// None before cleanup; Some(n) is the completed ascending Part-operation
    /// prefix of the latest cleanup attempt, including source map misses.
    pub(crate) fn cleanup_completed_parts(&self) -> Option<usize> {
        self.cleanup_completed_parts
    }
}

pub(crate) struct SimulationPhysicalBodyTarget {
    member: SimulationSkaterAssemblyHandle,
    part: u32,
}
impl SimulationPhysicalBodyTarget {
    pub(crate) fn member(&self) -> &SimulationSkaterAssemblyHandle {
        &self.member
    }
    pub(crate) fn part(&self) -> u32 {
        self.part
    }
}

/// The Island's original static/dynamic lists contain every Body category.
/// Keep one list and one ID allocator; typed handles identify existing pools.
#[derive(Clone, Debug)]
pub(crate) enum SimulationIslandBodyHandle {
    Skater(SimulationSkaterAssemblyHandle),
    Vehicle(SimulationDmoAssemblyHandle),
    Dmo(SimulationDmoAssemblyHandle),
    Pedestrian(SimulationDmoAssemblyHandle),
}
impl SimulationIslandBodyHandle {
    pub(crate) fn same_body(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Skater(a), Self::Skater(b)) => a.same_as(b),
            (Self::Vehicle(a), Self::Vehicle(b)) => a.same_as(b),
            (Self::Dmo(a), Self::Dmo(b)) => a.same_as(b),
            (Self::Pedestrian(a), Self::Pedestrian(b)) => a.same_as(b),
            _ => false,
        }
    }
    pub(crate) fn same_as(&self, skater: &SimulationSkaterAssemblyHandle) -> bool {
        matches!(self, Self::Skater(handle) if handle.same_as(skater))
    }
    fn same_vehicle(&self, vehicle: &SimulationDmoAssemblyHandle) -> bool {
        matches!(self, Self::Vehicle(handle) if handle.same_as(vehicle))
    }
}

enum IslandSpyTarget {
    Physical(SimulationPhysicalBodyTarget),
    LivingWorld {
        kind: vehicle::LivingWorldBodyKind,
        member: SimulationDmoAssemblyHandle,
        part: u32,
    },
}
impl IslandSpyTarget {
    fn physical(&self) -> Option<&SimulationPhysicalBodyTarget> {
        match self {
            Self::Physical(value) => Some(value),
            _ => None,
        }
    }
    #[cfg(test)]
    fn physical_mut(&mut self) -> Option<&mut SimulationPhysicalBodyTarget> {
        match self {
            Self::Physical(value) => Some(value),
            _ => None,
        }
    }
}

#[path = "../../../living_world/entities/living_world_contact_owners.rs"]
pub(crate) mod contact_owners;
#[path = "../../../living_world/vehicles/living_world_vehicle_island.rs"]
pub(crate) mod vehicle;

pub(crate) struct SimulationPhysicalBodyRegistry {
    next_body_id: u32,
    members: Vec<SimulationPhysicalBodyMember>,
    static_order: Vec<SimulationIslandBodyHandle>,
    dynamic_order: Vec<SimulationIslandBodyHandle>,
    targets: HashMap<SimulationRigidBodyObservation, IslandSpyTarget>,
    living_world_members: Vec<vehicle::LivingWorldIslandMember>,
}
impl SimulationPhysicalBodyRegistry {
    pub(crate) fn new() -> Self {
        Self {
            next_body_id: 1,
            members: Vec::new(),
            static_order: Vec::new(),
            dynamic_order: Vec::new(),
            targets: HashMap::new(),
            living_world_members: Vec::new(),
        }
    }
    fn allocate_body_id(&mut self, requested_state: u32) -> u32 {
        if requested_state == 1 {
            0
        } else {
            let issued = self.next_body_id;
            self.next_body_id = issued.wrapping_add(1);
            issued
        }
    }
    fn note_replaced_target(&mut self, previous: IslandSpyTarget) {
        match previous {
            IslandSpyTarget::Physical(target) => self.update_registered_count(&target.member),
            IslandSpyTarget::LivingWorld { kind, member, .. } => {
                self.update_living_world_registered_count(kind, &member)
            }
        }
    }
    pub(crate) fn next_body_id(&self) -> u32 {
        self.next_body_id
    }
    pub(crate) fn member_count(&self) -> usize {
        self.members.len()
    }
    pub(crate) fn target_count(&self) -> usize {
        self.targets.len()
    }
    pub(crate) fn static_order(&self) -> &[SimulationIslandBodyHandle] {
        &self.static_order
    }
    pub(crate) fn dynamic_order(&self) -> &[SimulationIslandBodyHandle] {
        &self.dynamic_order
    }
    fn member_index(&self, handle: &SimulationSkaterAssemblyHandle) -> Option<usize> {
        self.members
            .iter()
            .position(|member| member.handle.same_as(handle))
    }
    fn update_registered_count(&mut self, handle: &SimulationSkaterAssemblyHandle) {
        if let Some(index) = self.member_index(handle) {
            self.members[index].registered_parts = self
                .targets
                .values()
                .filter_map(IslandSpyTarget::physical)
                .filter(|target| target.member.same_as(handle))
                .count();
        }
    }
}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Narrow native Body-owner relation for contact lists. A valid partial
    /// target map is allowed; unrelated Part coverage is not sampled here.
    /// Unlike diagnostics this does not certify completed registration.
    pub(crate) fn physical_body_owner_for_contacts(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, SimulationPhysicalBodyError> {
        use SimulationPhysicalBodyError as Error;
        let (_, owner) = self
            .skater_pool()
            .map_err(Error::Pool)?
            .try_physical_member_owner(&self.backend, handle)
            .map_err(Error::Pool)?;
        let index = self
            .physical_bodies
            .member_index(handle)
            .ok_or(Error::MissingMembership)?;
        if owner.manager() != self.contacts.assembly_pool_authority()
            || !Arc::ptr_eq(owner, &self.physical_bodies.members[index].owner)
        {
            return Err(Error::ForeignManager);
        }
        Ok(owner)
    }
    /// Called AFTER actual `SkaterPool` Insert. Requested state ==1, not effective
    /// Part state or bit membership, selects ID0/static; all others issue next ID.
    pub(crate) fn try_add_physical_body_membership(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        requested_state: u32,
    ) -> Result<u32, SimulationPhysicalBodyError> {
        use SimulationPhysicalBodyError as Error;
        let (kind, owner) = self
            .skater_pool()
            .map_err(Error::Pool)?
            .try_physical_member_owner(&self.backend, handle)
            .map_err(Error::Pool)?;
        if owner.manager() != self.contacts.assembly_pool_authority() {
            return Err(Error::ForeignManager);
        }
        let expected = self
            .skater_pool()
            .map_err(Error::Pool)?
            .try_physical_requested_state(&self.backend, handle)
            .map_err(Error::Pool)?;
        if expected != requested_state {
            return Err(Error::RequestedStateMismatch {
                expected,
                actual: requested_state,
            });
        }
        if self.physical_bodies.member_index(handle).is_some() {
            return Err(Error::AlreadyMember);
        }
        let owner = Arc::clone(owner);
        self.physical_bodies
            .members
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        let body_id = self.physical_bodies.allocate_body_id(requested_state);
        // SAME retained Body construction receives the issued ID before list
        // allocation; removing membership later cannot discard/reset that ID.
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_set_physical_body_id(&self.backend, handle, body_id)
            .map_err(Error::Pool)?;
        self.physical_bodies
            .members
            .push(SimulationPhysicalBodyMember {
                handle: handle.clone(),
                owner,
                kind,
                body_id,
                requested_state,
                list_published: false,
                registered_parts: 0,
                registration_complete: false,
                cleanup_completed_parts: None,
            });
        // Source push_back occurs AFTER Body Island/ID publication. On host
        // allocation refusal the retained member reports this incomplete prefix.
        let list = if requested_state == 1 {
            &mut self.physical_bodies.static_order
        } else {
            &mut self.physical_bodies.dynamic_order
        };
        list.try_reserve(1).map_err(|_| Error::Allocation)?;
        list.push(SimulationIslandBodyHandle::Skater(handle.clone()));
        self.physical_bodies
            .members
            .last_mut()
            .expect("just published member")
            .list_published = true;
        Ok(body_id)
    }

    /// `SyncCheckBody` is lock-only in this build; exclusive access already covers
    /// it. Register actual Parts ascending, state prefix BEFORE replacing map.
    /// Repeated registration replaces the SAME keys and original ordinal tags.
    pub(crate) fn try_register_physical_body_targets(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationPhysicalBodyError> {
        use SimulationPhysicalBodyError as Error;
        let member_index = self
            .physical_bodies
            .member_index(handle)
            .ok_or(Error::MissingMembership)?;
        if !self.physical_bodies.members[member_index].list_published {
            return Err(Error::IncompleteMembership);
        }
        let pool = self
            .skaters
            .as_ref()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?;
        let (_, owner) = pool
            .try_physical_member_owner(&self.backend, handle)
            .map_err(Error::Pool)?;
        if !Arc::ptr_eq(owner, &self.physical_bodies.members[member_index].owner)
            || owner.manager() != self.contacts.assembly_pool_authority()
        {
            return Err(Error::ForeignManager);
        }
        let count = pool
            .try_physical_part_count(&self.backend, handle)
            .map_err(Error::Pool)?;
        self.physical_bodies.members[member_index].registration_complete = false;
        self.physical_bodies.members[member_index].cleanup_completed_parts = None;
        for part in 0..count {
            // Actual source producers have at most24 Parts. The existing
            // assembly template has already checked every count fits u32.
            let part_tag = u32::try_from(part).expect("validated assembly Part count");
            self.physical_bodies
                .targets
                .try_reserve(1)
                .map_err(|_| Error::Allocation)?;
            let observation = pool
                .try_register_physical_part(&mut self.backend, handle, part)
                .map_err(Error::Pool)?;
            if let Some(previous) = self.physical_bodies.targets.insert(
                observation,
                IslandSpyTarget::Physical(SimulationPhysicalBodyTarget {
                    member: handle.clone(),
                    part: part_tag,
                }),
            ) {
                self.physical_bodies.note_replaced_target(previous);
            }
            self.physical_bodies.update_registered_count(handle);
        }
        self.try_validate_physical_target_coverage(handle)?;
        self.physical_bodies.members[member_index].registration_complete = true;
        Ok(())
    }

    pub(crate) fn try_mark_physical_board_completed(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationPhysicalBodyError> {
        use SimulationPhysicalBodyError as Error;
        let member = self.physical_body_member(handle)?;
        if !member.registration_complete {
            return Err(Error::IncompleteRegistration);
        }
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_mark_physical_board_completed(&self.backend, handle)
            .map_err(Error::Pool)
    }

    pub(crate) fn physical_body_registry(&self) -> &SimulationPhysicalBodyRegistry {
        &self.physical_bodies
    }

    /// Diagnostics validate the current member and every published occurrence;
    /// retained registry metadata cannot certify a stale/reused body slot.
    pub(crate) fn physical_body_member(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&SimulationPhysicalBodyMember, SimulationPhysicalBodyError> {
        use SimulationPhysicalBodyError as Error;
        let pool = self.skater_pool().map_err(Error::Pool)?;
        let (_, owner) = pool
            .try_physical_member_owner(&self.backend, handle)
            .map_err(Error::Pool)?;
        let index = self
            .physical_bodies
            .member_index(handle)
            .ok_or(Error::MissingMembership)?;
        if !Arc::ptr_eq(owner, &self.physical_bodies.members[index].owner) {
            return Err(Error::ForeignManager);
        }
        let count = pool
            .try_physical_part_count(&self.backend, handle)
            .map_err(Error::Pool)?;
        for part in 0..count {
            pool.try_physical_part_observation(&self.backend, handle, part)
                .map_err(Error::Pool)?;
        }
        for (observation, target) in &self.physical_bodies.targets {
            let Some(target) = target.physical() else {
                continue;
            };
            if target.member.same_as(handle) {
                self.try_validate_physical_target(*observation, target)?;
            }
        }
        let actual = self
            .physical_bodies
            .targets
            .values()
            .filter_map(IslandSpyTarget::physical)
            .filter(|target| target.member.same_as(handle))
            .count();
        if actual != self.physical_bodies.members[index].registered_parts {
            return Err(Error::TargetCountMismatch {
                expected: self.physical_bodies.members[index].registered_parts,
                actual,
            });
        }
        if self.physical_bodies.members[index].registration_complete {
            self.try_validate_physical_target_coverage(handle)?;
        }
        Ok(&self.physical_bodies.members[index])
    }

    fn try_validate_physical_target(
        &self,
        observation: SimulationRigidBodyObservation,
        target: &SimulationPhysicalBodyTarget,
    ) -> Result<(), SimulationPhysicalBodyError> {
        use SimulationPhysicalBodyError as Error;
        if self.physical_bodies.member_index(&target.member).is_none() {
            return Err(Error::MissingMembership);
        }
        let actual = self
            .skater_pool()
            .map_err(Error::Pool)?
            .try_physical_part_observation(&self.backend, &target.member, target.part as usize)
            .map_err(Error::Pool)?;
        if actual != observation {
            return Err(Error::TargetMismatch);
        }
        let state = self
            .backend
            .collision_body_pool()
            .try_raw_state(observation)
            .map_err(|error| {
                Error::Pool(SimulationSkaterPoolError::PhysicalPart(
                    DynamicAssemblyPartContactSpyError::Body(error),
                ))
            })?;
        if state & 8 == 0 {
            return Err(Error::TargetSpyBitMissing {
                part: target.part,
                state,
            });
        }
        Ok(())
    }

    /// A historical completed loop is not a current completion capability.
    /// Every actual Part must still resolve to its exact target/tag and spy bit.
    fn try_validate_physical_target_coverage(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationPhysicalBodyError> {
        use SimulationPhysicalBodyError as Error;
        let pool = self.skater_pool().map_err(Error::Pool)?;
        let count = pool
            .try_physical_part_count(&self.backend, handle)
            .map_err(Error::Pool)?;
        let actual = self
            .physical_bodies
            .targets
            .values()
            .filter_map(IslandSpyTarget::physical)
            .filter(|target| target.member.same_as(handle))
            .count();
        if actual != count {
            return Err(Error::TargetCountMismatch {
                expected: count,
                actual,
            });
        }
        for part in 0..count {
            let observation = pool
                .try_physical_part_observation(&self.backend, handle, part)
                .map_err(Error::Pool)?;
            let target = self
                .physical_bodies
                .targets
                .get(&observation)
                .and_then(IslandSpyTarget::physical)
                .ok_or(Error::MissingTarget { part })?;
            if !target.member.same_as(handle) || target.part as usize != part {
                return Err(Error::TargetMismatch);
            }
            self.try_validate_physical_target(observation, target)?;
        }
        Ok(())
    }

    pub(crate) fn physical_body_target(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<Option<&SimulationPhysicalBodyTarget>, SimulationPhysicalBodyError> {
        use SimulationPhysicalBodyError as Error;
        self.backend
            .collision_body_pool()
            .try_validate_observation(observation)
            .map_err(|error| {
                Error::Pool(SimulationSkaterPoolError::PhysicalPart(
                    DynamicAssemblyPartContactSpyError::Body(error),
                ))
            })?;
        let target = match self.physical_bodies.targets.get(&observation) {
            Some(IslandSpyTarget::Physical(target)) => Some(target),
            Some(IslandSpyTarget::LivingWorld { .. }) => {
                return Err(Error::LivingWorldContactOwnerPending);
            }
            None => None,
        };
        if let Some(target) = target {
            self.try_validate_physical_target(observation, target)?;
        }
        Ok(target)
    }

    /// Called by existing typed board/skeleton removal wrappers. Legacy members
    /// have no Island registration and retain their unchanged direct removal.
    pub(super) fn try_prepare_physical_body_cleanup(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        kind: SimulationPhysicalBodyKind,
    ) -> Result<(), SimulationSkaterPoolError> {
        let Some(index) = self.physical_bodies.member_index(handle) else {
            return Ok(());
        };
        self.physical_bodies.members[index]
            .owner
            .require_controller_drives_removed()?;
        let pool = self
            .skaters
            .as_ref()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?;
        pool.try_validate_physical_detach(&self.backend, handle, kind)?;
        let count = pool.try_physical_part_count(&self.backend, handle)?;
        // All detach guards passed before ANY state/map mutation. Failure of a
        // later actual Part operation preserves the completed removal prefix.
        self.physical_bodies.members[index].registration_complete = false;
        self.physical_bodies.members[index].cleanup_completed_parts = Some(0);
        for part in 0..count {
            let observation =
                pool.try_remove_physical_part_target(&mut self.backend, handle, part)?;
            if let Some(previous) = self.physical_bodies.targets.remove(&observation) {
                self.physical_bodies.note_replaced_target(previous);
            }
            self.physical_bodies.members[index].cleanup_completed_parts = Some(part + 1);
        }
        // Native contact-entry erasure precedes actual pool Remove.
        self.physical_contacts.remove_body(handle);
        Ok(())
    }

    pub(super) fn finish_physical_body_cleanup(&mut self, handle: &SimulationSkaterAssemblyHandle) {
        // Actual SkaterPool removal completed first. Native RemoveSkaterBody then
        // removes ALL matching static-list entries followed by dynamic entries.
        self.physical_bodies
            .static_order
            .retain(|entry| !entry.same_as(handle));
        self.physical_bodies
            .dynamic_order
            .retain(|entry| !entry.same_as(handle));
        self.physical_bodies
            .members
            .retain(|member| !member.handle.same_as(handle));
        // next_body_id is intentionally not reset/refunded.
    }
}
