//! Last selected Update5 callbacks, physical `AnimOut` Reset, and count increment.
use super::PhysicalPlayerAssemblyOwner;
use crate::{
    actor_graph_construction::ActorGraphIntents,
    physical_player_phys_out::construct::update5_finish::{
        Update5FinishError, Update5FinishSource,
    },
    play_animation::{FastString30, FastString36},
};
use std::sync::{Arc, OnceLock};
#[derive(Debug)]
pub(crate) enum FinishRuntimeError {
    Poisoned,
    Missing(&'static str),
    UnknownAnimationFlag(u32),
}
impl std::fmt::Display for FinishRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Update5 finish owner poisoned"),
            Self::Missing(field) => write!(f, "Update5 finish requires {field}"),
            Self::UnknownAnimationFlag(mask) => {
                write!(f, "Update5 physical animation flag unknown: {mask:08x}")
            }
        }
    }
}

struct Source<'a>(&'a Arc<PhysicalPlayerAssemblyOwner>);
impl Update5FinishSource for Source<'_> {
    type Error = FinishRuntimeError;
    fn pushing_motion_intent_present(&mut self) -> Result<bool, Self::Error> {
        static PUSHING: OnceLock<FastString36> = OnceLock::new();
        let name = PUSHING.get_or_init(|| ActorGraphIntents::native_name(b"PUSHING"));
        let animation = Arc::clone(
            self.0
                .animation_physics_input
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .as_ref()
                .ok_or(Self::Error::Missing("AnimOutPhysIn2b4"))?,
        );
        let input = animation.lock().map_err(|_| Self::Error::Poisoned)?;
        Ok(input
            .intents
            .as_ref()
            .ok_or(Self::Error::Missing("actual motionGraphIntents"))?
            .get(name)
            .is_some())
    }
    fn animation_flag(&mut self, mask: u32) -> Result<bool, Self::Error> {
        let animation = self.0.anim_out.lock().map_err(|_| Self::Error::Poisoned)?;
        let animation = animation
            .as_ref()
            .ok_or(Self::Error::Missing("physical AnimOut"))?;
        if animation.flags_known_mask & mask == mask {
            Ok(animation.flags_bits & mask != 0)
        } else {
            Err(Self::Error::UnknownAnimationFlag(mask))
        }
    }
    fn animation_grind_name(&mut self) -> Result<FastString30, Self::Error> {
        Ok(self
            .0
            .anim_out
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .as_ref()
            .ok_or(Self::Error::Missing("physical AnimOut"))?
            .grind_name)
    }
}
/// Caller must first complete the actual surface callback61ec. This selected
/// route remains distinct from full Update4/UpdateAll and native grind support.
pub(crate) fn try_finish_selected_update5(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<(), Update5FinishError<FinishRuntimeError>> {
    {
        let mut output = owner
            .phys_out()
            .lock()
            .map_err(|_| Update5FinishError::Source(FinishRuntimeError::Poisoned))?;
        output.try_publish_update5_finish(&mut Source(owner))?;
    }
    owner
        .anim_out
        .lock()
        .map_err(|_| Update5FinishError::Source(FinishRuntimeError::Poisoned))?
        .as_mut()
        .ok_or(Update5FinishError::Source(FinishRuntimeError::Missing(
            "physical AnimOut",
        )))?
        .reset();
    let mut prefix = owner
        .prefix()
        .lock()
        .map_err(|_| Update5FinishError::Source(FinishRuntimeError::Poisoned))?;
    let prefix = prefix
        .as_mut()
        .ok_or(Update5FinishError::Source(FinishRuntimeError::Missing(
            "HiLOD prefix",
        )))?;
    prefix.update_count = prefix.update_count.wrapping_add(1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actor_construction::anim_out_phys_in::AnimOutPhysInOwner,
        physical_player_anim_out::PhysicalAnimOut,
        physical_player_assembly::{
            hilod_prefix::PhysicalHiLodPrefix, tests::issued_owner_for_grind_tests,
        },
        physical_player_phys_out::construct::PhysOutAllocation,
    };
    use std::sync::Mutex;
    fn owner() -> Arc<PhysicalPlayerAssemblyOwner> {
        let (_world, owner) = issued_owner_for_grind_tests();
        let mut input = AnimOutPhysInOwner::new_sim_allocation();
        input.try_construct(0).unwrap();
        *owner.animation_physics_input.lock().unwrap() = Some(Arc::new(Mutex::new(input)));
        *owner.anim_out.lock().unwrap() = Some(PhysicalAnimOut::new());
        *owner.prefix().lock().unwrap() = Some(PhysicalHiLodPrefix::construct_in_sim_allocation());
        owner
            .phys_out()
            .lock()
            .unwrap()
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        owner
    }
    #[test]
    fn actual_motion_map_presence_accepts_zero_and_nan_weights() {
        let owner = owner();
        assert!(!Source(&owner).pushing_motion_intent_present().unwrap());
        let input = Arc::clone(
            owner
                .animation_physics_input
                .lock()
                .unwrap()
                .as_ref()
                .unwrap(),
        );
        for value in [0.0, f32::NAN] {
            input
                .lock()
                .unwrap()
                .intents
                .as_mut()
                .unwrap()
                .insert_or_assign(ActorGraphIntents::native_name(b"PUSHING"), value)
                .unwrap();
            assert!(Source(&owner).pushing_motion_intent_present().unwrap());
        }
    }
    #[test]
    fn late_unknown_flag_preserves_animation_and_count_until_all_outputs_return() {
        let owner = owner();
        {
            let mut animation = owner.anim_out.lock().unwrap();
            let animation = animation.as_mut().unwrap();
            animation.flags_bits = u32::MAX;
            animation.flags_known_mask = 0xf000_0000;
        }
        owner
            .prefix()
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .update_count = i32::MAX;
        assert!(matches!(
            try_finish_selected_update5(&owner),
            Err(Update5FinishError::Source(
                FinishRuntimeError::UnknownAnimationFlag(0x0800_0000)
            ))
        ));
        assert_eq!(
            owner.anim_out.lock().unwrap().as_ref().unwrap().flags_bits,
            u32::MAX
        );
        assert_eq!(
            owner
                .prefix()
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .update_count,
            i32::MAX
        );
        owner
            .anim_out
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .flags_known_mask = u32::MAX;
        try_finish_selected_update5(&owner).unwrap();
        let animation = owner.anim_out.lock().unwrap();
        let animation = animation.as_ref().unwrap();
        assert_eq!(animation.flags_bits, 0x07ff_ffff);
        assert_eq!(animation.flags_known_mask, u32::MAX);
        assert_eq!(
            owner
                .prefix()
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .update_count,
            i32::MIN
        );
    }
}
