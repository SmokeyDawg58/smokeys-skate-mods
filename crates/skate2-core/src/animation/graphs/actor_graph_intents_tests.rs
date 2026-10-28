use super::*;

fn key(word: u32) -> FastString36 {
    FastString36 {
        words: [word, 0, 0, 0, 0, 0],
    }
}

#[test]
fn erase_preserves_collision_chain_and_reclaims_full_capacity() {
    let mut map = ActorGraphIntents::try_new::<()>().unwrap();
    for index in 0..64_u32 {
        map.insert_unique(key(index * 61), f32::from_bits(index))
            .unwrap();
    }
    assert!(map.erase(&key(31 * 61)));
    assert!(!map.erase(&key(31 * 61)));
    assert_eq!(
        map.iter()
            .map(|entry| entry.value.to_bits())
            .collect::<Vec<_>>(),
        (0..64_u32)
            .rev()
            .filter(|index| *index != 31)
            .collect::<Vec<_>>()
    );
    map.insert_unique(key(31 * 61), f32::from_bits(31)).unwrap();
    assert_eq!(map.len(), 64);
    assert_eq!(map.iter().next().unwrap().value.to_bits(), 31);
    assert!(map.erase(&key(31 * 61)));
    assert!(map.erase(&key(0)));
    assert!(map.erase(&key(63 * 61)));
    assert_eq!(
        map.iter()
            .map(|entry| entry.value.to_bits())
            .collect::<Vec<_>>(),
        (1..63_u32)
            .rev()
            .filter(|index| *index != 31)
            .collect::<Vec<_>>()
    );
}

