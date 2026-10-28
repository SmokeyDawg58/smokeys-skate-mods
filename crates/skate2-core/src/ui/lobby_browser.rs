//! The Lobby Browser over the Online options (PC port addition): the dedicated server's public
//! unranked games, one card each with the challenge's title and its location's small image on
//! the left and the challenge type and the game's phase on the right. Up and down move between
//! the cards, wrapping as `menu_picker` does; A joins the selected game, X asks for the list
//! again and B returns to the Online options. A failed join's notice opens over the list, and
//! its OK asks for the list again.

use super::pause_menu::PauseMenuInput;

/// The cards in view at once.
pub const VISIBLE: usize = 3;

/// One game as its card shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lobby {
    pub game: u32,
    /// The challenge's title (`GetDistrictHALID`), top left.
    pub title: String,
    /// The location's small image under `fe\source\images\locations\`, bottom left.
    pub image: String,
    /// The challenge type's title, top right.
    pub mode: &'static str,
    /// The game's phase, bottom right before its players; none for a free skate, which is no
    /// challenge.
    pub phase: Option<LobbyPhase>,
    pub players: u8,
    pub max_players: u8,
}

/// A challenge game's phase on its card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobbyPhase {
    /// Lobby skate before the challenge.
    PreGame,
    /// The challenge and what follows it.
    InGame,
}

/// Why a join failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    /// The game filled up (`SER_PLAYNOW_FULL`).
    Full,
    /// The game ended or became private (`SER_PLAYNOW_NO_MATCH`).
    Gone,
    Failed,
}

