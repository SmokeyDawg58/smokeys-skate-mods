//! `FrontEndState_NIS` (FE state 2)'s overlay screens: the list a NIS
//! preloads (`PreloadNISStates` `0x8268a948`), the screen states its overlay
//! channel sets (`SetState` `0x8266fe00`) and the front end screen manager
//! (`helper/screenmanager.apt` with `ScreenManagerBase`) the screens open and
//! close through.
//!
//! One Apt frame runs per front end update: `FrontEndManager::Update`
//! (`0x82640130`) hands `AptIntegration::Update` the update's 16 ms
//! (`dT / 100`), the overlay movies' frame time, before it dispatches its
//! events.

use std::collections::{BTreeMap, VecDeque};

/// `eSk8HUDNISState_Max`.
pub const STATE_MAX: i32 = 87;
/// `eSk8HUDNISState_Unknown`.
pub const STATE_UNKNOWN: i32 = 88;
const PRO_BIO_INFO_LEFT: i32 = 10;
const PRO_BIO_INFO_RIGHT: i32 = 11;
const KILLED_IT: [i32; 4] = [70, 72, 73, 74];
/// `eSk8HUDNISState_OpenResultsScreen`.
pub const OPEN_RESULTS_SCREEN: i32 = 79;
const GESTURE_OLLIE_PT1: i32 = 80;
const GESTURE_OLLIE_PT2: i32 = 81;
/// `eSk8FEScreenState_SpotFound`.
pub const SPOT_FOUND: i32 = 48;
/// `eSk8FEScreenState_SpotOwned`.
pub const SPOT_OWNED: i32 = 49;
/// `eSk8FEScreenState_SpotKilled`.
pub const SPOT_KILLED: i32 = 0x32;
/// `eSk8FEScreenState_Pro_Bio_Info` (template entry 0x82fdb5b8).
pub const PRO_BIO_INFO: i32 = 38;
/// `eSk8FEScreenState_PhotoTreatment` (template entry 0x82fdb728).
pub const PHOTO_TREATMENT: i32 = 61;
/// `eSk8FEScreenState_FixedPhotoTreatment` (template entry 0x82fdb738).
pub const FIXED_PHOTO_TREATMENT: i32 = 62;
/// `eSk8FEScreenState_NonFixedPhotoTreatment` (template entry 0x82fdb748).
pub const NON_FIXED_PHOTO_TREATMENT: i32 = 63;
/// `eSk8FEScreenState_SponsorLogo_Setup`.
const SPONSOR_LOGO_SETUP: i32 = 60;
/// `eSk8FEScreenState_TutorialGesture_Ollie`.
pub const TUTORIAL_GESTURE_OLLIE: i32 = 0x83;
/// `eSk8FEScreenState_GVR_Regular` (template entry 0x82fdbb98).
pub const GVR_REGULAR: i32 = 132;
/// `eSk8FEScreenState_GVR_Goofy` (template entry 0x82fdbba8).
pub const GVR_GOOFY: i32 = 133;
/// `eSk8FEScreenState_NISSanVanTV` (template entry 0x82fdbbd8).
pub const SAN_VAN_TV: i32 = 136;
/// `ProcessOpenScreens`' wait after it creates a screen (AS `0x001084`).
const OPEN_BLOCK: u32 = 8;

