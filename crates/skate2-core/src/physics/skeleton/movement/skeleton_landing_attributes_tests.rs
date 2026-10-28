//! Synthetic routing/storage tests; `check_original` is called by the real-owner
//! corpus and independently reads the mounted payloads and cached layout.
#![expect(
    clippy::float_cmp,
    reason = "exact finite routing fixtures; full bit-pattern preservation is checked separately"
)]
use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node, set_dependency_bytes};

#[test]
fn prephysics_angles_use_the_same_retained_global_default_instance() {
    use crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsAttributes;
    let minimum = 0xb76b_5101_768e_856c;
    let maximum = 0xa175_875d_ec9d_6db7;
    let mut vault = graph(
        CLASS,
        vec![definition(minimum, 4, 4), definition(maximum, 4, 4)],
    );
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        None,
        vec![
            node(minimum, 0x40, 7.0_f32.to_bits()),
            node(maximum, 0x40, 153.0_f32.to_bits()),
        ],
    );
    collection(
        &mut vault,
        CLASS,
        17,
        1,
        None,
        vec![
            node(minimum, 0x40, 71.0_f32.to_bits()),
            node(maximum, 0x40, 23.0_f32.to_bits()),
        ],
    );
    let vault = Arc::new(vault);
    let mut actual =
        crate::skeleton_current_animation::SkeletonCurrentAnimationAttributes::bind(&vault);
    assert_eq!(
        actual.minimum_angle_degrees().unwrap().to_bits(),
        7.0_f32.to_bits()
    );
    assert_eq!(
        actual.maximum_angle_degrees().unwrap().to_bits(),
        153.0_f32.to_bits()
    );
    assert_eq!(
        actual.minimum_angle_degrees().unwrap().to_bits(),
        7.0_f32.to_bits()
    );
}

#[test]
fn prephysics_angle_missing_is_zero_but_malformed_present_is_a_lazy_error() {
    use crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsAttributes;
    let minimum = 0xb76b_5101_768e_856c;
    let maximum = 0xa175_875d_ec9d_6db7;
    let mut vault = graph(
        CLASS,
        vec![definition(minimum, 4, 4), definition(maximum, 4, 1)],
    );
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        None,
        vec![node(maximum, 0x40, 0x7f00_0000)],
    );
    let mut actual = SkeletonLandingVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(actual.minimum_angle_degrees().unwrap().to_bits(), 0);
    assert!(matches!(
        actual.maximum_angle_degrees(),
        Err(AttributeRuntimeError::PayloadSize {
            key: 0xa175_875d_ec9d_6db7,
            expected: 4,
            actual: 1
        })
    ));
    assert_eq!(actual.minimum_angle_degrees().unwrap().to_bits(), 0);
}

const SCALARS: [(SkeletonLandingAdjustScalar, u64); 16] = [
    (
        SkeletonLandingAdjustScalar::MaximumVelocity,
        0x5199_e2e7_49e7_220b,
    ),
    (
        SkeletonLandingAdjustScalar::CoffinDuration,
        0x1d4c_65d8_fd17_9fe2,
    ),
    (
        SkeletonLandingAdjustScalar::CoffinVelocityDenominator,
        0x4f4e_3f45_66ec_34e3,
    ),
    (
        SkeletonLandingAdjustScalar::CoffinBase,
        0x2f99_6144_e5ef_4eea,
    ),
    (
        SkeletonLandingAdjustScalar::DesiredCom,
        0x9091_e734_f21a_004d,
    ),
    (
        SkeletonLandingAdjustScalar::MinimumYPosition,
        0x2a69_0d7e_8c1e_dcf4,
    ),
    (
        SkeletonLandingAdjustScalar::GroundMinimumElapsed,
        0x0175_8b41_9109_fdae,
    ),
    (
        SkeletonLandingAdjustScalar::CoffinFrames,
        0x9aba_ff27_0d53_f138,
    ),
    (
        SkeletonLandingAdjustScalar::GrindTargetSpeed,
        0x74e3_fc7c_dd2f_8c97,
    ),
    (SkeletonLandingAdjustScalar::GrindK1, 0x1098_9560_be00_f690),
    (SkeletonLandingAdjustScalar::GrindK2, 0xbf50_36b2_3111_79e8),
    (
        SkeletonLandingAdjustScalar::GrindTargetAfterTime,
        0x8dac_44f9_0326_2ddb,
    ),
    (SkeletonLandingAdjustScalar::GroundK1, 0xac53_2ffc_deba_62d8),
    (SkeletonLandingAdjustScalar::GroundK2, 0x2471_dd04_ee7e_e637),
    (SkeletonLandingAdjustScalar::ManualK1, 0x01ba_609b_b65c_adb5),
    (SkeletonLandingAdjustScalar::ManualK2, 0x475a_3cb8_59bd_b0a6),
];
fn put(bytes: &mut [u8], offset: usize, word: u32) {
    bytes[offset..offset + 4].copy_from_slice(&word.to_be_bytes());
}
fn put_graph(bytes: &mut [u8], offset: usize, values: [f32; 4]) {
    for index in 0..4 {
        put(bytes, offset + index * 4, 0x7fc1_2345); // Metadata must be ignored.
        put(
            bytes,
            offset + 0x10 + index * 4,
            [0.0_f32, 1.0, 2.0, 3.0][index].to_bits(),
        );
        put(bytes, offset + 0x20 + index * 4, values[index].to_bits());
    }
}

