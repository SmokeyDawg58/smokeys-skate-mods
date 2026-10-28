//! EA-XA audio blocks of EA movies, decoded as `rw::movie::AudioDecoder_EAXA`
//! (`Decode` `0x82b955b8`) and `rw::movie::Snd::CEAXABLKDecf::Decode`
//! (`0x82b94ef0`) do. A block (`SDEN` payload) is a big-endian sample count,
//! one big-endian offset per channel (from the end of the offsets), then
//! each channel's 15-byte frames of 28 samples. The predictor runs in
//! single floats (`decodexac` `0x82b94b40`) with a two-sample history that
//! carries across blocks; the output is truncated to an integer and its low
//! 16 bits are the PCM sample.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EaXaError {
    Truncated,
    ChannelCount { expected: usize },
}

impl fmt::Display for EaXaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid EA-XA block: {self:?}")
    }
}

impl std::error::Error for EaXaError {}

/// `rw::movie::Snd::xafilterf` (`0x8300922c`): the predictor coefficients
/// of the previous and the second previous sample.
const FILTER: [[f32; 4]; 2] = [
    [0.0, 0.9375, 1.796875, 1.53125],
    [0.0, 0.0, -0.8125, -0.859375],
];

/// A raw frame (`process_raw_block` `0x82b948d0`): this header byte, two
/// big-endian history samples, then 28 big-endian samples.
const RAW_FRAME: u8 = 0xEE;

/// `rw::movie::Snd::xatablef` (`0x83009250`): the signed nibble times 4096
/// shifted right by the frame's shift; shifts 13 to 15 give zero.
fn table(shift: u8, nibble: u8) -> f32 {
    if shift > 12 {
        return 0.0;
    }
    let signed = i32::from((nibble << 4) as i8 >> 4);
    #[expect(clippy::cast_precision_loss, reason = "exact small integers")]
    let value = ((signed << 12) >> shift) as f32;
    value
}

/// `MXAPACKETF`'s history of one channel.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct History {
    sample1: f32,
    sample2: f32,
}

/// The decoder state of every channel (`CEAXABLKDecf`, zeroed by
/// `AudioDecoder_EAXA::Init` `0x82b93778`).
#[derive(Clone, Debug, PartialEq)]
pub struct EaXaDecoder {
    channels: Vec<History>,
}

impl EaXaDecoder {
    #[must_use]
    pub fn new(channels: usize) -> Self {
        Self {
            channels: vec![History::default(); channels],
        }
    }

    #[must_use]
    pub fn channels(&self) -> usize {
        self.channels.len()
    }

    /// Decodes one block and appends its samples to `out`, interleaved by
    /// channel. Returns the samples per channel. A last frame cut short by
    /// the block's count is decoded whole and its remainder dropped, as the
    /// native's residual is cleared when a block runs out.
    ///
    /// # Errors
    ///
    /// Returns [`EaXaError::Truncated`] when the offsets or frames run past
    /// the block.
    pub fn decode_block(&mut self, block: &[u8], out: &mut Vec<i16>) -> Result<usize, EaXaError> {
        let channels = self.channels.len();
        let word = |at: usize| {
            block
                .get(at..at + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
        };
        let samples = word(0).ok_or(EaXaError::Truncated)?;
        let data_start = 4 * (channels + 1);
        let mut decoded = Vec::with_capacity(channels);
        for (channel, history) in self.channels.iter_mut().enumerate() {
            let offset = word(4 * (channel + 1)).ok_or(EaXaError::Truncated)?;
            let source = block
                .get(data_start + offset..)
                .ok_or(EaXaError::Truncated)?;
            decoded.push(decode_channel(source, samples, history)?);
        }
        out.reserve(samples * channels);
        for i in 0..samples {
            for channel in &decoded {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "fctiwz, then the low halfword"
                )]
                out.push((channel[i] as i32) as i16);
            }
        }
        Ok(samples)
    }
}

/// `decodexac` over the frames a block holds for one channel.
fn decode_channel(
    source: &[u8],
    samples: usize,
    history: &mut History,
) -> Result<Vec<f32>, EaXaError> {
    let mut out = Vec::with_capacity(samples.div_ceil(28) * 28);
    let mut at = 0;
    let (mut previous2, mut previous1) = (history.sample2, history.sample1);
    while out.len() < samples {
        let header = *source.get(at).ok_or(EaXaError::Truncated)?;
        if header == RAW_FRAME {
            let frame = source.get(at + 1..at + 61).ok_or(EaXaError::Truncated)?;
            let value = |i: usize| f32::from(i16::from_be_bytes([frame[2 * i], frame[2 * i + 1]]));
            history.sample1 = value(0);
            history.sample2 = value(1);
            for i in 0..28 {
                out.push(value(2 + i));
            }
            previous2 = out[out.len() - 2];
            previous1 = out[out.len() - 1];
            at += 61;
        } else {
            let frame = source.get(at + 1..at + 15).ok_or(EaXaError::Truncated)?;
            let coefficient = usize::from(header >> 4);
            let shift = header & 0x0F;
            let (k0, k1) = (FILTER[0][coefficient & 3], FILTER[1][coefficient & 3]);
            for &byte in frame {
                for nibble in [byte >> 4, byte & 0x0F] {
                    let sample = previous1.mul_add(k0, previous2.mul_add(k1, table(shift, nibble)));
                    previous2 = previous1;
                    previous1 = sample;
                    out.push(sample);
                }
            }
            history.sample2 = previous2;
            history.sample1 = previous1;
            at += 15;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(samples: u32, channel_frames: &[&[u8]]) -> Vec<u8> {
        let mut bytes = samples.to_be_bytes().to_vec();
        let mut offset = 0u32;
        for frames in channel_frames {
            bytes.extend_from_slice(&offset.to_be_bytes());
            offset += u32::try_from(frames.len()).unwrap();
        }
        for frames in channel_frames {
            bytes.extend_from_slice(frames);
        }
        bytes
    }

    #[test]
    fn the_table_matches_the_native_rows() {
        assert_eq!(table(0, 1), 4096.0);
        assert_eq!(table(0, 8), -32768.0);
        assert_eq!(table(12, 15), -1.0);
        assert_eq!(table(13, 7), 0.0);
    }

    #[test]
    fn decodes_frames_and_carries_the_history_across_blocks() {
        let mut first = [0u8; 15];
        first[0] = 0x0C;
        first[1] = 0x71;
        first[14] = 0x23;
        let mut second = [0u8; 15];
        second[0] = 0x2C;
        let mut decoder = EaXaDecoder::new(1);
        let mut out = Vec::new();
        assert_eq!(
            decoder
                .decode_block(&block(28, &[&first]), &mut out)
                .unwrap(),
            28
        );
        assert_eq!(&out[..3], &[7, 1, 0]);
        assert_eq!(&out[26..], &[2, 3]);
        out.clear();
        decoder
            .decode_block(&block(28, &[&second]), &mut out)
            .unwrap();
        // 3 * 1.796875 - 2 * 0.8125, then 3.765625 * 1.796875 - 3 * 0.8125.
        assert_eq!(&out[..2], &[3, 4]);
    }

    #[test]
    fn raw_frames_copy_their_samples() {
        let mut raw = vec![RAW_FRAME, 0, 5, 0, 6];
        for i in 0..28i16 {
            raw.extend_from_slice(&(i - 14).to_be_bytes());
        }
        let mut decoder = EaXaDecoder::new(1);
        let mut out = Vec::new();
        decoder.decode_block(&block(28, &[&raw]), &mut out).unwrap();
        assert_eq!(out[0], -14);
        assert_eq!(out[27], 13);
    }
}
