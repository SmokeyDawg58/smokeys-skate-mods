//! Actor825ff0d4..110: retained allocation and publication of animation input.
use super::anim_out_phys_in::{AnimOutConstructionError, AnimOutPhysInOwner};
use super::{ActorConstruction, ActorConstructionError, ActorPrefixPhase};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum ActorAnimOutError {
    Actor(ActorConstructionError),
    WrongPhase(ActorPrefixPhase),
    MissingAnimation,
    MissingBase,
    Poisoned,
    Constructor(AnimOutConstructionError),
}
impl std::fmt::Display for ActorAnimOutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "animation physics Actor: {error}"),
            Self::WrongPhase(phase) => write!(f, "animation physics construction phase: {phase:?}"),
            Self::Constructor(error) => write!(f, "animation physics constructor: {error:?}"),
            other => write!(f, "animation physics owner: {other:?}"),
        }
    }
}
impl ActorConstruction {
    pub(crate) fn try_construct_animation_physics_input(&self) -> Result<(), ActorAnimOutError> {
        use ActorAnimOutError as E;
        let mut prefix = self.prefix().map_err(E::Actor)?;
        if prefix.phase != ActorPrefixPhase::AnimationInitialized {
            return Err(E::WrongPhase(prefix.phase));
        }
        prefix.phase = ActorPrefixPhase::AnimationPhysicsInputConstructing;
        let animation = prefix
            .animation
            .as_ref()
            .map(Arc::clone)
            .ok_or(E::MissingAnimation)?;
        // Retain the actual allocated owner before the source reads mNbBones
        // for its constructor argument; Actor+204 is not published yet.
        let owner = Arc::new(Mutex::new(AnimOutPhysInOwner::new_sim_allocation()));
        prefix.animation_physics_input = Some(Arc::clone(&owner));
        let count = animation
            .lock()
            .map_err(|_| E::Poisoned)?
            .base()
            .ok_or(E::MissingBase)?
            .bone_count;
        owner
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_construct(u32::from_ne_bytes(count.to_ne_bytes()))
            .map_err(E::Constructor)?;
        prefix.animation_physics_input_published = true;
        prefix.phase = ActorPrefixPhase::AnimationPhysicsInputConstructed;
        Ok(())
    }
}
