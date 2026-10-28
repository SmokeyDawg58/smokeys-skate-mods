//! Shipped compiled living-world `BehaviourGraph` descriptor corpus.
//!
//! This validates both the recovered binary wire grammar and the proven
//! plugin/expression/condition construction subset. Controller materialization
//! and condition activation remain outside this collaborator.

use skate2_core::{
    behaviour_graph_runtime::{
        NativeOptionalF32, TransferCondition, TransferExpression, TransferExpressionChild,
        TransferableBehaviourClass, TransferableBehaviourGraph,
    },
    graph_asset_cache::{behaviour_graph_cache_key, compiled_graph_path},
    living_world_state_graph_preload::{
        LIVING_WORLD_BEHAVIOUR_GRAPH_PATHS, LIVING_WORLD_STATE_GRAPH_PATHS,
    },
};
use skate2_formats::{behaviour::BehaviourBinaryElement, mount::MountSet};
use std::error::Error;

const EXPECTED_COMPILED_BYTES: [usize; 10] = [517, 442, 299, 635, 348, 584, 608, 525, 595, 594];
const EXPECTED_NODES: [usize; 10] = [9, 7, 6, 13, 6, 12, 12, 11, 12, 12];
const EXPECTED_CLASSES: [TransferableBehaviourClass; 10] = [
    TransferableBehaviourClass::ActorTrackerSpectate,
    TransferableBehaviourClass::PedestrianConversation,
    TransferableBehaviourClass::LookAt,
    TransferableBehaviourClass::UseWorldProp,
    TransferableBehaviourClass::SpectatorWaypointGeneric,
    TransferableBehaviourClass::UseWorldProp,
    TransferableBehaviourClass::UseWorldProp,
    TransferableBehaviourClass::UseWorldProp,
    TransferableBehaviourClass::UseWorldProp,
    TransferableBehaviourClass::UseWorldProp,
];
const EXPECTED_PARTICIPANTS: [u32; 10] = [1, 3, 1, 1, 1, 1, 1, 1, 1, 1];
const EXPECTED_OPERATOR_COUNTS: [usize; 4] = [12, 0, 0, 27];
const EXPECTED_CLASS_COUNTS: [usize; 5] = [1, 1, 1, 6, 1];
const EXPECTED_CONDITION_COUNTS: [usize; 14] = [6, 7, 1, 1, 1, 5, 1, 1, 4, 1, 1, 10, 1, 1];
const EXPECTED_TREE_SIGNATURES: [&str; 10] = [
    "None(Not(IsOnRoad),CanRecognizePlayer:4,PluginTransferOdds:3d8f5c29,Not(IsChasing))",
    "None(ConversationIsGatheringMembers,EntityAllowedToJoinConversation,Not(IsChasing))",
    "None(HasLineOfSightToProp,Not(IsChasing))",
    "None(HasFirstWaypointAvailable,InFrontOfFirstWaypoint,None(CanSitWithHandProp),Not(IsOnRoad),Not(IsNearCrossWalk:Unwritten),Not(IsChasing))",
    "None(NearAvailableWaypoint:40a00000,Not(IsChasing))",
    "None(HasFirstWaypointAvailable,Not(HasHandProp),Not(IsOnRoad),Not(IsNearCrossWalk:Unwritten),Not(IsChasing))",
    "None(HasFirstWaypointAvailable,Not(HasHandProp),Not(IsOnRoad),Not(IsNearCrossWalk:Unwritten),Not(IsChasing))",
    "None(None(HasDisposableHandProp),Not(IsOnRoad),Not(IsNearCrossWalk:Unwritten),Not(IsChasing))",
    "None(HasFirstWaypointAvailable,Not(HasHandProp),Not(IsOnRoad),Not(IsNearCrossWalk:Unwritten),Not(IsChasing))",
    "None(HasFirstWaypointAvailable,Not(HasHandProp),Not(IsOnRoad),Not(IsNearCrossWalk:Unwritten),Not(IsChasing))",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BehaviourGraphCorpusSummary {
    pub descriptors: usize,
    pub decoded_bytes: usize,
    pub nodes: usize,
    pub runtime_expressions: usize,
    pub runtime_conditions: usize,
    pub participant_sum: u32,
}

impl BehaviourGraphCorpusSummary {
    #[must_use]
    pub const fn matches_shipped(&self) -> bool {
        self.descriptors == 10
            && self.decoded_bytes == 5_147
            && self.nodes == 100
            && self.runtime_expressions == 39
            && self.runtime_conditions == 41
            && self.participant_sum == 12
    }
}

/// Parses every compiled living-world plugin descriptor from the mounted
/// shipped corpus and validates its structural route to the matching state
/// path.
///
/// # Errors
///
/// Returns a checked cache-path, mount, binary grammar, byte-count, node-count,
/// root, plugin-count, or `file`-attribute mismatch.
pub fn load(mounts: &MountSet) -> Result<BehaviourGraphCorpusSummary, Box<dyn Error>> {
    let mut decoded_bytes = 0;
    let mut nodes = 0;
    let mut runtime_expressions = 0;
    let mut runtime_conditions = 0;
    let mut participant_sum = 0;
    let mut operator_counts = [0; 4];
    let mut class_counts = [0; 5];
    let mut condition_counts = [0; 14];
    for (index, descriptor_path) in LIVING_WORLD_BEHAVIOUR_GRAPH_PATHS.into_iter().enumerate() {
        let (byte_count, node_count, runtime) =
            validate_descriptor(mounts, index, descriptor_path)?;
        let (expressions, conditions) = runtime_tree_counts(&runtime.root.expression);
        class_counts[class_index(runtime.root.class)] += 1;
        accumulate_runtime_census(
            &runtime.root.expression,
            &mut operator_counts,
            &mut condition_counts,
        );
        runtime_expressions += expressions;
        runtime_conditions += conditions;
        participant_sum += runtime.root.maximum_participants;
        decoded_bytes += byte_count;
        nodes += node_count;
    }
    if operator_counts != EXPECTED_OPERATOR_COUNTS
        || class_counts != EXPECTED_CLASS_COUNTS
        || condition_counts != EXPECTED_CONDITION_COUNTS
    {
        return Err(format!(
            "BehaviourGraph runtime census differed: operators {operator_counts:?}, classes {class_counts:?}, conditions {condition_counts:?}"
        )
        .into());
    }
    Ok(BehaviourGraphCorpusSummary {
        descriptors: LIVING_WORLD_BEHAVIOUR_GRAPH_PATHS.len(),
        decoded_bytes,
        nodes,
        runtime_expressions,
        runtime_conditions,
        participant_sum,
    })
}

fn validate_descriptor(
    mounts: &MountSet,
    index: usize,
    descriptor_path: &str,
) -> Result<(usize, usize, TransferableBehaviourGraph), Box<dyn Error>> {
    let key = behaviour_graph_cache_key(descriptor_path)?;
    let compiled_path = compiled_graph_path(key.normalized_path())?;
    let bytes = mounts.read_path(&compiled_path)?;
    if bytes.len() != EXPECTED_COMPILED_BYTES[index] {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} had {} bytes, expected {}",
            bytes.len(),
            EXPECTED_COMPILED_BYTES[index]
        )
        .into());
    }
    let root = BehaviourBinaryElement::parse(&bytes)?;
    if root.name != "BehaviourSource" {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} root was {:?}",
            root.name
        )
        .into());
    }
    let node_count = root.node_count();
    if node_count != EXPECTED_NODES[index] {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} had {node_count} nodes, expected {}",
            EXPECTED_NODES[index]
        )
        .into());
    }
    let mut plugins = root
        .preorder()
        .filter(|element| element.name == "BehaviourPlugin");
    let plugin = plugins
        .next()
        .ok_or_else(|| format!("compiled BehaviourGraph {compiled_path} had no plugin"))?;
    if plugins.next().is_some() {
        return Err(format!("compiled BehaviourGraph {compiled_path} had multiple plugins").into());
    }
    let expected_state_path = LIVING_WORLD_STATE_GRAPH_PATHS[index + 3];
    let file = plugin
        .attribute("file")
        .ok_or_else(|| format!("compiled BehaviourGraph {compiled_path} had no file field"))?;
    if file.text != expected_state_path {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} targeted {:?}, expected {expected_state_path:?}",
            file.text
        )
        .into());
    }
    verify_serialized_runtime_inputs(index, &root, plugin, &compiled_path)?;
    let runtime = TransferableBehaviourGraph::build(&root)?;
    if runtime.root.class != EXPECTED_CLASSES[index]
        || runtime.root.file != expected_state_path
        || runtime.root.maximum_participants != EXPECTED_PARTICIPANTS[index]
    {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} runtime header differed: {:?}",
            runtime.root
        )
        .into());
    }
    let signature = expression_signature(&runtime.root.expression);
    if signature != EXPECTED_TREE_SIGNATURES[index] {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} runtime tree was {signature}, expected {}",
            EXPECTED_TREE_SIGNATURES[index]
        )
        .into());
    }
    Ok((bytes.len(), node_count, runtime))
}

