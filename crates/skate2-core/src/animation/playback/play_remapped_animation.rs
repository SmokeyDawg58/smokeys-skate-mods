//! Stored data and direct lifecycle for living-world `PlayRemappedAnimation`.
//!
//! The leaf delegates actual animation work to `LWAnimHelper`; this module
//! reconstructs only the exact constructor defaults/order and the direct
//! mirror-write, Setup, Update, and no-op End boundary.

use skate2_formats::state_graph::{StateGraphBinaryAttribute, state_graph_string_hash32};
use std::{error::Error, fmt};

pub const PLAY_REMAPPED_ANIMATION_OBJECT_SIZE: usize = 0xa8;
pub const PLAY_REMAPPED_ANIMATION_TRANSITION_OFFSET: usize = 0x1c;
pub const PLAY_REMAPPED_ANIMATION_NAME_OFFSET: usize = 0x20;
pub const PLAY_REMAPPED_ANIMATION_FRAME_TAG_OFFSET: usize = 0x45;
pub const PLAY_REMAPPED_ANIMATION_FRAME_OFFSET: usize = 0x6c;
pub const PLAY_REMAPPED_ANIMATION_CYCLE_TAG_OFFSET: usize = 0x70;
pub const PLAY_REMAPPED_ANIMATION_CYCLE_PERCENTAGE_OFFSET: usize = 0x98;
pub const PLAY_REMAPPED_ANIMATION_PLAYBACK_SPEED_OFFSET: usize = 0x9c;
pub const PLAY_REMAPPED_ANIMATION_BLEND_LENGTH_OFFSET: usize = 0xa0;
pub const PLAY_REMAPPED_ANIMATION_HAS_MIRROR_OFFSET: usize = 0xa4;
pub const PLAY_REMAPPED_ANIMATION_MIRROR_OFFSET: usize = 0xa5;
pub const PLAY_REMAPPED_ANIMATION_C_STRING_MAX_BYTES: usize = 36;
pub const PLAY_REMAPPED_ANIMATION_TRANSITION_MAX_BYTES: usize = 30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum RemappedAnimationTransitionType {
    Play = 0,
    Sequence = 1,
    Blend = 2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayRemappedAnimationBehaviour {
    pub transition_type: RemappedAnimationTransitionType,
    animation_name: Vec<u8>,
    frame_offset_tag: Vec<u8>,
    pub frame_offset_bits: u32,
    cycle_percentage_tag: Vec<u8>,
    pub cycle_percentage_bits: u32,
    pub playback_speed_bits: u32,
    pub blend_length_bits: u32,
    /// `None` is absent; `Some(0)` is observably present and distinct.
    pub mirror_override_raw: Option<u8>,
}

impl PlayRemappedAnimationBehaviour {
    /// Reconstructs `PlayRemappedAnimation::PlayRemappedAnimation` at
    /// `0x82717C60` on the checked host C-string/FastString domain.
    /// Attribute lookup is native hash-only and first-match.
    ///
    /// # Errors
    ///
    /// Returns checked forms of native assertion/overflow boundaries for an
    /// unknown transition, missing animation, embedded NUL, noncanonical
    /// `FastString` byte, or overlong fixed string. `FastString` rejection is a
    /// checked host policy: native packing permits aliases and truncation.
    /// Raw floating-point bits and the mirror byte are never normalized.
    pub fn from_attributes(
        attributes: &[StateGraphBinaryAttribute],
    ) -> Result<Self, PlayRemappedAnimationParseError> {
        let transition_type = if let Some(value) = attribute(attributes, b"transType") {
            checked_transition(&value.text)?
        } else {
            RemappedAnimationTransitionType::Blend
        };

        let animation = attribute(attributes, b"anim")
            .ok_or(PlayRemappedAnimationParseError::MissingAnimation)?;
        let animation_name = checked_c_array(b"anim", &animation.text)?;
        let playback_speed_bits = attribute(attributes, b"playBackSpeed")
            .map_or(1.0f32.to_bits(), |value| value.float_bits);
        let frame_offset_bits =
            attribute(attributes, b"frameOffset").map_or(0, |value| value.float_bits);
        let frame_offset_tag = match attribute(attributes, b"frameOffsetTag") {
            Some(value) => checked_c_array(b"frameOffsetTag", &value.text)?,
            None => Vec::new(),
        };
        let cycle_percentage_bits =
            attribute(attributes, b"cycleSwappingPercentage").map_or(0, |value| value.float_bits);
        let cycle_percentage_tag = match attribute(attributes, b"cycleSwappingPercentageTag") {
            Some(value) => checked_c_array(b"cycleSwappingPercentageTag", &value.text)?,
            None => Vec::new(),
        };
        let blend_length_bits = if transition_type == RemappedAnimationTransitionType::Blend {
            attribute(attributes, b"blendTime").map_or(0.1f32.to_bits(), |value| value.float_bits)
        } else {
            0
        };
        let mirror_override_raw =
            attribute(attributes, b"mirrorOverride").map(|value| value.boolean_raw);

        Ok(Self {
            transition_type,
            animation_name,
            frame_offset_tag,
            frame_offset_bits,
            cycle_percentage_tag,
            cycle_percentage_bits,
            playback_speed_bits,
            blend_length_bits,
            mirror_override_raw,
        })
    }

    #[must_use]
    pub fn animation_name(&self) -> &[u8] {
        &self.animation_name
    }

    #[must_use]
    pub fn frame_offset_tag(&self) -> &[u8] {
        &self.frame_offset_tag
    }

    #[must_use]
    pub fn cycle_percentage_tag(&self) -> &[u8] {
        &self.cycle_percentage_tag
    }

    /// Reconstructs `PlayRemappedAnimation::Begin` at `0x82718068`.
    /// The optional exact-byte mirror write precedes Setup and is not rolled
    /// back when a checked host collaborator reports a later Setup failure.
    ///
    /// # Errors
    ///
    /// Returns the first collaborator error with the already-completed effect
    /// prefix retained.
    pub fn begin<B: PlayRemappedAnimationCollaborator>(
        &self,
        backend: &mut B,
    ) -> Result<(), B::Error> {
        if let Some(value) = self.mirror_override_raw {
            backend.write_mirror_override_raw(value)?;
        }
        backend.setup_remapped_animation(RemappedAnimationSetup {
            animation_name: &self.animation_name,
            playback_speed_bits: self.playback_speed_bits,
            frame_offset_tag: &self.frame_offset_tag,
            frame_offset_bits: self.frame_offset_bits,
            cycle_percentage_tag: &self.cycle_percentage_tag,
            cycle_percentage_bits: self.cycle_percentage_bits,
            transition_type: self.transition_type,
            blend_length_bits: self.blend_length_bits,
        })
    }

    /// Reconstructs the complete wrapper at `0x826F33F0`.
    ///
    /// # Errors
    ///
    /// Returns the helper collaborator's exact error.
    pub fn update<B: PlayRemappedAnimationCollaborator>(
        &self,
        backend: &mut B,
    ) -> Result<(), B::Error> {
        backend.update_remapped_animation(RemappedAnimationUpdate {
            animation_name: &self.animation_name,
            playback_speed_bits: self.playback_speed_bits,
        })
    }

    /// The inherited End at `0x825187C0` is exactly `blr`.
    pub const fn end(&self) {}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlayRemappedAnimationParseError {
    MissingAnimation,
    TransitionContainsNul,
    TransitionTooLong {
        length: usize,
        maximum: usize,
    },
    TransitionInvalidCharacter {
        index: usize,
        byte: u8,
    },
    UnknownTransition(Vec<u8>),
    StringContainsNul {
        key: &'static [u8],
    },
    StringTooLong {
        key: &'static [u8],
        length: usize,
        maximum: usize,
    },
}

impl fmt::Display for PlayRemappedAnimationParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAnimation => formatter.write_str(
                "PlayRemappedAnimation is missing required anim and reaches a native assertion",
            ),
            Self::TransitionContainsNul => {
                formatter.write_str("PlayRemappedAnimation transType contains an embedded NUL")
            }
            Self::TransitionTooLong { length, maximum } => write!(
                formatter,
                "PlayRemappedAnimation transType has {length} bytes; checked maximum is {maximum}"
            ),
            Self::TransitionInvalidCharacter { index, byte } => write!(
                formatter,
                "PlayRemappedAnimation transType byte {index} is 0x{byte:02x}; checked FastString domain is [A-Za-z0-9_]"
            ),
            Self::UnknownTransition(value) => write!(
                formatter,
                "PlayRemappedAnimation transType {} reaches a native assertion",
                String::from_utf8_lossy(value)
            ),
            Self::StringContainsNul { key } => write!(
                formatter,
                "PlayRemappedAnimation {} contains an embedded NUL",
                String::from_utf8_lossy(key)
            ),
            Self::StringTooLong {
                key,
                length,
                maximum,
            } => write!(
                formatter,
                "PlayRemappedAnimation {} has {length} bytes; checked maximum is {maximum} plus NUL",
                String::from_utf8_lossy(key)
            ),
        }
    }
}

