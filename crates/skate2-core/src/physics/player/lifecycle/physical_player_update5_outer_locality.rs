//! Same Actor/pair Update5 outer stores82e04f88..53ec.
use super::{update5_managers::GrindManagerFillPolicy, update5_wipeout::Update5AfterWipeoutFill};
use crate::{
    actor_construction::{
        ActorConstruction, ActorConstructionError, physical_mount::ActorPhysicalMountError,
    },
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::InputCvarError,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::outer_locality::{
        Update5OuterLocalityError, Update5OuterLocalitySource,
    },
    physical_player_physics_state::PhysicalStateActorQuery,
    skater_manager_local_identity::{LocalActorQueryError, SkaterManagerLocalIdentity},
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum OuterLocalityRuntimeError {
    Poisoned,
    Missing(&'static str),
    Cvar(InputCvarError),
    LocalActor(LocalActorQueryError<ActorConstructionError>),
}
impl std::fmt::Display for OuterLocalityRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Update5 outer owner poisoned"),
            Self::Missing(field) => write!(f, "Update5 outer requires {field}"),
            Self::Cvar(error) => error.fmt(f),
            Self::LocalActor(error) => error.fmt(f),
        }
    }
}
#[derive(Debug)]
pub(crate) enum Update5OuterSessionError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    ForeignActor,
    ForeignWorld,
    Poisoned,
    Actor(ActorPhysicalMountError),
    Output(Update5OuterLocalityError<OuterLocalityRuntimeError>),
}
impl std::fmt::Display for Update5OuterSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Update5 outer requires skater pool"),
            Self::Pair(error) => error.fmt(f),
            Self::ForeignOwner => f.write_str("Update5 outer belongs to another pair"),
            Self::ForeignActor => f.write_str("Update5 outer belongs to another Actor"),
            Self::ForeignWorld => f.write_str("Update5 outer belongs to another World/GameInfo"),
            Self::Poisoned => f.write_str("Update5 outer session poisoned"),
            Self::Actor(error) => write!(f, "Update5 Actor allocation: {error:?}"),
            Self::Output(error) => error.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeCurrentType {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) grind_policy: GrindManagerFillPolicy,
}
struct Source<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    actor: &'a ActorConstruction,
    manager: Option<&'a Arc<SkaterManagerLocalIdentity>>,
}
impl Source<'_> {
    fn capture(&self) -> Result<Arc<Mutex<SkeletonProcessedContact>>, OuterLocalityRuntimeError> {
        Ok(Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| OuterLocalityRuntimeError::Poisoned)?
                .processed(),
        ))
    }
    fn read<T>(
        &self,
        field: &'static str,
        read: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, OuterLocalityRuntimeError> {
        let processed = self.capture()?;
        let processed = processed
            .lock()
            .map_err(|_| OuterLocalityRuntimeError::Poisoned)?;
        read(&processed).ok_or(OuterLocalityRuntimeError::Missing(field))
    }
    fn flag(
        processed: &SkeletonProcessedContact,
        offset: u32,
        bit: u32,
    ) -> Result<bool, OuterLocalityRuntimeError> {
        use OuterLocalityRuntimeError as E;
        let mask = 1u32.checked_shl(bit).ok_or(E::Missing("valid flag bit"))?;
        let word = match offset {
            0x8c4 => {
                let (value, known) = processed.known_flags_8c4();
                if known & mask != mask {
                    return Err(E::Missing("Processed8c4 named bit"));
                }
                Some(value)
            }
            0x8c8 => processed.known_flags_8c8(),
            0x8cc => processed.physics_state_fields().flags_8cc,
            0x8d0 => processed.physics_state_fields().flags_8d0,
            _ => None,
        }
        .ok_or(E::Missing("Processed flag word"))?;
        Ok(word & mask != 0)
    }
}
impl Update5OuterLocalitySource for Source<'_> {
    type Error = OuterLocalityRuntimeError;
    type Processed = Arc<Mutex<SkeletonProcessedContact>>;
    fn processed_flag(&mut self, offset: u32, bit: u32) -> Result<bool, Self::Error> {
        let processed = self.capture()?;
        let processed = processed.lock().map_err(|_| Self::Error::Poisoned)?;
        Self::flag(&processed, offset, bit)
    }
    fn debug_render_local_sim_player(&mut self) -> Result<bool, Self::Error> {
        self.owner
            .inputs()
            .cvars
            .try_debug_render_local_sim_player()
            .map_err(Self::Error::Cvar)
    }
    fn is_local_player(&mut self) -> Result<bool, Self::Error> {
        let manager = self
            .manager
            .ok_or(Self::Error::Missing("Simulation SkaterManager local owner"))?;
        manager
            .try_is_local_player(|| self.actor.player_id())
            .map_err(Self::Error::LocalActor)
    }
    fn is_local_sim_player(&mut self) -> Result<bool, Self::Error> {
        let manager = self
            .manager
            .ok_or(Self::Error::Missing("Simulation SkaterManager local owner"))?;
        manager
            .try_is_local_sim_player(|| self.actor.player_id())
            .map_err(Self::Error::LocalActor)
    }
    fn state_count(&mut self) -> Result<i32, Self::Error> {
        self.owner
            .physics_state()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .state_count
            .ok_or(Self::Error::Missing("HiLOD state count"))
    }
    fn collide_with_world_only(&mut self) -> Result<bool, Self::Error> {
        Ok(self
            .owner
            .tail()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .collide_with_world_only)
    }
    fn turn(&mut self) -> Result<f32, Self::Error> {
        self.read("Processed turn980", |p| p.animation_attribute_fields().turn)
    }
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error> {
        self.capture()
    }
    fn ground_position(
        &mut self,
        owner: &Self::Processed,
    ) -> Result<SkeletonRootVector, Self::Error> {
        owner
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .ground_position_fields_mut()
            .ground_position
            .ok_or(Self::Error::Missing("Processed ground position1e0"))
    }
    fn captured_flag(
        &mut self,
        owner: &Self::Processed,
        offset: u32,
        bit: u32,
    ) -> Result<bool, Self::Error> {
        let processed = owner.lock().map_err(|_| Self::Error::Poisoned)?;
        Self::flag(&processed, offset, bit)
    }
    fn jump_strength(&mut self) -> Result<f32, Self::Error> {
        self.read("Processed jump strength950", |p| p.jump_events().strength())
    }
    fn balance(&mut self) -> Result<f32, Self::Error> {
        self.read("Processed balance9a0", |p| p.physics_state_fields().balance)
    }
}
impl PhysicalAssemblyWorld {
    /// Identity checks are host preconditions. Missing manager is rejected only
    /// at the actual Actor locality query, retaining six prior output stores.
    pub(crate) fn try_continue_update5_outer_locality(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5AfterWipeoutFill,
        actor: &ActorConstruction,
    ) -> Result<Update5BeforeCurrentType, Update5OuterSessionError> {
        use Update5OuterSessionError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| {
                Arc::ptr_eq(owner, &before.owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        if !Arc::ptr_eq(
            actor
                .registered_physical_allocation()
                .map_err(E::Actor)?
                .owner(),
            &before.owner,
        ) || !before
            .owner
            .actor_query
            .lock()
            .map_err(|_| E::Poisoned)?
            .as_ref()
            .is_some_and(|query| query.belongs_to(actor))
        {
            return Err(E::ForeignActor);
        }
        if let Some(manager) = self.local_skater_identity.as_ref() {
            let services = self.physical_world_services().ok_or(E::ForeignWorld)?;
            if !before
                .owner
                .world_services()
                .is_some_and(|owned| Arc::ptr_eq(owned, services))
                || !manager.backend().shares_game_info(services.game_info())
            {
                return Err(E::ForeignWorld);
            }
        }
        {
            let mut source = Source {
                owner: &before.owner,
                actor,
                manager: self.local_skater_identity.as_ref(),
            };
            before
                .owner
                .phys_out()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_publish_update5_outer_locality(&mut source)
                .map_err(E::Output)?;
        }
        Ok(Update5BeforeCurrentType {
            owner: before.owner,
            grind_policy: before.grind_policy,
        })
    }
}
