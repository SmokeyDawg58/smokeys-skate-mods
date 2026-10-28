//! Private collision-Contact append storage.
//!
//! Native `GlobalBuffer<Contact, ExternalMemory>::Proxy::Store` at
//! `0x827cb5c8..0x827cb678` reserves a dense prefix through the Simulation
//! Contact count. `TempContactBuffer::GetNextFreeContact` at
//! `0x827cb7e8..0x827cb89c` layers a 50-record local buffer over that append.
//! This module reconstructs their serialized reservation and accepted-prefix
//! behavior, including the native reservation counter's temporary overshoot,
//! with typed host ownership. It does not construct a Contact, authorize a
//! collision producer, or claim the native cross-job reservation order.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the recovered non-network Contact producers"
    )
)]

use crate::simulation_contact_jacobian::SimulationContactBuildInput;
use crate::simulation_contact_jacobian::ground_preparation::SimulationGroundContactRecord;
use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

const TEMP_CONTACT_CAPACITY: usize = 50;

static NEXT_CONTACT_APPEND_AUTHORITY: AtomicU64 = AtomicU64::new(1);
static NEXT_TEMP_CONTACT_BUFFER_AUTHORITY: AtomicU64 = AtomicU64::new(1);

const _: () = assert!(size_of::<SimulationContactBuildInput>() == 0x100);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationContactAppendInitError {
    MaximumExceedsSignedRange { maximum: u32 },
    AllocationFailed { maximum: u32 },
    AuthorityExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationContactStoreError {
    RequestedCountOverflow { requested: usize },
    CounterOverflow { count: u32, requested: u32 },
    InvalidCounter { count: u32, maximum: u32 },
}

/// Host-visible description of one native Store result.
///
/// Native returns only `start`; `accepted` and `clamped` remain private host
/// observability. `clamped` describes selection of the bounded COPY count,
/// not a store to the native reservation counter. Exact fit selects that
/// bounded count too, but does not execute an `AtomicStore`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationContactStoreReceipt {
    start: u32,
    accepted: u32,
    clamped: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationContactStorePlan {
    receipt: SimulationContactStoreReceipt,
    final_count: u32,
    copy_records: u32,
}

/// Reconstructs Store's single serialized reservation result.
///
/// The native implementation always adds first. Only a nonpositive signed
/// `maximum - start` causes its distinct `AtomicStore(maximum)`. A positive
/// remainder limits copying without undoing any counter overshoot. Actual
/// overlapping reservations are not promoted into a linearizable host
/// contract. Unrepresentable signed differences and unsigned addition wrap
/// are explicit safe-host refusals, not reconstructed racing/malformed input.
fn try_plan_contact_store(
    count: u32,
    maximum: u32,
    requested: usize,
    destination_alias: bool,
) -> Result<SimulationContactStorePlan, SimulationContactStoreError> {
    if requested == 0 {
        return Ok(SimulationContactStorePlan {
            receipt: SimulationContactStoreReceipt {
                start: 0,
                accepted: 0,
                clamped: false,
            },
            final_count: count,
            copy_records: 0,
        });
    }
    if maximum > i32::MAX as u32 {
        return Err(SimulationContactStoreError::InvalidCounter { count, maximum });
    }
    let requested = u32::try_from(requested)
        .map_err(|_| SimulationContactStoreError::RequestedCountOverflow { requested })?;

    // Native obtains `start` by subtracting q from AtomicAdd's post-add value.
    // The checked host rejects wrap instead of reconstructing malformed/racing
    // counter behavior that has no supported game route.
    let post_add = count
        .checked_add(requested)
        .ok_or(SimulationContactStoreError::CounterOverflow { count, requested })?;
    let start = post_add - requested;
    let remaining = i32::try_from(i64::from(maximum) - i64::from(start))
        .map_err(|_| SimulationContactStoreError::InvalidCounter { count, maximum })?;
    let (accepted, clamped) = if remaining <= 0 {
        (0, true)
    } else {
        let positive_remaining = u32::try_from(remaining).expect("positive signed remainder");
        if requested < positive_remaining {
            (requested, false)
        } else {
            (positive_remaining, true)
        }
    };
    Ok(SimulationContactStorePlan {
        receipt: SimulationContactStoreReceipt {
            start,
            accepted,
            clamped,
        },
        final_count: if remaining <= 0 { maximum } else { post_add },
        copy_records: if destination_alias { 0 } else { accepted },
    })
}

/// Exclusive unsealed destination for complete producer-owned Contact values.
///
/// `T` will become the private occurrence-qualified collision Contact value;
/// keeping this owner generic prevents this storage primitive from becoming an
/// arbitrary public Contact constructor. Capacity is preallocated so Store has
/// no allocation or partial-allocation failure after reservation. Only the
/// accepted prefix is initialized, as in native Store's memcpy at 0x827cb668.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationContactAppendAuthority(NonZeroU64);

fn fresh_authority(source: &AtomicU64) -> Result<NonZeroU64, SimulationContactAppendInitError> {
    let raw = source
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| SimulationContactAppendInitError::AuthorityExhausted)?;
    NonZeroU64::new(raw).ok_or(SimulationContactAppendInitError::AuthorityExhausted)
}

struct SimulationContactAppend<T: Copy> {
    authority: SimulationContactAppendAuthority,
    maximum: u32,
    reservation_count: u32,
    published_count: u32,
    slots: Vec<T>,
}

