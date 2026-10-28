//! Main-thread, mod-owned Bevy audio adapter (API 2, audio extension 1).
//! Uses the existing DefaultPlugins AudioPlugin; Cargo enables bevy_audio + wav.
//! Playback speed changes pitch AND duration. This is not a time-stretch engine.
use super::{Mods, resolve_body};
use crate::game_audio::mod_voices::{MAX_NATIVE_VOICES, ModVoices, VoiceSpec};
use bevy::{
    audio::{AudioSinkPlayback, PlaybackMode, SpatialScale, Volume},
    prelude::*,
};
use skate_mods::audio::{AudioPlayOptions, AudioUpdateOptions, MAX_WAV_BYTES, canonical_pcm_wav};
use std::{collections::BTreeMap, path::Path};

type Key = (String, String);
const MAX_VOICES_PER_MOD: usize = 32;
const MAX_VOICES_TOTAL: usize = 128;
const MAX_CLIPS_PER_MOD: usize = 32;
const MAX_CLIPS_TOTAL: usize = 128;
const MAX_BYTES_PER_MOD: usize = 32 * 1024 * 1024;
const MAX_BYTES_TOTAL: usize = 128 * 1024 * 1024;

struct Clip {
    handle: Handle<AudioSource>,
    bytes: usize,
}
struct Voice {
    entity: Entity,
    body: Option<String>,
    position: Vec3,
    offset: Vec3,
    looping: bool,
    paused: bool,
    volume: f32,
    gain: f32,
    pitch: f32,
    speed: f32,
    attack: f32,
    attack_elapsed: f32,
    stopping: Option<(f32, f32)>, // remaining seconds, total seconds
    pending: f32,
    warned: bool,
}
/// A native voice's placement (audio extension 3: the voice itself lives in
/// `game_audio::mod_voices`, which the mod system feeds the resolved position every frame).
struct NativeVoice {
    body: Option<String>,
    position: Vec3,
    offset: Vec3,
    spatial: bool,
}
#[derive(Resource, Default)]
struct ModAudio {
    clips: BTreeMap<Key, Clip>,
    voices: BTreeMap<Key, Voice>,
    native: BTreeMap<Key, NativeVoice>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<ModAudio>();
    app.add_systems(
        Update,
        sync.after(super::update).after(super::present_camera),
    );
    app.add_systems(
        Update,
        session_marker_events
            .after(crate::app::FrameSet::Animation)
            .before(super::update),
    );
}

/// `sdk.audio.frontend(name)`: one of the game's front-end sounds, through the same message the
/// engine's UI sends (`crate::ui_audio::FrontendSound`). A one-shot: nothing to clean up when the
/// mod stops (retail's 10 front-end slots bound how many play at once).
pub(super) fn frontend(world: &mut World, name: &str) {
    world.write_message(crate::ui_audio::FrontendSound::named(name));
}

/// `sdk.audio.teleport_effect(amount)`: the teleport effect (`crate::ui_audio::TeleportEffect`, the
/// message the session marker's hold sends): the screen static and the skater's Class_Treatment
/// crackle. It lapses four UI ticks after the last send, so a stopped or disabled mod leaves nothing
/// behind.
pub(super) fn teleport_effect(world: &mut World, amount: f32) {
    let now = world.resource::<Time<Real>>().elapsed_secs_f64();
    if let Some(mut effect) = world.get_resource_mut::<crate::ui_audio::TeleportEffect>() {
        effect.set_from_mod(amount, now);
    }
}

/// The session marker's actions reach mods as `on_event {name = "session_marker", action =
/// "opened" | "placed" | "refused" | "returned"}`.
fn session_marker_events(
    mods: Option<ResMut<Mods>>,
    mut events: MessageReader<crate::ui_audio::SessionMarkerEvent>,
) {
    let Some(mut mods) = mods else {
        events.clear();
        return;
    };
    for e in events.read() {
        mods.manager.dispatch(
            "on_event",
            serde_json::json!({"name": "session_marker", "action": e.action.name()}),
        );
    }
}

