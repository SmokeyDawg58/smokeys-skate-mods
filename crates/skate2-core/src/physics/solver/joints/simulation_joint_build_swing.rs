//! Recovered typed continuation for `JointJacobian::Build` 0x82ae5dd0..0x82ae6170.
//!
//! This preserves the proven ordinary-finite Swing value graph without a PPC
//! interpreter or numbered-register interface. Native estimate seeds, VMX
//! exceptional-value comparisons, denormals, FPSCR, and exception behavior
//! remain explicit backend/hardware boundaries.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "private complete Build stays internal pending the owning full Horus runtime"
    )
)]

use crate::simulation_joint_numeric::{
    SimulationJointNumericBackend, SimulationJointReciprocalSite,
    SimulationJointReciprocalSqrtSite, SimulationJointVector,
};

type Vec4 = SimulationJointVector;

/// Meaningful values live at the Stage-1 checkpoint immediately after 0x5dcc.
///
/// RQD rows and basis rows are values, not native-compatible Rust layouts.
#[derive(Clone, Copy)]
pub(crate) struct JointSwingStageInput {
    pub(crate) raw_swing_type: u32,
    pub(crate) raw_twist_type: u32,
    pub(crate) swing_c: f32,
    pub(crate) rqd_jwj: Vec4,
    pub(crate) rqd_jwk: Vec4,
    pub(crate) relative_right: Vec4,
    pub(crate) relative_up: Vec4,
    pub(crate) relative_at: Vec4,
    pub(crate) body_a_frame_right: Vec4,
    pub(crate) t_right: Vec4,
    pub(crate) t_up: Vec4,
    pub(crate) t_at: Vec4,
    /// Physical v90, initialized by Stage 1 to four `0xff7fffff` words.
    pub(crate) angular_packet_a: Vec4,
    /// Physical v89, initialized by Stage 1 to four `0x7f7fffff` words.
    pub(crate) angular_packet_b: Vec4,
}

/// Values live at the 0x6170 Swing-to-Twist join.
#[derive(Clone, Copy)]
pub(crate) struct JointSwingStageOutput {
    pub(crate) t_right: Vec4,
    pub(crate) t_up: Vec4,
    pub(crate) t_at: Vec4,
    /// Physical v90; the later stages establish final field ownership.
    pub(crate) angular_packet_a: Vec4,
    /// Physical v89; the later stages establish final field ownership.
    pub(crate) angular_packet_b: Vec4,
}

/// Typed ordinary-finite translation of the Swing dispatch/value slice.
///
/// Raw Swing words 0, 1, 2 and 3 select LOCKED, CONE, HINGE and AXLE.
/// Every unsigned value greater than 3 selects the unchanged free/default
/// route. HINGE and AXLE test raw Twist for exact zero, not enum validity.
pub(crate) fn apply_joint_swing_stage<B: SimulationJointNumericBackend>(
    input: JointSwingStageInput,
    backend: &mut B,
) -> Result<JointSwingStageOutput, B::Error> {
    let mut output = JointSwingStageOutput {
        t_right: input.t_right,
        t_up: input.t_up,
        t_at: input.t_at,
        angular_packet_a: input.angular_packet_a,
        angular_packet_b: input.angular_packet_b,
    };

    if input.raw_swing_type > 3 {
        return Ok(output);
    }

    match input.raw_swing_type {
        0 => apply_locked(input, &mut output),
        1 => apply_cone(input, &mut output, backend)?,
        2 => apply_hinge(input, &mut output, backend)?,
        3 => apply_axle(input, &mut output),
        _ => unreachable!("unsigned range gate admits only Swing words 0 through 3"),
    }
    Ok(output)
}

#[allow(
    clippy::similar_names,
    reason = "packet/row names preserve the four distinct native operations"
)]
fn apply_locked(input: JointSwingStageInput, output: &mut JointSwingStageOutput) {
    let two = [2.0; 4];
    output.t_up = pack_xyz_duplicate_x(input.rqd_jwj);
    output.t_at = pack_xyz_duplicate_x(input.rqd_jwk);

    let packet_a_jwj = vector_mul(two, [input.rqd_jwj[3]; 4]);
    let packet_a_jwk = vector_mul(two, [input.rqd_jwk[3]; 4]);
    output.angular_packet_a[1] = packet_a_jwj[1];
    let packet_b_jwj = vector_mul(two, [input.rqd_jwj[3]; 4]);
    let packet_b_jwk = vector_mul(two, [input.rqd_jwk[3]; 4]);
    output.angular_packet_a[2] = packet_a_jwk[2];
    output.angular_packet_b[1] = packet_b_jwj[1];
    output.angular_packet_b[2] = packet_b_jwk[2];
}

