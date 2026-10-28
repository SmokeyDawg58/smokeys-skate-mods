//! Selected physical constructor completion on the existing retained factory.
use super::{
    PhysicalPlayerAssemblyConstruction, PhysicalPlayerAssemblyError, PhysicalPlayerAssemblyPhase,
    constructor_services::{ConstructorFinishError, PhysicalConstructorServices},
};
use crate::{
    actor_construction::{
        ActorConstruction, ActorConstructionError, physical_mount::ActorPhysicalMountError,
    },
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::{
        hilod_tail_session::HiLodTailSessionError,
        physical_skateboard_controller::PhysicalPlayerControllerSessionError,
        physical_skeleton_controller::PhysicalSkeletonControllerError,
        physical_state::PhysicalStateSessionError,
    },
};
use std::{convert::Infallible, sync::Arc};

pub(crate) struct SelectedPhysicalPair {
    pub(crate) board: SimulationSkaterAssemblyHandle,
    pub(crate) skeleton: SimulationSkaterAssemblyHandle,
}
#[derive(Debug)]
pub(crate) enum SelectedPhysicalConstructorError<N> {
    Actor(ActorConstructionError),
    Mount(ActorPhysicalMountError),
    ForeignActor,
    Assembly(Box<PhysicalPlayerAssemblyError>),
    Finish(Box<ConstructorFinishError>),
    MissingAssembly(&'static str),
    Phase(PhysicalPlayerAssemblyPhase),
    BoardController(Box<PhysicalPlayerControllerSessionError<InputCvarError>>),
    SkeletonController(PhysicalSkeletonControllerError),
    States(Box<PhysicalStateSessionError<Infallible, Infallible>>),
    Tail(Box<HiLodTailSessionError<N>>),
}
impl<N: std::fmt::Debug> std::fmt::Display for SelectedPhysicalConstructorError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(e) => write!(f, "selected physical Actor: {e}"),
            Self::Mount(e) => write!(f, "selected physical registration: {e}"),
            Self::ForeignActor => f.write_str("selected physical factory belongs to another Actor"),
            Self::Assembly(e) => write!(f, "selected physical assembly: {e}"),
            Self::Finish(e) => write!(f, "selected physical skeleton finish: {e}"),
            Self::MissingAssembly(name) => write!(f, "selected physical {name} absent"),
            Self::Phase(phase) => write!(f, "selected physical factory phase: {phase:?}"),
            Self::BoardController(e) => write!(f, "selected board controller: {e:?}"),
            Self::SkeletonController(e) => write!(f, "selected skeleton controller: {e:?}"),
            Self::States(e) => write!(f, "selected physics states: {e:?}"),
            Self::Tail(e) => write!(f, "selected physical constructor tail: {e:?}"),
        }
    }
}
impl PhysicalPlayerAssemblyConstruction<'_> {
    /// Complete the source-ordered board/skeleton, controller, state and `HiLOD`
    /// tail calls already used by the original-data driver. Caller retains this
    /// factory on failure; no rollback, cleanup or whole-constructor retry occurs.
    /// Assertion, hat-enable and capsule-allocation policies come from the host.
    pub(crate) fn try_complete_selected_constructor<N: ReckoningNumeric>(
        &mut self,
        actor: &ActorConstruction,
        references: &mut InputCvarReferences,
        numeric: &mut N,
        capsule_resource_allocated: bool,
        assertion: &mut impl FnMut(),
        hat_enabled: &mut impl FnMut() -> bool,
    ) -> Result<SelectedPhysicalPair, SelectedPhysicalConstructorError<N::Error>> {
        use SelectedPhysicalConstructorError as E;
        // Host ownership admission, before any constructor store.
        let registered = actor.registered_physical_allocation().map_err(E::Mount)?;
        if !Arc::ptr_eq(registered.owner(), self.owner()) {
            return Err(E::ForeignActor);
        }
        {
            let prefix = actor.prefix().map_err(E::Actor)?;
            let fields = prefix
                .fields()
                .ok_or(E::Actor(ActorConstructionError::ActorFieldsUninitialized))?;
            let mut services = PhysicalConstructorServices {
                fields,
                references,
                assertion,
            };
            self.try_construct_board(&mut services)
                .map_err(|e| E::Assembly(Box::new(e)))?;
            self.try_construct_skeleton(&mut services, hat_enabled)
                .map_err(|e| E::Assembly(Box::new(e)))?;
        }
        self.try_finish_skeleton_from_live_services()
            .map_err(|e| E::Finish(Box::new(e)))?;
        let board = self.board().cloned().ok_or(E::MissingAssembly("board"))?;
        let skeleton = self
            .skeleton()
            .cloned()
            .ok_or(E::MissingAssembly("skeleton"))?;
        let world = self.completed_world_mut().map_err(E::Phase)?;
        world
            .try_construct_pair_skateboard_controller(&board, &skeleton, references)
            .map_err(|e| E::BoardController(Box::new(e)))?;
        world
            .try_construct_pair_skeleton_controller(&board, &skeleton)
            .map_err(E::SkeletonController)?;
        world
            .try_construct_pair_physics_states(&board, &skeleton)
            .map_err(|e| E::States(Box::new(e)))?;
        world
            .try_finish_pair_hilod_constructor(
                &board,
                &skeleton,
                actor,
                numeric,
                capsule_resource_allocated,
            )
            .map_err(|e| E::Tail(Box::new(e)))?;
        actor
            .try_publish_physical_constructor_return()
            .map_err(E::Mount)?;
        Ok(SelectedPhysicalPair { board, skeleton })
    }
}
