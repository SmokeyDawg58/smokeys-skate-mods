//! Shared deferred-collection Pending/Active row ownership.
//!
//! `cAssetStreamManager::NotifyCollectionSimReg` and
//! `NotifyCollectionPresReg` append 48-byte `tCollectionReg` values to the
//! same fixed-capacity vector. `UpdateSimCollectionDefer` and
//! `UpdatePresCollectionDefer` then share one non-waiting atomic update gate,
//! one active-row mutex, one active record, and the same
//! `SetupNextActiveCollectionReg` tail.
//!
//! This module reconstructs that manager-wide ownership and ordering. It does
//! not model a native memory layout, invoke a callback, dispatch a queued
//! message, or claim cross-thread/frame order. In particular, a retired row's
//! subsystem-local queued messages may remain alive while Setup activates the
//! next pending row.

use crate::{
    collection_parent_manager::INVALID_PARENT_COLLECTION_ID,
    collection_resident_streamer::CollectionId,
};
use std::{
    error::Error,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// Native `tCollectionReg` byte size copied by six eight-byte moves.
pub const DEFERRED_COLLECTION_REG_NATIVE_ROW_SIZE: usize = 48;

/// Inline element count in the native fixed pending-row vector.
pub const DEFERRED_COLLECTION_REG_PENDING_CAPACITY: usize = 16;

/// Native active-row discriminator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DeferredCollectionRegType {
    Simulation = 1,
    Presentation = 2,
}

/// Sanitized owner for the fields populated before a deferred row enters the
/// native mixed pending vector. Index, maximum, and Collection pointer are
/// deliberately absent: native admission clears those fields and Setup fills
/// them only after destructive FIFO selection.
#[must_use]
pub struct DeferredCollectionRegRow<T> {
    collection_id: CollectionId,
    proxy_collection_id: CollectionId,
    userdata_raw: u32,
    registration_type: DeferredCollectionRegType,
    stream_size: u32,
    stream_type_raw: u32,
    payload: T,
}

impl<T> DeferredCollectionRegRow<T> {
    /// Builds the exact registration-row field class emitted by
    /// `NotifyCollectionSimReg`: registration type one and invalid proxy ID.
    pub const fn simulation(
        collection_id: CollectionId,
        userdata_raw: u32,
        stream_size: u32,
        stream_type_raw: u32,
        payload: T,
    ) -> Self {
        Self {
            collection_id,
            proxy_collection_id: INVALID_PARENT_COLLECTION_ID,
            userdata_raw,
            registration_type: DeferredCollectionRegType::Simulation,
            stream_size,
            stream_type_raw,
            payload,
        }
    }

    /// Builds the exact registration-row field class emitted by
    /// `NotifyCollectionPresReg`: registration type two and caller-supplied
    /// full proxy ID.
    pub const fn presentation(
        collection_id: CollectionId,
        proxy_collection_id: CollectionId,
        userdata_raw: u32,
        stream_size: u32,
        stream_type_raw: u32,
        payload: T,
    ) -> Self {
        Self {
            collection_id,
            proxy_collection_id,
            userdata_raw,
            registration_type: DeferredCollectionRegType::Presentation,
            stream_size,
            stream_type_raw,
            payload,
        }
    }

    #[must_use]
    pub const fn collection_id(&self) -> CollectionId {
        self.collection_id
    }

    #[must_use]
    pub const fn proxy_collection_id(&self) -> CollectionId {
        self.proxy_collection_id
    }

    #[must_use]
    pub const fn userdata_raw(&self) -> u32 {
        self.userdata_raw
    }

    #[must_use]
    pub const fn registration_type(&self) -> DeferredCollectionRegType {
        self.registration_type
    }

    #[must_use]
    pub const fn stream_size(&self) -> u32 {
        self.stream_size
    }

    #[must_use]
    pub const fn stream_type_raw(&self) -> u32 {
        self.stream_type_raw
    }

    #[must_use]
    pub const fn payload(&self) -> &T {
        &self.payload
    }

    #[must_use]
    pub fn payload_mut(&mut self) -> &mut T {
        &mut self.payload
    }