fn verify_serialized_runtime_inputs(
    index: usize,
    root: &BehaviourBinaryElement,
    plugin: &BehaviourBinaryElement,
    compiled_path: &str,
) -> Result<(), Box<dyn Error>> {
    let top_expression = plugin
        .children
        .first()
        .ok_or_else(|| format!("compiled BehaviourGraph {compiled_path} had no top expression"))?;
    let expected_top_op = if index == 2 { None } else { Some("and") };
    if top_expression
        .attribute("op")
        .map(|attribute| attribute.text.as_str())
        != expected_top_op
    {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} serialized top operator differed"
        )
        .into());
    }
    let participants = plugin.attribute("maxNumberOfParticipants");
    if participants.map(|attribute| attribute.float_bits)
        != (index == 1).then_some(3.0_f32.to_bits())
    {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} serialized participant source differed"
        )
        .into());
    }
    let radius = plugin.attribute("radius");
    if radius.map(|attribute| attribute.float_bits) != (index == 6).then_some(10.0_f32.to_bits()) {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} serialized ignored radius differed"
        )
        .into());
    }
    if index == 0 {
        verify_condition_source_bits(
            root,
            "CanRecognizePlayer",
            "maxrecognizers",
            4.0,
            compiled_path,
        )?;
        verify_condition_source_bits(root, "PluginTransferOdds", "percentage", 7.0, compiled_path)?;
    }
    if index == 4 {
        verify_condition_source_bits(
            root,
            "NearAvailableWaypoint",
            "distance",
            5.0,
            compiled_path,
        )?;
    }
    for element in root.preorder().filter(|element| {
        element
            .attribute("name")
            .map(|attribute| attribute.text.as_str())
            == Some("IsNearCrossWalk")
    }) {
        if element.attribute("distance").is_some() {
            return Err(format!(
                "compiled BehaviourGraph {compiled_path} unexpectedly initialized IsNearCrossWalk distance"
            )
            .into());
        }
    }
    Ok(())
}

