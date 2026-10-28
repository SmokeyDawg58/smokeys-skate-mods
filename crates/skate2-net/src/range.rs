//! An adaptive binary range coder for the replay frames (the LZMA coder's arithmetic: a 32-bit
//! range over a byte stream that carries into the bytes already out). Each decision is coded with
//! the probability its context has learnt so far in the same message, so a value that seldom
//! changes costs a small fraction of a bit. Every message starts its contexts afresh, so a lost
//! one never parts the two sides' models.

use crate::codec::{DecodeError, DecodeResult};

/// Probabilities are fifteen-bit fractions.
const PROBABILITY_BITS: u32 = 15;
const ONE: u32 = 1 << PROBABILITY_BITS;
/// The range is renormalised whenever it falls below this.
const TOP: u32 = 1 << 24;
/// A context moves toward each decision by `1 / (seen + 2)`, the running estimate of their mean,
/// until that step is this small; then it keeps following the latest decisions.
const SLOWEST_STEP: u32 = 32;
/// Magnitude lengths with a context of their own; longer ones share the last.
const LENGTHS: usize = 24;
/// The longest magnitude: 64 bits.
const MAX_LENGTH: u32 = 63;

/// The probability that one kind of binary decision is zero, as learnt from those seen.
#[derive(Clone, Copy, Debug)]
pub struct Bit {
    zero: u16,
    seen: u8,
}

impl Default for Bit {
    fn default() -> Self {
        Self { zero: 1 << (PROBABILITY_BITS - 1), seen: 0 }
    }
}

impl Bit {
    fn zero(self) -> u32 {
        u32::from(self.zero)
    }

    fn update(&mut self, bit: bool) {
        let step = (u32::from(self.seen) + 2).min(SLOWEST_STEP);
        self.seen = self.seen.saturating_add(1);
        let zero = self.zero();
        // The step is at least two, so the probability never reaches zero or one.
        let zero = if bit { zero - zero / step } else { zero + (ONE - zero) / step };
        self.zero = u16::try_from(zero).unwrap_or(u16::MAX);
    }
}

/// The contexts of one kind of integer: whether it is zero, its sign, its magnitude's length in
/// bits (unary, a context per step), and for each length the bit after the leading one; the
/// lower bits go as they are.
#[derive(Clone, Debug)]
pub struct Integer {
    zero: Bit,
    sign: Bit,
    length: [Bit; LENGTHS],
    second: [Bit; LENGTHS],
}

impl Default for Integer {
    fn default() -> Self {
        Self { zero: Bit::default(), sign: Bit::default(), length: [Bit::default(); LENGTHS], second: [Bit::default(); LENGTHS] }
    }
}

fn slot(length: u32) -> usize {
    usize::try_from(length).map_or(LENGTHS - 1, |length| length.min(LENGTHS - 1))
}

/// Codes decisions into bytes.
#[derive(Clone, Debug)]
pub struct RangeEncoder {
    low: u64,
    range: u32,
    /// The byte a carry may still reach, and the 0xFF bytes after it a carry would also turn.
    cache: u8,
    pending: u32,
    /// Whether `cache` holds a byte of the message: the first never does (the decoder reads past
    /// it), so it is left out.
    started: bool,
    bytes: Vec<u8>,
}

impl Default for RangeEncoder {
    fn default() -> Self {
        Self { low: 0, range: u32::MAX, cache: 0, pending: 0, started: false, bytes: Vec::new() }
    }
}

