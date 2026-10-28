//! Standalone typed translation of `JointJacobian::Build` 0x82ae6328..0x82ae6468.
//!
//! This value-only stage consumes copied body rows and the preceding prediction
//! boundary. It does not claim a native layout, output-record ownership, pool or
//! RF authority, publication, or any part of the continuation at 0x82ae646c.
//! Native exceptional VMX/FPSCR behavior remains outside this ordinary-finite
//! host-`mul_add` translation. This slice has no estimate/reduction callback.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "private stage awaits effective-mass composition")
)]
#![allow(
    clippy::large_types_passed_by_value,
    clippy::similar_names,
    clippy::struct_field_names,
    reason = "source-shaped names and copied boundaries preserve native chronology"
)]
#![deny(unsafe_code)]

use crate::simulation_joint_numeric::SimulationJointVector;

type JointVector = SimulationJointVector;

const ZERO: JointVector = [0.0; 4];
const ZXY_SELF_MASK: [u8; 16] = [
    0x08, 0x09, 0x0a, 0x0b, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0c, 0x0d, 0x0e, 0x0f,
];
const YZX_SELF_MASK: [u8; 16] = [
    0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x00, 0x01, 0x02, 0x03, 0x0c, 0x0d, 0x0e, 0x0f,
];
const PACK_Z_MASK: [u8; 16] = [
    0x08, 0x09, 0x0a, 0x0b, 0x18, 0x19, 0x1a, 0x1b, 0x10, 0x11, 0x12, 0x13, 0x0c, 0x0d, 0x0e, 0x0f,
];
const PACK_Y_MASK: [u8; 16] = [
    0x04, 0x05, 0x06, 0x07, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x0c, 0x0d, 0x0e, 0x0f,
];
pub(crate) const XYZ_SELECT_MASK: [u32; 4] = [u32::MAX, u32::MAX, u32::MAX, 0];

/// Three exact packed rows copied from one native body.
///
/// W of `center_of_mass_and_rf_word` and `split_inertia_and_state` is a raw word
/// carried in an `f32` lane only to preserve its bits; it is never arithmetic.
#[derive(Clone, Copy)]
pub(crate) struct JointEffectiveMassBodyRows {
    pub(crate) center_of_mass_and_rf_word: JointVector,
    pub(crate) full_inertia_and_inverse_mass: JointVector,
    pub(crate) split_inertia_and_state: JointVector,
}

/// Full and already-live permuted forms of one attachment at 0x82ae6328.
#[derive(Clone, Copy)]
pub(crate) struct JointAttachmentRows {
    pub(crate) full: JointVector,
    pub(crate) zxy: JointVector,
    pub(crate) yzx: JointVector,
}

#[derive(Clone, Copy)]
pub(crate) struct JointEffectiveMassPrefixInput {
    pub(crate) body_a: JointEffectiveMassBodyRows,
    pub(crate) body_b: JointEffectiveMassBodyRows,
    pub(crate) attachment_a: JointAttachmentRows,
    pub(crate) attachment_b: JointAttachmentRows,
    /// Linear Right/Up/At rows, physical v94/v93/v95 at entry.
    pub(crate) linear_axes: [JointVector; 3],
    /// Final angular Right/Up/At rows, physical v9/v10/v11 at entry.
    pub(crate) angular_axes: [JointVector; 3],
    pub(crate) position_packet_a: JointVector,
    pub(crate) position_packet_b: JointVector,
}

#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct JointSelectedInertia {
    pub(crate) full: JointVector,
    pub(crate) pack_z: JointVector,
    pub(crate) pack_y: JointVector,
}

#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct JointEffectiveMassCrossHandoff {
    pub(crate) body_a_cross_right: JointVector,
    pub(crate) body_b_cross_right: JointVector,
    /// The three exact operands entering the first excluded instruction 0x646c.
    pub(crate) body_a_attachment_zxy: JointVector,
    pub(crate) body_a_up_first: JointVector,
    pub(crate) up_yzx_for_body_a: JointVector,
    pub(crate) body_b_attachment_zxy: JointVector,
    pub(crate) body_b_at_first: JointVector,
    pub(crate) body_a_at_first: JointVector,
    pub(crate) body_b_up_first: JointVector,
    pub(crate) up_yzx_for_body_b: JointVector,
    pub(crate) at_yzx: JointVector,
}

