use super::*;
use crate::{
    action_graph_input_listener::tests::{gameplay, matched, pattern},
    motion_graph_entity_authority::ActorGraphAuthority,
};

#[test]
fn native_debug_producer_uses_same_pad_pressed_and_preserves_pattern_event_order() {
    use crate::{
        action_graph_input_listener::ActionInputIntentSink,
        game_input_manager::GameInputCommandManager,
        game_input_player::{GameInputCommand, GameInputPlayer, PlayerInputData},
        input_map::SelectedInputMap,
        input_pad_cache::PublishedInputPadSample,
    };
    struct Sink(Vec<String>);
    impl ActionInputIntentSink for Sink {
        type Error = std::convert::Infallible;
        fn insert(&mut self, name: &str, _value: f32) -> Result<(), Self::Error> {
            self.0.push(name.to_owned());
            Ok(())
        }
    }
    let text = format!(
        "{} DebugPrev=Button0.pressed; DebugNext=Button1.pressed; DebugPause=Button2.pressed; DebugReplaceSkateboard=Button3.pressed; DebugResetWorld=Button4.pressed;",
        crate::input_map::tests::config("0.0")
    );
    let player = Arc::new(Mutex::new(GameInputPlayer::new(
        2,
        Arc::new(SelectedInputMap::parse_with_input_events(&text).unwrap()),
    )));
    let mut manager = GameInputCommandManager::new();
    let receiver = manager.try_construct_player(&player).unwrap();
    let authority = ActorGraphAuthority::new("same input-event owner");
    let listener = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&authority, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    let _registration = route.try_construct_listener(&listener).unwrap();
    let publish = |values| {
        GameInputCommand::Input(PlayerInputData {
            sample: PublishedInputPadSample { count: 24, values },
            allow_sensor: false,
            has_focus: true,
        })
    };
    let mut player = player.lock().unwrap();
    // Native tButton starts transition_counter0 and suppresses transitions
    // for its first three updates. Admit those actual pad updates before rise.
    for _ in 0..3 {
        player
            .try_on_command(&receiver, publish([0.0; 24]))
            .unwrap();
        player
            .try_generate_input_events(route.broadcaster_mut())
            .unwrap();
        assert_eq!(route.pending(), 0);
    }
    let mut values = [0.0; 24];
    values[..5].fill(1.0);
    player.try_on_command(&receiver, publish(values)).unwrap();
    route
        .broadcaster_mut()
        .enqueue(matched(&receiver, &pattern("Ollie"), 2))
        .unwrap();
    player
        .try_generate_input_events(route.broadcaster_mut())
        .unwrap();
    route
        .broadcaster_mut()
        .enqueue(matched(&receiver, &pattern("Kickflip"), 2))
        .unwrap();
    assert_eq!(route.pending(), 7);
    for (_, message) in route.broadcaster_mut().messages() {
        if let GameInputPatternMessage::InputEvent { input, .. } = message {
            assert!(Arc::ptr_eq(&input.occurrence, &receiver.occurrence));
        }
    }
    assert_eq!(route.try_flush().unwrap(), 7);
    let mut sink = Sink(vec![]);
    listener
        .lock()
        .unwrap()
        .try_pattern_intent_suffix(&mut sink)
        .unwrap();
    assert_eq!(
        sink.0,
        [
            "Trick",
            "Ollie",
            "GestureSpeed",
            "DebugPrev",
            "DebugNext",
            "DebugPause",
            "DebugReplaceSkateboard",
            "DebugResetWorld",
            "Trick",
            "Kickflip",
            "GestureSpeed"
        ]
    );
    player.try_on_command(&receiver, publish(values)).unwrap();
    player
        .try_generate_input_events(route.broadcaster_mut())
        .unwrap();
    assert_eq!(route.pending(), 0);
}

