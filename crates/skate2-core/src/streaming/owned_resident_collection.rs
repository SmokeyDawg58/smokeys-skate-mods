//! Safe interim ownership for one decoded resident collection.
//!
//! The reference `cCollection` stores an ordered vector of non-owning
//! `assman::cAsset*` values. Until the native registration/asset-manager
//! boundary exists, this module can instead retain owned decoded values while
//! preserving full IDs, duplicates, insertion order, and a non-owning load
//! writer. That owned-value substitution is a high-confidence host bridge,
//! not native asset ownership parity.

use std::{
    fmt,
    marker::PhantomData,
    sync::{Arc, Mutex, MutexGuard, Weak},
};

use crate::{
    collection_load_pump::LoadedAssetCollection,
    collection_resident_streamer::{CollectionId, ResidentCollectionStorage},
    decoded_arena_asset::DecodedArenaAsset,
};

/// Supplies the full identity stored in the resident collection's ordered
/// membership list.
pub trait ResidentAssetIdentity {
    fn resident_asset_id(&self) -> CollectionId;
}

impl ResidentAssetIdentity for DecodedArenaAsset {
    fn resident_asset_id(&self) -> CollectionId {
        CollectionId(self.asset_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnedResidentCollectionOperation {
    Create,
    Writer,
    Snapshot,
}

/// Typed failures for the safe interim collection representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnedResidentCollectionError {
    NegativeReserve {
        raw: i32,
    },
    ReserveFailed {
        requested: usize,
    },
    SnapshotReserveFailed {
        requested: usize,
    },
    Poisoned {
        operation: OwnedResidentCollectionOperation,
    },
    NotAlive {
        operation: OwnedResidentCollectionOperation,
    },
}

impl fmt::Display for OwnedResidentCollectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeReserve { raw } => {
                write!(formatter, "negative resident asset reserve {raw}")
            }
            Self::ReserveFailed { requested } => {
                write!(formatter, "could not reserve {requested} resident assets")
            }
            Self::SnapshotReserveFailed { requested } => {
                write!(
                    formatter,
                    "could not reserve {requested} resident asset IDs"
                )
            }
            Self::Poisoned { operation } => {
                write!(
                    formatter,
                    "resident collection mutex poisoned during {operation:?}"
                )
            }
            Self::NotAlive { operation } => {
                write!(
                    formatter,
                    "resident collection is not alive during {operation:?}"
                )
            }
        }
    }
}

impl std::error::Error for OwnedResidentCollectionError {}

struct ResidentAssetEntry<A> {
    id: CollectionId,
    value: Arc<A>,
}

/// Non-owning identity for one exact resident asset allocation.
///
/// This is crate-private because it exists solely to reproduce native
/// registry pointer identity without exposing an asset owner to callers.
pub(crate) struct OwnedResidentWeakAsset<A> {
    id: CollectionId,
    value: Weak<A>,
}

impl<A> OwnedResidentWeakAsset<A> {
    #[must_use]
    pub(crate) const fn id(&self) -> CollectionId {
        self.id
    }

    #[must_use]
    pub(crate) fn weak(&self) -> &Weak<A> {
        &self.value
    }
}

impl<A> Clone for OwnedResidentWeakAsset<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            value: Weak::clone(&self.value),
        }
    }
}

struct OwnedResidentCollectionState<A> {
    alive: bool,
    assets: Vec<ResidentAssetEntry<A>>,
}

struct OwnedResidentCollectionInner<A> {
    state: Mutex<OwnedResidentCollectionState<A>>,
}

/// Sole persistent strong owner for one resident collection.
///
/// This type deliberately does not implement `Clone`. The resident table must
/// remain the sole persistent owner; load writers retain only a [`Weak`].
pub struct OwnedResidentCollection<A> {
    inner: Arc<OwnedResidentCollectionInner<A>>,
}

/// Opaque non-owning collection identity for a registry wrapper.
///
/// The view exposes no asset access, append capability, or strong owner. It is
/// intentionally not cloneable until a concrete registry demonstrates that
/// more than one non-owning alias is required.
pub struct OwnedResidentCollectionView<A> {
    inner: Weak<OwnedResidentCollectionInner<A>>,
}

