//! `rw::audio::core::PacketPlayer` ('PPl0', `Process` 0x82bf7710): plays the sample packets its
//! owner submits through a decoder of the `DecoderRegistry`. `rw::movie::AudioRenderer_Rwac` is
//! its only user: 16-bit packets for `Pcm16LittleDec` ('P6L0', `DecodeEvent` 0x82c14780), whose
//! samples become floats times 1/32767 (`0x38000100`).
//!
//! Events (`EventEvent` 0x82c012a0): 0 play (sample format, rate, channels), 1 stop and 2 submit go
//! through the command buffer; 3 answers at once whether a packet is still in use. Attribute 0 is
//! the time played, which `RwacTimerClient` (0x82c00918) sets before each mix: the samples decoded
//! over the rate.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::rwac::mixer::Mixer;
use crate::rwac::plugin::{BufferStatus, PlugIn, PlugInBase, Services, impl_plugin_any};
use crate::rwac::voice::{VoiceCore, VoiceState};

/// `rw::audio::core::Packet`: `mNumSamples` samples per channel, interleaved.
#[derive(Clone, Debug)]
pub struct Packet {
    /// The owner's name for the packet, which event 3 asks about.
    pub id: u32,
    pub num_samples: i32,
    pub data: Arc<[i16]>,
}

/// The feed descriptors (`mFeedDesc`) and the decoder's request slots, filled in step.
const FEED_SLOTS: usize = 20;
/// `StopHandler` (0x82bf75a0) arms a declick of this many samples.
const DECLICK_SAMPLES: u8 = 16;
/// `Pcm16LittleDec::DecodeEvent`'s scale.
const PCM16_SCALE: u32 = 0x3800_0100;

/// One feed descriptor: the packet, how much of it the decoder has produced, and its state
/// (0 free, 1 with the decoder, 2 decoded and waiting for `FeedCleanup`).
#[derive(Clone, Debug, Default)]
struct Feed {
    packet: Option<Packet>,
    decoded: i32,
    state: u8,
}

pub struct PacketPlayer {
    base: PlugInBase,
    /// `mSubmittedPackets`: submitted and not yet fed.
    submitted: VecDeque<Packet>,
    feed: [Feed; FEED_SLOTS],
    /// The next descriptor to feed and the one the decoder reads.
    fill_slot: usize,
    decode_slot: usize,
    /// 0 idle, 1 play requested, 2 playing, 4 stopped.
    state: u8,
    /// The play request's rate and channels; the format reported to the mixer follows them.
    request_rate: f32,
    request_channels: u8,
    current_rate: f32,
    /// A decoder exists (`StartPlaying` made one).
    decoder: bool,
    /// The samples decoded since the play request.
    samples_played: u32,
    /// `PreProcess`'s request.
    samples_requested: u16,
    /// The last value of each channel and whether one was gathered, for the declick.
    last_values: Vec<f32>,
    dc_gathered: bool,
    declick_samples: u8,
}

impl PacketPlayer {
    /// `CreateInstance` 0x82c01198: idle at 48 kHz; its timer is added at once.
    pub fn new(input_channels: u8, output_channels: u8) -> Self {
        Self {
            base: PlugInBase::new(&super::PACKET_PLAYER, input_channels, output_channels),
            submitted: VecDeque::new(),
            feed: Default::default(),
            fill_slot: 0,
            decode_slot: 0,
            state: 0,
            request_rate: 0.0,
            request_channels: 0,
            current_rate: 48000.0,
            decoder: false,
            samples_played: 0,
            samples_requested: 0,
            last_values: vec![0.0; usize::from(output_channels)],
            dc_gathered: false,
            declick_samples: 0,
        }
    }

    fn playing(&self) -> bool {
        self.state != 0 && self.state != 4
    }

