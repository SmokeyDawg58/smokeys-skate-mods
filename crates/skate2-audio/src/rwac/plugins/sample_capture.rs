//! `rw::audio::core::SampleCapture` (`Process` 0x82bfa338): records the signal only while a
//! capture is running (a debug feature the game never starts); otherwise passes it on.

use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;

pub struct SampleCapture {
    base: PlugInBase,
}

impl SampleCapture {
    pub fn new(input_channels: u8, output_channels: u8) -> Self {
        Self {
            base: PlugInBase::new(&super::SAMPLE_CAPTURE, input_channels, output_channels),
        }
    }
}

impl PlugIn for SampleCapture {
    impl_plugin_any!();

    fn process(&mut self, _voice: &mut VoiceCore, _services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        BufferStatus::Available
    }
}
