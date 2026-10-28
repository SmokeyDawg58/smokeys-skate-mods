//! Selected Skeleton.ProcessAnimAttributes82b5ada0..adc4 on SAME flags+8c4.
use super::SkeletonProcessedContact;
use crate::{
    actor_construction::{
        ActorConstruction, anim_out_phys_in::PhysicsAnimationAttribute,
        anim_out_phys_in::publication::AnimationPhysicsPublicationError,
        physical_mount::ActorPhysicalMountError,
    },
    play_animation::FastString30,
};
use std::sync::Arc;

const WIPEOUT: u32 = 0x0004_0000;

#[cfg(test)]
#[path = "skeleton_processed_animation_wipeout_tests.rs"]
mod tests;

impl SkeletonProcessedContact {
    /// Raw type-zero name dispatch. Status, value and sequence do not gate
    /// `WipeoutAtt`; initializer82f82288 assigns the exact literal "Wipeout".
    pub(crate) fn apply_animation_wipeout_attribute(
        &mut self,
        attribute: &PhysicsAnimationAttribute,
    ) -> bool {
        if attribute.kind != 0
            || attribute.name != FastString30::encode("Wipeout").expect("source literal")
        {
            return false;
        }
        self.jump_events.flags |= WIPEOUT;
        self.jump_events.known |= WIPEOUT;
        true
    }
}

#[derive(Debug)]
pub(crate) enum AnimationWipeoutError {
    Input(AnimationPhysicsPublicationError),
    Physical(ActorPhysicalMountError),
    MissingAttribute(usize),
    Poisoned,
}
impl std::fmt::Display for AnimationWipeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(e) => write!(f, "animation wipeout input: {e}"),
            Self::Physical(e) => write!(f, "animation wipeout physical owner: {e}"),
            Self::MissingAttribute(index) => {
                write!(f, "animation wipeout attribute {index} unavailable")
            }
            Self::Poisoned => f.write_str("animation wipeout owner lock poisoned"),
        }
    }
}
impl ActorConstruction {
    /// Selected per-record continuation, reading this Actor's actual published
    /// `AnimOutPhysIn` and mutating its cached physical owner's SAME Processed.
    /// Caller must schedule preceding Update1/ProcessData effects. A matching
    /// record is a live attribute producer, not a full input-produced bail.
    pub(crate) fn try_process_selected_animation_wipeout_attribute(
        &self,
        index: usize,
    ) -> Result<bool, AnimationWipeoutError> {
        use AnimationWipeoutError as E;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let input = self.published_animation_physics_input().map_err(E::Input)?;
        let input = input.lock().map_err(|_| E::Poisoned)?;
        let attribute = input.attribute(index).ok_or(E::MissingAttribute(index))?;
        let processed = Arc::clone(
            physical
                .owner()
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        Ok(processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .apply_animation_wipeout_attribute(attribute))
    }
}
