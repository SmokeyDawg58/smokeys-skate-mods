//! Owner-safe streamed Location registration for the separate kind-21 pass.
//!
//! The reference manager resolves every container again by its full asset ID,
//! inserts named descriptors after existing equal hashes, and removes the first
//! equal-hash entry with the exact descriptor pointer. This module preserves
//! that behavior while keeping serialized arena allocations alive explicitly.

use crate::location_arena_plan::{
    LocationArenaOperation, LocationArenaPlan, LocationDataHandle, LocationRecordHandle,
};
use skate2_formats::{hash::sk8_hash_value_32, pegasus::PegasusError};
use std::{
    error::Error,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Weak},
};

/// Live values read by `AddLocationData` and `RemoveLocationData`.
///
/// `resolve_location_data` models the fresh full-ID Simulation-registry lookup
/// performed once per container call. `high_priority_raw` is sampled once per
/// descriptor whose name pointer is non-null. Implementations may return any
/// raw byte; the native stored field itself is a `bool`, but preserving the byte
/// keeps this host boundary from silently normalizing unusual state.
pub trait LocationArenaRuntimeSource {
    fn resolve_location_data(&mut self, asset_id: u64) -> Option<LocationDataHandle>;
    fn high_priority_raw(&mut self) -> u8;
}

#[derive(Clone, Debug)]
struct PreparedLocationContainer {
    operation: LocationArenaOperation,
    planned_source: LocationDataHandle,
}

#[derive(Debug)]
struct PreparedLocationArenaInner {
    plan: Arc<LocationArenaPlan>,
    containers: Vec<PreparedLocationContainer>,
}

/// Fully validated, reusable description of one Location arena pass.
pub struct PreparedLocationArena {
    inner: Arc<PreparedLocationArenaInner>,
}

impl fmt::Debug for PreparedLocationArena {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedLocationArena")
            .field("container_count", &self.inner.containers.len())
            .finish_non_exhaustive()
    }
}

impl PreparedLocationArena {
    /// Validates operation/source agreement and every raw descriptor name before
    /// runtime registry reads or map effects.
    ///
    /// # Errors
    ///
    /// Returns a typed allocation, source, descriptor, or Pegasus error.
    pub fn try_new(plan: Arc<LocationArenaPlan>) -> Result<Self, LocationArenaPrepareError> {
        let requested = plan.operations().len();
        let mut containers = Vec::new();
        containers
            .try_reserve_exact(requested)
            .map_err(|_| LocationArenaPrepareError::ContainerStorageAllocation { requested })?;
        for (operation_index, operation) in plan.operations().iter().copied().enumerate() {
            let source = plan
                .source_handle(operation_index)
                .ok_or(LocationArenaPrepareError::MissingPlannedSource { operation_index })?;
            let expected_len = usize::try_from(operation.source_object_len)
                .map_err(|_| LocationArenaPrepareError::SourceLengthOverflow { operation_index })?;
            let expected_locations = usize::try_from(operation.location_count).map_err(|_| {
                LocationArenaPrepareError::LocationCountOverflow { operation_index }
            })?;
            if source.bytes().len() != expected_len {
                return Err(LocationArenaPrepareError::SourceLengthMismatch { operation_index });
            }
            if source.location_count() != expected_locations {
                return Err(LocationArenaPrepareError::LocationCountMismatch { operation_index });
            }
            for descriptor_index in 0..source.location_count() {
                let descriptor = source.location_handle(descriptor_index).ok_or(
                    LocationArenaPrepareError::MissingDescriptor {
                        operation_index,
                        descriptor_index,
                    },
                )?;
                descriptor
                    .name_bytes()
                    .map_err(LocationArenaPrepareError::Pegasus)?;
            }
            containers.push(PreparedLocationContainer {
                operation,
                planned_source: source,
            });
        }
        Ok(Self {
            inner: Arc::new(PreparedLocationArenaInner { plan, containers }),
        })
    }

    #[must_use]
    pub fn container_count(&self) -> usize {
        self.inner.containers.len()
    }

    #[must_use]
    pub fn plan(&self) -> &LocationArenaPlan {
        &self.inner.plan
    }

