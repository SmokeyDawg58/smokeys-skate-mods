//! Owner-bound processed assets for `cLocationManager`'s kind-21 arena pass.
//!
//! `cProcessArenaAsset::ProcessLocationData` (`0x82d5f2e0`) creates one
//! Simulation-registry kind-21 asset per first-TOC `tLocationDescData` entry.
//! `cLocationManager::RegArena` (`0x82a8eff8`) and `DRegArena` (`0x82a8e430`)
//! later consume the full asset ID through that registry.

use crate::{
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_static_arena_plan::SimulationStaticArenaPlan,
};
use skate2_formats::{
    pegasus::{
        LOCATION_DESC_DATA_TYPE_ID, LocationDescData, ObjectId, PegasusError,
        TABLE_OF_CONTENTS_TYPE_ID, TableOfContents,
    },
    rw4::{Arena, DictionaryEntry},
};
use std::{error::Error, fmt, sync::Arc};

const LOCATION_RECORD_SIZE: usize = 128;

/// One processed kind-21 action in authored first-TOC order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocationArenaOperation {
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
    pub location_count: u32,
    pub sub_location_count: u32,
    pub string_count: u32,
    pub locations_offset: u32,
    pub sub_locations_offset: u32,
    pub string_list_offset: u32,
    pub raw_object_id: u32,
}

#[derive(Clone, Copy, Debug)]
struct LocationDataLocator {
    resource_index: usize,
    object_start: usize,
    object_len: usize,
    locations_offset: usize,
    location_count: usize,
}

/// Cloneable authority for one complete serialized `tLocationDescData` object.
///
/// This is immutable pre-fixup provenance; serialized string offsets are not
/// treated as valid host pointers.
#[derive(Clone)]
pub struct LocationDataHandle {
    owner: Arc<SimulationStaticArenaPlan>,
    locator: LocationDataLocator,
}

impl fmt::Debug for LocationDataHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocationDataHandle")
            .field("resource_index", &self.locator.resource_index)
            .field("object_start", &self.locator.object_start)
            .field("object_len", &self.locator.object_len)
            .field("location_count", &self.locator.location_count)
            .finish_non_exhaustive()
    }
}

impl LocationDataHandle {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.owner.asset().decoded.resources.resources[self.locator.resource_index]
            [self.locator.object_start..self.locator.object_start + self.locator.object_len]
    }

    #[must_use]
    pub const fn location_count(&self) -> usize {
        self.locator.location_count
    }

    /// Clones authority for one exact 128-byte `tLocationDesc` record.
    #[must_use]
    pub fn location_handle(&self, location_index: usize) -> Option<LocationRecordHandle> {
        if location_index >= self.locator.location_count {
            return None;
        }
        let record_start = self
            .locator
            .object_start
            .checked_add(self.locator.locations_offset)?
            .checked_add(location_index.checked_mul(LOCATION_RECORD_SIZE)?)?;
        Some(LocationRecordHandle {
            owner: Arc::clone(&self.owner),
            resource_index: self.locator.resource_index,
            object_start: self.locator.object_start,
            object_len: self.locator.object_len,
            record_start,
            location_index,
        })
    }

    #[must_use]
    pub fn is_same_backing_object(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.locator.resource_index == other.locator.resource_index
            && self.locator.object_start == other.locator.object_start
    }
}

/// Cloneable authority for one exact serialized `tLocationDesc` record.
#[derive(Clone)]
pub struct LocationRecordHandle {
    owner: Arc<SimulationStaticArenaPlan>,
    resource_index: usize,
    object_start: usize,
    object_len: usize,
    record_start: usize,
    location_index: usize,
}

impl fmt::Debug for LocationRecordHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocationRecordHandle")
            .field("resource_index", &self.resource_index)
            .field("record_start", &self.record_start)
            .field("location_index", &self.location_index)
            .finish_non_exhaustive()
    }
}