    #[must_use]
    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T: fmt::Debug> fmt::Debug for DeferredCollectionRegRow<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeferredCollectionRegRow")
            .field("collection_id", &self.collection_id)
            .field("proxy_collection_id", &self.proxy_collection_id)
            .field("userdata_raw", &self.userdata_raw)
            .field("registration_type", &self.registration_type)
            .field("stream_size", &self.stream_size)
            .field("stream_type_raw", &self.stream_type_raw)
            .field("payload", &self.payload)
            .finish()
    }
}

struct ActiveRow<T> {
    row: DeferredCollectionRegRow<T>,
    index: u32,
    maximum: u32,
}

struct SetupFault<T> {
    row: DeferredCollectionRegRow<T>,
    retired: Option<DeferredCollectionRegRow<T>>,
}

/// Read-only active-record fields visible to a matching updater action.
pub struct DeferredCollectionRegActive<'a, T> {
    active: &'a mut ActiveRow<T>,
}

impl<T> DeferredCollectionRegActive<'_, T> {
    #[must_use]
    pub const fn collection_id(&self) -> CollectionId {
        self.active.row.collection_id
    }

    #[must_use]
    pub const fn proxy_collection_id(&self) -> CollectionId {
        self.active.row.proxy_collection_id
    }

    #[must_use]
    pub const fn userdata_raw(&self) -> u32 {
        self.active.row.userdata_raw
    }

    #[must_use]
    pub const fn stream_size(&self) -> u32 {
        self.active.row.stream_size
    }

    #[must_use]
    pub const fn stream_type_raw(&self) -> u32 {
        self.active.row.stream_type_raw
    }

    #[must_use]
    pub const fn index(&self) -> u32 {
        self.active.index
    }

    #[must_use]
    pub const fn maximum(&self) -> u32 {
        self.active.maximum
    }

    #[must_use]
    pub fn payload_mut(&mut self) -> &mut T {
        &mut self.active.row.payload
    }
}

/// Copyable diagnostic view of the single active record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeferredCollectionRegActiveSnapshot {
    pub collection_id: CollectionId,
    pub proxy_collection_id: CollectionId,
    pub userdata_raw: u32,
    pub registration_type: DeferredCollectionRegType,
    pub stream_size: u32,
    pub stream_type_raw: u32,
    pub index: u32,
    pub maximum: u32,
}

/// Non-waiting manager-wide update gate shared by both subsystem updaters.
#[derive(Clone)]
pub struct DeferredCollectionRegUpdateGate {
    held: Arc<AtomicBool>,
}

impl DeferredCollectionRegUpdateGate {
    /// Attempts the native exchange-like zero-to-one acquisition. A busy gate
    /// is returned immediately and is not released by the refusing caller.
    #[must_use]
    pub fn try_enter(&self) -> Option<DeferredCollectionRegUpdateGuard> {
        if self.held.swap(true, Ordering::AcqRel) {
            None
        } else {
            Some(DeferredCollectionRegUpdateGuard {
                held: Arc::clone(&self.held),
            })
        }
    }
}

/// RAII release for a successful update-gate acquisition.
#[must_use]
pub struct DeferredCollectionRegUpdateGuard {
    held: Arc<AtomicBool>,
}

impl Drop for DeferredCollectionRegUpdateGuard {
    fn drop(&mut self) {
        self.held.store(false, Ordering::Release);
    }
}

/// Pending-vector admission refusal. Native fixed-vector exhaustion is an
/// assertion/allocation-policy boundary; the host retains the exact row.
#[must_use]
pub struct DeferredCollectionRegAdmissionFailure<T> {
    row: DeferredCollectionRegRow<T>,
}

impl<T> DeferredCollectionRegAdmissionFailure<T> {
    pub fn into_row(self) -> DeferredCollectionRegRow<T> {
        self.row
    }
}

impl<T> fmt::Debug for DeferredCollectionRegAdmissionFailure<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeferredCollectionRegAdmissionFailure")
            .field("capacity", &DEFERRED_COLLECTION_REG_PENDING_CAPACITY)
            .finish_non_exhaustive()
    }
}

