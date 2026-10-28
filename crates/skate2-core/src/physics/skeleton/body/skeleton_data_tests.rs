//! Synthetic inputs discriminate the recovered producer; they are not an
//! authored humanoid definition or a native numeric trace.
use crate::{
    ragdoll::SkeletonPartIndex as Part,
    skeleton_data::{SkeletonData, SkeletonDataError as Error, SkeletonPoseView},
};
use skate2_formats::andale::{
    AndaleHierarchy, AndalePhysicsBone, AndalePhysicsParameters, PackedFastString,
};
use std::sync::Arc;

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

pub(crate) fn fixture() -> (AndaleHierarchy, AndalePhysicsParameters, Vec<[f32; 16]>) {
    let names: Vec<_> = (0..24).map(|index| format!("p{index}")).collect();
    let packed: Vec<_> = names
        .iter()
        .map(|name| PackedFastString::encode(name).unwrap())
        .collect();
    let hierarchy = AndaleHierarchy {
        has_trajectory: false,
        parents: (0..24)
            .map(|index| {
                if index == 0 || index == 23 {
                    -1
                } else {
                    index + 1
                }
            })
            .collect(),
        bone_metadata: vec![0; 24],
        parts: Vec::new(),
        bone_names: names.clone(),
        packed_bone_names: packed.clone(),
    };
    let parameters = AndalePhysicsParameters {
        record_offset: 0,
        name: "synthetic".into(),
        guid: 0,
        bones: names
            .into_iter()
            .zip(packed)
            .map(|(name, packed_name)| AndalePhysicsBone {
                parent_orientation: [0.0, 0.0, 0.0, 1.0],
                joint_orientation: [0.0, 0.0, 0.0, 1.0],
                size: [1.0; 4],
                orientation: [0.0, 0.0, 0.0, 1.0],
                position: [0.0; 4],
                twist: 0.0,
                swing: 0.0,
                mass: 1.0,
                name,
                packed_name,
            })
            .collect(),
    };
    (hierarchy, parameters, vec![IDENTITY; 24])
}

pub(crate) fn build(
    hierarchy: AndaleHierarchy,
    parameters: AndalePhysicsParameters,
    matrices: &[[f32; 16]],
) -> Result<SkeletonData, Error> {
    let hierarchy = Arc::new(hierarchy);
    SkeletonData::try_new(
        Arc::clone(&hierarchy),
        Arc::new(parameters),
        SkeletonPoseView {
            hierarchy: &hierarchy,
            matrices,
        },
    )
}

fn append_bone(hierarchy: &mut AndaleHierarchy, name: &str, parent: i32) {
    hierarchy.bone_names.push(name.into());
    hierarchy
        .packed_bone_names
        .push(PackedFastString::encode(name).unwrap());
    hierarchy.parents.push(parent);
    hierarchy.bone_metadata.push(0);
}

#[test]
fn source_part_order_builds_joints_children_and_absent_root_entries() {
    let (mut hierarchy, parameters, pose) = fixture();
    hierarchy.parents.fill(-1);
    // Animation traversal order is not used to sort these physics children.
    for child in [9, 2, 6] {
        hierarchy.parents[child] = 23;
    }
    let copied_parents = hierarchy.parents.clone();
    let data = build(hierarchy, parameters, &pose).unwrap();
    assert_eq!(*data.bone_ids(), std::array::from_fn(|index| index));
    assert_eq!(
        data.joints()
            .iter()
            .map(|joint| (joint.parent.index(), joint.child.index()))
            .collect::<Vec<_>>(),
        [(23, 2), (23, 6), (23, 9)]
    );
    assert_eq!(
        data.children(Part::Hips),
        &[Part::Neck, Part::LeftShoulder, Part::RightArm]
    );
    assert_eq!(data.part_joint_indices()[2], Some(0));
    assert_eq!(data.part_joint_indices()[6], Some(1));
    assert_eq!(data.part_joint_indices()[9], Some(2));
    assert_eq!(data.part_joint_indices()[0], None);
    assert_eq!(data.parents()[0], None);
    assert!(data.children(Part::SkateboardRoot).is_empty());
    assert_eq!(data.animation_parents(), copied_parents);
}

