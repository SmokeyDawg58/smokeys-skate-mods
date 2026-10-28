//! `Path::PathTrackRWAudioCoreStream` (`pathrwaudiocore.cpp`): the stream implementation
//! (`IPathTrack`) a Pathfinder track plays its nodes through. One stage 0 voice holds
//! `SndPlayer1` (`maxRequests` queued node segments), `Rechannel`, `Resample`, one `Route` per
//! channel set, one FX `Send` and the dry `Send`; each routed channel set has its own stage 1
//! voice (`Pan2D1ChannelSet`: `SubMix`, `Pan2D1`, `Send`).

use std::sync::Arc;

use crate::formats::pathfinder::{find_sample_offsets, TrackInfo};
use crate::rwac::core::{AudioCore, PlugInConfig, PlugInKind};
use crate::rwac::decoder::DataRef;
use crate::rwac::plugins::snd_player::PlayParams;
use crate::rwac::voice::VoiceState;
use crate::rwac::{PluginRef, VoiceId};

/// `1.0f / 127.0f` (0.0078740157).
const INV_127: f32 = f32::from_bits(0x3C01_0204);
/// `1.0f / 16384.0f`.
const INV_16384: f32 = f32::from_bits(0x3880_0000);
/// `1.0f / 4096.0f`.
const INV_4096: f32 = f32::from_bits(0x3980_0000);

/// `PATHSTATUS`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PathStatus {
    pub playing_node: i32,
    pub playing_bar: i32,
    pub playing_beat: i32,
    pub current_node: i32,
    pub current_request: i32,
    pub ready_for_request: i32,
    pub node_duration: u32,
    pub bar_duration: u32,
    pub beat_duration: u32,
    pub time_left_in_node: u32,
    pub time_left_in_bar: u32,
    pub time_left_in_beat: u32,
    pub time_buffered: i32,
    pub time_remaining: i32,
    pub num_bars: i32,
    pub num_beats: i32,
}

/// `PATHQUEUEINFO`: one queued node segment. `requesthandle` holds the player's float handle.
#[derive(Debug, Clone, Copy, PartialEq)]
struct QueueInfo {
    duration: i32,
    request_handle: f32,
    time_remaining: i32,
    node: i32,
    beat: i32,
}

impl QueueInfo {
    const FREE: Self = Self { duration: 0, request_handle: -1.0, time_remaining: 0, node: -1, beat: 0 };
}

/// `Path::Pan2D1ChannelSet` (`ChannelSet` + its voice and levels).
#[derive(Debug, Clone)]
pub struct ChannelSet {
    /// `channelset`: the first source channel `MapChannels` was given (-1 until then).
    channel_set: i16,
    sources: Vec<u8>,
    targets: Vec<u8>,
    num_target_channels: u8,
    front_angle: f32,
    rear_angle: f32,
    normalization: f32,
    voice: Option<VoiceId>,
    send_gain: i32,
    main_level: i32,
    centre_level: i32,
    lfe_level: i32,
    angle: i32,
    distance: i32,
    pan_size: i32,
    pan_twist: i32,
}

impl Default for ChannelSet {
    /// `AllocateVoiceData` 0x829be2e8's channel set.
    fn default() -> Self {
        Self {
            channel_set: -1,
            sources: Vec::new(),
            targets: Vec::new(),
            num_target_channels: 0,
            front_angle: 45.0,
            rear_angle: 135.0,
            normalization: 0.0,
            voice: None,
            send_gain: 127,
            main_level: 127,
            centre_level: 127,
            lfe_level: 127,
            angle: 0,
            distance: 0x4000,
            pan_size: 0,
            pan_twist: 0,
        }
    }
}

impl ChannelSet {
    /// `Pan2D1ChannelSet::Reset` 0x829b9230 (`channelset` keeps its value).
    fn reset(&mut self) {
        self.sources.clear();
        self.targets.clear();
        self.lfe_level = 127;
        self.centre_level = 127;
        self.main_level = 127;
        self.send_gain = 127;
        self.pan_twist = 0;
        self.pan_size = 0;
        self.angle = 0;
        self.distance = 0x4000;
    }

