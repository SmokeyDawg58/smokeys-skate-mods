//! `Sk8::Audio::SpeechSystem` and `SpeechInstance` (speechsystem.cpp): three instances of two
//! streams, each stream a voice of the instance's style that plays one `spch` sample request at
//! a time from its project's `.dat` through the 'Spch' stream pool, its plug-ins following the
//! speaking object's `VoiceInfo`.

use std::sync::Arc;

use skate2_formats::big::{BigArchive, BigEntryKey};

use super::spch::SampleRequestData;
use super::{Sk8SpeechSystem, SpeechCtx, VoiceInfoRef, STREAMS, STREAM_POOL};
use crate::rwac::core::{AudioCore, PlugInConfig, PlugInKind};
use crate::rwac::decoder::DataRef;
use crate::rwac::plugins::snd_player::{PlayParams, SndPlayer1};
use crate::rwac::{PluginRef, VoiceId};

/// The mixes the instances' voices send to: `AudioCore::GetSpeechMasterSubMix`,
/// `GetMasterSubMix` and `Sk8SpeechSystem::GetReverbVoice`.
#[derive(Clone, Copy)]
pub(super) struct SubMixes {
    pub speech_master: Option<PluginRef>,
    pub master: PluginRef,
    pub reverb: PluginRef,
}

/// `eVoiceStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VoiceStyle {
    Stereo,
    SixChannel,
    DistanceBased,
    Disabled,
}

/// `Sk8SpeechSystem::GetVoiceStyle` 0x824eb3b0: the characters' streams are distance based; the
/// voices instance has a stereo stream, the post instance a 6-channel one.
fn voice_style(instance: usize, stream: usize) -> VoiceStyle {
    match (instance, stream) {
        (0, _) => VoiceStyle::DistanceBased,
        (1, 0) => VoiceStyle::Stereo,
        (2, 0) => VoiceStyle::SixChannel,
        _ => VoiceStyle::Disabled,
    }
}

/// `SpeechPlugInIndex` slots of `mplugInIndexs`.
const SND_PLAYER: usize = 0;
const RESAMPLE: usize = 2;
const NOTCH: usize = 3;
const SEND_DELAY: usize = 4;
const GAIN: usize = 5;
const HIGH_PASS: usize = 6;
const LOW_PASS: usize = 7;
const SEND_WET: usize = 8;
const PAN: usize = 9;
const SEND: usize = 10;
/// `SpeechDelayPlugInIndex`.
const DELAY_HIGH_PASS: u8 = 1;
const DELAY_DELAY: u8 = 2;
const DELAY_LOW_PASS: u8 = 3;
const DELAY_SEND: u8 = 5;
/// `1 / 32767`, `360 / 65535` and `1 / 4096`.
const VOLUME_SCALE: f32 = f32::from_bits(0x3800_0100);
const AZIMUTH_SCALE: f32 = f32::from_bits(0x3BB4_00B4);
const PITCH_SCALE: f32 = f32::from_bits(0x3980_0000);
/// The longest delay a delay voice takes (its `Delay` constructor parameter).
const MAX_DELAY: f32 = f32::from_bits(0x3E19_999A);

/// `tSpeechProject`.
#[derive(Debug, Clone, Copy)]
struct SpeechProject {
    loaded: bool,
    channel: i32,
    sample_requested: bool,
    sample_request: Option<SampleRequestData>,
}

/// `SpeechBank`: a `.hdr`'s `.sth` (sample records: the `.dat` offset, then its SNR header) and
/// its `.dat`.
#[derive(Clone)]
struct SpeechBank {
    sth: DataRef,
    filename: String,
    handle: i32,
}

/// `SpeechInstance::tSpeechBank`: a project's banks in handle order.
#[derive(Clone)]
struct BankSet {
    channel: i32,
    banks: Vec<SpeechBank>,
    project_name: String,
}

