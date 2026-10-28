//! Retained Skateboard force records: Add 0x82b38ce0, Apply 0x82b35380.
//! The queue survives Apply and clears only at the postphysics reset boundary.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "board forces await complete native pre/postphysics scheduling"
    )
)]

use crate::{
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_force_attributes::SkateboardQueuedForceAttributes,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkateboardQueuedForce {
    pub(crate) kind: u32,
    pub(crate) force: SkateboardFootForce,
}

pub(crate) struct SkateboardForceQueue {
    entries: [Option<SkateboardQueuedForce>; 21],
    count: usize,
}

#[derive(Debug)]
pub(crate) enum SkateboardForceApplyError<A, B> {
    Attribute(A),
    Body { entry: usize, source: B },
}
impl<A: std::fmt::Display, B: std::fmt::Display> std::fmt::Display
    for SkateboardForceApplyError<A, B>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(error) => write!(f, "queued force attribute: {error}"),
            Self::Body { entry, source } => write!(f, "queued force entry {entry}: {source}"),
        }
    }
}

impl SkateboardForceQueue {
    /// Fixed-vector constructor/reset, 0x82b38b40 and 0x82b38c1c.
    pub(crate) fn new() -> Self {
        Self {
            entries: [None; 21],
            count: 0,
        }
    }

    /// Source silently ignores additional entries when all 21 slots are used.
    pub(crate) fn add(&mut self, kind: u32, force: SkateboardFootForce) {
        if self.count < self.entries.len() {
            self.entries[self.count] = Some(SkateboardQueuedForce { kind, force });
            self.count += 1;
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.count
    }

    #[cfg(test)]
    pub(crate) fn records_for_test(&self) -> Vec<SkateboardQueuedForce> {
        self.entries[..self.count]
            .iter()
            .map(|record| record.expect("occupied range"))
            .collect()
    }

    /// Immutable retained attributes cannot reenter/mutate this queue. Capture
    /// its iteration range after the initial lookup, as the native code does.
    /// Each body callback commits immediately; later errors never roll it back.
    pub(crate) fn try_apply<A: SkateboardQueuedForceAttributes, B>(
        &self,
        attributes: &mut A,
        mut apply: impl FnMut(SkateboardFootForce, f32) -> Result<(), B>,
    ) -> Result<(), SkateboardForceApplyError<A::Error, B>> {
        if self.count == 0 {
            return Ok(());
        }
        let offset = attributes
            .local_y_offset()
            .map_err(SkateboardForceApplyError::Attribute)?;
        let range = &self.entries[..self.count];
        for (entry, record) in range.iter().enumerate() {
            let record = record.expect("occupied native force range");
            apply(record.force, offset)
                .map_err(|source| SkateboardForceApplyError::Body { entry, source })?;
        }
        attributes
            .finish_lookup()
            .map_err(SkateboardForceApplyError::Attribute)
    }

    /// 0x82b37e78: caller must reach the original postphysics reset stage.
    /// Applying or a failed prephysics callback does not invoke this method.
    pub(crate) fn clear_post_physics(&mut self) {
        self.clear_logical_entries();
    }

    /// Outer TeleportReset rewrites end/begin/capacity against the same fixed
    /// buffer; logical count clears without reallocating or erasing records.
    pub(crate) fn reset_teleport(&mut self) {
        self.clear_logical_entries();
    }

    fn clear_logical_entries(&mut self) {
        self.count = 0;
    }
}

#[cfg(test)]
#[path = "skateboard_force_queue_tests.rs"]
mod tests;

#[path = "skateboard_force_queue_combined.rs"]
pub(crate) mod combined;
