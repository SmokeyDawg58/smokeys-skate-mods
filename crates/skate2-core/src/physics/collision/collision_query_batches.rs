//! Retained synchronous query batches: helper Acquire/Sync 0x827b2e48/0x827c8110,
//! `JobBatch` Begin/Sync/Release 0x827bc8f0/0x827bc6e0/0x827bf088.
//! The enclosing pool supplies its actual collection through an exclusive
//! borrowed execution callback. This owner contains no scene or geometry copy.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained batches await the actual pool query bridge"
    )
)]

use std::{fmt, sync::Arc};

use crate::{
    collision_line_collection::CollisionLineInput, collision_mesh_line::CollisionMeshLineNearest,
};

#[cfg(test)]
#[path = "collision_query_batches_tests.rs"]
mod tests;

#[path = "collision_query_step.rs"]
pub(crate) mod step;
#[path = "collision_query_trajectory.rs"]
pub(crate) mod trajectory;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CollisionQueryBatchFamily {
    Line,
    Box,
    Trajectory,
    StepSelector,
}

impl CollisionQueryBatchFamily {
    const ALL: [Self; 4] = [Self::Line, Self::Box, Self::Trajectory, Self::StepSelector];

    const fn index(self) -> usize {
        match self {
            Self::Line => 0,
            Self::Box => 1,
            Self::Trajectory => 2,
            Self::StepSelector => 3,
        }
    }
}

/// Actual constructor inputs; the caller samples its native configuration.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CollisionQueryBatchCapacity {
    pub(crate) batches: u32,
    pub(crate) tests: u32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CollisionQueryBatchConfig {
    pub(crate) line: CollisionQueryBatchCapacity,
    pub(crate) boxes: CollisionQueryBatchCapacity,
    pub(crate) trajectory: CollisionQueryBatchCapacity,
    pub(crate) step_selector: CollisionQueryBatchCapacity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CollisionQueryBatchState {
    Idle,
    AddTests,
    Running,
}

/// Not Clone: every acquisition is tied to this owner's current slot lease.
/// Dropping this token does not silently execute native Release.
pub(crate) struct CollisionQueryBatchLease {
    owner: Arc<()>,
    acquisition: Arc<()>,
    family: CollisionQueryBatchFamily,
    slot: usize,
}

impl fmt::Debug for CollisionQueryBatchLease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CollisionQueryBatchLease")
            .field("family", &self.family)
            .field("slot", &self.slot)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CollisionQueryBatchError {
    WrongOwner,
    ExpiredLease,
    Exhausted {
        family: CollisionQueryBatchFamily,
    },
    WrongFamily {
        actual: CollisionQueryBatchFamily,
    },
    State {
        actual: CollisionQueryBatchState,
    },
    TestIndex {
        index: u32,
        count: u32,
    },
    /// Explicit host refusal: a fallible local job did not return normally.
    Incomplete {
        family: CollisionQueryBatchFamily,
        slot: usize,
        test: u32,
    },
}

impl fmt::Display for CollisionQueryBatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "query batch: {self:?}")
    }
}
impl std::error::Error for CollisionQueryBatchError {}

#[derive(Debug)]
pub(crate) enum CollisionQueryBatchExecutionError<E> {
    Batch(CollisionQueryBatchError),
    Line { test: u32, source: E },
    Unsupported { family: CollisionQueryBatchFamily },
}

impl<E: fmt::Debug> fmt::Display for CollisionQueryBatchExecutionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "query batch execution: {self:?}")
    }
}
impl<E: fmt::Debug> std::error::Error for CollisionQueryBatchExecutionError<E> {}

struct LineTest<O> {
    input: CollisionLineInput,
    result: CollisionMeshLineNearest<O>,
}

struct Batch<O> {
    capacity: u32,
    count: u32,
    state: CollisionQueryBatchState,
    acquisition: Option<Arc<()>>,
    label: Option<String>,
    lines: Vec<LineTest<O>>,
    trajectories: Vec<trajectory::TrajectoryTest>,
    steps: Vec<step::StepTest>,
    incomplete: Option<u32>,
}

impl<O> Batch<O> {
    fn new(capacity: u32) -> Self {
        Self {
            capacity,
            count: 0,
            state: CollisionQueryBatchState::Idle,
            acquisition: None,
            label: None,
            lines: Vec::new(),
            trajectories: Vec::new(),
            steps: Vec::new(),
            incomplete: None,
        }
    }

    fn sync_jobs(
        &self,
        family: CollisionQueryBatchFamily,
        slot: usize,
    ) -> Result<(), CollisionQueryBatchError> {
        // Native count-zero early return comes before any job inspection.
        if self.count != 0
            && let Some(test) = self.incomplete
        {
            return Err(CollisionQueryBatchError::Incomplete { family, slot, test });
        }
        // End already ran the local callback synchronously. The four concrete
        // +2c postprocessing entries are empty; no state/count/lease changes.
        Ok(())
    }
}

