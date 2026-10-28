//! Validate a mod package before shipping it:
//!
//!   check_mod <package-folder-or-zip> [--install <assets folder>] [--with <other package>…]
//!
//! Checks `mod.json`, the Lua entry's syntax and, when the package has an `audio.json`, the audio
//! content overlay in depth (schema, every WAV / bank / Splice tree / MixMap / grain member it
//! names). With `--install` (the game's `assets` folder) the overlay is also merged over the
//! install's audio manifest, as the game does, to list identities the install does not have
//! (including speech clips and takes the install's speech indexes lack) and conflicts with the
//! `--with` packages (merged in mod-id order, the first owner wins), and every crossfade bank the
//! overlay names or lays out is checked: its layout comes from its `c_main_ambience_crossfade`
//! program (as retail) or from `add.crossfade_layouts`, else its crossfades stay silent.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;
use skate_mods::audio_content::{AudioOverlay, SpeechClips};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut path = None;
    let mut install = None;
    let mut others = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--install" => install = args.next(),
            "--with" => others.extend(args.next()),
            _ => path = Some(a),
        }
    }
    let path = path.expect(
        "usage: check_mod <package-folder-or-zip> [--install <assets>] [--with <package>…]",
    );
    let (manifest, audio, root) = match skate_mods::validate_package_content_at(Path::new(&path)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("FAIL: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "OK {} api={} entry={}",
        manifest.id, manifest.api, manifest.entry
    );
    let Some(audio) = audio else { return };
    println!(
        "audio.json: {} MiB of PCM",
        audio.pcm_bytes as f64 / (1024.0 * 1024.0)
    );
    for line in audio.overlay.summary() {
        println!("  {line}");
    }
    let Some(install) = install else { return };
    let audio_root = Path::new(&install).join("private/audio");
    let file = audio_root.join("audio_manifest.json");
    let mut m: Value = match std::fs::read(&file)
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
    {
        Ok(m) => m,
        Err(e) => {
            eprintln!("FAIL: {}: {e}", file.display());
            std::process::exit(1);
        }
    };
    let mut overlays = vec![(manifest.id.clone(), audio.overlay, root)];
    for other in &others {
        match skate_mods::validate_package_content_at(Path::new(other)) {
            Ok((m, Some(a), r)) if !overlays.iter().any(|(id, ..)| *id == m.id) => {
                overlays.push((m.id, a.overlay, r))
            }
            Ok((m, Some(_), _)) => println!("  ({} is already in the list)", m.id),
            Ok((m, None, _)) => println!("  ({} has no audio.json)", m.id),
            Err(e) => println!("  ({other}: {e})"),
        }
    }
    overlays.sort_by(|a, b| a.0.cmp(&b.0));
    // The speech indexes, read only when an overlay names speech (they are a few MB).
    let speech = overlays
        .iter()
        .any(|(_, o, _)| SpeechClips::needed(o))
        .then(|| SpeechClips::load(&audio_root, &m));
    // Mod Csis projects (doc 16 L4): their symbols against the install's and the earlier mods' (the
    // game leaves such an overlay out).
    let mut clashes: Vec<(String, String)> = Vec::new();
    if overlays.iter().any(|(_, o, _)| !o.add.projects.is_empty()) {
        let mut taken: std::collections::BTreeSet<(u8, String)> = m["aems"]["projects"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
            .filter_map(|f| {
                std::fs::read(audio_root.join(f))
                    .ok()
                    .and_then(|b| skate_mods::audio_content::project_symbols(&b, f).ok())
            })
            .flatten()
            .collect();
        for (id, o, root) in &overlays {
            let symbols: Vec<(u8, String)> = o
                .add
                .projects
                .iter()
                .filter_map(|f| {
                    skate_mods::read_bounded(root, f, skate_mods::audio_content::MAX_BINARY_BYTES)
                        .ok()
                        .and_then(|b| skate_mods::audio_content::project_symbols(&b, f).ok())
                })
                .flatten()
                .collect();
            match skate_mods::audio_content::project_clash(&taken, &symbols) {
                Some(c) => clashes.push((id.clone(), c)),
                None => {
                    if !o.add.projects.is_empty() {
                        println!(
                            "  Csis projects of {id}: {} symbols, none of the install's",
                            symbols.len()
                        );
                    }
                    taken.extend(symbols);
                }
            }
        }
    }
    let sources: Vec<_> = overlays
        .iter()
        .map(|(id, o, _)| skate_mods::audio_merge::Source { id, overlay: o })
        .collect();
    let report = skate_mods::audio_merge::merge_with(&mut m, &sources, speech.as_ref());
    let mine = |owner: &str| owner == manifest.id;
    let mut warnings: Vec<String> = report
        .warnings
        .iter()
        .filter(|w| mine(&w.owner))
        .map(|w| w.text.clone())
        .collect();
    let conflicts: Vec<_> = report.conflicts.iter().filter(|c| mine(&c.owner)).collect();
    let roots: BTreeMap<&str, &Path> = overlays
        .iter()
        .map(|(id, _, r)| (id.as_str(), r.as_path()))
        .collect();
    warnings.extend(
        clashes
            .iter()
            .filter(|(id, _)| mine(id))
            .map(|(_, c)| c.clone()),
    );
    let own = &overlays
        .iter()
        .find(|(id, ..)| *id == manifest.id)
        .expect("the checked mod")
        .1;
    let crossfades = crossfade_banks(&audio_root, &m, &roots, own);
    for (bank, result) in &crossfades {
        match result {
            Ok(line) => println!("  crossfade bank {bank}: {line}"),
            Err(e) => warnings.push(format!("crossfade bank {bank}: {e}")),
        }
    }
    println!(
        "install: {} identities changed, {} warnings, {} conflicts",
        report.claimed.values().filter(|o| mine(o)).count(),
        warnings.len(),
        conflicts.len()
    );
    for w in warnings {
        println!("  warning: {w}");
    }
    for c in conflicts {
        println!("  conflict: {}", c.text);
    }
}

/// Where a merged manifest's file reference lives: `mod:<id>/<path>` in that mod's root, else
/// under the install's audio folder.
fn resolve(audio_root: &Path, roots: &BTreeMap<&str, &Path>, file: &str) -> Option<PathBuf> {
    match skate_mods::audio_merge::split_mod_ref(file) {
        Some((id, rel)) => roots.get(id).map(|r| r.join(rel)),
        None => Some(audio_root.join(file)),
    }
}

/// The crossfade banks the checked overlay names (`maps.*.crossfade_bank`), lays out
/// (`add.crossfade_layouts`) or ships a program for (`.abk` in `replace` / `add.banks`), and where
/// each one's layout comes from, the way the game picks it: a declared layout, else the bank's
/// `c_main_ambience_crossfade` program. Programs that do not answer the class are only an error
/// for banks a map names.
fn crossfade_banks(
    audio_root: &Path,
    m: &Value,
    roots: &BTreeMap<&str, &Path>,
    o: &AudioOverlay,
) -> Vec<(String, Result<String, String>)> {
    let named: BTreeSet<&str> = o
        .maps
        .values()
        .filter_map(|d| d.crossfade_bank.as_deref())
        .collect();
    let programs: BTreeSet<&str> = o
        .replace
        .banks
        .iter()
        .chain(&o.add.banks)
        .filter(|(_, b)| b.abk.is_some())
        .map(|(s, _)| s.as_str())
        .collect();
    let banks: BTreeSet<&str> = named
        .iter()
        .copied()
        .chain(o.add.crossfade_layouts.keys().map(String::as_str))
        .chain(programs.iter().copied())
        .collect();
    let mut projects = None;
    let mut out = Vec::new();
    for bank in banks {
        if let Some(groups) = m
            .get("mod_crossfade_layouts")
            .and_then(|l| l.get(bank))
            .and_then(Value::as_object)
        {
            out.push((
                bank.to_owned(),
                Ok(format!(
                    "declared layout, groups {:?}",
                    groups.keys().collect::<Vec<_>>()
                )),
            ));
            continue;
        }
        let Some(abk) = m
            .get("aems")
            .and_then(|a| a.get("banks"))
            .and_then(|b| b.get(bank))
            .and_then(Value::as_str)
        else {
            if named.contains(bank) {
                out.push((
                    bank.to_owned(),
                    Err(
                        "no program and no add.crossfade_layouts entry: its crossfades stay silent"
                            .into(),
                    ),
                ));
            }
            continue;
        };
        let projects = projects.get_or_insert_with(|| {
            m.get("aems")
                .and_then(|a| a.get("projects"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter_map(|p| {
                    skate_audio::formats::Project::parse(
                        p,
                        &std::fs::read(audio_root.join(p)).ok()?,
                    )
                    .ok()
                })
                .collect::<Vec<_>>()
        });
        let result = resolve(audio_root, roots, abk)
            .and_then(|p| std::fs::read(p).ok())
            .ok_or_else(|| format!("{abk} cannot be read"))
            .and_then(|bytes| {
                skate_audio::formats::Bank::parse(bank, bytes).map_err(|e| e.to_string())
            })
            .and_then(|b| skate_audio::world::crossfade::layouts(projects, &b));
        match result {
            Ok(groups) if !groups.is_empty() => {
                let len = m.get("banks").and_then(|b| b.get(bank)).and_then(Value::as_array).map_or(0, Vec::len);
                let worst = groups.iter().flat_map(|(_, v)| v).map(|v| v.slot as usize).max().unwrap_or(0);
                if worst >= len {
                    out.push((bank.to_owned(), Err(format!("its program plays sample {worst}, the bank has {len} samples"))));
                } else {
                    out.push((bank.to_owned(), Ok(format!("{} groups from its program ({})", groups.len(), skate_audio::world::crossfade::CLASS))));
                }
            }
            Ok(_) if named.contains(bank) => out.push((bank.to_owned(), Err(format!("its program does not answer {} and there is no add.crossfade_layouts entry: its crossfades stay silent", skate_audio::world::crossfade::CLASS)))),
            Ok(_) => {}
            Err(e) => out.push((bank.to_owned(), Err(e))),
        }
    }
    out
}
