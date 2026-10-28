//! The EA `spch` speech library (`SPCH_*` / `iSPCH_*`, 0x829b3180-0x829b6ef0 and
//! 0x82ee5680-0x82ee5938): the event databases (`VoxData`, the `<group>Output_Events.evt`
//! files), the sample banks (`VOXBANKHDR`, the `.hdr` files), the queue of pending events per
//! channel, sentence and sample choice, and the sample requests handed to the speech system.
//!
//! The library reads and writes its files in place: sentences keep their match-parameter results
//! (`iSPCH_InitMatchParmIO`), event databases their global match parameters and banks their cycle
//! bits and repeat queues, so each file is an owned buffer here.
//!
//! `SPCH_Init` clears every rule callback and nothing sets them again, so the rule tests pass
//! (`iSPCH_GetRuleSettings` puts every rule in the user mask) and `iSPCH_RuleSet` /
//! `iSPCH_ConstantRuleSet` return at once.

/// `gVoxEvents.events`, `gEventDats` and the channels.
const EVENT_SLOTS: usize = 16;
const CHANNELS: usize = 8;
/// `SPCH_InitBankMem(5000, ...)` (`SpeechSystem::Initialize`).
pub const MAX_BANKS: usize = 5000;
/// `SentencePickInfo::phraseInfo` and `EventChoice::phrases`.
const MAX_PHRASES: usize = 12;
/// `iSPCH_OrderSentences`' `multiple` (0x82ff2794).
const MULTIPLE: [i32; 8] = [1, 4, 16, 64, 256, 1024, 4096, 16384];
/// `gClearCycle` (0x82ff2790).
const CLEAR_CYCLE: bool = true;
/// The `callbackData` of an event added without one (`SPCH_AddEventV`, `iSPCH_CsisCb`).
pub const NO_CALLBACK_DATA: u32 = 0xDEAD_BEEF;

fn rd8(bytes: &[u8], at: usize) -> u8 {
    bytes.get(at).copied().unwrap_or(0)
}

fn rd16(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([rd8(bytes, at), rd8(bytes, at + 1)])
}

fn rd32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([rd8(bytes, at), rd8(bytes, at + 1), rd8(bytes, at + 2), rd8(bytes, at + 3)])
}

fn wr8(bytes: &mut [u8], at: usize, value: u8) {
    if let Some(byte) = bytes.get_mut(at) {
        *byte = value;
    }
}

fn wr32(bytes: &mut [u8], at: usize, value: u32) {
    if let Some(word) = bytes.get_mut(at..at + 4) {
        word.copy_from_slice(&value.to_be_bytes());
    }
}

const fn align_up(n: usize) -> usize {
    (n + 3) & !3
}

const fn align_down(n: usize) -> usize {
    n & !3
}

/// `EventSpec`: the event, its event database and its project, kept as the word the parameter
/// arrays start with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventSpec {
    pub event_id: u16,
    pub dat_id: u8,
    pub project_id: u8,
}

impl EventSpec {
    /// `SPCH_MakeEventSpec` 0x829b3248.
    #[must_use]
    pub const fn new(project_id: u8, dat_id: u8, event_id: u16) -> Self {
        Self { event_id, dat_id, project_id }
    }

    #[must_use]
    pub const fn word(self) -> u32 {
        ((self.event_id as u32) << 16) | ((self.dat_id as u32) << 8) | self.project_id as u32
    }

    #[must_use]
    pub const fn from_word(word: u32) -> Self {
        Self { event_id: (word >> 16) as u16, dat_id: (word >> 8) as u8, project_id: word as u8 }
    }
}

/// A row of an event database's Csis table (`<project>Output_Events.evt`, or a context
/// system's `Trigger_Events.evt`): a function's `Csis::InterfaceId` and its event's id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CsisRow {
    pub(super) name: String,
    pub(super) system_crc: i16,
    pub(super) interface_crc: i16,
    pub(super) event_id: u16,
}

/// The Csis table `iSPCH_InitCsis` 0x82ee5938 and `ContextSystemImpl::AddCsisEvents` 0x82ab3dd0
/// walk: at the offset in word 1 (none while 0 or once byte 11 marks it bound), 32-byte rows of
/// `{name offset, system CRC, interface CRC, handle, client}` for each event, the names after
/// them; row `i` is event `i` of the event table at +0x18.
pub(super) fn csis_table(evt: &[u8]) -> Vec<CsisRow> {
    let table = rd32(evt, 4) as usize;
    if table == 0 || rd8(evt, 11) != 0 {
        return Vec::new();
    }
    let count = usize::from(rd16(evt, 0x10));
    let strings = table + 32 * count;
    (0..count)
        .map(|i| {
            let row = table + 32 * i;
            let name_at = strings + rd32(evt, row) as usize;
            let name = evt.get(name_at..).map_or(&[][..], |tail| &tail[..tail.iter().position(|&b| b == 0).unwrap_or(tail.len())]);
            CsisRow {
                name: String::from_utf8_lossy(name).into_owned(),
                system_crc: rd16(evt, row + 4) as i16,
                interface_crc: rd16(evt, row + 6) as i16,
                event_id: rd16(evt, 4 * usize::from(rd16(evt, 0x18 + 2 * i))),
            }
        })
        .collect()
}

/// `SPCHType_SampleRequestData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleRequestData {
    pub bank_num: i32,
    pub sample_offset: i32,
    pub num_bytes: i32,
    pub event_spec: EventSpec,
    pub channel: i32,
    pub sub_id: i32,
    pub dat_id: i32,
    pub interrupt_flag: i32,
    pub callback_data: u32,
}

/// A `VoxEvent` inside an event database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EventRef {
    db: usize,
    offset: usize,
}

/// `VoxEventItem`.
#[derive(Debug, Clone, Default)]
struct VoxEventItem {
    entry_time: u32,
    sub_ticks: u16,
    pending: bool,
    channel: u8,
    event: Option<EventRef>,
    mem_parms: Option<Vec<u32>>,
    callback_data: u32,
}

/// `VOXINGAME`.
#[derive(Debug, Clone, Copy)]
struct VoxInGame {
    last_event_spec: EventSpec,
    num_event_times: i32,
}

/// `EventDatInfo`.
#[derive(Debug, Clone, Copy)]
struct EventDatInfo {
    db: usize,
    channel: u32,
}

/// `VoxBankInfo`: the bank's handle and its header (an index into the bank headers).
#[derive(Debug, Clone, Copy, Default)]
struct VoxBankInfo {
    handle: i32,
    hdr: Option<usize>,
}

/// `PhraseChoice`.
#[derive(Debug, Clone, Copy)]
struct PhraseChoice {
    bank_handle: i32,
    sub_bank_index: u32,
    bank_index: i16,
    sample_index: u16,
}

impl Default for PhraseChoice {
    fn default() -> Self {
        Self { bank_handle: -1, sub_bank_index: u32::MAX, bank_index: -1, sample_index: 0 }
    }
}

/// `EventChoice`.
#[derive(Debug, Clone)]
struct EventChoice {
    event: Option<EventRef>,
    sentence: Option<usize>,
    sentence_num: u8,
    valid_choice: bool,
    num_phrases: u8,
    mem_parms: Option<Vec<u32>>,
    phrases: [PhraseChoice; MAX_PHRASES],
    callback_data: u32,
}

impl Default for EventChoice {
    fn default() -> Self {
        Self {
            event: None,
            sentence: None,
            sentence_num: 0,
            valid_choice: false,
            num_phrases: 0,
            mem_parms: None,
            phrases: [PhraseChoice::default(); MAX_PHRASES],
            callback_data: NO_CALLBACK_DATA,
        }
    }
}

/// `PhrasePickInfo`.
#[derive(Debug, Clone, Copy, Default)]
struct PhrasePickInfo {
    bank_handle: i32,
    sub_bank_index: u32,
    bank_index: i16,
    pick_start: u8,
    num_picks: u8,
    picked_index: u8,
    done: bool,
}

/// `SentencePickInfo`.
struct SentencePickInfo {
    phrase_info: [PhrasePickInfo; MAX_PHRASES],
    num_phrases: u8,
    pick_index: u8,
    valid_samples: [u8; 255],
}

impl SentencePickInfo {
    fn new() -> Self {
        Self { phrase_info: [PhrasePickInfo::default(); MAX_PHRASES], num_phrases: 0, pick_index: 0, valid_samples: [0; 255] }
    }
}

/// `iSPCH_EACrandom` 0x829b5f70 and `iSPCH_Rand` 0x829b6050's history (`seedX`, `gRandArray`,
/// `gRandArrayIndex`).
struct SpchRandom {
    seed: [u32; 6],
    /// `RandHandle { rand, handle }`.
    history: [(u16, u16); 32],
    index: i32,
}

impl SpchRandom {
    /// `iSPCH_InitRandom` 0x829b61c8: the six words from `x = seed * 65537`, then
    /// `32 - x % 32` draws (signed remainder).
    fn new(game_seed: u32) -> Self {
        let x = game_seed.wrapping_mul(65537);
        let mut random = Self { seed: Self::seeded(x), history: [(0xFFFF, 0xFFFF); 32], index: 0 };
        let draws = 32 - (x as i32) % 32;
        for _ in 0..draws.max(0) {
            random.eac_random();
        }
        random
    }

    fn seeded(x: u32) -> [u32; 6] {
        let s0 = x.wrapping_add(0xF22D_0000).wrapping_add(0x0E56);
        let s1 = s0.wrapping_add(0x9604_0000).wrapping_add(0x1893);
        let s2 = s1.wrapping_add(0x3DF4_0000).wrapping_sub(0x49BA);
        let s3 = s2.wrapping_add(0x40DE_0000).wrapping_sub(0x1893);
        let s4 = s3.wrapping_add(0x9732_0000).wrapping_add(0x7AE1);
        let s5 = s4.wrapping_add(0xD1AA_0000).wrapping_sub(0x419);
        [s0, s1, s2, s3, s4, s5]
    }

