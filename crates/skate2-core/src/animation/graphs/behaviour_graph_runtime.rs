//! Typed unprocessed construction snapshots for the ten shipped living-world
//! `BehaviourGraph`s.
//!
//! The wire decoder remains in `skate2-formats`. This layer reconstructs the
//! proven final plugin/expression/condition object tree. It deliberately stops
//! before `TransferableBehaviour::Process`, controller-wrapper creation,
//! generated/dynamic controller bodies, and condition activation functions.
//! It also does not expose native parser-stack or partial-construction timing.

use skate2_formats::{
    behaviour::{BehaviourBinaryAttribute, BehaviourBinaryElement},
    state_graph::state_graph_string_hash32,
};
use std::{error::Error, fmt};

const MAX_WIRE_ELEMENT_DEPTH: usize = 1_024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferableBehaviourClass {
    ActorTrackerSpectate,
    LookAt,
    SpectatorWaypointGeneric,
    UseWorldProp,
    PedestrianConversation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferableBehaviourGraph {
    pub root: TransferableBehaviour,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferableBehaviour {
    pub class: TransferableBehaviourClass,
    pub file: String,
    pub maximum_participants: u32,
    pub expression: TransferExpression,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferOperator {
    None,
    And,
    Or,
    Not,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferExpression {
    pub operator: TransferOperator,
    pub children: Vec<TransferExpressionChild>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferExpressionChild {
    Expression(TransferExpression),
    Condition(TransferCondition),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeOptionalF32 {
    Written(u32),
    Unwritten,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferCondition {
    IsNearCrossWalk { distance: NativeOptionalF32 },
    IsOnRoad,
    CanRecognizePlayer { maximum_recognizers: u32 },
    HasLineOfSightToProp,
    NearAvailableWaypoint { distance_bits: u32 },
    HasFirstWaypointAvailable,
    PluginTransferOdds { percentage_bits: u32 },
    InFrontOfFirstWaypoint,
    HasHandProp,
    HasDisposableHandProp,
    CanSitWithHandProp,
    IsChasing,
    EntityAllowedToJoinConversation,
    ConversationIsGatheringMembers,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BehaviourGraphRuntimeError {
    UnexpectedElement {
        name: String,
        context: &'static str,
    },
    MissingPlugin,
    MultiplePlugins,
    MissingAttribute {
        element: String,
        attribute: &'static str,
    },
    UnknownPlugin(String),
    UnknownCondition(String),
    MissingExpression,
    MultipleExpressions,
    ConditionHasChildren {
        name: String,
    },
    UnknownOperator(String),
    NestingDepth {
        depth: usize,
        maximum: usize,
    },
    Allocation,
    NumericRange {
        element: String,
        attribute: &'static str,
        bits: u32,
    },
}

impl fmt::Display for BehaviourGraphRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedElement { name, context } => {
                write!(
                    formatter,
                    "unexpected BehaviourGraph element {name:?} under {context}"
                )
            }
            Self::MissingPlugin => formatter.write_str("BehaviourSource has no BehaviourPlugin"),
            Self::MultiplePlugins => {
                formatter.write_str("BehaviourSource has more than one BehaviourPlugin")
            }
            Self::MissingAttribute { element, attribute } => {
                write!(
                    formatter,
                    "BehaviourGraph {element} is missing {attribute:?}"
                )
            }
            Self::UnknownPlugin(name) => write!(formatter, "unknown BehaviourPlugin {name:?}"),
            Self::UnknownCondition(name) => write!(formatter, "unknown TransferCondition {name:?}"),
            Self::MissingExpression => {
                formatter.write_str("BehaviourPlugin has no TransferExpression")
            }
            Self::MultipleExpressions => {
                formatter.write_str("BehaviourPlugin has more than one TransferExpression")
            }
            Self::ConditionHasChildren { name } => {
                write!(formatter, "TransferCondition {name:?} has child elements")
            }
            Self::UnknownOperator(operator) => {
                write!(
                    formatter,
                    "unknown nested TransferExpression operator {operator:?}"
                )
            }
            Self::NestingDepth { depth, maximum } => write!(
                formatter,
                "BehaviourGraph element depth {depth} exceeds checked maximum {maximum}"
            ),
            Self::Allocation => {
                formatter.write_str("could not allocate BehaviourGraph expression children")
            }
            Self::NumericRange {
                element,
                attribute,
                bits,
            } => write!(
                formatter,
                "BehaviourGraph {element} attribute {attribute:?} with f32 bits {bits:08x} is outside the checked integer-conversion domain"
            ),
        }
    }
}

impl Error for BehaviourGraphRuntimeError {}

impl TransferableBehaviourGraph {
    /// Constructs the exact final object-tree subset used by the ten shipped
    /// living-world descriptors.
    ///
    /// Native malformed/factory-miss behavior asserts and can continue through
    /// null pointers. This host boundary instead returns typed errors. Plugin
    /// and condition activation behavior remains outside this type.
    ///
    /// # Errors
    ///
    /// Returns a structural, required-attribute, factory-name, operator, or
    /// checked numeric-conversion failure. Exactly one plugin is required by
    /// this shipped-corpus boundary; native parsing instead retains the first
    /// nonnull root while still constructing later plugins. This boundary also
    /// rejects repeated top expressions and unknown tags, whereas native code
    /// can replace the expression or push/pop a null unknown leaf.
    pub fn build(source: &BehaviourBinaryElement) -> Result<Self, BehaviourGraphRuntimeError> {
        if tag_kind(&source.name) != Some(TagKind::BehaviourSource) {
            return Err(BehaviourGraphRuntimeError::UnexpectedElement {
                name: source.name.clone(),
                context: "document root",
            });
        }
        let mut root = None;
        for child in &source.children {
            if tag_kind(&child.name) != Some(TagKind::BehaviourPlugin) {
                return Err(BehaviourGraphRuntimeError::UnexpectedElement {
                    name: child.name.clone(),
                    context: "BehaviourSource",
                });
            }
            let plugin = build_plugin(child)?;
            if root.replace(plugin).is_some() {
                return Err(BehaviourGraphRuntimeError::MultiplePlugins);
            }
        }
        Ok(Self {
            root: root.ok_or(BehaviourGraphRuntimeError::MissingPlugin)?,
        })
    }
}

fn build_plugin(
    element: &BehaviourBinaryElement,
) -> Result<TransferableBehaviour, BehaviourGraphRuntimeError> {
    let name = required_text(element, "name")?;
    let class = plugin_class(name)
        .ok_or_else(|| BehaviourGraphRuntimeError::UnknownPlugin(name.to_owned()))?;
    let file = required_text(element, "file")?.to_owned();
    let maximum_participants = match native_attribute(element, "maxNumberOfParticipants") {
        Some(attribute) => checked_fctidz_u32(element, attribute, "maxNumberOfParticipants")?,
        None => 1,
    };
    let mut expression = None;
    for child in &element.children {
        if tag_kind(&child.name) != Some(TagKind::TransferExpression) {
            return Err(BehaviourGraphRuntimeError::UnexpectedElement {
                name: child.name.clone(),
                context: "BehaviourPlugin",
            });
        }
        let built = build_expression(child, true, 2)?;
        if expression.replace(built).is_some() {
            return Err(BehaviourGraphRuntimeError::MultipleExpressions);
        }
    }
    Ok(TransferableBehaviour {
        class,
        file,
        maximum_participants,
        expression: expression.ok_or(BehaviourGraphRuntimeError::MissingExpression)?,
    })
}

fn build_expression(
    element: &BehaviourBinaryElement,
    node_parent: bool,
    wire_depth: usize,
) -> Result<TransferExpression, BehaviourGraphRuntimeError> {
    if wire_depth > MAX_WIRE_ELEMENT_DEPTH {
        return Err(BehaviourGraphRuntimeError::NestingDepth {
            depth: wire_depth,
            maximum: MAX_WIRE_ELEMENT_DEPTH,
        });
    }
    let operator = if node_parent {
        // The node-parent constructor does not read the serialized `op`.
        TransferOperator::None
    } else {
        match native_attribute(element, "op").map(|attribute| attribute.text.as_str()) {
            None => TransferOperator::None,
            Some("and") => TransferOperator::And,
            Some("or") => TransferOperator::Or,
            Some("not") => TransferOperator::Not,
            Some(other) => {
                return Err(BehaviourGraphRuntimeError::UnknownOperator(
                    other.to_owned(),
                ));
            }
        }
    };
    let mut children = Vec::new();
    children
        .try_reserve_exact(element.children.len())
        .map_err(|_| BehaviourGraphRuntimeError::Allocation)?;
    for child in &element.children {
        let child_depth = wire_depth + 1;
        if child_depth > MAX_WIRE_ELEMENT_DEPTH {
            return Err(BehaviourGraphRuntimeError::NestingDepth {
                depth: child_depth,
                maximum: MAX_WIRE_ELEMENT_DEPTH,
            });
        }
        match tag_kind(&child.name) {
            Some(TagKind::TransferExpression) => {
                children.push(TransferExpressionChild::Expression(build_expression(
                    child,
                    false,
                    child_depth,
                )?));
            }
            Some(TagKind::TransferCondition) => {
                children.push(TransferExpressionChild::Condition(build_condition(child)?));
            }
            _ => {
                return Err(BehaviourGraphRuntimeError::UnexpectedElement {
                    name: child.name.clone(),
                    context: "TransferExpression",
                });
            }
        }
    }
    Ok(TransferExpression { operator, children })
}

fn build_condition(
    element: &BehaviourBinaryElement,
) -> Result<TransferCondition, BehaviourGraphRuntimeError> {
    let name = required_text(element, "name")?;
    if !element.children.is_empty() {
        return Err(BehaviourGraphRuntimeError::ConditionHasChildren {
            name: name.to_owned(),
        });
    }
    let condition = if name.eq_ignore_ascii_case("IsNearCrossWalk") {
        TransferCondition::IsNearCrossWalk {
            distance: native_attribute(element, "distance")
                .map_or(NativeOptionalF32::Unwritten, |attribute| {
                    NativeOptionalF32::Written(attribute.float_bits)
                }),
        }
    } else if name.eq_ignore_ascii_case("IsOnRoad") {
        TransferCondition::IsOnRoad
    } else if name.eq_ignore_ascii_case("CanRecognizePlayer") {
        TransferCondition::CanRecognizePlayer {
            maximum_recognizers: match native_attribute(element, "maxrecognizers") {
                Some(attribute) => checked_fctidz_u32(element, attribute, "maxrecognizers")?,
                None => 5,
            },
        }
    } else if name.eq_ignore_ascii_case("HasLineOfSightToProp") {
        TransferCondition::HasLineOfSightToProp
    } else if name.eq_ignore_ascii_case("NearAvailableWaypoint") {
        TransferCondition::NearAvailableWaypoint {
            distance_bits: required_attribute(element, "distance")?.float_bits,
        }
    } else if name.eq_ignore_ascii_case("HasFirstWaypointAvailable") {
        TransferCondition::HasFirstWaypointAvailable
    } else if name.eq_ignore_ascii_case("PluginTransferOdds") {
        let percentage = required_attribute(element, "percentage")?.float_value();
        let scaled = if percentage == 0.0 {
            0.0
        } else {
            percentage * f32::from_bits(0x3c23_d70a)
        };
        TransferCondition::PluginTransferOdds {
            percentage_bits: scaled.to_bits(),
        }
    } else if name.eq_ignore_ascii_case("InFrontOfFirstWaypoint") {
        TransferCondition::InFrontOfFirstWaypoint
    } else if name.eq_ignore_ascii_case("HasHandProp") {
        TransferCondition::HasHandProp
    } else if name.eq_ignore_ascii_case("HasDisposableHandProp") {
        TransferCondition::HasDisposableHandProp
    } else if name.eq_ignore_ascii_case("CanSitWithHandProp") {
        TransferCondition::CanSitWithHandProp
    } else if name.eq_ignore_ascii_case("IsChasing") {
        TransferCondition::IsChasing
    } else if name.eq_ignore_ascii_case("EntityAllowedToJoinConversation") {
        TransferCondition::EntityAllowedToJoinConversation
    } else if name.eq_ignore_ascii_case("ConversationIsGatheringMembers") {
        TransferCondition::ConversationIsGatheringMembers
    } else {
        return Err(BehaviourGraphRuntimeError::UnknownCondition(
            name.to_owned(),
        ));
    };
    Ok(condition)
}

fn plugin_class(name: &str) -> Option<TransferableBehaviourClass> {
    if name.eq_ignore_ascii_case("ActorTrackerSpectate") {
        Some(TransferableBehaviourClass::ActorTrackerSpectate)
    } else if name.eq_ignore_ascii_case("LookAt") {
        Some(TransferableBehaviourClass::LookAt)
    } else if name.eq_ignore_ascii_case("SpectatorWaypointGeneric") {
        Some(TransferableBehaviourClass::SpectatorWaypointGeneric)
    } else if name.eq_ignore_ascii_case("UseWorldProp") {
        Some(TransferableBehaviourClass::UseWorldProp)
    } else if name.eq_ignore_ascii_case("PedestrianConversation") {
        Some(TransferableBehaviourClass::PedestrianConversation)
    } else {
        None
    }
}

fn required_text<'a>(
    element: &'a BehaviourBinaryElement,
    name: &'static str,
) -> Result<&'a str, BehaviourGraphRuntimeError> {
    Ok(&required_attribute(element, name)?.text)
}

fn required_attribute<'a>(
    element: &'a BehaviourBinaryElement,
    name: &'static str,
) -> Result<&'a BehaviourBinaryAttribute, BehaviourGraphRuntimeError> {
    native_attribute(element, name).ok_or_else(|| BehaviourGraphRuntimeError::MissingAttribute {
        element: element.name.clone(),
        attribute: name,
    })
}

fn native_attribute<'a>(
    element: &'a BehaviourBinaryElement,
    name: &str,
) -> Option<&'a BehaviourBinaryAttribute> {
    let hash = state_graph_string_hash32(name.as_bytes());
    element
        .attributes
        .iter()
        .find(|attribute| state_graph_string_hash32(attribute.name.as_bytes()) == hash)
}