impl LocationRecordHandle {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.owner.asset().decoded.resources.resources[self.resource_index]
            [self.record_start..self.record_start + LOCATION_RECORD_SIZE]
    }

    #[must_use]
    pub const fn location_index(&self) -> usize {
        self.location_index
    }

    /// Returns the exact bytes consumed by the native name hash, excluding the
    /// terminating NUL.
    ///
    /// Unlike [`Self::name`], this does not decode or re-encode UTF-8.
    ///
    /// # Errors
    ///
    /// Returns a Pegasus range or unterminated-string error if the retained
    /// name offset is no longer valid.
    pub fn name_bytes(&self) -> Result<Option<&[u8]>, PegasusError> {
        let record = self.bytes();
        let raw_offset = u32::from_be_bytes(
            record[104..108]
                .try_into()
                .map_err(|_| PegasusError::IntegerOverflow)?,
        );
        if raw_offset == 0 {
            return Ok(None);
        }
        let start = usize::try_from(raw_offset).map_err(|_| PegasusError::IntegerOverflow)?;
        let object = &self.owner.asset().decoded.resources.resources[self.resource_index]
            [self.object_start..self.object_start + self.object_len];
        let field_offset = self
            .record_start
            .checked_sub(self.object_start)
            .and_then(|offset| offset.checked_add(104))
            .ok_or(PegasusError::IntegerOverflow)?;
        let tail = object.get(start..).ok_or(PegasusError::Truncated {
            offset: field_offset,
            required: start,
            actual: object.len(),
        })?;
        let length = tail
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(PegasusError::UnterminatedString { offset: start })?;
        Ok(Some(&tail[..length]))
    }

    /// Returns the record's byte offset within its retained RW4 resource.
    #[must_use]
    pub const fn resource_offset(&self) -> usize {
        self.record_start
    }

    /// Returns the index of the retained RW4 resource.
    #[must_use]
    pub const fn resource_index(&self) -> usize {
        self.resource_index
    }

    /// # Errors
    ///
    /// Returns a Pegasus parse error if retained object bytes no longer form a
    /// valid Location container.
    pub fn name(&self) -> Result<Option<String>, PegasusError> {
        let object = &self.owner.asset().decoded.resources.resources[self.resource_index]
            [self.object_start..self.object_start + self.object_len];
        Ok(LocationDescData::parse(object)?
            .locations
            .get(self.location_index)
            .and_then(|location| location.name.clone()))
    }

    #[must_use]
    pub fn is_same_backing_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.resource_index == other.resource_index
            && self.record_start == other.record_start
    }
}

/// One arena's complete processed kind-21 Location action list.
#[derive(Debug)]
pub struct LocationArenaPlan {
    normal_plan_owner: Arc<SimulationNormalArenaPlan>,
    operations: Vec<LocationArenaOperation>,
    sources: Vec<LocationDataLocator>,
}

impl LocationArenaPlan {
    /// Builds the exact first-TOC Location container sequence.
    ///
    /// # Errors
    ///
    /// Returns a typed allocation, TOC/source identity, object-range, or
    /// Pegasus parse error before any runtime effect exists.
    pub fn from_normal_plan(
        normal_plan_owner: Arc<SimulationNormalArenaPlan>,
    ) -> Result<Self, LocationArenaPlanError> {
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
    pub fn operations(&self) -> &[LocationArenaOperation] {
        &self.operations
    }

    #[must_use]
    pub fn source_handle(&self, operation_index: usize) -> Option<LocationDataHandle> {
        let locator = *self.sources.get(operation_index)?;
        Some(LocationDataHandle {
            owner: self.normal_plan_owner.static_plan_owner(),
            locator,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocationArenaPlanError {
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
    LocationCountOverflow(usize),
    SubLocationCountOverflow(usize),
    OffsetOverflow(u32),
    Pegasus(PegasusError),
}

impl fmt::Display for LocationArenaPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationStorageAllocation { requested } => {
                write!(
                    formatter,
                    "could not allocate {requested} Location operations"
                )
            }
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
                "TOC entry {toc_index} uses nonlocal Location object {object_id:08x} in section {section}"
            ),
            Self::MissingLocalObjectReference {
                toc_index,
                object_id,
                dictionary_index,
            } => write!(
                formatter,
                "TOC entry {toc_index} Location object {object_id:08x} targets missing entry {dictionary_index}"
            ),
            Self::WrongLocalObjectType {
                toc_index,
                object_id,
                actual,
            } => write!(
                formatter,
                "TOC entry {toc_index} Location object {object_id:08x} has type {actual:08x}, expected {LOCATION_DESC_DATA_TYPE_ID:08x}"
            ),
            Self::ObjectRangeOutsideResource { dictionary_index } => write!(
                formatter,
                "Location dictionary entry {dictionary_index} is outside its resource"
            ),
            Self::ResourceIndexOverflow(index) => {
                write!(formatter, "resource index {index} does not fit in u32")
            }
            Self::ObjectSpanOverflow(offset) => {
                write!(
                    formatter,
                    "Location object span {offset} does not fit in u64"
                )
            }
            Self::ObjectLengthOverflow(length) => {
                write!(
                    formatter,
                    "Location object length {length} does not fit in u32"
                )
            }
            Self::LocationCountOverflow(count) => {
                write!(formatter, "Location count {count} does not fit in u32")
            }
            Self::SubLocationCountOverflow(count) => {
                write!(formatter, "sublocation count {count} does not fit in u32")
            }
            Self::OffsetOverflow(offset) => {
                write!(formatter, "Location offset {offset} does not fit usize")
            }
            Self::Pegasus(source) => {
                write!(formatter, "processed Location data is invalid: {source}")
            }
        }
    }
}

impl Error for LocationArenaPlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Pegasus(source) => Some(source),
            _ => None,
        }
    }
}

