use crate::physics_state_ids::{PHYSICS_STATE_PHYSICS_GROUND, PHYSICS_STATE_TELEPORTING};
use super::*;

#[test]
fn constructor_unknown_current_and_pre_capture_prefix_are_preserved() {
    let mut fields = StateChangerFields::constructed();
    assert_eq!(fields.current, None);
    fields.teleport_countdown = 2;
    fields.request_teleport = true;
    fields.revert_exited_normally = true;
    fields.air_frames = 7;
    fields.begin(100);
    assert_eq!(fields.current, Some(PHYSICS_STATE_PHYSICS_GROUND));
    assert_eq!(fields.teleport_countdown, 1);
    assert!(!fields.request_teleport);
    assert!(!fields.revert_exited_normally);
    assert_eq!(fields.air_frames, 7);
    fields.teleport_countdown = -1;
    fields.begin(300);
    assert_eq!(fields.teleport_countdown, -1);
}

#[test]
fn contact_counters_wrap_and_keep_body_wheels_distinct_from_processed_contacts() {
    let mut fields = StateChangerFields::constructed();
    fields.two_wheel_counter = i32::MAX;
    fields.three_wheel_counter = 10;
    fields.air_frames = i32::MAX;
    fields.update_contact_counters(701, 200, 2, false, false);
    assert_eq!(fields.nonspecific_free_frames, 1);
    assert_eq!(fields.nonspecific_collision_frames, 0);
    assert_eq!(fields.two_wheel_counter, i32::MIN);
    assert_eq!(fields.three_wheel_counter, 0);
    assert_eq!(fields.something_colliding_frames, 0);
    assert_eq!(fields.post_grind_jump_counter, 1);
    assert_eq!(fields.air_frames, i32::MIN);
    fields.update_contact_counters(100, 400, 0, true, true);
    assert_eq!(fields.nonspecific_free_frames, 0);
    assert_eq!(fields.two_wheel_counter, 0);
    assert_eq!(fields.something_colliding_frames, 1);
    assert_eq!(fields.post_grind_jump_counter, 0);
    assert_eq!(fields.air_frames, 0);
}

#[test]
fn teleport_reset_clears_only_counters_then_loads_countdown() {
    let mut owner = PhysicalPlayerStateChangerOwner::new();
    let mut fields = StateChangerFields::constructed();
    fields.current = Some(PHYSICS_STATE_TELEPORTING);
    fields.nonspecific_free_frames = 4;
    fields.nonspecific_collision_frames = 5;
    fields.something_colliding_frames = 6;
    fields.two_wheel_counter = 7;
    fields.three_wheel_counter = 8;
    fields.post_grind_jump_counter = 9;
    fields.air_frames = 10;
    fields.revert_exited_normally = true;
    fields.request_teleport = true;
    owner.fields = Some(fields);
    let cvars = InputCvarReferences::new_static_initialized();
    let mut manager = crate::input_cvar_runtime::InputCvarManager::new();
    manager.try_link(&cvars).unwrap();

    owner.try_teleport_reset(&cvars).unwrap();

    let fields = owner.fields.unwrap();
    assert_eq!(fields.current, Some(PHYSICS_STATE_TELEPORTING));
    assert_eq!(fields.nonspecific_free_frames, 0);
    assert_eq!(fields.nonspecific_collision_frames, 0);
    assert_eq!(fields.something_colliding_frames, 0);
    assert_eq!(fields.two_wheel_counter, 0);
    assert_eq!(fields.three_wheel_counter, 0);
    assert_eq!(fields.post_grind_jump_counter, 0);
    assert_eq!(fields.air_frames, 0);
    assert_eq!(fields.teleport_countdown, 10);
    assert!(fields.revert_exited_normally);
    assert!(fields.request_teleport);
}