impl<T> fmt::Display for DeferredCollectionRegAdmissionFailure<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "deferred Collection registration pending vector reached fixed capacity {DEFERRED_COLLECTION_REG_PENDING_CAPACITY}"
        )
    }
}

impl<T> Error for DeferredCollectionRegAdmissionFailure<T> {}

/// Work attempted against the active record before the common Setup tail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredCollectionRegActiveEffect {
    NoActiveRow,
    OtherRegistrationType {
        active: DeferredCollectionRegType,
    },
    AlreadyComplete {
        registration_type: DeferredCollectionRegType,
    },
    Advanced {
        registration_type: DeferredCollectionRegType,
        count: u32,
        reached_maximum: bool,
    },
}

/// Result of the single native Setup attempt at an entered updater's tail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredCollectionRegSetupEffect {
    ActiveIncomplete,
    PendingEmpty,
    Activated {
        registration_type: DeferredCollectionRegType,
        maximum: u32,
    },
}

/// Successful result of one Simulation- or Presentation-side updater call.
#[must_use]
pub enum DeferredCollectionRegManagerUpdate<T> {
    GateBusy,
    Entered {
        active: DeferredCollectionRegActiveEffect,
        setup: DeferredCollectionRegSetupEffect,
        retired: Option<DeferredCollectionRegRow<T>>,
    },
}

impl<T> DeferredCollectionRegManagerUpdate<T> {
    #[must_use]
    pub fn into_retired(self) -> Option<DeferredCollectionRegRow<T>> {
        match self {
            Self::GateBusy => None,
            Self::Entered { retired, .. } => retired,
        }
    }
}

impl<T> fmt::Debug for DeferredCollectionRegManagerUpdate<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GateBusy => formatter.write_str("GateBusy"),
            Self::Entered {
                active,
                setup,
                retired,
            } => formatter
                .debug_struct("Entered")
                .field("active", active)
                .field("setup", setup)
                .field(
                    "retired",
                    &retired.as_ref().map(|row| row.registration_type),
                )
                .finish(),
        }
    }
}

/// Safe-host refusal while an entered update owns the manager state.
#[derive(Debug)]
pub enum DeferredCollectionRegManagerError<E> {
    Faulted,
    Active(E),
    Setup(E),
    AdvanceExceedsRemaining { count: u32, remaining: u32 },
}

impl<E: fmt::Display> fmt::Display for DeferredCollectionRegManagerError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Faulted => {
                formatter.write_str("deferred Collection registration manager faulted")
            }
            Self::Active(source) => write!(formatter, "active deferred update failed: {source}"),
            Self::Setup(source) => write!(formatter, "deferred Setup failed: {source}"),
            Self::AdvanceExceedsRemaining { count, remaining } => write!(
                formatter,
                "active deferred update advanced {count} Arenas with only {remaining} remaining"
            ),
        }
    }
}

impl<E: Error + 'static> Error for DeferredCollectionRegManagerError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Active(source) | Self::Setup(source) => Some(source),
            Self::Faulted | Self::AdvanceExceedsRemaining { .. } => None,
        }
    }
}

/// Owning manager-wide mixed pending/active row state.
#[must_use]
pub struct DeferredCollectionRegManager<T> {
    pending: Vec<DeferredCollectionRegRow<T>>,
    active: Option<ActiveRow<T>>,
    setup_fault: Option<SetupFault<T>>,
    invariant_fault: bool,
    gate: DeferredCollectionRegUpdateGate,
}

impl<T> Default for DeferredCollectionRegManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> DeferredCollectionRegManager<T> {
    pub fn new() -> Self {
        Self {
            pending: Vec::with_capacity(DEFERRED_COLLECTION_REG_PENDING_CAPACITY),
            active: None,
            setup_fault: None,
            invariant_fault: false,
            gate: DeferredCollectionRegUpdateGate {
                held: Arc::new(AtomicBool::new(false)),
            },
        }
    }

    #[must_use]
    pub fn update_gate(&self) -> DeferredCollectionRegUpdateGate {
        self.gate.clone()
    }

    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    #[must_use]
    pub const fn is_faulted(&self) -> bool {
        self.setup_fault.is_some() || self.invariant_fault
    }