fn ensure_clip(
    world: &mut World,
    audio: &mut ModAudio,
    owner: &str,
    root: &Path,
    path: &str,
) -> Result<Handle<AudioSource>, String> {
    let key = (owner.to_owned(), path.to_owned());
    if let Some(clip) = audio.clips.get(&key) {
        return Ok(clip.handle.clone());
    }
    if !skate_mods::audio::valid_audio_path(path) {
        return Err("Invalid audio path".into());
    }
    if audio.clips.len() >= MAX_CLIPS_TOTAL
        || audio.clips.keys().filter(|(o, _)| o == owner).count() >= MAX_CLIPS_PER_MOD
    {
        return Err("Audio cache limit reached (32 clips/mod, 128 total)".into());
    }
    // Existing sandbox reader canonicalizes paths and rejects symlink escapes.
    // This is done ONCE per clip, at preload/play, never in the update loop.
    let input = skate_mods::read_bounded(root, path, MAX_WAV_BYTES)?;
    let (bytes, _info) = canonical_pcm_wav(&input).map_err(|e| format!("{path}: {e}"))?;
    let total: usize = audio.clips.values().map(|c| c.bytes).sum();
    let owned: usize = audio
        .clips
        .iter()
        .filter(|((o, _), _)| o == owner)
        .map(|(_, c)| c.bytes)
        .sum();
    if total + bytes.len() > MAX_BYTES_TOTAL || owned + bytes.len() > MAX_BYTES_PER_MOD {
        return Err("Audio cache byte limit reached (32 MiB/mod, 128 MiB total)".into());
    }
    let size = bytes.len();
    let Some(mut assets) = world.get_resource_mut::<Assets<AudioSource>>() else {
        return Err(
            "AudioPlugin/PCM WAV support unavailable; rebuild with bevy_audio and wav features"
                .into(),
        );
    };
    let handle = assets.add(AudioSource {
        bytes: bytes.into(),
    });
    audio.clips.insert(
        key,
        Clip {
            handle: handle.clone(),
            bytes: size,
        },
    );
    Ok(handle)
}

pub(super) fn preload(
    world: &mut World,
    mods: &Mods,
    owner: &str,
    path: &str,
) -> Result<(), String> {
    let root = &mods
        .manager
        .packages
        .get(owner)
        .ok_or("Missing audio owner")?
        .root;
    world.resource_scope(|world, mut audio: Mut<ModAudio>| {
        ensure_clip(world, &mut audio, owner, root, path).map(|_| ())
    })
}
fn emitter_position(
    mods: &Mods,
    owner: &str,
    body: Option<&str>,
    position: Vec3,
    offset: Vec3,
) -> Option<Vec3> {
    if let Some(body) = body {
        let snapshot = mods.world.read(resolve_body(mods, owner, body).ok()?)?;
        let q = snapshot.rotation;
        Some(Vec3::from_array(snapshot.position) + Quat::from_xyzw(q[0], q[1], q[2], q[3]) * offset)
    } else {
        Some(position + offset)
    }
}
fn remove_voice(world: &mut World, audio: &mut ModAudio, key: &Key) {
    if audio.native.remove(key).is_some() {
        if let Some(mut voices) = world.get_resource_mut::<ModVoices>() {
            voices.stop(&key.0, &key.1, 0.0);
        }
    }
    if let Some(voice) = audio.voices.remove(key) {
        // Explicit stop also covers backend semantics where dropping a sink detaches it.
        if let Some(sink) = world.get::<AudioSink>(voice.entity) {
            sink.stop();
        }
        if let Some(sink) = world.get::<SpatialAudioSink>(voice.entity) {
            sink.stop();
        }
        world.despawn(voice.entity);
    }
}

/// Native voices of one owner / in all (`game_audio::mod_voices`).
fn native_usage(world: &World, owner: &str) -> (usize, usize) {
    world
        .get_resource::<ModVoices>()
        .map_or((0, 0), |v| v.voice_usage(owner))
}

