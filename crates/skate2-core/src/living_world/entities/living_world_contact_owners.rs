//! Typed Body/mpOwner reads of CreateContactSpyList827ba710 and board contact
//! consumers. Native compares pointers; existing host physical construction Arc
//! and same-world LivingWorld entity ID identify those retained owners.
use super::vehicle::{LivingWorldBodyKind, VehicleIslandError};
use super::*;
fn owner_error(
    kind: LivingWorldBodyKind,
    error: VehicleIslandError,
) -> SimulationPhysicalBodyError {
    match kind {
        LivingWorldBodyKind::Dmo => SimulationPhysicalBodyError::Dmo(error),
        LivingWorldBodyKind::Vehicle => SimulationPhysicalBodyError::Vehicle(error),
        LivingWorldBodyKind::Pedestrian => SimulationPhysicalBodyError::Pedestrian(error),
    }
}

#[derive(Clone)]
pub(crate) enum SimulationContactEntityOwner {
    Physical(Arc<PhysicalPlayerAssemblyOwner>),
    LivingWorld(u32),
}
impl SimulationContactEntityOwner {
    pub(crate) fn same_owner(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Physical(a), Self::Physical(b)) => Arc::ptr_eq(a, b),
            (Self::LivingWorld(a), Self::LivingWorld(b)) => a == b,
            _ => false,
        }
    }
}
pub(crate) struct SimulationIslandBodyTarget {
    pub(crate) member: SimulationIslandBodyHandle,
    pub(crate) part: u32,
}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    pub(crate) fn island_body_owner_for_contacts(
        &self,
        handle: &SimulationIslandBodyHandle,
    ) -> Result<SimulationContactEntityOwner, SimulationPhysicalBodyError> {
        match handle {
            SimulationIslandBodyHandle::Skater(handle) => self
                .physical_body_owner_for_contacts(handle)
                .map(|owner| SimulationContactEntityOwner::Physical(Arc::clone(owner))),
            SimulationIslandBodyHandle::Dmo(handle) => self
                .living_world_body_member_for_contacts(LivingWorldBodyKind::Dmo, handle)
                .map(|member| SimulationContactEntityOwner::LivingWorld(member.entity_id))
                .map_err(SimulationPhysicalBodyError::Dmo),
            SimulationIslandBodyHandle::Vehicle(handle) => self
                .vehicle_body_member_for_contacts(handle)
                .map(|member| SimulationContactEntityOwner::LivingWorld(member.entity_id))
                .map_err(SimulationPhysicalBodyError::Vehicle),
            SimulationIslandBodyHandle::Pedestrian(handle) => self
                .living_world_body_member_for_contacts(LivingWorldBodyKind::Pedestrian, handle)
                .map(|member| SimulationContactEntityOwner::LivingWorld(member.entity_id))
                .map_err(SimulationPhysicalBodyError::Pedestrian),
        }
    }

    pub(crate) fn island_body_group_for_contacts(
        &self,
        handle_kind: &SimulationIslandBodyHandle,
    ) -> Result<u32, SimulationPhysicalBodyError> {
        match handle_kind {
            SimulationIslandBodyHandle::Skater(handle) => self
                .skater_pool()
                .and_then(|pool| pool.physical_assembly_group(handle))
                .map_err(SimulationPhysicalBodyError::Pool),
            SimulationIslandBodyHandle::Vehicle(handle)
            | SimulationIslandBodyHandle::Dmo(handle)
            | SimulationIslandBodyHandle::Pedestrian(handle) => {
                let kind = if matches!(handle_kind, SimulationIslandBodyHandle::Dmo(_)) {
                    LivingWorldBodyKind::Dmo
                } else if matches!(handle_kind, SimulationIslandBodyHandle::Pedestrian(_)) {
                    LivingWorldBodyKind::Pedestrian
                } else {
                    LivingWorldBodyKind::Vehicle
                };
                self.living_world_body_member_for_contacts(kind, handle)
                    .map_err(|e| owner_error(kind, e))?;
                let pool = self.living_world_pool(kind).map_err(|e| owner_error(kind, e))?;
                Ok(pool.island_body(handle).expect("validated above").group)
            }
        }
    }

    pub(crate) fn island_body_target(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<Option<SimulationIslandBodyTarget>, SimulationPhysicalBodyError> {
        use SimulationPhysicalBodyError as E;
        self.backend
            .collision_body_pool()
            .try_validate_observation(observation)
            .map_err(|error| {
                E::Pool(SimulationSkaterPoolError::PhysicalPart(
                    DynamicAssemblyPartContactSpyError::Body(error),
                ))
            })?;
        let Some(target) = self.physical_bodies.targets.get(&observation) else {
            return Ok(None);
        };
        Ok(Some(match target {
            IslandSpyTarget::Physical(target) => {
                self.try_validate_physical_target(observation, target)?;
                SimulationIslandBodyTarget {
                    member: SimulationIslandBodyHandle::Skater(target.member.clone()),
                    part: target.part,
                }
            }
            IslandSpyTarget::LivingWorld { kind, member, part } => {
                self.living_world_body_member_for_contacts(*kind, member)
                    .map_err(|e| owner_error(*kind, e))?;
                let pool = self.living_world_pool(*kind).map_err(|e| owner_error(*kind, e))?;
                let candidate = pool.island_body(member).expect("validated above");
                let actual = candidate
                    .parts
                    .try_part_contact_spy_observation(&self.backend, *part as usize)
                    .map_err(|e| owner_error(*kind, VehicleIslandError::Spy(e)))?;
                if actual != observation {
                    return Err(E::TargetMismatch);
                }
                let state = self
                    .backend
                    .collision_body_pool()
                    .try_raw_state(observation)
                    .map_err(|e| {
                        owner_error(
                            *kind,
                            VehicleIslandError::Spy(DynamicAssemblyPartContactSpyError::Body(e)),
                        )
                    })?;
                if state & 8 == 0 {
                    return Err(E::TargetSpyBitMissing { part: *part, state });
                }
                SimulationIslandBodyTarget {
                    member: kind.body(member),
                    part: *part,
                }
            }
        }))
    }
}
