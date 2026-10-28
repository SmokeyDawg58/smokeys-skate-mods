//! Concrete collaborator for processed world-stream publication.
//!
//! The exact valid-domain ordering is split across `cStreamFile::Update`
//! (`0x824e0ba8`, local construction/insertion and later ascending removal),
//! `cAssetStreamSystem::UpdateStreamFileLoaded` (`0x824e1ad0`, first-wins
//! global ID publication followed by file publication), and
//! `cAssetStreamSystem::RemoveStreamFile` (`0x824dc098`, global removal after
//! file raw state six). This module composes the already recovered catalog,
//! local Octree, and global publication store; it does not invent asset-system
//! effects or a new container implementation.
//!
//! Opaque builder/owner/token types are publicly nameable only because Rust's
//! public trait implementation exposes associated types. Their fields and
//! constructors remain private and they are deliberately not `Clone`.
//! Checked errors, one-lock host synchronization, and permanent failure
//! latching are high-confidence safe PC policy rather than console ABI claims.

use crate::{
    game_stream::{RegistrationFileId, StreamKind, StreamRegistrationContext},
    octree::{
        AabbWords, CapacityError, LocalOctree, OctreeCapacity, OctreeInsertError,
        OctreeRemovalError, OctreeResourcePlan, ResourcePlanOverflow,
    },
    world_stream_adapter::{
        LoadedStreamPublisher, LocalSpatialBoundsWords, LocalSpatialIndexBounds, NativeCPath,
        ProcessedOctreeStorage, vmx_max_ordinary, vmx_min_ordinary,
    },
    world_stream_catalog::{CatalogError, WorldStreamCatalog},
    world_stream_store::{
        PublicationToken, PublishStoreError, PublishedFileSnapshot, PublishedRecordSnapshot,
        PublishedStreamStore, StreamPublicationRequest, UnpublishError,
    },
};
use std::{
    fmt,
    sync::{Arc, Mutex},
};

type RuntimeStore =
    PublishedStreamStore<RegistrationFileId, ProcessedOctreeStorage, OwnedLocalOctree>;
type StoreToken = PublicationToken<RegistrationFileId, ProcessedOctreeStorage, OwnedLocalOctree>;

/// Actual linked-setting failure; no cvar-owner internals are exposed.
pub use crate::input_cvar_runtime::InputCvarError as TextureStreamGateError;

/// Live source for the outer texture-stream decision.
///
/// The reference samples the cVar at the simulation-XML transition rather
/// than caching it at publisher construction.
pub trait TextureStreamGate {
    /// # Errors
    /// Returns the actual live-target lookup failure; no default is substituted.
    fn texture_stream_enabled(&mut self) -> Result<bool, TextureStreamGateError>;
}

/// Explicit seams for the two observable effects owned by this collaborator.
/// Their unrecovered internals deliberately remain outside this module.
pub trait WorldStreamEffects {
    fn add_high_collection(&mut self, id: u64, x: u32, z: u32);
    fn finalize_world_bounds(&mut self);
}

/// Checked status of the retained native world-bounds publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldBoundsError {
    Pending,
    PublicationPoisoned,
    Store(UnpublishError),
}
impl fmt::Display for WorldBoundsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "world bounds publication: {self:?}")
    }
}
impl std::error::Error for WorldBoundsError {}

type WorldBoundsState = Option<Result<LocalSpatialBoundsWords, WorldBoundsError>>;

/// Shared consumer capability for the SAME runtime's `CalcWorldBBox` result.
/// This is a host ownership boundary, not a second file or geometry registry.
#[derive(Clone, Debug)]
pub struct WorldBoundsPublication {
    value: Arc<Mutex<WorldBoundsState>>,
}
impl WorldBoundsPublication {
    fn new() -> Self {
        Self {
            value: Arc::new(Mutex::new(None)),
        }
    }

    /// Reads only a result actually retained by stream finalization.
    ///
    /// # Errors
    /// Returns `Pending` before finalization, the stored calculation failure,
    /// or `PublicationPoisoned`. No failed path returns a fallback world box.
    pub fn read(&self) -> Result<LocalSpatialBoundsWords, WorldBoundsError> {
        self.value
            .lock()
            .map_err(|_| WorldBoundsError::PublicationPoisoned)?
            .clone()
            .unwrap_or(Err(WorldBoundsError::Pending))
    }
}

/// Concrete catalog/local-Octree/global-store composition used by
/// [`crate::world_stream_adapter::WorldStreamAdapter`].
pub struct WorldStreamRuntime<Gate, Effects> {
    catalog: WorldStreamCatalog,
    store: RuntimeStore,
    gate: Gate,
    effects: Effects,
    world_bounds: WorldBoundsPublication,
}

impl<Gate, Effects> WorldStreamRuntime<Gate, Effects> {
    /// Creates an empty publication store over the supplied shared catalog.
    #[must_use]
    pub fn new(catalog: WorldStreamCatalog, gate: Gate, effects: Effects) -> Self {
        Self {
            catalog,
            store: RuntimeStore::new(),
            gate,
            effects,
            world_bounds: WorldBoundsPublication::new(),
        }
    }

    /// Retains the actual bounds publication for consumers such as camera capture.
    /// The legacy void finalize callback is not readiness: consumers must read
    /// this handle and propagate its checked failure before using bounds.
    #[must_use]
    pub fn world_bounds(&self) -> WorldBoundsPublication {
        self.world_bounds.clone()
    }

    fn calculate_world_bounds(&self) -> Result<LocalSpatialBoundsWords, WorldBoundsError> {
        // Existing one-lock snapshot preserves kind slots 0/1/2 and file order.
        let snapshot = self.store.snapshot().map_err(WorldBoundsError::Store)?;
        let mut files = snapshot.files.iter().flatten();
        let mut bounds = files.next().map_or(
            LocalSpatialBoundsWords {
                min: [0; 4],
                max: [0; 4],
            },
            |file| file.records.spatial_bounds().aggregate,
        );
        // CalcWorldBBox824d8df8 seeds first, then revisits ALL files, including
        // the seed. Reuse the existing finite-domain VMX host expressions;
        // exceptional NaN/signed-zero payload parity remains unproven.
        for file in snapshot.files.iter().flatten() {
            let candidate = file.records.spatial_bounds().aggregate;
            for lane in 0..4 {
                bounds.min[lane] = vmx_min_ordinary(
                    f32::from_bits(bounds.min[lane]),
                    f32::from_bits(candidate.min[lane]),
                )
                .to_bits();
                bounds.max[lane] = vmx_max_ordinary(
                    f32::from_bits(bounds.max[lane]),
                    f32::from_bits(candidate.max[lane]),
                )
                .to_bits();
            }
        }
        Ok(bounds)
    }

