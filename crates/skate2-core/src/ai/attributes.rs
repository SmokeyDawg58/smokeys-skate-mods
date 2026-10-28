//! `Attrib::Gen::ai_skater`, the `SH::ai_skater_default` instance (class `ai_skater`, collection
//! `default`). Native reads look each value up by its 32-bit key on every use; the vault is
//! immutable, so the values are read once.
use crate::attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault};
use std::sync::Arc;

pub(crate) const AI_SKATER_CLASS: u64 = 0xcb59_c218_2c87_f89f;
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

/// The scalars the AI reads, named for their use; each cites its 32-bit key.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AiSkaterAttributes {
    /// 0x0da11691: `FindNewPath`'s query radius.
    pub(crate) find_new_path_radius: f32,
    /// 0xbd8606f7: degrees of deviation for a full turn (`TurnOnGround`,
    /// `GenerateTurnIntentsOffBoard`).
    pub(crate) full_turn_deviation_degrees: f32,
    /// 0x93462295: degrees of air deviation ignored by `TurnInAir`.
    pub(crate) air_turn_dead_zone_degrees: f32,
    /// 0xbb965adc: degrees of air deviation for a full turn.
    pub(crate) air_full_turn_degrees: f32,
    /// 0x2386b08a: `AdjustDesiredSpeed`'s scale for the influence's speed adjustment.
    pub(crate) speed_adjustment_scale: f32,
    /// 0xe3fc0942: `AdjustDesiredSpeed`'s scale for the rubber band.
    pub(crate) rubber_band_scale: f32,
    /// 0xae48d280: path distance past which `GenerateSpeedIntents` raises the desired speed.
    pub(crate) off_path_distance_for_minimum_speed: f32,
    /// 0x3b4d18a2: that minimum.
    pub(crate) off_path_minimum_speed: f32,
    /// 0x7d661e2e: degrees of deviation under which `GenerateBrakeIntents` brakes.
    pub(crate) maximum_path_deviation_for_brake: f32,
    /// 0x4d12b528 `MaximumPathDeviationForPush`.
    pub(crate) maximum_path_deviation_for_push: f32,
    /// 0xa5c28868 `FullBrakeThreshold`.
    pub(crate) full_brake_threshold: f32,
    /// 0xb7498e75: path distance under which `CalculatePathDeviation` averages both angles.
    pub(crate) deviation_blend_distance: f32,
    /// 0xb69b48d8: path distance under which `AttemptToJoinPath` joins.
    pub(crate) join_path_distance: f32,
    /// 0x72352742: path distance past which `BlendToPath` leaves the path.
    pub(crate) leave_path_distance: f32,
    /// 0x30d72610: `GetMaxWipeoutDurationInSeconds` outside races and follow challenges.
    pub(crate) max_wipeout_duration: f32,
    /// 0xf4191e92: the same in race challenges.
    pub(crate) max_wipeout_duration_race: f32,
    /// 0x337d654e: `CalculateManualIntentValue`.
    pub(crate) manual_intent_value: f32,
    /// 0x610b41d4 and 0x827fd781: race rubber banding's time window.
    pub(crate) race_rubber_band_min: f32,
    pub(crate) race_rubber_band_max: f32,
    /// 0x2cf8f7ce: race rubber banding's floor when the local player wipes out.
    pub(crate) race_rubber_band_wipeout: f32,
    /// 0xfa888c9a and 0x22b63160: follow rubber banding's distance window.
    pub(crate) follow_rubber_band_min: f32,
    pub(crate) follow_rubber_band_max: f32,
    /// 0x835fd79c: follow rubber banding's floor when the player wipes out.
    pub(crate) follow_rubber_band_wipeout: f32,
    /// 0x7c1d006c: the obstacle gathering radius.
    pub(crate) obstacle_radius: f32,
    /// 0x01027e69: degrees off the heading inside which `TestObstacle` always keeps an obstacle.
    pub(crate) obstacle_ahead_angle_degrees: f32,
    /// 0xaab2ecdd and 0x54fde594: the wider angle in degrees and the surface distance inside
    /// which it keeps a near one.
    pub(crate) obstacle_near_angle_degrees: f32,
    pub(crate) obstacle_near_distance: f32,
    /// 0x11a9358a `LocalSkaterRadius`.
    pub(crate) local_skater_radius: f32,
    /// 0x1463e703: how far over the desired speed `CalculateSpeedAdjust` still passes ahead.
    pub(crate) pass_ahead_speed_margin: f32,
    /// 0x3c5881fa `AvoidanceSpeedAdjustment`.
    pub(crate) avoidance_speed_adjustment: f32,
    /// 0x05bede75: the surface distance under which `Obstacle::Set` must avoid an obstacle
    /// now; 0x8f0383d6 and 0xcc43fa72: a farther distance and the angle in degrees inside which
    /// it must too.
    pub(crate) must_avoid_distance: f32,
    pub(crate) must_avoid_near_distance: f32,
    pub(crate) must_avoid_near_angle_degrees: f32,
}

