//! Which looping voices each zone-pair crossfade group plays, per crossfade bank: data of the bank,
//! as in retail (`skate_audio::world::crossfade`):
//! 1. an audio content overlay's declared layout (`add.crossfade_layouts`, for banks without a
//!    program, e.g. a mod bank of WAVs);
//! 2. else the bank's own `c_main_ambience_crossfade` program, posted per group to a private
//!    evaluator (retail's three banks and any mod bank shipping a program);
//! 3. else none (the crossfade stays silent, logged once per map).
//!
//! The interim player (Bevy voices, `ambience.rs`) takes each voice as (sample slot, pan degrees,
//! level). Program layouts are rounded to the precision of the table the interim was first built
//! from (pan 0.1°, level 1e-4), so retail's three banks play exactly as before (test
//! `retail_layouts_come_from_the_banks_programs`, against that table in `crossfade_groups.rs`).
use super::Library;
use std::collections::BTreeMap;

/// (sample slot, pan degrees, level incl. the program's rear factor).
pub(super) type Layer = (usize, f32, f32);

/// Where a bank's layout came from (the log line and tests).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Declared,
    Program,
    None,
}

#[derive(Clone, Debug)]
pub(super) struct Layouts {
    pub source: Source,
    groups: BTreeMap<u32, Vec<Layer>>,
}

impl Layouts {
    pub(super) fn empty() -> Self {
        Self {
            source: Source::None,
            groups: BTreeMap::new(),
        }
    }

    pub(super) fn group(&self, group: u32) -> Option<&[Layer]> {
        self.groups.get(&group).map(Vec::as_slice)
    }

    pub(super) fn groups(&self) -> impl Iterator<Item = u32> + '_ {
        self.groups.keys().copied()
    }
}

/// Program azimuth (65536 = 360°) → degrees at 0.1°; level word (32767 = 1) → level at 1e-4.
fn layer(v: &skate_audio::world::crossfade::Voice) -> Layer {
    let pan = ((v.azimuth.rem_euclid(65536) as f64 * 3600.0 / 65536.0).round() / 10.0) as f32;
    let level = ((v.level.clamp(0, 32767) as f64 * 10000.0 / 32767.0).round() / 10000.0) as f32;
    (v.slot as usize, pan, level)
}

/// The bank's layouts (declared, else from its program), or why there are none.
pub(super) fn build(library: &Library, bank: &str) -> (Layouts, Option<String>) {
    if let Some(declared) = library.crossfade_layout(bank) {
        let groups = declared
            .iter()
            .filter_map(|(g, voices)| {
                Some((
                    g.parse().ok()?,
                    voices.iter().map(|v| (v.sample, v.pan, v.level)).collect(),
                ))
            })
            .collect();
        return (
            Layouts {
                source: Source::Declared,
                groups,
            },
            None,
        );
    }
    match from_program(library, bank) {
        Ok(groups) if !groups.is_empty() => (
            Layouts {
                source: Source::Program,
                groups,
            },
            None,
        ),
        Ok(_) => (
            Layouts {
                source: Source::None,
                groups: BTreeMap::new(),
            },
            Some(format!(
                "its program does not answer {}",
                skate_audio::world::crossfade::CLASS
            )),
        ),
        Err(e) => (
            Layouts {
                source: Source::None,
                groups: BTreeMap::new(),
            },
            Some(e),
        ),
    }
}

