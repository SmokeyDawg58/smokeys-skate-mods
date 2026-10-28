//! Non-owning publication and exact-occurrence lookup for resident collections.
//!
//! Native Simulation collection receivers resolve the full collection ID
//! through the global asset registry when their message is dequeued. This
//! module supplies a safe-host publication seam for that later lookup. It does
//! not prepare or dispatch a Simulation collection, increment loaded counts,
//! or translate a missing lookup into native assertion behavior.
//!
//! Monotonic private tokens distinguish two lifetimes of the same full-width
//! ID. Registry rows and lookup capabilities retain only weak resident views;
//! the resident table remains the sole collection owner. Dropping any type in
//! this module performs no publication, removal, callback, or teardown effect.
//! The shared host namespace represents Collection, outer-Arena, and the six
//! selected ID-only inner asset classes. Other native registry classes remain
//! an explicit unrepresented collision boundary.

use crate::{
    collection_resident_streamer::{CollectionId, ResidentCollectionRegistry},
    dmo_bank_processed_asset::{DMO_BANK_COLLECTION_ID, DmoBankProcessedArenaAsset},
    owned_resident_collection::{
        OwnedResidentAssetsView, OwnedResidentAssetsVisitError, OwnedResidentCollection,
        OwnedResidentCollectionError, OwnedResidentCollectionView, OwnedResidentWeakAsset,
    },
    simulation_dmo_assembly_part_session::{
        SimulationDmoBankModelRegistrationBatch, SimulationDmoBankModelRegistrationBatchError,
        SimulationDmoBankSnapshotConsumer,
    },
};
use std::{
    error::Error,
    fmt,
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

#[path = "simulation_resident_inner_bridge.rs"]
mod selected_inner;

/// Registry phase that encountered a poisoned host mutex.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationResidentRegistryOperation {
    CreateWrapper,
    Publish,
    Remove,
}

/// Storage whose pre-effect reservation failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationResidentRegistryStorage {
    Pending,
    Active,
    Container,
}

/// Typed safe-host publication failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationResidentRegistryError {
    SelectedProcessing(crate::selected_simulation_asset_services::SelectedAssetError),
    PendingSelectedPublicationFailure,
    Poisoned {
        operation: SimulationResidentRegistryOperation,
    },
    TokenExhausted,
    StorageAllocation {
        storage: SimulationResidentRegistryStorage,
        requested: usize,
    },
    ResidentAssetSnapshot(OwnedResidentCollectionError),
    UnknownPendingToken {
        id: CollectionId,
    },
    PendingIdMismatch {
        requested: CollectionId,
        pending: CollectionId,
    },
    DetachedPendingCollection {
        id: CollectionId,
    },
    DuplicateActiveId {
        id: CollectionId,
    },
    MissingActiveId {
        id: CollectionId,
    },
    TokenMismatch {
        id: CollectionId,
    },
}

impl fmt::Display for SimulationResidentRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectedProcessing(source) => {
                write!(formatter, "selected resident asset publication: {source:?}")
            }
            Self::PendingSelectedPublicationFailure => {
                formatter.write_str("selected resident publication has a retained failed prefix")
            }
            Self::Poisoned { operation } => {
                write!(
                    formatter,
                    "Simulation resident registry poisoned during {operation:?}"
                )
            }
            Self::TokenExhausted => {
                formatter.write_str("Simulation resident registry token space exhausted")
            }
            Self::StorageAllocation { storage, requested } => write!(
                formatter,
                "could not reserve {requested} {storage:?} Simulation resident registry rows"
            ),
            Self::ResidentAssetSnapshot(source) => {
                write!(
                    formatter,
                    "could not snapshot resident asset identities: {source}"
                )
            }
            Self::UnknownPendingToken { id } => write!(
                formatter,
                "collection {:#018x} has no matching pending registry token",
                id.0
            ),
            Self::PendingIdMismatch { requested, pending } => write!(
                formatter,
                "pending registry token belongs to {:#018x}, not {:#018x}",
                pending.0, requested.0
            ),
            Self::DetachedPendingCollection { id } => write!(
                formatter,
                "pending collection {:#018x} detached before publication",
                id.0
            ),
            Self::DuplicateActiveId { id } => {
                write!(formatter, "collection {:#018x} is already published", id.0)
            }
            Self::MissingActiveId { id } => write!(
                formatter,
                "collection {:#018x} has no active registry row",
                id.0
            ),
            Self::TokenMismatch { id } => write!(
                formatter,
                "collection {:#018x} registry token does not match",
                id.0
            ),
        }
    }
}

impl Error for SimulationResidentRegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResidentAssetSnapshot(source) => Some(source),
            Self::SelectedProcessing(_)
            | Self::PendingSelectedPublicationFailure
            | Self::Poisoned { .. }
            | Self::TokenExhausted
            | Self::StorageAllocation { .. }
            | Self::UnknownPendingToken { .. }
            | Self::PendingIdMismatch { .. }
            | Self::DetachedPendingCollection { .. }
            | Self::DuplicateActiveId { .. }
            | Self::MissingActiveId { .. }
            | Self::TokenMismatch { .. } => None,
        }
    }
}

/// Represented asset class currently occupying one full-ID container-registry key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationResidentContainerAssetKind {
    Collection,
    Arena,
    Inner(u32),
}

/// Failure while resolving one current native-style Arena registry winner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationResidentArenaLookupError<E> {
    RegistryDetached,
    Poisoned,
    PublishedKindMismatch {
        id: CollectionId,
        actual: SimulationResidentContainerAssetKind,
    },
    PublishedAssetExpired {
        id: CollectionId,
    },
    Visitor(E),
}

impl<E: fmt::Display> fmt::Display for SimulationResidentArenaLookupError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryDetached => formatter.write_str("Simulation Arena registry is detached"),
            Self::Poisoned => formatter.write_str("Simulation Arena registry is poisoned"),
            Self::PublishedKindMismatch { id, actual } => write!(
                formatter,
                "container registry ID {:#018x} contains {actual:?}, not an Arena",
                id.0
            ),
            Self::PublishedAssetExpired { id } => write!(
                formatter,
                "published Simulation Arena {:#018x} expired before lookup",
                id.0
            ),
            Self::Visitor(error) => write!(formatter, "Simulation Arena visitor failed: {error}"),
        }
    }
}

impl<E> Error for SimulationResidentArenaLookupError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Visitor(source) => Some(source),
            Self::RegistryDetached
            | Self::Poisoned
            | Self::PublishedKindMismatch { .. }
            | Self::PublishedAssetExpired { .. } => None,
        }
    }
}

/// Failure from full-ID lookup before an occurrence exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationResidentLookupError {
    RegistryDetached,
    Poisoned,
    PublishedKindMismatch {
        id: CollectionId,
        actual: SimulationResidentContainerAssetKind,
    },
}

impl fmt::Display for SimulationResidentLookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryDetached => {
                formatter.write_str("Simulation resident registry is detached")
            }
            Self::Poisoned => {
                formatter.write_str("Simulation resident registry is poisoned during lookup")
            }
            Self::PublishedKindMismatch { id, actual } => write!(
                formatter,
                "container registry ID {:#018x} contains {actual:?}, not a Collection",
                id.0
            ),
        }
    }
}

impl Error for SimulationResidentLookupError {}

/// Failure while using one exact published occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationResidentVisitError<E> {
    RegistryDetached,
    Poisoned,
    UnknownOccurrence { id: CollectionId },
    Assets(OwnedResidentAssetsVisitError<E>),
}

impl<E: fmt::Display> fmt::Display for SimulationResidentVisitError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryDetached => formatter.write_str("Simulation resident registry detached"),
            Self::Poisoned => {
                formatter.write_str("Simulation resident registry poisoned during occurrence use")
            }
            Self::UnknownOccurrence { id } => write!(
                formatter,
                "Simulation resident occurrence {:#018x} is stale or unknown",
                id.0
            ),
            Self::Assets(source) => write!(formatter, "resident asset visit failed: {source}"),
        }
    }
}

impl<E> Error for SimulationResidentVisitError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Assets(source) => Some(source),
            Self::RegistryDetached | Self::Poisoned | Self::UnknownOccurrence { .. } => None,
        }
    }
}

/// Non-owning safe-host publication identity for one staged resident row.
///
/// The serial is meaningful only inside the registry domain that issued it.
/// Keeping the domain weak prevents a retained streamer token from extending
/// the registry or lookup lifetime.
#[derive(Clone)]
pub struct SimulationResidentToken {
    domain: Weak<SimulationResidentTokenDomain>,
    serial: u64,
}

struct SimulationResidentTokenDomain;

impl PartialEq for SimulationResidentToken {
    fn eq(&self, other: &Self) -> bool {
        self.serial == other.serial && Weak::ptr_eq(&self.domain, &other.domain)
    }
}

impl Eq for SimulationResidentToken {}

impl fmt::Debug for SimulationResidentToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationResidentToken")
            .field("serial", &self.serial)
            .finish_non_exhaustive()
    }
}

struct RegistryRow<A> {
    id: CollectionId,
    token: SimulationResidentToken,
    collection: OwnedResidentCollectionView<A>,
    assets: Arc<OwnedResidentAssetsView<A>>,
    arena_candidates: Vec<OwnedResidentWeakAsset<A>>,
    inner_candidates: Vec<(
        u64,
        Weak<crate::simulation_inner_registry_asset::SimulationInnerRegistryAsset>,
    )>,
}

enum ContainerRegistryAsset<A> {
    Collection {
        token: SimulationResidentToken,
    },
    Arena {
        asset: Weak<A>,
    },
    Inner {
        kind: u32,
        asset: Weak<crate::simulation_inner_registry_asset::SimulationInnerRegistryAsset>,
    },
}

impl<A> ContainerRegistryAsset<A> {
    const fn kind(&self) -> SimulationResidentContainerAssetKind {
        match self {
            Self::Collection { .. } => SimulationResidentContainerAssetKind::Collection,
            Self::Arena { .. } => SimulationResidentContainerAssetKind::Arena,
            Self::Inner { kind, .. } => SimulationResidentContainerAssetKind::Inner(*kind),
        }
    }
}

struct ContainerRegistryRow<A> {
    id: CollectionId,
    asset: ContainerRegistryAsset<A>,
}

