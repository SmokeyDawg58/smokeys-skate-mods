//! Independent original-byte oracle, not frozen producer output.
//!
//! Original db.big: raw VLT offset256/851568 bytes, BIN851840/812520.
//! Existing `inspect_vault` located `physics_reckoning.default`'s sole collection
//! (no parent,32 explicit entries), cached Direct dependency1 offset0x9e110.
//! Independently reading that original archive region gives the1232-byte SHA256
//! 23c8bac085a9a5d945ea7d3ee48acfe6dc161f8acc63ce1ee1b676197ab581e6.
//! Its bytes also match the extracted layout (zero differences); unlike `PtrN`
//! slots, these original inline scalar/graph words contain no relocated pointers.

use super::*;

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
#[expect(
    clippy::too_many_lines,
    reason = "literal independent original-byte oracle stays adjacent to its one mounted validation"
)]
fn mounted_original_reckoning_cached_controls_graphs_and_lazy_scalar_routes() {
    let vault = crate::attribute_runtime_corpus::mounted_physics_attributes_with_classes(&[
        0x7ce1_b6b5_8170_9c6f,
    ]);
    let mut reader = ReckoningVaultAttributes::bind(&vault);
    let own = reader.capture_layout(ReckoningAttributeScope::Own).unwrap();
    let global = reader
        .capture_layout(ReckoningAttributeScope::Global)
        .unwrap();
    assert!(own.0.is_some() && global.0.is_some());
    assert!(!std::ptr::eq(
        &raw const reader.own,
        &raw const reader.global
    ));
    for (offset, expected) in [
        (0x4a0, [0x3d23_d70a, 0x3d23_d70a, 0x3e4c_cccd, 0x3e4c_cccd]),
        (0x4b0, [0x3e4c_cccd, 0x3ecc_cccd, 0x3e4c_cccd, 0x3e4c_cccd]),
        (0x4c0, [0x3e4c_cccd, 0x3e4c_cccd, 0x3d4c_cccd, 0x3dcc_cccd]),
    ] {
        let actual: [u32; 4] = std::array::from_fn(|index| {
            reader
                .layout_float(&own, offset + index * 4)
                .unwrap()
                .to_bits()
        });
        assert_eq!(actual, expected, "original controller layout {offset:#x}");
    }
    // Raw X[8]/Y[8] from original BIN9e110+offset, all independently read
    // before the provider existed. Negative-graph metadata100..10f is omitted
    // precisely because the original ordinary evaluator receives X110/Y130.
    let graphs: [(usize, [u32; 16]); 8] = [
        (
            0x110,
            [
                0,
                0x3fc0_0000,
                0x4040_0000,
                0x408a_2811,
                0x40c2_ebf8,
                0x40f9_999a,
                0x411b_4502,
                0x4140_0000,
                0,
                0,
                0,
                0x3d3e_2be2,
                0x3e33_3333,
                0x3e97_c57d,
                0x3ed5_f15e,
                0x3f0c_cccd,
            ],
        ),
        (
            0x1a0,
            [
                0,
                0x3e00_0000,
                0x3e86_410b,
                0x3ebf_caa1,
                0x3f18_996f,
                0x3f3d_b4f3,
                0x3f50_7813,
                0x3f80_0000,
                0,
                0,
                0x3d26_0dd1,
                0x3e22_9838,
                0x3ed6_7c8a,
                0x3f3e_4530,
                0x3f80_0000,
                0x3f80_0000,
            ],
        ),
        (
            0x1e0,
            [
                0,
                0x3de2_d07a,
                0x3e6b_2732,
                0x3ebc_74bf,
                0x3f0a_d724,
                0x3f34_88c1,
                0x3f66_fbd6,
                0x3f80_0000,
                0x3f10_6eb4,
                0x3f1e_4530,
                0x3f2e_b3e4,
                0x3f46_eb3e,
                0x3f60_0000,
                0x3f73_06eb,
                0x3f80_0000,
                0x3f80_0000,
            ],
        ),
        (
            0x2a0,
            [
                0,
                0x3e07_1682,
                0x3e96_1904,
                0x3f03_c0a0,
                0x3f40_355e,
                0x3f58_cecd,
                0x3f72_3db5,
                0x3f80_0000,
                0,
                0,
                0x3dcf_9153,
                0x3e9f_2298,
                0x3f2d_d67c,
                0x3f58_375a,
                0x3f78_375a,
                0x3f80_0000,
            ],
        ),
        (
            0x320,
            [
                0,
                0x3e19_999a,
                0x3e99_999a,
                0x3ee6_6666,
                0x3f19_999a,
                0x3f40_0000,
                0x3f66_6666,
                0x3f80_0000,
                0,
                0x3e19_999a,
                0x3e99_999a,
                0x3ee6_6666,
                0x3f19_999a,
                0x3f40_0000,
                0x3f66_6666,
                0x3f80_0000,
            ],
        ),
        (
            0x3a0,
            [
                0,
                0x3e70_2808,
                0x3ea8_7167,
                0x3ec3_2086,
                0x3eec_d222,
                0x3f17_c3f6,
                0x3f5a_e47a,
                0x3f80_0000,
                0,
                0x3b5d_67ff,
                0x3d4f_9150,
                0x3de4_5309,
                0x3eb3_e455,
                0x3f14_c1bb,
                0x3f46_0dd6,
                0x3f52_2984,
            ],
        ),
        (
            0x3e0,
            [
                0x3ad5_78e5,
                0x3e0a_6c64,
                0x3e84_961b,
                0x3ebd_4a36,
                0x3ef9_5439,
                0x3f0f_6d3d,
                0x3f1f_aff3,
                0x3f80_0000,
                0x3b5d_67ff,
                0,
                0x3ec6_eb3e,
                0x3f29_8376,
                0x3f5c_8a61,
                0x3f6b_3e45,
                0x3f72_2983,
                0x3f80_0000,
            ],
        ),
        (
            0x420,
            [
                0,
                0x3e67_d150,
                0x3ec4_cb77,
                0x3ef0_2808,
                0x3f09_96ef,
                0x3f18_2eb2,
                0x3f27_9bef,
                0x3f80_0000,
                0,
                0,
                0x3e11_4c1c,
                0x3e9d_67c8,
                0x3f04_5307,
                0x3f5e_4530,
                0x3f80_0000,
                0x3f80_0000,
            ],
        ),
    ];
    for (offset, expected) in graphs {
        let layout = if offset == 0x420 { &global } else { &own };
        let actual: [u32; 16] = std::array::from_fn(|index| {
            reader
                .layout_float(layout, offset + index * 4)
                .unwrap()
                .to_bits()
        });
        assert_eq!(actual, expected, "original graph layout {offset:#x}");
        assert_eq!(
            reader
                .evaluate_graph8(layout, offset, f32::NEG_INFINITY)
                .unwrap()
                .to_bits(),
            expected[8]
        );
        assert_eq!(
            reader
                .evaluate_graph8(layout, offset, f32::from_bits(expected[7]))
                .unwrap()
                .to_bits(),
            expected[15]
        );
    }
    // Independent raw VLT value addresses follow each key by8 bytes:
    // 5d710,5d730,5d6f0,5d7c0,5d690,5d7b0 respectively.
    for (scope, key, expected) in [
        (
            ReckoningAttributeScope::Own,
            0xda9e_8318_96b8_6ffc,
            0x3df5_c28f,
        ),
        (ReckoningAttributeScope::Own, 0xfae8_0499_e7d5_f3e4, 0),
        (
            ReckoningAttributeScope::Own,
            0x6ab1_ba81_9741_4fb3,
            0x3dcc_cccd,
        ),
        (
            ReckoningAttributeScope::Own,
            0x8b86_e017_4aee_5d6e,
            0x4000_0000,
        ),
        (
            ReckoningAttributeScope::Global,
            0x5366_a548_f61b_bbea,
            0x3f19_999a,
        ),
        (
            ReckoningAttributeScope::Own,
            0xaf8a_5d55_853a_d035,
            0x3f00_0000,
        ),
    ] {
        let pointer = reader.lookup(scope, key).unwrap();
        assert!(pointer.is_some());
        if key == 0xfae8_0499_e7d5_f3e4 {
            assert_eq!(reader.signed_word(&pointer).unwrap(), 0);
        } else {
            // Oracle inspection intentionally dereferences even the source dead
            // key; the runtime producer must still omit that particular load.
            assert_eq!(reader.float(&pointer).unwrap().to_bits(), expected);
        }
    }
}