impl Error for PlayRemappedAnimationParseError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemappedAnimationSetup<'a> {
    pub animation_name: &'a [u8],
    pub playback_speed_bits: u32,
    pub frame_offset_tag: &'a [u8],
    pub frame_offset_bits: u32,
    pub cycle_percentage_tag: &'a [u8],
    pub cycle_percentage_bits: u32,
    pub transition_type: RemappedAnimationTransitionType,
    pub blend_length_bits: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemappedAnimationUpdate<'a> {
    pub animation_name: &'a [u8],
    pub playback_speed_bits: u32,
}

#[allow(clippy::missing_errors_doc)]
pub trait PlayRemappedAnimationCollaborator {
    type Error;

    fn write_mirror_override_raw(&mut self, value: u8) -> Result<(), Self::Error>;
    fn setup_remapped_animation(
        &mut self,
        request: RemappedAnimationSetup<'_>,
    ) -> Result<(), Self::Error>;
    fn update_remapped_animation(
        &mut self,
        request: RemappedAnimationUpdate<'_>,
    ) -> Result<(), Self::Error>;
}

fn checked_transition(
    value: &[u8],
) -> Result<RemappedAnimationTransitionType, PlayRemappedAnimationParseError> {
    if value.contains(&0) {
        return Err(PlayRemappedAnimationParseError::TransitionContainsNul);
    }
    if value.len() > PLAY_REMAPPED_ANIMATION_TRANSITION_MAX_BYTES {
        return Err(PlayRemappedAnimationParseError::TransitionTooLong {
            length: value.len(),
            maximum: PLAY_REMAPPED_ANIMATION_TRANSITION_MAX_BYTES,
        });
    }
    if let Some((index, &byte)) = value
        .iter()
        .enumerate()
        .find(|(_, byte)| !byte.is_ascii_alphanumeric() && **byte != b'_')
    {
        return Err(PlayRemappedAnimationParseError::TransitionInvalidCharacter { index, byte });
    }
    if value.eq_ignore_ascii_case(b"play") {
        Ok(RemappedAnimationTransitionType::Play)
    } else if value.eq_ignore_ascii_case(b"sequence") {
        Ok(RemappedAnimationTransitionType::Sequence)
    } else if value.eq_ignore_ascii_case(b"blend") {
        Ok(RemappedAnimationTransitionType::Blend)
    } else {
        Err(PlayRemappedAnimationParseError::UnknownTransition(
            value.to_vec(),
        ))
    }
}

