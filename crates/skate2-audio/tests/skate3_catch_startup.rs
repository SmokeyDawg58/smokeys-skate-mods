use skate2_audio::rwac::{AudioCore, PluginRef};
use skate2_audio::rwac::core::{PlugInConfig, PlugInKind};
use skate2_audio::splicer::{SpliceManager, SplicePlayParams, SpliceUpdateParams};
use std::sync::Arc;

fn render(bytes: &Arc<[u8]>, slot: usize, cue: i32, volume: f32, corrected: bool) -> (f32, f32) {
    let mut core = AudioCore::new();
    let master = core.create_voice(255, "Mastering", &[
        PlugInConfig { kind: PlugInKind::SubMix { name: "Master".into() }, output_channels: 6 },
        PlugInConfig { kind: PlugInKind::Dac, output_channels: 6 },
    ]).unwrap();
    let mut splicer = SpliceManager::new();
    splicer.load_bank(Arc::clone(bytes), slot).unwrap();
    let id = splicer.create_splice(&mut core, slot, cue, PluginRef { voice: master, index: 0 }).unwrap();
    let input = SpliceUpdateParams { vol: volume, delta: 1.0 / 60.0, spread: 0.0, ..Default::default() };
    if corrected {
        splicer.play_splice_with_initial_mix(&mut core, id, SplicePlayParams::default(), &input);
    } else {
        splicer.play_splice(&mut core, id, SplicePlayParams::default(), &SpliceUpdateParams { vol: 0.0, ..input });
    }
    let mut first = 0.0_f32;
    let mut whole = 0.0_f32;
    for block in 0..96 {
        if block % 3 == 0 {
            // Fresh input every tick: update_splice multiplies its argument.
            let mut update = input;
            splicer.update_splice(&mut core, id, &mut update);
            splicer.update(&mut core);
        }
        for channels in core.render_frame().chunks_exact(6) {
            // Exactly the game's AudioSystem::render_stereo fold-down.
            let gain = f32::from_bits(0x3f34_fdf4);
            for sample in [channels[2] * gain + channels[0] + channels[4], channels[2] * gain + channels[1] + channels[5]] {
                assert!(sample.is_finite());
                whole = whole.max(sample.abs());
                if block < 3 { first = first.max(sample.abs()); }
            }
        }
    }
    (first, whole)
}

#[test]
#[ignore = "requires the original game sound bank via SKATE2_GAME_ROOT"]
fn test4_catch_bank_decodes_and_honors_sfx_mute() {
    let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("original game root"));
    let archive = skate2_formats::mount::MountedBig::open(root.join("data/audio/splicerskateboard.big"), false).unwrap();
    let bytes: Arc<[u8]> = archive.read_path("SK8_Foot_Catch.bnk").unwrap().into();
    // Test 4's runtime Vault ranges: first/second/both, each at soft/medium/hard.
    for cue in [0, 3, 9, 13, 16, 22, 26, 27, 29] {
        let old = render(&bytes, 16, cue, 0.3, false);
        let fixed = render(&bytes, 16, cue, 0.3, true);
        let muted = render(&bytes, 16, cue, 0.0, true);
        assert!(fixed.1 > 0.00001, "silent Test 4 cue {cue}");
        assert_eq!(muted, (0.0, 0.0), "SFX mute was bypassed");
        println!("Test4 cue={cue} initial={old:?} corrected={fixed:?} muted={muted:?}");
    }
}

#[test]
#[ignore = "requires the original Skate 3 catch bank via SKATE3_DARKCATCH_BANK"]
fn original_skate3_inverted_cues_render_at_user_volume_and_mute() {
    let bytes: Arc<[u8]> = std::fs::read(std::env::var_os("SKATE3_DARKCATCH_BANK").unwrap()).unwrap().into();
    for cue in 9..18 {
        let fixed = render(&bytes, skate2_audio::splicer::SKATE3_FOOT_CATCH_BANK, cue, 0.3, true);
        let muted = render(&bytes, skate2_audio::splicer::SKATE3_FOOT_CATCH_BANK, cue, 0.0, true);
        assert!(fixed.1 > 0.00001, "silent original Skate 3 cue {cue}");
        assert_eq!(muted, (0.0, 0.0), "SFX mute was bypassed");
        println!("Skate3 cue={cue} original={} corrected={fixed:?} muted={muted:?}", 1159+cue-9);
    }
}
