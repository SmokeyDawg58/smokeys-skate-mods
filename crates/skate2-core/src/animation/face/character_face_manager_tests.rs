use super::*;

#[test]
fn duplicate_pending_loads_share_holder_but_keep_callback_order() {
    let mut manager = CharacterFaceManager::new();
    let ticket = manager
        .begin_load(b"tutorial_male\0ignored", 1, 10)
        .load
        .unwrap();
    assert_eq!(
        ticket.archive_path,
        b"data\\big\\anim.big|data\\anim\\faces\\tutorial_male.abin"
    );
    assert!(manager.begin_load(b"tutorial_male", 2, 20).load.is_none());
    assert!(!manager.has_face_data(b"tutorial_male"));
    manager
        .complete_load(&ticket, true, vec![1, 2, 3].into_boxed_slice())
        .unwrap();
    assert!(manager.has_face_data(b"tutorial_male"));
    assert!(!manager.has_face_data(b"TUTORIAL_MALE"));
    let copy = manager.copy_face_data(b"tutorial_male").unwrap();
    let another = manager.copy_face_data(b"tutorial_male").unwrap();
    assert_eq!(copy, another);
    assert_ne!(copy.as_ptr(), another.as_ptr());
    let mut delivered = vec![];
    manager
        .dispatch_finished::<()>(|request| {
            delivered.push((request.asset_id, request.callback));
            Ok(())
        })
        .unwrap();
    assert_eq!(delivered, [(1, 10), (2, 20)]);
    let admission = manager.begin_load(b"tutorial_male", 3, 30);
    assert!(admission.already_loaded && admission.load.is_none());
    assert!(manager.requests.is_empty());
    assert_eq!(
        manager.holders[&CharacterFaceManager::face_hash(b"tutorial_male")].references,
        3
    );
}

#[test]
fn foreign_or_failed_completion_cannot_publish_loaded() {
    let mut manager = CharacterFaceManager::new();
    let ticket = manager.begin_load(b"face", 1, 1).load.unwrap();
    let mut other = CharacterFaceManager::new();
    assert_eq!(
        other.complete_load(&ticket, true, vec![1].into_boxed_slice()),
        Err(CharacterFaceError::ForeignLoad)
    );
    assert_eq!(
        manager.complete_load(&ticket, false, vec![1].into_boxed_slice()),
        Err(CharacterFaceError::LoadFailed)
    );
    assert!(!manager.has_face_data(b"face"));
    assert_eq!(
        manager.copy_face_data(b"face"),
        Err(CharacterFaceError::NotLoaded)
    );
}
