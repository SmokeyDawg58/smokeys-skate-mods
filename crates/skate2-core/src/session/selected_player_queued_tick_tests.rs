use super::*;
use crate::{
    game_input_manager::GameInputCommandManager,
    gameplay_input_update_manager::GameplayInputUpdateManager,
    native_skater_random::NativeRandomGate, selected_sim_command_queue::SimCommandCounters,
};
#[test]
fn real_empty_mailbox_and_empty_accepted_batch_have_different_tick_admission() {
    let mut input = GameInputFrameSession {
        commands: GameInputCommandManager::new(),
        gameplay: GameplayInputUpdateManager::<()>::new(),
    };
    let mut queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    let mut random =
        SimulationCommandRandom::construct_field(NativeRandomGate::from_static_initializer());
    let mut sync=crate::simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo::construct_network_field();
    let absent = input
        .try_deliver_queued_commands(&mut queue, &mut random, &mut sync)
        .unwrap();
    assert_eq!(queued_mask_admitted(&absent), Ok(false));
    assert!(queue.try_generate_sync_command().unwrap());
    let empty = input
        .try_deliver_queued_commands(&mut queue, &mut random, &mut sync)
        .unwrap();
    assert_eq!(queued_mask_admitted(&empty), Ok(true));
    assert_eq!(
        (empty.update_mask, empty.delivered, empty.missing_receivers),
        (3, 0, 0)
    );
    assert_eq!(random.random.index, 0);
}
#[test]
fn visual_only_and_partial_masks_are_explicit_unsupported_without_normalizing_counts() {
    let report = CommandStageDelivery {
        update_mask: 2,
        delivered: 7,
        missing_receivers: 1,
        skater_manager: Vec::new(),
    };
    assert_eq!(queued_mask_admitted(&report), Err(2));
    assert_eq!((report.delivered, report.missing_receivers), (7, 1));
    assert_eq!(
        queued_mask_admitted(&CommandStageDelivery {
            update_mask: 1,
            ..report
        }),
        Err(1)
    );
}
