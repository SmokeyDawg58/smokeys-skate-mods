//! Typed payloads for the existing retained StepSelector batch family.
use super::{
    CollisionQueryBatchError as BatchError, CollisionQueryBatchFamily as Family,
    CollisionQueryBatchLease, CollisionQueryBatchState as State, CollisionQueryBatches,
};
use crate::{
    actor_animation_construction::AnimationStepInput, walking_step_query::StepTrajectoryResult,
};

pub(crate) type StepResult = [[StepTrajectoryResult; 4]; 5];
pub(super) struct StepTest {
    inputs: Vec<AnimationStepInput>,
    result: StepResult,
}
#[derive(Debug)]
pub(crate) enum StepBatchError<E> {
    Batch(BatchError),
    InputCountAssertion,
    Execute { test: u32, source: E },
}
impl<O> CollisionQueryBatches<O> {
    /// AddTest827be790: capacity first, then Next/state/count, then nbTests<5.
    pub(crate) fn add_step_test(
        &mut self,
        lease: &CollisionQueryBatchLease,
        inputs: &[AnimationStepInput],
    ) -> Result<bool, StepBatchError<std::convert::Infallible>> {
        use StepBatchError as E;
        let batch = self.batch_mut(lease).map_err(E::Batch)?;
        if lease.family != Family::StepSelector {
            return Err(E::Batch(BatchError::WrongFamily {
                actual: lease.family,
            }));
        }
        if batch.count == batch.capacity {
            return Ok(false);
        }
        if batch.state != State::AddTests {
            return Err(E::Batch(BatchError::State {
                actual: batch.state,
            }));
        }
        let index = batch.count as usize;
        batch.count += 1;
        // The native Next has already consumed this slot when the assertion fails.
        batch.incomplete = Some(index as u32);
        if inputs.len() >= 5 {
            return Err(E::InputCountAssertion);
        }
        if index == batch.steps.len() {
            batch.steps.push(StepTest {
                inputs: inputs.to_vec(),
                result: [[StepTrajectoryResult::new(); 4]; 5],
            });
        } else {
            let test = &mut batch.steps[index];
            for row in &mut test.result {
                for result in row {
                    result.reset();
                }
            }
            test.inputs = inputs.to_vec();
        }
        batch.incomplete = None;
        Ok(true)
    }
    pub(crate) fn end_step_tests<E>(
        &mut self,
        lease: &CollisionQueryBatchLease,
        mut execute: impl FnMut(&[AnimationStepInput], &mut StepResult) -> Result<(), E>,
    ) -> Result<(), StepBatchError<E>> {
        use StepBatchError as Error;
        let batch = self.batch_mut(lease).map_err(Error::Batch)?;
        if lease.family != Family::StepSelector {
            return Err(Error::Batch(BatchError::WrongFamily {
                actual: lease.family,
            }));
        }
        if batch.state != State::AddTests {
            return Err(Error::Batch(BatchError::State {
                actual: batch.state,
            }));
        }
        if let Some(test) = batch.incomplete {
            return Err(Error::Batch(BatchError::Incomplete {
                family: lease.family,
                slot: lease.slot,
                test,
            }));
        }
        batch.state = State::Running;
        for index in 0..batch.count {
            batch.incomplete = Some(index);
            let test = &mut batch.steps[index as usize];
            execute(&test.inputs, &mut test.result).map_err(|source| Error::Execute {
                test: index,
                source,
            })?;
        }
        batch.incomplete = None;
        Ok(())
    }
    /// GetResult827bea48/GetTest827bccf0 returns storage even on a complete miss.
    pub(crate) fn get_step_result(
        &self,
        lease: &CollisionQueryBatchLease,
        index: u32,
    ) -> Result<StepResult, BatchError> {
        let batch = self.batch(lease)?;
        if lease.family != Family::StepSelector {
            return Err(BatchError::WrongFamily {
                actual: lease.family,
            });
        }
        if batch.state != State::Idle {
            return Err(BatchError::State {
                actual: batch.state,
            });
        }
        if index >= batch.count {
            return Err(BatchError::TestIndex {
                index,
                count: batch.count,
            });
        }
        Ok(batch.steps[index as usize].result)
    }
}

/// Aggregate GetResult827bea70's candidate comparison. A greater priority can
/// still replace the current result using the CURRENT priority's hit-time lane.
/// Equal times preserve the earlier child; unordered comparisons never replace.
pub(crate) fn consider_child(current: &mut StepTrajectoryResult, candidate: &StepTrajectoryResult) {
    if !candidate.collision.time.iter().all(|time| *time >= 0.0) {
        return;
    }
    let lower = (candidate.priority as u32) < (current.priority as u32);
    let earlier = !lower
        && usize::try_from(current.priority)
            .ok()
            .and_then(|i| candidate.hit_times.get(i).zip(current.hit_times.get(i)))
            .is_some_and(|(candidate, old)| candidate < old);
    if lower || earlier {
        *current = *candidate;
    }
}

#[cfg(test)]
#[path = "collision_query_step_tests.rs"]
mod tests;
