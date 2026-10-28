use super::*;

#[test]
fn physical_parameter_snapshot_rejects_unloaded_retained_record() {
    let managers = FaceAnimationManagers::new();
    let database = managers.database.lock().unwrap().allocate().unwrap();
    let reference = AnimationPhysicsRef {
        database,
        record_index: 0,
    };
    assert_eq!(reference.snapshot(), Err(ActorFaceError::MissingRecord));
}

#[test]
fn setup_frame_times_survive_command_quantization_in_original_face_domain() {
    let selected: Vec<u16> = (0..30)
        .map(|frame| select_face_frame(30.0, 60.0, 1.0, frame).unwrap())
        .collect();
    assert_eq!(selected, (0..30).collect::<Vec<_>>());
    assert_eq!(
        select_face_frame(30.0, 60.0, 1.0, 30),
        Err(ActorFaceError::UnsupportedTiming)
    );
    assert_eq!(
        select_face_frame(1.0, 60.0, 1.0, 0),
        Err(ActorFaceError::UnsupportedTiming)
    );
}

#[test]
fn manager_registries_are_actual_distinct_empty_owners() {
    let managers = FaceAnimationManagers::new();
    let retained = managers.clone();
    assert!(Arc::ptr_eq(&managers.database, &retained.database));
    assert_eq!(managers.database.lock().unwrap().slots.len(), 32);
    assert!(
        managers
            .database
            .lock()
            .unwrap()
            .slots
            .iter()
            .all(Option::is_none)
    );
    assert!(managers.command.lock().unwrap().entries.is_empty());
    assert!(managers.playback.lock().unwrap().entries.is_empty());
}

#[test]
#[ignore = "requires original external SKATE2_GAME_ROOT face corpus"]
fn original_tutorial_face_load_copy_register_and_fixed_frames() {
    use std::io::{Read, Seek, SeekFrom};
    let root = std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let path = std::path::PathBuf::from(root).join("data/big/anim.big");
    let mut file = std::fs::File::open(path).unwrap();
    // Independently established Viv4 original tutorial_male entry; corpus only.
    file.seek(SeekFrom::Start(16_816_128)).unwrap();
    let mut bytes = vec![0; 16_336];
    file.read_exact(&mut bytes).unwrap();
    let mut faces = CharacterFaceManager::new();
    let ticket = faces.begin_load(b"tutorial_male", 1, 1).load.unwrap();
    faces
        .complete_load(&ticket, true, bytes.into_boxed_slice())
        .unwrap();
    let managers = FaceAnimationManagers::new();
    let mut setup = ActorFacePlayback::new(b"tutorial_male", 0, managers.clone());
    setup.try_setup(b"tutorial_male", &faces).unwrap();
    assert!(setup.returned);
    assert_eq!(setup.local_pose.len().unwrap(), 31);
    assert!(setup.local_pose.is_unwritten().unwrap());
    let commands = setup.commands.as_ref().unwrap().lock().unwrap();
    assert_eq!(commands.fixed_frames.len(), 30);
    assert_eq!(commands.selected_frames, (0..30).collect::<Vec<_>>());
    assert!(commands.fixed_frames.iter().all(|frame| frame.len() == 31));
    assert_ne!(commands.fixed_frames[0], commands.fixed_frames[29]);
    assert_eq!(commands.phase, FaceCommandPhase::Ready);
    assert!(commands.scratch.is_none());
    let database = setup.database.as_ref().unwrap().lock().unwrap();
    assert_eq!(database.slot, 0);
    let parsed = database.parsed.as_ref().unwrap();
    assert_eq!(
        parsed
            .hierarchy
            .parts
            .iter()
            .map(|part| part.name_index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(
        parsed.clips[0]
            .parts
            .iter()
            .map(|part| part.part_name_index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(
        parsed.clips[0].frames_per_second.to_bits(),
        60.0_f32.to_bits()
    );
    assert_eq!(setup.playback_name.decode().unwrap(), "tutorial_male_000");
    assert_eq!(managers.playback.lock().unwrap().entries.len(), 1);
    assert_eq!(managers.command.lock().unwrap().entries.len(), 1);
}
