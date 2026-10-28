use super::*;
use crate::input_pad::device_manager::selected_runtime::player_sample::PlayerInputPause;
#[test]
fn constructor_init_is_retained_and_active_through_same_input_adapter() {
    let mut owner = SelectedPauseManager::construct();
    assert!(owner.try_is_simulation_active().unwrap());
    assert_eq!(owner.fields.lock().unwrap().stack, [0]);
    let other = Arc::clone(&owner);
    other.try_push(1).unwrap();
    assert!(!owner.try_is_simulation_active().unwrap());
    other.try_push(21).unwrap();
    assert!(owner.try_is_simulation_active().unwrap());
    other.try_pull(21).unwrap();
    assert!(!owner.try_is_simulation_active().unwrap());
}
#[test]
fn pull_removes_all_matches_and_preserves_top_order_and_empty_flags() {
    let owner = SelectedPauseManager::construct();
    for state in [1, 2, 1, 21, 1] {
        owner.try_push(state).unwrap();
    }
    owner.try_pull(1).unwrap();
    assert_eq!(owner.fields.lock().unwrap().stack, [0, 2, 21]);
    assert!(owner.try_is_active(1).unwrap());
    for state in [0, 2, 21] {
        owner.try_pull(state).unwrap();
    }
    assert!(owner.fields.lock().unwrap().stack.is_empty());
    assert!(owner.try_is_active(u32::MAX).unwrap());
}
#[test]
fn is_paused_reads_the_top_state_entry() {
    let owner = SelectedPauseManager::construct();
    assert!(!owner.try_is_paused(1).unwrap());
    owner.try_push(11).unwrap();
    assert!(owner.try_is_paused(1).unwrap());
    assert!(owner.try_is_paused(16).unwrap());
    owner.try_push(3).unwrap();
    assert!(owner.try_is_paused(1).unwrap());
    assert!(!owner.try_is_paused(16).unwrap());
    owner.try_pull(3).unwrap();
    owner.try_push(21).unwrap();
    assert!(!owner.try_is_paused(1).unwrap());
    owner.try_pull(21).unwrap();
    owner.try_pull(11).unwrap();
    assert!(!owner.try_is_paused(1).unwrap());
}
#[test]
fn capacity_assertion_preserves_existing_stack_and_flags() {
    let owner = SelectedPauseManager::construct();
    for _ in 0..52 {
        owner.try_push(1).unwrap();
    }
    assert_eq!(owner.try_push(21), Err(PauseError::StackCapacity));
    assert_eq!(owner.fields.lock().unwrap().stack.len(), 53);
    assert!(!owner.try_is_active(1).unwrap());
    assert_eq!(owner.try_push(26), Err(PauseError::UnsupportedState(26)));
}
