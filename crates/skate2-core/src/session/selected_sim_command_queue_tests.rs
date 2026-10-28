use super::*;
use crate::{
    game_input_player::GameInputPlayer, input_map::SelectedInputMap,
    input_pad_cache::PublishedInputPadSample, native_skater_random::NativeRandomGate,
    reckoning_numeric::HostReckoningNumeric,
};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
struct Sync {
    value: Result<i32, &'static str>,
    calls: usize,
}
impl GameInputSyncLevel for Sync {
    type Error = &'static str;
    fn sync_level(&mut self) -> Result<i32, Self::Error> {
        self.calls += 1;
        self.value
    }
}
fn owners() -> (
    GameInputCommandManager,
    Arc<Mutex<GameInputPlayer>>,
    SimulationCommandRandom,
    Sync,
) {
    let map =
        Arc::new(SelectedInputMap::parse(&crate::input_map::tests::config("Button0")).unwrap());
    let player = Arc::new(Mutex::new(GameInputPlayer::new(4, map)));
    let mut dispatcher = GameInputCommandManager::new();
    dispatcher.try_construct_player(&player).unwrap();
    (
        dispatcher,
        player,
        SimulationCommandRandom::construct_field(NativeRandomGate::from_static_initializer()),
        Sync {
            value: Ok(2),
            calls: 0,
        },
    )
}
#[test]
fn enqueue_copies_and_does_not_dispatch_until_mailbox_consumption() {
    let (mut dispatcher, player, mut random, mut sync) = owners();
    let mut queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    let mut payload = [0];
    queue
        .try_queue(&EncodedGameInputCommand {
            receiver_id: 14,
            kind: 1,
            payload: &payload,
        })
        .unwrap();
    payload[0] = 1; // A borrowed live slice would now fail codec decoding.
    assert_eq!(payload, [1]);
    assert_eq!(player.lock().unwrap().pad().unwrap().num_buttons(), 0);
    assert_eq!(queue.try_next_generation_frame().unwrap(), 1);
    assert!(queue.try_generate_sync_command().unwrap());
    assert_eq!(queue.try_next_generation_frame().unwrap(), 2);
    assert_eq!(dispatcher.crc(), 0);
    assert_eq!(random.random.index, 0);
    let result = queue
        .try_consume_admitted(&mut dispatcher, &mut random, &mut sync)
        .unwrap();
    assert_eq!(
        result,
        CommandStageDelivery {
            update_mask: 3,
            delivered: 1,
            missing_receivers: 0,
            skater_manager: Vec::new(),
        }
    );
    assert_eq!(player.lock().unwrap().pad().unwrap().num_buttons(), 24);
    assert_eq!(random.random.index, 1);
    assert_eq!(random.random.values[0], 0x559a_51ed);
    assert_eq!(sync.calls, 1);
    assert_eq!(
        queue
            .try_consume_admitted(&mut dispatcher, &mut random, &mut sync)
            .unwrap(),
        CommandStageDelivery::default()
    );
}
#[test]
fn empty_batches_are_valid_busy_send_retains_later_commands_and_overflow_is_allowed() {
    let (mut dispatcher, _player, mut random, mut sync) = owners();
    let mut queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    assert!(queue.try_generate_sync_command().unwrap());
    for _ in 0..12 {
        queue
            .try_queue(&EncodedGameInputCommand {
                receiver_id: 99,
                kind: 2,
                payload: &[],
            })
            .unwrap();
    }
    assert!(!queue.try_generate_sync_command().unwrap());
    assert_eq!(queue.commands.len(), 12);
    assert_eq!(queue.try_next_generation_frame().unwrap(), 2);
    let first = queue
        .try_consume_admitted(&mut dispatcher, &mut random, &mut sync)
        .unwrap();
    assert_eq!(first.update_mask, 3);
    assert_eq!(random.random.index, 0);
    assert!(queue.try_generate_sync_command().unwrap());
    let second = queue
        .try_consume_admitted(&mut dispatcher, &mut random, &mut sync)
        .unwrap();
    assert_eq!(second.missing_receivers, 12);
    assert_eq!(random.random.index, 12);
    assert_eq!(sync.calls, 12);
}
#[test]
fn actual_existing_codec_feeds_same_registered_player() {
    let (mut dispatcher, player, mut random, mut sync) = owners();
    let mut queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    let input = PlayerInputData {
        sample: PublishedInputPadSample {
            count: 24,
            values: [0.0; 24],
        },
        allow_sensor: true,
        has_focus: true,
    };
    queue
        .try_queue_player_input(14, &input, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(player.lock().unwrap().pad().unwrap().num_buttons(), 0);
    queue.try_generate_sync_command().unwrap();
    queue
        .try_consume_admitted(&mut dispatcher, &mut random, &mut sync)
        .unwrap();
    assert!(player.lock().unwrap().has_focus().unwrap());
    assert_eq!(player.lock().unwrap().pad().unwrap().num_buttons(), 24);
}
#[test]
fn normal_failure_keeps_rng_and_dispatch_prefix_and_refuses_replay() {
    let (mut dispatcher, player, mut random, mut sync) = owners();
    let mut queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    for bytes in [&[0][..], &[1][..]] {
        queue
            .try_queue(&EncodedGameInputCommand {
                receiver_id: 14,
                kind: 1,
                payload: bytes,
            })
            .unwrap();
    }
    queue.try_generate_sync_command().unwrap();
    assert!(matches!(
        queue.try_consume_admitted(&mut dispatcher, &mut random, &mut sync),
        Err(CommandConsumeError::Dispatch {
            phase: CommandBatchPhase::Simulation,
            index: 1,
            error: GameInputManagerError::Decode(_)
        })
    ));
    assert_eq!(random.random.index, 2);
    assert_eq!(sync.calls, 2);
    assert_eq!(player.lock().unwrap().pad().unwrap().num_buttons(), 24);
    assert!(!queue.offline.valid);
    assert!(!queue.valid_ghost_command);
    assert!(matches!(
        queue.try_consume_admitted(&mut dispatcher, &mut random, &mut sync),
        Err(CommandConsumeError::Queue(CommandQueueError::FailedStage))
    ));
    assert_eq!(random.random.index, 2);
}
#[test]
fn ghost_failure_keeps_native_flags_and_prevents_mailbox_or_rng_consumption() {
    let (mut dispatcher, _player, mut random, mut sync) = owners();
    let mut queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    // Inspect the already-queued ghost leaf state, without claiming a GhostProxy producer.
    queue.ghost_commands.push(
        QueuedCommand::try_copy(&EncodedGameInputCommand {
            receiver_id: 14,
            kind: 1,
            payload: &[1],
        })
        .unwrap(),
    );
    queue.try_generate_sync_command().unwrap();
    assert!(matches!(
        queue.try_consume_admitted(&mut dispatcher, &mut random, &mut sync),
        Err(CommandConsumeError::Dispatch {
            phase: CommandBatchPhase::Ghost,
            index: 0,
            ..
        })
    ));
    assert!(queue.valid_ghost_command);
    assert_eq!(queue.exec_ghost_commands.len(), 1);
    assert!(queue.offline.valid);
    assert_eq!((random.random.index, sync.calls), (0, 0));
}
#[test]
fn direct_mailbox_refuses_overwrite_and_retains_original_bytes_after_get() {
    let mut mailbox = OfflineCommandMailbox::construct_fields();
    let first = QueuedCommand::try_copy(&EncodedGameInputCommand {
        receiver_id: 12,
        kind: 2,
        payload: &[],
    })
    .unwrap();
    assert!(mailbox.try_send(&[first]).unwrap());
    assert!(!mailbox.try_send(&[]).unwrap());
    let output = mailbox.try_get().unwrap().unwrap();
    assert_eq!(output[0].receiver, 12);
    assert_eq!(mailbox.commands[0].receiver, 12);
    assert!(!mailbox.valid);
    assert!(mailbox.try_get().unwrap().is_none());
}

#[test]
fn source_global_counters_are_shared_wrap_and_keep_failure_prefix() {
    let (mut dispatcher, _player, mut random, mut sync) = owners();
    let counters = SimCommandCounters::from_static_initializer();
    let mut first =
        SelectedSimCommandQueue::construct_offline_command_fields(Arc::clone(&counters));
    let mut second =
        SelectedSimCommandQueue::construct_offline_command_fields(Arc::clone(&counters));
    first
        .try_consume_admitted(&mut dispatcher, &mut random, &mut sync)
        .unwrap();
    second
        .try_consume_admitted(&mut dispatcher, &mut random, &mut sync)
        .unwrap();
    assert_eq!(counters.visual.load(Ordering::Relaxed), 2);
    assert_eq!(counters.simulation.load(Ordering::Relaxed), 2);
    counters.visual.store(u32::MAX, Ordering::Relaxed);
    first
        .try_consume_admitted(&mut dispatcher, &mut random, &mut sync)
        .unwrap();
    assert_eq!(counters.visual.load(Ordering::Relaxed), 0);
    assert_eq!(counters.simulation.load(Ordering::Relaxed), 3);
    second
        .try_queue(&EncodedGameInputCommand {
            receiver_id: 14,
            kind: 1,
            payload: &[1],
        })
        .unwrap();
    second.try_generate_sync_command().unwrap();
    assert!(
        second
            .try_consume_admitted(&mut dispatcher, &mut random, &mut sync)
            .is_err()
    );
    assert_eq!(counters.visual.load(Ordering::Relaxed), 0);
    // Normal counter reset is AFTER the complete command loop, so failure keeps3.
    assert_eq!(counters.simulation.load(Ordering::Relaxed), 3);
}

#[test]
fn retained_device_to_sample_to_queue_to_same_player_dispatcher() {
    use crate::{
        input_cvar_runtime::InputCvarReferences,
        input_pad::device::tests::{Platform, Remap},
        input_pad::device_manager::{
            InputDeviceManager, selected_runtime::SelectedDeviceInputEvents,
        },
        input_pad_cache::InputPadCache,
        player::PlayerLoaderSlot,
        players_backend_settings::PlayersBackendSettings,
        skeleton_collision_numeric::HostSkeletonCollisionNumeric,
    };
    struct UnavailablePause;
    impl PlayerInputPause for UnavailablePause {
        type Error = &'static str;
        fn try_is_simulation_active(&mut self) -> Result<bool, Self::Error> {
            Err("pause not supplied for focused admitted slice")
        }
    }
    let map =
        Arc::new(SelectedInputMap::parse(&crate::input_map::tests::config("Button0")).unwrap());
    let player = Arc::new(Mutex::new(GameInputPlayer::new(4, Arc::clone(&map))));
    let mut dispatcher = GameInputCommandManager::new();
    dispatcher.try_construct_player(&player).unwrap();
    let mut devices = InputDeviceManager::new();
    devices.initialize().unwrap();
    let mut platform = Platform::new();
    for device in devices.devices_mut().unwrap() {
        device
            .reset_actuators(
                &mut platform,
                &InputCvarReferences::new_static_initialized(),
                &mut HostReckoningNumeric,
            )
            .unwrap();
    }
    devices.set_external_remap(Some(&42));
    let mut input = SelectedDeviceInputRuntime::from_owners(
        devices,
        InputPadCache::new(),
        InputPadCache::new(),
        Arc::clone(&map),
        HostSkeletonCollisionNumeric,
        Remap,
    );
    input.poll_hardware(&mut platform).unwrap();
    assert!(Arc::ptr_eq(input.map(), &map));
    let settings = PlayersBackendSettings::construct_fields();
    settings.try_set_controller_index(4, 2).unwrap();
    let loader = PlayerLoaderSlot::new(4);
    let selection = BackendControllerSelection::try_capture(&settings, &loader).unwrap();
    settings.try_set_controller_index(4, 3).unwrap();
    assert_eq!(selection.device_index(), 2);
    let info = PhysicalGameInfo::construct_network_field();
    let mut queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    input
        .try_sample_and_queue_player(
            SelectedCommandSampleRequest {
                selection: &selection,
                game_info: &info,
                has_focus: true,
            },
            &mut UnavailablePause,
            &mut queue,
            &mut HostReckoningNumeric,
        )
        .unwrap();
    assert_eq!(player.lock().unwrap().pad().unwrap().num_buttons(), 0);
    assert!(queue.try_generate_sync_command().unwrap());
    let mut random =
        SimulationCommandRandom::construct_field(NativeRandomGate::from_static_initializer());
    let result = queue
        .try_consume_admitted(&mut dispatcher, &mut random, &mut Arc::clone(&info))
        .unwrap();
    assert_eq!(result.delivered, 1);
    assert!(player.lock().unwrap().has_focus().unwrap());
    assert_eq!(player.lock().unwrap().pad().unwrap().num_buttons(), 24);
    assert_eq!(random.random.index, 1);
    assert_eq!(dispatcher.crc(), 0); // Same actual GameInfo ctor sync0, sampled live.
}

#[test]
fn retained_frame_session_recognizes_delivered_commands_without_repacking_or_redispatch() {
    use crate::{
        attribute_runtime,
        debug_overlay::DebugOverlaySlot,
        game_input_frame_session::GameInputFrameSession,
        game_input_recognizer_runtime_services::GameInputRuntimeServices,
        gameplay_input_update_manager::GameplayInputUpdateManager,
        input_cvar_runtime::{InputCvarManager, InputCvarReferences},
    };
    let mut text = crate::input_map::tests::config("0.0f");
    for name in [
        "DebugPrev",
        "DebugNext",
        "DebugPause",
        "DebugReplaceSkateboard",
        "DebugResetWorld",
    ] {
        writeln!(text, "{name} = 0.0f;").unwrap();
    }
    let map = Arc::new(SelectedInputMap::parse_with_input_events(&text).unwrap());
    let vault = Arc::new(attribute_runtime::tests::graph(
        0x7f08_6875_d621_c88c,
        vec![],
    ));
    let mut frame = GameInputFrameSession {
        commands: GameInputCommandManager::new(),
        gameplay: GameplayInputUpdateManager::<()>::new(),
    };
    frame
        .gameplay
        .try_construct(|_| {
            Ok::<_, std::convert::Infallible>(b"pattern Flick coord 0 1 coord 0 -1".to_vec())
        })
        .unwrap();
    frame
        .gameplay
        .try_reset(&map, &vault, &mut frame.commands)
        .unwrap();
    let same_player = Arc::clone(frame.gameplay.player(4).unwrap());
    let mut queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    queue
        .try_queue(&EncodedGameInputCommand {
            receiver_id: 14,
            kind: 1,
            payload: &[],
        })
        .unwrap();
    queue.try_generate_sync_command().unwrap();
    let mut random =
        SimulationCommandRandom::construct_field(NativeRandomGate::from_static_initializer());
    let mut sync = Sync {
        value: Ok(2),
        calls: 0,
    };
    let delivery = frame
        .try_deliver_queued_commands(&mut queue, &mut random, &mut sync)
        .unwrap();
    let crc = frame.commands.crc();
    assert_eq!(sync.calls, 1);
    let refs = InputCvarReferences::new_static_initialized();
    let mut cvars = InputCvarManager::new();
    cvars.try_link(&refs).unwrap();
    let overlay = DebugOverlaySlot::default();
    overlay.try_construct().unwrap();
    let mut recognizer = GameInputRuntimeServices::new(&refs, &vault, &overlay, None);
    frame
        .try_recognize_delivered(delivery.update_mask, &mut recognizer, &overlay)
        .unwrap();
    assert_eq!(frame.commands.crc(), crc);
    assert_eq!(sync.calls, 1);
    assert_eq!(random.random.index, 1);
    assert!(Arc::ptr_eq(frame.gameplay.player(4).unwrap(), &same_player));
    assert_eq!(same_player.lock().unwrap().pad().unwrap().num_buttons(), 24);
}
