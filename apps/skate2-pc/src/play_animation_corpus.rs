//! Mounted `MotionGraph_OnBoard` gate for the proven `PlayAnimation` stored
//! constructor and direct `Param` parsing boundary.
//!
//! This does not register a factory or claim animation/runtime execution. It
//! instantiates only exact `PlayAnimation` behaviour records and attaches only
//! their direct `Param` children in serialized order.

use crate::state_graph_registry_catalog::sha256_hex;
use skate2_core::play_animation::{
    AnimationTransitionPlacement, PLAY_ANIMATION_CORPUS_OCCURRENCES_SHA256,
    PLAY_ANIMATION_CORPUS_PARAM_TREE_SHA256, PlayAnimationBehaviour,
};
use skate2_formats::{
    mount::MountSet,
    state_graph::{StateGraphBinaryAttribute, StateGraphBinaryDocument, StateGraphBinaryElement},
};
use std::{error::Error, fmt::Write as _};

const MOTION_GRAPH_PATH: &str = r"data\state\MotionGraph_OnBoard.stategraph";
const MOTION_GRAPH_BYTES: usize = 641_241;
const MOTION_GRAPH_SHA256: &str =
    "E5F317CCAC9FAD0D126B3DF6810C9602CBB081F23FE39FB44662C92D4F13F7ED";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayAnimationCorpusSummary {
    pub behaviours: usize,
    pub params: usize,
    pub occurrence_sha256: String,
    pub param_tree_sha256: String,
    pub transition_type_counts: [usize; 5],
    pub switch_anim_present: usize,
    pub mirror_anim_present: usize,
    pub no_board_anim_present: usize,
    pub blend_with_current_frame_true: usize,
    pub transition_under_true: usize,
    pub use_channel_from_weights_true: usize,
    pub playback_speed_default: usize,
    pub blend_time_default: usize,
    pub param_type_counts: [usize; 3],
    pub ignored_trans_blend: usize,
    pub ignored_param_set_blend_true: usize,
}

impl PlayAnimationCorpusSummary {
    #[must_use]
    pub fn matches_shipped(&self) -> bool {
        self.behaviours == 807
            && self.params == 165
            && self.occurrence_sha256 == PLAY_ANIMATION_CORPUS_OCCURRENCES_SHA256
            && self.param_tree_sha256 == PLAY_ANIMATION_CORPUS_PARAM_TREE_SHA256
            && self.transition_type_counts == [0, 3, 652, 65, 87]
            && self.switch_anim_present == 8
            && self.mirror_anim_present == 28
            && self.no_board_anim_present == 4
            && self.blend_with_current_frame_true == 46
            && self.transition_under_true == 140
            && self.use_channel_from_weights_true == 64
            && self.playback_speed_default == 744
            && self.blend_time_default == 87
            && self.param_type_counts == [111, 0, 54]
            && self.ignored_trans_blend == 30
            && self.ignored_param_set_blend_true == 143
    }
}

#[derive(Default)]
struct CorpusBuilder {
    behaviours: usize,
    params: usize,
    transition_type_counts: [usize; 5],
    switch_anim_present: usize,
    mirror_anim_present: usize,
    no_board_anim_present: usize,
    blend_with_current_frame_true: usize,
    transition_under_true: usize,
    use_channel_from_weights_true: usize,
    playback_speed_default: usize,
    blend_time_default: usize,
    param_type_counts: [usize; 3],
    ignored_trans_blend: usize,
    ignored_param_set_blend_true: usize,
    occurrence_json: String,
    param_tree_json: String,
    preorder_index: usize,
}

impl CorpusBuilder {
    fn new() -> Self {
        Self {
            occurrence_json: "[".to_owned(),
            param_tree_json: "[".to_owned(),
            ..Self::default()
        }
    }

