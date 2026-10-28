//! Actor physical allocation825ff2d8 and `IPhysical` registration82dfe5d0.
//! Retains the actual pair owner before selected constructor work. Publication
//! at Actor+200 waits for the actual `HiLOD` constructor suffix to return.
use super::{ActorConstruction, ActorConstructionError, ActorIdentity, ActorPrefixPhase};
use crate::{
    motion_graph_entity_authority::MotionGraphEntityAuthority,
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyConstruction, PhysicalPlayerAssemblyInputs,
        PhysicalPlayerAssemblyOwner,
    },
};
use std::sync::Arc;

/// Source interface-name function address is its registry identity, not a host
/// executable pointer. Raw82dfe61c..628 and82dfe648..658 use this address directly.
pub(crate) const PHYSICAL_INTERFACE: u32 = 0x825e_be48;

#[path = "actor_physical_input_query.rs"]
pub(crate) mod input_query;

/// Actual Actor query occurrence copied through `Init_SimulationData`. A weak
/// state reference preserves the same Actor lifetime without a physical cycle.
pub(crate) struct ActorPhysicalQueryBinding {
    state: std::sync::Weak<std::sync::Mutex<super::ActorConstructorPrefix>>,
    entity: MotionGraphEntityAuthority<ActorIdentity>,
}
impl ActorPhysicalQueryBinding {
    pub(crate) fn belongs_to(&self, actor: &ActorConstruction) -> bool {
        self.state.ptr_eq(&Arc::downgrade(&actor.state))
            && self
                .entity
                .is_same_issuance(&actor.authority.entity_handle())
    }
}

pub(crate) struct ActorPhysicalAllocation {
    entity: MotionGraphEntityAuthority<ActorIdentity>,
    owner: Arc<PhysicalPlayerAssemblyOwner>,
}
impl ActorPhysicalAllocation {
    pub(crate) fn owner(&self) -> &Arc<PhysicalPlayerAssemblyOwner> {
        &self.owner
    }
}

#[derive(Debug)]
pub(crate) enum ActorPhysicalMountError {
    Actor(ActorConstructionError),
    WrongPhase(ActorPrefixPhase),
    ForeignWorld,
    DuplicateInterface,
    MissingAllocation,
    ConstructorNotPublished,
    ForeignInterface,
    PhysicalOwnerPoisoned,
    PendingSkeletonConstructor,
}
impl std::fmt::Display for ActorPhysicalMountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "physical Actor allocation: {error}"),
            Self::WrongPhase(phase) => write!(f, "physical Actor allocation phase: {phase:?}"),
            other => write!(f, "physical Actor allocation: {other:?}"),
        }
    }
}

