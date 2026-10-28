// Copyright (c) 2004 Michael Niedermayer
// SPDX-License-Identifier: LGPL-2.1-or-later
//
// Bit reader and writer with the semantics of FFmpeg's checked GetBitContext and
// PutBitContext (libavcodec/get_bits.h, put_bits.h): big-endian, most significant bit
// first; reads past the end return zero bits and the position stops 8 bits past the end.

pub(crate) struct BitReader<'a> {
    data: &'a [u8],
    index: usize,
    size_in_bits: usize,
    size_in_bits_plus8: usize,
}

impl<'a> BitReader<'a> {
    pub(crate) fn new(data: &'a [u8], size_in_bits: usize) -> Self {
        Self {
            data,
            index: 0,
            size_in_bits,
            size_in_bits_plus8: size_in_bits + 8,
        }
    }

    fn bit(&self, position: usize) -> u32 {
        self.data
            .get(position >> 3)
            .map_or(0, |byte| u32::from(byte >> (7 - (position & 7)) & 1))
    }

    fn peek(&self, count: u32) -> u32 {
        if count == 0 { 0 } else { self.show_bits32() >> (32 - count) }
    }

    pub(crate) fn show_bits(&self, count: u32) -> u32 {
        self.peek(count)
    }

    /// The next 32 bits, left aligned (for VLC lookup).
    pub(crate) fn show_bits32(&self) -> u32 {
        let byte = self.index >> 3;
        if byte + 5 <= self.data.len() {
            let word = u64::from_be_bytes([
                self.data[byte],
                self.data[byte + 1],
                self.data[byte + 2],
                self.data[byte + 3],
                self.data[byte + 4],
                0,
                0,
                0,
            ]);
            return (word << (self.index & 7) >> 32) as u32;
        }
        (0..32).fold(0u32, |value, offset| (value << 1) | self.bit(self.index + offset))
    }

    pub(crate) fn get_bits(&mut self, count: u32) -> u32 {
        let value = self.peek(count);
        self.skip_bits(count as usize);
        value
    }

    pub(crate) fn get_bits1(&mut self) -> u32 {
        self.get_bits(1)
    }

    /// `get_sbits`: a two's complement field.
    pub(crate) fn get_sbits(&mut self, count: u32) -> i32 {
        let value = self.get_bits(count);
        ((value << (32 - count)) as i32) >> (32 - count)
    }

    /// `get_bitsz`: zero bits read as 0.
    pub(crate) fn get_bitsz(&mut self, count: u32) -> u32 {
        if count == 0 { 0 } else { self.get_bits(count) }
    }

    pub(crate) fn skip_bits(&mut self, count: usize) {
        self.index = (self.index + count).min(self.size_in_bits_plus8);
    }

    pub(crate) fn position(&self) -> usize {
        self.index
    }

    pub(crate) fn bits_left(&self) -> isize {
        self.size_in_bits as isize - self.index as isize
    }

    pub(crate) fn data(&self) -> &'a [u8] {
        self.data
    }
}

/// Big-endian bit writer over a fixed buffer (`PutBitContext` with `ff_copy_bits`).
pub(crate) struct BitWriter {
    pub(crate) buffer: Vec<u8>,
    count: usize,
}

impl BitWriter {
    pub(crate) fn new(bytes: usize) -> Self {
        Self {
            buffer: vec![0; bytes],
            count: 0,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.count = 0;
    }

    pub(crate) fn count(&self) -> usize {
        self.count
    }

    pub(crate) fn put_bits(&mut self, count: u32, value: u32) {
        for offset in (0..count).rev() {
            let bit = (value >> offset) & 1;
            let byte = self.count >> 3;
            let shift = 7 - (self.count & 7);
            if let Some(slot) = self.buffer.get_mut(byte) {
                *slot = (*slot & !(1 << shift)) | ((bit as u8) << shift);
            }
            self.count += 1;
        }
    }

    /// `ff_copy_bits`: the first `length` bits of `source`, zero past its end.
    pub(crate) fn copy_bits(&mut self, source: &[u8], length: usize) {
        let words = length >> 4;
        let bits = (length & 15) as u32;
        let read16 = |index: usize| -> u32 {
            let high = source.get(index).copied().unwrap_or(0);
            let low = source.get(index + 1).copied().unwrap_or(0);
            u32::from(u16::from_be_bytes([high, low]))
        };
        for word in 0..words {
            self.put_bits(16, read16(2 * word));
        }
        if bits != 0 {
            self.put_bits(bits, read16(2 * words) >> (16 - bits));
        }
    }
}
