//! Value-only translation of `JointJacobian::Build` 0x82ae65e0..0x82ae6754.
//!
//! The input is the private 0x65dc effective-mass checkpoint. The output is the
//! ten fully scaled vectors consumed by the later final permutes/stores. This
//! file creates no Jacobian record, pool slot, body/RF capability, or
//! publication event.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "private stage awaits effective-mass composition")
)]
#![deny(unsafe_code)]

use crate::simulation_joint_numeric::{
    SimulationJointNumericBackend, SimulationJointReciprocalSite, SimulationJointVector,
};

type Vector = SimulationJointVector;

/// Live private values immediately after `0x82ae65dc`.
///
/// Every vector is computationally four-lane; W is never padding. This copied
/// value grants no body, RF, record, pool, or publication authority.
#[derive(Clone, Copy)]
pub(crate) struct JointEffectiveMassSuffixInput {
    pub(crate) inverse_inertia_a: [Vector; 3],
    pub(crate) inverse_inertia_b: [Vector; 3],
    pub(crate) attachment_cross_linear_a: [Vector; 3],
    pub(crate) attachment_cross_linear_b: [Vector; 3],
    pub(crate) angular_axes: [Vector; 3],
    pub(crate) linear_axes: [Vector; 3],
    pub(crate) inertia_t_right: Vector,
    pub(crate) inertia_t_up_a: Vector,
    pub(crate) inertia_t_up_b: Vector,
    pub(crate) inertia_t_at_a: Vector,
    pub(crate) inertia_t_at_b: Vector,
    pub(crate) inertia_cross_right_a: Vector,
    pub(crate) inertia_cross_right_b_xy: Vector,
    pub(crate) inertia_cross_up_a: Vector,
    pub(crate) inertia_cross_at_a_x: Vector,
    pub(crate) inertia_cross_at_b_x: Vector,
    pub(crate) inverse_mass_sum: Vector,
    pub(crate) angular_projection: Vector,
    pub(crate) angular_control_step: Vector,
    pub(crate) angular_trial_plus: Vector,
    pub(crate) angular_packet_a: Vector,
    pub(crate) angular_packet_b: Vector,
    pub(crate) position_packet_a: Vector,
    pub(crate) position_packet_b: Vector,
}

/// Fully scaled vectors live after `0x82ae6754`, before the first final permute.
///
/// Arrays are ordered X/Y/Z transposed packs. This private non-`Debug` value is not a
/// native record and carries no allocation or publication authority.
#[derive(Clone, Copy)]
pub(crate) struct JointEffectiveMassSuffixOutput {
    pub(crate) linear_transpose: [Vector; 3],
    pub(crate) angular_transpose: [Vector; 3],
    pub(crate) position_max: Vector,
    pub(crate) position_min: Vector,
    pub(crate) angular_max: Vector,
    pub(crate) angular_min: Vector,
}

