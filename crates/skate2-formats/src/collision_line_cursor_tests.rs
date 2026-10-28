use super::*;
use crate::collision::{KdAxisAlignedLineClipper, KdLineStackElement, UnitType};
use std::convert::Infallible;

fn fixture(streams: &[&[u8]]) -> (ClusteredMesh, Vec<u8>) {
    let mut bytes = vec![0_u8; 0x80];
    bytes[52..56].copy_from_slice(&0x60_u32.to_be_bytes());
    bytes[62] = 1;
    bytes[63] = 2;
    bytes[64..68].copy_from_slice(&u32::try_from(streams.len()).unwrap().to_be_bytes());
    bytes[68..72].copy_from_slice(&u32::try_from(streams.len()).unwrap().to_be_bytes());
    for (index, stream) in streams.iter().enumerate() {
        let start = bytes.len();
        bytes[0x60 + index * 4..0x64 + index * 4]
            .copy_from_slice(&u32::try_from(start).unwrap().to_be_bytes());
        bytes.resize(start + 16 + stream.len(), 0);
        let cluster = &mut bytes[start..];
        // Deliberately zero declared unit count: the KD range count is the
        // gameplay authority, not this header or the eventual GP count.
        cluster[2..4].copy_from_slice(&u16::try_from(stream.len()).unwrap().to_be_bytes());
        cluster[8..10].copy_from_slice(&u16::try_from(16 + stream.len()).unwrap().to_be_bytes());
        cluster[16..].copy_from_slice(stream);
    }
    (ClusteredMesh::parse(&bytes).unwrap(), bytes)
}

fn query(ranges: &[(u32, u32)]) -> KdTreeLineRangeQuery<'static> {
    KdTreeLineRangeQuery {
        branch_nodes: crate::collision::KdLineBranches::Parsed(vec![]),
        line_clipper: KdAxisAlignedLineClipper {
            origin: [0.0; 3],
            delta: [1.0; 3],
            recip: [1.0; 3],
            padding: [0.0; 3],
        },
        stack: ranges
            .iter()
            .rev()
            .map(|&(encoded_start, unit_count)| KdLineStackElement {
                content: unit_count,
                index: encoded_start,
                pa: 0.0,
                pb: 1.0,
            })
            .collect(),
        leaf_count: 0,
        next_entry: 0,
    }
}

#[test]
fn crosses_clusters_and_publishes_ceil_half_chunks_using_full_range_count() {
    let (mesh, bytes) = fixture(&[
        &[1, 0, 1, 2, 0, 0, 1, 2],
        &[3, 0, 0, 1, 1, 2, 3, 4, 1, 5, 6, 7],
    ]);
    let mut query = query(&[(0, 5)]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut seen = vec![];
    let mut chunks = vec![];
    while cursor
        .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
            seen.push((
                item.cluster_index,
                item.unit_byte_offset,
                item.unit.unit_type,
            ));
            Ok::<_, Infallible>(())
        })
        .unwrap()
    {
        chunks.push(cursor.progress());
    }
    assert_eq!(
        seen,
        [
            (0, 0, UnitType::Triangle),
            (0, 4, UnitType::OldTriangle),
            (1, 0, UnitType::TriangleList),
            (1, 4, UnitType::Triangle),
            (1, 8, UnitType::Triangle),
        ]
    );
    assert_eq!(
        chunks,
        [
            MeshLineCursorProgress {
                cluster_index: 1,
                unit_byte_offset: 4,
                processed: 3,
                total: 5
            },
            MeshLineCursorProgress {
                cluster_index: 1,
                unit_byte_offset: 8,
                processed: 4,
                total: 5
            },
            MeshLineCursorProgress {
                cluster_index: 2,
                unit_byte_offset: 0,
                processed: 5,
                total: 5
            },
        ]
    );
}

