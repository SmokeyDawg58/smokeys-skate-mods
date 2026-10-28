//! Durable resident assets for successfully processed Simulation arenas.
//!
//! The native load path constructs an arena asset before collection
//! publication, and the later Simulation receiver walks the collection's
//! first-null-bounded asset prefix. This module supplies the narrower dense,
//! successful host composition intended for the validated dense BAM corpus:
//! decode and collision projection, followed by construction of an immutable
//! [`SimulationNormalArenaPlan`] owner that can survive until a later receiver.
//!
//! This is not native ownership or allocator parity. In particular, the
//! current resident container omits null asset slots, so the native
//! `PreventNavmeshLoadingHack`/allocation-null terminator behavior remains an
//! explicit boundary. No registration, publication, callback,
//! acknowledgement, or teardown effect occurs here.

use crate::{
    catalog_resident_load_factory::CollectionAssetBuilderSessionFactory,
    collection_resident_streamer::CollectionId,
    owned_resident_collection::ResidentAssetIdentity,
    resident_collection_load_driver::{CollectionAssetBuildInput, CollectionAssetBuilder},
    simulation_collection_adapter::{
        PreparedSimulationCollection, SimulationCollectionPrepareError,
    },
    simulation_collision_asset::{
        SimulationCollisionAsset, SimulationCollisionAssetBuildError,
        SimulationCollisionAssetBuilder, SimulationCollisionAssetBuilderSessions,
        SimulationCollisionAssetSessionError,
    },
    simulation_normal_arena_plan::{SimulationNormalArenaPlan, SimulationNormalArenaPlanError},
    simulation_resident_registry::{SimulationResidentOccurrence, SimulationResidentVisitError},
    world_stream_catalog::{CollectionRegistrationSnapshot, WorldStreamCatalog},
};
use std::{error::Error, fmt, sync::Arc};

/// One fully owned, successfully processed Simulation arena with no runtime
/// registration effects.
#[derive(Debug)]
pub struct SimulationResidentArenaAsset {
    plan: Arc<SimulationNormalArenaPlan>,
}

impl SimulationResidentArenaAsset {
    /// Completes the currently reconstructed normal-plan construction before
    /// this asset can enter resident storage.
    ///
    /// # Errors
    ///
    /// Returns the exact normal-plan construction error without performing
    /// any manager effect.
    fn from_collision_asset(
        asset: SimulationCollisionAsset,
    ) -> Result<Self, SimulationNormalArenaPlanError> {
        Ok(Self {
            plan: Arc::new(SimulationNormalArenaPlan::from_collision_asset(asset)?),
        })
    }

    #[must_use]
    pub fn plan(&self) -> &SimulationNormalArenaPlan {
        &self.plan
    }

    /// Clones the immutable plan owner needed by a later prepared collection.
    #[must_use]
    pub fn plan_owner(&self) -> Arc<SimulationNormalArenaPlan> {
        Arc::clone(&self.plan)
    }

    #[cfg(test)]
    pub(crate) fn from_plan_for_test(plan: Arc<SimulationNormalArenaPlan>) -> Self {
        Self { plan }
    }
}

impl ResidentAssetIdentity for SimulationResidentArenaAsset {
    fn resident_asset_id(&self) -> CollectionId {
        CollectionId(self.plan.static_plan().asset().decoded.asset_id)
    }
}

/// Typed allocation failure while gathering immutable plan owners from one
/// exact resident occurrence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulationResidentPlanOwnerStorageError {
    requested: usize,
}

impl SimulationResidentPlanOwnerStorageError {
    #[must_use]
    pub const fn requested(self) -> usize {
        self.requested
    }
}

impl fmt::Display for SimulationResidentPlanOwnerStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "could not retain {} resident Simulation plan owners",
            self.requested
        )
    }
}

impl Error for SimulationResidentPlanOwnerStorageError {}

/// Failure while converting one exact dense resident occurrence into all
/// prevalidated Simulation receiver views.
#[derive(Debug)]
pub enum SimulationResidentCollectionPrepareError {
    Visit(SimulationResidentVisitError<SimulationResidentPlanOwnerStorageError>),
    Collection(SimulationCollectionPrepareError),
}

