//! Read-only original PROXY census through existing metadata and resident loaders.
//! Does not publish Presentation registrations or claim proxy activation.
use crate::{
    catalog_resident_load_factory::CatalogCollectionAssetMetadata,
    collection_load_pump::CollectionStreamFormat,
    collection_parent_manager::{
        CollectionParentEffects, CollectionParentManager, PlannedParentCallbackKind,
    },
    collection_resident_streamer::{CollectionId, ResidentCollectionStorage},
    decoded_arena_asset::{DecodedArenaAsset, DecodedArenaAssetBuilder},
    game_stream::{StreamFile, StreamKind},
    mounted_stream_backend::StreamRegistrationSink,
    owned_resident_collection::{OwnedResidentCollectionError, OwnedResidentCollectionStorage},
    resident_collection_load_driver::{
        CollectionDecompressionWake, MountedResidentCollectionLoadDriver,
        MountedResidentCollectionLoadSpec,
    },
    simulation_static_arena_plan::environment_is_hybrid,
    world_stream_catalog::{CatalogError, WorldStreamCatalog},
};
use skate2_formats::{
    pegasus::{
        INSTANCE_DATA_TYPE_ID, INSTANCE_ENVIRONMENT_ASSET_TYPE_ID, InstanceData,
        TABLE_OF_CONTENTS_TYPE_ID, TableOfContents,
    },
    stream::{AssetToc, CollectionMap, CollectionSpace},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
    sync::Arc,
    time::Instant,
};

// The diagnostic invokes only plan_load; callback effects are never delivered.
struct CensusOnlyParentEffects;

impl CollectionParentEffects for CensusOnlyParentEffects {
    type Error = Infallible;

    fn simulation_register(&mut self, _id: CollectionId) -> Result<(), Self::Error> {
        unreachable!("read-only ancestor census cannot register Simulation")
    }

    fn presentation_register(&mut self, _id: CollectionId) -> Result<(), Self::Error> {
        unreachable!("read-only ancestor census cannot register Presentation")
    }

    fn simulation_defer_update(&mut self) -> Result<(), Self::Error> {
        unreachable!("read-only ancestor census cannot update Simulation")
    }

    fn simulation_deregister(&mut self, _id: CollectionId) -> Result<(), Self::Error> {
        unreachable!("read-only ancestor census cannot deregister Simulation")
    }

