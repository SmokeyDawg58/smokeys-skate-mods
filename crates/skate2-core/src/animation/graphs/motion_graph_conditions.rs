//! Evidence-bounded `HasIntent` and `WillExpire` `MotionGraph` conditions.
//!
//! Parsing consumes the lossless `StateGraph` binary attribute representation.
//! Entity/interface access stays behind explicit collaborators; no Vault or
//! animation-database behavior is fabricated. Generated direct helpers are not
//! implemented here: their `+0x10` write targets a distinct generated
//! condition object's evaluation-parent field, not the dynamic arena's
//! structural `BaseId`. No equivalent generated-object seam is currently
//! represented by these types.

use skate2_formats::state_graph::{StateGraphBinaryAttribute, state_graph_string_hash32};
use std::{error::Error, fmt};

pub const HAS_INTENT_OBJECT_SIZE: usize = 0x40;
pub const HAS_INTENT_OP_OFFSET: usize = 0x1c;
pub const HAS_INTENT_REFERENCE_VALUE_OFFSET: usize = 0x20;
pub const HAS_INTENT_ABS_OFFSET: usize = 0x24;
pub const HAS_INTENT_NAME_OFFSET: usize = 0x28;

pub const WILL_EXPIRE_OBJECT_SIZE: usize = 0x48;
pub const WILL_EXPIRE_TIME_OFFSET: usize = 0x1c;
pub const WILL_EXPIRE_TIME_TAG_OFFSET: usize = 0x20;
pub const WILL_EXPIRE_WAIT_FOR_TRANSITIONS_OFFSET: usize = 0x45;
pub const WILL_EXPIRE_TIME_TAG_STORAGE: usize = 37;
pub const WILL_EXPIRE_TIME_TAG_MAX_BYTES: usize = WILL_EXPIRE_TIME_TAG_STORAGE - 1;

pub const ANIM_PROPERTY_SIZE: usize = 12;
pub const ANIM_PROPERTY_IS_PLAYING_OFFSET: usize = 0;
pub const ANIM_PROPERTY_HAS_EXPIRED_OR_LOOPED_OFFSET: usize = 1;
pub const ANIM_PROPERTY_HAS_EXPIRED_OR_LOOPED_BY_TIME_OFFSET: usize = 4;
pub const ANIM_PROPERTY_TIME_TO_EXPIRE_OR_LOOP_OFFSET: usize = 8;

pub const HAS_INTENT_CORPUS_OCCURRENCES: usize = 815;
pub const HAS_INTENT_CORPUS_DISTINCT_INTENTS: usize = 118;
pub const HAS_INTENT_CORPUS_OCCURRENCES_SHA256: &str =
    "BD94466E48246A4F54AA28E24DF715904B7E3D67ABA8C0289E4477867A6478E8";
/// Counts indexed by the exact `ComparisonOperator` discriminant.
pub const HAS_INTENT_CORPUS_OP_COUNTS: [usize; 7] = [800, 0, 0, 9, 4, 1, 1];
pub const HAS_INTENT_CORPUS_PRE_COND: usize = 708;
pub const HAS_INTENT_CORPUS_ALWAYS_COND: usize = 107;

pub const WILL_EXPIRE_CORPUS_OCCURRENCES: usize = 780;
pub const WILL_EXPIRE_CORPUS_OCCURRENCES_SHA256: &str =
    "B594419415B53139AD132E6F1C6B9141F3FF964C9582381AD91FD564C1A65B0D";