impl RangeEncoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bit(&mut self, context: &mut Bit, bit: bool) {
        let bound = (self.range >> PROBABILITY_BITS) * context.zero();
        if bit {
            self.low += u64::from(bound);
            self.range -= bound;
        } else {
            self.range = bound;
        }
        context.update(bit);
        self.normalize();
    }

    /// The low `count` bits of `value`, most significant first, each as likely zero as one.
    pub fn direct(&mut self, value: u64, count: u32) {
        for index in (0..count).rev() {
            self.range >>= 1;
            if (value >> index) & 1 == 1 {
                self.low += u64::from(self.range);
            }
            self.normalize();
        }
    }

    pub fn signed(&mut self, context: &mut Integer, value: i64) {
        self.bit(&mut context.zero, value != 0);
        if value != 0 {
            self.bit(&mut context.sign, value < 0);
            self.magnitude(context, value.unsigned_abs());
        }
    }

    pub fn unsigned(&mut self, context: &mut Integer, value: u64) {
        let magnitude = value.saturating_add(1);
        self.magnitude(context, magnitude);
        // The two largest values share the largest magnitude.
        if magnitude == u64::MAX {
            self.direct(u64::from(value == u64::MAX), 1);
        }
    }

    /// A magnitude of at least one: its length past the leading one in unary, then its bits.
    fn magnitude(&mut self, context: &mut Integer, magnitude: u64) {
        let length = MAX_LENGTH - magnitude.leading_zeros();
        for step in 0..length {
            self.bit(&mut context.length[slot(step)], true);
        }
        if length < MAX_LENGTH {
            self.bit(&mut context.length[slot(length)], false);
        }
        if length > 0 {
            self.bit(&mut context.second[slot(length)], (magnitude >> (length - 1)) & 1 == 1);
            self.direct(magnitude, length - 1);
        }
    }

    fn normalize(&mut self) {
        while self.range < TOP {
            self.range <<= 8;
            self.shift_low();
        }
    }

    fn shift_low(&mut self) {
        if self.low < 0xFF00_0000 || self.low >> 32 != 0 {
            let carry = u8::from(self.low >> 32 != 0);
            if self.started {
                self.bytes.push(self.cache.wrapping_add(carry));
            }
            self.started = true;
            for _ in 0..self.pending {
                self.bytes.push(0xFF_u8.wrapping_add(carry));
            }
            self.pending = 0;
            self.cache = self.low.to_be_bytes()[4];
        } else {
            self.pending += 1;
        }
        self.low = (self.low & 0x00FF_FFFF) << 8;
    }

    /// The message's bytes. The decoder reads zeros past the end, so the coder settles on the
    /// value in its last range with the most trailing zero bits and leaves out the zero bytes it
    /// ends in.
    #[must_use]
    pub fn finish(mut self) -> Vec<u8> {
        let end = self.low + u64::from(self.range);
        for bits in (0..=40).rev() {
            let mask = (1_u64 << bits) - 1;
            let value = (self.low + mask) & !mask;
            if value < end {
                self.low = value;
                break;
            }
        }
        for _ in 0..5 {
            self.shift_low();
        }
        while self.bytes.last() == Some(&0) {
            self.bytes.pop();
        }
        self.bytes
    }
}

/// Reads the decisions a [`RangeEncoder`] coded, with contexts that learn as the encoder's did.
#[derive(Clone, Debug)]
pub struct RangeDecoder<'a> {
    data: &'a [u8],
    at: usize,
    range: u32,
    code: u32,
}