/// `Sk8::Audio::SpeechInstance` (0x358 bytes).
pub(super) struct SpeechInstance {
    plug_in_indexes: [[i32; 11]; STREAMS],
    projects: [SpeechProject; 2],
    styles: [VoiceStyle; STREAMS],
    bank_sets: [BankSet; 2],
    last_handle: i32,
    current_event_ids: [u16; STREAMS],
    current_trigger_events: [i32; STREAMS],
    just_finished: [bool; STREAMS],
    is_playing: [bool; STREAMS],
    cur_channels: [i32; STREAMS],
    voices: [Option<VoiceId>; STREAMS],
    play_request_handles: [f32; STREAMS],
    delay_voices: [Option<VoiceId>; STREAMS],
    voice_infos: [Option<VoiceInfoRef>; STREAMS],
    last_delays: [f32; STREAMS],
    is_queueing: [bool; STREAMS],
    queue_speech: [bool; STREAMS],
    is_buffering: [bool; STREAMS],
    hold_on_finish: [bool; STREAMS],
    is_held: [bool; STREAMS],
    disable_volume: [bool; STREAMS],
}

impl SpeechInstance {
    /// The constructor 0x82d40a98: no project or bank, each stream's voice in its style.
    pub(super) fn new(index: usize, sub_mixes: &SubMixes, core: &mut AudioCore) -> Self {
        let project = SpeechProject { loaded: false, channel: -1, sample_requested: false, sample_request: None };
        let bank_set = BankSet { channel: -1, banks: Vec::new(), project_name: String::new() };
        let mut instance = Self {
            plug_in_indexes: [[-1; 11]; STREAMS],
            projects: [project; 2],
            styles: [VoiceStyle::Disabled; STREAMS],
            bank_sets: [bank_set.clone(), bank_set],
            last_handle: -1,
            current_event_ids: [0; STREAMS],
            current_trigger_events: [-1; STREAMS],
            just_finished: [false; STREAMS],
            is_playing: [false; STREAMS],
            cur_channels: [-1; STREAMS],
            voices: [None; STREAMS],
            play_request_handles: [f32::NAN; STREAMS],
            delay_voices: [None; STREAMS],
            voice_infos: [None; STREAMS],
            last_delays: [0.0; STREAMS],
            is_queueing: [false; STREAMS],
            queue_speech: [false; STREAMS],
            is_buffering: [false; STREAMS],
            hold_on_finish: [false; STREAMS],
            is_held: [false; STREAMS],
            disable_volume: [false; STREAMS],
        };
        for stream in 0..STREAMS {
            instance.create_voice(voice_style(index, stream), stream, sub_mixes, core);
        }
        instance
    }

    fn plugin(&self, stream: usize, slot: usize) -> Option<PluginRef> {
        let index = self.plug_in_indexes[stream][slot];
        Some(PluginRef { voice: self.voices[stream]?, index: u8::try_from(index).ok()? })
    }

