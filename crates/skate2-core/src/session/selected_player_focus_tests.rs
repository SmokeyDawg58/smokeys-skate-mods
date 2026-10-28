use super::*;
fn queue() -> SelectedSimCommandQueue {
    SelectedSimCommandQueue::construct_offline_command_fields(
        super::super::SimCommandCounters::from_static_initializer(),
    )
}
#[test]
fn gain_focus_queues_empty_kind2_once_and_loss_only_updates_retained_field() {
    let mut focus = SelectedPlayerFocus::construct_fields(u32::MAX);
    let mut queue = queue();
    assert!(focus.try_update_prefix(true, true, &mut queue).unwrap());
    assert_eq!(queue.commands.len(), 1);
    assert_eq!(
        (
            queue.commands[0].receiver,
            queue.commands[0].kind,
            queue.commands[0].length
        ),
        (9, 2, 0)
    );
    assert!(focus.try_update_prefix(true, true, &mut queue).unwrap());
    assert!(focus.try_update_prefix(true, false, &mut queue).unwrap());
    assert!(!focus.has_focus);
    assert_eq!(queue.commands.len(), 1);
    assert!(focus.try_update_prefix(true, true, &mut queue).unwrap());
    assert_eq!(queue.commands.len(), 2);
}
#[test]
fn no_simulation_keeps_focus_on_true_but_clears_on_false() {
    let mut focus = SelectedPlayerFocus::construct_fields(0);
    let mut queue = queue();
    focus.try_update_prefix(true, true, &mut queue).unwrap();
    assert!(!focus.try_update_prefix(false, true, &mut queue).unwrap());
    assert!(focus.has_focus);
    assert!(!focus.try_update_prefix(false, false, &mut queue).unwrap());
    assert!(!focus.has_focus);
    assert_eq!(queue.commands.len(), 1);
}
#[test]
fn focus_store_precedes_failed_command_queue() {
    let mut focus = SelectedPlayerFocus::construct_fields(0);
    let mut queue = queue();
    queue.failed = true;
    assert_eq!(
        focus.try_update_prefix(true, true, &mut queue),
        Err(CommandQueueError::FailedStage)
    );
    assert!(focus.has_focus);
    assert!(queue.commands.is_empty());
}
