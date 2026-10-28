//! `AI::SkaterProfile` 0x824c8690: an `ai_skater_profiles` record with its ollie and nollie
//! `WeightGroup`s.
use super::{crt_random, path_manager::PathSkaterAbilities, tricks::TrickTable};
use crate::attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault};
use std::sync::Arc;

pub(crate) const AI_SKATER_PROFILES_CLASS: u64 = 0xe940_7d23_0e50_6420;
const DEFAULT_PROFILE: &str = "default";
/// `ai_skater_profiles` scalars.
const ALLOWED_SKATER_FLAG: u64 = 0xa390_be5f_c0dd_a71c;
const CAN_SKATE_VERT: u64 = 0xaace_bda7_ca73_4319;
const CAN_SKATE_RACE: u64 = 0xe010_79d1_a094_b0c0;
const CAN_SKATE_STREET: u64 = 0xc36e_34b4_c619_e868;
const SPIN_PERCENTAGE_SCALAR: u64 = 0x1dd7_bea0_203d_d57b;
/// The layout's two `ProfileTrickEntry` arrays: their `Attrib::Private` headers.
const OLLIE_WEIGHTS: usize = 4;
const NOLLIE_WEIGHTS: usize = 164;
/// Layout floats 96..99 and byte 400.
const WIPEOUT_PERCENT_MEDIUM: usize = 384;
const WIPEOUT_PERCENT_HARD: usize = 388;
const WIPEOUT_PERCENT_BASIC: usize = 392;
const CELEBRATE_PERCENT: usize = 396;
const DOES_DATA_TRICKS: usize = 400;

/// `AI::WeightGroup::Weight`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Weight {
    value: f32,
    id: i32,
}

/// `AI::WeightGroup`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WeightGroup {
    weights: Vec<Weight>,
}

impl WeightGroup {
    /// `WeightGroup::Init` 0x824ba460: the entries, then each scaled by `1 / sum` when the sum
    /// is positive.
    fn new(entries: &[(i32, f32)]) -> Self {
        let mut weights: Vec<Weight> = entries.iter().map(|&(id, value)| Weight { value, id }).collect();
        let sum = entries.iter().fold(0.0f32, |sum, &(_, value)| value + sum);
        if sum > 0.0 {
            let scale = 1.0 / sum;
            for weight in &mut weights {
                weight.value *= scale;
            }
        }
        Self { weights }
    }

    /// `WeightGroup::GetRandomTrick` 0x824ba5c0: the first weight whose running sum passes a
    /// unit draw, else the first.
    pub(crate) fn random_trick(&self) -> i32 {
        let draw = crt_random::unit();
        let mut sum = 0.0f32;
        for weight in &self.weights {
            sum = weight.value + sum;
            if draw < sum {
                return weight.id;
            }
        }
        self.weights.first().map_or(-1, |weight| weight.id)
    }
}

/// `AI::SkaterProfile`.
#[derive(Clone)]
pub(crate) struct SkaterProfile {
    attribs: AttributeRuntimeInstance,
    layout: Vec<u8>,
    pub(crate) ollie_weights: WeightGroup,
    pub(crate) nollie_weights: WeightGroup,
}

impl std::fmt::Debug for SkaterProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkaterProfile").field("collection", &self.attribs.collection_key()).finish_non_exhaustive()
    }
}

impl SkaterProfile {
    /// `SkaterProfile(name, createWeightGroups)`: the named record, the default one when it is
    /// missing.
    pub(crate) fn try_new(vault: &Arc<AttributeRuntimeVault>, name: &str) -> Result<Self, AttributeRuntimeError> {
        let mut attribs = vault.instance(AI_SKATER_PROFILES_CLASS, skate2_formats::attrib::string_hash64(name));
        if attribs.is_null() {
            attribs = vault.instance(AI_SKATER_PROFILES_CLASS, skate2_formats::attrib::string_hash64(DEFAULT_PROFILE));
        }
        let layout = attribs
            .try_cached_layout_field(OLLIE_WEIGHTS, DOES_DATA_TRICKS + 4 - OLLIE_WEIGHTS)?
            .map(|data| data.bytes().to_vec())
            .unwrap_or_else(|| vec![0; DOES_DATA_TRICKS + 4 - OLLIE_WEIGHTS]);
        let mut profile = Self {
            attribs,
            layout,
            ollie_weights: WeightGroup::default(),
            nollie_weights: WeightGroup::default(),
        };
        profile.ollie_weights = WeightGroup::new(&profile.trick_entries(OLLIE_WEIGHTS));
        profile.nollie_weights = WeightGroup::new(&profile.trick_entries(NOLLIE_WEIGHTS));
        Ok(profile)
    }