    fn submix(&self) -> Option<PluginRef> {
        self.voice.map(|voice| PluginRef { voice, index: 0 })
    }

    fn pan(&self) -> Option<PluginRef> {
        self.voice.map(|voice| PluginRef { voice, index: 1 })
    }

    fn send(&self) -> Option<PluginRef> {
        self.voice.map(|voice| PluginRef { voice, index: 2 })
    }
}

/// The audio core and `Path::IPathToSnd::sndimp`'s dry submix (`PathToRWAudioCore`).
pub struct SndIo<'a> {
    pub core: &'a mut AudioCore,
    /// `GetDrySubmixPlugin`, or the system's default submix when none is set.
    pub dry_submix: PluginRef,
}

/// `Path::PathTrackRWAudioCoreStream`.
pub struct StreamTrack {
    /// `mHandle`: the stream pool's guid (-1 before `AttachStreamInstance`).
    handle: i32,
    fx_send_level: i32,
    gain: f32,
    dry_level: i32,
    volume: i32,
    max_requests: i32,
    active_requests: i32,
    update_status: bool,
    status: PathStatus,
    track_info: Option<TrackInfo>,
    max_channel_sets: i32,
    num_channel_sets: i32,
    channel_sets: Vec<ChannelSet>,
    name: String,
    path: String,
    pitch_mult: i32,
    stretch_mult: i32,
    saved_pitch_mult: i32,
    requests: Vec<QueueInfo>,
    voice: Option<VoiceId>,
    /// `pFXSendInfo[0]` of the one voice: (`fxbus`, `fxlevel`).
    fx_send: (i32, i32),
    configured: bool,
    fx_bus: i32,
    num_channels: u8,
    allow_rechannel: bool,
    plugin_player: u8,
    plugin_resample: u8,
    plugin_routes: u8,
    plugin_fx_sends: u8,
    plugin_send: u8,
    /// `mCacheMem`/`mCacheSize`: the stream file's first `maxaram` bytes.
    cache: Option<Arc<[u8]>>,
    cache_size: i32,
    /// `mFileOp`: a `LoadFile` still to be collected by `CheckStatus`.
    file_op: Option<Option<Arc<[u8]>>>,
}

impl StreamTrack {
    /// `PathToRWAudioCore::CreateStreamTrack` 0x829c2cb0: the constructor
    /// (`PathTrackRWAudioCore(maxrequests, 1, 1, 10)`), then, with a buffer time, a one-stream
    /// `StreamPool` of 0x40000 bytes (guid `'pth0'` and up) and `AttachStreamInstance`.
    #[must_use]
    pub fn new(max_requests: i32, pool_guid: Option<u32>) -> Self {
        let max_requests = max_requests.max(1);
        Self {
            handle: pool_guid.map_or(-1, |guid| guid as i32),
            fx_send_level: 0,
            gain: 0.0,
            dry_level: 127,
            volume: 127,
            max_requests,
            active_requests: 0,
            update_status: true,
            status: PathStatus { time_remaining: -1, ..PathStatus::default() },
            track_info: None,
            max_channel_sets: 10,
            num_channel_sets: 0,
            channel_sets: Vec::new(),
            name: String::new(),
            path: String::new(),
            pitch_mult: 4096,
            stretch_mult: 4096,
            saved_pitch_mult: 0,
            requests: vec![QueueInfo::FREE; max_requests as usize],
            voice: None,
            fx_send: (-1, -1),
            configured: false,
            fx_bus: 0,
            num_channels: 6,
            allow_rechannel: true,
            plugin_player: 0,
            plugin_resample: 0,
            plugin_routes: 0,
            plugin_fx_sends: 0,
            plugin_send: 0,
            cache: None,
            cache_size: 0,
            file_op: None,
        }
    }

    pub fn set_track_info(&mut self, info: Option<TrackInfo>) {
        self.track_info = info;
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.chars().take(15).collect();
    }

    pub fn set_file_path(&mut self, path: &str) {
        self.path = path.to_owned();
    }

    /// `SetNumChannels` 0x829b9638 (`MusicCore::CreateStreamTrack` sets 6).
    pub fn set_num_channels(&mut self, channels: u8) {
        self.num_channels = channels;
    }