struct RegistryState<A> {
    next_token: u64,
    pending: Vec<RegistryRow<A>>,
    active: Vec<RegistryRow<A>>,
    failed_publication: Option<RegistryRow<A>>,
    container: Vec<ContainerRegistryRow<A>>,
    #[cfg(test)]
    attached_lookup_attempts: Arc<std::sync::atomic::AtomicUsize>,
    #[cfg(test)]
    attached_arena_lookup_attempts: Arc<std::sync::atomic::AtomicUsize>,
}

impl<A> Default for RegistryState<A> {
    fn default() -> Self {
        Self {
            next_token: 0,
            pending: Vec::new(),
            active: Vec::new(),
            failed_publication: None,
            container: Vec::new(),
            #[cfg(test)]
            attached_lookup_attempts: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            #[cfg(test)]
            attached_arena_lookup_attempts: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

/// Wrapper owned by the resident table after physical load completion.
pub struct SimulationResidentWrapper<A> {
    token: SimulationResidentToken,
    collection: OwnedResidentCollectionView<A>,
}

impl<A> SimulationResidentWrapper<A> {
    /// Reports whether the weak resident collection allocation still exists.
    /// This observation does not publish, remove, or retain it.
    #[must_use]
    pub fn is_collection_attached(&self) -> bool {
        self.collection.is_attached()
    }
}

/// Trait adapter installed in [`crate::collection_resident_streamer::CollectionResidentStreamer`].
///
/// `M` converts this seam's local failures into the composition's shared
/// backend error without erasing their stage.
pub struct SimulationResidentRegistry<A, M, E> {
    selected_processor: Option<
        Arc<dyn crate::selected_simulation_asset_services::bridge::SelectedArenaProcessor<A>>,
    >,
    domain: Arc<SimulationResidentTokenDomain>,
    state: Arc<Mutex<RegistryState<A>>>,
    map_error: M,
    error: PhantomData<fn() -> E>,
}

/// Cloneable non-owning access to the Collection view of the shared full-ID namespace.
pub struct SimulationResidentLookup<A> {
    state: Weak<Mutex<RegistryState<A>>>,
}

/// Test-only observation of lookup calls that reached an attached, successfully
/// locked registry state. Detached or poisoned calls are not counted, and this
/// observer does not retain the registry.
#[cfg(test)]
pub(crate) struct SimulationResidentAttachedLookupAttemptObserver {
    attached_attempts: Arc<std::sync::atomic::AtomicUsize>,
}

/// Test-only observation of Arena lookup calls reaching an attached,
/// successfully locked registry state. The observer does not retain the
/// registry and excludes detached or poisoned calls.
#[cfg(test)]
pub(crate) struct SimulationResidentAttachedArenaLookupAttemptObserver {
    attached_attempts: Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
impl SimulationResidentAttachedArenaLookupAttemptObserver {
    pub(crate) fn count(&self) -> usize {
        self.attached_attempts
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
impl SimulationResidentAttachedLookupAttemptObserver {
    /// Number of calls that reached the attached, locked registry state.
    pub(crate) fn count(&self) -> usize {
        self.attached_attempts
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Cloneable non-owning access to the Arena view of the shared full-ID namespace.
pub struct SimulationResidentArenaLookup<A> {
    state: Weak<Mutex<RegistryState<A>>>,
}

impl<A> Clone for SimulationResidentArenaLookup<A> {
    fn clone(&self) -> Self {
        Self {
            state: Weak::clone(&self.state),
        }
    }
}

impl<A> Clone for SimulationResidentLookup<A> {
    fn clone(&self) -> Self {
        Self {
            state: Weak::clone(&self.state),
        }
    }
}

/// Non-cloneable identity for exactly one published collection lifetime.
pub struct SimulationResidentOccurrence<A> {
    state: Weak<Mutex<RegistryState<A>>>,
    id: CollectionId,
    token: SimulationResidentToken,
}

/// Effect-free refusal while retaining the exact published main-DMO-bank
/// wrappers selected at resident-wrapper construction.
#[allow(
    dead_code,
    reason = "staged exact main-DMO occurrence acquisition is not yet called by a production coordinator"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationMainDmoBankResidentSnapshotError {
    RegistryDetached,
    RegistryPoisoned,
    UnknownOccurrence,
    WrongCollection,
    Resident(OwnedResidentCollectionError),
    ResidentAssetSequenceMismatch,
}

impl fmt::Display for SimulationMainDmoBankResidentSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryDetached => {
                formatter.write_str("Simulation resident registry is detached")
            }
            Self::RegistryPoisoned => {
                formatter.write_str("Simulation resident registry is poisoned")
            }
            Self::UnknownOccurrence => {
                formatter.write_str("main DMO resident occurrence is stale or no longer published")
            }
            Self::WrongCollection => {
                formatter.write_str("published occurrence is not the exact main DMO bank")
            }
            Self::Resident(source) => write!(formatter, "resident snapshot failed: {source}"),
            Self::ResidentAssetSequenceMismatch => formatter.write_str(
                "resident assets differ from the exact wrapper-time main DMO bank sequence",
            ),
        }
    }
}

impl Error for SimulationMainDmoBankResidentSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resident(source) => Some(source),
            _ => None,
        }
    }
}

/// Retry-preserving main-DMO-bank snapshot refusal.
#[allow(
    dead_code,
    reason = "staged exact main-DMO occurrence acquisition is not yet called by a production coordinator"
)]
pub(crate) struct SimulationMainDmoBankResidentSnapshotFailure {
    error: SimulationMainDmoBankResidentSnapshotError,
    occurrence: SimulationResidentOccurrence<DmoBankProcessedArenaAsset>,
}

#[allow(
    dead_code,
    reason = "staged exact main-DMO occurrence acquisition is not yet called by a production coordinator"
)]
impl SimulationMainDmoBankResidentSnapshotFailure {
    #[must_use]
    pub(crate) const fn error(&self) -> SimulationMainDmoBankResidentSnapshotError {
        self.error
    }

    #[must_use]
    pub(crate) fn into_occurrence(
        self,
    ) -> SimulationResidentOccurrence<DmoBankProcessedArenaAsset> {
        self.occurrence
    }
}

impl fmt::Debug for SimulationMainDmoBankResidentSnapshotFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationMainDmoBankResidentSnapshotFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for SimulationMainDmoBankResidentSnapshotFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for SimulationMainDmoBankResidentSnapshotFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

/// Refusal while resampling one retained main-DMO-bank occurrence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationMainDmoBankResidentLivenessError {
    RegistryDetached,
    RegistryPoisoned,
    UnknownOccurrence,
    Resident(OwnedResidentCollectionError),
}

/// Refusal while comparing a fresh lookup with one retained published
/// main-DMO-bank occurrence and its exact wrapper-time asset sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationMainDmoBankResidentRevalidationError {
    DifferentOccurrence,
    Snapshot(SimulationMainDmoBankResidentSnapshotError),
    RetainedAssetSequenceMismatch,
}

impl fmt::Display for SimulationMainDmoBankResidentRevalidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DifferentOccurrence => {
                formatter.write_str("fresh lookup names another main DMO resident occurrence")
            }
            Self::Snapshot(source) => write!(formatter, "fresh resident snapshot failed: {source}"),
            Self::RetainedAssetSequenceMismatch => formatter.write_str(
                "fresh main DMO resident wrappers differ from the retained exact sequence",
            ),
        }
    }
}

impl Error for SimulationMainDmoBankResidentRevalidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(source) => Some(source),
            Self::DifferentOccurrence | Self::RetainedAssetSequenceMismatch => None,
        }
    }
}

impl fmt::Display for SimulationMainDmoBankResidentLivenessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryDetached => {
                formatter.write_str("Simulation resident registry is detached")
            }
            Self::RegistryPoisoned => {
                formatter.write_str("Simulation resident registry is poisoned")
            }
            Self::UnknownOccurrence => {
                formatter.write_str("main DMO resident occurrence is stale or no longer published")
            }
            Self::Resident(source) => write!(formatter, "resident liveness failed: {source}"),
        }
    }
}

impl Error for SimulationMainDmoBankResidentLivenessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resident(source) => Some(source),
            _ => None,
        }
    }
}

/// Opaque point-in-time liveness identity paired with one exact published
/// main-DMO-bank occurrence.
///
/// This is a safe-host check, not a resident hold or a native lifetime token.
/// It is intentionally non-cloneable and non-debuggable.
pub(crate) struct SimulationMainDmoBankResidentLiveness {
    state: Weak<Mutex<RegistryState<DmoBankProcessedArenaAsset>>>,
    id: CollectionId,
    token: SimulationResidentToken,
}

impl SimulationMainDmoBankResidentLiveness {
    /// Resamples registry occurrence identity and resident liveness without
    /// granting collection or asset ownership.
    pub(crate) fn try_require_alive(
        &self,
    ) -> Result<(), SimulationMainDmoBankResidentLivenessError> {
        let state = self
            .state
            .upgrade()
            .ok_or(SimulationMainDmoBankResidentLivenessError::RegistryDetached)?;
        let state = state
            .lock()
            .map_err(|_| SimulationMainDmoBankResidentLivenessError::RegistryPoisoned)?;
        let row = state
            .active
            .iter()
            .find(|row| row.id == self.id && row.token == self.token)
            .ok_or(SimulationMainDmoBankResidentLivenessError::UnknownOccurrence)?;
        row.collection
            .try_require_alive()
            .map_err(SimulationMainDmoBankResidentLivenessError::Resident)
    }

    /// Compares one freshly resolved occurrence with this exact retained
    /// identity, then resamples its wrapper-time asset sequence atomically.
    ///
    /// The returned snapshot remains opaque and is useful only for the sealed
    /// model-batch comparison. This is a safe-host pre-effect validation, not
    /// a native resident hold or a guarantee after this call returns.
    pub(crate) fn try_snapshot_same_occurrence(
        &self,
        occurrence: &SimulationResidentOccurrence<DmoBankProcessedArenaAsset>,
    ) -> Result<SimulationMainDmoBankResidentSnapshot, SimulationMainDmoBankResidentRevalidationError>
    {
        if self.id != occurrence.id
            || self.token != occurrence.token
            || !Weak::ptr_eq(&self.state, &occurrence.state)
        {
            return Err(SimulationMainDmoBankResidentRevalidationError::DifferentOccurrence);
        }
        occurrence
            .snapshot_main_dmo_bank()
            .map_err(SimulationMainDmoBankResidentRevalidationError::Snapshot)
    }
}

/// Exact wrapper-time main-DMO-bank owners behind a sealed handoff.
///
/// No generic strong-asset view is exposed. A later binding-side consumer
/// must take this value whole; it cannot be cloned or inspected through Debug.
pub(crate) struct SimulationMainDmoBankResidentSnapshot {
    liveness: SimulationMainDmoBankResidentLiveness,
    assets: Vec<Arc<DmoBankProcessedArenaAsset>>,
}