/// Scoped non-owning access to a resident collection's decoded assets.
///
/// Storing the view never keeps the collection alive. Each visit temporarily
/// upgrades its weak collection reference and snapshots internal asset
/// references, preventing concurrent destruction from invalidating the visit
/// without executing arbitrary visitor code under the collection mutex.
/// Borrowed assets cannot escape the visitor in safe Rust.
///
/// ```compile_fail
/// use skate2_core::owned_resident_collection::OwnedResidentAssetsView;
/// use std::convert::Infallible;
///
/// fn escape<A>(view: &OwnedResidentAssetsView<A>) -> &A {
///     let mut escaped = None;
///     view.try_for_each::<Infallible>(|asset| {
///         escaped = Some(asset);
///         Ok(())
///     }).unwrap();
///     escaped.unwrap()
/// }
/// ```
pub struct OwnedResidentAssetsView<A> {
    inner: Weak<OwnedResidentCollectionInner<A>>,
}

/// Typed failures from scoped resident-asset visitation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnedResidentAssetsVisitError<E> {
    Detached,
    NotAlive,
    Poisoned,
    SnapshotReserveFailed { requested: usize },
    Visitor(E),
}

impl<E: fmt::Display> fmt::Display for OwnedResidentAssetsVisitError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Detached => formatter.write_str("resident asset view is detached"),
            Self::NotAlive => formatter.write_str("resident collection is not alive"),
            Self::Poisoned => formatter.write_str("resident collection mutex is poisoned"),
            Self::SnapshotReserveFailed { requested } => {
                write!(formatter, "could not snapshot {requested} resident assets")
            }
            Self::Visitor(error) => write!(formatter, "resident asset visitor failed: {error}"),
        }
    }
}

impl<E> std::error::Error for OwnedResidentAssetsVisitError<E>
where
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Visitor(source) => Some(source),
            _ => None,
        }
    }
}

impl<A> OwnedResidentCollection<A> {
    /// Creates the non-owning identity retained by a resident wrapper.
    #[must_use]
    pub fn weak_view(&self) -> OwnedResidentCollectionView<A> {
        OwnedResidentCollectionView {
            inner: Arc::downgrade(&self.inner),
        }
    }

    /// Creates scoped non-owning access for subsystem registration adapters.
    #[must_use]
    pub fn resident_assets_view(&self) -> OwnedResidentAssetsView<A> {
        OwnedResidentAssetsView {
            inner: Arc::downgrade(&self.inner),
        }
    }

    /// Snapshots non-owning identities for native-style global publication.
    ///
    /// The returned weak handles preserve authored order and duplicates while
    /// adding no owner of either the collection or its assets.
    pub(crate) fn weak_assets_in_order(
        &self,
    ) -> Result<Vec<OwnedResidentWeakAsset<A>>, OwnedResidentCollectionError> {
        let state =
            self.inner
                .state
                .lock()
                .map_err(|_| OwnedResidentCollectionError::Poisoned {
                    operation: OwnedResidentCollectionOperation::Snapshot,
                })?;
        if !state.alive {
            return Err(OwnedResidentCollectionError::NotAlive {
                operation: OwnedResidentCollectionOperation::Snapshot,
            });
        }
        let requested = state.assets.len();
        let mut assets = Vec::new();
        assets
            .try_reserve_exact(requested)
            .map_err(|_| OwnedResidentCollectionError::SnapshotReserveFailed { requested })?;
        assets.extend(state.assets.iter().map(|entry| OwnedResidentWeakAsset {
            id: entry.id,
            value: Arc::downgrade(&entry.value),
        }));
        Ok(assets)
    }
}

impl<A> OwnedResidentCollectionView<A> {
    /// Reports whether the referenced collection allocation still exists.
    /// A temporary upgrade does not escape this call.
    #[must_use]
    pub fn is_attached(&self) -> bool {
        self.inner.upgrade().is_some()
    }

    /// Reports whether the exact allocation remains alive, treating a
    /// poisoned resident-state lock as unavailable.
    #[must_use]
    pub(crate) fn is_alive(&self) -> bool {
        let Some(inner) = self.inner.upgrade() else {
            return false;
        };
        inner.state.lock().is_ok_and(|state| state.alive)
    }

