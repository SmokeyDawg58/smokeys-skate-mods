use super::{
    TrajectoryError, TrajectoryFields, TrajectoryInput, TrajectoryPose, try_process_trajectory,
};
use crate::{
    input_cvar_runtime::{InputCvarManager, InputCvarReferences},
    reckoning_numeric::HostReckoningNumeric,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
const IDENTITY: [f32; 16] = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;

#[test]
fn retained_unknown_pose_keeps_native_current_copy_before_failure() {
    use crate::actor_construction::anim_out_phys_in::{
        AnimOutPhysInOwner,
        pose_lanes::{PoseAccessError, PoseBuffer},
    };
    let mut owner = AnimOutPhysInOwner::new_sim_allocation();
    owner.try_construct(1).unwrap();
    let mut matrix = owner.pose_matrix_typed(PoseBuffer::Global, 0).unwrap();
    matrix.set_column(
        3,
        SkeletonRootVector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        },
    );
    owner
        .set_pose_matrix_typed(PoseBuffer::Global, 0, &matrix)
        .unwrap();
    let mut next = SkeletonRootMatrix::known(IDENTITY);
    let mut current = [7.0; 16];
    let mut inverse = [11.0; 16];
    let mut velocity = SkeletonRootVector::known([13.0; 4]);
    let result = try_process_trajectory(
        TrajectoryFields {
            current: &mut current,
            inverse: &mut inverse,
            next: &mut next,
            velocity: &mut velocity,
        },
        &TrajectoryInput {
            pose: TrajectoryPose::Retained(&owner),
            trajectory_node: 0,
            processed_dt: Some(0.5),
            animation_to_world: &SkeletonRootMatrix::known(IDENTITY),
        },
        &InputCvarReferences::new_static_initialized(),
        &mut HostReckoningNumeric,
    );
    assert!(matches!(
        result,
        Err(TrajectoryError::Pose(PoseAccessError::UnknownW(
            PoseBuffer::Global
        )))
    ));
    assert_eq!(current.map(f32::to_bits), IDENTITY.map(f32::to_bits));
    assert_eq!(
        next.known_lanes().unwrap().map(f32::to_bits),
        IDENTITY.map(f32::to_bits)
    );
    assert_eq!(inverse.map(f32::to_bits), [11.0f32.to_bits(); 16]);
    assert_eq!(
        velocity.known_lanes().unwrap().map(f32::to_bits),
        [13.0f32.to_bits(); 4]
    );
}

#[test]
fn unknown_basis_w_preserves_known_velocity_xyz_and_late_missing_dt_writes() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut columns = *SkeletonRootMatrix::known(IDENTITY).columns();
    columns[0].w = None;
    let world = SkeletonRootMatrix::from_columns(columns);
    let mut current = [7.0; 16];
    let mut inverse = [11.0; 16];
    let mut velocity = SkeletonRootVector::known([13.0; 4]);
    let mut pose = IDENTITY;
    pose[12..15].copy_from_slice(&[2.0, 3.0, 5.0]);
    for dt in [None, Some(0.5)] {
        let mut next = SkeletonRootMatrix::known(IDENTITY);
        let result = try_process_trajectory(
            TrajectoryFields {
                current: &mut current,
                inverse: &mut inverse,
                next: &mut next,
                velocity: &mut velocity,
            },
            &TrajectoryInput {
                pose: TrajectoryPose::Known(Some(&[pose])),
                trajectory_node: 0,
                processed_dt: dt,
                animation_to_world: &world,
            },
            &refs,
            &mut HostReckoningNumeric,
        );
        assert_eq!(
            next.known_lanes().unwrap().map(f32::to_bits),
            pose.map(f32::to_bits)
        );
        if dt.is_none() {
            assert!(matches!(
                result,
                Err(TrajectoryError::Missing("processed dt"))
            ));
            assert_eq!(
                velocity.xyz.map(f32::to_bits),
                [13.0f32; 3].map(f32::to_bits)
            );
        } else {
            result.unwrap();
            assert_eq!(
                velocity.xyz.map(f32::to_bits),
                [4.0f32, 6.0, 10.0].map(f32::to_bits)
            );
            assert_eq!(velocity.w, None);
        }
    }
}

