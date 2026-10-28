//! Owner-bound runtime routing for pointer-backed Simulation kinds 11 and 16.
//!
//! This adapter is deliberately a subsequence, not a complete
//! `Simulation::RegArena` (`0x828a01e0`) implementation. It preserves the
//! relative order of kind-11 DMO and kind-16 trigger operations while skipping
//! every other normal-plan kind. A later unified adapter must preserve their
//! global interleaving.

use crate::simulation_normal_arena_plan::{
    SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRecordHandle,
    SimulationRecordSourceKind,
};
use std::{error::Error, fmt, sync::Arc, sync::Weak};

const DMO_USERDATA_MASK: u32 = 8;

/// Exact void-returning manager calls reached by kinds 11 and 16.
///
/// Kind-11 ordering is owned by the adapter: Living World model add/remove is
/// always called before the matching DMO-manager call. Implementations receive
/// an owned source authority which they may clone when native manager state
/// retains the record beyond the call.
pub trait SimulationPointerArenaDispatch {
    fn add_dmo_model(&mut self, asset_id: u64);
    fn register_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64);
    fn remove_dmo_model(&mut self, asset_id: u64);
    fn deregister_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64);
    fn add_trigger_volume(&mut self, source: &SimulationRecordHandle);
    fn remove_trigger_volume(&mut self, source: &SimulationRecordHandle);
}

#[derive(Clone, Debug)]
enum PreparedPointerOperation {
    Dmo {
        source: SimulationRecordHandle,
        asset_id: u64,
    },
    Trigger {
        source: SimulationRecordHandle,
    },
}

#[derive(Debug)]
struct PreparedSimulationPointerArenaInner {
    plan: Arc<SimulationNormalArenaPlan>,
    operations: Vec<PreparedPointerOperation>,
    has_dmo: bool,
}

/// Prevalidated pointer-backed subsequence for one owned normal arena plan.
pub struct PreparedSimulationPointerArena {
    inner: Arc<PreparedSimulationPointerArenaInner>,
}

impl fmt::Debug for PreparedSimulationPointerArena {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedSimulationPointerArena")
            .field(
                "asset_id",
                &self.inner.plan.static_plan().asset().decoded.asset_id,
            )
            .field("operation_count", &self.inner.operations.len())
            .finish()
    }
}

#[derive(Debug)]
struct PreparedSimulationPointerArenaBankInner {
    arenas: Vec<Arc<PreparedSimulationPointerArenaInner>>,
    operation_count: usize,
    has_dmo: bool,
}

/// One preallocated, source-owning pointer subsequence for a caller-supplied bank.
///
/// The bank preserves caller-supplied arena order. Construction clones every
/// already-prepared source owner before the registration adapter can emit its
/// first manager call.
pub struct PreparedSimulationPointerArenaBank {
    inner: Arc<PreparedSimulationPointerArenaBankInner>,
}

impl fmt::Debug for PreparedSimulationPointerArenaBank {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedSimulationPointerArenaBank")
            .field("arena_count", &self.inner.arenas.len())
            .field("operation_count", &self.inner.operation_count)
            .finish()
    }
}

impl PreparedSimulationPointerArenaBank {
    /// Retains a caller-supplied ordered set of previously validated arenas.
    ///
    /// # Errors
    ///
    /// Returns a typed host storage or operation-count overflow error before
    /// retaining a partial bank.
    pub fn try_from_prepared(
        arenas: &[PreparedSimulationPointerArena],
    ) -> Result<Self, SimulationPointerArenaBankPrepareError> {
        let requested = arenas.len();
        let mut retained = Vec::new();
        retained.try_reserve_exact(requested).map_err(|_| {
            SimulationPointerArenaBankPrepareError::ArenaStorageAllocation { requested }
        })?;
        let mut operation_count = 0_usize;
        let mut has_dmo = false;
        for arena in arenas {
            operation_count = operation_count
                .checked_add(arena.inner.operations.len())
                .ok_or(SimulationPointerArenaBankPrepareError::OperationCountOverflow)?;
            has_dmo |= arena.inner.has_dmo;
            retained.push(Arc::clone(&arena.inner));
        }
        Ok(Self {
            inner: Arc::new(PreparedSimulationPointerArenaBankInner {
                arenas: retained,
                operation_count,
                has_dmo,
            }),
        })
    }

    #[must_use]
    pub fn arena_count(&self) -> usize {
        self.inner.arenas.len()
    }

    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.inner.operation_count
    }
}