    /// Returns the row destructively selected before a Setup lookup refusal.
    /// An invariant fault has no such row.
    #[must_use]
    pub fn setup_faulted_row(&self) -> Option<&DeferredCollectionRegRow<T>> {
        self.setup_fault.as_ref().map(|fault| &fault.row)
    }

    /// Returns the completed active row that Setup overwrote before its
    /// selected pending row failed initialization.
    #[must_use]
    pub fn setup_faulted_retired_row(&self) -> Option<&DeferredCollectionRegRow<T>> {
        self.setup_fault
            .as_ref()
            .and_then(|fault| fault.retired.as_ref())
    }

    #[must_use]
    pub fn active_snapshot(&self) -> Option<DeferredCollectionRegActiveSnapshot> {
        self.active
            .as_ref()
            .map(|active| DeferredCollectionRegActiveSnapshot {
                collection_id: active.row.collection_id,
                proxy_collection_id: active.row.proxy_collection_id,
                userdata_raw: active.row.userdata_raw,
                registration_type: active.row.registration_type,
                stream_size: active.row.stream_size,
                stream_type_raw: active.row.stream_type_raw,
                index: active.index,
                maximum: active.maximum,
            })
    }

    /// Borrows the host payload retained beside the native-shaped active-row
    /// fields. This does not change index, maximum, or Setup eligibility.
    #[must_use]
    pub fn active_payload_mut(&mut self) -> Option<&mut T> {
        self.active.as_mut().map(|active| &mut active.row.payload)
    }

    /// Appends one already-admitted row to the shared mixed FIFO.
    ///
    /// # Errors
    ///
    /// Returns the unchanged row when the fixed pending capacity is full.
    pub fn enqueue(
        &mut self,
        row: DeferredCollectionRegRow<T>,
    ) -> Result<(), DeferredCollectionRegAdmissionFailure<T>> {
        if self.pending.len() == DEFERRED_COLLECTION_REG_PENDING_CAPACITY {
            return Err(DeferredCollectionRegAdmissionFailure { row });
        }
        self.pending.push(row);
        Ok(())
    }