    /// `iSPCH_EACrandom` 0x829b5f70: a six-word add-with-carry generator.
    fn eac_random(&mut self) -> u32 {
        let [s0, s1, s2, s3, s4, s5] = self.seed;
        let sum45 = s4.wrapping_add(s5);
        let carry45 = u32::from(sum45 < s5 || sum45 < s4);
        let new3 = s3.wrapping_add(sum45).wrapping_add(carry45);
        let new2 = s2.wrapping_add(new3).wrapping_add(u32::from(new3 < s3));
        let new1 = s1.wrapping_add(new2).wrapping_add(u32::from(new2 < s2));
        let new0 = s0.wrapping_add(new1).wrapping_add(u32::from(new1 < s1));
        let new5 = s5.wrapping_add(1);
        self.seed = [new0, new1, new2, new3, sum45, new5];
        if new5 != 0 {
            return new0;
        }
        self.seed[4] = sum45.wrapping_add(1);
        if self.seed[4] != 0 {
            return new0;
        }
        self.seed[3] = new3.wrapping_add(1);
        if self.seed[3] != 0 {
            return new0;
        }
        self.seed[2] = new2.wrapping_add(1);
        if self.seed[2] != 0 {
            return new0;
        }
        self.seed[1] = new1.wrapping_add(1);
        if self.seed[1] != 0 {
            return new0;
        }
        self.seed[0] = new0.wrapping_add(1);
        self.seed[0]
    }

    fn draw(&mut self, max: i32) -> u32 {
        ((self.eac_random() >> 16).wrapping_mul(max as u32)) >> 16
    }

    /// `iSPCH_Rand` 0x829b6050: a draw in `0..max`. With a handle it redraws (up to 32 times)
    /// a value its last `min(max / 2, 10)` draws gave, keeping the one seen longest ago in the
    /// history; it returns the last draw.
    fn rand(&mut self, max: i32, handle: i32) -> u32 {
        let limit = (max / 2).min(10);
        let mut result = self.draw(max);
        if handle == -1 {
            return result;
        }
        let start = self.index;
        let mut attempts = 0;
        let mut unique = false;
        let mut best_age = -1;
        let mut best = result as u16;
        loop {
            let window = limit.min(32);
            let mut age = -1;
            let mut matches = 0;
            let mut at = start;
            let mut stop = false;
            let mut visited = 0;
            while visited <= 32 && !stop {
                if matches >= window {
                    stop = true;
                } else {
                    let (value, owner) = self.history[at as usize];
                    if owner == handle as u16 {
                        matches += 1;
                        if value == result as u16 {
                            age = matches - 1;
                            stop = true;
                        }
                    }
                    at -= 1;
                    if at < 0 {
                        at += 32;
                    }
                    visited += 1;
                }
            }
            if age == -1 {
                unique = true;
                best = result as u16;
            } else if age > best_age {
                best_age = age;
                best = result as u16;
            }
            attempts += 1;
            if attempts >= 32 || unique {
                break;
            }
            result = self.draw(max);
        }
        self.index = start + 1;
        if self.index >= 32 {
            self.index = 0;
        }
        self.history[self.index as usize] = (best, handle as u16);
        result
    }
}

/// The library's globals.
pub struct Spch {
    /// The loaded event databases (`VoxData`), by the index `add_event_db` returns.
    dbs: Vec<Vec<u8>>,
    /// The bank headers (`VOXBANKHDR`), by the index `add_bank` takes.
    bank_hdrs: Vec<Vec<u8>>,
    event_dats: [Option<EventDatInfo>; CHANNELS],
    num_pending: [i32; CHANNELS],
    last_added_event: [i32; CHANNELS],
    events: [VoxEventItem; EVENT_SLOTS],
    in_game: [VoxInGame; CHANNELS],
    filter_setting: [i32; CHANNELS],
    pre_load_ticks: i32,
    last_tick: u32,
    last_sub_tick: u16,
    data_rate: i32,
    game_num: u32,
    /// `gVoxBanks` (`gNumBanks` entries), `gBankCount` of them in use, sorted by type and sub id.
    banks: Vec<VoxBankInfo>,
    bank_count: i32,
    unique_bank_handle: i32,
    event_choice: [EventChoice; CHANNELS],
    random: SpchRandom,
    add_event_status: bool,
    play_status: bool,
    /// `SPCHEXT_gettick`: `gExtVecs.spchGetTick`, the simulation tick.
    pub tick: u32,
    /// `gCallbacks.request` calls, in order.
    requests: Vec<SampleRequestData>,
    /// The output functions `iSPCH_InitCsis` subscribed `iSPCH_CsisCb` to, oldest first, with
    /// the event each adds.
    csis: Vec<(CsisRow, EventSpec)>,
}

impl Spch {
    /// `SPCH_Init` 0x829b40f8 (the request callback is the caller's queue) and
    /// `SPCH_InitBankMem(5000, ...)`.
    #[must_use]
    pub fn new(game_seed: u32, sample_data_rate: i32) -> Self {
        let mut spch = Self {
            dbs: Vec::new(),
            bank_hdrs: Vec::new(),
            event_dats: [None; CHANNELS],
            num_pending: [0; CHANNELS],
            last_added_event: [-1; CHANNELS],
            events: Default::default(),
            in_game: [VoxInGame { last_event_spec: EventSpec::from_word(0xFFFF_0000), num_event_times: 0 }; CHANNELS],
            filter_setting: [0; CHANNELS],
            pre_load_ticks: 0,
            last_tick: 0,
            last_sub_tick: 0,
            data_rate: sample_data_rate,
            game_num: game_seed,
            banks: vec![VoxBankInfo::default(); MAX_BANKS],
            bank_count: 0,
            unique_bank_handle: 0,
            event_choice: Default::default(),
            random: SpchRandom::new(game_seed),
            add_event_status: false,
            play_status: false,
            tick: 0,
            requests: Vec::new(),
            csis: Vec::new(),
        };
        spch.init_event_queue();
        spch
    }

    /// `iSPCH_InitEventQueue` 0x829b3598.
    fn init_event_queue(&mut self) {
        self.num_pending = [0; CHANNELS];
        self.last_added_event = [-1; CHANNELS];
        for item in &mut self.events {
            *item = VoxEventItem { channel: 0xFF, callback_data: NO_CALLBACK_DATA, ..VoxEventItem::default() };
        }
        self.last_tick = 0;
        self.last_sub_tick = 0;
    }

    /// The sample requests made since the last call.
    pub fn take_requests(&mut self) -> Vec<SampleRequestData> {
        std::mem::take(&mut self.requests)
    }

    // ---- event databases ----------------------------------------------------------------

    /// `SPCH_AddEventDB` 0x82ee57e8 / `iSPCH_BindData` 0x82ee5700: a revision 3.21 database
    /// takes the first free slot unless one with its project and dat ids is bound; the
    /// channel's global match parameters are cleared either way. Then `iSPCH_InitCsis` 0x82ee5938
    /// subscribes `iSPCH_CsisCb` to each event's Csis function (the context systems' outputs)
    /// with the event's spec. Returns the database index.
    pub fn add_event_db(&mut self, mut data: Vec<u8>, channel: u32) -> Option<usize> {
        if channel >= CHANNELS as u32 {
            return None;
        }
        let rows = csis_table(&data);
        if !rows.is_empty() {
            let (project, dat) = (rd8(&data, 8), rd8(&data, 9));
            self.csis.extend(rows.into_iter().map(|row| {
                let spec = EventSpec::new(project, dat, row.event_id);
                (row, spec)
            }));
            wr8(&mut data, 11, 1);
        }
        if rd8(&data, 0) != 3 || rd8(&data, 1) != 21 {
            return None;
        }
        let (project, dat) = (rd8(&data, 8), rd8(&data, 9));
        let bound = self.event_dats.iter().flatten().any(|info| {
            let db = &self.dbs[info.db];
            rd8(db, 9) == dat && rd8(db, 8) == project
        });
        if bound {
            return None;
        }
        let Some(slot) = self.event_dats.iter().position(Option::is_none) else {
            self.clear_match_parm_settings(channel);
            return None;
        };
        self.dbs.push(data);
        let db = self.dbs.len() - 1;
        self.event_dats[slot] = Some(EventDatInfo { db, channel });
        self.clear_match_parm_settings(channel);
        Some(db)
    }

    /// The event a context system's output function adds: `Csis::FunctionHandle::Set` finds the
    /// newest subscribed function of the name and interface CRC, first among those of the
    /// system CRC; `None` when no event database subscribed one.
    pub(super) fn csis_event(&self, name: &str, system_crc: i16, interface_crc: i16) -> Option<EventSpec> {
        let matches = |row: &CsisRow, by_system: bool| (!by_system || row.system_crc == system_crc) && row.interface_crc == interface_crc && row.name == name;
        [true, false].into_iter().find_map(|by_system| self.csis.iter().rev().find(|(row, _)| matches(row, by_system)).map(|&(_, spec)| spec))
    }

    /// `iSPCH_CsisCb` 0x82ee58b0: the event's spec and its `numParms` parameter words added
    /// (`iSPCH_AddEvent`, no callback data). `numParms` is a signed byte; no shipped event has a
    /// negative one, for which nothing is added.
    pub(super) fn csis_callback(&mut self, spec: EventSpec, params: &[i32]) {
        let Some(event) = self.find_event(spec) else { return };
        let Ok(count) = usize::try_from(self.event_u8(event, 0xB) as i8) else { return };
        let mut parms = Vec::with_capacity(count + 1);
        parms.push(spec.word());
        parms.extend((0..count).map(|i| params.get(i).copied().unwrap_or(0) as u32));
        self.add_event(parms, NO_CALLBACK_DATA);
    }