#[test]
fn trajectory_accumulates_then_derives_velocity_before_zero_override() {
    for zero in [false, true] {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        if zero {
            manager
                .try_deserialize_startup(Some(b"[Config]\r\nkZeroTrajectoryTransform=1\r\n"))
                .unwrap();
        }
        let mut current = [19.0; 16];
        let mut inverse = [23.0; 16];
        let mut prior = IDENTITY;
        prior[12..15].copy_from_slice(&[2.0, 3.0, 4.0]);
        let mut next = SkeletonRootMatrix::known(prior);
        let mut velocity = SkeletonRootVector::known([0.0; 4]);
        let mut pose = IDENTITY;
        pose[0] = 0.0;
        pose[2] = -1.0;
        pose[8] = 1.0;
        pose[10] = 0.0;
        pose[12..15].copy_from_slice(&[5.0, 7.0, 11.0]);
        let mut world = IDENTITY;
        world[0] = 0.0;
        world[2] = -1.0;
        world[8] = 1.0;
        world[10] = 0.0;
        let world = SkeletonRootMatrix::known(world);
        try_process_trajectory(
            TrajectoryFields {
                current: &mut current,
                inverse: &mut inverse,
                next: &mut next,
                velocity: &mut velocity,
            },
            &TrajectoryInput {
                pose: TrajectoryPose::Known(Some(&[pose])),
                trajectory_node: 0,
                processed_dt: Some(0.5),
                animation_to_world: &world,
            },
            &refs,
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(
            velocity.xyz.map(f32::to_bits),
            [18.0f32, 20.0, -18.0].map(f32::to_bits)
        );
        assert_eq!(velocity.w.map(f32::to_bits), Some(0));
        let expected = if zero { [0.0f32; 3] } else { [9.0, 10.0, 9.0] };
        assert_eq!(
            current[12..15]
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            expected.map(f32::to_bits)
        );
        assert_eq!(
            next.known_lanes().unwrap().map(f32::to_bits),
            current.map(f32::to_bits)
        );
        assert_eq!(
            inverse[12..15]
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            (if zero {
                [0.0f32; 3]
            } else {
                [9.0, -10.0, -9.0]
            })
            .map(f32::to_bits)
        );
    }
}

#[test]
fn unavailable_pose_preserves_copy_and_enabled_display_reaches_shared_blr() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut current = [19.0; 16];
    let mut inverse = [23.0; 16];
    let mut next = SkeletonRootMatrix::known(IDENTITY);
    let mut velocity = SkeletonRootVector::known([29.0; 4]);
    let world = SkeletonRootMatrix::known(IDENTITY);
    let result = try_process_trajectory(
        TrajectoryFields {
            current: &mut current,
            inverse: &mut inverse,
            next: &mut next,
            velocity: &mut velocity,
        },
        &TrajectoryInput {
            pose: TrajectoryPose::Known(Some(&[])),
            trajectory_node: 0,
            processed_dt: Some(0.5),
            animation_to_world: &world,
        },
        &refs,
        &mut HostReckoningNumeric,
    );
    assert!(matches!(
        result,
        Err(TrajectoryError::InvalidTrajectoryNode(0))
    ));
    assert_eq!(current.map(f32::to_bits), IDENTITY.map(f32::to_bits));
    assert_eq!(
        velocity.xyz.map(f32::to_bits),
        [29.0f32; 3].map(f32::to_bits)
    );
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkSkeletonDisplayTrajectoryNode=1\r\n"))
        .unwrap();
    let result = try_process_trajectory(
        TrajectoryFields {
            current: &mut current,
            inverse: &mut inverse,
            next: &mut next,
            velocity: &mut velocity,
        },
        &TrajectoryInput {
            pose: TrajectoryPose::Known(Some(&[IDENTITY])),
            trajectory_node: 0,
            processed_dt: Some(0.5),
            animation_to_world: &world,
        },
        &refs,
        &mut HostReckoningNumeric,
    );
    result.unwrap();
    assert_eq!(inverse.map(f32::to_bits), IDENTITY.map(f32::to_bits));
    assert_eq!(velocity.xyz.map(f32::to_bits), [0u32; 3]);
}
