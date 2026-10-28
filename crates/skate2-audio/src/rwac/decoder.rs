//! `rw::audio::core::Decoder` (feed slots) and `EaXmaDec`, the EA-XMA codec ('EXm0').
//!
//! On the 360 each stereo pair of channels owns an XMA hardware context. `Service` copies
//! 2048-byte packets into the context's two input buffers, the hardware decodes them into a
//! 6144-byte ring of 16-bit samples, and `DecodeEvent` reads the ring. Here the hardware is a
//! [`skate2_xma::XmaStream`] that runs whenever the code looks at it, as if it were infinitely
//! fast: it fills the ring as far as input and ring space allow, in whole 128-sample subframes.
//!
//! The hardware writes each sample 192 samples earlier than the WMA Pro decoder does: with the
//! game's 384-sample skip, every bank sample's audio starts at the window's first sample and
//! ends at its last only on the hardware's timing. A fresh context therefore drops the
//! decoder's first 192 samples, and once a stream's data is exhausted the decoder's final
//! overlap is drained for the samples the hardware had already written.

use skate2_xma::{XmaStream, PACKET_BYTES};
use std::collections::VecDeque;
use std::sync::Arc;

/// Shared bytes plus an offset: the `void*` into a bank or stream the native code passes around.
#[derive(Clone)]
pub struct DataRef {
    pub bytes: Arc<[u8]>,
    pub offset: usize,
}

impl DataRef {
    pub fn at(&self, offset: usize) -> DataRef {
        DataRef { bytes: Arc::clone(&self.bytes), offset: self.offset + offset }
    }

    pub fn slice(&self) -> &[u8] {
        self.bytes.get(self.offset..).unwrap_or(&[])
    }

