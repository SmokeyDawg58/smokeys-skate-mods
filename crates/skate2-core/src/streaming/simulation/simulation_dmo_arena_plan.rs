//! Owner-bound processed assets for Simulation's separate DMO-locator pass.
//!
//! `cProcessArenaAsset::ProcessDMOData` (`0x82d5f4f0`) creates one kind-31
//! asset for each first-TOC `tDMOData` entry. It passes the complete TOC GUID
//! and the whole fixed-up object pointer; it does not expand the contained
//! 128-byte locator records. `Simulation::RegDMOArena` (`0x828a0420`) and
//! `DRegDMOArena` (`0x828a0828`) consume those assets in a second arena pass,
//! separate from the normal `RegArena`/`DRegArena` actions.

use crate::{
    simulation_dmo_assembly_part_session::{
        SimulationDmoModelCurrentWinner, SimulationDmoModelRegistryResolveError,
        SimulationDmoModelRegistryResolver,
    },
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_static_arena_plan::SimulationStaticArenaPlan,
};
use skate2_formats::{
    pegasus::{
        DMO_DATA_TYPE_ID, DmoData, ObjectId, PegasusError, TABLE_OF_CONTENTS_TYPE_ID,
        TableOfContents,
    },
    rw4::{Arena, DictionaryEntry},
};
use std::{error::Error, fmt, sync::Arc};

const DMO_LOCATOR_RECORD_SIZE: usize = 128;
const DMO_LOCATOR_GUID_OFFSET: usize = 0x60;
const DMO_LOCATOR_LOCAL_GUID_OFFSET: usize = 0x68;
const DMO_LOCATOR_MODEL_KEY_OFFSET: usize = 0x70;

/// One processed kind-31 action in authored first-TOC order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulationDmoLocatorOperation {
    pub asset_id: u64,
    pub toc_dictionary_index: u32,
    pub source_toc_index: u32,
    pub source_dictionary_index: u32,
    pub source_resource_index: u32,
    pub source_object_offset: u32,
    pub source_dictionary_size: u32,
    pub source_object_len: u32,
    pub source_span_start: u64,
    pub source_span_end: u64,
    pub locator_count: u32,
    pub header_type_id: u32,
    pub string_count: u32,
    pub dmos_offset: u32,
    pub string_list_offset: u32,
    pub raw_object_id: u32,
}

#[derive(Clone, Copy, Debug)]
struct SimulationDmoDataLocator {
    resource_index: usize,
    object_start: usize,
    object_len: usize,
}

/// Cloneable authority for one complete serialized `tDMOData` object.
///
/// Native fixup clears each locator's serialized word at record `+0x78` and
/// turns its `+0x7c` name offset into a pointer. This immutable handle is
/// provenance for an eventual writable native representation; it deliberately
/// exposes neither a host address nor those serialized words as runtime state.
#[derive(Clone)]
pub struct SimulationDmoDataHandle {
    owner: Arc<SimulationStaticArenaPlan>,
    locator: SimulationDmoDataLocator,
}

impl fmt::Debug for SimulationDmoDataHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationDmoDataHandle")
            .field("resource_index", &self.locator.resource_index)
            .field("object_start", &self.locator.object_start)
            .field("object_len", &self.locator.object_len)
            .finish_non_exhaustive()
    }
}

impl SimulationDmoDataHandle {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.owner.asset().decoded.resources.resources[self.locator.resource_index]
            [self.locator.object_start..self.locator.object_start + self.locator.object_len]
    }

    /// Tests native-pointer-equivalent identity without exposing a host
    /// address. Dictionary aliases of one object retain identity; equal bytes
    /// in another arena allocation do not.
    #[must_use]
    pub fn is_same_backing_object(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.locator.resource_index == other.locator.resource_index
            && self.locator.object_start == other.locator.object_start
    }
}

/// Exact owner-bound occurrence of one processed locator operation.
///
/// The three IDs are frozen from the immutable serialized record. Identity is
/// deliberately the backing object allocation, authored operation index,
/// outer container asset ID, and locator ordinal. The operation association
/// prevents two TOC registrations which alias one `tDMOData` object from
/// collapsing. This staged type exposes none of those values or its source
/// handle outside this module.
#[allow(
    dead_code,
    reason = "consume-only exact occurrence awaits production locator traversal"
)]
pub(crate) struct SimulationDmoLocatorOccurrence {
    source: SimulationDmoDataHandle,
    operation_index: usize,
    container_asset_id: u64,
    locator_ordinal: usize,
    record_offset: usize,
    guid: u64,
    local_guid: u64,
    model_key: u64,
}

