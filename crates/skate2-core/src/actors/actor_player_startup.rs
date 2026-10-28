//! Production composition of existing animation constructor stages on one Actor.
use super::{ActorConstruction, animation_mount::ActorAnimationMountError};
use crate::{
    actor_animation_construction::ActorAnimationConstructionError,
    actor_construction::{
        input_generation::runtime::ActorInputRuntimeServices,
        physical_animation_inputs::{ActorPhysicalAnimationInputs, PhysicalAnimationInputsError},
        player_animation::{ActorAnimationPlayerServices, PlayerAnimationError},
    },
    character_face_manager::CharacterFaceManager,
    physical_player_assembly::PhysicalAssemblyWorld,
    simulation_animation_startup::{AnimationStartupError, AnimationStartupSource},
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

#[cfg(test)]
#[path = "actor_player_startup_corpus.rs"]
pub(crate) mod corpus;

#[derive(Debug)]
pub(crate) enum ActorPlayerStartupError<S> {
    Prefix(ActorAnimationMountError<PlayerAnimationError>),
    Suffix(ActorAnimationConstructionError<PlayerAnimationError>),
    Face(ActorAnimationConstructionError<AnimationStartupError<S>>),
    Publish(ActorAnimationMountError<Infallible>),
    Inputs(PhysicalAnimationInputsError),
    Lock,
}
impl<S: std::fmt::Debug> std::fmt::Display for ActorPlayerStartupError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prefix(e) => write!(f, "player animation prefix: {e}"),
            Self::Suffix(e) => write!(f, "player animation suffix: {e:?}"),
            Self::Face(e) => write!(f, "player face initialization: {e:?}"),
            Self::Publish(e) => write!(f, "player animation publication: {e}"),
            Self::Inputs(e) => write!(f, "player physical animation inputs: {e}"),
            Self::Lock => f.write_str("player animation startup lock poisoned"),
        }
    }
}
impl ActorConstruction {
    /// The caller already owns this Actor's fresh input/listener/graph prefix.
    /// Existing global startup, face residency and job ownership are supplied;
    /// this does not construct another manager or deliver gameplay input.
    pub(crate) fn try_complete_player_animation<S: AnimationStartupSource>(
        &self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        players: &mut ActorAnimationPlayerServices<'_>,
        faces: &CharacterFaceManager,
        startup_source: &mut S,
        input_services: ActorInputRuntimeServices,
    ) -> Result<ActorPhysicalAnimationInputs, ActorPlayerStartupError<S::Error>> {
        use ActorPlayerStartupError as E;
        let vault = Arc::clone(&players.players.vault);
        let retained = self
            .try_construct_animation_prefix(world, &vault, players)
            .map_err(E::Prefix)?;
        {
            let mut animation = retained.lock().map_err(|_| E::Lock)?;
            animation
                .try_construct_vault_suffix(vault, players)
                .map_err(E::Suffix)?;
            animation
                .try_finish_loaded_face_with_source(faces, startup_source)
                .map_err(E::Face)?;
        }
        self.try_publish_animation_constructor()
            .map_err(E::Publish)?;
        self.try_initialize_physical_animation_inputs(players, input_services)
            .map_err(E::Inputs)
    }
}