    /// `SPCH_ClearMatchParmSettings` 0x82ee5680: the channel's global match parameters to 0xFF.
    fn clear_match_parm_settings(&mut self, channel: u32) {
        for info in self.event_dats.iter().flatten() {
            if info.channel != channel {
                continue;
            }
            let db = &mut self.dbs[info.db];
            let start = 0x18 + align_up(usize::from(rd16(db, 0x10)) * 2);
            for i in 0..usize::from(rd8(db, 0x12)) {
                wr8(db, start + i, 0xFF);
            }
        }
    }

    /// `iSPCH_FindEventDatInfo` 0x829b3260.
    fn find_event_dat_info(&self, spec: EventSpec) -> Option<EventDatInfo> {
        self.event_dats.iter().flatten().copied().find(|info| {
            let db = &self.dbs[info.db];
            rd8(db, 9) == spec.dat_id && rd8(db, 8) == spec.project_id
        })
    }

    /// `iSPCH_FindEvent` 0x829b32d0.
    fn find_event(&self, spec: EventSpec) -> Option<EventRef> {
        let info = self.find_event_dat_info(spec)?;
        let db = &self.dbs[info.db];
        (0..usize::from(rd16(db, 0x10)))
            .map(|i| 4 * usize::from(rd16(db, 0x18 + 2 * i)))
            .find(|&offset| rd16(db, offset) == spec.event_id)
            .map(|offset| EventRef { db: info.db, offset })
    }

    /// `iSPCH_GetGlobalMatchParmsArray` 0x829b3428: the array's database and offset.
    fn global_match_parms(&self, spec: EventSpec) -> Option<(usize, usize)> {
        let info = self.find_event_dat_info(spec)?;
        let db = &self.dbs[info.db];
        Some((info.db, 0x18 + align_up(usize::from(rd16(db, 0x10)) * 2)))
    }

    // ---- event queue --------------------------------------------------------------------

    /// `iSPCH_ClearEvent` 0x829b36e0.
    fn clear_event(&mut self, slot: usize) {
        if slot >= EVENT_SLOTS {
            return;
        }
        let item = &mut self.events[slot];
        item.mem_parms = None;
        item.pending = false;
        let channel = usize::from(item.channel);
        if channel >= CHANNELS {
            return;
        }
        if self.last_added_event[channel] == slot as i32 {
            self.last_added_event[channel] = -1;
        }
        self.num_pending[channel] -= 1;
        if self.num_pending[channel] < 0 {
            self.num_pending[channel] = 0;
        }
    }

    /// `iSPCH_FindEventSlot` 0x829b3808: a free slot; when all are pending, the first whose event
    /// expired, else the first of the channel at or below the new event's priority, else none.
    fn find_event_slot(&mut self, priority: u16, channel: u32) -> Option<usize> {
        if let Some(slot) = self.events.iter().position(|item| !item.pending) {
            return Some(slot);
        }
        let tick = self.tick;
        for slot in 0..EVENT_SLOTS {
            let item = &self.events[slot];
            let Some(event) = item.event else { continue };
            let expiry = u32::from(self.event_u16(event, 2));
            if expiry != 0 && tick.wrapping_sub(item.entry_time) > expiry {
                self.clear_event(slot);
                return Some(slot);
            }
        }
        for slot in 0..EVENT_SLOTS {
            let item = &self.events[slot];
            let Some(event) = item.event else { continue };
            if self.event_u16(event, 4) <= priority && u32::from(item.channel) == channel {
                self.events[slot].pending = false;
                self.num_pending[usize::from(self.events[slot].channel)] -= 1;
                return Some(slot);
            }
        }
        None
    }

    /// `iSPCH_AddEvent` 0x829b3928: an event its frequency lets through takes a slot of its
    /// channel, playing at once when its flag 0x20 is set.
    fn add_event(&mut self, parms: Vec<u32>, callback_data: u32) -> bool {
        let mut added = false;
        if !self.add_event_status {
            self.add_event_status = true;
            added = self.add_event_imp(parms, callback_data);
        }
        self.add_event_status = false;
        added
    }

    fn add_event_imp(&mut self, parms: Vec<u32>, callback_data: u32) -> bool {
        let spec = EventSpec::from_word(parms.first().copied().unwrap_or(0));
        let Some(event) = self.find_event(spec) else { return false };
        let Some(info) = self.find_event_dat_info(spec) else { return false };
        let channel = info.channel;
        let frequency = i32::from(self.event_u8(event, 9) as i8);
        if self.random.rand(100, -1) as i32 > frequency {
            return false;
        }
        let Some(slot) = self.find_event_slot(self.event_u16(event, 4), channel) else { return false };
        let tick = self.tick;
        let sub_tick = if tick == self.last_tick { self.last_sub_tick.wrapping_add(1) } else { 0 };
        self.last_sub_tick = sub_tick;
        self.last_tick = tick;
        let item = &mut self.events[slot];
        item.callback_data = callback_data;
        item.event = Some(event);
        item.channel = channel as u8;
        item.sub_ticks = sub_tick;
        item.entry_time = tick;
        item.mem_parms = Some(parms);
        item.pending = true;
        self.num_pending[channel as usize] += 1;
        self.last_added_event[channel as usize] = slot as i32;
        if self.event_u8(event, 0xA) & 0x20 != 0 {
            self.play_last_event(channel);
        }
        true
    }

    /// `SPCH_AddSmartEventV` 0x829b3b88 (`gSPCH_AddSmartEvent`): the event spec and its
    /// arguments as the parameter array.
    pub fn add_smart_event(&mut self, callback_data: u32, spec: EventSpec, args: &[u32]) -> bool {
        let mut parms = Vec::with_capacity(args.len() + 1);
        parms.push(spec.word());
        parms.extend_from_slice(args);
        self.add_event(parms, callback_data)
    }

    /// `SPCH_PlayLastEvent` 0x829b3778.
    fn play_last_event(&mut self, channel: u32) {
        if !self.play_status {
            self.play_status = true;
            if channel < CHANNELS as u32 {
                let slot = self.last_added_event[channel as usize];
                if (0..EVENT_SLOTS as i32).contains(&slot) {
                    self.clear_sentence_choice_channel(channel as usize);
                    let chosen = self.choose_single_sentence(slot as usize);
                    self.clear_event(slot as usize);
                    if chosen != 0 {
                        self.play_chosen(channel as usize);
                    }
                }
            }
        }
        self.play_status = false;
    }

    /// `VoxEvent_GetFollowEventData` 0x829b3180: the events that may follow this one.
    fn follow_event_data(&self, event: EventRef) -> Option<Vec<u16>> {
        let flags = self.event_u8(event, 0xA);
        if flags & 0x10 == 0 {
            return None;
        }
        let db = &self.dbs[event.db];
        let mut at = self.event_layout(event).follow;
        if flags & 8 == 0 {
            at = self.event_layout(event).valid;
        }
        let count = usize::from(rd8(db, at));
        Some((0..count).map(|i| rd16(db, at + 2 + 2 * i)).collect())
    }

    /// `iSPCH_ChooseEventSearch` 0x829b3c50: the channel's pending event (one the follow data
    /// names, when given) of the highest priority, then the oldest, then the latest sub tick;
    /// expired events are cleared on the way.
    fn choose_event_search(&mut self, follow: Option<&[u16]>, channel: u32) -> Option<usize> {
        let tick = self.tick.wrapping_add(self.pre_load_ticks as u32);
        let mut best_priority = 0u16;
        let mut best = None;
        let mut best_age = u32::MAX;
        let mut best_sub_ticks = 0u16;
        for slot in 0..EVENT_SLOTS {
            let item = &self.events[slot];
            if !item.pending || u32::from(item.channel) != channel {
                continue;
            }
            let Some(event) = item.event else { continue };
            let id = self.event_u16(event, 0);
            let listed = follow.is_none_or(|ids| ids.contains(&id));
            let age = tick.wrapping_sub(item.entry_time);
            let expiry = u32::from(self.event_u16(event, 2));
            let expired = expiry != 0 && age > expiry;
            let filtered = self.filter_setting[channel as usize] == 1 && self.event_u8(event, 0xA) & 2 != 0 && {
                let spec = EventSpec::from_word(item.mem_parms.as_ref().and_then(|p| p.first().copied()).unwrap_or(0));
                self.test_event_priority(spec, event)
            };
            if expired {
                self.clear_event(slot);
                continue;
            }
            if listed && !filtered {
                let priority = self.event_u16(event, 4);
                let sub_ticks = self.events[slot].sub_ticks;
                if priority > best_priority || (priority == best_priority && (age < best_age || (age == best_age && sub_ticks > best_sub_ticks))) {
                    best_priority = priority;
                    best_sub_ticks = sub_ticks;
                    best_age = age;
                    best = Some(slot);
                }
            }
        }
        best
    }

    /// `iSPCH_TestEventPriority` 0x829b3530: below the database's filter priority.
    fn test_event_priority(&self, spec: EventSpec, event: EventRef) -> bool {
        self.find_event_dat_info(spec).is_some_and(|info| self.event_u16(event, 4) < rd16(&self.dbs[info.db], 0x16))
    }

