//! `rw::audio::core::System` with its mixer: voice creation (`Voice::CreateInstance`
//! 0x82be9938), the command buffer, timers, and one 256-sample frame (`Dac::XenonThread`
//! 0x82beb8d8: `System::ExecuteCommands` 0x82bea680, then `Dac::Mix` 0x82beb778).

use super::decoder::XmaContextPool;
use super::mixer::{execute_voice, Mixer};
use super::plugin::{BufferStatus, PlugIn, PluginRef, Services};
use super::plugins::{dac::Dac, delay::Delay, distortion_clip::DistortionClip, frequency_shift_ssb::FrequencyShiftSsb, gain::Gain,
    gain_fader::GainFader, iir2::{HighShelfIir2, PassIir2, PeakingIir2}, packet_player::{Packet, PacketPlayer}, pan2d::Pan2D1, rechannel::Rechannel,
    resample::Resample, reverb_model1::ReverbModel1, route::Route,
    sample_capture::SampleCapture, send::Send, snd_player::{PlayParams, SndPlayer1}, sub_mix::{SubMix, SubMixArena}};
use super::stream_pool::StreamPools;
use super::voice::{Voice, VoiceCore, VoiceId, VoiceState};
use super::{FRAME_SAMPLES, MASTER_CHANNELS, SAMPLE_RATE};
use crate::math::CrtRand;

/// A plug-in to construct: `PlugInConfig1` (handle, constructor parameters, output channels).
#[derive(Debug, Clone)]
pub enum PlugInKind {
    SndPlayer1 { max_requests: f32 },
    PacketPlayer,
    Rechannel,
    Resample,
    Gain,
    Pan2D1 { constructor: Option<(f32, f32, f32)> },
    Send,
    SubMix { name: String },
    Route,
    SampleCapture,
    Dac,
    HighPassIir2,
    LowPassIir2,
    HighShelfIir2,
    PeakingIir2,
    DistortionClip,
    FrequencyShiftSsb { constructor: f32 },
    GainFader,
    Delay { max_time: Option<f32> },
    ReverbModel1 { space_size: Option<f32> },
}

#[derive(Debug, Clone)]
pub struct PlugInConfig {
    pub kind: PlugInKind,
    pub output_channels: u8,
}

/// The command buffer entries.
pub enum Command {
    AddVoice(VoiceId),
    Release(VoiceId),
    SetPriority(VoiceId, f32),
    SetAttribute(PluginRef, usize, f32),
    Play { player: PluginRef, params: Box<PlayParams>, handle: f32 },
    ModifyStartTime { player: PluginRef, handle: f32, start_time: f64 },
    Stop(PluginRef),
    ConnectSend { send: PluginRef, target: Option<PluginRef> },
    ConnectSendByName { send: PluginRef, name: String },
    ConnectRoute { route: PluginRef, target: Option<PluginRef>, src: f32, dst: f32, count: f32 },
    StartFade { fader: PluginRef, start: f64, duration: f32, target: f32, curve: i32 },
    PacketPlay { player: PluginRef, rate: f32, channels: f32 },
    PacketStop(PluginRef),
    PacketSubmit { player: PluginRef, packet: Packet },
}

pub struct AudioCore {
    voices: Vec<Option<Voice>>,
    /// `mpVoiceListNodes`: voices the mixer runs, by processing stage.
    active: Vec<VoiceId>,
    commands: Vec<Command>,
    /// The `TIMERSTAGE_BEFOREMIXER` timers in the order `ExecuteTimers` runs them:
    /// `Collection::AddItem` 0x82be7c28 puts each new timer at the head.
    timers: Vec<PluginRef>,
    /// The `TIMERSTAGE_BEFORECOMMANDS` timers (game clients, by the id their owner gave them), in
    /// the same order.
    before_commands_timers: Vec<u32>,
    pub mixer: Mixer,
    pub submixes: SubMixArena,
    pub xma_contexts: XmaContextPool,
    /// `mSystemTime`.
    pub system_time: f64,
    timer_period: f32,
    dac_out: Vec<f32>,
    dac_start_requested: bool,
    /// The CRT `rand()` shared by the audio libraries.
    pub rand: CrtRand,
    time_stamp: u32,
    /// The `StreamPool`s and the host's files they read.
    pub stream_pools: StreamPools,
}

