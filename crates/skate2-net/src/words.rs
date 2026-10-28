//! The server's word list (PC port addition): `badwords.txt`, which the server keeps private and
//! reads beside itself. No nickname may hold one of its words, and no chat line holding one is
//! relayed. A text and the list's words are compared as [`normalize`] reads them, so spacing,
//! case, repeated letters and the digits and symbols that stand in for letters hide no word.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The list's file name.
pub const FILE_NAME: &str = "badwords.txt";

/// What a text reads as once its disguises are gone: its letters in lowercase, the digits and
/// symbols that stand in for letters read as those letters (`1`, `!`, `|` and `l` as `i`, `4` and
/// `@` as `a`, `3` as `e`, `0` as `o`, `5` and `$` as `s`, `7` as `t`, `8` as `b`, `6` and `9` as
/// `g`), everything else (spaces, punctuation) left out, and each run of one letter cut to one:
/// `N1GG4` and `n1ggggg4` both read `niga`.
#[must_use]
pub fn normalize(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    for character in text.chars() {
        let letter = match character.to_ascii_lowercase() {
            '1' | '!' | '|' | 'l' => 'i',
            '4' | '@' => 'a',
            '3' => 'e',
            '0' => 'o',
            '5' | '$' => 's',
            '7' => 't',
            '8' => 'b',
            '6' | '9' => 'g',
            letter if letter.is_ascii_lowercase() => letter,
            _ => continue,
        };
        if !normalized.ends_with(letter) {
            normalized.push(letter);
        }
    }
    normalized
}

/// The words of a list's lines: one word or phrase a line, blank lines and `#` comments skipped,
/// each normalized. A line that normalizes to fewer than two letters is left out, as one letter
/// would be found in nearly every text.
fn words<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut words: Vec<String> = lines
        .into_iter()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(normalize)
        .filter(|word| word.len() >= 2)
        .collect();
    words.sort_unstable();
    words.dedup();
    words
}

/// The server's word list. Its file is read again whenever it changes, so an edit applies to the
/// next nickname or chat line without a restart; with no file, nothing is filtered.
#[derive(Debug, Default)]
pub struct WordList {
    path: Option<PathBuf>,
    /// The file's modification time and length at the last read.
    stamp: Option<(SystemTime, u64)>,
    words: Vec<String>,
}

impl WordList {
    /// The list in the file at `path`.
    #[must_use]
    pub fn from_file(path: PathBuf) -> Self {
        let mut list = Self { path: Some(path), stamp: None, words: Vec::new() };
        list.refresh();
        list
    }

    /// A list of these lines' words, with no file.
    #[must_use]
    pub fn from_words<'a>(lines: impl IntoIterator<Item = &'a str>) -> Self {
        Self { path: None, stamp: None, words: words(lines) }
    }

    /// The list a server reads: [`FILE_NAME`] beside its executable, or in the working directory
    /// when only that one exists.
    #[must_use]
    pub fn beside_server() -> Self {
        Self::from_file(crate::server::server_file(FILE_NAME))
    }

    /// The file the list is read from.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The words, as last read.
    #[must_use]
    pub fn len(&self) -> usize {
        self.words.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Reads the file again when it changed or went since the last read.
    fn refresh(&mut self) {
        let Some(path) = &self.path else { return };
        let stamp = std::fs::metadata(path).ok().map(|metadata| (metadata.modified().unwrap_or(UNIX_EPOCH), metadata.len()));
        if stamp == self.stamp {
            return;
        }
        self.stamp = stamp;
        self.words = std::fs::read_to_string(path).map(|text| words(text.lines())).unwrap_or_default();
    }

    /// Whether `text` holds one of the list's words, the file read again first if it changed.
    pub fn holds_word(&mut self, text: &str) -> bool {
        self.refresh();
        if self.words.is_empty() {
            return false;
        }
        let text = normalize(text);
        self.words.iter().any(|word| text.contains(word.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disguises_read_as_the_letters_they_stand_for() {
        assert_eq!(normalize("N1GG4"), "niga");
        assert_eq!(normalize("n1ggggg4"), "niga");
        assert_eq!(normalize("n i g g a"), "niga");
        assert_eq!(normalize("F-a_G g 0 t!"), "fagoti");
        assert_eq!(normalize("Hello, World"), "heioworid");
        assert_eq!(normalize("$h3ll"), "shei");
        assert_eq!(normalize("caf\u{e9} 9"), "cafg");
        assert_eq!(normalize("--- '' __"), "");
    }

    #[test]
    fn the_lists_words_are_read_as_text_is() {
        let mut list = WordList::from_words(["# a comment", "", "  faggot  ", "nigga", "---", "!!!", "l"]);
        assert_eq!(list.len(), 2, "comments, blank lines and lines of fewer than two letters are left out");
        for text in ["fagot", "F4GG0T", "ur a f a g g o t", "N1GG4", "n1ggggg4", "xXniggaXx"] {
            assert!(list.holds_word(text), "{text:?}");
        }
        for text in ["Rob", "nice line", "gg, one more?", "flag pole"] {
            assert!(!list.holds_word(text), "{text:?}");
        }
    }

    #[test]
    fn without_a_file_nothing_is_filtered() {
        let mut list = WordList::from_file(std::env::temp_dir().join("skate2-net-no-such-word-list.txt"));
        assert!(list.is_empty());
        assert!(!list.holds_word("anything"));
    }

    #[test]
    fn an_edited_file_is_read_again() {
        let path = std::env::temp_dir().join(format!("skate2-net-word-list-{}.txt", std::process::id()));
        std::fs::write(&path, "first\n").unwrap();
        let mut list = WordList::from_file(path.clone());
        assert!(list.holds_word("the first one"));
        std::fs::write(&path, "first\nsecond word\n").unwrap();
        assert!(list.holds_word("a SECOND-word"));
        std::fs::remove_file(&path).unwrap();
        assert!(!list.holds_word("first"), "a removed file filters nothing");
    }
}