    /// Returns the producer-plan operation at `container_index`.
    #[must_use]
    pub fn operation(&self, container_index: usize) -> Option<LocationArenaOperation> {
        self.inner
            .containers
            .get(container_index)
            .map(|container| container.operation)
    }

    /// Clones the producer-time source authority for registry publication.
    #[must_use]
    pub fn planned_source(&self, container_index: usize) -> Option<LocationDataHandle> {
        self.inner
            .containers
            .get(container_index)
            .map(|container| container.planned_source.clone())
    }
}

/// One streamed entry in the reconstructed Location hash multimap.
#[derive(Clone, Debug)]
pub struct StreamedLocationEntry {
    name_hash: u32,
    high_priority_raw: u8,
    descriptor: LocationRecordHandle,
}

impl StreamedLocationEntry {
    #[must_use]
    pub const fn name_hash(&self) -> u32 {
        self.name_hash
    }

    #[must_use]
    pub const fn high_priority_raw(&self) -> u8 {
        self.high_priority_raw
    }

    #[must_use]
    pub const fn streamed_location(&self) -> bool {
        true
    }

    #[must_use]
    pub const fn descriptor(&self) -> &LocationRecordHandle {
        &self.descriptor
    }
}

#[derive(Debug)]
struct LiveLocationRegistration {
    id: u64,
    prepared: Arc<PreparedLocationArenaInner>,
    resolved_sources: Vec<LocationDataHandle>,
}

/// Executes Location kind-21 container calls and owns the streamed multimap.
///
/// The runtime source is declared first so it is destroyed before any retained
/// record handles it might reference. This is the streamed slice only: native
/// debug seed locations and nonstreamed query producers are outside this type.
/// Neither the adapter nor its tokens performs implicit work from `Drop`.
pub struct LocationArenaRegistrationAdapter<R> {
    runtime: R,
    entries: Vec<StreamedLocationEntry>,
    live: Vec<LiveLocationRegistration>,
    domain: Arc<()>,
    next_registration_id: u64,
    fault: Option<LocationAdapterFault>,
}

impl<R: fmt::Debug> fmt::Debug for LocationArenaRegistrationAdapter<R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocationArenaRegistrationAdapter")
            .field("runtime", &self.runtime)
            .field("entry_count", &self.entries.len())
            .field("live_registration_count", &self.live.len())
            .field("next_registration_id", &self.next_registration_id)
            .field("fault", &self.fault)
            .finish_non_exhaustive()
    }
}

impl<R: LocationArenaRuntimeSource> LocationArenaRegistrationAdapter<R> {
    #[must_use]
    pub fn new(runtime: R) -> Self {
        Self {
            runtime,
            entries: Vec::new(),
            live: Vec::new(),
            domain: Arc::new(()),
            next_registration_id: 0,
            fault: None,
        }
    }

    /// Resolves and inserts every container/descriptor in forward order.
    ///
    /// Registration IDs, owner storage, and per-container entry capacity are
    /// checked host safety. A runtime panic, missing registry asset, malformed
    /// resolved source, or allocation failure after the pass starts latches the
    /// exact prefix and performs no rollback.
    ///
    /// # Errors
    ///
    /// Returns a typed pre-effect storage/exhaustion error, an existing fault,
    /// or the newly latched runtime fault.
    pub fn register(
        &mut self,
        prepared: &PreparedLocationArena,
    ) -> Result<LocationArenaRegistration, LocationRegistrationError> {
        if let Some(fault) = self.fault {
            return Err(LocationRegistrationError::AdapterFaulted(fault));
        }
        self.live
            .try_reserve(1)
            .map_err(|_| LocationRegistrationError::LiveStorageAllocation {
                requested: self.live.len().saturating_add(1),
            })?;
        let requested = prepared.inner.containers.len();
        let mut resolved_sources = Vec::new();
        resolved_sources
            .try_reserve_exact(requested)
            .map_err(|_| LocationRegistrationError::SourceStorageAllocation { requested })?;
        let registration_id = self.next_registration_id;
        self.next_registration_id = registration_id
            .checked_add(1)
            .ok_or(LocationRegistrationError::RegistrationIdExhausted)?;
        self.live.push(LiveLocationRegistration {
            id: registration_id,
            prepared: Arc::clone(&prepared.inner),
            resolved_sources,
        });
        let live_index = self.live.len() - 1;
        let token = LocationArenaRegistration {
            domain: Arc::downgrade(&self.domain),
            id: registration_id,
        };

        for (operation_index, container) in prepared.inner.containers.iter().enumerate() {
            if let Err((descriptor_index, stage)) = self.register_container(live_index, container) {
                let fault = self.latch_fault(
                    LocationAdapterFaultPhase::Registration,
                    operation_index,
                    descriptor_index,
                    stage,
                );
                return Err(LocationRegistrationError::RuntimeFault(fault));
            }
        }
        Ok(token)
    }