impl Default for AudioCore {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! services {
    ($self:ident) => {
        Services {
            mixer: &mut $self.mixer,
            submixes: &mut $self.submixes,
            xma_contexts: &mut $self.xma_contexts,
            system_time: $self.system_time,
            dac_out: &mut $self.dac_out,
            dac_start_requested: &mut $self.dac_start_requested,
        }
    };
}

impl AudioCore {
    pub fn new() -> Self {
        Self {
            voices: Vec::new(),
            active: Vec::new(),
            commands: Vec::new(),
            timers: Vec::new(),
            before_commands_timers: Vec::new(),
            mixer: Mixer::new(),
            submixes: SubMixArena::default(),
            xma_contexts: XmaContextPool::default(),
            system_time: 0.0,
            timer_period: FRAME_SAMPLES as f32 / SAMPLE_RATE,
            dac_out: vec![0.0; FRAME_SAMPLES * MASTER_CHANNELS],
            dac_start_requested: true,
            rand: CrtRand::default(),
            time_stamp: 1,
            stream_pools: StreamPools::default(),
        }
    }

    /// `Voice::CreateInstance` 0x82be9938: builds the plug-ins at once and queues the voice for
    /// the active list. `None` when a plug-in cannot be built.
    pub fn create_voice(&mut self, processing_stage: u8, name: &'static str, configs: &[PlugInConfig]) -> Option<VoiceId> {
        let slot = self.voices.iter().position(Option::is_none).unwrap_or(self.voices.len());
        let id = VoiceId(slot as u32);
        let mut core = VoiceCore {
            id,
            name,
            processing_stage,
            priority: 100.0,
            state: VoiceState::Playing,
            decay_samples: 0.0,
            decay_eaten_samples: 0.0,
            expel_after_decay_samples: 0.0,
            pitch: 1.0,
            input_start_point: 255,
            discontinuity_plugin: 0,
            creation_time_stamp: self.time_stamp,
        };
        let mut plugins: Vec<Box<dyn PlugIn>> = Vec::with_capacity(configs.len());
        let mut input_channels = 0u8;
        for (index, config) in configs.iter().enumerate() {
            let out = config.output_channels;
            let plugin: Box<dyn PlugIn> = match &config.kind {
                PlugInKind::SndPlayer1 { max_requests } => Box::new(SndPlayer1::new(input_channels, out, *max_requests)),
                PlugInKind::PacketPlayer => Box::new(PacketPlayer::new(input_channels, out)),
                PlugInKind::Rechannel => Box::new(Rechannel::new(input_channels, out)),
                PlugInKind::Resample => Box::new(Resample::new(&mut core, input_channels, out)),
                PlugInKind::Gain => Box::new(Gain::new(input_channels, out)),
                PlugInKind::Pan2D1 { constructor } => Box::new(Pan2D1::new(input_channels, out, *constructor)),
                PlugInKind::Send => Box::new(Send::new(input_channels, out)),
                PlugInKind::SubMix { name } => Box::new(SubMix::new(&mut self.submixes, name, input_channels, out)),
                PlugInKind::Route => Box::new(Route::new(input_channels, out)),
                PlugInKind::SampleCapture => Box::new(SampleCapture::new(input_channels, out)),
                PlugInKind::Dac => Box::new(Dac::new(input_channels, out)),
                PlugInKind::HighPassIir2 => Box::new(PassIir2::new(&mut core, true, input_channels, out)),
                PlugInKind::LowPassIir2 => Box::new(PassIir2::new(&mut core, false, input_channels, out)),
                PlugInKind::HighShelfIir2 => Box::new(HighShelfIir2::new(&mut core, input_channels, out)),
                PlugInKind::PeakingIir2 => Box::new(PeakingIir2::new(&mut core, input_channels, out)),
                PlugInKind::DistortionClip => Box::new(DistortionClip::new(input_channels, out)),
                PlugInKind::FrequencyShiftSsb { constructor } => {
                    Box::new(FrequencyShiftSsb::new(&mut core, *constructor, input_channels, out)?)
                }
                PlugInKind::GainFader => Box::new(GainFader::new(input_channels, out)),
                PlugInKind::Delay { max_time } => Box::new(Delay::new(input_channels, out, *max_time)),
                PlugInKind::ReverbModel1 { space_size } => Box::new(ReverbModel1::new(input_channels, out, *space_size)),
            };
            if plugin.base().desc.plugin_type <= 3 {
                core.input_start_point = index as u8;
            }
            // `SndPlayer1`, `PacketPlayer`, `Delay` and `ReverbModel1` `CreateInstance` add their
            // `RwacTimerClient` timers at once.
            if plugin.as_any().is::<SndPlayer1>()
                || plugin.as_any().is::<PacketPlayer>()
                || plugin.as_any().is::<Delay>()
                || plugin.as_any().is::<ReverbModel1>()
            {
                self.timers.insert(0, PluginRef { voice: id, index: index as u8 });
            }
            plugins.push(plugin);
            input_channels = out;
        }
        let voice = Voice { core, plugins };
        if slot == self.voices.len() {
            self.voices.push(Some(voice));
        } else {
            self.voices[slot] = Some(voice);
        }
        self.commands.push(Command::AddVoice(id));
        Some(id)
    }