fn verify_condition_source_bits(
    root: &BehaviourBinaryElement,
    condition_name: &str,
    attribute_name: &str,
    expected: f32,
    compiled_path: &str,
) -> Result<(), Box<dyn Error>> {
    let condition = root
        .preorder()
        .find(|element| {
            element
                .attribute("name")
                .map(|attribute| attribute.text.as_str())
                == Some(condition_name)
        })
        .ok_or_else(|| {
            format!("compiled BehaviourGraph {compiled_path} lacked {condition_name}")
        })?;
    if condition
        .attribute(attribute_name)
        .map(|attribute| attribute.float_bits)
        != Some(expected.to_bits())
    {
        return Err(format!(
            "compiled BehaviourGraph {compiled_path} {condition_name}.{attribute_name} source bits differed"
        )
        .into());
    }
    Ok(())
}

fn expression_signature(expression: &TransferExpression) -> String {
    use std::fmt::Write as _;

    let mut result = format!("{:?}(", expression.operator);
    for (index, child) in expression.children.iter().enumerate() {
        if index != 0 {
            result.push(',');
        }
        match child {
            TransferExpressionChild::Expression(child) => {
                result.push_str(&expression_signature(child));
            }
            TransferExpressionChild::Condition(condition) => match condition {
                TransferCondition::IsNearCrossWalk { distance } => match distance {
                    NativeOptionalF32::Written(bits) => {
                        write!(result, "IsNearCrossWalk:{bits:08x}")
                            .expect("writing into a String cannot fail");
                    }
                    NativeOptionalF32::Unwritten => result.push_str("IsNearCrossWalk:Unwritten"),
                },
                TransferCondition::CanRecognizePlayer {
                    maximum_recognizers,
                } => {
                    write!(result, "CanRecognizePlayer:{maximum_recognizers}")
                        .expect("writing into a String cannot fail");
                }
                TransferCondition::NearAvailableWaypoint { distance_bits } => {
                    write!(result, "NearAvailableWaypoint:{distance_bits:08x}")
                        .expect("writing into a String cannot fail");
                }
                TransferCondition::PluginTransferOdds { percentage_bits } => {
                    write!(result, "PluginTransferOdds:{percentage_bits:08x}")
                        .expect("writing into a String cannot fail");
                }
                other => write!(result, "{other:?}").expect("writing into a String cannot fail"),
            },
        }
    }
    result.push(')');
    result
}

