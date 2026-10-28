use super::*;
use crate::{
    collection_load_pump::LoadedAssetCollection,
    collection_resident_streamer::{ResidentCollectionRegistry, ResidentCollectionStorage},
    owned_resident_collection::{
        OwnedResidentCollection, OwnedResidentCollectionError, OwnedResidentCollectionStorage,
    },
    simulation_resident_registry::{
        SimulationResidentContainerAssetKind, SimulationResidentRegistryError,
    },
    world_painter_layer_asset::tests::fixture,
};
type Registry = SimulationResidentRegistry<
    SimulationResidentArenaAsset,
    fn(SimulationResidentRegistryError) -> SimulationResidentRegistryError,
    SimulationResidentRegistryError,
>;
type Storage = OwnedResidentCollectionStorage<
    SimulationResidentArenaAsset,
    fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
    OwnedResidentCollectionError,
>;

#[test]
fn presentation_inner_publication_shares_namespace_and_exact_removal() {
    let (registry, _) = Registry::new(std::convert::identity);
    let lookup = registry.selected_inner_lookup();
    let first_plan = fixture(9, 0, 1, Some([64.; 8]));
    let second_plan = fixture(9, 0, 1, Some([64.; 8]));
    let wrap = |plan: Arc<SimulationNormalArenaPlan>| {
        let index = plan.operations().iter().position(|operation| matches!(operation,
            crate::simulation_normal_arena_plan::SimulationNormalOperation::RegistryAsset { .. })).unwrap();
        SimulationInnerRegistryAsset::try_from_operation(plan, index).unwrap()
    };
    let first = wrap(first_plan);
    let duplicate = wrap(second_plan);
    lookup.publish_inner(&first).unwrap();
    lookup.publish_inner(&duplicate).unwrap();
    assert!(Arc::ptr_eq(&registry.try_inner(CollectionId(9)).unwrap().unwrap(), &first));
    lookup.remove_inner(&duplicate).unwrap();
    assert!(Arc::ptr_eq(&lookup.try_inner(CollectionId(9)).unwrap().unwrap(), &first));
    lookup.remove_inner(&first).unwrap();
    assert!(lookup.try_inner(CollectionId(9)).unwrap().is_none());
    lookup.publish_inner(&duplicate).unwrap();
    lookup.remove_inner(&first).unwrap();
    assert!(Arc::ptr_eq(&lookup.try_inner(CollectionId(9)).unwrap().unwrap(), &duplicate));
}

#[test]
fn delayed_processed_release_cannot_erase_a_reloaded_same_id_plan() {
    let (mut registry, _) = Registry::new(std::convert::identity);
    let services = SelectedSimulationAssetServices::construct_resident(&mut registry);
    let first = collection(&[9]);
    let wrapper = registry.create_wrapper(CollectionId(100), &first).unwrap();
    let token = Registry::stable_token(&wrapper);
    registry.publish(CollectionId(100), &token).unwrap();
    let old = Arc::clone(&services.try_inner(9).unwrap().unwrap().plan);
    registry.remove_exact(CollectionId(100), &token).unwrap();
    let second = collection(&[9]);
    let next = registry.create_wrapper(CollectionId(100), &second).unwrap();
    registry
        .publish(CollectionId(100), &Registry::stable_token(&next))
        .unwrap();
    let current = services.try_inner(9).unwrap().unwrap();
    assert_eq!(services.retained_counts(), (2, 2));
    services.try_release_plan(&old).unwrap();
    assert_eq!(services.retained_counts(), (1, 1));
    assert!(Arc::ptr_eq(
        &current,
        &services.try_inner(9).unwrap().unwrap()
    ));
}

fn collection(ids: &[u64]) -> OwnedResidentCollection<SimulationResidentArenaAsset> {
    let mut storage = Storage::new(std::convert::identity);
    let collection = storage
        .create_empty(i32::try_from(ids.len()).unwrap())
        .unwrap();
    let mut writer = storage.writer(&collection).unwrap();
    for &id in ids {
        writer.add_asset(SimulationResidentArenaAsset::from_plan_for_test(fixture(
            id,
            0,
            1,
            Some([64.0; 8]),
        )));
    }
    collection
}

