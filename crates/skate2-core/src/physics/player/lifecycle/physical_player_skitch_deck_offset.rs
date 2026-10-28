//! UpdateDeckAnimOffset82d96430. Below the threshold no setter is called.
use super::{
    Matrix, Skitching,
    skitch_state::{SkitchError, clamp, need},
};
use crate::physical_player_grind_manager::math::{DEG_TO_RAD, GrindNum};
impl Skitching {
    pub(crate) fn deck_animation_offset(
        &mut self,
        blend: f32,
        max_delta: f32,
        min_angle: f32,
        n: &mut GrindNum<'_>,
    ) -> Result<Option<Matrix>, SkitchError> {
        let angle = need(self.deck_angle, "deck angle")?;
        let sign = if angle >= 0.0 { 1.0 } else { -1.0 };
        let target = self.graph(
            0x30fd_fce1_85cd_3d4f,
            (angle * sign) * f32::from_bits(0x4265_2ee1),
        )? * DEG_TO_RAD;
        let old = need(self.anim_deck_angle, "animation deck angle")?;
        let maximum = max_delta * DEG_TO_RAD;
        let next = clamp(target.mul_add(sign, -old) * blend, -maximum, maximum) + old;
        self.anim_deck_angle = Some(next);
        if !(next.abs() > min_angle * DEG_TO_RAD) {
            return Ok(None);
        }
        let (s, c) = n.sin_cos(next)?;
        Ok(Some([
            [c, 0., -s, 0.],
            [0., 1., 0., 0.],
            [s, 0., c, 0.],
            [0.; 4],
        ]))
    }
}
