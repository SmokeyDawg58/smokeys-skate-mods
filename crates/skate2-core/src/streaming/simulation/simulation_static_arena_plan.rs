//! Native-order static Simulation actions expanded from one processed arena.
//!
//! `cProcessArenaAsset::ProcessArena` (`0x82d5fee0`) walks the first
//! used-dictionary TOC in authored order and invokes each processor immediately. This bounded
//! plan reconstructs only the two processed classes needed by the selected
//! Rob's Park static-collision corpus: `ProcessSplineList` (`0x82d5f180`)
//! producing Simulation kind 6 and the Environment branch of
//! `ProcessInstanceList` (`0x82d5ec38`) producing kind 10. Other native
//! processed classes remain owned by the retained arena but are not actions in
//! this deliberately narrow plan.
//! `ProcessArena`'s single-Texture fast path and runtime
//! `PreventNavmeshLoadingHack` bypass remain outside this Simulation-static
//! corpus slice.

use crate::{grind::GrindAsset, simulation_collision_asset::SimulationCollisionAsset};
use skate2_formats::{
    pegasus::{
        C_MODEL_DATA_TYPE_ID, CollisionModelData, INSTANCE_DATA_TYPE_ID,
        INSTANCE_ENVIRONMENT_ASSET_TYPE_ID, Instance, InstanceData, ObjectId, PegasusError,
        SPLINE_DATA_TYPE_ID, SplineData, TABLE_OF_CONTENTS_TYPE_ID, TableOfContents,
    },
    render::{R_MODEL_DATA_TYPE_ID, RenderError, RenderModelData},
    rw4::{Arena, DictionaryEntry},
};
use std::{collections::BTreeMap, error::Error, fmt};

/// One owned arena and its native-order, static Simulation registration slice.
///
/// Operations contain owned values or stable indexes into `asset` or
/// `grind_assets`; no borrow points into relocatable decoded storage. The
/// complete decoded arena and its dictionary-order diagnostic collision view
/// remain unchanged for later processors and deregistration.
#[derive(Debug, PartialEq)]
pub struct SimulationStaticArenaPlan {
    asset: SimulationCollisionAsset,
    grind_assets: Vec<GrindAsset>,
    operations: Vec<SimulationStaticOperation>,
}

impl SimulationStaticArenaPlan {
    /// Expands kind-6 grind and kind-10 Environment actions in authored TOC
    /// order.
    ///
    /// The plan uses the first used dictionary TOC, matching the recovered
    /// first-match walk; all 68 selected Rob's Park Simulation arenas contain
    /// exactly one. An arena without a TOC produces an empty plan, as the
    /// reference constructs an empty `cArena` in that case.
    ///
    /// # Errors
    ///
    /// Returns a typed error for an unsupported nonlocal reference, a
    /// mismatched target type, malformed supported data, or disagreement with
    /// the retained dictionary-order collision view.
    pub fn from_collision_asset(
        asset: SimulationCollisionAsset,
    ) -> Result<Self, SimulationStaticArenaPlanError> {
        let (grind_assets, operations) = build_operations(&asset)?;
        Ok(Self {
            asset,
            grind_assets,
            operations,
        })
    }

    #[must_use]
    pub const fn asset(&self) -> &SimulationCollisionAsset {
        &self.asset
    }

    /// TOC-order grind assets addressed by [`SimulationStaticOperation::Grind`].
    ///
    /// These use the direct full TOC GUID. They are deliberately separate
    /// from `asset().collision.grind_assets`, whose established diagnostic
    /// contract remains dictionary order with arena-ID source labels.
    #[must_use]
    pub fn grind_assets(&self) -> &[GrindAsset] {
        &self.grind_assets
    }

    #[must_use]
    pub fn operations(&self) -> &[SimulationStaticOperation] {
        &self.operations
    }

    #[must_use]
    pub fn into_asset(self) -> SimulationCollisionAsset {
        self.asset
    }

    #[cfg(test)]
    pub(crate) fn from_parts_for_test(
        asset: SimulationCollisionAsset,
        operations: Vec<SimulationStaticOperation>,
    ) -> Self {
        Self::from_parts_with_grind_for_test(asset, Vec::new(), operations)
    }

