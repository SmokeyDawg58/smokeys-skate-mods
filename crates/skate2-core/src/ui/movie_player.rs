//! `FrontEndState_MoviePlayer` (FE state 28, screen 33 `movieplayer.swf`), the front end
//! manager's `FEMoviePlayer` (`FrontEndManager::mMoviePlayer`, +0x1528) and the front end's
//! side of `AptMovieIntegration` (`AptIntegration::m_AptMovieInt`): which `fe_movie` plays, when
//! it may be skipped or left, and the requests the presentation's `AptMovieIntegration::Render`
//! (0x82630198) serves. Decoding, drawing and the movie's sound belong to the platform.
//!
//! One front-end update is `FrontEndManager::Update` (0x82640130): `FEMoviePlayer::Update`, then
//! the pads' input (`ReceiveInput`), then `StateManagerFrontEndBase::Update` (pending pushes and
//! pops, then the state's `OnUpdate`).

use super::pause_menu::PauseMenuInput;
use crate::challenge::attrib_db::AttribDb;
use crate::nis::subtitles::{self, SubTitler};
use std::collections::{HashMap, VecDeque};

/// `Attrib::Gen::fe_movie`'s class.
pub const FE_MOVIE: u64 = 0x18b2_7535_7481_fb52;
/// `god_intro1`, which `StateMainMenu::NewCareerIntro` (0x82763790) plays.
pub const GOD_INTRO1: u64 = 0x5d7e_89af_30f2_7698;
/// `redacarride`, which `StateMainMenu::RedaIntro` (0x8275f5f8) plays.
pub const REDA_CAR_RIDE: u64 = 0xdea3_0a60_1f90_d8ba;
/// `ea_intro` and `blackbox_bumper`, which `StateBootFlow::Splash1State` (0x827773a0) and
/// `Splash2State` (0x827772a8) play before the main menu.
const BOOT_FLOW_MOVIES: [u64; 2] = [0x2ba7_201f_c1c3_d6c5, 0x09b1_baf7_898d_2ed0];

/// The `fe_movie` keys `OnEnter` (0x8265bb08) shows under the front end at
/// `kTutorialMovieScale`, with the FMV HUD screen template it opens for each:
/// `tutorial_basicreplay`, `tutorial_map`, `tutorial_createaspot`,
/// `tutorial_advancedreplay_dlc` and `hom`.
const TUTORIAL_SCREENS: [(u64, i32); 5] = [
    (0x4ff5_292a_7dfb_6081, 0x8A),
    (0x299b_75f9_860c_b72c, 0x8B),
    (0x1970_dd9e_472a_7126, 0x8C),
    (0xfce8_a97a_f24d_f012, 0x8D),
    (0x3404_9abc_e23c_10ac, 0x8E),
];

/// `kTutorialMovieScale` ("FE_TutorialMovie", default 0.85).
const TUTORIAL_MOVIE_SCALE: f32 = 0.85;

/// The raw step `GameModule::Update` (0x8273ee88) passes `FrontEndManager::Update`, which hands
/// it to `FEMoviePlayer::Update` and its subtitler unchanged.
pub const FE_UPDATE_DT: u32 = 0x682;

/// `fe_movie` attributes outside the layout.
mod field {
    /// `Play` (0x82638fb8): milliseconds before `IsInterruptible`.
    pub const INTERRUPT_THRESHOLD: u64 = 0x0ac1_1eda_5b6e_4e50;
    /// `Play`: subtitles forced on for French and German in Europe.
    pub const EUROPEAN_SUBTITLES: u64 = 0x2914_bca4_0bc7_de58;
    /// `CanLoop` (0x82615378).
    pub const CAN_LOOP: u64 = 0x6fc6_9d07_c414_57af;
    /// `Play` pushes and `Stop` (0x82639380) pulls `kPAUSE_STATE_FRONTEND` with it set.
    pub const PAUSE_GAME: u64 = 0x7b15_f1c2_4329_0782;
    /// `CanBackOut` (0x826152f8).
    pub const CAN_BACK_OUT: u64 = 0x9a07_a75e_2c18_6b08;
    /// `Play`: the movie has subtitles.
    pub const SUBTITLED: u64 = 0xa3a0_61ec_3931_b4df;
    /// `Play`: the directory under `data` ("movies").
    pub const PATH: u64 = 0xc4b1_8d4b_2056_ccf1;
}