    /// `UpdateStatus` 0x829b9628.
    pub fn mark_update(&mut self) {
        self.update_status = true;
    }

    #[must_use]
    pub fn gain(&self) -> f32 {
        self.gain
    }

    #[must_use]
    pub fn volume(&self) -> i32 {
        self.volume
    }

    #[must_use]
    pub fn dry_level(&self) -> i32 {
        self.dry_level
    }

    #[must_use]
    pub fn fx_send_level(&self) -> i32 {
        self.fx_send_level
    }

    #[must_use]
    pub fn pitch_mult(&self) -> i32 {
        self.pitch_mult
    }

    #[must_use]
    pub fn stretch_mult(&self) -> i32 {
        self.stretch_mult
    }

    /// `GetMaxChannelSets`.
    #[must_use]
    pub fn max_channel_sets(&self) -> i32 {
        self.max_channel_sets
    }

    /// `StreamCache` 0x829ba3a8: a given cache, or with none and no file operation a
    /// `LoadFile` of the stream file's first `cachesize` bytes, collected by `CheckStatus`.
    pub fn stream_cache(&mut self, cache: Option<Arc<[u8]>>, cache_size: i32, io: &SndIo<'_>) {
        let mut size = cache_size;
        let mut cache = cache;
        if cache_size >= 0 {
            if cache.is_none() && self.file_op.is_none() {
                self.file_op = Some(self.load_file(cache_size, io));
            }
        } else {
            size = 0;
        }
        if self.file_op.is_some() {
            cache = None;
        }
        self.cache = cache;
        self.cache_size = size;
    }

    fn load_file(&self, size: i32, io: &SndIo<'_>) -> Option<Arc<[u8]>> {
        let files = io.core.stream_pools.files()?;
        let path = self.path.replace('/', "\\");
        let mut bytes = vec![0u8; size.max(0) as usize];
        files.read_at(&path, 0, &mut bytes).then(|| Arc::from(bytes))
    }

    /// `GetStreamCache` 0x829ba450.
    #[must_use]
    pub fn take_stream_cache(&mut self) -> (Option<Arc<[u8]>>, i32) {
        (self.cache.clone(), self.cache_size)
    }

    fn alloc_channel_sets(&mut self) {
        if self.channel_sets.is_empty() {
            self.channel_sets = vec![ChannelSet::default(); self.max_channel_sets as usize];
        }
    }

    /// `AddChannelSet` 0x829b97d8.
    pub fn add_channel_set(&mut self, set: usize, num_channels: u8, front: f32, rear: f32, normalization: f32) {
        self.alloc_channel_sets();
        let channel_set = &mut self.channel_sets[set];
        channel_set.num_target_channels = num_channels;
        channel_set.front_angle = front;
        channel_set.rear_angle = rear;
        channel_set.normalization = normalization;
    }

    /// `RemoveChannelSet` 0x829b96c0: every route disconnected, then the set reset.
    pub fn remove_channel_set(&mut self, set: usize, io: &mut SndIo<'_>) {
        if self.channel_sets.is_empty() {
            return;
        }
        if self.channel_sets[set].channel_set >= 0 {
            self.num_channel_sets -= 1;
        }
        if self.voice.is_some() {
            self.route_all(io, false);
        }
        self.channel_sets[set].reset();
    }

    /// `RouteChannels(channelset, ...)` 0x829be930: the (source, target) pairs, the set's voice
    /// when it has none, then every route connected and its send pointed at the dry submix.
    pub fn route_channels(&mut self, set: usize, pairs: &[(u8, u8)], io: &mut SndIo<'_>) {
        self.alloc_channel_sets();
        if self.channel_sets[set].channel_set < 0 {
            self.num_channel_sets += 1;
        }
        let channel_set = &mut self.channel_sets[set];
        channel_set.channel_set = pairs.first().map_or(-1, |&(source, _)| i16::from(source));
        channel_set.sources = pairs.iter().map(|&(source, _)| source).collect();
        channel_set.targets = pairs.iter().map(|&(_, target)| target).collect();
        if channel_set.voice.is_none() {
            self.setup_channel_voice(set, io);
        }
        if self.voice.is_some() {
            self.route_all(io, true);
            if let Some(send) = self.channel_sets[set].send() {
                io.core.connect_send(send, Some(io.dry_submix));
            }
        }
    }