impl fmt::Display for SimulationResidentCollectionPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Visit(source) => write!(formatter, "resident occurrence visit failed: {source}"),
            Self::Collection(source) => {
                write!(
                    formatter,
                    "Simulation collection preparation failed: {source}"
                )
            }
        }
    }
}

impl Error for SimulationResidentCollectionPrepareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Visit(source) => Some(source),
            Self::Collection(source) => Some(source),
        }
    }
}

/// Retry-preserving preparation failure.
#[derive(Debug)]
pub struct SimulationResidentCollectionPrepareFailure {
    error: SimulationResidentCollectionPrepareError,
    occurrence: SimulationResidentOccurrence<SimulationResidentArenaAsset>,
}

impl SimulationResidentCollectionPrepareFailure {
    #[must_use]
    pub const fn error(&self) -> &SimulationResidentCollectionPrepareError {
        &self.error
    }

    #[must_use]
    pub const fn occurrence(&self) -> &SimulationResidentOccurrence<SimulationResidentArenaAsset> {
        &self.occurrence
    }

    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        SimulationResidentCollectionPrepareError,
        SimulationResidentOccurrence<SimulationResidentArenaAsset>,
    ) {
        (self.error, self.occurrence)
    }
}

impl fmt::Display for SimulationResidentCollectionPrepareFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for SimulationResidentCollectionPrepareFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

/// A fully prepared dense receiver input still bound to the exact registry
/// occurrence from which its owning plan handles were gathered.
#[derive(Debug)]
pub struct PreparedSimulationResidentCollection {
    prepared: PreparedSimulationCollection,
    occurrence: SimulationResidentOccurrence<SimulationResidentArenaAsset>,
}

impl PreparedSimulationResidentCollection {
    #[must_use]
    pub fn collection_id(&self) -> CollectionId {
        self.occurrence.id()
    }

    #[must_use]
    pub const fn prepared(&self) -> &PreparedSimulationCollection {
        &self.prepared
    }

    #[must_use]
    pub const fn occurrence(&self) -> &SimulationResidentOccurrence<SimulationResidentArenaAsset> {
        &self.occurrence
    }

    /// Compares the exact authored sequence of immutable resident plan owners.
    ///
    /// This is stronger than collection/arena value identity: reusing the same
    /// IDs with replacement plan allocations does not compare equal. Registry
    /// occurrence identity remains a separate check.
    #[must_use]
    pub fn has_same_plan_snapshot(&self, other: &Self) -> bool {
        self.prepared.has_same_plan_sequence(&other.prepared)
    }
}

impl SimulationResidentOccurrence<SimulationResidentArenaAsset> {
    /// Gathers the dense resident arena sequence and prevalidates every
    /// currently modeled receiver view without performing runtime effects.
    ///
    /// The occurrence is consumed into either the success bundle or the
    /// failure value, preserving exact lifetime identity for retry or later
    /// receiver composition.
    ///
    /// # Errors
    ///
    /// Returns the exact occurrence with the nested visit/allocation or
    /// collection-preparation failure. No registry or manager state changes.
    pub fn prepare_collection(
        self,
    ) -> Result<PreparedSimulationResidentCollection, SimulationResidentCollectionPrepareFailure>
    {
        let id = self.id();
        let mut plans = Vec::new();
        if let Err(source) = self.try_for_each(|asset| {
            let requested = plans.len().saturating_add(1);
            plans
                .try_reserve(1)
                .map_err(|_| SimulationResidentPlanOwnerStorageError { requested })?;
            plans.push(asset.plan_owner());
            Ok(())
        }) {
            return Err(SimulationResidentCollectionPrepareFailure {
                error: SimulationResidentCollectionPrepareError::Visit(source),
                occurrence: self,
            });
        }
        let prepared = match PreparedSimulationCollection::try_new(id.0, plans) {
            Ok(prepared) => prepared,
            Err(source) => {
                return Err(SimulationResidentCollectionPrepareFailure {
                    error: SimulationResidentCollectionPrepareError::Collection(source),
                    occurrence: self,
                });
            }
        };
        Ok(PreparedSimulationResidentCollection {
            prepared,
            occurrence: self,
        })
    }
}

