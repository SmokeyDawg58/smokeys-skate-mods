//! EA Pathfinder 5.3 (`path\5.03.00-sk8\source\cmn`): the interactive-music library the music
//! system drives. Each `.mpf` project (`PATHFINDERSTATE`, `Path::pfstates[4]`) holds up to 24
//! tracks (`PATHTRACK`) and a 16-entry event queue; events are scripts of `PATHACTION`s, and
//! tracks walk the node graph, queueing each audio node's sample on their stream implementation.
//!
//! Handles: bits 0..23 select tracks, bits 24..27 projects (`0x01000000 << projectID`) and bits
//! 28..31 voices (`0x10000000`). The locks (`PATHI_lock`) are the audio system's own.

use std::sync::Arc;

use crate::formats::pathfinder::{sbits, ubits, MapFile, NamedVar, EVENT_HEADER_SIZE, ACTION_SIZE};
use crate::music::path_track::{SndIo, StreamTrack};
use crate::rwac::core::AudioCore;
use crate::rwac::PluginRef;

/// `PATHFADEWHAT`.
pub mod fade {
    pub const INVALID: i32 = 0;
    pub const VOLUME: i32 = 1;
    pub const SFX_SEND_LEVEL: i32 = 2;
    pub const DRY_LEVEL: i32 = 3;
    pub const PITCH_MULT: i32 = 4;
    pub const STRETCH_MULT: i32 = 5;
    pub const CHANNEL_VOL: i32 = 6;
    pub const PAN_ANGLE: i32 = 7;
    pub const PAN_DISTANCE: i32 = 8;
    pub const PAN_SIZE: i32 = 9;
    pub const PAN_TWIST: i32 = 10;
}

/// `PATHTRACKPLAYSTATUS`.
pub mod track_status {
    pub const STOPPED: i32 = 1;
    pub const QUEUEING: i32 = 2;
    pub const PAUSED: i32 = 3;
    pub const READY_FOR_NEXT: i32 = 4;
    pub const PLAYING: i32 = 5;
    pub const FADING: i32 = 6;
}

/// `PATHEVENTRESULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventResult {
    Pending,
    Expired,
    Purged,
    Finished,
}

/// The size of `PATHFINDERSTATE::eventheap`.
const EVENT_HEAP_BYTES: usize = 4096;
/// `guid`: the first stream pool's (`'pth0'`).
const FIRST_POOL_GUID: u32 = 0x7074_6830;
/// `seedPATH`'s data-section value.
const SEED: [u32; 6] = [0xF22D_0E56, 0x8831_26E9, 0xC624_DD2F, 0x0702_C49C, 0x9E35_3F7D, 0x6FDF_3B64];
/// `Path::RWAudioCoreTimer::Update`'s 10 ms and `ServiceTaskThreadFunc`'s 50 ms sleep.
const TIMER_SECONDS: f64 = 0.01;
const TASK_MILLISECONDS: f32 = 50.0;
/// The id of the `"pathfinder timer"` (`RWAudioCoreTimer` 0x829c1318) among the core's
/// `TIMERSTAGE_BEFORECOMMANDS` timers.
pub const TIMER_ID: u32 = 0x5041_5448;

/// What `MusicCore::PathLibProgressCB` 0x82a80690 keeps of the song progress: `mCurrentNodeID`,
/// `mSongOver` and the `mStateReady` it clears when a song ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreFlags {
    pub current_node: i32,
    pub song_over: bool,
    pub state_ready: bool,
}

/// `PATHFADESTATS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FadeStats {
    what: i32,
    bus: i16,
    from: i16,
    to: i16,
    num: i16,
    start: u32,
    time: u32,
}

impl FadeStats {
    const CLEAR: Self = Self { what: fade::INVALID, bus: -1, from: -1, to: -1, num: -1, start: 0, time: 0 };

    fn reset(&mut self) {
        *self = Self { time: self.time, ..Self::CLEAR };
    }
}

/// `PATHTRACK`.
pub struct Track {
    latency: u16,
    volume: i8,
    track_id: u8,
    dry_level: u16,
    sfx_level: u16,
    repeat_node: i16,
    node: i16,
    main_voice: bool,
    ram_track: bool,
    paused: bool,
    repeat: i32,
    sfx_bus: u8,
    node_beat: i32,
    no_branch: bool,
    control: u32,
    gain: f32,
    fades: [FadeStats; 16],
    /// `entryinfo`: the entry node of the playing part.
    entry: Option<i32>,
    beat_time: u32,
    bar_time: u32,
    next_beat_time: u32,
    pause_at: u32,
    resume_at: u32,
    newest_request: i32,
    music_file: String,
    name: String,
    loading_subbank: i32,
    imp: Option<StreamTrack>,
    stream_cache: Option<Arc<[u8]>>,
    stream_cache_size: i32,
    status: i32,
}

/// The fields `PATHI_seeknextnode` and `PATHI_BRANCHTO` restore (`memcpy` of the `PATHTRACK`)
/// when queueing fails.
#[derive(Clone)]
struct TrackBackup {
    latency: u16,
    volume: i8,
    dry_level: u16,
    sfx_level: u16,
    repeat_node: i16,
    node: i16,
    main_voice: bool,
    paused: bool,
    repeat: i32,
    sfx_bus: u8,
    node_beat: i32,
    no_branch: bool,
    control: u32,
    gain: f32,
    fades: [FadeStats; 16],
    entry: Option<i32>,
    beat_time: u32,
    bar_time: u32,
    next_beat_time: u32,
    pause_at: u32,
    resume_at: u32,
    newest_request: i32,
    loading_subbank: i32,
    status: i32,
}

impl Track {
    fn backup(&self) -> TrackBackup {
        TrackBackup {
            latency: self.latency,
            volume: self.volume,
            dry_level: self.dry_level,
            sfx_level: self.sfx_level,
            repeat_node: self.repeat_node,
            node: self.node,
            main_voice: self.main_voice,
            paused: self.paused,
            repeat: self.repeat,
            sfx_bus: self.sfx_bus,
            node_beat: self.node_beat,
            no_branch: self.no_branch,
            control: self.control,
            gain: self.gain,
            fades: self.fades,
            entry: self.entry,
            beat_time: self.beat_time,
            bar_time: self.bar_time,
            next_beat_time: self.next_beat_time,
            pause_at: self.pause_at,
            resume_at: self.resume_at,
            newest_request: self.newest_request,
            loading_subbank: self.loading_subbank,
            status: self.status,
        }
    }

    fn restore(&mut self, b: TrackBackup) {
        self.latency = b.latency;
        self.volume = b.volume;
        self.dry_level = b.dry_level;
        self.sfx_level = b.sfx_level;
        self.repeat_node = b.repeat_node;
        self.node = b.node;
        self.main_voice = b.main_voice;
        self.paused = b.paused;
        self.repeat = b.repeat;
        self.sfx_bus = b.sfx_bus;
        self.node_beat = b.node_beat;
        self.no_branch = b.no_branch;
        self.control = b.control;
        self.gain = b.gain;
        self.fades = b.fades;
        self.entry = b.entry;
        self.beat_time = b.beat_time;
        self.bar_time = b.bar_time;
        self.next_beat_time = b.next_beat_time;
        self.pause_at = b.pause_at;
        self.resume_at = b.resume_at;
        self.newest_request = b.newest_request;
        self.loading_subbank = b.loading_subbank;
        self.status = b.status;
    }

    /// `PATHI_getfadestats` 0x829b8cb0: the fade of `what` on `bus`, else the first free one.
    fn fade_stats(&self, what: i32, bus: i16) -> Option<usize> {
        let mut free = None;
        for (index, stats) in self.fades.iter().enumerate() {
            if stats.what == what && stats.bus == bus {
                return Some(index);
            }
            if stats.what == fade::INVALID && free.is_none() {
                free = Some(index);
            }
        }
        free
    }
}

/// A queued `PATHEVENT` (a copy in `eventheap`) and its `PATHACTION`s.
#[derive(Debug, Clone)]
struct Event {
    queued: u32,
    expiry: u32,
    last_act: u32,
    /// `numactions:8`, `eventID:24`.
    w3: u32,
    /// `project:3` (11), `beingFiltered` (14), `bumplower` (15), `priority:4` (16), `voices:4`
    /// (20), `currentaction:8` (24).
    w4: u32,
    /// (`track`, flags word, `act`).
    actions: Vec<[u32; 3]>,
}

impl Event {
    fn from_template(bytes: &[u8]) -> Self {
        let word = |offset: usize| u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
        let count = usize::from(bytes[15]);
        let actions = (0..count)
            .map(|i| {
                let at = EVENT_HEADER_SIZE + ACTION_SIZE * i;
                [word(at), word(at + 4), word(at + 8)]
            })
            .collect();
        Self { queued: word(0), expiry: word(4), last_act: word(8), w3: word(12), w4: word(16), actions }
    }

    fn size(&self) -> usize {
        EVENT_HEADER_SIZE + ACTION_SIZE * self.actions.len()
    }

    fn num_actions(&self) -> usize {
        (self.w3 & 0xFF) as usize
    }

    fn current_action(&self) -> usize {
        (self.w4 >> 24) as usize
    }

    fn set_current_action(&mut self, action: usize) {
        self.w4 = (self.w4 & 0x00FF_FFFF) | ((action as u32 & 0xFF) << 24);
    }

    fn priority(&self) -> i32 {
        sbits(self.w4, 16, 4)
    }
}

fn indent(w1: u32) -> u32 {
    ubits(w1, 3, 3)
}

fn action_type(w1: u32) -> u32 {
    ubits(w1, 17, 7)
}

fn assess(w1: u32) -> u32 {
    ubits(w1, 9, 3)
}

const DONE: u32 = 0x1_0000;

/// `PATHFINDERSTATE`.
pub struct Project {
    map: MapFile,
    idflags: u32,
    named: Vec<NamedVar>,
    tracks: Vec<Option<Box<Track>>>,
    queue: Vec<Event>,
    master_track: i32,
    master_latency: u32,
    master_section: u32,
    task_interval: u32,
    timer_interval: u32,
}

impl Project {
    fn matches(&self, tracks: u32) -> bool {
        (tracks & 0xF000_0000 == 0 || self.idflags & tracks & 0xF000_0000 != 0) && self.idflags & tracks & 0x0F00_0000 != 0
    }

    fn heap_used(&self) -> usize {
        self.queue.iter().map(Event::size).sum()
    }
}

