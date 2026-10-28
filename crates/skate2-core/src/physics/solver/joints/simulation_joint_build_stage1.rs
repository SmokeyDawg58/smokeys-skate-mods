//! Evidence-bounded first stage of `rw::physics::JointJacobian::Build`.
//!
//! The pre-Swing numeric/value graph across `0x82ae596c..0x82ae5dcc`
//! composes the body/frame quaternions, RQD and basis values,
//! attachment/prediction values, nine horizontal reductions, prism packets,
//! and initial angular packets. Contemporaneous Swing-control loads/dispatch
//! setup and every later Build stage remain outside this module.
//!
//! This is an ordinary-finite typed translation. Xenon `vmsum3fp128`
//! reduction, denormal, NaN, infinity, signed-zero min/max, exception, and
//! FPSCR behavior remain explicit boundaries rather than host-parity claims.

use crate::{
    simulation_constraint_math::{
        compose_body_frame_quaternion_from_dot3, create_jacobian_rqd_rows,
    },
    simulation_joint_numeric::{
        SimulationJointNumericBackend, SimulationJointSumSite, SimulationJointVector,
    },
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
};

#[cfg(test)]
use crate::simulation_joint_numeric::{
    SimulationJointReciprocalSite, SimulationJointReciprocalSqrtSite,
};

type JointStage1Vector = SimulationJointVector;

/// Copied body values consumed by the first Joint Build stage.
///
/// COM W carries the supplied RF-row word. The other source W lanes are full
/// inputs for raw intermediate audits; the checked final-record adapter may
/// supply zero only under its separately proven metadata-W deadness boundary.
/// This private non-`Debug` value is not an owning body or solver capability.
#[derive(Clone, Copy)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "private complete Build stays internal pending the owning full Horus runtime"
    )
)]
pub(crate) struct SimulationJointBuildStage1Body {
    pub(crate) quaternion: JointStage1Vector,
    pub(crate) center_of_mass_and_rf_word: JointStage1Vector,
    pub(crate) velocity: JointStage1Vector,
    pub(crate) omega: JointStage1Vector,
    pub(crate) right: JointStage1Vector,
    pub(crate) up: JointStage1Vector,
    pub(crate) at: JointStage1Vector,
    pub(crate) force: JointStage1Vector,
    pub(crate) torque: JointStage1Vector,
}

/// Four value rows returned by native `Jacobian_RQD::Create`.
///
/// This value makes no native-compatible Rust layout or field-offset claim.
#[derive(Clone, Copy)]
pub(crate) struct SimulationJointStage1Rqd {
    pub(crate) rows: [JointStage1Vector; 4],
}

/// Values live immediately after `0x82ae5dcc`, before Swing dispatch.
#[derive(Clone, Copy)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "private complete Build stays internal pending the owning full Horus runtime"
    )
)]
pub(crate) struct SimulationJointBuildStage1 {
    quaternion_a: JointStage1Vector,
    quaternion_b: JointStage1Vector,
    limit_frame_quaternion: JointStage1Vector,
    pub(crate) rqd: SimulationJointStage1Rqd,
    pub(crate) attachment_a: JointStage1Vector,
    pub(crate) attachment_b: JointStage1Vector,
    /// Native A ZXY/YZX permutations retained from 0x5cb4/0x5cf0.
    pub(crate) attachment_a_zxy: JointStage1Vector,
    pub(crate) attachment_a_yzx: JointStage1Vector,
    /// Native B ZXY/YZX permutations retained from 0x5d0c/0x5d00.
    pub(crate) attachment_b_zxy: JointStage1Vector,
    pub(crate) attachment_b_yzx: JointStage1Vector,
    motion_delta: JointStage1Vector,
    predicted_delta: JointStage1Vector,
    /// Physical v94/v93/v95, in Right/Up/At semantic order.
    pub(crate) joint_axes: [JointStage1Vector; 3],
    /// Physical v9/v10/v11, in Right/Up/At semantic order.
    pub(crate) body_b_frame_axes: [JointStage1Vector; 3],
    /// Physical v85/v87/v86, in Right/Up/At semantic order.
    pub(crate) relative_axes: [JointStage1Vector; 3],
    /// Physical v88, the qA basis row retained by later Swing logic.
    pub(crate) body_a_frame_right: JointStage1Vector,
    /// Six hardware sums packed Right/Up/At with native duplicated W.
    motion_projection: JointStage1Vector,
    predicted_projection: JointStage1Vector,
    /// Physical v18 and v19; conservative names precede later clamp routing.
    pub(crate) position_packet_a: JointStage1Vector,
    pub(crate) position_packet_b: JointStage1Vector,
    /// Physical v90 and v89; conservative names precede later clamp routing.
    pub(crate) angular_packet_a: JointStage1Vector,
    pub(crate) angular_packet_b: JointStage1Vector,
}

