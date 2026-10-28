#![expect(clippy::float_cmp, reason = "exact source stores and fixture values")]
//! Tests within the existing wipeout-owner access child, no replacement owner.
use super::*;

#[test]
fn air_counter_access_requires_construction_and_does_not_reset_other_state() {
    let mut owner = PhysicalPlayerWipeoutOwner::new();
    assert!(owner.try_air_deck_counter().is_err());
    assert!(owner.try_store_air_deck_counter(4).is_err());
    owner.try_construct(true).unwrap();
    owner.try_clear_requests().unwrap();
    owner.try_ground_request(21).unwrap();
    owner.try_store_ground_invincibility(0.25).unwrap();
    owner.try_store_ground_state(1).unwrap();
    owner.try_store_air_deck_counter(-4).unwrap();
    assert_eq!(owner.try_air_deck_counter().unwrap(), -4);
    assert_eq!(owner.try_ground_state().unwrap(), 1);
    assert_eq!(owner.try_ground_invincibility().unwrap(), 0.25);
    assert_eq!(owner.try_request_count().unwrap(), 1);
    owner.try_store_air_deck_counter(0).unwrap();
    owner.try_store_ground_state(2).unwrap();
    assert_eq!(owner.try_air_deck_counter().unwrap(), 0);
    assert_eq!(owner.try_request_count().unwrap(), 1);
    assert!(
        owner
            .try_is_requested(crate::wipeout::WipeoutType::ALL[21])
            .unwrap()
    );
}
