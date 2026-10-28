//! Original mounted face asset into the SAME Actor's retained animation.
use crate::{actor_construction::ActorConstruction, character_face_manager::CharacterFaceManager};
use skate2_formats::mount::{MountedBig, SKATE2_ARCHIVES};

pub(crate) fn load_tutorial_face() -> CharacterFaceManager {
    let root = std::env::var_os("SKATE2_GAME_ROOT").expect("external original root");
    let spec = SKATE2_ARCHIVES
        .iter()
        .find(|spec| spec.relative_path == "data\\big\\anim.big")
        .unwrap();
    let archive = MountedBig::open(
        spec.path_under(std::path::Path::new(&root)),
        spec.use_external_header,
    )
    .unwrap();
    let mut faces = CharacterFaceManager::new();
    // Controlled recipe request/callback identities; actual file route and
    // loaded/copied ownership come from the retained manager and original BIG.
    let admission = faces.begin_load(b"tutorial_male", 1, 1);
    assert!(!admission.already_loaded);
    let ticket = admission.load.unwrap();
    let route = std::str::from_utf8(&ticket.archive_path).unwrap();
    let (container, entry) = route.split_once('|').unwrap();
    assert_eq!(container, spec.relative_path);
    let bytes = archive.read_path(entry).unwrap();
    assert_eq!(bytes.len(), 16_336);
    faces
        .complete_load(&ticket, true, bytes.into_boxed_slice())
        .unwrap();
    let mut delivered = Vec::new();
    faces
        .dispatch_finished::<()>(|request| {
            delivered.push((request.asset_id, request.callback));
            Ok(())
        })
        .unwrap();
    assert_eq!(delivered, [(1, 1)]);
    faces
}

struct Sprint(crate::input_cvar_runtime::InputCvarProvider);
impl crate::actor_animation_construction::ActorAnimationCvars for Sprint {
    type Error = crate::input_cvar_runtime::InputCvarError;
    fn use_sprint_button(&mut self) -> Result<u8, Self::Error> {
        use crate::action_graph_input_listener::fill::{ActionInputBoolCvar, ActionInputFillCvars};
        self.0
            .capture_boolean(ActionInputBoolCvar::UseSprintButtonInput)?;
        Ok(u8::from(
            self.0.boolean(ActionInputBoolCvar::UseSprintButtonInput)?,
        ))
    }
}
pub(crate) fn finish<S: crate::simulation_animation_startup::AnimationStartupSource>(
    actor: &ActorConstruction,
    vault: &std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    faces: &CharacterFaceManager,
    cvars: &crate::input_cvar_runtime::InputCvarReferences,
    startup_source: &mut S,
) where
    S::Error: std::fmt::Debug,
{
    let retained = actor.retained_animation().unwrap().unwrap();
    {
        let mut animation = retained.lock().unwrap();
        animation
            .try_construct_vault_suffix(
                std::sync::Arc::clone(vault),
                &mut Sprint(cvars.fill_provider()),
            )
            .unwrap();
        animation
            .try_finish_loaded_face_with_source(faces, startup_source)
            .unwrap();
        let face = animation.face().unwrap();
        assert!(face.returned);
        assert_eq!(face.database.as_ref().unwrap().lock().unwrap().slot, 4);
        assert_eq!(face.local_pose.len().unwrap(), 31);
        assert!(face.local_pose.is_unwritten().unwrap());
        let commands = face.commands.as_ref().unwrap().lock().unwrap();
        assert_eq!(commands.fixed_frames.len(), 30);
        assert_eq!(commands.selected_frames, (0..30).collect::<Vec<_>>());
        assert!(commands.fixed_frames.iter().all(|frame| frame.len() == 31));
        assert_ne!(commands.fixed_frames[0], commands.fixed_frames[29]);
    }
    actor.try_publish_animation_constructor().unwrap();
    let state = actor.prefix().unwrap();
    assert_eq!(
        state.phase(),
        crate::actor_construction::ActorPrefixPhase::AnimationConstructorReturned
    );
    assert!(!state.fields().unwrap().animation_null);
    eprintln!(
        "original anim BIG -> ordered body/hand startup -> actual face load/copy at slot4 -> SAME Actor animation/World -> 30 fixed RAW frames -> constructor publication; actual startup cvars, controlled backend/recipe schedule"
    );
}

struct InitializeSource<'a> {
    cvars: &'a crate::input_cvar_runtime::InputCvarReferences,
    startup: &'a crate::simulation_animation_startup::SimulationAnimationStartup,
    jobs: std::sync::Arc<crate::animation_job_owners::AnimationJobManager>,
}
impl crate::actor_construction::animation_start::ActorAnimationStartSource
    for InitializeSource<'_>
{
    // Controlled PlayersBackEnd responses; the actual caller selects its path.
    fn is_stance_regular(&mut self, _: u32) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn skater_id(&mut self, _: u32) -> Result<u64, Self::Error> {
        Ok(0)
    }
    fn player_gestures(&mut self, _: u32) -> Result<[u32; 4], Self::Error> {
        panic!("non-custom skater must not read custom gestures")
    }
    fn player_style(&mut self, _: u32) -> Result<u32, Self::Error> {
        panic!("non-custom skater must not read custom style")
    }
    fn command_manager(
        &mut self,
    ) -> Result<
        std::sync::Arc<std::sync::Mutex<crate::actor_face_playback::FaceCommandManager>>,
        Self::Error,
    > {
        Ok(std::sync::Arc::clone(&self.startup.managers.command))
    }
    fn playback_manager(
        &mut self,
    ) -> Result<
        std::sync::Arc<std::sync::Mutex<crate::actor_face_playback::FacePlaybackManager>>,
        Self::Error,
    > {
        Ok(std::sync::Arc::clone(&self.startup.managers.playback))
    }
}
impl crate::actor_animation_construction::initialization::ActorAnimationInitializeSource
    for InitializeSource<'_>
{
    type Error = crate::input_cvar_runtime::InputCvarError;
    fn read_use_jobs_for_skater_anim_update(&mut self) -> Result<bool, Self::Error> {
        self.cvars.try_use_jobs_for_skater_anim_update()
    }
    fn require_job_manager(
        &mut self,
    ) -> Result<std::sync::Arc<crate::animation_job_owners::AnimationJobManager>, Self::Error> {
        Ok(std::sync::Arc::clone(&self.jobs))
    }
}

