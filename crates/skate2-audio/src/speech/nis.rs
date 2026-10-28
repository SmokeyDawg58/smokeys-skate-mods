//! `Sk8SpeechSystem`'s NIS samples: the resident NIS, post and front-end post samples
//! (`LoadEarlyResidentData` 0x824fa538, `LoadResidentData` 0x824fa7a8) whose `.sns` streams a
//! scene's ICE speech and post channels queue and play (`QueueNISSpeech` 0x824ead68,
//! `PlayNISSpeech` 0x824eafe0) in two playing slots (`playingNISSample`), and which the mission
//! speech from id 5000 plays too.
//!
//! A sample is queued on an instance's stream 0 with `mQueueSpeech` set, so `PlayData` starts its
//! request 1000 s ahead and the stream buffers (`SpeechQueued` once `SndPlayer1` reports the data
//! read); playing it is `StartQueuedSpeech`, a start time of 0 for the request. The post (character
//! `eSk8Character_SFXPost`) takes the 6-channel instance 2; speech takes the voices instance 1, or
//! the characters instance 0 while instance 1 plays.

use std::sync::Arc;

use skate2_formats::big::{BigArchive, BigEntryKey};

use super::spch::{EventSpec, NO_CALLBACK_DATA, SampleRequestData};
use super::{CHANNEL_ANNOUNCER, CHANNEL_CAMERA_MAN, CHANNEL_MAIN_CAST, INSTANCE_CHARACTERS, INSTANCE_POST, INSTANCE_VOICES, Sk8SpeechSystem, SpeechCtx};
use crate::rwac::decoder::DataRef;

/// `eSk8Characters` the NIS speech treats specially.
pub(super) const CHARACTER_DYRDEK: i32 = 13;
pub(super) const CHARACTER_CUZ: i32 = 35;
pub(super) const CHARACTER_CYRUS: i32 = 36;
pub(super) const CHARACTER_CAMERA_MAN: i32 = 40;
pub(super) const CHARACTER_MUSIC_POST: i32 = 98;
pub const CHARACTER_SFX_POST: i32 = 99;
const CHARACTER_MAX: i32 = 100;
/// The playing slots (`playingNISSample[2]`); 2 is none.
const SLOTS: usize = 2;
/// `mXMPCount`'s reload when an overriding post ends.
const XMP_COUNT: i32 = 20;
/// The front-end posts (`99_5300` to `99_5307`) play on past the scene.
const FE_POSTS: std::ops::RangeInclusive<i32> = 5300..=5307;
/// The film challenge types (`eChallengeTypes_Film`, `AnywhereFilm`, `SpotFilm`, `FriendFilm`,
/// `PickaProFilm`, `TeamFilm`).
const FILM_CHALLENGES: [i32; 6] = [7, 39, 40, 41, 46, 48];

/// `eNISSampleType`: the archive the sample streams from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NisSampleType {
    Nis,
    Post,
    FePost,
}

/// `Sk8::Audio::NISSample` (0x30 bytes): the sample's SNR header in its resident archive, its
/// `.sns` name, character, event and type.
#[derive(Clone)]
pub(super) struct NisSample {
    pub(super) snr: DataRef,
    pub(super) filename: String,
    pub(super) character: i32,
    pub(super) event: i32,
    pub(super) kind: NisSampleType,
}

/// `Sk8SpeechSystem::tPlayingNISSample`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PlayingNisSample {
    pub(super) character: i32,
    pub(super) id: i32,
    pub(super) instance: usize,
    pub(super) stream: usize,
}

impl Default for PlayingNisSample {
    /// The constructor's and `UpdatePlayingNISSpeech`'s free slot: no character, id -1.
    fn default() -> Self {
        Self { character: CHARACTER_MAX, id: -1, instance: 0, stream: 0 }
    }
}

/// The NIS sample state of `Sk8SpeechSystem`.
#[derive(Default)]
pub(super) struct NisSpeech {
    /// `mpNISBigFilePath`, `mpPostBigFilePath` and `mpFEPostBigFilePath`.
    nis_path: String,
    post_path: String,
    fe_post_path: String,
    /// `pNISSamples`: the NIS samples, then the post samples.
    samples: Vec<NisSample>,
    /// `pFEPostSamples`.
    fe_post_samples: Vec<NisSample>,
    /// `mEarlyLoadDone` and `mResidentLoadDone`.
    early_load_done: bool,
    resident_load_done: bool,
    playing: [PlayingNisSample; SLOTS],
    /// `mbPlayingNISPost`, `mbOverridingMusic`, `mXMPCount` and `mbDisablingVolume`.
    playing_post: bool,
    overriding_music: bool,
    pub(super) xmp_count: i32,
    disabling_volume: bool,
}

