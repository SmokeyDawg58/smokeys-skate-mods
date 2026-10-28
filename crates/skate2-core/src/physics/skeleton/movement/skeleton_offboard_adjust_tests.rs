//! Frozen full4 mapping and source failure-prefix tests; no native trace claim.
use super::*;
use crate::skeleton_data_tests::{build, fixture};
use skate2_formats::andale::{AndaleHierarchy, AndalePhysicsParameters, PackedFastString};
use std::sync::Arc;

const LEFT_MAPPING: AnimationMatrix = [
    2.0, 0.0, 0.0, 22.0, 0.0, 6.0, 0.0, 39.0, 0.0, 0.0, 12.0, 68.0, 9.0, -13.0, -55.0, -789.0,
];
const RIGHT_MAPPING: AnimationMatrix = [
    0.0, 0.0, -1.0, 1.0, 0.0, 1.0, 0.0, 2.0, 1.0, 0.0, 0.0, 3.0, 1.0, 3.0, 7.0, -7.0,
];

fn inputs() -> (
    AndaleHierarchy,
    AndalePhysicsParameters,
    Vec<AnimationMatrix>,
) {
    let (mut hierarchy, mut parameters, mut pose) = fixture();
    for name in [
        "LeftToeBase_Reparented",
        "RightToeBase_Reparented",
        "LeftHand_Reparented",
        "RightHand_Reparented",
    ] {
        hierarchy.bone_names.push(name.into());
        hierarchy
            .packed_bone_names
            .push(PackedFastString::encode(name).unwrap());
        hierarchy.parents.push(-1);
        hierarchy.bone_metadata.push(0);
        pose.push(pose[0]);
    }
    // Authored volume offsets are deliberately large: raw hand mapping must
    // not consume them or the current IK target-volume buffer.
    parameters.bones[3].position = [100.0, 200.0, 300.0, 400.0];
    parameters.bones[7].position = [-100.0, -200.0, -300.0, -400.0];
    pose[3] = [
        1.0, 0.0, 0.0, 11.0, 0.0, 2.0, 0.0, 13.0, 0.0, 0.0, 3.0, 17.0, 19.0, 23.0, 29.0, 31.0,
    ];
    pose[7] = [
        0.0, 0.0, -1.0, 1.0, 0.0, 1.0, 0.0, 2.0, 1.0, 0.0, 0.0, 3.0, 4.0, 5.0, 6.0, 7.0,
    ];
    // Non-unit target axes discriminate rigid transpose from a general inverse
    // or normalization. All target W lanes are ignored by the source mapping.
    pose[26] = [
        2.0,
        0.0,
        0.0,
        f32::NAN,
        0.0,
        3.0,
        0.0,
        f32::NAN,
        0.0,
        0.0,
        4.0,
        f32::NAN,
        5.0,
        6.0,
        7.0,
        f32::NAN,
    ];
    pose[27][12..16].copy_from_slice(&[1.0, 2.0, 3.0, f32::NAN]);
    (hierarchy, parameters, pose)
}

fn initialized() -> (SkeletonIkTargets, Vec<AnimationMatrix>) {
    let (hierarchy, parameters, pose) = inputs();
    let mut ik = SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()));
    ik.initialize();
    (ik, pose)
}

struct Frames {
    results: Vec<Result<f32, &'static str>>,
    calls: usize,
}
impl SkeletonSkateboardOffsetCvars for Frames {
    type Error = &'static str;
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        let result = self.results[self.calls];
        self.calls += 1;
        result
    }
}
fn previous_offset() -> SkeletonSkateboardOffset {
    let mut offset = SkeletonSkateboardOffset::new();
    offset.transform[12..16].copy_from_slice(&[101.0, 103.0, 107.0, 109.0]);
    offset.full_count = 5.0;
    offset.vertical_count = 11.0;
    offset
}

#[test]
fn both_mirrors_use_raw_hands_rigid_target_inverse_and_all_four_actual_lanes() {
    let (mut ik, pose) = initialized();
    let original_targets = ik.targets().map(|matrix| matrix.map(f32::to_bits));
    assert_eq!(
        ik.try_hand_to_animation_mapping(&pose, true)
            .unwrap()
            .map(f32::to_bits),
        LEFT_MAPPING.map(f32::to_bits)
    );
    assert_eq!(
        ik.try_hand_to_animation_mapping(&pose, false)
            .unwrap()
            .map(f32::to_bits),
        RIGHT_MAPPING.map(f32::to_bits)
    );
    assert_eq!(
        ik.targets().map(|matrix| matrix.map(f32::to_bits)),
        original_targets
    );
    // Even after changing every retained target, raw hand mapping is unchanged.
    ik.add_offset_transform(&RIGHT_MAPPING);
    assert_ne!(
        ik.targets().map(|matrix| matrix.map(f32::to_bits)),
        original_targets
    );
    assert_eq!(
        ik.try_hand_to_animation_mapping(&pose, true)
            .unwrap()
            .map(f32::to_bits),
        LEFT_MAPPING.map(f32::to_bits)
    );
    assert_eq!(
        ik.try_hand_to_animation_mapping(&pose, false)
            .unwrap()
            .map(f32::to_bits),
        RIGHT_MAPPING.map(f32::to_bits)
    );
}

