//! PC port addition: the game's update over the main menu. The app looks for a newer release at
//! startup (`skate2-update`); once the main menu is idle a newer one is offered, Yes downloads it
//! under a progress popup and the game then closes and starts again on it, No asks again only at
//! the next start or after the server refuses this version (`OnlineNotice::UpdateRequired`).
//! The first start after an update lists the release's changes ([`Changelog`]) before anything
//! else is offered.

/// What the app's updater reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    /// No newer release is known.
    None,
    Available { version: String },
    Checking,
    UpToDate,
    GameUpdatePending,
    Unavailable,
    OpeningUpdater,
    Downloading { version: String, percent: u8 },
    /// The release is in place; the game closes and starts again.
    Installed { version: String },
    /// The update failed and the game is as it was: why, for the player.
    Failed { message: String },
}

/// What the main menu asks of the app's updater.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateRequest {
    /// Look for a newer release again.
    Check,
    /// Download and install the release offered.
    Install,
}

/// The update's popup over the main menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdatePrompt {
    /// A newer release: Yes (0) or No (1).
    Offer { version: String, current: String, selected: usize },
    Downloading { version: String, percent: u8 },
    OpeningUpdater,
    ModInfo { selected: usize },
    CheckResult { message: String },
    GameUpdateNotice { selected: usize },
    Installed { version: String },
    Failed { message: String },
    /// The release's changes at the first start after an update: OK.
    Changelog(Changelog),
}

impl UpdatePrompt {
    /// The popup as sk8popup shows it: title, text, options, the highlighted option and whether
    /// B closes it.
    #[must_use]
    pub fn dialog(&self) -> (String, String, Vec<String>, usize, bool) {
        match self {
            Self::Offer { version, selected, .. } if version == "Smokey mod" => (
                "#Smokey's Mods Update".into(),
                "#A new mod update is ready. Close the game and open the updater now?".into(),
                vec!["#Install Update".into(), "#Later".into()], *selected, true,
            ),
            Self::Offer { version, current, selected } => (
                "#Update Available".into(),
                format!("#Version {version} is out (you have {current}). Online play needs the newest version. Update now?"),
                vec!["ID_COMMON_YES".into(), "ID_COMMON_NO".into()],
                *selected,
                true,
            ),
            Self::Downloading { version, percent } => ("#Updating".into(), format!("#Downloading version {version}: {percent}%"), Vec::new(), 0, false),
            Self::OpeningUpdater => ("#Opening Updater".into(), "#Closing the game to open Smokey's updater.".into(), Vec::new(), 0, false),
            Self::GameUpdateNotice { selected } => ("#Game Update".into(), "#The game has updated. Smokey's mod needs a matching update for online play. Install the update now?".into(), vec!["#Install Update".into(), "#Later".into()], *selected, true),
            Self::ModInfo { selected } => ("#Smokey's Skate Mods".into(), "#LATEST UPDATES\nDebug camera with 20x speed\nPhysics, difficulty and Motorized\nWeather and water physics\n\nTIPS\nDebug Camera: Right Stick + D-pad Right.\nObject Dropper: LB + D-pad Left.\nChange the camera bind in Mods > Keybinds.\nUse Smokey's updater for new releases.".into(), vec!["#Check for Update".into(), "#Close".into()], *selected, true),
            Self::CheckResult {message} => ("#Update Check".into(), format!("#{message}"), vec!["ID_COMMON_OK".into()], 0, false),
            Self::Installed { version } => ("#Updating".into(), format!("#Version {version} is installed. The game starts again now."), Vec::new(), 0, false),
            Self::Failed { message } => (
                "ID_COMMON_WARNING_TITLE".into(),
                format!("#The update could not be installed, and the game is unchanged. {message}"),
                vec!["ID_COMMON_OK".into()],
                0,
                false,
            ),
            Self::Changelog(changelog) => {
                (format!("#Version {}", changelog.version), changelog.text(), vec!["ID_COMMON_OK".into()], 0, false)
            }
        }
    }
}

/// The most body lines the popup takes. sk8popup centres its panel on its core (configure_popup):
/// with the core 128 tall at panel y 35, the bottom bar's 32 under it, one 31.4 option row and
/// 22 px lines, 23 lines keep the panel's bottom at y 706 of 720.
const CHANGELOG_LINES: usize = 23;

/// Characters to a body line when counting an entry's wrapped lines: a 590 px line of 22 px
/// Helvetica holds about 53 of a sentence's, so this errs towards more lines.
const LINE_CHARACTERS: usize = 45;