/// `mNISStateTable` (constructor `0x82673d30`, its stores emulated): the
/// sponsor id of a `mIsSponsor` row and `m360Screen` (`mPS3Screen` is the
/// same in every row).
const TABLE: [(Option<i32>, i32); 87] = [
    (None, -1), (None, -1), (None, 48), (None, 48), (None, 48), (None, 48), (None, 49), (None, 49),
    (None, 49), (None, 49), (None, 38), (None, 38), (None, 39), (None, 39), (None, -1), (None, 54),
    (None, 55), (None, 56), (None, 57), (None, 58), (None, 59), (Some(1), 60), (Some(2), 60), (Some(3), 60),
    (Some(4), 60), (Some(5), 60), (Some(6), 60), (Some(7), 60), (Some(8), 60), (Some(9), 60), (Some(10), 60),
    (Some(11), 60), (Some(12), 60), (Some(13), 60), (Some(14), 60), (Some(15), 60), (Some(16), 60),
    (Some(17), 60), (Some(18), 60), (Some(19), 60), (Some(20), 60), (Some(21), 60), (Some(22), 60),
    (Some(23), 60), (Some(24), 60), (Some(25), 60), (Some(26), 60), (Some(27), 60), (Some(28), 60),
    (Some(29), 60), (Some(30), 60), (Some(31), 60), (None, 60), (None, 60), (None, 61), (Some(32), 60),
    (Some(33), 60), (Some(34), 60), (Some(35), 60), (Some(36), 60), (Some(37), 60), (Some(38), 60),
    (Some(39), 60), (Some(40), 60), (Some(41), 60), (Some(42), 60), (Some(43), 60), (Some(44), 60),
    (None, 67), (None, 67), (None, 50), (None, 80), (None, 50), (None, 50), (None, 50), (None, 130),
    (None, 130), (None, 130), (None, 130), (None, -1), (None, 131), (None, -1), (None, 132), (None, 133),
    (None, 136), (None, 62), (None, 63),
];

fn row(state: i32) -> Option<(Option<i32>, i32)> {
    usize::try_from(state).ok().and_then(|i| TABLE.get(i)).copied()
}

/// `OnEnterState`'s jump table (`0x826676f8`) for a state shown through
/// `mScreenMap`: the `SetScreenState` value (a sponsor logo's is its id).
fn enter_value(state: i32) -> Option<i32> {
    Some(match state {
        2 | 6 => 4,
        3 | 7 | 12 | 14 | 68 | 71 | 75 => 0,
        4 | 8 => 6,
        5 | 9 | 13 | 77 => 2,
        PRO_BIO_INFO_LEFT => 0,
        PRO_BIO_INFO_RIGHT => 2,
        15..=20 | 52..=54 | 69 | 76 | 82..=86 => 1,
        78 => 3,
        21..=51 | 55..=67 => row(state)?.0?,
        _ => return None,
    })
}

/// `OnLeaveState`'s jump table (`0x82667db4`).
fn leave_value(state: i32) -> Option<i32> {
    Some(match state {
        2 | 6 => 5,
        3 | 7 | 10 | 12 => 1,
        4 | 8 => 7,
        5 | 9 | 11 | 13 | 69 => 3,
        21..=51 | 55..=67 => 50,
        68 => 2,
        75..=78 | 82..=84 => -1,
        _ => return None,
    })
}

/// What the front end's screen manager does with a NIS screen's movie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenCommand {
    /// The movie `CreateScreen` loaded runs its first frame and its class
    /// registers (`ScreenManager.OnLoaded`: `Initialise`, the
    /// `InitialStandardScreenReady` callback's `PlayClip(id, 'intro')`, then
    /// the class's `OnLoaded`).
    Create { id: i32, template: i32 },
    /// `ScreenManager_SetScreenState` on a loaded screen: its class's
    /// `SetState`.
    SetState { id: i32, state: i32 },
    /// `RemoveScreen`: the class's `Destroy`, then `removeMovieClip`.
    Destroy { id: i32 },
}

/// `sScreenCallBack::mLoadedCallBack` of a NIS screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Loaded {
    /// `FrontEndState_NIS::OnLoaded` (`0x8267c740`) with its screen wrapper.
    Wrapper(i32),
    /// `KilledItReady` (`0x82658708`).
    KilledIt,
    /// `TutorialOllieReady` (`0x826587e8`).
    GestureOllie,
}

