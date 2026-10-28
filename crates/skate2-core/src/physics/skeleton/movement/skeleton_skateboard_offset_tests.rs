//! Source arithmetic and retained-state discriminators, not native traces.
use super::*;
use crate::{
    skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
    skeleton_data_tests::{build, fixture},
};
use skate2_formats::andale::PackedFastString;
use std::sync::Arc;

const IDENTITY: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

fn uninitialized_ik() -> SkeletonIkTargets {
    let (hierarchy, parameters, pose) = fixture();
    SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()))
}

fn distinct_ik() -> SkeletonIkTargets {
    let (mut hierarchy, parameters, mut pose) = fixture();
    for (index, name) in [
        "LeftToeBase_Reparented",
        "RightToeBase_Reparented",
        "LeftHand_Reparented",
        "RightHand_Reparented",
    ]
    .into_iter()
    .enumerate()
    {
        hierarchy.bone_names.push(name.into());
        hierarchy
            .packed_bone_names
            .push(PackedFastString::encode(name).unwrap());
        hierarchy.parents.push(-1);
        hierarchy.bone_metadata.push(0);
        let mut target = IDENTITY;
        target[12] = f32::from(u16::try_from(index + 1).unwrap());
        target[13] = target[12] * 2.0;
        target[14] = target[12] * 3.0;
        target[15] = target[12] * 4.0;
        pose.push(target);
    }
    let mut ik = SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()));
    ik.initialize();
    ik.try_calculate(&pose).unwrap();
    ik
}

#[derive(Default)]
struct Numeric {
    dots: Vec<([f32; 4], [f32; 4])>,
    estimates: Vec<f32>,
}
impl SkateboardSpawnMathNumeric for Numeric {
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> f32 {
        self.dots.push((left, right));
        HostSkateboardSpawnMathNumeric.dot3(left, right)
    }
    fn dot4(&mut self, _: [f32; 4], _: [f32; 4]) -> f32 {
        panic!("offset normalization does not call dot4")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> f32 {
        self.estimates.push(value);
        HostSkateboardSpawnMathNumeric.reciprocal_sqrt_estimate(value)
    }
}
struct Frames {
    result: Result<f32, &'static str>,
    calls: usize,
}
impl SkeletonSkateboardOffsetCvars for Frames {
    type Error = &'static str;
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.calls += 1;
        self.result
    }
}

#[test]
fn constructor_and_inactive_update_preserve_pose_and_clear_both_just_set_bytes() {
    let mut offset = SkeletonSkateboardOffset::new();
    assert_eq!(
        offset.transform.map(f32::to_bits),
        [
            0x3f80_0000,
            0,
            0,
            0,
            0,
            0x3f80_0000,
            0,
            0,
            0,
            0,
            0x3f80_0000,
            0,
            0,
            0,
            0,
            0,
        ]
    );
    assert_eq!(offset.full_count.to_bits(), 0);
    assert_eq!(offset.vertical_count.to_bits(), 0);
    assert!(!offset.full_just_set && !offset.vertical_just_set);
    offset.full_count = -0.0;
    offset.vertical_count = f32::NEG_INFINITY;
    offset.full_just_set = true;
    offset.vertical_just_set = true;
    let mut volume = [f32::from_bits(0x7fc0_1234); 16];
    let original = volume.map(f32::to_bits);
    let mut ik = uninitialized_ik();
    let original_targets = ik.targets().map(|matrix| matrix.map(f32::to_bits));
    let mut numeric = Numeric::default();
    offset.update(&mut volume, &mut ik, &mut numeric);
    assert_eq!(volume.map(f32::to_bits), original);
    assert_eq!(
        ik.targets().map(|matrix| matrix.map(f32::to_bits)),
        original_targets
    );
    assert!(numeric.dots.is_empty() && numeric.estimates.is_empty());
    assert!(!offset.full_just_set && !offset.vertical_just_set);
    assert_eq!(offset.full_count.to_bits(), (-0.0_f32).to_bits());
    assert_eq!(offset.vertical_count.to_bits(), f32::NEG_INFINITY.to_bits());
}

