//! Checked 32-bit address image of the native `Simulation` allocation.
//!
//! `Simulation::Initialize` lays out the RF rows, four intrusive-list anchors,
//! usable rigid bodies, Joints, and Drives inside one allocation. This module
//! reproduces only that proven address arithmetic. The absolute base is
//! supplied by the caller; accepting it does not claim parity with the native
//! allocator's placement or with any particular console run.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the SerialisedAssembly Xenon packed-word provider"
    )
)]

use crate::{
    serialised_assembly_template::{
        SerialisedAssemblyPartInertiaSource, SerialisedAssemblyRuntimeInertia,
    },
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_rigid_body_pool::{
        SimulationRigidBodyHandle, SimulationRigidBodyId, SimulationRigidBodyList,
        SimulationRigidBodyNode, SimulationRigidBodyObservation,
        SimulationRigidBodyPackedWordTopologySnapshot, SimulationRigidBodyPool,
        SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError,
    },
};
use std::{
    error::Error,
    fmt,
    num::NonZeroU64,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

const SIMULATION_ALLOCATION_ALIGNMENT: u32 = 0x80;
const SIMULATION_HEADER_SIZE: u32 = 0x300;
const SOLVER_DELTA_ROW_ALIGNMENT: u32 = 0x40;
const SOLVER_DELTA_ROW_STRIDE: u32 = 0x40;
const RIGID_BODY_ALIGNMENT: u32 = 0x10;
const RIGID_BODY_STRIDE: u32 = 0xb0;
const RIGID_BODY_ANCHOR_COUNT: u32 = 4;
const JOINT_DRIVE_ALIGNMENT: u32 = 4;
const JOINT_DRIVE_STRIDE: u32 = 0x20;
const JOINT_SENTINEL_COUNT: u32 = 2;
const DRIVE_SENTINEL_COUNT: u32 = 2;
const MANAGER_DESCRIPTOR_HEADER_ROW_COUNT: u32 = 12;
const PART_INERTIA_ALIGNMENT: u32 = 0x10;
const PART_INERTIA_SIZE: u32 = 0x30;
/// Process-local safe-host binding generation; native epoch storage is not claimed.
static NEXT_ARENA_BINDING_EPOCH: AtomicU64 = AtomicU64::new(1);
/// Process-local safe-host Simulation address-image generation.
static NEXT_SIMULATION_IMAGE_EPOCH: AtomicU64 = AtomicU64::new(1);

/// Safe-host refusal while binding caller-asserted Xenon arena bases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationArenaXenonAddressBindingError {
    BindingEpochExhausted,
    UnsupportedArenaBaseOffset {
        base_offset: u32,
    },
    ResourceSizeMismatch {
        resource_index: usize,
        descriptor_size: u32,
        retained_size: usize,
    },
    InvalidResourceAlignment {
        resource_index: usize,
        alignment: u32,
    },
    NullNonemptyResourceBase {
        resource_index: usize,
    },
    MisalignedResourceBase {
        resource_index: usize,
        base: u32,
        required_alignment: u32,
    },
    ResourceAddressOverflow {
        resource_index: usize,
    },
    ForeignInertiaOwner,
    InertiaResourceUnavailable {
        resource_index: usize,
    },
    InertiaAddressOverflow,
    InertiaOutsideResource {
        resource_index: usize,
        resource_offset: u32,
    },
    MisalignedInertiaAddress {
        address: u32,
        required_alignment: u32,
    },
}

impl fmt::Display for SimulationArenaXenonAddressBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BindingEpochExhausted => {
                formatter.write_str("Xenon arena binding epoch space is exhausted")
            }
            Self::UnsupportedArenaBaseOffset { base_offset } => write!(
                formatter,
                "RW4 arena old base {base_offset:#010x} is not supported by the resource-relative binder"
            ),
            Self::ResourceSizeMismatch {
                resource_index,
                descriptor_size,
                retained_size,
            } => write!(
                formatter,
                "Xenon arena resource {resource_index} descriptor size {descriptor_size} differs from retained size {retained_size}"
            ),
            Self::InvalidResourceAlignment {
                resource_index,
                alignment,
            } => write!(
                formatter,
                "Xenon arena resource {resource_index} has invalid alignment {alignment:#x}"
            ),
            Self::NullNonemptyResourceBase { resource_index } => write!(
                formatter,
                "nonempty Xenon arena resource {resource_index} has a null base"
            ),
            Self::MisalignedResourceBase {
                resource_index,
                base,
                required_alignment,
            } => write!(
                formatter,
                "Xenon arena resource {resource_index} base {base:#010x} is not aligned to {required_alignment:#x}"
            ),
            Self::ResourceAddressOverflow { resource_index } => write!(
                formatter,
                "Xenon arena resource {resource_index} address span overflows"
            ),
            Self::ForeignInertiaOwner => {
                formatter.write_str("Part Inertia source belongs to another arena owner")
            }
            Self::InertiaResourceUnavailable { resource_index } => write!(
                formatter,
                "Part Inertia selects unavailable Xenon arena resource {resource_index}"
            ),
            Self::InertiaAddressOverflow => {
                formatter.write_str("Part Inertia Xenon address span overflows")
            }
            Self::InertiaOutsideResource {
                resource_index,
                resource_offset,
            } => write!(
                formatter,
                "Part Inertia at resource {resource_index} offset {resource_offset:#x} exceeds the Xenon allocation"
            ),
            Self::MisalignedInertiaAddress {
                address,
                required_alignment,
            } => write!(
                formatter,
                "Part Inertia Xenon address {address:#010x} is not aligned to {required_alignment:#x}"
            ),
        }
    }
}

impl Error for SimulationArenaXenonAddressBindingError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationArenaXenonResourceSpan {
    base: u32,
    end: u32,
}

/// One arena owner's caller-supplied five-base Xenon address binding.
///
/// `CreateCompressedArenaAssetData` (`0x82d61fe0..0x82d621fc`) forwards the
/// XST size/alignment pairs to `DoAllocate`; `Arena::FixupStart`
/// (`0x82bc1e18..0x82bc1fb4`) receives the resulting five bases. This value
/// requires bases asserted to be allocator results; it cannot prove that
/// provenance or own the allocation lease. It never derives bases from host
/// buffers, embedded Arena descriptors, or serialized `Arena +0x94` words.
pub(crate) struct SimulationArenaXenonAddressBinding {
    owner: Arc<SimulationNormalArenaPlan>,
    binding_epoch: NonZeroU64,
    resource_bases: [u32; 5],
    resource_spans: [Option<SimulationArenaXenonResourceSpan>; 5],
}

impl fmt::Debug for SimulationArenaXenonAddressBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationArenaXenonAddressBinding")
            .field("binding_epoch", &self.binding_epoch)
            .finish_non_exhaustive()
    }
}

impl SimulationArenaXenonAddressBinding {
    /// Validates all five explicit bases before minting a unique host binding epoch.
    ///
    /// Nonzero/power-of-two alignment, checked arithmetic, retained-size
    /// agreement, and nonzero binding identity are safe-host publication
    /// policy. Native allocation code assumes its inputs and 32-bit arithmetic.
    pub(crate) fn try_new(
        owner: Arc<SimulationNormalArenaPlan>,
        resource_bases: [u32; 5],
    ) -> Result<Self, SimulationArenaXenonAddressBindingError> {
        let decoded = &owner.static_plan().asset().decoded;
        if decoded.arena.base_offset != 0 {
            return Err(
                SimulationArenaXenonAddressBindingError::UnsupportedArenaBaseOffset {
                    base_offset: decoded.arena.base_offset,
                },
            );
        }
        let mut resource_spans = [None; 5];
        for (resource_index, ((descriptor, retained), base)) in decoded
            .asset_resource_descriptors
            .iter()
            .zip(&decoded.resources.resources)
            .zip(resource_bases)
            .enumerate()
        {
            if usize::try_from(descriptor.size).ok() != Some(retained.len()) {
                return Err(
                    SimulationArenaXenonAddressBindingError::ResourceSizeMismatch {
                        resource_index,
                        descriptor_size: descriptor.size,
                        retained_size: retained.len(),
                    },
                );
            }
            if descriptor.size == 0 {
                continue;
            }
            if !descriptor.alignment.is_power_of_two() {
                return Err(
                    SimulationArenaXenonAddressBindingError::InvalidResourceAlignment {
                        resource_index,
                        alignment: descriptor.alignment,
                    },
                );
            }
            if base == 0 {
                return Err(
                    SimulationArenaXenonAddressBindingError::NullNonemptyResourceBase {
                        resource_index,
                    },
                );
            }
            if base & (descriptor.alignment - 1) != 0 {
                return Err(
                    SimulationArenaXenonAddressBindingError::MisalignedResourceBase {
                        resource_index,
                        base,
                        required_alignment: descriptor.alignment,
                    },
                );
            }
            let end = base.checked_add(descriptor.size).ok_or(
                SimulationArenaXenonAddressBindingError::ResourceAddressOverflow { resource_index },
            )?;
            resource_spans[resource_index] = Some(SimulationArenaXenonResourceSpan { base, end });
        }
        let binding_epoch = next_arena_binding_epoch()?;

        Ok(Self {
            owner,
            binding_epoch,
            resource_bases,
            resource_spans,
        })
    }