#[test]
fn input_event_wrong_player_is_ignored_and_late_map_failure_keeps_earlier_queue() {
    use crate::{
        game_input_player::recognizer::{GameInputEvent, GameInputEventError},
        input_map::SelectedInputMap,
    };
    let (player, receiver, _manager) = gameplay();
    let authority = ActorGraphAuthority::new("same queue");
    let listener = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&authority, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    let _registration = route.try_construct_listener(&listener).unwrap();
    route
        .broadcaster_mut()
        .enqueue(GameInputPatternMessage::InputEvent {
            input: receiver.clone(),
            event: GameInputEvent::DebugPrev,
            player_index: 99,
        })
        .unwrap();
    route.try_flush().unwrap();
    assert!(listener.lock().unwrap().events().is_empty());
    let text = format!(
        "{} DebugPrev=1.0; DebugNext=Button100.pressed; DebugPause=0.0; DebugReplaceSkateboard=0.0; DebugResetWorld=0.0;",
        crate::input_map::tests::config("0.0")
    );
    let mut player = player.lock().unwrap();
    player.map = Arc::new(SelectedInputMap::parse_with_input_events(&text).unwrap());
    // Retained constructed-empty pad returns false for all buttons: source
    // range checking only begins once a nonempty actual sample is admitted.
    let command = crate::game_input_player::GameInputCommand::Input(
        crate::game_input_player::PlayerInputData {
            sample: crate::input_pad_cache::PublishedInputPadSample {
                count: 24,
                values: [0.0; 24],
            },
            allow_sensor: false,
            has_focus: true,
        },
    );
    player.try_on_command(&receiver, command).unwrap();
    assert!(matches!(
        player.try_generate_input_events(route.broadcaster_mut()),
        Err(GameInputEventError::Player(_))
    ));
    assert_eq!(route.pending(), 1);
    route.try_flush().unwrap();
    assert_eq!(listener.lock().unwrap().events().len(), 1);
}

#[test]
fn same_queue_routes_port_kind_id_and_retains_actual_listener_events() {
    let actor = ActorGraphAuthority::new("diagnostic logical Actor");
    let (player, receiver, _manager) = gameplay();
    let listener = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&actor, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    let registration = route.try_construct_listener(&listener).unwrap();
    assert!(listener.lock().unwrap().is_constructed());
    assert_eq!(route.handlers.len(), 3);
    assert_eq!(route.handlers[0].kind, Kind::InputEvent);
    let original = pattern("Ollie");
    route
        .broadcaster_mut()
        .enqueue(matched(&receiver, &original, 2))
        .unwrap();
    route.broadcaster.messages[0].0 ^= 1;
    assert_eq!(route.try_flush().unwrap(), 1);
    assert!(listener.lock().unwrap().events().is_empty());
    route.handlers[1].id = 4;
    route
        .broadcaster_mut()
        .enqueue(matched(&receiver, &original, 2))
        .unwrap();
    assert_eq!(route.try_flush().unwrap(), 1);
    assert!(listener.lock().unwrap().events().is_empty());
    route.handlers[1].no_filter = true;
    route
        .broadcaster_mut()
        .enqueue(matched(&receiver, &original, 2))
        .unwrap();
    assert_eq!(route.pending(), 1);
    assert_eq!(route.try_flush().unwrap(), 1);
    assert!(Arc::ptr_eq(
        &listener.lock().unwrap().events()[0]
            .pattern()
            .unwrap()
            .pattern,
        &original
    ));
    assert_eq!(route.pending(), 0);
    route.try_unregister(&registration).unwrap();
    route
        .broadcaster_mut()
        .enqueue(matched(&receiver, &original, 2))
        .unwrap();
    assert_eq!(route.try_flush().unwrap(), 1);
    assert_eq!(listener.lock().unwrap().events().len(), 1);
}
#[test]
fn failed_late_listener_retains_message_and_does_not_repeat_delivered_prefix() {
    let actor = ActorGraphAuthority::new(1);
    let (player, receiver, _manager) = gameplay();
    let first = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&actor, &player).unwrap(),
    ));
    let second = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&actor, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    let registration = route.try_construct_listener(&first).unwrap();
    let _second_registration = route.try_construct_listener(&second).unwrap();
    let _caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = second.lock().unwrap();
        panic!("diagnostic late listener lock refusal");
    }));
    route
        .broadcaster_mut()
        .enqueue(matched(&receiver, &pattern("Ollie"), 2))
        .unwrap();
    assert_eq!(route.try_flush(), Err(GameInputRouteError::ListenerLock));
    assert_eq!(first.lock().unwrap().events().len(), 1);
    assert_eq!(route.pending(), 1);
    assert_eq!(
        route.try_unregister(&registration),
        Err(GameInputRouteError::DeliveryInProgress)
    );
    second.clear_poison();
    assert_eq!(route.try_flush().unwrap(), 1);
    assert_eq!(first.lock().unwrap().events().len(), 1);
    assert_eq!(second.lock().unwrap().events().len(), 1);
}
#[test]
fn foreign_unregister_and_constructor_retry_are_refused() {
    let actor = ActorGraphAuthority::new(1);
    let (player, _receiver, _manager) = gameplay();
    let listener = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&actor, &player).unwrap(),
    ));
    let mut first = GameInputEventRoute::new();
    let registration = first.try_construct_listener(&listener).unwrap();
    let mut second = GameInputEventRoute::new();
    assert_eq!(
        second.try_unregister(&registration),
        Err(GameInputRouteError::ForeignRegistration)
    );
    assert!(matches!(
        second.try_construct_listener(&listener),
        Err(GameInputRouteError::Listener(
            ActionInputListenerError::ConstructionAlreadyAttempted
        ))
    ));
    assert!(second.handlers.is_empty());
}