    pub fn voice(&self, id: VoiceId) -> Option<&Voice> {
        self.voices.get(id.0 as usize).and_then(Option::as_ref)
    }

    fn voice_mut(&mut self, id: VoiceId) -> Option<&mut Voice> {
        self.voices.get_mut(id.0 as usize).and_then(Option::as_mut)
    }

    /// `Voice::mState`, read by owners to see an expelled voice.
    pub fn voice_state(&self, id: VoiceId) -> Option<VoiceState> {
        self.voice(id).map(|v| v.core.state)
    }

    pub fn plugin<T: 'static>(&self, plugin: PluginRef) -> Option<&T> {
        self.voice(plugin.voice)?.plugins.get(usize::from(plugin.index))?.as_any().downcast_ref()
    }

    pub fn plugin_mut<T: 'static>(&mut self, plugin: PluginRef) -> Option<&mut T> {
        self.voice_mut(plugin.voice)?.plugins.get_mut(usize::from(plugin.index))?.as_any_mut().downcast_mut()
    }

    /// `Voice::Release` 0x82be9990.
    pub fn release_voice(&mut self, id: VoiceId) {
        self.commands.push(Command::Release(id));
    }

    /// `Voice::SetPriority` 0x82be99c0.
    pub fn set_priority(&mut self, id: VoiceId, priority: f32) {
        self.commands.push(Command::SetPriority(id, priority));
    }

    /// `PlugIn::SetAttribute` 0x82be8a78.
    pub fn set_attribute(&mut self, plugin: PluginRef, index: usize, value: f32) {
        self.commands.push(Command::SetAttribute(plugin, index, value));
    }

    /// `PlugIn::GetAttribute` 0x82be24b8.
    pub fn attribute(&self, plugin: PluginRef, index: usize) -> f32 {
        self.voice(plugin.voice)
            .and_then(|v| v.plugins.get(usize::from(plugin.index)))
            .map_or(0.0, |p| p.base().attr(index))
    }

    /// `PlugIn::GetAttribute(int, double*)`.
    pub fn attribute_f64(&self, plugin: PluginRef, index: usize) -> f64 {
        self.voice(plugin.voice)
            .and_then(|v| v.plugins.get(usize::from(plugin.index)))
            .and_then(|p| p.base().attributes.get(index).copied())
            .unwrap_or(0.0)
    }

    /// `SndPlayer1` event 5 (play); returns the request handle.
    pub fn play(&mut self, player: PluginRef, params: PlayParams) -> f32 {
        let Some(snd) = self.plugin_mut::<SndPlayer1>(player) else {
            return 0.0;
        };
        let handle = snd.begin_play();
        self.commands.push(Command::Play { player, params: Box::new(params), handle });
        handle
    }

    /// `SndPlayer1` event 1 (stop).
    pub fn stop(&mut self, player: PluginRef) {
        self.commands.push(Command::Stop(player));
    }

    /// `SndPlayer1` event 3, answered at once: the request's data is buffered.
    #[must_use]
    pub fn request_buffered(&self, player: PluginRef, handle: f32) -> bool {
        self.plugin::<SndPlayer1>(player).is_some_and(|snd| snd.request_buffered(handle))
    }

    /// `SndPlayer1` event 4: the request's new start time.
    pub fn modify_start_time(&mut self, player: PluginRef, handle: f32, start_time: f64) {
        self.commands.push(Command::ModifyStartTime { player, handle, start_time });
    }

    /// Send event 0 (connect by pointer).
    pub fn connect_send(&mut self, send: PluginRef, target: Option<PluginRef>) {
        self.commands.push(Command::ConnectSend { send, target });
    }

    /// Send event 1 (connect by name).
    pub fn connect_send_by_name(&mut self, send: PluginRef, name: &str) {
        self.commands.push(Command::ConnectSendByName { send, name: name.to_owned() });
    }

    /// Route event 0.
    pub fn connect_route(&mut self, route: PluginRef, target: Option<PluginRef>, src: f32, dst: f32, count: f32) {
        self.commands.push(Command::ConnectRoute { route, target, src, dst, count });
    }

    /// `GainFader` event 0 (`EventEvent` 0x82bfd6b8 truncates the curve with `fctiwz`).
    pub fn start_fade(&mut self, fader: PluginRef, start: f64, duration: f32, target: f32, curve: f32) {
        let curve = crate::nicotine::shape::fctiwz(curve);
        self.commands.push(Command::StartFade { fader, start, duration, target, curve });
    }

    /// `PacketPlayer` event 0 (play): the source's rate and channel count.
    pub fn packet_play(&mut self, player: PluginRef, rate: f32, channels: f32) {
        self.commands.push(Command::PacketPlay { player, rate, channels });
    }

    /// `PacketPlayer` event 1 (stop).
    pub fn packet_stop(&mut self, player: PluginRef) {
        self.commands.push(Command::PacketStop(player));
    }

    /// `PacketPlayer` event 2 (submit a packet).
    pub fn packet_submit(&mut self, player: PluginRef, packet: Packet) {
        self.commands.push(Command::PacketSubmit { player, packet });
    }

    /// `PacketPlayer` event 3, answered at once: the packet is still submitted or with the decoder.
    #[must_use]
    pub fn packet_in_use(&self, player: PluginRef, id: u32) -> bool {
        self.plugin::<PacketPlayer>(player).is_some_and(|p| p.packet_in_use(id))
    }

    /// `TimerManager::AddTimer` 0x82be9438 at `TIMERSTAGE_BEFORECOMMANDS`.
    pub fn add_before_commands_timer(&mut self, id: u32) {
        self.before_commands_timers.insert(0, id);
    }

    /// `System::RemoveTimer` 0x82be93c8.
    pub fn remove_before_commands_timer(&mut self, id: u32) {
        self.before_commands_timers.retain(|&timer| timer != id);
    }

    fn submix_slot(&self, plugin: Option<PluginRef>) -> Option<usize> {
        plugin.and_then(|p| self.plugin::<SubMix>(p)).map(|s| s.slot)
    }

    /// `SubMix::ReleaseEvent` 0x82bfc2d8: every Send and Route still connected to a released
    /// SubMix is unlinked from it.
    fn unlink_sub_mixes(&mut self, slots: &[usize]) {
        for voice in self.voices.iter_mut().flatten() {
            for plugin in &mut voice.plugins {
                if let Some(send) = plugin.as_any_mut().downcast_mut::<Send>() {
                    send.unlink(slots);
                } else if let Some(route) = plugin.as_any_mut().downcast_mut::<Route>() {
                    route.unlink(slots);
                }
            }
        }
    }

    /// `Voice::RemoveActiveVoice` 0x82be3a78.
    fn remove_active(&mut self, id: VoiceId) -> bool {
        if let Some(position) = self.active.iter().position(|&v| v == id) {
            self.active.remove(position);
            true
        } else {
            false
        }
    }

    /// `Voice::ExpelImmediate` 0x82be7a80.
    fn expel_immediate(&mut self, id: VoiceId) {
        if let Some(voice) = self.voice_mut(id) {
            if voice.core.state != VoiceState::Expelled {
                voice.core.state = VoiceState::Expelled;
                self.remove_active(id);
            }
        }
    }

    /// `System::UpdateExpellingVoices` 0x82be92c8 (removing a voice shifts the list, so the
    /// next one waits a frame, as in the native loop).
    fn update_expelling_voices(&mut self) {
        let mut index = 0;
        while index < self.active.len() {
            let id = self.active[index];
            let expel = match self.voice_mut(id) {
                Some(voice) if voice.core.state == VoiceState::ExpelAfterDecay => {
                    if voice.core.expel_after_decay_samples < voice.core.decay_samples {
                        voice.core.expel_after_decay_samples = voice.core.decay_samples;
                    }
                    voice.core.decay_eaten_samples >= voice.core.expel_after_decay_samples
                }
                _ => false,
            };
            if expel {
                self.expel_immediate(id);
            }
            index += 1;
        }
    }

    fn execute_command(&mut self, command: Command) {
        match command {
            Command::AddVoice(id) => {
                let Some(stage) = self.voice(id).map(|v| v.core.processing_stage) else {
                    return;
                };
                let position = self
                    .active
                    .iter()
                    .position(|&other| self.voice(other).is_some_and(|v| stage <= v.core.processing_stage))
                    .unwrap_or(self.active.len());
                self.active.insert(position, id);
            }
            Command::Release(id) => {
                if let Some(mut voice) = self.voices.get_mut(id.0 as usize).and_then(Option::take) {
                    let mut services = services!(self);
                    for plugin in &mut voice.plugins {
                        plugin.release(&mut voice.core, &mut services);
                    }
                    self.timers.retain(|timer| timer.voice != id);
                    self.remove_active(id);
                    let released: Vec<usize> = voice.plugins.iter().filter_map(|plugin| plugin.as_any().downcast_ref::<SubMix>().map(|sub_mix| sub_mix.slot)).collect();
                    if !released.is_empty() {
                        self.unlink_sub_mixes(&released);
                    }
                }
            }
            Command::SetPriority(id, priority) => {
                if let Some(voice) = self.voice_mut(id) {
                    voice.core.priority = priority;
                }
            }
            Command::SetAttribute(plugin, index, value) => {
                if let Some(p) = self.voice_mut(plugin.voice).and_then(|v| v.plugins.get_mut(usize::from(plugin.index))) {
                    if let Some(slot) = p.base_mut().attributes.get_mut(index) {
                        *slot = f64::from(value);
                    }
                }
            }
            Command::Play { player, params, handle } => {
                if let Some(voice) = self.voices.get_mut(player.voice.0 as usize).and_then(Option::as_mut) {
                    if let Some(snd) = voice.plugins.get_mut(usize::from(player.index)).and_then(|p| p.as_any_mut().downcast_mut::<SndPlayer1>()) {
                        snd.play_handler(*params, handle, &self.stream_pools);
                    }
                }
            }
            Command::ModifyStartTime { player, handle, start_time } => {
                let system_time = self.system_time;
                if let Some(snd) = self.plugin_mut::<SndPlayer1>(player) {
                    snd.modify_start_time_handler(handle, start_time, system_time);
                }
            }
            Command::Stop(player) => {
                if let Some(voice) = self.voices.get_mut(player.voice.0 as usize).and_then(Option::as_mut) {
                    if let Some(snd) = voice.plugins.get_mut(usize::from(player.index)).and_then(|p| p.as_any_mut().downcast_mut::<SndPlayer1>()) {
                        snd.stop_handler(&mut voice.core, &mut self.xma_contexts);
                    }
                }
            }
            Command::ConnectSend { send, target } => {
                let slot = self.submix_slot(target);
                self.with_plugin(send, |plugin: &mut Send, services| plugin.connect(slot, services));
            }
            Command::ConnectSendByName { send, name } => {
                let slot = self.submixes.find(&name);
                self.with_plugin(send, |plugin: &mut Send, services| plugin.connect(slot, services));
            }
            Command::ConnectRoute { route, target, src, dst, count } => {
                let slot = self.submix_slot(target);
                self.with_plugin(route, |plugin: &mut Route, services| plugin.connect(slot, src, dst, count, services));
            }
            Command::StartFade { fader, start, duration, target, curve } => {
                if let Some(plugin) = self.plugin_mut::<GainFader>(fader) {
                    plugin.start_fade(start, duration, target, curve);
                }
            }
            Command::PacketPlay { player, rate, channels } => {
                if let Some(plugin) = self.plugin_mut::<PacketPlayer>(player) {
                    plugin.play_handler(rate, channels);
                }
            }
            Command::PacketStop(player) => {
                if let Some(plugin) = self.plugin_mut::<PacketPlayer>(player) {
                    plugin.stop_handler();
                }
            }
            Command::PacketSubmit { player, packet } => {
                if let Some(plugin) = self.plugin_mut::<PacketPlayer>(player) {
                    plugin.submit_handler(packet);
                }
            }
        }
    }

    fn with_plugin<T: 'static>(&mut self, plugin: PluginRef, f: impl FnOnce(&mut T, &mut Services<'_>)) {
        let Some(mut voice) = self.voices.get_mut(plugin.voice.0 as usize).and_then(Option::take) else {
            return;
        };
        if let Some(p) = voice.plugins.get_mut(usize::from(plugin.index)).and_then(|p| p.as_any_mut().downcast_mut::<T>()) {
            let mut services = services!(self);
            f(p, &mut services);
        }
        self.voices[plugin.voice.0 as usize] = Some(voice);
    }

    /// `System::ExecuteCommands` 0x82bea680. `ExecuteTimers` 0x82be37a8 calls each
    /// `TIMERSTAGE_BEFORECOMMANDS` timer with the timer period; one removed by an earlier callback
    /// is skipped and one added runs from the next frame.
    fn execute_commands(&mut self, timer: &mut impl FnMut(&mut AudioCore, u32, f32)) {
        for id in self.before_commands_timers.clone() {
            if self.before_commands_timers.contains(&id) {
                let period = self.timer_period;
                timer(self, id, period);
            }
        }
        self.update_expelling_voices();
        let commands = std::mem::take(&mut self.commands);
        for command in commands {
            self.execute_command(command);
        }
        self.time_stamp = self.time_stamp.wrapping_add(1);
        for timer in self.timers.clone() {
            let Some(mut voice) = self.voices.get_mut(timer.voice.0 as usize).and_then(Option::take) else {
                continue;
            };
            if let Some(plugin) = voice.plugins.get_mut(usize::from(timer.index)) {
                let mut services = services!(self);
                plugin.timer(&mut voice.core, &mut services);
            }
            self.voices[timer.voice.0 as usize] = Some(voice);
        }
    }

    /// `Dac::Mix` 0x82beb778: `Mixer::Execute` 0x82be6c90 over the active voices, returning the
    /// status of the last one; then the system time advances by the timer period.
    fn mix(&mut self) -> BufferStatus {
        self.mixer.params_system_time = self.system_time;
        self.mixer.params_sample_rate = SAMPLE_RATE;
        let mut status = BufferStatus::Unavailable;
        for id in self.active.clone() {
            if let Some(mut voice) = self.voices.get_mut(id.0 as usize).and_then(Option::take) {
                let mut services = services!(self);
                status = execute_voice(&mut voice, &mut services);
                self.voices[id.0 as usize] = Some(voice);
            }
        }
        self.system_time += f64::from(self.timer_period);
        status
    }

    /// One frame of `Dac::XenonThread` 0x82beb8d8: commands, then the mix. Returns 256
    /// interleaved frames of FL, FR, C, LFE, SL, SR, silent when the mix was unavailable.
    pub fn render_frame(&mut self) -> &[f32] {
        self.render_frame_with(|_, _, _| {})
    }

    /// [`render_frame`](Self::render_frame) with `timer` serving the `TIMERSTAGE_BEFORECOMMANDS`
    /// timers: it gets the core, the timer's id and the timer period.
    pub fn render_frame_with(&mut self, mut timer: impl FnMut(&mut AudioCore, u32, f32)) -> &[f32] {
        self.execute_commands(&mut timer);
        if self.mix() != BufferStatus::Available {
            self.dac_out.fill(0.0);
        }
        &self.dac_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bus(core: &mut AudioCore, name: &str) -> VoiceId {
        core.create_voice(5, "bus", &[PlugInConfig { kind: PlugInKind::SubMix { name: name.into() }, output_channels: 1 }]).expect("bus")
    }

    fn sends(core: &AudioCore, send: PluginRef) -> bool {
        core.plugin::<Send>(send).is_some_and(Send::is_connected)
    }

    #[test]
    fn a_released_sub_mix_unlinks_the_sends_into_it() {
        let mut core = AudioCore::new();
        let speaker = bus(&mut core, "Speaker SubMix");
        let kept = bus(&mut core, "Kept");
        let source = core
            .create_voice(0, "source", &[PlugInConfig { kind: PlugInKind::SubMix { name: "Source".into() }, output_channels: 1 }, PlugInConfig { kind: PlugInKind::Send, output_channels: 1 }, PlugInConfig { kind: PlugInKind::Send, output_channels: 1 }])
            .expect("source");
        let into_speaker = PluginRef { voice: source, index: 1 };
        let into_kept = PluginRef { voice: source, index: 2 };
        core.connect_send(into_speaker, Some(PluginRef { voice: speaker, index: 0 }));
        core.connect_send(into_kept, Some(PluginRef { voice: kept, index: 0 }));
        core.render_frame();
        assert!(sends(&core, into_speaker) && sends(&core, into_kept));
        core.release_voice(speaker);
        core.render_frame();
        bus(&mut core, "Reuses the slot");
        core.render_frame();
        assert!(!sends(&core, into_speaker));
        assert!(sends(&core, into_kept));
    }
}
