//! Actual same Actor/pair constructor suffix82e04c24..82e04dd8.
use super::{
    physical_skeleton_controller::{PhysicalSkeletonControllerError, SkeletonCollisionTransition},
    physical_state::PhysicalStateSessionError,
    world_services::PhysicalWorldServicesError,
};
use crate::{
    actor_construction::{
        ActorConstruction, ActorConstructionError, physical_mount::ActorPhysicalMountError,
    },
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_collision_mode::BoardCollisionModeError,
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld,
        hilod_tail::{HiLodCollisionChange, HiLodTailError},
    },
    physical_player_wipeout::WipeoutOwnerError,
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum HiLodTailSessionError<N> {
    MissingPool,
    Poisoned,
    ForeignActor,
    ForeignWorldServices,
    MissingWorldServices,
    StateConstructorsIncomplete,
    Pair(SimulationSkeletonJumpPairError),
    Actor(ActorPhysicalMountError),
    Tail(HiLodTailError),
    Ground(PhysicalStateSessionError<ActorConstructionError, N>),
    Wipeout(WipeoutOwnerError),
    Board(BoardCollisionModeError),
    Skeleton(PhysicalSkeletonControllerError),
    Services(PhysicalWorldServicesError),
}
impl<N: std::fmt::Debug> std::fmt::Display for HiLodTailSessionError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(e) => write!(f, "HiLOD tail pair: {e}"),
            Self::Actor(e) => write!(f, "HiLOD tail Actor: {e}"),
            Self::Tail(e) => write!(f, "{e}"),
            Self::Ground(e) => write!(f, "HiLOD initial Ground: {e}"),
            Self::Wipeout(e) => write!(f, "HiLOD spawn invincibility: {e:?}"),
            Self::Board(e) => write!(f, "{e}"),
            Self::Skeleton(e) => write!(f, "{e}"),
            Self::Services(e) => write!(f, "{e}"),
            other => write!(f, "HiLOD tail: {other:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_set_pair_board_collision_mode(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        world_only: bool,
    ) -> Result<(), BoardCollisionModeError> {
        self.skaters
            .as_mut()
            .ok_or(BoardCollisionModeError::MissingPool)?
            .try_set_pair_board_collision_mode(&self.backend, board, skeleton, world_only)
    }

    /// `PhysicalPlayerHiLOD::DisableNonWorldCollisionsForFadeout` 0x82dffe40: the HiLOD
    /// world-only status bit clears while `Skateboard::CollideWithWorldOnly` and
    /// `Skeleton::CollideWithWorldOnly` leave the pair colliding with the world alone, so
    /// `UpdateCollisionStatus` sees no change to undo.
    pub(crate) fn try_disable_non_world_collisions_for_fadeout(
        &mut self,
        owner: &crate::physical_player_assembly::PhysicalPlayerAssemblyOwner,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), String> {
        owner
            .tail()
            .lock()
            .map_err(|_| "HiLOD tail lock poisoned".to_owned())?
            .collide_with_world_only = false;
        self.try_set_pair_board_collision_mode(board, skeleton, true)
            .map_err(|error| error.to_string())?;
        self.skaters
            .as_mut()
            .ok_or_else(|| "skater pool unavailable".to_owned())?
            .try_with_collision_pair(&self.backend, board, skeleton, |normal, _| {
                normal.try_collide_with_world_only()
            })
            .map_err(|error| error.to_string())?
            .map_err(|error| format!("{error:?}"))
    }

    /// All26 state sites must already have executed. The actual Actor is also
    /// the Ground transition query owner. No cached physical pointer is written
    /// here; Actor's source caller performs that only after successful return.
    #[expect(
        clippy::too_many_lines,
        reason = "native constructor tail ordering and partial ownership are retained"
    )]
    pub(crate) fn try_finish_pair_hilod_constructor<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        mut actor: &ActorConstruction,
        numeric: &mut N,
        capsule_resource_allocated: bool,
    ) -> Result<(), HiLodTailSessionError<N::Error>> {
        use HiLodTailSessionError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let physical = actor.registered_physical_allocation().map_err(E::Actor)?;
        if !Arc::ptr_eq(physical.owner(), &owner) {
            return Err(E::ForeignActor);
        }
        if !owner
            .actor_query
            .lock()
            .map_err(|_| E::Poisoned)?
            .as_ref()
            .is_some_and(|query| query.belongs_to(actor))
        {
            return Err(E::ForeignActor);
        }
        // Identity checks are host preconditions; a NULL mandatory source
        // service is rejected at its actual dereference after collision below.
        if let Some(services) = owner.world_services() {
            if !self
                .physical_world_services()
                .is_some_and(|actual| Arc::ptr_eq(actual, services))
            {
                return Err(E::ForeignWorldServices);
            }
        }
        if let Some(query) = owner.network_query() {
            if !self
                .physical_world_services()
                .is_some_and(|actual| Arc::ptr_eq(actual, query))
            {
                return Err(E::ForeignWorldServices);
            }
        }
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if !(state.sleeping_attempted
                && state.air_attempted
                && state.wipeout_attempted
                && state.ground_attempted
                && owner
                    .remaining_states()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .is_complete())
            {
                return Err(E::StateConstructorsIncomplete);
            }
        }
        owner
            .tail()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_initialize_pre_ground(capsule_resource_allocated)
            .map_err(E::Tail)?;
        self.try_start_pair_ground_state(
            board,
            skeleton,
            &mut actor,
            &owner.inputs().cvars,
            numeric,
        )
        .map_err(E::Ground)?;
        owner
            .wipeout()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_initialize_spawn_invincibility()
            .map_err(E::Wipeout)?;
        owner
            .tail()
            .lock()
            .map_err(|_| E::Poisoned)?
            .initialize_post_ground_jump_velocity();
        let network = owner
            .network_query()
            .map(|query| query.try_is_network_game())
            .transpose()
            .map_err(E::Services)?;
        let change = owner
            .tail()
            .lock()
            .map_err(|_| E::Poisoned)?
            .select_network_collision(network);
        if let Some(change) = change {
            let world_only = change == HiLodCollisionChange::WorldOnly;
            self.try_set_pair_board_collision_mode(board, skeleton, world_only)
                .map_err(E::Board)?;
            self.try_pair_skeleton_collision_transition(
                board,
                skeleton,
                if world_only {
                    SkeletonCollisionTransition::StartWorldOnly
                } else {
                    SkeletonCollisionTransition::StopWorldOnly
                },
            )
            .map_err(E::Skeleton)?;
        }
        owner
            .world_services()
            .ok_or(E::MissingWorldServices)?
            .try_add(&physical)
            .map_err(E::Services)?;
        owner.tail().lock().map_err(|_| E::Poisoned)?.world_added = true;
        owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .jphy_state = Some(0);
        owner.tail().lock().map_err(|_| E::Poisoned)?.returned = true;
        Ok(())
    }
}