/// The AS `ScreenManager` with `ScreenManagerBase`'s pending loaded
/// callbacks. Ids count from the constructor's `mScreenId = 0`; the game's
/// counter also numbers the menus and HUD screens it opened before.
#[derive(Debug, Default)]
struct ScreenManager {
    /// `mScreenId`, the id `ReadyScreen` hands out next.
    next_id: i32,
    /// `mScreenArray`: each readied screen's template and whether its class
    /// has registered.
    screens: BTreeMap<i32, (i32, bool)>,
    /// `mScreenOpenArray`.
    open: VecDeque<i32>,
    /// `mScreenDestroyArray`; slots from `mPendingScreensToClear` on keep the
    /// ids the shifts left there.
    destroy: Vec<Option<i32>>,
    /// `mPendingScreensToClear`.
    clear: usize,
    /// `mBlockScreenOpen`.
    block: u32,
    /// `mScreen.onEnterFrame` is `Update`.
    updating: bool,
    /// Movies `CreateScreen` started; `loadMovie` completes them at the
    /// next frame (the game's `AptLoadingIntegration` loads them
    /// asynchronously from the disc).
    loading: Vec<i32>,
    /// `mPendingLoadedCallback`.
    callbacks: BTreeMap<i32, Loaded>,
    commands: Vec<ScreenCommand>,
}

impl ScreenManager {
    /// `FrontEndManager::OpenScreen` -> `ScreenManagerBase::OpenScreen`
    /// (`0x82d6a840`): AS `ReadyScreen` queues the screen under `mScreenId++`
    /// and starts `Update`; the callback waits for the id.
    fn open(&mut self, template: i32, loaded: Loaded) -> i32 {
        let id = self.next_id;
        self.next_id += 1;
        self.screens.insert(id, (template, false));
        self.open.push_back(id);
        self.updating = true;
        self.callbacks.insert(id, loaded);
        id
    }

    /// `FrontEndManager::SetScreenState` -> AS `SetScreenState`: only a
    /// registered class has `SetState`.
    fn set_state(&mut self, id: i32, state: i32) {
        if self.screens.get(&id).is_some_and(|&(_, registered)| registered) {
            self.commands.push(ScreenCommand::SetState { id, state });
        }
    }

    /// `ScreenManagerBase::DestroyScreen` (`0x82d68138`): AS `DestroyScreen`
    /// queues the id unless the slot past the queue holds it, starts
    /// `Update`; the pending loaded callback is dropped.
    fn destroy(&mut self, id: i32) {
        if self.clear > 0 && self.destroy.get(self.clear).copied().flatten() == Some(id) {
            return;
        }
        if self.destroy.len() <= self.clear {
            self.destroy.resize(self.clear + 1, None);
        }
        self.destroy[self.clear] = Some(id);
        self.clear += 1;
        self.updating = true;
        self.callbacks.remove(&id);
    }

    /// One Apt frame: the movies that finished loading register (AS
    /// `OnLoaded`, then `Game.OnScreenLoaded` -> `ScreenManagerBase::
    /// OnScreenLoaded` `0x82d67f10` runs and drops the loaded callback), then
    /// `onEnterFrame`'s `Update`: `ProcessOpenScreens` only when
    /// `ProcessDestroyedScreens` removed nothing.
    fn frame(&mut self) -> Vec<(i32, Loaded)> {
        let mut ready = Vec::new();
        for id in std::mem::take(&mut self.loading) {
            if let Some((template, registered)) = self.screens.get_mut(&id) {
                *registered = true;
                self.commands.push(ScreenCommand::Create { id, template: *template });
                if let Some(loaded) = self.callbacks.remove(&id) {
                    ready.push((id, loaded));
                }
            }
        }
        if self.updating && !self.process_destroyed() {
            self.process_open();
        }
        ready
    }

    /// AS `ProcessDestroyedScreens`: the first queued id's `RemoveScreen`.
    fn process_destroyed(&mut self) -> bool {
        if self.clear == 0 {
            return false;
        }
        self.clear -= 1;
        if let Some(id) = self.destroy[0] {
            self.remove(id);
        }
        self.destroy[0] = None;
        for i in 0..self.clear {
            self.destroy[i] = self.destroy[i + 1];
        }
        true
    }

    /// AS `RemoveScreen`: the class's `Destroy` and the clip's
    /// `removeMovieClip` for a created movie; the entry is deleted.
    fn remove(&mut self, id: i32) {
        if let Some((_, registered)) = self.screens.remove(&id) {
            self.loading.retain(|&l| l != id);
            if registered {
                self.commands.push(ScreenCommand::Destroy { id });
            }
        }
    }