    /// Requires the exact collection allocation to remain alive while holding
    /// its state lock. This grants no asset access or ownership.
    pub(crate) fn try_require_alive(&self) -> Result<(), OwnedResidentCollectionError> {
        let inner = self
            .inner
            .upgrade()
            .ok_or(OwnedResidentCollectionError::NotAlive {
                operation: OwnedResidentCollectionOperation::Snapshot,
            })?;
        let state = inner
            .state
            .lock()
            .map_err(|_| OwnedResidentCollectionError::Poisoned {
                operation: OwnedResidentCollectionOperation::Snapshot,
            })?;
        if !state.alive {
            return Err(OwnedResidentCollectionError::NotAlive {
                operation: OwnedResidentCollectionOperation::Snapshot,
            });
        }
        Ok(())
    }

    /// Clones one exact resident row while holding the collection state lock.
    ///
    /// The allocation-ID check prevents a same-ID replacement from satisfying
    /// an older weak route. A successful clone linearizes before destruction;
    /// a concurrent destroy cannot remove the row until this lock is released.
    pub(crate) fn try_upgrade_exact_asset(
        &self,
        id: CollectionId,
        expected: &Weak<A>,
    ) -> Result<Option<Arc<A>>, OwnedResidentCollectionError> {
        let inner = self
            .inner
            .upgrade()
            .ok_or(OwnedResidentCollectionError::NotAlive {
                operation: OwnedResidentCollectionOperation::Snapshot,
            })?;
        let state = inner
            .state
            .lock()
            .map_err(|_| OwnedResidentCollectionError::Poisoned {
                operation: OwnedResidentCollectionOperation::Snapshot,
            })?;
        if !state.alive {
            return Err(OwnedResidentCollectionError::NotAlive {
                operation: OwnedResidentCollectionOperation::Snapshot,
            });
        }
        Ok(state
            .assets
            .iter()
            .find(|entry| entry.id == id && Weak::ptr_eq(expected, &Arc::downgrade(&entry.value)))
            .map(|entry| Arc::clone(&entry.value)))
    }

    /// Atomically clones one exact wrapper-time asset sequence while the
    /// collection remains alive.
    ///
    /// Count, authored order, full ID, and outer allocation identity must all
    /// match. `Ok(None)` is an identity mismatch; no partial owner escapes.
    #[allow(
        dead_code,
        reason = "staged exact main-DMO occurrence acquisition is not yet called by a production coordinator"
    )]
    pub(crate) fn try_upgrade_exact_assets_in_order(
        &self,
        expected: &[OwnedResidentWeakAsset<A>],
    ) -> Result<Option<Vec<Arc<A>>>, OwnedResidentCollectionError> {
        let inner = self
            .inner
            .upgrade()
            .ok_or(OwnedResidentCollectionError::NotAlive {
                operation: OwnedResidentCollectionOperation::Snapshot,
            })?;
        let state = inner
            .state
            .lock()
            .map_err(|_| OwnedResidentCollectionError::Poisoned {
                operation: OwnedResidentCollectionOperation::Snapshot,
            })?;
        if !state.alive {
            return Err(OwnedResidentCollectionError::NotAlive {
                operation: OwnedResidentCollectionOperation::Snapshot,
            });
        }
        if state.assets.len() != expected.len()
            || state.assets.iter().zip(expected).any(|(actual, expected)| {
                actual.id != expected.id
                    || !Weak::ptr_eq(expected.weak(), &Arc::downgrade(&actual.value))
            })
        {
            return Ok(None);
        }
        let requested = state.assets.len();
        let mut assets = Vec::new();
        assets
            .try_reserve_exact(requested)
            .map_err(|_| OwnedResidentCollectionError::SnapshotReserveFailed { requested })?;
        assets.extend(state.assets.iter().map(|entry| Arc::clone(&entry.value)));
        Ok(Some(assets))
    }

    /// Compares allocation identity without upgrading either weak reference.
    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.inner, &other.inner)
    }
}

