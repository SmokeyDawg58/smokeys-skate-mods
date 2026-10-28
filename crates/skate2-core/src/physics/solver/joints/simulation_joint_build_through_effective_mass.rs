//! Composes the recovered value graph of `rw::physics::JointJacobian::Build`.
//!
//! The numeric builder covers `0x82ae596c..0x82ae6754`. The intervening
//! `0x82ae6308..0x82ae6324` instructions clear the private destination record
//! and issue cache hints; the companion inert-record wrapper incorporates the
//! clear result and exact terminal stores through `0x82ae678c`. Checked record
//! ownership, checked RF/Joint binding, and island-table insertion are supplied
//! by a private sibling; checked table execution and RF effects remain outside
//! this module.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaits the owning full Horus runtime")
)]

use crate::{
    simulation_joint_build_effective_mass_middle::{
        AxisRows, InertiaRows, MiddleInput, MiddleOutput, PendingCross, build_effective_mass_middle,
    },
    simulation_joint_build_effective_mass_prefix::{
        JointAttachmentRows, JointEffectiveMassBodyRows, JointEffectiveMassPrefixHandoff,
        JointEffectiveMassPrefixInput, build_joint_effective_mass_prefix,
    },
    simulation_joint_build_effective_mass_suffix::{
        JointEffectiveMassSuffixInput, JointEffectiveMassSuffixOutput,
        build_joint_effective_mass_suffix,
    },
    simulation_joint_build_final_record::{
        JointFinalRecordAssembly, JointFinalRecordAssemblyInput, JointFinalScaledRows,
        JointFinalStableRows, assemble_joint_final_record,
    },
    simulation_joint_build_stage1::SimulationJointBuildStage1Body,
    simulation_joint_build_through_prediction::{
        SimulationJointBuildThroughPrediction, build_simulation_joint_through_prediction,
    },
    simulation_joint_numeric::{SimulationJointNumericBackend, SimulationJointVector},
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
};

/// One copied body snapshot containing every row consumed through 0x6754.
///
/// Packed live W lanes include an RF word, inverse mass, and state bits. Other
/// source W lanes remain explicit for raw intermediate audits; a checked
/// final-record adapter may zero them only under the proven metadata-W
/// deadness boundary. This private non-`Debug` value is not an owning body
/// capability.
#[derive(Clone, Copy)]
pub(crate) struct SimulationJointBuildBody {
    pub(crate) stage1: SimulationJointBuildStage1Body,
    pub(crate) full_inertia_and_inverse_mass: SimulationJointVector,
    pub(crate) split_inertia_and_state: SimulationJointVector,
}

/// Complete recovered value boundary after 0x82ae6754.
///
/// Partial record-store observations remain inert copied values. This result
/// grants no record, pool, body, RF, solver, or publication authority.
#[derive(Clone, Copy)]
pub(crate) struct SimulationJointBuildThroughEffectiveMass {
    pub(crate) prediction: SimulationJointBuildThroughPrediction,
    pub(crate) prefix: JointEffectiveMassPrefixHandoff,
    pub(crate) middle: MiddleOutput,
    pub(crate) suffix: JointEffectiveMassSuffixOutput,
}

/// Raw native-word observations needed only to freeze an inert final image.
///
/// These words do not create host bindings or confer Joint/pool/RF authority.
#[derive(Clone, Copy)]
pub(crate) struct SimulationJointInertRecordWords {
    pub(crate) joint_binding_word: u32,
    pub(crate) transient_spy_word: u32,
}

/// Recovered Build value graph plus its inert final 0x180-byte record image.
///
/// A checked pool-backed preparation path must replace raw binding words with
/// private validated tokens before a solver may consume this result.
#[derive(Clone, Copy)]
pub(crate) struct SimulationJointBuildInertRecord {
    pub(crate) build: SimulationJointBuildThroughEffectiveMass,
    pub(crate) record: JointFinalRecordAssembly,
}