    /// Returns the shared registration catalog used as offset/size authority.
    #[must_use]
    pub const fn catalog(&self) -> &WorldStreamCatalog {
        &self.catalog
    }

    /// Returns an owning snapshot of one globally mapped processed record.
    ///
    /// # Errors
    ///
    /// Returns a store invariant error if the host-policy registry is poisoned.
    pub fn published_record(
        &self,
        id: u64,
    ) -> Result<Option<PublishedRecordSnapshot<ProcessedOctreeStorage>>, UnpublishError> {
        self.store.record_snapshot(id)
    }

    /// Returns globally published files in the exact per-kind stable order.
    ///
    /// # Errors
    ///
    /// Returns a store invariant error if the host-policy registry is poisoned.
    pub fn published_files(
        &self,
        kind: StreamKind,
    ) -> Result<
        Vec<PublishedFileSnapshot<RegistrationFileId, ProcessedOctreeStorage>>,
        UnpublishError,
    > {
        self.store.file_snapshots(kind)
    }
}

/// Opaque, unique construction state supplied to the adapter. It retains the
/// descriptor/Initialize result and ascending inserts from
/// `0x824e12fc..0x824e13c0`.
#[doc(hidden)]
#[derive(Debug)]
pub struct RuntimeLocalOctreeBuilder {
    kind: StreamKind,
    raw_logical_len: u32,
    bounds: LocalSpatialIndexBounds,
    resource_plan: OctreeResourcePlan,
    tree: LocalOctree,
}