/// Private values corresponding to the three output-record stores in the slice.
/// This is not an owned or published native record.
#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct JointEffectiveMassPrefixRecordWrites {
    pub(crate) row_0x10_body_b_attachment_and_rf_word: JointVector,
    pub(crate) row_0x20_zero: JointVector,
    pub(crate) row_0x30_zero: JointVector,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum JointEffectiveMassPrefixWriteSlot {
    Row0x20Zero,
    Row0x30Zero,
    Row0x10BodyBAttachmentAndRfWord,
}

/// Native private-record store chronology at 0x643c, 0x6444 and 0x6448.
pub(crate) const JOINT_EFFECTIVE_MASS_PREFIX_WRITE_ORDER: [JointEffectiveMassPrefixWriteSlot; 3] = [
    JointEffectiveMassPrefixWriteSlot::Row0x20Zero,
    JointEffectiveMassPrefixWriteSlot::Row0x30Zero,
    JointEffectiveMassPrefixWriteSlot::Row0x10BodyBAttachmentAndRfWord,
];

/// Complete semantic handoff live immediately after 0x82ae6468.
#[derive(Clone, Copy)]
pub(crate) struct JointEffectiveMassPrefixHandoff {
    pub(crate) body_a_inertia: JointSelectedInertia,
    pub(crate) body_b_inertia: JointSelectedInertia,
    pub(crate) cross: JointEffectiveMassCrossHandoff,
    pub(crate) body_a_full_inertia_times_right_x: JointVector,
    pub(crate) right_x: JointVector,
    pub(crate) up_x: JointVector,
    pub(crate) angular_axes: [JointVector; 3],
    pub(crate) packed_body_b_attachment_and_rf_word: JointVector,
    pub(crate) body_a_center_of_mass_and_rf_word: JointVector,
    pub(crate) body_a_attachment: JointVector,
    pub(crate) position_packet_a: JointVector,
    pub(crate) position_packet_b: JointVector,
    pub(crate) position_packet_a_copy: JointVector,
    pub(crate) position_packet_b_copy: JointVector,
    pub(crate) xyz_select_mask: [u32; 4],
    /// Scratch +0x50 after 0x6468, overwritten with body-B attachment zxy.
    pub(crate) scratch_attachment_b_zxy: JointVector,
    /// Scratch +0x60, retaining body-A attachment zxy.
    pub(crate) scratch_attachment_a_zxy: JointVector,
    pub(crate) record_writes: JointEffectiveMassPrefixRecordWrites,
}

