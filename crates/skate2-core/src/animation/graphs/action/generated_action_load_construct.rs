//! Source constructor families for every generated Action declaration.
use super::{Attribute, BehaviourClass, ConditionClass, GeneratedActionLoadError, Section, data};
use crate::play_animation::{FastString30, FastString36};
use skate2_formats::andale::PackedFastString;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct BaseObject {
    pub(crate) name: Arc<str>,
    pub(crate) path: Arc<str>,
    // IndexToState829282c0 uses complemented index tags, with FFFF as NULL.
    // Direct condition wrappers mutate the same shared factory object's parent.
    pub(crate) parent: std::sync::Mutex<Option<u16>>,
    pub(crate) enabled: bool,
    pub(crate) cast_parent: bool,
}
impl BaseObject {
    /// Base82cf6f88: generated factories pass NULL parent; name/path share
    /// pooled storage. Ref-count ownership is represented by the containing Arc.
    pub(crate) fn new(name: Arc<str>) -> Self {
        Self {
            path: Arc::clone(&name),
            name,
            parent: std::sync::Mutex::new(None),
            enabled: true,
            cast_parent: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Filter {
    None,
    Negate,
    Abs,
    OneMinus,
    Clamp,
    AngleFlip,
    AngleRot90,
    AngleRotN90,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GestureGroup {
    Square,
    Nose,
    Tail,
    Nose90,
    Tail90,
    NoseN90,
    TailN90,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConditionMask {
    Precondition,
    Sustain,
    Always,
    Postcondition,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComparisonOp {
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    Nop,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Comparison {
    pub(crate) op: ComparisonOp,
    pub(crate) absolute: bool,
    pub(crate) reference: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilteredState {
    Ground,
    Air,
    Grind,
    Wipeout,
    Teleport,
    Invalid,
    Offboard,
    OffboardAir,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandPlantState {
    Air,
    Ground,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandPlantDirection {
    None,
    Frontside,
    Backside,
}

#[derive(Debug)]
pub(crate) enum BehaviourPayload {
    JuiceHook,
    BodyFlippingSignal,
    PrintText {
        x: i32,
        y: i32,
        frames: i32,
        text: Box<[u8; 1024]>,
    },
    ConstIntent {
        motion: FastString36,
        value: f32,
        on_update: bool,
    },
    TimeIntent {
        motion: FastString36,
        action: FastString36,
    },
    TransferIntent {
        motion: FastString36,
        action: FastString36,
        default_value: f32,
        has_default: bool,
        scale: f32,
        first: Filter,
        second: Filter,
        fakie: Filter,
        mirror: Filter,
        on_update: bool,
    },
    GestureIntent {
        group: GestureGroup,
        has_override: bool,
        override_name: FastString36,
    },
    BoardAdjust {
        magnitude: FastString36,
        angle: FastString36,
        filter: Filter,
        negate_on_mirror: bool,
    },
}
#[derive(Debug)]
pub(crate) struct BehaviourObject {
    pub(crate) base: BaseObject,
    pub(crate) class: BehaviourClass,
    pub(crate) payload: BehaviourPayload,
}
#[derive(Debug)]
pub(crate) enum ConditionPayload {
    Plain,
    Comparison(Comparison),
    Gesture(GestureGroup),
    PhysicalState(FilteredState),
    Speed {
        comparison: Comparison,
        along_skate_z: bool,
    },
    HandPlant {
        state: HandPlantState,
        direction: HandPlantDirection,
    },
    AnimationAttribute {
        comparison: Comparison,
        attribute: FastString30,
        sequence: i32,
    },
    CurrentState(String),
    ActionIntent {
        comparison: Comparison,
        intent: FastString36,
    },
    Grinding {
        has_specific: bool,
        name: FastString30,
    },
}
#[derive(Debug)]
pub(crate) struct ConditionObject {
    pub(crate) base: BaseObject,
    pub(crate) mask: ConditionMask,
    pub(crate) class: ConditionClass,
    pub(crate) payload: ConditionPayload,
}

struct Attributes {
    name: &'static str,
    values: &'static [Attribute],
}
impl Attributes {
    fn new(name: &'static str, section: Section) -> Result<Self, GeneratedActionLoadError> {
        if section.count == 0 {
            return Ok(Self { name, values: &[] });
        }
        let start = usize::from(section.index);
        let end = start + usize::from(section.count);
        let values = data::ATTRIBUTES
            .get(start..end)
            .ok_or(GeneratedActionLoadError::DescriptorBounds)?;
        Ok(Self { name, values })
    }
    /// `GenAttributeMap828f8cf8` returns synthetic name/enabled before the table.
    fn get(&self, key: &str) -> Option<&'static str> {
        match key {
            "name" => Some(self.name),
            "enabled" => Some("true"),
            _ => self
                .values
                .iter()
                .find(|attribute| attribute.key == key)
                .map(|attribute| attribute.value),
        }
    }
    fn has(&self, key: &str) -> bool {
        matches!(key, "name" | "enabled")
            || self.values.iter().any(|attribute| attribute.key == key)
    }
    fn required(&self, key: &'static str) -> Result<&'static str, GeneratedActionLoadError> {
        self.get(key)
            .ok_or(GeneratedActionLoadError::MissingAttribute(key))
    }
    fn boolean(&self, key: &'static str, default: bool) -> Result<bool, GeneratedActionLoadError> {
        match self.get(key) {
            None => Ok(default),
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            Some(value) => Err(GeneratedActionLoadError::InvalidAttribute(
                key,
                value.to_owned(),
            )),
        }
    }
    fn float(&self, key: &'static str, default: f32) -> Result<f32, GeneratedActionLoadError> {
        self.get(key)
            .map_or(Ok(default), |value| decimal(key, value))
    }
    fn required_float(&self, key: &'static str) -> Result<f32, GeneratedActionLoadError> {
        decimal(key, self.required(key)?)
    }
    fn filter(&self, key: &str) -> Result<Filter, GeneratedActionLoadError> {
        parse_filter(self.get(key).unwrap_or("none"))
    }
    fn fast<const N: usize>(
        &self,
        key: &'static str,
    ) -> Result<PackedFastString<N>, GeneratedActionLoadError> {
        packed(key, self.required(key)?)
    }
}
/// `GenAttributeMap` `GetFloat828f9280` / default overload828f9518 use CRT atof
/// (double) then frsp. Only finite decimal literals present in the generated
/// table are admitted; broader CRT grammar is a checked unsupported boundary.
#[expect(
    clippy::cast_possible_truncation,
    reason = "native atof double then frsp"
)]
fn decimal(key: &'static str, text: &str) -> Result<f32, GeneratedActionLoadError> {
    let unsigned = text
        .strip_prefix('-')
        .or_else(|| text.strip_prefix('+'))
        .unwrap_or(text);
    if unsigned.is_empty()
        || unsigned
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && byte != b'.')
        || unsigned.bytes().filter(|&byte| byte == b'.').count() > 1
        || !unsigned.bytes().any(|byte| byte.is_ascii_digit())
    {
        return Err(GeneratedActionLoadError::InvalidAttribute(
            key,
            text.to_owned(),
        ));
    }
    let value = text
        .parse::<f64>()
        .map_err(|_| GeneratedActionLoadError::InvalidAttribute(key, text.to_owned()))?
        as f32;
    if !value.is_finite() {
        return Err(GeneratedActionLoadError::InvalidAttribute(
            key,
            text.to_owned(),
        ));
    }
    Ok(value)
}
fn packed<const N: usize>(
    key: &'static str,
    text: &str,
) -> Result<PackedFastString<N>, GeneratedActionLoadError> {
    PackedFastString::encode(text)
        .map_err(|_| GeneratedActionLoadError::InvalidAttribute(key, text.to_owned()))
}
/// `ParseFilter82cb9c98` compares packed, case-folded `FastString30` identities.
pub(crate) fn parse_filter(text: &str) -> Result<Filter, GeneratedActionLoadError> {
    let value: FastString30 = packed("filter", text)?;
    for (name, filter) in [
        ("negate", Filter::Negate),
        ("abs", Filter::Abs),
        ("oneMinus", Filter::OneMinus),
        ("clamp", Filter::Clamp),
        ("angleFlip", Filter::AngleFlip),
        ("angleRot90", Filter::AngleRot90),
        ("angleRotN90", Filter::AngleRotN90),
    ] {
        if value == packed::<5>("filter", name)? {
            return Ok(filter);
        }
    }
    Ok(Filter::None)
}
fn gesture(text: &str) -> Result<GestureGroup, GeneratedActionLoadError> {
    match text {
        "Square" => Ok(GestureGroup::Square),
        "Nose" => Ok(GestureGroup::Nose),
        "Tail" => Ok(GestureGroup::Tail),
        "90Nose" => Ok(GestureGroup::Nose90),
        "90Tail" => Ok(GestureGroup::Tail90),
        "N90Nose" => Ok(GestureGroup::NoseN90),
        "N90Tail" => Ok(GestureGroup::TailN90),
        _ => Err(GeneratedActionLoadError::InvalidAttribute(
            "group",
            text.to_owned(),
        )),
    }
}
pub(crate) fn mask(text: Option<&str>) -> Result<ConditionMask, GeneratedActionLoadError> {
    match text {
        None | Some("precond") => Ok(ConditionMask::Precondition),
        Some("sustain") => Ok(ConditionMask::Sustain),
        Some("always") => Ok(ConditionMask::Always),
        Some("postcond") => Ok(ConditionMask::Postcondition),
        Some(value) => Err(GeneratedActionLoadError::InvalidAttribute(
            "mask",
            value.to_owned(),
        )),
    }
}
/// `ComparisonCondition82cf7ae8` preserves the source attribute precedence.
pub(crate) fn comparison_from_attributes(
    name: &'static str,
    values: &'static [Attribute],
) -> Result<Comparison, GeneratedActionLoadError> {
    comparison(&Attributes { name, values })
}
fn comparison(attributes: &Attributes) -> Result<Comparison, GeneratedActionLoadError> {
    comparison_from_lookup(attributes)
}
/// Shared actual `ComparisonCondition` field constructor; generated and mounted
/// binary maps retain their own lookup/ownership and feed the same read order.
pub(crate) fn comparison_from_lookup(
    attributes: &impl ConstructorAttributes,
) -> Result<Comparison, GeneratedActionLoadError> {
    let mut result = Comparison {
        op: ComparisonOp::Equal,
        absolute: false,
        reference: 0.0,
    };
    if let Some(text) = attributes.get("comparison") {
        result.reference = attributes.required_float("value")?;
        result.op = match text {
            "Equals" => ComparisonOp::Equal,
            "NotEqual" => ComparisonOp::NotEqual,
            "GreaterThan" => ComparisonOp::Greater,
            "LessThan" => ComparisonOp::Less,
            "GreaterEqual" => ComparisonOp::GreaterEqual,
            "LessEqual" => ComparisonOp::LessEqual,
            _ => {
                return Err(GeneratedActionLoadError::InvalidAttribute(
                    "comparison",
                    text.to_owned(),
                ));
            }
        };
    } else {
        result.op = ComparisonOp::Nop;
        for (key, op) in [
            ("equal", ComparisonOp::Equal),
            ("notEqual", ComparisonOp::NotEqual),
            ("greater", ComparisonOp::Greater),
            ("less", ComparisonOp::Less),
            ("greaterEqual", ComparisonOp::GreaterEqual),
            ("lessEqual", ComparisonOp::LessEqual),
        ] {
            if attributes.has(key) {
                result.op = op;
                result.reference = attributes.required_float(key)?;
                break;
            }
        }
    }
    result.absolute = attributes.boolean("abs", false)?;
    Ok(result)
}
/// Native attribute-map operations: binary maps retain their serialized
/// float/bool values; generated maps keep their existing text conversions.
pub(crate) trait ConstructorAttributes {
    fn get(&self, key: &str) -> Option<&str>;
    fn required_float(&self, key: &'static str) -> Result<f32, GeneratedActionLoadError>;
    fn boolean(&self, key: &'static str, default: bool) -> Result<bool, GeneratedActionLoadError>;
    fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
    fn required(&self, key: &'static str) -> Result<&str, GeneratedActionLoadError> {
        self.get(key)
            .ok_or(GeneratedActionLoadError::MissingAttribute(key))
    }
}
impl ConstructorAttributes for Attributes {
    fn get(&self, key: &str) -> Option<&str> {
        self.get(key)
    }
    fn required_float(&self, key: &'static str) -> Result<f32, GeneratedActionLoadError> {
        self.required_float(key)
    }
    fn boolean(&self, key: &'static str, default: bool) -> Result<bool, GeneratedActionLoadError> {
        self.boolean(key, default)
    }
}
/// Shared `PrintText2D` factory82c8fc08 constructor, including x/y/frames read order.
pub(crate) fn print_text_from_lookup(
    attributes: &impl ConstructorAttributes,
) -> Result<BehaviourPayload, GeneratedActionLoadError> {
    let x = integer(attributes.required_float("x")?)?;
    let y = integer(attributes.required_float("y")?)?;
    let frames = integer(attributes.required_float("numFrames")?)?;
    let source = attributes.required("text")?.as_bytes();
    let mut text = Box::new([0; 1024]);
    let count = source.len().min(1024);
    text[..count].copy_from_slice(&source[..count]);
    Ok(BehaviourPayload::PrintText { x, y, frames, text })
}
/// Selected generated values are finite and in i32 range. Native fctiwz
/// overflow/NaN is kept as an explicit checked boundary here.
#[expect(
    clippy::cast_possible_truncation,
    reason = "source fctiwz truncates toward zero"
)]
fn integer(value: f32) -> Result<i32, GeneratedActionLoadError> {
    if !value.is_finite() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        return Err(GeneratedActionLoadError::InvalidAttribute(
            "integer",
            value.to_string(),
        ));
    }
    Ok(value as i32)
}
pub(super) fn behaviour(
    class: BehaviourClass,
    name: Arc<str>,
    section: Section,
) -> Result<BehaviourObject, GeneratedActionLoadError> {
    let attributes = Attributes::new(class.binding().name, section)?;
    behaviour_from_attributes(class, name, attributes.values)
}
pub(crate) fn behaviour_from_attributes(
    class: BehaviourClass,
    name: Arc<str>,
    values: &'static [Attribute],
) -> Result<BehaviourObject, GeneratedActionLoadError> {
    let base = BaseObject::new(name);
    let attributes = Attributes {
        name: class.binding().name,
        values,
    };
    let payload = match class {
        BehaviourClass::JuiceHook => BehaviourPayload::JuiceHook,
        BehaviourClass::BodyFlippingSignal => BehaviourPayload::BodyFlippingSignal,
        BehaviourClass::PrintText2D => print_text_from_lookup(&attributes)?,
        BehaviourClass::CreateConstMGIntent => BehaviourPayload::ConstIntent {
            motion: attributes.fast("MGIntent")?,
            value: attributes.float("value", 0.0)?,
            on_update: attributes.boolean("onUpdate", true)?,
        },
        BehaviourClass::CreateMGTimeIntentFromAGIntent => {
            attributes.required("MGIntent")?;
            attributes.required("AGIntent")?;
            BehaviourPayload::TimeIntent {
                motion: attributes.fast("MGIntent")?,
                action: attributes.fast("AGIntent")?,
            }
        }
        BehaviourClass::CreateMGIntentFromAGIntent => {
            attributes.required("MGIntent")?;
            attributes.required("AGIntent")?;
            let motion = attributes.fast("MGIntent")?;
            let action = attributes.fast("AGIntent")?;
            let has_default = attributes.has("defaultValue");
            let default_value = attributes.float("defaultValue", 0.0)?;
            let scale = attributes.float("scale", 1.0)?;
            let mut first = attributes.filter("filter")?;
            if attributes.has("filter1") {
                first = attributes.filter("filter1")?;
            }
            let second = attributes.filter("filter2")?;
            let fakie = attributes.filter("fakieFilter")?;
            let mirror = attributes.filter("mirrorFilter")?;
            let on_update = attributes.boolean("onUpdate", true)?;
            BehaviourPayload::TransferIntent {
                motion,
                action,
                default_value,
                has_default,
                scale,
                first,
                second,
                fakie,
                mirror,
                on_update,
            }
        }
        BehaviourClass::CreateTrickIntentFromGesture => BehaviourPayload::GestureIntent {
            group: gesture(attributes.required("group")?)?,
            has_override: attributes.has("override"),
            override_name: packed("override", attributes.get("override").unwrap_or(""))?,
        },
        BehaviourClass::BoardAdjust => {
            let negate_on_mirror = attributes.boolean("negateOnMirror", true)?;
            attributes.required("MGIntentMag")?;
            attributes.required("MGIntentAngle")?;
            let magnitude = attributes.fast("MGIntentMag")?;
            let angle = attributes.fast("MGIntentAngle")?;
            let filter = attributes.filter("angleFilter")?;
            if !matches!(
                filter,
                Filter::None | Filter::AngleFlip | Filter::AngleRot90 | Filter::AngleRotN90
            ) {
                return Err(GeneratedActionLoadError::InvalidAttribute(
                    "angleFilter",
                    attributes.required("angleFilter")?.to_owned(),
                ));
            }
            BehaviourPayload::BoardAdjust {
                magnitude,
                angle,
                filter,
                negate_on_mirror,
            }
        }
    };
    Ok(BehaviourObject {
        base,
        class,
        payload,
    })
}
pub(super) fn condition(
    class: ConditionClass,
    name: Arc<str>,
    section: Section,
) -> Result<ConditionObject, GeneratedActionLoadError> {
    let attributes = Attributes::new(class.binding().name, section)?;
    condition_from_attributes(class, name, attributes.values)
}
pub(crate) fn condition_from_attributes(
    class: ConditionClass,
    name: Arc<str>,
    values: &'static [Attribute],
) -> Result<ConditionObject, GeneratedActionLoadError> {
    let base = BaseObject::new(name);
    let attributes = Attributes {
        name: class.binding().name,
        values,
    };
    let mask = mask(attributes.get("mask"))?;
    let payload = match class {
        ConditionClass::PhysicsRequestsDismount
        | ConditionClass::IsLandingOnBoard
        | ConditionClass::IsInLocomotion
        | ConditionClass::PhysicsWantsRunout
        | ConditionClass::DisablePushBrake
        | ConditionClass::IsMirrored
        | ConditionClass::IsRidingFakie
        | ConditionClass::IsGrabbingObject
        | ConditionClass::IsFootPlanting => ConditionPayload::Plain,
        ConditionClass::TimeSinceLastInput
        | ConditionClass::PhysicsSpeedAndSlopeCompare
        | ConditionClass::InParentStateForTime
        | ConditionClass::IsDismounting
        | ConditionClass::OBTimeToLand
        | ConditionClass::TimeToLand
        | ConditionClass::IsDoneAirDismount => ConditionPayload::Comparison(
            comparison_from_attributes(attributes.name, attributes.values)?,
        ),
        ConditionClass::HasGestureIntent => {
            ConditionPayload::Gesture(gesture(attributes.required("group")?)?)
        }
        ConditionClass::PhysFilteredState => {
            let state = match attributes.get("state").unwrap_or("") {
                "ground" => FilteredState::Ground,
                "air" => FilteredState::Air,
                "grind" => FilteredState::Grind,
                "wipeout" => FilteredState::Wipeout,
                "teleport" => FilteredState::Teleport,
                "invalid" => FilteredState::Invalid,
                "offboard" => FilteredState::Offboard,
                "offboardair" => FilteredState::OffboardAir,
                value => {
                    return Err(GeneratedActionLoadError::InvalidAttribute(
                        "state",
                        value.to_owned(),
                    ));
                }
            };
            ConditionPayload::PhysicalState(state)
        }
        ConditionClass::PhysicsSpeedCompare => ConditionPayload::Speed {
            comparison: comparison_from_attributes(attributes.name, attributes.values)?,
            along_skate_z: attributes.boolean("alongSkateZ", false)?,
        },
        ConditionClass::IsHandPlanting => {
            let state = match attributes.get("state").unwrap_or("") {
                "air" => HandPlantState::Air,
                "ground" => HandPlantState::Ground,
                value => {
                    return Err(GeneratedActionLoadError::InvalidAttribute(
                        "state",
                        value.to_owned(),
                    ));
                }
            };
            let direction = match attributes.get("dir").unwrap_or("") {
                "FS" => HandPlantDirection::Frontside,
                "BS" => HandPlantDirection::Backside,
                _ => HandPlantDirection::None,
            };
            ConditionPayload::HandPlant { state, direction }
        }
        ConditionClass::HasAnimAttribute => ConditionPayload::AnimationAttribute {
            comparison: comparison_from_attributes(attributes.name, attributes.values)?,
            attribute: attributes.fast("attribute")?,
            sequence: integer(attributes.float("sequenceid", -1.0)?)?,
        },
        ConditionClass::CurrentState => {
            ConditionPayload::CurrentState(attributes.required("state")?.to_owned())
        }
        ConditionClass::HasAGIntent => ConditionPayload::ActionIntent {
            comparison: comparison_from_attributes(attributes.name, attributes.values)?,
            intent: attributes.fast("intent")?,
        },
        ConditionClass::IsGrinding => ConditionPayload::Grinding {
            has_specific: attributes.has("grindName"),
            name: packed("grindName", attributes.get("grindName").unwrap_or(""))?,
        },
    };
    Ok(ConditionObject {
        base,
        mask,
        class,
        payload,
    })
}