/// The samples of a resident archive (`rw::core::Big` over the loaded file): each entry's SNR
/// header in place, its name with the last letter made 's' (the `.sns` stream), the character
/// (`atol` of the first two characters) and the event (`atol` of characters 3 to 6).
fn resident_samples(bytes: Vec<u8>, kind: NisSampleType) -> Vec<NisSample> {
    let Ok(table) = BigArchive::parse(&bytes) else { return Vec::new() };
    let bytes: Arc<[u8]> = Arc::from(bytes);
    table
        .entries
        .iter()
        .filter_map(|entry| {
            let BigEntryKey::Name(name) = &entry.key else { return None };
            let mut filename = name.clone();
            filename.pop();
            filename.push('s');
            let field = |range: std::ops::Range<usize>| super::atoi(name.as_bytes().get(range).unwrap_or(&[]));
            Some(NisSample { snr: DataRef { bytes: Arc::clone(&bytes), offset: entry.offset as usize }, filename, character: field(0..2), event: field(3..7), kind })
        })
        .collect()
}

impl Sk8SpeechSystem {
    /// `xcore::FileLoader::SyncLoad` of a resident archive inside a mounted big.
    fn read_resident(&self, archive: &str, name: &str) -> Option<Vec<u8>> {
        let files = self.files.as_ref()?;
        let path = format!("{archive}{name}");
        let mut bytes = vec![0u8; usize::try_from(files.size(&path)?).ok()?];
        files.read_at(&path, 0, &mut bytes).then_some(bytes)
    }

    /// `LoadEarlyResidentData` 0x824fa538 (`AudioSystem::LoadInGameData`): `fepost.big`'s
    /// `fepostresident.big`.
    pub fn load_early_resident_data(&mut self) {
        self.nis_samples.fe_post_path = "data\\audio\\fepost.big|".to_owned();
        let Some(bytes) = self.read_resident(&self.nis_samples.fe_post_path, "fepostresident.big") else { return };
        self.nis_samples.fe_post_samples = resident_samples(bytes, NisSampleType::FePost);
        self.nis_samples.early_load_done = true;
    }

    /// `LoadResidentData` 0x824fa7a8 (`AudioSystem::SecondaryLoad`): the language's `nis.big`
    /// `nisresident.big`, then `post.big`'s `postresident.big`.
    pub fn load_resident_data(&mut self, language: &str) {
        self.nis_samples.nis_path = format!("data\\audio\\{language}\\nis.big|");
        self.nis_samples.post_path = "data\\audio\\post.big|".to_owned();
        let (Some(nis), Some(post)) = (self.read_resident(&self.nis_samples.nis_path, "nisresident.big"), self.read_resident(&self.nis_samples.post_path, "postresident.big"))
        else {
            return;
        };
        let mut samples = resident_samples(nis, NisSampleType::Nis);
        samples.extend(resident_samples(post, NisSampleType::Post));
        self.nis_samples.samples = samples;
        self.nis_samples.resident_load_done = true;
    }

    /// `GetNISSample` 0x824ea9c0: the NIS and post samples once loaded, then the front-end posts.
    pub(super) fn nis_sample(&self, character: i32, event: i32) -> Option<NisSample> {
        let find = |samples: &[NisSample]| samples.iter().find(|sample| sample.character == character && sample.event == event).cloned();
        if self.nis_samples.resident_load_done
            && let Some(sample) = find(&self.nis_samples.samples)
        {
            return Some(sample);
        }
        if !self.nis_samples.early_load_done {
            return None;
        }
        find(&self.nis_samples.fe_post_samples)
    }

    /// The archive a sample streams from (`mpNISBigFilePath`, `mpPostBigFilePath`,
    /// `mpFEPostBigFilePath`).
    pub(super) fn nis_sample_path(&self, kind: NisSampleType) -> String {
        match kind {
            NisSampleType::Nis => self.nis_samples.nis_path.clone(),
            NisSampleType::Post => self.nis_samples.post_path.clone(),
            NisSampleType::FePost => self.nis_samples.fe_post_path.clone(),
        }
    }

    /// The speech channel of a NIS or mission sample's character: the announcers' (Cuz, Cyrus),
    /// the cameraman's, else the main cast's.
    pub(super) fn nis_channel(character: i32) -> i32 {
        if (CHARACTER_CUZ..=CHARACTER_CYRUS).contains(&character) {
            CHANNEL_ANNOUNCER
        } else if character == CHARACTER_CAMERA_MAN {
            CHANNEL_CAMERA_MAN
        } else {
            CHANNEL_MAIN_CAST
        }
    }

