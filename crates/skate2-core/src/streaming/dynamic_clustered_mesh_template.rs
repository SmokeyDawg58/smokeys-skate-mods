//! Shared, zero-copy clustered-mesh templates for dynamic assembly parts.
//!
//! `DynamicPool::BeginUpdate` at `0x827c93f0` visits assemblies in pool order,
//! their parts in index order, and calls `DynamicPool::AddVolume` at
//! `0x827c86d0` for each part volume. `AddVolume` follows enabled aggregate volumes through
//! `SimpleMappedArray` children in ascending inline order and emits every
//! `ClusteredMesh` occurrence. It does not descend into other aggregate
//! types. This module retains that structural walk without implementing live
//! body placement, pool capacity, or collision queries.
//!
//! Enabled/ignored state is sampled while the template is built; changing the
//! corresponding native cvar policy requires rebuilding the template. The DFS
//! consumes `CollisionArena`'s already-projected, validated original-data
//! graph. Typed malformed-resource and cycle errors are host safety policy,
//! not evidence of native recovery behavior.

use crate::world::{CollisionAggregateRef, CollisionArena, CollisionMesh};
use skate2_formats::{
    collision::{Volume, VolumeType},
    pegasus::ObjectId,
};
use std::{error::Error, fmt, sync::Arc};

/// RW collision volume enabled bit tested by `DynamicPool::AddVolume` at
/// `0x827c86ec..0x827c8710`.
const RW_COLLISION_VOLUME_FLAG_ENABLED: u32 = 1;

/// The assembly-part fields needed after the serialized assembly definition
/// has been resolved to one [`CollisionArena`] root.
///
/// The three body fields are retained exactly as serialized. This structural
/// template deliberately does not interpret or compose them with any
/// [`Volume::transform`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicClusteredMeshTemplatePart {
    pub root_index: usize,
    pub inverse_body_transform: [f32; 16],
    pub has_inverse_body_transform: u32,
    pub is_static_body: u32,
}

/// One occurrence emitted by the native-shaped part/SMA depth-first walk.
///
/// `volume` is the exact aggregate volume at the leaf occurrence. Repeated
/// references to one mesh therefore remain distinct and may retain different
/// volume transforms or flags.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicClusteredMeshTemplateLeaf {
    pub part_ordinal: usize,
    pub root_index: usize,
    pub mesh_index: usize,
    pub volume: Volume,
}

/// Immutable arena owner plus ordered indexes into its clustered meshes.
///
/// The arena is shared rather than cloned, so a template neither duplicates
/// `CollisionMesh::object_bytes` nor stores self-referential borrows.
#[derive(Clone, Debug)]
pub struct DynamicClusteredMeshTemplate {
    arena: Arc<CollisionArena>,
    parts: Vec<DynamicClusteredMeshTemplatePart>,
    leaves: Vec<DynamicClusteredMeshTemplateLeaf>,
    ignore_volume_enable: bool,
}

impl DynamicClusteredMeshTemplate {
    /// Builds a complete template before publishing it.
    ///
    /// `parts` must already be in native assembly-part order. When
    /// `ignore_volume_enable` is false, a disabled parent suppresses its whole
    /// subtree and a disabled inline child is skipped. Primitive leaves and
    /// unsupported aggregate kinds are ignored exactly like `AddVolume`.
    ///
    /// # Errors
    ///
    /// Returns a typed error for malformed reachable indexes or references,
    /// including a cycle on the active DFS path. These errors and cycle
    /// detection are safe host policy, not native recoverable behavior. No
    /// partial template is returned.
    pub fn try_new(
        arena: Arc<CollisionArena>,
        parts: Vec<DynamicClusteredMeshTemplatePart>,
        ignore_volume_enable: bool,
    ) -> Result<Self, DynamicClusteredMeshTemplateError> {
        let mut leaves = Vec::new();
        for (part_ordinal, part) in parts.iter().enumerate() {
            let root = arena.aggregate_roots.get(part.root_index).ok_or(
                DynamicClusteredMeshTemplateError::InvalidRootIndex {
                    part_ordinal,
                    root_index: part.root_index,
                },
            )?;
            if !volume_is_enabled(&root.volume, ignore_volume_enable) {
                continue;
            }
            if root.volume.volume_type() != Some(VolumeType::Aggregate) {
                continue;
            }
            let aggregate =
                root.aggregate
                    .ok_or(DynamicClusteredMeshTemplateError::MissingRootAggregate {
                        part_ordinal,
                        root_index: part.root_index,
                    })?;
            validate_aggregate_index(&arena, aggregate)?;
            let resolved = resolve_aggregate_target(&arena, root.volume.shape_words[0])?;
            if resolved != aggregate {
                return Err(
                    DynamicClusteredMeshTemplateError::RootAggregateLinkMismatch {
                        part_ordinal,
                        root_index: part.root_index,
                        linked: aggregate,
                        resolved,
                    },
                );
            }
            walk_part_aggregates(
                &arena,
                aggregate,
                root.volume.clone(),
                part_ordinal,
                part.root_index,
                ignore_volume_enable,
                &mut leaves,
            )?;
        }
        Ok(Self {
            arena,
            parts,
            leaves,
            ignore_volume_enable,
        })
    }