impl fmt::Debug for SimulationDmoLocatorOccurrence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationDmoLocatorOccurrence")
            .field("locator_ordinal", &self.locator_ordinal)
            .finish_non_exhaustive()
    }
}

#[allow(
    dead_code,
    reason = "consume-only exact occurrence awaits production locator traversal"
)]
impl SimulationDmoLocatorOccurrence {
    /// Original full locator matrix consumed by DMO Body construction
    /// (0x825ea858). The complete record was checked when this owner was made.
    pub(crate) fn transform(&self) -> [f32; 16] {
        let bytes = self.source.bytes();
        std::array::from_fn(|lane| {
            let offset = self.record_offset + lane * 4;
            f32::from_bits(u32::from_be_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("retained validated locator matrix"),
            ))
        })
    }

    /// Compares safe-host retained-source, operation, and ordinal identity.
    #[must_use]
    pub(crate) fn is_same_occurrence(&self, other: &Self) -> bool {
        self.operation_index == other.operation_index
            && self.container_asset_id == other.container_asset_id
            && self.locator_ordinal == other.locator_ordinal
            && self.source.is_same_backing_object(&other.source)
    }

    /// Consumes the private `+0x70` model key through one manager-registry
    /// resolver. No raw key leaves this occurrence boundary.
    pub(crate) fn try_select_current_model(
        self,
        resolver: SimulationDmoModelRegistryResolver<'_>,
    ) -> Result<(Self, SimulationDmoModelCurrentWinner<'_>), SimulationDmoModelRegistryResolveError>
    {
        let winner = resolver.try_current(self.model_key)?;
        Ok((self, winner))
    }
}

/// One arena's complete currently recovered `RegDMOArena` kind-31 action list.
#[derive(Debug)]
pub struct SimulationDmoArenaPlan {
    normal_plan_owner: Arc<SimulationNormalArenaPlan>,
    operations: Vec<SimulationDmoLocatorOperation>,
    sources: Vec<SimulationDmoDataLocator>,
}

impl SimulationDmoArenaPlan {
    /// Scans the first used-dictionary TOC and retains one whole-object source
    /// for every local `tDMOData` entry.
    ///
    /// # Errors
    ///
    /// Returns a typed error before runtime effects when the TOC, local object
    /// reference, object span, or supported `tDMOData` object is malformed.
    pub fn from_normal_plan(
        normal_plan_owner: Arc<SimulationNormalArenaPlan>,
    ) -> Result<Self, SimulationDmoArenaPlanError> {
        let (operations, sources) = build_operations(&normal_plan_owner)?;
        Ok(Self {
            normal_plan_owner,
            operations,
            sources,
        })
    }

    #[must_use]
    pub fn normal_plan(&self) -> &SimulationNormalArenaPlan {
        &self.normal_plan_owner
    }

    #[must_use]
    pub fn operations(&self) -> &[SimulationDmoLocatorOperation] {
        &self.operations
    }

    /// Clones the exact whole-object authority paired with one operation.
    #[must_use]
    pub fn source_handle(&self, operation_index: usize) -> Option<SimulationDmoDataHandle> {
        let locator = *self.sources.get(operation_index)?;
        Some(SimulationDmoDataHandle {
            owner: self.normal_plan_owner.static_plan_owner(),
            locator,
        })
    }

