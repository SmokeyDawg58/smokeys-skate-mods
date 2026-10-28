//! `AIPath::AIPathID` comparisons, names and the path map's hash.
use skate2_formats::ai_path::AiPathId;
use std::fmt::Write as _;

/// `EPathIDProgressionBranch`: Ambient 0, Main 1, Contest 2, Race 3, Freeskate 4, Throwdown 5,
/// Test 6, Wildcard 7, AmbientOnlyWildcard 8.
pub(crate) const BRANCH_MAIN: u8 = 1;
const BRANCH_TEST: u8 = 6;
const BRANCH_WILDCARD: u8 = 7;
const BRANCH_AMBIENT_ONLY_WILDCARD: u8 = 8;
/// `ePathIDChallengeType_Wildcard`.
pub(crate) const CHALLENGE_TYPE_WILDCARD: u8 = 20;
/// `ePathIDChallengeType_Demo`.
pub(crate) const CHALLENGE_TYPE_DEMO: u8 = 5;

/// `GetFriendlyString`'s branch names (0x82fd7038).
const BRANCH_NAMES: [&str; 10] = ["am", "m", "c", "r", "f", "t", "te", "*", "am_*", "Max"];
/// `GetFriendlyString`'s `EPathIDChallengeType` names (0x82fd6fe0).
const CHALLENGE_TYPE_NAMES: [&str; 22] = [
    "1up", "aphoto", "bestgrind", "besttrick", "deathrace", "demo", "film", "follow", "filmfriend", "goto", "hom",
    "jam", "nis", "ots", "pc", "photo", "pro", "skate", "spotfilm", "tutorial", "*", "Max",
];
/// `Challenge::gAIPathChallengeTypeMap` 0x82fdab48: each `eChallengeTypes` value's
/// `EPathIDChallengeType`, -1 for none.
pub(crate) const CHALLENGE_TYPE_MAP: [i32; 51] = [
    19, 14, 5, -1, 17, -1, 15, 6, 3, 13, -1, -1, -1, -1, -1, 11, -1, -1, -1, 4, -1, -1, -1, 7, -1, -1, 11, 11, 16, -1,
    9, 10, 0, 10, -1, -1, -1, 1, 15, 6, 18, 8, -1, -1, -1, -1, 18, 15, 18, 15, -1,
];

/// `AIPathID::GetFriendlyString` 0x824b83f0: the branch's name, then for a challenge its stage,
/// part, index and type (`m_1a_05_photo`); with `challenge_id_only` false also the description
/// and suffix. An ambient ID spells its fifteen bytes in hex.
pub(crate) fn friendly_string(id: &AiPathId, challenge_id_only: bool) -> String {
    let data = &id.0;
    let branch = data[0];
    let name = BRANCH_NAMES.get(usize::from(branch)).copied().unwrap_or("Invalid");
    let mut out = String::new();
    match branch {
        0 => {
            let _ = write!(out, "{name}_");
            for byte in &data[1..16] {
                let _ = write!(out, "{byte:02X}");
            }
            return out;
        }
        8 => return name.to_owned(),
        1..=7 => {
            let _ = write!(out, "{name}_");
        }
        _ => {}
    }
    if branch == BRANCH_TEST {
        let _ = write!(out, "{}", data[1]);
    } else {
        if data[1] > 9 {
            out.push('?');
        } else {
            let _ = write!(out, "{}", data[1]);
        }
        let part = data[2] as i8;
        if (97..=122).contains(&part) {
            let _ = write!(out, "{}_", data[2] as char);
        } else {
            out.push_str("?_");
        }
        if data[4] != 0 && data[4] <= 99 {
            let _ = write!(out, "{:02}_", data[4]);
        } else {
            out.push_str("*_");
        }
        out.push_str(CHALLENGE_TYPE_NAMES.get(usize::from(data[3])).copied().unwrap_or("Invalid"));
    }
    if challenge_id_only {
        return out;
    }
    let description = &data[5..];
    let length = description.iter().position(|&byte| byte == 0).unwrap_or(description.len());
    if length != 0 {
        let _ = write!(out, "_{}", String::from_utf8_lossy(&description[..length]));
    }
    let suffix = u16::from_be_bytes([data[14], data[15]]);
    if suffix == 0xffff {
        out.push_str("_*");
    } else {
        let _ = write!(out, "_{suffix}");
    }
    out
}