    /// AS `ProcessOpenScreens`: a blocked frame counts down; else one queued
    /// screen is created (`CreateScreen`: `createEmptyMovieClip`, then
    /// `loadMovie`) and the next eight frames are blocked. With both queues
    /// empty `onEnterFrame` is removed, which holds the block count.
    fn process_open(&mut self) {
        if self.block > 0 {
            self.block -= 1;
            return;
        }
        if self.clear == 0
            && let Some(id) = self.open.pop_front()
        {
            self.block = OPEN_BLOCK;
            if self.screens.contains_key(&id) {
                self.loading.push(id);
            }
        }
        if self.open.is_empty() && self.clear == 0 {
            self.updating = false;
        }
    }
}

/// `FrontEndState_NIS`'s overlay fields and statics.
#[derive(Debug)]
pub struct FeNisOverlays {
    /// `mNISState`.
    state: i32,
    /// `mScreenMap`: template to screen id, -1 while the screen loads.
    screen_map: BTreeMap<i32, i32>,
    /// `mLoadedScreenIds`.
    loaded_ids: Vec<i32>,
    /// `mSponsorsToLoad`.
    sponsors: Vec<i32>,
    /// `mbSponsorReady`.
    sponsor_ready: bool,
    /// `mNumBiosShown`.
    bios_shown: u32,
    /// `mICEOpened`: only ever stored 0 (the constructor at `0x8267470c` and
    /// `OnLeave`).
    ice_opened: bool,
    /// `mKilledItScreenId` (static, -1 when closed).
    killed_it_id: i32,
    /// `mGestureOllieScreenId` (static, -1 when closed).
    gesture_ollie_id: i32,
    /// `mNumLoaded` (static).
    num_loaded: usize,
    /// `mNumScreensToLoad` (static).
    num_to_load: usize,
    screens: ScreenManager,
    /// `CheckLoadComplete`'s `NISManager::FEPreloaded` calls not yet made.
    preloaded: u32,
    /// `OnEnterState(79)`'s `PushState(8)` calls not yet made.
    results_pushes: u32,
    /// The sponsor logo screen's `LoadImages` started loading logos; each
    /// logo's `OnComponentLoaded` counts it and the last calls
    /// `NIS.LogosReady()` (one frame later, as the screens load).
    logos_loading: bool,
}

impl Default for FeNisOverlays {
    fn default() -> Self {
        Self {
            state: STATE_UNKNOWN,
            screen_map: BTreeMap::new(),
            loaded_ids: Vec::new(),
            sponsors: Vec::new(),
            sponsor_ready: false,
            bios_shown: 0,
            ice_opened: false,
            killed_it_id: -1,
            gesture_ollie_id: -1,
            num_loaded: 0,
            num_to_load: 0,
            screens: ScreenManager::default(),
            preloaded: 0,
            results_pushes: 0,
            logos_loading: false,
        }
    }
}

impl FeNisOverlays {
    /// The front end update's Apt frame, with the loaded callbacks it runs.
    pub fn apt_frame(&mut self) {
        if std::mem::take(&mut self.logos_loading) {
            self.logos_ready();
        }
        for (id, loaded) in self.screens.frame() {
            match loaded {
                // OnLoaded -> MarkAsLoaded (0x82676d00). The sponsor logo
                // screen's constructor registers, then LoadImages loads
                // NIS.GetLogosToLoad() (mSponsorsToLoad).
                Loaded::Wrapper(screen) => {
                    self.screen_map.insert(screen, id);
                    self.loaded_ids.push(id);
                    self.num_loaded += 1;
                    if screen == SPONSOR_LOGO_SETUP && !self.sponsors.is_empty() {
                        self.logos_loading = true;
                    }
                }
                Loaded::KilledIt => {
                    self.killed_it_id = id;
                    self.num_loaded += 1;
                }
                Loaded::GestureOllie => {
                    self.gesture_ollie_id = id;
                    self.num_loaded += 1;
                }
            }
            self.check_load_complete();
        }
    }

    /// `CheckLoadComplete` (`0x82648538`).
    fn check_load_complete(&mut self) {
        if self.num_loaded == self.num_to_load && self.sponsor_ready {
            self.preloaded += 1;
            self.num_to_load = 0;
        }
    }

