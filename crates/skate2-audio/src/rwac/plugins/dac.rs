//! `rw::audio::core::Dac` (`Process` 0x82bebc70 -> `XenonDownMix` 0x82bebb98): the end of the
//! mastering voice. Writes the frame in WAVE channel order, ramps the first 128 samples after a
//! start and clips to [-1, 1].

use crate::rwac::kernels::{clip_floats, reorder_to_wave};
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;
use crate::rwac::{FRAME_SAMPLES, MASTER_CHANNELS};

pub struct Dac {
    base: PlugInBase,
}

impl Dac {
    pub fn new(input_channels: u8, output_channels: u8) -> Self {
        Self {
            base: PlugInBase::new(&super::DAC, input_channels, output_channels),
        }
    }
}

/// `Dac::RampOutput` 0x82beb5e8 (ramp up): frame `i` scaled by `i * (1 / n)`.
fn ramp_up(data: &mut [f32], frames: usize) {
    let reciprocal = 1.0 / frames as f32;
    for (i, frame) in data.chunks_exact_mut(MASTER_CHANNELS).take(frames).enumerate() {
        let scale = (i as f32) * reciprocal;
        for sample in frame {
            *sample *= scale;
        }
    }
}

impl PlugIn for Dac {
    impl_plugin_any!();

    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let channels: [Vec<f32>; MASTER_CHANNELS] = std::array::from_fn(|c| services.mixer.current(c).to_vec());
        let refs: [&[f32]; MASTER_CHANNELS] = std::array::from_fn(|c| channels[c].as_slice());
        reorder_to_wave(&mut services.dac_out[..FRAME_SAMPLES * MASTER_CHANNELS], &refs);
        if *services.dac_start_requested {
            ramp_up(services.dac_out, 128);
            *services.dac_start_requested = false;
        }
        clip_floats(services.dac_out, -1.0, 1.0);
        BufferStatus::Available
    }
}