    /// `SPCH_Choose` 0x829b3de8: choose a sentence for the channel's next event (preferring
    /// events that may follow the last one played); a chosen event clears the channel's older
    /// pending events without flag 4.
    fn choose(&mut self, channel: u32) -> i32 {
        if channel >= CHANNELS as u32 || self.num_pending[channel as usize] == 0 {
            return 0;
        }
        loop {
            let last = self.in_game[channel as usize].last_event_spec;
            let follow = if last.event_id == 0xFFFF { None } else { self.find_event(last).and_then(|event| self.follow_event_data(event)) };
            let slot = match follow.as_deref().and_then(|ids| self.choose_event_search(Some(ids), channel)) {
                Some(slot) => slot,
                None => match self.choose_event_search(None, channel) {
                    Some(slot) => slot,
                    None => return 0,
                },
            };
            self.clear_sentence_choice_channel(channel as usize);
            let chosen = self.choose_single_sentence(slot);
            if chosen != 0 {
                let (entry_time, sub_ticks, event_channel) = {
                    let item = &self.events[slot];
                    (item.entry_time, item.sub_ticks, item.channel)
                };
                for other in 0..EVENT_SLOTS {
                    if other == slot {
                        continue;
                    }
                    let item = &self.events[other];
                    if !item.pending || item.channel != event_channel {
                        continue;
                    }
                    let older = item.entry_time < entry_time || (item.entry_time == entry_time && item.sub_ticks < sub_ticks);
                    let keep = item.event.is_some_and(|event| self.event_u8(event, 0xA) & 4 != 0);
                    if older && !keep {
                        self.clear_event(other);
                    }
                }
                self.clear_event(slot);
                return chosen;
            }
            self.clear_event(slot);
        }
    }

    /// `SPCH_Play` 0x829b3fb0: play the chosen sentence, or choose one; the phrases requested.
    pub fn play(&mut self, channel: u32) -> i32 {
        if self.play_status {
            self.play_status = false;
            return 0;
        }
        self.play_status = true;
        let played = if self.event_choice.get(channel as usize).is_some_and(|choice| choice.valid_choice) || self.choose(channel) != 0 {
            self.play_chosen(channel as usize)
        } else {
            0
        };
        self.play_status = false;
        played
    }

    // ---- event layout -------------------------------------------------------------------

    fn event_u8(&self, event: EventRef, at: usize) -> u8 {
        rd8(&self.dbs[event.db], event.offset + at)
    }

    fn event_u16(&self, event: EventRef, at: usize) -> u16 {
        rd16(&self.dbs[event.db], event.offset + at)
    }

    /// The blocks after a `VoxEvent`'s header: sentence offsets, rules, the rule bits (two
    /// arrays of `(numRules + 7) / 8` bytes per sentence), the row contexts, the valid-parameter
    /// codes (only read with flag 8) and the follow data.
    fn event_layout(&self, event: EventRef) -> EventLayout {
        let ns = usize::from(self.event_u8(event, 6));
        let nr = usize::from(self.event_u8(event, 7));
        let nrc = usize::from(self.event_u8(event, 8) >> 4);
        let rules = event.offset + 12 + align_up(2 * ns);
        let bits = rules + align_down(3 * (nr + 1));
        let rule_bytes = nr.div_ceil(8);
        let row_contexts = bits + align_up(2 * rule_bytes * ns);
        let valid = row_contexts + align_down(3 * (nrc + 1));
        let follow = valid + ((2 * ns + 7) & !3);
        EventLayout { bits, row_contexts, valid, follow, rule_bytes }
    }

    fn sentence_offset(&self, event: EventRef, sentence: usize) -> usize {
        event.offset + 4 * usize::from(rd16(&self.dbs[event.db], event.offset + 12 + 2 * sentence))
    }

    // ---- sentences ----------------------------------------------------------------------

    /// `iSPCH_ChooseSingleSentence` 0x829b5e68 (no `reparm` callback).
    fn choose_single_sentence(&mut self, slot: usize) -> i32 {
        let Some(parms) = self.events[slot].mem_parms.clone() else { return 0 };
        let callback_data = self.events[slot].callback_data;
        self.choose_sentence(&parms, callback_data).max(0)
    }

    /// `iSPCH_ChooseSentence` 0x829b5bb8: the sentences in weighted random order; the first
    /// whose frequency draw, contexts, rules and valid parameters pass and whose phrases find
    /// samples is chosen. -1 for an event without sentences.
    fn choose_sentence(&mut self, parms: &[u32], callback_data: u32) -> i32 {
        let spec = EventSpec::from_word(parms.first().copied().unwrap_or(0));
        let (Some(event), Some(info)) = (self.find_event(spec), self.find_event_dat_info(spec)) else { return 0 };
        let channel = info.channel;
        let mut filter_setting = self.filter_setting[channel as usize];
        let mut filter_length = 0;
        if filter_setting == 1 {
            if self.event_u8(event, 0xA) & 1 != 0 {
                filter_length = i32::from(rd16(&self.dbs[info.db], 0x14));
            } else {
                filter_setting = 0;
            }
        }
        let (event_settings, user_mask) = self.rule_settings(event);
        let order = self.order_sentences(event);
        let num_sentences = usize::from(self.event_u8(event, 6));
        if num_sentences == 0 {
            return -1;
        }
        let check_valid = self.event_u8(event, 0xA) & 8 != 0;
        let valid_mask = if check_valid { self.make_valid_parms_mask(event, parms) } else { 0 };
        let layout = self.event_layout(event);
        let mut choice = 0;
        for &sentence_num in order.iter().take(num_sentences) {
            if choice != 0 {
                break;
            }
            let sentence_num = usize::from(sentence_num);
            if sentence_num >= num_sentences {
                break;
            }
            let sentence = self.sentence_offset(event, sentence_num);
            let frequency = i32::from(rd8(&self.dbs[event.db], sentence + 1));
            if (self.random.rand(100, -1) as i32) < frequency
                && self.sentence_is_context_match(event, sentence, parms)
                && self.check_sentence_rules(event, sentence_num, event_settings, user_mask)
                && (!check_valid || rd16(&self.dbs[event.db], layout.valid + 4 + 2 * sentence_num) == valid_mask)
            {
                let mut info = SentencePickInfo::new();
                if self.sentence_get_choices(&mut info, event.db, sentence, parms, filter_setting) {
                    choice = self.sentence_make_choice(spec, &mut info, event.db, sentence, filter_length);
                    if choice != 0 {
                        self.save_chosen_sentence(&info, channel as usize, event, sentence, sentence_num as u8, parms, callback_data);
                    }
                }
            }
        }
        choice
    }

    /// `iSPCH_OrderSentences` 0x829b4920: weights `4^(w >> 5) * (w & 31)` drawn without
    /// replacement, then the zero-weight sentences in order.
    fn order_sentences(&mut self, event: EventRef) -> Vec<u8> {
        let num_sentences = usize::from(self.event_u8(event, 6));
        let mut weights: Vec<i32> = (0..num_sentences)
            .map(|s| {
                let w = rd8(&self.dbs[event.db], self.sentence_offset(event, s));
                MULTIPLE[usize::from((w >> 5) & 7)] * i32::from(w & 0x1F)
            })
            .collect();
        let mut total: i32 = weights.iter().copied().fold(0, i32::wrapping_add);
        let mut order = Vec::with_capacity(num_sentences);
        while total > 0 {
            let mut r = self.random.rand(total, -1) as i32;
            let mut pick = 0;
            while pick < num_sentences {
                r -= weights[pick];
                if r < 0 {
                    break;
                }
                pick += 1;
            }
            order.push(pick as u8);
            let weight = weights.get(pick).copied().unwrap_or(0);
            if let Some(w) = weights.get_mut(pick) {
                *w = 0;
            }
            total -= weight;
        }
        for s in 0..num_sentences {
            let w = rd8(&self.dbs[event.db], self.sentence_offset(event, s));
            if MULTIPLE[usize::from((w >> 5) & 7)] * i32::from(w & 0x1F) == 0 {
                order.push(s as u8);
            }
        }
        order
    }

    /// `iSPCH_SentenceIsContextMatch` 0x829b5ad8: each of the sentence's context masks holds a
    /// bit of the parameter its row context names.
    fn sentence_is_context_match(&self, event: EventRef, sentence: usize, parms: &[u32]) -> bool {
        let db = &self.dbs[event.db];
        let rows = usize::from(self.event_u8(event, 8) >> 4);
        let masks = sentence + 8 + align_up(usize::from(rd8(db, sentence + 2) >> 2));
        let row_contexts = self.event_layout(event).row_contexts;
        (0..rows).all(|i| {
            let mask = rd32(db, masks + 4 * i);
            let parm = parm(parms, usize::from(rd8(db, row_contexts + 3 * i + 1)));
            mask == 0 || parm & mask != 0
        })
    }

    /// `iSPCH_GetRuleSettings` 0x829b6cb0 with no `testRule`: every rule's bit in the user mask,
    /// no event setting. Rule `i`'s bit is `slw 1, 31 - i`: no bit when the shift's 0x20 bit is
    /// set, and the low five bits otherwise.
    fn rule_settings(&self, event: EventRef) -> (u32, u32) {
        let num_rules = u32::from(self.event_u8(event, 7));
        let mut user_mask = 0u32;
        for i in 0..num_rules {
            let shift = 31u32.wrapping_sub(i) & 0x3F;
            if shift & 0x20 == 0 {
                user_mask |= 1 << shift;
            }
        }
        (0, user_mask)
    }

    /// `iSPCH_CheckSentenceRules` 0x829b6e08.
    fn check_sentence_rules(&self, event: EventRef, sentence_num: usize, event_settings: u32, user_mask: u32) -> bool {
        let layout = self.event_layout(event);
        let db = &self.dbs[event.db];
        let ns = usize::from(self.event_u8(event, 6));
        let bytes = layout.rule_bytes;
        let (mut values, mut applies) = (0u32, 0u32);
        if bytes <= 4 && bytes > 0 {
            let first = layout.bits + bytes * sentence_num;
            let second = layout.bits + bytes * (sentence_num + ns);
            for i in 0..bytes {
                let shift = 24 - 8 * i as u32;
                values = values.wrapping_add(u32::from(rd8(db, first + i)) << shift);
                applies = applies.wrapping_add(u32::from(rd8(db, second + i)) << shift);
            }
        }
        values & applies & !user_mask == applies & !user_mask & event_settings
    }

