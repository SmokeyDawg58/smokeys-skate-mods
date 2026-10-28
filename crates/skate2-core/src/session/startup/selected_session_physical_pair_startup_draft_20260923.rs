//! Selected physical-pair startup over the retained world and Actor.
//! This is a constructor handoff, not a new physics owner or tick scheduler.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaits selected session integration")
)]

use crate::{
    actor_construction::{
        ActorConstruction,
        physical_animation_inputs::{ActorPhysicalAnimationInputs, PhysicalAnimationInputsError},
        physical_mount::ActorPhysicalMountError,
        selected_startup::SelectedPlayerStartupOwners,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalConstructionBindError,
        PhysicalPlayerAssemblyConstructionState, PhysicalPlayerAssemblyInputs,
        PhysicalPlayerAssemblyOwner,
        selected_constructor::{SelectedPhysicalConstructorError, SelectedPhysicalPair},
    },
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;

/// The same factory state remains owned through later session boot and ticks.
/// Its owner is separately exposed for an independent Actor publication check.
pub(crate) struct SelectedSessionPhysicalPair {
    pub(crate) pair: SelectedPhysicalPair,
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) _retained: PhysicalPlayerAssemblyConstructionState,
}

#[derive(Debug)]
pub(crate) enum SelectedSessionPhysicalPairError<N> {
    ActorMount(ActorPhysicalMountError),
    Bind(PhysicalConstructionBindError),
    Constructor(SelectedPhysicalConstructorError<N>),
}

impl<N: std::fmt::Debug> std::fmt::Display for SelectedSessionPhysicalPairError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ActorMount(error) => write!(f, "selected physical allocation: {error}"),
            Self::Bind(error) => write!(f, "selected physical world rebind: {error}"),
            Self::Constructor(error) => write!(f, "selected physical constructor: {error}"),
        }
    }
}

/// Construction failure keeps the unique in-progress factory when it exists.
/// Allocation can fail before that state is returned, so its slot is optional.
pub(crate) struct SelectedSessionPhysicalPairFailure<N> {
    pub(crate) error: SelectedSessionPhysicalPairError<N>,
    pub(crate) retained: Option<PhysicalPlayerAssemblyConstructionState>,
}

impl<N: std::fmt::Debug> std::fmt::Debug for SelectedSessionPhysicalPairFailure<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectedSessionPhysicalPairFailure")
            .field("error", &self.error)
            .field("retained", &self.retained.is_some())
            .finish()
    }
}

impl<N: std::fmt::Debug> std::fmt::Display for SelectedSessionPhysicalPairFailure<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

/// `Actor825ff2b4..ff2c4`: use the already evaluated bind pose and the same
/// selected player's hat lookup. The caller performs its intervening service
/// binding before physical allocation, as in the mounted startup schedule.
pub(crate) fn try_selected_physical_inputs(
    players: &SelectedPlayerStartupOwners,
    actor: &ActorConstruction,
    animation: &ActorPhysicalAnimationInputs,
    spawn: [f32; 16],
) -> Result<PhysicalPlayerAssemblyInputs, PhysicalAnimationInputsError> {
    players.try_physical_inputs(actor, animation, spawn)
}

/// `Actor825ff2d8..ff300` / `PhysicalPlayerHiLOD82e03c50`: allocate exactly
/// one pair against the supplied world and Actor, then run the existing
/// source-ordered board, skeleton, controller, state and HiLOD tail methods.
/// The host supplies its assertion, hat-enable and capsule-allocation answers.
/// A failed attempt returns its unique in-progress state when available and
/// retains any partial Actor/world writes. It does not roll them back or make
/// construction retryable.
pub(crate) fn try_construct_selected_physical_pair<N: ReckoningNumeric>(
    world: &mut PhysicalAssemblyWorld,
    actor: &ActorConstruction,
    inputs: PhysicalPlayerAssemblyInputs,
    references: &mut InputCvarReferences,
    numeric: &mut N,
    capsule_resource_allocated: bool,
    assertion: &mut impl FnMut(),
    hat_enabled: &mut impl FnMut() -> bool,
) -> Result<SelectedSessionPhysicalPair, SelectedSessionPhysicalPairFailure<N::Error>> {
    use SelectedSessionPhysicalPairError as Error;
    let factory = actor
        .try_allocate_physical(world, inputs)
        .map_err(|error| SelectedSessionPhysicalPairFailure {
            error: Error::ActorMount(error),
            retained: None,
        })?;
    // Keep the existing retained-state handoff and its same-world authority
    // check. Rebinding does not allocate or reconstruct either physical body.
    let retained = factory.into_retained();
    let mut factory = match retained.try_bind(world) {
        Ok(factory) => factory,
        Err((error, retained)) => {
            return Err(SelectedSessionPhysicalPairFailure {
                error: Error::Bind(error),
                retained: Some(retained),
            });
        }
    };
    let pair = match factory.try_complete_selected_constructor(
        actor,
        references,
        numeric,
        capsule_resource_allocated,
        assertion,
        hat_enabled,
    ) {
        Ok(pair) => pair,
        Err(error) => {
            return Err(SelectedSessionPhysicalPairFailure {
                error: Error::Constructor(error),
                retained: Some(factory.into_retained()),
            });
        }
    };
    let owner = Arc::clone(factory.owner());
    Ok(SelectedSessionPhysicalPair {
        pair,
        owner,
        _retained: factory.into_retained(),
    })
}