fn apply_cone<B: SimulationJointNumericBackend>(
    input: JointSwingStageInput,
    output: &mut JointSwingStageOutput,
    backend: &mut B,
) -> Result<(), B::Error> {
    const ALMOST_ONE: f32 = f32::from_bits(0x3f7f_ff58);
    let correlation = input.relative_right[0];
    if correlation >= ALMOST_ONE {
        return Ok(());
    }

    let relative_at_x = [input.relative_at[0]; 4];
    let one = [1.0; 4];
    let relative_up_x = [input.relative_up[0]; 4];
    let zero = [0.0; 4];

    // Source interleaves these independent preparations before the first
    // estimate. Their dependency graphs retain the raw multiply/FMA order.
    let right_yzx = yzxw(output.t_right);
    let at_square = vector_mul(relative_at_x, relative_at_x);
    let body_a_right_yzx = yzxw(input.body_a_frame_right);
    let delta = input.swing_c - correlation;
    let delta_splat = [delta; 4];
    let axis_product = vector_mul(input.body_a_frame_right, right_yzx);
    let denominator = vector_madd(relative_up_x, relative_up_x, at_square);
    let axis_numerator = vector_nmsub(body_a_right_yzx, output.t_right, axis_product);

    // Native calls vrsqrtefp before forming the exact-zero select mask.
    let rsqrt_seed =
        backend.reciprocal_sqrt_estimate(SimulationJointReciprocalSqrtSite::Cone, denominator)?;
    let zero_mask = denominator.map(|lane| lane == 0.0);

    let rsqrt_square_0 = vector_mul(rsqrt_seed, rsqrt_seed);
    let rsqrt_half_0 = vector_mul(rsqrt_seed, [0.5; 4]);
    let rsqrt_residual_0 = vector_nmsub(denominator, rsqrt_square_0, one);
    let rsqrt_1 = vector_madd(rsqrt_half_0, rsqrt_residual_0, rsqrt_seed);

    let rsqrt_square_1 = vector_mul(rsqrt_1, rsqrt_1);
    let rsqrt_half_1 = vector_mul(rsqrt_1, [0.5; 4]);
    let rsqrt_residual_1 = vector_nmsub(denominator, rsqrt_square_1, one);
    let rsqrt_2 = vector_madd(rsqrt_half_1, rsqrt_residual_1, rsqrt_1);

    let root_unselected = vector_mul(denominator, rsqrt_2);
    let root = std::array::from_fn(|lane| {
        if zero_mask[lane] {
            zero[lane]
        } else {
            root_unselected[lane]
        }
    });

    let reciprocal_seed = backend.reciprocal_estimate(SimulationJointReciprocalSite::Cone, root)?;
    let reciprocal_residual_0 = vector_nmsub(reciprocal_seed, root, one);
    let reciprocal_1 = vector_madd(reciprocal_seed, reciprocal_residual_0, reciprocal_seed);
    let reciprocal_residual_1 = vector_nmsub(reciprocal_1, root, one);
    let reciprocal_2 = vector_madd(reciprocal_1, reciprocal_residual_1, reciprocal_1);

    // The multiply by one is an explicit native operation at 0x5f84.
    let normalization = vector_mul(reciprocal_2, one);
    output.t_up = vector_mul(yzxw(axis_numerator), normalization);
    let angular_correction = vector_mul(delta_splat, normalization);
    output.angular_packet_a[1] = angular_correction[1];

    let t_at_product = vector_mul(output.t_up, body_a_right_yzx);
    let t_at_numerator = vector_nmsub(yzxw(output.t_up), input.body_a_frame_right, t_at_product);
    output.t_at = yzxw(t_at_numerator);
    Ok(())
}