    /// `PlayHandler` 0x82c00878: the request is kept; `RwacTimerClient` starts it.
    pub fn play_handler(&mut self, rate: f32, channels: f32) {
        self.decoder = false;
        self.request_rate = rate;
        self.state = 1;
        self.request_channels = channels as i64 as u8;
    }

    /// `SubmitHandler` 0x82bf7648.
    pub fn submit_handler(&mut self, packet: Packet) {
        self.submitted.push_back(packet);
    }

    /// `StopHandler` 0x82bf75a0: the decoder released, the request cleared, every packet dropped
    /// and a declick armed.
    pub fn stop_handler(&mut self) {
        self.decoder = false;
        self.request_rate = 0.0;
        self.request_channels = 0;
        self.state = 4;
        self.submitted.clear();
        self.feed = Default::default();
        self.fill_slot = 0;
        self.decode_slot = 0;
        self.declick_samples = DECLICK_SAMPLES;
    }

    /// Event 3: a packet still submitted, or fed and not yet decoded, is in use.
    #[must_use]
    pub fn packet_in_use(&self, id: u32) -> bool {
        self.submitted.iter().any(|p| p.id == id)
            || self
                .feed
                .iter()
                .any(|f| f.packet.as_ref().is_some_and(|p| p.id == id) && f.state != 2)
    }

    /// `FeedCleanup` 0x82bf7690: decoded descriptors are freed.
    fn feed_cleanup(&mut self) {
        for feed in &mut self.feed {
            if feed.state == 2 {
                feed.state = 0;
                feed.packet = None;
            }
        }
    }

    /// `StartPlaying` 0x82bfe9b8: the decoder, then the first packets with a reset.
    fn start_playing(&mut self) -> bool {
        self.decoder = true;
        self.submit_next_packet();
        true
    }

    /// `SubmitNextPacket` 0x82bfe860: submitted packets into free descriptors, in order.
    fn submit_next_packet(&mut self) -> bool {
        let mut submitted = false;
        while !self.submitted.is_empty() {
            if self.feed[self.fill_slot].state != 0 {
                break;
            }
            let slot = self.fill_slot;
            self.fill_slot = (self.fill_slot + 1) % FEED_SLOTS;
            self.feed[slot] = Feed {
                packet: self.submitted.pop_front(),
                decoded: 0,
                state: 1,
            };
            submitted = true;
        }
        submitted
    }

    /// `Decoder::GetSamplesRemaining` of a descriptor's request.
    fn remaining(&self, slot: usize) -> i32 {
        self.feed[slot]
            .packet
            .as_ref()
            .map_or(0, |p| p.num_samples - self.feed[slot].decoded)
    }

    /// `Declick` 0x82bedb18: each channel's last value ramps to zero.
    fn declick(&mut self, mixer: &mut Mixer) -> BufferStatus {
        let count = self
            .declick_samples
            .min(u8::try_from(self.samples_requested).unwrap_or(u8::MAX));
        let scale = 1.0 / f32::from(self.declick_samples);
        for channel in 0..usize::from(self.base.output_channels) {
            let mut value = self.last_values.get(channel).copied().unwrap_or(0.0);
            let step = value * scale;
            for sample in mixer
                .scratch_mut(channel)
                .iter_mut()
                .take(usize::from(count))
            {
                value -= step;
                *sample = value;
            }
            if let Some(slot) = self.last_values.get_mut(channel) {
                *slot = value;
            }
        }
        self.declick_samples -= count;
        mixer.swap();
        mixer.src_num_channels = self.base.output_channels;
        mixer.src_sample_rate = self.current_rate;
        mixer.src_num_samples = i32::from(count);
        if self.declick_samples == 0 {
            self.dc_gathered = false;
        }
        BufferStatus::Available
    }
}

impl PlugIn for PacketPlayer {
    impl_plugin_any!();

    /// `PreProcess` 0x82bedca0.
    fn pre_process(&mut self, _mixer: &mut Mixer, _discontinuity: bool, requested: i32) -> i32 {
        self.samples_requested = requested as u16;
        0
    }

