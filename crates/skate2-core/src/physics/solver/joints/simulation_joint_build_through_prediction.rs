//! Composes the recovered prefix of `rw::physics::JointJacobian::Build`.
//!
//! The typed path covers `0x82ae596c..0x82ae6304`: pre-Swing construction,
//! Swing routing, Twist routing, and common angular prediction. Final
//! effective-mass construction and Jacobian-record publication remain outside
//! this module.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "private complete Build stays internal pending the owning full Horus runtime"
    )
)]

use crate::{
    simulation_joint_build_stage1::{
        SimulationJointBuildStage1, SimulationJointBuildStage1Body, build_simulation_joint_stage1,
    },
    simulation_joint_build_swing::{JointSwingStageInput, apply_joint_swing_stage},
    simulation_joint_build_twist_prediction::{
        FinalSwingPackets, JointAngularBodyValues, JointPostPredictionBoundary, JointRqdRows,
        JointTwistStage1Values, build_joint_twist_prediction,
    },
    simulation_joint_numeric::SimulationJointNumericBackend,
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
};

#[derive(Clone, Copy)]
pub(crate) struct SimulationJointBuildThroughPrediction {
    pub(crate) stage1: SimulationJointBuildStage1,
    pub(crate) prediction: JointPostPredictionBoundary,
}