/// A clip for the native mixer: read and checked once, under the same per-mod / total limits as
/// the Bevy clips (both caches count).
fn ensure_native_clip(
    world: &mut World,
    audio: &ModAudio,
    owner: &str,
    root: &Path,
    path: &str,
) -> Result<(), String> {
    let Some(voices) = world.get_resource::<ModVoices>() else {
        return Err("native audio is not available".into());
    };
    if voices.has_clip(owner, path) {
        return Ok(());
    }
    if !skate_mods::audio::valid_audio_path(path) {
        return Err("Invalid audio path".into());
    }
    let (mine, mine_bytes, all, all_bytes) = voices.clip_usage(owner);
    let bevy_mine = audio.clips.keys().filter(|(o, _)| o == owner).count();
    if all + audio.clips.len() >= MAX_CLIPS_TOTAL || mine + bevy_mine >= MAX_CLIPS_PER_MOD {
        return Err("Audio cache limit reached (32 clips/mod, 128 total)".into());
    }
    let input = skate_mods::read_bounded(root, path, MAX_WAV_BYTES)?;
    let (bytes, _info) = canonical_pcm_wav(&input).map_err(|e| format!("{path}: {e}"))?;
    let bevy_total: usize = audio.clips.values().map(|c| c.bytes).sum();
    let bevy_owned: usize = audio
        .clips
        .iter()
        .filter(|((o, _), _)| o == owner)
        .map(|(_, c)| c.bytes)
        .sum();
    if bevy_total + all_bytes + bytes.len() > MAX_BYTES_TOTAL
        || bevy_owned + mine_bytes + bytes.len() > MAX_BYTES_PER_MOD
    {
        return Err("Audio cache byte limit reached (32 MiB/mod, 128 MiB total)".into());
    }
    world
        .resource_mut::<ModVoices>()
        .add_clip(owner, path, &bytes)
}

/// Doc 16 L7: `owner`'s files changed while it runs (an audio-only reload). The native clips of
/// them are dropped (re-read at the next play / rule compile); false when the script holds a Bevy
/// clip of one (the Bevy voices cannot swap their source: the mod reloads as before).
pub(super) fn files_changed(world: &mut World, owner: &str, paths: &[String]) -> bool {
    let bevy = world.get_resource::<ModAudio>().is_some_and(|a| {
        paths
            .iter()
            .any(|p| a.clips.contains_key(&(owner.to_owned(), p.clone())))
    });
    if bevy {
        return false;
    }
    if let Some(mut voices) = world.get_resource_mut::<ModVoices>() {
        voices.forget_clips(owner, paths);
    }
    true
}

/// A rule's WAV (`sdk.audio.rule`): loaded into the mod's native bank under the same limits.
pub(super) fn load_native_clip(
    world: &mut World,
    mods: &Mods,
    owner: &str,
    path: &str,
) -> Result<(), String> {
    let root = &mods
        .manager
        .packages
        .get(owner)
        .ok_or("Missing audio owner")?
        .root;
    world.resource_scope(|world, audio: Mut<ModAudio>| {
        ensure_native_clip(world, &audio, owner, root, path)
    })
}

/// Whether a native voice can be had for `owner`'s `key` now: the native audio runs (with its
/// MixMap) and the key already holds a native voice or one of the native voices is free.
fn native_ready(world: &World, audio: &ModAudio, owner: &str, key: &str) -> bool {
    let running = world
        .get_resource::<crate::game_audio::Native>()
        .is_some_and(|n| n.mixmap.is_some())
        && world.get_resource::<ModVoices>().is_some();
    running
        && (audio
            .native
            .contains_key(&(owner.to_owned(), key.to_owned()))
            || native_usage(world, owner).1 < MAX_NATIVE_VOICES)
}