fn runtime_tree_counts(expression: &TransferExpression) -> (usize, usize) {
    let mut expressions = 1;
    let mut conditions = 0;
    for child in &expression.children {
        match child {
            TransferExpressionChild::Expression(child) => {
                let child_counts = runtime_tree_counts(child);
                expressions += child_counts.0;
                conditions += child_counts.1;
            }
            TransferExpressionChild::Condition(_) => conditions += 1,
        }
    }
    (expressions, conditions)
}

fn class_index(class: TransferableBehaviourClass) -> usize {
    match class {
        TransferableBehaviourClass::ActorTrackerSpectate => 0,
        TransferableBehaviourClass::LookAt => 1,
        TransferableBehaviourClass::SpectatorWaypointGeneric => 2,
        TransferableBehaviourClass::UseWorldProp => 3,
        TransferableBehaviourClass::PedestrianConversation => 4,
    }
}

fn accumulate_runtime_census(
    expression: &TransferExpression,
    operator_counts: &mut [usize; 4],
    condition_counts: &mut [usize; 14],
) {
    operator_counts[match expression.operator {
        skate2_core::behaviour_graph_runtime::TransferOperator::None => 0,
        skate2_core::behaviour_graph_runtime::TransferOperator::And => 1,
        skate2_core::behaviour_graph_runtime::TransferOperator::Or => 2,
        skate2_core::behaviour_graph_runtime::TransferOperator::Not => 3,
    }] += 1;
    for child in &expression.children {
        match child {
            TransferExpressionChild::Expression(child) => {
                accumulate_runtime_census(child, operator_counts, condition_counts);
            }
            TransferExpressionChild::Condition(condition) => {
                condition_counts[condition_index(condition)] += 1;
            }
        }
    }
}

fn condition_index(condition: &TransferCondition) -> usize {
    match condition {
        TransferCondition::IsNearCrossWalk { .. } => 0,
        TransferCondition::IsOnRoad => 1,
        TransferCondition::CanRecognizePlayer { .. } => 2,
        TransferCondition::HasLineOfSightToProp => 3,
        TransferCondition::NearAvailableWaypoint { .. } => 4,
        TransferCondition::HasFirstWaypointAvailable => 5,
        TransferCondition::PluginTransferOdds { .. } => 6,
        TransferCondition::InFrontOfFirstWaypoint => 7,
        TransferCondition::HasHandProp => 8,
        TransferCondition::HasDisposableHandProp => 9,
        TransferCondition::CanSitWithHandProp => 10,
        TransferCondition::IsChasing => 11,
        TransferCondition::EntityAllowedToJoinConversation => 12,
        TransferCondition::ConversationIsGatheringMembers => 13,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_core::behaviour_graph_runtime::TransferOperator;

    #[test]
    fn shipped_descriptor_oracle_totals_are_frozen() {
        assert_eq!(EXPECTED_COMPILED_BYTES.iter().sum::<usize>(), 5_147);
        assert_eq!(EXPECTED_NODES.iter().sum::<usize>(), 100);
        assert_eq!(LIVING_WORLD_STATE_GRAPH_PATHS[3..13].len(), 10);
        assert_eq!(EXPECTED_PARTICIPANTS.iter().sum::<u32>(), 12);
        assert_eq!(EXPECTED_TREE_SIGNATURES.len(), 10);
        assert_eq!(EXPECTED_OPERATOR_COUNTS.iter().sum::<usize>(), 39);
        assert_eq!(EXPECTED_CLASS_COUNTS.iter().sum::<usize>(), 10);
        assert_eq!(EXPECTED_CONDITION_COUNTS.iter().sum::<usize>(), 41);
    }

    #[test]
    fn signature_keeps_operator_and_native_field_bits() {
        let expression = TransferExpression {
            operator: TransferOperator::Not,
            children: vec![TransferExpressionChild::Condition(
                TransferCondition::PluginTransferOdds {
                    percentage_bits: 0x3d8f_5c29,
                },
            )],
        };
        assert_eq!(
            expression_signature(&expression),
            "Not(PluginTransferOdds:3d8f5c29)"
        );
    }
}
