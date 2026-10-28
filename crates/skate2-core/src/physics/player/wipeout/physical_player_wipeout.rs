//! Selected actual pair-owned Wipeout constructor and `SetState` fields.
//! The existing request algorithms are not implicitly scheduled or initialized.
//! Trajectory-selector construction remains outside this bounded projection.
#[path = "physical_player_wipeout_decision.rs"]
pub(crate) mod decision;
#[path = "../ground/physical_player_ground_wipeout_owner.rs"]
pub(crate) mod ground_access;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WipeoutOwnerPhase {
    Unattempted,
    AllocationNull,
    Constructed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WipeoutOwnerError {
    AlreadyAttempted,
    NotConstructed,
    RequestsUninitialized,
}

pub(crate) struct PhysicalPlayerWipeoutOwner {
    phase: WipeoutOwnerPhase,
    value: Option<WipeoutState>,
    #[cfg(test)]
    clear_calls: u32,
}
struct WipeoutState {
    invincible_timer: f32,
    deck_contact_counter: i32,
    request_counter: u32,
    requests: Option<crate::wipeout::WipeoutRequestLedger>,
    off_balance_total: Option<f32>,
    state: i32,
}
impl PhysicalPlayerWipeoutOwner {
    /// Native count+B4. Before Clear the constructor count exists independently
    /// of the uninitialized arrays; afterwards the ledger owns event count.
    pub(crate) fn try_request_count(&self) -> Result<u32, WipeoutOwnerError> {
        let state = self
            .value
            .as_ref()
            .ok_or(WipeoutOwnerError::NotConstructed)?;
        Ok(state.requests.as_ref().map_or(
            state.request_counter,
            crate::wipeout::WipeoutRequestLedger::request_count,
        ))
    }
    /// Actual enclosing constructor store82e04d24, after initial Ground entry.
    pub(crate) fn try_initialize_spawn_invincibility(&mut self) -> Result<(), WipeoutOwnerError> {
        self.value
            .as_mut()
            .ok_or(WipeoutOwnerError::NotConstructed)?
            .invincible_timer = 0.04;
        Ok(())
    }
    pub(crate) fn new() -> Self {
        Self {
            phase: WipeoutOwnerPhase::Unattempted,
            value: None,
            #[cfg(test)]
            clear_calls: 0,
        }
    }
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "construction phase inspected by retained-owner tests; full scheduler pending"
        )
    )]
    pub(crate) fn phase(&self) -> WipeoutOwnerPhase {
        self.phase
    }
    /// Inline ctor82e04260/78 then Reset82dd4480: invincible0/deckCounter0,
    /// state0/requestCounter0. `offBalance`+b8 is not written by either.
    /// Source board/Skeleton/processed/Reckoning links belong to SAME outer pair.
    pub(crate) fn try_construct(&mut self, allocated: bool) -> Result<(), WipeoutOwnerError> {
        if self.phase != WipeoutOwnerPhase::Unattempted {
            return Err(WipeoutOwnerError::AlreadyAttempted);
        }
        if allocated {
            self.value = Some(WipeoutState {
                invincible_timer: 0.0,
                deck_contact_counter: 0,
                state: 0,
                request_counter: 0,
                requests: None,
                off_balance_total: None,
            });
            self.phase = WipeoutOwnerPhase::Constructed;
        } else {
            self.phase = WipeoutOwnerPhase::AllocationNull;
        }
        Ok(())
    }
    /// Actual Update2 suffix82e01c5c calls Clear82dd4570 only after successful
    /// `CheckForStateChange`. Constructor Reset4480 never writes these arrays.
    pub(crate) fn try_clear_requests(&mut self) -> Result<(), WipeoutOwnerError> {
        let state = self
            .value
            .as_mut()
            .ok_or(WipeoutOwnerError::NotConstructed)?;
        state.request_counter = 0;
        if let Some(requests) = state.requests.as_mut() {
            requests.clear();
        } else {
            state.requests = Some(crate::wipeout::WipeoutRequestLedger::new());
        }
        #[cfg(test)]
        {
            self.clear_calls = self.clear_calls.wrapping_add(1);
        }
        Ok(())
    }
    /// Reset82dd4480 stores state+0xbc=0 before count+0xb4=0. It does not
    /// clear the request flags/values, timers, or other Wipeout fields.
    pub(crate) fn try_reset(&mut self) -> Result<(), WipeoutOwnerError> {
        let state = self
            .value
            .as_mut()
            .ok_or(WipeoutOwnerError::NotConstructed)?;
        state.state = 0;
        state.request_counter = 0;
        if let Some(requests) = state.requests.as_mut() {
            requests.reset_count_only();
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn clear_calls_for_test(&self) -> u32 {
        self.clear_calls
    }
    /// 82dd8c50: equality returns without clearing fields. Changed state is
    /// committed before Ground1 clears off-balance or Air2 clears contact count.
    pub(crate) fn try_set_state(&mut self, next: i32) -> Result<(), WipeoutOwnerError> {
        let state = self
            .value
            .as_mut()
            .ok_or(WipeoutOwnerError::NotConstructed)?;
        if state.state == next {
            return Ok(());
        }
        state.state = next;
        match next {
            1 => state.off_balance_total = Some(0.0),
            2 => state.deck_contact_counter = 0,
            _ => {}
        }
        Ok(())
    }
    /// Direct native request read (`WipeoutGround.Enter`82d8ec38 uses `HeavyLanding6`).
    /// Constructor/Reset do not initialize this ledger; only actual Clear does.
    pub(crate) fn try_is_requested(
        &self,
        kind: crate::wipeout::WipeoutType,
    ) -> Result<bool, WipeoutOwnerError> {
        let state = self
            .value
            .as_ref()
            .ok_or(WipeoutOwnerError::NotConstructed)?;
        Ok(state
            .requests
            .as_ref()
            .ok_or(WipeoutOwnerError::RequestsUninitialized)?
            .is_requested(kind))
    }
    /// Actual request-value read82dd46fc, after output State guard/lookup.
    pub(crate) fn try_request_value(
        &self,
        kind: crate::wipeout::WipeoutType,
    ) -> Result<f32, WipeoutOwnerError> {
        Ok(self
            .value
            .as_ref()
            .ok_or(WipeoutOwnerError::NotConstructed)?
            .requests
            .as_ref()
            .ok_or(WipeoutOwnerError::RequestsUninitialized)?
            .request_value(kind))
    }
    #[cfg(test)]
    pub(crate) fn request_snapshot(&self) -> Option<crate::wipeout::WipeoutRequestLedger> {
        self.value.as_ref().and_then(|state| state.requests.clone())
    }
    #[cfg(test)]
    pub(crate) fn requests_initialized(&self) -> bool {
        self.value
            .as_ref()
            .is_some_and(|state| state.requests.is_some())
    }
    #[cfg(test)]
    pub(crate) fn state_bits(&self) -> Option<(i32, Option<u32>, i32, u32, u32)> {
        self.value.as_ref().map(|s| {
            (
                s.state,
                s.off_balance_total.map(f32::to_bits),
                s.deck_contact_counter,
                s.request_counter,
                s.invincible_timer.to_bits(),
            )
        })
    }
}

#[cfg(test)]
#[path = "physical_player_wipeout_tests.rs"]
mod tests;
