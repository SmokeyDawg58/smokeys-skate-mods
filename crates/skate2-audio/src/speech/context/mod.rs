//! The context systems (`Contexts`, the contextsystem 1.01.00 package at 0x82aaf270-0x82ab89a0,
//! and `Sk8::Audio::Sk8ContextSysMan`): the cameraman's, the announcer's and the main cast's
//! rules for when their speech projects speak.
//!
//! `AudioSystem::Initialize` 0x82509c18 creates the three systems (`CreateContextSystem`
//! 0x82ab0278, seeded with the time base) and posts `kPostLoad` to each. A system is its
//! contexts (`<group>xml.big`, one XML file each), the scripts their conditions run
//! (`script.css` in `<group>context.big`, see [`gishell`]) and the trigger functions it
//! subscribes to (`Trigger_Events.evt`). The game calls a trigger (`Csis::gT_*`, through
//! `PlayCameraManCsisEvent` and the like): every context of that trigger takes the call's
//! parameters and joins the triggered list for its expiry time (`OnEvent` 0x82ab3b80). Each
//! audio update after the speech system, `Sk8ContextSysMan::Update` runs the systems
//! (`ContextSystemImpl::Update` 0x82ab7dd8): the triggered and passive contexts (passive ones
//! have no trigger) are evaluated by priority, one of the best whose conditions hold is picked
//! at random and its output event called (`EventSender::Emit` 0x82ab3080), which is a speech
//! project's `O_*` function: `iSPCH_CsisCb` adds that `spch` event. While a channel speaks, its
//! system only takes interrupting contexts (`Sk8SpeechSystem::UpdateContextSystems`).
//!
//! No shipped file is a context group, and no context has a follow event (nothing sends
//! `kDoFollow`), so neither is kept.

mod builder;
mod gishell;

use std::collections::HashMap;

use skate2_formats::big::BigArchive;

use super::spch::{self, Spch};
use crate::random::RwRandom;
use crate::rwac::stream_pool::StreamFiles;
use gishell::GiShell;

/// The context systems `AudioSystem::Initialize` creates, in its `mContextSystems` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Group {
    CameraMan,
    Announcer,
    MainCast,
}

/// A `ContextSystemInfo`: the name, the XML and data files under `data\audio\context\`, and the
/// setup file's `Module` sizes (`maxNumContexts`, `maxNumGroups`, `maxNumValidContexts`,
/// `maxNumTriggeredContexts` and `numPassives` are all equal).
struct SystemInfo {
    context_file: &'static str,
    data_file: &'static str,
    size: usize,
}

const SYSTEMS: [SystemInfo; 3] = [
    SystemInfo { context_file: "cameramanxml.big", data_file: "cameramancontext.big", size: 225 },
    SystemInfo { context_file: "announcerxml.big", data_file: "announcercontext.big", size: 200 },
    SystemInfo { context_file: "maincastxml.big", data_file: "maincastcontext.big", size: 200 },
];

/// `ContextSystemManager::Update` 0x82aaf8b8 walks its `eastl::hash_map` of systems: buckets of
/// the FNV-1 name hash modulo 5 (two buckets after "Sk8CameraMan", five after "Sk8MainCast"),
/// each newest first, so "Sk8CameraMan" (bucket 1), then "Sk8MainCast" and "Sk8Announcer"
/// (bucket 4).
const UPDATE_ORDER: [Group; 3] = [Group::CameraMan, Group::MainCast, Group::Announcer];

/// The context id a script queries as "the context being evaluated".
const CURRENT_CONTEXT: i32 = 0xFEED_BABE_u32 as i32;
/// `gChosenList` capacity.
const MAX_CHOSEN: usize = 20;
/// `mPriorityDecayRate`'s default (no setup file overrides it): priority lost per second.
const PRIORITY_DECAY_RATE: i32 = 1000;
/// `eChallengeTypes_Unknown`.
const CHALLENGE_UNKNOWN: i32 = 52;

