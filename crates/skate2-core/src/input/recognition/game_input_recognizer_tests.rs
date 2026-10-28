use super::*;
use crate::{
    game_input_manager::GameInputCommandManager,
    game_input_player::{GameInputCommand, PlayerInputData},
    input_pad_cache::PublishedInputPadSample,
    input_pattern_matcher::InputPatternCvar,
};
use std::{fmt::Write, sync::Mutex};
#[derive(Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent diagnostic source gates and fault injection"
)]
struct Services {
    events: Vec<&'static str>,
    debug: bool,
    input_debug: bool,
    fail_zone: bool,
    fail_overlay: bool,
    refs_ready: bool,
    ref_captures: usize,
    lookups: Vec<(u64, usize)>,
}
impl InputPatternCvars for Services {
    type Target = InputPatternCvar;
    type Error = &'static str;
    fn capture(&mut self, cvar: InputPatternCvar) -> Result<Self::Target, Self::Error> {
        Ok(cvar)
    }
    fn value(&mut self, cvar: &Self::Target) -> Result<f32, Self::Error> {
        Ok(match cvar {
            InputPatternCvar::MinError | InputPatternCvar::MaxError => 0.15,
            InputPatternCvar::FastestRatio => 1.75,
            InputPatternCvar::SlowestRatio => 4.4,
        })
    }
}
impl GameInputRecognizerServices for Services {
    type DeadzoneTarget = ();
    type DisplayTarget = InputPatternStick;
    type DebugInstance = InputPatternStick;
    fn capture_deadzone(&mut self) -> Result<(), Self::Error> {
        self.events.push("zone capture");
        Ok(())
    }
    fn deadzone(&mut self, (): &()) -> Result<[f32; 2], Self::Error> {
        self.events.push("zone value");
        if self.fail_zone {
            Err("zone")
        } else {
            Ok([0.1, 0.1])
        }
    }
    fn ensure_gesture_display_references(&mut self) -> Result<(), Self::Error> {
        if !self.refs_ready {
            self.refs_ready = true;
            self.ref_captures += 2;
        }
        self.events.push("ensure refs");
        Ok(())
    }
    fn gesture_display_value(&mut self, _: InputPatternStick) -> Result<bool, Self::Error> {
        self.events.push("gesture display");
        Ok(self.debug)
    }
    fn local_player_index(&mut self) -> Result<u32, Self::Error> {
        self.events.push("local");
        Ok(0)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.events.push("overlay");
        if self.fail_overlay {
            Err("overlay")
        } else {
            Ok(())
        }
    }
    fn capture_debug_instance(
        &mut self,
        stick: InputPatternStick,
    ) -> Result<Self::DebugInstance, Self::Error> {
        self.events.push("instance");
        Ok(stick)
    }
    fn debug_lookup(
        &mut self,
        _: &Self::DebugInstance,
        key: u64,
        size: usize,
    ) -> Result<(), Self::Error> {
        self.lookups.push((key, size));
        Ok(())
    }
    fn release_debug_instance(&mut self, _: Self::DebugInstance) -> Result<(), Self::Error> {
        self.events.push("release");
        Ok(())
    }
    fn capture_input_display(
        &mut self,
        stick: InputPatternStick,
    ) -> Result<Self::DisplayTarget, Self::Error> {
        self.events.push(if stick == InputPatternStick::Right {
            "input right"
        } else {
            "input left"
        });
        Ok(stick)
    }
    fn input_display_value(&mut self, _: &Self::DisplayTarget) -> Result<bool, Self::Error> {
        Ok(self.input_debug)
    }
}
fn player() -> (
    Arc<Mutex<GameInputPlayer>>,
    GameInputReceiver,
    GameInputCommandManager,
    [Arc<InputPatterns>; 7],
) {
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
    let mut cfg = String::new();
    for name in names {
        let expression = match name {
            "GP_LStickX" => "Button16 - Button17",
            "GP_LStickY" => "Button18 - Button19",
            "GP_RStickX" => "Button20 - Button21",
            "GP_RStickY" => "Button22 - Button23",
            _ => "0.0f",
        };
        writeln!(cfg, "{name} = {expression};").unwrap();
    }
    let player = Arc::new(Mutex::new(GameInputPlayer::new(
        0,
        Arc::new(SelectedInputMap::parse(&cfg).unwrap()),
    )));
    let mut manager = GameInputCommandManager::new();
    let token = manager.try_construct_player(&player).unwrap();
    let vault = Arc::new(crate::attribute_runtime::tests::graph(
        0x7f08_6875_d621_c88c,
        vec![],
    ));
    let patterns = std::array::from_fn(|_| {
        InputPatterns::parse(b"pattern Flick tolerance_dist .1 coord 0 1 coord 0 -1").unwrap()
    });
    player
        .lock()
        .unwrap()
        .try_initialize_recognizer(&vault, &patterns)
        .unwrap();
    (player, token, manager, patterns)
}
fn y(player: &mut GameInputPlayer, token: &GameInputReceiver, value: f32) {
    let mut values = [0.0; 24];
    if value > 0.0 {
        values[18] = value;
        values[22] = value;
    } else {
        values[19] = -value;
        values[23] = -value;
    }
    player
        .try_on_command(
            token,
            GameInputCommand::Input(PlayerInputData {
                sample: PublishedInputPadSample { count: 24, values },
                allow_sensor: false,
                has_focus: true,
            }),
        )
        .unwrap();
}
fn candidates(
    player: &mut GameInputPlayer,
    token: &GameInputReceiver,
    s: &mut Services,
    b: &mut GameInputPatternBroadcaster,
) {
    y(player, token, -1.0);
    player.try_update_recognizer(s, b).unwrap();
    player.try_update_recognizer(s, b).unwrap();
}
#[test]
fn same_gameplay_owner_feeds_both_histories_and_retains_ordered_real_patterns() {
    let (owner, token, _manager, patterns) = player();
    let mut player = owner.lock().unwrap();
    let mut s = Services::default();
    let mut b = GameInputPatternBroadcaster::new(0x1234, 6);
    candidates(&mut player, &token, &mut s, &mut b);
    assert!(b.messages().is_empty());
    y(&mut player, &token, 1.0);
    player.try_update_recognizer(&mut s, &mut b).unwrap();
    assert_eq!(b.messages().len(), 7);
    assert_eq!(s.ref_captures, 2);
    for ((port, message), original) in b.messages().iter().zip(patterns.iter()) {
        assert_eq!(*port, 0x1234);
        let GameInputPatternMessage::Match {
            input,
            matched,
            player_index,
        } = message
        else {
            panic!("source match pass");
        };
        assert_eq!(*player_index, 0);
        assert_eq!(input.id(), token.id());
        assert!(Arc::ptr_eq(&input.occurrence, &token.occurrence));
        assert!(Arc::ptr_eq(
            matched.pattern.as_ref().unwrap(),
            &original.patterns()[0]
        ));
        assert_eq!(matched.pattern.as_ref().unwrap().name(), "Flick");
    }
    player.try_update_recognizer(&mut s, &mut b).unwrap();
    assert_eq!(b.messages().len(), 14);
    for (_, message) in &b.messages()[7..] {
        let GameInputPatternMessage::Held {
            pattern,
            player_index,
        } = message
        else {
            panic!("held pass before cooldown");
        };
        assert_eq!(pattern.name(), "Flick");
        assert_eq!(*player_index, 0);
    }
    assert_eq!(s.ref_captures, 2);
    assert_eq!(
        &s.events[s.events.len() - 2..],
        ["input right", "input left"]
    );
}
#[test]
fn failed_deadzone_read_leaves_both_histories_empty() {
    let (owner, _token, _manager, _patterns) = player();
    let mut player = owner.lock().unwrap();
    let mut s = Services {
        fail_zone: true,
        ..Services::default()
    };
    let mut b = GameInputPatternBroadcaster::new(1, 6);
    assert!(matches!(
        player.try_update_recognizer(&mut s, &mut b),
        Err(GameInputRecognizerError::Source("zone"))
    ));
    let state = player.recognizer.as_ref().unwrap();
    assert_eq!(state.right.history.len(), 0);
    assert_eq!(state.left.history.len(), 0);
    assert_eq!(s.events, ["zone capture", "zone value"]);
}
#[test]
fn enabled_matched_debug_lookups_precede_messages_and_input_display_is_separate() {
    let (owner, token, _manager, _patterns) = player();
    let mut player = owner.lock().unwrap();
    let mut s = Services {
        debug: true,
        input_debug: true,
        ..Services::default()
    };
    let mut b = GameInputPatternBroadcaster::new(1, 6);
    candidates(&mut player, &token, &mut s, &mut b);
    s.lookups.clear();
    y(&mut player, &token, 1.0);
    player.try_update_recognizer(&mut s, &mut b).unwrap();
    assert_eq!(s.lookups.len(), 7 * 11 + 2);
    assert_eq!(&s.lookups[..2], [(0xcaaf_af92_d389_3b92, 4); 2]);
    assert_eq!(&s.lookups[2..6], [(0x06ec_96d7_ee3f_354f, 16); 4]);
    assert_eq!(&s.lookups[6..10], [(0x23ff_57fd_eddc_69ab, 16); 4]);
    assert_eq!(s.lookups[10], (0x0fec_b44c_9679_1635, 4));
    assert_eq!(b.messages().len(), 7);
    assert_eq!(player.recognizer.as_ref().unwrap().right.history.len(), 3);
}
#[test]
fn late_debug_refusal_retains_match_success_and_prevents_left_update() {
    let (owner, token, _manager, _patterns) = player();
    let mut player = owner.lock().unwrap();
    let mut s = Services::default();
    let mut b = GameInputPatternBroadcaster::new(1, 6);
    candidates(&mut player, &token, &mut s, &mut b);
    y(&mut player, &token, 1.0);
    s.debug = true;
    s.fail_overlay = true;
    assert!(matches!(
        player.try_update_recognizer(&mut s, &mut b),
        Err(GameInputRecognizerError::Source("overlay"))
    ));
    assert!(b.messages().is_empty());
    assert_eq!(player.recognizer.as_ref().unwrap().right.history.len(), 3);
    assert_eq!(player.recognizer.as_ref().unwrap().left.history.len(), 2);
    s.debug = false;
    s.fail_overlay = false;
    player.try_update_recognizer(&mut s, &mut b).unwrap();
    assert!(matches!(
        b.messages()[0].1,
        GameInputPatternMessage::Held { .. }
    ));
}
#[test]
fn wrong_queue_fails_only_at_first_enqueue_after_match() {
    let (owner, token, _manager, _patterns) = player();
    let mut player = owner.lock().unwrap();
    let mut s = Services::default();
    let mut b = GameInputPatternBroadcaster::new(1, 9);
    candidates(&mut player, &token, &mut s, &mut b);
    y(&mut player, &token, 1.0);
    assert!(matches!(
        player.try_update_recognizer(&mut s, &mut b),
        Err(GameInputRecognizerError::Queue(9))
    ));
    assert_eq!(player.recognizer.as_ref().unwrap().right.history.len(), 3);
    assert_eq!(player.recognizer.as_ref().unwrap().left.history.len(), 2);
    assert!(b.messages().is_empty());
}