/// The `fe_movie` values the player reads. A missing collection or attribute
/// reads `Attrib::DefaultDataArea`: zeros, empty text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FeMovie {
    /// Layout text 0: the file name `GetFMVMovieKeyFromName` matches and the
    /// subtitler loads.
    pub file_name: String,
    /// Layout float 2.
    pub volume: f32,
    /// Layout byte 12: set, `Play` keeps the interrupt threshold it had.
    pub keep_threshold: bool,
    pub interrupt_threshold: f32,
    pub european_subtitles: bool,
    pub can_loop: bool,
    pub pause_game: bool,
    pub can_back_out: bool,
    pub subtitled: bool,
    pub path: String,
}

impl FeMovie {
    #[must_use]
    pub fn load(db: &AttribDb, key: u64) -> Self {
        let attribute = |field| db.attribute(FE_MOVIE, key, field);
        let flag = |field| attribute(field).and_then(|a| a.bool(0)).unwrap_or(false);
        Self {
            file_name: db
                .layout_text(FE_MOVIE, key, 0)
                .unwrap_or_default()
                .to_owned(),
            volume: f32::from_bits(db.layout_word(FE_MOVIE, key, 2)),
            keep_threshold: db.layout_byte(FE_MOVIE, key, 12) != 0,
            interrupt_threshold: attribute(field::INTERRUPT_THRESHOLD)
                .and_then(|a| a.f32(0))
                .unwrap_or(0.0),
            european_subtitles: flag(field::EUROPEAN_SUBTITLES),
            can_loop: flag(field::CAN_LOOP),
            pause_game: flag(field::PAUSE_GAME),
            can_back_out: flag(field::CAN_BACK_OUT),
            subtitled: flag(field::SUBTITLED),
            path: attribute(field::PATH)
                .and_then(|a| a.text(0))
                .unwrap_or_default()
                .to_owned(),
        }
    }
}

/// `cSignPostUtil::GetFMVMovieKeyFromName` (0x825ca340): the last `fe_movie`
/// collection, in `AttribSysUtils::GetCollections` order, whose file name is
/// `name` (compared case-sensitively); 0 for none.
#[must_use]
pub fn fmv_movie_key_from_name(db: &AttribDb, name: &str) -> u64 {
    db.table_order(FE_MOVIE)
        .into_iter()
        .filter(|&key| db.layout_text(FE_MOVIE, key, 0) == Some(name))
        .last()
        .unwrap_or(0)
}

/// `FEMoviePlayer::eMovieState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MovieState {
    None,
    /// The front end manager's constructor (0x82635f18) and `OnEnter` set it.
    #[default]
    Initialized,
    Playing,
    Interrupted,
    Done,
}

/// A request to the presentation, in the order the front end made it.
#[derive(Clone, Debug, PartialEq)]
pub enum MovieRequest {
    /// `AudioSystem::PauseForMovie` (`OnEnter`).
    PauseAudio,
    /// `AudioSystem::ResumeAfterMovie` (`OnLeave`).
    ResumeAudio,
    /// `AptMovieIntegration::Play` after `Init` (0x82634c30).
    Play(MovieSettings),
    /// `AptMovieIntegration::Stop` (0x82633b08).
    Stop,
}

/// What `AptMovieIntegration::DoPlay` (0x8260a248) reads when it starts the movie.
#[derive(Clone, Debug, PartialEq)]
pub struct MovieSettings {
    /// `Init`'s `data\<path>\<file>.vp6` (`GetMovieType` 0 for a name without extension).
    pub file: String,
    /// `mFilter`: the audio language, `EN` for English.
    pub audio_filter: [u8; 2],
    /// The `fe_movie` volume, which `FEMoviePlayer::Play` multiplies by
    /// `AudioSystem::GetMovieVolume` for `Init`.
    pub volume: f32,
    /// `mScaleX` and `mScaleY`.
    pub scale: f32,
}

