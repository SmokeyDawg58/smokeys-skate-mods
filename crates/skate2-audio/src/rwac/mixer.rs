//! `rw::audio::core::Mixer`: three 64-channel x 256-sample buffers and the pull model that runs
//! each voice's plug-ins.
//!
//! Buffer roles (`mpSampleBuffer[3]`): 0 is the current signal, 1 the scratch a plug-in writes
//! before swapping 0 and 1, 2 accumulates chunks of input plug-ins that produce fewer than 256
//! samples per pass.

use super::plugin::{BufferStatus, Services};
use super::voice::{Voice, VoiceCore};
use super::{BUFFER_CHANNELS, FRAME_SAMPLES};

const BUFFER_LEN: usize = BUFFER_CHANNELS * FRAME_SAMPLES;

pub struct Mixer {
    buffers: [Box<[f32]>; 3],
    roles: [usize; 3],
    /// `mCurrentMixTime`: system time of the sample the input plug-ins are at.
    pub mix_time: f64,
    /// `mSrcNumSamples`: samples the last plug-in produced.
    pub src_num_samples: i32,
    /// `mSrcSampleRate`: rate of those samples.
    pub src_sample_rate: f32,
    /// `mTotalPitch`: product of the resample ratios of the input plug-ins.
    pub total_pitch: f32,
    /// `mSrcNumChannels`.
    pub src_num_channels: u8,
    /// `MixerExecuteParams::systemTime` of the frame.
    pub params_system_time: f64,
    /// `MixerExecuteParams` sample rate (48000).
    pub params_sample_rate: f32,
}

impl Default for Mixer {
    fn default() -> Self {
        Self::new()
    }
}

impl Mixer {
    pub fn new() -> Self {
        let buffer = || vec![0.0f32; BUFFER_LEN].into_boxed_slice();
        Self {
            buffers: [buffer(), buffer(), buffer()],
            roles: [0, 1, 2],
            mix_time: 0.0,
            src_num_samples: 0,
            src_sample_rate: 0.0,
            total_pitch: 1.0,
            src_num_channels: 0,
            params_system_time: 0.0,
            params_sample_rate: super::SAMPLE_RATE,
        }
    }

    fn range(channel: usize) -> std::ops::Range<usize> {
        channel * FRAME_SAMPLES..(channel + 1) * FRAME_SAMPLES
    }

    /// Channel `channel` of the current buffer (`mpSampleBuffer[0]`).
    pub fn current(&self, channel: usize) -> &[f32] {
        &self.buffers[self.roles[0]][Self::range(channel)]
    }

    pub fn current_mut(&mut self, channel: usize) -> &mut [f32] {
        &mut self.buffers[self.roles[0]][Self::range(channel)]
    }

    /// Channel `channel` of the scratch buffer (`mpSampleBuffer[1]`).
    pub fn scratch_mut(&mut self, channel: usize) -> &mut [f32] {
        &mut self.buffers[self.roles[1]][Self::range(channel)]
    }

    /// The current and scratch buffers at once.
    pub fn current_and_scratch(&mut self) -> (&[f32], &mut [f32]) {
        let [a, b, c] = &mut self.buffers;
        match (self.roles[0], self.roles[1]) {
            (0, 1) => (&**a, &mut **b),
            (0, 2) => (&**a, &mut **c),
            (1, 0) => (&**b, &mut **a),
            (1, 2) => (&**b, &mut **c),
            (2, 0) => (&**c, &mut **a),
            _ => (&**c, &mut **b),
        }
    }

    /// Channel `channel` of the current buffer and of the scratch buffer.
    pub fn current_and_scratch_channel(&mut self, channel: usize) -> (&[f32], &mut [f32]) {
        let range = Self::range(channel);
        let (current, scratch) = self.current_and_scratch();
        (&current[range.clone()], &mut scratch[range])
    }

    /// Swaps current and scratch (`mpSampleBuffer[0]` <-> `[1]`).
    pub fn swap(&mut self) {
        self.roles.swap(0, 1);
    }

    fn accumulation_mut(&mut self, channel: usize) -> &mut [f32] {
        &mut self.buffers[self.roles[2]][Self::range(channel)]
    }

    fn copy_current_to_accumulation(&mut self, channels: usize, offset: usize, count: usize) {
        for channel in 0..channels {
            let source = Self::range(channel).start;
            let (current, accumulation) = (self.roles[0], self.roles[2]);
            let mut tmp = [0.0f32; FRAME_SAMPLES];
            tmp[..count].copy_from_slice(&self.buffers[current][source..source + count]);
            self.buffers[accumulation][source + offset..source + offset + count]
                .copy_from_slice(&tmp[..count]);
        }
    }
}

