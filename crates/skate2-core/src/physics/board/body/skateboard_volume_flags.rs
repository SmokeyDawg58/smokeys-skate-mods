//! Mutable flags for the SAME constructed Body volumes (`EnableCollisionVolumes`
//! 0x82b241a8). Immutable component geometry remains constructor evidence.
//! Top slots are deck aggregate, shared truck volume, shared wheel volume.
use super::{SkateboardDeckDefinition, SkateboardTruckDefinitions, SkateboardWheelDefinitions};
use std::sync::{RwLock, RwLockReadGuard};

#[derive(Debug)]
pub(crate) struct SkateboardVolumeFlags {
    state: RwLock<SkateboardVolumeFlagState>,
}

#[derive(Debug)]
pub(crate) struct SkateboardVolumeFlagState {
    top: [u32; 3],
    deck: Vec<u32>,
    revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardVolumeFlagError {
    Poisoned,
    TopIndex(usize),
    DeckIndex(usize),
    RevisionExhausted,
}

impl SkateboardVolumeFlags {
    #[cfg(test)]
    pub(crate) fn disable_wheels_for_test(&self) {
        let mut state = self.state.write().unwrap();
        state.top[2] &= !1;
        state.revision += 1;
    }
    pub(super) fn from_components(
        wheels: &SkateboardWheelDefinitions,
        trucks: &SkateboardTruckDefinitions,
        deck: &SkateboardDeckDefinition,
    ) -> Self {
        Self {
            state: RwLock::new(SkateboardVolumeFlagState {
                top: [
                    deck.aggregate().flags(),
                    trucks.parts()[0].volume().flags,
                    wheels.parts()[0].volume().flags,
                ],
                deck: deck
                    .aggregate()
                    .children()
                    .iter()
                    .map(|v| v.flags)
                    .collect(),
                revision: 0,
            }),
        }
    }

    pub(crate) fn read(
        &self,
    ) -> Result<RwLockReadGuard<'_, SkateboardVolumeFlagState>, SkateboardVolumeFlagError> {
        self.state
            .read()
            .map_err(|_| SkateboardVolumeFlagError::Poisoned)
    }

    /// One native read/modify/write. The host revision invalidates previously
    /// published bounds even when the new value happens to equal the old value.
    pub(crate) fn enable_top(&self, index: usize) -> Result<(), SkateboardVolumeFlagError> {
        self.set_enabled(index, false, true)
    }

    pub(crate) fn enable_deck(&self, index: usize) -> Result<(), SkateboardVolumeFlagError> {
        self.set_enabled(index, true, true)
    }

    pub(crate) fn disable_top(&self, index: usize) -> Result<(), SkateboardVolumeFlagError> {
        self.set_enabled(index, false, false)
    }

    pub(crate) fn disable_deck(&self, index: usize) -> Result<(), SkateboardVolumeFlagError> {
        self.set_enabled(index, true, false)
    }

    fn set_enabled(
        &self,
        index: usize,
        deck: bool,
        enabled: bool,
    ) -> Result<(), SkateboardVolumeFlagError> {
        use SkateboardVolumeFlagError as E;
        let mut state = self.state.write().map_err(|_| E::Poisoned)?;
        let next = state.revision.checked_add(1).ok_or(E::RevisionExhausted)?;
        let flags = if deck {
            state.deck.get_mut(index).ok_or(E::DeckIndex(index))?
        } else {
            state.top.get_mut(index).ok_or(E::TopIndex(index))?
        };
        *flags = (*flags & !1) | u32::from(enabled);
        state.revision = next;
        Ok(())
    }
}

impl SkateboardVolumeFlagState {
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }
    pub(crate) fn deck_count(&self) -> usize {
        self.deck.len()
    }
    /// Publication emits four wheel occurrences, two truck occurrences, then
    /// deck children. Shared volumes therefore read the SAME flag word.
    pub(crate) fn emission_flags(&self, index: usize) -> Option<u32> {
        match index {
            0..=3 => Some(self.top[2]),
            4..=5 => Some(self.top[1]),
            _ => self.deck.get(index - 6).copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> SkateboardVolumeFlags {
        SkateboardVolumeFlags {
            state: RwLock::new(SkateboardVolumeFlagState {
                top: [0xa0, 0xb0, 0xc0],
                deck: vec![0x3e2, 0x22],
                revision: 0,
            }),
        }
    }
    #[test]
    fn source_enable_preserves_bits_and_shared_volume_occurrences() {
        let owner = fixture();
        owner.enable_top(2).unwrap();
        let state = owner.read().unwrap();
        for index in 0..4 {
            assert_eq!(state.emission_flags(index), Some(0xc1));
        }
        assert_eq!(state.emission_flags(4), Some(0xb0));
        assert_eq!(state.emission_flags(5), Some(0xb0));
        assert_eq!(state.emission_flags(6), Some(0x3e2));
        assert_eq!(state.revision(), 1);
        drop(state);
        owner.enable_deck(1).unwrap();
        let state = owner.read().unwrap();
        assert_eq!(state.emission_flags(7), Some(0x23));
        assert_eq!(state.emission_flags(8), None);
        assert_eq!(state.deck_count(), 2);
    }
    #[test]
    fn checked_late_failure_preserves_prior_native_writes() {
        let owner = fixture();
        owner.enable_top(0).unwrap();
        owner.enable_deck(0).unwrap();
        assert_eq!(
            owner.enable_deck(2),
            Err(SkateboardVolumeFlagError::DeckIndex(2))
        );
        assert_eq!(
            owner.enable_top(3),
            Err(SkateboardVolumeFlagError::TopIndex(3))
        );
        let state = owner.read().unwrap();
        assert_eq!(state.top, [0xa1, 0xb0, 0xc0]);
        assert_eq!(state.deck, [0x3e3, 0x22]);
        assert_eq!(state.revision(), 2);
    }
    #[test]
    fn exhausted_host_revision_refuses_before_native_write() {
        let owner = fixture();
        owner.state.write().unwrap().revision = u64::MAX;
        assert_eq!(
            owner.enable_top(1),
            Err(SkateboardVolumeFlagError::RevisionExhausted)
        );
        assert_eq!(owner.read().unwrap().top[1], 0xb0);
    }
}
