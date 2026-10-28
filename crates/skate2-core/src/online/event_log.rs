//! `Sk8::FE::OnlineEventLog`: the five lines of online notices at the top left
//! (`hud2/hud_online_eventlog2.swf`, HUD component `onlineeventlog`). `AddEvent` 0x82619af0
//! starts a line's intro; `Update` 0x82639bd8, run by each `FrontEndManager::Update`, holds it
//! for 1180 updates, plays its outro and drops it at 1200.

use std::collections::VecDeque;

/// `NUM_LOG_ENTRIES`.
pub const LINES: usize = 5;
/// The update a line's intro ends on.
const INTRO_UPDATES: u32 = 20;
/// The update a line's outro starts on.
const OUTRO_UPDATE: u32 = 1180;
/// The update a line is dropped on.
const DROP_UPDATE: u32 = 1200;

/// `eEventLogState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineState {
    Intro = 0,
    Outro = 1,
    Shown = 2,
}

/// A line's text: a localization key formatted with arguments
/// (`LocalizerManager::FormatString`), or a line shown as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventText {
    pub key: &'static str,
    pub arguments: Vec<String>,
    /// The arguments are HAL ids, each formatted first.
    pub localized: bool,
    /// A line shown in place of the key: lobby chat and the pre-game lobby notice (PC port
    /// additions).
    pub literal: Option<String>,
}

impl EventText {
    #[must_use]
    pub fn named(key: &'static str, name: &str) -> Self {
        Self { key, arguments: vec![name.to_owned()], localized: false, literal: None }
    }

    /// A key with no arguments.
    #[must_use]
    pub fn plain(key: &'static str) -> Self {
        Self { key, arguments: Vec::new(), localized: false, literal: None }
    }

    /// A key whose arguments are themselves HAL ids.
    #[must_use]
    pub fn localized(key: &'static str, arguments: Vec<String>) -> Self {
        Self { key, arguments, localized: true, literal: None }
    }

    /// A line shown as it is.
    #[must_use]
    pub fn literal(line: String) -> Self {
        Self { key: "", arguments: Vec::new(), localized: false, literal: Some(line) }
    }

    /// A line of lobby chat.
    #[must_use]
    pub fn chat(line: String) -> Self {
        Self::literal(line)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Line {
    text: EventText,
    timer: u32,
    state: LineState,
}

/// The log, oldest line first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventLog {
    lines: VecDeque<Line>,
    refresh: bool,
}

/// What the HUD movie's `Refresh` reads (`GetLogEntries`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventLogSnapshot {
    pub lines: Vec<(EventText, LineState)>,
}

impl EventLog {
    /// `AddEvent`: a new line in its intro; a full log drops its oldest.
    pub fn add(&mut self, text: EventText) {
        if self.lines.len() == LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(Line { text, timer: 0, state: LineState::Intro });
        self.refresh = true;
    }

    /// `Update`: every line's timer, with a refresh when a line changes state or goes.
    pub fn update(&mut self) {
        let mut changed = false;
        for line in &mut self.lines {
            line.timer += 1;
            if line.timer == INTRO_UPDATES && line.state != LineState::Shown {
                line.state = LineState::Shown;
            } else if line.timer == OUTRO_UPDATE && line.state != LineState::Outro {
                line.state = LineState::Outro;
                changed = true;
            }
        }
        let before = self.lines.len();
        self.lines.retain(|line| line.timer < DROP_UPDATE);
        if changed || self.lines.len() != before {
            self.refresh = true;
        }
    }

    /// `Clear` 0x8261a020, as `OE_GameLeft` and `OE_Disconnected` run it.
    pub fn clear(&mut self) {
        if !self.lines.is_empty() {
            self.lines.clear();
            self.refresh = true;
        }
    }

    /// Whether the movie should `Refresh` (`eASCall_UpdateOnlineEventLog`) since the last call.
    pub fn take_refresh(&mut self) -> bool {
        std::mem::take(&mut self.refresh)
    }

    #[must_use]
    pub fn snapshot(&self) -> EventLogSnapshot {
        EventLogSnapshot { lines: self.lines.iter().map(|line| (line.text.clone(), line.state)).collect() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_play_through_their_states() {
        let mut log = EventLog::default();
        log.add(EventText::named("ID_ONLINE_EVENT_JOIN_SKATER", "Rob"));
        assert!(log.take_refresh());
        assert_eq!(log.snapshot().lines[0].1, LineState::Intro);
        for _ in 0..INTRO_UPDATES {
            log.update();
        }
        assert_eq!(log.snapshot().lines[0].1, LineState::Shown);
        assert!(!log.take_refresh());
        for _ in INTRO_UPDATES..OUTRO_UPDATE {
            log.update();
        }
        assert_eq!(log.snapshot().lines[0].1, LineState::Outro);
        assert!(log.take_refresh());
        for _ in OUTRO_UPDATE..DROP_UPDATE {
            log.update();
        }
        assert!(log.snapshot().lines.is_empty());
        assert!(log.take_refresh());
    }

    #[test]
    fn a_full_log_drops_its_oldest_line() {
        let mut log = EventLog::default();
        for name in ["A", "B", "C", "D", "E", "F"] {
            log.add(EventText::named("ID_ONLINE_EVENT_LEAVE", name));
        }
        let names: Vec<_> = log.snapshot().lines.iter().map(|(text, _)| text.arguments[0].clone()).collect();
        assert_eq!(names, ["B", "C", "D", "E", "F"]);
    }
}
