//! Private typed final-record assembly for `JointJacobian::Build` 0x82ae6758..0x82ae67a8.
//!
//! The symbol ends at 0x82ae67a8 (exclusive): the last record store is at
//! 0x82ae678c, 0x6790..0x67a4 is epilogue, and 0x67a8 begins another function.
//! This helper returns an inert private image, not a pool/RF/publication handle.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaits the owning full Horus runtime")
)]
#![deny(unsafe_code)]

use crate::simulation_joint_numeric::SimulationJointVector;

type JointVector = SimulationJointVector;
pub(crate) type JointWords = [u32; 4];

const MASK_XYZ_X: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x10, 0x11, 0x12, 0x13,
];
const MASK_XYZ_Y: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x14, 0x15, 0x16, 0x17,
];
const MASK_XYZ_Z: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x18, 0x19, 0x1a, 0x1b,
];

/// Rows whose native stores have completed before the terminal assembly slice.
#[derive(Clone, Copy)]
pub(crate) struct JointFinalStableRows {
    pub(crate) attachment_a_and_rf_word: JointWords,
    pub(crate) attachment_b_and_rf_word: JointWords,
    pub(crate) linear_basis: [JointWords; 3],
    pub(crate) angular_basis: [JointWords; 3],
    pub(crate) inverse_inertia_a: [JointWords; 3],
    pub(crate) inverse_inertia_b: [JointWords; 3],
}

/// Fully scaled numeric values live at 0x82ae6758.
#[derive(Clone, Copy)]
pub(crate) struct JointFinalScaledRows {
    pub(crate) linear_right: JointVector,
    pub(crate) angular_right: JointVector,
    pub(crate) linear_up: JointVector,
    pub(crate) angular_up: JointVector,
    pub(crate) linear_at: JointVector,
    pub(crate) angular_at: JointVector,
    pub(crate) position_max: JointVector,
    pub(crate) position_min: JointVector,
    pub(crate) angular_max: JointVector,
    pub(crate) angular_min: JointVector,
}