/// The native mixer (`sdk.audio.play`, the default since 2026-10-04): the WAV through the game's
/// own mixer.
fn play_native(
    world: &mut World,
    mods: &Mods,
    owner: &str,
    key: String,
    opts: AudioPlayOptions,
) -> Result<(), String> {
    if world
        .get_resource::<crate::game_audio::Native>()
        .is_none_or(|n| n.mixmap.is_none())
    {
        return Err(
            "native audio is not running (play with native = false for a Bevy voice)".into(),
        );
    }
    let root = &mods
        .manager
        .packages
        .get(owner)
        .ok_or("Missing audio owner")?
        .root;
    let origin = Vec3::from_array(opts.position.unwrap_or([0.0; 3]));
    let offset = Vec3::from_array(opts.offset);
    let position = emitter_position(mods, owner, opts.body.as_deref(), origin, offset)
        .ok_or("Audio emitter body does not exist")?;
    world.resource_scope(|world, mut audio: Mut<ModAudio>| {
        let slot = (owner.to_owned(), key);
        let (mine, all) = native_usage(world, owner);
        let bevy_mine = audio.voices.keys().filter(|(o, _)| *o == owner).count();
        let exists = audio.voices.contains_key(&slot) || audio.native.contains_key(&slot);
        if !exists
            && (audio.voices.len() + all >= MAX_VOICES_TOTAL
                || bevy_mine + mine >= MAX_VOICES_PER_MOD)
        {
            return Err("Audio voice limit reached (32 voices/mod, 128 total)".into());
        }
        if !audio.native.contains_key(&slot) && all >= MAX_NATIVE_VOICES {
            return Err(format!(
                "Native audio voice limit reached ({MAX_NATIVE_VOICES} in all)"
            ));
        }
        ensure_native_clip(world, &audio, owner, root, &opts.path)?;
        remove_voice(world, &mut audio, &slot);
        let spec = VoiceSpec {
            path: opts.path.clone(),
            looping: opts.looping,
            volume: opts.volume,
            pitch: opts.pitch,
            paused: opts.paused,
            fade_in: opts.fade_in,
            position: opts.spatial.then_some(position),
            // A positional sound without `falloff` gets the default reach (40 m, squared).
            reach: opts
                .spatial
                .then(|| crate::game_audio::mod_voices::Reach::from_falloff(opts.reach())),
            follow: None,
            reverb: opts.reverb.unwrap_or(true),
            group: u8::from(opts.group.as_deref() == Some("player")),
        };
        world
            .resource_mut::<ModVoices>()
            .play(owner, &slot.1, spec)?;
        audio.native.insert(
            slot,
            NativeVoice {
                body: opts.body,
                position: origin,
                offset,
                spatial: opts.spatial,
            },
        );
        Ok(())
    })
}

