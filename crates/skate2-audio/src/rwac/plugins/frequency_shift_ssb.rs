//! `rw::audio::core::FrequencyShiftSsb` (`CreateInstance` 0x82bf5da0, `Process` 0x82bf5e98):
//! single-sideband frequency shift of channel 0 by the shift attribute (Hz), from two all-pass
//! paths 90 degrees apart.
//!
//! The constructor parameter 1.0 adds a `Fir64` band-pass ahead of the shift; that path is not
//! ported (the skateboard voices construct with 0.0), so [`FrequencyShiftSsb::new`] refuses it.

use super::iir2::{Coefficients, Iir2};
use crate::nicotine::shape::fctiwz;
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;
use crate::rwac::FRAME_SAMPLES;

const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
/// `__real_3e22f983` (1 / 2 pi).
const INV_TWO_PI: f32 = f32::from_bits(0x3E22_F983);

const fn coefficients(bits: [u32; 5]) -> Coefficients {
    Coefficients {
        a1: f32::from_bits(bits[0]),
        a2: f32::from_bits(bits[1]),
        b0: f32::from_bits(bits[2]),
        b1: f32::from_bits(bits[3]),
        b2: f32::from_bits(bits[4]),
    }
}

/// `sPath1PhaseShiftCoefficients0/1` 0x8300ce28 and `sPath2PhaseShiftCoefficients0/1` 0x8300ce50.
const PATH1: [Coefficients; 2] = [
    coefficients([0xBFF6_20C5, 0x3F6C_7469, 0x3F6C_7469, 0xBFF6_20C5, 0x3F80_0000]),
    coefficients([0xBECD_B811, 0xBE5A_43B1, 0xBE5A_43B1, 0xBECD_B811, 0x3F80_0000]),
];
const PATH2: [Coefficients; 2] = [
    coefficients([0xBFFB_DFCD, 0x3F77_C5D9, 0x3F77_C5D9, 0xBFFB_DFCD, 0x3F80_0000]),
    coefficients([0xBFA1_A207, 0x3EB1_E001, 0x3EB1_E001, 0xBFA1_A207, 0x3F80_0000]),
];

/// `fnmsubs`: -(a * c - b) with one rounding.
fn fnmsubs(a: f32, c: f32, b: f32) -> f32 {
    -a.mul_add(c, -b)
}

pub struct FrequencyShiftSsb {
    base: PlugInBase,
    /// +0x30, +0x40: path 1's sections; +0x50, +0x60: path 2's.
    path1: [Iir2; 2],
    path2: [Iir2; 2],
    /// +0xF4: the oscillator phase.
    phase: f32,
    /// +0xFC: the shift of the band-pass coefficients (written every frame).
    last_shift: f32,
}

impl FrequencyShiftSsb {
    /// `CreateInstance` with a constructor other than 1.0: 690 decay samples, no latency. `None`
    /// for 1.0 (the band-pass path).
    pub fn new(core: &mut VoiceCore, constructor: f32, input_channels: u8, output_channels: u8) -> Option<Self> {
        if constructor == 1.0 {
            return None;
        }
        let mut base = PlugInBase::new(&super::FREQUENCY_SHIFT_SSB, input_channels, output_channels);
        base.latency_samples = 0.0;
        core.decay_samples += 690.0 - base.decay_samples;
        base.decay_samples = 690.0;
        Some(Self { base, path1: [Iir2::default(); 2], path2: [Iir2::default(); 2], phase: 0.0, last_shift: -100_000.0 })
    }
}

impl PlugIn for FrequencyShiftSsb {
    impl_plugin_any!();

    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let mut input = [0.0f32; FRAME_SAMPLES];
        input.copy_from_slice(services.mixer.current(0));
        let (mut stage, mut real, mut imaginary) = ([0.0f32; FRAME_SAMPLES], [0.0f32; FRAME_SAMPLES], [0.0f32; FRAME_SAMPLES]);
        self.path1[0].filter(&mut stage, &input, &PATH1[0]);
        self.path1[1].filter(&mut real, &stage, &PATH1[1]);
        self.path2[0].filter(&mut stage, &input, &PATH2[0]);
        self.path2[1].filter(&mut imaginary, &stage, &PATH2[1]);
        let shift = self.base.attr(0);
        let step = (shift / services.mixer.params_sample_rate) * TWO_PI;
        let out = services.mixer.scratch_mut(0);
        for ((out, &re), &im) in out.iter_mut().zip(&real).zip(&imaginary) {
            let sin = f64::from(self.phase).sin() as f32;
            let cos = f64::from(self.phase).cos() as f32;
            let quadrature = im * sin;
            self.phase += step;
            *out = re.mul_add(cos, -quadrature);
        }
        let turns = fctiwz(self.phase * INV_TWO_PI) as f32;
        self.phase = fnmsubs(turns, TWO_PI, self.phase);
        services.mixer.swap();
        self.last_shift = shift;
        BufferStatus::Available
    }
}
