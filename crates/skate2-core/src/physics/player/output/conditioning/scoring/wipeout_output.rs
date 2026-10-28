//! WipeoutScorer.OutputBailData82df12c0 / GetBonusScore82dec6c8.
use super::{
    super::scoring::Scoring2,
    scoring_attributes::{ScoreError, curve},
    wipeout_score::WipeoutScorer,
};
use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningVector, try_length};
impl WipeoutScorer {
    pub(super) fn output_bail(&self, out: &mut Scoring2) -> Result<(), ScoreError> {
        let pedestrian = self
            .pedestrian
            .as_ref()
            .ok_or(ScoreError::Missing("pedestrian scorer"))?;
        let vehicle = self
            .vehicle
            .as_ref()
            .ok_or(ScoreError::Missing("vehicle scorer"))?;
        let dmo = &self
            .dmo
            .published_value()
            .ok_or(ScoreError::Missing("DMO scorer"))?
            .base;
        let dt = f32::from_bits(0x3c88_8889);
        let duration_score = curve(
            &self.attributes,
            2500,
            self.moving_wipeout_duration_frames as f32 * dt,
        )? as i64 as u32;
        let bonus = [
            self.body_tweak_score,
            self.max_rotation_score,
            self.bail_speed_score,
            self.current_air_drop.drop_height_score,
            self.current_air_drop.drop_time_score,
            self.accumulated_air_drop.drop_height_score,
            self.accumulated_air_drop.drop_time_score,
            duration_score,
            dmo.score,
            vehicle.score,
            pedestrian.score,
        ]
        .into_iter()
        .fold(0u32, u32::wrapping_add);
        let bone_score = self.bone_score(None)?;
        let bail = &mut out.ho_m_bail_data;
        bail.score = bone_score.wrapping_add(bonus);
        bail.bail_duration_score = duration_score;
        bail.bone_score = bone_score;
        bail.bonus_score = bonus;
        for (i, score) in bail.bone_score_for_damage_level.iter_mut().enumerate() {
            *score = self.bone_score(Some(i as i32))?;
        }
        bail.bail_speed_score = self.bail_speed_score;
        bail.rotation_score = self.max_rotation_score;
        bail.body_tweak_score = self.body_tweak_score;
        bail.current_body_tweak_score = self.current_body_tweak_score;
        bail.current_body_tweak = if self.body_tweak_valid {
            self.current_body_tweak
        } else {
            -1
        };
        bail.just_performed_body_tweak = self.just_performed_body_tweak;
        bail.num_body_tweaks_performed = self.num_body_tweaks_performed;
        bail.current_air_drop = self.current_air_drop.clone();
        bail.max_air_drop = self.max_air_drop.clone();
        bail.accumulated_air_drop = self.accumulated_air_drop.clone();
        bail.overall_wipeout_duration_in_seconds = self.overall_wipeout_duration_frames as f32 * dt;
        bail.moving_wipeout_duration_in_seconds = self.moving_wipeout_duration_frames as f32 * dt;
        bail.severe_wipeout_duration_in_seconds = self.severe_wipeout_duration_frames as f32 * dt;
        bail.skeleton_wipeout_over_duration_in_seconds =
            self.skeleton_wipeout_over_duration_frames as f32 * dt;
        bail.bail_speed = try_length(
            self.bail_start_velocity.operand(),
            &mut HostReckoningNumeric,
        )
        .map_err(ScoreError::Numeric)?;
        bail.rotation = self.max_combined_rotation;
        for (bone, dst) in self.bones.iter().zip(&mut bail.bones.bones) {
            // Raw82df15c0..15d4 loads BoneInfo+0x24 (previous level) and
            // stores BoneScoringOutData+0x10 (current level). +0x14 is untouched.
            dst.damage_level = bone.damage_level_last_frame;
            dst.pos = bone.position;
        }
        pedestrian.fill_entity_output(&mut bail.entity_sets[0])?;
        vehicle.fill_entity_output(&mut bail.entity_sets[1])?;
        dmo.fill_entity_output(&mut bail.entity_sets[2])?;
        Ok(())
    }
}