/// Inert copied words and prepared rows required by final assembly.
#[derive(Clone, Copy)]
pub(crate) struct JointFinalRecordAssemblyInput {
    pub(crate) stable: JointFinalStableRows,
    pub(crate) scaled: JointFinalScaledRows,
    /// Scalar winner at record +0xcc, written natively at 0x82ae66dc.
    pub(crate) joint_word: u32,
    /// Transient loser at record +0x4c, written at 0x82ae6704.
    pub(crate) transient_spy_word: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum JointFinalStoreKind {
    Scalar,
    Vector,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct JointFinalStoreEvent {
    pub(crate) pc: u32,
    pub(crate) kind: JointFinalStoreKind,
    pub(crate) byte_offset: u16,
}

/// Private 0x180-byte/96-word final image plus the relevant winner chronology.
#[derive(Clone, Copy)]
pub(crate) struct JointFinalRecordAssembly {
    pub(crate) rows: [JointWords; 24],
    pub(crate) store_chronology: [JointFinalStoreEvent; 10],
}

/// Applies the exact terminal permutes and stores to an inert private image.
pub(crate) fn assemble_joint_final_record(
    input: &JointFinalRecordAssemblyInput,
) -> JointFinalRecordAssembly {
    let stable = input.stable;
    let scaled = input.scaled;
    let mut rows = [[0_u32; 4]; 24];

    // Earlier stores that remain final owners. Rows 2/3 remain exact zero from
    // the native clears and zero stores.
    rows[0] = stable.attachment_a_and_rf_word;
    rows[1] = stable.attachment_b_and_rf_word;
    rows[12] = stable.linear_basis[0];
    rows[13] = stable.linear_basis[1];
    rows[14] = stable.linear_basis[2];
    rows[15] = stable.angular_basis[0];
    rows[16] = stable.angular_basis[1];
    rows[17] = stable.angular_basis[2];
    rows[18] = stable.inverse_inertia_a[0];
    rows[19] = stable.inverse_inertia_a[1];
    rows[20] = stable.inverse_inertia_a[2];
    rows[21] = stable.inverse_inertia_b[0];
    rows[22] = stable.inverse_inertia_b[1];
    rows[23] = stable.inverse_inertia_b[2];

    // These two scalar stores precede the terminal slice but are retained to
    // freeze their opposite winner behavior.
    rows[12][3] = input.joint_word;
    rows[4][3] = input.transient_spy_word;

    // 0x6758: [T_at.xyz, angular_min.y].
    let angular_at_and_y_min = vector_permute(scaled.angular_at, scaled.angular_min, MASK_XYZ_Y);
    // 0x675c: direct four-lane T-right store.
    rows[5] = scaled.angular_right.map(f32::to_bits);
    // 0x6760 and 0x6764.
    let position_min_and_z_min =
        vector_permute(scaled.position_min, scaled.angular_min, MASK_XYZ_Z);
    let linear_at_and_x_min = vector_permute(scaled.linear_at, scaled.angular_min, MASK_XYZ_X);
    // 0x6768 then 0x676c. The latter full-vector store wins over spy at +0x4c.
    rows[9] = angular_at_and_y_min;
    rows[4] = scaled.linear_right.map(f32::to_bits);
    // 0x6770, 0x6774 and 0x6778.
    let linear_up_and_x_max = vector_permute(scaled.linear_up, scaled.angular_max, MASK_XYZ_X);
    let angular_up_and_y_max = vector_permute(scaled.angular_up, scaled.angular_max, MASK_XYZ_Y);
    let position_max_and_z_max =
        vector_permute(scaled.position_max, scaled.angular_max, MASK_XYZ_Z);
    // 0x677c..0x678c terminal store order.
    rows[11] = position_min_and_z_min;
    rows[8] = linear_at_and_x_min;
    rows[6] = linear_up_and_x_max;
    rows[7] = angular_up_and_y_max;
    rows[10] = position_max_and_z_max;

    JointFinalRecordAssembly {
        rows,
        store_chronology: STORE_CHRONOLOGY,
    }
}

pub(crate) const STORE_CHRONOLOGY: [JointFinalStoreEvent; 10] = [
    JointFinalStoreEvent {
        pc: 0x82ae_66dc,
        kind: JointFinalStoreKind::Scalar,
        byte_offset: 0x00cc,
    },
    JointFinalStoreEvent {
        pc: 0x82ae_6704,
        kind: JointFinalStoreKind::Scalar,
        byte_offset: 0x004c,
    },
    JointFinalStoreEvent {
        pc: 0x82ae_675c,
        kind: JointFinalStoreKind::Vector,
        byte_offset: 0x0050,
    },
    JointFinalStoreEvent {
        pc: 0x82ae_6768,
        kind: JointFinalStoreKind::Vector,
        byte_offset: 0x0090,
    },
    JointFinalStoreEvent {
        pc: 0x82ae_676c,
        kind: JointFinalStoreKind::Vector,
        byte_offset: 0x0040,
    },
    JointFinalStoreEvent {
        pc: 0x82ae_677c,
        kind: JointFinalStoreKind::Vector,
        byte_offset: 0x00b0,
    },
    JointFinalStoreEvent {
        pc: 0x82ae_6780,
        kind: JointFinalStoreKind::Vector,
        byte_offset: 0x0080,
    },
    JointFinalStoreEvent {
        pc: 0x82ae_6784,
        kind: JointFinalStoreKind::Vector,
        byte_offset: 0x0060,
    },
    JointFinalStoreEvent {
        pc: 0x82ae_6788,
        kind: JointFinalStoreKind::Vector,
        byte_offset: 0x0070,
    },
    JointFinalStoreEvent {
        pc: 0x82ae_678c,
        kind: JointFinalStoreKind::Vector,
        byte_offset: 0x00a0,
    },
];

fn vector_permute(a: JointVector, b: JointVector, mask: [u8; 16]) -> JointWords {
    let mut source = [0_u8; 32];
    for (lane, value) in a.into_iter().chain(b).enumerate() {
        let bytes = value.to_bits().to_be_bytes();
        let base = lane * 4;
        source[base..base + 4].copy_from_slice(&bytes);
    }
    let selected = mask.map(|index| source[usize::from(index & 31)]);
    std::array::from_fn(|lane| {
        let base = lane * 4;
        u32::from_be_bytes([
            selected[base],
            selected[base + 1],
            selected[base + 2],
            selected[base + 3],
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONE_ARC: [JointWords; 24] = [
        [0x3ed1_eb85, 0xbe8a_3d71, 0x3e05_1eb8, 0x1111_1111],
        [0xbe84_f766, 0x3cf9_db21, 0x3f10_0000, 0x2222_2222],
        [0; 4],
        [0; 4],
        [0x3e48_9715, 0xbdd6_7032, 0x3d32_281e, 0x3bc1_a723],
        [0x3e21_650d, 0xbdc3_dab8, 0x3e58_7a5e, 0x3cd5_2596],
        [0x3dda_a69f, 0x3e4b_5839, 0x3bc7_85d5, 0xb8fc_3959],
        [0x3e47_d687, 0x3e29_1479, 0x3d57_f913, 0x3e21_14a3],
        [0xbd2b_8ca6, 0x3c85_0d16, 0x3e5f_9fec, 0x3ae2_4ed5],
        [0x3e82_8306, 0xbd89_cb20, 0xbe2f_303b, 0x3e21_14a3],
        [0xbe97_9cb3, 0x3f49_428d, 0x3ebe_41ca, 0xbaf6_9731],
        [0xbf1d_a14a, 0x3f0f_5a7e, 0xbcf3_4619, 0xbb22_8324],
        [0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33, 0x1000_1000],
        [0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a, 0xbeed_00f2],
        [0x3e47_c7e9, 0x3cdf_bd86, 0x3f7a_c486, 0x3eef_6e14],
        [0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe, 0x3f2d_684e],
        [0xbf06_fe20, 0x3f69_13ac, 0xbebd_f2c7, 0xb298_160b],
        [0x3f53_06b7, 0x3e52_88ad, 0xbf2a_c6c6, 0x25e8_bd68],
        [0x3fd9_999a, 0x3e05_1eb8, 0xbda3_d70a, 0x400c_cccd],
        [0x3e05_1eb8, 0x3fb3_3333, 0x3d23_d70a, 0x400c_cccd],
        [0xbda3_d70a, 0x3d23_d70a, 0x3db8_51ec, 0x400c_cccd],
        [0x3f99_999a, 0xbdb8_51ec, 0x3e2e_147b, 0x3fe6_6666],
        [0xbdb8_51ec, 0x3f86_6666, 0x3d8f_5c29, 0x3fe6_6666],
        [0x3e2e_147b, 0x3d8f_5c29, 0xbde1_47ae, 0x3fe6_6666],
    ];

    const LOCKED_LOCKED: [JointWords; 24] = [
        CONE_ARC[0],
        CONE_ARC[1],
        CONE_ARC[2],
        CONE_ARC[3],
        CONE_ARC[4],
        [0x3e0d_f5e3, 0xbe32_e680, 0x3e3e_7e4e, 0x3d63_4f7a],
        [0x3dda_a69f, 0x3e4b_5839, 0x3bc7_85d5, 0xbdb8_19f4],
        [0x3e29_8979, 0x3ea7_01ae, 0x3dc4_cad7, 0x3f86_cf18],
        [0xbd2b_8ca6, 0x3c85_0d16, 0x3e5f_9fec, 0xbdb8_19f4],
        [0xbd61_44e5, 0x3df0_f66f, 0x3e73_e21c, 0x3f86_cf18],
        [0xbe97_9cb3, 0x3f49_428d, 0x3ebe_41ca, 0xbe1e_a477],
        [0xbf1d_a14a, 0x3f0f_5a7e, 0xbcf3_4619, 0xbe1e_a477],
        CONE_ARC[12],
        CONE_ARC[13],
        CONE_ARC[14],
        [0x3f12_4c79, 0x3f2e_b7cc, 0xbe68_2739, 0x3f12_4c79],
        [0xbe80_1e13, 0x3eef_331f, 0x3e2c_900a, 0xbe80_1e13],
        [0x3f2e_b7cc, 0x3eb4_7ecb, 0x3f5f_afb9, 0x3f2e_b7cc],
        CONE_ARC[18],
        CONE_ARC[19],
        CONE_ARC[20],
        CONE_ARC[21],
        CONE_ARC[22],
        CONE_ARC[23],
    ];

    #[test]
    fn cone_arc_and_locked_locked_match_complete_corrected_records() {
        let cone = assemble_joint_final_record(&input_from_record(&CONE_ARC, 7));
        assert_eq!(cone.rows, CONE_ARC);
        assert_eq!(fnv64(&cone.rows), 0xd1a5_dd73_5b04_e060);
        let locked = assemble_joint_final_record(&input_from_record(&LOCKED_LOCKED, 7));
        assert_eq!(locked.rows, LOCKED_LOCKED);
        assert_eq!(fnv64(&locked.rows), 0xf01b_59e5_f4a4_bc19);
    }

    #[test]
    fn exact_permute_and_store_chronology_is_frozen() {
        let result = assemble_joint_final_record(&input_from_record(&CONE_ARC, 7));
        assert_eq!(result.store_chronology, STORE_CHRONOLOGY);
        assert_eq!(
            result
                .store_chronology
                .map(|event| (event.pc, event.kind, event.byte_offset)),
            [
                (0x82ae_66dc, JointFinalStoreKind::Scalar, 0x0cc),
                (0x82ae_6704, JointFinalStoreKind::Scalar, 0x04c),
                (0x82ae_675c, JointFinalStoreKind::Vector, 0x050),
                (0x82ae_6768, JointFinalStoreKind::Vector, 0x090),
                (0x82ae_676c, JointFinalStoreKind::Vector, 0x040),
                (0x82ae_677c, JointFinalStoreKind::Vector, 0x0b0),
                (0x82ae_6780, JointFinalStoreKind::Vector, 0x080),
                (0x82ae_6784, JointFinalStoreKind::Vector, 0x060),
                (0x82ae_6788, JointFinalStoreKind::Vector, 0x070),
                (0x82ae_678c, JointFinalStoreKind::Vector, 0x0a0),
            ]
        );
        assert_eq!(result.rows[12][3], 0x1000_1000);
        assert_eq!(result.rows[4][3], 0x3bc1_a723);
    }

    #[test]
    fn overwrite_and_live_field_mutations_have_exact_discriminators() {
        let baseline = input_from_record(&CONE_ARC, 7);
        let changed_spy = JointFinalRecordAssemblyInput {
            transient_spy_word: 0xa5a5_5a5a,
            ..baseline
        };
        assert_eq!(
            fnv64(&assemble_joint_final_record(&changed_spy).rows),
            0xd1a5_dd73_5b04_e060
        );

        let changed_joint = JointFinalRecordAssemblyInput {
            joint_word: 0xa5a5_5a5a,
            ..baseline
        };
        assert_eq!(
            fnv64(&assemble_joint_final_record(&changed_joint).rows),
            0x2471_8e1b_ab48_9b22
        );

        let mut changed_lt = baseline;
        changed_lt.scaled.linear_right[3] = f(0xdead_beef);
        let changed_lt_record = assemble_joint_final_record(&changed_lt);
        assert_eq!(changed_lt_record.rows[4][3], 0xdead_beef);
        assert_eq!(fnv64(&changed_lt_record.rows), 0x59cc_bda8_50c3_0920);

        let mut changed_rf = baseline;
        changed_rf.stable.attachment_a_and_rf_word[3] = 0xaaaa_aaaa;
        assert_eq!(
            fnv64(&assemble_joint_final_record(&changed_rf).rows),
            0x3e4a_d25b_3013_6294
        );

        let mut changed_max_x = baseline;
        changed_max_x.scaled.angular_max[0] = f(0x1234_5678);
        let changed_max_record = assemble_joint_final_record(&changed_max_x);
        assert_eq!(changed_max_record.rows[6][3], 0x1234_5678);
        assert_eq!(fnv64(&changed_max_record.rows), 0xb925_9100_307d_1666);
    }

    #[test]
    fn literal_masks_select_only_requested_scalar_lane() {
        let a = v([0x0123_4567, 0x89ab_cdef, 0x7654_3210, 0x8000_0000]);
        let b = v([0xfedc_ba98, 0x0bad_f00d, 0xaaaa_5555, 0x7fc0_1234]);
        assert_eq!(
            vector_permute(a, b, MASK_XYZ_X),
            [0x0123_4567, 0x89ab_cdef, 0x7654_3210, 0xfedc_ba98]
        );
        assert_eq!(
            vector_permute(a, b, MASK_XYZ_Y),
            [0x0123_4567, 0x89ab_cdef, 0x7654_3210, 0x0bad_f00d]
        );
        assert_eq!(
            vector_permute(a, b, MASK_XYZ_Z),
            [0x0123_4567, 0x89ab_cdef, 0x7654_3210, 0xaaaa_5555]
        );
    }

    fn input_from_record(
        rows: &[JointWords; 24],
        transient_spy_word: u32,
    ) -> JointFinalRecordAssemblyInput {
        let vector =
            |row: usize, poison_w: u32| v([rows[row][0], rows[row][1], rows[row][2], poison_w]);
        JointFinalRecordAssemblyInput {
            stable: JointFinalStableRows {
                attachment_a_and_rf_word: rows[0],
                attachment_b_and_rf_word: rows[1],
                linear_basis: [
                    [rows[12][0], rows[12][1], rows[12][2], 0x7abc_def0],
                    rows[13],
                    rows[14],
                ],
                angular_basis: [rows[15], rows[16], rows[17]],
                inverse_inertia_a: [rows[18], rows[19], rows[20]],
                inverse_inertia_b: [rows[21], rows[22], rows[23]],
            },
            scaled: JointFinalScaledRows {
                linear_right: v(rows[4]),
                angular_right: v(rows[5]),
                linear_up: vector(6, 0x7fc0_0001),
                angular_up: vector(7, 0x7fc0_0002),
                linear_at: vector(8, 0x7fc0_0003),
                angular_at: vector(9, 0x7fc0_0004),
                position_max: vector(10, 0x7fc0_0005),
                position_min: vector(11, 0x7fc0_0006),
                angular_max: v([rows[6][3], rows[7][3], rows[10][3], 0x7fc0_0007]),
                angular_min: v([rows[8][3], rows[9][3], rows[11][3], 0x7fc0_0008]),
            },
            joint_word: rows[12][3],
            transient_spy_word,
        }
    }

    fn fnv64(rows: &[JointWords; 24]) -> u64 {
        rows.iter()
            .copied()
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

    fn v(bits: JointWords) -> JointVector {
        bits.map(f)
    }
}