pub(super) fn play(
    world: &mut World,
    mods: &Mods,
    owner: &str,
    key: String,
    opts: AudioPlayOptions,
) -> Result<(), String> {
    if !opts.validate() {
        return Err("Invalid audio play options".into());
    }
    // Routing (user decision 2026-10-04): native by default; `native = true` insists on it (an
    // error without it); with the default a Bevy voice stands in when the native audio is not
    // running or its native voices are all taken; `native = false` is always the Bevy voice.
    match opts.native {
        Some(true) => return play_native(world, mods, owner, key, opts),
        None if native_ready(world, world.resource::<ModAudio>(), owner, &key) => {
            return play_native(world, mods, owner, key, opts);
        }
        _ => {}
    }
    let root = &mods
        .manager
        .packages
        .get(owner)
        .ok_or("Missing audio owner")?
        .root;
    let origin = Vec3::from_array(opts.position.unwrap_or([0.0; 3]));
    let offset = Vec3::from_array(opts.offset);
    let position = emitter_position(mods, owner, opts.body.as_deref(), origin, offset)
        .ok_or("Audio emitter body does not exist")?;
    world.resource_scope(|world, mut audio: Mut<ModAudio>| {
        let slot = (owner.to_owned(), key);
        let (native_mine, native_all) = native_usage(world, owner);
        if !audio.voices.contains_key(&slot)
            && !audio.native.contains_key(&slot)
            && (audio.voices.len() + native_all >= MAX_VOICES_TOTAL
                || audio.voices.keys().filter(|(o, _)| o == owner).count() + native_mine
                    >= MAX_VOICES_PER_MOD)
        {
            return Err("Audio voice limit reached (32 voices/mod, 128 total)".into());
        }
        let handle = ensure_clip(world, &mut audio, owner, root, &opts.path)?;
        remove_voice(world, &mut audio, &slot);
        let paused = opts.paused
            || mods.manager.snapshot["paused"].as_bool().unwrap_or(true)
            || mods.manager.snapshot["replay"].as_bool().unwrap_or(false);
        let initial_gain = if opts.fade_in > 0.0 { 0.0 } else { opts.volume };
        let settings = PlaybackSettings {
            mode: if opts.looping {
                PlaybackMode::Loop
            } else {
                PlaybackMode::Once
            },
            volume: Volume::Linear(initial_gain),
            speed: opts.pitch,
            paused,
            spatial: opts.spatial,
            spatial_scale: Some(SpatialScale::new(opts.spatial_scale)),
            ..Default::default()
        };
        let entity = world
            .spawn((
                AudioPlayer::new(handle),
                settings,
                Transform::from_translation(position),
            ))
            .id();
        audio.voices.insert(
            slot,
            Voice {
                entity,
                body: opts.body,
                position: origin,
                offset,
                looping: opts.looping,
                paused: opts.paused,
                volume: opts.volume,
                gain: initial_gain,
                pitch: opts.pitch,
                speed: opts.pitch,
                attack: opts.fade_in,
                attack_elapsed: 0.0,
                stopping: None,
                pending: 0.0,
                warned: false,
            },
        );
        Ok(())
    })
}

pub(super) fn update_voice(world: &mut World, owner: &str, key: &str, opts: AudioUpdateOptions) {
    let slot = (owner.to_owned(), key.to_owned());
    let native = {
        let mut audio = world.resource_mut::<ModAudio>();
        match audio.native.get_mut(&slot) {
            Some(n) => {
                if n.body.is_none() {
                    if let Some(x) = opts.position {
                        n.position = Vec3::from_array(x);
                    }
                }
                if let Some(x) = opts.offset {
                    n.offset = Vec3::from_array(x);
                }
                true
            }
            None => false,
        }
    };
    if native {
        if let Some(mut voices) = world.get_resource_mut::<ModVoices>() {
            voices.update(owner, key, opts.volume, opts.pitch, opts.paused);
        }
        return;
    }
    let mut audio = world.resource_mut::<ModAudio>();
    let Some(v) = audio.voices.get_mut(&(owner.to_owned(), key.to_owned())) else {
        return;
    };
    if v.stopping.is_some() {
        return;
    }
    // Commands already passed typed validation. Finished/missing keys are harmless.
    if let Some(x) = opts.volume {
        v.volume = x;
    }
    if let Some(x) = opts.pitch {
        v.pitch = x;
    }
    if let Some(x) = opts.paused {
        v.paused = x;
    }
    // World position is only meaningful for an unattached emitter.
    if v.body.is_none() {
        if let Some(x) = opts.position {
            v.position = Vec3::from_array(x);
        }
    }
    if let Some(x) = opts.offset {
        v.offset = Vec3::from_array(x);
    }
}

pub(super) fn stop(world: &mut World, owner: &str, key: &str, fade: f32) {
    world.resource_scope(|world, mut audio: Mut<ModAudio>| {
        let key = (owner.to_owned(), key.to_owned());
        if fade <= 0.0 {
            remove_voice(world, &mut audio, &key);
        } else if audio.native.contains_key(&key) {
            // The native voice fades in `game_audio::mod_voices` (repeated stops keep the first fade).
            if let Some(mut voices) = world.get_resource_mut::<ModVoices>() {
                voices.stop(&key.0, &key.1, fade);
            }
        } else if let Some(v) = audio.voices.get_mut(&key) {
            // Repeated stop calls must not extend the sound's lifetime.
            if v.stopping.is_none() {
                v.stopping = Some((fade, fade));
            }
        }
    });
}

