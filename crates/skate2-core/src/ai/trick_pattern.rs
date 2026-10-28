//! The S.K.A.T.E. AI's choice of a path for a trick pattern: `Score::TrickPatternMatcher` with
//! trick substitution, `AIPath::Path::MatchesPattern` and `MatchesSpinCount`, and
//! `PathManager::GetAIPathIDMatchingTrickPattern`.

use super::{
    path::{self, EVENT_START_TRICK, EXTENDED_VALID_TRAJECTORY, NODE_AIRBORNE, trick_type},
    path_id,
    path_manager::PathManager,
    tricks,
};
use crate::challenge::skate::TrickPattern;
use skate2_formats::ai_path::{AiPath, AiPathId};

/// `eTrickPatterMatchTypeExactTrick` and `eTrickPatterMatchTypeTrickType`.
const MATCH_EXACT_TRICK: i32 = 1;
const MATCH_TRICK_TYPE: i32 = 2;
/// The candidates `GetAIPathIDMatchingTrickPattern` collects at most.
const MAX_CANDIDATES: usize = 40;
/// The frames `MatchesSpinCount` looks past the first trajectory node for its spin.
const SPIN_LOOKAHEAD_FRAMES: u32 = 180;

/// What a pattern match reads of the `scoring_trick` records (`TrickScoringSettings`).
pub(crate) trait TrickRecords {
    /// `TrickScoringSettings(name)`'s `mTrickIndex`: the named record's index, `None` when the
    /// record is not valid.
    fn trick_index(&self, name: &str) -> Option<i32>;
    /// `TrickScoringSettings::LookupTrick(index)`'s `mTrickType` and `mSubstituteTrickGroup`,
    /// `None` when the record is not valid.
    fn trick_type_and_group(&self, index: i32) -> Option<(i32, u32)>;
}

impl TrickRecords for tricks::TrickTable {
    fn trick_index(&self, name: &str) -> Option<i32> {
        self.index_for_name(name)
    }

    fn trick_type_and_group(&self, index: i32) -> Option<(i32, u32)> {
        self.settings_for_index(index).map(|settings| (settings.trick_type, settings.substitute_trick_group))
    }
}

/// `Score::CanSubstituteTricks` 0x82df6008: the same trick type, or both aerial, in the same
/// substitute trick group. A record that is not valid asserts natively and reads as the
/// constructed settings (no type, group 0).
fn can_substitute_tricks(a: i32, b: i32, records: &impl TrickRecords) -> bool {
    let (type_a, group_a) = records.trick_type_and_group(a).unwrap_or((trick_type::NONE, 0));
    let (type_b, group_b) = records.trick_type_and_group(b).unwrap_or((trick_type::NONE, 0));
    let aerial = |kind: i32| kind == trick_type::AIR || kind == trick_type::FLIP;
    (type_a == type_b || (aerial(type_a) && aerial(type_b))) && group_a == group_b
}

/// `TrickPatternItem::MatchesTrick` 0x82df7010: an exact trick item takes its trick (or, with
/// substitution, one it can substitute), a trick type item its type.
fn item_matches_trick(item: (i32, i32), kind: i32, index: i32, allow_substitution: bool, records: &impl TrickRecords) -> bool {
    match item.0 {
        MATCH_EXACT_TRICK => index == item.1 || (allow_substitution && can_substitute_tricks(index, item.1, records)),
        MATCH_TRICK_TYPE => kind == item.1,
        _ => false,
    }
}

/// `TrickPattern::DoesContainTrickType` 0x82df6150.
fn does_contain_trick_type(pattern: &TrickPattern, kind: i32, records: &impl TrickRecords) -> bool {
    let length = usize::try_from(pattern.length).unwrap_or(0).min(4);
    pattern.items[..length].iter().any(|&(match_type, value)| {
        let item_kind = match match_type {
            MATCH_TRICK_TYPE => value,
            MATCH_EXACT_TRICK => records.trick_type_and_group(value).map_or(trick_type::NONE, |(kind, _)| kind),
            _ => trick_type::NONE,
        };
        item_kind == kind
    })
}

/// `Score::TrickPatternMatcher` (constructor 0x82ded828): the pattern, active, no item matched.
struct TrickPatternMatcher {
    pattern: TrickPattern,
    matches: [bool; 3],
}

impl TrickPatternMatcher {
    fn new(pattern: TrickPattern) -> Self {
        Self { pattern, matches: [false; 3] }
    }

