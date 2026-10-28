//! HoM bone update82df5980 and damage propagation82deca00. Consume the existing
//! SkeletonCollision, including its native separate face/groin contact parts.
use super::{
    super::collision::SkeletonCollision,
    scoring_attributes::{ScoreError, curve, layout_word, scalar},
    wipeout_score::WipeoutScorer,
};

pub(super) struct DamageTuning {
    pub propagate: bool,
    pub minimum_propagation: f32,
    pub scalar: f32,
    pub acceleration_for_force: bool,
}
impl WipeoutScorer {
    pub(super) fn damage_bone(
        &mut self,
        index: usize,
        source: usize,
        amount: f32,
        tuning: &DamageTuning,
    ) -> Result<(), ScoreError> {
        let bone = self
            .bones
            .get_mut(index)
            .ok_or(ScoreError::Invalid("wipeout bone index"))?;
        bone.damage_amount = amount + bone.damage_amount;
        for link in 0..4 {
            let offset = index * 84 + 4 + link * 8;
            let other = layout_word(&self.attributes, offset)? as i32;
            if other != -1 && other != source as i32 {
                let fraction = if tuning.propagate {
                    f32::from_bits(layout_word(&self.attributes, offset + 4)?)
                } else {
                    0.0
                };
                let damage = fraction * amount;
                if damage > tuning.minimum_propagation {
                    // Native excludes the immediate source, not every earlier
                    // bone; diminishing paths can propagate through a cycle.
                    self.damage_bone(other as usize, index, damage, tuning)?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn update_bones(
        &mut self,
        collision: &SkeletonCollision,
        tuning: &DamageTuning,
    ) -> Result<(), ScoreError> {
        // PreUpdateBones82decc10 must precede every propagation write.
        for bone in &mut self.bones {
            bone.position_last_frame = bone.position;
            bone.damage_level_last_frame = bone.damage_level;
        }
        self.is_airborne = true;
        for index in 0..25 {
            // EBoneIndexToPartIndex82debcb0: 23 face shares neck, 24 groin hips.
            let part = match index {
                23 => 1,
                24 => 23,
                _ => index + 1,
            };
            let contact = &collision.bones[part];
            let data = if index >= 23 {
                &contact.sub
            } else {
                &contact.main
            };
            self.bones[index].position = contact.position;
            self.bones[index].contact_surface_id = data.surface_id;
            self.bones[index].is_airborne =
                !(contact.hit_world && data.normal.xyz[1] > f32::from_bits(0x3ea8_f5c3));
            let force = if tuning.acceleration_for_force {
                data.acceleration
            } else {
                data.force
            };
            let damage = (tuning.scalar * scalar(&self.attributes, 0xb751_4de7_ebde_9d01)?) * force;
            if damage > tuning.minimum_propagation {
                self.damage_bone(index, index, damage, tuning)?;
            }
            self.is_airborne &= self.bones[index].is_airborne;
        }
        // PostUpdateBones82df1638: first positive threshold strictly exceeded,
        // descending from break-all5, otherwise Invalid-1.
        for (index, bone) in self.bones.iter_mut().enumerate() {
            bone.damage_level = -1;
            for level in (0..6).rev() {
                let threshold =
                    f32::from_bits(layout_word(&self.attributes, index * 84 + 40 + level * 8)?);
                if threshold > 0.0 && bone.damage_amount > threshold {
                    bone.damage_level = level as i32;
                    break;
                }
            }
            if self.overall_wipeout_duration_frames == 0 {
                bone.position_last_frame = bone.position;
            }
        }
        Ok(())
    }
    pub(super) fn bone_score(&self, level: Option<i32>) -> Result<u32, ScoreError> {
        let mut score = 0u32;
        for (index, bone) in self.bones.iter().enumerate() {
            if bone.damage_level != -1 && level.is_none_or(|l| l == bone.damage_level) {
                if !(0..6).contains(&bone.damage_level) {
                    return Err(ScoreError::Invalid("bone damage level"));
                }
                score = score.wrapping_add(layout_word(
                    &self.attributes,
                    index * 84 + 36 + bone.damage_level as usize * 8,
                )?);
            }
        }
        Ok(score)
    }
    pub(super) fn check_heavy_landing(&mut self, kind: i32, speed: f32, reset_frames: i32) {
        if self.frames_since_heavy_landing_signal >= 0 {
            self.frames_since_heavy_landing_signal =
                self.frames_since_heavy_landing_signal.wrapping_add(1);
            if self.frames_since_heavy_landing_signal == reset_frames {
                self.left_foot_heavy_landing_speed = 0.0;
                self.right_foot_heavy_landing_speed = 0.0;
                self.frames_since_heavy_landing_signal = -1;
            }
        }
        match kind {
            0 => {
                self.frames_since_heavy_landing_signal = 0;
                self.left_foot_heavy_landing_speed = speed;
                self.right_foot_heavy_landing_speed = speed;
            }
            1 => {
                self.frames_since_heavy_landing_signal = 0;
                self.left_foot_heavy_landing_speed = speed;
                self.right_foot_heavy_landing_speed = 0.0;
            }
            2 => {
                self.frames_since_heavy_landing_signal = 0;
                self.left_foot_heavy_landing_speed = 0.0;
                self.right_foot_heavy_landing_speed = speed;
            }
            _ => {}
        }
    }
    pub(super) fn damage_feet_after_landing(
        &mut self,
        tuning: &DamageTuning,
    ) -> Result<(), ScoreError> {
        let left = curve(&self.attributes, 2100, self.left_foot_heavy_landing_speed)?;
        let left = (tuning.scalar * scalar(&self.attributes, 0xb751_4de7_ebde_9d01)?) * left;
        let right = curve(&self.attributes, 2100, self.right_foot_heavy_landing_speed)?;
        let right = (tuning.scalar * scalar(&self.attributes, 0xb751_4de7_ebde_9d01)?) * right;
        self.damage_bone(15, 15, left, tuning)?;
        self.damage_bone(
            14,
            14,
            left * scalar(&self.attributes, 0x72a3_ccc3_c756_97cd)?,
            tuning,
        )?;
        self.damage_bone(19, 19, right, tuning)?;
        self.damage_bone(
            18,
            18,
            scalar(&self.attributes, 0x72a3_ccc3_c756_97cd)? * right,
            tuning,
        )?;
        self.left_foot_heavy_landing_speed = 0.0;
        self.right_foot_heavy_landing_speed = 0.0;
        Ok(())
    }
}