    /// `CreateVoice` 0x82d40368: stereo (to the speech master), 6-channel or distance based (with
    /// a notch and a delay voice, to the master), the wet send to the reverb.
    fn create_voice(&mut self, style: VoiceStyle, stream: usize, sub_mixes: &SubMixes, core: &mut AudioCore) {
        self.styles[stream] = style;
        let config = |kind: PlugInKind, output_channels: u8| PlugInConfig { kind, output_channels };
        let pan = || PlugInKind::Pan2D1 { constructor: Some((45.0, 135.0, 0.0)) };
        let (name, configs, indexes): (&'static str, Vec<PlugInConfig>, [i32; 11]) = match style {
            VoiceStyle::Disabled => return,
            VoiceStyle::Stereo | VoiceStyle::SixChannel => {
                let channels = if style == VoiceStyle::Stereo { 2 } else { 6 };
                (
                    if style == VoiceStyle::Stereo { "SpeechInstance::eVoiceStyle_Stereo" } else { "SpeechInstance::eVoiceStyle_6Channel" },
                    vec![
                        config(PlugInKind::SndPlayer1 { max_requests: 1.0 }, channels),
                        config(PlugInKind::Rechannel, channels),
                        config(PlugInKind::Resample, channels),
                        config(PlugInKind::Gain, channels),
                        config(PlugInKind::HighPassIir2, channels),
                        config(PlugInKind::LowPassIir2, channels),
                        config(PlugInKind::Send, channels),
                        config(pan(), 6),
                        config(PlugInKind::Send, 6),
                    ],
                    [0, 1, 2, -1, -1, 3, 4, 5, 6, 7, 8],
                )
            }
            VoiceStyle::DistanceBased => (
                "SpeechInstance::eVoiceStyle_DistanceBased",
                vec![
                    config(PlugInKind::SndPlayer1 { max_requests: 1.0 }, 2),
                    config(PlugInKind::Rechannel, 2),
                    config(PlugInKind::Resample, 2),
                    config(PlugInKind::PeakingIir2, 2),
                    config(PlugInKind::Send, 2),
                    config(PlugInKind::Gain, 2),
                    config(PlugInKind::HighPassIir2, 2),
                    config(PlugInKind::LowPassIir2, 2),
                    config(PlugInKind::Send, 2),
                    config(pan(), 6),
                    config(PlugInKind::Send, 6),
                ],
                [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            ),
        };
        self.plug_in_indexes[stream] = indexes;
        self.voices[stream] = core.create_voice(0, name, &configs);
        let main = match style {
            VoiceStyle::Stereo => sub_mixes.speech_master,
            _ => Some(sub_mixes.master),
        };
        match style {
            VoiceStyle::SixChannel => {
                if let Some(pan) = self.plugin(stream, PAN) {
                    core.set_attribute(pan, 1, 0.0);
                    core.set_attribute(pan, 2, 1000.0);
                }
            }
            VoiceStyle::DistanceBased => {
                if let Some(notch) = self.plugin(stream, NOTCH) {
                    core.set_attribute(notch, 2, 3.0);
                }
                self.create_delay_voice(stream, sub_mixes, core);
            }
            _ => {}
        }
        if let Some(send) = self.plugin(stream, SEND) {
            core.connect_send(send, main);
        }
        if let Some(wet) = self.plugin(stream, SEND_WET) {
            core.connect_send(wet, Some(sub_mixes.reverb));
        }
    }

    /// `CreateDelayVoice` 0x82d3fb40: a stage 2 voice (sub mix, high pass, a delay of at most
    /// 0.15 s, low pass, pan, send to the master) the stream's delay send feeds.
    pub(super) fn create_delay_voice(&mut self, stream: usize, sub_mixes: &SubMixes, core: &mut AudioCore) {
        if self.plug_in_indexes[stream][SEND_DELAY] == -1 || self.delay_voices[stream].is_some() {
            return;
        }
        let config = |kind: PlugInKind, output_channels: u8| PlugInConfig { kind, output_channels };
        self.delay_voices[stream] = core.create_voice(
            2,
            "SpeechInstance delay",
            &[
                config(PlugInKind::SubMix { name: String::new() }, 2),
                config(PlugInKind::HighPassIir2, 2),
                config(PlugInKind::Delay { max_time: Some(MAX_DELAY) }, 2),
                config(PlugInKind::LowPassIir2, 2),
                config(PlugInKind::Pan2D1 { constructor: None }, 6),
                config(PlugInKind::Send, 6),
            ],
        );
        let Some(delay) = self.delay_voices[stream] else { return };
        core.connect_send(PluginRef { voice: delay, index: DELAY_SEND }, Some(sub_mixes.master));
        if let Some(send) = self.plugin(stream, SEND_DELAY) {
            core.connect_send(send, Some(PluginRef { voice: delay, index: 0 }));
        }
    }

    /// `DestroyDelayVoice` 0x82d3fd28: the stream's delay send loses its voice, which is released.
    pub(super) fn destroy_delay_voice(&mut self, stream: usize, core: &mut AudioCore) {
        if self.plug_in_indexes[stream][SEND_DELAY] == -1 {
            return;
        }
        let Some(delay) = self.delay_voices[stream].take() else { return };
        if let Some(send) = self.plugin(stream, SEND_DELAY) {
            core.connect_send(send, None);
        }
        core.release_voice(delay);
    }

    /// `SetSubMix` 0x82d3e6f0: the stream's main send goes to `sub_mix`.
    pub(super) fn set_sub_mix(&self, sub_mix: Option<PluginRef>, stream: usize, core: &mut AudioCore) {
        if let Some(send) = self.plugin(stream, SEND) {
            core.connect_send(send, sub_mix);
        }
    }

    /// `SpeechPlaying` 0x824e2630.
    pub(super) fn speech_playing(&self, stream: usize) -> bool {
        self.is_playing[stream]
    }

    /// `SpeechActuallyPlaying` 0x824e26a0.
    pub(super) fn speech_actually_playing(&self, stream: usize) -> bool {
        self.is_playing[stream] && !self.is_queueing[stream]
    }

    /// `SpeechQueued` 0x82d3fdc0: a buffering stream whose request `SndPlayer1` event 3 reports
    /// buffered.
    fn speech_queued(&self, stream: usize, core: &AudioCore) -> bool {
        if !self.is_buffering[stream] {
            return false;
        }
        self.plugin(stream, SND_PLAYER).is_some_and(|player| core.request_buffered(player, self.play_request_handles[stream]))
    }

    /// `StartQueuedSpeech` 0x82d3feb8: a buffering stream's request starts at once (`SndPlayer1`
    /// event 4 with start time 0) and the stream stops buffering.
    fn start_queued_speech(&mut self, stream: usize, core: &mut AudioCore) {
        if !self.is_buffering[stream] {
            return;
        }
        if let Some(player) = self.plugin(stream, SND_PLAYER) {
            core.modify_start_time(player, self.play_request_handles[stream], 0.0);
        }
        self.is_buffering[stream] = false;
    }

    /// `mCurChannel[stream]` (+0x308), -1 while idle.
    pub(super) fn cur_channel(&self, stream: usize) -> i32 {
        self.cur_channels[stream]
    }

    /// `pVoiceInfos[stream]`.
    pub(super) fn voice_info(&self, stream: usize) -> Option<VoiceInfoRef> {
        self.voice_infos[stream]
    }

    /// `PlayRequest` 0x82d3e5f8: the request waits in its project.
    fn play_request(&mut self, data: SampleRequestData) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.loaded && project.channel == data.channel) {
            project.sample_request = Some(data);
            project.sample_requested = true;
        }
    }

    /// `ChooseSampleRequestData` 0x82d3e528: the request of the project with the higher
    /// `SpeechSystem::GetEventPriority` 0x82d3e8e8 (2 minus its slot).
    fn choose_sample_request_data(&self) -> Option<SampleRequestData> {
        let mut best_priority = -1;
        let mut best = None;
        for project in &self.projects {
            if project.loaded && project.sample_requested {
                let slot = self.projects.iter().position(|other| other.loaded && other.channel == project.channel).unwrap_or(2);
                let priority = 2 - slot as i32;
                if priority > best_priority {
                    best_priority = priority;
                    best = project.sample_request;
                }
            }
        }
        best
    }

    /// `FindBank` 0x82d3e450: the bank with the handle (binary search) among the channel's.
    fn find_bank(&self, handle: i32, channel: i32) -> Option<(usize, &SpeechBank)> {
        let set = self.bank_sets.iter().position(|set| set.channel == channel)?;
        let banks = &self.bank_sets[set].banks;
        let index = banks.binary_search_by(|bank| bank.handle.cmp(&handle)).ok()?;
        Some((set, &banks[index]))
    }

    /// `IsRequestDone` 0x82d3f5f0: an idle stream is done; a playing one once `SndPlayer1` event
    /// 2 reports its request done.
    pub(super) fn request_done(&self, stream: usize, core: &AudioCore) -> bool {
        if !self.is_playing[stream] {
            return true;
        }
        let Some(player) = self.plugin(stream, SND_PLAYER) else { return true };
        core.plugin::<SndPlayer1>(player).is_none_or(|player| player.request_done(self.play_request_handles[stream]))
    }
}