    /// Runs one exact updater class through the shared non-waiting gate and
    /// common Setup tail.
    ///
    /// `update_active` is called only for an unfinished active row with the
    /// requested registration type and returns the number of Arena ordinals
    /// advanced by that invocation. `setup` is called only after destructive
    /// FIFO removal of one pending row and returns the Collection Arena count
    /// installed as the active maximum. This mirrors native lookup occurring
    /// after the first pending value has already been copied and erased.
    ///
    /// A row retired at the Setup boundary is returned to the caller. This is
    /// safe-host ownership for subsystem-local staged messages: Setup may
    /// activate a new row even while the returned row remains undrained.
    ///
    /// # Errors
    ///
    /// Active-operation refusal preserves the active row. Setup refusal
    /// retains the destructively selected row in a sticky diagnostic state;
    /// subsequent updates return [`DeferredCollectionRegManagerError::Faulted`].
    pub fn try_update<F, S, E>(
        &mut self,
        updater: DeferredCollectionRegType,
        update_active: F,
        setup: S,
    ) -> Result<DeferredCollectionRegManagerUpdate<T>, DeferredCollectionRegManagerError<E>>
    where
        F: FnOnce(DeferredCollectionRegActive<'_, T>) -> Result<u32, E>,
        S: FnOnce(&mut DeferredCollectionRegRow<T>) -> Result<u32, E>,
    {
        if self.is_faulted() {
            return Err(DeferredCollectionRegManagerError::Faulted);
        }
        let Some(_guard) = self.gate.try_enter() else {
            return Ok(DeferredCollectionRegManagerUpdate::GateBusy);
        };

        let active_effect = match self.active.as_mut() {
            None => DeferredCollectionRegActiveEffect::NoActiveRow,
            Some(active) if active.row.registration_type != updater => {
                DeferredCollectionRegActiveEffect::OtherRegistrationType {
                    active: active.row.registration_type,
                }
            }
            Some(active) if active.index >= active.maximum => {
                DeferredCollectionRegActiveEffect::AlreadyComplete {
                    registration_type: active.row.registration_type,
                }
            }
            Some(active) => {
                let remaining = active.maximum - active.index;
                let count = update_active(DeferredCollectionRegActive { active })
                    .map_err(DeferredCollectionRegManagerError::Active)?;
                if count > remaining {
                    self.invariant_fault = true;
                    return Err(DeferredCollectionRegManagerError::AdvanceExceedsRemaining {
                        count,
                        remaining,
                    });
                }
                active.index += count;
                DeferredCollectionRegActiveEffect::Advanced {
                    registration_type: active.row.registration_type,
                    count,
                    reached_maximum: active.index == active.maximum,
                }
            }
        };

        let active_complete = self
            .active
            .as_ref()
            .is_none_or(|active| active.index >= active.maximum);
        let mut retired = None;
        let setup_effect = if !active_complete {
            DeferredCollectionRegSetupEffect::ActiveIncomplete
        } else if self.pending.is_empty() {
            DeferredCollectionRegSetupEffect::PendingEmpty
        } else {
            // Native overwrites the completed active record only when Setup
            // actually selected a pending row. An empty pending vector leaves
            // that completed record installed for the next invocation.
            retired = self.active.take().map(|active| active.row);
            let mut row = self.pending.remove(0);
            let maximum = match setup(&mut row) {
                Ok(maximum) => maximum,
                Err(source) => {
                    self.setup_fault = Some(SetupFault { row, retired });
                    return Err(DeferredCollectionRegManagerError::Setup(source));
                }
            };
            let registration_type = row.registration_type;
            self.active = Some(ActiveRow {
                row,
                index: 0,
                maximum,
            });
            DeferredCollectionRegSetupEffect::Activated {
                registration_type,
                maximum,
            }
        };

        Ok(DeferredCollectionRegManagerUpdate::Entered {
            active: active_effect,
            setup: setup_effect,
            retired,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    #[derive(Debug, Eq, PartialEq)]
    struct Payload(&'static str);

    fn sim(name: &'static str) -> DeferredCollectionRegRow<Payload> {
        DeferredCollectionRegRow::simulation(CollectionId(0x100), 8, 0x90000, 0, Payload(name))
    }

    fn pres(name: &'static str) -> DeferredCollectionRegRow<Payload> {
        DeferredCollectionRegRow::presentation(
            CollectionId(0x100),
            CollectionId(0x200),
            8,
            0x90000,
            0,
            Payload(name),
        )
    }

    fn entered<T>(
        update: DeferredCollectionRegManagerUpdate<T>,
    ) -> (
        DeferredCollectionRegActiveEffect,
        DeferredCollectionRegSetupEffect,
        Option<DeferredCollectionRegRow<T>>,
    ) {
        match update {
            DeferredCollectionRegManagerUpdate::GateBusy => panic!("update gate was busy"),
            DeferredCollectionRegManagerUpdate::Entered {
                active,
                setup,
                retired,
            } => (active, setup, retired),
        }
    }

    #[test]
    fn row_constructors_freeze_native_pending_fields() {
        assert_eq!(DEFERRED_COLLECTION_REG_NATIVE_ROW_SIZE, 48);
        assert_eq!(DEFERRED_COLLECTION_REG_PENDING_CAPACITY, 16);
        let simulation = sim("sim");
        assert_eq!(
            simulation.registration_type(),
            DeferredCollectionRegType::Simulation
        );
        assert_eq!(
            simulation.proxy_collection_id(),
            INVALID_PARENT_COLLECTION_ID
        );
        let presentation = pres("pres");
        assert_eq!(
            presentation.registration_type(),
            DeferredCollectionRegType::Presentation
        );
        assert_eq!(presentation.proxy_collection_id(), CollectionId(0x200));
        assert_eq!(presentation.collection_id(), CollectionId(0x100));
        assert_eq!(presentation.userdata_raw(), 8);
        assert_eq!(presentation.stream_size(), 0x90000);
        assert_eq!(presentation.stream_type_raw(), 0);
    }

    #[test]
    fn pending_vector_is_mixed_fixed_capacity_fifo() {
        let mut manager = DeferredCollectionRegManager::new();
        for index in 0..DEFERRED_COLLECTION_REG_PENDING_CAPACITY {
            let row = if index % 2 == 0 {
                sim("sim")
            } else {
                pres("pres")
            };
            manager.enqueue(row).unwrap();
        }
        let refusal = manager.enqueue(sim("overflow")).unwrap_err();
        assert_eq!(refusal.into_row().into_payload(), Payload("overflow"));
        assert_eq!(manager.pending_count(), 16);

        let update = manager
            .try_update(
                DeferredCollectionRegType::Presentation,
                |_| -> Result<u32, Infallible> { panic!("no active row exists") },
                |_| Ok(3),
            )
            .unwrap();
        let (active, setup, retired) = entered(update);
        assert_eq!(active, DeferredCollectionRegActiveEffect::NoActiveRow);
        assert_eq!(
            setup,
            DeferredCollectionRegSetupEffect::Activated {
                registration_type: DeferredCollectionRegType::Simulation,
                maximum: 3,
            }
        );
        assert!(retired.is_none());
        assert_eq!(manager.pending_count(), 15);
    }

    #[test]
    fn other_updater_leaves_unfinished_active_row_and_pending_fifo_untouched() {
        let mut manager = DeferredCollectionRegManager::new();
        manager.enqueue(sim("first")).unwrap();
        manager.enqueue(pres("second")).unwrap();
        let _ = manager
            .try_update(
                DeferredCollectionRegType::Presentation,
                |_| -> Result<u32, Infallible> { panic!("Setup-only call") },
                |_| Ok(4),
            )
            .unwrap();

        let mut active_calls = 0;
        let mut setup_calls = 0;
        let update = manager
            .try_update(
                DeferredCollectionRegType::Presentation,
                |_| {
                    active_calls += 1;
                    Ok::<u32, Infallible>(1)
                },
                |_| {
                    setup_calls += 1;
                    Ok(9)
                },
            )
            .unwrap();
        let (active, setup, retired) = entered(update);
        assert_eq!(
            active,
            DeferredCollectionRegActiveEffect::OtherRegistrationType {
                active: DeferredCollectionRegType::Simulation,
            }
        );
        assert_eq!(setup, DeferredCollectionRegSetupEffect::ActiveIncomplete);
        assert!(retired.is_none());
        assert_eq!((active_calls, setup_calls), (0, 0));
        assert_eq!(manager.pending_count(), 1);
        assert_eq!(manager.active_snapshot().unwrap().index, 0);
    }

    #[test]
    fn completing_simulation_row_activates_presentation_before_retired_queue_drains() {
        let mut manager = DeferredCollectionRegManager::new();
        manager.enqueue(sim("sim queued messages")).unwrap();
        manager.enqueue(pres("pres pending")).unwrap();
        let _ = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Infallible> { panic!("Setup-only call") },
                |_| Ok(3),
            )
            .unwrap();

        let update = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |mut active| {
                    assert_eq!(active.index(), 0);
                    assert_eq!(active.maximum(), 3);
                    assert_eq!(active.payload_mut(), &mut Payload("sim queued messages"));
                    Ok::<u32, Infallible>(3)
                },
                |row| {
                    assert_eq!(row.payload(), &Payload("pres pending"));
                    Ok(5)
                },
            )
            .unwrap();
        let (active, setup, retired) = entered(update);
        assert_eq!(
            active,
            DeferredCollectionRegActiveEffect::Advanced {
                registration_type: DeferredCollectionRegType::Simulation,
                count: 3,
                reached_maximum: true,
            }
        );
        assert_eq!(
            setup,
            DeferredCollectionRegSetupEffect::Activated {
                registration_type: DeferredCollectionRegType::Presentation,
                maximum: 5,
            }
        );
        assert_eq!(
            retired.unwrap().into_payload(),
            Payload("sim queued messages")
        );
        assert_eq!(
            manager.active_snapshot().unwrap().registration_type,
            DeferredCollectionRegType::Presentation
        );
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn completed_active_row_remains_installed_until_setup_finds_a_pending_row() {
        let mut manager = DeferredCollectionRegManager::new();
        manager.enqueue(sim("completed but installed")).unwrap();
        let _ = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Infallible> { panic!("Setup-only call") },
                |_| Ok(1),
            )
            .unwrap();
        let completion = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| Ok::<u32, Infallible>(1),
                |_| -> Result<u32, Infallible> { panic!("pending vector is empty") },
            )
            .unwrap();
        let (active, setup, retired) = entered(completion);
        assert_eq!(
            active,
            DeferredCollectionRegActiveEffect::Advanced {
                registration_type: DeferredCollectionRegType::Simulation,
                count: 1,
                reached_maximum: true,
            }
        );
        assert_eq!(setup, DeferredCollectionRegSetupEffect::PendingEmpty);
        assert!(retired.is_none());
        assert_eq!(manager.active_snapshot().unwrap().index, 1);

        manager.enqueue(pres("new pending")).unwrap();
        let overwrite = manager
            .try_update(
                DeferredCollectionRegType::Presentation,
                |_| -> Result<u32, Infallible> { panic!("completed row cannot advance") },
                |_| Ok(2),
            )
            .unwrap();
        let (active, setup, retired) = entered(overwrite);
        assert_eq!(
            active,
            DeferredCollectionRegActiveEffect::OtherRegistrationType {
                active: DeferredCollectionRegType::Simulation,
            }
        );
        assert_eq!(
            setup,
            DeferredCollectionRegSetupEffect::Activated {
                registration_type: DeferredCollectionRegType::Presentation,
                maximum: 2,
            }
        );
        assert_eq!(
            retired.unwrap().into_payload(),
            Payload("completed but installed")
        );
    }

    #[test]
    fn setup_activates_at_most_one_zero_arena_row_per_entered_update() {
        let mut manager = DeferredCollectionRegManager::new();
        manager.enqueue(sim("empty")).unwrap();
        manager.enqueue(pres("next")).unwrap();
        let first = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Infallible> { panic!("Setup-only call") },
                |_| Ok(0),
            )
            .unwrap();
        assert_eq!(
            entered(first).1,
            DeferredCollectionRegSetupEffect::Activated {
                registration_type: DeferredCollectionRegType::Simulation,
                maximum: 0,
            }
        );
        assert_eq!(manager.pending_count(), 1);

        let second = manager
            .try_update(
                DeferredCollectionRegType::Presentation,
                |_| -> Result<u32, Infallible> { panic!("zero row is already complete") },
                |_| Ok(7),
            )
            .unwrap();
        let (active, setup, retired) = entered(second);
        assert_eq!(
            active,
            DeferredCollectionRegActiveEffect::OtherRegistrationType {
                active: DeferredCollectionRegType::Simulation,
            }
        );
        assert_eq!(
            setup,
            DeferredCollectionRegSetupEffect::Activated {
                registration_type: DeferredCollectionRegType::Presentation,
                maximum: 7,
            }
        );
        assert_eq!(retired.unwrap().into_payload(), Payload("empty"));
    }

    #[test]
    fn shared_busy_gate_skips_both_updaters_without_setup_or_active_work() {
        let mut manager = DeferredCollectionRegManager::new();
        manager.enqueue(sim("sim")).unwrap();
        let gate = manager.update_gate();
        let _held = gate.try_enter().unwrap();
        for updater in [
            DeferredCollectionRegType::Simulation,
            DeferredCollectionRegType::Presentation,
        ] {
            let update = manager
                .try_update(
                    updater,
                    |_| -> Result<u32, Infallible> { panic!("busy gate ran active work") },
                    |_| -> Result<u32, Infallible> { panic!("busy gate ran Setup") },
                )
                .unwrap();
            assert!(matches!(
                update,
                DeferredCollectionRegManagerUpdate::GateBusy
            ));
        }
        assert_eq!(manager.pending_count(), 1);
        assert!(manager.active_snapshot().is_none());
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Refusal;

    impl fmt::Display for Refusal {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("refused")
        }
    }

    impl Error for Refusal {}

    #[test]
    fn destructive_setup_failure_latches_selected_row_and_releases_gate() {
        let mut manager = DeferredCollectionRegManager::new();
        manager.enqueue(sim("selected")).unwrap();
        manager.enqueue(pres("still pending")).unwrap();
        let error = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Refusal> { panic!("no active row") },
                |_| Err(Refusal),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            DeferredCollectionRegManagerError::Setup(Refusal)
        ));
        assert!(manager.is_faulted());
        assert_eq!(manager.pending_count(), 1);
        assert!(manager.update_gate().try_enter().is_some());
        assert!(matches!(
            manager.try_update(
                DeferredCollectionRegType::Presentation,
                |_| -> Result<u32, Refusal> { panic!("faulted manager ran active work") },
                |_| -> Result<u32, Refusal> { panic!("faulted manager ran Setup") },
            ),
            Err(DeferredCollectionRegManagerError::Faulted)
        ));
        assert_eq!(
            manager.setup_fault.as_ref().unwrap().row.payload,
            Payload("selected")
        );
        assert!(manager.setup_faulted_retired_row().is_none());
    }

    #[test]
    fn failed_setup_retains_the_completed_row_it_overwrote() {
        let mut manager = DeferredCollectionRegManager::new();
        manager.enqueue(sim("active")).unwrap();
        manager.enqueue(pres("selected")).unwrap();
        let _ = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Refusal> { panic!("Setup-only call") },
                |_| Ok(1),
            )
            .unwrap();
        let error = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| Ok::<u32, Refusal>(1),
                |_| Err(Refusal),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            DeferredCollectionRegManagerError::Setup(Refusal)
        ));
        assert_eq!(
            manager.setup_faulted_row().unwrap().payload(),
            &Payload("selected")
        );
        assert_eq!(
            manager.setup_faulted_retired_row().unwrap().payload(),
            &Payload("active")
        );
    }

    #[test]
    fn active_refusal_is_retryable_and_setup_is_not_run() {
        let mut manager = DeferredCollectionRegManager::new();
        manager.enqueue(sim("active")).unwrap();
        let _ = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Refusal> { panic!("Setup-only call") },
                |_| Ok(2),
            )
            .unwrap();
        let error = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| Err(Refusal),
                |_| -> Result<u32, Refusal> { panic!("refused active work reached Setup") },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            DeferredCollectionRegManagerError::Active(Refusal)
        ));
        assert_eq!(manager.active_snapshot().unwrap().index, 0);
        assert!(!manager.is_faulted());

        let retry = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| Ok::<u32, Refusal>(1),
                |_| -> Result<u32, Refusal> { panic!("active remains incomplete") },
            )
            .unwrap();
        assert_eq!(
            entered(retry).0,
            DeferredCollectionRegActiveEffect::Advanced {
                registration_type: DeferredCollectionRegType::Simulation,
                count: 1,
                reached_maximum: false,
            }
        );
    }

    #[test]
    fn impossible_advance_latches_before_setup_and_cannot_replay() {
        let mut manager = DeferredCollectionRegManager::new();
        manager.enqueue(sim("active")).unwrap();
        let _ = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Refusal> { panic!("Setup-only call") },
                |_| Ok(2),
            )
            .unwrap();
        let error = manager
            .try_update(
                DeferredCollectionRegType::Simulation,
                |_| Ok::<u32, Refusal>(3),
                |_| -> Result<u32, Refusal> { panic!("invalid advance reached Setup") },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            DeferredCollectionRegManagerError::AdvanceExceedsRemaining {
                count: 3,
                remaining: 2,
            }
        ));
        assert!(manager.is_faulted());
        assert_eq!(manager.active_snapshot().unwrap().index, 0);
        assert!(matches!(
            manager.try_update(
                DeferredCollectionRegType::Simulation,
                |_| -> Result<u32, Refusal> { panic!("faulted manager replayed active work") },
                |_| -> Result<u32, Refusal> { panic!("faulted manager reached Setup") },
            ),
            Err(DeferredCollectionRegManagerError::Faulted)
        ));
    }
}
