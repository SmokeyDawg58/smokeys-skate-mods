use super::*;

fn manager() -> CameraShotManager {
    let vault = crate::attribute_runtime::tests::graph(
        crate::camera_shot_manager::attributes::CAMERA_SHOTS_CLASS,
        vec![],
    );
    CameraShotManager::try_construct(Arc::new(Mutex::new(CameraShotDatabase::constructed(
        Arc::new(vault),
    ))))
    .unwrap()
}

#[test]
fn choices_publish_in_order_and_clear_retains_payload() {
    let mut manager = manager();
    manager.try_add_choice::<()>(b"first").unwrap();
    manager.try_add_choice::<()>(b"second").unwrap();
    assert_eq!(manager.choice_count, 2);
    assert_eq!(name_bytes::<()>(&manager.choices[0]).unwrap(), b"first");
    manager.clear_choices();
    assert_eq!(manager.choice_count, 0);
    assert_eq!(name_bytes::<()>(&manager.choices[1]).unwrap(), b"second");
    assert!(matches!(
        manager.try_add_choice::<()>(&[b'x'; 32]),
        Err(ShotSelectionError::Attribute(
            ShotAttributeError::NameLengthAssertion(32)
        ))
    ));
    assert_eq!(manager.choice_count, 0);
    assert_eq!(name_bytes::<()>(&manager.choices[0]).unwrap(), b"first");
}

#[test]
fn native_assignment_preserves_unknown_bytes_and_aliases_existing_slot() {
    let mut manager = manager();
    let child = Arc::clone(&manager.blends[7]);
    manager.interpolated.blend_shots[0] = Some(Arc::clone(&child));
    manager.interpolated.name[31] = Some(0xaa);
    manager.interpolated.blend_names[2][30] = Some(0xbb);
    manager.interpolated.world_arm_orientation = [1.0, 2.0, 3.0, -0.0];
    manager.previous = manager.interpolated.clone();
    assert_eq!(manager.blends.len(), 64);
    assert!(Arc::ptr_eq(
        manager.previous.blend_shots[0].as_ref().unwrap(),
        &child
    ));
    child.lock().unwrap().distance = 73.0;
    assert_eq!(
        manager.previous.blend_shots[0]
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .distance
            .to_bits(),
        73.0_f32.to_bits()
    );
    assert_eq!(manager.previous.name[31], Some(0xaa));
    assert_eq!(manager.previous.name[1], None);
    assert_eq!(manager.previous.blend_names[2][30], Some(0xbb));
    assert_eq!(
        manager.previous.world_arm_orientation[3].to_bits(),
        (-0.0_f32).to_bits()
    );
}

#[test]
fn host_storage_stop_does_not_advance_native_count() {
    let mut manager = manager();
    for _ in 0..10 {
        manager.try_add_choice::<()>(b"a").unwrap();
    }
    assert!(matches!(
        manager.try_add_choice::<()>(b"b"),
        Err(ShotSelectionError::ChoiceStorageBoundary(10))
    ));
    assert_eq!(manager.choice_count, 10);
    assert_eq!(name_bytes::<()>(&manager.choices[9]).unwrap(), b"a");
}
