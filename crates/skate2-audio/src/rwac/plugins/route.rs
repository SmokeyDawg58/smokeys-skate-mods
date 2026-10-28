//! `rw::audio::core::Route` (`Process` 0x82bf1750): mixes some channels of the voice into chosen
//! SubMix channels at unity gain (the Splicer's LFE-only samples use it).

use crate::rwac::FRAME_SAMPLES;
use crate::rwac::kernels::mix_with_gain;
use crate::math::round_half_away;
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;

pub struct Route {
    base: PlugInBase,
    target: Option<usize>,
    src_channel: usize,
    dst_channel: usize,
    num_channels: usize,
    /// `mDeClickValue[6]`, indexed by SubMix channel.
    declick_value: [f32; 6],
}

impl Route {
    pub fn new(input_channels: u8, output_channels: u8) -> Self {
        Self {
            base: PlugInBase::new(&super::ROUTE, input_channels, output_channels),
            target: None,
            src_channel: 0,
            dst_channel: 0,
            num_channels: 0,
            declick_value: [0.0; 6],
        }
    }

    /// `Route::ConnectByPointerHandler` 0x82bf9e90: disconnects, then connects
    /// `(src, dst, count)` (floats rounded half away from zero).
    pub fn connect(&mut self, slot: Option<usize>, src: f32, dst: f32, count: f32, services: &mut Services<'_>) {
        self.disconnect(services);
        if let Some(slot) = slot {
            self.target = Some(slot);
            self.src_channel = round_half_away(src).max(0) as usize;
            self.dst_channel = round_half_away(dst).max(0) as usize;
            self.num_channels = round_half_away(count).max(0) as usize;
        }
    }

    fn disconnect(&mut self, services: &mut Services<'_>) {
        if let Some(slot) = self.target.take() {
            services.submixes.add_declick(slot, &self.declick_value);
        }
        self.declick_value = [0.0; 6];
    }

    /// Unlinked by a released SubMix (`SubMix::ReleaseEvent` 0x82bfc2d8): unconnected, no declick.
    pub fn unlink(&mut self, slots: &[usize]) {
        if self.target.is_some_and(|slot| slots.contains(&slot)) {
            self.target = None;
        }
    }
}

impl PlugIn for Route {
    impl_plugin_any!();

    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        if let Some(slot) = self.target {
            for i in 0..self.num_channels {
                let src = services.mixer.current(self.src_channel + i).to_vec();
                if let Some(buffer) = services.submixes.get_mut(slot) {
                    if self.dst_channel + i < usize::from(buffer.channels) {
                        mix_with_gain(buffer.channel_mut(self.dst_channel + i), &src, 1.0);
                    }
                }
                if self.dst_channel + i < 6 {
                    self.declick_value[self.dst_channel + i] = src[FRAME_SAMPLES - 1];
                }
            }
        }
        BufferStatus::Available
    }

    fn release(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>) {
        self.disconnect(services);
    }
}