/// The release's changes, shown once over the main menu at a player's first launch of the release
/// (the app's `config.json` records it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changelog {
    pub version: String,
    pub entries: Vec<String>,
}

impl Changelog {
    /// A changelog file's entries, a change a line. Blank lines and lines starting with `#` are
    /// left out, a dash or star with a space after it that starts a line is dropped (the popup
    /// adds its own dash), and typographic quotes, dashes and ellipses become plain ones.
    #[must_use]
    pub fn parse(version: &str, text: &str) -> Self {
        let entries = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with('#'))
            .map(|line| match line.strip_prefix(['-', '*']) {
                Some(rest) if rest.is_empty() || rest.starts_with(char::is_whitespace) => rest.trim_start(),
                _ => line,
            })
            .filter(|line| !line.is_empty())
            .map(plain)
            .collect();
        Self { version: version.to_owned(), entries }
    }

    /// "Update Changelogs:" and each entry under a dash, a blank line before each. Entries that
    /// would run the popup past the screen's bottom are counted on a last line instead.
    fn text(&self) -> String {
        // An entry's blank line and its own lines as the popup wraps them, about.
        let lines: Vec<usize> = self.entries.iter().map(|entry| 1 + (entry.chars().count() + 2).div_ceil(LINE_CHARACTERS)).collect();
        let total = |count: usize| 1 + lines[..count].iter().sum::<usize>();
        let shown = if total(self.entries.len()) <= CHANGELOG_LINES {
            self.entries.len()
        } else {
            (0..self.entries.len()).rev().find(|&count| total(count) + 2 <= CHANGELOG_LINES).unwrap_or(0)
        };
        let mut text = String::from("#Update Changelogs:");
        for entry in &self.entries[..shown] {
            text.push_str("\n\n- ");
            text.push_str(entry);
        }
        if shown < self.entries.len() {
            text.push_str(&format!("\n\n- and {} more", self.entries.len() - shown));
        }
        text
    }
}

/// `text` with typographic quotes, dashes and ellipses in their plain forms, which the popup's
/// font has.
fn plain(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\u{2018}' | '\u{2019}' => plain.push('\''),
            '\u{201C}' | '\u{201D}' => plain.push('"'),
            '\u{2013}' | '\u{2014}' => plain.push('-'),
            '\u{2026}' => plain.push_str("..."),
            character => plain.push(character),
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changelog_file_lists_each_change_under_a_dash_a_blank_line_apart() {
        let file = "# Shown after an update\r\n\r\nLeave Challenge in the pause menu\r\n- Lobby ghosts\r\n* \u{201C}Alt-tab\u{201D} kicks \u{2014} fixed\u{2026}\r\n-flag stays\r\n - \r\n";
        let changelog = Changelog::parse("0.2.5", file);
        assert_eq!(changelog.entries, ["Leave Challenge in the pause menu", "Lobby ghosts", "\"Alt-tab\" kicks - fixed...", "-flag stays"]);
        let (title, text, options, selected, cancellable) = UpdatePrompt::Changelog(changelog).dialog();
        assert_eq!(title, "#Version 0.2.5");
        assert_eq!(
            text,
            "#Update Changelogs:\n\n- Leave Challenge in the pause menu\n\n- Lobby ghosts\n\n- \"Alt-tab\" kicks - fixed...\n\n- -flag stays"
        );
        assert_eq!((options, selected, cancellable), (vec!["ID_COMMON_OK".to_owned()], 0, false));
        assert!(Changelog::parse("0.3.0", "# nothing yet\n\n").entries.is_empty());
    }

    #[test]
    fn a_long_changelog_ends_with_how_many_more() {
        let file: String = (1..=15).map(|n| format!("Change {n}\n")).collect();
        let (_, text, ..) = UpdatePrompt::Changelog(Changelog::parse("0.3.0", &file)).dialog();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), CHANGELOG_LINES);
        assert_eq!(lines[CHANGELOG_LINES - 3], "- Change 10");
        assert_eq!(lines[CHANGELOG_LINES - 1], "- and 5 more");
        let long = format!("{}\n", "word ".repeat(20).trim_end());
        let (_, text, ..) = UpdatePrompt::Changelog(Changelog::parse("0.3.0", &long.repeat(6))).dialog();
        assert_eq!(text.matches("\n\n- word").count(), 5, "an entry the popup wraps counts its lines");
        assert!(text.ends_with("\n\n- and 1 more"));
    }
}
