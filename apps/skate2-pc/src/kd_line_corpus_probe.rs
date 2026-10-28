use skate2_formats::{
    collision::{BranchNode, ClusteredMesh, KdTree, KdTreeMappedArray, ObjectType},
    mount::MountSet,
    rw4::Arena,
    stream::{CollectionMap, StreamFile, decode_arena},
};
use std::error::Error;

// Production comparator for the independently frozen public projection.
// This file contains no local line clipping or KD traversal. All leaf ranges
// come from KdTree::line_range_query / next_range / clip_end at commit 43a690a.
// External independent oracle source SHA-256:
// 6A5A44B86CC0D4105BECF2012C0BB7398B9BF337F367BE28A90CEA01DD7579C9.
pub(crate) const CLI_FLAG: &str = "--inventory-bam-sim-kd-line";
const DOMAIN: &[u8] = b"SKATE2/COLLISION/KD_LINE_CLIPPER_PUBLIC\0V1\0";
const EXPECTED_SIGNATURE: u64 = 0x57c0_3f40_e465_e5f7;
const EXPECTED_BYTES: u64 = 70_139_064;

#[derive(Debug, Default, Eq, PartialEq)]
struct Counts {
    collections: u32,
    arenas: u32,
    trees: u32,
    mapped_trees: u32,
    mesh_trees: u32,
    branchless_trees: u32,
    branches: u64,
    entries: u64,
    real_probes: u64,
    synthetic_probes: u32,
    leaf_ranges: u64,
    leaf_units: u64,
    clip_end_calls: u32,
}

const EXPECTED_COUNTS: Counts = Counts {
    collections: 284,
    arenas: 2_135,
    trees: 1_825,
    mapped_trees: 1_033,
    mesh_trees: 792,
    branchless_trees: 212,
    branches: 1_238_312,
    entries: 4_643_391,
    real_probes: 12_775,
    synthetic_probes: 16,
    leaf_ranges: 886_115,
    leaf_units: 4_804_738,
    clip_end_calls: 2,
};

pub(crate) fn requested<I, S>(arguments: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    arguments
        .into_iter()
        .any(|argument| argument.as_ref() == CLI_FLAG)
}

struct Protocol {
    hash: u64,
    bytes: u64,
}

impl Protocol {
    fn new() -> Self {
        let mut result = Self {
            hash: 0xcbf2_9ce4_8422_2325,
            bytes: 0,
        };
        result.bytes(DOMAIN);
        result
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash ^= u64::from(*byte);
            self.hash = self.hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.bytes += bytes.len() as u64;
    }

    fn tag(&mut self, tag: u8) {
        self.bytes(&[tag]);
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_be_bytes());
    }

    fn f32(&mut self, value: f32) {
        self.u32(value.to_bits());
    }
}

#[derive(Clone, Copy)]
struct Line {
    start: [f32; 3],
    end: [f32; 3],
    fatness: f32,
}

#[allow(clippy::too_many_arguments)]
fn record_tree_header(
    protocol: &mut Protocol,
    tree_ordinal: u32,
    collection_ordinal: u32,
    arena_ordinal: u32,
    dictionary_ordinal: u32,
    outer_id: u64,
    inner_id: u32,
    object_type: u32,
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
    branch_count: u32,
    entry_count: u32,
) {
    protocol.tag(b'T');
    protocol.u32(tree_ordinal);
    protocol.u32(collection_ordinal);
    protocol.u32(arena_ordinal);
    protocol.u32(dictionary_ordinal);
    protocol.u64(outer_id);
    protocol.u32(inner_id);
    protocol.u32(object_type);
    for value in bbox_min.into_iter().chain(bbox_max) {
        protocol.f32(value);
    }
    protocol.u32(branch_count);
    protocol.u32(entry_count);
}

fn record_nodes(protocol: &mut Protocol, tree_ordinal: u32, nodes: &[BranchNode]) {
    for (node_ordinal, node) in nodes.iter().enumerate() {
        protocol.tag(b'N');
        protocol.u32(tree_ordinal);
        protocol.u32(u32::try_from(node_ordinal).expect("node ordinal fits u32"));
        protocol.u32(node.parent);
        protocol.u32(node.axis);
        for child in 0..2 {
            protocol.u32(node.child_content[child]);
            protocol.u32(node.child_index[child]);
        }
        protocol.f32(node.extents[0]);
        protocol.f32(node.extents[1]);
    }
}