    /// `iSPCH_MakeValidParmsMask` 0x829b5948: bit `k - 1` for each of the first sixteen
    /// parameters whose two-bit code is 1 (valid unless 0) or 2 (valid unless -1).
    fn make_valid_parms_mask(&self, event: EventRef, parms: &[u32]) -> u16 {
        let codes = rd32(&self.dbs[event.db], self.event_layout(event).valid);
        let mut mask = 0u16;
        for k in 1..=16usize {
            let code = (codes >> (2 * (k - 1))) & 3;
            let value = parm(parms, k);
            if (code == 1 && value != 0) || (code == 2 && value != u32::MAX) {
                mask |= 1 << (k - 1);
            }
        }
        mask
    }

    fn sentence_io(&self, db: usize, sentence: usize) -> usize {
        let bytes = &self.dbs[db];
        sentence + 8 + align_up(usize::from(rd8(bytes, sentence + 2) >> 2)) + align_up(4 * usize::from(rd8(bytes, sentence + 4)))
    }

    fn phrase_offset(&self, db: usize, sentence: usize, phrase: usize) -> usize {
        sentence + 4 * usize::from(rd8(&self.dbs[db], sentence + 8 + phrase))
    }

    /// `iSPCH_InitMatchParmIO` 0x829b4a68.
    fn init_match_parm_io(&mut self, db: usize, sentence: usize) {
        let io = self.sentence_io(db, sentence);
        let count = usize::from(rd8(&self.dbs[db], sentence + 3));
        for i in 0..count {
            wr32(&mut self.dbs[db], io + 4 * i, 0);
        }
    }

    /// `iSPCH_SentenceGetChoices` 0x829b4c70: each phrase's bank and matching samples, phrases
    /// waiting on a match parameter another phrase posts taking later passes (at most 200).
    fn sentence_get_choices(&mut self, info: &mut SentencePickInfo, db: usize, sentence: usize, parms: &[u32], filter_setting: i32) -> bool {
        let bit_field = rd8(&self.dbs[db], sentence + 2);
        let num_phrases = usize::from(bit_field >> 2);
        if num_phrases > MAX_PHRASES {
            return false;
        }
        let allowed = match bit_field & 3 {
            0 => true,
            1 => filter_setting == 2,
            2 => filter_setting != 2,
            _ => false,
        };
        if !allowed {
            return false;
        }
        self.init_match_parm_io(db, sentence);
        info.num_phrases = num_phrases as u8;
        let io = self.sentence_io(db, sentence);
        let mut passes = 200;
        loop {
            if passes <= 0 {
                return false;
            }
            passes -= 1;
            let mut all_done = true;
            for phrase_num in 0..num_phrases {
                let phrase = self.phrase_offset(db, sentence, phrase_num);
                let num_filters = rd8(&self.dbs[db], phrase + 4) as i8;
                let waiting = (0..num_filters.max(0) as usize).any(|f| {
                    let filter = phrase + 8 + 8 * f;
                    let bytes = &self.dbs[db];
                    let match_parm = rd8(bytes, filter + 6);
                    rd8(bytes, filter + 4) == 255 && match_parm != 255 && match_parm & 0x80 != 0 && rd32(bytes, io + 4 * usize::from(match_parm & 0x7F)) == 0
                });
                if waiting {
                    all_done = false;
                    continue;
                }
                if !info.phrase_info[phrase_num].done {
                    info.phrase_info[phrase_num].pick_start = info.pick_index;
                    if !self.get_phrase_bank(db, phrase, parms, &mut info.phrase_info[phrase_num]) {
                        return false;
                    }
                    let picks = self.choose_samples(info, db, sentence, phrase_num, phrase, parms);
                    if picks == 0 {
                        return false;
                    }
                    info.phrase_info[phrase_num].done = true;
                    info.phrase_info[phrase_num].num_picks = picks;
                }
            }
            if all_done {
                return passes != 0;
            }
        }
    }

    /// `iSPCH_GetPhraseBank` 0x829b4508: bank type 0 by id, 1 by id and sub id (the parameter
    /// `bankIDIndex` names), 2 by id with that parameter as the sub bank.
    fn get_phrase_bank(&self, db: usize, phrase: usize, parms: &[u32], pick: &mut PhrasePickInfo) -> bool {
        let bytes = &self.dbs[db];
        let bank_id = rd16(bytes, phrase);
        let id_index = usize::from(rd8(bytes, phrase + 2));
        let bank_type = rd8(bytes, phrase + 3);
        pick.sub_bank_index = u32::MAX;
        pick.bank_handle = -1;
        pick.bank_index = -1;
        match bank_type {
            0 => {
                let (index, handle) = self.find_bank(bank_id);
                pick.bank_handle = i32::from(handle as i16);
                pick.bank_index = index;
            }
            1 => {
                let (index, handle) = self.find_sub_bank(bank_id, parm(parms, id_index));
                pick.bank_handle = handle;
                pick.bank_index = index;
            }
            2 => {
                let (index, handle) = self.find_bank(bank_id);
                let sub = parm(parms, id_index);
                let mut found = -1;
                if self.test_sub_bank_bounds(i32::from(index), sub as i32) {
                    found = handle;
                    pick.sub_bank_index = sub;
                }
                pick.bank_handle = found;
                pick.bank_index = index;
            }
            _ => return false,
        }
        pick.bank_handle >= 0
    }

    /// `iSPCH_ChooseSamples` 0x829b46f8: the bank's samples whose parameters match the phrase,
    /// that exist in its sub bank and are not in the repeat queue (the one queued longest ago when
    /// all are); a filter posting a match parameter posts each pick's value.
    fn choose_samples(&mut self, info: &mut SentencePickInfo, db: usize, sentence: usize, phrase_num: usize, phrase: usize, parms: &[u32]) -> u8 {
        let num_filters = rd8(&self.dbs[db], phrase + 4) as i8;
        let posts = (0..num_filters.max(0) as usize).any(|f| rd8(&self.dbs[db], phrase + 8 + 8 * f + 4) == 254);
        let Some(hdr) = self.bank_hdr_index(info.phrase_info[phrase_num].bank_index) else { return 0 };
        let (num_samples, parm_flags) = {
            let h = &self.bank_hdrs[hdr];
            (usize::from(rd8(h, 3)), rd8(h, 2))
        };
        let num_parms = usize::from(parm_flags & 0x7F);
        let masks = bank_table_end(num_parms, num_samples);
        for j in 0..num_parms {
            let filter_parm = usize::from(rd8(&self.dbs[db], phrase + 8 + 8 * j + 4));
            if filter_parm != 0 && filter_parm != 255 && filter_parm != 254 && parm(parms, filter_parm) & rd32(&self.bank_hdrs[hdr], masks + 4 * j) == 0 {
                return 0;
            }
        }
        let record = num_parms + 2;
        let mut picks = 0u8;
        let mut oldest_time = 0;
        let mut oldest = -1i32;
        for sample in 0..num_samples {
            let sample_parms = 18 + record * sample;
            let parm_values = self.bank_hdrs[hdr][sample_parms.min(self.bank_hdrs[hdr].len())..].to_vec();
            if self.match_sample(db, sentence, phrase, parms, &parm_values) && self.sample_exists(&info.phrase_info[phrase_num], hdr, sample as i32) {
                let time = sample_time_in_queue(&self.bank_hdrs[hdr], sample as u8);
                if time <= 0 {
                    if info.pick_index != 255 {
                        picks += 1;
                        info.valid_samples[usize::from(info.pick_index)] = sample as u8;
                        info.pick_index += 1;
                    }
                    if posts {
                        self.post_match_parm_value(db, sentence, phrase, &parm_values);
                    }
                } else if time > oldest_time {
                    oldest_time = time;
                    oldest = sample as i32;
                }
            }
        }
        if picks == 0 && oldest != -1 {
            if info.pick_index != 255 {
                picks = 1;
                info.valid_samples[usize::from(info.pick_index)] = oldest as u8;
                info.pick_index += 1;
            }
            if posts {
                let at = 18 + record * oldest as usize;
                let parm_values = self.bank_hdrs[hdr][at.min(self.bank_hdrs[hdr].len())..].to_vec();
                self.post_match_parm_value(db, sentence, phrase, &parm_values);
            }
        }
        picks
    }

    /// `iSPCH_MatchSample` 0x829b43d0: every filter's mask holds the sample's value, which the
    /// event parameter, the sentence's match parameter or the global match parameter it names
    /// must also hold.
    fn match_sample(&self, db: usize, sentence: usize, phrase: usize, parms: &[u32], sample_parms: &[u8]) -> bool {
        let bytes = &self.dbs[db];
        let num_filters = rd8(bytes, phrase + 4) as i8;
        let io = self.sentence_io(db, sentence);
        for f in 0..num_filters.max(0) as usize {
            let value = u32::from(sample_parms.get(f).copied().unwrap_or(0));
            if value >= 32 {
                return false;
            }
            let bit = 1u32 << value;
            let filter = phrase + 8 + 8 * f;
            if rd32(bytes, filter) & bit == 0 {
                return false;
            }
            let filter_parm = rd8(bytes, filter + 4);
            if filter_parm == 0 || filter_parm == 254 {
                continue;
            }
            if filter_parm != 255 {
                if parm(parms, usize::from(filter_parm)) & bit == 0 {
                    return false;
                }
                continue;
            }
            let match_parm = rd8(bytes, filter + 6);
            if match_parm & 0x80 != 0 {
                if rd32(bytes, io + 4 * usize::from(match_parm & 0x7F)) & bit == 0 {
                    return false;
                }
            } else {
                let spec = EventSpec::from_word(parm(parms, 0));
                let Some((global_db, start)) = self.global_match_parms(spec) else { return false };
                if value != u32::from(rd8(&self.dbs[global_db], start + usize::from(match_parm))) {
                    return false;
                }
            }
        }
        true
    }