impl AiSkaterAttributes {
    pub(crate) fn try_load(vault: &Arc<AttributeRuntimeVault>) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(AI_SKATER_CLASS, DEFAULT_COLLECTION);
        let value = |key: u64| -> Result<f32, AttributeRuntimeError> {
            Ok(instance.try_scalar_word(key, 0)?.map_or(0.0, f32::from_bits))
        };
        Ok(Self {
            find_new_path_radius: value(0x4f87_e7a7_0da1_1691)?,
            full_turn_deviation_degrees: value(0xfed2_4a60_bd86_06f7)?,
            air_turn_dead_zone_degrees: value(0xdd88_43f7_9346_2295)?,
            air_full_turn_degrees: value(0x281f_55d7_bb96_5adc)?,
            speed_adjustment_scale: value(0x156a_6593_2386_b08a)?,
            rubber_band_scale: value(0xdcd5_de4b_e3fc_0942)?,
            off_path_distance_for_minimum_speed: value(0xa802_7415_ae48_d280)?,
            off_path_minimum_speed: value(0x8213_474c_3b4d_18a2)?,
            maximum_path_deviation_for_brake: value(0x8ac7_c833_7d66_1e2e)?,
            maximum_path_deviation_for_push: value(0x8692_3c10_4d12_b528)?,
            full_brake_threshold: value(0x7e70_24bd_a5c2_8868)?,
            deviation_blend_distance: value(0xe4e7_54ad_b749_8e75)?,
            join_path_distance: value(0x84fd_5f39_b69b_48d8)?,
            leave_path_distance: value(0x996c_5d66_7235_2742)?,
            max_wipeout_duration: value(0xecec_5858_30d7_2610)?,
            max_wipeout_duration_race: value(0x0c7f_6466_f419_1e92)?,
            manual_intent_value: value(0x742e_1ee3_337d_654e)?,
            race_rubber_band_min: value(0xbd0b_ca0e_610b_41d4)?,
            race_rubber_band_max: value(0xa1a0_3879_827f_d781)?,
            race_rubber_band_wipeout: value(0x3473_7e41_2cf8_f7ce)?,
            follow_rubber_band_min: value(0x0fcc_7faf_fa88_8c9a)?,
            follow_rubber_band_max: value(0x5044_2dae_22b6_3160)?,
            follow_rubber_band_wipeout: value(0x3da6_12f5_835f_d79c)?,
            obstacle_radius: value(0xab20_d7d4_7c1d_006c)?,
            obstacle_ahead_angle_degrees: value(0xd869_1a78_0102_7e69)?,
            obstacle_near_angle_degrees: value(0x2218_46f2_aab2_ecdd)?,
            obstacle_near_distance: value(0xe1c0_f69e_54fd_e594)?,
            local_skater_radius: value(0x8ef4_fb9d_11a9_358a)?,
            pass_ahead_speed_margin: value(0x3c55_9500_1463_e703)?,
            avoidance_speed_adjustment: value(0xc373_bac2_3c58_81fa)?,
            must_avoid_distance: value(0x07f5_ed41_05be_de75)?,
            must_avoid_near_distance: value(0x5aed_d653_8f03_83d6)?,
            must_avoid_near_angle_degrees: value(0x5101_0ffb_cc43_fa72)?,
        })
    }
}
