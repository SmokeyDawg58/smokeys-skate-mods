//! Single-argument `SetForceTurnAngle82b24cd8`, distinct from the contact overload.
use super::SkateboardTruckDrives;
impl SkateboardTruckDrives {
    pub(crate) fn set_wipeout_force_turn_angle(&mut self, angle: f32) {
        self.turn_front = angle;
        self.turn_back = angle;
        self.force_turn = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simple_turn_copies_zero_sign_and_preserves_contact_timers() {
        let mut value =
            SkateboardTruckDrives::new(crate::skateboard_joints::SkateboardTruckTransforms {
                front: super::super::IDENTITY,
                back: super::super::IDENTITY,
            });
        value.time_front = f32::from_bits(0x7fc0_1234);
        value.time_back = -3.0;
        value.set_wipeout_force_turn_angle(-0.0);
        assert_eq!(value.turn_front.to_bits(), 0x8000_0000);
        assert_eq!(value.turn_back.to_bits(), 0x8000_0000);
        assert_eq!(value.time_front.to_bits(), 0x7fc0_1234);
        assert_eq!(value.time_back.to_bits(), (-3.0_f32).to_bits());
        assert!(value.force_turn);
    }
}