#[allow(clippy::too_many_arguments)]
fn record_probe(
    protocol: &mut Protocol,
    counts: &mut Counts,
    tree_ordinal: u32,
    probe_kind: u32,
    tree: &KdTree,
    object: &[u8],
    line: Line,
    synthetic: bool,
) -> Result<Vec<(u32, u32)>, Box<dyn Error>> {
    protocol.tag(b'P');
    protocol.u32(tree_ordinal);
    protocol.u32(probe_kind);
    for value in line.start.into_iter().chain(line.end) {
        protocol.f32(value);
    }
    protocol.f32(line.fatness);
    let mut query = tree.line_range_query(object, line.start, line.end, line.fatness)?;
    let mut result = Vec::new();
    while let Some(range) = query.next_range()? {
        protocol.tag(b'L');
        protocol.u32(tree_ordinal);
        protocol.u32(probe_kind);
        protocol.u32(u32::try_from(result.len())?);
        protocol.u32(range.encoded_start);
        protocol.u32(range.unit_count);
        result.push((range.encoded_start, range.unit_count));
        counts.leaf_ranges += 1;
        counts.leaf_units += u64::from(range.unit_count);
    }
    protocol.tag(b'E');
    protocol.u32(tree_ordinal);
    protocol.u32(probe_kind);
    protocol.u32(u32::try_from(result.len())?);
    if synthetic {
        counts.synthetic_probes += 1;
    } else {
        counts.real_probes += 1;
    }
    Ok(result)
}

#[allow(clippy::manual_midpoint)] // Protocol V1 freezes this exact add-then-multiply order.
fn center(minimum: [f32; 3], maximum: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| (minimum[axis] + maximum[axis]) * 0.5)
}

fn real_lines(bbox_min: [f32; 3], bbox_max: [f32; 3]) -> [Line; 7] {
    let middle = center(bbox_min, bbox_max);
    let mut lines = [
        Line {
            start: bbox_min,
            end: bbox_max,
            fatness: 4.0,
        },
        Line {
            start: bbox_min,
            end: bbox_max,
            fatness: 0.0,
        },
        Line {
            start: bbox_max,
            end: bbox_min,
            fatness: 0.0,
        },
        Line {
            start: middle,
            end: middle,
            fatness: 0.0,
        },
        Line {
            start: middle,
            end: middle,
            fatness: 0.0,
        },
        Line {
            start: middle,
            end: middle,
            fatness: 0.0,
        },
        Line {
            start: middle,
            end: middle,
            fatness: 0.0,
        },
    ];
    lines[3].start[0] = bbox_min[0];
    lines[3].end[0] = bbox_max[0];
    lines[4].start[1] = bbox_min[1];
    lines[4].end[1] = bbox_max[1];
    lines[5].start[2] = bbox_min[2];
    lines[5].end[2] = bbox_max[2];
    lines
}

