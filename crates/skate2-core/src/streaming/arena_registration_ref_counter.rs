//! First-use/last-use arena registration counts shared by native subsystems.
//!
//! `PresThreadMsgMan` owns an `assman::cAssetRefCounter<4096, 2048>` at its
//! message boundary; `cAssetActivationManager` owns the other recovered
//! `<1024, 512>` specialization. `SimThreadMsgMan` has no such field and does
//! not apply this coalescing step. `AddAsset` (`0x82d5bd80`) registers only a
//! previously absent full 64-bit arena ID. `RemAsset` (`0x82d5a0d0`)
//! deregisters only when the old count was one. Existing counts use ordinary
//! wrapping `u32` arithmetic. This decision logic is separate from resident
//! collection loaded counts and from any eventual PC scene handle.

use crate::collection_resident_streamer::CollectionId;
use std::{collections::BTreeMap, error::Error, fmt};

/// Exact first-use decision returned by the native add method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArenaRegistrationAddTransition {
    Retained,
    FirstUse,
}

/// Exact last-use decision returned by the native remove method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArenaRegistrationRemoveTransition {
    Retained,
    LastUse,
}

/// Safe host representation of one subsystem's native arena-use counter.
///
/// The two recovered owners use fixed EASTL hash tables with different inline
/// node capacities and bucket counts, with overflow allocation enabled by a
/// separate template argument. Ordering is not observable through this
/// decision API, so the reconstruction deliberately uses an unbounded map
/// while preserving full ID equality and exact per-ID arithmetic.
/// Owner-specific fixed storage and overflow-allocation behavior remain
/// unimplemented.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct ArenaRegistrationRefCounter {
    counts: BTreeMap<u64, u32>,
}

impl ArenaRegistrationRefCounter {
    /// Mirrors `cAssetRefCounter::AddAsset`: insert one and report first use,
    /// or wrapping-increment an existing count and report retained use.
    pub fn add_asset(&mut self, id: CollectionId) -> ArenaRegistrationAddTransition {
        if let Some(count) = self.counts.get_mut(&id.0) {
            *count = count.wrapping_add(1);
            ArenaRegistrationAddTransition::Retained
        } else {
            self.counts.insert(id.0, 1);
            ArenaRegistrationAddTransition::FirstUse
        }
    }

    /// Mirrors `cAssetRefCounter::RemAsset`: wrapping-decrement an existing
    /// count, erasing and reporting last use only when its old value was one.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaRegistrationRefCounterError::MissingAsset`] without
    /// mutation when the ID is absent. The reference enters an assertion path;
    /// a typed fail-stop is the safe host policy rather than fabricated
    /// post-assert behavior.
    pub fn remove_asset(
        &mut self,
        id: CollectionId,
    ) -> Result<ArenaRegistrationRemoveTransition, ArenaRegistrationRefCounterError> {
        let count = self
            .counts
            .get_mut(&id.0)
            .ok_or(ArenaRegistrationRefCounterError::MissingAsset(id))?;
        let old = *count;
        *count = old.wrapping_sub(1);
        if old == 1 {
            self.counts.remove(&id.0);
            Ok(ArenaRegistrationRemoveTransition::LastUse)
        } else {
            Ok(ArenaRegistrationRemoveTransition::Retained)
        }
    }

    /// Returns the exact raw count for diagnostics and invariant checks.
    #[must_use]
    pub fn count(&self, id: CollectionId) -> Option<u32> {
        self.counts.get(&id.0).copied()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArenaRegistrationRefCounterError {
    MissingAsset(CollectionId),
}

impl fmt::Display for ArenaRegistrationRefCounterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAsset(id) => {
                write!(formatter, "arena {:#018x} has no registration count", id.0)
            }
        }
    }
}

impl Error for ArenaRegistrationRefCounterError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_add_and_last_remove_are_the_only_registration_transitions() {
        let id = CollectionId(0x1122_3344_5566_7788);
        let mut counter = ArenaRegistrationRefCounter::default();

        assert_eq!(
            counter.add_asset(id),
            ArenaRegistrationAddTransition::FirstUse
        );
        assert_eq!(counter.count(id), Some(1));
        assert_eq!(
            counter.add_asset(id),
            ArenaRegistrationAddTransition::Retained
        );
        assert_eq!(counter.count(id), Some(2));
        assert_eq!(
            counter.remove_asset(id),
            Ok(ArenaRegistrationRemoveTransition::Retained)
        );
        assert_eq!(counter.count(id), Some(1));
        assert_eq!(
            counter.remove_asset(id),
            Ok(ArenaRegistrationRemoveTransition::LastUse)
        );
        assert_eq!(counter.count(id), None);
        assert!(counter.is_empty());
    }

    #[test]
    fn existing_count_arithmetic_wraps_without_creating_transitions() {
        let id = CollectionId(7);
        let mut counter = ArenaRegistrationRefCounter::default();
        counter.counts.insert(id.0, u32::MAX);

        assert_eq!(
            counter.add_asset(id),
            ArenaRegistrationAddTransition::Retained
        );
        assert_eq!(counter.count(id), Some(0));
        assert_eq!(
            counter.remove_asset(id),
            Ok(ArenaRegistrationRemoveTransition::Retained)
        );
        assert_eq!(counter.count(id), Some(u32::MAX));
    }

    #[test]
    fn wrapped_zero_increment_is_retained_even_when_it_becomes_one() {
        let id = CollectionId(8);
        let mut counter = ArenaRegistrationRefCounter::default();
        counter.counts.insert(id.0, 0);

        assert_eq!(
            counter.add_asset(id),
            ArenaRegistrationAddTransition::Retained
        );
        assert_eq!(counter.count(id), Some(1));
        assert_eq!(
            counter.remove_asset(id),
            Ok(ArenaRegistrationRemoveTransition::LastUse)
        );
        assert!(counter.is_empty());
    }

    #[test]
    fn full_width_ids_with_equal_low_words_remain_distinct() {
        let low = CollectionId(0x1111_2222_dead_beef);
        let high = CollectionId(0x9999_aaaa_dead_beef);
        let mut counter = ArenaRegistrationRefCounter::default();

        assert_eq!(
            counter.add_asset(low),
            ArenaRegistrationAddTransition::FirstUse
        );
        assert_eq!(
            counter.add_asset(high),
            ArenaRegistrationAddTransition::FirstUse
        );
        assert_eq!(counter.count(low), Some(1));
        assert_eq!(counter.count(high), Some(1));
        assert_eq!(
            counter.remove_asset(low),
            Ok(ArenaRegistrationRemoveTransition::LastUse)
        );
        assert_eq!(counter.count(high), Some(1));
    }

    #[test]
    fn missing_remove_is_typed_and_does_not_mutate_other_ids() {
        let retained = CollectionId(3);
        let missing = CollectionId(4);
        let mut counter = ArenaRegistrationRefCounter::default();
        counter.add_asset(retained);

        assert_eq!(
            counter.remove_asset(missing),
            Err(ArenaRegistrationRefCounterError::MissingAsset(missing))
        );
        assert_eq!(counter.count(retained), Some(1));
    }
}
