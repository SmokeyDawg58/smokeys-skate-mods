//! Recovered typed continuation for `JointJacobian::Build` 0x82ae6170..0x82ae6304.
//!
//! The input begins after Swing has already selected its final Right/Up/At
//! rows and angular packets. This stage preserves Twist's unsigned dispatch,
//! its lazy reciprocal/refinement path, and the common angular prediction.
//! It is ordinary-finite only: Xenon estimate/reduction details, exceptional
//! values, denormals, FPSCR state, and final record assembly remain boundaries.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "private complete Build stays internal pending the owning full Horus runtime"
    )
)]

use crate::{
    simulation_joint_numeric::{
        SimulationJointNumericBackend, SimulationJointReciprocalSite, SimulationJointSumSite,
        SimulationJointVector,
    },
    simulation_joint_pool::SimulationJointLimits,
};

type JointVector = SimulationJointVector;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct JointRqdRows {
    pub(crate) rows: [JointVector; 4],
}

/// Copied angular body values retained by Stage1.
///
/// W lanes are preserved computational values, not padding.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct JointAngularBodyValues {
    pub(crate) omega: JointVector,
    pub(crate) torque: JointVector,
}

/// Stage1 values still consumed by Twist or common angular prediction.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct JointTwistStage1Values {
    pub(crate) rqd: JointRqdRows,
    /// Physical relative Right/Up/At rows in semantic order.
    pub(crate) relative_axes: [JointVector; 3],
    pub(crate) body_a_frame_right: JointVector,
    pub(crate) body_a: JointAngularBodyValues,
    pub(crate) body_b: JointAngularBodyValues,
}

/// Values after the already-recovered Swing route and immediately before
/// Twist dispatch at 0x82ae6170.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FinalSwingPackets {
    /// Final Swing Right/Up/At rows, including their native W lanes.
    pub(crate) axes: [JointVector; 3],
    /// Physical v90 and v89. Conservative names avoid unproven min/max names.
    pub(crate) angular_packet_a: JointVector,
    pub(crate) angular_packet_b: JointVector,
}

/// The two identical control-stack packets written at 0x62c0..0x62dc.
///
/// These are carried explicitly because later effective-mass/final assembly
/// consumes them. Their W lanes are integer-zero stores interpreted as +0.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct JointAngularControlStack {
    pub(crate) packet_a: JointVector,
    pub(crate) packet_b: JointVector,
}

/// Value-only boundary after 0x82ae6304.
///
/// This is not a Jacobian record, pool slot, RF capability, publication token,
/// or implementation of the still-unmodeled 0x6308..0x67a8 final assembly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct JointPostPredictionBoundary {
    /// Final Twist Right/Up/At rows. W duplicates are preserved.
    pub(crate) axes: [JointVector; 3],
    pub(crate) angular_packet_a: JointVector,
    pub(crate) angular_packet_b: JointVector,
    pub(crate) angular_delta: JointVector,
    /// Right/Up/At with the Right scalar duplicated in W.
    pub(crate) angular_projection: JointVector,
    pub(crate) control_stack: JointAngularControlStack,
}

