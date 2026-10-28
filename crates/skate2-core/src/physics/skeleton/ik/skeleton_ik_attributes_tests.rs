use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node, set_dependency_bytes};

const FIRST_VECTOR: u64 = 0xaa37_8f2e_bf7a_a313;
const SECOND_VECTOR: u64 = 0x7ce3_2881_4f93_09ae;
const FIRST_SCALAR: u64 = 0x39fa_8c07_e835_6284;
const SECOND_SCALAR: u64 = 0x094a_ce5d_e068_51fc;
const INITIAL_VECTOR: u64 = 0x121e_67e2_b409_6a41;

fn put_words(bytes: &mut [u8], offset: usize, words: [u32; 4]) {
    for (index, word) in words.into_iter().enumerate() {
        bytes[offset + 4 * index..offset + 4 * index + 4].copy_from_slice(&word.to_be_bytes());
    }
}

#[test]
fn active_keys_read_actual_default_scalars_and_full_vector_bits() {
    let mut definitions = vec![
        definition(FIRST_SCALAR, 4, 4),
        definition(SECOND_SCALAR, 4, 4),
    ];
    for (index, key) in [FIRST_VECTOR, SECOND_VECTOR, INITIAL_VECTOR]
        .into_iter()
        .enumerate()
    {
        let mut item = definition(key, 6, 16);
        item.offset = u16::try_from(index * 16).unwrap();
        definitions.push(item);
    }
    let mut vault = graph(CLASS, definitions);
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        Some(64),
        vec![
            node(FIRST_SCALAR, 0x40, 0x8000_0000),
            node(SECOND_SCALAR, 0x40, 0x7fc1_2345),
        ],
    );
    let words = [
        [0x3f80_0000, 0xbf80_0000, 0, 0x8000_0000],
        [0x7f80_0000, 0xff80_0000, 0x7fc1_2345, 0x0000_0001],
        [0x4000_0000, 0x4040_0000, 0x4080_0000, 0x40a0_0000],
    ];
    let mut payload = vec![0; 128];
    for (index, words) in words.into_iter().enumerate() {
        put_words(&mut payload, 64 + index * 16, words);
    }
    set_dependency_bytes(&mut vault, 1, payload);
    let mut provider = SkeletonIkVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(
        provider.scalar(FIRST_SCALAR).unwrap().to_bits(),
        0x8000_0000
    );
    assert_eq!(
        provider.scalar(SECOND_SCALAR).unwrap().to_bits(),
        0x7fc1_2345
    );
    for (key, words) in [FIRST_VECTOR, SECOND_VECTOR, INITIAL_VECTOR]
        .into_iter()
        .zip(words)
    {
        assert_eq!(provider.vector(key).unwrap().map(f32::to_bits), words);
    }
}

#[test]
fn null_binding_and_missing_attribute_have_zero_defaults_of_the_requested_size() {
    for with_collection in [false, true] {
        let mut vault = graph(CLASS, vec![]);
        if with_collection {
            collection(&mut vault, CLASS, DEFAULT, 0, None, vec![]);
        }
        let mut provider = SkeletonIkVaultAttributes::bind(&Arc::new(vault));
        assert_eq!(provider.scalar(FIRST_SCALAR).unwrap().to_bits(), 0);
        assert_eq!(
            provider.vector(FIRST_VECTOR).unwrap().map(f32::to_bits),
            [0; 4]
        );
    }
}