#[test]
fn packed_hierarchy_lookup_is_first_match_and_reverse_physics_mapping_is_last_write() {
    let (mut hierarchy, mut parameters, mut pose) = fixture();
    append_bone(&mut hierarchy, "p0", -1);
    pose.push(IDENTITY);
    // Human-readable labels are not the native five-word matching key.
    parameters.bones[0].name = "deliberately_not_the_key".into();
    let data = build(hierarchy.clone(), parameters.clone(), &pose).unwrap();
    assert_eq!(data.bone_ids()[0], 0);
    assert_eq!(data.part_from_bone()[24], None);
    parameters.bones[0].packed_name = parameters.bones[23].packed_name;
    let data = build(hierarchy, parameters, &pose).unwrap();
    assert_eq!(data.bone_ids()[0], 23);
    assert_eq!(data.bone_ids()[23], 23);
    assert_eq!(data.part_from_bone()[23], Some(Part::Hips));
    assert_eq!(data.part_from_bone()[0], None);
    assert_eq!(data.parents()[0], None);
    assert_eq!(data.parents()[22], Some(Part::Hips));
}

#[test]
fn parent_walk_skips_unrepresented_ancestors_and_refuses_duplicate_unmapped_target() {
    let (mut hierarchy, parameters, mut pose) = fixture();
    append_bone(&mut hierarchy, "helper", 2);
    hierarchy.parents[1] = 24;
    pose.push(IDENTITY);
    let data = build(hierarchy.clone(), parameters.clone(), &pose).unwrap();
    assert_eq!(data.parents()[1], Some(Part::Neck));
    assert_eq!(data.part_from_bone()[24], None);
    // The ancestor now matches a physics name but GetBone selected its first
    // occurrence at index2. Native would load the initialized -1 reverse-map slot;
    // the checked host producer refuses it, rather than retargeting by name.
    hierarchy.packed_bone_names[24] = hierarchy.packed_bone_names[2];
    assert_eq!(
        build(hierarchy, parameters, &pose).unwrap_err(),
        Error::ParentPartUnavailable {
            part: Part::Neck1,
            bone: 24,
        }
    );
}

#[test]
fn nonterminating_unrepresented_walk_and_invalid_ancestors_are_explicit_errors() {
    let (mut hierarchy, parameters, pose) = fixture();
    hierarchy.parents[0] = -2;
    assert_eq!(
        build(hierarchy.clone(), parameters.clone(), &pose).unwrap_err(),
        Error::InvalidAncestor {
            part: Part::SkateboardRoot,
            bone: -2
        }
    );
    hierarchy.parents[0] = 24;
    assert_eq!(
        build(hierarchy.clone(), parameters.clone(), &pose).unwrap_err(),
        Error::InvalidAncestor {
            part: Part::SkateboardRoot,
            bone: 24
        }
    );
    append_bone(&mut hierarchy, "unrepresented", 24);
    assert_eq!(
        build(hierarchy, parameters, &pose).unwrap_err(),
        Error::AncestorCycle {
            part: Part::SkateboardRoot
        }
    );
}

