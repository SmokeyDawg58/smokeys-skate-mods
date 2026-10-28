//! Read-only census of every DMO (dynamic movable object) placement in the shipped game data,
//! using the production asset parsers via `#[path]` includes (same approach as
//! `inspect_city_collections.rs`).
//!
//! Walks every `.xsm` collection map in every mounted (named) archive except `_Tex` streams,
//! decodes each collection asset's arena, and parses every kind-31 `tDMOData`
//! (`0x00eb001d`) object in the arena dictionary. That is the same object the runtime's
//! `RegDMOArena` pass hands to `ProcessDMOData`, without the TOC routing.
//!
//! It also scans the main DMO bank (`data\content\world\dmo\DMO\DMO_Pres.xsm`,
//! `cPres_Global.xsf`) for `tInstanceData` (`0x00eb000d`) records. A locator's
//! `instance_guid` is the model ID that `ModelMan` looks up, and with
//! `kEnableDMOBankIDLookup` the bank model ID is the bank instance's `local_guid`
//! (`GetSimModelIDFromInstanceID` 0x82d2ee20). A model "has grinds" when its
//! `m_pEmbedded` `tEmbeddedData` has a nonempty grind-spline group (group 1).
//!
//! Build: rustc --edition 2024 -O tools/list_dmo_props.rs -o target/list_dmo_props.exe
//! Run:   target/list_dmo_props.exe "<game root containing data\big>" [--sources]
#![allow(dead_code, unused_imports)]
#[path = "../crates/skate2-formats/src/big.rs"]
mod big;
#[path = "../crates/skate2-formats/src/host_path.rs"]
mod host_path;
#[path = "../crates/skate2-formats/src/mount.rs"]
mod mount;
#[path = "../crates/skate2-formats/src/pegasus.rs"]
mod pegasus;
#[path = "../crates/skate2-formats/src/refpack.rs"]
mod refpack;
#[path = "../crates/skate2-formats/src/rw4.rs"]
mod rw4;
#[path = "../crates/skate2-formats/src/stream.rs"]
mod stream;

use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Authored names carry a `_0x...` asset-path suffix; the readable part precedes it.
fn short_name(name: &str) -> &str {
    name.find("_0x").map_or(name, |i| &name[..i])
}

const MAIN_BANK_XSM: &str = "data\\content\\world\\dmo\\DMO\\DMO_Pres.xsm";

#[derive(Default)]
struct Prop {
    names: BTreeMap<String, usize>,
    placements: BTreeSet<u64>,
    /// Unique placements by source: the BAM city Sim stream, other world streams
    /// (training facilities), and mission/challenge collections.
    city: BTreeSet<u64>,
    world_other: BTreeSet<u64>,
    missions: BTreeSet<u64>,
    raw_records: usize,
    sample: Option<[f32; 3]>,
    sources: BTreeMap<String, usize>,
}

struct BankModel {
    name: String,
    grind_splines: Option<usize>,
    origin: String,
}

#[derive(Default)]
struct Stats {
    maps: usize,
    collections: usize,
    assets: usize,
    arenas: usize,
    non_arena: usize,
    dmo_objects: usize,
    errors: Vec<String>,
}

