//! Source CalculateOBCollisionTurn82c92c68 and CalculateOBStepTurn82c8b608.
use super::ActorAnimationConstruction;
use crate::{
    attribute_runtime::AttributeRuntimeError,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physics::PointGraph,
    reckoning_numeric::{ReckoningNumeric, try_normalize_safe, wrap_turns_to_radians},
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_ik_tangent::{SkeletonIkTangentError, try_tangent},
    skeleton_root_transforms::SkeletonRootVector as V,
};
#[derive(Debug)]
pub(crate) enum TurnError<N, R> {
    Missing(&'static str),
    Numeric(N),
    Tangent(SkeletonIkTangentError<N, R>),
    Cvar(InputCvarError),
}
#[derive(Debug)]
pub(crate) enum StepTurnError {
    Missing,
    Attribute(AttributeRuntimeError),
    Truncated(usize),
}
const DEGREES: f32 = f32::from_bits(0x3c8efa35);
impl<K, W> ActorAnimationConstruction<K, W> {
    pub(crate) fn try_calculate_ob_step_turn(&self, step_turn: f32) -> Result<f32, StepTurnError> {
        let sign = if step_turn > 0.0 { 1.0 } else { -1.0 };
        let magnitude = step_turn.abs();
        let data = self
            .offboard_attributes()
            .ok_or(StepTurnError::Missing)?
            .try_get_data(0x24e7_6e2c_af7a_90f8, 0)
            .map_err(StepTurnError::Attribute)?;
        if let Some(ref data) = data {
            if data.bytes().len() < 0x50 {
                return Err(StepTurnError::Truncated(data.bytes().len()));
            }
        }
        let word = |offset| {
            data.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(data.bytes()[offset..offset + 4].try_into().unwrap())
            })
        };
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| word(0x10 + 4 * i)),
            y: std::array::from_fn(|i| word(0x30 + 4 * i)),
        }
        .evaluate(magnitude)
            * sign)
    }
    pub(crate) fn try_calculate_ob_collision_turn<
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        heading: V,
        is_jumping: bool,
        refs: &InputCvarReferences,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<f32, TurnError<N::Error, R::Error>> {
        use TurnError as E;
        let normal = self
            .before_face
            .as_ref()
            .ok_or(E::Missing("locomotion suffix"))?
            .last_collision_normal;
        if numeric.dot_vector(normal, heading).map_err(E::Numeric)? > 0.0 {
            return Ok(0.0);
        }
        // Raw permute<0,5,0,0>, then replace Z from splatted normal.X. W=-normal.Z.
        let neg_z = f32::from_bits(normal.xyz[2].to_bits() ^ 0x8000_0000);
        let side = V::known([neg_z, 0.0, normal.xyz[0], neg_z]);
        let opposite = side.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
        let additional = refs
            .try_loco_collision_additional_angle()
            .map_err(E::Cvar)?
            * DEGREES;
        let tangent = try_tangent(additional, numeric, rounding).map_err(E::Tangent)?;
        let scaled = normal.map(|v| v * tangent);
        let left = side.zip(scaled, |a, b| a + b);
        let right = opposite.zip(scaled, |a, b| a + b);
        let left_dot = numeric.dot_vector(left, heading).map_err(E::Numeric)?;
        let right_dot = numeric.dot_vector(right, heading).map_err(E::Numeric)?;
        let selected = if left_dot > right_dot { left } else { right };
        let direction =
            try_normalize_safe(selected, V::known([0.0; 4]), numeric).map_err(E::Numeric)?;
        let angle = crate::physical_player_phys_out::construct::skeleton_fill_suffix::plane_angle(
            heading,
            direction,
            V::known([0.0, 1.0, 0.0, 0.0]),
            numeric,
        )
        .map_err(E::Numeric)?;
        let mut angle = wrap_turns_to_radians(angle * f32::from_bits(0x3e22f983));
        let max_start = refs
            .try_loco_collision_max_start_angle(is_jumping)
            .map_err(E::Cvar)?
            * DEGREES;
        if angle.abs() > max_start {
            angle = 0.0;
            let player = self
                .player
                .as_mut()
                .ok_or(E::Missing("animation player flags"))?;
            player.flags_bits |= 4;
            player.flags_known_mask |= 4;
        }
        let maximum = refs.try_loco_collision_max_turn_angle().map_err(E::Cvar)? * DEGREES;
        Ok(clamp_turn(angle, maximum))
    }
}
fn clamp_turn(angle: f32, maximum: f32) -> f32 {
    let low = if -maximum - angle >= 0.0 {
        -maximum
    } else {
        angle
    };
    if maximum - low >= 0.0 { low } else { maximum }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clamp_retains_native_fsel_unordered_and_negative_limit() {
        assert_eq!(clamp_turn(f32::NAN, 2.0), 2.0);
        assert!(clamp_turn(1.0, f32::NAN).is_nan());
        assert_eq!(clamp_turn(0.0, -2.0), -2.0);
        assert_eq!(clamp_turn(3.0, 2.0), 2.0);
    }
}
