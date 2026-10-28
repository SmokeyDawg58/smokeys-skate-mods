use super::{
    AnimationJobCode, AnimationJobWork, BodyPoseWork, BodyPoseWorkState, PoseOutputAllocation,
};
use crate::actor_face_playback::{AnimationPoseRef, FaceAnimationManagers};
use skate2_formats::andale::{AndaleDataHeader, AndaleDatabase, AndaleHierarchy, AndaleSqt, PackedFastString};
use std::sync::{Arc, Mutex};

struct Source {
    jobs: Arc<crate::animation_job_owners::AnimationJobManager>,
}
impl super::ActorAnimationInitializeSource for Source {
    type Error = std::convert::Infallible;
    fn read_use_jobs_for_skater_anim_update(&mut self) -> Result<bool, Self::Error> {
        panic!("body command dispatch does not read the blend-space jobs cvar")
    }
    fn require_job_manager(
        &mut self,
    ) -> Result<Arc<crate::animation_job_owners::AnimationJobManager>, Self::Error> {
        Ok(Arc::clone(&self.jobs))
    }
}
impl super::BodyAnimationDispatchSource for Source {
    fn body_command_manager(
        &mut self,
    ) -> Result<Arc<Mutex<super::FaceCommandManager>>, Self::Error> {
        panic!("command-engine fixture begins after actual named ACS lookup")
    }
    fn blend_cull_threshold(&mut self, _: bool) -> Result<f32, Self::Error> {
        panic!("command-engine fixture begins after the outer cvar read")
    }
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
#[expect(
    clippy::too_many_lines,
    reason = "isolated original startup proves null and nonnull source command paths across separate Sync boundaries"
)]
fn original_body_command_engine_extracts_both_retained_outputs_and_syncs_same_backend() {
    use super::super::bind_pose::{BindPoseCommand as Command, OwnedAnimationPoseBuffer};
    use crate::actor_animation_tree::{
        AnimationTreePool, eval::AnimationEvalParams, time::AnimationProperties,
    };
    use crate::{
        actor_graph_construction::ActorGraphIntents, simulation_animation_startup::corpus,
    };
    let startup = corpus::load_original_startup();
    let jobs = startup.retained_job_manager().unwrap().unwrap();
    let playback = startup.body[0].lock().unwrap();
    let pose = playback
        .get_pose(ActorGraphIntents::native_name(b"rig_Tpose"))
        .unwrap()
        .unwrap();
    let local =
        OwnedAnimationPoseBuffer::allocate_unknown(Arc::clone(&playback.database), 36).unwrap();
    let global =
        OwnedAnimationPoseBuffer::allocate_unknown(Arc::clone(&playback.database), 36).unwrap();
    let buffered = playback.commands.lock().unwrap().buffered.clone().unwrap();
    drop(playback);
    let mut commands = buffered.lock().unwrap();
    let mut source = Source { jobs };
    let mut evaluated = false;
    for iteration in 1..=2 {
        commands.in_command_set += 1;
        commands.in_commands += 1;
        commands
            .queue_body_pose(
                &pose,
                None,
                local.shared_storage().unwrap(),
                global.shared_storage().unwrap(),
                &mut evaluated,
                &mut source,
            )
            .unwrap();
        assert!(evaluated);
        assert!(local.matrices().unwrap().is_none());
        assert!(global.matrices().unwrap().is_none());
        assert_eq!(
            commands.last_submitted,
            [
                Command::FetchPose {
                    guid: pose.guid().unwrap(),
                    sqt: 0
                },
                Command::SqtToLocal { sqt: 0, matrix: 0 },
                Command::ExtractMatrix { matrix: 0 },
                Command::LocalToGlobal { matrix: 0 },
                Command::ExtractMatrix { matrix: 0 },
            ]
        );
        commands.sync_body_pose(&mut source).unwrap();
        assert_eq!(commands.synchronized_sets, iteration);
        assert!(commands.body_pending.is_none());
        assert_eq!(local.matrices().unwrap().unwrap().len(), 36);
        assert_eq!(global.matrices().unwrap().unwrap().len(), 36);
        assert_eq!(commands.local_pool.as_ref().unwrap().jobs.len(), 14);
        assert_eq!(commands.global_pool.as_ref().unwrap().jobs.len(), 14);
        assert_eq!(commands.local_pool.as_ref().unwrap().current_job, 0);
        assert_eq!(commands.global_pool.as_ref().unwrap().current_job, 0);
    }
    drop(commands);
    // Isolated original startup owns this tree; never modify the shared Actor
    // constructor fixture merely to make it show an arbitrary gesture.
    let mut pool = AnimationTreePool::new(2400, 64);
    let root = startup.body[0]
        .lock()
        .unwrap()
        .try_construct_animation_tree(
            &mut pool,
            &ActorGraphIntents::native_name(b"B_GSTR_SKY_BOTH"),
            &[],
        )
        .unwrap()
        .expect("original retained body tree");
    let mut adjustment_flags = 0;
    let mut known_adjustment_flags = u32::MAX;
    let root = crate::actor_animation_construction::channel_runtime::add_bind_pose(
        &mut pool,
        root,
        Some(&pose),
        None,
        None,
        None,
        &mut adjustment_flags,
        &mut known_adjustment_flags,
    )
    .unwrap();
    pool.set_speed(&root, 1.0).unwrap();
    pool.set_time(&root, 0.1).unwrap();
    let owner = Arc::clone(&startup.body[0].lock().unwrap().commands);
    let params = AnimationEvalParams {
        blend_space_cvars: None,
        commands: owner,
        update_old_time: true,
        blend_frames: 2,
        blend_cull_threshold: 0.01,
    };
    let mut previous = None;
    for _ in 0..2 {
        pool.add_time(&root, 0.1, &mut AnimationProperties::default())
            .unwrap();
        {
            let mut commands = buffered.lock().unwrap();
            commands.in_command_set += 1;
            commands.in_commands += 1;
        }
        let mut tree = super::TreeCommands::new(Arc::clone(&buffered));
        assert!(pool.eval(&root, &params, true, &mut tree).unwrap());
        let mut commands = buffered.lock().unwrap();
        commands
            .queue_body_pose(
                &pose,
                Some(tree),
                local.shared_storage().unwrap(),
                global.shared_storage().unwrap(),
                &mut false,
                &mut source,
            )
            .unwrap();
        assert!(
            commands
                .last_submitted
                .iter()
                .any(|command| matches!(command, Command::FetchClip { .. }))
        );
        assert!(
            commands
                .last_submitted
                .iter()
                .any(|command| matches!(command, Command::AddSqt { .. }))
        );
        commands.sync_body_pose(&mut source).unwrap();
        let current = local
            .matrices()
            .unwrap()
            .unwrap()
            .iter()
            .map(|row| row.map(f32::to_bits))
            .collect::<Vec<_>>();
        if let Some(previous) = previous {
            assert_ne!(
                previous, current,
                "retained original clip advances the actual output"
            );
        }
        previous = Some(current);
    }
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

#[test]
fn dynamic_matrix_storage_is_reused_and_local_extract_survives_global_conversion() {
    let managers = FaceAnimationManagers::new();
    let database = managers.database.lock().unwrap().allocate().unwrap();
    let local = Arc::new(Mutex::new(PoseOutputAllocation {
        values: vec![None; 3],
        synchronized: false,
    }));
    let global = Arc::new(Mutex::new(PoseOutputAllocation {
        values: vec![None; 3],
        synchronized: false,
    }));
    let work = BodyPoseWork {
        pose: Some(AnimationPoseRef {
            database,
            record_index: 0,
        }),
        tree: None,
        database: database_with(AndaleHierarchy {
            has_trajectory: true,
            parents: vec![-1, 0, 1],
            bone_metadata: Vec::new(),
            parts: Vec::new(),
            bone_names: vec!["trajectory".into(), "root".into(), "child".into()],
            packed_bone_names: Vec::new(),
        }),
        local: Arc::clone(&local),
        global: Arc::clone(&global),
        state: Mutex::new(BodyPoseWorkState {
            tree_sqts: Vec::new(),
            sqt: Some(
                [3.0, 5.0, 7.0]
                    .map(|x| AndaleSqt {
                        scale: [1.0; 4],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        translation: [x, 0.0, 0.0, 1.0],
                    })
                    .to_vec(),
            ),
            matrix: None,
            failure: None,
        }),
    };
    work.run(AnimationJobCode::SqtToLocal).unwrap();
    let internal_address = work.state.lock().unwrap().matrix.as_ref().unwrap().as_ptr();
    assert!(global.lock().unwrap().values.iter().all(Option::is_none));
    let local_before = local.lock().unwrap().values.clone();
    work.run(AnimationJobCode::LocalToGlobal).unwrap();
    assert_eq!(
        work.state.lock().unwrap().matrix.as_ref().unwrap().as_ptr(),
        internal_address
    );
    assert_eq!(
        local
            .lock()
            .unwrap()
            .values
            .as_slice()
            .iter()
            .map(|v| v.unwrap().map(f32::to_bits))
            .collect::<Vec<_>>(),
        local_before
            .iter()
            .map(|v| v.unwrap().map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    let output = global.lock().unwrap();
    assert_eq!(output.values[1].unwrap()[12].to_bits(), 5.0_f32.to_bits());
    assert_eq!(output.values[2].unwrap()[12].to_bits(), 12.0_f32.to_bits());
    assert_eq!(
        local.lock().unwrap().values[2].unwrap()[12].to_bits(),
        7.0_f32.to_bits()
    );
    assert!(!output.synchronized);
    assert!(!local.lock().unwrap().synchronized);
}