    fn layout_bytes(&self, offset: usize, size: usize) -> &[u8] {
        &self.layout[offset - OLLIE_WEIGHTS..offset - OLLIE_WEIGHTS + size]
    }

    fn layout_f32(&self, offset: usize) -> f32 {
        f32::from_bits(u32::from_be_bytes(self.layout_bytes(offset, 4).try_into().expect("word")))
    }

    /// `GetOllieTrickWeight` 0x824ba688 / `GetNollieTrickWeight` 0x824ba718 over the
    /// `Attrib::Private` array: its length is the header's second halfword, the
    /// `ProfileTrickEntry`s (id, weight) follow the 8-byte header.
    fn trick_entries(&self, header: usize) -> Vec<(i32, f32)> {
        let length = usize::from(u16::from_be_bytes(self.layout_bytes(header + 2, 2).try_into().expect("length")));
        (0..length)
            .map(|index| {
                let entry = self.layout_bytes(header + 8 + index * 8, 8);
                (
                    i32::from_be_bytes(entry[..4].try_into().expect("id")),
                    f32::from_bits(u32::from_be_bytes(entry[4..].try_into().expect("weight"))),
                )
            })
            .collect()
    }

    fn scalar(&self, key: u64) -> Result<Option<u32>, AttributeRuntimeError> {
        self.attribs.try_scalar_word(key, 0)
    }

    /// A `bool` attribute's byte, false from `DefaultDataArea` when absent.
    fn flag(&self, key: u64) -> Result<bool, AttributeRuntimeError> {
        Ok(self.attribs.try_get_data(key, 0)?.is_some_and(|data| data.bytes().first().is_some_and(|byte| *byte != 0)))
    }

    /// What `Path::IsUsableBySkater` reads: the allowed-skater flag and `CanSkateRace`,
    /// `CanSkateVert` 0x824b07b8, `CanSkateStreet`.
    pub(crate) fn try_abilities(&self) -> Result<PathSkaterAbilities, AttributeRuntimeError> {
        Ok(PathSkaterAbilities {
            allowed_skater_flag: self.scalar(ALLOWED_SKATER_FLAG)?.map_or(0, |word| word as i32),
            can_skate_race: self.flag(CAN_SKATE_RACE)?,
            can_skate_vert: self.flag(CAN_SKATE_VERT)?,
            can_skate_street: self.flag(CAN_SKATE_STREET)?,
        })
    }

    /// The layout byte `PathController::GenerateTrickIntents` reads before starting a finger
    /// flip, grab or boneless from the path data.
    pub(crate) fn does_data_tricks(&self) -> bool {
        self.layout_bytes(DOES_DATA_TRICKS, 1)[0] != 0
    }

    /// `GetSpinPercentageScalar` 0x824b2e90: the scalar raised to the spin count.
    fn spin_percentage_scalar(&self, spin_180_count: u32) -> Result<f32, AttributeRuntimeError> {
        let scalar = self.scalar(SPIN_PERCENTAGE_SCALAR)?.map_or(0.0, f32::from_bits);
        let mut value = 1.0f32;
        for _ in 0..spin_180_count {
            value = scalar * value;
        }
        Ok(value)
    }

    /// `GetRandomShouldCelebrate` 0x824ba7a8.
    pub(crate) fn random_should_celebrate(&self) -> bool {
        crt_random::unit() * 100.0 < self.layout_f32(CELEBRATE_PERCENT)
    }

    /// `GetRandomShouldWipeout` 0x824ba820: a percent draw over the difficulty's wipeout
    /// percentage times the spin scalar; any difficulty but basic and hard reads medium's.
    pub(crate) fn try_random_should_wipeout(
        &self,
        tricks: &TrickTable,
        scorable: i32,
        spin_180_count: u32,
    ) -> Result<bool, AttributeRuntimeError> {
        let difficulty = tricks.settings(scorable).map_or(3, |settings| settings.difficulty);
        let draw = crt_random::unit() * 100.0;
        let scalar = self.spin_percentage_scalar(spin_180_count)?;
        let percent = match difficulty {
            0 => self.layout_f32(WIPEOUT_PERCENT_BASIC),
            2 => self.layout_f32(WIPEOUT_PERCENT_HARD),
            _ => self.layout_f32(WIPEOUT_PERCENT_MEDIUM),
        };
        Ok(draw > percent * scalar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_normalize_and_pick_by_running_sum() {
        let group = WeightGroup::new(&[(5, 1.0), (7, 3.0)]);
        assert_eq!(group.weights[0].value, 0.25);
        assert_eq!(group.weights[1].value, 0.75);
    }
}