impl PreparedSimulationPointerArena {
    /// Validates every kind-11/16 source before runtime effects are possible.
    ///
    /// # Errors
    ///
    /// Returns a typed error for host storage exhaustion, a missing or
    /// wrong-kind source, a wrong fixed record width, or disagreement between
    /// the operation's ID lanes and its retained serialized record.
    pub fn try_new(
        plan: Arc<SimulationNormalArenaPlan>,
    ) -> Result<Self, SimulationPointerArenaPrepareError> {
        let requested = plan
            .operations()
            .iter()
            .filter(|operation| {
                matches!(
                    operation,
                    SimulationNormalOperation::DmoInstance { .. }
                        | SimulationNormalOperation::TriggerInstance { .. }
                )
            })
            .count();
        let mut operations = Vec::new();
        operations.try_reserve_exact(requested).map_err(|_| {
            SimulationPointerArenaPrepareError::OperationStorageAllocation { requested }
        })?;
        let mut has_dmo = false;
        for (normal_operation_index, operation) in plan.operations().iter().enumerate() {
            let (expected_kind, expected_len, primary_id, local_id, asset_id) = match *operation {
                SimulationNormalOperation::DmoInstance {
                    asset_id,
                    local_guid,
                    ..
                } => {
                    has_dmo = true;
                    (
                        SimulationRecordSourceKind::DmoInstance,
                        160,
                        asset_id,
                        local_guid,
                        Some(asset_id),
                    )
                }
                SimulationNormalOperation::TriggerInstance {
                    asset_id,
                    primary_guid,
                    ..
                } => (
                    SimulationRecordSourceKind::TriggerInstance,
                    240,
                    primary_guid,
                    asset_id,
                    None,
                ),
                _ => continue,
            };
            let source = plan.record_handle(normal_operation_index).ok_or(
                SimulationPointerArenaPrepareError::MissingRecordSource {
                    normal_operation_index,
                },
            )?;
            if source.kind() != expected_kind {
                return Err(SimulationPointerArenaPrepareError::WrongRecordKind {
                    normal_operation_index,
                    expected: expected_kind,
                    actual: source.kind(),
                });
            }
            if source.bytes().len() != expected_len {
                return Err(SimulationPointerArenaPrepareError::WrongRecordLength {
                    normal_operation_index,
                    expected: expected_len,
                    actual: source.bytes().len(),
                });
            }
            let (primary_offset, local_offset) = match expected_kind {
                SimulationRecordSourceKind::DmoInstance => (96, 104),
                SimulationRecordSourceKind::TriggerInstance => (176, 184),
            };
            if read_u64(source.bytes(), primary_offset) != Some(primary_id)
                || read_u64(source.bytes(), local_offset) != Some(local_id)
            {
                return Err(SimulationPointerArenaPrepareError::RecordIdentityMismatch {
                    normal_operation_index,
                });
            }
            operations.push(match asset_id {
                Some(asset_id) => PreparedPointerOperation::Dmo { source, asset_id },
                None => PreparedPointerOperation::Trigger { source },
            });
        }
        Ok(Self {
            inner: Arc::new(PreparedSimulationPointerArenaInner {
                plan,
                operations,
                has_dmo,
            }),
        })
    }

    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.inner.operations.len()
    }
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_be_bytes(
        bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?,
    ))
}

#[derive(Debug)]
struct LivePointerRegistration {
    id: u64,
    prepared: Arc<PreparedSimulationPointerArenaInner>,
    registration_userdata_raw: u32,
}

#[derive(Debug)]
struct LivePointerBankRegistration {
    id: u64,
    prepared: Arc<PreparedSimulationPointerArenaBankInner>,
    registration_userdata_raw: u32,
}

/// Routes pointer-backed operations and owns their source lifetimes.
///
/// `dispatch` is declared before the source-holding ledgers so Rust destroys
/// manager state before any still-referenced arena bytes. No implicit manager
/// call is made from `Drop`.
pub struct SimulationPointerArenaRegistrationAdapter<D> {
    dispatch: D,
    live: Vec<LivePointerRegistration>,
    live_banks: Vec<LivePointerBankRegistration>,
    orphaned_sources: Vec<Arc<PreparedSimulationPointerArenaInner>>,
    orphaned_banks: Vec<Arc<PreparedSimulationPointerArenaBankInner>>,
    domain: Arc<()>,
    next_registration_id: u64,
}

impl<D: fmt::Debug> fmt::Debug for SimulationPointerArenaRegistrationAdapter<D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationPointerArenaRegistrationAdapter")
            .field("dispatch", &self.dispatch)
            .field("live_registration_count", &self.live.len())
            .field("live_bank_registration_count", &self.live_banks.len())
            .field("orphaned_source_count", &self.orphaned_sources.len())
            .field("orphaned_bank_count", &self.orphaned_banks.len())
            .field("next_registration_id", &self.next_registration_id)
            .finish_non_exhaustive()
    }
}

impl<D: SimulationPointerArenaDispatch> SimulationPointerArenaRegistrationAdapter<D> {
    #[must_use]
    pub fn new(dispatch: D) -> Self {
        Self {
            dispatch,
            live: Vec::new(),
            live_banks: Vec::new(),
            orphaned_sources: Vec::new(),
            orphaned_banks: Vec::new(),
            domain: Arc::new(()),
            next_registration_id: 0,
        }
    }

    /// Walks the kind-11/16 subsequence forward.
    ///
    /// DMO operations sample bit 3 from `userdata_raw`; trigger operations are
    /// unconditional. The live ledger retains the exact plan before the first
    /// manager call.
    ///
    /// # Errors
    ///
    /// Host ledger allocation and registration-ID exhaustion are rejected
    /// before effects. The recovered manager calls return `void` and therefore
    /// have no invented recoverable error channel here.
    pub fn register(
        &mut self,
        prepared: &PreparedSimulationPointerArena,
        userdata_raw: u32,
    ) -> Result<SimulationPointerArenaRegistration, SimulationPointerArenaRegistrationError> {
        self.live.try_reserve(1).map_err(|_| {
            SimulationPointerArenaRegistrationError::LiveStorageAllocation {
                requested: self.live.len().saturating_add(1),
            }
        })?;
        let registration_id = self.next_registration_id;
        self.next_registration_id = self
            .next_registration_id
            .checked_add(1)
            .ok_or(SimulationPointerArenaRegistrationError::RegistrationIdExhausted)?;
        self.live.push(LivePointerRegistration {
            id: registration_id,
            prepared: Arc::clone(&prepared.inner),
            registration_userdata_raw: userdata_raw,
        });
        dispatch_registration(&mut self.dispatch, &prepared.inner, userdata_raw);
        Ok(SimulationPointerArenaRegistration {
            domain: Arc::downgrade(&self.domain),
            id: registration_id,
        })
    }