#[test]
fn setter_failure_retains_transform_and_flag_then_just_set_applies_without_init_or_decay() {
    let mut offset = SkeletonSkateboardOffset::new();
    offset.full_count = 7.0;
    offset.vertical_count = 9.0;
    offset.vertical_just_set = true;
    let transform = [
        0.0, 0.0, -1.0, 2.0, 0.0, 1.0, 0.0, 3.0, 1.0, 0.0, 0.0, 5.0, 7.0, 11.0, 13.0, 17.0,
    ];
    let mut frames = Frames {
        result: Err("current cvar"),
        calls: 0,
    };
    assert_eq!(
        offset.try_set_full_transform(&transform, &mut frames),
        Err("current cvar")
    );
    assert_eq!(frames.calls, 1);
    assert_eq!(
        offset.transform.map(f32::to_bits),
        transform.map(f32::to_bits)
    );
    assert!(offset.full_just_set && offset.vertical_just_set);
    assert_eq!(offset.full_count.to_bits(), 7.0_f32.to_bits());
    frames.result = Ok(3.5);
    offset
        .try_set_full_transform(&transform, &mut frames)
        .unwrap();
    assert_eq!(frames.calls, 2);
    let mut volume = [
        2.0, 3.0, 5.0, 19.0, 7.0, 11.0, 13.0, 23.0, 17.0, 19.0, 23.0, 29.0, 31.0, 37.0, 41.0, 43.0,
    ];
    let expected_volume = compose_oracle(&transform, &volume);
    let mut ik = uninitialized_ik();
    let expected_target = compose_oracle(&transform, &IDENTITY);
    let mut numeric = Numeric::default();
    offset.update(&mut volume, &mut ik, &mut numeric);
    assert!(
        !ik.initialized(),
        "AddOffset does not need or produce Init mappings"
    );
    assert!(numeric.dots.is_empty() && numeric.estimates.is_empty());
    assert_eq!(offset.full_count.to_bits(), 3.5_f32.to_bits());
    assert_eq!(offset.vertical_count.to_bits(), 9.0_f32.to_bits());
    assert_eq!(volume.map(f32::to_bits), expected_volume.map(f32::to_bits));
    for target in ik.targets() {
        assert_eq!(target.map(f32::to_bits), expected_target.map(f32::to_bits));
    }
    assert!(!offset.full_just_set && !offset.vertical_just_set);
}

#[test]
fn independent_full_and_y_decay_updates_all_four_existing_targets() {
    let mut offset = SkeletonSkateboardOffset::new();
    offset.transform[12..16].copy_from_slice(&[8.0, 12.0, 16.0, 20.0]);
    offset.full_count = 4.0;
    offset.vertical_count = 2.0;
    let mut expected_offset = IDENTITY;
    expected_offset[12..16].copy_from_slice(&[4.5, 3.0, 9.0, 11.25]);
    let mut volume = IDENTITY;
    volume[12] = 5.0;
    let expected_volume = compose_oracle(&expected_offset, &volume);
    let mut ik = distinct_ik();
    let old_targets = *ik.targets();
    let expected_targets = old_targets.map(|target| compose_oracle(&expected_offset, &target));
    let mut numeric = Numeric::default();
    offset.update(&mut volume, &mut ik, &mut numeric);
    assert_eq!(
        offset.transform.map(f32::to_bits),
        expected_offset.map(f32::to_bits)
    );
    assert_eq!(offset.full_count.to_bits(), 3.0_f32.to_bits());
    assert_eq!(offset.vertical_count.to_bits(), 1.0_f32.to_bits());
    assert_eq!(numeric.dots.len(), 6);
    assert_eq!(numeric.estimates.len(), 3);
    assert_eq!(volume.map(f32::to_bits), expected_volume.map(f32::to_bits));
    assert_eq!(
        ik.targets().map(|matrix| matrix.map(f32::to_bits)),
        expected_targets.map(|matrix| matrix.map(f32::to_bits))
    );
    for (before, after) in old_targets.iter().zip(ik.targets()) {
        assert_ne!(before.map(f32::to_bits), after.map(f32::to_bits));
    }
}