    #[cfg(test)]
    pub(crate) fn from_parts_with_grind_for_test(
        asset: SimulationCollisionAsset,
        grind_assets: Vec<GrindAsset>,
        operations: Vec<SimulationStaticOperation>,
    ) -> Self {
        Self {
            asset,
            grind_assets,
            operations,
        }
    }
}

/// One action from the exact order-preserving static subset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationStaticOperation {
    /// Simulation kind 6. Native identity is the direct TOC GUID, not an arena
    /// ID or a `0x00eb0064` spline subreference GUID.
    Grind {
        source_toc_index: u32,
        source_dictionary_index: u32,
        /// Index into [`SimulationStaticArenaPlan::grind_assets`].
        grind_asset_index: usize,
    },
    /// Simulation kind 10. Native identity is the inline instance GUID.
    Environment {
        source_toc_index: u32,
        source_dictionary_index: u32,
        source_inline_index: u32,
        /// Backing resource containing the exact inline `tInstance` record.
        source_resource_index: usize,
        /// Absolute byte start of that record within `source_resource_index`.
        source_record_start: usize,
        /// Index into `SimulationStaticArenaPlan::asset().collision.instances`.
        instance_index: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationStaticReference {
    TocTarget,
    CollisionModel,
    RenderModel,
}

impl fmt::Display for SimulationStaticReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TocTarget => "TOC target",
            Self::CollisionModel => "collision model",
            Self::RenderModel => "render model",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationStaticArenaPlanError {
    TocIndexOverflow(usize),
    InlineIndexOverflow(usize),
    NonLocalObjectReference {
        reference: SimulationStaticReference,
        object_id: u32,
        section: u16,
    },
    MissingLocalObjectReference {
        reference: SimulationStaticReference,
        object_id: u32,
        entry_index: u32,
    },
    WrongLocalObjectType {
        reference: SimulationStaticReference,
        object_id: u32,
        expected: u32,
        actual: u32,
    },
    ObjectRangeOutsideResource {
        dictionary_index: u32,
        type_id: u32,
    },
    CollisionInstanceCountMismatch {
        projected: usize,
        indexed: usize,
    },
    MissingProjectedInstance {
        dictionary_index: u32,
        inline_index: u32,
    },
    ProjectedInstanceMismatch {
        collision_instance_index: usize,
    },
    IndexOverflow,
    Pegasus(PegasusError),
    Render(RenderError),
}

impl fmt::Display for SimulationStaticArenaPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TocIndexOverflow(index) => {
                write!(formatter, "TOC entry index {index} does not fit in u32")
            }
            Self::InlineIndexOverflow(index) => {
                write!(
                    formatter,
                    "inline instance index {index} does not fit in u32"
                )
            }
            Self::NonLocalObjectReference {
                reference,
                object_id,
                section,
            } => write!(
                formatter,
                "{reference} uses unsupported nonlocal object ID {object_id:08x} in section {section}"
            ),
            Self::MissingLocalObjectReference {
                reference,
                object_id,
                entry_index,
            } => write!(
                formatter,
                "{reference} object ID {object_id:08x} targets missing local entry {entry_index}"
            ),
            Self::WrongLocalObjectType {
                reference,
                object_id,
                expected,
                actual,
            } => write!(
                formatter,
                "{reference} object ID {object_id:08x} has type {actual:08x}, expected {expected:08x}"
            ),
            Self::ObjectRangeOutsideResource {
                dictionary_index,
                type_id,
            } => write!(
                formatter,
                "dictionary entry {dictionary_index} type {type_id:08x} is outside its retained resource"
            ),
            Self::CollisionInstanceCountMismatch { projected, indexed } => write!(
                formatter,
                "collision projection contains {projected} instances but dictionary indexing found {indexed}"
            ),
            Self::MissingProjectedInstance {
                dictionary_index,
                inline_index,
            } => write!(
                formatter,
                "instance container {dictionary_index} inline record {inline_index} has no projected collision instance"
            ),
            Self::ProjectedInstanceMismatch {
                collision_instance_index,
            } => write!(
                formatter,
                "projected collision instance {collision_instance_index} disagrees with its decoded source"
            ),
            Self::IndexOverflow => {
                formatter.write_str("static arena plan index arithmetic overflowed")
            }
            Self::Pegasus(source) => {
                write!(formatter, "processed Pegasus data is invalid: {source}")
            }
            Self::Render(source) => write!(
                formatter,
                "processed render-model data is invalid: {source}"
            ),
        }
    }
}

