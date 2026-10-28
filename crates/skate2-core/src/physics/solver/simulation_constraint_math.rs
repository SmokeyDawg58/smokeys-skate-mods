//! Shared, evidence-bounded constraint arithmetic.
//!
//! This module contains pure value operations only. Pool ownership, solver-row
//! capabilities, and native estimate/reduction behavior remain with each
//! constraint builder or solver.

/// Composes a rigid-body quaternion with a constraint-frame quaternion.
///
/// Native references: `DriveJacobian::Build` at
/// `0x82ae4c98..0x82ae4d58` and `JointJacobian::Build` at
/// `0x82ae596c..0x82ae59b0`. The two builders use the same VMX graph. This is
/// not a textbook quaternion-multiply rewrite: the YZX permutations and fused
/// operations preserve the source evaluation order for ordinary finite `f32`
/// inputs.
///
/// `dot3` is the caller's result for the native `vmsum3fp128` site. Keeping it
/// explicit prevents this helper from claiming unproven Xenon horizontal-
/// reduction rounding. Exceptional values, denormals, signed zero, and VMX
/// exception behavior remain hardware boundaries.
pub(crate) fn compose_body_frame_quaternion_from_dot3(
    body: [f32; 4],
    frame: [f32; 4],
    dot3: f32,
) -> [f32; 4] {
    let frame_yzxw = yzxw(frame);
    let products: [f32; 4] = std::array::from_fn(|lane| body[lane] * frame_yzxw[lane]);
    let body_yzxw = yzxw(body);
    let rotated_cross =
        std::array::from_fn(|lane| (-body_yzxw[lane]).mul_add(frame[lane], products[lane]));
    let cross = yzxw(rotated_cross);
    let first: [f32; 4] = std::array::from_fn(|lane| frame[lane].mul_add(body[3], cross[lane]));
    let mut result: [f32; 4] =
        std::array::from_fn(|lane| body[lane].mul_add(frame[3], first[lane]));
    result[3] = body[3] * frame[3] - dot3;
    result
}

/// Produces the four value rows from `Jacobian_RQD::Create`.
///
/// Native range `0x82ae3750..0x82ae3930` is shared by Drive and Joint Build.
/// Each explicit operation preserves the recovered VMX order for ordinary
/// finite inputs; this function makes no ABI or native-struct layout claim.
#[allow(clippy::many_single_char_names, clippy::similar_names)]
pub(crate) fn create_jacobian_rqd_rows(a: [f32; 4], b: [f32; 4]) -> [[f32; 4]; 4] {
    let ax = b.map(|lane| lane * a[0]);
    let aw = b.map(|lane| lane * a[3]);
    let ay = b.map(|lane| lane * a[1]);
    let az = b.map(|lane| lane * a[2]);

    let jwi_x = ((aw[3] + ax[0]) - ay[1]) - az[2];
    let jwi_y = ((ay[0] + ax[1]) + aw[2]) + az[3];
    let jwi_z = ((ax[2] + az[0]) - aw[1]) - ay[3];
    let jwi_w = ((aw[0] - ax[3]) + az[1]) - ay[2];

    let jwj_x = ((ay[0] + ax[1]) - aw[2]) - az[3];
    let jwj_y = ((aw[3] + ay[1]) - az[2]) - ax[0];
    let jwj_z = ((ay[2] + az[1]) + aw[0]) + ax[3];
    let jwj_w = ((aw[1] - ay[3]) + ax[2]) - az[0];

    // 82ae3848/38a8 overwrite the temporary registers before the row-K
    // inserts at3898/38cc; they no longer contain the row-I intermediates.
    let jwk_x = ((ax[2] + az[0]) + ay[3]) + aw[1];
    let jwk_y = ((az[1] + ay[2]) - aw[0]) - ax[3];
    let jwk_z = ((aw[3] + az[2]) - ax[0]) - ay[1];
    let jwk_w = ((aw[2] - az[3]) + ay[0]) - ax[1];

    let jwr_x = ((ax[3] - aw[0]) + az[1]) - ay[2];
    let jwr_y = ((ay[3] - aw[1]) + ax[2]) - az[0];
    let jwr_z = ((az[3] - aw[2]) + ay[0]) - ax[1];
    let jwr_w = ((aw[3] + az[2]) + ay[1]) + ax[0];

    [
        [jwi_x, jwi_y, jwi_z, jwi_w],
        [jwj_x, jwj_y, jwj_z, jwj_w],
        [jwk_x, jwk_y, jwk_z, jwk_w],
        [jwr_x, jwr_y, jwr_z, jwr_w],
    ]
}

/// Packs a semantic XYZ vector with one source-defined fourth word.
pub(crate) const fn pack_xyz_word(xyz: [f32; 3], word: f32) -> [f32; 4] {
    [xyz[0], xyz[1], xyz[2], word]
}

/// Source-role spelling for `vmaddfp d,a,b,c`, whose result is `a*c+b`.
pub(crate) fn horus_madd(
    multiplier: [f32; 4],
    multiplicand: [f32; 4],
    addend: [f32; 4],
) -> [f32; 4] {
    std::array::from_fn(|lane| multiplier[lane].mul_add(multiplicand[lane], addend[lane]))
}

/// Source-role spelling for `vnmsubfp d,a,b,c`, whose result is `b-a*c`.
pub(crate) fn horus_nmsub(
    multiplier: [f32; 4],
    multiplicand: [f32; 4],
    addend: [f32; 4],
) -> [f32; 4] {
    std::array::from_fn(|lane| (-multiplier[lane]).mul_add(multiplicand[lane], addend[lane]))
}