#[test]
fn variable_encoded_sizes_and_culled_units_still_advance_and_roll_over() {
    let (mesh, bytes) = fixture(&[
        &[0x62, 0, 1, 2, 3, 10, 11, 12, 13, 0x7a],
        &[0x81, 1, 2, 3, 0x34, 0x12],
    ]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut query = query(&[(0, 2)]);
    let mut seen = vec![];
    while cursor
        .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
            seen.push((
                item.cluster_index,
                item.encoded_size,
                item.unit.group_id,
                item.unit.surface_id,
            ));
            // Models a GP builder's successful cull: zero emitted primitives.
            Ok::<_, Infallible>(())
        })
        .unwrap()
    {}
    assert_eq!(
        seen,
        [(0, 10, Some(0x7a), None), (1, 6, None, Some(0x1234))]
    );
    assert_eq!(cursor.progress().processed, 2);
    assert_eq!(cursor.progress().cluster_index, 2);
}

#[test]
fn maximum_cluster_count_selects_sixteen_or_twenty_bit_split_lazily() {
    for (maximum, start) in [(0x1_0000, 0x1_0004), (0x1_0001, 0x10_0004)] {
        let (mut mesh, mut bytes) = fixture(&[&[0xf], &[0xf, 0, 0, 0, 1, 7, 8, 9]]);
        mesh.maximum_cluster_count = maximum;
        // Both the unrelated table entry and preceding unit are invalid.
        bytes[0x60..0x64].copy_from_slice(&u32::MAX.to_be_bytes());
        let mut cursor = MeshLineRangeCursor::new();
        let mut query = query(&[(start, 1)]);
        assert!(
            cursor
                .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
                    assert_eq!((item.cluster_index, item.unit_byte_offset), (1, 4));
                    assert_eq!(item.unit.vertex_indices, [7, 8, 9]);
                    Ok::<_, Infallible>(())
                })
                .unwrap()
        );
        assert_eq!(cursor.progress().processed, 1);
    }
}

#[test]
fn duplicate_ranges_resume_before_next_kd_leaf_and_keep_first_equal_candidate() {
    let (mesh, bytes) = fixture(&[&[1, 0, 1, 2, 1, 3, 4, 5]]);
    let mut query = query(&[(0, 2), (0, 2)]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut seen = vec![];
    let mut nearest = (f32::MAX, usize::MAX);
    for expected_pending in [1, 1, 0, 0] {
        assert!(
            cursor
                .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
                    let parameter = 0.5;
                    if parameter < nearest.0 {
                        nearest = (parameter, seen.len());
                    }
                    seen.push(item.unit_byte_offset);
                    Ok::<_, Infallible>(())
                })
                .unwrap()
        );
        assert_eq!(query.stack.len(), expected_pending);
    }
    assert_eq!(seen, [0, 4, 0, 4]);
    assert_eq!(nearest, (0.5, 0));
    assert!(
        !cursor
            .visit_next_chunk(&mesh, &bytes, &mut query, |_| Ok::<_, Infallible>(()))
            .unwrap()
    );
}

#[test]
fn malformed_last_unit_is_not_decoded_until_its_resumed_chunk() {
    let (mesh, bytes) = fixture(&[&[1, 0, 1, 2, 1, 3, 4, 5], &[1, 6, 7, 8, 1, 9, 10, 11, 0xf]]);
    let mut query = query(&[(0, 5)]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut visited = 0;
    for expected in [3, 4] {
        assert!(
            cursor
                .visit_next_chunk(&mesh, &bytes, &mut query, |_| {
                    visited += 1;
                    Ok::<_, Infallible>(())
                })
                .unwrap()
        );
        assert_eq!(visited, expected);
    }
    let before = cursor.progress();
    let error = cursor
        .visit_next_chunk(&mesh, &bytes, &mut query, |_| {
            visited += 1;
            Ok::<_, Infallible>(())
        })
        .unwrap_err();
    assert!(matches!(
        error,
        MeshLineCursorError::Decode(CollisionError::UnknownUnitType { value: 15 })
    ));
    assert_eq!(visited, 4);
    assert_eq!(cursor.progress(), before);
}

#[test]
fn visitor_refusal_retains_initial_new_range_but_not_local_unit_advancement() {
    let (mesh, bytes) = fixture(&[&[1, 0, 1, 2], &[1, 3, 4, 5, 1, 6, 7, 8]]);
    let mut query = query(&[(0, 3)]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut effects = vec![];
    let error = cursor
        .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
            effects.push(item.cluster_index);
            if effects.len() == 2 {
                Err("GP numeric refusal")
            } else {
                Ok(())
            }
        })
        .unwrap_err();
    assert!(matches!(
        error,
        MeshLineCursorError::Visitor("GP numeric refusal")
    ));
    assert_eq!(effects, [0, 1]);
    assert_eq!(
        cursor.progress(),
        MeshLineCursorProgress {
            cluster_index: 0,
            unit_byte_offset: 0,
            processed: 0,
            total: 3
        }
    );
    assert!(query.stack.is_empty());
}