/// The four helpers belong to one actual pool and retain every allocated slot.
pub(crate) struct CollisionQueryBatches<O> {
    identity: Arc<()>,
    families: [Vec<Batch<O>>; 4],
}

impl<O> CollisionQueryBatches<O> {
    pub(crate) fn new(config: CollisionQueryBatchConfig) -> Self {
        let capacities = [
            config.line,
            config.boxes,
            config.trajectory,
            config.step_selector,
        ];
        Self {
            identity: Arc::new(()),
            families: capacities.map(|capacity| {
                (0..capacity.batches)
                    .map(|_| Batch::new(capacity.tests))
                    .collect()
            }),
        }
    }

    /// Native first-free ascending acquisition, 0x827b2e48. A Rust str proves
    /// the source nonnull label; an empty string is valid and is not rejected.
    pub(crate) fn acquire(
        &mut self,
        family: CollisionQueryBatchFamily,
        label: &str,
    ) -> Result<CollisionQueryBatchLease, CollisionQueryBatchError> {
        let (slot, batch) = self.families[family.index()]
            .iter_mut()
            .enumerate()
            .find(|(_, batch)| batch.acquisition.is_none())
            .ok_or(CollisionQueryBatchError::Exhausted { family })?;
        let acquisition = Arc::new(());
        batch.acquisition = Some(Arc::clone(&acquisition));
        batch.label = Some(label.to_owned());
        Ok(CollisionQueryBatchLease {
            owner: Arc::clone(&self.identity),
            acquisition,
            family,
            slot,
        })
    }

