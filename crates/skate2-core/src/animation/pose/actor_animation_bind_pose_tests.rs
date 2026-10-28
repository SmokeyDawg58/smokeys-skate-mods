use super::{BindPoseCommand, BufferedBodyCommands, OwnedAnimationPoseBuffer};
use crate::{
    actor_face_playback::FaceAnimationManagers,
    actor_graph_construction::ActorGraphIntents,
    animation_job_owners::AnimationJobManager,
    simulation_animation_startup::{AnimationStartupError, AnimationStartupSource},
};
use std::sync::Arc;

#[test]
fn pose_job_writes_the_retained_destination_but_read_access_waits_for_sync() {
    use super::{
        AnimationJobCode, AnimationJobWork, BindPoseWork, BindPoseWorkState, PoseOutputAllocation,
    };
    use skate2_formats::andale::{AndaleHierarchy, AndaleSqt};
    use std::sync::Mutex;
    let managers = FaceAnimationManagers::new();
    let database = managers.database.lock().unwrap().allocate().unwrap();
    let storage = Arc::new(Mutex::new(PoseOutputAllocation {
        values: vec![None; 2],
        synchronized: false,
    }));
    let output = OwnedAnimationPoseBuffer {
        hierarchy: Some(Arc::clone(&database)),
        storage: Some(Arc::clone(&storage)),
        local_pose: None,
    };
    let identity = AndaleSqt {
        scale: [1.0; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0, 0.0, 0.0, 1.0],
    };
    let work = BindPoseWork {
        pose: super::AnimationPoseRef {
            database,
            record_index: 0,
        },
        hierarchy: AndaleHierarchy {
            has_trajectory: true,
            parents: vec![-1, 0],
            bone_metadata: Vec::new(),
            parts: Vec::new(),
            bone_names: vec!["trajectory".into(), "root".into()],
            packed_bone_names: Vec::new(),
        },
        output: Arc::clone(&storage),
        state: Mutex::new(BindPoseWorkState {
            sqt: Some(vec![identity; 2]),
            internal_matrix: None,
            failure: None,
        }),
    };
    assert!(output.matrices().unwrap().is_none());
    work.run(AnimationJobCode::Pose).unwrap();
    assert!(storage.lock().unwrap().values.iter().all(Option::is_some));
    assert!(output.matrices().unwrap().is_none());
    storage.lock().unwrap().synchronized = true;
    assert_eq!(output.matrices().unwrap().unwrap().len(), 2);
    assert_eq!(output.allocation_len().unwrap(), 2);
}

struct Source {
    jobs: Arc<AnimationJobManager>,
    captures: usize,
    fail: bool,
}
impl super::super::ActorAnimationInitializeSource for Source {
    type Error = &'static str;
    fn read_use_jobs_for_skater_anim_update(&mut self) -> Result<bool, Self::Error> {
        panic!("GetBindpose does not sample jobs cvar")
    }
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        self.captures += 1;
        if self.fail {
            Err("missing manager")
        } else {
            Ok(Arc::clone(&self.jobs))
        }
    }
}
impl AnimationStartupSource for Source {
    type Error = &'static str;
    fn current_thread_id(&mut self) -> Result<u64, Self::Error> {
        panic!("construction only")
    }
    fn read_use_vbr(&mut self) -> Result<bool, Self::Error> {
        panic!("construction only")
    }
    fn read_animation(&mut self, _: &str) -> Result<Box<[u8]>, Self::Error> {
        panic!("construction only")
    }
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        super::super::ActorAnimationInitializeSource::require_job_manager(self)
    }
}

