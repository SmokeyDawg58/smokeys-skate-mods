use super::*;
use crate::skateboard_spawn_math::HostSkateboardSpawnMathNumeric;

#[test]
fn com_placement_uses_old_world_heading_and_preserves_unrelated_root_state() {
    let mut roots = SkeletonRootTransforms::new();
    let mut reckoning =
        SkeletonRootMatrix::translation(SkeletonRootVector::known([90.0, 80.0, 70.0, 0.0]));
    reckoning.set_column(0, SkeletonRootVector::known([0.0, 0.0, -1.0, 0.0]));
    reckoning.set_column(2, SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]));
    let old_board = roots.board;
    let old_anim_board = roots.animation_to_board;
    roots.publish_board_prediction(SkeletonRootVector::known([4.0; 4]));
    roots.update_known_air(
        &reckoning,
        [1.0, 2.0, 3.0, 99.0],
        SkeletonRootVector::known([11.0, 22.0, 33.0, 9.0]),
        &mut HostSkateboardSpawnMathNumeric,
    );
    assert_eq!(
        roots.animation_to_world.columns()[3],
        SkeletonRootVector::known([10.0, 20.0, 30.0, 9.0])
    );
    assert!(!roots.update_heading);
    assert_eq!(
        roots.animation_angle.columns()[3],
        SkeletonRootVector::known([0.0; 4])
    );
    assert_eq!(roots.board, old_board);
    assert_eq!(roots.animation_to_board, old_anim_board);
    assert!(roots.prediction_valid);
    assert_eq!(
        roots.predicted_board_position,
        SkeletonRootVector::known([4.0; 4])
    );
    let angle = roots.animation_angle;
    roots.update_known_air(
        &reckoning,
        [0.0; 4],
        SkeletonRootVector::known([0.0; 4]),
        &mut HostSkateboardSpawnMathNumeric,
    );
    assert_eq!(roots.animation_angle, angle);
}

#[test]
fn target_unknown_w_survives_normalization_without_poisoning_xyz() {
    let mut roots = SkeletonRootTransforms::new();
    let reckoning = roots.animation_to_world;
    roots.update_known_air(
        &reckoning,
        [1.0, 2.0, 3.0, 0.0],
        SkeletonRootVector {
            xyz: [8.0, 9.0, 10.0],
            w: None,
        },
        &mut HostSkateboardSpawnMathNumeric,
    );
    assert_eq!(
        roots.animation_to_world.columns()[3],
        SkeletonRootVector {
            xyz: [7.0; 3],
            w: None
        }
    );
    assert_eq!(
        roots.world_to_animation.columns()[3].xyz.map(f32::to_bits),
        [-7.0_f32; 3].map(f32::to_bits)
    );
}
