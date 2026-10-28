//! `PlayAnimation` collaborator backed by the same retained Actor allocations.
use super::{ActorConstruction, ActorIdentity, MotionUpdateError};
use crate::{
    motion_graph_entity_authority::BoundMotionGraphContext,
    play_animation::{FastString30, FastString36},
    play_animation_lifecycle::{
        AnimationDispatch, PhysicalOutputObservation, PlayAnimationInterfaceKind,
        PlayAnimationInterfaceResolution, PlayAnimationLifecycleBackend,
        RawAnimationTransitionParams, TreeAttributeSample,
    },
};
use std::sync::{Arc, Mutex};
type Animation = super::super::animation_mount::MountedActorAnimation;

pub(crate) struct PlayAnimationActor<'a> {
    pub(crate) actor: &'a ActorConstruction,
    pub(crate) context: &'a BoundMotionGraphContext<ActorIdentity>,
}
impl PlayAnimationActor<'_> {
    pub(crate) fn animation(&self) -> Result<Arc<Mutex<Animation>>, MotionUpdateError> {
        use MotionUpdateError as E;
        let actor = self.actor.prefix().map_err(E::Actor)?;
        let animation = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
            .ok_or(E::MissingOwner("published animation"))?;
        let retained = animation.lock().map_err(|_| E::Lock)?;
        if !retained
            .entity()
            .is_same_issuance(&self.actor.authority.entity_handle())
        {
            return Err(E::ForeignOwner("animation entity"));
        }
        Ok(Arc::clone(animation))
    }
}
impl PlayAnimationLifecycleBackend for PlayAnimationActor<'_> {
    type Error = MotionUpdateError;
    fn resolve_interface(
        &mut self,
        kind: PlayAnimationInterfaceKind,
    ) -> Result<PlayAnimationInterfaceResolution, Self::Error> {
        use MotionUpdateError as E;
        use PlayAnimationInterfaceKind as I;
        self.actor.check_motion_runtime_context(self.context)?;
        match kind {
            I::Physical => {
                self.actor
                    .cached_physical_allocation()
                    .map_err(E::Physical)?;
            }
            I::Animatable | I::SkaterAnim => {
                let owner = self.animation()?;
                let actor = self.actor.prefix().map_err(E::Actor)?;
                let animation = owner.lock().map_err(|_| E::Lock)?;
                let index = if kind == I::Animatable { 0 } else { 2 };
                if !actor.animation_interfaces[index]
                    .as_ref()
                    .is_some_and(|r| Arc::ptr_eq(r, animation.occurrence()))
                {
                    return Ok(PlayAnimationInterfaceResolution::IdentityMismatch);
                }
            }
            I::MotionGraph | I::SkaterMotionGraph => {}
        }
        Ok(PlayAnimationInterfaceResolution::Resolved)
    }
    fn motion_intent(&mut self, name: FastString36) -> Result<Option<u32>, Self::Error> {
        Ok(self
            .actor
            .motion_runtime_intent_value(self.context, &name)?
            .map(f32::to_bits))
    }
    fn filtered_motion_intent(&mut self, name: FastString36) -> Result<Option<u32>, Self::Error> {
        Ok(self
            .actor
            .motion_runtime_filtered_intent_value(self.context, &name)?
            .map(f32::to_bits))
    }
    fn tree_attribute(
        &mut self,
        name: FastString30,
    ) -> Result<Option<TreeAttributeSample>, Self::Error> {
        self.actor
            .motion_runtime_tree_attribute(self.context, name)?
            .map(|attribute| tree_sample(&attribute.value))
            .transpose()
    }
    fn add_settable_attribute(
        &mut self,
        name: FastString30,
        value_bits: u32,
        normalize: bool,
        sequence: i32,
    ) -> Result<(), Self::Error> {
        self.animation()?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .try_add_gesture_attribute(name, f32::from_bits(value_bits), normalize, sequence)
            .map_err(MotionUpdateError::AnimationWrite)
    }
    fn selected_pro_skater(&mut self) -> Result<FastString30, Self::Error> {
        Ok(self
            .animation()?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .player()
            .ok_or(MotionUpdateError::MissingOwner("SkaterAnim player"))?
            .pro_skater)
    }
    fn set_construction_attribute(
        &mut self,
        name: FastString30,
        value: FastString30,
    ) -> Result<(), Self::Error> {
        self.animation()?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .motion_set_construction_attribute(name, value)
    }
    fn next_transition_override(
        &mut self,
    ) -> Result<Option<RawAnimationTransitionParams>, Self::Error> {
        use MotionUpdateError as E;
        let actor = self.actor.prefix().map_err(E::Actor)?;
        let transition = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .and_then(|g| g.transition.as_ref())
            .ok_or(E::MissingOwner("Motion transition override"))?;
        if transition.flags.known_mask & 0xc000_0000 != 0xc000_0000 {
            return Err(E::MissingOwner("transition flag initialization"));
        }
        Ok(Some(RawAnimationTransitionParams {
            transition_type_raw: transition.transition_type,
            blend_length_bits: transition.blend_length.to_bits(),
            placement_raw: transition.placement,
            flags_raw: transition.flags.bits & 0xc000_0000,
        }))
    }
    fn clear_transition_override(&mut self) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        let mut actor = self.actor.prefix().map_err(E::Actor)?;
        actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.transition.as_mut())
            .ok_or(E::MissingOwner("Motion transition override"))?
            .clear_override();
        Ok(())
    }
    fn is_switch(&mut self) -> Result<bool, Self::Error> {
        use MotionUpdateError as E;
        let owner = self.animation()?;
        let animation = owner.lock().map_err(|_| E::Lock)?;
        let player = animation
            .player()
            .ok_or(E::MissingOwner("SkaterAnim player"))?;
        // GetIsSwitch82cb7580 reads mRelativeStance (+0xe60) and tests == 1.
        Ok(player.relative_stance == 1)
    }
    fn is_mirrored(&mut self) -> Result<bool, Self::Error> {
        use MotionUpdateError as E;
        let owner = self.animation()?;
        let animation = owner.lock().map_err(|_| E::Lock)?;
        let player = animation
            .player()
            .ok_or(E::MissingOwner("SkaterAnim player"))?;
        if player.flags_known_mask & 0x4000_0000 == 0 {
            return Err(E::MissingOwner("mirror flag initialization"));
        }
        Ok(player.flags_bits & 0x4000_0000 != 0)
    }
    fn physical_output(&mut self) -> Result<PhysicalOutputObservation, Self::Error> {
        use MotionUpdateError as E;
        let owner = self
            .actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        let output = owner.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(PhysicalOutputObservation::HoldingSkateboardRaw(u8::from(
            output
                .try_offboard_output()
                .map_err(E::PhysOut)?
                .is_holding_skateboard,
        )))
    }
    fn dispatch(&mut self, request: AnimationDispatch) -> Result<u8, Self::Error> {
        self.animation()?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .try_dispatch_owned_animation(request)
            .map_err(MotionUpdateError::AnimationBody)
    }
}

