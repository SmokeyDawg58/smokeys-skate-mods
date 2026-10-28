//! Simulation-stream collision and grind projection from a decoded RW4 arena.
//!
//! This is the owned Rust projection previously local to `skate2-pc`. It
//! preserves the app's dictionary-order spline, geometry, model, and instance
//! passes while retaining the complete [`DecodedArenaAsset`], including all
//! unsupported dictionary objects and all five resource buffers.
//!
//! The native stages continue beyond this boundary. In particular,
//! `cProcessArenaAsset::ProcessArena` (`0x82d5fee0`), `ProcessSplineList`
//! (`0x82d5f180`), `ProcessInstanceList` (`0x82d5ec38`), and
//! `Simulation::RegArena` (`0x828a01e0`) perform registration side effects
//! that are not modeled here. This module does not claim native allocator,
//! TOC publication, world registration, or destruction behavior.

use crate::{
    catalog_resident_load_factory::CollectionAssetBuilderSessionFactory,
    decoded_arena_asset::{DecodedArenaAsset, DecodedArenaAssetBuilder, DecodedArenaAssetError},
    game_stream::StreamKind,
    grind::GrindAsset,
    owned_resident_collection::ResidentAssetIdentity,
    resident_collection_load_driver::{CollectionAssetBuildInput, CollectionAssetBuilder},
    world::{
        CollisionAggregateRef, CollisionArena, CollisionInstance, CollisionMesh, CollisionModel,
        CollisionPrimitiveSet, CollisionRoot, CollisionSimplePrimitiveSet,
    },
    world_stream_catalog::{
        AssetRegistrationSnapshot, CatalogError, CollectionRegistrationSnapshot, WorldStreamCatalog,
    },
};
use skate2_formats::{
    collision::{
        ClusteredMesh, CollisionError, KdTreeMappedArray, ObjectType as CollisionObjectType,
        SimpleMappedArray, Volume,
    },
    pegasus::{
        C_MODEL_DATA_TYPE_ID, CollisionModelData, CollisionModelMeshes, INSTANCE_DATA_TYPE_ID,
        Instance, InstanceData, ObjectId, PegasusError, SPLINE_DATA_TYPE_ID, SplineData,
    },
    rw4::Arena,
};
use std::{collections::BTreeMap, error::Error, fmt};

type ObjectIndexMap = BTreeMap<u32, usize>;

/// One decoded Simulation arena plus its collision/grind-only owned view.
///
/// Keeping `decoded` is intentional: entries not consumed by the collision
/// projection remain available to later, separately evidenced subsystem
/// processors.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationCollisionAsset {
    pub decoded: DecodedArenaAsset,
    pub collision: CollisionArena,
}

impl SimulationCollisionAsset {
    /// Projects the currently supported collision/grind types, then moves the
    /// complete decoded arena into the resulting owner.
    ///
    /// This constructor cannot itself prove that its input came from a
    /// Simulation stream because [`DecodedArenaAsset`] carries no stream-kind
    /// tag. [`SimulationCollisionAssetBuilderSessions`] performs that check for
    /// the resident-streaming composition before creating a checked builder.
    ///
    /// # Errors
    ///
    /// Returns a typed error for an invalid supported object or an unresolved
    /// local collision reference. Unsupported valid objects remain untouched
    /// in `decoded`.
    pub fn from_decoded(decoded: DecodedArenaAsset) -> Result<Self, SimulationCollisionAssetError> {
        let collision = project_collision_arena(&decoded.arena, &decoded.resources.resources)?;
        Ok(Self { decoded, collision })
    }
}

impl ResidentAssetIdentity for SimulationCollisionAsset {
    fn resident_asset_id(&self) -> crate::collection_resident_streamer::CollectionId {
        crate::collection_resident_streamer::CollectionId(self.decoded.asset_id)
    }
}

/// Builder created only after its owning registered collection and file are
/// proven to be a Simulation stream.
///
/// The private field and lack of a public constructor prevent construction
/// without a successful session check. The build input itself carries no
/// collection kind; normal catalog-factory composition owns the association
/// between one checked session and its collection records.
#[derive(Clone, Debug)]
pub struct SimulationCollisionAssetBuilder {
    decoder: DecodedArenaAssetBuilder,
}