/// Legacy diagnostic driver now shares the production constructor handoff type.
pub(crate) type OriginalPhysicalAnimationInputs =
    crate::actor_construction::physical_animation_inputs::ActorPhysicalAnimationInputs;

pub(crate) fn initialize(
    actor: &ActorConstruction,
    startup: &crate::simulation_animation_startup::SimulationAnimationStartup,
    cvars: &crate::input_cvar_runtime::InputCvarReferences,
    input_services: crate::actor_construction::input_generation::runtime::ActorInputRuntimeServices,
) -> OriginalPhysicalAnimationInputs {
    use crate::actor_animation_construction::initialization::AnimationInitializeStage;
    use std::sync::Arc;
    let player = actor.prefix().unwrap().fields().unwrap().player_index;
    let scheduler = Arc::clone(&startup.managers.command.lock().unwrap().scheduler);
    let playback = named_playback(startup, b"Skater", player);
    let mut source = InitializeSource {
        cvars,
        startup,
        jobs: startup.retained_job_manager().unwrap().unwrap(),
    };
    actor
        .try_initialize_animation(&mut source, startup.attributes.as_ref().map(Arc::clone))
        .unwrap();
    assert_eq!(
        actor.prefix().unwrap().phase(),
        crate::actor_construction::ActorPrefixPhase::AnimationInitialized
    );
    actor.try_construct_animation_physics_input().unwrap();
    assert_eq!(
        actor.prefix().unwrap().phase(),
        crate::actor_construction::ActorPrefixPhase::AnimationPhysicsInputConstructed
    );
    let retained = actor.retained_animation().unwrap().unwrap();
    let mut animation = retained.lock().unwrap();
    let state = animation.initialization().unwrap();
    assert_eq!(state.stage, AnimationInitializeStage::BodyReturned);
    assert!(Arc::ptr_eq(state.playback.as_ref().unwrap(), &playback));
    assert!(Arc::ptr_eq(state.scheduler.as_ref().unwrap(), &scheduler));
    assert!(Arc::ptr_eq(
        state.hierarchy.as_ref().unwrap(),
        &startup.databases[1]
    ));
    assert_eq!(state.global_matrices.allocation_len().unwrap(), 36);
    assert_eq!(state.local_matrices.allocation_len().unwrap(), 36);
    assert!(state.job.is_some());
    assert_eq!(
        state
            .bind_pose
            .as_ref()
            .unwrap()
            .constant_vbr_sqt()
            .unwrap()
            .len(),
        36
    );
    for (index, hand) in state.hands.iter().enumerate() {
        let hand = hand.as_ref().unwrap();
        assert!(Arc::ptr_eq(&hand.playback, &startup.hands[index]));
        assert_eq!(hand.local_pose.len().unwrap(), 17);
    }
    let hierarchy = Arc::clone(state.hierarchy.as_ref().unwrap());
    let parameters = playback
        .lock()
        .unwrap()
        .get_physics_parameters(
            crate::actor_graph_construction::ActorGraphIntents::native_name(b"phys_Tpose"),
        )
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&parameters.database, &startup.databases[0]));
    let mut pose = crate::actor_animation_construction::initialization::bind_pose::OwnedAnimationPoseBuffer::new();
    animation.try_get_bind_pose(&mut pose, &mut source).unwrap();
    assert_eq!(pose.matrices().unwrap().unwrap().len(), 36);
    drop(animation);
    crate::actor_construction::body_session::corpus::check(actor, startup, cvars);
    eprintln!(
        "SAME Actor -> body/hand Initialize -> actual OffBoard physics-parameter lookup -> queued bind pose in OnBoard hierarchy; constructor inputs evaluated, controlled CAC/selected schedule, no live animation frame"
    );
    OriginalPhysicalAnimationInputs {
        // This component fixture has no retained player backend; preserve its
        // explicit non-bouncy human inputs. Selected startup samples real owners.
        constructor_flags: crate::actor_construction::player_animation::PhysicalConstructorFlags {
            bouncy_ragdoll: 0,
            is_human: 1,
        },
        input_services,
        body_services:
            crate::actor_construction::body_session::ActorBodyAnimationServices::try_from_startup(
                startup, cvars,
            )
            .unwrap(),
        hierarchy,
        parameters,
        pose,
    }
}

fn named_playback(
    startup: &crate::simulation_animation_startup::SimulationAnimationStartup,
    name: &[u8],
    player: u32,
) -> std::sync::Arc<std::sync::Mutex<crate::actor_face_playback::FacePlaybackData>> {
    let key = (
        crate::actor_graph_construction::ActorGraphIntents::native_name(name),
        i32::from_ne_bytes(player.to_ne_bytes()),
    );
    std::sync::Arc::clone(
        startup
            .managers
            .playback
            .lock()
            .unwrap()
            .entries
            .get(&key)
            .unwrap(),
    )
}
