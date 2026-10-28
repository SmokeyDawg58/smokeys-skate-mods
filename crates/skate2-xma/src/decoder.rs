// Copyright (c) 2007 Baptiste Coudurier, Benjamin Larsson, Ulion
// Copyright (c) 2008 - 2011 Sascha Sommer, Benjamin Larsson
// SPDX-License-Identifier: LGPL-2.1-or-later
//
// One XMA stream (one or two channels), ported from FFmpeg's libavcodec/wmaprodec.c
// (`WMAProDecodeCtx` with the XMA2 configuration: decode flags 0x10d6, 16-bit scaling,
// 512 samples per frame, 2048-byte packets). Function names follow the C source.
//
// Difference from FFmpeg: `skip_frame` is not applied. The Xbox 360 XMA hardware writes the
// first frame of a stream like every other one, and the game's decoder skips the codec
// delay itself.

use crate::bits::{BitReader, BitWriter};
use crate::mdct::Imdct;
use crate::tables::{
    COEF0_LENS, COEF0_LEVEL, COEF0_RUN, COEF0_SYMS, COEF1_LEVEL, COEF1_RUN, COEF1_TABLE,
    CRITICAL_FREQ, SCALE_RL_LEVEL, SCALE_RL_RUN, SCALE_RL_TABLE, SCALE_TABLE, VEC1_TABLE,
    VEC2_TABLE, VEC4_LENS, VEC4_SYMS,
};
use crate::vlc::Vlc;
use std::sync::OnceLock;

const MAX_SUBFRAMES: usize = 32;
const MAX_BANDS: usize = 29;
const MAX_FRAMESIZE: usize = 32768;
const INPUT_PADDING: usize = 64;
const BLOCK_MIN_BITS: u32 = 6;
const BLOCK_SIZES: usize = 8;
const MAX_CHANNELS: usize = 2;
const HUFF_VEC1_SIZE: i32 = 101;

const DECODE_FLAGS: u32 = 0x10d6;
const BITS_PER_SAMPLE: i32 = 16;
/// Samples per channel in every XMA frame.
pub const SAMPLES_PER_FRAME: usize = 512;
/// Bytes in one XMA packet (`block_align`).
pub const PACKET_BYTES: usize = 2048;
const LOG2_FRAME_SIZE: u32 = 15;
const OUT_LEN: usize = SAMPLES_PER_FRAME + SAMPLES_PER_FRAME / 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Only one- and two-channel XMA streams exist.
    Channels(usize),
    /// The context's decoder table is a two-bit field.
    DecoderTable(u32),
}

struct Codes {
    sf: Vlc,
    sf_rl: Vlc,
    coef: [Vlc; 2],
    vec4: Vlc,
    vec2: Vlc,
    vec1: Vlc,
    windows: Vec<Vec<f32>>,
    imdct: Vec<Imdct>,
}

/// `decode_init_static`, plus the IMDCT contexts and windows shared by every stream (all
/// XMA streams scale their IMDCT output for 16-bit samples).
fn codes() -> &'static Codes {
    static CODES: OnceLock<Codes> = OnceLock::new();
    CODES.get_or_init(|| {
        let pairs = |table: &'static [(u8, u8)]| table.iter().map(|&(sym, len)| (len, i32::from(sym)));
        let sf = Vlc::from_lengths(pairs(&SCALE_TABLE), -60);
        let sf_rl = Vlc::from_lengths(pairs(&SCALE_RL_TABLE), 0);
        let coef0 = Vlc::from_lengths(
            COEF0_LENS.iter().zip(COEF0_SYMS.iter()).map(|(&len, &sym)| (len, i32::from(sym))),
            0,
        );
        let coef1 = Vlc::from_lengths(pairs(&COEF1_TABLE), 0);
        let vec4 = Vlc::from_lengths(
            VEC4_LENS.iter().zip(VEC4_SYMS.iter()).map(|(&len, &sym)| (len, i32::from(sym))),
            -1,
        );
        let vec2 = Vlc::from_lengths(pairs(&VEC2_TABLE), -1);
        let vec1 = Vlc::from_lengths(pairs(&VEC1_TABLE), 0);
        // ff_sine_window_init for 2^6..2^13; index = log2(size) - BLOCK_MIN_BITS.
        let windows = (0..BLOCK_SIZES as u32)
            .map(|i| {
                let n = 1usize << (BLOCK_MIN_BITS + i);
                (0..n)
                    .map(|k| {
                        (((k as f64 + 0.5) * (std::f64::consts::PI / (2.0 * n as f64))) as f32).sin()
                    })
                    .collect()
            })
            .collect();
        let imdct = (0..BLOCK_SIZES as u32)
            .map(|i| {
                let scale = 1.0 / f64::from(1u32 << (BLOCK_MIN_BITS + i - 1))
                    / f64::from(1u32 << (BITS_PER_SAMPLE - 1));
                Imdct::new(BLOCK_MIN_BITS + 1 + i, scale)
            })
            .collect();
        Codes {
            sf,
            sf_rl,
            coef: [coef0, coef1],
            vec4,
            vec2,
            vec1,
            windows,
            imdct,
        }
    })
}