impl SimulationMainDmoBankResidentSnapshot {
    /// Compares retained wrapper owners without exposing either sequence.
    pub(crate) fn has_same_assets<'a>(
        &self,
        expected: impl ExactSizeIterator<Item = &'a Arc<DmoBankProcessedArenaAsset>>,
    ) -> bool {
        expected.len() == self.assets.len()
            && expected
                .zip(&self.assets)
                .all(|(expected, actual)| Arc::ptr_eq(expected, actual))
    }

    /// The held bank arenas in collection order, for the selected session's
    /// `Simulation::RegArena` model pass and LivingWorld body creation.
    pub(crate) fn assets(&self) -> &[Arc<DmoBankProcessedArenaAsset>] {
        &self.assets
    }
}

/// Owning refusal while the only binding-side consumer prepares a model batch.
pub(crate) struct SimulationMainDmoBankResidentSnapshotConsumeFailure {
    error: SimulationDmoBankModelRegistrationBatchError,
    snapshot: SimulationMainDmoBankResidentSnapshot,
}

impl SimulationMainDmoBankResidentSnapshotConsumeFailure {
    #[must_use]
    #[allow(
        dead_code,
        reason = "diagnostic accessor is staged for the production acquisition coordinator"
    )]
    pub(crate) const fn error(&self) -> &SimulationDmoBankModelRegistrationBatchError {
        &self.error
    }

    #[must_use]
    pub(crate) fn into_snapshot(self) -> SimulationMainDmoBankResidentSnapshot {
        self.snapshot
    }
}

impl fmt::Debug for SimulationMainDmoBankResidentSnapshotConsumeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationMainDmoBankResidentSnapshotConsumeFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for SimulationMainDmoBankResidentSnapshotConsumeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "main DMO resident snapshot consumer failed: {}",
            self.error
        )
    }
}

impl Error for SimulationMainDmoBankResidentSnapshotConsumeFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

impl SimulationMainDmoBankResidentSnapshot {
    /// Hands the snapshot to the binding subtree's unforgeable consumer.
    /// No liveness or retained-asset half is exposed on either outcome.
    pub(crate) fn consume(
        self,
        consumer: SimulationDmoBankSnapshotConsumer,
    ) -> Result<
        SimulationDmoBankModelRegistrationBatch,
        SimulationMainDmoBankResidentSnapshotConsumeFailure,
    > {
        match consumer.finish(self.liveness, self.assets) {
            Ok(batch) => Ok(batch),
            Err((error, liveness, assets)) => {
                Err(SimulationMainDmoBankResidentSnapshotConsumeFailure {
                    error,
                    snapshot: Self { liveness, assets },
                })
            }
        }
    }
}

impl<A> fmt::Debug for SimulationResidentOccurrence<A> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationResidentOccurrence")
            .field("id", &self.id)
            .field("token", &self.token)
            .finish_non_exhaustive()
    }
}

#[allow(
    dead_code,
    reason = "staged exact main-DMO occurrence acquisition is not yet called by a production coordinator"
)]
impl SimulationResidentOccurrence<DmoBankProcessedArenaAsset> {
    /// Revalidates this published occurrence and atomically retains its exact
    /// wrapper-time main-DMO-bank asset sequence without performing any
    /// registration, callback, or resident-count effect.
    ///
    /// The resulting token is a safe-host point-in-time snapshot. It is not a
    /// resident hold and makes no native lifetime or occurrence claim.
    pub(crate) fn try_snapshot_main_dmo_bank(
        self,
    ) -> Result<SimulationMainDmoBankResidentSnapshot, SimulationMainDmoBankResidentSnapshotFailure>
    {
        match self.snapshot_main_dmo_bank() {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => Err(SimulationMainDmoBankResidentSnapshotFailure {
                error,
                occurrence: self,
            }),
        }
    }

    fn snapshot_main_dmo_bank(
        &self,
    ) -> Result<SimulationMainDmoBankResidentSnapshot, SimulationMainDmoBankResidentSnapshotError>
    {
        let state = self
            .state
            .upgrade()
            .ok_or(SimulationMainDmoBankResidentSnapshotError::RegistryDetached)?;
        let state_guard = state
            .lock()
            .map_err(|_| SimulationMainDmoBankResidentSnapshotError::RegistryPoisoned)?;
        let row = state_guard
            .active
            .iter()
            .find(|row| row.id == self.id && row.token == self.token)
            .ok_or(SimulationMainDmoBankResidentSnapshotError::UnknownOccurrence)?;
        if self.id != CollectionId(DMO_BANK_COLLECTION_ID) {
            return Err(SimulationMainDmoBankResidentSnapshotError::WrongCollection);
        }
        let assets = row
            .collection
            .try_upgrade_exact_assets_in_order(&row.arena_candidates)
            .map_err(SimulationMainDmoBankResidentSnapshotError::Resident)?
            .ok_or(SimulationMainDmoBankResidentSnapshotError::ResidentAssetSequenceMismatch)?;
        Ok(SimulationMainDmoBankResidentSnapshot {
            liveness: SimulationMainDmoBankResidentLiveness {
                state: Weak::clone(&self.state),
                id: self.id,
                token: self.token.clone(),
            },
            assets,
        })
    }
}

/// Failure in the same native full-ID namespace for selected inner assets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationInnerRegistryError {
    Detached,
    Poisoned,
    Allocation,
    KindMismatch {
        id: CollectionId,
        actual: SimulationResidentContainerAssetKind,
    },
    Expired {
        id: CollectionId,
    },
}

impl<A, M, E> SimulationResidentRegistry<A, M, E> {
    /// Native `AddRegAsset82d64890` preserves the first full-ID winner.
    pub(crate) fn try_add_inner(
        &self,
        asset: &Arc<crate::simulation_inner_registry_asset::SimulationInnerRegistryAsset>,
    ) -> Result<(), SimulationInnerRegistryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SimulationInnerRegistryError::Poisoned)?;
        selected_inner::add_inner(&mut state, asset)
    }

    /// The outer arena is published after its processed inner asset list.
    pub(crate) fn try_add_selected_arena(
        &self,
        id: CollectionId,
        asset: &Arc<A>,
    ) -> Result<(), SimulationInnerRegistryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SimulationInnerRegistryError::Poisoned)?;
        if state.container.iter().any(|row| row.id == id) {
            return Ok(());
        }
        state
            .container
            .try_reserve(1)
            .map_err(|_| SimulationInnerRegistryError::Allocation)?;
        state.container.push(ContainerRegistryRow {
            id,
            asset: ContainerRegistryAsset::Arena {
                asset: Arc::downgrade(asset),
            },
        });
        Ok(())
    }

    /// `GetRegAsset82d62980` releases its lock before any consumer payload read.
    pub(crate) fn try_inner(
        &self,
        id: CollectionId,
    ) -> Result<
        Option<Arc<crate::simulation_inner_registry_asset::SimulationInnerRegistryAsset>>,
        SimulationInnerRegistryError,
    > {
        let state = self
            .state
            .lock()
            .map_err(|_| SimulationInnerRegistryError::Poisoned)?;
        selected_inner::lookup(&state, id)
    }

    /// `RemRegAsset82d63dd0` removes only the currently published same pointer.
    /// It never promotes a losing duplicate. This does not implement arena unload.
    pub(crate) fn try_remove_inner(
        &self,
        asset: &Arc<crate::simulation_inner_registry_asset::SimulationInnerRegistryAsset>,
    ) -> Result<(), SimulationInnerRegistryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SimulationInnerRegistryError::Poisoned)?;
        if let Some(index) = state.container.iter().position(|row| row.id.0 == asset.id && matches!(&row.asset,
            ContainerRegistryAsset::Inner { asset: current, .. } if Weak::ptr_eq(current, &Arc::downgrade(asset)))) {
            state.container.remove(index);
        }
        Ok(())
    }

    #[must_use]
    pub fn new(map_error: M) -> (Self, SimulationResidentLookup<A>) {
        let domain = Arc::new(SimulationResidentTokenDomain);
        let state = Arc::new(Mutex::new(RegistryState::default()));
        let lookup = SimulationResidentLookup {
            state: Arc::downgrade(&state),
        };
        (
            Self {
                selected_processor: None,
                domain,
                state,
                map_error,
                error: PhantomData,
            },
            lookup,
        )
    }

    fn map_error(&mut self, error: SimulationResidentRegistryError) -> E
    where
        M: FnMut(SimulationResidentRegistryError) -> E,
    {
        (self.map_error)(error)
    }
}

impl<A, M, E> ResidentCollectionRegistry<OwnedResidentCollection<A>, E>
    for SimulationResidentRegistry<A, M, E>