    /// Freezes one exact locator occurrence without exposing its raw IDs or
    /// complete source authority. Plan construction has already parsed and
    /// validated every complete locator record, so selection reads the exact
    /// immutable retained slice without reparsing or allocating.
    ///
    /// # Errors
    ///
    /// Returns a typed refusal when the operation or locator ordinal is out of
    /// range, or if the retained validated object cannot supply the selected
    /// 128-byte record without host arithmetic overflow.
    #[allow(
        dead_code,
        reason = "consume-only exact occurrence awaits production locator traversal"
    )]
    pub(crate) fn try_locator_occurrence(
        &self,
        operation_index: usize,
        locator_ordinal: usize,
    ) -> Result<SimulationDmoLocatorOccurrence, SimulationDmoLocatorOccurrenceError> {
        let operation = self.operations.get(operation_index).copied().ok_or(
            SimulationDmoLocatorOccurrenceError::OperationOutOfRange {
                operation_index,
                operation_count: self.operations.len(),
            },
        )?;
        let locator_count = usize::try_from(operation.locator_count)
            .expect("u32 locator count fits the host usize domain");
        if locator_ordinal >= locator_count {
            return Err(
                SimulationDmoLocatorOccurrenceError::LocatorOrdinalOutOfRange {
                    operation_index,
                    locator_ordinal,
                    locator_count,
                },
            );
        }
        let dmos_offset = usize::try_from(operation.dmos_offset)
            .expect("u32 DMO array offset fits the host usize domain");
        let record_offset = locator_ordinal
            .checked_mul(DMO_LOCATOR_RECORD_SIZE)
            .and_then(|offset| dmos_offset.checked_add(offset))
            .ok_or(SimulationDmoLocatorOccurrenceError::RecordSpanOverflow {
                operation_index,
                locator_ordinal,
            })?;
        let record_end = record_offset.checked_add(DMO_LOCATOR_RECORD_SIZE).ok_or(
            SimulationDmoLocatorOccurrenceError::RecordSpanOverflow {
                operation_index,
                locator_ordinal,
            },
        )?;
        let source = self
            .source_handle(operation_index)
            .expect("operation and source vectors are constructed positionally");
        let (guid, local_guid, model_key) = {
            let record = source.bytes().get(record_offset..record_end).ok_or(
                SimulationDmoLocatorOccurrenceError::RecordOutsideSource {
                    operation_index,
                    locator_ordinal,
                },
            )?;
            (
                read_locator_u64(record, DMO_LOCATOR_GUID_OFFSET),
                read_locator_u64(record, DMO_LOCATOR_LOCAL_GUID_OFFSET),
                read_locator_u64(record, DMO_LOCATOR_MODEL_KEY_OFFSET),
            )
        };
        Ok(SimulationDmoLocatorOccurrence {
            source,
            operation_index,
            container_asset_id: operation.asset_id,
            locator_ordinal,
            record_offset,
            guid,
            local_guid,
            model_key,
        })
    }
}

/// Safe-host refusal while selecting one exact locator occurrence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoLocatorOccurrenceError {
    OperationOutOfRange {
        operation_index: usize,
        operation_count: usize,
    },
    LocatorOrdinalOutOfRange {
        operation_index: usize,
        locator_ordinal: usize,
        locator_count: usize,
    },
    RecordSpanOverflow {
        operation_index: usize,
        locator_ordinal: usize,
    },
    RecordOutsideSource {
        operation_index: usize,
        locator_ordinal: usize,
    },
}

impl fmt::Display for SimulationDmoLocatorOccurrenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationOutOfRange {
                operation_index,
                operation_count,
            } => write!(
                formatter,
                "Simulation DMO operation {operation_index} is outside {operation_count} operations"
            ),
            Self::LocatorOrdinalOutOfRange {
                operation_index,
                locator_ordinal,
                locator_count,
            } => write!(
                formatter,
                "Simulation DMO operation {operation_index} locator {locator_ordinal} is outside {locator_count} locators"
            ),
            Self::RecordSpanOverflow {
                operation_index,
                locator_ordinal,
            } => write!(
                formatter,
                "Simulation DMO operation {operation_index} locator {locator_ordinal} span overflowed"
            ),
            Self::RecordOutsideSource {
                operation_index,
                locator_ordinal,
            } => write!(
                formatter,
                "Simulation DMO operation {operation_index} locator {locator_ordinal} lies outside its retained object"
            ),
        }
    }
}

impl Error for SimulationDmoLocatorOccurrenceError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationDmoArenaPlanError {
    OperationStorageAllocation {
        requested: usize,
    },
    DictionaryIndexOverflow(usize),
    TocIndexOverflow(usize),
    NonLocalObjectReference {
        toc_index: u32,
        object_id: u32,
        section: u16,
    },
    MissingLocalObjectReference {
        toc_index: u32,
        object_id: u32,
        dictionary_index: u32,
    },
    WrongLocalObjectType {
        toc_index: u32,
        object_id: u32,
        actual: u32,
    },
    ObjectRangeOutsideResource {
        dictionary_index: u32,
    },
    ResourceIndexOverflow(usize),
    ObjectSpanOverflow(usize),
    ObjectLengthOverflow(usize),
    LocatorCountOverflow(usize),
    Pegasus(PegasusError),
}