    /// `SpeechInstance::PlayData` of a NIS sample on the instance's stream: the resident SNR
    /// header at offset 0 of its `.sns`, the event, and callback data 0xDEADBEEF.
    pub(super) fn play_nis_sample(&mut self, instance: usize, stream: usize, sample: &NisSample, id: i32, ctx: &mut SpeechCtx<'_>) {
        let data = SampleRequestData {
            bank_num: 0,
            sample_offset: 0,
            num_bytes: 0,
            event_spec: EventSpec { event_id: id as u16, ..EventSpec::default() },
            channel: Self::nis_channel(sample.character),
            sub_id: 0,
            dat_id: 0,
            interrupt_flag: 0,
            callback_data: NO_CALLBACK_DATA,
        };
        let path = self.nis_sample_path(sample.kind);
        self.play_data(instance, sample.snr.clone(), 0, &sample.filename, &path, data, stream, ctx);
    }

    /// `QueueNISSpeech` 0x824ead68: the sample in the first free slot, on stream 0 of the post
    /// instance (a post stops the characters and voices first), else of the voices instance while
    /// it is free, else of the characters instance; returns the slot, 2 without one or without
    /// the sample.
    pub fn queue_nis_speech(&mut self, character: i32, id: i32, ctx: &mut SpeechCtx<'_>) -> usize {
        let instance = if character == CHARACTER_SFX_POST {
            self.stop_all_speech_on_instance(INSTANCE_CHARACTERS, ctx);
            self.stop_all_speech_on_instance(INSTANCE_VOICES, ctx);
            INSTANCE_POST
        } else if self.speech_playing_on_instance_stream(INSTANCE_VOICES, 0) {
            INSTANCE_CHARACTERS
        } else {
            INSTANCE_VOICES
        };
        let Some(slot) = self.nis_samples.playing.iter().position(|playing| playing.character == CHARACTER_MAX) else { return SLOTS };
        let Some(sample) = self.nis_sample(character, id) else { return SLOTS };
        self.nis_samples.playing[slot] = PlayingNisSample { character, id, instance, stream: 0 };
        self.stop_all_speech_on_instance(instance, ctx);
        self.queue_speech(instance, 0, true);
        self.play_nis_sample(instance, 0, &sample, id, ctx);
        slot
    }

    /// The slot playing the character's sample, 2 without one.
    fn playing_nis_slot(&self, character: i32, id: i32) -> usize {
        self.nis_samples.playing.iter().position(|playing| playing.character == character && playing.id == id).unwrap_or(SLOTS)
    }

    /// `IsNISSpeechQueued` 0x824eaf20: a sample not in a slot counts as queued; one in a slot
    /// once its stream has buffered it (`SpeechQueued`).
    #[must_use]
    pub fn is_nis_speech_queued(&self, character: i32, id: i32, core: &crate::rwac::AudioCore) -> bool {
        let slot = self.playing_nis_slot(character, id);
        let Some(playing) = self.nis_samples.playing.get(slot) else { return true };
        self.speech_queued(playing.instance, playing.stream, core)
    }

    /// `PlayNISSpeech` 0x824eafe0: the queued sample (queued now when it is not) starts, and its
    /// stream stops queueing. A post plays as the NIS post; post id 1 also overrides the
    /// background music (`XMPOverrideBackgroundMusic`, no user music here) and mutes its stream.
    pub fn play_nis_speech(&mut self, character: i32, id: i32, ctx: &mut SpeechCtx<'_>) {
        let mut slot = self.playing_nis_slot(character, id);
        if slot == SLOTS {
            slot = self.queue_nis_speech(character, id, ctx);
            if slot == SLOTS {
                return;
            }
        }
        let PlayingNisSample { instance, stream, .. } = self.nis_samples.playing[slot];
        self.start_queued_speech(instance, stream, ctx.core);
        self.queue_speech(instance, stream, false);
        if character == CHARACTER_SFX_POST {
            self.nis_samples.playing_post = true;
            if id == 1 {
                self.nis_samples.overriding_music = true;
                self.disable_volume(instance, stream, true);
                self.nis_samples.disabling_volume = true;
            }
        }
    }

    /// The post's end (in `UpdatePlayingNISSpeech` and `FinishNIS`): an overriding post counts the
    /// background music back in (`mXMPCount` 20) and unmutes its stream.
    fn end_nis_post(&mut self, instance: usize, stream: usize) {
        if self.nis_samples.playing_post {
            if self.nis_samples.overriding_music {
                self.nis_samples.xmp_count = XMP_COUNT;
                self.nis_samples.overriding_music = false;
            }
            if self.nis_samples.disabling_volume {
                self.disable_volume(instance, stream, false);
                self.nis_samples.disabling_volume = false;
            }
        }
        self.nis_samples.playing_post = false;
    }

