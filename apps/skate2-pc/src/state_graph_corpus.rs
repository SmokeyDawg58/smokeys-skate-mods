//! Shipped compiled `StateGraph` payload corpus.
//!
//! This validates the recovered binary grammar and native attribute-hash
//! representation. Runtime factory construction and graph execution remain
//! outside this collaborator.

use skate2_formats::{mount::MountSet, state_graph::StateGraphBinaryDocument};
use std::{collections::BTreeMap, error::Error};

pub(super) const EXPECTED: [(&str, usize, usize, usize, usize); 36] = [
    (
        r"data\livingworld\PluginContent\actortracker_spectate.stategraph",
        2_254,
        47,
        62,
        6,
    ),
    (
        r"data\livingworld\PluginContent\conversation.stategraph",
        3_333,
        55,
        96,
        8,
    ),
    (
        r"data\livingworld\PluginContent\lookat.stategraph",
        336,
        9,
        9,
        4,
    ),
    (
        r"data\livingworld\PluginContent\sit.stategraph",
        3_971,
        71,
        109,
        9,
    ),
    (
        r"data\livingworld\PluginContent\spectate.stategraph",
        2_376,
        49,
        60,
        5,
    ),
    (
        r"data\livingworld\PluginContent\useatm.stategraph",
        2_412,
        39,
        72,
        6,
    ),
    (
        r"data\livingworld\PluginContent\usenewspaperbox.stategraph",
        2_379,
        39,
        70,
        7,
    ),
    (
        r"data\livingworld\PluginContent\usetrashbin.stategraph",
        271,
        7,
        7,
        3,
    ),
    (
        r"data\livingworld\PluginContent\usevendingmachine.stategraph",
        2_570,
        42,
        75,
        7,
    ),
    (
        r"data\livingworld\PluginContent\usewaterfountain.stategraph",
        2_949,
        47,
        85,
        8,
    ),
    (
        r"data\state\ActionGraph_OnBoard.stategraph",
        92_825,
        1_494,
        2_904,
        11,
    ),
    (
        r"data\state\MotionGraph_OnBoard.stategraph",
        641_241,
        10_602,
        20_419,
        12,
    ),
    (
        r"data\state\livingworldentities\vehicle\Vehicle.stategraph",
        2_960,
        68,
        62,
        5,
    ),
    (
        r"data\state\livingworldentities\pedestrian\Big_Black.stategraph",
        5_228,
        85,
        146,
        8,
    ),
    (
        r"data\state\livingworldentities\pedestrian\MotionGraph_Pedestrian.stategraph",
        155_255,
        2_947,
        4_264,
        9,
    ),
    (
        r"data\state\livingworldentities\pedestrian\motiongraph_test.stategraph",
        183,
        3,
        6,
        2,
    ),
    (
        r"data\state\livingworldentities\pedestrian\Pedestrian.stategraph",
        38_817,
        641,
        1_101,
        11,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\actortracker_spectate.stategraph",
        2_254,
        47,
        62,
        6,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\conversation.stategraph",
        3_706,
        60,
        107,
        8,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\lookat.stategraph",
        336,
        9,
        9,
        4,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\sit.stategraph",
        3_388,
        57,
        97,
        8,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\spectate.stategraph",
        2_376,
        49,
        60,
        5,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\useatm.stategraph",
        2_733,
        43,
        82,
        6,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\usenewspaperbox.stategraph",
        2_752,
        44,
        81,
        7,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\usetrashbin.stategraph",
        323,
        8,
        8,
        3,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\usevendingmachine.stategraph",
        2_943,
        47,
        86,
        7,
    ),
    (
        r"data\state\livingworldentities\pedestrian\plugin\usewaterfountain.stategraph",
        3_270,
        51,
        95,
        8,
    ),
    (
        r"data\state\livingworldentities\marquee\Marquee.stategraph",
        749,
        17,
        20,
        5,
    ),
    (
        r"data\state\livingworldentities\marquee\Motiongraph_Marquee.stategraph",
        2_492,
        44,
        72,
        8,
    ),
    (
        r"data\state\livingworldentities\marquee\Motiongraph_Photographer.stategraph",
        4_000,
        71,
        113,
        8,
    ),
    (
        r"data\state\livingworldentities\marquee\Photographer.stategraph",
        1_257,
        27,
        32,
        5,
    ),
    (
        r"data\script\camera\Default_cameragraph.stategraph",
        21_341,
        357,
        687,
        7,
    ),
    (
        r"data\script\camera\replay01_cameragraph.stategraph",
        1_281,
        23,
        42,
        6,
    ),
    (
        r"data\script\camera\replay02_cameragraph.stategraph",
        577,
        12,
        18,
        4,
    ),
    (
        r"data\script\camera\replay03_cameragraph.stategraph",
        1_282,
        23,
        42,
        6,
    ),
    (
        r"data\script\camera\replay04_cameragraph.stategraph",
        1_284,
        23,
        42,
        6,
    ),
];

