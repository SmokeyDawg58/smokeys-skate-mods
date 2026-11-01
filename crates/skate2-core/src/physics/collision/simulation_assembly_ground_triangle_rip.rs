//! Private selected-range owner for Assembly/ground triangle ripping.
//!
//! `RipTriangles<BBoxTestRipFunctor<AssemblyTriangleIntersectionTester>>` at
//! `0x827cd7c8..0x827cd960` calls the old free unit-GP overload, applies the
//! final world triangle/AABB predicate per emitted sibling, consults `mSpew`
//! only afterward, and feeds accepted records through one tester-local
//! twenty-triangle cache. The owner persists across range and mesh calls and
//! can be drained only by its consuming job-tail `finish`.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the reconstructed Assembly/ground producer"
    )
)]

use crate::simulation_assembly_ground_triangle_cache::{
    AssemblyGroundTriangleBatch, AssemblyGroundTriangleCache,
};
use crate::simulation_assembly_ground_triangle_topology::{
    AssemblyGroundTriangle, UnitTopologyError, build_old_assembly_unit_gp,
};
use skate2_formats::collision::{
    ClusteredMesh, CollisionError, KdLeafRange, LeafClusterMemo, triangle_aabb_overlap,
};
use std::ops::ControlFlow;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum AssemblyGroundTriangleRipError {
    Format(CollisionError),
    Unit {
        cluster_index: usize,
        unit_stream_offset: usize,
        source: UnitTopologyError,
    },
}

pub(super) struct AssemblyGroundTriangleRipper {
    world_minimum: [f32; 3],
    world_maximum: [f32; 3],
    spew: bool,
    send_back_triangles: bool,
    cache: AssemblyGroundTriangleCache<AssemblyGroundTriangle>,
}

impl AssemblyGroundTriangleRipper {
    pub(super) const fn new(
        world_minimum: [f32; 3],
        world_maximum: [f32; 3],
        spew: bool,
        send_back_triangles: bool,
    ) -> Self {
        Self {
            world_minimum,
            world_maximum,
            spew,
            send_back_triangles,
            cache: AssemblyGroundTriangleCache::new(),
        }
    }

