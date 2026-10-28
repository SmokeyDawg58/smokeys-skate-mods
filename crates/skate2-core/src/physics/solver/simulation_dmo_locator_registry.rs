//! Manager-scoped ownership corresponding to the native manager-global DMO
//! locator table.
//!
//! `Sk8::Dmo::Registration::RegDmoLocator` (`0x825eac28`) keeps the first
//! registration for one full container GUID in a 50-entry manager table.
//! `Sk8::Dmo::Registration::DRegDmoLocator` (`0x825ea6b8`) erases by that GUID
//! and does not compare the supplied source pointer. This owner freezes only
//! that recovered locator registration behavior; model selection remains a
//! separate boundary.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "staged manager-scoped locator owner awaits live DMO registration wiring"
    )
)]

use crate::simulation_dmo_arena_plan::{
    SimulationDmoArenaPlan, SimulationDmoDataHandle, SimulationDmoLocatorOccurrence,
    SimulationDmoLocatorOccurrenceError,
};
use std::{error::Error, fmt};

const DMO_LOCATOR_REGISTRY_CAPACITY: usize = 50;

struct RegisteredDmoLocator {
    container_id: u64,
    source: SimulationDmoDataHandle,
}

/// Construction refusal for the fixed-capacity locator registry storage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoLocatorRegistryInitError {
    StorageAllocation { requested: usize },
}

impl fmt::Display for SimulationDmoLocatorRegistryInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageAllocation { requested } => {
                write!(formatter, "could not reserve {requested} DMO locator rows")
            }
        }
    }
}

impl Error for SimulationDmoLocatorRegistryInitError {}

/// Safe-host translation of the native `FailAssert`/`DebugBreak` path when a
/// new, nonduplicate locator registration exceeds the recovered capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoLocatorRegisterError {
    CapacityExceeded { capacity: usize },
}

impl fmt::Display for SimulationDmoLocatorRegisterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { capacity } => {
                write!(
                    formatter,
                    "DMO locator registry capacity {capacity} is exhausted"
                )
            }
        }
    }
}

impl Error for SimulationDmoLocatorRegisterError {}

/// Effect-free refusal while selecting a locator through its winning
/// manager-scoped registration.
#[derive(Debug)]
pub(crate) enum SimulationDmoRegisteredLocatorOccurrenceError {
    OperationOutOfRange {
        operation_index: usize,
        operation_count: usize,
    },
    OperationSourceUnavailable {
        operation_index: usize,
    },
    NotRegistered,
    RegisteredSourceMismatch,
    Locator(SimulationDmoLocatorOccurrenceError),
}

impl fmt::Display for SimulationDmoRegisteredLocatorOccurrenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationOutOfRange {
                operation_index,
                operation_count,
            } => write!(
                formatter,
                "Simulation DMO operation {operation_index} is outside {operation_count} operations"
            ),
            Self::OperationSourceUnavailable { operation_index } => write!(
                formatter,
                "Simulation DMO operation {operation_index} has no retained source"
            ),
            Self::NotRegistered => formatter.write_str("DMO locator container is not registered"),
            Self::RegisteredSourceMismatch => {
                formatter.write_str("DMO locator container is registered to another exact source")
            }
            Self::Locator(source) => write!(formatter, "could not select DMO locator: {source}"),
        }
    }
}

impl Error for SimulationDmoRegisteredLocatorOccurrenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Locator(source) => Some(source),
            _ => None,
        }
    }
}

/// Fixed-capacity manager-scoped locator registration owner.
///
/// Rows intentionally have no iterator or registration-order accessor. Their
/// storage order is an implementation detail, not a claim about the native
/// manager's hash-table traversal order.
pub(super) struct SimulationDmoLocatorRegistry {
    rows: Vec<RegisteredDmoLocator>,
}