where
    M: FnMut(SimulationResidentRegistryError) -> E,
{
    type Wrapper = SimulationResidentWrapper<A>;
    type Token = SimulationResidentToken;

    fn create_wrapper(
        &mut self,
        id: CollectionId,
        collection: &OwnedResidentCollection<A>,
    ) -> Result<Self::Wrapper, E> {
        // Physical-load completion is the publication boundary in this host
        // composition. Assets appended after wrapper creation are therefore
        // outside this occurrence and are intentionally not published later.
        let arena_candidates = collection.weak_assets_in_order().map_err(|source| {
            self.map_error(SimulationResidentRegistryError::ResidentAssetSnapshot(
                source,
            ))
        })?;
        let state_owner = Arc::clone(&self.state);
        let Ok(mut state) = state_owner.lock() else {
            return Err(self.map_error(SimulationResidentRegistryError::Poisoned {
                operation: SimulationResidentRegistryOperation::CreateWrapper,
            }));
        };
        let token = SimulationResidentToken {
            domain: Arc::downgrade(&self.domain),
            serial: state.next_token,
        };
        let Some(next_token) = state.next_token.checked_add(1) else {
            drop(state);
            return Err(self.map_error(SimulationResidentRegistryError::TokenExhausted));
        };
        let requested = state.pending.len().saturating_add(1);
        if state.pending.try_reserve(1).is_err() {
            drop(state);
            return Err(
                self.map_error(SimulationResidentRegistryError::StorageAllocation {
                    storage: SimulationResidentRegistryStorage::Pending,
                    requested,
                }),
            );
        }
        let wrapper_view = collection.weak_view();
        state.pending.push(RegistryRow {
            id,
            token: token.clone(),
            collection: collection.weak_view(),
            assets: Arc::new(collection.resident_assets_view()),
            arena_candidates,
            inner_candidates: Vec::new(),
        });
        state.next_token = next_token;
        Ok(SimulationResidentWrapper {
            token,
            collection: wrapper_view,
        })
    }

    fn stable_token(wrapper: &Self::Wrapper) -> Self::Token {
        wrapper.token.clone()
    }

    #[expect(
        clippy::too_many_lines,
        reason = "ordered publication and retained failure prefix under one registry guard"
    )]
    fn publish(&mut self, id: CollectionId, token: &Self::Token) -> Result<(), E> {
        let state_owner = Arc::clone(&self.state);
        let Ok(mut state) = state_owner.lock() else {
            return Err(self.map_error(SimulationResidentRegistryError::Poisoned {
                operation: SimulationResidentRegistryOperation::Publish,
            }));
        };
        if state.failed_publication.is_some() {
            drop(state);
            return Err(
                self.map_error(SimulationResidentRegistryError::PendingSelectedPublicationFailure)
            );
        }
        let Some(pending_index) = state.pending.iter().position(|row| row.token == *token) else {
            drop(state);
            return Err(self.map_error(SimulationResidentRegistryError::UnknownPendingToken { id }));
        };
        let pending_id = state.pending[pending_index].id;
        if pending_id != id {
            drop(state);
            return Err(
                self.map_error(SimulationResidentRegistryError::PendingIdMismatch {
                    requested: id,
                    pending: pending_id,
                }),
            );
        }
        if !state.pending[pending_index].collection.is_attached() {
            drop(state);
            return Err(
                self.map_error(SimulationResidentRegistryError::DetachedPendingCollection { id })
            );
        }
        if state.active.iter().any(|row| row.id == id) {
            drop(state);
            return Err(self.map_error(SimulationResidentRegistryError::DuplicateActiveId { id }));
        }
        let requested = state.active.len().saturating_add(1);
        if state.active.try_reserve(1).is_err() {
            drop(state);
            return Err(
                self.map_error(SimulationResidentRegistryError::StorageAllocation {
                    storage: SimulationResidentRegistryStorage::Active,
                    requested,
                }),
            );
        }
        let arena_candidate_count = state.pending[pending_index].arena_candidates.len();
        let container_candidate_count = arena_candidate_count.saturating_add(1);
        let container_requested = state
            .container
            .len()
            .saturating_add(container_candidate_count);
        if state
            .container
            .try_reserve(container_candidate_count)
            .is_err()
        {
            drop(state);
            return Err(
                self.map_error(SimulationResidentRegistryError::StorageAllocation {
                    storage: SimulationResidentRegistryStorage::Container,
                    requested: container_requested,
                }),
            );
        }
        let mut row = state.pending.remove(pending_index);
        // Native ProcessArena publishes every Arena wrapper before the later
        // collection wrapper. This host's atomic batch preserves that candidate
        // order in one shared first-wins namespace, while deliberately omitting
        // native's observable mid-load publication window.
        for candidate in &row.arena_candidates {
            if let Some(processor) = &self.selected_processor {
                let result = candidate.weak().upgrade()
                    .ok_or(crate::selected_simulation_asset_services::SelectedAssetError::DetachedResidentArena)
                    .and_then(|asset| processor.try_process(&asset, &mut selected_inner::SelectedPublication {
                        state: &mut state,
                        candidate,
                        inners: &mut row.inner_candidates,
                    }));
                if let Err(source) = result {
                    // Keep the exact candidate/token and already-published weak
                    // prefix. The processor retains its strong wrapper prefix.
                    state.failed_publication = Some(row);
                    drop(state);
                    return Err(
                        self.map_error(SimulationResidentRegistryError::SelectedProcessing(source))
                    );
                }
                continue;
            }
            if state
                .container
                .iter()
                .all(|active| active.id != candidate.id())
            {
                state.container.push(ContainerRegistryRow {
                    id: candidate.id(),
                    asset: ContainerRegistryAsset::Arena {
                        asset: Weak::clone(candidate.weak()),
                    },
                });
            }
        }
        if state.container.iter().all(|active| active.id != row.id) {
            state.container.push(ContainerRegistryRow {
                id: row.id,
                asset: ContainerRegistryAsset::Collection {
                    token: row.token.clone(),
                },
            });
        }
        state.active.push(row);
        Ok(())
    }

    fn remove_exact(&mut self, id: CollectionId, token: &Self::Token) -> Result<(), E> {
        let state_owner = Arc::clone(&self.state);
        let Ok(mut state) = state_owner.lock() else {
            return Err(self.map_error(SimulationResidentRegistryError::Poisoned {
                operation: SimulationResidentRegistryOperation::Remove,
            }));
        };
        let Some(active_index) = state.active.iter().position(|row| row.id == id) else {
            drop(state);
            return Err(self.map_error(SimulationResidentRegistryError::MissingActiveId { id }));
        };
        if state.active[active_index].token != *token {
            drop(state);
            return Err(self.map_error(SimulationResidentRegistryError::TokenMismatch { id }));
        }
        let row = state.active.remove(active_index);
        // Collection-zero teardown removes the exact collection wrapper before
        // child Arena teardown. Both happen atomically in this narrower host
        // composition, but kind/allocation checks preserve native RemRegAsset's
        // exact-pointer no-op behavior and never promote a losing candidate.
        if let Some(container_index) = state.container.iter().position(|active| {
            active.id == row.id
                && matches!(
                    &active.asset,
                    ContainerRegistryAsset::Collection { token } if *token == row.token
                )
        }) {
            state.container.remove(container_index);
        }
        // Inner removals share the same atomic host teardown and require the
        // exact wrapper pointer; losing duplicates never remove a winner.
        for (id, inner) in &row.inner_candidates {
            if let Some(index) = state.container.iter().position(|entry| entry.id.0 == *id
                && matches!(&entry.asset, ContainerRegistryAsset::Inner { asset, .. } if Weak::ptr_eq(asset, inner))) {
                state.container.remove(index);
            }
        }
        for candidate in &row.arena_candidates {
            if let Some(container_index) = state.container.iter().position(|active| {
                active.id == candidate.id()
                    && matches!(
                        &active.asset,
                        ContainerRegistryAsset::Arena { asset }
                            if Weak::ptr_eq(asset, candidate.weak())
                    )
            }) {
                state.container.remove(container_index);
            }
        }
        Ok(())
    }
}

impl<A> SimulationResidentLookup<A> {
    /// Observes only Collection-lookup calls that reach this currently attached
    /// and successfully locked registry state, without retaining that state.
    #[cfg(test)]
    pub(crate) fn attached_lookup_attempt_observer_for_test(
        &self,
    ) -> SimulationResidentAttachedLookupAttemptObserver {
        let state = self
            .state
            .upgrade()
            .expect("test lookup observer requires an attached registry");
        let attached_attempts = Arc::clone(
            &state
                .lock()
                .expect("test lookup observer requires an unpoisoned registry")
                .attached_lookup_attempts,
        );
        SimulationResidentAttachedLookupAttemptObserver { attached_attempts }
    }

    /// Derives Arena lookup access to the same shared container namespace.
    #[must_use]
    pub fn arena_lookup(&self) -> SimulationResidentArenaLookup<A> {
        SimulationResidentArenaLookup {
            state: Weak::clone(&self.state),
        }
    }

    /// Resolves the currently published Collection occurrence for one full-width ID.
    /// Missing IDs remain `Ok(None)` for the receiver to classify.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the owning registry is gone, its host mutex is
    /// poisoned, or the ID is occupied by a represented non-Collection wrapper.
    pub fn lookup(
        &self,
        id: CollectionId,
    ) -> Result<Option<SimulationResidentOccurrence<A>>, SimulationResidentLookupError> {
        let state_owner = self
            .state
            .upgrade()
            .ok_or(SimulationResidentLookupError::RegistryDetached)?;
        let occurrence_state = Arc::downgrade(&state_owner);
        let state = state_owner
            .lock()
            .map_err(|_| SimulationResidentLookupError::Poisoned)?;
        #[cfg(test)]
        state
            .attached_lookup_attempts
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let Some(container_row) = state.container.iter().find(|row| row.id == id) else {
            return Ok(None);
        };
        let ContainerRegistryAsset::Collection { token } = &container_row.asset else {
            return Err(SimulationResidentLookupError::PublishedKindMismatch {
                id,
                actual: container_row.asset.kind(),
            });
        };
        Ok(state
            .active
            .iter()
            .find(|row| row.id == id && row.token == *token)
            .map(|row| SimulationResidentOccurrence {
                state: Weak::clone(&occurrence_state),
                id,
                token: row.token.clone(),
            }))
    }
}

impl<A> SimulationResidentArenaLookup<A> {
    /// Observes only Arena-lookup calls that reach this attached and
    /// successfully locked registry state, without retaining it.
    #[cfg(test)]
    pub(crate) fn attached_lookup_attempt_observer_for_test(
        &self,
    ) -> SimulationResidentAttachedArenaLookupAttemptObserver {
        let state = self
            .state
            .upgrade()
            .expect("test Arena lookup observer requires an attached registry");
        let attached_attempts = Arc::clone(
            &state
                .lock()
                .expect("test Arena lookup observer requires an unpoisoned registry")
                .attached_arena_lookup_attempts,
        );
        SimulationResidentAttachedArenaLookupAttemptObserver { attached_attempts }
    }

    /// Resolves the current first-published winner for a full-width Arena ID.
    ///
    /// The selected weak asset is upgraded under the registry lock. The lock
    /// is then released before visitor code runs, matching native lookup lock
    /// scope. The temporary strong reference is a scoped safe-host lifetime
    /// hold; native external loaded-count exclusion and delayed garbage
    /// collection remain separate lifecycle responsibilities. The visitor can
    /// explicitly clone any ownership that `A` exposes, so callers must not
    /// treat this API itself as a non-escaping ownership boundary.
    ///
    /// # Errors
    ///
    /// Returns a typed detached/poisoned/kind-mismatch/expired invariant
    /// failure or the visitor's own error. A missing ID is `Ok(None)` for the
    /// receiver to classify at its recovered assertion boundary.
    pub fn try_with_asset<R, E>(
        &self,
        id: CollectionId,
        visitor: impl FnOnce(&A) -> Result<R, E>,
    ) -> Result<Option<R>, SimulationResidentArenaLookupError<E>> {
        let state = self
            .state
            .upgrade()
            .ok_or(SimulationResidentArenaLookupError::RegistryDetached)?;
        let state_guard = state
            .lock()
            .map_err(|_| SimulationResidentArenaLookupError::Poisoned)?;
        #[cfg(test)]
        state_guard
            .attached_arena_lookup_attempts
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let Some(row) = state_guard.container.iter().find(|row| row.id == id) else {
            return Ok(None);
        };
        let ContainerRegistryAsset::Arena { asset } = &row.asset else {
            return Err(SimulationResidentArenaLookupError::PublishedKindMismatch {
                id,
                actual: row.asset.kind(),
            });
        };
        let asset = asset
            .upgrade()
            .ok_or(SimulationResidentArenaLookupError::PublishedAssetExpired { id })?;
        drop(state_guard);
        drop(state);
        visitor(&asset)
            .map(Some)
            .map_err(SimulationResidentArenaLookupError::Visitor)
    }
}

