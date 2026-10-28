use crate::physics_state_ids::PHYSICS_STATE_PHYSICS_GROUND;
use super::{SkeletonProcessedConstructorAlreadyInitialized, SkeletonProcessedContact};

#[test]
fn actual_constructor_reset_fields_initialize_once_without_replacing_occurrence() {
    let mut value = SkeletonProcessedContact::new();
    assert_eq!(value.board_flipped(), None);
    assert_eq!(value.physics_state_fields().category, None);
    value.try_initialize_constructor_fields().unwrap();
    assert_eq!(value.board_flipped(), Some(false));
    assert_eq!(value.known_flags_8c4(), (0x2000, u32::MAX));
    assert_eq!(value.physics_state_fields().category, Some(0));
    value.physics_state_fields_mut().current = Some(PHYSICS_STATE_PHYSICS_GROUND);
    value.process_anim_out_board_flip_byte(1);
    assert_eq!(
        value.try_initialize_constructor_fields(),
        Err(SkeletonProcessedConstructorAlreadyInitialized)
    );
    assert_eq!(value.physics_state_fields().current, Some(PHYSICS_STATE_PHYSICS_GROUND));
    assert_eq!(value.board_flipped(), Some(true));
}

#[test]
fn reset_establishes_one_word_and_flip_preserves_other_bits() {
    let mut state = SkeletonProcessedContact::new();
    assert_eq!(state.known_flags_8c4(), (0, 0));
    assert_eq!(state.board_flipped(), None);
    state.reset();
    assert_eq!(state.known_flags_8c4(), (0x2000, u32::MAX));
    for (raw, expected) in [(1, true), (2, false), (255, true), (0, false)] {
        state.process_anim_out_board_flip_byte(raw);
        assert_eq!(state.board_flipped(), Some(expected));
        assert_eq!(state.known_flags_8c4().0 & !0x0010_0000, 0x2000);
        assert_eq!(state.jump_events().known_flags(), (0, 0x0060_0000));
    }
}

#[test]
fn anim_flip_store_does_not_establish_unwritten_bits() {
    let mut state = SkeletonProcessedContact::new();
    state.process_anim_out_board_flip_byte(3);
    assert_eq!(state.known_flags_8c4(), (0x0010_0000, 0x0010_0000));
    assert_eq!(state.jump_events().known_flags(), (0, 0));
    state.reset();
    assert_eq!(state.board_flipped(), Some(false));
}