    /// `SetupChannelVoice` 0x829beb28: stage 1 `SubMix` (the set's channels), `Pan2D1` (front,
    /// rear, normalization; channels rounded up to even) and `Send` to the dry submix.
    fn setup_channel_voice(&mut self, set: usize, io: &mut SndIo<'_>) {
        let channel_set = &self.channel_sets[set];
        let channels = channel_set.num_target_channels;
        let even = channels + channels % 2;
        let voice = io.core.create_voice(
            1,
            "PathChannelSet",
            &[
                PlugInConfig { kind: PlugInKind::SubMix { name: String::new() }, output_channels: channels },
                PlugInConfig {
                    kind: PlugInKind::Pan2D1 { constructor: Some((channel_set.front_angle, channel_set.rear_angle, channel_set.normalization)) },
                    output_channels: even,
                },
                PlugInConfig { kind: PlugInKind::Send, output_channels: even },
            ],
        );
        self.channel_sets[set].voice = voice;
        if let Some(send) = self.channel_sets[set].send() {
            io.core.connect_send(send, Some(io.dry_submix));
        }
    }

    /// `RouteChannels(VoiceData&, RouteSwitch)` 0x829beff8: each set's runs of consecutive
    /// (source, target) pairs take the next `Route` plug-in, connected to the set's `SubMix`
    /// (`ROUTE_CONNECT`) or to nothing.
    fn route_all(&mut self, io: &mut SndIo<'_>, connect: bool) {
        let Some(voice) = self.voice else { return };
        let mut route = self.plugin_routes;
        for channel_set in &self.channel_sets {
            let target = if connect { channel_set.submix() } else { None };
            let count = channel_set.sources.len();
            let mut i = 0;
            while i < count {
                let (source, dest) = (channel_set.sources[i], channel_set.targets[i]);
                let mut run = 1;
                while i + run < count && channel_set.sources[i + run] == channel_set.sources[i + run - 1] + 1 && channel_set.targets[i + run] == channel_set.targets[i + run - 1] + 1 {
                    run += 1;
                }
                io.core.connect_route(PluginRef { voice, index: route }, target, f32::from(source), f32::from(dest), run as f32);
                route += 1;
                i += run;
            }
        }
    }

    /// `SetupPluginsConfig` 0x829be510.
    fn setup_plugins_config(&mut self) -> Vec<PlugInConfig> {
        let channels = self.num_channels;
        let mut configs = Vec::new();
        self.plugin_player = configs.len() as u8;
        configs.push(PlugInConfig { kind: PlugInKind::SndPlayer1 { max_requests: self.max_requests as f32 }, output_channels: channels });
        if self.allow_rechannel {
            configs.push(PlugInConfig { kind: PlugInKind::Rechannel, output_channels: channels });
        }
        self.plugin_resample = configs.len() as u8;
        configs.push(PlugInConfig { kind: PlugInKind::Resample, output_channels: channels });
        self.plugin_routes = configs.len() as u8;
        for _ in 0..self.max_channel_sets {
            configs.push(PlugInConfig { kind: PlugInKind::Route, output_channels: channels });
        }
        let send_channels = channels.min(6);
        self.plugin_fx_sends = configs.len() as u8;
        configs.push(PlugInConfig { kind: PlugInKind::Send, output_channels: send_channels });
        self.plugin_send = configs.len() as u8;
        configs.push(PlugInConfig { kind: PlugInKind::Send, output_channels: send_channels });
        self.configured = true;
        configs
    }

    /// `CreateVoice` 0x829bee30: the stage 0 voice, its dry send, then the volume, pitch and FX
    /// level re-applied and the routes connected.
    fn create_voice(&mut self, io: &mut SndIo<'_>) {
        let configs = self.setup_plugins_config();
        self.fx_send = (-1, -1);
        self.voice = io.core.create_voice(0, "PathTrack", &configs);
        let Some(voice) = self.voice else { return };
        io.core.connect_send(PluginRef { voice, index: self.plugin_send }, Some(io.dry_submix));
        let volume = self.volume;
        self.set_volume(i32::from(volume <= 0), io);
        self.set_volume(volume, io);
        let pitch = self.pitch_mult;
        self.set_pitch_mult(if pitch <= 0 { 1 } else { pitch - 1 }, io);
        self.set_pitch_mult(pitch, io);
        self.set_fx_send_level(self.fx_bus, self.fx_send_level, io);
        self.route_all(io, true);
    }

