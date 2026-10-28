use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node};

const GLOBAL_DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;

#[test]
fn own_collection_inherits_its_parent_and_never_selects_global_default() {
    let mut vault = graph(CLASS, vec![definition(LOCAL_Y_OFFSET, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        GLOBAL_DEFAULT,
        0,
        None,
        vec![node(LOCAL_Y_OFFSET, 0x40, 99.0_f32.to_bits())],
    );
    collection(
        &mut vault,
        CLASS,
        77,
        0,
        None,
        vec![node(LOCAL_Y_OFFSET, 0x40, (-0.25_f32).to_bits())],
    );
    collection(&mut vault, CLASS, OWN_COLLECTION, 77, None, vec![]);
    let mut attributes = SkateboardQueuedForceVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(
        attributes.local_y_offset().unwrap().to_bits(),
        (-0.25_f32).to_bits()
    );
    attributes.finish_lookup().unwrap();
}

#[test]
fn missing_own_collection_or_key_uses_zero_even_when_global_has_a_value() {
    for own_exists in [false, true] {
        let mut vault = graph(CLASS, vec![definition(LOCAL_Y_OFFSET, 4, 4)]);
        collection(
            &mut vault,
            CLASS,
            GLOBAL_DEFAULT,
            0,
            None,
            vec![node(LOCAL_Y_OFFSET, 0x40, 9.0_f32.to_bits())],
        );
        if own_exists {
            collection(&mut vault, CLASS, OWN_COLLECTION, 0, None, vec![]);
        }
        let mut attributes = SkateboardQueuedForceVaultAttributes::bind(&Arc::new(vault));
        assert_eq!(attributes.local_y_offset().unwrap().to_bits(), 0);
        attributes.finish_lookup().unwrap();
    }
}

#[test]
fn scalar_payload_bits_survive_and_malformed_scalar_is_not_defaulted() {
    for bits in [0x8000_0000, 0x7fc1_2345] {
        let mut vault = graph(CLASS, vec![definition(LOCAL_Y_OFFSET, 4, 4)]);
        collection(
            &mut vault,
            CLASS,
            OWN_COLLECTION,
            0,
            None,
            vec![node(LOCAL_Y_OFFSET, 0x40, bits)],
        );
        let mut attributes = SkateboardQueuedForceVaultAttributes::bind(&Arc::new(vault));
        assert_eq!(attributes.local_y_offset().unwrap().to_bits(), bits);
    }
    let mut vault = graph(CLASS, vec![definition(LOCAL_Y_OFFSET, 4, 2)]);
    collection(
        &mut vault,
        CLASS,
        OWN_COLLECTION,
        0,
        None,
        vec![node(LOCAL_Y_OFFSET, 0x40, 0xffff_ffff)],
    );
    let mut attributes = SkateboardQueuedForceVaultAttributes::bind(&Arc::new(vault));
    assert!(matches!(
        attributes.local_y_offset(),
        Err(AttributeRuntimeError::ScalarSize { size: 2 })
    ));
}

#[test]
fn final_lookup_does_not_decode_value_or_require_byte_shape_but_keeps_lookup_errors() {
    for size in [0, 2, 4] {
        let mut vault = graph(CLASS, vec![definition(FINISH_LOOKUP, 4, size)]);
        collection(
            &mut vault,
            CLASS,
            OWN_COLLECTION,
            0,
            None,
            vec![node(FINISH_LOOKUP, 0x40, 0xffff_ffff)],
        );
        let mut attributes = SkateboardQueuedForceVaultAttributes::bind(&Arc::new(vault));
        attributes.finish_lookup().unwrap();
    }
    let mut vault = graph(CLASS, vec![definition(FINISH_LOOKUP, 4, 1)]);
    collection(
        &mut vault,
        CLASS,
        OWN_COLLECTION,
        0,
        None,
        vec![node(FINISH_LOOKUP, 0x42, 0)],
    );
    let mut attributes = SkateboardQueuedForceVaultAttributes::bind(&Arc::new(vault));
    assert!(matches!(
        attributes.finish_lookup(),
        Err(AttributeRuntimeError::UnsupportedArray { key: FINISH_LOOKUP })
    ));
}
