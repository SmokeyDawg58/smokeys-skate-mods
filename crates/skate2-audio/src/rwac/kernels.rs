//! RWAC sample kernels (`rwaudiocore` common code). The aligned VMX paths are the ones the mixer
//! reaches (128-byte aligned buffers, 256 samples); fused `vmaddfp` operations use
//! [`crate::math::fmadd`].

use crate::math::fmadd;

/// `CopyWithGain` 0x82bdc2c8: `dst = src * gain`.
pub fn copy_with_gain(dst: &mut [f32], src: &[f32], gain: f32) {
    for (d, s) in dst.iter_mut().zip(src) {
        *d = *s * gain;
    }
}

/// `MixWithGain` 0x82be13f8: `dst += src * gain` (`vmaddfp`).
pub fn mix_with_gain(dst: &mut [f32], src: &[f32], gain: f32) {
    for (d, s) in dst.iter_mut().zip(src) {
        *d = fmadd(*s, gain, *d);
    }
}

/// The per-sample gains of `CopyWithGainRamp` 0x82bdc458 / `MixWithGainRamp` 0x82be15d8: lanes
/// start at `gain + lane * inc`; each 32-sample step holds eight 4-sample vectors at
/// `base + j * (4 * inc)` and moves the base by `8 * (4 * inc)`; after 64 samples the gain holds
/// at `gain + 64 * inc`.
fn ramp_gains(gain: f32, inc: f32, len: usize) -> impl Iterator<Item = f32> {
    let four = inc * 4.0;
    let mut base = [gain, gain + inc, fmadd(inc, 2.0, gain), fmadd(inc, 3.0, gain)];
    let mut ramp = [0.0f32; 64];
    for step in 0..2 {
        for j in 0..8 {
            for lane in 0..4 {
                let g = if j == 0 { base[lane] } else { fmadd(j as f32, four, base[lane]) };
                ramp[step * 32 + j * 4 + lane] = g;
            }
        }
        for lane in &mut base {
            *lane = fmadd(8.0, four, *lane);
        }
    }
    let hold = fmadd(inc, 64.0, gain);
    (0..len).map(move |k| if k < 64 { ramp[k] } else { hold })
}

/// `CopyWithGainRamp` 0x82bdc458.
pub fn copy_with_gain_ramp(dst: &mut [f32], src: &[f32], gain: f32, inc: f32) {
    for ((d, s), g) in dst.iter_mut().zip(src).zip(ramp_gains(gain, inc, src.len())) {
        *d = *s * g;
    }
}

/// `MixWithGainRamp` 0x82be15d8.
pub fn mix_with_gain_ramp(dst: &mut [f32], src: &[f32], gain: f32, inc: f32) {
    for ((d, s), g) in dst.iter_mut().zip(src).zip(ramp_gains(gain, inc, src.len())) {
        *d = fmadd(*s, g, *d);
    }
}

/// `FoldDownMapper::sFoldDownGroups[src - 1][dst - 1]`: inclusive ranges in [`FOLD_DOWN_MAPS`];
/// `None` where the pair has no mapping (3 or 5 channels).
const FOLD_DOWN_GROUPS: [[Option<(u8, u8)>; 6]; 6] = [
    [Some((40, 40)), Some((0, 1)), None, Some((2, 3)), None, Some((4, 4))],
    [Some((5, 6)), Some((40, 41)), None, Some((7, 8)), None, Some((9, 10))],
    [None; 6],
    [Some((11, 14)), Some((15, 18)), None, Some((40, 43)), None, Some((19, 22))],
    [None; 6],
    [Some((23, 27)), Some((28, 33)), None, Some((34, 39)), None, Some((40, 45))],
];

/// `FoldDownMapper::sFoldDownMaps`: `gain index << 6 | src << 3 | dst`.
const FOLD_DOWN_MAPS: [u8; 46] = [
    0x40, 0x41, 0x40, 0x41, 0x01, 0x00, 0x08, 0x00, 0x09, 0x00, 0x0a, 0x00, 0x08, 0x10, 0x18, 0x00,
    0x09, 0x10, 0x19, 0x00, 0x0a, 0x13, 0x1c, 0x08, 0x10, 0x20, 0x18, 0x00, 0x48, 0x00, 0x18, 0x49,
    0x11, 0x21, 0x48, 0x00, 0x1a, 0x49, 0x11, 0x23, 0x00, 0x09, 0x12, 0x1b, 0x24, 0x2d,
];

/// `FoldDownMapper::sFoldDownGains`.
const FOLD_DOWN_GAINS: [f32; 3] = [1.0, 0.707, 0.5];

fn fold_down_maps(src_channels: usize, dst_channels: usize) -> Option<&'static [u8]> {
    if src_channels == 0 || dst_channels == 0 || src_channels > 6 || dst_channels > 6 {
        return None;
    }
    FOLD_DOWN_GROUPS[src_channels - 1][dst_channels - 1]
        .map(|(start, end)| &FOLD_DOWN_MAPS[usize::from(start)..=usize::from(end)])
}

fn map_parts(map: u8) -> (usize, usize, f32) {
    (
        usize::from(map & 7),
        usize::from((map >> 3) & 7),
        FOLD_DOWN_GAINS[usize::from(map >> 6).min(2)],
    )
}

