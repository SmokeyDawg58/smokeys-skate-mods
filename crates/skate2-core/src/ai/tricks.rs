//! The `scoring_trick` records the AI asks about: `TrickDatabase::CollectTrickData` 0x82df8988
//! (one `TrickInfo` per leaf collection, in key order, filed under its trick type) and the
//! fields `TrickScoringSettings::Set` 0x82de9a70 copies that the AI reads.
use crate::attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault};
use crate::physical_player_phys_out::conditioners::{
    scorable_id_base, scorable_id_category, scorable_id_consumed, scorable_id_string, scorable_id_trick_type,
};
use std::collections::BTreeMap;
use std::sync::Arc;

/// `scoring_trick`.
pub(crate) const TRICK_CLASS: u64 = 0x6918_4699_84a8_c596;
/// `eScorableID_NUM`.
pub(crate) const SCORABLE_COUNT: i32 = 269;
/// `eTrickTypeMax`.
const TRICK_TYPE_COUNT: usize = 14;

/// `Score::SkaterIntentTuple`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct IntentTuple {
    pub(crate) skater_intent: i32,
    pub(crate) value: f32,
    pub(crate) start_frame: i32,
    pub(crate) end_frame: i32,
}

/// A valid `TrickScoringSettings`, as far as the AI reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TrickSettings {
    pub(crate) scorable_id: i32,
    pub(crate) trick_type: i32,
    pub(crate) difficulty: i32,
    pub(crate) substitute_trick_group: u32,
    pub(crate) max_impending_trick_frames: i32,
    pub(crate) skater_intent_tuples: [IntentTuple; 6],
}

/// `Database::TrickInfo`.
#[derive(Clone, Copy, Debug)]
struct TrickInfo {
    scorable_id: i32,
    substitute_trick_group: u32,
}

/// The trick records by collection key, and `TrickDatabase::mTrickTypeSets`.
#[derive(Debug, Default)]
pub(crate) struct TrickTable {
    by_key: BTreeMap<u64, TrickSettings>,
    type_sets: [Vec<TrickInfo>; TRICK_TYPE_COUNT],
}

impl TrickTable {
    pub(crate) fn try_load(vault: &Arc<AttributeRuntimeVault>) -> Result<Self, AttributeRuntimeError> {
        let mut table = Self::default();
        for key in vault.leaf_collections_sorted(TRICK_CLASS)? {
            let instance = vault.instance(TRICK_CLASS, key);
            let layout = instance.try_cached_layout_field(8, 184)?;
            let zero = [0u8; 184];
            let bytes = layout.as_ref().map_or(zero.as_slice(), |data| data.bytes());
            let word = |offset: usize| u32::from_be_bytes(bytes[offset - 8..offset - 4].try_into().expect("word"));
            let count = usize::from(u16::from_be_bytes(bytes[2..4].try_into().expect("length")));
            let skater_intent_tuples = std::array::from_fn(|index| {
                if index < count {
                    let base = 16 + index * 16;
                    IntentTuple {
                        skater_intent: word(base) as i32,
                        value: f32::from_bits(word(base + 4)),
                        start_frame: word(base + 8) as i32,
                        end_frame: word(base + 12) as i32,
                    }
                } else {
                    IntentTuple::default()
                }
            });
            let settings = TrickSettings {
                scorable_id: word(136) as i32,
                trick_type: word(112) as i32,
                difficulty: word(116) as i32,
                substitute_trick_group: word(124),
                max_impending_trick_frames: word(144) as i32,
                skater_intent_tuples,
            };
            if let Some(set) = table.type_sets.get_mut(settings.trick_type as usize) {
                set.push(TrickInfo {
                    scorable_id: settings.scorable_id,
                    substitute_trick_group: settings.substitute_trick_group,
                });
            }
            table.by_key.insert(key, settings);
        }
        Ok(table)
    }

    /// `TrickScoringSettings(char const*)`: the record named like the scorable, if any.
    pub(crate) fn settings_for_name(&self, name: &str) -> Option<&TrickSettings> {
        self.by_key.get(&skate2_formats::attrib::string_hash64(&name.to_ascii_lowercase()))
    }

    /// `TrickScoringSettings(GetScorableIDString(id))`.
    pub(crate) fn settings(&self, scorable: i32) -> Option<&TrickSettings> {
        self.settings_for_name(scorable_id_string(scorable).unwrap_or("INVALID"))
    }

    /// `TrickScoringSettings(trickIndex)` (`LookupTrick`): the index's record in key order.
    pub(crate) fn settings_for_index(&self, index: i32) -> Option<&TrickSettings> {
        usize::try_from(index).ok().and_then(|index| self.by_key.values().nth(index))
    }

    /// `TrickScoringSettings(char const*)`'s `mTrickIndex`: the named record's position in key
    /// order.
    pub(crate) fn index_for_name(&self, name: &str) -> Option<i32> {
        let key = skate2_formats::attrib::string_hash64(&name.to_ascii_lowercase());
        self.by_key.keys().position(|&k| k == key).and_then(|index| i32::try_from(index).ok())
    }

    /// `TrickDatabase::GetTrickCount` 0x82ded728.
    pub(crate) fn trick_count(&self, trick_type: i32, group: u32) -> u32 {
        self.type_sets[trick_type as usize].iter().filter(|info| info.substitute_trick_group == group).count() as u32
    }

    /// `TrickDatabase::GetScorableID` 0x82ded7b0: the `entry`th trick of the group.
    pub(crate) fn scorable_id(&self, trick_type: i32, entry: u32, group: u32) -> i32 {
        self.type_sets[trick_type as usize]
            .iter()
            .filter(|info| info.substitute_trick_group == group)
            .nth(entry as usize)
            .map_or(-1, |info| info.scorable_id)
    }
}

/// `Score::IsValidScorableID` 0x82dea7e0.
pub(crate) fn is_valid_scorable(id: i32) -> bool {
    (0..SCORABLE_COUNT).contains(&id)
}

/// `Score::GetBaseScorableID` 0x82dee128: the base, or the scorable itself without one.
pub(crate) fn base_scorable(id: i32) -> i32 {
    scorable_id_base(id).filter(|base| is_valid_scorable(*base)).unwrap_or(id)
}

/// `Score::GetScorableIDConsumed` 0x82dee0a0.
pub(crate) fn consumed_scorable(id: i32) -> i32 {
    scorable_id_consumed(id).unwrap_or(-1)
}

/// `Score::GetScorableIDCategory` 0x82dedf40.
pub(crate) fn scorable_category(id: i32) -> i32 {
    scorable_id_category(id).unwrap_or(-1)
}

/// `Score::GetTrickType` 0x82dedfc8.
pub(crate) fn scorable_trick_type(id: i32) -> i32 {
    scorable_id_trick_type(id).unwrap_or(0)
}

/// The scorable's name, `"INVALID"` outside the catalogue.
pub(crate) fn scorable_name(id: i32) -> &'static str {
    scorable_id_string(id).unwrap_or("INVALID")
}

/// `eScorableIDCategory`.
pub(crate) mod category {
    pub(crate) const GRABS: i32 = 2;
    pub(crate) const FLIPS: i32 = 3;
    pub(crate) const AIR: i32 = 4;
}