/// A `Contexts::EventSystem::IParamSource` as a context reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    /// A `ParamRef` to the trigger's parameter.
    Trigger(usize),
    /// `CreateLocalParam`'s constant.
    Local(i32),
    /// A `ParamGetter`, by its place among the context's getters (`mLocalParams`).
    Getter(usize),
}

/// `EventSystem::ParamGetter`: a script whose return value is the parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Getter {
    script_id: i32,
    sources: Vec<Source>,
}

/// `Csis::InterfaceId` of an event; the name is `None` without a `name` attribute.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Interface {
    name: Option<String>,
    system_crc: i16,
    interface_crc: i16,
}

/// `EventSystem::EventReceiver`: the trigger and the parameters its last call gave.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Trigger {
    interface: Interface,
    params: Vec<i32>,
}

/// `Contexts::Condition`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Condition {
    script_id: i32,
    sources: Vec<Source>,
}

/// `EventSystem::EventSender`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sender {
    interface: Interface,
    sources: Vec<Source>,
}

/// `Contexts::Context`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Context {
    name: String,
    priority: i32,
    expiry: i32,
    times_used: i32,
    last_time: u32,
    id: i32,
    interrupt: i32,
    trigger: Option<Trigger>,
    conditions: Vec<Condition>,
    output: Option<Sender>,
    getters: Vec<Getter>,
}

impl Context {
    /// A trigger with a Csis interface (`mTrigger && mTrigger->mCsis.interfaceCrc`); the other
    /// contexts are passive.
    fn triggered(&self) -> bool {
        self.trigger.as_ref().is_some_and(|trigger| trigger.interface.interface_crc != 0)
    }

    fn trigger_name(&self) -> Option<&str> {
        self.trigger.as_ref().filter(|trigger| trigger.interface.interface_crc != 0).and_then(|trigger| trigger.interface.name.as_deref())
    }

    /// `GetTimeSince` 0x82ab0fd8.
    fn time_since(&self, now: u32) -> u32 {
        if self.times_used <= 0 { 0x7FFF_FFFF } else { now.wrapping_sub(self.last_time) }
    }
}

/// `Contexts::ValidContext`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ValidContext {
    time_stamp: u32,
    context: usize,
    /// `flags[0]` (valid), `flags[1]` (`mInterrupt`'s low byte) and `flags[2]` (triggered).
    valid: bool,
    interrupt: bool,
    triggered: bool,
    priority: i32,
    expiry: i32,
}

/// `Contexts::ValidContextList`.
#[derive(Debug, Clone, Default)]
struct ValidContextList {
    entries: Vec<ValidContext>,
    max: usize,
}

impl ValidContextList {
    fn new(max: usize) -> Self {
        Self { entries: Vec::new(), max }
    }

    /// `AddContext` 0x82ab25e8 / `ValidContext::AddContext` 0x82ab24d0, while there is room.
    fn add(&mut self, contexts: &[Context], context: usize, now: u32) {
        if self.entries.len() >= self.max {
            return;
        }
        let c = &contexts[context];
        self.entries.push(ValidContext {
            time_stamp: now,
            context,
            valid: true,
            interrupt: c.interrupt as u8 != 0,
            triggered: c.triggered(),
            priority: c.priority,
            expiry: c.expiry,
        });
    }

    /// `Compact` 0x82ab26b8.
    fn compact(&mut self) {
        self.entries.retain(|entry| entry.valid);
    }

    /// `RemoveExpired` 0x82ab3650: triggered contexts older than their expiry.
    fn remove_expired(&mut self, contexts: &[Context], now: u32) {
        for entry in &mut self.entries {
            let context = &contexts[entry.context];
            if entry.valid && context.triggered() && now.wrapping_sub(entry.time_stamp) as i32 > context.expiry {
                entry.valid = false;
            }
        }
        self.compact();
    }