/// `ReChannelGainMix` 0x82be26f8: mixes `src` channels into `dst` channels through the fold-down
/// map for the pair.
pub fn rechannel_gain_mix(dst: &mut [&mut [f32]], src: &[&[f32]], gain: f32) {
    if let Some(maps) = fold_down_maps(src.len(), dst.len()) {
        for &map in maps {
            let (d, s, g) = map_parts(map);
            mix_with_gain(dst[d], src[s], g * gain);
        }
    }
}

/// `ReChannelGainMixRamp` 0x82be27a0: as [`rechannel_gain_mix`], ramping from `from_gain`
/// towards `to_gain` with the increment `(to - from) * (1 / 65)`.
pub fn rechannel_gain_mix_ramp(dst: &mut [&mut [f32]], src: &[&[f32]], to_gain: f32, from_gain: f32) {
    let inc = (to_gain - from_gain) * 0.015_384_615;
    if let Some(maps) = fold_down_maps(src.len(), dst.len()) {
        for &map in maps {
            let (d, s, g) = map_parts(map);
            mix_with_gain_ramp(dst[d], src[s], g * from_gain, g * inc);
        }
    }
}

/// `ReChannelGainWrite` 0x82be74c8 / `FoldDownMapper::Fold` 0x82be6b00: writes (not mixes)
/// `src` into `dst`; destination channels no map writes are zeroed.
pub fn rechannel_gain_write(dst: &mut [&mut [f32]], src: &[&[f32]], gain: f32) {
    let src_channels = src.len();
    let dst_channels = dst.len();
    let supported = |n: usize| matches!(n, 1 | 2 | 4 | 6);
    if supported(src_channels) && supported(dst_channels) {
        let mut written = [false; 6];
        if let Some(maps) = fold_down_maps(src_channels, dst_channels) {
            for &map in maps {
                let (d, s, g) = map_parts(map);
                if written[d] {
                    mix_with_gain(dst[d], src[s], g * gain);
                } else {
                    copy_with_gain(dst[d], src[s], g * gain);
                    written[d] = true;
                }
            }
        }
        for (channel, done) in dst.iter_mut().zip(written) {
            if !done {
                channel.fill(0.0);
            }
        }
    } else {
        let common = src_channels.min(dst_channels);
        for c in 0..common {
            copy_with_gain(dst[c], src[c], gain);
        }
        for channel in dst.iter_mut().skip(common) {
            channel.fill(0.0);
        }
    }
}

/// `DeClick` 0x82bdc978: adds each channel's DC offset decaying over 16 samples
/// (16/17 down to 1/17) and clears the offset.
pub fn declick(channels: &mut [&mut [f32]], offsets: &mut [f32]) {
    const STEPS: [f32; 16] = [
        0.941_176_5, 0.882_352_95, 0.823_529_4, 0.764_705_9, 0.705_882_4, 0.647_058_84, 0.588_235_3,
        0.529_411_8, 0.470_588_24, 0.411_764_7, 0.352_941_2, 0.294_117_66, 0.235_294_12, 0.176_470_6,
        0.117_647_06, 0.058_823_53,
    ];
    for (channel, offset) in channels.iter_mut().zip(offsets.iter_mut()) {
        for (sample, step) in channel.iter_mut().zip(STEPS) {
            *sample = fmadd(*offset, step, *sample);
        }
        *offset = 0.0;
    }
}

/// `ReOrderRwAudioCoreToWave` 0x82be2a40 for six channels: RWAC order FL, C, FR, SL, SR, LFE to
/// interleaved WAVE order FL, FR, C, LFE, SL, SR.
pub fn reorder_to_wave(dst: &mut [f32], src: &[&[f32]; 6]) {
    for (i, frame) in dst.chunks_exact_mut(6).enumerate() {
        frame[0] = src[0][i];
        frame[1] = src[2][i];
        frame[2] = src[1][i];
        frame[3] = src[5][i];
        frame[4] = src[3][i];
        frame[5] = src[4][i];
    }
}

/// `ClipFloats` 0x82bd99b8.
pub fn clip_floats(buffer: &mut [f32], min: f32, max: f32) {
    for value in buffer {
        if *value >= min {
            if *value > max {
                *value = max;
            }
        } else {
            *value = min;
        }
    }
}

/// `LinearInterpolate` 0x82be0b00: `frames` outputs from `src` starting at index `acc_whole`
/// and 16-bit fraction `acc_frac`, stepping by the 16.16 increment `inc`:
/// `out = (s[i+1] - s[i]) * (frac * 0.000015258) + s[i]` (`fmadds`). Returns the new position.
pub fn linear_interpolate(
    dst: &mut [f32],
    src: &[f32],
    acc_whole: u32,
    acc_frac: u32,
    inc: u32,
) -> (u32, u32) {
    const SCALE: f32 = f32::from_bits(0x377F_FC9C);
    let mut whole = acc_whole;
    let mut frac = acc_frac & 0xFFFF;
    for out in dst.iter_mut() {
        let index = whole as usize;
        let s0 = src[index];
        let s1 = src[index + 1];
        let f = (frac as f32) * SCALE;
        *out = fmadd(s1 - s0, f, s0);
        frac += inc;
        whole += frac >> 16;
        frac &= 0xFFFF;
    }
    (whole, frac)
}