    #[must_use]
    pub fn arena_owner(&self) -> &Arc<CollisionArena> {
        &self.arena
    }

    #[must_use]
    pub fn parts(&self) -> &[DynamicClusteredMeshTemplatePart] {
        &self.parts
    }

    #[must_use]
    pub fn leaves(&self) -> &[DynamicClusteredMeshTemplateLeaf] {
        &self.leaves
    }

    #[must_use]
    pub const fn ignores_volume_enable(&self) -> bool {
        self.ignore_volume_enable
    }

    /// Resolves an emitted leaf ordinal without accepting a caller-constructed
    /// leaf descriptor or exposing a borrow that can outlive the arena owner.
    #[must_use]
    pub fn mesh_at(&self, leaf_ordinal: usize) -> Option<&CollisionMesh> {
        let leaf = self.leaves.get(leaf_ordinal)?;
        self.arena.meshes.get(leaf.mesh_index)
    }
}

enum AggregateWalkFrame {
    Enter {
        aggregate: CollisionAggregateRef,
        leaf_volume: Volume,
    },
    ResumeSimplePrimitiveSet {
        aggregate: CollisionAggregateRef,
        set_index: usize,
        next_child: usize,
    },
}

fn walk_part_aggregates(
    arena: &CollisionArena,
    aggregate: CollisionAggregateRef,
    root_volume: Volume,
    part_ordinal: usize,
    root_index: usize,
    ignore_volume_enable: bool,
    leaves: &mut Vec<DynamicClusteredMeshTemplateLeaf>,
) -> Result<(), DynamicClusteredMeshTemplateError> {
    let mut path = Vec::new();
    let mut frames = vec![AggregateWalkFrame::Enter {
        aggregate,
        leaf_volume: root_volume,
    }];
    while let Some(frame) = frames.pop() {
        match frame {
            AggregateWalkFrame::Enter {
                aggregate,
                leaf_volume,
            } => {
                if path.contains(&aggregate) {
                    return Err(DynamicClusteredMeshTemplateError::AggregateCycle { aggregate });
                }
                match aggregate {
                    CollisionAggregateRef::PrimitiveSet(index) => {
                        arena.primitive_sets.get(index).ok_or(
                            DynamicClusteredMeshTemplateError::InvalidAggregateIndex { aggregate },
                        )?;
                    }
                    CollisionAggregateRef::SimplePrimitiveSet(set_index) => {
                        arena.simple_primitive_sets.get(set_index).ok_or(
                            DynamicClusteredMeshTemplateError::InvalidAggregateIndex { aggregate },
                        )?;
                        path.push(aggregate);
                        frames.push(AggregateWalkFrame::ResumeSimplePrimitiveSet {
                            aggregate,
                            set_index,
                            next_child: 0,
                        });
                    }
                    CollisionAggregateRef::Mesh(index) => {
                        arena.meshes.get(index).ok_or(
                            DynamicClusteredMeshTemplateError::InvalidAggregateIndex { aggregate },
                        )?;
                        leaves.push(DynamicClusteredMeshTemplateLeaf {
                            part_ordinal,
                            root_index,
                            mesh_index: index,
                            volume: leaf_volume,
                        });
                    }
                }
            }
            AggregateWalkFrame::ResumeSimplePrimitiveSet {
                aggregate,
                set_index,
                next_child,
            } => {
                let set = &arena.simple_primitive_sets[set_index];
                let Some(volume) = set.volumes.get(next_child) else {
                    let completed = path.pop();
                    debug_assert_eq!(completed, Some(aggregate));
                    continue;
                };
                frames.push(AggregateWalkFrame::ResumeSimplePrimitiveSet {
                    aggregate,
                    set_index,
                    next_child: next_child + 1,
                });
                if !volume_is_enabled(volume, ignore_volume_enable) {
                    continue;
                }
                if volume.volume_type() != Some(VolumeType::Aggregate) {
                    continue;
                }
                let child = resolve_aggregate_target(arena, volume.shape_words[0])?;
                frames.push(AggregateWalkFrame::Enter {
                    aggregate: child,
                    leaf_volume: volume.clone(),
                });
            }
        }
    }
    Ok(())
}

