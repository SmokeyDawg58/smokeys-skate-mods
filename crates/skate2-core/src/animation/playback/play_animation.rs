//! Evidence-bounded stored data for `Sk8::Behaviours::PlayAnimation`.
//!
//! This module reconstructs only the constructor and ordered `AddParam`
//! parsing. It does not implement instance creation, `Begin`, `Update`,
//! animation dispatch, or generated-controller integration.
//!
//! The native packing routine accepts arbitrary non-NUL bytes and silently
//! truncates input beyond its fixed capacity. This checked host boundary
//! instead accepts only the alphabet exercised by the shipped assets and
//! rejects over-capacity input, so malformed data cannot be mistaken for a
//! fully faithful native parse.

use skate2_formats::{
    andale::PackedFastString,
    state_graph::{StateGraphBinaryAttribute, state_graph_string_hash32},
};
use std::{error::Error, fmt};

pub const PLAY_ANIMATION_OBJECT_SIZE: usize = 0xa4;
pub const PLAY_ANIMATION_TRANSITION_PARAMS_OFFSET: usize = 0x1c;
pub const PLAY_ANIMATION_ANIM_NAME_OFFSET: usize = 0x2c;
pub const PLAY_ANIMATION_SWITCH_ANIM_NAME_OFFSET: usize = 0x44;
pub const PLAY_ANIMATION_MIRROR_ANIM_NAME_OFFSET: usize = 0x5c;
pub const PLAY_ANIMATION_NO_BOARD_ANIM_NAME_OFFSET: usize = 0x74;
pub const PLAY_ANIMATION_PLAYBACK_SPEED_OFFSET: usize = 0x8c;
pub const PLAY_ANIMATION_PARAM_VECTOR_OFFSET: usize = 0x90;
pub const PLAY_ANIMATION_PRESENCE_FLAGS_OFFSET: usize = 0xa0;
pub const PLAY_ANIMATION_HAS_SWITCH_ANIM_MASK: u32 = 0x8000_0000;
pub const PLAY_ANIMATION_HAS_MIRROR_ANIM_MASK: u32 = 0x4000_0000;
pub const PLAY_ANIMATION_HAS_NO_BOARD_ANIM_MASK: u32 = 0x2000_0000;

pub const TRANSITION_PARAMS_SIZE: usize = 0x10;
pub const TRANSITION_TYPE_OFFSET: usize = 0;
pub const TRANSITION_BLEND_LENGTH_OFFSET: usize = 4;
pub const TRANSITION_PLACEMENT_OFFSET: usize = 8;
pub const TRANSITION_FLAGS_OFFSET: usize = 0xc;
pub const TRANSITION_USE_CHANNEL_FROM_WEIGHTS_MASK: u32 = 0x4000_0000;
pub const TRANSITION_BLEND_WITH_CURRENT_FRAME_MASK: u32 = 0x8000_0000;

pub const PLAY_ANIMATION_ATTRIBUTE_PARAM_SIZE: usize = 0x4c;
pub const ATTRIBUTE_PARAM_TYPE_OFFSET: usize = 0;
pub const ATTRIBUTE_PARAM_INTENT_OFFSET: usize = 4;
pub const ATTRIBUTE_PARAM_ATTRIBUTE_OFFSET: usize = 0x1c;
pub const ATTRIBUTE_PARAM_RENAME_OFFSET: usize = 0x30;
pub const ATTRIBUTE_PARAM_DEFAULT_VALUE_OFFSET: usize = 0x44;
pub const ATTRIBUTE_PARAM_FLAGS_OFFSET: usize = 0x48;
pub const ATTRIBUTE_PARAM_NORMALIZE_MASK: u32 = 0x4000_0000;
pub const ATTRIBUTE_PARAM_USE_DEFAULT_MASK: u32 = 0x8000_0000;

pub const FAST_STRING_36_CAPACITY: usize = 36;
pub const FAST_STRING_30_CAPACITY: usize = 30;

pub const PLAY_ANIMATION_CORPUS_OCCURRENCES: usize = 807;
pub const PLAY_ANIMATION_CORPUS_OCCURRENCES_SHA256: &str =
    "AFFB45C92FD711D67F55296A7A938B71D4CA34969A9821C23563EBEE4050600B";
