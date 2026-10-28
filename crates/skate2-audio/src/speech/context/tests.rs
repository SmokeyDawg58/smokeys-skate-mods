use std::sync::Arc;

use super::*;
use crate::global_effects::GlobalEffects;
use crate::rwac::PluginRef;
use crate::rwac::core::{AudioCore, PlugInConfig, PlugInKind};
use crate::rwac::stream_pool::GameStreamFiles;
use crate::speech::{CHANNEL_CAMERA_MAN, Sk8SpeechSystem, SpeakerObject, SpeechCtx, SpeechGameState, SpeechMessage, SpeechObjects, mission};
use crate::vault::Vault;

fn entry(priority: i32, context: usize) -> ValidContext {
    ValidContext { time_stamp: 0, context, valid: true, interrupt: false, triggered: true, priority, expiry: 1000 }
}

#[test]
fn merging_keeps_priority_order_and_puts_the_second_list_first_on_ties() {
    let mut merged = ValidContextList::new(10);
    merged.merge(&[entry(20, 0), entry(5, 1)], &[entry(20, 2), entry(0, 3)]);
    assert_eq!(merged.entries.iter().map(|e| e.context).collect::<Vec<_>>(), vec![2, 0, 1, 3]);
    let mut short = ValidContextList::new(3);
    short.merge(&[entry(9, 0), entry(8, 1)], &[entry(7, 2)]);
    assert_eq!(short.entries.len(), 2);
}

#[test]
fn the_random_state_carries_between_its_words() {
    let mut rand = ContextRand([0, 0, 0, 0xFFFF_FFFF, 1, 0xFFFF_FFFF]);
    let first = rand.next();
    assert_eq!(rand.0, [1, 1, 1, 0, 1, 0]);
    assert_eq!(first, 1);
    let seeded = ContextRand::new(1);
    assert_eq!(seeded, ContextRand::new(1));
    assert_ne!(seeded, ContextRand::new(2));
}

/// The single-player career values the training area runs with.
fn career_values() -> GameValues {
    GameValues {
        challenge_type: CHALLENGE_UNKNOWN,
        game_mode: 1,
        online_game: false,
        in_replay: false,
        player_providing_input: true,
        away_time: 0,
        is_male: true,
        fe_music_state: 4,
        scene_type: 0,
        paused: false,
        time_since_unpaused: 0,
        hud_state: 0,
        challenge_host: None,
        gvr: false,
        danny_way_jam: false,
        mega_challenge: false,
    }
}

fn game_files() -> Arc<dyn StreamFiles> {
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    Arc::new(GameStreamFiles::new(root.into()))
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_context_systems_load_their_contexts_scripts_and_triggers() {
    let files = game_files();
    let systems = ContextSystems::load(files.as_ref(), 1, 0);
    let spch = output_events(files.as_ref());
    for (group, contexts, passives) in [(Group::CameraMan, 217, 2), (Group::Announcer, 47, 2), (Group::MainCast, 98, 1)] {
        let system = &systems.systems[group as usize];
        eprintln!("{group:?}: {} contexts, {} triggers subscribed, {} passive", system.contexts.len(), system.subscribed.len(), system.passive_list.entries.len());
        assert_eq!((system.contexts.len(), system.passive_list.entries.len()), (contexts, passives));
        assert!(system.contexts.windows(2).all(|pair| pair[0].priority >= pair[1].priority));
        for context in &system.contexts {
            if let Some(name) = context.trigger_name() {
                assert!(system.subscribed.iter().any(|subscribed| subscribed == name), "{name} not subscribed");
            }
            let output = &context.output.as_ref().expect("every context has an output").interface;
            let name = output.name.as_deref().expect("named output");
            assert!(spch.csis_event(name, output.system_crc, output.interface_crc).is_some(), "{name} is no project's function");
        }
    }
}

/// Every mapped mission line's trigger exists in its system, and the parameter words the port
/// passes cover each of its contexts' trigger parameters.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mapped_triggers_take_the_parameters_their_contexts_declare() {
    let files = game_files();
    let systems = ContextSystems::load(files.as_ref(), 1, 0);
    let mut checked = 0;
    for (group, function, args) in mission::mapped_triggers() {
        let system = &systems.systems[group as usize];
        assert!(system.subscribed.iter().any(|name| name == function), "{function} is not a {group:?} trigger");
        for context in system.contexts.iter().filter(|context| context.trigger_name() == Some(function)) {
            let params = context.trigger.as_ref().map_or(0, |trigger| trigger.params.len());
            assert_eq!(params, args, "{} ({function})", context.name);
            checked += 1;
        }
    }
    eprintln!("{checked} triggered contexts checked");
    assert!(checked > 0);
}