#[test]
fn own_default_instance_inherits_nodes_without_selecting_an_unrelated_collection() {
    let mut vault = graph(CLASS, vec![definition(FIRST_SCALAR, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        77,
        0,
        None,
        vec![node(FIRST_SCALAR, 0x40, 5.0_f32.to_bits())],
    );
    collection(
        &mut vault,
        CLASS,
        88,
        0,
        None,
        vec![node(FIRST_SCALAR, 0x40, 9.0_f32.to_bits())],
    );
    collection(&mut vault, CLASS, DEFAULT, 77, None, vec![]);
    let mut provider = SkeletonIkVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(
        provider.scalar(FIRST_SCALAR).unwrap().to_bits(),
        5.0_f32.to_bits()
    );
}

#[test]
fn found_null_node_stops_parent_search_and_uses_missing_pointer_fallback() {
    let mut vault = graph(CLASS, vec![definition(FIRST_SCALAR, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        77,
        0,
        None,
        vec![node(FIRST_SCALAR, 0x40, 5.0_f32.to_bits())],
    );
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        77,
        None,
        vec![node(FIRST_SCALAR, 0, 0)],
    );
    let mut provider = SkeletonIkVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(provider.scalar(FIRST_SCALAR).unwrap().to_bits(), 0);
}

#[test]
fn vector_layout_fallback_uses_original_child_layout() {
    let mut vault = graph(CLASS, vec![definition(FIRST_VECTOR, 6, 16)]);
    collection(&mut vault, CLASS, 77, 0, Some(64), vec![]);
    collection(&mut vault, CLASS, DEFAULT, 77, Some(128), vec![]);
    let mut payload = vec![0; 160];
    put_words(&mut payload, 64, [1; 4]);
    put_words(&mut payload, 128, [2, 3, 4, 5]);
    set_dependency_bytes(&mut vault, 1, payload);
    let mut provider = SkeletonIkVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(
        provider.vector(FIRST_VECTOR).unwrap().map(f32::to_bits),
        [2, 3, 4, 5]
    );
}

#[test]
fn malformed_present_payloads_remain_lazy_errors_and_do_not_poison_other_reads() {
    let mut vector = definition(FIRST_VECTOR, 6, 12);
    vector.offset = 0;
    let mut vault = graph(
        CLASS,
        vec![
            definition(FIRST_SCALAR, 4, 1),
            definition(SECOND_SCALAR, 4, 4),
            vector,
        ],
    );
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        Some(64),
        vec![
            node(FIRST_SCALAR, 0x40, 0x7f00_0000),
            node(SECOND_SCALAR, 0x40, 6.0_f32.to_bits()),
        ],
    );
    let mut provider = SkeletonIkVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(
        provider.scalar(SECOND_SCALAR).unwrap().to_bits(),
        6.0_f32.to_bits()
    );
    assert!(matches!(
        provider.scalar(FIRST_SCALAR),
        Err(AttributeRuntimeError::PayloadSize {
            key: FIRST_SCALAR,
            expected: 4,
            actual: 1
        })
    ));
    assert!(matches!(
        provider.vector(FIRST_VECTOR),
        Err(AttributeRuntimeError::PayloadSize {
            key: FIRST_VECTOR,
            expected: 16,
            actual: 12
        })
    ));
    assert_eq!(
        provider.vector(INITIAL_VECTOR).unwrap().map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(
        provider.scalar(SECOND_SCALAR).unwrap().to_bits(),
        6.0_f32.to_bits()
    );
}

#[test]
fn unsupported_present_node_is_not_an_absent_zero_value() {
    let mut vault = graph(CLASS, vec![definition(FIRST_SCALAR, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        None,
        vec![node(FIRST_SCALAR, 2, 0)],
    );
    let mut provider = SkeletonIkVaultAttributes::bind(&Arc::new(vault));
    assert!(matches!(
        provider.scalar(FIRST_SCALAR),
        Err(AttributeRuntimeError::UnsupportedArray { key: FIRST_SCALAR })
    ));
}

/// Optional hook for the actual owner corpus; no independent tuning constants.
pub(crate) fn check_original(vault: &Arc<AttributeRuntimeVault>) {
    let original = vault.instance(CLASS, DEFAULT);
    assert!(
        !original.is_null(),
        "original physics_skeletonik.default binding"
    );
    let mut provider = SkeletonIkVaultAttributes::bind(vault);
    for key in [FIRST_SCALAR, SECOND_SCALAR] {
        let data = original
            .try_get_data(key, 0)
            .unwrap()
            .expect("original IK scalar");
        let expected = u32::from_be_bytes(data.bytes().try_into().expect("original scalar word"));
        assert_eq!(provider.scalar(key).unwrap().to_bits(), expected);
    }
    for key in [FIRST_VECTOR, SECOND_VECTOR, INITIAL_VECTOR] {
        let data = original
            .try_get_data(key, 0)
            .unwrap()
            .expect("original IK vector");
        assert_eq!(data.bytes().len(), 16);
        let expected: [u32; 4] = std::array::from_fn(|index| {
            u32::from_be_bytes(data.bytes()[4 * index..4 * index + 4].try_into().unwrap())
        });
        assert_eq!(provider.vector(key).unwrap().map(f32::to_bits), expected);
    }
}