/// The node words a caller needs, copied out of the map file.
#[derive(Debug, Clone, Copy)]
struct NodeInfo {
    w: [u32; 4],
    branches: [(i8, i8, i16); 32],
}

impl NodeInfo {
    fn index(&self) -> i32 {
        sbits(self.w[0], 16, 16)
    }
    fn repeat(&self) -> i32 {
        sbits(self.w[0], 0, 5)
    }
    fn section(&self) -> u32 {
        ubits(self.w[0], 5, 6)
    }
    fn track(&self) -> u32 {
        ubits(self.w[0], 11, 5)
    }
    fn bars(&self) -> u32 {
        ubits(self.w[1], 0, 8)
    }
    fn beats(&self) -> u32 {
        ubits(self.w[1], 8, 4)
    }
    fn num_branches(&self) -> usize {
        ubits(self.w[1], 15, 5) as usize
    }
    fn part(&self) -> i32 {
        (self.w[2] >> 16) as i32
    }
}

/// `Path::` globals, the projects, `IPathToSnd::sndimp` and the two service clocks.
pub struct Path {
    projects: [Option<Project>; 4],
    current: usize,
    pub milliseconds: u32,
    paused: bool,
    master_gain: f32,
    default_fx_bus: i32,
    seed: [u32; 6],
    bank_service: bool,
    pub flags: CoreFlags,
    /// `PathToRWAudioCore::pDrySubmixPlugin`.
    pub dry_submix: Option<PluginRef>,
    /// The stream implementation while no track holds it (`PATH_detachstreamimp` to
    /// `PATH_attachstreamimp`).
    loose_imp: Option<StreamTrack>,
    next_pool_guid: u32,
    timer_last_call: f32,
    timer_this_call: f32,
    task_clock: f32,
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

impl Path {
    /// `PATHI_init` 0x829bb1c8 and the library's statics.
    #[must_use]
    pub fn new() -> Self {
        Self {
            projects: [None, None, None, None],
            current: 0,
            milliseconds: 0,
            paused: false,
            master_gain: 1.0,
            default_fx_bus: 0,
            seed: SEED,
            bank_service: false,
            flags: CoreFlags { current_node: -1, song_over: false, state_ready: false },
            dry_submix: None,
            loose_imp: None,
            next_pool_guid: FIRST_POOL_GUID,
            timer_last_call: 0.0,
            timer_this_call: 0.0,
            task_clock: 0.0,
        }
    }

    fn project(&self) -> &Project {
        self.projects[self.current].as_ref().expect("current project")
    }

    fn project_mut(&mut self) -> &mut Project {
        self.projects[self.current].as_mut().expect("current project")
    }

    /// `PATH_seedrandom` 0x829b8428.
    pub fn seed_random(&mut self, game_seed: u32) {
        let fifth = game_seed / 5;
        self.seed[0] = game_seed.wrapping_sub(231_931_904).wrapping_add(3670);
        self.seed[1] = game_seed.wrapping_sub(2_010_050_986).wrapping_add(6291);
        self.seed[2] = game_seed.wrapping_sub(2_010_050_986).wrapping_add(1_039_388_377);
        self.seed[3] = game_seed.wrapping_sub(2_010_050_986).wrapping_add(2_127_672_902);
        self.seed[4] = game_seed.wrapping_sub(2_010_050_986).wrapping_add(369_373_479);
        self.seed[5] = game_seed.wrapping_add(1_876_900_708);
        let rounded = 32u32.wrapping_mul(game_seed / 0xA0 + u32::from(fifth & 0x8000_0000 != 0 && fifth & 0x1F != 0));
        let count = 32i32.wrapping_sub(fifth.wrapping_sub(rounded) as i32);
        for _ in 0..count.max(0) {
            self.random();
        }
    }

    /// `PATHI_random` 0x829b8348: a 192-bit add-with-carry generator.
    fn random(&mut self) -> u32 {
        let s = &mut self.seed;
        let s5 = s[5];
        let v1 = s[4].wrapping_add(s5);
        let carry = u32::from(v1 < s5 || v1 < s[4]);
        let v3 = v1;
        let v4 = s[2];
        s[4] = v1;
        let v5 = s[3].wrapping_add(v1).wrapping_add(carry);
        s[2] = s[2].wrapping_add(v5).wrapping_add(u32::from(v5 < s[3]));
        let v6 = s[2];
        s[3] = v5;
        let v7 = s[1].wrapping_add(v6).wrapping_add(u32::from(v6 < v4));
        let carry1 = u32::from(v7 < s[1]);
        s[1] = v7;
        s[5] = s5.wrapping_add(1);
        s[0] = s[0].wrapping_add(v7).wrapping_add(carry1);
        let result = s[0];
        if s5 == u32::MAX {
            s[4] = v3.wrapping_add(1);
            if v3 == u32::MAX {
                s[3] = v5.wrapping_add(1);
                if v5 == u32::MAX {
                    s[2] = s[2].wrapping_add(1);
                    if v6 == u32::MAX {
                        s[1] = v7.wrapping_add(1);
                        if v7 == u32::MAX {
                            s[0] = s[0].wrapping_add(1);
                            return s[0];
                        }
                    }
                }
            }
        }
        result
    }

    /// `PATH_addmapfile` 0x829bb2b0: a free project slot for the map (a project with the same
    /// `projectID` and `generateID` counts as another voice), `idflags` of the voice and project
    /// bits, the named variables, 50 ms task and 10 ms timer intervals. Returns the project id.
    pub fn add_map_file(&mut self, map: MapFile) -> i32 {
        let project_id = u32::from(map.project_id());
        let mut voice = 0u32;
        let mut free = None;
        for (slot, project) in self.projects.iter().enumerate() {
            match project {
                None => {
                    if free.is_none() {
                        free = Some(slot);
                    }
                }
                Some(project) => {
                    if u32::from(project.map.project_id()) == project_id {
                        if map.generate_id() == project.map.generate_id() {
                            voice += 1;
                        } else {
                            return -1;
                        }
                    }
                }
            }
        }
        let Some(slot) = free else { return -1 };
        let idflags = (0x1000_0000u32 << voice) | (0x0100_0000 << project_id);
        let named = map.named_vars();
        self.projects[slot] = Some(Project {
            map,
            idflags,
            named,
            tracks: (0..24).map(|_| None).collect(),
            queue: Vec::new(),
            master_track: 0,
            master_latency: 0,
            master_section: 0,
            task_interval: 50,
            timer_interval: 10,
        });
        project_id as i32
    }

    /// `PATHI_switchproject` 0x829b84e0.
    fn switch_project(&mut self, slot: usize, idflags: u32) -> bool {
        let Some(project) = self.projects.get(slot).and_then(Option::as_ref) else { return false };
        if !project.matches(idflags) {
            return false;
        }
        self.current = slot;
        true
    }

    /// `PATHI_switchvoice` 0x829b8560.
    fn switch_voice(&mut self, voice_flags: u32) -> bool {
        let has_project = voice_flags & 0x0F00_0000 != 0;
        for slot in 0..4 {
            let flags = if has_project { voice_flags } else { voice_flags | (0x0100_0000 << slot) };
            if self.projects[slot].as_ref().is_some_and(|p| p.matches(flags) && p.idflags != 0) {
                self.current = slot;
                return true;
            }
        }
        false
    }

    /// `PATHI_gettrackptr` 0x829b8640: (project, track) of the lowest track bit.
    fn track_ptr(&self, handle: u32) -> Option<(usize, usize)> {
        for (slot, project) in self.projects.iter().enumerate() {
            let Some(project) = project else { continue };
            if handle & 0xF000_0000 & project.idflags == 0 || handle & 0x0F00_0000 & project.idflags == 0 {
                continue;
            }
            let bit = (0..24).find(|&bit| (1u32 << bit) & handle & 0x00FF_FFFF != 0)?;
            return project.tracks[bit].is_some().then_some((slot, bit));
        }
        None
    }

    fn track(&self, slot: usize, index: usize) -> &Track {
        self.projects[slot].as_ref().expect("project").tracks[index].as_deref().expect("track")
    }

    fn track_mut(&mut self, slot: usize, index: usize) -> &mut Track {
        self.projects[slot].as_mut().expect("project").tracks[index].as_deref_mut().expect("track")
    }

    fn node(&self, node: i32) -> Option<NodeInfo> {
        let n = self.project().map.node(node)?;
        let mut branches = [(0, 0, 0); 32];
        for (i, branch) in branches.iter_mut().enumerate().take(n.num_branches() as usize) {
            *branch = n.branch(i);
        }
        Some(NodeInfo { w: [n.word(0), n.word(1), n.word(2), n.word(3)], branches })
    }

    /// `PATH_createstreamtrack` 0x829c3a78 (`PATHI_createtrack` 0x829c2950, `PATHI_inittrack`
    /// 0x829bca30): a track for the handle's lowest track bit with the mus file, `latency` ms.
    pub fn create_stream_track(&mut self, handle: u32, mus_file: &str, latency: i32) -> i32 {
        if !self.switch_voice(handle) {
            return -8;
        }
        if handle & 0x00FF_FFFF == 0 || handle & 0x0F00_0000 == 0 || handle & 0xF000_0000 == 0 {
            return -8;
        }
        let bit = (0..24).find(|&bit| handle & (1 << bit) != 0).unwrap_or(24);
        if self.project().tracks.get(bit).is_some_and(Option::is_some) {
            return -13;
        }
        let voice = handle & 0xFF00_0000;
        let voice_index = (0..4).find(|&i| (1u32 << (28 + i)) & voice != 0).unwrap_or(4);
        let base = mus_file.rsplit(['\\', '/', ':']).next().unwrap_or(mus_file);
        let text = format!("{:x}{base}", voice_index + 1);
        let mut name = [0u8; 32];
        let len = text.len().min(32);
        name[..len].copy_from_slice(&text.as_bytes()[..len]);
        let mut c = b'a';
        for i in 1..32 {
            c = if c != 0 && name[i] != 0 { name[i] | 0x20 } else { 0 };
            if !(c.is_ascii_lowercase() || c.is_ascii_digit()) {
                c = 0;
                if i >= 12 {
                    name[i] = 0;
                    break;
                }
                name[i] = b' ';
            }
        }
        name[11] = 0;
        let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(32)];
        let track = Track {
            latency: latency as u16,
            volume: 127,
            track_id: bit as u8,
            dry_level: 0,
            sfx_level: 0,
            repeat_node: -1,
            node: -1,
            main_voice: voice & 0xF000_0000 == 0x1000_0000,
            ram_track: false,
            paused: false,
            repeat: 0,
            sfx_bus: (self.default_fx_bus & 7) as u8,
            node_beat: -1,
            no_branch: false,
            control: 0,
            gain: self.master_gain,
            fades: [FadeStats::CLEAR; 16],
            entry: None,
            beat_time: 0,
            bar_time: 0,
            next_beat_time: 0,
            pause_at: 0,
            resume_at: 0,
            newest_request: -1,
            music_file: mus_file.to_owned(),
            name: String::from_utf8_lossy(name).into_owned(),
            loading_subbank: -1,
            imp: None,
            stream_cache: None,
            stream_cache_size: 0,
            status: track_status::STOPPED,
        };
        self.project_mut().tracks[bit] = Some(Box::new(track));
        0
    }