impl CollectionAssetBuilder<AssetRegistrationSnapshot> for SimulationCollisionAssetBuilder {
    type Asset = SimulationCollisionAsset;
    type Error = SimulationCollisionAssetBuildError;

    fn build(
        &mut self,
        input: CollectionAssetBuildInput<'_, AssetRegistrationSnapshot>,
    ) -> Result<Option<Self::Asset>, Self::Error> {
        let decoded = self
            .decoder
            .build(input)
            .map_err(SimulationCollisionAssetBuildError::Decode)?;
        decoded
            .map(SimulationCollisionAsset::from_decoded)
            .transpose()
            .map_err(SimulationCollisionAssetBuildError::Projection)
    }
}

/// Cloneable session factory sharing the registration catalog used by the
/// resident load factory.
#[derive(Clone, Debug)]
pub struct SimulationCollisionAssetBuilderSessions {
    catalog: WorldStreamCatalog,
}

impl SimulationCollisionAssetBuilderSessions {
    #[must_use]
    pub const fn new(catalog: WorldStreamCatalog) -> Self {
        Self { catalog }
    }

    #[must_use]
    pub const fn catalog(&self) -> &WorldStreamCatalog {
        &self.catalog
    }
}

impl CollectionAssetBuilderSessionFactory for SimulationCollisionAssetBuilderSessions {
    type Asset = SimulationCollisionAsset;
    type Builder = SimulationCollisionAssetBuilder;
    type Error = SimulationCollisionAssetSessionError;

    fn begin_session(
        &mut self,
        collection: &CollectionRegistrationSnapshot,
    ) -> Result<Self::Builder, Self::Error> {
        let registered = self
            .catalog
            .collection(collection.id)
            .map_err(SimulationCollisionAssetSessionError::Catalog)?
            .ok_or(SimulationCollisionAssetSessionError::UnknownCollection(
                collection.id,
            ))?;
        if &registered != collection {
            return Err(
                SimulationCollisionAssetSessionError::CollectionSnapshotMismatch {
                    id: collection.id,
                },
            );
        }
        let file = self
            .catalog
            .file(&registered.owner)
            .map_err(SimulationCollisionAssetSessionError::Catalog)?
            .ok_or(SimulationCollisionAssetSessionError::UnknownFileOwner)?;
        if file.kind != StreamKind::Simulation {
            return Err(SimulationCollisionAssetSessionError::WrongStreamKind(
                file.kind,
            ));
        }
        Ok(SimulationCollisionAssetBuilder {
            decoder: DecodedArenaAssetBuilder,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationCollisionAssetBuildError {
    Decode(DecodedArenaAssetError),
    Projection(SimulationCollisionAssetError),
}

impl fmt::Display for SimulationCollisionAssetBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(source) => write!(formatter, "Simulation arena decoding failed: {source}"),
            Self::Projection(source) => {
                write!(
                    formatter,
                    "Simulation collision projection failed: {source}"
                )
            }
        }
    }
}

impl Error for SimulationCollisionAssetBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(source) => Some(source),
            Self::Projection(source) => Some(source),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationCollisionAssetSessionError {
    Catalog(CatalogError),
    UnknownCollection(u64),
    CollectionSnapshotMismatch { id: u64 },
    UnknownFileOwner,
    WrongStreamKind(StreamKind),
}

impl fmt::Display for SimulationCollisionAssetSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(source) => write!(formatter, "stream catalog lookup failed: {source}"),
            Self::UnknownCollection(id) => {
                write!(
                    formatter,
                    "collection {id:#018x} is absent from the catalog"
                )
            }
            Self::CollectionSnapshotMismatch { id } => write!(
                formatter,
                "collection {id:#018x} snapshot differs from the registered collection"
            ),
            Self::UnknownFileOwner => {
                formatter.write_str("collection registration owner is absent from the catalog")
            }
            Self::WrongStreamKind(kind) => {
                write!(
                    formatter,
                    "collision projection requires Simulation, observed {kind:?}"
                )
            }
        }
    }
}

impl Error for SimulationCollisionAssetSessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(source) => Some(source),
            Self::UnknownCollection(_)
            | Self::CollectionSnapshotMismatch { .. }
            | Self::UnknownFileOwner
            | Self::WrongStreamKind(_) => None,
        }
    }
}