impl<T: Copy> SimulationContactAppend<T> {
    fn try_new(maximum: u32) -> Result<Self, SimulationContactAppendInitError> {
        if maximum > i32::MAX as u32 {
            return Err(SimulationContactAppendInitError::MaximumExceedsSignedRange { maximum });
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(maximum as usize)
            .map_err(|_| SimulationContactAppendInitError::AllocationFailed { maximum })?;
        Ok(Self {
            authority: SimulationContactAppendAuthority(fresh_authority(
                &NEXT_CONTACT_APPEND_AUTHORITY,
            )?),
            maximum,
            reservation_count: 0,
            published_count: 0,
            slots,
        })
    }

    fn store_prefix(
        &mut self,
        source: &[T],
    ) -> Result<SimulationContactStoreReceipt, SimulationContactStoreError> {
        let plan =
            try_plan_contact_store(self.reservation_count, self.maximum, source.len(), false)?;
        let start = plan.receipt.start as usize;
        let accepted = plan.receipt.accepted as usize;
        debug_assert_eq!(plan.copy_records as usize, accepted);
        // Native forms the destination address only when it copies. A full
        // Store can return a start beyond the actual allocation after an
        // earlier overshoot, so even an empty host slice at start is invalid.
        if accepted != 0 {
            debug_assert_eq!(start, self.published_count as usize);
            debug_assert_eq!(start, self.slots.len());
            debug_assert!(start + accepted <= self.slots.capacity());
            self.slots.extend_from_slice(&source[..accepted]);
            self.published_count += plan.receipt.accepted;
        }
        self.reservation_count = plan.final_count;
        Ok(plan.receipt)
    }

    /// Initialized dense prefix length, NOT the possibly overshot native
    /// atomic reservation counter. Only this count authorizes host row reads.
    const fn count(&self) -> u32 {
        self.published_count
    }

    fn occupied(&self) -> impl Iterator<Item = &T> {
        self.slots[..self.published_count as usize].iter()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationTempContactSlot {
    buffer: SimulationTempContactBufferAuthority,
    index: u8,
    generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationTempContactReservation {
    slot: SimulationTempContactSlot,
    flushed: Option<SimulationContactStoreReceipt>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationTempContactBufferError {
    AuthorityExhausted,
    DestinationMismatch,
    UninitializedSlot { index: usize },
    StaleSlot,
    SlotOutOfOrder { index: usize, count: usize },
    SlotAlreadyInitialized { index: usize },
    ReservationGenerationExhausted,
    Store(SimulationContactStoreError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationTempContactBufferAuthority(NonZeroU64);

/// One producer-local 50-record Contact buffer.
///
/// Reserving increments `count` before returning the slot, as native does. A
/// typed write is required before flush. The checked stale/out-of-order errors
/// are safe-host ownership policy around what were raw pointers in the game.
struct SimulationTempContactBuffer<T: Copy> {
    authority: SimulationTempContactBufferAuthority,
    destination: SimulationContactAppendAuthority,
    slots: [Option<T>; TEMP_CONTACT_CAPACITY],
    slot_generations: [u64; TEMP_CONTACT_CAPACITY],
    count: usize,
    can_flush: bool,
    next_generation: u64,
}

impl<T: Copy> SimulationTempContactBuffer<T> {
    fn try_new(
        can_flush: bool,
        destination: &SimulationContactAppend<T>,
    ) -> Result<Self, SimulationTempContactBufferError> {
        let raw = fresh_authority(&NEXT_TEMP_CONTACT_BUFFER_AUTHORITY)
            .map_err(|_| SimulationTempContactBufferError::AuthorityExhausted)?;
        Ok(Self {
            authority: SimulationTempContactBufferAuthority(raw),
            destination: destination.authority,
            slots: [None; TEMP_CONTACT_CAPACITY],
            slot_generations: [0; TEMP_CONTACT_CAPACITY],
            count: 0,
            can_flush,
            next_generation: 1,
        })
    }

    fn try_get_next_free_contact(
        &mut self,
        destination: &mut SimulationContactAppend<T>,
    ) -> Result<Option<SimulationTempContactReservation>, SimulationTempContactBufferError> {
        if self.count >= TEMP_CONTACT_CAPACITY && !self.can_flush {
            return Ok(None);
        }
        let generation = self.next_generation;
        let next_generation = generation
            .checked_add(1)
            .ok_or(SimulationTempContactBufferError::ReservationGenerationExhausted)?;
        let flushed = if self.count >= TEMP_CONTACT_CAPACITY {
            Some(self.flush_nonempty(destination)?)
        } else {
            None
        };

        self.next_generation = next_generation;
        let index = self.count;
        self.count += 1;
        self.slot_generations[index] = generation;
        Ok(Some(SimulationTempContactReservation {
            slot: SimulationTempContactSlot {
                buffer: self.authority,
                index: u8::try_from(index).expect("the fixed Contact buffer has 50 slots"),
                generation,
            },
            flushed,
        }))
    }

    fn try_write_reserved(
        &mut self,
        slot: SimulationTempContactSlot,
        value: T,
    ) -> Result<(), SimulationTempContactBufferError> {
        let index = slot.index as usize;
        if slot.buffer != self.authority
            || self.slot_generations.get(index).copied() != Some(slot.generation)
        {
            return Err(SimulationTempContactBufferError::StaleSlot);
        }
        if index >= self.count {
            return Err(SimulationTempContactBufferError::SlotOutOfOrder {
                index,
                count: self.count,
            });
        }
        if self.slots[index].is_some() {
            return Err(SimulationTempContactBufferError::SlotAlreadyInitialized { index });
        }
        self.slots[index] = Some(value);
        Ok(())
    }

    /// Models assembly/assembly and specialized-producer invalid admission.
    /// Ground production deliberately does not call this operation.
    fn try_discard_last(
        &mut self,
        slot: SimulationTempContactSlot,
    ) -> Result<(), SimulationTempContactBufferError> {
        let index = slot.index as usize;
        if slot.buffer != self.authority
            || self.slot_generations.get(index).copied() != Some(slot.generation)
        {
            return Err(SimulationTempContactBufferError::StaleSlot);
        }
        if index.checked_add(1) != Some(self.count) {
            return Err(SimulationTempContactBufferError::SlotOutOfOrder {
                index,
                count: self.count,
            });
        }
        self.slots[index] = None;
        self.slot_generations[index] = 0;
        self.count -= 1;
        Ok(())
    }

    fn flush_nonempty(
        &mut self,
        destination: &mut SimulationContactAppend<T>,
    ) -> Result<SimulationContactStoreReceipt, SimulationTempContactBufferError> {
        debug_assert!(self.count != 0, "native callers guard tail flushes");
        if destination.authority != self.destination {
            return Err(SimulationTempContactBufferError::DestinationMismatch);
        }
        let first = self.slots[0]
            .ok_or(SimulationTempContactBufferError::UninitializedSlot { index: 0 })?;
        let mut initialized = [first; TEMP_CONTACT_CAPACITY];
        for (index, (destination, slot)) in initialized[..self.count]
            .iter_mut()
            .zip(&self.slots[..self.count])
            .enumerate()
        {
            *destination =
                slot.ok_or(SimulationTempContactBufferError::UninitializedSlot { index })?;
        }
        let receipt = destination
            .store_prefix(&initialized[..self.count])
            .map_err(SimulationTempContactBufferError::Store)?;
        for (slot, generation) in self.slots[..self.count]
            .iter_mut()
            .zip(&mut self.slot_generations[..self.count])
        {
            *slot = None;
            *generation = 0;
        }
        self.count = 0;
        Ok(receipt)
    }

    fn tail_flush_and_reset(
        &mut self,
        destination: &mut SimulationContactAppend<T>,
    ) -> Result<Option<SimulationContactStoreReceipt>, SimulationTempContactBufferError> {
        if self.count == 0 {
            return Ok(None);
        }
        self.flush_nonempty(destination).map(Some)
    }

    const fn count(&self) -> usize {
        self.count
    }
}

/// Result of one Assembly/ground reserve-before-compose attempt.
///
/// `Unavailable` is the native null-return route from
/// `TempContactBuffer::GetNextFreeContact`; the surrounding
/// `RunIntersectionTests` owner must stop that invocation immediately.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AssemblyGroundContactWriteOutcome {
    Written,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssemblyBodyContactWriteOutcome {
    Written,
    Discarded,
    Unavailable,
}

/// Checked host-storage failure before Assembly/ground composition begins.
///
/// Native valid execution has no matching failure route. The source remains
/// private so this boundary cannot become generic temporary-buffer authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AssemblyGroundContactWriteError {
    source: SimulationTempContactBufferError,
}

/// Opaque write-only capability for the Assembly/ground Contact producer.
///
/// A future manager-owned collision frame is the only intended issuer. The
/// generic destination, temporary buffer, reservation token, flush receipt,
/// and discard operation remain inaccessible to the Run owner.
pub(super) struct AssemblyGroundContactWritePort<'frame, T: Copy = SimulationContactBuildInput> {
    destination: &'frame mut SimulationContactAppend<T>,
    temp: &'frame mut SimulationTempContactBuffer<T>,
}

impl<T: Copy> AssemblyGroundContactWritePort<'_, T> {
    /// Reserves at `0x827cd4e4..0x827cd4f0` before invoking the infallible
    /// final-value compositor represented by `compose`.
    ///
    /// A null reservation returns `Unavailable` without invoking `compose`.
    /// Once reservation succeeds, unwinding from `compose` is terminal for the
    /// owning buffer: this method deliberately does not imitate the
    /// Assembly/assembly discard route, which ground production never calls.
    pub(super) fn try_reserve_then_write(
        &mut self,
        compose: impl FnOnce() -> T,
    ) -> Result<AssemblyGroundContactWriteOutcome, AssemblyGroundContactWriteError> {
        let Some(reservation) = self
            .temp
            .try_get_next_free_contact(self.destination)
            .map_err(|source| AssemblyGroundContactWriteError { source })?
        else {
            return Ok(AssemblyGroundContactWriteOutcome::Unavailable);
        };

        let value = compose();
        self.temp
            .try_write_reserved(reservation.slot, value)
            .expect("fresh Assembly/ground reservation must remain exclusively owned");
        Ok(AssemblyGroundContactWriteOutcome::Written)
    }
}

/// CPU job-local temporary storage over one serialized global Contact prefix.
/// Only the recovered ground producer can construct the record payload.
pub(crate) struct SimulationGroundContactBuffer {
    destination: SimulationContactAppend<SimulationGroundContactRecord>,
    temp: SimulationTempContactBuffer<SimulationGroundContactRecord>,
    poisoned: bool,
}

#[derive(Debug)]
pub(crate) enum SimulationGroundContactBufferError {
    Initialization,
    Storage(AssemblyGroundContactWriteError),
    TailFlush,
    UnflushedTail,
    Poisoned,
    Allocation,
}

impl std::fmt::Display for SimulationGroundContactBufferError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initialization => {
                formatter.write_str("ground Contact buffer initialization failed")
            }
            Self::Storage(error) => write!(
                formatter,
                "ground Contact reservation failed: {:?}",
                error.source
            ),
            Self::TailFlush => formatter.write_str("ground Contact tail flush failed"),
            Self::UnflushedTail => {
                formatter.write_str("ground Contact buffer has an unflushed assembly tail")
            }
            Self::Poisoned => formatter.write_str("ground Contact buffer is poisoned"),
            Self::Allocation => formatter.write_str("ground Contact prefix allocation failed"),
        }
    }
}

impl SimulationGroundContactBuffer {
    pub(crate) fn try_new(
        maximum: u32,
        allow_multiple_writes: bool,
    ) -> Result<Self, SimulationGroundContactBufferError> {
        let destination = SimulationContactAppend::try_new(maximum)
            .map_err(|_| SimulationGroundContactBufferError::Initialization)?;
        let temp = SimulationTempContactBuffer::try_new(allow_multiple_writes, &destination)
            .map_err(|_| SimulationGroundContactBufferError::Initialization)?;
        Ok(Self {
            destination,
            temp,
            poisoned: false,
        })
    }

