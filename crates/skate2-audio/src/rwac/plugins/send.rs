//! `rw::audio::core::Send`: mixes the voice's signal into a SubMix with a gain that ramps when it
//! changes (`Process` 0x82bf1910); the signal itself passes on unchanged.

use crate::rwac::FRAME_SAMPLES;
use crate::rwac::kernels::{rechannel_gain_mix, rechannel_gain_mix_ramp, rechannel_gain_write};
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;

pub struct Send {
    base: PlugInBase,
    /// `mSubMixConnector`: the SubMix slot and its channel count.
    target: Option<(usize, u8)>,
    /// The gain of the previous frame (initially 1.0).
    previous_gain: f32,
    /// Set while unconnected so the next connected frame starts without a ramp.
    reset_ramp: bool,
    /// `mDeClickValue[6]`: last sample of each input channel times the gain.
    declick_value: [f32; 6],
}

impl Send {
    /// `Send::CreateInstance` 0x82bfaa10.
    pub fn new(input_channels: u8, output_channels: u8) -> Self {
        Self {
            base: PlugInBase::new(&super::SEND, input_channels, output_channels),
            target: None,
            previous_gain: 1.0,
            reset_ramp: false,
            declick_value: [0.0; 6],
        }
    }

    /// `Send::DisconnectImmediate` 0x82bfaaa0: hands the last values, folded to the SubMix's
    /// channels, to the SubMix's declick.
    pub fn disconnect(&mut self, services: &mut Services<'_>) {
        if let Some((slot, channels)) = self.target.take() {
            let inputs = usize::from(self.base.input_channels).min(6);
            let src_values: Vec<[f32; 1]> = self.declick_value[..inputs].iter().map(|v| [*v]).collect();
            let src: Vec<&[f32]> = src_values.iter().map(|v| &v[..]).collect();
            let mut dst_values = [[0.0f32; 1]; 6];
            {
                let mut dst: Vec<&mut [f32]> =
                    dst_values.iter_mut().take(usize::from(channels).min(6)).map(|v| &mut v[..]).collect();
                rechannel_gain_write(&mut dst, &src, 1.0);
            }
            let mut folded = [0.0f32; 6];
            for (out, value) in folded.iter_mut().zip(dst_values) {
                *out = value[0];
            }
            services.submixes.add_declick(slot, &folded);
            self.declick_value = [0.0; 6];
        }
    }

    /// `Send::ConnectByPointerHandler` 0x82bfabe0.
    pub fn connect(&mut self, slot: Option<usize>, services: &mut Services<'_>) {
        self.disconnect(services);
        if let Some(slot) = slot {
            let channels = services.submixes.channels(slot);
            self.target = Some((slot, channels));
        }
    }

    /// `mSubMixConnector.mpSubMix` set.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.target.is_some()
    }

    /// Unlinked by a released SubMix (`SubMix::ReleaseEvent` 0x82bfc2d8): unconnected, no declick.
    pub fn unlink(&mut self, slots: &[usize]) {
        if self.target.is_some_and(|(slot, _)| slots.contains(&slot)) {
            self.target = None;
        }
    }
}

impl PlugIn for Send {
    impl_plugin_any!();

    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, discontinuity: bool) -> BufferStatus {
        let gain = self.base.attr(0);
        if discontinuity || self.reset_ramp {
            self.previous_gain = gain;
            self.reset_ramp = false;
        }
        match self.target {
            Some((slot, channels)) if channels != 0 => {
                let inputs = usize::from(self.base.input_channels);
                let mut source = vec![0.0f32; inputs * FRAME_SAMPLES];
                for c in 0..inputs {
                    source[c * FRAME_SAMPLES..(c + 1) * FRAME_SAMPLES].copy_from_slice(services.mixer.current(c));
                }
                if let Some(buffer) = services.submixes.get_mut(slot) {
                    let src: Vec<&[f32]> = source.chunks_exact(FRAME_SAMPLES).collect();
                    let mut dst: Vec<&mut [f32]> =
                        buffer.data.chunks_exact_mut(FRAME_SAMPLES).take(usize::from(channels)).collect();
                    if gain == self.previous_gain {
                        rechannel_gain_mix(&mut dst, &src, gain);
                    } else {
                        rechannel_gain_mix_ramp(&mut dst, &src, gain, self.previous_gain);
                    }
                }
                self.previous_gain = gain;
                for c in 0..inputs.min(6) {
                    self.declick_value[c] = source[c * FRAME_SAMPLES + FRAME_SAMPLES - 1] * gain;
                }
            }
            _ => self.reset_ramp = true,
        }
        BufferStatus::Available
    }

    fn release(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>) {
        self.disconnect(services);
    }
}
