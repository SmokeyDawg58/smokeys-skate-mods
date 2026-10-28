//! Slappy's blog on the career: `FrontEndState_SlappysBlog` (FE state 61), which
//! `FrontEndState_Unlock::SaveAndClose` (`0x82659468`) sets after an overlay that unlocked a
//! magazine's coverage, its articles (`FrontEndState_SlappysArticles`, 62) and covers
//! (`FrontEndState_SlappysCovers`, 63). A push leaves the state below (`UpdateOnLeave` closes
//! its screen) and a pop enters it again (`UpdateOnEnter` opens a new one), so only the top
//! state's screen is open.

use super::{CareerSession, DialogOwner, PausingState, SelectedLiveSession};
use crate::{
    challenge::{
        Event, photo_articles,
        signpost_progress::blog_type,
        slappys_blog::{self, BlogEntry, SlappysCall, SlappysHud},
    },
    ui::{
        pause_menu::PauseMenuInput,
        slappys_blog::{ArticleScreen, BlogScreen, Key},
    },
};

/// The blog states' data.
#[derive(Debug, Default)]
pub(super) struct Slappys {
    /// FE state 61's `mUseHudContinueMessage`.
    use_hud_continue: bool,
    /// FE state 61's `mMenuStack` of `eSlappysScreenMode`.
    menu_stack: Vec<i32>,
    /// FE state 61's `mSlappysBlogList`.
    list: Vec<BlogEntry>,
    /// The blog screen's `slappys_blog` instance.
    blog: BlogScreen,
    /// FE state 62's `mPhotoArticleKey` and its screen's `articles` instance.
    article: u64,
    article_screen: ArticleScreen,
    /// FE state 63's `mPhotoArticleKey`, `mCurrentIndex` and `mbRequestPhoto`.
    cover_article: u64,
    cover_index: i32,
    request_photo: bool,
    /// `FrontEndManager::mbFadeBackground`: a background fade runs.
    fading: bool,
    /// The top state's screen reported `IntroComplete` (`IsScreenReady`).
    ready: bool,
    /// `CACRecord::IsCharacterMale` of player 1, for `GetGenderizedHALID`.
    male: bool,
    /// The profile's name (`ProfileBackEnd`), for `Game.GetProfileName`.
    profile_name: String,
    /// `FrontEndManager::EnableBackground(enable, snap)` calls since the session last took them.
    background: Vec<(bool, bool)>,
}

/// `FE_SOUND_CORE_B_BUTTON` (`0x4d927cd64cc8b818`), which the states' `ReceiveInput` plays for B.
const B_BUTTON: &str = "FE_SOUND_CORE_B_BUTTON";

impl CareerSession {
    /// `SaveAndClose`'s `SetState(61)` with `mUseHudContinueMessage` set (the unlock overlay's
    /// state has already left).
    pub(crate) fn open_slappys_blog(&mut self, use_hud_continue: bool, male: bool, profile_name: String) {
        self.slappys.use_hud_continue = use_hud_continue;
        self.slappys.male = male;
        self.slappys.profile_name = profile_name;
        self.pausing.push(PausingState::Blog);
        self.enter_blog();
    }

    /// FE state 61's `OnEnter` (`0x8268fc80`): `OpenScreen(125)`, a new `slappys_blog`, whose
    /// load runs `OnLoaded` (`0x826b1308`): the menu stack emptied, then `PushScreenState(0)`.
    pub(super) fn enter_blog(&mut self) {
        self.slappys.ready = false;
        self.slappys.blog = BlogScreen::new();
        self.slappys.menu_stack.clear();
        self.push_screen_state(slappys_blog::mode::SLAPPYS);
    }