/// `AIPathID::AIPathID(const char*)` 0x824c21c8: `LegacyChallengeNameConversion` 0x824b1558
/// turns `he_a1_demo<xy><rest>` into `m_0a_<xy>_demo<rest>`; then the branch token, and for a
/// test ID its index and suffix, else stage, part, index, type, description (a token only
/// when another follows it) and suffix, each token past its underscore. The native leaves the
/// bytes no part writes as they were; this ID starts zeroed. None for an ambient ID (the
/// native never returns) or a branch or type no token names.
pub(crate) fn parse(name: &str) -> Option<AiPathId> {
    const LEGACY: &str = "he_a1_demo";
    let converted;
    let mut text = name.as_bytes();
    if let Some(rest) = name.strip_prefix(LEGACY) {
        let rest = rest.as_bytes();
        let at = |index: usize| rest.get(index).copied().unwrap_or(0) as char;
        let tail = String::from_utf8_lossy(rest.get(2..).unwrap_or_default());
        converted = format!("m_0a_{}{}_demo{tail}", at(0), at(1));
        text = converted.as_bytes();
    }
    let mut id = AiPathId([0; 16]);
    let (branch, rest) = parse_token(text, &BRANCH_NAMES[..9])?;
    set_progression_branch(&mut id, u8::try_from(branch).ok()?);
    let mut ptr = past_underscore(rest);
    match id.0[0] {
        0 => return None,
        BRANCH_TEST => {
            let (index, rest) = parse_number(ptr);
            id.0[1] = index as u8;
            ptr = past_underscore(rest);
            let (suffix, _) = parse_number(ptr);
            id.0[14..16].copy_from_slice(&(suffix as u16).to_be_bytes());
            id.0[5] = 0;
        }
        _ => {
            let (stage, rest) = parse_number(ptr);
            id.0[1] = stage as u8;
            let (part, rest) = rest.split_first().map_or((0, rest), |(&part, rest)| (part, rest));
            id.0[2] = part;
            ptr = past_underscore(rest);
            let (index, rest) = parse_number(ptr);
            id.0[4] = index as u8;
            ptr = past_underscore(rest);
            let (challenge_type, rest) = parse_token(ptr, &CHALLENGE_TYPE_NAMES[..20])?;
            id.0[3] = u8::try_from(challenge_type).ok()?;
            ptr = past_underscore(rest);
            ptr = parse_description(&mut id, ptr);
            let (suffix, _) = parse_number(ptr);
            id.0[14..16].copy_from_slice(&(suffix as u16).to_be_bytes());
        }
    }
    Some(id)
}

/// `ParseToken` 0x824b1608: the longest token the text starts with, ignoring case.
fn parse_token<'a>(text: &'a [u8], tokens: &[&str]) -> Option<(usize, &'a [u8])> {
    let mut best: Option<(usize, usize)> = None;
    for (index, token) in tokens.iter().enumerate() {
        let token = token.as_bytes();
        let matches = text.len() >= token.len() && text[..token.len()].eq_ignore_ascii_case(token);
        if matches && best.is_none_or(|(_, length)| token.len() > length) {
            best = Some((index, token.len()));
        }
    }
    best.map(|(index, length)| (index, &text[length..]))
}

/// `ParseNumber` 0x824b16c8: the leading decimal digits.
fn parse_number(text: &[u8]) -> (u32, &[u8]) {
    let digits = text.iter().take_while(|byte| byte.is_ascii_digit()).count();
    let value = text[..digits].iter().fold(0_u32, |value, &digit| value.wrapping_mul(10).wrapping_add(u32::from(digit - b'0')));
    (value, &text[digits..])
}

/// The parsers' step past the underscore they expect next (`ptr + 1`).
fn past_underscore(text: &[u8]) -> &[u8] {
    text.get(1..).unwrap_or_default()
}

/// `ParseDescription` 0x824b8c10 with `ParseDescriptionPossible` 0x824b1838 and
/// `ParsePastNextUnderscore` 0x824b1780 (31 characters at most), then `SetDescription`
/// 0x824b5fe8 from byte 5 with its terminator.
fn parse_description<'a>(id: &mut AiPathId, text: &'a [u8]) -> &'a [u8] {
    let token = text.iter().take_while(|&&byte| byte != b'_').count();
    let rest = &text[token..];
    let rest = rest.strip_prefix(b"_").unwrap_or(rest);
    if rest.first().is_none_or(|&byte| byte == 0) {
        id.0[5] = 0;
        return text;
    }
    let description = &text[..token.min(31)];
    for (index, &byte) in description.iter().chain(std::iter::once(&0)).enumerate() {
        if let Some(slot) = id.0.get_mut(5 + index) {
            *slot = byte;
        }
    }
    rest
}