    /// `RemoveContext` 0x82ab36e8: after a triggered context spoke, every triggered context of
    /// the same trigger goes, and any other that expired.
    fn remove_context(&mut self, contexts: &[Context], context: usize, now: u32) {
        let Some(name) = contexts[context].trigger_name() else { return };
        for entry in &mut self.entries {
            let other = &contexts[entry.context];
            if !entry.valid || !other.triggered() {
                continue;
            }
            if other.trigger.as_ref().and_then(|trigger| trigger.interface.name.as_deref()) == Some(name) || now.wrapping_sub(entry.time_stamp) as i32 > other.expiry {
                entry.valid = false;
            }
        }
        self.compact();
    }

    /// `Merge` 0x82ab3820: the two lists by descending priority, `b`'s first on a tie (a
    /// negative priority ends its list); the last slot is never counted.
    fn merge(&mut self, a: &[ValidContext], b: &[ValidContext]) {
        self.entries.clear();
        let priority = |list: &[ValidContext], at: usize| list.get(at).map_or(-1, |entry| entry.priority);
        let (mut at_a, mut at_b) = (0, 0);
        while priority(a, at_a) >= 0 || priority(b, at_b) >= 0 {
            let entry = if priority(a, at_a) <= priority(b, at_b) {
                at_b += 1;
                b[at_b - 1]
            } else {
                at_a += 1;
                a[at_a - 1]
            };
            if self.entries.len() + 1 < self.max {
                self.entries.push(entry);
            }
        }
    }
}

/// `Contexts::ContextRand` (`contextSeedX`, one state the three systems share).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContextRand([u32; 6]);

impl ContextRand {
    /// `InitRandom` 0x82ab3ab8: the six words from the seed, then `32 - seed * 65537 % 32` draws.
    fn new(game_seed: u32) -> Self {
        let base = game_seed.wrapping_mul(65537);
        let s0 = base.wrapping_add(0xF22D_0000).wrapping_add(0x0E56);
        let s1 = s0.wrapping_add(0x9604_0000).wrapping_add(0x1893);
        let s2 = s1.wrapping_add(0x3DF4_0000).wrapping_sub(0x49BA);
        let s3 = s2.wrapping_add(0x40DE_0000).wrapping_sub(0x1893);
        let s4 = s3.wrapping_add(0x9732_0000).wrapping_add(0x7AE1);
        let s5 = s4.wrapping_add(0xD1AA_0000).wrapping_sub(0x0419);
        let mut rand = Self([s0, s1, s2, s3, s4, s5]);
        for _ in 0..32 - (base as i32) % 32 {
            rand.next();
        }
        rand
    }

    /// `EACrandom` 0x82ab15e8: the words chained with carries, the last counting up (its wrap
    /// carried down the others).
    fn next(&mut self) -> u32 {
        let [s0, s1, s2, s3, s4, s5] = self.0;
        let n4 = s4.wrapping_add(s5);
        let carry4 = u32::from(n4 < s5 || n4 < s4);
        let n3 = s3.wrapping_add(n4).wrapping_add(carry4);
        let n2 = s2.wrapping_add(n3).wrapping_add(u32::from(n3 < s3));
        let n1 = s1.wrapping_add(n2).wrapping_add(u32::from(n2 < s2));
        let n0 = s0.wrapping_add(n1).wrapping_add(u32::from(n1 < s1));
        let n5 = s5.wrapping_add(1);
        self.0 = [n0, n1, n2, n3, n4, n5];
        if n5 != 0 {
            return n0;
        }
        self.0[4] = n4.wrapping_add(1);
        if self.0[4] != 0 {
            return n0;
        }
        self.0[3] = n3.wrapping_add(1);
        if self.0[3] != 0 {
            return n0;
        }
        self.0[2] = n2.wrapping_add(1);
        if self.0[2] != 0 {
            return n0;
        }
        self.0[1] = n1.wrapping_add(1);
        if self.0[1] != 0 {
            return n0;
        }
        self.0[0] = n0.wrapping_add(1);
        self.0[0]
    }
}