    /// `UpdatePlayingNISSpeech` 0x824eb170: a slot whose stream no longer plays is freed (a post
    /// ends the NIS post).
    pub(super) fn update_playing_nis_speech(&mut self) {
        for slot in 0..SLOTS {
            let playing = self.nis_samples.playing[slot];
            if playing.character == CHARACTER_MAX || self.speech_playing_on_instance_stream(playing.instance, playing.stream) {
                continue;
            }
            if playing.character == CHARACTER_SFX_POST {
                self.end_nis_post(playing.instance, playing.stream);
            }
            self.queue_speech(playing.instance, playing.stream, false);
            self.nis_samples.playing[slot] = PlayingNisSample::default();
        }
    }

    /// `FinishNIS` 0x824faec0 (`AudioSystem::NotifyNISPlaybackFinish`): each slot's sample starts
    /// if it still waits, and stops with its instance when `ShouldStopNISSpeech` says so; the NIS
    /// post ends. The slots free themselves once their streams stop.
    pub fn finish_nis(&mut self, ctx: &mut SpeechCtx<'_>) {
        for slot in 0..SLOTS {
            let playing = self.nis_samples.playing[slot];
            if playing.character == CHARACTER_MAX {
                continue;
            }
            self.start_queued_speech(playing.instance, playing.stream, ctx.core);
            self.queue_speech(playing.instance, playing.stream, false);
            if self.should_stop_nis_speech(playing.character, playing.id) {
                self.stop_all_speech_on_instance(playing.instance, ctx);
            }
            self.end_nis_post(playing.instance, playing.stream);
        }
    }