impl<'a> RangeDecoder<'a> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        let mut decoder = Self { data, at: 0, range: u32::MAX, code: 0 };
        for _ in 0..4 {
            decoder.code = (decoder.code << 8) | u32::from(decoder.next());
        }
        decoder
    }

    fn next(&mut self) -> u8 {
        let byte = self.data.get(self.at).copied().unwrap_or(0);
        self.at += 1;
        byte
    }

    pub fn bit(&mut self, context: &mut Bit) -> bool {
        let bound = (self.range >> PROBABILITY_BITS) * context.zero();
        let bit = if self.code < bound {
            self.range = bound;
            false
        } else {
            self.code -= bound;
            self.range -= bound;
            true
        };
        context.update(bit);
        self.normalize();
        bit
    }

    pub fn direct(&mut self, count: u32) -> u64 {
        let mut value = 0;
        for _ in 0..count {
            self.range >>= 1;
            let bit = self.code >= self.range;
            if bit {
                self.code -= self.range;
            }
            value = (value << 1) | u64::from(bit);
            self.normalize();
        }
        value
    }

    /// # Errors
    /// A magnitude longer than 64 bits (a malformed message).
    pub fn signed(&mut self, context: &mut Integer) -> DecodeResult<i64> {
        if !self.bit(&mut context.zero) {
            return Ok(0);
        }
        let negative = self.bit(&mut context.sign);
        let magnitude = self.magnitude(context)?;
        if negative {
            Ok(0_i64.wrapping_sub_unsigned(magnitude))
        } else {
            i64::try_from(magnitude).map_err(|_| DecodeError("integer out of range"))
        }
    }

    /// # Errors
    /// A magnitude longer than 64 bits (a malformed message).
    pub fn unsigned(&mut self, context: &mut Integer) -> DecodeResult<u64> {
        let magnitude = self.magnitude(context)?;
        if magnitude == u64::MAX {
            return Ok(if self.direct(1) == 1 { u64::MAX } else { u64::MAX - 1 });
        }
        Ok(magnitude - 1)
    }

    fn magnitude(&mut self, context: &mut Integer) -> DecodeResult<u64> {
        let mut length = 0;
        while length < MAX_LENGTH && self.bit(&mut context.length[slot(length)]) {
            length += 1;
        }
        if length == 0 {
            return Ok(1);
        }
        let second = u64::from(self.bit(&mut context.second[slot(length)]));
        let rest = self.direct(length - 1);
        Ok((1 << length) | (second << (length - 1)) | rest)
    }

    fn normalize(&mut self) {
        while self.range < TOP {
            self.range <<= 8;
            self.code = (self.code << 8) | u32::from(self.next());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    #[test]
    fn decisions_and_integers_round_trip() {
        let mut seed = 0x9e37_79b9_7f4a_7c15;
        let mut bits = [Bit::default(); 4];
        let mut integers: [Integer; 3] = Default::default();
        let mut script = Vec::new();
        let mut encoder = RangeEncoder::new();
        for _ in 0..5000 {
            let pick = random(&mut seed);
            match pick % 5 {
                0 | 1 => {
                    let context = usize::try_from(pick >> 8).unwrap_or(0) % 4;
                    // Skewed by context, so the contexts learn different odds.
                    let bit = random(&mut seed) % 8 < u64::try_from(context).unwrap();
                    encoder.bit(&mut bits[context], bit);
                    script.push((0, context, u64::from(bit)));
                }
                2 => {
                    let count = u32::try_from(random(&mut seed) % 33).unwrap_or(0);
                    let value = random(&mut seed) & ((1_u64 << count) - 1);
                    encoder.direct(value, count);
                    script.push((1, usize::try_from(count).unwrap(), value));
                }
                3 => {
                    let width = random(&mut seed) % 64;
                    let value = (random(&mut seed) >> width).cast_signed();
                    let value = if pick & 0x100 == 0 { value } else { value.wrapping_neg() };
                    encoder.signed(&mut integers[0], value);
                    script.push((2, 0, value.cast_unsigned()));
                }
                _ => {
                    let value = random(&mut seed) >> (random(&mut seed) % 64);
                    encoder.unsigned(&mut integers[1], value);
                    script.push((3, 1, value));
                }
            }
        }
        for value in [0, 1, -1, i64::MIN, i64::MAX, i64::MIN + 1] {
            encoder.signed(&mut integers[2], value);
            script.push((2, 2, value.cast_unsigned()));
        }
        for value in [0, 1, u64::MAX, u64::MAX - 1, u64::MAX - 2, 1 << 63] {
            encoder.unsigned(&mut integers[2], value);
            script.push((3, 2, value));
        }
        let bytes = encoder.finish();
        let mut bits = [Bit::default(); 4];
        let mut integers: [Integer; 3] = Default::default();
        let mut decoder = RangeDecoder::new(&bytes);
        for &(kind, index, value) in &script {
            match kind {
                0 => assert_eq!(u64::from(decoder.bit(&mut bits[index])), value),
                1 => assert_eq!(decoder.direct(u32::try_from(index).unwrap()), value),
                2 => assert_eq!(decoder.signed(&mut integers[index]).unwrap(), value.cast_signed()),
                _ => assert_eq!(decoder.unsigned(&mut integers[index]).unwrap(), value),
            }
        }
    }

    #[test]
    fn a_decision_that_never_changes_costs_next_to_nothing() {
        let mut encoder = RangeEncoder::new();
        let mut context = Bit::default();
        for _ in 0..2000 {
            encoder.bit(&mut context, false);
        }
        let bytes = encoder.finish();
        assert!(bytes.len() <= 12, "{} bytes", bytes.len());
        let mut decoder = RangeDecoder::new(&bytes);
        let mut context = Bit::default();
        assert!((0..2000).all(|_| !decoder.bit(&mut context)));
    }

    #[test]
    fn an_empty_message_is_no_bytes() {
        assert!(RangeEncoder::new().finish().is_empty());
        let mut encoder = RangeEncoder::new();
        encoder.bit(&mut Bit::default(), true);
        let bytes = encoder.finish();
        assert!(RangeDecoder::new(&bytes).bit(&mut Bit::default()));
    }

    #[test]
    fn garbage_decodes_without_panicking() {
        let mut seed = 7;
        for length in 0..64 {
            let data: Vec<u8> = (0..length).map(|_| random(&mut seed).to_le_bytes()[0]).collect();
            let mut decoder = RangeDecoder::new(&data);
            let mut integer = Integer::default();
            let mut bit = Bit::default();
            for _ in 0..200 {
                let _ = decoder.signed(&mut integer);
                let _ = decoder.unsigned(&mut integer);
                let _ = decoder.bit(&mut bit);
                let _ = decoder.direct(17);
            }
        }
    }
}
