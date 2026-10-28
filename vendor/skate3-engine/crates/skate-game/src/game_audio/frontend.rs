//! The front-end sounds in the game (`crate::ui_audio::FrontendSound` → retail's front-end audio
//! object, `skate_audio::frontend`): the `fe` records from the install, played as `sk8_menu`
//! Splice one-shots in the audio pass (after the MixMap tick, once per pass, dt = the pass's host
//! ticks). Requests that arrive on a frame without a pass wait for the next one, as retail's queued
//! message waits for the next audio frame.
use bevy::prelude::*;
use skate_audio::frontend::{FeTable, Frontend};

use super::Library;
use super::native::{MIX_STEP, Native};

/// The front-end object and the records it plays.
pub(crate) struct FrontendHost {
    pub(crate) core: Frontend,
    pub(crate) table: FeTable,
    /// Splice sounds started (diagnostics, tests).
    pub(crate) started: Vec<(u64, u32)>,
}

impl FrontendHost {
    /// The install's `fe` records and their Splice bank, loaded into the runtime's Splice player;
    /// None (logged) on installs without them.
    pub(super) fn load(native: &Native, library: &Library) -> Option<Self> {
        let Some(table) = library.frontend_sounds() else {
            warn!(
                "Game audio: this install has no front-end sounds: the session marker is silent (run setup to refresh the audio)"
            );
            return None;
        };
        let Some((bank, pcm)) = library.splice_bank(&table.bank) else {
            warn!(
                "Game audio: {} has no patch tree in this install: the session marker is silent (run setup to refresh the audio)",
                table.bank
            );
            return None;
        };
        let mut runtime = native.shared.lock().ok()?;
        let rt = &mut *runtime;
        rt.splice.load_bank(&table.bank, bank, pcm, &mut rt.mixer);
        Some(Self {
            core: Frontend::default(),
            table,
            started: Vec::new(),
        })
    }

    /// Queue one record (an unknown key or a silent record plays nothing).
    pub(crate) fn request(&mut self, key: u64) -> bool {
        match self.table.sounds.get(&key) {
            Some(sound) => self.core.request(key, sound, false),
            None => {
                debug!("Game audio: no front-end sound {key:016X}");
                false
            }
        }
    }

    /// One audio frame of `dt` seconds.
    pub(crate) fn frame(&mut self, dt: f32, runtime: &mut skate_audio::runtime::Runtime) {
        if !self.core.busy() {
            return;
        }
        let report = self
            .core
            .frame(dt, &self.table.bank, &mut runtime.splice_host());
        for &(key, id) in &report.started {
            if std::env::var_os("SKATE_AUDIO_TRACE").is_some() {
                info!("AUDIO_NATIVE frontend {key:016X} sk8_menu {id}");
            }
        }
        self.started.extend(report.started);
    }
}

/// The front-end volume from the MixMap word (int → single × retail's 1/32767 constant `0x38000100`).
pub(crate) fn volume(word: i32) -> f32 {
    ((word as f64) as f32) * f32::from_bits(0x3800_0100)
}

