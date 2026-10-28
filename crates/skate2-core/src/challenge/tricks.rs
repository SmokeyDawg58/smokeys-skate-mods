//! `Sk8::Challenge::cChallengeTrickGroupManager`: whether a trick belongs to
//! a trick group or to one of the groups descending from it.

use super::attrib_db::AttribDb;

/// The trick group class.
pub const TRICK_GROUP_CLASS: u64 = 0x5b47_469d_2d89_5dde;
/// A group's tricks: `RefSpec[]` whose collection keys are trick attrib keys.
pub const TRICKS: u64 = 0x033c_23a6_5a34_839d;

#[derive(Debug)]
pub struct TrickGroupManager {
    /// `LastGroupKeyExamined`; -1 after a group that does not exist.
    last_group: u64,
    /// `LastExaminedGroupTrickKeys`: that group, then its descendant groups.
    last_keys: Vec<u64>,
}

impl Default for TrickGroupManager {
    /// `Init` (`0x82db1260`).
    fn default() -> Self {
        Self { last_group: u64::MAX, last_keys: Vec::new() }
    }
}

/// `AttribSysUtils::IsDescendantOf` (`0x82ab8be8`): an ancestor of `key`, not
/// `key` itself; every collection descends from 0.
fn is_descendant_of(db: &AttribDb, key: u64, ancestor: u64) -> bool {
    let mut parent = db.parent(TRICK_GROUP_CLASS, key).unwrap_or(0);
    loop {
        if parent == ancestor {
            return true;
        }
        if parent == 0 {
            return ancestor == 0;
        }
        parent = db.parent(TRICK_GROUP_CLASS, parent).unwrap_or(0);
    }
}

impl TrickGroupManager {
    /// `IsTrickInGroup` (`0x82daf620`).
    pub fn is_trick_in_group(&mut self, db: &AttribDb, trick: u64, group: u64) -> bool {
        if self.last_group != group {
            self.last_keys.clear();
            if db.has_collection(TRICK_GROUP_CLASS, group) {
                self.last_keys.push(group);
                for &key in db.collections(TRICK_GROUP_CLASS) {
                    if is_descendant_of(db, key, group) {
                        self.last_keys.push(key);
                    }
                }
                self.last_group = group;
            } else {
                self.last_group = u64::MAX;
            }
        }
        self.last_keys.iter().any(|&key| {
            db.attribute(TRICK_GROUP_CLASS, key, TRICKS)
                .is_some_and(|a| (0..a.len()).any(|i| a.ref_spec(i).map_or(0, |(_, k)| k) == trick))
        })
    }
}
