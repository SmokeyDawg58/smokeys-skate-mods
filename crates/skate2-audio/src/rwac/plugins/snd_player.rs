//! `rw::audio::core::SndPlayer1`: plays EAAC samples through a [`CodecDecoder`]. Requests queue from
//! `Play` events (`PlayHandler` 0x82c00b28); a `TIMERSTAGE_BEFOREMIXER` timer
//! (`RwacTimerClient` 0x82bff0e0) starts them and feeds their chunks; `Process` 0x82bfba90
//! decodes what the next plug-in asks for. RAM samples (play type 0) feed from memory; streamed
//! ones (type 1, and type 2's part past the RAM data) read their chunks through a stream of the
//! request's `StreamPool` ([`crate::rwac::stream_pool`]).

use std::sync::atomic::{AtomicU64, Ordering};

use crate::rwac::decoder::{CodecDecoder, DataRef, XmaContextPool};
use crate::rwac::mixer::Mixer;
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::seek;
use crate::rwac::stream_pool::{ChunkParser, ChunkTicket, StreamHandle, StreamPool, StreamPools};
use crate::rwac::voice::{Voice, VoiceCore, VoiceState};
use crate::rwac::FRAME_SAMPLES;

const FEED_SLOTS: usize = 20;
const DECODER_SLOTS: usize = 20;
/// `RwacTimerClient`: an EA-XMA request starts one mixer frame (256 / 48000 s) later.
const XMA_START_DELAY: f64 = f64::from_bits(0x3f75_d867_c3ec_e2a5);
/// The `AcquireStream` contexts: one per player, standing for its address.
static NEXT_CONTEXT: AtomicU64 = AtomicU64::new(1);

/// Parameters of `Play` event 5 (event 0 is the same without the seek).
#[derive(Clone, Default)]
pub struct PlayParams {
    pub start_time: f64,
    pub stream_file_offset: f64,
    pub seek_time: f64,
    pub stream_path: Option<String>,
    /// The sample: its EAAC header followed by its chunks.
    pub ram_sample: Option<DataRef>,
    pub seek_table: Option<DataRef>,
    pub stream_pool: u32,
    /// Rounded to a byte; 1 expels the voice after its decay once the request ends.
    pub expel_mode: f32,
}

/// `SndPlayer1::RequestInternal`.
#[derive(Default)]
struct RequestInternal {
    start_time: f64,
    decoder: Option<CodecDecoder>,
    request_handle: f32,
    sample_rate: f32,
    num_samples: i32,
    loop_start: i32,
    player_skip_remaining: i32,
    decoder_skip: i32,
    stream_skip: i32,
    /// 0 free, 1 queued, 2 started, 3 fully fed, 4 finished.
    state: u8,
    num_channels: u8,
}

/// `SndPlayer1::RequestExternal`.
#[derive(Default)]
struct RequestExternal {
    stream_file_offset: f64,
    sample_data: Option<DataRef>,
    loop_start_stream_offset: i32,
    /// +0x10: the samples a play type 2 sample keeps in RAM before its stream.
    giga: i32,
    num_samples_fed: i32,
    num_bytes_fed: i32,
    /// +0x1C: the stream path, kept when a loop or a header still has to be read from it.
    path: Option<String>,
    /// +0x24/+0x28: the acquired stream.
    stream: Option<StreamHandle>,
    /// +0x2C: the first `QueueFile` request.
    request_id: i32,
    next_chunk: Option<DataRef>,
    loop_start_chunk: Option<DataRef>,
    seek_data: Option<DataRef>,
    player_skip: i32,
    seek_chunk_offset: u32,
    seek_data_version: i32,
    codec: u8,
    play_type: u8,
    feed_slot_latest: u8,
    expel_mode: u8,
    format_version: u8,
    seek_chunk_is_new_feed: bool,
}

/// `SndPlayer1::SndPlayer1FeedDesc`.
#[derive(Clone, Copy, Default)]
struct FeedDesc {
    /// +0: the stream chunk the slot holds until `FeedCleanup`.
    chunk: Option<ChunkTicket>,
    chunk_samples_played: i32,
    decoder_request_handle: u8,
    /// 0 free, 1 fed, 2 consumed.
    feed_state: u8,
    request_index: u8,
}

pub struct SndPlayer1 {
    base: PlugInBase,
    internal: Vec<RequestInternal>,
    external: Vec<RequestExternal>,
    feed: [FeedDesc; FEED_SLOTS],
    current_request_handle: f32,
    current_request_sample_rate: f32,
    current_request_samples_played: i32,
    current_request_num_samples: i32,
    /// `*mpRequestHandle` and `*mpNumUnexecutedPlayHandlers` (the external buffer the game
    /// writes through `Play`).
    request_handle_counter: f32,
    num_unexecuted_play_handlers: u32,
    last_request_handle_processed: f32,
    last_request_handle_successfully_processed: f32,
    previous_sample_rate: f32,
    samples_requested: u16,
    declick: Vec<f32>,
    max_channels: u8,
    next_free_request: u8,
    next_request_to_free: u8,
    current_request: u8,
    max_requests: u8,
    dc_offsets_gathered: bool,
    num_declick_samples: u8,
    next_feed_slot_to_fill: u8,
    next_feed_slot_to_free: u8,
    /// What `AcquireStream` tells this player's streams apart by.
    context: u64,
}