    /// `iSPCH_PostMatchParmValue` 0x829b4340.
    fn post_match_parm_value(&mut self, db: usize, sentence: usize, phrase: usize, sample_parms: &[u8]) {
        let io = self.sentence_io(db, sentence);
        let num_filters = rd8(&self.dbs[db], phrase + 4) as i8;
        for f in 0..num_filters.max(0) as usize {
            let filter = phrase + 8 + 8 * f;
            if rd8(&self.dbs[db], filter + 4) != 254 {
                continue;
            }
            let match_parm = rd8(&self.dbs[db], filter + 6);
            if match_parm & 0x80 != 0 {
                let at = io + 4 * usize::from(match_parm & 0x7F);
                let value = rd32(&self.dbs[db], at) | 1u32.wrapping_shl(u32::from(sample_parms.get(f).copied().unwrap_or(0)));
                wr32(&mut self.dbs[db], at, value);
            }
        }
    }

    /// `iSPCH_SampleExists` 0x829b45f0: in range, in the sub bank's sample bits, and in the
    /// current cycle when the bank cycles.
    fn sample_exists(&self, pick: &PhrasePickInfo, hdr: usize, sample: i32) -> bool {
        let h = &self.bank_hdrs[hdr];
        let num_samples = i32::from(rd8(h, 3));
        if sample > num_samples {
            return false;
        }
        if pick.sub_bank_index != u32::MAX {
            if u32::from(rd16(h, 12)) <= pick.sub_bank_index {
                return false;
            }
            let bit = (pick.sub_bank_index as i32).wrapping_mul(num_samples) + sample;
            if rd8(h, array_bank_bits(h) + (bit / 8) as usize) & (1 << (bit % 8)) == 0 {
                return false;
            }
        }
        let parm_flags = rd8(h, 2);
        if parm_flags & 0x80 != 0 {
            let cycle = cycle_base(h);
            return rd8(h, cycle + 1 + (sample / 8) as usize) & (1 << (sample % 8)) != 0;
        }
        true
    }

    /// `iSPCH_SentenceMakeChoice` 0x829b50f0: with a filter length or match parameters each
    /// phrase's picks are shuffled and combinations tried in order; otherwise each phrase picks
    /// at random, avoiding its bank's recent picks.
    fn sentence_make_choice(&mut self, spec: EventSpec, info: &mut SentencePickInfo, db: usize, sentence: usize, filter_length: i32) -> i32 {
        let num_phrases = usize::from(rd8(&self.dbs[db], sentence + 2) >> 2);
        let chosen = if filter_length > 0 || rd8(&self.dbs[db], sentence + 3) != 0 {
            for phrase in &info.phrase_info[..num_phrases] {
                let start = usize::from(phrase.pick_start);
                let mut n = u32::from(phrase.num_picks);
                while n > 1 {
                    let r = self.random.rand(n as i32, -1) as usize;
                    info.valid_samples.swap(start + n as usize - 1, start + r);
                    n -= 1;
                }
            }
            self.choose_sentence_iteratively(info, db, sentence, filter_length)
        } else {
            for phrase in &mut info.phrase_info[..num_phrases] {
                if phrase.num_picks == 0 {
                    return 0;
                }
                let r = self.random.rand(i32::from(phrase.num_picks), i32::from(phrase.bank_index));
                phrase.picked_index = phrase.pick_start.wrapping_add(r as u8);
            }
            1
        };
        self.post_global_match_parms(spec, info, db, sentence);
        chosen
    }

    /// `iSPCH_ChooseSentenceIteratively` 0x829b4e30: the first combination (odometer order,
    /// last phrase fastest) within the filter length whose match parameters hold.
    fn choose_sentence_iteratively(&mut self, info: &mut SentencePickInfo, db: usize, sentence: usize, filter_length: i32) -> i32 {
        let num_phrases = usize::from(rd8(&self.dbs[db], sentence + 2) >> 2);
        for phrase in &mut info.phrase_info[..num_phrases] {
            phrase.picked_index = phrase.pick_start;
        }
        loop {
            let mut ok = true;
            if filter_length != 0 {
                let mut total = 0i32;
                for phrase in &info.phrase_info[..num_phrases] {
                    if let Some(hdr) = self.bank_hdr_index(phrase.bank_index) {
                        if let Some((_, bytes)) = sample_size_data(&self.bank_hdrs[hdr], i32::from(info.valid_samples[usize::from(phrase.picked_index)])) {
                            total = total.wrapping_add(bytes as i32);
                        }
                    }
                }
                let length = if self.data_rate != 0 { total.wrapping_mul(100) / self.data_rate } else { 0 };
                if length > filter_length {
                    ok = false;
                }
            }
            if ok && rd8(&self.dbs[db], sentence + 3) != 0 && !self.test_match_parms(info, db, sentence) {
                ok = false;
            }
            if ok {
                return 1;
            }
            if num_phrases == 0 {
                return 0;
            }
            let mut index = num_phrases as i32 - 1;
            let mut exhausted = false;
            loop {
                let phrase = &mut info.phrase_info[index as usize];
                phrase.picked_index = phrase.picked_index.wrapping_add(1);
                if u32::from(phrase.picked_index) < u32::from(phrase.num_picks) + u32::from(phrase.pick_start) {
                    break;
                }
                phrase.picked_index = phrase.pick_start;
                index -= 1;
                if index < 0 {
                    exhausted = true;
                    break;
                }
            }
            if exhausted {
                return 0;
            }
        }
    }

    /// `iSPCH_TestMatchParms` 0x829b4ac8: the picks post their sentence match parameters and the
    /// phrases reading them must find their sample's value set.
    fn test_match_parms(&mut self, info: &mut SentencePickInfo, db: usize, sentence: usize) -> bool {
        let io = self.sentence_io(db, sentence);
        self.init_match_parm_io(db, sentence);
        for phrase in &mut info.phrase_info {
            phrase.done = false;
        }
        let num_phrases = usize::from(rd8(&self.dbs[db], sentence + 2) >> 2);
        loop {
            let mut resolved = true;
            if num_phrases == 0 {
                break;
            }
            for phrase_num in 0..num_phrases {
                if info.phrase_info[phrase_num].done {
                    continue;
                }
                info.phrase_info[phrase_num].done = true;
                let phrase = self.phrase_offset(db, sentence, phrase_num);
                let num_filters = rd8(&self.dbs[db], phrase + 4) as i8;
                let Some(hdr) = self.bank_hdr_index(info.phrase_info[phrase_num].bank_index) else { return false };
                let sample = info.valid_samples[usize::from(info.phrase_info[phrase_num].picked_index)];
                let Some(sample_parms) = sample_parm_addr(&self.bank_hdrs[hdr], i32::from(sample)) else { return false };
                let sample_parms = self.bank_hdrs[hdr][sample_parms..].to_vec();
                for f in 0..num_filters.max(0) as usize {
                    let filter = phrase + 8 + 8 * f;
                    let match_parm = rd8(&self.dbs[db], filter + 6);
                    if match_parm == 255 || match_parm & 0x80 == 0 {
                        continue;
                    }
                    let at = io + 4 * usize::from(match_parm & 0x7F);
                    let value = 1u32.wrapping_shl(u32::from(sample_parms.get(f).copied().unwrap_or(0)));
                    match rd8(&self.dbs[db], filter + 4) {
                        255 => {
                            let word = rd32(&self.dbs[db], at);
                            if word == 0 {
                                resolved = false;
                                info.phrase_info[phrase_num].done = false;
                            } else if value & word == 0 {
                                return false;
                            }
                        }
                        254 => wr32(&mut self.dbs[db], at, value),
                        _ => {}
                    }
                }
            }
            if resolved {
                break;
            }
        }
        true
    }

    /// `iSPCH_PostGlobalMatchParms` 0x829b4ff8: the picks' values for the global match
    /// parameters their filters post.
    fn post_global_match_parms(&mut self, spec: EventSpec, info: &SentencePickInfo, db: usize, sentence: usize) {
        let Some((global_db, start)) = self.global_match_parms(spec) else { return };
        let num_phrases = usize::from(rd8(&self.dbs[db], sentence + 2) >> 2);
        for (phrase_num, pick) in info.phrase_info.iter().enumerate().take(num_phrases) {
            let phrase = self.phrase_offset(db, sentence, phrase_num);
            let num_filters = rd8(&self.dbs[db], phrase + 4) as i8;
            for f in 0..num_filters.max(0) as usize {
                let filter = phrase + 8 + 8 * f;
                let match_parm = rd8(&self.dbs[db], filter + 6);
                if rd8(&self.dbs[db], filter + 4) != 254 || match_parm & 0x80 != 0 {
                    continue;
                }
                let Some(hdr) = self.bank_hdr_index(pick.bank_index) else { continue };
                let sample = info.valid_samples[usize::from(pick.picked_index)];
                if let Some(at) = sample_parm_addr(&self.bank_hdrs[hdr], i32::from(sample)) {
                    let value = rd8(&self.bank_hdrs[hdr], at + f);
                    wr8(&mut self.dbs[global_db], start + usize::from(match_parm), value);
                }
            }
        }
    }

    /// `iSPCH_SaveChosenSentence` 0x829b5718.
    #[allow(clippy::too_many_arguments)]
    fn save_chosen_sentence(&mut self, info: &SentencePickInfo, channel: usize, event: EventRef, sentence: usize, sentence_num: u8, parms: &[u32], callback_data: u32) {
        let num_parms = self.event_u8(event, 0xB);
        let mut saved = Vec::with_capacity(usize::from(num_parms) + 1);
        if num_parms < 0x80 {
            for i in 0..=usize::from(num_parms) {
                saved.push(parm(parms, i));
            }
        }
        let num_phrases = usize::from(rd8(&self.dbs[event.db], sentence + 2) >> 2);
        let choice = &mut self.event_choice[channel];
        choice.mem_parms = Some(saved);
        choice.event = Some(event);
        choice.callback_data = callback_data;
        choice.sentence = Some(sentence);
        choice.sentence_num = sentence_num;
        choice.valid_choice = true;
        for (slot, pick) in choice.phrases.iter_mut().zip(&info.phrase_info).take(num_phrases) {
            *slot = PhraseChoice {
                bank_index: pick.bank_index,
                bank_handle: pick.bank_handle,
                sub_bank_index: pick.sub_bank_index,
                sample_index: u16::from(info.valid_samples[usize::from(pick.picked_index)]),
            };
        }
        choice.num_phrases = num_phrases as u8;
    }