    pub(crate) fn try_reserve_then_write(
        &mut self,
        compose: impl FnOnce() -> SimulationGroundContactRecord,
    ) -> Result<AssemblyGroundContactWriteOutcome, SimulationGroundContactBufferError> {
        if self.poisoned {
            return Err(SimulationGroundContactBufferError::Poisoned);
        }
        // A caught composition panic cannot turn a reserved, uninitialized slot
        // into a reusable producer. Native ground has no discard/rollback path.
        self.poisoned = true;
        let result = AssemblyGroundContactWritePort {
            destination: &mut self.destination,
            temp: &mut self.temp,
        }
        .try_reserve_then_write(compose)
        .map_err(SimulationGroundContactBufferError::Storage)?;
        self.poisoned = false;
        Ok(result)
    }

    /// Assembly/assembly 827cc108..3b8 reserves before composing, then releases
    /// an invalid candidate. Discarded still counts as a found contact for thaw.
    pub(crate) fn try_reserve_then_validate(
        &mut self,
        compose: impl FnOnce() -> Option<SimulationGroundContactRecord>,
    ) -> Result<AssemblyBodyContactWriteOutcome, SimulationGroundContactBufferError> {
        if self.poisoned {
            return Err(SimulationGroundContactBufferError::Poisoned);
        }
        self.poisoned = true;
        let reservation = self
            .temp
            .try_get_next_free_contact(&mut self.destination)
            .map_err(|source| {
                SimulationGroundContactBufferError::Storage(AssemblyGroundContactWriteError {
                    source,
                })
            })?;
        let result = if let Some(reservation) = reservation {
            if let Some(record) = compose() {
                self.temp
                    .try_write_reserved(reservation.slot, record)
                    .expect("fresh assembly contact slot");
                AssemblyBodyContactWriteOutcome::Written
            } else {
                self.temp
                    .try_discard_last(reservation.slot)
                    .expect("only the freshly reserved assembly contact is discarded");
                AssemblyBodyContactWriteOutcome::Discarded
            }
        } else {
            AssemblyBodyContactWriteOutcome::Unavailable
        };
        self.poisoned = false;
        Ok(result)
    }

