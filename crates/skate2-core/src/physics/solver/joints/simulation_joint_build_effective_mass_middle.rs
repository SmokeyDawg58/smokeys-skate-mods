//! Value-only translation of `JointJacobian::Build` 0x82ae646c..0x82ae65dc.
//!
//! This pure ordinary-finite slice completes the cross products and inertia
//! tensor paths. It owns no record, body, pool, RF, or publication authority.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "private stage awaits effective-mass composition")
)]
#![allow(
    clippy::large_types_passed_by_value,
    clippy::similar_names,
    clippy::struct_field_names,
    clippy::too_many_lines,
    reason = "source-shaped names and copied boundaries preserve native chronology"
)]
#![deny(unsafe_code)]

use crate::simulation_joint_numeric::SimulationJointVector;

type Vec4 = SimulationJointVector;

#[derive(Clone, Copy)]
pub(crate) struct InertiaRows {
    pub(crate) row0: Vec4,
    pub(crate) row1: Vec4,
    pub(crate) row2: Vec4,
}

#[derive(Clone, Copy)]
pub(crate) struct AxisRows {
    pub(crate) right: Vec4,
    pub(crate) up: Vec4,
    pub(crate) at: Vec4,
}

/// Live operands of a cross-product completion whose first fused product was
/// issued before this slice.
#[derive(Clone, Copy)]
pub(crate) struct PendingCross {
    pub(crate) attachment_zxy: Vec4,
    pub(crate) axis_yzx: Vec4,
    pub(crate) first_product: Vec4,
}

#[derive(Clone, Copy)]
pub(crate) struct MiddleInput {
    pub(crate) inertia_a: InertiaRows,
    pub(crate) inertia_b: InertiaRows,
    pub(crate) linear_axes: AxisRows,
    pub(crate) angular_axes: AxisRows,
    pub(crate) cross_right_a: Vec4,
    pub(crate) cross_right_b: Vec4,
    pub(crate) cross_up_a: PendingCross,
    pub(crate) cross_up_b: PendingCross,
    pub(crate) cross_at_a: PendingCross,
    pub(crate) cross_at_b: PendingCross,
    pub(crate) a_tr_x_term: Vec4,
    pub(crate) attachment_a: Vec4,
    pub(crate) body_a_rf_word: u32,
    pub(crate) limit_170: Vec4,
    pub(crate) limit_160: Vec4,
    pub(crate) time_step: Vec4,
    pub(crate) bias_base: Vec4,
}

#[derive(Clone, Copy)]
pub(crate) struct AttachmentCrosses {
    pub(crate) a_right: Vec4,
    pub(crate) b_right: Vec4,
    pub(crate) a_up: Vec4,
    pub(crate) b_up: Vec4,
    pub(crate) a_at: Vec4,
    pub(crate) b_at: Vec4,
}

#[derive(Clone, Copy)]
pub(crate) struct AngularTensorProducts {
    pub(crate) a_right: Vec4,
    pub(crate) b_right: Vec4,
    pub(crate) sum_right: Vec4,
    pub(crate) a_up: Vec4,
    pub(crate) b_up: Vec4,
    pub(crate) a_at: Vec4,
    pub(crate) b_at: Vec4,
}

#[derive(Clone, Copy)]
pub(crate) struct CrossTensorHandoff {
    pub(crate) a_right: Vec4,
    pub(crate) b_right_xy: Vec4,
    pub(crate) a_up: Vec4,
    pub(crate) a_at_x: Vec4,
    pub(crate) b_at_x: Vec4,
}

#[derive(Clone, Copy)]
pub(crate) struct ContinuationValues {
    pub(crate) ta_saved: Vec4,
    pub(crate) cross_at_a_saved: Vec4,
    pub(crate) linear_right_up_xy: Vec4,
    pub(crate) angular_right_up_xy: Vec4,
    pub(crate) inverse_mass_a: Vec4,
    pub(crate) inverse_mass_b: Vec4,
    pub(crate) linear_inverse_mass: Vec4,
    pub(crate) limit_170: Vec4,
    pub(crate) limit_160: Vec4,
    pub(crate) limit_170_dt: Vec4,
    pub(crate) bias_advanced: Vec4,
}