/// Counts indexed by `AnimationTransitionType`; `Default` is never retained.
pub const PLAY_ANIMATION_CORPUS_TRANSITION_COUNTS: [usize; 5] = [0, 3, 652, 65, 87];
pub const PLAY_ANIMATION_CORPUS_SWITCH_PRESENT: usize = 8;
pub const PLAY_ANIMATION_CORPUS_MIRROR_PRESENT: usize = 28;
pub const PLAY_ANIMATION_CORPUS_NO_BOARD_PRESENT: usize = 4;
pub const PLAY_ANIMATION_CORPUS_BLEND_CURRENT_TRUE: usize = 46;
pub const PLAY_ANIMATION_CORPUS_TRANSITION_UNDER_TRUE: usize = 140;
pub const PLAY_ANIMATION_CORPUS_USE_CHANNEL_TRUE: usize = 64;
pub const PLAY_ANIMATION_CORPUS_PLAYBACK_DEFAULT: usize = 744;
pub const PLAY_ANIMATION_CORPUS_BLEND_TIME_DEFAULT: usize = 87;
pub const PLAY_ANIMATION_CORPUS_PARAMS: usize = 165;
pub const PLAY_ANIMATION_CORPUS_PARAM_TREE_SHA256: &str =
    "44A5BBD22961EAEE9183078B7ED9D66B9D254D6FCE5B577981E45FDBCD320A1C";
/// Counts indexed by `AnimationAttributeParamType`.
pub const PLAY_ANIMATION_CORPUS_PARAM_TYPE_COUNTS: [usize; 3] = [111, 0, 54];
pub const PLAY_ANIMATION_CORPUS_IGNORED_TRANS: usize = 30;
pub const PLAY_ANIMATION_CORPUS_IGNORED_SET_BLEND: usize = 143;

pub type FastString36 = PackedFastString<6>;
pub type FastString30 = PackedFastString<5>;