    /// `ReleaseExpelledVoices` 0x829bed48.
    fn release_expelled(&mut self, io: &mut SndIo<'_>) {
        if let Some(voice) = self.voice {
            if io.core.voice_state(voice) == Some(VoiceState::Expelled) {
                io.core.release_voice(voice);
                self.voice = None;
                self.fx_send = (-1, -1);
            }
        }
    }

    /// `GetVoice` 0x829c0cb0.
    fn get_voice(&mut self, create: bool, io: &mut SndIo<'_>) -> Option<VoiceId> {
        self.alloc_channel_sets();
        self.release_expelled(io);
        if self.voice.is_none() && create {
            self.create_voice(io);
        }
        self.voice
    }

    /// `CheckStatus` 0x829c0e90 (the `LoadFile` collected) and 0x829bf418: each queued request's
    /// time from the player's current handle (attribute 0), position and length (1 and 2).
    pub fn check_status(&mut self, io: &mut SndIo<'_>) {
        if let Some(loaded) = self.file_op.take() {
            self.cache = loaded;
        }
        self.release_expelled(io);
        self.status = PathStatus { current_request: -1, current_node: -1, playing_node: -1, ready_for_request: 1, ..PathStatus::default() };
        let Some(voice) = self.voice else { return };
        self.update_status = true;
        self.active_requests = 0;
        let player = PluginRef { voice, index: self.plugin_player };
        let current = io.core.attribute_f64(player, 0) as f32;
        let position = io.core.attribute_f64(player, 1);
        let length = io.core.attribute_f64(player, 2);
        for slot in 0..self.max_requests as usize {
            self.requests[slot].time_remaining = 0;
            self.active_requests += 1;
            let handle = self.requests[slot].request_handle;
            if handle == -1.0 {
                break;
            }
            if handle >= current {
                if handle == current {
                    self.requests[slot].time_remaining = ((length - position) * 1000.0) as i32;
                } else {
                    self.requests[slot].time_remaining = self.requests[slot].duration;
                }
            }
            let request = self.requests[slot];
            if request.time_remaining <= 0 {
                self.requests[slot] = QueueInfo::FREE;
                break;
            }
            self.status.time_left_in_node = request.time_remaining as u32;
            self.status.time_buffered = 0;
            self.status.time_remaining += request.time_remaining;
            if request.node >= 0 {
                self.status.playing_node = request.node;
                self.status.playing_beat = request.beat;
            }
        }
        let first = self.requests[0];
        self.status.current_node = first.node;
        self.status.current_request = first.request_handle.to_bits() as i32;
        self.status.node_duration = first.duration as u32;
        if self.active_requests >= self.max_requests {
            self.status.ready_for_request = 0;
        }
        self.update_status = false;
    }