    pub(super) fn rip_leaf_range<D, I>(
        &mut self,
        mesh: &ClusteredMesh,
        mesh_bytes: &[u8],
        range: KdLeafRange,
        transform: [f32; 16],
        publish_debug: &mut D,
        run_intersections: &mut I,
    ) -> Result<(), AssemblyGroundTriangleRipError>
    where
        D: FnMut(&AssemblyGroundTriangle),
        I: for<'a> FnMut(AssemblyGroundTriangleBatch<'a, AssemblyGroundTriangle>),
    {
        let mut clusters = LeafClusterMemo::default();
        self.rip_mesh_leaf_range(
            mesh,
            mesh_bytes,
            range,
            &mut clusters,
            transform,
            publish_debug,
            run_intersections,
        )
    }

    /// [`Self::rip_leaf_range`] for one of a mesh's successive ranges, sharing
    /// `clusters` across them.
    #[allow(
        clippy::too_many_arguments,
        reason = "the native range, mesh memo, transform and two effects stay distinct"
    )]
    pub(super) fn rip_mesh_leaf_range<D, I>(
        &mut self,
        mesh: &ClusteredMesh,
        mesh_bytes: &[u8],
        range: KdLeafRange,
        clusters: &mut LeafClusterMemo<'_>,
        transform: [f32; 16],
        publish_debug: &mut D,
        run_intersections: &mut I,
    ) -> Result<(), AssemblyGroundTriangleRipError>
    where
        D: FnMut(&AssemblyGroundTriangle),
        I: for<'a> FnMut(AssemblyGroundTriangleBatch<'a, AssemblyGroundTriangle>),
    {
        self.rip_leaf_range_with_predicate(
            mesh,
            mesh_bytes,
            range,
            clusters,
            transform,
            &mut |minimum, maximum, points| triangle_aabb_overlap(minimum, maximum, points),
            publish_debug,
            run_intersections,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the test seam keeps the native range, transform, predicate, and two effects distinct"
    )]
    fn rip_leaf_range_with_predicate<P, D, I>(
        &mut self,
        mesh: &ClusteredMesh,
        mesh_bytes: &[u8],
        range: KdLeafRange,
        clusters: &mut LeafClusterMemo<'_>,
        transform: [f32; 16],
        predicate: &mut P,
        publish_debug: &mut D,
        run_intersections: &mut I,
    ) -> Result<(), AssemblyGroundTriangleRipError>
    where
        P: FnMut([f32; 3], [f32; 3], [[f32; 3]; 3]) -> bool,
        D: FnMut(&AssemblyGroundTriangle),
        I: for<'a> FnMut(AssemblyGroundTriangleBatch<'a, AssemblyGroundTriangle>),
    {
        let world_minimum = self.world_minimum;
        let world_maximum = self.world_maximum;
        let spew = &self.spew;
        let send_back_triangles = self.send_back_triangles;
        let cache = &mut self.cache;
        let outcome = mesh
            .visit_leaf_range_units_in(mesh_bytes, range, clusters, |cluster_index, cluster, unit| {
                let batch = match build_old_assembly_unit_gp(
                    mesh,
                    cluster_index,
                    cluster,
                    unit,
                    transform,
                    0,
                ) {
                    Ok(batch) => batch,
                    Err(source) => {
                        return ControlFlow::Break(AssemblyGroundTriangleRipError::Unit {
                            cluster_index,
                            unit_stream_offset: unit.stream_offset,
                            source,
                        });
                    }
                };

                for triangle in batch.iter().copied() {
                    let overlaps =
                        predicate(world_minimum, world_maximum, triangle.primitive().points);
                    if overlaps || *spew {
                        cache.accept_triangle(
                            triangle,
                            |triangle| {
                                if send_back_triangles {
                                    publish_debug(triangle);
                                }
                            },
                            &mut *run_intersections,
                        );
                    }
                }
                ControlFlow::Continue(())
            })
            .map_err(AssemblyGroundTriangleRipError::Format)?;

        match outcome {
            ControlFlow::Continue(()) => Ok(()),
            ControlFlow::Break(error) => Err(error),
        }
    }

    /// PC park editor: prebuilt world triangles (placed mission props) under the same world
    /// predicate and through the same cache as ripped mesh units.
    pub(super) fn rip_host_triangles<I>(&mut self, triangles: &[AssemblyGroundTriangle], run_intersections: &mut I)
    where
        I: for<'a> FnMut(AssemblyGroundTriangleBatch<'a, AssemblyGroundTriangle>),
    {
        for triangle in triangles {
            if triangle_aabb_overlap(self.world_minimum, self.world_maximum, triangle.primitive().points) {
                self.cache.accept_triangle(*triangle, |_| {}, &mut *run_intersections);
            }
        }
    }

    pub(super) fn finish<I>(self, run_intersections: I)
    where
        I: for<'a> FnOnce(AssemblyGroundTriangleBatch<'a, AssemblyGroundTriangle>),
    {
        self.cache.finish(run_intersections);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::simulation_assembly_ground_triangle_topology::UnitTopologyError;
    use skate2_formats::collision::UnitType;
    use std::cell::RefCell;

    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    pub(crate) fn fixture(
        vertices: &[[f32; 3]],
        unit_count: u16,
        units: &[u8],
    ) -> (ClusteredMesh, Vec<u8>) {
        const TABLE_OFFSET: usize = 0x60;
        const CLUSTER_OFFSET: usize = 0x80;
        let unit_start = 16 + vertices.len() * 16;
        let cluster_size = unit_start + units.len();
        let mut bytes = vec![0_u8; CLUSTER_OFFSET + cluster_size];

        bytes[52..56].copy_from_slice(&u32::try_from(TABLE_OFFSET).unwrap().to_be_bytes());
        bytes[56..60].copy_from_slice(&1.0_f32.to_be_bytes());
        bytes[62] = 1;
        bytes[63] = 1;
        bytes[64..68].copy_from_slice(&1_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&1_u32.to_be_bytes());
        bytes[72..76].copy_from_slice(&u32::from(unit_count).to_be_bytes());
        bytes[76..80].copy_from_slice(&u32::from(unit_count).to_be_bytes());
        let allocation_size = u32::try_from(bytes.len()).unwrap();
        bytes[80..84].copy_from_slice(&allocation_size.to_be_bytes());
        bytes[TABLE_OFFSET..TABLE_OFFSET + 4]
            .copy_from_slice(&u32::try_from(CLUSTER_OFFSET).unwrap().to_be_bytes());

        let cluster = &mut bytes[CLUSTER_OFFSET..];
        cluster[0..2].copy_from_slice(&unit_count.to_be_bytes());
        cluster[2..4].copy_from_slice(&u16::try_from(units.len()).unwrap().to_be_bytes());
        let unit_block_offset = u16::try_from(vertices.len()).unwrap();
        cluster[4..6].copy_from_slice(&unit_block_offset.to_be_bytes());
        cluster[6..8].copy_from_slice(&unit_block_offset.to_be_bytes());
        cluster[8..10].copy_from_slice(&u16::try_from(cluster_size).unwrap().to_be_bytes());
        cluster[10] = u8::try_from(vertices.len()).unwrap();
        for (vertex_index, point) in vertices.iter().enumerate() {
            let start = 16 + vertex_index * 16;
            for (axis, value) in [point[0], point[1], point[2], 0.0].into_iter().enumerate() {
                cluster[start + axis * 4..start + axis * 4 + 4]
                    .copy_from_slice(&value.to_be_bytes());
            }
        }
        cluster[unit_start..].copy_from_slice(units);

        (ClusteredMesh::parse(&bytes).unwrap(), bytes)
    }

    fn selected_midstream_fixture(maximum_cluster_count: u32) -> (ClusteredMesh, Vec<u8>) {
        const TABLE_OFFSET: usize = 0x60;
        const CLUSTER_OFFSET: usize = 0x80;
        let vertices = [[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        // Both sides of the selected record are intentionally malformed. A
        // correct one-record range starts at byte four and touches neither.
        let units = [0x0f, 0, 0, 0, 0x01, 0, 1, 2, 0x0f];
        let unit_start = 16 + vertices.len() * 16;
        let cluster_size = unit_start + units.len();
        let mut bytes = vec![0_u8; CLUSTER_OFFSET + cluster_size];

        bytes[52..56].copy_from_slice(&u32::try_from(TABLE_OFFSET).unwrap().to_be_bytes());
        bytes[56..60].copy_from_slice(&1.0_f32.to_be_bytes());
        bytes[62] = 1;
        bytes[63] = 1;
        bytes[64..68].copy_from_slice(&2_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&maximum_cluster_count.to_be_bytes());
        bytes[72..76].copy_from_slice(&1_u32.to_be_bytes());
        bytes[76..80].copy_from_slice(&1_u32.to_be_bytes());
        let allocation_size = u32::try_from(bytes.len()).unwrap();
        bytes[80..84].copy_from_slice(&allocation_size.to_be_bytes());
        bytes[TABLE_OFFSET..TABLE_OFFSET + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        bytes[TABLE_OFFSET + 4..TABLE_OFFSET + 8]
            .copy_from_slice(&u32::try_from(CLUSTER_OFFSET).unwrap().to_be_bytes());

        let cluster = &mut bytes[CLUSTER_OFFSET..];
        cluster[0..2].copy_from_slice(&1_u16.to_be_bytes());
        cluster[2..4].copy_from_slice(&u16::try_from(units.len()).unwrap().to_be_bytes());
        let unit_block_offset = u16::try_from(vertices.len()).unwrap();
        cluster[4..6].copy_from_slice(&unit_block_offset.to_be_bytes());
        cluster[6..8].copy_from_slice(&unit_block_offset.to_be_bytes());
        cluster[8..10].copy_from_slice(&u16::try_from(cluster_size).unwrap().to_be_bytes());
        cluster[10] = u8::try_from(vertices.len()).unwrap();
        for (vertex_index, point) in vertices.iter().enumerate() {
            let start = 16 + vertex_index * 16;
            for (axis, value) in [point[0], point[1], point[2], 0.0].into_iter().enumerate() {
                cluster[start + axis * 4..start + axis * 4 + 4]
                    .copy_from_slice(&value.to_be_bytes());
            }
        }
        cluster[unit_start..].copy_from_slice(&units);

        (ClusteredMesh::parse(&bytes).unwrap(), bytes)
    }

    const fn range(unit_offset: u32, unit_count: u32) -> KdLeafRange {
        KdLeafRange {
            encoded_start: unit_offset,
            unit_count,
        }
    }

    fn source_key(triangle: &AssemblyGroundTriangle) -> (usize, usize, u8) {
        let source = triangle.source();
        (
            source.cluster_index(),
            source.unit_stream_offset(),
            source.primitive_in_unit(),
        )
    }

    #[test]
    fn zero_range_is_lazy_and_consuming_finish_drains_empty_once() {
        let (mesh, _) = fixture(&[], 0, &[]);
        let mut owner = AssemblyGroundTriangleRipper::new([-1.0; 3], [1.0; 3], false, true);
        owner
            .rip_leaf_range(
                &mesh,
                &[],
                KdLeafRange {
                    encoded_start: u32::MAX,
                    unit_count: 0,
                },
                IDENTITY,
                &mut |_| panic!("zero range has no debug effect"),
                &mut |_| panic!("zero range does not flush"),
            )
            .unwrap();

        let drains = RefCell::new(Vec::new());
        owner.finish(|batch| drains.borrow_mut().push(batch.len()));
        assert_eq!(*drains.borrow(), [0]);
    }

    #[test]
    fn spew_forces_only_after_predicate_and_builder_does_not_precull() {
        let vertices = [[100.0, 0.0, 0.0], [101.0, 0.0, 0.0], [100.0, 1.0, 0.0]];
        let (mesh, bytes) = fixture(&vertices, 1, &[0x01, 0, 1, 2]);

        for (spew, expected) in [
            (false, vec!["predicate", "run:0"]),
            (true, vec!["predicate", "debug", "run:1"]),
        ] {
            let trace = RefCell::new(Vec::new());
            let mut owner = AssemblyGroundTriangleRipper::new([-1.0; 3], [1.0; 3], spew, true);
            owner
                .rip_leaf_range_with_predicate(
                    &mesh,
                    &bytes,
                    range(0, 1),
                    &mut LeafClusterMemo::default(),
                    IDENTITY,
                    &mut |_, _, points| {
                        assert_eq!(points[0][0].to_bits(), 100.0_f32.to_bits());
                        trace.borrow_mut().push("predicate");
                        false
                    },
                    &mut |_| trace.borrow_mut().push("debug"),
                    &mut |_| panic!("one record cannot fill the cache"),
                )
                .unwrap();
            owner.finish(|batch| {
                trace
                    .borrow_mut()
                    .push(if batch.len() == 0 { "run:0" } else { "run:1" });
            });
            assert_eq!(*trace.borrow(), expected);
        }

        for (spew, expected_count) in [(false, 0), (true, 1)] {
            let mut owner = AssemblyGroundTriangleRipper::new([-1.0; 3], [1.0; 3], spew, false);
            owner
                .rip_leaf_range(
                    &mesh,
                    &bytes,
                    range(0, 1),
                    IDENTITY,
                    &mut |_| unreachable!(),
                    &mut |_| unreachable!(),
                )
                .unwrap();
            owner.finish(|batch| assert_eq!(batch.len(), expected_count));
        }
    }

    #[test]
    fn forward_transform_precedes_the_real_world_predicate() {
        let vertices = [[100.0, 0.0, 0.0], [101.0, 0.0, 0.0], [100.0, 1.0, 0.0]];
        let (mesh, bytes) = fixture(&vertices, 1, &[0x01, 0, 1, 2]);
        let mut transform = IDENTITY;
        transform[12] = -100.0;
        let mut owner =
            AssemblyGroundTriangleRipper::new([-0.1, -0.1, -0.1], [0.1, 0.1, 0.1], false, false);
        owner
            .rip_leaf_range(
                &mesh,
                &bytes,
                range(0, 1),
                transform,
                &mut |_| panic!("debug is disabled"),
                &mut |_| panic!("one record cannot fill the cache"),
            )
            .unwrap();
        owner.finish(|batch| {
            let records = batch.iter().collect::<Vec<_>>();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].primitive().points[0][0].to_bits(), 0);
            assert_eq!(records[0].volume_tag(), 0);
        });
    }

    #[test]
    fn real_predicate_receives_each_quad_siblings_own_points() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let (mesh, bytes) = fixture(&vertices, 1, &[0x02, 0, 1, 2, 3]);
        let mut owner = AssemblyGroundTriangleRipper::new([-0.1; 3], [0.1; 3], false, false);
        owner
            .rip_leaf_range(
                &mesh,
                &bytes,
                range(0, 1),
                IDENTITY,
                &mut |_| panic!("debug is disabled"),
                &mut |_| panic!("one accepted sibling cannot fill the cache"),
            )
            .unwrap();
        owner.finish(|batch| {
            assert_eq!(
                batch.iter().map(source_key).collect::<Vec<_>>(),
                [(0, 0, 0)]
            );
        });
    }

    #[test]
    fn complete_encoded_range_selects_cluster_one_and_midstream_unit() {
        for (maximum_cluster_count, shift) in [(0x1_0000, 16), (0x1_0001, 20)] {
            let (mesh, bytes) = selected_midstream_fixture(maximum_cluster_count);
            let debug = RefCell::new(Vec::new());
            let mut owner = AssemblyGroundTriangleRipper::new([-2.0; 3], [2.0; 3], false, true);
            owner
                .rip_leaf_range(
                    &mesh,
                    &bytes,
                    range((1_u32 << shift) | 4, 1),
                    IDENTITY,
                    &mut |triangle| debug.borrow_mut().push(source_key(triangle)),
                    &mut |_| panic!("one selected record cannot fill the cache"),
                )
                .unwrap();
            assert_eq!(*debug.borrow(), [(1, 4, 0)]);
            owner.finish(|batch| {
                let records = batch.iter().collect::<Vec<_>>();
                assert_eq!(records.len(), 1);
                assert_eq!(source_key(records[0]), (1, 4, 0));
                assert_eq!(records[0].volume_tag(), 0);
            });
        }
    }

    #[test]
    fn predicate_is_per_sibling_and_rejection_does_not_stop_later_units() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let units = [0x02, 0, 1, 2, 3, 0x01, 0, 1, 2];
        let (mesh, bytes) = fixture(&vertices, 2, &units);
        let predicate_sources = RefCell::new(Vec::new());
        let debug_sources = RefCell::new(Vec::new());
        let mut owner = AssemblyGroundTriangleRipper::new([-2.0; 3], [2.0; 3], false, true);
        owner
            .rip_leaf_range_with_predicate(
                &mesh,
                &bytes,
                range(0, 2),
                &mut LeafClusterMemo::default(),
                IDENTITY,
                &mut |_, _, _| {
                    let index = predicate_sources.borrow().len();
                    predicate_sources.borrow_mut().push(index);
                    index != 1
                },
                &mut |triangle| debug_sources.borrow_mut().push(source_key(triangle)),
                &mut |_| panic!("three candidates cannot fill the cache"),
            )
            .unwrap();
        assert_eq!(*predicate_sources.borrow(), [0, 1, 2]);
        assert_eq!(*debug_sources.borrow(), [(0, 0, 0), (0, 5, 0)]);
        owner.finish(|batch| {
            assert_eq!(
                batch.iter().map(source_key).collect::<Vec<_>>(),
                [(0, 0, 0), (0, 5, 0)]
            );
        });
    }

    #[test]
    fn cache_persists_across_ranges_and_meshes_until_one_tail_drain() {
        let square = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let ten_quads = [0x02, 0, 1, 2, 3].repeat(10);
        let (first_mesh, first_bytes) = fixture(&square, 10, &ten_quads);
        let triangle = [[10.0, 0.0, 0.0], [11.0, 0.0, 0.0], [10.0, 1.0, 0.0]];
        let (second_mesh, second_bytes) = fixture(&triangle, 1, &[0x01, 0, 1, 2]);
        let debug = RefCell::new(Vec::new());
        let runs = RefCell::new(Vec::new());
        let callback_order = RefCell::new(Vec::new());
        let mut owner = AssemblyGroundTriangleRipper::new([-20.0; 3], [20.0; 3], false, true);
        let mut publish = |triangle: &AssemblyGroundTriangle| {
            debug.borrow_mut().push((
                triangle.primitive().points[0][0].to_bits(),
                source_key(triangle),
            ));
            callback_order.borrow_mut().push("debug");
        };
        let mut run = |batch: AssemblyGroundTriangleBatch<'_, AssemblyGroundTriangle>| {
            runs.borrow_mut()
                .push(batch.iter().map(source_key).collect::<Vec<_>>());
            callback_order.borrow_mut().push("run");
        };
        owner
            .rip_leaf_range(
                &first_mesh,
                &first_bytes,
                range(0, 10),
                IDENTITY,
                &mut publish,
                &mut run,
            )
            .unwrap();
        assert!(runs.borrow().is_empty());
        owner
            .rip_leaf_range(
                &second_mesh,
                &second_bytes,
                range(0, 1),
                IDENTITY,
                &mut publish,
                &mut run,
            )
            .unwrap();

        assert_eq!(debug.borrow().len(), 21);
        assert_eq!(runs.borrow().len(), 1);
        assert_eq!(&callback_order.borrow()[20..], ["debug", "run"]);
        assert_eq!(runs.borrow()[0].len(), 20);
        assert_eq!(runs.borrow()[0][0], (0, 0, 0));
        assert_eq!(runs.borrow()[0][1], (0, 0, 1));
        assert_eq!(runs.borrow()[0][18], (0, 45, 0));
        assert_eq!(runs.borrow()[0][19], (0, 45, 1));
        owner.finish(|batch| {
            assert_eq!(
                batch.iter().map(source_key).collect::<Vec<_>>(),
                [(0, 0, 0)]
            );
            assert_eq!(
                batch.iter().next().unwrap().primitive().points[0][0].to_bits(),
                10.0_f32.to_bits()
            );
        });
    }

    #[test]
    fn debug_disable_does_not_disable_local_cache() {
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let (mesh, bytes) = fixture(&vertices, 1, &[0x01, 0, 1, 2]);
        let mut owner = AssemblyGroundTriangleRipper::new([-2.0; 3], [2.0; 3], false, false);
        owner
            .rip_leaf_range(
                &mesh,
                &bytes,
                range(0, 1),
                IDENTITY,
                &mut |_| panic!("disabled debug hook must remain inert"),
                &mut |_| panic!("one record cannot fill the cache"),
            )
            .unwrap();
        owner.finish(|batch| assert_eq!(batch.len(), 1));
    }

    #[test]
    fn later_format_error_preserves_the_accepted_prefix() {
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let (mesh, bytes) = fixture(&vertices, 2, &[0x01, 0, 1, 2, 0x01, 0]);
        let mut owner = AssemblyGroundTriangleRipper::new([-2.0; 3], [2.0; 3], false, false);
        assert_eq!(
            owner.rip_leaf_range(
                &mesh,
                &bytes,
                range(0, 2),
                IDENTITY,
                &mut |_| unreachable!(),
                &mut |_| unreachable!(),
            ),
            Err(AssemblyGroundTriangleRipError::Format(
                CollisionError::Truncated {
                    required: 4,
                    actual: 2,
                }
            ))
        );
        owner.finish(|batch| {
            assert_eq!(
                batch.iter().map(source_key).collect::<Vec<_>>(),
                [(0, 0, 0)]
            );
        });
    }

    #[test]
    fn later_builder_refusal_preserves_prefix_and_exact_unit_location() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
        ];
        let units = [0x01, 0, 1, 2, 0x03, 3, 0, 1, 2, 3, 4];
        let (mesh, bytes) = fixture(&vertices, 2, &units);
        let mut owner = AssemblyGroundTriangleRipper::new([-3.0; 3], [3.0; 3], false, false);
        assert_eq!(
            owner.rip_leaf_range(
                &mesh,
                &bytes,
                range(0, 2),
                IDENTITY,
                &mut |_| unreachable!(),
                &mut |_| unreachable!(),
            ),
            Err(AssemblyGroundTriangleRipError::Unit {
                cluster_index: 0,
                unit_stream_offset: 4,
                source: UnitTopologyError::AssemblyNativeAssertedPrimitiveCount {
                    unit_type: UnitType::TriangleList,
                    count: 3,
                },
            })
        );
        owner.finish(|batch| {
            assert_eq!(
                batch.iter().map(source_key).collect::<Vec<_>>(),
                [(0, 0, 0)]
            );
        });
    }
}
