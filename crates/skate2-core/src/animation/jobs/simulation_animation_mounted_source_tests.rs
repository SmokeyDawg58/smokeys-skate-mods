use super::*;
use crate::{
    actor_face_playback::FaceAnimationManagers,
    input_cvar_runtime::InputCvarManager,
    simulation_animation_startup::{AnimationStartupError, SimulationAnimationStartup},
};

#[test]
fn thread_failure_keeps_startup_prefix_without_reading_cvars_or_mounts() {
    let mut source = MountedAnimationSource::new(
        Arc::new(MountSet::new()),
        Arc::new(AnimationJobManager::new()),
        InputCvarReferences::new_static_initialized(),
        || Err::<u64, _>("thread unavailable"),
    );
    let mut startup = SimulationAnimationStartup::new(FaceAnimationManagers::new());
    assert!(matches!(
        startup.try_initialize(&mut source),
        Err(AnimationStartupError::Source(
            MountedAnimationSourceError::Thread("thread unavailable")
        ))
    ));
    assert!(
        startup
            .attributes
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .creation_thread
            .is_none()
    );
    assert!(startup.databases.is_empty());
}

#[test]
fn uses_same_jobs_live_cvars_and_typed_mount_failure() {
    let cvars = InputCvarReferences::new_static_initialized();
    let jobs = Arc::new(AnimationJobManager::new());
    let mut source = MountedAnimationSource::new(
        Arc::new(MountSet::new()),
        Arc::clone(&jobs),
        cvars.clone(),
        || Ok::<_, std::convert::Infallible>(0x1234),
    );
    assert!(Arc::ptr_eq(&jobs, &source.require_job_manager().unwrap()));
    assert!(matches!(
        source.read_use_vbr(),
        Err(MountedAnimationSourceError::Cvar(_))
    ));
    let mut manager = InputCvarManager::new();
    manager.try_link(&cvars).unwrap();
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkUseVBR=0\r\n"))
        .unwrap();
    assert!(!source.read_use_vbr().unwrap());
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkUseVBR=1\r\n"))
        .unwrap();
    assert!(source.read_use_vbr().unwrap());
    assert_eq!(source.current_thread_id().unwrap(), 0x1234);
    assert!(matches!(
        source.read_animation("data/anim/missing.abin"),
        Err(MountedAnimationSourceError::Mount(
            MountError::EntryNotFound(_)
        ))
    ));
}