/// Builder that adds durable normal-plan construction after the checked
/// Simulation collision builder.
#[derive(Clone, Debug)]
pub struct SimulationResidentArenaAssetBuilder {
    inner: SimulationCollisionAssetBuilder,
}

impl CollectionAssetBuilder<crate::world_stream_catalog::AssetRegistrationSnapshot>
    for SimulationResidentArenaAssetBuilder
{
    type Asset = SimulationResidentArenaAsset;
    type Error = SimulationResidentArenaBuildError;

    fn build(
        &mut self,
        input: CollectionAssetBuildInput<
            '_,
            crate::world_stream_catalog::AssetRegistrationSnapshot,
        >,
    ) -> Result<Option<Self::Asset>, Self::Error> {
        let collision = self
            .inner
            .build(input)
            .map_err(SimulationResidentArenaBuildError::Collision)?;
        collision.map(finish_collision_asset).transpose()
    }

    /// Decode, collision projection and plan construction read only the
    /// job's own payload and metadata.
    fn prefetch(
        &self,
        job: &crate::resident_collection_load_driver::OwnedDecompressionJob<
            crate::world_stream_catalog::AssetRegistrationSnapshot,
        >,
    ) -> Option<crate::resident_collection_load_driver::Prefetch<Self::Asset, Self::Error>> {
        job.build_input()?;
        let mut builder = self.clone();
        let job = job.clone();
        crate::resident_collection_load_driver::Prefetch::spawn(move || {
            builder.build(job.build_input().expect("prefetched payload was in range"))
        })
    }
}

fn finish_collision_asset(
    asset: SimulationCollisionAsset,
) -> Result<SimulationResidentArenaAsset, SimulationResidentArenaBuildError> {
    SimulationResidentArenaAsset::from_collision_asset(asset)
        .map_err(SimulationResidentArenaBuildError::Plan)
}

/// Cloneable checked-session factory for ordinary Simulation collections.
#[derive(Clone, Debug)]
pub struct SimulationResidentArenaAssetBuilderSessions {
    inner: SimulationCollisionAssetBuilderSessions,
}

impl SimulationResidentArenaAssetBuilderSessions {
    #[must_use]
    pub const fn new(catalog: WorldStreamCatalog) -> Self {
        Self {
            inner: SimulationCollisionAssetBuilderSessions::new(catalog),
        }
    }

    #[must_use]
    pub const fn catalog(&self) -> &WorldStreamCatalog {
        self.inner.catalog()
    }
}

impl CollectionAssetBuilderSessionFactory for SimulationResidentArenaAssetBuilderSessions {
    type Asset = SimulationResidentArenaAsset;
    type Builder = SimulationResidentArenaAssetBuilder;
    type Error = SimulationCollisionAssetSessionError;

    fn begin_session(
        &mut self,
        collection: &CollectionRegistrationSnapshot,
    ) -> Result<Self::Builder, Self::Error> {
        Ok(SimulationResidentArenaAssetBuilder {
            inner: self.inner.begin_session(collection)?,
        })
    }
}

/// Failure while turning one streamed Simulation arena into a durable plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationResidentArenaBuildError {
    Collision(SimulationCollisionAssetBuildError),
    Plan(SimulationNormalArenaPlanError),
}

impl fmt::Display for SimulationResidentArenaBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Collision(source) => {
                write!(
                    formatter,
                    "Simulation collision construction failed: {source}"
                )
            }
            Self::Plan(source) => {
                write!(
                    formatter,
                    "Simulation normal-plan construction failed: {source}"
                )
            }
        }
    }
}

