//! Same assembly occurrence group and Part userdata, never template defaults.
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct BoardCollisionGroupSnapshot {
    pub(crate) assembly: u32,
    pub(crate) parts: [u32; 7],
    pub(crate) revision: u64,
}
#[derive(Debug, Default)]
pub(crate) struct BoardCollisionGroups {
    value: Mutex<Option<BoardCollisionGroupSnapshot>>,
    revision: AtomicU64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoardCollisionGroupError {
    Poisoned,
    Uninitialized,
    AlreadyInitialized,
    RevisionExhausted,
    PartCount,
}
impl std::fmt::Display for BoardCollisionGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "board collision groups: {self:?}")
    }
}
impl BoardCollisionGroups {
    /// Selected SkateboardBody.Init group4 then seven ascending userdata4 stores.
    pub(crate) fn try_initialize(&self, part_count: usize) -> Result<(), BoardCollisionGroupError> {
        if part_count != 7 {
            return Err(BoardCollisionGroupError::PartCount);
        }
        let mut value = self
            .value
            .lock()
            .map_err(|_| BoardCollisionGroupError::Poisoned)?;
        if value.is_some() {
            return Err(BoardCollisionGroupError::AlreadyInitialized);
        }
        *value = Some(BoardCollisionGroupSnapshot {
            assembly: 4,
            parts: [4; 7],
            revision: 1,
        });
        self.revision.store(1, Ordering::Release);
        Ok(())
    }
    pub(crate) fn try_snapshot(
        &self,
    ) -> Result<BoardCollisionGroupSnapshot, BoardCollisionGroupError> {
        self.value
            .lock()
            .map_err(|_| BoardCollisionGroupError::Poisoned)?
            .ok_or(BoardCollisionGroupError::Uninitialized)
    }
    pub(crate) fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    /// PC Object Dropper riderless board: one group for the assembly and all
    /// Parts, stored like `try_set_mode` (a port addition, no source caller).
    pub(crate) fn try_set_group(&self, group: u32) -> Result<(), BoardCollisionGroupError> {
        let mut value = self
            .value
            .lock()
            .map_err(|_| BoardCollisionGroupError::Poisoned)?;
        let value = value
            .as_mut()
            .ok_or(BoardCollisionGroupError::Uninitialized)?;
        let revision = value
            .revision
            .checked_add(1)
            .ok_or(BoardCollisionGroupError::RevisionExhausted)?;
        self.revision.store(revision, Ordering::Release);
        value.assembly = group;
        value.parts = [group; 7];
        value.revision = revision;
        Ok(())
    }
    /// Skateboard.CollideWithWorldOnly82b26880/CollideNormally82b268c8.
    /// Invalidate publications before source assembly store and ascending Parts.
    #[expect(
        clippy::manual_slice_fill,
        reason = "preserve source ascending per-Part stores"
    )]
    pub(crate) fn try_set_mode(&self, world_only: bool) -> Result<(), BoardCollisionGroupError> {
        let mut value = self
            .value
            .lock()
            .map_err(|_| BoardCollisionGroupError::Poisoned)?;
        let value = value
            .as_mut()
            .ok_or(BoardCollisionGroupError::Uninitialized)?;
        let revision = value
            .revision
            .checked_add(1)
            .ok_or(BoardCollisionGroupError::RevisionExhausted)?;
        self.revision.store(revision, Ordering::Release);
        let group = if world_only { 2 } else { 4 };
        value.assembly = group;
        for part in &mut value.parts {
            *part = group;
        }
        value.revision = revision;
        Ok(())
    }
}