#[test]
fn full_decay_preserves_y_and_classical_normalization_consumes_original_interpolated_axes() {
    let mut offset = SkeletonSkateboardOffset::new();
    offset.transform = [
        2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0, 31.0, 37.0, 8.0, -0.0, 16.0, 20.0,
    ];
    offset.full_count = 4.0;
    offset.vertical_count = 2.0;
    offset.vertical_just_set = true;
    let mut numeric = Numeric::default();
    let mut volume = IDENTITY;
    offset.update(&mut volume, &mut uninitialized_ik(), &mut numeric);
    let expected_z = [12.9375_f32, 16.3125, 17.875, 20.8125];
    let expected_y = [6.1875_f32, 7.75, 9.5625, 10.6875];
    let expected_x = [1.5625_f32, 1.6875, 2.8125, 3.9375];
    assert_eq!(
        numeric.dots[0].0.map(f32::to_bits),
        expected_z.map(f32::to_bits)
    );
    assert_eq!(
        numeric.dots[0].1.map(f32::to_bits),
        expected_z.map(f32::to_bits)
    );
    assert_eq!(
        numeric.dots[1].1.map(f32::to_bits),
        expected_y.map(f32::to_bits)
    );
    assert_eq!(
        numeric.dots[3].1.map(f32::to_bits),
        expected_x.map(f32::to_bits)
    );
    assert_eq!(
        numeric.dots[4].1.map(f32::to_bits),
        expected_x.map(f32::to_bits)
    );
    assert_eq!(offset.transform[13].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(offset.transform[15].to_bits(), 11.25_f32.to_bits());
    assert_eq!(offset.vertical_count.to_bits(), 2.0_f32.to_bits());
    assert_ne!(
        offset.transform[11].to_bits(),
        0,
        "normalization preserves computed W"
    );
}

#[test]
fn interpolation_fuses_old_times_weight_after_separate_identity_product() {
    let mut offset = SkeletonSkateboardOffset::new();
    offset.full_count = 3.0;
    offset.transform[10] = f32::from_bits(0xbf96_36fb);
    let mut numeric = Numeric::default();
    let mut volume = IDENTITY;
    offset.update(&mut volume, &mut uninitialized_ik(), &mut numeric);
    // Frozen separate arithmetic audit: source=3d0b2ade, defused=3d0b2ae0,
    // identity+(old-identity)*weight=3d0b2ad0. Observe before normalization.
    assert_eq!(numeric.dots[0].0[2].to_bits(), 0x3d0b_2ade);
    assert_eq!(numeric.dots[0].1[2].to_bits(), 0x3d0b_2ade);
}

#[test]
fn positive_counts_apply_even_when_decrement_reaches_zero_or_negative() {
    for (count, expected_count, x, y, z, w) in [
        (1.0, 0.0, 0.0, 12.0, 0.0, 0.0),
        (0.5, -0.5, 8.0, 12.0, 16.0, 20.0),
    ] {
        let mut offset = SkeletonSkateboardOffset::new();
        offset.full_count = count;
        offset.transform[12..16].copy_from_slice(&[8.0, 12.0, 16.0, 20.0]);
        let mut volume = IDENTITY;
        offset.update(
            &mut volume,
            &mut uninitialized_ik(),
            &mut Numeric::default(),
        );
        assert_eq!(offset.full_count.to_bits(), f32::to_bits(expected_count));
        assert_eq!(
            std::array::from_fn::<_, 4, _>(|lane| offset.transform[12 + lane].to_bits()),
            [x, y, z, w].map(f32::to_bits)
        );
        assert_eq!(
            volume[13].to_bits(),
            12.0_f32.to_bits(),
            "decrement must not cancel application"
        );
    }
}

#[test]
fn nan_counts_skip_both_branches_preserve_bits_and_clear_just_set() {
    let mut offset = SkeletonSkateboardOffset::new();
    offset.full_count = f32::from_bits(0x7fc0_1234);
    offset.vertical_count = f32::from_bits(0xffc0_5678);
    offset.full_just_set = true;
    offset.vertical_just_set = true;
    offset.transform[12..16].copy_from_slice(&[8.0, 12.0, 16.0, 20.0]);
    let transform = offset.transform.map(f32::to_bits);
    let mut volume = [f32::from_bits(0x7fc0_2468); 16];
    let original_volume = volume.map(f32::to_bits);
    let mut ik = distinct_ik();
    let targets = ik.targets().map(|matrix| matrix.map(f32::to_bits));
    let mut numeric = Numeric::default();
    offset.update(&mut volume, &mut ik, &mut numeric);
    assert_eq!(offset.full_count.to_bits(), 0x7fc0_1234);
    assert_eq!(offset.vertical_count.to_bits(), 0xffc0_5678);
    assert_eq!(offset.transform.map(f32::to_bits), transform);
    assert_eq!(volume.map(f32::to_bits), original_volume);
    assert_eq!(ik.targets().map(|matrix| matrix.map(f32::to_bits)), targets);
    assert!(numeric.dots.is_empty() && numeric.estimates.is_empty());
    assert!(!offset.full_just_set && !offset.vertical_just_set);
}

#[test]
fn nan_count_does_not_suppress_other_active_branch_or_offset_application() {
    for full_nan in [true, false] {
        let mut offset = SkeletonSkateboardOffset::new();
        offset.full_count = if full_nan { f32::NAN } else { 2.0 };
        offset.vertical_count = if full_nan { 2.0 } else { f32::NAN };
        offset.transform[12..16].copy_from_slice(&[8.0, 12.0, 16.0, 20.0]);
        let expected_position = if full_nan {
            [8.0, 3.0, 16.0, 20.0]
        } else {
            [2.0, 12.0, 4.0, 5.0]
        };
        let mut volume = IDENTITY;
        let mut ik = uninitialized_ik();
        let mut numeric = Numeric::default();
        offset.update(&mut volume, &mut ik, &mut numeric);
        assert_eq!(
            std::array::from_fn::<_, 4, _>(|lane| offset.transform[12 + lane].to_bits()),
            expected_position.map(f32::to_bits)
        );
        assert_eq!(
            std::array::from_fn::<_, 4, _>(|lane| volume[12 + lane].to_bits()),
            expected_position.map(f32::to_bits)
        );
        for target in ik.targets() {
            assert_eq!(
                std::array::from_fn::<_, 4, _>(|lane| target[12 + lane].to_bits()),
                expected_position.map(f32::to_bits)
            );
        }
        assert!(if full_nan {
            offset.full_count.is_nan()
        } else {
            offset.vertical_count.is_nan()
        });
        let active_count = if full_nan {
            offset.vertical_count
        } else {
            offset.full_count
        };
        assert_eq!(active_count.to_bits(), 1.0_f32.to_bits());
        assert_eq!(numeric.dots.len(), if full_nan { 0 } else { 6 });
    }
}

#[test]
fn positive_infinite_counts_enter_arithmetic_without_repair() {
    for full_active in [true, false] {
        let mut offset = SkeletonSkateboardOffset::new();
        if full_active {
            offset.full_count = f32::INFINITY;
        } else {
            offset.vertical_count = f32::INFINITY;
        }
        let mut volume = IDENTITY;
        let mut ik = uninitialized_ik();
        let mut numeric = Numeric::default();
        offset.update(&mut volume, &mut ik, &mut numeric);
        if full_active {
            assert_eq!(numeric.dots.len(), 6);
            assert_eq!(numeric.estimates.len(), 3);
            assert!(offset.transform[..12].iter().all(|lane| lane.is_nan()));
            assert!(volume.iter().all(|lane| lane.is_nan()));
            assert!(
                ik.targets()
                    .iter()
                    .all(|matrix| matrix.iter().all(|lane| lane.is_nan()))
            );
            assert_eq!(offset.full_count.to_bits(), f32::INFINITY.to_bits());
        } else {
            assert!(numeric.dots.is_empty() && numeric.estimates.is_empty());
            assert!(offset.transform[13].is_nan() && volume[13].is_nan());
            assert!(ik.targets().iter().all(|matrix| matrix[13].is_nan()));
            assert_eq!(offset.vertical_count.to_bits(), f32::INFINITY.to_bits());
        }
    }
}

fn compose_oracle(outer: &AnimationMatrix, inner: &AnimationMatrix) -> AnimationMatrix {
    let mut result = [0.0; 16];
    for column in 0..4 {
        for lane in 0..4 {
            let index = column * 4;
            let x = if column == 3 {
                inner[index].mul_add(outer[lane], outer[12 + lane])
            } else {
                inner[index] * outer[lane]
            };
            let xy = inner[index + 1].mul_add(outer[4 + lane], x);
            result[index + lane] = inner[index + 2].mul_add(outer[8 + lane], xy);
        }
    }
    result
}
