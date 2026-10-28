//! Byte and bit codecs for the PC online protocol: little-endian scalars, LEB128 varints,
//! zigzag integers, and an MSB-first bit stream with Elias gamma codes for the frame codec.

use std::fmt;

/// A malformed or truncated message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError(pub &'static str);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "malformed online message: {}", self.0)
    }
}

impl std::error::Error for DecodeError {}

pub type DecodeResult<T> = Result<T, DecodeError>;

/// Maps a signed value onto the unsigned line so small magnitudes stay small.
#[must_use]
pub fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)).cast_unsigned()
}

#[must_use]
pub fn unzigzag(value: u64) -> i64 {
    (value >> 1).cast_signed() ^ -((value & 1).cast_signed())
}

#[derive(Clone, Debug, Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self { buf: Vec::with_capacity(capacity) }
    }

    pub fn u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    pub fn bool(&mut self, value: bool) {
        self.buf.push(u8::from(value));
    }

    pub fn u16(&mut self, value: u16) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn u64(&mut self, value: u64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn f32(&mut self, value: f32) {
        self.u32(value.to_bits());
    }

    pub fn varint(&mut self, mut value: u64) {
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                self.buf.push(byte);
                return;
            }
            self.buf.push(byte | 0x80);
        }
    }

    pub fn signed(&mut self, value: i64) {
        self.varint(zigzag(value));
    }

    /// A length-prefixed byte string.
    pub fn bytes(&mut self, value: &[u8]) {
        self.varint(value.len() as u64);
        self.buf.extend_from_slice(value);
    }

    pub fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    /// Bytes without a length, for a field whose size the reader knows.
    pub fn raw(&mut self, value: &[u8]) {
        self.buf.extend_from_slice(value);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}

#[derive(Clone, Debug)]
pub struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.data.len() - self.at
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.at == self.data.len()
    }

    /// Fails unless every byte was read.
    pub fn finish(&self) -> DecodeResult<()> {
        if self.is_empty() { Ok(()) } else { Err(DecodeError("trailing bytes")) }
    }

    pub fn raw(&mut self, count: usize) -> DecodeResult<&'a [u8]> {
        let end = self.at.checked_add(count).ok_or(DecodeError("length overflow"))?;
        let slice = self.data.get(self.at..end).ok_or(DecodeError("truncated"))?;
        self.at = end;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> DecodeResult<[u8; N]> {
        let mut out = [0; N];
        out.copy_from_slice(self.raw(N)?);
        Ok(out)
    }

    pub fn u8(&mut self) -> DecodeResult<u8> {
        Ok(self.raw(1)?[0])
    }

    pub fn bool(&mut self) -> DecodeResult<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError("invalid boolean")),
        }
    }

    pub fn u16(&mut self) -> DecodeResult<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub fn u32(&mut self) -> DecodeResult<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub fn u64(&mut self) -> DecodeResult<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub fn f32(&mut self) -> DecodeResult<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    pub fn varint(&mut self) -> DecodeResult<u64> {
        let mut value = 0_u64;
        for shift in (0..64).step_by(7) {
            let byte = self.u8()?;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(DecodeError("varint too long"))
    }

    pub fn varint_u32(&mut self) -> DecodeResult<u32> {
        u32::try_from(self.varint()?).map_err(|_| DecodeError("varint exceeds 32 bits"))
    }

    pub fn len(&mut self, max: usize) -> DecodeResult<usize> {
        let length = usize::try_from(self.varint()?).map_err(|_| DecodeError("length overflow"))?;
        if length > max || length > self.remaining() {
            return Err(DecodeError("length out of range"));
        }
        Ok(length)
    }

    pub fn signed(&mut self) -> DecodeResult<i64> {
        Ok(unzigzag(self.varint()?))
    }

    pub fn bytes(&mut self, max: usize) -> DecodeResult<&'a [u8]> {
        let length = self.len(max)?;
        self.raw(length)
    }

    pub fn string(&mut self, max: usize) -> DecodeResult<String> {
        let bytes = self.bytes(max)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| DecodeError("invalid UTF-8"))
    }
}

/// An MSB-first bit stream.
#[derive(Clone, Debug, Default)]
pub struct BitWriter {
    bytes: Vec<u8>,
    accumulator: u64,
    bits: u32,
}

impl BitWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The low `count` bits of `value`, most significant first.
    pub fn write(&mut self, value: u64, count: u32) {
        debug_assert!(count <= 64);
        let mut remaining = count;
        while remaining > 0 {
            let take = remaining.min(32);
            remaining -= take;
            let chunk = (value >> remaining) & ((1_u64 << take) - 1);
            self.accumulator = (self.accumulator << take) | chunk;
            self.bits += take;
            while self.bits >= 8 {
                self.bits -= 8;
                self.bytes.push((self.accumulator >> self.bits).to_le_bytes()[0]);
            }
            self.accumulator &= (1_u64 << self.bits) - 1;
        }
    }

    pub fn bit(&mut self, value: bool) {
        self.write(u64::from(value), 1);
    }

    /// Elias gamma of `value + 1`: zero is one bit, `n` costs `2 * floor(log2(n + 1)) + 1`.
    pub fn gamma(&mut self, value: u64) {
        let coded = value.wrapping_add(1);
        if coded == 0 {
            self.write(0, 64);
            self.write(1, 1);
            self.write(value, 64);
            return;
        }
        let width = 64 - coded.leading_zeros();
        self.write(0, width - 1);
        self.write(coded, width);
    }

    pub fn signed(&mut self, value: i64) {
        self.gamma(zigzag(value));
    }

    #[must_use]
    pub fn bit_len(&self) -> usize {
        self.bytes.len() * 8 + self.bits as usize
    }

    #[must_use]
    pub fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            let pad = 8 - self.bits;
            self.bytes.push((self.accumulator << pad).to_le_bytes()[0]);
        }
        self.bytes
    }
}