    /// `PushScreenState(mode)` (`0x826b0940`) and `PopScreenState` (`0x826b09c8`) end here:
    /// `UpdateBlogList` (`0x826ac3e8`), then `SetScreenState(mode)`.
    fn set_blog_mode(&mut self) {
        let mode = self.slappys.menu_stack.last().copied().unwrap_or_default();
        let world = self.system.world();
        self.slappys.list = slappys_blog::blog_list(world, mode);
        let view = slappys_blog::blog_view(world, mode, &self.slappys.list, self.slappys.male);
        let count = if mode < 3 { i32::try_from(self.slappys.list.len()).unwrap_or(i32::MAX) } else { 0 };
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            let rows: Vec<String> = self.slappys.list.iter().zip(&view.rows).map(|(entry, row)| format!("{}:{}{}", entry.kind, row.text, if row.photo_slot.is_some() { "*" } else { "" })).collect();
            eprintln!("career: Slappy's blog mode {mode}: {rows:?}");
        }
        let mut calls = Vec::new();
        self.slappys.blog.set_state(mode, count, &mut calls);
        self.hud.slappys = Some(SlappysHud::Blog { view });
        self.hud.slappys_screen.extend(calls.into_iter().map(|(call, screen)| SlappysCall::Blog(call, screen)));
    }

    fn push_screen_state(&mut self, mode: i32) {
        self.slappys.menu_stack.push(mode);
        self.set_blog_mode();
    }

    /// FE state 61's `OnLeave` (`0x82693480`): `ResetNewBLOGCount`, then `CloseScreen`.
    fn leave_blog(&mut self) {
        slappys_blog::reset_new_blog_count(self.system.world_mut());
    }

    /// FE state 62's `OnEnter` (`0x8268fcd8`): `OpenScreen(126)`, whose load
    /// (`OnLoaded`, `SetScreenState(0)`) runs `SetState(0)`.
    fn enter_articles(&mut self) {
        self.slappys.ready = false;
        let world = self.system.world();
        let view = slappys_blog::article_view(world, self.slappys.article, self.slappys.male);
        let count = photo_articles::article_photo_count(world, self.slappys.article);
        self.slappys.article_screen = ArticleScreen::new(count);
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career: Slappy's article {} {:?}: photos {:?}, text {}", view.banner, view.title, view.photo_slots, view.text);
        }
        self.hud.slappys = Some(SlappysHud::Article { view, screen: self.slappys.article_screen });
        self.hud.slappys_screen.push(SlappysCall::ArticleState);
    }

    /// FE state 63's `OnEnter` (`0x8269d350`): `EnableBackground(1, 1)`, then `OpenScreen(127)`,
    /// whose load (`OnLoaded` `0x826a1d50`) runs `UpdateCover`.
    fn enter_covers(&mut self) {
        self.slappys.ready = false;
        self.enable_background(true, true);
        self.update_cover();
    }

    /// `FrontEndManager::EnableBackground(enable, snap)` (`0x826066c8`).
    fn enable_background(&mut self, enable: bool, snap: bool) {
        self.slappys.fading = true;
        self.slappys.background.push((enable, snap));
    }

    /// `UpdateCover` (`0x8269d428`): the background snapped on unless a fade runs, the photo
    /// requested, and `SetScreenState` of the cover's state.
    fn update_cover(&mut self) {
        if !self.slappys.fading {
            self.enable_background(true, true);
        }
        self.slappys.request_photo = true;
        let mut view = slappys_blog::cover_view(self.system.world(), self.slappys.cover_article, self.slappys.cover_index);
        view.profile_name = format!("#{}", self.slappys.profile_name);
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career: Slappy's cover state {} photo {} slot {} help {}", view.state, self.slappys.cover_index, view.photo_slot, view.trigger_text);
        }
        self.hud.slappys_screen.push(SlappysCall::Cover(view.state));
        self.hud.slappys = Some(SlappysHud::Cover { view });
    }

    /// `PushState(state)` over a blog state: its `OnLeave`, then the new state's `OnEnter`.
    fn push_slappys_state(&mut self, state: PausingState) {
        self.leave_slappys_state();
        self.pausing.push(state);
        match state {
            PausingState::Articles => self.enter_articles(),
            PausingState::Covers => self.enter_covers(),
            _ => self.enter_blog(),
        }
    }

    /// The top blog state's `OnLeave`.
    pub(super) fn leave_slappys_state(&mut self) {
        match self.pausing.last() {
            Some(PausingState::Blog) => self.leave_blog(),
            // FE state 63's OnLeave (0x8268fd40).
            Some(PausingState::Covers) => self.slappys.request_photo = false,
            _ => {}
        }
        self.slappys.ready = false;
    }

    /// `PopState` of the top blog state: its `OnLeave`, then the state below enters again, or
    /// the game resumes.
    fn pop_slappys_state(&mut self) {
        self.leave_slappys_state();
        self.pausing.pop();
        match self.pausing.last() {
            Some(PausingState::Blog) => self.enter_blog(),
            Some(PausingState::Articles) => self.enter_articles(),
            Some(PausingState::Covers) => self.enter_covers(),
            _ => self.hud.slappys = None,
        }
    }

    /// The FE states' `SetScreenInput` key for a front-end input.
    fn slappys_key(input: PauseMenuInput) -> Option<Key> {
        if input.right {
            Some(Key::Right)
        } else if input.left {
            Some(Key::Left)
        } else if input.down {
            Some(Key::Down)
        } else if input.up {
            Some(Key::Up)
        } else {
            input.select.then_some(Key::Next)
        }
    }

    /// FE state 61's `ReceiveInput` (`0x826b1318`) once its screen is ready: B plays its sound and
    /// leaves the magazine for Slappy's own blog, or with Slappy's on top sends `cMsgHUDContinue`
    /// when `SaveAndClose` asked for it and pops the state. Every key then reaches the screen
    /// (`slappys_blog::UpdateInput`), whose choice is `OnMenuSelect` (`0x826b1428`).
    pub(super) fn blog_input(&mut self, input: PauseMenuInput) {
        if !self.slappys.ready {
            return;
        }
        if input.cancel {
            self.sounds.push(B_BUTTON);
            if self.slappys.menu_stack.len() <= 1 {
                if std::mem::take(&mut self.slappys.use_hud_continue) {
                    self.system.post(Event::HudContinue);
                }
                self.pop_slappys_state();
            } else {
                self.slappys.menu_stack.pop();
                self.set_blog_mode();
            }
            return;
        }
        let Some(key) = Self::slappys_key(input) else { return };
        let mut calls = Vec::new();
        let choice = self.slappys.blog.update_input(key, &mut self.sounds, &mut calls);
        self.hud.slappys_screen.extend(calls.into_iter().map(|(call, screen)| SlappysCall::Blog(call, screen)));
        if let Some((menu, index)) = choice {
            self.blog_menu_select(menu, index);
        }
    }

    /// `OnMenuSelect(menu, index)`: the panel's Thrasher and Skateboard Mag rows show their
    /// magazine (`PushScreenState`), its Annals of Meat row opens FE state 77 once `hom_tutorial`
    /// is complete and a warning before; a photo article's row opens its cover when it has one
    /// photo (`SetArticleKeyAndPhotoIndex(key, 1)`), else its article.
    fn blog_menu_select(&mut self, menu: i32, index: i32) {
        if self.slappys.menu_stack.last().is_none_or(|&mode| mode >= 3) {
            return;
        }
        match (menu, index) {
            (0, 0) => self.push_screen_state(slappys_blog::mode::THRASHER),
            (0, 1) => self.push_screen_state(slappys_blog::mode::SBM),
            (0, 2) => {
                let signpost = &self.system.world().signpost;
                if signpost.key_from_title("hom_tutorial").is_some_and(|key| signpost.is_challenge_complete(key)) {
                    self.open_hom_annals();
                } else {
                    self.dialog_owner = DialogOwner::Notice;
                    self.hud.dialog_options = Some(vec!["ID_COMMON_OK".to_owned()]);
                    self.hud.dialog = Some(("ID_COMMON_WARNING_TITLE".to_owned(), "ID_ANNALSOFMEAT_NOTYETUNLOCKED".to_owned(), 0));
                    self.sounds.push("FE_SOUND_CORE_POPUP");
                }
            }
            (1, _) => {
                let Some(&entry) = usize::try_from(index).ok().and_then(|index| self.slappys.list.get(index)) else { return };
                if entry.kind != blog_type::PHOTO_ARTICLE {
                    return;
                }
                if photo_articles::article_photo_count(self.system.world(), entry.key) == 1 {
                    self.slappys.cover_article = entry.key;
                    self.slappys.cover_index = 1;
                    self.push_slappys_state(PausingState::Covers);
                } else {
                    self.slappys.article = entry.key;
                    self.push_slappys_state(PausingState::Articles);
                }
            }
            _ => {}
        }
    }

    /// FE state 62's `ReceiveInput` (`0x8269d298`) once its screen is ready: B plays its sound and
    /// pops the state; every key then reaches the screen (`articles::UpdateInput`), whose A is
    /// `OnMenuSelect` (`0x8269d2f8`): the photo's cover (`SetArticleKeyAndPhotoIndex(key,
    /// index + 1)`).
    pub(super) fn articles_input(&mut self, input: PauseMenuInput) {
        if !self.slappys.ready {
            return;
        }
        if input.cancel {
            self.sounds.push(B_BUTTON);
            self.pop_slappys_state();
            return;
        }
        let Some(key) = Self::slappys_key(input) else { return };
        let (selected, choice) = self.slappys.article_screen.update_input(key, &mut self.sounds);
        if selected {
            self.hud.slappys_screen.push(SlappysCall::Article(self.slappys.article_screen.current));
            if let Some(SlappysHud::Article { screen, .. }) = self.hud.slappys.as_mut() {
                *screen = self.slappys.article_screen;
            }
        }
        if let Some(index) = choice {
            self.slappys.cover_article = self.slappys.article;
            self.slappys.cover_index = index + 1;
            self.push_slappys_state(PausingState::Covers);
        }
    }

    /// FE state 63's `OnUpdate` (`0x8269d3b0`), then its `ReceiveInput` (`0x826a67f0`) once the
    /// screen is ready: a requested photo fades the background out; B plays its sound and pops the
    /// state, the right trigger shows the next photo and the left the previous one
    /// (`NextPhoto` `0x826a1d58`, `PrevPhoto` `0x826a1de0`: with more than one, wrapping).
    pub(super) fn covers_input(&mut self, input: PauseMenuInput) {
        if std::mem::take(&mut self.slappys.request_photo) {
            self.enable_background(false, false);
        }
        if !self.slappys.ready {
            return;
        }
        if input.cancel {
            self.sounds.push(B_BUTTON);
            self.pop_slappys_state();
            return;
        }
        let photos = photo_articles::article_photo_count(self.system.world(), self.slappys.cover_article);
        if photos <= 1 || !(input.right_shift || input.left_shift) {
            return;
        }
        if input.right_shift {
            self.sounds.push("FE_SOUND_CORE_NAV_RIGHT");
            self.slappys.cover_index = if self.slappys.cover_index + 1 > photos { 1 } else { self.slappys.cover_index + 1 };
        } else {
            self.sounds.push("FE_SOUND_CORE_NAV_LEFT");
            self.slappys.cover_index = if self.slappys.cover_index - 1 < 1 { photos } else { self.slappys.cover_index - 1 };
        }
        self.update_cover();
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// A blog state's `OnScreenReady` when its screen's intro reports `IntroComplete`.
    pub fn acknowledge_career_slappys_intro(&mut self) {
        if let Some(career) = self.career.as_mut()
            && career.hud.slappys.is_some()
        {
            career.slappys.ready = true;
        }
    }

    /// Diagnostics: the signpost's `CompleteChallenge` of a progression challenge by name. False
    /// without a career.
    pub fn diagnostic_complete_challenge(&mut self, name: &str) -> bool {
        self.career.as_mut().map(|career| career.system.diagnostic_complete_challenge(name)).is_some()
    }

    /// The blog states' `EnableBackground` calls on the front end's fade, then whether a fade
    /// runs (`mbFadeBackground`, which `UpdateFade` `0x82614f10` clears once it has faded out).
    pub(crate) fn apply_career_fe_background(&mut self) {
        let Some(career) = self.career.as_mut() else { return };
        for (enable, snap) in std::mem::take(&mut career.slappys.background) {
            match (enable, snap) {
                (true, true) => self.frontend_fade.cover_snap(),
                (true, false) => self.frontend_fade.cover(),
                (false, _) => self.frontend_fade.reveal(),
            }
        }
        career.slappys.fading = self.frontend_fade.fading();
    }
}