fn checked_c_array(
    key: &'static [u8],
    value: &[u8],
) -> Result<Vec<u8>, PlayRemappedAnimationParseError> {
    if value.contains(&0) {
        return Err(PlayRemappedAnimationParseError::StringContainsNul { key });
    }
    if value.len() > PLAY_REMAPPED_ANIMATION_C_STRING_MAX_BYTES {
        return Err(PlayRemappedAnimationParseError::StringTooLong {
            key,
            length: value.len(),
            maximum: PLAY_REMAPPED_ANIMATION_C_STRING_MAX_BYTES,
        });
    }
    Ok(value.to_vec())
}

fn attribute<'a>(
    attributes: &'a [StateGraphBinaryAttribute],
    key: &[u8],
) -> Option<&'a StateGraphBinaryAttribute> {
    let hash = state_graph_string_hash32(key);
    attributes.iter().find(|value| value.key_hash == hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(
        key: &[u8],
        text: &[u8],
        float_bits: u32,
        boolean_raw: u8,
    ) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits,
            boolean_raw,
        }
    }

    #[test]
    fn minimal_mounted_leaf_uses_exact_defaults() {
        let leaf = PlayRemappedAnimationBehaviour::from_attributes(&[attr(
            b"anim",
            b"FACE_HAND_TEST",
            0,
            0,
        )])
        .unwrap();
        assert_eq!(leaf.transition_type, RemappedAnimationTransitionType::Blend);
        assert_eq!(leaf.animation_name(), b"FACE_HAND_TEST");
        assert_eq!(leaf.playback_speed_bits, 1.0f32.to_bits());
        assert_eq!(leaf.frame_offset_bits, 0);
        assert_eq!(leaf.frame_offset_tag(), b"");
        assert_eq!(leaf.cycle_percentage_bits, 0);
        assert_eq!(leaf.cycle_percentage_tag(), b"");
        assert_eq!(leaf.blend_length_bits, 0.1f32.to_bits());
        assert_eq!(leaf.mirror_override_raw, None);
    }

    #[test]
    fn transition_matching_is_case_insensitive_and_blend_query_is_conditional() {
        for (text, expected) in [
            (b"PLAY".as_slice(), RemappedAnimationTransitionType::Play),
            (
                b"Sequence".as_slice(),
                RemappedAnimationTransitionType::Sequence,
            ),
            (b"bLeNd".as_slice(), RemappedAnimationTransitionType::Blend),
        ] {
            let leaf = PlayRemappedAnimationBehaviour::from_attributes(&[
                attr(b"transType", text, 0, 0),
                attr(b"anim", b"A", 0, 0),
                attr(b"blendTime", b"", 0x7fc1_2345, 0),
            ])
            .unwrap();
            assert_eq!(leaf.transition_type, expected);
            assert_eq!(
                leaf.blend_length_bits,
                if expected == RemappedAnimationTransitionType::Blend {
                    0x7fc1_2345
                } else {
                    0
                }
            );
        }
    }

    #[test]
    fn transition_failure_precedes_required_animation_failure() {
        assert_eq!(
            PlayRemappedAnimationBehaviour::from_attributes(&[attr(b"transType", b"other", 0, 0,)]),
            Err(PlayRemappedAnimationParseError::UnknownTransition(
                b"other".to_vec()
            ))
        );
    }

    #[test]
    fn transition_checked_domain_rejects_aliases_before_enum_matching() {
        for byte in [b'-', 0xff] {
            assert_eq!(
                PlayRemappedAnimationBehaviour::from_attributes(&[attr(
                    b"transType",
                    &[b'p', byte],
                    0,
                    0,
                )]),
                Err(PlayRemappedAnimationParseError::TransitionInvalidCharacter { index: 1, byte })
            );
        }
        for text in [
            b"play_1".as_slice(),
            b"",
            b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        ] {
            let expected = if text.len() > 30 {
                PlayRemappedAnimationParseError::TransitionTooLong {
                    length: text.len(),
                    maximum: 30,
                }
            } else {
                PlayRemappedAnimationParseError::UnknownTransition(text.to_vec())
            };
            assert_eq!(checked_transition(text), Err(expected));
        }
        assert_eq!(
            checked_transition(b"play\0"),
            Err(PlayRemappedAnimationParseError::TransitionContainsNul)
        );
        // All three raw C arrays retain their wider byte domain.
        let bytes = [b'-', 0xff];
        let leaf = PlayRemappedAnimationBehaviour::from_attributes(&[
            attr(b"anim", &bytes, 0, 0),
            attr(b"frameOffsetTag", &bytes, 0, 0),
            attr(b"cycleSwappingPercentageTag", &bytes, 0, 0),
        ])
        .unwrap();
        assert_eq!(leaf.animation_name(), bytes);
        assert_eq!(leaf.frame_offset_tag(), bytes);
        assert_eq!(leaf.cycle_percentage_tag(), bytes);
    }

    #[test]
    fn raw_float_bits_and_mirror_presence_are_preserved() {
        let attributes = [
            attr(b"anim", b"A", 0, 0),
            attr(b"playBackSpeed", b"", 0x8000_0000, 0),
            attr(b"frameOffset", b"", 0x7fc1_2345, 0),
            attr(b"cycleSwappingPercentage", b"", 0x7f80_0000, 0),
            attr(b"blendTime", b"", 0xff80_0000, 0),
            attr(b"mirrorOverride", b"", 0, 0x80),
        ];
        let leaf = PlayRemappedAnimationBehaviour::from_attributes(&attributes).unwrap();
        assert_eq!(leaf.playback_speed_bits, 0x8000_0000);
        assert_eq!(leaf.frame_offset_bits, 0x7fc1_2345);
        assert_eq!(leaf.cycle_percentage_bits, 0x7f80_0000);
        assert_eq!(leaf.blend_length_bits, 0xff80_0000);
        assert_eq!(leaf.mirror_override_raw, Some(0x80));

        let present_zero = PlayRemappedAnimationBehaviour::from_attributes(&[
            attr(b"anim", b"A", 0, 0),
            attr(b"mirrorOverride", b"", 0, 0),
        ])
        .unwrap();
        assert_eq!(present_zero.mirror_override_raw, Some(0));
    }

    #[test]
    fn checked_string_boundaries_are_field_specific() {
        let mut accepted = vec![b'x'; 36];
        accepted[0] = 0xff;
        assert_eq!(
            PlayRemappedAnimationBehaviour::from_attributes(&[attr(b"anim", &accepted, 0, 0)])
                .unwrap()
                .animation_name()
                .len(),
            36
        );
        for (key, text, expected) in [
            (
                b"anim".as_slice(),
                vec![b'x'; 37],
                PlayRemappedAnimationParseError::StringTooLong {
                    key: b"anim",
                    length: 37,
                    maximum: 36,
                },
            ),
            (
                b"anim".as_slice(),
                b"a\0b".to_vec(),
                PlayRemappedAnimationParseError::StringContainsNul { key: b"anim" },
            ),
        ] {
            assert_eq!(
                PlayRemappedAnimationBehaviour::from_attributes(&[attr(key, &text, 0, 0)]),
                Err(expected)
            );
        }
    }

    #[derive(Default)]
    struct Spy {
        calls: Vec<String>,
        fail: Option<&'static str>,
    }

    impl PlayRemappedAnimationCollaborator for Spy {
        type Error = &'static str;

        fn write_mirror_override_raw(&mut self, value: u8) -> Result<(), Self::Error> {
            self.calls.push(format!("mirror:{value}"));
            if self.fail == Some("mirror") {
                Err("mirror")
            } else {
                Ok(())
            }
        }

        fn setup_remapped_animation(
            &mut self,
            request: RemappedAnimationSetup<'_>,
        ) -> Result<(), Self::Error> {
            self.calls.push(format!(
                "setup:{}:{:08x}",
                String::from_utf8_lossy(request.animation_name),
                request.playback_speed_bits
            ));
            if self.fail == Some("setup") {
                Err("setup")
            } else {
                Ok(())
            }
        }

        fn update_remapped_animation(
            &mut self,
            request: RemappedAnimationUpdate<'_>,
        ) -> Result<(), Self::Error> {
            self.calls.push(format!(
                "update:{}:{:08x}",
                String::from_utf8_lossy(request.animation_name),
                request.playback_speed_bits
            ));
            if self.fail == Some("update") {
                Err("update")
            } else {
                Ok(())
            }
        }
    }

    fn mirrored() -> PlayRemappedAnimationBehaviour {
        PlayRemappedAnimationBehaviour::from_attributes(&[
            attr(b"anim", b"FACE", 0, 0),
            attr(b"mirrorOverride", b"", 0, 0x80),
        ])
        .unwrap()
    }

    #[test]
    fn begin_preserves_optional_mirror_then_setup_order_and_partial_effects() {
        let mut spy = Spy::default();
        mirrored().begin(&mut spy).unwrap();
        assert_eq!(spy.calls, ["mirror:128", "setup:FACE:3f800000"]);

        let mut no_mirror = Spy::default();
        PlayRemappedAnimationBehaviour::from_attributes(&[attr(b"anim", b"FACE", 0, 0)])
            .unwrap()
            .begin(&mut no_mirror)
            .unwrap();
        assert_eq!(no_mirror.calls, ["setup:FACE:3f800000"]);

        let mut mirror_failure = Spy {
            fail: Some("mirror"),
            ..Spy::default()
        };
        assert_eq!(mirrored().begin(&mut mirror_failure), Err("mirror"));
        assert_eq!(mirror_failure.calls, ["mirror:128"]);

        let mut setup_failure = Spy {
            fail: Some("setup"),
            ..Spy::default()
        };
        assert_eq!(mirrored().begin(&mut setup_failure), Err("setup"));
        assert_eq!(setup_failure.calls, ["mirror:128", "setup:FACE:3f800000"]);
    }

    #[test]
    fn update_is_one_call_and_end_is_effect_free() {
        let leaf = mirrored();
        let mut spy = Spy::default();
        leaf.update(&mut spy).unwrap();
        leaf.end();
        assert_eq!(spy.calls, ["update:FACE:3f800000"]);
    }

    #[test]
    fn lifecycle_projects_every_argument_with_exact_raw_values() {
        struct ArgumentSpy {
            calls: Vec<&'static str>,
        }
        impl PlayRemappedAnimationCollaborator for ArgumentSpy {
            type Error = &'static str;

            fn write_mirror_override_raw(&mut self, value: u8) -> Result<(), Self::Error> {
                assert_eq!(value, 0);
                self.calls.push("mirror");
                Ok(())
            }

            fn setup_remapped_animation(
                &mut self,
                request: RemappedAnimationSetup<'_>,
            ) -> Result<(), Self::Error> {
                // Source ABI order at 0x827180DC..0x82718100, after Entity.
                assert_eq!(request.animation_name, b"ANIM\xff");
                assert_eq!(request.playback_speed_bits, 0x8000_0000);
                assert_eq!(request.frame_offset_tag, b"FRAME-");
                assert_eq!(request.frame_offset_bits, 0x7fc1_2345);
                assert_eq!(request.cycle_percentage_tag, b"CYCLE\xff");
                assert_eq!(request.cycle_percentage_bits, 0x7f80_0000);
                assert_eq!(
                    request.transition_type,
                    RemappedAnimationTransitionType::Blend
                );
                assert_eq!(request.blend_length_bits, 0xff80_0000);
                self.calls.push("setup");
                Ok(())
            }

            fn update_remapped_animation(
                &mut self,
                request: RemappedAnimationUpdate<'_>,
            ) -> Result<(), Self::Error> {
                assert_eq!(request.animation_name, b"ANIM\xff");
                assert_eq!(request.playback_speed_bits, 0x8000_0000);
                self.calls.push("update");
                Err("helper update failed")
            }
        }

        let leaf = PlayRemappedAnimationBehaviour::from_attributes(&[
            attr(b"anim", b"ANIM\xff", 0, 0),
            attr(b"playBackSpeed", b"", 0x8000_0000, 0),
            attr(b"frameOffsetTag", b"FRAME-", 0, 0),
            attr(b"frameOffset", b"", 0x7fc1_2345, 0),
            attr(b"cycleSwappingPercentageTag", b"CYCLE\xff", 0, 0),
            attr(b"cycleSwappingPercentage", b"", 0x7f80_0000, 0),
            attr(b"blendTime", b"", 0xff80_0000, 0),
            attr(b"mirrorOverride", b"", 0, 0),
        ])
        .unwrap();
        let mut spy = ArgumentSpy { calls: Vec::new() };
        leaf.begin(&mut spy).unwrap();
        assert_eq!(leaf.update(&mut spy), Err("helper update failed"));
        leaf.end();
        assert_eq!(spy.calls, ["mirror", "setup", "update"]);
    }

    #[test]
    fn hash_only_first_match_is_retained() {
        let first = attr(b"anim", b"FIRST", 0, 0);
        let mut collision = attr(b"different", b"SECOND", 0, 0);
        collision.key_hash = state_graph_string_hash32(b"anim");
        let leaf = PlayRemappedAnimationBehaviour::from_attributes(&[first, collision]).unwrap();
        assert_eq!(leaf.animation_name(), b"FIRST");
    }
}