/// What the presentation reports after its `AptMovieIntegration::Render`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoviePresentation {
    /// `mMoviePlayer` exists.
    pub has_player: bool,
    /// `mIsPlaying`.
    pub playing: bool,
    /// `RequestStop` (0x8260a458) ran: the movie ended or a stop was taken.
    pub stop_requested: bool,
}

/// What the front end manager did that its owner carries out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MovieEvent {
    /// `cMsgFMVFinishedPlaying` (0xACAEBECE) to the challenge system.
    Finished,
    /// `FrontEndManager::EnableBackground(enable, snap, ...)` (0x826066c8).
    Background { enable: bool, snap: bool },
}

/// The front end's half of `AptMovieIntegration`.
#[derive(Debug)]
struct MovieIntegration {
    /// `mRequests` not yet taken by the presentation.
    requests: VecDeque<MovieRequest>,
    presentation: MoviePresentation,
    /// `mbForeground`: drawn after the front end's layers (1 from the constructor 0x82631cb0).
    foreground: bool,
    /// `mScaleX`/`mScaleY` (1.0 from the constructor).
    scale: f32,
    /// `mbForceBlackOnExit`.
    force_black_on_exit: bool,
}

impl Default for MovieIntegration {
    fn default() -> Self {
        Self {
            requests: VecDeque::new(),
            presentation: MoviePresentation::default(),
            foreground: true,
            scale: 1.0,
            force_black_on_exit: false,
        }
    }
}

impl MovieIntegration {
    /// `IsPlaying` (0x8260a1d8): playing, opening, or a request waiting.
    fn is_playing(&self) -> bool {
        self.presentation.playing
            || self
                .requests
                .iter()
                .any(|r| matches!(r, MovieRequest::Play(_) | MovieRequest::Stop))
    }

    /// `Init` with `startPlaying`: refused while a movie player exists; English
    /// filters the audio to `EN`.
    fn init_and_play(&mut self, path: &str, file_name: &str, volume: f32) -> bool {
        if self.presentation.has_player {
            return false;
        }
        let settings = MovieSettings {
            file: format!("data\\{path}\\{file_name}.vp6"),
            audio_filter: *b"EN",
            volume,
            scale: self.scale,
        };
        self.requests.push_back(MovieRequest::Play(settings));
        true
    }

    /// `Stop`: with `mbForceBlackOnExit` the background snaps on first.
    fn stop(&mut self, events: &mut Vec<MovieEvent>) {
        if std::mem::take(&mut self.force_black_on_exit) {
            events.push(MovieEvent::Background {
                enable: true,
                snap: true,
            });
        }
        self.requests.push_back(MovieRequest::Stop);
    }
}

/// `FEMoviePlayer`.
#[derive(Debug)]
struct FeMoviePlayer {
    /// `mInterruptTimer` in milliseconds.
    interrupt_timer: f32,
    /// `mInterruptThreshold`, 1.0 from the front end manager's constructor.
    interrupt_threshold: f32,
    subtitler: Option<SubTitler>,
    state: MovieState,
    /// `mMovie`.
    movie: u64,
    /// `mMovieInterupted`.
    interrupted: bool,
    /// `mbDisableBackground`.
    disable_background: bool,
}

impl Default for FeMoviePlayer {
    fn default() -> Self {
        Self {
            interrupt_timer: 0.0,
            interrupt_threshold: 1.0,
            subtitler: None,
            state: MovieState::Initialized,
            movie: 0,
            interrupted: false,
            disable_background: false,
        }
    }
}

impl FeMoviePlayer {
    /// `IsInterruptible` (0x82615288): past the threshold, which also marks the
    /// movie interrupted.
    fn is_interruptible(&mut self) -> bool {
        let interruptible = self.interrupt_timer > self.interrupt_threshold;
        if interruptible {
            self.interrupted = true;
        }
        interruptible
    }
}

