//! Small read-only inspector for `AttribSys` VLT metadata during research.

use std::{collections::BTreeMap, env, fs, path::PathBuf, process::ExitCode};

use skate2_formats::attrib::{
    AttributeExportKind, AttributePointerLocation, AttributePointerValue, AttributeVaultMetadata,
    string_hash64,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("pass a .vlt path")?;
    let class_name = env::args()
        .nth(2)
        .unwrap_or_else(|| "physics_jump".to_owned());
    let collection_name = env::args().nth(3).unwrap_or_else(|| "default".to_owned());
    let bytes = fs::read(&path)?;
    let vault = AttributeVaultMetadata::parse(&bytes)?;
    let parent = path.parent().ok_or("VLT path has no parent directory")?;
    let dependency_bytes = vault
        .dependencies
        .iter()
        .map(|dependency| fs::read(parent.join(&dependency.name)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut buffers = vec![bytes.as_slice()];
    buffers.extend(dependency_bytes.iter().map(Vec::as_slice));
    let sizes = buffers
        .iter()
        .map(|buffer| buffer.len())
        .collect::<Vec<_>>();
    let pointers = vault.resolve_pointers(&sizes)?;
    let class_id = string_hash64(&class_name);
    let collection_id = string_hash64(&collection_name);
    println!("file: {}", path.display());
    println!("dependencies: {:?}", vault.dependencies);
    println!("exports: {}", vault.exports.len());
    let class = vault
        .exports
        .iter()
        .find(|export| export.kind() == AttributeExportKind::Class && export.object_id == class_id);
    println!("class {class_name} ({class_id:016x}): {class:?}");
    let definitions = class
        .map(|export| export.class_definitions(&buffers, &pointers))
        .transpose()?
        .unwrap_or_default();
    println!("definitions: {definitions:#?}");
    let mut matching_collections = 0;
    for export in &vault.exports {
        if export.kind() != AttributeExportKind::Collection {
            continue;
        }
        let collection = export.collection_load_data(&bytes)?;
        if collection.class_id != class_id {
            continue;
        }
        matching_collections += 1;
        let marker = if collection.collection_id == collection_id {
            " requested"
        } else {
            ""
        };
        println!(
            "collection {:016x}{marker}: parent {:016x}, {} explicit entries:",
            collection.collection_id,
            collection.parent_id,
            collection.entries.len(),
        );
        if collection.collection_id == collection_id {
            print_requested_layout(export.data_offset, &buffers, &pointers);
        }
        let entries_start = export.data_offset + 48 + u32::from(collection.types_length) * 8;
        for (index, entry) in collection.entries.iter().enumerate() {
            let pointer_slot = entries_start + u32::try_from(index)? * 16 + 8;
            let pointer = pointers.get(&AttributePointerLocation {
                dependency_index: 0,
                offset: pointer_slot,
            });
            let definition = definitions
                .iter()
                .find(|definition| definition.key == entry.key);
            println!(
                "  {:016x}: raw={:08x} type={} node={:02x} entry={:02x} pointer={pointer:?} definition={definition:?}",
                entry.key,
                entry.data_pointer,
                entry.type_index,
                entry.node_flags,
                entry.entry_flags,
            );
        }
    }
    println!("matching collections: {matching_collections}");
    Ok(())
}

fn print_requested_layout(
    export_data_offset: u32,
    buffers: &[&[u8]],
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
) {
    let layout_slot = AttributePointerLocation {
        dependency_index: 0,
        offset: export_data_offset + 40,
    };
    let layout_bytes = match pointers.get(&layout_slot) {
        Some(AttributePointerValue::Direct {
            dependency_index,
            offset,
        }) => buffers
            .get(usize::from(*dependency_index))
            .and_then(|buffer| {
                usize::try_from(*offset)
                    .ok()
                    .and_then(|start| buffer.get(start..start + 40))
            }),
        _ => None,
    };
    println!(
        "  layout pointer={:?}, bytes={layout_bytes:02x?}",
        pointers.get(&layout_slot)
    );
}