impl<A> SimulationResidentOccurrence<A> {
    #[must_use]
    pub const fn id(&self) -> CollectionId {
        self.id
    }

    /// Reports whether two capabilities name the same published lifetime in
    /// the same registry domain.
    ///
    /// Full collection ID and the private monotonic token are both checked;
    /// registry identity prevents equal-looking occurrences from independent
    /// worlds from comparing equal. This performs no lookup or registry
    /// effect.
    #[must_use]
    pub fn is_same_occurrence(&self, other: &Self) -> bool {
        self.id == other.id && self.token == other.token && Weak::ptr_eq(&self.state, &other.state)
    }

    /// Visits this exact occurrence's assets in resident insertion order.
    ///
    /// The occurrence is revalidated before a weak asset view is copied out of
    /// the registry lock. A concurrent exact removal may subsequently make
    /// that old view detached, but it can never bind to a replacement row with
    /// the same ID.
    ///
    /// # Errors
    ///
    /// Returns a typed detached/poisoned/stale error or the underlying scoped
    /// resident-asset visit failure.
    pub fn try_for_each<E>(
        &self,
        visitor: impl FnMut(&A) -> Result<(), E>,
    ) -> Result<(), SimulationResidentVisitError<E>> {
        let state = self
            .state
            .upgrade()
            .ok_or(SimulationResidentVisitError::RegistryDetached)?;
        let state_guard = state
            .lock()
            .map_err(|_| SimulationResidentVisitError::Poisoned)?;
        let row = state_guard
            .active
            .iter()
            .find(|row| row.id == self.id && row.token == self.token)
            .ok_or(SimulationResidentVisitError::UnknownOccurrence { id: self.id })?;
        let assets = Arc::clone(&row.assets);
        drop(state_guard);
        assets
            .try_for_each(visitor)
            .map_err(SimulationResidentVisitError::Assets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collection_load_pump::LoadedAssetCollection,
        collection_resident_streamer::ResidentCollectionStorage,
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
        owned_resident_collection::{
            OwnedResidentCollectionError, OwnedResidentCollectionStorage, ResidentAssetIdentity,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_normal_arena_plan::SimulationNormalArenaPlan,
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::{convert::Infallible, sync::Arc};

    #[derive(Debug)]
    struct TestAsset(CollectionId);

    impl ResidentAssetIdentity for TestAsset {
        fn resident_asset_id(&self) -> CollectionId {
            self.0
        }
    }

    type TestStorage = OwnedResidentCollectionStorage<
        TestAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;
    type TestRegistry = SimulationResidentRegistry<
        TestAsset,
        fn(SimulationResidentRegistryError) -> SimulationResidentRegistryError,
        SimulationResidentRegistryError,
    >;
    type DmoStorage = OwnedResidentCollectionStorage<
        DmoBankProcessedArenaAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;
    type DmoRegistry = SimulationResidentRegistry<
        DmoBankProcessedArenaAsset,
        fn(SimulationResidentRegistryError) -> SimulationResidentRegistryError,
        SimulationResidentRegistryError,
    >;

    fn storage() -> TestStorage {
        OwnedResidentCollectionStorage::new(std::convert::identity)
    }

    fn registry() -> (TestRegistry, SimulationResidentLookup<TestAsset>) {
        SimulationResidentRegistry::new(std::convert::identity)
    }

    fn collection(storage: &mut TestStorage, ids: &[u64]) -> OwnedResidentCollection<TestAsset> {
        let collection = storage
            .create_empty(i32::try_from(ids.len()).unwrap())
            .unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        for id in ids {
            writer.add_asset(TestAsset(CollectionId(*id)));
        }
        collection
    }

    fn dmo_asset(asset_id: u64) -> DmoBankProcessedArenaAsset {
        let resources = std::array::from_fn(|_| Vec::new());
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
            entry_count: 0,
            used_entry_count: 0,
            alignment: 0x10,
            virtual_size: 0,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: [ResourceDescriptor {
                size: 0,
                alignment: 0x10,
            }; 5],
            resources_used: [ResourceDescriptor {
                size: 0,
                alignment: 0x10,
            }; 5],
            resource_offsets: [u32::MAX; 5],
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        let collision = SimulationCollisionAsset {
            decoded: DecodedArenaAsset::from_parts_for_test(
                asset_id,
                WORLD_STREAM_ARENA_ASSET_TYPE,
                ArenaResources { resources },
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
        let plan = Arc::new(
            SimulationNormalArenaPlan::from_parts_with_record_sources_for_test(
                static_plan,
                Vec::new(),
                Vec::new(),
            ),
        );
        DmoBankProcessedArenaAsset::from_parts_for_test(plan, Vec::new())
    }

    fn dmo_collection(
        storage: &mut DmoStorage,
        ids: &[u64],
    ) -> OwnedResidentCollection<DmoBankProcessedArenaAsset> {
        let collection = storage
            .create_empty(i32::try_from(ids.len()).unwrap())
            .unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        for id in ids {
            writer.add_asset(dmo_asset(*id));
        }
        collection
    }

    fn dmo_registry() -> (
        DmoRegistry,
        SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    ) {
        SimulationResidentRegistry::new(std::convert::identity)
    }

    fn main_dmo_snapshot_failure(
        result: Result<
            SimulationMainDmoBankResidentSnapshot,
            SimulationMainDmoBankResidentSnapshotFailure,
        >,
    ) -> SimulationMainDmoBankResidentSnapshotFailure {
        let Err(failure) = result else {
            panic!("main DMO occurrence unexpectedly produced a snapshot");
        };
        failure
    }

    fn visited(occurrence: &SimulationResidentOccurrence<TestAsset>) -> Vec<CollectionId> {
        let mut ids = Vec::new();
        occurrence
            .try_for_each::<Infallible>(|asset| {
                ids.push(asset.0);
                Ok(())
            })
            .unwrap();
        ids
    }

    fn arena_pointer(
        lookup: &SimulationResidentArenaLookup<TestAsset>,
        id: CollectionId,
    ) -> Option<usize> {
        lookup
            .try_with_asset::<_, Infallible>(id, |asset| Ok(std::ptr::from_ref(asset) as usize))
            .unwrap()
    }

    fn container_kind(
        registry: &TestRegistry,
        id: CollectionId,
    ) -> Option<SimulationResidentContainerAssetKind> {
        registry
            .state
            .lock()
            .unwrap()
            .container
            .iter()
            .find(|row| row.id == id)
            .map(|row| row.asset.kind())
    }

    #[test]
    fn unpublished_wrapper_is_invisible_and_publication_preserves_order_and_duplicates() {
        let id = CollectionId(0xfedc_ba98_7654_3210);
        let mut storage = storage();
        let collection = collection(&mut storage, &[4, 9, 4]);
        let (mut registry, lookup) = registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestRegistry::stable_token(&wrapper);

        assert!(lookup.lookup(id).unwrap().is_none());
        assert!(wrapper.is_collection_attached());
        registry.publish(id, &token).unwrap();
        let occurrence = lookup.lookup(id).unwrap().unwrap();
        assert_eq!(occurrence.id(), id);
        assert_eq!(
            visited(&occurrence),
            [CollectionId(4), CollectionId(9), CollectionId(4)]
        );

        registry.remove_exact(id, &token).unwrap();
        assert!(lookup.lookup(id).unwrap().is_none());
        assert_eq!(
            occurrence.try_for_each::<Infallible>(|_| Ok(())),
            Err(SimulationResidentVisitError::UnknownOccurrence { id })
        );
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn full_width_ids_with_equal_low_words_remain_distinct() {
        let first_id = CollectionId(0x1111_2222_5566_7788);
        let second_id = CollectionId(0x9999_aaaa_5566_7788);
        let mut storage = storage();
        let first = collection(&mut storage, &[1]);
        let second = collection(&mut storage, &[2]);
        let (mut registry, lookup) = registry();
        let first_wrapper = registry.create_wrapper(first_id, &first).unwrap();
        let second_wrapper = registry.create_wrapper(second_id, &second).unwrap();
        let first_token = TestRegistry::stable_token(&first_wrapper);
        let second_token = TestRegistry::stable_token(&second_wrapper);
        registry.publish(first_id, &first_token).unwrap();
        registry.publish(second_id, &second_token).unwrap();

        assert_eq!(
            visited(&lookup.lookup(first_id).unwrap().unwrap()),
            [CollectionId(1)]
        );
        assert_eq!(
            visited(&lookup.lookup(second_id).unwrap().unwrap()),
            [CollectionId(2)]
        );

        registry.remove_exact(first_id, &first_token).unwrap();
        registry.remove_exact(second_id, &second_token).unwrap();
        drop((first_wrapper, second_wrapper));
        storage.destroy(first);
        storage.destroy(second);
    }

    #[test]
    fn authored_arena_candidate_wins_its_collection_id_in_the_shared_namespace() {
        let shared_id = CollectionId(0xaaaa_bbbb_cccc_dddd);
        let mut storage = storage();
        let collection = collection(&mut storage, &[shared_id.0]);
        let (mut registry, lookup) = registry();
        let arenas = lookup.arena_lookup();
        let wrapper = registry.create_wrapper(shared_id, &collection).unwrap();
        let token = TestRegistry::stable_token(&wrapper);

        registry.publish(shared_id, &token).unwrap();
        assert_eq!(
            container_kind(&registry, shared_id),
            Some(SimulationResidentContainerAssetKind::Arena)
        );
        assert!(matches!(
            lookup.lookup(shared_id),
            Err(SimulationResidentLookupError::PublishedKindMismatch {
                id,
                actual: SimulationResidentContainerAssetKind::Arena,
            }) if id == shared_id
        ));
        assert!(arena_pointer(&arenas, shared_id).is_some());

        registry.remove_exact(shared_id, &token).unwrap();
        assert_eq!(container_kind(&registry, shared_id), None);
        assert!(lookup.lookup(shared_id).unwrap().is_none());
        assert_eq!(arena_pointer(&arenas, shared_id), None);
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn published_collection_makes_a_later_cross_kind_arena_candidate_a_silent_loser() {
        let shared_id = CollectionId(0x1111_2222_3333_4444);
        let later_collection_id = CollectionId(0x5555);
        let mut storage = storage();
        let first = collection(&mut storage, &[0xaaaa]);
        let later = collection(&mut storage, &[shared_id.0]);
        let (mut registry, lookup) = registry();
        let arenas = lookup.arena_lookup();
        let first_wrapper = registry.create_wrapper(shared_id, &first).unwrap();
        let later_wrapper = registry
            .create_wrapper(later_collection_id, &later)
            .unwrap();
        let first_token = TestRegistry::stable_token(&first_wrapper);
        let later_token = TestRegistry::stable_token(&later_wrapper);

        registry.publish(shared_id, &first_token).unwrap();
        let first_occurrence = lookup.lookup(shared_id).unwrap().unwrap();
        registry.publish(later_collection_id, &later_token).unwrap();
        assert_eq!(
            container_kind(&registry, shared_id),
            Some(SimulationResidentContainerAssetKind::Collection)
        );
        assert!(
            lookup
                .lookup(shared_id)
                .unwrap()
                .unwrap()
                .is_same_occurrence(&first_occurrence)
        );
        assert_eq!(
            arenas.try_with_asset::<_, Infallible>(shared_id, |_| Ok(())),
            Err(SimulationResidentArenaLookupError::PublishedKindMismatch {
                id: shared_id,
                actual: SimulationResidentContainerAssetKind::Collection,
            })
        );

        registry
            .remove_exact(later_collection_id, &later_token)
            .unwrap();
        assert_eq!(
            container_kind(&registry, shared_id),
            Some(SimulationResidentContainerAssetKind::Collection)
        );
        registry.remove_exact(shared_id, &first_token).unwrap();
        assert_eq!(container_kind(&registry, shared_id), None);
        drop((first_wrapper, later_wrapper));
        storage.destroy(first);
        storage.destroy(later);
    }

    #[test]
    fn published_arena_makes_a_later_cross_kind_collection_candidate_a_silent_loser() {
        let first_collection_id = CollectionId(0x1111);
        let shared_id = CollectionId(0xaaaa_bbbb_cccc_dddd);
        let mut storage = storage();
        let first = collection(&mut storage, &[shared_id.0]);
        let later = collection(&mut storage, &[0xbbbb]);
        let (mut registry, lookup) = registry();
        let arenas = lookup.arena_lookup();
        let first_wrapper = registry
            .create_wrapper(first_collection_id, &first)
            .unwrap();
        let later_wrapper = registry.create_wrapper(shared_id, &later).unwrap();
        let first_token = TestRegistry::stable_token(&first_wrapper);
        let later_token = TestRegistry::stable_token(&later_wrapper);

        registry.publish(first_collection_id, &first_token).unwrap();
        let first_arena_pointer = arena_pointer(&arenas, shared_id).unwrap();
        registry.publish(shared_id, &later_token).unwrap();
        assert!(matches!(
            lookup.lookup(shared_id),
            Err(SimulationResidentLookupError::PublishedKindMismatch {
                id,
                actual: SimulationResidentContainerAssetKind::Arena,
            }) if id == shared_id
        ));
        assert_eq!(arena_pointer(&arenas, shared_id), Some(first_arena_pointer));

        registry.remove_exact(shared_id, &later_token).unwrap();
        assert_eq!(arena_pointer(&arenas, shared_id), Some(first_arena_pointer));
        registry
            .remove_exact(first_collection_id, &first_token)
            .unwrap();
        assert_eq!(container_kind(&registry, shared_id), None);
        drop((first_wrapper, later_wrapper));
        storage.destroy(first);
        storage.destroy(later);
    }

    #[test]
    fn arena_registry_is_full_width_first_wins_and_exact_pointer_removal() {
        let arena_id = CollectionId(0xaaaa_bbbb_5566_7788);
        let other_high = CollectionId(0x1111_2222_5566_7788);
        let mut storage = storage();
        let first = collection(&mut storage, &[arena_id.0, other_high.0]);
        let losing = collection(&mut storage, &[arena_id.0]);
        let replacement = collection(&mut storage, &[arena_id.0]);
        let (mut registry, lookup) = registry();
        let arenas = lookup.arena_lookup();
        let first_wrapper = registry.create_wrapper(CollectionId(1), &first).unwrap();
        let losing_wrapper = registry.create_wrapper(CollectionId(2), &losing).unwrap();
        let replacement_wrapper = registry
            .create_wrapper(CollectionId(3), &replacement)
            .unwrap();
        let first_token = TestRegistry::stable_token(&first_wrapper);
        let losing_token = TestRegistry::stable_token(&losing_wrapper);
        let replacement_token = TestRegistry::stable_token(&replacement_wrapper);

        assert_eq!(arena_pointer(&arenas, arena_id), None);
        registry.publish(CollectionId(1), &first_token).unwrap();
        let first_pointer = arena_pointer(&arenas, arena_id).unwrap();
        assert!(arena_pointer(&arenas, other_high).is_some());
        registry.publish(CollectionId(2), &losing_token).unwrap();
        assert_eq!(arena_pointer(&arenas, arena_id), Some(first_pointer));

        registry
            .remove_exact(CollectionId(1), &first_token)
            .unwrap();
        assert_eq!(arena_pointer(&arenas, arena_id), None);

        registry
            .publish(CollectionId(3), &replacement_token)
            .unwrap();
        let replacement_pointer = arena_pointer(&arenas, arena_id).unwrap();
        assert_ne!(replacement_pointer, first_pointer);
        registry
            .remove_exact(CollectionId(2), &losing_token)
            .unwrap();
        assert_eq!(arena_pointer(&arenas, arena_id), Some(replacement_pointer));

        registry
            .remove_exact(CollectionId(3), &replacement_token)
            .unwrap();
        drop((first_wrapper, losing_wrapper, replacement_wrapper));
        storage.destroy(first);
        storage.destroy(losing);
        storage.destroy(replacement);
    }

    #[test]
    fn duplicate_arenas_in_one_collection_never_promote_the_loser() {
        let arena_id = CollectionId(9);
        let mut storage = storage();
        let collection = collection(&mut storage, &[arena_id.0, arena_id.0]);
        let (mut registry, lookup) = registry();
        let arenas = lookup.arena_lookup();
        let wrapper = registry
            .create_wrapper(CollectionId(1), &collection)
            .unwrap();
        let token = TestRegistry::stable_token(&wrapper);
        registry.publish(CollectionId(1), &token).unwrap();
        assert!(arena_pointer(&arenas, arena_id).is_some());
        assert_eq!(
            registry
                .state
                .lock()
                .unwrap()
                .container
                .iter()
                .filter(|row| matches!(row.asset, ContainerRegistryAsset::Arena { .. }))
                .count(),
            1
        );
        registry.remove_exact(CollectionId(1), &token).unwrap();
        assert_eq!(arena_pointer(&arenas, arena_id), None);
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn arena_lookup_unlocks_before_visitor_and_retains_selected_asset_for_the_call() {
        let arena_id = CollectionId(9);
        let mut storage = storage();
        let collection = collection(&mut storage, &[arena_id.0]);
        let (mut registry, lookup) = registry();
        let arenas = lookup.arena_lookup();
        let wrapper = registry
            .create_wrapper(CollectionId(1), &collection)
            .unwrap();
        let token = TestRegistry::stable_token(&wrapper);
        registry.publish(CollectionId(1), &token).unwrap();

        let observed = arenas
            .try_with_asset::<_, Infallible>(arena_id, |asset| {
                registry.remove_exact(CollectionId(1), &token).unwrap();
                storage.destroy(collection);
                assert_eq!(asset.0, arena_id);
                Ok(asset.0)
            })
            .unwrap();
        assert_eq!(observed, Some(arena_id));
        assert_eq!(arena_pointer(&arenas, arena_id), None);
        drop(wrapper);
    }

    #[test]
    fn expired_arena_winner_is_not_deleted_or_promoted_by_lookup() {
        let arena_id = CollectionId(9);
        let mut storage = storage();
        let collection = collection(&mut storage, &[arena_id.0]);
        let (mut registry, lookup) = registry();
        let arenas = lookup.arena_lookup();
        let wrapper = registry
            .create_wrapper(CollectionId(1), &collection)
            .unwrap();
        let token = TestRegistry::stable_token(&wrapper);
        registry.publish(CollectionId(1), &token).unwrap();
        storage.destroy(collection);

        assert_eq!(
            arenas.try_with_asset::<_, Infallible>(arena_id, |_| Ok(())),
            Err(SimulationResidentArenaLookupError::PublishedAssetExpired { id: arena_id })
        );
        assert_eq!(
            registry
                .state
                .lock()
                .unwrap()
                .container
                .iter()
                .filter(|row| matches!(row.asset, ContainerRegistryAsset::Arena { .. }))
                .count(),
            1
        );
        registry.remove_exact(CollectionId(1), &token).unwrap();
        drop(wrapper);
    }

    #[test]
    fn wrong_id_and_token_removal_are_zero_effect() {
        let id = CollectionId(7);
        let mut storage = storage();
        let collection = collection(&mut storage, &[1]);
        let (mut registry, lookup) = registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();

        assert_eq!(
            registry.remove_exact(CollectionId(8), &token),
            Err(SimulationResidentRegistryError::MissingActiveId {
                id: CollectionId(8),
            })
        );
        assert_eq!(
            registry.remove_exact(
                id,
                &SimulationResidentToken {
                    domain: Weak::clone(&token.domain),
                    serial: token.serial + 1,
                },
            ),
            Err(SimulationResidentRegistryError::TokenMismatch { id })
        );
        assert_eq!(
            visited(&lookup.lookup(id).unwrap().unwrap()),
            [CollectionId(1)]
        );

        registry.remove_exact(id, &token).unwrap();
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn stale_occurrence_and_remove_cannot_bind_a_same_id_reload() {
        let id = CollectionId(0x1234_5678_9abc_def0);
        let mut storage = storage();
        let first = collection(&mut storage, &[11]);
        let (mut registry, lookup) = registry();
        let first_wrapper = registry.create_wrapper(id, &first).unwrap();
        let first_token = TestRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let stale = lookup.lookup(id).unwrap().unwrap();
        registry.remove_exact(id, &first_token).unwrap();
        drop(first_wrapper);
        storage.destroy(first);

        let second = collection(&mut storage, &[22]);
        let second_wrapper = registry.create_wrapper(id, &second).unwrap();
        let second_token = TestRegistry::stable_token(&second_wrapper);
        assert_ne!(first_token, second_token);
        registry.publish(id, &second_token).unwrap();

        assert_eq!(
            stale.try_for_each::<Infallible>(|_| Ok(())),
            Err(SimulationResidentVisitError::UnknownOccurrence { id })
        );
        assert_eq!(
            registry.remove_exact(id, &first_token),
            Err(SimulationResidentRegistryError::TokenMismatch { id })
        );
        assert_eq!(
            visited(&lookup.lookup(id).unwrap().unwrap()),
            [CollectionId(22)]
        );

        registry.remove_exact(id, &second_token).unwrap();
        drop(second_wrapper);
        storage.destroy(second);
    }

    #[test]
    fn duplicate_publication_retains_pending_and_original_active_rows() {
        let id = CollectionId(7);
        let mut storage = storage();
        let first = collection(&mut storage, &[1]);
        let second = collection(&mut storage, &[2]);
        let (mut registry, lookup) = registry();
        let first_wrapper = registry.create_wrapper(id, &first).unwrap();
        let second_wrapper = registry.create_wrapper(id, &second).unwrap();
        let first_token = TestRegistry::stable_token(&first_wrapper);
        let second_token = TestRegistry::stable_token(&second_wrapper);
        registry.publish(id, &first_token).unwrap();

        assert_eq!(
            registry.publish(id, &second_token),
            Err(SimulationResidentRegistryError::DuplicateActiveId { id })
        );
        assert_eq!(
            visited(&lookup.lookup(id).unwrap().unwrap()),
            [CollectionId(1)]
        );
        let state = registry.state.lock().unwrap();
        assert_eq!(state.pending.len(), 1);
        assert_eq!(state.active.len(), 1);
        drop(state);

        registry.remove_exact(id, &first_token).unwrap();
        registry.publish(id, &second_token).unwrap();
        assert_eq!(
            visited(&lookup.lookup(id).unwrap().unwrap()),
            [CollectionId(2)]
        );
        registry.remove_exact(id, &second_token).unwrap();
        drop((first_wrapper, second_wrapper));
        storage.destroy(first);
        storage.destroy(second);
    }

    #[test]
    fn pending_identity_failures_leave_the_exact_row_staged() {
        let id = CollectionId(7);
        let mut storage = storage();
        let collection = collection(&mut storage, &[1]);
        let (mut registry, lookup) = registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestRegistry::stable_token(&wrapper);

        assert_eq!(
            registry.publish(
                id,
                &SimulationResidentToken {
                    domain: Weak::clone(&token.domain),
                    serial: token.serial + 1,
                },
            ),
            Err(SimulationResidentRegistryError::UnknownPendingToken { id })
        );
        assert_eq!(
            registry.publish(CollectionId(8), &token),
            Err(SimulationResidentRegistryError::PendingIdMismatch {
                requested: CollectionId(8),
                pending: id,
            })
        );
        let state = registry.state.lock().unwrap();
        assert_eq!(state.pending.len(), 1);
        assert!(state.active.is_empty());
        drop(state);

        registry.publish(id, &token).unwrap();
        assert!(lookup.lookup(id).unwrap().is_some());
        registry.remove_exact(id, &token).unwrap();
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn detached_pending_collection_is_not_published() {
        let id = CollectionId(7);
        let mut storage = storage();
        let collection = collection(&mut storage, &[1]);
        let (mut registry, lookup) = registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestRegistry::stable_token(&wrapper);
        storage.destroy(collection);
        assert!(!wrapper.is_collection_attached());

        assert_eq!(
            registry.publish(id, &token),
            Err(SimulationResidentRegistryError::DetachedPendingCollection { id })
        );
        assert!(lookup.lookup(id).unwrap().is_none());
        let state = registry.state.lock().unwrap();
        assert_eq!(state.pending.len(), 1);
        assert!(state.active.is_empty());
    }

    #[test]
    fn visitor_error_is_preserved_and_occurrence_does_not_own_registry() {
        let id = CollectionId(7);
        let mut storage = storage();
        let collection = collection(&mut storage, &[1]);
        let (mut registry, lookup) = registry();
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = TestRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let occurrence = lookup.lookup(id).unwrap().unwrap();

        assert_eq!(
            occurrence.try_for_each(|_| Err("visitor stopped")),
            Err(SimulationResidentVisitError::Assets(
                OwnedResidentAssetsVisitError::Visitor("visitor stopped")
            ))
        );
        drop(registry);
        assert_eq!(
            occurrence.try_for_each::<Infallible>(|_| Ok(())),
            Err(SimulationResidentVisitError::RegistryDetached)
        );
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn token_exhaustion_has_no_staging_or_counter_effect() {
        let mut storage = storage();
        let collection = collection(&mut storage, &[]);
        let (mut registry, _) = registry();
        registry.state.lock().unwrap().next_token = u64::MAX;

        assert!(matches!(
            registry.create_wrapper(CollectionId(1), &collection),
            Err(SimulationResidentRegistryError::TokenExhausted)
        ));
        let state = registry.state.lock().unwrap();
        assert_eq!(state.next_token, u64::MAX);
        assert!(state.pending.is_empty());
        assert!(state.active.is_empty());
        drop(state);
        storage.destroy(collection);
    }

    #[test]
    fn equal_serial_foreign_token_cannot_publish_pending_row() {
        let id = CollectionId(7);
        let mut storage = storage();
        let first_collection = collection(&mut storage, &[1]);
        let second_collection = collection(&mut storage, &[2]);
        let (mut first_registry, first_lookup) = registry();
        let (mut second_registry, second_lookup) = registry();
        let first_wrapper = first_registry
            .create_wrapper(id, &first_collection)
            .unwrap();
        let second_wrapper = second_registry
            .create_wrapper(id, &second_collection)
            .unwrap();
        let first_token = TestRegistry::stable_token(&first_wrapper);
        let repeated_first_token = TestRegistry::stable_token(&first_wrapper);
        let second_token = TestRegistry::stable_token(&second_wrapper);
        assert_eq!(first_token, repeated_first_token);
        assert_eq!(first_token.serial, second_token.serial);
        assert_ne!(first_token, second_token);

        assert_eq!(
            first_registry.publish(id, &second_token),
            Err(SimulationResidentRegistryError::UnknownPendingToken { id })
        );
        assert!(first_lookup.lookup(id).unwrap().is_none());
        assert!(second_lookup.lookup(id).unwrap().is_none());
        let first_state = first_registry.state.lock().unwrap();
        assert_eq!(first_state.pending.len(), 1);
        assert!(first_state.active.is_empty());
        drop(first_state);

        first_registry.publish(id, &repeated_first_token).unwrap();
        second_registry.publish(id, &second_token).unwrap();
        assert_eq!(
            visited(&first_lookup.lookup(id).unwrap().unwrap()),
            [CollectionId(1)]
        );
        assert_eq!(
            visited(&second_lookup.lookup(id).unwrap().unwrap()),
            [CollectionId(2)]
        );
        first_registry.remove_exact(id, &first_token).unwrap();
        second_registry.remove_exact(id, &second_token).unwrap();
        drop((first_wrapper, second_wrapper));
        storage.destroy(first_collection);
        storage.destroy(second_collection);
    }

    #[test]
    fn equal_serial_foreign_token_cannot_remove_active_row() {
        let id = CollectionId(7);
        let mut storage = storage();
        let first_collection = collection(&mut storage, &[1]);
        let second_collection = collection(&mut storage, &[2]);
        let (mut first_registry, first_lookup) = registry();
        let (mut second_registry, second_lookup) = registry();
        let first_wrapper = first_registry
            .create_wrapper(id, &first_collection)
            .unwrap();
        let second_wrapper = second_registry
            .create_wrapper(id, &second_collection)
            .unwrap();
        let first_token = TestRegistry::stable_token(&first_wrapper);
        let second_token = TestRegistry::stable_token(&second_wrapper);
        first_registry.publish(id, &first_token).unwrap();
        second_registry.publish(id, &second_token).unwrap();

        assert_eq!(
            first_registry.remove_exact(CollectionId(8), &second_token),
            Err(SimulationResidentRegistryError::MissingActiveId {
                id: CollectionId(8)
            })
        );
        assert_eq!(
            first_registry.remove_exact(id, &second_token),
            Err(SimulationResidentRegistryError::TokenMismatch { id })
        );
        assert_eq!(
            visited(&first_lookup.lookup(id).unwrap().unwrap()),
            [CollectionId(1)]
        );
        assert_eq!(
            visited(&second_lookup.lookup(id).unwrap().unwrap()),
            [CollectionId(2)]
        );

        first_registry.remove_exact(id, &first_token).unwrap();
        second_registry.remove_exact(id, &second_token).unwrap();
        drop((first_wrapper, second_wrapper));
        storage.destroy(first_collection);
        storage.destroy(second_collection);
    }

    #[test]
    fn stale_token_domain_is_non_owning_and_cannot_alias_a_new_registry() {
        let id = CollectionId(7);
        let mut storage = storage();
        let stale_collection = collection(&mut storage, &[1]);
        let current_collection = collection(&mut storage, &[2]);
        let (mut stale_registry, stale_lookup) = registry();
        let stale_wrapper = stale_registry
            .create_wrapper(id, &stale_collection)
            .unwrap();
        let stale_token = TestRegistry::stable_token(&stale_wrapper);
        let stale_domain = Weak::clone(&stale_token.domain);
        drop(stale_registry);
        assert!(stale_domain.upgrade().is_none());
        assert!(matches!(
            stale_lookup.lookup(id),
            Err(SimulationResidentLookupError::RegistryDetached)
        ));

        let (mut current_registry, current_lookup) = registry();
        let current_wrapper = current_registry
            .create_wrapper(id, &current_collection)
            .unwrap();
        let current_token = TestRegistry::stable_token(&current_wrapper);
        assert_eq!(stale_token.serial, current_token.serial);
        assert_ne!(stale_token, current_token);
        assert_eq!(
            current_registry.publish(id, &stale_token),
            Err(SimulationResidentRegistryError::UnknownPendingToken { id })
        );
        assert!(current_lookup.lookup(id).unwrap().is_none());
        current_registry.publish(id, &current_token).unwrap();
        current_registry.remove_exact(id, &current_token).unwrap();

        drop((stale_wrapper, current_wrapper));
        storage.destroy(stale_collection);
        storage.destroy(current_collection);
    }

    #[test]
    fn poisoned_registry_precedes_foreign_token_validation() {
        let id = CollectionId(7);
        let mut storage = storage();
        let first_collection = collection(&mut storage, &[1]);
        let second_collection = collection(&mut storage, &[2]);
        let (mut first_registry, _) = registry();
        let (mut second_registry, _) = registry();
        let first_wrapper = first_registry
            .create_wrapper(id, &first_collection)
            .unwrap();
        let second_wrapper = second_registry
            .create_wrapper(id, &second_collection)
            .unwrap();
        let foreign_token = TestRegistry::stable_token(&second_wrapper);
        let poison_state = Arc::clone(&first_registry.state);
        let _ = std::thread::spawn(move || {
            let _guard = poison_state.lock().unwrap();
            panic!("poison registry");
        })
        .join();

        assert_eq!(
            first_registry.publish(id, &foreign_token),
            Err(SimulationResidentRegistryError::Poisoned {
                operation: SimulationResidentRegistryOperation::Publish
            })
        );
        assert_eq!(
            first_registry.remove_exact(id, &foreign_token),
            Err(SimulationResidentRegistryError::Poisoned {
                operation: SimulationResidentRegistryOperation::Remove
            })
        );

        drop((
            first_registry,
            second_registry,
            first_wrapper,
            second_wrapper,
        ));
        storage.destroy(first_collection);
        storage.destroy(second_collection);
    }

    #[test]
    fn occurrence_identity_includes_registry_domain_and_reload_token() {
        let id = CollectionId(7);
        let mut storage = storage();
        let first = collection(&mut storage, &[]);
        let second = collection(&mut storage, &[]);
        let (mut primary_registry, lookup) = registry();
        let first_wrapper = primary_registry.create_wrapper(id, &first).unwrap();
        let first_token = TestRegistry::stable_token(&first_wrapper);
        primary_registry.publish(id, &first_token).unwrap();
        let first_occurrence = lookup.lookup(id).unwrap().unwrap();
        let repeated_lookup = lookup.lookup(id).unwrap().unwrap();
        assert!(first_occurrence.is_same_occurrence(&repeated_lookup));

        let (mut other_registry, other_lookup) = registry();
        let other_wrapper = other_registry.create_wrapper(id, &second).unwrap();
        let other_token = TestRegistry::stable_token(&other_wrapper);
        other_registry.publish(id, &other_token).unwrap();
        let other_occurrence = other_lookup.lookup(id).unwrap().unwrap();
        assert!(!first_occurrence.is_same_occurrence(&other_occurrence));

        primary_registry.remove_exact(id, &first_token).unwrap();
        let replacement = collection(&mut storage, &[]);
        let replacement_wrapper = primary_registry.create_wrapper(id, &replacement).unwrap();
        let replacement_token = TestRegistry::stable_token(&replacement_wrapper);
        primary_registry.publish(id, &replacement_token).unwrap();
        let replacement_occurrence = lookup.lookup(id).unwrap().unwrap();
        assert!(!first_occurrence.is_same_occurrence(&replacement_occurrence));

        primary_registry
            .remove_exact(id, &replacement_token)
            .unwrap();
        other_registry.remove_exact(id, &other_token).unwrap();
        drop((first_wrapper, replacement_wrapper, other_wrapper));
        storage.destroy(first);
        storage.destroy(second);
        storage.destroy(replacement);
    }

    #[test]
    fn main_dmo_snapshot_retains_exact_wrapper_time_order_and_outer_owners() {
        let mut storage = DmoStorage::new(std::convert::identity);
        let collection = dmo_collection(&mut storage, &[0xbb, 0xaa, 0xbb]);
        let expected = collection.weak_assets_in_order().unwrap();
        let (mut registry, lookup) = dmo_registry();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();

        let snapshot = lookup
            .lookup(id)
            .unwrap()
            .unwrap()
            .try_snapshot_main_dmo_bank()
            .unwrap();
        assert_eq!(snapshot.assets.len(), 3);
        assert_eq!(
            snapshot
                .assets
                .iter()
                .map(|asset| asset.resident_asset_id())
                .collect::<Vec<_>>(),
            [CollectionId(0xbb), CollectionId(0xaa), CollectionId(0xbb)]
        );
        assert!(
            snapshot
                .assets
                .iter()
                .zip(&expected)
                .all(|(asset, expected)| Weak::ptr_eq(&Arc::downgrade(asset), expected.weak()))
        );
        snapshot.liveness.try_require_alive().unwrap();

        registry.remove_exact(id, &token).unwrap();
        assert_eq!(
            snapshot.liveness.try_require_alive(),
            Err(SimulationMainDmoBankResidentLivenessError::UnknownOccurrence)
        );
        drop(wrapper);
        storage.destroy(collection);
        assert!(
            expected
                .iter()
                .all(|asset| asset.weak().upgrade().is_some())
        );
        drop(snapshot);
        assert!(
            expected
                .iter()
                .all(|asset| asset.weak().upgrade().is_none())
        );
    }

    #[test]
    fn main_dmo_snapshot_revalidates_before_id_gate_and_returns_occurrence() {
        let mut storage = DmoStorage::new(std::convert::identity);
        let collection = dmo_collection(&mut storage, &[7]);
        let (mut registry, lookup) = dmo_registry();
        let wrong_id = CollectionId(0x1111_2222_3333_4444);
        let wrapper = registry.create_wrapper(wrong_id, &collection).unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        registry.publish(wrong_id, &token).unwrap();
        let occurrence = lookup.lookup(wrong_id).unwrap().unwrap();

        let failure = main_dmo_snapshot_failure(occurrence.try_snapshot_main_dmo_bank());
        assert_eq!(
            failure.error(),
            SimulationMainDmoBankResidentSnapshotError::WrongCollection
        );
        let occurrence = failure.into_occurrence();
        let mut seen = Vec::new();
        occurrence
            .try_for_each::<Infallible>(|asset| {
                seen.push(asset.resident_asset_id());
                Ok(())
            })
            .unwrap();
        assert_eq!(seen, [CollectionId(7)]);

        registry.remove_exact(wrong_id, &token).unwrap();
        assert_eq!(
            main_dmo_snapshot_failure(occurrence.try_snapshot_main_dmo_bank()).error(),
            SimulationMainDmoBankResidentSnapshotError::UnknownOccurrence
        );
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn main_dmo_snapshot_refuses_wrapper_time_sequence_mutation_atomically() {
        let mut storage = DmoStorage::new(std::convert::identity);
        let collection = dmo_collection(&mut storage, &[1]);
        let (mut registry, lookup) = dmo_registry();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(dmo_asset(2));
        registry.publish(id, &token).unwrap();

        let failure = main_dmo_snapshot_failure(
            lookup
                .lookup(id)
                .unwrap()
                .unwrap()
                .try_snapshot_main_dmo_bank(),
        );
        assert_eq!(
            failure.error(),
            SimulationMainDmoBankResidentSnapshotError::ResidentAssetSequenceMismatch
        );
        let occurrence = failure.into_occurrence();
        let mut seen = 0;
        occurrence
            .try_for_each::<Infallible>(|_| {
                seen += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(seen, 2);

        registry.remove_exact(id, &token).unwrap();
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn stale_main_dmo_occurrence_cannot_alias_same_id_reload() {
        let mut storage = DmoStorage::new(std::convert::identity);
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let first = dmo_collection(&mut storage, &[1]);
        let (mut registry, lookup) = dmo_registry();
        let first_wrapper = registry.create_wrapper(id, &first).unwrap();
        let first_token = DmoRegistry::stable_token(&first_wrapper);
        registry.publish(id, &first_token).unwrap();
        let stale = lookup.lookup(id).unwrap().unwrap();
        registry.remove_exact(id, &first_token).unwrap();
        drop(first_wrapper);
        storage.destroy(first);

        let replacement = dmo_collection(&mut storage, &[2]);
        let replacement_wrapper = registry.create_wrapper(id, &replacement).unwrap();
        let replacement_token = DmoRegistry::stable_token(&replacement_wrapper);
        registry.publish(id, &replacement_token).unwrap();
        assert_eq!(
            main_dmo_snapshot_failure(stale.try_snapshot_main_dmo_bank()).error(),
            SimulationMainDmoBankResidentSnapshotError::UnknownOccurrence
        );
        let replacement_snapshot = lookup
            .lookup(id)
            .unwrap()
            .unwrap()
            .try_snapshot_main_dmo_bank()
            .unwrap();
        assert_eq!(
            replacement_snapshot.assets[0].resident_asset_id(),
            CollectionId(2)
        );

        registry.remove_exact(id, &replacement_token).unwrap();
        drop((replacement_snapshot, replacement_wrapper));
        storage.destroy(replacement);
    }

    #[test]
    fn dead_empty_main_dmo_resident_refuses_snapshot_before_publication_escape() {
        let mut storage = DmoStorage::new(std::convert::identity);
        let collection = dmo_collection(&mut storage, &[]);
        let (mut registry, lookup) = dmo_registry();
        let id = CollectionId(DMO_BANK_COLLECTION_ID);
        let wrapper = registry.create_wrapper(id, &collection).unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        registry.publish(id, &token).unwrap();
        let occurrence = lookup.lookup(id).unwrap().unwrap();
        storage.destroy(collection);

        assert!(matches!(
            main_dmo_snapshot_failure(occurrence.try_snapshot_main_dmo_bank()).error(),
            SimulationMainDmoBankResidentSnapshotError::Resident(
                OwnedResidentCollectionError::NotAlive { .. }
            )
        ));
        registry.remove_exact(id, &token).unwrap();
        drop(wrapper);
    }

    #[test]
    fn poisoned_and_detached_lookup_fail_without_aliasing_a_collection() {
        let (registry, lookup) = registry();
        let poison_state = Arc::clone(&registry.state);
        std::thread::spawn(move || {
            let _guard = poison_state.lock().unwrap();
            panic!("poison registry");
        })
        .join()
        .unwrap_err();
        assert!(matches!(
            lookup.lookup(CollectionId(1)),
            Err(SimulationResidentLookupError::Poisoned)
        ));
        drop(registry);
        assert!(matches!(
            lookup.lookup(CollectionId(1)),
            Err(SimulationResidentLookupError::RegistryDetached)
        ));
    }
}
