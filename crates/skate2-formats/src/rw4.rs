//! `RenderWare` 4 arena metadata used by streamed Skate 2 assets.
//!
//! This module deliberately stops at the arena dictionary. Dictionary type IDs
//! are retained verbatim until their constructors and serializers can be tied
//! to named code in the reference executable.

use std::{fmt, ops::Range};

use crate::pegasus::ObjectId;

const FILE_HEADER_SIZE: usize = 28;
const ARENA_SIZE: usize = 172;
const DICTIONARY_ENTRY_SIZE: usize = 24;
const RESOURCE_CLASS_COUNT: usize = 5;
const RW4_MAGIC: [u8; 12] = *b"\x89RW4xb2\0\r\n\x1a\n";
const SECTION_MANIFEST_TYPE_ID: u32 = 0x0001_0004;
const SECTION_EXTERNAL_ARENAS_TYPE_ID: u32 = 0x0001_0006;
const SECTION_SUBREFERENCES_TYPE_ID: u32 = 0x0001_0007;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceDescriptor {
    pub size: u32,
    pub alignment: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileHeader {
    pub magic: [u8; 12],
    pub is_big_endian: bool,
    pub pointer_size_bits: u8,
    pub pointer_alignment: u8,
    pub unused: u8,
    pub major_version: [u8; 4],
    pub minor_version: [u8; 4],
    pub build_number: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DictionaryEntry {
    pub resource_index: usize,
    pub object_offset: u32,
    pub relocation_offset: u32,
    pub size: u32,
    pub alignment: u32,
    pub type_index: u32,
    pub type_id: u32,
    pub object_range: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arena {
    pub file_header: FileHeader,
    pub id: u32,
    pub entry_count: u32,
    pub used_entry_count: u32,
    pub alignment: u32,
    pub virtual_size: u32,
    pub dictionary_offset: u32,
    pub sections_offset: u32,
    pub base_offset: u32,
    pub unfix_context_offset: u32,
    pub fix_context_offset: u32,
    pub resource_descriptor: [ResourceDescriptor; RESOURCE_CLASS_COUNT],
    pub resources_used: [ResourceDescriptor; RESOURCE_CLASS_COUNT],
    pub resource_offsets: [u32; RESOURCE_CLASS_COUNT],
    pub arena_group_offset: u32,
    pub dictionary: Vec<DictionaryEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArenaSections {
    pub manifest_offset: u32,
    pub sections: Vec<ArenaSection>,
    /// Arena IDs for object-ID sections 3 and above. Sections 0, 1, and 2
    /// are the local arena, null arena, and subreference dictionary.
    pub external_arena_ids: Vec<u32>,
    pub subreferences: Vec<ArenaSubreference>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArenaSection {
    pub type_id: u32,
    pub entry_count: u32,
    pub object_offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArenaSubreference {
    pub object: ObjectId,
    pub object_offset: u32,
    pub size: u32,
    pub alignment: u32,
    pub type_index: u32,
    pub type_id: u32,
}

impl ArenaSections {
    /// Maps an external object-ID section to the arena ID serialized in the
    /// `ArenaSectionExternalArenas` record.
    #[must_use]
    pub fn external_arena_id(&self, section_index: u16) -> Option<u32> {
        let index = usize::from(section_index).checked_sub(3)?;
        self.external_arena_ids.get(index).copied()
    }
}

impl Arena {
    /// Splits a standalone `.rx2` file into the five resource buffers named by
    /// its embedded arena header.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError`] when the header is invalid or a declared
    /// resource range lies outside the file.
    pub fn split_file_resources(
        bytes: &[u8],
    ) -> Result<[Vec<u8>; RESOURCE_CLASS_COUNT], ArenaError> {
        parse_file_header(bytes)?;
        let mut resources: [Vec<u8>; RESOURCE_CLASS_COUNT] = std::array::from_fn(|_| Vec::new());
        let mut cursor = 0_usize;
        for (index, resource) in resources.iter_mut().enumerate() {
            let resource_descriptor = descriptor(bytes, 68 + index * 8)?;
            let size = usize::try_from(resource_descriptor.size)
                .map_err(|_| ArenaError::IntegerOverflow)?;
            if size == 0 {
                continue;
            }
            // Standalone files concatenate serialized resource bytes. The
            // descriptor alignment applies when allocating the destination
            // buffers, not to their physical positions in the file.
            let start = cursor;
            let end = start.checked_add(size).ok_or(ArenaError::IntegerOverflow)?;
            let source = bytes
                .get(start..end)
                .ok_or(ArenaError::ResourceOutsideFile {
                    index,
                    range: start..end,
                    file_size: bytes.len(),
                })?;
            resource.extend_from_slice(source);
            cursor = end;
        }
        Ok(resources)
    }

    /// Parses the metadata and object dictionary spanning all five RW4
    /// resource classes.
    ///
    /// Layout evidence comes from the reference debug types
    /// `rw::core::arena::ArenaFileHeaderMagicNumber`, `ArenaFileHeader`,
    /// `Arena`, and `ArenaDictEntry` (sizes 12, 28, 172, and 24 bytes).
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError`] for truncated headers, unsupported target
    /// properties, invalid dictionary bounds, or object ranges outside this
    /// resource class.
    pub fn parse(resources: &[Vec<u8>; RESOURCE_CLASS_COUNT]) -> Result<Self, ArenaError> {
        let resource_zero = &resources[0];
        if resource_zero.len() < ARENA_SIZE {
            return Err(ArenaError::Truncated {
                required: ARENA_SIZE,
                actual: resource_zero.len(),
            });
        }

        let file_header = parse_file_header(resource_zero)?;
        debug_assert_eq!(FILE_HEADER_SIZE, 28);

        let entry_count = be_u32(resource_zero, 32)?;
        let used_entry_count = be_u32(resource_zero, 36)?;
        if used_entry_count > entry_count {
            return Err(ArenaError::UsedCountExceedsCapacity {
                used: used_entry_count,
                capacity: entry_count,
            });
        }
        let dictionary_offset = be_u32(resource_zero, 48)?;
        let dictionary_start =
            usize::try_from(dictionary_offset).map_err(|_| ArenaError::IntegerOverflow)?;
        let dictionary_size = usize::try_from(used_entry_count)
            .ok()
            .and_then(|count| count.checked_mul(DICTIONARY_ENTRY_SIZE))
            .ok_or(ArenaError::IntegerOverflow)?;
        let dictionary_end = dictionary_start
            .checked_add(dictionary_size)
            .ok_or(ArenaError::IntegerOverflow)?;
        if dictionary_start < ARENA_SIZE || dictionary_end > resource_zero.len() {
            return Err(ArenaError::DictionaryOutsideResource {
                range: dictionary_start..dictionary_end,
                resource_size: resource_zero.len(),
            });
        }

        let mut resource_descriptor = [ResourceDescriptor {
            size: 0,
            alignment: 0,
        }; RESOURCE_CLASS_COUNT];
        let mut resources_used = resource_descriptor;
        let mut resource_offsets = [0_u32; RESOURCE_CLASS_COUNT];
        for index in 0..RESOURCE_CLASS_COUNT {
            resource_descriptor[index] = descriptor(resource_zero, 68 + index * 8)?;
            resources_used[index] = descriptor(resource_zero, 108 + index * 8)?;
            resource_offsets[index] = be_u32(resource_zero, 148 + index * 4)?;
            if usize::try_from(resource_descriptor[index].size).ok() != Some(resources[index].len())
            {
                return Err(ArenaError::ResourceSizeMismatch {
                    index,
                    expected: resource_descriptor[index].size,
                    actual: resources[index].len(),
                });
            }
        }

        let dictionary = parse_dictionary(resources, dictionary_start, used_entry_count)?;

        Ok(Self {
            file_header,
            id: be_u32(resource_zero, 28)?,
            entry_count,
            used_entry_count,
            alignment: be_u32(resource_zero, 40)?,
            virtual_size: be_u32(resource_zero, 44)?,
            dictionary_offset,
            sections_offset: be_u32(resource_zero, 52)?,
            base_offset: be_u32(resource_zero, 56)?,
            unfix_context_offset: be_u32(resource_zero, 60)?,
            fix_context_offset: be_u32(resource_zero, 64)?,
            resource_descriptor,
            resources_used,
            resource_offsets,
            arena_group_offset: be_u32(resource_zero, 168)?,
            dictionary,
        })
    }

    /// Returns an object's serialized bytes when the entry range belongs to
    /// this resource buffer.
    #[must_use]
    pub fn object_bytes<'a>(
        &self,
        resources: &'a [Vec<u8>; RESOURCE_CLASS_COUNT],
        entry: &DictionaryEntry,
    ) -> Option<&'a [u8]> {
        resources
            .get(entry.resource_index)?
            .get(entry.object_range.clone())
    }

    /// Resolves an object ID in this arena's local (section zero) dictionary.
    /// External-section IDs are intentionally retained for a future mounted
    /// arena-group resolver.
    #[must_use]
    pub fn local_entry(&self, id: ObjectId) -> Option<&DictionaryEntry> {
        if id.section_index() != 0 {
            return None;
        }
        self.dictionary.get(usize::try_from(id.entry_index()).ok()?)
    }

    /// Parses the arena's section manifest and external-arena ID table.
    ///
    /// The manifest and its section offsets are relative to the manifest base.
    /// The external-arena fixup reserves sections 0 through 2, then consumes
    /// one serialized arena ID for each remaining object-ID section.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError`] for invalid section types, counts, or ranges.
    pub fn sections(
        &self,
        resources: &[Vec<u8>; RESOURCE_CLASS_COUNT],
    ) -> Result<ArenaSections, ArenaError> {
        let bytes = &resources[0];
        let manifest_start =
            usize::try_from(self.sections_offset).map_err(|_| ArenaError::IntegerOverflow)?;
        let manifest_type = be_u32(bytes, manifest_start)?;
        if manifest_type != SECTION_MANIFEST_TYPE_ID {
            return Err(ArenaError::WrongSectionType {
                expected: SECTION_MANIFEST_TYPE_ID,
                actual: manifest_type,
            });
        }
        let section_count = be_u32(bytes, manifest_start + 4)?;
        let section_dict_offset = be_u32(bytes, manifest_start + 8)?;
        let section_dict_start = manifest_start
            .checked_add(
                usize::try_from(section_dict_offset).map_err(|_| ArenaError::IntegerOverflow)?,
            )
            .ok_or(ArenaError::IntegerOverflow)?;
        let section_dict_size = usize::try_from(section_count)
            .ok()
            .and_then(|count| count.checked_mul(4))
            .ok_or(ArenaError::IntegerOverflow)?;
        require_range(bytes, section_dict_start, section_dict_size)?;

        let mut sections = Vec::with_capacity(section_count as usize);
        for index in 0..section_count as usize {
            let relative_offset = be_u32(bytes, section_dict_start + index * 4)?;
            let start = manifest_start
                .checked_add(
                    usize::try_from(relative_offset).map_err(|_| ArenaError::IntegerOverflow)?,
                )
                .ok_or(ArenaError::IntegerOverflow)?;
            require_range(bytes, start, 12)?;
            sections.push(ArenaSection {
                type_id: be_u32(bytes, start)?,
                entry_count: be_u32(bytes, start + 4)?,
                object_offset: u32::try_from(start).map_err(|_| ArenaError::IntegerOverflow)?,
            });
        }

        let external_arena_ids = parse_external_arenas(bytes, &sections)?;
        let subreferences = parse_subreferences(bytes, &sections)?;

        Ok(ArenaSections {
            manifest_offset: self.sections_offset,
            sections,
            external_arena_ids,
            subreferences,
        })
    }
}

fn parse_external_arenas(bytes: &[u8], sections: &[ArenaSection]) -> Result<Vec<u32>, ArenaError> {
    let external = sections
        .get(1)
        .ok_or(ArenaError::MissingExternalArenaSection)?;
    if external.type_id != SECTION_EXTERNAL_ARENAS_TYPE_ID {
        return Err(ArenaError::WrongSectionType {
            expected: SECTION_EXTERNAL_ARENAS_TYPE_ID,
            actual: external.type_id,
        });
    }
    if external.entry_count < 3 {
        return Err(ArenaError::ExternalArenaReservedEntriesMissing {
            actual: external.entry_count,
        });
    }
    let external_count = external.entry_count - 3;
    let start = usize::try_from(external.object_offset)
        .map_err(|_| ArenaError::IntegerOverflow)?
        .checked_add(24)
        .ok_or(ArenaError::IntegerOverflow)?;
    require_range(bytes, start, external_count as usize * 4)?;
    (0..external_count as usize)
        .map(|index| be_u32(bytes, start + index * 4))
        .collect()
}

fn parse_subreferences(
    bytes: &[u8],
    sections: &[ArenaSection],
) -> Result<Vec<ArenaSubreference>, ArenaError> {
    let section = sections
        .get(2)
        .ok_or(ArenaError::MissingSubreferenceSection)?;
    if section.type_id != SECTION_SUBREFERENCES_TYPE_ID {
        return Err(ArenaError::WrongSectionType {
            expected: SECTION_SUBREFERENCES_TYPE_ID,
            actual: section.type_id,
        });
    }
    let start = usize::try_from(section.object_offset).map_err(|_| ArenaError::IntegerOverflow)?;
    require_range(bytes, start, 28)?;
    let dictionary_start =
        usize::try_from(be_u32(bytes, start + 16)?).map_err(|_| ArenaError::IntegerOverflow)?;
    let records_start =
        usize::try_from(be_u32(bytes, start + 20)?).map_err(|_| ArenaError::IntegerOverflow)?;
    let used_count = be_u32(bytes, start + 24)?;
    if used_count > section.entry_count {
        return Err(ArenaError::UsedCountExceedsCapacity {
            used: used_count,
            capacity: section.entry_count,
        });
    }
    require_range(
        bytes,
        dictionary_start,
        used_count as usize * DICTIONARY_ENTRY_SIZE,
    )?;
    require_range(bytes, records_start, used_count as usize * 8)?;
    (0..used_count as usize)
        .map(|index| {
            let dictionary = dictionary_start + index * DICTIONARY_ENTRY_SIZE;
            let record = records_start + index * 8;
            Ok(ArenaSubreference {
                object: ObjectId(be_u32(bytes, record)?),
                object_offset: be_u32(bytes, record + 4)?,
                size: be_u32(bytes, dictionary + 8)?,
                alignment: be_u32(bytes, dictionary + 12)?,
                type_index: be_u32(bytes, dictionary + 16)?,
                type_id: be_u32(bytes, dictionary + 20)?,
            })
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArenaError {
    Truncated {
        required: usize,
        actual: usize,
    },
    WrongMagic {
        actual: [u8; 12],
    },
    UnsupportedLittleEndian,
    UnsupportedPointerSize {
        pointer_size_bits: u8,
    },
    UsedCountExceedsCapacity {
        used: u32,
        capacity: u32,
    },
    DictionaryOutsideResource {
        range: Range<usize>,
        resource_size: usize,
    },
    ObjectOutsideResource {
        index: usize,
        resource_index: usize,
        range: Range<usize>,
        resource_size: usize,
    },
    ResourceSizeMismatch {
        index: usize,
        expected: u32,
        actual: usize,
    },
    ResourceOutsideFile {
        index: usize,
        range: Range<usize>,
        file_size: usize,
    },
    WrongSectionType {
        expected: u32,
        actual: u32,
    },
    MissingExternalArenaSection,
    MissingSubreferenceSection,
    ExternalArenaReservedEntriesMissing {
        actual: u32,
    },
    IntegerOverflow,
}

impl fmt::Display for ArenaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Skate 2 RW4 arena: {self:?}")
    }
}

impl std::error::Error for ArenaError {}

fn descriptor(bytes: &[u8], offset: usize) -> Result<ResourceDescriptor, ArenaError> {
    Ok(ResourceDescriptor {
        size: be_u32(bytes, offset)?,
        alignment: be_u32(bytes, offset + 4)?,
    })
}

fn parse_file_header(bytes: &[u8]) -> Result<FileHeader, ArenaError> {
    let magic = fixed_field::<12>(bytes, 0)?;
    if magic != RW4_MAGIC {
        return Err(ArenaError::WrongMagic { actual: magic });
    }
    let is_big_endian = bytes.get(12).copied().unwrap_or_default() != 0;
    if !is_big_endian {
        return Err(ArenaError::UnsupportedLittleEndian);
    }
    let pointer_size_bits = bytes.get(13).copied().unwrap_or_default();
    if pointer_size_bits != 32 {
        return Err(ArenaError::UnsupportedPointerSize { pointer_size_bits });
    }
    Ok(FileHeader {
        magic,
        is_big_endian,
        pointer_size_bits,
        pointer_alignment: bytes.get(14).copied().unwrap_or_default(),
        unused: bytes.get(15).copied().unwrap_or_default(),
        major_version: fixed_field::<4>(bytes, 16)?,
        minor_version: fixed_field::<4>(bytes, 20)?,
        build_number: be_u32(bytes, 24)?,
    })
}

fn parse_dictionary(
    resources: &[Vec<u8>; RESOURCE_CLASS_COUNT],
    start: usize,
    count: u32,
) -> Result<Vec<DictionaryEntry>, ArenaError> {
    let bytes = &resources[0];
    let mut dictionary = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let offset = start + index * DICTIONARY_ENTRY_SIZE;
        let object_offset = be_u32(bytes, offset)?;
        let size = be_u32(bytes, offset + 8)?;
        let type_id = be_u32(bytes, offset + 20)?;
        let resource_index = resource_index_for_type(type_id);
        let object_resource = &resources[resource_index];
        let object_start =
            usize::try_from(object_offset).map_err(|_| ArenaError::IntegerOverflow)?;
        let object_end = object_start
            .checked_add(usize::try_from(size).map_err(|_| ArenaError::IntegerOverflow)?)
            .ok_or(ArenaError::IntegerOverflow)?;
        if object_end > object_resource.len() {
            return Err(ArenaError::ObjectOutsideResource {
                index,
                resource_index,
                range: object_start..object_end,
                resource_size: object_resource.len(),
            });
        }
        dictionary.push(DictionaryEntry {
            resource_index,
            object_offset,
            relocation_offset: be_u32(bytes, offset + 4)?,
            size,
            alignment: be_u32(bytes, offset + 12)?,
            type_index: be_u32(bytes, offset + 16)?,
            type_id,
            object_range: object_start..object_end,
        });
    }
    Ok(dictionary)
}

const fn resource_index_for_type(type_id: u32) -> usize {
    match type_id {
        0x0001_0030 => 1,
        0x0001_0031 => 2,
        0x0001_0032 => 3,
        0x0001_0033 => 4,
        _ => 0,
    }
}

fn fixed_field<const SIZE: usize>(bytes: &[u8], offset: usize) -> Result<[u8; SIZE], ArenaError> {
    bytes
        .get(offset..offset + SIZE)
        .and_then(|field| field.try_into().ok())
        .ok_or(ArenaError::Truncated {
            required: offset + SIZE,
            actual: bytes.len(),
        })
}

fn be_u32(bytes: &[u8], offset: usize) -> Result<u32, ArenaError> {
    bytes
        .get(offset..offset + 4)
        .ok_or(ArenaError::Truncated {
            required: offset + 4,
            actual: bytes.len(),
        })
        .map(|field| u32::from_be_bytes(field.try_into().unwrap()))
}

fn require_range(bytes: &[u8], start: usize, size: usize) -> Result<(), ArenaError> {
    let end = start.checked_add(size).ok_or(ArenaError::IntegerOverflow)?;
    if end > bytes.len() {
        return Err(ArenaError::Truncated {
            required: end,
            actual: bytes.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_shaped_arena() -> Vec<u8> {
        let mut bytes = vec![0_u8; 0x200];
        bytes[..12].copy_from_slice(&RW4_MAGIC);
        bytes[12] = 1;
        bytes[13] = 32;
        bytes[14] = 4;
        bytes[16..20].copy_from_slice(b"454\0");
        bytes[20..24].copy_from_slice(b"000\0");
        bytes[28..32].copy_from_slice(&0x8646_a1f7_u32.to_be_bytes());
        bytes[32..36].copy_from_slice(&2_u32.to_be_bytes());
        bytes[36..40].copy_from_slice(&2_u32.to_be_bytes());
        bytes[40..44].copy_from_slice(&16_u32.to_be_bytes());
        bytes[48..52].copy_from_slice(&0x1d0_u32.to_be_bytes());
        bytes[52..56].copy_from_slice(&0xac_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&0x200_u32.to_be_bytes());
        bytes[72..76].copy_from_slice(&16_u32.to_be_bytes());
        bytes[108..112].copy_from_slice(&0xac_u32.to_be_bytes());
        bytes[112..116].copy_from_slice(&4_u32.to_be_bytes());

        for (offset, object_offset, size, type_index, type_id) in [
            (0x1d0, 0x100_u32, 0x10_u32, 0x10_u32, 0x00eb_0008_u32),
            (0x1e8, 0x110, 0x20, 0x0d, 0x00eb_000d),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&object_offset.to_be_bytes());
            bytes[offset + 8..offset + 12].copy_from_slice(&size.to_be_bytes());
            bytes[offset + 12..offset + 16].copy_from_slice(&16_u32.to_be_bytes());
            bytes[offset + 16..offset + 20].copy_from_slice(&type_index.to_be_bytes());
            bytes[offset + 20..offset + 24].copy_from_slice(&type_id.to_be_bytes());
        }
        bytes
    }

    #[test]
    fn parses_reference_shaped_arena_dictionary() {
        let bytes = reference_shaped_arena();
        let mut resources: [Vec<u8>; RESOURCE_CLASS_COUNT] = std::array::from_fn(|_| Vec::new());
        resources[0] = bytes;
        let arena = Arena::parse(&resources).unwrap();

        assert_eq!(arena.id, 0x8646_a1f7);
        assert_eq!(arena.file_header.major_version, *b"454\0");
        assert_eq!(arena.dictionary.len(), 2);
        assert_eq!(arena.dictionary[0].object_range, 0x100..0x110);
        assert_eq!(arena.dictionary[1].type_id, 0x00eb_000d);
        assert_eq!(
            arena
                .object_bytes(&resources, &arena.dictionary[1])
                .unwrap()
                .len(),
            0x20
        );
    }

    #[test]
    fn splits_standalone_file_resources_from_descriptors() {
        let mut bytes = reference_shaped_arena();
        bytes[68..72].copy_from_slice(&0x201_u32.to_be_bytes());
        bytes[84..88].copy_from_slice(&8_u32.to_be_bytes());
        bytes[88..92].copy_from_slice(&0x1000_u32.to_be_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&[0xaa; 8]);
        let resources = Arena::split_file_resources(&bytes).unwrap();
        assert_eq!(resources[0], bytes[..0x201]);
        assert_eq!(resources[2], [0xaa; 8]);
        assert!(resources[1].is_empty());
        assert!(resources[3..].iter().all(Vec::is_empty));
    }

    #[test]
    fn rejects_dictionary_objects_outside_resource_zero() {
        let mut bytes = reference_shaped_arena();
        bytes[0x1d8..0x1dc].copy_from_slice(&0x101_u32.to_be_bytes());
        let mut resources: [Vec<u8>; RESOURCE_CLASS_COUNT] = std::array::from_fn(|_| Vec::new());
        resources[0] = bytes;
        assert!(matches!(
            Arena::parse(&resources),
            Err(ArenaError::ObjectOutsideResource { index: 0, .. })
        ));
    }

    #[test]
    fn resolves_base_resource_entries_to_their_resource_class() {
        let mut bytes = reference_shaped_arena();
        bytes[0x1d0..0x1d4].copy_from_slice(&0_u32.to_be_bytes());
        bytes[0x1d8..0x1dc].copy_from_slice(&8_u32.to_be_bytes());
        bytes[0x1e4..0x1e8].copy_from_slice(&0x0001_0031_u32.to_be_bytes());
        let mut resources: [Vec<u8>; RESOURCE_CLASS_COUNT] = std::array::from_fn(|_| Vec::new());
        resources[0] = bytes;
        resources[2] = vec![0xaa; 8];
        resources[0][84..88].copy_from_slice(&8_u32.to_be_bytes());
        resources[0][88..92].copy_from_slice(&1_u32.to_be_bytes());

        let arena = Arena::parse(&resources).unwrap();
        assert_eq!(arena.dictionary[0].resource_index, 2);
        assert_eq!(
            arena.object_bytes(&resources, &arena.dictionary[0]),
            Some(&[0xaa; 8][..])
        );
    }
}