#[derive(Clone)]
struct Channel {
    prev_block_len: i32,
    transmit_coefs: bool,
    num_subframes: usize,
    subframe_len: [u16; MAX_SUBFRAMES],
    subframe_offset: [u16; MAX_SUBFRAMES],
    cur_subframe: usize,
    decoded_samples: u16,
    grouped: bool,
    quant_step: i32,
    reuse_sf: bool,
    scale_factor_step: i32,
    max_scale_factor: i32,
    saved_scale_factors: [[i32; MAX_BANDS]; 2],
    scale_factor_idx: usize,
    /// Which of `saved_scale_factors` `scale_factors` points at.
    scale_factors: usize,
    table_idx: usize,
    /// Offset of `coeffs` in `out`.
    coeffs: usize,
    num_vec_coeffs: u16,
    out: [f32; OUT_LEN],
}

impl Channel {
    fn new() -> Self {
        Self {
            prev_block_len: SAMPLES_PER_FRAME as i32,
            transmit_coefs: false,
            num_subframes: 0,
            subframe_len: [0; MAX_SUBFRAMES],
            subframe_offset: [0; MAX_SUBFRAMES],
            cur_subframe: 0,
            decoded_samples: 0,
            grouped: false,
            quant_step: 0,
            reuse_sf: false,
            scale_factor_step: 0,
            max_scale_factor: 0,
            saved_scale_factors: [[0; MAX_BANDS]; 2],
            scale_factor_idx: 0,
            scale_factors: 0,
            table_idx: 0,
            coeffs: 0,
            num_vec_coeffs: 0,
            out: [0.0; OUT_LEN],
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ChannelGroup {
    num_channels: usize,
    transform: bool,
    transform_band: [u8; MAX_BANDS],
    decorrelation_matrix: [f32; MAX_CHANNELS * MAX_CHANNELS],
    channel_data: [usize; MAX_CHANNELS],
}

/// One XMA stream: one or two channels decoded from consecutive 2048-byte packets.
pub struct XmaStream {
    nb_channels: usize,
    max_num_subframes: usize,
    subframe_len_bits: u32,
    max_subframe_len_bit: bool,
    min_samples_per_subframe: usize,
    num_sfb: [usize; BLOCK_SIZES],
    sfb_offsets: [[i16; MAX_BANDS]; BLOCK_SIZES],
    sf_offsets: [[[i8; MAX_BANDS]; BLOCK_SIZES]; BLOCK_SIZES],

    pb: BitWriter,
    num_saved_bits: usize,
    frame_offset: usize,
    packet_offset: usize,
    packet_loss: bool,
    packet_done: bool,
    eof_done: bool,
    skip_packets: u32,

    drc_gain: u32,
    trim_start: u32,
    trim_end: u32,
    parsed_all_subframes: bool,

    subframe_len: usize,
    channels_for_cur_subframe: usize,
    channel_indexes_for_cur_subframe: [usize; MAX_CHANNELS],
    num_bands: usize,
    transmit_num_vec_coeffs: bool,
    table_idx: usize,
    esc_len: u32,
    num_chgroups: usize,
    chgroup: [ChannelGroup; MAX_CHANNELS],
    channel: Vec<Channel>,
    tmp: [f32; SAMPLES_PER_FRAME],
}

fn av_log2(value: u32) -> u32 {
    31 - (value | 1).leading_zeros()
}

/// The rate whose critical frequencies split the scale factor bands, by the XMA context's
/// decoder table (`XMACONTEXTINIT::DecoderTable`, two bits). FFmpeg's `get_rate` picks the same
/// classes from the sample rate: 24000, 32000, 44100, 48000.
fn rate(decoder_table: u32) -> u32 {
    [24000, 32000, 44100, 48000][decoder_table as usize]
}

/// `ff_wma_get_large_val`.
fn get_large_val(gb: &mut BitReader<'_>) -> u32 {
    let mut n_bits = 8;
    if gb.get_bits1() != 0 {
        n_bits += 8;
        if gb.get_bits1() != 0 {
            n_bits += 8;
            if gb.get_bits1() != 0 {
                n_bits += 7;
            }
        }
    }
    gb.get_bits(n_bits)
}

impl XmaStream {
    /// `decode_init` for an XMA2 stream of `channels` channels whose context uses
    /// `decoder_table` (0..=3).
    pub fn new(channels: usize, decoder_table: u32) -> Result<Self, Error> {
        if channels == 0 || channels > MAX_CHANNELS {
            return Err(Error::Channels(channels));
        }
        if decoder_table > 3 {
            return Err(Error::DecoderTable(decoder_table));
        }
        let log2_max_num_subframes = (DECODE_FLAGS & 0x38) >> 3;
        let max_num_subframes = 1usize << log2_max_num_subframes;
        let max_subframe_len_bit = max_num_subframes == 16 || max_num_subframes == 4;
        let subframe_len_bits = av_log2(log2_max_num_subframes) + 1;
        let num_possible_block_sizes = log2_max_num_subframes as usize + 1;
        let min_samples_per_subframe = SAMPLES_PER_FRAME / max_num_subframes;

        let mut num_sfb = [0usize; BLOCK_SIZES];
        let mut sfb_offsets = [[0i16; MAX_BANDS]; BLOCK_SIZES];
        let rate = rate(decoder_table);
        for i in 0..num_possible_block_sizes {
            let subframe_len = (SAMPLES_PER_FRAME >> i) as i32;
            let mut band = 1usize;
            sfb_offsets[i][0] = 0;
            for &freq in CRITICAL_FREQ.iter().take(MAX_BANDS - 1) {
                if i32::from(sfb_offsets[i][band - 1]) >= subframe_len {
                    break;
                }
                let mut offset = (subframe_len * 2 * i32::from(freq)) / rate as i32 + 2;
                offset &= !3;
                if offset > i32::from(sfb_offsets[i][band - 1]) {
                    sfb_offsets[i][band] = offset as i16;
                    band += 1;
                }
                if offset >= subframe_len {
                    break;
                }
            }
            sfb_offsets[i][band - 1] = subframe_len as i16;
            num_sfb[i] = band - 1;
        }

        let mut sf_offsets = [[[0i8; MAX_BANDS]; BLOCK_SIZES]; BLOCK_SIZES];
        for i in 0..num_possible_block_sizes {
            for b in 0..num_sfb[i] {
                let offset = ((i32::from(sfb_offsets[i][b]) + i32::from(sfb_offsets[i][b + 1]) - 1)
                    << i)
                    >> 1;
                for x in 0..num_possible_block_sizes {
                    let mut v = 0usize;
                    while (i32::from(sfb_offsets[x][v + 1]) << x) < offset {
                        v += 1;
                    }
                    sf_offsets[i][x][b] = v as i8;
                }
            }
        }

        Ok(Self {
            nb_channels: channels,
            max_num_subframes,
            subframe_len_bits,
            max_subframe_len_bit,
            min_samples_per_subframe,
            num_sfb,
            sfb_offsets,
            sf_offsets,
            pb: BitWriter::new(MAX_FRAMESIZE + INPUT_PADDING),
            num_saved_bits: 0,
            frame_offset: 0,
            packet_offset: 0,
            packet_loss: true,
            packet_done: false,
            eof_done: false,
            skip_packets: 0,
            drc_gain: 0,
            trim_start: 0,
            trim_end: 0,
            parsed_all_subframes: false,
            subframe_len: 0,
            channels_for_cur_subframe: 0,
            channel_indexes_for_cur_subframe: [0; MAX_CHANNELS],
            num_bands: 0,
            transmit_num_vec_coeffs: false,
            table_idx: 0,
            esc_len: 0,
            num_chgroups: 0,
            chgroup: [ChannelGroup::default(); MAX_CHANNELS],
            channel: vec![Channel::new(); channels],
            tmp: [0.0; SAMPLES_PER_FRAME],
        })
    }

    pub fn channels(&self) -> usize {
        self.nb_channels
    }

    /// The packet-skip count of the last packet header (packets of other streams to pass
    /// before this stream's next packet).
    pub fn skip_packets(&self) -> u32 {
        self.skip_packets
    }

    /// `flush`: clears the overlap and waits for the next packet header.
    pub fn reset(&mut self) {
        for channel in &mut self.channel {
            channel.out[..SAMPLES_PER_FRAME].fill(0.0);
        }
        self.packet_loss = true;
        self.skip_packets = 0;
        self.eof_done = false;
    }

    /// Decodes one 2048-byte packet. `emit` receives every decoded frame: one slice of
    /// 512 samples per channel, scaled to [-1, 1).
    pub fn decode_packet(&mut self, packet: &[u8], emit: &mut dyn FnMut(&[&[f32]])) {
        self.decode_packet_from(packet, None, emit);
    }

    /// As [`Self::decode_packet`], with decoding resuming at the frame `frame_offset_bits` after
    /// the packet header, as the hardware does when the context's input read offset is set: the
    /// bits before it are dropped like a frame cut off by a lost packet. The overlap with the
    /// previous frame is kept.
    pub fn decode_packet_at(&mut self, packet: &[u8], frame_offset_bits: usize, emit: &mut dyn FnMut(&[&[f32]])) {
        self.packet_loss = true;
        self.decode_packet_from(packet, Some(frame_offset_bits), emit);
    }

    fn decode_packet_from(&mut self, packet: &[u8], frame_offset_bits: Option<usize>, emit: &mut dyn FnMut(&[&[f32]])) {
        let size = packet.len().min(PACKET_BYTES);
        let mut pgb = BitReader::new(&packet[..size], size << 3);
        let mut first = true;
        loop {
            let override_bits = if first { frame_offset_bits } else { None };
            if !self.decode_packet_step(&mut pgb, first, override_bits, emit) {
                return;
            }
            first = false;
            if pgb.position() >> 3 >= size {
                return;
            }
        }
    }

    /// `decode_packet` called with an empty packet: the second half of the last frame.
    pub fn drain(&mut self, emit: &mut dyn FnMut(&[&[f32]])) {
        self.packet_done = false;
        if self.eof_done {
            return;
        }
        let tails: Vec<Vec<f32>> = self
            .channel
            .iter()
            .map(|channel| {
                let mut frame = vec![0.0f32; SAMPLES_PER_FRAME];
                frame[..SAMPLES_PER_FRAME / 2].copy_from_slice(&channel.out[..SAMPLES_PER_FRAME / 2]);
                frame
            })
            .collect();
        let slices: Vec<&[f32]> = tails.iter().map(Vec::as_slice).collect();
        emit(&slices);
        self.eof_done = true;
        self.packet_done = true;
    }

    /// One call of `decode_packet` on the rest of the packet; false once it returns an error.
    fn decode_packet_step(
        &mut self,
        gb: &mut BitReader<'_>,
        first: bool,
        frame_offset_bits: Option<usize>,
        emit: &mut dyn FnMut(&[&[f32]]),
    ) -> bool {
        let buf_bit_size = gb.data().len() << 3;
        let remaining = |gb: &BitReader<'_>| buf_bit_size as isize - gb.position() as isize;
        if first && (self.packet_done || self.packet_loss) {
            self.packet_done = false;
            let _num_frames = gb.get_bits(6);
            let header_bits = gb.get_bits(LOG2_FRAME_SIZE) as isize;
            let mut num_bits_prev_frame = frame_offset_bits.map_or(header_bits, |bits| bits as isize);
            gb.skip_bits(3);
            self.skip_packets = gb.get_bits(8);
            if num_bits_prev_frame > 0 {
                let remaining_packet_bits = remaining(gb);
                if num_bits_prev_frame >= remaining_packet_bits {
                    num_bits_prev_frame = remaining_packet_bits;
                    self.packet_done = true;
                }
                self.save_bits(gb, num_bits_prev_frame, true);
                if !self.packet_loss {
                    self.decode_frame(emit);
                }
            }
            if self.packet_loss {
                self.num_saved_bits = 0;
                self.packet_loss = false;
            }
        } else {
            if first {
                gb.skip_bits(self.packet_offset);
            }
            let rest = remaining(gb);
            let frame_size = if rest > LOG2_FRAME_SIZE as isize {
                gb.show_bits(LOG2_FRAME_SIZE) as isize
            } else {
                0
            };
            if frame_size != 0 && frame_size <= rest {
                self.save_bits(gb, frame_size, false);
                if !self.packet_loss {
                    self.packet_done = !self.decode_frame(emit);
                }
            } else {
                self.packet_done = true;
            }
        }

        if remaining(gb) < 0 {
            self.packet_loss = true;
        }
        if self.packet_done && !self.packet_loss && remaining(gb) > 0 {
            let rest = remaining(gb);
            self.save_bits(gb, rest, false);
        }
        self.packet_offset = gb.position() & 7;
        !self.packet_loss
    }

    /// `save_bits`: copies `len` bits of the packet into the frame reservoir.
    fn save_bits(&mut self, gb: &mut BitReader<'_>, len: isize, append: bool) {
        let buflen = if append {
            (self.pb.count() as isize + len + 7) >> 3
        } else {
            self.frame_offset = gb.position() & 7;
            self.num_saved_bits = self.frame_offset;
            self.pb.reset();
            (self.num_saved_bits as isize + len + 7) >> 3
        };
        if len <= 0 || buflen > MAX_FRAMESIZE as isize {
            self.packet_loss = true;
            return;
        }
        let mut len = len as usize;
        self.num_saved_bits += len;
        let data = gb.data();
        if append {
            let align = (8 - (gb.position() & 7)).min(len);
            let bits = gb.get_bits(align as u32);
            self.pb.put_bits(align as u32, bits);
            len -= align;
            let start = (gb.position() >> 3).min(data.len());
            self.pb.copy_bits(&data[start..], len);
        } else {
            let start = (gb.position() >> 3).min(data.len());
            self.pb.copy_bits(&data[start..], self.num_saved_bits);
        }
        gb.skip_bits(len);
    }

    /// `decode_frame`: true when the trailer bit says more frames follow in the packet.
    fn decode_frame(&mut self, emit: &mut dyn FnMut(&[&[f32]])) -> bool {
        let buffer = std::mem::take(&mut self.pb.buffer);
        let result = {
            let mut gb = BitReader::new(&buffer, self.num_saved_bits);
            gb.skip_bits(self.frame_offset);
            self.decode_frame_bits(&mut gb, emit)
        };
        self.pb.buffer = buffer;
        result
    }

    fn decode_frame_bits(&mut self, gb: &mut BitReader<'_>, emit: &mut dyn FnMut(&[&[f32]])) -> bool {
        let len = gb.get_bits(LOG2_FRAME_SIZE) as isize;

        if self.decode_tilehdr(gb).is_err() {
            self.packet_loss = true;
            return false;
        }

        if self.nb_channels > 1 && gb.get_bits1() != 0 && gb.get_bits1() != 0 {
            for _ in 0..self.nb_channels * self.nb_channels {
                gb.skip_bits(4);
            }
        }

        self.drc_gain = gb.get_bits(8);

        if gb.get_bits1() != 0 {
            let bits = av_log2((SAMPLES_PER_FRAME * 2) as u32);
            if gb.get_bits1() != 0 {
                self.trim_start = gb.get_bits(bits);
            }
            if gb.get_bits1() != 0 {
                self.trim_end = gb.get_bits(bits);
            }
        } else {
            self.trim_start = 0;
            self.trim_end = 0;
        }

        self.parsed_all_subframes = false;
        for channel in &mut self.channel {
            channel.decoded_samples = 0;
            channel.cur_subframe = 0;
            channel.reuse_sf = false;
        }

        while !self.parsed_all_subframes {
            if self.decode_subframe(gb).is_err() {
                self.packet_loss = true;
                return false;
            }
        }

        {
            let slices: Vec<&[f32]> = self
                .channel
                .iter()
                .map(|channel| &channel.out[..SAMPLES_PER_FRAME])
                .collect();
            emit(&slices);
        }

        for channel in &mut self.channel {
            channel
                .out
                .copy_within(SAMPLES_PER_FRAME..SAMPLES_PER_FRAME + SAMPLES_PER_FRAME / 2, 0);
        }

        let consumed = gb.position() as isize - self.frame_offset as isize;
        if len != consumed + 2 {
            self.packet_loss = true;
            return false;
        }
        gb.skip_bits((len - consumed - 1) as usize);

        gb.get_bits1() != 0
    }

    /// `decode_tilehdr`.
    fn decode_tilehdr(&mut self, gb: &mut BitReader<'_>) -> Result<(), ()> {
        let mut num_samples = [0u16; MAX_CHANNELS];
        let mut contains_subframe = [false; MAX_CHANNELS];
        let mut channels_for_cur_subframe = self.nb_channels;
        let mut min_channel_len = 0usize;

        for channel in &mut self.channel {
            channel.num_subframes = 0;
        }

        let fixed_channel_layout = self.max_num_subframes == 1 || gb.get_bits1() != 0;

        loop {
            for c in 0..self.nb_channels {
                contains_subframe[c] = if usize::from(num_samples[c]) == min_channel_len {
                    if fixed_channel_layout
                        || channels_for_cur_subframe == 1
                        || min_channel_len == SAMPLES_PER_FRAME - self.min_samples_per_subframe
                    {
                        true
                    } else {
                        gb.get_bits1() != 0
                    }
                } else {
                    false
                };
            }

            let subframe_len = self.decode_subframe_length(gb, min_channel_len)?;

            min_channel_len += subframe_len;
            for c in 0..self.nb_channels {
                let chan = &mut self.channel[c];
                if contains_subframe[c] {
                    if chan.num_subframes >= MAX_SUBFRAMES {
                        return Err(());
                    }
                    chan.subframe_len[chan.num_subframes] = subframe_len as u16;
                    num_samples[c] += subframe_len as u16;
                    chan.num_subframes += 1;
                    if usize::from(num_samples[c]) > SAMPLES_PER_FRAME {
                        return Err(());
                    }
                } else if usize::from(num_samples[c]) <= min_channel_len {
                    if usize::from(num_samples[c]) < min_channel_len {
                        channels_for_cur_subframe = 0;
                        min_channel_len = usize::from(num_samples[c]);
                    }
                    channels_for_cur_subframe += 1;
                }
            }
            if min_channel_len >= SAMPLES_PER_FRAME {
                break;
            }
        }

        for channel in &mut self.channel {
            let mut offset = 0u16;
            for i in 0..channel.num_subframes {
                channel.subframe_offset[i] = offset;
                offset += channel.subframe_len[i];
            }
        }
        Ok(())
    }

    /// `decode_subframe_length`.
    fn decode_subframe_length(&self, gb: &mut BitReader<'_>, offset: usize) -> Result<usize, ()> {
        if offset == SAMPLES_PER_FRAME - self.min_samples_per_subframe {
            return Ok(self.min_samples_per_subframe);
        }
        if gb.bits_left() < 1 {
            return Err(());
        }
        let mut frame_len_shift = 0u32;
        if self.max_subframe_len_bit {
            if gb.get_bits1() != 0 {
                frame_len_shift = 1 + gb.get_bits(self.subframe_len_bits - 1);
            }
        } else {
            frame_len_shift = gb.get_bits(self.subframe_len_bits);
        }
        let subframe_len = SAMPLES_PER_FRAME >> frame_len_shift;
        if subframe_len < self.min_samples_per_subframe || subframe_len > SAMPLES_PER_FRAME {
            return Err(());
        }
        Ok(subframe_len)
    }

    /// `decode_channel_transform` (one or two channels).
    fn decode_channel_transform(&mut self, gb: &mut BitReader<'_>) -> Result<(), ()> {
        self.num_chgroups = 0;
        if self.nb_channels > 1 {
            let mut remaining_channels = self.channels_for_cur_subframe;
            if gb.get_bits1() != 0 {
                return Err(());
            }
            while remaining_channels != 0 && self.num_chgroups < self.channels_for_cur_subframe {
                let mut group = ChannelGroup {
                    transform_band: self.chgroup[self.num_chgroups].transform_band,
                    decorrelation_matrix: self.chgroup[self.num_chgroups].decorrelation_matrix,
                    ..ChannelGroup::default()
                };
                let mut data_count = 0usize;
                group.num_channels = remaining_channels;
                for i in 0..self.channels_for_cur_subframe {
                    let channel_idx = self.channel_indexes_for_cur_subframe[i];
                    if !self.channel[channel_idx].grouped {
                        group.channel_data[data_count] = channel_idx;
                        data_count += 1;
                    }
                    self.channel[channel_idx].grouped = true;
                }

                if group.num_channels == 2 {
                    if gb.get_bits1() != 0 {
                        if gb.get_bits1() != 0 {
                            return Err(());
                        }
                    } else {
                        group.transform = true;
                        group.decorrelation_matrix = [1.0, -1.0, 1.0, 1.0];
                    }
                }

                if group.transform {
                    if gb.get_bits1() == 0 {
                        for band in &mut group.transform_band[..self.num_bands] {
                            *band = gb.get_bits1() as u8;
                        }
                    } else {
                        group.transform_band[..self.num_bands].fill(1);
                    }
                }
                remaining_channels -= group.num_channels;
                self.chgroup[self.num_chgroups] = group;
                self.num_chgroups += 1;
            }
        }
        Ok(())
    }

    /// `decode_coeffs` with `ff_wma_run_level_decode`.
    fn decode_coeffs(&mut self, gb: &mut BitReader<'_>, c: usize) {
        const FVAL_TAB: [u32; 16] = [
            0x0000_0000, 0x3f80_0000, 0x4000_0000, 0x4040_0000, 0x4080_0000, 0x40a0_0000,
            0x40c0_0000, 0x40e0_0000, 0x4100_0000, 0x4110_0000, 0x4120_0000, 0x4130_0000,
            0x4140_0000, 0x4150_0000, 0x4160_0000, 0x4170_0000,
        ];
        let codes = codes();
        let vlctable = gb.get_bits1() as usize;
        let vlc = &codes.coef[vlctable];
        let (run, level): (&[u16], &[f32]) = if vlctable != 0 {
            (&COEF1_RUN, &COEF1_LEVEL)
        } else {
            (&COEF0_RUN, &COEF0_LEVEL)
        };
        let subframe_len = self.subframe_len;
        let transmit_num_vec_coeffs = self.transmit_num_vec_coeffs;
        let ci = &mut self.channel[c];
        let base = ci.coeffs;
        let num_vec_coeffs = usize::from(ci.num_vec_coeffs);
        let mut rl_mode = false;
        let mut cur_coeff = 0usize;
        let mut num_zeros = 0usize;

        while (transmit_num_vec_coeffs || !rl_mode) && cur_coeff + 3 < num_vec_coeffs {
            let mut vals = [0u32; 4];
            let idx = codes.vec4.decode(gb);
            if idx < 0 {
                for i in (0..4).step_by(2) {
                    let idx = codes.vec2.decode(gb);
                    if idx < 0 {
                        let mut v0 = codes.vec1.decode(gb) as u32;
                        if v0 as i32 == HUFF_VEC1_SIZE - 1 {
                            v0 = v0.wrapping_add(get_large_val(gb));
                        }
                        let mut v1 = codes.vec1.decode(gb) as u32;
                        if v1 as i32 == HUFF_VEC1_SIZE - 1 {
                            v1 = v1.wrapping_add(get_large_val(gb));
                        }
                        vals[i] = (v0 as f32).to_bits();
                        vals[i + 1] = (v1 as f32).to_bits();
                    } else {
                        vals[i] = FVAL_TAB[(idx >> 4) as usize & 0xF];
                        vals[i + 1] = FVAL_TAB[idx as usize & 0xF];
                    }
                }
            } else {
                vals[0] = FVAL_TAB[(idx >> 12) as usize & 0xF];
                vals[1] = FVAL_TAB[(idx >> 8) as usize & 0xF];
                vals[2] = FVAL_TAB[(idx >> 4) as usize & 0xF];
                vals[3] = FVAL_TAB[idx as usize & 0xF];
            }

            for value in vals {
                if value != 0 {
                    let sign = gb.get_bits1().wrapping_sub(1);
                    ci.out[base + cur_coeff] = f32::from_bits(value ^ (sign << 31));
                    num_zeros = 0;
                } else {
                    ci.out[base + cur_coeff] = 0.0;
                    num_zeros += 1;
                    rl_mode |= num_zeros > subframe_len >> 8;
                }
                cur_coeff += 1;
            }
        }

        if cur_coeff < subframe_len {
            ci.out[base + cur_coeff..base + subframe_len].fill(0.0);
            let coef_mask = subframe_len - 1;
            let mut offset = cur_coeff;
            while offset < subframe_len {
                let code = vlc.decode(gb);
                if code > 1 {
                    offset += usize::from(run[code as usize]);
                    let sign = gb.get_bits1().wrapping_sub(1);
                    ci.out[base + (offset & coef_mask)] =
                        f32::from_bits(level[code as usize].to_bits() ^ (sign & 0x8000_0000));
                } else if code == 1 {
                    break;
                } else {
                    let level = get_large_val(gb) as i32;
                    if gb.get_bits1() != 0 {
                        if gb.get_bits1() != 0 {
                            if gb.get_bits1() != 0 {
                                return;
                            }
                            offset += gb.get_bits(self.esc_len) as usize + 4;
                        } else {
                            offset += gb.get_bits(2) as usize + 1;
                        }
                    }
                    let sign = gb.get_bits1() as i32 - 1;
                    ci.out[base + (offset & coef_mask)] = ((level ^ sign) - sign) as f32;
                }
                offset += 1;
            }
        }
    }

    /// `decode_scale_factors`.
    fn decode_scale_factors(&mut self, gb: &mut BitReader<'_>) -> Result<(), ()> {
        let codes = codes();
        for i in 0..self.channels_for_cur_subframe {
            let c = self.channel_indexes_for_cur_subframe[i];
            let num_bands = self.num_bands;
            let table_idx = self.table_idx;
            let sf_offsets = self.sf_offsets[table_idx][self.channel[c].table_idx];
            let chan = &mut self.channel[c];
            chan.scale_factors = usize::from(chan.scale_factor_idx == 0);
            let sf = chan.scale_factors;

            if chan.reuse_sf {
                for b in 0..num_bands {
                    chan.saved_scale_factors[sf][b] =
                        chan.saved_scale_factors[chan.scale_factor_idx][sf_offsets[b] as usize];
                }
            }

            if chan.cur_subframe == 0 || gb.get_bits1() != 0 {
                if chan.reuse_sf {
                    let mut b = 0usize;
                    while b < num_bands {
                        let idx = codes.sf_rl.decode(gb);
                        let (skip, val, sign);
                        if idx == 0 {
                            let code = gb.get_bits(14);
                            val = (code >> 6) as i32;
                            sign = (code & 1) as i32 - 1;
                            skip = ((code & 0x3f) >> 1) as usize;
                        } else if idx == 1 {
                            break;
                        } else {
                            skip = usize::from(SCALE_RL_RUN[idx as usize]);
                            val = i32::from(SCALE_RL_LEVEL[idx as usize]);
                            sign = gb.get_bits1() as i32 - 1;
                        }
                        b += skip;
                        if b >= num_bands {
                            return Err(());
                        }
                        chan.saved_scale_factors[sf][b] += (val ^ sign) - sign;
                        b += 1;
                    }
                } else {
                    chan.scale_factor_step = gb.get_bits(2) as i32 + 1;
                    let mut val = 45 / chan.scale_factor_step;
                    for b in 0..num_bands {
                        val += codes.sf.decode(gb);
                        chan.saved_scale_factors[sf][b] = val;
                    }
                }
                chan.scale_factor_idx = usize::from(chan.scale_factor_idx == 0);
                chan.table_idx = table_idx;
                chan.reuse_sf = true;
            }

            chan.max_scale_factor = chan.saved_scale_factors[sf][0];
            for b in 1..num_bands {
                chan.max_scale_factor = chan.max_scale_factor.max(chan.saved_scale_factors[sf][b]);
            }
        }
        Ok(())
    }

    /// `inverse_channel_transform`.
    fn inverse_channel_transform(&mut self) {
        let sfb = self.sfb_offsets[self.table_idx];
        for g in 0..self.num_chgroups {
            let group = self.chgroup[g];
            if !group.transform {
                continue;
            }
            let num_channels = group.num_channels;
            for b in 0..self.num_bands {
                let start = sfb[b] as usize;
                let end = (sfb[b + 1] as usize).min(self.subframe_len);
                if group.transform_band[b] == 1 {
                    for y in start..end {
                        let mut data = [0.0f32; MAX_CHANNELS];
                        for (k, &ch) in group.channel_data[..num_channels].iter().enumerate() {
                            data[k] = self.channel[ch].out[self.channel[ch].coeffs + y];
                        }
                        let mut mat = 0usize;
                        for &ch in &group.channel_data[..num_channels] {
                            let mut sum = 0.0f32;
                            for value in &data[..num_channels] {
                                sum += *value * group.decorrelation_matrix[mat];
                                mat += 1;
                            }
                            let coeffs = self.channel[ch].coeffs;
                            self.channel[ch].out[coeffs + y] = sum;
                        }
                    }
                } else if self.nb_channels == 2 {
                    for &ch in &group.channel_data[..2] {
                        let coeffs = self.channel[ch].coeffs;
                        for value in &mut self.channel[ch].out[coeffs + start..coeffs + end] {
                            *value *= 181.0 / 128.0;
                        }
                    }
                }
            }
        }
    }

    /// `wmapro_window`: `vector_fmul_window` over the previous block's tail.
    fn wmapro_window(&mut self) {
        let codes = codes();
        for i in 0..self.channels_for_cur_subframe {
            let c = self.channel_indexes_for_cur_subframe[i];
            let chan = &mut self.channel[c];
            let mut winlen = chan.prev_block_len as usize;
            let mut start = chan.coeffs - (winlen >> 1);
            if self.subframe_len < winlen {
                start += (winlen - self.subframe_len) >> 1;
                winlen = self.subframe_len;
            }
            let window = &codes.windows[(av_log2(winlen as u32) - BLOCK_MIN_BITS) as usize];
            let len = winlen >> 1;
            let dst = start + len;
            for (step, j) in (0..len).zip((0..len).rev()) {
                let i = step as isize - len as isize;
                let s0 = chan.out[(dst as isize + i) as usize];
                let s1 = chan.out[dst + j];
                let wi = window[(len as isize + i) as usize];
                let wj = window[len + j];
                chan.out[(dst as isize + i) as usize] = s0 * wj - s1 * wi;
                chan.out[dst + j] = s0 * wi + s1 * wj;
            }
            chan.prev_block_len = self.subframe_len as i32;
        }
    }

    /// `decode_subframe`.
    fn decode_subframe(&mut self, gb: &mut BitReader<'_>) -> Result<(), ()> {
        let mut offset = SAMPLES_PER_FRAME;
        let mut subframe_len = SAMPLES_PER_FRAME;
        let mut total_samples = (SAMPLES_PER_FRAME * self.nb_channels) as isize;
        let mut transmit_coeffs = false;

        for chan in &mut self.channel {
            chan.grouped = false;
            if offset > usize::from(chan.decoded_samples) {
                offset = usize::from(chan.decoded_samples);
                subframe_len = usize::from(chan.subframe_len[chan.cur_subframe]);
            }
        }

        self.channels_for_cur_subframe = 0;
        for (i, chan) in self.channel.iter_mut().enumerate() {
            let cur_subframe = chan.cur_subframe;
            total_samples -= chan.decoded_samples as isize;
            if offset == usize::from(chan.decoded_samples)
                && subframe_len == usize::from(chan.subframe_len[cur_subframe])
            {
                total_samples -= chan.subframe_len[cur_subframe] as isize;
                chan.decoded_samples += chan.subframe_len[cur_subframe];
                self.channel_indexes_for_cur_subframe[self.channels_for_cur_subframe] = i;
                self.channels_for_cur_subframe += 1;
            }
        }

        if total_samples == 0 {
            self.parsed_all_subframes = true;
        }

        self.table_idx = av_log2((SAMPLES_PER_FRAME / subframe_len) as u32) as usize;
        self.num_bands = self.num_sfb[self.table_idx];

        offset += SAMPLES_PER_FRAME >> 1;
        for i in 0..self.channels_for_cur_subframe {
            let c = self.channel_indexes_for_cur_subframe[i];
            self.channel[c].coeffs = offset;
        }

        self.subframe_len = subframe_len;
        self.esc_len = av_log2(subframe_len as u32 - 1) + 1;

        if gb.get_bits1() != 0 {
            let mut num_fill_bits = gb.get_bits(2) as usize;
            if num_fill_bits == 0 {
                let len = gb.get_bits(4);
                num_fill_bits = gb.get_bitsz(len) as usize + 1;
            }
            if gb.position() + num_fill_bits > self.num_saved_bits {
                return Err(());
            }
            gb.skip_bits(num_fill_bits);
        }

        if gb.get_bits1() != 0 {
            return Err(());
        }

        self.decode_channel_transform(gb)?;

        for i in 0..self.channels_for_cur_subframe {
            let c = self.channel_indexes_for_cur_subframe[i];
            self.channel[c].transmit_coefs = gb.get_bits1() != 0;
            if self.channel[c].transmit_coefs {
                transmit_coeffs = true;
            }
        }

        if transmit_coeffs {
            let mut quant_step = (90 * BITS_PER_SAMPLE) >> 4;
            self.transmit_num_vec_coeffs = gb.get_bits1() != 0;
            if self.transmit_num_vec_coeffs {
                let num_bits = av_log2(self.subframe_len.div_ceil(4) as u32) + 1;
                for i in 0..self.channels_for_cur_subframe {
                    let c = self.channel_indexes_for_cur_subframe[i];
                    let num_vec_coeffs = (gb.get_bits(num_bits) << 2) as usize;
                    if num_vec_coeffs > self.subframe_len {
                        return Err(());
                    }
                    self.channel[c].num_vec_coeffs = num_vec_coeffs as u16;
                }
            } else {
                for i in 0..self.channels_for_cur_subframe {
                    let c = self.channel_indexes_for_cur_subframe[i];
                    self.channel[c].num_vec_coeffs = self.subframe_len as u16;
                }
            }
            let mut step = gb.get_sbits(6);
            quant_step += step;
            if step == -32 || step == 31 {
                let sign = i32::from(step == 31) - 1;
                let mut quant = 0;
                while gb.position() + 5 < self.num_saved_bits {
                    step = gb.get_bits(5) as i32;
                    if step != 31 {
                        break;
                    }
                    quant += 31;
                }
                quant_step += ((quant + step) ^ sign) - sign;
            }

            if self.channels_for_cur_subframe == 1 {
                let c = self.channel_indexes_for_cur_subframe[0];
                self.channel[c].quant_step = quant_step;
            } else {
                let modifier_len = gb.get_bits(3);
                for i in 0..self.channels_for_cur_subframe {
                    let c = self.channel_indexes_for_cur_subframe[i];
                    self.channel[c].quant_step = quant_step;
                    if gb.get_bits1() != 0 {
                        if modifier_len != 0 {
                            self.channel[c].quant_step += gb.get_bits(modifier_len) as i32 + 1;
                        } else {
                            self.channel[c].quant_step += 1;
                        }
                    }
                }
            }

            self.decode_scale_factors(gb)?;
        }

        for i in 0..self.channels_for_cur_subframe {
            let c = self.channel_indexes_for_cur_subframe[i];
            if self.channel[c].transmit_coefs && gb.position() < self.num_saved_bits {
                self.decode_coeffs(gb, c);
            } else {
                let coeffs = self.channel[c].coeffs;
                self.channel[c].out[coeffs..coeffs + subframe_len].fill(0.0);
            }
        }

        if transmit_coeffs {
            let codes = codes();
            let imdct = &codes.imdct[(av_log2(subframe_len as u32) - BLOCK_MIN_BITS) as usize];
            self.inverse_channel_transform();
            let sfb = self.sfb_offsets[self.table_idx];
            for i in 0..self.channels_for_cur_subframe {
                let c = self.channel_indexes_for_cur_subframe[i];
                let chan = &mut self.channel[c];
                let sf = chan.saved_scale_factors[chan.scale_factors];
                for b in 0..self.num_bands {
                    let end = (sfb[b + 1] as usize).min(self.subframe_len);
                    let exp = chan.quant_step - (chan.max_scale_factor - sf[b]) * chan.scale_factor_step;
                    let quant = 10.0f64.powf(f64::from(exp) / 20.0) as f32;
                    let start = sfb[b] as usize;
                    for k in start..end {
                        self.tmp[k] = chan.out[chan.coeffs + k] * quant;
                    }
                }
                let coeffs = chan.coeffs;
                imdct.half(&mut chan.out[coeffs..coeffs + subframe_len], &self.tmp[..subframe_len]);
            }
        }

        self.wmapro_window();

        for i in 0..self.channels_for_cur_subframe {
            let c = self.channel_indexes_for_cur_subframe[i];
            if self.channel[c].cur_subframe >= self.channel[c].num_subframes {
                return Err(());
            }
            self.channel[c].cur_subframe += 1;
        }
        Ok(())
    }
}