fn active(state: u8) -> bool {
    state != 0 && state != 4
}

impl SndPlayer1 {
    /// `SndPlayer1::CreateInstance` 0x82bff4f8; `max_requests` is the constructor parameter.
    pub fn new(input_channels: u8, output_channels: u8, max_requests: f32) -> Self {
        let max_requests = crate::math::round_half_away(max_requests).clamp(1, 255) as u8;
        Self {
            base: PlugInBase::new(&super::SND_PLAYER1, input_channels, output_channels),
            internal: (0..max_requests).map(|_| RequestInternal::default()).collect(),
            external: (0..max_requests).map(|_| RequestExternal::default()).collect(),
            feed: [FeedDesc::default(); FEED_SLOTS],
            current_request_handle: 0.0,
            current_request_sample_rate: 48000.0,
            current_request_samples_played: 0,
            current_request_num_samples: 0,
            request_handle_counter: 0.0,
            num_unexecuted_play_handlers: 0,
            last_request_handle_processed: 0.0,
            last_request_handle_successfully_processed: 0.0,
            previous_sample_rate: 48000.0,
            samples_requested: 0,
            declick: vec![0.0; usize::from(output_channels)],
            max_channels: output_channels,
            next_free_request: 0,
            next_request_to_free: 0,
            current_request: 0,
            max_requests,
            dc_offsets_gathered: false,
            num_declick_samples: 0,
            next_feed_slot_to_fill: 0,
            next_feed_slot_to_free: 0,
            context: NEXT_CONTEXT.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// `EventEvent` events 0/5, the game side: takes the next request handle (wrapping at
    /// 4194304) and counts an unexecuted play. The caller queues [`Self::play_handler`].
    pub fn begin_play(&mut self) -> f32 {
        self.request_handle_counter += 1.0;
        self.num_unexecuted_play_handlers += 1;
        if self.request_handle_counter > 4_194_304.0 {
            self.request_handle_counter = 1.0;
        }
        self.request_handle_counter
    }

    /// `EventEvent` event 2: whether request `handle` is done. One older than the current request
    /// is; the current one, or one the play handler processed without success, is once the player
    /// is idle (attribute 2, the current request's length, is 0); one not processed yet is not.
    #[must_use]
    pub fn request_done(&self, handle: f32) -> bool {
        let current = self.base.attributes[0] as f32;
        let idle = self.base.attributes[2] == 0.0;
        if handle < current {
            return true;
        }
        if handle == current {
            return idle;
        }
        if handle > self.last_request_handle_processed || handle <= self.last_request_handle_successfully_processed {
            return false;
        }
        idle
    }

    /// `EventEvent` event 3's second answer: the active request with the handle is buffered. A
    /// RAM sample (play type 0) is; a streamed one (1, 2) once it has no stream, or its first
    /// stream request is read to its end or the stream stopped with a full buffer.
    #[must_use]
    pub fn request_buffered(&self, handle: f32) -> bool {
        for (internal, external) in self.internal.iter().zip(&self.external) {
            if internal.request_handle != handle || !active(internal.state) {
                continue;
            }
            match external.play_type {
                1 | 2 => return external.stream.as_ref().is_none_or(|stream| stream.request_buffered(external.request_id)),
                0 => return true,
                _ => {}
            }
        }
        false
    }

    /// `ModifyStartTimeHandler` 0x82bfb0b8 (event 4): the first active request with the handle
    /// takes the new start time while its own is still ahead of the system time.
    pub fn modify_start_time_handler(&mut self, handle: f32, start_time: f64, system_time: f64) {
        let Some(request) = self.internal.iter_mut().find(|request| request.request_handle == handle && active(request.state)) else {
            return;
        };
        if request.start_time > system_time {
            request.start_time = start_time;
        }
    }

    /// `PlayHandler` 0x82c00b28. A streamed sample takes a stream of its pool (`GetInstance`,
    /// `AcquireStream`; the request ends when there is none) and queues its file from the seek
    /// chunk, then, when looped, twice from the loop start.
    pub fn play_handler(&mut self, params: PlayParams, handle: f32, pools: &StreamPools) {
        self.num_unexecuted_play_handlers = self.num_unexecuted_play_handlers.wrapping_sub(1);
        self.last_request_handle_processed = handle;
        let index = usize::from(self.next_free_request);
        if self.internal[index].state != 0 {
            return;
        }
        {
            let (req, ext) = (&mut self.internal[index], &mut self.external[index]);
            req.request_handle = handle;
            req.decoder = None;
            req.start_time = params.start_time;
            ext.stream_file_offset = params.stream_file_offset;
            ext.expel_mode = crate::math::round_half_away(params.expel_mode) as u8;
            req.state = 1;
            ext.num_samples_fed = 0;
            ext.num_bytes_fed = 0;
            ext.stream = None;
            ext.request_id = 0;
            ext.path = None;
        }
        self.unpack_header(index, params.ram_sample.as_ref());
        if params.seek_time > 0.0 && self.internal[index].num_channels == 0 {
            // `SeekTableParser::ParseHeaderData` 0x82be3220: a version 1 table carries the header.
            let header = params.seek_table.as_ref().filter(|table| table.slice().first() == Some(&1)).map(|table| table.at(12));
            self.unpack_header(index, header.as_ref());
        }
        let req = &self.internal[index];
        let mut seek_sample = (f64::from(req.sample_rate) * params.seek_time) as i32;
        if seek_sample <= 0 || self.external[index].play_type == 2 || req.loop_start >= 0 {
            seek_sample = 0;
        }
        if req.num_samples <= seek_sample {
            self.internal[index].state = 0;
            self.internal[index].num_samples = 0;
            return;
        }
        self.set_seek_data(index, params.seek_table.as_ref(), seek_sample);
        let ext = &mut self.external[index];
        if ext.format_version == 1 && params.stream_path.as_deref().is_none_or(str::is_empty) {
            if let Some(sample) = &params.ram_sample {
                let next = sample.at(ext.seek_chunk_offset as usize);
                ext.loop_start_chunk = Some(next.at(ext.loop_start_stream_offset as u32 as usize));
                ext.next_chunk = Some(next);
            }
        } else if ext.play_type == 1 || ext.play_type == 2 {
            let Some(stream) = pools.instance(params.stream_pool).and_then(|pool| StreamPool::acquire(&pool, self.context)) else {
                self.internal[index].state = 0;
                self.internal[index].num_samples = 0;
                return;
            };
            let path = params.stream_path.unwrap_or_default();
            let req = &self.internal[index];
            if req.loop_start >= 0 || req.num_channels == 0 {
                ext.path = Some(path.clone());
            }
            let parser = if ext.format_version == 0 { ChunkParser::Format0 } else { ChunkParser::Format1 };
            if !(ext.play_type == 2 && req.loop_start >= 0 && ext.giga > req.loop_start) {
                let offset = (ext.stream_file_offset as i64).wrapping_add(i64::from(ext.seek_chunk_offset));
                ext.request_id = stream.queue_file(&path, offset as u64, parser);
            }
            let queue_loop = req.loop_start >= 0 && !(ext.play_type == 2 && ext.giga >= req.num_samples);
            ext.stream = Some(stream);
            if queue_loop {
                for _ in 0..2 {
                    let id = self.queue_loop(index, parser);
                    if self.external[index].request_id == 0 {
                        self.external[index].request_id = id;
                    }
                }
            }
        }
        self.internal[index].state = 1;
        self.next_free_request += 1;
        if self.next_free_request == self.max_requests {
            self.next_free_request = 0;
        }
        self.last_request_handle_successfully_processed = handle;
    }

    /// `StopHandler` 0x82bff708.
    pub fn stop_handler(&mut self, voice: &mut VoiceCore, pool: &mut XmaContextPool) {
        self.remove_all_requests(voice, pool);
    }

    /// `UnpackHeader` 0x82bf2390.
    fn unpack_header(&mut self, index: usize, header: Option<&DataRef>) {
        let (req, ext) = (&mut self.internal[index], &mut self.external[index]);
        let Some(header) = header else {
            req.num_channels = 0;
            req.sample_rate = 0.0;
            req.num_samples = 0x7FFF_FFFF;
            req.loop_start = -1;
            ext.codec = 0xFF;
            ext.play_type = 1;
            ext.loop_start_stream_offset = 0;
            ext.sample_data = None;
            ext.format_version = 1;
            return;
        };
        let mut start = header.clone();
        if start.slice().first() == Some(&b'H') {
            start = start.at(4);
        }
        let mut bits = crate::formats::snr::BitGetter::new(start.slice());
        ext.format_version = bits.get(4) as u8;
        ext.codec = bits.get(4) as u8;
        req.num_channels = bits.get(6) as u8 + 1;
        req.sample_rate = bits.get(18) as f32;
        ext.play_type = bits.get(2) as u8;
        let looping = bits.get(1) != 0;
        req.num_samples = bits.get(29) as i32;
        req.loop_start = if looping { bits.get(32) as i32 } else { -1 };
        ext.giga = 0;
        if ext.play_type == 2 {
            ext.giga = bits.get(32) as i32;
        }
        if looping {
            ext.loop_start_stream_offset =
                if ext.play_type == 1 || (ext.play_type == 2 && req.loop_start >= ext.giga) { bits.get(32) as i32 } else { 0 };
        }
        ext.sample_data = Some(start.at(bits.position() >> 3));
    }

    /// `SetSeekData` 0x82bf2568.
    fn set_seek_data(&mut self, index: usize, table: Option<&DataRef>, sample: i32) {
        let (req, ext) = (&mut self.internal[index], &mut self.external[index]);
        match table {
            Some(table) if sample > 0 => {
                let position = seek::parse(table.slice(), sample);
                req.decoder_skip = position.decoder_skip;
                req.stream_skip = position.stream_skip;
                ext.player_skip = position.player_skip;
                ext.seek_chunk_offset = position.chunk_offset;
                ext.seek_data = position.seek_data.map(|offset| table.at(offset));
                ext.seek_data_version = position.seek_data_version;
                ext.seek_chunk_is_new_feed = position.is_new_feed_chunk;
                req.player_skip_remaining = 0;
                ext.num_samples_fed = req.stream_skip;
            }
            _ => {
                req.decoder_skip = 0;
                ext.player_skip = 0;
                ext.seek_chunk_offset = 0;
                ext.seek_data = None;
                ext.seek_chunk_is_new_feed = true;
                req.player_skip_remaining = 0;
                req.stream_skip = 0;
            }
        }
    }

    fn claim_feed_slot(&mut self) -> Option<u8> {
        let slot = self.next_feed_slot_to_fill;
        if self.feed[usize::from(slot)].feed_state == 0 {
            self.next_feed_slot_to_fill = ((usize::from(slot) + 1) % FEED_SLOTS) as u8;
            Some(slot)
        } else {
            None
        }
    }

    /// `SubmitChunk` 0x82bf2218: feeds one chunk to the request's decoder; returns the chunk
    /// after it.
    fn submit_chunk(&mut self, chunk: &DataRef, index: usize, reset_decoder: bool, seeking: bool) -> DataRef {
        let data = chunk.slice();
        let word = |at: usize| data.get(at..at + 4).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
        let size = if self.external[index].format_version == 0 { word(0) } else { word(0) & 0x00FF_FFFF };
        let samples = word(4) as i32;
        let slot = usize::from(self.external[index].feed_slot_latest);
        self.feed[slot].request_index = index as u8;
        self.feed[slot].feed_state = 1;
        self.feed[slot].chunk_samples_played = 0;
        let feed_type = u8::from(!reset_decoder);
        let decoder_skip = if seeking { self.internal[index].decoder_skip } else { 0 };
        if seeking {
            self.feed[slot].chunk_samples_played = decoder_skip;
        }
        let (seek_data, seek_data_version) = if seeking {
            (self.external[index].seek_data.clone(), self.external[index].seek_data_version as u8)
        } else {
            (None, 0)
        };
        let handle = match self.internal[index].decoder.as_mut() {
            Some(decoder) => {
                decoder.feed(chunk.at(8), samples, feed_type, decoder_skip, seek_data, seek_data_version, size as i32 - 8)
            }
            None => 0,
        };
        self.feed[slot].decoder_request_handle = handle;
        self.external[index].num_samples_fed += samples;
        chunk.at(size as usize)
    }

    /// `GetNextChunk` 0x82bf2658: the next data chunk from the request's stream (`Stream::GetChunk`,
    /// with the chunk's ticket) or its RAM data, skipping 'U' chunks, 'H' chunks once the header is
    /// known and 'E' (which in RAM data leads back to the loop start). A skipped stream chunk is
    /// released at once.
    fn next_chunk(&mut self, index: usize) -> Option<(DataRef, u32, Option<ChunkTicket>)> {
        loop {
            let (chunk, size, ticket) = if let Some(stream) = &self.external[index].stream {
                let chunk = stream.get_chunk()?;
                (chunk.data, chunk.ticket.size(), Some(chunk.ticket))
            } else {
                let chunk = self.external[index].next_chunk.clone()?;
                let data = chunk.slice();
                let kind = *data.first()?;
                let size = data.get(1..4).map_or(0, |b| u32::from_be_bytes([0, b[0], b[1], b[2]]));
                let next = if kind == b'E' { self.external[index].loop_start_chunk.clone() } else { Some(chunk.at(size as usize)) };
                self.external[index].next_chunk = next;
                (chunk, size, None)
            };
            let kind = chunk.slice().first().copied();
            match kind {
                Some(b'E' | b'U') => {}
                Some(b'H') if self.internal[index].num_channels != 0 => {}
                _ => return Some((chunk, size, ticket)),
            }
            match (ticket, &self.external[index].stream) {
                (Some(ticket), Some(stream)) => stream.release_chunk(ticket),
                _ if size == 0 && kind != Some(b'E') => return None,
                _ => {}
            }
        }
    }

    /// `StreamNextChunk` 0x82bfb150. While the request waits to start, a first stream request
    /// that has gone free (its file could not be read) ends it.
    fn stream_next_chunk(&mut self, index: usize, reset_decoder: bool, seeking: bool) -> bool {
        let ext = &self.external[index];
        if self.internal[index].state == 1 && ext.request_id != 0 && ext.stream.as_ref().is_some_and(|stream| !stream.request_live(ext.request_id)) {
            self.internal[index].num_samples = 0;
            return false;
        }
        let Some((chunk, size, ticket)) = self.next_chunk(index) else {
            return false;
        };
        let slot = self.claim_feed_slot();
        self.external[index].num_bytes_fed += size as i32;
        let Some(slot) = slot else {
            return false;
        };
        self.external[index].feed_slot_latest = slot;
        self.feed[usize::from(slot)].chunk = ticket;
        self.submit_chunk(&chunk, index, reset_decoder, seeking);
        true
    }

    /// `Stream::QueueFile` of the loop: the request's path from its loop start stream offset.
    fn queue_loop(&self, index: usize, parser: ChunkParser) -> i32 {
        let ext = &self.external[index];
        let Some(stream) = &ext.stream else { return 0 };
        let offset = (f64::from(ext.loop_start_stream_offset) + ext.stream_file_offset) as i64;
        stream.queue_file(ext.path.as_deref().unwrap_or_default(), offset as u64, parser)
    }

    /// A stream chunk a feed slot let go: `ReleaseChunk`, and the request's fed bytes drop.
    fn release_feed_chunk(&mut self, index: usize, ticket: ChunkTicket) {
        let ext = &mut self.external[index];
        ext.num_bytes_fed -= ticket.size() as i32;
        if let Some(stream) = &ext.stream {
            stream.release_chunk(ticket);
        }
    }

    /// `StartRequest` 0x82bfb718: a version 1 stream played without a header reads it from its
    /// first chunk (and queues its loop twice); then the decoder is created and the first chunk
    /// fed.
    fn start_request(&mut self, index: usize, pool: &mut XmaContextPool) -> bool {
        if self.external[index].format_version == 1 && self.internal[index].num_channels == 0 {
            let Some((chunk, _, ticket)) = self.next_chunk(index) else {
                return false;
            };
            self.unpack_header(index, Some(&chunk.at(4)));
            if self.internal[index].loop_start >= 0 {
                for _ in 0..2 {
                    self.queue_loop(index, ChunkParser::Format1);
                }
            }
            if let (Some(stream), Some(ticket)) = (&self.external[index].stream, ticket) {
                stream.release_chunk(ticket);
            }
        }
        // `DecoderRegistry::DecoderFactory` with `sDecoderGuidLut[codec]`.
        let Some(decoder) = CodecDecoder::new(self.external[index].codec, pool, self.internal[index].num_channels, DECODER_SLOTS) else {
            return false;
        };
        self.internal[index].decoder = Some(decoder);
        let req = &self.internal[index];
        let seeking = req.stream_skip != 0 || req.decoder_skip != 0 || self.external[index].player_skip != 0;
        let play_type = self.external[index].play_type;
        if play_type == 0 || play_type == 2 {
            let slot = self.claim_feed_slot().unwrap_or(0);
            self.external[index].feed_slot_latest = slot;
            let Some(data) = self.external[index].sample_data.clone() else {
                return false;
            };
            let first = data.at(self.external[index].seek_chunk_offset as usize);
            let reset = self.external[index].seek_chunk_is_new_feed;
            let next = self.submit_chunk(&first, index, reset, seeking);
            self.external[index].next_chunk = Some(next);
            true
        } else {
            let reset = self.external[index].seek_chunk_is_new_feed;
            if self.stream_next_chunk(index, reset, seeking) {
                true
            } else {
                if let Some(decoder) = self.internal[index].decoder.take() {
                    decoder.release(pool);
                }
                false
            }
        }
    }

    /// `HandleLoopStart` 0x82bfb298: RAM data keeps the loop start chunk and feeds it; a stream
    /// (or a play type 2 loop past the RAM data) feeds its next chunk.
    fn handle_loop_start(&mut self, index: usize) -> bool {
        let play_type = self.external[index].play_type;
        if play_type == 1 || (play_type != 0 && self.internal[index].loop_start >= self.external[index].giga) {
            return self.stream_next_chunk(index, true, false);
        }
        self.external[index].loop_start_chunk = self.external[index].next_chunk.clone();
        let slot = self.claim_feed_slot().unwrap_or(0);
        self.external[index].feed_slot_latest = slot;
        let Some(chunk) = self.external[index].next_chunk.clone() else {
            return false;
        };
        let next = self.submit_chunk(&chunk, index, true, false);
        self.external[index].next_chunk = Some(next);
        true
    }

    /// `HandleSampleEnd` 0x82bfb440: completes an unlooped request; a looped one goes back to its
    /// loop start. RAM data feeds the loop start chunk; a stream queues the loop once more (one
    /// stays queued ahead) and feeds its next chunk; a play type 2 sample does the RAM part, then
    /// queues its streamed part.
    fn handle_sample_end(&mut self, index: usize) -> (bool, bool) {
        let loop_start = self.internal[index].loop_start;
        if loop_start < 0 {
            return (true, true);
        }
        let play_type = self.external[index].play_type;
        if play_type == 1 {
            let ext = &self.external[index];
            if !(ext.format_version == 1 && ext.path.is_none()) {
                let parser = if ext.format_version == 0 { ChunkParser::Format0 } else { ChunkParser::Format1 };
                self.queue_loop(index, parser);
            }
            self.external[index].num_samples_fed = loop_start;
            return (self.stream_next_chunk(index, true, false), false);
        }
        self.external[index].num_samples_fed = loop_start;
        if play_type == 0 {
            return (self.feed_loop_start(index), false);
        }
        let giga = self.external[index].giga;
        if loop_start < giga {
            self.feed_loop_start(index);
        }
        if giga < self.internal[index].num_samples {
            self.queue_loop(index, ChunkParser::Format0);
            if loop_start >= giga && !self.stream_next_chunk(index, true, false) {
                return (false, false);
            }
        }
        (true, false)
    }

    /// The RAM loop: the loop start chunk (the sample's first for a loop at 0), fed with a decoder
    /// reset.
    fn feed_loop_start(&mut self, index: usize) -> bool {
        if self.internal[index].loop_start == 0 {
            self.external[index].loop_start_chunk = self.external[index].sample_data.clone();
        }
        let slot = self.claim_feed_slot().unwrap_or(0);
        self.external[index].feed_slot_latest = slot;
        let Some(chunk) = self.external[index].loop_start_chunk.clone() else {
            return false;
        };
        let next = self.submit_chunk(&chunk, index, true, false);
        self.external[index].next_chunk = Some(next);
        true
    }

    /// `FeedCleanup` 0x82bfafc0: frees consumed feed slots the decoder no longer holds and
    /// releases their stream chunks.
    fn feed_cleanup(&mut self) {
        for slot in 0..FEED_SLOTS {
            let feed = self.feed[slot];
            if feed.feed_state == 2 {
                let request = usize::from(feed.request_index);
                let remaining = self.internal[request].decoder.as_ref().map_or(0, |d| d.samples_remaining(feed.decoder_request_handle));
                if remaining == 0 {
                    self.feed[slot].feed_state = 0;
                    if let Some(ticket) = self.feed[slot].chunk.take() {
                        self.release_feed_chunk(request, ticket);
                    }
                }
            }
        }
    }

    /// `RemoveRequest` 0x82bfb978: also releases the request's stream chunks, its stream
    /// (`ReleaseStream`) and its path.
    fn remove_request(&mut self, index: usize, voice: &mut VoiceCore, pool: &mut XmaContextPool) {
        if let Some(decoder) = self.internal[index].decoder.take() {
            decoder.release(pool);
        }
        for slot in 0..FEED_SLOTS {
            if usize::from(self.feed[slot].request_index) == index {
                self.feed[slot].feed_state = 0;
                if let Some(ticket) = self.feed[slot].chunk.take() {
                    self.release_feed_chunk(index, ticket);
                }
            }
        }
        if let Some(stream) = self.external[index].stream.take() {
            stream.release();
        }
        self.external[index].path = None;
        self.internal[index].state = 0;
        if self.external[index].expel_mode == 1 {
            Voice::expel_after_decay(voice);
        }
    }

    /// `RequestCleanup` 0x82bff038.
    fn request_cleanup(&mut self, voice: &mut VoiceCore, pool: &mut XmaContextPool) {
        while self.internal[usize::from(self.next_request_to_free)].state == 4 {
            let index = usize::from(self.next_request_to_free);
            self.remove_request(index, voice, pool);
            self.next_request_to_free = (self.next_request_to_free + 1) % self.max_requests;
        }
    }

    /// `RemoveAllRequests` 0x82bff468: also arms a 16-sample declick.
    fn remove_all_requests(&mut self, voice: &mut VoiceCore, pool: &mut XmaContextPool) {
        for index in 0..usize::from(self.max_requests) {
            if self.internal[index].state != 0 {
                self.remove_request(index, voice, pool);
            }
        }
        self.current_request = 0;
        self.next_free_request = 0;
        self.next_request_to_free = 0;
        self.current_request_samples_played = 0;
        self.current_request_num_samples = 0;
        self.next_feed_slot_to_fill = 0;
        self.next_feed_slot_to_free = 0;
        self.num_declick_samples = 16;
    }

    /// `AdvanceCurrentRequest` 0x82bf2930.
    fn advance_current_request(&mut self) {
        self.current_request = (self.current_request + 1) % self.max_requests;
        self.current_request_samples_played = 0;
        self.current_request_num_samples = 0;
        let req = &self.internal[usize::from(self.current_request)];
        if active(req.state) {
            self.current_request_samples_played = 0;
            self.current_request_handle = req.request_handle;
            self.current_request_sample_rate = req.sample_rate;
            self.current_request_num_samples = req.num_samples;
        }
    }

    /// `Declick` 0x82bf2798: ramps the last output values to zero.
    fn declick_output(&mut self, mixer: &mut Mixer) -> BufferStatus {
        let total = u32::from(self.num_declick_samples);
        let count = u32::from(self.num_declick_samples).min(u32::from(self.samples_requested));
        let step_scale = 1.0 / total as f32;
        for channel in 0..usize::from(self.base.output_channels) {
            let mut value = self.declick.get(channel).copied().unwrap_or(0.0);
            let step = value * step_scale;
            let out = mixer.scratch_mut(channel);
            for sample in out.iter_mut().take(count as usize) {
                value -= step;
                *sample = value;
            }
            if let Some(slot) = self.declick.get_mut(channel) {
                *slot = value;
            }
        }
        self.num_declick_samples -= count as u8;
        mixer.swap();
        mixer.src_num_channels = self.base.output_channels;
        mixer.src_sample_rate = self.previous_sample_rate;
        mixer.src_num_samples = count as i32;
        if self.num_declick_samples == 0 {
            self.dc_offsets_gathered = false;
        }
        BufferStatus::Available
    }

    /// `GenericPlayer::WaitForStartTime` 0x82bec5d0: `None` while the start is a frame or more
    /// away, else the source samples of silence before it.
    fn wait_for_start_time(mixer: &Mixer, start_time: f64) -> Option<u32> {
        let ahead = start_time - mixer.mix_time;
        if ahead > 0.0 {
            let samples = (f64::from(mixer.params_sample_rate) * ahead) as f32;
            if samples >= 256.0 {
                return None;
            }
            Some((mixer.total_pitch * samples) as i64 as u32)
        } else {
            Some(0)
        }
    }

    /// The end of `Process` (`abort_48`): reports the current format, keeps the last values
    /// for a declick, and decides the buffer status.
    fn finish(&mut self, mixer: &mut Mixer, produced: i32, skipped: i32) -> BufferStatus {
        mixer.src_num_channels = self.base.output_channels;
        mixer.src_sample_rate = self.previous_sample_rate;
        if produced != 0 {
            let channels = usize::from(mixer.src_num_channels.min(self.max_channels));
            for channel in 0..channels {
                self.declick[channel] = mixer.current(channel)[produced as usize - 1];
            }
            self.dc_offsets_gathered = true;
            BufferStatus::Available
        } else if skipped != 0 || self.samples_requested == 0 {
            mixer.src_num_samples = 0;
            BufferStatus::Available
        } else {
            BufferStatus::Unavailable
        }
    }
}

impl PlugIn for SndPlayer1 {
    impl_plugin_any!();

    /// `SndPlayer1::PreProcess` 0x82bf2920.
    fn pre_process(&mut self, _mixer: &mut Mixer, _discontinuity: bool, requested: i32) -> i32 {
        self.samples_requested = requested as u16;
        0
    }

    /// `SndPlayer1::Process` 0x82bfba90.
    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let mixer = &mut *services.mixer;
        if self.num_declick_samples != 0 && self.dc_offsets_gathered {
            return self.declick_output(mixer);
        }
        let mut index = usize::from(self.current_request);
        if !active(self.internal[index].state) {
            return self.finish(mixer, 0, 0);
        }
        while self.internal[index].num_samples == 0 {
            self.internal[index].state = 4;
            self.advance_current_request();
            index = usize::from(self.current_request);
            if !active(self.internal[index].state) {
                return self.finish(mixer, 0, 0);
            }
        }
        let state = self.internal[index].state;
        if state != 2 && state != 3 {
            return self.finish(mixer, 0, 0);
        }
        let req = &self.internal[index];
        if req.sample_rate != self.previous_sample_rate || req.num_channels != self.base.output_channels {
            mixer.src_num_samples = 0;
            mixer.src_num_channels = req.num_channels;
            mixer.src_sample_rate = req.sample_rate;
            self.previous_sample_rate = req.sample_rate;
            self.base.output_channels = req.num_channels;
            return BufferStatus::Available;
        }
        if self.feed[usize::from(self.next_feed_slot_to_free)].feed_state == 0 {
            while self.next_feed_slot_to_free != self.next_feed_slot_to_fill {
                self.next_feed_slot_to_free = ((usize::from(self.next_feed_slot_to_free) + 1) % FEED_SLOTS) as u8;
                if self.feed[usize::from(self.next_feed_slot_to_free)].feed_state != 0 {
                    break;
                }
            }
        }
        if self.feed[usize::from(self.next_feed_slot_to_free)].feed_state != 1 {
            return self.finish(mixer, 0, 0);
        }
        if self.internal[index].start_time != 0.0 {
            match Self::wait_for_start_time(mixer, self.internal[index].start_time) {
                None => {
                    self.current_request_samples_played = 0;
                    return self.finish(mixer, 0, 0);
                }
                Some(silence) if silence != 0 => {
                    let silence = silence.min(u32::from(self.samples_requested)) as usize;
                    for channel in 0..usize::from(self.internal[index].num_channels) {
                        mixer.scratch_mut(channel)[..silence].fill(0.0);
                    }
                    mixer.swap();
                    mixer.src_num_samples = silence as i32;
                    mixer.src_num_channels = self.internal[index].num_channels;
                    mixer.src_sample_rate = self.internal[index].sample_rate;
                    self.current_request_samples_played = 0;
                    return BufferStatus::Available;
                }
                Some(_) => self.internal[index].start_time = 0.0,
            }
        }

        // `mpLoadedDecoder`: the current request's decoder, which also answers for the next feed
        // slot below.
        let mut loaded = Some(index);
        let feed_slot = usize::from(self.next_feed_slot_to_free);
        let decoder_handle = self.feed[feed_slot].decoder_request_handle;
        let channels = usize::from(self.internal[index].num_channels);
        let remaining = self.internal[index].decoder.as_ref().map_or(0, |d| d.samples_remaining(decoder_handle));
        let mut skip = self.internal[index].player_skip_remaining.min(remaining);
        let count = i32::from(self.samples_requested).min(remaining - skip);
        let mut skipped = 0;
        let produced;
        {
            let (_, scratch) = mixer.current_and_scratch();
            let mut outputs: Vec<&mut [f32]> = scratch.chunks_exact_mut(FRAME_SAMPLES).take(channels).collect();
            let decoder = self.internal[index].decoder.as_mut();
            if let Some(decoder) = decoder {
                while skip != 0 {
                    let n = skip.min(256);
                    skip -= n;
                    skipped += decoder.decode(&mut outputs, n);
                }
                produced = decoder.decode(&mut outputs, count);
            } else {
                produced = 0;
            }
        }
        mixer.src_num_samples = produced;
        mixer.swap();
        mixer.src_num_channels = self.internal[index].num_channels;
        mixer.src_sample_rate = self.internal[index].sample_rate;
        self.current_request_handle = self.internal[index].request_handle;
        if self.current_request_samples_played == 0 {
            self.current_request_samples_played = self.internal[index].stream_skip + self.internal[index].decoder_skip;
        }
        let left_in_feed = remaining - produced - skipped;
        self.current_request_samples_played += produced + skipped;
        self.current_request_sample_rate = self.internal[index].sample_rate;
        self.current_request_num_samples = self.internal[index].num_samples;
        self.feed[feed_slot].chunk_samples_played += produced + skipped;
        if self.current_request_samples_played == self.internal[index].num_samples {
            let loop_start = self.internal[index].loop_start;
            if loop_start >= 0 {
                self.current_request_samples_played = loop_start;
            } else {
                self.internal[index].state = 4;
                loaded = None;
                self.advance_current_request();
                let next = usize::from(self.current_request);
                if active(self.internal[next].state) && self.internal[next].decoder.is_some() {
                    loaded = Some(next);
                }
            }
        }
        if left_in_feed == 0 {
            loop {
                let slot = usize::from(self.next_feed_slot_to_free);
                if self.feed[slot].feed_state != 1 {
                    break;
                }
                self.feed[slot].feed_state = 2;
                self.next_feed_slot_to_free = ((slot + 1) % FEED_SLOTS) as u8;
                let next = usize::from(self.next_feed_slot_to_free);
                let mut next_remaining = 0;
                if let Some(decoder) = loaded.and_then(|request| self.internal[request].decoder.as_ref()) {
                    if self.feed[next].feed_state == 1 {
                        next_remaining = decoder.samples_remaining(self.feed[next].decoder_request_handle);
                    }
                }
                if next_remaining != 0 {
                    break;
                }
            }
        }
        self.finish(mixer, produced, skipped)
    }

    /// `RwacTimerClient` 0x82bff0e0.
    fn timer(&mut self, voice: &mut VoiceCore, services: &mut Services<'_>) {
        if voice.state == VoiceState::Expelled {
            return;
        }
        self.feed_cleanup();
        self.request_cleanup(voice, services.xma_contexts);
        let current = usize::from(self.current_request);
        if !active(self.internal[current].state) {
            if self.num_unexecuted_play_handlers == 0 {
                self.current_request_handle = self.request_handle_counter;
            }
            self.base.attributes[0] = f64::from(self.current_request_handle);
            self.base.attributes[1] = 0.0;
            self.base.attributes[2] = 0.0;
            return;
        }
        self.base.attributes[0] = f64::from(self.current_request_handle);
        let rate = f64::from(self.current_request_sample_rate);
        self.base.attributes[2] = f64::from(self.current_request_num_samples) / rate;
        self.base.attributes[1] = f64::from(self.current_request_samples_played) / rate;
        let max = usize::from(self.max_requests);
        let mut index = current;
        if self.internal[index].num_samples == 0 {
            loop {
                index = (index + 1) % max;
                if index == current || !active(self.internal[index].state) {
                    return;
                }
                if self.internal[index].num_samples != 0 {
                    break;
                }
            }
        }
        loop {
            if !active(self.internal[index].state) {
                return;
            }
            if self.feed[usize::from(self.next_feed_slot_to_fill)].feed_state != 0 {
                return;
            }
            if self.internal[index].state == 1 {
                if !self.start_request(index, services.xma_contexts) {
                    return;
                }
                self.internal[index].state = 2;
                if self.external[index].codec == 3 && self.internal[index].start_time == 0.0 && index == usize::from(self.current_request) {
                    self.internal[index].start_time = services.system_time + XMA_START_DELAY;
                }
            }
            let mut advance = true;
            if self.internal[index].state == 2 && self.feed[usize::from(self.next_feed_slot_to_fill)].feed_state == 0 {
                let fed = self.external[index].num_samples_fed;
                if fed == self.internal[index].loop_start {
                    if !self.handle_loop_start(index) {
                        return;
                    }
                    advance = false;
                } else if fed == self.internal[index].num_samples {
                    let (ok, completed) = self.handle_sample_end(index);
                    if !ok {
                        return;
                    }
                    if completed {
                        self.internal[index].state = 3;
                    } else {
                        advance = false;
                    }
                } else {
                    if !self.stream_next_chunk(index, false, false) {
                        return;
                    }
                    advance = false;
                }
            }
            if advance {
                index = (index + 1) % max;
                if index == usize::from(self.current_request) {
                    return;
                }
            }
        }
    }

    /// `ReleaseEvent` 0x82c00ac8.
    fn release(&mut self, voice: &mut VoiceCore, services: &mut Services<'_>) {
        self.remove_all_requests(voice, services.xma_contexts);
    }
}