/// Executes the exact ordinary-finite suffix value graph.
///
/// `FinalAngular` is called at the 0x6720 position. Linear-denominator formation
/// and native half scaling follow at 0x6724/0x6728, then `FinalLinear` is called
/// at the 0x672c position. No result is returned unless both calls succeed.
#[allow(
    clippy::large_types_passed_by_value,
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "source-shaped names and chronology preserve the uninterrupted native graph"
)]
pub(crate) fn build_joint_effective_mass_suffix<B: SimulationJointNumericBackend>(
    input: JointEffectiveMassSuffixInput,
    backend: &mut B,
) -> Result<JointEffectiveMassSuffixOutput, B::Error> {
    let [_, ia_y, ia_z] = input.inverse_inertia_a;
    let [ib_x, ib_y, ib_z] = input.inverse_inertia_b;
    let [cr_a, cu_a, ca_a] = input.attachment_cross_linear_a;
    let [cr_b, cu_b, ca_b] = input.attachment_cross_linear_b;
    let [t_right, t_up, t_at] = input.angular_axes;
    let [l_right, l_up, l_at] = input.linear_axes;

    // 0x65e4..0x6618.
    let inertia_t_up = vector_add(input.inertia_t_up_a, input.inertia_t_up_b);
    let cu_b_x = splat(cu_b[0]);
    let inertia_cross_at_a_xy = vector_madd(ia_y, splat(ca_a[1]), input.inertia_cross_at_a_x);
    let inertia_cross_up_b_x = vector_mul0(ib_x, cu_b_x);
    let cr_b_z = splat(cr_b[2]);
    let quadratic_right_a = vector_mul0(cr_a, input.inertia_cross_right_a);
    let inertia_cross_right_b = vector_madd(ib_z, cr_b_z, input.inertia_cross_right_b_xy);
    let ca_a_z = splat(ca_a[2]);
    let ca_b_y = splat(ca_b[1]);
    let angular_trial_minus = vector_sub(input.angular_projection, input.angular_control_step);

    // 0x6620..0x665c. Pack construction is lane selection, not reduction.
    let l_y_pack = pack_y(l_right, l_up, l_at);
    let inertia_cross_at_b_xy = vector_madd(ib_y, ca_b_y, input.inertia_cross_at_b_x);
    let inertia_cross_up_b_xy = vector_madd(ib_y, splat(cu_b[1]), inertia_cross_up_b_x);
    let quadratic_up_a = vector_mul0(cu_a, input.inertia_cross_up_a);
    let ca_b_z = splat(ca_b[2]);
    let inertia_t_at = vector_add(input.inertia_t_at_a, input.inertia_t_at_b);
    let product_t_right = vector_mul0(input.inertia_t_right, t_right);
    let product_t_up = vector_mul0(inertia_t_up, t_up);
    let inertia_cross_at_a = vector_madd(ia_z, ca_a_z, inertia_cross_at_a_xy);
    let quadratic_linear_right = vector_madd(cr_b, inertia_cross_right_b, quadratic_right_a);
    let inertia_cross_up_b = vector_madd(ib_z, splat(cu_b[2]), inertia_cross_up_b_xy);
    let l_x_pack = pack_x(l_right, l_up, l_at);
    let t_at_live = t_at;
    let inertia_cross_at_b = vector_madd(ib_z, ca_b_z, inertia_cross_at_b_xy);

    // 0x6660..0x66e8. These merges/permutations retain their native W maps.
    let product_t_ru_hi = merge_hi(product_t_right, product_t_up);
    let product_t_ru_lo = merge_lo(product_t_right, product_t_up);
    let quadratic_linear_up = vector_madd(cu_b, inertia_cross_up_b, quadratic_up_a);
    let product_t_at = vector_mul0(inertia_t_at, t_at_live);
    let quadratic_at_a = vector_mul0(ca_a, inertia_cross_at_a);
    let l_ru_lo = merge_lo(l_right, l_up);
    let angular_outer_plus = vector_max(input.angular_trial_plus, input.angular_packet_a);
    let angular_outer_minus = vector_min(angular_trial_minus, input.angular_packet_b);
    let t_ru_lo = merge_lo(t_right, t_up);
    let t_y_pack = pack_y(t_right, t_up, t_at_live);
    let l_z_pack = pack_z_from_low_merge(l_ru_lo, l_at);
    let t_z_pack = pack_z_from_low_merge(t_ru_lo, t_at_live);
    let t_x_pack = pack_x(t_right, t_up, t_at_live);
    let ql_ru_lo = merge_lo(quadratic_linear_right, quadratic_linear_up);
    let angular_y_pack = pack_y_from_high_merge(product_t_ru_hi, product_t_at);
    let angular_x_pack = pack_x_from_high_merge(product_t_ru_hi, product_t_at);
    let angular_z_pack = pack_z_from_low_merge(product_t_ru_lo, product_t_at);
    let quadratic_linear_at = vector_madd(ca_b, inertia_cross_at_b, quadratic_at_a);
    let ql_ru_hi = merge_hi(quadratic_linear_right, quadratic_linear_up);

    // The native half vector is loaded at 0x66ec from 0x8225a9f0.
    let half = [0.5_f32; 4];
    let angular_min_unscaled = vector_min(input.angular_packet_b, angular_outer_plus);
    let angular_xy = vector_add(angular_x_pack, angular_y_pack);
    let angular_max_unscaled = vector_max(input.angular_packet_a, angular_outer_minus);
    let linear_x_pack = pack_x_from_high_merge(ql_ru_hi, quadratic_linear_at);
    let angular_denominator = vector_add(angular_z_pack, angular_xy);
    let linear_y_pack = pack_y_from_high_merge(ql_ru_hi, quadratic_linear_at);
    let linear_z_pack = pack_z_from_low_merge(ql_ru_lo, quadratic_linear_at);
    let linear_x_mass = vector_add(linear_x_pack, input.inverse_mass_sum);
    let linear_yz = vector_add(linear_z_pack, linear_y_pack);

    // 0x6720: first and unconditional final estimate.
    let final_angular = backend.reciprocal_estimate(
        SimulationJointReciprocalSite::FinalAngular,
        angular_denominator,
    )?;

    // 0x6724 then 0x6728: both occur before the second estimate call.
    let linear_denominator = vector_add(linear_yz, linear_x_mass);
    let half_angular = vector_mul0(final_angular, half);

    // 0x672c: second and unconditional final estimate.
    let final_linear = backend.reciprocal_estimate(
        SimulationJointReciprocalSite::FinalLinear,
        linear_denominator,
    )?;

    // 0x6730..0x6754, in native source order.
    let angular_min = vector_mul0(angular_min_unscaled, half_angular);
    let t_y_scaled = vector_mul0(t_y_pack, half_angular);
    let t_z_scaled = vector_mul0(t_z_pack, half_angular);
    let t_x_scaled = vector_mul0(t_x_pack, half_angular);
    let l_y_scaled = vector_mul0(l_y_pack, final_linear);
    let position_max = vector_mul0(input.position_packet_b, final_linear);
    let position_min = vector_mul0(input.position_packet_a, final_linear);
    let l_z_scaled = vector_mul0(l_z_pack, final_linear);
    let l_x_scaled = vector_mul0(l_x_pack, final_linear);
    let angular_max = vector_mul0(angular_max_unscaled, half_angular);

    Ok(JointEffectiveMassSuffixOutput {
        linear_transpose: [l_x_scaled, l_y_scaled, l_z_scaled],
        angular_transpose: [t_x_scaled, t_y_scaled, t_z_scaled],
        position_max,
        position_min,
        angular_max,
        angular_min,
    })
}

