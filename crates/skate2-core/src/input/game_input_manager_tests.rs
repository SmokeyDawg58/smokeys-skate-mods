use super::*;
use crate::input_map::SelectedInputMap;
use std::fmt::Write;
fn player(index: u32) -> Arc<Mutex<GameInputPlayer>> {
    let names = [
        "GP_LStickX",
        "GP_LStickY",
        "GP_LStickIn",
        "GP_RStickX",
        "GP_RStickY",
        "GP_RStickIn",
        "GP_LTrigger",
        "GP_RTrigger",
        "GP_LBumper",
        "GP_RBumper",
        "GP_UDPad",
        "GP_DDPad",
        "GP_LDPad",
        "GP_RDPad",
        "GP_XFace",
        "GP_YFace",
        "GP_AFace",
        "GP_BFace",
        "GP_LStickRawX",
        "GP_LStickRawY",
        "GP_SensorZ",
        "GP_SensorX",
    ];
    let mut text = String::new();
    for name in names {
        writeln!(text, "{name} = Button13;").unwrap();
    }
    Arc::new(Mutex::new(GameInputPlayer::new(
        index,
        Arc::new(SelectedInputMap::parse(&text).unwrap()),
    )))
}
struct Level {
    value: Result<i32, &'static str>,
    calls: usize,
}
impl GameInputSyncLevel for Level {
    type Error = &'static str;
    fn sync_level(&mut self) -> Result<i32, Self::Error> {
        self.calls += 1;
        self.value
    }
}
fn level() -> Level {
    Level {
        value: Ok(2),
        calls: 0,
    }
}
#[test]
fn crc_matches_original_table_oracles_and_zero_special_case() {
    assert_eq!(game_input_crc(&[1, 2, 3]), 0);
    assert_eq!(game_input_crc(&[0; 4]), u32::MAX);
    assert_eq!(game_input_crc(&[1, 2, 3, 4]), 0x0102_0304);
    assert_eq!(game_input_crc(&[1, 2, 3, 4, 5]), 0xb735_59f9);
    assert_eq!(game_input_crc(&[0, 1, 2, 3, 4, 5, 6, 7, 8]), 0x03da_a9dd);
    assert_eq!(game_input_crc(&[255; 9]), 0xb1f7_40b4);
}
#[test]
fn private_registration_binds_actual_player_and_duplicate_does_not_replace() {
    let mut manager = GameInputCommandManager::new();
    let first = player(0);
    let token = manager.try_construct_player(&first).unwrap();
    let second = player(0);
    assert!(matches!(
        manager.try_construct_player(&second),
        Err(GameInputManagerError::Construction(
            GameInputConstructionError::Registration(
                GameInputRegistrationError::DuplicateReceiver(10)
            )
        ))
    ));
    assert_eq!(
        manager
            .try_do_command(
                &EncodedGameInputCommand {
                    receiver_id: token.id(),
                    kind: 2,
                    payload: &[]
                },
                false,
                &mut level()
            )
            .unwrap(),
        GameInputDispatch::Delivered
    );
    assert!(first.lock().unwrap().has_focus().unwrap());
    assert!(second.lock().unwrap().has_focus().is_err());
}
#[test]
fn codec_dispatch_updates_same_pad_and_crc_before_reset_keeps_membership() {
    let mut manager = GameInputCommandManager::new();
    let player = player(0);
    manager.try_construct_player(&player).unwrap();
    let mut source = level();
    let command = EncodedGameInputCommand {
        receiver_id: 10,
        kind: 1,
        payload: &[0],
    };
    assert_eq!(
        manager.try_do_command(&command, true, &mut source).unwrap(),
        GameInputDispatch::Delivered
    );
    assert_eq!(manager.crc(), 360_491);
    assert_eq!(source.calls, 1);
    assert_eq!(player.lock().unwrap().pad().unwrap().num_buttons(), 24);
    manager.clear_selected_crc();
    assert_eq!(manager.crc(), 0);
    // Empty focus decodes Button13=1; prior focusedzero unlocked its gate.
    manager
        .try_do_command(
            &EncodedGameInputCommand {
                receiver_id: 10,
                kind: 1,
                payload: &[],
            },
            false,
            &mut source,
        )
        .unwrap();
    assert_eq!(source.calls, 1);
    let mut player = player.lock().unwrap();
    assert!(!player.has_focus().unwrap());
    assert_eq!(player.try_value(74).unwrap().to_bits(), 1.0_f32.to_bits());
}
#[test]
fn crc_commits_before_decode_failure_and_missing_id_skips_decode() {
    let mut manager = GameInputCommandManager::new();
    let player = player(0);
    manager.try_construct_player(&player).unwrap();
    assert!(matches!(
        manager.try_do_command(
            &EncodedGameInputCommand {
                receiver_id: 10,
                kind: 1,
                payload: &[1]
            },
            true,
            &mut level()
        ),
        Err(GameInputManagerError::Decode(_))
    ));
    assert_eq!(manager.crc(), 360_491);
    assert_eq!(player.lock().unwrap().pad().unwrap().num_buttons(), 0);
    assert_eq!(
        manager
            .try_do_command(
                &EncodedGameInputCommand {
                    receiver_id: 99,
                    kind: 1,
                    payload: &[255]
                },
                false,
                &mut level()
            )
            .unwrap(),
        GameInputDispatch::MissingReceiver
    );
}
#[test]
fn conditional_global_read_precedes_lookup_and_expired_is_not_missing() {
    let mut manager = GameInputCommandManager::new();
    let command = EncodedGameInputCommand {
        receiver_id: 10,
        kind: 2,
        payload: &[],
    };
    let mut source = Level {
        value: Err("global"),
        calls: 0,
    };
    assert_eq!(
        manager
            .try_do_command(&command, false, &mut source)
            .unwrap(),
        GameInputDispatch::MissingReceiver
    );
    assert_eq!(source.calls, 0);
    assert!(matches!(
        manager.try_do_command(&command, true, &mut source),
        Err(GameInputManagerError::Source("global"))
    ));
    assert_eq!(manager.crc(), 0);
    let player = player(0);
    manager.try_construct_player(&player).unwrap();
    drop(player);
    assert!(matches!(
        manager.try_do_command(&command, false, &mut source),
        Err(GameInputManagerError::PlayerExpired(10))
    ));
}
#[test]
fn explicit_unregister_asserts_missing_key_and_low_sync_level_skips_crc() {
    let mut manager = GameInputCommandManager::new();
    let player = player(0);
    let receiver = manager.try_construct_player(&player).unwrap();
    let mut source = Level {
        value: Ok(-1),
        calls: 0,
    };
    let command = EncodedGameInputCommand {
        receiver_id: 10,
        kind: 2,
        payload: &[255],
    };
    manager.try_do_command(&command, true, &mut source).unwrap();
    assert_eq!(manager.crc(), 0);
    manager.try_unregister(&receiver).unwrap();
    assert!(matches!(
        manager.try_unregister(&receiver),
        Err(GameInputManagerError::MissingRegistration(10))
    ));
    assert_eq!(
        manager
            .try_do_command(&command, false, &mut source)
            .unwrap(),
        GameInputDispatch::MissingReceiver
    );
}