    fn visit(
        &mut self,
        element: &StateGraphBinaryElement,
        locator: &mut String,
        state_path: &mut Vec<Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        let is_state = element.name == b"state";
        if is_state {
            state_path.push(
                exact_attribute_text(element, b"name")
                    .unwrap_or(b"__unknown__")
                    .to_vec(),
            );
        }

        let current_index = self.preorder_index;
        self.preorder_index += 1;
        if element.name == b"behaviour"
            && exact_attribute_text(element, b"name") == Some(b"PlayAnimation")
        {
            self.add_behaviour(element, locator, state_path, current_index)?;
        }

        let parent_length = locator.len();
        for (child_index, child) in element.children.iter().enumerate() {
            write!(locator, ".{child_index}")?;
            self.visit(child, locator, state_path)?;
            locator.truncate(parent_length);
        }
        if is_state {
            state_path.pop();
        }
        Ok(())
    }

    fn add_behaviour(
        &mut self,
        element: &StateGraphBinaryElement,
        locator: &str,
        state_path: &[Vec<u8>],
        preorder_index: usize,
    ) -> Result<(), Box<dyn Error>> {
        let mut behaviour = PlayAnimationBehaviour::from_attributes(&element.attributes)?;
        let direct_params = element
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.name == b"param")
            .collect::<Vec<_>>();
        for (child_index, param) in &direct_params {
            if !param.children.is_empty() {
                return Err(format!(
                    "PlayAnimation direct Param {locator}.{child_index} retained nested children"
                )
                .into());
            }
            behaviour.add_param(&param.attributes)?;
        }

        if self.behaviours != 0 {
            self.occurrence_json.push(',');
            self.param_tree_json.push(',');
        }
        write_occurrence_json(
            &mut self.occurrence_json,
            element,
            locator,
            state_path,
            preorder_index,
            &direct_params,
        )?;
        write_param_tree_json(&mut self.param_tree_json, locator, &direct_params)?;

        self.behaviours += 1;
        self.params += behaviour.params().len();
        self.transition_type_counts[behaviour.transition.transition_type as usize] += 1;
        self.switch_anim_present += usize::from(behaviour.has_switch_anim);
        self.mirror_anim_present += usize::from(behaviour.has_mirror_anim);
        self.no_board_anim_present += usize::from(behaviour.has_no_board_anim);
        self.blend_with_current_frame_true +=
            usize::from(behaviour.transition.blend_with_current_frame);
        self.transition_under_true +=
            usize::from(behaviour.transition.placement == AnimationTransitionPlacement::Under);
        self.use_channel_from_weights_true +=
            usize::from(behaviour.transition.use_channel_from_weights);
        self.playback_speed_default +=
            usize::from(exact_attribute(element, b"playBackSpeed").is_none());
        self.blend_time_default += usize::from(exact_attribute(element, b"time").is_none());
        self.ignored_trans_blend += usize::from(
            exact_attribute_text(element, b"trans").is_some_and(|value| value == b"blend"),
        );

        for ((_, source), parsed) in direct_params.iter().zip(behaviour.params()) {
            self.param_type_counts[parsed.source_type as usize] += 1;
            self.ignored_param_set_blend_true += usize::from(
                exact_attribute_text(source, b"SetBlend").is_some_and(|value| value == b"true"),
            );
        }
        Ok(())
    }

    fn finish(mut self) -> PlayAnimationCorpusSummary {
        self.occurrence_json.push(']');
        self.param_tree_json.push(']');
        PlayAnimationCorpusSummary {
            behaviours: self.behaviours,
            params: self.params,
            occurrence_sha256: sha256_hex(self.occurrence_json.as_bytes()),
            param_tree_sha256: sha256_hex(self.param_tree_json.as_bytes()),
            transition_type_counts: self.transition_type_counts,
            switch_anim_present: self.switch_anim_present,
            mirror_anim_present: self.mirror_anim_present,
            no_board_anim_present: self.no_board_anim_present,
            blend_with_current_frame_true: self.blend_with_current_frame_true,
            transition_under_true: self.transition_under_true,
            use_channel_from_weights_true: self.use_channel_from_weights_true,
            playback_speed_default: self.playback_speed_default,
            blend_time_default: self.blend_time_default,
            param_type_counts: self.param_type_counts,
            ignored_trans_blend: self.ignored_trans_blend,
            ignored_param_set_blend_true: self.ignored_param_set_blend_true,
        }
    }
}