/// Translates the pre-Swing numeric/value graph produced across
/// `JointJacobian::Build` `0x82ae596c..0x82ae5dcc`.
///
/// Callback order is `QuaternionA`, `QuaternionB`, `LimitFrameQuaternion`,
/// `MotionUp`, `MotionRight`, `MotionAt`, `PredictedUp`, `PredictedRight`,
/// `PredictedAt`.
/// Failure returns no stage output. Backend effects through and including the
/// failing callback remain observable; no callback after it executes.
#[allow(
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "the uninterrupted source-shaped stage preserves native arithmetic order"
)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "private complete Build stays internal pending the owning full Horus runtime"
    )
)]
pub(crate) fn build_simulation_joint_stage1<B: SimulationJointNumericBackend>(
    body_a: SimulationJointBuildStage1Body,
    body_b: SimulationJointBuildStage1Body,
    frames: SimulationJointFrames,
    limits: SimulationJointLimits,
    time_step: f32,
    backend: &mut B,
) -> Result<SimulationJointBuildStage1, B::Error> {
    let quaternion_a = compose_body_frame(
        body_a.quaternion,
        frames.quaternion_a,
        SimulationJointSumSite::QuaternionA,
        backend,
    )?;
    let quaternion_b = compose_body_frame(
        body_b.quaternion,
        frames.quaternion_b,
        SimulationJointSumSite::QuaternionB,
        backend,
    )?;
    let rqd = SimulationJointStage1Rqd::create(quaternion_a, quaternion_b);

    // The third reduction uses raw body B times mQuatL. Substituting the
    // already-composed quaternion_b changes the recovered operand graph.
    let limit_frame_quaternion = compose_body_frame(
        body_b.quaternion,
        frames.quaternion_l,
        SimulationJointSumSite::LimitFrameQuaternion,
        backend,
    )?;

    let joint_basis = basis_from_quaternion(limit_frame_quaternion);
    let body_b_basis = basis_from_quaternion(quaternion_b);
    let body_a_basis = basis_from_quaternion(quaternion_a);
    let relative_quaternion = [
        rqd.rows[0][3],
        rqd.rows[1][3],
        rqd.rows[2][3],
        rqd.rows[3][3],
    ];
    let relative_basis = basis_from_quaternion(relative_quaternion);

    let attachment_a = attachment_offset(body_a, frames.position_a);
    let attachment_b = attachment_offset(body_b, frames.position_b);
    let attachment_a_zxy = rotate_zxy(attachment_a);
    let attachment_a_yzx = rotate_yzx(attachment_a);
    let attachment_b_yzx = rotate_yzx(attachment_b);
    let attachment_b_zxy = rotate_zxy(attachment_b);
    let world_a = vector_add(body_a.center_of_mass_and_rf_word, attachment_a);
    let world_b = vector_add(body_b.center_of_mass_and_rf_word, attachment_b);

    let velocity_a = cross_add(body_a.omega, attachment_a, body_a.velocity);
    let velocity_b = cross_add(body_b.omega, attachment_b, body_b.velocity);
    let acceleration_a = cross_add(body_a.torque, attachment_a, body_a.force);
    let acceleration_b = cross_add(body_b.torque, attachment_b, body_b.force);

    // Native forms (acceleration*h + velocity)*h per body before subtraction.
    let step_a = vector_mul_scalar(
        vector_madd_scalar(acceleration_a, time_step, velocity_a),
        time_step,
    );
    let step_b = vector_mul_scalar(
        vector_madd_scalar(acceleration_b, time_step, velocity_b),
        time_step,
    );
    let predicted_a = vector_add(world_a, step_a);
    let predicted_b = vector_add(world_b, step_b);
    let motion_delta = vector_sub(step_b, step_a);
    let predicted_delta = vector_sub(predicted_b, predicted_a);

    // Native physical call order is Up, Right, At. The later packing changes
    // only semantic lane order to Right, Up, At.
    let motion_up = call_sum(
        backend,
        SimulationJointSumSite::MotionUp,
        motion_delta,
        joint_basis[1],
    )?;
    let motion_right = call_sum(
        backend,
        SimulationJointSumSite::MotionRight,
        motion_delta,
        joint_basis[0],
    )?;
    let motion_at = call_sum(
        backend,
        SimulationJointSumSite::MotionAt,
        motion_delta,
        joint_basis[2],
    )?;
    let predicted_up = call_sum(
        backend,
        SimulationJointSumSite::PredictedUp,
        predicted_delta,
        joint_basis[1],
    )?;
    let predicted_right = call_sum(
        backend,
        SimulationJointSumSite::PredictedRight,
        predicted_delta,
        joint_basis[0],
    )?;
    let predicted_at = call_sum(
        backend,
        SimulationJointSumSite::PredictedAt,
        predicted_delta,
        joint_basis[2],
    )?;
    let motion_projection = [motion_right, motion_up, motion_at, motion_right];
    let predicted_projection = [predicted_right, predicted_up, predicted_at, predicted_right];

    let velocity_limit_step = vector_mul_scalar(limits.v_prism, time_step);
    let motion_plus = vector_add(motion_projection, velocity_limit_step);
    let motion_minus = vector_sub(motion_projection, velocity_limit_step);
    let predicted_minus = vector_sub(predicted_projection, limits.p_prism);
    let predicted_plus = vector_add(predicted_projection, limits.p_prism);
    let outer_a = vector_max(motion_plus, predicted_minus);
    let outer_b = vector_min(motion_minus, predicted_plus);
    let position_packet_a = vector_min(predicted_plus, outer_a);
    let position_packet_b = vector_max(predicted_minus, outer_b);

    Ok(SimulationJointBuildStage1 {
        quaternion_a,
        quaternion_b,
        limit_frame_quaternion,
        rqd,
        attachment_a,
        attachment_b,
        attachment_a_zxy,
        attachment_a_yzx,
        attachment_b_zxy,
        attachment_b_yzx,
        motion_delta,
        predicted_delta,
        joint_axes: joint_basis,
        body_b_frame_axes: body_b_basis,
        relative_axes: relative_basis,
        body_a_frame_right: body_a_basis[0],
        motion_projection,
        predicted_projection,
        position_packet_a,
        position_packet_b,
        angular_packet_a: [f32::from_bits(0xff7f_ffff); 4],
        angular_packet_b: [f32::from_bits(0x7f7f_ffff); 4],
    })
}

