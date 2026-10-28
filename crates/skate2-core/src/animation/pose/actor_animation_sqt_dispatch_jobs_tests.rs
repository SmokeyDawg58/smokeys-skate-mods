use super::{
    AndaleHierarchy, AndaleSqt, AnimationJobCode, AnimationJobWork, OwnedAnimationSqtBuffer,
    QueuedSqtProgram, SqtDispatchWork, TreeCommands,
};
use crate::actor_animation_tree::eval::AnimationEvalCommands;

fn program(blend: bool) -> QueuedSqtProgram {
    let (mut commands, fixed) = TreeCommands::fixed_test_collector();
    let pose = AndaleSqt {
        scale: [1.0; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [2.0, 3.0, 4.0, 1.0],
    };
    fixed.lock().unwrap().fixed_frames = vec![vec![pose]; 3];
    commands.set_fixed_sqt(&fixed, 0).unwrap();
    if blend {
        commands.set_fixed_sqt(&fixed, 1).unwrap();
        commands.blend(0.5).unwrap();
    }
    let output = OwnedAnimationSqtBuffer::allocate_unknown(1);
    output.begin_extract().unwrap();
    QueuedSqtProgram {
        commands,
        output: super::matrix::ProgramOutput::Sqt(output),
        hierarchy: AndaleHierarchy {
            has_trajectory: false,
            parents: vec![-1],
            bone_metadata: vec![0],
            parts: vec![],
            bone_names: vec![],
            packed_bone_names: vec![],
        }.into(),
    }
}

#[test]
fn fixed_extraction_copies_at_scheduling_without_a_work_job() {
    let program = program(false);
    assert_eq!(program.extraction_job().unwrap(), None);
    let output = program.output.sqt().clone();
    let work = SqtDispatchWork::new(vec![program]);
    assert!(!work.has_fetch());
    assert!(!work.has_blend());
    work.extract_fixed_outputs().unwrap();
    assert!(!output.is_unwritten().unwrap());
    assert!(output.read().is_err());
    work.complete_sync().unwrap();
    assert_eq!(output.read().unwrap().0, 1);
}

#[test]
fn blend_extraction_is_written_by_blend_without_a_set_data_job() {
    let program = program(true);
    assert_eq!(
        program.extraction_job().unwrap(),
        Some(AnimationJobCode::SqtBlend)
    );
    let output = program.output.sqt().clone();
    let work = SqtDispatchWork::new(vec![program]);
    work.extract_fixed_outputs().unwrap();
    assert!(output.is_unwritten().unwrap());
    work.run(AnimationJobCode::SqtBlend).unwrap();
    assert!(!output.is_unwritten().unwrap());
    assert!(output.read().is_err());
    work.complete_sync().unwrap();
    assert_eq!(output.read().unwrap().0, 1);
}
