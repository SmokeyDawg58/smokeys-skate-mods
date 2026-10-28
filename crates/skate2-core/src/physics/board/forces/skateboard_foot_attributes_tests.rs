use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node};

#[test]
fn original_class_hash_and_missing_global_defaults_are_exact() {
    assert_eq!(
        skate2_formats::attrib::string_hash64("physics_feet"),
        SkateboardFootVaultAttributes::CLASS
    );
    assert_eq!(
        skate2_formats::attrib::string_hash64("default"),
        SkateboardFootVaultAttributes::DEFAULT_COLLECTION
    );
    let vault = Arc::new(graph(SkateboardFootVaultAttributes::CLASS, vec![]));
    let mut reader = SkateboardFootVaultAttributes::bind(&vault);
    let absent = reader.lookup(FOOT_OFFSET).unwrap();
    assert_eq!(reader.float(&absent).unwrap().to_bits(), 0);
}

#[test]
fn explicit_parent_value_is_retained_and_foreign_pointer_cannot_substitute() {
    let class = SkateboardFootVaultAttributes::CLASS;
    let mut vault = graph(class, vec![definition(FOOT_OFFSET, 4, 4)]);
    collection(
        &mut vault,
        class,
        7,
        0,
        None,
        vec![node(FOOT_OFFSET, 0x40, 0x8000_0000)],
    );
    collection(
        &mut vault,
        class,
        SkateboardFootVaultAttributes::DEFAULT_COLLECTION,
        7,
        None,
        vec![],
    );
    let vault = Arc::new(vault);
    let mut first = SkateboardFootVaultAttributes::bind(&vault);
    let mut second = SkateboardFootVaultAttributes::bind(&vault);
    let pointer = first.lookup(FOOT_OFFSET).unwrap();
    let other = second.lookup(FOOT_OFFSET).unwrap();
    assert!(matches!(
        first.float(&other),
        Err(SkateboardFootAttributeError::ForeignPointer)
    ));
    drop(vault);
    drop(second);
    assert_eq!(first.float(&pointer).unwrap().to_bits(), 0x8000_0000);
}

#[test]
fn found_malformed_scalar_is_deferred_until_value_read_and_never_defaults() {
    let class = SkateboardFootVaultAttributes::CLASS;
    let mut vault = graph(
        class,
        vec![
            definition(SPEED_COEFFICIENT, 4, 1),
            definition(INITIAL_COEFFICIENT, 4, 4),
        ],
    );
    collection(
        &mut vault,
        class,
        SkateboardFootVaultAttributes::DEFAULT_COLLECTION,
        0,
        None,
        vec![
            node(SPEED_COEFFICIENT, 0x40, 0x7f00_0000),
            node(INITIAL_COEFFICIENT, 0x40, 0x7fc1_2345),
        ],
    );
    let mut reader = SkateboardFootVaultAttributes::bind(&Arc::new(vault));
    let speed = reader.lookup(SPEED_COEFFICIENT).unwrap();
    let initial = reader.lookup(INITIAL_COEFFICIENT).unwrap();
    assert!(matches!(
        reader.float(&speed),
        Err(SkateboardFootAttributeError::Runtime(
            AttributeRuntimeError::ScalarSize { size: 1 }
        ))
    ));
    assert_eq!(reader.float(&initial).unwrap().to_bits(), 0x7fc1_2345);
}

const ORIGINAL: [(u64, usize, u32); 5] = [
    (FOOT_OFFSET, 0x5c320, 0x3e61_47ae),
    (REAR_ABSORPTION_SCALE, 0x5c2f0, 0x3f00_0000),
    (LANDING_DENOMINATOR, 0x5c330, 0x3dcc_cccd),
    (SPEED_COEFFICIENT, 0x5c380, 0x41a0_0000),
    (INITIAL_COEFFICIENT, 0x5c360, 0x42c8_0000),
];

/// Original values are test assertions, never runtime provider defaults.
pub(crate) fn check_original(vault: &Arc<AttributeRuntimeVault>) -> [u32; 5] {
    let mut reader = SkateboardFootVaultAttributes::bind(vault);
    ORIGINAL.map(|(key, _, expected)| {
        let pointer = reader.lookup(key).unwrap();
        assert!(
            pointer.data.is_some(),
            "original scalar {key:016x} must exist"
        );
        let actual = reader.float(&pointer).unwrap().to_bits();
        assert_eq!(actual, expected, "original scalar {key:016x}");
        actual
    })
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mounted_original_feet_matches_independent_archive_inline_scalar_words() {
    let vault = crate::attribute_runtime_corpus::mounted_physics_attributes_with_classes(&[
        SkateboardFootVaultAttributes::CLASS,
    ]);
    let words = check_original(&vault);
    let root = std::env::var("SKATE2_GAME_ROOT").unwrap();
    let archive = skate2_formats::mount::SKATE2_ARCHIVES
        .iter()
        .find(|archive| archive.relative_path == "data\\db\\db.big")
        .unwrap();
    let bytes = std::fs::read(archive.path_under(std::path::Path::new(&root))).unwrap();
    // Independent original BIG table region: skater.vlt begins at 256. These
    // explicit nodes belong to physics_feet.default (no parent, 19 entries).
    for ((key, offset, _), word) in ORIGINAL.into_iter().zip(words) {
        let begin = 256 + offset;
        assert_eq!(&bytes[begin..begin + 8], &key.to_be_bytes());
        assert_eq!(&bytes[begin + 8..begin + 12], &word.to_be_bytes());
        assert_eq!(&bytes[begin + 12..begin + 16], &[0, 1, 0x40, 0]);
    }
}