impl Sk8SpeechSystem {
    /// `SpeechInstance::Load` 0x82d3e958: the project's event database (`SPCH_AddEventDB`) and its
    /// banks: each `.hdr` of `<project>hdr.big` (`SPCH_AddBank`, handles rising) with the `.sth` at
    /// the same place in `<project>sth.big`, named after the `.hdr` with a `dat` extension. The
    /// `<project>Output_Events.csi` functions the context systems call are the event database's
    /// Csis table (`iSPCH_InitCsis`).
    pub(super) fn load(&mut self, instance: usize, channel: i32, project: &str, language: &str) {
        let Some(files) = self.files.clone() else { return };
        let archive = format!("data\\audio\\{language}\\{project}speech.big|");
        let read = |name: String| -> Option<Vec<u8>> {
            let path = format!("{archive}{name}");
            let mut bytes = vec![0u8; usize::try_from(files.size(&path)?).ok()?];
            files.read_at(&path, 0, &mut bytes).then_some(bytes)
        };
        let Some(slot) = self.instances[instance].projects.iter().position(|project| !project.loaded) else { return };
        self.instances[instance].projects[slot].channel = channel;
        let Some(events) = read(format!("{project}Output_Events.evt")) else { return };
        self.spch.add_event_db(events, channel as u32);
        let (Some(hdr), Some(sth)) = (read(format!("{project}hdr.big")), read(format!("{project}sth.big"))) else { return };
        let (Ok(hdr_table), Ok(sth_table)) = (BigArchive::parse(&hdr), BigArchive::parse(&sth)) else { return };
        if hdr_table.entries.len() != sth_table.entries.len() {
            return;
        }
        let Some(set) = self.instances[instance].bank_sets.iter().position(|set| set.channel == -1) else { return };
        let sth: Arc<[u8]> = Arc::from(sth);
        let mut banks = Vec::with_capacity(hdr_table.entries.len());
        for (hdr_entry, sth_entry) in hdr_table.entries.iter().zip(&sth_table.entries) {
            let start = hdr_entry.offset as usize;
            let end = start + hdr_entry.stored_size as usize;
            let handle = self.spch.add_bank(hdr.get(start..end).unwrap_or(&[]).to_vec());
            self.instances[instance].last_handle = handle;
            let BigEntryKey::Name(name) = &hdr_entry.key else { continue };
            let mut filename = name.clone();
            filename.truncate(filename.len().saturating_sub(3));
            filename.push_str("dat");
            banks.push(SpeechBank { sth: DataRef { bytes: Arc::clone(&sth), offset: sth_entry.offset as usize }, filename, handle });
        }
        let bank_set = &mut self.instances[instance].bank_sets[set];
        bank_set.channel = channel;
        bank_set.banks = banks;
        bank_set.project_name = archive;
        self.instances[instance].projects[slot].loaded = true;
    }