fn aggregate_by_dictionary_index(
    arena: &CollisionArena,
    dictionary_index: u32,
) -> Result<CollisionAggregateRef, DynamicClusteredMeshTemplateError> {
    let mut found = None;
    for (index, set) in arena.primitive_sets.iter().enumerate() {
        if set.dictionary_index == dictionary_index {
            record_aggregate_match(
                &mut found,
                CollisionAggregateRef::PrimitiveSet(index),
                dictionary_index,
            )?;
        }
    }
    for (index, set) in arena.simple_primitive_sets.iter().enumerate() {
        if set.dictionary_index == dictionary_index {
            record_aggregate_match(
                &mut found,
                CollisionAggregateRef::SimplePrimitiveSet(index),
                dictionary_index,
            )?;
        }
    }
    for (index, mesh) in arena.meshes.iter().enumerate() {
        if mesh.dictionary_index == dictionary_index {
            record_aggregate_match(
                &mut found,
                CollisionAggregateRef::Mesh(index),
                dictionary_index,
            )?;
        }
    }
    found.ok_or(DynamicClusteredMeshTemplateError::MissingAggregateTarget { dictionary_index })
}

fn record_aggregate_match(
    found: &mut Option<CollisionAggregateRef>,
    candidate: CollisionAggregateRef,
    dictionary_index: u32,
) -> Result<(), DynamicClusteredMeshTemplateError> {
    if found.replace(candidate).is_some() {
        return Err(
            DynamicClusteredMeshTemplateError::AmbiguousAggregateTarget { dictionary_index },
        );
    }
    Ok(())
}

pub(crate) fn resolve_aggregate_target(
    arena: &CollisionArena,
    raw_target: u32,
) -> Result<CollisionAggregateRef, DynamicClusteredMeshTemplateError> {
    if raw_target == 0 {
        return Err(DynamicClusteredMeshTemplateError::NullAggregateTarget);
    }
    let target = ObjectId(raw_target);
    if target.section_index() != 0 {
        return Err(DynamicClusteredMeshTemplateError::NonLocalAggregateTarget {
            raw_object_id: raw_target,
        });
    }
    aggregate_by_dictionary_index(arena, target.entry_index())
}

fn validate_aggregate_index(
    arena: &CollisionArena,
    aggregate: CollisionAggregateRef,
) -> Result<(), DynamicClusteredMeshTemplateError> {
    let valid = match aggregate {
        CollisionAggregateRef::PrimitiveSet(index) => arena.primitive_sets.get(index).is_some(),
        CollisionAggregateRef::SimplePrimitiveSet(index) => {
            arena.simple_primitive_sets.get(index).is_some()
        }
        CollisionAggregateRef::Mesh(index) => arena.meshes.get(index).is_some(),
    };
    if valid {
        Ok(())
    } else {
        Err(DynamicClusteredMeshTemplateError::InvalidAggregateIndex { aggregate })
    }
}

