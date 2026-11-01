//! Read-only census of Skate 3's park editor models (`parkassets.big`, extracted): parses each
//! standalone `.rx2` arena with the production RW4 parser and counts its dictionary's object
//! types, per folder.
//!
//! Build: rustc --edition 2024 -O tools/inspect_parkassets.rs -o target/inspect_parkassets.exe
//! Run:   target/inspect_parkassets.exe "<extracted parkassets root>" [file.rx2]
#![allow(dead_code, unused_imports)]
#[path = "../crates/skate2-formats/src/pegasus.rs"]
mod pegasus;
#[path = "../crates/skate2-formats/src/rw4.rs"]
mod rw4;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("rx2")) {
            out.push(path);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("pass the extracted root")?);
    if let Some(one) = std::env::args().nth(2) {
        let bytes = std::fs::read(root.join(one))?;
        let resources = rw4::Arena::split_file_resources(&bytes).map_err(|e| format!("{e:?}"))?;
        let arena = rw4::Arena::parse(&resources).map_err(|e| format!("{e:?}"))?;
        println!("arena id {:#x}, header {:?}", arena.id, arena.file_header);
        let dump = std::env::args().nth(3).and_then(|t| u32::from_str_radix(t.trim_start_matches("0x"), 16).ok());
        for (index, entry) in arena.dictionary.iter().enumerate() {
            println!("{index:3} type {:#010x} res {} size {}", entry.type_id, entry.resource_index, entry.size);
            if Some(entry.type_id) == dump {
                let bytes = &resources[entry.resource_index][entry.object_range.clone()];
                for (row, chunk) in bytes.chunks(16).enumerate() {
                    let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
                    let floats: Vec<String> = chunk.chunks(4).filter(|w| w.len() == 4).map(|w| format!("{:.3}", f32::from_be_bytes([w[0], w[1], w[2], w[3]]))).collect();
                    println!("    {:04x}: {}   {}", row * 16, hex.join(" "), floats.join(" "));
                }
            }
        }
        return Ok(());
    }
    let mut files = Vec::new();
    walk(&root, &mut files);
    if std::env::var_os("SPLINE_CENSUS").is_some() {
        // 0x00eb0004 (tSplineData) through the production Pegasus parser: splines and segments
        // per folder, parse failures, and one file's segments in full (SPLINE_FILE).
        let mut folders: BTreeMap<String, (usize, usize, usize, usize)> = BTreeMap::new();
        for file in &files {
            let folder = file.parent().and_then(|p| p.file_name()).map_or(String::new(), |n| n.to_string_lossy().into_owned());
            let Ok(bytes) = std::fs::read(file) else { continue };
            let Ok(resources) = rw4::Arena::split_file_resources(&bytes) else { continue };
            let Ok(arena) = rw4::Arena::parse(&resources) else { continue };
            let slot = folders.entry(folder).or_default();
            slot.0 += 1;
            for entry in arena.dictionary.iter().filter(|e| e.type_id == pegasus::SPLINE_DATA_TYPE_ID) {
                let b = &resources[entry.resource_index][entry.object_range.clone()];
                match pegasus::SplineData::parse(b) {
                    Ok(data) => {
                        slot.1 += data.splines.len();
                        slot.2 += data.segments.len();
                        if std::env::var("SPLINE_FILE").is_ok_and(|f| file.to_string_lossy().contains(&f)) {
                            println!("{} bytes {}: {:#?}", file.display(), b.len(), data.splines);
                            for segment in &data.segments {
                                println!("  {:?}", segment);
                            }
                        }
                    }
                    Err(e) => {
                        slot.3 += 1;
                        eprintln!("{}: {e:?}", file.display());
                    }
                }
            }
        }
        for (folder, (files, splines, segments, failed)) in folders {
            println!("{folder}: {files} files, {splines} splines, {segments} segments, {failed} failed");
        }
        return Ok(());
    }
    if std::env::var_os("SNAP_CENSUS").is_some() {
        // 0x00eb001e: u32 count, u32 ?, 8 pad, then count records of 0x70 bytes: a 4x4 matrix,
        // bounds min and max (vec4 each), u32, kind byte.
        let mut kinds: BTreeMap<(String, u8, u32), usize> = BTreeMap::new();
        for file in &files {
            let folder = file.parent().and_then(|p| p.file_name()).map_or(String::new(), |n| n.to_string_lossy().into_owned());
            let Ok(bytes) = std::fs::read(file) else { continue };
            let Ok(resources) = rw4::Arena::split_file_resources(&bytes) else { continue };
            let Ok(arena) = rw4::Arena::parse(&resources) else { continue };
            for entry in arena.dictionary.iter().filter(|e| e.type_id == 0x00eb001e) {
                let b = &resources[entry.resource_index][entry.object_range.clone()];
                let be = |at: usize| u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
                let count = be(0) as usize;
                if std::env::var("SNAP_FILE").is_ok_and(|f| file.to_string_lossy().contains(&f)) {
                    for i in 0..count {
                        let r = 16 + i * 0x70;
                        let f = |at: usize| f32::from_bits(be(r + at));
                        println!("  {i:2} kind {} word {:#x} pos [{:.2} {:.2} {:.2}] x [{:.2} {:.2} {:.2}] z [{:.2} {:.2} {:.2}]", b[r + 0x64], be(r + 0x60), f(0x30), f(0x34), f(0x38), f(0), f(4), f(8), f(0x20), f(0x24), f(0x28));
                    }
                }
                for i in 0..count {
                    let r = 16 + i * 0x70;
                    if r + 0x70 <= b.len() {
                        *kinds.entry((if folder.starts_with("dmo") { "dmo".into() } else { folder.clone() }, b[r + 0x64], be(r + 0x60))).or_default() += 1;
                    }
                }
            }
        }
        for ((folder, kind, word), n) in kinds {
            println!("{folder}: kind {kind} word {word:#x}: {n}");
        }
        return Ok(());
    }
    let mut by_folder: BTreeMap<String, (usize, usize, BTreeMap<u32, usize>)> = BTreeMap::new();
    for file in &files {
        let folder = file.parent().and_then(|p| p.file_name()).map_or(String::new(), |n| n.to_string_lossy().into_owned());
        let slot = by_folder.entry(folder).or_default();
        slot.0 += 1;
        let parsed = std::fs::read(file)
            .map_err(|e| format!("{e}"))
            .and_then(|bytes| rw4::Arena::split_file_resources(&bytes).map_err(|e| format!("{e:?}")))
            .and_then(|resources| rw4::Arena::parse(&resources).map_err(|e| format!("{e:?}")));
        match parsed {
            Ok(arena) => {
                for entry in &arena.dictionary {
                    *slot.2.entry(entry.type_id).or_default() += 1;
                }
            }
            Err(e) => {
                slot.1 += 1;
                eprintln!("{}: {e}", file.display());
            }
        }
    }
    for (folder, (count, failed, types)) in by_folder {
        let types: Vec<String> = types.iter().map(|(t, n)| format!("{t:#010x}x{n}")).collect();
        println!("{folder}: {count} files, {failed} failed: {}", types.join(" "));
    }
    Ok(())
}