    /// Resolves a Definition `Inertia` as a non-Static address candidate.
    ///
    /// This does not prove which rigid body retained the address and therefore
    /// does not issue a complete packed-body-word provider.
    pub(crate) fn try_bind_nonstatic_inertia_source<'binding>(
        &'binding self,
        source: &SerialisedAssemblyPartInertiaSource,
    ) -> Result<BoundPartInertiaAddress<'binding>, SimulationArenaXenonAddressBindingError> {
        if !Arc::ptr_eq(&self.owner, source.owner()) {
            return Err(SimulationArenaXenonAddressBindingError::ForeignInertiaOwner);
        }
        let resource_index = source.resource_index();
        let span = self
            .resource_spans
            .get(resource_index)
            .copied()
            .flatten()
            .ok_or(
                SimulationArenaXenonAddressBindingError::InertiaResourceUnavailable {
                    resource_index,
                },
            )?;
        let relative_end = source
            .resource_offset()
            .checked_add(PART_INERTIA_SIZE)
            .ok_or(SimulationArenaXenonAddressBindingError::InertiaAddressOverflow)?;
        if relative_end > span.end - span.base {
            return Err(
                SimulationArenaXenonAddressBindingError::InertiaOutsideResource {
                    resource_index,
                    resource_offset: source.resource_offset(),
                },
            );
        }
        let address = span
            .base
            .checked_add(source.resource_offset())
            .ok_or(SimulationArenaXenonAddressBindingError::InertiaAddressOverflow)?;
        let inertia_end = address
            .checked_add(PART_INERTIA_SIZE)
            .ok_or(SimulationArenaXenonAddressBindingError::InertiaAddressOverflow)?;
        if inertia_end > span.end {
            return Err(
                SimulationArenaXenonAddressBindingError::InertiaOutsideResource {
                    resource_index,
                    resource_offset: source.resource_offset(),
                },
            );
        }
        if address & (PART_INERTIA_ALIGNMENT - 1) != 0 {
            return Err(
                SimulationArenaXenonAddressBindingError::MisalignedInertiaAddress {
                    address,
                    required_alignment: PART_INERTIA_ALIGNMENT,
                },
            );
        }
        Ok(BoundPartInertiaAddress {
            binding: self,
            resource_index,
            resource_offset: source.resource_offset(),
            address,
        })
    }
}

/// Owner- and host-binding-generation-qualified non-Static Inertia address.
///
/// The borrow prevents this typed candidate from surviving the staged binding.
/// It is not the final body `mInertia` word: Static selection and exact live
/// Part/body occurrence ownership remain outside this value. Its diagnostic
/// scalar accessor does not itself carry lifetime or packed-word authority.
pub(crate) struct BoundPartInertiaAddress<'binding> {
    binding: &'binding SimulationArenaXenonAddressBinding,
    resource_index: usize,
    resource_offset: u32,
    address: u32,
}

impl fmt::Debug for BoundPartInertiaAddress<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundPartInertiaAddress")
            .field("binding_epoch", &self.binding.binding_epoch)
            .field("resource_index", &self.resource_index)
            .field("resource_offset", &self.resource_offset)
            .field("address", &format_args!("{:#010x}", self.address))
            .finish_non_exhaustive()
    }
}

impl BoundPartInertiaAddress<'_> {
    pub(crate) const fn address(&self) -> u32 {
        self.address
    }

    pub(crate) const fn binding_epoch(&self) -> u64 {
        self.binding.binding_epoch.get()
    }

    pub(crate) fn is_same_binding(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.binding.owner, &other.binding.owner)
            && self.binding.binding_epoch == other.binding.binding_epoch
    }

    pub(crate) fn is_same_backing_inertia(&self, other: &Self) -> bool {
        self.is_same_binding(other)
            && self.resource_index == other.resource_index
            && self.resource_offset == other.resource_offset
    }
}

/// Semantic Inertia input paired with its optional native address provenance.
///
/// `Definition` is the source-aware `Part` path. `Null` preserves the generic
/// `AddRigidBody` case in which Frozen/Active receives a null pointer.
#[derive(Clone, Copy)]
pub(crate) enum SimulationRigidBodyAddInertiaInput<'input> {
    Null,
    Definition(&'input SerialisedAssemblyRuntimeInertia),
}

/// Exact Add-selected native `mInertia` outcome for one live occurrence.
///
/// Absence is represented by no table entry, not by either null variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationRigidBodyAddInertiaOutcome<'outcome> {
    StaticNull,
    NonStaticNull,
    Definition {
        address: u32,
        source: &'outcome SerialisedAssemblyPartInertiaSource,
    },
}

#[derive(Debug)]
enum SimulationRigidBodyAddInertiaRecordOutcome {
    StaticNull,
    NonStaticNull,
    Definition {
        address: u32,
        inertia: SerialisedAssemblyRuntimeInertia,
    },
}

#[derive(Debug)]
struct SimulationRigidBodyAddInertiaRecord {
    observation: SimulationRigidBodyObservation,
    outcome: SimulationRigidBodyAddInertiaRecordOutcome,
}

/// Safe-host refusal from the occurrence-qualified Add-time Inertia owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationRigidBodyAddInertiaOwnerError {
    StorageAllocation {
        capacity: usize,
    },
    ForeignBodyPool,
    Pool(SimulationRigidBodyPoolError),
    Binding(SimulationArenaXenonAddressBindingError),
    OccupiedSlot {
        slot: u32,
    },
    MissingOccurrenceOutcome {
        slot: u32,
    },
    RetainedInertiaMismatch {
        slot: u32,
        expected: bool,
        actual: bool,
    },
}

impl fmt::Display for SimulationRigidBodyAddInertiaOwnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageAllocation { capacity } => write!(
                formatter,
                "could not allocate {capacity} rigid-body Inertia outcome slots"
            ),
            Self::ForeignBodyPool => {
                formatter.write_str("rigid-body Inertia outcomes belong to another pool")
            }
            Self::Pool(source) => write!(formatter, "rigid-body pool refused operation: {source}"),
            Self::Binding(source) => write!(formatter, "Inertia source binding failed: {source}"),
            Self::OccupiedSlot { slot } => write!(
                formatter,
                "rigid-body slot {slot} still has an Inertia outcome record"
            ),
            Self::MissingOccurrenceOutcome { slot } => write!(
                formatter,
                "rigid-body slot {slot} has no matching Inertia outcome occurrence"
            ),
            Self::RetainedInertiaMismatch {
                slot,
                expected,
                actual,
            } => write!(
                formatter,
                "rigid-body slot {slot} retained-Inertia presence {actual} differs from Add outcome {expected}"
            ),
        }
    }
}

impl Error for SimulationRigidBodyAddInertiaOwnerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Pool(source) => Some(source),
            Self::Binding(source) => Some(source),
            _ => None,
        }
    }
}

/// Preallocated Add-time `mInertia` authority for one body pool and arena binding.
///
/// The table owns the exact binding generation and records complete host body
/// occurrences. It does not model a native sidecar: the table, recoverable
/// errors, and ABA occurrences are safe-host authority policy. A Definition
/// record retains its source owner and can only be minted by resolving that
/// source through this table's binding before body effects.
pub(crate) struct SimulationRigidBodyAddInertiaOwner {
    binding: Arc<SimulationArenaXenonAddressBinding>,
    pool_authority: SimulationRigidBodyPoolAuthority,
    entries: Box<[Option<SimulationRigidBodyAddInertiaRecord>]>,
}