const fn volume_is_enabled(volume: &Volume, ignore_volume_enable: bool) -> bool {
    ignore_volume_enable || volume.flags & RW_COLLISION_VOLUME_FLAG_ENABLED != 0
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DynamicClusteredMeshTemplateError {
    InvalidRootIndex {
        part_ordinal: usize,
        root_index: usize,
    },
    InvalidAggregateIndex {
        aggregate: CollisionAggregateRef,
    },
    MissingRootAggregate {
        part_ordinal: usize,
        root_index: usize,
    },
    RootAggregateLinkMismatch {
        part_ordinal: usize,
        root_index: usize,
        linked: CollisionAggregateRef,
        resolved: CollisionAggregateRef,
    },
    NullAggregateTarget,
    NonLocalAggregateTarget {
        raw_object_id: u32,
    },
    MissingAggregateTarget {
        dictionary_index: u32,
    },
    AmbiguousAggregateTarget {
        dictionary_index: u32,
    },
    AggregateCycle {
        aggregate: CollisionAggregateRef,
    },
}

impl fmt::Display for DynamicClusteredMeshTemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRootIndex {
                part_ordinal,
                root_index,
            } => write!(
                formatter,
                "dynamic clustered-mesh part {part_ordinal} references missing root {root_index}"
            ),
            Self::InvalidAggregateIndex { aggregate } => {
                write!(
                    formatter,
                    "dynamic clustered-mesh aggregate index is invalid: {aggregate:?}"
                )
            }
            Self::MissingRootAggregate {
                part_ordinal,
                root_index,
            } => write!(
                formatter,
                "dynamic clustered-mesh part {part_ordinal} root {root_index} has an aggregate volume without a resolved aggregate"
            ),
            Self::RootAggregateLinkMismatch {
                part_ordinal,
                root_index,
                linked,
                resolved,
            } => write!(
                formatter,
                "dynamic clustered-mesh part {part_ordinal} root {root_index} links {linked:?} but its raw target resolves to {resolved:?}"
            ),
            Self::NullAggregateTarget => {
                formatter.write_str("dynamic clustered-mesh aggregate volume has a null target")
            }
            Self::NonLocalAggregateTarget { raw_object_id } => write!(
                formatter,
                "dynamic clustered-mesh aggregate target {raw_object_id:#010x} is nonlocal"
            ),
            Self::MissingAggregateTarget { dictionary_index } => write!(
                formatter,
                "dynamic clustered-mesh aggregate dictionary target {dictionary_index} is missing"
            ),
            Self::AmbiguousAggregateTarget { dictionary_index } => write!(
                formatter,
                "dynamic clustered-mesh aggregate dictionary target {dictionary_index} is ambiguous"
            ),
            Self::AggregateCycle { aggregate } => write!(
                formatter,
                "dynamic clustered-mesh aggregate path contains a cycle at {aggregate:?}"
            ),
        }
    }
}