/// Typed local relationship resolved by the collision projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationCollisionRelationship {
    Aggregate,
    CollisionModelBoundingRoot,
    CollisionModelMeshRoot,
    InstanceCollisionModel,
}

impl fmt::Display for SimulationCollisionRelationship {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Aggregate => "aggregate",
            Self::CollisionModelBoundingRoot => "collision-model bounding root",
            Self::CollisionModelMeshRoot => "collision-model mesh root",
            Self::InstanceCollisionModel => "instance collision model",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationCollisionAssetError {
    DictionaryIndexOverflow(usize),
    ObjectRangeOutsideResource {
        arena_id: u32,
        dictionary_index: usize,
        type_id: u32,
    },
    NonLocalObjectReference {
        relationship: SimulationCollisionRelationship,
        object_id: u32,
        section: u16,
    },
    MissingLocalObjectReference {
        relationship: SimulationCollisionRelationship,
        object_id: u32,
        entry_index: u32,
    },
    Collision(CollisionError),
    Pegasus(PegasusError),
}

impl fmt::Display for SimulationCollisionAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DictionaryIndexOverflow(index) => {
                write!(
                    formatter,
                    "arena dictionary index {index} does not fit in u32"
                )
            }
            Self::ObjectRangeOutsideResource {
                arena_id,
                dictionary_index,
                type_id,
            } => write!(
                formatter,
                "arena {arena_id:08x} dictionary entry {dictionary_index} type {type_id:08x} is outside its retained resource"
            ),
            Self::NonLocalObjectReference {
                relationship,
                object_id,
                section,
            } => write!(
                formatter,
                "{relationship} uses nonlocal object ID {object_id:08x} in section {section}"
            ),
            Self::MissingLocalObjectReference {
                relationship,
                object_id,
                entry_index,
            } => write!(
                formatter,
                "{relationship} object ID {object_id:08x} targets missing local entry {entry_index}"
            ),
            Self::Collision(source) => write!(formatter, "collision projection failed: {source}"),
            Self::Pegasus(source) => write!(formatter, "Pegasus projection failed: {source}"),
        }
    }
}

impl Error for SimulationCollisionAssetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Collision(source) => Some(source),
            Self::Pegasus(source) => Some(source),
            Self::DictionaryIndexOverflow(_)
            | Self::ObjectRangeOutsideResource { .. }
            | Self::NonLocalObjectReference { .. }
            | Self::MissingLocalObjectReference { .. } => None,
        }
    }
}

impl From<CollisionError> for SimulationCollisionAssetError {
    fn from(source: CollisionError) -> Self {
        Self::Collision(source)
    }
}

impl From<PegasusError> for SimulationCollisionAssetError {
    fn from(source: PegasusError) -> Self {
        Self::Pegasus(source)
    }
}

struct CollisionGeometryEntries {
    roots: ObjectIndexMap,
    primitive_sets: ObjectIndexMap,
    simple_primitive_sets: ObjectIndexMap,
    meshes: ObjectIndexMap,
}

#[derive(Debug, Eq, PartialEq)]
struct ResolvedCollisionModelLinks {
    bounding_root: Option<usize>,
    collision_roots: Vec<usize>,
    assembly_object_id: Option<u32>,
}

pub(crate) fn project_collision_arena(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
) -> Result<CollisionArena, SimulationCollisionAssetError> {
    let mut owned = CollisionArena {
        id: arena.id,
        grind_assets: Vec::new(),
        aggregate_roots: Vec::new(),
        primitive_sets: Vec::new(),
        simple_primitive_sets: Vec::new(),
        meshes: Vec::new(),
        collision_models: Vec::new(),
        instances: Vec::new(),
    };

    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != SPLINE_DATA_TYPE_ID {
            continue;
        }
        let bytes = object_bytes(arena, resources, entry_index, entry)?;
        let dictionary_index = dictionary_index(entry_index)?;
        let data = SplineData::parse(bytes)?;
        // This preserves the existing PC diagnostic scene's arena-ID source
        // tag. Native registration instead uses the enclosing TOC entry's
        // full 64-bit GUID; reconstructing that TOC-ordered registration stays
        // outside this dictionary-order projection.
        owned.grind_assets.push(GrindAsset::from_spline_data(
            u64::from(arena.id),
            dictionary_index,
            &data,
        ));
    }

    let geometry_entries = project_collision_geometry(arena, resources, &mut owned)?;
    link_collision_roots(
        arena.id,
        &mut owned.aggregate_roots,
        &geometry_entries.primitive_sets,
        &geometry_entries.simple_primitive_sets,
        &geometry_entries.meshes,
    )?;
    let model_entries =
        project_collision_models(arena, resources, &geometry_entries.roots, &mut owned)?;
    project_collision_instances(arena, resources, &model_entries, &mut owned)?;
    Ok(owned)
}