/// Preserves the fused YZX/ZXY cross-product schedule shared by Horus solvers.
pub(crate) fn horus_cross_add(left: [f32; 4], right: [f32; 4], addend: [f32; 4]) -> [f32; 4] {
    let partial = horus_madd(yzxw(left), zxyw(right), addend);
    horus_nmsub(zxyw(left), yzxw(right), partial)
}

/// Applies packed Right, Up, then At columns with one fused add per column.
pub(crate) fn horus_columns_from(
    columns: [[f32; 4]; 3],
    scalars: [f32; 4],
    addend: [f32; 4],
) -> [f32; 4] {
    let right = horus_madd(columns[0], [scalars[0]; 4], addend);
    let up = horus_madd(columns[1], [scalars[1]; 4], right);
    horus_madd(columns[2], [scalars[2]; 4], up)
}

/// Applies negated packed Right, Up, then At columns with fused operations.
pub(crate) fn horus_negative_columns_from(
    columns: [[f32; 4]; 3],
    scalars: [f32; 4],
    addend: [f32; 4],
) -> [f32; 4] {
    let right = horus_nmsub(columns[0], [scalars[0]; 4], addend);
    let up = horus_nmsub(columns[1], [scalars[1]; 4], right);
    horus_nmsub(columns[2], [scalars[2]; 4], up)
}

fn yzxw(value: [f32; 4]) -> [f32; 4] {
    [value[1], value[2], value[0], value[3]]
}

fn zxyw(value: [f32; 4]) -> [f32; 4] {
    [value[2], value[0], value[1], value[3]]
}

#[cfg(test)]
mod tests {
    use super::{compose_body_frame_quaternion_from_dot3, create_jacobian_rqd_rows};

    #[test]
    fn equal_identity_frames_have_identity_axes_and_zero_error() {
        let identity = [0.0, 0.0, 0.0, 1.0];
        assert_eq!(
            create_jacobian_rqd_rows(identity, identity).map(bits),
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]
            .map(bits)
        );
    }

    #[test]
    fn equal_quarter_turn_frames_retain_all_three_constraint_axes() {
        let half = f32::from_bits(0x3f35_04f3);
        let heading = [0.0, half, 0.0, half];
        assert_eq!(
            create_jacobian_rqd_rows(heading, heading).map(bits),
            [
                [0, 0, 0xbf7f_ffff, 0],
                [0, 0x3f7f_ffff, 0, 0],
                [0x3f7f_ffff, 0, 0, 0],
                [0, 0, 0, 0x3f7f_ffff],
            ]
        );
    }

    fn bits(value: [f32; 4]) -> [u32; 4] {
        value.map(f32::to_bits)
    }

    fn values(value: [u32; 4]) -> [f32; 4] {
        value.map(f32::from_bits)
    }

    #[test]
    fn composes_body_times_frame_in_native_operand_order() {
        let body = values([0x3f80_0000, 0x4000_0000, 0x4080_0000, 0x4100_0000]);
        let frame = values([0x4180_0000, 0x4200_0000, 0x4100_0000, 0x4280_0000]);
        let dot3 = f32::from_bits(0x42e0_0000);

        assert_eq!(
            bits(compose_body_frame_quaternion_from_dot3(body, frame, dot3)),
            [0x42a0_0000, 0x43dc_0000, 0x43a0_0000, 0x43c8_0000]
        );
        assert_eq!(
            bits(compose_body_frame_quaternion_from_dot3(frame, body, dot3)),
            [0x4398_0000, 0x43a4_0000, 0x43a0_0000, 0x43c8_0000]
        );
    }

    #[test]
    fn preserves_inner_native_fused_add_stage() {
        let body = values([0x3a29_f774, 0x437c_bc43, 0x3fd2_34c6, 0xc0f8_856d]);
        let frame = values([0x445c_27e8, 0x432b_6a27, 0x39bd_e15a, 0xbca7_40f1]);

        assert_eq!(
            bits(compose_body_frame_quaternion_from_dot3(
                body,
                frame,
                f32::from_bits(0x4729_3b1f),
            )),
            [0xc5de_84b2, 0x42db_8d32, 0xc859_592a, 0xc729_3af6]
        );
    }

    #[test]
    fn preserves_outer_native_fused_add_stage() {
        let body = values([0xbc1f_8ec8, 0x43f4_e387, 0xc5b5_4b3a, 0xbb46_e151]);
        let frame = values([0x4437_aa7c, 0xbfcc_03ab, 0xbc74_450e, 0x45b8_0915]);

        assert_eq!(
            bits(compose_body_frame_quaternion_from_dot3(body, frame, 0.0,)),
            [0xc611_8615, 0xc9a8_2d96, 0xcc03_b3e4, 0xc18e_f900]
        );
    }

    #[test]
    fn preserves_native_fused_negative_multiply_subtract() {
        let body = values([0x3fd6_459c, 0x3f48_a44b, 0xc1e8_2d2e, 0xbbb7_92b5]);
        let frame = values([0x3ee2_8490, 0x39d1_0eaf, 0x427d_6c42, 0x3f61_beb9]);

        assert_eq!(
            bits(compose_body_frame_quaternion_from_dot3(
                body,
                frame,
                f32::from_bits(0xc4e5_bf1f),
            )),
            [0x424c_901e, 0xc2ec_698f, 0xc1d2_585b, 0x44e5_bef7]
        );
    }
}
