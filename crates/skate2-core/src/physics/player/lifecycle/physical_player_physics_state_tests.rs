use super::*;

#[test]
fn source_constructor_preserves_unwritten_ground_fields() {
    for human in [0, 1, 255] {
        let state = PhysicsGroundFields::new(human);
        assert_eq!(state.is_human, human);
        assert_eq!(state.push_scalar.to_bits(), 0);
        assert!(state.hangup_force.is_none());
        assert!(state.anti_flip_torque.is_none());
        assert_eq!(state.hang_timer, None);
        assert_eq!(state.hang_pulse_countdown, None);
        assert_eq!(
            state.collision_force,
            [crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]); 2]
        );
    }
}

#[test]
fn entry_clears_only_source_fields_and_defers_final_stores() {
    let mut state = PhysicsGroundFields::new(1);
    state.do_jump = true;
    state.jump_direction = crate::skeleton_root_transforms::SkeletonRootVector::known([9.0; 4]);
    state.jump_com_height = 8.0;
    state.is_pinning = true;
    state.was_pinning = true;
    state.exiting_wallride = true;
    state.player_provided_controls = true;
    state.spawn_coordinates_set = true;
    state.enter_local_prefix();
    assert!(!state.do_jump);
    assert!(!state.was_pinning);
    assert!(!state.player_provided_controls);
    assert!(!state.spawn_coordinates_set);
    assert_eq!(
        state
            .jump_direction
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [9.0_f32.to_bits(); 4]
    );
    assert_eq!(state.jump_com_height.to_bits(), 8.0_f32.to_bits());
    assert!(state.is_pinning);
    assert!(state.exiting_wallride);
    assert_eq!(state.push_scalar.to_bits(), 0);
    assert!(state.hangup_force.is_none());
    state.finish_enter();
    assert_eq!(state.push_scalar.to_bits(), 1.0_f32.to_bits());
    assert_eq!(state.hangup_force.unwrap().map(f32::to_bits), [0; 4]);
    assert_eq!(state.hang_timer, Some(0));
    assert!(state.is_pinning);
    assert!(state.exiting_wallride);
}

#[test]
fn allocation_and_selected_scalar_constructor_never_establish_jphy() {
    let mut owner = PhysicalPlayerPhysicsStateOwner::new();
    assert_eq!(owner.current(), CurrentPhysicalState::Unselected);
    assert_eq!(owner.state_timer, None);
    owner.initialize_constructor_scalars();
    assert_eq!(owner.state_timer.unwrap().to_bits(), 0);
    assert_eq!(owner.state_count, Some(0));
    assert_eq!(owner.jphy_state, None);
    assert_eq!(
        owner.try_select_initial_sleeping(),
        Err(PhysicalStateConstructionError::WrongPhase)
    );
}

#[test]
fn actual_sim_allocation_establishes_jphy_but_not_separate_ground_storage() {
    let state = PhysicalPlayerPhysicsStateOwner::from_sim_zeroed_allocation();
    assert_eq!(state.jphy_state, Some(0));
    assert!(state.ground.is_none());
    let ground = PhysicsGroundFields::new(1);
    assert!(ground.hangup_force.is_none());
}