fn project_collision_geometry(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    owned: &mut CollisionArena,
) -> Result<CollisionGeometryEntries, SimulationCollisionAssetError> {
    let mut root_entries = BTreeMap::new();
    let mut primitive_set_entries = BTreeMap::new();
    let mut simple_primitive_set_entries = BTreeMap::new();
    let mut mesh_entries = BTreeMap::new();

    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        let bytes = object_bytes(arena, resources, entry_index, entry)?;
        let dictionary_index = dictionary_index(entry_index)?;
        match CollisionObjectType::from_type_id(entry.type_id) {
            Some(CollisionObjectType::Volume) => {
                root_entries.insert(dictionary_index, owned.aggregate_roots.len());
                owned.aggregate_roots.push(CollisionRoot {
                    dictionary_index,
                    volume: Volume::parse(bytes)?,
                    aggregate: None,
                });
            }
            Some(CollisionObjectType::KdTreeMappedArray) => {
                let mapped = KdTreeMappedArray::parse(bytes)?;
                primitive_set_entries.insert(dictionary_index, owned.primitive_sets.len());
                owned.primitive_sets.push(CollisionPrimitiveSet {
                    dictionary_index,
                    bounding_box_min: xyz(mapped.bounding_box_min),
                    bounding_box_max: xyz(mapped.bounding_box_max),
                    volumes: mapped.volumes(bytes)?,
                    mapped_array: mapped,
                    object_bytes: bytes.to_vec(),
                });
            }
            Some(CollisionObjectType::SimpleMappedArray) => {
                let mapped = SimpleMappedArray::parse(bytes)?;
                simple_primitive_set_entries
                    .insert(dictionary_index, owned.simple_primitive_sets.len());
                owned
                    .simple_primitive_sets
                    .push(CollisionSimplePrimitiveSet {
                        dictionary_index,
                        bounding_box_min: xyz(mapped.bounding_box_min),
                        bounding_box_max: xyz(mapped.bounding_box_max),
                        volumes: mapped.volumes(bytes)?,
                    });
            }
            Some(CollisionObjectType::ClusteredMesh) => {
                let mesh = ClusteredMesh::parse(bytes)?;
                mesh_entries.insert(dictionary_index, owned.meshes.len());
                let minimum = xyz(mesh.bounding_box_min);
                let maximum = xyz(mesh.bounding_box_max);
                let ranges = mesh
                    .kd_tree(bytes)?
                    .leaf_ranges_for_aabb(bytes, minimum, maximum)?;
                owned.meshes.push(CollisionMesh {
                    dictionary_index,
                    bounding_box_min: minimum,
                    bounding_box_max: maximum,
                    triangles: mesh.triangles_for_leaf_ranges(bytes, &ranges)?,
                    clustered_mesh: mesh,
                    object_bytes: bytes.to_vec(),
                });
            }
            Some(
                CollisionObjectType::TriangleKdTreeProcedural | CollisionObjectType::BoundingBox,
            )
            | None => {}
        }
    }

    Ok(CollisionGeometryEntries {
        roots: root_entries,
        primitive_sets: primitive_set_entries,
        simple_primitive_sets: simple_primitive_set_entries,
        meshes: mesh_entries,
    })
}