// Query copies native value bits; the existing lifecycle owns type validation
// and the FromLastAnim flag/default/rename behavior.
fn tree_sample(
    value: &skate2_formats::andale::AndaleAttributeValue,
) -> Result<TreeAttributeSample, MotionUpdateError> {
    use skate2_formats::andale::AndaleAttributeValue as V;
    let (type_raw, value_bits) = match value {
        V::Float(value) => (0, value.to_bits()),
        V::Vector(value) => (1, value[0].to_bits()),
        V::Bytes24(value) => (
            3,
            u32::from_be_bytes(value[..4].try_into().expect("24 byte payload")),
        ),
        V::Unsupported {
            attribute_type,
            bytes,
        } => {
            let word = bytes.get(..4).ok_or(MotionUpdateError::MissingOwner(
                "tree attribute payload word",
            ))?;
            (
                i8::from_ne_bytes([*attribute_type]),
                u32::from_be_bytes(word.try_into().expect("four bytes")),
            )
        }
    };
    Ok(TreeAttributeSample {
        type_raw,
        value_bits,
    })
}
#[cfg(test)]
mod cached_tree_sample_tests {
    use super::*;
    use skate2_formats::andale::AndaleAttributeValue as V;
    #[test]
    fn original_type_and_payload_bits_reach_existing_lifecycle_type_dispatch() {
        let value = f32::from_bits(0x8000_0000);
        for (input, kind, bits) in [
            (V::Float(value), 0, 0x8000_0000),
            (V::Vector([value, 1.0, 2.0, 3.0]), 1, 0x8000_0000),
            (V::Bytes24([0x12; 24]), 3, 0x1212_1212),
            (
                V::Unsupported {
                    attribute_type: 2,
                    bytes: vec![0x7f, 0xc0, 0, 1],
                },
                2,
                0x7fc0_0001,
            ),
            (
                V::Unsupported {
                    attribute_type: 0xff,
                    bytes: vec![1, 2, 3, 4],
                },
                -1,
                0x0102_0304,
            ),
        ] {
            assert_eq!(
                tree_sample(&input).unwrap(),
                TreeAttributeSample {
                    type_raw: kind,
                    value_bits: bits
                }
            );
        }
        assert!(matches!(
            tree_sample(&V::Unsupported {
                attribute_type: 2,
                bytes: vec![]
            }),
            Err(MotionUpdateError::MissingOwner(
                "tree attribute payload word"
            ))
        ));
    }
}