/// The training contexts' conditions hold in the single-player career and fail online.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn training_contexts_hold_offline_in_single_player() {
    let files = game_files();
    let mut systems = ContextSystems::load(files.as_ref(), 1, 0);
    let mut rng = RwRandom::new(1);
    for (group, name) in [(Group::CameraMan, "Context4004"), (Group::CameraMan, "Context2727"), (Group::CameraMan, "Context712"), (Group::MainCast, "Context1011")] {
        let system = &mut systems.systems[group as usize];
        let index = system.contexts.iter().position(|context| context.name == name).unwrap_or_else(|| panic!("{name}"));
        let offline = career_values();
        let online = GameValues { online_game: true, ..offline };
        assert!(system.evaluate(index, 0, &mut GameInterface { values: &offline, rng: &mut rng }), "{name} offline");
        assert!(!system.evaluate(index, 0, &mut GameInterface { values: &online, rng: &mut rng }), "{name} online");
    }
}

/// `m_1a_05`'s reminder (`PlayProgressionReminder(4004)`): the cameraman's trigger picks
/// `Context4004`, whose `O_4004_Rmd_Atiba` the cameraman project plays on the voices instance;
/// once it ends the challenge hears `cMsgSpeechEnded`.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_cameramans_reminder_plays_and_ends() {
    let files = game_files();
    let mut core = AudioCore::new();
    let master = core
        .create_voice(
            255,
            "Mastering",
            &[
                PlugInConfig { kind: PlugInKind::SubMix { name: "Master".into() }, output_channels: 6 },
                PlugInConfig { kind: PlugInKind::SampleCapture, output_channels: 6 },
                PlugInConfig { kind: PlugInKind::Dac, output_channels: 6 },
            ],
        )
        .expect("mastering voice");
    let master = PluginRef { voice: master, index: 0 };
    let reverb = GlobalEffects::new(&mut core, master).reverb_sub_mix();
    core.stream_pools.set_files(Arc::clone(&files));
    let mut speech = Sk8SpeechSystem::new(&mut core, master, reverb, 0);
    Sk8SpeechSystem::create_stream_pool(&mut core);
    speech.set_files(Some(files));
    speech.create_context_systems(1, game_time(core.system_time));
    speech.load_projects("english");
    let vault = Vault::default();
    let mut rng = RwRandom::new(1);
    let values = career_values();
    let objects = SpeechObjects { camera_man: Some(SpeakerObject { vol: 0x7FFF, pitch: 4096, low_pass: 96000, ..SpeakerObject::default() }), ..SpeechObjects::default() };
    {
        let mut ctx = SpeechCtx { core: &mut core, vault: &vault, rng: &mut rng, master_sub_mix: master, reverb_sub_mix: reverb };
        speech.play_mission_speech_msg(40, 4004, 0, &mut ctx);
    }
    let (mut spoke_at, mut ended_at) = (None, None);
    for frame in 0..(30 * 48000 / 256) {
        {
            let mut ctx = SpeechCtx { core: &mut core, vault: &vault, rng: &mut rng, master_sub_mix: master, reverb_sub_mix: reverb };
            speech.update(&mut ctx, SpeechGameState { presentation_live: true, ..SpeechGameState::default() }, &objects);
            speech.update_contexts(&mut ctx, &values);
        }
        let _ = core.render_frame();
        std::thread::sleep(std::time::Duration::from_millis(1));
        if spoke_at.is_none() && speech.voices_speaking_on_channel(CHANNEL_CAMERA_MAN) {
            spoke_at = Some(frame);
        }
        if speech.take_messages().contains(&SpeechMessage::Ended { character: 40, id: 4004 }) {
            ended_at = Some(frame);
            break;
        }
    }
    eprintln!("spoke at block {spoke_at:?}, ended at block {ended_at:?}");
    assert!(spoke_at.is_some(), "the cameraman never spoke");
    assert!(ended_at > spoke_at, "the line never ended after it spoke");
}