    /// `PATH_createstreamimp` 0x829c2948 (`PATH_createstreamimplementation` 0x829bc550 and
    /// `PathToRWAudioCore::CreateStreamTrack` 0x829c2cb0): the stream implementation of
    /// `maxrequests` queued nodes and its stream pool, attached to the track.
    pub fn create_stream_imp(&mut self, handle: u32, max_requests: i32, buffer_time: f32, io: &mut SndIo<'_>) -> bool {
        if !self.switch_voice(handle) {
            return false;
        }
        let Some((slot, index)) = self.track_ptr(handle) else { return false };
        let guid = (buffer_time != 0.0).then(|| {
            let guid = self.next_pool_guid;
            io.core.stream_pools.create(guid, 1, 0x4_0000);
            self.next_pool_guid = self.next_pool_guid.wrapping_add(1);
            guid
        });
        let mut imp = StreamTrack::new(max_requests, guid);
        let info = self.project().map.track_info(self.track(slot, index).track_id);
        let track = self.track_mut(slot, index);
        imp.set_name(&track.name);
        imp.set_file_path(&track.music_file);
        imp.set_track_info(info);
        imp.set_gain(track.gain, io);
        imp.set_volume(i32::from(track.volume), io);
        if track.stream_cache.is_some() {
            imp.stream_cache(track.stream_cache.clone(), track.stream_cache_size, io);
        }
        track.imp = Some(imp);
        true
    }

    /// `PATH_attachstreamimp` 0x829bc730.
    pub fn attach_stream_imp(&mut self, handle: u32, io: &mut SndIo<'_>) -> i32 {
        if !self.switch_voice(handle) {
            return -8;
        }
        let Some((slot, index)) = self.track_ptr(handle) else { return -8 };
        let Some(mut imp) = self.loose_imp.take() else {
            self.track_mut(slot, index).imp = None;
            return -8;
        };
        let Some(info) = self.project().map.track_info(self.track(slot, index).track_id) else {
            self.track_mut(slot, index).imp = Some(imp);
            return -8;
        };
        let track = self.track_mut(slot, index);
        imp.set_name(&track.name);
        imp.set_file_path(&track.music_file);
        imp.set_track_info(Some(info));
        imp.set_gain(track.gain, io);
        imp.set_volume(i32::from(track.volume), io);
        if track.stream_cache.is_some() {
            imp.stream_cache(track.stream_cache.clone(), track.stream_cache_size, io);
        }
        track.imp = Some(imp);
        0
    }

    /// `PATH_detachstreamimp` 0x829bc920: the implementation leaves the track, which keeps its
    /// stream cache.
    pub fn detach_stream_imp(&mut self, handle: u32, io: &mut SndIo<'_>) {
        let Some((slot, index)) = self.track_ptr(handle) else { return };
        let track = self.track_mut(slot, index);
        if let Some(mut imp) = track.imp.take() {
            let (cache, size) = imp.take_stream_cache();
            track.stream_cache = cache;
            track.stream_cache_size = size;
            imp.stream_cache(None, -1, io);
            imp.mark_update();
            self.loose_imp = Some(imp);
        }
    }

    /// `PATH_cachestreamdata` 0x829bb5f8 with no memory: the implementation caches the
    /// track's `maxaram` bytes.
    pub fn cache_stream_data(&mut self, handle: u32, io: &mut SndIo<'_>) -> i32 {
        let Some((slot, index)) = self.track_ptr(handle) else { return -8 };
        if self.track(slot, index).imp.is_none() {
            return -8;
        }
        let track_id = self.track(slot, index).track_id;
        let size = (0..4).find_map(|s| self.projects[s].as_ref().filter(|p| p.matches(handle)).and_then(|p| p.map.track_info(track_id))).map_or(0, |info| info.max_aram as i32);
        let track = self.track_mut(slot, index);
        track.stream_cache = None;
        track.stream_cache_size = size;
        if let Some(imp) = track.imp.as_mut() {
            imp.stream_cache(None, size, io);
        }
        0
    }

    /// The stream implementation wherever it is (`MusicCore::mpStreamImp`).
    pub fn stream_imp(&mut self) -> Option<&mut StreamTrack> {
        if self.loose_imp.is_some() {
            return self.loose_imp.as_mut();
        }
        self.projects.iter_mut().flatten().flat_map(|p| p.tracks.iter_mut().flatten()).find_map(|t| t.imp.as_mut())
    }

    #[must_use]
    pub fn has_stream_imp(&self) -> bool {
        self.loose_imp.is_some() || self.projects.iter().flatten().flat_map(|p| p.tracks.iter().flatten()).any(|t| t.imp.is_some())
    }