    fn batch(
        &self,
        lease: &CollisionQueryBatchLease,
    ) -> Result<&Batch<O>, CollisionQueryBatchError> {
        if !Arc::ptr_eq(&self.identity, &lease.owner) {
            return Err(CollisionQueryBatchError::WrongOwner);
        }
        let batch = &self.families[lease.family.index()][lease.slot];
        if !batch
            .acquisition
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &lease.acquisition))
        {
            return Err(CollisionQueryBatchError::ExpiredLease);
        }
        Ok(batch)
    }

    fn batch_mut(
        &mut self,
        lease: &CollisionQueryBatchLease,
    ) -> Result<&mut Batch<O>, CollisionQueryBatchError> {
        self.batch(lease)?;
        Ok(&mut self.families[lease.family.index()][lease.slot])
    }

    pub(crate) fn begin(
        &mut self,
        lease: &CollisionQueryBatchLease,
    ) -> Result<(), CollisionQueryBatchError> {
        let batch = self.batch_mut(lease)?;
        if batch.state != CollisionQueryBatchState::Idle {
            return Err(CollisionQueryBatchError::State {
                actual: batch.state,
            });
        }
        batch.state = CollisionQueryBatchState::AddTests;
        batch.count = 0;
        // Old result storage is retained, exactly as native Begin retains tests.
        batch.incomplete = None;
        Ok(())
    }

    /// `AddTest` 0x827bd9a0: full returns false before Next's state assertion.
    pub(crate) fn add_line(
        &mut self,
        lease: &CollisionQueryBatchLease,
        input: CollisionLineInput,
    ) -> Result<bool, CollisionQueryBatchError> {
        let batch = self.batch_mut(lease)?;
        if lease.family != CollisionQueryBatchFamily::Line {
            return Err(CollisionQueryBatchError::WrongFamily {
                actual: lease.family,
            });
        }
        if batch.count == batch.capacity {
            return Ok(false);
        }
        if batch.state != CollisionQueryBatchState::AddTests {
            return Err(CollisionQueryBatchError::State {
                actual: batch.state,
            });
        }
        let index = batch.count as usize;
        batch.count += 1;
        // The concrete constructor supplies every result pointer. Rust stores
        // only addressable initialized slots; no unobserved request is invented.
        let test = LineTest {
            input,
            result: CollisionMeshLineNearest::new(),
        };
        if index == batch.lines.len() {
            batch.lines.push(test);
        } else {
            batch.lines[index] = test;
        }
        Ok(true)
    }

    /// Synchronous local End. The enclosing pool binds its ACTUAL collection
    /// for the duration of this callback. A callback returns a result only on
    /// normal completion of the source local job; failure cannot publish a miss.
    pub(crate) fn end_line<E>(
        &mut self,
        lease: &CollisionQueryBatchLease,
        mut execute: impl FnMut(CollisionLineInput) -> Result<CollisionMeshLineNearest<O>, E>,
    ) -> Result<(), CollisionQueryBatchExecutionError<E>> {
        let batch = self
            .batch_mut(lease)
            .map_err(CollisionQueryBatchExecutionError::Batch)?;
        if batch.state != CollisionQueryBatchState::AddTests {
            return Err(CollisionQueryBatchExecutionError::Batch(
                CollisionQueryBatchError::State {
                    actual: batch.state,
                },
            ));
        }
        batch.state = CollisionQueryBatchState::Running;
        // Running commits before Prepare/any local execution. Non-line callers
        // have no public request-acceptance API until their typed inputs exist.
        if lease.family != CollisionQueryBatchFamily::Line {
            if batch.count != 0 {
                batch.incomplete = Some(0);
                return Err(CollisionQueryBatchExecutionError::Unsupported {
                    family: lease.family,
                });
            }
            return Ok(());
        }
        for index in 0..batch.count {
            batch.incomplete = Some(index);
            let result = execute(batch.lines[index as usize].input).map_err(|source| {
                CollisionQueryBatchExecutionError::Line {
                    test: index,
                    source,
                }
            })?;
            batch.lines[index as usize].result = result;
        }
        batch.incomplete = None;
        Ok(())
    }

    /// `DynamicPool::BeginUpdate`'s four helper calls in line/box/trajectory/step
    /// order, every allocated slot. This deliberately does not call public Sync.
    pub(crate) fn sync_jobs_all(&self) -> Result<(), CollisionQueryBatchError> {
        for family in CollisionQueryBatchFamily::ALL {
            for (slot, batch) in self.families[family.index()].iter().enumerate() {
                batch.sync_jobs(family, slot)?;
            }
        }
        Ok(())
    }

    pub(crate) fn sync(
        &mut self,
        lease: &CollisionQueryBatchLease,
    ) -> Result<(), CollisionQueryBatchError> {
        let batch = self.batch_mut(lease)?;
        if batch.state == CollisionQueryBatchState::AddTests {
            return Err(CollisionQueryBatchError::State {
                actual: batch.state,
            });
        }
        batch.sync_jobs(lease.family, lease.slot)?;
        batch.state = CollisionQueryBatchState::Idle;
        Ok(())
    }

    /// `GetTest` 0x827bc480 checks Idle before index; `GetResult` then checks actual
    /// hit count. It does not implicitly wait or transition a Running batch.
    pub(crate) fn get_line_result(
        &self,
        lease: &CollisionQueryBatchLease,
        index: u32,
    ) -> Result<Option<&CollisionMeshLineNearest<O>>, CollisionQueryBatchError> {
        let batch = self.batch(lease)?;
        if lease.family != CollisionQueryBatchFamily::Line {
            return Err(CollisionQueryBatchError::WrongFamily {
                actual: lease.family,
            });
        }
        if batch.state != CollisionQueryBatchState::Idle {
            return Err(CollisionQueryBatchError::State {
                actual: batch.state,
            });
        }
        if index >= batch.count {
            return Err(CollisionQueryBatchError::TestIndex {
                index,
                count: batch.count,
            });
        }
        let result = &batch.lines[index as usize].result;
        Ok((result.hit_count != 0).then_some(result))
    }

    /// Diagnostic memory observation after the synchronous local job completed.
    /// This is NOT native `GetResult` and performs no Sync/state transition.
    #[cfg(test)]
    pub(crate) fn completed_line_for_test(
        &self,
        lease: &CollisionQueryBatchLease,
        index: u32,
    ) -> Result<Option<&CollisionMeshLineNearest<O>>, CollisionQueryBatchError> {
        let batch = self.batch(lease)?;
        if batch.state == CollisionQueryBatchState::AddTests {
            return Err(CollisionQueryBatchError::State {
                actual: batch.state,
            });
        }
        batch.sync_jobs(lease.family, lease.slot)?;
        let line = batch
            .lines
            .get(index as usize)
            .ok_or(CollisionQueryBatchError::TestIndex {
                index,
                count: batch.count,
            })?;
        Ok((line.result.hit_count != 0).then_some(&line.result))
    }

    /// Release synchronizes before Idle and lease clearing; it retains count,
    /// results and the diagnostic label. A refused job retains its lease.
    pub(crate) fn release(
        &mut self,
        lease: &CollisionQueryBatchLease,
    ) -> Result<(), CollisionQueryBatchError> {
        let batch = self.batch_mut(lease)?;
        batch.sync_jobs(lease.family, lease.slot)?;
        batch.state = CollisionQueryBatchState::Idle;
        batch.acquisition = None;
        Ok(())
    }

    pub(crate) fn state(
        &self,
        lease: &CollisionQueryBatchLease,
    ) -> Result<CollisionQueryBatchState, CollisionQueryBatchError> {
        Ok(self.batch(lease)?.state)
    }

    pub(crate) fn test_count(
        &self,
        lease: &CollisionQueryBatchLease,
    ) -> Result<u32, CollisionQueryBatchError> {
        Ok(self.batch(lease)?.count)
    }
}