/// Executes the exact recovered ordinary-finite value graph through 0x6754.
///
/// One numeric backend preserves the global native callback order. The prefix
/// and middle effective-mass stages are infallible; the only callbacks after
/// angular prediction are `FinalAngular` and `FinalLinear`.
#[allow(
    clippy::large_types_passed_by_value,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one copied native call snapshot prevents mismatched body rows"
)]
pub(crate) fn build_simulation_joint_through_effective_mass<B: SimulationJointNumericBackend>(
    body_a: SimulationJointBuildBody,
    body_b: SimulationJointBuildBody,
    frames: SimulationJointFrames,
    limits: SimulationJointLimits,
    time_step: f32,
    backend: &mut B,
) -> Result<SimulationJointBuildThroughEffectiveMass, B::Error> {
    let prediction = build_simulation_joint_through_prediction(
        body_a.stage1,
        body_b.stage1,
        frames,
        limits,
        time_step,
        backend,
    )?;

    let attachment_a = JointAttachmentRows {
        full: prediction.stage1.attachment_a,
        zxy: prediction.stage1.attachment_a_zxy,
        yzx: prediction.stage1.attachment_a_yzx,
    };
    let attachment_b = JointAttachmentRows {
        full: prediction.stage1.attachment_b,
        zxy: prediction.stage1.attachment_b_zxy,
        yzx: prediction.stage1.attachment_b_yzx,
    };
    let prefix = build_joint_effective_mass_prefix(JointEffectiveMassPrefixInput {
        body_a: JointEffectiveMassBodyRows {
            center_of_mass_and_rf_word: body_a.stage1.center_of_mass_and_rf_word,
            full_inertia_and_inverse_mass: body_a.full_inertia_and_inverse_mass,
            split_inertia_and_state: body_a.split_inertia_and_state,
        },
        body_b: JointEffectiveMassBodyRows {
            center_of_mass_and_rf_word: body_b.stage1.center_of_mass_and_rf_word,
            full_inertia_and_inverse_mass: body_b.full_inertia_and_inverse_mass,
            split_inertia_and_state: body_b.split_inertia_and_state,
        },
        attachment_a,
        attachment_b,
        linear_axes: prediction.stage1.joint_axes,
        angular_axes: prediction.prediction.axes,
        position_packet_a: prediction.stage1.position_packet_a,
        position_packet_b: prediction.stage1.position_packet_b,
    });

    let inertia_a = inertia_rows(prefix.body_a_inertia);
    let inertia_b = inertia_rows(prefix.body_b_inertia);
    let [linear_right, linear_up, linear_at] = prediction.stage1.joint_axes;
    let [angular_right, angular_up, angular_at] = prediction.prediction.axes;
    let middle = build_effective_mass_middle(MiddleInput {
        inertia_a,
        inertia_b,
        linear_axes: AxisRows {
            right: linear_right,
            up: linear_up,
            at: linear_at,
        },
        angular_axes: AxisRows {
            right: angular_right,
            up: angular_up,
            at: angular_at,
        },
        cross_right_a: prefix.cross.body_a_cross_right,
        cross_right_b: prefix.cross.body_b_cross_right,
        cross_up_a: PendingCross {
            attachment_zxy: prefix.cross.body_a_attachment_zxy,
            axis_yzx: prefix.cross.up_yzx_for_body_a,
            first_product: prefix.cross.body_a_up_first,
        },
        cross_up_b: PendingCross {
            attachment_zxy: prefix.cross.body_b_attachment_zxy,
            axis_yzx: prefix.cross.up_yzx_for_body_b,
            first_product: prefix.cross.body_b_up_first,
        },
        cross_at_a: PendingCross {
            attachment_zxy: prefix.cross.body_a_attachment_zxy,
            axis_yzx: prefix.cross.at_yzx,
            first_product: prefix.cross.body_a_at_first,
        },
        cross_at_b: PendingCross {
            attachment_zxy: prefix.cross.body_b_attachment_zxy,
            axis_yzx: prefix.cross.at_yzx,
            first_product: prefix.cross.body_b_at_first,
        },
        a_tr_x_term: prefix.body_a_full_inertia_times_right_x,
        attachment_a: prefix.body_a_attachment,
        body_a_rf_word: prefix.body_a_center_of_mass_and_rf_word[3].to_bits(),
        limit_170: prediction.prediction.control_stack.packet_a,
        limit_160: prediction.prediction.control_stack.packet_b,
        time_step: [time_step; 4],
        bias_base: prediction.prediction.angular_projection,
    });

    let suffix = build_joint_effective_mass_suffix(
        JointEffectiveMassSuffixInput {
            inverse_inertia_a: inertia_array(inertia_a),
            inverse_inertia_b: inertia_array(inertia_b),
            attachment_cross_linear_a: [
                middle.crosses.a_right,
                middle.crosses.a_up,
                middle.crosses.a_at,
            ],
            attachment_cross_linear_b: [
                middle.crosses.b_right,
                middle.crosses.b_up,
                middle.crosses.b_at,
            ],
            angular_axes: prediction.prediction.axes,
            linear_axes: prediction.stage1.joint_axes,
            inertia_t_right: middle.angular.sum_right,
            inertia_t_up_a: middle.angular.a_up,
            inertia_t_up_b: middle.angular.b_up,
            inertia_t_at_a: middle.angular.a_at,
            inertia_t_at_b: middle.angular.b_at,
            inertia_cross_right_a: middle.cross_tensor.a_right,
            inertia_cross_right_b_xy: middle.cross_tensor.b_right_xy,
            inertia_cross_up_a: middle.cross_tensor.a_up,
            inertia_cross_at_a_x: middle.cross_tensor.a_at_x,
            inertia_cross_at_b_x: middle.cross_tensor.b_at_x,
            inverse_mass_sum: middle.continuation.linear_inverse_mass,
            angular_projection: prediction.prediction.angular_projection,
            angular_control_step: middle.continuation.limit_170_dt,
            angular_trial_plus: middle.continuation.bias_advanced,
            angular_packet_a: prediction.prediction.angular_packet_a,
            angular_packet_b: prediction.prediction.angular_packet_b,
            position_packet_a: prediction.stage1.position_packet_a,
            position_packet_b: prediction.stage1.position_packet_b,
        },
        backend,
    )?;

    Ok(SimulationJointBuildThroughEffectiveMass {
        prediction,
        prefix,
        middle,
        suffix,
    })
}

