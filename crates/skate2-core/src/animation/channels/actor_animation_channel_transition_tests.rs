use super::*;
use crate::{
    actor_animation_construction::{
        AnimationTransitionStore,
        channel_runtime::{ChannelBaseReference, ChannelBlendRuntime},
    },
    actor_animation_tree::AnimationTreePool,
};

fn fixture() -> (Channel, Vec<OwnedAnimationPool>) {
    let mut trees = (0..2)
        .map(|_| OwnedAnimationPool {
            runtime: AnimationTreePool::new(1024, 16),
            memory: vec![],
            elements: vec![],
        })
        .collect::<Vec<_>>();
    let database = crate::actor_face_playback::FaceAnimationManagers::new()
        .database
        .lock()
        .unwrap()
        .allocate()
        .unwrap();
    let pose = crate::actor_face_playback::AnimationPoseRef {
        database,
        record_index: 0,
    };
    let from = trees[0].runtime.push_pose(Some(pose.clone())).unwrap();
    let to = trees[1].runtime.push_pose(Some(pose)).unwrap();
    let store = AnimationTransitionStore::new();
    let mut blend = store.blends[0].clone();
    blend.length = 0.1;
    let channel = Channel {
        runtime: Some(ChannelBlendRuntime {
            base: ChannelBaseReference::Empty,
            animation: from,
            cycle: true,
            blend_in_first_frame: false,
            blend_out_last_frame: false,
            blend_out_early: false,
            blending_out: true,
            expired: false,
            use_attributes: false,
        }),
        channel_tree: Some(0),
        transition_tree: Some(1),
        node_evaluated: false,
        base_animation: AnimationPointerState::Null,
        channel_animation: AnimationPointerState::Constructed,
        blend_coefficient: Some(1.0),
        influence: Some(1.0),
        blend_in_time: Some(0.3),
        blend_in_length: Some(0.3),
        blend_out_time: Some(0.0),
        blend_out_length: Some(0.3),
        resurrect_time: Some(0.0),
        resurrect_length: Some(0.0),
        channel_flags: Some(0x18000000),
        transition: AnimationPointerState::Constructed,
        transition_kind: Some(ChannelTransitionKind::Blend),
        sequence: store.sequences[0],
        transition_blend: blend,
        transition_to: Some(to),
        transitioning: true,
        parent: None,
        base: None,
        priority: 0,
    };
    (channel, trees)
}

#[test]
fn own_channel_pools_advance_and_failed_to_preserves_clock() {
    let (mut channel, mut trees) = fixture();
    let mut properties = AnimationProperties::default();
    add_time(&mut channel, &mut trees, 0.05, &mut properties).unwrap();
    assert_eq!(channel.transition_blend.time, 0.05);
    assert_eq!(target(&channel).unwrap().0, 1);
    add_time(&mut channel, &mut trees, f32::NAN, &mut properties).unwrap();
    assert!(channel.transition_blend.time.is_nan());
    channel.transition_blend.time = 0.05;
    trees[1].runtime.clear();
    assert!(matches!(
        add_time(&mut channel, &mut trees, 0.01, &mut properties),
        Err(E::Tree(_))
    ));
    assert_eq!(channel.transition_blend.time, 0.05);
    assert!(matches!(
        add_time(&mut channel, &mut trees, -0.01, &mut properties),
        Err(E::Tree(AnimationTreeError::NegativeTime))
    ));
}