    /// `GetPathStatus` 0x829bf7a0: the status with bar and beat times from the playing node
    /// (`bars`, `beats`), scaled by the pitch and stretch multipliers.
    pub fn path_status(&mut self, node: impl Fn(i32) -> Option<(u32, u32)>, io: &mut SndIo<'_>) -> PathStatus {
        if self.update_status {
            self.check_status(io);
        }
        let status = &mut self.status;
        let mut beat = status.node_duration;
        let mut bar = beat;
        if status.playing_node >= 0 {
            if let Some((bars, beats)) = node(status.playing_node) {
                if bars != 0 {
                    status.bar_duration = status.node_duration / bars;
                    bar = beat / bars;
                    if beats != 0 {
                        status.beat_duration = status.node_duration / (beats * bars);
                        beat /= beats * bars;
                    }
                }
            }
        }
        status.playing_beat = if status.beat_duration != 0 { ((status.node_duration.wrapping_sub(status.time_left_in_node)) / status.beat_duration + 1) as i32 } else { 0 };
        status.time_left_in_bar = if bar as i32 > 0 { (status.time_left_in_node + 1) % bar } else { 0 };
        status.time_left_in_beat = if beat as i32 > 0 { (status.time_left_in_node + 1) % beat } else { 0 };
        if self.pitch_mult != 4096 || self.stretch_mult != 4096 {
            let pitch = if self.pitch_mult != 0 { 4096.0 / f64::from(self.pitch_mult) } else { 0.0 };
            let stretch = if self.stretch_mult != 0 { f64::from(self.stretch_mult) * 0.000_244_140_625 } else { 0.0 };
            let scale = stretch * pitch;
            if scale == 0.0 {
                for value in [&mut status.beat_duration, &mut status.bar_duration, &mut status.node_duration, &mut status.time_left_in_beat, &mut status.time_left_in_bar, &mut status.time_left_in_node] {
                    *value = 0x7FFF_FFFF;
                }
                status.time_remaining = 0x7FFF_FFFF;
                status.time_buffered = 0x7FFF_FFFF;
            } else {
                let s = |v: u32| (f64::from(v) * scale) as i64 as u32;
                status.node_duration = s(status.node_duration);
                status.bar_duration = s(status.bar_duration);
                status.beat_duration = s(status.beat_duration);
                status.time_left_in_node = s(status.time_left_in_node);
                status.time_left_in_bar = s(status.time_left_in_bar);
                status.time_left_in_beat = s(status.time_left_in_beat);
                status.time_buffered = (f64::from(status.time_buffered) * scale) as i64 as i32;
                status.time_remaining = (f64::from(status.time_remaining) * scale) as i64 as i32;
            }
        }
        *status
    }

    /// `ReadyForNewRequest` 0x829b99e0.
    pub fn ready_for_new_request(&mut self, io: &mut SndIo<'_>) -> bool {
        if self.update_status {
            self.check_status(io);
        }
        self.status.ready_for_request != 0
    }

    /// `TimeRemaining` 0x829ba478.
    #[must_use]
    pub fn time_remaining(&self, request: i32) -> i32 {
        if request < 0 {
            return self.status.time_remaining;
        }
        if request >= self.max_requests {
            return -8;
        }
        let queued = self.requests[request as usize];
        if queued.request_handle < 0.0 {
            return -8;
        }
        queued.time_remaining
    }

    /// `TimeBuffered` (`mStatus.timebuffered`).
    #[must_use]
    pub fn time_buffered(&self) -> i32 {
        self.status.time_buffered
    }

    /// `Play` 0x829c0f08: the node's sample (`FindSampleOffsets` of its 128-byte offset) queued on
    /// the player (start 1 ms, the `.sns` byte offset, the `.mus` path, the cached header and the
    /// stream pool), the queue shifted down and the request recorded first. -9999 while the stream
    /// cache loads (the first call starts the load).
    pub fn play(&mut self, node: i32, offset: i32, duration: i32, io: &mut SndIo<'_>) -> i32 {
        if self.file_op.is_some() {
            return -9999;
        }
        let Some(cache) = self.cache.clone() else {
            let size = self.track_info.map_or(0, |info| info.max_aram as i32);
            self.stream_cache(None, size, io);
            return -9999;
        };
        let Some(offsets) = find_sample_offsets(&cache[..(self.cache_size.max(0) as usize).min(cache.len())], offset) else { return -1 };
        let Some(voice) = self.get_voice(true, io) else { return -13 };
        let params = PlayParams {
            start_time: 0.001,
            stream_file_offset: f64::from(offsets.sns_offset << 7),
            stream_path: Some(self.path.replace('/', "\\")),
            ram_sample: Some(DataRef { bytes: Arc::clone(&cache), offset: 16 * offsets.snr_offset as usize }),
            stream_pool: self.handle as u32,
            expel_mode: 0.0,
            ..PlayParams::default()
        };
        let handle = io.core.play(PluginRef { voice, index: self.plugin_player }, params);
        if handle >= 0.0 {
            let max = self.max_requests as usize;
            self.active_requests = self.max_requests;
            for slot in (1..max).rev() {
                self.requests[slot] = self.requests[slot - 1];
                if self.requests[slot].request_handle < 0.0 {
                    self.active_requests = slot as i32 - 1;
                }
            }
            self.requests[0] = QueueInfo { duration, request_handle: handle, time_remaining: self.requests[0].time_remaining, node, beat: -1 };
            self.status.ready_for_request = 0;
        }
        self.update_status = true;
        self.update_send_gain(io);
        handle.to_bits() as i32
    }

