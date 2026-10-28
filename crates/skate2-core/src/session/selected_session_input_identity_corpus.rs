//! Original-data check for the one selected input owner before a full frame exists.
//! The pad and disabled Juice transport are explicit test fixtures. All map,
//! device, command, player, and Actor owners come from the production path.
use super::Platform as DisabledJuicePlatform;
use crate::{
    input_pad::device::tests::Platform as PadPlatform,
    juice_trick_input::boot::JuiceHostOption,
    native_skater_random::{NativeRandomGate, SimulationCommandRandom},
    player::{DEFAULT_MARQUEE_ID, NO_PEER_ID, PlayerLoadPhase},
    selected_session_boot_owner::{ControlledPlayerPolicy, SelectedSession},
    selected_sim_command_queue::{SelectedSimCommandQueue, SimCommandCounters},
};
use std::{convert::Infallible, path::PathBuf, sync::Arc};

#[expect(
    clippy::unnecessary_wraps,
    reason = "the selected thread provider's fallible callback signature is explicit"
)]
fn selected_thread() -> Result<u64, Infallible> {
    Ok(1)
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
#[expect(
    clippy::too_many_lines,
    reason = "single original-data boot checks one retained input occurrence through command delivery"
)]
fn original_boot_input_map_and_juice_reach_one_actor_and_command_dispatcher() {
    let root = PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("original game root"));
    let mounts =
        Arc::new(crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus());
    let mut session =
        SelectedSession::<fn() -> Result<u64, Infallible>, u32>::new(Arc::clone(&mounts), &root);
    let mut pad = PadPlatform::new();
    let mut juice_platform = DisabledJuicePlatform::new();
    juice_platform.initialized = false;
    let juice_ini = std::fs::read(root.join("data").join("juice").join("Juice_m.ini"))
        .expect("original Juice config");
    session
        .try_boot_prefix(
            &mut pad,
            &mut juice_platform,
            Some(&juice_ini),
            JuiceHostOption::DisableBeforeStart,
        )
        .unwrap();

    let input = session.input.as_ref().expect("early input owner");
    let map = input.shared_map().expect("mounted input.cfg map");
    let juice = input.trick_input();
    let assets = session.assets.as_ref().expect("original assets");
    assert!(Arc::ptr_eq(&assets.mounts, &mounts));
    let spawn = assets.spawn;
    session
        .try_continue_controlled_player(
            ControlledPlayerPolicy::HostSpawnWhileRegistrationPending,
            spawn,
            DEFAULT_MARQUEE_ID,
            1,
            selected_thread,
        )
        .unwrap();

    let player = session.player.as_ref().expect("one selected player");
    let selected = &player.selected;
    assert!(selected.actor.is_some(), "Actor must be published");
    assert_eq!(
        player.registration_request.player_index,
        selected.owners.loader.player_index()
    );
    assert_eq!(player.registration_request.peer_id, NO_PEER_ID);
    assert_eq!(
        selected.owners.loader.phase(),
        PlayerLoadPhase::Registering {
            request_id: player.registration_request.request_id,
            character_id: player.registration_request.character_id,
        },
    );
    assert!(Arc::ptr_eq(
        &map,
        selected.owners.map.as_ref().expect("same player map"),
    ));
    assert!(Arc::ptr_eq(&juice.state, &selected.owners.juice.state));
    assert!(Arc::ptr_eq(
        &juice.state,
        &selected
            .physical_animation
            .as_ref()
            .expect("same Actor animation owner")
            .input_services
            .juice
            .state,
    ));

    // Bind consumes the exact early device/map/remap owner. The production
    // binder rejects a foreign map Arc before it can poll or queue anything.
    let queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    let random =
        SimulationCommandRandom::construct_field(NativeRandomGate::from_static_initializer());
    session
        .try_bind_selected_input(queue, random)
        .unwrap_or_else(|error| {
            panic!(
                "retained production input bind: {:?}",
                std::mem::discriminant(&error)
            )
        });
    assert!(session.input.is_none(), "bootstrap moved into bound owner");
    let mut game_info = Arc::clone(
        session
            .world
            .as_ref()
            .expect("same world")
            .world
            .lock()
            .unwrap()
            .physical_world_services()
            .expect("same physical world services")
            .game_info(),
    );
    let input = session
        .bound_input
        .as_mut()
        .expect("same bound input owner");
    input.poll_and_cache(&mut pad).unwrap();
    assert!(
        input
            .try_preupdate_enqueue(
                &mut pad,
                &session
                    .player
                    .as_ref()
                    .expect("same selected player")
                    .selected
                    .owners
                    .loader,
                true,
                true,
                &game_info,
            )
            .unwrap()
    );
    let queued = input.queued_tick_input();
    let report = session
        .player
        .as_mut()
        .expect("same player")
        .selected
        .input
        .try_deliver_queued_commands(queued.queue, queued.random, &mut game_info)
        .unwrap();
    assert_eq!(report.delivered, 2, "focus and player sample");
    assert_eq!(report.missing_receivers, 0);
    assert_eq!(report.update_mask, 3);
}