#[derive(Clone, Copy)]
pub(crate) struct ObservedRecordWrites {
    pub(crate) at_000: Vec4,
    pub(crate) at_0c0: Vec4,
    pub(crate) at_120: Vec4,
    pub(crate) at_130: Vec4,
}

#[derive(Clone, Copy)]
pub(crate) struct MiddleOutput {
    pub(crate) crosses: AttachmentCrosses,
    pub(crate) angular: AngularTensorProducts,
    pub(crate) cross_tensor: CrossTensorHandoff,
    pub(crate) continuation: ContinuationValues,
    pub(crate) record_writes: ObservedRecordWrites,
}

#[inline]
fn fused(a: Vec4, multiplier: Vec4, addend: Vec4) -> Vec4 {
    std::array::from_fn(|lane| a[lane].mul_add(multiplier[lane], addend[lane]))
}

#[inline]
fn fused_negative(a: Vec4, multiplier: Vec4, addend: Vec4) -> Vec4 {
    std::array::from_fn(|lane| (-a[lane]).mul_add(multiplier[lane], addend[lane]))
}

#[inline]
fn add(a: Vec4, b: Vec4) -> Vec4 {
    std::array::from_fn(|lane| a[lane] + b[lane])
}

#[inline]
fn multiply(a: Vec4, b: Vec4) -> Vec4 {
    std::array::from_fn(|lane| a[lane] * b[lane])
}

#[inline]
fn broadcast(value: f32) -> Vec4 {
    [value; 4]
}

#[inline]
fn tensor_x(row0: Vec4, scalar_x: f32) -> Vec4 {
    fused(row0, broadcast(scalar_x), [0.0; 4])
}

#[inline]
fn tensor_y(row1: Vec4, scalar_y: f32, x_term: Vec4) -> Vec4 {
    fused(row1, broadcast(scalar_y), x_term)
}

#[inline]
fn tensor_z(row2: Vec4, scalar_z: f32, xy_term: Vec4) -> Vec4 {
    fused(row2, broadcast(scalar_z), xy_term)
}

#[inline]
fn complete_cross(pending: PendingCross) -> Vec4 {
    fused_negative(
        pending.attachment_zxy,
        pending.axis_yzx,
        pending.first_product,
    )
}

