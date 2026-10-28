//! Source order and ownership discriminators, not captured Xenon traces.
use super::*;
use crate::skeleton_data_tests::{build, fixture};

fn append_target(hierarchy: &mut skate2_formats::andale::AndaleHierarchy, name: &str) {
    hierarchy.bone_names.push(name.into());
    hierarchy
        .packed_bone_names
        .push(PackedFastString::encode(name).unwrap());
    hierarchy.parents.push(-1);
    hierarchy.bone_metadata.push(0);
}

fn target_fixture() -> (Arc<SkeletonData>, Vec<AnimationMatrix>) {
    let (mut hierarchy, parameters, mut pose) = fixture();
    for (target, name) in TARGET_NAMES.into_iter().enumerate() {
        append_target(&mut hierarchy, name);
        let mut matrix = pose[0];
        matrix[12] = f32::from(u16::try_from(target + 10).unwrap());
        pose.push(matrix);
    }
    (Arc::new(build(hierarchy, parameters, &pose).unwrap()), pose)
}

#[test]
fn fresh_targets_have_literal_native_identity_words_and_init_keeps_data_authority() {
    let (data, pose) = target_fixture();
    let mut targets = SkeletonIkTargets::new(Arc::clone(&data));
    let identity_words = [
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
    ];
    for matrix in targets.targets() {
        assert_eq!(matrix.map(f32::to_bits), identity_words);
    }
    assert!(!targets.initialized());
    assert_eq!(
        targets.try_calculate(&pose),
        Err(SkeletonIkTargetError::NotInitialized)
    );
    assert!(Arc::ptr_eq(targets.data(), &data));
    targets.initialize();
    assert!(targets.initialized());
    assert_eq!(targets.bone_ids, [Some(15), Some(19), Some(3), Some(7)]);
    assert_eq!(
        *targets.target_bone_ids(),
        [Some(24), Some(25), Some(26), Some(27)]
    );
    assert_eq!(targets.parts, PARTS.map(Some));
    assert_eq!(
        targets.parents,
        [Some(Part::LeftFoot), Some(Part::RightFoot), None, None]
    );
}

#[test]
fn init_matches_first_complete_packed_name_and_missing_name_does_not_stop_later_lookups() {
    let (mut hierarchy, parameters, mut pose) = fixture();
    for name in [
        TARGET_NAMES[2],
        TARGET_NAMES[0],
        TARGET_NAMES[3],
        TARGET_NAMES[0],
    ] {
        append_target(&mut hierarchy, name);
        pose.push(pose[0]);
    }
    // Native lookup consumes packed words, not the diagnostic display string.
    hierarchy.bone_names[25] = "unrelated_display_label".into();
    let data = Arc::new(build(hierarchy, parameters, &pose).unwrap());
    let mut targets = SkeletonIkTargets::new(data);
    targets.initialize();
    assert_eq!(
        *targets.target_bone_ids(),
        [Some(25), None, Some(24), Some(26)]
    );
    assert_eq!(targets.parts, PARTS.map(Some));
    pose[25][12] = 77.0;
    let previous = *targets.targets();
    assert_eq!(
        targets.try_calculate(&pose),
        Err(SkeletonIkTargetError::PoseUnavailable {
            target: 1,
            bone: None,
            pose_count: 28,
        })
    );
    assert_eq!(targets.targets()[0][12].to_bits(), 77.0_f32.to_bits());
    assert_eq!(targets.targets()[1..], previous[1..]);
}

#[test]
fn short_current_pose_commits_new_prefix_and_preserves_old_suffix() {
    let (data, mut pose) = target_fixture();
    let mut targets = SkeletonIkTargets::new(data);
    targets.initialize();
    targets.try_calculate(&pose).unwrap();
    let previous = *targets.targets();
    pose[24][12] = 123.0;
    assert_eq!(
        targets.try_calculate(&pose[..25]),
        Err(SkeletonIkTargetError::PoseUnavailable {
            target: 1,
            bone: Some(25),
            pose_count: 25,
        })
    );
    assert_eq!(targets.targets()[0][12].to_bits(), 123.0_f32.to_bits());
    assert_eq!(targets.targets()[1..], previous[1..]);
    // Init does not reset target transforms; Reset's target slice does.
    let partial = *targets.targets();
    targets.initialize();
    assert_eq!(*targets.targets(), partial);
    targets.reset_target_transforms();
    assert_eq!(
        *targets.target_bone_ids(),
        [Some(24), Some(25), Some(26), Some(27)]
    );
    for matrix in targets.targets() {
        assert_eq!(
            std::array::from_fn::<_, 4, _>(|lane| matrix[12 + lane].to_bits()),
            [0; 4]
        );
    }
}

#[test]
fn pose_failure_precedes_invalid_part_and_neither_overwrites_current_target() {
    let (data, pose) = target_fixture();
    let mut targets = SkeletonIkTargets::new(data);
    targets.initialize();
    targets.parts[0] = None;
    let previous = *targets.targets();
    assert_eq!(
        targets.try_calculate(&[]),
        Err(SkeletonIkTargetError::PoseUnavailable {
            target: 0,
            bone: Some(24),
            pose_count: 0,
        })
    );
    assert_eq!(
        targets.try_calculate(&pose),
        Err(SkeletonIkTargetError::InvalidPart { target: 0 })
    );
    assert_eq!(*targets.targets(), previous);
}

#[test]
fn target_uses_reparented_pose_and_actual_part_offset_with_fused_full_lane_arithmetic() {
    let (mut hierarchy, mut parameters, mut pose) = fixture();
    for name in TARGET_NAMES {
        append_target(&mut hierarchy, name);
        pose.push(pose[0]);
    }
    parameters.bones[15].position = [4097.0, 0.0, 0.0, f32::NAN];
    // 4097 squared = 16785409: the source fused translation retains +1.
    // An initial separate product rounds to 16785408 and instead yields zero.
    pose[24] = [
        4097.0,
        2.0,
        3.0,
        5.0,
        7.0,
        11.0,
        13.0,
        17.0,
        19.0,
        23.0,
        29.0,
        31.0,
        -16_785_408.0,
        37.0,
        41.0,
        43.0,
    ];
    pose[15][12] = -99.0;
    let data = Arc::new(build(hierarchy, parameters, &pose).unwrap());
    let mut targets = SkeletonIkTargets::new(data);
    targets.initialize();
    targets.try_calculate(&pose).unwrap();
    let actual = targets.targets()[0];
    assert_eq!(actual[12].to_bits(), 1.0_f32.to_bits());
    assert_eq!(actual[13].to_bits(), 8231.0_f32.to_bits());
    assert_eq!(actual[14].to_bits(), 12332.0_f32.to_bits());
    assert_eq!(actual[15].to_bits(), 20528.0_f32.to_bits());
    assert!(actual.iter().all(|lane| lane.is_finite()));
    assert_ne!(actual[3].to_bits(), 0);
}