impl SimulationRigidBodyAddInertiaOwner {
    pub(crate) fn try_new(
        pool: &SimulationRigidBodyPool,
        binding: Arc<SimulationArenaXenonAddressBinding>,
    ) -> Result<Self, SimulationRigidBodyAddInertiaOwnerError> {
        let capacity = pool.slot_capacity();
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationRigidBodyAddInertiaOwnerError::StorageAllocation { capacity })?;
        if entries.capacity() < capacity {
            return Err(SimulationRigidBodyAddInertiaOwnerError::StorageAllocation { capacity });
        }
        entries.resize_with(capacity, || None);
        Ok(Self {
            binding,
            pool_authority: pool.authority(),
            entries: entries.into_boxed_slice(),
        })
    }

    /// Adds one body and publishes its exact Add-selected Inertia outcome.
    ///
    /// Empty Free returns before the input source is bound. For a nonempty
    /// pool, Definition binding and a vacant preallocated sidecar cell are
    /// proven before body mutation. Static never binds or validates its input
    /// source and records the zero forced by the native Static branch.
    pub(crate) fn try_add(
        &mut self,
        pool: &mut SimulationRigidBodyPool,
        matrix: [f32; 16],
        input: SimulationRigidBodyAddInertiaInput<'_>,
        raw_state: u32,
    ) -> Result<Option<SimulationRigidBodyHandle>, SimulationRigidBodyAddInertiaOwnerError> {
        self.validate_pool(pool)?;
        let Some(reservation) = pool
            .try_prepare_add_rigid_body(raw_state)
            .map_err(SimulationRigidBodyAddInertiaOwnerError::Pool)?
        else {
            return Ok(None);
        };
        let slot = reservation.id().index();
        let entry = self
            .entries
            .get(usize::try_from(slot).expect("u32 body slot fits host usize"))
            .expect("reservation slot fits its pool capacity");
        if entry.is_some() {
            return Err(SimulationRigidBodyAddInertiaOwnerError::OccupiedSlot { slot });
        }

        let semantic = match &input {
            SimulationRigidBodyAddInertiaInput::Null => None,
            SimulationRigidBodyAddInertiaInput::Definition(inertia) => Some(inertia.semantic()),
        };
        let outcome = if reservation.is_static() {
            SimulationRigidBodyAddInertiaRecordOutcome::StaticNull
        } else {
            match input {
                SimulationRigidBodyAddInertiaInput::Null => {
                    SimulationRigidBodyAddInertiaRecordOutcome::NonStaticNull
                }
                SimulationRigidBodyAddInertiaInput::Definition(inertia) => {
                    let source = inertia.source();
                    let address = self
                        .binding
                        .try_bind_nonstatic_inertia_source(source)
                        .map_err(SimulationRigidBodyAddInertiaOwnerError::Binding)?
                        .address();
                    SimulationRigidBodyAddInertiaRecordOutcome::Definition {
                        address,
                        inertia: inertia.clone(),
                    }
                }
            }
        };

        let handle = reservation.commit(matrix, semantic);
        let index = usize::try_from(handle.id().index()).expect("u32 body slot fits host usize");
        debug_assert!(self.entries[index].is_none());
        self.entries[index] = Some(SimulationRigidBodyAddInertiaRecord {
            observation: handle.observation(),
            outcome,
        });
        Ok(Some(handle))
    }

    /// Removes one registered body, then retires its host authority record.
    ///
    /// Native Remove preserves the stale pointer bits while Free. Clearing the
    /// table entry after successful pool mutation retires only safe-host packed
    /// word authority; it is not represented as a native body-field write.
    pub(crate) fn try_remove(
        &mut self,
        pool: &mut SimulationRigidBodyPool,
        handle: SimulationRigidBodyHandle,
    ) -> Result<(), SimulationRigidBodyAddInertiaOwnerError> {
        self.validate_pool(pool)?;
        pool.try_validate_observation(handle.observation())
            .map_err(SimulationRigidBodyAddInertiaOwnerError::Pool)?;
        let index = usize::try_from(handle.id().index()).expect("u32 body slot fits host usize");
        let slot = handle.id().index();
        if self
            .entries
            .get(index)
            .and_then(Option::as_ref)
            .is_none_or(|entry| entry.observation != handle.observation())
        {
            return Err(SimulationRigidBodyAddInertiaOwnerError::MissingOccurrenceOutcome { slot });
        }
        pool.try_remove_rigid_body(handle)
            .map_err(SimulationRigidBodyAddInertiaOwnerError::Pool)?;
        let retired = self.entries[index]
            .take()
            .expect("validated occurrence has an outcome record");
        drop(retired);
        Ok(())
    }

    /// Resolves the immutable Add-time outcome after revalidating live topology.
    pub(crate) fn try_outcome(
        &self,
        pool: &SimulationRigidBodyPool,
        observation: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodyAddInertiaOutcome<'_>, SimulationRigidBodyAddInertiaOwnerError>
    {
        self.try_packed_word_inputs(pool, observation)
            .map(|(_, outcome)| outcome)
    }

    /// Resolves one occurrence-consistent live topology and Add-time Inertia
    /// outcome. The immutable pool and owner borrows keep both inputs stable
    /// while the private packed-word provider projects their Xenon addresses.
    pub(crate) fn try_packed_word_inputs(
        &self,
        pool: &SimulationRigidBodyPool,
        observation: SimulationRigidBodyObservation,
    ) -> Result<
        (
            SimulationRigidBodyPackedWordTopologySnapshot,
            SimulationRigidBodyAddInertiaOutcome<'_>,
        ),
        SimulationRigidBodyAddInertiaOwnerError,
    > {
        self.validate_pool(pool)?;
        let topology = pool
            .try_packed_word_topology_snapshot(observation)
            .map_err(SimulationRigidBodyAddInertiaOwnerError::Pool)?;
        let slot = observation.identity().id().index();
        let index = usize::try_from(slot).expect("u32 body slot fits host usize");
        let record = self
            .entries
            .get(index)
            .and_then(Option::as_ref)
            .filter(|entry| entry.observation == observation)
            .ok_or(SimulationRigidBodyAddInertiaOwnerError::MissingOccurrenceOutcome { slot })?;
        let (expected, outcome) = match &record.outcome {
            SimulationRigidBodyAddInertiaRecordOutcome::StaticNull => {
                (false, SimulationRigidBodyAddInertiaOutcome::StaticNull)
            }
            SimulationRigidBodyAddInertiaRecordOutcome::NonStaticNull => {
                (false, SimulationRigidBodyAddInertiaOutcome::NonStaticNull)
            }
            SimulationRigidBodyAddInertiaRecordOutcome::Definition { address, inertia } => (
                true,
                SimulationRigidBodyAddInertiaOutcome::Definition {
                    address: *address,
                    source: inertia.source(),
                },
            ),
        };
        if topology.has_retained_inertia != expected {
            return Err(
                SimulationRigidBodyAddInertiaOwnerError::RetainedInertiaMismatch {
                    slot,
                    expected,
                    actual: topology.has_retained_inertia,
                },
            );
        }
        Ok((topology, outcome))
    }

    pub(crate) fn is_same_pool(&self, pool: &SimulationRigidBodyPool) -> bool {
        self.pool_authority == pool.authority()
    }

    pub(crate) fn is_same_arena_owner(&self, owner: &Arc<SimulationNormalArenaPlan>) -> bool {
        Arc::ptr_eq(&self.binding.owner, owner)
    }

    pub(crate) fn binding_epoch(&self) -> NonZeroU64 {
        self.binding.binding_epoch
    }

    fn validate_pool(
        &self,
        pool: &SimulationRigidBodyPool,
    ) -> Result<(), SimulationRigidBodyAddInertiaOwnerError> {
        if pool.authority() == self.pool_authority {
            Ok(())
        } else {
            Err(SimulationRigidBodyAddInertiaOwnerError::ForeignBodyPool)
        }
    }
}

fn next_arena_binding_epoch() -> Result<NonZeroU64, SimulationArenaXenonAddressBindingError> {
    let epoch = NEXT_ARENA_BINDING_EPOCH
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| SimulationArenaXenonAddressBindingError::BindingEpochExhausted)?;
    NonZeroU64::new(epoch).ok_or(SimulationArenaXenonAddressBindingError::BindingEpochExhausted)
}

fn next_simulation_image_epoch() -> Result<NonZeroU64, SimulationPackedBodyWordImageDomainError> {
    let epoch = NEXT_SIMULATION_IMAGE_EPOCH
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| SimulationPackedBodyWordImageDomainError::ImageEpochExhausted)?;
    NonZeroU64::new(epoch).ok_or(SimulationPackedBodyWordImageDomainError::ImageEpochExhausted)
}

/// Exact checked-arithmetic stage which could not be represented in the
/// 32-bit Xenon address domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationXenonAddressStep {
    ManagerRigidBodyCapacity,
    DescriptorHeaderRowCount,
    DescriptorHeaderAndRows,
    DescriptorRigidBodyAllocationCount,
    DescriptorRigidBodyExtent,
    DescriptorAfterBodies,
    DescriptorJointAllocationCount,
    DescriptorJointExtent,
    DescriptorAfterJoints,
    DescriptorDriveAllocationCount,
    DescriptorDriveExtent,
    DescriptorAllocationSize,
    SolverDeltaRowBase,
    SolverDeltaRowExtent,
    RigidBodyBase,
    RigidBodyAllocationCount,
    RigidBodyExtent,
    RigidBodySlotBase,
    JointAllocationCount,
    JointExtent,
    JointBase,
    DriveAllocationCount,
    DriveExtent,
    DriveBase,
    AllocationEnd,
    RigidBodyAddress,
    SolverDeltaRowAddress,
}

/// Safe-host refusal while constructing or indexing a Xenon address image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationXenonAddressImageError {
    NullSimulationBase,
    MisalignedSimulationBase { base: u32, required_alignment: u32 },
    AddressOverflow { step: SimulationXenonAddressStep },
    RigidBodySlotOutOfRange { slot: u32, capacity: u32 },
}

impl fmt::Display for SimulationXenonAddressImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullSimulationBase => {
                formatter.write_str("Xenon Simulation base address is null")
            }
            Self::MisalignedSimulationBase {
                base,
                required_alignment,
            } => write!(
                formatter,
                "Xenon Simulation base {base:#010x} is not aligned to {required_alignment:#x}"
            ),
            Self::AddressOverflow { step } => {
                write!(formatter, "Xenon Simulation address overflow at {step:?}")
            }
            Self::RigidBodySlotOutOfRange { slot, capacity } => write!(
                formatter,
                "rigid-body slot {slot} is outside Xenon Simulation capacity {capacity}"
            ),
        }
    }
}

impl Error for SimulationXenonAddressImageError {}

/// Checked `SimulationMemory` request made by `SimulationManager`.
///
/// The wrapper deliberately asks `GetResourceDescriptor` for one more body
/// than it passes to `Simulation::Initialize`. For ordinary representable
/// inputs that makes this request exactly `0xf0` bytes larger than the
/// initialized image; the extra record is allocation slack, not capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationManagerXenonAllocationDescriptor {
    size: u32,
}