/// Runtime value assigned from `Andale::nullStringName` by the native static
/// initializer. This is observably different from a present empty string.
pub const ANDALE_NULL_STRING_NAME_30: FastString30 = FastString30 {
    words: [0x7546_0604, 0x86ad_e6d4, 0, 0, 0],
};
/// Six-word form of [`ANDALE_NULL_STRING_NAME_30`] used by the intent field.
pub const ANDALE_NULL_STRING_NAME_36: FastString36 = FastString36 {
    words: [0x7546_0604, 0x86ad_e6d4, 0, 0, 0, 0],
};
pub const EMPTY_FAST_STRING_30: FastString30 = FastString30 { words: [0; 5] };
pub const EMPTY_FAST_STRING_36: FastString36 = FastString36 { words: [0; 6] };

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum AnimationTransitionType {
    Default = 0,
    Play = 1,
    Blend = 2,
    ChannelBlend = 3,
    Sequence = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum AnimationTransitionPlacement {
    Over = 0,
    Under = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationTransitionParams {
    pub transition_type: AnimationTransitionType,
    pub blend_length_bits: u32,
    pub placement: AnimationTransitionPlacement,
    /// The semantic native bit at word bit 31. Unnamed padding is not modeled.
    pub blend_with_current_frame: bool,
    /// The semantic native bit at word bit 30. Unnamed padding is not modeled.
    pub use_channel_from_weights: bool,
}

impl AnimationTransitionParams {
    #[must_use]
    pub const fn blend_length(self) -> f32 {
        f32::from_bits(self.blend_length_bits)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum AnimationAttributeParamType {
    FromLastAnim = 0,
    FromIntent = 1,
    FromFilteredIntent = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayAnimationAttributeParam {
    pub source_type: AnimationAttributeParamType,
    /// Native ctor default is [`ANDALE_NULL_STRING_NAME_36`].
    pub intent: FastString36,
    /// Native ctor default is the zero/empty packed string, not `nullString`.
    pub attribute: FastString30,
    /// Native ctor default is [`ANDALE_NULL_STRING_NAME_30`]; a present empty
    /// `rename` overwrites it with [`EMPTY_FAST_STRING_30`].
    pub rename: FastString30,
    pub default_value_bits: u32,
    /// The semantic native bit at word bit 30. Unnamed padding is not modeled.
    pub normalize: bool,
    /// The semantic native bit at word bit 31. Unnamed padding is not modeled.
    pub use_default: bool,
}

impl PlayAnimationAttributeParam {
    #[must_use]
    pub const fn default_value(self) -> f32 {
        f32::from_bits(self.default_value_bits)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayAnimationBehaviour {
    pub transition: AnimationTransitionParams,
    pub anim_name: FastString36,
    pub switch_anim_name: FastString36,
    pub mirror_anim_name: FastString36,
    pub no_board_anim_name: FastString36,
    pub playback_speed_bits: u32,
    pub has_switch_anim: bool,
    pub has_mirror_anim: bool,
    pub has_no_board_anim: bool,
    params: Vec<PlayAnimationAttributeParam>,
}

impl PlayAnimationBehaviour {
    /// Reconstructs the stored-data portion of `PlayAnimation::PlayAnimation`
    /// at `0x82cbdab0`. Attribute lookup is native hash-only/first-match.
    ///
    /// # Errors
    ///
    /// Returns a checked form of the native required-`anim` assertion or a
    /// checked FastString-domain error. Callers must supply attributes already
    /// validated as C-string data by the `StateGraph` runtime boundary.
    pub fn from_attributes(
        attributes: &[StateGraphBinaryAttribute],
    ) -> Result<Self, PlayAnimationParseError> {
        let transition_type = match attribute(attributes, b"transType").map(|value| &*value.text) {
            Some(b"play") => AnimationTransitionType::Play,
            Some(b"sequence") => AnimationTransitionType::Sequence,
            Some(b"channelblend") => AnimationTransitionType::ChannelBlend,
            _ => AnimationTransitionType::Blend,
        };

        let anim = required_attribute(attributes, "PlayAnimation", b"anim")?;
        let anim_name = encode_fast_string_36("anim", &anim.text)?;

        let switch = attribute(attributes, b"switchAnim");
        let has_switch_anim = switch.is_some();
        let switch_anim_name = match switch {
            Some(value) => encode_fast_string_36("switchAnim", &value.text)?,
            None => anim_name,
        };

        let mirror = attribute(attributes, b"mirrorAnim");
        let has_mirror_anim = mirror.is_some();
        let mirror_anim_name = match mirror {
            Some(value) => encode_fast_string_36("mirrorAnim", &value.text)?,
            None => anim_name,
        };

        let no_board = attribute(attributes, b"noBoardAnim");
        let has_no_board_anim = no_board.is_some();
        let no_board_anim_name = match no_board {
            Some(value) => encode_fast_string_36("noBoardAnim", &value.text)?,
            None => anim_name,
        };

        let playback_speed_bits = attribute(attributes, b"playBackSpeed")
            .map_or(1.0f32.to_bits(), |value| value.float_bits);
        let blend_length_bits =
            attribute(attributes, b"time").map_or(0.2f32.to_bits(), |value| value.float_bits);
        let blend_with_current_frame = attribute(attributes, b"blendWithCurrentFrame")
            .is_some_and(|value| value.boolean_raw & 1 != 0);
        let placement = if attribute(attributes, b"transitionUnder")
            .is_some_and(|value| value.boolean_raw != 0)
        {
            AnimationTransitionPlacement::Under
        } else {
            AnimationTransitionPlacement::Over
        };
        let use_channel_from_weights = attribute(attributes, b"useChannelFromWeights")
            .is_some_and(|value| value.boolean_raw & 1 != 0);

        Ok(Self {
            transition: AnimationTransitionParams {
                transition_type,
                blend_length_bits,
                placement,
                blend_with_current_frame,
                use_channel_from_weights,
            },
            anim_name,
            switch_anim_name,
            mirror_anim_name,
            no_board_anim_name,
            playback_speed_bits,
            has_switch_anim,
            has_mirror_anim,
            has_no_board_anim,
            params: Vec::new(),
        })
    }

    #[must_use]
    pub const fn playback_speed(&self) -> f32 {
        f32::from_bits(self.playback_speed_bits)
    }

    #[must_use]
    pub fn params(&self) -> &[PlayAnimationAttributeParam] {
        &self.params
    }

    /// Reconstructs `PlayAnimation::AddParam` at `0x82cd4298`. The new value
    /// is parsed and allocation is reserved before the ordered append, so a
    /// checked host error leaves the existing list unchanged.
    ///
    /// # Errors
    ///
    /// Returns checked forms of native required-field/unknown-source
    /// assertions, checked-host FastString-domain failures, or allocation
    /// failure. Native `FastString` packing truncates over-capacity input and
    /// accepts a wider byte domain; rejection here is an intentional safety
    /// policy for the shipped-data boundary.
    pub fn add_param(
        &mut self,
        attributes: &[StateGraphBinaryAttribute],
    ) -> Result<(), PlayAnimationParseError> {
        let parsed = parse_param(attributes)?;
        self.params
            .try_reserve(1)
            .map_err(|_| PlayAnimationParseError::Allocation)?;
        self.params.push(parsed);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlayAnimationParseError {
    MissingAttribute {
        context: &'static str,
        key: &'static [u8],
    },
    UnknownParamSource(Vec<u8>),
    FastStringTooLong {
        field: &'static str,
        length: usize,
        maximum: usize,
    },
    InvalidFastStringCharacter {
        field: &'static str,
        index: usize,
        byte: u8,
    },
    Allocation,
}

impl fmt::Display for PlayAnimationParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAttribute { context, key } => write!(
                formatter,
                "{context} is missing required attribute {}",
                String::from_utf8_lossy(key)
            ),
            Self::UnknownParamSource(source) => write!(
                formatter,
                "PlayAnimation Param source {} reaches a native assertion",
                String::from_utf8_lossy(source)
            ),
            Self::FastStringTooLong {
                field,
                length,
                maximum,
            } => write!(
                formatter,
                "PlayAnimation {field} has {length} bytes; checked FastString maximum is {maximum}"
            ),
            Self::InvalidFastStringCharacter { field, index, byte } => write!(
                formatter,
                "PlayAnimation {field} byte {index} is outside the FastString alphabet: {byte:02x}"
            ),
            Self::Allocation => {
                formatter.write_str("could not allocate PlayAnimation Param storage")
            }
        }
    }
}

impl Error for PlayAnimationParseError {}

fn parse_param(
    attributes: &[StateGraphBinaryAttribute],
) -> Result<PlayAnimationAttributeParam, PlayAnimationParseError> {
    let from = required_attribute(attributes, "PlayAnimation Param", b"from")?;

    let (source_type, intent, attribute_name, normalize) = match from.text.as_slice() {
        b"intent" => {
            let value = required_attribute(attributes, "PlayAnimation intent Param", b"intent")?;
            (
                AnimationAttributeParamType::FromIntent,
                encode_fast_string_36("Param intent", &value.text)?,
                EMPTY_FAST_STRING_30,
                attribute(attributes, b"normalize").is_some_and(|value| value.boolean_raw & 1 != 0),
            )
        }
        b"lastAnim" => {
            let value =
                required_attribute(attributes, "PlayAnimation lastAnim Param", b"attribute")?;
            (
                AnimationAttributeParamType::FromLastAnim,
                ANDALE_NULL_STRING_NAME_36,
                encode_fast_string_30("Param attribute", &value.text)?,
                false,
            )
        }
        b"filteredIntent" => {
            let value = required_attribute(
                attributes,
                "PlayAnimation filteredIntent Param",
                b"filteredIntent",
            )?;
            (
                AnimationAttributeParamType::FromFilteredIntent,
                encode_fast_string_36("Param filteredIntent", &value.text)?,
                EMPTY_FAST_STRING_30,
                attribute(attributes, b"normalize").is_some_and(|value| value.boolean_raw & 1 != 0),
            )
        }
        unknown => {
            return Err(PlayAnimationParseError::UnknownParamSource(
                unknown.to_vec(),
            ));
        }
    };

    let rename = match attribute(attributes, b"rename") {
        Some(value) => encode_fast_string_30("Param rename", &value.text)?,
        None => ANDALE_NULL_STRING_NAME_30,
    };
    let default = attribute(attributes, b"defaultValue");
    let (default_value_bits, use_default) =
        default.map_or((0, false), |value| (value.float_bits, true));

    Ok(PlayAnimationAttributeParam {
        source_type,
        intent,
        attribute: attribute_name,
        rename,
        default_value_bits,
        normalize,
        use_default,
    })
}

fn encode_fast_string_36(
    field: &'static str,
    value: &[u8],
) -> Result<FastString36, PlayAnimationParseError> {
    encode_fast_string(field, value, FAST_STRING_36_CAPACITY)
}

fn encode_fast_string_30(
    field: &'static str,
    value: &[u8],
) -> Result<FastString30, PlayAnimationParseError> {
    encode_fast_string(field, value, FAST_STRING_30_CAPACITY)
}

fn encode_fast_string<const WORDS: usize>(
    field: &'static str,
    value: &[u8],
    maximum: usize,
) -> Result<PackedFastString<WORDS>, PlayAnimationParseError> {
    if value.len() > maximum {
        return Err(PlayAnimationParseError::FastStringTooLong {
            field,
            length: value.len(),
            maximum,
        });
    }
    if let Some((index, &byte)) = value
        .iter()
        .enumerate()
        .find(|(_, byte)| !matches!(byte, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_'))
    {
        return Err(PlayAnimationParseError::InvalidFastStringCharacter { field, index, byte });
    }
    let text = std::str::from_utf8(value)
        .expect("validated FastString alphabet contains only ASCII bytes");
    PackedFastString::encode(text).map_err(|_| {
        unreachable!("capacity and alphabet were validated before PackedFastString::encode")
    })
}

fn attribute<'a>(
    attributes: &'a [StateGraphBinaryAttribute],
    key: &[u8],
) -> Option<&'a StateGraphBinaryAttribute> {
    let hash = state_graph_string_hash32(key);
    attributes
        .iter()
        .find(|attribute| attribute.key_hash == hash)
}

fn required_attribute<'a>(
    attributes: &'a [StateGraphBinaryAttribute],
    context: &'static str,
    key: &'static [u8],
) -> Result<&'a StateGraphBinaryAttribute, PlayAnimationParseError> {
    attribute(attributes, key).ok_or(PlayAnimationParseError::MissingAttribute { context, key })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(key: &[u8], text: &[u8], float: f32, boolean_raw: u8) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits: float.to_bits(),
            boolean_raw,
        }
    }

    fn base(anim: &[u8]) -> Vec<StateGraphBinaryAttribute> {
        vec![
            attr(b"name", b"PlayAnimation", 0.0, 0),
            attr(b"anim", anim, 0.0, 0),
        ]
    }

    fn decode<const WORDS: usize>(value: PackedFastString<WORDS>) -> String {
        value.decode().unwrap()
    }

    #[test]
    fn layout_discriminants_and_corpus_oracles_are_frozen() {
        assert_eq!(PLAY_ANIMATION_OBJECT_SIZE, 0xa4);
        assert_eq!(
            (
                PLAY_ANIMATION_TRANSITION_PARAMS_OFFSET,
                PLAY_ANIMATION_ANIM_NAME_OFFSET,
                PLAY_ANIMATION_SWITCH_ANIM_NAME_OFFSET,
                PLAY_ANIMATION_MIRROR_ANIM_NAME_OFFSET,
                PLAY_ANIMATION_NO_BOARD_ANIM_NAME_OFFSET,
                PLAY_ANIMATION_PLAYBACK_SPEED_OFFSET,
                PLAY_ANIMATION_PARAM_VECTOR_OFFSET,
                PLAY_ANIMATION_PRESENCE_FLAGS_OFFSET,
            ),
            (0x1c, 0x2c, 0x44, 0x5c, 0x74, 0x8c, 0x90, 0xa0)
        );
        assert_eq!(
            (
                TRANSITION_PARAMS_SIZE,
                TRANSITION_TYPE_OFFSET,
                TRANSITION_BLEND_LENGTH_OFFSET,
                TRANSITION_PLACEMENT_OFFSET,
                TRANSITION_FLAGS_OFFSET,
            ),
            (0x10, 0, 4, 8, 0xc)
        );
        assert_eq!(
            (
                PLAY_ANIMATION_ATTRIBUTE_PARAM_SIZE,
                ATTRIBUTE_PARAM_TYPE_OFFSET,
                ATTRIBUTE_PARAM_INTENT_OFFSET,
                ATTRIBUTE_PARAM_ATTRIBUTE_OFFSET,
                ATTRIBUTE_PARAM_RENAME_OFFSET,
                ATTRIBUTE_PARAM_DEFAULT_VALUE_OFFSET,
                ATTRIBUTE_PARAM_FLAGS_OFFSET,
            ),
            (0x4c, 0, 4, 0x1c, 0x30, 0x44, 0x48)
        );
        assert_eq!(
            (
                AnimationTransitionType::Default as i32,
                AnimationTransitionType::Play as i32,
                AnimationTransitionType::Blend as i32,
                AnimationTransitionType::ChannelBlend as i32,
                AnimationTransitionType::Sequence as i32,
            ),
            (0, 1, 2, 3, 4)
        );
        assert_eq!(
            (
                AnimationAttributeParamType::FromLastAnim as i32,
                AnimationAttributeParamType::FromIntent as i32,
                AnimationAttributeParamType::FromFilteredIntent as i32,
            ),
            (0, 1, 2)
        );
        assert_eq!(PLAY_ANIMATION_CORPUS_OCCURRENCES, 807);
        assert_eq!(PLAY_ANIMATION_CORPUS_TRANSITION_COUNTS, [0, 3, 652, 65, 87]);
        assert_eq!(
            (
                PLAY_ANIMATION_CORPUS_SWITCH_PRESENT,
                PLAY_ANIMATION_CORPUS_MIRROR_PRESENT,
                PLAY_ANIMATION_CORPUS_NO_BOARD_PRESENT,
                PLAY_ANIMATION_CORPUS_BLEND_CURRENT_TRUE,
                PLAY_ANIMATION_CORPUS_TRANSITION_UNDER_TRUE,
                PLAY_ANIMATION_CORPUS_USE_CHANNEL_TRUE,
                PLAY_ANIMATION_CORPUS_PLAYBACK_DEFAULT,
                PLAY_ANIMATION_CORPUS_BLEND_TIME_DEFAULT,
                PLAY_ANIMATION_CORPUS_IGNORED_TRANS,
                PLAY_ANIMATION_CORPUS_IGNORED_SET_BLEND,
            ),
            (8, 28, 4, 46, 140, 64, 744, 87, 30, 143)
        );
        assert_eq!(PLAY_ANIMATION_CORPUS_PARAM_TYPE_COUNTS, [111, 0, 54]);
        assert_eq!(PLAY_ANIMATION_CORPUS_PARAMS, 165);
        assert_eq!(PLAY_ANIMATION_CORPUS_OCCURRENCES_SHA256.len(), 64);
        assert_eq!(PLAY_ANIMATION_CORPUS_PARAM_TREE_SHA256.len(), 64);
    }

    #[test]
    fn fast_string_null_sentinel_and_reference_vectors_are_frozen() {
        assert_eq!(
            ANDALE_NULL_STRING_NAME_30.words,
            [0x7546_0604, 0x86ad_e6d4, 0, 0, 0]
        );
        assert_eq!(
            ANDALE_NULL_STRING_NAME_36.words,
            [0x7546_0604, 0x86ad_e6d4, 0, 0, 0, 0]
        );
        assert_eq!(
            FastString30::encode("nullString").unwrap(),
            ANDALE_NULL_STRING_NAME_30
        );
        assert_eq!(
            FastString36::encode("NULLSTRING").unwrap(),
            ANDALE_NULL_STRING_NAME_36
        );
        assert_ne!(ANDALE_NULL_STRING_NAME_30, EMPTY_FAST_STRING_30);
        assert_ne!(ANDALE_NULL_STRING_NAME_36, EMPTY_FAST_STRING_36);
        assert_eq!(
            FastString30::encode("Trajectory").unwrap().words,
            [0x9133_81df, 0x90e2_9cdc, 0, 0, 0]
        );
        assert_eq!(
            FastString36::encode("INVERT_HANDPLANT_FS_1FTFRONT_INTO_FC")
                .unwrap()
                .words,
            [
                0x5cd2_5c82,
                0xb104_a856,
                0x6959_7af6,
                0x8d91_35a4,
                0x876d_1c19,
                0x7529_0a99,
            ]
        );
    }

    #[test]
    fn constructor_defaults_and_case_insensitive_fast_strings_are_exact() {
        let parsed = PlayAnimationBehaviour::from_attributes(&base(b"B_PUSH_CYC")).unwrap();
        assert_eq!(
            parsed.transition,
            AnimationTransitionParams {
                transition_type: AnimationTransitionType::Blend,
                blend_length_bits: 0.2f32.to_bits(),
                placement: AnimationTransitionPlacement::Over,
                blend_with_current_frame: false,
                use_channel_from_weights: false,
            }
        );
        assert_eq!(parsed.playback_speed_bits, 1.0f32.to_bits());
        assert_eq!(decode(parsed.anim_name), "b_push_cyc");
        assert_eq!(
            parsed.anim_name,
            FastString36::encode("b_push_cyc").unwrap()
        );
        assert_eq!(parsed.switch_anim_name, parsed.anim_name);
        assert_eq!(parsed.mirror_anim_name, parsed.anim_name);
        assert_eq!(parsed.no_board_anim_name, parsed.anim_name);
        assert!(!parsed.has_switch_anim);
        assert!(!parsed.has_mirror_anim);
        assert!(!parsed.has_no_board_anim);
        assert!(parsed.params().is_empty());
    }

    #[test]
    fn transition_type_spellings_and_ignored_trans_are_exact() {
        for (text, expected) in [
            (b"play".as_slice(), AnimationTransitionType::Play),
            (b"sequence", AnimationTransitionType::Sequence),
            (b"channelblend", AnimationTransitionType::ChannelBlend),
            (b"blend", AnimationTransitionType::Blend),
            (b"Play", AnimationTransitionType::Blend),
            (b"unknown", AnimationTransitionType::Blend),
        ] {
            let mut attributes = base(b"A");
            attributes.push(attr(b"transType", text, 0.0, 0));
            attributes.push(attr(b"trans", b"play", 0.0, 0));
            let parsed = PlayAnimationBehaviour::from_attributes(&attributes).unwrap();
            assert_eq!(parsed.transition.transition_type, expected, "{text:?}");
        }
    }

    #[test]
    fn constructor_requires_anim_and_uses_hash_only_first_match() {
        assert!(matches!(
            PlayAnimationBehaviour::from_attributes(&[]),
            Err(PlayAnimationParseError::MissingAttribute {
                context: "PlayAnimation",
                key: b"anim",
            })
        ));
        assert!(matches!(
            PlayAnimationBehaviour::from_attributes(&[attr(b"Anim", b"WRONG_CASE", 0.0, 0)]),
            Err(PlayAnimationParseError::MissingAttribute { key: b"anim", .. })
        ));
        let parsed = PlayAnimationBehaviour::from_attributes(&[
            attr(b"anim", b"FIRST", 0.0, 0),
            attr(b"anim", b"SECOND", 0.0, 0),
        ])
        .unwrap();
        assert_eq!(decode(parsed.anim_name), "first");

        assert_eq!(state_graph_string_hash32(b"a05763"), 0xfe11_cd43);
        assert_eq!(state_graph_string_hash32(b"a063d3"), 0xfe11_cd43);
        let collisions = [
            attr(b"a05763", b"FIRST", 0.0, 0),
            attr(b"a063d3", b"SECOND", 0.0, 0),
        ];
        assert_eq!(attribute(&collisions, b"a063d3").unwrap().text, b"FIRST");
    }

    #[test]
    fn alternative_presence_is_independent_of_packed_value() {
        let mut attributes = base(b"BASE");
        attributes.extend([
            attr(b"switchAnim", b"base", 0.0, 0),
            attr(b"mirrorAnim", b"", 0.0, 0),
            attr(b"noBoardAnim", b"NO_BOARD", 0.0, 0),
        ]);
        let parsed = PlayAnimationBehaviour::from_attributes(&attributes).unwrap();
        assert!(parsed.has_switch_anim);
        assert_eq!(parsed.switch_anim_name, parsed.anim_name);
        assert!(parsed.has_mirror_anim);
        assert_eq!(parsed.mirror_anim_name, FastString36 { words: [0; 6] });
        assert!(parsed.has_no_board_anim);
        assert_eq!(decode(parsed.no_board_anim_name), "no_board");
    }

    #[test]
    fn floats_preserve_bits_and_bool_consumers_use_exact_source_bits() {
        let mut attributes = base(b"A");
        let mut playback = attr(b"playBackSpeed", b"", 0.0, 0);
        playback.float_bits = 0x7fc0_1234;
        let mut blend = attr(b"time", b"", 0.0, 0);
        blend.float_bits = (-0.0f32).to_bits();
        attributes.extend([
            playback,
            blend,
            attr(b"blendWithCurrentFrame", b"", 0.0, 0x80),
            attr(b"transitionUnder", b"", 0.0, 0x80),
            attr(b"useChannelFromWeights", b"", 0.0, 0x81),
        ]);
        let parsed = PlayAnimationBehaviour::from_attributes(&attributes).unwrap();
        assert_eq!(parsed.playback_speed_bits, 0x7fc0_1234);
        assert_eq!(parsed.transition.blend_length_bits, (-0.0f32).to_bits());
        assert!(!parsed.transition.blend_with_current_frame);
        assert_eq!(
            parsed.transition.placement,
            AnimationTransitionPlacement::Under
        );
        assert!(parsed.transition.use_channel_from_weights);

        let mut bit_zero = base(b"A");
        bit_zero.extend([
            attr(b"blendWithCurrentFrame", b"", 0.0, 0x81),
            attr(b"transitionUnder", b"", 0.0, 0),
            attr(b"useChannelFromWeights", b"", 0.0, 0x80),
        ]);
        let parsed = PlayAnimationBehaviour::from_attributes(&bit_zero).unwrap();
        assert!(parsed.transition.blend_with_current_frame);
        assert_eq!(
            parsed.transition.placement,
            AnimationTransitionPlacement::Over
        );
        assert!(!parsed.transition.use_channel_from_weights);
    }

    #[test]
    fn fast_string_capacity_and_alphabet_are_checked() {
        let accepted = vec![b'A'; FAST_STRING_36_CAPACITY];
        assert!(PlayAnimationBehaviour::from_attributes(&base(&accepted)).is_ok());
        let too_long = vec![b'A'; FAST_STRING_36_CAPACITY + 1];
        assert!(matches!(
            PlayAnimationBehaviour::from_attributes(&base(&too_long)),
            Err(PlayAnimationParseError::FastStringTooLong {
                field: "anim",
                length: 37,
                maximum: 36,
            })
        ));
        assert_eq!(
            PlayAnimationBehaviour::from_attributes(&base(b"BAD-NAME")),
            Err(PlayAnimationParseError::InvalidFastStringCharacter {
                field: "anim",
                index: 3,
                byte: b'-',
            })
        );
        assert!(matches!(
            PlayAnimationBehaviour::from_attributes(&base(&[0xff])),
            Err(PlayAnimationParseError::InvalidFastStringCharacter {
                index: 0,
                byte: 0xff,
                ..
            })
        ));
    }

    #[test]
    fn add_param_parses_all_sources_and_appends_in_source_order() {
        let mut parsed = PlayAnimationBehaviour::from_attributes(&base(b"A")).unwrap();
        parsed
            .add_param(&[
                attr(b"from", b"intent", 0.0, 0),
                attr(b"intent", b"Push", 0.0, 0),
                attr(b"normalize", b"", 0.0, 0x81),
            ])
            .unwrap();
        parsed
            .add_param(&[
                attr(b"from", b"lastAnim", 0.0, 0),
                attr(b"attribute", b"Speed", 0.0, 0),
                attr(b"normalize", b"", 0.0, 1),
            ])
            .unwrap();
        parsed
            .add_param(&[
                attr(b"from", b"filteredIntent", 0.0, 0),
                attr(b"filteredIntent", b"TweakX", 0.0, 0),
                attr(b"normalize", b"", 0.0, 0x80),
            ])
            .unwrap();
        assert_eq!(
            parsed
                .params()
                .iter()
                .map(|value| value.source_type)
                .collect::<Vec<_>>(),
            [
                AnimationAttributeParamType::FromIntent,
                AnimationAttributeParamType::FromLastAnim,
                AnimationAttributeParamType::FromFilteredIntent,
            ]
        );
        assert_eq!(decode(parsed.params()[0].intent), "push");
        assert_eq!(parsed.params()[0].attribute, EMPTY_FAST_STRING_30);
        assert_eq!(parsed.params()[0].rename, ANDALE_NULL_STRING_NAME_30);
        assert!(parsed.params()[0].normalize);
        assert_eq!(parsed.params()[1].intent, ANDALE_NULL_STRING_NAME_36);
        assert_eq!(decode(parsed.params()[1].attribute), "speed");
        assert_eq!(parsed.params()[1].rename, ANDALE_NULL_STRING_NAME_30);
        assert!(!parsed.params()[1].normalize);
        assert_eq!(decode(parsed.params()[2].intent), "tweakx");
        assert_eq!(parsed.params()[2].attribute, EMPTY_FAST_STRING_30);
        assert_eq!(parsed.params()[2].rename, ANDALE_NULL_STRING_NAME_30);
        assert!(!parsed.params()[2].normalize);
    }

    #[test]
    fn add_param_required_and_unknown_errors_are_atomic() {
        let mut parsed = PlayAnimationBehaviour::from_attributes(&base(b"A")).unwrap();
        for (attributes, expected_key) in [
            (Vec::new(), b"from".as_slice()),
            (vec![attr(b"from", b"intent", 0.0, 0)], b"intent"),
            (vec![attr(b"from", b"lastAnim", 0.0, 0)], b"attribute"),
            (
                vec![attr(b"from", b"filteredIntent", 0.0, 0)],
                b"filteredIntent",
            ),
        ] {
            assert!(matches!(
                parsed.add_param(&attributes),
                Err(PlayAnimationParseError::MissingAttribute { key, .. }) if key == expected_key
            ));
            assert!(parsed.params().is_empty());
        }
        assert_eq!(
            parsed.add_param(&[attr(b"from", b"Intent", 0.0, 0)]),
            Err(PlayAnimationParseError::UnknownParamSource(
                b"Intent".to_vec()
            ))
        );
        assert!(parsed.params().is_empty());
    }

    #[test]
    fn param_rename_default_presence_and_ignored_attributes_are_exact() {
        let mut parsed = PlayAnimationBehaviour::from_attributes(&base(b"A")).unwrap();
        let mut default = attr(b"defaultValue", b"", 0.0, 0);
        default.float_bits = 0x7fc0_9876;
        parsed
            .add_param(&[
                attr(b"SetBlend", b"true", 0.0, 1),
                attr(b"from", b"lastAnim", 0.0, 0),
                attr(b"attribute", b"TrickHeight", 0.0, 0),
                attr(b"rename", b"", 0.0, 0),
                default,
            ])
            .unwrap();
        let value = parsed.params()[0];
        assert_eq!(value.rename, FastString30 { words: [0; 5] });
        assert!(value.use_default);
        assert_eq!(value.default_value_bits, 0x7fc0_9876);

        parsed
            .add_param(&[
                attr(b"from", b"lastAnim", 0.0, 0),
                attr(b"attribute", b"TrickHeight", 0.0, 0),
            ])
            .unwrap();
        assert_ne!(parsed.params()[1].rename, value.rename);
        assert_eq!(parsed.params()[1].rename, ANDALE_NULL_STRING_NAME_30);
        assert_ne!(parsed.params()[1].rename, EMPTY_FAST_STRING_30);
        assert!(!parsed.params()[1].use_default);
        assert_eq!(parsed.params()[1].default_value_bits, 0);
    }

    #[test]
    fn param_fast_string_30_bound_and_collision_lookup_are_exact() {
        let mut parsed = PlayAnimationBehaviour::from_attributes(&base(b"A")).unwrap();
        let accepted = vec![b'Z'; FAST_STRING_30_CAPACITY];
        parsed
            .add_param(&[
                attr(b"from", b"lastAnim", 0.0, 0),
                attr(b"attribute", &accepted, 0.0, 0),
            ])
            .unwrap();
        assert_eq!(decode(parsed.params()[0].attribute).len(), 30);

        let too_long = vec![b'Z'; FAST_STRING_30_CAPACITY + 1];
        assert!(matches!(
            parsed.add_param(&[
                attr(b"from", b"lastAnim", 0.0, 0),
                attr(b"attribute", &too_long, 0.0, 0),
            ]),
            Err(PlayAnimationParseError::FastStringTooLong {
                field: "Param attribute",
                length: 31,
                maximum: 30,
            })
        ));
        assert_eq!(parsed.params().len(), 1);

        let long_rename = vec![b'Z'; FAST_STRING_30_CAPACITY + 1];
        assert!(matches!(
            parsed.add_param(&[
                attr(b"from", b"lastAnim", 0.0, 0),
                attr(b"attribute", b"A", 0.0, 0),
                attr(b"rename", &long_rename, 0.0, 0),
            ]),
            Err(PlayAnimationParseError::FastStringTooLong {
                field: "Param rename",
                length: 31,
                maximum: 30,
            })
        ));
        assert_eq!(parsed.params().len(), 1);

        parsed
            .add_param(&[
                attr(b"from", b"lastAnim", 0.0, 0),
                attr(b"attribute", b"FIRST", 0.0, 0),
                attr(b"attribute", b"SECOND", 0.0, 0),
            ])
            .unwrap();
        assert_eq!(decode(parsed.params()[1].attribute), "first");
    }
}