pub(super) fn stop_body(world: &mut World, owner: &str, body: &str) {
    world.resource_scope(|world, mut audio: Mut<ModAudio>| {
        let keys: Vec<_> = audio
            .voices
            .iter()
            .filter(|((o, _), v)| o == owner && v.body.as_deref() == Some(body))
            .map(|(k, _)| k.clone())
            .chain(
                audio
                    .native
                    .iter()
                    .filter(|((o, _), v)| o == owner && v.body.as_deref() == Some(body))
                    .map(|(k, _)| k.clone()),
            )
            .collect();
        for key in keys {
            remove_voice(world, &mut audio, &key);
        }
    });
}

pub(super) fn stop_owner(world: &mut World, owner: &str, release_clips: bool) {
    world.resource_scope(|world, mut audio: Mut<ModAudio>| {
        let keys: Vec<_> = audio
            .voices
            .keys()
            .chain(audio.native.keys())
            .filter(|(o, _)| o == owner)
            .cloned()
            .collect();
        for key in keys {
            remove_voice(world, &mut audio, &key);
        }
        if let Some(mut voices) = world.get_resource_mut::<ModVoices>() {
            voices.stop_owner(owner, release_clips);
        }
        if release_clips {
            let keys: Vec<_> = audio
                .clips
                .keys()
                .filter(|(o, _)| o == owner)
                .cloned()
                .collect();
            for key in keys {
                if let Some(clip) = audio.clips.remove(&key) {
                    world
                        .resource_mut::<Assets<AudioSource>>()
                        .remove(clip.handle.id());
                }
            }
        }
    });
}

pub(super) fn clear(world: &mut World) {
    world.resource_scope(|world, mut audio: Mut<ModAudio>| {
        let keys: Vec<_> = audio
            .voices
            .keys()
            .chain(audio.native.keys())
            .cloned()
            .collect();
        for key in keys {
            remove_voice(world, &mut audio, &key);
        }
        if let Some(mut voices) = world.get_resource_mut::<ModVoices>() {
            voices.clear();
        }
        for (_, clip) in std::mem::take(&mut audio.clips) {
            world
                .resource_mut::<Assets<AudioSource>>()
                .remove(clip.handle.id());
        }
    });
}

fn set_sink(sink: &mut impl AudioSinkPlayback, volume: f32, pitch: f32, paused: bool) -> bool {
    sink.set_volume(Volume::Linear(volume));
    sink.set_speed(pitch);
    if paused {
        if !sink.is_paused() {
            sink.pause();
        }
    } else if sink.is_paused() {
        sink.play();
    }
    sink.empty()
}

