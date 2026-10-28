//! Read-only collection dependency census using the production asset parsers.
#![allow(dead_code)]
#[path = "../crates/skate2-formats/src/big.rs"]
mod big;
#[path = "../crates/skate2-formats/src/mount.rs"]
mod mount;
#[path = "../crates/skate2-formats/src/refpack.rs"]
mod refpack;
#[path = "../crates/skate2-formats/src/stream.rs"]
mod stream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("game root required")?);
    let mut mounts = mount::MountSet::new();
    for spec in &mount::SKATE2_ARCHIVES {
        let path = spec.path_under(&root);
        if path.is_file() {
            mounts.push(mount::MountedBig::open(path, spec.use_external_header)?);
        }
    }
    for name in ["BAM_Pres", "BAM_Tex"] {
        let base = format!("data\\content\\world\\stream\\BAM\\{name}");
        let map = stream::CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
        let mut parents = std::collections::BTreeMap::new();
        for row in &map.collections {
            *parents.entry(row.parent_id).or_insert(0usize) += 1;
        }
        println!(
            "{name}: {} collections, parents={parents:016x?}",
            map.collections.len()
        );
        for parent in parents.keys() {
            if let Some(row) = map.collections.iter().find(|row| row.id == *parent) {
                println!(
                    "  parent {parent:016x}: {} assets={} offset={}",
                    row.filename, row.asset_count, row.payload_offset
                );
            }
        }
    }
    Ok(())
}