    /// Walks every arena and pointer operation in a prepared bank forward.
    ///
    /// The aggregate live owner, registration ID, and returned token are all
    /// ready before the first manager call. No fallible adapter work remains
    /// once dispatch begins. The `void` dispatch seam cannot preflight manager-
    /// internal allocation and does not invent a rollback result.
    ///
    /// # Errors
    ///
    /// Host live-ledger allocation and ID exhaustion are rejected without
    /// manager effects.
    pub fn register_bank(
        &mut self,
        prepared: &PreparedSimulationPointerArenaBank,
        userdata_raw: u32,
    ) -> Result<SimulationPointerArenaBankRegistration, SimulationPointerArenaRegistrationError>
    {
        self.live_banks.try_reserve(1).map_err(|_| {
            SimulationPointerArenaRegistrationError::LiveBankStorageAllocation {
                requested: self.live_banks.len().saturating_add(1),
            }
        })?;
        let registration_id = self.next_registration_id;
        self.next_registration_id = self
            .next_registration_id
            .checked_add(1)
            .ok_or(SimulationPointerArenaRegistrationError::RegistrationIdExhausted)?;
        let registration = SimulationPointerArenaBankRegistration {
            domain: Arc::downgrade(&self.domain),
            id: registration_id,
        };
        self.live_banks.push(LivePointerBankRegistration {
            id: registration_id,
            prepared: Arc::clone(&prepared.inner),
            registration_userdata_raw: userdata_raw,
        });
        dispatch_bank_registration(&mut self.dispatch, &prepared.inner, userdata_raw);
        Ok(registration)
    }

    /// Consumes one same-domain occurrence and walks its pointer subsequence
    /// forward using freshly supplied deregistration userdata.
    ///
    /// If Reg sampled bit 3 set but `DReg` samples it clear, the DMO manager can
    /// still retain a record pointer. The adapter therefore keeps a private
    /// orphan source hold until its own dispatcher is destroyed.
    ///
    /// # Errors
    ///
    /// Foreign or unknown tokens are returned without effects. Any required
    /// orphan storage is reserved before effects and allocation failure also
    /// returns the still-valid token unchanged.
    pub fn deregister(
        &mut self,
        registration: SimulationPointerArenaRegistration,
        userdata_raw: u32,
    ) -> Result<(), SimulationPointerArenaDeregistrationError> {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(SimulationPointerArenaDeregistrationError {
                kind: SimulationPointerArenaDeregistrationErrorKind::ForeignDomain,
                registration,
            });
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            return Err(SimulationPointerArenaDeregistrationError {
                kind: SimulationPointerArenaDeregistrationErrorKind::UnknownRegistration,
                registration,
            });
        };
        let retain_orphan = self.live[live_index].prepared.has_dmo
            && dmo_enabled(self.live[live_index].registration_userdata_raw)
            && !dmo_enabled(userdata_raw);
        if retain_orphan {
            self.orphaned_sources.try_reserve(1).map_err(|_| {
                SimulationPointerArenaDeregistrationError {
                    kind: SimulationPointerArenaDeregistrationErrorKind::OrphanStorageAllocation,
                    registration,
                }
            })?;
        }
        let prepared = Arc::clone(&self.live[live_index].prepared);
        dispatch_deregistration(&mut self.dispatch, &prepared, userdata_raw);
        if retain_orphan {
            self.orphaned_sources.push(Arc::clone(&prepared));
        }
        self.live.swap_remove(live_index);
        Ok(())
    }

    /// Consumes one complete same-domain bank occurrence in forward order.
    ///
    /// Token validation and any asymmetric DMO owner allocation finish before
    /// the first manager call. Deregistration samples its fresh userdata word
    /// independently from registration.
    ///
    /// # Errors
    ///
    /// A foreign/unknown token or required orphan-owner allocation is returned
    /// without manager effects and with the token still recoverable.
    pub fn deregister_bank(
        &mut self,
        registration: SimulationPointerArenaBankRegistration,
        userdata_raw: u32,
    ) -> Result<(), SimulationPointerArenaBankDeregistrationError> {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(SimulationPointerArenaBankDeregistrationError {
                kind: SimulationPointerArenaDeregistrationErrorKind::ForeignDomain,
                registration,
            });
        }
        let Some(live_index) = self
            .live_banks
            .iter()
            .position(|live| live.id == registration.id)
        else {
            return Err(SimulationPointerArenaBankDeregistrationError {
                kind: SimulationPointerArenaDeregistrationErrorKind::UnknownRegistration,
                registration,
            });
        };
        let retain_orphan = self.live_banks[live_index].prepared.has_dmo
            && dmo_enabled(self.live_banks[live_index].registration_userdata_raw)
            && !dmo_enabled(userdata_raw);
        if retain_orphan {
            self.orphaned_banks.try_reserve(1).map_err(|_| {
                SimulationPointerArenaBankDeregistrationError {
                    kind: SimulationPointerArenaDeregistrationErrorKind::OrphanStorageAllocation,
                    registration,
                }
            })?;
        }
        let prepared = Arc::clone(&self.live_banks[live_index].prepared);
        dispatch_bank_deregistration(&mut self.dispatch, &prepared, userdata_raw);
        if retain_orphan {
            self.orphaned_banks.push(Arc::clone(&prepared));
        }
        self.live_banks.swap_remove(live_index);
        Ok(())
    }

    #[must_use]
    pub fn live_registration_count(&self) -> usize {
        self.live.len()
    }

    #[must_use]
    pub fn orphaned_source_count(&self) -> usize {
        self.orphaned_sources.len()
    }

    #[must_use]
    pub fn live_bank_registration_count(&self) -> usize {
        self.live_banks.len()
    }

    #[must_use]
    pub fn orphaned_bank_count(&self) -> usize {
        self.orphaned_banks.len()
    }

    #[must_use]
    pub const fn dispatch(&self) -> &D {
        &self.dispatch
    }
}

fn dispatch_registration<D: SimulationPointerArenaDispatch>(
    dispatch: &mut D,
    prepared: &PreparedSimulationPointerArenaInner,
    userdata_raw: u32,
) {
    for operation in &prepared.operations {
        match operation {
            PreparedPointerOperation::Dmo { source, asset_id } if dmo_enabled(userdata_raw) => {
                dispatch.add_dmo_model(*asset_id);
                dispatch.register_dmo_model(source, *asset_id);
            }
            PreparedPointerOperation::Trigger { source } => dispatch.add_trigger_volume(source),
            PreparedPointerOperation::Dmo { .. } => {}
        }
    }
}