#[test]
fn checked_capacities_do_not_silently_truncate_parts_joints_or_children() {
    let (hierarchy, mut parameters, pose) = fixture();
    parameters.bones.pop();
    assert_eq!(
        build(hierarchy, parameters, &pose).unwrap_err(),
        Error::PhysicsBoneCount { actual: 23 }
    );
    let (mut hierarchy, parameters, pose) = fixture();
    for index in 24..101 {
        append_bone(&mut hierarchy, &format!("extra{index}"), -1);
    }
    assert_eq!(
        build(hierarchy, parameters, &pose).unwrap_err(),
        Error::AnimationBoneCapacity { actual: 101 }
    );
    let (mut hierarchy, parameters, pose) = fixture();
    hierarchy.packed_bone_names.pop();
    assert_eq!(
        build(hierarchy, parameters, &pose).unwrap_err(),
        Error::HierarchyTableLengths {
            names: 24,
            packed_names: 23,
            parents: 24
        }
    );
    let (mut hierarchy, parameters, pose) = fixture();
    hierarchy.parents[0] = 1;
    assert_eq!(
        build(hierarchy, parameters, &pose).unwrap_err(),
        Error::JointCapacity {
            child: Part::RightUpLeg
        }
    );
    let (mut hierarchy, parameters, pose) = fixture();
    hierarchy.parents.fill(-1);
    for child in 1..=4 {
        hierarchy.parents[child] = 23;
    }
    assert_eq!(
        build(hierarchy, parameters, &pose).unwrap_err(),
        Error::ChildCapacity {
            parent: Part::Hips,
            child: Part::LeftForearm
        }
    );
}

#[test]
fn missing_named_bone_precedes_pose_checks() {
    let (hierarchy, mut parameters, _) = fixture();
    parameters.bones[3].packed_name = PackedFastString::encode("missing").unwrap();
    assert_eq!(
        build(hierarchy, parameters, &[]).unwrap_err(),
        Error::MissingBone {
            part: Part::LeftHand
        }
    );
}

#[test]
fn exact_animation_capacity_and_represented_self_parent_are_not_globally_rejected() {
    let (mut hierarchy, parameters, pose) = fixture();
    for index in 24..100 {
        append_bone(&mut hierarchy, &format!("unused{index}"), 999);
    }
    // Source stops as soon as an ancestor is represented. The host walk guard
    // must not become an invented global tree validator or reject unused rows.
    hierarchy.parents[1] = 1;
    let data = build(hierarchy, parameters, &pose).unwrap();
    assert_eq!(data.part_from_bone().len(), 100);
    assert!(data.part_from_bone()[24..].iter().all(Option::is_none));
    assert_eq!(data.animation_parents()[99], 999);
    assert_eq!(data.parents()[1], Some(Part::Neck1));
    assert_eq!(data.joints()[0].parent, Part::Neck1);
    assert_eq!(data.joints()[0].child, Part::Neck1);
}