/// What `AudioSystem` answers as `Contexts::GameInterface` (`GetValue` 0x824f12f0 over the
/// getters `RegisterGetValueFunctions` 0x824ece08 registers), from its fields as its last update
/// left them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameValues {
    pub challenge_type: i32,
    pub game_mode: i32,
    pub online_game: bool,
    /// `Presentation::IsInReplay`, false without a `Presentation`.
    pub in_replay: bool,
    /// `mbIsPlayerProvidingInput`, `mAwayTime` (ms) and `mIsMale`.
    pub player_providing_input: bool,
    pub away_time: i32,
    pub is_male: bool,
    /// `mFEMusicState`.
    pub fe_music_state: i32,
    /// `mCurrentSceneInfo.sceneType`.
    pub scene_type: i32,
    /// `mbIsPaused` and `mTimeSinceUnPaused` (ms).
    pub paused: bool,
    pub time_since_unpaused: i32,
    /// `mSk8HUDState`.
    pub hud_state: i32,
    /// `mCurrentChallenge`'s host (layout word 0); none without a current challenge.
    pub challenge_host: Option<i32>,
    /// `mbIsGVR`, `mbIsDannyWayJam` and `mbIsMegaChallenge`.
    pub gvr: bool,
    pub danny_way_jam: bool,
    pub mega_challenge: bool,
}

/// The game interface a script runs against: the values and `rw::math::Random`.
struct GameInterface<'a> {
    values: &'a GameValues,
    rng: &'a mut RwRandom,
}

impl gishell::Game for GameInterface<'_> {
    /// `AudioSystem::GetValue` 0x824f12f0: variable `n` is getter `n - 1` of `mFuncPointers`
    /// (`GetRandomBool`, `GetRandomValue`, `GetChallengeType`, `GetGameMode`, `GetIsInOnlineGame`,
    /// `GetIsInReplay`, `GetPlayerProvidingInput`, `GetIsInChallenge`, a getter folded to
    /// `return 1`, `GetPlayerAwayTime`, `GetPlayerGender`, `GetCurrentMenu`,
    /// `GetCurrentSceneType`, `GetIsPaused`, `GetTimeSinceUnPaused`, `GetCurrentFEHudScreen`,
    /// `GetIsGVR`, `GetChallengeHost`, `GetChallengeSecondsRemaining`,
    /// `GetChallengeMinutesRemaining`, `GetIsDannyWayJam`, `GetIsMegaJam`). Nothing writes the
    /// challenge's seconds and minutes remaining after the constructor's 0.
    fn value(&mut self, variable: i32, params: &[(i32, i32)]) -> i32 {
        let v = self.values;
        let param = |index: usize| params.get(index).map_or(0, |&(_, value)| value);
        match variable {
            1 => {
                let range = param(1) as u32;
                let chance = param(0);
                let chance = if chance <= range as i32 { chance.max(0) } else { range as i32 };
                let draw = self.rng.next_u32();
                draw.checked_rem(range).is_some_and(|draw| chance > draw as i32).into()
            }
            2 => {
                let draw = self.rng.next_u32();
                draw.checked_rem(param(0) as u32).map_or(0, |value| value as i32)
            }
            3 => v.challenge_type,
            4 => v.game_mode,
            5 => v.online_game.into(),
            6 => v.in_replay.into(),
            7 => v.player_providing_input.into(),
            8 => (v.challenge_type != CHALLENGE_UNKNOWN).into(),
            9 => 1,
            10 => v.away_time,
            11 => (!v.is_male).into(),
            12 => v.fe_music_state,
            13 => v.scene_type,
            14 => v.paused.into(),
            15 => v.time_since_unpaused,
            16 => v.hud_state,
            17 => v.gvr.into(),
            18 => v.challenge_host.unwrap_or(100),
            19 | 20 => 0,
            21 => v.danny_way_jam.into(),
            22 => v.mega_challenge.into(),
            _ => 0,
        }
    }
}

/// `ContextQuery` over a system's contexts while one is evaluated.
struct ContextQuery<'a> {
    contexts: &'a [Context],
    ids: &'a [i32],
    /// `mCurrentContextID`: the evaluated context's id, 0 between evaluations.
    current: i32,
    now: u32,
}