/// Opaque, unique local ownership transferred into the publication store.
/// Its destructor performs the state-five removals at
/// `0x824e1420..0x824e1468` before destroying local storage.
#[doc(hidden)]
#[derive(Debug)]
pub struct OwnedLocalOctree {
    tree: Option<LocalOctree>,
    accepted_count: usize,
    release_report: Arc<Mutex<LocalReleaseReport>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LocalRemovalFailure {
    index: usize,
    source: OctreeRemovalError,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LocalReleaseReport {
    bounds: LocalSpatialIndexBounds,
    resource_plan: OctreeResourcePlan,
    expected_count: usize,
    started: bool,
    completed: usize,
    failure: Option<LocalRemovalFailure>,
    storage_destroyed: bool,
}

impl Drop for OwnedLocalOctree {
    fn drop(&mut self) {
        if let Ok(mut report) = self.release_report.lock() {
            report.started = true;
        }

        let mut completed = 0usize;
        let mut failure = None;
        if let Some(tree) = self.tree.as_mut() {
            for index in 0..self.accepted_count {
                match tree.remove_next() {
                    Ok(removal) => {
                        debug_assert_eq!(removal.entry.index(), index);
                        completed += 1;
                    }
                    Err(source) => {
                        failure = Some(LocalRemovalFailure { index, source });
                        break;
                    }
                }
            }
        }

        // The local allocation is destroyed only after the complete ascending
        // removal attempt. Native valid-domain execution cannot report an
        // error; stopping at the first checked error is safe PC policy.
        drop(self.tree.take());
        if let Ok(mut report) = self.release_report.lock() {
            report.completed = completed;
            report.failure = failure;
            report.storage_destroyed = true;
        }
    }
}

#[derive(Clone, Debug)]
enum RuntimeTokenState {
    Published,
    Released,
    ReleaseFailed(WorldStreamRuntimeError),
    Unpublished,
    UnpublishFailed(WorldStreamRuntimeError),
}

/// Opaque one-publication capability pairing local release with the later
/// `RemoveStreamFile` boundary at `0x824e1d2c..0x824e1d50`. It is
/// intentionally not cloneable.
#[doc(hidden)]
pub struct RuntimePublicationToken {
    store_token: StoreToken,
    release_report: Arc<Mutex<LocalReleaseReport>>,
    state: Mutex<RuntimeTokenState>,
}

/// Checked failure from the concrete runtime collaborator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldStreamRuntimeError {
    TextureGate(TextureStreamGateError),
    Catalog(CatalogError),
    MissingStreamOffset {
        id: u64,
    },
    MissingStreamSize {
        id: u64,
    },
    StoreQuery(UnpublishError),
    Capacity(CapacityError),
    ResourcePlan(ResourcePlanOverflow),
    CompactIndexOutOfOrder {
        expected: u32,
        supplied: u32,
    },
    LocalInsert {
        compact_index: u32,
        source: OctreeInsertError,
    },
    SealKindMismatch {
        builder: StreamKind,
        supplied: StreamKind,
    },
    SealRawLengthMismatch {
        builder: u32,
        records: u32,
    },
    SealAcceptedCountMismatch {
        inserted: usize,
        records: usize,
    },
    SealStreamTypeMismatch {
        compact_index: usize,
        expected: StreamKind,
        actual: StreamKind,
    },
    SealBboxIndexMismatch {
        compact_index: usize,
        actual: i32,
    },
    SealBboxMismatch {
        compact_index: usize,
    },
    Publish(PublishStoreError),
    StoreRelease(UnpublishError),
    ReleaseReportPoisoned,
    LocalRemovalFailed {
        index: usize,
        source: OctreeRemovalError,
    },
    LocalReleaseIncomplete {
        expected: usize,
        completed: usize,
    },
    UnpublishBlockedByLocalRemoval {
        index: usize,
        source: OctreeRemovalError,
    },
    StoreUnpublish(UnpublishError),
    TokenStatePoisoned,
    TokenAlreadyReleased,
    TokenAlreadyUnpublished,
    TokenReleaseFailed,
    TokenUnpublishFailed,
    TokenStillPublished,
}

impl fmt::Display for WorldStreamRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WorldStreamRuntimeError {}

impl RuntimeLocalOctreeBuilder {
    /// Converts construction state into the unique store-owned resource only
    /// after checking every adapter/store join invariant. These checks are a
    /// high-confidence safe boundary; the native valid path assumes them.
    fn seal(
        self,
        kind: StreamKind,
        records: &ProcessedOctreeStorage,
    ) -> Result<(OwnedLocalOctree, Arc<Mutex<LocalReleaseReport>>), WorldStreamRuntimeError> {
        if self.kind != kind {
            return Err(WorldStreamRuntimeError::SealKindMismatch {
                builder: self.kind,
                supplied: kind,
            });
        }
        if self.raw_logical_len != records.raw_logical_len() {
            return Err(WorldStreamRuntimeError::SealRawLengthMismatch {
                builder: self.raw_logical_len,
                records: records.raw_logical_len(),
            });
        }
        let accepted = records.accepted_prefix();
        if self.tree.bboxes().len() != accepted.len() {
            return Err(WorldStreamRuntimeError::SealAcceptedCountMismatch {
                inserted: self.tree.bboxes().len(),
                records: accepted.len(),
            });
        }
        for (compact_index, (inserted, record)) in
            self.tree.bboxes().iter().zip(accepted).enumerate()
        {
            if record.metadata.stream_type != kind {
                return Err(WorldStreamRuntimeError::SealStreamTypeMismatch {
                    compact_index,
                    expected: kind,
                    actual: record.metadata.stream_type,
                });
            }
            let expected_bbox_index = i32::try_from(compact_index)
                .expect("local Octree capacity bounds every compact index to u16");
            if record.metadata.bbox_index != expected_bbox_index {
                return Err(WorldStreamRuntimeError::SealBboxIndexMismatch {
                    compact_index,
                    actual: record.metadata.bbox_index,
                });
            }
            if inserted.min != record.metadata.bbox_min_words
                || inserted.max != record.metadata.bbox_max_words
            {
                return Err(WorldStreamRuntimeError::SealBboxMismatch { compact_index });
            }
        }

        debug_assert_eq!(self.tree.max_entries(), self.raw_logical_len);
        let report = Arc::new(Mutex::new(LocalReleaseReport {
            bounds: self.bounds,
            resource_plan: self.resource_plan,
            expected_count: accepted.len(),
            started: false,
            completed: 0,
            failure: None,
            storage_destroyed: false,
        }));
        let owner = OwnedLocalOctree {
            tree: Some(self.tree),
            accepted_count: accepted.len(),
            release_report: Arc::clone(&report),
        };
        Ok((owner, report))
    }
}

impl<Gate, Effects> LoadedStreamPublisher for WorldStreamRuntime<Gate, Effects>
where
    Gate: TextureStreamGate,
    Effects: WorldStreamEffects,
{
    type Error = WorldStreamRuntimeError;
    type StreamFileIdentity = RegistrationFileId;
    type LocalSpatialIndexIdentity = RuntimeLocalOctreeBuilder;
    type PublicationToken = RuntimePublicationToken;

    fn retain_stream_file_identity(
        &mut self,
        context: StreamRegistrationContext<'_>,
        _file_path: &NativeCPath,
    ) -> Result<Self::StreamFileIdentity, Self::Error> {
        Ok(context.id().clone())
    }

    fn is_collection_valid(&mut self, id: u64) -> Result<bool, Self::Error> {
        self.store
            .record_snapshot(id)
            .map(|record| record.is_some())
            .map_err(WorldStreamRuntimeError::StoreQuery)
    }

    fn stream_offset(&mut self, id: u64) -> Result<u64, Self::Error> {
        self.catalog
            .stream_offset(id)
            .map_err(WorldStreamRuntimeError::Catalog)?
            .ok_or(WorldStreamRuntimeError::MissingStreamOffset { id })
    }

    fn stream_size(&mut self, id: u64) -> Result<u32, Self::Error> {
        self.catalog
            .stream_size(id)
            .map_err(WorldStreamRuntimeError::Catalog)?
            .ok_or(WorldStreamRuntimeError::MissingStreamSize { id })
    }

    fn add_high_collection(&mut self, id: u64, x: u32, z: u32) {
        self.effects.add_high_collection(id, x, z);
    }

    fn construct_local_spatial_index(
        &mut self,
        kind: StreamKind,
        raw_logical_len: u32,
        bounds: LocalSpatialIndexBounds,
    ) -> Result<Self::LocalSpatialIndexIdentity, Self::Error> {
        // Capacity and the five-slot resource plan are exact for the proven
        // 0..=65,535 domain. The checked 65,536 rejection is safe PC policy.
        let capacity = OctreeCapacity::new(
            usize::try_from(raw_logical_len).expect("u32 always fits supported host usize"),
        )
        .map_err(WorldStreamRuntimeError::Capacity)?;
        let resource_plan = OctreeResourcePlan::for_capacity(capacity)
            .map_err(WorldStreamRuntimeError::ResourcePlan)?;
        // The expanded four-lane words are forwarded unchanged. LocalOctree's
        // normalized floating result is high confidence for ordinary finite
        // inputs; exceptional PPC/VMX bit parity remains unknown.
        let tree = LocalOctree::new(
            capacity,
            AabbWords {
                min: bounds.expanded.min,
                max: bounds.expanded.max,
            },
        );
        Ok(RuntimeLocalOctreeBuilder {
            kind,
            raw_logical_len,
            bounds,
            resource_plan,
            tree,
        })
    }

    fn insert_local_spatial_record(
        &mut self,
        local_spatial_index: &mut Self::LocalSpatialIndexIdentity,
        compact_index: u32,
        bbox_min_words: [u32; 4],
        bbox_max_words: [u32; 4],
    ) -> Result<(), Self::Error> {
        let expected = u32::try_from(local_spatial_index.tree.bboxes().len())
            .expect("local Octree capacity bounds every compact index to u16");
        if compact_index != expected {
            return Err(WorldStreamRuntimeError::CompactIndexOutOfOrder {
                expected,
                supplied: compact_index,
            });
        }
        let bbox = AabbWords {
            min: bbox_min_words,
            max: bbox_max_words,
        };
        let inserted = local_spatial_index
            .tree
            .insert_next(bbox)
            .map_err(|source| WorldStreamRuntimeError::LocalInsert {
                compact_index,
                source,
            })?;
        debug_assert_eq!(
            inserted.raw(),
            u16::try_from(compact_index).expect("validated local Octree compact index fits u16")
        );
        Ok(())
    }

    fn publish_loaded_stream_transactionally(
        &mut self,
        kind: StreamKind,
        stream_file: &Arc<Self::StreamFileIdentity>,
        local_spatial_index: Self::LocalSpatialIndexIdentity,
        file_path: &NativeCPath,
        records: &Arc<ProcessedOctreeStorage>,
    ) -> Result<Self::PublicationToken, Self::Error> {
        let (owner, release_report) = local_spatial_index.seal(kind, records)?;
        let compact_ids: Vec<u64> = records
            .accepted_prefix()
            .iter()
            .map(|record| record.metadata.id)
            .collect();
        let store_token = self
            .store
            .publish(StreamPublicationRequest {
                kind,
                path: file_path.clone(),
                file: Arc::clone(stream_file),
                records: Arc::clone(records),
                local_spatial: owner,
                compact_ids: &compact_ids,
            })
            .map_err(WorldStreamRuntimeError::Publish)?;
        Ok(RuntimePublicationToken {
            store_token,
            release_report,
            state: Mutex::new(RuntimeTokenState::Published),
        })
    }

    fn release_local_spatial(&mut self, token: &Self::PublicationToken) -> Result<(), Self::Error> {
        let mut state = token
            .state
            .lock()
            .map_err(|_| WorldStreamRuntimeError::TokenStatePoisoned)?;
        match &*state {
            RuntimeTokenState::Published => {}
            RuntimeTokenState::Released => {
                return Err(WorldStreamRuntimeError::TokenAlreadyReleased);
            }
            RuntimeTokenState::ReleaseFailed(source) => return Err(source.clone()),
            RuntimeTokenState::Unpublished => {
                return Err(WorldStreamRuntimeError::TokenAlreadyUnpublished);
            }
            RuntimeTokenState::UnpublishFailed(_) => {
                return Err(WorldStreamRuntimeError::TokenUnpublishFailed);
            }
        }

        if let Err(source) = self.store.release_local_spatial(&token.store_token) {
            let error = WorldStreamRuntimeError::StoreRelease(source);
            *state = RuntimeTokenState::ReleaseFailed(error.clone());
            return Err(error);
        }
        let Ok(report) = token.release_report.lock() else {
            let error = WorldStreamRuntimeError::ReleaseReportPoisoned;
            *state = RuntimeTokenState::ReleaseFailed(error.clone());
            return Err(error);
        };
        let error = if let Some(failure) = report.failure {
            Some(WorldStreamRuntimeError::LocalRemovalFailed {
                index: failure.index,
                source: failure.source,
            })
        } else if !report.started
            || !report.storage_destroyed
            || report.completed != report.expected_count
        {
            Some(WorldStreamRuntimeError::LocalReleaseIncomplete {
                expected: report.expected_count,
                completed: report.completed,
            })
        } else {
            None
        };
        drop(report);
        if let Some(error) = error {
            *state = RuntimeTokenState::ReleaseFailed(error.clone());
            return Err(error);
        }
        *state = RuntimeTokenState::Released;
        Ok(())
    }

    fn unpublish_loaded_stream(
        &mut self,
        token: &Self::PublicationToken,
    ) -> Result<(), Self::Error> {
        let mut state = token
            .state
            .lock()
            .map_err(|_| WorldStreamRuntimeError::TokenStatePoisoned)?;
        match &*state {
            RuntimeTokenState::Released => {}
            RuntimeTokenState::ReleaseFailed(WorldStreamRuntimeError::LocalRemovalFailed {
                index,
                source,
            }) => {
                return Err(WorldStreamRuntimeError::UnpublishBlockedByLocalRemoval {
                    index: *index,
                    source: *source,
                });
            }
            RuntimeTokenState::ReleaseFailed(_) => {
                return Err(WorldStreamRuntimeError::TokenReleaseFailed);
            }
            RuntimeTokenState::Published => {
                return Err(WorldStreamRuntimeError::TokenStillPublished);
            }
            RuntimeTokenState::Unpublished => {
                return Err(WorldStreamRuntimeError::TokenAlreadyUnpublished);
            }
            RuntimeTokenState::UnpublishFailed(source) => return Err(source.clone()),
        }
        if let Err(source) = self.store.unpublish(&token.store_token) {
            let error = WorldStreamRuntimeError::StoreUnpublish(source);
            *state = RuntimeTokenState::UnpublishFailed(error.clone());
            return Err(error);
        }
        *state = RuntimeTokenState::Unpublished;
        Ok(())
    }

    fn texture_stream_enabled(&mut self) -> Result<bool, Self::Error> {
        self.gate
            .texture_stream_enabled()
            .map_err(WorldStreamRuntimeError::TextureGate)
    }

    fn finalize_world_bounds(&mut self) {
        let result = self.calculate_world_bounds();
        let succeeded = result.is_ok();
        if let Ok(mut published) = self.world_bounds.value.lock() {
            *published = Some(result);
        } else {
            // The same consumer handle reports poison; never fabricate zeros.
            return;
        }
        if succeeded {
            self.effects.finalize_world_bounds();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game_stream::StreamFile,
        octree::ResourceSlotPlan,
        world_stream_adapter::{
            LocalSpatialBoundsWords, ProcessedOctreeInfo, ProcessedOctreeMetadata,
        },
    };
    use skate2_formats::stream::{CollectionMap, CollectionMapEntry};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    type TestFile = StreamFile<(), CatalogError>;

    #[derive(Clone)]
    struct TestGate {
        enabled: Arc<AtomicBool>,
        samples: Arc<AtomicUsize>,
    }

    impl TextureStreamGate for TestGate {
        fn texture_stream_enabled(&mut self) -> Result<bool, TextureStreamGateError> {
            self.samples.fetch_add(1, Ordering::SeqCst);
            Ok(self.enabled.load(Ordering::SeqCst))
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum EffectEvent {
        AddHigh { id: u64, x: u32, z: u32 },
        Finalize,
    }

    #[derive(Clone)]
    struct TestEffects(Arc<Mutex<Vec<EffectEvent>>>);

    impl WorldStreamEffects for TestEffects {
        fn add_high_collection(&mut self, id: u64, x: u32, z: u32) {
            self.0
                .lock()
                .unwrap()
                .push(EffectEvent::AddHigh { id, x, z });
        }

        fn finalize_world_bounds(&mut self) {
            self.0.lock().unwrap().push(EffectEvent::Finalize);
        }
    }

    type TestRuntimeParts = (
        WorldStreamRuntime<TestGate, TestEffects>,
        Arc<AtomicBool>,
        Arc<AtomicUsize>,
        Arc<Mutex<Vec<EffectEvent>>>,
    );

    fn runtime() -> TestRuntimeParts {
        let enabled = Arc::new(AtomicBool::new(false));
        let samples = Arc::new(AtomicUsize::new(0));
        let events = Arc::new(Mutex::new(Vec::new()));
        (
            WorldStreamRuntime::new(
                WorldStreamCatalog::new(),
                TestGate {
                    enabled: Arc::clone(&enabled),
                    samples: Arc::clone(&samples),
                },
                TestEffects(Arc::clone(&events)),
            ),
            enabled,
            samples,
            events,
        )
    }

    fn file(kind: StreamKind) -> TestFile {
        StreamFile::load("world", "boot", kind).unwrap()
    }

    fn native_path(name: &str) -> NativeCPath {
        NativeCPath::from_bytes(Arc::<[u8]>::from(name.as_bytes()))
    }

    fn bounds() -> LocalSpatialIndexBounds {
        LocalSpatialIndexBounds {
            aggregate: LocalSpatialBoundsWords {
                min: [0xc120_0000, 0xc0a0_0000, 0xc000_0000, 0x3f80_0000],
                max: [0x4120_0000, 0x40a0_0000, 0x4000_0000, 0x3f80_0000],
            },
            expanded: LocalSpatialBoundsWords {
                min: [0xc34a_0000, 0xc348_0000, 0xc346_0000, 0x3f80_0000],
                max: [0x434a_0000, 0x4348_0000, 0x4346_0000, 0x3f80_0000],
            },
        }
    }

    fn zero_accepted_bounds() -> LocalSpatialIndexBounds {
        LocalSpatialIndexBounds {
            aggregate: LocalSpatialBoundsWords {
                min: [0xbf80_0000, 0xbf80_0000, 0xbf80_0000, 0],
                max: [0x3f80_0000, 0x3f80_0000, 0x3f80_0000, 0],
            },
            expanded: LocalSpatialBoundsWords {
                min: [0xc349_0000, 0xc349_0000, 0xc349_0000, 0],
                max: [0x4349_0000, 0x4349_0000, 0x4349_0000, 0],
            },
        }
    }

    fn bbox(index: u16) -> AabbWords {
        let base = f32::from(index) * 4.0;
        AabbWords::from_f32(
            [base, base + 1.0, base + 2.0, 1.0],
            [base + 0.5, base + 1.5, base + 2.5, 1.0],
        )
    }

    fn record(id: u64, index: usize, kind: StreamKind) -> ProcessedOctreeInfo {
        let bbox = bbox(u16::try_from(index).unwrap());
        let index_i32 = i32::try_from(index).unwrap();
        ProcessedOctreeInfo::new(ProcessedOctreeMetadata {
            bbox_min_words: bbox.min,
            bbox_max_words: bbox.max,
            geometry_min_words: [1, 2, 3, 0x3f80_0000],
            geometry_max_words: [4, 5, 6, 0x3f80_0000],
            id,
            offset: 0x1234_0000 + u64::try_from(index).unwrap(),
            position_words: [7, 8, 9, 0x3f80_0000],
            stream_order: 10 + index_i32,
            bbox_index: index_i32,
            ticks: 11,
            size: 12,
            stream_type: kind,
        })
    }

    fn storage(
        raw_logical_len: u32,
        records: Vec<ProcessedOctreeInfo>,
    ) -> Arc<ProcessedOctreeStorage> {
        Arc::new(ProcessedOctreeStorage::new(raw_logical_len, records.into()).unwrap())
    }

    fn builder_for(
        runtime: &mut WorldStreamRuntime<TestGate, TestEffects>,
        kind: StreamKind,
        raw_logical_len: u32,
        records: &ProcessedOctreeStorage,
    ) -> RuntimeLocalOctreeBuilder {
        let mut builder = runtime
            .construct_local_spatial_index(kind, raw_logical_len, bounds())
            .unwrap();
        for (index, record) in records.accepted_prefix().iter().enumerate() {
            runtime
                .insert_local_spatial_record(
                    &mut builder,
                    u32::try_from(index).unwrap(),
                    record.metadata.bbox_min_words,
                    record.metadata.bbox_max_words,
                )
                .unwrap();
        }
        builder
    }

    fn publish(
        runtime: &mut WorldStreamRuntime<TestGate, TestEffects>,
        file: &TestFile,
        kind: StreamKind,
        path: &NativeCPath,
        records: &Arc<ProcessedOctreeStorage>,
    ) -> RuntimePublicationToken {
        let identity = Arc::new(
            runtime
                .retain_stream_file_identity(file.registration_context(), path)
                .unwrap(),
        );
        let builder = builder_for(runtime, kind, records.raw_logical_len(), records);
        runtime
            .publish_loaded_stream_transactionally(kind, &identity, builder, path, records)
            .unwrap()
    }

    fn collection(id: u64, offset: u32, size: u32) -> CollectionMapEntry {
        CollectionMapEntry {
            id,
            parent_id: 0,
            payload_offset: offset,
            payload_size: size,
            asset_count: 0,
            filename: "collection".to_owned(),
            name_offset: 0,
            alignment_pad: 0,
        }
    }

    fn register_map(
        catalog: &mut WorldStreamCatalog,
        file: &TestFile,
        entries: Vec<CollectionMapEntry>,
    ) {
        use crate::mounted_stream_backend::StreamRegistrationSink;

        catalog
            .collection_map_loaded(
                file.registration_context(),
                &CollectionMap {
                    version: 3,
                    stamp: 1,
                    stream_format: 9,
                    name_table_offset: 0,
                    collections: entries,
                },
            )
            .unwrap();
    }

    #[test]
    fn raw_capacities_resource_plans_and_bounds_cover_corpus_edges() {
        let expected = [
            (0, 1, 0x60, 0x40, 0x60),
            (284, 143, 0x3e80, 0x23c0, 0x35a0),
            (285, 143, 0x3ea8, 0x23e0, 0x35c0),
            (65_535, 32_768, 0x38_0018, 0x20_0020, 0x30_0020),
        ];
        let (mut runtime, _, _, _) = runtime();
        for (raw, nodes, total, node_offset, entry_offset) in expected {
            let builder = runtime
                .construct_local_spatial_index(StreamKind::Presentation, raw, bounds())
                .unwrap();
            assert_eq!(builder.raw_logical_len, raw);
            assert_eq!(builder.kind, StreamKind::Presentation);
            assert_eq!(builder.bounds, bounds());
            assert_eq!(builder.tree.max_entries(), raw);
            assert_eq!(
                builder.tree.extent(),
                AabbWords {
                    min: [0xc34a_0000, 0xc34a_0000, 0xc34a_0000, 0x3f80_0000],
                    max: [0x434a_0000, 0x434a_0000, 0x434a_0000, 0x3f80_0000],
                }
            );
            assert_eq!(builder.resource_plan.node_count, nodes);
            assert_eq!(builder.resource_plan.bbox_offset, 0x40);
            assert_eq!(builder.resource_plan.node_offset, node_offset);
            assert_eq!(builder.resource_plan.entry_offset, entry_offset);
            assert_eq!(
                builder.resource_plan.resources,
                [
                    ResourceSlotPlan {
                        size: total,
                        alignment: 32,
                    },
                    ResourceSlotPlan {
                        size: 0,
                        alignment: 1
                    },
                    ResourceSlotPlan {
                        size: 0,
                        alignment: 1
                    },
                    ResourceSlotPlan {
                        size: 0,
                        alignment: 1
                    },
                    ResourceSlotPlan {
                        size: 0,
                        alignment: 1
                    },
                ]
            );
        }
        assert_eq!(
            runtime
                .construct_local_spatial_index(StreamKind::Presentation, 65_536, bounds())
                .unwrap_err(),
            WorldStreamRuntimeError::Capacity(CapacityError {
                requested: 65_536,
                maximum: 65_535,
            })
        );
    }

    #[test]
    fn zero_accepted_default_bounds_reach_the_local_octree_bit_exact() {
        let (mut runtime, _, _, _) = runtime();
        let builder = runtime
            .construct_local_spatial_index(StreamKind::Texture, 2, zero_accepted_bounds())
            .unwrap();
        assert_eq!(builder.bounds, zero_accepted_bounds());
        assert_eq!(
            builder.tree.extent(),
            AabbWords {
                min: [0xc349_0000, 0xc349_0000, 0xc349_0000, 0],
                max: [0x4349_0000, 0x4349_0000, 0x4349_0000, 0],
            }
        );
    }

    #[test]
    fn inserts_are_ascending_and_forward_all_bbox_bits() {
        let (mut runtime, _, _, _) = runtime();
        let mut builder = runtime
            .construct_local_spatial_index(StreamKind::Simulation, 2, bounds())
            .unwrap();
        let first = bbox(0);
        let second = bbox(1);
        assert_eq!(
            runtime.insert_local_spatial_record(&mut builder, 1, first.min, first.max),
            Err(WorldStreamRuntimeError::CompactIndexOutOfOrder {
                expected: 0,
                supplied: 1,
            })
        );
        runtime
            .insert_local_spatial_record(&mut builder, 0, first.min, first.max)
            .unwrap();
        runtime
            .insert_local_spatial_record(&mut builder, 1, second.min, second.max)
            .unwrap();
        assert_eq!(builder.tree.bboxes(), &[first, second]);
    }

    #[test]
    fn seal_checks_kind_raw_count_stream_type_index_and_bbox_bits() {
        let (mut runtime, _, _, _) = runtime();
        let empty = storage(0, vec![]);
        let builder = builder_for(&mut runtime, StreamKind::Presentation, 0, &empty);
        assert_eq!(
            builder.seal(StreamKind::Simulation, &empty).unwrap_err(),
            WorldStreamRuntimeError::SealKindMismatch {
                builder: StreamKind::Presentation,
                supplied: StreamKind::Simulation,
            }
        );

        let records = storage(1, vec![record(1, 0, StreamKind::Presentation)]);
        let builder = runtime
            .construct_local_spatial_index(StreamKind::Presentation, 1, bounds())
            .unwrap();
        assert_eq!(
            builder
                .seal(StreamKind::Presentation, &records)
                .unwrap_err(),
            WorldStreamRuntimeError::SealAcceptedCountMismatch {
                inserted: 0,
                records: 1,
            }
        );

        let wrong_raw = storage(2, vec![]);
        let builder = builder_for(&mut runtime, StreamKind::Presentation, 1, &empty);
        assert_eq!(
            builder
                .seal(StreamKind::Presentation, &wrong_raw)
                .unwrap_err(),
            WorldStreamRuntimeError::SealRawLengthMismatch {
                builder: 1,
                records: 2,
            }
        );

        for mutation in 0..3 {
            let mut value = record(1, 0, StreamKind::Presentation);
            match mutation {
                0 => value.metadata.stream_type = StreamKind::Texture,
                1 => value.metadata.bbox_index = 1,
                2 => value.metadata.bbox_min_words[3] ^= 1,
                _ => unreachable!(),
            }
            let mutated = storage(1, vec![value]);
            let mut builder = runtime
                .construct_local_spatial_index(StreamKind::Presentation, 1, bounds())
                .unwrap();
            let exact = bbox(0);
            runtime
                .insert_local_spatial_record(&mut builder, 0, exact.min, exact.max)
                .unwrap();
            let error = builder
                .seal(StreamKind::Presentation, &mutated)
                .unwrap_err();
            assert!(matches!(
                (mutation, error),
                (0, WorldStreamRuntimeError::SealStreamTypeMismatch { .. })
                    | (1, WorldStreamRuntimeError::SealBboxIndexMismatch { .. })
                    | (2, WorldStreamRuntimeError::SealBboxMismatch { .. })
            ));
        }
    }

    #[test]
    fn raw_tail_is_retained_but_only_the_accepted_prefix_is_published() {
        let (mut runtime, _, _, _) = runtime();
        let file = file(StreamKind::Presentation);
        let records = storage(3, vec![record(0x77, 0, StreamKind::Presentation)]);
        let token = publish(
            &mut runtime,
            &file,
            StreamKind::Presentation,
            &native_path("p"),
            &records,
        );
        let snapshot = runtime.published_record(0x77).unwrap().unwrap();
        assert_eq!(snapshot.compact_index, 0);
        assert_eq!(snapshot.records.raw_logical_len(), 3);
        assert_eq!(snapshot.records.opaque_tail_len(), 2);
        assert!(runtime.published_record(0).unwrap().is_none());
        runtime.release_local_spatial(&token).unwrap();
        runtime.unpublish_loaded_stream(&token).unwrap();
    }

    #[test]
    fn catalog_is_offset_size_authority_and_missing_values_are_typed() {
        let (mut runtime, _, _, _) = runtime();
        assert_eq!(
            runtime.stream_offset(0x55),
            Err(WorldStreamRuntimeError::MissingStreamOffset { id: 0x55 })
        );
        assert_eq!(
            runtime.stream_size(0x55),
            Err(WorldStreamRuntimeError::MissingStreamSize { id: 0x55 })
        );
        let file = file(StreamKind::Simulation);
        let mut sink = runtime.catalog.clone();
        register_map(
            &mut sink,
            &file,
            vec![collection(0x55, 0xf123_4567, 0xe987_6543)],
        );
        assert_eq!(runtime.stream_offset(0x55).unwrap(), 0xf123_4567);
        assert_eq!(runtime.stream_size(0x55).unwrap(), 0xe987_6543);
    }

    #[test]
    fn global_store_not_catalog_is_collection_validity_authority() {
        let (mut runtime, _, _, _) = runtime();
        let file = file(StreamKind::Presentation);
        let mut sink = runtime.catalog.clone();
        register_map(&mut sink, &file, vec![collection(7, 1, 2)]);
        assert!(!runtime.is_collection_valid(7).unwrap());
        let records = storage(1, vec![record(7, 0, StreamKind::Presentation)]);
        let token = publish(
            &mut runtime,
            &file,
            StreamKind::Presentation,
            &native_path("valid"),
            &records,
        );
        assert!(runtime.is_collection_valid(7).unwrap());
        runtime.release_local_spatial(&token).unwrap();
        runtime.unpublish_loaded_stream(&token).unwrap();
        assert!(!runtime.is_collection_valid(7).unwrap());
    }

    #[test]
    fn retained_context_identity_is_exact_and_equal_paths_stay_isolated() {
        let (mut runtime, _, _, _) = runtime();
        let first = file(StreamKind::Presentation);
        let second = file(StreamKind::Presentation);
        let path = native_path("same");
        let first_id = runtime
            .retain_stream_file_identity(first.registration_context(), &path)
            .unwrap();
        let second_id = runtime
            .retain_stream_file_identity(second.registration_context(), &path)
            .unwrap();
        assert_eq!(first_id, *first.registration_id());
        assert_eq!(second_id, *second.registration_id());
        assert_ne!(first_id, second_id);

        let first_records = storage(0, vec![]);
        let second_records = storage(0, vec![]);
        let first_token = publish(
            &mut runtime,
            &first,
            StreamKind::Presentation,
            &path,
            &first_records,
        );
        let second_token = publish(
            &mut runtime,
            &second,
            StreamKind::Presentation,
            &path,
            &second_records,
        );
        let files = runtime.published_files(StreamKind::Presentation).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(*files[0].file, *first.registration_id());
        assert_eq!(*files[1].file, *second.registration_id());
        for token in [&first_token, &second_token] {
            runtime.release_local_spatial(token).unwrap();
            runtime.unpublish_loaded_stream(token).unwrap();
        }
    }

    #[test]
    fn first_wins_collision_keeps_both_files_and_each_token_owns_only_its_wins() {
        let (mut runtime, _, _, _) = runtime();
        let first = file(StreamKind::Presentation);
        let second = file(StreamKind::Simulation);
        let first_records = storage(1, vec![record(9, 0, StreamKind::Presentation)]);
        let second_records = storage(1, vec![record(9, 0, StreamKind::Simulation)]);
        let first_token = publish(
            &mut runtime,
            &first,
            StreamKind::Presentation,
            &native_path("first"),
            &first_records,
        );
        let second_token = publish(
            &mut runtime,
            &second,
            StreamKind::Simulation,
            &native_path("second"),
            &second_records,
        );
        let winner = runtime.published_record(9).unwrap().unwrap();
        assert_eq!(
            winner.publication_id,
            first_token.store_token.publication_id()
        );
        assert!(second_token.store_token.won_ids().is_empty());
        assert_eq!(
            runtime
                .published_files(StreamKind::Presentation)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            runtime
                .published_files(StreamKind::Simulation)
                .unwrap()
                .len(),
            1
        );

        runtime.release_local_spatial(&second_token).unwrap();
        runtime.unpublish_loaded_stream(&second_token).unwrap();
        assert_eq!(
            runtime.published_record(9).unwrap().unwrap().publication_id,
            first_token.store_token.publication_id()
        );
        runtime.release_local_spatial(&first_token).unwrap();
        runtime.unpublish_loaded_stream(&first_token).unwrap();
        assert!(runtime.published_record(9).unwrap().is_none());
    }

    #[test]
    fn successful_release_removes_every_entry_before_destroying_local_storage() {
        let (mut runtime, _, _, _) = runtime();
        let file = file(StreamKind::Texture);
        let records = storage(
            3,
            vec![
                record(1, 0, StreamKind::Texture),
                record(2, 1, StreamKind::Texture),
                record(3, 2, StreamKind::Texture),
            ],
        );
        let token = publish(
            &mut runtime,
            &file,
            StreamKind::Texture,
            &native_path("release"),
            &records,
        );
        runtime.release_local_spatial(&token).unwrap();
        let report = *token.release_report.lock().unwrap();
        assert_eq!(
            report,
            LocalReleaseReport {
                bounds: bounds(),
                resource_plan: OctreeResourcePlan::for_capacity(OctreeCapacity::new(3).unwrap())
                    .unwrap(),
                expected_count: 3,
                started: true,
                completed: 3,
                failure: None,
                storage_destroyed: true,
            }
        );
        runtime.unpublish_loaded_stream(&token).unwrap();
    }

    #[test]
    fn removal_failure_is_reported_and_permanently_blocks_unpublication() {
        let (mut runtime, _, _, _) = runtime();
        let file = file(StreamKind::Simulation);
        let records = storage(
            2,
            vec![
                record(1, 0, StreamKind::Simulation),
                record(2, 1, StreamKind::Simulation),
            ],
        );
        let capacity = OctreeCapacity::new(2).unwrap();
        let mut invalid_tree = LocalOctree::new(
            capacity,
            AabbWords {
                min: bounds().expanded.min,
                max: bounds().expanded.max,
            },
        );
        invalid_tree.insert_next(bbox(0)).unwrap();
        let report = Arc::new(Mutex::new(LocalReleaseReport {
            bounds: bounds(),
            resource_plan: OctreeResourcePlan::for_capacity(capacity).unwrap(),
            expected_count: 2,
            started: false,
            completed: 0,
            failure: None,
            storage_destroyed: false,
        }));
        let owner = OwnedLocalOctree {
            tree: Some(invalid_tree),
            accepted_count: 2,
            release_report: Arc::clone(&report),
        };
        let store_token = runtime
            .store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Simulation,
                path: native_path("failure"),
                file: Arc::new(file.registration_id().clone()),
                records: Arc::clone(&records),
                local_spatial: owner,
                compact_ids: &[1, 2],
            })
            .unwrap();
        let token = RuntimePublicationToken {
            store_token,
            release_report: report,
            state: Mutex::new(RuntimeTokenState::Published),
        };
        let expected_source = OctreeRemovalError::NoInsertedEntry {
            next_removal: 1,
            inserted: 1,
        };
        assert_eq!(
            runtime.release_local_spatial(&token),
            Err(WorldStreamRuntimeError::LocalRemovalFailed {
                index: 1,
                source: expected_source,
            })
        );
        let report = *token.release_report.lock().unwrap();
        assert_eq!(report.completed, 1);
        assert_eq!(report.failure.unwrap().source, expected_source);
        assert!(report.storage_destroyed);
        for _ in 0..2 {
            assert_eq!(
                runtime.unpublish_loaded_stream(&token),
                Err(WorldStreamRuntimeError::UnpublishBlockedByLocalRemoval {
                    index: 1,
                    source: expected_source,
                })
            );
        }
        assert!(runtime.published_record(1).unwrap().is_some());
        assert_eq!(
            runtime
                .published_files(StreamKind::Simulation)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn runtime_token_is_one_shot_for_release_and_unpublish() {
        let (mut runtime, _, _, _) = runtime();
        let file = file(StreamKind::Texture);
        let records = storage(0, vec![]);
        let token = publish(
            &mut runtime,
            &file,
            StreamKind::Texture,
            &native_path("token"),
            &records,
        );
        assert_eq!(
            runtime.unpublish_loaded_stream(&token),
            Err(WorldStreamRuntimeError::TokenStillPublished)
        );
        runtime.release_local_spatial(&token).unwrap();
        assert_eq!(
            runtime.release_local_spatial(&token),
            Err(WorldStreamRuntimeError::TokenAlreadyReleased)
        );
        runtime.unpublish_loaded_stream(&token).unwrap();
        assert_eq!(
            runtime.unpublish_loaded_stream(&token),
            Err(WorldStreamRuntimeError::TokenAlreadyUnpublished)
        );
    }

    #[test]
    fn texture_gate_is_live_and_effects_are_forwarded_without_extra_work() {
        let (mut runtime, enabled, samples, events) = runtime();
        assert!(!runtime.texture_stream_enabled().unwrap());
        enabled.store(true, Ordering::SeqCst);
        assert!(runtime.texture_stream_enabled().unwrap());
        assert_eq!(samples.load(Ordering::SeqCst), 2);

        runtime.add_high_collection(0xfedc_ba98_7654_3210, 0x8000_0000, 0x7fc0_1234);
        runtime.finalize_world_bounds();
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                EffectEvent::AddHigh {
                    id: 0xfedc_ba98_7654_3210,
                    x: 0x8000_0000,
                    z: 0x7fc0_1234,
                },
                EffectEvent::Finalize,
            ]
        );
    }

    #[test]
    fn world_bounds_seed_actual_files_and_fold_all_four_lanes_without_expansion() {
        let (mut runtime, _, _, events) = runtime();
        let retained = runtime.world_bounds();
        assert_eq!(retained.read(), Err(WorldBoundsError::Pending));
        // Publish in reverse kind order: native traverses kind vectors instead.
        let mut tokens = Vec::new();
        for (kind, id, low, high) in [
            (
                StreamKind::Texture,
                3,
                [3.0, 1.0, 0.0, -7.0],
                [4.0, 2.0, 1.0, 9.0],
            ),
            (
                StreamKind::Simulation,
                2,
                [2.0, -3.0, 0.0, -2.0],
                [3.0, -2.0, 1.0, 2.0],
            ),
            (
                StreamKind::Presentation,
                1,
                [1.0, 1.0, -1.0, 1.0],
                [2.0, 2.0, 0.0, 1.0],
            ),
        ] {
            let mut item = record(id, 0, kind);
            item.metadata.bbox_min_words = low.map(f32::to_bits);
            item.metadata.bbox_max_words = high.map(f32::to_bits);
            let records = storage(1, vec![item]);
            assert_ne!(
                records.spatial_bounds().aggregate,
                records.spatial_bounds().expanded
            );
            tokens.push(publish(
                &mut runtime,
                &file(kind),
                kind,
                &native_path("bounds"),
                &records,
            ));
        }
        runtime.finalize_world_bounds();
        assert_eq!(
            retained.read().unwrap(),
            LocalSpatialBoundsWords {
                min: [1.0, -3.0, -1.0, -7.0].map(f32::to_bits),
                max: [4.0, 2.0, 1.0, 9.0].map(f32::to_bits),
            }
        );
        assert_eq!(*events.lock().unwrap(), [EffectEvent::Finalize]);
        for token in &tokens {
            runtime.release_local_spatial(token).unwrap();
            runtime.unpublish_loaded_stream(token).unwrap();
        }
        runtime.finalize_world_bounds();
        assert_eq!(
            retained.read().unwrap(),
            LocalSpatialBoundsWords {
                min: [0; 4],
                max: [0; 4]
            }
        );
    }

    #[test]
    fn empty_accepted_file_uses_its_actual_local_aggregate_instead_of_empty_world() {
        let (mut runtime, _, _, _) = runtime();
        let records = storage(0, vec![]);
        let token = publish(
            &mut runtime,
            &file(StreamKind::Simulation),
            StreamKind::Simulation,
            &native_path("empty-file"),
            &records,
        );
        runtime.finalize_world_bounds();
        assert_eq!(
            runtime.world_bounds().read().unwrap(),
            zero_accepted_bounds().aggregate
        );
        runtime.release_local_spatial(&token).unwrap();
        runtime.unpublish_loaded_stream(&token).unwrap();
    }

    #[test]
    fn successful_finalization_effect_observes_the_same_already_published_owner() {
        struct Observe {
            handle: Option<WorldBoundsPublication>,
            observed: Vec<LocalSpatialBoundsWords>,
        }
        impl WorldStreamEffects for Observe {
            fn add_high_collection(&mut self, _: u64, _: u32, _: u32) {}
            fn finalize_world_bounds(&mut self) {
                self.observed
                    .push(self.handle.as_ref().unwrap().read().unwrap());
            }
        }
        let (_, enabled, samples, _) = runtime();
        let mut runtime = WorldStreamRuntime::new(
            WorldStreamCatalog::new(),
            TestGate { enabled, samples },
            Observe {
                handle: None,
                observed: vec![],
            },
        );
        runtime.effects.handle = Some(runtime.world_bounds());
        runtime.finalize_world_bounds();
        assert_eq!(
            runtime.effects.observed,
            [LocalSpatialBoundsWords {
                min: [0; 4],
                max: [0; 4]
            }]
        );
    }

    #[test]
    fn poisoned_publication_never_falls_back_or_emits_success_effect() {
        let (mut runtime, _, _, events) = runtime();
        let handle = runtime.world_bounds();
        let _ = std::panic::catch_unwind(|| {
            let _guard = handle.value.lock().unwrap();
            panic!("controlled publication poison");
        });
        runtime.finalize_world_bounds();
        assert_eq!(handle.read(), Err(WorldBoundsError::PublicationPoisoned));
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn poisoned_file_store_replaces_old_bounds_with_checked_failure() {
        let (mut runtime, _, _, events) = runtime();
        runtime.finalize_world_bounds();
        let retained = runtime.world_bounds();
        assert!(retained.read().is_ok());
        events.lock().unwrap().clear();
        let kind = StreamKind::Simulation;
        let file = file(kind);
        let path = native_path("poison");
        let identity = Arc::new(
            runtime
                .retain_stream_file_identity(file.registration_context(), &path)
                .unwrap(),
        );
        let records = storage(0, vec![]);
        let builder = builder_for(&mut runtime, kind, 0, &records);
        let (owner, _) = builder.seal(kind, &records).unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = runtime.store.publish_with_precommit(
                StreamPublicationRequest {
                    kind,
                    path,
                    file: identity,
                    records,
                    local_spatial: owner,
                    compact_ids: &[],
                },
                |_| -> Result<(), ()> {
                    panic!("controlled store poison");
                },
            );
        }));
        runtime.finalize_world_bounds();
        assert_eq!(
            retained.read(),
            Err(WorldBoundsError::Store(UnpublishError::StoreLockPoisoned))
        );
        assert!(events.lock().unwrap().is_empty());
    }
}