    /// `iSPCH_ClearSentenceChoiceChannel` 0x829b55f0.
    fn clear_sentence_choice_channel(&mut self, channel: usize) {
        if let Some(choice) = self.event_choice.get_mut(channel) {
            *choice = EventChoice::default();
        }
    }

    /// `iSPCH_PlayChosen` 0x829b5860: the chosen sentence's sample requests (`iSPCH_RuleSet` and
    /// `iSPCH_ConstantRuleSet` do nothing without `setRule`); the channel remembers the event
    /// and how many times in a row it played.
    fn play_chosen(&mut self, channel: usize) -> i32 {
        let mut played = 0;
        if channel < CHANNELS && self.event_choice[channel].valid_choice {
            let choice = self.event_choice[channel].clone();
            let spec = EventSpec::from_word(choice.mem_parms.as_ref().and_then(|p| p.first().copied()).unwrap_or(0));
            if let (Some(event), Some(sentence)) = (choice.event, choice.sentence) {
                self.make_sample_requests(&choice, event, sentence, spec);
                played = i32::from(rd8(&self.dbs[event.db], sentence + 2) >> 2);
            }
            let in_game = &mut self.in_game[channel];
            if spec.event_id == in_game.last_event_spec.event_id {
                in_game.num_event_times += 1;
            } else {
                in_game.last_event_spec = spec;
                in_game.num_event_times = 1;
            }
        }
        self.clear_sentence_choice_channel(channel);
        played
    }

    /// `iSPCH_MakeSampleRequests` 0x829b5358: a request per phrase (a sub bank's samples after
    /// the earlier sub banks' blocks), the sample leaving the bank's cycle and entering its
    /// repeat queue; the first carries the event's interrupt flag (0x20).
    fn make_sample_requests(&mut self, choice: &EventChoice, event: EventRef, sentence: usize, spec: EventSpec) -> i32 {
        let Some(info) = self.find_event_dat_info(spec) else { return 0 };
        let channel = info.channel;
        let dat_id = i32::from(rd8(&self.dbs[info.db], 9));
        let num_phrases = usize::from(rd8(&self.dbs[event.db], sentence + 2) >> 2);
        let mut phrases = choice.phrases;
        for phrase in &mut phrases[..num_phrases] {
            if self.banks.get(phrase.bank_index as usize).map(|bank| bank.handle) != Some(phrase.bank_handle) || phrase.bank_index < 0 {
                let index = self.find_bank_index_from_handle(phrase.bank_handle);
                phrase.bank_index = index as i16;
                if index < 0 {
                    return 0;
                }
            }
        }
        self.event_choice[channel as usize].phrases = phrases;
        let flags = self.event_u8(event, 0xA);
        let mut total = 0i32;
        for (index, phrase) in phrases.iter().enumerate().take(num_phrases) {
            let Some(hdr) = self.bank_hdr_index(phrase.bank_index) else { continue };
            let sample = i32::from(phrase.sample_index);
            let h = &mut self.bank_hdrs[hdr];
            if rd8(h, 2) & 0x80 != 0 && CLEAR_CYCLE {
                let at = cycle_base(h) + 1 + (sample / 8) as usize;
                let byte = rd8(h, at) & !(1u8 << (sample % 8));
                wr8(h, at, byte);
            }
            let Some((offset, bytes)) = sample_size_data(h, sample) else { continue };
            let mut sample_offset = offset as i32;
            if phrase.sub_bank_index != u32::MAX {
                let blocks = (i32::from(rd8(h, 9)) + 1).wrapping_mul(i32::from(rd16(h, 10))).wrapping_mul(phrase.sub_bank_index as i32);
                sample_offset = (blocks << 8).wrapping_add(offset as i32);
            }
            total = total.wrapping_add(bytes as i32);
            self.requests.push(SampleRequestData {
                bank_num: phrase.bank_handle,
                sample_offset,
                num_bytes: bytes as i32,
                event_spec: spec,
                channel: channel as i32,
                sub_id: rd32(h, 4) as i32,
                dat_id,
                interrupt_flag: if index == 0 { i32::from((flags >> 5) & 1) } else { 0 },
                callback_data: self.event_choice[channel as usize].callback_data,
            });
            add_to_queue(h, sample as u8);
        }
        if self.data_rate != 0 {
            total.wrapping_mul(100) / self.data_rate
        } else {
            0
        }
    }

    // ---- banks ----------------------------------------------------------------------------

    fn bank_hdr_index(&self, bank_index: i16) -> Option<usize> {
        self.banks.get(usize::try_from(bank_index).ok()?)?.hdr
    }

    /// `SPCH_AddBank` 0x829b6988: the bank inserted in (type, sub id) order with the next unique
    /// handle; a cycling bank first marks the samples of this game's cycle.
    pub fn add_bank(&mut self, mut hdr: Vec<u8>) -> i32 {
        let position = self.find_insert_position(&hdr);
        if position < 0 || position >= MAX_BANKS as i32 {
            return -1;
        }
        if rd8(&hdr, 2) & 0x80 != 0 {
            set_cycle_bits(&mut hdr, self.game_num);
        }
        let position = position as usize;
        self.bank_hdrs.push(hdr);
        let hdr = self.bank_hdrs.len() - 1;
        let count = self.bank_count as usize;
        for i in (position..count).rev() {
            self.banks[i + 1] = self.banks[i];
        }
        let handle = self.unique_bank_handle;
        self.banks[position] = VoxBankInfo { handle, hdr: Some(hdr) };
        self.bank_count += 1;
        self.unique_bank_handle += 1;
        handle
    }

    fn bank_key(&self, index: usize) -> Option<(u16, u32)> {
        let h = &self.bank_hdrs[self.banks.get(index)?.hdr?];
        Some((rd16(h, 0), rd32(h, 4)))
    }

    /// `iSPCH_FindInsertPosition` 0x829b6350 (-1 for a bank already added).
    fn find_insert_position(&self, hdr: &[u8]) -> i32 {
        let (ty, sub) = (rd16(hdr, 0), rd32(hdr, 4));
        let mut lo = 0i32;
        let mut hi = self.bank_count - 1;
        if self.bank_count == 0 {
            return 0;
        }
        while hi >= 0 {
            let mid = (hi + lo) / 2;
            let Some((mt, msub)) = self.bank_key(mid as usize) else { return -1 };
            if ty == mt && sub == msub {
                return -1;
            }
            if mid == 0 {
                let Some((t0, s0)) = self.bank_key(0) else { return -1 };
                if ty < t0 || (ty == t0 && sub < s0) {
                    return 0;
                }
            }
            let below = ty < mt || (ty == mt && sub < msub);
            if below {
                let Some((pt, ps)) = self.bank_key(mid as usize - 1) else { return -1 };
                if ty > pt || (ty == pt && sub > ps) {
                    return mid;
                }
            }
            if ty > mt || (ty == mt && sub > msub) {
                lo = mid + 1;
                if self.banks.get(lo as usize).is_none_or(|bank| bank.hdr.is_none()) {
                    return lo;
                }
            } else {
                hi = mid - 1;
            }
            if lo > hi {
                return -1;
            }
        }
        -1
    }

    /// `iSPCH_FindBank` 0x829b6498: a bank of the type (binary search), and its handle.
    fn find_bank(&self, bank_type: u16) -> (i16, i32) {
        let (mut lo, mut hi) = (0i32, self.bank_count - 1);
        if self.bank_count == 0 || hi < 0 {
            return (-1, -1);
        }
        loop {
            let mid = (hi + lo) / 2;
            let Some((ty, _)) = self.bank_key(mid as usize) else { return (-1, -1) };
            if bank_type == ty {
                return (mid as i16, self.banks[mid as usize].handle);
            }
            if bank_type <= ty {
                hi = mid - 1;
            } else {
                lo = mid + 1;
            }
            if lo > hi {
                return (-1, -1);
            }
        }
    }

