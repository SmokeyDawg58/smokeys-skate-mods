//! Source-late input reads for `Toolkit_CalcSteeringTilt82b27ae0`.
use super::SkeletonProcessedContact;
use crate::skateboard_steering::calculate::{SteeringCalculationInput, SteeringScalar};
impl SteeringCalculationInput for SkeletonProcessedContact {
    type Error = &'static str;
    fn scalar(&self, field: SteeringScalar) -> Result<f32, Self::Error> {
        use SteeringScalar as S;
        let (value, name) = match field {
            S::Turn => (self.anim_attributes.turn, "turn+980"),
            S::HardTurn => (self.anim_attributes.hard_turn, "hard turn+99c"),
            S::AbsoluteSpeed => (self.toolkit.abs_body_speed(), "absolute speed+948"),
            S::TruckTightness => (self.animation_input.truck_tightness, "truck tightness+9c4"),
            S::Balance => (self.physics_state.balance, "balance+9a0"),
            S::FlippedControls => (
                self.toolkit.flipped_controls_scalar(),
                "flipped controls+94c",
            ),
        };
        value.ok_or(name)
    }
    fn pushing(&self) -> Result<bool, Self::Error> {
        let (bits, known) = self.known_flags_8c4();
        if known & 0x0800_0000 == 0 {
            return Err("push contact flag+8c4/08000000");
        }
        Ok(bits & 0x0800_0000 != 0)
    }
}