#[test]
fn native_name_matches_corpus_alphabet_and_source_noncanonical_byte_arithmetic() {
    for name in [
        "",
        "Ollie",
        "GestureSpeed",
        "HoldPattern",
        "Kickflip",
        "abc_123",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
    ] {
        assert_eq!(
            ActorGraphIntents::native_name(name.as_bytes()),
            FastString36::encode(name).unwrap()
        );
    }
    assert_eq!(
        ActorGraphIntents::native_name(b"Ollie\0ignored"),
        FastString36::encode("Ollie").unwrap()
    );
    assert_eq!(
        ActorGraphIntents::native_name(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789suffix"),
        FastString36::encode("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789").unwrap()
    );
    // Frozen arithmetic from signed-byte Assign824b1a78, 38^5 = 79235168.
    assert_eq!(
        ActorGraphIntents::native_name(&[0x80]).words,
        [0xc583_4660, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        ActorGraphIntents::native_name(&[0xff]).words,
        [0x1d4e_6e00, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        ActorGraphIntents::native_name(b"!").words,
        [0xbde1_8ac0, 0, 0, 0, 0, 0]
    );
    assert_eq!(ActorGraphIntents::native_name(b"/"), key(0));
}

#[test]
fn wrapping_hash_uses_all_words_and_full_key_equality_with_head_chain_iteration() {
    let mut map = ActorGraphIntents::try_new::<()>().unwrap();
    let wrapped = FastString36 {
        words: [u32::MAX, 2, 0, 0, 0, 0],
    };
    for (name, value) in [
        (key(2), 2.0),
        (key(1), 1.0),
        (key(62), 62.0),
        (wrapped, 3.0),
    ] {
        assert!(map.insert_unique(name, value).unwrap());
    }
    assert_eq!(
        map.iter().map(|entry| entry.name).collect::<Vec<_>>(),
        [wrapped, key(62), key(1), key(2)]
    );
    assert_eq!(map.get(&key(1)).map(f32::to_bits), Some(1.0_f32.to_bits()));
    assert_eq!(map.get(&wrapped).map(f32::to_bits), Some(3.0_f32.to_bits()));
    assert!(map.get(&key(123)).is_none());
    assert!(!map.insert_unique(key(62), -99.0).unwrap());
    assert_eq!(
        map.get(&key(62)).map(f32::to_bits),
        Some(62.0_f32.to_bits())
    );
    assert_eq!(
        map.iter().map(|entry| entry.name).collect::<Vec<_>>(),
        [wrapped, key(62), key(1), key(2)]
    );
}

#[test]
fn capacity_failure_preserves_map_duplicates_still_succeed_and_clear_reuses_storage() {
    let mut map = ActorGraphIntents::try_new::<()>().unwrap();
    for word in 0..64 {
        assert!(map.insert_unique(key(word), f32::from_bits(word)).unwrap());
    }
    let before = map
        .iter()
        .map(|entry| (entry.name, entry.value.to_bits()))
        .collect::<Vec<_>>();
    assert_eq!(
        map.insert_unique(key(64), 1.0),
        Err(ActorGraphIntentError::Capacity)
    );
    assert!(!map.insert_unique(key(1), 999.0).unwrap());
    assert_eq!(
        map.iter()
            .map(|entry| (entry.name, entry.value.to_bits()))
            .collect::<Vec<_>>(),
        before
    );
    let storage = map.nodes.as_ptr();
    map.clear();
    assert_eq!(map.len(), 0);
    assert_eq!(map.iter().count(), 0);
    assert!(map.get(&key(1)).is_none());
    for word in 0..64 {
        assert!(map.insert_unique(key(word), -0.0).unwrap());
    }
    assert_eq!(map.nodes.as_ptr(), storage);
    assert_eq!(map.get(&key(1)).map(f32::to_bits), Some(0x8000_0000));
}

#[test]
fn sink_packing_preserves_first_value_and_nan_payload() {
    let mut map = ActorGraphIntents::try_new::<()>().unwrap();
    ActionInputIntentSink::insert(&mut map, "GestureSpeed", f32::from_bits(0x7fc1_2345)).unwrap();
    ActionInputIntentSink::insert(&mut map, "gesturespeed", 9.0).unwrap();
    assert_eq!(map.len(), 1);
    assert_eq!(
        map.get(&ActorGraphIntents::native_name(b"GestureSpeed"))
            .map(f32::to_bits),
        Some(0x7fc1_2345)
    );
}

#[test]
fn actual_listener_suffix_inserts_into_retained_map_and_keeps_first_event_speed() {
    use crate::{
        action_graph_input_listener::{
            ActionGraphInputListener,
            tests::{gameplay, matched, pattern},
        },
        game_input_player::recognizer::{
            GameInputPatternMessage, event_route::GameInputEventRoute,
        },
        motion_graph_entity_authority::ActorGraphAuthority,
    };
    use std::sync::{Arc, Mutex};
    let authority = ActorGraphAuthority::new(1);
    let (player, receiver, _manager) = gameplay();
    let listener = Arc::new(Mutex::new(
        ActionGraphInputListener::new(&authority, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    let registration = route.try_construct_listener(&listener).unwrap();
    let mut state = listener.lock().unwrap();
    state
        .receive(&matched(&receiver, &pattern("Ollie"), 2))
        .unwrap();
    let mut later = matched(&receiver, &pattern("Kickflip"), 2);
    if let GameInputPatternMessage::Match { matched, .. } = &mut later {
        matched.normalized_speed = 0.75;
    }
    state.receive(&later).unwrap();
    let mut map = ActorGraphIntents::try_new::<()>().unwrap();
    state.try_pattern_intent_suffix(&mut map).unwrap();
    assert_eq!(map.len(), 4);
    for name in ["Trick", "Ollie", "Kickflip"] {
        assert_eq!(
            map.get(&ActorGraphIntents::native_name(name.as_bytes()))
                .map(f32::to_bits),
            Some(1.0_f32.to_bits())
        );
    }
    assert_eq!(
        map.get(&ActorGraphIntents::native_name(b"GestureSpeed"))
            .map(f32::to_bits),
        Some(0.25_f32.to_bits())
    );
    assert_eq!(state.events().len(), 2);
    drop(state);
    route.try_unregister(&registration).unwrap();
}