    /// `iSPCH_FindSubBank` 0x829b6528.
    fn find_sub_bank(&self, bank_id: u16, sub_id: u32) -> (i16, i32) {
        let (mut lo, mut hi) = (0i32, self.bank_count - 1);
        if self.bank_count == 0 || hi < 0 {
            return (-1, -1);
        }
        loop {
            let mid = (hi + lo) / 2;
            let Some((ty, sub)) = self.bank_key(mid as usize) else { return (-1, -1) };
            if bank_id == ty && sub_id == sub {
                return (mid as i16, self.banks[mid as usize].handle);
            }
            if bank_id > ty || (bank_id == ty && sub_id > sub) {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
            if lo > hi {
                return (-1, -1);
            }
        }
    }

    /// `iSPCH_TestSubBankBounds` 0x829b65e8.
    fn test_sub_bank_bounds(&self, bank_index: i32, sub_bank_index: i32) -> bool {
        let Ok(index) = usize::try_from(bank_index) else { return false };
        let Some(hdr) = self.banks.get(index).and_then(|bank| bank.hdr) else { return false };
        let num_sub_banks = rd16(&self.bank_hdrs[hdr], 12);
        num_sub_banks != 0xFFFF && sub_bank_index >= 0 && i32::from(num_sub_banks) > sub_bank_index
    }

    /// `iSPCH_FindBankIndexFromHandle` 0x829b6638.
    fn find_bank_index_from_handle(&self, handle: i32) -> i32 {
        (0..self.bank_count as usize).find(|&i| self.banks[i].handle == handle).map_or(-1, |i| i as i32)
    }
}

/// The blocks of a `VoxEvent` (offsets in its database).
struct EventLayout {
    bits: usize,
    row_contexts: usize,
    valid: usize,
    follow: usize,
    rule_bytes: usize,
}

/// Parameter `index` of an event's parameter array (0 past its end).
fn parm(parms: &[u32], index: usize) -> u32 {
    parms.get(index).copied().unwrap_or(0)
}

/// The end of a bank header's sample table: each sample's block offset and parameter values.
fn bank_table_end(num_parms: usize, num_samples: usize) -> usize {
    ((num_parms + 2) * num_samples + 19) & !3
}

/// The cycle count and bits of a bank (`parmFlags & 0x80`), after the parameter masks.
fn cycle_base(hdr: &[u8]) -> usize {
    let num_parms = usize::from(rd8(hdr, 2) & 0x7F);
    bank_table_end(num_parms, usize::from(rd8(hdr, 3))) + 4 * num_parms
}

/// The repeat queue (`sampleRepeat` entries after its index byte), after the cycle bits.
fn queue_base(hdr: &[u8]) -> usize {
    let num_samples = usize::from(rd8(hdr, 3));
    let cycle = if rd8(hdr, 2) & 0x80 != 0 { num_samples.div_ceil(8) + 1 } else { 0 };
    cycle_base(hdr) + cycle
}

/// `BANKHDR_GetArrayBankBitsAddr` 0x829b42c0: the sub banks' sample bits.
fn array_bank_bits(hdr: &[u8]) -> usize {
    let repeat = usize::from(rd8(hdr, 8));
    queue_base(hdr) + if repeat != 0 { repeat + 1 } else { 0 }
}

/// `iSPCH_GetSampleParmAddr` 0x829b6a70.
fn sample_parm_addr(hdr: &[u8], sample: i32) -> Option<usize> {
    let num_samples = i32::from(rd8(hdr, 3));
    (sample < num_samples).then(|| 18 + (usize::from(rd8(hdr, 2) & 0x7F) + 2) * sample as usize)
}

/// `iSPCH_GetSampleSizeData` 0x829b6aa0: the sample's byte offset and size in blocks of
/// `(blockSize + 1) * 256` bytes, the last running to the bank's end.
fn sample_size_data(hdr: &[u8], sample: i32) -> Option<(u32, u32)> {
    let num_samples = i32::from(rd8(hdr, 3));
    if sample >= num_samples {
        return None;
    }
    let record = usize::from(rd8(hdr, 2) & 0x7F) + 2;
    let block = (u32::from(rd8(hdr, 9)) + 1) << 8;
    let at = 16 + record * sample as usize;
    let offset = ((u32::from(rd8(hdr, at)) << 8) + u32::from(rd8(hdr, at + 1))).wrapping_mul(block);
    let end = if sample + 1 < num_samples {
        let next = at + record;
        ((u32::from(rd8(hdr, next)) << 8) + u32::from(rd8(hdr, next + 1))).wrapping_mul(block)
    } else {
        u32::from(rd16(hdr, 10)).wrapping_mul(block)
    };
    Some((offset, end.wrapping_sub(offset)))
}

/// `iSPCHBank_AddToQueue` 0x829b6738.
fn add_to_queue(hdr: &mut [u8], sample: u8) {
    let repeat = rd8(hdr, 8);
    if repeat == 0 {
        return;
    }
    let base = queue_base(hdr);
    let index = rd8(hdr, base);
    if index < repeat {
        wr8(hdr, base + 1 + usize::from(index), sample);
    }
    let mut next = index.wrapping_add(1);
    if next >= repeat {
        next = 0;
    }
    wr8(hdr, base, next);
}

/// `iSPCHBank_GetSampleTimeInQueue` 0x829b67c8: how many plays ago the sample was queued (1 for
/// the last), 0 when it is not in the queue.
fn sample_time_in_queue(hdr: &[u8], sample: u8) -> i32 {
    let repeat = i32::from(rd8(hdr, 8));
    if repeat == 0 {
        return 0;
    }
    let base = queue_base(hdr);
    let mut index = i32::from(rd8(hdr, base));
    for age in 0..repeat {
        index -= 1;
        if index < 0 {
            index = repeat - 1;
        }
        if rd8(hdr, base + 1 + index as usize) == sample {
            return age + 1;
        }
    }
    0
}

/// `iSPCH_SetCycleBits` 0x829b6870: the samples of cycle `gameNum % count` (the first
/// `numSamples % count` cycles take one more).
fn set_cycle_bits(hdr: &mut [u8], game_num: u32) {
    let base = cycle_base(hdr);
    let count = u32::from(rd8(hdr, base));
    if count == 0 {
        return;
    }
    let num_samples = u32::from(rd8(hdr, 3));
    let cycle = game_num % count;
    let remainder = num_samples % count;
    let quotient = num_samples / count;
    let start = if cycle <= remainder { (quotient + 1) * cycle } else { quotient * cycle + remainder };
    let next = cycle + 1;
    let end = if next <= remainder { (quotient + 1) * next } else { quotient * next + remainder };
    for bit in start..end.max(start) {
        let at = base + 1 + (bit / 8) as usize;
        let byte = rd8(hdr, at) | (1 << (bit % 8));
        wr8(hdr, at, byte);
    }
}

/// `SPCH_GetSampleDataRate` 0x829b40c0 for uncompressed data.
#[must_use]
pub const fn sample_data_rate(sample_rate: i32, sample_bits: i32) -> i32 {
    sample_rate * sample_bits / 8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seed 0 gives `seedX`'s static words (0x82ff27b4).
    #[test]
    fn seed_zero_is_the_static_seed() {
        assert_eq!(SpchRandom::seeded(0), [0xF22D_0E56, 0x8831_26E9, 0xC624_DD2F, 0x0702_C49C, 0x9E35_3F7D, 0x6FDF_3B64]);
    }

    #[test]
    fn eac_random_carries() {
        let mut random = SpchRandom { seed: [0, 0, 0, 0, 0, 0xFFFF_FFFF], history: [(0xFFFF, 0xFFFF); 32], index: 0 };
        // s4 + s5 = 0xFFFFFFFF, no carry; s5 wraps, so s4 increments to 0 and the ripple reaches
        // s3 (0xFFFFFFFF + 1).
        let value = random.eac_random();
        assert_eq!(random.seed[5], 0);
        assert_eq!(random.seed[4], 0);
        assert_eq!(random.seed[3], 0);
        assert_eq!(value, random.seed[0]);
    }

    #[test]
    fn rand_stays_below_max() {
        let mut random = SpchRandom::new(1234);
        for max in [1, 2, 3, 7, 100, 1000] {
            for _ in 0..50 {
                assert!(random.rand(max, -1) < max as u32);
                assert!(random.rand(max, 5) < max as u32);
            }
        }
    }

    /// A bank of three samples (one parameter each, values 0/1/2) in blocks of 512 bytes over a
    /// 6-block bank, with a two-entry repeat queue.
    fn bank() -> Vec<u8> {
        let mut hdr = vec![0u8; 64];
        hdr[0..2].copy_from_slice(&7u16.to_be_bytes());
        hdr[2] = 1;
        hdr[3] = 3;
        hdr[4..8].copy_from_slice(&9u32.to_be_bytes());
        hdr[8] = 2;
        hdr[9] = 1;
        hdr[10..12].copy_from_slice(&6u16.to_be_bytes());
        for (i, (block, value)) in [(0u16, 0u8), (2, 1), (5, 2)].into_iter().enumerate() {
            hdr[16 + 3 * i..18 + 3 * i].copy_from_slice(&block.to_be_bytes());
            hdr[18 + 3 * i] = value;
        }
        hdr
    }

    #[test]
    fn bank_sample_sizes_and_queue() {
        let mut hdr = bank();
        assert_eq!(sample_size_data(&hdr, 0), Some((0, 1024)));
        assert_eq!(sample_size_data(&hdr, 1), Some((1024, 1536)));
        assert_eq!(sample_size_data(&hdr, 2), Some((2560, 512)));
        assert_eq!(sample_size_data(&hdr, 3), None);
        assert_eq!(sample_time_in_queue(&hdr, 1), 0);
        add_to_queue(&mut hdr, 1);
        add_to_queue(&mut hdr, 2);
        assert_eq!(sample_time_in_queue(&hdr, 2), 1);
        assert_eq!(sample_time_in_queue(&hdr, 1), 2);
        add_to_queue(&mut hdr, 0);
        assert_eq!(sample_time_in_queue(&hdr, 1), 0);
        assert_eq!(sample_time_in_queue(&hdr, 0), 1);
    }

    #[test]
    fn banks_sort_by_type_and_sub_id() {
        let mut spch = Spch::new(0, sample_data_rate(22050, 16));
        let make = |ty: u16, sub: u32| {
            let mut hdr = bank();
            hdr[0..2].copy_from_slice(&ty.to_be_bytes());
            hdr[4..8].copy_from_slice(&sub.to_be_bytes());
            hdr
        };
        assert_eq!(spch.add_bank(make(5, 1)), 0);
        assert_eq!(spch.add_bank(make(3, 0)), 1);
        assert_eq!(spch.add_bank(make(5, 0)), 2);
        assert_eq!(spch.add_bank(make(9, 4)), 3);
        assert_eq!(spch.add_bank(make(5, 1)), -1);
        let keys: Vec<_> = (0..4).map(|i| spch.bank_key(i).unwrap()).collect();
        assert_eq!(keys, vec![(3, 0), (5, 0), (5, 1), (9, 4)]);
        assert_eq!(spch.find_sub_bank(5, 1), (2, 0));
        assert_eq!(spch.find_bank(9).1, 3);
        assert_eq!(spch.find_bank(4), (-1, -1));
    }

    #[test]
    fn event_spec_word() {
        let spec = EventSpec::new(1, 0, 0x200A);
        assert_eq!(spec.word(), 0x200A_0001);
        assert_eq!(EventSpec::from_word(spec.word()), spec);
    }
}