    fn process(
        &mut self,
        _voice: &mut VoiceCore,
        services: &mut Services<'_>,
        _discontinuity: bool,
    ) -> BufferStatus {
        let mixer = &mut *services.mixer;
        if self.declick_samples != 0 && self.dc_gathered {
            return self.declick(mixer);
        }
        let mut produced = 0i32;
        if self.playing() {
            if self.request_rate != self.current_rate
                || self.request_channels != self.base.output_channels
            {
                mixer.src_num_samples = 0;
                mixer.src_num_channels = self.request_channels;
                mixer.src_sample_rate = self.request_rate;
                self.base.output_channels = self.request_channels;
                self.last_values
                    .resize(usize::from(self.request_channels), 0.0);
                self.current_rate = self.request_rate;
                return BufferStatus::Available;
            }
            if self.feed[self.decode_slot].state == 0 {
                while self.decode_slot != self.fill_slot {
                    self.decode_slot = (self.decode_slot + 1) % FEED_SLOTS;
                    if self.feed[self.decode_slot].state != 0 {
                        break;
                    }
                }
            }
            if self.feed[self.decode_slot].state == 1 {
                let slot = self.decode_slot;
                let remaining = self.remaining(slot);
                let count = i32::from(self.samples_requested).min(remaining).max(0);
                let channels = usize::from(self.base.output_channels);
                if let Some(packet) = self.feed[slot].packet.as_ref() {
                    let scale = f32::from_bits(PCM16_SCALE);
                    let start = usize::try_from(self.feed[slot].decoded).unwrap_or(0);
                    for channel in 0..channels {
                        let out = mixer.scratch_mut(channel);
                        for (i, sample) in out.iter_mut().take(count as usize).enumerate() {
                            let value = packet
                                .data
                                .get((start + i) * channels + channel)
                                .copied()
                                .unwrap_or(0);
                            *sample = f32::from(value) * scale;
                        }
                    }
                }
                produced = count;
                mixer.swap();
                mixer.src_num_samples = produced;
                self.samples_played = self.samples_played.wrapping_add(produced as u32);
                self.feed[slot].decoded += produced;
                let mut left = remaining - produced;
                while left == 0 && self.feed[self.decode_slot].state == 1 {
                    self.feed[self.decode_slot].state = 2;
                    self.decode_slot = (self.decode_slot + 1) % FEED_SLOTS;
                    if self.decoder && self.feed[self.decode_slot].state == 1 {
                        left = self.remaining(self.decode_slot);
                    }
                }
            }
        }
        mixer.src_num_channels = self.base.output_channels;
        mixer.src_sample_rate = self.current_rate;
        if produced != 0 {
            let channels = usize::from(mixer.src_num_channels.min(self.request_channels));
            for channel in 0..channels {
                self.last_values[channel] = mixer.current(channel)[produced as usize - 1];
            }
            self.dc_gathered = true;
            BufferStatus::Available
        } else if self.samples_requested != 0 {
            BufferStatus::Unavailable
        } else {
            mixer.src_num_samples = 0;
            BufferStatus::Available
        }
    }

    /// `RwacTimerClient` 0x82c00918.
    fn timer(&mut self, voice: &mut VoiceCore, _services: &mut Services<'_>) {
        if voice.state == VoiceState::Expelled {
            return;
        }
        self.feed_cleanup();
        if !self.playing() {
            self.base.attributes[0] = 0.0;
            return;
        }
        self.base.attributes[0] =
            f64::from(self.samples_played as i32) / f64::from(self.request_rate);
        loop {
            if !self.playing() || self.feed[self.fill_slot].state != 0 {
                break;
            }
            if self.state == 1 {
                if !self.start_playing() {
                    return;
                }
                self.state = 2;
            }
            if self.state == 2 && self.feed[self.fill_slot].state == 0 && !self.submit_next_packet()
            {
                break;
            }
        }
    }
}