impl Error for DynamicClusteredMeshTemplateError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{
        CollisionMesh, CollisionPrimitiveSet, CollisionRoot, CollisionSimplePrimitiveSet,
    };
    use skate2_formats::collision::{ClusteredMesh, KdTreeMappedArray};
    use std::sync::{Arc, Weak};

    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    fn transform_marker(marker: f32) -> [f32; 16] {
        let mut transform = IDENTITY;
        transform[12] = marker;
        transform
    }

    fn aggregate_volume(raw_target: u32, flags: u32, marker: f32) -> Volume {
        Volume {
            transform: transform_marker(marker),
            vtable_offset: 6,
            shape_words: [raw_target, 0, 0],
            radius: marker,
            group_id: marker.to_bits(),
            surface_id: marker.to_bits().wrapping_add(1),
            flags,
        }
    }

    fn primitive_volume(flags: u32, marker: f32) -> Volume {
        Volume {
            transform: transform_marker(marker),
            vtable_offset: 1,
            shape_words: [0; 3],
            radius: marker,
            group_id: 0,
            surface_id: 0,
            flags,
        }
    }

    fn mesh(dictionary_index: u32, byte: u8) -> CollisionMesh {
        CollisionMesh {
            dictionary_index,
            bounding_box_min: [0.0; 3],
            bounding_box_max: [1.0; 3],
            triangles: Vec::new(),
            clustered_mesh: ClusteredMesh::parse(&[0; 96]).unwrap(),
            object_bytes: vec![byte; 128],
        }
    }

    fn simple_set(dictionary_index: u32, volumes: Vec<Volume>) -> CollisionSimplePrimitiveSet {
        CollisionSimplePrimitiveSet {
            dictionary_index,
            bounding_box_min: [0.0; 3],
            bounding_box_max: [1.0; 3],
            volumes,
        }
    }

    fn primitive_set(dictionary_index: u32, volumes: Vec<Volume>) -> CollisionPrimitiveSet {
        CollisionPrimitiveSet {
            dictionary_index,
            bounding_box_min: [0.0; 3],
            bounding_box_max: [1.0; 3],
            volumes,
            mapped_array: KdTreeMappedArray::parse(&[0; 80]).unwrap(),
            object_bytes: vec![0; 80],
        }
    }

    fn root(
        aggregate: Option<CollisionAggregateRef>,
        raw_target: u32,
        flags: u32,
        marker: f32,
    ) -> CollisionRoot {
        CollisionRoot {
            dictionary_index: marker.to_bits(),
            volume: aggregate_volume(raw_target, flags, marker),
            aggregate,
        }
    }

    fn nonaggregate_root(
        aggregate: Option<CollisionAggregateRef>,
        flags: u32,
        marker: f32,
    ) -> CollisionRoot {
        CollisionRoot {
            dictionary_index: marker.to_bits(),
            volume: primitive_volume(flags, marker),
            aggregate,
        }
    }

    fn part(root_index: usize, marker: f32) -> DynamicClusteredMeshTemplatePart {
        DynamicClusteredMeshTemplatePart {
            root_index,
            inverse_body_transform: transform_marker(marker),
            has_inverse_body_transform: marker.to_bits(),
            is_static_body: marker.to_bits().wrapping_add(1),
        }
    }

    fn arena(
        roots: Vec<CollisionRoot>,
        simple_primitive_sets: Vec<CollisionSimplePrimitiveSet>,
        primitive_sets: Vec<CollisionPrimitiveSet>,
        meshes: Vec<CollisionMesh>,
    ) -> CollisionArena {
        CollisionArena {
            id: 7,
            grind_assets: Vec::new(),
            aggregate_roots: roots,
            primitive_sets,
            simple_primitive_sets,
            meshes,
            collision_models: Vec::new(),
            instances: Vec::new(),
        }
    }

    #[test]
    fn emits_direct_and_nested_meshes_in_part_then_sma_dfs_order() {
        let source = Arc::new(arena(
            vec![
                root(
                    Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                    100,
                    1,
                    10.0,
                ),
                root(Some(CollisionAggregateRef::Mesh(2)), 202, 1, 20.0),
            ],
            vec![
                simple_set(
                    100,
                    vec![
                        aggregate_volume(200, 1, 1.0),
                        primitive_volume(1, 99.0),
                        aggregate_volume(101, 1, 2.0),
                    ],
                ),
                simple_set(
                    101,
                    vec![aggregate_volume(201, 1, 3.0), aggregate_volume(202, 1, 4.0)],
                ),
            ],
            Vec::new(),
            vec![mesh(200, 1), mesh(201, 2), mesh(202, 3)],
        ));
        let parts = vec![part(0, 31.0), part(1, 32.0)];
        let template = DynamicClusteredMeshTemplate::try_new(source, parts.clone(), false).unwrap();

        assert_eq!(template.parts(), parts);
        assert_eq!(
            template
                .leaves()
                .iter()
                .map(|leaf| (
                    leaf.part_ordinal,
                    leaf.root_index,
                    leaf.mesh_index,
                    leaf.volume.radius
                ))
                .collect::<Vec<_>>(),
            vec![
                (0, 0, 0, 1.0),
                (0, 0, 1, 3.0),
                (0, 0, 2, 4.0),
                (1, 1, 2, 20.0)
            ]
        );
        assert_eq!(
            template.leaves()[0].volume.transform.map(f32::to_bits),
            transform_marker(1.0).map(f32::to_bits)
        );
        assert_eq!(
            template.parts()[0].inverse_body_transform.map(f32::to_bits),
            transform_marker(31.0).map(f32::to_bits)
        );
    }

    #[test]
    fn enable_gate_skips_disabled_parent_and_child_unless_explicitly_ignored() {
        let make_arena = || {
            Arc::new(arena(
                vec![
                    root(Some(CollisionAggregateRef::Mesh(0)), 200, 0, 10.0),
                    root(
                        Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                        100,
                        1,
                        20.0,
                    ),
                ],
                vec![simple_set(
                    100,
                    vec![aggregate_volume(200, 0, 1.0), aggregate_volume(200, 1, 2.0)],
                )],
                Vec::new(),
                vec![mesh(200, 1)],
            ))
        };
        let parts = vec![part(0, 1.0), part(1, 2.0)];
        let gated =
            DynamicClusteredMeshTemplate::try_new(make_arena(), parts.clone(), false).unwrap();
        assert_eq!(
            gated
                .leaves()
                .iter()
                .map(|leaf| leaf.volume.radius)
                .collect::<Vec<_>>(),
            vec![2.0]
        );
        let ignored = DynamicClusteredMeshTemplate::try_new(make_arena(), parts, true).unwrap();
        assert!(ignored.ignores_volume_enable());
        assert_eq!(
            ignored
                .leaves()
                .iter()
                .map(|leaf| leaf.volume.radius)
                .collect::<Vec<_>>(),
            vec![10.0, 1.0, 2.0]
        );
    }

    #[test]
    fn disabled_intermediate_sma_suppresses_its_subtree_unless_ignored() {
        let make_arena = || {
            Arc::new(arena(
                vec![root(
                    Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                    100,
                    1,
                    10.0,
                )],
                vec![
                    simple_set(100, vec![aggregate_volume(101, 0, 1.0)]),
                    simple_set(101, vec![aggregate_volume(200, 1, 2.0)]),
                ],
                Vec::new(),
                vec![mesh(200, 1)],
            ))
        };

        let gated =
            DynamicClusteredMeshTemplate::try_new(make_arena(), vec![part(0, 1.0)], false).unwrap();
        assert!(gated.leaves().is_empty());

        let ignored =
            DynamicClusteredMeshTemplate::try_new(make_arena(), vec![part(0, 1.0)], true).unwrap();
        assert_eq!(ignored.leaves().len(), 1);
        assert_eq!(ignored.leaves()[0].mesh_index, 0);
        assert_eq!(
            ignored.leaves()[0].volume.radius.to_bits(),
            2.0_f32.to_bits()
        );
    }

    #[test]
    fn disabled_malformed_aggregate_target_is_not_resolved_until_enable_is_ignored() {
        let make_arena = || {
            Arc::new(arena(
                vec![root(
                    Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                    100,
                    1,
                    10.0,
                )],
                vec![simple_set(100, vec![aggregate_volume(0, 0, 1.0)])],
                Vec::new(),
                Vec::new(),
            ))
        };

        let gated =
            DynamicClusteredMeshTemplate::try_new(make_arena(), vec![part(0, 1.0)], false).unwrap();
        assert!(gated.leaves().is_empty());

        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(make_arena(), vec![part(0, 1.0)], true)
                .unwrap_err(),
            DynamicClusteredMeshTemplateError::NullAggregateTarget
        );
    }

    #[test]
    fn unsupported_aggregate_and_nonaggregate_children_are_skipped() {
        let source = Arc::new(arena(
            vec![
                root(Some(CollisionAggregateRef::PrimitiveSet(0)), 300, 1, 10.0),
                root(
                    Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                    100,
                    1,
                    20.0,
                ),
                nonaggregate_root(None, 1, 30.0),
            ],
            vec![simple_set(
                100,
                vec![primitive_volume(1, 1.0), aggregate_volume(300, 1, 2.0)],
            )],
            vec![primitive_set(300, vec![aggregate_volume(200, 1, 3.0)])],
            vec![mesh(200, 1)],
        ));
        let template = DynamicClusteredMeshTemplate::try_new(
            source,
            vec![part(0, 1.0), part(1, 2.0), part(2, 3.0)],
            false,
        )
        .unwrap();
        assert!(template.leaves().is_empty());
    }

    #[test]
    fn repeated_mesh_aliases_emit_distinct_exact_leaf_volumes() {
        let source = Arc::new(arena(
            vec![root(
                Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                100,
                1,
                10.0,
            )],
            vec![simple_set(
                100,
                vec![aggregate_volume(200, 1, 1.0), aggregate_volume(200, 1, 2.0)],
            )],
            Vec::new(),
            vec![mesh(200, 1)],
        ));
        let template =
            DynamicClusteredMeshTemplate::try_new(source, vec![part(0, 1.0)], false).unwrap();
        assert_eq!(template.leaves().len(), 2);
        assert_eq!(template.leaves()[0].mesh_index, 0);
        assert_eq!(template.leaves()[1].mesh_index, 0);
        assert_eq!(
            template.leaves()[0].volume.radius.to_bits(),
            1.0_f32.to_bits()
        );
        assert_eq!(
            template.leaves()[1].volume.radius.to_bits(),
            2.0_f32.to_bits()
        );
    }

    #[test]
    fn completed_nested_sma_alias_is_removed_from_the_active_path() {
        let source = Arc::new(arena(
            vec![root(
                Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                100,
                1,
                10.0,
            )],
            vec![
                simple_set(
                    100,
                    vec![aggregate_volume(101, 1, 1.0), aggregate_volume(101, 1, 2.0)],
                ),
                simple_set(101, vec![aggregate_volume(200, 1, 3.0)]),
            ],
            Vec::new(),
            vec![mesh(200, 1)],
        ));

        let template =
            DynamicClusteredMeshTemplate::try_new(source, vec![part(0, 1.0)], false).unwrap();

        assert_eq!(template.leaves().len(), 2);
        assert!(
            template
                .leaves()
                .iter()
                .all(|leaf| leaf.mesh_index == 0
                    && leaf.volume.radius.to_bits() == 3.0_f32.to_bits())
        );
    }

    #[test]
    fn rejects_self_and_two_node_sma_cycles() {
        let self_cycle = Arc::new(arena(
            vec![root(
                Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                100,
                1,
                1.0,
            )],
            vec![simple_set(100, vec![aggregate_volume(100, 1, 1.0)])],
            Vec::new(),
            Vec::new(),
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(self_cycle, vec![part(0, 1.0)], false)
                .unwrap_err(),
            DynamicClusteredMeshTemplateError::AggregateCycle {
                aggregate: CollisionAggregateRef::SimplePrimitiveSet(0),
            }
        );

        let two_node = Arc::new(arena(
            vec![root(
                Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                100,
                1,
                1.0,
            )],
            vec![
                simple_set(100, vec![aggregate_volume(101, 1, 1.0)]),
                simple_set(101, vec![aggregate_volume(100, 1, 2.0)]),
            ],
            Vec::new(),
            Vec::new(),
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(two_node, vec![part(0, 1.0)], false).unwrap_err(),
            DynamicClusteredMeshTemplateError::AggregateCycle {
                aggregate: CollisionAggregateRef::SimplePrimitiveSet(0),
            }
        );
    }

    #[test]
    fn reports_bad_indexes_nonlocal_and_missing_targets() {
        let empty = || Arc::new(arena(Vec::new(), Vec::new(), Vec::new(), Vec::new()));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(empty(), vec![part(4, 1.0)], false).unwrap_err(),
            DynamicClusteredMeshTemplateError::InvalidRootIndex {
                part_ordinal: 0,
                root_index: 4,
            }
        );

        let bad_mesh = Arc::new(arena(
            vec![root(Some(CollisionAggregateRef::Mesh(3)), 200, 1, 1.0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(bad_mesh, vec![part(0, 1.0)], false).unwrap_err(),
            DynamicClusteredMeshTemplateError::InvalidAggregateIndex {
                aggregate: CollisionAggregateRef::Mesh(3),
            }
        );

        let nonlocal_raw = (1 << 22) | 0x00c8;
        let nonlocal = Arc::new(arena(
            vec![root(
                Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                100,
                1,
                1.0,
            )],
            vec![simple_set(
                100,
                vec![aggregate_volume(nonlocal_raw, 1, 1.0)],
            )],
            Vec::new(),
            Vec::new(),
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(nonlocal, vec![part(0, 1.0)], false).unwrap_err(),
            DynamicClusteredMeshTemplateError::NonLocalAggregateTarget {
                raw_object_id: nonlocal_raw,
            }
        );

        let missing = Arc::new(arena(
            vec![root(
                Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                100,
                1,
                1.0,
            )],
            vec![simple_set(100, vec![aggregate_volume(999, 1, 1.0)])],
            Vec::new(),
            Vec::new(),
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(missing, vec![part(0, 1.0)], false).unwrap_err(),
            DynamicClusteredMeshTemplateError::MissingAggregateTarget {
                dictionary_index: 999,
            }
        );
    }

    #[test]
    fn aggregate_type_and_link_inconsistencies_are_discriminated() {
        let nonaggregate_with_link = Arc::new(arena(
            vec![nonaggregate_root(
                Some(CollisionAggregateRef::Mesh(0)),
                1,
                1.0,
            )],
            Vec::new(),
            Vec::new(),
            vec![mesh(200, 1)],
        ));
        let skipped = DynamicClusteredMeshTemplate::try_new(
            nonaggregate_with_link,
            vec![part(0, 1.0)],
            false,
        )
        .unwrap();
        assert!(skipped.leaves().is_empty());

        let aggregate_without_link = Arc::new(arena(
            vec![root(None, 100, 1, 1.0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(
                aggregate_without_link,
                vec![part(0, 1.0)],
                false,
            )
            .unwrap_err(),
            DynamicClusteredMeshTemplateError::MissingRootAggregate {
                part_ordinal: 0,
                root_index: 0,
            }
        );

        let null_child = Arc::new(arena(
            vec![root(
                Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                100,
                1,
                1.0,
            )],
            vec![simple_set(100, vec![aggregate_volume(0, 1, 2.0)])],
            Vec::new(),
            Vec::new(),
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(null_child, vec![part(0, 1.0)], false)
                .unwrap_err(),
            DynamicClusteredMeshTemplateError::NullAggregateTarget
        );

        let null_root_target = Arc::new(arena(
            vec![root(Some(CollisionAggregateRef::Mesh(0)), 0, 1, 1.0)],
            Vec::new(),
            Vec::new(),
            vec![mesh(200, 1)],
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(null_root_target, vec![part(0, 1.0)], false,)
                .unwrap_err(),
            DynamicClusteredMeshTemplateError::NullAggregateTarget
        );

        let mismatched_root_link = Arc::new(arena(
            vec![root(Some(CollisionAggregateRef::Mesh(0)), 201, 1, 1.0)],
            Vec::new(),
            Vec::new(),
            vec![mesh(200, 1), mesh(201, 2)],
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(mismatched_root_link, vec![part(0, 1.0)], false,)
                .unwrap_err(),
            DynamicClusteredMeshTemplateError::RootAggregateLinkMismatch {
                part_ordinal: 0,
                root_index: 0,
                linked: CollisionAggregateRef::Mesh(0),
                resolved: CollisionAggregateRef::Mesh(1),
            }
        );

        let ambiguous_root_target = Arc::new(arena(
            vec![root(
                Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                100,
                1,
                1.0,
            )],
            vec![simple_set(100, Vec::new())],
            Vec::new(),
            vec![mesh(100, 1)],
        ));
        assert_eq!(
            DynamicClusteredMeshTemplate::try_new(
                ambiguous_root_target,
                vec![part(0, 1.0)],
                false,
            )
            .unwrap_err(),
            DynamicClusteredMeshTemplateError::AmbiguousAggregateTarget {
                dictionary_index: 100,
            }
        );
    }

    #[test]
    fn retains_part_metadata_and_arc_backing_without_copying_mesh_bytes() {
        let source = Arc::new(arena(
            vec![root(Some(CollisionAggregateRef::Mesh(0)), 200, 1, 10.0)],
            Vec::new(),
            Vec::new(),
            vec![mesh(200, 0x5a), mesh(201, 0xa5)],
        ));
        let weak: Weak<CollisionArena> = Arc::downgrade(&source);
        let bytes_pointer = source.meshes[0].object_bytes.as_ptr();
        let bytes_len = source.meshes[0].object_bytes.len();
        let bytes_capacity = source.meshes[0].object_bytes.capacity();
        let metadata = part(0, 37.0);
        let template =
            DynamicClusteredMeshTemplate::try_new(Arc::clone(&source), vec![metadata], false)
                .unwrap();
        drop(source);

        assert!(weak.upgrade().is_some());
        assert_eq!(template.parts(), &[metadata]);
        let retained = &template.mesh_at(0).unwrap().object_bytes;
        assert_eq!(retained.as_ptr(), bytes_pointer);
        assert_eq!(retained.len(), bytes_len);
        assert_eq!(retained.capacity(), bytes_capacity);

        // `1` is a valid arena mesh index, but not a valid emitted-leaf
        // ordinal. The accessor cannot be tricked with a foreign descriptor.
        assert!(template.mesh_at(1).is_none());

        let cloned = template.clone();
        assert!(Arc::ptr_eq(template.arena_owner(), cloned.arena_owner()));
        assert_eq!(
            cloned.mesh_at(0).unwrap().object_bytes.as_ptr(),
            bytes_pointer
        );
    }

    #[test]
    fn template_is_send_sync_and_static() {
        fn assert_send_sync_static<T: Send + Sync + 'static>() {}
        assert_send_sync_static::<DynamicClusteredMeshTemplate>();
    }
}
