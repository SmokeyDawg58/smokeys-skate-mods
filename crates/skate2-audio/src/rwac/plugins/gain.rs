//! `rw::audio::core::Gain` (`Process` 0x82bec150): gain ramped over 64 samples on change.

use crate::rwac::kernels::copy_with_gain_ramp;
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;

pub struct Gain {
    base: PlugInBase,
    previous_gain: f32,
}

impl Gain {
    /// `Gain::CreateInstance` 0x82bf60d0: the previous gain starts at the attribute.
    pub fn new(input_channels: u8, output_channels: u8) -> Self {
        let base = PlugInBase::new(&super::GAIN, input_channels, output_channels);
        let previous_gain = base.attr(0);
        Self { base, previous_gain }
    }
}

impl PlugIn for Gain {
    impl_plugin_any!();

    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, discontinuity: bool) -> BufferStatus {
        let gain = self.base.attr(0);
        if discontinuity {
            self.previous_gain = gain;
        }
        let inc = (gain - self.previous_gain) * 0.015_625;
        for channel in 0..usize::from(self.base.output_channels) {
            let (current, scratch) = services.mixer.current_and_scratch_channel(channel);
            copy_with_gain_ramp(scratch, current, self.previous_gain, inc);
        }
        services.mixer.swap();
        self.previous_gain = gain;
        BufferStatus::Available
    }
}