/// `FrontEndManager::mGenericSubtitles` as the movie player drives it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MovieSubtitle {
    /// `mSubtitleText`, as the HAL id `cMsgSetSubtitle` carried.
    pub text: Option<u32>,
    /// `mbSubtitleBottom`.
    pub bottom: bool,
    /// `mbOverrideProfileSetting`: shown even with the profile's subtitles off.
    pub override_profile: bool,
}

/// FE28's loaded APT screens. The generation identifies this open operation
/// so a late presentation acknowledgement cannot ready a replacement movie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MovieScreen {
    pub generation: u64,
    pub tutorial: Option<i32>,
    pub playing: bool,
}

/// `FrontEndState_MoviePlayer` while it is on the stack.
#[derive(Debug, Default)]
struct FeState {
    /// `mbPlaying` (+0x3C).
    playing: bool,
    /// `mFMVHUDFlaggedToPlay` (+0x3E).
    hud_flagged: bool,
    /// `mScreenId` is set: `movieplayer.swf` loaded.
    screen_ready: bool,
    /// The FMV HUD screen template opened, and whether `FMVHUDScreenReady` set
    /// `mFMVHUDScreenId`.
    hud_screen: Option<(i32, bool)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Transition {
    Push,
    Pop,
}

/// What the player needs from its owner for one update.
#[derive(Clone, Copy, Debug)]
pub struct MovieContext<'a> {
    /// The profile's subtitles option (`ProfileBackEnd::tSettings::mSubtitles`).
    pub subtitles: bool,
    /// `data\fe\subtitles\subtitles.big`, for a subtitled movie's `SubTitler`.
    pub subtitle_big: Option<&'a [u8]>,
    /// The front end has focus: the pads' input reaches the states.
    pub has_focus: bool,
}

/// The movie player of the front end manager and its FE state.
#[derive(Debug)]
pub struct MoviePlayer {
    /// `FrontEndState_MoviePlayer::mMovieKey`.
    movie_key: u64,
    /// FE state 29: credits intro completed, and ExitCredits requested.
    credits: Option<(bool, bool)>,
    state: Option<FeState>,
    screen_generation: u64,
    /// Pushes and pops of FE state 28 the state manager has not run yet, in order.
    pending: VecDeque<Transition>,
    /// `mIsSkippable_Override` (+0x3D), set by `StateManagerFrontEnd::Initialize` (0x8268ca08)
    /// and every `OnLeave`.
    skippable_override: bool,
    /// `FrontEndManager::mAlwaysAllowMovieSkip` (+0x2305), cleared by its constructor.
    always_allow_skip: bool,
    player: FeMoviePlayer,
    integration: MovieIntegration,
    subtitle: MovieSubtitle,
    events: Vec<MovieEvent>,
    /// The `fe_movie` collections, read once.
    movies: Option<HashMap<u64, FeMovie>>,
}

impl Default for MoviePlayer {
    fn default() -> Self {
        Self {
            movie_key: u64::MAX,
            credits: None,
            state: None,
            screen_generation: 0,
            pending: VecDeque::new(),
            skippable_override: true,
            always_allow_skip: false,
            player: FeMoviePlayer::default(),
            integration: MovieIntegration::default(),
            subtitle: MovieSubtitle::default(),
            events: Vec::new(),
            movies: None,
        }
    }
}

impl MoviePlayer {
    /// The `fe_movie` collections have been read.
    #[must_use]
    pub fn has_movies(&self) -> bool {
        self.movies.is_some()
    }

    /// Reads every `fe_movie` collection.
    pub fn load_movies(&mut self, db: &AttribDb) {
        self.movies = Some(
            db.collections(FE_MOVIE)
                .iter()
                .map(|&key| (key, FeMovie::load(db, key)))
                .collect(),
        );
    }

    /// The interrupt threshold the boot flow's two movies leave: each `Play` takes its movie's
    /// unless the movie keeps the one before. The port starts at the main menu without them.
    pub fn boot_flow_played(&mut self) {
        for key in BOOT_FLOW_MOVIES {
            let movie = self.movie(key);
            if !movie.keep_threshold {
                self.player.interrupt_threshold = movie.interrupt_threshold;
            }
        }
    }