/// `AIPathID::SetProgressionBranch` 0x824b8390: leaving an ambient ID for a branch clears the
/// rest of it first.
pub(crate) fn set_progression_branch(id: &mut AiPathId, branch: u8) {
    if branch != 0 && id.0[0] == 0 {
        id.0[1..].fill(0);
    }
    id.0[0] = branch;
}

/// `AIPathID_hash`, inlined in the map's `find` 0x824c78c8 and `DoInsertValue` 0x824d4150:
/// FNV-style over the four big-endian words, the last one only XORed in.
pub(crate) fn hash(id: &AiPathId) -> u32 {
    const PRIME: u32 = 0x0100_0193;
    let h = (id.word(0) ^ 0x050c_5d1f).wrapping_mul(PRIME);
    let h = (h ^ id.word(4)).wrapping_mul(PRIME);
    let h = (h ^ id.word(8)).wrapping_mul(PRIME);
    h ^ id.word(12)
}

/// `AIPathID(branch, stage, part, type, index, description, suffix)` 0x824c2090 with
/// `SetDescription` 0x824b5fe8 copying the description and its terminator from byte 5.
pub(crate) fn challenge_id(
    branch: u8,
    stage: u8,
    part: u8,
    challenge_type: u8,
    index: u8,
    description: &[u8],
    suffix: u16,
) -> AiPathId {
    let mut data = [0; 16];
    data[0] = branch;
    data[1] = stage;
    data[2] = part;
    data[3] = challenge_type;
    data[4] = index;
    data[5..5 + description.len()].copy_from_slice(description);
    data[14..16].copy_from_slice(&suffix.to_be_bytes());
    AiPathId(data)
}

/// `PathManager::mUniversalFilterPathID`: every branch, stage, part, type, index,
/// description and suffix.
pub(crate) fn universal_filter_id() -> AiPathId {
    challenge_id(BRANCH_WILDCARD, 127, 63, CHALLENGE_TYPE_WILDCARD, 0, b"*", 0xffff)
}

/// `PathManager::mAmbientFilterPathID`: only byte 0 is set (AmbientOnlyWildcard).
pub(crate) fn ambient_filter_id() -> AiPathId {
    let mut data = [0; 16];
    data[0] = BRANCH_AMBIENT_ONLY_WILDCARD;
    AiPathId(data)
}

/// `CompareDescription` 0x824b87f0: a filter description starting with `*` matches all,
/// otherwise `strcmp` from byte 5 including the filter's terminator.
fn compare_description(id: &AiPathId, filter: &AiPathId) -> bool {
    if filter.0[5] == b'*' {
        return true;
    }
    for (index, &expected) in filter.0[5..].iter().enumerate() {
        if expected != id.0[5 + index] {
            return false;
        }
        if expected == 0 {
            return true;
        }
    }
    true
}

/// `AIPathID::CompareAgainstFilter` 0x824c22d8.
pub(crate) fn compare_against_filter(id: &AiPathId, filter: &AiPathId) -> bool {
    let branch = id.0[0];
    let filter_branch = filter.0[0];
    if branch == 0 {
        if filter_branch == 0 {
            return filter.word(4) == id.word(4) && filter.word(12) == id.word(12);
        }
        return filter_branch > BRANCH_TEST && filter_branch <= BRANCH_AMBIENT_ONLY_WILDCARD;
    }
    if filter_branch != BRANCH_WILDCARD && filter_branch != branch {
        return false;
    }
    if branch == BRANCH_TEST {
        if filter_branch == BRANCH_WILDCARD {
            return true;
        }
        // GetTestIndex 0x824b5f48.
        let test = filter.0[1];
        if test != 0 && test != id.0[1] {
            return false;
        }
    } else {
        // GetProgressionStage / GetProgressionPart / GetChallengeType / GetChallengeIndex.
        let stage = filter.0[1];
        if stage != 127 && stage != id.0[1] {
            return false;
        }
        let part = filter.0[2] as i8;
        if part != 63 && part != id.0[2] as i8 {
            return false;
        }
        let challenge_type = filter.0[3];
        if challenge_type != CHALLENGE_TYPE_WILDCARD && challenge_type != id.0[3] {
            return false;
        }
        let index = filter.0[4];
        if index != 0 && index != id.0[4] {
            return false;
        }
    }
    if !compare_description(id, filter) {
        return false;
    }
    let suffix = u16::from_be_bytes([filter.0[14], filter.0[15]]);
    suffix == 0xffff || suffix == u16::from_be_bytes([id.0[14], id.0[15]])
}

/// `AIPath::IFilter`: `SingleFilter` (one ID) or `MultipleFilter` (a list); a path is included
/// when its ID compares equal against any filter ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PathFilter {
    Single(AiPathId),
    Multiple(Vec<AiPathId>),
}