#[cfg(test)]
impl<A> OwnedResidentCollection<A> {
    pub(crate) fn tombstone_without_releasing_assets_for_test(&self) {
        let mut state = self.inner.state.lock().unwrap();
        state.alive = false;
    }
}

impl<A> OwnedResidentAssetsView<A> {
    /// Visits each decoded asset in resident insertion order.
    ///
    /// The ordered asset snapshot is taken atomically with respect to append
    /// and destruction, then the mutex is released before the visitor runs.
    /// Reentrant collection operations therefore cannot self-deadlock. A visit
    /// that snapshots before destruction may finish afterward, with its
    /// temporary internal references keeping those values valid.
    ///
    /// Visitor effects are not transactional. If a later asset is rejected,
    /// effects already performed for earlier assets remain.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the weak view is detached, the collection
    /// has begun destruction, its mutex is poisoned, snapshot allocation
    /// fails, or the visitor rejects an asset.
    pub fn try_for_each<E>(
        &self,
        mut visitor: impl FnMut(&A) -> Result<(), E>,
    ) -> Result<(), OwnedResidentAssetsVisitError<E>> {
        let inner = self
            .inner
            .upgrade()
            .ok_or(OwnedResidentAssetsVisitError::Detached)?;
        let state = inner
            .state
            .lock()
            .map_err(|_| OwnedResidentAssetsVisitError::Poisoned)?;
        if !state.alive {
            return Err(OwnedResidentAssetsVisitError::NotAlive);
        }
        let requested = state.assets.len();
        let mut assets = Vec::new();
        assets
            .try_reserve_exact(requested)
            .map_err(|_| OwnedResidentAssetsVisitError::SnapshotReserveFailed { requested })?;
        assets.extend(state.assets.iter().map(|entry| Arc::clone(&entry.value)));
        drop(state);
        drop(inner);

        for asset in &assets {
            visitor(asset).map_err(OwnedResidentAssetsVisitError::Visitor)?;
        }
        Ok(())
    }
}

/// Non-owning append capability moved through the collection load driver.
///
/// This type deliberately does not implement `Clone`. A temporary successful
/// `Weak::upgrade` may overlap destruction. A racing append either linearizes
/// before the destruction tombstone and is included in destruction, or
/// observes the tombstone and cannot commit afterward.
pub struct OwnedResidentCollectionWriter<A> {
    inner: Weak<OwnedResidentCollectionInner<A>>,
}

impl<A: ResidentAssetIdentity> LoadedAssetCollection<A> for OwnedResidentCollectionWriter<A> {
    fn add_asset(&mut self, asset: A) {
        let id = asset.resident_asset_id();
        let inner = self
            .inner
            .upgrade()
            .unwrap_or_else(|| panic!("resident collection writer outlived its collection"));
        let mut state = inner
            .state
            .lock()
            .unwrap_or_else(|_| panic!("resident collection mutex poisoned during asset append"));
        if !state.alive {
            drop(state);
            panic!("resident collection writer appended after destruction began");
        }
        state.assets.push(ResidentAssetEntry {
            id,
            value: Arc::new(asset),
        });
    }
}

/// Stateless storage adapter with an injected error conversion.
///
/// Every resident collaborator currently shares one outer error type. The
/// mapper keeps storage failures typed without coupling this generic container
/// to a particular composition's unified error enum.
pub struct OwnedResidentCollectionStorage<A, M, E> {
    map_error: M,
    type_marker: PhantomData<fn(A, OwnedResidentCollectionError) -> E>,
}

impl<A, M, E> OwnedResidentCollectionStorage<A, M, E> {
    #[must_use]
    pub const fn new(map_error: M) -> Self {
        Self {
            map_error,
            type_marker: PhantomData,
        }
    }

    fn map_error(&mut self, error: OwnedResidentCollectionError) -> E
    where
        M: FnMut(OwnedResidentCollectionError) -> E,
    {
        (self.map_error)(error)
    }
}

