use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node};

#[test]
fn original_class_hash_and_missing_global_defaults_are_exact() {
    assert_eq!(
        skate2_formats::attrib::string_hash64("physics_brakes"),
        SkateboardBrakeVaultAttributes::CLASS
    );
    assert_eq!(
        skate2_formats::attrib::string_hash64("default"),
        SkateboardBrakeVaultAttributes::DEFAULT_COLLECTION
    );
    let vault = Arc::new(graph(SkateboardBrakeVaultAttributes::CLASS, vec![]));
    let mut reader = SkateboardBrakeVaultAttributes::bind(&vault);
    let absent = reader.lookup(FOOT_COEFFICIENT).unwrap();
    assert_eq!(reader.float(&absent).unwrap().to_bits(), 0);
}

#[test]
fn explicit_parent_value_is_retained_and_foreign_pointer_cannot_substitute() {
    let class = SkateboardBrakeVaultAttributes::CLASS;
    let mut vault = graph(class, vec![definition(FOOT_COEFFICIENT, 4, 4)]);
    collection(
        &mut vault,
        class,
        7,
        0,
        None,
        vec![node(FOOT_COEFFICIENT, 0x40, 0x8000_0000)],
    );
    collection(
        &mut vault,
        class,
        SkateboardBrakeVaultAttributes::DEFAULT_COLLECTION,
        7,
        None,
        vec![],
    );
    let vault = Arc::new(vault);
    let mut first = SkateboardBrakeVaultAttributes::bind(&vault);
    let mut second = SkateboardBrakeVaultAttributes::bind(&vault);
    let pointer = first.lookup(FOOT_COEFFICIENT).unwrap();
    let other = second.lookup(FOOT_COEFFICIENT).unwrap();
    assert!(matches!(
        first.float(&other),
        Err(SkateboardBrakeAttributeError::ForeignPointer)
    ));
    drop(vault);
    drop(second);
    assert_eq!(first.float(&pointer).unwrap().to_bits(), 0x8000_0000);
}

#[test]
fn found_malformed_scalar_is_deferred_until_value_read_and_never_defaults() {
    let class = SkateboardBrakeVaultAttributes::CLASS;
    let mut vault = graph(
        class,
        vec![
            definition(STOP_SPEED, 4, 1),
            definition(MANUAL_COEFFICIENT, 4, 4),
        ],
    );
    collection(
        &mut vault,
        class,
        SkateboardBrakeVaultAttributes::DEFAULT_COLLECTION,
        0,
        None,
        vec![
            node(STOP_SPEED, 0x40, 0x7f00_0000),
            node(MANUAL_COEFFICIENT, 0x40, 0x7fc1_2345),
        ],
    );
    let mut reader = SkateboardBrakeVaultAttributes::bind(&Arc::new(vault));
    let speed = reader.lookup(STOP_SPEED).unwrap();
    let initial = reader.lookup(MANUAL_COEFFICIENT).unwrap();
    assert!(matches!(
        reader.float(&speed),
        Err(SkateboardBrakeAttributeError::Runtime(
            AttributeRuntimeError::ScalarSize { size: 1 }
        ))
    ));
    assert_eq!(reader.float(&initial).unwrap().to_bits(), 0x7fc1_2345);
}

/// Independently inspected original default nodes, for the existing corpus.
pub(crate) fn check_original(vault: &Arc<AttributeRuntimeVault>) -> [u32; 6] {
    let mut reader = SkateboardBrakeVaultAttributes::bind(vault);
    [
        (FOOT_COEFFICIENT, 0x4248_0000),
        (MANUAL_COEFFICIENT, 0x420c_0000),
        (STOP_SPEED, 0x3c23_d70a),
        (BALANCE_SPEED, 0x3f80_0000),
        (NORMAL_Y_THRESHOLD, 0x3f6b_851f),
        (BALANCE_DRAG, 0x3c23_d70a),
    ]
    .map(|(key, expected)| {
        let pointer = reader.lookup(key).unwrap();
        assert!(pointer.data.is_some());
        let actual = reader.float(&pointer).unwrap().to_bits();
        assert_eq!(actual, expected, "original brake attribute {key:016x}");
        actual
    })
}
