use super::*;

#[test]
fn logical_listener_refuses_unconstructed_gameplay_storage() {
    let actor = crate::motion_graph_entity_authority::ActorGraphAuthority::new("diagnostic graph");
    let player = Arc::new(std::sync::Mutex::new(GameInputPlayer::new(0, map())));
    assert!(matches!(crate::action_graph_input_listener::ActionGraphInputListener::new(&actor,&player),
        Err(crate::action_graph_input_listener::ActionInputListenerError::GameplayConstructionRequired)));
}
use std::fmt::Write;
fn map() -> Arc<SelectedInputMap> {
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
        writeln!(text, "{name} = Button12;").unwrap();
    }
    Arc::new(SelectedInputMap::parse(&text).unwrap())
}
struct Registration {
    fail: bool,
    receivers: Vec<GameInputReceiver>,
}
impl GameInputReceiverRegistration for Registration {
    type Error = &'static str;
    fn register(&mut self, receiver: &GameInputReceiver) -> Result<(), Self::Error> {
        self.receivers.push(receiver.clone());
        if self.fail {
            Err("registration")
        } else {
            Ok(())
        }
    }
}
fn player() -> (GameInputPlayer, GameInputReceiver) {
    let mut player = GameInputPlayer::new(0, map());
    let receiver = player
        .try_construct(&mut Registration {
            fail: false,
            receivers: vec![],
        })
        .unwrap();
    (player, receiver)
}
fn command(values: [f32; 24], count: i32, focus: bool) -> GameInputCommand {
    GameInputCommand::Input(PlayerInputData {
        sample: PublishedInputPadSample { count, values },
        allow_sensor: false,
        has_focus: focus,
    })
}
#[test]
fn registration_is_required_one_attempt_and_preserves_constructor_prefix() {
    let mut player = GameInputPlayer::new(u32::MAX, map());
    assert_eq!(
        player.try_value(74),
        Err(GameInputPlayerError::ConstructionRequired)
    );
    let mut registration = Registration {
        fail: true,
        receivers: vec![],
    };
    assert!(matches!(
        player.try_construct(&mut registration),
        Err(GameInputConstructionError::Registration("registration"))
    ));
    assert_eq!(registration.receivers[0].id(), 9);
    assert_eq!(player.pad.num_buttons(), 0);
    let fields = player.fields.as_ref().unwrap();
    assert_eq!(fields.globals.map(f32::to_bits), [0; 4]);
    assert_eq!(fields.bounced, [false; 4]);
    assert!(!fields.has_focus);
    assert_eq!(
        player.try_on_command(&registration.receivers[0], GameInputCommand::GainFocus),
        Err(GameInputPlayerError::ConstructionRequired)
    );
    assert!(matches!(
        player.try_construct(&mut registration),
        Err(GameInputConstructionError::State(
            GameInputPlayerError::ConstructionAlreadyAttempted
        ))
    ));
    assert_eq!(registration.receivers.len(), 1);
}
#[test]
fn foreign_equal_id_refuses_before_any_command_write() {
    let (mut player, receiver) = player();
    let (_, foreign) = self::player();
    assert_eq!(receiver.id(), foreign.id());
    assert_eq!(
        player.try_on_command(&foreign, GameInputCommand::GainFocus),
        Err(GameInputPlayerError::ForeignReceiver)
    );
    assert!(!player.has_focus().unwrap());
    assert_eq!(player.pad().unwrap().num_buttons(), 0);
    assert_eq!(player.globals().unwrap().map(f32::to_bits), [0; 4]);
    assert!(player.dpd_position_updated().unwrap());
}
#[test]
fn channels_wait_for_independent_zero_then_pass_signed_zero_and_nan() {
    let (mut player, receiver) = player();
    let mut values = [0.75; 24];
    values[12] = -0.0;
    values[13] = f32::NAN;
    values[14] = 0.0;
    player
        .try_on_command(&receiver, command(values, 0, false))
        .unwrap();
    let pad = player.pad().unwrap();
    assert_eq!(pad.num_buttons(), 24);
    assert_eq!(pad.value(12).unwrap().to_bits(), (-0.0_f32).to_bits());
    assert_eq!(pad.value(13).unwrap().to_bits(), 0);
    assert_eq!(pad.value(15).unwrap().to_bits(), 0);
    assert_eq!(pad.value(16).unwrap().to_bits(), 0.75_f32.to_bits());
    assert!(!player.has_focus().unwrap());
    values[12] = f32::from_bits(0x7fc0_0123);
    values[13] = 0.0;
    values[14] = 1.0;
    player
        .try_on_command(&receiver, command(values, -9, true))
        .unwrap();
    assert_eq!(
        player.pad().unwrap().value(12).unwrap().to_bits(),
        0x7fc0_0123
    );
    assert_eq!(
        player.pad().unwrap().value(14).unwrap().to_bits(),
        1.0_f32.to_bits()
    );
    assert!(player.has_focus().unwrap());
}
#[test]
fn gain_focus_rearms_bounce_without_resetting_pad_or_history() {
    let (mut player, receiver) = player();
    player
        .try_on_command(&receiver, command([0.0; 24], 24, true))
        .unwrap();
    for _ in 0..3 {
        player
            .try_on_command(&receiver, command([1.0; 24], 24, true))
            .unwrap();
    }
    assert!(player.pad().unwrap().held(12).unwrap());
    player
        .try_on_command(&receiver, GameInputCommand::GainFocus)
        .unwrap();
    assert!(player.pad().unwrap().held(12).unwrap());
    assert_eq!(
        player.pad().unwrap().value(12).unwrap().to_bits(),
        1.0_f32.to_bits()
    );
    player
        .try_on_command(&receiver, command([1.0; 24], 24, false))
        .unwrap();
    assert_eq!(player.pad().unwrap().value(12).unwrap().to_bits(), 0);
    assert!(player.pad().unwrap().held(12).unwrap());
}
#[test]
fn mapped_reads_use_same_retained_map_and_command_updated_pad() {
    let (mut player, receiver) = player();
    let map_ptr = Arc::as_ptr(&player.map);
    player
        .try_on_command(&receiver, command([0.0; 24], 0, false))
        .unwrap();
    player
        .try_on_command(&receiver, command([0.625; 24], 900, false))
        .unwrap();
    assert_eq!(
        player
            .try_mapped_value(ControllerControl::LeftStickX)
            .unwrap()
            .to_bits(),
        0.625_f32.to_bits()
    );
    assert_eq!(map_ptr, Arc::as_ptr(&player.map));
    assert_eq!(
        player.try_on_command(&receiver, GameInputCommand::Unknown(9)),
        Err(GameInputPlayerError::UnknownCommand(9))
    );
    assert_eq!(
        player.try_on_command(&receiver, GameInputCommand::Unknown(1)),
        Err(GameInputPlayerError::MissingPayload(1))
    );
    assert_eq!(player.try_value(74).unwrap().to_bits(), 0.625_f32.to_bits());
}