/// Parses the exact mounted onboard `MotionGraph` and validates the concrete
/// `PlayAnimation` stored-data subset against the frozen real-data oracle.
///
/// # Errors
///
/// Returns a mount, payload-identity, binary grammar, parser, canonicalization,
/// or exact-corpus mismatch.
pub fn load(mounts: &MountSet) -> Result<PlayAnimationCorpusSummary, Box<dyn Error>> {
    let bytes = mounts.read_path(MOTION_GRAPH_PATH)?;
    load_exact_payload(&bytes)
}

pub(crate) fn load_exact_payload(
    bytes: &[u8],
) -> Result<PlayAnimationCorpusSummary, Box<dyn Error>> {
    let payload_hash = sha256_hex(bytes);
    if bytes.len() != MOTION_GRAPH_BYTES || payload_hash != MOTION_GRAPH_SHA256 {
        return Err(format!(
            "mounted {MOTION_GRAPH_PATH} identity was {} bytes/{payload_hash}, expected {MOTION_GRAPH_BYTES}/{MOTION_GRAPH_SHA256}",
            bytes.len()
        )
        .into());
    }
    let document = StateGraphBinaryDocument::parse_exact(bytes)?;
    let summary = summarize(&document.root)?;
    if !summary.matches_shipped() {
        return Err(format!("PlayAnimation mounted corpus differed: {summary:?}").into());
    }
    Ok(summary)
}

fn summarize(root: &StateGraphBinaryElement) -> Result<PlayAnimationCorpusSummary, Box<dyn Error>> {
    let mut builder = CorpusBuilder::new();
    builder.visit(root, &mut "0".to_owned(), &mut Vec::new())?;
    Ok(builder.finish())
}

fn exact_attribute<'a>(
    element: &'a StateGraphBinaryElement,
    key: &[u8],
) -> Option<&'a StateGraphBinaryAttribute> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.key == key)
}

fn exact_attribute_text<'a>(element: &'a StateGraphBinaryElement, key: &[u8]) -> Option<&'a [u8]> {
    exact_attribute(element, key).map(|attribute| attribute.text.as_slice())
}

fn write_occurrence_json(
    output: &mut String,
    element: &StateGraphBinaryElement,
    locator: &str,
    state_path: &[Vec<u8>],
    preorder_index: usize,
    direct_params: &[(usize, &StateGraphBinaryElement)],
) -> Result<(), Box<dyn Error>> {
    output.push_str("{\"attributes\":");
    write_attributes_json(output, &element.attributes)?;
    output.push_str(",\"children\":[");
    for (index, child) in element.children.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write_json_ascii(output, &child.name)?;
    }
    output.push_str("],\"locator\":");
    write_json_ascii(output, locator.as_bytes())?;
    output.push_str(",\"param_tree\":");
    write_param_tree_json(output, locator, direct_params)?;
    write!(
        output,
        ",\"preorder_index\":{preorder_index},\"state_path\":["
    )?;
    for (index, component) in state_path.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write_json_ascii(output, component)?;
    }
    output.push_str("]}");
    Ok(())
}

fn write_param_tree_json(
    output: &mut String,
    parent_locator: &str,
    direct_params: &[(usize, &StateGraphBinaryElement)],
) -> Result<(), Box<dyn Error>> {
    output.push('[');
    for (position, (child_index, param)) in direct_params.iter().enumerate() {
        if position != 0 {
            output.push(',');
        }
        output.push_str("{\"attributes\":");
        write_attributes_json(output, &param.attributes)?;
        output.push_str(",\"children\":[],\"locator\":");
        write_json_ascii(output, format!("{parent_locator}.{child_index}").as_bytes())?;
        output.push('}');
    }
    output.push(']');
    Ok(())
}