impl Notice {
    /// The notice's title and message.
    #[must_use]
    pub const fn text(self) -> (&'static str, &'static str) {
        match self {
            Self::Full => ("ID_JOIN_ONLINE_GAME_FAILED_POPUP_TITLE", "ID_JOIN_ONLINE_GAME_FAILED_POPUP_MESSAGE"),
            Self::Gone => ("ID_EANATION", "ID_ONLINE_GAMESESSION_NOTEXIST"),
            Self::Failed => ("ID_EANATION", "ID_SIDEKICK_ONLINE_SESSION_FULL_ERROR"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Waiting for the server's list.
    Loading,
    List,
    /// Waiting for the server to put the player in the selected game.
    Joining,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Back,
    /// Asks the server for the list again.
    Refresh,
    Join(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub state: State,
    /// The cards in view, from `first`.
    pub lobbies: Vec<Lobby>,
    pub first: usize,
    /// How many games the list holds.
    pub count: usize,
    pub selected: usize,
    pub notice: Option<Notice>,
}

#[derive(Clone, Debug)]
pub struct LobbyBrowser {
    lobbies: Vec<Lobby>,
    selected: usize,
    first: usize,
    state: State,
    notice: Option<Notice>,
    /// The screen's intro completed: it takes input.
    pub ready: bool,
    sounds: Vec<&'static str>,
}

impl Default for LobbyBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl LobbyBrowser {
    /// The browser opens waiting for the list.
    #[must_use]
    pub const fn new() -> Self {
        Self { lobbies: Vec::new(), selected: 0, first: 0, state: State::Loading, notice: None, ready: false, sounds: Vec::new() }
    }

    #[must_use]
    pub const fn state(&self) -> State {
        self.state
    }

    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }

    /// The server's list: the selection stays on its game while that is listed, else on the
    /// same row.
    pub fn listed(&mut self, lobbies: Vec<Lobby>) {
        let game = self.lobbies.get(self.selected).map(|lobby| lobby.game);
        let row = self.selected.min(lobbies.len().saturating_sub(1));
        self.selected = game.and_then(|game| lobbies.iter().position(|lobby| lobby.game == game)).unwrap_or(row);
        self.lobbies = lobbies;
        self.state = State::List;
        self.scroll();
    }

    /// The join failed: its notice over the list.
    pub fn join_failed(&mut self, notice: Notice) {
        self.state = State::List;
        self.notice = Some(notice);
        self.sounds.push("FE_SOUND_CORE_POPUP");
    }

    /// Keeps the selected card in view and the view full.
    fn scroll(&mut self) {
        self.first = self.first.min(self.lobbies.len().saturating_sub(VISIBLE));
        if self.selected < self.first {
            self.first = self.selected;
        } else if self.selected >= self.first + VISIBLE {
            self.first = self.selected + 1 - VISIBLE;
        }
    }

    pub fn update(&mut self, input: PauseMenuInput) -> Action {
        if !self.ready || self.state == State::Joining {
            return Action::None;
        }
        if self.notice.is_some() {
            if !input.select {
                return Action::None;
            }
            self.notice = None;
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            self.state = State::Loading;
            return Action::Refresh;
        }
        if input.cancel {
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            return Action::Back;
        }
        if self.state != State::List {
            return Action::None;
        }
        if input.misc {
            self.sounds.push("FE_SOUND_CORE_X_BUTTON");
            self.state = State::Loading;
            return Action::Refresh;
        }
        let count = self.lobbies.len();
        for (pressed, up) in [(input.up, true), (input.down, false)] {
            if pressed && count > 0 {
                self.selected = if up { (self.selected + count - 1) % count } else { (self.selected + 1) % count };
                self.sounds.push(if up { "FE_SOUND_CORE_NAV_UP" } else { "FE_SOUND_CORE_NAV_DOWN" });
            }
        }
        self.scroll();
        if input.select
            && let Some(lobby) = self.lobbies.get(self.selected)
        {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            self.state = State::Joining;
            return Action::Join(lobby.game);
        }
        Action::None
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            state: self.state,
            lobbies: self.lobbies.iter().skip(self.first).take(VISIBLE).cloned().collect(),
            first: self.first,
            count: self.lobbies.len(),
            selected: self.selected,
            notice: self.notice,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lobby(game: u32) -> Lobby {
        Lobby { game, title: format!("T{game}"), image: format!("i{game}"), mode: "M", phase: Some(LobbyPhase::PreGame), players: 1, max_players: 8 }
    }

    fn press(f: impl FnOnce(&mut PauseMenuInput)) -> PauseMenuInput {
        let mut input = PauseMenuInput::default();
        f(&mut input);
        input
    }

    fn browser(games: &[u32]) -> LobbyBrowser {
        let mut browser = LobbyBrowser::new();
        browser.ready = true;
        browser.listed(games.iter().copied().map(lobby).collect());
        browser
    }

    #[test]
    fn the_view_follows_the_selection_and_wraps() {
        let mut screen = browser(&[1, 2, 3, 4, 5]);
        for _ in 0..3 {
            screen.update(press(|i| i.down = true));
        }
        let snapshot = screen.snapshot();
        assert_eq!((snapshot.selected, snapshot.first, snapshot.count), (3, 1, 5));
        assert_eq!(snapshot.lobbies.iter().map(|lobby| lobby.game).collect::<Vec<_>>(), [2, 3, 4]);
        screen.update(press(|i| i.down = true));
        screen.update(press(|i| i.down = true));
        assert_eq!((screen.snapshot().selected, screen.snapshot().first), (0, 0));
        screen.update(press(|i| i.up = true));
        assert_eq!((screen.snapshot().selected, screen.snapshot().first), (4, 2));
        assert_eq!(screen.take_sounds().last(), Some(&"FE_SOUND_CORE_NAV_UP"));
    }

    #[test]
    fn a_joins_the_selected_game_and_waits_for_the_server() {
        let mut screen = browser(&[7, 9]);
        screen.update(press(|i| i.down = true));
        assert_eq!(screen.update(press(|i| i.select = true)), Action::Join(9));
        assert_eq!(screen.state(), State::Joining);
        assert_eq!(screen.update(press(|i| i.cancel = true)), Action::None);
        screen.join_failed(Notice::Full);
        assert_eq!(screen.snapshot().notice, Some(Notice::Full));
        assert_eq!(screen.update(press(|i| i.cancel = true)), Action::None);
        assert_eq!(screen.update(press(|i| i.select = true)), Action::Refresh);
        assert_eq!((screen.state(), screen.snapshot().notice), (State::Loading, None));
    }

    #[test]
    fn a_new_list_keeps_the_selected_game() {
        let mut screen = browser(&[1, 2, 3, 4]);
        screen.update(press(|i| i.up = true));
        assert_eq!(screen.update(press(|i| i.misc = true)), Action::Refresh);
        assert_eq!(screen.update(press(|i| i.misc = true)), Action::None);
        screen.listed([5, 4, 1].into_iter().map(lobby).collect());
        assert_eq!((screen.snapshot().selected, screen.snapshot().first), (1, 0));
        screen.listed([8].into_iter().map(lobby).collect());
        assert_eq!(screen.snapshot().selected, 0);
        screen.listed(Vec::new());
        assert_eq!(screen.update(press(|i| i.select = true)), Action::None);
        assert_eq!(screen.update(press(|i| i.cancel = true)), Action::Back);
    }

    #[test]
    fn nothing_moves_before_the_intro_or_the_list() {
        let mut screen = LobbyBrowser::new();
        assert_eq!(screen.update(press(|i| i.cancel = true)), Action::None);
        screen.ready = true;
        assert_eq!(screen.update(press(|i| i.select = true)), Action::None);
        assert_eq!(screen.update(press(|i| i.misc = true)), Action::None);
        assert_eq!(screen.update(press(|i| i.cancel = true)), Action::Back);
    }
}