    /// `SpeechSystem::Update` 0x82d40fd8: every instance's streams; a stream that finished
    /// reports its event (`EventFinished`, through `EventFinished(VoiceInfo*)` with its speaker's
    /// record, else with the stream's event, trigger and channel) and lets go of its speaker.
    pub(super) fn speech_system_update(&mut self, ctx: &mut SpeechCtx<'_>) {
        for instance in 0..self.instances.len() {
            for stream in 0..STREAMS {
                self.instance_update(instance, stream, ctx);
                let this = &mut self.instances[instance];
                if !this.just_finished[stream] {
                    continue;
                }
                this.just_finished[stream] = false;
                let (event, trigger, channel) = (this.current_event_ids[stream], this.current_trigger_events[stream], this.cur_channels[stream]);
                match this.voice_infos[stream].take() {
                    Some(at) => {
                        let info = *self.voice_info(at);
                        self.event_finished(info.sed_event, info.common_event_id, info.instance, info.channel, info.instance_stream, info.character, info.speaker_id);
                    }
                    None => self.event_finished(event, trigger, instance as i32, channel, stream as i32, -1, u32::MAX),
                }
                let this = &mut self.instances[instance];
                this.cur_channels[stream] = -1;
                this.current_event_ids[stream] = 0;
                this.current_trigger_events[stream] = -1;
            }
        }
    }

    /// `SpeechInstance::Update` 0x82d40be0: a playing stream follows its speaker until done; an
    /// idle one plays each project's next `spch` choice (`SPCH_Play`, the requests going to their
    /// projects) and starts the request `ChooseSampleRequestData` picks.
    fn instance_update(&mut self, instance: usize, stream: usize, ctx: &mut SpeechCtx<'_>) {
        if self.instances[instance].is_playing[stream] {
            if !self.instances[instance].is_buffering[stream] {
                self.update_params(instance, stream, ctx);
                self.update_playing(instance, stream, ctx.core);
            }
            return;
        }
        let mut played = 0;
        for project in 0..2 {
            let (loaded, channel) = {
                let p = &self.instances[instance].projects[project];
                (p.loaded, p.channel)
            };
            if !loaded || self.instances[instance].styles[stream] == VoiceStyle::Disabled {
                continue;
            }
            self.instances[instance].projects[project].sample_requested = false;
            played += self.spch.play(channel as u32);
            for request in self.spch.take_requests() {
                self.sample_request(request);
            }
        }
        if played > 0 {
            if let Some(request) = self.instances[instance].choose_sample_request_data() {
                if self.instances[instance].styles[stream] != VoiceStyle::Disabled {
                    self.play_requested_sample(instance, request, stream, ctx);
                }
            }
        }
    }