/// Source-ordered ordinary-finite translation of
/// `JointJacobian::Build` 0x82ae646c..0x82ae65dc.
pub(crate) fn build_effective_mass_middle(input: MiddleInput) -> MiddleOutput {
    let zero = [0.0; 4];

    // 646c, 6474, 6484, 6498: complete A-up and B-up/B-at crosses around the
    // first body-B angular product. Independent operations retain native order.
    let cross_up_a = complete_cross(input.cross_up_a);
    let b_tr_x = fused(
        input.inertia_b.row0,
        broadcast(input.angular_axes.right[0]),
        zero,
    );
    let cross_up_b = complete_cross(input.cross_up_b);
    let cross_at_b = complete_cross(input.cross_at_b);

    // 64a0, 64a8..652c.
    let a_tr_xy = tensor_y(
        input.inertia_a.row1,
        input.angular_axes.right[1],
        input.a_tr_x_term,
    );
    let cross_at_a = complete_cross(input.cross_at_a);
    let a_tu_x = tensor_x(input.inertia_a.row0, input.angular_axes.up[0]);
    let a_ta_x = tensor_x(input.inertia_a.row0, input.angular_axes.at[0]);
    let b_ta_x = tensor_x(input.inertia_b.row0, input.angular_axes.at[0]);
    let a_cross_right_x = tensor_x(input.inertia_a.row0, input.cross_right_a[0]);
    let b_tu_x = tensor_x(input.inertia_b.row0, input.angular_axes.up[0]);
    let b_tr_xy = tensor_y(input.inertia_b.row1, input.angular_axes.right[1], b_tr_x);
    let b_cross_right_x = tensor_x(input.inertia_b.row0, input.cross_right_b[0]);
    let a_cross_up_x = tensor_x(input.inertia_a.row0, cross_up_a[0]);
    let a_tu_xy = tensor_y(input.inertia_a.row1, input.angular_axes.up[1], a_tu_x);
    let a_ta_xy = tensor_y(input.inertia_a.row1, input.angular_axes.at[1], a_ta_x);
    let b_ta_xy = tensor_y(input.inertia_b.row1, input.angular_axes.at[1], b_ta_x);
    let b_tu_xy = tensor_y(input.inertia_b.row1, input.angular_axes.up[1], b_tu_x);
    let a_cross_right_xy = tensor_y(
        input.inertia_a.row1,
        input.cross_right_a[1],
        a_cross_right_x,
    );
    let b_cross_right_xy = tensor_y(
        input.inertia_b.row1,
        input.cross_right_b[1],
        b_cross_right_x,
    );
    let a_cross_up_xy = tensor_y(input.inertia_a.row1, cross_up_a[1], a_cross_up_x);

    // 6534..659c: finish the paths that are complete at the 65dc boundary.
    let b_tr = tensor_z(input.inertia_b.row2, input.angular_axes.right[2], b_tr_xy);
    let a_tr = tensor_z(input.inertia_a.row2, input.angular_axes.right[2], a_tr_xy);
    let a_cross_right = tensor_z(
        input.inertia_a.row2,
        input.cross_right_a[2],
        a_cross_right_xy,
    );
    let b_ta = tensor_z(input.inertia_b.row2, input.angular_axes.at[2], b_ta_xy);
    let a_ta = tensor_z(input.inertia_a.row2, input.angular_axes.at[2], a_ta_xy);
    let a_tu = tensor_z(input.inertia_a.row2, input.angular_axes.up[2], a_tu_xy);
    let b_tu = tensor_z(input.inertia_b.row2, input.angular_axes.up[2], b_tu_xy);
    let a_cross_at_x = tensor_x(input.inertia_a.row0, cross_at_a[0]);
    let a_cross_up = tensor_z(input.inertia_a.row2, cross_up_a[2], a_cross_up_xy);
    let sum_tr = add(a_tr, b_tr);

    // 65a4..65dc: the final middle-slice products and packed continuation rows.
    let limit_170_dt = multiply(input.limit_170, input.time_step);
    let b_cross_at_x = tensor_x(input.inertia_b.row0, cross_at_b[0]);
    let inverse_mass_a = broadcast(input.inertia_a.row0[3]);
    let inverse_mass_b = broadcast(input.inertia_b.row0[3]);
    let bias_advanced = fused(input.limit_160, input.time_step, input.bias_base);
    let linear_inverse_mass = add(inverse_mass_a, inverse_mass_b);

    let attachment_a_with_rf_word = [
        input.attachment_a[0],
        input.attachment_a[1],
        input.attachment_a[2],
        f32::from_bits(input.body_a_rf_word),
    ];

    MiddleOutput {
        crosses: AttachmentCrosses {
            a_right: input.cross_right_a,
            b_right: input.cross_right_b,
            a_up: cross_up_a,
            b_up: cross_up_b,
            a_at: cross_at_a,
            b_at: cross_at_b,
        },
        angular: AngularTensorProducts {
            a_right: a_tr,
            b_right: b_tr,
            sum_right: sum_tr,
            a_up: a_tu,
            b_up: b_tu,
            a_at: a_ta,
            b_at: b_ta,
        },
        cross_tensor: CrossTensorHandoff {
            a_right: a_cross_right,
            b_right_xy: b_cross_right_xy,
            a_up: a_cross_up,
            a_at_x: a_cross_at_x,
            b_at_x: b_cross_at_x,
        },
        continuation: ContinuationValues {
            ta_saved: input.angular_axes.at,
            cross_at_a_saved: cross_at_a,
            linear_right_up_xy: [
                input.linear_axes.right[0],
                input.linear_axes.up[0],
                input.linear_axes.right[1],
                input.linear_axes.up[1],
            ],
            angular_right_up_xy: [
                input.angular_axes.right[0],
                input.angular_axes.up[0],
                input.angular_axes.right[1],
                input.angular_axes.up[1],
            ],
            inverse_mass_a,
            inverse_mass_b,
            linear_inverse_mass,
            limit_170: input.limit_170,
            limit_160: input.limit_160,
            limit_170_dt,
            bias_advanced,
        },
        record_writes: ObservedRecordWrites {
            at_000: attachment_a_with_rf_word,
            at_0c0: input.linear_axes.right,
            at_120: input.inertia_a.row0,
            at_130: input.inertia_a.row1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(words: [u32; 4]) -> Vec4 {
        words.map(f32::from_bits)
    }

    fn bits(value: Vec4) -> [u32; 4] {
        value.map(f32::to_bits)
    }

    fn rows_for_state(state: u32, active: InertiaRows) -> InertiaRows {
        if state & 4 == 4 {
            active
        } else {
            InertiaRows {
                row0: [0.0; 4],
                row1: [0.0; 4],
                row2: [0.0; 4],
            }
        }
    }

    fn fixture(state_a: u32, state_b: u32) -> MiddleInput {
        let inertia_a = InertiaRows {
            row0: v([0x3fd9_999a, 0x3e05_1eb8, 0xbda3_d70a, 0x400c_cccd]),
            row1: v([0x3e05_1eb8, 0x3fb3_3333, 0x3d23_d70a, 0x400c_cccd]),
            row2: v([0xbda3_d70a, 0x3d23_d70a, 0x3db8_51ec, 0x400c_cccd]),
        };
        let inertia_b = InertiaRows {
            row0: v([0x3f99_999a, 0xbdb8_51ec, 0x3e2e_147b, 0x3fe6_6666]),
            row1: v([0xbdb8_51ec, 0x3f86_6666, 0x3d8f_5c29, 0x3fe6_6666]),
            row2: v([0x3e2e_147b, 0x3d8f_5c29, 0xbde1_47ae, 0x3fe6_6666]),
        };
        let attachment_a_zxy = v([0x3e05_1eb8, 0x3ed1_eb85, 0xbe8a_3d71, 0x4105_c28f]);
        let attachment_b_zxy = v([0x3f10_0000, 0xbe84_f766, 0x3cf9_db21, 0x415d_999a]);
        let up_yzx = v([0x3f60_be20, 0x3d93_0d5a, 0xbeed_00f2, 0xbeed_00f2]);
        let at_yzx = v([0x3cdf_bd86, 0x3f7a_c486, 0x3e47_c7e9, 0x3eef_6e14]);

        MiddleInput {
            inertia_a: rows_for_state(state_a, inertia_a),
            inertia_b: rows_for_state(state_b, inertia_b),
            linear_axes: AxisRows {
                right: v([0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33, 0x3f7a_c486]),
                up: v([0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a, 0xbeed_00f2]),
                at: v([0x3e47_c7e9, 0x3cdf_bd86, 0x3f7a_c486, 0x3eef_6e14]),
            },
            angular_axes: AxisRows {
                right: v([0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe, 0x3f2d_684e]),
                up: v([0xbf06_fe20, 0x3f69_13ac, 0xbebd_f2c7, 0xb298_160b]),
                at: v([0x3f53_06b7, 0x3e52_88ad, 0xbf2a_c6c6, 0x25e8_bd68]),
            },
            cross_right_a: v([0xbc38_81af, 0x3e3f_3d2d, 0x3ed8_c765, 0x34c1_5a4c]),
            cross_right_b: v([0xbe89_8b4b, 0x3ede_b788, 0xbe17_2858, 0x326c_5900]),
            cross_up_a: PendingCross {
                attachment_zxy: attachment_a_zxy,
                axis_yzx: up_yzx,
                first_product: v([0xbc9e_d0fb, 0xbd76_7bdc, 0x3eb8_49fb, 0xc077_ab3a]),
            },
            cross_up_b: PendingCross {
                attachment_zxy: attachment_b_zxy,
                axis_yzx: up_yzx,
                first_product: v([0x3b0f_85dc, 0xbe85_5088, 0xbe69_7673, 0xc0cd_2805]),
            },
            cross_at_a: PendingCross {
                attachment_zxy: attachment_a_zxy,
                axis_yzx: at_yzx,
                first_product: v([0xbe87_6a20, 0x3ccf_c5aa, 0x3c37_7792, 0x407a_344f]),
            },
            cross_at_b: PendingCross {
                attachment_zxy: attachment_b_zxy,
                axis_yzx: at_yzx,
                first_product: v([0x3cf4_bfcd, 0x3de0_c0e6, 0xbbe8_6be4, 0x40cf_41b0]),
            },
            a_tr_x_term: if state_a & 4 == 4 {
                v([0x3f39_b369, 0x3d63_35dc, 0xbd0b_d260, 0x3f70_5196])
            } else {
                [0.0; 4]
            },
            attachment_a: v([0x3ed1_eb85, 0xbe8a_3d71, 0x3e05_1eb8, 0x4105_c28f]),
            body_a_rf_word: 0x1111_1111,
            limit_170: v([0x3e19_999a, 0xbd8f_5c29, 0xbd8f_5c29, 0x0000_0000]),
            limit_160: v([0x3e19_999a, 0xbd8f_5c29, 0xbd8f_5c29, 0x0000_0000]),
            time_step: v([0x3c88_8889; 4]),
            bias_base: v([0x3b0e_8049, 0xbb13_0767, 0xbc0b_4e0e, 0x3b0e_8049]),
        }
    }

    fn checkpoint(output: MiddleOutput, input: MiddleInput) -> Vec<Vec4> {
        vec![
            output.crosses.a_right,
            output.crosses.b_right,
            output.crosses.a_up,
            output.crosses.b_up,
            output.crosses.b_at,
            output.crosses.a_at,
            output.angular.b_right,
            output.angular.a_right,
            output.cross_tensor.a_right,
            output.angular.b_at,
            output.angular.a_at,
            output.angular.a_up,
            output.angular.b_up,
            output.cross_tensor.a_up,
            output.angular.sum_right,
            output.cross_tensor.b_at_x,
            output.cross_tensor.b_right_xy,
            output.cross_tensor.a_at_x,
            output.continuation.linear_inverse_mass,
            input.inertia_a.row0,
            input.inertia_a.row1,
            input.inertia_a.row2,
            input.inertia_b.row0,
            input.inertia_b.row1,
            input.inertia_b.row2,
            output.record_writes.at_000,
            output.record_writes.at_0c0,
            output.record_writes.at_120,
            output.record_writes.at_130,
        ]
    }

    fn fnv64(vectors: &[Vec4]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for vector in vectors {
            for word in vector.map(f32::to_bits) {
                for byte in word.to_be_bytes() {
                    hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        hash
    }

    #[test]
    fn canonical_boundary_matches_every_published_literal() {
        let input = fixture(4, 4);
        let output = build_effective_mass_middle(input);

        assert_eq!(
            bits(output.crosses.a_up),
            [0xbe08_b7de, 0xbdb7_8886, 0x3e70_9892, 0xb3ac_5348]
        );
        assert_eq!(
            bits(output.crosses.b_up),
            [0xbefb_b6d8, 0xbe77_88d0, 0xbe5b_0166, 0x32d3_3940]
        );
        assert_eq!(
            bits(output.crosses.b_at),
            [0x3c6d_ca63, 0x3eba_6fe6, 0xbc55_b42c, 0xb331_6040]
        );
        assert_eq!(
            bits(output.crosses.a_at),
            [0xbe89_3b81, 0xbec0_a4cb, 0x3d82_d0a9, 0xb2da_52c0]
        );

        assert_eq!(
            bits(output.angular.b_right),
            [0x3f14_f180, 0x3f10_8da5, 0x3d09_ac53, 0x403d_8561]
        );
        assert_eq!(
            bits(output.angular.a_right),
            [0x3f3d_268f, 0x3f52_9fdb, 0x3d49_2454, 0x4067_a305]
        );
        assert_eq!(
            bits(output.angular.b_at),
            [0x3f5b_7665, 0x3dc2_8c25, 0x3e69_6079, 0x3f27_3098]
        );
        assert_eq!(
            bits(output.angular.a_at),
            [0x3fbd_9fe1, 0x3ebc_9414, 0xbdf1_2c52, 0x3f4c_57da]
        );
        assert_eq!(
            bits(output.angular.a_up),
            [0xbf3f_96f2, 0x3f98_7ad2, 0x3d39_326e, 0x3cda_f1a7]
        );
        assert_eq!(
            bits(output.angular.b_up),
            [0xbf47_1d1e, 0x3f7a_3b58, 0x3c74_1505, 0x3cb3_2299]
        );
        assert_eq!(
            bits(output.angular.sum_right),
            [0x3fa9_0c08, 0x3fb1_96c0, 0x3da9_6854, 0x40d2_9433]
        );

        assert_eq!(
            bits(output.cross_tensor.a_right),
            [0xbceb_6b46, 0x3e8d_c9f4, 0x3d3e_5e82, 0x3fa8_a5dd]
        );
        assert_eq!(
            bits(output.cross_tensor.b_right_xy),
            [0xbeb9_18f4, 0x3ef6_3b52, 0xbc79_5aa6, 0x3e99_4fa1]
        );
        assert_eq!(
            bits(output.cross_tensor.a_up),
            [0xbe83_cc96, 0xbe08_9f72, 0x3ce7_5d0b, 0x3cd5_27c3]
        );
        assert_eq!(
            bits(output.cross_tensor.a_at_x),
            [0xbee9_4b8f, 0xbd0e_b8c3, 0x3caf_a853, 0xbf16_f4a8]
        );
        assert_eq!(
            bits(output.cross_tensor.b_at_x),
            [0x3c8e_aca2, 0xbaab_358f, 0x3b21_b295, 0x3cd6_02f2]
        );

        assert_eq!(
            bits(output.continuation.ta_saved),
            [0x3f53_06b7, 0x3e52_88ad, 0xbf2a_c6c6, 0x25e8_bd68]
        );
        assert_eq!(
            bits(output.continuation.cross_at_a_saved),
            [0xbe89_3b81, 0xbec0_a4cb, 0x3d82_d0a9, 0xb2da_52c0]
        );
        assert_eq!(
            bits(output.continuation.linear_right_up_xy),
            [0x3f5b_a726, 0xbeed_00f2, 0x3eef_6e14, 0x3f60_be20]
        );
        assert_eq!(
            bits(output.continuation.angular_right_up_xy),
            [0x3eda_78b7, 0xbf06_fe20, 0x3f07_414d, 0x3f69_13ac]
        );
        assert_eq!(bits(output.continuation.inverse_mass_a), [0x400c_cccd; 4]);
        assert_eq!(bits(output.continuation.inverse_mass_b), [0x3fe6_6666; 4]);
        assert_eq!(
            bits(output.continuation.linear_inverse_mass),
            [0x4080_0000; 4]
        );
        assert_eq!(
            bits(output.continuation.limit_170_dt),
            [0x3b23_d70b, 0xba98_ead7, 0xba98_ead7, 0x0000_0000]
        );
        assert_eq!(
            bits(output.continuation.bias_advanced),
            [0x3b99_2baa, 0xbb5f_7cd2, 0xbc1e_6b69, 0x3b0e_8049]
        );

        assert_eq!(
            bits(output.record_writes.at_000),
            [0x3ed1_eb85, 0xbe8a_3d71, 0x3e05_1eb8, 0x1111_1111]
        );
        assert_eq!(
            bits(output.record_writes.at_0c0),
            [0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33, 0x3f7a_c486]
        );
        assert_eq!(
            bits(output.record_writes.at_120),
            [0x3fd9_999a, 0x3e05_1eb8, 0xbda3_d70a, 0x400c_cccd]
        );
        assert_eq!(
            bits(output.record_writes.at_130),
            [0x3e05_1eb8, 0x3fb3_3333, 0x3d23_d70a, 0x400c_cccd]
        );
        assert_eq!(fnv64(&checkpoint(output, input)), 0xc68e_40ab_f858_1c12);
    }

    #[test]
    fn prior_body_state_gate_discriminators_match_the_oracle() {
        for (state_a, state_b, expected) in [
            (4, 4, 0xc68e_40ab_f858_1c12),
            (0, 4, 0xa26f_4bed_3870_6815),
            (4, 0, 0x4db6_1091_81f4_542b),
            (0, 0, 0x2d9e_2ced_accd_981d),
        ] {
            let input = fixture(state_a, state_b);
            let output = build_effective_mass_middle(input);
            assert_eq!(fnv64(&checkpoint(output, input)), expected);
        }
    }

    #[test]
    fn prior_gate_is_bit_two_not_equality_to_four() {
        let four = fixture(4, 4);
        let five = fixture(5, 5);
        assert_eq!(
            checkpoint(build_effective_mass_middle(four), four)
                .into_iter()
                .map(bits)
                .collect::<Vec<_>>(),
            checkpoint(build_effective_mass_middle(five), five)
                .into_iter()
                .map(bits)
                .collect::<Vec<_>>()
        );
    }
}