fn sync(world: &mut World) {
    let dt = world.resource::<Time<Real>>().delta_secs().clamp(0.0, 0.25);
    let global_gain = world
        .get_resource::<GlobalVolume>()
        .map_or(1.0, |volume| volume.volume.to_linear());
    let global_gain = if global_gain.is_finite() {
        global_gain.clamp(0.0, 1.0)
    } else {
        0.0
    };
    // The spatial listener (shared with game audio) follows the camera in game_audio.
    let camera = world
        .query_filtered::<&Transform, With<crate::camera::GameplayCamera>>()
        .iter(world)
        .next()
        .copied();
    let paused = {
        let mods = world.resource::<Mods>();
        camera.is_none()
            || mods.manager.snapshot["paused"].as_bool().unwrap_or(true)
            || mods.manager.snapshot["replay"].as_bool().unwrap_or(false)
    };
    world.resource_scope(|world, mut audio: Mut<ModAudio>| {
        // Native voices: the resolved position (a body, or the position + offset); gone when the
        // voice ended in the mixer or its body is gone.
        if !audio.native.is_empty() {
            let mut gone = Vec::new();
            for (key, n) in &audio.native {
                let alive = world.get_resource::<ModVoices>().is_some_and(|v| v.has_voice(&key.0, &key.1));
                let position = emitter_position(world.resource::<Mods>(), &key.0, n.body.as_deref(), n.position, n.offset);
                match (alive, position) {
                    (true, Some(p)) => {
                        if n.spatial { world.resource_mut::<ModVoices>().set_position(&key.0, &key.1, p); }
                    }
                    _ => gone.push(key.clone()),
                }
            }
            for key in gone { remove_voice(world, &mut audio, &key); }
        }
        let mut remove = Vec::new();
        for (key, v) in &mut audio.voices {
            if world.get_entity(v.entity).is_err() { remove.push(key.clone()); continue; }
            let position = emitter_position(world.resource::<Mods>(), &key.0, v.body.as_deref(), v.position, v.offset);
            let Some(position) = position else { remove.push(key.clone()); continue; };
            if let Some(mut transform) = world.get_mut::<Transform>(v.entity) { transform.translation = position; }
            let active = !(paused || v.paused);
            if active { v.attack_elapsed += dt; }
            let attack = if v.attack <= 0.0 { 1.0 } else { (v.attack_elapsed / v.attack).min(1.0) };
            let mut release = 1.0;
            if let Some((remaining, total)) = &mut v.stopping {
                *remaining -= dt; release = (*remaining / *total).max(0.0);
                if *remaining <= 0.0 { remove.push(key.clone()); continue; }
            }
            // This smooths audible parameter steps only. It never touches dynamics.
            v.gain += (v.volume - v.gain) * (1.0 - (-dt / 0.012).exp());
            v.speed += (v.pitch - v.speed) * (1.0 - (-dt / 0.025).exp());
            let gain = (v.gain * attack * release).clamp(0.0, 1.0);
            let speed = v.speed.clamp(0.25, 4.0);
            // Keep initial settings current while Bevy has not yet created a sink.
            // Once started, only the sink calls below change actual playback.
            if let Some(mut settings) = world.get_mut::<PlaybackSettings>(v.entity) {
                settings.volume = Volume::Linear(gain); settings.speed = speed; settings.paused = !active;
            }
            let finished = if let Some(mut sink) = world.get_mut::<AudioSink>(v.entity) {
                Some(set_sink(&mut *sink, gain * global_gain, speed, !active))
            } else if let Some(mut sink) = world.get_mut::<SpatialAudioSink>(v.entity) {
                Some(set_sink(&mut *sink, gain * global_gain, speed, !active))
            } else { None };
            match finished {
                Some(true) => remove.push(key.clone()),
                Some(false) => { v.pending = 0.0; },
                None => {
                    if active { v.pending += dt; }
                    if v.pending > 1.0 && !v.looping { remove.push(key.clone()); }
                    if v.pending > 2.0 && !v.warned {
                        warn!("Lua audio {}:{} has no playback sink; check the output device and AudioPlugin", key.0, key.1);
                        v.warned = true;
                    }
                }
            }
        }
        for key in remove { remove_voice(world, &mut audio, &key); }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> World {
        let mut w = World::new();
        w.insert_resource(ModAudio::default());
        w.insert_resource(Assets::<AudioSource>::default());
        w
    }
    fn voice(w: &mut World, owner: &str, key: &str, body: Option<&str>) -> Entity {
        let entity = w.spawn_empty().id();
        w.resource_mut::<ModAudio>().voices.insert(
            (owner.into(), key.into()),
            Voice {
                entity,
                body: body.map(str::to_owned),
                position: Vec3::ZERO,
                offset: Vec3::ZERO,
                looping: true,
                paused: false,
                volume: 1.0,
                gain: 1.0,
                pitch: 1.0,
                speed: 1.0,
                attack: 0.0,
                attack_elapsed: 0.0,
                stopping: None,
                pending: 0.0,
                warned: false,
            },
        );
        entity
    }
    #[test]
    fn owners_and_bodies_are_isolated() {
        let mut w = world();
        let a = voice(&mut w, "a", "engine", Some("car"));
        let b = voice(&mut w, "b", "engine", Some("car"));
        stop_body(&mut w, "a", "car");
        assert!(w.get_entity(a).is_err());
        assert!(w.get_entity(b).is_ok());
        stop_owner(&mut w, "b", true);
        assert!(w.get_entity(b).is_err());
    }
    #[test]
    fn update_preserves_entity_and_stop_is_idempotent() {
        let mut w = world();
        let entity = voice(&mut w, "a", "engine", None);
        update_voice(
            &mut w,
            "a",
            "engine",
            AudioUpdateOptions {
                volume: Some(0.0),
                pitch: Some(2.0),
                ..Default::default()
            },
        );
        let v = &w.resource::<ModAudio>().voices[&("a".into(), "engine".into())];
        assert_eq!(v.entity, entity);
        assert_eq!(v.volume, 0.0);
        assert_eq!(v.pitch, 2.0);
        stop(&mut w, "a", "engine", 0.1);
        stop(&mut w, "a", "engine", 1.0);
        assert_eq!(
            w.resource::<ModAudio>().voices[&("a".into(), "engine".into())].stopping,
            Some((0.1, 0.1))
        );
        stop(&mut w, "a", "engine", 0.0);
        stop(&mut w, "a", "engine", 0.0);
        assert!(w.resource::<ModAudio>().voices.is_empty());
    }
    /// Native is the default (user decision 2026-10-04) where it can be had: without the native
    /// audio a default play takes the Bevy path.
    #[test]
    fn default_routing_needs_the_native_audio() {
        let mut w = world();
        assert!(
            !native_ready(&w, w.resource::<ModAudio>(), "a", "k"),
            "no native audio: the Bevy voice"
        );
        w.insert_resource(ModVoices::default());
        assert!(
            !native_ready(&w, w.resource::<ModAudio>(), "a", "k"),
            "no runtime yet"
        );
    }
    /// With the native audio running (data-gated) a default play is native until the 24 native
    /// voices are taken; a key that already holds a native voice keeps it.
    #[test]
    #[ignore = "needs the private install data"]
    fn default_routing_is_native_until_the_native_voices_are_taken() {
        use crate::game_audio::mod_voices::tests::{library, spec, tone_wav};
        let mut w = world();
        let library = library();
        w.insert_resource(
            crate::game_audio::Native::start_for_test(&library)
                .unwrap_or_else(|e| panic!("missing private data: {e}")),
        );
        w.insert_resource(ModVoices::default());
        assert!(native_ready(&w, w.resource::<ModAudio>(), "a", "k"));
        let wav = tone_wav(440.0, 0.1, 22050, 0.5);
        w.resource_mut::<ModVoices>()
            .add_clip("b", "t.wav", &wav)
            .unwrap();
        for i in 0..MAX_NATIVE_VOICES {
            w.resource_mut::<ModVoices>()
                .play("b", &format!("v{i}"), spec("t.wav", None))
                .unwrap();
        }
        assert!(
            !native_ready(&w, w.resource::<ModAudio>(), "a", "k"),
            "all native voices taken: the Bevy voice"
        );
        w.resource_mut::<ModAudio>().native.insert(
            ("a".into(), "k".into()),
            NativeVoice {
                body: None,
                position: Vec3::ZERO,
                offset: Vec3::ZERO,
                spatial: false,
            },
        );
        assert!(
            native_ready(&w, w.resource::<ModAudio>(), "a", "k"),
            "a key with a native voice keeps it"
        );
    }
    #[test]
    fn unloading_releases_cached_assets() {
        let mut w = world();
        let h = w.resource_mut::<Assets<AudioSource>>().add(AudioSource {
            bytes: Vec::<u8>::new().into(),
        });
        w.resource_mut::<ModAudio>().clips.insert(
            ("a".into(), "x.wav".into()),
            Clip {
                handle: h.clone(),
                bytes: 0,
            },
        );
        stop_owner(&mut w, "a", false);
        assert!(w.resource::<Assets<AudioSource>>().get(h.id()).is_some());
        stop_owner(&mut w, "a", true);
        assert!(w.resource::<Assets<AudioSource>>().get(h.id()).is_none());
    }
}
