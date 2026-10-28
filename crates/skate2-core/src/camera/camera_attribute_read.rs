//! Scheduled CameraAttribManager.ReadAttribs825835a8 on its retained bindings.
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance},
    camera_attributes::{BINDINGS, CameraAttributes},
    camera_parameters::CameraParameters,
    physics::PointGraph,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::sync::atomic::Ordering;

#[derive(Debug)]
pub(crate) enum CameraAttributeReadError {
    MissingBinding(usize),
    Runtime(AttributeRuntimeError),
    NearClipDurationAssertion { bits: u32 },
}
impl From<AttributeRuntimeError> for CameraAttributeReadError {
    fn from(error: AttributeRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

/// Generated cached-layout readers and generic key readers remain separate.
/// Instances retain immutable cached layout identities for their full lifetime.
struct ReadBindings<'a>(&'a [AttributeRuntimeInstance]);
impl ReadBindings<'_> {
    fn instance(
        &self,
        binding: usize,
    ) -> Result<&AttributeRuntimeInstance, CameraAttributeReadError> {
        self.0
            .get(binding)
            .ok_or(CameraAttributeReadError::MissingBinding(binding))
    }
    fn layout(
        &self,
        binding: usize,
        offset: usize,
        size: usize,
    ) -> Result<Option<AttributeRuntimeData>, CameraAttributeReadError> {
        // Every layout route here is a source-generated binding with its own
        // constructor DefaultDataArea. Positioner uses generic keys only.
        debug_assert!(BINDINGS[binding].2 >= offset + size);
        Ok(self
            .instance(binding)?
            .try_cached_layout_field(offset, size)?)
    }
    fn float(&self, binding: usize, word: usize) -> Result<f32, CameraAttributeReadError> {
        Ok(self.layout(binding, word * 4, 4)?.map_or(0.0, |data| {
            f32::from_be_bytes(data.bytes().try_into().expect("four-byte layout field"))
        }))
    }
    fn byte(&self, binding: usize, offset: usize) -> Result<u8, CameraAttributeReadError> {
        Ok(self
            .layout(binding, offset, 1)?
            .map_or(0, |data| data.bytes()[0]))
    }
    fn key(&self, binding: usize, key: u64) -> Result<f32, CameraAttributeReadError> {
        // Missing pointer selects actual DefaultDataArea(4), not absent-owner success.
        Ok(f32::from_bits(
            self.instance(binding)?
                .try_scalar_word(key, 0)?
                .unwrap_or(0),
        ))
    }
    fn words(&self, binding: usize, offset: usize) -> Result<[f32; 16], CameraAttributeReadError> {
        let Some(data) = self.layout(binding, offset, 64)? else {
            return Ok([0.0; 16]);
        };
        Ok(std::array::from_fn(|i| {
            f32::from_be_bytes(
                data.bytes()[i * 4..i * 4 + 4]
                    .try_into()
                    .expect("64-byte layout"),
            )
        }))
    }
    fn curve(
        &self,
        binding: usize,
        offset: usize,
    ) -> Result<PointGraph<8>, CameraAttributeReadError> {
        let words = self.words(binding, offset)?;
        Ok(PointGraph {
            x: std::array::from_fn(|i| words[i]),
            y: std::array::from_fn(|i| words[8 + i]),
        })
    }
    /// `GetMatrix44FromAttrib82582b48` copies all sixteen input scalars without
    /// transpose or homogeneous-lane substitution. Both matrices remain full4.
    fn matrix(
        &self,
        binding: usize,
        offset: usize,
    ) -> Result<SkeletonRootMatrix, CameraAttributeReadError> {
        Ok(SkeletonRootMatrix::known(self.words(binding, offset)?))
    }
}
impl CameraAttributes {
    /// CameraManager.Update82584468..47c: clear Changed only after `ReadAttribs`.
    /// This method is never called from construction or observer admission.
    pub(crate) fn try_read_if_changed(&mut self) -> Result<bool, CameraAttributeReadError> {
        if !self.changed.load(Ordering::Relaxed) {
            return Ok(false);
        }
        self.try_read_attribs()?;
        self.changed.store(false, Ordering::Relaxed);
        Ok(true)
    }
    pub(crate) fn try_read_attribs(&mut self) -> Result<(), CameraAttributeReadError> {
        read_parameters(&ReadBindings(&self.bindings), self.parameters.fields_mut())
    }
}
#[expect(
    clippy::too_many_lines,
    reason = "native ordered parameter publication; failures retain earlier stores"
)]
fn read_parameters(
    read: &ReadBindings<'_>,
    p: &mut CameraParameters,
) -> Result<(), CameraAttributeReadError> {
    p.read_completed = false;
    p.heading_change_response_speed = Some(read.float(1, 149)?);
    p.mirroring_speed = Some(read.float(1, 143)?);
    p.turning_centred_deadzone = Some(read.float(1, 110)?);
    p.collision_interpolation_duration = Some(read.float(1, 159)?);
    p.pumping_min_acceleration = Some(read.key(1, 0x3682_afd4_2fe0_135b)?);
    p.avoidance_lookahead = Some(read.float(1, 161)?);
    p.avoidance_heading_smoothing = Some(read.float(1, 162)?);
    p.avoidance_elevation_smoothing = Some(read.float(1, 163)?);
    p.avoidance_ease_out = Some(read.float(1, 164)?);
    p.dutch_interpolation_speed = Some(read.float(1, 156)?);
    p.blend_function = Some(i32::from(read.byte(1, 0x2ac)?));
    p.max_pitch = Some(read.float(1, 144)?);
    let near_min = read.key(1, 0x5e52_78b6_e7c5_00a2)?;
    let near_max = read.key(1, 0x786e_d757_520f_d296)?;
    let duration = read.key(1, 0x80cc_1cba_de8e_f0fb)?;
    if duration.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return Err(CameraAttributeReadError::NearClipDurationAssertion {
            bits: duration.to_bits(),
        });
    }
    let delta = near_max - near_min;
    let fraction = f32::from_bits(0x3c88_8889) / duration;
    p.near_clipping_distance_min = Some(near_min);
    p.near_clipping_distance_max = Some(near_max);
    p.near_clipping_distance_step = Some(delta * fraction);
    p.drop_elevation_offset = Some(read.float(2, 56)?);
    p.drop_min_test_distance = Some(read.float(2, 52)?);
    p.drop_total_test_time = Some(read.float(2, 51)?);
    p.max_look_ahead_distance = Some(read.float(2, 55)?);
    p.drop_min_drop_distance = Some(read.float(2, 53)?);
    p.drop_max_drop_distance = Some(read.float(2, 57)?);
    p.drop_tracker_speed_clamp = Some(read.float(2, 48)?);
    p.drop_tracker_acceleration_clamp = Some(read.float(2, 50)?);
    p.drop_tracker_smoothing = Some(read.float(2, 49)?);
    p.drop_min_actor_velocity = Some(read.float(2, 54)?);
    p.drop_elevation_curve = Some(read.curve(2, 0x80)?);
    p.drop_smoothing_curve = Some(read.curve(2, 0)?);
    p.incline_smoothing_curve = Some(read.curve(2, 0x40)?);
    p.grind_min_lookahead_distance = Some(read.float(3, 3)?);
    p.grind_max_lookahead_distance = Some(read.float(3, 6)?);
    p.grind_lookahead_time = Some(read.float(3, 7)?);
    p.grind_frustum_angle_horizontal = Some(read.float(3, 12)?);
    p.grind_frustum_angle_vertical = Some(read.float(3, 11)?);
    p.grind_max_relative_angle = Some(read.float(3, 5)?);
    p.grind_frustum_pitch_offset = Some(read.float(3, 10)?);
    p.grind_opacity = Some(read.float(3, 2)?);
    p.grind_skateboard_offset = Some(read.float(3, 1)?);
    p.grind_fade_in_speed = Some(read.float(3, 9)?);
    p.grind_fade_out_speed = Some(read.float(3, 8)?);
    p.grind_skater_offset = Some(read.float(3, 0)?);
    p.grind_max_relative_height = Some(read.float(3, 4)?);
    p.shake_grind_magnitude = Some(read.key(6, 0x8e71_ca2d_c19c_7250)?);
    p.shake_one_shot_magnitude = Some(read.key(6, 0xeb2c_6d41_c03a_4b9f)?);
    p.shake_one_shot_min_velocity = Some(read.key(6, 0x978f_0d37_de3b_5fa6)?);
    p.shake_one_shot_max_velocity = Some(read.key(6, 0x83e9_879b_b1d1_4f15)?);
    p.shake_one_shot_frequency = Some(read.key(6, 0xadb9_7057_4de4_4304)?);
    p.shake_one_shot_decay = Some(read.float(6, 35)?);
    p.shake_amplitude_curve = Some(read.matrix(6, 0x40)?);
    p.shake_amplitude_min = Some(read.float(6, 41)?);
    p.shake_amplitude_max = Some(read.float(6, 42)?);
    p.shake_amplitude_top_skater_speed = Some(read.float(6, 40)?);
    p.shake_frequency_curve = Some(read.matrix(6, 0)?);
    p.shake_frequency_min = Some(read.float(6, 37)?);
    p.shake_frequency_max = Some(read.float(6, 38)?);
    p.shake_frequency_top_skater_speed = Some(read.float(6, 36)?);
    p.shake_data_fps = Some(read.float(6, 39)?);
    p.shake_translation_multiplier = Some(read.float(6, 32)?);
    p.shake_rotation_multiplier = Some(read.float(6, 34)?);
    p.shake_dutch_multiplier = Some(read.key(6, 0xf09e_bf4c_6e2f_f83a)?);
    p.rig_y_offset = Some(read.float(4, 109)?);
    p.heading_speed_clamp = Some(read.float(13, 0)?);
    p.heading_acceleration_clamp_min = Some(read.float(13, 4)?);
    p.heading_acceleration_clamp_max = Some(read.float(13, 5)?);
    p.heading_delta_umbra = Some(read.float(13, 2)?);
    p.heading_delta_penumbra = Some(read.float(13, 3)?);
    p.distance_speed_clamp = Some(read.float(11, 0)?);
    p.distance_acceleration_clamp_max = Some(read.float(11, 5)?);
    p.distance_smoothing = Some(read.float(11, 1)?);
    p.elevation_speed_clamp = Some(read.float(12, 0)?);
    p.elevation_acceleration_clamp_min = Some(read.float(12, 4)?);
    p.elevation_acceleration_clamp_max = Some(read.float(12, 5)?);
    p.elevation_delta_umbra = Some(read.float(12, 2)?);
    p.elevation_delta_penumbra = Some(read.float(12, 3)?);
    p.tracker_subject_acceleration_for_max_smoothing = Some(read.float(5, 120)?);
    p.tracker_subject_smoothing_duration = Some(read.float(5, 115)?);
    p.tracker_subject_max_speed = Some(read.float(5, 117)?);
    p.tracker_subject_max_acceleration = Some(read.float(5, 118)?);
    p.push_lag_duration = Some(read.float(9, 17)?);
    p.push_lag_max_smoothing = Some(read.float(9, 16)?);
    p.push_lag_curve = Some(read.curve(9, 0)?);
    p.pump_lag_duration = Some(read.float(10, 17)?);
    p.pump_lag_max_smoothing = Some(read.float(10, 16)?);
    p.pump_lag_curve = Some(read.curve(10, 0)?);
    p.pan_umbra = Some(read.float(8, 136)?);
    p.pan_penumbra = Some(read.float(8, 138)?);
    p.pan_smoothing_curve = Some(read.curve(8, 93 * 4)?);
    p.pan_min_smoothing = Some(read.float(8, 139)?);
    p.tilt_speed_clamp = Some(read.float(15, 0)?);
    p.tilt_acceleration_clamp_min = Some(read.float(15, 4)?);
    p.tilt_acceleration_clamp_max = Some(read.float(15, 5)?);
    p.tilt_delta_umbra = Some(read.float(15, 2)?);
    p.tilt_delta_penumbra = Some(read.float(15, 3)?);
    p.tilt_smoothing_min = Some(read.key(15, 0xa173_4944_d1ca_9582)?);
    p.tilt_smoothing_max = Some(read.key(15, 0x6be6_7dc9_f6b9_2478)?);
    p.pan_speed_clamp = Some(read.float(14, 0)?);
    p.pan_acceleration_clamp_min = Some(read.float(14, 4)?);
    p.pan_acceleration_clamp_max = Some(read.float(14, 5)?);
    p.pan_delta_umbra = Some(read.float(14, 2)?);
    p.pan_delta_penumbra = Some(read.float(14, 3)?);
    p.pan_smoothing_min = Some(read.key(14, 0xa173_4944_d1ca_9582)?);
    p.pan_smoothing_max = Some(read.key(14, 0x6be6_7dc9_f6b9_2478)?);
    p.compass_time_before_auto_lineup = Some(read.float(16, 0)?);
    p.compass_auto_lineup_speed = Some(read.float(16, 6)?);
    p.compass_min_deadzone_speed = Some(read.float(16, 2)?);
    p.compass_max_deadzone_speed = Some(read.float(16, 3)?);
    p.compass_max_deadzone_size = Some(read.float(16, 4)?);
    p.compass_deadzone_size_smoothing = Some(read.float(16, 5)?);
    p.positioner_distance_smoothing = Some(read.key(17, 0x5da7_a3ab_955a_a369)?);
    p.positioner_min_distance = Some(read.key(17, 0x1b45_309e_c3f9_d7a9)?);
    p.positioner_min_distance_during_wipeout = Some(read.key(17, 0x84c3_3901_b78d_f25c)?);
    p.positioner_camera_radius = Some(read.key(17, 0x110f_2c41_61a3_b665)?);
    p.positioner_avoidance_collision_offset = Some(read.key(17, 0x102a_6098_e794_9048)?);
    p.free_cam_elevation_offset = Some(read.float(19, 155)?);
    p.free_cam_elevation_smoothing = Some(read.float(19, 154)?);
    p.free_cam_elevation_speed = Some(read.float(19, 153)?);
    p.free_cam_heading_offset = Some(read.float(19, 152)?);
    p.free_cam_heading_smoothing = Some(read.float(19, 151)?);
    p.free_cam_heading_speed = Some(read.float(19, 150)?);
    p.read_completed = true;
    Ok(())
}

#[cfg(test)]
#[path = "camera_attribute_read_tests.rs"]
mod tests;