impl PathFilter {
    /// `SingleFilter::IsPathIDIncluded` 0x824c24f8 / `MultipleFilter::IsPathIDIncluded` 0x824c2588.
    pub(crate) fn includes(&self, id: &AiPathId) -> bool {
        match self {
            Self::Single(filter) => compare_against_filter(id, filter),
            Self::Multiple(filters) => filters.iter().any(|filter| compare_against_filter(id, filter)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ambient(word4: u32, word12: u32) -> AiPathId {
        let mut data = [0; 16];
        data[4..8].copy_from_slice(&word4.to_be_bytes());
        data[12..16].copy_from_slice(&word12.to_be_bytes());
        AiPathId(data)
    }

    #[test]
    fn the_ambient_filter_takes_every_ambient_path_and_no_challenge_path() {
        let filter = ambient_filter_id();
        assert!(compare_against_filter(&ambient(0x809e_6400, 0xe1a6_6300), &filter));
        let challenge = challenge_id(1, 1, b'a' as u8, 4, 2, b"race", 0);
        assert!(!compare_against_filter(&challenge, &filter));
        assert!(compare_against_filter(&challenge, &universal_filter_id()));
    }

    #[test]
    fn ambient_ids_compare_words_four_and_twelve() {
        let id = ambient(1, 2);
        assert!(compare_against_filter(&id, &ambient(1, 2)));
        assert!(!compare_against_filter(&id, &ambient(1, 3)));
    }

    #[test]
    fn challenge_filters_honour_wildcards_descriptions_and_suffixes() {
        let id = challenge_id(1, 2, 3, 4, 5, b"jam", 7);
        assert!(compare_against_filter(&id, &challenge_id(1, 127, 63, 20, 0, b"*", 0xffff)));
        assert!(compare_against_filter(&id, &challenge_id(1, 2, 3, 4, 5, b"jam", 7)));
        assert!(!compare_against_filter(&id, &challenge_id(1, 2, 3, 4, 5, b"ja", 7)));
        assert!(!compare_against_filter(&id, &challenge_id(1, 2, 3, 4, 5, b"jam", 8)));
        assert!(!compare_against_filter(&id, &challenge_id(2, 127, 63, 20, 0, b"*", 0xffff)));
    }

    #[test]
    fn challenge_ids_name_their_package() {
        let id = challenge_id(BRANCH_MAIN, 1, b'a', 15, 5, b"", 0);
        assert_eq!(friendly_string(&id, true), "m_1a_05_photo");
        assert_eq!(friendly_string(&id, false), "m_1a_05_photo_0");
        let wild = challenge_id(3, 2, b'a', 4, 0, b"*", 0xffff);
        assert_eq!(friendly_string(&wild, false), "r_2a_*_deathrace_*_*");
        assert_eq!(friendly_string(&ambient_filter_id(), true), "am_*");
        assert_eq!(CHALLENGE_TYPE_MAP[38], 15);
    }

    #[test]
    fn round_path_names_parse_as_the_native_reads_them() {
        let skate = parse("t_5b_01_skate_0").unwrap();
        assert_eq!(skate, challenge_id(5, 5, b'b', 17, 1, b"", 0));
        let lettered = parse("c_1a_01_besttrick_!B_12").unwrap();
        assert_eq!(lettered, challenge_id(2, 1, b'a', 3, 1, b"!B", 12));
        assert_eq!(friendly_string(&lettered, false), "c_1a_01_besttrick_!B_12");
        let test = parse("te_3_7").unwrap();
        assert_eq!(&test.0[..2], &[6, 3]);
        assert_eq!(u16::from_be_bytes([test.0[14], test.0[15]]), 7);
        assert_eq!(parse("he_a1_demo05_0"), parse("m_0a_05_demo_0"));
        assert_eq!(parse("am_0123"), None);
    }

    #[test]
    fn a_branch_clears_an_ambient_id_only() {
        let mut id = ambient(0x0102_0304, 0x0506_0708);
        set_progression_branch(&mut id, BRANCH_MAIN);
        assert_eq!(id.0, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        id.0[7] = 9;
        set_progression_branch(&mut id, 3);
        assert_eq!(id.0[7], 9);
    }

    #[test]
    fn the_hash_folds_the_last_word_without_a_final_multiply() {
        let id = AiPathId([0; 16]);
        let expected = ((0x050c_5d1fu32.wrapping_mul(0x0100_0193)).wrapping_mul(0x0100_0193))
            .wrapping_mul(0x0100_0193);
        assert_eq!(hash(&id), expected);
    }
}