    /// `StopPlaying` 0x829c1250.
    pub fn stop_playing(&mut self, io: &mut SndIo<'_>) {
        if let Some(voice) = self.get_voice(false, io) {
            io.core.stop(PluginRef { voice, index: self.plugin_player });
            self.status.ready_for_request = 1;
            self.requests[0] = QueueInfo { beat: self.requests[0].beat, ..QueueInfo::FREE };
            self.update_status = true;
        }
    }

    /// `Pause` 0x829b9dd0: the pitch multiplier to 0, back to the saved one on resume.
    pub fn pause(&mut self, pause: bool, io: &mut SndIo<'_>) {
        if self.pitch_mult > 0 {
            self.saved_pitch_mult = self.pitch_mult;
        }
        let pitch = if pause { 0 } else { self.saved_pitch_mult };
        self.set_pitch_mult(pitch, io);
    }

    /// `SetGain` 0x829b9a28.
    pub fn set_gain(&mut self, gain: f32, io: &mut SndIo<'_>) -> i32 {
        if !(0.0..=1.0).contains(&gain) {
            return -8;
        }
        if self.gain != gain {
            self.gain = gain;
            self.update_send_gain(io);
        }
        0
    }

    /// `SetVolume` 0x829b9b18.
    pub fn set_volume(&mut self, volume: i32, io: &mut SndIo<'_>) -> i32 {
        if !(0..0x80).contains(&volume) {
            return -8;
        }
        if self.volume != volume {
            if self.volume < 0 {
                return -1;
            }
            self.volume = volume;
            self.update_send_gain(io);
        }
        0
    }

    /// `SetDryLevel` 0x829b9c00.
    pub fn set_dry_level(&mut self, level: i32, io: &mut SndIo<'_>) -> i32 {
        if !(0..0x80).contains(&level) {
            return -8;
        }
        if self.dry_level != level {
            if self.dry_level < 0 {
                return -1;
            }
            self.dry_level = level;
            self.update_send_gain(io);
        }
        0
    }

    /// `SetPitchMult` 0x829c0288: the `Resample` rate (`pitch / 4096`).
    pub fn set_pitch_mult(&mut self, pitch: i32, io: &mut SndIo<'_>) -> i32 {
        if !(0..=0x4000).contains(&pitch) {
            return -8;
        }
        if self.pitch_mult == pitch || self.pitch_mult < 0 {
            return -1;
        }
        self.pitch_mult = pitch;
        if let Some(voice) = self.voice {
            io.core.set_attribute(PluginRef { voice, index: self.plugin_resample }, 0, pitch as f32 * INV_4096);
        }
        0
    }

    /// `SetStretchMult` 0x829b9d78: built without `RWAUDIOCORE_ALLOW_TIMESTRETCH`.
    pub fn set_stretch_mult(&self, _stretch: i32) -> i32 {
        -8
    }

    /// `SetFXSendLevel` 0x829c0010. `GetFXSendIndex` 0x829b9918 finds the send on the bus; to
    /// connect a positive level `ConnectFXSubmix` 0x829bf220 takes the free send (bus -1), but no
    /// FX submix is registered (`SetFXSubmixPlugin`), so the send keeps bus -1 and no target.
    pub fn set_fx_send_level(&mut self, fx_bus: i32, level: i32, io: &mut SndIo<'_>) -> i32 {
        self.fx_send_level = level;
        self.fx_bus = fx_bus;
        let Some(voice) = self.voice else { return 0 };
        let found = self.fx_send.0 == fx_bus || (level > 0 && self.fx_send.0 == -1);
        if found {
            self.fx_send.1 = level;
            io.core.set_attribute(PluginRef { voice, index: self.plugin_fx_sends }, 0, self.gain * level as f32 * INV_127);
            if level <= 0 && self.fx_send.0 == fx_bus {
                io.core.connect_send(PluginRef { voice, index: self.plugin_fx_sends }, None);
                self.fx_send = (-1, -1);
            }
        } else if level > 0 {
            return -8;
        }
        0
    }