#[test]
fn new_range_decode_failure_publishes_its_location_after_previous_completion() {
    let (mesh, bytes) = fixture(&[&[1, 0, 1, 2]]);
    let mut query = query(&[(0, 1), (7 << 16 | 9, 3)]);
    let mut cursor = MeshLineRangeCursor::new();
    assert!(
        cursor
            .visit_next_chunk(&mesh, &bytes, &mut query, |_| Ok::<_, Infallible>(()))
            .unwrap()
    );
    let error = cursor
        .visit_next_chunk(&mesh, &bytes, &mut query, |_| Ok::<_, Infallible>(()))
        .unwrap_err();
    assert!(matches!(
        error,
        MeshLineCursorError::Decode(CollisionError::EncodedClusterOutsideMesh {
            index: 7,
            cluster_count: 1
        })
    ));
    assert_eq!(
        cursor.progress(),
        MeshLineCursorProgress {
            cluster_index: 7,
            unit_byte_offset: 9,
            processed: 0,
            total: 3
        }
    );
}

#[test]
fn range_tail_does_not_predecode_next_unit_or_resolve_rolled_cluster() {
    let (mesh, bytes) = fixture(&[&[1, 0, 1, 2, 0xf]]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut query = query(&[(0, 1)]);
    assert!(
        cursor
            .visit_next_chunk(&mesh, &bytes, &mut query, |_| Ok::<_, Infallible>(()))
            .unwrap()
    );
    assert_eq!(cursor.progress().unit_byte_offset, 4);
    assert!(
        !cursor
            .visit_next_chunk(&mesh, &[], &mut query, |_| Ok::<_, Infallible>(()))
            .unwrap()
    );

    let (mesh, bytes) = fixture(&[&[1, 0, 1, 2]]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut query = self::query(&[(0, 1)]);
    assert!(
        cursor
            .visit_next_chunk(&mesh, &bytes, &mut query, |_| Ok::<_, Infallible>(()))
            .unwrap()
    );
    assert_eq!(cursor.progress().cluster_index, mesh.cluster_count);
    assert!(
        !cursor
            .visit_next_chunk(&mesh, &[], &mut query, |_| Ok::<_, Infallible>(()))
            .unwrap()
    );
}

#[test]
fn next_kd_branch_is_not_visited_while_current_range_is_incomplete() {
    let (mesh, bytes) = fixture(&[&[1, 0, 1, 2, 1, 3, 4, 5]]);
    let mut query = query(&[(0, 2)]);
    query.stack.insert(
        0,
        KdLineStackElement {
            content: u32::MAX,
            index: 9,
            pa: 0.0,
            pb: 1.0,
        },
    );
    let mut cursor = MeshLineRangeCursor::new();
    for _ in 0..2 {
        assert!(
            cursor
                .visit_next_chunk(&mesh, &bytes, &mut query, |_| Ok::<_, Infallible>(()))
                .unwrap()
        );
    }
    let before = cursor.progress();
    let error = cursor
        .visit_next_chunk(&mesh, &bytes, &mut query, |_| Ok::<_, Infallible>(()))
        .unwrap_err();
    assert!(matches!(
        error,
        MeshLineCursorError::Decode(CollisionError::KdBranchOutsideTree {
            index: 9,
            branch_count: 0
        })
    ));
    assert_eq!(cursor.progress(), before);
}

#[test]
fn empty_query_does_not_read_mesh_or_call_visitor() {
    let (mesh, _) = fixture(&[]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut query = query(&[(u32::MAX, 0)]);
    assert!(
        !cursor
            .visit_next_chunk(&mesh, &[], &mut query, |_| -> Result<(), Infallible> {
                panic!("empty traversal cannot visit a unit")
            })
            .unwrap()
    );
    assert_eq!(cursor.progress(), MeshLineCursorProgress::default());
}

fn vertex_fixture(
    compression: u8,
    count: u8,
    vertex_bytes: &[u8],
    units: &[u8],
) -> (ClusteredMesh, Vec<u8>) {
    let (mesh, mut bytes) = fixture(&[&[]]);
    let unit_start = (16 + vertex_bytes.len()).div_ceil(16) * 16;
    bytes.resize(0x80 + unit_start + units.len(), 0);
    let cluster = &mut bytes[0x80..];
    cluster[2..4].copy_from_slice(&u16::try_from(units.len()).unwrap().to_be_bytes());
    cluster[4..6].copy_from_slice(&u16::try_from((unit_start - 16) / 16).unwrap().to_be_bytes());
    cluster[8..10].copy_from_slice(
        &u16::try_from(unit_start + units.len())
            .unwrap()
            .to_be_bytes(),
    );
    cluster[10] = count;
    cluster[12] = compression;
    cluster[16..16 + vertex_bytes.len()].copy_from_slice(vertex_bytes);
    cluster[unit_start..].copy_from_slice(units);
    (mesh, bytes)
}

#[test]
fn ignores_unused_normals_unreferenced_vertices_and_missing_declared_tails() {
    let vertex = [1.25_f32, -2.5, 3.75, f32::NAN]
        .into_iter()
        .flat_map(f32::to_be_bytes)
        .collect::<Vec<_>>();
    let (mesh, mut bytes) = vertex_fixture(0, 255, &vertex, &[1, 0, 0, 0]);
    let cluster = &mut bytes[0x80..];
    cluster[6..8].copy_from_slice(&u16::MAX.to_be_bytes());
    cluster[11] = 255; // Entire serialized normal table is unavailable.
    cluster[8..10].copy_from_slice(&u16::MAX.to_be_bytes());
    cluster[2..4].copy_from_slice(&u16::MAX.to_be_bytes());
    assert!(crate::collision::MeshCluster::parse(cluster).is_err());
    let mut cursor = MeshLineRangeCursor::new();
    let mut query = query(&[(0, 1)]);
    assert!(
        cursor
            .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
                assert_eq!(
                    item.cluster.vertex_position(&mesh, 0)?.map(f32::to_bits),
                    [1.25_f32, -2.5, 3.75].map(f32::to_bits)
                );
                Ok::<_, CollisionError>(())
            })
            .unwrap()
    );
    assert_eq!(cursor.progress().processed, 1);
}

#[test]
fn later_referenced_truncated_vertex_refuses_after_earlier_unit_effects() {
    let vertex = [1.0_f32, 2.0, 3.0, 4.0]
        .into_iter()
        .flat_map(f32::to_be_bytes)
        .collect::<Vec<_>>();
    let (mesh, bytes) = vertex_fixture(0, 6, &vertex, &[1, 0, 0, 0, 1, 5, 5, 5]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut query = query(&[(0, 2)]);
    let mut effects = vec![];
    assert!(
        cursor
            .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
                effects.push(
                    item.cluster
                        .vertex_position(&mesh, item.unit.vertex_indices[0])?,
                );
                Ok::<_, CollisionError>(())
            })
            .unwrap()
    );
    let before = cursor.progress();
    let error = cursor
        .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
            effects.push(
                item.cluster
                    .vertex_position(&mesh, item.unit.vertex_indices[0])?,
            );
            Ok::<_, CollisionError>(())
        })
        .unwrap_err();
    assert!(matches!(
        error,
        MeshLineCursorError::Visitor(CollisionError::RangeOutsideObject { start: 96, .. })
    ));
    assert_eq!(effects, [[1.0, 2.0, 3.0]]);
    assert_eq!(cursor.progress(), before);
}

