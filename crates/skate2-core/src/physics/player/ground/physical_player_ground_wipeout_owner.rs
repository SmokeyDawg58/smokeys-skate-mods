//! Access to actual private fields through `physical_player_wipeout::ground_access`.
use super::{PhysicalPlayerWipeoutOwner, WipeoutOwnerError};
#[derive(Debug)]
pub(crate) enum GroundOwnerError {
    Owner(WipeoutOwnerError),
    MissingBalance,
}
impl std::fmt::Display for GroundOwnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "Ground wipeout owner: {error:?}"),
            Self::MissingBalance => f.write_str("Ground wipeout off-balance total is unknown"),
        }
    }
}
impl PhysicalPlayerWipeoutOwner {
    /// Actual Wipeout+B0 counter used by `CheckForAirWipeout82de1118`.
    pub(crate) fn try_air_deck_counter(&self) -> Result<i32, GroundOwnerError> {
        Ok(self
            .value
            .as_ref()
            .ok_or(GroundOwnerError::Owner(WipeoutOwnerError::NotConstructed))?
            .deck_contact_counter)
    }
    /// Raw store: caller performs counter-clear BEFORE storing state2.
    pub(crate) fn try_store_air_deck_counter(
        &mut self,
        value: i32,
    ) -> Result<(), GroundOwnerError> {
        self.value
            .as_mut()
            .ok_or(GroundOwnerError::Owner(WipeoutOwnerError::NotConstructed))?
            .deck_contact_counter = value;
        Ok(())
    }

    pub(crate) fn try_ground_state(&self) -> Result<i32, GroundOwnerError> {
        Ok(self
            .value
            .as_ref()
            .ok_or(GroundOwnerError::Owner(WipeoutOwnerError::NotConstructed))?
            .state)
    }
    /// Raw detector writes state AFTER clearing balance, unlike `SetState` helper.
    pub(crate) fn try_store_ground_state(&mut self, state: i32) -> Result<(), GroundOwnerError> {
        self.value
            .as_mut()
            .ok_or(GroundOwnerError::Owner(WipeoutOwnerError::NotConstructed))?
            .state = state;
        Ok(())
    }
    pub(crate) fn try_ground_balance(&self) -> Result<f32, GroundOwnerError> {
        self.value
            .as_ref()
            .ok_or(GroundOwnerError::Owner(WipeoutOwnerError::NotConstructed))?
            .off_balance_total
            .ok_or(GroundOwnerError::MissingBalance)
    }
    pub(crate) fn try_store_ground_balance(&mut self, total: f32) -> Result<(), GroundOwnerError> {
        self.value
            .as_mut()
            .ok_or(GroundOwnerError::Owner(WipeoutOwnerError::NotConstructed))?
            .off_balance_total = Some(total);
        Ok(())
    }
    pub(crate) fn try_ground_invincibility(&self) -> Result<f32, GroundOwnerError> {
        Ok(self
            .value
            .as_ref()
            .ok_or(GroundOwnerError::Owner(WipeoutOwnerError::NotConstructed))?
            .invincible_timer)
    }
    pub(crate) fn try_store_ground_invincibility(
        &mut self,
        timer: f32,
    ) -> Result<(), GroundOwnerError> {
        self.value
            .as_mut()
            .ok_or(GroundOwnerError::Owner(WipeoutOwnerError::NotConstructed))?
            .invincible_timer = timer;
        Ok(())
    }
    pub(crate) fn try_ground_request(&mut self, index: usize) -> Result<(), GroundOwnerError> {
        self.try_wipeout_request_value(index, 0.0)
    }
    /// `BadLanding`82dd98cc: heavy landing stores actual downward speed.
    pub(crate) fn try_wipeout_request_value(
        &mut self,
        index: usize,
        value: f32,
    ) -> Result<(), GroundOwnerError> {
        let state = self
            .value
            .as_mut()
            .ok_or(GroundOwnerError::Owner(WipeoutOwnerError::NotConstructed))?;
        let ledger = state.requests.as_mut().ok_or(GroundOwnerError::Owner(
            WipeoutOwnerError::RequestsUninitialized,
        ))?;
        ledger.request(crate::wipeout::WipeoutType::ALL[index], value);
        // Preserve both existing representations until the owner consolidates.
        state.request_counter = ledger.request_count();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valued_request_retains_latest_payload_and_counts_every_event() {
        let mut owner = PhysicalPlayerWipeoutOwner::new();
        owner.try_construct(true).unwrap();
        assert!(owner.try_wipeout_request_value(6, 5.0).is_err());
        owner.try_clear_requests().unwrap();
        owner.try_wipeout_request_value(6, 5.0).unwrap();
        owner
            .try_wipeout_request_value(6, f32::from_bits(0x7fc0_1234))
            .unwrap();
        assert_eq!(owner.try_request_count().unwrap(), 2);
        assert_eq!(
            owner
                .try_request_value(crate::wipeout::WipeoutType::ALL[6])
                .unwrap()
                .to_bits(),
            0x7fc0_1234
        );
        owner.try_ground_request(6).unwrap();
        assert_eq!(owner.try_request_count().unwrap(), 3);
        assert_eq!(
            owner
                .try_request_value(crate::wipeout::WipeoutType::ALL[6])
                .unwrap()
                .to_bits(),
            0
        );
    }
    #[test]
    fn requests_require_clear_and_count_duplicate_events_on_the_same_owner() {
        let mut owner = PhysicalPlayerWipeoutOwner::new();
        owner.try_construct(true).unwrap();
        assert!(matches!(
            owner.try_ground_request(2),
            Err(GroundOwnerError::Owner(
                WipeoutOwnerError::RequestsUninitialized
            ))
        ));
        assert_eq!(owner.try_request_count().unwrap(), 0);
        owner.try_clear_requests().unwrap();
        owner.try_ground_request(2).unwrap();
        owner.try_ground_request(2).unwrap();
        assert_eq!(owner.try_request_count().unwrap(), 2);
        assert_eq!(owner.value.as_ref().unwrap().request_counter, 2);
        assert!(
            owner
                .try_is_requested(crate::wipeout::WipeoutType::ALL[2])
                .unwrap()
        );
        assert_eq!(
            owner
                .try_request_value(crate::wipeout::WipeoutType::ALL[2])
                .unwrap()
                .to_bits(),
            0
        );
    }
    #[test]
    fn detector_field_writes_do_not_invoke_set_state_side_effects() {
        let mut owner = PhysicalPlayerWipeoutOwner::new();
        owner.try_construct(true).unwrap();
        assert!(matches!(
            owner.try_ground_balance(),
            Err(GroundOwnerError::MissingBalance)
        ));
        owner.try_store_ground_balance(7.0).unwrap();
        owner.try_store_ground_state(1).unwrap();
        assert_eq!(
            owner.try_ground_balance().unwrap().to_bits(),
            7.0_f32.to_bits()
        );
        owner.try_store_ground_invincibility(-0.06).unwrap();
        assert_eq!(
            owner.try_ground_invincibility().unwrap().to_bits(),
            (-0.06_f32).to_bits()
        );
    }
}

#[cfg(test)]
#[path = "../air/physical_player_air_wipeout_tests.rs"]
mod air_tests;