    /// `UpdateSendGain` 0x829bfac8: the dry send at `volume * gain * drylevel / 127 / 127`, then
    /// every set's levels re-applied with the gain.
    fn update_send_gain(&mut self, io: &mut SndIo<'_>) {
        let Some(voice) = self.voice else { return };
        let level = self.gain * self.dry_level as f32 * INV_127;
        io.core.set_attribute(PluginRef { voice, index: self.plugin_send }, 0, self.volume as f32 * level * INV_127);
        for set in 0..self.channel_sets.len() {
            let channel_set = &self.channel_sets[set];
            let (send, main, centre, lfe) = (channel_set.send_gain, channel_set.main_level, channel_set.centre_level, channel_set.lfe_level);
            self.set_channels_volume(set, send, main, centre, lfe, io);
        }
    }

    /// `GetChannelsVolume` 0x829b9ce8.
    #[must_use]
    pub fn channels_volume(&self, set: usize) -> (i32, i32, i32, i32) {
        self.channel_sets.get(set).map_or((-1, -1, -1, -1), |c| (c.send_gain, c.main_level, c.centre_level, c.lfe_level))
    }

    /// `GetChannelsPosition` 0x829b9d38.
    #[must_use]
    pub fn channels_position(&self, set: usize) -> (i32, i32, i32, i32) {
        self.channel_sets.get(set).map_or((0, 0, 0, 0), |c| (c.angle, c.distance, c.pan_size, c.pan_twist))
    }

    /// `SetChannelsVolume` 0x829bfc48: with a voice, the send at `send * gain / 127` and the
    /// `Pan2D1` main (attribute 5, from the send volume), centre (4) and LFE (6) levels.
    pub fn set_channels_volume(&mut self, set: usize, send: i32, _main: i32, centre: i32, lfe: i32, io: &mut SndIo<'_>) -> i32 {
        let gain = self.gain;
        let Some(channel_set) = self.channel_sets.get_mut(set) else { return -14 };
        let (Some(pan), Some(send_plugin)) = (channel_set.pan(), channel_set.send()) else { return 0 };
        if send >= 0 {
            let level = send as f32 * gain;
            io.core.set_attribute(send_plugin, 0, level * INV_127);
            channel_set.send_gain = send;
            io.core.set_attribute(pan, 5, level * INV_127);
            channel_set.main_level = send;
        }
        if centre >= 0 {
            io.core.set_attribute(pan, 4, centre as f32 * gain * INV_127);
            channel_set.centre_level = centre;
        }
        if lfe >= 0 {
            io.core.set_attribute(pan, 6, lfe as f32 * gain * INV_127);
            channel_set.lfe_level = lfe;
        }
        0
    }

    /// `SetChannelsPosition` 0x829bfe08: `Pan2D1` angle (0, the angle modulo 0x4000 as a share
    /// of 360), distance (1) and size (2) (per 0x4000, times 1000) and twist (3).
    pub fn set_channels_position(&mut self, set: usize, angle: i32, distance: i32, pan_size: i32, pan_twist: i32, io: &mut SndIo<'_>) -> i32 {
        let Some(channel_set) = self.channel_sets.get_mut(set) else { return -14 };
        let Some(pan) = channel_set.pan() else { return 0 };
        if angle >= 0 {
            io.core.set_attribute(pan, 0, (angle % 0x4000) as f32 * INV_16384 * 360.0);
            channel_set.angle = angle;
        }
        if distance >= 0 {
            io.core.set_attribute(pan, 1, distance as f32 * INV_16384 * 1000.0);
            channel_set.distance = distance;
        }
        if pan_size >= 0 {
            io.core.set_attribute(pan, 2, pan_size as f32 * INV_16384 * 1000.0);
            channel_set.pan_size = pan_size;
        }
        if pan_twist >= 0 {
            io.core.set_attribute(pan, 3, (pan_twist % 0x4000) as f32 * INV_16384 * 360.0);
            channel_set.pan_twist = pan_twist;
        }
        0
    }
}