#[test]
fn distinct_pose_hierarchy_is_allowed_and_full_pose_words_are_copied() {
    let (hierarchy, parameters, mut pose) = fixture();
    for (bone, matrix) in pose.iter_mut().enumerate() {
        for (lane, value) in matrix.iter_mut().enumerate() {
            *value = f32::from_bits(0x3f00_0000 + u32::try_from(bone * 16 + lane).unwrap());
        }
        matrix[3] = f32::from_bits(0x8000_0000);
        matrix[15] = f32::from_bits(0x7fc1_0000 + u32::try_from(bone).unwrap());
    }
    let mut pose_hierarchy = hierarchy.clone();
    pose_hierarchy
        .bone_names
        .fill("different_pose_label".into());
    let data = SkeletonData::try_new(
        Arc::new(hierarchy),
        Arc::new(parameters),
        SkeletonPoseView {
            hierarchy: &pose_hierarchy,
            matrices: &pose,
        },
    )
    .unwrap();
    assert_eq!(
        data.physics_pose()
            .map(|matrix| matrix.map(f32::to_bits))
            .as_slice(),
        pose.iter()
            .map(|matrix| matrix.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
}

#[test]
fn pose_hierarchy_count_and_matrix_availability_are_separate_checked_boundaries() {
    let (hierarchy, parameters, pose) = fixture();
    let mut pose_hierarchy = hierarchy.clone();
    pose_hierarchy.bone_names.truncate(3);
    assert_eq!(
        SkeletonData::try_new(
            Arc::new(hierarchy.clone()),
            Arc::new(parameters.clone()),
            SkeletonPoseView {
                hierarchy: &pose_hierarchy,
                matrices: &pose,
            }
        )
        .unwrap_err(),
        Error::PoseBoneOutsideHierarchy {
            part: Part::LeftHand,
            bone: 3,
            count: 3
        }
    );
    assert_eq!(
        build(hierarchy, parameters, &pose[..3]).unwrap_err(),
        Error::PoseMatrixUnavailable {
            part: Part::LeftHand,
            bone: 3,
            count: 3
        }
    );
}

#[test]
fn identity_expansion_preserves_noncanonical_axis_w_and_full_position() {
    let (hierarchy, mut parameters, pose) = fixture();
    parameters.bones[0].position = [3.0, 4.0, 5.0, f32::from_bits(0x7fc1_2345)];
    let data = build(hierarchy, parameters, &pose).unwrap();
    let forward = data.bone_to_volume()[0];
    assert_eq!(
        [forward[3], forward[7], forward[11]].map(f32::to_bits),
        [0x3f80_0000, 0, 0]
    );
    assert_eq!(forward[15].to_bits(), 0x7fc1_2345);
    let inverse = data.volume_to_bone()[0];
    assert_eq!(
        [inverse[3], inverse[7], inverse[11], inverse[15]].map(f32::to_bits),
        [0, 0x3f80_0000, 0, 0xc080_0000]
    );
    assert_eq!(
        inverse[12..15]
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        [-3.0_f32, -4.0, -5.0].map(f32::to_bits)
    );
}

#[test]
fn nonunit_orientation_is_not_normalized_and_inverse_is_not_rigid_transpose() {
    let (hierarchy, mut parameters, pose) = fixture();
    parameters.bones[0].orientation = [0.0, 0.0, 1.0, 1.0];
    parameters.bones[0].position = [3.0, 4.0, 5.0, -0.0];
    let data = build(hierarchy, parameters, &pose).unwrap();
    let matrix = data.bone_to_volume()[0];
    let inverse = data.volume_to_bone()[0];
    // Independent ordinary-finite algebraic oracle: the unnormalized matrix's
    // XY block is approximately [-1,-2;2,-1], whose inverse divides by five.
    for (actual, expected) in [matrix[0], matrix[1], matrix[4], matrix[5]]
        .into_iter()
        .zip([-1.0, 2.0, -2.0, -1.0])
    {
        assert!((actual - expected).abs() < 0.000_001);
    }
    for (actual, expected) in [inverse[0], inverse[1], inverse[4], inverse[5]]
        .into_iter()
        .zip([-0.2, -0.4, 0.4, -0.2])
    {
        assert!((actual - expected).abs() < 0.000_001);
    }
    assert!((inverse[1] - matrix[4]).abs() > 1.0);
    assert_eq!(matrix[15].to_bits(), 0x8000_0000);
    for column in 0..3 {
        for row in 0..3 {
            let product = (0..3)
                .map(|axis| matrix[axis * 4 + row] * inverse[column * 4 + axis])
                .sum::<f32>();
            assert!((product - f32::from(u8::from(row == column))).abs() < 0.000_001);
        }
    }
}

#[test]
fn exceptional_orientation_is_not_replaced_by_identity_or_a_default_inverse() {
    let (hierarchy, mut parameters, pose) = fixture();
    parameters.bones[0].orientation[0] = f32::from_bits(0x7fc1_2345);
    let data = build(hierarchy, parameters, &pose).unwrap();
    // Presence of NaNs is a host-policy discriminator, not a native NaN-payload
    // oracle. This producer has no finite-input rejection or identity fallback.
    assert!(data.bone_to_volume()[0].iter().any(|value| value.is_nan()));
    assert!(data.volume_to_bone()[0].iter().any(|value| value.is_nan()));
}
