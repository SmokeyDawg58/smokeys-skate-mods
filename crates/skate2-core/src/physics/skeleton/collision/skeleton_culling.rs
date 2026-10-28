//! Retained normal/ragdoll skeleton culling resources.
//!
//! Source: `CreateNormalCullingTable` (0x82b5f348) and
//! `CreateRagdollCullingTable` (0x82b5f7d0). The caller supplies one session's
//! inventory lifetime; this is not a fabricated process-global inventory or
//! a general implementation of native inventory reload/notification services.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "skeleton culling resources await assembly admission"
    )
)]

use crate::ragdoll::{NORMAL_CULLING_CLEAR_PAIRS, RAGDOLL_CULLING_CLEAR_PAIRS};
use skate2_formats::collision::BitTable;
use std::{collections::TryReserveError, fmt, sync::Arc};

#[cfg(test)]
#[path = "skeleton_culling_tests.rs"]
mod tests;

/// The two original inventory `FastString` identities, not Vault hashes or
/// simulated resource addresses. Other inventory names are out of scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonCullingKey {
    Normal,
    Ragdoll,
}

impl SkeletonCullingKey {
    pub(crate) const fn source_name(self) -> &'static str {
        match self {
            Self::Normal => "cullTableNormal",
            Self::Ragdoll => "cullTableRagdoll",
        }
    }

    const fn slot(self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Ragdoll => 1,
        }
    }
}

/// Actual table ownership. No mutable table accessor or Clone implementation
/// is exposed: callers share the completed resource, not writable snapshots.
#[derive(Debug)]
pub(crate) struct SkeletonCullingResource {
    key: SkeletonCullingKey,
    table: BitTable,
}

impl SkeletonCullingResource {
    pub(crate) const fn key(&self) -> SkeletonCullingKey {
        self.key
    }
    pub(crate) const fn table(&self) -> &BitTable {
        &self.table
    }
}

#[derive(Debug)]
pub(crate) struct SkeletonCullingError {
    pub(crate) key: SkeletonCullingKey,
    source: TryReserveError,
}

impl fmt::Display for SkeletonCullingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot allocate {} packed words: {}",
            self.key.source_name(),
            self.source
        )
    }
}
impl std::error::Error for SkeletonCullingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Share this owner across skeleton constructors in the same host session.
/// A successful find returns the existing resource without running Initialize
/// or any clears. Separate inventories deliberately have separate identities.
#[derive(Debug, Default)]
pub(crate) struct SkeletonCullingInventory {
    resources: [Option<Arc<SkeletonCullingResource>>; 2],
}

impl SkeletonCullingInventory {
    pub(crate) fn find(&self, key: SkeletonCullingKey) -> Option<Arc<SkeletonCullingResource>> {
        self.resources[key.slot()].as_ref().map(Arc::clone)
    }

    /// Original missing-resource result, including both directed clears for
    /// every listed pair. Native allocates an 84-byte/alignment-four `BitTable`,
    /// adds it to inventory, then writes its bits. This exclusive host owner
    /// publishes only the completed table: no reentrant partial-resource view.
    ///
    /// Vec allocation failure leaves the inventory unchanged. Arc control-block
    /// allocation follows standard Rust OOM handling, not a claimed fallible
    /// native resource-allocator / `Inventory::AddResource` reconstruction.
    pub(crate) fn try_get_or_create(
        &mut self,
        key: SkeletonCullingKey,
    ) -> Result<Arc<SkeletonCullingResource>, SkeletonCullingError> {
        if let Some(existing) = self.find(key) {
            return Ok(existing);
        }
        let mut words = Vec::new();
        words
            .try_reserve_exact(18)
            .map_err(|source| SkeletonCullingError { key, source })?;
        // BitTable::Initialize (0x82bc3308): header then zero packed words.
        words.resize(18, 0);
        let mut table = BitTable {
            rows: 24,
            columns: 24,
            array_size: 18,
            words,
        };
        for row in 0..24_usize {
            for column in 0..24_usize {
                let bit = row * 24 + column;
                table.words[bit >> 5] |= 1_u32 << (bit & 31);
            }
        }
        let pairs = match key {
            SkeletonCullingKey::Normal => NORMAL_CULLING_CLEAR_PAIRS.as_slice(),
            SkeletonCullingKey::Ragdoll => RAGDOLL_CULLING_CLEAR_PAIRS.as_slice(),
        };
        for &(first, second) in pairs {
            let forward = first.index() * 24 + second.index();
            table.words[forward >> 5] &= !(1_u32 << (forward & 31));
            let reverse = second.index() * 24 + first.index();
            table.words[reverse >> 5] &= !(1_u32 << (reverse & 31));
        }
        let resource = Arc::new(SkeletonCullingResource { key, table });
        self.resources[key.slot()] = Some(Arc::clone(&resource));
        Ok(resource)
    }
}
