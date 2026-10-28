use super::super::{
    ActorAnimationInputs, ActorGraphAuthority, ActorSkaterAnimationPrefix, AnimationLifetime,
    FaceAnimationManagers,
};
use super::*;
use crate::actor_face_playback::{FaceCommandPhase, FaceCommands};

struct MissingJobManager {
    calls: Vec<&'static str>,
}
impl ActorAnimationInitializeSource for MissingJobManager {
    type Error = &'static str;
    fn read_use_jobs_for_skater_anim_update(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("cvar");
        Ok(true)
    }
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        self.calls.push("manager");
        Err("missing actual manager")
    }
}

#[test]
fn missing_job_manager_preserves_allocated_entry_and_actor_attributes_before_trees() {
    // Controlled already-constructed fields isolate Initialize's failure prefix.
    // The separate original Actor corpus supplies the real constructor path.
    let authority = ActorGraphAuthority::new(0_u32);
    let mut animation = ActorAnimationConstruction::new(
        &authority,
        ActorAnimationInputs {
            physics_world: Arc::new(()),
            life: AnimationLifetime::Transient,
            use_low_blend_cull_threshold: false,
            face_name: None,
        },
    );
    animation.phase = ActorAnimationPhase::ConstructorReturned;
    animation.skater = Some(ActorSkaterAnimationPrefix::new(
        &animation.inputs.physics_world,
    ));
    let managers = FaceAnimationManagers::new();
    let database = managers.database.lock().unwrap().allocate().unwrap();
    let commands = Arc::new(Mutex::new(FaceCommands {
        database: Arc::clone(&database),
fixed_frames: Vec::new(),
            fixed_matrices: Vec::new(),
        max_frames: 0,
        phase: FaceCommandPhase::Ready,
        scratch: None,
        selected_frames: Vec::new(),
        setup: None,
        buffered: None,
    }));
    let playback = Arc::new(Mutex::new(FacePlaybackData {
        database,
        databases: Vec::new(),
        commands,
    }));
    let mut source = MissingJobManager { calls: Vec::new() };
    let result = animation.try_initialize_body(
        ActorAnimationInitializeInputs {
            playback: Some(playback),
            scheduler: Some(Arc::new(AnimationJobScheduler::new())),
            shared_attributes: None,
            stance: 0,
            gestures: [0; 4],
            pro_skater: None,
        },
        &mut source,
    );
    assert_eq!(
        result,
        Err(ActorAnimationInitializeError::Source(
            "missing actual manager"
        ))
    );
    assert_eq!(source.calls, ["cvar", "manager"]);
    let retained = animation.initialization().unwrap();
    assert_eq!(
        retained.stage,
        AnimationInitializeStage::EntryPointAllocation
    );
    assert_eq!(
        retained
            .attributes
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .memory
            .len(),
        0x26400
    );
    assert!(retained.entry_point.as_ref().unwrap().value.is_none());
    assert!(retained.entry_point.as_ref().unwrap().manager.is_none());
    assert!(retained.tree_pools.is_empty());
    assert!(retained.playback.is_none());
    assert!(retained.job.is_none());
}

#[test]
fn channel_constructor_retains_unknown_blend_fields_and_ordered_actual_pools() {
    let mut initialized = ActorAnimationInitialization::new();
    let mut channel = ActorChannelAnimatable::new();
    initialized.initialize_channels::<()>(&mut channel, false).unwrap();
    let retained = initialized.channels.as_ref().unwrap();
    assert_eq!(retained.free_channels, [0, 1, 2, 3, 4, 5]);
    assert_eq!(retained.free_trees, (0..12).collect::<Vec<_>>());
    assert!(
        retained
            .trees
            .iter()
            .all(|pool| pool.memory.len() == 1024 && pool.elements.len() == 20)
    );
    assert!(retained.dictionary.is_empty());
    assert!(
        retained
            .channels
            .iter()
            .all(|channel| channel.blend_coefficient.is_none()
                && channel.channel_flags.is_none()
                && channel.base_animation == AnimationPointerState::Unwritten
                && channel.transition == AnimationPointerState::Null
                && !channel.transitioning)
    );
    assert_eq!(channel.root_channel, AnimationPointerState::Null);
}