fn pack_x(right: Vector, up: Vector, at: Vector) -> Vector {
    [right[0], up[0], at[0], at[1]]
}

fn pack_y(right: Vector, up: Vector, at: Vector) -> Vector {
    [right[1], up[1], at[1], at[0]]
}

fn pack_x_from_high_merge(right_up_hi: Vector, at: Vector) -> Vector {
    [right_up_hi[0], right_up_hi[1], at[0], at[1]]
}

fn pack_y_from_high_merge(right_up_hi: Vector, at: Vector) -> Vector {
    [right_up_hi[2], right_up_hi[3], at[1], at[0]]
}

fn pack_z_from_low_merge(right_up_lo: Vector, at: Vector) -> Vector {
    [right_up_lo[0], right_up_lo[1], at[2], at[0]]
}

fn merge_hi(left: Vector, right: Vector) -> Vector {
    [left[0], right[0], left[1], right[1]]
}

fn merge_lo(left: Vector, right: Vector) -> Vector {
    [left[2], right[2], left[3], right[3]]
}

fn splat(value: f32) -> Vector {
    [value; 4]
}

fn vector_add(left: Vector, right: Vector) -> Vector {
    std::array::from_fn(|lane| left[lane] + right[lane])
}

fn vector_sub(left: Vector, right: Vector) -> Vector {
    std::array::from_fn(|lane| left[lane] - right[lane])
}

fn vector_mul0(multiplier: Vector, multiplicand: Vector) -> Vector {
    std::array::from_fn(|lane| multiplier[lane].mul_add(multiplicand[lane], 0.0))
}

fn vector_madd(multiplier: Vector, multiplicand: Vector, addend: Vector) -> Vector {
    std::array::from_fn(|lane| multiplier[lane].mul_add(multiplicand[lane], addend[lane]))
}