#[derive(Clone, Debug)]
pub struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, bit: 0 }
    }

    pub fn read(&mut self, count: u32) -> DecodeResult<u64> {
        debug_assert!(count <= 64);
        if self.bit + count as usize > self.data.len() * 8 {
            return Err(DecodeError("bit stream truncated"));
        }
        let mut value = 0_u64;
        for _ in 0..count {
            let byte = self.data[self.bit / 8];
            let bit = (byte >> (7 - self.bit % 8)) & 1;
            value = (value << 1) | u64::from(bit);
            self.bit += 1;
        }
        Ok(value)
    }

    pub fn bit(&mut self) -> DecodeResult<bool> {
        Ok(self.read(1)? == 1)
    }

    pub fn gamma(&mut self) -> DecodeResult<u64> {
        let mut zeros = 0_u32;
        while !self.bit()? {
            zeros += 1;
            if zeros > 64 {
                return Err(DecodeError("gamma code too long"));
            }
        }
        if zeros == 64 {
            return self.read(64);
        }
        let rest = self.read(zeros)?;
        Ok(((1_u64 << zeros) | rest) - 1)
    }

    pub fn signed(&mut self) -> DecodeResult<i64> {
        Ok(unzigzag(self.gamma()?))
    }

    /// Fails unless only the final byte's padding is left.
    pub fn finish(&self) -> DecodeResult<()> {
        if self.data.len() == self.bit.div_ceil(8) { Ok(()) } else { Err(DecodeError("trailing frame bytes")) }
    }

    /// The bytes read so far, the last one's padding included.
    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.bit.div_ceil(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_and_varints_round_trip() {
        let mut w = Writer::new();
        w.u8(7);
        w.bool(true);
        w.u16(0xbeef);
        w.u32(0xdead_beef);
        w.u64(u64::MAX - 3);
        w.f32(-1.5);
        for value in [0, 1, 127, 128, 300, u64::MAX] {
            w.varint(value);
        }
        for value in [0, -1, 1, -64, 64, i64::MIN, i64::MAX] {
            w.signed(value);
        }
        w.string("Skater99");
        let bytes = w.into_vec();
        let mut r = Reader::new(&bytes);
        assert_eq!(r.u8().unwrap(), 7);
        assert!(r.bool().unwrap());
        assert_eq!(r.u16().unwrap(), 0xbeef);
        assert_eq!(r.u32().unwrap(), 0xdead_beef);
        assert_eq!(r.u64().unwrap(), u64::MAX - 3);
        assert_eq!(r.f32().unwrap().to_bits(), (-1.5_f32).to_bits());
        for value in [0, 1, 127, 128, 300, u64::MAX] {
            assert_eq!(r.varint().unwrap(), value);
        }
        for value in [0, -1, 1, -64, 64, i64::MIN, i64::MAX] {
            assert_eq!(r.signed().unwrap(), value);
        }
        assert_eq!(r.string(15).unwrap(), "Skater99");
        r.finish().unwrap();
    }

    #[test]
    fn truncated_input_is_an_error() {
        let mut r = Reader::new(&[0x80, 0x80]);
        assert!(r.varint().is_err());
        let mut r = Reader::new(&[5, b'a']);
        assert!(r.string(15).is_err());
    }

    #[test]
    fn bit_stream_round_trips() {
        let mut w = BitWriter::new();
        w.bit(true);
        w.write(0x2d, 6);
        w.write(u64::MAX, 64);
        let values = [0_u64, 1, 2, 3, 254, 255, 1 << 40, u64::MAX - 1, u64::MAX];
        for value in values {
            w.gamma(value);
        }
        for value in [0_i64, -1, 1, -1000, 1000, i64::MIN, i64::MAX] {
            w.signed(value);
        }
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        assert!(r.bit().unwrap());
        assert_eq!(r.read(6).unwrap(), 0x2d);
        assert_eq!(r.read(64).unwrap(), u64::MAX);
        for value in values {
            assert_eq!(r.gamma().unwrap(), value);
        }
        for value in [0_i64, -1, 1, -1000, 1000, i64::MIN, i64::MAX] {
            assert_eq!(r.signed().unwrap(), value);
        }
        r.finish().unwrap();
    }

    #[test]
    fn gamma_zero_costs_one_bit() {
        let mut w = BitWriter::new();
        for _ in 0..16 {
            w.gamma(0);
        }
        assert_eq!(w.bit_len(), 16);
    }
}