    pub fn u32_be(&self, at: usize) -> u32 {
        let s = self.slice();
        s.get(at..at + 4).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// `EaXmaDec::mNumContexts`: the XMA hardware contexts shared by every decoder.
pub const XMA_CONTEXTS: u32 = 256;

/// Output ring of a context: `XMAInitializeContext` with 0x18 blocks of 256 bytes.
const RING_BYTES: usize = 0x1800;
/// The hardware writes the ring in subframes of 128 samples.
const SUBFRAME: usize = 128;
/// The read offset the decoder hands back is rounded down to whole 512-sample frames.
const FRAME: usize = 512;
/// How many samples the hardware's output leads the WMA Pro decoder's.
const HARDWARE_LEAD: usize = 192;

/// `mXmaContextFreeList`.
#[derive(Debug)]
pub struct XmaContextPool {
    free: u32,
}

impl Default for XmaContextPool {
    fn default() -> Self {
        Self { free: XMA_CONTEXTS }
    }
}

/// `Decoder::RequestDesc`.
#[derive(Clone, Default)]
struct RequestDesc {
    src: Option<DataRef>,
    seek_data: Option<DataRef>,
    decoder_skip: i32,
    num_samples: i32,
    feed_type: u8,
}

/// A valid input buffer: a packet and the read offset `XMASetInputBufferReadOffset` gave it.
struct InputBuffer {
    packet: Box<[u8; PACKET_BYTES]>,
    read_offset_bits: Option<usize>,
}

/// `SndXmaContext` with the state of the hardware context behind it.
struct Context {
    channels: usize,
    stream: Option<XmaStream>,
    /// Valid input buffers in the order the hardware reads them (at most 2).
    inputs: VecDeque<InputBuffer>,
    /// Samples decoded from consumed input but not yet written to the ring (interleaved).
    pending: VecDeque<i16>,
    ring: Vec<i16>,
    /// `XMAGetOutputBufferWriteOffset`, in sample frames.
    write: usize,
    /// The output read offset last given to the hardware, in sample frames.
    hardware_read: usize,
    /// `XMAIsOutputBufferValid`: false once the hardware has filled the ring.
    output_valid: bool,
    /// Decoder samples (per channel) still to drop to reach the hardware's timing.
    lead_to_drop: usize,
    /// Whether the decoder's final overlap was drained.
    drained: bool,
}

impl Context {
    fn new(channels: usize) -> Self {
        Self {
            channels,
            stream: None,
            inputs: VecDeque::new(),
            pending: VecDeque::new(),
            ring: vec![0; RING_BYTES / 2],
            write: 0,
            hardware_read: 0,
            output_valid: true,
            lead_to_drop: HARDWARE_LEAD,
            drained: false,
        }
    }

    /// Ring capacity in sample frames.
    fn capacity(&self) -> usize {
        RING_BYTES / 2 / self.channels
    }

    /// `XMADisableContext` + `XMAInitializeContext` + `XMASetOutputBufferValid`.
    fn initialize(&mut self, decoder_table: u32) {
        self.stream = XmaStream::new(self.channels, decoder_table).ok();
        self.inputs.clear();
        self.pending.clear();
        self.write = 0;
        self.hardware_read = 0;
        self.output_valid = true;
        self.lead_to_drop = HARDWARE_LEAD;
        self.drained = false;
    }

    /// The samples the hardware wrote from the stream's last frame: the decoder's overlap,
    /// once all input is consumed. False when already drained.
    fn drain(&mut self) -> bool {
        let Some(stream) = self.stream.as_mut() else {
            return false;
        };
        if self.drained {
            return false;
        }
        self.drained = true;
        let channels = self.channels;
        let pending = &mut self.pending;
        stream.drain(&mut |frame: &[&[f32]]| {
            for i in 0..frame[0].len() {
                for channel in frame.iter().take(channels) {
                    pending.push_back(to_s16(channel[i]));
                }
            }
        });
        true
    }

    /// The hardware catching up: decode input and write subframes while the ring has room.
    fn run(&mut self) {
        let capacity = self.capacity();
        let subframe = SUBFRAME * self.channels;
        while self.output_valid {
            if self.lead_to_drop != 0 && !self.pending.is_empty() {
                let dropped = (self.lead_to_drop * self.channels).min(self.pending.len());
                self.pending.drain(..dropped);
                self.lead_to_drop -= dropped / self.channels;
                continue;
            }
            if self.pending.len() < subframe {
                let Some(input) = self.inputs.pop_front() else {
                    break;
                };
                let Some(stream) = self.stream.as_mut() else {
                    break;
                };
                let channels = self.channels;
                let pending = &mut self.pending;
                let mut emit = |frame: &[&[f32]]| {
                    for i in 0..frame[0].len() {
                        for channel in frame.iter().take(channels) {
                            pending.push_back(to_s16(channel[i]));
                        }
                    }
                };
                match input.read_offset_bits {
                    Some(bits) => stream.decode_packet_at(&input.packet[..], bits, &mut emit),
                    None => stream.decode_packet(&input.packet[..], &mut emit),
                }
                continue;
            }
            let at = self.write * self.channels;
            for (slot, sample) in self.ring[at..at + subframe].iter_mut().zip(self.pending.drain(..subframe)) {
                *slot = sample;
            }
            self.write = (self.write + SUBFRAME) % capacity;
            if self.write == self.hardware_read {
                self.output_valid = false;
            }
        }
    }
}

/// Sample frames between a read offset and the hardware's write offset.
fn ring_available(context: &Context, output_offset: usize, uninitialised: bool) -> isize {
    let mut available = context.write as isize - output_offset as isize;
    if available <= 0 && !uninitialised && (available != 0 || !context.output_valid) {
        available += context.capacity() as isize;
    }
    available
}

/// The hardware's 16-bit output: nearest, saturated.
fn to_s16(sample: f32) -> i16 {
    let scaled = (sample * 32768.0).round_ties_even();
    scaled.clamp(-32768.0, 32767.0) as i16
}

/// `EaXmaDec::StereoPair`.
struct Pair {
    context: Context,
    /// `pFeedData`.
    feed: Option<DataRef>,
    bytes_remaining: i32,
    /// `outputBufferByteOffset`, in sample frames.
    output_offset: usize,
    num_samples_to_skip: i32,
    seek_frame_offset_bits: i32,
}

/// `EaXmaDec::ParseToNextFrame` 0x82c0e8a0: steps (packet, frame bit offset) over one frame by
/// its 15-bit length, which may continue in the next packet's data.
fn parse_to_next_frame(data: &[u8], packet: &mut usize, frame_offset: &mut i32) {
    let bit = |position: i64| -> u32 {
        usize::try_from(position)
            .ok()
            .and_then(|p| data.get(p >> 3).map(|b| u32::from(b >> (7 - (p & 7)) & 1)))
            .unwrap_or(0)
    };
    let read = |start: i64, count: i32| -> u32 { (0..i64::from(count)).fold(0, |v, i| v << 1 | bit(start + i)) };
    let offset = *frame_offset;
    let over = if offset > 0x3FD1 { offset - 0x3FD1 } else { 0 };
    let here = 15 - over;
    let base = (*packet * 8) as i64;
    let mut length = if here > 0 { read(base + i64::from(offset) + 32, here) } else { 0 };
    if over > 0 {
        length = (length << over) | read(base + i64::from(offset + here + 64), over);
    }
    let next = offset.wrapping_add(length as i32);
    if (next as u32) < 0x3FE0 {
        *frame_offset = next;
    } else {
        *packet += PACKET_BYTES;
        *frame_offset = next.wrapping_sub(0x3FE0);
    }
}

/// `rw::audio::core::Decoder` with the `EaXmaDec` instance data.
pub struct Decoder {
    pub num_channels: u8,
    slots: Vec<RequestDesc>,
    feed_slot: usize,
    prepare_slot: usize,
    decode_slot: usize,
    decode_slot_samples_decoded: i32,
    pairs: Vec<Pair>,
    current_request: usize,
    total_bytes_remaining: i32,
    samples_left_this_feed_desc: i32,
    num_padding_samples: i32,
    current_decoder_table: u32,
    failed: bool,
    uninitialised: bool,
}

impl Decoder {
    /// `DecoderRegistry::DecoderFactory` 0x82be3ee8 with `EaXmaDec::CreateInstanceEvent`
    /// 0x82c13f88: one context per pair from the pool, all or nothing.
    pub fn new(pool: &mut XmaContextPool, num_channels: u8, max_slots: usize) -> Option<Self> {
        let num_pairs = u32::from(num_channels).div_ceil(2);
        if pool.free < num_pairs {
            return None;
        }
        pool.free -= num_pairs;
        let mut remaining = num_channels;
        let pairs = (0..num_pairs)
            .map(|_| {
                let channels = if remaining > 1 {
                    remaining -= 2;
                    2
                } else {
                    1
                };
                Pair {
                    context: Context::new(channels),
                    feed: None,
                    bytes_remaining: 0,
                    output_offset: 0,
                    num_samples_to_skip: 0,
                    seek_frame_offset_bits: 0,
                }
            })
            .collect();
        Some(Self {
            num_channels,
            slots: vec![RequestDesc::default(); max_slots],
            feed_slot: 0,
            prepare_slot: 0,
            decode_slot: 0,
            decode_slot_samples_decoded: 0,
            pairs,
            current_request: 0,
            total_bytes_remaining: 0,
            samples_left_this_feed_desc: 0,
            num_padding_samples: 0,
            current_decoder_table: 0,
            failed: false,
            uninitialised: true,
        })
    }

    /// `Decoder::Release` 0x82be3dd0 / `EaXmaDec::ReleaseEvent` 0x82c0e548.
    pub fn release(self, pool: &mut XmaContextPool) {
        pool.free += self.pairs.len() as u32;
    }

    /// `Decoder::Feed` 0x82bdcb68: queues a chunk and returns its slot. A full queue returns 0,
    /// which is also the first slot's handle. The request also records the seek data version and
    /// byte count, which `EaXmaDec` never reads.
    #[allow(clippy::too_many_arguments)]
    pub fn feed(
        &mut self,
        src: DataRef,
        num_samples: i32,
        feed_type: u8,
        decoder_skip: i32,
        seek_data: Option<DataRef>,
        _seek_data_version: u8,
        _num_bytes: i32,
    ) -> u8 {
        let slot = self.feed_slot;
        if self.slots[slot].num_samples != 0 {
            return 0;
        }
        self.slots[slot] = RequestDesc {
            src: Some(src),
            seek_data,
            decoder_skip,
            num_samples,
            feed_type,
        };
        // `EaXmaDec::FeedEvent` 0x82c16478.
        self.service();
        if self.feed_slot == self.decode_slot {
            self.decode_slot_samples_decoded = decoder_skip;
        }
        self.feed_slot += 1;
        if self.feed_slot >= self.slots.len() {
            self.feed_slot = 0;
        }
        slot as u8
    }

    /// `Decoder::GetSamplesRemaining` 0x82bf61a0.
    pub fn samples_remaining(&self, slot: u8) -> i32 {
        let slot = usize::from(slot);
        let desc = &self.slots[slot];
        if desc.num_samples == 0 {
            0
        } else if slot == self.decode_slot {
            desc.num_samples - self.decode_slot_samples_decoded
        } else {
            desc.num_samples - desc.decoder_skip
        }
    }

    /// `Decoder::Decode` 0x82bdccb0, the path for decoders that are not block based. Each slot's
    /// samples are written from index 0 of `out`, so callers stay within one slot per call.
    pub fn decode(&mut self, out: &mut [&mut [f32]], count: i32) -> i32 {
        let mut produced = 0;
        if count <= 0 {
            return produced;
        }
        while self.slots[self.decode_slot].num_samples != 0 {
            let n = (count - produced).min(self.slots[self.decode_slot].num_samples - self.decode_slot_samples_decoded);
            self.decode_event(out, n);
            produced += n;
            self.decode_slot_samples_decoded += n;
            if self.decode_slot_samples_decoded == self.slots[self.decode_slot].num_samples {
                self.slots[self.decode_slot].num_samples = 0;
                self.decode_slot += 1;
                if self.decode_slot >= self.slots.len() {
                    self.decode_slot = 0;
                }
                self.decode_slot_samples_decoded = self.slots[self.decode_slot].decoder_skip;
            }
            if produced >= count {
                break;
            }
        }
        produced
    }

    /// `EaXmaDec::Reset` 0x82c0e650.
    fn reset(&mut self, decoder_table: u32) {
        self.current_decoder_table = decoder_table;
        for pair in &mut self.pairs {
            pair.context.initialize(decoder_table);
        }
    }

    /// `EaXmaDec::StartPair` 0x82c14200: positions a pair at the frame its part of the request
    /// starts with; returns the size of the pair's block.
    fn start_pair(pair: &mut Pair, pair_index: usize, desc: &RequestDesc, block: &DataRef) -> i32 {
        let size = (block.u32_be(0) >> 2) as i32;
        let mut frames = desc.decoder_skip / 512;
        if desc.feed_type == 0 {
            frames = (desc.decoder_skip + 0x180) / 512;
        }
        let packets = block.at(4);
        let data = packets.slice();
        let header = |packet: usize| -> i32 {
            let word = data.get(packet..packet + 4).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
            let offset = ((word >> 11) & 0x7FFF) as i32;
            if offset >= 0x3FE0 {
                -1
            } else {
                offset
            }
        };
        let mut packet = 0usize;
        let mut frame_offset = header(packet);
        if let Some(seek) = &desc.seek_data {
            let table = seek.slice();
            let entry = table
                .get(pair_index * 4..pair_index * 4 + 4)
                .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]])) as usize;
            let mut cursor = entry;
            let unpack = |cursor: &mut usize| {
                let (value, used) = super::seek::unpack_uint32(table.get(*cursor..).unwrap_or(&[]));
                *cursor += used;
                value as i32
            };
            let mut skipped = 0;
            let mut per_packet = unpack(&mut cursor);
            if frames >= per_packet {
                loop {
                    packet += PACKET_BYTES;
                    skipped += per_packet;
                    per_packet = unpack(&mut cursor);
                    if frames < per_packet + skipped {
                        break;
                    }
                }
            }
            frames -= skipped;
            frame_offset = header(packet);
        }
        for _ in 0..frames.max(0) {
            parse_to_next_frame(data, &mut packet, &mut frame_offset);
        }
        pair.feed = Some(packets.at(packet));
        pair.seek_frame_offset_bits = frame_offset;
        pair.bytes_remaining = size - 4 - packet as i32;
        size
    }

    /// `EaXmaDec::Service` 0x82c161b0: refills the input buffers; once every packet of the
    /// prepared request is in, prepares the next fed request.
    fn service(&mut self) {
        loop {
            for pair in &mut self.pairs {
                pair.context.run();
                while pair.bytes_remaining != 0 && pair.context.inputs.len() < 2 {
                    let Some(feed) = pair.feed.clone() else {
                        break;
                    };
                    let bytes = (pair.bytes_remaining as u32).min(PACKET_BYTES as u32) as usize;
                    let mut packet = Box::new([0u8; PACKET_BYTES]);
                    let src = feed.slice();
                    let copy = bytes.min(src.len());
                    packet[..copy].copy_from_slice(&src[..copy]);
                    let read_offset_bits = (pair.seek_frame_offset_bits > 0).then(|| {
                        let bits = pair.seek_frame_offset_bits as usize;
                        pair.seek_frame_offset_bits = 0;
                        bits
                    });
                    pair.context.inputs.push_back(InputBuffer { packet, read_offset_bits });
                    pair.bytes_remaining -= bytes as i32;
                    self.total_bytes_remaining -= bytes as i32;
                    pair.feed = Some(feed.at(bytes));
                }
            }
            if self.total_bytes_remaining != 0 {
                return;
            }
            let desc = self.slots[self.prepare_slot].clone();
            if desc.num_samples == 0 {
                return;
            }
            self.prepare_slot += 1;
            if self.prepare_slot >= self.slots.len() {
                self.prepare_slot = 0;
            }
            let Some(mut block) = desc.src.clone() else {
                return;
            };
            if self.uninitialised {
                self.reset(block.u32_be(0) & 3);
                self.uninitialised = false;
            }
            for (index, pair) in self.pairs.iter_mut().enumerate() {
                let size = Self::start_pair(pair, index, &desc, &block);
                block = block.at(size as usize);
                self.total_bytes_remaining += pair.bytes_remaining;
            }
        }
    }

    /// `EaXmaDec::EnterRecoveryMode` 0x82c16480: the hardware stalled; the rest of this request
    /// is silence and decoding restarts at the next one on reset contexts.
    fn enter_recovery_mode(&mut self) {
        self.failed = true;
        self.reset(self.current_decoder_table);
        for pair in &mut self.pairs {
            pair.bytes_remaining = 0;
            pair.output_offset = 0;
            pair.num_samples_to_skip = 0;
            pair.feed = None;
        }
        self.prepare_slot = self.decode_slot + 1;
        if self.prepare_slot >= self.slots.len() {
            self.prepare_slot = 0;
        }
        self.total_bytes_remaining = 0;
        self.service();
    }

    /// `EaXmaDec::DecodeEvent` 0x82c169f8.
    fn decode_event(&mut self, out: &mut [&mut [f32]], count: i32) -> i32 {
        let mut written = 0;
        let mut left = count;
        if count == 0 {
            return 0;
        }
        loop {
            if self.samples_left_this_feed_desc == 0 {
                let slot = self.decode_slot;
                let desc = &self.slots[slot];
                let extra = if desc.feed_type == 0 { 0x180 } else { 0 };
                let skip = desc.decoder_skip - ((desc.decoder_skip + extra) / 512) * 512 + self.num_padding_samples + extra;
                for pair in &mut self.pairs {
                    pair.num_samples_to_skip += skip;
                }
                self.num_padding_samples = 0;
                self.samples_left_this_feed_desc = desc.num_samples - desc.decoder_skip;
                self.current_request = slot;
                self.failed = false;
            }
            let n = left.min(self.samples_left_this_feed_desc);
            let mut done = 0;
            if !self.failed {
                let exhausted = self.total_bytes_remaining == 0 && self.slots[self.prepare_slot].num_samples == 0;
                for index in 0..self.pairs.len() {
                    let uninitialised = self.uninitialised;
                    let pair = &mut self.pairs[index];
                    pair.context.run();
                    let capacity = pair.context.capacity();
                    // The hardware had written the stream's last samples with its last frame;
                    // the decoder gives them up once the data is used up.
                    if exhausted
                        && pair.bytes_remaining == 0
                        && pair.context.inputs.is_empty()
                        && ring_available(&pair.context, pair.output_offset, uninitialised) < pair.num_samples_to_skip as isize + n as isize
                        && pair.context.drain()
                    {
                        pair.context.run();
                    }
                    let mut available = ring_available(&pair.context, pair.output_offset, uninitialised);
                    if pair.num_samples_to_skip != 0 && available != 0 {
                        let skip = (available as usize).min(pair.num_samples_to_skip as usize);
                        pair.output_offset += skip;
                        if pair.output_offset >= capacity {
                            pair.output_offset -= capacity;
                        }
                        available -= skip as isize;
                        pair.num_samples_to_skip -= skip as i32;
                    }
                    let stalled = available < n as isize || n == 0;
                    if !stalled {
                        let n = n as usize;
                        let channels = pair.context.channels;
                        let first = n.min(capacity - pair.output_offset);
                        for i in 0..n {
                            let frame = if i < first { pair.output_offset + i } else { i - first };
                            for c in 0..channels {
                                if let Some(channel) = out.get_mut(index * 2 + c) {
                                    channel[i] = f32::from(pair.context.ring[frame * channels + c]) * (1.0 / 32768.0);
                                }
                            }
                        }
                        pair.output_offset = if n > first { n - first } else { pair.output_offset + n };
                        done = n as i32;
                    } else {
                        self.service();
                    }
                    let pair = &mut self.pairs[index];
                    let read = if pair.output_offset == pair.context.capacity() {
                        pair.output_offset = 0;
                        0
                    } else {
                        pair.output_offset / FRAME * FRAME
                    };
                    if read != pair.context.hardware_read {
                        pair.context.hardware_read = read;
                        pair.context.output_valid = true;
                    }
                    if stalled {
                        self.enter_recovery_mode();
                        break;
                    }
                }
            }
            if self.failed {
                for channel in out.iter_mut().take(usize::from(self.num_channels)) {
                    channel[written as usize..(written + n) as usize].fill(0.0);
                }
                done = n;
            }
            written += done;
            left -= n;
            self.samples_left_this_feed_desc -= n;
            if self.samples_left_this_feed_desc == 0 && n != 0 {
                let desc = &self.slots[self.current_request];
                let mut end = desc.num_samples;
                if desc.feed_type == 0 {
                    end += 0x180;
                }
                let tail = end & 0x1FF;
                if tail != 0 {
                    self.num_padding_samples = 0x200 - tail;
                }
                if self.failed {
                    self.num_padding_samples = 0;
                }
            }
            self.service();
            if left == 0 {
                return written;
            }
        }
    }
}