impl fmt::Display for SimulationDmoArenaPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationStorageAllocation { requested } => write!(
                formatter,
                "could not allocate {requested} Simulation DMO-locator operations"
            ),
            Self::DictionaryIndexOverflow(index) => {
                write!(formatter, "dictionary index {index} does not fit in u32")
            }
            Self::TocIndexOverflow(index) => {
                write!(formatter, "TOC entry index {index} does not fit in u32")
            }
            Self::NonLocalObjectReference {
                toc_index,
                object_id,
                section,
            } => write!(
                formatter,
                "TOC entry {toc_index} uses unsupported nonlocal DMO object ID {object_id:08x} in section {section}"
            ),
            Self::MissingLocalObjectReference {
                toc_index,
                object_id,
                dictionary_index,
            } => write!(
                formatter,
                "TOC entry {toc_index} DMO object ID {object_id:08x} targets missing local entry {dictionary_index}"
            ),
            Self::WrongLocalObjectType {
                toc_index,
                object_id,
                actual,
            } => write!(
                formatter,
                "TOC entry {toc_index} DMO object ID {object_id:08x} has type {actual:08x}, expected {DMO_DATA_TYPE_ID:08x}"
            ),
            Self::ObjectRangeOutsideResource { dictionary_index } => write!(
                formatter,
                "DMO dictionary entry {dictionary_index} is outside its retained resource"
            ),
            Self::ResourceIndexOverflow(index) => {
                write!(formatter, "resource index {index} does not fit in u32")
            }
            Self::ObjectSpanOverflow(offset) => {
                write!(
                    formatter,
                    "DMO object span offset {offset} does not fit in u64"
                )
            }
            Self::ObjectLengthOverflow(length) => {
                write!(formatter, "DMO object length {length} does not fit in u32")
            }
            Self::LocatorCountOverflow(count) => {
                write!(formatter, "DMO locator count {count} does not fit in u32")
            }
            Self::Pegasus(source) => write!(formatter, "processed DMO data is invalid: {source}"),
        }
    }
}

impl Error for SimulationDmoArenaPlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Pegasus(source) => Some(source),
            _ => None,
        }
    }
}

impl From<PegasusError> for SimulationDmoArenaPlanError {
    fn from(source: PegasusError) -> Self {
        Self::Pegasus(source)
    }
}

fn build_operations(
    normal_plan: &SimulationNormalArenaPlan,
) -> Result<
    (
        Vec<SimulationDmoLocatorOperation>,
        Vec<SimulationDmoDataLocator>,
    ),
    SimulationDmoArenaPlanError,
> {
    let static_plan = normal_plan.static_plan();
    let arena = &static_plan.asset().decoded.arena;
    let resources = &static_plan.asset().decoded.resources.resources;
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
    let toc_dictionary_index = u32::try_from(toc_dictionary_index)
        .map_err(|_| SimulationDmoArenaPlanError::DictionaryIndexOverflow(toc_dictionary_index))?;
    let supported = toc
        .entries
        .iter()
        .filter(|entry| entry.object_type == DMO_DATA_TYPE_ID)
        .count();
    let mut operations = Vec::new();
    let mut sources = Vec::new();
    operations.try_reserve(supported).map_err(|_| {
        SimulationDmoArenaPlanError::OperationStorageAllocation {
            requested: supported,
        }
    })?;
    sources.try_reserve(supported).map_err(|_| {
        SimulationDmoArenaPlanError::OperationStorageAllocation {
            requested: supported,
        }
    })?;

    for (toc_index, toc_entry) in toc.entries.iter().enumerate() {
        if toc_entry.object_type != DMO_DATA_TYPE_ID {
            continue;
        }
        let source_toc_index = u32::try_from(toc_index)
            .map_err(|_| SimulationDmoArenaPlanError::TocIndexOverflow(toc_index))?;
        let (source_dictionary_index, entry) =
            validate_source(arena, source_toc_index, toc_entry.object)?;
        let bytes = object_bytes(arena, resources, source_dictionary_index as usize, entry)?;
        let parsed = DmoData::parse(bytes)?;
        let source_resource_index = u32::try_from(entry.resource_index).map_err(|_| {
            SimulationDmoArenaPlanError::ResourceIndexOverflow(entry.resource_index)
        })?;
        let source_object_len = u32::try_from(bytes.len())
            .map_err(|_| SimulationDmoArenaPlanError::ObjectLengthOverflow(bytes.len()))?;
        let source_span_start = u64::try_from(entry.object_range.start).map_err(|_| {
            SimulationDmoArenaPlanError::ObjectSpanOverflow(entry.object_range.start)
        })?;
        let source_span_end = u64::try_from(entry.object_range.end)
            .map_err(|_| SimulationDmoArenaPlanError::ObjectSpanOverflow(entry.object_range.end))?;
        let locator_count = u32::try_from(parsed.dmos.len())
            .map_err(|_| SimulationDmoArenaPlanError::LocatorCountOverflow(parsed.dmos.len()))?;
        operations.push(SimulationDmoLocatorOperation {
            asset_id: toc_entry.guid,
            toc_dictionary_index,
            source_toc_index,
            source_dictionary_index,
            source_resource_index,
            source_object_offset: entry.object_offset,
            source_dictionary_size: entry.size,
            source_object_len,
            source_span_start,
            source_span_end,
            locator_count,
            header_type_id: parsed.type_id,
            string_count: parsed.string_count,
            dmos_offset: parsed.dmos_offset,
            string_list_offset: parsed.string_list_offset,
            raw_object_id: toc_entry.object.0,
        });
        sources.push(SimulationDmoDataLocator {
            resource_index: entry.resource_index,
            object_start: entry.object_range.start,
            object_len: bytes.len(),
        });
    }
    Ok((operations, sources))
}