#[test]
fn target_pose_failure_precedes_actual_pose_failure_and_preinit_ids_remain_unknown() {
    let (mut hierarchy, parameters, pose) = inputs();
    // Actual left hand now lives at26, and named left target lives at3.
    hierarchy.bone_names.swap(3, 26);
    hierarchy.packed_bone_names.swap(3, 26);
    let mut ik = SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()));
    assert_eq!(
        ik.try_hand_to_animation_mapping(&pose, true),
        Err(SkeletonIkTargetError::NotInitialized)
    );
    ik.initialize();
    assert_eq!(
        ik.try_hand_to_animation_mapping(&pose[..3], true),
        Err(SkeletonIkTargetError::PoseUnavailable {
            target: 2,
            bone: Some(3),
            pose_count: 3,
        })
    );
    assert_eq!(
        ik.try_hand_to_animation_mapping(&pose[..4], true),
        Err(SkeletonIkTargetError::ActualHandPoseUnavailable {
            target: 2,
            bone: Some(26),
            pose_count: 4,
        })
    );
}

#[test]
fn missing_named_hand_stops_before_offset_mutation_or_any_cvar_read() {
    let (mut hierarchy, parameters, pose) = inputs();
    hierarchy.packed_bone_names[26] = PackedFastString::encode("other_bone").unwrap();
    let mut ik = SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()));
    ik.initialize();
    let mut offset = previous_offset();
    let original = offset.transform.map(f32::to_bits);
    let mut frames = Frames {
        results: Vec::new(),
        calls: 0,
    };
    assert_eq!(
        try_update_offboard_adjust(&ik, &pose, &mut offset, true, &mut frames),
        Err(SkeletonOffboardAdjustError::Ik(
            SkeletonIkTargetError::PoseUnavailable {
                target: 2,
                bone: None,
                pose_count: 28
            }
        ))
    );
    assert_eq!(frames.calls, 0);
    assert_eq!(offset.transform.map(f32::to_bits), original);
    assert_eq!(offset.full_count.to_bits(), 5.0_f32.to_bits());
    assert_eq!(offset.vertical_count.to_bits(), 11.0_f32.to_bits());
    assert!(!offset.full_just_set && !offset.vertical_just_set);
}

#[test]
fn first_blend_read_failure_preserves_new_full_transform_and_flag_with_old_counts() {
    let (ik, pose) = initialized();
    let mut offset = previous_offset();
    let mut frames = Frames {
        results: vec![Err("first")],
        calls: 0,
    };
    assert_eq!(
        try_update_offboard_adjust(&ik, &pose, &mut offset, true, &mut frames),
        Err(SkeletonOffboardAdjustError::Cvar("first"))
    );
    assert_eq!(frames.calls, 1);
    assert_eq!(
        offset.transform.map(f32::to_bits),
        LEFT_MAPPING.map(f32::to_bits)
    );
    assert!(offset.full_just_set);
    assert!(!offset.vertical_just_set);
    assert_eq!(offset.full_count.to_bits(), 5.0_f32.to_bits());
    assert_eq!(offset.vertical_count.to_bits(), 11.0_f32.to_bits());
}

#[test]
fn second_blend_read_failure_commits_full_count_before_any_y_flag_or_count_store() {
    let (ik, pose) = initialized();
    let mut offset = previous_offset();
    let mut frames = Frames {
        results: vec![Ok(7.5), Err("second")],
        calls: 0,
    };
    assert_eq!(
        try_update_offboard_adjust(&ik, &pose, &mut offset, false, &mut frames),
        Err(SkeletonOffboardAdjustError::Cvar("second"))
    );
    assert_eq!(frames.calls, 2);
    assert_eq!(
        offset.transform.map(f32::to_bits),
        RIGHT_MAPPING.map(f32::to_bits)
    );
    assert!(offset.full_just_set);
    assert!(!offset.vertical_just_set);
    assert_eq!(offset.full_count.to_bits(), 7.5_f32.to_bits());
    assert_eq!(offset.vertical_count.to_bits(), 11.0_f32.to_bits());
}

#[test]
fn successful_offboard_prepares_both_independent_float_counts_without_applying_offsets() {
    let (ik, pose) = initialized();
    let original_targets = ik.targets().map(|matrix| matrix.map(f32::to_bits));
    let mut offset = previous_offset();
    let mut frames = Frames {
        results: vec![Ok(3.5), Ok(-2.25)],
        calls: 0,
    };
    try_update_offboard_adjust(&ik, &pose, &mut offset, true, &mut frames).unwrap();
    assert_eq!(frames.calls, 2);
    assert_eq!(offset.full_count.to_bits(), 3.5_f32.to_bits());
    assert_eq!(offset.vertical_count.to_bits(), (-2.25_f32).to_bits());
    assert!(offset.full_just_set && offset.vertical_just_set);
    assert_eq!(
        offset.transform.map(f32::to_bits),
        LEFT_MAPPING.map(f32::to_bits)
    );
    assert_eq!(
        ik.targets().map(|matrix| matrix.map(f32::to_bits)),
        original_targets
    );
}