    /// Freshly resolves every container and removes exact descriptor matches in
    /// forward order. No high-priority value is sampled during teardown.
    ///
    /// # Errors
    ///
    /// Foreign, unknown, and pre-fault tokens are returned without effects. A
    /// runtime failure after validation consumes the token and retains all live
    /// owners and the reached removal prefix in a permanently faulted adapter.
    pub fn deregister(
        &mut self,
        registration: LocationArenaRegistration,
    ) -> Result<(), LocationDeregistrationError> {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(LocationDeregistrationError::ForeignDomain(registration));
        }
        if let Some(fault) = self.fault {
            return Err(LocationDeregistrationError::AdapterFaulted {
                fault,
                registration,
            });
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            return Err(LocationDeregistrationError::UnknownRegistration(
                registration,
            ));
        };
        let prepared = Arc::clone(&self.live[live_index].prepared);
        for (operation_index, container) in prepared.containers.iter().enumerate() {
            if let Err((descriptor_index, stage)) = self.deregister_container(container) {
                let fault = self.latch_fault(
                    LocationAdapterFaultPhase::Deregistration,
                    operation_index,
                    descriptor_index,
                    stage,
                );
                return Err(LocationDeregistrationError::RuntimeFault(fault));
            }
        }
        self.live.swap_remove(live_index);
        Ok(())
    }

    fn resolve_source(
        &mut self,
        asset_id: u64,
    ) -> Result<LocationDataHandle, LocationAdapterFaultStage> {
        match catch_unwind(AssertUnwindSafe(|| {
            self.runtime.resolve_location_data(asset_id)
        })) {
            Ok(Some(source)) => Ok(source),
            Ok(None) => Err(LocationAdapterFaultStage::MissingRegistrySource),
            Err(_) => Err(LocationAdapterFaultStage::RegistryResolve),
        }
    }

    fn register_container(
        &mut self,
        live_index: usize,
        container: &PreparedLocationContainer,
    ) -> Result<(), (Option<usize>, LocationAdapterFaultStage)> {
        let source = self
            .resolve_source(container.operation.asset_id)
            .map_err(|stage| (None, stage))?;
        self.live[live_index].resolved_sources.push(source.clone());
        self.entries
            .try_reserve(source.location_count())
            .map_err(|_| (None, LocationAdapterFaultStage::EntryStorage))?;
        for descriptor_index in 0..source.location_count() {
            let descriptor = source.location_handle(descriptor_index).ok_or((
                Some(descriptor_index),
                LocationAdapterFaultStage::Descriptor,
            ))?;
            let name = descriptor.name_bytes().map_err(|_| {
                (
                    Some(descriptor_index),
                    LocationAdapterFaultStage::Descriptor,
                )
            })?;
            let Some(name) = name else {
                continue;
            };
            let name_hash = sk8_hash_value_32(name);
            let high_priority_raw =
                catch_unwind(AssertUnwindSafe(|| self.runtime.high_priority_raw())).map_err(
                    |_| {
                        (
                            Some(descriptor_index),
                            LocationAdapterFaultStage::HighPriority,
                        )
                    },
                )?;
            let insertion = self
                .entries
                .partition_point(|entry| entry.name_hash <= name_hash);
            self.entries.insert(
                insertion,
                StreamedLocationEntry {
                    name_hash,
                    high_priority_raw,
                    descriptor,
                },
            );
        }
        Ok(())
    }

    fn deregister_container(
        &mut self,
        container: &PreparedLocationContainer,
    ) -> Result<(), (Option<usize>, LocationAdapterFaultStage)> {
        let source = self
            .resolve_source(container.operation.asset_id)
            .map_err(|stage| (None, stage))?;
        for descriptor_index in 0..source.location_count() {
            let descriptor = source.location_handle(descriptor_index).ok_or((
                Some(descriptor_index),
                LocationAdapterFaultStage::Descriptor,
            ))?;
            let name = descriptor.name_bytes().map_err(|_| {
                (
                    Some(descriptor_index),
                    LocationAdapterFaultStage::Descriptor,
                )
            })?;
            let Some(name) = name else {
                continue;
            };
            let name_hash = sk8_hash_value_32(name);
            let first = self
                .entries
                .partition_point(|entry| entry.name_hash < name_hash);
            let last = self
                .entries
                .partition_point(|entry| entry.name_hash <= name_hash);
            if let Some(relative) = self.entries[first..last]
                .iter()
                .position(|entry| entry.descriptor.is_same_backing_record(&descriptor))
            {
                self.entries.remove(first + relative);
            }
        }
        Ok(())
    }

    fn latch_fault(
        &mut self,
        phase: LocationAdapterFaultPhase,
        operation_index: usize,
        descriptor_index: Option<usize>,
        stage: LocationAdapterFaultStage,
    ) -> LocationAdapterFault {
        let fault = LocationAdapterFault {
            phase,
            operation_index,
            descriptor_index,
            stage,
        };
        self.fault.get_or_insert(fault);
        self.fault.unwrap_or(fault)
    }

    #[must_use]
    pub fn entries(&self) -> &[StreamedLocationEntry] {
        &self.entries
    }

    #[must_use]
    pub fn live_registration_count(&self) -> usize {
        self.live.len()
    }

    #[must_use]
    pub const fn runtime(&self) -> &R {
        &self.runtime
    }

    #[must_use]
    pub const fn runtime_mut(&mut self) -> &mut R {
        &mut self.runtime
    }

    #[must_use]
    pub const fn fault(&self) -> Option<LocationAdapterFault> {
        self.fault
    }
}