fn validate_source(
    arena: &Arena,
    toc_index: u32,
    object: ObjectId,
) -> Result<(u32, &DictionaryEntry), SimulationDmoArenaPlanError> {
    let section = object.section_index();
    if section != 0 {
        return Err(SimulationDmoArenaPlanError::NonLocalObjectReference {
            toc_index,
            object_id: object.0,
            section,
        });
    }
    let dictionary_index = object.entry_index();
    let entry = arena.dictionary.get(dictionary_index as usize).ok_or(
        SimulationDmoArenaPlanError::MissingLocalObjectReference {
            toc_index,
            object_id: object.0,
            dictionary_index,
        },
    )?;
    if entry.type_id != DMO_DATA_TYPE_ID {
        return Err(SimulationDmoArenaPlanError::WrongLocalObjectType {
            toc_index,
            object_id: object.0,
            actual: entry.type_id,
        });
    }
    Ok((dictionary_index, entry))
}

fn object_bytes<'a>(
    arena: &Arena,
    resources: &'a [Vec<u8>; 5],
    dictionary_index: usize,
    entry: &DictionaryEntry,
) -> Result<&'a [u8], SimulationDmoArenaPlanError> {
    arena.object_bytes(resources, entry).ok_or_else(|| {
        SimulationDmoArenaPlanError::ObjectRangeOutsideResource {
            dictionary_index: u32::try_from(dictionary_index).unwrap_or(u32::MAX),
        }
    })
}