impl SimulationDmoLocatorRegistry {
    /// Preallocates all 50 possible manager rows before publication.
    pub(super) fn try_new() -> Result<Self, SimulationDmoLocatorRegistryInitError> {
        let mut rows = Vec::new();
        rows.try_reserve_exact(DMO_LOCATOR_REGISTRY_CAPACITY)
            .map_err(
                |_| SimulationDmoLocatorRegistryInitError::StorageAllocation {
                    requested: DMO_LOCATOR_REGISTRY_CAPACITY,
                },
            )?;
        Ok(Self { rows })
    }

    /// Keeps the first registration for a full container ID. Duplicate lookup
    /// precedes the capacity refusal and does not replace the winning source.
    pub(super) fn register(
        &mut self,
        source: &SimulationDmoDataHandle,
        container_id: u64,
    ) -> Result<(), SimulationDmoLocatorRegisterError> {
        if self.rows.iter().any(|row| row.container_id == container_id) {
            return Ok(());
        }
        if self.rows.len() == DMO_LOCATOR_REGISTRY_CAPACITY {
            return Err(SimulationDmoLocatorRegisterError::CapacityExceeded {
                capacity: DMO_LOCATOR_REGISTRY_CAPACITY,
            });
        }
        self.rows.push(RegisteredDmoLocator {
            container_id,
            source: source.clone(),
        });
        Ok(())
    }

    /// Erases by full container ID. Native ignores the supplied source pointer;
    /// an absent ID is therefore an intentional no-op.
    pub(super) fn deregister(&mut self, _source: &SimulationDmoDataHandle, container_id: u64) {
        if let Some(index) = self
            .rows
            .iter()
            .position(|row| row.container_id == container_id)
        {
            self.rows.remove(index);
        }
    }

    /// Releases every retained locator registration and no other manager state.
    pub(super) fn clear(&mut self) {
        self.rows.clear();
    }

