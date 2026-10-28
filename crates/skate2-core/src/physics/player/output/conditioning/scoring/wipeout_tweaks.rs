//! WipeoutScorer.UpdateBodyTweaks82dec470. Authored directions/hold frames/points.
use super::{
    scoring_attributes::{ScoreError, scalar},
    wipeout_score::WipeoutScorer,
};
use crate::reckoning_numeric::{HostReckoningNumeric, try_unsigned_angle_2d};
impl WipeoutScorer {
    pub(super) fn update_body_tweaks(&mut self, gesture: [f32; 2]) -> Result<(), ScoreError> {
        let previous = self.current_body_tweak;
        self.body_tweak_valid = false;
        self.just_performed_body_tweak = false;
        self.current_body_tweak = -1;
        self.current_body_tweak_score = 0;
        let list = 0x5717_cfcc_ca59_bd47;
        let count = self.attributes.try_length(list)?;
        let angle_limit = f32::from_bits(0x4049_0fdb) / count as f32;
        let threshold = scalar(&self.attributes, 0x9c49_4e96_14fa_5ed6)?;
        let squared = gesture[0] * gesture[0] + gesture[1] * gesture[1];
        if squared > threshold * threshold {
            for index in 0..count {
                // Native82dec5cc..68c reads ID+0, direction+8/+12, hold+16,
                // points+20. The relocated name pointer at+4 is not consumed.
                // Project those numeric fields separately, retaining fixup
                // rejection if a relocation overlaps an actual numeric field.
                let id = self.attributes.try_array_field_data(
                    list, index, 0x1457_419c_501e_10ed, 24, 0..4,
                )?;
                let data = self.attributes.try_array_field_data(
                    list, index, 0x1457_419c_501e_10ed, 24, 8..24,
                )?;
                let b = match data.as_ref() {
                    Some(value) => value
                        .bytes()
                        .get(..16)
                        .ok_or(ScoreError::Invalid("body tweak entry"))?,
                    None => &[0; 16],
                };
                let u = |i: usize| u32::from_be_bytes(b[i * 4..i * 4 + 4].try_into().unwrap());
                let direction = [f32::from_bits(u(0)), f32::from_bits(u(1))];
                let angle = try_unsigned_angle_2d(direction, gesture, &mut HostReckoningNumeric)
                    .map_err(ScoreError::Numeric)?;
                if angle < angle_limit {
                    self.current_body_tweak = id.as_ref().map_or(0, |value|
                        i32::from_be_bytes(value.bytes().try_into().expect("checked ID width")));
                    self.body_tweak_duration_frames = if self.current_body_tweak == previous {
                        self.body_tweak_duration_frames.wrapping_add(1)
                    } else {
                        0
                    };
                    if self.body_tweak_duration_frames == u(2) {
                        self.body_tweak_score = self.body_tweak_score.wrapping_add(u(3));
                        self.current_body_tweak_score = u(3); // raw lwz, not float conversion
                        self.just_performed_body_tweak = true;
                        self.num_body_tweaks_performed =
                            self.num_body_tweaks_performed.wrapping_add(1);
                    }
                    self.body_tweak_valid = self.body_tweak_duration_frames >= u(2);
                    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
                    if *TRACE.get_or_init(|| std::env::var("SKATE2_TRACE_SCORING").is_ok_and(|v| v == "1"))
                        && (self.body_tweak_duration_frames == 0 || self.just_performed_body_tweak)
                    {
                        std::eprintln!("scoring body_tweak={} frames={} required={} awarded={}",
                            self.current_body_tweak, self.body_tweak_duration_frames,
                            u(2), self.current_body_tweak_score);
                    }
                    return Ok(());
                }
            }
        }
        self.body_tweak_duration_frames = 0;
        Ok(())
    }
}