/// Translates exactly the private value graph at 0x82ae6328..0x82ae6468.
pub(crate) fn build_joint_effective_mass_prefix(
    input: JointEffectiveMassPrefixInput,
) -> JointEffectiveMassPrefixHandoff {
    // 0x6328..0x637c: preserve the two independent raw bit-2 gates and the
    // interleaved basis permutations.
    let body_a_active = state_has_bit_two(input.body_a.split_inertia_and_state);
    let body_b_active = state_has_bit_two(input.body_b.split_inertia_and_state);
    let right_zxy = vector_permute(input.linear_axes[0], input.linear_axes[0], ZXY_SELF_MASK);
    let up_zxy = vector_permute(input.linear_axes[1], input.linear_axes[1], ZXY_SELF_MASK);
    let right_yzx = vector_permute(input.linear_axes[0], input.linear_axes[0], YZX_SELF_MASK);
    let at_zxy = vector_permute(input.linear_axes[2], input.linear_axes[2], ZXY_SELF_MASK);
    let up_yzx = vector_permute(input.linear_axes[1], input.linear_axes[1], YZX_SELF_MASK);
    // The fused operations remain in exact native source order. This slice has
    // no estimate/reduction callback and is therefore deliberately infallible.
    let body_a_right_first = vector_multiply_add(input.attachment_a.yzx, right_zxy, ZERO);
    // Native forms this final basis permutation at 0x6388, after the first
    // fused product at 0x6380.
    let at_yzx = vector_permute(input.linear_axes[2], input.linear_axes[2], YZX_SELF_MASK);
    let body_a_full = gate_vector(input.body_a.full_inertia_and_inverse_mass, body_a_active);
    let body_a_split = gate_vector(input.body_a.split_inertia_and_state, body_a_active);

    let body_b_right_first = vector_multiply_add(input.attachment_b.yzx, right_zxy, ZERO);
    let body_b_full = gate_vector(input.body_b.full_inertia_and_inverse_mass, body_b_active);
    let body_b_at_first = vector_multiply_add(input.attachment_b.yzx, at_zxy, ZERO);
    let body_b_split = gate_vector(input.body_b.split_inertia_and_state, body_b_active);
    let body_a_up_first = vector_multiply_add(input.attachment_a.yzx, up_zxy, ZERO);
    let body_a_at_first = vector_multiply_add(input.attachment_a.yzx, at_zxy, ZERO);
    let body_b_up_first = vector_multiply_add(input.attachment_b.yzx, up_zxy, ZERO);
    let body_a_cross_right =
        vector_negative_multiply_subtract(input.attachment_a.zxy, right_yzx, body_a_right_first);
    let body_b_cross_right =
        vector_negative_multiply_subtract(input.attachment_b.zxy, right_yzx, body_b_right_first);

    // 0x63e8..0x6408 literal byte-mask packing. Neither selector reads the
    // state W word from the gated split-inertia row.
    let body_a_pack_z = vector_permute(body_a_full, body_a_split, PACK_Z_MASK);
    let body_b_pack_z = vector_permute(body_b_full, body_b_split, PACK_Z_MASK);
    let body_a_pack_y = vector_permute(body_a_full, body_a_split, PACK_Y_MASK);
    let body_b_pack_y = vector_permute(body_b_full, body_b_split, PACK_Y_MASK);

    let right_x = [input.angular_axes[0][0]; 4];
    let up_x = [input.angular_axes[1][0]; 4];
    let packed_body_b_attachment_and_rf_word = vector_select_bits(
        input.body_b.center_of_mass_and_rf_word,
        input.attachment_b.full,
        XYZ_SELECT_MASK,
    );
    let body_a_full_inertia_times_right_x = vector_multiply_add(body_a_full, right_x, ZERO);

    JointEffectiveMassPrefixHandoff {
        body_a_inertia: JointSelectedInertia {
            full: body_a_full,
            pack_z: body_a_pack_z,
            pack_y: body_a_pack_y,
        },
        body_b_inertia: JointSelectedInertia {
            full: body_b_full,
            pack_z: body_b_pack_z,
            pack_y: body_b_pack_y,
        },
        cross: JointEffectiveMassCrossHandoff {
            body_a_cross_right,
            body_b_cross_right,
            body_a_attachment_zxy: input.attachment_a.zxy,
            body_a_up_first,
            up_yzx_for_body_a: up_yzx,
            body_b_attachment_zxy: input.attachment_b.zxy,
            body_b_at_first,
            body_a_at_first,
            body_b_up_first,
            up_yzx_for_body_b: up_yzx,
            at_yzx,
        },
        body_a_full_inertia_times_right_x,
        right_x,
        up_x,
        angular_axes: input.angular_axes,
        packed_body_b_attachment_and_rf_word,
        body_a_center_of_mass_and_rf_word: input.body_a.center_of_mass_and_rf_word,
        body_a_attachment: input.attachment_a.full,
        position_packet_a: input.position_packet_a,
        position_packet_b: input.position_packet_b,
        position_packet_a_copy: input.position_packet_a,
        position_packet_b_copy: input.position_packet_b,
        xyz_select_mask: XYZ_SELECT_MASK,
        scratch_attachment_b_zxy: input.attachment_b.zxy,
        scratch_attachment_a_zxy: input.attachment_a.zxy,
        record_writes: JointEffectiveMassPrefixRecordWrites {
            row_0x10_body_b_attachment_and_rf_word: packed_body_b_attachment_and_rf_word,
            row_0x20_zero: ZERO,
            row_0x30_zero: ZERO,
        },
    }
}