    /// Selects one opaque occurrence only after the operation's exact current
    /// full-ID winner has matching backing. This is a point-in-time membership
    /// attestation, not a durable native registration token.
    pub(super) fn try_locator_occurrence(
        &self,
        plan: &SimulationDmoArenaPlan,
        operation_index: usize,
        locator_ordinal: usize,
    ) -> Result<SimulationDmoLocatorOccurrence, SimulationDmoRegisteredLocatorOccurrenceError> {
        let operation = plan.operations().get(operation_index).ok_or(
            SimulationDmoRegisteredLocatorOccurrenceError::OperationOutOfRange {
                operation_index,
                operation_count: plan.operations().len(),
            },
        )?;
        let source = plan.source_handle(operation_index).ok_or(
            SimulationDmoRegisteredLocatorOccurrenceError::OperationSourceUnavailable {
                operation_index,
            },
        )?;
        let registered = self
            .rows
            .iter()
            .find(|row| row.container_id == operation.asset_id)
            .ok_or(SimulationDmoRegisteredLocatorOccurrenceError::NotRegistered)?;
        if !registered.source.is_same_backing_object(&source) {
            return Err(SimulationDmoRegisteredLocatorOccurrenceError::RegisteredSourceMismatch);
        }
        plan.try_locator_occurrence(operation_index, locator_ordinal)
            .map_err(SimulationDmoRegisteredLocatorOccurrenceError::Locator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_normal_arena_plan::SimulationNormalArenaPlan,
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        pegasus::{DMO_DATA_TYPE_ID, ObjectId, TABLE_OF_CONTENTS_TYPE_ID},
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::sync::Arc;

    type LocatorRecord = (u64, u64, u64);

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn dmo_data(records: &[LocatorRecord]) -> Vec<u8> {
        let records_end = 20 + records.len() * 128;
        let mut bytes = vec![0; records_end + records.len() * 2 + 1];
        put_u32(&mut bytes, 0, DMO_DATA_TYPE_ID);
        put_u32(&mut bytes, 4, u32::try_from(records.len()).unwrap());
        put_u32(&mut bytes, 12, 20);
        put_u32(&mut bytes, 16, u32::try_from(records_end).unwrap());
        for (ordinal, &(guid, local_guid, model_key)) in records.iter().enumerate() {
            let record = 20 + ordinal * 128;
            put_u64(&mut bytes, record + 0x60, guid);
            put_u64(&mut bytes, record + 0x68, local_guid);
            put_u64(&mut bytes, record + 0x70, model_key);
            let name = records_end + ordinal * 2;
            put_u32(&mut bytes, record + 0x7c, u32::try_from(name).unwrap());
            bytes[name] = b'x';
        }
        bytes
    }

    fn toc(entries: &[(u64, ObjectId)]) -> Vec<u8> {
        let names = 20 + entries.len() * 24;
        let mut bytes = vec![0; names + 1];
        put_u32(&mut bytes, 0, u32::try_from(entries.len()).unwrap());
        put_u32(&mut bytes, 4, 20);
        put_u32(&mut bytes, 8, u32::try_from(names).unwrap());
        put_u32(&mut bytes, 16, u32::try_from(names).unwrap());
        for (ordinal, &(container_id, object_id)) in entries.iter().enumerate() {
            let entry = 20 + ordinal * 24;
            put_u32(&mut bytes, entry, u32::try_from(names).unwrap());
            put_u64(&mut bytes, entry + 8, container_id);
            put_u32(&mut bytes, entry + 16, DMO_DATA_TYPE_ID);
            put_u32(&mut bytes, entry + 20, object_id.0);
        }
        bytes
    }

    fn push_object(
        bytes: &mut Vec<u8>,
        dictionary: &mut Vec<DictionaryEntry>,
        type_id: u32,
        object: &[u8],
    ) -> ObjectId {
        let start = bytes.len();
        bytes.extend_from_slice(object);
        let end = bytes.len();
        let object_id = ObjectId(u32::try_from(dictionary.len()).unwrap());
        dictionary.push(DictionaryEntry {
            resource_index: 0,
            object_offset: u32::try_from(start).unwrap(),
            relocation_offset: 0,
            size: u32::try_from(object.len()).unwrap(),
            alignment: 1,
            type_index: 0,
            type_id,
            object_range: start..end,
        });
        object_id
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one coherent fixture preserves TOC, object, and allocation identity"
    )]
    fn locator_plan(
        container_ids: &[u64],
        records: &[LocatorRecord],
        alias_one_object: bool,
    ) -> SimulationDmoArenaPlan {
        let mut bytes = Vec::new();
        let mut dictionary = Vec::new();
        let object = dmo_data(records);
        let shared = alias_one_object
            .then(|| push_object(&mut bytes, &mut dictionary, DMO_DATA_TYPE_ID, &object));
        let mut entries = Vec::new();
        for &container_id in container_ids {
            let object_id = shared.unwrap_or_else(|| {
                push_object(&mut bytes, &mut dictionary, DMO_DATA_TYPE_ID, &object)
            });
            entries.push((container_id, object_id));
        }
        let toc = toc(&entries);
        push_object(&mut bytes, &mut dictionary, TABLE_OF_CONTENTS_TYPE_ID, &toc);
        let descriptor = ResourceDescriptor {
            size: u32::try_from(bytes.len()).unwrap(),
            alignment: 1,
        };
        let empty = ResourceDescriptor {
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
            entry_count: u32::try_from(dictionary.len()).unwrap(),
            used_entry_count: u32::try_from(dictionary.len()).unwrap(),
            alignment: 1,
            virtual_size: descriptor.size,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: [descriptor, empty, empty, empty, empty],
            resources_used: [descriptor, empty, empty, empty, empty],
            resource_offsets: [u32::MAX; 5],
            arena_group_offset: 0,
            dictionary,
        };
        let collision = SimulationCollisionAsset {
            decoded: DecodedArenaAsset::from_parts_for_test(
                0x1234_5678_9abc_def0,
                WORLD_STREAM_ARENA_ASSET_TYPE,
                ArenaResources {
                    resources: [bytes, Vec::new(), Vec::new(), Vec::new(), Vec::new()],
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
            collision,
            Vec::new(),
        ));
        let normal_plan =
            Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap());
        SimulationDmoArenaPlan::from_normal_plan(normal_plan).unwrap()
    }

    #[test]
    fn capacity_duplicate_precedence_and_reuse_are_atomic() {
        let plan_a = locator_plan(&[7], &[(1, 2, 3)], false);
        let plan_b = locator_plan(&[7], &[(1, 2, 3)], false);
        let source_a = plan_a.source_handle(0).unwrap();
        let source_b = plan_b.source_handle(0).unwrap();
        let mut registry = SimulationDmoLocatorRegistry::try_new().unwrap();
        assert!(registry.rows.capacity() >= DMO_LOCATOR_REGISTRY_CAPACITY);
        for container_id in 0..u64::try_from(DMO_LOCATOR_REGISTRY_CAPACITY).unwrap() {
            registry.register(&source_a, container_id).unwrap();
        }
        registry.register(&source_b, 7).unwrap();
        assert_eq!(registry.rows.len(), DMO_LOCATOR_REGISTRY_CAPACITY);
        let duplicate_winner = registry
            .rows
            .iter()
            .find(|row| row.container_id == 7)
            .unwrap();
        assert!(duplicate_winner.source.is_same_backing_object(&source_a));
        assert!(!duplicate_winner.source.is_same_backing_object(&source_b));

        assert_eq!(
            registry.register(&source_b, 50),
            Err(SimulationDmoLocatorRegisterError::CapacityExceeded { capacity: 50 })
        );
        assert_eq!(registry.rows.len(), 50);
        assert!(!registry.rows.iter().any(|row| row.container_id == 50));
        registry.deregister(&source_b, 7);
        registry.register(&source_b, 50).unwrap();
        assert_eq!(registry.rows.len(), 50);
        assert!(
            registry
                .rows
                .iter()
                .find(|row| row.container_id == 50)
                .unwrap()
                .source
                .is_same_backing_object(&source_b)
        );
    }

    #[test]
    fn deregister_ignores_source_missing_is_noop_and_aba_uses_new_source() {
        let plan_a = locator_plan(&[u64::MAX], &[(1, 2, 3)], false);
        let plan_b = locator_plan(&[u64::MAX], &[(1, 2, 3)], false);
        let source_a = plan_a.source_handle(0).unwrap();
        let source_b = plan_b.source_handle(0).unwrap();
        let mut registry = SimulationDmoLocatorRegistry::try_new().unwrap();
        registry.register(&source_a, u64::MAX).unwrap();
        registry.deregister(&source_b, 0);
        assert_eq!(registry.rows.len(), 1);
        registry.deregister(&source_b, u64::MAX);
        assert!(registry.rows.is_empty());
        registry.register(&source_b, u64::MAX).unwrap();
        let replacement = registry
            .rows
            .iter()
            .find(|row| row.container_id == u64::MAX)
            .unwrap();
        assert!(replacement.source.is_same_backing_object(&source_b));
        assert!(!replacement.source.is_same_backing_object(&source_a));
        assert!(matches!(
            registry.try_locator_occurrence(&plan_a, 0, 0),
            Err(SimulationDmoRegisteredLocatorOccurrenceError::RegisteredSourceMismatch)
        ));
        assert!(registry.try_locator_occurrence(&plan_b, 0, 0).is_ok());
    }

    #[test]
    fn full_width_ids_with_equal_low_words_coexist_and_deregister_independently() {
        const FIRST: u64 = 0x1111_0000_dead_beef;
        const SECOND: u64 = 0x2222_0000_dead_beef;
        let plan = locator_plan(&[FIRST, SECOND], &[(1, 2, 3)], true);
        let first_source = plan.source_handle(0).unwrap();
        let second_source = plan.source_handle(1).unwrap();
        let mut registry = SimulationDmoLocatorRegistry::try_new().unwrap();
        registry.register(&first_source, FIRST).unwrap();
        registry.register(&second_source, SECOND).unwrap();
        assert_eq!(registry.rows.len(), 2);
        assert!(registry.rows.iter().any(|row| row.container_id == FIRST));
        assert!(registry.rows.iter().any(|row| row.container_id == SECOND));
        registry.deregister(&first_source, SECOND);
        assert!(registry.rows.iter().any(|row| row.container_id == FIRST));
        assert!(!registry.rows.iter().any(|row| row.container_id == SECOND));
        assert!(registry.try_locator_occurrence(&plan, 0, 0).is_ok());
        assert!(matches!(
            registry.try_locator_occurrence(&plan, 1, 0),
            Err(SimulationDmoRegisteredLocatorOccurrenceError::NotRegistered)
        ));
    }

    #[test]
    fn clear_releases_exact_sources_and_can_be_reused() {
        let plan = locator_plan(&[9], &[(1, 2, 3)], false);
        let owner = plan.normal_plan().static_plan_owner();
        let weak_owner = Arc::downgrade(&owner);
        let source = plan.source_handle(0).unwrap();
        let mut registry = SimulationDmoLocatorRegistry::try_new().unwrap();
        registry.register(&source, 9).unwrap();
        drop(source);
        drop(owner);
        drop(plan);
        assert!(weak_owner.upgrade().is_some());
        registry.clear();
        assert!(weak_owner.upgrade().is_none());
        registry.clear();
        assert!(registry.rows.is_empty());

        let replacement = locator_plan(&[10], &[(4, 5, 6)], false);
        registry
            .register(&replacement.source_handle(0).unwrap(), 10)
            .unwrap();
        assert_eq!(registry.rows.len(), 1);
    }

    #[test]
    fn selection_checks_operation_then_registration_source_then_ordinal() {
        let winning = locator_plan(&[0x7000_0000_0000_0007], &[(1, 2, 3)], false);
        let equal_bytes_foreign = locator_plan(&[0x7000_0000_0000_0007], &[(1, 2, 3)], false);
        let source = winning.source_handle(0).unwrap();
        let mut registry = SimulationDmoLocatorRegistry::try_new().unwrap();
        registry
            .register(&source, winning.operations()[0].asset_id)
            .unwrap();

        assert!(matches!(
            registry.try_locator_occurrence(&winning, 1, usize::MAX),
            Err(
                SimulationDmoRegisteredLocatorOccurrenceError::OperationOutOfRange {
                    operation_index: 1,
                    operation_count: 1,
                }
            )
        ));
        assert!(matches!(
            registry.try_locator_occurrence(&equal_bytes_foreign, 0, usize::MAX),
            Err(SimulationDmoRegisteredLocatorOccurrenceError::RegisteredSourceMismatch)
        ));
        assert!(matches!(
            registry.try_locator_occurrence(&winning, 0, 1),
            Err(SimulationDmoRegisteredLocatorOccurrenceError::Locator(
                SimulationDmoLocatorOccurrenceError::LocatorOrdinalOutOfRange {
                    locator_ordinal: 1,
                    locator_count: 1,
                    ..
                }
            ))
        ));
        assert!(registry.try_locator_occurrence(&winning, 0, 0).is_ok());

        let absent = locator_plan(&[0x8000_0000_0000_0008], &[(1, 2, 3)], false);
        assert!(matches!(
            registry.try_locator_occurrence(&absent, 0, usize::MAX),
            Err(SimulationDmoRegisteredLocatorOccurrenceError::NotRegistered)
        ));
    }

    #[test]
    fn aliased_object_registrations_consume_two_rows_and_distinct_occurrences() {
        let plan = locator_plan(
            &[0x1111_0000_0000_0001, 0x2222_0000_0000_0002],
            &[(0, 0, 0)],
            true,
        );
        let first_source = plan.source_handle(0).unwrap();
        let second_source = plan.source_handle(1).unwrap();
        assert!(first_source.is_same_backing_object(&second_source));
        let mut registry = SimulationDmoLocatorRegistry::try_new().unwrap();
        registry
            .register(&first_source, plan.operations()[0].asset_id)
            .unwrap();
        registry
            .register(&second_source, plan.operations()[1].asset_id)
            .unwrap();
        assert_eq!(registry.rows.len(), 2);
        let first = registry.try_locator_occurrence(&plan, 0, 0).unwrap();
        let second = registry.try_locator_occurrence(&plan, 1, 0).unwrap();
        assert!(!first.is_same_occurrence(&second));
    }
}