pub const WILL_EXPIRE_CORPUS_EXACT_IN_TIME: usize = 765;
pub const WILL_EXPIRE_CORPUS_WRONG_CASE_IN_TIME: usize = 15;
pub const WILL_EXPIRE_CORPUS_WAIT_DEFAULT_TRUE: usize = 471;
pub const WILL_EXPIRE_CORPUS_WAIT_FALSE: usize = 309;
pub const MOTION_LEAF_CLUSTER_ORACLE_SHA256: &str =
    "9716F1E652D4CA7F5150BEAB48612BEC97488E9209FED1A19DDCE38D3B4DBED6";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ComparisonOperator {
    Nop = 0,
    Equals = 1,
    NotEqual = 2,
    GreaterThan = 3,
    LessThan = 4,
    GreaterEqual = 5,
    LessEqual = 6,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComparisonCondition {
    pub operator: ComparisonOperator,
    pub reference_value_bits: u32,
    /// Exact byte stored by native `GetBool`; consumers test it as nonzero.
    pub absolute_value_raw: u8,
}

impl ComparisonCondition {
    #[must_use]
    pub const fn reference_value(self) -> f32 {
        f32::from_bits(self.reference_value_bits)
    }

    #[must_use]
    pub const fn absolute_value(self) -> bool {
        self.absolute_value_raw != 0
    }

    /// Reconstructs `ComparisonCondition::EvalCompare` at `0x82ceeea0`.
    /// PPC `bge`/`ble` test the negated LT/GT bits, so their unordered results
    /// are true rather than Rust's ordinary `>=`/`<=` NaN behavior.
    #[must_use]
    pub fn evaluate(self, value: f32) -> bool {
        let value = if self.absolute_value_raw != 0 {
            f32::from_bits(value.to_bits() & 0x7fff_ffff)
        } else {
            value
        };
        let reference = self.reference_value();
        match self.operator {
            ComparisonOperator::Nop => false,
            ComparisonOperator::Equals => matches!(
                value.partial_cmp(&reference),
                Some(std::cmp::Ordering::Equal)
            ),
            ComparisonOperator::NotEqual => !matches!(
                value.partial_cmp(&reference),
                Some(std::cmp::Ordering::Equal)
            ),
            ComparisonOperator::GreaterThan => matches!(
                value.partial_cmp(&reference),
                Some(std::cmp::Ordering::Greater)
            ),
            ComparisonOperator::LessThan => matches!(
                value.partial_cmp(&reference),
                Some(std::cmp::Ordering::Less)
            ),
            ComparisonOperator::GreaterEqual => !matches!(
                value.partial_cmp(&reference),
                Some(std::cmp::Ordering::Less)
            ),
            ComparisonOperator::LessEqual => !matches!(
                value.partial_cmp(&reference),
                Some(std::cmp::Ordering::Greater)
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HasIntentCondition {
    pub comparison: ComparisonCondition,
    pub intent_name: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WillExpireCondition {
    pub time_bits: u32,
    /// Initialized C-string content only. Native leaves bytes after its copied
    /// terminator uninitialized, so this host representation does not expose
    /// or assign semantic meaning to that tail storage.
    time_tag: Vec<u8>,
    /// Exact byte stored by native `GetBool`; consumers test it as nonzero.
    pub wait_for_transitions_raw: u8,
}

impl WillExpireCondition {
    #[must_use]
    pub const fn time(&self) -> f32 {
        f32::from_bits(self.time_bits)
    }

    #[must_use]
    pub fn time_tag(&self) -> &[u8] {
        &self.time_tag
    }

    #[must_use]
    pub const fn wait_for_transitions(&self) -> bool {
        self.wait_for_transitions_raw != 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionConditionParseError {
    MissingAttribute {
        condition: &'static str,
        key: &'static [u8],
    },
    UnknownComparison(Vec<u8>),
    TimeTagTooLong {
        length: usize,
        maximum: usize,
    },
    TimeTagContainsNul,
}

impl fmt::Display for MotionConditionParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAttribute { condition, key } => write!(
                formatter,
                "{condition} is missing required attribute {}",
                String::from_utf8_lossy(key)
            ),
            Self::UnknownComparison(value) => write!(
                formatter,
                "ComparisonCondition comparison {} reaches a native assertion",
                String::from_utf8_lossy(value)
            ),
            Self::TimeTagTooLong { length, maximum } => write!(
                formatter,
                "WillExpire InTimeTag has {length} bytes; checked maximum is {maximum} plus NUL"
            ),
            Self::TimeTagContainsNul => {
                write!(formatter, "WillExpire InTimeTag contains an embedded NUL")
            }
        }
    }
}

impl Error for MotionConditionParseError {}

impl HasIntentCondition {
    /// Parses the exact constructor attribute surface. The caller supplies the
    /// validated binary attributes passed to the `StateGraph` factory.
    ///
    /// # Errors
    ///
    /// Returns checked-host forms of native required-attribute and unknown
    /// comparison assertion paths.
    pub fn from_attributes(
        attributes: &[StateGraphBinaryAttribute],
    ) -> Result<Self, MotionConditionParseError> {
        let comparison = parse_comparison(attributes)?;
        let intent_name = required_attribute(attributes, "HasIntent", b"intent")?
            .text
            .clone();
        Ok(Self {
            comparison,
            intent_name,
        })
    }

    /// Reconstructs `HasIntent::ActivationFunc` at `0x82cc24b0`.
    ///
    /// # Errors
    ///
    /// Returns collaborator failures or checked forms of native interface
    /// identity/non-null assertions.
    pub fn activate<B: HasIntentCollaborator>(
        &self,
        backend: &mut B,
    ) -> Result<bool, HasIntentActivationError<B::Error>> {
        let cached = backend
            .cached_motion_graph()
            .map_err(HasIntentActivationError::Backend)?;
        let found = backend
            .find_motion_graph()
            .map_err(HasIntentActivationError::Backend)?;
        if cached != found {
            return Err(HasIntentActivationError::MotionGraphIdentityMismatch);
        }
        let motion_graph = cached.ok_or(HasIntentActivationError::MissingMotionGraph)?;
        let intents = backend
            .motion_graph_intents(&motion_graph)
            .map_err(HasIntentActivationError::Backend)?;
        let value = backend
            .lookup_intent(&intents, &self.intent_name)
            .map_err(HasIntentActivationError::Backend)?;
        let Some(value) = value else {
            return Ok(false);
        };
        if self.comparison.operator == ComparisonOperator::Nop {
            Ok(true)
        } else {
            Ok(self.comparison.evaluate(value))
        }
    }
}

impl WillExpireCondition {
    /// Parses `WillExpire::WillExpire` at `0x82c8cfc8`, rejecting the native
    /// unbounded-copy case when the 37-byte destination cannot hold tag+NUL.
    ///
    /// # Errors
    ///
    /// Returns a checked-host tag storage error.
    pub fn from_attributes(
        attributes: &[StateGraphBinaryAttribute],
    ) -> Result<Self, MotionConditionParseError> {
        let time_bits = attribute(attributes, b"InTime").map_or(0, |value| value.float_bits);
        let wait_for_transitions_raw =
            attribute(attributes, b"waitForTransitions").map_or(1, |value| value.boolean_raw);
        let time_tag = if let Some(value) = attribute(attributes, b"InTimeTag") {
            if value.text.contains(&0) {
                return Err(MotionConditionParseError::TimeTagContainsNul);
            }
            if value.text.len() > WILL_EXPIRE_TIME_TAG_MAX_BYTES {
                return Err(MotionConditionParseError::TimeTagTooLong {
                    length: value.text.len(),
                    maximum: WILL_EXPIRE_TIME_TAG_MAX_BYTES,
                });
            }
            value.text.clone()
        } else {
            Vec::new()
        };
        Ok(Self {
            time_bits,
            time_tag,
            wait_for_transitions_raw,
        })
    }

    /// Reconstructs `WillExpire::ActivationFunc` at `0x82c9bc78` and its tag
    /// resolver at `0x82a74390`.
    ///
    /// # Errors
    ///
    /// Returns collaborator failures or checked forms of native interface,
    /// identity, and non-null `AnimProperty` assumptions.
    pub fn activate<B: WillExpireCollaborator>(
        &self,
        backend: &mut B,
    ) -> Result<bool, WillExpireActivationError<B::Error>> {
        Self::activate_values(
            self.time(),
            self.time_tag(),
            self.wait_for_transitions_raw,
            backend,
        )
    }

    /// Same activation for the generated graph's already constructed payload.
    pub(crate) fn activate_values<B: WillExpireCollaborator>(
        time: f32,
        time_tag: &[u8],
        wait_for_transitions_raw: u8,
        backend: &mut B,
    ) -> Result<bool, WillExpireActivationError<B::Error>> {
        let threshold = if time_tag.is_empty() {
            time
        } else {
            match backend
                .find_anim_info()
                .map_err(WillExpireActivationError::Backend)?
            {
                Some(anim_info) => backend
                    .resolve_special_tag(&anim_info, time_tag)
                    .map_err(WillExpireActivationError::Backend)?,
                None => time,
            }
        };

        let cached = backend
            .cached_animatable()
            .map_err(WillExpireActivationError::Backend)?;
        let found = backend
            .find_animatable()
            .map_err(WillExpireActivationError::Backend)?;
        if cached != found {
            return Err(WillExpireActivationError::AnimatableIdentityMismatch);
        }
        let animatable = cached.ok_or(WillExpireActivationError::MissingAnimatable)?;
        let transitioning = backend
            .transitioning(&animatable)
            .map_err(WillExpireActivationError::Backend)?;
        if wait_for_transitions_raw != 0 && transitioning {
            return Ok(false);
        }
        let property = backend
            .anim_property(&animatable)
            .map_err(WillExpireActivationError::Backend)?
            .ok_or(WillExpireActivationError::MissingAnimProperty)?;
        Ok(property.has_expired_or_looped_raw != 0
            || matches!(
                property.time_to_expire_or_loop.partial_cmp(&threshold),
                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            ))
    }
}

fn parse_comparison(
    attributes: &[StateGraphBinaryAttribute],
) -> Result<ComparisonCondition, MotionConditionParseError> {
    let (operator, reference_value_bits) =
        if let Some(comparison) = attribute(attributes, b"comparison") {
            let value = required_attribute(attributes, "ComparisonCondition", b"value")?;
            let operator = match comparison.text.as_slice() {
                b"Equals" => ComparisonOperator::Equals,
                b"NotEqual" => ComparisonOperator::NotEqual,
                b"GreaterThan" => ComparisonOperator::GreaterThan,
                b"LessThan" => ComparisonOperator::LessThan,
                b"GreaterEqual" => ComparisonOperator::GreaterEqual,
                b"LessEqual" => ComparisonOperator::LessEqual,
                unknown => {
                    return Err(MotionConditionParseError::UnknownComparison(
                        unknown.to_vec(),
                    ));
                }
            };
            (operator, value.float_bits)
        } else {
            const SHORTHAND: [(&[u8], ComparisonOperator); 6] = [
                (b"equal", ComparisonOperator::Equals),
                (b"notEqual", ComparisonOperator::NotEqual),
                (b"greater", ComparisonOperator::GreaterThan),
                (b"less", ComparisonOperator::LessThan),
                (b"greaterEqual", ComparisonOperator::GreaterEqual),
                (b"lessEqual", ComparisonOperator::LessEqual),
            ];
            SHORTHAND
                .into_iter()
                .find_map(|(key, operator)| {
                    attribute(attributes, key).map(|value| (operator, value.float_bits))
                })
                .unwrap_or((ComparisonOperator::Nop, 0))
        };
    let absolute_value_raw = attribute(attributes, b"abs").map_or(0, |value| value.boolean_raw);
    Ok(ComparisonCondition {
        operator,
        reference_value_bits,
        absolute_value_raw,
    })
}

fn attribute<'a>(
    attributes: &'a [StateGraphBinaryAttribute],
    key: &[u8],
) -> Option<&'a StateGraphBinaryAttribute> {
    let hash = state_graph_string_hash32(key);
    attributes.iter().find(|value| value.key_hash == hash)
}

fn required_attribute<'a>(
    attributes: &'a [StateGraphBinaryAttribute],
    condition: &'static str,
    key: &'static [u8],
) -> Result<&'a StateGraphBinaryAttribute, MotionConditionParseError> {
    attribute(attributes, key).ok_or(MotionConditionParseError::MissingAttribute { condition, key })
}

pub trait HasIntentCollaborator {
    type Error;
    type MotionGraph: PartialEq;
    type Intents;

    /// Reads the entity's cached `IMotionGraph` slot.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn cached_motion_graph(&mut self) -> Result<Option<Self::MotionGraph>, Self::Error>;
    /// Performs the generic entity interface lookup used for identity checking.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn find_motion_graph(&mut self) -> Result<Option<Self::MotionGraph>, Self::Error>;
    /// Calls `IMotionGraph::GetMotionGraphIntents_2`.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn motion_graph_intents(
        &mut self,
        motion_graph: &Self::MotionGraph,
    ) -> Result<Self::Intents, Self::Error>;
    /// Reads one exact named value through `Intents::HasIntent`.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn lookup_intent(
        &mut self,
        intents: &Self::Intents,
        intent_name: &[u8],
    ) -> Result<Option<f32>, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HasIntentActivationError<E> {
    Backend(E),
    MotionGraphIdentityMismatch,
    MissingMotionGraph,
}