    /// CPU ground job `0x827cdecc..0x827cdee4`: flush after each assembly.
    /// Assembly/assembly 827ccaf0..b1c also flushes after each A assembly.
    pub(crate) fn try_finish_assembly(&mut self) -> Result<(), SimulationGroundContactBufferError> {
        if self.poisoned {
            return Err(SimulationGroundContactBufferError::Poisoned);
        }
        self.poisoned = true;
        self.temp
            .tail_flush_and_reset(&mut self.destination)
            .map_err(|_| SimulationGroundContactBufferError::TailFlush)?;
        self.poisoned = false;
        Ok(())
    }

    pub(crate) fn try_into_records(
        self,
    ) -> Result<Vec<SimulationGroundContactRecord>, SimulationGroundContactBufferError> {
        if self.poisoned {
            return Err(SimulationGroundContactBufferError::Poisoned);
        }
        if self.temp.count() != 0 {
            return Err(SimulationGroundContactBufferError::UnflushedTail);
        }
        // Keep the returned allocation sized to actual contacts. The producer's
        // maximum-capacity scratch must not survive in retained frame results.
        let mut records = Vec::new();
        records
            .try_reserve_exact(self.destination.count() as usize)
            .map_err(|_| SimulationGroundContactBufferError::Allocation)?;
        records.extend(self.destination.occupied().copied());
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn assembly_body_invalid_candidates_release_the_reserved_slot() {
        let mut buffer = SimulationGroundContactBuffer::try_new(0, false).unwrap();
        for _ in 0..100 {
            assert_eq!(
                buffer.try_reserve_then_validate(|| None).unwrap(),
                AssemblyBodyContactWriteOutcome::Discarded
            );
        }
        buffer.try_finish_assembly().unwrap();
        assert!(buffer.try_into_records().unwrap().is_empty());
    }

    #[test]
    fn collision_storage_uses_the_complete_0x100_contact_input() {
        assert_eq!(size_of::<SimulationContactBuildInput>(), 0x100);
    }

    fn fill_temp(
        temp: &mut SimulationTempContactBuffer<u32>,
        destination: &mut SimulationContactAppend<u32>,
        count: u32,
    ) {
        for value in 0..count {
            let reservation = temp
                .try_get_next_free_contact(destination)
                .expect("reserve")
                .expect("slot");
            assert_eq!(reservation.flushed, None);
            temp.try_write_reserved(reservation.slot, value)
                .expect("write");
        }
    }

    fn contact_words(value: u32) -> SimulationContactBuildInput {
        SimulationContactBuildInput::from_words([[value; 4]; 16])
    }

    fn fill_contact_temp(
        temp: &mut SimulationTempContactBuffer<SimulationContactBuildInput>,
        destination: &mut SimulationContactAppend<SimulationContactBuildInput>,
        count: u32,
    ) {
        for value in 0..count {
            let reservation = temp
                .try_get_next_free_contact(destination)
                .expect("reserve Contact")
                .expect("Contact slot");
            temp.try_write_reserved(reservation.slot, contact_words(value))
                .expect("write Contact");
        }
    }

    #[test]
    fn assembly_ground_full_nonflush_stops_before_composition_and_keeps_prefix() {
        let mut destination = SimulationContactAppend::try_new(100).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(false, &destination).expect("temp buffer");
        fill_contact_temp(&mut temp, &mut destination, 50);
        let composed = Cell::new(false);

        {
            let mut writer = AssemblyGroundContactWritePort {
                destination: &mut destination,
                temp: &mut temp,
            };
            assert_eq!(
                writer
                    .try_reserve_then_write(|| {
                        composed.set(true);
                        contact_words(50)
                    })
                    .expect("ordinary full route"),
                AssemblyGroundContactWriteOutcome::Unavailable
            );
        }

        assert!(!composed.get());
        assert_eq!(temp.count(), 50);
        assert_eq!(destination.count(), 0);
        temp.tail_flush_and_reset(&mut destination)
            .expect("publish retained prefix");
        assert_eq!(
            destination
                .occupied()
                .map(|contact| contact.test_words()[0][0])
                .collect::<Vec<_>>(),
            (0..50).collect::<Vec<_>>()
        );
    }

    #[test]
    fn assembly_ground_later_null_retains_first_new_point_without_rollback() {
        let mut destination = SimulationContactAppend::try_new(100).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(false, &destination).expect("temp buffer");
        fill_contact_temp(&mut temp, &mut destination, 49);
        let second_composed = Cell::new(false);

        {
            let mut writer = AssemblyGroundContactWritePort {
                destination: &mut destination,
                temp: &mut temp,
            };
            assert_eq!(
                writer
                    .try_reserve_then_write(|| contact_words(49))
                    .expect("first point"),
                AssemblyGroundContactWriteOutcome::Written
            );
            assert_eq!(
                writer
                    .try_reserve_then_write(|| {
                        second_composed.set(true);
                        contact_words(50)
                    })
                    .expect("second point full route"),
                AssemblyGroundContactWriteOutcome::Unavailable
            );
        }

        assert!(!second_composed.get());
        assert_eq!(temp.count(), 50);
        temp.tail_flush_and_reset(&mut destination)
            .expect("publish retained prefix");
        assert_eq!(
            destination
                .occupied()
                .map(|contact| contact.test_words()[0][0])
                .collect::<Vec<_>>(),
            (0..50).collect::<Vec<_>>()
        );
    }

    #[test]
    fn assembly_ground_full_flushes_then_composes_into_new_slot_zero() {
        let mut destination = SimulationContactAppend::try_new(100).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(true, &destination).expect("temp buffer");
        fill_contact_temp(&mut temp, &mut destination, 50);
        let compose_calls = Cell::new(0);

        {
            let mut writer = AssemblyGroundContactWritePort {
                destination: &mut destination,
                temp: &mut temp,
            };
            assert_eq!(
                writer
                    .try_reserve_then_write(|| {
                        compose_calls.set(compose_calls.get() + 1);
                        contact_words(50)
                    })
                    .expect("mid-flush write"),
                AssemblyGroundContactWriteOutcome::Written
            );
        }

        assert_eq!(compose_calls.get(), 1);
        assert_eq!(destination.count(), 50);
        assert_eq!(temp.count(), 1);
        temp.tail_flush_and_reset(&mut destination)
            .expect("publish new slot zero");
        assert_eq!(
            destination
                .occupied()
                .map(|contact| contact.test_words()[0][0])
                .collect::<Vec<_>>(),
            (0..51).collect::<Vec<_>>()
        );
    }

    #[test]
    fn assembly_ground_write_has_no_contact_validity_discard_route() {
        let mut destination = SimulationContactAppend::try_new(1).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(false, &destination).expect("temp buffer");
        let mut words = [[0_u32; 4]; 16];
        words[2][3] = 2.5_f32.to_bits();
        words[3][3] = (-2.0_f32).to_bits();
        words[4][3] = (-2.0_f32).to_bits();

        {
            let mut writer = AssemblyGroundContactWritePort {
                destination: &mut destination,
                temp: &mut temp,
            };
            assert_eq!(
                writer
                    .try_reserve_then_write(|| SimulationContactBuildInput::from_words(words))
                    .expect("ground write"),
                AssemblyGroundContactWriteOutcome::Written
            );
        }

        assert_eq!(temp.count(), 1);
        temp.tail_flush_and_reset(&mut destination)
            .expect("publish ground Contact");
        assert_eq!(destination.count(), 1);
        assert_eq!(destination.occupied().next().unwrap().test_words(), words);
    }

    #[test]
    fn store_zero_is_the_unique_no_atomic_no_copy_route() {
        let mut destination = SimulationContactAppend::try_new(4).expect("destination");
        destination.store_prefix(&[10, 11]).expect("seed");
        let before = destination.occupied().copied().collect::<Vec<_>>();
        let receipt = destination.store_prefix(&[]).expect("zero Store");

        assert_eq!(
            receipt,
            SimulationContactStoreReceipt {
                start: 0,
                accepted: 0,
                clamped: false,
            }
        );
        assert_eq!(destination.count(), 2);
        assert_eq!(destination.occupied().copied().collect::<Vec<_>>(), before);
    }

    #[test]
    fn store_under_exact_over_and_full_preserve_native_prefix_results() {
        let mut under = SimulationContactAppend::try_new(5).expect("under");
        assert_eq!(
            under.store_prefix(&[1, 2]).expect("under Store"),
            SimulationContactStoreReceipt {
                start: 0,
                accepted: 2,
                clamped: false,
            }
        );
        assert_eq!(under.occupied().copied().collect::<Vec<_>>(), [1, 2]);

        let mut exact = SimulationContactAppend::try_new(3).expect("exact");
        exact.store_prefix(&[4]).expect("seed");
        assert_eq!(
            exact.store_prefix(&[5, 6]).expect("exact Store"),
            SimulationContactStoreReceipt {
                start: 1,
                accepted: 2,
                clamped: true,
            }
        );
        assert_eq!(exact.occupied().copied().collect::<Vec<_>>(), [4, 5, 6]);

        let mut over = SimulationContactAppend::try_new(4).expect("over");
        over.store_prefix(&[7, 8]).expect("seed");
        assert_eq!(
            over.store_prefix(&[9, 10, 11]).expect("over Store"),
            SimulationContactStoreReceipt {
                start: 2,
                accepted: 2,
                clamped: true,
            }
        );
        assert_eq!(over.occupied().copied().collect::<Vec<_>>(), [7, 8, 9, 10]);
        assert_eq!(
            over.store_prefix(&[12]).expect("full Store"),
            SimulationContactStoreReceipt {
                start: 5,
                accepted: 0,
                clamped: true,
            }
        );
        assert_eq!(over.occupied().copied().collect::<Vec<_>>(), [7, 8, 9, 10]);
    }

    #[test]
    fn store_plan_preserves_alias_no_copy_and_signed_host_guard() {
        let alias = try_plan_contact_store(2, 5, 2, true).expect("alias plan");
        assert_eq!(alias.receipt.start, 2);
        assert_eq!(alias.receipt.accepted, 2);
        assert_eq!(alias.final_count, 4);
        assert_eq!(alias.copy_records, 0);
        assert_eq!(
            try_plan_contact_store(i32::MAX as u32 + 2, 0, 1, false),
            Err(SimulationContactStoreError::InvalidCounter {
                count: i32::MAX as u32 + 2,
                maximum: 0,
            })
        );

        assert!(matches!(
            SimulationContactAppend::<u32>::try_new(i32::MAX as u32 + 1),
            Err(SimulationContactAppendInitError::MaximumExceedsSignedRange { maximum })
                if maximum == i32::MAX as u32 + 1
        ));
        assert_eq!(
            try_plan_contact_store(u32::MAX, 0, 1, false),
            Err(SimulationContactStoreError::CounterOverflow {
                count: u32::MAX,
                requested: 1,
            })
        );
        assert_eq!(
            try_plan_contact_store(u32::MAX, 0, 0, false),
            Ok(SimulationContactStorePlan {
                receipt: SimulationContactStoreReceipt {
                    start: 0,
                    accepted: 0,
                    clamped: false,
                },
                final_count: u32::MAX,
                copy_records: 0,
            })
        );
        #[cfg(target_pointer_width = "64")]
        assert_eq!(
            try_plan_contact_store(0, 1, usize::MAX, false),
            Err(SimulationContactStoreError::RequestedCountOverflow {
                requested: usize::MAX,
            })
        );
        assert_eq!(
            try_plan_contact_store(i32::MAX as u32, i32::MAX as u32, u32::MAX as usize, false),
            Err(SimulationContactStoreError::CounterOverflow {
                count: i32::MAX as u32,
                requested: u32::MAX,
            })
        );

        let mut zero_maximum = SimulationContactAppend::try_new(0).expect("zero maximum");
        assert_eq!(
            zero_maximum.store_prefix(&[1]).expect("full zero Store"),
            SimulationContactStoreReceipt {
                start: 0,
                accepted: 0,
                clamped: true,
            }
        );
    }

    #[test]
    fn store_overshoot_survives_zero_then_next_nonempty_store_resets_counter() {
        let mut destination = SimulationContactAppend::try_new(10).expect("destination");
        destination.store_prefix(&[100, 101]).expect("seed");
        let source: Vec<_> = (0..50).collect();
        let receipt = destination
            .store_prefix(&source)
            .expect("overshooting Store");
        assert_eq!(receipt.start, 2);
        assert_eq!(receipt.accepted, 8);
        assert!(receipt.clamped);
        assert_eq!(destination.reservation_count, 52);
        assert_eq!(destination.count(), 10);
        let expected = [100, 101, 0, 1, 2, 3, 4, 5, 6, 7];
        assert_eq!(
            destination.occupied().copied().collect::<Vec<_>>(),
            expected
        );

        destination
            .store_prefix(&[])
            .expect("zero leaves raw overshoot intact");
        assert_eq!(destination.reservation_count, 52);
        let receipt = destination
            .store_prefix(&[200])
            .expect("negative remainder");
        assert_eq!(receipt.start, 52);
        assert_eq!(receipt.accepted, 0);
        assert_eq!(destination.reservation_count, 10);
        assert_eq!(destination.count(), 10);
        assert_eq!(
            destination.occupied().copied().collect::<Vec<_>>(),
            expected
        );
        let receipt = destination.store_prefix(&[201]).expect("zero remainder");
        assert_eq!(receipt.start, 10);
        assert_eq!(receipt.accepted, 0);
        assert_eq!(destination.reservation_count, 10);
    }

    #[test]
    fn store_signed_remainder_accepts_source_created_overshoot_above_signed_maximum() {
        let maximum = i32::MAX as u32;
        let first = try_plan_contact_store(maximum - 2, maximum, 50, false).unwrap();
        assert_eq!(first.receipt.accepted, 2);
        assert_eq!(first.final_count, maximum + 48);
        let next = try_plan_contact_store(first.final_count, maximum, 1, false).unwrap();
        assert_eq!(next.receipt.start, maximum + 48);
        assert_eq!(next.receipt.accepted, 0);
        assert_eq!(next.final_count, maximum);
    }

    #[test]
    fn ground_buffer_caught_composition_panic_poison_is_permanent() {
        let mut buffer = SimulationGroundContactBuffer::try_new(100, true).unwrap();
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = buffer.try_reserve_then_write(|| panic!("test composition panic"));
        }));
        assert!(unwind.is_err());
        assert_eq!(buffer.temp.count(), 1);
        assert_eq!(buffer.destination.count(), 0);
        assert!(matches!(
            buffer.try_reserve_then_write(|| panic!("must not reenter composition")),
            Err(SimulationGroundContactBufferError::Poisoned)
        ));
        assert!(matches!(
            buffer.try_finish_assembly(),
            Err(SimulationGroundContactBufferError::Poisoned)
        ));
        assert!(matches!(
            buffer.try_into_records(),
            Err(SimulationGroundContactBufferError::Poisoned)
        ));
    }

    #[test]
    fn temp_slot_49_fills_then_no_flush_route_returns_none_without_effect() {
        let mut destination = SimulationContactAppend::try_new(100).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(false, &destination).expect("temp buffer");
        fill_temp(&mut temp, &mut destination, 50);
        assert_eq!(temp.count(), 50);
        assert_eq!(destination.count(), 0);

        assert_eq!(
            temp.try_get_next_free_contact(&mut destination)
                .expect("full no-flush route"),
            None
        );
        assert_eq!(temp.count(), 50);
        assert_eq!(destination.count(), 0);
    }

    #[test]
    fn temp_full_flushes_50_then_reserves_slot_zero_at_count_one() {
        let mut destination = SimulationContactAppend::try_new(100).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(true, &destination).expect("temp buffer");
        fill_temp(&mut temp, &mut destination, 50);

        let next = temp
            .try_get_next_free_contact(&mut destination)
            .expect("full flush route")
            .expect("new slot");
        assert_eq!(
            next.flushed,
            Some(SimulationContactStoreReceipt {
                start: 0,
                accepted: 50,
                clamped: false,
            })
        );
        assert_eq!(next.slot.index, 0);
        assert_eq!(temp.count(), 1);
        assert_eq!(destination.count(), 50);
        assert_eq!(
            destination.occupied().copied().collect::<Vec<_>>(),
            (0..50).collect::<Vec<_>>()
        );
        temp.try_write_reserved(next.slot, 50)
            .expect("write new slot");
    }

    #[test]
    fn tail_flush_resets_even_when_global_store_truncates_the_prefix() {
        let mut destination = SimulationContactAppend::try_new(10).expect("destination");
        destination.store_prefix(&[100, 101]).expect("seed");
        let mut temp =
            SimulationTempContactBuffer::try_new(true, &destination).expect("temp buffer");
        fill_temp(&mut temp, &mut destination, 50);

        assert_eq!(
            temp.tail_flush_and_reset(&mut destination)
                .expect("tail flush"),
            Some(SimulationContactStoreReceipt {
                start: 2,
                accepted: 8,
                clamped: true,
            })
        );
        assert_eq!(temp.count(), 0);
        assert_eq!(destination.count(), 10);
        assert_eq!(
            destination.occupied().copied().collect::<Vec<_>>(),
            [100, 101, 0, 1, 2, 3, 4, 5, 6, 7]
        );
        assert_eq!(
            temp.tail_flush_and_reset(&mut destination)
                .expect("empty tail"),
            None
        );
    }

    #[test]
    fn mid_flush_ignores_global_truncation_and_still_reserves_slot_zero() {
        let mut destination = SimulationContactAppend::try_new(10).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(true, &destination).expect("temp buffer");
        fill_temp(&mut temp, &mut destination, 50);

        let next = temp
            .try_get_next_free_contact(&mut destination)
            .expect("mid flush")
            .expect("new slot");
        assert_eq!(
            next.flushed,
            Some(SimulationContactStoreReceipt {
                start: 0,
                accepted: 10,
                clamped: true,
            })
        );
        assert_eq!(next.slot.index, 0);
        assert_eq!(temp.count(), 1);
        assert_eq!(destination.count(), 10);
        assert_eq!(
            destination.occupied().copied().collect::<Vec<_>>(),
            (0..10).collect::<Vec<_>>()
        );
    }

    #[test]
    fn discard_last_matches_invalid_candidate_count_rollback() {
        let mut destination = SimulationContactAppend::try_new(50).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(false, &destination).expect("temp buffer");
        let first = temp
            .try_get_next_free_contact(&mut destination)
            .expect("reserve")
            .expect("slot");
        temp.try_write_reserved(first.slot, 7).expect("write");
        temp.try_discard_last(first.slot).expect("discard");
        assert_eq!(temp.count(), 0);

        let replacement = temp
            .try_get_next_free_contact(&mut destination)
            .expect("reserve replacement")
            .expect("replacement");
        assert_eq!(replacement.slot.index, 0);
        assert_eq!(replacement.flushed, None);
        assert_eq!(
            temp.try_write_reserved(first.slot, 8),
            Err(SimulationTempContactBufferError::StaleSlot)
        );
        temp.try_write_reserved(replacement.slot, 9)
            .expect("replacement write");
    }

    #[test]
    fn temp_tokens_are_buffer_bound_and_non_last_discard_is_inert() {
        let mut destination_a = SimulationContactAppend::try_new(50).expect("destination A");
        let mut destination_b = SimulationContactAppend::try_new(50).expect("destination B");
        let mut temp_a =
            SimulationTempContactBuffer::try_new(false, &destination_a).expect("temp A");
        let mut temp_b =
            SimulationTempContactBuffer::try_new(false, &destination_b).expect("temp B");
        let slot_a = temp_a
            .try_get_next_free_contact(&mut destination_a)
            .expect("reserve A")
            .expect("slot A")
            .slot;
        let slot_b = temp_b
            .try_get_next_free_contact(&mut destination_b)
            .expect("reserve B")
            .expect("slot B")
            .slot;
        assert_eq!(
            temp_b.try_write_reserved(slot_a, 1),
            Err(SimulationTempContactBufferError::StaleSlot)
        );
        temp_b.try_write_reserved(slot_b, 2).expect("write B");

        let second_a = temp_a
            .try_get_next_free_contact(&mut destination_a)
            .expect("reserve second A")
            .expect("second A")
            .slot;
        assert_eq!(
            temp_a.try_discard_last(slot_a),
            Err(SimulationTempContactBufferError::SlotOutOfOrder { index: 0, count: 2 })
        );
        assert_eq!(temp_a.count(), 2);
        temp_a.try_write_reserved(slot_a, 3).expect("write first A");
        temp_a
            .try_write_reserved(second_a, 4)
            .expect("write second A");
    }

    #[test]
    fn temp_buffer_rejects_a_foreign_destination_before_flush_effects() {
        let mut destination_a = SimulationContactAppend::try_new(100).expect("destination A");
        let mut destination_b = SimulationContactAppend::try_new(100).expect("destination B");
        let mut temp =
            SimulationTempContactBuffer::try_new(true, &destination_a).expect("temp buffer");
        fill_temp(&mut temp, &mut destination_a, 50);

        assert_eq!(
            temp.try_get_next_free_contact(&mut destination_b),
            Err(SimulationTempContactBufferError::DestinationMismatch)
        );
        assert_eq!(temp.count(), 50);
        assert_eq!(destination_a.count(), 0);
        assert_eq!(destination_b.count(), 0);
        assert_eq!(
            temp.tail_flush_and_reset(&mut destination_b),
            Err(SimulationTempContactBufferError::DestinationMismatch)
        );
        assert_eq!(temp.count(), 50);
        assert_eq!(destination_a.count(), 0);
        assert_eq!(destination_b.count(), 0);
    }

    #[test]
    fn exhausted_reservation_generation_refuses_before_a_required_mid_flush() {
        let mut below_destination: SimulationContactAppend<u32> =
            SimulationContactAppend::try_new(100).expect("below-full destination");
        let mut below = SimulationTempContactBuffer::try_new(true, &below_destination)
            .expect("below-full temp buffer");
        below.next_generation = u64::MAX;
        assert_eq!(
            below.try_get_next_free_contact(&mut below_destination),
            Err(SimulationTempContactBufferError::ReservationGenerationExhausted)
        );
        assert_eq!(below.count(), 0);
        assert_eq!(below_destination.count(), 0);

        let mut destination = SimulationContactAppend::try_new(100).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(true, &destination).expect("temp buffer");
        fill_temp(&mut temp, &mut destination, 50);
        temp.next_generation = u64::MAX;

        assert_eq!(
            temp.try_get_next_free_contact(&mut destination),
            Err(SimulationTempContactBufferError::ReservationGenerationExhausted)
        );
        assert_eq!(temp.count(), 50);
        assert_eq!(destination.count(), 0);
    }

    #[test]
    fn uninitialized_or_stale_temp_slots_cannot_publish() {
        let mut destination = SimulationContactAppend::try_new(100).expect("destination");
        let mut temp =
            SimulationTempContactBuffer::try_new(true, &destination).expect("temp buffer");
        let reservation = temp
            .try_get_next_free_contact(&mut destination)
            .expect("reserve")
            .expect("slot");
        assert_eq!(
            temp.tail_flush_and_reset(&mut destination),
            Err(SimulationTempContactBufferError::UninitializedSlot { index: 0 })
        );
        assert_eq!(temp.count(), 1);
        assert_eq!(destination.count(), 0);

        temp.try_write_reserved(reservation.slot, 1).expect("write");
        temp.tail_flush_and_reset(&mut destination).expect("flush");
        assert_eq!(
            temp.try_write_reserved(reservation.slot, 2),
            Err(SimulationTempContactBufferError::StaleSlot)
        );

        let first = temp
            .try_get_next_free_contact(&mut destination)
            .expect("reserve first")
            .expect("first");
        let _second = temp
            .try_get_next_free_contact(&mut destination)
            .expect("reserve second")
            .expect("second");
        temp.try_write_reserved(first.slot, 3).expect("write first");
        let before_destination = destination.occupied().copied().collect::<Vec<_>>();
        assert_eq!(
            temp.tail_flush_and_reset(&mut destination),
            Err(SimulationTempContactBufferError::UninitializedSlot { index: 1 })
        );
        assert_eq!(temp.count(), 2);
        assert_eq!(
            destination.occupied().copied().collect::<Vec<_>>(),
            before_destination
        );
    }
}