/// Ordinary-finite translation of 0x82ae6170..0x82ae6304.
///
/// Raw Twist dispatch is unsigned: 0=LOCKED, 1=ARC, every other u32=FREE.
/// ARC writes the body-A frame Right row even when its denominator is zero.
/// A zero denominator skips the estimate entirely. For a nonzero ARC the
/// callback order is reciprocal, `AngularUp`, `AngularRight`, `AngularAt`; all
/// other routes call only `AngularUp`, `AngularRight`, `AngularAt`.
#[allow(
    clippy::similar_names,
    reason = "separate packet names preserve two native LOCKED multiplies"
)]
pub(crate) fn build_joint_twist_prediction<B: SimulationJointNumericBackend>(
    stage1: JointTwistStage1Values,
    swing: FinalSwingPackets,
    limits: SimulationJointLimits,
    time_step: f32,
    backend: &mut B,
) -> Result<JointPostPredictionBoundary, B::Error> {
    let mut axes = swing.axes;
    let mut angular_packet_a = swing.angular_packet_a;
    let mut angular_packet_b = swing.angular_packet_b;

    match limits.raw_twist_type {
        0 => {
            // 0x6230..0x6290: Jwi is the first RQD row. Native constructs
            // [x,y,z,x] and inserts 2*Jwi.w into lane X of both packets.
            let jwi = stage1.rqd.rows[0];
            axes[0] = [jwi[0], jwi[1], jwi[2], jwi[0]];
            let packet_a_x = 2.0_f32 * jwi[3];
            let packet_b_x = 2.0_f32 * jwi[3];
            angular_packet_a[0] = packet_a_x;
            angular_packet_b[0] = packet_b_x;
        }
        1 => {
            // 0x6180..0x6184 occurs before the zero test.
            axes[0] = stage1.body_a_frame_right;

            let relative_right = stage1.relative_axes[0];
            let relative_up = stage1.relative_axes[1];
            let relative_at = stage1.relative_axes[2];
            let denominator_scalar = relative_up[2] - relative_at[1];

            // Native compares a splat against +0. IEEE ordinary-finite +0 and
            // -0 therefore both take the skip route.
            if denominator_scalar != 0.0 {
                let denominator = [denominator_scalar; 4];
                let one = [1.0_f32; 4];
                let seed = backend
                    .reciprocal_estimate(SimulationJointReciprocalSite::TwistArc, denominator)?;

                // Preserve the source interleave around the two Newton steps:
                // numerator terms at 0x61e4/61e8, first step at 0x61ec/61f0,
                // scaled limit at 0x61fc, then the second step around the
                // final numerator subtraction at 0x6204.
                let up_y_plus_at_z = relative_up[1] + relative_at[2];
                let one_plus_right_x = 1.0_f32 + relative_right[0];
                let residual_0 = vector_negative_multiply_add(seed, denominator, one);
                let refined_0 = vector_multiply_add(seed, residual_0, seed);
                let scaled_twist_c = one_plus_right_x * limits.twist_c;
                let residual_1 = vector_negative_multiply_add(refined_0, denominator, one);
                let numerator = scaled_twist_c - up_y_plus_at_z;
                let refined_1 = vector_multiply_add(refined_0, residual_1, refined_0);
                let correction = vector_multiply(refined_1, [numerator; 4]);

                // Strict `0 > denominator`: negative selects physical v89;
                // nonnegative selects physical v90. Only X is inserted.
                if 0.0_f32 > denominator_scalar {
                    angular_packet_b[0] = correction[0];
                } else {
                    angular_packet_a[0] = correction[0];
                }
            }
        }
        _ => {
            // Unsigned FREE/default route, including all unknown/high words.
        }
    }

    // 0x6294..0x62e8. Native forms B-A, then fma(B.torque,h,relative),
    // subtracts A.torque*h, and multiplies the result by h. Preserve this
    // chronology and all four lanes.
    let relative_omega = vector_sub(stage1.body_b.omega, stage1.body_a.omega);
    let body_a_torque_step = vector_multiply(stage1.body_a.torque, [time_step; 4]);
    let body_b_added = vector_multiply_add(stage1.body_b.torque, [time_step; 4], relative_omega);
    let torque_adjusted = vector_sub(body_b_added, body_a_torque_step);
    let angular_delta = vector_multiply(torque_adjusted, [time_step; 4]);

    // Exact native reduction chronology is Up, Right, At.
    let angular_up = call_sum(
        backend,
        SimulationJointSumSite::AngularUp,
        axes[1],
        angular_delta,
    )?;
    let angular_right = call_sum(
        backend,
        SimulationJointSumSite::AngularRight,
        axes[0],
        angular_delta,
    )?;
    let angular_at = call_sum(
        backend,
        SimulationJointSumSite::AngularAt,
        axes[2],
        angular_delta,
    )?;
    let angular_projection = [angular_right, angular_up, angular_at, angular_right];

    // 0x6300 then 0x6304. Both additions are complete four-lane operations.
    angular_packet_a = vector_add(angular_packet_a, angular_projection);
    angular_packet_b = vector_add(angular_packet_b, angular_projection);

    let control_packet = [limits.v_twist, limits.v_swing, limits.v_swing, 0.0];
    Ok(JointPostPredictionBoundary {
        axes,
        angular_packet_a,
        angular_packet_b,
        angular_delta,
        angular_projection,
        control_stack: JointAngularControlStack {
            packet_a: control_packet,
            packet_b: control_packet,
        },
    })
}