impl gishell::Query for ContextQuery<'_> {
    /// `QueryContext` 0x82ab17d8: the context's last use, times used or time since its last use
    /// (by its id, `0xFEEDBABE` for the current one); 0 for an unknown context.
    fn context(&self, func: i32, id: i32) -> i32 {
        let id = if id == CURRENT_CONTEXT { self.current } else { id };
        let Some(index) = self.ids.iter().position(|&candidate| candidate == id) else { return 0 };
        let context = &self.contexts[index];
        match func {
            502 => context.last_time as i32,
            503 => context.times_used,
            504 => context.time_since(self.now) as i32,
            _ => 0,
        }
    }
}

/// `Contexts::ContextSystemImpl`.
#[derive(Debug, Default)]
struct ContextSystem {
    /// `mContexts` by descending priority (`InsertContextInOrder`), `mMaxNumContexts`, and
    /// `mContextIds` (set at `kPostLoad`).
    contexts: Vec<Context>,
    max_contexts: usize,
    context_ids: Vec<i32>,
    /// `mTriggerHashTable`: each trigger's contexts in `mContexts` order.
    triggers: HashMap<String, Vec<usize>>,
    /// The trigger functions `AddCsisEvents` subscribed to (`Trigger_Events.evt`).
    subscribed: Vec<String>,
    shell: GiShell,
    /// `mTriggerList` (current and merge), `mAddList`, `mPassiveList`, `mEvaluationList`.
    trigger_lists: [ValidContextList; 2],
    current_trigger_list: usize,
    add_list: ValidContextList,
    passive_list: ValidContextList,
    evaluation_list: ValidContextList,
    /// `mSpeakerPriority`, `mInterruptsOnly`, `mInterruptPriority`, `mLastTick`, `mEventSuccess`
    /// and `mCurrentContextID`.
    speaker_priority: i32,
    interrupts_only: bool,
    interrupt_priority: i32,
    last_tick: u32,
    event_success: bool,
    current_context_id: i32,
    /// `gChosenList` for this update.
    chosen: Vec<usize>,
}

impl ContextSystem {
    /// The `ContextSystemImpl` constructor 0x82ab8418 for a setup file's sizes: the lists
    /// (`mEvaluationList` holds the passives and the valid contexts) and `mLastTick`.
    fn new(size: usize, now: u32) -> Self {
        Self {
            max_contexts: size,
            trigger_lists: [ValidContextList::new(size), ValidContextList::new(size)],
            add_list: ValidContextList::new(size),
            passive_list: ValidContextList::new(size),
            evaluation_list: ValidContextList::new(2 * size),
            last_tick: now,
            ..Self::default()
        }
    }

    /// `AddContext` 0x82ab6330: a context replaces those of its id (`RemoveContextById` 0x82ab54f8,
    /// which skips the one after each it removes) and joins by priority while there is room.
    fn add_context(&mut self, context: Context) {
        let mut index = 0;
        while index < self.contexts.len() {
            if self.contexts[index].id == context.id {
                self.contexts.remove(index);
            }
            index += 1;
        }
        if self.contexts.len() >= self.max_contexts {
            return;
        }
        let at = self.contexts.iter().position(|other| context.priority > other.priority).unwrap_or(self.contexts.len());
        self.contexts.insert(at, context);
    }

    /// `kPostLoad` (`HandleMessage` 0x82ab8880): `InitLoadedContexts` 0x82ab2fc8 (the ids and
    /// `CreateTriggerHashTable` 0x82ab2d28) and `PopulatePassiveList` 0x82ab2ca0.
    fn post_load(&mut self, now: u32) {
        self.context_ids = self.contexts.iter().map(|context| context.id).collect();
        self.triggers.clear();
        for (index, context) in self.contexts.iter().enumerate() {
            if let Some(name) = context.trigger_name() {
                self.triggers.entry(name.to_owned()).or_default().push(index);
            }
        }
        self.passive_list.entries.clear();
        for index in 0..self.contexts.len() {
            if !self.contexts[index].triggered() {
                self.passive_list.add(&self.contexts, index, now);
            }
        }
    }