impl SimulationManagerXenonAllocationDescriptor {
    pub(crate) fn try_new(
        initialized_rigid_body_capacity: u32,
        joint_capacity: u32,
        drive_capacity: u32,
    ) -> Result<Self, SimulationXenonAddressImageError> {
        let descriptor_rigid_body_capacity = checked_add(
            initialized_rigid_body_capacity,
            1,
            SimulationXenonAddressStep::ManagerRigidBodyCapacity,
        )?;
        let header_row_count = checked_add(
            descriptor_rigid_body_capacity,
            MANAGER_DESCRIPTOR_HEADER_ROW_COUNT,
            SimulationXenonAddressStep::DescriptorHeaderRowCount,
        )?;
        let header_and_rows = checked_align_up(
            checked_mul(
                header_row_count,
                SOLVER_DELTA_ROW_STRIDE,
                SimulationXenonAddressStep::DescriptorHeaderAndRows,
            )?,
            RIGID_BODY_ALIGNMENT,
            SimulationXenonAddressStep::DescriptorHeaderAndRows,
        )?;
        let rigid_body_allocation_count = checked_add(
            descriptor_rigid_body_capacity,
            RIGID_BODY_ANCHOR_COUNT,
            SimulationXenonAddressStep::DescriptorRigidBodyAllocationCount,
        )?;
        let rigid_body_extent = checked_mul(
            rigid_body_allocation_count,
            RIGID_BODY_STRIDE,
            SimulationXenonAddressStep::DescriptorRigidBodyExtent,
        )?;
        let after_bodies = checked_align_up(
            checked_add(
                header_and_rows,
                rigid_body_extent,
                SimulationXenonAddressStep::DescriptorAfterBodies,
            )?,
            JOINT_DRIVE_ALIGNMENT,
            SimulationXenonAddressStep::DescriptorAfterBodies,
        )?;
        let joint_allocation_count = checked_add(
            joint_capacity,
            JOINT_SENTINEL_COUNT,
            SimulationXenonAddressStep::DescriptorJointAllocationCount,
        )?;
        let joint_extent = checked_mul(
            joint_allocation_count,
            JOINT_DRIVE_STRIDE,
            SimulationXenonAddressStep::DescriptorJointExtent,
        )?;
        let after_joints = checked_align_up(
            checked_add(
                after_bodies,
                joint_extent,
                SimulationXenonAddressStep::DescriptorAfterJoints,
            )?,
            JOINT_DRIVE_ALIGNMENT,
            SimulationXenonAddressStep::DescriptorAfterJoints,
        )?;
        let drive_allocation_count = checked_add(
            drive_capacity,
            DRIVE_SENTINEL_COUNT,
            SimulationXenonAddressStep::DescriptorDriveAllocationCount,
        )?;
        let drive_extent = checked_mul(
            drive_allocation_count,
            JOINT_DRIVE_STRIDE,
            SimulationXenonAddressStep::DescriptorDriveExtent,
        )?;
        let size = checked_add(
            after_joints,
            drive_extent,
            SimulationXenonAddressStep::DescriptorAllocationSize,
        )?;
        Ok(Self { size })
    }

    pub(crate) const fn size(self) -> u32 {
        self.size
    }

    pub(crate) const fn alignment() -> u32 {
        SIMULATION_ALLOCATION_ALIGNMENT
    }
}

/// Pure, checked address layout of one native `Simulation` allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationXenonAddressImage {
    simulation: u32,
    solver_delta_rows: u32,
    rigid_bodies: u32,
    rigid_body_slots: u32,
    joints: u32,
    drives: u32,
    end: u32,
    rigid_body_capacity: u32,
    joint_capacity: u32,
    drive_capacity: u32,
}

/// Safe-host refusal while binding a Simulation image generation to one body
/// pool. The native game has no image epoch or recoverable capacity check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationPackedBodyWordImageDomainError {
    CapacityMismatch { image: u32, pool: usize },
    ImageEpochExhausted,
}

impl fmt::Display for SimulationPackedBodyWordImageDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityMismatch { image, pool } => write!(
                formatter,
                "Simulation image has {image} rigid-body slots but its pool has {pool}"
            ),
            Self::ImageEpochExhausted => {
                formatter.write_str("Simulation image-domain epoch space is exhausted")
            }
        }
    }
}

impl Error for SimulationPackedBodyWordImageDomainError {}

/// One host-qualified generation of the global native `Simulation` address
/// image. It binds exact layout arithmetic to one nonreused body-pool owner.
///
/// The epoch prevents an equal-valued reconstructed image from authorizing a
/// prepared assembly. This does not yet own or prove the native allocator
/// lease behind the caller-supplied base.
pub(crate) struct SimulationPackedBodyWordImageDomain {
    image: SimulationXenonAddressImage,
    pool_authority: SimulationRigidBodyPoolAuthority,
    epoch: NonZeroU64,
}

impl SimulationPackedBodyWordImageDomain {
    pub(crate) fn try_new(
        pool: &SimulationRigidBodyPool,
        image: SimulationXenonAddressImage,
    ) -> Result<Self, SimulationPackedBodyWordImageDomainError> {
        let pool_capacity = pool.slot_capacity();
        if usize::try_from(image.rigid_body_capacity()).ok() != Some(pool_capacity) {
            return Err(SimulationPackedBodyWordImageDomainError::CapacityMismatch {
                image: image.rigid_body_capacity(),
                pool: pool_capacity,
            });
        }
        let epoch = next_simulation_image_epoch()?;
        Ok(Self {
            image,
            pool_authority: pool.authority(),
            epoch,
        })
    }

    pub(crate) const fn epoch(&self) -> NonZeroU64 {
        self.epoch
    }

    pub(crate) fn is_same_pool(&self, pool: &SimulationRigidBodyPool) -> bool {
        self.pool_authority == pool.authority()
    }

    pub(crate) const fn simulation_address(&self) -> u32 {
        self.image.simulation_address()
    }

    pub(crate) fn solver_delta_row_address(
        &self,
        body: SimulationRigidBodyId,
    ) -> Result<u32, SimulationXenonAddressImageError> {
        self.image.solver_delta_row_address(body)
    }

    pub(crate) fn rigid_body_node_address(
        &self,
        node: SimulationRigidBodyNode,
    ) -> Result<u32, SimulationXenonAddressImageError> {
        self.image.rigid_body_node_address(node)
    }
}

impl SimulationXenonAddressImage {
    /// Constructs the exact internal layout from a caller-supplied absolute
    /// Xenon base and the three native capacities.
    ///
    /// The native allocation has 0x80 alignment. All additions,
    /// multiplications, and inner alignment operations are checked rather than
    /// allowing malformed host input to wrap through the 32-bit address space.
    pub(crate) fn try_new(
        simulation: u32,
        rigid_body_capacity: u32,
        joint_capacity: u32,
        drive_capacity: u32,
    ) -> Result<Self, SimulationXenonAddressImageError> {
        if simulation == 0 {
            return Err(SimulationXenonAddressImageError::NullSimulationBase);
        }
        if simulation % SIMULATION_ALLOCATION_ALIGNMENT != 0 {
            return Err(SimulationXenonAddressImageError::MisalignedSimulationBase {
                base: simulation,
                required_alignment: SIMULATION_ALLOCATION_ALIGNMENT,
            });
        }

        let solver_delta_rows = checked_align_up(
            checked_add(
                simulation,
                SIMULATION_HEADER_SIZE,
                SimulationXenonAddressStep::SolverDeltaRowBase,
            )?,
            SOLVER_DELTA_ROW_ALIGNMENT,
            SimulationXenonAddressStep::SolverDeltaRowBase,
        )?;
        let solver_delta_row_extent = checked_mul(
            rigid_body_capacity,
            SOLVER_DELTA_ROW_STRIDE,
            SimulationXenonAddressStep::SolverDeltaRowExtent,
        )?;
        let rigid_bodies = checked_align_up(
            checked_add(
                solver_delta_rows,
                solver_delta_row_extent,
                SimulationXenonAddressStep::RigidBodyBase,
            )?,
            RIGID_BODY_ALIGNMENT,
            SimulationXenonAddressStep::RigidBodyBase,
        )?;

        let rigid_body_allocation_count = checked_add(
            rigid_body_capacity,
            RIGID_BODY_ANCHOR_COUNT,
            SimulationXenonAddressStep::RigidBodyAllocationCount,
        )?;
        let rigid_body_extent = checked_mul(
            rigid_body_allocation_count,
            RIGID_BODY_STRIDE,
            SimulationXenonAddressStep::RigidBodyExtent,
        )?;
        let rigid_body_slots = checked_add(
            rigid_bodies,
            RIGID_BODY_ANCHOR_COUNT * RIGID_BODY_STRIDE,
            SimulationXenonAddressStep::RigidBodySlotBase,
        )?;
        let joints = checked_align_up(
            checked_add(
                rigid_bodies,
                rigid_body_extent,
                SimulationXenonAddressStep::JointBase,
            )?,
            JOINT_DRIVE_ALIGNMENT,
            SimulationXenonAddressStep::JointBase,
        )?;

        let joint_allocation_count = checked_add(
            joint_capacity,
            JOINT_SENTINEL_COUNT,
            SimulationXenonAddressStep::JointAllocationCount,
        )?;
        let joint_extent = checked_mul(
            joint_allocation_count,
            JOINT_DRIVE_STRIDE,
            SimulationXenonAddressStep::JointExtent,
        )?;
        let drives = checked_align_up(
            checked_add(joints, joint_extent, SimulationXenonAddressStep::DriveBase)?,
            JOINT_DRIVE_ALIGNMENT,
            SimulationXenonAddressStep::DriveBase,
        )?;

        let drive_allocation_count = checked_add(
            drive_capacity,
            DRIVE_SENTINEL_COUNT,
            SimulationXenonAddressStep::DriveAllocationCount,
        )?;
        let drive_extent = checked_mul(
            drive_allocation_count,
            JOINT_DRIVE_STRIDE,
            SimulationXenonAddressStep::DriveExtent,
        )?;
        let end = checked_add(
            drives,
            drive_extent,
            SimulationXenonAddressStep::AllocationEnd,
        )?;

        Ok(Self {
            simulation,
            solver_delta_rows,
            rigid_bodies,
            rigid_body_slots,
            joints,
            drives,
            end,
            rigid_body_capacity,
            joint_capacity,
            drive_capacity,
        })
    }

