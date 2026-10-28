//! CharacterDamage82deb9f0/ba70/bb58/bbf0, existing seven-area output owner.
use super::{
    super::scoring::Scoring2,
    scoring_attributes::{ScoreError, scalar},
    wipeout_score::WipeoutScorer,
};

// EBoneIndexToCharacterDamageArea82debd58. Native enum order documented in
// scoring-wipeout-extra-2026-09-30.txt; -1 bones do not affect render damage.
const AREAS: [i32; 25] = [
    -1, -1, 0, 0, 0, 2, 1, 1, 1, 2, 2, 2, 3, 3, -1, -1, 4, 4, -1, -1, 5, 5, 3, 6, 3,
];
impl WipeoutScorer {
    pub(super) fn update_character_damage(
        &mut self,
        reset: bool,
        forced_level: i32,
        fade_in: f32,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        if reset {
            for area in &mut self.character_damage {
                area.damage_level = -1;
                area.intensity_percentage = 0.0;
                area.is_fading_in = false;
            }
        }
        let fade_out = if self.is_wiping_out || forced_level > 0 {
            for (bone, &area_index) in self.bones.iter().zip(&AREAS) {
                if area_index == -1 {
                    continue;
                }
                let area = &mut self.character_damage[area_index as usize];
                let level = if forced_level > 0 {
                    forced_level - 2
                } else {
                    bone.damage_level
                };
                if level > area.damage_level || forced_level > 0 {
                    area.damage_level = level;
                    area.is_fading_in = true;
                }
            }
            0.0
        } else {
            scalar(&self.attributes, 0xd6cb_5a92_88be_7ac1)?
        };
        // Source rates are per Update, not multiplied by simulation delta.
        for (area, dst) in self
            .character_damage
            .iter_mut()
            .zip(&mut out.character_damage_data.areas)
        {
            if area.is_fading_in {
                area.intensity_percentage += fade_in;
                if area.intensity_percentage >= 100.0 {
                    area.intensity_percentage = 100.0;
                    area.is_fading_in = false;
                }
            } else if area.intensity_percentage > 0.0 {
                area.intensity_percentage -= fade_out;
                if area.intensity_percentage < 0.0 {
                    area.intensity_percentage = 0.0;
                    area.damage_level = -1;
                }
            }
            if area.damage_level == -1 {
                area.intensity_percentage = 0.0;
                area.is_fading_in = false;
            }
            dst.damage_level = area.damage_level as i8;
            dst.intensity = (area.intensity_percentage * 2.55) as i64 as u8;
        }
        Ok(())
    }
}
