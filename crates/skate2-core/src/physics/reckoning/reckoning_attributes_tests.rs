//! Controlled storage fixtures; original tuning is verified separately below.
use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node, set_dependency_bytes};

const CLASS: u64 = 0x7ce1_b6b5_8170_9c6f;
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
const DEAD_KEY: u64 = 0xda9e_8318_96b8_6ffc;
const WHEEL_KEY: u64 = 0xfae8_0499_e7d5_f3e4;
const REVERSE_KEY: u64 = 0xaf8a_5d55_853a_d035;

fn put(bytes: &mut [u8], offset: usize, word: u32) {
    bytes[offset..offset + 4].copy_from_slice(&word.to_be_bytes());
}

#[test]
fn captured_layout_bypasses_explicit_parent_nodes_and_survives_reader_drop() {
    let mut vault = graph(CLASS, vec![definition(REVERSE_KEY, 6, 4)]);
    collection(
        &mut vault,
        CLASS,
        7,
        0,
        None,
        vec![node(REVERSE_KEY, 0x40, 0x3f40_0000)],
    );
    collection(&mut vault, CLASS, DEFAULT, 7, Some(64), vec![]);
    let mut payload = vec![0; 64 + 0x4d0];
    put(&mut payload, 64, 0x8000_0000);
    put(&mut payload, 64 + 0x4c0, 0x7fc1_2345);
    set_dependency_bytes(&mut vault, 1, payload);
    let vault = Arc::new(vault);
    let mut reader = ReckoningVaultAttributes::bind(&vault);
    assert!(!std::ptr::eq(
        &raw const reader.own,
        &raw const reader.global
    ));
    let own = reader.capture_layout(ReckoningAttributeScope::Own).unwrap();
    let global = reader
        .capture_layout(ReckoningAttributeScope::Global)
        .unwrap();
    for scope in [
        ReckoningAttributeScope::Own,
        ReckoningAttributeScope::Global,
    ] {
        let pointer = reader.lookup(scope, REVERSE_KEY).unwrap();
        assert_eq!(reader.float(&pointer).unwrap().to_bits(), 0x3f40_0000);
    }
    assert_eq!(reader.layout_float(&own, 0).unwrap().to_bits(), 0x8000_0000);
    assert_eq!(
        reader.layout_float(&global, 0x4c0).unwrap().to_bits(),
        0x7fc1_2345
    );
    drop(reader);
    drop(vault);
    assert_eq!(own.word(0x4c0).unwrap(), 0x7fc1_2345_u32.to_be_bytes());
}

#[test]
fn generated_null_layout_defaults_even_for_found_collection_without_parent_substitution() {
    for found in [false, true] {
        let mut vault = graph(CLASS, vec![]);
        if found {
            collection(&mut vault, CLASS, 7, 0, Some(64), vec![]);
            collection(&mut vault, CLASS, DEFAULT, 7, None, vec![]);
        }
        let mut reader = ReckoningVaultAttributes::bind(&Arc::new(vault));
        for scope in [
            ReckoningAttributeScope::Own,
            ReckoningAttributeScope::Global,
        ] {
            let layout = reader.capture_layout(scope).unwrap();
            assert!(layout.0.is_none());
            for offset in [0, 0x110, 0x4a0, 0x4b0, 0x4c0, 0x4cc] {
                assert_eq!(reader.layout_float(&layout, offset).unwrap().to_bits(), 0);
            }
            assert_eq!(
                reader
                    .evaluate_graph8(&layout, 0x110, f32::NAN)
                    .unwrap()
                    .to_bits(),
                0
            );
        }
    }
}