impl<E: fmt::Display> fmt::Display for HasIntentActivationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(formatter, "HasIntent collaborator failed: {error}"),
            Self::MotionGraphIdentityMismatch => {
                write!(
                    formatter,
                    "cached and generic IMotionGraph identities differ"
                )
            }
            Self::MissingMotionGraph => write!(formatter, "HasIntent requires IMotionGraph"),
        }
    }
}

impl<E: Error + 'static> Error for HasIntentActivationError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::MotionGraphIdentityMismatch | Self::MissingMotionGraph => None,
        }
    }
}

/// One entity-bound view of the tag and animatable interfaces used by
/// `WillExpire`; a single collaborator prevents cross-entity backend pairing.
pub trait WillExpireCollaborator {
    type Error;
    type AnimInfo;
    type Animatable: PartialEq;

    /// Looks up the optional `IAnimInfo` interface.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn find_anim_info(&mut self) -> Result<Option<Self::AnimInfo>, Self::Error>;
    /// Calls `GetAnimInfoWithSpecialTags_2` for an exact nonempty tag.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn resolve_special_tag(
        &mut self,
        anim_info: &Self::AnimInfo,
        tag: &[u8],
    ) -> Result<f32, Self::Error>;
    /// Reads the entity's cached `IAnimatable` slot.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn cached_animatable(&mut self) -> Result<Option<Self::Animatable>, Self::Error>;
    /// Performs the generic entity interface lookup used for identity checking.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn find_animatable(&mut self) -> Result<Option<Self::Animatable>, Self::Error>;
    /// Calls `IAnimatable::Transitioning`.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn transitioning(&mut self, animatable: &Self::Animatable) -> Result<bool, Self::Error>;
    /// Calls `IAnimatable::GetAnimProp` and snapshots its four fields.
    ///
    /// # Errors
    ///
    /// Returns a host collaborator failure.
    fn anim_property(
        &mut self,
        animatable: &Self::Animatable,
    ) -> Result<Option<AnimProperty>, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimProperty {
    pub is_playing_raw: u8,
    pub has_expired_or_looped_raw: u8,
    pub has_expired_or_looped_by_time: f32,
    pub time_to_expire_or_loop: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WillExpireActivationError<E> {
    Backend(E),
    AnimatableIdentityMismatch,
    MissingAnimatable,
    MissingAnimProperty,
}

impl<E: fmt::Display> fmt::Display for WillExpireActivationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(formatter, "WillExpire collaborator failed: {error}"),
            Self::AnimatableIdentityMismatch => {
                write!(
                    formatter,
                    "cached and generic IAnimatable identities differ"
                )
            }
            Self::MissingAnimatable => write!(formatter, "WillExpire requires IAnimatable"),
            Self::MissingAnimProperty => write!(formatter, "WillExpire requires AnimProperty"),
        }
    }
}

