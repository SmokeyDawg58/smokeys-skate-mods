//! Assertions over the actual retained first-player authored presentation startup.
use crate::actor_construction::selected_startup::SelectedPlayerStartupOwners;
use std::sync::Arc;

pub(crate) fn check(owners: &SelectedPlayerStartupOwners) {
    let selected = owners.presentation.as_ref().unwrap();
    let animation = &selected.animation;
    let restored = selected.restored.as_ref().unwrap();
    assert!(animation.initialized);
    assert!(animation.twist_bones.iter().all(Option::is_some));
    assert_eq!(animation.databases.len(), 2);
    assert_eq!(animation.replay.len(), 18);
    assert_eq!(animation.interpolated.len(), 9);
    assert!(Arc::ptr_eq(
        &animation.managers.database,
        &owners.animation.managers.database
    ));
    assert!(!Arc::ptr_eq(
        &animation.managers.command,
        &owners.animation.managers.command
    ));
    assert!(!Arc::ptr_eq(
        &animation.managers.playback,
        &owners.animation.managers.playback
    ));
    let jobs = owners.animation.retained_job_manager().unwrap().unwrap();
    for (index, commands) in animation.pending_commands.iter().enumerate() {
        let commands = commands.lock().unwrap();
        let buffered = commands.buffered.as_ref().unwrap().lock().unwrap();
        assert_eq!(buffered.job_category, 2);
        assert_eq!(buffered.max_dynamic_sqts, if index == 0 { 36 } else { 18 });
        assert_eq!(buffered.max_pose_operations, if index == 0 { 9 } else { 0 });
        assert_eq!(
            buffered.set_pool.as_ref().unwrap().max_work_units,
            if index == 0 { 54 } else { 18 }
        );
        for entry in &buffered.entries {
            assert!(Arc::ptr_eq(entry.manager.as_ref().unwrap(), &jobs));
        }
    }
    let bind = animation.bind.as_ref().unwrap().read().unwrap().1;
    let previous = restored.previous.read().unwrap().1;
    let next = restored.next.read().unwrap().1;
    assert!(!bind.is_empty());
    assert_eq!(previous.len(), bind.len());
    assert_eq!(next.len(), bind.len());
    for (left, right) in previous.iter().zip(&next) {
        assert_eq!(left.scale.map(f32::to_bits), right.scale.map(f32::to_bits));
        assert_eq!(
            left.rotation.map(f32::to_bits),
            right.rotation.map(f32::to_bits)
        );
        assert_eq!(
            left.translation.map(f32::to_bits),
            right.translation.map(f32::to_bits)
        );
    }
    let offsets = animation.offsets.as_ref().unwrap();
    assert_eq!(animation.ghost_next.len(), offsets.pose_bytes);
    assert_eq!(animation.ghost_previous.len(), offsets.pose_bytes);
    let character = owners
        .services
        .settings
        .try_character(owners.loader.player_index())
        .unwrap();
    let instance = owners
        .services
        .vault
        .instance(0x91bc_a669_3efc_9aa7, character);
    let name = instance.try_cached_layout_text(0).unwrap().unwrap();
    assert_eq!(restored.recipe.schema_name.as_bytes(), name.bytes());
    eprintln!(
        "same-player presentation initialized: {} bones, {} packed bytes, authored face found={}",
        bind.len(),
        offsets.pose_bytes,
        restored.face_pose_found
    );
}