#[test]
fn resurrection_fsel_preserves_native_endpoints_and_unordered() {
    assert_eq!(
        clamp_unit(1.0 - 0.1 / 0.3) * 0.3,
        (1.0_f32 - 0.1 / 0.3) * 0.3
    );
    assert_eq!(clamp_unit(1.0 - 0.4 / 0.3), 0.0);
    assert_eq!(clamp_unit(f32::NAN), 1.0);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "two real channel pool children and command-failure publication fixture"
)]
fn inline_eval_visits_both_endpoints_and_preserves_command_failure_prefix() {
    use crate::actor_animation_tree::eval::{AnimationEvalCommands, AnimationEvalParams};
    use crate::actor_face_playback::{
        AnimationPoseRef, FaceAnimationManagers, FaceCommandPhase, FaceCommands,
        tree_database::clip_fetch::ClipFetchPlan,
    };
    use std::sync::{Arc, Mutex};
    struct Sink {
        calls: Vec<&'static str>,
        fail: bool,
    }
    impl AnimationEvalCommands for Sink {
        type Error = ();
        fn fetch(&mut self, _: ClipFetchPlan) -> Result<(), ()> {
            unreachable!()
        }
        fn fetch_pose(&mut self, _: &AnimationPoseRef) -> Result<(), ()> {
            self.calls.push("pose");
            Ok(())
        }
        fn n_blend(&mut self, _: &[f32]) -> Result<(), ()> {
            unreachable!()
        }
        fn blend(&mut self, _: f32) -> Result<(), ()> {
            self.calls.push("blend");
            if self.fail { Err(()) } else { Ok(()) }
        }
        fn channel_blend(&mut self, _: f32, _: bool) -> Result<(), ()> {
            unreachable!()
        }
        fn add_sqt(&mut self, _: bool) -> Result<(), ()> {
            unreachable!()
        }
        fn mirror(&mut self, _: u32) -> Result<(), ()> {
            unreachable!()
        }
    }
    let (mut channel, mut trees) = fixture();
    let database = FaceAnimationManagers::new()
        .database
        .lock()
        .unwrap()
        .allocate()
        .unwrap();
    let pose = AnimationPoseRef {
        database: Arc::clone(&database),
        record_index: 0,
    };
    channel.runtime.as_mut().unwrap().animation =
        trees[0].runtime.push_pose(Some(pose.clone())).unwrap();
    channel.transition_to = Some(trees[1].runtime.push_pose(Some(pose)).unwrap());
    let params = AnimationEvalParams {
        commands: Arc::new(Mutex::new(FaceCommands {
            database,
fixed_frames: vec![],
            fixed_matrices: Vec::new(),
            max_frames: 1,
            phase: FaceCommandPhase::Ready,
            scratch: None,
            selected_frames: vec![],
            setup: None,
            buffered: None,
        })),
        update_old_time: true,
        blend_frames: 2,
        blend_cull_threshold: 0.0,
        blend_space_cvars: None,
    };
    let mut sink = Sink {
        calls: vec![],
        fail: false,
    };
    for time in [0.0, 0.1] {
        channel.transition_blend.time = time;
        assert!(eval(&mut channel, &mut trees, &params, true, &mut sink).unwrap());
        assert_eq!(sink.calls, ["pose", "pose", "blend"]);
        sink.calls.clear();
    }
    channel.transition_blend.evaluated = false;
    sink.fail = true;
    assert!(eval(&mut channel, &mut trees, &params, true, &mut sink).is_err());
    assert_eq!(sink.calls, ["pose", "pose", "blend"]);
    assert!(!channel.transition_blend.evaluated);
    let (tree, to) = target(&channel).unwrap();
    assert!(trees[tree].runtime.node(&to).unwrap().evaluated);
    sink.calls.clear();
    sink.fail = false;
    channel.transition_blend.length = 0.0;
    assert!(eval(&mut channel, &mut trees, &params, true, &mut sink).is_err());
    assert_eq!(sink.calls, ["pose", "pose"]);
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Reuses the disposable ORIGINAL-asset Actor, after its channels are removed.
    #[expect(
        clippy::too_many_lines,
        reason = "one original Actor checks allocation, restart, failure, resurrection and reclamation"
    )]
    pub(crate) fn check_fakie_channel_transition_corpus(&mut self) {
        let name = FastString36::encode("FakieTransitionRegression").unwrap();
        let missing = FastString36::encode("__MISSING_FAKIE_TRANSITION__").unwrap();
        let counts = |s: &Self| {
            let c = s
                .initialization
                .as_ref()
                .unwrap()
                .channels
                .as_ref()
                .unwrap();
            (c.free_channels.len(), c.free_trees.len())
        };
        let initial = counts(self);
        assert_eq!(
            self.try_start_fakie_channel(name).unwrap(),
            NewGestureChannelStatus::Created
        );
        let index = *self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap()
            .dictionary
            .get(&name)
            .unwrap();
        let old_tree = self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap()
            .channels[index]
            .channel_tree
            .unwrap();
        assert_eq!(
            self.try_start_fakie_channel(name).unwrap(),
            NewGestureChannelStatus::Created
        );
        assert_eq!(counts(self), (initial.0 - 1, initial.1 - 2));
        let c = &self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap()
            .channels[index];
        assert!(c.transitioning);
        assert_eq!(c.transition_blend.time, 0.0);
        assert_eq!(c.channel_tree, Some(old_tree));
        let first_to = c.transition_tree.unwrap();
        assert_ne!(first_to, old_tree);
        // Equality must not complete before AddTime. Next representable time does.
        self.initialization
            .as_mut()
            .unwrap()
            .channels
            .as_mut()
            .unwrap()
            .channels[index]
            .transition_blend
            .time = 0.1;
        self.try_update_channel_transitions().unwrap();
        assert!(
            self.initialization
                .as_ref()
                .unwrap()
                .channels
                .as_ref()
                .unwrap()
                .channels[index]
                .transitioning
        );
        self.initialization
            .as_mut()
            .unwrap()
            .channels
            .as_mut()
            .unwrap()
            .channels[index]
            .transition_blend
            .time = f32::from_bits(0.1_f32.to_bits() + 1);
        self.try_update_channel_transitions().unwrap();
        let c = &self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap()
            .channels[index];
        assert!(!c.transitioning);
        assert_eq!(c.channel_tree, Some(first_to));
        assert_eq!(counts(self), (initial.0 - 1, initial.1 - 1));
        // Resurrection uses previous out-time/out-length; blend-in stays untouched.
        let c = &mut self
            .initialization
            .as_mut()
            .unwrap()
            .channels
            .as_mut()
            .unwrap()
            .channels[index];
        c.blend_out_time = Some(0.15);
        c.blend_out_length = Some(0.3);
        c.blend_in_time = Some(0.27);
        self.try_start_fakie_channel(name).unwrap();
        let c = &self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap()
            .channels[index];
        assert_eq!(c.resurrect_time, Some(0.15));
        assert_eq!(c.resurrect_length, Some(0.3));
        assert_eq!(c.blend_out_time, Some(0.0));
        assert_eq!(c.blend_in_time, Some(0.27));
        let prior_to = c.transition_tree;
        // Restart force-completes old transition, then allocates exactly one pool.
        self.try_start_fakie_channel(name).unwrap();
        let c = &self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap()
            .channels[index];
        assert_eq!(c.channel_tree, prior_to);
        assert!(c.transitioning);
        assert_eq!(counts(self), (initial.0 - 1, initial.1 - 2));
        // Missing target after restart retains completed predecessor, no pool pop.
        assert_eq!(
            self.try_transition_fakie_channel(name, missing).unwrap(),
            NewGestureChannelStatus::AnimationMissing
        );
        assert!(
            !self
                .initialization
                .as_ref()
                .unwrap()
                .channels
                .as_ref()
                .unwrap()
                .channels[index]
                .transitioning
        );
        assert_eq!(counts(self), (initial.0 - 1, initial.1 - 1));
        self.try_start_fakie_channel(name).unwrap();
        self.try_remove_channel(&name).unwrap();
        assert_eq!(counts(self), initial);
        // Reuse the stale inline slot after removal, then release without transition.
        self.try_start_fakie_channel(name).unwrap();
        self.try_remove_channel(&name).unwrap();
        assert_eq!(counts(self), initial);
    }
}