fn link_collision_roots(
    _arena_id: u32,
    roots: &mut [CollisionRoot],
    primitive_set_entries: &ObjectIndexMap,
    simple_primitive_set_entries: &ObjectIndexMap,
    mesh_entries: &ObjectIndexMap,
) -> Result<(), SimulationCollisionAssetError> {
    for root in roots {
        let Some(id) = root.volume.aggregate_object_id().map(ObjectId) else {
            continue;
        };
        let relationship = SimulationCollisionRelationship::Aggregate;
        let entry_index = require_local_entry(id, relationship)?;
        root.aggregate = if let Some(index) = primitive_set_entries.get(&entry_index) {
            Some(CollisionAggregateRef::PrimitiveSet(*index))
        } else if let Some(index) = simple_primitive_set_entries.get(&entry_index) {
            Some(CollisionAggregateRef::SimplePrimitiveSet(*index))
        } else if let Some(index) = mesh_entries.get(&entry_index) {
            Some(CollisionAggregateRef::Mesh(*index))
        } else {
            return Err(SimulationCollisionAssetError::MissingLocalObjectReference {
                relationship,
                object_id: id.0,
                entry_index,
            });
        };
    }
    Ok(())
}

fn project_collision_models(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    root_entries: &ObjectIndexMap,
    owned: &mut CollisionArena,
) -> Result<ObjectIndexMap, SimulationCollisionAssetError> {
    let mut model_entries = BTreeMap::new();
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != C_MODEL_DATA_TYPE_ID {
            continue;
        }
        let bytes = object_bytes(arena, resources, entry_index, entry)?;
        let dictionary_index = dictionary_index(entry_index)?;
        let data = CollisionModelData::parse(bytes)?;
        let links = resolve_collision_model_links(data, root_entries)?;
        model_entries.insert(dictionary_index, owned.collision_models.len());
        owned.collision_models.push(CollisionModel {
            dictionary_index,
            bounding_root: links.bounding_root,
            collision_roots: links.collision_roots,
            assembly_object_id: links.assembly_object_id,
        });
    }
    Ok(model_entries)
}

fn resolve_collision_model_links(
    data: CollisionModelData,
    root_entries: &ObjectIndexMap,
) -> Result<ResolvedCollisionModelLinks, SimulationCollisionAssetError> {
    let bounding_root = data
        .bounding_volume
        .map(|id| {
            resolve_local_entry(
                id,
                root_entries,
                SimulationCollisionRelationship::CollisionModelBoundingRoot,
            )
        })
        .transpose()?;
    let (collision_roots, assembly_object_id) = match data.meshes {
        CollisionModelMeshes::Assembly(id) => (Vec::new(), Some(id.0)),
        CollisionModelMeshes::Meshes(meshes) => (
            meshes
                .into_iter()
                .map(|mesh| {
                    resolve_local_entry(
                        mesh.mesh,
                        root_entries,
                        SimulationCollisionRelationship::CollisionModelMeshRoot,
                    )
                })
                .collect::<Result<_, _>>()?,
            None,
        ),
    };
    Ok(ResolvedCollisionModelLinks {
        bounding_root,
        collision_roots,
        assembly_object_id,
    })
}

fn project_collision_instances(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    model_entries: &ObjectIndexMap,
    owned: &mut CollisionArena,
) -> Result<(), SimulationCollisionAssetError> {
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != INSTANCE_DATA_TYPE_ID {
            continue;
        }
        let bytes = object_bytes(arena, resources, entry_index, entry)?;
        for instance in InstanceData::parse(bytes)?.instances {
            owned
                .instances
                .push(project_collision_instance(instance, model_entries)?);
        }
    }
    Ok(())
}

fn object_bytes<'a>(
    arena: &Arena,
    resources: &'a [Vec<u8>; 5],
    dictionary_index: usize,
    entry: &skate2_formats::rw4::DictionaryEntry,
) -> Result<&'a [u8], SimulationCollisionAssetError> {
    arena.object_bytes(resources, entry).ok_or(
        SimulationCollisionAssetError::ObjectRangeOutsideResource {
            arena_id: arena.id,
            dictionary_index,
            type_id: entry.type_id,
        },
    )
}