/// Non-cloneable authority for one live Location arena-pass occurrence.
pub struct LocationArenaRegistration {
    domain: Weak<()>,
    id: u64,
}

impl fmt::Debug for LocationArenaRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocationArenaRegistration")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum LocationArenaPrepareError {
    ContainerStorageAllocation {
        requested: usize,
    },
    MissingPlannedSource {
        operation_index: usize,
    },
    SourceLengthOverflow {
        operation_index: usize,
    },
    SourceLengthMismatch {
        operation_index: usize,
    },
    LocationCountOverflow {
        operation_index: usize,
    },
    LocationCountMismatch {
        operation_index: usize,
    },
    MissingDescriptor {
        operation_index: usize,
        descriptor_index: usize,
    },
    Pegasus(PegasusError),
}

impl fmt::Display for LocationArenaPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContainerStorageAllocation { requested } => {
                write!(
                    formatter,
                    "could not retain {requested} Location containers"
                )
            }
            Self::MissingPlannedSource { operation_index } => {
                write!(
                    formatter,
                    "Location operation {operation_index} has no source"
                )
            }
            Self::SourceLengthOverflow { operation_index } => write!(
                formatter,
                "Location operation {operation_index} source length does not fit usize"
            ),
            Self::SourceLengthMismatch { operation_index } => write!(
                formatter,
                "Location operation {operation_index} source length changed"
            ),
            Self::LocationCountOverflow { operation_index } => write!(
                formatter,
                "Location operation {operation_index} descriptor count does not fit usize"
            ),
            Self::LocationCountMismatch { operation_index } => write!(
                formatter,
                "Location operation {operation_index} descriptor count changed"
            ),
            Self::MissingDescriptor {
                operation_index,
                descriptor_index,
            } => write!(
                formatter,
                "Location operation {operation_index} lost descriptor {descriptor_index}"
            ),
            Self::Pegasus(source) => {
                write!(formatter, "prepared Location data is invalid: {source}")
            }
        }
    }
}