impl<A, M, E> ResidentCollectionStorage<E> for OwnedResidentCollectionStorage<A, M, E>
where
    A: ResidentAssetIdentity,
    M: FnMut(OwnedResidentCollectionError) -> E,
{
    type Collection = OwnedResidentCollection<A>;
    type Writer = OwnedResidentCollectionWriter<A>;

    fn create_empty(&mut self, reserve: i32) -> Result<Self::Collection, E> {
        let requested = usize::try_from(reserve).map_err(|_| {
            self.map_error(OwnedResidentCollectionError::NegativeReserve { raw: reserve })
        })?;
        let mut assets = Vec::new();
        assets.try_reserve_exact(requested).map_err(|_| {
            self.map_error(OwnedResidentCollectionError::ReserveFailed { requested })
        })?;
        Ok(OwnedResidentCollection {
            inner: Arc::new(OwnedResidentCollectionInner {
                state: Mutex::new(OwnedResidentCollectionState {
                    alive: true,
                    assets,
                }),
            }),
        })
    }

    fn writer(&mut self, collection: &Self::Collection) -> Result<Self::Writer, E> {
        let state = self.lock_state(collection, OwnedResidentCollectionOperation::Writer)?;
        if !state.alive {
            return Err(self.map_error(OwnedResidentCollectionError::NotAlive {
                operation: OwnedResidentCollectionOperation::Writer,
            }));
        }
        drop(state);
        Ok(OwnedResidentCollectionWriter {
            inner: Arc::downgrade(&collection.inner),
        })
    }

    fn asset_ids_in_order(
        &mut self,
        collection: &Self::Collection,
    ) -> Result<Vec<CollectionId>, E> {
        let state = self.lock_state(collection, OwnedResidentCollectionOperation::Snapshot)?;
        if !state.alive {
            drop(state);
            return Err(self.map_error(OwnedResidentCollectionError::NotAlive {
                operation: OwnedResidentCollectionOperation::Snapshot,
            }));
        }
        let requested = state.assets.len();
        let mut ids = Vec::new();
        if ids.try_reserve_exact(requested).is_err() {
            drop(state);
            return Err(
                self.map_error(OwnedResidentCollectionError::SnapshotReserveFailed { requested })
            );
        }
        ids.extend(state.assets.iter().map(|entry| entry.id));
        Ok(ids)
    }

    fn destroy(&mut self, collection: Self::Collection) {
        let mut state = collection
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(state.alive, "resident collection destroyed more than once");
        state.alive = false;
        let assets = std::mem::take(&mut state.assets);
        drop(state);
        drop(assets);
        drop(collection);
    }
}

