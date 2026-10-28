use super::*;
use crate::{
    actor_animation_construction::initialization::bind_pose::OwnedAnimationPoseBuffer,
    actor_face_playback::FaceAnimationManagers,
};

#[test]
fn foreign_presentation_and_wrong_replay_rows_fail_before_command_mutation() {
    use crate::actor_animation_construction::initialization::body_dispatch::tree_commands::TreeCommands;
    let (tree, commands) = TreeCommands::fixed_test_collector();
    let mut origin = PresentationAnimation::new(FaceAnimationManagers::new());
    origin
        .replay
        .push(OwnedAnimationSqtBuffer::allocate_unknown(1));
    origin
        .replay
        .push(OwnedAnimationSqtBuffer::allocate_unknown(1));
    origin
        .replay
        .push(OwnedAnimationSqtBuffer::allocate_unknown(1));
    let token = DecodedPresentationPass::from_read(&origin, commands, origin.replay[0].clone());
    let foreign = PresentationAnimation::new(FaceAnimationManagers::new());
    assert!(matches!(
        token.validate::<()>(&foreign, 0),
        Err(PresentationFinalPoseError::ForeignPresentation)
    ));
    assert!(matches!(
        token.validate::<()>(&origin, 1),
        Err(PresentationFinalPoseError::WrongReplayRow(1))
    ));
    token.validate::<()>(&origin, 0).unwrap();
    let owner = tree.owner.lock().unwrap();
    assert_eq!(owner.in_command_set, 0);
    assert!(owner.commands.is_empty());
}

#[test]
fn saved_hips_replace_only_zero_after_actual_sync() {
    let managers = FaceAnimationManagers::new();
    let database = managers.database.lock().unwrap().allocate().unwrap();
    let mut presentation = PresentationAnimation::new(managers);
    let output = OwnedAnimationPoseBuffer::allocate_unknown(database, 2).unwrap();
    let storage = output.shared_storage().unwrap();
    storage.lock().unwrap().values = vec![Some([2.0; 16]), Some([3.0; 16])];
    presentation.interpolated.push(output);
    presentation.hips_transforms.lock().unwrap()[0] = Some([7.0; 16]);
    assert!(publish_saved_hips::<()>(&presentation, 0).is_err());
    assert_eq!(storage.lock().unwrap().values[0], Some([2.0; 16]));
    storage.lock().unwrap().synchronized = true;
    publish_saved_hips::<()>(&presentation, 0).unwrap();
    assert_eq!(
        presentation.interpolated[0].matrices().unwrap().unwrap(),
        [[7.0; 16], [3.0; 16]]
    );
}

#[test]
fn missing_saved_hips_does_not_replace_completed_matrix_work() {
    let managers = FaceAnimationManagers::new();
    let database = managers.database.lock().unwrap().allocate().unwrap();
    let mut presentation = PresentationAnimation::new(managers);
    let output = OwnedAnimationPoseBuffer::allocate_unknown(database, 1).unwrap();
    let storage = output.shared_storage().unwrap();
    {
        let mut storage = storage.lock().unwrap();
        storage.values[0] = Some([4.0; 16]);
        storage.synchronized = true;
    }
    presentation.interpolated.push(output);
    assert!(matches!(
        publish_saved_hips::<()>(&presentation, 0),
        Err(PresentationFinalPoseError::Missing("saved hips transform"))
    ));
    assert_eq!(
        presentation.interpolated[0].matrices().unwrap().unwrap(),
        [[4.0; 16]]
    );
}