#[test]
fn late_construction_failure_retains_teardown_capability_on_same_owner() {
    let actor = ActorGraphAuthority::new(1);
    let (player, _receiver, _manager) = gameplay();
    let listener = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&actor, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    route.fail_registration = Some(2);
    assert!(matches!(
        route.try_construct_listener(&listener),
        Err(GameInputRouteError::Allocation)
    ));
    assert_eq!(route.handlers.len(), 2);
    let partial = {
        let state = listener.lock().unwrap();
        assert!(!state.is_constructed());
        state.registration().unwrap().clone()
    };
    assert_eq!(partial.keys.len(), 2);
    route.try_unregister(&partial).unwrap();
    assert!(route.handlers.is_empty());
    assert!(matches!(
        route.try_construct_listener(&listener),
        Err(GameInputRouteError::Listener(
            ActionInputListenerError::ConstructionAlreadyAttempted
        ))
    ));
}

#[test]
fn expired_callback_can_be_explicitly_unregistered_without_dropping_current_message() {
    let actor = ActorGraphAuthority::new(1);
    let (player, receiver, _manager) = gameplay();
    let first = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&actor, &player).unwrap(),
    ));
    let expired = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&actor, &player).unwrap(),
    ));
    let last = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&actor, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    let _first_registration = route.try_construct_listener(&first).unwrap();
    let expired_registration = route.try_construct_listener(&expired).unwrap();
    let _last_registration = route.try_construct_listener(&last).unwrap();
    drop(expired);
    route
        .broadcaster_mut()
        .enqueue(matched(&receiver, &pattern("Ollie"), 2))
        .unwrap();
    assert_eq!(route.try_flush(), Err(GameInputRouteError::ExpiredListener));
    assert_eq!(first.lock().unwrap().events().len(), 1);
    route
        .try_unregister_failed_delivery(&expired_registration)
        .unwrap();
    assert_eq!(route.pending(), 1);
    assert_eq!(route.try_flush().unwrap(), 1);
    assert_eq!(first.lock().unwrap().events().len(), 1);
    assert_eq!(last.lock().unwrap().events().len(), 1);
}