/// Builds the recovered `0x82ae596c..0x82ae6304` prefix with one numeric
/// backend, preserving the native global callback order and failure prefix.
pub(crate) fn build_simulation_joint_through_prediction<B: SimulationJointNumericBackend>(
    body_a: SimulationJointBuildStage1Body,
    body_b: SimulationJointBuildStage1Body,
    frames: SimulationJointFrames,
    limits: SimulationJointLimits,
    time_step: f32,
    backend: &mut B,
) -> Result<SimulationJointBuildThroughPrediction, B::Error> {
    let stage1 = build_simulation_joint_stage1(body_a, body_b, frames, limits, time_step, backend)?;
    let swing = apply_joint_swing_stage(
        JointSwingStageInput {
            raw_swing_type: limits.raw_swing_type,
            raw_twist_type: limits.raw_twist_type,
            swing_c: limits.swing_c,
            rqd_jwj: stage1.rqd.rows[1],
            rqd_jwk: stage1.rqd.rows[2],
            relative_right: stage1.relative_axes[0],
            relative_up: stage1.relative_axes[1],
            relative_at: stage1.relative_axes[2],
            body_a_frame_right: stage1.body_a_frame_right,
            t_right: stage1.body_b_frame_axes[0],
            t_up: stage1.body_b_frame_axes[1],
            t_at: stage1.body_b_frame_axes[2],
            angular_packet_a: stage1.angular_packet_a,
            angular_packet_b: stage1.angular_packet_b,
        },
        backend,
    )?;
    let prediction = build_joint_twist_prediction(
        JointTwistStage1Values {
            rqd: JointRqdRows {
                rows: stage1.rqd.rows,
            },
            relative_axes: stage1.relative_axes,
            body_a_frame_right: stage1.body_a_frame_right,
            body_a: JointAngularBodyValues {
                omega: body_a.omega,
                torque: body_a.torque,
            },
            body_b: JointAngularBodyValues {
                omega: body_b.omega,
                torque: body_b.torque,
            },
        },
        FinalSwingPackets {
            axes: [swing.t_right, swing.t_up, swing.t_at],
            angular_packet_a: swing.angular_packet_a,
            angular_packet_b: swing.angular_packet_b,
        },
        limits,
        time_step,
        backend,
    )?;

    Ok(SimulationJointBuildThroughPrediction { stage1, prediction })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation_joint_numeric::{
        SimulationJointReciprocalSite, SimulationJointReciprocalSqrtSite, SimulationJointSumSite,
        SimulationJointVector,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        Sum(SimulationJointSumSite, [u32; 3], [u32; 3]),
        Reciprocal(SimulationJointReciprocalSite, [u32; 4]),
        ReciprocalSqrt(SimulationJointReciprocalSqrtSite, [u32; 4]),
    }

    impl Event {
        fn site(self) -> Site {
            match self {
                Self::Sum(site, _, _) => Site::Sum(site),
                Self::Reciprocal(site, _) => Site::Reciprocal(site),
                Self::ReciprocalSqrt(site, _) => Site::ReciprocalSqrt(site),
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Site {
        Sum(SimulationJointSumSite),
        Reciprocal(SimulationJointReciprocalSite),
        ReciprocalSqrt(SimulationJointReciprocalSqrtSite),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Failure;

    #[derive(Default)]
    struct HostBackend {
        events: Vec<Event>,
        fail_at: Option<usize>,
    }

    impl HostBackend {
        fn fails_now(&self) -> bool {
            self.fail_at == Some(self.events.len())
        }
    }

    impl SimulationJointNumericBackend for HostBackend {
        type Error = Failure;

        fn sum3(
            &mut self,
            site: SimulationJointSumSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.events.push(Event::Sum(
                site,
                left.map(f32::to_bits),
                right.map(f32::to_bits),
            ));
            if self.fails_now() {
                return Err(Failure);
            }
            Ok(left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2])))
        }

        fn reciprocal_estimate(
            &mut self,
            site: SimulationJointReciprocalSite,
            input: SimulationJointVector,
        ) -> Result<SimulationJointVector, Self::Error> {
            self.events
                .push(Event::Reciprocal(site, input.map(f32::to_bits)));
            if self.fails_now() {
                return Err(Failure);
            }
            Ok(input.map(|lane| 1.0_f32 / lane))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            site: SimulationJointReciprocalSqrtSite,
            input: SimulationJointVector,
        ) -> Result<SimulationJointVector, Self::Error> {
            self.events
                .push(Event::ReciprocalSqrt(site, input.map(f32::to_bits)));
            if self.fails_now() {
                return Err(Failure);
            }
            Ok(input.map(|lane| lane.sqrt().recip()))
        }
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one literal test freezes the complete composed boundary"
    )]
    fn composed_hinge_arc_route_matches_literal_boundary_and_callback_order() {
        let (body_a, body_b, frames, limits) = fixture();
        let mut backend = HostBackend::default();
        let output = build_simulation_joint_through_prediction(
            body_a,
            body_b,
            frames,
            limits,
            f32::from_bits(0x3c88_8889),
            &mut backend,
        )
        .expect("host numeric backend succeeds");

        assert_eq!(
            backend
                .events
                .iter()
                .copied()
                .map(Event::site)
                .collect::<Vec<_>>(),
            [
                Site::Sum(SimulationJointSumSite::QuaternionA),
                Site::Sum(SimulationJointSumSite::QuaternionB),
                Site::Sum(SimulationJointSumSite::LimitFrameQuaternion),
                Site::Sum(SimulationJointSumSite::MotionUp),
                Site::Sum(SimulationJointSumSite::MotionRight),
                Site::Sum(SimulationJointSumSite::MotionAt),
                Site::Sum(SimulationJointSumSite::PredictedUp),
                Site::Sum(SimulationJointSumSite::PredictedRight),
                Site::Sum(SimulationJointSumSite::PredictedAt),
                Site::Reciprocal(SimulationJointReciprocalSite::Hinge),
                Site::Reciprocal(SimulationJointReciprocalSite::TwistArc),
                Site::Sum(SimulationJointSumSite::AngularUp),
                Site::Sum(SimulationJointSumSite::AngularRight),
                Site::Sum(SimulationJointSumSite::AngularAt),
            ]
        );
        assert_eq!(
            backend.events[9],
            Event::Reciprocal(SimulationJointReciprocalSite::Hinge, [0x3f44_cdca; 4])
        );
        assert_eq!(
            backend.events[10],
            Event::Reciprocal(SimulationJointReciprocalSite::TwistArc, [0xbea4_eafc; 4])
        );
        assert_eq!(
            output_words(output.prediction),
            [
                0x3eda_78b7,
                0x3f07_414d,
                0x3f30_aabe,
                0x3f2d_684e,
                0xbf02_4f5d,
                0x3f49_0560,
                0xbe9c_5bb6,
                0xbf02_4f5d,
                0xbf34_07e5,
                0xbe62_4612,
                0x3f1a_9fc6,
                0xb254_26a8,
                0xff7f_ffff,
                0x3f5e_50d0,
                0x3cfe_9a00,
                0xff7f_ffff,
                0x4012_5f18,
                0x7f7f_ffff,
                0x3cfe_9a00,
                0x7f7f_ffff,
                0xbb85_41ad,
                0xbafa_db77,
                0x3be9_a771,
                0x0000_0000,
                0x3b0e_8049,
                0xbad3_1468,
                0x3bf8_b27f,
                0x3b0e_8049,
                0x3e19_999a,
                0xbd8f_5c29,
                0xbd8f_5c29,
                0x0000_0000,
                0x3e19_999a,
                0xbd8f_5c29,
                0xbd8f_5c29,
                0x0000_0000,
            ]
        );
        assert_eq!(
            output.stage1.position_packet_a.map(f32::to_bits),
            [0xc02c_9c33, 0x401e_704b, 0xbe08_66a0, 0xbe6e_abc8]
        );
        assert_eq!(
            output.stage1.position_packet_b.map(f32::to_bits),
            [0xbfa6_0534, 0x405e_704b, 0x3fd5_5992, 0x3e6f_3218]
        );
    }

    #[test]
    fn composed_locked_route_uses_corrected_rqd_rows_and_packet_lanes() {
        let (body_a, body_b, frames, mut limits) = fixture();
        limits.raw_swing_type = 0;
        limits.raw_twist_type = 0;
        let mut backend = HostBackend::default();
        let output = build_simulation_joint_through_prediction(
            body_a,
            body_b,
            frames,
            limits,
            f32::from_bits(0x3c88_8889),
            &mut backend,
        )
        .expect("host numeric backend succeeds");

        assert_eq!(
            output_words(output.prediction),
            [
                0x3f12_4c79,
                0x3f2e_b7cc,
                0xbe68_2739,
                0x3f12_4c79,
                0xbe80_1e13,
                0x3eef_331f,
                0x3e2c_900a,
                0xbe80_1e13,
                0x3d4b_f330,
                0x3e30_42b2,
                0x3f5f_afb9,
                0x3d4b_f330,
                0xbebd_ba38,
                0x3fc1_1575,
                0xbf10_c1f4,
                0xff7f_ffff,
                0xbebd_ba38,
                0x3fc1_1575,
                0xbf10_c1f4,
                0x7f7f_ffff,
                0xbb85_41ad,
                0xbafa_db77,
                0x3be9_a771,
                0x0000_0000,
                0xbbab_ed77,
                0x3aad_ae75,
                0x3bba_bb1f,
                0xbbab_ed77,
                0x3e19_999a,
                0xbd8f_5c29,
                0xbd8f_5c29,
                0x0000_0000,
                0x3e19_999a,
                0xbd8f_5c29,
                0xbd8f_5c29,
                0x0000_0000,
            ]
        );
        assert_eq!(backend.events.len(), 12);
        assert!(
            backend
                .events
                .iter()
                .all(|event| matches!(event, Event::Sum(..)))
        );
    }

    #[test]
    fn every_numeric_failure_returns_no_value_and_retains_exact_global_prefix() {
        let (body_a, body_b, frames, mut limits) = fixture();
        limits.raw_swing_type = 1;
        let mut success = HostBackend::default();
        let successful = build_simulation_joint_through_prediction(
            body_a,
            body_b,
            frames,
            limits,
            f32::from_bits(0x3c88_8889),
            &mut success,
        );
        assert!(successful.is_ok());
        assert_eq!(success.events.len(), 15);
        assert_eq!(
            success
                .events
                .iter()
                .copied()
                .map(Event::site)
                .collect::<Vec<_>>(),
            [
                Site::Sum(SimulationJointSumSite::QuaternionA),
                Site::Sum(SimulationJointSumSite::QuaternionB),
                Site::Sum(SimulationJointSumSite::LimitFrameQuaternion),
                Site::Sum(SimulationJointSumSite::MotionUp),
                Site::Sum(SimulationJointSumSite::MotionRight),
                Site::Sum(SimulationJointSumSite::MotionAt),
                Site::Sum(SimulationJointSumSite::PredictedUp),
                Site::Sum(SimulationJointSumSite::PredictedRight),
                Site::Sum(SimulationJointSumSite::PredictedAt),
                Site::ReciprocalSqrt(SimulationJointReciprocalSqrtSite::Cone),
                Site::Reciprocal(SimulationJointReciprocalSite::Cone),
                Site::Reciprocal(SimulationJointReciprocalSite::TwistArc),
                Site::Sum(SimulationJointSumSite::AngularUp),
                Site::Sum(SimulationJointSumSite::AngularRight),
                Site::Sum(SimulationJointSumSite::AngularAt),
            ]
        );

        for fail_at in 1..=success.events.len() {
            let mut backend = HostBackend {
                events: Vec::new(),
                fail_at: Some(fail_at),
            };
            let result = build_simulation_joint_through_prediction(
                body_a,
                body_b,
                frames,
                limits,
                f32::from_bits(0x3c88_8889),
                &mut backend,
            );
            assert!(matches!(result, Err(Failure)));
            assert_eq!(backend.events, success.events[..fail_at]);
        }
    }

    fn output_words(output: JointPostPredictionBoundary) -> Vec<u32> {
        let mut words = Vec::new();
        for row in output.axes {
            words.extend(row.map(f32::to_bits));
        }
        words.extend(output.angular_packet_a.map(f32::to_bits));
        words.extend(output.angular_packet_b.map(f32::to_bits));
        words.extend(output.angular_delta.map(f32::to_bits));
        words.extend(output.angular_projection.map(f32::to_bits));
        words.extend(output.control_stack.packet_a.map(f32::to_bits));
        words.extend(output.control_stack.packet_b.map(f32::to_bits));
        words
    }

    fn fixture() -> (
        SimulationJointBuildStage1Body,
        SimulationJointBuildStage1Body,
        SimulationJointFrames,
        SimulationJointLimits,
    ) {
        let body_a = SimulationJointBuildStage1Body {
            quaternion: [0.11, -0.07, 0.19, 0.96],
            center_of_mass_and_rf_word: [1.2, -0.8, 0.4, f32::from_bits(0x1111_1111)],
            velocity: [0.31, -0.22, 0.17, 91.0],
            omega: [0.14, 0.27, -0.19, 92.0],
            right: [1.0, 0.0, 0.0, 31.0],
            up: [0.0, 1.0, 0.0, 32.0],
            at: [0.0, 0.0, 1.0, 33.0],
            force: [0.21, -0.16, 0.12, 93.0],
            torque: [-0.17, 0.08, 0.23, 94.0],
        };
        let body_b = SimulationJointBuildStage1Body {
            quaternion: [-0.13, 0.21, 0.05, 0.95],
            center_of_mass_and_rf_word: [-1.1, 0.6, 1.3, f32::from_bits(0x2222_2222)],
            velocity: [-0.18, 0.34, -0.29, 91.0],
            omega: [-0.11, 0.16, 0.24, 92.0],
            right: [0.87, 0.29, -0.19, 41.0],
            up: [-0.24, 0.94, 0.23, 42.0],
            at: [0.25, -0.15, 0.96, 43.0],
            force: [-0.14, 0.25, -0.06, 93.0],
            torque: [0.19, -0.21, 0.1, 94.0],
        };
        let frames = SimulationJointFrames {
            quaternion_a: [0.17, -0.23, 0.31, 0.89],
            position_a: [0.41, -0.27, 0.13, 9.0],
            quaternion_b: [-0.19, 0.29, 0.11, 0.91],
            position_b: [-0.37, 0.22, 0.46, -8.0],
            quaternion_l: [0.09, -0.14, 0.21, 0.96],
        };
        let limits = SimulationJointLimits {
            p_prism: [-0.7, -0.5, -0.9, 13.0],
            v_prism: [0.8, 0.65, 1.1, -14.0],
            v_twist: 0.15,
            v_swing: -0.07,
            swing_a: 0.72,
            twist_a: 0.63,
            swing_c: 0.37,
            twist_c: -0.24,
            raw_swing_type: 2,
            raw_twist_type: 1,
        };
        (body_a, body_b, frames, limits)
    }
}