#[test]
fn ordinary_dead_and_reversal_pointers_defer_size_checks_and_signed_words_keep_sign() {
    let mut vault = graph(
        CLASS,
        vec![
            definition(DEAD_KEY, 4, 1),
            definition(REVERSE_KEY, 4, 1),
            definition(WHEEL_KEY, 4, 4),
        ],
    );
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        None,
        vec![
            node(DEAD_KEY, 0x40, 0x7f00_0000),
            node(REVERSE_KEY, 0x40, 0x8000_0000),
            node(WHEEL_KEY, 0x40, 0xffff_fffd),
        ],
    );
    let mut reader = ReckoningVaultAttributes::bind(&Arc::new(vault));
    let dead = reader
        .lookup(ReckoningAttributeScope::Own, DEAD_KEY)
        .unwrap();
    let reversal = reader
        .lookup(ReckoningAttributeScope::Own, REVERSE_KEY)
        .unwrap();
    assert!(dead.is_some() && reversal.is_some());
    let wheels = reader
        .lookup(ReckoningAttributeScope::Own, WHEEL_KEY)
        .unwrap();
    assert_eq!(reader.signed_word(&wheels).unwrap(), -3);
    for pointer in [&dead, &reversal] {
        assert!(matches!(
            reader.float(pointer),
            Err(ReckoningAttributeError::Runtime(
                AttributeRuntimeError::ScalarSize { size: 1 }
            ))
        ));
    }
    let absent = reader.lookup(ReckoningAttributeScope::Global, 123).unwrap();
    assert!(absent.is_none());
    assert_eq!(reader.float(&absent).unwrap().to_bits(), 0);
    assert_eq!(reader.signed_word(&absent).unwrap(), 0);
}

#[test]
fn graph_uses_negative_graph_xy_subrange_and_source_fused_evaluation() {
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, DEFAULT, 0, Some(64), vec![]);
    let mut payload = vec![0; 64 + 0x4d0];
    for offset in [0x100, 0x104, 0x108, 0x10c] {
        put(&mut payload, 64 + offset, 0x7fc1_2345);
    }
    let xs = [0.0_f32, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    for (index, value) in xs.into_iter().enumerate() {
        put(&mut payload, 64 + 0x110 + index * 4, value.to_bits());
    }
    put(&mut payload, 64 + 0x130, 0xbf80_0000);
    put(&mut payload, 64 + 0x134, 0x3400_0000);
    set_dependency_bytes(&mut vault, 1, payload);
    let mut reader = ReckoningVaultAttributes::bind(&Arc::new(vault));
    let layout = reader.capture_layout(ReckoningAttributeScope::Own).unwrap();
    assert_eq!(
        reader
            .evaluate_graph8(&layout, 0x110, f32::from_bits(0x3f7f_fffe))
            .unwrap()
            .to_bits(),
        0xa880_0000
    );
    assert_eq!(
        reader.layout_float(&layout, 0x100).unwrap().to_bits(),
        0x7fc1_2345
    );
    for offset in [1, 0x4d0, usize::MAX] {
        assert!(matches!(
            reader.layout_float(&layout, offset),
            Err(ReckoningAttributeError::LayoutRange { .. })
        ));
    }
    assert!(matches!(
        reader.evaluate_graph8(&layout, 0x4a0, 0.0),
        Err(ReckoningAttributeError::LayoutRange { .. })
    ));
}

#[test]
fn malformed_nonnull_layout_or_unsupported_lookup_never_becomes_default() {
    let mut vault = graph(CLASS, vec![definition(DEAD_KEY, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        Some(64),
        vec![node(DEAD_KEY, 0x42, 0)],
    );
    // Existing fixture buffer is only256 bytes: its actual nonnull range cannot
    // hold the source1232-byte layout. Never replace this error with zeros.
    let mut reader = ReckoningVaultAttributes::bind(&Arc::new(vault));
    assert!(matches!(
        reader.capture_layout(ReckoningAttributeScope::Own),
        Err(ReckoningAttributeError::Runtime(_))
    ));
    assert!(matches!(
        reader.lookup(ReckoningAttributeScope::Own, DEAD_KEY),
        Err(ReckoningAttributeError::Runtime(
            AttributeRuntimeError::UnsupportedArray { .. }
        ))
    ));
}

#[path = "reckoning_attributes_corpus.rs"]
mod corpus;