fn project_collision_instance(
    instance: Instance,
    model_entries: &ObjectIndexMap,
) -> Result<CollisionInstance, SimulationCollisionAssetError> {
    let collision_model = instance
        .collision_model
        .map(|id| {
            resolve_local_entry(
                id,
                model_entries,
                SimulationCollisionRelationship::InstanceCollisionModel,
            )
        })
        .transpose()?;
    Ok(CollisionInstance {
        guid: instance.guid,
        local_guid: instance.local_guid,
        name: instance.name,
        transform: instance.transform,
        bounding_box_min: xyz(instance.bounding_box_min),
        bounding_box_max: xyz(instance.bounding_box_max),
        collision_model,
    })
}

fn dictionary_index(index: usize) -> Result<u32, SimulationCollisionAssetError> {
    u32::try_from(index).map_err(|_| SimulationCollisionAssetError::DictionaryIndexOverflow(index))
}

fn require_local_entry(
    id: ObjectId,
    relationship: SimulationCollisionRelationship,
) -> Result<u32, SimulationCollisionAssetError> {
    let section = id.section_index();
    if section != 0 {
        return Err(SimulationCollisionAssetError::NonLocalObjectReference {
            relationship,
            object_id: id.0,
            section,
        });
    }
    Ok(id.entry_index())
}

fn resolve_local_entry(
    id: ObjectId,
    entries: &ObjectIndexMap,
    relationship: SimulationCollisionRelationship,
) -> Result<usize, SimulationCollisionAssetError> {
    let entry_index = require_local_entry(id, relationship)?;
    entries.get(&entry_index).copied().ok_or(
        SimulationCollisionAssetError::MissingLocalObjectReference {
            relationship,
            object_id: id.0,
            entry_index,
        },
    )
}