fn dispatch_deregistration<D: SimulationPointerArenaDispatch>(
    dispatch: &mut D,
    prepared: &PreparedSimulationPointerArenaInner,
    userdata_raw: u32,
) {
    for operation in &prepared.operations {
        match operation {
            PreparedPointerOperation::Dmo { source, asset_id } if dmo_enabled(userdata_raw) => {
                dispatch.remove_dmo_model(*asset_id);
                dispatch.deregister_dmo_model(source, *asset_id);
            }
            PreparedPointerOperation::Trigger { source } => {
                dispatch.remove_trigger_volume(source);
            }
            PreparedPointerOperation::Dmo { .. } => {}
        }
    }
}

fn dispatch_bank_registration<D: SimulationPointerArenaDispatch>(
    dispatch: &mut D,
    prepared: &PreparedSimulationPointerArenaBankInner,
    userdata_raw: u32,
) {
    for arena in &prepared.arenas {
        dispatch_registration(dispatch, arena, userdata_raw);
    }
}

fn dispatch_bank_deregistration<D: SimulationPointerArenaDispatch>(
    dispatch: &mut D,
    prepared: &PreparedSimulationPointerArenaBankInner,
    userdata_raw: u32,
) {
    for arena in &prepared.arenas {
        dispatch_deregistration(dispatch, arena, userdata_raw);
    }
}

const fn dmo_enabled(userdata_raw: u32) -> bool {
    userdata_raw & DMO_USERDATA_MASK == DMO_USERDATA_MASK
}

/// Non-cloneable authority for one live pointer-subsequence occurrence.
pub struct SimulationPointerArenaRegistration {
    domain: Weak<()>,
    id: u64,
}

/// Non-cloneable authority for one live aggregate bank occurrence.
pub struct SimulationPointerArenaBankRegistration {
    domain: Weak<()>,
    id: u64,
}

impl fmt::Debug for SimulationPointerArenaBankRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationPointerArenaBankRegistration")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationPointerArenaBankPrepareError {
    ArenaStorageAllocation { requested: usize },
    OperationCountOverflow,
}

impl fmt::Display for SimulationPointerArenaBankPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArenaStorageAllocation { requested } => write!(
                formatter,
                "could not retain {requested} prepared pointer-backed Simulation arenas"
            ),
            Self::OperationCountOverflow => {
                formatter.write_str("pointer-backed Simulation bank operation count overflowed")
            }
        }
    }
}

impl Error for SimulationPointerArenaBankPrepareError {}

impl fmt::Debug for SimulationPointerArenaRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationPointerArenaRegistration")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum SimulationPointerArenaPrepareError {
    OperationStorageAllocation {
        requested: usize,
    },
    MissingRecordSource {
        normal_operation_index: usize,
    },
    WrongRecordKind {
        normal_operation_index: usize,
        expected: SimulationRecordSourceKind,
        actual: SimulationRecordSourceKind,
    },
    WrongRecordLength {
        normal_operation_index: usize,
        expected: usize,
        actual: usize,
    },
    RecordIdentityMismatch {
        normal_operation_index: usize,
    },
}

impl fmt::Display for SimulationPointerArenaPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationStorageAllocation { requested } => write!(
                formatter,
                "could not allocate {requested} pointer-backed Simulation operations"
            ),
            Self::MissingRecordSource {
                normal_operation_index,
            } => write!(
                formatter,
                "normal Simulation operation {normal_operation_index} has no retained record source"
            ),
            Self::WrongRecordKind {
                normal_operation_index,
                expected,
                actual,
            } => write!(
                formatter,
                "normal Simulation operation {normal_operation_index} has record kind {actual:?}, expected {expected:?}"
            ),
            Self::WrongRecordLength {
                normal_operation_index,
                expected,
                actual,
            } => write!(
                formatter,
                "normal Simulation operation {normal_operation_index} has {actual} record bytes, expected {expected}"
            ),
            Self::RecordIdentityMismatch {
                normal_operation_index,
            } => write!(
                formatter,
                "normal Simulation operation {normal_operation_index} disagrees with its retained record IDs"
            ),
        }
    }
}

impl Error for SimulationPointerArenaPrepareError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationPointerArenaRegistrationError {
    LiveStorageAllocation { requested: usize },
    LiveBankStorageAllocation { requested: usize },
    RegistrationIdExhausted,
}

impl fmt::Display for SimulationPointerArenaRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LiveStorageAllocation { requested } => write!(
                formatter,
                "could not allocate pointer-backed Simulation registration {requested}"
            ),
            Self::LiveBankStorageAllocation { requested } => write!(
                formatter,
                "could not allocate pointer-backed Simulation bank registration {requested}"
            ),
            Self::RegistrationIdExhausted => {
                formatter.write_str("pointer-backed Simulation registration IDs are exhausted")
            }
        }
    }
}

impl Error for SimulationPointerArenaRegistrationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationPointerArenaDeregistrationErrorKind {
    ForeignDomain,
    UnknownRegistration,
    OrphanStorageAllocation,
}

#[derive(Debug)]
pub struct SimulationPointerArenaDeregistrationError {
    kind: SimulationPointerArenaDeregistrationErrorKind,
    registration: SimulationPointerArenaRegistration,
}

impl SimulationPointerArenaDeregistrationError {
    #[must_use]
    pub const fn kind(&self) -> SimulationPointerArenaDeregistrationErrorKind {
        self.kind
    }

    #[must_use]
    pub fn into_registration(self) -> SimulationPointerArenaRegistration {
        self.registration
    }
}

impl fmt::Display for SimulationPointerArenaDeregistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            SimulationPointerArenaDeregistrationErrorKind::ForeignDomain => {
                "registration belongs to a different pointer adapter domain"
            }
            SimulationPointerArenaDeregistrationErrorKind::UnknownRegistration => {
                "registration is not live in this pointer adapter"
            }
            SimulationPointerArenaDeregistrationErrorKind::OrphanStorageAllocation => {
                "could not retain a DMO source skipped by deregistration userdata"
            }
        })
    }
}

impl Error for SimulationPointerArenaDeregistrationError {}

#[derive(Debug)]
pub struct SimulationPointerArenaBankDeregistrationError {
    kind: SimulationPointerArenaDeregistrationErrorKind,
    registration: SimulationPointerArenaBankRegistration,
}