fn from_program(library: &Library, bank: &str) -> Result<BTreeMap<u32, Vec<Layer>>, String> {
    use skate_audio::formats::{Bank, Project};
    let files = library.aems();
    let abk = files
        .banks
        .get(bank)
        .ok_or("no program (.abk) and no declared layout (add.crossfade_layouts)")?;
    let bytes = library.read(abk).map_err(|e| format!("{abk}: {e}"))?;
    let bank = Bank::parse(bank, bytes).map_err(|e| format!("{abk}: {e}"))?;
    let mut projects = Vec::with_capacity(files.projects.len());
    for p in &files.projects {
        let bytes = library.read(p).map_err(|e| format!("{p}: {e}"))?;
        projects.push(Project::parse(p, &bytes).map_err(|e| format!("{p}: {e}"))?);
    }
    let groups = skate_audio::world::crossfade::layouts(&projects, &bank)?;
    Ok(groups
        .into_iter()
        .map(|(g, voices)| (g, voices.iter().map(layer).collect()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Retail's three crossfade banks: the layouts read from their programs are exactly the table
    /// the interim player used (same groups, voices in the same order, the same f32 bits), so the
    /// change from table to data plays retail byte for byte as before.
    #[test]
    #[ignore = "needs the private install data"]
    fn retail_layouts_come_from_the_banks_programs() {
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        let Ok(library) = Library::load(root) else {
            panic!("missing private data: no audio install")
        };
        let start = std::time::Instant::now();
        for bank in [
            "Main_Ambience_Crossfade_DT",
            "Main_Ambience_Crossfade_Ind",
            "Main_Ambience_Crossfade_Uni",
        ] {
            let (layouts, why) = build(&library, bank);
            assert_eq!((layouts.source, why), (Source::Program, None), "{bank}");
            let want: Vec<(u32, Vec<Layer>)> = super::super::crossfade_groups::GROUPS
                .iter()
                .filter(|(b, ..)| *b == bank)
                .map(|(_, g, voices)| (*g, voices.to_vec()))
                .collect();
            let got: Vec<(u32, Vec<Layer>)> = layouts
                .groups
                .iter()
                .map(|(g, v)| (*g, v.clone()))
                .collect();
            let bits = |rows: &[(u32, Vec<Layer>)]| {
                rows.iter()
                    .map(|(g, v)| {
                        (
                            *g,
                            v.iter()
                                .map(|(s, p, l)| (*s, p.to_bits(), l.to_bits()))
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(bits(&got), bits(&want), "{bank}");
        }
        eprintln!("three banks derived in {:?}", start.elapsed());
    }

    /// A mod crossfade bank: a declared layout (WAV-only bank), or the bank's own program (here a
    /// retail program shipped under a mod bank name, with the mod's WAVs), drives the mod's
    /// crossfade; without either the bank has none and says why.
    #[test]
    #[ignore = "needs the private install data"]
    fn a_mod_crossfade_bank_gets_its_layout() {
        use super::super::library::OverlaySource;
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        let Ok(plain) = Library::load(root) else {
            panic!("missing private data: no audio install")
        };
        let mods = std::env::temp_dir().join(format!("skate-crossfade-mod-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&mods);
        std::fs::create_dir_all(mods.join("audio")).unwrap();
        for i in 0..10 {
            std::fs::write(
                mods.join(format!("audio/f{i}.wav")),
                super::super::library::tests::test_wav(4800, 48000, 100 + i),
            )
            .unwrap();
        }
        std::fs::copy(
            plain.path(&plain.aems().banks["Main_Ambience_Crossfade_Ind"]),
            mods.join("audio/prog.abk"),
        )
        .unwrap();
        let wavs: Vec<String> = (0..10).map(|i| format!("audio/f{i}.wav")).collect();
        let o: skate_mods::audio_content::AudioOverlay = serde_json::from_value(serde_json::json!({"version": 1, "add": {
            "banks": {"MOD_fade_wav": {"samples": wavs[..2]}, "MOD_fade_prog": {"abk": "audio/prog.abk", "samples": wavs}, "MOD_fade_none": {"samples": wavs[..1]}},
            "crossfade_layouts": {"MOD_fade_wav": {"1": [{"sample": 1, "pan": 90}, {"sample": 0, "pan": 270, "level": 0.5}], "3": [{"sample": 0}]}}
        }})).unwrap();
        o.validate().unwrap();
        let (library, report) = Library::load_with(
            root,
            &[OverlaySource {
                id: "dev.a",
                root: &mods,
                overlay: &o,
            }],
        )
        .unwrap();
        assert!(
            report.warnings.is_empty() && report.conflicts.is_empty() && report.rejected.is_empty(),
            "{report:?}"
        );
        let (wav, why) = build(&library, "MOD_fade_wav");
        assert_eq!((wav.source, why), (Source::Declared, None));
        assert_eq!(wav.group(1), Some(&[(1, 90.0, 1.0), (0, 270.0, 0.5)][..]));
        assert_eq!(wav.groups().collect::<Vec<_>>(), [1, 3]);
        let (prog, why) = build(&library, "MOD_fade_prog");
        assert_eq!((prog.source, why), (Source::Program, None));
        let (retail, _) = build(&plain, "Main_Ambience_Crossfade_Ind");
        assert_eq!(
            format!("{:?}", prog.groups),
            format!("{:?}", retail.groups),
            "the same program lays out the same groups"
        );
        assert!(prog.groups().all(|g| {
            prog.group(g)
                .unwrap()
                .iter()
                .all(|(s, ..)| *s < library.bank_len("MOD_fade_prog"))
        }));
        let (none, why) = build(&library, "MOD_fade_none");
        assert_eq!(none.source, Source::None);
        assert!(why.unwrap().contains("no program"));
        let _ = std::fs::remove_dir_all(mods);
    }
}