impl Error for SimulationStaticArenaPlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Pegasus(source) => Some(source),
            Self::Render(source) => Some(source),
            _ => None,
        }
    }
}

impl From<PegasusError> for SimulationStaticArenaPlanError {
    fn from(source: PegasusError) -> Self {
        Self::Pegasus(source)
    }
}

impl From<RenderError> for SimulationStaticArenaPlanError {
    fn from(source: RenderError) -> Self {
        Self::Render(source)
    }
}

fn build_operations(
    asset: &SimulationCollisionAsset,
) -> Result<(Vec<GrindAsset>, Vec<SimulationStaticOperation>), SimulationStaticArenaPlanError> {
    let arena = &asset.decoded.arena;
    let resources = &asset.decoded.resources.resources;
    let Some((toc_dictionary_index, toc_entry)) = arena
        .dictionary
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID)
    else {
        return Ok((Vec::new(), Vec::new()));
    };
    let toc = TableOfContents::parse(object_bytes(
        arena,
        resources,
        toc_dictionary_index,
        toc_entry,
    )?)?;
    let instance_bases = projected_instance_bases(asset)?;
    let collision_models: BTreeMap<_, _> = asset
        .collision
        .collision_models
        .iter()
        .enumerate()
        .map(|(index, model)| (model.dictionary_index, index))
        .collect();
    let mut grind_assets = Vec::new();
    let mut operations = Vec::new();

    for (toc_index, entry) in toc.entries.into_iter().enumerate() {
        let source_toc_index = u32::try_from(toc_index)
            .map_err(|_| SimulationStaticArenaPlanError::TocIndexOverflow(toc_index))?;
        append_toc_entry(
            asset,
            arena,
            resources,
            &instance_bases,
            &collision_models,
            source_toc_index,
            &entry,
            &mut grind_assets,
            &mut operations,
        )?;
    }
    Ok((grind_assets, operations))
}