impl SimulationPointerArenaBankDeregistrationError {
    #[must_use]
    pub const fn kind(&self) -> SimulationPointerArenaDeregistrationErrorKind {
        self.kind
    }

    #[must_use]
    pub fn into_registration(self) -> SimulationPointerArenaBankRegistration {
        self.registration
    }
}

impl fmt::Display for SimulationPointerArenaBankDeregistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            SimulationPointerArenaDeregistrationErrorKind::ForeignDomain => {
                "bank registration belongs to a different pointer adapter domain"
            }
            SimulationPointerArenaDeregistrationErrorKind::UnknownRegistration => {
                "bank registration is not live in this pointer adapter"
            }
            SimulationPointerArenaDeregistrationErrorKind::OrphanStorageAllocation => {
                "could not retain a DMO bank source skipped by deregistration userdata"
            }
        })
    }
}

impl Error for SimulationPointerArenaBankDeregistrationError {}

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
        pegasus::{
            INSTANCE_DATA_TYPE_ID, INSTANCE_DMO_ASSET_TYPE_ID, ObjectId, TABLE_OF_CONTENTS_TYPE_ID,
            TRIGGER_INSTANCE_DATA_TYPE_ID,
        },
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::{
        ops::Range,
        sync::atomic::{AtomicBool, Ordering},
    };

    #[derive(Debug, Eq, PartialEq)]
    enum Event {
        AddDmo(u64),
        RegisterDmo { primary: u64, asset_id: u64 },
        RemoveDmo(u64),
        DeregisterDmo { primary: u64, asset_id: u64 },
        AddTrigger { primary: u64, local: u64 },
        RemoveTrigger { primary: u64, local: u64 },
    }

    #[derive(Default)]
    struct Recorder {
        events: Vec<Event>,
        registration_sources: Vec<usize>,
        deregistration_sources: Vec<usize>,
    }

    impl SimulationPointerArenaDispatch for Recorder {
        fn add_dmo_model(&mut self, asset_id: u64) {
            self.events.push(Event::AddDmo(asset_id));
        }

        fn register_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64) {
            self.events.push(Event::RegisterDmo {
                primary: read_record_u64(source, 96),
                asset_id,
            });
            self.registration_sources
                .push(source.bytes().as_ptr() as usize);
        }

        fn remove_dmo_model(&mut self, asset_id: u64) {
            self.events.push(Event::RemoveDmo(asset_id));
        }

        fn deregister_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64) {
            self.events.push(Event::DeregisterDmo {
                primary: read_record_u64(source, 96),
                asset_id,
            });
            self.deregistration_sources
                .push(source.bytes().as_ptr() as usize);
        }

        fn add_trigger_volume(&mut self, source: &SimulationRecordHandle) {
            self.events.push(Event::AddTrigger {
                primary: read_record_u64(source, 176),
                local: read_record_u64(source, 184),
            });
            self.registration_sources
                .push(source.bytes().as_ptr() as usize);
        }

        fn remove_trigger_volume(&mut self, source: &SimulationRecordHandle) {
            self.events.push(Event::RemoveTrigger {
                primary: read_record_u64(source, 176),
                local: read_record_u64(source, 184),
            });
            self.deregistration_sources
                .push(source.bytes().as_ptr() as usize);
        }
    }

    fn read_record_u64(source: &SimulationRecordHandle, offset: usize) -> u64 {
        u64::from_be_bytes(source.bytes()[offset..offset + 8].try_into().unwrap())
    }

    struct ArenaFixture {
        bytes: Vec<u8>,
        dictionary: Vec<DictionaryEntry>,
    }

    struct DropObservationDispatch {
        plan: Weak<SimulationNormalArenaPlan>,
        source_was_alive: Arc<AtomicBool>,
    }

    impl Drop for DropObservationDispatch {
        fn drop(&mut self) {
            self.source_was_alive
                .store(self.plan.upgrade().is_some(), Ordering::Release);
        }
    }

    impl SimulationPointerArenaDispatch for DropObservationDispatch {
        fn add_dmo_model(&mut self, _asset_id: u64) {}
        fn register_dmo_model(&mut self, _source: &SimulationRecordHandle, _asset_id: u64) {}
        fn remove_dmo_model(&mut self, _asset_id: u64) {}
        fn deregister_dmo_model(&mut self, _source: &SimulationRecordHandle, _asset_id: u64) {}
        fn add_trigger_volume(&mut self, _source: &SimulationRecordHandle) {}
        fn remove_trigger_volume(&mut self, _source: &SimulationRecordHandle) {}
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

    fn instance_data(records: &[(u64, u64)]) -> Vec<u8> {
        let mut bytes = vec![0; 20 + records.len() * 160];
        put_u32(&mut bytes, 0, INSTANCE_DMO_ASSET_TYPE_ID);
        put_u32(&mut bytes, 4, u32::try_from(records.len()).unwrap());
        put_u32(&mut bytes, 12, 20);
        let string_list_offset = u32::try_from(bytes.len()).unwrap();
        put_u32(&mut bytes, 16, string_list_offset);
        for (index, &(primary, local)) in records.iter().enumerate() {
            let start = 20 + index * 160;
            put_u64(&mut bytes, start + 96, primary);
            put_u64(&mut bytes, start + 104, local);
        }
        bytes
    }

    fn trigger_data(records: &[(u64, u64)]) -> Vec<u8> {
        let mut bytes = vec![0; 20 + records.len() * 240];
        put_u32(&mut bytes, 0, 0xdead_beef);
        put_u32(&mut bytes, 4, u32::try_from(records.len()).unwrap());
        put_u32(&mut bytes, 12, 20);
        let string_list_offset = u32::try_from(bytes.len()).unwrap();
        put_u32(&mut bytes, 16, string_list_offset);
        for (index, &(primary, local)) in records.iter().enumerate() {
            let start = 20 + index * 240;
            put_u64(&mut bytes, start + 176, primary);
            put_u64(&mut bytes, start + 184, local);
        }
        bytes
    }

    fn mixed_plan() -> Arc<SimulationNormalArenaPlan> {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(
            INSTANCE_DATA_TYPE_ID,
            &instance_data(&[(0x11, 0x101), (0x22, 0x202)]),
        );
        let ignored = fixture.push(0x00eb_0013, &[0]);
        let trigger = fixture.push(
            TRIGGER_INSTANCE_DATA_TYPE_ID,
            &trigger_data(&[(0x33, 0x303), (0x44, 0x404)]),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (1, INSTANCE_DATA_TYPE_ID, dmo),
                (2, 0x00eb_0013, ignored),
                (3, TRIGGER_INSTANCE_DATA_TYPE_ID, trigger),
                (4, INSTANCE_DATA_TYPE_ID, dmo),
            ]),
        );
        fixture.finish()
    }

    fn one_each_plan() -> Arc<SimulationNormalArenaPlan> {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(INSTANCE_DATA_TYPE_ID, &instance_data(&[(0x11, 0x101)]));
        let trigger = fixture.push(
            TRIGGER_INSTANCE_DATA_TYPE_ID,
            &trigger_data(&[(0x33, 0x303)]),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (1, INSTANCE_DATA_TYPE_ID, dmo),
                (2, TRIGGER_INSTANCE_DATA_TYPE_ID, trigger),
            ]),
        );
        fixture.finish()
    }

    #[test]
    fn mixed_pointer_subsequence_is_forward_and_preserves_aliases() {
        let prepared = PreparedSimulationPointerArena::try_new(mixed_plan()).unwrap();
        assert_eq!(prepared.operation_count(), 6);
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());

        let registration = adapter.register(&prepared, 0x8000_0008).unwrap();
        adapter.deregister(registration, 8).unwrap();

        assert_eq!(
            adapter.dispatch().events,
            [
                Event::AddDmo(0x11),
                Event::RegisterDmo {
                    primary: 0x11,
                    asset_id: 0x11,
                },
                Event::AddDmo(0x22),
                Event::RegisterDmo {
                    primary: 0x22,
                    asset_id: 0x22,
                },
                Event::AddTrigger {
                    primary: 0x33,
                    local: 0x303,
                },
                Event::AddTrigger {
                    primary: 0x44,
                    local: 0x404,
                },
                Event::AddDmo(0x11),
                Event::RegisterDmo {
                    primary: 0x11,
                    asset_id: 0x11,
                },
                Event::AddDmo(0x22),
                Event::RegisterDmo {
                    primary: 0x22,
                    asset_id: 0x22,
                },
                Event::RemoveDmo(0x11),
                Event::DeregisterDmo {
                    primary: 0x11,
                    asset_id: 0x11,
                },
                Event::RemoveDmo(0x22),
                Event::DeregisterDmo {
                    primary: 0x22,
                    asset_id: 0x22,
                },
                Event::RemoveTrigger {
                    primary: 0x33,
                    local: 0x303,
                },
                Event::RemoveTrigger {
                    primary: 0x44,
                    local: 0x404,
                },
                Event::RemoveDmo(0x11),
                Event::DeregisterDmo {
                    primary: 0x11,
                    asset_id: 0x11,
                },
                Event::RemoveDmo(0x22),
                Event::DeregisterDmo {
                    primary: 0x22,
                    asset_id: 0x22,
                },
            ]
        );
        assert_eq!(
            adapter.dispatch().registration_sources,
            adapter.dispatch().deregistration_sources
        );
        assert_eq!(
            adapter.dispatch().registration_sources[0],
            adapter.dispatch().registration_sources[4]
        );
        assert_ne!(
            adapter.dispatch().registration_sources[2],
            adapter.dispatch().registration_sources[3]
        );
    }

    #[test]
    fn registration_and_deregistration_sample_userdata_independently() {
        for (registration_userdata, deregistration_userdata, reg_dmo, dreg_dmo) in [
            (0, 0, false, false),
            (0x8000_0000, 0x4000_0008, false, true),
            (0x8000_0008, 0x4000_0000, true, false),
            (u32::MAX, 8, true, true),
        ] {
            let prepared = PreparedSimulationPointerArena::try_new(one_each_plan()).unwrap();
            let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
            let registration = adapter.register(&prepared, registration_userdata).unwrap();
            adapter
                .deregister(registration, deregistration_userdata)
                .unwrap();

            assert_eq!(
                adapter
                    .dispatch()
                    .events
                    .iter()
                    .filter(|event| matches!(event, Event::AddDmo(_)))
                    .count(),
                usize::from(reg_dmo)
            );
            assert_eq!(
                adapter
                    .dispatch()
                    .events
                    .iter()
                    .filter(|event| matches!(event, Event::RemoveDmo(_)))
                    .count(),
                usize::from(dreg_dmo)
            );
            assert_eq!(
                adapter
                    .dispatch()
                    .events
                    .iter()
                    .filter(|event| matches!(event, Event::AddTrigger { .. }))
                    .count(),
                1
            );
            assert_eq!(
                adapter
                    .dispatch()
                    .events
                    .iter()
                    .filter(|event| matches!(event, Event::RemoveTrigger { .. }))
                    .count(),
                1
            );
            assert_eq!(
                adapter.orphaned_source_count(),
                usize::from(reg_dmo && !dreg_dmo)
            );
        }
    }

    #[test]
    fn live_and_asymmetrically_orphaned_sources_retain_the_plan() {
        let plan = one_each_plan();
        let weak = Arc::downgrade(&plan);
        let prepared = PreparedSimulationPointerArena::try_new(Arc::clone(&plan)).unwrap();
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let registration = adapter.register(&prepared, 8).unwrap();
        drop(plan);
        drop(prepared);
        assert!(weak.upgrade().is_some());

        adapter.deregister(registration, 0).unwrap();
        assert_eq!(adapter.live_registration_count(), 0);
        assert_eq!(adapter.orphaned_source_count(), 1);
        assert!(weak.upgrade().is_some());
        drop(adapter);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn matched_deregistration_releases_the_last_plan_owner() {
        let plan = one_each_plan();
        let weak = Arc::downgrade(&plan);
        let prepared = PreparedSimulationPointerArena::try_new(Arc::clone(&plan)).unwrap();
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let registration = adapter.register(&prepared, 8).unwrap();
        drop(plan);
        drop(prepared);

        adapter.deregister(registration, 8).unwrap();
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn foreign_token_is_recoverable_and_has_no_effects() {
        let prepared = PreparedSimulationPointerArena::try_new(one_each_plan()).unwrap();
        let mut owner = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let mut foreign = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let registration = owner.register(&prepared, 8).unwrap();

        let rejection = foreign.deregister(registration, 8).unwrap_err();
        assert_eq!(
            rejection.kind(),
            SimulationPointerArenaDeregistrationErrorKind::ForeignDomain
        );
        assert!(foreign.dispatch().events.is_empty());
        owner.deregister(rejection.into_registration(), 8).unwrap();
        assert_eq!(owner.live_registration_count(), 0);
    }

    #[test]
    fn dropping_a_token_neither_cleans_up_nor_releases_its_source() {
        let plan = one_each_plan();
        let weak = Arc::downgrade(&plan);
        let prepared = PreparedSimulationPointerArena::try_new(Arc::clone(&plan)).unwrap();
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let registration = adapter.register(&prepared, 8).unwrap();
        let registered_event_count = adapter.dispatch().events.len();
        drop(plan);
        drop(prepared);
        drop(registration);

        assert_eq!(adapter.live_registration_count(), 1);
        assert_eq!(adapter.dispatch().events.len(), registered_event_count);
        assert!(weak.upgrade().is_some());
    }

    #[test]
    fn dispatcher_is_destroyed_before_abandoned_source_storage() {
        let plan = one_each_plan();
        let weak = Arc::downgrade(&plan);
        let source_was_alive = Arc::new(AtomicBool::new(false));
        let prepared = PreparedSimulationPointerArena::try_new(Arc::clone(&plan)).unwrap();
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(DropObservationDispatch {
            plan: Weak::clone(&weak),
            source_was_alive: Arc::clone(&source_was_alive),
        });
        let registration = adapter.register(&prepared, 8).unwrap();
        drop(registration);
        drop(plan);
        drop(prepared);

        drop(adapter);
        assert!(source_was_alive.load(Ordering::Acquire));
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn empty_subsequence_and_distinct_occurrences_are_supported() {
        let mut empty_fixture = ArenaFixture::new();
        empty_fixture.push(TABLE_OF_CONTENTS_TYPE_ID, &toc(&[]));
        let empty = PreparedSimulationPointerArena::try_new(empty_fixture.finish()).unwrap();
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let empty_registration = adapter.register(&empty, u32::MAX).unwrap();
        adapter.deregister(empty_registration, u32::MAX).unwrap();
        assert!(adapter.dispatch().events.is_empty());

        let prepared = PreparedSimulationPointerArena::try_new(one_each_plan()).unwrap();
        let first = adapter.register(&prepared, 0).unwrap();
        let second = adapter.register(&prepared, 0).unwrap();
        assert_eq!(adapter.live_registration_count(), 2);
        adapter.deregister(first, 0).unwrap();
        assert_eq!(adapter.live_registration_count(), 1);
        adapter.deregister(second, 0).unwrap();
        assert_eq!(adapter.live_registration_count(), 0);
    }

    #[test]
    fn same_domain_unknown_token_is_rejected_without_effects() {
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let unknown = SimulationPointerArenaRegistration {
            domain: Arc::downgrade(&adapter.domain),
            id: 99,
        };
        let rejection = adapter.deregister(unknown, 8).unwrap_err();
        assert_eq!(
            rejection.kind(),
            SimulationPointerArenaDeregistrationErrorKind::UnknownRegistration
        );
        assert!(adapter.dispatch().events.is_empty());
    }

    #[test]
    fn aggregate_bank_walks_arenas_and_operations_forward() {
        let first = PreparedSimulationPointerArena::try_new(mixed_plan()).unwrap();
        let second = PreparedSimulationPointerArena::try_new(one_each_plan()).unwrap();
        let bank = PreparedSimulationPointerArenaBank::try_from_prepared(&[first, second]).unwrap();
        assert_eq!(bank.arena_count(), 2);
        assert_eq!(bank.operation_count(), 8);

        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let registration = adapter.register_bank(&bank, 8).unwrap();
        assert_eq!(adapter.live_registration_count(), 0);
        assert_eq!(adapter.live_bank_registration_count(), 1);
        assert_eq!(adapter.dispatch().events.len(), 13);
        adapter.deregister_bank(registration, 8).unwrap();

        assert_eq!(adapter.live_bank_registration_count(), 0);
        assert_eq!(adapter.dispatch().events.len(), 26);
        assert_eq!(
            adapter.dispatch().registration_sources,
            adapter.dispatch().deregistration_sources
        );
        assert_eq!(
            &adapter.dispatch().events[..4],
            [
                Event::AddDmo(0x11),
                Event::RegisterDmo {
                    primary: 0x11,
                    asset_id: 0x11,
                },
                Event::AddDmo(0x22),
                Event::RegisterDmo {
                    primary: 0x22,
                    asset_id: 0x22,
                },
            ]
        );
    }

    #[test]
    fn bank_userdata_is_independent_and_asymmetry_retains_one_owner() {
        for (registration_userdata, deregistration_userdata, reg_dmo, dreg_dmo) in [
            (0, 0, false, false),
            (0, 8, false, true),
            (8, 0, true, false),
            (u32::MAX, 8, true, true),
        ] {
            let plan = one_each_plan();
            let weak = Arc::downgrade(&plan);
            let prepared = PreparedSimulationPointerArena::try_new(Arc::clone(&plan)).unwrap();
            let bank = PreparedSimulationPointerArenaBank::try_from_prepared(&[prepared]).unwrap();
            let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
            let registration = adapter.register_bank(&bank, registration_userdata).unwrap();
            drop(plan);
            drop(bank);
            adapter
                .deregister_bank(registration, deregistration_userdata)
                .unwrap();

            assert_eq!(
                adapter
                    .dispatch()
                    .events
                    .iter()
                    .filter(|event| matches!(event, Event::AddDmo(_)))
                    .count(),
                usize::from(reg_dmo)
            );
            assert_eq!(
                adapter
                    .dispatch()
                    .events
                    .iter()
                    .filter(|event| matches!(event, Event::RemoveDmo(_)))
                    .count(),
                usize::from(dreg_dmo)
            );
            assert_eq!(
                adapter.orphaned_bank_count(),
                usize::from(reg_dmo && !dreg_dmo)
            );
            assert_eq!(weak.upgrade().is_some(), reg_dmo && !dreg_dmo);
        }
    }

    #[test]
    fn bank_id_exhaustion_and_foreign_token_have_no_effects() {
        let prepared = PreparedSimulationPointerArena::try_new(one_each_plan()).unwrap();
        let bank = PreparedSimulationPointerArenaBank::try_from_prepared(&[prepared]).unwrap();
        let mut exhausted = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        exhausted.next_registration_id = u64::MAX;
        assert_eq!(
            exhausted.register_bank(&bank, 8).unwrap_err(),
            SimulationPointerArenaRegistrationError::RegistrationIdExhausted
        );
        assert_eq!(exhausted.live_bank_registration_count(), 0);
        assert!(exhausted.dispatch().events.is_empty());

        let mut owner = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let mut foreign = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let registration = owner.register_bank(&bank, 8).unwrap();
        let rejection = foreign.deregister_bank(registration, 8).unwrap_err();
        assert_eq!(
            rejection.kind(),
            SimulationPointerArenaDeregistrationErrorKind::ForeignDomain
        );
        assert!(foreign.dispatch().events.is_empty());
        owner
            .deregister_bank(rejection.into_registration(), 8)
            .unwrap();
        assert_eq!(owner.live_bank_registration_count(), 0);
    }

    #[test]
    fn dropped_bank_token_performs_no_cleanup_and_retains_all_sources() {
        let first_plan = one_each_plan();
        let second_plan = mixed_plan();
        let first_weak = Arc::downgrade(&first_plan);
        let second_weak = Arc::downgrade(&second_plan);
        let first = PreparedSimulationPointerArena::try_new(Arc::clone(&first_plan)).unwrap();
        let second = PreparedSimulationPointerArena::try_new(Arc::clone(&second_plan)).unwrap();
        let bank = PreparedSimulationPointerArenaBank::try_from_prepared(&[first, second]).unwrap();
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let registration = adapter.register_bank(&bank, 8).unwrap();
        let registered_events = adapter.dispatch().events.len();
        drop(registration);
        drop(bank);
        drop(first_plan);
        drop(second_plan);

        assert_eq!(adapter.live_bank_registration_count(), 1);
        assert_eq!(adapter.dispatch().events.len(), registered_events);
        assert!(first_weak.upgrade().is_some());
        assert!(second_weak.upgrade().is_some());
        drop(adapter);
        assert!(first_weak.upgrade().is_none());
        assert!(second_weak.upgrade().is_none());
    }

    #[test]
    fn repeated_bank_occurrences_are_distinct_and_unknown_tokens_are_recoverable() {
        let empty = PreparedSimulationPointerArenaBank::try_from_prepared(&[]).unwrap();
        assert_eq!(empty.arena_count(), 0);
        assert_eq!(empty.operation_count(), 0);
        let mut empty_adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let empty_registration = empty_adapter.register_bank(&empty, u32::MAX).unwrap();
        empty_adapter
            .deregister_bank(empty_registration, u32::MAX)
            .unwrap();
        assert!(empty_adapter.dispatch().events.is_empty());

        let prepared = PreparedSimulationPointerArena::try_new(one_each_plan()).unwrap();
        let bank = PreparedSimulationPointerArenaBank::try_from_prepared(&[prepared]).unwrap();
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(Recorder::default());
        let first = adapter.register_bank(&bank, 0).unwrap();
        let second = adapter.register_bank(&bank, 0).unwrap();
        assert_eq!(adapter.live_bank_registration_count(), 2);
        adapter.deregister_bank(second, 0).unwrap();
        assert_eq!(adapter.live_bank_registration_count(), 1);
        adapter.deregister_bank(first, 0).unwrap();
        assert_eq!(adapter.live_bank_registration_count(), 0);

        let unknown = SimulationPointerArenaBankRegistration {
            domain: Arc::downgrade(&adapter.domain),
            id: 99,
        };
        let rejection = adapter.deregister_bank(unknown, 8).unwrap_err();
        assert_eq!(
            rejection.kind(),
            SimulationPointerArenaDeregistrationErrorKind::UnknownRegistration
        );
        assert!(matches!(
            rejection.into_registration(),
            SimulationPointerArenaBankRegistration { id: 99, .. }
        ));
        assert_eq!(adapter.live_bank_registration_count(), 0);
    }

    #[test]
    fn bank_dispatcher_drops_before_abandoned_source_storage() {
        let plan = one_each_plan();
        let weak = Arc::downgrade(&plan);
        let source_was_alive = Arc::new(AtomicBool::new(false));
        let prepared = PreparedSimulationPointerArena::try_new(Arc::clone(&plan)).unwrap();
        let bank = PreparedSimulationPointerArenaBank::try_from_prepared(&[prepared]).unwrap();
        let mut adapter = SimulationPointerArenaRegistrationAdapter::new(DropObservationDispatch {
            plan: Weak::clone(&weak),
            source_was_alive: Arc::clone(&source_was_alive),
        });
        let registration = adapter.register_bank(&bank, 8).unwrap();
        drop(registration);
        drop(bank);
        drop(plan);

        drop(adapter);
        assert!(source_was_alive.load(Ordering::Acquire));
        assert!(weak.upgrade().is_none());
    }
}