#[test]
fn selected_vertices_reuse_all_three_compression_graphs() {
    let uncompressed = [1.25_f32, -2.5, 3.75, 9.0]
        .into_iter()
        .flat_map(f32::to_be_bytes)
        .collect::<Vec<_>>();
    let mut unsigned = [i32::MAX - 1, i32::MIN, -10]
        .into_iter()
        .flat_map(i32::to_be_bytes)
        .collect::<Vec<_>>();
    unsigned.extend([3_u16, 65535, 9].into_iter().flat_map(u16::to_be_bytes));
    let signed = [16_777_217_i32, -16_777_217, 3]
        .into_iter()
        .flat_map(i32::to_be_bytes)
        .collect::<Vec<_>>();
    for (compression, data, granularity, expected) in [
        (0, uncompressed, 99.0, [1.25, -2.5, 3.75]),
        (1, unsigned, 0.5, [1_073_741_824.0, -1_073_709_056.0, -0.5]),
        (2, signed, 0.25, [4_194_304.0, -4_194_304.0, 0.75]),
    ] {
        let (mut mesh, bytes) = vertex_fixture(compression, 1, &data, &[1, 0, 0, 0]);
        mesh.vertex_compression_granularity = granularity;
        let mut cursor = MeshLineRangeCursor::new();
        let mut query = query(&[(0, 1)]);
        assert!(
            cursor
                .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
                    assert_eq!(
                        item.cluster.vertex_position(&mesh, 0)?.map(f32::to_bits),
                        expected.map(f32::to_bits)
                    );
                    Ok::<_, CollisionError>(())
                })
                .unwrap()
        );
    }
}