fn call_sum<B: SimulationJointNumericBackend>(
    backend: &mut B,
    site: SimulationJointSumSite,
    left: JointStage1Vector,
    right: JointStage1Vector,
) -> Result<f32, B::Error> {
    backend.sum3(site, first3(left), first3(right))
}

fn rotate_zxy(value: JointStage1Vector) -> JointStage1Vector {
    [value[2], value[0], value[1], value[3]]
}

fn rotate_yzx(value: JointStage1Vector) -> JointStage1Vector {
    [value[1], value[2], value[0], value[3]]
}

fn compose_body_frame<B: SimulationJointNumericBackend>(
    body: JointStage1Vector,
    frame: JointStage1Vector,
    site: SimulationJointSumSite,
    backend: &mut B,
) -> Result<JointStage1Vector, B::Error> {
    let dot3 = call_sum(backend, site, body, frame)?;
    Ok(compose_body_frame_quaternion_from_dot3(body, frame, dot3))
}

fn attachment_offset(
    body: SimulationJointBuildStage1Body,
    position: JointStage1Vector,
) -> JointStage1Vector {
    std::array::from_fn(|lane| {
        body.at[lane].mul_add(
            position[2],
            body.up[lane].mul_add(position[1], body.right[lane] * position[0]),
        )
    })
}