    pub(crate) const fn simulation_address(self) -> u32 {
        self.simulation
    }

    pub(crate) const fn solver_delta_row_base_address(self) -> u32 {
        self.solver_delta_rows
    }

    pub(crate) const fn rigid_body_base_address(self) -> u32 {
        self.rigid_bodies
    }

    pub(crate) const fn rigid_body_slot_base_address(self) -> u32 {
        self.rigid_body_slots
    }

    pub(crate) const fn joint_base_address(self) -> u32 {
        self.joints
    }

    pub(crate) const fn drive_base_address(self) -> u32 {
        self.drives
    }

    /// Exclusive end address of the allocation image.
    pub(crate) const fn end_address(self) -> u32 {
        self.end
    }

    /// Initialized byte span stored by native `Simulation::Initialize`.
    pub(crate) const fn initialized_size(self) -> u32 {
        self.end - self.simulation
    }

    pub(crate) const fn rigid_body_capacity(self) -> u32 {
        self.rigid_body_capacity
    }

    pub(crate) const fn joint_capacity(self) -> u32 {
        self.joint_capacity
    }

    pub(crate) const fn drive_capacity(self) -> u32 {
        self.drive_capacity
    }

    pub(crate) const fn rigid_body_anchor_address(self, list: SimulationRigidBodyList) -> u32 {
        let ordinal = match list {
            SimulationRigidBodyList::Free => 0,
            SimulationRigidBodyList::Static => 1,
            SimulationRigidBodyList::Frozen => 2,
            SimulationRigidBodyList::Active => 3,
        };
        self.rigid_bodies + ordinal * RIGID_BODY_STRIDE
    }

    pub(crate) fn rigid_body_address(
        self,
        body: SimulationRigidBodyId,
    ) -> Result<u32, SimulationXenonAddressImageError> {
        let slot = body.index();
        self.validate_body_slot(slot)?;
        let offset = checked_mul(
            slot,
            RIGID_BODY_STRIDE,
            SimulationXenonAddressStep::RigidBodyAddress,
        )?;
        checked_add(
            self.rigid_body_slots,
            offset,
            SimulationXenonAddressStep::RigidBodyAddress,
        )
    }

    pub(crate) fn solver_delta_row_address(
        self,
        body: SimulationRigidBodyId,
    ) -> Result<u32, SimulationXenonAddressImageError> {
        let slot = body.index();
        self.validate_body_slot(slot)?;
        let offset = checked_mul(
            slot,
            SOLVER_DELTA_ROW_STRIDE,
            SimulationXenonAddressStep::SolverDeltaRowAddress,
        )?;
        checked_add(
            self.solver_delta_rows,
            offset,
            SimulationXenonAddressStep::SolverDeltaRowAddress,
        )
    }

    pub(crate) fn rigid_body_node_address(
        self,
        node: SimulationRigidBodyNode,
    ) -> Result<u32, SimulationXenonAddressImageError> {
        match node {
            SimulationRigidBodyNode::Anchor(list) => Ok(self.rigid_body_anchor_address(list)),
            SimulationRigidBodyNode::Body(body) => self.rigid_body_address(body),
        }
    }

    fn validate_body_slot(self, slot: u32) -> Result<(), SimulationXenonAddressImageError> {
        if slot < self.rigid_body_capacity {
            Ok(())
        } else {
            Err(SimulationXenonAddressImageError::RigidBodySlotOutOfRange {
                slot,
                capacity: self.rigid_body_capacity,
            })
        }
    }
}

fn checked_add(
    left: u32,
    right: u32,
    step: SimulationXenonAddressStep,
) -> Result<u32, SimulationXenonAddressImageError> {
    left.checked_add(right)
        .ok_or(SimulationXenonAddressImageError::AddressOverflow { step })
}

fn checked_mul(
    left: u32,
    right: u32,
    step: SimulationXenonAddressStep,
) -> Result<u32, SimulationXenonAddressImageError> {
    left.checked_mul(right)
        .ok_or(SimulationXenonAddressImageError::AddressOverflow { step })
}

