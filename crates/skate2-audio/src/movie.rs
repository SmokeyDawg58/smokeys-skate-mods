//! `rw::movie::AudioRenderer_Rwac`: a movie's sound as the voice "RWMovie Voice" (`Init`
//! 0x82b8c678): `PacketPlayer`, `Rechannel`, `Resample`, `GainFader` and `Send`, all with the
//! movie's channel count, at processing stage 0, the Send connected to the mastering sub mix. Its
//! `RwAudioCoreTimer` (0x82b8ceb0, the `TIMERSTAGE_BEFORECOMMANDS` timer `Begin` adds) hands the
//! player 256-sample packets of the movie's decoded sound and takes back the ones it finished.
//!
//! The renderer's finished callback stays null for the game's movies
//! (`MovieRenderer::PrepareRenderer` 0x82b80b80 passes the movie renderer's, which the game's
//! `MoviePlayer` never sets), so the timer's end check never stops the feed.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::rwac::core::{AudioCore, PlugInConfig, PlugInKind};
use crate::rwac::plugins::packet_player::Packet;
use crate::rwac::{PluginRef, VoiceId};

/// The movie timer's id among the core's `TIMERSTAGE_BEFORECOMMANDS` timers.
pub const TIMER_ID: u32 = 0x5257_4D56;

/// `PacketManager::Init` (0x82b8c4e8) as `Init` calls it: 16 packets of 256 samples.
const PACKETS: usize = 16;
const SAMPLES_PER_PACKET: usize = 256;

/// The movie's decoded sound, interleaved 16-bit samples, which the decode callback
/// (`AudioDecoder_EAXA::Decode` 0x82b955b8) hands the renderer up to a packet at a time.
#[derive(Clone, Debug, Default)]
pub struct MovieSound {
    samples: Arc<Mutex<VecDeque<i16>>>,
}

impl MovieSound {
    /// Decoded samples from the movie's decoder.
    pub fn push(&self, samples: &[i16]) {
        let mut queue = self
            .samples
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        queue.extend(samples.iter().copied());
    }

    /// Up to `frames` whole frames of `channels` samples.
    fn take(&self, frames: usize, channels: usize) -> Vec<i16> {
        let mut queue = self
            .samples
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let count = (queue.len() / channels.max(1)).min(frames) * channels;
        queue.drain(..count).collect()
    }
}

pub struct MovieAudio {
    voice: VoiceId,
    packet_player: PluginRef,
    gain_fader: PluginRef,
    sample_rate: u32,
    channels: u8,
    /// `mVolume`, 0 from the constructor (0x82b8cc00).
    volume: f32,
    timer_added: bool,
    sound: MovieSound,
    /// The packet manager's ring: the id each packet slot holds (0 for one never submitted),
    /// `mHead`, `mTail` and `mNumPacketsInUse`.
    packets: [u32; PACKETS],
    head: usize,
    tail: usize,
    in_use: usize,
    next_id: u32,
}

impl MovieAudio {
    /// The constructor and `Init`: the voice, its `GainFader` set to the volume at once and its
    /// `Send` connected to the mastering sub mix. `None` when the voice cannot be made.
    pub fn new(
        core: &mut AudioCore,
        master_sub_mix: PluginRef,
        sample_rate: u32,
        channels: u8,
        sound: MovieSound,
    ) -> Option<Self> {
        let config = |kind| PlugInConfig {
            kind,
            output_channels: channels,
        };
        let voice = core.create_voice(
            0,
            "RWMovie Voice",
            &[
                config(PlugInKind::PacketPlayer),
                config(PlugInKind::Rechannel),
                config(PlugInKind::Resample),
                config(PlugInKind::GainFader),
                config(PlugInKind::Send),
            ],
        )?;
        let audio = Self {
            voice,
            packet_player: PluginRef { voice, index: 0 },
            gain_fader: PluginRef { voice, index: 3 },
            sample_rate,
            channels,
            volume: 0.0,
            timer_added: false,
            sound,
            packets: [0; PACKETS],
            head: 0,
            tail: 0,
            in_use: 0,
            next_id: 1,
        };
        core.start_fade(audio.gain_fader, 0.0, 0.0, audio.volume, 0.0);
        core.connect_send(PluginRef { voice, index: 4 }, Some(master_sub_mix));
        Some(audio)
    }

    /// `Begin` (0x82b8d320): the player's play event with the rate and channel count, then the
    /// timer.
    pub fn begin(&mut self, core: &mut AudioCore) {
        core.packet_play(
            self.packet_player,
            self.sample_rate as f32,
            f32::from(self.channels),
        );
        core.add_before_commands_timer(TIMER_ID);
        self.timer_added = true;
    }

    /// `SetVolume` (0x82b8c988): a 0.02 s fade to the volume.
    pub fn set_volume(&mut self, core: &mut AudioCore, volume: f32) {
        self.volume = volume;
        core.start_fade(self.gain_fader, 0.0, 0.02, volume, 0.0);
    }

    /// `GetRenderingPosition` (0x82b8c868): the player's time played in milliseconds.
    #[must_use]
    pub fn rendering_position(&self, core: &AudioCore) -> f32 {
        core.attribute_f64(self.packet_player, 0) as f32 * 1000.0
    }

    /// `Stop` (0x82b8cba0), then the destructor's `Voice::Release` (0x82b8cd18).
    pub fn shutdown(self, core: &mut AudioCore) {
        if self.timer_added {
            core.remove_before_commands_timer(TIMER_ID);
        }
        core.packet_stop(self.packet_player);
        core.release_voice(self.voice);
    }

    /// `PacketManager`'s allocation: the head slot while one is free.
    fn allocate(&mut self) -> Option<usize> {
        if self.in_use == PACKETS {
            return None;
        }
        let slot = self.head;
        self.head = (self.head + 1) % PACKETS;
        self.in_use += 1;
        Some(slot)
    }

    /// `RwAudioCoreTimer`: the packets at the tail the player no longer uses are freed, then up to
    /// two new ones are filled and submitted. Each packet filled takes a second ring slot it does
    /// not use, as the native timer allocates again after a successful decode.
    pub fn timer(&mut self, core: &mut AudioCore) {
        while self.in_use > 0 {
            if core.packet_in_use(self.packet_player, self.packets[self.tail]) {
                break;
            }
            self.packets[self.tail] = 0;
            self.tail = (self.tail + 1) % PACKETS;
            self.in_use -= 1;
        }
        let channels = usize::from(self.channels);
        let mut submitted = 0;
        let mut packet = self.allocate();
        while let Some(slot) = packet {
            let data = self.sound.take(SAMPLES_PER_PACKET, channels);
            if data.is_empty() {
                break;
            }
            self.allocate();
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
            self.packets[slot] = id;
            let num_samples = (data.len() / channels) as i32;
            core.packet_submit(
                self.packet_player,
                Packet {
                    id,
                    num_samples,
                    data: data.into(),
                },
            );
            submitted += 1;
            if submitted > 1 {
                break;
            }
            packet = self.allocate();
        }
    }
}