/// Executes the recovered Build graph and assembles its exact inert record.
///
/// Any numeric callback failure returns no build value and no record image.
#[allow(
    clippy::large_types_passed_by_value,
    clippy::too_many_arguments,
    reason = "one copied native snapshot preserves coherent binding observations"
)]
pub(crate) fn build_simulation_joint_inert_record<B: SimulationJointNumericBackend>(
    body_a: SimulationJointBuildBody,
    body_b: SimulationJointBuildBody,
    frames: SimulationJointFrames,
    limits: SimulationJointLimits,
    time_step: f32,
    inert_words: SimulationJointInertRecordWords,
    backend: &mut B,
) -> Result<SimulationJointBuildInertRecord, B::Error> {
    let build = build_simulation_joint_through_effective_mass(
        body_a, body_b, frames, limits, time_step, backend,
    )?;
    let suffix = build.suffix;
    let record_input = JointFinalRecordAssemblyInput {
        stable: JointFinalStableRows {
            attachment_a_and_rf_word: build.middle.record_writes.at_000.map(f32::to_bits),
            attachment_b_and_rf_word: build
                .prefix
                .record_writes
                .row_0x10_body_b_attachment_and_rf_word
                .map(f32::to_bits),
            linear_basis: build.prediction.stage1.joint_axes.map(bits4),
            angular_basis: build.prediction.prediction.axes.map(bits4),
            inverse_inertia_a: selected_inertia_array(build.prefix.body_a_inertia).map(bits4),
            inverse_inertia_b: selected_inertia_array(build.prefix.body_b_inertia).map(bits4),
        },
        scaled: JointFinalScaledRows {
            linear_right: suffix.linear_transpose[0],
            angular_right: suffix.angular_transpose[0],
            linear_up: suffix.linear_transpose[1],
            angular_up: suffix.angular_transpose[1],
            linear_at: suffix.linear_transpose[2],
            angular_at: suffix.angular_transpose[2],
            position_max: suffix.position_max,
            position_min: suffix.position_min,
            angular_max: suffix.angular_max,
            angular_min: suffix.angular_min,
        },
        joint_word: inert_words.joint_binding_word,
        transient_spy_word: inert_words.transient_spy_word,
    };
    let record = assemble_joint_final_record(&record_input);

    Ok(SimulationJointBuildInertRecord { build, record })
}

