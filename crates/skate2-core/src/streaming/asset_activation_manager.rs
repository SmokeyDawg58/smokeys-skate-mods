//! Exact queued/active scheduling owned by `cAssetActivationManager::Update`.
//!
//! Request-specific state effects remain behind a typed runtime. This module
//! owns the independently recovered forward admission, eight-active bound,
//! full-ID duplicate suppression, forward active scan, stable erase, and
//! immediate refill loop after any completed request.

use std::collections::TryReserveError;

/// Eight inline `tActiveRequest` records are constructed by the native owner.
pub const ASSET_ACTIVATION_ACTIVE_CAPACITY: usize = 8;

/// Identity read during queued-to-active admission.
pub trait AssetActivationRequest {
    fn asset_id(&self) -> u64;
}

/// Result of visiting one active request during `UpdateRequests`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetActivationRequestPoll {
    Retain,
    Complete,
}

/// Request-specific state-machine effects called in active-vector order.
///
/// `R` values move when either vector is stably erased. An asynchronous
/// implementation must therefore place callback-visible state in stable owned
/// storage (for example, a generation-checked `Arc` signal), never retain a
/// pointer or borrow into `R`. The later concrete activation owner also has to
/// model native LIFO reuse of its eight fixed slots; slot identity is outside
/// this scheduling-only type.
pub trait AssetActivationRequestRuntime<R> {
    type Error;

    /// Advances one active request once.
    ///
    /// Returning [`AssetActivationRequestPoll::Complete`] means all native
    /// completion-tail effects have already succeeded and authorizes the
    /// manager to erase this pointer and return its conceptual inline slot.
    ///
    /// # Errors
    ///
    /// Returns a checked host failure at the reached request. Previously
    /// completed requests stay erased; no native rollback is invented.
    fn update_request(
        &mut self,
        request: &mut R,
    ) -> Result<AssetActivationRequestPoll, Self::Error>;
}

/// Queue and active ownership around request-specific activation states.
#[derive(Debug)]
pub struct AssetActivationManager<R> {
    queued: Vec<R>,
    active: Vec<R>,
}

impl<R> Default for AssetActivationManager<R> {
    fn default() -> Self {
        Self {
            // The constructor's raw `maxRequests=4` reserves queued storage;
            // it is not a semantic request limit.
            queued: Vec::with_capacity(4),
            active: Vec::with_capacity(ASSET_ACTIVATION_ACTIVE_CAPACITY),
        }
    }
}