    /// An `fe_movie` instance's values; `DefaultDataArea` for an unknown key.
    fn movie(&self, key: u64) -> FeMovie {
        self.movies
            .as_ref()
            .and_then(|movies| movies.get(&key))
            .cloned()
            .unwrap_or_default()
    }

    /// `mMovieKey = key`, then `PushState(28)`: `cSignPostUtil::HandleCaughtMessage`
    /// (0x825de4e0, message type 15) and the sign-up poster's tutorial video.
    pub fn push_state(&mut self, key: u64) {
        self.movie_key = key;
        self.pending.push_back(Transition::Push);
    }

    /// `mMovieKey = key`, `ForceBlackOnClose` (0x8264b8a8) and `SetState(28, -1)`:
    /// the new career's `NewCareerIntro` and `RedaIntro`.
    pub fn set_state_black_on_close(&mut self, key: u64) {
        self.movie_key = key;
        self.integration.force_black_on_exit = true;
        self.pending.push_back(Transition::Push);
    }

    /// `StateManagerFrontEndBase::IsStateInStack(28)`.
    #[must_use]
    pub fn state_in_stack(&self) -> bool {
        self.state.is_some()
    }

    pub fn screen(&self) -> Option<MovieScreen> {
        self.state.as_ref().map(|state| MovieScreen {
            generation: self.screen_generation,
            tutorial: state.hud_screen.map(|(template, _)| template),
            playing: state.playing,
        })
    }

    /// ScreenReady / FMVHUDScreenReady826438b8 after the presentation has
    /// constructed both requested screens and run their OnLoaded methods.
    pub fn acknowledge_screen_loaded(&mut self, generation: u64) {
        if generation == self.screen_generation && let Some(state) = self.state.as_mut() {
            state.screen_ready = true;
            if let Some((_, ready)) = state.hud_screen.as_mut() { *ready = true; }
        }
    }

    /// FE state 28 is on the stack or about to be: its `gFEStateInfo` row
    /// (0x82fded80) pauses the game.
    #[must_use]
    pub fn holds_front_end(&self) -> bool {
        self.credits.is_some() || self.state.is_some() || self.pending.contains(&Transition::Push)
    }

