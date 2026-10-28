use super::super::{AnimationJobCode, AnimationJobWork, SqtDispatchWork};
use super::*;

fn hierarchy() -> AndaleHierarchy {
    AndaleHierarchy {
        has_trajectory: true,
        parents: vec![-1, 0, 1],
        bone_metadata: vec![0; 3],
        parts: vec![],
        bone_names: vec!["trajectory".into(), "hips".into(), "child".into()],
        packed_bone_names: vec![],
    }
}
fn sqt(x: f32) -> AndaleSqt {
    AndaleSqt {
        scale: [1.0; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [x, 0.0, 0.0, 1.0],
    }
}
fn matrix(x: f32) -> AnimationMatrix {
    let mut result = [0.0; 16];
    for i in [0, 5, 10, 15] {
        result[i] = 1.0;
    }
    result[12] = x;
    result
}
fn output(count: usize) -> Arc<Mutex<PoseOutputAllocation>> {
    Arc::new(Mutex::new(PoseOutputAllocation {
        values: vec![None; count],
        synchronized: false,
    }))
}

#[test]
fn native_affine_graph_keeps_fused_translation_and_ignores_bind_w() {
    let output = output(3);
    let mut local = sqt(0.0);
    local.scale[0] = 1.0 + f32::EPSILON;
    local.translation[0] = -1.0;
    let mut inverse = matrix(1.0 - f32::EPSILON);
    inverse[15] = f32::NAN;
    let program = MatrixProgram {
        fixed: false,
        sqt_extract: None,
        source: Arc::from([inverse; 3]),
        output: Arc::clone(&output),
    };
    program
        .execute(&hierarchy(), &[local; 3], &[inverse; 3])
        .unwrap();
    let first = output.lock().unwrap().values[0].unwrap();
    assert_eq!(first[12].to_bits(), (-f32::EPSILON.powi(2)).to_bits());
    assert_eq!(first[15].to_bits(), 1.0f32.to_bits());
    assert!(!output.lock().unwrap().synchronized);
}

#[test]
fn retained_sqt_is_sampled_by_set_then_matrix_pose_then_sync() {
    let (mut commands, _) = TreeCommands::fixed_test_collector();
    {
        let mut owner = commands.owner.lock().unwrap();
        owner.in_command_set = 1;
        owner.in_commands = 1;
        owner.max_set_replace_sqts = 3;
    }
    let source = OwnedAnimationSqtBuffer::allocate_for_hierarchy(Arc::new(hierarchy()));
    source.extract(&[sqt(1.0); 3]).unwrap();
    source.complete_sync().unwrap();
    commands.set_external_sqt(&source).unwrap();
    let destination = output(3);
    let work = SqtDispatchWork::new(vec![QueuedSqtProgram {
        commands,
        hierarchy: hierarchy().into(),
        output: ProgramOutput::Matrix(MatrixProgram {
        fixed: false,
        sqt_extract: None,
            source: Arc::from([matrix(-2.0); 3]),
            output: Arc::clone(&destination),
        }),
    }]);
    assert!(work.has_matrix_pose());
    assert!(work.has_set());
    // Mutations after emission are consumed at actual SetData execution.
    source.extract(&[sqt(3.0), sqt(5.0), sqt(7.0)]).unwrap();
    source.complete_sync().unwrap();
    work.run(AnimationJobCode::SetData).unwrap();
    assert!(
        destination
            .lock()
            .unwrap()
            .values
            .iter()
            .all(Option::is_none)
    );
    // SetData already copied the native input; later changes are not reread.
    source.extract(&[sqt(100.0); 3]).unwrap();
    work.run(AnimationJobCode::PoseMatMult).unwrap();
    let values = destination.lock().unwrap().values.clone();
    assert_eq!(
        values
            .into_iter()
            .map(|v| v.unwrap()[12])
            .collect::<Vec<_>>(),
        [1.0, 3.0, 10.0]
    );
    assert!(!destination.lock().unwrap().synchronized);
    work.complete_sync().unwrap();
    assert!(destination.lock().unwrap().synchronized);
}

#[test]
fn invalid_late_hierarchy_keeps_external_extraction_unwritten() {
    let output = output(3);
    let mut hierarchy = hierarchy();
    hierarchy.parents[2] = 7;
    let program = MatrixProgram {
        fixed: false,
        sqt_extract: None,
        source: Arc::from([matrix(0.0); 3]),
        output: Arc::clone(&output),
    };
    assert!(
        program
            .execute(&hierarchy, &[sqt(1.0); 3], &[matrix(0.0); 3])
            .is_err()
    );
    let retained = output.lock().unwrap();
    assert!(retained.values.iter().all(Option::is_none));
    assert!(!retained.synchronized);
}