fn apply_hinge<B: SimulationJointNumericBackend>(
    input: JointSwingStageInput,
    output: &mut JointSwingStageOutput,
    backend: &mut B,
) -> Result<(), B::Error> {
    apply_hinge_or_axle_twist_subroute(input, output, false);

    let denominator = [input.relative_at[0]; 4];
    // Both +0 and -0 satisfy the native vector floating equality.
    if denominator.iter().all(|lane| *lane == 0.0) {
        return Ok(());
    }

    let reciprocal_seed =
        backend.reciprocal_estimate(SimulationJointReciprocalSite::Hinge, denominator)?;
    let denominator_copy_a = denominator;
    let one = [1.0; 4];
    let denominator_copy_b = denominator;
    let swing_c = input.swing_c;
    let relative_right_x = input.relative_right[0];
    let negative_side = [0.0; 4]
        .iter()
        .zip(denominator.iter())
        .all(|(zero, lane)| zero > lane);
    let delta = swing_c - relative_right_x;
    let delta_splat = [delta; 4];

    let reciprocal_residual_0 = vector_nmsub(reciprocal_seed, denominator_copy_a, one);
    let reciprocal_1 = vector_madd(reciprocal_seed, reciprocal_residual_0, reciprocal_seed);
    let reciprocal_residual_1 = vector_nmsub(reciprocal_1, denominator_copy_b, one);
    let reciprocal_2 = vector_madd(reciprocal_1, reciprocal_residual_1, reciprocal_1);
    let correction = vector_mul(reciprocal_2, delta_splat);

    if negative_side {
        output.angular_packet_b[1] = correction[1];
    } else {
        output.angular_packet_a[1] = correction[1];
    }
    Ok(())
}

fn apply_axle(input: JointSwingStageInput, output: &mut JointSwingStageOutput) {
    apply_hinge_or_axle_twist_subroute(input, output, true);
}

#[allow(
    clippy::similar_names,
    reason = "packet names preserve native write and multiply order"
)]
fn apply_hinge_or_axle_twist_subroute(
    input: JointSwingStageInput,
    output: &mut JointSwingStageOutput,
    axle_zero_packet_b_first: bool,
) {
    if input.raw_twist_type == 0 {
        output.t_at = pack_xyz_duplicate_x(input.rqd_jwk);
        // HINGE writes physical v90 then v89 at 0x6010/0x6014. AXLE
        // writes v89 then v90 at 0x6134/0x6138.
        if axle_zero_packet_b_first {
            let packet_b_jwk = vector_mul([2.0; 4], [input.rqd_jwk[3]; 4]);
            let packet_a_jwk = vector_mul([2.0; 4], [input.rqd_jwk[3]; 4]);
            output.angular_packet_b[2] = packet_b_jwk[2];
            output.angular_packet_a[2] = packet_a_jwk[2];
        } else {
            let packet_a_jwk = vector_mul([2.0; 4], [input.rqd_jwk[3]; 4]);
            let packet_b_jwk = vector_mul([2.0; 4], [input.rqd_jwk[3]; 4]);
            output.angular_packet_a[2] = packet_a_jwk[2];
            output.angular_packet_b[2] = packet_b_jwk[2];
        }
        return;
    }

    let up_yzx = yzxw(output.t_up);
    let sign_mask_a = [f32::from_bits(0x8000_0000); 4];
    let body_a_right_yzx = yzxw(input.body_a_frame_right);
    let sign_mask_b = [f32::from_bits(0x8000_0000); 4];
    let relative_up_x = [input.relative_up[0]; 4];
    let axis_product = vector_mul(input.body_a_frame_right, up_yzx);
    let negative_relative_up_a = vector_xor(relative_up_x, sign_mask_a);
    let negative_relative_up_b = vector_xor(relative_up_x, sign_mask_b);
    output.angular_packet_a[2] = negative_relative_up_a[2];
    output.angular_packet_b[2] = negative_relative_up_b[2];
    let axis_numerator = vector_nmsub(body_a_right_yzx, output.t_up, axis_product);
    output.t_at = yzxw(axis_numerator);
}

fn pack_xyz_duplicate_x(value: Vec4) -> Vec4 {
    [value[0], value[1], value[2], value[0]]
}

