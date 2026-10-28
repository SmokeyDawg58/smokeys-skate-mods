use super::*;

#[test]
fn actual_sim_allocation_establishes_residuals_once_without_reclearing_on_retry() {
    let mut processed = SkeletonProcessedContact::new();
    processed
        .try_initialize_constructor_fields_in_sim_allocation()
        .unwrap();
    assert_eq!(
        processed
            .animation_input
            .living_world_impulse
            .unwrap()
            .map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(
        processed.animation_input.external_reset_on_board,
        Some(false)
    );
    processed.try_process_animation_input(&input()).unwrap();
    assert!(
        processed
            .try_initialize_constructor_fields_in_sim_allocation()
            .is_err()
    );
    assert_eq!(
        processed
            .animation_input
            .living_world_impulse
            .unwrap()
            .map(f32::to_bits),
        [1.0, 2.0, 3.0, -0.0].map(f32::to_bits)
    );
}

fn input() -> AnimOutPhysInOwner {
    let mut input = AnimOutPhysInOwner::new_sim_allocation();
    input.try_construct(1).unwrap();
    input.time_delta = -0.0;
    input.compression = f32::from_bits(0x7fc0_1234);
    input.living_world_impulse = [1.0, 2.0, 3.0, -0.0];
    input.external_reset_transform =
        crate::skeleton_root_transforms::SkeletonRootMatrix::known(std::array::from_fn(|i| {
            f32::from_bits(0x3f80_0000 + u32::try_from(i).unwrap())
        }));
    input.external_reset_on_board = true;
    input.truck_tightness = 0.25;
    input.wheel_hardness = 0.75;
    input
}

#[test]
fn inactive_payloads_copy_all_lanes_and_flags_preserve_other_producers() {
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    processed.physics_state.flags_8cc = Some(0xdead_beef);
    let mut input = input();
    input.flags.board_flipped = true;
    input.flags.riding_switch = false;
    processed.try_process_animation_input(&input).unwrap();
    assert_eq!(processed.known_flags_8c4(), (0x0010_2000, u32::MAX));
    assert_eq!(
        processed.physics_state.flags_8cc,
        Some(0xdead_beef & !0x0040_0000)
    );
    assert_eq!(
        processed.physics_state.time_delta.unwrap().to_bits(),
        0x8000_0000
    );
    assert_eq!(
        processed.animation_input.compression.unwrap().to_bits(),
        0x7fc0_1234
    );
    assert_eq!(
        processed
            .animation_input
            .living_world_impulse
            .unwrap()
            .map(f32::to_bits),
        input.living_world_impulse.map(f32::to_bits)
    );
    for column in 0..4 {
        assert_eq!(
            processed.animation_input.external_reset_transform[column]
                .unwrap()
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            input.external_reset_transform.columns()[column]
                .known_lanes()
                .unwrap()
                .map(f32::to_bits)
        );
    }
    assert_eq!(
        processed.animation_input.external_reset_on_board,
        Some(true)
    );
    input.flags.board_flipped = false;
    input.flags.living_world_impulse_active = true;
    input.external_reset_active = true;
    input.flags.riding_switch = true;
    processed.try_process_animation_input(&input).unwrap();
    assert_eq!(processed.known_flags_8c4(), (0x200c, u32::MAX));
    assert_eq!(
        processed.physics_state.flags_8cc,
        Some(0xdead_beef | 0x0040_0000)
    );
    let mut translation = input.external_reset_transform.columns()[3];
    translation.w = None;
    input.external_reset_transform.set_column(3, translation);
    processed.try_process_animation_input(&input).unwrap();
    assert_eq!(
        processed.animation_input.external_reset_transform[3]
            .unwrap()
            .w,
        None
    );
}

#[test]
fn reset_preserves_unwritten_payloads_then_copied_payloads() {
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    assert!(processed.animation_input.living_world_impulse.is_none());
    assert_eq!(
        processed.animation_input.external_reset_transform,
        [None; 4]
    );
    assert_eq!(processed.animation_input.external_reset_on_board, None);
    processed.try_process_animation_input(&input()).unwrap();
    let impulse = processed
        .animation_input
        .living_world_impulse
        .map(|v| v.map(f32::to_bits));
    let transform = processed
        .animation_input
        .external_reset_transform
        .map(|v| v.map(|v| v.known_lanes().unwrap().map(f32::to_bits)));
    processed.reset();
    assert_eq!(
        processed
            .animation_input
            .living_world_impulse
            .map(|v| v.map(f32::to_bits)),
        impulse
    );
    assert_eq!(
        processed
            .animation_input
            .external_reset_transform
            .map(|v| v.map(|v| v.known_lanes().unwrap().map(f32::to_bits))),
        transform
    );
    assert_eq!(
        processed.animation_input.external_reset_on_board,
        Some(true)
    );
    assert_eq!(processed.animation_input.compression.unwrap().to_bits(), 0);
    assert_eq!(
        processed.animation_input.truck_tightness.unwrap().to_bits(),
        0
    );
    assert_eq!(
        processed.animation_input.wheel_hardness.unwrap().to_bits(),
        0
    );
}

#[test]
fn unknown_final_word_retains_prior_copies_and_unconstructed_input_writes_nothing() {
    let mut processed = SkeletonProcessedContact::new();
    assert_eq!(
        processed.try_process_animation_input(&AnimOutPhysInOwner::new_sim_allocation()),
        Err(ProcessedAnimationInputError::InputNotConstructed)
    );
    assert_eq!(processed.known_flags_8c4(), (0, 0));
    assert_eq!(
        processed.try_process_animation_input(&input()),
        Err(ProcessedAnimationInputError::UnknownRidingSwitchWord)
    );
    assert_eq!(processed.known_flags_8c4(), (0, 0x0010_000c));
    assert_eq!(
        processed.animation_input.wheel_hardness.unwrap().to_bits(),
        0.75_f32.to_bits()
    );
    assert_eq!(processed.physics_state.flags_8cc, None);
}