#[allow(clippy::too_many_arguments)]
fn append_toc_entry(
    asset: &SimulationCollisionAsset,
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    instance_bases: &BTreeMap<u32, usize>,
    collision_models: &BTreeMap<u32, usize>,
    source_toc_index: u32,
    entry: &skate2_formats::pegasus::TocEntry,
    grind_assets: &mut Vec<GrindAsset>,
    operations: &mut Vec<SimulationStaticOperation>,
) -> Result<(), SimulationStaticArenaPlanError> {
    match entry.object_type {
        SPLINE_DATA_TYPE_ID => {
            let (dictionary_index, bytes) = resolve_typed_object(
                arena,
                resources,
                entry.object,
                SPLINE_DATA_TYPE_ID,
                SimulationStaticReference::TocTarget,
            )?;
            let data = SplineData::parse(bytes)?;
            let grind_asset_index = grind_assets.len();
            grind_assets.push(GrindAsset::from_spline_data(
                entry.guid,
                dictionary_index,
                &data,
            ));
            operations.push(SimulationStaticOperation::Grind {
                source_toc_index,
                source_dictionary_index: dictionary_index,
                grind_asset_index,
            });
        }
        INSTANCE_DATA_TYPE_ID => append_environment_entries(
            asset,
            arena,
            resources,
            instance_bases,
            collision_models,
            source_toc_index,
            entry.object,
            operations,
        )?,
        _ => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_environment_entries(
    asset: &SimulationCollisionAsset,
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    instance_bases: &BTreeMap<u32, usize>,
    collision_models: &BTreeMap<u32, usize>,
    source_toc_index: u32,
    object: ObjectId,
    operations: &mut Vec<SimulationStaticOperation>,
) -> Result<(), SimulationStaticArenaPlanError> {
    let (dictionary_index, bytes) = resolve_typed_object(
        arena,
        resources,
        object,
        INSTANCE_DATA_TYPE_ID,
        SimulationStaticReference::TocTarget,
    )?;
    let records = InstanceData::record_table(bytes)?;
    let data = InstanceData::parse(bytes)?;
    if data.type_id != INSTANCE_ENVIRONMENT_ASSET_TYPE_ID {
        return Ok(());
    }
    let base = instance_bases.get(&dictionary_index).copied().ok_or(
        SimulationStaticArenaPlanError::MissingProjectedInstance {
            dictionary_index,
            inline_index: 0,
        },
    )?;
    for (inline_index, instance) in data.instances.into_iter().enumerate() {
        if environment_is_hybrid(arena, resources, &instance)? {
            continue;
        }
        let source_inline_index = u32::try_from(inline_index)
            .map_err(|_| SimulationStaticArenaPlanError::InlineIndexOverflow(inline_index))?;
        let record = records
            .record(inline_index)
            .ok_or(PegasusError::IntegerOverflow)?;
        let source_record_start = arena.dictionary[dictionary_index as usize]
            .object_range
            .start
            .checked_add(record.object_offset() as usize)
            .ok_or(SimulationStaticArenaPlanError::ObjectRangeOutsideResource {
                dictionary_index,
                type_id: INSTANCE_DATA_TYPE_ID,
            })?;
        let instance_index = base
            .checked_add(inline_index)
            .ok_or(SimulationStaticArenaPlanError::IndexOverflow)?;
        validate_projected_instance(
            asset,
            collision_models,
            dictionary_index,
            source_inline_index,
            instance_index,
            &instance,
        )?;
        operations.push(SimulationStaticOperation::Environment {
            source_toc_index,
            source_dictionary_index: dictionary_index,
            source_inline_index,
            source_resource_index: arena.dictionary[dictionary_index as usize].resource_index,
            source_record_start,
            instance_index,
        });
    }
    Ok(())
}

fn validate_projected_instance(
    asset: &SimulationCollisionAsset,
    collision_models: &BTreeMap<u32, usize>,
    dictionary_index: u32,
    inline_index: u32,
    instance_index: usize,
    instance: &Instance,
) -> Result<(), SimulationStaticArenaPlanError> {
    let projected = asset.collision.instances.get(instance_index).ok_or(
        SimulationStaticArenaPlanError::MissingProjectedInstance {
            dictionary_index,
            inline_index,
        },
    )?;
    let expected_collision_model = instance
        .collision_model
        .map(|id| {
            let entry_index = require_local_index(id, SimulationStaticReference::CollisionModel)?;
            collision_models.get(&entry_index).copied().ok_or(
                SimulationStaticArenaPlanError::MissingLocalObjectReference {
                    reference: SimulationStaticReference::CollisionModel,
                    object_id: id.0,
                    entry_index,
                },
            )
        })
        .transpose()?;
    if projected.guid != instance.guid
        || projected.local_guid != instance.local_guid
        || projected.collision_model != expected_collision_model
    {
        return Err(SimulationStaticArenaPlanError::ProjectedInstanceMismatch {
            collision_instance_index: instance_index,
        });
    }
    Ok(())
}

fn projected_instance_bases(
    asset: &SimulationCollisionAsset,
) -> Result<BTreeMap<u32, usize>, SimulationStaticArenaPlanError> {
    let arena = &asset.decoded.arena;
    let resources = &asset.decoded.resources.resources;
    let mut bases = BTreeMap::new();
    let mut indexed = 0_usize;
    for (dictionary_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != INSTANCE_DATA_TYPE_ID {
            continue;
        }
        let bytes = object_bytes(arena, resources, dictionary_index, entry)?;
        let data = InstanceData::parse(bytes)?;
        let dictionary_index = u32::try_from(dictionary_index)
            .map_err(|_| SimulationStaticArenaPlanError::IndexOverflow)?;
        bases.insert(dictionary_index, indexed);
        indexed = indexed
            .checked_add(data.instances.len())
            .ok_or(SimulationStaticArenaPlanError::IndexOverflow)?;
    }
    let projected = asset.collision.instances.len();
    if indexed != projected {
        return Err(
            SimulationStaticArenaPlanError::CollisionInstanceCountMismatch { projected, indexed },
        );
    }
    Ok(bases)
}

pub(crate) fn environment_is_hybrid(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    instance: &Instance,
) -> Result<bool, SimulationStaticArenaPlanError> {
    let collision_has_meshes = if let Some(id) = instance.collision_model {
        let (_, bytes) = resolve_typed_object(
            arena,
            resources,
            id,
            C_MODEL_DATA_TYPE_ID,
            SimulationStaticReference::CollisionModel,
        )?;
        CollisionModelData::raw_mesh_count(bytes)? != 0
    } else {
        false
    };
    let render_has_meshes = if let Some(id) = instance.render_model {
        let (_, bytes) = resolve_typed_object(
            arena,
            resources,
            id,
            R_MODEL_DATA_TYPE_ID,
            SimulationStaticReference::RenderModel,
        )?;
        RenderModelData::raw_mesh_count(bytes)? != 0
    } else {
        false
    };
    Ok(collision_has_meshes && render_has_meshes)
}

fn require_local_index(
    id: ObjectId,
    reference: SimulationStaticReference,
) -> Result<u32, SimulationStaticArenaPlanError> {
    let section = id.section_index();
    if section != 0 {
        return Err(SimulationStaticArenaPlanError::NonLocalObjectReference {
            reference,
            object_id: id.0,
            section,
        });
    }
    Ok(id.entry_index())
}

fn resolve_typed_object<'a>(
    arena: &Arena,
    resources: &'a [Vec<u8>; 5],
    id: ObjectId,
    expected: u32,
    reference: SimulationStaticReference,
) -> Result<(u32, &'a [u8]), SimulationStaticArenaPlanError> {
    let entry_index = require_local_index(id, reference)?;
    let entry = arena.dictionary.get(entry_index as usize).ok_or(
        SimulationStaticArenaPlanError::MissingLocalObjectReference {
            reference,
            object_id: id.0,
            entry_index,
        },
    )?;
    if entry.type_id != expected {
        return Err(SimulationStaticArenaPlanError::WrongLocalObjectType {
            reference,
            object_id: id.0,
            expected,
            actual: entry.type_id,
        });
    }
    let bytes = object_bytes(arena, resources, entry_index as usize, entry)?;
    Ok((entry_index, bytes))
}

fn object_bytes<'a>(
    arena: &Arena,
    resources: &'a [Vec<u8>; 5],
    dictionary_index: usize,
    entry: &DictionaryEntry,
) -> Result<&'a [u8], SimulationStaticArenaPlanError> {
    arena.object_bytes(resources, entry).ok_or_else(|| {
        SimulationStaticArenaPlanError::ObjectRangeOutsideResource {
            dictionary_index: u32::try_from(dictionary_index).unwrap_or(u32::MAX),
            type_id: entry.type_id,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{CollisionArena, CollisionInstance, CollisionModel};
    use skate2_formats::{
        rw4::{FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::ops::Range;

    struct ArenaFixture {
        bytes: Vec<u8>,
        dictionary: Vec<DictionaryEntry>,
    }

    impl ArenaFixture {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                dictionary: Vec::new(),
            }
        }

        fn push(&mut self, type_id: u32, object: &[u8]) -> ObjectId {
            let index = self.dictionary.len();
            let start = self.bytes.len();
            self.bytes.extend_from_slice(object);
            let end = self.bytes.len();
            self.dictionary.push(DictionaryEntry {
                resource_index: 0,
                object_offset: u32::try_from(start).unwrap(),
                relocation_offset: 0,
                size: u32::try_from(object.len()).unwrap(),
                alignment: 1,
                type_index: 0,
                type_id,
                object_range: Range { start, end },
            });
            ObjectId(u32::try_from(index).unwrap())
        }

        fn finish(self, instances: Vec<CollisionInstance>) -> SimulationCollisionAsset {
            let descriptor = ResourceDescriptor {
                size: 0,
                alignment: 1,
            };
            let arena = Arena {
                file_header: FileHeader {
                    magic: *b"\x89RW4xb2\0\r\n\x1a\n",
                    is_big_endian: true,
                    pointer_size_bits: 32,
                    pointer_alignment: 4,
                    unused: 0,
                    major_version: *b"454\0",
                    minor_version: *b"000\0",
                    build_number: 0,
                },
                id: 0x1234_5678,
                entry_count: u32::try_from(self.dictionary.len()).unwrap(),
                used_entry_count: u32::try_from(self.dictionary.len()).unwrap(),
                alignment: 1,
                virtual_size: u32::try_from(self.bytes.len()).unwrap(),
                dictionary_offset: 0,
                sections_offset: 0,
                base_offset: 0,
                unfix_context_offset: 0,
                fix_context_offset: 0,
                resource_descriptor: [descriptor; 5],
                resources_used: [descriptor; 5],
                resource_offsets: [0; 5],
                arena_group_offset: 0,
                dictionary: self.dictionary,
            };
            let collision_models = arena
                .dictionary
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.type_id == C_MODEL_DATA_TYPE_ID)
                .map(|(dictionary_index, _)| CollisionModel {
                    dictionary_index: u32::try_from(dictionary_index).unwrap(),
                    bounding_root: None,
                    collision_roots: Vec::new(),
                    assembly_object_id: None,
                })
                .collect();
            SimulationCollisionAsset {
                decoded: crate::decoded_arena_asset::DecodedArenaAsset::from_parts_for_test(
                    0xabcd_ef01_2345_6789,
                    crate::decoded_arena_asset::WORLD_STREAM_ARENA_ASSET_TYPE,
                    ArenaResources {
                        resources: [self.bytes, Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                    },
                    arena,
                ),
                collision: CollisionArena {
                    id: 0x1234_5678,
                    grind_assets: Vec::new(),
                    aggregate_roots: Vec::new(),
                    primitive_sets: Vec::new(),
                    simple_primitive_sets: Vec::new(),
                    meshes: Vec::new(),
                    collision_models,
                    instances,
                },
            }
        }
    }

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn spline_data() -> Vec<u8> {
        let mut bytes = vec![0; 160];
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 8, 16);
        put_u32(&mut bytes, 12, 16);
        bytes
    }

    fn instance_data(
        type_id: u32,
        guid: u64,
        render: Option<ObjectId>,
        collision: Option<ObjectId>,
    ) -> Vec<u8> {
        let mut bytes = vec![0; 181];
        put_u32(&mut bytes, 0, type_id);
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 12, 20);
        put_u32(&mut bytes, 16, 180);
        put_u64(&mut bytes, 20 + 96, guid);
        put_u64(&mut bytes, 20 + 104, guid.wrapping_add(1));
        put_u32(&mut bytes, 20 + 128, render.map_or(0, |id| id.0));
        put_u32(&mut bytes, 20 + 132, collision.map_or(0, |id| id.0));
        for offset in [140, 144, 148, 152] {
            put_u32(&mut bytes, 20 + offset, 180);
        }
        bytes
    }

    fn toc(entries: &[(u64, u32, ObjectId)]) -> Vec<u8> {
        let names = 20 + entries.len() * 24;
        let mut bytes = vec![0; names + 1];
        put_u32(&mut bytes, 0, u32::try_from(entries.len()).unwrap());
        put_u32(&mut bytes, 4, 20);
        put_u32(&mut bytes, 8, u32::try_from(names).unwrap());
        put_u32(&mut bytes, 16, u32::try_from(names).unwrap());
        for (index, (guid, object_type, object)) in entries.iter().enumerate() {
            let offset = 20 + index * 24;
            put_u32(&mut bytes, offset, u32::try_from(names).unwrap());
            put_u64(&mut bytes, offset + 8, *guid);
            put_u32(&mut bytes, offset + 16, *object_type);
            put_u32(&mut bytes, offset + 20, object.0);
        }
        bytes
    }

    fn projected_instance(guid: u64, collision_model: Option<usize>) -> CollisionInstance {
        CollisionInstance {
            guid,
            local_guid: guid.wrapping_add(1),
            name: String::new(),
            transform: [0.0; 16],
            bounding_box_min: [0.0; 3],
            bounding_box_max: [0.0; 3],
            collision_model,
        }
    }

    fn collision_model(mesh_count: u32) -> Vec<u8> {
        let mut bytes = vec![0; if mesh_count == 1 { 20 } else { 12 }];
        put_u32(&mut bytes, 4, mesh_count);
        put_u32(&mut bytes, 8, if mesh_count == 1 { 12 } else { 7 });
        bytes
    }

    fn render_model(mesh_count: u16) -> Vec<u8> {
        let mut bytes = vec![0; if mesh_count == 1 { 72 } else { 64 }];
        if mesh_count == 1 {
            put_u32(&mut bytes, 36, 64);
        }
        put_u16(&mut bytes, 50, mesh_count);
        bytes
    }

    #[test]
    fn authored_toc_order_and_native_grind_identity_are_separate_from_dictionary_order() {
        let mut fixture = ArenaFixture::new();
        let spline = fixture.push(SPLINE_DATA_TYPE_ID, &spline_data());
        let instance_guid = 0x8877_6655_4433_2211;
        let instances = fixture.push(
            INSTANCE_DATA_TYPE_ID,
            &instance_data(
                INSTANCE_ENVIRONMENT_ASSET_TYPE_ID,
                instance_guid,
                None,
                None,
            ),
        );
        let grind_guid = 0xfedc_ba98_7654_3210;
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (1, INSTANCE_DATA_TYPE_ID, instances),
                (grind_guid, SPLINE_DATA_TYPE_ID, spline),
            ]),
        );

        let plan = SimulationStaticArenaPlan::from_collision_asset(
            fixture.finish(vec![projected_instance(instance_guid, None)]),
        )
        .unwrap();
        assert!(matches!(
            &plan.operations()[0],
            SimulationStaticOperation::Environment {
                source_toc_index: 0,
                source_dictionary_index: 1,
                source_inline_index: 0,
                instance_index: 0,
                ..
            }
        ));
        let SimulationStaticOperation::Environment {
            source_resource_index,
            source_record_start,
            ..
        } = plan.operations()[0]
        else {
            unreachable!();
        };
        let instance_entry = &plan.asset().decoded.arena.dictionary[1];
        assert_eq!(source_resource_index, instance_entry.resource_index);
        assert_eq!(source_record_start, instance_entry.object_range.start + 20);
        assert!(matches!(
            &plan.operations()[1],
            SimulationStaticOperation::Grind {
                source_toc_index: 1,
                source_dictionary_index: 0,
                grind_asset_index: 0,
            }
        ));
        assert_eq!(plan.asset().collision.instances[0].guid, instance_guid);
        assert_eq!(plan.grind_assets()[0].asset_id, grind_guid);
        assert_eq!(
            plan.grind_assets()[0].primitives[0].source_asset_id,
            grind_guid
        );
        assert_eq!(plan.grind_assets()[0].source_dictionary_index, 0);
    }

    #[test]
    fn repeated_direct_spline_references_remain_distinct_ordered_actions() {
        let mut fixture = ArenaFixture::new();
        let spline = fixture.push(SPLINE_DATA_TYPE_ID, &spline_data());
        let first = 0x1111_2222_3333_4444;
        let second = 0xaaaa_bbbb_3333_4444;
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (first, SPLINE_DATA_TYPE_ID, spline),
                (second, SPLINE_DATA_TYPE_ID, spline),
            ]),
        );
        let plan =
            SimulationStaticArenaPlan::from_collision_asset(fixture.finish(Vec::new())).unwrap();

        assert_eq!(plan.operations().len(), 2);
        assert_eq!(plan.grind_assets().len(), 2);
        assert_eq!(plan.grind_assets()[0].asset_id, first);
        assert_eq!(plan.grind_assets()[1].asset_id, second);
        assert_eq!(plan.grind_assets()[1].primitives[0].source_asset_id, second);
    }

    #[test]
    fn spatial_instance_container_is_retained_but_emits_no_static_operation() {
        let mut fixture = ArenaFixture::new();
        let guid = 7;
        let instances = fixture.push(
            INSTANCE_DATA_TYPE_ID,
            &instance_data(
                skate2_formats::pegasus::INSTANCE_SPATIAL_ASSET_TYPE_ID,
                guid,
                None,
                None,
            ),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, INSTANCE_DATA_TYPE_ID, instances)]),
        );
        let plan = SimulationStaticArenaPlan::from_collision_asset(
            fixture.finish(vec![projected_instance(guid, None)]),
        )
        .unwrap();
        assert!(plan.operations().is_empty());
    }

    #[test]
    fn hybrid_filter_uses_collision_first_and_skips_only_when_both_are_nonempty() {
        for (collision_count, render_count, expected_operations) in [
            (0_u32, 0_u16, 1_usize),
            (1, 0, 1),
            (0, 1, 1),
            (1, 1, 0),
            (u32::MAX, 1, 0),
        ] {
            let mut fixture = ArenaFixture::new();
            fixture.push(0x00eb_0008, &[]);
            let collision = fixture.push(C_MODEL_DATA_TYPE_ID, &collision_model(collision_count));
            let render = fixture.push(R_MODEL_DATA_TYPE_ID, &render_model(render_count));
            let guid = 99;
            let instances = fixture.push(
                INSTANCE_DATA_TYPE_ID,
                &instance_data(
                    INSTANCE_ENVIRONMENT_ASSET_TYPE_ID,
                    guid,
                    Some(render),
                    Some(collision),
                ),
            );
            fixture.push(
                TABLE_OF_CONTENTS_TYPE_ID,
                &toc(&[(1, INSTANCE_DATA_TYPE_ID, instances)]),
            );
            let plan = SimulationStaticArenaPlan::from_collision_asset(
                fixture.finish(vec![projected_instance(guid, Some(0))]),
            )
            .unwrap();
            assert_eq!(plan.operations().len(), expected_operations);
        }
    }

    #[test]
    fn render_count_is_read_even_when_collision_count_is_zero() {
        let mut fixture = ArenaFixture::new();
        fixture.push(0x00eb_0008, &[]);
        let collision = fixture.push(C_MODEL_DATA_TYPE_ID, &collision_model(0));
        let guid = 101;
        let instances = fixture.push(
            INSTANCE_DATA_TYPE_ID,
            &instance_data(
                INSTANCE_ENVIRONMENT_ASSET_TYPE_ID,
                guid,
                Some(ObjectId((3 << 22) | 0x004d)),
                Some(collision),
            ),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, INSTANCE_DATA_TYPE_ID, instances)]),
        );
        let error = SimulationStaticArenaPlan::from_collision_asset(
            fixture.finish(vec![projected_instance(guid, Some(0))]),
        )
        .unwrap_err();
        assert_eq!(
            error,
            SimulationStaticArenaPlanError::NonLocalObjectReference {
                reference: SimulationStaticReference::RenderModel,
                object_id: (3 << 22) | 0x004d,
                section: 3,
            }
        );
    }

    #[test]
    fn missing_toc_is_empty_and_only_the_first_of_multiple_tocs_is_used() {
        let mut missing_asset = ArenaFixture::new().finish(Vec::new());
        let diagnostic = GrindAsset::from_spline_data(
            u64::from(missing_asset.collision.id),
            17,
            &SplineData::parse(&spline_data()).unwrap(),
        );
        missing_asset
            .collision
            .grind_assets
            .push(diagnostic.clone());
        let missing = SimulationStaticArenaPlan::from_collision_asset(missing_asset).unwrap();
        assert!(missing.operations().is_empty());
        assert!(missing.grind_assets().is_empty());
        assert_eq!(missing.asset().collision.grind_assets, vec![diagnostic]);

        let mut fixture = ArenaFixture::new();
        fixture.push(TABLE_OF_CONTENTS_TYPE_ID, &toc(&[]));
        let ignored_spline = fixture.push(SPLINE_DATA_TYPE_ID, &spline_data());
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(0xfeed_face_cafe_beef, SPLINE_DATA_TYPE_ID, ignored_spline)]),
        );
        let multiple =
            SimulationStaticArenaPlan::from_collision_asset(fixture.finish(Vec::new())).unwrap();
        assert!(multiple.operations().is_empty());
        assert!(multiple.grind_assets().is_empty());
    }
}