impl<E: Error + 'static> Error for WillExpireActivationError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::AnimatableIdentityMismatch
            | Self::MissingAnimatable
            | Self::MissingAnimProperty => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, convert::Infallible, rc::Rc};

    #[derive(Debug, Eq, PartialEq)]
    struct ActivationSource;

    impl fmt::Display for ActivationSource {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("activation source")
        }
    }

    impl Error for ActivationSource {}

    fn attr(key: &[u8], text: &[u8], value: f32, boolean_raw: u8) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits: value.to_bits(),
            boolean_raw,
        }
    }

    fn has_intent(operator_key: Option<(&[u8], &[u8], f32)>, abs: u8) -> HasIntentCondition {
        let mut attributes = vec![attr(b"intent", b"Push", 0.0, 0)];
        if let Some((key, text, value)) = operator_key {
            attributes.push(attr(key, text, value, 0));
            if key == b"comparison" {
                attributes.push(attr(b"value", b"", value, 0));
            }
        }
        if abs != 0 {
            attributes.push(attr(b"abs", b"true", 0.0, abs));
        }
        HasIntentCondition::from_attributes(&attributes).unwrap()
    }

    #[test]
    fn activation_errors_preserve_only_concrete_backend_sources() {
        let has_intent = HasIntentActivationError::Backend(ActivationSource);
        assert_eq!(
            Error::source(&has_intent).and_then(|source| source.downcast_ref::<ActivationSource>()),
            Some(&ActivationSource)
        );
        let will_expire = WillExpireActivationError::Backend(ActivationSource);
        assert_eq!(
            Error::source(&will_expire)
                .and_then(|source| source.downcast_ref::<ActivationSource>()),
            Some(&ActivationSource)
        );
        assert!(
            Error::source(&HasIntentActivationError::<ActivationSource>::MissingMotionGraph)
                .is_none()
        );
        assert!(
            Error::source(&WillExpireActivationError::<ActivationSource>::MissingAnimProperty)
                .is_none()
        );
    }

    #[test]
    fn storage_offsets_discriminants_and_corpus_oracles_are_frozen() {
        assert_eq!(HAS_INTENT_OBJECT_SIZE, 0x40);
        assert_eq!(
            (
                HAS_INTENT_OP_OFFSET,
                HAS_INTENT_REFERENCE_VALUE_OFFSET,
                HAS_INTENT_ABS_OFFSET,
                HAS_INTENT_NAME_OFFSET
            ),
            (0x1c, 0x20, 0x24, 0x28)
        );
        assert_eq!(
            (
                ComparisonOperator::Nop as i32,
                ComparisonOperator::Equals as i32,
                ComparisonOperator::NotEqual as i32,
                ComparisonOperator::GreaterThan as i32,
                ComparisonOperator::LessThan as i32,
                ComparisonOperator::GreaterEqual as i32,
                ComparisonOperator::LessEqual as i32
            ),
            (0, 1, 2, 3, 4, 5, 6)
        );
        assert_eq!(WILL_EXPIRE_OBJECT_SIZE, 0x48);
        assert_eq!(
            (
                WILL_EXPIRE_TIME_OFFSET,
                WILL_EXPIRE_TIME_TAG_OFFSET,
                WILL_EXPIRE_WAIT_FOR_TRANSITIONS_OFFSET,
                WILL_EXPIRE_TIME_TAG_STORAGE
            ),
            (0x1c, 0x20, 0x45, 37)
        );
        assert_eq!(
            (
                ANIM_PROPERTY_SIZE,
                ANIM_PROPERTY_IS_PLAYING_OFFSET,
                ANIM_PROPERTY_HAS_EXPIRED_OR_LOOPED_OFFSET,
                ANIM_PROPERTY_HAS_EXPIRED_OR_LOOPED_BY_TIME_OFFSET,
                ANIM_PROPERTY_TIME_TO_EXPIRE_OR_LOOP_OFFSET
            ),
            (12, 0, 1, 4, 8)
        );
        assert_eq!(HAS_INTENT_CORPUS_OCCURRENCES, 815);
        assert_eq!(HAS_INTENT_CORPUS_DISTINCT_INTENTS, 118);
        assert_eq!(HAS_INTENT_CORPUS_OP_COUNTS, [800, 0, 0, 9, 4, 1, 1]);
        assert_eq!(
            HAS_INTENT_CORPUS_PRE_COND + HAS_INTENT_CORPUS_ALWAYS_COND,
            815
        );
        assert_eq!(WILL_EXPIRE_CORPUS_OCCURRENCES, 780);
        assert_eq!(
            WILL_EXPIRE_CORPUS_EXACT_IN_TIME + WILL_EXPIRE_CORPUS_WRONG_CASE_IN_TIME,
            780
        );
        assert_eq!(
            WILL_EXPIRE_CORPUS_WAIT_DEFAULT_TRUE + WILL_EXPIRE_CORPUS_WAIT_FALSE,
            780
        );
        assert_eq!(HAS_INTENT_CORPUS_OCCURRENCES_SHA256.len(), 64);
        assert_eq!(WILL_EXPIRE_CORPUS_OCCURRENCES_SHA256.len(), 64);
        assert_eq!(MOTION_LEAF_CLUSTER_ORACLE_SHA256.len(), 64);
    }

    #[test]
    fn leaf_attribute_lookup_is_hash_only_and_first_collision_wins() {
        assert_eq!(state_graph_string_hash32(b"a05763"), 0xfe11_cd43);
        assert_eq!(state_graph_string_hash32(b"a063d3"), 0xfe11_cd43);
        let attributes = [
            attr(b"a05763", b"first", 1.0, 0x80),
            attr(b"a063d3", b"second", 2.0, 0xa5),
        ];
        let found = attribute(&attributes, b"a063d3").unwrap();
        assert_eq!(found.key, b"a05763");
        assert_eq!(found.text, b"first");
        assert_eq!(found.float_bits, 1.0f32.to_bits());
        assert_eq!(found.boolean_raw, 0x80);

        let duplicate_intent = HasIntentCondition::from_attributes(&[
            attr(b"intent", b"first", 0.0, 0),
            attr(b"intent", b"second", 0.0, 0),
        ])
        .unwrap();
        assert_eq!(duplicate_intent.intent_name, b"first");
    }

    #[test]
    fn comparison_constructor_uses_exact_precedence_case_and_defaults() {
        let attributes = vec![
            attr(b"intent", b"Push", 0.0, 0),
            attr(b"comparison", b"LessEqual", 0.0, 0),
            attr(b"value", b"", -0.0, 0),
            attr(b"equal", b"", 99.0, 0),
            attr(b"notEqual", b"", 98.0, 0),
        ];
        let parsed = HasIntentCondition::from_attributes(&attributes).unwrap();
        assert_eq!(parsed.comparison.operator, ComparisonOperator::LessEqual);
        assert_eq!(parsed.comparison.reference_value_bits, (-0.0f32).to_bits());

        let long_forms: [(&[u8], ComparisonOperator); 6] = [
            (b"Equals", ComparisonOperator::Equals),
            (b"NotEqual", ComparisonOperator::NotEqual),
            (b"GreaterThan", ComparisonOperator::GreaterThan),
            (b"LessThan", ComparisonOperator::LessThan),
            (b"GreaterEqual", ComparisonOperator::GreaterEqual),
            (b"LessEqual", ComparisonOperator::LessEqual),
        ];
        for (text, expected) in long_forms {
            let parsed = HasIntentCondition::from_attributes(&[
                attr(b"intent", b"Push", 0.0, 0),
                attr(b"comparison", text, 0.0, 0),
                attr(b"value", b"", 7.0, 0),
            ])
            .unwrap();
            assert_eq!(parsed.comparison.operator, expected, "{text:?}");
            assert_eq!(parsed.comparison.reference_value_bits, 7.0f32.to_bits());
        }

        let keys: [(&[u8], ComparisonOperator); 6] = [
            (b"equal", ComparisonOperator::Equals),
            (b"notEqual", ComparisonOperator::NotEqual),
            (b"greater", ComparisonOperator::GreaterThan),
            (b"less", ComparisonOperator::LessThan),
            (b"greaterEqual", ComparisonOperator::GreaterEqual),
            (b"lessEqual", ComparisonOperator::LessEqual),
        ];
        let values = [0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0];
        for (selected, (key, expected)) in keys.iter().enumerate() {
            let mut attributes = vec![attr(b"intent", b"Push", 0.0, 0)];
            for (index, (candidate, _)) in keys.iter().enumerate().skip(selected) {
                attributes.push(attr(candidate, b"", values[index], 0));
            }
            let parsed = HasIntentCondition::from_attributes(&attributes).unwrap();
            assert_eq!(parsed.comparison.operator, *expected, "{key:?}");
            assert_eq!(
                parsed.comparison.reference_value_bits,
                values[selected].to_bits()
            );
        }

        let default = has_intent(None, 0);
        assert_eq!(default.comparison.operator, ComparisonOperator::Nop);
        assert_eq!(default.comparison.reference_value_bits, 0);
        assert_eq!(default.comparison.absolute_value_raw, 0);
        assert!(!default.comparison.absolute_value());
        assert_eq!(default.intent_name, b"Push");
        let raw_abs = has_intent(None, 0x80).comparison;
        assert_eq!(raw_abs.absolute_value_raw, 0x80);
        assert!(raw_abs.absolute_value());
    }

    #[test]
    fn comparison_long_form_errors_are_checked_in_native_order() {
        let missing_value = vec![
            attr(b"intent", b"Push", 0.0, 0),
            attr(b"comparison", b"bad", 0.0, 0),
        ];
        assert!(matches!(
            HasIntentCondition::from_attributes(&missing_value),
            Err(MotionConditionParseError::MissingAttribute {
                condition: "ComparisonCondition",
                key: b"value"
            })
        ));
        let unknown = vec![
            attr(b"intent", b"Push", 0.0, 0),
            attr(b"comparison", b"equals", 0.0, 0),
            attr(b"value", b"", 1.0, 0),
            attr(b"equal", b"", 2.0, 0),
        ];
        assert_eq!(
            HasIntentCondition::from_attributes(&unknown),
            Err(MotionConditionParseError::UnknownComparison(
                b"equals".to_vec()
            ))
        );
        assert!(matches!(
            HasIntentCondition::from_attributes(&[]),
            Err(MotionConditionParseError::MissingAttribute {
                condition: "HasIntent",
                key: b"intent"
            })
        ));
    }

    #[test]
    fn eval_compare_matches_ppc_ordered_unordered_abs_and_signed_zero() {
        let cases: [(ComparisonOperator, f32, f32, bool); 10] = [
            (ComparisonOperator::Equals, 2.0, 2.0, true),
            (ComparisonOperator::Equals, 2.0, 3.0, false),
            (ComparisonOperator::NotEqual, 2.0, 3.0, true),
            (ComparisonOperator::NotEqual, 2.0, 2.0, false),
            (ComparisonOperator::GreaterThan, 2.0, 3.0, true),
            (ComparisonOperator::GreaterThan, 2.0, 2.0, false),
            (ComparisonOperator::LessThan, 2.0, 1.0, true),
            (ComparisonOperator::LessThan, 2.0, 2.0, false),
            (ComparisonOperator::GreaterEqual, 2.0, 2.0, true),
            (ComparisonOperator::LessEqual, 2.0, 2.0, true),
        ];
        for (operator, reference, value, expected) in cases {
            assert_eq!(
                ComparisonCondition {
                    operator,
                    reference_value_bits: reference.to_bits(),
                    absolute_value_raw: 0
                }
                .evaluate(value),
                expected
            );
        }
        for operator in [
            ComparisonOperator::Equals,
            ComparisonOperator::GreaterThan,
            ComparisonOperator::LessThan,
        ] {
            assert!(
                !ComparisonCondition {
                    operator,
                    reference_value_bits: 0,
                    absolute_value_raw: 0
                }
                .evaluate(f32::NAN)
            );
        }
        for operator in [
            ComparisonOperator::NotEqual,
            ComparisonOperator::GreaterEqual,
            ComparisonOperator::LessEqual,
        ] {
            assert!(
                ComparisonCondition {
                    operator,
                    reference_value_bits: 0,
                    absolute_value_raw: 0
                }
                .evaluate(f32::NAN)
            );
        }
        assert!(
            ComparisonCondition {
                operator: ComparisonOperator::Equals,
                reference_value_bits: (-0.0f32).to_bits(),
                absolute_value_raw: 0
            }
            .evaluate(0.0)
        );
        assert!(
            ComparisonCondition {
                operator: ComparisonOperator::Equals,
                reference_value_bits: 2.0f32.to_bits(),
                absolute_value_raw: 1
            }
            .evaluate(-2.0)
        );
        assert!(
            !ComparisonCondition {
                operator: ComparisonOperator::Nop,
                reference_value_bits: 0,
                absolute_value_raw: 1
            }
            .evaluate(-0.0)
        );
    }

    struct IntentSpy {
        events: Vec<&'static str>,
        cached: Option<u8>,
        found: Option<u8>,
        value: Option<f32>,
    }

    impl HasIntentCollaborator for IntentSpy {
        type Error = Infallible;
        type MotionGraph = u8;
        type Intents = u8;

        fn cached_motion_graph(&mut self) -> Result<Option<u8>, Infallible> {
            self.events.push("cached");
            Ok(self.cached)
        }

        fn find_motion_graph(&mut self) -> Result<Option<u8>, Infallible> {
            self.events.push("find");
            Ok(self.found)
        }

        fn motion_graph_intents(&mut self, motion_graph: &u8) -> Result<u8, Infallible> {
            assert_eq!(*motion_graph, 7);
            self.events.push("intents");
            Ok(11)
        }

        fn lookup_intent(
            &mut self,
            intents: &u8,
            intent_name: &[u8],
        ) -> Result<Option<f32>, Infallible> {
            assert_eq!((*intents, intent_name), (11, b"Push".as_slice()));
            self.events.push("intent");
            Ok(self.value)
        }
    }

    fn intent_spy(value: Option<f32>) -> IntentSpy {
        IntentSpy {
            events: Vec::new(),
            cached: Some(7),
            found: Some(7),
            value,
        }
    }

    #[test]
    fn has_intent_miss_nop_hit_compare_and_call_order_are_exact() {
        let nop = has_intent(None, 0);
        let mut miss = intent_spy(None);
        assert!(!nop.activate(&mut miss).unwrap());
        assert_eq!(miss.events, ["cached", "find", "intents", "intent"]);
        let mut hit = intent_spy(Some(f32::NAN));
        assert!(nop.activate(&mut hit).unwrap());

        let comparisons: [(&[u8], f32, f32, bool); 6] = [
            (b"equal", 2.0, 2.0, true),
            (b"notEqual", 2.0, 3.0, true),
            (b"greater", 2.0, 3.0, true),
            (b"less", 2.0, 1.0, true),
            (b"greaterEqual", 2.0, 2.0, true),
            (b"lessEqual", 2.0, 2.0, true),
        ];
        for (key, reference, value, expected) in comparisons {
            let condition = has_intent(Some((key, b"", reference)), 0);
            let mut hit = intent_spy(Some(value));
            assert_eq!(condition.activate(&mut hit).unwrap(), expected, "{key:?}");
            assert_eq!(hit.events, ["cached", "find", "intents", "intent"]);
        }

        let absolute = has_intent(Some((b"equal", b"", 2.0)), 0x80);
        assert_eq!(absolute.comparison.absolute_value_raw, 0x80);
        let mut negative = intent_spy(Some(-2.0));
        assert!(absolute.activate(&mut negative).unwrap());

        let greater_equal = has_intent(Some((b"greaterEqual", b"", 2.0)), 0);
        let mut nan = intent_spy(Some(f32::NAN));
        assert!(greater_equal.activate(&mut nan).unwrap());
        assert_eq!(nan.events, ["cached", "find", "intents", "intent"]);
    }

    #[test]
    fn has_intent_identity_and_nonnull_assumptions_are_typed() {
        let condition = has_intent(None, 0);
        let mut mismatch = IntentSpy {
            events: Vec::new(),
            cached: Some(7),
            found: Some(8),
            value: Some(1.0),
        };
        assert_eq!(
            condition.activate(&mut mismatch),
            Err(HasIntentActivationError::MotionGraphIdentityMismatch)
        );
        assert_eq!(mismatch.events, ["cached", "find"]);
        let mut missing = IntentSpy {
            events: Vec::new(),
            cached: None,
            found: None,
            value: Some(1.0),
        };
        assert_eq!(
            condition.activate(&mut missing),
            Err(HasIntentActivationError::MissingMotionGraph)
        );
        assert_eq!(missing.events, ["cached", "find"]);
    }

    #[test]
    fn will_expire_constructor_is_case_sensitive_and_preserves_bits() {
        let wrong_case = WillExpireCondition::from_attributes(&[
            attr(b"inTime", b"1", 1.0, 0),
            attr(b"waitforTransitions", b"false", 0.0, 0),
        ])
        .unwrap();
        assert_eq!(wrong_case.time_bits, 0);
        assert_eq!(wrong_case.wait_for_transitions_raw, 1);
        assert!(wrong_case.wait_for_transitions());
        assert_eq!(wrong_case.time_tag(), b"");

        let exact = WillExpireCondition::from_attributes(&[
            attr(b"InTime", b"-0", -0.0, 0),
            attr(b"waitForTransitions", b"false", 0.0, 0xa5),
            attr(b"InTimeTag", b"Tag", 0.0, 0),
        ])
        .unwrap();
        assert_eq!(exact.time_bits, (-0.0f32).to_bits());
        assert_eq!(exact.wait_for_transitions_raw, 0xa5);
        assert!(exact.wait_for_transitions());
        assert_eq!(exact.time_tag(), b"Tag");
    }

    #[test]
    fn will_expire_tag_bound_keeps_36_bytes_plus_nul() {
        let accepted = vec![b'a'; 36];
        let parsed =
            WillExpireCondition::from_attributes(&[attr(b"InTimeTag", &accepted, 0.0, 0)]).unwrap();
        assert_eq!(parsed.time_tag(), accepted);
        assert_eq!(parsed.time_tag().len() + 1, WILL_EXPIRE_TIME_TAG_STORAGE);
        assert_eq!(
            WillExpireCondition::from_attributes(&[attr(b"InTimeTag", &[b'a'; 37], 0.0, 0)]),
            Err(MotionConditionParseError::TimeTagTooLong {
                length: 37,
                maximum: 36
            })
        );
        assert_eq!(
            WillExpireCondition::from_attributes(&[attr(b"InTimeTag", b"A\0B", 0.0, 0)]),
            Err(MotionConditionParseError::TimeTagContainsNul)
        );
    }

    struct WillSpy {
        events: Rc<RefCell<Vec<&'static str>>>,
        info: Option<u8>,
        resolved: f32,
        cached: Option<u8>,
        found: Option<u8>,
        transitioning: bool,
        property: Option<AnimProperty>,
    }

    impl WillExpireCollaborator for WillSpy {
        type Error = Infallible;
        type AnimInfo = u8;
        type Animatable = u8;

        fn find_anim_info(&mut self) -> Result<Option<u8>, Infallible> {
            self.events.borrow_mut().push("find_tag");
            Ok(self.info)
        }

        fn resolve_special_tag(&mut self, info: &u8, tag: &[u8]) -> Result<f32, Infallible> {
            assert_eq!((*info, tag), (3, b"Tag".as_slice()));
            self.events.borrow_mut().push("resolve_tag");
            Ok(self.resolved)
        }

        fn cached_animatable(&mut self) -> Result<Option<u8>, Infallible> {
            self.events.borrow_mut().push("cached_anim");
            Ok(self.cached)
        }

        fn find_animatable(&mut self) -> Result<Option<u8>, Infallible> {
            self.events.borrow_mut().push("find_anim");
            Ok(self.found)
        }

        fn transitioning(&mut self, animatable: &u8) -> Result<bool, Infallible> {
            assert_eq!(*animatable, 9);
            self.events.borrow_mut().push("transitioning");
            Ok(self.transitioning)
        }

        fn anim_property(&mut self, animatable: &u8) -> Result<Option<AnimProperty>, Infallible> {
            assert_eq!(*animatable, 9);
            self.events.borrow_mut().push("property");
            Ok(self.property)
        }
    }

    fn property(expired: u8, time: f32) -> AnimProperty {
        AnimProperty {
            is_playing_raw: 0,
            has_expired_or_looped_raw: expired,
            has_expired_or_looped_by_time: 0.0,
            time_to_expire_or_loop: time,
        }
    }

    fn will_attributes(
        time: f32,
        tag: Option<&[u8]>,
        wait: bool,
    ) -> Vec<StateGraphBinaryAttribute> {
        let mut attributes = vec![
            attr(b"InTime", b"", time, 0),
            attr(
                b"waitForTransitions",
                if wait { b"true" } else { b"false" },
                0.0,
                u8::from(wait),
            ),
        ];
        if let Some(tag) = tag {
            attributes.push(attr(b"InTimeTag", tag, 0.0, 0));
        }
        attributes
    }

    fn spy(
        events: &Rc<RefCell<Vec<&'static str>>>,
        info: Option<u8>,
        resolved: f32,
        transitioning: bool,
        property: Option<AnimProperty>,
    ) -> WillSpy {
        WillSpy {
            events: Rc::clone(events),
            info,
            resolved,
            cached: Some(9),
            found: Some(9),
            transitioning,
            property,
        }
    }

    #[test]
    fn will_expire_empty_tag_skips_resolver_and_orders_animatable_calls() {
        let condition =
            WillExpireCondition::from_attributes(&will_attributes(2.0, None, true)).unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut backend = spy(&events, Some(3), 99.0, false, Some(property(0, 2.0)));
        assert!(condition.activate(&mut backend).unwrap());
        assert_eq!(
            *events.borrow(),
            ["cached_anim", "find_anim", "transitioning", "property"]
        );
    }

    #[test]
    fn will_expire_tag_override_and_missing_interface_preserve_sequence() {
        let condition =
            WillExpireCondition::from_attributes(&will_attributes(2.0, Some(b"Tag"), false))
                .unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut backend = spy(&events, Some(3), 4.0, true, Some(property(0, 3.0)));
        assert!(condition.activate(&mut backend).unwrap());
        assert_eq!(
            *events.borrow(),
            [
                "find_tag",
                "resolve_tag",
                "cached_anim",
                "find_anim",
                "transitioning",
                "property"
            ]
        );

        events.borrow_mut().clear();
        let mut backend = spy(&events, None, 99.0, false, Some(property(0, 3.0)));
        assert!(!condition.activate(&mut backend).unwrap());
        assert_eq!(
            *events.borrow(),
            [
                "find_tag",
                "cached_anim",
                "find_anim",
                "transitioning",
                "property"
            ]
        );
    }

    #[test]
    fn will_expire_transitioning_is_unconditional_but_wait_gates_property() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let wait = WillExpireCondition::from_attributes(&[
            attr(b"InTime", b"", 1.0, 0),
            attr(b"waitForTransitions", b"true", 0.0, 0xa5),
        ])
        .unwrap();
        assert_eq!(wait.wait_for_transitions_raw, 0xa5);
        let mut backend = spy(&events, None, 0.0, true, Some(property(1, 0.0)));
        assert!(!wait.activate(&mut backend).unwrap());
        assert_eq!(
            *events.borrow(),
            ["cached_anim", "find_anim", "transitioning"]
        );

        events.borrow_mut().clear();
        let no_wait =
            WillExpireCondition::from_attributes(&will_attributes(1.0, None, false)).unwrap();
        let mut backend = spy(&events, None, 0.0, true, Some(property(1, 99.0)));
        assert!(no_wait.activate(&mut backend).unwrap());
        assert_eq!(
            *events.borrow(),
            ["cached_anim", "find_anim", "transitioning", "property"]
        );
    }

    #[test]
    fn will_expire_ppc_comparison_and_unused_fields_are_exact() {
        let condition =
            WillExpireCondition::from_attributes(&will_attributes(2.0, None, false)).unwrap();
        for (time, expected) in [(1.0, true), (2.0, true), (3.0, false), (f32::NAN, false)] {
            for is_playing_raw in [0, 0xff] {
                for by_time in [f32::NEG_INFINITY, f32::NAN, f32::INFINITY] {
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let mut value = property(0, time);
                    value.is_playing_raw = is_playing_raw;
                    value.has_expired_or_looped_by_time = by_time;
                    let mut backend = spy(&events, None, 0.0, false, Some(value));
                    assert_eq!(condition.activate(&mut backend).unwrap(), expected);
                    assert_eq!(
                        *events.borrow(),
                        ["cached_anim", "find_anim", "transitioning", "property"]
                    );
                }
            }
        }
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut backend = spy(&events, None, 0.0, false, Some(property(0x80, f32::NAN)));
        assert!(condition.activate(&mut backend).unwrap());

        let unordered_threshold =
            WillExpireCondition::from_attributes(&will_attributes(f32::NAN, None, false)).unwrap();
        events.borrow_mut().clear();
        let mut backend = spy(&events, None, 0.0, false, Some(property(0, 99.0)));
        assert!(!unordered_threshold.activate(&mut backend).unwrap());
        assert_eq!(
            *events.borrow(),
            ["cached_anim", "find_anim", "transitioning", "property"]
        );
    }

    #[test]
    fn will_expire_native_pointer_assumptions_are_checked_errors() {
        let condition =
            WillExpireCondition::from_attributes(&will_attributes(1.0, None, false)).unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut mismatch = spy(&events, None, 0.0, false, Some(property(0, 0.0)));
        mismatch.found = Some(8);
        assert_eq!(
            condition.activate(&mut mismatch),
            Err(WillExpireActivationError::AnimatableIdentityMismatch)
        );
        assert_eq!(*events.borrow(), ["cached_anim", "find_anim"]);

        events.borrow_mut().clear();
        let mut missing = spy(&events, None, 0.0, false, Some(property(0, 0.0)));
        missing.cached = None;
        missing.found = None;
        assert_eq!(
            condition.activate(&mut missing),
            Err(WillExpireActivationError::MissingAnimatable)
        );
        assert_eq!(*events.borrow(), ["cached_anim", "find_anim"]);

        events.borrow_mut().clear();
        let mut missing_property = spy(&events, None, 0.0, false, None);
        assert_eq!(
            condition.activate(&mut missing_property),
            Err(WillExpireActivationError::MissingAnimProperty)
        );
        assert_eq!(
            *events.borrow(),
            ["cached_anim", "find_anim", "transitioning", "property"]
        );
    }
}
