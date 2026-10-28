//! Opt-in original mounted `skater.vlt/bin` to fresh wheel definitions.
//!
//! Owner inputs now pass through the shared retained Vault read graph and
//! wheel attribute bindings. The older test-local inline decoder is retained
//! only as an independent corpus oracle, not as the production fetch service.
//! Frozen words below are assertions only; mutable edit timing is not claimed.
//! Initial debug count zero and host reciprocal seeds are harness choices;
//! this is neither a native spawn nor a captured Xenon arithmetic trace.

use super::*;
use skate2_formats::{
    attrib::{
        AttributeExportKind, AttributePointerLocation, AttributePointerValue,
        AttributeVaultMetadata,
    },
    mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
};
use std::{collections::BTreeMap, path::Path};

fn byte_signature(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn mounted_database_bytes() -> (Vec<u8>, Vec<u8>) {
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
    // Independently measured original BIG4 payloads. External extracted copies
    // differ; their fingerprints are deliberately not used as archive oracles.
    assert_eq!(
        (vlt.len(), byte_signature(&vlt)),
        (851_568, 0xf7db_0ed5_27db_57b4)
    );
    assert_eq!(
        (bin.len(), byte_signature(&bin)),
        (812_520, 0x6b4f_c086_777f_e4df)
    );
    println!(
        "real mounted skater.vlt bytes={} fnv1a64={:016x}; skater.bin bytes={} fnv1a64={:016x}",
        vlt.len(),
        byte_signature(&vlt),
        bin.len(),
        byte_signature(&bin)
    );
    (vlt, bin)
}

fn mounted_tuning_words() -> [u32; 7] {
    let (vlt, bin) = mounted_database_bytes();
    let vault = AttributeVaultMetadata::parse(&vlt).unwrap();
    assert_eq!(vault.dependencies.len(), 1);
    assert_eq!(vault.dependencies[0].name, "skater.bin");
    assert_eq!(vault.dependencies[0].id, 16_762_145_128_419_181_554);
    assert!(vault.dependencies[0].is_asset);
    let pointers = vault.resolve_pointers(&[vlt.len(), bin.len()]).unwrap();
    let buffers = [vlt.as_slice(), bin.as_slice()];
    let mut classes = BTreeMap::new();
    let mut collections = BTreeMap::new();
    for export in &vault.exports {
        match export.kind() {
            AttributeExportKind::Class => {
                if SkateboardWheelAttribute::ALL
                    .iter()
                    .any(|attribute| attribute.class_key() == export.object_id)
                {
                    let class = export.class_load_data(&vlt).unwrap();
                    assert_eq!(class.class_id, export.object_id);
                    assert_eq!(class.layout_size, 0, "corpus has no class layout");
                    assert_eq!(class.layout_count, 0);
                    let definitions = export.class_definitions(&buffers, &pointers).unwrap();
                    assert!(
                        classes.insert(class.class_id, definitions).is_none(),
                        "unique selected class"
                    );
                }
            }
            AttributeExportKind::Collection => {
                let collection = export.collection_load_data(&vlt).unwrap();
                if collection.collection_id == SkateboardWheelAttribute::COLLECTION_KEY
                    && SkateboardWheelAttribute::ALL
                        .iter()
                        .any(|attribute| attribute.class_key() == collection.class_id)
                {
                    assert_eq!(
                        collection.parent_id, 0,
                        "no inheritance resolver needed for this corpus"
                    );
                    assert_eq!(collection.layout_pointer, 0, "explicit table values only");
                    assert!(
                        matches!(
                            pointers.get(&AttributePointerLocation {
                                dependency_index: 0,
                                offset: export.data_offset.checked_add(40).unwrap(),
                            }),
                            None | Some(AttributePointerValue::Null)
                        ),
                        "no non-null relocated layout pointer"
                    );
                    assert_eq!(
                        collection.entry_count,
                        if collection.class_id == SkateboardWheelAttribute::Mass.class_key() {
                            13
                        } else {
                            1
                        }
                    );
                    assert!(
                        collections
                            .insert(collection.class_id, collection)
                            .is_none(),
                        "unique selected class/default collection"
                    );
                }
            }
            _ => {}
        }
    }
    assert_eq!(classes.len(), 2);
    assert_eq!(collections.len(), 2);
    SkateboardWheelAttribute::ALL.map(|attribute| {
        let definitions = &classes[&attribute.class_key()];
        let mut matching = definitions.iter().filter(|definition| definition.key == attribute.key());
        let definition = matching.next().expect("selected attribute has a class definition");
        assert!(matching.next().is_none(), "unique definition key");
        assert_eq!(definition.type_id, 16_294_631_463_297_616_344);
        assert_eq!(definition.offset, 0);
        assert_eq!(definition.size, 4);
        assert_eq!(definition.maximum_count, 1);
        assert_eq!(definition.flags, 4);
        assert_eq!(definition.alignment, 2);
        assert_eq!(SkateboardWheelAttribute::ELEMENT_INDEX, 0);
        let collection = &collections[&attribute.class_key()];
        let mut matching = collection.entries.iter().filter(|entry| entry.key == attribute.key());
        let entry = matching.next().expect("actual shipped collection supplies the attribute");
        assert!(matching.next().is_none(), "unique explicit attribute key");
        assert_eq!(entry.type_index, 0);
        assert_eq!(collection.types[usize::from(entry.type_index)], definition.type_id);
        assert_eq!(entry.node_flags, 0x40);
        assert_eq!(entry.entry_flags, 0);
        let word = u32::from_be_bytes(entry.inline_value_word().expect("inline four-byte scalar"));
        println!(
            "real attribute={attribute:?} class={:016x} collection={:016x} key={:016x} index=0 word={word:08x}",
            attribute.class_key(), SkateboardWheelAttribute::COLLECTION_KEY, attribute.key()
        );
        word
    })
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mounted_skater_attributes_create_four_source_wheel_definitions() {
    let vault = crate::attribute_runtime_corpus::mounted_physics_attributes();
    let attributes = SkateboardWheelAttributes::bind(&vault);
    drop(vault);
    let snapshot = attributes.try_snapshot().unwrap();
    let actual_words = SkateboardWheelAttribute::ALL.map(|attribute| snapshot.word(attribute));
    assert_eq!(
        actual_words,
        mounted_tuning_words(),
        "independent old inline corpus oracle"
    );
    // Oracle only: never use this array to construct the snapshot or owner.
    assert_eq!(
        actual_words,
        [
            0,
            0x3cfd_f3b6,
            0x3f33_3333,
            0x3f4c_cccd,
            0,
            0x3da9_fbe7,
            0x40a0_0000
        ]
    );
    let definitions =
        SkateboardWheelDefinitions::try_create(snapshot, 0, &mut HostSkateboardWheelMassNumeric)
            .unwrap();
    for (attribute, word) in SkateboardWheelAttribute::ALL.into_iter().zip(actual_words) {
        assert_eq!(definitions.tuning().word(attribute), word);
        assert_eq!(
            definitions.tuning().origin(attribute),
            SkateboardWheelTuningOrigin::ResolvedAttribute
        );
    }
    assert_eq!(definitions.parts().len(), 4);
    assert_eq!(definitions.debug_end(), 4);
    for (index, part) in definitions.parts().iter().enumerate() {
        assert_eq!(usize::from(part.ordinal()), index);
        assert!(Arc::ptr_eq(part.volume(), definitions.parts()[0].volume()));
        assert_eq!(part.volume().volume_type(), Some(VolumeType::Sphere));
        assert_eq!(part.volume().radius.to_bits(), 0x3cfd_f3b6);
        assert_eq!(part.volume().group_id, 5);
        assert_eq!(part.volume().surface_id, 0);
        assert_eq!(part.volume().flags, 1);
        assert_eq!(
            part.volume().transform.map(f32::to_bits),
            FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
        );
        assert_eq!(part.material().static_friction.to_bits(), 0x3f4c_cccd);
        assert_eq!(part.material().dynamic_friction.to_bits(), 0x3f33_3333);
        assert_eq!(part.material().restitution.to_bits(), 0);
        assert_eq!(part.inertia().inverse_mass.to_bits(), 0x401a_3785);
        assert_eq!(part.inertia().max_velocity.to_bits(), 0x7f7f_ffff);
        assert_eq!(part.inertia().max_omega.to_bits(), 0x476a_5fff);
        assert_eq!(part.inertia().linear_drag.to_bits(), 0);
        assert_eq!(part.inertia().angular_drag.to_bits(), 0);
        assert!(
            part.inertia()
                .inverse_tensor
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
        );
        assert_eq!(
            part.inverse_tensor_w().to_bits(),
            part.inertia().inverse_tensor[0].to_bits()
        );
        assert!(part.inverse_body_transform().is_none());
        assert!(!part.is_static_body());
        assert_eq!(usize::try_from(part.debug().index).unwrap(), index);
        assert_eq!(part.debug().radius.to_bits(), 0x3cfd_f3b6);
    }
    println!(
        "real mounted tuning -> retained Vault instances and scalar FETCH -> immutable snapshot -> 4 ordered wheel definitions sharing 1 sphere; no runtime skateboard registration claimed"
    );
}