fn checked_fctidz_u32(
    element: &BehaviourBinaryElement,
    attribute: &BehaviourBinaryAttribute,
    name: &'static str,
) -> Result<u32, BehaviourGraphRuntimeError> {
    const U32_EXCLUSIVE_LIMIT: f32 = 4_294_967_296.0;

    let value = attribute.float_value();
    if !value.is_finite() || !(0.0..U32_EXCLUSIVE_LIMIT).contains(&value) {
        return Err(BehaviourGraphRuntimeError::NumericRange {
            element: element.name.clone(),
            attribute: name,
            bits: attribute.float_bits,
        });
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let converted = value.trunc() as u32;
    Ok(converted)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TagKind {
    BehaviourSource,
    BehaviourPlugin,
    TransferExpression,
    TransferCondition,
}

fn tag_kind(name: &str) -> Option<TagKind> {
    let hash = state_graph_string_hash32(name.as_bytes());
    if hash == state_graph_string_hash32(b"BehaviourSource") {
        Some(TagKind::BehaviourSource)
    } else if hash == state_graph_string_hash32(b"BehaviourPlugin") {
        Some(TagKind::BehaviourPlugin)
    } else if hash == state_graph_string_hash32(b"TransferExpression") {
        Some(TagKind::TransferExpression)
    } else if hash == state_graph_string_hash32(b"TransferCondition") {
        Some(TagKind::TransferCondition)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attribute(name: &str, text: &str, value: f32, boolean_raw: u8) -> BehaviourBinaryAttribute {
        BehaviourBinaryAttribute {
            name: name.to_owned(),
            text: text.to_owned(),
            float_bits: value.to_bits(),
            boolean_raw,
        }
    }

    fn element(
        name: &str,
        attributes: Vec<BehaviourBinaryAttribute>,
        children: Vec<BehaviourBinaryElement>,
    ) -> BehaviourBinaryElement {
        BehaviourBinaryElement {
            name: name.to_owned(),
            attributes,
            children,
        }
    }

    fn condition(name: &str) -> BehaviourBinaryElement {
        element(
            "TransferCondition",
            vec![attribute("name", name, 0.0, 0)],
            Vec::new(),
        )
    }

    #[test]
    fn top_expression_ignores_op_while_nested_expression_parses_it() {
        let nested = element(
            "TransferExpression",
            vec![attribute("op", "not", 0.0, 0)],
            vec![condition("IsOnRoad")],
        );
        let top = element(
            "TransferExpression",
            vec![attribute("op", "and", 0.0, 0)],
            vec![nested],
        );
        let plugin = element(
            "BehaviourPlugin",
            vec![
                attribute("name", "LookAt", 0.0, 0),
                attribute("file", "state/example.xml", 0.0, 0),
            ],
            vec![top],
        );
        let graph = TransferableBehaviourGraph::build(&element(
            "BehaviourSource",
            Vec::new(),
            vec![plugin],
        ))
        .unwrap();
        assert_eq!(graph.root.expression.operator, TransferOperator::None);
        assert!(matches!(
            &graph.root.expression.children[..],
            [TransferExpressionChild::Expression(TransferExpression {
                operator: TransferOperator::Not,
                children,
            })] if matches!(children.as_slice(), [TransferExpressionChild::Condition(TransferCondition::IsOnRoad)])
        ));
    }

    #[test]
    fn nested_operator_spellings_are_exact_and_missing_means_none() {
        for (spelling, expected) in [
            (Some("and"), TransferOperator::And),
            (Some("or"), TransferOperator::Or),
            (Some("not"), TransferOperator::Not),
            (None, TransferOperator::None),
        ] {
            let attributes =
                spelling.map_or_else(Vec::new, |spelling| vec![attribute("op", spelling, 0.0, 0)]);
            let expression = build_expression(
                &element("TransferExpression", attributes, Vec::new()),
                false,
                2,
            )
            .unwrap();
            assert_eq!(expression.operator, expected);
        }
        assert!(matches!(
            build_expression(
                &element(
                    "TransferExpression",
                    vec![attribute("op", "AND", 0.0, 0)],
                    Vec::new(),
                ),
                false,
                2,
            ),
            Err(BehaviourGraphRuntimeError::UnknownOperator(operator)) if operator == "AND"
        ));
    }

    #[test]
    fn expression_depth_matches_the_checked_wire_boundary() {
        let empty = element("TransferExpression", Vec::new(), Vec::new());
        assert!(build_expression(&empty, false, MAX_WIRE_ELEMENT_DEPTH).is_ok());
        assert_eq!(
            build_expression(&empty, false, MAX_WIRE_ELEMENT_DEPTH + 1),
            Err(BehaviourGraphRuntimeError::NestingDepth {
                depth: MAX_WIRE_ELEMENT_DEPTH + 1,
                maximum: MAX_WIRE_ELEMENT_DEPTH,
            })
        );
        let condition_child = element(
            "TransferExpression",
            Vec::new(),
            vec![condition("IsOnRoad")],
        );
        assert!(build_expression(&condition_child, false, MAX_WIRE_ELEMENT_DEPTH - 1).is_ok());
        assert_eq!(
            build_expression(&condition_child, false, MAX_WIRE_ELEMENT_DEPTH),
            Err(BehaviourGraphRuntimeError::NestingDepth {
                depth: MAX_WIRE_ELEMENT_DEPTH + 1,
                maximum: MAX_WIRE_ELEMENT_DEPTH,
            })
        );
    }

    #[test]
    fn factory_values_are_case_insensitive() {
        let plugin = element(
            "BehaviourPlugin",
            vec![
                attribute("name", "lookat", 0.0, 0),
                attribute("file", "state/example.xml", 0.0, 0),
            ],
            vec![element(
                "TransferExpression",
                Vec::new(),
                vec![condition("isonroad")],
            )],
        );
        let graph = TransferableBehaviourGraph::build(&element(
            "BehaviourSource",
            Vec::new(),
            vec![plugin],
        ))
        .unwrap();
        assert_eq!(graph.root.class, TransferableBehaviourClass::LookAt);
        assert!(matches!(
            graph.root.expression.children.as_slice(),
            [TransferExpressionChild::Condition(
                TransferCondition::IsOnRoad
            )]
        ));
    }

    #[test]
    fn attribute_lookup_uses_only_the_native_hash_and_first_collision() {
        assert_eq!(
            state_graph_string_hash32(b"a05763"),
            state_graph_string_hash32(b"a063d3")
        );
        let element = element(
            "TransferCondition",
            vec![
                attribute("a05763", "first", 1.0, 0),
                attribute("a063d3", "second", 2.0, 0),
            ],
            Vec::new(),
        );
        let found = native_attribute(&element, "a063d3").unwrap();
        assert_eq!(found.name, "a05763");
        assert_eq!(found.text, "first");
    }

    #[test]
    fn reconstructs_proven_numeric_fields_and_leaves_crosswalk_unwritten() {
        let conditions = vec![
            element(
                "TransferCondition",
                vec![
                    attribute("name", "CanRecognizePlayer", 0.0, 0),
                    attribute("maxrecognizers", "4", 4.0, 0),
                ],
                Vec::new(),
            ),
            element(
                "TransferCondition",
                vec![
                    attribute("name", "PluginTransferOdds", 0.0, 0),
                    attribute("percentage", "7", 7.0, 0),
                ],
                Vec::new(),
            ),
            element(
                "TransferCondition",
                vec![
                    attribute("name", "NearAvailableWaypoint", 0.0, 0),
                    attribute("distance", "5", 5.0, 0),
                ],
                Vec::new(),
            ),
            condition("IsNearCrossWalk"),
        ];
        let built = build_expression(
            &element("TransferExpression", Vec::new(), conditions),
            true,
            2,
        )
        .unwrap();
        assert_eq!(
            built.children,
            [
                TransferExpressionChild::Condition(TransferCondition::CanRecognizePlayer {
                    maximum_recognizers: 4,
                }),
                TransferExpressionChild::Condition(TransferCondition::PluginTransferOdds {
                    percentage_bits: 0x3d8f_5c29,
                }),
                TransferExpressionChild::Condition(TransferCondition::NearAvailableWaypoint {
                    distance_bits: 5.0_f32.to_bits(),
                }),
                TransferExpressionChild::Condition(TransferCondition::IsNearCrossWalk {
                    distance: NativeOptionalF32::Unwritten,
                }),
            ]
        );
    }

    #[test]
    fn checked_integer_conversion_covers_the_host_policy_boundaries() {
        let element = element("BehaviourPlugin", Vec::new(), Vec::new());
        let convert = |bits| {
            let attribute = attribute("value", "", f32::from_bits(bits), 0);
            checked_fctidz_u32(&element, &attribute, "maxNumberOfParticipants")
        };
        assert_eq!(convert((-0.0_f32).to_bits()), Ok(0));
        assert_eq!(convert(3.9_f32.to_bits()), Ok(3));
        assert_eq!(convert(0x4f7f_ffff), Ok(4_294_967_040));
        for bits in [
            0x4f80_0000,
            (-1.0_f32).to_bits(),
            f32::NAN.to_bits(),
            f32::INFINITY.to_bits(),
            f32::NEG_INFINITY.to_bits(),
        ] {
            assert!(matches!(
                convert(bits),
                Err(BehaviourGraphRuntimeError::NumericRange {
                    bits: error_bits,
                    ..
                }) if error_bits == bits
            ));
        }
    }

    #[test]
    fn condition_defaults_present_bits_and_negative_zero_odds_are_explicit() {
        assert_eq!(
            build_condition(&condition("CanRecognizePlayer")).unwrap(),
            TransferCondition::CanRecognizePlayer {
                maximum_recognizers: 5,
            }
        );
        assert_eq!(
            build_condition(&element(
                "TransferCondition",
                vec![
                    attribute("name", "IsNearCrossWalk", 0.0, 0),
                    attribute("distance", "-0", -0.0, 0),
                ],
                Vec::new(),
            ))
            .unwrap(),
            TransferCondition::IsNearCrossWalk {
                distance: NativeOptionalF32::Written(0x8000_0000),
            }
        );
        assert_eq!(
            build_condition(&element(
                "TransferCondition",
                vec![
                    attribute("name", "PluginTransferOdds", 0.0, 0),
                    attribute("percentage", "-0", -0.0, 0),
                ],
                Vec::new(),
            ))
            .unwrap(),
            TransferCondition::PluginTransferOdds { percentage_bits: 0 }
        );
    }

    #[test]
    fn plugin_fields_use_required_file_and_default_or_truncated_participants() {
        let make_plugin = |participants: Option<f32>| {
            let mut attributes = vec![
                attribute("name", "PedestrianConversation", 0.0, 0),
                attribute("file", "state/conversation.xml", 0.0, 0),
                attribute("radius", "10", 10.0, 0),
            ];
            if let Some(value) = participants {
                attributes.push(attribute("maxNumberOfParticipants", "3.9", value, 0));
            }
            build_plugin(&element(
                "BehaviourPlugin",
                attributes,
                vec![element("TransferExpression", Vec::new(), Vec::new())],
            ))
            .unwrap()
        };
        assert_eq!(make_plugin(None).maximum_participants, 1);
        let plugin = make_plugin(Some(3.9));
        assert_eq!(plugin.maximum_participants, 3);
        assert_eq!(plugin.file, "state/conversation.xml");
    }
}