    /// `NISManager::FEPreloaded` calls `CheckLoadComplete` made since the
    /// last call.
    pub fn take_preloaded(&mut self) -> u32 {
        std::mem::take(&mut self.preloaded)
    }

    /// `PushState(8)` calls `OnEnterState(79)` made since the last call.
    pub fn take_results_pushes(&mut self) -> u32 {
        std::mem::take(&mut self.results_pushes)
    }

    /// The screen manager's work since the last call.
    pub fn take_commands(&mut self) -> Vec<ScreenCommand> {
        std::mem::take(&mut self.screens.commands)
    }

    /// `ReceiveNISFERequestPreloadStateMsg` (`0x82634dc8`) ->
    /// `PreloadNISStates` (`0x8268a948`), on FE state 2 whether or not it is
    /// pushed.
    pub fn preload_nis_states(&mut self, states: &[u8]) {
        self.num_loaded = 0;
        self.num_to_load = states.len();
        let mut unused: Vec<i32> = self.screen_map.keys().copied().collect();
        self.sponsors.clear();
        self.sponsor_ready = true;
        for state in states.iter().map(|&s| i32::from(s)) {
            if state < STATE_MAX
                && state != OPEN_RESULTS_SCREEN
                && let Some((Some(sponsor), _)) = row(state)
            {
                self.sponsors.push(sponsor);
            }
        }
        if !self.sponsors.is_empty() {
            self.sponsor_ready = false;
            if let Some(&id) = self.screen_map.get(&SPONSOR_LOGO_SETUP) {
                // kReloadImages.
                self.screens.set_state(id, 100);
            }
        }
        self.bios_shown = 0;
        for state in states.iter().map(|&s| i32::from(s)) {
            self.preload_nis_state(state);
            if let Some((_, screen)) = row(state) {
                unused.retain(|&s| s != screen);
            }
        }
        for screen in unused {
            self.unload_screen(screen);
        }
    }

    /// `NIS.LogosReady()` from the sponsor logo movie (`sMethod_LogosReady`
    /// `0x8260db40`).
    fn logos_ready(&mut self) {
        self.sponsor_ready = true;
        self.check_load_complete();
    }

    /// `PreloadNISState` (`0x82689520`).
    fn preload_nis_state(&mut self, state: i32) {
        if let Some((_, screen)) = row(state)
            && screen != -1
            && state != GESTURE_OLLIE_PT2
        {
            if KILLED_IT.contains(&state) {
                if self.killed_it_id == -1 {
                    self.screens.open(SPOT_KILLED, Loaded::KilledIt);
                    return;
                }
            } else if state == GESTURE_OLLIE_PT1 {
                if self.gesture_ollie_id == -1 {
                    self.screens.open(TUTORIAL_GESTURE_OLLIE, Loaded::GestureOllie);
                    return;
                }
            } else {
                self.open_screen(screen);
                return;
            }
        }
        self.num_loaded += 1;
        self.check_load_complete();
    }

    /// `FrontEndState_NIS::OpenScreen` (`0x82684a40`): a screen already in
    /// `mScreenMap` counts as loaded; else it goes in as -1 and opens with
    /// the `OnLoaded` wrapper.
    fn open_screen(&mut self, screen: i32) {
        if self.screen_map.contains_key(&screen) {
            self.num_loaded += 1;
            self.check_load_complete();
            return;
        }
        self.screen_map.insert(screen, -1);
        self.screens.open(screen, Loaded::Wrapper(screen));
    }

    /// `UnloadScreen` (`0x8266fe50`).
    fn unload_screen(&mut self, screen: i32) {
        if let Some(id) = self.screen_map.remove(&screen) {
            self.screens.destroy(id);
            if let Some(i) = self.loaded_ids.iter().position(|&l| l == id) {
                self.loaded_ids.remove(i);
            }
        }
    }

    /// `StateManagerFrontEndBase::SetState(2, id)` (`0x8266fe00`):
    /// `OnLeaveState(mNISState)`, the new state, `OnEnterState`.
    pub fn set_state(&mut self, state: i32) {
        self.on_leave_state(self.state);
        self.state = state;
        self.on_enter_state(state);
    }

