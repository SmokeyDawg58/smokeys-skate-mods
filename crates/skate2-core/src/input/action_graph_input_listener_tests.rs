use super::*;
use crate::{
    game_input_manager::GameInputCommandManager,
    game_input_player::{GameInputReceiver, recognizer::event_route::GameInputEventRoute},
    input_map::SelectedInputMap,
    input_pattern::InputPatterns,
    input_pattern_matcher::InputPatternMatch,
};

pub(crate) fn gameplay() -> (
    Arc<Mutex<GameInputPlayer>>,
    GameInputReceiver,
    GameInputCommandManager,
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
    let cfg = names.map(|name| format!("{name} = 0.0f;\n")).concat();
    let player = Arc::new(Mutex::new(GameInputPlayer::new(
        2,
        Arc::new(SelectedInputMap::parse(&cfg).unwrap()),
    )));
    let mut manager = GameInputCommandManager::new();
    let receiver = manager.try_construct_player(&player).unwrap();
    (player, receiver, manager)
}
pub(crate) fn pattern(name: &str) -> Arc<InputPattern> {
    Arc::clone(
        &InputPatterns::parse(format!("pattern {name} coord 0 1 coord 0 -1").as_bytes())
            .unwrap()
            .patterns()[0],
    )
}
pub(crate) fn matched(
    receiver: &GameInputReceiver,
    pattern: &Arc<InputPattern>,
    index: u32,
) -> GameInputPatternMessage {
    GameInputPatternMessage::Match {
        input: receiver.clone(),
        matched: InputPatternMatch {
            pattern: Some(Arc::clone(pattern)),
            normalized_speed: 0.25,
            error: 8.0,
            time: 9.0,
        },
        player_index: index,
    }
}
#[derive(Default)]
struct Sink {
    writes: Vec<(String, u32)>,
    fail: Option<usize>,
}
impl ActionInputIntentSink for Sink {
    type Error = &'static str;
    fn insert(&mut self, name: &str, value: f32) -> Result<(), Self::Error> {
        if self.fail == Some(self.writes.len()) {
            return Err("insert");
        }
        self.writes.push((name.to_owned(), value.to_bits()));
        Ok(())
    }
}
#[test]
fn ollie_is_event_but_only_flip_names_arm_held_latch() {
    let authority = ActorGraphAuthority::new("diagnostic existing graph owner");
    let (player, receiver, _manager) = gameplay();
    let listener = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&authority, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    let _registration = route.try_construct_listener(&listener).unwrap();
    let mut state = listener.lock().unwrap();
    assert!(state.actor().is_same_issuance(&authority.entity_handle()));
    assert!(Arc::ptr_eq(state.player(), &player));
    let ollie = pattern("Ollie");
    state.receive(&matched(&receiver, &ollie, 9)).unwrap();
    assert!(state.events().is_empty());
    state.receive(&matched(&receiver, &ollie, 2)).unwrap();
    state
        .receive(&GameInputPatternMessage::Held {
            pattern: Arc::clone(&ollie),
            player_index: 2,
        })
        .unwrap();
    assert!(!state.held_pending());
    let flip = pattern("Kickflip");
    state.receive(&matched(&receiver, &flip, 2)).unwrap();
    state
        .receive(&GameInputPatternMessage::Held {
            pattern: pattern("kICKFLIP"),
            player_index: 2,
        })
        .unwrap();
    assert!(state.held_pending());
    state
        .receive(&GameInputPatternMessage::Held {
            pattern: ollie,
            player_index: 2,
        })
        .unwrap();
    assert!(
        state.held_pending(),
        "unrelated held does not clear an earlier latch"
    );
    let mut sink = Sink::default();
    state.try_pattern_intent_suffix(&mut sink).unwrap();
    assert_eq!(
        sink.writes
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        [
            "HoldPattern",
            "Trick",
            "Ollie",
            "GestureSpeed",
            "Trick",
            "Kickflip",
            "GestureSpeed"
        ]
    );
    assert_eq!(sink.writes[3].1, 0.25_f32.to_bits());
    assert_eq!(
        state.events().len(),
        2,
        "Fill suffix does not clear Actor events"
    );
    assert!(!state.held_pending());
    state.clear_events_after_generate();
    let mut released = Sink::default();
    state.try_pattern_intent_suffix(&mut released).unwrap();
    assert!(released.writes.is_empty());
}
#[test]
fn failed_hold_insert_retains_latch_and_actor_clear_does_not_consume_it() {
    let authority = ActorGraphAuthority::new(1);
    let (player, receiver, _manager) = gameplay();
    let listener = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&authority, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    let _registration = route.try_construct_listener(&listener).unwrap();
    let mut state = listener.lock().unwrap();
    let flip = pattern("N_Heelflip");
    state.receive(&matched(&receiver, &flip, 2)).unwrap();
    state
        .receive(&GameInputPatternMessage::Held {
            pattern: flip,
            player_index: 2,
        })
        .unwrap();
    state.clear_events_after_generate();
    let mut sink = Sink {
        fail: Some(0),
        ..Sink::default()
    };
    assert_eq!(
        state.try_pattern_intent_suffix(&mut sink),
        Err(ActionInputSuffixError::Insert("insert"))
    );
    assert!(state.held_pending());
    sink.fail = None;
    state.try_pattern_intent_suffix(&mut sink).unwrap();
    assert_eq!(sink.writes.len(), 1);
    assert!(!state.held_pending());
}

#[test]
fn same_listener_constructs_derived_before_registration_and_updates_command_owned_player() {
    use crate::{attribute_runtime::tests::graph, reckoning_numeric::HostReckoningNumeric};
    let authority = ActorGraphAuthority::new(1);
    let (player, _receiver, _manager) = gameplay();
    let listener = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&authority, &player).unwrap(),
    ));
    assert!(listener.lock().unwrap().derived().is_none());
    let mut route = GameInputEventRoute::new();
    let registration = route.try_construct_listener(&listener).unwrap();
    let attributes = DerivedInputAttributes::new(&Arc::new(graph(INPUT_LISTENER_CLASS, vec![])));
    let mut state = listener.lock().unwrap();
    assert!(Arc::ptr_eq(state.player(), &player));
    assert_eq!(
        state.derived().unwrap().stick_edge_age.map(f32::to_bits),
        [0x7eff_ffff; 4]
    );
    state
        .try_update_derived(0.25, 501, Some(&attributes), &mut HostReckoningNumeric)
        .unwrap();
    let derived = state.derived().unwrap();
    assert_eq!(
        derived.current,
        crate::controller_input::RawControllerInput::default()
    );
    assert_eq!(derived.face_held.map(f32::to_bits), [0; 4]);
    assert_eq!(derived.magnitude_held.map(f32::to_bits), [0; 2]);
    assert!(state.events().is_empty());
    drop(state);
    route.try_unregister(&registration).unwrap();
}
