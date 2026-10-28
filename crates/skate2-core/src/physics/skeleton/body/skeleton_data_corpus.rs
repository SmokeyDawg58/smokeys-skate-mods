//! Mounted original `PhysicsParam` plus an explicitly named original rig pose.
//! The supplied pose comes from `SkaterBodyPres`'s explicitly named RAW `rig_tpose`,
//! not the unrecovered live caller. `OnBoard`'s same-name poses are all VBR and
//! must not be interpreted by the raw sampler. Original hierarchies remain
//! distinct; matching packed bone names establish the shared index meanings.
use crate::{
    animation::{AnimationMatrix, AnimationSkeleton, LocalPose},
    ragdoll::SkeletonPartIndex,
    skeleton_data::{SkeletonData, SkeletonPoseView},
};
use skate2_formats::{
    andale::{AndaleDatabase, AndaleHierarchy, AndalePhysicsParameters},
    mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
};
use std::{path::Path, sync::Arc};

fn signature(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mounted_named_physics_and_controlled_rig_pose_construct_skeleton_data() {
    with_mounted_skeleton_data(|_| {});
}

#[expect(
    clippy::too_many_lines,
    reason = "one independently fingerprinted hierarchy/physics/pose ownership proof"
)]
pub(crate) fn with_mounted_skeleton_inputs(
    inspect: impl FnOnce(
        Arc<AndaleHierarchy>,
        Arc<AndalePhysicsParameters>,
        Arc<AndaleHierarchy>,
        Arc<[AnimationMatrix]>,
    ),
) {
    let root = std::env::var("SKATE2_GAME_ROOT").expect("set the original external game-data root");
    let archive = SKATE2_ARCHIVES
        .iter()
        .find(|archive| archive.relative_path == "data\\big\\anim.big")
        .unwrap();
    assert!(archive.use_external_header);
    let mut mounts = MountSet::new();
    mounts.push(
        MountedBig::open(
            archive.path_under(Path::new(&root)),
            archive.use_external_header,
        )
        .unwrap(),
    );
    let bytes = mounts.read_path("data\\anim\\OnBoard.abin").unwrap();
    let database = AndaleDatabase::parse(&bytes).unwrap();
    let parameters: Vec<_> = database
        .physics_parameters
        .iter()
        .filter(|parameters| parameters.name == "phys_tpose")
        .collect();
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0].record_offset, 0x2e00);
    let physics_record = database
        .records
        .iter()
        .find(|record| record.offset == parameters[0].record_offset)
        .unwrap();
    assert_eq!(physics_record.header.data_type, 5);
    assert_eq!(physics_record.header.size, 0xac0);
    assert_eq!(
        signature(&bytes[physics_record.offset..physics_record.data_range.end]),
        0xadc2_0621_dd1c_e628
    );

    assert_eq!(database.poses.len(), 72);
    assert!(
        database
            .records
            .iter()
            .filter(|record| record.header.data_type == 3)
            .all(|record| record.header.codec == 0x0052_4256)
    );
    let pose_bytes = mounts.read_path("data\\anim\\SkaterBodyPres.abin").unwrap();
    let pose_database = AndaleDatabase::parse(&pose_bytes).unwrap();
    let pose_records: Vec<_> = pose_database
        .records
        .iter()
        .filter(|record| {
            record.header.data_type == 3 && record.header.name.decode().unwrap() == "rig_tpose"
        })
        .collect();
    assert_eq!(pose_records.len(), 1);
    let pose_record = pose_records[0];
    assert_eq!(pose_record.header.codec, 0x0000_4452);
    assert_eq!(
        (pose_record.offset, pose_record.header.size),
        (0x580, 0x3d0)
    );
    assert_eq!(
        signature(&pose_bytes[pose_record.offset..pose_record.data_range.end]),
        0xc945_75ce_e13f_e704
    );
    let pose = pose_database
        .poses
        .iter()
        .find(|pose| pose.record_offset == pose_record.offset)
        .unwrap();
    assert_eq!(
        pose_database.hierarchy.bone_names,
        database.hierarchy.bone_names
    );
    assert_eq!(
        pose_database.hierarchy.packed_bone_names,
        database.hierarchy.packed_bone_names
    );
    assert_eq!(pose_database.hierarchy.parts, database.hierarchy.parts);
    let different_parents: Vec<_> = database
        .hierarchy
        .parents
        .iter()
        .zip(&pose_database.hierarchy.parents)
        .enumerate()
        .filter_map(|(bone, (mapping, supplied))| {
            (mapping != supplied).then_some((bone, *mapping, *supplied))
        })
        .collect();
    assert_eq!(
        different_parents,
        [
            (1, 0, -1),
            (25, 0, -1),
            (32, 25, -1),
            (33, 25, -1),
            (34, 25, -1),
            (35, 25, -1)
        ]
    );
    let hierarchy_records: Vec<_> = database
        .records
        .iter()
        .filter(|record| record.header.data_type == 4)
        .collect();
    assert_eq!(hierarchy_records.len(), 1);
    let hierarchy_record = hierarchy_records[0];
    assert_eq!(hierarchy_record.header.name.decode().unwrap(), "rig_tpose");
    assert_eq!(
        (hierarchy_record.offset, hierarchy_record.header.size),
        (0x30, 0x550)
    );
    assert_eq!(
        signature(&bytes[hierarchy_record.offset..hierarchy_record.data_range.end]),
        0x0bc8_8016_eab1_90d9
    );
    let pose_hierarchy_records: Vec<_> = pose_database
        .records
        .iter()
        .filter(|record| record.header.data_type == 4)
        .collect();
    assert_eq!(pose_hierarchy_records.len(), 1);
    let pose_hierarchy_record = pose_hierarchy_records[0];
    assert_eq!(
        (
            pose_hierarchy_record.offset,
            pose_hierarchy_record.header.size
        ),
        (0x30, 0x550)
    );
    assert_eq!(
        signature(&pose_bytes[pose_hierarchy_record.offset..pose_hierarchy_record.data_range.end]),
        0x12e7_f9ba_8e82_8c41
    );
    let sqt = pose.sample_raw_sqt(&pose_database.hierarchy).unwrap();
    assert_eq!(sqt.len(), 36);
    let skeleton = AnimationSkeleton::from_andale(&pose_database.hierarchy).unwrap();
    let matrices = skeleton
        .global_matrices(&LocalPose::from_andale(&sqt))
        .unwrap();
    let hierarchy = Arc::new(database.hierarchy.clone());
    let parameters = Arc::new(parameters[0].clone());
    let pose_hierarchy = Arc::new(pose_database.hierarchy.clone());
    // Return verified original inputs without constructing SkeletonData. The
    // paired physical producer must finish its real board FIRST. Sampling this
    // explicitly named RAW fixture is not a live Actor pose-selection claim.
    drop(database);
    drop(bytes);
    drop(pose_database);
    drop(pose_bytes);
    inspect(hierarchy, parameters, pose_hierarchy, matrices.into());
}