const fn xyz(vector: [f32; 4]) -> [f32; 3] {
    [vector[0], vector[1], vector[2]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game_stream::StreamFile, mounted_stream_backend::StreamRegistrationSink};
    use skate2_formats::{
        pegasus::CollisionModelMesh,
        rw4::Arena,
        stream::{ArenaResources, CollectionMap, CollectionMapEntry},
    };

    type TestFile = StreamFile<(), CatalogError>;

    fn registered_collection(
        kind: StreamKind,
    ) -> (WorldStreamCatalog, CollectionRegistrationSnapshot) {
        let file = TestFile::load("world", "boot", kind).unwrap();
        let map = CollectionMap {
            version: 3,
            stamp: 0x1020_3040_5060_7080,
            stream_format: 2,
            name_table_offset: 0,
            collections: vec![CollectionMapEntry {
                id: 7,
                parent_id: 8,
                payload_offset: 0x100,
                payload_size: 0x200,
                asset_count: 1,
                filename: "cell".to_owned(),
                name_offset: 0,
                alignment_pad: -1,
            }],
        };
        let mut catalog = WorldStreamCatalog::new();
        catalog
            .collection_map_loaded(file.registration_context(), &map)
            .unwrap();
        let collection = catalog.collection(7).unwrap().unwrap();
        (catalog, collection)
    }

    fn opaque_decoded_asset() -> DecodedArenaAsset {
        let mut resource_zero = vec![0_u8; 0x200];
        resource_zero[..12].copy_from_slice(b"\x89RW4xb2\0\r\n\x1a\n");
        resource_zero[12] = 1;
        resource_zero[13] = 32;
        resource_zero[14] = 4;
        resource_zero[16..20].copy_from_slice(b"454\0");
        resource_zero[20..24].copy_from_slice(b"000\0");
        resource_zero[28..32].copy_from_slice(&0x8646_a1f7_u32.to_be_bytes());
        resource_zero[32..36].copy_from_slice(&1_u32.to_be_bytes());
        resource_zero[36..40].copy_from_slice(&1_u32.to_be_bytes());
        resource_zero[40..44].copy_from_slice(&16_u32.to_be_bytes());
        resource_zero[48..52].copy_from_slice(&0x1e8_u32.to_be_bytes());
        resource_zero[68..72].copy_from_slice(&0x200_u32.to_be_bytes());
        resource_zero[72..76].copy_from_slice(&16_u32.to_be_bytes());
        resource_zero[84..88].copy_from_slice(&3_u32.to_be_bytes());
        resource_zero[88..92].copy_from_slice(&4096_u32.to_be_bytes());
        resource_zero[0x100..0x110].copy_from_slice(b"opaque-stays-put");
        resource_zero[0x1e8..0x1ec].copy_from_slice(&0x100_u32.to_be_bytes());
        resource_zero[0x1f0..0x1f4].copy_from_slice(&0x10_u32.to_be_bytes());
        resource_zero[0x1f4..0x1f8].copy_from_slice(&16_u32.to_be_bytes());
        resource_zero[0x1f8..0x1fc].copy_from_slice(&0x10_u32.to_be_bytes());
        resource_zero[0x1fc..0x200].copy_from_slice(&0x00eb_0008_u32.to_be_bytes());
        let resources = ArenaResources {
            resources: [
                resource_zero,
                Vec::new(),
                vec![0x7f, 0x80, 0xff],
                Vec::new(),
                Vec::new(),
            ],
        };
        let arena = Arena::parse(&resources.resources).unwrap();
        DecodedArenaAsset::from_parts_for_test(0xfedc_ba98_7654_3210, 0xab32_9a6a, resources, arena)
    }

    #[test]
    fn opaque_entries_and_all_decoded_resources_remain_owned() {
        let decoded = opaque_decoded_asset();
        let expected = decoded.clone();
        let projected = SimulationCollisionAsset::from_decoded(decoded).unwrap();

        assert_eq!(projected.decoded, expected);
        assert_eq!(projected.decoded.asset_id, 0xfedc_ba98_7654_3210);
        assert_eq!(
            projected.resident_asset_id(),
            crate::collection_resident_streamer::CollectionId(0xfedc_ba98_7654_3210)
        );
        assert_eq!(projected.decoded.arena.dictionary[0].type_id, 0x00eb_0008);
        assert_eq!(projected.decoded.resources.resources[2], [0x7f, 0x80, 0xff]);
        assert!(projected.collision.grind_assets.is_empty());
        assert!(projected.collision.aggregate_roots.is_empty());
        assert!(projected.collision.primitive_sets.is_empty());
        assert!(projected.collision.simple_primitive_sets.is_empty());
        assert!(projected.collision.meshes.is_empty());
        assert!(projected.collision.collision_models.is_empty());
        assert!(projected.collision.instances.is_empty());
    }

    #[test]
    fn local_reference_resolution_distinguishes_nonlocal_and_missing_targets() {
        let entries = BTreeMap::from([(7, 3)]);
        assert_eq!(
            resolve_local_entry(
                ObjectId(7),
                &entries,
                SimulationCollisionRelationship::Aggregate
            ),
            Ok(3)
        );
        assert_eq!(
            resolve_local_entry(
                ObjectId((2 << 22) | 7),
                &entries,
                SimulationCollisionRelationship::Aggregate
            ),
            Err(SimulationCollisionAssetError::NonLocalObjectReference {
                relationship: SimulationCollisionRelationship::Aggregate,
                object_id: (2 << 22) | 7,
                section: 2,
            })
        );
        assert_eq!(
            resolve_local_entry(
                ObjectId(9),
                &entries,
                SimulationCollisionRelationship::Aggregate
            ),
            Err(SimulationCollisionAssetError::MissingLocalObjectReference {
                relationship: SimulationCollisionRelationship::Aggregate,
                object_id: 9,
                entry_index: 9,
            })
        );
    }

    #[test]
    fn collision_model_mesh_order_is_preserved_and_assembly_remains_opaque() {
        let roots = BTreeMap::from([(4, 11), (7, 13)]);
        let links = resolve_collision_model_links(
            CollisionModelData {
                bounding_volume: Some(ObjectId(7)),
                meshes: CollisionModelMeshes::Meshes(vec![
                    CollisionModelMesh {
                        mesh: ObjectId(7),
                        bone_name: 99,
                    },
                    CollisionModelMesh {
                        mesh: ObjectId(4),
                        bone_name: 100,
                    },
                ]),
            },
            &roots,
        )
        .unwrap();
        assert_eq!(
            links,
            ResolvedCollisionModelLinks {
                bounding_root: Some(13),
                collision_roots: vec![13, 11],
                assembly_object_id: None,
            }
        );

        let assembly = resolve_collision_model_links(
            CollisionModelData {
                bounding_volume: None,
                meshes: CollisionModelMeshes::Assembly(ObjectId(0x00c0_ffee)),
            },
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(
            assembly,
            ResolvedCollisionModelLinks {
                bounding_root: None,
                collision_roots: Vec::new(),
                assembly_object_id: Some(0x00c0_ffee),
            }
        );
    }

    #[test]
    fn xyz_copy_preserves_unusual_first_three_lane_bits() {
        let source = [
            f32::from_bits(0x8000_0000),
            f32::from_bits(0x7fc1_2345),
            f32::from_bits(0xff80_0000),
            f32::from_bits(0x3f80_0000),
        ];
        assert_eq!(
            xyz(source).map(f32::to_bits),
            [0x8000_0000, 0x7fc1_2345, 0xff80_0000]
        );
    }

    #[test]
    fn instance_projection_preserves_all_transform_lane_bits() {
        let transform = std::array::from_fn(|index| {
            f32::from_bits([0x8000_0000, 0x7fc1_2345, 0xff80_0000, 0x0000_0001][index % 4])
        });
        let projected = project_collision_instance(
            Instance {
                transform,
                bounding_box_min: [0.0, 1.0, 2.0, 3.0],
                bounding_box_max: [4.0, 5.0, 6.0, 7.0],
                guid: 11,
                local_guid: 12,
                class_key: 13,
                instance_key: 14,
                render_model: None,
                collision_model: Some(ObjectId(7)),
                embedded_data: None,
                name: "bit fixture".to_owned(),
                description: String::new(),
                component: String::new(),
                category: String::new(),
            },
            &BTreeMap::from([(7, 3)]),
        )
        .unwrap();

        assert_eq!(
            projected.transform.map(f32::to_bits),
            transform.map(f32::to_bits)
        );
        assert_eq!(projected.collision_model, Some(3));
    }

    #[test]
    fn session_factory_checks_builder_against_registered_simulation_owner() {
        let (catalog, collection) = registered_collection(StreamKind::Simulation);
        let mut tampered = collection.clone();
        tampered.offset += 1;
        let mut sessions = SimulationCollisionAssetBuilderSessions::new(catalog);
        sessions.begin_session(&collection).unwrap();
        assert_eq!(
            sessions.begin_session(&tampered).unwrap_err(),
            SimulationCollisionAssetSessionError::CollectionSnapshotMismatch { id: 7 }
        );
    }

    #[test]
    fn session_factory_rejects_non_simulation_and_tampered_owners() {
        let (catalog, presentation) = registered_collection(StreamKind::Presentation);
        let mut sessions = SimulationCollisionAssetBuilderSessions::new(catalog);
        assert_eq!(
            sessions.begin_session(&presentation).unwrap_err(),
            SimulationCollisionAssetSessionError::WrongStreamKind(StreamKind::Presentation)
        );

        let orphan_file = TestFile::load("world", "orphan", StreamKind::Simulation).unwrap();
        let mut orphan = presentation;
        orphan.owner = orphan_file.registration_id().clone();
        assert_eq!(
            sessions.begin_session(&orphan).unwrap_err(),
            SimulationCollisionAssetSessionError::CollectionSnapshotMismatch { id: 7 }
        );
    }

    #[test]
    fn session_factory_rejects_a_snapshot_retargeted_to_a_simulation_file() {
        let (mut catalog, mut presentation) = registered_collection(StreamKind::Presentation);
        let simulation = TestFile::load("world", "simulation", StreamKind::Simulation).unwrap();
        let map = CollectionMap {
            version: 3,
            stamp: 0x1020_3040_5060_7080,
            stream_format: 2,
            name_table_offset: 0,
            collections: vec![CollectionMapEntry {
                id: 8,
                parent_id: 9,
                payload_offset: 0x300,
                payload_size: 0x400,
                asset_count: 1,
                filename: "sim-cell".to_owned(),
                name_offset: 0,
                alignment_pad: -1,
            }],
        };
        catalog
            .collection_map_loaded(simulation.registration_context(), &map)
            .unwrap();
        presentation.owner = simulation.registration_context().id().clone();
        let mut sessions = SimulationCollisionAssetBuilderSessions::new(catalog);

        assert_eq!(
            sessions.begin_session(&presentation).unwrap_err(),
            SimulationCollisionAssetSessionError::CollectionSnapshotMismatch { id: 7 }
        );
    }
}