/// Native cross-add order: `-offset_yzx*source_zxy + base`, then
/// `offset_zxy*source_yzx + first`, including computational W.
fn cross_add(
    source: JointStage1Vector,
    offset: JointStage1Vector,
    base: JointStage1Vector,
) -> JointStage1Vector {
    let source_yzx = yzxw(source);
    let source_zxy = zxyw(source);
    let offset_yzx = yzxw(offset);
    let offset_zxy = zxyw(offset);
    std::array::from_fn(|lane| {
        let first = (-offset_yzx[lane]).mul_add(source_zxy[lane], base[lane]);
        offset_zxy[lane].mul_add(source_yzx[lane], first)
    })
}

fn basis_from_quaternion(quaternion: JointStage1Vector) -> [JointStage1Vector; 3] {
    let scaled = vector_mul(quaternion, [f32::from_bits(0x3fb5_04f3); 4]);
    let negative_squares = vector_nmsub(scaled, scaled, [0.5; 4]);
    let products = vector_mul(scaled, yzxx(scaled));
    let w_products = vector_mul([scaled[3]; 4], zxyx(scaled));
    let sum = vector_add(products, w_products);
    let difference = vector_sub(products, w_products);
    let diagonal = vector_add(negative_squares, yzxx(negative_squares));
    [
        [diagonal[1], sum[0], difference[2], diagonal[0]],
        [difference[0], diagonal[2], sum[1], difference[0]],
        [sum[2], difference[1], diagonal[0], sum[0]],
    ]
}

impl SimulationJointStage1Rqd {
    fn create(a: JointStage1Vector, b: JointStage1Vector) -> Self {
        Self {
            rows: create_jacobian_rqd_rows(a, b),
        }
    }
}

fn vector_madd_scalar(
    multiplier: JointStage1Vector,
    scalar: f32,
    addend: JointStage1Vector,
) -> JointStage1Vector {
    std::array::from_fn(|lane| multiplier[lane].mul_add(scalar, addend[lane]))
}

fn vector_mul_scalar(value: JointStage1Vector, scalar: f32) -> JointStage1Vector {
    value.map(|lane| lane * scalar)
}

fn vector_add(left: JointStage1Vector, right: JointStage1Vector) -> JointStage1Vector {
    std::array::from_fn(|lane| left[lane] + right[lane])
}

fn vector_sub(left: JointStage1Vector, right: JointStage1Vector) -> JointStage1Vector {
    std::array::from_fn(|lane| left[lane] - right[lane])
}

fn vector_mul(left: JointStage1Vector, right: JointStage1Vector) -> JointStage1Vector {
    std::array::from_fn(|lane| left[lane] * right[lane])
}

fn vector_nmsub(
    multiplier: JointStage1Vector,
    multiplicand: JointStage1Vector,
    addend: JointStage1Vector,
) -> JointStage1Vector {
    std::array::from_fn(|lane| (-multiplier[lane]).mul_add(multiplicand[lane], addend[lane]))
}

fn vector_max(left: JointStage1Vector, right: JointStage1Vector) -> JointStage1Vector {
    std::array::from_fn(|lane| left[lane].max(right[lane]))
}

fn vector_min(left: JointStage1Vector, right: JointStage1Vector) -> JointStage1Vector {
    std::array::from_fn(|lane| left[lane].min(right[lane]))
}

const fn first3(value: JointStage1Vector) -> [f32; 3] {
    [value[0], value[1], value[2]]
}

const fn yzxw(value: JointStage1Vector) -> JointStage1Vector {
    [value[1], value[2], value[0], value[3]]
}

const fn zxyw(value: JointStage1Vector) -> JointStage1Vector {
    [value[2], value[0], value[1], value[3]]
}

const fn yzxx(value: JointStage1Vector) -> JointStage1Vector {
    [value[1], value[2], value[0], value[0]]
}