impl Error for LocationArenaPrepareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Pegasus(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocationAdapterFault {
    phase: LocationAdapterFaultPhase,
    operation_index: usize,
    descriptor_index: Option<usize>,
    stage: LocationAdapterFaultStage,
}

impl LocationAdapterFault {
    #[must_use]
    pub const fn phase(self) -> LocationAdapterFaultPhase {
        self.phase
    }

    #[must_use]
    pub const fn operation_index(self) -> usize {
        self.operation_index
    }

    #[must_use]
    pub const fn descriptor_index(self) -> Option<usize> {
        self.descriptor_index
    }

    #[must_use]
    pub const fn stage(self) -> LocationAdapterFaultStage {
        self.stage
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocationAdapterFaultPhase {
    Registration,
    Deregistration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocationAdapterFaultStage {
    RegistryResolve,
    MissingRegistrySource,
    Descriptor,
    HighPriority,
    EntryStorage,
}

#[derive(Debug)]
pub enum LocationRegistrationError {
    AdapterFaulted(LocationAdapterFault),
    LiveStorageAllocation { requested: usize },
    SourceStorageAllocation { requested: usize },
    RegistrationIdExhausted,
    RuntimeFault(LocationAdapterFault),
}

impl fmt::Display for LocationRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AdapterFaulted(_) => "Location adapter is faulted",
            Self::LiveStorageAllocation { .. } => "could not allocate a live Location registration",
            Self::SourceStorageAllocation { .. } => "could not retain Location registry sources",
            Self::RegistrationIdExhausted => "Location registration IDs are exhausted",
            Self::RuntimeFault(_) => "Location registration faulted after runtime work began",
        })
    }
}

impl Error for LocationRegistrationError {}

#[derive(Debug)]
pub enum LocationDeregistrationError {
    ForeignDomain(LocationArenaRegistration),
    UnknownRegistration(LocationArenaRegistration),
    AdapterFaulted {
        fault: LocationAdapterFault,
        registration: LocationArenaRegistration,
    },
    RuntimeFault(LocationAdapterFault),
}

impl LocationDeregistrationError {
    #[must_use]
    pub fn into_registration(self) -> Option<LocationArenaRegistration> {
        match self {
            Self::ForeignDomain(registration)
            | Self::UnknownRegistration(registration)
            | Self::AdapterFaulted { registration, .. } => Some(registration),
            Self::RuntimeFault(_) => None,
        }
    }
}

impl fmt::Display for LocationDeregistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ForeignDomain(_) => "registration belongs to another Location adapter",
            Self::UnknownRegistration(_) => "Location registration is not live",
            Self::AdapterFaulted { .. } => "Location adapter is faulted",
            Self::RuntimeFault(_) => "Location deregistration faulted after runtime work began",
        })
    }
}