impl<R: AssetActivationRequest> AssetActivationManager<R> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one request without duplicate suppression, matching the native
    /// filename overload's locked vector append.
    ///
    /// # Errors
    ///
    /// Returns a checked host allocation failure before appending.
    pub fn enqueue(&mut self, request: R) -> Result<(), TryReserveError> {
        self.queued.try_reserve(1)?;
        self.queued.push(request);
        Ok(())
    }

    #[must_use]
    pub fn is_pending_requests(&self) -> bool {
        !self.active.is_empty() || !self.queued.is_empty()
    }

    #[must_use]
    pub fn queued(&self) -> &[R] {
        &self.queued
    }

    #[must_use]
    pub fn active(&self) -> &[R] {
        &self.active
    }

    /// Reconstructs `cAssetActivationManager::Update` (`0x82d5d8e0`).
    ///
    /// Each outer pass admits eligible queued requests in forward order until
    /// eight are active or no queued request is eligible. It then visits every
    /// active request in forward order. A completion is stably erased. If any
    /// request completed, admission and the complete active scan repeat within
    /// this same call; otherwise the call returns.
    ///
    /// # Errors
    ///
    /// Returns the first checked request-runtime error without retry or
    /// rollback. Admission itself requires no allocation because both vectors
    /// reserve their proven bounds before the operation.
    pub fn update<T>(&mut self, runtime: &mut T) -> Result<(), T::Error>
    where
        T: AssetActivationRequestRuntime<R>,
    {
        loop {
            self.admit_eligible_requests();
            let mut completed_any = false;
            let mut active_index = 0;
            while active_index < self.active.len() {
                match runtime.update_request(&mut self.active[active_index])? {
                    AssetActivationRequestPoll::Retain => active_index += 1,
                    AssetActivationRequestPoll::Complete => {
                        self.active.remove(active_index);
                        completed_any = true;
                    }
                }
            }
            if !completed_any {
                return Ok(());
            }
        }
    }

    fn admit_eligible_requests(&mut self) {
        let mut queued_index = 0;
        while queued_index < self.queued.len()
            && self.active.len() < ASSET_ACTIVATION_ACTIVE_CAPACITY
        {
            let asset_id = self.queued[queued_index].asset_id();
            if self
                .active
                .iter()
                .any(|active| active.asset_id() == asset_id)
            {
                queued_index += 1;
                continue;
            }
            let request = self.queued.remove(queued_index);
            self.active.push(request);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, convert::Infallible};

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Request {
        id: u64,
        ordinal: u32,
    }

    impl AssetActivationRequest for Request {
        fn asset_id(&self) -> u64 {
            self.id
        }
    }

    #[derive(Default)]
    struct Runtime {
        polls: Vec<u32>,
        completions_remaining: BTreeMap<u32, usize>,
    }

    impl AssetActivationRequestRuntime<Request> for Runtime {
        type Error = Infallible;

        fn update_request(
            &mut self,
            request: &mut Request,
        ) -> Result<AssetActivationRequestPoll, Self::Error> {
            self.polls.push(request.ordinal);
            let remaining = self
                .completions_remaining
                .entry(request.ordinal)
                .or_default();
            if *remaining == 0 {
                Ok(AssetActivationRequestPoll::Retain)
            } else {
                *remaining -= 1;
                Ok(AssetActivationRequestPoll::Complete)
            }
        }
    }

    fn request(id: u64, ordinal: u32) -> Request {
        Request { id, ordinal }
    }

    #[test]
    fn first_update_admits_only_eight_distinct_requests_in_forward_order() {
        let mut manager = AssetActivationManager::new();
        for ordinal in 0..10 {
            manager
                .enqueue(request(u64::from(ordinal), ordinal))
                .unwrap();
        }
        let mut runtime = Runtime::default();

        manager.update(&mut runtime).unwrap();

        assert_eq!(runtime.polls, (0..8).collect::<Vec<_>>());
        assert_eq!(
            manager
                .active()
                .iter()
                .map(|request| request.ordinal)
                .collect::<Vec<_>>(),
            (0..8).collect::<Vec<_>>()
        );
        assert_eq!(
            manager
                .queued()
                .iter()
                .map(|request| request.ordinal)
                .collect::<Vec<_>>(),
            [8, 9]
        );
    }

    #[test]
    fn active_duplicate_stays_queued_while_later_distinct_request_is_admitted() {
        let mut manager = AssetActivationManager::new();
        manager.enqueue(request(7, 0)).unwrap();
        manager.enqueue(request(7, 1)).unwrap();
        manager.enqueue(request(9, 2)).unwrap();
        let mut runtime = Runtime::default();

        manager.update(&mut runtime).unwrap();

        assert_eq!(runtime.polls, [0, 2]);
        assert_eq!(
            manager
                .active()
                .iter()
                .map(|request| request.ordinal)
                .collect::<Vec<_>>(),
            [0, 2]
        );
        assert_eq!(manager.queued(), [request(7, 1)]);
    }

    #[test]
    fn any_completion_refills_and_rescans_active_requests_in_the_same_update() {
        let mut manager = AssetActivationManager::new();
        for ordinal in 0..9 {
            manager
                .enqueue(request(u64::from(ordinal), ordinal))
                .unwrap();
        }
        let mut runtime = Runtime::default();
        runtime.completions_remaining.insert(0, 1);

        manager.update(&mut runtime).unwrap();

        assert_eq!(
            runtime.polls,
            [0, 1, 2, 3, 4, 5, 6, 7, 1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert!(manager.queued().is_empty());
        assert_eq!(
            manager
                .active()
                .iter()
                .map(|request| request.ordinal)
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6, 7, 8]
        );
    }

    #[test]
    fn full_width_ids_are_compared_without_low_word_aliasing() {
        let mut manager = AssetActivationManager::new();
        manager.enqueue(request(0x0000_0001_1234_5678, 0)).unwrap();
        manager.enqueue(request(0xffff_ffff_1234_5678, 1)).unwrap();
        let mut runtime = Runtime::default();

        manager.update(&mut runtime).unwrap();

        assert_eq!(runtime.polls, [0, 1]);
        assert!(manager.queued().is_empty());
    }

    #[test]
    fn pending_is_true_for_queued_or_active_ownership() {
        let mut manager = AssetActivationManager::new();
        assert!(!manager.is_pending_requests());
        manager.enqueue(request(1, 0)).unwrap();
        assert!(manager.is_pending_requests());
        let mut runtime = Runtime::default();
        manager.update(&mut runtime).unwrap();
        assert!(manager.is_pending_requests());
        runtime.completions_remaining.insert(0, 1);
        manager.update(&mut runtime).unwrap();
        assert!(!manager.is_pending_requests());
    }
}
