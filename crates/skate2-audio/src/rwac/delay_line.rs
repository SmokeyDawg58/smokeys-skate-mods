//! `rw::audio::core::DelayLine` (`Init` 0x82be4108, `Resize` 0x82be41e0, `Reset` 0x82bdd978,
//! `SetDelay` 0x82bdd790, `ApplyFilter` 0x82be7f68): the per-channel history `Delay` and
//! `ReverbModel1` filter through.
//!
//! `ApplyFilter` marshals each channel's taps into a local buffer whose tap region runs straight
//! into the samples the filter writes back, so a tap shorter than the frame reads this frame's
//! own output (`LoadTaps` 0x82be45b8, `UnmarshalDelayData` 0x82be4828), and zeroes what lies
//! behind the samples written since `Reset` (`IncrementalClean` 0x82bdd7d8). [`Location::tap`]
//! reads the history in place with the same values. The pad (+0x18) is always 0.

use super::mixer::Mixer;
use super::FRAME_SAMPLES;

/// The samples a changed delay cross-fades over (`ApplyFilter`'s 128-entry ramp).
pub const CROSS_FADE: usize = 128;

#[derive(Debug, Default)]
pub struct DelayLine {
    /// +0x00: `channels * len` samples, empty until `Init`.
    buffer: Vec<f32>,
    /// +0x0C.
    max_delay: i32,
    /// +0x10: samples the filter reads per output sample.
    max_read: i32,
    /// +0x14.
    len: i32,
    /// +0x20: delays the history can serve.
    valid: i32,
    /// +0x24: samples written since `Reset`.
    cleaned: i32,
    /// +0x28.
    delay: i32,
    /// +0x2C: the delay `SetDelay` replaced.
    old_delay: i32,
    /// +0x30.
    channels: i32,
    /// +0x34.
    position: i32,
    /// +0x38.
    cross_fade: bool,
}

/// One channel's history and this frame's write buffer while a filter runs.
pub struct Location<'a> {
    history: &'a [f32],
    position: usize,
    cleaned: i32,
    pub write: &'a mut [f32; FRAME_SAMPLES],
}

impl Location<'_> {
    /// The sample `delay` behind write index `index`: the write buffer from the write position
    /// on, 0 behind the samples written since `Reset`.
    #[must_use]
    pub fn tap(&self, index: usize, delay: i32) -> f32 {
        let behind = delay - index as i32;
        if behind <= 0 {
            self.write[(-behind) as usize]
        } else if behind > self.cleaned {
            0.0
        } else {
            let len = self.history.len();
            self.history[(self.position + len - behind as usize) % len]
        }
    }

    /// A store through the tap pointer: only the write-buffer part outlives the filter call.
    pub fn set_tap(&mut self, index: usize, delay: i32, value: f32) {
        let behind = delay - index as i32;
        if behind <= 0 {
            self.write[(-behind) as usize] = value;
        }
    }
}

/// One filter call of `ApplyFilter`: samples `start..start + count` at `delay`, the first
/// [`CROSS_FADE`] after `SetDelay` also reading the replaced delay.
#[derive(Debug, Clone, Copy)]
pub struct Chunk {
    pub start: usize,
    pub count: usize,
    pub delay: i32,
    pub fade: Option<i32>,
}

impl DelayLine {
    /// `Init`: at least `max_read + 255` samples of delay, the length rounded up to 32 plus the
    /// read margin.
    pub fn init(&mut self, channels: i32, max_delay: i32, max_read: i32) {
        let max_delay = max_delay.max(max_read + 0xFF);
        let len = ((max_delay + 0x20) & !0x1F) + ((max_read + 0x1E) & !0x1F);
        self.buffer = vec![0.0; (len * channels) as usize];
        self.max_delay = max_delay;
        self.max_read = max_read;
        self.position = 0;
        self.channels = channels;
        self.len = len;
        self.valid = len;
    }

    /// `Resize`: a longer line keeps the samples written since `Reset`, ending where the next
    /// write goes (`len` rounds up without the read margin).
    pub fn resize(&mut self, max_delay: i32) {
        if self.buffer.is_empty() {
            self.init(self.channels, max_delay, self.max_read);
            return;
        }
        let len = (max_delay + 0x20) & !0x1F;
        if self.len < len {
            let (old_len, new_len, kept) = (self.len as usize, len as usize, self.cleaned as usize);
            let mut buffer = vec![0.0; new_len * self.channels as usize];
            for channel in 0..self.channels as usize {
                let old = &self.buffer[channel * old_len..(channel + 1) * old_len];
                let new = &mut buffer[channel * new_len..(channel + 1) * new_len];
                for i in 0..kept {
                    new[new_len - kept + i] = old[(self.position as usize + old_len - kept + i) % old_len];
                }
            }
            self.buffer = buffer;
            self.len = len;
            self.position = 0;
        }
        self.max_delay = max_delay;
    }

    /// `ResizeNeeded` 0x82be48d8.
    #[must_use]
    pub fn resize_needed(&self, max_delay: i32) -> bool {
        self.len < (max_delay + 0x20) & !0x1F
    }

    /// `Reset`: the history counts as silent again.
    pub fn reset(&mut self, delay: i32) {
        self.cleaned = 0;
        self.delay = delay;
        self.cross_fade = false;
        self.valid = self.len;
        self.position = 0;
    }

    /// `SetDelay`: a change cross-fades from the old delay in the next frame.
    pub fn set_delay(&mut self, delay: i32) {
        if delay != self.delay {
            self.old_delay = self.delay;
            self.delay = delay;
            self.cross_fade = true;
        }
    }

    /// `GetValidDelaySamples` 0x82bdd7c0.
    #[must_use]
    pub fn valid_delay_samples(&self) -> i32 {
        self.valid.min(self.max_delay)
    }

    #[must_use]
    pub fn delay(&self) -> i32 {
        self.delay
    }

    #[must_use]
    pub fn max_delay(&self) -> i32 {
        self.max_delay
    }

    /// `ApplyFilter` over one 256-sample frame from the mixer's current buffer into its scratch
    /// buffer (channel by channel), then the write buffer goes into the history.
    pub fn apply_filter(&mut self, mixer: &mut Mixer, mut filter: impl FnMut(&mut Location<'_>, &[f32], &mut [f32], Chunk)) {
        let len = self.len as usize;
        for channel in 0..self.channels as usize {
            let mut write = [0.0f32; FRAME_SAMPLES];
            {
                let history = &self.buffer[channel * len..(channel + 1) * len];
                let mut location = Location { history, position: self.position as usize, cleaned: self.cleaned, write: &mut write };
                let (input, output) = mixer.current_and_scratch_channel(channel);
                let mut start = 0;
                if self.cross_fade {
                    start = CROSS_FADE.min(FRAME_SAMPLES);
                    filter(&mut location, input, output, Chunk { start: 0, count: start, delay: self.delay, fade: Some(self.old_delay) });
                }
                if start < FRAME_SAMPLES {
                    filter(&mut location, input, output, Chunk { start, count: FRAME_SAMPLES - start, delay: self.delay, fade: None });
                }
            }
            let history = &mut self.buffer[channel * len..(channel + 1) * len];
            for (i, value) in write.into_iter().enumerate() {
                history[(self.position as usize + i) % len] = value;
            }
        }
        let frame = FRAME_SAMPLES as i32;
        self.position = (self.position + frame) % self.len;
        self.cleaned = (self.cleaned + frame).min(self.len);
        self.valid = (self.valid + frame).min(self.len);
        self.cross_fade = false;
    }
}
