//! Original raw mounted Vault -> retained read graph -> generated physics layout.
//! Frozen words below are assertions, never runtime constructor inputs.

use crate::attribute_runtime::{AttributePhysicsLayoutOrigin, AttributeRuntimeVault};
use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
use std::{path::Path, sync::Arc};

const PHYSICS_CLASS: u64 = 0x0429_3d0f_2adc_fbb4;
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

fn byte_signature(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

pub(crate) fn mounted_physics_attributes() -> Arc<AttributeRuntimeVault> {
    mounted_physics_attributes_with_classes(&[])
}

pub(crate) fn mounted_physics_attributes_with_classes(
    additional_classes: &[u64],
) -> Arc<AttributeRuntimeVault> {
    let root = std::env::var("SKATE2_GAME_ROOT")
        .expect("set SKATE2_GAME_ROOT to the external original game-data directory");
    let archive = SKATE2_ARCHIVES
        .iter()
        .find(|archive| archive.relative_path == "data\\db\\db.big")
        .expect("source mount table contains the attribute database archive");
    let mut mounts = MountSet::new();
    mounts.push(
        MountedBig::open(
            archive.path_under(Path::new(&root)),
            archive.use_external_header,
        )
        .expect("open the original database BIG archive"),
    );
    let vlt = mounts.read_path("data\\db\\skater.vlt").unwrap();
    let bin = mounts.read_path("data\\db\\skater.bin").unwrap();
    assert_eq!(
        (vlt.len(), byte_signature(&vlt)),
        (851_568, 0xf7db_0ed5_27db_57b4)
    );
    assert_eq!(
        (bin.len(), byte_signature(&bin)),
        (812_520, 0x6b4f_c086_777f_e4df)
    );
    let mut selected_classes =
        crate::selected_original_asset_classes::BASE_PHYSICS_CLASSES.to_vec();
    selected_classes.extend_from_slice(additional_classes);
    AttributeRuntimeVault::try_load_mounted(&mounts, "data\\db\\skater.vlt", &selected_classes)
        .expect("source-backed selected physics/wheel read graph from original raw PtrN payloads")
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mounted_physics_instance_reads_original_cached_gravity_layout() {
    let vault = mounted_physics_attributes();
    let instance = vault.physics(DEFAULT_COLLECTION);
    let layout = instance.try_physics_layout().unwrap();
    assert_eq!(layout.origin, AttributePhysicsLayoutOrigin::CachedLayout);
    let gravity: [u32; 4] = std::array::from_fn(|index| {
        u32::from_be_bytes(
            layout.raw_bytes[index * 4..index * 4 + 4]
                .try_into()
                .unwrap(),
        )
    });
    assert_eq!(gravity, [0, 0xc11c_cccd, 0, 0]);
    // This shipped record has no explicit WorldGravity override. A separate
    // synthetic discriminator must establish that generated access bypasses
    // explicit nodes when generic FETCH and cached layout disagree.
    let ordinary = vault.instance(PHYSICS_CLASS, DEFAULT_COLLECTION);
    let node = ordinary
        .try_get_data(0x48b8_137e_1b32_a2e4, 0)
        .unwrap()
        .unwrap();
    assert_eq!(&node.bytes()[..16], &layout.raw_bytes[..16]);
    let missing = vault.physics(0);
    let default = missing.try_physics_layout().unwrap();
    assert_eq!(default.origin, AttributePhysicsLayoutOrigin::DefaultZero32);
    assert_eq!(default.raw_bytes, [0; 32]);
    // The instance retains the loaded storage independently of this handle.
    drop(vault);
    assert_eq!(
        instance.try_physics_layout().unwrap().raw_bytes,
        layout.raw_bytes
    );
    println!(
        "real raw db.big -> retained physics.default cached layout -> gravity words {gravity:08x?}; no generic-node substitution or live-edit claim"
    );
}