#[test]
fn inner_then_actual_outer_per_arena_share_first_winner_namespace() {
    let (mut registry, _) = Registry::new(std::convert::identity);
    let services = SelectedSimulationAssetServices::construct_resident(&mut registry);
    // First arena inner9 then outer(high9); second arena's inner(high9)
    // must lose to that actual outer. Publishing all inners first is wrong.
    let high = 9 ^ 0x8000_0000_0000_0000;
    let assets = collection(&[9, high]);
    let wrapper = registry.create_wrapper(CollectionId(9), &assets).unwrap();
    let token = Registry::stable_token(&wrapper);
    registry.publish(CollectionId(9), &token).unwrap();
    let first = services.try_inner(9).unwrap().unwrap();
    assert_eq!(first.id, 9);
    assert!(matches!(
        services.try_inner(high),
        Err(SelectedAssetError::Registry(
            SimulationInnerRegistryError::KindMismatch {
                actual: SimulationResidentContainerAssetKind::Arena,
                ..
            }
        ))
    ));
    assert_eq!(services.retained_counts(), (2, 2));
    // The collection ID also collided with inner9; exact collection teardown
    // may not accidentally erase it as a collection or promote later losers.
    registry.remove_exact(CollectionId(9), &token).unwrap();
    assert!(services.try_inner(9).unwrap().is_none());
    assert!(services.try_inner(high).unwrap().is_none());
}

#[test]
fn service_lookup_is_weak_and_cannot_keep_registry_alive() {
    let (mut registry, _) = Registry::new(std::convert::identity);
    let services = SelectedSimulationAssetServices::construct_resident(&mut registry);
    assert!(services.try_inner(7).unwrap().is_none());
    drop(registry);
    assert!(matches!(
        services.try_inner(7),
        Err(SelectedAssetError::Registry(
            SimulationInnerRegistryError::Detached
        ))
    ));
}

struct FailSecond {
    services: Arc<SelectedSimulationAssetServices>,
    calls: std::sync::atomic::AtomicUsize,
}
impl SelectedArenaProcessor<SimulationResidentArenaAsset> for FailSecond {
    fn try_process(
        &self,
        asset: &Arc<SimulationResidentArenaAsset>,
        sink: &mut dyn SelectedAssetPublicationSink,
    ) -> Result<(), SelectedAssetError> {
        if self
            .calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            == 1
        {
            return Err(SelectedAssetError::Allocation);
        }
        self.services
            .try_process_plan_into(&asset.plan_owner(), sink)
    }
}

#[test]
fn later_failure_keeps_published_prefix_and_refuses_replay() {
    let (mut registry, _) = Registry::new(std::convert::identity);
    let services = SelectedSimulationAssetServices::construct_resident(&mut registry);
    registry.set_selected_processor(Arc::new(FailSecond {
        services: Arc::clone(&services),
        calls: std::sync::atomic::AtomicUsize::new(0),
    }));
    let assets = collection(&[9, 10]);
    let wrapper = registry.create_wrapper(CollectionId(100), &assets).unwrap();
    let token = Registry::stable_token(&wrapper);
    assert!(matches!(
        registry.publish(CollectionId(100), &token),
        Err(SimulationResidentRegistryError::SelectedProcessing(
            SelectedAssetError::Allocation
        ))
    ));
    let first = services.try_inner(9).unwrap().unwrap();
    assert!(services.try_inner(10).unwrap().is_none());
    assert_eq!(services.retained_counts(), (1, 1));
    assert!(matches!(
        registry.publish(CollectionId(100), &token),
        Err(SimulationResidentRegistryError::PendingSelectedPublicationFailure)
    ));
    assert!(Arc::ptr_eq(
        &first,
        &services.try_inner(9).unwrap().unwrap()
    ));
}