    /// The screen `state` shows through `mScreenMap`, when it has loaded.
    fn mapped_id(&self, state: i32) -> Option<i32> {
        let (_, screen) = row(state)?;
        self.screen_map.get(&screen).copied().filter(|&id| id != -1)
    }

    /// `OnEnterState` (`0x826675a0`).
    fn on_enter_state(&mut self, state: i32) {
        if state == STATE_UNKNOWN || state == -1 {
            return;
        }
        if state == OPEN_RESULTS_SCREEN {
            self.results_pushes += 1;
            return;
        }
        let mapped = self.mapped_id(state);
        let gesture = matches!(self.state, GESTURE_OLLIE_PT1 | GESTURE_OLLIE_PT2) && self.gesture_ollie_id != -1;
        if mapped.is_none() && !KILLED_IT.contains(&self.state) && !gesture {
            return;
        }
        let id = mapped.unwrap_or(-1);
        match self.state {
            PRO_BIO_INFO_LEFT | PRO_BIO_INFO_RIGHT => {
                self.bios_shown += 1;
                if let Some(value) = enter_value(self.state) {
                    self.screens.set_state(id, value);
                }
            }
            70 | 72 | 73 | 74 => {
                if self.killed_it_id != -1 {
                    let value = KILLED_IT.iter().position(|&k| k == self.state).map_or(1, |i| i as i32 + 1);
                    self.screens.set_state(self.killed_it_id, value);
                }
            }
            GESTURE_OLLIE_PT1 | GESTURE_OLLIE_PT2 => {
                if self.gesture_ollie_id != -1 {
                    self.screens.set_state(self.gesture_ollie_id, self.state - GESTURE_OLLIE_PT1 + 1);
                }
            }
            other => {
                if let Some(value) = enter_value(other) {
                    self.screens.set_state(id, value);
                }
            }
        }
    }

    /// `OnLeaveState` (`0x82667cc8`): always ends at `eSk8HUDNISState_Unknown`.
    fn on_leave_state(&mut self, state: i32) {
        if state == STATE_UNKNOWN || state == -1 {
            return;
        }
        let mapped = self.mapped_id(state);
        let gesture = self.state == GESTURE_OLLIE_PT2 && self.gesture_ollie_id != -1;
        if mapped.is_some() || gesture {
            match self.state {
                0 => self.force_gesture_ollie_unload(),
                other => {
                    if let (Some(id), Some(value)) = (mapped, leave_value(other)) {
                        self.screens.set_state(id, value);
                    }
                }
            }
        }
        self.state = STATE_UNKNOWN;
    }

    /// `ForceGestureOllieUnload` (`0x82658830`).
    fn force_gesture_ollie_unload(&mut self) {
        if self.gesture_ollie_id != -1 {
            self.screens.destroy(self.gesture_ollie_id);
            self.gesture_ollie_id = -1;
        }
    }

    /// `FrontEndManager::PopNIS` (`0x82640cb8`) with FE state 2 on the stack:
    /// `DeletePreloadedScreens` (`0x82658678`), then `PopState`'s `OnLeave`
    /// (`0x8266fd70`): `OnLeaveState(mNISState)` (the map is empty by then),
    /// the KilledIt screen only with `mICEOpened`, the gesture screen always.
    pub fn pop_nis(&mut self) {
        for id in std::mem::take(&mut self.loaded_ids) {
            self.screens.destroy(id);
        }
        self.num_loaded = 0;
        self.num_to_load = 0;
        self.bios_shown = 0;
        self.screen_map.clear();
        self.sponsors.clear();
        self.on_leave_state(self.state);
        if self.ice_opened && self.killed_it_id != -1 {
            self.screens.destroy(self.killed_it_id);
            self.killed_it_id = -1;
        }
        if self.gesture_ollie_id != -1 {
            self.screens.destroy(self.gesture_ollie_id);
            self.gesture_ollie_id = -1;
        }
        self.ice_opened = false;
    }
}

#[cfg(test)]
#[path = "fe_overlays_tests.rs"]
mod tests;