/// `Mixer::HandleBufferStatusUnavailable` 0x82be0f18: while the voice still has decay samples to
/// eat, output `num_samples` of silence on `output_channels` and carry on.
fn handle_unavailable(voice: &mut VoiceCore, mixer: &mut Mixer, output_channels: u8, num_samples: i32) -> BufferStatus {
    if voice.expel_after_decay_samples < voice.decay_samples {
        voice.expel_after_decay_samples = voice.decay_samples;
    }
    if voice.decay_eaten_samples < voice.expel_after_decay_samples {
        voice.decay_eaten_samples += num_samples as f32;
        let count = (num_samples.max(0) as usize).min(FRAME_SAMPLES);
        for channel in 0..usize::from(output_channels) {
            mixer.current_mut(channel)[..count].fill(0.0);
        }
        mixer.src_num_samples = num_samples;
        BufferStatus::Available
    } else {
        voice.discontinuity_plugin = 0;
        BufferStatus::Unavailable
    }
}

/// `Mixer::ProcessInputPlugIns` 0x82be0ff0: runs the input plug-ins in passes until 256 samples
/// exist at the output rate.
fn process_input_plugins(voice: &mut Voice, services: &mut Services<'_>) -> BufferStatus {
    let start = usize::from(voice.core.input_start_point);
    let mut direct = false;
    let mut saved_rate = 0.0f32;
    let mut produced = 0usize;
    let mut channels = 0u8;
    let mut status;
    loop {
        let mut needed = (FRAME_SAMPLES - produced) as i32;
        services.mixer.total_pitch = 1.0;
        let mut requested_from_source = needed;
        for index in (0..=start).rev() {
            requested_from_source = needed;
            let discontinuity = index > usize::from(voice.core.discontinuity_plugin);
            needed = voice.plugins[index].pre_process(services.mixer, discontinuity, needed);
            if needed >= FRAME_SAMPLES as i32 {
                needed = FRAME_SAMPLES as i32;
            }
        }
        status = BufferStatus::Available;
        for index in 0..=start {
            let discontinuity = index > usize::from(voice.core.discontinuity_plugin);
            status = voice.plugins[index].process(&mut voice.core, services, discontinuity);
            if status == BufferStatus::Available {
                if index == 0 {
                    voice.core.decay_eaten_samples = 0.0;
                }
            } else {
                let output_channels = voice.plugins[index].base().output_channels;
                status = handle_unavailable(&mut voice.core, services.mixer, output_channels, requested_from_source);
                if status == BufferStatus::Unavailable {
                    break;
                }
            }
        }
        if status != BufferStatus::Available {
            if produced != 0 {
                for channel in 0..usize::from(channels) {
                    services.mixer.accumulation_mut(channel)[produced..].fill(0.0);
                }
                services.mixer.src_num_channels = channels;
                status = BufferStatus::Available;
                services.mixer.src_sample_rate = saved_rate;
            }
            break;
        }
        let count = services.mixer.src_num_samples.max(0) as usize;
        let source_channels = services.mixer.src_num_channels;
        if produced == 0 && count == FRAME_SAMPLES {
            direct = true;
        }
        if direct || count != 0 {
            if !direct {
                let count = count.min(FRAME_SAMPLES - produced);
                services.mixer.copy_current_to_accumulation(usize::from(source_channels), produced, count);
            }
            channels = source_channels;
            saved_rate = services.mixer.src_sample_rate;
            services.mixer.mix_time += f64::from(count as f32 / services.mixer.params_sample_rate);
        }
        produced += count;
        if produced >= FRAME_SAMPLES {
            break;
        }
    }
    if !direct {
        let channels = usize::from(services.mixer.src_num_channels);
        for channel in 0..channels {
            let mut tmp = [0.0f32; FRAME_SAMPLES];
            tmp.copy_from_slice(services.mixer.accumulation_mut(channel));
            services.mixer.scratch_mut(channel).copy_from_slice(&tmp);
        }
        services.mixer.swap();
    }
    services.mixer.src_num_samples = FRAME_SAMPLES as i32;
    status
}

/// One voice of `Mixer::Execute` 0x82be6c90.
pub fn execute_voice(voice: &mut Voice, services: &mut Services<'_>) -> BufferStatus {
    services.mixer.total_pitch = 1.0;
    let (mut status, start) = if voice.core.input_start_point == 255 {
        (BufferStatus::Available, 0usize)
    } else {
        services.mixer.mix_time = services.mixer.params_system_time;
        let status = process_input_plugins(voice, services);
        (status, usize::from(voice.core.input_start_point) + 1)
    };
    services.mixer.mix_time = services.mixer.params_system_time;
    if status == BufferStatus::Available {
        let count = voice.plugins.len();
        for index in start..count {
            let discontinuity = index > usize::from(voice.core.discontinuity_plugin);
            status = voice.plugins[index].process(&mut voice.core, services, discontinuity);
            if status == BufferStatus::Available {
                if index == 0 {
                    voice.core.decay_eaten_samples = 0.0;
                }
            } else {
                let output_channels = voice.plugins[index].base().output_channels;
                status = handle_unavailable(&mut voice.core, services.mixer, output_channels, FRAME_SAMPLES as i32);
                if status == BufferStatus::Unavailable {
                    break;
                }
            }
        }
        if status == BufferStatus::Available {
            voice.core.discontinuity_plugin = count as u8;
        }
    }
    status
}
