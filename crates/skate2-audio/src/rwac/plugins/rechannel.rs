//! `rw::audio::core::Rechannel` (`Process` 0x82bf05d0, pre-process `MapChannels::PreProcess`
//! 0x82bed798): an input plug-in that maps the source channels to its output channels through
//! `ReChannelGainWrite` at unity gain.

use crate::rwac::kernels::rechannel_gain_write;
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;
use crate::rwac::FRAME_SAMPLES;

pub struct Rechannel {
    base: PlugInBase,
}

impl Rechannel {
    pub fn new(input_channels: u8, output_channels: u8) -> Self {
        Self { base: PlugInBase::new(&super::RECHANNEL, input_channels, output_channels) }
    }
}

impl PlugIn for Rechannel {
    impl_plugin_any!();

    /// `MapChannels::PreProcess`: the samples asked for.
    fn pre_process(&mut self, _mixer: &mut crate::rwac::mixer::Mixer, _discontinuity: bool, requested: i32) -> i32 {
        requested
    }

    /// `Process`: the source's channel count becomes the input count while no samples came; another
    /// count than the output's is rewritten into the scratch buffer (at most six source channels),
    /// which becomes current.
    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let mixer = &mut *services.mixer;
        let src_channels = mixer.src_num_channels;
        let samples = usize::try_from(mixer.src_num_samples).unwrap_or(0).min(FRAME_SAMPLES);
        if samples == 0 {
            self.base.input_channels = src_channels;
        }
        if src_channels != self.base.output_channels {
            if samples != 0 {
                let (current, scratch) = mixer.current_and_scratch();
                let src: Vec<&[f32]> = current.chunks_exact(FRAME_SAMPLES).take(usize::from(src_channels).min(6)).map(|c| &c[..samples]).collect();
                let mut dst: Vec<&mut [f32]> =
                    scratch.chunks_exact_mut(FRAME_SAMPLES).take(usize::from(self.base.output_channels)).map(|c| &mut c[..samples]).collect();
                rechannel_gain_write(&mut dst, &src, 1.0);
            }
            mixer.swap();
            mixer.src_num_channels = self.base.output_channels;
        }
        BufferStatus::Available
    }
}