#[test]
fn compression_and_vertex_index_are_validated_only_on_vertex_access() {
    let (mesh, bytes) = vertex_fixture(99, 1, &[], &[0, 0, 0, 0]);
    let mut cursor = MeshLineRangeCursor::new();
    let mut query = query(&[(0, 1)]);
    assert!(
        cursor
            .visit_next_chunk(&mesh, &bytes, &mut query, |item| {
                assert_eq!(item.unit.unit_type, UnitType::OldTriangle);
                // The zero-GP native arm has no reason to decode vertices.
                Ok::<_, Infallible>(())
            })
            .unwrap()
    );
    let cluster = parse_selected_cluster(&mesh, &bytes, 0).unwrap();
    assert!(matches!(
        cluster.vertex_position(&mesh, 0),
        Err(CollisionError::UnknownVertexCompression { value: 99 })
    ));
    assert!(matches!(
        cluster.vertex_position(&mesh, 1),
        Err(CollisionError::VertexIndexOutsideCluster { cluster: 0 })
    ));
}

#[test]
fn missing_unit_tail_refuses_only_when_that_unit_is_reached() {
    let (mesh, mut bytes) = fixture(&[&[1, 0, 1, 2]]);
    bytes[0x82..0x84].copy_from_slice(&8_u16.to_be_bytes());
    bytes[0x88..0x8a].copy_from_slice(&24_u16.to_be_bytes());
    let mut cursor = MeshLineRangeCursor::new();
    let mut query = query(&[(0, 2)]);
    assert!(
        cursor
            .visit_next_chunk(&mesh, &bytes, &mut query, |_| Ok::<_, Infallible>(()))
            .unwrap()
    );
    let before = cursor.progress();
    let error = cursor
        .visit_next_chunk(&mesh, &bytes, &mut query, |_| Ok::<_, Infallible>(()))
        .unwrap_err();
    assert!(matches!(
        error,
        MeshLineCursorError::Decode(CollisionError::Truncated {
            required: 1,
            actual: 0
        })
    ));
    assert_eq!(cursor.progress(), before);
}
