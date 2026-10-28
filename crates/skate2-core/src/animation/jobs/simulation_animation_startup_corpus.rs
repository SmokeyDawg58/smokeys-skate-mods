use super::*;
use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
use std::path::Path;

type FixtureThread = fn() -> Result<u64, std::convert::Infallible>;
pub(crate) struct MountedSource {
    production: super::mounted_source::MountedAnimationSource<FixtureThread>,
    reads: Vec<String>,
    cvar_reads: usize,
    _cvar_manager: Option<crate::input_cvar_runtime::InputCvarManager>,
}
impl AnimationStartupSource for MountedSource {
    type Error = String;
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        self.production
            .require_job_manager()
            .map_err(|error| error.to_string())
    }
    fn current_thread_id(&mut self) -> Result<u64, Self::Error> {
        self.production
            .current_thread_id()
            .map_err(|error| error.to_string())
    }
    fn read_use_vbr(&mut self) -> Result<bool, Self::Error> {
        self.cvar_reads += 1;
        self.production
            .read_use_vbr()
            .map_err(|error| error.to_string())
    }
    fn read_animation(&mut self, path: &str) -> Result<Box<[u8]>, Self::Error> {
        self.reads.push(path.to_owned());
        self.production
            .read_animation(path)
            .map_err(|error| error.to_string())
    }
}
// Explicit corpus policy; production receives its host's thread service.
#[expect(
    clippy::unnecessary_wraps,
    reason = "fixture implements the fallible host thread callback contract"
)]
fn fixture_thread() -> Result<u64, std::convert::Infallible> {
    Ok(1)
}

pub(crate) fn load_original_startup() -> SimulationAnimationStartup {
    load_original_with_source().0
}

pub(crate) fn load_original_startup_with_cvars(
    cvars: &crate::input_cvar_runtime::InputCvarReferences,
) -> SimulationAnimationStartup {
    load_original_using_cvars(cvars.clone(), None).0
}

pub(crate) fn load_original_startup_with_cvars_and_source(
    cvars: &crate::input_cvar_runtime::InputCvarReferences,
) -> (SimulationAnimationStartup, MountedSource) {
    load_original_using_cvars(cvars.clone(), None)
}

fn load_original_with_source() -> (SimulationAnimationStartup, MountedSource) {
    let (manager, cvars) = crate::game_input_recognizer_corpus::original_input_cvars();
    load_original_using_cvars(cvars, Some(manager))
}