impl Error for SimulationResidentArenaBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Collision(source) => Some(source),
            Self::Plan(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection_load_pump::LoadedAssetCollection;
    use crate::{
        collection_load_pump::CollectionStreamFormat,
        collection_resident_streamer::{ResidentCollectionRegistry, ResidentCollectionStorage},
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        game_stream::{StreamFile, StreamKind},
        mounted_collection_producer::MountedCollectionBlockView,
        mounted_stream_backend::StreamRegistrationSink,
        owned_resident_collection::{
            OwnedResidentAssetsVisitError, OwnedResidentCollection, OwnedResidentCollectionError,
            OwnedResidentCollectionStorage,
        },
        resident_collection_load_driver::CollectionAssetBuildMode,
        simulation_resident_registry::{
            SimulationResidentRegistry, SimulationResidentRegistryError,
        },
        simulation_static_arena_plan::SimulationStaticArenaPlanError,
        world::CollisionArena,
        world_stream_catalog::CatalogError,
    };
    use skate2_formats::{
        pegasus::TABLE_OF_CONTENTS_TYPE_ID,
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::{
            ArenaResources, AssetMapEntry, BaseResourceDescriptor, CollectionMap,
            CollectionMapEntry,
        },
    };

    fn empty_collision_asset(asset_id: u64, arena_id: u32) -> SimulationCollisionAsset {
        let descriptor = ResourceDescriptor {
            size: 0,
            alignment: 1,
        };
        let resources = ArenaResources {
            resources: std::array::from_fn(|_| Vec::new()),
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
            id: arena_id,
            entry_count: 0,
            used_entry_count: 0,
            alignment: 1,
            virtual_size: 0,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: [descriptor; 5],
            resources_used: [descriptor; 5],
            resource_offsets: [0; 5],
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        SimulationCollisionAsset {
            decoded: DecodedArenaAsset::from_parts_for_test(
                asset_id,
                WORLD_STREAM_ARENA_ASSET_TYPE,
                resources,
                arena,
            ),
            collision: CollisionArena {
                id: arena_id,
                grind_assets: Vec::new(),
                aggregate_roots: Vec::new(),
                primitive_sets: Vec::new(),
                simple_primitive_sets: Vec::new(),
                meshes: Vec::new(),
                collision_models: Vec::new(),
                instances: Vec::new(),
            },
        }
    }

    fn registered_collection(
        kind: StreamKind,
    ) -> (WorldStreamCatalog, CollectionRegistrationSnapshot) {
        type TestFile = StreamFile<(), CatalogError>;
        let file = TestFile::load("world", "resident", kind).unwrap();
        let map = CollectionMap {
            version: 3,
            stamp: 0x1020_3040_5060_7080,
            stream_format: 2,
            name_table_offset: 0,
            collections: vec![CollectionMapEntry {
                id: 0xfedc_ba98_7654_3210,
                parent_id: 8,
                payload_offset: 0x100,
                payload_size: 0x200,
                asset_count: 1,
                filename: "cell".to_owned(),
                name_offset: 0,
                alignment_pad: -1,
            }],
        };
        let mut catalog = WorldStreamCatalog::new();
        catalog
            .collection_map_loaded(file.registration_context(), &map)
            .unwrap();
        let collection = catalog.collection(0xfedc_ba98_7654_3210).unwrap().unwrap();
        (catalog, collection)
    }

    fn arena_resource() -> Vec<u8> {
        let mut bytes = vec![0_u8; 0x200];
        bytes[..12].copy_from_slice(b"\x89RW4xb2\0\r\n\x1a\n");
        bytes[12] = 1;
        bytes[13] = 32;
        bytes[14] = 4;
        bytes[16..20].copy_from_slice(b"454\0");
        bytes[20..24].copy_from_slice(b"000\0");
        bytes[28..32].copy_from_slice(&0x8646_a1f7_u32.to_be_bytes());
        bytes[32..36].copy_from_slice(&1_u32.to_be_bytes());
        bytes[36..40].copy_from_slice(&1_u32.to_be_bytes());
        bytes[40..44].copy_from_slice(&16_u32.to_be_bytes());
        bytes[48..52].copy_from_slice(&0x1e8_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&0x200_u32.to_be_bytes());
        bytes[72..76].copy_from_slice(&16_u32.to_be_bytes());
        bytes[0x1e8..0x1ec].copy_from_slice(&0x100_u32.to_be_bytes());
        bytes[0x1f0..0x1f4].copy_from_slice(&0x10_u32.to_be_bytes());
        bytes[0x1f4..0x1f8].copy_from_slice(&16_u32.to_be_bytes());
        bytes[0x1f8..0x1fc].copy_from_slice(&0x10_u32.to_be_bytes());
        bytes[0x1fc..0x200].copy_from_slice(&0x00eb_0008_u32.to_be_bytes());
        bytes
    }

    fn arena_envelope(resource: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0_u8; 80];
        let size = u32::try_from(resource.len()).unwrap();
        bytes[..4].copy_from_slice(&size.to_be_bytes());
        bytes[4..8].copy_from_slice(&size.to_be_bytes());
        bytes.extend_from_slice(resource);
        bytes
    }

    fn asset_snapshot(
        collection: &CollectionRegistrationSnapshot,
        asset_id: u64,
        serialized_size: u32,
    ) -> crate::world_stream_catalog::AssetRegistrationSnapshot {
        crate::world_stream_catalog::AssetRegistrationSnapshot {
            first_owner: collection.owner.clone(),
            entry: AssetMapEntry {
                id: asset_id,
                asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
                serialized_size,
                resources: [
                    BaseResourceDescriptor {
                        size: serialized_size,
                        alignment: 0,
                    },
                    BaseResourceDescriptor {
                        size: 0,
                        alignment: 0,
                    },
                    BaseResourceDescriptor {
                        size: 0,
                        alignment: 0,
                    },
                    BaseResourceDescriptor {
                        size: 0,
                        alignment: 0,
                    },
                    BaseResourceDescriptor {
                        size: 0,
                        alignment: 0,
                    },
                ],
                name_offset: 0,
                unused: 0,
            },
            reference_count: 1,
        }
    }

    type ResidentStorage = OwnedResidentCollectionStorage<
        SimulationResidentArenaAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;
    type ResidentRegistry = SimulationResidentRegistry<
        SimulationResidentArenaAsset,
        fn(SimulationResidentRegistryError) -> SimulationResidentRegistryError,
        SimulationResidentRegistryError,
    >;

    fn resident_storage() -> ResidentStorage {
        OwnedResidentCollectionStorage::new(std::convert::identity)
    }

    fn resident_registry() -> (
        ResidentRegistry,
        crate::simulation_resident_registry::SimulationResidentLookup<SimulationResidentArenaAsset>,
    ) {
        SimulationResidentRegistry::new(std::convert::identity)
    }

    fn resident_collection(
        storage: &mut ResidentStorage,
        arenas: &[(u64, u32)],
    ) -> OwnedResidentCollection<SimulationResidentArenaAsset> {
        let collection = storage
            .create_empty(i32::try_from(arenas.len()).unwrap())
            .unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        for &(asset_id, arena_id) in arenas {
            writer.add_asset(
                SimulationResidentArenaAsset::from_collision_asset(empty_collision_asset(
                    asset_id, arena_id,
                ))
                .unwrap(),
            );
        }
        collection
    }

    #[test]
    fn resident_asset_retains_full_identity_and_one_plan_owner() {
        let asset_id = 0xfedc_ba98_7654_3210;
        let resident =
            SimulationResidentArenaAsset::from_collision_asset(empty_collision_asset(asset_id, 9))
                .unwrap();
        let owner = resident.plan_owner();

        assert_eq!(resident.resident_asset_id(), CollectionId(asset_id));
        assert_eq!(resident.plan().operations(), []);
        assert_eq!(resident.plan().static_plan().asset().decoded.arena.id, 9);
        assert!(std::ptr::eq(resident.plan(), owner.as_ref()));
        assert_eq!(Arc::strong_count(&owner), 2);
    }

    #[test]
    fn plan_owner_can_outlive_the_resident_asset_without_runtime_effects() {
        let resident =
            SimulationResidentArenaAsset::from_collision_asset(empty_collision_asset(7, 11))
                .unwrap();
        let owner = resident.plan_owner();
        let weak = Arc::downgrade(&owner);

        drop(resident);
        assert!(weak.upgrade().is_some());
        assert_eq!(owner.static_plan().asset().decoded.asset_id, 7);
        drop(owner);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn normal_plan_failure_is_mapped_before_a_resident_asset_exists() {
        let mut collision = empty_collision_asset(7, 11);
        collision.decoded.arena.entry_count = 1;
        collision.decoded.arena.used_entry_count = 1;
        collision.decoded.arena.dictionary.push(DictionaryEntry {
            resource_index: 0,
            object_offset: 0,
            relocation_offset: 0,
            size: 1,
            alignment: 1,
            type_index: 0,
            type_id: TABLE_OF_CONTENTS_TYPE_ID,
            object_range: 0..1,
        });

        assert!(matches!(
            finish_collision_asset(collision),
            Err(SimulationResidentArenaBuildError::Plan(
                SimulationNormalArenaPlanError::Static(
                    SimulationStaticArenaPlanError::ObjectRangeOutsideResource {
                        dictionary_index: 0,
                        type_id: TABLE_OF_CONTENTS_TYPE_ID,
                    }
                )
            ))
        ));
    }

    #[test]
    fn session_factory_delegates_simulation_kind_and_snapshot_checks() {
        let (catalog, collection) = registered_collection(StreamKind::Simulation);
        let mut tampered = collection.clone();
        tampered.size += 1;
        let mut sessions = SimulationResidentArenaAssetBuilderSessions::new(catalog);

        sessions.begin_session(&collection).unwrap();
        assert_eq!(
            sessions.begin_session(&tampered).unwrap_err(),
            SimulationCollisionAssetSessionError::CollectionSnapshotMismatch {
                id: 0xfedc_ba98_7654_3210,
            }
        );

        let (catalog, collection) = registered_collection(StreamKind::Presentation);
        let mut sessions = SimulationResidentArenaAssetBuilderSessions::new(catalog);
        assert_eq!(
            sessions.begin_session(&collection).unwrap_err(),
            SimulationCollisionAssetSessionError::WrongStreamKind(StreamKind::Presentation)
        );
    }

    #[test]
    fn checked_builder_constructs_the_durable_plan_only_after_collision_decode() {
        let (catalog, collection) = registered_collection(StreamKind::Simulation);
        let mut sessions = SimulationResidentArenaAssetBuilderSessions::new(catalog);
        let mut builder = sessions.begin_session(&collection).unwrap();
        let resource = arena_resource();
        let payload = arena_envelope(&resource);
        let block = MountedCollectionBlockView::from_test_bytes(payload);
        let stored_size = u32::try_from(block.bytes().len()).unwrap();
        let asset_id = 0xeeee_aaaa_5555_1111;
        let snapshot = asset_snapshot(
            &collection,
            asset_id,
            u32::try_from(resource.len()).unwrap(),
        );

        let error = builder
            .build(CollectionAssetBuildInput {
                asset_id,
                asset_info: &snapshot,
                block: &block,
                payload: block.bytes(),
                stored_size,
                raw_format: CollectionStreamFormat(1),
                mode: CollectionAssetBuildMode::Compressed,
            })
            .unwrap_err();
        assert!(matches!(
            error,
            SimulationResidentArenaBuildError::Collision(_)
        ));

        let resident = builder
            .build(CollectionAssetBuildInput {
                asset_id,
                asset_info: &snapshot,
                block: &block,
                payload: block.bytes(),
                stored_size,
                raw_format: CollectionStreamFormat(2),
                mode: CollectionAssetBuildMode::CompressedArena,
            })
            .unwrap()
            .unwrap();
        assert_eq!(resident.resident_asset_id(), CollectionId(asset_id));
        assert!(resident.plan().operations().is_empty());
        assert_eq!(
            resident
                .plan()
                .static_plan()
                .asset()
                .decoded
                .resources
                .resources[0],
            resource
        );
    }

    #[test]
    fn exact_occurrence_preparation_preserves_dense_order_and_duplicate_arenas() {
        let collection_id = CollectionId(0xfedc_ba98_7654_3210);
        let mut storage = resident_storage();
        let collection = resident_collection(
            &mut storage,
            &[
                (0x1111_2222_3333_4444, 7),
                (9, 8),
                (0x1111_2222_3333_4444, 7),
            ],
        );
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(collection_id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(collection_id, &token).unwrap();

        let bundle = lookup
            .lookup(collection_id)
            .unwrap()
            .unwrap()
            .prepare_collection()
            .unwrap();
        assert_eq!(bundle.collection_id(), collection_id);
        assert_eq!(bundle.prepared().collection_id(), collection_id.0);
        assert_eq!(bundle.prepared().arena_count(), 3);
        assert_eq!(
            (0..3)
                .map(|index| bundle.prepared().arena_identity(index).unwrap())
                .collect::<Vec<_>>(),
            [
                (0x1111_2222_3333_4444, 7),
                (9, 8),
                (0x1111_2222_3333_4444, 7),
            ]
        );

        assert_eq!(bundle.occurrence().id(), collection_id);
        registry.remove_exact(collection_id, &token).unwrap();
        drop(wrapper);
        storage.destroy(collection);
        assert_eq!(bundle.prepared().arena_identity(1), Some((9, 8)));
    }

    #[test]
    fn empty_preparation_and_bundle_drop_do_not_change_publication() {
        let collection_id = CollectionId(7);
        let mut storage = resident_storage();
        let collection = resident_collection(&mut storage, &[]);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(collection_id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(collection_id, &token).unwrap();

        let bundle = lookup
            .lookup(collection_id)
            .unwrap()
            .unwrap()
            .prepare_collection()
            .unwrap();
        assert_eq!(bundle.prepared().arena_count(), 0);
        drop(bundle);
        assert!(lookup.lookup(collection_id).unwrap().is_some());
        assert!(wrapper.is_collection_attached());

        registry.remove_exact(collection_id, &token).unwrap();
        drop(wrapper);
        storage.destroy(collection);
    }

    #[test]
    fn stale_prepare_failure_returns_the_same_occurrence_across_reload() {
        let collection_id = CollectionId(7);
        let mut storage = resident_storage();
        let first = resident_collection(&mut storage, &[(11, 1)]);
        let (mut registry, lookup) = resident_registry();
        let first_wrapper = registry.create_wrapper(collection_id, &first).unwrap();
        let first_token = ResidentRegistry::stable_token(&first_wrapper);
        registry.publish(collection_id, &first_token).unwrap();
        let stale = lookup.lookup(collection_id).unwrap().unwrap();
        registry.remove_exact(collection_id, &first_token).unwrap();
        drop(first_wrapper);
        storage.destroy(first);

        let second = resident_collection(&mut storage, &[(22, 2)]);
        let second_wrapper = registry.create_wrapper(collection_id, &second).unwrap();
        let second_token = ResidentRegistry::stable_token(&second_wrapper);
        registry.publish(collection_id, &second_token).unwrap();

        let failure = stale.prepare_collection().unwrap_err();
        assert!(matches!(
            failure.error(),
            SimulationResidentCollectionPrepareError::Visit(
                SimulationResidentVisitError::UnknownOccurrence { id }
            ) if *id == collection_id
        ));
        let (_, stale) = failure.into_parts();
        assert!(matches!(
            stale.prepare_collection().unwrap_err().error(),
            SimulationResidentCollectionPrepareError::Visit(
                SimulationResidentVisitError::UnknownOccurrence { .. }
            )
        ));
        let fresh = lookup
            .lookup(collection_id)
            .unwrap()
            .unwrap()
            .prepare_collection()
            .unwrap();
        assert_eq!(fresh.prepared().arena_identity(0), Some((22, 2)));

        drop(fresh);
        registry.remove_exact(collection_id, &second_token).unwrap();
        drop(second_wrapper);
        storage.destroy(second);
    }

    #[test]
    fn detached_asset_visit_is_not_converted_to_an_empty_collection() {
        let collection_id = CollectionId(7);
        let mut storage = resident_storage();
        let collection = resident_collection(&mut storage, &[(11, 1)]);
        let (mut registry, lookup) = resident_registry();
        let wrapper = registry.create_wrapper(collection_id, &collection).unwrap();
        let token = ResidentRegistry::stable_token(&wrapper);
        registry.publish(collection_id, &token).unwrap();
        let occurrence = lookup.lookup(collection_id).unwrap().unwrap();
        storage.destroy(collection);

        let failure = occurrence.prepare_collection().unwrap_err();
        assert!(matches!(
            failure.error(),
            SimulationResidentCollectionPrepareError::Visit(SimulationResidentVisitError::Assets(
                OwnedResidentAssetsVisitError::Detached
            ))
        ));
        assert_eq!(failure.into_parts().1.id(), collection_id);
        registry.remove_exact(collection_id, &token).unwrap();
        drop(wrapper);
    }
}