fn state_has_bit_two(split_inertia_and_state: JointVector) -> bool {
    split_inertia_and_state[3].to_bits() & 4 == 4
}

fn gate_vector(value: JointVector, active: bool) -> JointVector {
    let mask = [if active { u32::MAX } else { 0 }; 4];
    vector_select_bits(ZERO, value, mask)
}

fn vector_select_bits(a: JointVector, b: JointVector, mask: [u32; 4]) -> JointVector {
    std::array::from_fn(|lane| {
        f32::from_bits((a[lane].to_bits() & !mask[lane]) | (b[lane].to_bits() & mask[lane]))
    })
}

/// Ordinary-finite host policy for native `vmaddfp`.
fn vector_multiply_add(
    multiplier: JointVector,
    multiplicand: JointVector,
    addend: JointVector,
) -> JointVector {
    std::array::from_fn(|lane| multiplier[lane].mul_add(multiplicand[lane], addend[lane]))
}

/// Ordinary-finite host policy for native `vnmsubfp`: addend - a*b.
fn vector_negative_multiply_subtract(
    multiplier: JointVector,
    multiplicand: JointVector,
    addend: JointVector,
) -> JointVector {
    std::array::from_fn(|lane| (-multiplier[lane]).mul_add(multiplicand[lane], addend[lane]))
}