fn node(extent_low: f32, extent_high: f32, low: (u32, u32), high: (u32, u32)) -> BranchNode {
    BranchNode {
        parent: u32::MAX,
        axis: 0,
        child_content: [low.0, high.0],
        child_index: [low.1, high.1],
        extents: [extent_low, extent_high],
    }
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn synthetic_object(nodes: &[BranchNode]) -> Vec<u8> {
    let mut bytes = vec![0_u8; nodes.len() * 32];
    for (ordinal, node) in nodes.iter().enumerate() {
        let base = ordinal * 32;
        write_u32(&mut bytes, base, node.parent);
        write_u32(&mut bytes, base + 4, node.axis);
        write_u32(&mut bytes, base + 8, node.child_content[0]);
        write_u32(&mut bytes, base + 12, node.child_index[0]);
        write_u32(&mut bytes, base + 16, node.child_content[1]);
        write_u32(&mut bytes, base + 20, node.child_index[1]);
        write_u32(&mut bytes, base + 24, node.extents[0].to_bits());
        write_u32(&mut bytes, base + 28, node.extents[1].to_bits());
    }
    bytes
}

fn synthetic_tree(
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
    nodes: &[BranchNode],
    entry_count: u32,
) -> (KdTree, Vec<u8>) {
    let mut min = [0.0_f32; 4];
    let mut max = [0.0_f32; 4];
    min[..3].copy_from_slice(&bbox_min);
    max[..3].copy_from_slice(&bbox_max);
    (
        KdTree {
            branch_nodes_offset: 0,
            branch_node_count: u32::try_from(nodes.len()).expect("synthetic nodes fit u32"),
            entry_count,
            bounding_box_min: min,
            bounding_box_max: max,
        },
        synthetic_object(nodes),
    )
}

fn record_synthetic_tree(
    protocol: &mut Protocol,
    case: u32,
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
    nodes: &[BranchNode],
    entry_count: u32,
) -> u32 {
    let tree = 0x8000_0000 | case;
    record_tree_header(
        protocol,
        tree,
        u32::MAX,
        u32::MAX,
        case,
        u64::from(case),
        case,
        0,
        bbox_min,
        bbox_max,
        u32::try_from(nodes.len()).expect("synthetic node count fits"),
        entry_count,
    );
    record_nodes(protocol, tree, nodes);
    tree
}

fn assert_starts(actual: &[(u32, u32)], expected: &[u32]) -> Result<(), Box<dyn Error>> {
    let starts: Vec<_> = actual.iter().map(|range| range.0).collect();
    if starts != expected {
        return Err(format!("production synthetic starts {starts:?} != {expected:?}").into());
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // The authored synthetic protocol order is intentionally linear.
fn run_synthetics(protocol: &mut Protocol, counts: &mut Counts) -> Result<(), Box<dyn Error>> {
    let bbox_min = [0.0, -1.0, -1.0];
    let bbox_max = [1.0, 1.0, 1.0];
    let positive = Line {
        start: [0.0, 0.0, 0.0],
        end: [1.0, 0.0, 0.0],
        fatness: -0.000_001,
    };
    let negative = Line {
        start: [1.0, 0.0, 0.0],
        end: [0.0, 0.0, 0.0],
        fatness: -0.000_001,
    };
    let cases = [
        (node(0.5, 0.5, (1, 10), (1, 20)), positive, vec![10, 20]),
        (node(0.5, 0.5, (1, 10), (1, 20)), negative, vec![20, 10]),
        (node(0.5, 1.0, (1, 10), (1, 20)), positive, vec![10]),
        (node(0.0, 0.5, (1, 10), (1, 20)), positive, vec![20]),
        (node(0.5, 0.5, (0, 10), (1, 20)), positive, vec![20]),
    ];
    for (case, (node, line, expected)) in cases.into_iter().enumerate() {
        let case = u32::try_from(case)?;
        let nodes = [node];
        let tree_id = record_synthetic_tree(protocol, case, bbox_min, bbox_max, &nodes, 2);
        let (tree, object) = synthetic_tree(bbox_min, bbox_max, &nodes, 2);
        let ranges = record_probe(protocol, counts, tree_id, case, &tree, &object, line, true)?;
        assert_starts(&ranges, &expected)?;
    }

    let static_bbox_min = [-1.0; 3];
    let static_bbox_max = [1.0; 3];
    let shared_node = node(0.0, 0.0, (1, 10), (1, 20));
    let lines = [
        Line {
            start: [0.0, 0.0, 0.0],
            end: [0.0, 1.0, 0.0],
            fatness: 0.0,
        },
        Line {
            start: [2.0, 0.0, 0.0],
            end: [2.0, 1.0, 0.0],
            fatness: 0.0,
        },
        Line {
            start: [0.0; 3],
            end: [0.0; 3],
            fatness: 0.0,
        },
        Line {
            start: [2.0; 3],
            end: [2.0; 3],
            fatness: 0.0,
        },
        Line {
            start: [-0.0, 0.0, -0.0],
            end: [0.0, 1.0, 0.0],
            fatness: 0.0,
        },
    ];
    let expected = [vec![10, 20], vec![], vec![10, 20], vec![], vec![10, 20]];
    for (offset, (line, expected)) in lines.into_iter().zip(expected).enumerate() {
        let case = 5 + u32::try_from(offset)?;
        let nodes = [shared_node.clone()];
        let tree_id =
            record_synthetic_tree(protocol, case, static_bbox_min, static_bbox_max, &nodes, 2);
        let (tree, object) = synthetic_tree(static_bbox_min, static_bbox_max, &nodes, 2);
        let ranges = record_probe(protocol, counts, tree_id, case, &tree, &object, line, true)?;
        assert_starts(&ranges, &expected)?;
    }

    for (offset, entry_count) in [7_u32, 0].into_iter().enumerate() {
        let case = 10 + u32::try_from(offset)?;
        let tree_id = record_synthetic_tree(
            protocol,
            case,
            static_bbox_min,
            static_bbox_max,
            &[],
            entry_count,
        );
        let (tree, object) = synthetic_tree(static_bbox_min, static_bbox_max, &[], entry_count);
        let ranges = record_probe(
            protocol,
            counts,
            tree_id,
            case,
            &tree,
            &object,
            Line {
                start: static_bbox_min,
                end: static_bbox_max,
                fatness: 0.0,
            },
            true,
        )?;
        let expected = if entry_count == 0 { vec![] } else { vec![0] };
        assert_starts(&ranges, &expected)?;
    }

    let root_lines = [
        (
            Line {
                start: [-2.0, -0.5, -0.5],
                end: [-1.0, 0.5, 0.5],
                fatness: -0.000_000_5,
            },
            vec![],
        ),
        (
            Line {
                start: [-2.0, -0.5, -0.5],
                end: [-0.999, 0.5, 0.5],
                fatness: -0.000_000_5,
            },
            vec![0],
        ),
    ];
    for (offset, (line, expected)) in root_lines.into_iter().enumerate() {
        let case = 12 + u32::try_from(offset)?;
        let tree_id =
            record_synthetic_tree(protocol, case, static_bbox_min, static_bbox_max, &[], 1);
        let (tree, object) = synthetic_tree(static_bbox_min, static_bbox_max, &[], 1);
        let ranges = record_probe(protocol, counts, tree_id, case, &tree, &object, line, true)?;
        assert_starts(&ranges, &expected)?;
    }

    for (offset, clip) in [0.5_f32, f32::from_bits(0.5_f32.to_bits() - 1)]
        .into_iter()
        .enumerate()
    {
        let case = 14 + u32::try_from(offset)?;
        let nodes = [node(0.5, 0.5, (1, 10), (1, 20))];
        let tree_id = record_synthetic_tree(protocol, case, bbox_min, bbox_max, &nodes, 2);
        let (tree, object) = synthetic_tree(bbox_min, bbox_max, &nodes, 2);
        protocol.tag(b'P');
        protocol.u32(tree_id);
        protocol.u32(case);
        for value in positive.start.into_iter().chain(positive.end) {
            protocol.f32(value);
        }
        protocol.f32(positive.fatness);
        let mut query =
            tree.line_range_query(&object, positive.start, positive.end, positive.fatness)?;
        let first = query
            .next_range()?
            .ok_or("production ClipEnd first leaf missing")?;
        protocol.tag(b'L');
        protocol.u32(tree_id);
        protocol.u32(case);
        protocol.u32(0);
        protocol.u32(first.encoded_start);
        protocol.u32(first.unit_count);
        counts.leaf_ranges += 1;
        counts.leaf_units += u64::from(first.unit_count);
        protocol.tag(b'K');
        protocol.u32(tree_id);
        protocol.u32(case);
        protocol.u32(1);
        protocol.f32(clip);
        query.clip_end(clip);
        let mut ranges = vec![(first.encoded_start, first.unit_count)];
        while let Some(range) = query.next_range()? {
            protocol.tag(b'L');
            protocol.u32(tree_id);
            protocol.u32(case);
            protocol.u32(u32::try_from(ranges.len())?);
            protocol.u32(range.encoded_start);
            protocol.u32(range.unit_count);
            counts.leaf_ranges += 1;
            counts.leaf_units += u64::from(range.unit_count);
            ranges.push((range.encoded_start, range.unit_count));
        }
        protocol.tag(b'E');
        protocol.u32(tree_id);
        protocol.u32(case);
        protocol.u32(u32::try_from(ranges.len())?);
        let expected: &[u32] = if offset == 0 { &[10, 20] } else { &[10] };
        assert_starts(&ranges, expected)?;
        counts.synthetic_probes += 1;
        counts.clip_end_calls += 1;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Corpus iteration stays visibly aligned with the frozen protocol.
pub(crate) fn run(mounts: &MountSet) -> Result<(), Box<dyn Error>> {
    let base = "data\\content\\world\\stream\\BAM\\BAM_Sim";
    let map = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let mut protocol = Protocol::new();
    let mut counts = Counts::default();
    for (collection_ordinal, collection) in map.collections.iter().enumerate() {
        counts.collections += 1;
        let path = format!(
            "data\\content\\world\\stream\\BAM\\{}.xsf",
            collection.filename
        );
        let bytes = mounts.read_path(&path)?;
        let file = StreamFile::parse(&bytes, collection.asset_count)?;
        for (arena_ordinal, block) in file.assets.iter().enumerate() {
            counts.arenas += 1;
            let decoded = decode_arena(&bytes[block.data_range.clone()])?;
            let arena = Arena::parse(&decoded.resources)?;
            for (dictionary_ordinal, entry) in arena.dictionary.iter().enumerate() {
                let Some(object) = arena.object_bytes(&decoded.resources, entry) else {
                    return Err("object range missing".into());
                };
                let (object_type, tree) = match ObjectType::from_type_id(entry.type_id) {
                    Some(ObjectType::KdTreeMappedArray) => {
                        counts.mapped_trees += 1;
                        let mapped = KdTreeMappedArray::parse(object)?;
                        (entry.type_id, mapped.kd_tree(object)?)
                    }
                    Some(ObjectType::ClusteredMesh) => {
                        counts.mesh_trees += 1;
                        let mesh = ClusteredMesh::parse(object)?;
                        (entry.type_id, mesh.kd_tree(object)?)
                    }
                    _ => continue,
                };
                let nodes = tree.branch_nodes(object)?;
                let tree_ordinal = counts.trees;
                counts.trees += 1;
                counts.branchless_trees += u32::from(nodes.is_empty());
                counts.branches += u64::from(tree.branch_node_count);
                counts.entries += u64::from(tree.entry_count);
                let bbox_min = [
                    tree.bounding_box_min[0],
                    tree.bounding_box_min[1],
                    tree.bounding_box_min[2],
                ];
                let bbox_max = [
                    tree.bounding_box_max[0],
                    tree.bounding_box_max[1],
                    tree.bounding_box_max[2],
                ];
                record_tree_header(
                    &mut protocol,
                    tree_ordinal,
                    u32::try_from(collection_ordinal)?,
                    u32::try_from(arena_ordinal)?,
                    u32::try_from(dictionary_ordinal)?,
                    block.id,
                    arena.id,
                    object_type,
                    bbox_min,
                    bbox_max,
                    tree.branch_node_count,
                    tree.entry_count,
                );
                record_nodes(&mut protocol, tree_ordinal, &nodes);
                for (probe, line) in real_lines(bbox_min, bbox_max).into_iter().enumerate() {
                    let _ = record_probe(
                        &mut protocol,
                        &mut counts,
                        tree_ordinal,
                        u32::try_from(probe)?,
                        &tree,
                        object,
                        line,
                        false,
                    )?;
                }
            }
        }
    }
    run_synthetics(&mut protocol, &mut counts)?;
    protocol.tag(b'Q');
    protocol.u32(counts.collections);
    protocol.u32(counts.arenas);
    protocol.u32(counts.trees);
    protocol.u32(counts.mapped_trees);
    protocol.u32(counts.mesh_trees);
    protocol.u32(counts.branchless_trees);
    protocol.u64(counts.branches);
    protocol.u64(counts.entries);
    protocol.u64(counts.real_probes);
    protocol.u32(counts.synthetic_probes);
    protocol.u64(counts.leaf_ranges);
    protocol.u64(counts.leaf_units);
    protocol.u32(counts.clip_end_calls);

    println!(
        "collections={} arenas={} trees={} mapped_trees={} mesh_trees={} branchless_trees={} branches={} entries={}",
        counts.collections,
        counts.arenas,
        counts.trees,
        counts.mapped_trees,
        counts.mesh_trees,
        counts.branchless_trees,
        counts.branches,
        counts.entries,
    );
    println!(
        "real_probes={} synthetic_probes={} leaf_ranges={} leaf_units={} clip_end_calls={}",
        counts.real_probes,
        counts.synthetic_probes,
        counts.leaf_ranges,
        counts.leaf_units,
        counts.clip_end_calls,
    );
    println!(
        "public_protocol_bytes={} public_protocol_signature={:016x}",
        protocol.bytes, protocol.hash
    );
    if counts != EXPECTED_COUNTS {
        return Err(format!(
            "production corpus counters changed: {counts:?}; expected {EXPECTED_COUNTS:?}"
        )
        .into());
    }
    if protocol.bytes != EXPECTED_BYTES || protocol.hash != EXPECTED_SIGNATURE {
        return Err(format!(
            "production mismatch: bytes={} signature={:016x}; expected bytes={} signature={:016x}",
            protocol.bytes, protocol.hash, EXPECTED_BYTES, EXPECTED_SIGNATURE
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_projection_contract_is_frozen() {
        assert_eq!(CLI_FLAG, "--inventory-bam-sim-kd-line");
        assert_eq!(DOMAIN, b"SKATE2/COLLISION/KD_LINE_CLIPPER_PUBLIC\0V1\0");
        assert_eq!(EXPECTED_SIGNATURE, 0x57c0_3f40_e465_e5f7);
        assert_eq!(EXPECTED_BYTES, 70_139_064);
        assert_eq!(Protocol::new().bytes, DOMAIN.len() as u64);
        assert_eq!(EXPECTED_COUNTS.trees, 1_825);
        assert_eq!(EXPECTED_COUNTS.real_probes, 12_775);
        assert_eq!(EXPECTED_COUNTS.synthetic_probes, 16);
    }

    #[test]
    fn cli_flag_is_opt_in() {
        assert!(requested(["skate2-pc", CLI_FLAG]));
        assert!(!requested(["skate2-pc", "--inventory-bam-sim"]));
        assert!(!requested(["skate2-pc"]));
    }
}