fn vector_mul(left: Vec4, right: Vec4) -> Vec4 {
    std::array::from_fn(|lane| left[lane] * right[lane])
}

fn vector_xor(left: Vec4, right: Vec4) -> Vec4 {
    std::array::from_fn(|lane| f32::from_bits(left[lane].to_bits() ^ right[lane].to_bits()))
}

fn vector_madd(multiplier: Vec4, multiplicand: Vec4, addend: Vec4) -> Vec4 {
    std::array::from_fn(|lane| multiplier[lane].mul_add(multiplicand[lane], addend[lane]))
}

/// Source `vnmsubfp`: `addend - multiplier*multiplicand` in one fused step.
fn vector_nmsub(multiplier: Vec4, multiplicand: Vec4, addend: Vec4) -> Vec4 {
    std::array::from_fn(|lane| (-multiplier[lane]).mul_add(multiplicand[lane], addend[lane]))
}

const fn yzxw(value: Vec4) -> Vec4 {
    [value[1], value[2], value[0], value[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum JointSwingEstimateSite {
        ConeReciprocalSqrt,
        ConeReciprocal,
        HingeReciprocal,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum InjectedError {
        Estimate,
    }

    #[derive(Default)]
    struct HostBackend {
        trace: Vec<(JointSwingEstimateSite, [u32; 4], [u32; 4])>,
    }

    impl SimulationJointNumericBackend for HostBackend {
        type Error = InjectedError;

        fn reciprocal_sqrt_estimate(
            &mut self,
            site: SimulationJointReciprocalSqrtSite,
            input: Vec4,
        ) -> Result<Vec4, Self::Error> {
            let output = input.map(|lane| lane.sqrt().recip());
            assert_eq!(site, SimulationJointReciprocalSqrtSite::Cone);
            self.trace.push((
                JointSwingEstimateSite::ConeReciprocalSqrt,
                bits4(input),
                bits4(output),
            ));
            Ok(output)
        }

        fn reciprocal_estimate(
            &mut self,
            site: SimulationJointReciprocalSite,
            input: Vec4,
        ) -> Result<Vec4, Self::Error> {
            let output = input.map(f32::recip);
            let site = match site {
                SimulationJointReciprocalSite::Cone => JointSwingEstimateSite::ConeReciprocal,
                SimulationJointReciprocalSite::Hinge => JointSwingEstimateSite::HingeReciprocal,
                _ => unreachable!("the Swing stage has no other reciprocal site"),
            };
            self.trace.push((site, bits4(input), bits4(output)));
            Ok(output)
        }

        fn sum3(
            &mut self,
            _site: crate::simulation_joint_numeric::SimulationJointSumSite,
            _left: [f32; 3],
            _right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            unreachable!("the Swing stage has no sum site")
        }
    }

    struct FailingBackend {
        fail_at: JointSwingEstimateSite,
        trace: Vec<(JointSwingEstimateSite, [u32; 4])>,
    }

    impl SimulationJointNumericBackend for FailingBackend {
        type Error = InjectedError;

        fn reciprocal_sqrt_estimate(
            &mut self,
            site: SimulationJointReciprocalSqrtSite,
            input: Vec4,
        ) -> Result<Vec4, Self::Error> {
            assert_eq!(site, SimulationJointReciprocalSqrtSite::Cone);
            let site = JointSwingEstimateSite::ConeReciprocalSqrt;
            self.trace.push((site, bits4(input)));
            if site == self.fail_at {
                return Err(InjectedError::Estimate);
            }
            Ok(input.map(|lane| lane.sqrt().recip()))
        }

        fn reciprocal_estimate(
            &mut self,
            site: SimulationJointReciprocalSite,
            input: Vec4,
        ) -> Result<Vec4, Self::Error> {
            let site = match site {
                SimulationJointReciprocalSite::Cone => JointSwingEstimateSite::ConeReciprocal,
                SimulationJointReciprocalSite::Hinge => JointSwingEstimateSite::HingeReciprocal,
                _ => unreachable!("the Swing stage has no other reciprocal site"),
            };
            self.trace.push((site, bits4(input)));
            if site == self.fail_at {
                return Err(InjectedError::Estimate);
            }
            Ok(input.map(f32::recip))
        }

        fn sum3(
            &mut self,
            _site: crate::simulation_joint_numeric::SimulationJointSumSite,
            _left: [f32; 3],
            _right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            unreachable!("the Swing stage has no sum site")
        }
    }

    #[test]
    fn canonical_routes_match_literal_join_vectors_and_hashes() {
        let cases = [
            (0, 0, 0x2e6f_e525_b252_b98f),
            (1, 1, 0xc7a2_30c6_2a94_df20),
            (2, 0, 0x027d_c274_615a_d059),
            (2, 1, 0xb3b9_b8f2_c212_dab5),
            (3, 0, 0xdc84_119d_b3d7_cc5c),
            (3, 1, 0x98a0_ed8c_6fd4_7d78),
            (4, 2, 0xe5d3_5ace_f00d_061f),
            (0x8000_0000, 0xffff_ffff, 0xe5d3_5ace_f00d_061f),
        ];

        for (swing, twist, expected_hash) in cases {
            let mut input = fixture();
            input.raw_swing_type = swing;
            input.raw_twist_type = twist;
            let mut backend = HostBackend::default();
            let got = apply_joint_swing_stage(input, &mut backend).expect("host estimates succeed");
            assert_eq!(
                join_hash(got),
                expected_hash,
                "route {swing:08x}/{twist:08x}"
            );
        }

        assert_output(
            run(0, 0),
            [0x3f02_5d30, 0xbd1c_6020, 0xbf51_4793, 0x3ec6_362b],
            [0xbe80_1e13, 0x3eef_331f, 0x3e2c_900a, 0xbe80_1e13],
            [0x3f2e_b7cc, 0x3eb4_7ecb, 0x3f5f_afb9, 0x3f2e_b7cc],
            [0xff7f_ffff, 0x3fc0_ea09, 0xbf12_376a, 0xff7f_ffff],
            [0x7f7f_ffff, 0x3fc0_ea09, 0xbf12_376a, 0x7f7f_ffff],
        );
        assert_output(
            run(1, 1),
            [0x3f02_5d30, 0xbd1c_6020, 0xbf51_4793, 0x3ec6_362b],
            [0xbf06_fe20, 0x3f69_13ac, 0xbebd_f2c7, 0xb298_160b],
            [0x3f53_06b7, 0x3e52_88ad, 0xbf2a_c6c6, 0x25e8_bd68],
            [0xff7f_ffff, 0x3f5e_9fc1, 0xff7f_ffff, 0xff7f_ffff],
            [0x7f7f_ffff; 4],
        );
        assert_output(
            run(2, 0),
            [0x3f02_5d30, 0xbd1c_6020, 0xbf51_4793, 0x3ec6_362b],
            [0xbf02_4f5d, 0x3f49_0560, 0xbe9c_5bb6, 0xbf02_4f5d],
            [0x3f2e_b7cc, 0x3eb4_7ecb, 0x3f5f_afb9, 0x3f2e_b7cc],
            [0xff7f_ffff, 0x3f5e_ba5a, 0xbf12_376a, 0xff7f_ffff],
            [0x7f7f_ffff, 0x7f7f_ffff, 0xbf12_376a, 0x7f7f_ffff],
        );
        assert_output(
            run(2, 1),
            [0x3f02_5d30, 0xbd1c_6020, 0xbf51_4793, 0x3ec6_362b],
            [0xbf02_4f5d, 0x3f49_0560, 0xbe9c_5bb6, 0xbf02_4f5d],
            [0xbf34_07e5, 0xbe62_4612, 0x3f1a_9fc6, 0xb254_26a8],
            [0xff7f_ffff, 0x3f5e_ba5a, 0x3cc0_6d60, 0xff7f_ffff],
            [0x7f7f_ffff, 0x7f7f_ffff, 0x3cc0_6d60, 0x7f7f_ffff],
        );
        assert_output(
            run(3, 0),
            [0x3f02_5d30, 0xbd1c_6020, 0xbf51_4793, 0x3ec6_362b],
            [0xbf02_4f5d, 0x3f49_0560, 0xbe9c_5bb6, 0xbf02_4f5d],
            [0x3f2e_b7cc, 0x3eb4_7ecb, 0x3f5f_afb9, 0x3f2e_b7cc],
            [0xff7f_ffff, 0xff7f_ffff, 0xbf12_376a, 0xff7f_ffff],
            [0x7f7f_ffff, 0x7f7f_ffff, 0xbf12_376a, 0x7f7f_ffff],
        );
        assert_output(
            run(3, 1),
            [0x3f02_5d30, 0xbd1c_6020, 0xbf51_4793, 0x3ec6_362b],
            [0xbf02_4f5d, 0x3f49_0560, 0xbe9c_5bb6, 0xbf02_4f5d],
            [0xbf34_07e5, 0xbe62_4612, 0x3f1a_9fc6, 0xb254_26a8],
            [0xff7f_ffff, 0xff7f_ffff, 0x3cc0_6d60, 0xff7f_ffff],
            [0x7f7f_ffff, 0x7f7f_ffff, 0x3cc0_6d60, 0x7f7f_ffff],
        );
    }

    #[test]
    fn cone_threshold_equal_skips_and_one_ulp_below_enters() {
        let mut equal = fixture();
        equal.raw_swing_type = 1;
        equal.raw_twist_type = 2;
        equal.relative_right[0] = f32::from_bits(0x3f7f_ff58);
        let baseline = unchanged_output(equal);
        let mut equal_backend = HostBackend::default();
        let equal_got = apply_joint_swing_stage(equal, &mut equal_backend).unwrap();
        assert_output_bits_eq(equal_got, baseline);
        assert!(equal_backend.trace.is_empty());

        let mut below = equal;
        below.relative_right[0] = f32::from_bits(0x3f7f_ff57);
        let mut below_backend = HostBackend::default();
        let below_got = apply_joint_swing_stage(below, &mut below_backend).unwrap();
        assert_eq!(below_got.angular_packet_a[1].to_bits(), 0xbf51_b096);
        assert_eq!(below_backend.trace.len(), 2);
        assert_eq!(
            below_backend.trace[0].0,
            JointSwingEstimateSite::ConeReciprocalSqrt
        );
        assert_eq!(
            below_backend.trace[1].0,
            JointSwingEstimateSite::ConeReciprocal
        );
    }

    #[test]
    fn hinge_denominator_zero_sign_and_bound_side_are_exact() {
        for zero in [0x0000_0000, 0x8000_0000] {
            let mut input = fixture();
            input.raw_swing_type = 2;
            input.raw_twist_type = 1;
            input.relative_at[0] = f32::from_bits(zero);
            let mut backend = HostBackend::default();
            let got = apply_joint_swing_stage(input, &mut backend).unwrap();
            assert!(backend.trace.is_empty());
            assert_eq!(got.angular_packet_a[1].to_bits(), 0xff7f_ffff);
            assert_eq!(got.angular_packet_b[1].to_bits(), 0x7f7f_ffff);
            assert_eq!(got.angular_packet_a[2].to_bits(), 0x3cc0_6d60);
            assert_eq!(got.angular_packet_b[2].to_bits(), 0x3cc0_6d60);
            assert_eq!(
                bits4(got.t_at),
                [0xbf34_07e5, 0xbe62_4612, 0x3f1a_9fc6, 0xb254_26a8]
            );
        }

        let positive = run(2, 1);
        assert_eq!(positive.angular_packet_a[1].to_bits(), 0x3f5e_ba5a);
        assert_eq!(positive.angular_packet_b[1].to_bits(), 0x7f7f_ffff);

        let mut negative_input = fixture();
        negative_input.raw_swing_type = 2;
        negative_input.raw_twist_type = 1;
        negative_input.relative_at[0] = f32::from_bits(0xbf44_cdca);
        let mut negative_backend = HostBackend::default();
        let negative = apply_joint_swing_stage(negative_input, &mut negative_backend).unwrap();
        assert_eq!(negative.angular_packet_a[1].to_bits(), 0xff7f_ffff);
        assert_eq!(negative.angular_packet_b[1].to_bits(), 0xbf5e_ba5a);
        assert_eq!(negative_backend.trace.len(), 1);
        assert_eq!(
            negative_backend.trace[0].0,
            JointSwingEstimateSite::HingeReciprocal
        );
        assert_eq!(negative_backend.trace[0].1, [0xbf44_cdca; 4]);
        assert_eq!(negative_backend.trace[0].2, [0xbfa6_8034; 4]);
    }

    #[test]
    fn unknown_unsigned_swing_words_are_unchanged_and_call_nothing() {
        for raw in [4, 5, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
            let mut input = fixture();
            input.raw_swing_type = raw;
            input.raw_twist_type = 0xffff_ffff;
            let expected = unchanged_output(input);
            let mut backend = HostBackend::default();
            let got = apply_joint_swing_stage(input, &mut backend).unwrap();
            assert_output_bits_eq(got, expected);
            assert!(backend.trace.is_empty());
            assert_eq!(join_hash(got), 0xe5d3_5ace_f00d_061f);
        }

        for swing in [2, 3] {
            let expected = run(swing, 1);
            for raw_twist in [2, 4, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
                assert_output_bits_eq(run(swing, raw_twist), expected);
            }
        }
    }

    #[test]
    fn estimate_operands_and_lazy_order_match_literal_oracle() {
        let mut cone = fixture();
        cone.raw_swing_type = 1;
        cone.raw_twist_type = 1;
        let mut cone_backend = HostBackend::default();
        apply_joint_swing_stage(cone, &mut cone_backend).unwrap();
        assert_eq!(
            cone_backend.trace,
            vec![
                (
                    JointSwingEstimateSite::ConeReciprocalSqrt,
                    [0x3f17_6fec; 4],
                    [0x3fa6_6c52; 4],
                ),
                (
                    JointSwingEstimateSite::ConeReciprocal,
                    [0x3f44_e54e; 4],
                    [0x3fa6_6c52; 4],
                ),
            ]
        );

        let mut hinge = fixture();
        hinge.raw_swing_type = 2;
        hinge.raw_twist_type = 1;
        let mut hinge_backend = HostBackend::default();
        apply_joint_swing_stage(hinge, &mut hinge_backend).unwrap();
        assert_eq!(
            hinge_backend.trace,
            vec![(
                JointSwingEstimateSite::HingeReciprocal,
                [0x3f44_cdca; 4],
                [0x3fa6_8034; 4],
            )]
        );
    }

    #[test]
    fn failures_retain_exact_backend_prefix_and_stop() {
        let mut cone = fixture();
        cone.raw_swing_type = 1;
        let mut fail_first = FailingBackend {
            fail_at: JointSwingEstimateSite::ConeReciprocalSqrt,
            trace: Vec::new(),
        };
        assert!(matches!(
            apply_joint_swing_stage(cone, &mut fail_first),
            Err(InjectedError::Estimate)
        ));
        assert_eq!(
            fail_first.trace,
            vec![(JointSwingEstimateSite::ConeReciprocalSqrt, [0x3f17_6fec; 4])]
        );

        let mut fail_second = FailingBackend {
            fail_at: JointSwingEstimateSite::ConeReciprocal,
            trace: Vec::new(),
        };
        assert!(matches!(
            apply_joint_swing_stage(cone, &mut fail_second),
            Err(InjectedError::Estimate)
        ));
        assert_eq!(
            fail_second.trace,
            vec![
                (JointSwingEstimateSite::ConeReciprocalSqrt, [0x3f17_6fec; 4]),
                (JointSwingEstimateSite::ConeReciprocal, [0x3f44_e54e; 4]),
            ]
        );

        let mut hinge = fixture();
        hinge.raw_swing_type = 2;
        hinge.raw_twist_type = 1;
        let mut fail_hinge = FailingBackend {
            fail_at: JointSwingEstimateSite::HingeReciprocal,
            trace: Vec::new(),
        };
        assert!(matches!(
            apply_joint_swing_stage(hinge, &mut fail_hinge),
            Err(InjectedError::Estimate)
        ));
        assert_eq!(
            fail_hinge.trace,
            vec![(JointSwingEstimateSite::HingeReciprocal, [0x3f44_cdca; 4])]
        );
    }

    fn fixture() -> JointSwingStageInput {
        JointSwingStageInput {
            raw_swing_type: 4,
            raw_twist_type: 2,
            swing_c: f32::from_bits(0x3ebd_70a4),
            rqd_jwj: [
                f32::from_bits(0xbe80_1e13),
                f32::from_bits(0x3eef_331f),
                f32::from_bits(0x3e2c_900a),
                f32::from_bits(0x3f40_ea09),
            ],
            rqd_jwk: [
                f32::from_bits(0x3f2e_b7cc),
                f32::from_bits(0x3eb4_7ecb),
                f32::from_bits(0x3f5f_afb9),
                f32::from_bits(0xbe92_376a),
            ],
            relative_right: [
                f32::from_bits(0xbe99_02cd),
                f32::from_bits(0xbf06_ef75),
                f32::from_bits(0xbf0f_638c),
                f32::from_bits(0xbe4f_5346),
            ],
            relative_up: [
                f32::from_bits(0xbcc0_6d60),
                f32::from_bits(0x3f45_2954),
                f32::from_bits(0xbf17_69fa),
                f32::from_bits(0xbcc0_6d60),
            ],
            relative_at: [
                f32::from_bits(0x3f44_cdca),
                f32::from_bits(0xbe89_e8f8),
                f32::from_bits(0xbe4f_5346),
                f32::from_bits(0xbf06_ef75),
            ],
            body_a_frame_right: [
                f32::from_bits(0x3eda_78b7),
                f32::from_bits(0x3f07_414d),
                f32::from_bits(0x3f30_aabe),
                f32::from_bits(0x3f2d_684e),
            ],
            t_right: [
                f32::from_bits(0x3f02_5d30),
                f32::from_bits(0xbd1c_6020),
                f32::from_bits(0xbf51_4793),
                f32::from_bits(0x3ec6_362b),
            ],
            t_up: [
                f32::from_bits(0xbf02_4f5d),
                f32::from_bits(0x3f49_0560),
                f32::from_bits(0xbe9c_5bb6),
                f32::from_bits(0xbf02_4f5d),
            ],
            t_at: [
                f32::from_bits(0x3f24_0d15),
                f32::from_bits(0x3f17_a5ec),
                f32::from_bits(0x3ec6_362b),
                f32::from_bits(0xbd1c_6020),
            ],
            angular_packet_a: [f32::from_bits(0xff7f_ffff); 4],
            angular_packet_b: [f32::from_bits(0x7f7f_ffff); 4],
        }
    }

    fn run(swing: u32, twist: u32) -> JointSwingStageOutput {
        let mut input = fixture();
        input.raw_swing_type = swing;
        input.raw_twist_type = twist;
        apply_joint_swing_stage(input, &mut HostBackend::default()).unwrap()
    }

    fn unchanged_output(input: JointSwingStageInput) -> JointSwingStageOutput {
        JointSwingStageOutput {
            t_right: input.t_right,
            t_up: input.t_up,
            t_at: input.t_at,
            angular_packet_a: input.angular_packet_a,
            angular_packet_b: input.angular_packet_b,
        }
    }

    fn assert_output(
        got: JointSwingStageOutput,
        right: [u32; 4],
        up: [u32; 4],
        at: [u32; 4],
        packet_a: [u32; 4],
        packet_b: [u32; 4],
    ) {
        assert_eq!(bits4(got.t_right), right);
        assert_eq!(bits4(got.t_up), up);
        assert_eq!(bits4(got.t_at), at);
        assert_eq!(bits4(got.angular_packet_a), packet_a);
        assert_eq!(bits4(got.angular_packet_b), packet_b);
    }

    fn assert_output_bits_eq(left: JointSwingStageOutput, right: JointSwingStageOutput) {
        assert_output(
            left,
            bits4(right.t_right),
            bits4(right.t_up),
            bits4(right.t_at),
            bits4(right.angular_packet_a),
            bits4(right.angular_packet_b),
        );
    }

    fn bits4(value: Vec4) -> [u32; 4] {
        value.map(f32::to_bits)
    }

    fn join_hash(output: JointSwingStageOutput) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for vector in [
            output.t_right,
            output.t_up,
            output.t_at,
            output.angular_packet_a,
            output.angular_packet_b,
        ] {
            for word in bits4(vector) {
                for byte in word.to_be_bytes() {
                    hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        hash
    }
}