    /// The tracks selected by `tracks` in each matching project, as (project, track) pairs.
    fn selected(&self, tracks: u32) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (slot, project) in self.projects.iter().enumerate() {
            let Some(project) = project else { continue };
            if !project.matches(tracks) || project.idflags == 0 {
                continue;
            }
            for index in 0..24 {
                if project.tracks[index].is_some() && tracks & (1 << index) != 0 {
                    out.push((slot, index));
                }
            }
        }
        out
    }

    /// `PATH_gain` 0x829bce20.
    pub fn gain(&mut self, tracks: u32, gain: f32, io: &mut SndIo<'_>) -> i32 {
        let gain = if gain > 1.0 { 1.0 } else if gain < 0.0 { 0.0 } else { gain };
        if tracks == u32::MAX {
            self.master_gain = gain;
        }
        let mut result = -8;
        for (slot, index) in self.selected(tracks) {
            self.current = slot;
            let track = self.track_mut(slot, index);
            track.gain = gain;
            let fading = track.fade_stats(fade::VOLUME, -1).is_some_and(|f| track.fades[f].what != fade::INVALID);
            let volume = i32::from(track.volume);
            if let Some(imp) = track.imp.as_mut() {
                imp.set_gain(gain, io);
                if !fading {
                    imp.set_volume(volume, io);
                }
            }
            result = 0;
        }
        result
    }

    /// `PATH_setvolume` 0x829bd030.
    pub fn set_volume(&mut self, tracks: u32, volume: i8, io: &mut SndIo<'_>) -> i32 {
        let volume = volume.max(0);
        let mut result = -8;
        for (slot, index) in self.selected(tracks) {
            self.current = slot;
            let track = self.track_mut(slot, index);
            track.volume = volume;
            let fading = track.fade_stats(fade::VOLUME, -1).is_some_and(|f| track.fades[f].what != fade::INVALID);
            if let Some(imp) = track.imp.as_mut() {
                if !fading {
                    imp.set_volume(i32::from(volume), io);
                }
            }
            result = 0;
        }
        result
    }

    /// `PATH_pitchmult` 0x829bd1f8.
    pub fn pitch_mult(&mut self, tracks: u32, pitch: i32, io: &mut SndIo<'_>) -> i32 {
        let mut result = -8;
        for (slot, index) in self.selected(tracks) {
            if let Some(imp) = self.track_mut(slot, index).imp.as_mut() {
                result = imp.set_pitch_mult(pitch, io);
            }
        }
        result
    }

    /// `PATH_channelsvolume` 0x829bd388 (`SetChannelsVolume(set, volume, -1, -1, -1)`).
    pub fn channels_volume(&mut self, tracks: u32, set: usize, volume: i32, io: &mut SndIo<'_>) -> i32 {
        let mut result = -8;
        for (slot, index) in self.selected(tracks) {
            if let Some(imp) = self.track_mut(slot, index).imp.as_mut() {
                result = imp.set_channels_volume(set, volume, -1, -1, -1, io);
            }
        }
        result
    }

    /// `PATH_pause` 0x829c1d58.
    pub fn pause(&mut self, tracks: u32, pause: bool, io: &mut SndIo<'_>) -> i32 {
        if tracks == u32::MAX {
            self.paused = pause;
        }
        let mut result = -8;
        for (slot, index) in self.selected(tracks) {
            let track = self.track_mut(slot, index);
            if let Some(imp) = track.imp.as_mut() {
                imp.pause(pause, io);
            }
            track.paused = pause;
            result = 0;
        }
        result
    }

    /// `PATH_stop` 0x829c1ef8.
    pub fn stop(&mut self, tracks: u32, io: &mut SndIo<'_>) -> i32 {
        let mut result = -8;
        for (slot, index) in self.selected(tracks) {
            self.current = slot;
            result = self.stop_track(slot, index, io);
        }
        result
    }

    /// `PATHI_stop` 0x829bac80.
    fn stop_track(&mut self, slot: usize, index: usize, io: &mut SndIo<'_>) -> i32 {
        let track = self.track_mut(slot, index);
        if let Some(imp) = track.imp.as_mut() {
            imp.stop_playing(io);
        }
        let was_playing = track.node >= 0;
        track.node = -1;
        track.node_beat = -1;
        if was_playing {
            self.song_progress(-1, -1, -1);
        }
        0
    }

    /// `PATH_latency` 0x829bac08.
    pub fn latency(&mut self, handle: u32, latency: i16) -> i32 {
        let Some((slot, index)) = self.track_ptr(handle) else { return -8 };
        self.track_mut(slot, index).latency = latency as u16;
        0
    }

    /// `PATH_control` 0x829c1c18: the control value of the tracks; a playing channel-branching
    /// node switches its channel sets at once.
    pub fn control(&mut self, tracks: u32, controller: u32, io: &mut SndIo<'_>) -> i32 {
        if controller > 0x7F {
            return -8;
        }
        let mut result = -8;
        for slot in 0..4 {
            if !self.switch_project(slot, tracks) {
                continue;
            }
            for index in 0..24 {
                if tracks & (1 << index) == 0 || self.project().tracks[index].is_none() {
                    continue;
                }
                self.track_mut(slot, index).control = controller;
                let node = i32::from(self.track(slot, index).node);
                if node >= 0 && self.node(node).is_some_and(|n| n.w[2] & 2 != 0) {
                    self.channel_branch(slot, index, io);
                }
                result = 0;
            }
        }
        result
    }

    /// `PATH_pitchmultfade` 0x829c2bb8.
    pub fn pitch_mult_fade(&mut self, tracks: u32, pitch: i32, time: u32, curve: i32) -> i32 {
        self.ramp(tracks, -1, pitch, time, curve, fade::PITCH_MULT)
    }

    /// `PATH_channelsvolumefade` 0x829c2bd0.
    pub fn channels_volume_fade(&mut self, tracks: u32, set: i16, volume: i32, time: u32, curve: i32) -> i32 {
        self.ramp(tracks, set, volume, time, curve, fade::CHANNEL_VOL)
    }

    /// `PATHI_rampfunc` 0x829c2a20.
    fn ramp(&mut self, tracks: u32, bus: i16, to: i32, time: u32, curve: i32, what: i32) -> i32 {
        let mut result = -8;
        for (slot, index) in self.selected(tracks) {
            if self.track(slot, index).imp.is_some() {
                self.start_fade(slot, index, what, bus, to, time, curve);
                result = 0;
            }
        }
        result
    }

    /// `PATH_setnamedvalue` 0x829b7c90: the variable (case-folded to 16 characters) in every
    /// project of the handle's project bits.
    pub fn set_named_value(&mut self, handle: u32, name: &str, value: i32) -> i32 {
        let mut folded = [0u8; 16];
        for (i, byte) in name.bytes().take(16).enumerate() {
            folded[i] = byte | 0x20;
        }
        let mut result = -8;
        for project in self.projects.iter_mut().flatten() {
            if project.idflags & handle & 0x0F00_0000 == 0 {
                continue;
            }
            for var in &mut project.named {
                if var.name[0] == 0 {
                    continue;
                }
                let len = var.name.iter().position(|&b| b == 0).unwrap_or(16);
                let wanted = folded.iter().position(|&b| b == 0).unwrap_or(16);
                if wanted == len && var.name[..len] == folded[..len] {
                    var.value = value;
                    result = 0;
                }
            }
        }
        result
    }

    /// `PATH_event` 0x829c2060: a copy of the event in each project of the event's project bits,
    /// stamped with `now` (`IPathToReal::GetMilliseconds`).
    pub fn event(&mut self, tracks: u32, event_id: u32, now: u32) -> i32 {
        let projects = event_id & 0x0F00_0000;
        let id = event_id & 0x00FF_FFFF;
        let mut result = -8;
        let mut copy: Option<Event> = None;
        for slot in 0..4 {
            let Some(project) = self.projects[slot].as_ref() else { continue };
            if !project.matches(tracks) || project.idflags == 0 {
                continue;
            }
            self.current = slot;
            if (0x0100_0000u32 << project.map.project_id()) & projects == 0 {
                continue;
            }
            let Some(template) = project.map.find_event(id, u32::MAX) else { continue };
            let template = Event::from_template(template);
            if template.w4 & 0x4000 != 0 {
                result = -8;
                continue;
            }
            if copy.is_none() {
                copy = self.copy_event(&template);
            }
            if let Some(event) = copy.clone() {
                let flags = self.projects[slot].as_ref().map_or(0, |p| p.idflags);
                result = self.add_event(flags, event, now);
            }
        }
        result
    }

    /// `PATH_clearallevents` 0x829c22a8.
    pub fn clear_all_events(&mut self, projects: u32) -> i32 {
        let mut result = -8;
        for slot in 0..4 {
            let Some(project) = self.projects[slot].as_ref() else { continue };
            if !project.matches(projects) || project.idflags == 0 {
                continue;
            }
            self.current = slot;
            result = 0;
            while !self.project().queue.is_empty() {
                self.release_event(0, EventResult::Purged);
            }
        }
        result
    }

    /// `PATHI_copyevent` 0x829b77f0: the event copied into the project's 4096-byte event heap
    /// (`project` taken from the `idflags`).
    fn copy_event(&mut self, event: &Event) -> Option<Event> {
        let project = self.project();
        if project.heap_used() + event.size() >= EVENT_HEAP_BYTES {
            return None;
        }
        let mut copy = event.clone();
        copy.w4 = ((project.idflags << 11) & 0x3800) | (copy.w4 & !0x3800);
        Some(copy)
    }

    /// `PATHI_addevent` 0x829b7690: the actions' indents, then the event queued (stamped with
    /// `now`) in every project whose `idflags` lie within the voice and project bits.
    fn add_event(&mut self, project_flags: u32, mut event: Event, now: u32) -> i32 {
        let mut depth = 0i32;
        let mut open = -1i32;
        for action in &mut event.actions {
            let w1 = action[1];
            let set = |d: i32| ((8 * d) as u32 & 0x38) | (w1 & !0x38);
            if action_type(w1) == 1 {
                match assess(w1) {
                    0 | 1 => {
                        action[1] = set(depth);
                        open += 1;
                        depth += 1;
                    }
                    2 | 3 => action[1] = set(open),
                    4 => {
                        depth -= 1;
                        open -= 1;
                        action[1] = set(depth);
                    }
                    _ => {}
                }
            } else {
                action[1] = set(depth);
            }
        }
        let mask = project_flags & 0xFF00_0000;
        let mut result = -8;
        for project in self.projects.iter_mut().flatten() {
            if project.idflags & mask != project.idflags {
                continue;
            }
            if project.queue.len() >= 16 {
                return -13;
            }
            result = 0;
            event.last_act = now;
            event.queued = now;
            project.queue.push(event.clone());
        }
        result
    }

    /// `PATHI_releaseevent` 0x829bad58 and `PATHI_removeevent` 0x829b79d0: the heap copy goes and
    /// the queue closes up.
    fn release_event(&mut self, index: usize, _result: EventResult) {
        let queue = &mut self.project_mut().queue;
        if index < queue.len() {
            queue.remove(index);
        }
    }

    /// `PATHI_eventtakespriority` 0x829baf10: a prioritized event purges the later events of
    /// lower priority, or yields to one it does not outrank.
    fn event_takes_priority(&mut self, e: usize) -> bool {
        let own = self.project().queue[e].priority();
        let half = own - 2 * ((own >> 1) + i32::from(own < 0 && own & 1 != 0));
        let level = own.abs();
        let mut i = e + 1;
        while i < 16 {
            let Some(other) = self.project().queue.get(i).map(Event::priority) else { break };
            if other == 0 {
                i += 1;
                continue;
            }
            let other_level = other.abs();
            if own > 0 && other_level < half + level {
                self.release_event(i, EventResult::Purged);
                continue;
            }
            if !(other >= 0 && other_level <= level || level >= other_level % 2 + other_level) {
                return false;
            }
            i += 1;
        }
        true
    }

    /// `PATHI_serviceeventqueue` 0x829c4908.
    fn service_event_queue(&mut self, io: &mut SndIo<'_>) -> usize {
        let mut advanced = 0;
        let mut e = 0;
        while e < self.project().queue.len() {
            let event = &self.project().queue[e];
            let mut result = EventResult::Pending;
            if event.current_action() >= event.num_actions() {
                result = EventResult::Finished;
            }
            if event.expiry != 0 && event.queued.wrapping_add(event.expiry) < self.milliseconds {
                result = EventResult::Expired;
            }
            if event.w4 & 0xF_0000 != 0 && !self.event_takes_priority(e) {
                self.release_event(e, EventResult::Purged);
                continue;
            }
            if result != EventResult::Pending {
                self.release_event(e, result);
                continue;
            }
            advanced += self.service_event(e, io);
            e += 1;
        }
        advanced
    }

    /// `PATHI_serviceevent` 0x829c4840: the event's actions from `currentaction` until one is not
    /// done.
    fn service_event(&mut self, e: usize, io: &mut SndIo<'_>) -> usize {
        if self.project().queue[e].last_act > self.milliseconds {
            return 0;
        }
        let mut event = self.project().queue[e].clone();
        let mut advanced = 0;
        let mut action = event.current_action();
        while action < event.num_actions().min(event.actions.len()) {
            if event.actions[action][1] & DONE == 0 && !self.service_action(&mut event, action, io) {
                break;
            }
            action += 1;
            advanced += 1;
            event.set_current_action(event.current_action() + 1);
            event.last_act = self.milliseconds;
        }
        if let Some(slot) = self.project_mut().queue.get_mut(e) {
            *slot = event;
        }
        advanced
    }

    /// `PATHI_conditiondone` 0x829b7030: a true condition (`skip_to_next_else`) runs its body
    /// and marks the following branches done up to the `endif`; a false one marks its body done.
    fn condition_done(event: &mut Event, action: usize, skip_to_next_else: bool) {
        let level = indent(event.actions[action][1]);
        let mut skip = skip_to_next_else;
        for next in action + 1..event.actions.len() {
            let w1 = event.actions[next][1];
            if indent(w1) == level {
                if action_type(w1) == 1 && assess(w1) == 4 {
                    event.actions[next][1] |= DONE;
                    return;
                }
                if !skip_to_next_else {
                    return;
                }
                skip = false;
                event.actions[next][1] = w1 | DONE;
            } else if !skip {
                event.actions[next][1] = w1 | DONE;
            }
        }
    }

    /// `PATHI_restoretolastwhile` 0x829b70c0.
    fn restore_to_last_while(event: &mut Event, action: usize) -> bool {
        let level = indent(event.actions[action][1]);
        let Some(start) = (0..action).rev().find(|&a| indent(event.actions[a][1]) == level && action_type(event.actions[a][1]) == 1) else { return false };
        if event.actions[start][1] & 0xE00 != 0 {
            return false;
        }
        event.set_current_action(start);
        for a in start..=action {
            event.actions[a][1] &= !DONE;
        }
        true
    }

    /// `PATHI_serviceaction` 0x829c3c48: the action for each track it names (main-voice tracks
    /// only with bit 28, others only with bits 29..31); returns whether it is done.
    fn service_action(&mut self, event: &mut Event, a: usize, io: &mut SndIo<'_>) -> bool {
        let event_index = self.project().queue.len();
        let act = event.actions[a][2];
        for bit in 0..24 {
            let tracks = event.actions[a][0];
            if tracks & (1 << bit) == 0 {
                continue;
            }
            let handle = ((0xF000_0000 | (tracks & 0x0F00_0000)) & self.project().idflags) | (1 << bit);
            let Some((slot, t)) = self.track_ptr(handle) else { continue };
            if self.track(slot, t).imp.is_none() {
                continue;
            }
            let main = self.track(slot, t).main_voice;
            if tracks & 0x1000_0000 != 0 && !main {
                continue;
            }
            if tracks & 0xE000_0000 != 0 && tracks & 0x1000_0000 == 0 && main {
                continue;
            }
            let w1 = event.actions[a][1];
            let left_type = ubits(w1, 14, 2);
            let right_type = ubits(w1, 12, 2);
            match action_type(w1) {
                1 => {
                    let assess = assess(w1);
                    if assess == 3 {
                        event.actions[a][1] |= DONE;
                        return true;
                    }
                    if assess == 4 {
                        event.actions[a][1] |= DONE;
                        if !Self::restore_to_last_while(event, a) {
                            return event.actions[a][1] & DONE != 0;
                        }
                        if event.last_act == self.milliseconds {
                            event.last_act += 1;
                        }
                        event.actions[a][1] &= !DONE;
                        return event.actions[a][1] & DONE != 0;
                    }
                    let left_val = (act as i32) >> 16;
                    let left = self.get_value(left_type, left_val, slot, t, event, io);
                    let right = self.get_value(right_type, i32::from(act as i16), slot, t, event, io);
                    if w1 & 0xC000 == 0x4000 && left_val == 17 {
                        let status = self.track_status(slot, t, io);
                        if right == track_status::READY_FOR_NEXT && status == track_status::STOPPED {
                            event.actions[a][1] |= DONE;
                        }
                    }
                    let w1 = event.actions[a][1];
                    let result = match ubits(w1, 6, 3) {
                        0 => Some(true),
                        1 => Some(right == left),
                        2 => Some(right != left),
                        3 => Some(left > right),
                        4 => Some(left < right),
                        5 => Some(left >= right),
                        6 => Some(left <= right),
                        _ => None,
                    };
                    if let Some(result) = result {
                        event.actions[a][1] = if result { w1 | DONE } else { w1 & !DONE };
                    }
                    let w1 = event.actions[a][1];
                    if w1 & 0xE00 == 0 {
                        if w1 & DONE == 0 {
                            Self::condition_done(event, a, false);
                        }
                        event.actions[a][1] ^= DONE;
                        return true;
                    }
                    Self::condition_done(event, a, w1 & DONE != 0);
                    event.actions[a][1] |= DONE;
                }
                2 => {
                    let val = (act as i32) >> 16;
                    let mut wait = self.get_value(right_type, val, slot, t, event, io);
                    let lowest = i32::from(act as i16);
                    if lowest > 0 && lowest < wait {
                        wait = 0;
                    }
                    if w1 & 0x3000 == 0x1000 && (20..=22).contains(&val) {
                        let current = i32::from(event.actions[a][2] as i16);
                        if current == 0 || current > wait {
                            event.actions[a][2] = (event.actions[a][2] & 0xFFFF_0000) | (wait as u32 & 0xFFFF);
                        }
                        if wait > 0 {
                            wait = self.milliseconds.wrapping_add(3000) as i32;
                        }
                    }
                    if wait <= self.milliseconds.wrapping_sub(event.last_act) as i32 {
                        event.actions[a][2] &= 0xFFFF_0000;
                        event.actions[a][1] |= DONE;
                    }
                }
                3 => {
                    let millis = (act as i32) >> 16;
                    if millis == 0 {
                        if let Some(imp) = self.track_mut(slot, t).imp.as_mut() {
                            imp.mark_update();
                        }
                        let Some(info) = self.beat_info(slot, t, io) else {
                            event.actions[a][1] |= DONE;
                            continue;
                        };
                        let every = ubits(act, 12, 4) as i32;
                        let note = 1 << ubits(act, 8, 4);
                        let offset = ubits(act, 4, 4) as i32;
                        let wait = calc_wait_beat(every, note, offset, &info);
                        event.actions[a][2] = (event.actions[a][2] & 0xFFFF) | ((wait as u32) << 16);
                    } else if millis <= self.milliseconds.wrapping_sub(event.last_act) as i32 {
                        event.actions[a][2] = (event.actions[a][2] & 0xFFFF) | 0xFFFF_0000;
                        event.actions[a][1] |= DONE;
                    }
                }
                4 => {
                    let backup = self.track(slot, t).backup();
                    let mut node = self.get_value(left_type, (act as i32) >> 16, slot, t, event, io);
                    let section = self.get_value(right_type, ((act << 16) as i32) >> 24, slot, t, event, io);
                    if node < -1 || node > i32::from(self.project().map.num_nodes()) {
                        node = -1;
                    }
                    if node >= 0 {
                        let target = if section < 0 {
                            self.node(node)
                        } else {
                            let count = i32::from(self.project().map.num_nodes());
                            let mut found = None;
                            let mut index = 0;
                            while index < count {
                                let n = self.node(index);
                                if let Some(n) = n.filter(|n| n.section() == section as u32 && n.w[0] & 0xFFFF_0000 == 0) {
                                    node -= 1;
                                    if node == 0 {
                                        found = Some(n);
                                        node = index;
                                        break;
                                    }
                                }
                                index += 1;
                            }
                            if index >= count {
                                continue;
                            }
                            found
                        };
                        let Some(target) = target else { continue };
                        if target.track() != u32::from(self.track(slot, t).track_id) {
                            continue;
                        }
                    }
                    if self.track(slot, t).no_branch {
                        continue;
                    }
                    let origin = i32::from(self.track(slot, t).node);
                    if act & 0x80 != 0 {
                        self.stop_track(slot, t, io);
                    }
                    let ready = self.track(slot, t).loading_subbank < 0 && self.track_mut(slot, t).imp.as_mut().is_some_and(|imp| imp.ready_for_new_request(io));
                    let queued = ready && {
                        let control = self.track(slot, t).control;
                        self.track_mut(slot, t).node_beat = -1;
                        let entered = self.enter_node(origin, node, control, true);
                        self.track_mut(slot, t).node = entered as i16;
                        self.queue_node(slot, t, io) >= 0
                    };
                    if queued {
                        event.actions[a][1] |= DONE;
                    } else {
                        self.project_mut().queue.truncate(event_index);
                        self.track_mut(slot, t).restore(backup);
                    }
                }
                5 | 6 | 7 => {
                    let time = self.get_value(right_type, (act & 0xFFFF) as i32, slot, t, event, io);
                    let what = if action_type(w1) == 5 { fade::VOLUME } else { fade::SFX_SEND_LEVEL };
                    self.start_fade(slot, t, what, -1, (act >> 24) as i32, time as u32, sbits(act, 17, 7));
                    event.actions[a][1] |= DONE;
                }
                8 => {
                    let value = self.get_value(right_type, (act as i32) >> 16, slot, t, event, io);
                    event.actions[a][1] |= DONE;
                    self.set_value(ubits(event.actions[a][1], 14, 2), ubits(act, 8, 8) as i32, value, slot, t, event, io);
                }
                9 => {
                    let wanted = act >> 8;
                    let id = self.get_value(left_type, wanted as i32, slot, t, event, io);
                    let mask = if id as u32 == wanted { u32::MAX } else { 0xFFFF };
                    if let Some(template) = self.project().map.find_event(id as u32, mask).map(Event::from_template) {
                        if let Some(copy) = self.copy_event(&template) {
                            let flags = self.project().idflags | 0xF000_0000;
                            self.add_event(flags, copy, self.milliseconds);
                        }
                    }
                    event.actions[a][1] |= DONE;
                }
                13 => {
                    self.get_value(right_type, (act & 0xFFFF) as i32, slot, t, event, io);
                    self.get_value(left_type, (act as i32) >> 16, slot, t, event, io);
                    event.actions[a][1] |= DONE;
                }
                14 => {
                    let mut value = self.get_value(left_type, (act >> 24) as i32, slot, t, event, io);
                    let by = self.get_value(right_type, i32::from(act as i16), slot, t, event, io);
                    event.actions[a][1] |= DONE;
                    match ubits(act, 16, 8) {
                        1 => value = value.wrapping_add(by),
                        2 => value = value.wrapping_sub(by),
                        3 => value = value.wrapping_mul(by),
                        4 => value = value.checked_div(by).unwrap_or(0),
                        5 => value = value.checked_rem(by).unwrap_or(0),
                        _ => {}
                    }
                    self.set_value(ubits(event.actions[a][1], 14, 2), (act >> 24) as i32, value, slot, t, event, io);
                }
                15 => {
                    let when = self.get_value(right_type, (act >> 16) as i32, slot, t, event, io);
                    event.actions[a][1] |= DONE;
                    if when > 0 {
                        let at = self.milliseconds.wrapping_add(when as u32);
                        if act & 0x8000 != 0 {
                            self.track_mut(slot, t).pause_at = at;
                        } else {
                            self.track_mut(slot, t).resume_at = at;
                        }
                    } else {
                        let on = act & 0x8000 != 0;
                        let track = self.track_mut(slot, t);
                        track.paused = on;
                        if let Some(imp) = track.imp.as_mut() {
                            imp.pause(on, io);
                        }
                    }
                }
                16 => {
                    self.get_value(right_type, (act >> 24) as i32, slot, t, event, io);
                    event.actions[a][1] |= DONE;
                }
                17 | 18 => {
                    let time = self.get_value(right_type, (act & 0x3FFF) as i32, slot, t, event, io);
                    self.start_fade(slot, t, fade::SFX_SEND_LEVEL, -1, (act >> 18) as i32, time as u32, sbits(act, 15, 3));
                    event.actions[a][1] |= DONE;
                }
                _ => event.actions[a][1] |= DONE,
            }
        }
        event.actions[a][1] & DONE != 0
    }

    /// `PATHI_getvalue` 0x829c30c8: a constant (type 0), a special value (1) or a named variable
    /// (2).
    #[allow(clippy::too_many_arguments)]
    fn get_value(&mut self, kind: u32, value: i32, slot: usize, t: usize, event: &Event, io: &mut SndIo<'_>) -> i32 {
        match kind {
            2 => self.project().named.get(value as usize).map_or(0, |var| var.value),
            1 => {
                let node = i32::from(self.track(slot, t).node);
                match value {
                    1 => self.track(slot, t).control as i32,
                    2 => node,
                    3 => self.node(node).map_or(-1, |n| n.part()),
                    4 => self.node(node).map_or(-1, |n| n.section() as i32),
                    5 => event.expiry as i32,
                    6 => event.priority(),
                    7 => i32::from(self.track(slot, t).sfx_bus),
                    8 => i32::from(self.track(slot, t).dry_level),
                    9 => i32::from(self.track(slot, t).sfx_level),
                    10 => i32::from(self.track(slot, t).main_voice),
                    11 => {
                        let control = self.track(slot, t).control;
                        let next = self.next_node(node, control, false);
                        self.enter_node(node, next, control, false)
                    }
                    12 => i32::from(self.track(slot, t).no_branch),
                    14 => i32::from(self.track(slot, t).paused),
                    15 => self.track(slot, t).imp.as_ref().map_or(track_status::STOPPED, StreamTrack::pitch_mult),
                    17 => self.track_status(slot, t, io),
                    18 => i32::from((self.random() / 0x17) as u16),
                    19 => self.milliseconds as i32,
                    23 => {
                        let track = self.track(slot, t);
                        let volume = i32::from(track.volume);
                        if track.fade_stats(fade::VOLUME, -1).is_none() {
                            return volume;
                        }
                        track.imp.as_ref().map_or(volume, StreamTrack::volume)
                    }
                    24 => self.track(slot, t).imp.as_ref().map_or(track_status::STOPPED, StreamTrack::stretch_mult),
                    27 => self.channel_set(node),
                    13 | 16 | 20 | 21 | 22 | 25 | 26 | 28 => {
                        let status = self.path_status(slot, t, io);
                        match value {
                            13 => status.node_duration as i32,
                            16 => status.playing_node,
                            20 => status.time_left_in_beat as i32,
                            21 => status.time_left_in_bar as i32,
                            22 => status.time_left_in_node as i32,
                            25 => status.bar_duration as i32,
                            26 => status.beat_duration as i32,
                            _ => self.channel_set(status.playing_node),
                        }
                    }
                    _ => value,
                }
            }
            _ => value,
        }
    }

    fn path_status(&mut self, slot: usize, t: usize, io: &mut SndIo<'_>) -> crate::music::path_track::PathStatus {
        let map = self.project().map.clone();
        let node_info = move |node: i32| map.node(node).map(|n| (n.bars(), n.beats()));
        self.track_mut(slot, t).imp.as_mut().map(|imp| imp.path_status(node_info, io)).unwrap_or_default()
    }

    /// `PATHI_setvalue` 0x829c1420.
    #[allow(clippy::too_many_arguments)]
    fn set_value(&mut self, kind: u32, what: i32, to: i32, slot: usize, t: usize, event: &mut Event, io: &mut SndIo<'_>) {
        match kind {
            2 => {
                let flags = self.project().idflags;
                for project in self.projects.iter_mut().flatten() {
                    if (project.idflags ^ flags) & 0x0F00_0000 == 0 {
                        if let Some(var) = project.named.get_mut(what as usize) {
                            var.value = to;
                        }
                    }
                }
            }
            1 => match what {
                1 => self.track_mut(slot, t).control = (to as u32) & 0x7F,
                5 => event.expiry = to as u32,
                6 => {
                    let clipped = if to < -7 { 9 } else if to > 7 { 7 } else { to as u32 & 0xF };
                    event.w4 = (event.w4 & 0xFFF0_FFFF) | (clipped << 16);
                }
                7 => self.track_mut(slot, t).sfx_bus = (to & 7) as u8,
                8 => {
                    let level = to.clamp(0, 127);
                    let track = self.track_mut(slot, t);
                    track.dry_level = level as u16;
                    if let Some(imp) = track.imp.as_mut() {
                        imp.set_dry_level(level, io);
                    }
                }
                9 => {
                    let level = to.clamp(0, 127);
                    let track = self.track_mut(slot, t);
                    let bus = i32::from(track.sfx_bus);
                    if let Some(imp) = track.imp.as_mut() {
                        imp.set_fx_send_level(bus, level, io);
                    }
                    track.sfx_level = level as u16;
                }
                10 => self.main_voice(slot, t, to),
                12 => self.track_mut(slot, t).no_branch = to != 0,
                14 => {
                    let track = self.track_mut(slot, t);
                    track.paused = to != 0;
                    if let Some(imp) = track.imp.as_mut() {
                        imp.pause(to != 0, io);
                    }
                }
                15 => {
                    if let Some(imp) = self.track_mut(slot, t).imp.as_mut() {
                        imp.set_pitch_mult(to, io);
                    }
                }
                23 => self.set_track_volume(slot, t, to.clamp(0, 127) as i8, io),
                24 => {
                    if let Some(imp) = self.track(slot, t).imp.as_ref() {
                        imp.set_stretch_mult(to);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// `PATHI_mainvoice` 0x829b8938: the same track of the other voices takes the main voice
    /// when this one gives it up; a track with no other voice keeps it.
    fn main_voice(&mut self, slot: usize, t: usize, main: i32) {
        let track_id = usize::from(self.track(slot, t).track_id);
        self.track_mut(slot, t).main_voice = main != 0;
        let mut given = main > 0;
        for other in 0..3 {
            if other == slot {
                continue;
            }
            if let Some(track) = self.projects[other].as_mut().and_then(|p| p.tracks[track_id].as_deref_mut()) {
                track.main_voice = !given;
                given = true;
            }
        }
        let last = if slot == 3 { None } else { self.projects[3].as_mut().and_then(|p| p.tracks[track_id].as_deref_mut()) };
        if let Some(track) = last {
            track.main_voice = !given;
        } else if !given {
            self.track_mut(slot, t).main_voice = true;
        }
    }

    /// `PATHI_volume` 0x829bd528.
    fn set_track_volume(&mut self, slot: usize, t: usize, volume: i8, io: &mut SndIo<'_>) {
        let track = self.track_mut(slot, t);
        track.volume = volume;
        if let Some(imp) = track.imp.as_mut() {
            imp.set_volume(i32::from(volume), io);
        }
        if let Some(index) = track.fade_stats(fade::VOLUME, -1) {
            track.fades[index].reset();
        }
    }

    /// `PATHI_trackstatus` 0x829ba668. `PATHI_getfadestats` answers with a free slot when no
    /// volume fade runs, so a playing track reads `PATHTRACK_FADING`.
    fn track_status(&mut self, slot: usize, t: usize, io: &mut SndIo<'_>) -> i32 {
        let track = self.track_mut(slot, t);
        track.status = track_status::STOPPED;
        if track.paused {
            track.status = track_status::PAUSED;
            return track.status;
        }
        if track.node < 0 {
            return track.status;
        }
        track.status = track_status::PLAYING;
        let latency = i32::from(track.latency);
        let ready = track.loading_subbank < 0 && track.imp.as_mut().is_some_and(|imp| imp.ready_for_new_request(io));
        if ready && self.time_remaining(slot, t) <= latency {
            self.track_mut(slot, t).status = track_status::READY_FOR_NEXT;
        } else if let Some(imp) = self.track(slot, t).imp.as_ref() {
            if imp.time_buffered() < imp.time_remaining(-1) {
                self.track_mut(slot, t).status = track_status::QUEUEING;
            }
        }
        let track = self.track_mut(slot, t);
        if track.fade_stats(fade::VOLUME, -1).is_some() {
            track.status = track_status::FADING;
        }
        track.status
    }

    /// `PATHI_timeremaining` 0x829b8268: the queued requests' remaining times.
    fn time_remaining(&self, slot: usize, t: usize) -> i32 {
        let Some(imp) = self.track(slot, t).imp.as_ref() else { return 0 };
        let mut total = 0;
        for request in 0.. {
            let remaining = imp.time_remaining(request);
            if remaining < 0 {
                break;
            }
            total += remaining;
        }
        total
    }

    /// `PATHI_getchannelset` 0x829ba5f0.
    fn channel_set(&self, node: i32) -> i32 {
        if node < 0 || node > i32::from(self.project().map.num_nodes()) {
            return -1;
        }
        self.node(node).filter(|n| n.w[2] & 2 != 0).map_or(-1, |n| ((n.w[3] & 0xFF) >> 4) as i32)
    }

    /// `PATHI_beatinfo` 0x829bb898: the track node's note value (`notes`, bits 4..7 of its third
    /// word) with the implementation's `PATHSTATUS` beat and bar durations and time left in the
    /// bar.
    fn beat_info(&mut self, slot: usize, t: usize, io: &mut SndIo<'_>) -> Option<BeatInfo> {
        let node = i32::from(self.track(slot, t).node);
        if node < 0 {
            return None;
        }
        let info = self.node(node)?;
        let status = self.path_status(slot, t, io);
        Some(BeatInfo {
            notes: ubits(info.w[2], 4, 4),
            beat_duration: status.beat_duration,
            bar_duration: status.bar_duration,
            time_left_in_bar: status.time_left_in_bar,
        })
    }

    /// `PATHI_pickclosestbranch` 0x829b82c8.
    fn pick_closest_branch(branches: &[(i8, i8, i16)], control: i32) -> i32 {
        let mut best = 127;
        let mut pick = None;
        for &(min, max, dst) in branches {
            let low = (i32::from(min) - control).abs();
            let high = (i32::from(max) - control).abs();
            let distance = low.min(high);
            if best > distance {
                best = distance;
                pick = Some(dst);
            }
        }
        match (branches.is_empty(), pick) {
            (true, _) => -1,
            (false, Some(dst)) => i32::from(dst),
            (false, None) => i32::from(branches[0].2),
        }
    }

    /// `PATHI_nextnode` 0x829bbc28: the branch whose control range holds the control (the
    /// track's repeat count while it repeats this node, counted down for real), else the closest.
    fn next_node(&mut self, node: i32, control: u32, for_real: bool) -> i32 {
        if node < 0 {
            return -1;
        }
        let Some(info) = self.node(node) else { return -1 };
        if info.w[0] & 0xF800 >= 0xC000 {
            return -1;
        }
        let track_index = info.track() as usize;
        let Some(track) = self.project_mut().tracks.get_mut(track_index).and_then(|t| t.as_deref_mut()) else { return -1 };
        let mut control = control as i32;
        if i32::from(track.repeat_node) == node {
            let repeat = track.repeat;
            control = repeat & 0x7F;
            if info.repeat() > 0 && for_real {
                track.repeat = wrap5(repeat - 1);
                if repeat == 0 {
                    track.repeat_node = -1;
                }
            }
        }
        let branches = &info.branches[..info.num_branches()];
        let chosen = branches.iter().find(|&&(min, max, _)| control >= i32::from(min) && control <= i32::from(max)).map(|&(_, _, dst)| i32::from(dst));
        match chosen {
            Some(dst) if dst >= 0 => dst,
            _ => Self::pick_closest_branch(branches, control),
        }
    }

    /// `PATHI_routenode` 0x829bbea0: the origin's router may redirect the destination.
    fn route_node(&self, origin: i32, dest: i32) -> i32 {
        if origin < 0 {
            return dest;
        }
        let Some(info) = self.node(origin) else { return dest };
        let router = (info.w[1] >> 20) as usize;
        if router == 0 {
            return dest;
        }
        let map = &self.project().map;
        let start = map.router(router - 1);
        let count = map.router(router) - start;
        let mut dest = dest;
        for i in 0..count.max(0) {
            let entry = map.word((start + i) as usize);
            if dest == (entry as i32) >> 16 {
                dest = (entry & 0xFFFF) as i32;
            }
        }
        dest
    }

    /// `PATHI_enternode` 0x829c2418: from `node`, through the logic nodes (entry, repeat,
    /// random control, event) to the next audio node. The logic nodes act on the first node's
    /// track, and each step starts again from the given control.
    fn enter_node(&mut self, origin: i32, node: i32, control: u32, for_real: bool) -> i32 {
        if node < 0 {
            return -1;
        }
        let mut node = self.route_node(origin, node) as i16 as i32;
        if node < 0 {
            return -1;
        }
        let Some(mut info) = self.node(node) else { return -1 };
        let track_index = info.track() as usize;
        let mut step_control = control;
        loop {
            let index = info.index();
            if index > 0 {
                return self.route_node(origin, node) as i16 as i32;
            }
            if for_real {
                match index {
                    0 => {
                        if let Some(track) = self.project_mut().tracks[track_index].as_deref_mut() {
                            track.entry = Some(node);
                        }
                        if info.w[1] & 0x7000 == 0x1000 {
                            step_control = self.random() & 0x7F;
                        }
                        self.song_progress(info.section() as i32, info.part(), node);
                    }
                    -1 => {
                        let repeat = info.repeat();
                        if let Some(track) = self.project_mut().tracks[track_index].as_deref_mut() {
                            if repeat != 0 && node != i32::from(track.repeat_node) {
                                track.repeat_node = node as i16;
                                track.repeat = repeat;
                            }
                        }
                    }
                    -2 => step_control = self.random() & 0x7F,
                    -3 => {
                        if let Some(template) = self.project().map.find_event(info.w[3] >> 8, u32::MAX).map(Event::from_template) {
                            if let Some(copy) = self.copy_event(&template) {
                                let flags = self.project().idflags | 0xF000_0000;
                                self.add_event(flags, copy, self.milliseconds);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let next = self.next_node(node, step_control, for_real);
            if next < 0 {
                if for_real && origin >= 0 {
                    self.song_progress(-1, -1, next);
                }
                return -1;
            }
            node = self.route_node(origin, next) as i16 as i32;
            if node < 0 {
                if for_real && origin >= 0 {
                    self.song_progress(-1, -1, node);
                }
                return -1;
            }
            let Some(next_info) = self.node(node) else { return -1 };
            info = next_info;
            step_control = control;
        }
    }

    /// `PATHI_channelbranch` 0x829bbf38: the next node's channel set at full volume, the
    /// current one's silenced.
    fn channel_branch(&mut self, slot: usize, t: usize, io: &mut SndIo<'_>) {
        let node = i32::from(self.track(slot, t).node);
        let control = self.track(slot, t).control;
        let next = self.next_node(node, control, false);
        if self.track(slot, t).imp.is_some() && next >= 0 {
            if let (Some(from), Some(to)) = (self.node(node), self.node(next)) {
                let imp = self.track_mut(slot, t).imp.as_mut().expect("imp");
                imp.set_channels_volume(((from.w[3] & 0xFF) >> 4) as usize, 0, -1, -1, -1, io);
                imp.set_channels_volume(((to.w[3] & 0xFF) >> 4) as usize, 127, -1, -1, -1, io);
            }
        }
        self.track_mut(slot, t).node = next as i16;
    }

    /// `PATHI_queuenode` 0x829bc000 for a stream track: the node's sample queued with its full
    /// duration (the beat synchronisation is the RAM tracks').
    fn queue_node(&mut self, slot: usize, t: usize, io: &mut SndIo<'_>) -> i32 {
        let node = i32::from(self.track(slot, t).node);
        if node < 0 {
            let track = self.track_mut(slot, t);
            track.entry = None;
            track.newest_request = -1;
            track.node = -1;
            track.node_beat = -1;
            return 0;
        }
        let Some(info) = self.node(node) else { return -1 };
        let duration = self.project().map.sample((info.index() - 1) as usize).duration;
        let offset = self.sample_offset(node);
        let entry = self.node(info.part()).map(|_| info.part());
        let track = self.track_mut(slot, t);
        track.entry = entry;
        track.bar_time = duration;
        track.beat_time = duration;
        if info.bars() != 0 {
            track.bar_time = duration / info.bars();
        }
        if info.w[1] & 0xF00 != 0 {
            track.beat_time = duration / (info.beats() * info.bars().max(1));
        }
        track.node_beat = -1;
        let result = track.imp.as_mut().map_or(-1, |imp| imp.play(node, offset as i32, duration as i32, io));
        track.newest_request = result;
        if result >= 0 {
            self.song_progress(info.section() as i32, info.part(), node);
        }
        result
    }

    /// `PATHI_sampleoffset` 0x829bb7b0.
    fn sample_offset(&self, node: i32) -> u32 {
        if node < 0 || node >= i32::from(self.project().map.num_nodes()) {
            return 0;
        }
        let index = self.node(node).map_or(0, |n| n.index());
        if index > 0 {
            self.project().map.sample((index - 1) as usize).offset
        } else {
            0
        }
    }

    /// `PATHI_seeknextnode` 0x829c27c8: the node after the playing one entered and queued; a
    /// failure other than a stream cache still loading (-9999) restores the track.
    fn seek_next_node(&mut self, slot: usize, t: usize, io: &mut SndIo<'_>) {
        let node = i32::from(self.track(slot, t).node);
        if node < 0 {
            return;
        }
        let event_index = self.project().queue.len();
        let backup = self.track(slot, t).backup();
        let Some(info) = self.node(node) else { return };
        let node_beat = self.track(slot, t).node_beat;
        let mut beat = if self.track(slot, t).ram_track { 1 } else { -1 };
        let mut next;
        if node_beat > 0 && info.w[3] & 0x4000_0000 != 0 && node_beat <= (info.beats() * info.bars()) as i32 {
            beat = node_beat;
            next = node;
        } else {
            let control = self.track(slot, t).control;
            next = self.next_node(node, control, true);
            if next < 0 {
                self.song_progress(-1, -1, next);
            }
        }
        if next >= 0 {
            let control = self.track(slot, t).control;
            next = self.enter_node(node, next, control, true);
        }
        let track = self.track_mut(slot, t);
        track.node = next as i16;
        track.node_beat = if next < 0 { -1 } else { wrap5(beat) };
        let result = self.queue_node(slot, t, io);
        if result < 0 && result != -9999 {
            self.project_mut().queue.truncate(event_index);
            self.track_mut(slot, t).restore(backup);
        }
    }

    /// `MusicCore::PathLibProgressCB` 0x82a80690 (`Path::songprogress`).
    fn song_progress(&mut self, _section: i32, _part: i32, node: i32) {
        if node < 0 && self.flags.current_node >= 0 {
            self.flags.song_over = true;
            self.flags.state_ready = false;
        }
        self.flags.current_node = node;
    }

    /// `PATHI_startfade` 0x829bd728: the fade's start value from the implementation, its target
    /// clipped to the setting's range.
    #[allow(clippy::too_many_arguments)]
    fn start_fade(&mut self, slot: usize, t: usize, what: i32, bus: i16, to: i32, time: u32, curve: i32) {
        let now = self.milliseconds;
        let track = self.track_mut(slot, t);
        let Some(index) = track.fade_stats(what, bus) else { return };
        let Some(imp) = track.imp.as_ref() else { return };
        let set = usize::try_from(bus).unwrap_or(0);
        let (from, low, high) = match what {
            fade::VOLUME => (imp.volume(), 0, 127),
            fade::SFX_SEND_LEVEL => (imp.fx_send_level(), 0, 127),
            fade::DRY_LEVEL => (imp.dry_level(), 0, 127),
            fade::PITCH_MULT => (imp.pitch_mult(), 0, 0x4000),
            fade::STRETCH_MULT => (imp.stretch_mult(), 2048, 0x2000),
            fade::CHANNEL_VOL => (imp.channels_volume(set).0, 0, 127),
            fade::PAN_ANGLE => (imp.channels_position(set).0, 0, 0x7FFF),
            fade::PAN_DISTANCE => (imp.channels_position(set).1, 0, 0x4000),
            fade::PAN_SIZE => (imp.channels_position(set).2, 0, 0x4000),
            fade::PAN_TWIST => (imp.channels_position(set).3, 0, 0x4000),
            _ => (0, 0, 127),
        };
        if what == fade::VOLUME {
            track.volume = to as i8;
        }
        let stats = &mut track.fades[index];
        stats.what = what;
        stats.bus = bus;
        stats.time = time;
        stats.to = to.clamp(low, high) as i16;
        stats.start = now;
        if curve >= 0 {
            stats.num = curve as i16;
        }
        stats.from = from as i16;
    }

    /// `PATHI_updatefade` 0x829bda48: the fade's value applied when it moved.
    fn update_fade(&mut self, slot: usize, t: usize, index: usize, io: &mut SndIo<'_>) {
        let now = self.milliseconds;
        let track = self.track_mut(slot, t);
        let stats = track.fades[index];
        let value = calc_fade(&mut track.fades[index], now);
        if value < 0 {
            return;
        }
        let bus = track.sfx_bus;
        let Some(imp) = track.imp.as_mut() else { return };
        let set = usize::try_from(stats.bus).unwrap_or(0);
        let current = match stats.what {
            fade::VOLUME => imp.volume(),
            fade::SFX_SEND_LEVEL => imp.fx_send_level(),
            fade::DRY_LEVEL => imp.dry_level(),
            fade::PITCH_MULT => imp.pitch_mult(),
            fade::STRETCH_MULT => imp.stretch_mult(),
            fade::CHANNEL_VOL => imp.channels_volume(set).0,
            fade::PAN_ANGLE => imp.channels_position(set).0,
            fade::PAN_DISTANCE => imp.channels_position(set).1,
            fade::PAN_SIZE => imp.channels_position(set).2,
            fade::PAN_TWIST => imp.channels_position(set).3,
            _ => -1,
        };
        if current == value {
            return;
        }
        match stats.what {
            fade::VOLUME => {
                imp.set_volume(value, io);
            }
            fade::SFX_SEND_LEVEL => {
                imp.set_fx_send_level(i32::from(bus), value, io);
            }
            fade::DRY_LEVEL => {
                imp.set_dry_level(value, io);
            }
            fade::PITCH_MULT => {
                imp.set_pitch_mult(value, io);
            }
            fade::STRETCH_MULT => {
                imp.set_stretch_mult(value);
            }
            fade::CHANNEL_VOL => {
                imp.set_channels_volume(set, value, -1, -1, -1, io);
            }
            fade::PAN_ANGLE => {
                imp.set_channels_position(set, value, -1, -1, -1, io);
            }
            fade::PAN_DISTANCE => {
                imp.set_channels_position(set, -1, value, -1, -1, io);
            }
            fade::PAN_SIZE => {
                imp.set_channels_position(set, -1, -1, value, -1, io);
            }
            fade::PAN_TWIST => {
                imp.set_channels_position(set, -1, -1, -1, value, io);
            }
            _ => {}
        }
    }

    /// `PATHI_getmastertrack` 0x829b86d8: the playing, unpaused track of the longest latency.
    fn find_master_track(&mut self) {
        let project = self.project_mut();
        project.master_latency = 0;
        project.master_track = -1;
        project.master_section = 0;
        let mut best = (0u32, -1i32, 0u32);
        for (index, track) in project.tracks.iter().enumerate() {
            let Some(track) = track.as_deref() else { continue };
            if track.paused || track.node < 0 {
                continue;
            }
            let Some(entry) = track.entry else { continue };
            if best.0 < u32::from(track.latency) {
                best = (u32::from(track.latency), index as i32, entry as u32);
            }
        }
        let map = project.map.clone();
        project.master_latency = best.0;
        project.master_track = best.1;
        project.master_section = map.node(best.2 as i32).map_or(0, |n| n.section());
    }

    /// `PATHI_service` 0x829c4d10: every implementation's status, each project's master track
    /// and `PATHI_serviceproject` 0x829c4a50, then the status marked stale.
    fn service(&mut self, bank: bool, io: &mut SndIo<'_>) {
        self.bank_service = bank;
        self.milliseconds = get_milliseconds(io.core);
        self.status_all(false, io);
        for slot in 0..4 {
            let Some(project) = self.projects[slot].as_ref() else { continue };
            if project.idflags & 0xF000_0000 == 0 || project.idflags & 0x0F00_0000 == 0 {
                continue;
            }
            self.current = slot;
            self.find_master_track();
            self.service_project(bank, io);
        }
        self.status_all(true, io);
    }

    /// `PATHI_statusall` 0x829b8a88.
    fn status_all(&mut self, clear: bool, io: &mut SndIo<'_>) {
        for project in self.projects.iter_mut().flatten() {
            for track in project.tracks.iter_mut().flatten() {
                if let Some(imp) = track.imp.as_mut() {
                    if clear {
                        imp.mark_update();
                    } else {
                        imp.check_status(io);
                    }
                }
            }
        }
    }

    fn service_project(&mut self, bank: bool, io: &mut SndIo<'_>) {
        let interval = if bank { self.project().timer_interval } else { self.project().task_interval };
        if !bank {
            self.service_event_queue(io);
        }
        for t in 0..24 {
            let slot = self.current;
            if self.project().tracks[t].as_ref().is_none_or(|track| track.imp.is_none()) {
                continue;
            }
            let playing = self.track(slot, t).node >= 0 && self.track(slot, t).entry.is_some();
            if playing {
                for index in 0..16 {
                    if self.track(slot, t).fades[index].num >= 0 {
                        self.update_fade(slot, t, index, io);
                    }
                }
            }
            let now = self.milliseconds;
            let track = self.track_mut(slot, t);
            if track.pause_at != 0 && track.pause_at <= (interval >> 1).wrapping_add(now) {
                if let Some(imp) = track.imp.as_mut() {
                    imp.pause(true, io);
                }
                track.pause_at = 0;
                track.paused = true;
            }
            if track.resume_at != 0 && track.resume_at <= (interval >> 1).wrapping_add(now) {
                if let Some(imp) = track.imp.as_mut() {
                    imp.pause(false, io);
                }
                track.resume_at = 0;
                track.paused = false;
            }
            if track.ram_track != bank || !playing || track.paused || track.loading_subbank >= 0 {
                continue;
            }
            if !track.imp.as_mut().is_some_and(|imp| imp.ready_for_new_request(io)) {
                continue;
            }
            let remaining = self.time_remaining(slot, t);
            let track = self.track(slot, t);
            if track.next_beat_time != 0 {
                if track.next_beat_time > now {
                    continue;
                }
            } else if remaining >= i32::from(track.latency) {
                continue;
            }
            self.seek_next_node(slot, t, io);
        }
        if !bank {
            while self.service_event_queue(io) != 0 {}
        }
    }

    /// The Pathfinder timer (`RWAudioCoreTimer::Update` 0x829c50e0, a `TIMERSTAGE_BEFORECOMMANDS`
    /// timer: `PATHI_servicetimer` 0x829c4df8 every 10 ms) and the service task
    /// (`ServiceTaskThreadFunc` 0x829c4e90: `PATHI_service(32)` every 50 ms, here on the same
    /// audio clock).
    pub fn timer(&mut self, period: f32, io: &mut SndIo<'_>) {
        if f64::from(self.timer_this_call) <= f64::from(self.timer_last_call) + TIMER_SECONDS {
            self.timer_this_call += period;
        } else {
            if !self.paused {
                self.service(true, io);
            }
            self.timer_last_call = self.timer_this_call;
            self.timer_this_call += period;
        }
        self.task_clock += period * 1000.0;
        if self.task_clock >= TASK_MILLISECONDS {
            self.task_clock -= TASK_MILLISECONDS;
            if !self.paused {
                self.service(false, io);
            }
        }
    }

    /// The core and dry submix the implementation works with.
    #[must_use]
    pub fn snd_io<'a>(&self, core: &'a mut AudioCore, default_submix: PluginRef) -> SndIo<'a> {
        SndIo { core, dry_submix: self.dry_submix.unwrap_or(default_submix) }
    }
}

/// `PathToRWFileSystem::GetMilliseconds` 0x829b8e38: the library's stopwatch, here the audio
/// core's clock.
#[must_use]
pub fn get_milliseconds(core: &AudioCore) -> u32 {
    (core.system_time * 1000.0) as u32
}

/// `PATHBEATINFO`'s fields `PATHI_calcwaitbeat` reads (the low nibble of byte 0, +4, +8 and
/// +0x14).
struct BeatInfo {
    notes: u32,
    beat_duration: u32,
    bar_duration: u32,
    time_left_in_bar: u32,
}

/// `PATHI_calcwaitbeat` 0x829b8120: the wait to the next `every`-th `note` beat after `offset`.
fn calc_wait_beat(every: i32, note: i32, offset: i32, info: &BeatInfo) -> i32 {
    let bar = info.bar_duration as i32;
    let time_in_bar = bar - info.time_left_in_bar as i32;
    let scale = info.notes as f32 / note as f32;
    let beat = info.beat_duration as f32;
    let first = (offset as f32 * scale * beat) as i32;
    let mut next = first;
    if first < time_in_bar && first < bar {
        let step = (beat * (every as f32 * scale)) as i32;
        if step <= 0 {
            return next - time_in_bar;
        }
        loop {
            next += step;
            if next > time_in_bar || next >= bar {
                break;
            }
        }
    }
    if next > bar {
        next = bar + first;
    }
    next - time_in_bar
}

/// `PATHI_calcfade` 0x829bd5b8: the fade's value now along its curve (1 linear, 2 quadratic,
/// 3 inverse), -1 when the curve leaves 0..1; at the end the fade clears and its target returns.
fn calc_fade(stats: &mut FadeStats, now: u32) -> i32 {
    let to = i32::from(stats.to);
    let from = i32::from(stats.from);
    let diff = to as f32 - from as f32;
    let t = now.wrapping_sub(stats.start) as f32 / stats.time as f32;
    if !(t < 1.0) {
        stats.reset();
        return to;
    }
    let t = if diff >= 0.0 { 1.0 - t } else { t };
    let curve = match stats.num {
        1 => 1.0 - t,
        2 => -t.mul_add(t, -1.0),
        3 => (1.0 / t) * f32::from_bits(0x3D23_D70A),
        _ => -1.0,
    };
    let base = if to < from { to as f32 } else { from as f32 };
    if (0.0..=1.0).contains(&curve) {
        return curve.mul_add(diff.abs(), base) as i32;
    }
    -1
}

/// A 5-bit signed bit-field's value.
fn wrap5(value: i32) -> i32 {
    ((value << 27) as i32) >> 27
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fades_follow_their_curve() {
        let mut stats = FadeStats { what: fade::VOLUME, bus: -1, from: 0, to: 100, num: 1, start: 0, time: 1000 };
        assert_eq!(calc_fade(&mut stats, 250), 25);
        let mut down = FadeStats { from: 100, to: 0, ..stats };
        assert_eq!(calc_fade(&mut down, 250), 75);
        assert_eq!(calc_fade(&mut stats, 1000), 100);
        assert_eq!(stats.what, fade::INVALID);
    }

    #[test]
    fn random_matches_the_seed_steps() {
        let mut path = Path::new();
        let a = path.random();
        let b = path.random();
        assert_ne!(a, b);
        assert_eq!(wrap5(31), -1);
        assert_eq!(wrap5(-2), -2);
    }
}