fn call_sum<B: SimulationJointNumericBackend>(
    backend: &mut B,
    site: SimulationJointSumSite,
    left: JointVector,
    right: JointVector,
) -> Result<f32, B::Error> {
    backend.sum3(
        site,
        [left[0], left[1], left[2]],
        [right[0], right[1], right[2]],
    )
}

fn vector_add(a: JointVector, b: JointVector) -> JointVector {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
}

fn vector_sub(a: JointVector, b: JointVector) -> JointVector {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
}

fn vector_multiply(a: JointVector, b: JointVector) -> JointVector {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2], a[3] * b[3]]
}

fn vector_multiply_add(a: JointVector, b: JointVector, addend: JointVector) -> JointVector {
    [
        a[0].mul_add(b[0], addend[0]),
        a[1].mul_add(b[1], addend[1]),
        a[2].mul_add(b[2], addend[2]),
        a[3].mul_add(b[3], addend[3]),
    ]
}

fn vector_negative_multiply_add(
    a: JointVector,
    b: JointVector,
    addend: JointVector,
) -> JointVector {
    [
        (-a[0]).mul_add(b[0], addend[0]),
        (-a[1]).mul_add(b[1], addend[1]),
        (-a[2]).mul_add(b[2], addend[2]),
        (-a[3]).mul_add(b[3], addend[3]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of, size_of};

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        Reciprocal(SimulationJointReciprocalSite, [u32; 4]),
        Sum(SimulationJointSumSite, [u32; 3], [u32; 3]),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Failure;

    #[derive(Default)]
    struct OracleBackend {
        events: Vec<Event>,
        fail_at: Option<usize>,
    }

    impl OracleBackend {
        fn should_fail(&self) -> bool {
            self.fail_at == Some(self.events.len())
        }
    }

    impl SimulationJointNumericBackend for OracleBackend {
        type Error = Failure;

        fn reciprocal_estimate(
            &mut self,
            site: SimulationJointReciprocalSite,
            denominator: JointVector,
        ) -> Result<JointVector, Self::Error> {
            self.events
                .push(Event::Reciprocal(site, bits4(denominator)));
            if self.should_fail() {
                return Err(Failure);
            }
            Ok(denominator.map(|x| 1.0_f32 / x))
        }

        fn sum3(
            &mut self,
            site: SimulationJointSumSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.events
                .push(Event::Sum(site, bits3(left), bits3(right)));
            if self.should_fail() {
                return Err(Failure);
            }
            Ok(left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2])))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            _site: crate::simulation_joint_numeric::SimulationJointReciprocalSqrtSite,
            _input: JointVector,
        ) -> Result<JointVector, Self::Error> {
            unreachable!("the Twist/prediction stage has no reciprocal-square-root site")
        }
    }

    fn f(bits: u32) -> f32 {
        f32::from_bits(bits)
    }

    fn v(bits: [u32; 4]) -> JointVector {
        bits.map(f)
    }

    fn bits3(value: [f32; 3]) -> [u32; 3] {
        value.map(f32::to_bits)
    }

    fn bits4(value: JointVector) -> [u32; 4] {
        value.map(f32::to_bits)
    }

    fn fixture_limits(raw_twist_type: u32) -> SimulationJointLimits {
        SimulationJointLimits {
            p_prism: v([0xbf33_3333, 0xbf00_0000, 0xbf66_6666, 0x4150_0000]),
            v_prism: v([0x3f4c_cccd, 0x3f26_6666, 0x3f8c_cccd, 0xc160_0000]),
            v_twist: f(0x3e19_999a),
            v_swing: f(0xbd8f_5c29),
            swing_a: f(0x3f38_51ec),
            twist_a: f(0x3f21_47ae),
            swing_c: f(0x3ebd_70a4),
            twist_c: f(0xbe75_c28f),
            raw_swing_type: 2,
            raw_twist_type,
        }
    }

    fn fixture_stage1() -> JointTwistStage1Values {
        JointTwistStage1Values {
            rqd: JointRqdRows {
                rows: [
                    v([0x3f12_4c79, 0x3f2e_b7cc, 0xbe68_2739, 0xbe3b_0a82]),
                    v([0xbe80_1e13, 0x3eef_331f, 0x3e2c_900a, 0x3f40_ea09]),
                    v([0x3f2e_b7cc, 0x3eb4_7ecb, 0x3f5f_afb9, 0xbe92_376a]),
                    v([0x3f32_5e58, 0xbecf_006e, 0x3e3c_71ed, 0x3ee1_b86d]),
                ],
            },
            relative_axes: [
                v([0xbe99_02cd, 0xbf06_ef75, 0xbf0f_638c, 0xbe4f_5346]),
                v([0xbcc0_6d60, 0x3f45_2954, 0xbf17_69fa, 0xbcc0_6d60]),
                v([0x3f44_cdca, 0xbe89_e8f8, 0xbe4f_5346, 0xbf06_ef75]),
            ],
            body_a_frame_right: v([0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe, 0x3f2d_684e]),
            body_a: JointAngularBodyValues {
                omega: v([0x3e0f_5c29, 0x3e8a_3d71, 0xbe42_8f5c, 0x42b8_0000]),
                torque: v([0xbe2e_147b, 0x3da3_d70a, 0x3e6b_851f, 0x42bc_0000]),
            },
            body_b: JointAngularBodyValues {
                omega: v([0xbde1_47ae, 0x3e23_d70a, 0x3e75_c28f, 0x42b8_0000]),
                torque: v([0x3e42_8f5c, 0xbe57_0a3d, 0x3dcc_cccd, 0x42bc_0000]),
            },
        }
    }

    fn nonzero_swing() -> FinalSwingPackets {
        FinalSwingPackets {
            axes: [
                v([0x3f02_5d30, 0xbd1c_6020, 0xbf51_4793, 0x3ec6_362b]),
                v([0xbf02_4f5d, 0x3f49_0560, 0xbe9c_5bb6, 0xbf02_4f5d]),
                v([0xbf34_07e5, 0xbe62_4612, 0x3f1a_9fc6, 0xb254_26a8]),
            ],
            angular_packet_a: v([0xff7f_ffff, 0x3f5e_ba5a, 0x3cc0_6d60, 0xff7f_ffff]),
            angular_packet_b: v([0x7f7f_ffff, 0x7f7f_ffff, 0x3cc0_6d60, 0x7f7f_ffff]),
        }
    }

    fn locked_swing() -> FinalSwingPackets {
        let mut swing = nonzero_swing();
        swing.axes[2] = v([0x3f2e_b7cc, 0x3eb4_7ecb, 0x3f5f_afb9, 0x3f2e_b7cc]);
        swing.angular_packet_a[2] = f(0xbf12_376a);
        swing.angular_packet_b[2] = f(0xbf12_376a);
        swing
    }

    fn build(
        raw_twist_type: u32,
        swing: FinalSwingPackets,
    ) -> (JointPostPredictionBoundary, Vec<Event>) {
        let mut backend = OracleBackend::default();
        let output = build_joint_twist_prediction(
            fixture_stage1(),
            swing,
            fixture_limits(raw_twist_type),
            f(0x3c88_8889),
            &mut backend,
        )
        .expect("oracle backend succeeds");
        (output, backend.events)
    }

    fn output_words(output: JointPostPredictionBoundary) -> Vec<u32> {
        let mut words = Vec::new();
        for row in output.axes {
            words.extend(bits4(row));
        }
        words.extend(bits4(output.angular_packet_a));
        words.extend(bits4(output.angular_packet_b));
        words.extend(bits4(output.angular_delta));
        words.extend(bits4(output.angular_projection));
        words.extend(bits4(output.control_stack.packet_a));
        words.extend(bits4(output.control_stack.packet_b));
        words
    }

    #[test]
    fn exact_joint_limits_layout_and_discriminants() {
        assert_eq!(size_of::<SimulationJointLimits>(), 0x40);
        assert_eq!(align_of::<SimulationJointLimits>(), 0x10);
        assert_eq!(offset_of!(SimulationJointLimits, p_prism), 0x00);
        assert_eq!(offset_of!(SimulationJointLimits, v_prism), 0x10);
        assert_eq!(offset_of!(SimulationJointLimits, v_twist), 0x20);
        assert_eq!(offset_of!(SimulationJointLimits, v_swing), 0x24);
        assert_eq!(offset_of!(SimulationJointLimits, swing_a), 0x28);
        assert_eq!(offset_of!(SimulationJointLimits, twist_a), 0x2c);
        assert_eq!(offset_of!(SimulationJointLimits, swing_c), 0x30);
        assert_eq!(offset_of!(SimulationJointLimits, twist_c), 0x34);
        assert_eq!(offset_of!(SimulationJointLimits, raw_swing_type), 0x38);
        assert_eq!(offset_of!(SimulationJointLimits, raw_twist_type), 0x3c);

        let (free, _) = build(2, nonzero_swing());
        for raw in [3, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
            assert_eq!(build(raw, nonzero_swing()).0, free);
        }
    }

    #[test]
    fn locked_route_matches_complete_corrected_literal() {
        let (output, events) = build(0, locked_swing());
        assert_eq!(
            output_words(output),
            [
                0x3f12_4c79,
                0x3f2e_b7cc,
                0xbe68_2739,
                0x3f12_4c79,
                0xbf02_4f5d,
                0x3f49_0560,
                0xbe9c_5bb6,
                0xbf02_4f5d,
                0x3f2e_b7cc,
                0x3eb4_7ecb,
                0x3f5f_afb9,
                0x3f2e_b7cc,
                0xbebd_ba38,
                0x3f5e_50d0,
                0xbf11_8134,
                0xff7f_ffff,
                0xbebd_ba38,
                0x7f7f_ffff,
                0xbf11_8134,
                0x7f7f_ffff,
                0xbb85_41ad,
                0xbafa_db77,
                0x3be9_a771,
                0x0000_0000,
                0xbbab_ed77,
                0xbad3_1468,
                0x3b36_3639,
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
        assert!(matches!(
            events.as_slice(),
            [
                Event::Sum(SimulationJointSumSite::AngularUp, _, _),
                Event::Sum(SimulationJointSumSite::AngularRight, _, _),
                Event::Sum(SimulationJointSumSite::AngularAt, _, _)
            ]
        ));
    }

    #[test]
    fn arc_route_matches_complete_corrected_literal_and_callback_operands() {
        let (output, events) = build(1, nonzero_swing());
        assert_eq!(
            output_words(output),
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
        assert_eq!(events.len(), 4);
        assert_eq!(
            events[0],
            Event::Reciprocal(SimulationJointReciprocalSite::TwistArc, [0xbea4_eafc; 4],)
        );
        let delta = [0xbb85_41ad, 0xbafa_db77, 0x3be9_a771];
        assert_eq!(
            events[1],
            Event::Sum(
                SimulationJointSumSite::AngularUp,
                [0xbf02_4f5d, 0x3f49_0560, 0xbe9c_5bb6],
                delta,
            )
        );
        assert_eq!(
            events[2],
            Event::Sum(
                SimulationJointSumSite::AngularRight,
                [0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe],
                delta,
            )
        );
        assert_eq!(
            events[3],
            Event::Sum(
                SimulationJointSumSite::AngularAt,
                [0xbf34_07e5, 0xbe62_4612, 0x3f1a_9fc6],
                delta,
            )
        );
    }

    #[test]
    fn free_route_matches_complete_corrected_literal() {
        let (output, events) = build(2, nonzero_swing());
        assert_eq!(
            output_words(output),
            [
                0x3f02_5d30,
                0xbd1c_6020,
                0xbf51_4793,
                0x3ec6_362b,
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
                0x7f7f_ffff,
                0x7f7f_ffff,
                0x3cfe_9a00,
                0x7f7f_ffff,
                0xbb85_41ad,
                0xbafa_db77,
                0x3be9_a771,
                0x0000_0000,
                0xbc00_3cf9,
                0xbad3_1468,
                0x3bf8_b27f,
                0xbc00_3cf9,
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
        assert!(matches!(
            events.as_slice(),
            [
                Event::Sum(SimulationJointSumSite::AngularUp, _, _),
                Event::Sum(SimulationJointSumSite::AngularRight, _, _),
                Event::Sum(SimulationJointSumSite::AngularAt, _, _)
            ]
        ));
    }

    #[test]
    fn arc_zero_skips_reciprocal_but_still_replaces_right_axis() {
        for zero_bits in [0x0000_0000, 0x8000_0000] {
            let mut stage1 = fixture_stage1();
            stage1.relative_axes[1][2] = f(zero_bits);
            stage1.relative_axes[2][1] = 0.0;
            let mut backend = OracleBackend::default();
            let output = build_joint_twist_prediction(
                stage1,
                nonzero_swing(),
                fixture_limits(1),
                f(0x3c88_8889),
                &mut backend,
            )
            .expect("zero ARC succeeds without estimate");
            assert_eq!(bits4(output.axes[0]), bits4(stage1.body_a_frame_right));
            assert!(
                backend
                    .events
                    .iter()
                    .all(|event| matches!(event, Event::Sum(_, _, _)))
            );
            assert_eq!(backend.events.len(), 3);
        }
    }

    #[test]
    fn arc_positive_denominator_selects_packet_a_only() {
        let mut stage1 = fixture_stage1();
        stage1.relative_axes[1][2] = 1.0;
        stage1.relative_axes[2][1] = 0.25;
        let input = nonzero_swing();
        let mut backend = OracleBackend::default();
        let output = build_joint_twist_prediction(
            stage1,
            input,
            fixture_limits(1),
            f(0x3c88_8889),
            &mut backend,
        )
        .expect("positive ARC succeeds");
        assert_ne!(
            output.angular_packet_a[0].to_bits(),
            input.angular_packet_a[0].to_bits()
        );
        assert_eq!(
            output.angular_packet_b[0].to_bits(),
            (input.angular_packet_b[0] + output.angular_projection[0]).to_bits()
        );
    }

    #[test]
    fn callback_failures_retain_exact_prefix_and_return_no_output() {
        for fail_at in 1..=4 {
            let mut backend = OracleBackend {
                events: Vec::new(),
                fail_at: Some(fail_at),
            };
            let result = build_joint_twist_prediction(
                fixture_stage1(),
                nonzero_swing(),
                fixture_limits(1),
                f(0x3c88_8889),
                &mut backend,
            );
            assert_eq!(result, Err(Failure));
            assert_eq!(backend.events.len(), fail_at);
        }

        for fail_at in 1..=3 {
            let mut backend = OracleBackend {
                events: Vec::new(),
                fail_at: Some(fail_at),
            };
            let result = build_joint_twist_prediction(
                fixture_stage1(),
                nonzero_swing(),
                fixture_limits(2),
                f(0x3c88_8889),
                &mut backend,
            );
            assert_eq!(result, Err(Failure));
            assert_eq!(backend.events.len(), fail_at);
            assert!(
                backend
                    .events
                    .iter()
                    .all(|event| matches!(event, Event::Sum(_, _, _)))
            );
        }
    }
}