fn load_original_using_cvars(
    cvars: crate::input_cvar_runtime::InputCvarReferences,
    manager: Option<crate::input_cvar_runtime::InputCvarManager>,
) -> (SimulationAnimationStartup, MountedSource) {
    let root = std::env::var("SKATE2_GAME_ROOT").expect("original external root");
    let archive = SKATE2_ARCHIVES
        .iter()
        .find(|entry| entry.relative_path == "data\\big\\anim.big")
        .unwrap();
    let mut mounts = MountSet::new();
    mounts.push(
        MountedBig::open(
            archive.path_under(Path::new(&root)),
            archive.use_external_header,
        )
        .unwrap(),
    );
    let mut jobs = AnimationJobManager::new();
    jobs.try_initialize_local_backend()
        .expect("original manager Init local scheduler");
    let mut source = MountedSource {
        production: super::mounted_source::MountedAnimationSource::new(
            Arc::new(mounts),
            Arc::new(jobs),
            cvars,
            fixture_thread as FixtureThread,
        ),
        reads: Vec::new(),
        cvar_reads: 0,
        _cvar_manager: manager,
    };
    let managers = FaceAnimationManagers::new();
    let mut startup = SimulationAnimationStartup::new(managers.clone());
    startup.try_initialize(&mut source).unwrap();
    (startup, source)
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_ordered_body_pose_winner_and_both_hand_fixed_sets() {
    let (startup, source) = load_original_with_source();
    let managers = &startup.managers;
    assert!(startup.databases_and_fixed_hands_ready);
    assert_eq!(source.cvar_reads, 2);
    assert_eq!(
        source.reads,
        [
            "data\\anim/OffBoard.abin",
            "data\\anim/OnBoard.abin",
            "data\\anim/SkaterLeftHand.abin",
            "data\\anim/SkaterRightHand.abin"
        ]
    );
    assert_eq!(startup.body.len(), 9);
    assert_eq!(startup.hands.len(), 2);
    assert_eq!(managers.command.lock().unwrap().entries.len(), 11);
    assert_eq!(managers.playback.lock().unwrap().entries.len(), 11);
    for (slot, database) in startup.databases.iter().enumerate() {
        assert_eq!(usize::from(database.lock().unwrap().slot), slot);
    }
    for playback in &startup.body {
        let retained = playback.lock().unwrap();
        assert!(Arc::ptr_eq(&retained.database, &startup.databases[1]));
        assert!(Arc::ptr_eq(&retained.databases[0], &startup.databases[0]));
        assert!(Arc::ptr_eq(&retained.databases[1], &startup.databases[1]));
        let physics = retained
            .get_physics_parameters(ActorGraphIntents::native_name(b"phys_Tpose"))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&physics.database, &startup.databases[0]));
        let parameters = physics.snapshot().unwrap();
        assert_eq!(parameters.record_offset, 0x8b80);
        assert!(
            retained
                .get_physics_parameters(ActorGraphIntents::native_name(b"absent_physics_key"))
                .unwrap()
                .is_none()
        );
        let onboard = startup.databases[1].lock().unwrap();
        let onboard_parameters = &onboard.parsed.as_ref().unwrap().physics_parameters[0];
        assert_eq!(onboard_parameters.record_offset, 0x2e00);
        assert_eq!(parameters.bones, onboard_parameters.bones);
        drop(onboard);
        let pose = retained
            .get_pose(ActorGraphIntents::native_name(b"rig_Tpose"))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&pose.database, &startup.databases[0]));
        {
            let database = pose.database.lock().unwrap();
            let parsed = database.parsed.as_ref().unwrap();
            assert_eq!(parsed.records[pose.record_index].offset, 0x0046_14d0);
            assert_eq!(parsed.records[pose.record_index].header.codec, 0x0052_4256);
        }
        let sqt = pose.constant_vbr_sqt().unwrap();
        assert_eq!(sqt.len(), 36);
        assert!(sqt.iter().all(|sqt| {
            sqt.scale
                .iter()
                .chain(&sqt.rotation)
                .chain(&sqt.translation)
                .all(|value| value.is_finite())
        }));
        for (name, offset) in [
            (b"board_backwards".as_slice(), 0x0046_1b80),
            (b"board_backwards_ik".as_slice(), 0x0046_20b0),
        ] {
            let pose = retained
                .get_pose(ActorGraphIntents::native_name(name))
                .unwrap()
                .unwrap();
            let database = pose.database.lock().unwrap();
            assert_eq!(
                database.parsed.as_ref().unwrap().records[pose.record_index].offset,
                offset
            );
        }
    }
    for hand in &startup.hands {
        let retained = hand.lock().unwrap();
        let command = retained.commands.lock().unwrap();
        assert_eq!(command.fixed_frames.len(), 61);
        assert_eq!(command.selected_frames, (0..61).collect::<Vec<_>>());
        assert!(command.fixed_frames.iter().all(|frame| frame.len() == 17));
        assert!(command.scratch.is_none());
        assert_eq!(command.setup.as_ref().unwrap().characters, 9);
        assert_eq!(command.setup.as_ref().unwrap().codec, 0);
    }
    // Body/hand initialization and selected face must share the actual slot pool.
    let next = managers.database.lock().unwrap().allocate().unwrap();
    assert_eq!(next.lock().unwrap().slot, 4);
}
