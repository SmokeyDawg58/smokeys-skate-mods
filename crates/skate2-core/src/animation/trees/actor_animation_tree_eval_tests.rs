use super::*;
use crate::actor_face_playback::{FaceAnimationManagers, FaceCommandPhase};

struct Sink {
    calls: Vec<&'static str>,
    fail: bool,
}
impl AnimationEvalCommands for Sink {
    type Error = &'static str;
    fn fetch(&mut self, _: ClipFetchPlan) -> Result<(), Self::Error> {
        unreachable!()
    }
    fn fetch_pose(&mut self, _: &AnimationPoseRef) -> Result<(), Self::Error> {
        self.calls.push("pose");
        if self.fail { Err("acceptance") } else { Ok(()) }
    }
    fn n_blend(&mut self, _: &[f32]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn blend(&mut self, _: f32) -> Result<(), Self::Error> {
        self.calls.push("blend");
        Ok(())
    }
    fn channel_blend(&mut self, _: f32, _: bool) -> Result<(), Self::Error> {
        self.calls.push("channel");
        Ok(())
    }
    fn add_sqt(&mut self, motion_a: bool) -> Result<(), Self::Error> {
        self.calls.push(if motion_a { "add-a" } else { "add-b" });
        Ok(())
    }
    fn mirror(&mut self, _: u32) -> Result<(), Self::Error> {
        self.calls.push("mirror");
        Ok(())
    }
}
fn fixture() -> (
    AnimationTreePool,
    AnimationEvalParams,
    AnimationPoseRef,
    Sink,
) {
    let database = FaceAnimationManagers::new()
        .database
        .lock()
        .unwrap()
        .allocate()
        .unwrap();
    let commands = Arc::new(Mutex::new(FaceCommands {
        database: Arc::clone(&database),
fixed_frames: vec![],
            fixed_matrices: Vec::new(),
        max_frames: 1,
        phase: FaceCommandPhase::Ready,
        scratch: None,
        selected_frames: vec![],
        setup: None,
        buffered: None,
    }));
    (
        AnimationTreePool::new(1024, 16),
        AnimationEvalParams {
            blend_space_cvars: None,
            commands,
            update_old_time: true,
            blend_frames: 2,
            blend_cull_threshold: 0.0,
        },
        AnimationPoseRef {
            database,
            record_index: 0,
        },
        Sink {
            calls: vec![],
            fail: false,
        },
    )
}

#[test]
fn phase_endpoint_smoothing_keeps_source_fused_curve_and_interior_unsmoothed() {
    for (blend, expected) in [
        (0.0, 0.0_f32),
        (0.25, 0.1875),
        (0.5, 0.5),
        (0.75, 0.8125),
        (1.0, 1.0),
    ] {
        assert_eq!(
            phase_coefficient(blend, 0.0, true, true).to_bits(),
            expected.to_bits()
        );
        assert_eq!(
            phase_coefficient(blend, 0.0, false, false).to_bits(),
            blend.to_bits()
        );
    }
}

#[test]
fn pose_acceptance_failure_does_not_commit_evaluation_bit() {
    let (mut pool, params, pose, mut sink) = fixture();
    let node = pool.push_pose(Some(pose)).unwrap();
    sink.fail = true;
    assert!(matches!(
        pool.eval(&node, &params, true, &mut sink),
        Err(AnimationEvalError::Command("acceptance"))
    ));
    assert!(!pool.node(&node).unwrap().evaluated);
    assert_eq!(sink.calls, ["pose"]);
}

#[test]
fn culled_clip_commits_old_time_and_clears_loops_without_fetch() {
    let (mut pool, params, pose, mut sink) = fixture();
    let mut clip = super::super::ClipNode::new();
    clip.header = Some(super::super::AnimationClipRef {
        database: pose.database,
        record_index: 0,
    });
    clip.time = 3.0;
    clip.old_time = 1.0;
    clip.loops = 2;
    let node = pool.push(96, AnimationNodeKind::Clip(clip)).unwrap();
    assert!(!pool.eval(&node, &params, false, &mut sink).unwrap());
    let AnimationNodeKind::Clip(clip) = &pool.node(&node).unwrap().kind else {
        unreachable!()
    };
    assert_eq!(clip.old_time.to_bits(), 3.0_f32.to_bits());
    assert_eq!(clip.loops, 0);
    assert!(sink.calls.is_empty());
}

#[test]
fn add_sqt_evaluates_only_motion_when_culled_and_keeps_motion_b_order() {
    let (mut pool, params, pose, mut sink) = fixture();
    let a = pool.push_pose(Some(pose.clone())).unwrap();
    let b = pool.push_pose(Some(pose)).unwrap();
    pool.node_mut(&a).unwrap().evaluated = true;
    let add = pool.push_add_sqt(a.clone(), b.clone(), b.clone()).unwrap();
    assert!(!pool.eval(&add, &params, false, &mut sink).unwrap());
    assert!(
        pool.node(&a).unwrap().evaluated,
        "nonmotion A was not visited"
    );
    assert!(!pool.node(&b).unwrap().evaluated);
    assert!(pool.eval(&add, &params, true, &mut sink).unwrap());
    assert_eq!(sink.calls, ["pose", "pose", "add-b"]);
}