    /// `OnEvent` 0x82ab3b80: the trigger's contexts take the call's parameters and join the
    /// current triggered list (after its expired ones go) by priority.
    fn on_event(&mut self, name: &str, params: &[i32], now: u32) {
        self.add_list.entries.clear();
        let Some(indices) = self.triggers.get(name) else { return };
        for &index in indices {
            if let Some(trigger) = self.contexts[index].trigger.as_mut() {
                for (at, param) in trigger.params.iter_mut().enumerate() {
                    *param = params.get(at).copied().unwrap_or(0);
                }
            }
            self.add_list.add(&self.contexts, index, now);
        }
        if indices.is_empty() {
            return;
        }
        let current = self.current_trigger_list;
        self.trigger_lists[current].remove_expired(&self.contexts, now);
        let merged = {
            let mut merged = std::mem::take(&mut self.trigger_lists[1 - current]);
            merged.merge(&self.trigger_lists[current].entries, &self.add_list.entries);
            merged
        };
        self.trigger_lists[1 - current] = merged;
        self.current_trigger_list = 1 - current;
    }

    /// A source's value (`IParamSource::GetParamValue`): a getter runs its script with its own
    /// sources (`ParamGetter::GetParamValue` 0x82ab5750).
    fn source_value(&mut self, context: usize, source: Source, now: u32, game: &mut GameInterface<'_>) -> i32 {
        match source {
            Source::Trigger(index) => self.contexts[context].trigger.as_ref().and_then(|trigger| trigger.params.get(index)).copied().unwrap_or(0),
            Source::Local(value) => value,
            Source::Getter(getter) => {
                let Getter { script_id, sources } = self.contexts[context].getters[getter].clone();
                let params: Vec<i32> = sources.into_iter().map(|source| self.source_value(context, source, now, game)).collect();
                self.run_script(script_id, &params, now, game)
            }
        }
    }

    fn run_script(&mut self, script_id: i32, params: &[i32], now: u32, game: &mut GameInterface<'_>) -> i32 {
        let query = ContextQuery { contexts: &self.contexts, ids: &self.context_ids, current: self.current_context_id, now };
        self.shell.execute(script_id, params, &query, game)
    }

    /// `Context::Evaluate` 0x82ab4fb8: the conditions in order (`Condition::ExecuteScript`
    /// 0x82ab4cb0) as the current context; the first that fails moves to the front.
    fn evaluate(&mut self, context: usize, now: u32, game: &mut GameInterface<'_>) -> bool {
        self.current_context_id = self.contexts[context].id;
        let mut result = true;
        for at in 0..self.contexts[context].conditions.len() {
            let Condition { script_id, sources } = self.contexts[context].conditions[at].clone();
            let params: Vec<i32> = sources.into_iter().map(|source| self.source_value(context, source, now, game)).collect();
            result = self.run_script(script_id, &params, now, game) != 0;
            if !result {
                self.contexts[context].conditions.swap(0, at);
                break;
            }
        }
        self.current_context_id = 0;
        result
    }

    /// `Choose` 0x82ab55b8: from `start`, the evaluation list down to the best priority found
    /// (at least `priority`), skipping non-interrupting contexts while only interrupts are taken
    /// and expired triggered ones; every context of the best priority whose conditions hold is
    /// chosen (at most 20).
    fn choose(&mut self, start: &mut usize, priority: i32, now: u32, game: &mut GameInterface<'_>) {
        let mut priority = priority;
        if priority < 0 {
            return;
        }
        while *start < self.evaluation_list.entries.len() {
            let entry = self.evaluation_list.entries[*start];
            if entry.priority < priority {
                return;
            }
            let in_time = !entry.triggered || now.wrapping_sub(entry.time_stamp) as i32 <= entry.expiry;
            if (!self.interrupts_only || entry.interrupt) && in_time {
                if self.evaluate(entry.context, now, game) {
                    let context_priority = self.contexts[entry.context].priority;
                    if context_priority == priority {
                        if self.chosen.len() < MAX_CHOSEN {
                            self.chosen.push(entry.context);
                        }
                    } else {
                        self.chosen.clear();
                        self.chosen.push(entry.context);
                        priority = context_priority;
                    }
                } else if entry.triggered {
                    self.evaluation_list.entries[*start].valid = false;
                }
            }
            *start += 1;
        }
    }

