use super::*;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use skate2_core::{
    collection_load_pump::LoadedAssetCollection,
    collection_resident_streamer::ResidentCollectionStorage,
    owned_resident_collection::{OwnedResidentCollectionError, OwnedResidentCollectionStorage},
};

struct TestProduct {
    id: CollectionId,
    drops: Arc<AtomicUsize>,
}

impl ResidentAssetIdentity for TestProduct {
    fn resident_asset_id(&self) -> CollectionId {
        self.id
    }
}

impl Drop for TestProduct {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Debug)]
struct TestBuildError;

impl fmt::Display for TestBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("injected construction failure")
    }
}

impl Error for TestBuildError {}

type TestStorage = OwnedResidentCollectionStorage<
    SharedLoadedArenaAsset<TestProduct>,
    fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
    OwnedResidentCollectionError,
>;

fn storage() -> TestStorage {
    TestStorage::new(std::convert::identity)
}

fn product(id: CollectionId, drops: &Arc<AtomicUsize>) -> TestProduct {
    TestProduct {
        id,
        drops: Arc::clone(drops),
    }
}

#[test]
fn two_resident_collections_share_one_processed_allocation_until_both_unload() {
    let id = CollectionId(0x1234_5678_dead_beef);
    let drops = Arc::new(AtomicUsize::new(0));
    let constructions = AtomicUsize::new(0);
    let cache = SharedLoadedArenaCache::new();
    let mut storage = storage();
    let first = storage.create_empty(1).unwrap();
    let second = storage.create_empty(1).unwrap();
    let mut first_writer = storage.writer(&first).unwrap();
    let mut second_writer = storage.writer(&second).unwrap();

    let first_occurrence = cache
        .acquire_with(id, || -> Result<_, TestBuildError> {
            constructions.fetch_add(1, Ordering::SeqCst);
            Ok(Some(product(id, &drops)))
        })
        .unwrap()
        .unwrap();
    let weak = first_occurrence.weak_product();
    let second_occurrence = cache
        .acquire_with(id, || -> Result<_, TestBuildError> {
            panic!("same full asset ID must bypass decode and GPU upload")
        })
        .unwrap()
        .unwrap();
    assert!(first_occurrence.same_product(&second_occurrence));
    let first_use_owner = first_occurrence.retained_product();
    let second_use_owner = second_occurrence.retained_product();
    assert!(Arc::ptr_eq(&first_use_owner, &second_use_owner));
    first_writer.add_asset(first_occurrence);
    second_writer.add_asset(second_occurrence);
    assert_eq!(storage.asset_ids_in_order(&first).unwrap(), [id]);
    assert_eq!(storage.asset_ids_in_order(&second).unwrap(), [id]);
    assert_eq!(cache.physical_loaded_uses(id).unwrap(), Some(2));
    assert_eq!(constructions.load(Ordering::SeqCst), 1);

    storage.destroy(first);
    assert_eq!(cache.physical_loaded_uses(id).unwrap(), Some(1));
    assert!(weak.upgrade().is_some());
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(first_use_owner);
    drop(second_use_owner);
    storage.destroy(second);
    assert_eq!(cache.physical_loaded_uses(id).unwrap(), None);
    assert!(weak.upgrade().is_none());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn full_width_ids_with_equal_low_words_have_distinct_products() {
    let low = CollectionId(0x1111_2222_dead_beef);
    let high = CollectionId(0x9999_aaaa_dead_beef);
    let drops = Arc::new(AtomicUsize::new(0));
    let cache = SharedLoadedArenaCache::new();
    let one = cache
        .acquire_with(low, || -> Result<_, TestBuildError> {
            Ok(Some(product(low, &drops)))
        })
        .unwrap()
        .unwrap();
    let two = cache
        .acquire_with(high, || -> Result<_, TestBuildError> {
            Ok(Some(product(high, &drops)))
        })
        .unwrap()
        .unwrap();
    assert!(!one.same_product(&two));
    assert_eq!(cache.physical_loaded_uses(low).unwrap(), Some(1));
    assert_eq!(cache.physical_loaded_uses(high).unwrap(), Some(1));
    drop(one);
    assert_eq!(cache.physical_loaded_uses(low).unwrap(), None);
    assert_eq!(cache.physical_loaded_uses(high).unwrap(), Some(1));
    drop(two);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn failed_second_construction_keeps_prefix_and_allows_direct_cache_retry() {
    let first_id = CollectionId(1);
    let failed_id = CollectionId(2);
    let drops = Arc::new(AtomicUsize::new(0));
    let cache = SharedLoadedArenaCache::new();
    let mut storage = storage();
    let collection = storage.create_empty(2).unwrap();
    let mut writer = storage.writer(&collection).unwrap();
    let first = cache
        .acquire_with(first_id, || -> Result<_, TestBuildError> {
            Ok(Some(product(first_id, &drops)))
        })
        .unwrap()
        .unwrap();
    writer.add_asset(first);

    let failure = cache.acquire_with(failed_id, || -> Result<_, TestBuildError> {
        Err(TestBuildError)
    });
    assert!(matches!(
        failure,
        Err(SharedLoadedArenaBuildError::Source(_))
    ));
    assert_eq!(storage.asset_ids_in_order(&collection).unwrap(), [first_id]);
    assert_eq!(cache.physical_loaded_uses(first_id).unwrap(), Some(1));
    assert_eq!(cache.physical_loaded_uses(failed_id).unwrap(), None);

    let retry = cache
        .acquire_with(failed_id, || -> Result<_, TestBuildError> {
            Ok(Some(product(failed_id, &drops)))
        })
        .unwrap()
        .unwrap();
    writer.add_asset(retry);
    assert_eq!(
        storage.asset_ids_in_order(&collection).unwrap(),
        [first_id, failed_id]
    );
    storage.destroy(collection);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

struct BlockingTeardownProduct {
    id: CollectionId,
    teardown_started: Option<mpsc::Sender<()>>,
    allow_teardown: Mutex<Option<mpsc::Receiver<()>>>,
}

impl ResidentAssetIdentity for BlockingTeardownProduct {
    fn resident_asset_id(&self) -> CollectionId {
        self.id
    }
}

impl Drop for BlockingTeardownProduct {
    fn drop(&mut self) {
        if let Some(started) = self.teardown_started.take() {
            started.send(()).unwrap();
            self.allow_teardown
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .recv()
                .unwrap();
        }
    }
}

#[test]
fn final_teardown_serializes_same_id_acquire_before_map_erasure() {
    let id = CollectionId(0x1234_5678_dead_beef);
    let cache = SharedLoadedArenaCache::new();
    let (teardown_started_tx, teardown_started_rx) = mpsc::channel();
    let (allow_teardown_tx, allow_teardown_rx) = mpsc::channel();
    let first = cache
        .acquire_with(id, || -> Result<_, TestBuildError> {
            Ok(Some(BlockingTeardownProduct {
                id,
                teardown_started: Some(teardown_started_tx),
                allow_teardown: Mutex::new(Some(allow_teardown_rx)),
            }))
        })
        .unwrap()
        .unwrap();

    let final_release = thread::spawn(move || drop(first));
    teardown_started_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    let map_lock_held_during_teardown = matches!(
        cache.entries.try_lock(),
        Err(std::sync::TryLockError::WouldBlock)
    );

    let next_cache = cache.clone();
    let (acquire_started_tx, acquire_started_rx) = mpsc::channel();
    let (construction_started_tx, construction_started_rx) = mpsc::channel();
    let next_acquire = thread::spawn(move || {
        acquire_started_tx.send(()).unwrap();
        next_cache
            .acquire_with(id, || -> Result<_, TestBuildError> {
                construction_started_tx.send(()).unwrap();
                Ok(Some(BlockingTeardownProduct {
                    id,
                    teardown_started: None,
                    allow_teardown: Mutex::new(None),
                }))
            })
            .unwrap()
            .unwrap()
    });
    acquire_started_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    let constructed_during_teardown = construction_started_rx
        .recv_timeout(Duration::from_millis(100))
        .is_ok();
    allow_teardown_tx.send(()).unwrap();
    final_release.join().unwrap();
    let second = next_acquire.join().unwrap();
    let constructed_after_teardown = construction_started_rx.try_recv().is_ok();
    assert!(map_lock_held_during_teardown);
    assert!(
        !constructed_during_teardown,
        "same-ID construction began before the previous product finished teardown"
    );
    assert!(
        constructed_after_teardown,
        "replacement was not constructed"
    );
    assert_eq!(cache.physical_loaded_uses(id).unwrap(), Some(1));
    drop(second);
    assert_eq!(cache.physical_loaded_uses(id).unwrap(), None);
}

#[test]
fn wrong_builder_identity_never_enters_loaded_map() {
    let requested = CollectionId(10);
    let wrong = CollectionId(11);
    let drops = Arc::new(AtomicUsize::new(0));
    let cache = SharedLoadedArenaCache::new();
    let result = cache.acquire_with(requested, || -> Result<_, TestBuildError> {
        Ok(Some(product(wrong, &drops)))
    });
    assert!(matches!(
        result,
        Err(SharedLoadedArenaBuildError::IdentityMismatch {
            requested: CollectionId(10),
            built: CollectionId(11)
        })
    ));
    assert_eq!(cache.physical_loaded_uses(requested).unwrap(), None);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