    /// `SpeechSystem::SampleRequest` 0x82d3e830 (`SpeechSampleRequestCallback`): the instance
    /// with the request's channel takes it.
    fn sample_request(&mut self, data: SampleRequestData) {
        if let Some(instance) = self.instances.iter_mut().find(|instance| instance.projects.iter().any(|project| project.loaded && project.channel == data.channel)) {
            instance.play_request(data);
        }
    }

    /// `PlayRequestedSample` 0x82d409b8: the bank's `.sth` record at `sampleOffset / 256` gives
    /// the `.dat` offset and the stream header.
    fn play_requested_sample(&mut self, instance: usize, data: SampleRequestData, stream: usize, ctx: &mut SpeechCtx<'_>) {
        if self.instances[instance].is_playing[stream] {
            return;
        }
        let Some((set, bank)) = self.instances[instance].find_bank(data.bank_num, data.channel) else { return };
        let record = bank.sth.at((data.sample_offset / 256) as usize);
        let bytes = record.slice();
        let offset = u32::from_be_bytes([
            bytes.first().copied().unwrap_or(0),
            bytes.get(1).copied().unwrap_or(0),
            bytes.get(2).copied().unwrap_or(0),
            bytes.get(3).copied().unwrap_or(0),
        ]);
        let header = record.at(4);
        let filename = bank.filename.clone();
        let project_name = self.instances[instance].bank_sets[set].project_name.clone();
        self.play_data(instance, header, offset, &filename, &project_name, data, stream, ctx);
    }