impl<A, M, E> OwnedResidentCollectionStorage<A, M, E> {
    fn lock_state<'a>(
        &mut self,
        collection: &'a OwnedResidentCollection<A>,
        operation: OwnedResidentCollectionOperation,
    ) -> Result<MutexGuard<'a, OwnedResidentCollectionState<A>>, E>
    where
        M: FnMut(OwnedResidentCollectionError) -> E,
    {
        collection
            .inner
            .state
            .lock()
            .map_err(|_| self.map_error(OwnedResidentCollectionError::Poisoned { operation }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug)]
    struct TestAsset {
        id: CollectionId,
        drops: Arc<AtomicUsize>,
    }

    impl ResidentAssetIdentity for TestAsset {
        fn resident_asset_id(&self) -> CollectionId {
            self.id
        }
    }

    impl Drop for TestAsset {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    type TestStorage = OwnedResidentCollectionStorage<
        TestAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;

    fn storage() -> TestStorage {
        OwnedResidentCollectionStorage::new(std::convert::identity)
    }

    fn asset(id: u64, drops: &Arc<AtomicUsize>) -> TestAsset {
        TestAsset {
            id: CollectionId(id),
            drops: Arc::clone(drops),
        }
    }

    #[test]
    fn negative_reserve_is_rejected_without_unsigned_conversion() {
        let mut storage = storage();
        let error = storage.create_empty(-1).err().unwrap();
        assert_eq!(
            error,
            OwnedResidentCollectionError::NegativeReserve { raw: -1 }
        );
    }

    #[test]
    fn writer_is_weak_and_preserves_full_ids_duplicates_and_order() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(3).unwrap();
        assert_eq!(Arc::strong_count(&collection.inner), 1);
        let mut writer = storage.writer(&collection).unwrap();
        assert_eq!(Arc::strong_count(&collection.inner), 1);

        writer.add_asset(asset(0x1111_2222_5566_7788, &drops));
        writer.add_asset(asset(0x9999_aaaa_5566_7788, &drops));
        writer.add_asset(asset(0x1111_2222_5566_7788, &drops));

        assert_eq!(
            storage.asset_ids_in_order(&collection).unwrap(),
            [
                CollectionId(0x1111_2222_5566_7788),
                CollectionId(0x9999_aaaa_5566_7788),
                CollectionId(0x1111_2222_5566_7788),
            ]
        );
        storage.destroy(collection);
        assert_eq!(drops.load(Ordering::SeqCst), 3);
        assert!(writer.inner.upgrade().is_none());
    }

    #[test]
    fn registry_view_is_nonowning_and_expires_with_the_collection() {
        let mut storage = storage();
        let collection: OwnedResidentCollection<TestAsset> = storage.create_empty(0).unwrap();
        let view = collection.weak_view();
        let second_view = collection.weak_view();
        assert!(view.is_attached());
        assert!(view.ptr_eq(&second_view));
        assert_eq!(Arc::strong_count(&collection.inner), 1);

        storage.destroy(collection);

        assert!(!view.is_attached());
        assert!(!second_view.is_attached());
    }

    #[test]
    fn asset_view_is_nonowning_and_visits_duplicates_in_insertion_order() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(3).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(4, &drops));
        writer.add_asset(asset(9, &drops));
        writer.add_asset(asset(4, &drops));

        let view = collection.resident_assets_view();
        let second_view = collection.resident_assets_view();
        assert_eq!(Arc::strong_count(&collection.inner), 1);

        let mut visited = Vec::new();
        view.try_for_each::<std::convert::Infallible>(|asset| {
            visited.push(asset.id);
            Ok(())
        })
        .unwrap();
        assert_eq!(visited, [CollectionId(4), CollectionId(9), CollectionId(4)]);
        assert_eq!(Arc::strong_count(&collection.inner), 1);

        storage.destroy(collection);
        assert_eq!(drops.load(Ordering::SeqCst), 3);
        assert_eq!(
            second_view.try_for_each::<std::convert::Infallible>(|_| Ok(())),
            Err(OwnedResidentAssetsVisitError::Detached)
        );
    }

    #[test]
    fn asset_view_stops_at_and_preserves_the_visitor_error() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(3).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(1, &drops));
        writer.add_asset(asset(2, &drops));
        writer.add_asset(asset(3, &drops));
        let view = collection.resident_assets_view();
        let mut visited = Vec::new();

        let result = view.try_for_each(|asset| {
            visited.push(asset.id);
            if asset.id == CollectionId(2) {
                Err("registration rejected asset")
            } else {
                Ok(())
            }
        });

        assert_eq!(
            result,
            Err(OwnedResidentAssetsVisitError::Visitor(
                "registration rejected asset"
            ))
        );
        assert_eq!(visited, [CollectionId(1), CollectionId(2)]);
        storage.destroy(collection);
    }

    #[test]
    fn asset_visitor_can_reenter_without_locking_the_collection() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(2).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(1, &drops));
        let view = collection.resident_assets_view();
        let nested_view = collection.resident_assets_view();
        let mut outer = Vec::new();
        let mut nested = Vec::new();

        view.try_for_each::<std::convert::Infallible>(|resident| {
            outer.push(resident.id);
            writer.add_asset(asset(2, &drops));
            nested_view
                .try_for_each::<std::convert::Infallible>(|resident| {
                    nested.push(resident.id);
                    Ok(())
                })
                .unwrap();
            Ok(())
        })
        .unwrap();

        assert_eq!(outer, [CollectionId(1)]);
        assert_eq!(nested, [CollectionId(1), CollectionId(2)]);
        assert_eq!(
            storage.asset_ids_in_order(&collection).unwrap(),
            [CollectionId(1), CollectionId(2)]
        );
        storage.destroy(collection);
        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn asset_view_observes_a_live_tombstone_when_allocation_is_retained() {
        let mut storage = storage();
        let collection: OwnedResidentCollection<TestAsset> = storage.create_empty(0).unwrap();
        let view = collection.resident_assets_view();
        let transient_inner = view.inner.upgrade().unwrap();

        storage.destroy(collection);

        assert_eq!(
            view.try_for_each::<std::convert::Infallible>(|_| Ok(())),
            Err(OwnedResidentAssetsVisitError::NotAlive)
        );
        drop(transient_inner);
        assert_eq!(
            view.try_for_each::<std::convert::Infallible>(|_| Ok(())),
            Err(OwnedResidentAssetsVisitError::Detached)
        );
    }

    #[test]
    fn poisoned_asset_view_is_typed_and_destroy_still_reclaims_values() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(1).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(3, &drops));
        let view = collection.resident_assets_view();
        let inner = Arc::clone(&collection.inner);
        std::thread::spawn(move || {
            let _guard = inner.state.lock().unwrap();
            panic!("poison asset-view mutex");
        })
        .join()
        .unwrap_err();

        assert_eq!(
            view.try_for_each::<std::convert::Infallible>(|_| Ok(())),
            Err(OwnedResidentAssetsVisitError::Poisoned)
        );
        storage.destroy(collection);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn panicking_visitor_does_not_poison_collection_or_leak_snapshot_assets() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(1).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(5, &drops));
        let view = collection.resident_assets_view();

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = view.try_for_each::<std::convert::Infallible>(|_| {
                panic!("visitor panic");
            });
        }));

        assert!(result.is_err());
        assert_eq!(
            storage.asset_ids_in_order(&collection).unwrap(),
            [CollectionId(5)]
        );
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        storage.destroy(collection);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn active_visit_keeps_only_snapshot_assets_valid_through_destruction() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(1).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(7, &drops));
        let view = collection.resident_assets_view();
        let (entered_sender, entered_receiver) = std::sync::mpsc::channel();
        let (continue_sender, continue_receiver) = std::sync::mpsc::channel();

        let visit = std::thread::spawn(move || {
            view.try_for_each::<std::convert::Infallible>(|resident| {
                assert_eq!(resident.id, CollectionId(7));
                entered_sender.send(()).unwrap();
                continue_receiver.recv().unwrap();
                assert_eq!(resident.id, CollectionId(7));
                Ok(())
            })
            .unwrap();
        });
        entered_receiver.recv().unwrap();

        storage.destroy(collection);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        continue_sender.send(()).unwrap();
        visit.join().unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn visitor_error_is_exposed_as_the_error_source() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(1).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(6, &drops));
        let view = collection.resident_assets_view();

        let error = view
            .try_for_each(|_| Err(std::io::Error::other("visitor source")))
            .unwrap_err();

        let source = std::error::Error::source(&error).unwrap();
        assert_eq!(source.to_string(), "visitor source");
        storage.destroy(collection);
    }

    #[test]
    fn stale_writer_panics_instead_of_silently_discarding_an_asset() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(1).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        storage.destroy(collection);

        let result = catch_unwind(AssertUnwindSafe(|| {
            writer.add_asset(asset(7, &drops));
        }));

        assert!(result.is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn upgraded_writer_cannot_commit_after_the_destroy_tombstone() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(1).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        let transient_inner = writer.inner.upgrade().unwrap();
        storage.destroy(collection);

        let result = catch_unwind(AssertUnwindSafe(|| {
            writer.add_asset(asset(8, &drops));
        }));

        assert!(result.is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(!transient_inner.state.lock().unwrap().alive);
    }

    #[test]
    fn destroy_drops_owned_values_once_and_does_not_need_the_writer() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(2).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(1, &drops));
        writer.add_asset(asset(2, &drops));
        drop(writer);

        storage.destroy(collection);

        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn zero_reserve_accepts_null_builder_completion() {
        let mut storage = storage();
        let collection: OwnedResidentCollection<TestAsset> = storage.create_empty(0).unwrap();
        let _writer = storage.writer(&collection).unwrap();
        assert!(storage.asset_ids_in_order(&collection).unwrap().is_empty());
        storage.destroy(collection);
    }

    #[test]
    fn append_and_snapshot_linearize_under_the_same_mutex() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(2).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(10, &drops));
        assert_eq!(
            storage.asset_ids_in_order(&collection).unwrap(),
            [CollectionId(10)]
        );
        writer.add_asset(asset(11, &drops));
        assert_eq!(
            storage.asset_ids_in_order(&collection).unwrap(),
            [CollectionId(10), CollectionId(11)]
        );
        storage.destroy(collection);
    }

    #[test]
    fn exact_asset_sequence_requires_order_and_outer_allocation_identity() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let first = storage.create_empty(2).unwrap();
        let mut first_writer = storage.writer(&first).unwrap();
        first_writer.add_asset(asset(0x1111_2222_0000_0007, &drops));
        first_writer.add_asset(asset(0x3333_4444_0000_0009, &drops));
        let expected = first.weak_assets_in_order().unwrap();

        let equal_ids = storage.create_empty(2).unwrap();
        let mut equal_writer = storage.writer(&equal_ids).unwrap();
        equal_writer.add_asset(asset(0x1111_2222_0000_0007, &drops));
        equal_writer.add_asset(asset(0x3333_4444_0000_0009, &drops));
        let foreign = equal_ids.weak_assets_in_order().unwrap();
        assert!(
            equal_ids
                .weak_view()
                .try_upgrade_exact_assets_in_order(&expected)
                .unwrap()
                .is_none()
        );

        let wrong_full_id = [
            expected[0].clone(),
            OwnedResidentWeakAsset {
                id: CollectionId(0x5555_6666_0000_0009),
                value: Weak::clone(expected[1].weak()),
            },
        ];
        assert!(
            first
                .weak_view()
                .try_upgrade_exact_assets_in_order(&wrong_full_id)
                .unwrap()
                .is_none()
        );

        let one_foreign_owner = [expected[0].clone(), foreign[1].clone()];
        assert!(
            first
                .weak_view()
                .try_upgrade_exact_assets_in_order(&one_foreign_owner)
                .unwrap()
                .is_none()
        );

        let reversed = [expected[1].clone(), expected[0].clone()];
        assert!(
            first
                .weak_view()
                .try_upgrade_exact_assets_in_order(&reversed)
                .unwrap()
                .is_none()
        );

        let retained = first
            .weak_view()
            .try_upgrade_exact_assets_in_order(&expected)
            .unwrap()
            .unwrap();
        assert_eq!(
            retained.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            [
                CollectionId(0x1111_2222_0000_0007),
                CollectionId(0x3333_4444_0000_0009)
            ]
        );
        assert!(
            retained.iter().zip(&expected).all(|(actual, expected)| {
                Weak::ptr_eq(&Arc::downgrade(actual), expected.weak())
            })
        );

        storage.destroy(first);
        storage.destroy(equal_ids);
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        drop(retained);
        assert_eq!(drops.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn poisoned_snapshot_is_typed_and_destroy_still_reclaims_values() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(1).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(12, &drops));
        let inner = Arc::clone(&collection.inner);
        std::thread::spawn(move || {
            let _guard = inner.state.lock().unwrap();
            panic!("poison test mutex");
        })
        .join()
        .unwrap_err();

        let error = storage.asset_ids_in_order(&collection).unwrap_err();
        assert_eq!(
            error,
            OwnedResidentCollectionError::Poisoned {
                operation: OwnedResidentCollectionOperation::Snapshot,
            }
        );
        storage.destroy(collection);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn decoded_arena_asset_implements_resident_identity() {
        fn require_identity<T: ResidentAssetIdentity>() {}
        require_identity::<DecodedArenaAsset>();
    }

    #[test]
    fn retained_entry_owns_the_value_not_only_its_cached_id() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut storage = storage();
        let collection = storage.create_empty(1).unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        writer.add_asset(asset(5, &drops));
        {
            let state = collection.inner.state.lock().unwrap();
            assert_eq!(state.assets.len(), 1);
            assert_eq!(state.assets[0].id, CollectionId(5));
            assert_eq!(drops.load(Ordering::SeqCst), 0);
        }
        storage.destroy(collection);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
}
