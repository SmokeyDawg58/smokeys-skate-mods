use super::*;

fn attribute(name: &str, kind: i8, flags: u8) -> PhysicsAnimationAttribute {
    PhysicsAnimationAttribute {
        payload: [None; 24],
        begin: f32::NAN,
        end: f32::NAN,
        name: FastString30::encode(name).unwrap(),
        flags,
        kind,
        sequence: None,
    }
}

#[test]
fn wipeout_name_sets_same_word_without_status_payload_or_sequence_gate() {
    let mut processed = SkeletonProcessedContact::new();
    processed.try_initialize_constructor_fields().unwrap();
    processed.process_anim_out_board_flip_byte(1);
    let before = processed.known_flags_8c4();
    assert!(!processed.apply_animation_wipeout_attribute(&attribute("wrong", 0, 6)));
    assert!(!processed.apply_animation_wipeout_attribute(&attribute("Wipeout", 3, 6)));
    assert!(!processed.apply_animation_wipeout_attribute(&attribute("Wipeout", -1, 6)));
    assert_eq!(processed.known_flags_8c4(), before);
    for flags in [0, 1, 6, 9, 17, 255] {
        assert!(processed.apply_animation_wipeout_attribute(&attribute("Wipeout", 0, flags)));
        assert_eq!(processed.known_flags_8c4(), (before.0 | WIPEOUT, before.1));
    }
    processed.reset();
    assert_eq!(processed.known_flags_8c4(), (0x2000, u32::MAX));
}

#[test]
fn wipeout_does_not_invent_unwritten_neighbor_flags() {
    let mut processed = SkeletonProcessedContact::new();
    assert!(processed.apply_animation_wipeout_attribute(&attribute("Wipeout", 0, 0)));
    assert_eq!(processed.known_flags_8c4(), (WIPEOUT, WIPEOUT));
}