impl From<PegasusError> for LocationArenaPlanError {
    fn from(source: PegasusError) -> Self {
        Self::Pegasus(source)
    }
}

fn build_operations(
    normal_plan: &SimulationNormalArenaPlan,
) -> Result<(Vec<LocationArenaOperation>, Vec<LocationDataLocator>), LocationArenaPlanError> {
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
        .map_err(|_| LocationArenaPlanError::DictionaryIndexOverflow(toc_dictionary_index))?;
    let supported = toc
        .entries
        .iter()
        .filter(|entry| entry.object_type == LOCATION_DESC_DATA_TYPE_ID)
        .count();
    let mut operations = Vec::new();
    let mut sources = Vec::new();
    operations.try_reserve_exact(supported).map_err(|_| {
        LocationArenaPlanError::OperationStorageAllocation {
            requested: supported,
        }
    })?;
    sources.try_reserve_exact(supported).map_err(|_| {
        LocationArenaPlanError::OperationStorageAllocation {
            requested: supported,
        }
    })?;

    for (toc_index, toc_entry) in toc.entries.iter().enumerate() {
        if toc_entry.object_type != LOCATION_DESC_DATA_TYPE_ID {
            continue;
        }
        let source_toc_index = u32::try_from(toc_index)
            .map_err(|_| LocationArenaPlanError::TocIndexOverflow(toc_index))?;
        let (source_dictionary_index, entry) =
            validate_source(arena, source_toc_index, toc_entry.object)?;
        let bytes = object_bytes(arena, resources, source_dictionary_index as usize, entry)?;
        let parsed = LocationDescData::parse(bytes)?;
        let source_resource_index = u32::try_from(entry.resource_index)
            .map_err(|_| LocationArenaPlanError::ResourceIndexOverflow(entry.resource_index))?;
        let source_object_len = u32::try_from(bytes.len())
            .map_err(|_| LocationArenaPlanError::ObjectLengthOverflow(bytes.len()))?;
        let source_span_start = u64::try_from(entry.object_range.start)
            .map_err(|_| LocationArenaPlanError::ObjectSpanOverflow(entry.object_range.start))?;
        let source_span_end = u64::try_from(entry.object_range.end)
            .map_err(|_| LocationArenaPlanError::ObjectSpanOverflow(entry.object_range.end))?;
        let location_count = u32::try_from(parsed.locations.len())
            .map_err(|_| LocationArenaPlanError::LocationCountOverflow(parsed.locations.len()))?;
        let sub_location_count = u32::try_from(parsed.sub_locations.len()).map_err(|_| {
            LocationArenaPlanError::SubLocationCountOverflow(parsed.sub_locations.len())
        })?;
        let locations_offset = usize::try_from(parsed.locations_offset)
            .map_err(|_| LocationArenaPlanError::OffsetOverflow(parsed.locations_offset))?;
        operations.push(LocationArenaOperation {
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
            location_count,
            sub_location_count,
            string_count: parsed.string_count,
            locations_offset: parsed.locations_offset,
            sub_locations_offset: parsed.sub_locations_offset,
            string_list_offset: parsed.string_list_offset,
            raw_object_id: toc_entry.object.0,
        });
        sources.push(LocationDataLocator {
            resource_index: entry.resource_index,
            object_start: entry.object_range.start,
            object_len: bytes.len(),
            locations_offset,
            location_count: parsed.locations.len(),
        });
    }
    Ok((operations, sources))
}