/// The output speech projects' event databases, through which an emit adds its event.
fn output_events(files: &dyn StreamFiles) -> Spch {
    let mut spch = Spch::new(1, spch::sample_data_rate(22050, 16));
    for (channel, project) in [(0, "maincast"), (2, "cameraman"), (3, "announcer")] {
        if let Some(events) = read_file(files, &format!("data\\audio\\english\\{project}speech.big|{project}Output_Events.evt")) {
            spch.add_event_db(events, channel);
        }
    }
    spch
}

/// What each training line's trigger leads to in the photo challenges (`m_1a_05`, `m_1a_06`:
/// `ChallengeType` 38, Atiba hosting): the context that speaks within the trigger's expiry, by
/// its output event.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn photo_challenge_lines_pick_their_contexts() {
    let files = game_files();
    let mut systems = ContextSystems::load(files.as_ref(), 1, 0);
    let mut spch = output_events(files.as_ref());
    let mut rng = RwRandom::new(1);
    let photo = GameValues { challenge_type: 38, challenge_host: Some(38), ..career_values() };
    let main_cast = vec![0; 12];
    let lines: [(Group, &str, Vec<i32>, &[&str]); 8] = [
        (Group::CameraMan, "T_920_pickphot", vec![], &["O_920_pickphot"]),
        (Group::CameraMan, "T_104_slam_cam", vec![0], &[]),
        (Group::CameraMan, "T_712_replay", vec![2], &["O_712_replay"]),
        (Group::CameraMan, "T_4004_Rmd_Atiba", vec![], &["O_4004_Rmd_Atiba"]),
        (Group::CameraMan, "T_2727_SpnsrMeRmd", vec![], &["O_2727_SpnsrMeRmd"]),
        (Group::MainCast, "T_1003_almost", main_cast.clone(), &["O_1003_almost", "O_1005_sketch"]),
        (Group::MainCast, "T_1011_succ_PC", main_cast.clone(), &["O_1011_succ_PC"]),
        (Group::MainCast, "T_1015_failure", main_cast, &[]),
    ];
    let mut now = 10_000;
    for (group, trigger, args, expected) in lines {
        systems.trigger(group, trigger, &args, now);
        let mut spoke = None;
        for _ in 0..40 {
            let used: Vec<i32> = systems.systems[group as usize].contexts.iter().map(|c| c.times_used).collect();
            systems.update(now, &photo, &mut rng, &mut spch);
            let system = &systems.systems[group as usize];
            if let Some(index) = system.contexts.iter().zip(&used).position(|(c, &before)| c.times_used > before) {
                spoke = system.contexts[index].output.as_ref().and_then(|output| output.interface.name.clone());
                break;
            }
            now += 33;
        }
        eprintln!("{trigger}: {spoke:?}");
        match spoke {
            Some(output) => assert!(expected.contains(&output.as_str()), "{trigger} spoke {output}"),
            None => assert!(expected.is_empty(), "{trigger} spoke nothing"),
        }
        now += 10_000;
    }
}