/// Every arena in one collection map: `(origin, resources, arena)`.
fn for_each_arena(
    mounts: &mount::MountSet,
    xsm: &str,
    stats: &mut Stats,
    mut visit: impl FnMut(&[Vec<u8>; 5], &rw4::Arena),
) {
    let map = match mounts.read_path(xsm).map_err(|e| format!("{e:?}")).and_then(|b| {
        stream::CollectionMap::parse(&b).map_err(|e| format!("{e:?}"))
    }) {
        Ok(map) => map,
        Err(e) => {
            stats.errors.push(format!("{xsm}: map: {e}"));
            return;
        }
    };
    stats.maps += 1;
    let dir = &xsm[..xsm.rfind('\\').unwrap_or(0)];
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    for row in &map.collections {
        stats.collections += 1;
        let mut xsf = if row.filename.contains('\\') {
            row.filename.clone()
        } else {
            format!("{dir}\\{}", row.filename)
        };
        // Map rows name the file with or without its `.xsf` extension.
        if !xsf.to_ascii_lowercase().ends_with(".xsf") {
            xsf.push_str(".xsf");
        }
        if !files.contains_key(&xsf) {
            match mounts.read_path(&xsf) {
                Ok(bytes) => {
                    files.insert(xsf.clone(), bytes);
                }
                Err(e) => {
                    stats.errors.push(format!("{xsf}: read: {e:?}"));
                    continue;
                }
            }
        }
        let file = &files[&xsf];
        let start = row.payload_offset as usize;
        let Some(window) = file.get(start..start.saturating_add(row.payload_size as usize)) else {
            stats.errors.push(format!("{xsf}: payload outside file"));
            continue;
        };
        let payload = match stream::CollectionPayload::parse(window, row.asset_count) {
            Ok(p) => p,
            Err(e) => {
                stats.errors.push(format!("{xsf} ({}): payload: {e:?}", row.filename));
                continue;
            }
        };
        for asset in &payload.assets {
            stats.assets += 1;
            let Ok(resources) = stream::decode_arena(&window[asset.data_range.clone()]) else {
                stats.non_arena += 1;
                continue;
            };
            let Ok(arena) = rw4::Arena::parse(&resources.resources) else {
                stats.non_arena += 1;
                continue;
            };
            stats.arenas += 1;
            visit(&resources.resources, &arena);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = std::path::PathBuf::from(args.next().ok_or("game root required")?);
    let show_sources = args.any(|a| a == "--sources");
    let mut mounts = mount::MountSet::new();
    for spec in &mount::SKATE2_ARCHIVES {
        let path = spec.path_under(&root);
        if path.is_file() {
            mounts.push(mount::MountedBig::open(path, spec.use_external_header)?);
        }
    }
    let mut maps = BTreeSet::new();
    for mounted in mounts.archives() {
        for entry in &mounted.archive().entries {
            if let big::BigEntryKey::Name(name) = &entry.key {
                let lower = name.to_ascii_lowercase();
                if lower.ends_with(".xsm") && !lower.ends_with("_tex.xsm") {
                    maps.insert(name.clone());
                }
            }
        }
    }

    let mut stats = Stats::default();

    // Bank models first: main bank, then every other tInstanceData seen anywhere.
    let mut bank: HashMap<u64, BankModel> = HashMap::new();
    let mut elsewhere: HashMap<u64, BankModel> = HashMap::new();
    let mut props: BTreeMap<u64, Prop> = BTreeMap::new();

    for xsm in &maps {
        let is_bank = xsm.eq_ignore_ascii_case(MAIN_BANK_XSM);
        let origin = xsm
            .trim_start_matches("data\\content\\")
            .trim_end_matches(".xsm")
            .replace('\\', "/");
        let mut dmo_objects = 0usize;
        let mut dmo_errors = Vec::new();
        for_each_arena(&mounts, xsm, &mut stats, |resources, arena| {
            for entry in &arena.dictionary {
                let Some(bytes) = arena.object_bytes(resources, entry) else { continue };
                if entry.type_id == pegasus::DMO_DATA_TYPE_ID {
                    dmo_objects += 1;
                    match pegasus::DmoData::parse(bytes) {
                        Ok(data) => {
                            for dmo in data.dmos {
                                let prop = props.entry(dmo.instance_guid).or_default();
                                *prop.names.entry(dmo.name).or_default() += 1;
                                prop.placements.insert(dmo.guid);
                                if origin.starts_with("world/stream/BAM/") {
                                    prop.city.insert(dmo.guid);
                                } else if origin.starts_with("world/") {
                                    prop.world_other.insert(dmo.guid);
                                } else {
                                    prop.missions.insert(dmo.guid);
                                }
                                prop.raw_records += 1;
                                *prop.sources.entry(origin.clone()).or_default() += 1;
                                prop.sample.get_or_insert([
                                    dmo.transform[12],
                                    dmo.transform[13],
                                    dmo.transform[14],
                                ]);
                            }
                        }
                        Err(e) => dmo_errors.push(format!("{origin}: DmoData: {e}")),
                    }
                } else if entry.type_id == pegasus::INSTANCE_DATA_TYPE_ID {
                    let Ok(data) = pegasus::InstanceData::parse(bytes) else { continue };
                    for inst in data.instances {
                        let grind_splines = match inst.embedded_data {
                            None => Some(0),
                            Some(id) => arena
                                .local_entry(id)
                                .and_then(|e| arena.object_bytes(resources, e))
                                .and_then(|b| pegasus::EmbeddedData::parse(b).ok())
                                .map(|e| e.grind_splines().len()),
                        };
                        let model = BankModel {
                            name: inst.name,
                            grind_splines,
                            origin: origin.clone(),
                        };
                        let target = if is_bank { &mut bank } else { &mut elsewhere };
                        target.entry(inst.local_guid).or_insert(model);
                    }
                }
            }
        });
        stats.dmo_objects += dmo_objects;
        stats.errors.extend(dmo_errors);
    }

    let mut rows: Vec<(String, u64, &Prop)> = props
        .iter()
        .map(|(guid, prop)| {
            let display = bank
                .get(guid)
                .map(|m| short_name(&m.name).to_owned())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| {
                    prop.names.keys().next().map(|n| short_name(n).to_owned()).unwrap_or_default()
                });
            (display, *guid, prop)
        })
        .collect();
    rows.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()).then(a.1.cmp(&b.1)));

    println!(
        "# Skate 2 DMO props: {} distinct model ids (locator instance_guid), {} placements (unique locator guid)",
        rows.len(),
        props.values().map(|p| p.placements.len()).sum::<usize>()
    );
    println!(
        "# scanned {} maps (non-Tex), {} collections, {} assets, {} RW4 arenas ({} non-arena), {} tDMOData objects; main bank models: {}",
        stats.maps, stats.collections, stats.assets, stats.arenas, stats.non_arena, stats.dmo_objects, bank.len()
    );
    println!("# model_name = main-bank tInstance name (else first locator name); in_bank = model id is a main DMO bank instance local_guid");
    println!("# has_grinds = bank model's tEmbeddedData grind-spline group nonempty (n = count); '-' = not in bank");
    println!("# count = unique placements (locator guid) = city + other_world + missions; city = BAM_Sim stream,");
    println!("#   other_world = DIST_TrainingFacility_* streams, missions = data/content/missions challenge/prop collections");
    println!("# names = distinct readable locator names (text before the '_0x' asset-path suffix) with counts");
    println!(
        "{:<36} {:<16} {:>5} {:>5} {:>5} {:>5} {:<7} {:<10} {:<24} names",
        "model_name", "instance_guid", "count", "city", "other", "miss", "in_bank", "has_grinds", "sample_pos(x,y,z)"
    );
    for (display, guid, prop) in &rows {
        let model = bank.get(guid);
        let in_bank = if model.is_some() { "yes" } else { "no" };
        let grinds = match model.map(|m| m.grind_splines) {
            None => "-".to_owned(),
            Some(None) => "unknown".to_owned(),
            Some(Some(0)) => "no".to_owned(),
            Some(Some(n)) => format!("yes ({n})"),
        };
        let pos = prop
            .sample
            .map(|p| format!("{:.1},{:.1},{:.1}", p[0], p[1], p[2]))
            .unwrap_or_default();
        let mut short: BTreeMap<&str, usize> = BTreeMap::new();
        for (n, c) in &prop.names {
            *short.entry(short_name(n)).or_default() += c;
        }
        let names: Vec<String> = short.iter().map(|(n, c)| format!("{n} x{c}")).collect();
        let mut extra = String::new();
        if model.is_none() {
            if let Some(m) = elsewhere.get(guid) {
                extra = format!("  [model found in {} as '{}']", m.origin, m.name);
            }
        }
        println!(
            "{:<36} {:016x} {:>5} {:>5} {:>5} {:>5} {:<7} {:<10} {:<24} {}{}",
            display,
            guid,
            prop.placements.len(),
            prop.city.len(),
            prop.world_other.len(),
            prop.missions.len(),
            in_bank,
            grinds,
            pos,
            names.join(", "),
            extra
        );
        if show_sources {
            for (source, count) in &prop.sources {
                println!("      from {source}: {count}");
            }
        }
    }

    let unused: Vec<&BankModel> = bank
        .iter()
        .filter(|(guid, _)| !props.contains_key(guid))
        .map(|(_, m)| m)
        .collect();
    let mut unused_rows: Vec<(String, u64, String)> = bank
        .iter()
        .filter(|(guid, _)| !props.contains_key(guid))
        .map(|(guid, m)| {
            let grinds = match m.grind_splines {
                None => "unknown".to_owned(),
                Some(0) => "no".to_owned(),
                Some(n) => format!("yes ({n})"),
            };
            (short_name(&m.name).to_owned(), *guid, grinds)
        })
        .collect();
    unused_rows.sort();
    println!();
    println!("# main-bank models with NO placement anywhere ({}):", unused.len());
    for (name, guid, grinds) in unused_rows {
        println!("{name:<40} {guid:016x} has_grinds={grinds}");
    }
    if !stats.errors.is_empty() {
        println!();
        println!("# {} read/parse errors:", stats.errors.len());
        for e in &stats.errors {
            println!("#   {e}");
        }
    }
    Ok(())
}