fn validate_source(
    arena: &Arena,
    toc_index: u32,
    object: ObjectId,
) -> Result<(u32, &DictionaryEntry), LocationArenaPlanError> {
    let section = object.section_index();
    if section != 0 {
        return Err(LocationArenaPlanError::NonLocalObjectReference {
            toc_index,
            object_id: object.0,
            section,
        });
    }
    let dictionary_index = object.entry_index();
    let entry = arena.dictionary.get(dictionary_index as usize).ok_or(
        LocationArenaPlanError::MissingLocalObjectReference {
            toc_index,
            object_id: object.0,
            dictionary_index,
        },
    )?;
    if entry.type_id != LOCATION_DESC_DATA_TYPE_ID {
        return Err(LocationArenaPlanError::WrongLocalObjectType {
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
) -> Result<&'a [u8], LocationArenaPlanError> {
    arena.object_bytes(resources, entry).ok_or_else(|| {
        LocationArenaPlanError::ObjectRangeOutsideResource {
            dictionary_index: u32::try_from(dictionary_index).unwrap_or(u32::MAX),
        }
    })
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

    fn location_data(names: &[Option<&str>]) -> Vec<u8> {
        let records_start = 24;
        let records_end = records_start + names.len() * LOCATION_RECORD_SIZE;
        let string_bytes = names
            .iter()
            .flatten()
            .map(|name| name.len() + 1)
            .sum::<usize>();
        let mut bytes = vec![0; records_end + string_bytes];
        put_u32(&mut bytes, 0, u32::try_from(names.len()).unwrap());
        put_u32(
            &mut bytes,
            8,
            u32::try_from(names.iter().flatten().count()).unwrap(),
        );
        put_u32(&mut bytes, 12, u32::try_from(records_start).unwrap());
        put_u32(&mut bytes, 16, u32::try_from(records_end).unwrap());
        put_u32(&mut bytes, 20, u32::try_from(records_end).unwrap());
        let mut string_offset = records_end;
        for (index, name) in names.iter().enumerate() {
            let record = records_start + index * LOCATION_RECORD_SIZE;
            put_u64(&mut bytes, record + 112, 0x1000 + index as u64);
            if let Some(name) = name {
                put_u32(
                    &mut bytes,
                    record + 104,
                    u32::try_from(string_offset).unwrap(),
                );
                bytes[string_offset..string_offset + name.len()].copy_from_slice(name.as_bytes());
                string_offset += name.len() + 1;
            }
        }
        bytes
    }

    #[test]
    fn emits_whole_object_operation_and_exact_record_handles() {
        let mut fixture = ArenaFixture::new();
        let location = fixture.push(
            LOCATION_DESC_DATA_TYPE_ID,
            &location_data(&[Some("alpha"), None]),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(0xfedc_ba98_7654_3210, LOCATION_DESC_DATA_TYPE_ID, location)]),
        );
        let plan = LocationArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        let operation = plan.operations()[0];
        assert_eq!(operation.asset_id, 0xfedc_ba98_7654_3210);
        assert_eq!(
            (operation.location_count, operation.sub_location_count),
            (2, 0)
        );
        let source = plan.source_handle(0).unwrap();
        let first = source.location_handle(0).unwrap();
        let second = source.location_handle(1).unwrap();
        assert_eq!(first.name_bytes().unwrap(), Some(b"alpha".as_slice()));
        assert_eq!(first.name().unwrap().as_deref(), Some("alpha"));
        assert_eq!(second.name_bytes().unwrap(), None);
        assert_eq!(second.name().unwrap(), None);
        assert!(!first.is_same_backing_record(&second));
        assert!(source.location_handle(2).is_none());
    }

    #[test]
    fn first_toc_only_and_absent_toc_match_processor_scope() {
        let mut fixture = ArenaFixture::new();
        let first = fixture.push(LOCATION_DESC_DATA_TYPE_ID, &location_data(&[]));
        let second = fixture.push(LOCATION_DESC_DATA_TYPE_ID, &location_data(&[Some("later")]));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, LOCATION_DESC_DATA_TYPE_ID, first)]),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(2, LOCATION_DESC_DATA_TYPE_ID, second)]),
        );
        let plan = LocationArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        assert_eq!(plan.operations().len(), 1);
        assert_eq!(
            (
                plan.operations()[0].asset_id,
                plan.operations()[0].location_count
            ),
            (1, 0)
        );
        assert!(
            LocationArenaPlan::from_normal_plan(ArenaFixture::new().finish())
                .unwrap()
                .operations()
                .is_empty()
        );
    }

    #[test]
    fn aliases_share_identity_but_separate_equal_objects_do_not() {
        let bytes = location_data(&[Some("same")]);
        let mut fixture = ArenaFixture::new();
        let first = fixture.push(LOCATION_DESC_DATA_TYPE_ID, &bytes);
        let alias = fixture.alias(first);
        let separate = fixture.push(LOCATION_DESC_DATA_TYPE_ID, &bytes);
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (1, LOCATION_DESC_DATA_TYPE_ID, first),
                (2, LOCATION_DESC_DATA_TYPE_ID, alias),
                (3, LOCATION_DESC_DATA_TYPE_ID, separate),
            ]),
        );
        let plan = LocationArenaPlan::from_normal_plan(fixture.finish()).unwrap();
        let first = plan.source_handle(0).unwrap();
        let alias = plan.source_handle(1).unwrap();
        let separate = plan.source_handle(2).unwrap();
        assert!(first.is_same_backing_object(&alias));
        assert!(!first.is_same_backing_object(&separate));
        assert!(
            first
                .location_handle(0)
                .unwrap()
                .is_same_backing_record(&alias.location_handle(0).unwrap())
        );
        assert!(
            !first
                .location_handle(0)
                .unwrap()
                .is_same_backing_record(&separate.location_handle(0).unwrap())
        );
        assert_eq!(first.bytes(), separate.bytes());
    }

    #[test]
    fn rejects_invalid_sources_before_runtime_effects() {
        let mut nonlocal = ArenaFixture::new();
        nonlocal.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, LOCATION_DESC_DATA_TYPE_ID, ObjectId((2 << 22) | 7))]),
        );
        assert!(matches!(
            LocationArenaPlan::from_normal_plan(nonlocal.finish()),
            Err(LocationArenaPlanError::NonLocalObjectReference { section: 2, .. })
        ));

        let mut missing = ArenaFixture::new();
        missing.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, LOCATION_DESC_DATA_TYPE_ID, ObjectId(99))]),
        );
        assert!(matches!(
            LocationArenaPlan::from_normal_plan(missing.finish()),
            Err(LocationArenaPlanError::MissingLocalObjectReference {
                dictionary_index: 99,
                ..
            })
        ));

        let mut wrong = ArenaFixture::new();
        let object = wrong.push(0x00eb_001c, &[0; 24]);
        wrong.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, LOCATION_DESC_DATA_TYPE_ID, object)]),
        );
        assert!(matches!(
            LocationArenaPlan::from_normal_plan(wrong.finish()),
            Err(LocationArenaPlanError::WrongLocalObjectType {
                actual: 0x00eb_001c,
                ..
            })
        ));

        let mut malformed = ArenaFixture::new();
        let object = malformed.push(LOCATION_DESC_DATA_TYPE_ID, &[0; 23]);
        malformed.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, LOCATION_DESC_DATA_TYPE_ID, object)]),
        );
        assert!(matches!(
            LocationArenaPlan::from_normal_plan(malformed.finish()),
            Err(LocationArenaPlanError::Pegasus(_))
        ));
    }
}