impl Error for LocationDeregistrationError {}

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
        pegasus::{LOCATION_DESC_DATA_TYPE_ID, ObjectId, TABLE_OF_CONTENTS_TYPE_ID},
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::{
        collections::{BTreeMap, VecDeque},
        ops::Range,
    };

    #[derive(Default)]
    struct Runtime {
        sources: BTreeMap<u64, LocationDataHandle>,
        priorities: VecDeque<u8>,
        resolutions: Vec<u64>,
        priority_samples: usize,
        panic_resolve_at: Option<usize>,
        panic_priority_at: Option<usize>,
    }

    impl LocationArenaRuntimeSource for Runtime {
        fn resolve_location_data(&mut self, asset_id: u64) -> Option<LocationDataHandle> {
            let index = self.resolutions.len();
            self.resolutions.push(asset_id);
            assert_ne!(self.panic_resolve_at, Some(index));
            self.sources.get(&asset_id).cloned()
        }

        fn high_priority_raw(&mut self) -> u8 {
            let index = self.priority_samples;
            self.priority_samples += 1;
            assert_ne!(self.panic_priority_at, Some(index));
            self.priorities.pop_front().unwrap_or(0)
        }
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn location_data(names: &[Option<&str>]) -> Vec<u8> {
        let records_start = 24;
        let records_end = records_start + names.len() * 128;
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
            let record = records_start + index * 128;
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

    fn toc(entries: &[(u64, ObjectId)]) -> Vec<u8> {
        let names = 20 + entries.len() * 24;
        let mut bytes = vec![0; names + 1];
        put_u32(&mut bytes, 0, u32::try_from(entries.len()).unwrap());
        put_u32(&mut bytes, 4, 20);
        put_u32(&mut bytes, 8, u32::try_from(names).unwrap());
        put_u32(&mut bytes, 16, u32::try_from(names).unwrap());
        for (index, (guid, object)) in entries.iter().enumerate() {
            let offset = 20 + index * 24;
            put_u32(&mut bytes, offset, u32::try_from(names).unwrap());
            put_u64(&mut bytes, offset + 8, *guid);
            put_u32(&mut bytes, offset + 16, LOCATION_DESC_DATA_TYPE_ID);
            put_u32(&mut bytes, offset + 20, object.0);
        }
        bytes
    }

    fn make_prepared(containers: &[(u64, Vec<Option<&str>>)]) -> PreparedLocationArena {
        let mut resource = Vec::new();
        let mut dictionary = Vec::new();
        let mut objects = Vec::new();
        for (_, names) in containers {
            let object = location_data(names);
            let start = resource.len();
            resource.extend_from_slice(&object);
            let end = resource.len();
            let index = dictionary.len();
            dictionary.push(DictionaryEntry {
                resource_index: 0,
                object_offset: u32::try_from(start).unwrap(),
                relocation_offset: 0,
                size: u32::try_from(object.len()).unwrap(),
                alignment: 1,
                type_index: 0,
                type_id: LOCATION_DESC_DATA_TYPE_ID,
                object_range: Range { start, end },
            });
            objects.push(ObjectId(u32::try_from(index).unwrap()));
        }
        let toc = toc(&containers
            .iter()
            .zip(objects)
            .map(|((id, _), object)| (*id, object))
            .collect::<Vec<_>>());
        let start = resource.len();
        resource.extend_from_slice(&toc);
        let end = resource.len();
        dictionary.push(DictionaryEntry {
            resource_index: 0,
            object_offset: u32::try_from(start).unwrap(),
            relocation_offset: 0,
            size: u32::try_from(toc.len()).unwrap(),
            alignment: 1,
            type_index: 0,
            type_id: TABLE_OF_CONTENTS_TYPE_ID,
            object_range: Range { start, end },
        });
        let descriptor = ResourceDescriptor {
            size: 0,
            alignment: 1,
        };
        let entry_count = u32::try_from(dictionary.len()).unwrap();
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
            id: 1,
            entry_count,
            used_entry_count: entry_count,
            alignment: 1,
            virtual_size: u32::try_from(resource.len()).unwrap(),
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: [descriptor; 5],
            resources_used: [descriptor; 5],
            resource_offsets: [0; 5],
            arena_group_offset: 0,
            dictionary,
        };
        let collision = SimulationCollisionAsset {
            decoded: DecodedArenaAsset::from_parts_for_test(
                2,
                WORLD_STREAM_ARENA_ASSET_TYPE,
                ArenaResources {
                    resources: [resource, Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                },
                arena,
            ),
            collision: CollisionArena {
                id: 1,
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
        let normal = Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap());
        let plan = Arc::new(LocationArenaPlan::from_normal_plan(normal).unwrap());
        PreparedLocationArena::try_new(plan).unwrap()
    }

    fn runtime_for(prepared: &PreparedLocationArena, priorities: &[u8]) -> Runtime {
        let sources = (0..prepared.container_count())
            .map(|index| {
                (
                    prepared.operation(index).unwrap().asset_id,
                    prepared.planned_source(index).unwrap(),
                )
            })
            .collect();
        Runtime {
            sources,
            priorities: priorities.iter().copied().collect(),
            ..Runtime::default()
        }
    }

    #[test]
    fn resolves_forward_skips_only_null_names_and_samples_raw_priority_per_descriptor() {
        let first_id = 0x1111_2222_3333_4444;
        let second_id = 0xaaaa_bbbb_3333_4444;
        let prepared = make_prepared(&[
            (
                first_id,
                vec![Some("z"), None, Some(""), Some("a\0ignored")],
            ),
            (second_id, vec![]),
        ]);
        let mut adapter =
            LocationArenaRegistrationAdapter::new(runtime_for(&prepared, &[0, 0x80, 0xff]));
        let token = adapter.register(&prepared).unwrap();
        assert_eq!(adapter.runtime().resolutions, [first_id, second_id]);
        assert_eq!(adapter.runtime().priority_samples, 3);
        assert_eq!(adapter.entries().len(), 3);
        let by_index = adapter
            .entries()
            .iter()
            .map(|entry| {
                (
                    entry.descriptor().location_index(),
                    (
                        entry.name_hash(),
                        entry.high_priority_raw(),
                        entry.streamed_location(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(by_index[&0], (sk8_hash_value_32(b"z"), 0, true));
        assert_eq!(by_index[&2], (0, 0x80, true));
        assert_eq!(by_index[&3], (sk8_hash_value_32(b"a"), 0xff, true));
        adapter.deregister(token).unwrap();
        assert_eq!(
            adapter.runtime().resolutions,
            [first_id, second_id, first_id, second_id]
        );
        assert_eq!(adapter.runtime().priority_samples, 3);
        assert!(adapter.entries().is_empty());
        assert_eq!(adapter.live_registration_count(), 0);
    }

    #[test]
    fn repeated_same_pointer_removes_the_oldest_equal_entry_not_the_token_entry() {
        let prepared = make_prepared(&[(7, vec![Some("same")])]);
        let mut adapter =
            LocationArenaRegistrationAdapter::new(runtime_for(&prepared, &[0x11, 0x22]));
        let first = adapter.register(&prepared).unwrap();
        let second = adapter.register(&prepared).unwrap();
        assert_eq!(
            adapter
                .entries()
                .iter()
                .map(StreamedLocationEntry::high_priority_raw)
                .collect::<Vec<_>>(),
            [0x11, 0x22]
        );
        adapter.deregister(second).unwrap();
        assert_eq!(adapter.entries()[0].high_priority_raw(), 0x22);
        adapter.deregister(first).unwrap();
        assert!(adapter.entries().is_empty());
    }

    #[test]
    fn fresh_equal_bytes_at_a_different_address_cannot_remove_the_old_entry() {
        let id = 9;
        let original = make_prepared(&[(id, vec![Some("same")])]);
        let replacement = make_prepared(&[(id, vec![Some("same")])]);
        let original_record = original
            .planned_source(0)
            .unwrap()
            .location_handle(0)
            .unwrap();
        let mut adapter = LocationArenaRegistrationAdapter::new(runtime_for(&original, &[1]));
        let token = adapter.register(&original).unwrap();
        adapter.runtime_mut().sources.insert(
            id,
            replacement.planned_source(0).expect("replacement source"),
        );
        adapter.deregister(token).unwrap();
        assert_eq!(adapter.entries().len(), 1);
        assert!(
            adapter.entries()[0]
                .descriptor()
                .is_same_backing_record(&original_record)
        );
        assert_eq!(adapter.live_registration_count(), 0);
    }

    #[test]
    fn equal_hash_different_records_keep_stable_order_and_remove_exact_pointers() {
        let prepared = make_prepared(&[(5, vec![Some("duplicate"), Some("duplicate")])]);
        let mut adapter = LocationArenaRegistrationAdapter::new(runtime_for(&prepared, &[1, 2]));
        let token = adapter.register(&prepared).unwrap();
        assert_eq!(
            adapter
                .entries()
                .iter()
                .map(|entry| entry.descriptor().location_index())
                .collect::<Vec<_>>(),
            [0, 1]
        );
        adapter.deregister(token).unwrap();
        assert!(adapter.entries().is_empty());
    }

    #[test]
    fn missing_source_and_priority_panics_latch_exact_nonrollback_prefixes() {
        let prepared = make_prepared(&[(5, vec![Some("first"), Some("second")])]);
        let mut missing = LocationArenaRegistrationAdapter::new(Runtime::default());
        let fault = match missing.register(&prepared) {
            Err(LocationRegistrationError::RuntimeFault(fault)) => fault,
            other => panic!("unexpected result: {other:?}"),
        };
        assert_eq!(fault.phase(), LocationAdapterFaultPhase::Registration);
        assert_eq!(fault.operation_index(), 0);
        assert_eq!(fault.descriptor_index(), None);
        assert_eq!(
            fault.stage(),
            LocationAdapterFaultStage::MissingRegistrySource
        );
        assert_eq!(missing.live_registration_count(), 1);
        assert!(matches!(
            missing.register(&prepared),
            Err(LocationRegistrationError::AdapterFaulted(value)) if value == fault
        ));

        let mut runtime = runtime_for(&prepared, &[3, 4]);
        runtime.panic_priority_at = Some(1);
        let mut panicking = LocationArenaRegistrationAdapter::new(runtime);
        let fault = match panicking.register(&prepared) {
            Err(LocationRegistrationError::RuntimeFault(fault)) => fault,
            other => panic!("unexpected result: {other:?}"),
        };
        assert_eq!(fault.descriptor_index(), Some(1));
        assert_eq!(fault.stage(), LocationAdapterFaultStage::HighPriority);
        assert_eq!(panicking.entries().len(), 1);
        assert_eq!(panicking.live_registration_count(), 1);
    }

    #[test]
    fn deregistration_resolve_panic_consumes_token_and_keeps_live_owner() {
        let prepared = make_prepared(&[(5, vec![Some("only")])]);
        let mut adapter = LocationArenaRegistrationAdapter::new(runtime_for(&prepared, &[1]));
        let token = adapter.register(&prepared).unwrap();
        adapter.runtime_mut().panic_resolve_at = Some(1);
        let fault = match adapter.deregister(token) {
            Err(LocationDeregistrationError::RuntimeFault(fault)) => fault,
            other => panic!("unexpected result: {other:?}"),
        };
        assert_eq!(fault.phase(), LocationAdapterFaultPhase::Deregistration);
        assert_eq!(fault.stage(), LocationAdapterFaultStage::RegistryResolve);
        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.entries().len(), 1);
    }

    #[test]
    fn foreign_token_is_returned_without_effects_then_remains_valid_at_owner() {
        let prepared = make_prepared(&[(5, vec![Some("only")])]);
        let mut owner = LocationArenaRegistrationAdapter::new(runtime_for(&prepared, &[1]));
        let mut foreign = LocationArenaRegistrationAdapter::new(runtime_for(&prepared, &[]));
        let token = owner.register(&prepared).unwrap();
        let error = foreign.deregister(token).unwrap_err();
        assert!(foreign.runtime().resolutions.is_empty());
        let token = error.into_registration().expect("returned token");
        owner.deregister(token).unwrap();
        assert!(owner.entries().is_empty());
    }

    #[test]
    fn streamed_slice_does_not_turn_native_inline_capacity_into_a_hard_limit() {
        let names = (0..1_025)
            .map(|index| Some(format!("location-{index}")))
            .collect::<Vec<_>>();
        let borrowed = names.iter().map(|name| name.as_deref()).collect::<Vec<_>>();
        let prepared = make_prepared(&[(5, borrowed)]);
        let mut adapter = LocationArenaRegistrationAdapter::new(runtime_for(&prepared, &[]));
        let token = adapter.register(&prepared).unwrap();
        assert_eq!(adapter.entries().len(), 1_025);
        adapter.deregister(token).unwrap();
        assert!(adapter.entries().is_empty());
    }

    #[test]
    fn dropped_token_is_inert_and_adapter_retains_the_complete_plan() {
        let prepared = make_prepared(&[(5, vec![Some("only")])]);
        let weak = Arc::downgrade(&prepared.inner.plan);
        let mut adapter = LocationArenaRegistrationAdapter::new(runtime_for(&prepared, &[1]));
        let token = adapter.register(&prepared).unwrap();
        drop(token);
        drop(prepared);
        assert!(weak.upgrade().is_some());
        drop(adapter);
        assert!(weak.upgrade().is_none());
    }
}