    /// `EventSender::Emit` 0x82ab3080: the output's parameters, then its `O_*` function
    /// (`Csis::FunctionHandle::Set`, `Csis::Function::Call`); a function no speech project binds
    /// fails the emit.
    fn emit(&mut self, context: usize, now: u32, game: &mut GameInterface<'_>, spch: &mut Spch) {
        let Some(output) = self.contexts[context].output.clone() else { return };
        let params: Vec<i32> = output.sources.iter().map(|&source| self.source_value(context, source, now, game)).collect();
        let Some(name) = output.interface.name.as_deref().filter(|_| output.interface.interface_crc != 0) else { return };
        match spch.csis_event(name, output.interface.system_crc, output.interface.interface_crc) {
            Some(spec) => spch.csis_callback(spec, &params),
            None => self.event_success = false,
        }
    }

    /// `ContextSystemImpl::Update` 0x82ab7dd8: the speaker's priority decays by
    /// `mPriorityDecayRate` per second since `mLastTick`; the evaluation list merges the triggered
    /// and passive lists; of the contexts chosen at the best priority no lower than the speaker's
    /// (while only interrupts are taken, no lower than the last speaker's), one at a time is
    /// picked at random (`EACrandom`) and emitted until an emit succeeds. The one that spoke sets
    /// both priorities and is tracked; its trigger's contexts and the expired ones leave the
    /// triggered list.
    fn update(&mut self, now: u32, rand: &mut ContextRand, game: &mut GameInterface<'_>, spch: &mut Spch) {
        self.chosen.clear();
        if self.speaker_priority > 0 {
            let elapsed = now.wrapping_sub(self.last_tick);
            self.last_tick = now;
            let decay = (elapsed as f32 * 0.001 * PRIORITY_DECAY_RATE as f32) as i32;
            self.speaker_priority = self.speaker_priority.wrapping_sub(decay);
        }
        if self.speaker_priority < 0 {
            self.speaker_priority = 0;
        }
        if !self.interrupts_only {
            self.interrupt_priority = 0;
        }
        let priority = self.speaker_priority.max(self.interrupt_priority);
        let current = self.current_trigger_list;
        self.evaluation_list.merge(&self.trigger_lists[current].entries, &self.passive_list.entries);
        let mut start = 0;
        let mut spoke;
        loop {
            spoke = None;
            let again = if self.chosen.is_empty() {
                self.choose(&mut start, priority, now, game);
                !self.chosen.is_empty()
            } else {
                let count = self.chosen.len() as u32;
                let index = ((rand.next() >> 16).wrapping_mul(count) >> 16) as usize;
                let context = self.chosen.remove(index);
                let context_priority = self.contexts[context].priority;
                self.interrupt_priority = context_priority;
                self.speaker_priority = context_priority;
                self.event_success = true;
                self.emit(context, now, game, spch);
                spoke = Some(context);
                !self.event_success
            };
            if !again {
                break;
            }
        }
        if let Some(context) = spoke {
            self.trigger_lists[current].remove_context(&self.contexts, context, now);
            let context = &mut self.contexts[context];
            context.times_used += 1;
            context.last_time = now;
        }
        self.trigger_lists[current].remove_expired(&self.contexts, now);
    }
}

/// `Sk8ContextSysMan`: the three context systems and the random state they share.
#[derive(Debug)]
pub(super) struct ContextSystems {
    systems: [ContextSystem; 3],
    rand: ContextRand,
}