    /// `PlayData` 0x82d3f798: with a free speaker (`GetVoiceInfoForSample`) the stream plays the
    /// sample from its `.dat`, `GetEventStartTime` after now (or held back 1000 s while
    /// `mQueueSpeech`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn play_data(&mut self, instance: usize, header: DataRef, offset: u32, filename: &str, project_name: &str, data: SampleRequestData, stream: usize, ctx: &mut SpeechCtx<'_>) {
        let (Some(at), common_event_id) = self.voice_info_for_sample(ctx, instance, data.channel, stream, data.event_spec.event_id, filename, data.callback_data) else { return };
        {
            let info = self.voice_info_mut(at);
            info.instance = instance as i32;
            info.sed_event = data.event_spec.event_id;
            info.channel = data.channel;
            info.instance_stream = stream as i32;
            info.common_event_id = common_event_id;
        }
        let start_time = if self.instances[instance].queue_speech[stream] {
            self.instances[instance].is_buffering[stream] = true;
            ctx.core.system_time + 1000.0
        } else {
            self.instances[instance].is_buffering[stream] = false;
            ctx.core.system_time + f64::from(self.event_start_time(data.channel, stream))
        };
        let this = &mut self.instances[instance];
        this.voice_infos[stream] = Some(at);
        this.current_event_ids[stream] = data.event_spec.event_id;
        this.current_trigger_events[stream] = common_event_id;
        this.cur_channels[stream] = data.channel;
        let Some(player) = this.plugin(stream, SND_PLAYER) else { return };
        let params = PlayParams {
            start_time,
            stream_file_offset: f64::from(offset),
            stream_path: Some(format!("{project_name}{filename}")),
            ram_sample: Some(header),
            stream_pool: STREAM_POOL,
            expel_mode: 0.0,
            ..PlayParams::default()
        };
        this.play_request_handles[stream] = ctx.core.play(player, params);
        this.is_playing[stream] = true;
        this.is_queueing[stream] = true;
        this.is_held[stream] = this.hold_on_finish[stream];
        self.voice_info_mut(at).is_queueing = true;
    }

    /// `SpeechInstance::UpdateParams` 0x82d3f0d0: the speaker's volume, azimuth, pitch (a pitch
    /// of 256 resamples by 0), filters, wet level, notch and delay; without one, unity.
    fn update_params(&mut self, instance: usize, stream: usize, ctx: &mut SpeechCtx<'_>) {
        let this = &self.instances[instance];
        let plugin = |slot: usize| this.plugin(stream, slot);
        let delay_voice = this.delay_voices[stream];
        let actually_playing = this.speech_actually_playing(stream);
        let queueing = this.is_queueing[stream];
        let disable_volume = this.disable_volume[stream];
        let last_delay = this.last_delays[stream];
        let handle = this.play_request_handles[stream];
        let slots = [GAIN, PAN, RESAMPLE, HIGH_PASS, LOW_PASS, SEND_WET, NOTCH, SEND_DELAY, SND_PLAYER].map(plugin);
        let [gain, pan, resample, high_pass, low_pass, wet, notch, send_delay, player] = slots;
        let core = &mut *ctx.core;
        let set = |core: &mut AudioCore, plugin: Option<PluginRef>, index: usize, value: f32| {
            if let Some(plugin) = plugin {
                core.set_attribute(plugin, index, value);
            }
        };
        let mut new_last_delay = last_delay;
        let speaker = this.voice_infos[stream];
        match speaker {
            Some(at) => {
                let info = {
                    let info = self.voice_info_mut(at);
                    info.is_actually_playing = actually_playing;
                    info.is_queueing = queueing;
                    *info
                };
                set(core, gain, 0, if disable_volume { 0.0 } else { info.vol as f32 * VOLUME_SCALE });
                set(core, pan, 0, info.azimuth as f32 * AZIMUTH_SCALE);
                set(core, resample, 0, if info.pitch == 256 { 0.0 } else { info.pitch as f32 * PITCH_SCALE });
                set(core, high_pass, 0, info.high_pass as f32);
                set(core, low_pass, 0, info.low_pass as u32 as f32);
                set(core, wet, 0, info.wet_vol as f32 * VOLUME_SCALE);
                if notch.is_some() {
                    set(core, notch, 0, info.notch_freq);
                    set(core, notch, 1, info.notch_gain);
                    set(core, notch, 2, info.notch_q);
                }
                if let Some(delay) = delay_voice {
                    set(core, send_delay, 0, info.delay_vol as f32 * VOLUME_SCALE);
                    core.set_attribute(PluginRef { voice: delay, index: DELAY_HIGH_PASS }, 0, info.delay_high_pass as f32);
                    core.set_attribute(PluginRef { voice: delay, index: DELAY_LOW_PASS }, 0, info.delay_low_pass as u32 as f32);
                    if info.delay != last_delay {
                        let time = if info.delay <= MAX_DELAY { info.delay } else { MAX_DELAY };
                        core.set_attribute(PluginRef { voice: delay, index: DELAY_DELAY }, 0, time);
                    }
                    new_last_delay = info.delay;
                }
            }
            None => {
                set(core, gain, 0, 1.0);
                set(core, pan, 0, 0.0);
                set(core, resample, 0, 1.0);
                set(core, high_pass, 0, 0.0);
                set(core, low_pass, 0, 25000.0);
                set(core, wet, 0, 0.0);
                if delay_voice.is_some() {
                    set(core, send_delay, 0, 0.0);
                }
            }
        }
        let current = player.map_or(0.0, |player| core.attribute(player, 0));
        let this = &mut self.instances[instance];
        this.last_delays[stream] = new_last_delay;
        if current == handle {
            this.is_queueing[stream] = false;
        }
    }

    /// `UpdatePlaying` 0x82d3f6f0: once the request is done (and the stream not held) the stream
    /// is free and just finished.
    fn update_playing(&mut self, instance: usize, stream: usize, core: &AudioCore) {
        let this = &self.instances[instance];
        if this.is_held[stream] || !this.request_done(stream, core) {
            return;
        }
        let this = &mut self.instances[instance];
        this.is_playing[stream] = false;
        this.is_queueing[stream] = false;
        this.just_finished[stream] = true;
        let speaker = this.voice_infos[stream];
        if let Some(at) = speaker {
            let info = self.voice_info_mut(at);
            info.is_queueing = false;
            info.is_actually_playing = false;
        }
    }

    /// `SpeechSystem::QueueSpeech` 0x824e28e0: the stream's next sample waits to be started.
    pub(super) fn queue_speech(&mut self, instance: usize, stream: usize, queue: bool) {
        if let Some(this) = self.instances.get_mut(instance) {
            this.queue_speech[stream] = queue;
        }
    }

    /// `SpeechSystem::DisableVolume` 0x824e2980.
    pub(super) fn disable_volume(&mut self, instance: usize, stream: usize, value: bool) {
        if let Some(this) = self.instances.get_mut(instance) {
            this.disable_volume[stream] = value;
        }
    }

    /// `mHoldOnFinish[stream]`: a finished request keeps the stream until it is released.
    pub(super) fn set_hold_on_finish(&mut self, instance: usize, stream: usize, value: bool) {
        if let Some(this) = self.instances.get_mut(instance) {
            this.hold_on_finish[stream] = value;
        }
    }

    pub(super) fn hold_on_finish(&self, instance: usize, stream: usize) -> bool {
        self.instances[instance].hold_on_finish[stream]
    }

    /// `mIsHeld[stream] = 0`.
    pub(super) fn release_held(&mut self, instance: usize, stream: usize) {
        if let Some(this) = self.instances.get_mut(instance) {
            this.is_held[stream] = false;
        }
    }

    /// `SpeechSystem::SpeechPlayingOnInstanceStream` 0x824e27d8.
    pub(super) fn speech_playing_on_instance_stream(&self, instance: usize, stream: usize) -> bool {
        self.instances.get(instance).is_some_and(|this| this.speech_playing(stream))
    }

    /// `SpeechInstance::SpeechQueued` 0x82d3fdc0 of the instance's stream.
    pub(super) fn speech_queued(&self, instance: usize, stream: usize, core: &AudioCore) -> bool {
        self.instances.get(instance).is_some_and(|this| this.speech_queued(stream, core))
    }

    /// `SpeechInstance::StartQueuedSpeech` 0x82d3feb8 of the instance's stream.
    pub(super) fn start_queued_speech(&mut self, instance: usize, stream: usize, core: &mut AudioCore) {
        if let Some(this) = self.instances.get_mut(instance) {
            this.start_queued_speech(stream, core);
        }
    }

    /// `SpeechInstance::IsRequestDone` 0x82d3f5f0 of the instance's stream.
    pub(super) fn is_request_done(&self, instance: usize, stream: usize, core: &AudioCore) -> bool {
        self.instances.get(instance).is_none_or(|this| this.request_done(stream, core))
    }

    /// `SpeechSystem::StopAllSpeechOnInstance` 0x82d40000: both streams stop; one that was
    /// actually playing reports its speaker stopped (`EventStopped`, empty in this build) and
    /// forgets its channel, event and trigger.
    pub(crate) fn stop_all_speech_on_instance(&mut self, instance: usize, ctx: &mut SpeechCtx<'_>) {
        if instance >= self.instances.len() {
            return;
        }
        for stream in 0..STREAMS {
            let was_playing = self.instances[instance].speech_actually_playing(stream);
            self.stop_speech_on_instance_stream(instance, stream, ctx);
            if !was_playing {
                continue;
            }
            if let Some(at) = self.instances[instance].voice_infos[stream] {
                self.voice_info_mut(at).is_actually_playing = false;
            }
            let this = &mut self.instances[instance];
            this.cur_channels[stream] = -1;
            this.current_event_ids[stream] = 0;
            this.current_trigger_events[stream] = -1;
        }
    }

    /// `SpeechSystem::StopSpeechOnInstanceStream` 0x824e2860 / `SpeechInstance::StopSpeech`
    /// 0x82d3fa58: the player's stop event; the speaker stops speaking and the stream is no
    /// longer held.
    pub(super) fn stop_speech_on_instance_stream(&mut self, instance: usize, stream: usize, ctx: &mut SpeechCtx<'_>) {
        let Some(this) = self.instances.get_mut(instance) else { return };
        if stream >= STREAMS || !this.is_playing[stream] || this.styles[stream] == VoiceStyle::Disabled {
            return;
        }
        if let Some(player) = this.plugin(stream, SND_PLAYER) {
            ctx.core.stop(player);
        }
        this.is_playing[stream] = false;
        this.is_queueing[stream] = false;
        this.is_held[stream] = false;
        let speaker = this.voice_infos[stream];
        if let Some(at) = speaker {
            let info = self.voice_info_mut(at);
            info.is_actually_playing = false;
            info.is_queueing = false;
        }
    }
}
