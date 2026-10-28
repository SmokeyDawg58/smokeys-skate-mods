//! `rw::audio::core::DistortionClip` (`Process` 0x82bebd88): hard clipping at the level
//! attribute, bypassed from 100 up.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;

pub struct DistortionClip {
    base: PlugInBase,
}

impl DistortionClip {
    /// `CreateInstance` 0x82bf5d00.
    pub fn new(input_channels: u8, output_channels: u8) -> Self {
        Self { base: PlugInBase::new(&super::DISTORTION_CLIP, input_channels, output_channels) }
    }
}

impl PlugIn for DistortionClip {
    impl_plugin_any!();

    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let level = self.base.attr(0);
        if !(level < 100.0) {
            return BufferStatus::Available;
        }
        let floor = level * -1.0;
        for channel in 0..usize::from(self.base.output_channels) {
            let (current, scratch) = services.mixer.current_and_scratch_channel(channel);
            for (out, &sample) in scratch.iter_mut().zip(current) {
                *out = if sample > level {
                    level
                } else if sample < floor {
                    floor
                } else {
                    sample
                };
            }
        }
        services.mixer.swap();
        BufferStatus::Available
    }
}