/// `AudioSystem::GetTime` 0x82509ba8: the audio core's system time in milliseconds (`fctidz`,
/// the low word).
#[must_use]
pub fn game_time(system_time: f64) -> u32 {
    (system_time * 1000.0) as i64 as u32
}

/// A whole file through the host's files.
fn read_file(files: &dyn StreamFiles, path: &str) -> Option<Vec<u8>> {
    let mut bytes = vec![0u8; usize::try_from(files.size(path)?).ok()?];
    files.read_at(path, 0, &mut bytes).then_some(bytes)
}

/// A BIG archive's entries in table order.
fn big_entries(bytes: &[u8]) -> Vec<&[u8]> {
    let Ok(table) = BigArchive::parse(bytes) else { return Vec::new() };
    table
        .entries
        .iter()
        .map(|entry| {
            let start = entry.offset as usize;
            bytes.get(start..start + entry.stored_size as usize).unwrap_or(&[])
        })
        .collect()
}

impl ContextSystems {
    /// `AudioSystem::Initialize`'s `CreateContextSystem` and `kPostLoad` for each system, all
    /// seeded with the same time base word, at audio time `now` (ms). `Sk8ContextSysMan::LoadFiles`
    /// 0x824f9db0: `<group>context.big` holds the scripts (entry 0), the trigger functions'
    /// `.csi` (1) and their `.evt` (2, `AddCsisEvents` 0x82ab3dd0); every entry of `<group>xml.big`
    /// is a context file (`AddData` 0x82ab6808, `AddContextXml` 0x82ab63c0).
    pub(super) fn load(files: &dyn StreamFiles, seed: u32, now: u32) -> Self {
        let systems = SYSTEMS.map(|info| {
            let mut system = ContextSystem::new(info.size, now);
            if let Some(data) = read_file(files, &format!("data\\audio\\context\\{}", info.data_file)) {
                let entries = big_entries(&data);
                if let Some(script) = entries.first() {
                    system.shell = GiShell::new(script.to_vec());
                }
                if let Some(evt) = entries.get(2) {
                    system.subscribed = spch::csis_table(evt).into_iter().map(|row| row.name).collect();
                }
            }
            if let Some(xml) = read_file(files, &format!("data\\audio\\context\\{}", info.context_file)) {
                for file in big_entries(&xml) {
                    if builder::file_type(file) == builder::FileType::Context {
                        system.add_context(builder::build_context(file));
                    }
                }
            }
            system.post_load(now);
            system
        });
        Self { systems, rand: ContextRand::new(seed) }
    }

    fn system(&mut self, group: Group) -> &mut ContextSystem {
        &mut self.systems[group as usize]
    }

    /// A call of the group's trigger function (`Csis::Function::Call` on its `gT_*` handle, whose
    /// client is `CsisCallback` 0x82ab3cf8 when the system subscribed to it).
    pub(super) fn trigger(&mut self, group: Group, function: &str, params: &[i32], now: u32) {
        let system = self.system(group);
        if system.subscribed.iter().any(|name| name == function) {
            system.on_event(function, params, now);
        }
    }

    /// `kInterruptsOn` / `kInterruptsOff` (`HandleMessage` 0x82ab8880): while on, only
    /// interrupting contexts are chosen.
    pub(super) fn set_interrupts_only(&mut self, group: Group, interrupts_only: bool) {
        self.system(group).interrupts_only = interrupts_only;
    }

    /// `Sk8ContextSysMan::Update` (`ContextSystemManager::Update` 0x82aaf8b8): each system in the
    /// manager's order.
    pub(super) fn update(&mut self, now: u32, values: &GameValues, rng: &mut RwRandom, spch: &mut Spch) {
        let mut game = GameInterface { values, rng };
        for group in UPDATE_ORDER {
            self.systems[group as usize].update(now, &mut self.rand, &mut game, spch);
        }
    }
}

#[cfg(test)]
mod tests;
