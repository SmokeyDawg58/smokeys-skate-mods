//! Unregistered physical loaded-arena owner for the existing resident builder.
//!
//! `cAssetManager::AddCompressedArenaAsset` reuses one processed allocation for
//! a full arena ID across collections. This wrapper does that at the existing
//! `CollectionAssetBuilderSessionFactory` seam. It does not implement the
//! separate Presentation first/last-use counter, registry, or callbacks.

use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    sync::{Arc, Mutex, Weak},
};

use skate2_core::{
    catalog_resident_load_factory::CollectionAssetBuilderSessionFactory,
    collection_resident_streamer::CollectionId,
    owned_resident_collection::ResidentAssetIdentity,
    resident_collection_load_driver::{CollectionAssetBuildInput, CollectionAssetBuilder},
    world_stream_catalog::CollectionRegistrationSnapshot,
};

struct LoadedArenaEntry<A> {
    // Taken only during final teardown, while the map mutex stays locked.
    product: Option<Arc<A>>,
    physical_uses: u32,
}

/// One manager-domain loaded-asset map, shared by every relevant collection
/// builder session. The map owns the processed allocation while uses remain.
pub(crate) struct SharedLoadedArenaCache<A> {
    entries: Arc<Mutex<BTreeMap<u64, LoadedArenaEntry<A>>>>,
}

impl<A> Clone for SharedLoadedArenaCache<A> {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
        }
    }
}

impl<A> Default for SharedLoadedArenaCache<A> {
    fn default() -> Self {
        Self {
            entries: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SharedLoadedArenaCacheError {
    Poisoned,
}

impl fmt::Display for SharedLoadedArenaCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("shared loaded-arena map is poisoned")
    }
}

impl Error for SharedLoadedArenaCacheError {}

/// One physical collection membership. Deliberately not `Clone`: obtaining a
/// second membership must pass through the full-ID cache and increment uses.
pub(crate) struct SharedLoadedArenaAsset<A> {
    id: CollectionId,
    product: Option<Arc<A>>,
    entries: Arc<Mutex<BTreeMap<u64, LoadedArenaEntry<A>>>>,
}

impl<A> SharedLoadedArenaAsset<A> {
    pub(crate) fn product(&self) -> &A {
        self.product
            .as_deref()
            .expect("loaded arena occurrence is being destroyed")
    }

    /// Retain the exact processed allocation for first-use Presentation
    /// registration. The caller must hold the underlying collection resident
    /// until this clone is released after last-use deregistration; otherwise
    /// the physical loaded-map row could reach zero while an external product
    /// owner remains alive.
    pub(crate) fn retained_product(&self) -> Arc<A> {
        Arc::clone(
            self.product
                .as_ref()
                .expect("loaded arena occurrence is being destroyed"),
        )
    }

    pub(crate) fn weak_product(&self) -> Weak<A> {
        Arc::downgrade(
            self.product
                .as_ref()
                .expect("loaded arena occurrence is being destroyed"),
        )
    }

    pub(crate) fn same_product(&self, other: &Self) -> bool {
        Arc::ptr_eq(
            self.product
                .as_ref()
                .expect("loaded arena occurrence is being destroyed"),
            other
                .product
                .as_ref()
                .expect("loaded arena occurrence is being destroyed"),
        )
    }
}

impl<A> ResidentAssetIdentity for SharedLoadedArenaAsset<A> {
    fn resident_asset_id(&self) -> CollectionId {
        self.id
    }
}

impl<A> Drop for SharedLoadedArenaAsset<A> {
    fn drop(&mut self) {
        // Native DownLoadedCount unprocesses before erasing the loaded-map row
        // or releasing its mutex. Drop cannot return a checked error. Recover
        // poison only to release this already counted use; invariants assert.
        let occurrence_product = self.product.take().expect("occurrence product is absent");
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let entry = entries
            .get_mut(&self.id.0)
            .expect("loaded arena occurrence lost its map row");
        assert!(
            Arc::ptr_eq(
                entry.product.as_ref().expect("map product is absent"),
                &occurrence_product
            ),
            "loaded arena occurrence changed product identity"
        );
        assert!(entry.physical_uses > 0, "loaded arena use underflow");
        entry.physical_uses -= 1;
        if entry.physical_uses == 0 {
            let map_product = entry.product.take().expect("map product is absent");
            // Both strong references are explicitly released while the row
            // and mutex still exist. No concurrent same-ID builder can start
            // until product teardown completes and the row is erased.
            drop(occurrence_product);
            drop(map_product);
            entries.remove(&self.id.0);
        } else {
            drop(entries);
            drop(occurrence_product);
        }
    }
}

impl<A> SharedLoadedArenaCache<A> {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn physical_loaded_uses(
        &self,
        id: CollectionId,
    ) -> Result<Option<u32>, SharedLoadedArenaCacheError> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| SharedLoadedArenaCacheError::Poisoned)?;
        Ok(entries.get(&id.0).map(|entry| entry.physical_uses))
    }
}