/// The pass's front-end step: take this frame's requests, then (on frames with a pass) run the
/// object once.
pub(super) fn frame(
    native: Option<ResMut<Native>>,
    mut sounds: MessageReader<crate::ui_audio::FrontendSound>,
) {
    let Some(mut native) = native else {
        sounds.clear();
        return;
    };
    let native = &mut *native;
    let Some(host) = &mut native.frontend else {
        sounds.clear();
        return;
    };
    for s in sounds.read() {
        host.request(s.key);
    }
    if native.frame_ticks == 0 {
        return;
    }
    // sub_824958F0: sk8_menu (and HOM_Set_1) starts scale by the Master controller's output 0
    // (`[[X+88]+40]`, X = the audio system; 14568 with free-skate's Master inputs, recomp watch run
    // `marker_fevol`), × 1/32767 as retail converts it.
    if let Some(m) = &native.mixmap {
        host.core.volume = volume(m.level(skate_audio::mixmap::keys::MASTER, 0));
    }
    let dt = native.frame_ticks as f32 * MIX_STEP;
    if let Ok(mut runtime) = super::timing::lock(&native.shared, &super::timing::GAME_LOCK) {
        host.frame(dt, &mut runtime);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One record through the real runtime (the install's sk8_menu and `fe` records): the voices it
    /// starts over 0.5 s with the game's pass at 60 Hz (one host tick per pass, as the player's Splice
    /// sounds run), as (sample, delay ms, first gain, peak gain over the voice's first 400 ms).
    fn play(library: &Library, name: &str) -> Vec<(u16, f32, f32, f32)> {
        let Ok(mut native) = Native::start(library) else {
            panic!("missing private data: no AEMS install")
        };
        let Some(mut host) = native.frontend.take() else {
            panic!("missing private data: no front-end sounds (stage_frontend.py / setup)")
        };
        let (key, _) = host
            .table
            .named(name)
            .unwrap_or_else(|| panic!("no fe record {name}"));
        assert!(host.request(key));
        // The volume word as the pass reads it: the MixMap's Master output 0 with free skate's inputs.
        let m = native.mixmap.as_mut().expect("MixMap");
        for _ in 0..30 {
            for id in 1..=4 {
                m.set_input(skate_audio::mixmap::keys::MASTER, id, 32767);
            }
            for id in [1, 2, 5] {
                m.set_input(skate_audio::mixmap::keys::MUSIC, id, 32767);
            }
            m.set_input(skate_audio::mixmap::keys::REVERB, 5, 32767);
            m.tick(skate_audio::mixmap::cadence::CONSOLE_DT);
        }
        let word = m.level(skate_audio::mixmap::keys::MASTER, 0);
        assert_eq!(
            word, 14568,
            "Master out 0 = the recomp's [[X+88]+40] (watch run marker_fevol)"
        );
        host.core.volume = volume(word);
        let mut rt = native.shared.lock().unwrap();
        let bank =
            skate_audio::splice::MIXER_BANK_BASE + rt.splice.bank_index(&host.table.bank).unwrap();
        let block_s = skate_audio::BLOCK as f64 / 48_000.0;
        let (mut t, mut next_frame) = (0.0f64, 0.0f64);
        let mut seen: std::collections::HashMap<u32, usize> = Default::default();
        let mut out: Vec<(u16, f32, f32, f32)> = Vec::new();
        while t < 0.5 {
            if t >= next_frame {
                host.frame(MIX_STEP, &mut rt);
                next_frame += f64::from(MIX_STEP);
            }
            for v in rt.mixer.snapshot().into_iter().filter(|v| v.bank == bank) {
                let i = *seen.entry(v.id).or_insert_with(|| {
                    out.push((v.slot, (t * 1000.0) as f32, v.gain, 0.0));
                    out.len() - 1
                });
                if f64::from(out[i].1) + 400.0 > t * 1000.0 {
                    out[i].3 = out[i].3.max(v.gain);
                }
            }
            rt.render_block();
            t += block_s;
        }
        out
    }

    /// The session marker's records against the recomp (user sessions `audiox_bail_20261003_094336`,
    /// `audiox_20261003_095054`, scripted `bailrun_ok*`, `all_20261002_223613`: 50 activates, 11
    /// places, 18 go-tos; medians of the local tool `marker_sounds.py`): per sk8_menu sample its
    /// first gain (or, for sample 40, which fades in, its peak) and its order. The volume: the
    /// MixMap's Master output 0 (14568 / 32767), as retail reads it.
    #[test]
    #[ignore = "needs the private install data"]
    fn marker_sounds_follow_the_recomp() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        let Ok(library) = Library::load(root) else {
            panic!("missing private data: no audio install")
        };
        // (record, [(sample, recomp delay ms, recomp first gain, recomp peak gain)]).
        let cases: [(&str, &[(u16, f32, f32, f32)]); 3] = [
            (
                skate_audio::frontend::marker::ACTIVATE,
                &[(58, 10.0, 0.6288, 0.6288), (59, 46.0, 0.1572, 0.1572)],
            ),
            (
                skate_audio::frontend::marker::PLACE,
                &[
                    (58, 13.0, 0.8892, 0.8892),
                    (40, 77.0, 0.0039, 0.3048),
                    (59, 144.0, 0.6288, 0.6288),
                ],
            ),
            (
                skate_audio::frontend::marker::GOTO,
                &[
                    (58, 8.0, 0.6288, 0.6288),
                    (36, 162.0, 0.4446, 0.4446),
                    (40, 189.0, 0.0007, 0.2223),
                    (59, 278.0, 0.6288, 0.6288),
                ],
            ),
        ];
        for (name, recomp) in cases {
            let ours = play(&library, name);
            eprintln!("{name}: ours {ours:?}");
            let samples: Vec<u16> = ours.iter().map(|v| v.0).collect();
            let want: Vec<u16> = recomp.iter().map(|v| v.0).collect();
            assert_eq!(samples, want, "{name}: the samples in retail's order");
            for (o, r) in ours.iter().zip(recomp) {
                // Levels: the peak (the steady level; sample 40 fades in). The recomp's first GAIN is
                // already the steady level because its game frames (hundreds per second) put the
                // start and the first update into one render drain; by the code the start block has
                // no record gain yet (sub_82975A60 → sub_82975CC8 copies the owner's block), so ours
                // starts at level / record gain for one pass.
                // Sample 40's fade peak depends on where the updates fall on its envelope (the recomp's
                // frames are much shorter than our pass): 5 %; the steady ones 1 %.
                let (got, exp) = (o.3, r.3);
                let tolerance = if r.0 == 40 { 0.05 } else { 0.01 };
                assert!(
                    (got - exp).abs() <= tolerance * exp,
                    "{name} sample {}: gain {got} vs the recomp's {exp}",
                    r.0
                );
                // Delays: the recomp's frame (~33 ms) plus its message hop.
                assert!(
                    (o.1 - r.1).abs() <= 50.0,
                    "{name} sample {}: {} ms vs the recomp's {} ms",
                    r.0,
                    o.1,
                    r.1
                );
            }
        }
    }

    /// The engine path end to end (data-gated): the session marker's events → `ui_audio`'s retail
    /// records → this system → the runtime's sk8_menu starts; nothing starts on a frame without a
    /// pass, the requests wait for the next one.
    #[test]
    #[ignore = "needs the private install data"]
    fn session_marker_events_start_retails_sounds() {
        use crate::ui_audio::{SessionMarkerAction, SessionMarkerEvent};
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        let Ok(library) = Library::load(root) else {
            panic!("missing private data: no audio install")
        };
        let Ok(native) = Native::start(&library) else {
            panic!("missing private data: no AEMS install")
        };
        assert!(
            native.frontend.is_some(),
            "missing private data: no front-end sounds (stage_frontend.py / setup)"
        );
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .configure_sets(Update, crate::app::FrameSet::Animation);
        app.add_plugins(crate::ui_audio::UiAudioPlugin);
        app.add_systems(Update, frame.after(crate::ui_audio::UiAudioSet));
        app.insert_resource(native);
        for action in [SessionMarkerAction::Opened, SessionMarkerAction::Placed] {
            app.world_mut().write_message(SessionMarkerEvent { action });
        }
        app.update();
        let started = |app: &App| {
            app.world()
                .resource::<Native>()
                .frontend
                .as_ref()
                .unwrap()
                .started
                .clone()
        };
        assert!(
            started(&app).is_empty(),
            "no pass this frame: the requests wait"
        );
        app.world_mut().resource_mut::<Native>().frame_ticks = 1;
        app.update();
        use skate_audio::frontend::{key, marker};
        assert_eq!(
            started(&app),
            [(key(marker::ACTIVATE), 235), (key(marker::PLACE), 237)]
        );
        app.world_mut().write_message(SessionMarkerEvent {
            action: SessionMarkerAction::Returned,
        });
        app.update();
        assert_eq!(started(&app).last(), Some(&(key(marker::GOTO), 236)));
    }
}