    fn presentation_deregister(&mut self, _id: CollectionId) -> Result<(), Self::Error> {
        unreachable!("read-only ancestor census cannot deregister Presentation")
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "single read-only mounted census retains actual decoded collection owners"
)]
#[test]
#[ignore = "requires SKATE2_GAME_ROOT; root-run read-only PROXY capability census"]
fn original_proxy_assets_through_existing_resident_decoder() {
    let mounts =
        Arc::new(crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus());
    let file = StreamFile::<(), CatalogError>::load(
        "data\\content\\world\\stream",
        "PROXY",
        StreamKind::Presentation,
    )
    .unwrap();
    let context = file.registration_context();
    let paths = context.paths();
    let map = CollectionMap::parse(&mounts.read_path(&paths.collection_map()).unwrap()).unwrap();
    let toc = AssetToc::parse(&mounts.read_path(&paths.asset_toc()).unwrap()).unwrap();
    let space =
        CollectionSpace::parse(&mounts.read_path(&paths.collection_space).unwrap()).unwrap();
    let mut catalog = WorldStreamCatalog::new();
    // Actual parser outputs through existing sink. This diagnostic does not
    // mark the StreamFile state loaded or run normal-world XML/high effects.
    catalog.collection_map_loaded(context, &map).unwrap();
    catalog.asset_toc_loaded(context, &toc).unwrap();
    eprintln!(
        "PROXY metadata: collections={} spatial={} assets={} format={}",
        map.collections.len(),
        space.collections.len(),
        toc.assets.len(),
        map.stream_format
    );
    let mut parents = CollectionParentManager::new(CensusOnlyParentEffects);
    for descriptor in &map.collections {
        parents.add_parent(
            CollectionId(descriptor.id),
            CollectionId(descriptor.parent_id),
        );
        let registered = catalog.collection(descriptor.id).unwrap().unwrap();
        eprintln!(
            "PROXY XSM: id={:016x} parent={:016x} assets={} path={}",
            registered.id, registered.parent_id, registered.asset_count, registered.path
        );
    }
    let spatial_ids: BTreeSet<_> = space.collections.iter().map(|row| row.id).collect();
    let mut required_ids = BTreeSet::new();
    let mut required_order = Vec::new();
    // XSS authored order is used only to enumerate diagnostic requests. The
    // proxy's native spatial sort and asynchronous completion are not modeled.
    for spatial in &space.collections {
        let plan = parents.plan_load(CollectionId(spatial.id)).unwrap();
        assert_eq!(plan.last().unwrap().id, CollectionId(spatial.id));
        assert_eq!(
            plan.last().unwrap().callback,
            PlannedParentCallbackKind::Requested
        );
        assert!(
            plan[..plan.len() - 1]
                .iter()
                .all(|request| request.callback == PlannedParentCallbackKind::LoadParent)
        );
        for request in plan {
            let id = request.id.0;
            assert!(
                catalog.collection(id).unwrap().is_some(),
                "required XSM id {id:016x}"
            );
            if required_ids.insert(id) {
                required_order.push(id);
            }
        }
    }
    let parent_only_ids: BTreeSet<_> = required_ids.difference(&spatial_ids).copied().collect();
    let unrequired_xsm_ids: BTreeSet<_> = map
        .collections
        .iter()
        .map(|descriptor| descriptor.id)
        .filter(|id| !required_ids.contains(id))
        .collect();
    eprintln!(
        "PROXY required collections: unique={} spatial={} parent_only={} parent_ids={parent_only_ids:016x?} unrequired_xsm_ids={unrequired_xsm_ids:016x?}",
        required_order.len(),
        spatial_ids.len(),
        parent_only_ids.len()
    );
    let mut storage =
        OwnedResidentCollectionStorage::<DecodedArenaAsset, _, OwnedResidentCollectionError>::new(
            std::convert::identity::<OwnedResidentCollectionError>,
        );
    let mut retained = Vec::new();
    let mut dictionary_types = BTreeMap::<u32, usize>::new();
    let mut toc_types = BTreeMap::<u32, usize>::new();
    let mut instance_types = BTreeMap::<u32, usize>::new();
    let mut spatial_environment_count = 0_usize;
    let mut admitted_kind10_count = 0_usize;
    let mut hybrid_skipped_count = 0_usize;
    let mut arena_count = 0_usize;
    let mut required_assets = BTreeMap::<u64, u32>::new();
    let mut parent_only_assets = BTreeSet::<u64>::new();
    let deadline = Instant::now() + std::time::Duration::from_secs(300);
    // First-seen plan order follows authored XSS enumeration for this census;
    // it does not reproduce native proxy spatial sorting or completion order.
    for (ordinal, id) in required_order.iter().copied().enumerate() {
        let descriptor = catalog
            .collection(id)
            .unwrap()
            .expect("planned collection in XSM");
        let count = i32::try_from(descriptor.asset_count).unwrap();
        let collection = storage.create_empty(count).unwrap();
        if count != 0 {
            let writer = storage.writer(&collection).unwrap();
            let wake = CollectionDecompressionWake::default();
            let mut driver = MountedResidentCollectionLoadDriver::new(
                Arc::clone(&mounts),
                MountedResidentCollectionLoadSpec {
                    path: Arc::clone(&descriptor.path),
                    payload_offset: descriptor.offset,
                    request_id: 0,
                    target_assets: count,
                    format: CollectionStreamFormat(
                        i32::try_from(descriptor.stream_format).unwrap(),
                    ),
                },
                CatalogCollectionAssetMetadata::new(catalog.clone()),
                writer,
                DecodedArenaAssetBuilder,
                wake.clone(),
            )
            .unwrap();
            while !driver.pump().is_complete() {
                assert!(Instant::now() < deadline, "PROXY diagnostic deadline");
                driver.update_one_tick().unwrap();
                // The diagnostic drives the wake normally requested by the
                // resident streamer after its collection update.
                let _ = wake.request();
                std::thread::yield_now();
            }
        }
        eprintln!(
            "PROXY collection {ordinal}: id={:016x} assets={count} path={}",
            id, descriptor.path
        );
        let mut visited = 0;
        collection
            .resident_assets_view()
            .try_for_each::<Infallible>(|asset| {
                visited += 1;
                arena_count += 1;
                if let Some(previous_type) =
                    required_assets.insert(asset.asset_id, asset.asset_type)
                {
                    assert_eq!(previous_type, asset.asset_type, "asset ID type conflict");
                }
                if parent_only_ids.contains(&id) {
                    parent_only_assets.insert(asset.asset_id);
                    eprintln!(
                        "PROXY parent asset: collection={id:016x} id={:016x} type={:08x}",
                        asset.asset_id, asset.asset_type
                    );
                }
                eprintln!(
                    "PROXY arena: id={:016x} type={:08x} dictionaries={}",
                    asset.asset_id,
                    asset.asset_type,
                    asset.arena.dictionary.len()
                );
                for entry in &asset.arena.dictionary {
                    *dictionary_types.entry(entry.type_id).or_default() += 1;
                    if entry.type_id == TABLE_OF_CONTENTS_TYPE_ID {
                        let bytes = asset
                            .arena
                            .object_bytes(&asset.resources.resources, entry)
                            .unwrap();
                        let inner = TableOfContents::parse(bytes).unwrap();
                        for row in inner.entries {
                            *toc_types.entry(row.object_type).or_default() += 1;
                            eprintln!(
                                "PROXY TOC: guid={:016x} type={:08x} object={:?} name={:?}",
                                row.guid, row.object_type, row.object, row.name
                            );
                        }
                    } else if entry.type_id == INSTANCE_DATA_TYPE_ID {
                        let bytes = asset
                            .arena
                            .object_bytes(&asset.resources.resources, entry)
                            .unwrap();
                        let data = InstanceData::record_table(bytes).unwrap();
                        *instance_types.entry(data.type_id()).or_default() += data.len();
                        eprintln!(
                            "PROXY instances: type={:08x} count={}",
                            data.type_id(),
                            data.len()
                        );
                        if spatial_ids.contains(&id)
                            && data.type_id() == INSTANCE_ENVIRONMENT_ASSET_TYPE_ID
                        {
                            let parsed = InstanceData::parse(bytes).unwrap();
                            assert_eq!(parsed.instances.len(), data.len());
                            for (inline_index, instance) in parsed.instances.iter().enumerate() {
                                assert_eq!(data.record(inline_index).unwrap().guid(), instance.guid);
                                let hybrid = environment_is_hybrid(
                                    &asset.arena,
                                    &asset.resources.resources,
                                    instance,
                                )
                                .unwrap();
                                spatial_environment_count += 1;
                                if hybrid {
                                    hybrid_skipped_count += 1;
                                } else {
                                    admitted_kind10_count += 1;
                                }
                                eprintln!(
                                    "PROXY Environment: collection={id:016x} arena={:016x} inline={inline_index} guid={:016x} collision_model={:?} render_model={:?} hybrid={hybrid} admit_pres_kind10={}",
                                    asset.asset_id,
                                    instance.guid,
                                    instance.collision_model,
                                    instance.render_model,
                                    !hybrid
                                );
                            }
                        }
                    }
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(visited, descriptor.asset_count);
        retained.push(collection);
    }
    eprintln!(
        "PROXY totals: retained={} arenas={arena_count} unique_required_assets={} parent_collection_unique_assets={} dictionary={dictionary_types:x?} toc={toc_types:x?} instance={instance_types:x?}",
        retained.len(),
        required_assets.len(),
        parent_only_assets.len()
    );
    eprintln!("PROXY required asset IDs/types: {required_assets:016x?}");
    eprintln!(
        "PROXY Environment totals: spatial_records={spatial_environment_count} admitted_pres_kind10={admitted_kind10_count} hybrid_skipped={hybrid_skipped_count}"
    );
    assert_eq!(spatial_environment_count, space.collections.len());
    let xst_assets: BTreeMap<_, _> = toc
        .assets
        .iter()
        .map(|asset| (asset.id, asset.asset_type))
        .collect();
    let missing_xst_ids: BTreeSet<_> = required_assets
        .keys()
        .filter(|id| !xst_assets.contains_key(id))
        .copied()
        .collect();
    let unrequired_xst_ids: BTreeSet<_> = xst_assets
        .keys()
        .filter(|id| !required_assets.contains_key(id))
        .copied()
        .collect();
    eprintln!(
        "PROXY XST comparison: entries={} missing_xst_ids={missing_xst_ids:016x?} unrequired_xst_ids={unrequired_xst_ids:016x?}",
        xst_assets.len()
    );
    assert!(
        missing_xst_ids.is_empty(),
        "decoded required asset absent from XST"
    );
    assert_eq!(retained.len(), required_ids.len());
    assert!(spatial_ids.is_subset(&required_ids));
}