    /// `AptMovieIntegration::IsPlaying`.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.integration.is_playing()
    }

    /// `mbForeground`: the movie draws after the front end's layers.
    #[must_use]
    pub fn foreground(&self) -> bool {
        self.integration.foreground
    }

    #[must_use]
    pub fn subtitle(&self) -> MovieSubtitle {
        self.subtitle
    }

    /// The presentation's requests since the last call.
    pub fn take_requests(&mut self) -> Vec<MovieRequest> {
        self.integration.requests.drain(..).collect()
    }

    /// The front end manager's events since the last call.
    pub fn take_events(&mut self) -> Vec<MovieEvent> {
        std::mem::take(&mut self.events)
    }

    /// The presentation's state after its render. `RequestStop` snaps the
    /// background on for a movie flagged `ForceBlackOnClose`.
    pub fn set_presentation(&mut self, presentation: MoviePresentation) {
        if presentation.stop_requested && std::mem::take(&mut self.integration.force_black_on_exit)
        {
            self.events.push(MovieEvent::Background {
                enable: true,
                snap: true,
            });
        }
        self.integration.presentation = presentation;
    }

    /// Credits::OnEnter8264a550: separate state 29, movie behind scrolling text.
    pub fn force_movie_background(&self) -> bool { self.credits.is_some() }
    pub fn push_credits(&mut self) {
        self.credits = Some((false, false));
        self.movie_key = super::extras::CREDITS_MOVIE;
        self.integration.foreground = false;
        self.integration.scale = 1.0;
        self.player.state = MovieState::Initialized;
        self.integration.requests.push_back(MovieRequest::PauseAudio);
    }
    pub fn acknowledge_credits(&mut self, intro: bool, done: bool) {
        if let Some((ready, exit)) = self.credits.as_mut() { *ready |= intro; *exit |= done; }
    }

    /// One front-end update.
    pub fn update(&mut self, input: PauseMenuInput, context: &MovieContext<'_>) {
        self.player_update();
        // Credits::OnUpdate8265a318 loops its background movie until the APT roll ends.
        if let Some((ready, exit)) = self.credits {
            if exit || (context.has_focus && input.cancel && self.movie(self.movie_key).can_back_out) {
                self.credits = None;
                self.on_leave();
            } else if ready && self.player.state != MovieState::Playing && !self.integration.presentation.has_player {
                self.play(self.movie_key, context);
            }
            return;
        }
        if context.has_focus
            && self.pending.is_empty()
            && self.state.as_ref().is_some_and(|state| state.screen_ready)
        {
            if input.start {
                self.receive_input(0);
            }
            if input.select {
                self.receive_input(2);
            }
            if input.cancel {
                self.receive_input(3);
            }
        }
        match self.pending.pop_front() {
            Some(Transition::Push) if self.state.is_none() => {
                self.state = Some(FeState::default());
                self.on_enter();
            }
            Some(Transition::Pop) if self.state.is_some() => {
                self.on_leave();
                self.state = None;
            }
            _ => {
                if self.state.is_some() {
                    self.on_update(context);
                }
            }
        }
    }

    /// `FEMoviePlayer::Update` (0x82628df0): the interrupt timer gains a hundredth of the step
    /// and the background goes once; a movie no longer playing is done, which
    /// `cMsgFMVFinishedPlaying` reports; one still playing feeds its subtitler.
    fn player_update(&mut self) {
        if self.player.state == MovieState::Playing {
            #[expect(
                clippy::cast_precision_loss,
                reason = "the native (float)(u64) of the step over 100"
            )]
            let step = (FE_UPDATE_DT / 100) as f32;
            self.player.interrupt_timer = step + self.player.interrupt_timer;
            if std::mem::take(&mut self.player.disable_background) {
                self.events.push(MovieEvent::Background {
                    enable: false,
                    snap: false,
                });
            }
        }
        if self.player.state != MovieState::Playing {
            return;
        }
        if self.integration.is_playing() {
            if let Some(hal) = self
                .player
                .subtitler
                .as_mut()
                .and_then(|subtitler| subtitler.update(FE_UPDATE_DT))
            {
                self.subtitle.text = (hal != 0).then_some(hal);
            }
        } else {
            self.player.state = MovieState::Done;
            self.events.push(MovieEvent::Finished);
            self.player.interrupted = false;
        }
    }

    /// `FrontEndState_MoviePlayer::ReceiveInput` (0x8265bea0) while the movie
    /// plays: Start (0) or A (2) once interruptible, or at once for a movie
    /// that cannot be backed out of; B (3) for one that can.
    fn receive_input(&mut self, keycode: i32) {
        if !self.skippable_override || self.player.state != MovieState::Playing {
            return;
        }
        match keycode {
            0 | 2 => {
                if (self.always_allow_skip || self.player.is_interruptible())
                    && (self.player.is_interruptible()
                        || !self.movie(self.player.movie).can_back_out)
                {
                    self.pending.push_back(Transition::Pop);
                }
            }
            3 if self.movie(self.player.movie).can_back_out => {
                self.pending.push_back(Transition::Pop);
            }
            _ => {}
        }
    }

    /// `SetState` (0x82643820, `mbPlaying` cleared) and `OnEnter` (0x8265bb08): the music paused,
    /// the movie player initialized and screen 33 opened; the five tutorial movies draw under the
    /// front end at `kTutorialMovieScale` and wait for their FMV HUD screen.
    fn on_enter(&mut self) {
        self.screen_generation = self.screen_generation.wrapping_add(1);
        let Some(state) = self.state.as_mut() else {
            return;
        };
        state.playing = false;
        state.hud_flagged = false;
        self.integration
            .requests
            .push_back(MovieRequest::PauseAudio);
        self.player.state = MovieState::Initialized;
        if let Some(&(_, template)) = TUTORIAL_SCREENS
            .iter()
            .find(|&&(key, _)| key == self.movie_key)
        {
            self.integration.foreground = false;
            self.integration.scale = TUTORIAL_MOVIE_SCALE;
            state.hud_screen = Some((template, false));
            state.hud_flagged = true;
        }
    }

    /// `OnUpdate` (0x8266a880): both screens must have reported OnLoaded, and
    /// the preceding player must have completed its pending stop.
    fn on_update(&mut self, context: &MovieContext<'_>) {
        let can_loop = self.movie(self.player.movie).can_loop;
        let player_playing = self.player.state == MovieState::Playing;
        let loaded = !self.integration.presentation.has_player;
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let play = if state.playing {
            if !player_playing && !can_loop {
                state.playing = false;
                self.pending.push_back(Transition::Pop);
                return;
            }
            !player_playing
        } else {
            if state.hud_screen.is_some_and(|(_, ready)| ready) {
                state.hud_flagged = false;
            }
            state.screen_ready && !state.hud_flagged && loaded
        };
        if play {
            self.play_movie(context);
        }
    }

    /// `PlayMovie` (0x8265bf98).
    fn play_movie(&mut self, context: &MovieContext<'_>) {
        if let Some(state) = self.state.as_mut() {
            state.playing = true;
        }
        self.play(self.movie_key, context);
    }

    /// `FEMoviePlayer::Play` (0x82638fb8): the threshold unless the movie keeps it, the
    /// subtitles at the bottom (a subtitled movie with them on draws under the front end and
    /// starts its `SubTitler`), then `Init` of `<file>_english_ntsc` with `Play`.
    fn play(&mut self, key: u64, context: &MovieContext<'_>) {
        self.player.movie = key;
        self.player.state = MovieState::Playing;
        let movie = self.movie(key);
        if !movie.keep_threshold {
            self.player.interrupt_threshold = movie.interrupt_threshold;
        }
        self.subtitle = MovieSubtitle {
            text: None,
            bottom: true,
            override_profile: false,
        };
        if movie.subtitled && context.subtitles {
            self.integration.foreground = false;
            let keys = context
                .subtitle_big
                .and_then(|big| subtitles::load(big, "English", &movie.file_name));
            self.player.subtitler = Some(SubTitler::start(keys));
        }
        self.player.interrupt_timer = 0.0;
        self.integration.init_and_play(
            &movie.path,
            &format!("{}_english_ntsc", movie.file_name),
            movie.volume,
        );
        self.player.disable_background = true;
    }

    /// `OnLeave` (0x8265bd78): the key cleared, the screens closed, the movie stopped, the music
    /// resumed and the movie drawn in the foreground at full scale again.
    fn on_leave(&mut self) {
        self.skippable_override = true;
        self.movie_key = u64::MAX;
        self.stop();
        self.integration
            .requests
            .push_back(MovieRequest::ResumeAudio);
        self.integration.foreground = true;
        self.integration.scale = 1.0;
    }

    /// `FEMoviePlayer::Stop` (0x82639380): the subtitler and the subtitle line go, then
    /// `AptMovieIntegration::Stop`.
    fn stop(&mut self) {
        self.player.subtitler = None;
        self.subtitle = MovieSubtitle::default();
        self.integration.stop(&mut self.events);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_threshold_marks_the_movie_interrupted() {
        let mut player = FeMoviePlayer {
            interrupt_threshold: 5000.0,
            ..FeMoviePlayer::default()
        };
        player.interrupt_timer = 5000.0;
        assert!(!player.is_interruptible());
        assert!(!player.interrupted);
        player.interrupt_timer += 16.0;
        assert!(player.is_interruptible());
        assert!(player.interrupted);
    }

    #[test]
    fn stop_snaps_the_background_once_for_black_on_close() {
        let mut integration = MovieIntegration {
            force_black_on_exit: true,
            ..MovieIntegration::default()
        };
        let mut events = Vec::new();
        integration.stop(&mut events);
        integration.stop(&mut events);
        assert_eq!(
            events,
            [MovieEvent::Background {
                enable: true,
                snap: true
            }]
        );
        assert_eq!(integration.requests.len(), 2);
        assert!(integration.is_playing());
    }
}