#[derive(Debug)]
pub(crate) enum SharedLoadedArenaBuildError<E> {
    Poisoned,
    PhysicalUseOverflow(CollectionId),
    IdentityMismatch {
        requested: CollectionId,
        built: CollectionId,
    },
    Source(E),
}

impl<E: fmt::Display> fmt::Display for SharedLoadedArenaBuildError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Poisoned => formatter.write_str("shared loaded-arena map is poisoned"),
            Self::PhysicalUseOverflow(id) => {
                write!(formatter, "physical loaded-use overflow for {:#018x}", id.0)
            }
            Self::IdentityMismatch { requested, built } => write!(
                formatter,
                "arena builder returned {:#018x} for requested {:#018x}",
                built.0, requested.0
            ),
            Self::Source(error) => write!(formatter, "arena builder failed: {error}"),
        }
    }
}

impl<E: Error + 'static> Error for SharedLoadedArenaBuildError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            _ => None,
        }
    }
}

impl<A: ResidentAssetIdentity> SharedLoadedArenaCache<A> {
    /// Native loaded-map admission: an existing full ID reuses its processed
    /// pointer without running `construct`; a miss constructs once and inserts
    /// before the occurrence is handed to the resident collection writer.
    ///
    /// This serializes construction under the cache lock, matching the native
    /// add-asset mutex's single-winner property. `construct` must not reenter
    /// this same cache. Final product teardown also runs under this lock, so
    /// `A::drop` must not reenter the same cache. An error leaves prior
    /// IDs/occurrences untouched.
    fn acquire_with<E>(
        &self,
        id: CollectionId,
        construct: impl FnOnce() -> Result<Option<A>, E>,
    ) -> Result<Option<SharedLoadedArenaAsset<A>>, SharedLoadedArenaBuildError<E>> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| SharedLoadedArenaBuildError::Poisoned)?;
        if let Some(entry) = entries.get_mut(&id.0) {
            entry.physical_uses = entry
                .physical_uses
                .checked_add(1)
                .ok_or(SharedLoadedArenaBuildError::PhysicalUseOverflow(id))?;
            return Ok(Some(SharedLoadedArenaAsset {
                id,
                product: Some(Arc::clone(
                    entry.product.as_ref().expect("live map product is absent"),
                )),
                entries: Arc::clone(&self.entries),
            }));
        }
        let Some(product) = construct().map_err(SharedLoadedArenaBuildError::Source)? else {
            return Ok(None);
        };
        let built = product.resident_asset_id();
        if built != id {
            return Err(SharedLoadedArenaBuildError::IdentityMismatch {
                requested: id,
                built,
            });
        }
        let product = Arc::new(product);
        entries.insert(
            id.0,
            LoadedArenaEntry {
                product: Some(Arc::clone(&product)),
                physical_uses: 1,
            },
        );
        Ok(Some(SharedLoadedArenaAsset {
            id,
            product: Some(product),
            entries: Arc::clone(&self.entries),
        }))
    }
}

/// Wraps the existing per-collection builder sessions; the accepted catalog,
/// mount driver, decoded parser, and GPU uploader remain in `source`.
pub(crate) struct SharedLoadedArenaSessions<F, A> {
    source: F,
    cache: SharedLoadedArenaCache<A>,
}

impl<F, A> SharedLoadedArenaSessions<F, A> {
    pub(crate) fn new(source: F, cache: SharedLoadedArenaCache<A>) -> Self {
        Self { source, cache }
    }
}

pub(crate) struct SharedLoadedArenaBuilder<B, A> {
    source: B,
    cache: SharedLoadedArenaCache<A>,
}

impl<F, A> CollectionAssetBuilderSessionFactory for SharedLoadedArenaSessions<F, A>
where
    F: CollectionAssetBuilderSessionFactory<Asset = A>,
    A: ResidentAssetIdentity,
{
    type Asset = SharedLoadedArenaAsset<A>;
    type Builder = SharedLoadedArenaBuilder<F::Builder, A>;
    type Error = F::Error;

    fn begin_session(
        &mut self,
        collection: &CollectionRegistrationSnapshot,
    ) -> Result<Self::Builder, Self::Error> {
        Ok(SharedLoadedArenaBuilder {
            source: self.source.begin_session(collection)?,
            cache: self.cache.clone(),
        })
    }
}

impl<I, B, A> CollectionAssetBuilder<I> for SharedLoadedArenaBuilder<B, A>
where
    B: CollectionAssetBuilder<I, Asset = A>,
    A: ResidentAssetIdentity,
{
    type Asset = SharedLoadedArenaAsset<A>;
    type Error = SharedLoadedArenaBuildError<B::Error>;

    fn build(
        &mut self,
        input: CollectionAssetBuildInput<'_, I>,
    ) -> Result<Option<Self::Asset>, Self::Error> {
        let id = CollectionId(input.asset_id);
        self.cache.acquire_with(id, || self.source.build(input))
    }
}

#[cfg(test)]
#[path = "proxy_shared_loaded_arena_draft_20260923_tests.rs"]
mod tests;