fn inertia_rows(
    selected: crate::simulation_joint_build_effective_mass_prefix::JointSelectedInertia,
) -> InertiaRows {
    InertiaRows {
        row0: selected.full,
        row1: selected.pack_y,
        row2: selected.pack_z,
    }
}

fn inertia_array(rows: InertiaRows) -> [SimulationJointVector; 3] {
    [rows.row0, rows.row1, rows.row2]
}

fn selected_inertia_array(
    selected: crate::simulation_joint_build_effective_mass_prefix::JointSelectedInertia,
) -> [SimulationJointVector; 3] {
    [selected.full, selected.pack_y, selected.pack_z]
}

fn bits4(row: SimulationJointVector) -> [u32; 4] {
    row.map(f32::to_bits)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::too_many_lines,
        reason = "literal fixtures freeze the complete native call boundary"
    )]

    use super::*;
    use crate::simulation_joint_numeric::{
        SimulationJointReciprocalSite, SimulationJointReciprocalSqrtSite, SimulationJointSumSite,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        Sum(SimulationJointSumSite),
        Reciprocal(SimulationJointReciprocalSite, [u32; 4]),
        ReciprocalSqrt(SimulationJointReciprocalSqrtSite),
    }

    impl Event {
        fn site(self) -> Site {
            match self {
                Self::Sum(site) => Site::Sum(site),
                Self::Reciprocal(site, _) => Site::Reciprocal(site),
                Self::ReciprocalSqrt(site) => Site::ReciprocalSqrt(site),
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
            self.events.push(Event::Sum(site));
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
            Ok(input.map(f32::recip))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            site: SimulationJointReciprocalSqrtSite,
            input: SimulationJointVector,
        ) -> Result<SimulationJointVector, Self::Error> {
            self.events.push(Event::ReciprocalSqrt(site));
            if self.fails_now() {
                return Err(Failure);
            }
            Ok(input.map(|lane| lane.sqrt().recip()))
        }
    }

    #[test]
    fn cone_arc_composition_matches_suffix_literals_and_seventeen_callbacks() {
        let (body_a, body_b, frames, limits, time_step) = fixture();
        let mut backend = HostBackend::default();
        let result = build_simulation_joint_inert_record(
            body_a,
            body_b,
            frames,
            limits,
            time_step,
            inert_words(),
            &mut backend,
        )
        .expect("host numeric backend succeeds");
        let output = result.build;

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
                Site::ReciprocalSqrt(SimulationJointReciprocalSqrtSite::Cone),
                Site::Reciprocal(SimulationJointReciprocalSite::Cone),
                Site::Reciprocal(SimulationJointReciprocalSite::TwistArc),
                Site::Sum(SimulationJointSumSite::AngularUp),
                Site::Sum(SimulationJointSumSite::AngularRight),
                Site::Sum(SimulationJointSumSite::AngularAt),
                Site::Reciprocal(SimulationJointReciprocalSite::FinalAngular),
                Site::Reciprocal(SimulationJointReciprocalSite::FinalLinear),
            ]
        );
        assert_eq!(
            backend.events[15],
            Event::Reciprocal(
                SimulationJointReciprocalSite::FinalAngular,
                [0x3fad_4451, 0x4030_72b5, 0x3ff9_8d8c, 0x407c_dc9b],
            )
        );
        assert_eq!(
            backend.events[16],
            Event::Reciprocal(
                SimulationJointReciprocalSite::FinalLinear,
                [0x408c_2a0e, 0x408d_7831, 0x408f_894b, 0x4093_e319],
            )
        );
        assert_eq!(
            output_words(output.suffix),
            [
                0x3e48_9715,
                0xbdd6_7032,
                0x3d32_281e,
                0x3bc1_a723,
                0x3dda_a69f,
                0x3e4b_5839,
                0x3bc7_85d5,
                0x3d2c_ea57,
                0xbd2b_8ca6,
                0x3c85_0d16,
                0x3e5f_9fec,
                0x3d2c_ea57,
                0x3e21_650d,
                0xbdc3_dab8,
                0x3e58_7a5e,
                0x3cd5_2596,
                0x3e47_d687,
                0x3e29_1479,
                0x3d57_f913,
                0x3dd5_a531,
                0x3e82_8306,
                0xbd89_cb20,
                0xbe2f_303b,
                0x3dd5_a531,
                0xbe97_9cb3,
                0x3f49_428d,
                0x3ebe_41ca,
                0x3d4f_07a6,
                0xbf1d_a14a,
                0x3f0f_5a7e,
                0xbcf3_4619,
                0xbd4e_9365,
                0xb8fc_3959,
                0x3e21_14a3,
                0xbaf6_9731,
                0x3990_450b,
                0x3ae2_4ed5,
                0x3e21_14a3,
                0xbb22_8324,
                0x3990_450b,
            ]
        );
        assert_eq!(
            output.prediction.prediction.axes[0].map(f32::to_bits),
            [0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe, 0x3f2d_684e]
        );
        assert_eq!(
            output
                .prefix
                .packed_body_b_attachment_and_rf_word
                .map(f32::to_bits),
            [0xbe84_f766, 0x3cf9_db21, 0x3f10_0000, 0x2222_2222]
        );
        assert_eq!(
            output.middle.continuation.limit_170.map(f32::to_bits),
            [0x3e19_999a, 0xbd8f_5c29, 0xbd8f_5c29, 0x0000_0000]
        );
        assert_eq!(
            output.middle.continuation.limit_160.map(f32::to_bits),
            [0x3e19_999a, 0xbd8f_5c29, 0xbd8f_5c29, 0x0000_0000]
        );
        assert_eq!(record_fnv(&result.record.rows), 0xd1a5_dd73_5b04_e060);
    }

    #[test]
    fn locked_locked_composition_matches_route_dependent_suffix_literal() {
        let (body_a, body_b, frames, mut limits, time_step) = fixture();
        limits.raw_swing_type = 0;
        limits.raw_twist_type = 0;
        let mut backend = HostBackend::default();
        let result = build_simulation_joint_inert_record(
            body_a,
            body_b,
            frames,
            limits,
            time_step,
            inert_words(),
            &mut backend,
        )
        .expect("host numeric backend succeeds");
        let output = result.build;

        assert_eq!(backend.events.len(), 14);
        assert_eq!(
            output_words(output.suffix),
            [
                0x3e48_9715,
                0xbdd6_7032,
                0x3d32_281e,
                0x3bc1_a723,
                0x3dda_a69f,
                0x3e4b_5839,
                0x3bc7_85d5,
                0x3d2c_ea57,
                0xbd2b_8ca6,
                0x3c85_0d16,
                0x3e5f_9fec,
                0x3d2c_ea57,
                0x3e0d_f5e3,
                0xbe32_e680,
                0x3e70_3ea6,
                0x3f44_1098,
                0x3e29_8979,
                0x3ea7_01ae,
                0x3f4f_a0b2,
                0x3e62_dd89,
                0xbd61_44e5,
                0x3df0_f66f,
                0x4083_bf2a,
                0x3e62_dd89,
                0xbe97_9cb3,
                0x3f49_428d,
                0x3ebe_41ca,
                0x3d4f_07a6,
                0xbf1d_a14a,
                0x3f0f_5a7e,
                0xbcf3_4619,
                0xbd4e_9365,
                0xbdb8_19f4,
                0x3f86_cf18,
                0xc02a_84c2,
                0xbcbf_3ebc,
                0xbdb8_19f4,
                0x3f86_cf18,
                0xc02a_84c2,
                0xbcbf_3ebc,
            ]
        );
        assert_eq!(record_fnv(&result.record.rows), 0x04da_1bfa_565b_c125);
    }

    #[test]
    fn every_numeric_failure_returns_no_value_and_retains_exact_global_prefix() {
        let (body_a, body_b, frames, limits, time_step) = fixture();
        let mut success = HostBackend::default();
        assert!(
            build_simulation_joint_inert_record(
                body_a,
                body_b,
                frames,
                limits,
                time_step,
                inert_words(),
                &mut success,
            )
            .is_ok()
        );
        assert_eq!(success.events.len(), 17);

        for fail_at in 1..=success.events.len() {
            let mut backend = HostBackend {
                events: Vec::new(),
                fail_at: Some(fail_at),
            };
            let result = build_simulation_joint_inert_record(
                body_a,
                body_b,
                frames,
                limits,
                time_step,
                inert_words(),
                &mut backend,
            );
            assert!(matches!(result, Err(Failure)));
            assert_eq!(backend.events, success.events[..fail_at]);
        }
    }

    fn fixture() -> (
        SimulationJointBuildBody,
        SimulationJointBuildBody,
        SimulationJointFrames,
        SimulationJointLimits,
        f32,
    ) {
        let body_a = SimulationJointBuildBody {
            stage1: SimulationJointBuildStage1Body {
                quaternion: [0.11, -0.07, 0.19, 0.96],
                center_of_mass_and_rf_word: [1.2, -0.8, 0.4, f32::from_bits(0x1111_1111)],
                velocity: [0.31, -0.22, 0.17, 91.0],
                omega: [0.14, 0.27, -0.19, 92.0],
                right: [1.0, 0.0, 0.0, 31.0],
                up: [0.0, 1.0, 0.0, 32.0],
                at: [0.0, 0.0, 1.0, 33.0],
                force: [0.21, -0.16, 0.12, 93.0],
                torque: [-0.17, 0.08, 0.23, 94.0],
            },
            full_inertia_and_inverse_mass: v([0x3fd9_999a, 0x3e05_1eb8, 0xbda3_d70a, 0x400c_cccd]),
            split_inertia_and_state: v([0x3db8_51ec, 0x3fb3_3333, 0x3d23_d70a, 0x0000_0004]),
        };
        let body_b = SimulationJointBuildBody {
            stage1: SimulationJointBuildStage1Body {
                quaternion: [-0.13, 0.21, 0.05, 0.95],
                center_of_mass_and_rf_word: [-1.1, 0.6, 1.3, f32::from_bits(0x2222_2222)],
                velocity: [-0.18, 0.34, -0.29, 91.0],
                omega: [-0.11, 0.16, 0.24, 92.0],
                right: [0.87, 0.29, -0.19, 41.0],
                up: [-0.24, 0.94, 0.23, 42.0],
                at: [0.25, -0.15, 0.96, 43.0],
                force: [-0.14, 0.25, -0.06, 93.0],
                torque: [0.19, -0.21, 0.1, 94.0],
            },
            full_inertia_and_inverse_mass: v([0x3f99_999a, 0xbdb8_51ec, 0x3e2e_147b, 0x3fe6_6666]),
            split_inertia_and_state: v([0xbde1_47ae, 0x3f86_6666, 0x3d8f_5c29, 0x0000_0004]),
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
            raw_swing_type: 1,
            raw_twist_type: 1,
        };
        (body_a, body_b, frames, limits, f32::from_bits(0x3c88_8889))
    }

    fn output_words(output: JointEffectiveMassSuffixOutput) -> Vec<u32> {
        let mut words = Vec::new();
        for row in output.linear_transpose {
            words.extend(row.map(f32::to_bits));
        }
        for row in output.angular_transpose {
            words.extend(row.map(f32::to_bits));
        }
        words.extend(output.position_max.map(f32::to_bits));
        words.extend(output.position_min.map(f32::to_bits));
        words.extend(output.angular_max.map(f32::to_bits));
        words.extend(output.angular_min.map(f32::to_bits));
        words
    }

    fn inert_words() -> SimulationJointInertRecordWords {
        SimulationJointInertRecordWords {
            joint_binding_word: 0x1000_1000,
            transient_spy_word: 7,
        }
    }

    fn record_fnv(rows: &[[u32; 4]; 24]) -> u64 {
        rows.iter()
            .copied()
            .flatten()
            .fold(0xcbf2_9ce4_8422_2325, |hash, word| {
                word.to_be_bytes().into_iter().fold(hash, |inner, byte| {
                    (inner ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
                })
            })
    }

    fn v(words: [u32; 4]) -> SimulationJointVector {
        words.map(f32::from_bits)
    }
}