fn vector_max(left: Vector, right: Vector) -> Vector {
    std::array::from_fn(|lane| left[lane].max(right[lane]))
}

fn vector_min(left: Vector, right: Vector) -> Vector {
    std::array::from_fn(|lane| left[lane].min(right[lane]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Failure;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Event {
        site: SimulationJointReciprocalSite,
        input: [u32; 4],
    }

    #[derive(Default)]
    struct HostBackend {
        events: Vec<Event>,
        fail_at: Option<SimulationJointReciprocalSite>,
    }

    impl SimulationJointNumericBackend for HostBackend {
        type Error = Failure;

        fn sum3(
            &mut self,
            _site: crate::simulation_joint_numeric::SimulationJointSumSite,
            _left: [f32; 3],
            _right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            unreachable!("the effective-mass suffix has no reduction site")
        }

        fn reciprocal_estimate(
            &mut self,
            site: SimulationJointReciprocalSite,
            denominator: Vector,
        ) -> Result<Vector, Self::Error> {
            self.events.push(Event {
                site,
                input: bits4(denominator),
            });
            if self.fail_at == Some(site) {
                return Err(Failure);
            }
            Ok(denominator.map(f32::recip))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            _site: crate::simulation_joint_numeric::SimulationJointReciprocalSqrtSite,
            _input: Vector,
        ) -> Result<Vector, Self::Error> {
            unreachable!("the effective-mass suffix has no reciprocal-square-root site")
        }
    }

    #[test]
    fn cone_arc_matches_corrected_half_scaled_literal_and_estimate_order() {
        let mut backend = HostBackend::default();
        let output = build_joint_effective_mass_suffix(cone_arc_input(), &mut backend)
            .expect("host reciprocal backend succeeds");

        assert_eq!(
            output_words(output),
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
            backend.events,
            [
                Event {
                    site: SimulationJointReciprocalSite::FinalAngular,
                    input: [0x3fad_4451, 0x4030_72b5, 0x3ff9_8d8c, 0x407c_dc9b],
                },
                Event {
                    site: SimulationJointReciprocalSite::FinalLinear,
                    input: [0x408c_2a0e, 0x408d_7831, 0x408f_894b, 0x4093_e319],
                },
            ]
        );
    }

    #[test]
    fn locked_locked_matches_route_dependent_angular_literal() {
        let mut backend = HostBackend::default();
        let output = build_joint_effective_mass_suffix(locked_locked_input(), &mut backend)
            .expect("host reciprocal backend succeeds");

        assert_eq!(
            output_words(output),
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
                0x3e3e_7e4e,
                0x3d63_4f7a,
                0x3e29_8979,
                0x3ea7_01ae,
                0x3dc4_cad7,
                0x3ddc_08df,
                0xbd61_44e5,
                0x3df0_f66f,
                0x3e73_e21c,
                0x3ddc_08df,
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
                0xbe1e_a477,
                0xba58_8543,
                0xbdb8_19f4,
                0x3f86_cf18,
                0xbe1e_a477,
                0xba58_8543,
            ]
        );
        assert_eq!(
            backend.events,
            [
                Event {
                    site: SimulationJointReciprocalSite::FinalAngular,
                    input: [0x4003_e95f, 0x3f37_54dc, 0x3fea_ccbd, 0x404b_46b2],
                },
                Event {
                    site: SimulationJointReciprocalSite::FinalLinear,
                    input: [0x408c_2a0e, 0x408d_7831, 0x408f_894b, 0x4093_e319],
                },
            ]
        );
    }

    #[test]
    fn estimate_failures_retain_exact_prefix_and_return_no_output() {
        let mut fail_first = HostBackend {
            events: Vec::new(),
            fail_at: Some(SimulationJointReciprocalSite::FinalAngular),
        };
        assert!(matches!(
            build_joint_effective_mass_suffix(cone_arc_input(), &mut fail_first),
            Err(Failure)
        ));
        assert_eq!(
            fail_first.events,
            [Event {
                site: SimulationJointReciprocalSite::FinalAngular,
                input: [0x3fad_4451, 0x4030_72b5, 0x3ff9_8d8c, 0x407c_dc9b],
            }]
        );

        let mut fail_second = HostBackend {
            events: Vec::new(),
            fail_at: Some(SimulationJointReciprocalSite::FinalLinear),
        };
        assert!(matches!(
            build_joint_effective_mass_suffix(cone_arc_input(), &mut fail_second),
            Err(Failure)
        ));
        assert_eq!(
            fail_second.events,
            [
                Event {
                    site: SimulationJointReciprocalSite::FinalAngular,
                    input: [0x3fad_4451, 0x4030_72b5, 0x3ff9_8d8c, 0x407c_dc9b],
                },
                Event {
                    site: SimulationJointReciprocalSite::FinalLinear,
                    input: [0x408c_2a0e, 0x408d_7831, 0x408f_894b, 0x4093_e319],
                },
            ]
        );
    }

    fn cone_arc_input() -> JointEffectiveMassSuffixInput {
        JointEffectiveMassSuffixInput {
            inverse_inertia_a: [
                v([0x3fd9_999a, 0x3e05_1eb8, 0xbda3_d70a, 0x400c_cccd]),
                v([0x3e05_1eb8, 0x3fb3_3333, 0x3d23_d70a, 0x400c_cccd]),
                v([0xbda3_d70a, 0x3d23_d70a, 0x3db8_51ec, 0x400c_cccd]),
            ],
            inverse_inertia_b: [
                v([0x3f99_999a, 0xbdb8_51ec, 0x3e2e_147b, 0x3fe6_6666]),
                v([0xbdb8_51ec, 0x3f86_6666, 0x3d8f_5c29, 0x3fe6_6666]),
                v([0x3e2e_147b, 0x3d8f_5c29, 0xbde1_47ae, 0x3fe6_6666]),
            ],
            attachment_cross_linear_a: [
                v([0xbc38_81af, 0x3e3f_3d2d, 0x3ed8_c765, 0x34c1_5a4c]),
                v([0xbe08_b7de, 0xbdb7_8886, 0x3e70_9892, 0xb3ac_5348]),
                v([0xbe89_3b81, 0xbec0_a4cb, 0x3d82_d0a9, 0xb2da_52c0]),
            ],
            attachment_cross_linear_b: [
                v([0xbe89_8b4b, 0x3ede_b788, 0xbe17_2858, 0x326c_5900]),
                v([0xbefb_b6d8, 0xbe77_88d0, 0xbe5b_0166, 0x32d3_3940]),
                v([0x3c6d_ca63, 0x3eba_6fe6, 0xbc55_b42c, 0xb331_6040]),
            ],
            angular_axes: [
                v([0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe, 0x3f2d_684e]),
                v([0xbf06_fe20, 0x3f69_13ac, 0xbebd_f2c7, 0xb298_160b]),
                v([0x3f53_06b7, 0x3e52_88ad, 0xbf2a_c6c6, 0x25e8_bd68]),
            ],
            linear_axes: linear_axes(),
            inertia_t_right: v([0x3fa9_0c08, 0x3fb1_96c0, 0x3da9_6854, 0x40d2_9433]),
            inertia_t_up_a: v([0xbf3f_96f2, 0x3f98_7ad2, 0x3d39_326e, 0x3cda_f1a7]),
            inertia_t_up_b: v([0xbf47_1d1e, 0x3f7a_3b58, 0x3c74_1505, 0x3cb3_2299]),
            inertia_t_at_a: v([0x3fbd_9fe1, 0x3ebc_9414, 0xbdf1_2c52, 0x3f4c_57da]),
            inertia_t_at_b: v([0x3f5b_7665, 0x3dc2_8c25, 0x3e69_6079, 0x3f27_3098]),
            inertia_cross_right_a: v([0xbceb_6b46, 0x3e8d_c9f4, 0x3d3e_5e82, 0x3fa8_a5dd]),
            inertia_cross_right_b_xy: v([0xbeb9_18f4, 0x3ef6_3b52, 0xbc79_5aa6, 0x3e99_4fa1]),
            inertia_cross_up_a: v([0xbe83_cc96, 0xbe08_9f72, 0x3ce7_5d0b, 0x3cd5_27c3]),
            inertia_cross_at_a_x: v([0xbee9_4b8f, 0xbd0e_b8c3, 0x3caf_a853, 0xbf16_f4a8]),
            inertia_cross_at_b_x: v([0x3c8e_aca2, 0xbaab_358f, 0x3b21_b295, 0x3cd6_02f2]),
            inverse_mass_sum: [4.0; 4],
            angular_projection: v([0x3b0e_8049, 0xbb13_0767, 0xbc0b_4e0e, 0x3b0e_8049]),
            angular_control_step: v([0x3b23_d70b, 0xba98_ead7, 0xba98_ead7, 0x0000_0000]),
            angular_trial_plus: v([0x3b99_2baa, 0xbb5f_7cd2, 0xbc1e_6b69, 0x3b0e_8049]),
            angular_packet_a: v([0xff7f_ffff, 0x3f5e_0cba, 0xff7f_ffff, 0xff7f_ffff]),
            angular_packet_b: v([0x4012_5f18, 0x7f7f_ffff, 0x7f7f_ffff, 0x7f7f_ffff]),
            position_packet_a: v([0xc02c_9c33, 0x401e_704b, 0xbe08_66a0, 0xbe6e_abc8]),
            position_packet_b: v([0xbfa6_0534, 0x405e_704b, 0x3fd5_5992, 0x3e6f_3218]),
        }
    }

    fn locked_locked_input() -> JointEffectiveMassSuffixInput {
        let mut input = cone_arc_input();
        input.angular_axes = [
            v([0x3f12_4c79, 0x3f2e_b7cc, 0xbe68_2739, 0x3f12_4c79]),
            v([0xbe80_1e13, 0x3eef_331f, 0x3e2c_900a, 0xbe80_1e13]),
            v([0x3f2e_b7cc, 0x3eb4_7ecb, 0x3f5f_afb9, 0x3f2e_b7cc]),
        ];
        input.inertia_t_right = v([0x3fd5_040b, 0x3fd5_c36b, 0x3e06_2fbe, 0x4083_7d3c]);
        input.inertia_t_up_a = v([0xbec1_9b34, 0x3f20_d679, 0x3d5c_a962, 0x3f59_1993]);
        input.inertia_t_up_b = v([0xbea0_99e3, 0x3f06_5d6f, 0xbce8_6e58, 0x3f31_a08f]);
        input.inertia_t_at_a = v([0x3f91_6dcd, 0x3f1e_01eb, 0x3d1c_3a5c, 0x4086_611a]);
        input.inertia_t_at_b = v([0x3f6f_90e3, 0x3ebd_630c, 0x3d36_9f41, 0x405b_e4b6]);
        input.angular_projection = v([0xbbab_ed77, 0x3aad_ae75, 0x3b36_3639, 0xbbab_ed77]);
        input.angular_trial_plus = v([0xbb34_03e3, 0x3926_1cf1, 0x3ad3_819b, 0xbbab_ed77]);
        input.angular_packet_a = v([0xbebd_ba38, 0x3fc1_1575, 0xbf11_8134, 0xff7f_ffff]);
        input.angular_packet_b = v([0xbebd_ba38, 0x3fc1_1575, 0xbf11_8134, 0x7f7f_ffff]);
        input
    }

    fn linear_axes() -> [Vector; 3] {
        [
            v([0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33, 0x3f7a_c486]),
            v([0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a, 0xbeed_00f2]),
            v([0x3e47_c7e9, 0x3cdf_bd86, 0x3f7a_c486, 0x3eef_6e14]),
        ]
    }

    fn output_words(output: JointEffectiveMassSuffixOutput) -> Vec<u32> {
        let mut words = Vec::new();
        for vector in output.linear_transpose {
            words.extend(bits4(vector));
        }
        for vector in output.angular_transpose {
            words.extend(bits4(vector));
        }
        words.extend(bits4(output.position_max));
        words.extend(bits4(output.position_min));
        words.extend(bits4(output.angular_max));
        words.extend(bits4(output.angular_min));
        words
    }

    fn v(words: [u32; 4]) -> Vector {
        words.map(f32::from_bits)
    }

    fn bits4(vector: Vector) -> [u32; 4] {
        vector.map(f32::to_bits)
    }
}