fn read_locator_u64(record: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(
        record[offset..offset + 8]
            .try_into()
            .expect("validated locator record span"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
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

        fn alias(&mut self, object: ObjectId) -> ObjectId {
            let entry = self.dictionary[object.entry_index() as usize].clone();
            let index = self.dictionary.len();
            self.dictionary.push(entry);
            ObjectId(u32::try_from(index).unwrap())
        }

        fn finish(self) -> Arc<SimulationNormalArenaPlan> {
            let descriptor = ResourceDescriptor {
                size: 0,
                alignment: 1,
            };
            let entry_count = u32::try_from(self.dictionary.len()).unwrap();
            let virtual_size = u32::try_from(self.bytes.len()).unwrap();
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
                entry_count,
                used_entry_count: entry_count,
                alignment: 1,
                virtual_size,
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
            let asset = SimulationCollisionAsset {
                decoded: DecodedArenaAsset::from_parts_for_test(
                    0xabcd_ef01_2345_6789,
                    WORLD_STREAM_ARENA_ASSET_TYPE,
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
                    collision_models: Vec::new(),
                    instances: Vec::new(),
                },
            };
            let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
                asset,
                Vec::new(),
            ));
            Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap())
        }
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
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

    fn dmo_data(records: &[(u64, u64, u64, u32)]) -> Vec<u8> {
        dmo_data_at(records, 20)
    }

    fn dmo_data_at(records: &[(u64, u64, u64, u32)], dmos_offset: usize) -> Vec<u8> {
        let records_end = dmos_offset + records.len() * 128;
        let mut bytes = vec![0; records_end + records.len() * 2 + 1];
        put_u32(&mut bytes, 0, DMO_DATA_TYPE_ID);
        put_u32(&mut bytes, 4, u32::try_from(records.len()).unwrap());
        put_u32(&mut bytes, 12, u32::try_from(dmos_offset).unwrap());
        put_u32(&mut bytes, 16, u32::try_from(records_end).unwrap());
        for (index, &(guid, local_guid, instance_guid, serialized_word)) in
            records.iter().enumerate()
        {
            let start = dmos_offset + index * 128;
            put_u64(&mut bytes, start + 96, guid);
            put_u64(&mut bytes, start + 104, local_guid);
            put_u64(&mut bytes, start + 112, instance_guid);
            put_u32(&mut bytes, start + 120, serialized_word);
            let name = records_end + index * 2;
            put_u32(&mut bytes, start + 124, u32::try_from(name).unwrap());
            bytes[name] = b'x';
        }
        bytes
    }

    #[test]
    fn emits_one_whole_object_operation_per_toc_entry() {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(
            DMO_DATA_TYPE_ID,
            &dmo_data(&[(1, 2, 3, 0xfeed_beef), (4, 5, 6, 0x1234_5678)]),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(0xfedc_ba98_7654_3210, DMO_DATA_TYPE_ID, dmo)]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        assert_eq!(plan.operations().len(), 1);
        let operation = plan.operations()[0];
        assert_eq!(operation.asset_id, 0xfedc_ba98_7654_3210);
        assert_eq!(operation.locator_count, 2);
        assert_eq!(operation.source_toc_index, 0);
        assert_eq!(operation.source_dictionary_index, 0);
        assert_eq!(operation.source_object_len as usize, 20 + 2 * 128 + 5);
        let handle = plan.source_handle(0).unwrap();
        assert_eq!(handle.bytes().len(), operation.source_object_len as usize);
        assert_eq!(
            u32::from_be_bytes(handle.bytes()[268..272].try_into().unwrap()),
            0x1234_5678
        );
    }

    #[test]
    fn first_toc_only_and_no_toc_match_native_processing_scope() {
        let mut fixture = ArenaFixture::new();
        let first = fixture.push(DMO_DATA_TYPE_ID, &dmo_data(&[]));
        let second = fixture.push(DMO_DATA_TYPE_ID, &dmo_data(&[(1, 2, 3, 4)]));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(10, DMO_DATA_TYPE_ID, first)]),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(20, DMO_DATA_TYPE_ID, second)]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        assert_eq!(plan.operations().len(), 1);
        assert_eq!(plan.operations()[0].asset_id, 10);
        assert_eq!(plan.operations()[0].locator_count, 0);

        let empty = SimulationDmoArenaPlan::from_normal_plan(ArenaFixture::new().finish()).unwrap();
        assert!(empty.operations().is_empty());
        assert!(empty.source_handle(0).is_none());
    }

    #[test]
    fn aliases_share_identity_but_equal_separate_objects_do_not() {
        let bytes = dmo_data(&[(1, 2, 3, 4)]);
        let mut fixture = ArenaFixture::new();
        let first = fixture.push(DMO_DATA_TYPE_ID, &bytes);
        let alias = fixture.alias(first);
        let separate = fixture.push(DMO_DATA_TYPE_ID, &bytes);
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (1, DMO_DATA_TYPE_ID, first),
                (2, DMO_DATA_TYPE_ID, alias),
                (3, DMO_DATA_TYPE_ID, separate),
            ]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        let first = plan.source_handle(0).unwrap();
        let alias = plan.source_handle(1).unwrap();
        let separate = plan.source_handle(2).unwrap();
        assert!(first.is_same_backing_object(&alias));
        assert!(!first.is_same_backing_object(&separate));
        assert_eq!(first.bytes(), separate.bytes());
    }

    #[test]
    fn locator_occurrence_selects_authored_ordinal_and_freezes_three_exact_ids() {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(
            DMO_DATA_TYPE_ID,
            &dmo_data(&[
                (
                    0xb001_0203_0405_0607,
                    0xb101_1213_1415_1617,
                    0xb201_2223_2425_2627,
                    0,
                ),
                (
                    0xa001_0203_0405_0607,
                    0xa101_1213_1415_1617,
                    0xa201_2223_2425_2627,
                    0,
                ),
            ]),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, dmo)]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        let occurrence = plan.try_locator_occurrence(0, 1).unwrap();
        assert_eq!(occurrence.operation_index, 0);
        assert_eq!(occurrence.container_asset_id, 1);
        assert_eq!(occurrence.locator_ordinal, 1);
        assert_eq!(occurrence.guid, 0xa001_0203_0405_0607);
        assert_eq!(occurrence.local_guid, 0xa101_1213_1415_1617);
        assert_eq!(occurrence.model_key, 0xa201_2223_2425_2627);
        assert_eq!(
            occurrence.source.bytes(),
            plan.source_handle(0).unwrap().bytes()
        );
    }

    #[test]
    fn locator_occurrence_obeys_noncanonical_valid_dmos_offset() {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(
            DMO_DATA_TYPE_ID,
            &dmo_data_at(
                &[
                    (
                        0x0101_0202_0303_0404,
                        0x1111_1212_1313_1414,
                        0x2121_2222_2323_2424,
                        0,
                    ),
                    (
                        0xa1a1_a2a2_a3a3_a4a4,
                        0xb1b1_b2b2_b3b3_b4b4,
                        0xc1c1_c2c2_c3c3_c4c4,
                        0,
                    ),
                ],
                36,
            ),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(0x55, DMO_DATA_TYPE_ID, dmo)]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        let occurrence = plan.try_locator_occurrence(0, 1).unwrap();
        assert_eq!(plan.operations()[0].dmos_offset, 36);
        assert_eq!(occurrence.guid, 0xa1a1_a2a2_a3a3_a4a4);
        assert_eq!(occurrence.local_guid, 0xb1b1_b2b2_b3b3_b4b4);
        assert_eq!(occurrence.model_key, 0xc1c1_c2c2_c3c3_c4c4);
    }

    #[test]
    fn locator_occurrence_uses_the_selected_operations_positional_source() {
        let mut fixture = ArenaFixture::new();
        let first = fixture.push(DMO_DATA_TYPE_ID, &dmo_data(&[(0x101, 0x102, 0x103, 0)]));
        let second = fixture.push(DMO_DATA_TYPE_ID, &dmo_data(&[(0x201, 0x202, 0x203, 0)]));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (0xaaaa, DMO_DATA_TYPE_ID, first),
                (0xbbbb, DMO_DATA_TYPE_ID, second),
            ]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        let occurrence = plan.try_locator_occurrence(1, 0).unwrap();
        assert_eq!(occurrence.operation_index, 1);
        assert_eq!(occurrence.container_asset_id, 0xbbbb);
        assert_eq!(
            (occurrence.guid, occurrence.local_guid, occurrence.model_key),
            (0x201, 0x202, 0x203)
        );
        assert!(
            occurrence
                .source
                .is_same_backing_object(&plan.source_handle(1).unwrap())
        );
        assert!(
            !occurrence
                .source
                .is_same_backing_object(&plan.source_handle(0).unwrap())
        );
    }

    #[test]
    fn locator_occurrence_rejects_operation_and_locator_bounds_in_order() {
        let empty = SimulationDmoArenaPlan::from_normal_plan(ArenaFixture::new().finish()).unwrap();
        assert_eq!(
            empty.try_locator_occurrence(0, 0).unwrap_err(),
            SimulationDmoLocatorOccurrenceError::OperationOutOfRange {
                operation_index: 0,
                operation_count: 0,
            }
        );

        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(DMO_DATA_TYPE_ID, &dmo_data(&[]));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, dmo)]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        assert_eq!(
            plan.try_locator_occurrence(0, 0).unwrap_err(),
            SimulationDmoLocatorOccurrenceError::LocatorOrdinalOutOfRange {
                operation_index: 0,
                locator_ordinal: 0,
                locator_count: 0,
            }
        );
        assert!(matches!(
            plan.try_locator_occurrence(1, usize::MAX),
            Err(SimulationDmoLocatorOccurrenceError::OperationOutOfRange {
                operation_index: 1,
                operation_count: 1,
            })
        ));

        let mut nonempty = ArenaFixture::new();
        let dmo = nonempty.push(DMO_DATA_TYPE_ID, &dmo_data(&[(1, 2, 3, 0)]));
        nonempty.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, dmo)]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(nonempty.finish()).unwrap();
        assert!(plan.try_locator_occurrence(0, 0).is_ok());
        assert_eq!(
            plan.try_locator_occurrence(0, 1).unwrap_err(),
            SimulationDmoLocatorOccurrenceError::LocatorOrdinalOutOfRange {
                operation_index: 0,
                locator_ordinal: 1,
                locator_count: 1,
            }
        );
    }

    #[test]
    fn locator_identity_includes_operation_association_source_and_ordinal() {
        let bytes = dmo_data(&[(1, 2, 3, 0), (1, 2, 3, 0)]);
        let mut fixture = ArenaFixture::new();
        let first = fixture.push(DMO_DATA_TYPE_ID, &bytes);
        let alias = fixture.alias(first);
        let separate = fixture.push(DMO_DATA_TYPE_ID, &bytes);
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (1, DMO_DATA_TYPE_ID, first),
                (2, DMO_DATA_TYPE_ID, alias),
                (3, DMO_DATA_TYPE_ID, separate),
            ]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        let first_zero = plan.try_locator_occurrence(0, 0).unwrap();
        let first_zero_again = plan.try_locator_occurrence(0, 0).unwrap();
        let alias_zero = plan.try_locator_occurrence(1, 0).unwrap();
        let first_one = plan.try_locator_occurrence(0, 1).unwrap();
        let separate_zero = plan.try_locator_occurrence(2, 0).unwrap();
        assert!(first_zero.is_same_occurrence(&first_zero_again));
        assert!(!first_zero.is_same_occurrence(&alias_zero));
        assert_eq!(first_zero.container_asset_id, 1);
        assert_eq!(alias_zero.container_asset_id, 2);
        assert!(!first_zero.is_same_occurrence(&first_one));
        assert!(!first_zero.is_same_occurrence(&separate_zero));
        assert_eq!(first_zero.source.bytes(), separate_zero.source.bytes());
        assert!(
            first_zero
                .source
                .is_same_backing_object(&first_zero.source.clone())
        );
    }

    #[test]
    fn zero_locator_ids_are_retained_without_sentinel_rejection() {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(DMO_DATA_TYPE_ID, &dmo_data(&[(0, 0, 0, 0)]));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, dmo)]),
        );
        let plan = SimulationDmoArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        let occurrence = plan.try_locator_occurrence(0, 0).unwrap();
        assert_eq!(
            (occurrence.guid, occurrence.local_guid, occurrence.model_key),
            (0, 0, 0)
        );
    }

    #[test]
    fn handle_keeps_the_arena_owner_alive() {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(DMO_DATA_TYPE_ID, &dmo_data(&[(1, 2, 3, 4)]));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, dmo)]),
        );
        let normal = fixture.finish();
        let weak = Arc::downgrade(&normal);
        let plan = SimulationDmoArenaPlan::from_normal_plan(normal).unwrap();
        let handle = plan.source_handle(0).unwrap();
        drop(plan);
        assert!(weak.upgrade().is_none());
        assert_eq!(handle.bytes().len(), 151);
    }

    #[test]
    fn locator_occurrence_keeps_only_exact_static_source_owner_alive() {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(DMO_DATA_TYPE_ID, &dmo_data(&[(1, 2, 3, 4)]));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, dmo)]),
        );
        let normal = fixture.finish();
        let weak_normal = Arc::downgrade(&normal);
        let static_owner = normal.static_plan_owner();
        let weak_static = Arc::downgrade(&static_owner);
        drop(static_owner);
        let plan = SimulationDmoArenaPlan::from_normal_plan(normal).unwrap();
        let occurrence = plan.try_locator_occurrence(0, 0).unwrap();
        drop(plan);
        assert!(weak_normal.upgrade().is_none());
        assert!(weak_static.upgrade().is_some());
        assert_eq!(occurrence.operation_index, 0);
        assert_eq!(occurrence.container_asset_id, 1);
        assert_eq!(occurrence.model_key, 3);
        drop(occurrence);
        assert!(weak_static.upgrade().is_none());
    }

    #[test]
    fn rejects_nonlocal_missing_and_wrong_type_sources() {
        let mut nonlocal = ArenaFixture::new();
        nonlocal.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, ObjectId((2 << 22) | 7))]),
        );
        assert!(matches!(
            SimulationDmoArenaPlan::from_normal_plan(nonlocal.finish()),
            Err(SimulationDmoArenaPlanError::NonLocalObjectReference { section: 2, .. })
        ));

        let mut missing = ArenaFixture::new();
        missing.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, ObjectId(99))]),
        );
        assert!(matches!(
            SimulationDmoArenaPlan::from_normal_plan(missing.finish()),
            Err(SimulationDmoArenaPlanError::MissingLocalObjectReference {
                dictionary_index: 99,
                ..
            })
        ));

        let mut wrong = ArenaFixture::new();
        let object = wrong.push(0x00eb_001c, &[0; 20]);
        wrong.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, object)]),
        );
        assert!(matches!(
            SimulationDmoArenaPlan::from_normal_plan(wrong.finish()),
            Err(SimulationDmoArenaPlanError::WrongLocalObjectType {
                actual: 0x00eb_001c,
                ..
            })
        ));
    }

    #[test]
    fn malformed_dmo_object_is_rejected_before_effects() {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(DMO_DATA_TYPE_ID, &[0; 19]);
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, DMO_DATA_TYPE_ID, dmo)]),
        );
        assert!(matches!(
            SimulationDmoArenaPlan::from_normal_plan(fixture.finish()),
            Err(SimulationDmoArenaPlanError::Pegasus(_))
        ));
    }
}
