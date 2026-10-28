//! Owned animation writes used by `CharacterGesture::Update82cc3198`.
use super::{ActorAnimationConstruction, AnimationSettableAttribute};
use crate::play_animation::{FastString30, FastString36};
use skate2_formats::andale::{AndaleAttributeValue, AndaleRuntimeAttribute};

#[derive(Debug)]
pub(crate) enum GestureAnimationWriteError {
    MissingInitialization,
    MissingBase,
    UnknownChannelField(&'static str),
    InvalidChannelIndex,
    InvalidAttributeName,
    AttributeNan,
    AttributeCapacity,
    InvalidAttributeStorage,
}
impl std::fmt::Display for GestureAnimationWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownChannelField(name) => {
                write!(f, "gesture channel field is unwritten: {name}")
            }
            other => write!(f, "gesture animation write: {other:?}"),
        }
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// `ChannelAnimatable::EndChannelPrematurely82d7d5d0`.
    /// Missing names succeed; existing channels change only when their actual
    /// blend-out time equals zero (including negative zero).
    pub(crate) fn try_end_gesture_channel(
        &mut self,
        name: &FastString36,
    ) -> Result<(), GestureAnimationWriteError> {
        use GestureAnimationWriteError as E;
        let channels = self
            .initialization
            .as_mut()
            .and_then(|initialization| initialization.channels.as_mut())
            .ok_or(E::MissingInitialization)?;
        let Some(&index) = channels.dictionary.get(name) else {
            return Ok(());
        };
        let channel = channels
            .channels
            .get_mut(index)
            .ok_or(E::InvalidChannelIndex)?;
        if channel
            .blend_out_time
            .ok_or(E::UnknownChannelField("blend out time"))?
            == 0.0
        {
            let flags = channel
                .channel_flags
                .as_mut()
                .ok_or(E::UnknownChannelField("flags"))?;
            *flags |= 0x2800_0000;
            if let Some(runtime) = &mut channel.runtime {
                runtime.blending_out = true;
                runtime.blend_out_early = true;
            }
        }
        Ok(())
    }

    /// `Animatable::AddSettableAtt82d7f5f0`: preserve first name+sequence match,
    /// original append order, and the source's exact non-NaN assertion.
    pub(crate) fn try_add_gesture_attribute(
        &mut self,
        name: FastString30,
        value: f32,
        normalized: bool,
        sequence_id: i32,
    ) -> Result<(), GestureAnimationWriteError> {
        use GestureAnimationWriteError as E;
        // Raw 1bbef065 compares the splatted value with itself (VA62/VB62).
        // Infinity is accepted; only unordered self-comparison asserts.
        if value.is_nan() {
            return Err(E::AttributeNan);
        }
        if std::env::var_os("SKATE2_TRACE_VERT").is_some()
            && let Ok(text) = name.decode()
            && matches!(
                text.as_str(),
                "disttocog"
                    | "tilt_x"
                    | "tilt_z"
                    | "speedtuck"
                    | "angle"
                    | "holding"
                    | "quickness"
                    | "dir"
            )
        {
            eprintln!("vert anim attribute: {text}={value} normalized={normalized}");
        }
        let attributes = &mut self
            .base
            .as_mut()
            .ok_or(E::MissingBase)?
            .settable_attributes;
        for slot in &mut attributes.slots[..attributes.len] {
            let attribute = slot.as_mut().ok_or(E::InvalidAttributeStorage)?;
            if attribute.name == name && attribute.sequence_id == sequence_id {
                attribute.attribute.value = AndaleAttributeValue::Float(value);
                attribute.sequence_id = sequence_id;
                attribute.normalized = normalized;
                return Ok(());
            }
        }
        if attributes.len == attributes.slots.len() {
            return Err(E::AttributeCapacity);
        }
        let attribute = AnimationSettableAttribute {
            name,
            sequence_id,
            normalized,
            attribute: AndaleRuntimeAttribute {
                sequence_id: -1,
                begin_seconds: -1.0,
                end_seconds: -1.0,
                name: name.decode().map_err(|_| E::InvalidAttributeName)?,
                status: 6,
                value: AndaleAttributeValue::Float(value),
            },
        };
        attributes.slots[attributes.len] = Some(attribute);
        attributes.len += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actor_animation_construction::{
            ActorAnimatableBase, ActorAnimationInputs, AnimationLifetime,
        },
        motion_graph_entity_authority::ActorGraphAuthority,
    };
    use std::sync::Arc;

    fn base_owner() -> ActorAnimationConstruction<u32, ()> {
        let authority = ActorGraphAuthority::new(7);
        let mut owner = ActorAnimationConstruction::new(
            &authority,
            ActorAnimationInputs {
                physics_world: Arc::new(()),
                life: AnimationLifetime::Transient,
                use_low_blend_cull_threshold: false,
                face_name: None,
            },
        );
        owner.base = Some(ActorAnimatableBase::new(AnimationLifetime::Transient));
        owner
    }

    #[test]
    fn settable_identity_includes_sequence_and_updates_original_slot() {
        let mut owner = base_owner();
        let name = FastString30::encode("GstrDistToCog").unwrap();
        owner
            .try_add_gesture_attribute(name, 2.0, false, -1)
            .unwrap();
        owner.try_add_gesture_attribute(name, 3.0, true, 9).unwrap();
        owner
            .try_add_gesture_attribute(name, -0.0, true, -1)
            .unwrap();
        let attributes = &owner.base.as_ref().unwrap().settable_attributes;
        assert_eq!(attributes.len, 2);
        let first = attributes.slots[0].as_ref().unwrap();
        assert_eq!(
            first.attribute.scalar_value().unwrap().to_bits(),
            (-0.0_f32).to_bits()
        );
        assert!(first.normalized);
        assert_eq!(first.sequence_id, -1);
        assert_eq!(first.attribute.status, 6);
        assert_eq!(
            first.attribute.begin_seconds.to_bits(),
            (-1.0_f32).to_bits()
        );
        assert_eq!(attributes.slots[1].as_ref().unwrap().sequence_id, 9);
    }

    #[test]
    fn attribute_nan_asserts_before_mutation_but_infinity_is_valid() {
        let mut owner = base_owner();
        let name = FastString30::encode("GstrDistToCog").unwrap();
        assert!(matches!(
            owner.try_add_gesture_attribute(name, f32::NAN, false, -1),
            Err(GestureAnimationWriteError::AttributeNan)
        ));
        assert_eq!(owner.base.as_ref().unwrap().settable_attributes.len, 0);
        owner
            .try_add_gesture_attribute(name, f32::INFINITY, false, -1)
            .unwrap();
        assert_eq!(
            owner.base.as_ref().unwrap().settable_attributes.slots[0]
                .as_ref()
                .unwrap()
                .attribute
                .scalar_value()
                .unwrap()
                .to_bits(),
            f32::INFINITY.to_bits()
        );
    }

    #[test]
    fn construction_removal_preserves_neighbors_and_absent_key_is_noop() {
        let mut owner = base_owner();
        let key = FastString30::encode("Trick").unwrap();
        for name in ["Actor", "Trick", "Style"] {
            owner
                .motion_set_construction_attribute(
                    FastString30::encode(name).unwrap(),
                    FastString30::encode("Authored").unwrap(),
                )
                .unwrap();
        }
        let before = owner
            .base
            .as_ref()
            .unwrap()
            .construction_map
            .occupied_slots()
            .to_vec();
        owner
            .motion_remove_construction_attribute(FastString30::encode("Missing").unwrap())
            .unwrap();
        assert_eq!(
            owner
                .base
                .as_ref()
                .unwrap()
                .construction_map
                .occupied_slots(),
            before
        );
        owner.motion_remove_construction_attribute(key).unwrap();
        let after = owner
            .base
            .as_ref()
            .unwrap()
            .construction_map
            .occupied_slots()
            .to_vec();
        assert_eq!(
            after,
            before
                .into_iter()
                .filter(|entry| entry.as_ref().unwrap().0 != key)
                .collect::<Vec<_>>()
        );
        owner.motion_remove_construction_attribute(key).unwrap();
        assert_eq!(
            owner
                .base
                .as_ref()
                .unwrap()
                .construction_map
                .occupied_slots(),
            after
        );
        owner.base = None;
        assert!(matches!(
            owner.motion_remove_construction_attribute(key),
            Err(
                crate::generated_motion_load::runtime::MotionUpdateError::MissingOwner(
                    "Animatable construction map"
                )
            )
        ));
    }
}
