use super::super::super::bind_pose::BindPoseCommand as Command;
use super::super::{BodyPoseWorkState, PoseOutputAllocation, TreeCommands};
use super::*;
use crate::actor_animation_tree::eval::AnimationEvalCommands;
use skate2_formats::andale::{AndaleDataHeader, AndaleDatabase, AndaleHierarchy, AndaleSqt, PackedFastString};
fn pose(x: f32) -> Vec<AndaleSqt> {
    vec![AndaleSqt {
        scale: [1.0; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [x, 0.0, 0.0, 1.0],
    }]
}
/// The test hierarchy as its owner's parsed database.
fn database_with(hierarchy: AndaleHierarchy) -> Arc<AndaleDatabase> {
    Arc::new(AndaleDatabase {
        file_header: AndaleDataHeader {
            size: 0,
            data_type: 0,
            codec: 0,
            guid: 0,
            name: PackedFastString::encode("").unwrap(),
        },
        records: Vec::new(),
        hierarchy,
        clips: Vec::new(),
        poses: Vec::new(),
        physics_parameters: Vec::new(),
        controllers: Vec::new(),
    })
}

fn body() -> Arc<BodyPoseWork> {
    let (mut tree, fixed) = TreeCommands::fixed_test_collector();
    // Public command methods establish actual dynamic output0. The test then
    // supplies the already-emitted dynamic operation under examination.
    tree.set_fixed_sqt(&fixed, 0).unwrap();
    tree.set_fixed_sqt(&fixed, 1).unwrap();
    tree.blend(0.0).unwrap();
    tree.operations[0] = Command::Blend {
        first: 0,
        second: 1,
        output: 0,
        coefficient: 0,
    };
    let output = || {
        Arc::new(Mutex::new(PoseOutputAllocation {
            values: vec![None],
            synchronized: false,
        }))
    };
    Arc::new(BodyPoseWork {
        pose: None,
        tree: Some(tree),
        database: database_with(AndaleHierarchy {
            has_trajectory: false,
            parents: vec![-1],
            bone_metadata: vec![],
            parts: vec![],
            bone_names: vec!["root".into()],
            packed_bone_names: vec![],
        }),
        local: output(),
        global: output(),
        state: Mutex::new(BodyPoseWorkState {
            sqt: None,
            tree_sqts: vec![None, Some(pose(7.0))],
            matrix: None,
            failure: None,
        }),
    })
}
#[test]
fn actual_segment_work_copies_then_blends_then_extracts_final_output() {
    let body = body();
    let first = SegmentWork {
        body: Arc::clone(&body),
        segment: Segment {
            external_sqts: 0..0,
            fetches: 0..0,
            operations: 0..0,
            copies: vec![(0, 1)],
        },
        final_segment: false,
    };
    first.run(AnimationJobCode::ReuseBuffer).unwrap();
    assert!(body.state.lock().unwrap().sqt.is_none());
    body.state.lock().unwrap().tree_sqts[1] = Some(pose(100.0));
    let last = SegmentWork {
        body: Arc::clone(&body),
        segment: Segment {
            external_sqts: 0..0,
            fetches: 0..0,
            operations: 0..1,
            copies: vec![],
        },
        final_segment: true,
    };
    last.run(AnimationJobCode::SqtBlend).unwrap();
    let state = body.state.lock().unwrap();
    assert_eq!(
        state.sqt.as_ref().unwrap()[0].translation[0].to_bits(),
        7.0_f32.to_bits()
    );
    assert!(state.tree_sqts[0].is_none());
    assert!(state.failure.is_none());
    assert!(body.local.lock().unwrap().values[0].is_none());
}
#[test]
fn actual_segment_failure_retains_typed_missing_source_without_publication() {
    let body = body();
    body.state.lock().unwrap().tree_sqts[1] = None;
    let segment = SegmentWork {
        body: Arc::clone(&body),
        segment: Segment {
            external_sqts: 0..0,
            fetches: 0..0,
            operations: 0..0,
            copies: vec![(0, 1)],
        },
        final_segment: false,
    };
    assert!(segment.run(AnimationJobCode::ReuseBuffer).is_err());
    let state = body.state.lock().unwrap();
    assert_eq!(
        state.failure.as_deref(),
        Some(&BindPoseError::MissingBindPose)
    );
    assert!(state.sqt.is_none());
    assert!(state.tree_sqts[0].is_none());
}

#[test]
fn body_set_data_reads_retained_pose_before_fetch_and_matrix_jobs() {
    use crate::actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer;
    let mut work=body();
    let work=Arc::get_mut(&mut work).unwrap();
    let (mut tree,_)=TreeCommands::fixed_test_collector();
    {let mut owner=tree.owner.lock().unwrap();owner.in_command_set=1;owner.in_commands=1;owner.max_set_replace_sqts=1;}
    let source=OwnedAnimationSqtBuffer::allocate_unknown(1);
    source.extract(&pose(3.)).unwrap();source.complete_sync().unwrap();
    tree.set_external_sqt(&source).unwrap();
    work.tree=Some(tree);
    work.state.get_mut().unwrap().tree_sqts=vec![None];
    source.extract(&pose(7.)).unwrap();source.complete_sync().unwrap();
    work.run(AnimationJobCode::SetData).unwrap();
    source.extract(&pose(100.)).unwrap();
    work.run(AnimationJobCode::FetchVbr).unwrap();
    assert_eq!(work.state.lock().unwrap().sqt.as_ref().unwrap()[0].translation[0],7.);
    assert!(work.local.lock().unwrap().values[0].is_none());
}
