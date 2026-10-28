//! PerfectBodyFlips uses TU3 UpdateBodyFlip82D8EC70 on the retained S2 axes.
use super::{Reckoning, Vector};
use crate::{
    reckoning_attributes::ReckoningAttributeRead, skate3_native::physics::player::tuning::Tuning,
};
use skate3_core::air::body_flip::{self, BodyFlipInput, BodyFlipState};

impl<A: ReckoningAttributeRead> Reckoning<A> {
    pub(crate) fn update_source_body_flip(&mut self, target: f32, tuning: &Tuning) {
        let mut state = BodyFlipState {
            angle: self.flip_angle,
            speed: self.flip_speed,
            requested_speed: self.target_flip_speed,
            // Both are output buffers: the first is always overwritten, the
            // second only at nonzero angle. Do not read unobserved S2 W lanes.
            spin_transform: [[0.; 4]; 4],
            combined_transform: [[0.; 4]; 4],
        };
        let [x, y, z] = self.up.xyz;
        let [fx, fy, fz] = self.flip_axis.xyz;
        body_flip::update(
            &mut state,
            &tuning.air_assists.flip,
            &BodyFlipInput {
                requested_speed: target,
                spin_angle: self.spin_angle,
                normal: [x, y, z, 0.],
                flip_axis: [fx, fy, fz, 0.],
                // Perfect mode uses its fixed step and does not read this value.
                timestep: 0.,
                perfect_body_flips: true,
            },
        );
        self.target_flip_speed = state.requested_speed;
        self.flip_speed = state.speed;
        self.flip_angle = state.angle;
        self.body_spin_transform =
            super::super::Matrix::from_columns(state.spin_transform.map(Vector::known));
        if state.angle != 0. {
            self.flip_transform =
                super::super::Matrix::from_columns(state.combined_transform.map(Vector::known));
        }
    }
}
