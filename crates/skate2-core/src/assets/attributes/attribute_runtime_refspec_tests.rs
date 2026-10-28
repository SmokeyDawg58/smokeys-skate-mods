use super::*;
use crate::attribute_runtime::{
    AttributeRuntimeVault,
    tests::{collection, graph, set_dependency_bytes},
};
use std::sync::Arc;

const CLASS: u64 = 0x1122_3344_0000_0000;
const CHILD: u64 = 9;
const TARGET: u64 = 0xaabb_ccdd_0000_0000;

fn fixture(collection_key: u64, pointer: u32) -> Arc<AttributeRuntimeVault> {
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, CHILD, 0, Some(0), vec![]);
    collection(&mut vault, CLASS, TARGET, 0, None, vec![]);
    let mut bytes = vec![0; 256];
    bytes[0..8].copy_from_slice(&CLASS.to_be_bytes());
    bytes[8..16].copy_from_slice(&collection_key.to_be_bytes());
    bytes[16..20].copy_from_slice(&pointer.to_be_bytes());
    set_dependency_bytes(&mut vault, 1, bytes);
    Arc::new(vault)
}

#[test]
fn full64_keys_and_same_layout_cache_are_shared_between_fresh_instances() {
    let vault = fixture(TARGET, 0);
    let first = vault.instance(CLASS, CHILD);
    let second = vault.instance(CLASS, CHILD);
    let result = first.try_cached_layout_reference(0).unwrap().unwrap();
    assert_eq!(result.collection, Some((CLASS, TARGET)));
    assert!(Arc::ptr_eq(&result.vault, &vault));
    assert_eq!(vault.refspec_cache.0.lock().unwrap().len(), 1);
    assert_eq!(
        second
            .try_cached_layout_reference(0)
            .unwrap()
            .unwrap()
            .collection,
        result.collection
    );
    assert_eq!(vault.refspec_cache.0.lock().unwrap().len(), 1);
}

#[test]
fn null_key_skips_unadmitted_pointer_and_keeps_cache_unmodified() {
    let vault = fixture(0, 0x1234_5678);
    assert!(
        vault
            .instance(CLASS, CHILD)
            .try_cached_layout_reference(0)
            .unwrap()
            .is_none()
    );
    assert!(vault.refspec_cache.0.lock().unwrap().is_empty());
}

#[test]
fn nonnull_authored_pointer_stops_before_any_cache_publication() {
    let vault = fixture(TARGET, 0x1234_5678);
    assert!(matches!(
        vault.instance(CLASS, CHILD).try_cached_layout_reference(0),
        Err(RefSpecError::InitialCachedPointer { .. })
    ));
    assert!(vault.refspec_cache.0.lock().unwrap().is_empty());
}

#[test]
fn absent_target_publishes_null_and_remains_retryable() {
    let vault = fixture(TARGET + 1, 0);
    let instance = vault.instance(CLASS, CHILD);
    assert!(instance.try_cached_layout_reference(0).unwrap().is_none());
    assert_eq!(
        vault
            .refspec_cache
            .0
            .lock()
            .unwrap()
            .values()
            .copied()
            .collect::<Vec<_>>(),
        vec![None]
    );
    assert!(instance.try_cached_layout_reference(0).unwrap().is_none());
}