fn write_attributes_json(
    output: &mut String,
    attributes: &[StateGraphBinaryAttribute],
) -> Result<(), Box<dyn Error>> {
    output.push('[');
    for (index, attribute) in attributes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            output,
            "{{\"boolean_raw\":{},\"float_bits\":\"0x{:08X}\",\"key\":",
            attribute.boolean_raw, attribute.float_bits
        )?;
        write_json_ascii(output, &attribute.key)?;
        write!(
            output,
            ",\"key_hash\":\"0x{:08X}\",\"text\":",
            attribute.key_hash
        )?;
        write_json_ascii(output, &attribute.text)?;
        output.push('}');
    }
    output.push(']');
    Ok(())
}

fn write_json_ascii(output: &mut String, value: &[u8]) -> Result<(), Box<dyn Error>> {
    output.push('"');
    for &byte in value {
        match byte {
            b'"' => output.push_str("\\\""),
            b'\\' => output.push_str("\\\\"),
            0x08 => output.push_str("\\b"),
            0x09 => output.push_str("\\t"),
            0x0a => output.push_str("\\n"),
            0x0c => output.push_str("\\f"),
            0x0d => output.push_str("\\r"),
            0x00..=0x1f => write!(output, "\\u{byte:04x}")?,
            0x20..=0x7e => output.push(char::from(byte)),
            0x7f => output.push_str("\\u007f"),
            _ => {
                return Err(format!(
                    "compiled StateGraph JSON field contained non-ASCII byte {byte:02x}"
                )
                .into());
            }
        }
    }
    output.push('"');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_core::play_animation::{AnimationAttributeParamType, AnimationTransitionType};
    use skate2_formats::state_graph::state_graph_string_hash32;

    fn attribute(key: &[u8], text: &[u8]) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits: 0,
            boolean_raw: 0,
        }
    }

    fn element(
        name: &[u8],
        attributes: Vec<StateGraphBinaryAttribute>,
        children: Vec<StateGraphBinaryElement>,
    ) -> StateGraphBinaryElement {
        StateGraphBinaryElement {
            name: name.to_vec(),
            attributes,
            children,
        }
    }

    fn last_anim_param(name: &[u8]) -> StateGraphBinaryElement {
        element(
            b"param",
            vec![
                attribute(b"from", b"lastAnim"),
                attribute(b"attribute", name),
            ],
            Vec::new(),
        )
    }

    #[test]
    fn traversal_selects_exact_behaviour_and_only_direct_params_in_order() {
        let root = element(
            b"state",
            vec![attribute(b"name", b"Root")],
            vec![
                element(
                    b"behaviour",
                    vec![
                        attribute(b"name", b"PlayAnimation"),
                        attribute(b"anim", b"A"),
                    ],
                    vec![
                        last_anim_param(b"First"),
                        element(b"wrapper", Vec::new(), vec![last_anim_param(b"Nested")]),
                        last_anim_param(b"Second"),
                    ],
                ),
                element(
                    b"behaviour",
                    vec![
                        attribute(b"name", b"playanimation"),
                        attribute(b"anim", b"Ignored"),
                    ],
                    vec![last_anim_param(b"Ignored")],
                ),
            ],
        );
        let summary = summarize(&root).unwrap();
        assert_eq!(summary.behaviours, 1);
        assert_eq!(summary.params, 2);
        assert_eq!(summary.param_type_counts, [2, 0, 0]);
    }

    #[test]
    fn canonical_writer_matches_sorted_python_json_shape() {
        let attributes = [StateGraphBinaryAttribute {
            key: b"k\"".to_vec(),
            key_hash: 0x0123_abcd,
            text: b"x\n\x7f".to_vec(),
            float_bits: 0x8000_0000,
            boolean_raw: 1,
        }];
        let mut output = String::new();
        write_attributes_json(&mut output, &attributes).unwrap();
        assert_eq!(
            output,
            r#"[{"boolean_raw":1,"float_bits":"0x80000000","key":"k\"","key_hash":"0x0123ABCD","text":"x\n\u007f"}]"#
        );

        let param = last_anim_param(b"Speed");
        let behaviour = element(
            b"behaviour",
            vec![
                attribute(b"name", b"PlayAnimation"),
                attribute(b"anim", b"A"),
            ],
            vec![param],
        );
        let direct = vec![(0, &behaviour.children[0])];
        let mut occurrence = String::new();
        write_occurrence_json(
            &mut occurrence,
            &behaviour,
            "0.2",
            &[b"Root".to_vec()],
            7,
            &direct,
        )
        .unwrap();
        assert_eq!(
            occurrence,
            r#"{"attributes":[{"boolean_raw":0,"float_bits":"0x00000000","key":"name","key_hash":"0xBA41A46A","text":"PlayAnimation"},{"boolean_raw":0,"float_bits":"0x00000000","key":"anim","key_hash":"0x5304EE81","text":"A"}],"children":["param"],"locator":"0.2","param_tree":[{"attributes":[{"boolean_raw":0,"float_bits":"0x00000000","key":"from","key_hash":"0x2EA45B13","text":"lastAnim"},{"boolean_raw":0,"float_bits":"0x00000000","key":"attribute","key_hash":"0x4EB5A937","text":"Speed"}],"children":[],"locator":"0.2.0"}],"preorder_index":7,"state_path":["Root"]}"#
        );
    }

    #[test]
    fn direct_param_with_nested_content_is_rejected_before_digesting() {
        let root = element(
            b"state",
            vec![attribute(b"name", b"Root")],
            vec![element(
                b"behaviour",
                vec![
                    attribute(b"name", b"PlayAnimation"),
                    attribute(b"anim", b"A"),
                ],
                vec![element(
                    b"param",
                    vec![
                        attribute(b"from", b"lastAnim"),
                        attribute(b"attribute", b"Speed"),
                    ],
                    vec![element(b"unexpected", Vec::new(), Vec::new())],
                )],
            )],
        );
        assert!(
            summarize(&root)
                .unwrap_err()
                .to_string()
                .contains("retained nested children")
        );
    }

    #[test]
    fn frozen_summary_matcher_rejects_nearby_mutation() {
        let mut summary = PlayAnimationCorpusSummary {
            behaviours: 807,
            params: 165,
            occurrence_sha256: PLAY_ANIMATION_CORPUS_OCCURRENCES_SHA256.to_owned(),
            param_tree_sha256: PLAY_ANIMATION_CORPUS_PARAM_TREE_SHA256.to_owned(),
            transition_type_counts: [0, 3, 652, 65, 87],
            switch_anim_present: 8,
            mirror_anim_present: 28,
            no_board_anim_present: 4,
            blend_with_current_frame_true: 46,
            transition_under_true: 140,
            use_channel_from_weights_true: 64,
            playback_speed_default: 744,
            blend_time_default: 87,
            param_type_counts: [111, 0, 54],
            ignored_trans_blend: 30,
            ignored_param_set_blend_true: 143,
        };
        assert!(summary.matches_shipped());
        summary.params += 1;
        assert!(!summary.matches_shipped());
    }

    #[test]
    fn transition_enum_indexes_remain_the_frozen_distribution_order() {
        assert_eq!(AnimationTransitionType::Default as usize, 0);
        assert_eq!(AnimationTransitionType::Play as usize, 1);
        assert_eq!(AnimationTransitionType::Blend as usize, 2);
        assert_eq!(AnimationTransitionType::ChannelBlend as usize, 3);
        assert_eq!(AnimationTransitionType::Sequence as usize, 4);
        assert_eq!(AnimationAttributeParamType::FromLastAnim as usize, 0);
        assert_eq!(AnimationAttributeParamType::FromIntent as usize, 1);
        assert_eq!(AnimationAttributeParamType::FromFilteredIntent as usize, 2);
    }
}
