//! `rw::audio::core::SeekTableParser` and `Unpack0`: where to start decoding to reach a sample.

/// `Unpack0::UnpackUInt32` 0x82c0e1f0: 1 to 5 byte unsigned varint; returns (value, bytes).
pub fn unpack_uint32(src: &[u8]) -> (u32, usize) {
    let b = |i: usize| u32::from(src.get(i).copied().unwrap_or(0));
    let b0 = b(0);
    if b0 < 0xC0 {
        (b0, 1)
    } else if b0 < 0xF0 {
        (((b0 << 8 | b(1)) & 0x3FFF) + 192, 2)
    } else if b0 < 0xFC {
        (((b0 << 16 | b(1) << 8 | b(2)) & 0x0F_FFFF) + 12480, 3)
    } else if b0 < 0xFF {
        (((b(1) << 16 | b(2) << 8 | b(3)) & 0x3FF_FFFF | (b0 << 24) & 0x300_0000) + 798_912, 4)
    } else {
        (b(1) << 24 | b(2) << 16 | b(3) << 8 | b(4), 5)
    }
}

/// `Unpack0::UnpackInt32` 0x82be2fa8: zig-zag style signed varint (bit 0 of the last byte is
/// the sign, value `-1 - v` when set); returns (value, bytes).
pub fn unpack_int32(src: &[u8]) -> (i32, usize) {
    let b = |i: usize| u32::from(src.get(i).copied().unwrap_or(0));
    let b0 = b(0);
    let (value, sign, len) = if b0 < 0xC0 {
        (b0 >> 1, b0 & 1, 1)
    } else if b0 < 0xF0 {
        let v = (b0 << 8) | b(1);
        (((v >> 1) & 0xFFFF_9FFF) + 96, b(1) & 1, 2)
    } else if b0 < 0xFC {
        let v = (((b(1) | (b0 << 8)) << 8) & 0xFF0F_FFFF) | b(2);
        ((v >> 1) + 6240, b(2) & 1, 3)
    } else if b0 < 0xFF {
        let high = (b0 << 16) & 0x30000;
        let v = ((((b(1) << 8) | b(2)) & 0x3FFFF | high) << 8) | (b(3) & 0xFE);
        ((v >> 1) + 393_216 + 6240, b(3) & 1, 4)
    } else {
        return ((b(1) << 24 | b(2) << 16 | b(3) << 8 | b(4)) as i32, 5);
    };
    let value = value as i32;
    (if sign != 0 { -1 - value } else { value }, len)
}

/// `PackedColumnReader`: run-length coded deltas; all columns share one byte cursor.
#[derive(Default)]
struct Column {
    prev: i32,
    run_remaining: i32,
    like_run: bool,
}

impl Column {
    fn next(&mut self, data: &[u8], cursor: &mut usize) -> i32 {
        let read = |cursor: &mut usize| {
            let (v, n) = unpack_int32(data.get(*cursor..).unwrap_or(&[]));
            *cursor += n;
            v
        };
        if self.run_remaining <= 0 {
            let k = read(cursor);
            self.run_remaining = k + 1;
            self.like_run = true;
            if k < 0 {
                self.like_run = false;
                self.run_remaining = 1 - k;
            }
            if self.like_run {
                self.prev += read(cursor);
            }
        }
        if !self.like_run {
            self.prev += read(cursor);
        }
        self.run_remaining -= 1;
        self.prev
    }
}

/// The result of `SeekTableParser::Parse` 0x82bea880.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SeekPosition {
    pub decoder_skip: i32,
    pub stream_skip: i32,
    pub player_skip: i32,
    pub chunk_offset: u32,
    /// Offset of this chunk's seek data within the table.
    pub seek_data: Option<usize>,
    pub seek_data_version: i32,
    pub is_new_feed_chunk: bool,
}

struct Parser {
    latency: i32,
    seek_data: Option<usize>,
    out: SeekPosition,
}

impl Parser {
    /// Applies one chunk record; true when `sample` lies before its end.
    fn record(&mut self, sample: i32, target: i32, acc: (u32, usize, i32), row: (i32, i32, i32, i32)) -> bool {
        let (acc_bytes, acc_seek, acc_samples) = acc;
        let (_, seek_bytes, samples, new_feed) = row;
        if (acc_samples <= target && target < samples + acc_samples) || new_feed == 1 {
            self.out.seek_data = if seek_bytes != 0 { self.seek_data.map(|base| base + acc_seek) } else { None };
            self.out.stream_skip = acc_samples;
            let player_skip = (sample - acc_samples).min(self.latency);
            self.out.player_skip = player_skip;
            self.out.chunk_offset = acc_bytes;
            self.out.decoder_skip = sample - player_skip - acc_samples;
            self.out.is_new_feed_chunk = new_feed == 1;
        }
        sample < samples + acc_samples
    }

    /// `ParseChunkSection0` 0x82be7618 (16-byte records) and `ParseChunkSection1` 0x82be8c90
    /// (packed columns). Returns true on failure.
    fn sections(&mut self, table: &[u8], start: usize, packed: bool, sample: i32) -> bool {
        let target = (sample - self.latency).max(0);
        let mut acc = (0u32, 0usize, 0i32);
        let mut columns: [Column; 4] = Default::default();
        let mut cursor = start;
        let mut index = 0usize;
        let mut read_row = |cursor: &mut usize, index: &mut usize| -> (i32, i32, i32, i32) {
            if packed {
                (
                    columns[0].next(table, cursor),
                    columns[1].next(table, cursor),
                    columns[2].next(table, cursor),
                    columns[3].next(table, cursor),
                )
            } else {
                let at = start + *index * 16;
                *index += 1;
                let word = |o: usize| table.get(at + o..at + o + 4).map_or(-1, |b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]));
                (word(0), word(4), word(8), word(12))
            }
        };
        let mut row = read_row(&mut cursor, &mut index);
        if row.2 < 0 {
            return true;
        }
        loop {
            if self.record(sample, target, acc, row) {
                return false;
            }
            acc.0 = acc.0.wrapping_add(row.0 as u32);
            acc.1 += row.1.max(0) as usize;
            acc.2 += row.2;
            row = read_row(&mut cursor, &mut index);
            if row.2 < 0 {
                return true;
            }
        }
    }
}

/// `SeekTableParser::Parse` 0x82bea880 for the first chunk that holds `sample`.
pub fn parse(table: &[u8], sample: i32) -> SeekPosition {
    let byte = |i: usize| table.get(i).copied().unwrap_or(0);
    let word = |i: usize| table.get(i..i + 4).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let format = byte(0);
    let version = i32::from(byte(1) & 0xF);
    let packed = byte(1) >> 4;
    let latency = i32::from(u16::from_be_bytes([byte(2), byte(3)]));
    let mut parser = Parser { latency, seek_data: None, out: SeekPosition { seek_data_version: version, ..Default::default() } };
    let failed = match format {
        0 => {
            let offset = word(4) as usize;
            parser.seek_data = (offset != 0).then_some(offset);
            packed > 1 || parser.sections(table, 8, packed == 1, sample)
        }
        1 => {
            let section = word(4) as usize;
            let offset = word(8) as usize;
            parser.seek_data = (offset != 0).then_some(offset);
            packed > 1 || parser.sections(table, section, packed == 1, sample)
        }
        _ => true,
    };
    if failed {
        parser.out.seek_data = None;
        parser.out.stream_skip = 0;
        parser.out.decoder_skip = 0;
        parser.out.chunk_offset = 0;
        parser.out.seek_data_version = 0;
    }
    parser.out
}