    /// `ShouldStopNISSpeech` 0x824fac80: the announcers always stop and the posts but the
    /// front end's; the cameraman (and Dyrdek in a challenge he hosts) stop in a film challenge.
    fn should_stop_nis_speech(&self, character: i32, id: i32) -> bool {
        match character {
            CHARACTER_DYRDEK | CHARACTER_CAMERA_MAN => {
                if self.game.challenge_host != Some(CHARACTER_DYRDEK) && character != CHARACTER_CAMERA_MAN {
                    return false;
                }
                FILM_CHALLENGES.contains(&self.game.challenge_type)
            }
            CHARACTER_CUZ | CHARACTER_CYRUS => true,
            CHARACTER_SFX_POST => !FE_POSTS.contains(&id),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global_effects::GlobalEffects;
    use crate::random::RwRandom;
    use crate::rwac::PluginRef;
    use crate::rwac::core::{AudioCore, PlugInConfig, PlugInKind};
    use crate::rwac::stream_pool::{GameStreamFiles, StreamFiles};
    use crate::speech::{NisSpeechObject, SpeechGameState, SpeechObjects};
    use crate::vault::Vault;

    /// A `BIGF` archive of `entries` (name, data) as the resident archives are laid out.
    fn big(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let header = 16 + entries.iter().map(|(name, _)| 8 + name.len() + 1).sum::<usize>();
        let mut table = Vec::new();
        let mut data = Vec::new();
        for (name, bytes) in entries {
            table.extend_from_slice(&u32::try_from(header + data.len()).unwrap().to_be_bytes());
            table.extend_from_slice(&u32::try_from(bytes.len()).unwrap().to_be_bytes());
            table.extend_from_slice(name.as_bytes());
            table.push(0);
            data.extend_from_slice(bytes);
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"BIGF");
        out.extend_from_slice(&u32::try_from(header + data.len()).unwrap().to_le_bytes());
        out.extend_from_slice(&u32::try_from(entries.len()).unwrap().to_be_bytes());
        out.extend_from_slice(&u32::try_from(header).unwrap().to_be_bytes());
        out.extend_from_slice(&table);
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn resident_samples_name_their_streams_characters_and_events() {
        let bytes = big(&[("04_5003_3b_01_dway_big.snr", b"head"), ("99_5002_story.snr", b"post")]);
        let samples = resident_samples(bytes, NisSampleType::Post);
        let read: Vec<_> = samples.iter().map(|s| (s.filename.as_str(), s.character, s.event, s.kind)).collect();
        assert_eq!(read, [("04_5003_3b_01_dway_big.sns", 4, 5003, NisSampleType::Post), ("99_5002_story.sns", 99, 5002, NisSampleType::Post)]);
        assert_eq!(&samples[1].snr.slice()[..4], b"post");
    }

    #[test]
    fn nis_samples_speak_on_their_speakers_channels() {
        assert_eq!(Sk8SpeechSystem::nis_channel(CHARACTER_CAMERA_MAN), CHANNEL_CAMERA_MAN);
        assert_eq!(Sk8SpeechSystem::nis_channel(CHARACTER_CYRUS), CHANNEL_ANNOUNCER);
        assert_eq!(Sk8SpeechSystem::nis_channel(CHARACTER_SFX_POST), CHANNEL_MAIN_CAST);
    }

    /// A speech system on a core with the mastering voice and the global effects, reading the game
    /// root's files, with the resident samples loaded.
    fn speech_system(root: &str) -> (AudioCore, Sk8SpeechSystem, PluginRef, PluginRef) {
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
            .unwrap();
        let master = PluginRef { voice: master, index: 0 };
        let reverb = GlobalEffects::new(&mut core, master).reverb_sub_mix();
        let files: Arc<dyn StreamFiles> = Arc::new(GameStreamFiles::new(root.into()));
        core.stream_pools.set_files(Arc::clone(&files));
        let mut speech = Sk8SpeechSystem::new(&mut core, master, reverb, 0);
        Sk8SpeechSystem::create_stream_pool(&mut core);
        speech.set_files(Some(files));
        speech.load_early_resident_data();
        speech.load_resident_data("english");
        (core, speech, master, reverb)
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn the_resident_samples_hold_the_scenes_posts_and_lines() {
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let (_, speech, _, _) = speech_system(&root);
        let (resident, fe_posts) = (speech.nis_samples.samples.len(), speech.nis_samples.fe_post_samples.len());
        eprintln!("{resident} NIS and post samples, {fe_posts} front-end posts");
        assert!(resident > 0 && fe_posts > 0);
        for (character, id) in [(99, 5002), (99, 5003), (40, 5008), (40, 5012), (99, 5300)] {
            let sample = speech.nis_sample(character, id).unwrap_or_else(|| panic!("{character}_{id}"));
            eprintln!("{character}_{id}: {} ({:?})", sample.filename, sample.kind);
        }
    }

    /// `02_Pros_Arrival`'s post: queued, it buffers until `IsNISSpeechQueued`; played, the post
    /// voice renders it at the NIS volume and frees its slot once the stream ends.
    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn a_story_post_buffers_then_plays() {
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let (mut core, mut speech, master, reverb) = speech_system(&root);
        let vault = Vault::default();
        let mut rng = RwRandom::new(1);
        let objects = SpeechObjects {
            nis: Some(NisSpeechObject { scripted_speech_vol: 0x7FFF, scripted_speech_pitch: 4096, nis_vol: 0x7FFF, ..NisSpeechObject::default() }),
            ..SpeechObjects::default()
        };
        let update = |core: &mut AudioCore, speech: &mut Sk8SpeechSystem, rng: &mut RwRandom| {
            let mut ctx = SpeechCtx { core, vault: &vault, rng, master_sub_mix: master, reverb_sub_mix: reverb };
            speech.update(&mut ctx, SpeechGameState::default(), &objects);
        };
        {
            let mut ctx = SpeechCtx { core: &mut core, vault: &vault, rng: &mut rng, master_sub_mix: master, reverb_sub_mix: reverb };
            assert_eq!(speech.queue_nis_speech(CHARACTER_SFX_POST, 5002, &mut ctx), 0);
        }
        let mut frames = 0;
        while !speech.is_nis_speech_queued(CHARACTER_SFX_POST, 5002, &core) {
            update(&mut core, &mut speech, &mut rng);
            let _ = core.render_frame();
            std::thread::sleep(std::time::Duration::from_millis(1));
            frames += 1;
            assert!(frames < 5000, "never buffered");
        }
        let silent = core.render_frame().iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert_eq!(silent, 0.0);
        {
            let mut ctx = SpeechCtx { core: &mut core, vault: &vault, rng: &mut rng, master_sub_mix: master, reverb_sub_mix: reverb };
            speech.play_nis_speech(CHARACTER_SFX_POST, 5002, &mut ctx);
        }
        let mut peak = 0.0f32;
        for _ in 0..(48000 / 256) {
            update(&mut core, &mut speech, &mut rng);
            peak = core.render_frame().iter().fold(peak, |m, x| m.max(x.abs()));
        }
        eprintln!("buffered after {frames} frames, peak {peak} in the first second");
        assert!(peak > 0.0);
        assert_eq!(speech.nis_samples.playing[0].id, 5002);
    }
}