#[test]
fn missing_manager_retains_fetch_entry_allocation_before_acs_and_registry_effects() {
    let managers = FaceAnimationManagers::new();
    let hierarchy = managers.database.lock().unwrap().allocate().unwrap();
    let scheduler = Arc::clone(&managers.command.lock().unwrap().scheduler);
    let mut commands =
        BufferedBodyCommands::new(hierarchy, Arc::clone(&managers.database), scheduler);
    let mut source = Source {
        jobs: Arc::new(AnimationJobManager::new()),
        captures: 0,
        fail: true,
    };
    let setup = crate::actor_face_playback::AnimationCommandSetup {
        capacities: [200, 100, 0, 0, 2],
        averages: [30.0, 20.0, 0.0, 0.0, 2.0],
        max_nblend: 4,
        fixed_sqt: 0,
        fixed_matrix: 0,
        mode: 1,
        fetch_mode: 1,
        category: 0,
        characters: 1,
        truncation_offset: 0.5,
        fetch_time_error: 0.0,
        trajectory: true,
        codec: 1,
    };
    assert_eq!(
        commands.try_construct(&setup, &mut source),
        Err(AnimationStartupError::Source("missing manager"))
    );
    assert_eq!(commands.entries.len(), 1);
    assert!(commands.entries[0].entry.is_none());
    assert!(commands.entries[0].manager.is_none());
    assert!(commands.null_job.is_none());
    assert!(commands.fetch_pool.is_none());
    assert!(managers.command.lock().unwrap().entries.is_empty());
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_offboard_guid_fetch_to_onboard_buffered_pose_and_sync() {
    let startup = crate::simulation_animation_startup::corpus::load_original_startup();
    let playback = startup.body[0].lock().unwrap();
    let pose = playback
        .get_pose(ActorGraphIntents::native_name(b"rig_Tpose"))
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&pose.database, &startup.databases[0]));
    let buffered = playback.commands.lock().unwrap().buffered.clone().unwrap();
    drop(playback);
    let jobs = startup.retained_job_manager().unwrap().unwrap();
    let mut commands = buffered.lock().unwrap();
    assert_eq!(commands.entries.len(), 4);
    assert_eq!(commands.fetch_pool.as_ref().unwrap().jobs.len(), 13);
    assert_eq!(commands.blend_pool.as_ref().unwrap().jobs.len(), 18);
    assert_eq!(commands.set_pool.as_ref().unwrap().jobs.len(), 12);
    assert_eq!(commands.extract_memory.len(), 53_120);
    let mut source = Source {
        jobs,
        captures: 0,
        fail: false,
    };
    let mut output = OwnedAnimationPoseBuffer::new();
    let mut evaluated = false;
    commands
        .try_evaluate(&pose, &mut output, &mut evaluated, &mut source)
        .unwrap();
    assert!(evaluated);
    assert!(Arc::ptr_eq(
        output.hierarchy.as_ref().unwrap(),
        &startup.databases[1]
    ));
    assert_eq!(output.allocation_len().unwrap(), 36);
    assert_eq!(output.matrices().unwrap().unwrap().len(), 36);
    assert_eq!(output.local_pose.as_ref().unwrap().len(), 36);
    assert!(
        output
            .matrices()
            .unwrap()
            .unwrap()
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
    assert_eq!(
        commands.last_submitted,
        [
            BindPoseCommand::FetchPose {
                guid: pose.guid().unwrap(),
                sqt: 0
            },
            BindPoseCommand::Pose { sqt: 0, matrix: 0 },
            BindPoseCommand::ExtractMatrix { matrix: 0 }
        ]
    );
    assert!(commands.commands.is_empty());
    assert_eq!(commands.initiated_commands, 0);
    assert_eq!(commands.synchronized_sets, 1);
    assert_eq!(source.captures, 3); // first lazy pose entry, ScheduleTree, Sync.
    let matrices = output.matrices().unwrap();
    commands
        .try_evaluate(&pose, &mut output, &mut evaluated, &mut source)
        .unwrap();
    assert_eq!(output.matrices().unwrap(), matrices);
    assert_eq!(source.captures, 5); // lazy entry/pool retained across sets.
}