impl ActorConstruction {
    /// Caller store825ff300, after `HiLOD82e03c50` returns. Registration alone
    /// cannot publish this cached member, nor can body/controller completion.
    pub(crate) fn try_publish_physical_constructor_return(
        &self,
    ) -> Result<(), ActorPhysicalMountError> {
        use ActorPhysicalMountError as E;
        let physical = self.registered_physical_allocation()?;
        if !physical
            .owner()
            .tail()
            .lock()
            .map_err(|_| E::PhysicalOwnerPoisoned)?
            .returned
        {
            return Err(E::ConstructorNotPublished);
        }
        if !physical
            .owner()
            .skeleton_constructor_complete
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(E::PendingSkeletonConstructor);
        }
        let mut state = self.prefix().map_err(E::Actor)?;
        if state.phase != ActorPrefixPhase::PhysicalConstructing {
            return Err(E::WrongPhase(state.phase));
        }
        state.physical_cached = Some(physical);
        state.phase = ActorPrefixPhase::PhysicalConstructorReturned;
        Ok(())
    }
    /// Cached Actor+200 query, distinct from early UCOM registration.
    pub(crate) fn cached_physical_allocation(
        &self,
    ) -> Result<Arc<ActorPhysicalAllocation>, ActorPhysicalMountError> {
        use ActorPhysicalMountError as E;
        let state = self.prefix().map_err(E::Actor)?;
        let cached = state
            .physical_cached
            .as_ref()
            .ok_or(E::ConstructorNotPublished)?;
        if !state
            .physical_allocation
            .as_ref()
            .is_some_and(|allocation| Arc::ptr_eq(cached, allocation))
            || !state
                .physical_interface
                .as_ref()
                .is_some_and(|interface| Arc::ptr_eq(cached, interface))
            || !cached
                .entity
                .is_same_issuance(&self.authority.entity_handle())
            || state.interfaces.binary_search(&PHYSICAL_INTERFACE).is_err()
        {
            return Err(E::ForeignInterface);
        }
        Ok(Arc::clone(cached))
    }
    /// Host-success allocation path. The caller supplies selected `Init_IPhysical`
    /// data; this entry does not recover the missing outer selection services.
    /// It cannot attach an already constructed pair to an unrelated Actor.
    pub(crate) fn try_allocate_physical<'world>(
        &self,
        world: &'world mut PhysicalAssemblyWorld,
        inputs: PhysicalPlayerAssemblyInputs,
    ) -> Result<PhysicalPlayerAssemblyConstruction<'world>, ActorPhysicalMountError> {
        use ActorPhysicalMountError as E;
        if world.physical_manager_authority() != self.authority.entity_key().manager() {
            return Err(E::ForeignWorld);
        }
        let mut state = self.prefix().map_err(E::Actor)?;
        if state.phase != ActorPrefixPhase::AnimationPhysicsInputConstructed {
            return Err(E::WrongPhase(state.phase));
        }
        state.phase = ActorPrefixPhase::PhysicalConstructing;
        let construction = PhysicalPlayerAssemblyConstruction::new(world, inputs);
        *construction
            .owner()
            .actor_query
            .lock()
            .map_err(|_| E::PhysicalOwnerPoisoned)? = Some(Arc::new(ActorPhysicalQueryBinding {
            state: Arc::downgrade(&self.state),
            entity: self.authority.entity_handle(),
        }));
        let allocation = Arc::new(ActorPhysicalAllocation {
            entity: self.authority.entity_handle(),
            owner: Arc::clone(construction.owner()),
        });
        // Physical allocation and its _mCOMObject binding survive a later
        // registration or constructor failure. No body has been constructed.
        state.physical_allocation = Some(Arc::clone(&allocation));
        if state.interfaces.binary_search(&PHYSICAL_INTERFACE).is_ok() {
            return Err(E::DuplicateInterface);
        }
        state
            .register::<std::convert::Infallible>(PHYSICAL_INTERFACE)
            .map_err(E::Actor)?;
        state.physical_interface = Some(allocation);
        // HiLOD return and Actor+200 publication are deliberately not inferred
        // from the selected assembly's Complete phase.
        Ok(construction)
    }

    /// Actual registered `IPhysical` occurrence. This is explicitly not the
    /// cached Actor+200 accessor, which requires full constructor completion.
    pub(crate) fn registered_physical_allocation(
        &self,
    ) -> Result<Arc<ActorPhysicalAllocation>, ActorPhysicalMountError> {
        use ActorPhysicalMountError as E;
        let state = self.prefix().map_err(E::Actor)?;
        let allocation = state
            .physical_allocation
            .as_ref()
            .ok_or(E::MissingAllocation)?;
        let interface = state
            .physical_interface
            .as_ref()
            .ok_or(E::ForeignInterface)?;
        if !Arc::ptr_eq(allocation, interface)
            || !allocation
                .entity
                .is_same_issuance(&self.authority.entity_handle())
            || state.interfaces.binary_search(&PHYSICAL_INTERFACE).is_err()
        {
            return Err(E::ForeignInterface);
        }
        Ok(Arc::clone(allocation))
    }
}