    /// `AddTrick` 0x82df70c0: a grab only counts toward a pattern holding one; from the last item
    /// down, an item whose predecessor matched takes the trick, the last one completing the
    /// pattern and the others remembering whether they matched.
    fn add_trick(&mut self, kind: i32, index: i32, allow_substitution: bool, records: &impl TrickRecords) -> bool {
        if kind == trick_type::GRAB && !does_contain_trick_type(&self.pattern, trick_type::GRAB, records) {
            return false;
        }
        let last = self.pattern.length - 1;
        let mut at = last;
        while at >= 0 {
            let previous_matched = at == 0 || usize::try_from(at - 1).ok().and_then(|i| self.matches.get(i)).copied().unwrap_or(false);
            if previous_matched {
                let Some(&item) = usize::try_from(at).ok().and_then(|i| self.pattern.items.get(i)) else { return false };
                let matched = item_matches_trick(item, kind, index, allow_substitution, records);
                if matched && at == last {
                    return true;
                }
                if (matched || at < last)
                    && let Some(slot) = usize::try_from(at).ok().and_then(|i| self.matches.get_mut(i))
                {
                    *slot = matched;
                }
            }
            at -= 1;
        }
        false
    }
}

/// `Path::MatchesPattern` 0x824b8258: the path's trick-start nodes, in order, complete the
/// pattern (each node's trick by its scorable's record).
pub(crate) fn matches_pattern(path: &AiPath, pattern: &TrickPattern, allow_substitution: bool, records: &impl TrickRecords) -> bool {
    let mut matcher = TrickPatternMatcher::new(*pattern);
    for node in &path.nodes {
        if node.event_type != EVENT_START_TRICK {
            continue;
        }
        let index = records.trick_index(tricks::scorable_name(path::trick_scorable(node))).unwrap_or(-1);
        if matcher.add_trick(path::node_trick_type(node), index, allow_substitution, records) {
            return true;
        }
    }
    false
}

/// `Path::MatchesSpinCount` 0x824c1e90: from the first trajectory node at or after the first
/// trick start, its spin, or failing one the first spin of a later trajectory node within 180
/// frames before an airborne stretch lands; that many half turns, unsigned.
pub(crate) fn matches_spin_count(path: &AiPath, spin_count: i32) -> bool {
    let wanted = spin_count.wrapping_abs();
    let nodes = &path.nodes;
    let mut seen_start = false;
    let Some(first) = nodes.iter().position(|node| {
        seen_start |= node.event_type == EVENT_START_TRICK;
        seen_start && node.extended.is_some_and(|ext| ext.bit_flags & EXTENDED_VALID_TRAJECTORY != 0)
    }) else {
        return false;
    };
    let mut airborne = nodes[first].bit_flags & NODE_AIRBORNE != 0;
    let mut spin = path::air_spin_180_count(&nodes[first]).wrapping_abs();
    if spin == 0 {
        let mut frames = 0u32;
        for node in &nodes[first + 1..] {
            let node_airborne = node.bit_flags & NODE_AIRBORNE != 0;
            frames += u32::from(node.frame_count_since_last_node);
            if airborne && !node_airborne {
                break;
            }
            if frames > SPIN_LOOKAHEAD_FRAMES {
                break;
            }
            if node.extended.is_some_and(|ext| ext.bit_flags & EXTENDED_VALID_TRAJECTORY != 0) {
                spin = path::air_spin_180_count(node).wrapping_abs();
                if spin > 0 {
                    break;
                }
            }
            airborne |= node_airborne;
        }
    }
    spin == wanted
}

/// `PathManager::GetAIPathIDMatchingTrickPattern` 0x824c7f38: in the path map's order, the first
/// path of each alias inside the filter (`AIPathID::CompareAgainstFilter`) that matches the
/// pattern and the spin count, at most 40; an `rw::math::Random` pick of them.
///
/// # Errors
///
/// Returns the random number generator's lock error.
pub(crate) fn path_id_matching_trick_pattern(
    paths: &PathManager,
    pattern: &TrickPattern,
    spin_count: i32,
    filter: Option<&AiPathId>,
    allow_substitution: bool,
    records: &impl TrickRecords,
) -> Result<Option<AiPathId>, crate::native_skater_random::rw_math::RwRandomLockError> {
    let mut candidates = Vec::new();
    for path in paths.first_paths() {
        let path = path.path();
        if filter.is_none_or(|filter| path_id::compare_against_filter(&path.id, filter))
            && matches_pattern(path, pattern, allow_substitution, records)
            && matches_spin_count(path, spin_count)
        {
            candidates.push(path.id);
            if candidates.len() == MAX_CANDIDATES {
                break;
            }
        }
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    let draw = crate::native_skater_random::rw_math::global_next()?;
    let count = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
    Ok(usize::try_from(draw % count).ok().and_then(|index| candidates.get(index)).copied())
}
