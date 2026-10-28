use super::*;
use crate::{
    actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer,
    reckoning_numeric::HostReckoningNumeric,
};
fn buffer() -> OwnedAnimationSqtBuffer {
    let buffer = OwnedAnimationSqtBuffer::allocate_unknown(1);
    buffer
        .extract(&[AndaleSqt {
            scale: [7.; 4],
            rotation: [0., 0., 0., 1.],
            translation: [2., 3., 4., 1.],
        }])
        .unwrap();
    buffer.complete_sync().unwrap();
    buffer
}
#[test]
fn actual_allocation_late_board_failure_retains_hips_transform_and_invalidates_publication() {
    let replay = buffer();
    let mut transform = None;
    let result = replay.try_edit_channels(&replay, |channels| {
        try_move_root_channels(channels, 0, 4, &mut transform, &mut HostReckoningNumeric)
    });
    assert!(matches!(
        result,
        Err(SqtMutationError::Operation(MoveRootError::MissingBone(4)))
    ));
    assert_eq!(
        transform.unwrap()[12..]
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        [2., 3., 4., 1.].map(f32::to_bits)
    );
    assert!(replay.read().is_err());
}
#[test]
fn actual_alias_observes_scale_translation_and_rotation_stores() {
    let replay = buffer();
    replay
        .try_edit_channels(&replay, |channels| {
            channels.scale(0, [1.; 4]).unwrap();
            channels.rotation(0, [0.; 4]).unwrap();
            channels.translation(0, [9.; 4]).unwrap();
            let aliased = channels.bind(0).unwrap();
            assert_eq!(aliased.scale.map(f32::to_bits), [1.; 4].map(f32::to_bits));
            assert_eq!(
                aliased.rotation.map(f32::to_bits),
                [0.; 4].map(f32::to_bits)
            );
            Ok::<(), std::convert::Infallible>(())
        })
        .unwrap();
    assert_eq!(
        replay.read().unwrap().1[0].translation.map(f32::to_bits),
        [9.; 4].map(f32::to_bits)
    );
}

#[test]
fn zero_count_does_not_require_indices_bind_or_global() {
    let animation =
        PresentationAnimation::new(crate::actor_face_playback::FaceAnimationManagers::new());
    animation
        .try_correct_replay(
            0,
            None,
            &mut HostReckoningNumeric,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        )
        .unwrap();
}
#[test]
fn signed_missing_board_fails_after_actual_hips_stores() {
    let mut animation =
        PresentationAnimation::new(crate::actor_face_playback::FaceAnimationManagers::new());
    animation.hips = Some(0);
    animation.skateboard = Some(-1);
    animation.replay.push(buffer());
    let result = animation.try_correct_replay(
        1,
        None,
        &mut HostReckoningNumeric,
        &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
    );
    assert!(matches!(
        result,
        Err(CorrectionError::Move(SqtMutationError::Operation(
            MoveRootError::MissingBone(_)
        )))
    ));
    assert!(animation.hips_transforms.lock().unwrap()[0].is_some());
    assert!(animation.replay[0].read().is_err());
}