#[expect(
    clippy::too_many_lines,
    reason = "existing retained SkeletonData verification is preserved after separating original input loading"
)]
pub(crate) fn with_mounted_skeleton_data(inspect: impl FnOnce(&Arc<SkeletonData>)) {
    with_mounted_skeleton_inputs(|hierarchy, parameters, pose_hierarchy, matrices| {
        let data = Arc::new(
            SkeletonData::try_new(
                Arc::clone(&hierarchy),
                Arc::clone(&parameters),
                SkeletonPoseView {
                    hierarchy: &pose_hierarchy,
                    matrices: &matrices,
                },
            )
            .unwrap(),
        );
        assert!(Arc::ptr_eq(data.hierarchy(), &hierarchy));
        assert!(Arc::ptr_eq(data.parameters(), &parameters));
        assert_eq!(
            *data.bone_ids(),
            [
                25, 7, 6, 16, 15, 14, 13, 12, 11, 10, 9, 5, 4, 3, 2, 24, 23, 22, 21, 20, 19, 18,
                17, 1
            ]
        );
        let expected_parents = [
            None,
            Some(2),
            Some(11),
            Some(4),
            Some(5),
            Some(6),
            Some(11),
            Some(8),
            Some(9),
            Some(10),
            Some(11),
            Some(12),
            Some(13),
            Some(14),
            Some(23),
            Some(16),
            Some(17),
            Some(18),
            Some(23),
            Some(20),
            Some(21),
            Some(22),
            Some(23),
            None,
        ];
        assert_eq!(
            data.parents()
                .map(|parent| parent.map(SkeletonPartIndex::index)),
            expected_parents
        );
        assert_eq!(data.joints().len(), 22);
        for (joint, mapping) in data.joints().iter().enumerate() {
            assert_eq!(mapping.child.index(), joint + 1);
            assert_eq!(Some(mapping.parent.index()), expected_parents[joint + 1]);
            assert_eq!(data.part_joint_indices()[joint + 1], Some(joint));
        }
        assert_eq!(data.part_joint_indices()[0], None);
        assert_eq!(data.part_joint_indices()[23], None);
        assert_eq!(
            data.children(SkeletonPartIndex::Hips)
                .iter()
                .map(|part| part.index())
                .collect::<Vec<_>>(),
            [14, 18, 22]
        );
        assert_eq!(
            data.children(SkeletonPartIndex::Spine3)
                .iter()
                .map(|part| part.index())
                .collect::<Vec<_>>(),
            [2, 6, 10]
        );
        for (part, bone) in data.bone_ids().iter().copied().enumerate() {
            assert_eq!(
                data.physics_pose()[part].map(f32::to_bits),
                matrices[bone].map(f32::to_bits)
            );
            assert_eq!(data.part_from_bone()[bone].unwrap().index(), part);
            assert!(
                data.bone_to_volume()[part]
                    .iter()
                    .chain(&data.volume_to_bone()[part])
                    .all(|value| value.is_finite())
            );
        }
        let weak_hierarchy = Arc::downgrade(&hierarchy);
        let weak_parameters = Arc::downgrade(&parameters);
        drop(hierarchy);
        drop(parameters);
        assert!(weak_hierarchy.upgrade().is_some());
        assert!(weak_parameters.upgrade().is_some());
        inspect(&data);
        drop(data);
        assert!(weak_hierarchy.upgrade().is_none());
        assert!(weak_parameters.upgrade().is_none());
        println!(
            "original mounted OnBoard phys_tpose24 + SkaterBodyPres named RAW rig_tpose36 from distinct index-compatible hierarchy -> retained mappings/22 joints/matrices; controlled supplied-pose fixture, not live Skeleton::Init pose provenance or native numeric trace"
        );
    });
}