fn vector_permute(a: JointVector, b: JointVector, mask: [u8; 16]) -> JointVector {
    let mut source = [0_u8; 32];
    for (lane, value) in a.into_iter().chain(b).enumerate() {
        let bytes = value.to_bits().to_be_bytes();
        let base = lane * 4;
        source[base..base + 4].copy_from_slice(&bytes);
    }
    let selected = mask.map(|index| source[usize::from(index & 31)]);
    std::array::from_fn(|lane| {
        let base = lane * 4;
        f32::from_bits(u32::from_be_bytes([
            selected[base],
            selected[base + 1],
            selected[base + 2],
            selected[base + 3],
        ]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_both_active_handoff_is_literal() {
        let output = build_joint_effective_mass_prefix(fixture());
        assert_eq!(
            JOINT_EFFECTIVE_MASS_PREFIX_WRITE_ORDER,
            [
                JointEffectiveMassPrefixWriteSlot::Row0x20Zero,
                JointEffectiveMassPrefixWriteSlot::Row0x30Zero,
                JointEffectiveMassPrefixWriteSlot::Row0x10BodyBAttachmentAndRfWord,
            ]
        );

        let words = boundary_words(output);
        assert_eq!(words, literal_boundary_words());
        assert_eq!(fnv1a64(&words), 0xdadc_539a_224b_26c6);
        assert_eq!(
            bits4(output.cross.body_a_attachment_zxy),
            [0x3e05_1eb8, 0x3ed1_eb85, 0xbe8a_3d71, 0x4105_c28f]
        );
        assert_eq!(
            bits4(output.cross.body_a_up_first),
            [0xbc9e_d0fb, 0xbd76_7bdc, 0x3eb8_49fb, 0xc077_ab3a]
        );
        assert_eq!(
            bits4(output.cross.up_yzx_for_body_a),
            [0x3f60_be20, 0x3d93_0d5a, 0xbeed_00f2, 0xbeed_00f2]
        );
        let first_excluded_result = std::array::from_fn(|lane| {
            (-output.cross.body_a_attachment_zxy[lane]).mul_add(
                output.cross.up_yzx_for_body_a[lane],
                output.cross.body_a_up_first[lane],
            )
        });
        assert_eq!(
            bits4(first_excluded_result),
            [0xbe08_b7de, 0xbdb7_8886, 0x3e70_9892, 0xb3ac_5348]
        );
    }

    #[test]
    fn body_state_bit_two_gates_are_independent_and_exact() {
        let baseline = run(fixture());

        let mut a_inactive = fixture();
        a_inactive.body_a.split_inertia_and_state[3] = f(0);
        let a_output = run(a_inactive);
        assert_eq!(fnv1a64(&boundary_words(a_output)), 0x6811_ae12_db9a_c3d5);
        assert_eq!(bits4(a_output.body_a_inertia.full), [0; 4]);
        assert_eq!(bits4(a_output.body_a_inertia.pack_z), [0; 4]);
        assert_eq!(bits4(a_output.body_a_inertia.pack_y), [0; 4]);
        assert_eq!(bits4(a_output.body_a_full_inertia_times_right_x), [0; 4]);
        assert_eq!(a_output.body_b_inertia, baseline.body_b_inertia);
        assert_eq!(a_output.cross, baseline.cross);
        assert_eq!(a_output.record_writes, baseline.record_writes);

        let mut b_inactive = fixture();
        b_inactive.body_b.split_inertia_and_state[3] = f(0);
        let b_output = run(b_inactive);
        assert_eq!(fnv1a64(&boundary_words(b_output)), 0xb496_64ea_4f44_f002);
        assert_eq!(bits4(b_output.body_b_inertia.full), [0; 4]);
        assert_eq!(bits4(b_output.body_b_inertia.pack_z), [0; 4]);
        assert_eq!(bits4(b_output.body_b_inertia.pack_y), [0; 4]);
        assert_eq!(b_output.body_a_inertia, baseline.body_a_inertia);
        assert_eq!(b_output.cross, baseline.cross);
        assert_eq!(b_output.record_writes, baseline.record_writes);

        for active_word in [5, u32::MAX] {
            let mut active = fixture();
            active.body_a.split_inertia_and_state[3] = f(active_word);
            let output = run(active);
            assert_eq!(output.body_a_inertia, baseline.body_a_inertia);
            assert_eq!(
                bits4(output.body_a_full_inertia_times_right_x),
                bits4(baseline.body_a_full_inertia_times_right_x)
            );
        }
    }

    #[test]
    fn literal_masks_preserve_words_and_arbitrary_bits() {
        let a = [
            f(0x0123_4567),
            f(0x89ab_cdef),
            f(0x7654_3210),
            f(0x8000_0000),
        ];
        let b = [
            f(0xfedc_ba98),
            f(0x0bad_f00d),
            f(0xaaaa_5555),
            f(0x7fc0_1234),
        ];
        assert_eq!(
            bits4(vector_permute(a, a, ZXY_SELF_MASK)),
            [0x7654_3210, 0x0123_4567, 0x89ab_cdef, 0x8000_0000]
        );
        assert_eq!(
            bits4(vector_permute(a, a, YZX_SELF_MASK)),
            [0x89ab_cdef, 0x7654_3210, 0x0123_4567, 0x8000_0000]
        );
        assert_eq!(
            bits4(vector_permute(a, b, PACK_Z_MASK)),
            [0x7654_3210, 0xaaaa_5555, 0xfedc_ba98, 0x8000_0000]
        );
        assert_eq!(
            bits4(vector_permute(a, b, PACK_Y_MASK)),
            [0x89ab_cdef, 0x0bad_f00d, 0xaaaa_5555, 0x8000_0000]
        );
        assert_eq!(
            bits4(vector_select_bits(a, b, XYZ_SELECT_MASK)),
            [0xfedc_ba98, 0x0bad_f00d, 0xaaaa_5555, 0x8000_0000]
        );
    }

    fn run(input: JointEffectiveMassPrefixInput) -> JointEffectiveMassPrefixHandoff {
        build_joint_effective_mass_prefix(input)
    }

    fn fixture() -> JointEffectiveMassPrefixInput {
        JointEffectiveMassPrefixInput {
            body_a: JointEffectiveMassBodyRows {
                center_of_mass_and_rf_word: v([0x3f99_999a, 0xbf4c_cccd, 0x3ecc_cccd, 0x1111_1111]),
                full_inertia_and_inverse_mass: v([
                    0x3fd9_999a,
                    0x3e05_1eb8,
                    0xbda3_d70a,
                    0x400c_cccd,
                ]),
                split_inertia_and_state: v([0x3db8_51ec, 0x3fb3_3333, 0x3d23_d70a, 0x0000_0004]),
            },
            body_b: JointEffectiveMassBodyRows {
                center_of_mass_and_rf_word: v([0xbf8c_cccd, 0x3f19_999a, 0x3fa6_6666, 0x2222_2222]),
                full_inertia_and_inverse_mass: v([
                    0x3f99_999a,
                    0xbdb8_51ec,
                    0x3e2e_147b,
                    0x3fe6_6666,
                ]),
                split_inertia_and_state: v([0xbde1_47ae, 0x3f86_6666, 0x3d8f_5c29, 0x0000_0004]),
            },
            attachment_a: JointAttachmentRows {
                full: v([0x3ed1_eb85, 0xbe8a_3d71, 0x3e05_1eb8, 0x4105_c28f]),
                zxy: v([0x3e05_1eb8, 0x3ed1_eb85, 0xbe8a_3d71, 0x4105_c28f]),
                yzx: v([0xbe8a_3d71, 0x3e05_1eb8, 0x3ed1_eb85, 0x4105_c28f]),
            },
            attachment_b: JointAttachmentRows {
                full: v([0xbe84_f766, 0x3cf9_db21, 0x3f10_0000, 0x415d_999a]),
                zxy: v([0x3f10_0000, 0xbe84_f766, 0x3cf9_db21, 0x415d_999a]),
                yzx: v([0x3cf9_db21, 0x3f10_0000, 0xbe84_f766, 0x415d_999a]),
            },
            linear_axes: [
                v([0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33, 0x3f7a_c486]),
                v([0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a, 0xbeed_00f2]),
                v([0x3e47_c7e9, 0x3cdf_bd86, 0x3f7a_c486, 0x3eef_6e14]),
            ],
            angular_axes: [
                v([0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe, 0x3f2d_684e]),
                v([0xbf06_fe20, 0x3f69_13ac, 0xbebd_f2c7, 0xb298_160b]),
                v([0x3f53_06b7, 0x3e52_88ad, 0xbf2a_c6c6, 0x25e8_bd68]),
            ],
            position_packet_a: v([0xc02c_9c33, 0x401e_704b, 0xbe08_66a0, 0xbe6e_abc8]),
            position_packet_b: v([0xbfa6_0534, 0x405e_704b, 0x3fd5_5992, 0x3e6f_3218]),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn literal_boundary_words() -> Vec<[u32; 4]> {
        vec![
            [0x3fd9_999a, 0x3e05_1eb8, 0xbda3_d70a, 0x400c_cccd],
            [0xbda3_d70a, 0x3d23_d70a, 0x3db8_51ec, 0x400c_cccd],
            [0x3e05_1eb8, 0x3fb3_3333, 0x3d23_d70a, 0x400c_cccd],
            [0x3f99_999a, 0xbdb8_51ec, 0x3e2e_147b, 0x3fe6_6666],
            [0x3e2e_147b, 0x3d8f_5c29, 0xbde1_47ae, 0x3fe6_6666],
            [0xbdb8_51ec, 0x3f86_6666, 0x3d8f_5c29, 0x3fe6_6666],
            [0xbc38_81af, 0x3e3f_3d2d, 0x3ed8_c765, 0x34c1_5a4c],
            [0xbe89_8b4b, 0x3ede_b788, 0xbe17_2858, 0x326c_5900],
            [0x3e05_1eb8, 0x3ed1_eb85, 0xbe8a_3d71, 0x4105_c28f],
            [0xbc9e_d0fb, 0xbd76_7bdc, 0x3eb8_49fb, 0xc077_ab3a],
            [0x3f60_be20, 0x3d93_0d5a, 0xbeed_00f2, 0xbeed_00f2],
            [0x3f10_0000, 0xbe84_f766, 0x3cf9_db21, 0x415d_999a],
            [0x3cf4_bfcd, 0x3de0_c0e6, 0xbbe8_6be4, 0x40cf_41b0],
            [0xbe87_6a20, 0x3ccf_c5aa, 0x3c37_7792, 0x407a_344f],
            [0x3b0f_85dc, 0xbe85_5088, 0xbe69_7673, 0xc0cd_2805],
            [0x3f60_be20, 0x3d93_0d5a, 0xbeed_00f2, 0xbeed_00f2],
            [0x3cdf_bd86, 0x3f7a_c486, 0x3e47_c7e9, 0x3eef_6e14],
            [0x3f39_b369, 0x3d63_35dc, 0xbd0b_d260, 0x3f70_5196],
            [0x3eda_78b7; 4],
            [0xbf06_fe20; 4],
            [0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe, 0x3f2d_684e],
            [0xbf06_fe20, 0x3f69_13ac, 0xbebd_f2c7, 0xb298_160b],
            [0x3f53_06b7, 0x3e52_88ad, 0xbf2a_c6c6, 0x25e8_bd68],
            [0xbe84_f766, 0x3cf9_db21, 0x3f10_0000, 0x2222_2222],
            [0x3f99_999a, 0xbf4c_cccd, 0x3ecc_cccd, 0x1111_1111],
            [0x3ed1_eb85, 0xbe8a_3d71, 0x3e05_1eb8, 0x4105_c28f],
            [0xc02c_9c33, 0x401e_704b, 0xbe08_66a0, 0xbe6e_abc8],
            [0xbfa6_0534, 0x405e_704b, 0x3fd5_5992, 0x3e6f_3218],
            [0xc02c_9c33, 0x401e_704b, 0xbe08_66a0, 0xbe6e_abc8],
            [0xbfa6_0534, 0x405e_704b, 0x3fd5_5992, 0x3e6f_3218],
            [u32::MAX, u32::MAX, u32::MAX, 0],
            [0x3f10_0000, 0xbe84_f766, 0x3cf9_db21, 0x415d_999a],
            [0x3e05_1eb8, 0x3ed1_eb85, 0xbe8a_3d71, 0x4105_c28f],
            [0xbe84_f766, 0x3cf9_db21, 0x3f10_0000, 0x2222_2222],
            [0; 4],
            [0; 4],
        ]
    }

    fn boundary_words(output: JointEffectiveMassPrefixHandoff) -> Vec<[u32; 4]> {
        vec![
            bits4(output.body_a_inertia.full),
            bits4(output.body_a_inertia.pack_z),
            bits4(output.body_a_inertia.pack_y),
            bits4(output.body_b_inertia.full),
            bits4(output.body_b_inertia.pack_z),
            bits4(output.body_b_inertia.pack_y),
            bits4(output.cross.body_a_cross_right),
            bits4(output.cross.body_b_cross_right),
            bits4(output.cross.body_a_attachment_zxy),
            bits4(output.cross.body_a_up_first),
            bits4(output.cross.up_yzx_for_body_a),
            bits4(output.cross.body_b_attachment_zxy),
            bits4(output.cross.body_b_at_first),
            bits4(output.cross.body_a_at_first),
            bits4(output.cross.body_b_up_first),
            bits4(output.cross.up_yzx_for_body_b),
            bits4(output.cross.at_yzx),
            bits4(output.body_a_full_inertia_times_right_x),
            bits4(output.right_x),
            bits4(output.up_x),
            bits4(output.angular_axes[0]),
            bits4(output.angular_axes[1]),
            bits4(output.angular_axes[2]),
            bits4(output.packed_body_b_attachment_and_rf_word),
            bits4(output.body_a_center_of_mass_and_rf_word),
            bits4(output.body_a_attachment),
            bits4(output.position_packet_a),
            bits4(output.position_packet_b),
            bits4(output.position_packet_a_copy),
            bits4(output.position_packet_b_copy),
            output.xyz_select_mask,
            bits4(output.scratch_attachment_b_zxy),
            bits4(output.scratch_attachment_a_zxy),
            bits4(output.record_writes.row_0x10_body_b_attachment_and_rf_word),
            bits4(output.record_writes.row_0x20_zero),
            bits4(output.record_writes.row_0x30_zero),
        ]
    }

    fn fnv1a64(words: &[[u32; 4]]) -> u64 {
        words
            .iter()
            .flatten()
            .fold(0xcbf2_9ce4_8422_2325, |hash, word| {
                word.to_be_bytes().into_iter().fold(hash, |inner, byte| {
                    (inner ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
                })
            })
    }

    fn f(bits: u32) -> f32 {
        f32::from_bits(bits)
    }

    fn v(bits: [u32; 4]) -> JointVector {
        bits.map(f)
    }

    fn bits4(value: JointVector) -> [u32; 4] {
        value.map(f32::to_bits)
    }
}