const EXPECTED_TAGS: [(&str, usize); 7] = [
    ("behaviour", 3_828),
    ("condition", 4_485),
    ("expression", 3_561),
    ("hook", 80),
    ("param", 165),
    ("state", 2_681),
    ("transition", 2_457),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateGraphCorpusSummary {
    pub graphs: usize,
    pub decoded_bytes: usize,
    pub nodes: usize,
    pub attributes: usize,
    pub maximum_depth: usize,
}

impl StateGraphCorpusSummary {
    #[must_use]
    pub const fn matches_shipped(&self) -> bool {
        self.graphs == 36
            && self.decoded_bytes == 1_017_704
            && self.nodes == 17_257
            && self.attributes == 31_302
            && self.maximum_depth == 12
    }
}

/// Parses every named compiled `StateGraph` in the mounted shipped corpus and
/// validates its exact structural oracle.
///
/// # Errors
///
/// Returns a mount, binary grammar, exact-consumption, structure, tag, boolean,
/// duplicate-key, or within-element hash-collision mismatch.
pub fn load(mounts: &MountSet) -> Result<StateGraphCorpusSummary, Box<dyn Error>> {
    let mut summary = StateGraphCorpusSummary {
        graphs: 0,
        decoded_bytes: 0,
        nodes: 0,
        attributes: 0,
        maximum_depth: 0,
    };
    let mut tags = BTreeMap::<Vec<u8>, usize>::new();
    let mut false_booleans = 0_usize;
    let mut true_booleans = 0_usize;
    let mut maximum_attributes = 0_usize;
    let mut maximum_children = 0_usize;

    for &(path, expected_bytes, expected_nodes, expected_attributes, expected_depth) in &EXPECTED {
        let bytes = mounts.read_path(path)?;
        if bytes.len() != expected_bytes {
            return Err(format!(
                "compiled StateGraph {path} had {} bytes, expected {expected_bytes}",
                bytes.len()
            )
            .into());
        }
        let document = StateGraphBinaryDocument::parse_exact(&bytes)?;
        if document.root.name != b"state" {
            return Err(format!(
                "compiled StateGraph {path} root was {:?}",
                document.root.name
            )
            .into());
        }
        let nodes = document.root.node_count();
        let attributes = document.root.attribute_count();
        let depth = document.root.maximum_depth();
        if (nodes, attributes, depth) != (expected_nodes, expected_attributes, expected_depth) {
            return Err(format!(
                "compiled StateGraph {path} structure was ({nodes}, {attributes}, {depth}), expected ({expected_nodes}, {expected_attributes}, {expected_depth})"
            )
            .into());
        }
        for element in document.root.preorder() {
            *tags.entry(element.name.clone()).or_default() += 1;
            maximum_attributes = maximum_attributes.max(element.attributes.len());
            maximum_children = maximum_children.max(element.children.len());
            let mut hashes = BTreeMap::<u32, &[u8]>::new();
            for attribute in &element.attributes {
                match attribute.boolean_raw {
                    0 => false_booleans += 1,
                    1 => true_booleans += 1,
                    other => {
                        return Err(format!(
                            "compiled StateGraph {path} had non-boolean byte {other}"
                        )
                        .into());
                    }
                }
                if let Some(previous) = hashes.insert(attribute.key_hash, &attribute.key) {
                    let kind = if previous == attribute.key {
                        "duplicate key"
                    } else {
                        "attribute hash collision"
                    };
                    return Err(format!(
                        "compiled StateGraph {path} had {kind} {:?}/{:?}",
                        previous, attribute.key
                    )
                    .into());
                }
            }
        }
        summary.graphs += 1;
        summary.decoded_bytes += bytes.len();
        summary.nodes += nodes;
        summary.attributes += attributes;
        summary.maximum_depth = summary.maximum_depth.max(depth);
    }

    let expected_tags = EXPECTED_TAGS
        .into_iter()
        .map(|(name, count)| (name.as_bytes().to_vec(), count))
        .collect::<BTreeMap<_, _>>();
    if tags != expected_tags {
        return Err(format!("compiled StateGraph tag census differed: {tags:?}").into());
    }
    if (false_booleans, true_booleans) != (30_163, 1_139) {
        return Err(format!(
            "compiled StateGraph boolean census was ({false_booleans}, {true_booleans})"
        )
        .into());
    }
    if (maximum_attributes, maximum_children) != (8, 38) {
        return Err(format!(
            "compiled StateGraph maxima were ({maximum_attributes}, {maximum_children})"
        )
        .into());
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_state_graph_oracle_totals_are_frozen() {
        assert_eq!(EXPECTED.len(), 36);
        assert_eq!(
            EXPECTED.iter().map(|entry| entry.1).sum::<usize>(),
            1_017_704
        );
        assert_eq!(EXPECTED.iter().map(|entry| entry.2).sum::<usize>(), 17_257);
        assert_eq!(EXPECTED.iter().map(|entry| entry.3).sum::<usize>(), 31_302);
        assert_eq!(EXPECTED.iter().map(|entry| entry.4).max(), Some(12));
        assert_eq!(
            EXPECTED_TAGS.iter().map(|entry| entry.1).sum::<usize>(),
            17_257
        );
    }
}