/// The decoder `DecoderRegistry::DecoderFactory` makes for an EAAC codec (`SndPlayer1::sDecoderGuidLut`
/// 0x822af934): 3 (`EXm0`) is `EaXmaDec`; 5, 6 and 7 (`EL31`, `L32P`, `L32S`) are `EaLayer3DecBase`
/// versions 0 to 2. PCM (2, `P6B0`) and XAS (4, `Xas1`) are not ported, so their factory call fails
/// as for an unregistered GUID.
pub enum CodecDecoder {
    EaXma(Decoder),
    EaLayer3(super::ealayer3::EaLayer3Decoder),
}

impl CodecDecoder {
    pub fn new(codec: u8, pool: &mut XmaContextPool, num_channels: u8, max_slots: usize) -> Option<Self> {
        match codec {
            3 => Decoder::new(pool, num_channels, max_slots).map(Self::EaXma),
            5..=7 => Some(Self::EaLayer3(super::ealayer3::EaLayer3Decoder::new(codec - 5, num_channels, max_slots))),
            _ => None,
        }
    }

    pub fn release(self, pool: &mut XmaContextPool) {
        if let Self::EaXma(decoder) = self {
            decoder.release(pool);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn feed(
        &mut self,
        src: DataRef,
        num_samples: i32,
        feed_type: u8,
        decoder_skip: i32,
        seek_data: Option<DataRef>,
        seek_data_version: u8,
        num_bytes: i32,
    ) -> u8 {
        match self {
            Self::EaXma(decoder) => decoder.feed(src, num_samples, feed_type, decoder_skip, seek_data, seek_data_version, num_bytes),
            Self::EaLayer3(decoder) => decoder.feed(src, num_samples, feed_type, decoder_skip, seek_data, seek_data_version, num_bytes),
        }
    }

    #[must_use]
    pub fn samples_remaining(&self, slot: u8) -> i32 {
        match self {
            Self::EaXma(decoder) => decoder.samples_remaining(slot),
            Self::EaLayer3(decoder) => decoder.samples_remaining(slot),
        }
    }

    pub fn decode(&mut self, out: &mut [&mut [f32]], count: i32) -> i32 {
        match self {
            Self::EaXma(decoder) => decoder.decode(out, count),
            Self::EaLayer3(decoder) => decoder.decode(out, count),
        }
    }
}