#[test]
fn all_scalar_modes_route_to_the_actual_global_keys_without_float_sanitization() {
    let definitions = SCALARS.map(|(_, key)| definition(key, 4, 4)).to_vec();
    let mut vault = graph(CLASS, definitions);
    let words = [
        0x8000_0000,
        0x7fc1_2345,
        0x7f80_0000,
        0xff80_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x4000_0000,
        0x4040_0000,
        0x4080_0000,
        0x40a0_0000,
        0x40c0_0000,
        0x40e0_0000,
        0x4100_0000,
        0x4110_0000,
        0x4120_0000,
        0x4130_0000,
    ];
    let nodes = SCALARS
        .iter()
        .zip(words)
        .map(|((_, key), word)| node(*key, 0x40, word))
        .collect();
    collection(&mut vault, CLASS, DEFAULT, 0, None, nodes);
    let mut provider = SkeletonLandingVaultAttributes::bind(&Arc::new(vault));
    for ((scalar, _), expected) in SCALARS.into_iter().zip(words) {
        assert_eq!(provider.scalar(scalar).unwrap().to_bits(), expected);
    }
}

#[test]
fn global_default_selection_does_not_follow_a_separate_skeleton_instance() {
    let key = SCALARS[0].1;
    let mut vault = graph(CLASS, vec![definition(key, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        None,
        vec![node(key, 0x40, 8.0_f32.to_bits())],
    );
    collection(
        &mut vault,
        CLASS,
        77,
        0,
        None,
        vec![node(key, 0x40, 17.0_f32.to_bits())],
    );
    let vault = Arc::new(vault);
    let own = vault.instance(CLASS, 77);
    let mut provider = SkeletonLandingVaultAttributes::bind(&vault);
    assert_eq!(
        own.try_scalar_word(key, 0).unwrap(),
        Some(17.0_f32.to_bits())
    );
    assert_eq!(
        provider
            .scalar(SkeletonLandingAdjustScalar::MaximumVelocity)
            .unwrap(),
        8.0
    );
}

#[test]
fn null_generated_layout_and_missing_keys_use_their_separate_zero_defaults() {
    for found in [false, true] {
        let mut vault = graph(CLASS, vec![]);
        if found {
            collection(&mut vault, CLASS, DEFAULT, 0, None, vec![]);
        }
        let mut provider = SkeletonLandingVaultAttributes::bind(&Arc::new(vault));
        for (scalar, _) in SCALARS {
            assert_eq!(provider.scalar(scalar).unwrap().to_bits(), 0);
        }
        for time in [-1.0, 0.0, 1.0, f32::NAN] {
            assert_eq!(provider.manual_blend(time).unwrap().to_bits(), 0);
            assert_eq!(provider.grind_blend(time).unwrap().to_bits(), 0);
            assert_eq!(provider.coffin_bounce(time).unwrap().to_bits(), 0);
        }
    }
}

#[test]
fn found_null_scalar_stops_inherited_value_and_generated_defaults_are_not_parent_layouts() {
    let key = SCALARS[0].1;
    let mut vault = graph(CLASS, vec![definition(key, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        77,
        0,
        Some(64),
        vec![node(key, 0x40, 8.0_f32.to_bits())],
    );
    collection(&mut vault, CLASS, DEFAULT, 77, None, vec![node(key, 0, 0)]);
    let mut payload = vec![0; 64 + LAYOUT_SIZE];
    put_graph(&mut payload, 64 + 0x140, [100.0; 4]);
    set_dependency_bytes(&mut vault, 1, payload);
    let mut provider = SkeletonLandingVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(
        provider
            .scalar(SkeletonLandingAdjustScalar::MaximumVelocity)
            .unwrap()
            .to_bits(),
        0
    );
    assert_eq!(provider.manual_blend(1.5).unwrap().to_bits(), 0);
}

#[test]
fn manual_graph_uses_child_cached_140_without_any_hash_definition_or_parent_layout() {
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, 77, 0, Some(640), vec![]);
    collection(&mut vault, CLASS, DEFAULT, 77, Some(64), vec![]);
    let mut payload = vec![0; 640 + LAYOUT_SIZE];
    put_graph(
        &mut payload,
        64 + 0x140,
        [-1.0, f32::from_bits(0x3400_0000), 4.0, 8.0],
    );
    put_graph(&mut payload, 640 + 0x140, [100.0; 4]);
    set_dependency_bytes(&mut vault, 1, payload);
    let mut provider = SkeletonLandingVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(
        provider
            .manual_blend(f32::from_bits(0x3f7f_fffe))
            .unwrap()
            .to_bits(),
        0xa880_0000
    );
    assert_eq!(provider.manual_blend(f32::NAN).unwrap(), 8.0);
    assert_eq!(provider.manual_blend(-2.0).unwrap(), -1.0);
}

#[test]
fn coffin_and_grind_graphs_lookup_their_own_payloads_and_ignore_metadata() {
    let mut coffin = definition(COFFIN_GRAPH, 6, 0x30);
    coffin.offset = 0;
    let mut grind = definition(GRIND_GRAPH, 6, 0x30);
    grind.offset = 0x30;
    let mut vault = graph(CLASS, vec![coffin, grind]);
    collection(&mut vault, CLASS, DEFAULT, 0, Some(64), vec![]);
    let mut payload = vec![0; 64 + LAYOUT_SIZE];
    put_graph(&mut payload, 64, [0.0, -1.0, -2.0, -3.0]);
    put_graph(&mut payload, 64 + 0x30, [0.0, 4.0, 8.0, 12.0]);
    put_graph(&mut payload, 64 + 0x140, [20.0; 4]);
    set_dependency_bytes(&mut vault, 1, payload);
    let mut provider = SkeletonLandingVaultAttributes::bind(&Arc::new(vault));
    assert_eq!(provider.coffin_bounce(1.5).unwrap(), -1.5);
    assert_eq!(provider.grind_blend(1.5).unwrap(), 6.0);
    assert_eq!(provider.manual_blend(1.5).unwrap(), 20.0);
}

#[test]
fn malformed_scalar_graph_and_cached_layout_remain_lazy_errors_not_defaults() {
    let key = SCALARS[0].1;
    let mut bad_graph = definition(COFFIN_GRAPH, 6, 0x2c);
    bad_graph.offset = 0;
    let mut vault = graph(CLASS, vec![definition(key, 4, 1), bad_graph]);
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        Some(64),
        vec![node(key, 0x40, 0x7f00_0000)],
    );
    let mut provider = SkeletonLandingVaultAttributes::bind(&Arc::new(vault));
    // Binding does not preflight active/manual paths or poison unrelated misses.
    assert_eq!(
        provider
            .scalar(SkeletonLandingAdjustScalar::CoffinFrames)
            .unwrap(),
        0.0
    );
    assert!(matches!(
        provider.scalar(SkeletonLandingAdjustScalar::MaximumVelocity),
        Err(AttributeRuntimeError::PayloadSize {
            expected: 4,
            actual: 1,
            ..
        })
    ));
    assert!(matches!(
        provider.coffin_bounce(0.0),
        Err(AttributeRuntimeError::PayloadSize {
            expected: 0x30,
            actual: 0x2c,
            ..
        })
    ));
    assert!(provider.manual_blend(0.0).is_err()); // Nonnull220 exceeds this real allocation.
}

#[test]
fn unsupported_found_nodes_do_not_become_missing_scalar_fallbacks() {
    let key = SCALARS[0].1;
    let mut vault = graph(CLASS, vec![definition(key, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        None,
        vec![node(key, 0x20, 0)],
    );
    let mut provider = SkeletonLandingVaultAttributes::bind(&Arc::new(vault));
    assert!(
        provider
            .scalar(SkeletonLandingAdjustScalar::MaximumVelocity)
            .is_err()
    );
}

/// Called by the actual original physical-owner corpus, not another ignored
/// test. Returns the independently read manual X/Y words for evidence capture.
pub(crate) fn check_original(vault: &Arc<AttributeRuntimeVault>) -> [u32; 8] {
    let original = vault.instance(CLASS, DEFAULT);
    assert!(
        !original.is_null(),
        "mounted original physics_animation.default"
    );
    let mut provider = SkeletonLandingVaultAttributes::bind(vault);
    for (scalar, key) in SCALARS {
        let data = original.try_get_data(key, 0).unwrap();
        let expected = data.as_ref().map_or(0, |data| {
            u32::from_be_bytes(data.bytes().try_into().expect("original scalar payload"))
        });
        if key == 0x5199_e2e7_49e7_220b {
            assert_eq!(expected, 8.0_f32.to_bits());
            assert!(data.is_some());
        }
        assert_eq!(
            provider.scalar(scalar).unwrap().to_bits(),
            expected,
            "original scalar {key:016x}"
        );
    }
    for key in [COFFIN_GRAPH, GRIND_GRAPH] {
        let data = original
            .try_get_data(key, 0)
            .unwrap()
            .expect("original landing graph node");
        assert_eq!(data.bytes().len(), 0x30);
        let (x, y) = original_graph(data.bytes());
        for time in [x[0] - 1.0, x[0], x[0].midpoint(x[3]), x[3], f32::NAN] {
            let actual = if key == COFFIN_GRAPH {
                provider.coffin_bounce(time)
            } else {
                provider.grind_blend(time)
            }
            .unwrap();
            assert_eq!(
                actual.to_bits(),
                original_evaluate(x, y, time).to_bits(),
                "original graph {key:016x}"
            );
        }
    }
    let layout = original
        .try_cached_layout(0x220)
        .unwrap()
        .expect("original generated cached220 layout");
    let (x, y) = original_graph(&layout.bytes()[0x140..0x170]);
    assert!(
        x.iter().chain(&y).any(|value| *value != 0.0),
        "original manual graph must not be the zero fallback"
    );
    for time in [x[0] - 1.0, x[0], x[0].midpoint(x[3]), x[3], f32::NAN] {
        assert_eq!(
            provider.manual_blend(time).unwrap().to_bits(),
            original_evaluate(x, y, time).to_bits()
        );
    }
    [x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3]].map(f32::to_bits)
}
fn original_graph(bytes: &[u8]) -> ([f32; 4], [f32; 4]) {
    let values: Vec<_> = bytes[0x10..0x30]
        .chunks_exact(4)
        .map(|word| f32::from_be_bytes(word.try_into().unwrap()))
        .collect();
    (
        [values[0], values[1], values[2], values[3]],
        [values[4], values[5], values[6], values[7]],
    )
}
fn original_evaluate(x: [f32; 4], y: [f32; 4], time: f32) -> f32 {
    if time < x[0] {
        return y[0];
    }
    if time.is_nan() || time >= x[3] {
        return y[3];
    }
    let upper = (1..4).find(|index| time < x[*index]).unwrap();
    let width = x[upper] - x[upper - 1];
    if width > 0.0 {
        ((y[upper] - y[upper - 1]) / width).mul_add(time - x[upper - 1], y[upper - 1])
    } else {
        y[upper]
    }
}
