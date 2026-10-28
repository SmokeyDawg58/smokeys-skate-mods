use super::*;

#[test]
fn real_system_logic_clear_establishes_unknown_arrays_then_clears_same_ledger() {
    let mut owner = PhysicalPlayerWipeoutOwner::new();
    let heavy_landing = crate::wipeout::WipeoutType::ALL[6];
    assert_eq!(
        owner.try_is_requested(heavy_landing),
        Err(WipeoutOwnerError::NotConstructed)
    );
    assert_eq!(
        owner.try_clear_requests(),
        Err(WipeoutOwnerError::NotConstructed)
    );
    owner.try_construct(true).unwrap();
    assert_eq!(
        owner.try_is_requested(heavy_landing),
        Err(WipeoutOwnerError::RequestsUninitialized)
    );
    assert!(owner.value.as_ref().unwrap().requests.is_none());
    owner.try_clear_requests().unwrap();
    assert_eq!(owner.try_is_requested(heavy_landing), Ok(false));
    let requests = owner.value.as_mut().unwrap().requests.as_mut().unwrap();
    let kind = crate::wipeout::WipeoutType::ALL[0];
    requests.request(kind, 3.0);
    requests.request(kind, 4.0);
    assert_eq!(requests.request_count(), 2);
    requests.request(heavy_landing, 1.0);
    assert_eq!(owner.try_is_requested(heavy_landing), Ok(true));
    owner.try_clear_requests().unwrap();
    assert_eq!(owner.try_is_requested(heavy_landing), Ok(false));
    let requests = owner.value.as_ref().unwrap().requests.as_ref().unwrap();
    assert_eq!(requests.request_count(), 0);
    for kind in crate::wipeout::WipeoutType::ALL {
        assert!(!requests.is_requested(kind));
        assert_eq!(requests.request_value(kind).to_bits(), 0);
    }
}

#[test]
fn actual_initial_ground_transition_establishes_previously_unknown_balance() {
    let mut owner = PhysicalPlayerWipeoutOwner::new();
    owner.try_construct(true).unwrap();
    assert_eq!(owner.state_bits(), Some((0, None, 0, 0, 0)));
    owner.try_set_state(1).unwrap();
    assert_eq!(owner.state_bits(), Some((1, Some(0), 0, 0, 0)));
    let state = owner.value.as_mut().unwrap();
    state.off_balance_total = Some(17.0);
    state.deck_contact_counter = 8;
    owner.try_set_state(1).unwrap();
    assert_eq!(owner.state_bits().unwrap().1, Some(17.0_f32.to_bits()));
    owner.try_set_state(2).unwrap();
    assert_eq!(owner.state_bits().unwrap().2, 0);
    assert_eq!(owner.state_bits().unwrap().1, Some(17.0_f32.to_bits()));
    owner.try_set_state(99).unwrap();
    assert_eq!(owner.state_bits().unwrap().0, 99);
}

#[test]
fn null_allocation_is_retained_and_refuses_ground_transition() {
    let mut owner = PhysicalPlayerWipeoutOwner::new();
    owner.try_construct(false).unwrap();
    assert_eq!(owner.phase(), WipeoutOwnerPhase::AllocationNull);
    assert_eq!(
        owner.try_set_state(1),
        Err(WipeoutOwnerError::NotConstructed)
    );
    assert_eq!(
        owner.try_construct(true),
        Err(WipeoutOwnerError::AlreadyAttempted)
    );
}

#[test]
fn teleport_reset_only_clears_state_and_event_count() {
    let mut owner = PhysicalPlayerWipeoutOwner::new();
    owner.try_construct(true).unwrap();
    owner.try_clear_requests().unwrap();
    owner.try_set_state(1).unwrap();
    let kind = crate::wipeout::WipeoutType::ALL[6];
    let state = owner.value.as_mut().unwrap();
    state.invincible_timer = 0.75;
    state.deck_contact_counter = 8;
    state.off_balance_total = Some(17.0);
    state.requests.as_mut().unwrap().request(kind, 3.0);
    assert_eq!(owner.try_request_count(), Ok(1));

    owner.try_reset().unwrap();

    assert_eq!(owner.try_request_count(), Ok(0));
    assert_eq!(owner.try_is_requested(kind), Ok(true));
    assert_eq!(owner.try_request_value(kind), Ok(3.0));
    let state = owner.value.as_ref().unwrap();
    assert_eq!(state.state, 0);
    assert_eq!(state.request_counter, 0);
    assert_eq!(state.invincible_timer, 0.75);
    assert_eq!(state.deck_contact_counter, 8);
    assert_eq!(state.off_balance_total, Some(17.0));
}
