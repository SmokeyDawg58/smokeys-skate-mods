use super::{Arc, FIXED_SQT_BIT, Mutex, TreeCommands};
use crate::{
    actor_animation_construction::initialization::bind_pose::{
        BindPoseCommand, BufferedBodyCommands,
    },
    actor_animation_tree::eval::AnimationEvalCommands,
    actor_face_playback::{FaceAnimationManagers, FaceCommandPhase, FaceCommands},
};

impl TreeCommands {
    pub(crate) fn fixed_test_collector() -> (Self, Arc<Mutex<FaceCommands>>) {
        let managers = FaceAnimationManagers::new();
        let hierarchy = managers.database.lock().unwrap().allocate().unwrap();
        let scheduler = Arc::clone(&managers.command.lock().unwrap().scheduler);
        let owner = Arc::new(Mutex::new(BufferedBodyCommands::new(
            Arc::clone(&hierarchy),
            Arc::clone(&managers.database),
            scheduler,
        )));
        owner.lock().unwrap().max_dynamic_sqts = 4;
        let commands = Arc::new(Mutex::new(FaceCommands {
            database: hierarchy,
fixed_frames: vec![],
            fixed_matrices: Vec::new(),
            max_frames: 3,
            phase: FaceCommandPhase::Ready,
            scratch: None,
            selected_frames: vec![],
            setup: None,
            buffered: Some(Arc::clone(&owner)),
        }));
        (TreeCommands::new(owner), commands)
    }
}

#[test]
fn fixed_pair_allocates_one_dynamic_result_without_fetching_fixed_frames() {
    let (mut program, fixed) = TreeCommands::fixed_test_collector();
    program.set_fixed_sqt(&fixed, 1).unwrap();
    program.set_fixed_sqt(&fixed, 2).unwrap();
    assert_eq!(program.dynamic_count(), 0);
    assert!(program.fetches.is_empty());
    program.blend(0.5).unwrap();
    assert_eq!(program.dynamic_count(), 1);
    assert_eq!(program.output().unwrap(), 0);
    assert_eq!(
        program.operations,
        [BindPoseCommand::Blend {
            first: FIXED_SQT_BIT | 1,
            second: FIXED_SQT_BIT | 2,
            output: 0,
            coefficient: 4095
        }]
    );
    program.set_fixed_sqt(&fixed, 0).unwrap();
    program.blend(0.25).unwrap();
    assert_eq!(program.dynamic_count(), 1);
    assert_eq!(program.output().unwrap(), 0);
}

#[test]
fn fixed_frame_from_another_acs_is_rejected_without_appending_commands() {
    let (mut program, _) = TreeCommands::fixed_test_collector();
    let (_, foreign) = TreeCommands::fixed_test_collector();
    assert!(program.set_fixed_sqt(&foreign, 1).is_err());
    assert!(program.owner.lock().unwrap().commands.is_empty());
    assert_eq!(program.dynamic_count(), 0);
}

#[test]
fn hand_stream_extracts_last_producer_without_deleting_earlier_commands() {
    let (mut program, fixed) = TreeCommands::fixed_test_collector();
    program.set_fixed_sqt(&fixed, 1).unwrap();
    program.set_fixed_sqt(&fixed, 2).unwrap();
    assert_eq!(
        program.select_stream_extraction_root().unwrap(),
        FIXED_SQT_BIT | 2
    );
    assert_eq!(program.output().unwrap(), FIXED_SQT_BIT | 2);
    assert_eq!(program.owner.lock().unwrap().commands.len(), 2);
}