fn checked_align_up(
    value: u32,
    alignment: u32,
    step: SimulationXenonAddressStep,
) -> Result<u32, SimulationXenonAddressImageError> {
    debug_assert!(alignment.is_power_of_two());
    let mask = alignment - 1;
    let remainder = value & mask;
    let adjustment = if remainder == 0 {
        0
    } else {
        alignment - remainder
    };
    checked_add(value, adjustment, step)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        serialised_assembly_template::{
            SerialisedAssemblyPartInertiaSource, SerialisedAssemblyRuntimeInertia,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyPool},
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::{CollisionArena, CollisionPrimitiveSet},
    };
    use skate2_formats::{
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::{ArenaResources, BaseResourceDescriptor},
    };

    fn body_ids(capacity: usize) -> Vec<SimulationRigidBodyId> {
        SimulationRigidBodyPool::try_new(capacity)
            .unwrap()
            .snapshot()
            .slots
            .into_iter()
            .map(|slot| slot.id)
            .collect()
    }

    fn arena_owner(
        base_offset: u32,
        resources: [Vec<u8>; 5],
        descriptors: [BaseResourceDescriptor; 5],
        embedded_alignments: [u32; 5],
        resource_offsets: [u32; 5],
    ) -> Arc<SimulationNormalArenaPlan> {
        let embedded_descriptors = std::array::from_fn(|index| ResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: embedded_alignments[index],
        });
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
            id: 0x8646_a1f7,
            entry_count: 0,
            used_entry_count: 0,
            alignment: 16,
            virtual_size: u32::try_from(resources[0].len()).unwrap(),
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: embedded_descriptors,
            resources_used: embedded_descriptors,
            resource_offsets,
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        let decoded = DecodedArenaAsset {
            asset_id: 0x0123_4567_89ab_cdef,
            asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
            asset_resource_descriptors: descriptors,
            resources: ArenaResources { resources },
            arena,
        };
        let asset = SimulationCollisionAsset {
            decoded,
            collision: CollisionArena {
                id: 0x8646_a1f7,
                grind_assets: Vec::new(),
                aggregate_roots: Vec::new(),
                primitive_sets: Vec::<CollisionPrimitiveSet>::new(),
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

    fn descriptors(sizes: [u32; 5], alignments: [u32; 5]) -> [BaseResourceDescriptor; 5] {
        std::array::from_fn(|index| BaseResourceDescriptor {
            size: sizes[index],
            alignment: alignments[index],
        })
    }

    fn resources(sizes: [usize; 5]) -> [Vec<u8>; 5] {
        std::array::from_fn(|index| vec![0; sizes[index]])
    }

    #[test]
    fn explicit_base_binds_nonstatic_inertia_and_retains_owner_binding_identity() {
        let owner = arena_owner(
            0,
            resources([0x1300, 0, 0, 0, 0]),
            descriptors([0x1300, 0, 0, 0, 0], [0x80, 3, 0, 7, 0]),
            [0x10; 5],
            [u32::MAX, 1, 2, 3, 4],
        );
        let weak_owner = Arc::downgrade(&owner);
        let source =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x110);
        let binding = SimulationArenaXenonAddressBinding::try_new(
            Arc::clone(&owner),
            [0x2000_0080, 0xdead_beef, u32::MAX, 3, 5],
        )
        .unwrap();
        {
            let bound = binding.try_bind_nonstatic_inertia_source(&source).unwrap();
            let alias = binding.try_bind_nonstatic_inertia_source(&source).unwrap();
            let next_binding = SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner),
                [0x2000_0080, 0, 0, 0, 0],
            )
            .unwrap();
            let next_epoch = next_binding
                .try_bind_nonstatic_inertia_source(&source)
                .unwrap();

            assert_eq!(bound.address(), 0x2000_0190);
            assert_ne!(bound.binding_epoch(), 0);
            assert!(bound.is_same_binding(&alias));
            assert!(bound.is_same_backing_inertia(&alias));
            assert!(!bound.is_same_binding(&next_epoch));
        }
        drop(source);
        drop(owner);
        assert!(
            weak_owner.upgrade().is_some(),
            "live allocation binding owns the arena"
        );
        drop(binding);
        assert!(weak_owner.upgrade().is_none());
    }

    #[test]
    fn binding_revalidates_old_base_and_retained_size() {
        let old_base = arena_owner(
            0x1000_0000,
            resources([0x100, 0, 0, 0, 0]),
            descriptors([0x100, 0, 0, 0, 0], [0x10, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        assert!(matches!(
            SimulationArenaXenonAddressBinding::try_new(old_base, [0; 5]),
            Err(
                SimulationArenaXenonAddressBindingError::UnsupportedArenaBaseOffset {
                    base_offset: 0x1000_0000
                }
            )
        ));

        let wrong_size = arena_owner(
            0,
            resources([0x100, 0, 0, 0, 0]),
            descriptors([0xff, 0, 0, 0, 0], [0x10, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        assert!(matches!(
            SimulationArenaXenonAddressBinding::try_new(wrong_size, [0x1000; 5]),
            Err(
                SimulationArenaXenonAddressBindingError::ResourceSizeMismatch {
                    resource_index: 0,
                    descriptor_size: 0xff,
                    retained_size: 0x100
                }
            )
        ));
    }

    #[test]
    fn binding_revalidates_nonempty_alignment_base_and_address_span() {
        let owner = arena_owner(
            0,
            resources([0x100, 0, 0, 0, 0]),
            descriptors([0x100, 0, 0, 0, 0], [0x80, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        assert!(matches!(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner),
                [0x2000_0010, 0, 0, 0, 0]
            ),
            Err(
                SimulationArenaXenonAddressBindingError::MisalignedResourceBase {
                    resource_index: 0,
                    base: 0x2000_0010,
                    required_alignment: 0x80
                }
            )
        ));
        assert!(matches!(
            SimulationArenaXenonAddressBinding::try_new(owner, [0; 5]),
            Err(
                SimulationArenaXenonAddressBindingError::NullNonemptyResourceBase {
                    resource_index: 0
                }
            )
        ));

        for alignment in [0, 3] {
            let invalid_alignment = arena_owner(
                0,
                resources([0x100, 0, 0, 0, 0]),
                descriptors([0x100, 0, 0, 0, 0], [alignment, 0, 0, 0, 0]),
                [0x10; 5],
                [0; 5],
            );
            assert!(matches!(
                SimulationArenaXenonAddressBinding::try_new(
                    invalid_alignment,
                    [0x2000_0000, 0, 0, 0, 0]
                ),
                Err(
                    SimulationArenaXenonAddressBindingError::InvalidResourceAlignment {
                        resource_index: 0,
                        alignment: actual
                    }
                ) if actual == alignment
            ));
        }

        let overflow = arena_owner(
            0,
            resources([0x1300, 0, 0, 0, 0]),
            descriptors([0x1300, 0, 0, 0, 0], [0x80, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        assert!(matches!(
            SimulationArenaXenonAddressBinding::try_new(overflow, [0xffff_f000, 0, 0, 0, 0]),
            Err(
                SimulationArenaXenonAddressBindingError::ResourceAddressOverflow {
                    resource_index: 0
                }
            )
        ));
    }

    #[test]
    fn binding_accepts_the_highest_checked_nonwrapping_span() {
        let high_owner = arena_owner(
            0,
            resources([0x1300, 0, 0, 0, 0]),
            descriptors([0x1300, 0, 0, 0, 0], [0x80, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        let high_source = SerialisedAssemblyPartInertiaSource::from_parts_for_test(
            Arc::clone(&high_owner),
            0,
            0x110,
        );
        let high_binding =
            SimulationArenaXenonAddressBinding::try_new(high_owner, [0xffff_ec80, 0, 0, 0, 0])
                .unwrap();
        assert_eq!(
            high_binding
                .try_bind_nonstatic_inertia_source(&high_source)
                .unwrap()
                .address(),
            0xffff_ed90
        );
    }

    #[test]
    fn binding_preserves_empty_and_cross_lane_allocator_unknowns() {
        let owner = arena_owner(
            0,
            resources([0x200, 0x200, 0, 0, 0]),
            descriptors([0x200, 0x200, 0, 0, 0], [0x10, 0x10, 0, 3, 0]),
            [0x100; 5],
            [0x1111_1111, 0x2222_2222, u32::MAX, 4, 5],
        );
        let binding = SimulationArenaXenonAddressBinding::try_new(
            Arc::clone(&owner),
            [0x3000, 0x3000, u32::MAX, 0xdead_beef, 1],
        )
        .unwrap();
        assert_eq!(
            binding.resource_bases,
            [0x3000, 0x3000, u32::MAX, 0xdead_beef, 1]
        );
        let lane_one = SerialisedAssemblyPartInertiaSource::from_parts_for_test(owner, 1, 0x20);

        assert_eq!(
            binding
                .try_bind_nonstatic_inertia_source(&lane_one)
                .unwrap()
                .address(),
            0x3020
        );
    }

    #[test]
    fn binding_selects_reversed_noncontiguous_resource_bases_independently() {
        let owner = arena_owner(
            0,
            resources([0x200, 0x200, 0, 0, 0]),
            descriptors([0x200, 0x200, 0, 0, 0], [0x10, 0x10, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        let binding = SimulationArenaXenonAddressBinding::try_new(
            Arc::clone(&owner),
            [0x4000, 0x3000, 0, 0, 0],
        )
        .unwrap();
        let source_zero =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x20);
        let source_one = SerialisedAssemblyPartInertiaSource::from_parts_for_test(owner, 1, 0x20);
        let bound_zero = binding
            .try_bind_nonstatic_inertia_source(&source_zero)
            .unwrap();
        let bound_one = binding
            .try_bind_nonstatic_inertia_source(&source_one)
            .unwrap();

        assert_eq!(bound_zero.address(), 0x4020);
        assert_eq!(bound_one.address(), 0x3020);
        assert!(bound_zero.is_same_binding(&bound_one));
        assert!(!bound_zero.is_same_backing_inertia(&bound_one));
    }

    #[test]
    fn inertia_resolution_checks_owner_resource_span_overflow_and_final_alignment() {
        let owner = arena_owner(
            0,
            resources([0x100, 0, 0, 0, 0]),
            descriptors([0x100, 0, 0, 0, 0], [1, 0, 0, 0, 0]),
            [0x100; 5],
            [0; 5],
        );
        let binding = SimulationArenaXenonAddressBinding::try_new(
            Arc::clone(&owner),
            [0xffff_fe00, 0, 0, 0, 0],
        )
        .unwrap();
        let foreign_owner = arena_owner(
            0,
            resources([0x100, 0, 0, 0, 0]),
            descriptors([0x100, 0, 0, 0, 0], [1, 0, 0, 0, 0]),
            [0x100; 5],
            [0; 5],
        );
        let foreign =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(foreign_owner, 0, 0x10);
        assert!(matches!(
            binding.try_bind_nonstatic_inertia_source(&foreign),
            Err(SimulationArenaXenonAddressBindingError::ForeignInertiaOwner)
        ));

        let unavailable =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 2, 0);
        assert!(matches!(
            binding.try_bind_nonstatic_inertia_source(&unavailable),
            Err(
                SimulationArenaXenonAddressBindingError::InertiaResourceUnavailable {
                    resource_index: 2
                }
            )
        ));
        let out_of_range =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 5, 0);
        assert!(matches!(
            binding.try_bind_nonstatic_inertia_source(&out_of_range),
            Err(
                SimulationArenaXenonAddressBindingError::InertiaResourceUnavailable {
                    resource_index: 5
                }
            )
        ));

        let outside =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0xf0);
        assert!(matches!(
            binding.try_bind_nonstatic_inertia_source(&outside),
            Err(
                SimulationArenaXenonAddressBindingError::InertiaOutsideResource {
                    resource_index: 0,
                    resource_offset: 0xf0
                }
            )
        ));

        let outside_before_absolute_math =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x300);
        assert!(matches!(
            binding.try_bind_nonstatic_inertia_source(&outside_before_absolute_math),
            Err(
                SimulationArenaXenonAddressBindingError::InertiaOutsideResource {
                    resource_index: 0,
                    resource_offset: 0x300
                }
            )
        ));

        let relative_overflow = SerialisedAssemblyPartInertiaSource::from_parts_for_test(
            Arc::clone(&owner),
            0,
            u32::MAX - 0x10,
        );
        assert!(matches!(
            binding.try_bind_nonstatic_inertia_source(&relative_overflow),
            Err(SimulationArenaXenonAddressBindingError::InertiaAddressOverflow)
        ));

        let misaligned =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x18);
        assert!(matches!(
            binding.try_bind_nonstatic_inertia_source(&misaligned),
            Err(
                SimulationArenaXenonAddressBindingError::MisalignedInertiaAddress {
                    address: 0xffff_fe18,
                    required_alignment: 0x10
                }
            )
        ));

        let exact_tail = SerialisedAssemblyPartInertiaSource::from_parts_for_test(owner, 0, 0xd0);
        assert_eq!(
            binding
                .try_bind_nonstatic_inertia_source(&exact_tail)
                .unwrap()
                .address(),
            0xffff_fed0
        );
    }

    fn identity() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn semantic_inertia(seed: f32) -> Arc<SimulationInertia> {
        Arc::new(SimulationInertia {
            inverse_tensor: [seed, seed + 1.0, seed + 2.0],
            inverse_mass: seed + 3.0,
            ..SimulationInertia::native_default()
        })
    }

    fn definition_inertia(
        semantic: &Arc<SimulationInertia>,
        source: &SerialisedAssemblyPartInertiaSource,
    ) -> SerialisedAssemblyRuntimeInertia {
        SerialisedAssemblyRuntimeInertia::from_parts_for_test(Arc::clone(semantic), source.clone())
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one source-aware matrix keeps all native Add outcomes directly comparable"
    )]
    fn add_inertia_owner_distinguishes_static_and_both_nonstatic_outcomes() {
        let owner = arena_owner(
            0,
            resources([0x200, 0, 0, 0, 0]),
            descriptors([0x200, 0, 0, 0, 0], [0x10, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        let source_a =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x10);
        let source_b =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x50);
        let foreign = SerialisedAssemblyPartInertiaSource::from_parts_for_test(
            arena_owner(
                0,
                resources([0x100, 0, 0, 0, 0]),
                descriptors([0x100, 0, 0, 0, 0], [0x10, 0, 0, 0, 0]),
                [0x10; 5],
                [0; 5],
            ),
            0,
            0x10,
        );
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(owner, [0x2000, 0, 0, 0, 0]).unwrap(),
        );
        let inertia = semantic_inertia(1.0);
        let foreign_inertia = definition_inertia(&inertia, &foreign);
        let inertia_a = definition_inertia(&inertia, &source_a);
        let inertia_b = definition_inertia(&inertia, &source_b);
        let mut pool = SimulationRigidBodyPool::try_new(5).unwrap();
        let mut outcomes = SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();

        let static_body = outcomes
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&foreign_inertia),
                1,
            )
            .expect("Static skips the foreign source")
            .expect("body");
        let frozen_definition = outcomes
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&inertia_a),
                2,
            )
            .unwrap()
            .unwrap();
        let active_definition = outcomes
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&inertia_b),
                4,
            )
            .unwrap()
            .unwrap();
        let frozen_null = outcomes
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Null,
                2,
            )
            .unwrap()
            .unwrap();
        let active_null = outcomes
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Null,
                u32::MAX,
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            outcomes.try_outcome(&pool, static_body.observation()),
            Ok(SimulationRigidBodyAddInertiaOutcome::StaticNull)
        );
        assert_eq!(
            outcomes.try_outcome(&pool, frozen_null.observation()),
            Ok(SimulationRigidBodyAddInertiaOutcome::NonStaticNull)
        );
        assert_eq!(
            outcomes.try_outcome(&pool, active_null.observation()),
            Ok(SimulationRigidBodyAddInertiaOutcome::NonStaticNull)
        );
        for (body, expected_address, expected_source) in [
            (frozen_definition, 0x2010, &source_a),
            (active_definition, 0x2050, &source_b),
        ] {
            let SimulationRigidBodyAddInertiaOutcome::Definition { address, source } =
                outcomes.try_outcome(&pool, body.observation()).unwrap()
            else {
                panic!("non-Static Definition input retains its exact source");
            };
            assert_eq!(address, expected_address);
            assert!(source.is_same_backing_inertia(expected_source));
        }
        assert!(
            !pool
                .try_packed_word_topology_snapshot(static_body.observation())
                .unwrap()
                .has_retained_inertia
        );
        assert!(
            !pool
                .try_packed_word_topology_snapshot(frozen_null.observation())
                .unwrap()
                .has_retained_inertia
        );
        assert!(
            pool.try_packed_word_topology_snapshot(active_definition.observation())
                .unwrap()
                .has_retained_inertia
        );
    }

    #[test]
    fn add_inertia_owner_binds_after_preflight_and_before_body_effects() {
        let owner = arena_owner(
            0,
            resources([0x100, 0, 0, 0, 0]),
            descriptors([0x100, 0, 0, 0, 0], [0x10, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        let valid =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x10);
        let foreign = SerialisedAssemblyPartInertiaSource::from_parts_for_test(
            arena_owner(
                0,
                resources([0x100, 0, 0, 0, 0]),
                descriptors([0x100, 0, 0, 0, 0], [0x10, 0, 0, 0, 0]),
                [0x10; 5],
                [0; 5],
            ),
            0,
            0x10,
        );
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(owner, [0x4000, 0, 0, 0, 0]).unwrap(),
        );
        let inertia = semantic_inertia(2.0);
        let valid_inertia = definition_inertia(&inertia, &valid);
        let foreign_inertia = definition_inertia(&inertia, &foreign);
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let mut outcomes = SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();
        let before = pool.snapshot();

        assert_eq!(
            outcomes.try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&foreign_inertia),
                2,
            ),
            Err(SimulationRigidBodyAddInertiaOwnerError::Binding(
                SimulationArenaXenonAddressBindingError::ForeignInertiaOwner
            ))
        );
        assert_eq!(pool.snapshot(), before);
        assert!(outcomes.entries[0].is_none());

        let body = outcomes
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&valid_inertia),
                2,
            )
            .unwrap()
            .unwrap();
        assert_eq!(body.id().index(), 0);
        assert_eq!(pool.snapshot().slots[0].occurrence, Some(1));

        let full_before = pool.snapshot();
        assert_eq!(
            outcomes.try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&foreign_inertia),
                u32::MAX,
            ),
            Ok(None)
        );
        assert_eq!(pool.snapshot(), full_before);
        assert_eq!(
            outcomes.try_outcome(&pool, body.observation()),
            Ok(SimulationRigidBodyAddInertiaOutcome::Definition {
                address: 0x4010,
                source: &valid,
            })
        );
    }

    #[test]
    fn add_inertia_owner_retires_successful_removal_and_replaces_on_reuse() {
        let owner = arena_owner(
            0,
            resources([0x100, 0, 0, 0, 0]),
            descriptors([0x100, 0, 0, 0, 0], [0x10, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        let source_a =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x10);
        let source_b =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x50);
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(owner, [0x6000, 0, 0, 0, 0]).unwrap(),
        );
        let inertia = semantic_inertia(3.0);
        let inertia_a = definition_inertia(&inertia, &source_a);
        let inertia_b = definition_inertia(&inertia, &source_b);
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let mut outcomes = SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();
        let first = outcomes
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&inertia_a),
                4,
            )
            .unwrap()
            .unwrap();

        outcomes.try_remove(&mut pool, first).unwrap();
        assert!(outcomes.entries[0].is_none());
        assert!(matches!(
            outcomes.try_outcome(&pool, first.observation()),
            Err(SimulationRigidBodyAddInertiaOwnerError::Pool(
                SimulationRigidBodyPoolError::BodyIsFree { slot: 0 }
            ))
        ));

        let second = outcomes
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&inertia_b),
                4,
            )
            .unwrap()
            .unwrap();
        assert_eq!(second.id(), first.id());
        assert_ne!(second.observation(), first.observation());
        assert!(matches!(
            outcomes.try_outcome(&pool, first.observation()),
            Err(SimulationRigidBodyAddInertiaOwnerError::Pool(
                SimulationRigidBodyPoolError::StaleHandle { slot: 0 }
            ))
        ));
        assert_eq!(
            outcomes.try_outcome(&pool, second.observation()),
            Ok(SimulationRigidBodyAddInertiaOutcome::Definition {
                address: 0x6050,
                source: &source_b,
            })
        );
        assert!(matches!(
            outcomes.try_remove(&mut pool, first),
            Err(SimulationRigidBodyAddInertiaOwnerError::Pool(
                SimulationRigidBodyPoolError::StaleHandle { slot: 0 }
            ))
        ));
        assert_eq!(
            outcomes.try_outcome(&pool, second.observation()),
            Ok(SimulationRigidBodyAddInertiaOutcome::Definition {
                address: 0x6050,
                source: &source_b,
            })
        );
    }

    #[test]
    fn add_inertia_owner_never_authorizes_slot_reuse_after_bypassed_removal() {
        let owner = arena_owner(
            0,
            resources([0x100, 0, 0, 0, 0]),
            descriptors([0x100, 0, 0, 0, 0], [0x10, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        let source =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x10);
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(owner, [0x7000, 0, 0, 0, 0]).unwrap(),
        );
        let inertia = semantic_inertia(3.5);
        let definition = definition_inertia(&inertia, &source);
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let mut outcomes = SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();
        let first = outcomes
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&definition),
                4,
            )
            .unwrap()
            .unwrap();

        pool.try_remove_rigid_body(first).unwrap();
        let replacement = pool
            .try_add_rigid_body(identity(), Some(&inertia), 4)
            .unwrap()
            .unwrap();
        assert_eq!(replacement.id(), first.id());
        assert!(matches!(
            outcomes.try_outcome(&pool, first.observation()),
            Err(SimulationRigidBodyAddInertiaOwnerError::Pool(
                SimulationRigidBodyPoolError::StaleHandle { slot: 0 }
            ))
        ));
        assert_eq!(
            outcomes.try_outcome(&pool, replacement.observation()),
            Err(SimulationRigidBodyAddInertiaOwnerError::MissingOccurrenceOutcome { slot: 0 })
        );

        pool.try_remove_rigid_body(replacement).unwrap();
        let before = pool.snapshot();
        assert_eq!(
            outcomes.try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&definition),
                4,
            ),
            Err(SimulationRigidBodyAddInertiaOwnerError::OccupiedSlot { slot: 0 })
        );
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn add_inertia_owner_rejects_foreign_and_unregistered_occurrences() {
        let owner = arena_owner(
            0,
            resources([0x100, 0, 0, 0, 0]),
            descriptors([0x100, 0, 0, 0, 0], [0x10, 0, 0, 0, 0]),
            [0x10; 5],
            [0; 5],
        );
        let source =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x10);
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(owner, [0x8000, 0, 0, 0, 0]).unwrap(),
        );
        let inertia = semantic_inertia(4.0);
        let definition_inertia = definition_inertia(&inertia, &source);
        let mut owned_pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let mut foreign_pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let mut outcomes =
            SimulationRigidBodyAddInertiaOwner::try_new(&owned_pool, binding).unwrap();
        let foreign = foreign_pool
            .try_add_rigid_body(identity(), Some(&inertia), 4)
            .unwrap()
            .unwrap();

        assert_eq!(
            outcomes.try_add(
                &mut foreign_pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&definition_inertia),
                4,
            ),
            Err(SimulationRigidBodyAddInertiaOwnerError::ForeignBodyPool)
        );
        assert_eq!(
            outcomes.try_outcome(&foreign_pool, foreign.observation()),
            Err(SimulationRigidBodyAddInertiaOwnerError::ForeignBodyPool)
        );

        let unregistered = owned_pool
            .try_add_rigid_body(identity(), Some(&inertia), 4)
            .unwrap()
            .unwrap();
        assert_eq!(
            outcomes.try_outcome(&owned_pool, unregistered.observation()),
            Err(SimulationRigidBodyAddInertiaOwnerError::MissingOccurrenceOutcome { slot: 0 })
        );
        assert_eq!(
            outcomes.try_remove(&mut owned_pool, unregistered),
            Err(SimulationRigidBodyAddInertiaOwnerError::MissingOccurrenceOutcome { slot: 0 })
        );
        assert_eq!(owned_pool.counts().active, 1);
    }

    #[test]
    fn literal_nonzero_capacity_layout_matches_initialize() {
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 2, 3, 4).unwrap();

        assert_eq!(image.simulation_address(), 0x1000_0080);
        assert_eq!(image.solver_delta_row_base_address(), 0x1000_0380);
        assert_eq!(image.rigid_body_base_address(), 0x1000_0400);
        assert_eq!(image.rigid_body_slot_base_address(), 0x1000_06c0);
        assert_eq!(image.joint_base_address(), 0x1000_0820);
        assert_eq!(image.drive_base_address(), 0x1000_08c0);
        assert_eq!(image.end_address(), 0x1000_0980);
        assert_eq!(image.initialized_size(), 0x900);
        assert_eq!(image.rigid_body_capacity(), 2);
        assert_eq!(image.joint_capacity(), 3);
        assert_eq!(image.drive_capacity(), 4);

        let descriptor = SimulationManagerXenonAllocationDescriptor::try_new(2, 3, 4).unwrap();
        assert_eq!(descriptor.size(), 0x9f0);
        assert_eq!(
            SimulationManagerXenonAllocationDescriptor::alignment(),
            0x80
        );
        assert_eq!(descriptor.size(), image.end_address() - 0x1000_0080 + 0xf0);
    }

    #[test]
    fn packed_word_image_domain_binds_capacity_pool_and_unique_generation() {
        let pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 2, 3, 4).unwrap();
        let first = SimulationPackedBodyWordImageDomain::try_new(&pool, image).unwrap();
        let second = SimulationPackedBodyWordImageDomain::try_new(&pool, image).unwrap();
        assert!(first.is_same_pool(&pool));
        assert_ne!(first.epoch(), second.epoch());
        assert_eq!(first.simulation_address(), 0x1000_0080);

        let wrong = SimulationXenonAddressImage::try_new(0x2000_0080, 1, 0, 0).unwrap();
        assert!(matches!(
            SimulationPackedBodyWordImageDomain::try_new(&pool, wrong),
            Err(SimulationPackedBodyWordImageDomainError::CapacityMismatch { image: 1, pool: 2 })
        ));
        let foreign = SimulationRigidBodyPool::try_new(2).unwrap();
        assert!(!first.is_same_pool(&foreign));
    }

    #[test]
    fn zero_capacities_retain_all_native_sentinels() {
        let image = SimulationXenonAddressImage::try_new(0x80, 0, 0, 0).unwrap();

        assert_eq!(image.solver_delta_row_base_address(), 0x380);
        assert_eq!(image.rigid_body_base_address(), 0x380);
        assert_eq!(image.rigid_body_slot_base_address(), 0x640);
        assert_eq!(image.joint_base_address(), 0x640);
        assert_eq!(image.drive_base_address(), 0x680);
        assert_eq!(image.end_address(), 0x6c0);
        assert_eq!(image.initialized_size(), 0x640);

        let descriptor = SimulationManagerXenonAllocationDescriptor::try_new(0, 0, 0).unwrap();
        assert_eq!(descriptor.size(), 0x730);
        assert_eq!(descriptor.size(), image.end_address() - 0x80 + 0xf0);
    }

    #[test]
    fn anchors_slots_nodes_and_rf_rows_use_distinct_native_strides() {
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 2, 3, 4).unwrap();
        let ids = body_ids(2);

        assert_eq!(
            image.rigid_body_anchor_address(SimulationRigidBodyList::Free),
            0x1000_0400
        );
        assert_eq!(
            image.rigid_body_anchor_address(SimulationRigidBodyList::Static),
            0x1000_04b0
        );
        assert_eq!(
            image.rigid_body_anchor_address(SimulationRigidBodyList::Frozen),
            0x1000_0560
        );
        assert_eq!(
            image.rigid_body_anchor_address(SimulationRigidBodyList::Active),
            0x1000_0610
        );
        assert_eq!(image.rigid_body_address(ids[0]).unwrap(), 0x1000_06c0);
        assert_eq!(image.rigid_body_address(ids[1]).unwrap(), 0x1000_0770);
        assert_eq!(image.solver_delta_row_address(ids[0]).unwrap(), 0x1000_0380);
        assert_eq!(image.solver_delta_row_address(ids[1]).unwrap(), 0x1000_03c0);
        assert_eq!(
            image
                .rigid_body_node_address(SimulationRigidBodyNode::Anchor(
                    SimulationRigidBodyList::Frozen,
                ))
                .unwrap(),
            0x1000_0560
        );
        assert_eq!(
            image
                .rigid_body_node_address(SimulationRigidBodyNode::Body(ids[1]))
                .unwrap(),
            0x1000_0770
        );
        assert_eq!(
            image.rigid_body_address(ids[1]).unwrap() + RIGID_BODY_STRIDE,
            image.joint_base_address()
        );
        assert_eq!(
            image.solver_delta_row_address(ids[1]).unwrap() + SOLVER_DELTA_ROW_STRIDE,
            image.rigid_body_base_address()
        );
    }

    #[test]
    fn body_and_rf_lookup_reject_the_same_out_of_range_slot() {
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 2, 0, 0).unwrap();
        let outside = body_ids(3)[2];
        let expected = SimulationXenonAddressImageError::RigidBodySlotOutOfRange {
            slot: 2,
            capacity: 2,
        };

        assert_eq!(image.rigid_body_address(outside), Err(expected));
        assert_eq!(image.solver_delta_row_address(outside), Err(expected));
        assert_eq!(
            image.rigid_body_node_address(SimulationRigidBodyNode::Body(outside)),
            Err(expected)
        );
    }

    #[test]
    fn base_must_be_nonnull_and_native_aligned() {
        assert_eq!(
            SimulationXenonAddressImage::try_new(0, 0, 0, 0),
            Err(SimulationXenonAddressImageError::NullSimulationBase)
        );
        assert_eq!(
            SimulationXenonAddressImage::try_new(0x1000_0040, 0, 0, 0),
            Err(SimulationXenonAddressImageError::MisalignedSimulationBase {
                base: 0x1000_0040,
                required_alignment: 0x80,
            })
        );
    }

    #[test]
    fn checked_construction_reports_the_first_unrepresentable_stage() {
        assert_eq!(
            SimulationXenonAddressImage::try_new(0xffff_ff80, 0, 0, 0),
            Err(SimulationXenonAddressImageError::AddressOverflow {
                step: SimulationXenonAddressStep::SolverDeltaRowBase,
            })
        );
        assert_eq!(
            SimulationXenonAddressImage::try_new(0x80, 0x0400_0000, 0, 0),
            Err(SimulationXenonAddressImageError::AddressOverflow {
                step: SimulationXenonAddressStep::SolverDeltaRowExtent,
            })
        );
        assert_eq!(
            SimulationXenonAddressImage::try_new(0x80, 0, u32::MAX, 0),
            Err(SimulationXenonAddressImageError::AddressOverflow {
                step: SimulationXenonAddressStep::JointAllocationCount,
            })
        );
        assert_eq!(
            SimulationXenonAddressImage::try_new(0x80, 0, 0, u32::MAX),
            Err(SimulationXenonAddressImageError::AddressOverflow {
                step: SimulationXenonAddressStep::DriveAllocationCount,
            })
        );
        assert_eq!(
            SimulationXenonAddressImage::try_new(0x80, 0, 0, 0x07ff_fffd),
            Err(SimulationXenonAddressImageError::AddressOverflow {
                step: SimulationXenonAddressStep::AllocationEnd,
            })
        );
        assert_eq!(
            SimulationManagerXenonAllocationDescriptor::try_new(u32::MAX, 0, 0),
            Err(SimulationXenonAddressImageError::AddressOverflow {
                step: SimulationXenonAddressStep::ManagerRigidBodyCapacity,
            })
        );
        assert_eq!(
            SimulationManagerXenonAllocationDescriptor::try_new(u32::MAX - 1, 0, 0),
            Err(SimulationXenonAddressImageError::AddressOverflow {
                step: SimulationXenonAddressStep::DescriptorHeaderRowCount,
            })
        );
    }

    #[test]
    fn checked_alignment_accepts_an_aligned_high_address_but_refuses_wrap() {
        assert_eq!(
            checked_align_up(
                0xffff_ffc0,
                0x40,
                SimulationXenonAddressStep::SolverDeltaRowBase,
            ),
            Ok(0xffff_ffc0)
        );
        assert_eq!(
            checked_align_up(
                0xffff_ffe1,
                0x40,
                SimulationXenonAddressStep::SolverDeltaRowBase,
            ),
            Err(SimulationXenonAddressImageError::AddressOverflow {
                step: SimulationXenonAddressStep::SolverDeltaRowBase,
            })
        );
    }
}