const fn zxyx(value: JointStage1Vector) -> JointStage1Vector {
    [value[2], value[0], value[1], value[0]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct HostBackend {
        trace: Vec<(SimulationJointSumSite, [u32; 3], [u32; 3])>,
    }

    impl SimulationJointNumericBackend for HostBackend {
        type Error = ();

        fn sum3(
            &mut self,
            site: SimulationJointSumSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.trace.push((site, bits3(left), bits3(right)));
            Ok(left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2])))
        }

        fn reciprocal_estimate(
            &mut self,
            _site: SimulationJointReciprocalSite,
            _input: JointStage1Vector,
        ) -> Result<JointStage1Vector, Self::Error> {
            unreachable!("the pre-Swing stage has no estimate site")
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            _site: SimulationJointReciprocalSqrtSite,
            _input: JointStage1Vector,
        ) -> Result<JointStage1Vector, Self::Error> {
            unreachable!("the pre-Swing stage has no estimate site")
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum InjectedError {
        Interior,
    }

    struct FailingBackend {
        fail_at: SimulationJointSumSite,
        trace: Vec<(SimulationJointSumSite, [u32; 3], [u32; 3])>,
    }

    impl SimulationJointNumericBackend for FailingBackend {
        type Error = InjectedError;

        fn sum3(
            &mut self,
            site: SimulationJointSumSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.trace.push((site, bits3(left), bits3(right)));
            if site == self.fail_at {
                return Err(InjectedError::Interior);
            }
            Ok(left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2])))
        }

        fn reciprocal_estimate(
            &mut self,
            _site: SimulationJointReciprocalSite,
            _input: JointStage1Vector,
        ) -> Result<JointStage1Vector, Self::Error> {
            unreachable!("the pre-Swing stage has no estimate site")
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            _site: SimulationJointReciprocalSqrtSite,
            _input: JointStage1Vector,
        ) -> Result<JointStage1Vector, Self::Error> {
            unreachable!("the pre-Swing stage has no estimate site")
        }
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one literal assertion keeps the complete native stage checkpoint together"
    )]
    fn pre_swing_checkpoint_matches_oracle_with_raw_rqd_correction() {
        let (body_a, body_b, frames, limits) = fixture();
        let mut backend = HostBackend::default();
        let got = build_simulation_joint_stage1(
            body_a,
            body_b,
            frames,
            limits,
            f32::from_bits(0x3c88_8889),
            &mut backend,
        )
        .expect("host reduction backend succeeds");

        // Jwk.y corrects the earlier full-record oracle shortcut's duplicated
        // row typo using the raw 0x82ae3750..0x82ae3930 RQD discriminator.
        assert_eq!(
            bits4(got.limit_frame_quaternion),
            [0x3c41_54d4, 0x3dcd_9e83, 0x3e7c_b922, 0x3f71_4e3b]
        );
        assert_eq!(
            bits4(got.quaternion_a),
            [0x3e90_f27b, 0xbe91_de6a, 0x3ee8_16ef, 0x3f42_bd3c]
        );
        assert_eq!(
            bits4(got.quaternion_b),
            [0xbe94_9518, 0x3ef1_5b57, 0x3e1b_da51, 0x3f45_fd8b]
        );
        assert_eq!(
            got.rqd.rows.map(bits4),
            [
                [0x3f12_4c79, 0x3f2e_b7cc, 0xbe68_2739, 0xbe3b_0a82],
                [0xbe80_1e13, 0x3eef_331f, 0x3e2c_900a, 0x3f40_ea09],
                [0x3d4b_f330, 0x3e30_42b2, 0x3f5f_afb9, 0xbe92_376a],
                [0x3f32_5e58, 0xbecf_006e, 0x3e3c_71ed, 0x3ee1_b86d],
            ]
        );
        assert_eq!(
            bits4(got.attachment_a),
            [0x3ed1_eb85, 0xbe8a_3d71, 0x3e05_1eb8, 0x4105_c28f]
        );
        assert_eq!(
            bits4(got.attachment_b),
            [0xbe84_f766, 0x3cf9_db21, 0x3f10_0000, 0x415d_999a]
        );
        assert_eq!(
            bits4(got.attachment_a_zxy),
            [0x3e05_1eb8, 0x3ed1_eb85, 0xbe8a_3d71, 0x4105_c28f]
        );
        assert_eq!(
            bits4(got.attachment_a_yzx),
            [0xbe8a_3d71, 0x3e05_1eb8, 0x3ed1_eb85, 0x4105_c28f]
        );
        assert_eq!(
            bits4(got.attachment_b_zxy),
            [0x3f10_0000, 0xbe84_f766, 0x3cf9_db21, 0x415d_999a]
        );
        assert_eq!(
            bits4(got.attachment_b_yzx),
            [0x3cf9_db21, 0x3f10_0000, 0xbe84_f766, 0x415d_999a]
        );
        assert_eq!(
            bits4(got.motion_delta),
            [0xbbda_8d66, 0x3c33_c458, 0xbb97_7610, 0xb400_0000]
        );
        assert_eq!(
            bits4(got.predicted_delta),
            [0xc03e_7cd7, 0x3fdb_1186, 0x3fa9_f7e6, 0x40af_ae16]
        );
        assert_eq!(
            got.joint_axes.map(bits4),
            [
                [0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33, 0x3f7a_c486],
                [0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a, 0xbeed_00f2],
                [0x3e47_c7e9, 0x3cdf_bd86, 0x3f7a_c486, 0x3eef_6e14],
            ]
        );
        assert_eq!(
            got.body_b_frame_axes.map(bits4),
            [
                [0x3f02_5d30, 0xbd1c_6020, 0xbf51_4793, 0x3ec6_362b],
                [0xbf02_4f5d, 0x3f49_0560, 0xbe9c_5bb6, 0xbf02_4f5d],
                [0x3f24_0d15, 0x3f17_a5ec, 0x3ec6_362b, 0xbd1c_6020],
            ]
        );
        assert_eq!(
            got.relative_axes.map(bits4),
            [
                [0xbe99_02cd, 0xbf06_ef75, 0xbf0f_638c, 0xbe4f_5346],
                [0xbcc0_6d60, 0x3f45_2954, 0xbf17_69fa, 0xbcc0_6d60],
                [0x3f44_cdca, 0xbe89_e8f8, 0xbe4f_5346, 0xbf06_ef75],
            ]
        );
        assert_eq!(
            bits4(got.body_a_frame_right),
            [0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe, 0x3f2d_684e]
        );
        assert_eq!(
            bits4(got.motion_projection),
            [0x3986_5071, 0x3c4a_f6b2, 0xbbb5_2f64, 0x3986_5071]
        );
        assert_eq!(
            bits4(got.predicted_projection),
            [0xbfff_9ecd, 0x403e_704b, 0x3f44_4cbe, 0xbfff_9ecd]
        );
        assert_eq!(
            bits4(got.position_packet_a),
            [0xc02c_9c33, 0x401e_704b, 0xbe08_66a0, 0xbe6e_abc8]
        );
        assert_eq!(
            bits4(got.position_packet_b),
            [0xbfa6_0534, 0x405e_704b, 0x3fd5_5992, 0x3e6f_3218]
        );
        assert_eq!(bits4(got.angular_packet_a), [0xff7f_ffff; 4]);
        assert_eq!(bits4(got.angular_packet_b), [0x7f7f_ffff; 4]);

        assert_eq!(
            backend
                .trace
                .iter()
                .map(|entry| entry.0)
                .collect::<Vec<_>>(),
            vec![
                SimulationJointSumSite::QuaternionA,
                SimulationJointSumSite::QuaternionB,
                SimulationJointSumSite::LimitFrameQuaternion,
                SimulationJointSumSite::MotionUp,
                SimulationJointSumSite::MotionRight,
                SimulationJointSumSite::MotionAt,
                SimulationJointSumSite::PredictedUp,
                SimulationJointSumSite::PredictedRight,
                SimulationJointSumSite::PredictedAt,
            ]
        );
        let literal_operands = [
            (
                [0x3de1_47ae, 0xbd8f_5c29, 0x3e42_8f5c],
                [0x3e2e_147b, 0xbe6b_851f, 0x3e9e_b852],
            ),
            (
                [0xbe05_1eb8, 0x3e57_0a3d, 0x3d4c_cccd],
                [0xbe42_8f5c, 0x3e94_7ae1, 0x3de1_47ae],
            ),
            (
                [0xbe05_1eb8, 0x3e57_0a3d, 0x3d4c_cccd],
                [0x3db8_51ec, 0xbe0f_5c29, 0x3e57_0a3d],
            ),
            (
                [0xbbda_8d66, 0x3c33_c458, 0xbb97_7610],
                [0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a],
            ),
            (
                [0xbbda_8d66, 0x3c33_c458, 0xbb97_7610],
                [0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33],
            ),
            (
                [0xbbda_8d66, 0x3c33_c458, 0xbb97_7610],
                [0x3e47_c7e9, 0x3cdf_bd86, 0x3f7a_c486],
            ),
            (
                [0xc03e_7cd7, 0x3fdb_1186, 0x3fa9_f7e6],
                [0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a],
            ),
            (
                [0xc03e_7cd7, 0x3fdb_1186, 0x3fa9_f7e6],
                [0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33],
            ),
            (
                [0xc03e_7cd7, 0x3fdb_1186, 0x3fa9_f7e6],
                [0x3e47_c7e9, 0x3cdf_bd86, 0x3f7a_c486],
            ),
        ];
        assert_eq!(
            backend
                .trace
                .iter()
                .map(|entry| (entry.1, entry.2))
                .collect::<Vec<_>>(),
            literal_operands
        );
    }

    #[test]
    fn interior_failure_retains_exact_callback_prefix_and_stops() {
        let (body_a, body_b, frames, limits) = fixture();
        let mut backend = FailingBackend {
            fail_at: SimulationJointSumSite::MotionRight,
            trace: Vec::new(),
        };
        let result = build_simulation_joint_stage1(
            body_a,
            body_b,
            frames,
            limits,
            f32::from_bits(0x3c88_8889),
            &mut backend,
        );
        assert!(matches!(result, Err(InjectedError::Interior)));
        assert_eq!(
            backend.trace,
            vec![
                (
                    SimulationJointSumSite::QuaternionA,
                    [0x3de1_47ae, 0xbd8f_5c29, 0x3e42_8f5c],
                    [0x3e2e_147b, 0xbe6b_851f, 0x3e9e_b852],
                ),
                (
                    SimulationJointSumSite::QuaternionB,
                    [0xbe05_1eb8, 0x3e57_0a3d, 0x3d4c_cccd],
                    [0xbe42_8f5c, 0x3e94_7ae1, 0x3de1_47ae],
                ),
                (
                    SimulationJointSumSite::LimitFrameQuaternion,
                    [0xbe05_1eb8, 0x3e57_0a3d, 0x3d4c_cccd],
                    [0x3db8_51ec, 0xbe0f_5c29, 0x3e57_0a3d],
                ),
                (
                    SimulationJointSumSite::MotionUp,
                    [0xbbda_8d66, 0x3c33_c458, 0xbb97_7610],
                    [0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a],
                ),
                (
                    SimulationJointSumSite::MotionRight,
                    [0xbbda_8d66, 0x3c33_c458, 0xbb97_7610],
                    [0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33],
                ),
            ]
        );
    }

    #[test]
    fn position_w_uses_full_prism_words() {
        let (body_a, body_b, frames, limits) = fixture();
        let mut backend = HostBackend::default();
        let got = build_simulation_joint_stage1(
            body_a,
            body_b,
            frames,
            limits,
            f32::from_bits(0x3c88_8889),
            &mut backend,
        )
        .expect("host reduction backend succeeds");

        assert_eq!(
            got.motion_projection[3].to_bits(),
            got.motion_projection[0].to_bits()
        );
        assert_eq!(
            got.predicted_projection[3].to_bits(),
            got.predicted_projection[0].to_bits()
        );
        assert_eq!(got.position_packet_a[3].to_bits(), 0xbe6e_abc8);
        assert_eq!(got.position_packet_b[3].to_bits(), 0x3e6f_3218);
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
            v_twist: 0.0,
            v_swing: 0.0,
            swing_a: 0.0,
            twist_a: 0.0,
            swing_c: 0.0,
            twist_c: 0.0,
            raw_swing_type: 0,
            raw_twist_type: 0,
        };
        (body_a, body_b, frames, limits)
    }

    fn bits3(value: [f32; 3]) -> [u32; 3] {
        value.map(f32::to_bits)
    }

    fn bits4(value: JointStage1Vector) -> [u32; 4] {
        value.map(f32::to_bits)
    }
}
