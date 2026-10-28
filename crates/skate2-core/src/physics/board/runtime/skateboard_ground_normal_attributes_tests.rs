use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node};
fn vault(bits: u32, size: u16) -> Arc<AttributeRuntimeVault> {
    let class = SkateboardGroundNormalVaultAttributes::CLASS;
    let mut vault = graph(class, vec![definition(GROUND_NORMAL_ANGLE, 4, size)]);
    collection(
        &mut vault,
        class,
        7,
        0,
        None,
        vec![node(GROUND_NORMAL_ANGLE, 0x40, bits)],
    );
    collection(
        &mut vault,
        class,
        SkateboardGroundNormalVaultAttributes::DEFAULT_COLLECTION,
        7,
        None,
        vec![],
    );
    Arc::new(vault)
}
#[test]
fn exact_class_hash_missing_lookup_uses_only_source_default_area() {
    assert_eq!(
        skate2_formats::attrib::string_hash64("physics_reckoning"),
        SkateboardGroundNormalVaultAttributes::CLASS
    );
    assert_eq!(
        skate2_formats::attrib::string_hash64("default"),
        SkateboardGroundNormalVaultAttributes::DEFAULT_COLLECTION
    );
    let vault = Arc::new(graph(SkateboardGroundNormalVaultAttributes::CLASS, vec![]));
    let mut reader = SkateboardGroundNormalVaultAttributes::bind(&vault);
    let global = reader.capture_global().unwrap();
    let pointer = reader.lookup(&global, GROUND_NORMAL_ANGLE).unwrap();
    assert!(pointer.data.is_none());
    assert_eq!(reader.float(&pointer).unwrap().to_bits(), 0);
}
#[test]
fn inherited_runtime_value_and_retained_capture_survive_external_vault_drop() {
    for bits in [0x8000_0000, 0x7fc1_2345, 0x4234_0000] {
        let vault = vault(bits, 4);
        let mut reader = SkateboardGroundNormalVaultAttributes::bind(&vault);
        let global = reader.capture_global().unwrap();
        drop(vault);
        let pointer = reader.lookup(&global, GROUND_NORMAL_ANGLE).unwrap();
        drop(global);
        assert_eq!(reader.float(&pointer).unwrap().to_bits(), bits);
    }
}
#[test]
fn equal_value_other_instance_global_and_pointer_are_rejected() {
    let vault = vault(0x4234_0000, 4);
    let mut first = SkateboardGroundNormalVaultAttributes::bind(&vault);
    let mut second = SkateboardGroundNormalVaultAttributes::bind(&vault);
    let global = second.capture_global().unwrap();
    let pointer = second.lookup(&global, GROUND_NORMAL_ANGLE).unwrap();
    assert!(matches!(
        first.lookup(&global, GROUND_NORMAL_ANGLE),
        Err(SkateboardGroundNormalAttributeError::ForeignGlobal)
    ));
    assert!(matches!(
        first.float(&pointer),
        Err(SkateboardGroundNormalAttributeError::ForeignPointer)
    ));
    let own = first.capture_global().unwrap();
    let own_pointer = first.lookup(&own, GROUND_NORMAL_ANGLE).unwrap();
    assert_eq!(
        first.float(&own_pointer).unwrap().to_bits(),
        second.float(&pointer).unwrap().to_bits()
    );
}
#[test]
fn malformed_scalar_is_not_decoded_by_capture_or_lookup_and_does_not_default() {
    let mut reader = SkateboardGroundNormalVaultAttributes::bind(&vault(0x7f00_0000, 1));
    let global = reader.capture_global().unwrap();
    let pointer = reader.lookup(&global, GROUND_NORMAL_ANGLE).unwrap();
    assert!(matches!(
        reader.float(&pointer),
        Err(SkateboardGroundNormalAttributeError::Runtime(
            AttributeRuntimeError::ScalarSize { size: 1 }
        ))
    ));
}

/// Independent original scalar word, not a runtime tuning constant.
pub(crate) fn check_original(vault: &Arc<AttributeRuntimeVault>) -> u32 {
    let mut reader = SkateboardGroundNormalVaultAttributes::bind(vault);
    let global = reader.capture_global().unwrap();
    let pointer = reader.lookup(&global, GROUND_NORMAL_ANGLE).unwrap();
    assert!(pointer.data.is_some());
    let actual = reader.float(&pointer).unwrap().to_bits();
    assert_eq!(actual, 0x42a0_0000);
    actual
}
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mounted_original_global_angle_matches_independent_archive_node() {
    let vault = crate::attribute_runtime_corpus::mounted_physics_attributes_with_classes(&[
        SkateboardGroundNormalVaultAttributes::CLASS,
    ]);
    let actual = check_original(&vault);
    let root = std::env::var("SKATE2_GAME_ROOT").unwrap();
    let archive = skate2_formats::mount::SKATE2_ARCHIVES
        .iter()
        .find(|archive| archive.relative_path == "data\\db\\db.big")
        .unwrap();
    let bytes = std::fs::read(archive.path_under(std::path::Path::new(&root))).unwrap();
    let node = 256 + 0x5d6f8;
    assert_eq!(&bytes[node..node + 8], &GROUND_NORMAL_ANGLE.to_be_bytes());
    assert_eq!(&bytes[node + 8..node + 12], &actual.to_be_bytes());
    assert_eq!(&bytes[node + 12..node + 16], &[0, 4, 0x40, 0]);
}
